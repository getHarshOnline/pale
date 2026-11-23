//! Values retained by effective-profile explanations.
//!
//! Getters copy the SDK's selected fields; they do not classify configuration
//! or infer whether a configured provider is available.

use pale_sdk::{
    EffectiveProfileDeclaredSelection, EffectiveProfileDeclaredSelector, EffectiveProfileFinding,
    EffectiveProfileModeAxis, EffectiveProfileNativeRealization, EffectiveProfilePathReason,
    EffectiveProfileProviderDisposition, EffectiveProfileProviderKind,
    EffectiveProfileResourceKind, EffectiveProfileSelection,
};
use pyo3::prelude::*;

use crate::PyRequirementDisposition;

/// Package semantic family of a selected resource.
#[pyclass(
    name = "EffectiveProfileResourceKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyEffectiveProfileResourceKind {
    /// Operation procedure.
    Operation,
    /// Specialist role.
    Role,
    /// Standalone doctrine.
    Doctrine,
    /// Doctrine selected as startup policy.
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

impl PyEffectiveProfileResourceKind {
    pub(crate) fn from_api(value: EffectiveProfileResourceKind) -> Self {
        match value {
            EffectiveProfileResourceKind::Operation => Self::Operation,
            EffectiveProfileResourceKind::Role => Self::Role,
            EffectiveProfileResourceKind::Doctrine => Self::Doctrine,
            EffectiveProfileResourceKind::StartupPolicy => Self::StartupPolicy,
            EffectiveProfileResourceKind::RunMode => Self::RunMode,
            EffectiveProfileResourceKind::Stack => Self::Stack,
            EffectiveProfileResourceKind::PolicyFamily => Self::PolicyFamily,
            EffectiveProfileResourceKind::Recipe => Self::Recipe,
        }
    }
}

/// Immutable Package release selected by the SDK.
#[pyclass(name = "EffectiveProfilePackageSelection", frozen, get_all)]
pub struct PyEffectiveProfilePackageSelection {
    /// Package identity.
    package_id: String,
    /// Selected Package version.
    version: String,
    /// Exact Package release digest.
    content_digest: String,
}

/// Attestor-scoped normative Pack release.
#[pyclass(name = "EffectiveProfileNormativePackSelection", frozen, get_all)]
pub struct PyEffectiveProfileNormativePackSelection {
    /// Normative Pack identity.
    pack_id: String,
    /// Attestor owning the release statement.
    attestor_id: String,
    /// Selected Pack version.
    version: String,
    /// Exact Pack release digest.
    content_digest: String,
}

/// Package resource selected for one repository.
#[pyclass(name = "EffectiveProfileSemanticResourceSelection", frozen)]
pub struct PyEffectiveProfileSemanticResourceSelection {
    /// Logical repository receiving the resource.
    #[pyo3(get)]
    repository_id: String,
    resource_kind: EffectiveProfileResourceKind,
    /// Owner-qualified resource identity.
    #[pyo3(get)]
    resource_id: String,
}

#[pymethods]
impl PyEffectiveProfileSemanticResourceSelection {
    /// Selected semantic resource family.
    #[getter]
    fn resource_kind(&self) -> PyEffectiveProfileResourceKind {
        PyEffectiveProfileResourceKind::from_api(self.resource_kind)
    }
}

/// Selected private run policy.
#[pyclass(name = "EffectiveProfileRunPolicySelection", frozen, get_all)]
pub struct PyEffectiveProfileRunPolicySelection {
    /// Exact run-policy identity.
    run_policy_id: String,
}

/// Selected session model and bounded cardinalities, including zero.
#[pyclass(name = "EffectiveProfileSessionSelection", frozen, get_all)]
pub struct PyEffectiveProfileSessionSelection {
    /// Execution workspace owning the selection.
    execution_workspace_id: String,
    /// Number of activation roots.
    activation_root_count: u32,
    /// Number of additional roots.
    additional_root_count: u32,
    /// Number of activation paths.
    activation_path_count: u32,
}

