//! Historical principal control-checkpoint recovery laws.

use super::*;
use crate::{principal_trust_test_support, producer_record_fixture};
use pale_core::records::*;
use pale_ports::{
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecord, ControlRecordBytes,
    ControlRecordCoordinate, ControlRecordDigest, PrincipalControlCheckpointStoredRecord,
    PrincipalTrustStoreBinding,
};

struct Fixture {
    evidence: PrincipalControlCheckpointChainSnapshot,
    initial_ref: ResidentRecordRefV1,
    initial: ControlStateCheckpointV1,
    initial_preimage: ControlStateCheckpointPreimageV1,
    signer_ref: ResidentRecordRefV1,
    signer: CheckpointSigningAuthorityV1,
    schema: SchemaDigest,
    profile_content: Box<[u8]>,
}

fn make_fixture(successor: bool) -> Fixture {
    let profile_content = SupportedPrincipalTrustCheckpointProfile::canonical_content()
        .unwrap()
        .into_bytes()
        .into_boxed_slice();
    let profile_digest = SupportedPrincipalTrustCheckpointProfile::content_digest().unwrap();
    let profile_ref = ResidentRecordRefV1::parse("source-profile").unwrap();
    let profile = SupportedPrincipalTrustCheckpointProfile::admit(
        profile_ref.clone(),
        &profile_digest,
        &profile_content,
    )
    .unwrap();
    let trust_store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("trust-store-1").unwrap(),
        ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap(),
    );
    let control_store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("principal-control-store-1").unwrap(),
        ControlIncarnationIdV1::parse("principal-control-incarnation-1").unwrap(),
    );
    let schema = SchemaDigest::parse("8".repeat(64)).unwrap();
    let selection = profile.bootstrap_profile(&trust_store, &control_store, &schema);
    let bootstrap_id = PrincipalBootstrapIdV1::parse("recovery-bootstrap").unwrap();
    let authorities =
        principal_trust_test_support::admit_profile_authorities(&bootstrap_id, &selection).unwrap();
    let signer_ref = authorities.checkpoint_authority_ref().clone();
    let signer = authorities.checkpoint_authority().clone();
    let trust_preimage =
        principal_trust_test_support::initial_trust_preimage(&authorities, Some(&selection));
    let trust_digest = PrincipalTrustCheckpointDigest::hash_canonical(&trust_preimage).unwrap();
    let trust = authorities
        .admit_initial_trust_checkpoint(
            ResidentRecordRefV1::parse("recovery-trust").unwrap(),
            principal_trust_test_support::signed_wire(
                &trust_preimage,
                "canonical_trust_checkpoint_sha256",
                &trust_digest,
            ),
            &trust_preimage.schema_sha256,
            selection,
            |_, _, _, _, _| Ok(()),
        )
        .unwrap();
    let initial_preimage = ControlStateCheckpointPreimageV1 {
        schema_id: SchemaId::parse(CONTROL_STATE_CHECKPOINT_V1_SCHEMA).unwrap(),
        schema_sha256: schema.clone(),
        control_checkpoint_id: ControlCheckpointIdV1::parse("recovery-control-1").unwrap(),
        control_store_id: control_store.store_id().clone(),
        control_incarnation_id: control_store.incarnation_id().clone(),
        checkpoint_domain_id: ControlCheckpointDomainIdV1::parse("principal-trust").unwrap(),
        domain_epoch: ControlDomainEpochV1::new(1).unwrap(),
        domain_sequence: ControlDomainSequenceV1::new(1).unwrap(),
        previous_control_checkpoint_ref: None,
        previous_control_checkpoint_sha256: None,
        checkpoint_profile_ref: profile_ref,
        checkpoint_profile_sha256: profile_digest,
        control_schema_version: 1,
        committed_transaction_sequence: ControlTransactionSequenceV1::new(2).unwrap(),
        state_root_sha256: ArtifactDigest::parse("a".repeat(64)).unwrap(),
        leaf_count: ControlLeafCountV1::new(0).unwrap(),
        changed_leaf_keys: Vec::new(),
        unresolved_custody_root_sha256: ArtifactDigest::parse("b".repeat(64)).unwrap(),
        issued_at: UnixTimeV1::new(45).unwrap(),
        checkpoint_signing_authority_ref: signer_ref.clone(),
        checkpoint_signing_authority_sha256: signer
            .canonical_checkpoint_signing_authority_sha256()
            .clone(),
        issuer_principal_id: signer.principal_id().clone(),
        issuer_key_id: signer.subject_key_id().clone(),
        issuer_key_epoch: *signer.subject_key_epoch(),
    };
    let initial_digest = ControlStateCheckpointDigest::hash_canonical(&initial_preimage).unwrap();
    let initial_ref = ResidentRecordRefV1::parse("recovery-control-1").unwrap();
    let frontier = trust
        .admit_control_checkpoints(
            vec![(
                initial_ref.clone(),
                principal_trust_test_support::signed_wire(
                    &initial_preimage,
                    "canonical_control_checkpoint_sha256",
                    &initial_digest,
                ),
            )],
            |_, _, _, _, _| Ok(()),
            |prefixes, domain| {
                prefixes
                    .iter()
                    .any(|prefix| prefix.as_str() == domain.as_str())
            },
        )
        .unwrap();
    let (_, initial) = frontier.initial_control_checkpoints().next().unwrap();
    let initial = initial.clone();
    let initial_coordinate = record_projection::coordinate(
        &initial_ref,
        initial.canonical_control_checkpoint_sha256().as_str(),
    )
    .unwrap();
    let initial_record = record_projection::record(initial_coordinate.clone(), &initial).unwrap();
    let mut records = vec![PrincipalControlCheckpointStoredRecord::new(
        ControlTransactionSequenceV1::new(2).unwrap(),
        initial_record,
    )];
    let (through, observed_through) = if successor {
        let (next_ref, next) = producer_record_fixture::control_successor(
            initial_preimage.clone(),
            &initial_ref,
            &initial,
            "recovery-control-2",
        );
        let coordinate = record_projection::coordinate(
            &next_ref,
            next.canonical_control_checkpoint_sha256().as_str(),
        )
        .unwrap();
        records.push(PrincipalControlCheckpointStoredRecord::new(
            ControlTransactionSequenceV1::new(3).unwrap(),
            record_projection::record(coordinate.clone(), &next).unwrap(),
        ));
        (
            ControlCheckpointHead::new(
                next.checkpoint_domain_id().clone(),
                *next.domain_epoch(),
                *next.domain_sequence(),
                *next.committed_transaction_sequence(),
                coordinate,
            ),
            ControlTransactionSequenceV1::new(3).unwrap(),
        )
    } else {
        (
            ControlCheckpointHead::new(
                initial.checkpoint_domain_id().clone(),
                *initial.domain_epoch(),
                *initial.domain_sequence(),
                *initial.committed_transaction_sequence(),
                initial_coordinate.clone(),
            ),
            ControlTransactionSequenceV1::new(2).unwrap(),
        )
    };
    let binding = PrincipalTrustStoreBinding::new(
        trust_store.store_id().clone(),
        trust_store.incarnation_id().clone(),
        signer.principal_id().clone(),
    );
    Fixture {
        evidence: PrincipalControlCheckpointChainSnapshot::new(
            binding,
            ControlCheckpointHead::new(
                initial.checkpoint_domain_id().clone(),
                *initial.domain_epoch(),
                *initial.domain_sequence(),
                *initial.committed_transaction_sequence(),
                initial_coordinate,
            ),
            through,
            observed_through,
            records.into_boxed_slice(),
        )
        .unwrap(),
        initial_ref,
        initial,
        initial_preimage,
        signer_ref,
        signer,
        schema,
        profile_content,
    }
}

