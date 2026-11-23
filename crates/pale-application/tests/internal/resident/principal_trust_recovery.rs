//! Signed historical trust-checkpoint recovery tests.

use super::*;
use crate::{deployment_finalization_fixture, principal_trust_test_support};
use pale_core::records::{
    ControlIncarnationIdV1, ControlStoreIdV1, ControlTransactionSequenceV1, ResidentPrincipalIdV1,
    ResidentRecordRefV1, SchemaDigest, PRINCIPAL_TRUST_CHECKPOINT_V1_SIGNATURE_DOMAIN,
};
use pale_ports::{
    ControlRecordBytes, ControlRecordCoordinate, ControlRecordDigest,
    PrincipalTrustCheckpointRecord, PrincipalTrustStoreBinding,
};

struct Fixture {
    evidence: PrincipalTrustCheckpointSetSnapshot,
    initial_ref: ResidentRecordRefV1,
    initial: PrincipalTrustCheckpointV1,
    schema: SchemaDigest,
    key: Ed25519PublicKeyV1,
}

fn fixture() -> Fixture {
    let (fixture, _) = principal_trust_test_support::admitted_bootstrap_fixture();
    let view = fixture.admitted.atom_view();
    let (reference, checkpoint) = view.trust_checkpoint_entry();
    let coordinate = record_projection::coordinate(
        reference,
        checkpoint.canonical_trust_checkpoint_sha256().as_str(),
    )
    .unwrap();
    let record = record_projection::record(coordinate.clone(), checkpoint).unwrap();
    let binding = PrincipalTrustStoreBinding::new(
        checkpoint.trust_store_id().clone(),
        checkpoint.trust_store_incarnation_id().clone(),
        checkpoint.principal_id().clone(),
    );
    let head = PrincipalTrustCheckpointHead::new(
        checkpoint.principal_id().clone(),
        *checkpoint.checkpoint_epoch(),
        coordinate,
    );
    Fixture {
        evidence: PrincipalTrustCheckpointSetSnapshot::new(
            binding,
            head,
            ControlTransactionSequenceV1::new(1).unwrap(),
            vec![PrincipalTrustCheckpointRecord::new(
                ControlTransactionSequenceV1::new(1).unwrap(),
                record,
            )]
            .into_boxed_slice(),
        )
        .unwrap(),
        initial_ref: reference.clone(),
        initial: checkpoint.clone(),
        schema: fixture.trust_preimage.schema_sha256,
        key: view
            .authorities()
            .genesis()
            .principal_record()
            .root_public_key()
            .clone(),
    }
}

fn fixture_with_successor() -> Fixture {
    let (bootstrap, _) = principal_trust_test_support::admitted_bootstrap_fixture();
    let view = bootstrap.admitted.atom_view();
    let (initial_ref, initial) = view.trust_checkpoint_entry();
    let authorities = view.authorities();
    let successor = deployment_finalization_fixture::Fixture::from_trust(
        initial_ref,
        initial,
        authorities.checkpoint_authority_ref(),
        authorities.checkpoint_authority(),
    );
    let initial_coordinate = record_projection::coordinate(
        initial_ref,
        initial.canonical_trust_checkpoint_sha256().as_str(),
    )
    .unwrap();
    let next_coordinate = record_projection::coordinate(
        &successor.next_ref,
        successor.next.canonical_trust_checkpoint_sha256().as_str(),
    )
    .unwrap();
    let initial_record = record_projection::record(initial_coordinate, initial).unwrap();
    let next_record = record_projection::record(next_coordinate.clone(), &successor.next).unwrap();
    let binding = PrincipalTrustStoreBinding::new(
        initial.trust_store_id().clone(),
        initial.trust_store_incarnation_id().clone(),
        initial.principal_id().clone(),
    );
    let head = PrincipalTrustCheckpointHead::new(
        initial.principal_id().clone(),
        *successor.next.checkpoint_epoch(),
        next_coordinate,
    );
    Fixture {
        evidence: PrincipalTrustCheckpointSetSnapshot::new(
            binding,
            head,
            ControlTransactionSequenceV1::new(2).unwrap(),
            vec![
                PrincipalTrustCheckpointRecord::new(
                    ControlTransactionSequenceV1::new(1).unwrap(),
                    initial_record,
                ),
                PrincipalTrustCheckpointRecord::new(
                    ControlTransactionSequenceV1::new(2).unwrap(),
                    next_record,
                ),
            ]
            .into_boxed_slice(),
        )
        .unwrap(),
        initial_ref: initial_ref.clone(),
        initial: initial.clone(),
        schema: bootstrap.trust_preimage.schema_sha256,
        key: authorities
            .genesis()
            .principal_record()
            .root_public_key()
            .clone(),
    }
}

