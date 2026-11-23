//! Runtime-signed evidence for one deployment's initial control checkpoints.
//! Admission joins exact candidates; physical co-commit and independent anchor
//! completion remain separate application/native obligations.

use serde::Serialize;

use super::*;

/// Shared five-field initial-checkpoint member; reuses the bootstrap wire shape.
/// The alias leaves the existing generated bootstrap schema and bytes unchanged.
pub type InitialControlCheckpointV1 = PrincipalBootstrapInitialControlCheckpointV1;

/// Public schema root for a deployment genesis receipt.
pub const DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA_ID: &str =
    "deployment-control-store-genesis-receipt.v1";
/// Wire schema carried by a deployment genesis receipt.
pub const DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA: &str =
    "pale/deployment-control-store-genesis-receipt.v1";
/// Domain signed by the exact admitted deployment runtime key, not the checkpoint key.
pub const DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1\0";

resident_signed_record!(
    /// Candidate receipt binding an exact deployment genesis checkpoint set.
    DeploymentControlStoreGenesisReceiptV1,
    DeploymentControlStoreGenesisReceiptWireV1,
    DeploymentControlStoreGenesisReceiptPreimageV1,
    canonical_deployment_control_store_genesis_receipt_sha256: DeploymentControlStoreGenesisReceiptDigest,
    DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA,
    "deployment control-store genesis receipt",
    {
        /// Stable identity reused for exact genesis recovery.
        genesis_id: DeploymentControlStoreGenesisIdV1,
        /// Principal that owns the deployment control store.
        principal_id: ResidentPrincipalIdV1,
        /// Exact workspace from the admitted binding terms.
        workspace_id: WorkspaceId,
        /// Immutable binding committed by the prior principal-store transaction.
        deployment_binding_ref: ResidentRecordRefV1,
        /// Canonical digest of that binding.
        deployment_binding_sha256: DeploymentBindingDigest,
        /// Logical deployment control store being initialized.
        control_store_id: ControlStoreIdV1,
        /// Exact incarnation being initialized.
        control_incarnation_id: ControlIncarnationIdV1,
        /// Delegated authority that signs the checkpoint records, not this receipt.
        checkpoint_signing_authority_ref: ResidentRecordRefV1,
        /// Canonical identity of the checkpoint signer delegation.
        checkpoint_signing_authority_sha256: CheckpointSigningAuthorityDigest,
        /// Complete domain-ordered epoch-one/sequence-one checkpoint set.
        initial_control_checkpoints: Vec<InitialControlCheckpointV1>,
        /// Logical commit time in Unix seconds; not proof of physical commit.
        committed_at: UnixTimeV1,
        /// Exact delegated runtime authority that signs this receipt.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Canonical identity of the selected runtime delegation.
        deployment_runtime_authority_sha256: DeploymentRuntimeAuthorityDigest,
        /// Principal of the selected runtime signer.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Delegated online runtime key, distinct from the checkpoint key.
        issuer_key_id: ResidentKeyIdV1,
        /// Delegated runtime subject-key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_genesis_shape
);

/// Exact admitted dependencies of a deployment genesis candidate.
///
/// The caller resolves current authority from the accepted principal frontier.
/// This context cannot prove the prior principal-store commit, the new store's
/// atomic genesis transaction or the independent anchors.
pub struct DeploymentControlStoreGenesisAdmissionContextV1<'a> {
    /// Selected schema and owner-resolved integrated runtime signer binding.
    pub signed: SignedResidentAdmissionContextV1<'a>,
    /// Immutable reference of the admitted deployment binding.
    pub deployment_binding_ref: &'a ResidentRecordRefV1,
    /// Admitted binding whose complete terms fix principal and workspace.
    pub deployment_binding: &'a DeploymentBindingV1,
    /// Current admitted runtime delegation resolved by the signing owner.
    pub runtime_authority: &'a DeploymentRuntimeAuthorityV1,
    /// Immutable reference of the current checkpoint signer delegation.
    pub checkpoint_signing_authority_ref: &'a ResidentRecordRefV1,
    /// Current admitted checkpoint signer, used only for the checkpoint set.
    pub checkpoint_signing_authority: &'a CheckpointSigningAuthorityV1,
    /// Complete sorted required domains from the selected checkpoint profile.
    pub required_checkpoint_domains: &'a [ControlCheckpointDomainIdV1],
    /// Exact contextually admitted checkpoints in required-domain order.
    /// Their own admission establishes profile version and signer-domain coverage.
    pub initial_control_checkpoints:
        &'a [(&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1)],
}

