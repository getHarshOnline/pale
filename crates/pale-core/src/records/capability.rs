//! Package-owned capability procedure and private provider selections.
//!
//! [`CapabilityPolicy`], [`ToolRecipe`], and
//! [`OperationCapabilityRequirement`] are portable Package substance. Private
//! workspace records may narrow those ceilings, expose an operation, and bind a
//! concrete provider, but cannot restate the Package procedure or declare it
//! executable. A [`CapabilityBinding`] proves only configuration; neither a
//! Package declaration nor a private binding can fabricate
//! [`crate::records::ProviderExecutionAvailability`] or native evidence.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::records::{
    Approval, CapabilityProvider, CapabilityUnavailability, EffectClass, FailurePosture,
    HostToolBinding, NonRootJsonPointer, ProviderBindingRefV1, Requirement, SchemaId,
};
use crate::records::{
    InvalidPackageIdV1, PackageResourceLinkV1, PackageResourceRefV1, PackageResourceSelectorV1,
    ProfileDigest,
};

/// Maximum members on one capability or effect axis in initial-v1 policy.
///
/// The bound matches the 4,096-resource Package scale. It is shared by
/// Package-owned [`CapabilityPolicy`] and private [`CapabilityNarrowingV1`], so
/// meeting the two records never introduces an unbounded policy inventory.
pub const MAX_CAPABILITY_POLICY_AXIS_MEMBERS: usize = 4_096;

/// Portable capability identity shared by Package procedure and private binding.
///
/// The lexical grammar matches Package-owned semantic identities while the
/// type remains non-substitutable with Package or resource identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct CapabilityIdV1(
    #[schemars(
        length(min = 1, max = 255),
        regex(pattern = r"^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)*$")
    )]
    String,
);

impl CapabilityIdV1 {
    /// Validate and own one portable capability identity.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidPackageIdV1`] when `value` is empty, exceeds
    /// [`crate::records::MAX_PACKAGE_ID_BYTES`], or violates the lowercase
    /// Package-identity grammar.
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidPackageIdV1> {
        let value = value.into();
        crate::records::package::package_identifier_is_valid(&value)
            .then_some(Self(value))
            .ok_or(InvalidPackageIdV1)
    }

    /// Borrow the validated wire representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CapabilityIdV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for CapabilityIdV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A capability-procedure identity violates the bounded initial-v1 grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("identity must be lowercase Package-style text of at most 255 bytes")]
pub struct InvalidCapabilityProcedureIdV1;

macro_rules! capability_procedure_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
        #[serde(transparent)]
        #[schemars(transparent)]
        pub struct $name(
            #[schemars(
                length(min = 1, max = 255),
                regex(pattern = r"^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)*$")
            )]
            String,
        );

        impl $name {
            /// Validate and own one bounded initial-v1 identity.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidCapabilityProcedureIdV1`] when `value` violates
            /// the shared lowercase Package-style identity grammar.
            pub fn parse(value: impl Into<String>) -> Result<Self, InvalidCapabilityProcedureIdV1> {
                let value = value.into();
                crate::records::package::package_identifier_is_valid(&value)
                    .then_some(Self(value))
                    .ok_or(InvalidCapabilityProcedureIdV1)
            }

            /// Borrow the validated wire representation.
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
                formatter.write_str(self.as_str())
            }
        }
    };
}

capability_procedure_id!(
    /// Operation-local identity of one Package-owned capability requirement.
    CapabilityRequirementIdV1
);
capability_procedure_id!(
    /// Provider-neutral tool-contract identity used by recipes and bindings.
    ToolContractIdV1
);
capability_procedure_id!(
    /// Operation identity within a provider-neutral tool contract.
    ToolOperationIdV1
);
capability_procedure_id!(
    /// Strongest conclusion supported by one recipe's evidence contract.
    ProofCeilingIdV1
);
capability_procedure_id!(
    /// Stable semantic identity of one explicitly reduced operation outcome.
    DegradedOutcomeIdV1
);

