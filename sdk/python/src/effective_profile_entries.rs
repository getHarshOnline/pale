//! Concrete row payloads shared by profile pages and authoring inspection.
//!
//! Each entry retains the SDK row and its explanation identity. Its detail
//! is a generated union of frozen classes, so narrowing selects required
//! fields instead of an object containing every variant's optional fields.

use pale_sdk::{EffectiveProfileEntry, EffectiveProfileEntryDetail};
use pyo3::prelude::*;

use crate::effective_profile_entry_values::{
    PyEffectiveProfileParticipant, PyEffectiveProfileResponsibility,
};
use crate::workspace_subject_value::PyWorkspaceSubjectValue;
use crate::{
    PyEffectiveProfileDeclaredSelection, PyEffectiveProfileEffectClass, PyEffectiveProfileFinding,
    PyEffectiveProfileNativeRealization, PyEffectiveProfilePathReason,
    PyEffectiveProfileProviderBindingRef, PyEffectiveProfileProviderDisposition,
    PyEffectiveProfileRepositoryAccess, PyEffectiveProfileSelection, PyOperationExposure,
    PyRequirementDisposition,
};

/// A declared selector before closure expansion.
#[pyclass(name = "EffectiveProfileDeclaredSelectionDetail", frozen)]
pub struct PyEffectiveProfileDeclaredSelectionDetail {
    selection: pale_sdk::EffectiveProfileDeclaredSelection,
}

#[pymethods]
impl PyEffectiveProfileDeclaredSelectionDetail {
    /// Declared resource or mode selector.
    #[getter]
    fn selection(&self) -> PyEffectiveProfileDeclaredSelection {
        PyEffectiveProfileDeclaredSelection::from_api(self.selection.clone())
    }
}

/// One admitted responsibility contract required by a selected path.
#[pyclass(name = "EffectiveProfileResponsibilityDetail", frozen)]
pub struct PyEffectiveProfileResponsibilityDetail {
    /// Exact responsibility-contract identity.
    #[pyo3(get)]
    contract_id: String,
    /// Execution-path selection carrying the contract.
    #[pyo3(get)]
    execution_path_id: String,
    participant: pale_sdk::EffectiveProfileParticipant,
    responsibilities: Vec<pale_sdk::EffectiveProfileResponsibility>,
    authority_ceiling: Vec<pale_sdk::EffectiveProfileEffectClass>,
    reason_steps: Vec<pale_sdk::EffectiveProfilePathReason>,
    subject: pale_sdk::PortableWorkspaceSubjectSelector,
}

#[pymethods]
impl PyEffectiveProfileResponsibilityDetail {
    /// Concrete participant bounded by the contract.
    #[getter]
    fn participant(&self) -> PyEffectiveProfileParticipant {
        PyEffectiveProfileParticipant::from_api(self.participant.clone())
    }

    /// Responsibilities in SDK order, including an empty list.
    #[getter]
    fn responsibilities(&self) -> Vec<PyEffectiveProfileResponsibility> {
        self.responsibilities
            .iter()
            .cloned()
            .map(PyEffectiveProfileResponsibility::from_api)
            .collect()
    }

    /// Maximum effect classes admitted for the contract.
    #[getter]
    fn authority_ceiling(&self) -> Vec<PyEffectiveProfileEffectClass> {
        self.authority_ceiling
            .iter()
            .copied()
            .map(PyEffectiveProfileEffectClass::from_api)
            .collect()
    }

    /// Ordered application-owned reasons for the carrying path.
    #[getter]
    fn reason_steps(&self) -> Vec<PyEffectiveProfilePathReason> {
        self.reason_steps
            .iter()
            .cloned()
            .map(PyEffectiveProfilePathReason::from_api)
            .collect()
    }

    /// Copied causal selector, not an admitted proof handle.
    #[getter]
    fn subject(&self) -> PyWorkspaceSubjectValue {
        PyWorkspaceSubjectValue::from_api(self.subject.clone())
    }
}

/// One exact closure, policy, lifecycle or semantic-resource selection.
#[pyclass(name = "EffectiveProfileSelectionDetail", frozen)]
pub struct PyEffectiveProfileSelectionDetail {
    selection: pale_sdk::EffectiveProfileSelection,
}

#[pymethods]
impl PyEffectiveProfileSelectionDetail {
    /// Effective structured selection coordinate.
    #[getter]
    fn selection(&self) -> PyEffectiveProfileSelection {
        PyEffectiveProfileSelection::from_api(self.selection.clone())
    }
}