fn fixture_with_insertion_sequences(first: u64, second: u64) -> Fixture {
    let mut fixture = fixture_with_successor();
    let records = fixture
        .evidence
        .checkpoint_records()
        .iter()
        .zip([first, second])
        .map(|(record, inserted_in)| {
            PrincipalTrustCheckpointRecord::new(
                ControlTransactionSequenceV1::new(inserted_in).unwrap(),
                record.record().clone(),
            )
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    fixture.evidence = PrincipalTrustCheckpointSetSnapshot::new(
        fixture.evidence.binding().clone(),
        fixture.evidence.through().clone(),
        ControlTransactionSequenceV1::new(first.max(second)).unwrap(),
        records,
    )
    .unwrap();
    fixture
}

fn admit(
    fixture: Fixture,
    resolution: HistoricalPrincipalKeyResolution,
) -> Result<RecoveredSignedPrincipalTrustCheckpointChain, PrincipalTrustCheckpointChainRecoveryError>
{
    let Fixture {
        evidence,
        initial_ref,
        initial,
        schema,
        key,
    } = fixture;
    let expected_key = key.clone();
    let mut resolver = move |principal: &ResidentPrincipalIdV1,
                             _: &ResidentKeyIdV1,
                             epoch: PrincipalKeyEpochV1| {
        assert_eq!(principal.as_str(), "principal-1");
        assert_eq!(epoch.get(), 1);
        resolution.clone()
    };
    let mut verifier = move |domain: &[u8],
                             key: &Ed25519PublicKeyV1,
                             _: &PrincipalTrustCheckpointPreimageV1,
                             _: &PrincipalTrustCheckpointDigest,
                             _: &CanonicalSignatureV1| {
        assert_eq!(domain, PRINCIPAL_TRUST_CHECKPOINT_V1_SIGNATURE_DOMAIN);
        assert_eq!(key, &expected_key);
        Ok(())
    };
    recover_signed_principal_trust_checkpoint_chain(
        evidence,
        &initial_ref,
        &initial,
        &schema,
        PrincipalTrustCheckpointSetReadLimits::new(8, 64 * 1024),
        &mut resolver,
        &mut verifier,
    )
}

#[test]
fn recovery_preserves_exact_bytes_and_admits_only_record_time_signature() {
    let fixture = fixture_with_successor();
    let expected_evidence = fixture.evidence.clone();
    let expected_key = fixture.key.clone();
    let recovered = admit(
        fixture,
        HistoricalPrincipalKeyResolution::Resolved(expected_key),
    )
    .unwrap();
    assert_eq!(recovered.evidence(), &expected_evidence);
    assert_eq!(recovered.checkpoints().len(), 2);
    assert_eq!(
        recovered.current().coordinate(),
        expected_evidence.through().coordinate()
    );
    assert_eq!(recovered.current().checkpoint().checkpoint_epoch().get(), 2);
}

#[test]
fn recovery_distinguishes_absent_and_conflicting_historical_keys() {
    assert!(matches!(
        admit(
            fixture_with_successor(),
            HistoricalPrincipalKeyResolution::Unavailable
        ),
        Err(PrincipalTrustCheckpointChainRecoveryError::HistoricalKeyUnavailable)
    ));
    assert!(matches!(
        admit(
            fixture_with_successor(),
            HistoricalPrincipalKeyResolution::Conflicting
        ),
        Err(PrincipalTrustCheckpointChainRecoveryError::HistoricalKeyConflict)
    ));
}

#[test]
fn recovery_rejects_equal_or_reversed_physical_insertions() {
    for fixture in [
        fixture_with_insertion_sequences(1, 1),
        fixture_with_insertion_sequences(2, 1),
    ] {
        let mut resolver =
            |_: &ResidentPrincipalIdV1, _: &ResidentKeyIdV1, _: PrincipalKeyEpochV1| {
                panic!("physical order must fail before historical key resolution")
            };
        let mut verifier = |_: &[u8],
                            _: &Ed25519PublicKeyV1,
                            _: &PrincipalTrustCheckpointPreimageV1,
                            _: &PrincipalTrustCheckpointDigest,
                            _: &CanonicalSignatureV1| {
            panic!("physical order must fail before signature verification")
        };
        assert!(matches!(
            recover_signed_principal_trust_checkpoint_chain(
                fixture.evidence,
                &fixture.initial_ref,
                &fixture.initial,
                &fixture.schema,
                PrincipalTrustCheckpointSetReadLimits::new(8, 64 * 1024),
                &mut resolver,
                &mut verifier,
            ),
            Err(PrincipalTrustCheckpointChainRecoveryError::PhysicalTransactionOrder)
        ));
    }
}

#[test]
fn recovery_propagates_historical_signature_rejection() {
    let fixture = fixture_with_successor();
    let key = fixture.key.clone();
    let mut resolver =
        move |_: &ResidentPrincipalIdV1, _: &ResidentKeyIdV1, _: PrincipalKeyEpochV1| {
            HistoricalPrincipalKeyResolution::Resolved(key.clone())
        };
    let mut verifier = |_: &[u8],
                        _: &Ed25519PublicKeyV1,
                        _: &PrincipalTrustCheckpointPreimageV1,
                        _: &PrincipalTrustCheckpointDigest,
                        _: &CanonicalSignatureV1| {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "principal trust checkpoint",
            invariant: "test signature",
        })
    };
    assert!(matches!(
        recover_signed_principal_trust_checkpoint_chain(
            fixture.evidence,
            &fixture.initial_ref,
            &fixture.initial,
            &fixture.schema,
            PrincipalTrustCheckpointSetReadLimits::new(8, 64 * 1024),
            &mut resolver,
            &mut verifier,
        ),
        Err(PrincipalTrustCheckpointChainRecoveryError::Record(
            InvalidResidentRecordV1::ContextualAdmission {
                invariant: "test signature",
                ..
            }
        ))
    ));
}

