//! Machine-local declarations for executable identities and absolute paths.
//!
//! Deserialization establishes record shape only. The application loader must
//! validate absolute paths, confinement, executable bytes, and identity probes
//! before any binding participates in an admitted route.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::records::{
    DurableEvidenceBinding, ExecutableDigest, LogicalRepositoryIdV1, MaterializationLineageBinding,
    ModeAxis, PackageResourceRefV1, ProviderBindingRefV1, SessionStateBinding,
};

/// Machine identity and optional distribution checkout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineMeta {
    /// Stable machine-binding identity used when resolving an instance.
    #[schemars(length(min = 1))]
    pub id: String,
    /// Version of the machine-binding declaration.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Required by canonical workspace entry; direct compilation supplies its
    /// distribution root separately.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schemars(with = "String", length(min = 1))]
    pub distribution_root: Option<String>,
}

/// Expected identity of the Pale executable selected on this machine.
///
/// Absence is represented by `Option<SurfaceExecutableBinding>` at the owning
/// record. A present value participates in the instance digest. It identifies
/// executable bytes and the interface they must report. It does not prove that
/// a resident is running, healthy, active, or ready.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurfaceExecutableBinding {
    /// Declared absolute executable path outside Git checkouts.
    #[schemars(length(min = 1))]
    pub exec: String,
    /// Expected digest of the executable bytes; the loader verifies it before use.
    pub digest: ExecutableDigest,
    /// Expected version returned by the bounded interface probe.
    #[schemars(length(min = 1))]
    pub version: String,
    /// Interface protocol expected by the compiled projection.
    #[schemars(with = "String", extend("const" = "pale.agent-surface-interface.v1"))]
    pub protocol: String,
}

/// Machine-local executable pin for one exact terminal provider binding.
///
/// The embedded provider reference binds this vector entry to the complete
/// provider-binding identity. The executable pin can vary by machine without
/// weakening the portable binding's semantic identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeHarnessBinding {
    /// Exact provider binding whose terminal adapter owns this executable.
    pub provider_binding: ProviderBindingRefV1,
    /// Declared absolute path to the native-harness executable.
    #[schemars(length(min = 1))]
    pub exec: String,
    /// Expected SHA-256 identity of the executable bytes.
    pub digest: ExecutableDigest,
    /// Expected bounded output of the harness's native `--version` probe.
    #[schemars(length(min = 1, max = 1_024))]
    pub version: String,
}

/// Machine-local checkout projection for one canonical desired repository.
///
/// The path and Git metadata location are machine facts. They never become the
/// repository's semantic identity and changing either cannot rewrite the
/// principal-owned desired graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RepositoryCheckoutBindingV1 {
    /// Canonical desired repository being projected.
    pub repository_id: LogicalRepositoryIdV1,
    /// Declared absolute checkout path.
    #[schemars(length(min = 1))]
    pub checkout_root: String,
    /// Optional absolute common Git metadata directory for linked worktrees.
    ///
    /// Ordinary clones omit this member. A repository-controlled `.git` pointer
    /// is never trusted as write authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String", length(min = 1))]
    pub git_metadata_root: Option<String>,
}

/// Machine-local provider projection for one canonical desired repository.
///
/// The provider contract owns the meaning of the opaque native coordinate.
/// Pale carries it only after provider admission and never parses it into
/// project, execution-workspace, or repository authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderRepositoryProjectionV1 {
    /// Canonical desired repository being projected.
    pub repository_id: LogicalRepositoryIdV1,
    /// Existing neutral [`crate::records::ProviderBindingV1`] identity.
    #[schemars(length(min = 1, max = 256), regex(pattern = r"^[A-Za-z0-9._/-]+$"))]
    pub provider_binding_id: String,
    /// Opaque repository coordinate issued and interpreted by the provider.
    #[schemars(length(min = 1))]
    pub native_repository_coordinate: String,
}

/// User-local defaults. Project overrides win before values enter an instance.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserPreferences {
    /// User-local mode defaults by independent mode axis. Project declarations
    /// may override these values before the instance is compiled.
    #[serde(default)]
    #[schemars(with = "BTreeMap<ModeAxis, PackageResourceRefV1>")]
    pub mode_defaults: BTreeMap<ModeAxis, PackageResourceRefV1>,
}

/// The only input record permitted to declare absolute machine paths.
///
/// String fields remain untrusted until the machine-binding loader parses and
/// validates them; this record does not itself establish filesystem authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineBindingFile {
    /// Machine identity and optional distribution checkout binding.
    pub machine: MachineMeta,
    /// Typed desired-repository to machine-checkout projections.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("uniqueItems" = true))]
    pub repository_checkouts: Vec<RepositoryCheckoutBindingV1>,
    /// Typed desired-repository to provider-native projections.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("uniqueItems" = true))]
    pub provider_repositories: Vec<ProviderRepositoryProjectionV1>,
    /// Logical authority-root id to a declared absolute path.
    pub authority_roots: BTreeMap<String, String>,
    /// Optional user-local defaults, kept outside the portable profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "UserPreferences")]
    pub preferences: Option<UserPreferences>,
    /// Expected Pale executable, absent for compilation-only profiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "SurfaceExecutableBinding")]
    pub surface_executable: Option<SurfaceExecutableBinding>,
    /// Provider-bound terminal executables in provider-reference order.
    /// Unselected entries do not enter a resolved instance or its digest.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("uniqueItems" = true))]
    pub native_harnesses: Vec<NativeHarnessBinding>,
    /// Ephemeral machine-local session-state root and expiry policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "SessionStateBinding")]
    pub session_state: Option<SessionStateBinding>,
    /// Recoverable low-frequency evidence, distinct from session continuity and
    /// immutable materialization lineage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "DurableEvidenceBinding")]
    pub durable_evidence: Option<DurableEvidenceBinding>,
    /// Durable ownership-lineage root and route-evidence capacity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "MaterializationLineageBinding")]
    pub materialization_lineage: Option<MaterializationLineageBinding>,
}
