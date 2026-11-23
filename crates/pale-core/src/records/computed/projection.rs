//! Digest inventory for one repository's terminal-provider projection.

use serde::{Deserialize, Serialize};

use super::super::{
    ArtifactDigest, InstanceDigest, LogicalRepositoryIdV1, ProfileDigest, ProviderBindingRefV1,
    SchemaId, SurfaceDigest, SurfaceOwnership,
};

/// Stable wire identity carried by [`HarnessProjectionManifestV1::schema`].
pub const HARNESS_PROJECTION_SCHEMA: &str = "pale.agent-surface.harness-projection.v1";

/// Wire major carried by [`HarnessProjectionManifestV1::schema_version`].
pub const HARNESS_PROJECTION_SCHEMA_VERSION: u32 = 1;

/// Repository-relative location of one native harness's projection manifest.
#[must_use]
pub fn harness_projection_manifest_path(provider: &ProviderBindingRefV1) -> String {
    format!(
        "{}/projection-manifest.json",
        provider_binding_projection_root(provider)
    )
}

/// Repository-relative location of the sealed instruction payload for a harness.
#[must_use]
pub fn harness_injection_path(provider: &ProviderBindingRefV1) -> String {
    format!(
        "{}/injection.md",
        provider_binding_projection_root(provider)
    )
}

/// Repository-relative root owned by one exact provider binding.
#[must_use]
pub fn provider_binding_projection_root(provider: &ProviderBindingRefV1) -> String {
    format!(
        ".pale/provider-bindings/{}",
        provider.provider_binding_identity.provider_binding_sha256
    )
}

/// The marker-delimited portion of a user-owned native-harness front door.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ManagedFrontDoorDigest {
    /// Native-harness front-door path relative to the repository root.
    pub rel_path: String,
    /// Stable surface kind; canonical records use `front-door`.
    pub kind: String,
    /// Ownership mode; the front door is a user file with one managed block.
    pub ownership: SurfaceOwnership,
    /// SHA-256 identity of only the Pale-managed block.
    pub managed_block_sha256: ArtifactDigest,
}

/// Every artifact in the projection Pale owns as a whole file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HarnessProjectionManifestV1 {
    /// Stable wire identity of the harness-projection manifest.
    #[schemars(with = "String", extend("const" = HARNESS_PROJECTION_SCHEMA))]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Harness-projection schema major.
    #[schemars(extend("const" = HARNESS_PROJECTION_SCHEMA_VERSION))]
    pub schema_version: u32,
    /// Logical repository receiving this native-harness projection.
    pub repository_id: LogicalRepositoryIdV1,
    /// Exact provider binding whose adapter owns this projection.
    pub provider_binding: ProviderBindingRefV1,
    /// Portable semantic identity shared by all projection artifacts.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity shared by all projection artifacts.
    pub instance_digest: InstanceDigest,
    /// Digest of the managed block inside the user-owned front door.
    pub front_door: ManagedFrontDoorDigest,
    /// Complete set of whole-file artifacts owned by this harness projection.
    pub artifacts: Vec<SurfaceDigest>,
}
