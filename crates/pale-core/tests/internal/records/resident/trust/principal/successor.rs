//! Principal key- and policy-successor law tests.

use super::support::{
    admit_normal_transition, genesis, new_public_key, root_key_id, root_public_key, signed_wire,
};
use super::*;

fn policy_successor_preimage(
    genesis: &AdmittedPrincipalGenesisV1,
    predecessor_ref: ResidentRecordRefV1,
) -> PrincipalRecoveryPolicyPreimageV1 {
    PrincipalRecoveryPolicyPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RECOVERY_POLICY_V1_SCHEMA).expect("policy schema"),
        schema_sha256: schema_digest('a'),
        recovery_policy_id: genesis.recovery_policy().recovery_policy_id().clone(),
        principal_id: principal(),
        policy_epoch: PrincipalRecoveryPolicyEpochV1::new(2).expect("next policy epoch"),
        eligible_signers: genesis.recovery_policy().eligible_signers().to_vec(),
        threshold: *genesis.recovery_policy().threshold(),
        proof_validity_seconds: *genesis.recovery_policy().proof_validity_seconds(),
        replay_scope: *genesis.recovery_policy().replay_scope(),
        issued_at: UnixTimeV1::new(80).expect("policy issue"),
        expires_at: Some(UnixTimeV1::new(1_100).expect("policy expiry")),
        previous_policy_ref: Some(predecessor_ref),
        previous_policy_sha256: Some(genesis.recovery_policy().canonical_record_sha256().clone()),
        issuer_key_id: root_key_id(),
        issuer_principal_key_epoch: PrincipalKeyEpochV1::new(1).expect("root epoch"),
    }
}

#[test]
fn key_transition_successor_binds_new_key_and_preserves_recovery_policy() {
    let genesis = genesis();
    let transition = admit_normal_transition(&genesis);
    let preimage = PrincipalRecordPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RECORD_V1_SCHEMA).expect("principal schema"),
        schema_sha256: schema_digest('b'),
        principal_id: principal(),
        display_hint: BoundedResidentTextV1::parse("Rotated principal").expect("display hint"),
        root_key_id: transition.new_key_id().clone(),
        root_public_key: transition.new_public_key().clone(),
        key_epoch: transition.new_key_epoch(),
        record_transition_kind: PrincipalRecordTransitionKindV1::KeyTransition,
        previous_record_sha256: Some(genesis.principal_record().canonical_record_sha256().clone()),
        key_transition_ref: Some(record_ref("key-transition-1")),
        key_transition_sha256: Some(transition.canonical_record_sha256().clone()),
        issued_at: UnixTimeV1::new(60).expect("principal issue"),
        expires_at: UnixTimeV1::new(1_200).expect("principal expiry"),
        recovery_policy_ref: genesis.principal_record().recovery_policy_ref().clone(),
        recovery_policy_sha256: genesis.principal_record().recovery_policy_sha256().clone(),
    };
    let digest = PrincipalRecordDigest::hash_canonical(&preimage).expect("principal digest");
    let successor = PrincipalRecordV1::admit_key_transition_successor_wire(
        signed_wire(&preimage, &digest),
        &preimage.schema_sha256,
        genesis.principal_record(),
        &record_ref("key-transition-1"),
        &transition,
        |domain, key, observed, observed_digest, _| {
            assert_eq!(domain, PRINCIPAL_RECORD_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &new_public_key());
            assert_eq!(observed, &preimage);
            assert_eq!(observed_digest, &digest);
            Ok(())
        },
    )
    .expect("admit key successor");

    assert_eq!(successor.root_key_id(), transition.new_key_id());
    assert_eq!(
        successor.recovery_policy_sha256(),
        genesis.principal_record().recovery_policy_sha256()
    );
}