#[test]
fn recovery_rechecks_item_and_aggregate_byte_bounds_before_decoding() {
    let fixture = fixture();
    let bytes = fixture.evidence.checkpoint_records()[0]
        .record()
        .canonical_record_jcs()
        .as_slice()
        .len();
    let mut resolver = |_: &ResidentPrincipalIdV1, _: &ResidentKeyIdV1, _: PrincipalKeyEpochV1| {
        panic!("bounds must be checked before key resolution")
    };
    let mut verifier = |_: &[u8],
                        _: &Ed25519PublicKeyV1,
                        _: &PrincipalTrustCheckpointPreimageV1,
                        _: &PrincipalTrustCheckpointDigest,
                        _: &CanonicalSignatureV1| {
        panic!("bounds must be checked before signature verification")
    };
    assert!(matches!(
        recover_signed_principal_trust_checkpoint_chain(
            fixture.evidence.clone(),
            &fixture.initial_ref,
            &fixture.initial,
            &fixture.schema,
            PrincipalTrustCheckpointSetReadLimits::new(0, bytes),
            &mut resolver,
            &mut verifier,
        ),
        Err(PrincipalTrustCheckpointChainRecoveryError::BoundExceeded)
    ));
    assert!(matches!(
        recover_signed_principal_trust_checkpoint_chain(
            fixture.evidence,
            &fixture.initial_ref,
            &fixture.initial,
            &fixture.schema,
            PrincipalTrustCheckpointSetReadLimits::new(1, bytes - 1),
            &mut resolver,
            &mut verifier,
        ),
        Err(PrincipalTrustCheckpointChainRecoveryError::BoundExceeded)
    ));
}

