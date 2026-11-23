//! Recovery-proof threshold, freshness, and canonical-set law tests.

use super::support::{
    genesis, principal_preimage, proof_preimage, proof_wire_value, recovery_policy_preimage,
    recovery_public_key, signed_wire,
};
use super::*;

fn two_signer_genesis() -> AdmittedPrincipalGenesisV1 {
    let mut policy_preimage = recovery_policy_preimage();
    policy_preimage
        .eligible_signers
        .push(PrincipalRecoveryEligibleSignerV1 {
            recovery_principal_id: principal(),
            recovery_key_id: ResidentKeyIdV1::parse("recovery-key-2")
                .expect("second recovery key id"),
            recovery_public_key: Ed25519PublicKeyV1::parse("M".repeat(43))
                .expect("second recovery public key"),
            recovery_key_epoch: PrincipalKeyEpochV1::new(1).expect("second recovery key epoch"),
        });
    policy_preimage.eligible_signers.sort();
    policy_preimage.threshold = 2;
    let policy_digest =
        PrincipalRecoveryPolicyDigest::hash_canonical(&policy_preimage).expect("policy digest");
    let principal_preimage = principal_preimage(policy_digest.clone());
    let principal_digest =
        PrincipalRecordDigest::hash_canonical(&principal_preimage).expect("principal digest");
    AdmittedPrincipalGenesisV1::admit_initial(
        InitialPrincipalGenesisCandidateV1 {
            principal_record_ref: record_ref("principal-record-1"),
            principal_wire: signed_wire(&principal_preimage, &principal_digest),
            expected_principal_schema_sha256: &principal_preimage.schema_sha256,
            recovery_policy_ref: record_ref("recovery-policy-1"),
            recovery_wire: signed_wire(&policy_preimage, &policy_digest),
            expected_recovery_schema_sha256: &policy_preimage.schema_sha256,
        },
        |_, _, _, _, _| Ok(()),
        |_, _, _, _, _| Ok(()),
    )
    .expect("two-signer genesis")
}

fn two_signer_proof_wire(
    preimage: &PrincipalRecoveryProofPreimageV1,
    digest: &PrincipalRecoveryProofDigest,
) -> serde_json::Value {
    let mut value = proof_wire_value(preimage, digest);
    value["signatures"]
        .as_array_mut()
        .expect("signature array")
        .push(serde_json::json!({
            "signer_principal_id": "principal-1",
            "signer_key_id": "recovery-key-2",
            "signer_key_epoch": 1,
            "signed_record_sha256": digest,
            "signature": "Q".repeat(86),
        }));
    value
}

#[test]
fn recovery_proof_admits_exact_threshold_without_claiming_consumption() {
    let genesis = genesis();
    let preimage = proof_preimage(&genesis);
    let digest = PrincipalRecoveryProofDigest::hash_canonical(&preimage).expect("proof digest");
    let wire =
        serde_json::from_value(proof_wire_value(&preimage, &digest)).expect("proof wire decodes");
    let mut verified = 0;
    let proof = PrincipalRecoveryProofV1::admit_wire(
        wire,
        &PrincipalRecoveryProofAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            prior_principal_record_ref: genesis.principal_record_ref(),
            prior_principal_record: genesis.principal_record(),
            recovery_policy_ref: genesis.recovery_policy_ref(),
            recovery_policy: genesis.recovery_policy(),
            presented_at: UnixTimeV1::new(30).expect("presentation time"),
        },
        |domain, key, observed_preimage, observed_digest, _| {
            verified += 1;
            assert_eq!(domain, PRINCIPAL_RECOVERY_PROOF_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &recovery_public_key());
            assert_eq!(observed_preimage, &preimage);
            assert_eq!(observed_digest, &digest);
            Ok(())
        },
    )
    .expect("admit proof");

    assert_eq!(verified, 1);
    assert_eq!(proof.signatures().len(), 1);
    assert_eq!(proof.canonical_record_sha256(), &digest);
    assert_eq!(proof.nonce().as_str(), "recovery-nonce-1");
    proof.verify().expect("local proof integrity");
}

#[test]
fn recovery_signature_set_permutations_have_one_admitted_projection() {
    let genesis = two_signer_genesis();
    let preimage = proof_preimage(&genesis);
    let digest = PrincipalRecoveryProofDigest::hash_canonical(&preimage).expect("proof digest");
    let first = two_signer_proof_wire(&preimage, &digest);
    let mut reversed = first.clone();
    reversed["signatures"]
        .as_array_mut()
        .expect("signature array")
        .reverse();
    let context = PrincipalRecoveryProofAdmissionContextV1 {
        expected_schema_sha256: &preimage.schema_sha256,
        prior_principal_record_ref: genesis.principal_record_ref(),
        prior_principal_record: genesis.principal_record(),
        recovery_policy_ref: genesis.recovery_policy_ref(),
        recovery_policy: genesis.recovery_policy(),
        presented_at: UnixTimeV1::new(30).expect("presentation time"),
    };
    let admit = |value| {
        PrincipalRecoveryProofV1::admit_wire(
            serde_json::from_value(value).expect("proof wire decodes"),
            &context,
            |_, _, _, _, _| Ok(()),
        )
        .expect("admit proof permutation")
    };
    let first = admit(first);
    let reversed = admit(reversed);

    assert_eq!(first, reversed);
    assert_eq!(
        serde_json::to_value(first).expect("first proof serializes"),
        serde_json::to_value(reversed).expect("reversed proof serializes")
    );
}