#[derive(Clone)]
struct SignerResolver {
    resolution: HistoricalCheckpointSignerResolution,
    covers: bool,
}

impl HistoricalCheckpointSignerResolver for SignerResolver {
    fn resolve_historical_checkpoint_signer(
        &mut self,
        _: &ResidentRecordRefV1,
        _: &CheckpointSigningAuthorityDigest,
    ) -> HistoricalCheckpointSignerResolution {
        self.resolution.clone()
    }

    fn signer_covers_domain(
        &mut self,
        _: &CheckpointSigningAuthorityV1,
        _: &ControlCheckpointDomainIdV1,
    ) -> bool {
        self.covers
    }
}

fn recover(
    fixture: Fixture,
    profile: HistoricalControlCheckpointProfileResolution,
    signer: HistoricalCheckpointSignerResolution,
    covers: bool,
    signature_ok: bool,
    construction: HistoricalAccumulatorConstructionResolution,
    limits: PrincipalControlCheckpointChainReadLimits,
) -> Result<RecoveredPrincipalControlCheckpointChain, PrincipalControlCheckpointChainRecoveryError>
{
    let mut profile_resolver =
        move |_: &ResidentRecordRefV1, _: &CheckpointProfileDigest| profile.clone();
    let mut signer_resolver = SignerResolver {
        resolution: signer,
        covers,
    };
    let mut signature = move |_: &[u8],
                              _: &Ed25519PublicKeyV1,
                              _: &ControlStateCheckpointPreimageV1,
                              _: &ControlStateCheckpointDigest,
                              _: &CanonicalSignatureV1| {
        if signature_ok {
            Ok(())
        } else {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "test checkpoint",
                invariant: "authenticated",
            })
        }
    };
    let mut construction_verifier =
        move |_: &SupportedPrincipalTrustCheckpointProfile,
              _: Option<(&ResidentRecordRefV1, &ControlStateCheckpointV1)>,
              _: &ResidentRecordRefV1,
              _: &ControlStateCheckpointV1| construction;
    recover_principal_control_checkpoint_chain(
        fixture.evidence,
        &fixture.initial_ref,
        &fixture.initial,
        &fixture.schema,
        limits,
        &mut profile_resolver,
        &mut signer_resolver,
        &mut signature,
        &mut construction_verifier,
    )
}

