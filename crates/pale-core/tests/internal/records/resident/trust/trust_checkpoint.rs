//! Principal-trust-checkpoint successor-law tests.

use std::cell::Cell;

use super::*;

pub(super) fn trust_preimage(
    epoch: u64,
    previous: Option<(&ResidentRecordRefV1, &PrincipalTrustCheckpointV1)>,
) -> PrincipalTrustCheckpointPreimageV1 {
    PrincipalTrustCheckpointPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA)
            .expect("trust checkpoint schema"),
        schema_sha256: schema_digest('a'),
        trust_checkpoint_id: PrincipalTrustCheckpointIdV1::parse(format!(
            "trust-checkpoint-{epoch}"
        ))
        .expect("trust checkpoint id"),
        principal_id: principal(),
        trust_store_id: ControlStoreIdV1::parse("trust-store-1").expect("trust store"),
        trust_store_incarnation_id: ControlIncarnationIdV1::parse("trust-incarnation-1")
            .expect("trust incarnation"),
        principal_control_store_id: ControlStoreIdV1::parse("control-store-1")
            .expect("control store"),
        principal_control_incarnation_id: ControlIncarnationIdV1::parse("control-incarnation-1")
            .expect("control incarnation"),
        checkpoint_epoch: PrincipalTrustCheckpointEpochV1::new(epoch).expect("checkpoint epoch"),
        previous_trust_checkpoint_ref: previous.map(|(record_ref, _)| record_ref.clone()),
        previous_trust_checkpoint_sha256: previous
            .map(|(_, checkpoint)| checkpoint.canonical_trust_checkpoint_sha256().clone()),
        principal_record_ref: record_ref("principal-record-1"),
        principal_record_sha256: PrincipalRecordDigest::parse(sha('b')).expect("principal digest"),
        recovery_policy_ref: record_ref("recovery-policy-1"),
        recovery_policy_sha256: PrincipalRecoveryPolicyDigest::parse(sha('c'))
            .expect("recovery-policy digest"),
        local_admin_authority_ref: record_ref("admin-authority-1"),
        local_admin_authority_sha256: LocalAdminAuthorityDigest::parse(sha('d'))
            .expect("admin-authority digest"),
        producer_incarnation_authority_ref: record_ref("producer-authority-1"),
        producer_incarnation_authority_sha256: ProducerIncarnationAuthorityDigest::parse(sha('e'))
            .expect("producer-authority digest"),
        producer_incarnation_heads_root_sha256: ProducerIncarnationHeadsRootDigest::parse(sha('f'))
            .expect("producer-head root"),
        session_floor_epoch: PrincipalSessionFloorEpochV1::new(1).expect("session floor"),
        checkpoint_signing_authority_heads_root_sha256:
            CheckpointSigningAuthorityHeadsRootDigest::parse(sha('1'))
                .expect("checkpoint-signer root"),
        deployment_runtime_authority_heads_root_sha256:
            DeploymentRuntimeAuthorityHeadsRootDigest::parse(sha('2'))
                .expect("runtime-authority root"),
        owner_record_signing_authority_heads_root_sha256:
            OwnerRecordSigningAuthorityHeadsRootDigest::parse(sha('3')).expect("owner-signer root"),
        owner_signer_publication_frontier_heads_root_sha256:
            OwnerSignerPublicationFrontierHeadsRootDigest::parse(sha('4'))
                .expect("owner-publication root"),
        deployment_acceptance_revocation_heads_root_sha256:
            DeploymentAcceptanceRevocationHeadsRootDigest::parse(sha('5'))
                .expect("acceptance-revocation root"),
        federation_space_home_heads_root_sha256: FederationSpaceHomeHeadsRootDigest::parse(sha(
            '6',
        ))
        .expect("federation-home root"),
        node_delegation_heads_root_sha256: NodeDelegationHeadsRootDigest::parse(sha('7'))
            .expect("delegation root"),
        node_delegation_revocation_heads_root_sha256:
            NodeDelegationRevocationHeadsRootDigest::parse(sha('8'))
                .expect("delegation-revocation root"),
        issued_at: UnixTimeV1::new(10 + epoch).expect("issue time"),
        issuer_principal_id: principal(),
        issuer_key_id: issuer_key(),
        issuer_key_epoch: key_epoch(),
    }
}

fn checkpoint(preimage: PrincipalTrustCheckpointPreimageV1) -> PrincipalTrustCheckpointV1 {
    PrincipalTrustCheckpointV1::seal(preimage, signature()).expect("trust checkpoint")
}

fn successor_wire(preimage: PrincipalTrustCheckpointPreimageV1) -> PrincipalTrustCheckpointWireV1 {
    wire_from(&checkpoint(preimage))
}

