//! Native-context Python projections over API-owned one-shot use cases.

use std::sync::{Arc, Mutex};

use pale_sdk::{
    CompilationDocument, MachineLocalEffectCandidate, OneShotAdmittedPlan, OneShotBaselineEntry,
    OneShotBaselineObservation, OneShotBoundPlan, OneShotCompilation, OneShotCompilationRequest,
    OneShotPlanCandidate, OneShotPlanKind, OneShotPlanningRequest, OneShotPriorMaterialization,
    OneShotPrivateInputs, PortableWorkspaceDocument, PortableWorkspaceRequest,
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::error::diagnostic_error;
use crate::PalePython;

/// Closed effect-free repository-plan operation.
#[pyclass(name = "OneShotPlanKind", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyOneShotPlanKind {
    /// Reconcile desired surfaces with prior ownership.
    Apply,
    /// Explicitly adopt matching pre-existing targets.
    Adopt,
    /// Remove only previously owned surfaces.
    Remove,
}

impl From<PyOneShotPlanKind> for OneShotPlanKind {
    fn from(value: PyOneShotPlanKind) -> Self {
        match value {
            PyOneShotPlanKind::Apply => Self::Apply,
            PyOneShotPlanKind::Adopt => Self::Adopt,
            PyOneShotPlanKind::Remove => Self::Remove,
        }
    }
}

impl From<OneShotPlanKind> for PyOneShotPlanKind {
    fn from(value: OneShotPlanKind) -> Self {
        match value {
            OneShotPlanKind::Apply => Self::Apply,
            OneShotPlanKind::Adopt => Self::Adopt,
            OneShotPlanKind::Remove => Self::Remove,
        }
    }
}

/// Opaque admitted one-shot compilation; Python collection has no effect.
#[pyclass(name = "OneShotCompilation", frozen)]
pub struct PyOneShotCompilation {
    inner: Arc<OneShotCompilation>,
}

/// One owned labelled Package or lifecycle document.
#[pyclass(name = "CompilationDocument", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyCompilationDocument {
    label: String,
    bytes: Vec<u8>,
}

#[pymethods]
impl PyCompilationDocument {
    /// Capture one logical label and complete document bytes.
    #[new]
    pub fn new(label: String, bytes: Vec<u8>) -> Self {
        Self { label, bytes }
    }
}

impl PyCompilationDocument {
    pub(crate) fn into_api(self) -> CompilationDocument {
        CompilationDocument::new(self.label, self.bytes)
    }
}

/// Fixed-role private inputs for one one-shot compilation.
#[pyclass(name = "OneShotPrivateInputs", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyOneShotPrivateInputs {
    capabilities: PyCompilationDocument,
    machine_binding: PyCompilationDocument,
    distribution_lock: PyCompilationDocument,
    lifecycle_guidance: Vec<PyCompilationDocument>,
}

#[pymethods]
impl PyOneShotPrivateInputs {
    /// Bind complete bytes to the three singular roles and guidance batch.
    #[new]
    #[pyo3(signature = (*, capabilities, machine_binding, distribution_lock, lifecycle_guidance))]
    pub fn new(
        capabilities: PyCompilationDocument,
        machine_binding: PyCompilationDocument,
        distribution_lock: PyCompilationDocument,
        lifecycle_guidance: Vec<PyCompilationDocument>,
    ) -> Self {
        Self {
            capabilities,
            machine_binding,
            distribution_lock,
            lifecycle_guidance,
        }
    }
}

impl PyOneShotPrivateInputs {
    pub(crate) fn into_api(self) -> OneShotPrivateInputs {
        OneShotPrivateInputs::new(
            self.capabilities.into_api(),
            self.machine_binding.into_api(),
            self.distribution_lock.into_api(),
            self.lifecycle_guidance
                .into_iter()
                .map(PyCompilationDocument::into_api)
                .collect(),
        )
    }
}

/// Complete path-free workspace source family for one one-shot request.
#[pyclass(name = "OneShotWorkspaceInputs", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyOneShotWorkspaceInputs {
    root_manifest: String,
    documents: Vec<PyCompilationDocument>,
}

#[pymethods]
impl PyOneShotWorkspaceInputs {
    /// Capture one root manifest and its complete labelled document batch.
    #[new]
    pub fn new(root_manifest: String, documents: Vec<PyCompilationDocument>) -> Self {
        Self {
            root_manifest,
            documents,
        }
    }
}