/// One admitted lifecycle behavior.
#[pyclass(name = "EffectiveProfileLifecycleSelection", frozen, get_all)]
pub struct PyEffectiveProfileLifecycleSelection {
    /// Lifecycle behavior identity.
    lifecycle_id: String,
    /// Lifecycle event identity.
    event_id: String,
    /// Lifecycle action identity.
    action_id: String,
}

/// Logical authority-root ceiling, not effect authority.
#[pyclass(name = "EffectiveProfileAuthorityRootSelection", frozen, get_all)]
pub struct PyEffectiveProfileAuthorityRootSelection {
    /// Exact authority-root identity.
    authority_root_id: String,
}

/// Forbidden-token authority coordinate.
#[pyclass(name = "EffectiveProfileDisclosureCeilingSelection", frozen, get_all)]
pub struct PyEffectiveProfileDisclosureCeilingSelection {
    /// Exact authority coordinate.
    authority_coordinate: String,
}

// Transparent outputs preserve required fields without a second classifier,
// admission path or public constructor. Each conversion owns its copied value.
#[derive(IntoPyObject)]
pub(crate) enum PyEffectiveProfileSelection {
    Package(PyEffectiveProfilePackageSelection),
    NormativePack(PyEffectiveProfileNormativePackSelection),
    SemanticResource(PyEffectiveProfileSemanticResourceSelection),
    RunPolicy(PyEffectiveProfileRunPolicySelection),
    Session(PyEffectiveProfileSessionSelection),
    Lifecycle(PyEffectiveProfileLifecycleSelection),
    AuthorityRoot(PyEffectiveProfileAuthorityRootSelection),
    DisclosureCeiling(PyEffectiveProfileDisclosureCeilingSelection),
}

impl PyEffectiveProfileSelection {
    pub(crate) fn from_api(inner: EffectiveProfileSelection) -> Self {
        match inner {
            EffectiveProfileSelection::Package {
                package_id,
                version,
                content_digest,
            } => Self::Package(PyEffectiveProfilePackageSelection {
                package_id,
                version,
                content_digest,
            }),
            EffectiveProfileSelection::NormativePack {
                pack_id,
                attestor_id,
                version,
                content_digest,
            } => Self::NormativePack(PyEffectiveProfileNormativePackSelection {
                pack_id,
                attestor_id,
                version,
                content_digest,
            }),
            EffectiveProfileSelection::SemanticResource {
                repository_id,
                resource_kind,
                resource_id,
            } => Self::SemanticResource(PyEffectiveProfileSemanticResourceSelection {
                repository_id,
                resource_kind,
                resource_id,
            }),
            EffectiveProfileSelection::RunPolicy { run_policy_id } => {
                Self::RunPolicy(PyEffectiveProfileRunPolicySelection { run_policy_id })
            }
            EffectiveProfileSelection::Session {
                execution_workspace_id,
                activation_root_count,
                additional_root_count,
                activation_path_count,
            } => Self::Session(PyEffectiveProfileSessionSelection {
                execution_workspace_id,
                activation_root_count,
                additional_root_count,
                activation_path_count,
            }),
            EffectiveProfileSelection::Lifecycle {
                lifecycle_id,
                event_id,
                action_id,
            } => Self::Lifecycle(PyEffectiveProfileLifecycleSelection {
                lifecycle_id,
                event_id,
                action_id,
            }),
            EffectiveProfileSelection::AuthorityRoot { authority_root_id } => {
                Self::AuthorityRoot(PyEffectiveProfileAuthorityRootSelection { authority_root_id })
            }
            EffectiveProfileSelection::DisclosureCeiling {
                authority_coordinate,
            } => Self::DisclosureCeiling(PyEffectiveProfileDisclosureCeilingSelection {
                authority_coordinate,
            }),
        }
    }
}

/// Orthogonal mode-default axis.
#[pyclass(
    name = "EffectiveProfileModeAxis",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyEffectiveProfileModeAxis {
    /// Process and verification guidance.
    Intensity,
    /// Specialist task stance.
    TaskStance,
}

