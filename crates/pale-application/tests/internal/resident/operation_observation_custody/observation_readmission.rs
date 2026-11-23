//! Contextual start/append records consumed by actual cold observation recovery.
//! Custody and membership here are declared read-only test doubles, not native
//! SQLite/Merkle proof. Existing adapter tests own that separate proof boundary.

use super::super::{checkpoint_preparation, local_start};
use super::*;
use crate::local_operation_start_fixture::StartAdmission;
use crate::resident::{
    recover_operation_observation, OperationCheckpointAncestryLimits,
    OperationObservationReadmissionError as Error, OperationObservationRecoveryAdmission,
    OperationObservationRecoveryInput, OperationObservationRecoveryLimits,
    OperationObservationSelection,
};
use std::collections::BTreeMap;

struct ObservationStore {
    handle: ControlRecord,
    selected: Option<ControlRecord>,
    history: BTreeMap<ResidentRecordRefV1, ControlRecord>,
    checkpoint: ControlRecord,
    committing_checkpoint: ControlRecord,
    head: ControlCheckpointHead,
    expected_leaf: AccumulatorMembershipLeaf,
    fault: &'static str,
    indexes: usize,
    bodies: usize,
    heads: usize,
    proofs: usize,
}

impl ControlTransactionPort for ObservationStore {
    fn prepare_transaction(
        &mut self,
        _: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        panic!("Observe cannot reserve a transaction")
    }
}

impl OperationCustodyPort for ObservationStore {
    fn read_a2a_task_bindings(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::OperationA2aBindingScope,
        _: Option<&pale_ports::OperationA2aBindingCursor>,
        _: usize,
        _: usize,
    ) -> Result<pale_ports::OperationA2aBindings, pale_ports::ControlStoreError> {
        panic!("remote-task discovery is outside this fixture's tested journey")
    }

    fn read_publication_sources(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: Option<&OperationPublicationCursor>,
        _: usize,
        _: usize,
    ) -> Result<OperationPublicationSources, OperationPublicationReadError> {
        panic!("observation recovery does not discover publication sources")
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        maximum: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        self.indexes += 1;
        assert_eq!(store, &store_coordinate());
        let record = match query {
            OperationCustodyIndexQuery::OperationHandle(_) => Some(&self.handle),
            OperationCustodyIndexQuery::LatestObservation(_) => self.selected.as_ref(),
            OperationCustodyIndexQuery::ObservationCheckpoint(_) => {
                Some(&self.committing_checkpoint)
            }
            _ => panic!("Observe does not start or reselect an original request"),
        };
        Ok(match record {
            Some(record) => {
                assert!(record.canonical_record_jcs().as_slice().len() <= maximum);
                OperationCustodyIndexObservation::exact(query.clone(), record.clone()).unwrap()
            }
            None => OperationCustodyIndexObservation::absent(query.clone()),
        })
    }
    fn read_operation_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        self.bodies += 1;
        assert_eq!(store, &store_coordinate());
        let mut record = self.history.get(coordinate.record_ref()).cloned();
        if let Some(value) = &record {
            if value.canonical_record_jcs().as_slice().len() > maximum {
                assert_eq!(self.fault, "parent-bytes");
                assert_eq!(value.canonical_record_jcs().as_slice().len(), maximum + 1);
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
            assert_eq!(value.coordinate(), coordinate);
            if self.fault == "foreign-parent" {
                record = Some(ControlRecord::new(
                    ControlRecordCoordinate::new(
                        record_ref("foreign-parent"),
                        value.coordinate().record_sha256().clone(),
                    ),
                    value.schema_id().clone(),
                    value.schema_sha256().clone(),
                    value.canonical_record_jcs().clone(),
                ));
            }
        }
        Ok(record)
    }
    fn read_checkpoint_head(
        &mut self,
        store: &AuthorityStoreCoordinate,
        domain: &ControlCheckpointDomainIdV1,
        maximum: usize,
    ) -> Result<Option<(ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
        self.heads += 1;
        assert_eq!(store, &store_coordinate());
        assert_eq!(domain, self.head.checkpoint_domain_id());
        assert!(self.checkpoint.canonical_record_jcs().as_slice().len() <= maximum);
        Ok(Some((self.head.clone(), self.checkpoint.clone())))
    }
    fn verify_accumulator_membership(
        &mut self,
        request: &OperationAccumulatorMembership,
    ) -> Result<VerifiedOperationAccumulatorMembership, OperationAccumulatorPreparationError> {
        self.proofs += 1;
        assert_eq!(request.checkpoint().checkpoint(), &self.head);
        if self.fault == "stale-selected-leaf" {
            // Current expected leaf comes from revision two, selected index from one.
            assert_eq!(request.leaves().len(), 1);
            assert_eq!(request.leaves()[0].key(), self.expected_leaf.key());
            assert_ne!(request.leaves()[0].value(), self.expected_leaf.value());
            return Err(ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired).into());
        }
        assert_eq!(request.leaves(), std::slice::from_ref(&self.expected_leaf));
        if self.fault == "head-before-proof" {
            return Err(ControlStoreError::new(ControlStoreErrorKind::CandidateConflict).into());
        }
        let returned = if self.fault == "foreign-proof-limit" {
            OperationAccumulatorMembership::new(
                request.checkpoint().clone(),
                request.leaves().to_vec(),
                AccumulatorMembershipLimits::new(2, 10_000, 100_000).unwrap(),
                request.implementation(),
            )
            .unwrap()
        } else {
            request.clone()
        };
        Ok(VerifiedOperationAccumulatorMembership::new(returned))
    }
    fn prepare_accumulator(
        &mut self,
        _: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        panic!("Observe never prepares a mutation")
    }
    fn compare_and_retain_operation_lease(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("Observe never retains or renews a lease")
    }
    fn compare_and_commit(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("Observe never commits")
    }
    fn observe_exact(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        panic!("Observe never replays a candidate")
    }
}