/// Portable Package-owned ceiling for capability and effect use.
///
/// Allow and deny sets are independent typed boundaries rather than a numeric
/// effect ranking. A private [`CapabilityNarrowingV1`] may meet this ceiling but
/// cannot widen it. Presence of this record grants no provider or native effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityPolicy {
    /// Capability identities that Package operations may require.
    #[schemars(length(max = 4_096), extend("uniqueItems" = true))]
    pub permitted_capabilities: Vec<CapabilityIdV1>,
    /// Capability identities that no occurrence may grant.
    #[serde(default)]
    #[schemars(length(max = 4_096), extend("uniqueItems" = true))]
    pub forbidden_capabilities: Vec<CapabilityIdV1>,
    /// Effect classes permitted by this Package ceiling.
    #[schemars(length(max = 4_096), extend("uniqueItems" = true))]
    pub permitted_effects: Vec<EffectClass>,
    /// Effect classes explicitly prohibited by this Package ceiling.
    #[serde(default)]
    #[schemars(length(max = 4_096), extend("uniqueItems" = true))]
    pub forbidden_effects: Vec<EffectClass>,
}

/// Private narrowing applied to selected Package capability procedure.
///
/// The compiler forms an intersection with Package-owned [`CapabilityPolicy`]
/// values. These fields can remove capability/effect choices or add denials;
/// they cannot make an unpermitted Package requirement available.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityNarrowingV1 {
    /// Capabilities retained from the Package-owned permitted set.
    #[schemars(length(max = 4_096), extend("uniqueItems" = true))]
    pub permitted_capabilities: Vec<CapabilityIdV1>,
    /// Additional capabilities denied by private configuration.
    #[serde(default)]
    #[schemars(length(max = 4_096), extend("uniqueItems" = true))]
    pub forbidden_capabilities: Vec<CapabilityIdV1>,
    /// Effects retained from the Package-owned permitted set.
    #[schemars(length(max = 4_096), extend("uniqueItems" = true))]
    pub permitted_effects: Vec<EffectClass>,
    /// Additional effects denied by private configuration.
    #[serde(default)]
    #[schemars(length(max = 4_096), extend("uniqueItems" = true))]
    pub forbidden_effects: Vec<EffectClass>,
}

/// Evidence shape and proof ceiling required by a Package-owned [`ToolRecipe`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolEvidenceContract {
    /// Required response fields expressed as JSON Pointers.
    pub required_fields: Vec<NonRootJsonPointer>,
    /// Stable identity of the strongest conclusion these fields can support.
    pub proof_ceiling: ProofCeilingIdV1,
    /// Whether exact successful evidence may refresh repository verification.
    #[serde(default)]
    pub repository_verifier: bool,
}

/// Reusable provider-neutral call recipe owned by one Package resource.
///
/// [`crate::records::PackageResourceV1::id`] supplies the recipe identity, so
/// this body deliberately carries no second `id` field. It names no endpoint,
/// machine path, credential, selected provider, or execution availability. Those
/// values remain in [`CapabilityBinding`] and owner-native observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolRecipe {
    /// Compact explanation of the mechanical check.
    pub description: String,
    /// Portable capability identity provided by the recipe.
    pub capability: CapabilityIdV1,
    /// Stable provider-neutral tool contract.
    pub tool_contract: ToolContractIdV1,
    /// Operation within [`Self::tool_contract`] to call.
    pub tool_operation: ToolOperationIdV1,
    /// Maximum effect permitted for the call.
    pub effect_class: EffectClass,
    /// Provider argument to operation-input expression.
    pub input_mapping: BTreeMap<String, String>,
    /// Evidence required before the recipe's conclusion is available.
    pub evidence: ToolEvidenceContract,
}

/// One Package-owned capability requirement attached to an operation.
///
/// The requirement describes portable procedure, evidence, fallback, and
/// forbidden-effect meaning. A private [`OperationBinding`] only exposes its
/// owning operation; provider selection remains in [`CapabilityBinding`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationCapabilityRequirement {
    /// Operation-local requirement identity.
    pub id: CapabilityRequirementIdV1,
    /// Compact explanation shown in the effective operation contract.
    pub description: String,
    /// Portable capability identity required by this binding.
    pub capability: CapabilityIdV1,
    /// Local or dependency-qualified Package recipe source link, when selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<PackageResourceLinkV1>,
    /// Maximum effect permitted for satisfying the requirement.
    pub effect_class: EffectClass,
    /// Closed unavailable-provider policy for this requirement.
    pub policy: CapabilityRequirementPolicy,
    /// Posture after an admitted provider invocation fails.
    pub failure_posture: FailurePosture,
}

