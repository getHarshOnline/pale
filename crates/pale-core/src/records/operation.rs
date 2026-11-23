//! Package-owned semantic operation declarations.
//!
//! An [`OnDemandOperation`] describes reusable meaning: its doctrine,
//! composition, specialist roles, expected semantic outputs, and portable
//! capability procedure. Private configuration may expose the operation,
//! narrow its Package-owned ceilings, and bind concrete providers; it cannot
//! rewrite requirements, fallbacks, evidence, or forbidden effects. Keeping
//! those owners explicit prevents Package presence from becoming execution
//! authority or runtime-readiness evidence.

use serde::{Deserialize, Serialize};

use crate::paths::OwnedConfinedRelativePath;
use crate::records::{OperationCapabilityRequirement, PackageResourceIdV1, PackageResourceLinkV1};

/// One model-executed operation, distinct from a materialization [`crate::records::Operation`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OnDemandOperation {
    /// Stable operation identity used by selection and rendering.
    pub id: PackageResourceIdV1,
    /// Compact purpose shown during operation discovery.
    pub description: String,
    /// Relative path (under the package) to the Markdown doctrine body carrying
    /// the nuanced steps. The generated surface presents the pointer; the model
    /// executes the doctrine.
    pub doctrine: OwnedConfinedRelativePath,
    /// Procedure archetypes composed before this operation's own doctrine.
    #[serde(default)]
    pub extends: Vec<PackageResourceLinkV1>,
    /// Additional doctrine ids required by this operation.
    #[serde(default)]
    pub doctrine_refs: Vec<PackageResourceLinkV1>,
    /// Specialist roles this operation delegates judgment to.
    #[serde(default)]
    pub roles: Vec<PackageResourceLinkV1>,
    /// Expected semantic deliverables; this grants no native write or publication authority.
    #[serde(default)]
    pub outputs: Vec<String>,
    /// Portable capability requirements owned by this Package operation.
    ///
    /// An empty set means semantic-only when the operation is privately bound.
    /// A nonempty set makes the bound operation capability-bearing, but still
    /// does not prove provider selection, provider execution availability,
    /// admission, or an effect.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub capability_requirements: Vec<OperationCapabilityRequirement>,
    /// Package-owned ceiling applicable to the capability requirements.
    ///
    /// A capability-bearing operation must name exactly one exported
    /// [`crate::records::PackageResourceKindV1::CapabilityPolicy`] resource.
    /// Admission rejects a missing, wrong-kind, unreachable, or widening
    /// policy; private configuration may only narrow the admitted ceiling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_policy: Option<PackageResourceLinkV1>,
}

/// A reusable operation procedure. Archetypes carry shared workflow doctrine;
/// concrete operations add only their semantic delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationArchetype {
    /// Stable reusable operation-archetype identity.
    pub id: PackageResourceIdV1,
    /// Compact purpose of the shared procedure.
    pub description: String,
    /// Package-relative Markdown procedure body.
    pub doctrine: OwnedConfinedRelativePath,
    /// Archetype identities composed before this one.
    #[serde(default)]
    pub extends: Vec<PackageResourceLinkV1>,
    /// Additional doctrine identities required by the archetype.
    #[serde(default)]
    pub doctrine_refs: Vec<PackageResourceLinkV1>,
}

/// The distribution's on-demand operation catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationCatalog {
    /// Version of the on-demand operation declaration contract.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    #[serde(rename = "archetype", default)]
    /// Reusable operation archetypes in authored order.
    pub archetypes: Vec<OperationArchetype>,
    #[serde(rename = "operation")]
    /// Concrete on-demand operations in authored order.
    pub operations: Vec<OnDemandOperation>,
}

impl OperationCatalog {
    /// An operation by id, if known.
    pub fn get(&self, id: &PackageResourceIdV1) -> Option<&OnDemandOperation> {
        self.operations.iter().find(|operation| &operation.id == id)
    }
}

/// `operations.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationsFile {
    /// The single on-demand operation catalog in this file.
    pub operations: OperationCatalog,
}

#[cfg(test)]
#[path = "../../tests/internal/records/operation.rs"]
mod tests;