fn change_body(
    record: &ControlRecord,
    change: impl FnOnce(&mut serde_json::Value),
) -> ControlRecord {
    let mut value =
        pale_core::canon::parse_strict(record.canonical_record_jcs().as_slice()).unwrap();
    change(&mut value);
    let bytes = pale_core::canon::to_canonical_string(&value)
        .unwrap()
        .into_bytes();
    assert_ne!(bytes, record.canonical_record_jcs().as_slice());
    ControlRecord::new(
        record.coordinate().clone(),
        record.schema_id().clone(),
        record.schema_sha256().clone(),
        ControlRecordBytes::new(bytes).unwrap(),
    )
}

fn rejection() -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission {
        record: "observation recovery fixture",
        invariant: "explicit rejected historical evidence",
    }
}

#[test]
fn cold_observe_admits_real_initial_and_successor_records_with_bounded_failure_stages() {
    cold_observe_cases(None);
}

#[test]
fn historical_observe_checks_descendants_and_current_membership_before_returning_a_member() {
    cold_observe_cases(Some(1));
    cold_observe_cases(Some(2));
}

fn cold_observe_cases(history_revision: Option<u64>) {
    cold_observe_cases_matching(history_revision, None);
}

#[test]
fn same_profile_current_checkpoint_without_the_selected_head_is_rejected() {
    cold_observe_cases_matching(None, Some("stale-selected-leaf"));
}