impl PyOneShotWorkspaceInputs {
    pub(crate) fn into_api(self) -> PortableWorkspaceRequest {
        PortableWorkspaceRequest::new(
            self.root_manifest,
            self.documents
                .into_iter()
                .map(|document| PortableWorkspaceDocument::new(document.label, document.bytes))
                .collect(),
        )
    }
}

/// Cohesive complete request for one effect-free one-shot compilation.
#[pyclass(name = "OneShotCompilationRequest", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyOneShotCompilationRequest {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    private_inputs: PyOneShotPrivateInputs,
    package_documents: Vec<PyCompilationDocument>,
    workspace: PyOneShotWorkspaceInputs,
}

#[pymethods]
impl PyOneShotCompilationRequest {
    /// Assemble already-owned source families without parsing them.
    #[new]
    #[pyo3(signature = (*, project_id, execution_workspace_id, machine_id, private_inputs, package_documents, workspace))]
    pub fn new(
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
        private_inputs: PyOneShotPrivateInputs,
        package_documents: Vec<PyCompilationDocument>,
        workspace: PyOneShotWorkspaceInputs,
    ) -> Self {
        Self {
            project_id,
            execution_workspace_id,
            machine_id,
            private_inputs,
            package_documents,
            workspace,
        }
    }
}

impl PyOneShotCompilationRequest {
    fn into_api(self) -> OneShotCompilationRequest {
        OneShotCompilationRequest::new(
            self.project_id,
            self.execution_workspace_id,
            self.machine_id,
            self.private_inputs.into_api(),
            self.package_documents
                .into_iter()
                .map(PyCompilationDocument::into_api)
                .collect(),
            self.workspace.into_api(),
        )
    }
}

/// Binding-safe identity and cardinality summary of one admitted compilation.
#[pyclass(name = "OneShotSummary", frozen)]
pub struct PyOneShotSummary {
    project_id: String,
    execution_workspace_id: String,
    profile_digest: String,
    instance_digest: String,
    one_shot_compilation_digest: String,
    schema_manifest_sha256: String,
    schema_set_sha256: String,
    repository_ids: Vec<String>,
    canonical_bytes: Vec<u8>,
}

impl PyOneShotSummary {
    /// Project API-owned scalars without exposing an application type.
    pub(crate) fn from_api(summary: pale_sdk::OneShotCompilationSummary) -> Self {
        Self {
            project_id: summary.project_id().to_owned(),
            execution_workspace_id: summary.execution_workspace_id().to_owned(),
            profile_digest: summary.profile_digest().to_owned(),
            instance_digest: summary.instance_digest().to_owned(),
            one_shot_compilation_digest: summary.one_shot_compilation_digest().to_owned(),
            schema_manifest_sha256: summary.schema_manifest_sha256().to_owned(),
            schema_set_sha256: summary.schema_set_sha256().to_owned(),
            repository_ids: summary
                .repositories()
                .iter()
                .map(|repository| repository.repository_id().to_owned())
                .collect(),
            canonical_bytes: summary.canonical_bytes().to_vec(),
        }
    }
}

#[pymethods]
impl PyOneShotSummary {
    /// Canonical project identity.
    #[getter]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Canonical execution-workspace identity.
    #[getter]
    pub fn execution_workspace_id(&self) -> &str {
        &self.execution_workspace_id
    }

    /// Effective profile digest.
    #[getter]
    pub fn profile_digest(&self) -> &str {
        &self.profile_digest
    }

    /// Complete private-instance digest.
    #[getter]
    pub fn instance_digest(&self) -> &str {
        &self.instance_digest
    }

    /// Post-render one-shot output digest.
    #[getter]
    pub fn one_shot_compilation_digest(&self) -> &str {
        &self.one_shot_compilation_digest
    }

    /// Descriptor-manifest digest.
    #[getter]
    pub fn schema_manifest_sha256(&self) -> &str {
        &self.schema_manifest_sha256
    }

    /// Raw-schema-set digest.
    #[getter]
    pub fn schema_set_sha256(&self) -> &str {
        &self.schema_set_sha256
    }

    /// Stable repository identities; the length is the repository count.
    #[getter]
    pub fn repository_ids(&self) -> Vec<String> {
        self.repository_ids.clone()
    }

    /// API-owned canonical JSON containing every typed summary field.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, &self.canonical_bytes)
    }
}

