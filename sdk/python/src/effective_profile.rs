//! Python-native handles for API-owned effective-profile inspection.
//!
//! [`pale_sdk::EffectiveProfileCompilation`] owns the admitted profile and
//! page algebra. This leaf retains that opaque handle, copies typed rows and
//! forwards exact canonical page bytes while Python is detached.

use std::sync::Arc;

use pale_sdk::{
    EffectiveProfileCompilation, EffectiveProfileCompilationRequest,
    EffectiveProfileExplanationHandle, EffectiveProfilePageRequest, EffectiveProfileSection,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::error::diagnostic_error;
use crate::{
    PalePython, PyCompilationDocument, PyEffectiveProfileEntry, PyEffectiveProfileExplanation,
    PyOneShotPrivateInputs, PyPortableWorkspaceCompilation,
};

/// Complete captured input required after portable workspace admission.
#[pyclass(name = "EffectiveProfileCompilationRequest", from_py_object)]
#[derive(Clone)]
pub struct PyEffectiveProfileCompilationRequest {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    private_inputs: PyOneShotPrivateInputs,
    package_documents: Vec<PyCompilationDocument>,
}

#[pymethods]
impl PyEffectiveProfileCompilationRequest {
    /// Capture exact selectors, fixed-role private bytes, and Package documents.
    #[new]
    #[pyo3(signature = (*, project_id, execution_workspace_id, machine_id, private_inputs, package_documents))]
    pub fn new(
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
        private_inputs: PyOneShotPrivateInputs,
        package_documents: Vec<PyCompilationDocument>,
    ) -> Self {
        Self {
            project_id,
            execution_workspace_id,
            machine_id,
            private_inputs,
            package_documents,
        }
    }
}

impl PyEffectiveProfileCompilationRequest {
    /// Consume Python-owned values into the one API-owned request.
    fn into_api(self) -> EffectiveProfileCompilationRequest {
        EffectiveProfileCompilationRequest::new(
            self.project_id,
            self.execution_workspace_id,
            self.machine_id,
            self.private_inputs.into_api(),
            self.package_documents
                .into_iter()
                .map(PyCompilationDocument::into_api)
                .collect(),
        )
    }
}

/// Opaque admitted profile phase; Python collection has no semantic effect.
#[pyclass(name = "EffectiveProfileCompilation", frozen)]
pub struct PyEffectiveProfileCompilation {
    pub(crate) inner: Arc<EffectiveProfileCompilation>,
}

#[pymethods]
impl PyEffectiveProfileCompilation {
    /// Principal workspace owning the effective profile.
    #[getter]
    pub fn workspace_id(&self) -> &str {
        self.inner.workspace_id()
    }

    /// Selected logical project.
    #[getter]
    pub fn project_id(&self) -> &str {
        self.inner.project_id()
    }

    /// Selected logical execution workspace.
    #[getter]
    pub fn execution_workspace_id(&self) -> &str {
        self.inner.execution_workspace_id()
    }

    /// Canonical identity of the complete portable profile.
    #[getter]
    pub fn profile_digest(&self) -> &str {
        self.inner.profile_digest()
    }

    /// Canonical identity of the admitted private machine projection.
    ///
    /// This is a compilation coordinate, not machine-readiness evidence.
    #[getter]
    pub fn instance_digest(&self) -> &str {
        self.inner.instance_digest()
    }

    /// Identity of the admitted Package semantic closure.
    #[getter]
    pub fn package_semantic_ir_digest(&self) -> &str {
        self.inner.package_semantic_ir_digest()
    }

    /// Identity of the neutral composition proof.
    #[getter]
    pub fn composition_plan_digest(&self) -> &str {
        self.inner.composition_plan_digest()
    }

    /// Return the exact semantic coordinate batch shared by every page.
    pub fn coordinates(&self, py: Python<'_>) -> PyEffectiveProfileCoordinates {
        let compilation = Arc::clone(&self.inner);
        py.detach(move || {
            let coordinates = compilation.coordinates();
            PyEffectiveProfileCoordinates {
                workspace_id: coordinates.workspace_id,
                project_id: coordinates.project_id,
                execution_workspace_id: coordinates.execution_workspace_id,
                machine_id: coordinates.machine_id,
                profile_digest: coordinates.profile_digest,
                instance_digest: coordinates.instance_digest,
                selected_topology_digest: coordinates.selected_topology_digest,
                package_semantic_ir_digest: coordinates.package_semantic_ir_digest,
                composition_plan_digest: coordinates.composition_plan_digest,
                workspace_semantic_ir_digest: coordinates.workspace_semantic_ir_digest,
                workspace_admission_evidence_digest: coordinates
                    .workspace_admission_evidence_digest,
            }
        })
    }

    /// Project one bounded page without reparsing or recompiling.
    pub fn inspect(
        &self,
        py: Python<'_>,
        section: PyEffectiveProfileSection,
        offset: u32,
        limit: u16,
    ) -> PyEffectiveProfilePage {
        let compilation = Arc::clone(&self.inner);
        py.detach(move || {
            let page = compilation.inspect(EffectiveProfilePageRequest::new(
                section.into(),
                offset,
                limit,
            ));
            let entry_count = u32::from(page.entry_count());
            PyEffectiveProfilePage {
                section: page.section().into(),
                offset: page.offset(),
                entry_count,
                next_offset: page.next_offset(),
                entries: page.entries().to_vec(),
                canonical_bytes: page.canonical_bytes().to_vec(),
            }
        })
    }

    /// Resolve one exact retained explanation handle.
    ///
    /// `None` means the valid handle belongs to another compilation. This
    /// method never reparses source or recomputes semantic closure.
    ///
    /// # Errors
    ///
    /// Raises a Python value error when `handle` is not a lowercase SHA-256.
    pub fn explain(
        &self,
        py: Python<'_>,
        handle: String,
    ) -> PyResult<Option<PyEffectiveProfileExplanation>> {
        let handle = EffectiveProfileExplanationHandle::parse(handle)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        let compilation = Arc::clone(&self.inner);
        Ok(py
            .detach(move || compilation.explain(&handle))
            .map(PyEffectiveProfileExplanation::from_api))
    }

    /// Join one compiler-resolved authoring identity to retained profile rows.
    pub fn inspect_effective_authoring(
        &self,
        py: Python<'_>,
        identity: &crate::PyAuthoringIdentity,
    ) -> PyEffectiveAuthoringInspection {
        let compilation = Arc::clone(&self.inner);
        let identity = identity.inner.clone();
        py.detach(move || {
            PyEffectiveAuthoringInspection::from_api(
                compilation.inspect_effective_authoring(&identity),
            )
        })
    }

    /// Page associated profile rows without reparsing or recompiling.
    ///
    /// Zero selects 64 rows; larger limits are capped at 256. Keep the raw
    /// limit unchanged when resuming, including the distinction between 0 and
    /// 64. The token requires this retained profile and the full identity.
    /// This is a row limit, not a byte limit or disclosure permission.
    ///
    /// # Errors
    /// Invalid Python limits raise `TypeError` or `OverflowError`. SDK
    /// continuation rejection raises [`crate::PaleDiagnosticError`].
    #[pyo3(signature = (identity, limit: "int", continuation=None))]
    pub fn inspect_effective_authoring_page(
        &self,
        py: Python<'_>,
        identity: &crate::PyAuthoringIdentity,
        #[pyo3(from_py_with = crate::authoring::page_limit)] limit: u16,
        continuation: Option<&PyEffectiveAuthoringContinuation>,
    ) -> PyResult<PyEffectiveAuthoringPage> {
        let compilation = Arc::clone(&self.inner);
        let identity = identity.inner.clone();
        let continuation = continuation.map(|token| token.inner.clone());
        py.detach(move || {
            compilation.inspect_effective_authoring_page(&identity, continuation.as_ref(), limit)
        })
        .map(|inner| PyEffectiveAuthoringPage { inner })
        .map_err(|error| diagnostic_error(py, error))
    }
}

/// Resume one authoring-to-profile query on the same retained projection.
///
/// The token cannot be constructed from coordinates or serialized. It retains
/// the shared Rust projection until Python releases it; it grants no effect.
#[pyclass(name = "EffectiveAuthoringContinuation", frozen)]
pub struct PyEffectiveAuthoringContinuation {
    inner: pale_sdk::EffectiveAuthoringContinuation,
}

/// Owned profile rows and an optional continuation for the same association.
#[pyclass(name = "EffectiveAuthoringPage", frozen)]
pub struct PyEffectiveAuthoringPage {
    inner: pale_sdk::EffectiveAuthoringPage,
}

#[pymethods]
impl PyEffectiveAuthoringPage {
    /// Complete profile coordinate, including the private instance identity.
    #[getter]
    pub fn coordinates(&self) -> PyEffectiveProfileCoordinates {
        PyEffectiveProfileCoordinates::from_api(self.inner.coordinates().clone())
    }

    /// Existing profile rows, with their unchanged explanation handles.
    #[getter]
    pub fn entries(&self) -> Vec<PyEffectiveProfileEntry> {
        self.inner
            .entries()
            .iter()
            .cloned()
            .map(PyEffectiveProfileEntry::from_api)
            .collect()
    }

    /// Token for later associated rows, or `None` when the result is complete.
    #[getter]
    pub fn continuation(&self) -> Option<PyEffectiveAuthoringContinuation> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| PyEffectiveAuthoringContinuation { inner })
    }
}

