//! Typed logical heads shared by command expectations and receipt transitions.

use serde::{Deserialize, Serialize};

use super::*;

/// Logical head selection, retaining the selected owner's exact scope.
///
/// Record coordinates and changing generations are expected contents, not
/// slot identity. Naming a slot neither implements its transaction nor grants
/// authority to mutate it. Scope/body joins belong to that head's owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LocalAdminHeadSelectorV1 {
    /// Current principal record.
    PrincipalRecord {
        /// Principal that owns this logical head.
        principal_id: ResidentPrincipalIdV1,
    },
    /// Current recovery policy, including replacement by a different policy ID.
    PrincipalRecoveryPolicy {
        /// Principal that owns this logical head.
        principal_id: ResidentPrincipalIdV1,
    },
    /// Current administrator authority, including recovery replacement.
    LocalAdminAuthority {
        /// Principal that owns this logical head.
        principal_id: ResidentPrincipalIdV1,
    },
    /// Current producer-allocation authority.
    ProducerIncarnationAuthority {
        /// Principal that owns this logical head.
        principal_id: ResidentPrincipalIdV1,
    },
    /// Current principal trust frontier in its physical trust store.
    PrincipalTrustCheckpoint {
        /// Principal that owns this logical head.
        principal_id: ResidentPrincipalIdV1,
        /// Physical trust store selected by its owner.
        trust_store_id: ControlStoreIdV1,
        /// Exact physical trust-store incarnation.
        trust_store_incarnation_id: ControlIncarnationIdV1,
    },
    /// Principal-wide allocation slot for one producer.
    ProducerIncarnation {
        /// Origin principal of the allocated producer.
        origin_principal_id: ResidentPrincipalIdV1,
        /// Producer whose allocation head is selected.
        producer_id: ProducerIdV1,
    },
    /// One checkpoint-signing lineage and its fixed delegated scope.
    CheckpointSigningAuthority {
        /// Principal that owns this logical head.
        principal_id: ResidentPrincipalIdV1,
        /// Stable checkpoint signer lineage.
        checkpoint_signing_authority_id: CheckpointSigningAuthorityIdV1,
        /// Logical control store.
        control_store_id: ControlStoreIdV1,
        /// Exact control-store incarnation.
        control_incarnation_id: ControlIncarnationIdV1,
        /// Checkpoint signer's full fixed delegated scope.
        scope: CheckpointSigningScopeV1,
    },
    /// One online runtime-signing lineage in an exact deployment store.
    DeploymentRuntimeAuthority {
        /// Principal that owns this logical head.
        principal_id: ResidentPrincipalIdV1,
        /// Stable runtime signer lineage.
        runtime_authority_id: DeploymentRuntimeAuthorityIdV1,
        /// Immutable binding fixing this head's deployment scope.
        deployment_binding_ref: ResidentRecordRefV1,
        /// Canonical digest paired with that binding.
        deployment_binding_sha256: DeploymentBindingDigest,
        /// Logical control store.
        control_store_id: ControlStoreIdV1,
        /// Exact control-store incarnation.
        control_incarnation_id: ControlIncarnationIdV1,
    },
    /// Current checkpoint for a store-local hard-conflict domain.
    ControlCheckpoint {
        /// Logical control store.
        control_store_id: ControlStoreIdV1,
        /// Exact control-store incarnation.
        control_incarnation_id: ControlIncarnationIdV1,
        /// Hard-conflict domain within the selected store.
        checkpoint_domain_id: ControlCheckpointDomainIdV1,
    },
    /// One route-owner-local route lineage.
    LocalRoute {
        /// Principal that owns and signs this route lineage.
        route_owner_principal_id: ResidentPrincipalIdV1,
        /// Stable route identity within its owner.
        route_id: LocalRouteIdV1,
        /// Workspace fixed by this route lineage.
        workspace_id: ResidentWorkspaceIdV1,
        /// Immutable binding fixing this head's deployment scope.
        deployment_binding_ref: ResidentRecordRefV1,
        /// Canonical digest paired with that binding.
        deployment_binding_sha256: DeploymentBindingDigest,
    },
    /// One fence lineage; selected executor and version remain head contents.
    ExecutionFence {
        /// Stable execution-fence lineage.
        execution_fence_id: ExecutionOwnerFenceIdV1,
        /// Execution path fixed across attempts in this fence lineage.
        execution_path_id: ExecutionPathSelectionIdV1,
        /// Immutable binding fixing this head's deployment scope.
        deployment_binding_ref: ResidentRecordRefV1,
        /// Canonical digest paired with that binding.
        deployment_binding_sha256: DeploymentBindingDigest,
        /// Principal in the fence's fixed scope.
        principal_scope: ResidentPrincipalIdV1,
        /// Controller in the fence's fixed scope.
        controller_scope: ResidentControllerIdV1,
        /// Optional relationship in the fence's fixed scope.
        relationship_scope: Option<RelationshipScopeV1>,
    },
    /// Revocation head for one immutable deployment acceptance.
    DeploymentAcceptanceRevocation {
        /// Immutable acceptance whose revocation head is selected.
        deployment_acceptance_ref: ResidentRecordRefV1,
        /// Canonical digest paired with the acceptance.
        deployment_acceptance_sha256: DeploymentAcceptanceDigest,
    },
    /// Revocation head for one immutable capability grant.
    CapabilityGrantRevocation {
        /// Immutable grant whose revocation head is selected.
        grant_ref: ResidentRecordRefV1,
        /// Canonical digest paired with the grant.
        grant_sha256: CapabilitySessionGrantDigest,
    },
    /// Complete grant-revocation frontier of one authority principal.
    CapabilityRevocationCheckpoint {
        /// Principal that owns the revocation checkpoint.
        authority_principal_id: ResidentPrincipalIdV1,
    },
    /// One owner signer's immutable component/deployment/contract scope.
    OwnerRecordSigningAuthority {
        /// Principal that owns this logical head.
        principal_id: ResidentPrincipalIdV1,
        /// Stable owner-record signer lineage.
        owner_record_signing_authority_id: OwnerRecordSigningAuthorityIdV1,
        /// Component in the signer's fixed scope.
        owner_component_id: OwnerComponentIdV1,
        /// Immutable binding fixing this head's deployment scope.
        deployment_binding_ref: ResidentRecordRefV1,
        /// Canonical digest paired with that binding.
        deployment_binding_sha256: DeploymentBindingDigest,
        /// Immutable owner-contract manifest.
        owner_contract_manifest_ref: ResidentRecordRefV1,
        /// Canonical digest paired with the manifest.
        owner_contract_manifest_sha256: ArtifactDigest,
    },
    /// Active node-delegation set entry, keyed by delegation rather than node.
    NodeDelegation {
        /// Principal that owns this logical head.
        principal_id: ResidentPrincipalIdV1,
        /// Stable node-delegation identity, not the node ID.
        delegation_id: NodeDelegationIdV1,
        /// Node named by the selected delegation or revocation.
        node_id: ResidentNodeIdV1,
    },
    /// Node-keyed revocation frontier, distinct from the delegation set entry.
    NodeRevocation {
        /// Principal that owns this logical head.
        principal_id: ResidentPrincipalIdV1,
        /// Node named by the selected delegation or revocation.
        node_id: ResidentNodeIdV1,
    },
    /// One federation route lineage owned by its signing principal.
    FederationRoute {
        /// Principal that owns and signs this route lineage.
        route_owner_principal_id: ResidentPrincipalIdV1,
        /// Stable route identity within its owner.
        route_id: ResidentRouteIdV1,
    },
}

