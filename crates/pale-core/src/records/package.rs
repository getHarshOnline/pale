//! Package-release manifests and Package-owned semantic-resource declarations.
//!
//! A manifest identifies one immutable Package release, its exact Package
//! dependencies, and the semantic resources and source members it owns. It does not
//! select the release for a workspace, attest normative Pack bytes, bind a
//! provider, or grant native authority. Those responsibilities remain at their
//! respective workspace, Pack, and native-owner boundaries.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize};

use crate::paths::OwnedConfinedRelativePath;
use crate::records::PackageVersionV1;

/// Maximum UTF-8 byte length of a Package or Package-resource identity.
pub const MAX_PACKAGE_ID_BYTES: usize = 255;

/// Maximum exact Package dependencies declared by one initial-v1 release.
///
/// This matches the 1,024-node selected-closure scale while keeping the
/// per-release dependency fan-out independently enforceable.
pub const MAX_PACKAGE_DEPENDENCIES: usize = 1_024;

/// Maximum confined source members in one initial-v1 Package release.
///
/// Four members per selected-closure node at the 1,024-node scale accommodates
/// declaration, body, documentation, and supporting source without making the
/// source inventory unbounded.
pub const MAX_PACKAGE_SOURCES: usize = 4_096;

/// Maximum semantic resources owned by one initial-v1 Package release.
pub const MAX_PACKAGE_RESOURCES: usize = 4_096;

/// A Package identity violates the initial-v1 grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("identity must be a lowercase Package identifier of at most 255 bytes")]
pub struct InvalidPackageIdV1;

/// A resource identity cannot represent a confined lowercase page path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("resource identity must be a lowercase relative path of at most 255 bytes")]
pub struct InvalidPackageResourceIdV1;

/// Validate the one shared lexical grammar for Package-owned identities.
pub(crate) fn package_identifier_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PACKAGE_ID_BYTES
        && value.split('.').all(|segment| {
            let mut bytes = segment.bytes();
            bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
                && bytes.all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'_')
                })
        })
}

/// Canonical Package identity, independent of version, path, and workspace use.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct PackageIdV1(
    #[schemars(
        length(min = 1, max = 255),
        regex(pattern = r"^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)*$")
    )]
    String,
);

impl PackageIdV1 {
    /// Validate and own one canonical Package identity.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidPackageIdV1`] when `value` is empty, exceeds
    /// [`MAX_PACKAGE_ID_BYTES`], or violates the lowercase identifier grammar.
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidPackageIdV1> {
        let value = value.into();
        package_identifier_is_valid(&value)
            .then_some(Self(value))
            .ok_or(InvalidPackageIdV1)
    }

    /// Borrow the validated wire representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for PackageIdV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for PackageIdV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for PackageIdV1 {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Identity of one semantic resource within its owning Package release.
///
/// Resource identities may contain slash-separated path components. Each
/// component follows the lowercase Package identifier grammar; absolute paths,
/// empty components and traversal are rejected. The identity names source
/// meaning, not a path the compiler may open without a checked source owner.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct PackageResourceIdV1(
    #[schemars(
        length(min = 1, max = 255),
        regex(pattern = r"^[a-z][a-z0-9_-]*(?:[./][a-z][a-z0-9_-]*)*$")
    )]
    String,
);

impl PackageResourceIdV1 {
    /// Validate and own one Package-local semantic-resource identity.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidPackageResourceIdV1`] for an empty or oversized value,
    /// traversal, or a component outside the lowercase identifier grammar.
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidPackageResourceIdV1> {
        let value = value.into();
        (value.len() <= MAX_PACKAGE_ID_BYTES && value.split('/').all(package_identifier_is_valid))
            .then_some(Self(value))
            .ok_or(InvalidPackageResourceIdV1)
    }

    /// Borrow the validated wire representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for PackageResourceIdV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for PackageResourceIdV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for PackageResourceIdV1 {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// A parsed `package.toml` for one immutable Package release.
///
/// Parsing establishes the wire shape only. Admission must still validate
/// identities, dependency selection, compiler compatibility, resource ownership,
/// exposure, source completeness, and digest agreement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentSurfacePackageV1 {
    /// Portable package identity and dependency declaration.
    pub package: PackageMeta,
    /// Optional distribution and attribution declaration.
    #[serde(default)]
    pub license: Option<PackageLicense>,
}

