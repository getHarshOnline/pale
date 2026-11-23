//! Execution-path-specific records consumed by the one-shot hook runtime.
//!
//! These wire values carry digest and binding claims. Before use, the owning
//! loader verifies their schema coordinates, referenced artifact bytes, and
//! path relationships; declarations and policy lists are not filesystem
//! or native-effect authority by themselves.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::super::{
    ArtifactDigest, CapabilityIdV1, CompilationInputDigest, DurableEvidenceBinding,
    ExecutionPathWitnessV1, HostCommandConstraint, HostToolEffectBinding, InstanceDigest,
    LifecycleBehavior, LifecycleBehaviorIdV1, LogicalRepositoryIdV1, MaterializationLineageBinding,
    ModeAxis, NativeHarnessBinding, PackageResourceRefV1, ProfileDigest, RunPolicyEntry, SchemaId,
    SchemaManifestDigest, SchemaSetDigest, SealedArtifactRef, SessionStateBinding,
};

/// One repository-declared command whose observed successful completion may
/// satisfy verification evidence after a structured write. Rust tracks recency;
/// the harness still judges whether the check was sufficient for the change.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct RepositoryVerifierBinding {
    /// Portable capability identity whose repository command is being observed.
    pub capability_id: CapabilityIdV1,
    /// Native-harness tool that exposes the command invocation.
    pub tool_name: String,
    /// Declared shell-free command shape considered during evidence admission.
    pub command: HostCommandConstraint,
}

/// Digest-bound runtime context for one exact neutral execution path.
///
/// Materialization resolves mutable workspace inputs into this bounded record.
/// One-shot hooks reread it instead of recompiling those inputs; `profile_digest`
/// binds portable semantics and `instance_digest` binds the machine resolution.
/// On a decoded value, all such relationships remain claims until the owning
/// loader verifies the schema, digests, referenced bytes, and selected path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HookContextV1 {
    /// Stable wire identity for this hook-context major.
    #[schemars(
        with = "String",
        extend("const" = "pale.agent-surface.hook-context.v1")
    )]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Hook-context schema major encoded as an integer for strict readers.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Descriptor-manifest identity claimed by the producer.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity claimed by the producer.
    pub schema_set_sha256: SchemaSetDigest,
    /// Non-circular identity of the exact inputs from which this projection was rendered.
    pub compilation_input_digest: CompilationInputDigest,
    /// Portable semantic identity shared across machines.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity for paths and executable bindings.
    pub instance_digest: InstanceDigest,
    /// Logical repository whose boundaries and commands this context carries.
    pub repository_id: LogicalRepositoryIdV1,
    /// Compiler proof binding the selected neutral path, its responsibility
    /// identity, ordered delegation spine, and composition-plan identity.
    pub execution_path_witness: ExecutionPathWitnessV1,
    /// Opaque adapter registry identity resolved from the exact provider binding.
    pub adapter_plugin_id: String,
    /// Native-harness executable binding projected when this context was
    /// compiled. It is an audit and dispatch claim; one-shot hooks do not rehash
    /// the parent binary on every tool call, so activation must verify it first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_harness_binding: Option<NativeHarnessBinding>,
    /// Exact declarative run policy selected for the session.
    pub run_policy: RunPolicyEntry,
    /// SHA-256 of the separately materialized injection payload. Keeping only
    /// its digest bounds this hot record; context-carrying events load the body
    /// and verify its bytes before emission.
    pub injection_digest: ArtifactDigest,
    /// Harness-wide integrity closure over the model- and host-facing projection.
    pub projection_manifest: SealedArtifactRef,
    /// Small path-specific binding to a shared native-harness operation
    /// catalog. Activation follows the sealed binding to the compact catalog,
    /// then loads only the selected operation contract.
    pub operation_binding: SealedArtifactRef,
    /// Machine-local session-state declaration. Omitted for profiles that do
    /// not activate a native harness.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_state: Option<SessionStateBinding>,
    /// Retention-bounded evidence storage used for recoverable decisions and
    /// diagnostics that must survive a one-shot hook process. This is not
    /// session continuity and grants no lineage authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub durable_evidence: Option<DurableEvidenceBinding>,
    /// Durable request/result and reconciliation lineage root. Hooks use it
    /// only through separately admitted resident contracts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub materialization_lineage: Option<MaterializationLineageBinding>,
    /// Selected mode defaults, one per independent axis. Empty when the profile
    /// selects no runtime modes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mode_defaults: BTreeMap<ModeAxis, PackageResourceRefV1>,
    /// Digest-bound mode payload identities. Bodies remain separate so the hot
    /// tool-hook path never reads prompt doctrine.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_modes: Vec<SealedRunMode>,
    /// Write-deny roots from authority directories and repositories selected
    /// with read-only access. This projection does not enforce the boundary; a
    /// separately admitted hook or effect adapter must apply it after path admission.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_only_dirs: Vec<String>,
    /// Machine-local state-root paths declared protected from model-facing tools.
    /// Pale itself writes them through confined adapters outside hook policy.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protected_runtime_dirs: Vec<String>,
    /// Path-specific host-tool effects used as the sealed fallback when no
    /// operation lease is active.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub host_tool_effects: Vec<HostToolEffectBinding>,
    /// Exact post-tool-observable verification bindings resolved for this
    /// repository and native harness. No binding means no automatic debt gate.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repository_verifiers: Vec<RepositoryVerifierBinding>,
    /// Lifecycle requirement ids claimed available for this path's native
    /// harness. The hook uses the admitted set to keep
    /// capability-dependent behaviour fail-closed and path-specific.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available_tool_requirements: Vec<CapabilityIdV1>,
    /// Digest-bound, path-local guidance for lifecycle behaviours. The source
    /// package path is discarded at compile time; the hook verifies and loads
    /// only the bodies needed by behaviours that fire for the current event.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub behaviour_guidance: BTreeMap<LifecycleBehaviorIdV1, SealedArtifactRef>,
    /// Lifecycle behaviours selected for this repository and path. Prompt
    /// bodies remain outside this hot record and are verified only when loaded.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behaviours: Vec<LifecycleBehavior>,
}

/// One selected run-mode projection consumed by the one-shot hook runtime.
///
/// A present body digest is a claim until the referenced bytes are loaded and
/// compared; this value does not activate the mode by itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SealedRunMode {
    /// Canonical run-mode ID from the selected catalog.
    pub id: PackageResourceRefV1,
    /// Independent mode axis on which this mode competes.
    pub axis: ModeAxis,
    /// Exact alternate invocations accepted for this mode.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Whole-message triggers that activate this mode; substring matching is not
    /// implied by their presence.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exact_triggers: Vec<String>,
    /// Sealed body artifact. A suppressing mode has no payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<SealedArtifactRef>,
}
