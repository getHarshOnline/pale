//! Root-delegated local administration, producer, and checkpoint authorities.
//!
//! Each authority is an immutable signed succession record. Bootstrap admits
//! only the active epoch-one/no-predecessor branch. Rotation and retirement
//! remain later owner transactions; representing those states here does not
//! authorize either transition.

use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

/// Public schema root for local-administration authority wires.
pub const LOCAL_ADMIN_AUTHORITY_V1_SCHEMA_ID: &str = "local-admin-authority.v1";
/// Wire schema carried by every local-administration authority.
pub const LOCAL_ADMIN_AUTHORITY_V1_SCHEMA: &str = "pale/local-admin-authority.v1";
/// Signature domain for [`LocalAdminAuthorityV1`].
pub const LOCAL_ADMIN_AUTHORITY_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_LOCAL_ADMIN_AUTHORITY_V1\0";

/// Public schema root for producer-incarnation authority wires.
pub const PRODUCER_INCARNATION_AUTHORITY_V1_SCHEMA_ID: &str = "producer-incarnation-authority.v1";
/// Wire schema carried by every producer-incarnation authority.
pub const PRODUCER_INCARNATION_AUTHORITY_V1_SCHEMA: &str = "pale/producer-incarnation-authority.v1";
/// Signature domain for [`ProducerIncarnationAuthorityV1`].
pub const PRODUCER_INCARNATION_AUTHORITY_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_PRODUCER_INCARNATION_AUTHORITY_V1\0";

/// Public schema root for checkpoint-signing authority wires.
pub const CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA_ID: &str = "checkpoint-signing-authority.v1";
/// Wire schema carried by every checkpoint-signing authority.
pub const CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA: &str = "pale/checkpoint-signing-authority.v1";
/// Signature domain for [`CheckpointSigningAuthorityV1`].
pub const CHECKPOINT_SIGNING_AUTHORITY_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_CHECKPOINT_SIGNING_AUTHORITY_V1\0";

/// Lifecycle state shared by root-delegated trust authorities.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalAuthorityStateV1 {
    /// The authority may be used within its exact scope and validity interval.
    Active,
    /// A successor transaction permanently closed this authority epoch.
    Retired,
}

/// Closed local-administration operation vocabulary.
///
/// The allowlist authorizes submission only. Every candidate record retains
/// its independent signer, semantic owner, and admission law.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum LocalAdminOperationV1 {
    /// Submit a normal principal-key rotation candidate set.
    RotatePrincipalKey,
    /// Submit an authenticated principal-key recovery candidate set.
    RecoverPrincipalKey,
    /// Submit a same-key recovery-policy successor.
    UpdatePrincipalRecoveryPolicy,
    /// Rotate or retire the local administrator authority.
    RotateAdminAuthority,
    /// Rotate or retire the producer-incarnation authority.
    RotateProducerIncarnationAuthority,
    /// Submit a principal-signed node delegation.
    IssueNodeDelegation,
    /// Submit a principal-signed node-delegation revocation.
    RevokeNodeDelegation,
    /// Rotate or retire one checkpoint signer.
    RotateCheckpointSigner,
    /// Rotate or retire one deployment runtime authority.
    RotateDeploymentRuntimeAuthority,
    /// Submit a new owner-record signing authority.
    IssueOwnerRecordSigningAuthority,
    /// Rotate one owner-record signing authority.
    RotateOwnerRecordSigningAuthority,
    /// Retire one owner-record signing authority.
    RetireOwnerRecordSigningAuthority,
    /// Submit a producer-authority-signed incarnation allocation.
    AllocateProducerIncarnation,
    /// Submit a producer-authority-signed incarnation retirement.
    RetireProducerIncarnation,
    /// Create the first generation of one local route.
    CreateLocalRoute,
    /// Advance one local route generation.
    AdvanceLocalRoute,
    /// Submit an immutable deployment proposal.
    ProposeDeployment,
    /// Submit an independently signed deployment acceptance.
    AcceptDeployment,
    /// Submit an independently signed acceptance revocation.
    RevokeDeploymentAcceptance,
    /// Finalize one accepted deployment and its trust-store authorities.
    FinalizeDeployment,
    /// Issue one bounded capability session grant.
    IssueSessionGrant,
    /// Revoke one capability session grant.
    RevokeSessionGrant,
    /// Advance an independently owner-authorized execution fence.
    AdvanceExecutionFence,
    /// Issue one managed-resource apply authorization.
    IssueApplyAuthorization,
    /// Create one federation route binding.
    CreateRouteBinding,
    /// Advance one federation route binding.
    AdvanceRouteBinding,
}