impl PyEffectiveProfileModeAxis {
    pub(crate) fn from_api(value: EffectiveProfileModeAxis) -> Self {
        match value {
            EffectiveProfileModeAxis::Intensity => Self::Intensity,
            EffectiveProfileModeAxis::TaskStance => Self::TaskStance,
        }
    }
}

/// Provider kind selected by private configuration.
#[pyclass(
    name = "EffectiveProfileProviderKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyEffectiveProfileProviderKind {
    /// Native-harness tool binding.
    HostNative,
    /// Pale-owned Rust mechanic.
    NativeRust,
    /// Admitted connector binding.
    Connector,
}

impl PyEffectiveProfileProviderKind {
    pub(crate) fn from_api(value: EffectiveProfileProviderKind) -> Self {
        match value {
            EffectiveProfileProviderKind::HostNative => Self::HostNative,
            EffectiveProfileProviderKind::NativeRust => Self::NativeRust,
            EffectiveProfileProviderKind::Connector => Self::Connector,
        }
    }
}

/// Owner-native evidence available to this compiler inspection.
#[pyclass(
    name = "EffectiveProfileNativeRealization",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyEffectiveProfileNativeRealization {
    /// No owner-native observation has been admitted.
    Unknown,
}

impl PyEffectiveProfileNativeRealization {
    pub(crate) fn from_api(value: EffectiveProfileNativeRealization) -> Self {
        match value {
            EffectiveProfileNativeRealization::Unknown => Self::Unknown,
        }
    }
}

/// One exact declared resource, before compiler closure expansion.
#[pyclass(name = "EffectiveProfileDeclaredExactSelector", frozen, get_all)]
pub struct PyEffectiveProfileDeclaredExactSelector {
    /// Owner-qualified declared resource.
    resource_id: String,
}

/// A Package-scoped declaration, not its expanded resource closure.
#[pyclass(
    name = "EffectiveProfileDeclaredAllFromPackageSelector",
    frozen,
    get_all
)]
pub struct PyEffectiveProfileDeclaredAllFromPackageSelector {
    /// Package scope of the declared family.
    package_id: String,
}

#[derive(IntoPyObject)]
pub(crate) enum PyEffectiveProfileDeclaredSelector {
    #[pyo3(transparent)]
    Exact(PyEffectiveProfileDeclaredExactSelector),
    #[pyo3(transparent)]
    AllFromPackage(PyEffectiveProfileDeclaredAllFromPackageSelector),
}

impl PyEffectiveProfileDeclaredSelector {
    fn from_api(value: EffectiveProfileDeclaredSelector) -> Self {
        match value {
            EffectiveProfileDeclaredSelector::Exact { resource_id } => {
                Self::Exact(PyEffectiveProfileDeclaredExactSelector { resource_id })
            }
            EffectiveProfileDeclaredSelector::AllFromPackage { package_id } => {
                Self::AllFromPackage(PyEffectiveProfileDeclaredAllFromPackageSelector {
                    package_id,
                })
            }
        }
    }
}

/// An admitted resource declaration with its unexpanded selector.
/// Copied values do not admit sources or authorize effects.
#[pyclass(name = "EffectiveProfileDeclaredResourceSelection", frozen)]
pub struct PyEffectiveProfileDeclaredResourceSelection {
    resource_kind: EffectiveProfileResourceKind,
    selector: EffectiveProfileDeclaredSelector,
}

#[pymethods]
impl PyEffectiveProfileDeclaredResourceSelection {
    /// Semantic family selected by the enclosing declaration.
    #[getter]
    fn resource_kind(&self) -> PyEffectiveProfileResourceKind {
        PyEffectiveProfileResourceKind::from_api(self.resource_kind)
    }

    /// Return a copied exact or Package-scoped selector.
    #[getter]
    fn selector(&self) -> PyEffectiveProfileDeclaredSelector {
        PyEffectiveProfileDeclaredSelector::from_api(self.selector.clone())
    }
}

/// An explicit default for one runtime-mode dimension.
#[pyclass(name = "EffectiveProfileDeclaredModeDefaultSelection", frozen)]
pub struct PyEffectiveProfileDeclaredModeDefaultSelection {
    axis: EffectiveProfileModeAxis,
    /// Exact declared run-mode resource.
    #[pyo3(get)]
    resource_id: String,
}