impl OperationCapabilityRequirement {
    /// Whether the operation's full declared outcome depends on this requirement.
    #[must_use]
    pub fn requirement(&self) -> Requirement {
        self.policy.requirement()
    }

    /// Declared handling when no usable provider observation exists.
    #[must_use]
    pub fn on_unavailable(&self) -> CapabilityUnavailability {
        self.policy.on_unavailable()
    }

    /// Model-facing guidance for the admitted unavailable path, when one exists.
    #[must_use]
    pub fn fallback(&self) -> Option<&str> {
        self.policy
            .fallback()
            .map(|fallback| fallback.guidance.as_str())
    }

    /// Package-owned recipe selected for the unavailable path, when one exists.
    #[must_use]
    pub fn fallback_recipe(&self) -> Option<&PackageResourceLinkV1> {
        self.policy
            .fallback()
            .and_then(|fallback| fallback.recipe.as_ref())
    }

    /// Stable reduced-outcome identity admitted by required degradation.
    #[must_use]
    pub fn degraded_outcome(&self) -> Option<&DegradedOutcomeIdV1> {
        self.policy.degraded_outcome()
    }
}

/// Guidance and optional recipe for one explicitly admitted fallback path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityFallback {
    /// Model-facing guidance that explains the bounded fallback.
    #[schemars(length(min = 1))]
    pub guidance: String,
    /// Optional local or dependency-qualified Package recipe source link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<PackageResourceLinkV1>,
}

/// The three legal unavailable-provider contracts for one requirement.
///
/// Keeping these states distinct prevents optional requirements from blocking,
/// prevents blocking requirements from carrying fallback data, and makes a
/// required degradation's reduced outcome mandatory in the v1 wire shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CapabilityRequirementPolicy {
    /// The full result is unavailable when the required capability is unusable.
    RequiredBlock,
    /// A required capability admits one explicitly named reduced outcome.
    RequiredDegrade {
        /// Guidance and optional recipe for the degraded path.
        fallback: CapabilityFallback,
        /// Stable identity of the reduced conclusion.
        #[schemars(length(min = 1))]
        degraded_outcome: DegradedOutcomeIdV1,
    },
    /// An optional capability may be omitted without lowering the core result.
    Optional {
        /// Guidance and optional recipe for the omitted-capability path.
        fallback: CapabilityFallback,
    },
}

/// Qualified fallback carried after Package semantic admission.
///
/// The source [`CapabilityFallback::recipe`] is an authored
/// [`PackageResourceLinkV1`]. Package admission must prove and qualify it before
/// constructing this generated form; no source shorthand survives here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedCapabilityFallbackV1 {
    /// Model-facing bounded fallback guidance.
    #[schemars(length(min = 1))]
    pub guidance: String,
    /// Qualified admitted Package recipe, when selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<PackageResourceRefV1>,
}

/// Qualified unavailable-provider policy in generated contracts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResolvedCapabilityRequirementPolicyV1 {
    /// The full result is unavailable without a usable required provider.
    RequiredBlock,
    /// Required capability admits one named reduced outcome.
    RequiredDegrade {
        /// Qualified fallback contract.
        fallback: ResolvedCapabilityFallbackV1,
        /// Stable identity of the reduced conclusion.
        #[schemars(length(min = 1))]
        degraded_outcome: DegradedOutcomeIdV1,
    },
    /// Optional capability may be omitted without lowering the core result.
    Optional {
        /// Qualified omitted-capability fallback.
        fallback: ResolvedCapabilityFallbackV1,
    },
}

impl ResolvedCapabilityRequirementPolicyV1 {
    /// Whether the full operation result depends on the capability.
    #[must_use]
    pub fn requirement(&self) -> Requirement {
        match self {
            Self::RequiredBlock | Self::RequiredDegrade { .. } => Requirement::Required,
            Self::Optional { .. } => Requirement::Optional,
        }
    }

    /// Unavailable-provider behavior encoded by this admitted policy.
    #[must_use]
    pub fn on_unavailable(&self) -> CapabilityUnavailability {
        match self {
            Self::RequiredBlock => CapabilityUnavailability::Block,
            Self::RequiredDegrade { .. } | Self::Optional { .. } => {
                CapabilityUnavailability::Degrade
            }
        }
    }