impl DeploymentControlStoreGenesisReceiptV1 {
    /// Issue a receipt candidate under the exact deployment runtime key.
    ///
    /// The signer runs only after the binding, runtime authority, checkpoint
    /// signer, required domains and complete initial checkpoint set agree. The
    /// returned record is still only a candidate: it proves neither database
    /// commitment nor completion of the independent anti-rollback anchors.
    ///
    /// # Errors
    /// Rejects mismatched scopes, signers, schemas, validity, checkpoint sets,
    /// canonical digests or a signing failure.
    pub fn issue_contextual<F>(
        preimage: DeploymentControlStoreGenesisReceiptPreimageV1,
        context: &DeploymentControlStoreGenesisAdmissionContextV1<'_>,
        sign: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &DeploymentControlStoreGenesisReceiptPreimageV1,
            &DeploymentControlStoreGenesisReceiptDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        context.deployment_binding.verify()?;
        context.runtime_authority.verify()?;
        context.checkpoint_signing_authority.verify()?;
        strictly_sorted(
            "deployment genesis required domains",
            context.required_checkpoint_domains,
        )?;
        bounded_len(
            "deployment genesis checkpoints",
            context.initial_control_checkpoints.len(),
        )?;
        validate_genesis_shape(&preimage)?;
        validate_genesis_context(&preimage, context)?;
        let digest = DeploymentControlStoreGenesisReceiptDigest::hash_canonical(&preimage)?;
        let signature = sign(
            DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SIGNATURE_DOMAIN,
            &preimage,
            &digest,
        )?;
        Self::seal(preimage, signature)
    }

    /// Admit a receipt candidate under the exact runtime key and joined genesis set.
    ///
    /// `verify_signature` authenticates the runtime public key over the dedicated
    /// receipt domain followed by its canonical digest. Checkpoints must already
    /// be contextually admitted by their own signing/profile owner. No callback
    /// can waive the binding, store, time, issuer or complete-set joins here.
    ///
    /// # Errors
    /// Rejects mismatched scopes, signers, schemas, validity, checkpoint sets,
    /// canonical digests or a rejected runtime signature.
    pub fn admit_contextual_wire<F>(
        wire: DeploymentControlStoreGenesisReceiptWireV1,
        context: &DeploymentControlStoreGenesisAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &Ed25519PublicKeyV1,
            &[u8],
            &DeploymentControlStoreGenesisReceiptPreimageV1,
            &DeploymentControlStoreGenesisReceiptDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        context.deployment_binding.verify()?;
        context.runtime_authority.verify()?;
        context.checkpoint_signing_authority.verify()?;
        strictly_sorted(
            "deployment genesis required domains",
            context.required_checkpoint_domains,
        )?;
        bounded_len(
            "deployment genesis checkpoints",
            context.initial_control_checkpoints.len(),
        )?;
        Self::admit_wire(wire, |value, digest, signature| {
            validate_genesis_context(value, context)?;
            verify_signature(
                context.runtime_authority.subject_public_key(),
                DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SIGNATURE_DOMAIN,
                value,
                digest,
                signature,
            )
        })
    }
}

fn validate_genesis_shape(
    value: &DeploymentControlStoreGenesisReceiptPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    bounded_len(
        "deployment genesis checkpoint set",
        value.initial_control_checkpoints.len(),
    )?;
    if value.initial_control_checkpoints.is_empty()
        || !value
            .initial_control_checkpoints
            .windows(2)
            .all(|pair| pair[0].checkpoint_domain_id < pair[1].checkpoint_domain_id)
        || value
            .initial_control_checkpoints
            .iter()
            .any(|entry| entry.domain_epoch.get() != 1 || entry.domain_sequence.get() != 1)
        || value.principal_id != value.issuer_principal_id
    {
        return Err(contextual(
            "deployment control-store genesis receipt",
            "same principal and complete canonical initial checkpoint set",
        ));
    }
    Ok(())
}

