//! Application laws for producer-incarnation projection and custody.

use super::*;

use crate::principal_trust_test_support as principal_trust_bootstrap;
use crate::producer_record_fixture::{
    admit_allocation, control_successor, producer_preimage, schema_digest, trust_successor,
};
use pale_core::records::*;
use pale_ports::{
    ControlRecord, ControlRecordBytes, ControlRecordCoordinate, ControlRecordDigest,
    PrincipalProducerIncarnationHeadCursor, PrincipalProducerIncarnationHeadPage,
    PrincipalProducerIncarnationHeadPageEntry, PrincipalProducerIncarnationReadObservation,
    PrincipalTrustSnapshot,
};
use std::collections::BTreeMap;

#[path = "producer/anchors.rs"]
mod anchors;

struct FakeStore {
    commit: Option<Result<PrincipalProducerIncarnationCommitDisposition, ControlStoreError>>,
    observation: Option<Result<PrincipalProducerIncarnationDatabaseObservation, ControlStoreError>>,
    observation_calls: usize,
}

impl pale_ports::ControlTransactionPort for FakeStore {
    fn prepare_transaction(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, pale_ports::ControlStoreError> {
        panic!("this physical-custody/readback fixture does not prepare a new checkpoint")
    }
}

impl PrincipalProducerIncarnationCustodyPort for FakeStore {
    fn prepare_accumulator(
        &mut self,
        _: &pale_ports::OperationAccumulatorPreparation,
    ) -> Result<
        pale_ports::PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        panic!("this custody-state fixture does not construct checkpoint roots")
    }

    fn compare_and_commit(
        &mut self,
        _: &PreparedPrincipalProducerIncarnationTransition,
    ) -> Result<PrincipalProducerIncarnationCommitDisposition, ControlStoreError> {
        self.commit.take().expect("one commit call")
    }

    fn observe_exact(
        &mut self,
        _: &PreparedPrincipalProducerIncarnationTransition,
    ) -> Result<PrincipalProducerIncarnationDatabaseObservation, ControlStoreError> {
        self.observation_calls += 1;
        self.observation.take().expect("one observation call")
    }
}

fn digest(byte: char) -> ControlRecordDigest {
    ControlRecordDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>())
        .expect("record digest")
}

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record ref"),
        digest(byte),
    )
}

