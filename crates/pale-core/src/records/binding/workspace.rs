//! Entry records for a principal-owned private workspace.
//!
//! [`WorkspaceManifestFile`] is the root composition manifest. It names the
//! distribution and machine-binding roots, owns one canonical desired topology,
//! and explicitly enumerates portable workspace-language sources and locks; the
//! compiler never scans for implicit declarations.

use serde::{Deserialize, Serialize};

use crate::{
    paths::OwnedConfinedRelativePath,
    records::{
        WorkspaceDesiredTopologyV1, WorkspaceId, MAX_WORKSPACE_COMPONENTS, MAX_WORKSPACE_MODULES,
    },
};

fn workspace_file_inventory_schema(
    generator: &mut schemars::SchemaGenerator,
    maximum: usize,
) -> schemars::Schema {
    let member = generator.subschema_for::<OwnedConfinedRelativePath>();
    schemars::Schema::try_from(serde_json::json!({
        "type": "array",
        "maxItems": maximum,
        "items": member
    }))
    .expect("the static workspace file inventory schema is valid")
}

fn module_manifest_paths_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    workspace_file_inventory_schema(generator, MAX_WORKSPACE_MODULES)
}

fn source_component_paths_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    workspace_file_inventory_schema(generator, MAX_WORKSPACE_COMPONENTS)
}

/// Maximum attributable responsibility-source records named by one root.
pub const MAX_RESPONSIBILITY_DERIVATION_SOURCES: usize = 1_024;
/// One organizational source is the complete principal-local translation input.
pub const MAX_ORGANIZATIONAL_TRANSLATION_SOURCES: usize = 1;
/// Zero or one explicit file supplies the complete neutral composition input.
pub const MAX_EXPERIENCE_COMPOSITION_SOURCES: usize = 1;

fn responsibility_derivation_paths_schema(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    workspace_file_inventory_schema(generator, MAX_RESPONSIBILITY_DERIVATION_SOURCES)
}

fn organizational_translation_paths_schema(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    workspace_file_inventory_schema(generator, MAX_ORGANIZATIONAL_TRANSLATION_SOURCES)
}

fn experience_composition_paths_schema(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    workspace_file_inventory_schema(generator, MAX_EXPERIENCE_COMPOSITION_SOURCES)
}

/// The `[workspace]` declaration in the root manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceMeta {
    /// Version of the private-workspace entry contract.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Stable principal-workspace identity; not a project or machine identity.
    pub workspace_id: WorkspaceId,
    /// Path relative to the root manifest.
    pub distribution_lock: OwnedConfinedRelativePath,
    /// Canonical desired project, execution-workspace, and repository graph.
    ///
    /// Machine paths and provider-native coordinates are separate projections;
    /// neither can manufacture or rewrite this graph.
    pub desired_topology: WorkspaceDesiredTopologyV1,
    /// Machine-binding directory relative to the root manifest.
    pub machine_binding_root: OwnedConfinedRelativePath,
    /// Explicit module-manifest files, relative to the root manifest.
    #[schemars(schema_with = "module_manifest_paths_schema")]
    pub module_manifests: Vec<OwnedConfinedRelativePath>,
    /// Explicit local module sources, before publication or adoption.
    /// These are accepted only by the source-only lowering path.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(schema_with = "module_manifest_paths_schema")]
    pub module_sources: Vec<OwnedConfinedRelativePath>,
    /// Explicit source-component files, relative to the root manifest.
    #[schemars(schema_with = "source_component_paths_schema")]
    pub source_components: Vec<OwnedConfinedRelativePath>,
    /// Explicit attributable responsibility-derivation records.
    #[schemars(schema_with = "responsibility_derivation_paths_schema")]
    pub responsibility_derivations: Vec<OwnedConfinedRelativePath>,
    /// Zero or one complete organizational-translation source.
    ///
    /// An empty inventory is an explicit absence; the compiler must not infer
    /// an organization from roles, labels, topology, or responsibility data.
    #[schemars(schema_with = "organizational_translation_paths_schema")]
    pub organizational_translations: Vec<OwnedConfinedRelativePath>,
    /// Zero or one complete neutral experience-composition source.
    ///
    /// An empty inventory selects the explicit empty declaration set. The
    /// compiler does not infer composition from responsibility, organization,
    /// Package, provider, or repository topology.
    #[schemars(schema_with = "experience_composition_paths_schema")]
    pub experience_compositions: Vec<OwnedConfinedRelativePath>,
    /// Resolution-lock file, relative to the root manifest.
    pub resolution_lock: OwnedConfinedRelativePath,
    /// Principal-relative universe-lock file, relative to the root manifest.
    pub universe_lock: OwnedConfinedRelativePath,
}

/// The canonical private-workspace entry point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceManifestFile {
    /// The single private-workspace layout declaration in this file.
    pub workspace: WorkspaceMeta,
}

#[cfg(test)]
#[path = "../../../tests/internal/records/binding/workspace.rs"]
mod tests;
