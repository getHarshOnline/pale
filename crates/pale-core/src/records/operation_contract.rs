//! Sealed operation contracts consumed by lifecycle admission.
//!
//! Markdown owns nuanced procedure judgment. These records contain only the
//! deterministic identity, capability, provider, approval, and observability
//! facts the runtime can verify without interpreting prose.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::records::{
    Approval, CapabilityIdV1, CapabilityProvider, CapabilityRequirementIdV1, EffectClass,
    ExecutionPathWitnessV1, FailurePosture, LogicalRepositoryIdV1, NonRootJsonPointer,
    OperationBindingDigest, OperationCatalogDigest, OperationContractDigest, OperationSupport,
    PackageResourceRefV1, ProfileDigest, ProviderExecutionAvailability,
    ResolvedCapabilityRequirementPolicyV1, SchemaId, SealedArtifactRef, ToolObservability,
};

// Each family validates its own exact initial-v1 coordinate. Keeping the
// spellings beside the shared family admission code prevents callers from
// supplying a schema identity that belongs to a sibling record.
/// Exact schema coordinate admitted for an [`OperationContractV1`].
const OPERATION_CONTRACT_SCHEMA: &str = "pale/operation-contract.v1";
/// Exact schema coordinate admitted for an [`OperationContractCatalogV1`].
const OPERATION_CATALOG_SCHEMA: &str = "pale/operation-catalog.v1";
/// Exact schema coordinate admitted for an [`OperationRouteBindingV1`].
const OPERATION_ROUTE_BINDING_SCHEMA: &str = "pale/operation-route-binding.v1";

/// A sealed operation-family record contradicts its initial-v1 contract.
#[derive(Debug, thiserror::Error)]
pub enum InvalidOperationContractFamilyV1 {
    /// The record does not carry its exact initial-v1 schema coordinate.
    #[error("{family} schema coordinate is not initial-v1")]
    SchemaCoordinate {
        /// Stable admitted family name.
        family: &'static str,
    },
    /// Capability-bearing mechanics contain no requirements.
    #[error("capability-bearing operation has no capability requirements")]
    EmptyCapabilities,
    /// A set-like sequence is duplicated or not strictly canonical.
    #[error("{relation} is not strictly ordered and unique")]
    NonCanonical {
        /// Stable relation name.
        relation: &'static str,
    },
    /// The claimed digest differs from the validated canonical preimage.
    #[error("{family} digest does not match its canonical preimage")]
    DigestMismatch {
        /// Stable admitted family name.
        family: &'static str,
    },
    /// The validated body cannot be represented by canonical JSON.
    #[error(transparent)]
    Canon(#[from] crate::canon::CanonError),
}

/// A shell-free command identity. The host may expose a shell-shaped tool, but
/// Pale admits only a parsed executable and argument prefix; shell operators,
/// expansion, redirection, and command substitution are rejected before this
/// constraint is evaluated.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct HostCommandConstraint {
    /// Exact executable token admitted without shell interpretation.
    pub executable: String,
    /// Required leading arguments, compared as parsed argument values.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub argv_prefix: Vec<String>,
    /// Whether arguments may follow `argv_prefix`; otherwise the vector must match
    /// the complete argument list.
    #[serde(default)]
    pub allow_trailing_args: bool,
}

/// Declarative source for command constraints that are repository-specific.
/// Sources are expanded while projecting one repository and never survive in
/// its sealed operation contract.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum HostCommandSource {
    /// Expand constraints from the repository's selected capability commands.
    RepositoryCapabilityCommands,
}

/// Model-callable and native-event tool identities at one observable host boundary.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct HostToolBinding {
    /// Exact tool name reported by the native lifecycle event.
    pub name: String,
    /// Exact model-callable name when it differs from the native event name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String", length(min = 1))]
    pub model_name: Option<String>,
    /// Earliest host boundary at which Pale can observe the request.
    pub observability: ToolObservability,
    /// Optional parsed-command allowlist. Empty means the named tool has one
    /// admitted semantic surface; multipurpose shell tools require either
    /// concrete command constraints or a repository command source.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<HostCommandConstraint>,
    /// Repository-derived command source used instead of inline constraints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_source: Option<HostCommandSource>,
    /// Optional exact string discriminators over the host tool's JSON input.
    /// Keys are JSON Pointers (for example `/action`); all predicates must
    /// match. This narrows multiplexed connector tools without interpreting
    /// their domain payload in Rust.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub input_equals: BTreeMap<NonRootJsonPointer, String>,
}

impl HostToolBinding {
    /// Tool name exposed to the model; uniform surfaces use the native name.
    /// This projection never changes native event matching or command admission.
    #[must_use]
    pub fn model_tool_name(&self) -> &str {
        self.model_name.as_deref().unwrap_or(&self.name)
    }
}