/// Private capability and effect narrowing after Package policy.
#[pyclass(name = "EffectiveProfileCapabilityNarrowingDetail", frozen)]
pub struct PyEffectiveProfileCapabilityNarrowingDetail {
    /// Capabilities retained by private configuration.
    #[pyo3(get)]
    permitted_capabilities: Vec<String>,
    /// Capabilities forbidden by private configuration.
    #[pyo3(get)]
    forbidden_capabilities: Vec<String>,
    permitted_effects: Vec<pale_sdk::EffectiveProfileEffectClass>,
    forbidden_effects: Vec<pale_sdk::EffectiveProfileEffectClass>,
}

#[pymethods]
impl PyEffectiveProfileCapabilityNarrowingDetail {
    /// Effects retained by private configuration.
    #[getter]
    fn permitted_effects(&self) -> Vec<PyEffectiveProfileEffectClass> {
        self.permitted_effects
            .iter()
            .copied()
            .map(PyEffectiveProfileEffectClass::from_api)
            .collect()
    }

    /// Effects forbidden by private configuration.
    #[getter]
    fn forbidden_effects(&self) -> Vec<PyEffectiveProfileEffectClass> {
        self.forbidden_effects
            .iter()
            .copied()
            .map(PyEffectiveProfileEffectClass::from_api)
            .collect()
    }
}

/// Application-owned Package-policy meet with contributor provenance.
#[pyclass(name = "EffectiveProfileGoverningCeilingDetail", frozen)]
pub struct PyEffectiveProfileGoverningCeilingDetail {
    /// Semantic scope of the result.
    #[pyo3(get)]
    scope: String,
    /// Governed Package resource.
    #[pyo3(get)]
    subject_resource: String,
    /// Canonical semantic contributors in SDK order.
    #[pyo3(get)]
    contributors: Vec<String>,
    /// Exact Package policy resources whose bodies were met.
    #[pyo3(get)]
    policies: Vec<String>,
    /// Capabilities permitted by every contributor.
    #[pyo3(get)]
    permitted_capabilities: Vec<String>,
    /// Capabilities forbidden by any contributor.
    #[pyo3(get)]
    forbidden_capabilities: Vec<String>,
    permitted_effects: Vec<pale_sdk::EffectiveProfileEffectClass>,
    forbidden_effects: Vec<pale_sdk::EffectiveProfileEffectClass>,
}

#[pymethods]
impl PyEffectiveProfileGoverningCeilingDetail {
    /// Effects permitted by every contributor.
    #[getter]
    fn permitted_effects(&self) -> Vec<PyEffectiveProfileEffectClass> {
        self.permitted_effects
            .iter()
            .copied()
            .map(PyEffectiveProfileEffectClass::from_api)
            .collect()
    }

    /// Effects forbidden by any contributor.
    #[getter]
    fn forbidden_effects(&self) -> Vec<PyEffectiveProfileEffectClass> {
        self.forbidden_effects
            .iter()
            .copied()
            .map(PyEffectiveProfileEffectClass::from_api)
            .collect()
    }
}

/// One repository in exact effective materialization order.
#[pyclass(name = "EffectiveProfileRepositoryDetail", frozen)]
pub struct PyEffectiveProfileRepositoryDetail {
    /// Logical repository identity.
    #[pyo3(get)]
    repository_id: String,
    /// Zero-based effective order, preserving the SDK's full u32 range.
    #[pyo3(get)]
    materialization_order: u32,
    access_ceiling: pale_sdk::EffectiveProfileRepositoryAccess,
}

#[pymethods]
impl PyEffectiveProfileRepositoryDetail {
    /// Access ceiling before independent effect admission.
    #[getter]
    fn access_ceiling(&self) -> PyEffectiveProfileRepositoryAccess {
        PyEffectiveProfileRepositoryAccess::from_api(self.access_ceiling)
    }
}

/// One provider-native projection over a selected repository.
#[pyclass(name = "EffectiveProfileProviderRepositoryDetail", frozen)]
pub struct PyEffectiveProfileProviderRepositoryDetail {
    /// Logical repository selected by the execution workspace.
    #[pyo3(get)]
    repository_id: String,
    /// Neutral provider-binding identity selected by an experience path.
    #[pyo3(get)]
    provider_binding_id: String,
    /// Opaque provider-owned coordinate, not a local path.
    #[pyo3(get)]
    native_repository_coordinate: String,
    native_realization: pale_sdk::EffectiveProfileNativeRealization,
}

