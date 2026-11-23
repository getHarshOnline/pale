//! Portable records for the principal-relative workspace language.
//!
//! The records in this module describe desired, secret-free configuration.
//! They do not represent acquisition, installation, activation, runtime state,
//! native data, or publisher verification. [`WorkspaceModuleV1`] is the module
//! release boundary; [`WorkspaceLanguagePrimitivesV1`] is the finer-grained,
//! content-addressed source boundary used by that module.

use serde::{Deserialize, Deserializer, Serialize};

use crate::paths::OwnedConfinedRelativePath;

use super::{
    ArtifactDigest, ExperienceCompositionSourceDigest, NormativePackReleaseV1,
    OrganizationalTranslationClosureDigest, PackageReleaseV1,
    ResponsibilityDerivationClosureDigest, WorkspaceComponentDigest,
    WorkspaceDerivationReceiptDigest, WorkspaceImportSurfaceDigest, WorkspaceModuleDigest,
    WorkspaceResolutionLockDigest, WorkspaceSemanticIrDigest, WorkspaceUniverseLockDigest,
};

/// Shared lexical grammar for portable workspace identifiers.
const PORTABLE_ID_PATTERN: &str = r"^[a-z][a-z0-9_-]*(?:[.:/][a-z][a-z0-9_-]*)*$";
/// Shared UTF-8 byte ceiling for portable workspace identifiers.
const PORTABLE_ID_MAX_LENGTH: usize = 255;

/// Maximum modules in one resolution or principal adoption frontier.
pub const MAX_WORKSPACE_MODULES: usize = 1_024;
/// Maximum semantic source components admitted by one workspace manifest.
pub const MAX_WORKSPACE_COMPONENTS: usize = 1_024;
/// Maximum declarations in one adopted workspace graph.
pub const MAX_WORKSPACE_DECLARATIONS: usize = 1_024;
/// Maximum dependency edges in one adopted workspace graph.
pub const MAX_WORKSPACE_DECLARATION_EDGES: usize = 4_096;
/// Maximum materialized declaration-access facts in one adopted workspace graph.
pub const MAX_WORKSPACE_ACCESS_ENTRIES: usize = 4_096;
/// Maximum direct dependencies declared by one semantic declaration.
pub const MAX_WORKSPACE_DECLARATION_DEPENDENCIES: usize = 1_024;
/// Maximum imports in one adopted workspace graph.
pub const MAX_WORKSPACE_IMPORTS: usize = 1_024;
/// Maximum selected namespace paths in one adopted workspace graph.
pub const MAX_WORKSPACE_IMPORT_PATHS: usize = 1_024;
/// Maximum exports in one adopted workspace graph.
pub const MAX_WORKSPACE_EXPORTS: usize = 1_024;
/// Maximum content-bearing declaration projections in one named export.
pub const MAX_WORKSPACE_EXPORT_PROJECTIONS: usize = 1_024;
/// Maximum content-bearing export projections across one adopted workspace graph.
pub const MAX_WORKSPACE_EXPORT_PROJECTION_ENTRIES: usize = 4_096;
/// Maximum extensions in one adopted workspace graph.
pub const MAX_WORKSPACE_EXTENSIONS: usize = 1_024;
/// Maximum trait references added by one extension.
pub const MAX_WORKSPACE_EXTENSION_TRAITS: usize = 1_024;
/// Maximum obligations carried by one policy-bearing semantic surface.
pub const MAX_WORKSPACE_OBLIGATIONS: usize = 1_024;
/// Maximum exclusive-authority claims carried by one semantic surface.
pub const MAX_WORKSPACE_AUTHORITY_CLAIMS: usize = 1_024;
/// Maximum members admitted by one independently narrowed policy-set axis.
pub const MAX_WORKSPACE_POLICY_MEMBERS: usize = 1_024;
/// Maximum logical resources in one adopted workspace graph.
pub const MAX_WORKSPACE_RESOURCES: usize = 1_024;
/// Maximum causal source roots retained by one workspace derivation receipt.
pub const MAX_WORKSPACE_DERIVATION_SOURCES: usize =
    MAX_WORKSPACE_COMPONENTS + MAX_WORKSPACE_MODULES + MAX_RESPONSIBILITY_DERIVATIONS + 5;
/// Maximum causal facts retained by one workspace derivation receipt.
pub const MAX_WORKSPACE_DERIVATION_NODES: usize = MAX_WORKSPACE_DERIVATION_SOURCES
    + MAX_WORKSPACE_COMPONENTS
    + MAX_WORKSPACE_MODULES
    + MAX_WORKSPACE_DECLARATIONS * 2
    + MAX_WORKSPACE_IMPORT_PATHS
    + MAX_WORKSPACE_EXTENSIONS
    + MAX_WORKSPACE_RESOURCES
    + 6;
/// Maximum direct causal inputs admitted by one derivation node.
///
/// The semantic-IR sealing rule joins one workspace-manifest fact, the three
/// independently typed responsibility, organization, and experience closure
/// facts, and every bounded module, policy, import, extension, and resource
/// fact. Other derivation rules have narrower limits.
pub const MAX_WORKSPACE_DERIVATION_INPUTS: usize = MAX_WORKSPACE_MODULES
    + MAX_WORKSPACE_DECLARATIONS
    + MAX_WORKSPACE_IMPORT_PATHS
    + MAX_WORKSPACE_EXTENSIONS
    + MAX_WORKSPACE_RESOURCES
    + 4;