/// One execution-path-specific host tool contract sealed into the hot hook context.
///
/// This is the deterministic effect registry used by authority-boundary enforcement outside an
/// active operation. It reuses the exact binding grammar used by operation admission rather than
/// maintaining a second list of command names.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct HostToolEffectBinding {
    /// Effect enforced when the host tool constraint matches.
    pub effect_class: EffectClass,
    /// Exact host tool and optional input constraints.
    pub tool: HostToolBinding,
}

/// One operation requirement resolved for one native-harness execution path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationCapabilityContractV1 {
    /// Operation-local requirement identity.
    pub requirement_id: CapabilityRequirementIdV1,
    /// Portable capability identity required by Package procedure.
    pub capability_id: CapabilityIdV1,
    /// Admitted Package-owned recipe used for this check, when selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<PackageResourceRefV1>,
    /// Effect expected from the concrete provider.
    pub effect_class: EffectClass,
    /// Closed unavailable-provider policy selected for this requirement.
    pub policy: ResolvedCapabilityRequirementPolicyV1,
    /// Failure posture when an admitted provider invocation fails.
    pub failure_posture: FailurePosture,
    /// Provider selection and execution-path-specific availability observation.
    pub provider_state: OperationCapabilityProviderStateV1,
}

impl OperationCapabilityContractV1 {
    /// Whether the operation's full result depends on this capability.
    #[must_use]
    pub fn required(&self) -> bool {
        self.policy.requirement() == crate::records::Requirement::Required
    }

    /// Whether the selected provider is usable on this exact route.
    #[must_use]
    pub fn available(&self) -> bool {
        self.provider_state.execution_availability() == ProviderExecutionAvailability::Available
    }

    /// Support contribution of an unavailable required capability.
    #[must_use]
    pub fn unavailable_support(&self) -> Option<OperationSupport> {
        if self.available() || !self.required() {
            return None;
        }
        Some(match self.policy.on_unavailable() {
            crate::records::CapabilityUnavailability::Block => OperationSupport::Blocked,
            crate::records::CapabilityUnavailability::Degrade => OperationSupport::Degraded,
        })
    }
}

/// Provider selection plus its independently observed route state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationCapabilityProviderStateV1 {
    /// No compatible provider was selected by private configuration.
    Unbound,
    /// A compatible provider is selected; observed execution state remains inside its variant.
    Bound {
        /// Installation-local authorization required before provider invocation.
        approval: Approval,
        /// Provider-specific configuration and route observation.
        provider: RouteCapabilityProviderV1,
        /// Optional private explanation for the selected state.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
}

impl OperationCapabilityProviderStateV1 {
    /// Authorization floor carried only by a selected provider.
    #[must_use]
    pub fn approval(&self) -> Option<Approval> {
        match self {
            Self::Unbound => None,
            Self::Bound { approval, .. } => Some(*approval),
        }
    }

    /// Concrete provider kind, absent only for the unbound state.
    #[must_use]
    pub fn provider(&self) -> Option<CapabilityProvider> {
        match self {
            Self::Unbound => None,
            Self::Bound { provider, .. } => Some(provider.provider()),
        }
    }

    /// Latest execution-path-specific observation without inferring it from selection.
    #[must_use]
    pub fn execution_availability(&self) -> ProviderExecutionAvailability {
        match self {
            Self::Unbound => ProviderExecutionAvailability::Unknown,
            Self::Bound { provider, .. } => provider.execution_availability(),
        }
    }

    /// Exact host tools available to the route, when any.
    #[must_use]
    pub fn host_tools(&self) -> &[HostToolBinding] {
        match self {
            Self::Unbound => &[],
            Self::Bound { provider, .. } => provider.host_tools(),
        }
    }

    /// Native bounded-tool or connector target, when selected.
    #[must_use]
    pub fn target(&self) -> Option<&str> {
        match self {
            Self::Unbound => None,
            Self::Bound { provider, .. } => provider.target(),
        }
    }

    /// Compiled native action available on this route, when any.
    #[must_use]
    pub fn native_action(&self) -> Option<&str> {
        match self {
            Self::Unbound => None,
            Self::Bound { provider, .. } => provider.native_action(),
        }
    }

    /// Private note carried only by a selected provider.
    #[must_use]
    pub fn note(&self) -> Option<&str> {
        match self {
            Self::Unbound => None,
            Self::Bound { note, .. } => note.as_deref(),
        }
    }
}

/// Provider-specific route projection whose execution evidence cannot cross kinds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RouteCapabilityProviderV1 {
    /// A native harness tool supplies the capability.
    HostNative {
        /// Independently observed route availability.
        execution: HostedProviderExecutionV1,
    },
    /// A compiled Pale bounded tool supplies the capability.
    NativeRust {
        /// Exact bounded-tool identity selected by private configuration.
        target: String,
        /// Route observation including the compiled action when available.
        execution: NativeRustProviderExecutionV1,
    },
    /// A connector target and native host carrier jointly supply the capability.
    Connector {
        /// Exact connector-tool identity selected by private configuration.
        target: String,
        /// Route observation including exact carrier tools when available.
        execution: HostedProviderExecutionV1,
    },
}