/// Bounded join from one authoring identity to existing profile rows.
#[pyclass(name = "EffectiveAuthoringInspection", frozen)]
pub struct PyEffectiveAuthoringInspection {
    coordinates: PyEffectiveProfileCoordinates,
    entries: Vec<pale_sdk::EffectiveProfileEntry>,
    truncated: bool,
}

impl PyEffectiveAuthoringInspection {
    fn from_api(inspection: pale_sdk::EffectiveAuthoringInspection) -> Self {
        Self {
            coordinates: PyEffectiveProfileCoordinates::from_api(inspection.coordinates().clone()),
            entries: inspection.entries().to_vec(),
            truncated: inspection.truncated(),
        }
    }
}

#[pymethods]
impl PyEffectiveAuthoringInspection {
    /// Exact admitted profile coordinate owning every associated row.
    #[getter]
    pub fn coordinates(&self) -> PyEffectiveProfileCoordinates {
        self.coordinates.clone()
    }

    /// Complete associated rows in deterministic projection order.
    #[getter]
    pub fn entries(&self) -> Vec<PyEffectiveProfileEntry> {
        self.entries
            .iter()
            .cloned()
            .map(PyEffectiveProfileEntry::from_api)
            .collect()
    }

    /// Whether the fixed association ceiling omitted later rows.
    #[getter]
    pub fn truncated(&self) -> bool {
        self.truncated
    }
}