/// Canonical effect-free plan candidate and its transition counts.
#[pyclass(name = "OneShotPlanCandidate", frozen)]
pub struct PyOneShotPlanCandidate {
    inner: Mutex<Option<OneShotPlanCandidate>>,
    kind: PyOneShotPlanKind,
    repository_id: String,
    plan_digest: String,
    one_shot_compilation_digest: String,
    operation_count: u32,
    deferred_effect_count: u32,
    canonical_bytes: Vec<u8>,
}

impl PyOneShotPlanCandidate {
    fn from_api(inner: OneShotPlanCandidate) -> Self {
        Self {
            kind: inner.kind().into(),
            repository_id: inner.repository_id().to_owned(),
            plan_digest: inner.plan_digest().to_owned(),
            one_shot_compilation_digest: inner.one_shot_compilation_digest().to_owned(),
            operation_count: inner.operation_count(),
            deferred_effect_count: inner.deferred_effect_count(),
            canonical_bytes: inner.canonical_bytes().to_vec(),
            inner: Mutex::new(Some(inner)),
        }
    }

    fn take(&self) -> PyResult<OneShotPlanCandidate> {
        self.inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("one-shot plan-candidate handle is poisoned"))?
            .take()
            .ok_or_else(|| {
                PyValueError::new_err(
                    "one-shot plan candidate was already consumed by machine-effect binding",
                )
            })
    }
}

#[pymethods]
impl PyOneShotPlanCandidate {
    /// Requested planning operation.
    #[getter]
    pub fn kind(&self) -> PyOneShotPlanKind {
        self.kind
    }

    /// Selected logical repository.
    #[getter]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Canonical identity of the complete candidate.
    #[getter]
    pub fn plan_digest(&self) -> &str {
        &self.plan_digest
    }

    /// One-shot output identity consumed by planning.
    #[getter]
    pub fn one_shot_compilation_digest(&self) -> &str {
        &self.one_shot_compilation_digest
    }

    /// Number of planned repository-surface transitions.
    #[getter]
    pub fn operation_count(&self) -> u32 {
        self.operation_count
    }

    /// Number of declared host effects intentionally excluded from the plan.
    #[getter]
    pub fn deferred_effect_count(&self) -> u32 {
        self.deferred_effect_count
    }

    /// Exact canonical `agent-surface-plan.v1` bytes.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, &self.canonical_bytes)
    }
}

/// One inert canonical machine-local observation supplied for plan binding.
#[pyclass(name = "MachineLocalEffectCandidate", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyMachineLocalEffectCandidate {
    canonical_bytes: Vec<u8>,
}

#[pymethods]
impl PyMachineLocalEffectCandidate {
    /// Capture exact canonical `machine-local-effect.v1` bytes.
    #[new]
    pub fn new(canonical_bytes: Vec<u8>) -> Self {
        Self { canonical_bytes }
    }
}

impl PyMachineLocalEffectCandidate {
    fn into_api(self) -> MachineLocalEffectCandidate {
        MachineLocalEffectCandidate::new(self.canonical_bytes)
    }
}

/// Complete plan after machine-local observations are bound.
///
/// The handle remains a candidate until consumed exactly once by saved-plan
/// admission. Reading its canonical bytes grants no mutation authority.
#[pyclass(name = "OneShotBoundPlan", frozen)]
pub struct PyOneShotBoundPlan {
    inner: Mutex<Option<OneShotBoundPlan>>,
    kind: PyOneShotPlanKind,
    repository_id: String,
    canonical_bytes: Vec<u8>,
}

impl PyOneShotBoundPlan {
    fn from_api(inner: OneShotBoundPlan) -> Self {
        Self {
            kind: inner.kind().into(),
            repository_id: inner.repository_id().to_owned(),
            canonical_bytes: inner.canonical_bytes().to_vec(),
            inner: Mutex::new(Some(inner)),
        }
    }

    fn take(&self) -> PyResult<OneShotBoundPlan> {
        self.inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("one-shot bound-plan handle is poisoned"))?
            .take()
            .ok_or_else(|| {
                PyValueError::new_err(
                    "one-shot bound plan was already consumed by saved-plan admission",
                )
            })
    }
}

#[pymethods]
impl PyOneShotBoundPlan {
    /// Requested planning operation.
    #[getter]
    pub fn kind(&self) -> PyOneShotPlanKind {
        self.kind
    }

    /// Selected logical repository.
    #[getter]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Exact canonical complete materialization-plan bytes.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, &self.canonical_bytes)
    }
}

