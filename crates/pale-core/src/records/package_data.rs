//! Typed Package data whose identity comes from its source path.

use serde::{Deserialize, Serialize};

use crate::records::{
    CapabilityPolicy, GoverningOrderV1, PackageResourceExposureV1, PackageResourceKindV1,
    ToolRecipe,
};

/// A Package-owned tool recipe, capability ceiling or governing order.
///
/// These records contain data rather than an instruction body. The discriminator
/// selects the grammar; the enclosing relative `.toml` path supplies identity.
/// Admission still resolves references and checks any resulting capability use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PackageDataV1 {
    /// Describe a portable tool need and the evidence its result must carry.
    ToolRecipe {
        /// Whether other Packages and workspaces may select this recipe.
        #[serde(default = "exported")]
        exposure: PackageResourceExposureV1,
        /// The tool requirement; the consumer supplies the actual binding.
        recipe: ToolRecipe,
    },
    /// Limit the capabilities and effects that a task may request.
    CapabilityPolicy {
        /// Whether another Package may refer to this ceiling.
        #[serde(default = "exported")]
        exposure: PackageResourceExposureV1,
        /// The ceiling, independent of provider availability or permission.
        policy: CapabilityPolicy,
    },
    /// Declare scoped constraints and ordering relationships.
    GoverningOrder {
        /// Whether another Package may select this order.
        #[serde(default = "exported")]
        exposure: PackageResourceExposureV1,
        /// The relationships, which admission resolves against typed resources.
        governing_order: GoverningOrderV1,
    },
}

fn exported() -> PackageResourceExposureV1 {
    PackageResourceExposureV1::Exported
}

impl PackageDataV1 {
    /// The semantic use represented by this data file.
    #[must_use]
    pub fn resource_kind(&self) -> PackageResourceKindV1 {
        match self {
            Self::ToolRecipe { .. } => PackageResourceKindV1::ToolRecipe,
            Self::CapabilityPolicy { .. } => PackageResourceKindV1::CapabilityPolicy,
            Self::GoverningOrder { .. } => PackageResourceKindV1::GoverningOrder,
        }
    }

    /// Visibility declared by this file, with exported visibility as the default.
    #[must_use]
    pub fn exposure(&self) -> PackageResourceExposureV1 {
        match self {
            Self::ToolRecipe { exposure, .. }
            | Self::CapabilityPolicy { exposure, .. }
            | Self::GoverningOrder { exposure, .. } => *exposure,
        }
    }
}