#[pymethods]
impl PyEffectiveProfileProviderRepositoryDetail {
    /// Separately qualified native evidence ceiling.
    #[getter]
    fn native_realization(&self) -> PyEffectiveProfileNativeRealization {
        PyEffectiveProfileNativeRealization::from_api(self.native_realization)
    }
}

/// One selected execution path and its exact responsibility and ceiling proof.
#[pyclass(name = "EffectiveProfileExecutionPathDetail", frozen)]
pub struct PyEffectiveProfileExecutionPathDetail {
    /// Stable execution-path selection identity.
    #[pyo3(get)]
    selection_id: String,
    /// Selected experience-path identity.
    #[pyo3(get)]
    experience_path_id: String,
    /// Domain-separated responsibility-path identity.
    #[pyo3(get)]
    responsibility_path_identity: String,
    /// Responsibility contracts consumed by the path.
    #[pyo3(get)]
    responsibility_contract_ids: Vec<String>,
    /// Ordered neutral delegation edges traversed by the path.
    #[pyo3(get)]
    work_delegation_edge_ids: Vec<String>,
    /// Provider-binding declarations required by the path.
    #[pyo3(get)]
    provider_binding_ids: Vec<String>,
    /// Portable capability ceiling.
    #[pyo3(get)]
    capability_ceiling: Vec<String>,
    effect_ceiling: Vec<pale_sdk::EffectiveProfileEffectClass>,
    terminal_provider_binding: Option<pale_sdk::EffectiveProfileProviderBindingRef>,
    native_realization: pale_sdk::EffectiveProfileNativeRealization,
    reason_steps: Vec<pale_sdk::EffectiveProfilePathReason>,
    subject: pale_sdk::PortableWorkspaceSubjectSelector,
}

#[pymethods]
impl PyEffectiveProfileExecutionPathDetail {
    /// Portable effect ceiling, without native-effect authority.
    #[getter]
    fn effect_ceiling(&self) -> Vec<PyEffectiveProfileEffectClass> {
        self.effect_ceiling
            .iter()
            .copied()
            .map(PyEffectiveProfileEffectClass::from_api)
            .collect()
    }

    /// Exact terminal binding; `None` means a native-only path without a terminal leaf.
    #[getter]
    fn terminal_provider_binding(&self) -> Option<PyEffectiveProfileProviderBindingRef> {
        self.terminal_provider_binding
            .clone()
            .map(PyEffectiveProfileProviderBindingRef::from_api)
    }

    /// Separately qualified native evidence ceiling.
    #[getter]
    fn native_realization(&self) -> PyEffectiveProfileNativeRealization {
        PyEffectiveProfileNativeRealization::from_api(self.native_realization)
    }

    /// Ordered application-owned causal selectors.
    #[getter]
    fn reason_steps(&self) -> Vec<PyEffectiveProfilePathReason> {
        self.reason_steps
            .iter()
            .cloned()
            .map(PyEffectiveProfilePathReason::from_api)
            .collect()
    }

    /// Copied composition-proof selector, not an admitted proof handle.
    #[getter]
    fn subject(&self) -> PyWorkspaceSubjectValue {
        PyWorkspaceSubjectValue::from_api(self.subject.clone())
    }
}

/// One operation and its exact private exposure state.
#[pyclass(name = "EffectiveProfileOperationDetail", frozen)]
pub struct PyEffectiveProfileOperationDetail {
    /// Logical repository containing the occurrence.
    #[pyo3(get)]
    repository_id: String,
    /// Owner-qualified Package operation identity.
    #[pyo3(get)]
    operation_id: String,
    exposure: pale_sdk::EffectiveProfileOperationExposure,
    native_realization: pale_sdk::EffectiveProfileNativeRealization,
}

#[pymethods]
impl PyEffectiveProfileOperationDetail {
    /// Operation exposure, independent of native readiness.
    #[getter]
    fn exposure(&self) -> PyOperationExposure {
        PyOperationExposure::from_api(self.exposure)
    }

    /// Separately qualified native evidence ceiling.
    #[getter]
    fn native_realization(&self) -> PyEffectiveProfileNativeRealization {
        PyEffectiveProfileNativeRealization::from_api(self.native_realization)
    }
}