impl RouteCapabilityProviderV1 {
    /// Closed provider vocabulary represented by this projection.
    #[must_use]
    pub fn provider(&self) -> CapabilityProvider {
        match self {
            Self::HostNative { .. } => CapabilityProvider::HostNative,
            Self::NativeRust { .. } => CapabilityProvider::NativeRust,
            Self::Connector { .. } => CapabilityProvider::Connector,
        }
    }

    /// Latest independently observed route availability.
    #[must_use]
    pub fn execution_availability(&self) -> ProviderExecutionAvailability {
        match self {
            Self::HostNative { execution } | Self::Connector { execution, .. } => {
                execution.availability()
            }
            Self::NativeRust { execution, .. } => execution.availability(),
        }
    }

    /// Exact tools carried only by an available route state.
    #[must_use]
    pub fn host_tools(&self) -> &[HostToolBinding] {
        match self {
            Self::HostNative { execution } | Self::Connector { execution, .. } => {
                execution.host_tools()
            }
            Self::NativeRust { execution, .. } => execution.host_tools(),
        }
    }

    /// Native bounded-tool or connector target selected by non-host providers.
    #[must_use]
    pub fn target(&self) -> Option<&str> {
        match self {
            Self::HostNative { .. } => None,
            Self::NativeRust { target, .. } | Self::Connector { target, .. } => Some(target),
        }
    }

    /// Compiled action carried only by an available native-Rust route.
    #[must_use]
    pub fn native_action(&self) -> Option<&str> {
        match self {
            Self::NativeRust { execution, .. } => execution.native_action(),
            Self::HostNative { .. } | Self::Connector { .. } => None,
        }
    }
}

/// Execution availability for a provider whose native carrier is a host tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HostedProviderExecutionV1 {
    /// No owner-native availability observation has been supplied.
    Unknown,
    /// The owner-native system explicitly reports the provider unavailable.
    Unavailable,
    /// Exact host tools are available on this route.
    Available {
        /// Non-empty route-local host carrier set.
        #[schemars(length(min = 1))]
        host_tools: Vec<HostToolBinding>,
    },
}

impl HostedProviderExecutionV1 {
    /// Closed availability vocabulary represented by this state.
    #[must_use]
    pub fn availability(&self) -> ProviderExecutionAvailability {
        match self {
            Self::Unknown => ProviderExecutionAvailability::Unknown,
            Self::Unavailable => ProviderExecutionAvailability::Unavailable,
            Self::Available { .. } => ProviderExecutionAvailability::Available,
        }
    }

    /// Exact tools carried only by the available state.
    #[must_use]
    pub fn host_tools(&self) -> &[HostToolBinding] {
        match self {
            Self::Available { host_tools } => host_tools,
            Self::Unknown | Self::Unavailable => &[],
        }
    }
}

/// Execution availability for a compiled native-Rust provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeRustProviderExecutionV1 {
    /// No owner-native availability observation has been supplied.
    Unknown,
    /// The Pale Runtime explicitly reports the compiled provider unavailable.
    Unavailable,
    /// The Pale Runtime exposes the exact compiled action through an observable host tool.
    Available {
        /// Non-empty route-local host carrier set.
        #[schemars(length(min = 1))]
        host_tools: Vec<HostToolBinding>,
        /// Exact compiled action identity behind the carrier.
        native_action: String,
    },
}

impl NativeRustProviderExecutionV1 {
    /// Closed availability vocabulary represented by this state.
    #[must_use]
    pub fn availability(&self) -> ProviderExecutionAvailability {
        match self {
            Self::Unknown => ProviderExecutionAvailability::Unknown,
            Self::Unavailable => ProviderExecutionAvailability::Unavailable,
            Self::Available { .. } => ProviderExecutionAvailability::Available,
        }
    }

    /// Exact tools carried only by the available state.
    #[must_use]
    pub fn host_tools(&self) -> &[HostToolBinding] {
        match self {
            Self::Available { host_tools, .. } => host_tools,
            Self::Unknown | Self::Unavailable => &[],
        }
    }

    /// Compiled action carried only by the available state.
    #[must_use]
    pub fn native_action(&self) -> Option<&str> {
        match self {
            Self::Available { native_action, .. } => Some(native_action),
            Self::Unknown | Self::Unavailable => None,
        }
    }
}

/// Kind of model-facing dependency transitively selected by an operation.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SemanticDependencyKind {
    /// Model-facing doctrine dependency.
    Doctrine,
    /// Specialist-role dependency.
    Role,
    /// Bounded-tool guidance dependency.
    Tool,
}