/// Exact scope of one [`CheckpointSigningAuthorityV1`].
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CheckpointSigningScopeV1 {
    /// Bootstrap-scoped signer for the principal trust store.
    PrincipalTrustStore {
        /// Exact offline bootstrap transaction that introduced the signer.
        bootstrap_id: PrincipalBootstrapIdV1,
    },
    /// Store-bound signer introduced by an admitted deployment binding.
    Deployment {
        /// Exact immutable deployment binding.
        deployment_binding_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-binding bytes.
        deployment_binding_sha256: DeploymentBindingDigest,
    },
}

resident_signed_record!(
    /// Principal-root-signed authority for authenticated local administration.
    LocalAdminAuthorityV1,
    LocalAdminAuthorityWireV1,
    LocalAdminAuthorityPreimageV1,
    canonical_admin_authority_sha256: LocalAdminAuthorityDigest,
    LOCAL_ADMIN_AUTHORITY_V1_SCHEMA,
    "local admin authority",
    {
        /// Stable identity shared by this authority lineage.
        admin_authority_id: LocalAdminAuthorityIdV1,
        /// Principal whose local trust state may be administered.
        principal_id: ResidentPrincipalIdV1,
        /// Exact principal record active when this authority was issued.
        principal_record_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the principal record.
        principal_record_sha256: PrincipalRecordDigest,
        /// Exact recovery policy active when this authority was issued.
        recovery_policy_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the recovery policy.
        recovery_policy_sha256: PrincipalRecoveryPolicyDigest,
        /// Stable identity of the administrator subject.
        subject_identity: LocalAdminSubjectIdentityV1,
        /// Stable identity of the delegated subject key.
        subject_key_id: ResidentKeyIdV1,
        /// Positive delegated subject-key epoch.
        subject_key_epoch: SigningKeyEpochV1,
        /// Exact Ed25519 subject public key.
        subject_public_key: Ed25519PublicKeyV1,
        /// Canonically ordered operations the subject may submit.
        allowed_operations: Vec<LocalAdminOperationV1>,
        /// Active or terminal lifecycle state.
        state: PrincipalAuthorityStateV1,
        /// Unix issue time in whole seconds.
        issued_at: UnixTimeV1,
        /// Unix expiry time for new submissions.
        expires_at: UnixTimeV1,
        /// Positive authority succession epoch.
        authority_epoch: LocalAdminAuthorityEpochV1,
        /// Exact predecessor authority, absent only at epoch one.
        previous_authority_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_authority_ref`].
        previous_authority_sha256: Option<LocalAdminAuthorityDigest>,
        /// Principal whose active accepted key issued this authority.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Active accepted root-key identity.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive accepted root-key epoch.
        issuer_key_epoch: PrincipalKeyEpochV1
    },
    validate_local_admin_authority_shape
);

resident_signed_record!(
    /// Root-delegated online authority for producer-incarnation records.
    ProducerIncarnationAuthorityV1,
    ProducerIncarnationAuthorityWireV1,
    ProducerIncarnationAuthorityPreimageV1,
    canonical_producer_authority_sha256: ProducerIncarnationAuthorityDigest,
    PRODUCER_INCARNATION_AUTHORITY_V1_SCHEMA,
    "producer incarnation authority",
    {
        /// Stable identity shared by this authority lineage.
        producer_authority_id: ProducerIncarnationAuthorityIdV1,
        /// Principal whose producer epochs this authority may allocate.
        principal_id: ResidentPrincipalIdV1,
        /// Stable delegated subject-key identity.
        subject_key_id: ResidentKeyIdV1,
        /// Positive delegated subject-key epoch.
        subject_key_epoch: SigningKeyEpochV1,
        /// Exact Ed25519 subject public key.
        subject_public_key: Ed25519PublicKeyV1,
        /// Non-empty canonical producer namespace rule set.
        permitted_producer_namespaces: Vec<ProducerNamespaceV1>,
        /// Positive authority succession epoch.
        authority_epoch: ProducerIncarnationAuthorityEpochV1,
        /// Exact predecessor authority, absent only at epoch one.
        previous_authority_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_authority_ref`].
        previous_authority_sha256: Option<ProducerIncarnationAuthorityDigest>,
        /// Immutable final predecessor publication frontier when one is closed.
        predecessor_final_publication_frontier_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::predecessor_final_publication_frontier_ref`].
        predecessor_final_publication_frontier_sha256: Option<ArtifactDigest>,
        /// Active or terminal lifecycle state.
        state: PrincipalAuthorityStateV1,
        /// Unix issue time in whole seconds.
        issued_at: UnixTimeV1,
        /// Unix expiry time for new allocation or retirement records.
        expires_at: UnixTimeV1,
        /// Principal whose active accepted key issued this authority.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Active accepted root-key identity.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive accepted root-key epoch.
        issuer_key_epoch: PrincipalKeyEpochV1
    },
    validate_producer_authority_shape
);

resident_signed_record!(
    /// Root-delegated online authority for one exact checkpoint scope.
    CheckpointSigningAuthorityV1,
    CheckpointSigningAuthorityWireV1,
    CheckpointSigningAuthorityPreimageV1,
    canonical_checkpoint_signing_authority_sha256: CheckpointSigningAuthorityDigest,
    CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA,
    "checkpoint signing authority",
    {
        /// Stable identity shared by this signer lineage.
        checkpoint_signing_authority_id: CheckpointSigningAuthorityIdV1,
        /// Principal that owns the control store.
        principal_id: ResidentPrincipalIdV1,
        /// Exact logical control store covered by this signer.
        control_store_id: ControlStoreIdV1,
        /// Exact control-store incarnation covered by this signer.
        control_incarnation_id: ControlIncarnationIdV1,
        /// Discriminated principal-trust or deployment scope.
        scope: CheckpointSigningScopeV1,
        /// Immutable accumulator-profile reference.
        checkpoint_profile_ref: ResidentRecordRefV1,
        /// Digest of the exact checkpoint-profile bytes.
        checkpoint_profile_sha256: CheckpointProfileDigest,
        /// Non-empty canonical hard-domain prefix set.
        allowed_domain_prefixes: Vec<CheckpointDomainPrefixV1>,
        /// Stable delegated subject-key identity.
        subject_key_id: ResidentKeyIdV1,
        /// Positive delegated subject-key epoch.
        subject_key_epoch: SigningKeyEpochV1,
        /// Exact Ed25519 subject public key.
        subject_public_key: Ed25519PublicKeyV1,
        /// Positive signer succession epoch.
        signer_epoch: CheckpointSignerEpochV1,
        /// Active or terminal lifecycle state.
        state: PrincipalAuthorityStateV1,
        /// Unix issue time in whole seconds.
        issued_at: UnixTimeV1,
        /// Unix expiry time for new checkpoints.
        expires_at: UnixTimeV1,
        /// Exact predecessor authority, absent only at epoch one.
        previous_authority_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_authority_ref`].
        previous_authority_sha256: Option<CheckpointSigningAuthorityDigest>,
        /// Principal whose active accepted key issued this authority.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Active accepted root-key identity.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive accepted root-key epoch.
        issuer_key_epoch: PrincipalKeyEpochV1
    },
    validate_checkpoint_authority_shape
);

