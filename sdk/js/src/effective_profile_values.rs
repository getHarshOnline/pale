//! Selections, path reasons, and findings retained in effective-profile evidence.
//!
//! These values copy the Rust SDK's admitted classifications. A configured
//! provider or exposure does not establish native readiness or effect authority.

use napi_derive::napi;
use pale_sdk::{
    EffectiveProfileDeclaredSelection as ApiDeclaredSelection,
    EffectiveProfileDeclaredSelector as ApiDeclaredSelector, EffectiveProfileFinding as ApiFinding,
    EffectiveProfileModeAxis as ApiModeAxis,
    EffectiveProfileNativeRealization as ApiNativeRealization,
    EffectiveProfilePathReason as ApiPathReason,
    EffectiveProfileProviderDisposition as ApiProviderDisposition,
    EffectiveProfileProviderKind as ApiProviderKind,
    EffectiveProfileResourceKind as ApiResourceKind, EffectiveProfileSelection as ApiSelection,
};

use crate::RequirementDisposition;

/// Package semantic family selected by an admitted profile.
#[napi]
pub enum EffectiveProfileResourceKind {
    /// Operation procedure.
    Operation,
    /// Specialist role.
    Role,
    /// Standalone doctrine.
    Doctrine,
    /// Doctrine selected for startup policy.
    StartupPolicy,
    /// Run-mode declaration.
    RunMode,
    /// Technology or workflow stack.
    Stack,
    /// Policy-family body.
    PolicyFamily,
    /// Reusable Package recipe.
    Recipe,
}

impl EffectiveProfileResourceKind {
    fn from_api(value: ApiResourceKind) -> Self {
        match value {
            ApiResourceKind::Operation => Self::Operation,
            ApiResourceKind::Role => Self::Role,
            ApiResourceKind::Doctrine => Self::Doctrine,
            ApiResourceKind::StartupPolicy => Self::StartupPolicy,
            ApiResourceKind::RunMode => Self::RunMode,
            ApiResourceKind::Stack => Self::Stack,
            ApiResourceKind::PolicyFamily => Self::PolicyFamily,
            ApiResourceKind::Recipe => Self::Recipe,
        }
    }
}

/// Independent mode dimension selected by a declared default.
#[napi]
pub enum EffectiveProfileModeAxis {
    /// Amount of process and verification guidance.
    Intensity,
    /// Specialist task stance.
    TaskStance,
}

impl EffectiveProfileModeAxis {
    fn from_api(value: ApiModeAxis) -> Self {
        match value {
            ApiModeAxis::Intensity => Self::Intensity,
            ApiModeAxis::TaskStance => Self::TaskStance,
        }
    }
}

/// Configured provider family, not a readiness observation.
#[napi]
pub enum EffectiveProfileProviderKind {
    /// Native-harness tool binding.
    HostNative,
    /// Bounded Pale Rust mechanic.
    NativeRust,
    /// Admitted connector binding.
    Connector,
}

impl EffectiveProfileProviderKind {
    fn from_api(value: ApiProviderKind) -> Self {
        match value {
            ApiProviderKind::HostNative => Self::HostNative,
            ApiProviderKind::NativeRust => Self::NativeRust,
            ApiProviderKind::Connector => Self::Connector,
        }
    }
}

/// Native evidence admitted by the effective-profile owner.
#[napi]
pub enum EffectiveProfileNativeRealization {
    /// No owner-native observation has been admitted.
    Unknown,
}

impl EffectiveProfileNativeRealization {
    pub(crate) fn from_api(value: ApiNativeRealization) -> Self {
        match value {
            ApiNativeRealization::Unknown => Self::Unknown,
        }
    }
}

/// Copied declared selector before compiler closure expansion.
/// Each literal kind exposes only its required coordinate.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum EffectiveProfileDeclaredSelector {
    /// One owner-qualified resource.
    Exact {
        /// Exact declared resource.
        resource_id: String,
    },
    /// Every exported resource of the declared family from one Package.
    AllFromPackage {
        /// Package scope of the declaration.
        package_id: String,
    },
}

impl EffectiveProfileDeclaredSelector {
    fn from_api(value: &ApiDeclaredSelector) -> Self {
        match value {
            ApiDeclaredSelector::Exact { resource_id } => Self::Exact {
                resource_id: resource_id.clone(),
            },
            ApiDeclaredSelector::AllFromPackage { package_id } => Self::AllFromPackage {
                package_id: package_id.clone(),
            },
        }
    }
}

/// Copied admitted declaration, distinct from its expanded effective selection.
/// Required variant fields do not admit sources or authorize effects.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum EffectiveProfileDeclaredSelection {
    /// Exact or Package-scoped resource declaration.
    Resource {
        /// Semantic family selected by the enclosing declaration.
        resource_kind: EffectiveProfileResourceKind,
        /// Unexpanded declared selector.
        selector: EffectiveProfileDeclaredSelector,
    },
    /// Explicit default for one runtime-mode dimension.
    ModeDefault {
        /// Orthogonal mode dimension.
        axis: EffectiveProfileModeAxis,
        /// Exact declared run-mode resource.
        resource_id: String,
    },
}