pub(in crate::resident) fn recovered_cold_start_control_chain(
    binding: PrincipalTrustStoreBinding,
    checkpoints: &[(&ResidentRecordRefV1, &ControlStateCheckpointV1)],
) -> RecoveredPrincipalControlCheckpointChain {
    assert!(!checkpoints.is_empty(), "test control chain is non-empty");
    let admitted = checkpoints
        .iter()
        .map(|(reference, checkpoint)| {
            let coordinate = record_projection::coordinate(
                reference,
                checkpoint.canonical_control_checkpoint_sha256().as_str(),
            )
            .unwrap();
            RecoveredPrincipalControlCheckpoint {
                coordinate,
                checkpoint: (*checkpoint).clone(),
            }
        })
        .collect::<Vec<_>>();
    let head = |checkpoint: &RecoveredPrincipalControlCheckpoint| {
        ControlCheckpointHead::new(
            checkpoint.checkpoint.checkpoint_domain_id().clone(),
            *checkpoint.checkpoint.domain_epoch(),
            *checkpoint.checkpoint.domain_sequence(),
            *checkpoint.checkpoint.committed_transaction_sequence(),
            checkpoint.coordinate.clone(),
        )
    };
    let records = admitted
        .iter()
        .map(|checkpoint| {
            PrincipalControlCheckpointStoredRecord::new(
                *checkpoint.checkpoint.committed_transaction_sequence(),
                record_projection::record(checkpoint.coordinate.clone(), &checkpoint.checkpoint)
                    .unwrap(),
            )
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let initial = head(admitted.first().unwrap());
    let through = head(admitted.last().unwrap());
    let evidence = PrincipalControlCheckpointChainSnapshot::new(
        binding,
        initial,
        through.clone(),
        through.committed_transaction_sequence(),
        records,
    )
    .unwrap();
    let content = SupportedPrincipalTrustCheckpointProfile::canonical_content().unwrap();
    let profile = SupportedPrincipalTrustCheckpointProfile::admit(
        ResidentRecordRefV1::parse("cold-start-test-control-profile").unwrap(),
        &SupportedPrincipalTrustCheckpointProfile::content_digest().unwrap(),
        content.as_bytes(),
    )
    .unwrap();
    RecoveredPrincipalControlCheckpointChain {
        evidence,
        profile,
        checkpoints: admitted.into_boxed_slice(),
    }
}

fn exact_signer(fixture: &Fixture) -> HistoricalCheckpointSignerResolution {
    HistoricalCheckpointSignerResolution::Resolved(Box::new(
        ResolvedHistoricalCheckpointSigner::new(fixture.signer_ref.clone(), fixture.signer.clone()),
    ))
}

fn stored_wire_record(
    preimage: &ControlStateCheckpointPreimageV1,
    reference: &str,
    inserted_in: u64,
) -> (
    PrincipalControlCheckpointStoredRecord,
    ControlCheckpointHead,
) {
    let digest = ControlStateCheckpointDigest::hash_canonical(preimage).unwrap();
    let wire: ControlStateCheckpointWireV1 = principal_trust_test_support::signed_wire(
        preimage,
        "canonical_control_checkpoint_sha256",
        &digest,
    );
    let coordinate = ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).unwrap(),
        ControlRecordDigest::parse(digest.as_str()).unwrap(),
    );
    let body = pale_core::canon::to_canonical_string(&wire)
        .unwrap()
        .into_bytes();
    (
        PrincipalControlCheckpointStoredRecord::new(
            ControlTransactionSequenceV1::new(inserted_in).unwrap(),
            ControlRecord::new(
                coordinate.clone(),
                preimage.schema_id.clone(),
                preimage.schema_sha256.clone(),
                ControlRecordBytes::new(body).unwrap(),
            ),
        ),
        ControlCheckpointHead::new(
            preimage.checkpoint_domain_id.clone(),
            preimage.domain_epoch,
            preimage.domain_sequence,
            ControlTransactionSequenceV1::new(inserted_in).unwrap(),
            coordinate,
        ),
    )
}