/// Canonical head projection committed by a principal trust checkpoint.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct CheckpointSigningAuthorityHeadV1 {
    /// Stable signer-lineage identity.
    pub checkpoint_signing_authority_id: CheckpointSigningAuthorityIdV1,
    /// Logical control store covered by the authority.
    pub control_store_id: ControlStoreIdV1,
    /// Exact control-store incarnation covered by the authority.
    pub control_incarnation_id: ControlIncarnationIdV1,
    /// Exact authority scope.
    pub scope: CheckpointSigningScopeV1,
    /// Immutable authority record reference.
    pub authority_ref: ResidentRecordRefV1,
    /// Canonical unsigned-preimage digest of the authority.
    pub authority_sha256: CheckpointSigningAuthorityDigest,
    /// Positive signer succession epoch.
    pub signer_epoch: CheckpointSignerEpochV1,
}

/// Hash a supplied active signer-head array in signer-lineage order.
///
/// This verifies ordering and the shared collection bound, not membership in
/// a trust checkpoint or admission of the records named by these projections.
///
/// # Errors
/// Rejects duplicate or unordered lineage IDs, excessive heads, or failed
/// canonical encoding.
pub fn checkpoint_signing_authority_heads_root_sha256(
    heads: &[CheckpointSigningAuthorityHeadV1],
) -> Result<CheckpointSigningAuthorityHeadsRootDigest, InvalidResidentRecordV1> {
    bounded_len("checkpoint signing authority heads", heads.len())?;
    if !heads.windows(2).all(|pair| {
        pair[0].checkpoint_signing_authority_id < pair[1].checkpoint_signing_authority_id
    }) {
        return Err(InvalidResidentRecordV1::NonCanonical {
            relation: "checkpoint signing authority heads",
        });
    }
    Ok(CheckpointSigningAuthorityHeadsRootDigest::hash_canonical(
        heads,
    )?)
}

