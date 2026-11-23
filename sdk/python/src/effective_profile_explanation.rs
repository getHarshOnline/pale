//! Typed evidence returned by the retained SDK explanation lookup.
//!
//! These are owned compiler results, not a second explanation engine or proof
//! of owner-native readiness. Canonical profile pages remain separate bytes.

use pale_sdk::{EffectiveProfileExplanation, EffectiveProfileExplanationEvidence};
use pyo3::prelude::*;

use crate::{
    PyEffectiveProfileDeclaredSelection, PyEffectiveProfileFinding,
    PyEffectiveProfileNativeRealization, PyEffectiveProfilePathReason,
    PyEffectiveProfileProviderDisposition, PyEffectiveProfileSelection, PyOperationExposure,
    PyRequirementDisposition,
};

/// One API-owned explanation selected by an opaque profile-local handle.
#[pyclass(name = "EffectiveProfileExplanation", frozen)]
pub struct PyEffectiveProfileExplanation {
    inner: EffectiveProfileExplanation,
}

impl PyEffectiveProfileExplanation {
    pub(crate) fn from_api(inner: EffectiveProfileExplanation) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyEffectiveProfileExplanation {
    /// Opaque profile-local explanation identity.
    #[getter]
    pub fn handle(&self) -> &str {
        self.inner.handle.as_str()
    }

    /// Copy complete evidence, independent of the compilation's lifetime.
    #[getter]
    pub(crate) fn evidence(&self) -> PyEffectiveProfileExplanationEvidence {
        PyEffectiveProfileExplanationEvidence::from_api(self.inner.evidence.clone())
    }
}

/// Exact declared intent, distinct from its effective closure.
#[pyclass(name = "EffectiveProfileDeclaredSelectionEvidence", frozen)]
pub struct PyEffectiveProfileDeclaredSelectionEvidence {
    selection: pale_sdk::EffectiveProfileDeclaredSelection,
}
#[pymethods]
impl PyEffectiveProfileDeclaredSelectionEvidence {
    /// Normalized unexpanded selection.
    #[getter]
    fn selection(&self) -> PyEffectiveProfileDeclaredSelection {
        PyEffectiveProfileDeclaredSelection::from_api(self.selection.clone())
    }
}

/// An admitted responsibility and its ordered composition proof.
#[pyclass(name = "EffectiveProfileResponsibilityEvidence", frozen)]
pub struct PyEffectiveProfileResponsibilityEvidence {
    /// Exact responsibility contract.
    #[pyo3(get)]
    contract_id: String,
    /// Selected path carrying the contract.
    #[pyo3(get)]
    execution_path_id: String,
    reason_steps: Vec<pale_sdk::EffectiveProfilePathReason>,
}
#[pymethods]
impl PyEffectiveProfileResponsibilityEvidence {
    /// Copy ordered reasons, preserving an empty selected list.
    #[getter]
    fn reason_steps(&self) -> Vec<PyEffectiveProfilePathReason> {
        self.reason_steps
            .iter()
            .cloned()
            .map(PyEffectiveProfilePathReason::from_api)
            .collect()
    }
}

/// An exact member of the admitted effective selection.
#[pyclass(name = "EffectiveProfileEffectiveSelectionEvidence", frozen)]
pub struct PyEffectiveProfileEffectiveSelectionEvidence {
    selection: pale_sdk::EffectiveProfileSelection,
}
#[pymethods]
impl PyEffectiveProfileEffectiveSelectionEvidence {
    /// Exact selected configuration coordinate.
    #[getter]
    fn selection(&self) -> PyEffectiveProfileSelection {
        PyEffectiveProfileSelection::from_api(self.selection.clone())
    }
}

/// A selected execution path and its ordered composition proof.
#[pyclass(name = "EffectiveProfileExecutionPathEvidence", frozen)]
pub struct PyEffectiveProfileExecutionPathEvidence {
    /// Stable selected-path identity.
    #[pyo3(get)]
    selection_id: String,
    reason_steps: Vec<pale_sdk::EffectiveProfilePathReason>,
}
#[pymethods]
impl PyEffectiveProfileExecutionPathEvidence {
    /// Copy ordered reasons, preserving an empty selected list.
    #[getter]
    fn reason_steps(&self) -> Vec<PyEffectiveProfilePathReason> {
        self.reason_steps
            .iter()
            .cloned()
            .map(PyEffectiveProfilePathReason::from_api)
            .collect()
    }
}

/// Independent operation exposure and native-evidence classifications.
#[pyclass(name = "EffectiveProfileOperationClassificationEvidence", frozen)]
pub struct PyEffectiveProfileOperationClassificationEvidence {
    exposure: pale_sdk::EffectiveProfileOperationExposure,
    native_realization: pale_sdk::EffectiveProfileNativeRealization,
}
#[pymethods]
impl PyEffectiveProfileOperationClassificationEvidence {
    /// Application-owned operation exposure.
    #[getter]
    fn exposure(&self) -> PyOperationExposure {
        PyOperationExposure::from_api(self.exposure)
    }
    /// Independently qualified native evidence.
    #[getter]
    fn native_realization(&self) -> PyEffectiveProfileNativeRealization {
        PyEffectiveProfileNativeRealization::from_api(self.native_realization)
    }
}

/// Independent portable, provider-selection and native-evidence outcomes.
#[pyclass(name = "EffectiveProfileCapabilityClassificationEvidence", frozen)]
pub struct PyEffectiveProfileCapabilityClassificationEvidence {
    requirement_disposition: pale_sdk::EffectiveProfileRequirementDisposition,
    provider_disposition: pale_sdk::EffectiveProfileProviderDisposition,
    native_realization: pale_sdk::EffectiveProfileNativeRealization,
}
#[pymethods]
impl PyEffectiveProfileCapabilityClassificationEvidence {
    /// Exact portable requirement outcome.
    #[getter]
    fn requirement_disposition(&self) -> PyRequirementDisposition {
        PyRequirementDisposition::from_api(self.requirement_disposition)
    }
    /// Private provider selection, not native readiness.
    #[getter]
    fn provider_disposition(&self) -> PyEffectiveProfileProviderDisposition {
        PyEffectiveProfileProviderDisposition::from_api(self.provider_disposition)
    }
    /// Independently qualified native evidence.
    #[getter]
    fn native_realization(&self) -> PyEffectiveProfileNativeRealization {
        PyEffectiveProfileNativeRealization::from_api(self.native_realization)
    }
}

/// One exact application-owned finding, without added inference.
#[pyclass(name = "EffectiveProfileFindingClassificationEvidence", frozen)]
pub struct PyEffectiveProfileFindingClassificationEvidence {
    finding: pale_sdk::EffectiveProfileFinding,
}
#[pymethods]
impl PyEffectiveProfileFindingClassificationEvidence {
    /// Copy the structured finding.
    #[getter]
    fn finding(&self) -> PyEffectiveProfileFinding {
        PyEffectiveProfileFinding::from_api(self.finding.clone())
    }
}

/// Separately qualified owner-native realization state.
#[pyclass(name = "EffectiveProfileMachineRealizationEvidence", frozen)]
pub struct PyEffectiveProfileMachineRealizationEvidence {
    realization: pale_sdk::EffectiveProfileNativeRealization,
}
#[pymethods]
impl PyEffectiveProfileMachineRealizationEvidence {
    /// Independent native-evidence state.
    #[getter]
    fn realization(&self) -> PyEffectiveProfileNativeRealization {
        PyEffectiveProfileNativeRealization::from_api(self.realization)
    }
}

/// Retained configuration evidence requiring no second inference.
#[pyclass(name = "EffectiveProfileConfigurationEvidence", frozen)]
pub struct PyEffectiveProfileConfigurationEvidence {}

#[derive(IntoPyObject)]
pub(crate) enum PyEffectiveProfileExplanationEvidence {
    #[pyo3(transparent)]
    DeclaredSelection(PyEffectiveProfileDeclaredSelectionEvidence),
    #[pyo3(transparent)]
    Responsibility(PyEffectiveProfileResponsibilityEvidence),
    #[pyo3(transparent)]
    EffectiveSelection(PyEffectiveProfileEffectiveSelectionEvidence),
    #[pyo3(transparent)]
    ExecutionPath(PyEffectiveProfileExecutionPathEvidence),
    #[pyo3(transparent)]
    OperationClassification(PyEffectiveProfileOperationClassificationEvidence),
    #[pyo3(transparent)]
    CapabilityClassification(PyEffectiveProfileCapabilityClassificationEvidence),
    #[pyo3(transparent)]
    FindingClassification(PyEffectiveProfileFindingClassificationEvidence),
    #[pyo3(transparent)]
    MachineRealization(PyEffectiveProfileMachineRealizationEvidence),
    #[pyo3(transparent)]
    Configuration(PyEffectiveProfileConfigurationEvidence),
}

impl PyEffectiveProfileExplanationEvidence {
    pub(crate) fn from_api(value: EffectiveProfileExplanationEvidence) -> Self {
        match value {
            EffectiveProfileExplanationEvidence::DeclaredSelection { selection } => {
                Self::DeclaredSelection(PyEffectiveProfileDeclaredSelectionEvidence { selection })
            }
            EffectiveProfileExplanationEvidence::Responsibility {
                contract_id,
                execution_path_id,
                reason_steps,
            } => Self::Responsibility(PyEffectiveProfileResponsibilityEvidence {
                contract_id,
                execution_path_id,
                reason_steps,
            }),
            EffectiveProfileExplanationEvidence::EffectiveSelection { selection } => {
                Self::EffectiveSelection(PyEffectiveProfileEffectiveSelectionEvidence { selection })
            }
            EffectiveProfileExplanationEvidence::ExecutionPath {
                selection_id,
                reason_steps,
            } => Self::ExecutionPath(PyEffectiveProfileExecutionPathEvidence {
                selection_id,
                reason_steps,
            }),
            EffectiveProfileExplanationEvidence::OperationClassification {
                exposure,
                native_realization,
            } => Self::OperationClassification(PyEffectiveProfileOperationClassificationEvidence {
                exposure,
                native_realization,
            }),
            EffectiveProfileExplanationEvidence::CapabilityClassification {
                requirement_disposition,
                provider_disposition,
                native_realization,
            } => {
                Self::CapabilityClassification(PyEffectiveProfileCapabilityClassificationEvidence {
                    requirement_disposition,
                    provider_disposition,
                    native_realization,
                })
            }
            EffectiveProfileExplanationEvidence::FindingClassification { finding } => {
                Self::FindingClassification(PyEffectiveProfileFindingClassificationEvidence {
                    finding,
                })
            }
            EffectiveProfileExplanationEvidence::MachineRealization { realization } => {
                Self::MachineRealization(PyEffectiveProfileMachineRealizationEvidence {
                    realization,
                })
            }
            EffectiveProfileExplanationEvidence::Configuration => {
                Self::Configuration(PyEffectiveProfileConfigurationEvidence {})
            }
        }
    }
}