/// One exact prompt surface in an operation's dependency closure.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct OperationSemanticDependencyV1 {
    /// Semantic kind of the selected dependency.
    pub kind: SemanticDependencyKind,
    /// Canonical dependency identity.
    pub id: PackageResourceRefV1,
    /// Sealed rendered artifact carrying the dependency body.
    pub artifact: SealedArtifactRef,
}

/// Selection surface sealed for one bound operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationDispatchV1 {
    /// Model- and user-selectable operation with rendered discovery identities.
    ModelOrUser {
        /// Collision-resistant native-harness invocation name.
        invocation: String,
        /// Additional discovery aliases that do not create new operation identities.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        aliases: Vec<String>,
    },
    /// Explicit-user-only operation with rendered manual discovery identities.
    UserOnly {
        /// Collision-resistant native-harness invocation name.
        invocation: String,
        /// Additional manual discovery aliases.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        aliases: Vec<String>,
    },
    /// Runtime-only operation selected by its exact canonical operation id.
    Internal,
}

impl OperationDispatchV1 {
    /// Rendered invocation identity, absent for exact-id internal dispatch.
    #[must_use]
    pub fn invocation(&self) -> Option<&str> {
        match self {
            Self::ModelOrUser { invocation, .. } | Self::UserOnly { invocation, .. } => {
                Some(invocation)
            }
            Self::Internal => None,
        }
    }

    /// Discovery aliases, absent for internal dispatch.
    #[must_use]
    pub fn aliases(&self) -> &[String] {
        match self {
            Self::ModelOrUser { aliases, .. } | Self::UserOnly { aliases, .. } => aliases,
            Self::Internal => &[],
        }
    }

    /// Whether model-origin selection is permitted.
    #[must_use]
    pub fn model_selectable(&self) -> bool {
        matches!(self, Self::ModelOrUser { .. })
    }

    /// Whether an explicit user command may select this operation.
    #[must_use]
    pub fn user_selectable(&self) -> bool {
        !matches!(self, Self::Internal)
    }
}

/// Mechanical state of one bound operation contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationContractMechanicsV1 {
    /// Reasoning-only operation with no provider or effect grant.
    SemanticOnly,
    /// Operation with one or more route-resolved capability requirements.
    CapabilityBearing {
        /// Non-empty mechanical requirement set.
        #[schemars(length(min = 1))]
        capabilities: Vec<OperationCapabilityContractV1>,
    },
}

impl OperationContractMechanicsV1 {
    /// Requirements carried only by capability-bearing mechanics.
    #[must_use]
    pub fn capabilities(&self) -> &[OperationCapabilityContractV1] {
        match self {
            Self::SemanticOnly => &[],
            Self::CapabilityBearing { capabilities } => capabilities,
        }
    }

    /// Route support derived solely from the typed requirement states.
    #[must_use]
    pub fn support(&self) -> OperationSupport {
        self.capabilities()
            .iter()
            .filter_map(OperationCapabilityContractV1::unavailable_support)
            .fold(OperationSupport::Full, |support, requirement| {
                match (support, requirement) {
                    (OperationSupport::Blocked, _) | (_, OperationSupport::Blocked) => {
                        OperationSupport::Blocked
                    }
                    (OperationSupport::Degraded, _) | (_, OperationSupport::Degraded) => {
                        OperationSupport::Degraded
                    }
                    _ => OperationSupport::Full,
                }
            })
    }
}

/// One model-facing operation bound to its exact rendered skill, dependency
/// closure, and mechanical requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationContractV1 {
    /// Stable wire identity of the sealed operation contract.
    #[schemars(with = "String", extend("const" = "pale/operation-contract.v1"))]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Operation-contract schema major.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Canonical on-demand operation identity.
    pub id: PackageResourceRefV1,
    /// Closed selection policy; internal operations expose no rendered command.
    pub dispatch: OperationDispatchV1,
    /// Bound semantic-only or capability-bearing mechanical state.
    pub mechanics: OperationContractMechanicsV1,
    /// Sealed rendered operation procedure.
    pub skill: SealedArtifactRef,
    /// Optional sealed invocation-policy artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation_policy: Option<SealedArtifactRef>,
    /// Complete transitive prompt dependency closure.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_dependencies: Vec<OperationSemanticDependencyV1>,
    /// Additional host tools exposed to the operation without granting effects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auxiliary_tools: Vec<HostToolBinding>,
    /// Specialist-role identities available to the operation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<PackageResourceRefV1>,
    /// Canonical identity of the complete contract body.
    pub contract_digest: OperationContractDigest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationContractWireV1 {
    schema: SchemaId,
    schema_sha256: crate::records::SchemaDigest,
    schema_version: u32,
    id: PackageResourceRefV1,
    dispatch: OperationDispatchV1,
    mechanics: OperationContractMechanicsV1,
    skill: SealedArtifactRef,
    #[serde(default)]
    invocation_policy: Option<SealedArtifactRef>,
    #[serde(default)]
    semantic_dependencies: Vec<OperationSemanticDependencyV1>,
    #[serde(default)]
    auxiliary_tools: Vec<HostToolBinding>,
    #[serde(default)]
    roles: Vec<PackageResourceRefV1>,
    contract_digest: OperationContractDigest,
}