fn record(reference: &str, byte: char) -> ControlRecord {
    ControlRecord::new(
        coordinate(reference, byte),
        SchemaId::parse("pale/test.v1").expect("schema"),
        SchemaDigest::parse("a".repeat(64)).expect("schema digest"),
        ControlRecordBytes::new(format!(r#"{{"record":"{reference}"}}"#).into_bytes())
            .expect("record bytes"),
    )
}

fn producer_record<R>(
    coordinate: ControlRecordCoordinate,
    schema: &str,
    schema_sha256: &SchemaDigest,
    record: &R,
) -> ControlRecord
where
    R: serde::Serialize,
{
    ControlRecord::new(
        coordinate,
        SchemaId::parse(schema).expect("producer schema"),
        schema_sha256.clone(),
        ControlRecordBytes::new(
            canon::to_canonical_string(record)
                .expect("canonical producer record")
                .into_bytes(),
        )
        .expect("record bytes"),
    )
}

struct FakeProducerHeadReader {
    heads: Vec<PrincipalProducerIncarnationRecordSet>,
    history: BTreeMap<ControlRecordCoordinate, PrincipalProducerIncarnationRecordSet>,
}

impl PrincipalProducerIncarnationReadPort for FakeProducerHeadReader {
    fn read_by_epoch(
        &mut self,
        _: &PrincipalTrustStoreBinding,
        _: &ProducerEpochV1,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        unreachable!("frontier recovery uses bounded page reads")
    }

    fn read_head(
        &mut self,
        _: &PrincipalTrustStoreBinding,
        _: &ProducerIdV1,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        unreachable!("frontier recovery uses bounded page reads")
    }

    fn read_by_allocation(
        &mut self,
        _: &PrincipalTrustSnapshot,
        allocation: &ControlRecordCoordinate,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        Ok(self.history.get(allocation).cloned().map_or(
            PrincipalProducerIncarnationReadObservation::Absent,
            |records| PrincipalProducerIncarnationReadObservation::Exact(Box::new(records)),
        ))
    }

    fn read_heads_page(
        &mut self,
        request: &PrincipalProducerIncarnationHeadPageRead,
    ) -> Result<PrincipalProducerIncarnationHeadPage, ControlStoreError> {
        let after = request.after().map(|cursor| cursor.producer_id());
        let mut remaining = self
            .heads
            .iter()
            .filter(|records| after.is_none_or(|after| records.head().producer_id() > after))
            .take(request.maximum_items() + 1)
            .cloned()
            .collect::<Vec<_>>();
        let has_more = remaining.len() > request.maximum_items();
        remaining.truncate(request.maximum_items());
        let entries = remaining
            .into_iter()
            .map(|records| {
                PrincipalProducerIncarnationHeadPageEntry::new(
                    PrincipalProducerIncarnationHeadCursor::new(
                        records.head().producer_id().clone(),
                    ),
                    records,
                )
                .expect("matching fake cursor")
            })
            .collect::<Vec<_>>();
        let boundary = if has_more {
            PrincipalProducerIncarnationHeadPageBoundary::More {
                resume_after: entries.last().expect("non-empty page").cursor().clone(),
            }
        } else {
            PrincipalProducerIncarnationHeadPageBoundary::Complete
        };
        PrincipalProducerIncarnationHeadPage::new(request, entries, boundary)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
    }
}

struct TestRecoveredIncarnation<'a> {
    allocation_ref: &'a ResidentRecordRefV1,
    allocation: &'a ProducerIncarnationV1,
    retirement: Option<(&'a ResidentRecordRefV1, &'a ProducerIncarnationRetirementV1)>,
}

fn physical_record_set(
    incarnation: &TestRecoveredIncarnation<'_>,
) -> PrincipalProducerIncarnationRecordSet {
    let head = incarnation
        .retirement
        .map_or_else(
            || {
                ProducerIncarnationHeadV1::active(
                    incarnation.allocation_ref,
                    incarnation.allocation,
                )
            },
            |(_, retirement)| {
                ProducerIncarnationHeadV1::retired(
                    incarnation.allocation_ref,
                    incarnation.allocation,
                    retirement,
                )
            },
        )
        .expect("semantic producer head");
    let allocation = ControlRecordCoordinate::new(
        head.incarnation_ref().clone(),
        ControlRecordDigest::parse(head.incarnation_sha256().as_str()).expect("allocation digest"),
    );
    let retirement = incarnation.retirement.map(|(record_ref, record)| {
        ControlRecordCoordinate::new(
            record_ref.clone(),
            ControlRecordDigest::parse(record.canonical_retirement_sha256().as_str())
                .expect("retirement digest"),
        )
    });
    let previous_allocation = incarnation
        .allocation
        .previous_incarnation_ref()
        .as_ref()
        .zip(
            incarnation
                .allocation
                .previous_incarnation_sha256()
                .as_ref(),
        )
        .map(|(record_ref, digest)| {
            ControlRecordCoordinate::new(
                record_ref.clone(),
                ControlRecordDigest::parse(digest.as_str()).expect("predecessor allocation digest"),
            )
        });
    let physical = PrincipalProducerIncarnationHead::new(
        head.origin_principal_id().clone(),
        head.producer_id().clone(),
        head.producer_epoch().clone(),
        head.state(),
        allocation.clone(),
        retirement.clone(),
    )
    .expect("physical producer head");
    PrincipalProducerIncarnationRecordSet::new(
        physical,
        previous_allocation,
        producer_record(
            allocation,
            PRODUCER_INCARNATION_V1_SCHEMA,
            incarnation.allocation.schema_sha256(),
            incarnation.allocation,
        ),
        retirement.map(|coordinate| {
            let (_, record) = incarnation.retirement.expect("retirement coordinate");
            producer_record(
                coordinate,
                PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA,
                record.schema_sha256(),
                record,
            )
        }),
    )
    .expect("closed producer records")
}

#[derive(Default)]
struct TestRecoveryAdmission {
    allocations: BTreeMap<Vec<u8>, ProducerIncarnationV1>,
    retirements: BTreeMap<Vec<u8>, ProducerIncarnationRetirementV1>,
}

impl TestRecoveryAdmission {
    fn from_incarnations(incarnations: &[TestRecoveredIncarnation<'_>]) -> Self {
        let allocations = incarnations
            .iter()
            .map(|incarnation| {
                (
                    canon::to_canonical_string(incarnation.allocation)
                        .expect("canonical allocation")
                        .into_bytes(),
                    incarnation.allocation.clone(),
                )
            })
            .collect();
        let retirements = incarnations
            .iter()
            .filter_map(|incarnation| incarnation.retirement.map(|(_, record)| record))
            .map(|retirement| {
                (
                    canon::to_canonical_string(retirement)
                        .expect("canonical retirement")
                        .into_bytes(),
                    retirement.clone(),
                )
            })
            .collect();
        Self {
            allocations,
            retirements,
        }
    }
}

impl PrincipalProducerIncarnationRecoveryAdmission for TestRecoveryAdmission {
    fn admit_allocation(
        &mut self,
        wire: ProducerIncarnationWireV1,
        head_state: &ProducerIncarnationHeadAdmissionStateV1<'_>,
    ) -> Result<ProducerIncarnationV1, InvalidResidentRecordV1> {
        let allocation = self
            .allocations
            .get(
                canon::to_canonical_string(&wire)
                    .expect("canonical allocation wire")
                    .as_bytes(),
            )
            .cloned()
            .ok_or(InvalidResidentRecordV1::ContextualAdmission {
                record: "producer-incarnation test allocation",
                invariant: "wire matches one admitted fixture allocation",
            })?;
        let predecessor_matches = match head_state {
            ProducerIncarnationHeadAdmissionStateV1::Unallocated => {
                allocation.previous_incarnation_ref().is_none()
                    && allocation.previous_incarnation_sha256().is_none()
            }
            ProducerIncarnationHeadAdmissionStateV1::Retired {
                incarnation_ref,
                incarnation,
                ..
            } => {
                allocation.previous_incarnation_ref().as_ref() == Some(*incarnation_ref)
                    && allocation.previous_incarnation_sha256().as_ref()
                        == Some(incarnation.canonical_incarnation_sha256())
            }
            ProducerIncarnationHeadAdmissionStateV1::Active { .. } => false,
        };
        predecessor_matches.then_some(allocation).ok_or(
            InvalidResidentRecordV1::ContextualAdmission {
                record: "producer-incarnation test allocation",
                invariant: "wire matches the recovered predecessor state",
            },
        )
    }

    fn admit_retirement(
        &mut self,
        wire: ProducerIncarnationRetirementWireV1,
        head_state: &ProducerIncarnationHeadAdmissionStateV1<'_>,
    ) -> Result<ProducerIncarnationRetirementV1, InvalidResidentRecordV1> {
        let retirement = self
            .retirements
            .get(
                canon::to_canonical_string(&wire)
                    .expect("canonical retirement wire")
                    .as_bytes(),
            )
            .cloned()
            .ok_or(InvalidResidentRecordV1::ContextualAdmission {
                record: "producer-incarnation test retirement",
                invariant: "wire matches one admitted fixture retirement",
            })?;
        let allocation_matches = match head_state {
            ProducerIncarnationHeadAdmissionStateV1::Active {
                incarnation_ref,
                incarnation,
            } => {
                retirement.incarnation_ref() == *incarnation_ref
                    && retirement.incarnation_sha256() == incarnation.canonical_incarnation_sha256()
            }
            ProducerIncarnationHeadAdmissionStateV1::Unallocated
            | ProducerIncarnationHeadAdmissionStateV1::Retired { .. } => false,
        };
        allocation_matches.then_some(retirement).ok_or(
            InvalidResidentRecordV1::ContextualAdmission {
                record: "producer-incarnation test retirement",
                invariant: "wire retires the recovered active allocation",
            },
        )
    }
}

/// Deliberately permissive witness used to prove recovery's own lineage checks.
struct LenientRecoveryAdmission(TestRecoveryAdmission);

impl PrincipalProducerIncarnationRecoveryAdmission for LenientRecoveryAdmission {
    fn admit_allocation(
        &mut self,
        wire: ProducerIncarnationWireV1,
        _: &ProducerIncarnationHeadAdmissionStateV1<'_>,
    ) -> Result<ProducerIncarnationV1, InvalidResidentRecordV1> {
        self.0
            .allocations
            .get(
                canon::to_canonical_string(&wire)
                    .expect("canonical allocation wire")
                    .as_bytes(),
            )
            .cloned()
            .ok_or(InvalidResidentRecordV1::ContextualAdmission {
                record: "producer-incarnation lenient test allocation",
                invariant: "wire matches one admitted fixture allocation",
            })
    }

    fn admit_retirement(
        &mut self,
        wire: ProducerIncarnationRetirementWireV1,
        _: &ProducerIncarnationHeadAdmissionStateV1<'_>,
    ) -> Result<ProducerIncarnationRetirementV1, InvalidResidentRecordV1> {
        self.0
            .retirements
            .get(
                canon::to_canonical_string(&wire)
                    .expect("canonical retirement wire")
                    .as_bytes(),
            )
            .cloned()
            .ok_or(InvalidResidentRecordV1::ContextualAdmission {
                record: "producer-incarnation lenient test retirement",
                invariant: "wire matches one admitted fixture retirement",
            })
    }
}

fn recovered_frontier(
    incarnations: &[TestRecoveredIncarnation<'_>],
    checkpoint_ref: &ResidentRecordRefV1,
    checkpoint: &PrincipalTrustCheckpointV1,
) -> RecoveredPrincipalProducerIncarnationFrontier {
    try_recovered_frontier(incarnations, checkpoint_ref, checkpoint)
        .expect("recovered producer frontier")
}

fn try_recovered_frontier(
    incarnations: &[TestRecoveredIncarnation<'_>],
    checkpoint_ref: &ResidentRecordRefV1,
    checkpoint: &PrincipalTrustCheckpointV1,
) -> Result<
    RecoveredPrincipalProducerIncarnationFrontier,
    PrincipalProducerIncarnationProjectionError,
> {
    let records = incarnations
        .iter()
        .map(physical_record_set)
        .collect::<Vec<_>>();
    let heads = records.clone();
    let history = records
        .into_iter()
        .map(|records| (records.allocation().coordinate().clone(), records))
        .collect();
    let mut admission = TestRecoveryAdmission::from_incarnations(incarnations);
    recover_principal_producer_incarnation_frontier(
        &mut FakeProducerHeadReader { heads, history },
        binding(),
        CurrentPrincipalTrustHead {
            checkpoint_ref,
            checkpoint,
        },
        1,
        &mut admission,
    )
}

fn binding() -> PrincipalTrustStoreBinding {
    PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("trust-store-1").expect("trust store"),
        ControlIncarnationIdV1::parse("trust-incarnation-1").expect("trust incarnation"),
        ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
    )
}

fn admit_retirement(
    allocation_ref: &ResidentRecordRefV1,
    allocation: &ProducerIncarnationV1,
    authority_ref: &ResidentRecordRefV1,
    authority: &ProducerIncarnationAuthorityV1,
) -> ProducerIncarnationRetirementV1 {
    admit_retirement_at(
        allocation_ref,
        allocation,
        authority_ref,
        authority,
        "retirement-a",
        60,
    )
}

fn admit_retirement_at(
    allocation_ref: &ResidentRecordRefV1,
    allocation: &ProducerIncarnationV1,
    authority_ref: &ResidentRecordRefV1,
    authority: &ProducerIncarnationAuthorityV1,
    retirement_id: &str,
    retired_at: u64,
) -> ProducerIncarnationRetirementV1 {
    let preimage = ProducerIncarnationRetirementPreimageV1 {
        schema_id: SchemaId::parse(PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA)
            .expect("retirement schema"),
        schema_sha256: schema_digest('d'),
        retirement_id: ProducerIncarnationRetirementIdV1::parse(retirement_id)
            .expect("retirement id"),
        origin_principal_id: allocation.origin_principal_id().clone(),
        producer_id: allocation.producer_id().clone(),
        producer_epoch: allocation.producer_epoch().clone(),
        incarnation_ref: allocation_ref.clone(),
        incarnation_sha256: allocation.canonical_incarnation_sha256().clone(),
        retired_at: UnixTimeV1::new(retired_at).expect("retirement time"),
        reason_code: ResidentReasonV1::parse("clean-rotation").expect("reason"),
        producer_incarnation_authority_ref: authority_ref.clone(),
        producer_incarnation_authority_sha256: authority
            .canonical_producer_authority_sha256()
            .clone(),
        issuer_principal_id: authority.principal_id().clone(),
        issuer_key_id: authority.subject_key_id().clone(),
        issuer_key_epoch: *authority.subject_key_epoch(),
    };
    let digest =
        ProducerIncarnationRetirementDigest::hash_canonical(&preimage).expect("retirement digest");
    ProducerIncarnationRetirementV1::admit_contextual_wire(
        principal_trust_bootstrap::signed_wire(&preimage, "canonical_retirement_sha256", &digest),
        &ProducerIncarnationHeadAdmissionStateV1::active(allocation_ref, allocation)
            .expect("active head"),
        &preimage.schema_sha256,
        &ProducerIncarnationAuthorityAdmissionContextV1::new(authority_ref, authority)
            .expect("authority context"),
        |_, _| true,
        |_, _, _, _, _| Ok(()),
    )
    .expect("retirement admits")
}

struct TwoGenerationLineage {
    first_ref: ResidentRecordRefV1,
    first: ProducerIncarnationV1,
    retirement_ref: ResidentRecordRefV1,
    retirement: ProducerIncarnationRetirementV1,
    second_ref: ResidentRecordRefV1,
    second: ProducerIncarnationV1,
}

fn two_generation_lineage() -> TwoGenerationLineage {
    let (fixture, _) = principal_trust_bootstrap::admitted_bootstrap_fixture();
    let atom = fixture.admitted.atom_view();
    let authority_ref = atom.authorities().producer_authority_ref();
    let authority = atom.authorities().producer_authority();
    let first_ref = ResidentRecordRefV1::parse("allocation-a").expect("first allocation ref");
    let first = admit_allocation(
        &producer_preimage(authority_ref, authority, 'a', 50),
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
    );
    let retirement_ref = ResidentRecordRefV1::parse("retirement-a").expect("retirement ref");
    let retirement = admit_retirement(&first_ref, &first, authority_ref, authority);
    let second_ref = ResidentRecordRefV1::parse("allocation-b").expect("successor ref");
    let mut second_preimage = producer_preimage(authority_ref, authority, 'b', 61);
    second_preimage.previous_incarnation_ref = Some(first_ref.clone());
    second_preimage.previous_incarnation_sha256 =
        Some(first.canonical_incarnation_sha256().clone());
    let second = admit_allocation(
        &second_preimage,
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::retired(&first_ref, &first, &retirement)
            .expect("retired predecessor"),
    );
    TwoGenerationLineage {
        first_ref,
        first,
        retirement_ref,
        retirement,
        second_ref,
        second,
    }
}

fn replace_allocation_bytes(
    records: &PrincipalProducerIncarnationRecordSet,
    bytes: Vec<u8>,
) -> PrincipalProducerIncarnationRecordSet {
    let allocation = ControlRecord::new(
        records.allocation().coordinate().clone(),
        records.allocation().schema_id().clone(),
        records.allocation().schema_sha256().clone(),
        ControlRecordBytes::new(bytes).expect("replacement allocation bytes"),
    );
    PrincipalProducerIncarnationRecordSet::new(
        records.head().clone(),
        records.previous_allocation().cloned(),
        allocation,
        records.retirement().cloned(),
    )
    .expect("replacement retains physical head closure")
}

fn trust_checkpoint_for_heads(
    reference: &str,
    heads: &[ProducerIncarnationHeadV1],
) -> (ResidentRecordRefV1, PrincipalTrustCheckpointV1) {
    let (fixture, _) = principal_trust_bootstrap::admitted_bootstrap_fixture();
    let (predecessor_ref, predecessor) = {
        let atom = fixture.admitted.atom_view();
        let (record_ref, record) = atom.trust_checkpoint_entry();
        (record_ref.clone(), record.clone())
    };
    trust_successor(
        fixture.trust_preimage,
        &predecessor_ref,
        &predecessor,
        reference,
        producer_incarnation_heads_root_sha256(heads).expect("producer head root"),
        |_| {},
    )
}

fn trust_head(epoch: u64) -> PrincipalTrustCheckpointHead {
    PrincipalTrustCheckpointHead::new(
        binding().principal_id().clone(),
        PrincipalTrustCheckpointEpochV1::new(epoch).expect("trust epoch"),
        coordinate(&format!("trust-{epoch}"), decimal(epoch)),
    )
}

fn control_head(sequence: u64) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("producer:producer-1").expect("domain"),
        ControlDomainEpochV1::new(1).expect("domain epoch"),
        ControlDomainSequenceV1::new(sequence).expect("domain sequence"),
        pale_core::records::ControlTransactionSequenceV1::new(sequence + 20).unwrap(),
        coordinate(&format!("control-{sequence}"), decimal(sequence + 4)),
    )
}

fn decimal(value: u64) -> char {
    char::from(b'0' + u8::try_from(value).expect("single decimal digit"))
}

fn physical_producer_head() -> PrincipalProducerIncarnationHead {
    PrincipalProducerIncarnationHead::new(
        binding().principal_id().clone(),
        ProducerIdV1::parse("producer-1").expect("producer"),
        ProducerEpochV1::parse("a".repeat(32)).expect("producer epoch"),
        ProducerIncarnationHeadStateV1::Active,
        coordinate("allocation-a", 'a'),
        None,
    )
    .expect("producer head")
}

#[test]
fn shared_principal_trust_fixture_crosses_only_public_admission_boundaries() {
    let (admitted, _) = principal_trust_bootstrap::admitted_bootstrap_fixture();
    assert_eq!(admitted.admitted.principal_id(), binding().principal_id());
}

#[test]
fn restart_recovery_reassembles_multiple_pages_under_one_signed_root() {
    let (fixture, _) = principal_trust_bootstrap::admitted_bootstrap_fixture();
    let atom = fixture.admitted.atom_view();
    let authority_ref = atom.authorities().producer_authority_ref();
    let authority = atom.authorities().producer_authority();
    let first_ref = ResidentRecordRefV1::parse("allocation-a").expect("allocation ref");
    let first = admit_allocation(
        &producer_preimage(authority_ref, authority, 'a', 50),
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
    );
    let second_ref = ResidentRecordRefV1::parse("allocation-b").expect("allocation ref");
    let mut second_preimage = producer_preimage(authority_ref, authority, 'b', 51);
    second_preimage.incarnation_id =
        ProducerIncarnationIdV1::parse("incarnation-b").expect("incarnation id");
    second_preimage.producer_id = ProducerIdV1::parse("producer-2").expect("producer");
    let second = admit_allocation(
        &second_preimage,
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
    );
    let heads = vec![
        ProducerIncarnationHeadV1::active(&first_ref, &first).expect("first head"),
        ProducerIncarnationHeadV1::active(&second_ref, &second).expect("second head"),
    ];
    let (trust_ref, trust) = atom.trust_checkpoint_entry();
    let (successor_ref, successor) = trust_successor(
        fixture.trust_preimage,
        trust_ref,
        trust,
        "trust-checkpoint-multiple-producers",
        producer_incarnation_heads_root_sha256(&heads).expect("producer root"),
        |_| {},
    );

    let recovered = recovered_frontier(
        &[
            TestRecoveredIncarnation {
                allocation_ref: &first_ref,
                allocation: &first,
                retirement: None,
            },
            TestRecoveredIncarnation {
                allocation_ref: &second_ref,
                allocation: &second,
                retirement: None,
            },
        ],
        &successor_ref,
        &successor,
    );
    assert_eq!(recovered.heads(), heads);
    assert_eq!(
        recovered.through().checkpoint_epoch(),
        *successor.checkpoint_epoch()
    );
}

#[test]
fn restart_recovery_replays_successor_history_oldest_to_newest() {
    let (fixture, _) = principal_trust_bootstrap::admitted_bootstrap_fixture();
    let atom = fixture.admitted.atom_view();
    let authority_ref = atom.authorities().producer_authority_ref();
    let authority = atom.authorities().producer_authority();
    let first_ref = ResidentRecordRefV1::parse("allocation-a").expect("first allocation ref");
    let first = admit_allocation(
        &producer_preimage(authority_ref, authority, 'a', 50),
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
    );
    let retirement_ref = ResidentRecordRefV1::parse("retirement-a").expect("retirement record ref");
    let retirement = admit_retirement(&first_ref, &first, authority_ref, authority);
    let second_ref = ResidentRecordRefV1::parse("allocation-b").expect("successor ref");
    let mut second_preimage = producer_preimage(authority_ref, authority, 'b', 61);
    second_preimage.previous_incarnation_ref = Some(first_ref.clone());
    second_preimage.previous_incarnation_sha256 =
        Some(first.canonical_incarnation_sha256().clone());
    let second = admit_allocation(
        &second_preimage,
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::retired(&first_ref, &first, &retirement)
            .expect("retired predecessor"),
    );
    let predecessor = TestRecoveredIncarnation {
        allocation_ref: &first_ref,
        allocation: &first,
        retirement: Some((&retirement_ref, &retirement)),
    };
    let successor = TestRecoveredIncarnation {
        allocation_ref: &second_ref,
        allocation: &second,
        retirement: None,
    };
    let predecessor_physical = physical_record_set(&predecessor);
    let successor_physical = physical_record_set(&successor);
    let mut history = BTreeMap::new();
    history.insert(
        predecessor_physical.allocation().coordinate().clone(),
        predecessor_physical,
    );
    history.insert(
        successor_physical.allocation().coordinate().clone(),
        successor_physical.clone(),
    );
    let expected_head =
        ProducerIncarnationHeadV1::active(&second_ref, &second).expect("successor head");
    let (trust_ref, trust) = atom.trust_checkpoint_entry();
    let (current_ref, current) = trust_successor(
        fixture.trust_preimage,
        trust_ref,
        trust,
        "trust-checkpoint-successor-recovery",
        producer_incarnation_heads_root_sha256(std::slice::from_ref(&expected_head))
            .expect("successor root"),
        |_| {},
    );
    let mut admission = TestRecoveryAdmission::from_incarnations(&[predecessor, successor]);
    let recovered = recover_principal_producer_incarnation_frontier(
        &mut FakeProducerHeadReader {
            heads: vec![successor_physical],
            history,
        },
        binding(),
        CurrentPrincipalTrustHead {
            checkpoint_ref: &current_ref,
            checkpoint: &current,
        },
        1,
        &mut admission,
    )
    .expect("successor lineage recovers");

    assert_eq!(recovered.heads(), std::slice::from_ref(&expected_head));
}

#[test]
fn restart_recovery_rejects_physical_predecessor_substitution_independently() {
    let (fixture, _) = principal_trust_bootstrap::admitted_bootstrap_fixture();
    let atom = fixture.admitted.atom_view();
    let authority_ref = atom.authorities().producer_authority_ref();
    let authority = atom.authorities().producer_authority();
    let first_ref = ResidentRecordRefV1::parse("allocation-a").expect("first allocation ref");
    let first = admit_allocation(
        &producer_preimage(authority_ref, authority, 'a', 50),
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
    );
    let retirement_ref = ResidentRecordRefV1::parse("retirement-a").expect("retirement ref");
    let retirement = admit_retirement(&first_ref, &first, authority_ref, authority);
    let successor_ref = ResidentRecordRefV1::parse("allocation-b").expect("successor ref");
    let mut successor_preimage = producer_preimage(authority_ref, authority, 'b', 61);
    successor_preimage.previous_incarnation_ref = Some(first_ref.clone());
    successor_preimage.previous_incarnation_sha256 =
        Some(first.canonical_incarnation_sha256().clone());
    let successor = admit_allocation(
        &successor_preimage,
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::retired(&first_ref, &first, &retirement)
            .expect("retired predecessor"),
    );
    let semantic_head =
        ProducerIncarnationHeadV1::active(&successor_ref, &successor).expect("successor head");
    let exact_physical = physical_record_set(&TestRecoveredIncarnation {
        allocation_ref: &successor_ref,
        allocation: &successor,
        retirement: None,
    });
    let substituted_physical = PrincipalProducerIncarnationRecordSet::new(
        exact_physical.head().clone(),
        None,
        exact_physical.allocation().clone(),
        None,
    )
    .expect("head remains structurally closed");
    let (trust_ref, trust) = atom.trust_checkpoint_entry();
    let (current_ref, current) = trust_successor(
        fixture.trust_preimage,
        trust_ref,
        trust,
        "trust-checkpoint-substituted-predecessor",
        producer_incarnation_heads_root_sha256(std::slice::from_ref(&semantic_head))
            .expect("successor root"),
        |_| {},
    );
    let mut admission = LenientRecoveryAdmission(TestRecoveryAdmission::from_incarnations(&[
        TestRecoveredIncarnation {
            allocation_ref: &first_ref,
            allocation: &first,
            retirement: Some((&retirement_ref, &retirement)),
        },
        TestRecoveredIncarnation {
            allocation_ref: &successor_ref,
            allocation: &successor,
            retirement: None,
        },
    ]));
    let error = recover_principal_producer_incarnation_frontier(
        &mut FakeProducerHeadReader {
            heads: vec![substituted_physical.clone()],
            history: BTreeMap::from([(
                substituted_physical.allocation().coordinate().clone(),
                substituted_physical,
            )]),
        },
        binding(),
        CurrentPrincipalTrustHead {
            checkpoint_ref: &current_ref,
            checkpoint: &current,
        },
        1,
        &mut admission,
    )
    .expect_err("physical predecessor substitution must fail independently");

    assert!(matches!(
        error,
        PrincipalProducerIncarnationProjectionError::PhysicalPredecessorMismatch
    ));
}

#[test]
fn restart_recovery_rejects_a_successor_whose_retirement_is_missing() {
    let (fixture, _) = principal_trust_bootstrap::admitted_bootstrap_fixture();
    let atom = fixture.admitted.atom_view();
    let authority_ref = atom.authorities().producer_authority_ref();
    let authority = atom.authorities().producer_authority();
    let first_ref = ResidentRecordRefV1::parse("allocation-a").expect("first allocation ref");
    let first = admit_allocation(
        &producer_preimage(authority_ref, authority, 'a', 50),
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
    );
    let retirement_ref = ResidentRecordRefV1::parse("retirement-a").expect("retirement record ref");
    let retirement = admit_retirement(&first_ref, &first, authority_ref, authority);
    let second_ref = ResidentRecordRefV1::parse("allocation-b").expect("successor ref");
    let mut second_preimage = producer_preimage(authority_ref, authority, 'b', 61);
    second_preimage.previous_incarnation_ref = Some(first_ref.clone());
    second_preimage.previous_incarnation_sha256 =
        Some(first.canonical_incarnation_sha256().clone());
    let second = admit_allocation(
        &second_preimage,
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::retired(&first_ref, &first, &retirement)
            .expect("retired predecessor"),
    );
    let predecessor = TestRecoveredIncarnation {
        allocation_ref: &first_ref,
        allocation: &first,
        retirement: None,
    };
    let successor = TestRecoveredIncarnation {
        allocation_ref: &second_ref,
        allocation: &second,
        retirement: None,
    };
    let predecessor_physical = physical_record_set(&predecessor);
    let successor_physical = physical_record_set(&successor);
    let mut history = BTreeMap::new();
    history.insert(
        predecessor_physical.allocation().coordinate().clone(),
        predecessor_physical,
    );
    history.insert(
        successor_physical.allocation().coordinate().clone(),
        successor_physical.clone(),
    );
    let expected_head =
        ProducerIncarnationHeadV1::active(&second_ref, &second).expect("successor head");
    let (trust_ref, trust) = atom.trust_checkpoint_entry();
    let (current_ref, current) = trust_successor(
        fixture.trust_preimage,
        trust_ref,
        trust,
        "trust-checkpoint-missing-retirement",
        producer_incarnation_heads_root_sha256(std::slice::from_ref(&expected_head))
            .expect("successor root"),
        |_| {},
    );
    let mut admission = TestRecoveryAdmission::from_incarnations(&[
        TestRecoveredIncarnation {
            allocation_ref: &first_ref,
            allocation: &first,
            retirement: Some((&retirement_ref, &retirement)),
        },
        successor,
    ]);
    let error = recover_principal_producer_incarnation_frontier(
        &mut FakeProducerHeadReader {
            heads: vec![successor_physical],
            history,
        },
        binding(),
        CurrentPrincipalTrustHead {
            checkpoint_ref: &current_ref,
            checkpoint: &current,
        },
        1,
        &mut admission,
    )
    .expect_err("missing predecessor retirement must fail closed");

    assert!(matches!(
        error,
        PrincipalProducerIncarnationProjectionError::MissingPredecessor
    ));
}

#[test]
fn restart_recovery_preserves_a_retired_current_head() {
    let (fixture, _) = principal_trust_bootstrap::admitted_bootstrap_fixture();
    let atom = fixture.admitted.atom_view();
    let authority_ref = atom.authorities().producer_authority_ref();
    let authority = atom.authorities().producer_authority();
    let allocation_ref = ResidentRecordRefV1::parse("allocation-retired").expect("allocation ref");
    let allocation = admit_allocation(
        &producer_preimage(authority_ref, authority, 'a', 50),
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
    );
    let retirement_ref = ResidentRecordRefV1::parse("retirement-current").expect("retirement ref");
    let retirement = admit_retirement(&allocation_ref, &allocation, authority_ref, authority);
    let expected = ProducerIncarnationHeadV1::retired(&allocation_ref, &allocation, &retirement)
        .expect("retired semantic head");
    let (current_ref, current) = trust_checkpoint_for_heads(
        "trust-checkpoint-retired-head",
        std::slice::from_ref(&expected),
    );

    let recovered = recovered_frontier(
        &[TestRecoveredIncarnation {
            allocation_ref: &allocation_ref,
            allocation: &allocation,
            retirement: Some((&retirement_ref, &retirement)),
        }],
        &current_ref,
        &current,
    );

    assert_eq!(recovered.heads(), std::slice::from_ref(&expected));
}

#[test]
fn restart_recovery_replays_three_generations_before_admitting_the_current_head() {
    let lineage = two_generation_lineage();
    let (fixture, _) = principal_trust_bootstrap::admitted_bootstrap_fixture();
    let atom = fixture.admitted.atom_view();
    let authority_ref = atom.authorities().producer_authority_ref();
    let authority = atom.authorities().producer_authority();
    let second_retirement_ref =
        ResidentRecordRefV1::parse("retirement-b").expect("second retirement ref");
    let second_retirement = admit_retirement_at(
        &lineage.second_ref,
        &lineage.second,
        authority_ref,
        authority,
        "retirement-b",
        70,
    );
    let third_ref = ResidentRecordRefV1::parse("allocation-c").expect("third allocation ref");
    let mut third_preimage = producer_preimage(authority_ref, authority, 'c', 71);
    third_preimage.incarnation_id =
        ProducerIncarnationIdV1::parse("incarnation-c").expect("third incarnation id");
    third_preimage.previous_incarnation_ref = Some(lineage.second_ref.clone());
    third_preimage.previous_incarnation_sha256 =
        Some(lineage.second.canonical_incarnation_sha256().clone());
    let third = admit_allocation(
        &third_preimage,
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::retired(
            &lineage.second_ref,
            &lineage.second,
            &second_retirement,
        )
        .expect("second retired predecessor"),
    );
    let first = TestRecoveredIncarnation {
        allocation_ref: &lineage.first_ref,
        allocation: &lineage.first,
        retirement: Some((&lineage.retirement_ref, &lineage.retirement)),
    };
    let second = TestRecoveredIncarnation {
        allocation_ref: &lineage.second_ref,
        allocation: &lineage.second,
        retirement: Some((&second_retirement_ref, &second_retirement)),
    };
    let third_incarnation = TestRecoveredIncarnation {
        allocation_ref: &third_ref,
        allocation: &third,
        retirement: None,
    };
    let first_physical = physical_record_set(&first);
    let second_physical = physical_record_set(&second);
    let third_physical = physical_record_set(&third_incarnation);
    let history = BTreeMap::from([
        (
            first_physical.allocation().coordinate().clone(),
            first_physical,
        ),
        (
            second_physical.allocation().coordinate().clone(),
            second_physical,
        ),
    ]);
    let expected =
        ProducerIncarnationHeadV1::active(&third_ref, &third).expect("third semantic head");
    let (current_ref, current) = trust_checkpoint_for_heads(
        "trust-checkpoint-three-generations",
        std::slice::from_ref(&expected),
    );
    let mut admission =
        TestRecoveryAdmission::from_incarnations(&[first, second, third_incarnation]);

    let recovered = recover_principal_producer_incarnation_frontier(
        &mut FakeProducerHeadReader {
            heads: vec![third_physical],
            history,
        },
        binding(),
        CurrentPrincipalTrustHead {
            checkpoint_ref: &current_ref,
            checkpoint: &current,
        },
        1,
        &mut admission,
    )
    .expect("three-generation lineage recovers");

    assert_eq!(recovered.heads(), std::slice::from_ref(&expected));
}

#[test]
fn restart_recovery_rejects_an_absent_physical_predecessor() {
    let lineage = two_generation_lineage();
    let successor = TestRecoveredIncarnation {
        allocation_ref: &lineage.second_ref,
        allocation: &lineage.second,
        retirement: None,
    };
    let successor_physical = physical_record_set(&successor);
    let expected = ProducerIncarnationHeadV1::active(&lineage.second_ref, &lineage.second)
        .expect("successor head");
    let (current_ref, current) = trust_checkpoint_for_heads(
        "trust-checkpoint-absent-predecessor",
        std::slice::from_ref(&expected),
    );
    let first = TestRecoveredIncarnation {
        allocation_ref: &lineage.first_ref,
        allocation: &lineage.first,
        retirement: Some((&lineage.retirement_ref, &lineage.retirement)),
    };
    let mut admission = TestRecoveryAdmission::from_incarnations(&[first, successor]);

    let error = recover_principal_producer_incarnation_frontier(
        &mut FakeProducerHeadReader {
            heads: vec![successor_physical],
            history: BTreeMap::new(),
        },
        binding(),
        CurrentPrincipalTrustHead {
            checkpoint_ref: &current_ref,
            checkpoint: &current,
        },
        1,
        &mut admission,
    )
    .expect_err("an absent physical predecessor must fail closed");

    assert!(matches!(
        error,
        PrincipalProducerIncarnationProjectionError::MissingPredecessor
    ));
}

#[test]
fn restart_recovery_rejects_a_physical_predecessor_cycle() {
    let lineage = two_generation_lineage();
    let first = TestRecoveredIncarnation {
        allocation_ref: &lineage.first_ref,
        allocation: &lineage.first,
        retirement: Some((&lineage.retirement_ref, &lineage.retirement)),
    };
    let successor = TestRecoveredIncarnation {
        allocation_ref: &lineage.second_ref,
        allocation: &lineage.second,
        retirement: None,
    };
    let first_physical = physical_record_set(&first);
    let successor_physical = physical_record_set(&successor);
    let cyclic_first = PrincipalProducerIncarnationRecordSet::new(
        first_physical.head().clone(),
        Some(successor_physical.allocation().coordinate().clone()),
        first_physical.allocation().clone(),
        first_physical.retirement().cloned(),
    )
    .expect("cycle is representable only as physical traversal metadata");
    let expected = ProducerIncarnationHeadV1::active(&lineage.second_ref, &lineage.second)
        .expect("successor head");
    let (current_ref, current) = trust_checkpoint_for_heads(
        "trust-checkpoint-predecessor-cycle",
        std::slice::from_ref(&expected),
    );
    let mut admission = TestRecoveryAdmission::from_incarnations(&[first, successor]);

    let error = recover_principal_producer_incarnation_frontier(
        &mut FakeProducerHeadReader {
            heads: vec![successor_physical],
            history: BTreeMap::from([(
                cyclic_first.allocation().coordinate().clone(),
                cyclic_first,
            )]),
        },
        binding(),
        CurrentPrincipalTrustHead {
            checkpoint_ref: &current_ref,
            checkpoint: &current,
        },
        1,
        &mut admission,
    )
    .expect_err("a physical predecessor cycle must fail closed");

    assert!(matches!(
        error,
        PrincipalProducerIncarnationProjectionError::LineageCycle
    ));
}

#[test]
fn restart_recovery_rejects_principal_wide_producer_epoch_reuse() {
    let (fixture, _) = principal_trust_bootstrap::admitted_bootstrap_fixture();
    let atom = fixture.admitted.atom_view();
    let authority_ref = atom.authorities().producer_authority_ref();
    let authority = atom.authorities().producer_authority();
    let first_ref = ResidentRecordRefV1::parse("allocation-a").expect("first allocation ref");
    let first = admit_allocation(
        &producer_preimage(authority_ref, authority, 'a', 50),
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
    );
    let second_ref = ResidentRecordRefV1::parse("allocation-other").expect("second allocation ref");
    let mut second_preimage = producer_preimage(authority_ref, authority, 'a', 51);
    second_preimage.incarnation_id =
        ProducerIncarnationIdV1::parse("incarnation-other").expect("second incarnation id");
    second_preimage.producer_id = ProducerIdV1::parse("producer-2").expect("second producer");
    let second = admit_allocation(
        &second_preimage,
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
    );
    let incarnations = [
        TestRecoveredIncarnation {
            allocation_ref: &first_ref,
            allocation: &first,
            retirement: None,
        },
        TestRecoveredIncarnation {
            allocation_ref: &second_ref,
            allocation: &second,
            retirement: None,
        },
    ];
    let heads = incarnations
        .iter()
        .map(|incarnation| {
            ProducerIncarnationHeadV1::active(incarnation.allocation_ref, incarnation.allocation)
                .expect("semantic head")
        })
        .collect::<Vec<_>>();
    let (current_ref, current) =
        trust_checkpoint_for_heads("trust-checkpoint-duplicate-producer-epoch", &heads);

    let error = try_recovered_frontier(&incarnations, &current_ref, &current)
        .expect_err("principal-wide epoch reuse must fail closed");

    assert!(matches!(
        error,
        PrincipalProducerIncarnationProjectionError::DuplicateProducerEpoch
    ));
}

#[test]
fn restart_recovery_rejects_noncanonical_and_wrong_wire_allocation_bytes() {
    enum ExpectedStoredFailure {
        NonCanonical,
        WrongWire,
    }

    let (fixture, _) = principal_trust_bootstrap::admitted_bootstrap_fixture();
    let atom = fixture.admitted.atom_view();
    let authority_ref = atom.authorities().producer_authority_ref();
    let authority = atom.authorities().producer_authority();
    let allocation_ref = ResidentRecordRefV1::parse("allocation-a").expect("allocation ref");
    let allocation = admit_allocation(
        &producer_preimage(authority_ref, authority, 'a', 50),
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
    );
    let incarnation = TestRecoveredIncarnation {
        allocation_ref: &allocation_ref,
        allocation: &allocation,
        retirement: None,
    };
    let physical = physical_record_set(&incarnation);
    let expected =
        ProducerIncarnationHeadV1::active(&allocation_ref, &allocation).expect("semantic head");
    let (current_ref, current) = trust_checkpoint_for_heads(
        "trust-checkpoint-invalid-stored-bytes",
        std::slice::from_ref(&expected),
    );
    let value: serde_json::Value =
        serde_json::from_slice(physical.allocation().canonical_record_jcs().as_slice())
            .expect("canonical allocation value");
    let noncanonical = serde_json::to_vec_pretty(&value).expect("pretty allocation bytes");
    let mut wrong_wire = value;
    wrong_wire
        .as_object_mut()
        .expect("allocation object")
        .remove("producer_id");
    let wrong_wire = canon::to_canonical_string(&wrong_wire)
        .expect("canonical wrong-wire value")
        .into_bytes();

    for (bytes, expected_failure) in [
        (noncanonical, ExpectedStoredFailure::NonCanonical),
        (wrong_wire, ExpectedStoredFailure::WrongWire),
    ] {
        let stored = replace_allocation_bytes(&physical, bytes);
        let mut admission =
            TestRecoveryAdmission::from_incarnations(std::slice::from_ref(&incarnation));
        let error = recover_principal_producer_incarnation_frontier(
            &mut FakeProducerHeadReader {
                heads: vec![stored.clone()],
                history: BTreeMap::from([(stored.allocation().coordinate().clone(), stored)]),
            },
            binding(),
            CurrentPrincipalTrustHead {
                checkpoint_ref: &current_ref,
                checkpoint: &current,
            },
            1,
            &mut admission,
        )
        .expect_err("substituted stored bytes must fail closed");

        match expected_failure {
            ExpectedStoredFailure::NonCanonical => assert!(matches!(
                error,
                PrincipalProducerIncarnationProjectionError::NonCanonicalStoredRecord { .. }
            )),
            ExpectedStoredFailure::WrongWire => assert!(matches!(
                error,
                PrincipalProducerIncarnationProjectionError::StoredRecordDecode { .. }
            )),
        }
    }
}

#[test]
fn all_three_public_prepare_paths_bind_exact_owner_checkpoint_construction() {
    let (initial, control_preimages) =
        crate::producer_custody_fixture::InitialProducerFixture::new();
    let prepared = initial.prepare();
    let crate::producer_custody_fixture::InitialProducerFixture {
        fixture,
        allocation_ref,
        allocation,
        next_trust_ref,
        next_trust,
        next_control_ref,
        next_control,
        empty_frontier,
    } = initial;
    let atom = fixture.admitted.atom_view();
    let authority_ref = atom.authorities().producer_authority_ref();
    let authority = atom.authorities().producer_authority();
    let (trust_ref, trust) = atom.trust_checkpoint_entry();
    let (control_ref, control) = atom
        .initial_control_checkpoints()
        .next()
        .expect("initial control checkpoint");
    let next_heads = vec![
        ProducerIncarnationHeadV1::active(&allocation_ref, &allocation)
            .expect("active semantic head"),
    ];
    assert_eq!(
        prepared.candidate().kind(),
        PrincipalProducerIncarnationTransitionKind::InitialAllocation
    );

    let retirement_ref = ResidentRecordRefV1::parse("retirement-a").expect("retirement ref");
    let retirement = admit_retirement(&allocation_ref, &allocation, authority_ref, authority);
    let retired_heads =
        vec![
            ProducerIncarnationHeadV1::retired(&allocation_ref, &allocation, &retirement)
                .expect("retired semantic head"),
        ];
    let (trust3_ref, trust3) = trust_successor(
        fixture.trust_preimage.clone(),
        &next_trust_ref,
        &next_trust,
        "trust-checkpoint-3",
        producer_incarnation_heads_root_sha256(&retired_heads).expect("retired root"),
        |_| {},
    );
    let (control3_ref, control3) = control_successor(
        control_preimages[0].clone(),
        &next_control_ref,
        &next_control,
        "control-checkpoint-producer-3",
    );
    let retirement_verifier = |construction: &PrincipalProducerCheckpointConstruction<'_, '_>| {
        assert_eq!(
            construction.transition_kind(),
            PrincipalProducerIncarnationTransitionKind::Retirement
        );
        assert_eq!(
            construction
                .appended_retirement()
                .expect("appended retirement")
                .0,
            &retirement_ref
        );
        Ok(())
    };
    let active_frontier = recovered_frontier(
        &[TestRecoveredIncarnation {
            allocation_ref: &allocation_ref,
            allocation: &allocation,
            retirement: None,
        }],
        &next_trust_ref,
        &next_trust,
    );
    let _retirement_prepared = prepare_principal_producer_incarnation_retirement(
        AdmittedProducerIncarnation::new(&allocation_ref, &allocation),
        AdmittedProducerIncarnationRetirement::new(&retirement_ref, &retirement),
        PrincipalProducerIncarnationProjectionContext::new(
            &active_frontier,
            AdmittedPrincipalTrustCheckpointTransition::new(
                &next_trust_ref,
                &next_trust,
                &trust3_ref,
                &trust3,
            )
            .expect("retirement trust transition"),
            AdmittedProducerControlCheckpointTransition::new(
                &next_control_ref,
                &next_control,
                &control3_ref,
                &control3,
                &retirement_verifier,
            ),
        )
        .expect("retirement projection context"),
    )
    .expect("retirement public projection");

    let successor_ref = ResidentRecordRefV1::parse("allocation-b").expect("successor ref");
    let mut successor_preimage = producer_preimage(authority_ref, authority, 'b', 61);
    successor_preimage.previous_incarnation_ref = Some(allocation_ref.clone());
    successor_preimage.previous_incarnation_sha256 =
        Some(allocation.canonical_incarnation_sha256().clone());
    let successor = admit_allocation(
        &successor_preimage,
        authority_ref,
        authority,
        &ProducerIncarnationHeadAdmissionStateV1::retired(
            &allocation_ref,
            &allocation,
            &retirement,
        )
        .expect("retired head"),
    );
    let successor_heads = vec![
        ProducerIncarnationHeadV1::active(&successor_ref, &successor)
            .expect("successor semantic head"),
    ];
    let (trust4_ref, trust4) = trust_successor(
        fixture.trust_preimage.clone(),
        &trust3_ref,
        &trust3,
        "trust-checkpoint-4",
        producer_incarnation_heads_root_sha256(&successor_heads).expect("successor root"),
        |_| {},
    );
    let (control4_ref, control4) = control_successor(
        control_preimages[0].clone(),
        &control3_ref,
        &control3,
        "control-checkpoint-producer-4",
    );
    let successor_verifier = |construction: &PrincipalProducerCheckpointConstruction<'_, '_>| {
        assert_eq!(
            construction.transition_kind(),
            PrincipalProducerIncarnationTransitionKind::SuccessorAllocation
        );
        assert!(construction.predecessor_retirement().is_some());
        Ok(())
    };
    let retired_frontier = recovered_frontier(
        &[TestRecoveredIncarnation {
            allocation_ref: &allocation_ref,
            allocation: &allocation,
            retirement: Some((&retirement_ref, &retirement)),
        }],
        &trust3_ref,
        &trust3,
    );
    let _successor_prepared = prepare_principal_producer_incarnation_successor(
        AdmittedProducerIncarnation::new(&allocation_ref, &allocation),
        AdmittedProducerIncarnationRetirement::new(&retirement_ref, &retirement),
        AdmittedProducerIncarnation::new(&successor_ref, &successor),
        PrincipalProducerIncarnationProjectionContext::new(
            &retired_frontier,
            AdmittedPrincipalTrustCheckpointTransition::new(
                &trust3_ref,
                &trust3,
                &trust4_ref,
                &trust4,
            )
            .expect("successor trust transition"),
            AdmittedProducerControlCheckpointTransition::new(
                &control3_ref,
                &control3,
                &control4_ref,
                &control4,
                &successor_verifier,
            ),
        )
        .expect("successor projection context"),
    )
    .expect("successor public projection");

    let accepting = |_: &PrincipalProducerCheckpointConstruction<'_, '_>| Ok(());
    let root_error = try_recovered_frontier(
        &[TestRecoveredIncarnation {
            allocation_ref: &allocation_ref,
            allocation: &allocation,
            retirement: None,
        }],
        trust_ref,
        trust,
    )
    .expect_err("predecessor producer-root mismatch");
    assert!(matches!(
        root_error,
        PrincipalProducerIncarnationProjectionError::CurrentHeadSet
    ));

    let (wrong_root_trust_ref, wrong_root_trust) = trust_successor(
        fixture.trust_preimage.clone(),
        trust_ref,
        trust,
        "trust-checkpoint-wrong-producer-root",
        producer_incarnation_heads_root_sha256(&[]).expect("empty producer root"),
        |_| {},
    );
    let successor_root_error = prepare_initial_principal_producer_incarnation(
        AdmittedProducerIncarnation::new(&allocation_ref, &allocation),
        PrincipalProducerIncarnationProjectionContext::new(
            &empty_frontier,
            AdmittedPrincipalTrustCheckpointTransition::new(
                trust_ref,
                trust,
                &wrong_root_trust_ref,
                &wrong_root_trust,
            )
            .expect("wrong-root trust transition remains generically valid"),
            AdmittedProducerControlCheckpointTransition::new(
                control_ref,
                control,
                &next_control_ref,
                &next_control,
                &accepting,
            ),
        )
        .expect("wrong-root projection context"),
    )
    .expect_err("successor producer-root mismatch");
    assert!(matches!(
        successor_root_error,
        PrincipalProducerIncarnationProjectionError::TrustCheckpointClosure
    ));

    let (unstable_trust_ref, unstable_trust) = trust_successor(
        fixture.trust_preimage.clone(),
        trust_ref,
        trust,
        "trust-checkpoint-unstable",
        producer_incarnation_heads_root_sha256(&next_heads).expect("next producer root"),
        |preimage| {
            preimage.session_floor_epoch =
                PrincipalSessionFloorEpochV1::new(2).expect("changed session floor")
        },
    );
    let unstable_error = prepare_initial_principal_producer_incarnation(
        AdmittedProducerIncarnation::new(&allocation_ref, &allocation),
        PrincipalProducerIncarnationProjectionContext::new(
            &empty_frontier,
            AdmittedPrincipalTrustCheckpointTransition::new(
                trust_ref,
                trust,
                &unstable_trust_ref,
                &unstable_trust,
            )
            .expect("stable-frontier trust transition"),
            AdmittedProducerControlCheckpointTransition::new(
                control_ref,
                control,
                &next_control_ref,
                &next_control,
                &accepting,
            ),
        )
        .expect("unstable-frontier projection context"),
    )
    .expect_err("stable trust field changed");
    assert!(matches!(
        unstable_error,
        PrincipalProducerIncarnationProjectionError::TrustCheckpointClosure
    ));

    let wrong_trust_ref =
        ResidentRecordRefV1::parse("wrong-trust-predecessor").expect("wrong trust predecessor ref");
    let predecessor_error = AdmittedPrincipalTrustCheckpointTransition::new(
        &wrong_trust_ref,
        trust,
        &next_trust_ref,
        &next_trust,
    )
    .expect_err("trust predecessor mismatch");
    assert!(matches!(
        predecessor_error,
        InvalidResidentRecordV1::ContextualAdmission { .. }
    ));

    let rejecting = |_: &PrincipalProducerCheckpointConstruction<'_, '_>| {
        Err(PrincipalProducerCheckpointConstructionRejection)
    };
    let construction_error = prepare_initial_principal_producer_incarnation(
        AdmittedProducerIncarnation::new(&allocation_ref, &allocation),
        PrincipalProducerIncarnationProjectionContext::new(
            &empty_frontier,
            AdmittedPrincipalTrustCheckpointTransition::new(
                trust_ref,
                trust,
                &next_trust_ref,
                &next_trust,
            )
            .expect("construction-check trust transition"),
            AdmittedProducerControlCheckpointTransition::new(
                control_ref,
                control,
                &next_control_ref,
                &next_control,
                &rejecting,
            ),
        )
        .expect("construction-check projection context"),
    )
    .expect_err("owner construction mismatch");
    assert!(matches!(
        construction_error,
        PrincipalProducerIncarnationProjectionError::CheckpointConstruction(_)
    ));
}

fn prepared() -> PreparedPrincipalProducerIncarnationCustody {
    let next_producer = physical_producer_head();
    let candidate = PreparedPrincipalProducerIncarnationTransition::new(
        binding(),
        PrincipalProducerIncarnationTransitionKind::InitialAllocation,
        PrincipalProducerIncarnationExpectedFrontier::new(
            None,
            (trust_head(1), record("trust-1", '1')),
            (control_head(1), record("control-1", '5')),
        ),
        vec![
            record("allocation-a", 'a'),
            record("trust-2", '2'),
            record("control-2", '6'),
        ],
        PrincipalProducerIncarnationSuccessorFrontier::new(
            next_producer,
            trust_head(2),
            control_head(2),
        ),
    )
    .expect("closed producer candidate");
    PreparedPrincipalProducerIncarnationCustody::from_candidate(candidate, producer_control_store())
}

fn producer_control_store() -> pale_ports::AuthorityStoreCoordinate {
    pale_ports::AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("principal-control-store-1").expect("control store"),
        ControlIncarnationIdV1::parse("principal-control-incarnation-1")
            .expect("control incarnation"),
    )
}