#[test]
fn recovery_proof_rejects_duplicate_and_stale_signer_coordinates_at_exact_threshold() {
    let genesis = two_signer_genesis();
    let preimage = proof_preimage(&genesis);
    let digest = PrincipalRecoveryProofDigest::hash_canonical(&preimage).expect("proof digest");
    let base = two_signer_proof_wire(&preimage, &digest);
    let context = PrincipalRecoveryProofAdmissionContextV1 {
        expected_schema_sha256: &preimage.schema_sha256,
        prior_principal_record_ref: genesis.principal_record_ref(),
        prior_principal_record: genesis.principal_record(),
        recovery_policy_ref: genesis.recovery_policy_ref(),
        recovery_policy: genesis.recovery_policy(),
        presented_at: UnixTimeV1::new(30).expect("presentation time"),
    };

    for (name, member, replacement) in [
        (
            "duplicate coordinate",
            "signer_key_id",
            serde_json::json!("recovery-key-1"),
        ),
        ("stale epoch", "signer_key_epoch", serde_json::json!(2)),
    ] {
        let mut value = base.clone();
        value["signatures"][1][member] = replacement;
        let error = PrincipalRecoveryProofV1::admit_wire(
            serde_json::from_value(value).expect("proof wire decodes"),
            &context,
            |_, _, _, _, _| Ok(()),
        )
        .expect_err(name);
        assert!(matches!(
            error,
            InvalidResidentRecordV1::ContextualAdmission { .. }
        ));
    }
}

#[test]
fn recovery_proof_rejects_non_exact_or_ineligible_signature_sets() {
    let genesis = genesis();
    let preimage = proof_preimage(&genesis);
    let digest = PrincipalRecoveryProofDigest::hash_canonical(&preimage).expect("proof digest");
    let base = proof_wire_value(&preimage, &digest);

    for (name, mutate) in [
        (
            "missing",
            Box::new(|value: &mut serde_json::Value| {
                value["signatures"] = serde_json::json!([]);
            }) as Box<dyn Fn(&mut serde_json::Value)>,
        ),
        (
            "extra",
            Box::new(|value: &mut serde_json::Value| {
                let duplicate = value["signatures"][0].clone();
                value["signatures"]
                    .as_array_mut()
                    .expect("signature array")
                    .push(duplicate);
            }),
        ),
        (
            "ineligible",
            Box::new(|value: &mut serde_json::Value| {
                value["signatures"][0]["signer_key_id"] = serde_json::json!("unknown-key");
            }),
        ),
        (
            "wrong digest",
            Box::new(|value: &mut serde_json::Value| {
                value["signatures"][0]["signed_record_sha256"] = serde_json::json!(sha('f'));
            }),
        ),
    ] {
        let mut value = base.clone();
        mutate(&mut value);
        let wire = serde_json::from_value(value).expect("structural proof wire");
        let error = PrincipalRecoveryProofV1::admit_wire(
            wire,
            &PrincipalRecoveryProofAdmissionContextV1 {
                expected_schema_sha256: &preimage.schema_sha256,
                prior_principal_record_ref: genesis.principal_record_ref(),
                prior_principal_record: genesis.principal_record(),
                recovery_policy_ref: genesis.recovery_policy_ref(),
                recovery_policy: genesis.recovery_policy(),
                presented_at: UnixTimeV1::new(30).expect("presentation time"),
            },
            |_, _, _, _, _| Ok(()),
        )
        .expect_err(name);
        assert!(matches!(
            error,
            InvalidResidentRecordV1::ContextualAdmission { .. }
        ));
    }
}

#[test]
fn recovery_proof_enforces_policy_and_presentation_windows() {
    let genesis = genesis();
    for (issued_at, expires_at, presented_at) in [(20, 400, 30), (20, 100, 100), (901, 950, 920)] {
        let mut preimage = proof_preimage(&genesis);
        preimage.issued_at = UnixTimeV1::new(issued_at).expect("issue time");
        preimage.expires_at = UnixTimeV1::new(expires_at).expect("expiry time");
        let digest = PrincipalRecoveryProofDigest::hash_canonical(&preimage).expect("proof digest");
        let wire = serde_json::from_value(proof_wire_value(&preimage, &digest))
            .expect("proof wire decodes");
        let error = PrincipalRecoveryProofV1::admit_wire(
            wire,
            &PrincipalRecoveryProofAdmissionContextV1 {
                expected_schema_sha256: &preimage.schema_sha256,
                prior_principal_record_ref: genesis.principal_record_ref(),
                prior_principal_record: genesis.principal_record(),
                recovery_policy_ref: genesis.recovery_policy_ref(),
                recovery_policy: genesis.recovery_policy(),
                presented_at: UnixTimeV1::new(presented_at).expect("presentation time"),
            },
            |_, _, _, _, _| Ok(()),
        )
        .expect_err("invalid proof window");
        assert!(matches!(
            error,
            InvalidResidentRecordV1::ContextualAdmission { .. }
        ));
    }
}

#[test]
fn recovery_proof_wire_rejects_null_and_unknown_members() {
    let genesis = genesis();
    let preimage = proof_preimage(&genesis);
    let digest = PrincipalRecoveryProofDigest::hash_canonical(&preimage).expect("proof digest");
    let base = proof_wire_value(&preimage, &digest);

    let mut null = base.clone();
    null["nonce"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<PrincipalRecoveryProofWireV1>(null).is_err());

    let mut unknown = base;
    unknown["consumed"] = serde_json::json!(true);
    assert!(serde_json::from_value::<PrincipalRecoveryProofWireV1>(unknown).is_err());
}

#[test]
fn recovery_proof_jcs_vector_is_stable() {
    let genesis = genesis();
    let digest = PrincipalRecoveryProofDigest::hash_canonical(&proof_preimage(&genesis))
        .expect("proof digest");
    assert_eq!(
        digest.as_str(),
        "bcd65d009ea9cbcdeb7ee30a1dc9b203bbf87c6f5b696292b1f6ae3ed4287154"
    );
}