/// Exact binding, store and immutable profile selected for a deployment signer.
///
/// Selection is supplied by the admitting owner; this value performs no lookup,
/// authenticates no administrator and does not activate a deployment.
#[derive(Clone, Copy)]
pub struct DeploymentCheckpointSigningAdmissionContextV1<'a> {
    /// Expected checkpoint-signing-authority schema identity.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Immutable reference of the admitted deployment binding.
    pub deployment_binding_ref: &'a ResidentRecordRefV1,
    /// Binding already admitted against its proposal and owner acceptance.
    pub deployment_binding: &'a DeploymentBindingV1,
    /// Selected deployment control store.
    pub control_store_id: &'a ControlStoreIdV1,
    /// Selected store incarnation.
    pub control_incarnation_id: &'a ControlIncarnationIdV1,
    /// Reference of the independently resolved supported checkpoint profile.
    pub checkpoint_profile_ref: &'a ResidentRecordRefV1,
    /// Digest of that exact profile content.
    pub checkpoint_profile_sha256: &'a CheckpointProfileDigest,
    /// Active principal key resolved from the accepted trust frontier.
    pub signer: PrincipalKeyAdmissionContextV1<'a>,
}

impl CheckpointSigningAuthorityV1 {
    /// Admit an active epoch-one checkpoint signer for an admitted deployment binding.
    ///
    /// `resolve_domain_prefix` must establish that each delegated prefix belongs
    /// to the selected supported profile. `verify_signature` authenticates the
    /// existing signature domain and canonical digest with the selected root
    /// key. This does not establish deployment finalization, command permission
    /// or database custody. The caller must resolve the current principal key
    /// and the exact supported profile before calling this method.
    /// Record admission checks the delegation's own ordered validity interval.
    /// Its use still requires the binding and delegation to be valid at that
    /// operation's time; this method does not read a clock or renew either.
    ///
    /// # Errors
    /// Rejects substituted binding, principal, store, profile, root-key or
    /// initial-state coordinates, unsupported domain permission, or a rejected
    /// record digest/signature.
    pub fn admit_initial_deployment_wire<R, F>(
        wire: CheckpointSigningAuthorityWireV1,
        context: &DeploymentCheckpointSigningAdmissionContextV1<'_>,
        mut resolve_domain_prefix: R,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        R: FnMut(&CheckpointDomainPrefixV1) -> Result<(), InvalidResidentRecordV1>,
        F: FnOnce(
            &[u8],
            &CheckpointSigningAuthorityPreimageV1,
            &CheckpointSigningAuthorityDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        context.deployment_binding.verify()?;
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_principal_signed_coordinates(
                "checkpoint signing authority",
                context.expected_schema_sha256,
                PrincipalSignedCoordinatesV1 {
                    schema_sha256: &preimage.schema_sha256,
                    subject_principal_id: &preimage.principal_id,
                    issuer_principal_id: &preimage.issuer_principal_id,
                    issuer_key_id: &preimage.issuer_key_id,
                    issuer_key_epoch: preimage.issuer_key_epoch,
                },
                &context.signer,
            )?;
            let exact_scope = matches!(
                &preimage.scope,
                CheckpointSigningScopeV1::Deployment {
                    deployment_binding_ref,
                    deployment_binding_sha256,
                } if deployment_binding_ref == context.deployment_binding_ref
                    && deployment_binding_sha256
                        == context.deployment_binding.canonical_binding_sha256()
            );
            if !exact_scope
                || &preimage.principal_id
                    != context
                        .deployment_binding
                        .deployment_terms()
                        .principal_scope()
                || &preimage.control_store_id != context.control_store_id
                || &preimage.control_incarnation_id != context.control_incarnation_id
                || &preimage.checkpoint_profile_ref != context.checkpoint_profile_ref
                || &preimage.checkpoint_profile_sha256 != context.checkpoint_profile_sha256
                || preimage.state != PrincipalAuthorityStateV1::Active
                || preimage.signer_epoch.get() != 1
                || preimage.previous_authority_ref.is_some()
            {
                return Err(contextual(
                    "checkpoint signing authority",
                    "exact active initial deployment, store, and profile",
                ));
            }
            for prefix in &preimage.allowed_domain_prefixes {
                resolve_domain_prefix(prefix)?;
            }
            verify_signature(
                CHECKPOINT_SIGNING_AUTHORITY_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }

    /// Project this active record into its existing canonical head shape.
    ///
    /// # Errors
    /// Rejects an invalid record or retired authority. This is not proof that
    /// the head is current in any principal trust store.
    pub fn active_head(
        &self,
        authority_ref: ResidentRecordRefV1,
    ) -> Result<CheckpointSigningAuthorityHeadV1, InvalidResidentRecordV1> {
        self.verify()?;
        if self.state() != &PrincipalAuthorityStateV1::Active {
            return Err(contextual("checkpoint signing authority", "active head"));
        }
        Ok(CheckpointSigningAuthorityHeadV1 {
            checkpoint_signing_authority_id: self.checkpoint_signing_authority_id().clone(),
            control_store_id: self.control_store_id().clone(),
            control_incarnation_id: self.control_incarnation_id().clone(),
            scope: self.scope().clone(),
            authority_ref,
            authority_sha256: self.canonical_checkpoint_signing_authority_sha256().clone(),
            signer_epoch: *self.signer_epoch(),
        })
    }
}

/// Untrusted root-delegated authority set presented for initial admission.
///
/// The three authority records form one bootstrap stage but retain distinct
/// wire types and schema identities. Constructing this value neither verifies
/// a signature nor grants any authority.
pub struct InitialPrincipalAuthoritiesCandidateV1<'schema> {
    /// Immutable reference assigned to the local-administration authority.
    pub local_admin_authority_ref: ResidentRecordRefV1,
    /// Signed but not yet admitted local-administration authority.
    pub local_admin_wire: LocalAdminAuthorityWireV1,
    /// Expected schema identity for the local-administration authority.
    pub expected_local_admin_schema_sha256: &'schema SchemaDigest,
    /// Immutable reference assigned to the producer-incarnation authority.
    pub producer_authority_ref: ResidentRecordRefV1,
    /// Signed but not yet admitted producer-incarnation authority.
    pub producer_wire: ProducerIncarnationAuthorityWireV1,
    /// Expected schema identity for the producer-incarnation authority.
    pub expected_producer_schema_sha256: &'schema SchemaDigest,
    /// Immutable reference assigned to the checkpoint-signing authority.
    pub checkpoint_authority_ref: ResidentRecordRefV1,
    /// Signed but not yet admitted checkpoint-signing authority.
    pub checkpoint_wire: CheckpointSigningAuthorityWireV1,
    /// Expected schema identity for the checkpoint-signing authority.
    pub expected_checkpoint_schema_sha256: &'schema SchemaDigest,
    /// Bootstrap transaction that must scope the checkpoint authority.
    pub expected_bootstrap_id: &'schema PrincipalBootstrapIdV1,
}

/// Admitted initial root-delegated authority set and its principal genesis.
pub struct AdmittedPrincipalAuthoritiesV1 {
    genesis: AdmittedPrincipalGenesisV1,
    local_admin_authority_ref: ResidentRecordRefV1,
    local_admin_authority: LocalAdminAuthorityV1,
    producer_authority_ref: ResidentRecordRefV1,
    producer_authority: ProducerIncarnationAuthorityV1,
    checkpoint_authority_ref: ResidentRecordRefV1,
    checkpoint_authority: CheckpointSigningAuthorityV1,
}

impl AdmittedPrincipalGenesisV1 {
    /// Admit the three epoch-one active root delegations required by bootstrap.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when any authority is not the exact
    /// active epoch-one/no-predecessor delegation bound to this genesis, or a
    /// signature callback rejects its root-key signature.
    pub fn admit_initial_authorities<AS, PS, CS>(
        self,
        candidate: InitialPrincipalAuthoritiesCandidateV1<'_>,
        verify_admin_signature: AS,
        verify_producer_signature: PS,
        verify_checkpoint_signature: CS,
    ) -> Result<AdmittedPrincipalAuthoritiesV1, InvalidResidentRecordV1>
    where
        AS: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &LocalAdminAuthorityPreimageV1,
            &LocalAdminAuthorityDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        PS: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &ProducerIncarnationAuthorityPreimageV1,
            &ProducerIncarnationAuthorityDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        CS: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &CheckpointSigningAuthorityPreimageV1,
            &CheckpointSigningAuthorityDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let InitialPrincipalAuthoritiesCandidateV1 {
            local_admin_authority_ref,
            local_admin_wire,
            expected_local_admin_schema_sha256,
            producer_authority_ref,
            producer_wire,
            expected_producer_schema_sha256,
            checkpoint_authority_ref,
            checkpoint_wire,
            expected_checkpoint_schema_sha256,
            expected_bootstrap_id,
        } = candidate;
        let root_key = self.principal_record().root_public_key();
        let local_admin_authority =
            LocalAdminAuthorityV1::admit_wire(local_admin_wire, |preimage, digest, signature| {
                validate_initial_admin_context(
                    preimage,
                    &self,
                    expected_local_admin_schema_sha256,
                )?;
                verify_admin_signature(
                    LOCAL_ADMIN_AUTHORITY_V1_SIGNATURE_DOMAIN,
                    root_key,
                    preimage,
                    digest,
                    signature,
                )
            })?;
        let producer_authority = ProducerIncarnationAuthorityV1::admit_wire(
            producer_wire,
            |preimage, digest, signature| {
                validate_initial_producer_context(
                    preimage,
                    &self,
                    expected_producer_schema_sha256,
                )?;
                verify_producer_signature(
                    PRODUCER_INCARNATION_AUTHORITY_V1_SIGNATURE_DOMAIN,
                    root_key,
                    preimage,
                    digest,
                    signature,
                )
            },
        )?;
        let checkpoint_authority = CheckpointSigningAuthorityV1::admit_wire(
            checkpoint_wire,
            |preimage, digest, signature| {
                validate_initial_checkpoint_context(
                    preimage,
                    &self,
                    expected_checkpoint_schema_sha256,
                    expected_bootstrap_id,
                )?;
                verify_checkpoint_signature(
                    CHECKPOINT_SIGNING_AUTHORITY_V1_SIGNATURE_DOMAIN,
                    root_key,
                    preimage,
                    digest,
                    signature,
                )
            },
        )?;
        Ok(AdmittedPrincipalAuthoritiesV1 {
            genesis: self,
            local_admin_authority_ref,
            local_admin_authority,
            producer_authority_ref,
            producer_authority,
            checkpoint_authority_ref,
            checkpoint_authority,
        })
    }
}

