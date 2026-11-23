//! Deterministic trust fixtures shared by principal law tests.

use serde::de::DeserializeOwned;
use serde::Serialize;

use super::*;

pub(super) fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

pub(super) fn schema_digest(byte: char) -> SchemaDigest {
    SchemaDigest::parse(sha(byte)).expect("schema digest")
}

pub(super) fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("record reference")
}

pub(super) fn principal() -> ResidentPrincipalIdV1 {
    ResidentPrincipalIdV1::parse("principal-1").expect("principal")
}

pub(super) fn root_key_id() -> ResidentKeyIdV1 {
    ResidentKeyIdV1::parse("root-key-1").expect("root key id")
}

pub(super) fn root_public_key() -> Ed25519PublicKeyV1 {
    Ed25519PublicKeyV1::parse("A".repeat(43)).expect("root public key")
}

pub(super) fn recovery_public_key() -> Ed25519PublicKeyV1 {
    Ed25519PublicKeyV1::parse("E".repeat(43)).expect("recovery public key")
}

pub(super) fn new_public_key() -> Ed25519PublicKeyV1 {
    Ed25519PublicKeyV1::parse("I".repeat(43)).expect("new public key")
}

pub(super) fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("signature")
}

pub(super) fn signed_wire<P, D, W>(preimage: &P, digest: &D) -> W
where
    P: Serialize,
    D: Serialize,
    W: DeserializeOwned,
{
    let mut value = serde_json::to_value(preimage).expect("preimage serializes");
    let object = value.as_object_mut().expect("preimage object");
    object.insert(
        "canonical_record_sha256".into(),
        serde_json::to_value(digest).expect("digest serializes"),
    );
    object.insert(
        "signature".into(),
        serde_json::to_value(signature()).expect("signature serializes"),
    );
    serde_json::from_value(value).expect("signed wire decodes")
}

pub(super) fn recovery_policy_preimage() -> PrincipalRecoveryPolicyPreimageV1 {
    PrincipalRecoveryPolicyPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RECOVERY_POLICY_V1_SCHEMA).expect("policy schema"),
        schema_sha256: schema_digest('a'),
        recovery_policy_id: PrincipalRecoveryPolicyIdV1::parse("recovery-policy-1")
            .expect("policy id"),
        principal_id: principal(),
        policy_epoch: PrincipalRecoveryPolicyEpochV1::new(1).expect("policy epoch"),
        eligible_signers: vec![PrincipalRecoveryEligibleSignerV1 {
            recovery_principal_id: principal(),
            recovery_key_id: ResidentKeyIdV1::parse("recovery-key-1").expect("recovery key id"),
            recovery_public_key: recovery_public_key(),
            recovery_key_epoch: PrincipalKeyEpochV1::new(1).expect("recovery key epoch"),
        }],
        threshold: 1,
        proof_validity_seconds: RecoveryProofValiditySecondsV1::new(300).expect("proof validity"),
        replay_scope: PrincipalRecoveryReplayScopeV1::PrincipalKeyRecoveryProof,
        issued_at: UnixTimeV1::new(10).expect("policy issue"),
        expires_at: Some(UnixTimeV1::new(900).expect("policy expiry")),
        previous_policy_ref: None,
        previous_policy_sha256: None,
        issuer_key_id: root_key_id(),
        issuer_principal_key_epoch: PrincipalKeyEpochV1::new(1).expect("root epoch"),
    }
}

pub(super) fn principal_preimage(
    recovery_policy_sha256: PrincipalRecoveryPolicyDigest,
) -> PrincipalRecordPreimageV1 {
    PrincipalRecordPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RECORD_V1_SCHEMA).expect("principal schema"),
        schema_sha256: schema_digest('b'),
        principal_id: principal(),
        display_hint: BoundedResidentTextV1::parse("Local principal").expect("display hint"),
        root_key_id: root_key_id(),
        root_public_key: root_public_key(),
        key_epoch: PrincipalKeyEpochV1::new(1).expect("root epoch"),
        record_transition_kind: PrincipalRecordTransitionKindV1::Initial,
        previous_record_sha256: None,
        key_transition_ref: None,
        key_transition_sha256: None,
        issued_at: UnixTimeV1::new(10).expect("principal issue"),
        expires_at: UnixTimeV1::new(1_000).expect("principal expiry"),
        recovery_policy_ref: record_ref("recovery-policy-1"),
        recovery_policy_sha256,
    }
}

pub(super) fn genesis() -> AdmittedPrincipalGenesisV1 {
    let policy_preimage = recovery_policy_preimage();
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
        |domain, key, _, _, _| {
            assert_eq!(domain, PRINCIPAL_RECORD_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &root_public_key());
            Ok(())
        },
        |domain, key, _, _, _| {
            assert_eq!(domain, PRINCIPAL_RECOVERY_POLICY_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &root_public_key());
            Ok(())
        },
    )
    .expect("admit genesis")
}

pub(super) fn proof_preimage(
    genesis: &AdmittedPrincipalGenesisV1,
) -> PrincipalRecoveryProofPreimageV1 {
    PrincipalRecoveryProofPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RECOVERY_PROOF_V1_SCHEMA).expect("proof schema"),
        schema_sha256: schema_digest('c'),
        recovery_proof_id: PrincipalRecoveryProofIdV1::parse("recovery-proof-1").expect("proof id"),
        principal_id: principal(),
        prior_principal_record_ref: genesis.principal_record_ref().clone(),
        prior_principal_record_sha256: genesis.principal_record().canonical_record_sha256().clone(),
        recovery_policy_ref: genesis.recovery_policy_ref().clone(),
        recovery_policy_sha256: genesis.recovery_policy().canonical_record_sha256().clone(),
        new_key_id: ResidentKeyIdV1::parse("root-key-2").expect("new key id"),
        new_public_key: new_public_key(),
        new_key_epoch: PrincipalKeyEpochV1::new(2).expect("next key epoch"),
        nonce: PrincipalRecoveryNonceV1::parse("recovery-nonce-1").expect("nonce"),
        issued_at: UnixTimeV1::new(20).expect("proof issue"),
        expires_at: UnixTimeV1::new(100).expect("proof expiry"),
    }
}

