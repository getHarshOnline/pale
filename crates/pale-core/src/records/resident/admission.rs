//! Session, local-route, and neutral execution-owner admission contracts.
//!
//! These records carry authority inputs and immutable succession coordinates.
//! They do not open a transport, dispatch work, acquire an owner reservation,
//! mutate a native system, or settle another owner's state.

use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

/// Public schema root for capability-session grants.
pub const CAPABILITY_SESSION_GRANT_V1_SCHEMA_ID: &str = "capability-session-grant.v1";
/// Wire schema carried by every capability-session grant.
pub const CAPABILITY_SESSION_GRANT_V1_SCHEMA: &str = "pale/capability-session-grant.v1";
/// Signature domain for [`CapabilitySessionGrantV1`].
pub const CAPABILITY_SESSION_GRANT_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_CAPABILITY_SESSION_GRANT_V1\0";

/// Public schema root for append-only session-grant revocations.
pub const CAPABILITY_SESSION_GRANT_REVOCATION_V1_SCHEMA_ID: &str =
    "capability-session-grant-revocation.v1";
/// Wire schema carried by every session-grant revocation.
pub const CAPABILITY_SESSION_GRANT_REVOCATION_V1_SCHEMA: &str =
    "pale/capability-session-grant-revocation.v1";
/// Signature domain for [`CapabilitySessionGrantRevocationV1`].
pub const CAPABILITY_SESSION_GRANT_REVOCATION_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_CAPABILITY_SESSION_GRANT_REVOCATION_V1\0";

/// Public schema root for capability-revocation checkpoints.
pub const CAPABILITY_REVOCATION_CHECKPOINT_V1_SCHEMA_ID: &str =
    "capability-revocation-checkpoint.v1";
/// Wire schema carried by every capability-revocation checkpoint.
pub const CAPABILITY_REVOCATION_CHECKPOINT_V1_SCHEMA: &str =
    "pale/capability-revocation-checkpoint.v1";
/// Signature domain for [`CapabilityRevocationCheckpointV1`].
pub const CAPABILITY_REVOCATION_CHECKPOINT_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_CAPABILITY_REVOCATION_CHECKPOINT_V1\0";

/// Public schema root for local route bindings.
pub const LOCAL_ROUTE_BINDING_V1_SCHEMA_ID: &str = "local-route-binding.v1";
/// Wire schema carried by every local route binding.
pub const LOCAL_ROUTE_BINDING_V1_SCHEMA: &str = "pale/local-route-binding.v1";
/// Signature domain for [`LocalRouteBindingV1`].
pub const LOCAL_ROUTE_BINDING_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_LOCAL_ROUTE_BINDING_V1\0";

/// Public schema root for neutral execution-owner fences.
pub const EXECUTION_OWNER_FENCE_V1_SCHEMA_ID: &str = "execution-owner-fence.v1";
/// Wire schema carried by every neutral execution-owner fence.
pub const EXECUTION_OWNER_FENCE_V1_SCHEMA: &str = "pale/execution-owner-fence.v1";
/// Signature domain for [`ExecutionOwnerFenceV1`].
pub const EXECUTION_OWNER_FENCE_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_EXECUTION_OWNER_FENCE_V1\0";

/// One subject key authorized by a capability-session grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilitySessionSubjectV1 {
    /// Stable admitted subject identity.
    pub identity: RequestCallerIdV1,
    /// Role under which the subject is admitted.
    pub role: RequestCallerRoleV1,
    /// Surface on which the subject acts.
    pub surface: RequestCallerSurfaceV1,
    /// Exact subject-key identity.
    pub subject_key_id: ResidentKeyIdV1,
    /// Positive subject-key epoch.
    pub subject_key_epoch: SigningKeyEpochV1,
    /// Exact Ed25519 public key admitted for the session.
    pub subject_public_key: Ed25519PublicKeyV1,
}

/// One immutable route generation admitted by a session grant.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct SessionRouteConstraintV1 {
    /// Exact route-binding schema identity.
    pub route_binding_schema_id: SchemaId,
    /// Immutable route-binding reference.
    pub route_binding_ref: ResidentRecordRefV1,
    /// Digest of the exact route-binding bytes.
    pub route_binding_sha256: ArtifactDigest,
    /// Stable route identity.
    pub route_id: ResidentRouteIdV1,
    /// Exact selected route generation.
    pub route_generation: RouteGenerationV1,
}

/// One immutable contract set admitted by a session grant.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct SessionContractSetConstraintV1 {
    /// Immutable contract-set reference.
    pub contract_set_ref: ResidentRecordRefV1,
    /// Digest of the exact contract-set bytes.
    pub contract_set_sha256: ContractSetDigest,
}

/// Transport classes that a local route may select.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum LocalTransportClassV1 {
    /// Calls remain inside one process boundary.
    InProcess,
    /// Local Unix-domain socket transport.
    UnixSocket,
    /// Local Windows named-pipe transport.
    NamedPipe,
    /// Loopback-only HTTP transport.
    LoopbackHttp,
    /// Transport remains native to the selected owner.
    OwnerNative,
    /// Authenticated remote transport between nodes of one principal.
    SamePrincipalRemote,
}