impl EffectiveProfileDeclaredSelection {
    pub(crate) fn from_api(value: &ApiDeclaredSelection) -> Self {
        match value {
            ApiDeclaredSelection::Resource {
                resource_kind,
                selector,
            } => Self::Resource {
                resource_kind: EffectiveProfileResourceKind::from_api(*resource_kind),
                selector: EffectiveProfileDeclaredSelector::from_api(selector),
            },
            ApiDeclaredSelection::ModeDefault { axis, resource_id } => Self::ModeDefault {
                axis: EffectiveProfileModeAxis::from_api(*axis),
                resource_id: resource_id.clone(),
            },
        }
    }
}

/// Exact compiler-selected coordinate, narrowed by its literal `kind`.
///
/// Every field belongs to its selected variant and is required. These copied
/// outputs describe configuration; they do not admit sources or authorize effects.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum EffectiveProfileSelection {
    /// Immutable Package release.
    Package {
        /// Package identity.
        package_id: String,
        /// Selected Package version.
        version: String,
        /// Exact Package release digest.
        content_digest: String,
    },
    /// Attestor-scoped normative Pack release.
    NormativePack {
        /// Normative Pack identity.
        pack_id: String,
        /// Attestor owning the release statement.
        attestor_id: String,
        /// Selected Pack version.
        version: String,
        /// Exact Pack release digest.
        content_digest: String,
    },
    /// Package resource selected for one repository.
    SemanticResource {
        /// Logical repository receiving the resource.
        repository_id: String,
        /// Selected semantic resource family.
        resource_kind: EffectiveProfileResourceKind,
        /// Owner-qualified resource identity.
        resource_id: String,
    },
    /// Selected private run policy.
    RunPolicy {
        /// Exact run-policy identity.
        run_policy_id: String,
    },
    /// Selected session model and bounded cardinalities.
    Session {
        /// Execution workspace owning the selection.
        execution_workspace_id: String,
        /// Activation-root count, including zero.
        activation_root_count: u32,
        /// Additional-root count, including zero.
        additional_root_count: u32,
        /// Activation-path count, including zero.
        activation_path_count: u32,
    },
    /// One admitted lifecycle behavior.
    Lifecycle {
        /// Lifecycle behavior identity.
        lifecycle_id: String,
        /// Lifecycle event identity.
        event_id: String,
        /// Lifecycle action identity.
        action_id: String,
    },
    /// Logical authority-root ceiling, not effect authority.
    AuthorityRoot {
        /// Exact authority-root identity.
        authority_root_id: String,
    },
    /// Forbidden-token authority coordinate.
    DisclosureCeiling {
        /// Exact authority coordinate.
        authority_coordinate: String,
    },
}

impl EffectiveProfileSelection {
    pub(crate) fn from_api(value: &ApiSelection) -> Self {
        match value {
            ApiSelection::Package {
                package_id,
                version,
                content_digest,
            } => Self::Package {
                package_id: package_id.clone(),
                version: version.clone(),
                content_digest: content_digest.clone(),
            },
            ApiSelection::NormativePack {
                pack_id,
                attestor_id,
                version,
                content_digest,
            } => Self::NormativePack {
                pack_id: pack_id.clone(),
                attestor_id: attestor_id.clone(),
                version: version.clone(),
                content_digest: content_digest.clone(),
            },
            ApiSelection::SemanticResource {
                repository_id,
                resource_kind,
                resource_id,
            } => Self::SemanticResource {
                repository_id: repository_id.clone(),
                resource_kind: EffectiveProfileResourceKind::from_api(*resource_kind),
                resource_id: resource_id.clone(),
            },
            ApiSelection::RunPolicy { run_policy_id } => Self::RunPolicy {
                run_policy_id: run_policy_id.clone(),
            },
            ApiSelection::Session {
                execution_workspace_id,
                activation_root_count,
                additional_root_count,
                activation_path_count,
            } => Self::Session {
                execution_workspace_id: execution_workspace_id.clone(),
                activation_root_count: *activation_root_count,
                additional_root_count: *additional_root_count,
                activation_path_count: *activation_path_count,
            },
            ApiSelection::Lifecycle {
                lifecycle_id,
                event_id,
                action_id,
            } => Self::Lifecycle {
                lifecycle_id: lifecycle_id.clone(),
                event_id: event_id.clone(),
                action_id: action_id.clone(),
            },
            ApiSelection::AuthorityRoot { authority_root_id } => Self::AuthorityRoot {
                authority_root_id: authority_root_id.clone(),
            },
            ApiSelection::DisclosureCeiling {
                authority_coordinate,
            } => Self::DisclosureCeiling {
                authority_coordinate: authority_coordinate.clone(),
            },
        }
    }
}