/// One Package-owned capability requirement occurrence.
#[pyclass(name = "EffectiveProfileCapabilityDetail", frozen)]
pub struct PyEffectiveProfileCapabilityDetail {
    /// Portable capability identity.
    #[pyo3(get)]
    capability_id: String,
    /// Logical repository containing the occurrence.
    #[pyo3(get)]
    repository_id: String,
    /// Owner-qualified operation identity.
    #[pyo3(get)]
    operation_id: String,
    /// Requirement identity local to the operation.
    #[pyo3(get)]
    requirement_id: String,
    effect_class: pale_sdk::EffectiveProfileEffectClass,
    requirement_disposition: pale_sdk::EffectiveProfileRequirementDisposition,
    provider_disposition: pale_sdk::EffectiveProfileProviderDisposition,
    native_realization: pale_sdk::EffectiveProfileNativeRealization,
}

#[pymethods]
impl PyEffectiveProfileCapabilityDetail {
    /// Portable effect class required by this occurrence.
    #[getter]
    fn effect_class(&self) -> PyEffectiveProfileEffectClass {
        PyEffectiveProfileEffectClass::from_api(self.effect_class)
    }

    /// Configuration outcome already classified by the SDK.
    #[getter]
    fn requirement_disposition(&self) -> PyRequirementDisposition {
        PyRequirementDisposition::from_api(self.requirement_disposition)
    }

    /// Private provider selection, independently typed from the requirement.
    #[getter]
    fn provider_disposition(&self) -> PyEffectiveProfileProviderDisposition {
        PyEffectiveProfileProviderDisposition::from_api(self.provider_disposition)
    }

    /// Separately qualified native evidence ceiling.
    #[getter]
    fn native_realization(&self) -> PyEffectiveProfileNativeRealization {
        PyEffectiveProfileNativeRealization::from_api(self.native_realization)
    }
}

/// Separately qualified realization on one selected private machine binding.
#[pyclass(name = "EffectiveProfileMachineRealizationDetail", frozen)]
pub struct PyEffectiveProfileMachineRealizationDetail {
    /// Selected private machine-binding identity, not observed machine readiness.
    #[pyo3(get)]
    machine_id: String,
    realization: pale_sdk::EffectiveProfileNativeRealization,
}

#[pymethods]
impl PyEffectiveProfileMachineRealizationDetail {
    /// Current owner-native evidence ceiling.
    #[getter]
    fn realization(&self) -> PyEffectiveProfileNativeRealization {
        PyEffectiveProfileNativeRealization::from_api(self.realization)
    }
}

/// One exact application-owned gap or narrowing finding.
#[pyclass(name = "EffectiveProfileFindingDetail", frozen)]
pub struct PyEffectiveProfileFindingDetail {
    finding: pale_sdk::EffectiveProfileFinding,
}

#[pymethods]
impl PyEffectiveProfileFindingDetail {
    /// Structured finding without added presentation prose.
    #[getter]
    fn finding(&self) -> PyEffectiveProfileFinding {
        PyEffectiveProfileFinding::from_api(self.finding.clone())
    }
}

#[derive(IntoPyObject)]
pub(crate) enum PyEffectiveProfileEntryDetail {
    DeclaredSelection(PyEffectiveProfileDeclaredSelectionDetail),
    Responsibility(PyEffectiveProfileResponsibilityDetail),
    Selection(PyEffectiveProfileSelectionDetail),
    CapabilityNarrowing(PyEffectiveProfileCapabilityNarrowingDetail),
    GoverningCeiling(PyEffectiveProfileGoverningCeilingDetail),
    Repository(PyEffectiveProfileRepositoryDetail),
    ProviderRepository(PyEffectiveProfileProviderRepositoryDetail),
    ExecutionPath(PyEffectiveProfileExecutionPathDetail),
    Operation(PyEffectiveProfileOperationDetail),
    Capability(PyEffectiveProfileCapabilityDetail),
    MachineRealization(PyEffectiveProfileMachineRealizationDetail),
    Finding(PyEffectiveProfileFindingDetail),
}