#[test]
fn recovery_rejects_noncanonical_and_physically_substituted_bytes() {
    let base = fixture();
    let original = base.evidence.current_checkpoint_record().record();
    let noncanonical = ControlRecord::new(
        original.coordinate().clone(),
        original.schema_id().clone(),
        original.schema_sha256().clone(),
        ControlRecordBytes::new(b"{ \"not\":\"canonical\"}".to_vec()).unwrap(),
    );
    let evidence = PrincipalTrustCheckpointSetSnapshot::new(
        base.evidence.binding().clone(),
        base.evidence.through().clone(),
        base.evidence.observed_through(),
        vec![PrincipalTrustCheckpointRecord::new(
            base.evidence.checkpoint_records()[0].inserted_in(),
            noncanonical,
        )]
        .into_boxed_slice(),
    )
    .unwrap();
    assert!(matches!(
        admit(
            Fixture {
                evidence,
                initial_ref: base.initial_ref.clone(),
                initial: base.initial.clone(),
                schema: base.schema.clone(),
                key: base.key.clone(),
            },
            HistoricalPrincipalKeyResolution::Resolved(base.key.clone()),
        ),
        Err(PrincipalTrustCheckpointChainRecoveryError::NonCanonicalStoredRecord)
            | Err(PrincipalTrustCheckpointChainRecoveryError::StoredRecordDecode(_))
    ));

    let base = fixture();
    let original = base.evidence.current_checkpoint_record().record();
    let substituted = ControlRecord::new(
        original.coordinate().clone(),
        original.schema_id().clone(),
        SchemaDigest::parse("f".repeat(64)).unwrap(),
        ControlRecordBytes::new(original.canonical_record_jcs().as_slice().to_vec()).unwrap(),
    );
    let evidence = PrincipalTrustCheckpointSetSnapshot::new(
        base.evidence.binding().clone(),
        base.evidence.through().clone(),
        base.evidence.observed_through(),
        vec![PrincipalTrustCheckpointRecord::new(
            base.evidence.checkpoint_records()[0].inserted_in(),
            substituted,
        )]
        .into_boxed_slice(),
    )
    .unwrap();
    assert!(matches!(
        admit(
            Fixture {
                evidence,
                initial_ref: base.initial_ref,
                initial: base.initial,
                schema: base.schema,
                key: base.key.clone(),
            },
            HistoricalPrincipalKeyResolution::Resolved(base.key),
        ),
        Err(PrincipalTrustCheckpointChainRecoveryError::StoredRecordMismatch)
    ));
}

#[test]
fn recovery_rejects_a_forked_epoch_even_when_each_signature_is_accepted() {
    let fixture = fixture();
    let original = fixture.evidence.current_checkpoint_record().record();
    let second_coordinate = ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse("trust-checkpoint-fork").unwrap(),
        ControlRecordDigest::parse(original.coordinate().record_sha256().as_str().to_owned())
            .unwrap(),
    );
    let fork = ControlRecord::new(
        second_coordinate,
        original.schema_id().clone(),
        original.schema_sha256().clone(),
        ControlRecordBytes::new(original.canonical_record_jcs().as_slice().to_vec()).unwrap(),
    );
    let evidence = PrincipalTrustCheckpointSetSnapshot::new(
        fixture.evidence.binding().clone(),
        fixture.evidence.through().clone(),
        ControlTransactionSequenceV1::new(2).unwrap(),
        vec![
            PrincipalTrustCheckpointRecord::new(
                ControlTransactionSequenceV1::new(1).unwrap(),
                original.clone(),
            ),
            PrincipalTrustCheckpointRecord::new(
                ControlTransactionSequenceV1::new(2).unwrap(),
                fork,
            ),
        ]
        .into_boxed_slice(),
    )
    .unwrap();
    assert!(matches!(
        admit(
            Fixture {
                evidence,
                initial_ref: fixture.initial_ref,
                initial: fixture.initial,
                schema: fixture.schema,
                key: fixture.key.clone(),
            },
            HistoricalPrincipalKeyResolution::Resolved(fixture.key),
        ),
        Err(PrincipalTrustCheckpointChainRecoveryError::Chain)
    ));
}