#[test]
fn recovery_admits_bootstrap_and_producer_continuation_without_inventing_batches() {
    let fixture = make_fixture(true);
    let profile =
        HistoricalControlCheckpointProfileResolution::Resolved(fixture.profile_content.clone());
    let signer = exact_signer(&fixture);
    let recovered = recover(
        fixture,
        profile,
        signer,
        true,
        true,
        HistoricalAccumulatorConstructionResolution::Verified,
        PrincipalControlCheckpointChainReadLimits::new(2, 16_384),
    )
    .unwrap();
    assert_eq!(recovered.checkpoints().len(), 2);
    assert_eq!(
        recovered
            .checkpoints()
            .last()
            .unwrap()
            .checkpoint
            .domain_sequence()
            .get(),
        2
    );
    assert_eq!(recovered.evidence().through().domain_sequence().get(), 2);
    assert!(recovered
        .profile()
        .supports_domain(recovered.evidence().through().checkpoint_domain_id()));
}

#[test]
fn recovery_distinguishes_profile_signer_and_construction_evidence_failures() {
    let cases = [
        (
            HistoricalControlCheckpointProfileResolution::Unavailable,
            HistoricalAccumulatorConstructionResolution::Verified,
            PrincipalControlCheckpointChainRecoveryError::ProfileUnavailable,
        ),
        (
            HistoricalControlCheckpointProfileResolution::Conflicting,
            HistoricalAccumulatorConstructionResolution::Verified,
            PrincipalControlCheckpointChainRecoveryError::ProfileConflict,
        ),
    ];
    for (profile, construction, expected) in cases {
        let fixture = make_fixture(false);
        let signer = exact_signer(&fixture);
        let error = recover(
            fixture,
            profile,
            signer,
            true,
            true,
            construction,
            PrincipalControlCheckpointChainReadLimits::new(1, 8192),
        )
        .unwrap_err();
        assert_eq!(
            std::mem::discriminant(&error),
            std::mem::discriminant(&expected)
        );
    }

    for (resolution, expected) in [
        (
            HistoricalCheckpointSignerResolution::Unavailable,
            PrincipalControlCheckpointChainRecoveryError::SignerUnavailable,
        ),
        (
            HistoricalCheckpointSignerResolution::Conflicting,
            PrincipalControlCheckpointChainRecoveryError::SignerConflict,
        ),
    ] {
        let fixture = make_fixture(false);
        let profile =
            HistoricalControlCheckpointProfileResolution::Resolved(fixture.profile_content.clone());
        let error = recover(
            fixture,
            profile,
            resolution,
            true,
            true,
            HistoricalAccumulatorConstructionResolution::Verified,
            PrincipalControlCheckpointChainReadLimits::new(1, 8192),
        )
        .unwrap_err();
        assert_eq!(
            std::mem::discriminant(&error),
            std::mem::discriminant(&expected)
        );
    }

    for (construction, expected) in [
        (
            HistoricalAccumulatorConstructionResolution::Unavailable,
            PrincipalControlCheckpointChainRecoveryError::ConstructionUnavailable,
        ),
        (
            HistoricalAccumulatorConstructionResolution::Conflicting,
            PrincipalControlCheckpointChainRecoveryError::ConstructionConflict,
        ),
    ] {
        let fixture = make_fixture(false);
        let profile =
            HistoricalControlCheckpointProfileResolution::Resolved(fixture.profile_content.clone());
        let signer = exact_signer(&fixture);
        let error = recover(
            fixture,
            profile,
            signer,
            true,
            true,
            construction,
            PrincipalControlCheckpointChainReadLimits::new(1, 8192),
        )
        .unwrap_err();
        assert_eq!(
            std::mem::discriminant(&error),
            std::mem::discriminant(&expected)
        );
    }
}

