//! Shared admission fixture for resident application tests.
//!
//! This is the test-only owner of the complete bootstrap ceremony needed to
//! obtain an admitted initial trust frontier without weakening sealed record
//! constructors.

use pale_core::records::*;
use serde::de::DeserializeOwned;
use serde::Serialize;

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn artifact(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(sha(byte)).expect("artifact digest")
}

fn schema_digest(byte: char) -> SchemaDigest {
    SchemaDigest::parse(sha(byte)).expect("schema digest")
}

fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("record reference")
}

fn principal() -> ResidentPrincipalIdV1 {
    ResidentPrincipalIdV1::parse("principal-1").expect("principal")
}

fn root_key_id() -> ResidentKeyIdV1 {
    ResidentKeyIdV1::parse("root-key-1").expect("root key id")
}

fn root_public_key() -> Ed25519PublicKeyV1 {
    Ed25519PublicKeyV1::parse("A".repeat(43)).expect("root public key")
}

fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("signature")
}

pub(crate) fn signed_wire<P, D, W>(preimage: &P, digest_field: &str, digest: &D) -> W
where
    P: Serialize,
    D: Serialize,
    W: DeserializeOwned,
{
    let mut value = serde_json::to_value(preimage).expect("preimage serializes");
    let object = value.as_object_mut().expect("preimage is an object");
    object.insert(
        digest_field.into(),
        serde_json::to_value(digest).expect("digest serializes"),
    );
    object.insert(
        "signature".into(),
        serde_json::to_value(signature()).expect("signature serializes"),
    );
    serde_json::from_value(value).expect("strict signed wire decodes")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BootstrapSignatureStage {
    PrincipalRecord,
    RecoveryPolicy,
    LocalAdminAuthority,
    ProducerAuthority,
    CheckpointAuthority,
    TrustCheckpoint,
    ControlCheckpoint,
    BootstrapReceipt,
}

fn verify_bootstrap_signature(
    rejected: Option<BootstrapSignatureStage>,
    stage: BootstrapSignatureStage,
    observed_domain: &[u8],
    expected_domain: &[u8],
    observed_key: &Ed25519PublicKeyV1,
    expected_key: &Ed25519PublicKeyV1,
) -> Result<(), InvalidResidentRecordV1> {
    assert_eq!(observed_domain, expected_domain);
    assert_eq!(observed_key, expected_key);
    if rejected == Some(stage) {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "test signature",
            invariant: "authenticated",
        })
    } else {
        Ok(())
    }
}

fn recovery_policy_preimage() -> PrincipalRecoveryPolicyPreimageV1 {
    PrincipalRecoveryPolicyPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RECOVERY_POLICY_V1_SCHEMA).expect("recovery schema"),
        schema_sha256: schema_digest('1'),
        recovery_policy_id: PrincipalRecoveryPolicyIdV1::parse("recovery-policy-1")
            .expect("policy id"),
        principal_id: principal(),
        policy_epoch: PrincipalRecoveryPolicyEpochV1::new(1).expect("policy epoch"),
        eligible_signers: vec![PrincipalRecoveryEligibleSignerV1 {
            recovery_principal_id: principal(),
            recovery_key_id: ResidentKeyIdV1::parse("recovery-key-1").expect("recovery key id"),
            recovery_public_key: Ed25519PublicKeyV1::parse("E".repeat(43))
                .expect("recovery public key"),
            recovery_key_epoch: PrincipalKeyEpochV1::new(1).expect("recovery key epoch"),
        }],
        threshold: 1,
        proof_validity_seconds: RecoveryProofValiditySecondsV1::new(300).expect("proof validity"),
        replay_scope: PrincipalRecoveryReplayScopeV1::PrincipalKeyRecoveryProof,
        issued_at: UnixTimeV1::new(10).expect("policy issue time"),
        expires_at: Some(UnixTimeV1::new(900).expect("policy expiry")),
        previous_policy_ref: None,
        previous_policy_sha256: None,
        issuer_key_id: root_key_id(),
        issuer_principal_key_epoch: PrincipalKeyEpochV1::new(1).expect("root key epoch"),
    }
}