/// Closed Profile Explorer section.
#[pyclass(name = "EffectiveProfileSection", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyEffectiveProfileSection {
    /// Declared responsibilities and selected path evidence.
    IntendedUseCase,
    /// Effective releases, policy, repositories, and semantic selections.
    EffectiveConfiguration,
    /// Operations and independently typed capability requirements.
    CapabilitiesAndOperations,
    /// Separately qualified owner-native realization.
    MachineRealization,
    /// Exact gaps, narrowing, and policy findings.
    GapsAndExplanations,
}

impl From<PyEffectiveProfileSection> for EffectiveProfileSection {
    fn from(value: PyEffectiveProfileSection) -> Self {
        match value {
            PyEffectiveProfileSection::IntendedUseCase => Self::IntendedUseCase,
            PyEffectiveProfileSection::EffectiveConfiguration => Self::EffectiveConfiguration,
            PyEffectiveProfileSection::CapabilitiesAndOperations => Self::CapabilitiesAndOperations,
            PyEffectiveProfileSection::MachineRealization => Self::MachineRealization,
            PyEffectiveProfileSection::GapsAndExplanations => Self::GapsAndExplanations,
        }
    }
}

impl From<EffectiveProfileSection> for PyEffectiveProfileSection {
    fn from(value: EffectiveProfileSection) -> Self {
        match value {
            EffectiveProfileSection::IntendedUseCase => Self::IntendedUseCase,
            EffectiveProfileSection::EffectiveConfiguration => Self::EffectiveConfiguration,
            EffectiveProfileSection::CapabilitiesAndOperations => Self::CapabilitiesAndOperations,
            EffectiveProfileSection::MachineRealization => Self::MachineRealization,
            EffectiveProfileSection::GapsAndExplanations => Self::GapsAndExplanations,
        }
    }
}