#[pymethods]
impl PyEffectiveProfileDeclaredModeDefaultSelection {
    /// Orthogonal runtime-mode dimension.
    #[getter]
    fn axis(&self) -> PyEffectiveProfileModeAxis {
        PyEffectiveProfileModeAxis::from_api(self.axis)
    }
}

#[derive(IntoPyObject)]
pub(crate) enum PyEffectiveProfileDeclaredSelection {
    #[pyo3(transparent)]
    Resource(PyEffectiveProfileDeclaredResourceSelection),
    #[pyo3(transparent)]
    ModeDefault(PyEffectiveProfileDeclaredModeDefaultSelection),
}

impl PyEffectiveProfileDeclaredSelection {
    pub(crate) fn from_api(value: EffectiveProfileDeclaredSelection) -> Self {
        match value {
            EffectiveProfileDeclaredSelection::Resource {
                resource_kind,
                selector,
            } => Self::Resource(PyEffectiveProfileDeclaredResourceSelection {
                resource_kind,
                selector,
            }),
            EffectiveProfileDeclaredSelection::ModeDefault { axis, resource_id } => {
                Self::ModeDefault(PyEffectiveProfileDeclaredModeDefaultSelection {
                    axis,
                    resource_id,
                })
            }
        }
    }
}

/// A configured provider family, not proof of native availability.
#[pyclass(name = "EffectiveProfileBoundProviderDisposition", frozen)]
pub struct PyEffectiveProfileBoundProviderDisposition {
    provider: EffectiveProfileProviderKind,
}
#[pymethods]
impl PyEffectiveProfileBoundProviderDisposition {
    /// Exact selected provider family.
    #[getter]
    fn provider(&self) -> PyEffectiveProfileProviderKind {
        PyEffectiveProfileProviderKind::from_api(self.provider)
    }
}

/// No private provider binding exists.
#[pyclass(name = "EffectiveProfileMissingBindingProviderDisposition", frozen)]
pub struct PyEffectiveProfileMissingBindingProviderDisposition {}

/// Private narrowing excluded this requirement occurrence.
#[pyclass(
    name = "EffectiveProfileExcludedByNarrowingProviderDisposition",
    frozen
)]
pub struct PyEffectiveProfileExcludedByNarrowingProviderDisposition {}

/// The provider and requirement effects differ.
#[pyclass(name = "EffectiveProfileEffectMismatchProviderDisposition", frozen)]
pub struct PyEffectiveProfileEffectMismatchProviderDisposition {}

/// The provider lacks the required recipe contract.
#[pyclass(
    name = "EffectiveProfileRecipeContractMismatchProviderDisposition",
    frozen
)]
pub struct PyEffectiveProfileRecipeContractMismatchProviderDisposition {}

#[derive(IntoPyObject)]
pub(crate) enum PyEffectiveProfileProviderDisposition {
    #[pyo3(transparent)]
    Bound(PyEffectiveProfileBoundProviderDisposition),
    #[pyo3(transparent)]
    MissingBinding(PyEffectiveProfileMissingBindingProviderDisposition),
    #[pyo3(transparent)]
    ExcludedByNarrowing(PyEffectiveProfileExcludedByNarrowingProviderDisposition),
    #[pyo3(transparent)]
    EffectMismatch(PyEffectiveProfileEffectMismatchProviderDisposition),
    #[pyo3(transparent)]
    RecipeContractMismatch(PyEffectiveProfileRecipeContractMismatchProviderDisposition),
}