    /// Qualified fallback carried by degradable or optional states.
    #[must_use]
    pub fn fallback(&self) -> Option<&ResolvedCapabilityFallbackV1> {
        match self {
            Self::RequiredBlock => None,
            Self::RequiredDegrade { fallback, .. } | Self::Optional { fallback } => Some(fallback),
        }
    }

    /// Stable reduced-outcome identity admitted by required degradation.
    #[must_use]
    pub fn degraded_outcome(&self) -> Option<&DegradedOutcomeIdV1> {
        match self {
            Self::RequiredDegrade {
                degraded_outcome, ..
            } => Some(degraded_outcome),
            Self::RequiredBlock | Self::Optional { .. } => None,
        }
    }
}

impl CapabilityRequirementPolicy {
    /// Whether the operation's full result depends on the capability.
    #[must_use]
    pub fn requirement(&self) -> Requirement {
        match self {
            Self::RequiredBlock | Self::RequiredDegrade { .. } => Requirement::Required,
            Self::Optional { .. } => Requirement::Optional,
        }
    }

    /// Unavailable-provider behavior encoded by this legal state.
    #[must_use]
    pub fn on_unavailable(&self) -> CapabilityUnavailability {
        match self {
            Self::RequiredBlock => CapabilityUnavailability::Block,
            Self::RequiredDegrade { .. } | Self::Optional { .. } => {
                CapabilityUnavailability::Degrade
            }
        }
    }

    /// Fallback contract carried by degradable or optional states.
    #[must_use]
    pub fn fallback(&self) -> Option<&CapabilityFallback> {
        match self {
            Self::RequiredBlock => None,
            Self::RequiredDegrade { fallback, .. } | Self::Optional { fallback } => Some(fallback),
        }
    }

    /// Stable reduced-outcome identity carried only by required degradation.
    #[must_use]
    pub fn degraded_outcome(&self) -> Option<&DegradedOutcomeIdV1> {
        match self {
            Self::RequiredDegrade {
                degraded_outcome, ..
            } => Some(degraded_outcome),
            Self::RequiredBlock | Self::Optional { .. } => None,
        }
    }
}

/// Who may select a privately bound operation before effect authorization.
///
/// This is workspace configuration, not Package meaning. A selectable
/// operation may still require provider-specific approval at activation.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationSelection {
    /// The model or the user may select the operation through rendered discovery.
    ModelOrUser {
        /// Additional discovery names; none creates another operation identity.
        #[serde(default)]
        aliases: Vec<String>,
    },
    /// Only an explicit user request may select the rendered operation.
    UserOnly {
        /// Additional user-facing discovery names.
        #[serde(default)]
        aliases: Vec<String>,
    },
    /// Runtime machinery may select the exact operation id; no command is rendered.
    Internal,
}

impl OperationSelection {
    /// Discovery aliases carried only by user- or model-visible selections.
    #[must_use]
    pub fn aliases(&self) -> &[String] {
        match self {
            Self::ModelOrUser { aliases } | Self::UserOnly { aliases } => aliases,
            Self::Internal => &[],
        }
    }

    /// Mutable discovery aliases used by deterministic profile normalization.
    #[must_use]
    pub fn aliases_mut(&mut self) -> Option<&mut Vec<String>> {
        match self {
            Self::ModelOrUser { aliases } | Self::UserOnly { aliases } => Some(aliases),
            Self::Internal => None,
        }
    }

    /// Whether model-origin selection is permitted.
    #[must_use]
    pub fn model_selectable(&self) -> bool {
        matches!(self, Self::ModelOrUser { .. })
    }

    /// Whether this selection may be rendered as a user-facing command.
    #[must_use]
    pub fn user_selectable(&self) -> bool {
        !matches!(self, Self::Internal)
    }
}

/// Private exposure policy for one semantic operation.
///
/// Absence of this record means unbound: the operation remains in the semantic
/// graph but no host skill, activation lease, provider, or effect is emitted.
/// A present binding carries exposure only. Whether the bound operation is
/// semantic-only or capability-bearing is derived from the Package-owned
/// operation requirement set; private configuration cannot assert it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationBinding {
    /// Who may select the operation.
    pub selection: OperationSelection,
}

/// Owner-qualified private mechanics for one selected semantic operation.
///
/// The Package resource reference identifies meaning; [`Self::binding`] owns
/// only private exposure and capability mechanics. Presence of this entry is
/// distinct from Package selection, observed execution availability, effect
/// admission, and native success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationBindingEntryV1 {
    /// Exact Package-owned operation selected for private binding.
    pub operation: PackageResourceRefV1,
    /// Private exposure mechanics for that operation.
    pub binding: OperationBinding,
}