/// Exact routes and destinations a capability-session grant may use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilitySessionRouteConstraintsV1 {
    /// Exact origin principal.
    pub origin_principal_id: ResidentPrincipalIdV1,
    /// Exact origin node.
    pub origin_node_id: ResidentNodeIdV1,
    /// Canonically ordered destination-principal allowlist.
    pub destination_principal_ids: Vec<ResidentPrincipalIdV1>,
    /// Canonically ordered destination-node allowlist.
    pub destination_node_ids: Vec<ResidentNodeIdV1>,
    /// Canonically ordered immutable route generations.
    pub routes: Vec<SessionRouteConstraintV1>,
    /// Canonically ordered transport-class allowlist.
    pub transport_classes: Vec<LocalTransportClassV1>,
    /// Canonically ordered immutable contract-set allowlist.
    pub contract_sets: Vec<SessionContractSetConstraintV1>,
}

/// Maximum scope that one session grant may exercise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilitySessionScopeCeilingV1 {
    /// Canonically ordered principal scopes.
    pub principal_scopes: Vec<ResidentPrincipalIdV1>,
    /// Canonically ordered controller scopes.
    pub controller_scopes: Vec<ResidentControllerIdV1>,
    /// Canonically ordered relationship scopes; empty means non-relationship scope only.
    pub relationship_scopes: Vec<RelationshipScopeV1>,
    /// Canonically ordered environment allowlist.
    pub environments: Vec<DeploymentEnvironmentV1>,
    /// Maximum admitted data classification.
    pub data_classification_ceiling: DataClassificationV1,
    /// Canonically ordered permitted purposes.
    pub purposes: Vec<PermittedPurposeV1>,
    /// Canonically ordered immutable authority evidence.
    pub authority_evidence: Vec<AuthorityEvidenceV1>,
}