impl PyEffectiveProfileProviderDisposition {
    pub(crate) fn from_api(value: EffectiveProfileProviderDisposition) -> Self {
        match value {
            EffectiveProfileProviderDisposition::Bound { provider } => {
                Self::Bound(PyEffectiveProfileBoundProviderDisposition { provider })
            }
            EffectiveProfileProviderDisposition::MissingBinding => {
                Self::MissingBinding(PyEffectiveProfileMissingBindingProviderDisposition {})
            }
            EffectiveProfileProviderDisposition::ExcludedByNarrowing => Self::ExcludedByNarrowing(
                PyEffectiveProfileExcludedByNarrowingProviderDisposition {},
            ),
            EffectiveProfileProviderDisposition::EffectMismatch => {
                Self::EffectMismatch(PyEffectiveProfileEffectMismatchProviderDisposition {})
            }
            EffectiveProfileProviderDisposition::RecipeContractMismatch => {
                Self::RecipeContractMismatch(
                    PyEffectiveProfileRecipeContractMismatchProviderDisposition {},
                )
            }
        }
    }
}

/// No owner-native observation has qualified this machine.
#[pyclass(
    name = "EffectiveProfileNativeRealizationUnknownFinding",
    frozen,
    get_all
)]
pub struct PyEffectiveProfileNativeRealizationUnknownFinding {
    /// Exact selected machine binding.
    machine_id: String,
}

/// A semantic operation has no private binding.
#[pyclass(name = "EffectiveProfileOperationUnboundFinding", frozen, get_all)]
pub struct PyEffectiveProfileOperationUnboundFinding {
    /// Logical repository containing the occurrence.
    repository_id: String,
    /// Owner-qualified operation identity.
    operation_id: String,
}

/// One capability requirement's exact portable outcome.
#[pyclass(name = "EffectiveProfileRequirementFinding", frozen)]
pub struct PyEffectiveProfileRequirementFinding {
    /// Logical repository containing the occurrence.
    #[pyo3(get)]
    repository_id: String,
    /// Owner-qualified operation identity.
    #[pyo3(get)]
    operation_id: String,
    /// Requirement identity local to the operation.
    #[pyo3(get)]
    requirement_id: String,
    /// Portable capability identity.
    #[pyo3(get)]
    capability_id: String,
    disposition: pale_sdk::EffectiveProfileRequirementDisposition,
}

#[pymethods]
impl PyEffectiveProfileRequirementFinding {
    /// Independent portable requirement outcome.
    #[getter]
    fn disposition(&self) -> PyRequirementDisposition {
        PyRequirementDisposition::from_api(self.disposition)
    }
}

#[derive(IntoPyObject)]
pub(crate) enum PyEffectiveProfileFinding {
    #[pyo3(transparent)]
    NativeRealizationUnknown(PyEffectiveProfileNativeRealizationUnknownFinding),
    #[pyo3(transparent)]
    OperationUnbound(PyEffectiveProfileOperationUnboundFinding),
    #[pyo3(transparent)]
    Requirement(PyEffectiveProfileRequirementFinding),
}

impl PyEffectiveProfileFinding {
    pub(crate) fn from_api(value: EffectiveProfileFinding) -> Self {
        match value {
            EffectiveProfileFinding::NativeRealizationUnknown { machine_id } => {
                Self::NativeRealizationUnknown(PyEffectiveProfileNativeRealizationUnknownFinding {
                    machine_id,
                })
            }
            EffectiveProfileFinding::OperationUnbound {
                repository_id,
                operation_id,
            } => Self::OperationUnbound(PyEffectiveProfileOperationUnboundFinding {
                repository_id,
                operation_id,
            }),
            EffectiveProfileFinding::Requirement {
                repository_id,
                operation_id,
                requirement_id,
                capability_id,
                disposition,
            } => Self::Requirement(PyEffectiveProfileRequirementFinding {
                repository_id,
                operation_id,
                requirement_id,
                capability_id,
                disposition,
            }),
        }
    }
}

/// Reference to the selected owner-issued origin-receipt contract.
#[pyclass(name = "EffectiveProfileOriginReceiptReason", frozen, get_all)]
pub struct PyEffectiveProfileOriginReceiptReason {
    /// Origin-receipt contract identity.
    contract_id: String,
}

/// Contract beginning the selected traversal.
#[pyclass(name = "EffectiveProfileEntryContractReason", frozen, get_all)]
pub struct PyEffectiveProfileEntryContractReason {
    /// Entry-contract identity.
    contract_id: String,
}