impl OperationBinding {
    /// Selection policy for this bound operation.
    #[must_use]
    pub fn selection(&self) -> &OperationSelection {
        &self.selection
    }

    /// Mutable selection policy used by deterministic profile normalization.
    #[must_use]
    pub fn selection_mut(&mut self) -> &mut OperationSelection {
        &mut self.selection
    }
}

/// One project-local selection of a concrete provider for a portable capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityBinding {
    /// Exact effect supplied by this private provider selection.
    pub effect_class: EffectClass,
    /// Installation-local authorization required before invocation.
    pub approval: Approval,
    /// Tool contract implemented by the selected provider, when recipe-bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_contract: Option<ToolContractIdV1>,
    /// Provider-specific configuration whose shape is valid by construction.
    pub implementation: CapabilityProviderBinding,
    /// Optional private explanation for the provider selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl CapabilityBinding {
    /// Concrete provider selected by this private configuration.
    #[must_use]
    pub fn provider(&self) -> CapabilityProvider {
        self.implementation.provider()
    }

    /// Provider-bound tool rows carried by host-native and connector providers.
    #[must_use]
    pub fn provider_tools(&self) -> Option<&[ProviderToolBindingV1]> {
        self.implementation.provider_tools()
    }

    /// Mutable provider-bound tool rows used during repository-local expansion.
    #[must_use]
    pub fn provider_tools_mut(&mut self) -> Option<&mut Vec<ProviderToolBindingV1>> {
        self.implementation.provider_tools_mut()
    }

    /// Native bounded-tool or connector target, when selected by the provider kind.
    #[must_use]
    pub fn target(&self) -> Option<&str> {
        self.implementation.target()
    }
}

/// Tool bindings supplied through one exact terminal provider binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderToolBindingV1 {
    /// Exact provider binding whose adapter exposes these tools.
    pub provider_binding: ProviderBindingRefV1,
    /// Non-empty tool inventory owned by that provider adapter.
    #[schemars(length(min = 1), extend("uniqueItems" = true))]
    pub tools: Vec<HostToolBinding>,
}

/// Provider-specific private configuration for one portable capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CapabilityProviderBinding {
    /// Exact provider tools implement the capability.
    HostNative {
        /// Exact provider bindings and their non-empty admitted tool rows.
        provider_tools: Vec<ProviderToolBindingV1>,
    },
    /// One compiled Pale bounded tool implements the capability.
    NativeRust {
        /// Exact bounded-tool identity.
        target: String,
    },
    /// One connector plus exact host carriers implements the capability.
    Connector {
        /// Exact bounded connector-tool identity.
        target: String,
        /// Provider-bound carriers that can invoke the connector.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        provider_tools: Vec<ProviderToolBindingV1>,
    },
}

impl CapabilityProviderBinding {
    /// Closed provider vocabulary represented by this shape.
    #[must_use]
    pub fn provider(&self) -> CapabilityProvider {
        match self {
            Self::HostNative { .. } => CapabilityProvider::HostNative,
            Self::NativeRust { .. } => CapabilityProvider::NativeRust,
            Self::Connector { .. } => CapabilityProvider::Connector,
        }
    }

    /// Provider-bound tools selected by kinds that require a host carrier.
    #[must_use]
    pub fn provider_tools(&self) -> Option<&[ProviderToolBindingV1]> {
        match self {
            Self::HostNative { provider_tools } | Self::Connector { provider_tools, .. } => {
                Some(provider_tools)
            }
            Self::NativeRust { .. } => None,
        }
    }

    fn provider_tools_mut(&mut self) -> Option<&mut Vec<ProviderToolBindingV1>> {
        match self {
            Self::HostNative { provider_tools } | Self::Connector { provider_tools, .. } => {
                Some(provider_tools)
            }
            Self::NativeRust { .. } => None,
        }
    }

    /// Native bounded-tool or connector target carried by this provider kind.
    #[must_use]
    pub fn target(&self) -> Option<&str> {
        match self {
            Self::HostNative { .. } => None,
            Self::NativeRust { target } | Self::Connector { target, .. } => Some(target),
        }
    }
}