impl<'de> Deserialize<'de> for OperationContractV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = OperationContractWireV1::deserialize(deserializer)?;
        let value = Self {
            schema: wire.schema,
            schema_sha256: wire.schema_sha256,
            schema_version: wire.schema_version,
            id: wire.id,
            dispatch: wire.dispatch,
            mechanics: wire.mechanics,
            skill: wire.skill,
            invocation_policy: wire.invocation_policy,
            semantic_dependencies: wire.semantic_dependencies,
            auxiliary_tools: wire.auxiliary_tools,
            roles: wire.roles,
            contract_digest: wire.contract_digest,
        };
        value.verify().map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

/// Unsealed construction input for [`OperationContractV1`].
///
/// The application constructs this non-deserializable body, then calls
/// [`OperationContractV1::seal`] before exposing a portable contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationContractPreimageV1 {
    /// Stable wire identity of the sealed operation contract.
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Operation-contract schema major.
    pub schema_version: u32,
    /// Canonical on-demand operation identity.
    pub id: PackageResourceRefV1,
    /// Closed selection policy; internal operations expose no rendered command.
    pub dispatch: OperationDispatchV1,
    /// Bound semantic-only or capability-bearing mechanical state.
    pub mechanics: OperationContractMechanicsV1,
    /// Sealed rendered operation procedure.
    pub skill: SealedArtifactRef,
    /// Optional sealed invocation-policy artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation_policy: Option<SealedArtifactRef>,
    /// Complete transitive prompt dependency closure.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_dependencies: Vec<OperationSemanticDependencyV1>,
    /// Additional host tools exposed without granting effects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auxiliary_tools: Vec<HostToolBinding>,
    /// Specialist-role identities available to the operation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<PackageResourceRefV1>,
}

impl OperationContractV1 {
    /// Seal a complete operation-contract body with its canonical identity.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the preimage cannot satisfy
    /// the canonical JSON contract.
    pub fn seal(
        preimage: OperationContractPreimageV1,
    ) -> Result<Self, InvalidOperationContractFamilyV1> {
        validate_operation_contract(&preimage)?;
        let contract_digest = OperationContractDigest::hash_canonical(&preimage)?;
        let OperationContractPreimageV1 {
            schema,
            schema_sha256,
            schema_version,
            id,
            dispatch,
            mechanics,
            skill,
            invocation_policy,
            semantic_dependencies,
            auxiliary_tools,
            roles,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            schema_version,
            id,
            dispatch,
            mechanics,
            skill,
            invocation_policy,
            semantic_dependencies,
            auxiliary_tools,
            roles,
            contract_digest,
        })
    }

    /// Clone the unhashed contract body for explicit amendment and resealing.
    #[must_use]
    pub fn to_preimage(&self) -> OperationContractPreimageV1 {
        OperationContractPreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            schema_version: self.schema_version,
            id: self.id.clone(),
            dispatch: self.dispatch.clone(),
            mechanics: self.mechanics.clone(),
            skill: self.skill.clone(),
            invocation_policy: self.invocation_policy.clone(),
            semantic_dependencies: self.semantic_dependencies.clone(),
            auxiliary_tools: self.auxiliary_tools.clone(),
            roles: self.roles.clone(),
        }
    }

    /// Route-resolved requirements carried by capability-bearing mechanics.
    #[must_use]
    pub fn capabilities(&self) -> &[OperationCapabilityContractV1] {
        self.mechanics.capabilities()
    }

    /// Route support derived from the typed mechanical state.
    #[must_use]
    pub fn support(&self) -> OperationSupport {
        self.mechanics.support()
    }

    /// Recompute the canonical identity of this contract's unhashed body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the body cannot satisfy the
    /// canonical JSON contract.
    pub fn recompute_digest(&self) -> Result<OperationContractDigest, crate::canon::CanonError> {
        OperationContractDigest::hash_canonical(&self.to_preimage())
    }

    /// Revalidate a deserialized operation contract and its canonical identity.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidOperationContractFamilyV1`] for the same contract
    /// failure rejected by [`Self::seal`], or for a digest mismatch.
    pub fn verify(&self) -> Result<(), InvalidOperationContractFamilyV1> {
        let preimage = self.to_preimage();
        validate_operation_contract(&preimage)?;
        let observed = OperationContractDigest::hash_canonical(&preimage)?;
        if observed == self.contract_digest {
            Ok(())
        } else {
            Err(InvalidOperationContractFamilyV1::DigestMismatch {
                family: "operation contract",
            })
        }
    }
}

