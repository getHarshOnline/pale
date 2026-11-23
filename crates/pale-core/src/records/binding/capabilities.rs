//! Project capability providers and repository commands.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::records::{
    CapabilityBinding, CapabilityIdV1, CapabilityNarrowingV1, LifecycleBehaviorCatalog,
    OperationBindingEntryV1, RunPolicyEntry, SemanticSelection,
};

/// JSON Schema for one nonempty capability-to-command map.
fn repository_commands_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    let mut schema = <BTreeMap<
        super::CapabilityIdSchema,
        super::NonEmptyCommandListSchema,
    > as schemars::JsonSchema>::json_schema(generator);
    schema.insert("minProperties".into(), 1.into());
    schema
}

/// Exact capability commands for one repository role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RepositoryCommandCapabilities {
    /// Portable capability id to repository-declared command strings. Commands are
    /// parsed and admitted before execution; this record grants no shell access.
    #[schemars(schema_with = "repository_commands_schema")]
    pub commands: BTreeMap<CapabilityIdV1, Vec<String>>,
    /// Commands known to mutate state even when their capability class could
    /// otherwise be used for inspection.
    #[serde(default)]
    #[schemars(
        inner(length(min = 1)),
        extend("uniqueItems" = true)
    )]
    pub mutating_not_readonly: Vec<String>,
    /// Optional operator-facing context about this repository-role command set.
    /// The note is descriptive and grants no capability or execution authority.
    #[serde(default)]
    #[schemars(with = "String", length(min = 1))]
    pub note: Option<String>,
}

/// Authority roots from which the volatile forbidden-token set is resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoundaryCapability {
    /// Logical authority-root ids, resolved only through the machine binding.
    #[schemars(
        inner(length(min = 1)),
        extend("uniqueItems" = true)
    )]
    pub forbidden_token_authority: Vec<String>,
}

/// Render-time project data that reusable packages deliberately do not hardcode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilitiesFile {
    /// Repository-role IDs to their exact project-declared command providers.
    #[serde(default)]
    #[schemars(with = "BTreeMap<super::BindingIdSchema, RepositoryCommandCapabilities>")]
    pub repository_commands: BTreeMap<String, RepositoryCommandCapabilities>,
    /// Optional authority roots used to resolve volatile boundary tokens.
    #[serde(default)]
    #[schemars(with = "BoundaryCapability")]
    pub boundary: Option<BoundaryCapability>,
    /// Distribution semantic records selected for this project.
    #[serde(default)]
    pub semantic: SemanticSelection,
    /// Private narrowing applied to selected Package-owned capability procedure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "CapabilityNarrowingV1")]
    pub capability_narrowing: Option<CapabilityNarrowingV1>,
    /// Portable capability id to a concrete private provider selection.
    #[serde(default)]
    #[schemars(with = "BTreeMap<super::CapabilityIdSchema, CapabilityBinding>")]
    pub capability_bindings: BTreeMap<CapabilityIdV1, CapabilityBinding>,
    /// Owner-qualified semantic operations and their private exposure.
    ///
    /// A selected operation absent from this list is **unbound** and receives no
    /// discovery or execution surface. A present binding with no capability
    /// Package-owned requirements is the distinct semantic-only state. A
    /// present binding with Package requirements is capability-bearing; the
    /// private record cannot assert either classification. Admission rejects
    /// duplicate [`OperationBindingEntryV1::operation`] values before
    /// deterministic normalization.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub operation_bindings: Vec<OperationBindingEntryV1>,
    /// Private lifecycle action declarations for this project.
    ///
    /// This is the only source home for concrete lifecycle mechanics. It is
    /// neither Package meaning nor a distribution-global semantic catalog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<LifecycleBehaviorCatalog>,
    /// Private lifecycle policies available to this project.
    ///
    /// Package source does not supply these mechanics. The project selects
    /// exactly one entry by [`RunPolicyEntry::id`], and compilation carries the
    /// selected record into the effective-profile digest.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub run_policies: Vec<RunPolicyEntry>,
}