/// A parsed Package manifest violates an initial-v1 ownership invariant.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidPackageManifestV1 {
    /// The manifest declares another Package contract major.
    #[error("Package manifest schema_version must be 1, got {0}")]
    UnsupportedSchemaVersion(u32),
    /// The required source inventory is empty.
    #[error("Package source inventory must contain at least one member")]
    EmptySources,
    /// The required semantic-resource inventory is empty.
    #[error("Package resource inventory must contain at least one resource")]
    EmptyResources,
    /// One bounded manifest inventory exceeds its initial-v1 ceiling.
    #[error("Package {inventory} inventory contains {actual} entries; maximum is {maximum}")]
    InventoryTooLarge {
        /// Stable manifest inventory name.
        inventory: &'static str,
        /// Observed entry count.
        actual: usize,
        /// Initial-v1 maximum.
        maximum: usize,
    },
    /// The dependency list selects the same Package owner more than once.
    #[error("duplicate Package dependency: {package_id}")]
    DuplicateDependency {
        /// Duplicated dependency identity.
        package_id: PackageIdV1,
    },
    /// The exact source inventory repeats one confined member path.
    #[error("duplicate Package source member: {path}")]
    DuplicateSource {
        /// Duplicated confined source path.
        path: OwnedConfinedRelativePath,
    },
    /// The manifest assigns one semantic identity more than once.
    #[error("duplicate Package resource: {resource_id}")]
    DuplicateResource {
        /// Duplicated Package-local resource identity.
        resource_id: PackageResourceIdV1,
    },
    /// A resource points outside the declared source inventory.
    #[error("Package resource {resource_id} uses undeclared source {path}")]
    MissingDefinitionSource {
        /// Resource whose source is absent.
        resource_id: PackageResourceIdV1,
        /// Missing confined source path.
        path: OwnedConfinedRelativePath,
    },
    /// A resource points to a source member that is not a declaration.
    #[error("Package resource {resource_id} uses non-declaration source {path}")]
    DefinitionSourceRole {
        /// Resource whose source role is invalid.
        resource_id: PackageResourceIdV1,
        /// Confined source path with the wrong role.
        path: OwnedConfinedRelativePath,
    },
}

/// The `[package]` table for one immutable Package release.
///
/// [`Self::resources`] is the sole manifest-level ownership surface. Directory
/// placement and matching identifiers do not create Package ownership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageMeta {
    /// Canonical Package identity selected through a distribution lock.
    pub id: PackageIdV1,
    /// Exact Package release declared by this source.
    pub version: PackageVersionV1,
    /// Version of the Package declaration contract.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Human-readable Package title; never used as identity or authority.
    pub title: String,
    /// Compact Package purpose shown during inspection.
    pub description: String,
    /// Compiler release constraints declared by this Package.
    #[schemars(extend("uniqueItems" = true))]
    pub compatible_compilers: Vec<String>,
    /// Exact Package releases required to make imported meaning available.
    ///
    /// Dependencies do not select every exported resource from the dependency
    /// and do not transfer semantic or native authority.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub dependencies: Vec<PackageDependencyV1>,
    /// Complete confined source inventory for this Package release.
    /// When omitted with `resources`, admission derives both from the sources.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(min = 1, max = 4_096), extend("uniqueItems" = true))]
    pub sources: Vec<PackageSourceV1>,
    /// Semantic resources canonically owned by this release.
    /// An omitted inventory is derived from page headers and typed data files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(min = 1, max = 4_096), extend("uniqueItems" = true))]
    pub resources: Vec<PackageResourceV1>,
}