/// Compact lookup entry. The full mechanical contract lives in its own sealed
/// file and is loaded only after this entry is selected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationCatalogEntryV1 {
    /// Canonical operation identity.
    pub id: PackageResourceRefV1,
    /// Selection surface copied exactly from the full contract.
    pub dispatch: OperationDispatchV1,
    /// Compact mechanical state derived from the full contract.
    pub state: CatalogOperationStateV1,
    /// Sealed full mechanical and semantic operation contract.
    pub contract: SealedArtifactRef,
    /// Canonical identity of that full contract.
    pub contract_digest: OperationContractDigest,
}

/// Compact, mechanically coherent state retained by the lazy-load catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CatalogOperationStateV1 {
    /// Bound reasoning-only operation.
    SemanticOnly,
    /// Bound capability-bearing operation with its derived route support summary.
    CapabilityBearing {
        /// Support derived from the full contract's typed capabilities.
        support: OperationSupport,
    },
}

impl CatalogOperationStateV1 {
    /// Derive the compact lookup state from the owning full contract.
    #[must_use]
    pub fn from_contract(contract: &OperationContractV1) -> Self {
        match &contract.mechanics {
            OperationContractMechanicsV1::SemanticOnly => Self::SemanticOnly,
            OperationContractMechanicsV1::CapabilityBearing { .. } => Self::CapabilityBearing {
                support: contract.support(),
            },
        }
    }

    /// Route support represented by the compact state.
    #[must_use]
    pub fn support(self) -> OperationSupport {
        match self {
            Self::SemanticOnly => OperationSupport::Full,
            Self::CapabilityBearing { support } => support,
        }
    }
}

impl OperationCatalogEntryV1 {
    /// Build the compact lookup row from its sealed full contract.
    #[must_use]
    pub fn from_contract(contract: &OperationContractV1, artifact: SealedArtifactRef) -> Self {
        Self {
            id: contract.id.clone(),
            dispatch: contract.dispatch.clone(),
            state: CatalogOperationStateV1::from_contract(contract),
            contract: artifact,
            contract_digest: contract.contract_digest.clone(),
        }
    }
}

/// One repository's operation lookup catalog for one exact provider binding. Neutral
/// execution paths reuse this exact semantic body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationContractCatalogV1 {
    /// Stable wire identity of the operation lookup catalog.
    #[schemars(with = "String", extend("const" = "pale/operation-catalog.v1"))]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Operation-catalog schema major.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Portable profile whose selected operations produced this catalog.
    pub profile_digest: ProfileDigest,
    /// Logical repository receiving the catalog.
    pub repository_id: LogicalRepositoryIdV1,
    /// Exact terminal provider represented by this catalog.
    pub provider_binding: crate::records::ProviderBindingRefV1,
    /// Compact lookup rows in canonical operation order.
    pub entries: Vec<OperationCatalogEntryV1>,
    /// Canonical identity of the complete catalog body.
    pub catalog_digest: OperationCatalogDigest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationContractCatalogWireV1 {
    schema: SchemaId,
    schema_sha256: crate::records::SchemaDigest,
    schema_version: u32,
    profile_digest: ProfileDigest,
    repository_id: LogicalRepositoryIdV1,
    provider_binding: crate::records::ProviderBindingRefV1,
    entries: Vec<OperationCatalogEntryV1>,
    catalog_digest: OperationCatalogDigest,
}

impl<'de> Deserialize<'de> for OperationContractCatalogV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = OperationContractCatalogWireV1::deserialize(deserializer)?;
        let value = Self {
            schema: wire.schema,
            schema_sha256: wire.schema_sha256,
            schema_version: wire.schema_version,
            profile_digest: wire.profile_digest,
            repository_id: wire.repository_id,
            provider_binding: wire.provider_binding,
            entries: wire.entries,
            catalog_digest: wire.catalog_digest,
        };
        value.verify().map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

/// Unsealed construction input for [`OperationContractCatalogV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationContractCatalogPreimageV1 {
    /// Stable wire identity of the operation lookup catalog.
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Operation-catalog schema major.
    pub schema_version: u32,
    /// Portable profile whose selected operations produced this catalog.
    pub profile_digest: ProfileDigest,
    /// Logical repository receiving the catalog.
    pub repository_id: LogicalRepositoryIdV1,
    /// Exact terminal provider represented by this catalog.
    pub provider_binding: crate::records::ProviderBindingRefV1,
    /// Compact lookup rows in canonical operation order.
    pub entries: Vec<OperationCatalogEntryV1>,
}