#[test]
fn policy_update_admits_only_the_exact_same_key_pair() {
    let genesis = genesis();
    let policy_preimage =
        policy_successor_preimage(&genesis, genesis.recovery_policy_ref().clone());
    let policy_digest =
        PrincipalRecoveryPolicyDigest::hash_canonical(&policy_preimage).expect("policy digest");
    let principal_preimage = PrincipalRecordPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RECORD_V1_SCHEMA).expect("principal schema"),
        schema_sha256: schema_digest('b'),
        principal_id: principal(),
        display_hint: BoundedResidentTextV1::parse("Policy-updated principal")
            .expect("display hint"),
        root_key_id: root_key_id(),
        root_public_key: root_public_key(),
        key_epoch: PrincipalKeyEpochV1::new(1).expect("root epoch"),
        record_transition_kind: PrincipalRecordTransitionKindV1::PolicyUpdate,
        previous_record_sha256: Some(genesis.principal_record().canonical_record_sha256().clone()),
        key_transition_ref: None,
        key_transition_sha256: None,
        issued_at: UnixTimeV1::new(80).expect("principal issue"),
        expires_at: UnixTimeV1::new(1_200).expect("principal expiry"),
        recovery_policy_ref: record_ref("recovery-policy-2"),
        recovery_policy_sha256: policy_digest.clone(),
    };
    let principal_digest =
        PrincipalRecordDigest::hash_canonical(&principal_preimage).expect("principal digest");
    let update = AdmittedPrincipalPolicyUpdateV1::admit(
        PrincipalPolicyUpdateCandidateV1 {
            policy_wire: signed_wire(&policy_preimage, &policy_digest),
            recovery_policy_ref: record_ref("recovery-policy-2"),
            expected_policy_schema_sha256: &policy_preimage.schema_sha256,
            principal_wire: signed_wire(&principal_preimage, &principal_digest),
            expected_principal_schema_sha256: &principal_preimage.schema_sha256,
            predecessor_policy_ref: genesis.recovery_policy_ref(),
        },
        genesis.principal_record(),
        genesis.recovery_policy(),
        |domain, key, _, digest, _| {
            assert_eq!(domain, PRINCIPAL_RECOVERY_POLICY_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &root_public_key());
            assert_eq!(digest, &policy_digest);
            Ok(())
        },
        |domain, key, _, digest, _| {
            assert_eq!(domain, PRINCIPAL_RECORD_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &root_public_key());
            assert_eq!(digest, &principal_digest);
            Ok(())
        },
    )
    .expect("admit policy update pair");

    assert_eq!(update.recovery_policy_ref().as_str(), "recovery-policy-2");
    assert_eq!(
        update.principal_record().recovery_policy_sha256(),
        update.recovery_policy().canonical_record_sha256()
    );
    assert_eq!(
        update.principal_record().root_public_key(),
        genesis.principal_record().root_public_key()
    );
}

#[test]
fn policy_successor_rejects_a_predecessor_not_pinned_by_the_active_principal() {
    let genesis = genesis();
    let inactive_ref = record_ref("inactive-policy");
    let preimage = policy_successor_preimage(&genesis, inactive_ref.clone());
    let digest = PrincipalRecoveryPolicyDigest::hash_canonical(&preimage).expect("policy digest");

    assert!(PrincipalRecoveryPolicyV1::admit_successor_wire(
        signed_wire(&preimage, &digest),
        &inactive_ref,
        genesis.recovery_policy(),
        genesis.principal_record(),
        &preimage.schema_sha256,
        |_, _, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn key_successor_rejects_recovery_policy_substitution() {
    let genesis = genesis();
    let transition = admit_normal_transition(&genesis);
    let mut preimage = PrincipalRecordPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RECORD_V1_SCHEMA).expect("principal schema"),
        schema_sha256: schema_digest('b'),
        principal_id: principal(),
        display_hint: BoundedResidentTextV1::parse("Local principal").expect("display hint"),
        root_key_id: transition.new_key_id().clone(),
        root_public_key: transition.new_public_key().clone(),
        key_epoch: transition.new_key_epoch(),
        record_transition_kind: PrincipalRecordTransitionKindV1::KeyTransition,
        previous_record_sha256: Some(genesis.principal_record().canonical_record_sha256().clone()),
        key_transition_ref: Some(record_ref("key-transition-1")),
        key_transition_sha256: Some(transition.canonical_record_sha256().clone()),
        issued_at: UnixTimeV1::new(60).expect("principal issue"),
        expires_at: UnixTimeV1::new(1_200).expect("principal expiry"),
        recovery_policy_ref: genesis.principal_record().recovery_policy_ref().clone(),
        recovery_policy_sha256: genesis.principal_record().recovery_policy_sha256().clone(),
    };
    preimage.recovery_policy_ref = record_ref("substituted-policy");
    let digest = PrincipalRecordDigest::hash_canonical(&preimage).expect("principal digest");
    assert!(PrincipalRecordV1::admit_key_transition_successor_wire(
        signed_wire(&preimage, &digest),
        &preimage.schema_sha256,
        genesis.principal_record(),
        &record_ref("key-transition-1"),
        &transition,
        |_, _, _, _, _| Ok(()),
    )
    .is_err());
}