/// Saved-plan equality proof retained as an opaque API capability.
#[pyclass(name = "OneShotAdmittedPlan", frozen)]
pub struct PyOneShotAdmittedPlan {
    inner: OneShotAdmittedPlan,
}

#[pymethods]
impl PyOneShotAdmittedPlan {
    /// Admitted operation class.
    #[getter]
    pub fn kind(&self) -> PyOneShotPlanKind {
        self.inner.kind().into()
    }

    /// Selected logical repository.
    #[getter]
    pub fn repository_id(&self) -> &str {
        self.inner.repository_id()
    }

    /// Digest of the admitted complete materialization plan.
    #[getter]
    pub fn plan_digest(&self) -> &str {
        self.inner.plan_digest()
    }
}

#[derive(Clone)]
enum PyPriorState {
    Unowned,
    Owned(Vec<u8>),
}

/// Explicit closed prior-ownership claim.
#[pyclass(name = "OneShotPriorMaterialization", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyOneShotPriorMaterialization {
    state: PyPriorState,
}

#[pymethods]
impl PyOneShotPriorMaterialization {
    /// Construct an explicit unowned claim.
    #[staticmethod]
    pub fn unowned() -> Self {
        Self {
            state: PyPriorState::Unowned,
        }
    }

    /// Construct an explicit owned claim from canonical state bytes.
    #[staticmethod]
    pub fn owned(canonical_state: Vec<u8>) -> Self {
        Self {
            state: PyPriorState::Owned(canonical_state),
        }
    }
}

impl PyOneShotPriorMaterialization {
    fn into_api(self) -> OneShotPriorMaterialization {
        match self.state {
            PyPriorState::Unowned => OneShotPriorMaterialization::Unowned,
            PyPriorState::Owned(canonical_state) => {
                OneShotPriorMaterialization::Owned { canonical_state }
            }
        }
    }
}

#[derive(Clone)]
enum PyBaselineState {
    Absent,
    Present { sha256: String, unix_mode: u32 },
}

/// One explicit closed baseline observation bound to a relative target.
#[pyclass(name = "BaselineEntry", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyBaselineEntry {
    relative_path: String,
    state: PyBaselineState,
}

#[pymethods]
impl PyBaselineEntry {
    /// Construct an explicit absent-file observation.
    #[staticmethod]
    pub fn absent(relative_path: String) -> Self {
        Self {
            relative_path,
            state: PyBaselineState::Absent,
        }
    }

    /// Construct an explicit present-file observation.
    #[staticmethod]
    pub fn present(relative_path: String, sha256: String, unix_mode: u32) -> Self {
        Self {
            relative_path,
            state: PyBaselineState::Present { sha256, unix_mode },
        }
    }
}

impl PyBaselineEntry {
    fn into_api(self) -> OneShotBaselineEntry {
        let observation = match self.state {
            PyBaselineState::Absent => OneShotBaselineObservation::Absent,
            PyBaselineState::Present { sha256, unix_mode } => {
                OneShotBaselineObservation::Present { sha256, unix_mode }
            }
        };
        OneShotBaselineEntry::new(self.relative_path, observation)
    }
}

/// Cohesive context for observation-target derivation.
#[pyclass(name = "OneShotObservationRequest", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyOneShotObservationRequest {
    repository_id: String,
    kind: PyOneShotPlanKind,
    prior: PyOneShotPriorMaterialization,
}

#[pymethods]
impl PyOneShotObservationRequest {
    /// Bind one repository, closed plan kind, and explicit prior claim.
    #[new]
    pub fn new(
        repository_id: String,
        kind: PyOneShotPlanKind,
        prior: PyOneShotPriorMaterialization,
    ) -> Self {
        Self {
            repository_id,
            kind,
            prior,
        }
    }
}

/// Cohesive complete request for pure one-shot planning.
#[pyclass(name = "OneShotPlanningRequest", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyOneShotPlanningRequest {
    observation: PyOneShotObservationRequest,
    baseline: Vec<PyBaselineEntry>,
}

#[pymethods]
impl PyOneShotPlanningRequest {
    /// Add the complete explicit baseline to one observation context.
    #[new]
    pub fn new(observation: PyOneShotObservationRequest, baseline: Vec<PyBaselineEntry>) -> Self {
        Self {
            observation,
            baseline,
        }
    }
}