impl OperationContractCatalogV1 {
    /// Seal a complete catalog body with its canonical identity.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the preimage cannot satisfy
    /// the canonical JSON contract.
    pub fn seal(
        preimage: OperationContractCatalogPreimageV1,
    ) -> Result<Self, InvalidOperationContractFamilyV1> {
        validate_operation_catalog(&preimage)?;
        let catalog_digest = OperationCatalogDigest::hash_canonical(&preimage)?;
        let OperationContractCatalogPreimageV1 {
            schema,
            schema_sha256,
            schema_version,
            profile_digest,
            repository_id,
            provider_binding,
            entries,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            schema_version,
            profile_digest,
            repository_id,
            provider_binding,
            entries,
            catalog_digest,
        })
    }

    /// Clone the unhashed catalog body for explicit amendment and resealing.
    #[must_use]
    pub fn to_preimage(&self) -> OperationContractCatalogPreimageV1 {
        OperationContractCatalogPreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            schema_version: self.schema_version,
            profile_digest: self.profile_digest.clone(),
            repository_id: self.repository_id.clone(),
            provider_binding: self.provider_binding.clone(),
            entries: self.entries.clone(),
        }
    }

    /// Recompute the canonical identity of this catalog's unhashed body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the body cannot satisfy the
    /// canonical JSON contract.
    pub fn recompute_digest(&self) -> Result<OperationCatalogDigest, crate::canon::CanonError> {
        OperationCatalogDigest::hash_canonical(&self.to_preimage())
    }

    /// Revalidate a deserialized operation catalog and its canonical identity.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidOperationContractFamilyV1`] for structural or digest
    /// disagreement.
    pub fn verify(&self) -> Result<(), InvalidOperationContractFamilyV1> {
        let preimage = self.to_preimage();
        validate_operation_catalog(&preimage)?;
        let observed = OperationCatalogDigest::hash_canonical(&preimage)?;
        if observed == self.catalog_digest {
            Ok(())
        } else {
            Err(InvalidOperationContractFamilyV1::DigestMismatch {
                family: "operation catalog",
            })
        }
    }
}

/// Small path-specific binding to a shared provider-bound catalog. The hook
/// context seals this record, preserving exact path identity without copying
/// every operation contract for each selected execution path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationRouteBindingV1 {
    /// Stable wire identity of the route binding.
    #[schemars(
        with = "String",
        extend("const" = "pale/operation-route-binding.v1")
    )]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Route-binding schema major.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Portable profile shared with the referenced catalog.
    pub profile_digest: ProfileDigest,
    /// Logical repository shared with the referenced catalog.
    pub repository_id: LogicalRepositoryIdV1,
    /// Compiler proof binding the selected neutral path to its composition plan.
    pub execution_path_witness: ExecutionPathWitnessV1,
    /// Sealed shared provider-bound operation catalog.
    pub catalog: SealedArtifactRef,
    /// Canonical identity of the referenced catalog.
    pub catalog_digest: OperationCatalogDigest,
    /// Canonical identity of the complete route-binding body.
    pub binding_digest: OperationBindingDigest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationRouteBindingWireV1 {
    schema: SchemaId,
    schema_sha256: crate::records::SchemaDigest,
    schema_version: u32,
    profile_digest: ProfileDigest,
    repository_id: LogicalRepositoryIdV1,
    execution_path_witness: ExecutionPathWitnessV1,
    catalog: SealedArtifactRef,
    catalog_digest: OperationCatalogDigest,
    binding_digest: OperationBindingDigest,
}

impl<'de> Deserialize<'de> for OperationRouteBindingV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = OperationRouteBindingWireV1::deserialize(deserializer)?;
        let value = Self {
            schema: wire.schema,
            schema_sha256: wire.schema_sha256,
            schema_version: wire.schema_version,
            profile_digest: wire.profile_digest,
            repository_id: wire.repository_id,
            execution_path_witness: wire.execution_path_witness,
            catalog: wire.catalog,
            catalog_digest: wire.catalog_digest,
            binding_digest: wire.binding_digest,
        };
        value.verify().map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

/// Unsealed construction input for [`OperationRouteBindingV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationRouteBindingPreimageV1 {
    /// Stable wire identity of the route binding.
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Route-binding schema major.
    pub schema_version: u32,
    /// Portable profile shared with the referenced catalog.
    pub profile_digest: ProfileDigest,
    /// Logical repository shared with the referenced catalog.
    pub repository_id: LogicalRepositoryIdV1,
    /// Compiler proof binding the selected neutral path to its composition plan.
    pub execution_path_witness: ExecutionPathWitnessV1,
    /// Sealed shared provider-bound operation catalog.
    pub catalog: SealedArtifactRef,
    /// Canonical identity of the referenced catalog.
    pub catalog_digest: OperationCatalogDigest,
}