impl AgentSurfacePackageV1 {
    /// Validate bounded inventories and manifest-owned source relationships.
    ///
    /// This is source-shape admission only. It does not resolve the selected
    /// [`crate::records::PackageReleaseV1`], dependency closure, semantic
    /// references, Pack coordinates, workspace occurrence, or native effects.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidPackageManifestV1`] for empty or oversized required
    /// inventories, duplicate semantic owners, or a resource definition that
    /// is absent from the exact declaration-source inventory.
    pub fn validate_contract(&self) -> Result<(), InvalidPackageManifestV1> {
        let package = &self.package;
        if package.schema_version != 1 {
            return Err(InvalidPackageManifestV1::UnsupportedSchemaVersion(
                package.schema_version,
            ));
        }
        if package.sources.is_empty() {
            return Err(InvalidPackageManifestV1::EmptySources);
        }
        if package.resources.is_empty() {
            return Err(InvalidPackageManifestV1::EmptyResources);
        }
        for (inventory, actual, maximum) in [
            (
                "dependency",
                package.dependencies.len(),
                MAX_PACKAGE_DEPENDENCIES,
            ),
            ("source", package.sources.len(), MAX_PACKAGE_SOURCES),
            ("resource", package.resources.len(), MAX_PACKAGE_RESOURCES),
        ] {
            if actual > maximum {
                return Err(InvalidPackageManifestV1::InventoryTooLarge {
                    inventory,
                    actual,
                    maximum,
                });
            }
        }

        let mut dependencies = BTreeSet::new();
        for dependency in &package.dependencies {
            if !dependencies.insert(&dependency.id) {
                return Err(InvalidPackageManifestV1::DuplicateDependency {
                    package_id: dependency.id.clone(),
                });
            }
        }

        let mut sources = std::collections::BTreeMap::new();
        for source in &package.sources {
            if sources.insert(&source.path, source.role).is_some() {
                return Err(InvalidPackageManifestV1::DuplicateSource {
                    path: source.path.clone(),
                });
            }
        }

        let mut resources = BTreeSet::new();
        let mut page_owners = std::collections::BTreeMap::new();
        for resource in &package.resources {
            let owner_conflict = page_owners
                .insert(&resource.id, &resource.definition_source)
                .is_some_and(|previous| {
                    previous != &resource.definition_source
                        || !resource.definition_source.as_str().ends_with(".md")
                });
            if !resources.insert((&resource.id, resource.kind)) || owner_conflict {
                return Err(InvalidPackageManifestV1::DuplicateResource {
                    resource_id: resource.id.clone(),
                });
            }
            match sources.get(&resource.definition_source) {
                None => {
                    return Err(InvalidPackageManifestV1::MissingDefinitionSource {
                        resource_id: resource.id.clone(),
                        path: resource.definition_source.clone(),
                    });
                }
                Some(PackageSourceRoleV1::Declaration) => {}
                Some(PackageSourceRoleV1::Body | PackageSourceRoleV1::Documentation) => {
                    return Err(InvalidPackageManifestV1::DefinitionSourceRole {
                        resource_id: resource.id.clone(),
                        path: resource.definition_source.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

/// One exact Package dependency declared by a Package release.
///
/// Content identity is supplied by the selected [`crate::records::PackageReleaseV1`]
/// in the distribution lock. Keeping the digest out of the source manifest avoids
/// a dependency-digest cycle while retaining an exact `(id, version)` edge.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct PackageDependencyV1 {
    /// Canonical dependency Package identity.
    pub id: PackageIdV1,
    /// Exact required Package release.
    pub version: PackageVersionV1,
}

/// Closed semantic-resource vocabulary owned by Package manifests.
///
/// This classification controls discovery and selection; it does not turn an
/// resource into Pack truth, workspace selection, provider configuration,
/// Pale Runtime assembly, activation, or native effect.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PackageResourceKindV1 {
    /// One reusable judgment or startup-policy doctrine record.
    Doctrine,
    /// One reusable on-demand operation archetype.
    OperationArchetype,
    /// One reusable on-demand operation.
    Operation,
    /// One specialist-role archetype.
    RoleArchetype,
    /// One specialist role.
    Role,
    /// One technology or workflow stack overlay.
    Stack,
    /// One independent run-mode declaration.
    RunMode,
    /// One reusable policy-family declaration and doctrine body.
    PolicyFamily,
    /// One scoped governing-constraint and ordering fragment.
    GoverningOrder,
    /// One portable Package-owned capability/effect ceiling.
    CapabilityPolicy,
    /// One portable provider-neutral call, evidence, and fallback recipe.
    ToolRecipe,
}

/// Owner-qualified identity of one Package semantic resource.
///
/// The distribution lock resolves [`Self::package_id`] to one exact Package
/// release. The resource kind remains implicit in typed consuming fields such
/// as repository roles and stacks, and is checked against the owning export at
/// admission. This avoids two independently mutable kind declarations.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct PackageResourceRefV1 {
    /// Canonical identity of the Package that owns the resource.
    pub package_id: PackageIdV1,
    /// Resource identity within the owning Package release.
    pub resource_id: PackageResourceIdV1,
}

impl std::fmt::Display for PackageResourceRefV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:{}", self.package_id, self.resource_id)
    }
}

/// Authored reference to a Package-owned semantic resource.
///
/// [`Self::Local`] keeps same-Package authoring concise. [`Self::External`]
/// makes a cross-Package owner explicit, so dependency reachability and a
/// matching local string can never silently substitute for ownership.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "scope", rename_all = "snake_case", deny_unknown_fields)]
pub enum PackageResourceLinkV1 {
    /// Resource owned by the Package containing this declaration.
    Local {
        /// Package-local semantic-resource identity.
        resource_id: PackageResourceIdV1,
    },
    /// Resource exported by one exact dependency Package.
    External {
        /// Owner-qualified dependency resource.
        resource: PackageResourceRefV1,
    },
}

impl PackageResourceLinkV1 {
    /// Resolve this authored link against the containing Package owner.
    ///
    /// Resolution qualifies a local identity; it does not prove that the
    /// resource exists, is exported, has the expected kind, or is reachable
    /// through the selected Package dependency closure.
    #[must_use]
    pub fn qualify(&self, local_package: &PackageIdV1) -> PackageResourceRefV1 {
        match self {
            Self::Local { resource_id } => PackageResourceRefV1 {
                package_id: local_package.clone(),
                resource_id: resource_id.clone(),
            },
            Self::External { resource } => resource.clone(),
        }
    }
}

/// Why one source member belongs to an immutable Package release.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PackageSourceRoleV1 {
    /// TOML that declares Package-owned semantic structure or relationships.
    Declaration,
    /// Markdown carrying reusable semantic substance.
    Body,
    /// Maintainer-facing material shipped as part of the exact release.
    Documentation,
}

/// One member of the complete Package source inventory.
///
/// Package digesting includes `package.toml` plus every listed member. Admission
/// rejects duplicate, absent, escaping, and orphaned files so this inventory is
/// an exact byte boundary rather than a best-effort discovery hint.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct PackageSourceV1 {
    /// Confined Package-relative member path.
    pub path: OwnedConfinedRelativePath,
    /// Role of the member in the immutable release.
    pub role: PackageSourceRoleV1,
}