/// Maximum change roots retained by one workspace derivation receipt.
pub const MAX_WORKSPACE_DERIVATION_ROOTS: usize = 2 * MAX_WORKSPACE_DERIVATION_SOURCES;
/// Maximum affected/unaffected subjects retained by one derivation receipt.
pub const MAX_WORKSPACE_DERIVATION_IMPACTS: usize = MAX_WORKSPACE_DERIVATION_NODES;
fn bounded_array_schema<T: schemars::JsonSchema>(
    generator: &mut schemars::SchemaGenerator,
    maximum: usize,
    minimum: usize,
) -> schemars::Schema {
    let item = generator.subschema_for::<T>();
    schemars::Schema::try_from(serde_json::json!({
        "type": "array",
        "minItems": minimum,
        "maxItems": maximum,
        "items": item
    }))
    .expect("the static workspace-language inventory schema is valid")
}

fn declarations_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceDeclarationV1>(generator, MAX_WORKSPACE_DECLARATIONS, 0)
}

fn dependencies_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceDeclarationDependencyV1>(
        generator,
        MAX_WORKSPACE_DECLARATION_DEPENDENCIES,
        0,
    )
}

fn source_components_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceSourceComponentRefV1>(generator, MAX_WORKSPACE_COMPONENTS, 0)
}

fn imports_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceImportV1>(generator, MAX_WORKSPACE_IMPORTS, 0)
}

fn import_paths_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceImportPathV1>(generator, MAX_WORKSPACE_IMPORT_PATHS, 1)
}

fn exports_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceExportV1>(generator, MAX_WORKSPACE_EXPORTS, 0)
}

fn export_projections_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceExportProjectionV1>(
        generator,
        MAX_WORKSPACE_EXPORT_PROJECTIONS,
        1,
    )
}

fn extension_traits_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceDeclarationRefV1>(generator, MAX_WORKSPACE_EXTENSION_TRAITS, 0)
}

fn obligation_ids_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceObligationId>(generator, MAX_WORKSPACE_OBLIGATIONS, 0)
}

fn authority_claims_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceExclusiveAuthorityClaimV1>(
        generator,
        MAX_WORKSPACE_AUTHORITY_CLAIMS,
        0,
    )
}

fn extensions_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceExtensionV1>(generator, MAX_WORKSPACE_EXTENSIONS, 0)
}

fn resources_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceResourceSelectionV1>(generator, MAX_WORKSPACE_RESOURCES, 0)
}

fn derivation_sources_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceDerivationSourceV1>(
        generator,
        MAX_WORKSPACE_DERIVATION_SOURCES,
        0,
    )
}

fn derivation_inputs_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceDerivationNodeDigest>(
        generator,
        MAX_WORKSPACE_DERIVATION_INPUTS,
        0,
    )
}

fn derivation_nodes_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceDerivationNodeV1>(generator, MAX_WORKSPACE_DERIVATION_NODES, 0)
}

fn derivation_roots_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceDerivationRootV1>(generator, MAX_WORKSPACE_DERIVATION_ROOTS, 0)
}

fn derivation_root_refs_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceDerivationRootDigest>(
        generator,
        MAX_WORKSPACE_DERIVATION_ROOTS,
        1,
    )
}

fn derivation_impacts_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceDerivationImpactV1>(
        generator,
        MAX_WORKSPACE_DERIVATION_IMPACTS,
        0,
    )
}

fn resolved_releases_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceResolvedReleaseV1>(generator, MAX_WORKSPACE_MODULES, 0)
}

fn adopted_releases_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceAdoptedReleaseV1>(generator, MAX_WORKSPACE_MODULES, 0)
}

macro_rules! portable_id {
    ($(#[$meta:meta])* $name:ident, $label:literal) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema,
        )]
        #[serde(transparent)]
        pub struct $name(
            #[schemars(length(min = 1, max = 255), regex(pattern = PORTABLE_ID_PATTERN))] String,
        );

        impl $name {
            /// Validate and construct this logical identity.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidWorkspaceLanguageId`] when `value` is not a
            /// bounded lowercase logical identity.
            pub fn parse(value: impl Into<String>) -> Result<Self, InvalidWorkspaceLanguageId> {
                let value = value.into();
                let valid = !value.is_empty()
                    && value.len() <= PORTABLE_ID_MAX_LENGTH
                    && value
                        .split(['.', ':', '/'])
                        .all(|part| {
                            let mut chars = part.chars();
                            chars.next().is_some_and(|first| first.is_ascii_lowercase())
                                && chars.all(|ch| {
                                    ch.is_ascii_lowercase()
                                        || ch.is_ascii_digit()
                                        || ch == '_'
                                        || ch == '-'
                                })
                        });
                if valid {
                    Ok(Self(value))
                } else {
                    Err(InvalidWorkspaceLanguageId { kind: $label })
                }
            }

            /// Borrow the stable wire spelling.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::parse(String::deserialize(deserializer)?)
                    .map_err(serde::de::Error::custom)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = InvalidWorkspaceLanguageId;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = InvalidWorkspaceLanguageId;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = InvalidWorkspaceLanguageId;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }
    };
}

/// A logical workspace-language identity is malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{kind} must be a bounded lowercase logical identity")]
pub struct InvalidWorkspaceLanguageId {
    kind: &'static str,
}

#[cfg(test)]
#[path = "../../tests/internal/records/workspace_language.rs"]
mod tests;

mod identity_policy;
pub use identity_policy::*;
mod declaration;
pub use declaration::*;
mod module;
pub use module::*;
mod module_source;
pub use module_source::*;
mod locks;
pub use locks::*;
mod derivation;
pub use derivation::*;
mod responsibility;
pub use responsibility::*;
mod organization;
pub use organization::*;
mod topology;
pub use topology::*;
mod project_owner;
pub use project_owner::*;