impl AdmittedPrincipalAuthoritiesV1 {
    /// Borrow the admitted principal genesis pair.
    #[must_use]
    pub fn genesis(&self) -> &AdmittedPrincipalGenesisV1 {
        &self.genesis
    }

    /// Borrow the immutable local-admin authority reference.
    #[must_use]
    pub fn local_admin_authority_ref(&self) -> &ResidentRecordRefV1 {
        &self.local_admin_authority_ref
    }

    /// Borrow the admitted initial local-admin authority.
    #[must_use]
    pub fn local_admin_authority(&self) -> &LocalAdminAuthorityV1 {
        &self.local_admin_authority
    }

    /// Borrow the immutable producer-authority reference.
    #[must_use]
    pub fn producer_authority_ref(&self) -> &ResidentRecordRefV1 {
        &self.producer_authority_ref
    }

    /// Borrow the admitted initial producer-incarnation authority.
    #[must_use]
    pub fn producer_authority(&self) -> &ProducerIncarnationAuthorityV1 {
        &self.producer_authority
    }

    /// Borrow the immutable checkpoint-authority reference.
    #[must_use]
    pub fn checkpoint_authority_ref(&self) -> &ResidentRecordRefV1 {
        &self.checkpoint_authority_ref
    }

    /// Borrow the admitted initial checkpoint-signing authority.
    #[must_use]
    pub fn checkpoint_authority(&self) -> &CheckpointSigningAuthorityV1 {
        &self.checkpoint_authority
    }