pub(super) fn proof_wire_value(
    preimage: &PrincipalRecoveryProofPreimageV1,
    digest: &PrincipalRecoveryProofDigest,
) -> serde_json::Value {
    let mut value = serde_json::to_value(preimage).expect("proof preimage serializes");
    let object = value.as_object_mut().expect("proof object");
    object.insert(
        "canonical_record_sha256".into(),
        serde_json::to_value(digest).expect("proof digest serializes"),
    );
    object.insert(
        "signatures".into(),
        serde_json::json!([{
            "signer_principal_id": "principal-1",
            "signer_key_id": "recovery-key-1",
            "signer_key_epoch": 1,
            "signed_record_sha256": digest,
            "signature": "A".repeat(86),
        }]),
    );
    value
}

pub(super) fn admit_proof(genesis: &AdmittedPrincipalGenesisV1) -> PrincipalRecoveryProofV1 {
    let preimage = proof_preimage(genesis);
    let digest = PrincipalRecoveryProofDigest::hash_canonical(&preimage).expect("proof digest");
    let wire =
        serde_json::from_value(proof_wire_value(&preimage, &digest)).expect("proof wire decodes");
    PrincipalRecoveryProofV1::admit_wire(
        wire,
        &PrincipalRecoveryProofAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            prior_principal_record_ref: genesis.principal_record_ref(),
            prior_principal_record: genesis.principal_record(),
            recovery_policy_ref: genesis.recovery_policy_ref(),
            recovery_policy: genesis.recovery_policy(),
            presented_at: UnixTimeV1::new(30).expect("presentation time"),
        },
        |domain, key, _, observed, _| {
            assert_eq!(domain, PRINCIPAL_RECOVERY_PROOF_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &recovery_public_key());
            assert_eq!(observed, &digest);
            Ok(())
        },
    )
    .expect("admit proof")
}

pub(super) fn normal_transition_preimage(
    genesis: &AdmittedPrincipalGenesisV1,
) -> PrincipalKeyTransitionPreimageV1 {
    PrincipalKeyTransitionPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_KEY_TRANSITION_V1_SCHEMA).expect("transition schema"),
        schema_sha256: schema_digest('d'),
        principal_id: principal(),
        transition_mode: PrincipalKeyTransitionModeV1::NormalRotation,
        old_key_epoch: PrincipalKeyEpochV1::new(1).expect("old epoch"),
        new_key_epoch: PrincipalKeyEpochV1::new(2).expect("new epoch"),
        old_key_id: root_key_id(),
        new_key_id: ResidentKeyIdV1::parse("root-key-2").expect("new key id"),
        old_public_key: root_public_key(),
        new_public_key: new_public_key(),
        transition_reason: BoundedResidentTextV1::parse("scheduled rotation")
            .expect("transition reason"),
        previous_record_sha256: genesis.principal_record().canonical_record_sha256().clone(),
        recovery_authority_proof_ref: None,
        recovery_authority_proof_sha256: None,
        effective_at: UnixTimeV1::new(50).expect("effective time"),
    }
}

pub(super) fn transition_wire_value(
    preimage: &PrincipalKeyTransitionPreimageV1,
    digest: &PrincipalKeyTransitionDigest,
) -> serde_json::Value {
    let mut value = serde_json::to_value(preimage).expect("transition preimage serializes");
    let object = value.as_object_mut().expect("transition object");
    object.insert(
        "canonical_record_sha256".into(),
        serde_json::to_value(digest).expect("transition digest serializes"),
    );
    object.insert(
        "new_key_signature".into(),
        serde_json::to_value(signature()).expect("new signature serializes"),
    );
    if preimage.transition_mode == PrincipalKeyTransitionModeV1::NormalRotation {
        object.insert(
            "old_key_signature".into(),
            serde_json::to_value(signature()).expect("old signature serializes"),
        );
    }
    value
}

pub(super) fn admit_normal_transition(
    genesis: &AdmittedPrincipalGenesisV1,
) -> PrincipalKeyTransitionV1 {
    let preimage = normal_transition_preimage(genesis);
    let digest =
        PrincipalKeyTransitionDigest::hash_canonical(&preimage).expect("transition digest");
    let wire = serde_json::from_value(transition_wire_value(&preimage, &digest))
        .expect("transition wire decodes");
    PrincipalKeyTransitionV1::admit_normal_rotation_wire(
        wire,
        &PrincipalKeyTransitionAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            predecessor_ref: genesis.principal_record_ref(),
            predecessor: genesis.principal_record(),
        },
        |domain, key, _, observed, _| {
            assert_eq!(domain, PRINCIPAL_KEY_TRANSITION_OLD_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &root_public_key());
            assert_eq!(observed, &digest);
            Ok(())
        },
        |domain, key, _, observed, _| {
            assert_eq!(domain, PRINCIPAL_KEY_TRANSITION_NEW_V1_SIGNATURE_DOMAIN);
            assert_eq!(key, &new_public_key());
            assert_eq!(observed, &digest);
            Ok(())
        },
    )
    .expect("admit normal transition")
}