#[test]
fn commit_and_replay_stop_at_database_committed_anchors_pending() {
    for (disposition, evidence) in [
        (
            PrincipalProducerIncarnationCommitDisposition::Committed,
            PrincipalProducerIncarnationCommitEvidence::New,
        ),
        (
            PrincipalProducerIncarnationCommitDisposition::ExactReplay,
            PrincipalProducerIncarnationCommitEvidence::ExactReplay,
        ),
    ] {
        let mut store = FakeStore {
            commit: Some(Ok(disposition)),
            observation: None,
            observation_calls: 0,
        };
        let committed = prepared().commit(&mut store).expect("database custody");
        assert_eq!(
            committed.phase(),
            PrincipalProducerIncarnationCustodyPhase::DatabaseCommittedAnchorsPending
        );
        assert_eq!(committed.evidence(), evidence);
        assert_eq!(store.observation_calls, 0);
    }
}

#[test]
fn ambiguous_commit_accepts_only_an_exact_successor_read_back() {
    let mut exact = FakeStore {
        commit: Some(Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        ))),
        observation: Some(Ok(
            PrincipalProducerIncarnationDatabaseObservation::ExactSuccessor,
        )),
        observation_calls: 0,
    };
    let committed = prepared().commit(&mut exact).expect("exact read-back");
    assert_eq!(
        committed.evidence(),
        PrincipalProducerIncarnationCommitEvidence::RecoveredExactAfterAmbiguousCommit
    );
    assert_eq!(exact.observation_calls, 1);

    for (observation, expected_recovery) in [
        (
            PrincipalProducerIncarnationDatabaseObservation::Predecessor,
            false,
        ),
        (
            PrincipalProducerIncarnationDatabaseObservation::ConflictingOrPartial,
            true,
        ),
    ] {
        let mut store = FakeStore {
            commit: Some(Err(ControlStoreError::new(
                ControlStoreErrorKind::OutcomeUnknown,
            ))),
            observation: Some(Ok(observation)),
            observation_calls: 0,
        };
        let failure = prepared().commit(&mut store).expect_err("must fail closed");
        assert_eq!(store.observation_calls, 1);
        let uncertain = match failure.reason() {
            PrincipalProducerIncarnationCommitFailureReason::CommitExcludedByExactReadBack {
                uncertain,
            } if !expected_recovery => uncertain,
            PrincipalProducerIncarnationCommitFailureReason::RecoveryRequiredAfterExactReadBack {
                uncertain,
            } if expected_recovery => uncertain,
            other => panic!("unexpected ambiguity disposition: {other}"),
        };
        assert_eq!(uncertain.kind(), ControlStoreErrorKind::OutcomeUnknown);
        assert_eq!(
            failure.into_prepared().phase(),
            PrincipalProducerIncarnationCustodyPhase::Prepared
        );
    }

    let mut failed_read_back = FakeStore {
        commit: Some(Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        ))),
        observation: Some(Err(ControlStoreError::new(ControlStoreErrorKind::Storage))),
        observation_calls: 0,
    };
    let failure = prepared()
        .commit(&mut failed_read_back)
        .expect_err("failed read-back cannot prove custody");
    match failure.reason() {
        PrincipalProducerIncarnationCommitFailureReason::ExactReadBackFailed {
            uncertain,
            read_back,
        } => {
            assert_eq!(uncertain.kind(), ControlStoreErrorKind::OutcomeUnknown);
            assert_eq!(read_back.kind(), ControlStoreErrorKind::Storage);
        }
        other => panic!("unexpected read-back failure: {other}"),
    }
}