    /// Reproduce the canonical singleton checkpoint-signer head root.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] if the retained authority cannot be
    /// projected or the canonical head array cannot be encoded.
    pub fn checkpoint_authority_heads_root(
        &self,
    ) -> Result<CheckpointSigningAuthorityHeadsRootDigest, InvalidResidentRecordV1> {
        checkpoint_signing_authority_heads_root_sha256(&[self
            .checkpoint_authority
            .active_head(self.checkpoint_authority_ref.clone())?])
    }
}

fn validate_local_admin_authority_shape(
    value: &LocalAdminAuthorityPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    strictly_sorted("local admin allowed operations", &value.allowed_operations)?;
    validate_authority_lineage(
        "local admin authority",
        AuthorityLineageV1 {
            epoch: value.authority_epoch.get(),
            previous_ref: value.previous_authority_ref.as_ref(),
            previous_digest: value.previous_authority_sha256.as_ref(),
            issued_at: value.issued_at,
            expires_at: value.expires_at,
            principal_id: &value.principal_id,
            issuer_principal_id: &value.issuer_principal_id,
        },
    )
}

fn validate_producer_authority_shape(
    value: &ProducerIncarnationAuthorityPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    strictly_sorted(
        "producer authority namespaces",
        &value.permitted_producer_namespaces,
    )?;
    if value.permitted_producer_namespaces.is_empty()
        || value.predecessor_final_publication_frontier_ref.is_some()
            != value
                .predecessor_final_publication_frontier_sha256
                .is_some()
        || (value.authority_epoch.get() == 1
            && value.predecessor_final_publication_frontier_ref.is_some())
    {
        return Err(contextual(
            "producer incarnation authority",
            "non-empty namespaces and canonical predecessor frontier",
        ));
    }
    validate_authority_lineage(
        "producer incarnation authority",
        AuthorityLineageV1 {
            epoch: value.authority_epoch.get(),
            previous_ref: value.previous_authority_ref.as_ref(),
            previous_digest: value.previous_authority_sha256.as_ref(),
            issued_at: value.issued_at,
            expires_at: value.expires_at,
            principal_id: &value.principal_id,
            issuer_principal_id: &value.issuer_principal_id,
        },
    )
}