fn principal_record_preimage(
    recovery_policy_ref: ResidentRecordRefV1,
    recovery_policy_sha256: PrincipalRecoveryPolicyDigest,
) -> PrincipalRecordPreimageV1 {
    PrincipalRecordPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_RECORD_V1_SCHEMA).expect("principal schema"),
        schema_sha256: schema_digest('2'),
        principal_id: principal(),
        display_hint: BoundedResidentTextV1::parse("Local principal").expect("display hint"),
        root_key_id: root_key_id(),
        root_public_key: root_public_key(),
        key_epoch: PrincipalKeyEpochV1::new(1).expect("root key epoch"),
        record_transition_kind: PrincipalRecordTransitionKindV1::Initial,
        previous_record_sha256: None,
        key_transition_ref: None,
        key_transition_sha256: None,
        issued_at: UnixTimeV1::new(10).expect("principal issue time"),
        expires_at: UnixTimeV1::new(1_000).expect("principal expiry"),
        recovery_policy_ref,
        recovery_policy_sha256,
    }
}

fn admit_genesis_with_recovery_rejecting(
    recovery_preimage: PrincipalRecoveryPolicyPreimageV1,
    rejected: Option<BootstrapSignatureStage>,
) -> Result<AdmittedPrincipalGenesisV1, InvalidResidentRecordV1> {
    let recovery_policy_ref = record_ref("recovery-policy-1");
    let recovery_digest =
        PrincipalRecoveryPolicyDigest::hash_canonical(&recovery_preimage).expect("recovery digest");
    let principal_preimage =
        principal_record_preimage(recovery_policy_ref.clone(), recovery_digest.clone());
    let principal_digest =
        PrincipalRecordDigest::hash_canonical(&principal_preimage).expect("principal digest");
    AdmittedPrincipalGenesisV1::admit_initial(
        InitialPrincipalGenesisCandidateV1 {
            principal_record_ref: record_ref("principal-record-1"),
            principal_wire: signed_wire(
                &principal_preimage,
                "canonical_record_sha256",
                &principal_digest,
            ),
            expected_principal_schema_sha256: &principal_preimage.schema_sha256,
            recovery_policy_ref,
            recovery_wire: signed_wire(
                &recovery_preimage,
                "canonical_record_sha256",
                &recovery_digest,
            ),
            expected_recovery_schema_sha256: &recovery_preimage.schema_sha256,
        },
        |domain, key, _, _, _| {
            verify_bootstrap_signature(
                rejected,
                BootstrapSignatureStage::PrincipalRecord,
                domain,
                PRINCIPAL_RECORD_V1_SIGNATURE_DOMAIN,
                key,
                &root_public_key(),
            )
        },
        |domain, key, _, _, _| {
            verify_bootstrap_signature(
                rejected,
                BootstrapSignatureStage::RecoveryPolicy,
                domain,
                PRINCIPAL_RECOVERY_POLICY_V1_SIGNATURE_DOMAIN,
                key,
                &root_public_key(),
            )
        },
    )
}

fn admit_authorities_rejecting(
    bootstrap_id: &PrincipalBootstrapIdV1,
    rejected: Option<BootstrapSignatureStage>,
) -> Result<AdmittedPrincipalAuthoritiesV1, InvalidResidentRecordV1> {
    admit_authorities_for_profile(bootstrap_id, rejected, None)
}

