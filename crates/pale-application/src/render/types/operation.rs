//! Model-facing artifacts and mechanical operation support.

use std::collections::BTreeMap;

use pale_core::records::{
    OperationCapabilityContractV1, OperationSupport, PackageResourceRefV1, SealedArtifactRef,
};

/// Exact prompt artifacts emitted for one operation.
#[derive(Debug, Clone)]
pub(in crate::render) struct OperationSurfaceRefs {
    /// Sealed prompt artifact containing the operation's procedural doctrine.
    pub(in crate::render) skill: SealedArtifactRef,
    /// Optional host-native invocation policy paired with the prompt artifact.
    pub(in crate::render) invocation_policy: Option<SealedArtifactRef>,
    /// Mechanical support projection consumed by rendering and admission.
    pub(in crate::render) support: HostOperationProjection,
}

/// One host's mechanical view of an operation, shared by prompt rendering and
/// resident-operation admission so the two surfaces cannot disagree.
#[derive(Debug, Clone)]
pub(in crate::render) struct HostOperationProjection {
    /// Capability contracts reachable through this host projection.
    pub(in crate::render) capabilities: Vec<OperationCapabilityContractV1>,
    /// Effective support posture exposed to the native harness.
    pub(in crate::render) support: OperationSupport,
    /// Fail-closed reasons that prevent invocation.
    pub(in crate::render) blocked_reasons: Vec<String>,
    /// Reasons the operation remains usable with reduced guarantees.
    pub(in crate::render) degraded_reasons: Vec<String>,
    /// Unbound optional capabilities and their declared fallbacks.
    pub(in crate::render) optional_reasons: Vec<String>,
    /// Whether this projection embeds a machine-conditioned native tool grant.
    /// Such prompt and contract surfaces are reproducible for removal, but are
    /// not portable configuration and therefore cannot be committed.
    pub(in crate::render) machine_bound: bool,
}

/// Exact model-facing artifacts emitted once for one native harness.
#[derive(Debug, Clone, Default)]
pub(in crate::render) struct HarnessSurfaceRefs {
    /// Owner-qualified operation to the exact artifacts emitted for it.
    pub(in crate::render) operations: BTreeMap<PackageResourceRefV1, OperationSurfaceRefs>,
    /// Owner-qualified role to the sealed prompt artifact emitted for it.
    pub(in crate::render) roles: BTreeMap<PackageResourceRefV1, SealedArtifactRef>,
}