/// Private provider selection, independent of native readiness.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum EffectiveProfileProviderDisposition {
    /// A provider family is configured, not necessarily available.
    Bound {
        /// Exact selected provider family.
        provider: EffectiveProfileProviderKind,
    },
    /// No private provider binding exists.
    MissingBinding {},
    /// Private narrowing excluded the occurrence.
    ExcludedByNarrowing {},
    /// Provider and requirement effects differ.
    EffectMismatch {},
    /// The required recipe contract is absent.
    RecipeContractMismatch {},
}

impl EffectiveProfileProviderDisposition {
    pub(crate) fn from_api(value: ApiProviderDisposition) -> Self {
        match value {
            ApiProviderDisposition::Bound { provider } => Self::Bound {
                provider: EffectiveProfileProviderKind::from_api(provider),
            },
            ApiProviderDisposition::MissingBinding => Self::MissingBinding {},
            ApiProviderDisposition::ExcludedByNarrowing => Self::ExcludedByNarrowing {},
            ApiProviderDisposition::EffectMismatch => Self::EffectMismatch {},
            ApiProviderDisposition::RecipeContractMismatch => Self::RecipeContractMismatch {},
        }
    }
}

/// Copied application finding with only the selected variant's required fields.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum EffectiveProfileFinding {
    /// No owner-native observation has qualified this machine.
    NativeRealizationUnknown {
        /// Selected private machine-binding identity.
        machine_id: String,
    },
    /// A semantic operation has no private binding.
    OperationUnbound {
        /// Logical repository containing the occurrence.
        repository_id: String,
        /// Owner-qualified operation identity.
        operation_id: String,
    },
    /// One capability requirement's exact portable disposition.
    Requirement {
        /// Logical repository containing the occurrence.
        repository_id: String,
        /// Owner-qualified operation identity.
        operation_id: String,
        /// Requirement identity local to the operation.
        requirement_id: String,
        /// Portable capability identity.
        capability_id: String,
        /// Independent portable requirement outcome.
        disposition: RequirementDisposition,
    },
}

impl EffectiveProfileFinding {
    pub(crate) fn from_api(value: &ApiFinding) -> Self {
        match value {
            ApiFinding::NativeRealizationUnknown { machine_id } => Self::NativeRealizationUnknown {
                machine_id: machine_id.clone(),
            },
            ApiFinding::OperationUnbound {
                repository_id,
                operation_id,
            } => Self::OperationUnbound {
                repository_id: repository_id.clone(),
                operation_id: operation_id.clone(),
            },
            ApiFinding::Requirement {
                repository_id,
                operation_id,
                requirement_id,
                capability_id,
                disposition,
            } => Self::Requirement {
                repository_id: repository_id.clone(),
                operation_id: operation_id.clone(),
                requirement_id: requirement_id.clone(),
                capability_id: capability_id.clone(),
                disposition: RequirementDisposition::from_api(*disposition),
            },
        }
    }
}

/// Copied selected-path reason, distinct from an observation causal step.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum EffectiveProfilePathReason {
    /// Owner-issued origin receipt.
    OriginReceipt {
        /// Exact origin-receipt contract.
        contract_id: String,
    },
    /// Contract beginning traversal.
    EntryContract {
        /// Exact entry contract.
        contract_id: String,
    },
    /// Responsibility carried by the path.
    ResponsibilityContract {
        /// Exact responsibility contract.
        contract_id: String,
    },
    /// Required provider binding.
    ProviderBinding {
        /// Exact provider-binding identity.
        binding_id: String,
    },
    /// Ordered neutral delegation edge.
    DelegationEdge {
        /// Exact delegation-edge identity.
        edge_id: String,
    },
    /// Contract bounding completion.
    TerminationContract {
        /// Exact termination contract.
        contract_id: String,
    },
    /// Contract requiring settlement.
    SettlementContract {
        /// Exact settlement contract.
        contract_id: String,
    },
    /// Contract surviving completion.
    ContinuityContract {
        /// Exact continuity contract.
        contract_id: String,
    },
}

impl EffectiveProfilePathReason {
    pub(crate) fn from_api(value: &ApiPathReason) -> Self {
        match value {
            ApiPathReason::OriginReceipt { contract_id } => Self::OriginReceipt {
                contract_id: contract_id.clone(),
            },
            ApiPathReason::EntryContract { contract_id } => Self::EntryContract {
                contract_id: contract_id.clone(),
            },
            ApiPathReason::ResponsibilityContract { contract_id } => Self::ResponsibilityContract {
                contract_id: contract_id.clone(),
            },
            ApiPathReason::ProviderBinding { binding_id } => Self::ProviderBinding {
                binding_id: binding_id.clone(),
            },
            ApiPathReason::DelegationEdge { edge_id } => Self::DelegationEdge {
                edge_id: edge_id.clone(),
            },
            ApiPathReason::TerminationContract { contract_id } => Self::TerminationContract {
                contract_id: contract_id.clone(),
            },
            ApiPathReason::SettlementContract { contract_id } => Self::SettlementContract {
                contract_id: contract_id.clone(),
            },
            ApiPathReason::ContinuityContract { contract_id } => Self::ContinuityContract {
                contract_id: contract_id.clone(),
            },
        }
    }
}