fn admit_authorities_for_profile(
    bootstrap_id: &PrincipalBootstrapIdV1,
    rejected: Option<BootstrapSignatureStage>,
    profile: Option<&PrincipalBootstrapCheckpointProfileV1<'_>>,
) -> Result<AdmittedPrincipalAuthoritiesV1, InvalidResidentRecordV1> {
    let genesis = admit_genesis_with_recovery_rejecting(recovery_policy_preimage(), rejected)?;
    let principal_record = genesis.principal_record();
    let local_admin_preimage = LocalAdminAuthorityPreimageV1 {
        schema_id: SchemaId::parse(LOCAL_ADMIN_AUTHORITY_V1_SCHEMA).expect("admin schema"),
        schema_sha256: schema_digest('3'),
        admin_authority_id: LocalAdminAuthorityIdV1::parse("admin-authority-1")
            .expect("admin authority id"),
        principal_id: principal(),
        principal_record_ref: genesis.principal_record_ref().clone(),
        principal_record_sha256: principal_record.canonical_record_sha256().clone(),
        recovery_policy_ref: genesis.recovery_policy_ref().clone(),
        recovery_policy_sha256: genesis.recovery_policy().canonical_record_sha256().clone(),
        subject_identity: LocalAdminSubjectIdentityV1::parse("local-admin-1")
            .expect("admin subject"),
        subject_key_id: ResidentKeyIdV1::parse("admin-key-1").expect("admin key"),
        subject_key_epoch: SigningKeyEpochV1::new(1).expect("admin key epoch"),
        subject_public_key: Ed25519PublicKeyV1::parse("I".repeat(43)).expect("admin public key"),
        allowed_operations: vec![
            LocalAdminOperationV1::RotatePrincipalKey,
            LocalAdminOperationV1::FinalizeDeployment,
        ],
        state: PrincipalAuthorityStateV1::Active,
        issued_at: UnixTimeV1::new(20).expect("admin issue time"),
        expires_at: UnixTimeV1::new(800).expect("admin expiry"),
        authority_epoch: LocalAdminAuthorityEpochV1::new(1).expect("admin epoch"),
        previous_authority_ref: None,
        previous_authority_sha256: None,
        issuer_principal_id: principal(),
        issuer_key_id: root_key_id(),
        issuer_key_epoch: PrincipalKeyEpochV1::new(1).expect("root epoch"),
    };
    let producer_preimage = ProducerIncarnationAuthorityPreimageV1 {
        schema_id: SchemaId::parse(PRODUCER_INCARNATION_AUTHORITY_V1_SCHEMA)
            .expect("producer authority schema"),
        schema_sha256: schema_digest('4'),
        producer_authority_id: ProducerIncarnationAuthorityIdV1::parse("producer-authority-1")
            .expect("producer authority id"),
        principal_id: principal(),
        subject_key_id: ResidentKeyIdV1::parse("producer-key-1").expect("producer key"),
        subject_key_epoch: SigningKeyEpochV1::new(1).expect("producer key epoch"),
        subject_public_key: Ed25519PublicKeyV1::parse("M".repeat(43)).expect("producer public key"),
        permitted_producer_namespaces: vec![
            ProducerNamespaceV1::parse("pale").expect("producer namespace")
        ],
        authority_epoch: ProducerIncarnationAuthorityEpochV1::new(1)
            .expect("producer authority epoch"),
        previous_authority_ref: None,
        previous_authority_sha256: None,
        predecessor_final_publication_frontier_ref: None,
        predecessor_final_publication_frontier_sha256: None,
        state: PrincipalAuthorityStateV1::Active,
        issued_at: UnixTimeV1::new(20).expect("producer issue time"),
        expires_at: UnixTimeV1::new(800).expect("producer expiry"),
        issuer_principal_id: principal(),
        issuer_key_id: root_key_id(),
        issuer_key_epoch: PrincipalKeyEpochV1::new(1).expect("root epoch"),
    };
    let mut checkpoint_preimage = CheckpointSigningAuthorityPreimageV1 {
        schema_id: SchemaId::parse(CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA)
            .expect("checkpoint authority schema"),
        schema_sha256: schema_digest('5'),
        checkpoint_signing_authority_id: CheckpointSigningAuthorityIdV1::parse(
            "checkpoint-authority-1",
        )
        .expect("checkpoint authority id"),
        principal_id: principal(),
        control_store_id: ControlStoreIdV1::parse("principal-control-store-1")
            .expect("principal control store"),
        control_incarnation_id: ControlIncarnationIdV1::parse("principal-control-incarnation-1")
            .expect("principal control incarnation"),
        scope: CheckpointSigningScopeV1::PrincipalTrustStore {
            bootstrap_id: bootstrap_id.clone(),
        },
        checkpoint_profile_ref: record_ref("checkpoint-profile-1"),
        checkpoint_profile_sha256: CheckpointProfileDigest::parse(sha('6'))
            .expect("checkpoint profile digest"),
        allowed_domain_prefixes: vec![
            CheckpointDomainPrefixV1::parse("admin-authority").expect("domain prefix"),
            CheckpointDomainPrefixV1::parse("principal-head").expect("domain prefix"),
        ],
        subject_key_id: ResidentKeyIdV1::parse("checkpoint-key-1").expect("checkpoint key"),
        subject_key_epoch: SigningKeyEpochV1::new(1).expect("checkpoint key epoch"),
        subject_public_key: Ed25519PublicKeyV1::parse("Q".repeat(43))
            .expect("checkpoint public key"),
        signer_epoch: CheckpointSignerEpochV1::new(1).expect("checkpoint signer epoch"),
        state: PrincipalAuthorityStateV1::Active,
        issued_at: UnixTimeV1::new(20).expect("checkpoint authority issue time"),
        expires_at: UnixTimeV1::new(800).expect("checkpoint authority expiry"),
        previous_authority_ref: None,
        previous_authority_sha256: None,
        issuer_principal_id: principal(),
        issuer_key_id: root_key_id(),
        issuer_key_epoch: PrincipalKeyEpochV1::new(1).expect("root epoch"),
    };
    if let Some(profile) = profile {
        checkpoint_preimage.control_store_id = profile.principal_control_store_id.clone();
        checkpoint_preimage.control_incarnation_id =
            profile.principal_control_incarnation_id.clone();
        checkpoint_preimage.checkpoint_profile_ref = profile.checkpoint_profile_ref.clone();
        checkpoint_preimage.checkpoint_profile_sha256 = profile.checkpoint_profile_sha256.clone();
        checkpoint_preimage.allowed_domain_prefixes = profile
            .required_checkpoint_domains
            .iter()
            .map(|domain| CheckpointDomainPrefixV1::parse(domain.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
    }
    let local_admin_digest = LocalAdminAuthorityDigest::hash_canonical(&local_admin_preimage)
        .expect("admin authority digest");
    let producer_digest = ProducerIncarnationAuthorityDigest::hash_canonical(&producer_preimage)
        .expect("producer authority digest");
    let checkpoint_digest = CheckpointSigningAuthorityDigest::hash_canonical(&checkpoint_preimage)
        .expect("checkpoint authority digest");
    genesis.admit_initial_authorities(
        InitialPrincipalAuthoritiesCandidateV1 {
            local_admin_authority_ref: record_ref("admin-authority-1"),
            local_admin_wire: signed_wire(
                &local_admin_preimage,
                "canonical_admin_authority_sha256",
                &local_admin_digest,
            ),
            expected_local_admin_schema_sha256: &local_admin_preimage.schema_sha256,
            producer_authority_ref: record_ref("producer-authority-1"),
            producer_wire: signed_wire(
                &producer_preimage,
                "canonical_producer_authority_sha256",
                &producer_digest,
            ),
            expected_producer_schema_sha256: &producer_preimage.schema_sha256,
            checkpoint_authority_ref: record_ref("checkpoint-authority-1"),
            checkpoint_wire: signed_wire(
                &checkpoint_preimage,
                "canonical_checkpoint_signing_authority_sha256",
                &checkpoint_digest,
            ),
            expected_checkpoint_schema_sha256: &checkpoint_preimage.schema_sha256,
            expected_bootstrap_id: bootstrap_id,
        },
        |domain, key, _, _, _| {
            verify_bootstrap_signature(
                rejected,
                BootstrapSignatureStage::LocalAdminAuthority,
                domain,
                LOCAL_ADMIN_AUTHORITY_V1_SIGNATURE_DOMAIN,
                key,
                &root_public_key(),
            )
        },
        |domain, key, _, _, _| {
            verify_bootstrap_signature(
                rejected,
                BootstrapSignatureStage::ProducerAuthority,
                domain,
                PRODUCER_INCARNATION_AUTHORITY_V1_SIGNATURE_DOMAIN,
                key,
                &root_public_key(),
            )
        },
        |domain, key, _, _, _| {
            verify_bootstrap_signature(
                rejected,
                BootstrapSignatureStage::CheckpointAuthority,
                domain,
                CHECKPOINT_SIGNING_AUTHORITY_V1_SIGNATURE_DOMAIN,
                key,
                &root_public_key(),
            )
        },
    )
}

/// Reuse the bootstrap ceremony with an explicitly selected test profile.
pub(crate) fn admit_profile_authorities(
    bootstrap_id: &PrincipalBootstrapIdV1,
    profile: &PrincipalBootstrapCheckpointProfileV1<'_>,
) -> Result<AdmittedPrincipalAuthoritiesV1, InvalidResidentRecordV1> {
    admit_authorities_for_profile(bootstrap_id, None, Some(profile))
}

/// Admitted bootstrap atom and its authored trust input.
pub(crate) struct AdmittedBootstrapFixture {
    pub(crate) admitted: AdmittedPrincipalBootstrapV1,
    pub(crate) trust_preimage: PrincipalTrustCheckpointPreimageV1,
}

fn admit_bootstrap_with_receipt_ref_rejecting(
    bootstrap: &str,
    receipt_ref: ResidentRecordRefV1,
    committed_at: u64,
    rejected: Option<BootstrapSignatureStage>,
) -> Result<
    (
        AdmittedBootstrapFixture,
        Vec<ControlStateCheckpointPreimageV1>,
    ),
    InvalidResidentRecordV1,
> {
    let bootstrap_id = PrincipalBootstrapIdV1::parse(bootstrap).expect("bootstrap id");
    let authorities = admit_authorities_rejecting(&bootstrap_id, rejected)?;
    let domains = vec![
        ControlCheckpointDomainIdV1::parse("admin-authority").expect("domain"),
        ControlCheckpointDomainIdV1::parse("principal-head").expect("domain"),
    ];
    let trust_preimage = initial_trust_preimage(&authorities, None);
    let trust_digest = PrincipalTrustCheckpointDigest::hash_canonical(&trust_preimage)
        .expect("trust checkpoint digest");
    let checkpoint_schema_sha256 = schema_digest('8');
    let checkpoint_profile_ref = record_ref("checkpoint-profile-1");
    let checkpoint_profile_sha256 =
        CheckpointProfileDigest::parse(sha('6')).expect("checkpoint profile digest");
    let checkpoint_authority = authorities.checkpoint_authority();
    let checkpoint_authority_sha256 = checkpoint_authority
        .canonical_checkpoint_signing_authority_sha256()
        .clone();
    let checkpoint_subject_key_id = checkpoint_authority.subject_key_id().clone();
    let checkpoint_subject_key_epoch = *checkpoint_authority.subject_key_epoch();
    let checkpoint_subject_public_key = checkpoint_authority.subject_public_key().clone();
    let mut checkpoint_candidates = Vec::new();
    let mut control_preimages = Vec::new();
    let mut receipt_checkpoints = Vec::new();
    for (index, domain) in domains.iter().enumerate() {
        let preimage = ControlStateCheckpointPreimageV1 {
            schema_id: SchemaId::parse(CONTROL_STATE_CHECKPOINT_V1_SCHEMA)
                .expect("control checkpoint schema"),
            schema_sha256: checkpoint_schema_sha256.clone(),
            control_checkpoint_id: ControlCheckpointIdV1::parse(format!(
                "control-checkpoint-{}",
                index + 1
            ))
            .expect("control checkpoint id"),
            control_store_id: trust_preimage.principal_control_store_id.clone(),
            control_incarnation_id: trust_preimage.principal_control_incarnation_id.clone(),
            checkpoint_domain_id: domain.clone(),
            domain_epoch: ControlDomainEpochV1::new(1).expect("domain epoch"),
            domain_sequence: ControlDomainSequenceV1::new(1).expect("domain sequence"),
            previous_control_checkpoint_ref: None,
            previous_control_checkpoint_sha256: None,
            checkpoint_profile_ref: checkpoint_profile_ref.clone(),
            checkpoint_profile_sha256: checkpoint_profile_sha256.clone(),
            control_schema_version: 1,
            // Both domain checkpoints close the same post-initialization atom.
            committed_transaction_sequence: ControlTransactionSequenceV1::new(2)
                .expect("one bootstrap transaction"),
            state_root_sha256: artifact(if index == 0 { '9' } else { 'a' }),
            leaf_count: ControlLeafCountV1::new(1).expect("leaf count"),
            changed_leaf_keys: vec![ChangedControlLeafKeyV1 {
                record_class: ControlRecordClassV1::parse("bootstrap-record")
                    .expect("record class"),
                scope_key_sha256: artifact('b'),
                logical_key: ControlLogicalKeyV1::parse(format!("leaf-{}", index + 1))
                    .expect("logical key"),
            }],
            unresolved_custody_root_sha256: artifact('c'),
            issued_at: UnixTimeV1::new(45).expect("control checkpoint issue time"),
            checkpoint_signing_authority_ref: authorities.checkpoint_authority_ref().clone(),
            checkpoint_signing_authority_sha256: checkpoint_authority_sha256.clone(),
            issuer_principal_id: principal(),
            issuer_key_id: checkpoint_subject_key_id.clone(),
            issuer_key_epoch: checkpoint_subject_key_epoch,
        };
        let digest = ControlStateCheckpointDigest::hash_canonical(&preimage)
            .expect("control checkpoint digest");
        let checkpoint_ref = record_ref(&format!("control-checkpoint-{}", index + 1));
        receipt_checkpoints.push(PrincipalBootstrapInitialControlCheckpointV1 {
            checkpoint_domain_id: domain.clone(),
            domain_epoch: preimage.domain_epoch,
            domain_sequence: preimage.domain_sequence,
            control_checkpoint_ref: checkpoint_ref.clone(),
            control_checkpoint_sha256: digest.clone(),
        });
        checkpoint_candidates.push((
            checkpoint_ref,
            signed_wire(&preimage, "canonical_control_checkpoint_sha256", &digest),
        ));
        control_preimages.push(preimage);
    }
    let profile = PrincipalBootstrapCheckpointProfileV1 {
        trust_store_id: &trust_preimage.trust_store_id,
        trust_store_incarnation_id: &trust_preimage.trust_store_incarnation_id,
        principal_control_store_id: &trust_preimage.principal_control_store_id,
        principal_control_incarnation_id: &trust_preimage.principal_control_incarnation_id,
        checkpoint_profile_ref: &checkpoint_profile_ref,
        checkpoint_profile_sha256: &checkpoint_profile_sha256,
        required_checkpoint_domains: &domains,
        control_checkpoint_schema_sha256: &checkpoint_schema_sha256,
        control_schema_version: 1,
    };
    let trust_checkpoint_ref = record_ref("trust-checkpoint-1");
    let frontier = authorities.admit_initial_trust_frontier(
        InitialPrincipalTrustFrontierCandidateV1 {
            trust_checkpoint_ref: trust_checkpoint_ref.clone(),
            trust_wire: signed_wire(
                &trust_preimage,
                "canonical_trust_checkpoint_sha256",
                &trust_digest,
            ),
            expected_trust_schema_sha256: &trust_preimage.schema_sha256,
            profile,
            checkpoint_candidates,
        },
        |domain, key, _, _, _| {
            verify_bootstrap_signature(
                rejected,
                BootstrapSignatureStage::TrustCheckpoint,
                domain,
                PRINCIPAL_TRUST_CHECKPOINT_V1_SIGNATURE_DOMAIN,
                key,
                &root_public_key(),
            )
        },
        |domain, key, _, _, _| {
            verify_bootstrap_signature(
                rejected,
                BootstrapSignatureStage::ControlCheckpoint,
                domain,
                CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN,
                key,
                &checkpoint_subject_public_key,
            )
        },
        |prefixes, domain| {
            prefixes
                .iter()
                .any(|prefix| prefix.as_str() == domain.as_str())
        },
    )?;

    let fixture_trust_preimage = trust_preimage.clone();
    let receipt_preimage = bootstrap_receipt_preimage(
        bootstrap_id,
        trust_preimage,
        checkpoint_authority_sha256,
        trust_checkpoint_ref,
        trust_digest,
        receipt_checkpoints,
        committed_at,
    );
    let receipt_digest = PrincipalBootstrapReceiptDigest::hash_canonical(&receipt_preimage)
        .expect("bootstrap receipt digest");
    frontier
        .admit_receipt(
            receipt_ref,
            signed_wire(
                &receipt_preimage,
                "canonical_bootstrap_receipt_sha256",
                &receipt_digest,
            ),
            &receipt_preimage.schema_sha256,
            |domain, key, _, _, _| {
                verify_bootstrap_signature(
                    rejected,
                    BootstrapSignatureStage::BootstrapReceipt,
                    domain,
                    PRINCIPAL_BOOTSTRAP_RECEIPT_V1_SIGNATURE_DOMAIN,
                    key,
                    &root_public_key(),
                )
            },
        )
        .map(|admitted| {
            (
                AdmittedBootstrapFixture {
                    admitted,
                    trust_preimage: fixture_trust_preimage,
                },
                control_preimages,
            )
        })
}

fn checkpoint_authority_ref() -> ResidentRecordRefV1 {
    record_ref("checkpoint-authority-1")
}

pub(crate) fn admitted_bootstrap_fixture() -> (
    AdmittedBootstrapFixture,
    Vec<ControlStateCheckpointPreimageV1>,
) {
    admit_bootstrap_with_receipt_ref_rejecting(
        "bootstrap-producer-fixture",
        record_ref("bootstrap-producer-receipt"),
        50,
        None,
    )
    .expect("principal-trust bootstrap fixture admits")
}

/// Build the initial trust head from the same admitted authority ceremony.
pub(crate) fn initial_trust_preimage(
    authorities: &AdmittedPrincipalAuthoritiesV1,
    profile: Option<&PrincipalBootstrapCheckpointProfileV1<'_>>,
) -> PrincipalTrustCheckpointPreimageV1 {
    let mut trust_preimage = PrincipalTrustCheckpointPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA)
            .expect("trust checkpoint schema"),
        schema_sha256: schema_digest('7'),
        trust_checkpoint_id: PrincipalTrustCheckpointIdV1::parse("trust-checkpoint-1")
            .expect("trust checkpoint id"),
        principal_id: principal(),
        trust_store_id: ControlStoreIdV1::parse("trust-store-1").expect("trust store"),
        trust_store_incarnation_id: ControlIncarnationIdV1::parse("trust-incarnation-1")
            .expect("trust incarnation"),
        principal_control_store_id: ControlStoreIdV1::parse("principal-control-store-1")
            .expect("principal control store"),
        principal_control_incarnation_id: ControlIncarnationIdV1::parse(
            "principal-control-incarnation-1",
        )
        .expect("principal control incarnation"),
        checkpoint_epoch: PrincipalTrustCheckpointEpochV1::new(1).expect("trust checkpoint epoch"),
        previous_trust_checkpoint_ref: None,
        previous_trust_checkpoint_sha256: None,
        principal_record_ref: authorities.genesis().principal_record_ref().clone(),
        principal_record_sha256: authorities
            .genesis()
            .principal_record()
            .canonical_record_sha256()
            .clone(),
        recovery_policy_ref: authorities.genesis().recovery_policy_ref().clone(),
        recovery_policy_sha256: authorities
            .genesis()
            .recovery_policy()
            .canonical_record_sha256()
            .clone(),
        local_admin_authority_ref: authorities.local_admin_authority_ref().clone(),
        local_admin_authority_sha256: authorities
            .local_admin_authority()
            .canonical_admin_authority_sha256()
            .clone(),
        producer_incarnation_authority_ref: authorities.producer_authority_ref().clone(),
        producer_incarnation_authority_sha256: authorities
            .producer_authority()
            .canonical_producer_authority_sha256()
            .clone(),
        producer_incarnation_heads_root_sha256: ProducerIncarnationHeadsRootDigest::hash_canonical(
            &[] as &[()],
        )
        .expect("empty producer root"),
        session_floor_epoch: PrincipalSessionFloorEpochV1::new(1).expect("session floor"),
        checkpoint_signing_authority_heads_root_sha256: authorities
            .checkpoint_authority_heads_root()
            .expect("checkpoint authority root"),
        deployment_runtime_authority_heads_root_sha256:
            DeploymentRuntimeAuthorityHeadsRootDigest::hash_canonical(&[] as &[()])
                .expect("empty runtime authority root"),
        owner_record_signing_authority_heads_root_sha256:
            OwnerRecordSigningAuthorityHeadsRootDigest::hash_canonical(&[] as &[()])
                .expect("empty owner authority root"),
        owner_signer_publication_frontier_heads_root_sha256:
            OwnerSignerPublicationFrontierHeadsRootDigest::hash_canonical(&[] as &[()])
                .expect("empty owner frontier root"),
        deployment_acceptance_revocation_heads_root_sha256:
            DeploymentAcceptanceRevocationHeadsRootDigest::hash_canonical(&[] as &[()])
                .expect("empty acceptance revocation root"),
        federation_space_home_heads_root_sha256:
            FederationSpaceHomeHeadsRootDigest::hash_canonical(&[] as &[()])
                .expect("empty federation home root"),
        node_delegation_heads_root_sha256: NodeDelegationHeadsRootDigest::hash_canonical(
            &[] as &[()]
        )
        .expect("empty node delegation root"),
        node_delegation_revocation_heads_root_sha256:
            NodeDelegationRevocationHeadsRootDigest::hash_canonical(&[] as &[()])
                .expect("empty node revocation root"),
        issued_at: UnixTimeV1::new(40).expect("trust checkpoint issue time"),
        issuer_principal_id: principal(),
        issuer_key_id: root_key_id(),
        issuer_key_epoch: PrincipalKeyEpochV1::new(1).expect("root epoch"),
    };
    if let Some(profile) = profile {
        trust_preimage.trust_store_id = profile.trust_store_id.clone();
        trust_preimage.trust_store_incarnation_id = profile.trust_store_incarnation_id.clone();
        trust_preimage.principal_control_store_id = profile.principal_control_store_id.clone();
        trust_preimage.principal_control_incarnation_id =
            profile.principal_control_incarnation_id.clone();
    }
    trust_preimage
}

