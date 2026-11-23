//! Exact uses of Package pages in relations without a kind-specific field.

use serde::{Deserialize, Serialize};

use super::{
    PackageResourceIdV1, PackageResourceKindV1, PackageResourceLinkV1, PackageResourceRefV1,
};

/// One admitted use of a Package resource, independent of its shared body.
///
/// A page may supply both a guide and a role. This pair keeps their selection,
/// visibility and governing relations distinct throughout the semantic graph.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct PackageResourceUseV1 {
    /// Page or data resource that owns this use.
    pub resource: PackageResourceRefV1,
    /// Meaning selected from that resource.
    pub kind: PackageResourceKindV1,
}

impl PackageResourceUseV1 {
    /// Bind a source reference to the kind required by its consuming relation.
    #[must_use]
    pub fn new(resource: PackageResourceRefV1, kind: PackageResourceKindV1) -> Self {
        Self { resource, kind }
    }
}

impl std::fmt::Display for PackageResourceUseV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} ({:?})", self.resource, self.kind)
    }
}

/// Authored governing endpoint with an optional explicit use kind.
///
/// Omission is valid only when the target has one declared use. Admission must
/// reject ambiguity rather than infer a kind from another endpoint or silently
/// combine all the meanings supplied by a shared page.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "scope", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoverningResourceLinkV1 {
    /// Use owned by the Package containing this declaration.
    Local {
        /// Package-local page or data identity.
        resource_id: PackageResourceIdV1,
        /// Exact use; omission requires an unambiguous target.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<PackageResourceKindV1>,
    },
    /// Use exported by a declared dependency Package.
    External {
        /// Owner-qualified dependency resource.
        resource: PackageResourceRefV1,
        /// Exact exported use; omission requires an unambiguous target.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<PackageResourceKindV1>,
    },
}

impl GoverningResourceLinkV1 {
    /// Source coordinate to check for ownership, dependency and visibility.
    #[must_use]
    pub fn resource_link(&self) -> PackageResourceLinkV1 {
        match self {
            Self::Local { resource_id, .. } => PackageResourceLinkV1::Local {
                resource_id: resource_id.clone(),
            },
            Self::External { resource, .. } => PackageResourceLinkV1::External {
                resource: resource.clone(),
            },
        }
    }

    /// Kind chosen by the author, if the endpoint does not rely on uniqueness.
    #[must_use]
    pub fn kind(&self) -> Option<PackageResourceKindV1> {
        match self {
            Self::Local { kind, .. } | Self::External { kind, .. } => *kind,
        }
    }
}

impl From<PackageResourceLinkV1> for GoverningResourceLinkV1 {
    fn from(link: PackageResourceLinkV1) -> Self {
        match link {
            PackageResourceLinkV1::Local { resource_id } => Self::Local {
                resource_id,
                kind: None,
            },
            PackageResourceLinkV1::External { resource } => Self::External {
                resource,
                kind: None,
            },
        }
    }
}
