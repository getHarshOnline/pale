//! One Package page's declaration and the ways its single body may be used.
//!
//! The enclosing source supplies the page identity and Markdown body. The
//! declaration supplies its purpose, reading relationships and optional uses.
//! Selecting a task, role or policy does not create another authored body.

use serde::{Deserialize, Serialize};

use crate::records::{
    OfficialReference, OperationCapabilityRequirement, PackageResourceExposureV1,
    PackageResourceKindV1, PackageResourceLinkV1,
};

/// Typed contents of one Package page's `+++` TOML header.
///
/// A page without an explicit use is a guide. Adding a task, role, policy or
/// stack makes that use explicit; include `[guide]` as well when the same page
/// should also be available as a guide. Its identity comes from its relative
/// source path, so the header cannot repeat or override an ID or body path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackagePageV1 {
    /// The job this page helps a reader do.
    #[schemars(length(min = 1, max = 4_096))]
    pub description: String,
    /// Whether another Package or workspace may select the page.
    #[serde(default = "exported")]
    pub exposure: PackageResourceExposureV1,
    /// Cross-topic prerequisites needed for every declared use of this page.
    /// The folder's parent topic is supplied separately as a reading link.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub uses: Vec<PackageResourceLinkV1>,
    /// Related reading that remains optional when this page is selected.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub see_also: Vec<PackageResourceLinkV1>,
    /// Primary references useful for version-sensitive or uncertain claims.
    #[serde(default)]
    #[schemars(length(max = 1_024))]
    pub official_sources: Vec<OfficialReference>,
    /// Explicit guide use when this body also serves another purpose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guide: Option<PackageGuideV1>,
    /// A job that an agent may perform using this method.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<PackageTaskV1>,
    /// A specialist view that applies this method.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<PackageRoleV1>,
    /// Guidance governing the work from startup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<PackagePolicyV1>,
    /// Guidance selected for a language or tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<PackageStackV1>,
}

fn exported() -> PackageResourceExposureV1 {
    PackageResourceExposureV1::Exported
}

impl PackagePageV1 {
    /// Declared semantic uses and their effective visibility, in canonical order.
    /// The implicit guide participates only when no other use was declared.
    #[must_use]
    pub fn declared_uses(&self) -> Vec<(PackageResourceKindV1, PackageResourceExposureV1)> {
        use PackageResourceKindV1 as Kind;
        let mut uses = Vec::new();
        if self.is_guide() {
            uses.push((
                Kind::Doctrine,
                self.guide
                    .as_ref()
                    .and_then(|usage| usage.exposure)
                    .unwrap_or(self.exposure),
            ));
        }
        if let Some(usage) = &self.task {
            uses.push((Kind::Operation, usage.exposure.unwrap_or(self.exposure)));
        }
        if let Some(usage) = &self.role {
            uses.push((Kind::Role, usage.exposure.unwrap_or(self.exposure)));
        }
        if let Some(usage) = &self.policy {
            uses.push((Kind::PolicyFamily, usage.exposure.unwrap_or(self.exposure)));
        }
        if let Some(usage) = &self.stack {
            uses.push((Kind::Stack, usage.exposure.unwrap_or(self.exposure)));
        }
        uses.sort_unstable();
        uses
    }

    /// Whether the page declares a guide, including the ordinary default.
    #[must_use]
    pub fn is_guide(&self) -> bool {
        self.guide.is_some()
            || (self.task.is_none()
                && self.role.is_none()
                && self.policy.is_none()
                && self.stack.is_none())
    }
}

/// The page is available as reusable guidance on its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageGuideV1 {
    /// Override the page's visibility for this use alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposure: Option<PackageResourceExposureV1>,
}

/// The page governs work from startup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackagePolicyV1 {
    /// Override the page's visibility for this use alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposure: Option<PackageResourceExposureV1>,
}

/// Task-specific relationships and requirements, sharing the page's body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageTaskV1 {
    /// Override the page's visibility for this use alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposure: Option<PackageResourceExposureV1>,
    /// Additional reading required by this task rather than every use of the page.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub uses: Vec<PackageResourceLinkV1>,
    /// Specialist roles whose judgment the task needs.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub roles: Vec<PackageResourceLinkV1>,
    /// What the task should produce and where each result belongs.
    #[serde(default)]
    #[schemars(length(max = 1_024))]
    pub outputs: Vec<String>,
    /// Portable capabilities, required evidence and fallback behavior.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub capability_requirements: Vec<OperationCapabilityRequirement>,
    /// Package-owned ceiling for capability-bearing tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_policy: Option<PackageResourceLinkV1>,
}

/// A specialist view that uses the same method as the rest of the page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageRoleV1 {
    /// Override the page's visibility for this use alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposure: Option<PackageResourceExposureV1>,
    /// Additional reading required when the page is used as a role.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub uses: Vec<PackageResourceLinkV1>,
    /// Other roles whose guidance this role includes, without spawning an agent.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub extends: Vec<PackageResourceLinkV1>,
    /// Stack selections for which this specialist view applies.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub applies_to: Vec<PackageResourceLinkV1>,
}

/// Language or tool guidance with explicit parent stacks and role-safe reading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageStackV1 {
    /// Override the page's visibility for this use alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposure: Option<PackageResourceExposureV1>,
    /// More general stacks needed by this one.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub extends: Vec<PackageResourceLinkV1>,
    /// Additional reading needed by this stack use.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub uses: Vec<PackageResourceLinkV1>,
    /// Guidance safe to include in a read-only specialist for this stack.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub role_uses: Vec<PackageResourceLinkV1>,
}

#[cfg(test)]
#[path = "../../tests/internal/records/package_page.rs"]
mod tests;