#[test]
fn recovery_rejects_uncovered_domain_signature_schema_profile_and_bounds() {
    let fixture = make_fixture(false);
    let profile =
        HistoricalControlCheckpointProfileResolution::Resolved(fixture.profile_content.clone());
    let signer = exact_signer(&fixture);
    assert!(matches!(
        recover(
            fixture,
            profile,
            signer,
            false,
            true,
            HistoricalAccumulatorConstructionResolution::Verified,
            PrincipalControlCheckpointChainReadLimits::new(1, 8192),
        ),
        Err(PrincipalControlCheckpointChainRecoveryError::SignerContext)
    ));

    let fixture = make_fixture(false);
    let profile =
        HistoricalControlCheckpointProfileResolution::Resolved(fixture.profile_content.clone());
    let signer = exact_signer(&fixture);
    assert!(matches!(
        recover(
            fixture,
            profile,
            signer,
            true,
            false,
            HistoricalAccumulatorConstructionResolution::Verified,
            PrincipalControlCheckpointChainReadLimits::new(1, 8192),
        ),
        Err(PrincipalControlCheckpointChainRecoveryError::Record(_))
    ));

    let fixture = make_fixture(false);
    let profile =
        HistoricalControlCheckpointProfileResolution::Resolved(b"{}".to_vec().into_boxed_slice());
    let signer = exact_signer(&fixture);
    assert!(matches!(
        recover(
            fixture,
            profile,
            signer,
            true,
            true,
            HistoricalAccumulatorConstructionResolution::Verified,
            PrincipalControlCheckpointChainReadLimits::new(1, 8192),
        ),
        Err(PrincipalControlCheckpointChainRecoveryError::UnsupportedProfile)
    ));

    let fixture = make_fixture(false);
    let profile =
        HistoricalControlCheckpointProfileResolution::Resolved(fixture.profile_content.clone());
    let signer = exact_signer(&fixture);
    assert!(matches!(
        recover(
            fixture,
            profile,
            signer,
            true,
            true,
            HistoricalAccumulatorConstructionResolution::Verified,
            PrincipalControlCheckpointChainReadLimits::new(0, 0),
        ),
        Err(PrincipalControlCheckpointChainRecoveryError::BoundExceeded)
    ));
}

#[test]
fn recovery_rejects_noncanonical_stored_bytes() {
    let mut fixture = make_fixture(false);
    let retained = &fixture.evidence.records()[0];
    let mut bytes = retained.record().canonical_record_jcs().as_slice().to_vec();
    bytes.push(b' ');
    let substituted = ControlRecord::new(
        retained.record().coordinate().clone(),
        retained.record().schema_id().clone(),
        retained.record().schema_sha256().clone(),
        ControlRecordBytes::new(bytes).unwrap(),
    );
    fixture.evidence = PrincipalControlCheckpointChainSnapshot::new(
        fixture.evidence.binding().clone(),
        fixture.evidence.initial().clone(),
        fixture.evidence.through().clone(),
        fixture.evidence.observed_through(),
        vec![PrincipalControlCheckpointStoredRecord::new(
            retained.inserted_in(),
            substituted,
        )]
        .into_boxed_slice(),
    )
    .unwrap();
    let profile =
        HistoricalControlCheckpointProfileResolution::Resolved(fixture.profile_content.clone());
    let signer = exact_signer(&fixture);
    assert!(recover(
        fixture,
        profile,
        signer,
        true,
        true,
        HistoricalAccumulatorConstructionResolution::Verified,
        PrincipalControlCheckpointChainReadLimits::new(1, 8192),
    )
    .is_err());
}