fn cold_observe_cases_matching(history_revision: Option<u64>, only_fault: Option<&str>) {
    let (generation, profile) = source_generation();
    let (foreign_observation, foreign_signer) = with_start(
        &generation,
        &profile,
        "foreign-observe-operation",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, _, owners, signs, retained| {
            assert_eq!((owners, signs), (1, 2));
            (
                prepared.unwrap().records()[2].record().clone(),
                retained.signing.clone(),
            )
        },
    );
    let mut faults = vec![
        "initial",
        "successor",
        "missing-selected",
        "missing-parent",
        "malformed",
        "tampered",
        "foreign-parent",
        "foreign-handle",
        "cycle",
        "revision-gap",
        "chain-count",
        "chain-bytes",
        "parent-bytes",
        "historical-authority",
        "state-evidence",
        "signature",
        "checkpoint-signature",
        "checkpoint-stamp",
        "anchor-before",
        "anchor-after",
        "anchor-changed",
        "head-before-proof",
        "stale-selected-leaf",
        "foreign-proof-limit",
        "checkpoint-ancestor-not-commit-proof",
        "checkpoint-parent-missing",
        "checkpoint-count",
        "checkpoint-bytes",
        "checkpoint-authority",
        "checkpoint-historical-signature",
        "checkpoint-returned-other",
        "checkpoint-exact-budgets",
    ];
    if history_revision.is_some() {
        faults.extend([
            "checkpoint-admission-count",
            "missing-member",
            "foreign-member",
            "changed-member-digest",
            "tampered-intermediate",
        ]);
    }
    if let Some(only_fault) = only_fault {
        faults.retain(|fault| *fault == only_fault);
        assert_eq!(faults, [only_fault]);
    }
    for fault in faults {
        with_start(
            &generation,
            &profile,
            fault,
            |_| {},
            StartAdmission::AcceptedOwnerModel,
            |prepared, request, owners, signs, retained| {
                assert_eq!((owners, signs), (1, 2));
                let prepared = prepared.unwrap();
                let records = prepared.records().to_vec();
                let mut captured = None;
                let mut native = checkpoint_preparation::PreparationOnly::new(
                    |request: &OperationAccumulatorPreparation| {
                        captured = Some(request.clone());
                        local_start::initial_response(request)
                    },
                    ControlTransactionSequenceV1::new(1).unwrap(),
                );
                let prepared = prepared
                    .prepare_checkpoint(
                        AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4096, 100_000)
                            .unwrap(),
                        &mut native,
                    )
                    .unwrap();
                let checkpoint_signs = Cell::new(0);
                let (physical, pending, mut checkpoint) =
                    local_start::issue_initial(prepared, request, |_| {}, &checkpoint_signs)
                        .unwrap();
                let mut committing = local_start::absent_start_store(request);
                let mut initial_anchor = local_start::initial_anchor();
                let database = physical
                    .commit(&mut committing, &mut initial_anchor, 1024 * 1024)
                    .unwrap();
                let mut head = database.candidate().checkpoint().successor().clone();
                let mut checkpoint_record =
                    database.candidate().checkpoint().successor_record().clone();
                let initial_checkpoint_record = checkpoint_record.clone();
                let operation = pending
                    .finish(database.advance_anchor(&mut initial_anchor).unwrap())
                    .unwrap_or_else(|_| panic!("exact start"));
                let original = records[2].record().clone();
                let (handle_ref, handle) = operation.handle();
                let first = issue_observation(
                    retained.signing,
                    handle_ref,
                    handle,
                    schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                    OperationObservationRevisionV1::new(1).unwrap(),
                    None,
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::Resolving,
                    },
                    handle.created_at().get(),
                );
                assert_eq!(
                    pale_core::canon::to_canonical_string(&first)
                        .unwrap()
                        .as_bytes(),
                    original.canonical_record_jcs().as_slice()
                );
                let is_initial = fault == "initial";
                let (mut reference, mut expected) = if is_initial {
                    (original.coordinate().record_ref().clone(), first.clone())
                } else {
                    // Existing helper performs the real contextual append/issuer and
                    // its declared custody/anchor test double before recovery.
                    (head, checkpoint_record, checkpoint) = replace_observation(
                        &operation,
                        &original,
                        &checkpoint,
                        retained.signing,
                        retained.session,
                    );
                    (
                        record_ref("recovery/observation-2"),
                        issue_observation(
                            retained.signing,
                            handle_ref,
                            handle,
                            schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                            OperationObservationRevisionV1::new(2).unwrap(),
                            Some((original.coordinate().record_ref(), &first)),
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::Resolving,
                            },
                            handle.created_at().get() + 1,
                        ),
                    )
                };
                let middle_reference = reference.clone();
                let middle_checkpoint_record = checkpoint_record.clone();
                let middle = expected.clone();
                let middle_record = crate::resident::record_projection::record(
                    crate::resident::record_projection::coordinate(
                        &middle_reference,
                        middle.canonical_operation_observation_sha256().as_str(),
                    )
                    .unwrap(),
                    &middle,
                )
                .unwrap();
                let has_third_revision = history_revision.is_some() && !is_initial;
                if has_third_revision {
                    (head, checkpoint_record, checkpoint) = append_observation(
                        &operation,
                        &middle_record,
                        &middle,
                        &checkpoint,
                        retained.signing,
                        retained.session,
                    );
                    reference = record_ref("recovery/observation-3");
                    expected = issue_observation(
                        retained.signing,
                        handle_ref,
                        handle,
                        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                        OperationObservationRevisionV1::new(3).unwrap(),
                        Some((&middle_reference, &middle)),
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::Resolving,
                        },
                        handle.created_at().get() + 2,
                    );
                }
                let selected = crate::resident::record_projection::record(
                    crate::resident::record_projection::coordinate(
                        &reference,
                        expected.canonical_operation_observation_sha256().as_str(),
                    )
                    .unwrap(),
                    &expected,
                )
                .unwrap();
                let expected_leaf = if is_initial {
                    // Compare with the actual initial preparation request, rather
                    // than producing a second recovery expectation from its helper.
                    let initial = captured.as_ref().unwrap();
                    let leaf =
                        crate::resident::checkpoint_profile::operation_observation_membership(
                            &store_coordinate(),
                            &checkpoint,
                            &reference,
                            &expected,
                        )
                        .unwrap();
                    assert!(initial
                        .state()
                        .changes()
                        .iter()
                        .any(|change| change.key() == leaf.key()
                            && change.successor() == Some(leaf.value())));
                    leaf
                } else {
                    let append = prepare_operation_observation_append(
                        OperationObservationTransition::new(
                            handle_ref,
                            handle,
                            if has_third_revision {
                                &middle_reference
                            } else {
                                original.coordinate().record_ref()
                            },
                            if has_third_revision { &middle } else { &first },
                            &reference,
                            &expected,
                        ),
                        &StartStore(retained.session),
                    )
                    .unwrap();
                    let change = crate::resident::checkpoint_profile::operation_observation_change(
                        &append,
                        &checkpoint,
                    )
                    .unwrap();
                    AccumulatorMembershipLeaf::new(change.key_sha256, change.successor_sha256)
                        .unwrap()
                };
                let total_bytes = selected.canonical_record_jcs().as_slice().len()
                    + if is_initial {
                        0
                    } else {
                        original.canonical_record_jcs().as_slice().len()
                    }
                    + if has_third_revision {
                        middle_record.canonical_record_jcs().as_slice().len()
                    } else {
                        0
                    };
                let expected_anchor =
                    AntiRollbackAnchor::ControlDomain(ControlDomainAntiRollbackAnchor::new(
                        store_coordinate().store_id().clone(),
                        store_coordinate().incarnation_id().clone(),
                        head.checkpoint_domain_id().clone(),
                        head.domain_epoch(),
                        head.domain_sequence(),
                        head.coordinate().clone(),
                    ));
                let foreign_anchor =
                    AntiRollbackAnchor::ControlDomain(ControlDomainAntiRollbackAnchor::new(
                        store_coordinate().store_id().clone(),
                        store_coordinate().incarnation_id().clone(),
                        ControlCheckpointDomainIdV1::parse("operation:foreign").unwrap(),
                        head.domain_epoch(),
                        head.domain_sequence(),
                        head.coordinate().clone(),
                    ));
                let mut anchor = ReadAnchor {
                    replies: [
                        if fault == "anchor-before" {
                            None
                        } else {
                            Some(expected_anchor.clone())
                        },
                        if fault == "anchor-after" {
                            None
                        } else if fault == "anchor-changed" {
                            Some(foreign_anchor)
                        } else {
                            Some(expected_anchor)
                        },
                    ]
                    .into(),
                    reads: 0,
                    unavailable: false,
                    trace: None,
                };
                let mut store = ObservationStore {
                    handle: records[1].record().clone(),
                    selected: Some(selected.clone()),
                    history: [(original.coordinate().record_ref().clone(), original.clone())]
                        .into(),
                    committing_checkpoint: checkpoint_record.clone(),
                    checkpoint: checkpoint_record,
                    head,
                    expected_leaf,
                    fault,
                    indexes: 0,
                    bodies: 0,
                    heads: 0,
                    proofs: 0,
                };
                store.history.insert(
                    initial_checkpoint_record.coordinate().record_ref().clone(),
                    initial_checkpoint_record.clone(),
                );
                if has_third_revision {
                    store.history.insert(
                        middle_checkpoint_record.coordinate().record_ref().clone(),
                        middle_checkpoint_record.clone(),
                    );
                }
                if has_third_revision {
                    assert!(store
                        .history
                        .insert(middle_reference.clone(), middle_record.clone())
                        .is_none());
                }
                match fault {
                    "foreign-handle" => store.selected = Some(foreign_observation.clone()),
                    "stale-selected-leaf" => {
                        store.selected = Some(original.clone());
                        store.committing_checkpoint = initial_checkpoint_record.clone();
                    }
                    "checkpoint-ancestor-not-commit-proof"
                    | "checkpoint-count"
                    | "checkpoint-parent-missing"
                    | "checkpoint-authority"
                    | "checkpoint-historical-signature"
                    | "checkpoint-returned-other"
                    | "checkpoint-exact-budgets"
                    | "checkpoint-admission-count" => {
                        // A genuine earlier checkpoint is an ancestor, not proof
                        // that this later observation was committed with it.
                        store.committing_checkpoint = initial_checkpoint_record.clone();
                        if fault == "checkpoint-parent-missing" {
                            store
                                .history
                                .remove(initial_checkpoint_record.coordinate().record_ref());
                        }
                    }
                    "missing-selected" => store.selected = None,
                    "missing-parent" => {
                        assert!(store
                            .history
                            .remove(original.coordinate().record_ref())
                            .is_some());
                    }
                    "malformed" => {
                        store.selected = Some(ControlRecord::new(
                            selected.coordinate().clone(),
                            selected.schema_id().clone(),
                            selected.schema_sha256().clone(),
                            ControlRecordBytes::new(b"not-json".to_vec()).unwrap(),
                        ))
                    }
                    "tampered" => {
                        store.selected = Some(change_body(&selected, |value| {
                            value["observed_at"] =
                                serde_json::json!(expected.observed_at().get() + 1)
                        }))
                    }
                    "tampered-intermediate" => {
                        store.history.insert(
                            middle_reference.clone(),
                            change_body(&middle_record, |value| {
                                value["observed_at"] =
                                    serde_json::json!(middle.observed_at().get() + 1);
                            }),
                        );
                    }
                    "revision-gap" => {
                        store.selected = Some(change_body(&selected, |value| {
                            value["revision"] = serde_json::json!(expected.revision().get() + 1)
                        }))
                    }
                    "cycle" => {
                        let cycle = change_body(&selected, |value| {
                            value["predecessor_ref"] = serde_json::json!(reference.as_str());
                            value["predecessor_sha256"] =
                                serde_json::json!(selected.coordinate().record_sha256().as_str());
                        });
                        assert!(store
                            .history
                            .insert(reference.clone(), cycle.clone())
                            .is_none());
                        store.selected = Some(cycle);
                    }
                    "checkpoint-stamp" => {
                        store.head = ControlCheckpointHead::new(
                            store.head.checkpoint_domain_id().clone(),
                            store.head.domain_epoch(),
                            store.head.domain_sequence(),
                            ControlTransactionSequenceV1::new(
                                store.head.committed_transaction_sequence().get() + 1,
                            )
                            .unwrap(),
                            store.head.coordinate().clone(),
                        )
                    }
                    _ => {}
                }
                let signer_calls = Cell::new(0);
                let state_calls = Cell::new(0);
                let signature_calls = Cell::new(0);
                let checkpoint_calls = Cell::new(0);
                let ancestry_calls = Cell::new(0);
                let exact_checkpoint_bytes = 2 * initial_checkpoint_record
                    .canonical_record_jcs()
                    .as_slice()
                    .len()
                    + store.checkpoint.canonical_record_jcs().as_slice().len()
                    + if has_third_revision {
                        middle_checkpoint_record
                            .canonical_record_jcs()
                            .as_slice()
                            .len()
                    } else {
                        0
                    };
                let authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
                    authority_ref: checkpoint.checkpoint_signing_authority_ref(),
                    authority_sha256: checkpoint.checkpoint_signing_authority_sha256(),
                    principal_id: checkpoint.issuer_principal_id(),
                    control_store_id: checkpoint.control_store_id(),
                    control_incarnation_id: checkpoint.control_incarnation_id(),
                    checkpoint_domain_id: checkpoint.checkpoint_domain_id(),
                    checkpoint_profile_ref: checkpoint.checkpoint_profile_ref(),
                    checkpoint_profile_sha256: checkpoint.checkpoint_profile_sha256(),
                    subject_key_id: checkpoint.issuer_key_id(),
                    subject_key_epoch: *checkpoint.issuer_key_epoch(),
                };
                let chain_bytes = if fault == "chain-bytes" {
                    selected.canonical_record_jcs().as_slice().len() - 1
                } else if fault == "parent-bytes" {
                    total_bytes - 1
                } else if ["tampered", "cycle", "revision-gap"].contains(&fault) {
                    2 * (store
                        .selected
                        .as_ref()
                        .unwrap()
                        .canonical_record_jcs()
                        .as_slice()
                        .len()
                        + original.canonical_record_jcs().as_slice().len()
                        + if has_third_revision {
                            middle_record.canonical_record_jcs().as_slice().len()
                        } else {
                            0
                        })
                } else {
                    total_bytes
                };
                let limits = OperationObservationRecoveryLimits::new(
                    if fault == "chain-count" || is_initial {
                        1
                    } else {
                        expected.revision().get() as usize
                    },
                    chain_bytes,
                    1024 * 1024,
                )
                .unwrap();
                let (mut requested_reference, requested_observation) =
                    if history_revision == Some(2) && !is_initial {
                        (middle_reference.clone(), middle.clone())
                    } else {
                        (original.coordinate().record_ref().clone(), first.clone())
                    };
                let mut requested_digest = requested_observation
                    .canonical_operation_observation_sha256()
                    .clone();
                match fault {
                    "missing-member" => requested_reference = record_ref("missing/history-member"),
                    "foreign-member" => {
                        requested_reference = foreign_observation.coordinate().record_ref().clone();
                        requested_digest = OperationObservationDigest::parse(
                            foreign_observation
                                .coordinate()
                                .record_sha256()
                                .as_str()
                                .to_owned(),
                        )
                        .unwrap();
                        assert_ne!(requested_reference, *original.coordinate().record_ref());
                        assert_ne!(requested_reference, middle_reference);
                    }
                    "changed-member-digest" => {
                        requested_digest = expected.canonical_operation_observation_sha256().clone()
                    }
                    _ => {}
                }
                let result = recover_operation_observation(
                    OperationObservationRecoveryInput {
                        store: &store_coordinate(),
                        operation: &operation,
                        observation: if history_revision.is_some() {
                            OperationObservationSelection::HistoryMember {
                                reference: &requested_reference,
                                digest: &requested_digest,
                            }
                        } else {
                            OperationObservationSelection::SelectedHead
                        },
                        checkpoint_authority: &authority,
                        limits,
                        checkpoint_limits: OperationCheckpointAncestryLimits::new(
                            if fault == "checkpoint-count" {
                                1
                            } else if fault == "checkpoint-exact-budgets" {
                                if has_third_revision {
                                    3
                                } else {
                                    2
                                }
                            } else {
                                8
                            },
                            if fault == "checkpoint-bytes" {
                                1
                            } else if fault == "checkpoint-exact-budgets" {
                                exact_checkpoint_bytes
                            } else {
                                1 << 20
                            },
                            if fault == "checkpoint-admission-count" {
                                1
                            } else if fault == "checkpoint-exact-budgets" {
                                if has_third_revision {
                                    2
                                } else {
                                    1
                                }
                            } else {
                                8
                            },
                        )
                        .unwrap(),
                        membership_limits: AccumulatorMembershipLimits::new(1, 10_000, 100_000)
                            .unwrap(),
                    },
                    &mut store,
                    &mut anchor,
                    OperationObservationRecoveryAdmission {
                        admit_checkpoint:
                            |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                                ancestry_calls.set(ancestry_calls.get() + 1);
                                if fault == "checkpoint-authority" {
                                    return Err(rejection());
                                }
                                if fault == "checkpoint-returned-other" {
                                    return Ok(checkpoint.clone());
                                }
                                ControlStateCheckpointV1::admit_signed_wire(
                                    wire,
                                    &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                    &authority,
                                    |domain, _, _, actual| {
                                        assert_eq!(
                                            domain,
                                            CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN
                                        );
                                        assert_eq!(actual, &signature());
                                        if fault == "checkpoint-historical-signature" {
                                            Err(rejection())
                                        } else {
                                            Ok(())
                                        }
                                    },
                                )
                            },
                        resolve_signer:
                            |_: &ResidentRecordRefV1, wire: &OperationObservationWireV1| {
                                signer_calls.set(signer_calls.get() + 1);
                                if fault == "foreign-handle" {
                                    return Ok(foreign_signer.clone());
                                }
                                assert_eq!(
                                    wire.deployment_runtime_authority_ref(),
                                    first.deployment_runtime_authority_ref()
                                );
                                assert_eq!(
                                    wire.deployment_runtime_authority_sha256(),
                                    first.deployment_runtime_authority_sha256()
                                );
                                assert_eq!(wire.issuer_principal_id(), first.issuer_principal_id());
                                assert_eq!(wire.issuer_key_id(), first.issuer_key_id());
                                assert_eq!(wire.issuer_key_epoch(), first.issuer_key_epoch());
                                assert!(wire.observed_at() >= handle.created_at());
                                if fault == "historical-authority" && signer_calls.get() == 2 {
                                    Err(rejection())
                                } else {
                                    Ok(retained.signing.clone())
                                }
                            },
                        admit_state: |state: &OperationObservationStateV1| {
                            state_calls.set(state_calls.get() + 1);
                            assert!(matches!(
                                state,
                                OperationObservationStateV1::Active {
                                    phase: OperationActivePhaseV1::Resolving
                                }
                            ));
                            if fault == "state-evidence" && state_calls.get() == 2 {
                                Err(rejection())
                            } else {
                                Ok(())
                            }
                        },
                        admit_rotation:
                            |_: &ResidentRecordRefV1,
                             _: &ArtifactDigest,
                             _: &ResidentRecordRefV1,
                             _: &ArtifactDigest| {
                                panic!("this admitted fixture never rotates runtime authority")
                            },
                        verify_signature:
                            |domain: &[u8],
                             _: &OperationObservationPreimageV1,
                             _: &OperationObservationDigest,
                             actual: &CanonicalSignatureV1| {
                                signature_calls.set(signature_calls.get() + 1);
                                assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                                assert_eq!(actual, &signature());
                                if fault == "signature" && signature_calls.get() == 2 {
                                    Err(rejection())
                                } else {
                                    Ok(())
                                }
                            },
                    },
                    |domain, _, _, actual| {
                        checkpoint_calls.set(checkpoint_calls.get() + 1);
                        assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                        assert_eq!(actual, &signature());
                        if fault == "checkpoint-signature" {
                            Err(rejection())
                        } else {
                            Ok(())
                        }
                    },
                );
                match fault {
                    "initial"
                    | "successor"
                    | "checkpoint-ancestor-not-commit-proof"
                    | "checkpoint-exact-budgets" => {
                        let actual = result.unwrap();
                        assert_eq!(
                            actual.selected(),
                            if history_revision.is_some() {
                                (&requested_reference, &requested_observation)
                            } else {
                                (&reference, &expected)
                            }
                        );
                        assert_eq!(actual.selected_head(), (&reference, &expected));
                        assert_eq!(actual.store(), &store_coordinate());
                        if history_revision.is_some() && has_third_revision {
                            assert_ne!(actual.selected().0, actual.selected_head().0);
                            assert!(
                                actual.selected().1.revision().get()
                                    < actual.selected_head().1.revision().get()
                            );
                        }
                        assert_eq!((store.proofs, anchor.reads), (1, 2));
                    }
                    "missing-selected" => assert!(matches!(
                        result,
                        Err(Error::Lookup(
                            OperationRecoveryError::IncompleteObservationIndexes
                        ))
                    )),
                    "missing-parent" => assert!(matches!(result, Err(Error::Missing))),
                    "missing-member" | "foreign-member" | "changed-member-digest" => {
                        assert!(matches!(result, Err(Error::Missing)));
                        assert_eq!((store.proofs, anchor.reads), (1, 2));
                        assert_eq!(
                            (
                                signer_calls.get(),
                                state_calls.get(),
                                signature_calls.get(),
                                checkpoint_calls.get()
                            ),
                            (3, 3, 3, 1)
                        );
                    }
                    "malformed" => assert!(matches!(result, Err(Error::Canonical(_)))),
                    "tampered" | "tampered-intermediate" => {
                        assert!(matches!(result, Err(Error::Record(_))))
                    }
                    "foreign-parent" => assert!(matches!(result, Err(Error::Association))),
                    "foreign-handle" => assert!(matches!(
                        result,
                        Err(Error::Record(
                            InvalidResidentRecordV1::ContextualAdmission {
                                record: "operation observation",
                                invariant: "exact retained handle"
                            }
                        ))
                    )),
                    "parent-bytes" => assert!(
                        matches!(result, Err(Error::Custody(error)) if error.kind() == ControlStoreErrorKind::RecoveryRequired)
                    ),
                    "cycle" | "revision-gap" => assert!(matches!(result, Err(Error::Lineage))),
                    "chain-count" | "chain-bytes" => assert!(matches!(result, Err(Error::Bounds))),
                    "historical-authority" => {
                        assert!(matches!(result, Err(Error::HistoricalAuthority(_))))
                    }
                    "state-evidence" => assert!(matches!(result, Err(Error::StateEvidence(_)))),
                    "signature" => assert!(matches!(result, Err(Error::Signature(_)))),
                    "checkpoint-signature" => assert!(matches!(
                        result,
                        Err(Error::Checkpoint(LocalOperationRecoveryError::Record(_)))
                    )),
                    "checkpoint-authority" | "checkpoint-historical-signature" => {
                        assert!(matches!(
                            result,
                            Err(Error::Checkpoint(LocalOperationRecoveryError::Record(_)))
                        ))
                    }
                    "checkpoint-parent-missing" => assert!(matches!(
                        result,
                        Err(Error::Checkpoint(LocalOperationRecoveryError::Missing))
                    )),
                    "checkpoint-returned-other" => assert!(matches!(
                        result,
                        Err(Error::Checkpoint(LocalOperationRecoveryError::Association))
                    )),
                    "checkpoint-admission-count" => {
                        assert!(matches!(
                            result,
                            Err(Error::Checkpoint(LocalOperationRecoveryError::Bounds))
                        ));
                        assert_eq!(ancestry_calls.get(), 1);
                    }
                    "checkpoint-count" | "checkpoint-bytes" => {
                        assert!(matches!(
                            result,
                            Err(Error::Checkpoint(LocalOperationRecoveryError::Bounds))
                        ));
                        assert_eq!(ancestry_calls.get(), 0);
                    }
                    "checkpoint-stamp" | "anchor-changed" | "foreign-proof-limit" => {
                        assert!(matches!(
                            result,
                            Err(Error::Checkpoint(LocalOperationRecoveryError::Association))
                        ))
                    }
                    "anchor-before" | "anchor-after" => assert!(matches!(
                        result,
                        Err(Error::Checkpoint(LocalOperationRecoveryError::Missing))
                    )),
                    "head-before-proof" | "stale-selected-leaf" => assert!(matches!(
                        result,
                        Err(Error::Checkpoint(LocalOperationRecoveryError::Membership(
                            _
                        )))
                    )),
                    _ => unreachable!(),
                }
                if [
                    "missing-selected",
                    "missing-parent",
                    "malformed",
                    "foreign-parent",
                    "cycle",
                    "revision-gap",
                    "chain-count",
                    "chain-bytes",
                    "parent-bytes",
                ]
                .contains(&fault)
                {
                    assert_eq!(
                        (
                            signer_calls.get(),
                            state_calls.get(),
                            signature_calls.get(),
                            store.heads,
                            store.proofs,
                            anchor.reads
                        ),
                        (0, 0, 0, 0, 0, 0),
                        "{fault}"
                    );
                }
                if [
                    "chain-count",
                    "chain-bytes",
                    "malformed",
                    "missing-selected",
                    "initial",
                    "foreign-handle",
                ]
                .contains(&fault)
                {
                    assert_eq!(store.bodies, 0, "{fault}");
                } else if fault == "stale-selected-leaf" {
                    assert_eq!(
                        store.bodies,
                        if has_third_revision { 2 } else { 1 },
                        "{fault}"
                    );
                } else {
                    assert_eq!(
                        store.bodies,
                        (if has_third_revision
                            && !["foreign-parent", "cycle", "revision-gap"].contains(&fault)
                        {
                            2
                        } else {
                            1
                        }) + if [
                            "checkpoint-ancestor-not-commit-proof",
                            "checkpoint-parent-missing",
                            "checkpoint-exact-budgets"
                        ]
                        .contains(&fault)
                        {
                            if has_third_revision {
                                2
                            } else {
                                1
                            }
                        } else if [
                            "checkpoint-authority",
                            "checkpoint-historical-signature",
                            "checkpoint-returned-other",
                            "checkpoint-admission-count"
                        ]
                        .contains(&fault)
                        {
                            1
                        } else {
                            0
                        },
                        "{fault}"
                    );
                }
                if fault == "historical-authority" {
                    assert_eq!(
                        (signer_calls.get(), state_calls.get(), signature_calls.get()),
                        (2, 1, 1)
                    );
                }
                if fault == "foreign-handle" {
                    assert_eq!(
                        (
                            signer_calls.get(),
                            state_calls.get(),
                            signature_calls.get(),
                            store.heads,
                            store.proofs,
                            anchor.reads
                        ),
                        (1, 0, 0, 0, 0, 0)
                    );
                }
                if fault == "state-evidence" {
                    assert_eq!(
                        (signer_calls.get(), state_calls.get(), signature_calls.get()),
                        (2, 2, 1)
                    );
                }
                if fault == "signature" {
                    assert_eq!(
                        (signer_calls.get(), state_calls.get(), signature_calls.get()),
                        (2, 2, 2)
                    );
                }
                if ["initial", "successor"].contains(&fault) {
                    let expected_calls = expected.revision().get() as usize;
                    assert_eq!(
                        (
                            signer_calls.get(),
                            state_calls.get(),
                            signature_calls.get(),
                            checkpoint_calls.get()
                        ),
                        (expected_calls, expected_calls, expected_calls, 1)
                    );
                }
                if ["checkpoint-stamp", "checkpoint-signature"].contains(&fault) {
                    assert_eq!((store.proofs, anchor.reads), (0, 0));
                }
                if fault == "anchor-before" {
                    assert_eq!((store.proofs, anchor.reads), (0, 1));
                }
                if [
                    "head-before-proof",
                    "stale-selected-leaf",
                    "foreign-proof-limit",
                ]
                .contains(&fault)
                {
                    assert_eq!((store.proofs, anchor.reads), (1, 1));
                }
                if ["anchor-after", "anchor-changed"].contains(&fault) {
                    assert_eq!((store.proofs, anchor.reads), (1, 2));
                }
                assert_eq!(
                    (
                        committing.commits,
                        initial_anchor.advances,
                        checkpoint_signs.get()
                    ),
                    (1, 1, 1)
                );
                assert_eq!(operation.handle(), (handle_ref, handle));
            },
        );
    }
}

#[test]
fn zero_history_limits_are_rejected_without_a_recovery_attempt() {
    for (revisions, bytes, body) in [(0, 1, 1), (1, 0, 1), (1, 1, 0)] {
        assert!(matches!(
            OperationObservationRecoveryLimits::new(revisions, bytes, body),
            Err(Error::Bounds)
        ));
    }
}