resident_signed_record!(
    /// Expiring, non-transferable authorization for one caller and deployment.
    CapabilitySessionGrantV1, CapabilitySessionGrantWireV1, CapabilitySessionGrantPreimageV1,
    canonical_grant_sha256: CapabilitySessionGrantDigest,
    CAPABILITY_SESSION_GRANT_V1_SCHEMA, "capability session grant",
    {
        /// Stable grant identity.
        session_grant_id: SessionGrantIdV1,
        /// Exact subject and key admitted by this grant.
        subject: CapabilitySessionSubjectV1,
        /// Immutable deployment binding.
        deployment_binding_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-binding bytes.
        deployment_binding_sha256: DeploymentBindingDigest,
        /// Principal trust checkpoint used during issuance.
        checked_principal_trust_checkpoint_ref: ResidentRecordRefV1,
        /// Digest of the checked trust-checkpoint bytes.
        checked_principal_trust_checkpoint_sha256: PrincipalTrustCheckpointDigest,
        /// Exact deployment-acceptance revocation epoch checked at issuance.
        checked_deployment_acceptance_revocation_epoch: DeploymentAcceptanceRevocationEpochV1,
        /// Principal session floor that must still equal the active trusted floor.
        session_floor_epoch: PrincipalSessionFloorEpochV1,
        /// Canonically ordered semantic capability allowlist.
        allowed_capability_ids: Vec<CapabilityIdV1>,
        /// Exact route and destination ceiling.
        allowed_route_constraints: CapabilitySessionRouteConstraintsV1,
        /// Exact scope ceiling.
        scope_ceiling: CapabilitySessionScopeCeilingV1,
        /// Unix issue time.
        issued_at: UnixTimeV1,
        /// Unix expiry time for new admissions.
        expires_at: UnixTimeV1,
        /// One-use issue nonce.
        nonce: CapabilitySessionNonceV1,
        /// Must be `true`; grants cannot be transferred between subjects.
        non_transferable: bool,
        /// Principal whose admitted authority issued the grant.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Exact issuer-key identity.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_capability_session_grant
);

resident_signed_record!(
    /// Append-only revocation of one capability-session grant.
    CapabilitySessionGrantRevocationV1, CapabilitySessionGrantRevocationWireV1,
    CapabilitySessionGrantRevocationPreimageV1,
    canonical_revocation_sha256: CapabilitySessionGrantRevocationDigest,
    CAPABILITY_SESSION_GRANT_REVOCATION_V1_SCHEMA, "capability session grant revocation",
    {
        /// Stable revocation-record identity.
        revocation_id: CapabilityGrantRevocationIdV1,
        /// Immutable revoked grant reference.
        grant_ref: ResidentRecordRefV1,
        /// Canonical digest of the revoked grant.
        grant_sha256: CapabilitySessionGrantDigest,
        /// Stable revoked grant identity.
        grant_id: SessionGrantIdV1,
        /// Positive append-only revocation epoch.
        revocation_epoch: CapabilityGrantRevocationEpochV1,
        /// Previous revocation, absent only at epoch one.
        previous_revocation_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_revocation_ref`].
        previous_revocation_sha256: Option<CapabilitySessionGrantRevocationDigest>,
        /// Epoch paired with [`Self::previous_revocation_ref`].
        previous_revocation_epoch: Option<CapabilityGrantRevocationEpochV1>,
        /// Time from which the revocation blocks new admissions.
        effective_at: UnixTimeV1,
        /// Bounded owner-defined reason code.
        reason_code: ResidentReasonV1,
        /// Principal whose admitted grant authority issued the revocation.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Exact issuer-key identity.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_capability_session_revocation,
    wire_transform = capability_session_revocation_schema_v1
);

resident_signed_record!(
    /// Signed anti-rollback frontier over capability-grant revocation heads.
    CapabilityRevocationCheckpointV1, CapabilityRevocationCheckpointWireV1,
    CapabilityRevocationCheckpointPreimageV1,
    canonical_checkpoint_sha256: CapabilityRevocationCheckpointDigest,
    CAPABILITY_REVOCATION_CHECKPOINT_V1_SCHEMA, "capability revocation checkpoint",
    {
        /// Stable checkpoint-record identity.
        checkpoint_id: CapabilityRevocationCheckpointIdV1,
        /// Positive checkpoint succession epoch.
        checkpoint_epoch: CapabilityRevocationCheckpointEpochV1,
        /// Previous checkpoint, absent only at epoch one.
        previous_checkpoint_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_checkpoint_ref`].
        previous_checkpoint_sha256: Option<CapabilityRevocationCheckpointDigest>,
        /// Principal whose grant-revocation heads are covered.
        authority_principal_id: ResidentPrincipalIdV1,
        /// Canonical root of the complete active revocation-head set.
        authority_heads_root_sha256: ArtifactDigest,
        /// Number of grant heads covered by the root.
        covered_grant_count: CoveredGrantCountV1,
        /// Unix checkpoint construction time.
        generated_at: UnixTimeV1,
        /// Exact checkpoint signer key.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive checkpoint signer epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_capability_revocation_checkpoint,
    wire_transform = capability_revocation_checkpoint_schema_v1
);

resident_signed_record!(
    /// Immutable, generation-bound local route selected by one principal.
    LocalRouteBindingV1, LocalRouteBindingWireV1, LocalRouteBindingPreimageV1,
    canonical_local_route_binding_sha256: LocalRouteBindingDigest,
    LOCAL_ROUTE_BINDING_V1_SCHEMA, "local route binding",
    {
        /// Stable route lineage identity.
        route_id: LocalRouteIdV1,
        /// Positive immutable route generation.
        route_generation: RouteGenerationV1,
        /// Previous route binding, absent only at generation one.
        previous_route_binding_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_route_binding_ref`].
        previous_route_binding_sha256: Option<LocalRouteBindingDigest>,
        /// Principal that owns and signs this route.
        route_owner_principal_id: ResidentPrincipalIdV1,
        /// Exact workspace identity selected for the route.
        workspace_id: ResidentWorkspaceIdV1,
        /// Immutable deployment binding.
        deployment_binding_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-binding bytes.
        deployment_binding_sha256: DeploymentBindingDigest,
        /// Origin principal.
        origin_principal_id: ResidentPrincipalIdV1,
        /// Origin node.
        origin_node_id: ResidentNodeIdV1,
        /// Exact active origin-node delegation.
        origin_node_delegation_ref: ResidentRecordRefV1,
        /// Digest of the origin-node delegation bytes.
        origin_node_delegation_sha256: ArtifactDigest,
        /// Destination principal.
        destination_principal_id: ResidentPrincipalIdV1,
        /// Optional exact destination node.
        destination_node_id: Option<ResidentNodeIdV1>,
        /// Destination-node delegation paired with [`Self::destination_node_id`].
        destination_node_delegation_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::destination_node_id`].
        destination_node_delegation_sha256: Option<ArtifactDigest>,
        /// Principal trust checkpoint covering the selected delegations.
        checked_trust_checkpoint_ref: ResidentRecordRefV1,
        /// Digest of the checked trust-checkpoint bytes.
        checked_trust_checkpoint_sha256: PrincipalTrustCheckpointDigest,
        /// Selected local transport class.
        transport_class: LocalTransportClassV1,
        /// Immutable transport-profile reference.
        transport_profile_ref: ResidentRecordRefV1,
        /// Digest of the exact transport-profile bytes.
        transport_profile_sha256: ArtifactDigest,
        /// Immutable endpoint-binding reference.
        endpoint_binding_ref: ResidentRecordRefV1,
        /// Digest of the exact endpoint-binding bytes.
        endpoint_binding_sha256: ArtifactDigest,
        /// Immutable selected contract set.
        contract_set_ref: ResidentRecordRefV1,
        /// Canonical digest of the selected contract set.
        contract_set_sha256: ContractSetDigest,
        /// Unix issue time.
        issued_at: UnixTimeV1,
        /// Optional Unix expiry time.
        expires_at: Option<UnixTimeV1>,
        /// Principal whose active key signed the route.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Exact issuer-key identity.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive principal-key epoch.
        issuer_key_epoch: PrincipalKeyEpochV1
    },
    validate_local_route_binding,
    wire_transform = local_route_binding_schema_v1
);

/// Lifecycle state of a neutral execution-owner fence.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionOwnerFenceStateV1 {
    /// The selected owner may admit work under this exact fence.
    Active,
    /// This fence is terminal and admits no new work.
    Retired,
}