/// Exact semantic coordinates for one admitted profile inspection.
#[pyclass(name = "EffectiveProfileCoordinates", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct PyEffectiveProfileCoordinates {
    /// Principal workspace identity.
    #[pyo3(get)]
    workspace_id: String,
    /// Logical project identity.
    #[pyo3(get)]
    project_id: String,
    /// Logical execution-workspace identity.
    #[pyo3(get)]
    execution_workspace_id: String,
    /// Selected private machine-binding identity.
    #[pyo3(get)]
    machine_id: String,
    /// Canonical effective-profile identity.
    #[pyo3(get)]
    profile_digest: String,
    /// Canonical admitted private machine-projection identity.
    #[pyo3(get)]
    instance_digest: String,
    /// Identity of the exact project-relative desired topology selection.
    #[pyo3(get)]
    selected_topology_digest: String,
    /// Admitted Package semantic-IR identity.
    #[pyo3(get)]
    package_semantic_ir_digest: String,
    /// Neutral composition-proof identity.
    #[pyo3(get)]
    composition_plan_digest: String,
    /// Admitted workspace semantic-IR identity.
    #[pyo3(get)]
    workspace_semantic_ir_digest: String,
    /// Owner-proof set identity used for workspace admission.
    #[pyo3(get)]
    workspace_admission_evidence_digest: String,
}

impl PyEffectiveProfileCoordinates {
    pub(crate) fn from_api(coordinates: pale_sdk::EffectiveProfileCoordinates) -> Self {
        Self {
            workspace_id: coordinates.workspace_id,
            project_id: coordinates.project_id,
            execution_workspace_id: coordinates.execution_workspace_id,
            machine_id: coordinates.machine_id,
            profile_digest: coordinates.profile_digest,
            instance_digest: coordinates.instance_digest,
            selected_topology_digest: coordinates.selected_topology_digest,
            package_semantic_ir_digest: coordinates.package_semantic_ir_digest,
            composition_plan_digest: coordinates.composition_plan_digest,
            workspace_semantic_ir_digest: coordinates.workspace_semantic_ir_digest,
            workspace_admission_evidence_digest: coordinates.workspace_admission_evidence_digest,
        }
    }
}

/// One exact API-owned bounded page.
#[pyclass(name = "EffectiveProfilePage", frozen)]
pub struct PyEffectiveProfilePage {
    /// Section represented by the page.
    #[pyo3(get)]
    section: PyEffectiveProfileSection,
    /// Exact zero-based offset represented by the page.
    #[pyo3(get)]
    offset: u32,
    /// Number of complete entries in the page.
    #[pyo3(get)]
    entry_count: u32,
    /// Next zero-based offset, or `None` for the terminal page.
    #[pyo3(get)]
    next_offset: Option<u32>,
    /// SDK-owned entries retained independently of the compilation handle.
    entries: Vec<pale_sdk::EffectiveProfileEntry>,
    /// Exact RFC 8785 bytes produced once by `pale-sdk`.
    canonical_bytes: Vec<u8>,
}

#[pymethods]
impl PyEffectiveProfilePage {
    /// Complete rows in the same order as the canonical page.
    #[getter]
    pub fn entries(&self) -> Vec<PyEffectiveProfileEntry> {
        self.entries
            .iter()
            .cloned()
            .map(PyEffectiveProfileEntry::from_api)
            .collect()
    }

    /// Copy exact RFC 8785 bytes produced once by `pale-sdk`.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, &self.canonical_bytes)
    }
}

#[pymethods]
impl PalePython {
    /// Compile one selected effective profile over an admitted workspace.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] whose message is the canonical
    /// summary and whose `diagnostic` attribute carries exact structured bytes.
    pub fn compile_effective_profile(
        &self,
        py: Python<'_>,
        workspace: &PyPortableWorkspaceCompilation,
        request: PyEffectiveProfileCompilationRequest,
    ) -> PyResult<PyEffectiveProfileCompilation> {
        let api = Arc::clone(&self.api);
        let workspace = Arc::clone(&workspace.inner);
        let request = request.into_api();
        py.detach(move || api.compile_effective_profile(&workspace, request))
            .map(|inner| PyEffectiveProfileCompilation {
                inner: Arc::new(inner),
            })
            .map_err(|error| diagnostic_error(py, error))
    }
}