impl OperationRouteBindingV1 {
    /// Seal a complete route-binding body with its canonical identity.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the preimage cannot satisfy
    /// the canonical JSON contract.
    pub fn seal(
        preimage: OperationRouteBindingPreimageV1,
    ) -> Result<Self, InvalidOperationContractFamilyV1> {
        validate_operation_route_binding(&preimage)?;
        let binding_digest = OperationBindingDigest::hash_canonical(&preimage)?;
        let OperationRouteBindingPreimageV1 {
            schema,
            schema_sha256,
            schema_version,
            profile_digest,
            repository_id,
            execution_path_witness,
            catalog,
            catalog_digest,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            schema_version,
            profile_digest,
            repository_id,
            execution_path_witness,
            catalog,
            catalog_digest,
            binding_digest,
        })
    }

    /// Clone the unhashed route-binding body for amendment and resealing.
    #[must_use]
    pub fn to_preimage(&self) -> OperationRouteBindingPreimageV1 {
        OperationRouteBindingPreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            schema_version: self.schema_version,
            profile_digest: self.profile_digest.clone(),
            repository_id: self.repository_id.clone(),
            execution_path_witness: self.execution_path_witness.clone(),
            catalog: self.catalog.clone(),
            catalog_digest: self.catalog_digest.clone(),
        }
    }

    /// Recompute the canonical identity of this binding's unhashed body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the body cannot satisfy the
    /// canonical JSON contract.
    pub fn recompute_digest(&self) -> Result<OperationBindingDigest, crate::canon::CanonError> {
        OperationBindingDigest::hash_canonical(&self.to_preimage())
    }

    /// Revalidate a deserialized route binding and its canonical identity.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidOperationContractFamilyV1`] for structural or digest
    /// disagreement.
    pub fn verify(&self) -> Result<(), InvalidOperationContractFamilyV1> {
        let preimage = self.to_preimage();
        validate_operation_route_binding(&preimage)?;
        let observed = OperationBindingDigest::hash_canonical(&preimage)?;
        if observed == self.binding_digest {
            Ok(())
        } else {
            Err(InvalidOperationContractFamilyV1::DigestMismatch {
                family: "operation route binding",
            })
        }
    }
}

fn validate_operation_contract(
    contract: &OperationContractPreimageV1,
) -> Result<(), InvalidOperationContractFamilyV1> {
    validate_schema_coordinate(
        &contract.schema,
        contract.schema_version,
        OPERATION_CONTRACT_SCHEMA,
        "operation contract",
    )?;
    if let OperationContractMechanicsV1::CapabilityBearing { capabilities } = &contract.mechanics {
        if capabilities.is_empty() {
            return Err(InvalidOperationContractFamilyV1::EmptyCapabilities);
        }
        if !capabilities
            .windows(2)
            .all(|pair| pair[0].requirement_id < pair[1].requirement_id)
        {
            return Err(InvalidOperationContractFamilyV1::NonCanonical {
                relation: "operation capability requirements",
            });
        }
    }
    validate_sorted(&contract.semantic_dependencies, "semantic dependencies")?;
    validate_sorted(&contract.auxiliary_tools, "auxiliary tools")?;
    validate_sorted(&contract.roles, "operation roles")?;
    validate_sorted(contract.dispatch.aliases(), "dispatch aliases")
}

fn validate_operation_catalog(
    catalog: &OperationContractCatalogPreimageV1,
) -> Result<(), InvalidOperationContractFamilyV1> {
    validate_schema_coordinate(
        &catalog.schema,
        catalog.schema_version,
        OPERATION_CATALOG_SCHEMA,
        "operation catalog",
    )?;
    if catalog
        .entries
        .windows(2)
        .all(|pair| pair[0].id < pair[1].id)
    {
        Ok(())
    } else {
        Err(InvalidOperationContractFamilyV1::NonCanonical {
            relation: "operation catalog entries",
        })
    }
}

fn validate_operation_route_binding(
    binding: &OperationRouteBindingPreimageV1,
) -> Result<(), InvalidOperationContractFamilyV1> {
    validate_schema_coordinate(
        &binding.schema,
        binding.schema_version,
        OPERATION_ROUTE_BINDING_SCHEMA,
        "operation route binding",
    )
}

fn validate_schema_coordinate(
    schema: &SchemaId,
    version: u32,
    expected: &str,
    family: &'static str,
) -> Result<(), InvalidOperationContractFamilyV1> {
    if schema.as_str() == expected && version == 1 {
        Ok(())
    } else {
        Err(InvalidOperationContractFamilyV1::SchemaCoordinate { family })
    }
}

fn validate_sorted<T: Ord>(
    values: &[T],
    relation: &'static str,
) -> Result<(), InvalidOperationContractFamilyV1> {
    if values.windows(2).all(|pair| pair[0] < pair[1]) {
        Ok(())
    } else {
        Err(InvalidOperationContractFamilyV1::NonCanonical { relation })
    }
}

#[cfg(test)]
#[path = "../../tests/internal/records/operation_contract.rs"]
mod tests;