impl PyOneShotPlanningRequest {
    fn into_api(self) -> OneShotPlanningRequest {
        OneShotPlanningRequest::new(
            self.observation.repository_id,
            self.observation.kind.into(),
            self.observation.prior.into_api(),
            self.baseline
                .into_iter()
                .map(PyBaselineEntry::into_api)
                .collect(),
        )
    }
}

#[pymethods]
impl PalePython {
    /// Compile one complete captured source family without host I/O.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when captured source compilation
    /// fails admission; the complete record is attached as `diagnostic`.
    pub fn compile_one_shot(
        &self,
        py: Python<'_>,
        request: PyOneShotCompilationRequest,
    ) -> PyResult<PyOneShotCompilation> {
        let request = request.into_api();
        let api = Arc::clone(&self.api);
        py.detach(move || api.compile_one_shot(request))
            .map(|inner| PyOneShotCompilation {
                inner: Arc::new(inner),
            })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Return the binding-safe summary of one admitted compilation.
    pub fn one_shot_summary(
        &self,
        py: Python<'_>,
        compilation: &PyOneShotCompilation,
    ) -> PyOneShotSummary {
        let compilation = Arc::clone(&compilation.inner);
        py.detach(move || PyOneShotSummary::from_api(compilation.summary()))
    }

    /// Derive the exact target batch a host must observe before planning.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when the API rejects the
    /// repository, typed plan kind, or claimed prior state.
    pub fn one_shot_observation_targets(
        &self,
        py: Python<'_>,
        compilation: &PyOneShotCompilation,
        request: PyOneShotObservationRequest,
    ) -> PyResult<Vec<String>> {
        let repository_id = request.repository_id;
        let kind = request.kind.into();
        let prior = request.prior.into_api();
        let api = Arc::clone(&self.api);
        let compilation = Arc::clone(&compilation.inner);
        py.detach(move || {
            api.one_shot_observation_targets(&compilation, repository_id, kind, &prior)
        })
        .map(|targets| {
            targets
                .into_iter()
                .map(|target| target.relative_path().to_owned())
                .collect()
        })
        .map_err(|error| diagnostic_error(py, error))
    }

    /// Construct one deterministic plan candidate without host effects.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when API-owned planning rejects
    /// the explicit request.
    pub fn plan_one_shot(
        &self,
        py: Python<'_>,
        compilation: &PyOneShotCompilation,
        request: PyOneShotPlanningRequest,
    ) -> PyResult<PyOneShotPlanCandidate> {
        let request = request.into_api();
        let api = Arc::clone(&self.api);
        let compilation = Arc::clone(&compilation.inner);
        py.detach(move || api.plan_one_shot(&compilation, request))
            .map(PyOneShotPlanCandidate::from_api)
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Bind exact machine-local observations into one complete plan candidate.
    ///
    /// The plan handle is consumed exactly once. This transition performs no
    /// saved-plan admission and grants no filesystem mutation authority.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] for malformed or contradictory
    /// observation records. Reusing a consumed handle raises a value error.
    pub fn bind_one_shot_machine_effects(
        &self,
        py: Python<'_>,
        candidate: &PyOneShotPlanCandidate,
        effects: Vec<PyMachineLocalEffectCandidate>,
    ) -> PyResult<PyOneShotBoundPlan> {
        let candidate = candidate.take()?;
        let effects = effects
            .into_iter()
            .map(PyMachineLocalEffectCandidate::into_api)
            .collect();
        let api = Arc::clone(&self.api);
        py.detach(move || api.bind_one_shot_machine_effects(candidate, effects))
            .map(PyOneShotBoundPlan::from_api)
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Admit exact saved bytes against one freshly reconstructed bound plan.
    ///
    /// The bound handle is consumed exactly once. Success returns the opaque
    /// API admission capability but causes no native effect.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when saved bytes are malformed,
    /// stale, forged, or unequal. Reusing a consumed handle raises a value
    /// error.
    pub fn admit_one_shot_saved_plan(
        &self,
        py: Python<'_>,
        rebuilt: &PyOneShotBoundPlan,
        saved_bytes: Vec<u8>,
    ) -> PyResult<PyOneShotAdmittedPlan> {
        let rebuilt = rebuilt.take()?;
        let api = Arc::clone(&self.api);
        py.detach(move || api.admit_one_shot_saved_plan(rebuilt, &saved_bytes))
            .map(|inner| PyOneShotAdmittedPlan { inner })
            .map_err(|error| diagnostic_error(py, error))
    }
}