fn validate_genesis_context(
    value: &DeploymentControlStoreGenesisReceiptPreimageV1,
    context: &DeploymentControlStoreGenesisAdmissionContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    const FAMILY: &str = "deployment control-store genesis receipt";
    validate_signed_coordinates(
        FAMILY,
        &value.schema_sha256,
        context.signed.expected_schema_sha256,
        &value.issuer_principal_id,
        &value.issuer_key_id,
        value.issuer_key_epoch,
        context.signed.signing_authority,
    )?;
    // The shared signer binding uses an artifact digest; convert only at this join.
    let signing_digest = ArtifactDigest::parse(value.deployment_runtime_authority_sha256.as_str())
        .map_err(|_| contextual(FAMILY, "runtime authority digest representation"))?;
    validate_integrated_runtime_authority_coordinates(
        FAMILY,
        &value.deployment_runtime_authority_ref,
        &signing_digest,
        context.signed.signing_authority,
    )?;
    let binding = context.deployment_binding;
    let runtime = context.runtime_authority;
    let checkpoint = context.checkpoint_signing_authority;
    let integrated = context
        .signed
        .signing_authority
        .integrated_runtime_authority()
        .ok_or_else(|| contextual(FAMILY, "integrated runtime authority"))?;
    if value.deployment_binding_ref != *context.deployment_binding_ref
        || value.deployment_binding_sha256 != *binding.canonical_binding_sha256()
        || value.principal_id != *binding.deployment_terms().principal_scope()
        || value.workspace_id != *binding.deployment_terms().workspace_id()
        || value.committed_at < *binding.issued_at()
        || value.committed_at >= *binding.expires_at()
        || integrated.deployment_binding_ref != value.deployment_binding_ref
        || integrated.deployment_binding_sha256.as_str() != value.deployment_binding_sha256.as_str()
        || runtime.canonical_deployment_runtime_authority_sha256()
            != &value.deployment_runtime_authority_sha256
        || runtime.deployment_binding_ref() != &value.deployment_binding_ref
        || runtime.deployment_binding_sha256() != &value.deployment_binding_sha256
        || runtime.principal_id() != &value.principal_id
        || runtime.control_store_id() != &value.control_store_id
        || runtime.control_incarnation_id() != &value.control_incarnation_id
        || runtime.subject_key_id() != &value.issuer_key_id
        || runtime.subject_key_epoch() != &value.issuer_key_epoch
        || runtime.state() != &PrincipalAuthorityStateV1::Active
        || value.committed_at < *runtime.issued_at()
        || value.committed_at >= *runtime.expires_at()
        || !runtime
            .permitted_record_schema_ids()
            .contains(&value.schema_id)
        || value.checkpoint_signing_authority_ref != *context.checkpoint_signing_authority_ref
        || value.checkpoint_signing_authority_sha256
            != *checkpoint.canonical_checkpoint_signing_authority_sha256()
        || checkpoint.principal_id() != &value.principal_id
        || checkpoint.control_store_id() != &value.control_store_id
        || checkpoint.control_incarnation_id() != &value.control_incarnation_id
        || checkpoint.state() != &PrincipalAuthorityStateV1::Active
        || value.committed_at < *checkpoint.issued_at()
        || value.committed_at >= *checkpoint.expires_at()
        || checkpoint.scope()
            != &(CheckpointSigningScopeV1::Deployment {
                deployment_binding_ref: value.deployment_binding_ref.clone(),
                deployment_binding_sha256: value.deployment_binding_sha256.clone(),
            })
        || context.required_checkpoint_domains.len() != value.initial_control_checkpoints.len()
        || context.initial_control_checkpoints.len() != value.initial_control_checkpoints.len()
    {
        return Err(contextual(
            FAMILY,
            "exact deployment, store, authorities and validity",
        ));
    }
    let mut transaction = None;
    let mut references = std::collections::BTreeSet::new();
    for ((entry, required_domain), (reference, record)) in value
        .initial_control_checkpoints
        .iter()
        .zip(context.required_checkpoint_domains)
        .zip(context.initial_control_checkpoints)
    {
        record.verify()?;
        if &entry.checkpoint_domain_id != required_domain
            || record.checkpoint_domain_id() != required_domain
            || entry.control_checkpoint_ref != **reference
            || entry.control_checkpoint_sha256 != *record.canonical_control_checkpoint_sha256()
            || !references.insert(*reference)
            || record.domain_epoch().get() != 1
            || record.domain_sequence().get() != 1
            || record.previous_control_checkpoint_ref().is_some()
            || record.previous_control_checkpoint_sha256().is_some()
            || record.control_store_id() != &value.control_store_id
            || record.control_incarnation_id() != &value.control_incarnation_id
            || record.checkpoint_profile_ref() != checkpoint.checkpoint_profile_ref()
            || record.checkpoint_profile_sha256() != checkpoint.checkpoint_profile_sha256()
            || record.checkpoint_signing_authority_ref() != &value.checkpoint_signing_authority_ref
            || record.checkpoint_signing_authority_sha256()
                != &value.checkpoint_signing_authority_sha256
            || record.issuer_principal_id() != checkpoint.principal_id()
            || record.issuer_key_id() != checkpoint.subject_key_id()
            || record.issuer_key_epoch() != checkpoint.subject_key_epoch()
            || record.issued_at() < checkpoint.issued_at()
            || record.issued_at() >= checkpoint.expires_at()
            || *record.issued_at() > value.committed_at
            || transaction.is_some_and(|prior| prior != *record.committed_transaction_sequence())
        {
            return Err(contextual(
                FAMILY,
                "exact complete co-transaction initial checkpoints",
            ));
        }
        transaction = Some(*record.committed_transaction_sequence());
    }
    Ok(())
}