#[test]
fn recovery_rejects_canonical_bytes_under_a_substituted_coordinate() {
    let mut fixture = make_fixture(false);
    let retained = &fixture.evidence.records()[0];
    let substituted_coordinate = ControlRecordCoordinate::new(
        retained.record().coordinate().record_ref().clone(),
        ControlRecordDigest::parse("f".repeat(64)).unwrap(),
    );
    let substituted_record = ControlRecord::new(
        substituted_coordinate.clone(),
        retained.record().schema_id().clone(),
        retained.record().schema_sha256().clone(),
        retained.record().canonical_record_jcs().clone(),
    );
    let substituted_head = ControlCheckpointHead::new(
        fixture.evidence.initial().checkpoint_domain_id().clone(),
        fixture.evidence.initial().domain_epoch(),
        fixture.evidence.initial().domain_sequence(),
        fixture.evidence.initial().committed_transaction_sequence(),
        substituted_coordinate,
    );
    fixture.evidence = PrincipalControlCheckpointChainSnapshot::new(
        fixture.evidence.binding().clone(),
        substituted_head.clone(),
        substituted_head,
        fixture.evidence.observed_through(),
        vec![PrincipalControlCheckpointStoredRecord::new(
            retained.inserted_in(),
            substituted_record,
        )]
        .into_boxed_slice(),
    )
    .unwrap();
    let profile =
        HistoricalControlCheckpointProfileResolution::Resolved(fixture.profile_content.clone());
    let signer = exact_signer(&fixture);
    assert!(matches!(
        recover(
            fixture,
            profile,
            signer,
            true,
            true,
            HistoricalAccumulatorConstructionResolution::Verified,
            PrincipalControlCheckpointChainReadLimits::new(1, 8192),
        ),
        Err(PrincipalControlCheckpointChainRecoveryError::StoredRecordMismatch)
    ));
}

#[test]
fn recovery_rejects_a_substituted_admitted_bootstrap_head() {
    let mut fixture = make_fixture(false);
    let substituted_initial = ControlCheckpointHead::new(
        fixture.evidence.initial().checkpoint_domain_id().clone(),
        fixture.evidence.initial().domain_epoch(),
        fixture.evidence.initial().domain_sequence(),
        ControlTransactionSequenceV1::new(3).unwrap(),
        fixture.evidence.initial().coordinate().clone(),
    );
    fixture.evidence = PrincipalControlCheckpointChainSnapshot::new(
        fixture.evidence.binding().clone(),
        substituted_initial,
        fixture.evidence.through().clone(),
        ControlTransactionSequenceV1::new(3).unwrap(),
        fixture.evidence.records().to_vec().into_boxed_slice(),
    )
    .unwrap();
    let profile =
        HistoricalControlCheckpointProfileResolution::Resolved(fixture.profile_content.clone());
    let signer = exact_signer(&fixture);
    assert!(matches!(
        recover(
            fixture,
            profile,
            signer,
            true,
            true,
            HistoricalAccumulatorConstructionResolution::Verified,
            PrincipalControlCheckpointChainReadLimits::new(1, 8192),
        ),
        Err(PrincipalControlCheckpointChainRecoveryError::Chain)
    ));
}