fn validate_checkpoint_authority_shape(
    value: &CheckpointSigningAuthorityPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    strictly_sorted(
        "checkpoint authority domain prefixes",
        &value.allowed_domain_prefixes,
    )?;
    if value.allowed_domain_prefixes.is_empty() {
        return Err(contextual(
            "checkpoint signing authority",
            "non-empty allowed domain prefixes",
        ));
    }
    validate_authority_lineage(
        "checkpoint signing authority",
        AuthorityLineageV1 {
            epoch: value.signer_epoch.get(),
            previous_ref: value.previous_authority_ref.as_ref(),
            previous_digest: value.previous_authority_sha256.as_ref(),
            issued_at: value.issued_at,
            expires_at: value.expires_at,
            principal_id: &value.principal_id,
            issuer_principal_id: &value.issuer_principal_id,
        },
    )
}

/// Borrowed successor coordinates shared by the three authority families.
struct AuthorityLineageV1<'record, Digest> {
    epoch: u64,
    previous_ref: Option<&'record ResidentRecordRefV1>,
    previous_digest: Option<&'record Digest>,
    issued_at: UnixTimeV1,
    expires_at: UnixTimeV1,
    principal_id: &'record ResidentPrincipalIdV1,
    issuer_principal_id: &'record ResidentPrincipalIdV1,
}