/// Build the existing receipt from its exact admitted references and digests.
pub(crate) fn bootstrap_receipt_preimage(
    bootstrap_id: PrincipalBootstrapIdV1,
    trust_preimage: PrincipalTrustCheckpointPreimageV1,
    checkpoint_authority_sha256: CheckpointSigningAuthorityDigest,
    trust_checkpoint_ref: ResidentRecordRefV1,
    trust_digest: PrincipalTrustCheckpointDigest,
    receipt_checkpoints: Vec<PrincipalBootstrapInitialControlCheckpointV1>,
    committed_at: u64,
) -> PrincipalBootstrapReceiptPreimageV1 {
    PrincipalBootstrapReceiptPreimageV1 {
        schema_id: SchemaId::parse(PRINCIPAL_BOOTSTRAP_RECEIPT_V1_SCHEMA)
            .expect("bootstrap receipt schema"),
        schema_sha256: schema_digest('d'),
        bootstrap_id,
        trust_store_id: trust_preimage.trust_store_id,
        trust_store_incarnation_id: trust_preimage.trust_store_incarnation_id,
        principal_control_store_id: trust_preimage.principal_control_store_id,
        principal_control_incarnation_id: trust_preimage.principal_control_incarnation_id,
        principal_record_ref: trust_preimage.principal_record_ref,
        principal_record_sha256: trust_preimage.principal_record_sha256,
        recovery_policy_ref: trust_preimage.recovery_policy_ref,
        recovery_policy_sha256: trust_preimage.recovery_policy_sha256,
        initial_admin_authority_ref: trust_preimage.local_admin_authority_ref,
        initial_admin_authority_sha256: trust_preimage.local_admin_authority_sha256,
        initial_producer_incarnation_authority_ref: trust_preimage
            .producer_incarnation_authority_ref,
        initial_producer_incarnation_authority_sha256: trust_preimage
            .producer_incarnation_authority_sha256,
        initial_trust_store_checkpoint_signing_authority_ref: checkpoint_authority_ref(),
        initial_trust_store_checkpoint_signing_authority_sha256: checkpoint_authority_sha256,
        trust_checkpoint_ref,
        trust_checkpoint_sha256: trust_digest,
        trust_checkpoint_epoch: PrincipalTrustCheckpointEpochV1::new(1)
            .expect("trust checkpoint epoch"),
        initial_control_checkpoints: receipt_checkpoints,
        committed_at: UnixTimeV1::new(committed_at).expect("commit time"),
        candidate_root_key_id: root_key_id(),
    }
}
