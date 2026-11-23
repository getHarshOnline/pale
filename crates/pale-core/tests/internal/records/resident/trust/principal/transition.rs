//! Normal-rotation and recovery-transition branch law tests.

use super::support::{
    admit_proof, genesis, new_public_key, normal_transition_preimage, proof_preimage,
    proof_wire_value, root_public_key, transition_wire_value,
};
use super::*;

#[test]
fn normal_transition_requires_both_keys_over_one_digest() {
    let genesis = genesis();
    let preimage = normal_transition_preimage(&genesis);
    let digest =
        PrincipalKeyTransitionDigest::hash_canonical(&preimage).expect("transition digest");
    let wire = serde_json::from_value(transition_wire_value(&preimage, &digest))
        .expect("transition wire decodes");
    let transition = PrincipalKeyTransitionV1::admit_normal_rotation_wire(
        wire,
        &PrincipalKeyTransitionAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            predecessor_ref: genesis.principal_record_ref(),
            predecessor: genesis.principal_record(),
        },
        |domain, key, observed_preimage, observed_digest, _| {
            assert_eq!(domain, PRINCIPAL_KEY_TRANSITION_OLD_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &root_public_key());
            assert_eq!(observed_preimage, &preimage);
            assert_eq!(observed_digest, &digest);
            Ok(())
        },
        |domain, key, observed_preimage, observed_digest, _| {
            assert_eq!(domain, PRINCIPAL_KEY_TRANSITION_NEW_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &new_public_key());
            assert_eq!(observed_preimage, &preimage);
            assert_eq!(observed_digest, &digest);
            Ok(())
        },
    )
    .expect("admit normal transition");

    assert_eq!(
        transition.transition_mode(),
        PrincipalKeyTransitionModeV1::NormalRotation
    );
    assert!(transition.old_key_signature().is_some());
    assert!(transition.recovery_authority_proof_ref().is_none());
    transition.verify().expect("transition integrity");
}

#[test]
fn recovery_transition_binds_exact_acyclic_proof() {
    let genesis = genesis();
    let proof = admit_proof(&genesis);
    let mut preimage = normal_transition_preimage(&genesis);
    preimage.transition_mode = PrincipalKeyTransitionModeV1::Recovery;
    preimage.recovery_authority_proof_ref = Some(record_ref("recovery-proof-1"));
    preimage.recovery_authority_proof_sha256 = Some(proof.canonical_record_sha256().clone());
    let digest =
        PrincipalKeyTransitionDigest::hash_canonical(&preimage).expect("transition digest");
    let wire = serde_json::from_value(transition_wire_value(&preimage, &digest))
        .expect("recovery transition wire decodes");
    let transition = PrincipalKeyTransitionV1::admit_recovery_wire(
        wire,
        &PrincipalKeyTransitionAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            predecessor_ref: genesis.principal_record_ref(),
            predecessor: genesis.principal_record(),
        },
        &record_ref("recovery-proof-1"),
        &proof,
        |domain, key, observed_preimage, observed_digest, _| {
            assert_eq!(domain, PRINCIPAL_KEY_TRANSITION_NEW_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &new_public_key());
            assert_eq!(observed_preimage, &preimage);
            assert_eq!(observed_digest, &digest);
            Ok(())
        },
    )
    .expect("admit recovery transition");

    assert_eq!(
        transition.transition_mode(),
        PrincipalKeyTransitionModeV1::Recovery
    );
    assert!(transition.old_key_signature().is_none());
    assert_eq!(
        transition.recovery_authority_proof_sha256(),
        Some(proof.canonical_record_sha256())
    );
}