#[test]
fn trust_checkpoint_successor_runs_frontier_validation_before_signature() {
    let predecessor_ref = record_ref("trust-checkpoint-1");
    let predecessor = checkpoint(trust_preimage(1, None));
    let successor = trust_preimage(2, Some((&predecessor_ref, &predecessor)));
    let frontier_checked = Cell::new(false);
    let signature_checked = Cell::new(false);

    let admitted = PrincipalTrustCheckpointV1::admit_successor_wire(
        successor_wire(successor.clone()),
        &predecessor_ref,
        &predecessor,
        &successor.schema_sha256,
        |candidate, observed_predecessor| {
            assert_eq!(candidate.checkpoint_epoch.get(), 2);
            assert_eq!(observed_predecessor, &predecessor);
            frontier_checked.set(true);
            Ok(())
        },
        |domain, _, _, _| {
            assert!(frontier_checked.get());
            assert_eq!(domain, PRINCIPAL_TRUST_CHECKPOINT_V1_SIGNATURE_DOMAIN);
            signature_checked.set(true);
            Ok(())
        },
    )
    .expect("exact successor admits");

    assert_eq!(admitted.checkpoint_epoch().get(), 2);
    admitted
        .verify_successor_of(&predecessor_ref, &predecessor)
        .expect("admitted successor relation rechecks");
    assert!(signature_checked.get());
}

#[test]
fn trust_checkpoint_successor_rejects_skips_predecessor_and_store_drift() {
    let predecessor_ref = record_ref("trust-checkpoint-1");
    let predecessor = checkpoint(trust_preimage(1, None));
    let candidate = trust_preimage(2, Some((&predecessor_ref, &predecessor)));

    let mut skipped = candidate.clone();
    skipped.checkpoint_epoch = PrincipalTrustCheckpointEpochV1::new(3).expect("skipped epoch");
    for rejected in [
        skipped,
        {
            let mut changed = candidate.clone();
            changed.previous_trust_checkpoint_ref = Some(record_ref("other-checkpoint"));
            changed
        },
        {
            let mut changed = candidate.clone();
            changed.previous_trust_checkpoint_sha256 = Some(
                PrincipalTrustCheckpointDigest::parse(sha('9')).expect("other checkpoint digest"),
            );
            changed
        },
        {
            let mut changed = candidate.clone();
            changed.schema_sha256 = schema_digest('9');
            changed
        },
        {
            let mut changed = candidate.clone();
            changed.principal_id =
                ResidentPrincipalIdV1::parse("other-principal").expect("other principal");
            changed
        },
        {
            let mut changed = candidate.clone();
            changed.trust_store_id =
                ControlStoreIdV1::parse("other-trust-store").expect("other trust store");
            changed
        },
        {
            let mut changed = candidate.clone();
            changed.trust_store_incarnation_id =
                ControlIncarnationIdV1::parse("restored-store").expect("store incarnation");
            changed
        },
        {
            let mut changed = candidate.clone();
            changed.principal_control_store_id =
                ControlStoreIdV1::parse("other-control-store").expect("other control store");
            changed
        },
        {
            let mut changed = candidate.clone();
            changed.principal_control_incarnation_id =
                ControlIncarnationIdV1::parse("restored-control").expect("control incarnation");
            changed
        },
        {
            let mut changed = candidate.clone();
            changed.issued_at =
                UnixTimeV1::new(predecessor.issued_at().get() - 1).expect("regressed issue time");
            changed
        },
    ] {
        assert!(checkpoint(rejected.clone())
            .verify_successor_of(&predecessor_ref, &predecessor)
            .is_err());
        assert!(PrincipalTrustCheckpointV1::admit_successor_wire(
            successor_wire(rejected),
            &predecessor_ref,
            &predecessor,
            &candidate.schema_sha256,
            |_, _| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .is_err());
    }

    let mut changed_schema = candidate.clone();
    changed_schema.schema_sha256 = schema_digest('9');
    let changed_expected = changed_schema.schema_sha256.clone();
    assert!(PrincipalTrustCheckpointV1::admit_successor_wire(
        successor_wire(changed_schema),
        &predecessor_ref,
        &predecessor,
        &changed_expected,
        |_, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn trust_checkpoint_successor_requires_owner_frontier_and_signature_proofs() {
    let predecessor_ref = record_ref("trust-checkpoint-1");
    let predecessor = checkpoint(trust_preimage(1, None));
    let candidate = trust_preimage(2, Some((&predecessor_ref, &predecessor)));

    let frontier_rejected = PrincipalTrustCheckpointV1::admit_successor_wire(
        successor_wire(candidate.clone()),
        &predecessor_ref,
        &predecessor,
        &candidate.schema_sha256,
        |_, _| {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "test frontier",
                invariant: "complete heads",
            })
        },
        |_, _, _, _| panic!("signature must follow frontier admission"),
    );
    assert!(frontier_rejected.is_err());

    let signature_rejected = PrincipalTrustCheckpointV1::admit_successor_wire(
        successor_wire(candidate.clone()),
        &predecessor_ref,
        &predecessor,
        &candidate.schema_sha256,
        |_, _| Ok(()),
        |_, _, _, _| reject_test_signature(),
    );
    assert!(signature_rejected.is_err());
}