/// Responsibility carried by the selected path.
#[pyclass(name = "EffectiveProfileResponsibilityContractReason", frozen, get_all)]
pub struct PyEffectiveProfileResponsibilityContractReason {
    /// Responsibility-contract identity.
    contract_id: String,
}

/// Provider binding explicitly required by the path.
#[pyclass(name = "EffectiveProfileProviderBindingReason", frozen, get_all)]
pub struct PyEffectiveProfileProviderBindingReason {
    /// Provider-binding identity.
    binding_id: String,
}

/// Ordered neutral delegation edge.
#[pyclass(name = "EffectiveProfileDelegationEdgeReason", frozen, get_all)]
pub struct PyEffectiveProfileDelegationEdgeReason {
    /// Delegation-edge identity.
    edge_id: String,
}

/// Contract bounding completion.
#[pyclass(name = "EffectiveProfileTerminationContractReason", frozen, get_all)]
pub struct PyEffectiveProfileTerminationContractReason {
    /// Termination-contract identity.
    contract_id: String,
}

/// Contract requiring settlement.
#[pyclass(name = "EffectiveProfileSettlementContractReason", frozen, get_all)]
pub struct PyEffectiveProfileSettlementContractReason {
    /// Settlement-contract identity.
    contract_id: String,
}

/// Contract surviving completion.
#[pyclass(name = "EffectiveProfileContinuityContractReason", frozen, get_all)]
pub struct PyEffectiveProfileContinuityContractReason {
    /// Continuity-contract identity.
    contract_id: String,
}

#[derive(IntoPyObject)]
pub(crate) enum PyEffectiveProfilePathReason {
    #[pyo3(transparent)]
    OriginReceipt(PyEffectiveProfileOriginReceiptReason),
    #[pyo3(transparent)]
    EntryContract(PyEffectiveProfileEntryContractReason),
    #[pyo3(transparent)]
    ResponsibilityContract(PyEffectiveProfileResponsibilityContractReason),
    #[pyo3(transparent)]
    ProviderBinding(PyEffectiveProfileProviderBindingReason),
    #[pyo3(transparent)]
    DelegationEdge(PyEffectiveProfileDelegationEdgeReason),
    #[pyo3(transparent)]
    TerminationContract(PyEffectiveProfileTerminationContractReason),
    #[pyo3(transparent)]
    SettlementContract(PyEffectiveProfileSettlementContractReason),
    #[pyo3(transparent)]
    ContinuityContract(PyEffectiveProfileContinuityContractReason),
}

impl PyEffectiveProfilePathReason {
    pub(crate) fn from_api(value: EffectiveProfilePathReason) -> Self {
        match value {
            EffectiveProfilePathReason::OriginReceipt { contract_id } => {
                Self::OriginReceipt(PyEffectiveProfileOriginReceiptReason { contract_id })
            }
            EffectiveProfilePathReason::EntryContract { contract_id } => {
                Self::EntryContract(PyEffectiveProfileEntryContractReason { contract_id })
            }
            EffectiveProfilePathReason::ResponsibilityContract { contract_id } => {
                Self::ResponsibilityContract(PyEffectiveProfileResponsibilityContractReason {
                    contract_id,
                })
            }
            EffectiveProfilePathReason::ProviderBinding { binding_id } => {
                Self::ProviderBinding(PyEffectiveProfileProviderBindingReason { binding_id })
            }
            EffectiveProfilePathReason::DelegationEdge { edge_id } => {
                Self::DelegationEdge(PyEffectiveProfileDelegationEdgeReason { edge_id })
            }
            EffectiveProfilePathReason::TerminationContract { contract_id } => {
                Self::TerminationContract(PyEffectiveProfileTerminationContractReason {
                    contract_id,
                })
            }
            EffectiveProfilePathReason::SettlementContract { contract_id } => {
                Self::SettlementContract(PyEffectiveProfileSettlementContractReason { contract_id })
            }
            EffectiveProfilePathReason::ContinuityContract { contract_id } => {
                Self::ContinuityContract(PyEffectiveProfileContinuityContractReason { contract_id })
            }
        }
    }
}