resident_signed_record!(
    /// Neutral fence selecting one execution owner for one admitted attempt.
    ///
    /// The record references owner authorization and predecessor disposition;
    /// it does not copy a product's task graph or make the owner result true.
    ExecutionOwnerFenceV1, ExecutionOwnerFenceWireV1, ExecutionOwnerFencePreimageV1,
    canonical_execution_fence_sha256: ExecutionOwnerFenceDigest,
    EXECUTION_OWNER_FENCE_V1_SCHEMA, "execution owner fence",
    {
        /// Stable fence lineage identity.
        execution_fence_id: ExecutionOwnerFenceIdV1,
        /// Exact admitted execution attempt.
        execution_attempt_id: ExecutionAttemptIdV1,
        /// Neutral path selected by the canonical workspace graph.
        execution_path_id: ExecutionPathSelectionIdV1,
        /// Native system that owns this execution attempt.
        execution_owner_system_id: ExecutionOwnerSystemIdV1,
        /// Selected participant within the owner system.
        execution_owner_participant_id: ExecutionOwnerParticipantIdV1,
        /// Exact schema of the owner authorization.
        owner_authorization_schema_id: SchemaId,
        /// Digest of the exact authorization schema bytes.
        owner_authorization_schema_sha256: SchemaDigest,
        /// Immutable owner-authorization reference.
        owner_authorization_ref: ResidentRecordRefV1,
        /// Digest of the exact authorization bytes.
        owner_authorization_sha256: ArtifactDigest,
        /// Immutable deployment binding.
        deployment_binding_ref: ResidentRecordRefV1,
        /// Canonical digest of the deployment-binding bytes.
        deployment_binding_sha256: DeploymentBindingDigest,
        /// Principal scope of the attempt.
        principal_scope: ResidentPrincipalIdV1,
        /// Controller scope of the attempt.
        controller_scope: ResidentControllerIdV1,
        /// Optional relationship scope of the attempt.
        relationship_scope: Option<RelationshipScopeV1>,
        /// Positive selected-owner generation.
        owner_generation: ExecutionOwnerGenerationV1,
        /// Positive control epoch within the owner generation.
        control_epoch: ExecutionControlEpochV1,
        /// Previous fence, absent only for the first generation.
        previous_fence_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_fence_ref`].
        previous_fence_sha256: Option<ExecutionOwnerFenceDigest>,
        /// Prior attempt identity paired with [`Self::previous_fence_ref`].
        previous_execution_attempt_id: Option<ExecutionAttemptIdV1>,
        /// Prior owner generation paired with [`Self::previous_fence_ref`].
        previous_owner_generation: Option<ExecutionOwnerGenerationV1>,
        /// Prior control epoch paired with [`Self::previous_fence_ref`].
        previous_control_epoch: Option<ExecutionControlEpochV1>,
        /// Exact schema of the predecessor owner's terminal disposition.
        predecessor_disposition_schema_id: Option<SchemaId>,
        /// Digest of the predecessor-disposition schema bytes.
        predecessor_disposition_schema_sha256: Option<SchemaDigest>,
        /// Immutable predecessor-disposition reference.
        predecessor_disposition_ref: Option<ResidentRecordRefV1>,
        /// Digest of the predecessor-disposition bytes.
        predecessor_disposition_sha256: Option<ArtifactDigest>,
        /// Active or terminal fence state.
        state: ExecutionOwnerFenceStateV1,
        /// Unix issue time.
        issued_at: UnixTimeV1,
        /// Optional Unix expiry time.
        expires_at: Option<UnixTimeV1>,
        /// Exact deployment-runtime authority under which this record was issued.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Principal whose admitted Pale authority issued the fence.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Exact issuer-key identity.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_execution_owner_fence,
    wire_transform = execution_owner_fence_schema_v1
);

/// Owner-resolved principal signer for a local route.
#[derive(Clone, Copy)]
pub struct LocalRouteAdmissionContextV1<'a> {
    /// Digest of the exact selected route schema.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Accepted route-owner principal.
    pub route_owner_principal_id: &'a ResidentPrincipalIdV1,
    /// Active accepted principal-key identity.
    pub issuer_key_id: &'a ResidentKeyIdV1,
    /// Active accepted principal-key epoch.
    pub issuer_key_epoch: PrincipalKeyEpochV1,
}

macro_rules! impl_signed_admission {
    ($record:ident, $wire:ident, $preimage:ident, $digest:ident, $domain:ident, $family:literal) => {
        impl $record {
            /// Admit an exact wire against its resolved schema and signing authority.
            ///
            /// # Errors
            /// Returns [`InvalidResidentRecordV1`] for structural, digest,
            /// authority-coordinate, or signature admission failure.
            pub fn admit_contextual_wire<D, F>(
                wire: $wire,
                context: &SignedResidentAdmissionContextV1<'_>,
                admit_dependencies: D,
                verify_signature: F,
            ) -> Result<Self, InvalidResidentRecordV1>
            where
                D: FnOnce(&$preimage) -> Result<(), InvalidResidentRecordV1>,
                F: FnOnce(
                    &[u8],
                    &$preimage,
                    &$digest,
                    &CanonicalSignatureV1,
                ) -> Result<(), InvalidResidentRecordV1>,
            {
                Self::admit_wire(wire, |preimage, digest, signature| {
                    validate_signed_coordinates(
                        $family,
                        &preimage.schema_sha256,
                        context.expected_schema_sha256,
                        &preimage.issuer_principal_id,
                        &preimage.issuer_key_id,
                        preimage.issuer_key_epoch,
                        context.signing_authority,
                    )?;
                    admit_dependencies(preimage)?;
                    verify_signature($domain, preimage, digest, signature)
                })
            }
        }
    };
}

impl_signed_admission!(
    CapabilitySessionGrantV1,
    CapabilitySessionGrantWireV1,
    CapabilitySessionGrantPreimageV1,
    CapabilitySessionGrantDigest,
    CAPABILITY_SESSION_GRANT_V1_SIGNATURE_DOMAIN,
    "capability session grant"
);

impl LocalRouteBindingV1 {
    /// Admit an exact local route against its resolved principal signer.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for structural, digest, route-owner,
    /// principal-key, or signature admission failure.
    pub fn admit_initial_wire<D, F>(
        wire: LocalRouteBindingWireV1,
        context: &LocalRouteAdmissionContextV1<'_>,
        admit_dependencies: D,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        D: FnOnce(&LocalRouteBindingPreimageV1) -> Result<(), InvalidResidentRecordV1>,
        F: FnOnce(
            &[u8],
            &LocalRouteBindingPreimageV1,
            &LocalRouteBindingDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            if preimage.route_generation.get() != 1 {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "local route binding",
                    invariant: "initial route generation",
                });
            }
            if preimage.schema_sha256 != *context.expected_schema_sha256
                || preimage.route_owner_principal_id != *context.route_owner_principal_id
                || preimage.issuer_principal_id != *context.route_owner_principal_id
                || preimage.issuer_key_id != *context.issuer_key_id
                || preimage.issuer_key_epoch != context.issuer_key_epoch
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "local route binding",
                    invariant: "exact schema and active route-owner principal key",
                });
            }
            admit_dependencies(preimage)?;
            verify_signature(
                LOCAL_ROUTE_BINDING_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }

    /// Admit the exact successor of one active route generation.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] unless the lineage, immutable owner,
    /// workspace and deployment, exact predecessor, generation, dependencies,
    /// signer, and signature all agree.
    pub fn admit_successor_wire<D, F>(
        wire: LocalRouteBindingWireV1,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &Self,
        context: &LocalRouteAdmissionContextV1<'_>,
        admit_dependencies: D,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        D: FnOnce(&LocalRouteBindingPreimageV1) -> Result<(), InvalidResidentRecordV1>,
        F: FnOnce(
            &[u8],
            &LocalRouteBindingPreimageV1,
            &LocalRouteBindingDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            if preimage.schema_sha256 != *context.expected_schema_sha256
                || preimage.route_owner_principal_id != *context.route_owner_principal_id
                || preimage.issuer_principal_id != *context.route_owner_principal_id
                || preimage.issuer_key_id != *context.issuer_key_id
                || preimage.issuer_key_epoch != context.issuer_key_epoch
                || preimage.route_id != *predecessor.route_id()
                || preimage.workspace_id != *predecessor.workspace_id()
                || preimage.deployment_binding_ref != *predecessor.deployment_binding_ref()
                || preimage.deployment_binding_sha256 != *predecessor.deployment_binding_sha256()
                || preimage.route_owner_principal_id != *predecessor.route_owner_principal_id()
                || preimage.previous_route_binding_ref.as_ref() != Some(predecessor_ref)
                || preimage.previous_route_binding_sha256.as_ref()
                    != Some(predecessor.canonical_local_route_binding_sha256())
                || predecessor.route_generation().checked_successor()
                    != Some(preimage.route_generation)
                || preimage.issued_at < *predecessor.issued_at()
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "local route binding",
                    invariant: "exact route successor and active route-owner key",
                });
            }
            admit_dependencies(preimage)?;
            verify_signature(
                LOCAL_ROUTE_BINDING_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

impl CapabilityRevocationCheckpointV1 {
    /// Admit a checkpoint against its exact schema and signing authority.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for structural, digest,
    /// authority-coordinate, or signature admission failure.
    pub fn admit_initial_wire<D, F>(
        wire: CapabilityRevocationCheckpointWireV1,
        context: &SignedResidentAdmissionContextV1<'_>,
        admit_dependencies: D,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        D: FnOnce(&CapabilityRevocationCheckpointPreimageV1) -> Result<(), InvalidResidentRecordV1>,
        F: FnOnce(
            &[u8],
            &CapabilityRevocationCheckpointPreimageV1,
            &CapabilityRevocationCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            if preimage.checkpoint_epoch.get() != 1 {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "capability revocation checkpoint",
                    invariant: "initial checkpoint epoch",
                });
            }
            validate_signed_coordinates(
                "capability revocation checkpoint",
                &preimage.schema_sha256,
                context.expected_schema_sha256,
                &preimage.authority_principal_id,
                &preimage.issuer_key_id,
                preimage.issuer_key_epoch,
                context.signing_authority,
            )?;
            admit_dependencies(preimage)?;
            verify_signature(
                CAPABILITY_REVOCATION_CHECKPOINT_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }

    /// Admit the exact successor of one capability-revocation checkpoint.
    ///
    /// Writing `e` for the checkpoint epoch and `n` for the covered-grant
    /// count, the admitted transition is monotone in both coordinates:
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/capability-revocation-checkpoint-successor.display.mathml"
    ))]
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] unless the authority, exact
    /// predecessor, epoch, non-regressing covered count, dependencies, signer,
    /// and signature all agree.
    pub fn admit_successor_wire<D, F>(
        wire: CapabilityRevocationCheckpointWireV1,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &Self,
        context: &SignedResidentAdmissionContextV1<'_>,
        admit_dependencies: D,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        D: FnOnce(&CapabilityRevocationCheckpointPreimageV1) -> Result<(), InvalidResidentRecordV1>,
        F: FnOnce(
            &[u8],
            &CapabilityRevocationCheckpointPreimageV1,
            &CapabilityRevocationCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_signed_coordinates(
                "capability revocation checkpoint",
                &preimage.schema_sha256,
                context.expected_schema_sha256,
                &preimage.authority_principal_id,
                &preimage.issuer_key_id,
                preimage.issuer_key_epoch,
                context.signing_authority,
            )?;
            if preimage.authority_principal_id != *predecessor.authority_principal_id()
                || preimage.previous_checkpoint_ref.as_ref() != Some(predecessor_ref)
                || preimage.previous_checkpoint_sha256.as_ref()
                    != Some(predecessor.canonical_checkpoint_sha256())
                || predecessor.checkpoint_epoch().checked_successor()
                    != Some(preimage.checkpoint_epoch)
                || preimage.covered_grant_count < *predecessor.covered_grant_count()
                || preimage.generated_at < *predecessor.generated_at()
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "capability revocation checkpoint",
                    invariant: "exact non-regressing checkpoint successor",
                });
            }
            admit_dependencies(preimage)?;
            verify_signature(
                CAPABILITY_REVOCATION_CHECKPOINT_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

impl CapabilitySessionGrantRevocationV1 {
    /// Admit the first revocation of one exact session grant.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] unless epoch one, grant resolution,
    /// signer coordinates, canonical digest, and signature all agree.
    pub fn admit_initial_wire<D, F>(
        wire: CapabilitySessionGrantRevocationWireV1,
        grant_ref: &ResidentRecordRefV1,
        grant: &CapabilitySessionGrantV1,
        context: &SignedResidentAdmissionContextV1<'_>,
        admit_dependencies: D,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        D: FnOnce(
            &CapabilitySessionGrantRevocationPreimageV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        F: FnOnce(
            &[u8],
            &CapabilitySessionGrantRevocationPreimageV1,
            &CapabilitySessionGrantRevocationDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_session_revocation_authority(preimage, context)?;
            if preimage.revocation_epoch.get() != 1
                || preimage.grant_ref != *grant_ref
                || preimage.grant_sha256 != *grant.canonical_grant_sha256()
                || preimage.grant_id != *grant.session_grant_id()
                || preimage.issuer_principal_id != *grant.issuer_principal_id()
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "capability session grant revocation",
                    invariant: "exact grant and initial revocation epoch",
                });
            }
            admit_dependencies(preimage)?;
            verify_signature(
                CAPABILITY_SESSION_GRANT_REVOCATION_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }

    /// Admit the exact successor of one grant-revocation head.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] unless the grant, predecessor,
    /// epoch, chronology, dependencies, signer, digest, and signature agree.
    pub fn admit_successor_wire<D, F>(
        wire: CapabilitySessionGrantRevocationWireV1,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &Self,
        context: &SignedResidentAdmissionContextV1<'_>,
        admit_dependencies: D,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        D: FnOnce(
            &CapabilitySessionGrantRevocationPreimageV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        F: FnOnce(
            &[u8],
            &CapabilitySessionGrantRevocationPreimageV1,
            &CapabilitySessionGrantRevocationDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_session_revocation_authority(preimage, context)?;
            if preimage.grant_ref != *predecessor.grant_ref()
                || preimage.grant_sha256 != *predecessor.grant_sha256()
                || preimage.grant_id != *predecessor.grant_id()
                || preimage.issuer_principal_id != *predecessor.issuer_principal_id()
                || preimage.previous_revocation_ref.as_ref() != Some(predecessor_ref)
                || preimage.previous_revocation_sha256.as_ref()
                    != Some(predecessor.canonical_revocation_sha256())
                || preimage.previous_revocation_epoch.as_ref()
                    != Some(predecessor.revocation_epoch())
                || predecessor.revocation_epoch().checked_successor()
                    != Some(preimage.revocation_epoch)
                || preimage.effective_at < *predecessor.effective_at()
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "capability session grant revocation",
                    invariant: "exact grant revocation successor",
                });
            }
            admit_dependencies(preimage)?;
            verify_signature(
                CAPABILITY_SESSION_GRANT_REVOCATION_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

impl ExecutionOwnerFenceV1 {
    /// Admit the first active fence under exact neutral owner authorization.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] unless initial counters, owner
    /// authorization, dependencies, signer, digest, and signature agree.
    pub fn admit_initial_wire<D, F>(
        wire: ExecutionOwnerFenceWireV1,
        context: &SignedResidentAdmissionContextV1<'_>,
        admit_owner_records: D,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        D: FnOnce(&ExecutionOwnerFencePreimageV1) -> Result<(), InvalidResidentRecordV1>,
        F: FnOnce(
            &[u8],
            &ExecutionOwnerFencePreimageV1,
            &ExecutionOwnerFenceDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_execution_fence_authority(preimage, context)?;
            if preimage.owner_generation.get() != 1
                || preimage.control_epoch.get() != 1
                || preimage.previous_fence_ref.is_some()
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "execution owner fence",
                    invariant: "initial active owner counters",
                });
            }
            admit_owner_records(preimage)?;
            verify_signature(
                EXECUTION_OWNER_FENCE_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }

    /// Admit one exact owner/control successor or terminal retirement.
    ///
    /// Same-owner succession advances the control epoch exactly once. An owner
    /// change advances the owner generation exactly once and restarts control at
    /// one. Retirement is a same-owner control successor. Every branch binds the
    /// exact predecessor and generic owner-disposition evidence. In the law
    /// below, `o`, `g`, and `c` denote owner identity, owner generation, and
    /// control epoch; primed symbols denote their successor values.
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/execution-owner-fence-successor.display.mathml"
    ))]
    ///
    /// # Errors
    /// `admit_authority_rotation` must prove that changed runtime-authority
    /// coordinates continue the predecessor's admitted deployment lineage.
    ///
    /// Returns [`InvalidResidentRecordV1`] when any lineage, counter, scope,
    /// owner evidence, runtime-authority, signer, digest, or signature
    /// invariant fails.
    pub fn admit_successor_wire<D, A, F>(
        wire: ExecutionOwnerFenceWireV1,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &Self,
        context: &SignedResidentAdmissionContextV1<'_>,
        admit_owner_records: D,
        admit_authority_rotation: A,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        D: FnOnce(&ExecutionOwnerFencePreimageV1) -> Result<(), InvalidResidentRecordV1>,
        A: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        F: FnOnce(
            &[u8],
            &ExecutionOwnerFencePreimageV1,
            &ExecutionOwnerFenceDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_execution_fence_authority(preimage, context)?;
            let owner_unchanged = preimage.execution_owner_system_id
                == *predecessor.execution_owner_system_id()
                && preimage.execution_owner_participant_id
                    == *predecessor.execution_owner_participant_id();
            let counters_valid = if owner_unchanged {
                preimage.owner_generation == *predecessor.owner_generation()
                    && predecessor.control_epoch().checked_successor()
                        == Some(preimage.control_epoch)
            } else {
                predecessor.owner_generation().checked_successor()
                    == Some(preimage.owner_generation)
                    && preimage.control_epoch.get() == 1
            };
            if *predecessor.state() != ExecutionOwnerFenceStateV1::Active
                || preimage.execution_fence_id != *predecessor.execution_fence_id()
                || preimage.execution_path_id != *predecessor.execution_path_id()
                || preimage.deployment_binding_ref != *predecessor.deployment_binding_ref()
                || preimage.deployment_binding_sha256 != *predecessor.deployment_binding_sha256()
                || preimage.principal_scope != *predecessor.principal_scope()
                || preimage.controller_scope != *predecessor.controller_scope()
                || preimage.relationship_scope.as_ref() != predecessor.relationship_scope().as_ref()
                || preimage.execution_attempt_id == *predecessor.execution_attempt_id()
                || preimage.previous_fence_ref.as_ref() != Some(predecessor_ref)
                || preimage.previous_fence_sha256.as_ref()
                    != Some(predecessor.canonical_execution_fence_sha256())
                || preimage.previous_execution_attempt_id.as_ref()
                    != Some(predecessor.execution_attempt_id())
                || preimage.previous_owner_generation.as_ref()
                    != Some(predecessor.owner_generation())
                || preimage.previous_control_epoch.as_ref() != Some(predecessor.control_epoch())
                || !counters_valid
                || (preimage.state == ExecutionOwnerFenceStateV1::Retired && !owner_unchanged)
                || preimage.issued_at < *predecessor.issued_at()
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "execution owner fence",
                    invariant: "exact neutral owner-fence successor",
                });
            }
            admit_runtime_authority_rotation(
                "execution owner fence",
                predecessor.deployment_runtime_authority_ref(),
                predecessor.deployment_runtime_authority_sha256(),
                &preimage.deployment_runtime_authority_ref,
                &preimage.deployment_runtime_authority_sha256,
                admit_authority_rotation,
            )?;
            admit_owner_records(preimage)?;
            verify_signature(
                EXECUTION_OWNER_FENCE_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

fn validate_session_revocation_authority(
    preimage: &CapabilitySessionGrantRevocationPreimageV1,
    context: &SignedResidentAdmissionContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    validate_signed_coordinates(
        "capability session grant revocation",
        &preimage.schema_sha256,
        context.expected_schema_sha256,
        &preimage.issuer_principal_id,
        &preimage.issuer_key_id,
        preimage.issuer_key_epoch,
        context.signing_authority,
    )
}

fn validate_execution_fence_authority(
    preimage: &ExecutionOwnerFencePreimageV1,
    context: &SignedResidentAdmissionContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    validate_signed_coordinates(
        "execution owner fence",
        &preimage.schema_sha256,
        context.expected_schema_sha256,
        &preimage.issuer_principal_id,
        &preimage.issuer_key_id,
        preimage.issuer_key_epoch,
        context.signing_authority,
    )?;
    validate_integrated_runtime_authority_coordinates(
        "execution owner fence",
        &preimage.deployment_runtime_authority_ref,
        &preimage.deployment_runtime_authority_sha256,
        context.signing_authority,
    )
}

fn validate_capability_session_grant(
    value: &CapabilitySessionGrantPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    strictly_sorted("session capabilities", &value.allowed_capability_ids)?;
    let route = &value.allowed_route_constraints;
    strictly_sorted(
        "session destination principals",
        &route.destination_principal_ids,
    )?;
    strictly_sorted("session destination nodes", &route.destination_node_ids)?;
    strictly_sorted("session routes", &route.routes)?;
    strictly_sorted("session transport classes", &route.transport_classes)?;
    strictly_sorted("session contract sets", &route.contract_sets)?;
    let scope = &value.scope_ceiling;
    strictly_sorted("session principal scopes", &scope.principal_scopes)?;
    strictly_sorted("session controller scopes", &scope.controller_scopes)?;
    strictly_sorted("session relationship scopes", &scope.relationship_scopes)?;
    strictly_sorted("session environments", &scope.environments)?;
    strictly_sorted("session purposes", &scope.purposes)?;
    strictly_sorted("session authority evidence", &scope.authority_evidence)?;
    if !value.non_transferable || value.issued_at >= value.expires_at {
        Err(InvalidResidentRecordV1::InvalidCoordinate {
            coordinate: "capability session grant lifetime",
        })
    } else {
        Ok(())
    }
}

fn validate_capability_session_revocation(
    value: &CapabilitySessionGrantRevocationPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    let predecessor_present = value.previous_revocation_ref.is_some()
        && value.previous_revocation_sha256.is_some()
        && value.previous_revocation_epoch.is_some();
    let predecessor_absent = value.previous_revocation_ref.is_none()
        && value.previous_revocation_sha256.is_none()
        && value.previous_revocation_epoch.is_none();
    if (!predecessor_present && !predecessor_absent)
        || (value.revocation_epoch.get() == 1) != predecessor_absent
        || value
            .previous_revocation_epoch
            .is_some_and(|epoch| epoch.checked_successor() != Some(value.revocation_epoch))
    {
        Err(InvalidResidentRecordV1::InvalidCoordinate {
            coordinate: "capability grant revocation successor",
        })
    } else {
        Ok(())
    }
}

fn validate_capability_revocation_checkpoint(
    value: &CapabilityRevocationCheckpointPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    let predecessor_absent =
        value.previous_checkpoint_ref.is_none() && value.previous_checkpoint_sha256.is_none();
    let predecessor_present =
        value.previous_checkpoint_ref.is_some() && value.previous_checkpoint_sha256.is_some();
    if (!predecessor_absent && !predecessor_present)
        || (value.checkpoint_epoch.get() == 1) != predecessor_absent
    {
        Err(InvalidResidentRecordV1::InvalidCoordinate {
            coordinate: "capability revocation checkpoint successor",
        })
    } else {
        Ok(())
    }
}

fn validate_local_route_binding(
    value: &LocalRouteBindingPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    let predecessor_absent =
        value.previous_route_binding_ref.is_none() && value.previous_route_binding_sha256.is_none();
    let predecessor_present =
        value.previous_route_binding_ref.is_some() && value.previous_route_binding_sha256.is_some();
    let destination_absent = value.destination_node_id.is_none()
        && value.destination_node_delegation_ref.is_none()
        && value.destination_node_delegation_sha256.is_none();
    let destination_present = value.destination_node_id.is_some()
        && value.destination_node_delegation_ref.is_some()
        && value.destination_node_delegation_sha256.is_some();
    let same_principal_remote = value.transport_class == LocalTransportClassV1::SamePrincipalRemote;
    if (!predecessor_absent && !predecessor_present)
        || (value.route_generation.get() == 1) != predecessor_absent
        || (!destination_absent && !destination_present)
        || (same_principal_remote
            && (!destination_present
                || value.origin_principal_id != value.route_owner_principal_id
                || value.destination_principal_id != value.route_owner_principal_id))
        || value
            .expires_at
            .is_some_and(|expires| expires <= value.issued_at)
    {
        Err(InvalidResidentRecordV1::InvalidCoordinate {
            coordinate: "local route binding",
        })
    } else {
        Ok(())
    }
}

fn validate_execution_owner_fence(
    value: &ExecutionOwnerFencePreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    let predecessor_fields = [
        value.previous_fence_ref.is_some(),
        value.previous_fence_sha256.is_some(),
        value.previous_execution_attempt_id.is_some(),
        value.previous_owner_generation.is_some(),
        value.previous_control_epoch.is_some(),
        value.predecessor_disposition_schema_id.is_some(),
        value.predecessor_disposition_schema_sha256.is_some(),
        value.predecessor_disposition_ref.is_some(),
        value.predecessor_disposition_sha256.is_some(),
    ];
    let all_absent = predecessor_fields.iter().all(|present| !present);
    let all_present = predecessor_fields.iter().all(|present| *present);
    if (!all_absent && !all_present)
        || (all_absent
            && (value.owner_generation.get() != 1
                || value.control_epoch.get() != 1
                || value.state != ExecutionOwnerFenceStateV1::Active))
        || value
            .expires_at
            .is_some_and(|expires| expires <= value.issued_at)
    {
        Err(InvalidResidentRecordV1::InvalidCoordinate {
            coordinate: "execution owner fence",
        })
    } else {
        Ok(())
    }
}

fn capability_session_revocation_schema_v1(schema: &mut schemars::Schema) {
    require_all_or_none_properties_v1(
        schema,
        &[
            "previous_revocation_ref",
            "previous_revocation_sha256",
            "previous_revocation_epoch",
        ],
    );
}

fn capability_revocation_checkpoint_schema_v1(schema: &mut schemars::Schema) {
    require_all_or_none_properties_v1(
        schema,
        &["previous_checkpoint_ref", "previous_checkpoint_sha256"],
    );
}

fn local_route_binding_schema_v1(schema: &mut schemars::Schema) {
    require_all_or_none_properties_v1(
        schema,
        &[
            "previous_route_binding_ref",
            "previous_route_binding_sha256",
        ],
    );
    require_all_or_none_properties_v1(
        schema,
        &[
            "destination_node_id",
            "destination_node_delegation_ref",
            "destination_node_delegation_sha256",
        ],
    );
}

fn execution_owner_fence_schema_v1(schema: &mut schemars::Schema) {
    require_all_or_none_properties_v1(
        schema,
        &[
            "previous_fence_ref",
            "previous_fence_sha256",
            "previous_execution_attempt_id",
            "previous_owner_generation",
            "previous_control_epoch",
            "predecessor_disposition_schema_id",
            "predecessor_disposition_schema_sha256",
            "predecessor_disposition_ref",
            "predecessor_disposition_sha256",
        ],
    );
}