impl PyEffectiveProfileEntryDetail {
    fn from_api(value: EffectiveProfileEntryDetail) -> Self {
        match value {
            EffectiveProfileEntryDetail::DeclaredSelection { selection } => {
                Self::DeclaredSelection(PyEffectiveProfileDeclaredSelectionDetail { selection })
            }
            EffectiveProfileEntryDetail::Responsibility {
                contract_id,
                execution_path_id,
                participant,
                responsibilities,
                authority_ceiling,
                reason_steps,
                subject,
            } => Self::Responsibility(PyEffectiveProfileResponsibilityDetail {
                contract_id,
                execution_path_id,
                participant,
                responsibilities,
                authority_ceiling,
                reason_steps,
                subject,
            }),
            EffectiveProfileEntryDetail::Selection { selection } => {
                Self::Selection(PyEffectiveProfileSelectionDetail { selection })
            }
            EffectiveProfileEntryDetail::CapabilityNarrowing {
                permitted_capabilities,
                forbidden_capabilities,
                permitted_effects,
                forbidden_effects,
            } => Self::CapabilityNarrowing(PyEffectiveProfileCapabilityNarrowingDetail {
                permitted_capabilities,
                forbidden_capabilities,
                permitted_effects,
                forbidden_effects,
            }),
            EffectiveProfileEntryDetail::GoverningCeiling {
                scope,
                subject_resource,
                contributors,
                policies,
                permitted_capabilities,
                forbidden_capabilities,
                permitted_effects,
                forbidden_effects,
            } => Self::GoverningCeiling(PyEffectiveProfileGoverningCeilingDetail {
                scope,
                subject_resource,
                contributors,
                policies,
                permitted_capabilities,
                forbidden_capabilities,
                permitted_effects,
                forbidden_effects,
            }),
            EffectiveProfileEntryDetail::Repository {
                repository_id,
                materialization_order,
                access_ceiling,
            } => Self::Repository(PyEffectiveProfileRepositoryDetail {
                repository_id,
                materialization_order,
                access_ceiling,
            }),
            EffectiveProfileEntryDetail::ProviderRepository {
                repository_id,
                provider_binding_id,
                native_repository_coordinate,
                native_realization,
            } => Self::ProviderRepository(PyEffectiveProfileProviderRepositoryDetail {
                repository_id,
                provider_binding_id,
                native_repository_coordinate,
                native_realization,
            }),
            EffectiveProfileEntryDetail::ExecutionPath {
                selection_id,
                experience_path_id,
                responsibility_path_identity,
                responsibility_contract_ids,
                work_delegation_edge_ids,
                provider_binding_ids,
                capability_ceiling,
                effect_ceiling,
                terminal_provider_binding,
                native_realization,
                reason_steps,
                subject,
            } => Self::ExecutionPath(PyEffectiveProfileExecutionPathDetail {
                selection_id,
                experience_path_id,
                responsibility_path_identity,
                responsibility_contract_ids,
                work_delegation_edge_ids,
                provider_binding_ids,
                capability_ceiling,
                effect_ceiling,
                terminal_provider_binding,
                native_realization,
                reason_steps,
                subject,
            }),
            EffectiveProfileEntryDetail::Operation {
                repository_id,
                operation_id,
                exposure,
                native_realization,
            } => Self::Operation(PyEffectiveProfileOperationDetail {
                repository_id,
                operation_id,
                exposure,
                native_realization,
            }),
            EffectiveProfileEntryDetail::Capability {
                capability_id,
                repository_id,
                operation_id,
                requirement_id,
                effect_class,
                requirement_disposition,
                provider_disposition,
                native_realization,
            } => Self::Capability(PyEffectiveProfileCapabilityDetail {
                capability_id,
                repository_id,
                operation_id,
                requirement_id,
                effect_class,
                requirement_disposition,
                provider_disposition,
                native_realization,
            }),
            EffectiveProfileEntryDetail::MachineRealization {
                machine_id,
                realization,
            } => Self::MachineRealization(PyEffectiveProfileMachineRealizationDetail {
                machine_id,
                realization,
            }),
            EffectiveProfileEntryDetail::Finding { finding } => {
                Self::Finding(PyEffectiveProfileFindingDetail { finding })
            }
        }
    }
}

/// One explainable SDK row, owned independently of its profile or page.
#[pyclass(name = "EffectiveProfileEntry", frozen)]
pub struct PyEffectiveProfileEntry {
    inner: EffectiveProfileEntry,
}

impl PyEffectiveProfileEntry {
    pub(crate) fn from_api(inner: EffectiveProfileEntry) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyEffectiveProfileEntry {
    /// Opaque explanation identity assigned by the SDK.
    #[getter]
    pub fn explanation_handle(&self) -> &str {
        self.inner.explanation_handle().as_str()
    }

    /// Copy the concrete payload; use `isinstance` to narrow its generated union type.
    ///
    /// Each read copies the payload. Retain the returned object when reading
    /// several fields from the same detail.
    #[getter]
    fn detail(&self) -> PyEffectiveProfileEntryDetail {
        PyEffectiveProfileEntryDetail::from_api(self.inner.detail().clone())
    }
}