impl LocalAdminHeadSelectorV1 {
    /// Compare slot identity without allowing altered scope to disguise a duplicate.
    /// Exact scope agreement is separately checked against the owning record.
    #[must_use]
    pub(super) fn same_slot(&self, other: &Self) -> bool {
        use LocalAdminHeadSelectorV1 as H;
        match (self, other) {
            (
                H::PrincipalTrustCheckpoint {
                    principal_id: a, ..
                },
                H::PrincipalTrustCheckpoint {
                    principal_id: b, ..
                },
            ) => a == b,
            (
                H::CheckpointSigningAuthority {
                    principal_id: a,
                    checkpoint_signing_authority_id: x,
                    ..
                },
                H::CheckpointSigningAuthority {
                    principal_id: b,
                    checkpoint_signing_authority_id: y,
                    ..
                },
            ) => a == b && x == y,
            (
                H::DeploymentRuntimeAuthority {
                    principal_id: a,
                    runtime_authority_id: x,
                    ..
                },
                H::DeploymentRuntimeAuthority {
                    principal_id: b,
                    runtime_authority_id: y,
                    ..
                },
            ) => a == b && x == y,
            (
                H::LocalRoute {
                    route_owner_principal_id: a,
                    route_id: x,
                    ..
                },
                H::LocalRoute {
                    route_owner_principal_id: b,
                    route_id: y,
                    ..
                },
            ) => a == b && x == y,
            (
                H::ExecutionFence {
                    principal_scope: a,
                    execution_fence_id: x,
                    ..
                },
                H::ExecutionFence {
                    principal_scope: b,
                    execution_fence_id: y,
                    ..
                },
            ) => a == b && x == y,
            (
                H::DeploymentAcceptanceRevocation {
                    deployment_acceptance_ref: a,
                    ..
                },
                H::DeploymentAcceptanceRevocation {
                    deployment_acceptance_ref: b,
                    ..
                },
            ) => a == b,
            (
                H::CapabilityGrantRevocation { grant_ref: a, .. },
                H::CapabilityGrantRevocation { grant_ref: b, .. },
            ) => a == b,
            (
                H::OwnerRecordSigningAuthority {
                    principal_id: a,
                    owner_record_signing_authority_id: x,
                    ..
                },
                H::OwnerRecordSigningAuthority {
                    principal_id: b,
                    owner_record_signing_authority_id: y,
                    ..
                },
            ) => a == b && x == y,
            (
                H::NodeDelegation {
                    principal_id: a,
                    delegation_id: x,
                    ..
                },
                H::NodeDelegation {
                    principal_id: b,
                    delegation_id: y,
                    ..
                },
            ) => a == b && x == y,
            _ => self == other,
        }
    }
}

/// Exact expected contents of one logical head.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalAdminExpectedHeadV1 {
    /// Closed owner slot and full fixed scope.
    pub head_kind: LocalAdminHeadSelectorV1,
    /// Exact predecessor; absent only when the selected owner expects no head.
    pub expected_head_ref: Option<ResidentRecordRefV1>,
    /// Canonical digest paired with the predecessor reference.
    pub expected_head_sha256: Option<ArtifactDigest>,
    /// Optional one-dimensional epoch whose meaning the selected owner checks.
    /// This cannot replace a fence's two version coordinates or a domain sequence.
    pub expected_head_epoch: Option<LocalAdminHeadEpochV1>,
}