fn validate_authority_lineage<Digest>(
    record: &'static str,
    lineage: AuthorityLineageV1<'_, Digest>,
) -> Result<(), InvalidResidentRecordV1> {
    let valid_lineage = matches!(
        (lineage.epoch, lineage.previous_ref, lineage.previous_digest),
        (1, None, None) | (2.., Some(_), Some(_))
    );
    if !valid_lineage
        || lineage.expires_at <= lineage.issued_at
        || lineage.principal_id != lineage.issuer_principal_id
    {
        Err(contextual(
            record,
            "epoch lineage, validity interval, and same-principal issuer",
        ))
    } else {
        Ok(())
    }
}

fn validate_initial_admin_context(
    value: &LocalAdminAuthorityPreimageV1,
    genesis: &AdmittedPrincipalGenesisV1,
    expected_schema_sha256: &SchemaDigest,
) -> Result<(), InvalidResidentRecordV1> {
    let principal = genesis.principal_record();
    let policy = genesis.recovery_policy();
    if value.schema_sha256 != *expected_schema_sha256
        || value.state != PrincipalAuthorityStateV1::Active
        || value.authority_epoch.get() != 1
        || value.allowed_operations.is_empty()
        || value.principal_id != *principal.principal_id()
        || value.principal_record_ref != *genesis.principal_record_ref()
        || value.principal_record_sha256 != *principal.canonical_record_sha256()
        || value.recovery_policy_ref != *genesis.recovery_policy_ref()
        || value.recovery_policy_sha256 != *policy.canonical_record_sha256()
        || value.issuer_key_id != *principal.root_key_id()
        || value.issuer_key_epoch != *principal.key_epoch()
    {
        Err(contextual(
            "local admin authority",
            "exact active initial principal delegation",
        ))
    } else {
        Ok(())
    }
}

fn validate_initial_producer_context(
    value: &ProducerIncarnationAuthorityPreimageV1,
    genesis: &AdmittedPrincipalGenesisV1,
    expected_schema_sha256: &SchemaDigest,
) -> Result<(), InvalidResidentRecordV1> {
    let principal = genesis.principal_record();
    if value.schema_sha256 != *expected_schema_sha256
        || value.state != PrincipalAuthorityStateV1::Active
        || value.authority_epoch.get() != 1
        || value.principal_id != *principal.principal_id()
        || value.issuer_key_id != *principal.root_key_id()
        || value.issuer_key_epoch != *principal.key_epoch()
    {
        Err(contextual(
            "producer incarnation authority",
            "exact active initial principal delegation",
        ))
    } else {
        Ok(())
    }
}

fn validate_initial_checkpoint_context(
    value: &CheckpointSigningAuthorityPreimageV1,
    genesis: &AdmittedPrincipalGenesisV1,
    expected_schema_sha256: &SchemaDigest,
    expected_bootstrap_id: &PrincipalBootstrapIdV1,
) -> Result<(), InvalidResidentRecordV1> {
    let principal = genesis.principal_record();
    let bootstrap_scope = matches!(
        &value.scope,
        CheckpointSigningScopeV1::PrincipalTrustStore { bootstrap_id }
            if bootstrap_id == expected_bootstrap_id
    );
    if value.schema_sha256 != *expected_schema_sha256
        || value.state != PrincipalAuthorityStateV1::Active
        || value.signer_epoch.get() != 1
        || !bootstrap_scope
        || value.principal_id != *principal.principal_id()
        || value.issuer_key_id != *principal.root_key_id()
        || value.issuer_key_epoch != *principal.key_epoch()
    {
        Err(contextual(
            "checkpoint signing authority",
            "exact active bootstrap-scoped principal delegation",
        ))
    } else {
        Ok(())
    }
}
