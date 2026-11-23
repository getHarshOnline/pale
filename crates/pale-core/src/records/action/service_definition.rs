//! Provider-neutral service definitions normalized before host materialization.

use serde::{Deserialize, Serialize};

/// Contract and dispatch key for service-definition normalization in
/// [`crate::records::NativeActionCatalogV1`].
pub const SERVICE_DEFINITION_NORMALIZE_ACTION_ID: &str =
    "pale.compute.service-definition-normalize";
/// Maximum service definitions in one initial-v1 normalization request.
pub const SERVICE_DEFINITION_MAX_SERVICES_V1: usize = 128;
/// Maximum literal process arguments in one initial-v1 service declaration.
pub const SERVICE_DEFINITION_MAX_ARGUMENTS_V1: usize = 128;
/// Maximum UTF-8 bytes in one literal initial-v1 process argument.
pub const SERVICE_DEFINITION_MAX_ARGUMENT_BYTES_V1: usize = 4_096;
/// Maximum ports in one initial-v1 service declaration.
pub const SERVICE_DEFINITION_MAX_PORTS_V1: usize = 64;
/// Maximum direct dependencies in one initial-v1 service declaration.
pub const SERVICE_DEFINITION_MAX_DEPENDENCIES_V1: usize = 128;
/// Maximum UTF-8 bytes in an initial-v1 service identity.
pub const SERVICE_DEFINITION_MAX_NAME_BYTES_V1: usize = 128;
/// Maximum UTF-8 bytes in an initial-v1 service class.
pub const SERVICE_DEFINITION_MAX_KIND_BYTES_V1: usize = 128;
/// Maximum UTF-8 bytes in an initial-v1 launch-program token.
pub const SERVICE_DEFINITION_MAX_PROGRAM_BYTES_V1: usize = 512;

/// Executable and literal argument vector used to launch one service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServiceLaunch {
    /// Literal executable identity; this action never resolves or runs it.
    ///
    /// The schema uses the same numeric ceiling as
    /// [`SERVICE_DEFINITION_MAX_PROGRAM_BYTES_V1`]; application admission also
    /// interprets that constant as a UTF-8 byte ceiling.
    #[schemars(length(min = 1, max = 512))]
    pub program: String,
    /// Literal argument vector preserved without shell interpretation.
    ///
    /// Its schema uses the numeric ceilings
    /// [`SERVICE_DEFINITION_MAX_ARGUMENTS_V1`] and
    /// [`SERVICE_DEFINITION_MAX_ARGUMENT_BYTES_V1`]; application admission interprets
    /// the latter as a UTF-8 byte ceiling.
    #[serde(default)]
    #[schemars(length(max = 128), inner(length(max = 4_096)))]
    pub args: Vec<String>,
}

/// Provider-neutral declaration of one locally materializable service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServiceDefinition {
    /// Portable lowercase service identity.
    ///
    /// The lexical schema is locked to [`SERVICE_DEFINITION_MAX_NAME_BYTES_V1`].
    #[schemars(regex(pattern = r"^[a-z][a-z0-9-]{0,127}$"))]
    pub name: String,
    /// Caller-declared service class interpreted by a later materializer.
    ///
    /// The schema uses the same numeric ceiling as
    /// [`SERVICE_DEFINITION_MAX_KIND_BYTES_V1`]; application admission also
    /// interprets that constant as a UTF-8 byte ceiling.
    #[schemars(length(min = 1, max = 128))]
    pub kind: String,
    /// Dependency order; every dependency must have a lower value.
    pub precedence: u32,
    /// Provider-neutral launch declaration; normalization performs no process effect.
    pub launch: ServiceLaunch,
    /// Unique, non-zero ports reserved by this service.
    ///
    /// The schema numeric ceiling tracks [`SERVICE_DEFINITION_MAX_PORTS_V1`].
    #[serde(default)]
    #[schemars(
        length(max = 64),
        inner(range(min = 1, max = 65_535)),
        extend("uniqueItems" = true)
    )]
    pub ports: Vec<u16>,
    /// Names of declared services with lower precedence.
    ///
    /// The schema numeric ceilings track
    /// [`SERVICE_DEFINITION_MAX_DEPENDENCIES_V1`] and
    /// [`SERVICE_DEFINITION_MAX_NAME_BYTES_V1`].
    #[serde(default)]
    #[schemars(
        length(max = 128),
        inner(regex(pattern = r"^[a-z][a-z0-9-]{0,127}$")),
        extend("uniqueItems" = true)
    )]
    pub depends_on: Vec<String>,
}

/// Service definitions to validate and normalize as one dependency set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServiceDefinitionInput {
    /// Complete service set validated as one dependency and port namespace.
    ///
    /// The schema numeric ceiling tracks [`SERVICE_DEFINITION_MAX_SERVICES_V1`].
    #[schemars(length(min = 1, max = 128))]
    pub services: Vec<ServiceDefinition>,
}

/// Validated services sorted by precedence then name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServiceDefinitionOutput {
    /// Definitions with ports and dependencies sorted, then services ordered by
    /// precedence and name.
    pub services: Vec<ServiceDefinition>,
}