/// Why a selected semantic node carries a capability contract.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityUseKind {
    /// A capability required by one on-demand operation.
    OperationRequirement,
}

/// One normalized provider binding used by the selected semantic graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedCapabilityProviderV1 {
    /// Portable capability identity supplied by this binding.
    pub capability_id: CapabilityIdV1,
    /// Exact effect selected for the provider.
    pub effect_class: EffectClass,
    /// Installation-local authorization floor.
    pub approval: Approval,
    /// Provider-neutral tool contract implemented by the selection.
    pub tool_contract: Option<ToolContractIdV1>,
    /// Provider-specific project-local selection after repository expansion.
    pub implementation: CapabilityProviderBinding,
    /// Optional project explanation for the resolved state.
    pub note: Option<String>,
}

/// One operation requirement, retaining its context-specific contract instead
/// of collapsing it into the provider row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedCapabilityUseV1 {
    /// Semantic node class that owns this use.
    pub kind: CapabilityUseKind,
    /// Operation or other semantic-node identity that owns the requirement.
    pub owner: PackageResourceRefV1,
    /// Original requirement identity declared by the Package operation.
    pub requirement_id: CapabilityRequirementIdV1,
    /// Portable capability identity required by the operation.
    pub capability_id: CapabilityIdV1,
    /// Owner-qualified Package recipe selected by the operation requirement.
    pub recipe: Option<PackageResourceRefV1>,
    /// Effect expected by the requirement.
    pub effect_class: EffectClass,
    /// Closed unavailable-provider policy retained from Package procedure.
    pub policy: ResolvedCapabilityRequirementPolicyV1,
    /// Recovery posture when the admitted mechanic fails.
    pub failure_posture: FailurePosture,
}

/// Machine-readable capability projection emitted beside the agent surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityBindingsV1 {
    /// Stable wire identity of the capability projection.
    #[schemars(
        with = "String",
        extend("const" = "pale.agent-surface.capability-bindings.v1")
    )]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Capability-projection schema major.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Portable profile whose selected graph produced this projection.
    pub profile_digest: ProfileDigest,
    /// Owner-qualified Package recipes retained with their portable procedure.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recipes: Vec<ResolvedToolRecipeV1>,
    /// Resolved provider rows keyed semantically by `capability_id`.
    pub providers: Vec<ResolvedCapabilityProviderV1>,
    /// Context-specific requirement rows retained beside those providers.
    pub requirements: Vec<ResolvedCapabilityUseV1>,
}

/// Owner-qualified occurrence seeds for Package-owned semantic resources.
///
/// Package-semantic fields supply one resource kind, so
/// [`PackageResourceSelectorV1`] needs only the Package owner and exact ID or
/// Package-scoped wildcard. There is no distribution-global wildcard.
/// Semantic references and Package dependencies close transitively during
/// compilation rather than being copied into this private selection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SemanticSelection {
    /// Exact operations or Package-scoped operation wildcards.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub operations: Vec<PackageResourceSelectorV1>,
    /// Exact specialist roles or Package-scoped role wildcards.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub roles: Vec<PackageResourceSelectorV1>,
    /// Exact doctrine or Package-scoped doctrine wildcards.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub doctrine: Vec<PackageResourceSelectorV1>,
    /// Exact run modes or Package-scoped mode wildcards.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub run_modes: Vec<PackageResourceSelectorV1>,
    /// Optional execution-workspace overrides for selected runtime-scoped mode defaults. Omitted axes use
    /// the distribution fallback; session-only modes are rejected during projection.
    #[serde(default)]
    pub mode_defaults: BTreeMap<crate::records::ModeAxis, PackageResourceRefV1>,
    /// Exact stack overlays or Package-scoped stack wildcards.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub stacks: Vec<PackageResourceSelectorV1>,
}

/// One admitted Package recipe with its exact semantic owner.
///
/// Retaining only a reference would force a projection consumer to reload
/// Package source and would lose the non-copying compiled boundary. The body is
/// carried once here, while requirement rows refer to [`Self::resource`] rather
/// than duplicating procedure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedToolRecipeV1 {
    /// Owner-qualified Package recipe identity.
    pub resource: PackageResourceRefV1,
    /// Provider-neutral procedure admitted from the owning Package resource.
    pub procedure: ToolRecipe,
}

#[cfg(test)]
#[path = "../../tests/internal/records/capability.rs"]
mod tests;