#[test]
fn recovery_transition_rejects_a_proof_for_an_alias_predecessor_reference() {
    let genesis = genesis();
    let alias_ref = record_ref("principal-record-alias");
    let mut proof_preimage = proof_preimage(&genesis);
    proof_preimage.prior_principal_record_ref = alias_ref.clone();
    let proof_digest =
        PrincipalRecoveryProofDigest::hash_canonical(&proof_preimage).expect("proof digest");
    let proof = PrincipalRecoveryProofV1::admit_wire(
        serde_json::from_value(proof_wire_value(&proof_preimage, &proof_digest))
            .expect("proof wire decodes"),
        &PrincipalRecoveryProofAdmissionContextV1 {
            expected_schema_sha256: &proof_preimage.schema_sha256,
            prior_principal_record_ref: &alias_ref,
            prior_principal_record: genesis.principal_record(),
            recovery_policy_ref: genesis.recovery_policy_ref(),
            recovery_policy: genesis.recovery_policy(),
            presented_at: UnixTimeV1::new(30).expect("presentation time"),
        },
        |_, _, _, _, _| Ok(()),
    )
    .expect("proof admitted for alias coordinate");
    let mut preimage = normal_transition_preimage(&genesis);
    preimage.transition_mode = PrincipalKeyTransitionModeV1::Recovery;
    preimage.recovery_authority_proof_ref = Some(record_ref("recovery-proof-1"));
    preimage.recovery_authority_proof_sha256 = Some(proof.canonical_record_sha256().clone());
    let digest =
        PrincipalKeyTransitionDigest::hash_canonical(&preimage).expect("transition digest");
    let wire = serde_json::from_value(transition_wire_value(&preimage, &digest))
        .expect("transition wire decodes");

    assert!(PrincipalKeyTransitionV1::admit_recovery_wire(
        wire,
        &PrincipalKeyTransitionAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            predecessor_ref: genesis.principal_record_ref(),
            predecessor: genesis.principal_record(),
        },
        &record_ref("recovery-proof-1"),
        &proof,
        |_, _, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn transition_rejects_mixed_branches_and_skipped_epoch() {
    let genesis = genesis();
    let base = normal_transition_preimage(&genesis);

    let mut mixed = base.clone();
    mixed.recovery_authority_proof_ref = Some(record_ref("recovery-proof-1"));
    mixed.recovery_authority_proof_sha256 =
        Some(PrincipalRecoveryProofDigest::parse(sha('e')).expect("recovery proof digest"));
    let mixed_digest = PrincipalKeyTransitionDigest::hash_canonical(&mixed).expect("mixed digest");
    let mixed_wire = serde_json::from_value(transition_wire_value(&mixed, &mixed_digest))
        .expect("mixed structural wire");
    assert!(PrincipalKeyTransitionV1::admit_normal_rotation_wire(
        mixed_wire,
        &PrincipalKeyTransitionAdmissionContextV1 {
            expected_schema_sha256: &mixed.schema_sha256,
            predecessor_ref: genesis.principal_record_ref(),
            predecessor: genesis.principal_record(),
        },
        |_, _, _, _, _| Ok(()),
        |_, _, _, _, _| Ok(()),
    )
    .is_err());

    let proof = admit_proof(&genesis);
    let mut recovery_with_old_signature = base.clone();
    recovery_with_old_signature.transition_mode = PrincipalKeyTransitionModeV1::Recovery;
    recovery_with_old_signature.recovery_authority_proof_ref = Some(record_ref("recovery-proof-1"));
    recovery_with_old_signature.recovery_authority_proof_sha256 =
        Some(proof.canonical_record_sha256().clone());
    let recovery_digest =
        PrincipalKeyTransitionDigest::hash_canonical(&recovery_with_old_signature)
            .expect("recovery digest");
    let mut recovery_wire = transition_wire_value(&recovery_with_old_signature, &recovery_digest);
    recovery_wire["old_key_signature"] = serde_json::json!("A".repeat(86));
    let recovery_wire =
        serde_json::from_value(recovery_wire).expect("mixed recovery structural wire");
    assert!(PrincipalKeyTransitionV1::admit_recovery_wire(
        recovery_wire,
        &PrincipalKeyTransitionAdmissionContextV1 {
            expected_schema_sha256: &recovery_with_old_signature.schema_sha256,
            predecessor_ref: genesis.principal_record_ref(),
            predecessor: genesis.principal_record(),
        },
        &record_ref("recovery-proof-1"),
        &proof,
        |_, _, _, _, _| Ok(()),
    )
    .is_err());

    let mut skipped = base;
    skipped.new_key_epoch = PrincipalKeyEpochV1::new(3).expect("skipped epoch");
    let skipped_digest =
        PrincipalKeyTransitionDigest::hash_canonical(&skipped).expect("skipped digest");
    let skipped_wire = serde_json::from_value(transition_wire_value(&skipped, &skipped_digest))
        .expect("skipped structural wire");
    assert!(PrincipalKeyTransitionV1::admit_normal_rotation_wire(
        skipped_wire,
        &PrincipalKeyTransitionAdmissionContextV1 {
            expected_schema_sha256: &skipped.schema_sha256,
            predecessor_ref: genesis.principal_record_ref(),
            predecessor: genesis.principal_record(),
        },
        |_, _, _, _, _| Ok(()),
        |_, _, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn transition_wire_rejects_explicit_null() {
    let genesis = genesis();
    let preimage = normal_transition_preimage(&genesis);
    let digest =
        PrincipalKeyTransitionDigest::hash_canonical(&preimage).expect("transition digest");
    let mut value = transition_wire_value(&preimage, &digest);
    value["recovery_authority_proof_ref"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<PrincipalKeyTransitionWireV1>(value).is_err());
}

#[test]
fn transition_jcs_vector_is_stable_and_excludes_both_signatures() {
    let genesis = genesis();
    let preimage = normal_transition_preimage(&genesis);
    let digest =
        PrincipalKeyTransitionDigest::hash_canonical(&preimage).expect("transition digest");
    assert_eq!(
        digest.as_str(),
        "b62ff388a6e6e57c4b8c88a5721d8bd3505abaed75af7bfa0bdb716000d2e7e4"
    );

    let first = transition_wire_value(&preimage, &digest);
    let mut second = first.clone();
    second["old_key_signature"] = serde_json::json!("Q".repeat(86));
    second["new_key_signature"] = serde_json::json!("g".repeat(86));
    assert_eq!(
        first["canonical_record_sha256"],
        second["canonical_record_sha256"]
    );
}