/// Whether a Package-owned resource is available across the Package boundary.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PackageResourceExposureV1 {
    /// Available only while compiling resources owned by this Package.
    Internal,
    /// Available to selected dependent Packages and workspace occurrences.
    Exported,
}

/// One semantic resource owned by a Package release.
///
/// [`Self::definition_source`] must name a declaration member in the manifest's
/// [`PackageMeta::sources`] inventory. The path is a source coordinate, not a
/// filesystem capability; a Pale adapter must still resolve it beneath the
/// retained Package root without following escaping links.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct PackageResourceV1 {
    /// Resource identity unique within the owning Package release.
    pub id: PackageResourceIdV1,
    /// Closed semantic class used for typed loading and selection.
    pub kind: PackageResourceKindV1,
    /// Confined Package-relative declaration source from [`PackageMeta::sources`].
    pub definition_source: OwnedConfinedRelativePath,
    /// Visibility across this Package's semantic boundary.
    pub exposure: PackageResourceExposureV1,
}

/// Exact or Package-scoped selection of one typed semantic resource family.
///
/// The enclosing field supplies the required [`PackageResourceKindV1`]. The
/// wildcard variant is deliberately scoped to one selected Package and selects
/// only exported resources; there is no distribution-global `*` that can expose
/// resources from unrelated releases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "selection", rename_all = "snake_case", deny_unknown_fields)]
pub enum PackageResourceSelectorV1 {
    /// Select one exact owner-qualified resource.
    Exact {
        /// Exact semantic resource seed.
        resource: PackageResourceRefV1,
    },
    /// Select every exported resource of the enclosing kind from one Package.
    AllFromPackage {
        /// Canonical Package identity resolved through the distribution lock.
        package_id: PackageIdV1,
    },
}

/// The optional `[license]` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageLicense {
    /// SPDX license expression governing the Package source.
    pub spdx: String,
    /// Optional attribution or scope clarification.
    #[serde(default)]
    pub note: Option<String>,
}

/// A parsed `family.toml` (`[family]` wraps a [`PolicyFamilyV1`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FamilyFile {
    /// The single policy-family declaration in this file.
    pub family: PolicyFamilyV1,
}

/// Reusable policy judgment owned by one Package export.
///
/// A family owns its identity, purpose, and doctrine body. Governing
/// constraints, narrowing, semantic dependencies, and presentation order live
/// in their respective typed relations rather than being collapsed into one
/// universal stage label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyFamilyV1 {
    /// Package-local family identity.
    pub id: PackageResourceIdV1,
    /// Human-readable family title.
    pub title: String,
    /// Compact semantic purpose of the family.
    pub purpose: String,
    /// Confined Package-relative path to renderer-neutral doctrine Markdown.
    pub body: OwnedConfinedRelativePath,
}

#[cfg(test)]
#[path = "../../tests/internal/records/package.rs"]
mod tests;