#[test]
fn recovery_rejects_a_selected_head_behind_a_valid_signed_suffix() {
    let fixture = fixture_with_successor();
    let initial_record = fixture.evidence.checkpoint_records()[0].record().clone();
    let stale_head = PrincipalTrustCheckpointHead::new(
        fixture.initial.principal_id().clone(),
        *fixture.initial.checkpoint_epoch(),
        initial_record.coordinate().clone(),
    );
    let evidence = PrincipalTrustCheckpointSetSnapshot::new(
        fixture.evidence.binding().clone(),
        stale_head,
        fixture.evidence.observed_through(),
        fixture
            .evidence
            .checkpoint_records()
            .to_vec()
            .into_boxed_slice(),
    )
    .unwrap();
    assert!(matches!(
        admit(
            Fixture {
                evidence,
                initial_ref: fixture.initial_ref,
                initial: fixture.initial,
                schema: fixture.schema,
                key: fixture.key.clone(),
            },
            HistoricalPrincipalKeyResolution::Resolved(fixture.key),
        ),
        Err(PrincipalTrustCheckpointChainRecoveryError::CurrentHead)
    ));
}

#[test]
fn recovery_rejects_another_selected_schema_or_store() {
    let fixture = fixture();
    let mut resolver = |_: &ResidentPrincipalIdV1, _: &ResidentKeyIdV1, _: PrincipalKeyEpochV1| {
        HistoricalPrincipalKeyResolution::Resolved(fixture.key.clone())
    };
    let mut verifier = |_: &[u8],
                        _: &Ed25519PublicKeyV1,
                        _: &PrincipalTrustCheckpointPreimageV1,
                        _: &PrincipalTrustCheckpointDigest,
                        _: &CanonicalSignatureV1| Ok(());
    assert!(matches!(
        recover_signed_principal_trust_checkpoint_chain(
            fixture.evidence.clone(),
            &fixture.initial_ref,
            &fixture.initial,
            &SchemaDigest::parse("e".repeat(64)).unwrap(),
            PrincipalTrustCheckpointSetReadLimits::new(8, 64 * 1024),
            &mut resolver,
            &mut verifier,
        ),
        Err(PrincipalTrustCheckpointChainRecoveryError::InitialCheckpoint)
    ));

    let wrong_binding = PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("other-store").unwrap(),
        ControlIncarnationIdV1::parse("other-incarnation").unwrap(),
        fixture.evidence.binding().principal_id().clone(),
    );
    let wrong_evidence = PrincipalTrustCheckpointSetSnapshot::new(
        wrong_binding,
        fixture.evidence.through().clone(),
        fixture.evidence.observed_through(),
        fixture
            .evidence
            .checkpoint_records()
            .to_vec()
            .into_boxed_slice(),
    )
    .unwrap();
    assert!(matches!(
        recover_signed_principal_trust_checkpoint_chain(
            wrong_evidence,
            &fixture.initial_ref,
            &fixture.initial,
            &fixture.schema,
            PrincipalTrustCheckpointSetReadLimits::new(8, 64 * 1024),
            &mut resolver,
            &mut verifier,
        ),
        Err(PrincipalTrustCheckpointChainRecoveryError::InitialCheckpoint)
    ));
}