#[test]
fn recovery_rejects_missing_sequence_and_same_position_unlike_identity() {
    let mut fixture = make_fixture(false);
    let initial_record = fixture.evidence.records()[0].clone();
    let mut gap = fixture.initial_preimage.clone();
    gap.control_checkpoint_id = ControlCheckpointIdV1::parse("recovery-control-3").unwrap();
    gap.domain_sequence = ControlDomainSequenceV1::new(3).unwrap();
    gap.previous_control_checkpoint_ref = Some(fixture.initial_ref.clone());
    gap.previous_control_checkpoint_sha256 = Some(
        fixture
            .initial
            .canonical_control_checkpoint_sha256()
            .clone(),
    );
    gap.committed_transaction_sequence = ControlTransactionSequenceV1::new(4).unwrap();
    gap.issued_at = UnixTimeV1::new(46).unwrap();
    let (gap_record, gap_head) = stored_wire_record(&gap, "recovery-control-3", 4);
    fixture.evidence = PrincipalControlCheckpointChainSnapshot::new(
        fixture.evidence.binding().clone(),
        fixture.evidence.initial().clone(),
        gap_head,
        ControlTransactionSequenceV1::new(4).unwrap(),
        vec![initial_record, gap_record].into_boxed_slice(),
    )
    .unwrap();
    let profile =
        HistoricalControlCheckpointProfileResolution::Resolved(fixture.profile_content.clone());
    let signer = exact_signer(&fixture);
    assert!(matches!(
        recover(
            fixture,
            profile,
            signer,
            true,
            true,
            HistoricalAccumulatorConstructionResolution::Verified,
            PrincipalControlCheckpointChainReadLimits::new(2, 16_384),
        ),
        Err(PrincipalControlCheckpointChainRecoveryError::Chain)
    ));

    let mut fixture = make_fixture(false);
    let initial_record = fixture.evidence.records()[0].clone();
    let mut unlike = fixture.initial_preimage.clone();
    unlike.control_checkpoint_id = ControlCheckpointIdV1::parse("unlike-control-1").unwrap();
    let (unlike_record, unlike_head) = stored_wire_record(&unlike, "unlike-control-1", 3);
    fixture.evidence = PrincipalControlCheckpointChainSnapshot::new(
        fixture.evidence.binding().clone(),
        fixture.evidence.initial().clone(),
        unlike_head,
        ControlTransactionSequenceV1::new(3).unwrap(),
        vec![initial_record, unlike_record].into_boxed_slice(),
    )
    .unwrap();
    let profile =
        HistoricalControlCheckpointProfileResolution::Resolved(fixture.profile_content.clone());
    let signer = exact_signer(&fixture);
    assert!(matches!(
        recover(
            fixture,
            profile,
            signer,
            true,
            true,
            HistoricalAccumulatorConstructionResolution::Verified,
            PrincipalControlCheckpointChainReadLimits::new(2, 16_384),
        ),
        Err(PrincipalControlCheckpointChainRecoveryError::Chain)
    ));
}

#[test]
fn recovery_rejects_wrong_schema_and_signer_time() {
    let mut fixture = make_fixture(false);
    fixture.schema = SchemaDigest::parse("9".repeat(64)).unwrap();
    let profile =
        HistoricalControlCheckpointProfileResolution::Resolved(fixture.profile_content.clone());
    let signer = exact_signer(&fixture);
    assert!(matches!(
        recover(
            fixture,
            profile,
            signer,
            true,
            true,
            HistoricalAccumulatorConstructionResolution::Verified,
            PrincipalControlCheckpointChainReadLimits::new(1, 8192),
        ),
        Err(PrincipalControlCheckpointChainRecoveryError::Record(_))
    ));

    let mut fixture = make_fixture(false);
    let mut expired = fixture.initial_preimage.clone();
    expired.issued_at = UnixTimeV1::new(fixture.signer.expires_at().get()).unwrap();
    let (record, head) = stored_wire_record(&expired, "expired-control-1", 2);
    fixture.evidence = PrincipalControlCheckpointChainSnapshot::new(
        fixture.evidence.binding().clone(),
        ControlCheckpointHead::new(
            expired.checkpoint_domain_id.clone(),
            expired.domain_epoch,
            expired.domain_sequence,
            ControlTransactionSequenceV1::new(2).unwrap(),
            record.record().coordinate().clone(),
        ),
        head,
        ControlTransactionSequenceV1::new(2).unwrap(),
        vec![record].into_boxed_slice(),
    )
    .unwrap();
    let profile =
        HistoricalControlCheckpointProfileResolution::Resolved(fixture.profile_content.clone());
    let signer = exact_signer(&fixture);
    assert!(matches!(
        recover(
            fixture,
            profile,
            signer,
            true,
            true,
            HistoricalAccumulatorConstructionResolution::Verified,
            PrincipalControlCheckpointChainReadLimits::new(1, 8192),
        ),
        Err(PrincipalControlCheckpointChainRecoveryError::SignerContext)
    ));
}
