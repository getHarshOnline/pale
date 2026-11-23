//! Python projection of the API-owned configuration-improvement lifecycle.
//!
//! Each consuming phase is retained behind one linear handle. Python never
//! clones application state or keeps a borrow while the interpreter is
//! detached. Preparation borrows its explanation so a rejected candidate can
//! be corrected and retried; commit consumes that exact explanation and the
//! preparation it owns. Terminal dispositions remain effect-free, and
//! [`PyConfigurationImprovementPreview::mark_stale`] requires a caller-owned
//! proof that an outer coordinate changed.

use std::sync::{Arc, Mutex};

use pale_sdk::{
    ApiError, ConfigurationImprovementCandidateRequest, ConfigurationImprovementCoordinateChange,
    ConfigurationImprovementDiagnosis, ConfigurationImprovementDisposition,
    ConfigurationImprovementEvaluation, ConfigurationImprovementExplanation,
    ConfigurationImprovementInspection, ConfigurationImprovementPhase,
    ConfigurationImprovementPreview, ConfigurationImprovementPreviewSummary,
    ConfigurationImprovementProposal, ConfigurationImprovementProposalRequest,
    ConfigurationImprovementTerminal, PreparedConfigurationImprovementPreview,
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::error::diagnostic_error;
use crate::{
    PalePython, PyEffectiveProfileCompilation, PyOneShotPrivateInputs, PyOneShotWorkspaceInputs,
    PyPortableWorkspaceCompilation, PyWorkspaceChangeRootPage, PyWorkspaceExplanation,
    PyWorkspaceImpactPage, PyWorkspaceSubject,
};

type LinearHandle<T> = Arc<Mutex<Option<T>>>;

#[derive(Clone, Copy)]
enum ProfileOwner {
    Baseline,
    Candidate,
}

#[derive(Debug)]
enum BridgeFailure {
    Api(ApiError),
    Consumed(&'static str),
    Poisoned(&'static str),
}

impl From<ApiError> for BridgeFailure {
    fn from(error: ApiError) -> Self {
        Self::Api(error)
    }
}

fn retain<T>(value: T) -> LinearHandle<T> {
    Arc::new(Mutex::new(Some(value)))
}

fn take<T>(handle: &LinearHandle<T>, name: &'static str) -> Result<T, BridgeFailure> {
    handle
        .lock()
        .map_err(|_| BridgeFailure::Poisoned(name))?
        .take()
        .ok_or(BridgeFailure::Consumed(name))
}

fn with<T, R>(
    handle: &LinearHandle<T>,
    name: &'static str,
    project: impl FnOnce(&T) -> R,
) -> Result<R, BridgeFailure> {
    handle
        .lock()
        .map_err(|_| BridgeFailure::Poisoned(name))?
        .as_ref()
        .map(project)
        .ok_or(BridgeFailure::Consumed(name))
}

fn bridge_error(py: Python<'_>, error: BridgeFailure) -> PyErr {
    match error {
        BridgeFailure::Api(error) => diagnostic_error(py, error),
        BridgeFailure::Consumed(name) => {
            PyValueError::new_err(format!("{name} has already been consumed"))
        }
        BridgeFailure::Poisoned(name) => {
            PyRuntimeError::new_err(format!("{name} is unavailable after an internal panic"))
        }
    }
}

/// Closed, ordered phase of one configuration-improvement lifecycle.
#[pyclass(
    name = "ConfigurationImprovementPhase",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyConfigurationImprovementPhase {
    /// An admitted baseline is retained for inspection.
    Inspected,
    /// The baseline is joined to one effective-profile proof owner.
    Explained,
    /// A bounded source candidate is captured but not admitted.
    Proposed,
    /// The candidate passes canonical workspace admission.
    Evaluated,
    /// The canonical derivation graph establishes the impact partition.
    Diagnosed,
    /// The candidate and proof are frozen for explicit review.
    Previewed,
}

impl From<ConfigurationImprovementPhase> for PyConfigurationImprovementPhase {
    fn from(value: ConfigurationImprovementPhase) -> Self {
        match value {
            ConfigurationImprovementPhase::Inspected => Self::Inspected,
            ConfigurationImprovementPhase::Explained => Self::Explained,
            ConfigurationImprovementPhase::Proposed => Self::Proposed,
            ConfigurationImprovementPhase::Evaluated => Self::Evaluated,
            ConfigurationImprovementPhase::Diagnosed => Self::Diagnosed,
            ConfigurationImprovementPhase::Previewed => Self::Previewed,
        }
    }
}

/// Explicit effect-free terminal disposition selected by the caller.
#[pyclass(
    name = "ConfigurationImprovementDisposition",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyConfigurationImprovementDisposition {
    /// Permit presentation to a separate authority and effect gate.
    Accepted,
    /// Reject the candidate.
    Rejected,
    /// Cancel the review.
    Cancelled,
    /// Record caller-proved drift of an outer coordinate.
    Stale,
}

impl From<ConfigurationImprovementDisposition> for PyConfigurationImprovementDisposition {
    fn from(value: ConfigurationImprovementDisposition) -> Self {
        match value {
            ConfigurationImprovementDisposition::Accepted => Self::Accepted,
            ConfigurationImprovementDisposition::Rejected => Self::Rejected,
            ConfigurationImprovementDisposition::Cancelled => Self::Cancelled,
            ConfigurationImprovementDisposition::Stale => Self::Stale,
        }
    }
}

/// Closed digest-transition kind in one preview summary.
#[pyclass(
    name = "ConfigurationImprovementCoordinateChangeKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyConfigurationImprovementCoordinateChangeKind {
    /// Baseline and candidate retain one digest.
    Unchanged,
    /// Candidate compilation establishes a distinct digest.
    Changed,
}

/// One exact API-owned digest transition.
#[pyclass(name = "ConfigurationImprovementCoordinateChange", frozen)]
pub struct PyConfigurationImprovementCoordinateChange {
    kind: PyConfigurationImprovementCoordinateChangeKind,
    digest: Option<String>,
    prior_digest: Option<String>,
    candidate_digest: Option<String>,
}

impl PyConfigurationImprovementCoordinateChange {
    fn from_api(change: &ConfigurationImprovementCoordinateChange) -> Self {
        match change {
            ConfigurationImprovementCoordinateChange::Unchanged { digest } => Self {
                kind: PyConfigurationImprovementCoordinateChangeKind::Unchanged,
                digest: Some(digest.clone()),
                prior_digest: None,
                candidate_digest: None,
            },
            ConfigurationImprovementCoordinateChange::Changed {
                prior_digest,
                candidate_digest,
            } => Self {
                kind: PyConfigurationImprovementCoordinateChangeKind::Changed,
                digest: None,
                prior_digest: Some(prior_digest.clone()),
                candidate_digest: Some(candidate_digest.clone()),
            },
        }
    }
}

#[pymethods]
impl PyConfigurationImprovementCoordinateChange {
    /// Closed transition kind.
    #[getter]
    pub fn kind(&self) -> PyConfigurationImprovementCoordinateChangeKind {
        self.kind
    }

    /// Shared digest for an unchanged coordinate.
    #[getter]
    pub fn digest(&self) -> Option<&str> {
        self.digest.as_deref()
    }

    /// Baseline digest for a changed coordinate.
    #[getter]
    pub fn prior_digest(&self) -> Option<&str> {
        self.prior_digest.as_deref()
    }

    /// Candidate digest for a changed coordinate.
    #[getter]
    pub fn candidate_digest(&self) -> Option<&str> {
        self.candidate_digest.as_deref()
    }
}

/// Complete portable and private source for the direct proposal route.
#[pyclass(
    name = "ConfigurationImprovementProposalRequest",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyConfigurationImprovementProposalRequest {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    workspace: PyOneShotWorkspaceInputs,
    private_inputs: PyOneShotPrivateInputs,
}

#[pymethods]
impl PyConfigurationImprovementProposalRequest {
    /// Bind exact selectors to already-owned workspace and private source.
    #[new]
    #[pyo3(signature = (*, project_id, execution_workspace_id, machine_id, workspace, private_inputs))]
    pub fn new(
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
        workspace: PyOneShotWorkspaceInputs,
        private_inputs: PyOneShotPrivateInputs,
    ) -> Self {
        Self {
            project_id,
            execution_workspace_id,
            machine_id,
            workspace,
            private_inputs,
        }
    }
}

impl PyConfigurationImprovementProposalRequest {
    fn into_api(self) -> ConfigurationImprovementProposalRequest {
        ConfigurationImprovementProposalRequest::new(
            self.project_id,
            self.execution_workspace_id,
            self.machine_id,
            self.workspace.into_api(),
            self.private_inputs.into_api(),
        )
    }
}

/// Private/profile input for an already admitted candidate workspace.
#[pyclass(
    name = "ConfigurationImprovementCandidateRequest",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyConfigurationImprovementCandidateRequest {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    private_inputs: PyOneShotPrivateInputs,
}

#[pymethods]
impl PyConfigurationImprovementCandidateRequest {
    /// Bind exact selectors and private source to a retained candidate owner.
    #[new]
    #[pyo3(signature = (*, project_id, execution_workspace_id, machine_id, private_inputs))]
    pub fn new(
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
        private_inputs: PyOneShotPrivateInputs,
    ) -> Self {
        Self {
            project_id,
            execution_workspace_id,
            machine_id,
            private_inputs,
        }
    }
}

impl PyConfigurationImprovementCandidateRequest {
    fn into_api(self) -> ConfigurationImprovementCandidateRequest {
        ConfigurationImprovementCandidateRequest::new(
            self.project_id,
            self.execution_workspace_id,
            self.machine_id,
            self.private_inputs.into_api(),
        )
    }
}

/// Linear inspected-baseline handle.
#[pyclass(name = "ConfigurationImprovementInspection", frozen)]
pub struct PyConfigurationImprovementInspection {
    inner: LinearHandle<ConfigurationImprovementInspection>,
}

#[pymethods]
impl PyConfigurationImprovementInspection {
    /// Current phase represented by this handle type.
    ///
    /// # Errors
    ///
    /// Raises a value error after this inspection has been consumed.
    #[getter]
    pub fn phase(&self, py: Python<'_>) -> PyResult<PyConfigurationImprovementPhase> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement inspection", |value| {
                value.phase().into()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Principal workspace identity.
    ///
    /// # Errors
    ///
    /// Raises a value error after this inspection has been consumed.
    #[getter]
    pub fn workspace_id(&self, py: Python<'_>) -> PyResult<String> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement inspection", |value| {
                value.workspace_id().to_owned()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Exact admitted baseline-universe digest.
    ///
    /// # Errors
    ///
    /// Raises a value error after this inspection has been consumed.
    #[getter]
    pub fn principal_universe_digest(&self, py: Python<'_>) -> PyResult<String> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement inspection", |value| {
                value.principal_universe_digest().to_owned()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }
}

/// Linear explanation handle joined to one exact effective-profile owner.
#[pyclass(name = "ConfigurationImprovementExplanation", frozen)]
pub struct PyConfigurationImprovementExplanation {
    inner: LinearHandle<ConfigurationImprovementExplanation>,
}

#[pymethods]
impl PyConfigurationImprovementExplanation {
    /// Current phase represented by this handle type.
    ///
    /// # Errors
    ///
    /// Raises a value error after this explanation has been consumed.
    #[getter]
    pub fn phase(&self, py: Python<'_>) -> PyResult<PyConfigurationImprovementPhase> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement explanation", |value| {
                value.phase().into()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Effective-profile identity explaining the baseline.
    ///
    /// # Errors
    ///
    /// Raises a value error after this explanation has been consumed.
    #[getter]
    pub fn profile_digest(&self, py: Python<'_>) -> PyResult<String> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement explanation", |value| {
                value.profile_digest().to_owned()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Private-instance projection identity, not native realization proof.
    ///
    /// # Errors
    ///
    /// Raises a value error after this explanation has been consumed.
    #[getter]
    pub fn instance_digest(&self, py: Python<'_>) -> PyResult<String> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement explanation", |value| {
                value.instance_digest().to_owned()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }
}

/// Linear bounded proposal handle.
#[pyclass(name = "ConfigurationImprovementProposal", frozen)]
pub struct PyConfigurationImprovementProposal {
    inner: LinearHandle<ConfigurationImprovementProposal>,
}

#[pymethods]
impl PyConfigurationImprovementProposal {
    /// Current phase represented by this handle type.
    ///
    /// # Errors
    ///
    /// Raises a value error after this proposal has been consumed.
    #[getter]
    pub fn phase(&self, py: Python<'_>) -> PyResult<PyConfigurationImprovementPhase> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement proposal", |value| {
                value.phase().into()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }
}

/// Linear admitted-candidate handle.
#[pyclass(name = "ConfigurationImprovementEvaluation", frozen)]
pub struct PyConfigurationImprovementEvaluation {
    inner: LinearHandle<ConfigurationImprovementEvaluation>,
}

#[pymethods]
impl PyConfigurationImprovementEvaluation {
    /// Current phase represented by this handle type.
    ///
    /// # Errors
    ///
    /// Raises a value error after this evaluation has been consumed.
    #[getter]
    pub fn phase(&self, py: Python<'_>) -> PyResult<PyConfigurationImprovementPhase> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement evaluation", |value| {
                value.phase().into()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Principal-relative candidate-universe identity.
    ///
    /// # Errors
    ///
    /// Raises a value error after this evaluation has been consumed.
    #[getter]
    pub fn candidate_principal_universe_digest(&self, py: Python<'_>) -> PyResult<String> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement evaluation", |value| {
                value.candidate_principal_universe_digest().to_owned()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Candidate effective-profile identity.
    ///
    /// # Errors
    ///
    /// Raises a value error after this evaluation has been consumed.
    #[getter]
    pub fn candidate_profile_digest(&self, py: Python<'_>) -> PyResult<String> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement evaluation", |value| {
                value.candidate_profile_digest().to_owned()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Candidate private-instance identity.
    ///
    /// # Errors
    ///
    /// Raises a value error after this evaluation has been consumed.
    #[getter]
    pub fn candidate_instance_digest(&self, py: Python<'_>) -> PyResult<String> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement evaluation", |value| {
                value.candidate_instance_digest().to_owned()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }
}

/// Linear diagnosed-candidate handle.
#[pyclass(name = "ConfigurationImprovementDiagnosis", frozen)]
pub struct PyConfigurationImprovementDiagnosis {
    inner: LinearHandle<ConfigurationImprovementDiagnosis>,
}

#[pymethods]
impl PyConfigurationImprovementDiagnosis {
    /// Current phase represented by this handle type.
    ///
    /// # Errors
    ///
    /// Raises a value error after this diagnosis has been consumed.
    #[getter]
    pub fn phase(&self, py: Python<'_>) -> PyResult<PyConfigurationImprovementPhase> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement diagnosis", |value| {
                value.phase().into()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Exact derivation-proof identity.
    ///
    /// # Errors
    ///
    /// Raises a value error after this diagnosis has been consumed.
    #[getter]
    pub fn derivation_receipt_digest(&self, py: Python<'_>) -> PyResult<String> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement diagnosis", |value| {
                value.derivation_receipt_digest().to_owned()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }
}

/// Immutable API-owned preview coordinates and impact counts.
#[pyclass(name = "ConfigurationImprovementPreviewSummary", frozen)]
pub struct PyConfigurationImprovementPreviewSummary {
    inner: Arc<ConfigurationImprovementPreviewSummary>,
}

impl PyConfigurationImprovementPreviewSummary {
    fn from_api(inner: &ConfigurationImprovementPreviewSummary) -> Self {
        Self {
            inner: Arc::new(inner.clone()),
        }
    }
}

#[pymethods]
impl PyConfigurationImprovementPreviewSummary {
    /// Principal workspace whose configuration is reviewed.
    #[getter]
    pub fn workspace_id(&self) -> &str {
        self.inner.workspace_id()
    }

    /// Canonical project selected by the explanatory profile.
    #[getter]
    pub fn project_id(&self) -> &str {
        self.inner.project_id()
    }

    /// Canonical execution workspace selected by the explanatory profile.
    #[getter]
    pub fn execution_workspace_id(&self) -> &str {
        self.inner.execution_workspace_id()
    }

    /// Principal-relative identity of the inspected baseline.
    #[getter]
    pub fn baseline_principal_universe_digest(&self) -> &str {
        self.inner.baseline_principal_universe_digest()
    }

    /// Semantic-IR identity of the inspected baseline workspace.
    #[getter]
    pub fn baseline_workspace_semantic_ir_digest(&self) -> &str {
        self.inner.baseline_workspace_semantic_ir_digest()
    }

    /// Admission-evidence identity of the inspected baseline workspace.
    #[getter]
    pub fn baseline_workspace_admission_evidence_digest(&self) -> &str {
        self.inner.baseline_workspace_admission_evidence_digest()
    }

    /// Desired-topology identity selected for the baseline.
    #[getter]
    pub fn selected_topology_digest(&self) -> &str {
        self.inner.selected_topology_digest()
    }

    /// Effective-profile identity used for explanation.
    #[getter]
    pub fn profile_digest(&self) -> &str {
        self.inner.profile_digest()
    }

    /// Private-instance identity used for explanation.
    #[getter]
    pub fn instance_digest(&self) -> &str {
        self.inner.instance_digest()
    }

    /// Package semantic-IR identity retained by the baseline profile.
    #[getter]
    pub fn package_semantic_ir_digest(&self) -> &str {
        self.inner.package_semantic_ir_digest()
    }

    /// Neutral composition-proof identity retained by the baseline profile.
    #[getter]
    pub fn composition_plan_digest(&self) -> &str {
        self.inner.composition_plan_digest()
    }

    /// Principal-relative identity of the admitted candidate.
    #[getter]
    pub fn candidate_principal_universe_digest(&self) -> &str {
        self.inner.candidate_principal_universe_digest()
    }

    /// Semantic-IR identity of the admitted candidate workspace.
    #[getter]
    pub fn candidate_workspace_semantic_ir_digest(&self) -> &str {
        self.inner.candidate_workspace_semantic_ir_digest()
    }

    /// Admission-evidence identity of the admitted candidate workspace.
    #[getter]
    pub fn candidate_workspace_admission_evidence_digest(&self) -> &str {
        self.inner.candidate_workspace_admission_evidence_digest()
    }

    /// Desired-topology identity selected for the candidate.
    #[getter]
    pub fn candidate_selected_topology_digest(&self) -> &str {
        self.inner.candidate_selected_topology_digest()
    }

    /// Effective-profile identity of the candidate.
    #[getter]
    pub fn candidate_profile_digest(&self) -> &str {
        self.inner.candidate_profile_digest()
    }

    /// Private-instance identity of the candidate.
    #[getter]
    pub fn candidate_instance_digest(&self) -> &str {
        self.inner.candidate_instance_digest()
    }

    /// Package semantic-IR identity retained by the candidate profile.
    #[getter]
    pub fn candidate_package_semantic_ir_digest(&self) -> &str {
        self.inner.candidate_package_semantic_ir_digest()
    }

    /// Neutral composition-proof identity retained by the candidate profile.
    #[getter]
    pub fn candidate_composition_plan_digest(&self) -> &str {
        self.inner.candidate_composition_plan_digest()
    }

    /// Exact derivation-proof identity.
    #[getter]
    pub fn derivation_receipt_digest(&self) -> &str {
        self.inner.derivation_receipt_digest()
    }

    /// Exact source-change-root count.
    #[getter]
    pub fn change_root_count(&self) -> u32 {
        self.inner.change_root_count()
    }

    /// Exact affected-subject count.
    #[getter]
    pub fn affected_count(&self) -> u32 {
        self.inner.affected_count()
    }

    /// Exact unaffected-subject count.
    #[getter]
    pub fn unaffected_count(&self) -> u32 {
        self.inner.unaffected_count()
    }

    /// Selected-topology digest transition.
    #[getter]
    pub fn selected_topology_change(&self) -> PyConfigurationImprovementCoordinateChange {
        PyConfigurationImprovementCoordinateChange::from_api(self.inner.selected_topology_change())
    }

    /// Effective-profile digest transition.
    #[getter]
    pub fn profile_change(&self) -> PyConfigurationImprovementCoordinateChange {
        PyConfigurationImprovementCoordinateChange::from_api(self.inner.profile_change())
    }

    /// Private-instance digest transition.
    #[getter]
    pub fn instance_change(&self) -> PyConfigurationImprovementCoordinateChange {
        PyConfigurationImprovementCoordinateChange::from_api(self.inner.instance_change())
    }

    /// Package semantic-IR digest transition.
    #[getter]
    pub fn package_semantic_ir_change(&self) -> PyConfigurationImprovementCoordinateChange {
        PyConfigurationImprovementCoordinateChange::from_api(
            self.inner.package_semantic_ir_change(),
        )
    }

    /// Neutral composition-proof digest transition.
    #[getter]
    pub fn composition_plan_change(&self) -> PyConfigurationImprovementCoordinateChange {
        PyConfigurationImprovementCoordinateChange::from_api(self.inner.composition_plan_change())
    }

    /// Copy the exact API-owned canonical JSON bytes.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, self.inner.canonical_bytes())
    }
}

/// Recoverable preparation that has not consumed its explanation owner.
#[pyclass(name = "PreparedConfigurationImprovementPreview", frozen)]
pub struct PyPreparedConfigurationImprovementPreview {
    inner: LinearHandle<PreparedConfigurationImprovementPreview>,
}

#[pymethods]
impl PyPreparedConfigurationImprovementPreview {
    /// Borrow the bounded projected summary before commit.
    ///
    /// # Errors
    ///
    /// Raises a value error after this preparation has been consumed.
    #[getter]
    pub fn summary(&self, py: Python<'_>) -> PyResult<PyConfigurationImprovementPreviewSummary> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement preparation", |value| {
                PyConfigurationImprovementPreviewSummary::from_api(value.summary())
            })
        })
        .map_err(|error| bridge_error(py, error))
    }
}

/// Linear reviewable preview and retained proof owner.
#[pyclass(name = "ConfigurationImprovementPreview", frozen)]
pub struct PyConfigurationImprovementPreview {
    inner: LinearHandle<ConfigurationImprovementPreview>,
}

/// View over the comparison retained by one live preview.
#[pyclass(name = "ConfigurationImprovementComparison", frozen)]
pub struct PyConfigurationImprovementComparison {
    preview: LinearHandle<ConfigurationImprovementPreview>,
}

#[pymethods]
impl PyConfigurationImprovementComparison {
    /// Principal-relative predecessor-universe digest.
    ///
    /// # Errors
    ///
    /// Raises a value error after the parent preview has been consumed.
    #[getter]
    pub fn prior_principal_universe_digest(&self, py: Python<'_>) -> PyResult<String> {
        let preview = Arc::clone(&self.preview);
        py.detach(move || {
            with(&preview, "configuration-improvement preview", |value| {
                value
                    .comparison()
                    .prior_principal_universe_digest()
                    .to_owned()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Principal-relative current-universe digest.
    ///
    /// # Errors
    ///
    /// Raises a value error after the parent preview has been consumed.
    #[getter]
    pub fn current_principal_universe_digest(&self, py: Python<'_>) -> PyResult<String> {
        let preview = Arc::clone(&self.preview);
        py.detach(move || {
            with(&preview, "configuration-improvement preview", |value| {
                value
                    .comparison()
                    .current_principal_universe_digest()
                    .to_owned()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Verified predecessor semantic-IR digest.
    ///
    /// # Errors
    ///
    /// Raises a value error after the parent preview has been consumed.
    #[getter]
    pub fn prior_semantic_ir_digest(&self, py: Python<'_>) -> PyResult<String> {
        let preview = Arc::clone(&self.preview);
        py.detach(move || {
            with(&preview, "configuration-improvement preview", |value| {
                value.comparison().prior_semantic_ir_digest().to_owned()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Verified current semantic-IR digest.
    ///
    /// # Errors
    ///
    /// Raises a value error after the parent preview has been consumed.
    #[getter]
    pub fn current_semantic_ir_digest(&self, py: Python<'_>) -> PyResult<String> {
        let preview = Arc::clone(&self.preview);
        py.detach(move || {
            with(&preview, "configuration-improvement preview", |value| {
                value.comparison().current_semantic_ir_digest().to_owned()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Exact derivation-receipt digest.
    ///
    /// # Errors
    ///
    /// Raises a value error after the parent preview has been consumed.
    #[getter]
    pub fn receipt_digest(&self, py: Python<'_>) -> PyResult<String> {
        let preview = Arc::clone(&self.preview);
        py.detach(move || {
            with(&preview, "configuration-improvement preview", |value| {
                value.comparison().receipt_digest().to_owned()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Number of exact source-change roots.
    ///
    /// # Errors
    ///
    /// Raises a value error after the parent preview has been consumed.
    #[getter]
    pub fn change_root_count(&self, py: Python<'_>) -> PyResult<u32> {
        let preview = Arc::clone(&self.preview);
        py.detach(move || {
            with(&preview, "configuration-improvement preview", |value| {
                value.comparison().change_root_count()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Number of current subjects reachable from a change root.
    ///
    /// # Errors
    ///
    /// Raises a value error after the parent preview has been consumed.
    #[getter]
    pub fn affected_count(&self, py: Python<'_>) -> PyResult<u32> {
        let preview = Arc::clone(&self.preview);
        py.detach(move || {
            with(&preview, "configuration-improvement preview", |value| {
                value.comparison().affected_count()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Number of current subjects outside every change-root closure.
    ///
    /// # Errors
    ///
    /// Raises a value error after the parent preview has been consumed.
    #[getter]
    pub fn unaffected_count(&self, py: Python<'_>) -> PyResult<u32> {
        let preview = Arc::clone(&self.preview);
        py.detach(move || {
            with(&preview, "configuration-improvement preview", |value| {
                value.comparison().unaffected_count()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Project one bounded page of exact change roots.
    ///
    /// # Errors
    ///
    /// Raises a value error after the parent preview has been consumed.
    pub fn change_roots(
        &self,
        py: Python<'_>,
        offset: u32,
        limit: u16,
    ) -> PyResult<PyWorkspaceChangeRootPage> {
        let preview = Arc::clone(&self.preview);
        py.detach(move || {
            with(&preview, "configuration-improvement preview", |value| {
                PyWorkspaceChangeRootPage::from_api(value.comparison().change_roots(offset, limit))
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Project one bounded page of the complete impact partition.
    ///
    /// # Errors
    ///
    /// Raises a value error after the parent preview has been consumed.
    pub fn impacts(
        &self,
        py: Python<'_>,
        offset: u32,
        limit: u16,
    ) -> PyResult<PyWorkspaceImpactPage> {
        let preview = Arc::clone(&self.preview);
        py.detach(move || {
            with(&preview, "configuration-improvement preview", |value| {
                PyWorkspaceImpactPage::from_api(value.comparison().impacts(offset, limit))
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Explain one subject obtained from this retained comparison.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] for a foreign subject and a
    /// value error after the parent preview has been consumed.
    pub fn explain(
        &self,
        py: Python<'_>,
        subject: &PyWorkspaceSubject,
    ) -> PyResult<PyWorkspaceExplanation> {
        let preview = Arc::clone(&self.preview);
        let subject = subject.inner.clone();
        py.detach(move || {
            with(&preview, "configuration-improvement preview", |value| {
                value.comparison().explain(&subject)
            })?
            .map(PyWorkspaceExplanation::from_api)
            .map_err(BridgeFailure::Api)
        })
        .map_err(|error| bridge_error(py, error))
    }
}

#[pymethods]
impl PyConfigurationImprovementPreview {
    /// Current phase represented by this handle type.
    ///
    /// # Errors
    ///
    /// Raises a value error after this preview has been consumed.
    #[getter]
    pub fn phase(&self, py: Python<'_>) -> PyResult<PyConfigurationImprovementPhase> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement preview", |value| {
                value.phase().into()
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Borrow the immutable API-owned summary.
    ///
    /// # Errors
    ///
    /// Raises a value error after this preview has been consumed.
    #[getter]
    pub fn summary(&self, py: Python<'_>) -> PyResult<PyConfigurationImprovementPreviewSummary> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement preview", |value| {
                PyConfigurationImprovementPreviewSummary::from_api(value.summary())
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Borrow the comparison retained by this live preview.
    ///
    /// # Errors
    ///
    /// Raises a value error after this preview has been consumed.
    pub fn comparison(&self, py: Python<'_>) -> PyResult<PyConfigurationImprovementComparison> {
        let inner = Arc::clone(&self.inner);
        py.detach(move || {
            with(&inner, "configuration-improvement preview", |_| {
                PyConfigurationImprovementComparison {
                    preview: Arc::clone(&inner),
                }
            })
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Clone the immutable baseline-profile inspection owner without recompiling.
    ///
    /// # Errors
    ///
    /// Raises a value error after this preview has been consumed.
    pub fn baseline_profile(&self, py: Python<'_>) -> PyResult<PyEffectiveProfileCompilation> {
        self.profile(py, ProfileOwner::Baseline)
    }

    /// Clone the immutable candidate-profile inspection owner without recompiling.
    ///
    /// # Errors
    ///
    /// Raises a value error after this preview has been consumed.
    pub fn candidate_profile(&self, py: Python<'_>) -> PyResult<PyEffectiveProfileCompilation> {
        self.profile(py, ProfileOwner::Candidate)
    }

    /// Accept this preview without mutating source or granting authority.
    ///
    /// # Errors
    ///
    /// Raises a value error if this preview was already consumed.
    pub fn accept(&self, py: Python<'_>) -> PyResult<PyConfigurationImprovementTerminal> {
        self.finish(py, ConfigurationImprovementDisposition::Accepted)
    }

    /// Reject this preview without mutating source.
    ///
    /// # Errors
    ///
    /// Raises a value error if this preview was already consumed.
    pub fn reject(&self, py: Python<'_>) -> PyResult<PyConfigurationImprovementTerminal> {
        self.finish(py, ConfigurationImprovementDisposition::Rejected)
    }

    /// Cancel this preview without mutating source.
    ///
    /// # Errors
    ///
    /// Raises a value error if this preview was already consumed.
    pub fn cancel(&self, py: Python<'_>) -> PyResult<PyConfigurationImprovementTerminal> {
        self.finish(py, ConfigurationImprovementDisposition::Cancelled)
    }

    /// Record caller-proved drift of a separately owned outer coordinate.
    ///
    /// # Errors
    ///
    /// Raises a value error if this preview was already consumed. This method
    /// does not infer staleness from Pale coordinates.
    pub fn mark_stale(&self, py: Python<'_>) -> PyResult<PyConfigurationImprovementTerminal> {
        self.finish(py, ConfigurationImprovementDisposition::Stale)
    }
}

impl PyConfigurationImprovementPreview {
    pub(crate) fn from_api(inner: ConfigurationImprovementPreview) -> Self {
        Self {
            inner: retain(inner),
        }
    }

    fn profile(
        &self,
        py: Python<'_>,
        owner: ProfileOwner,
    ) -> PyResult<PyEffectiveProfileCompilation> {
        let preview = Arc::clone(&self.inner);
        py.detach(move || {
            with(
                &preview,
                "configuration-improvement preview",
                |value| match owner {
                    ProfileOwner::Baseline => value.baseline_profile().clone(),
                    ProfileOwner::Candidate => value.candidate_profile().clone(),
                },
            )
        })
        .map(|inner| PyEffectiveProfileCompilation {
            inner: Arc::new(inner),
        })
        .map_err(|error| bridge_error(py, error))
    }

    fn finish(
        &self,
        py: Python<'_>,
        disposition: ConfigurationImprovementDisposition,
    ) -> PyResult<PyConfigurationImprovementTerminal> {
        let preview = Arc::clone(&self.inner);
        py.detach(move || {
            take(&preview, "configuration-improvement preview").map(|value| match disposition {
                ConfigurationImprovementDisposition::Accepted => value.accept(),
                ConfigurationImprovementDisposition::Rejected => value.reject(),
                ConfigurationImprovementDisposition::Cancelled => value.cancel(),
                ConfigurationImprovementDisposition::Stale => value.mark_stale(),
            })
        })
        .map(PyConfigurationImprovementTerminal::from_api)
        .map_err(|error| bridge_error(py, error))
    }
}

/// Effect-free terminal record for one reviewed candidate.
#[pyclass(name = "ConfigurationImprovementTerminal", frozen)]
pub struct PyConfigurationImprovementTerminal {
    inner: ConfigurationImprovementTerminal,
}

impl PyConfigurationImprovementTerminal {
    fn from_api(inner: ConfigurationImprovementTerminal) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyConfigurationImprovementTerminal {
    /// Explicit terminal disposition selected by the caller.
    #[getter]
    pub fn disposition(&self) -> PyConfigurationImprovementDisposition {
        self.inner.disposition().into()
    }

    /// Exact preview coordinates governed by the terminal disposition.
    #[getter]
    pub fn preview(&self) -> PyConfigurationImprovementPreviewSummary {
        PyConfigurationImprovementPreviewSummary::from_api(self.inner.preview())
    }
}

#[pymethods]
impl PalePython {
    /// Inspect one admitted workspace as the sole lifecycle entry.
    pub fn inspect_configuration_improvement(
        &self,
        py: Python<'_>,
        baseline: &PyPortableWorkspaceCompilation,
    ) -> PyConfigurationImprovementInspection {
        let api = Arc::clone(&self.api);
        let baseline = Arc::clone(&baseline.inner);
        py.detach(move || {
            let inner = api.inspect_configuration_improvement(&baseline);
            PyConfigurationImprovementInspection {
                inner: retain(inner),
            }
        })
    }

    /// Join an inspection to its exact effective-profile explanation owner.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] for a frontier mismatch and a
    /// value error if the inspection was already consumed.
    pub fn explain_configuration_improvement(
        &self,
        py: Python<'_>,
        inspection: &PyConfigurationImprovementInspection,
        profile: &PyEffectiveProfileCompilation,
    ) -> PyResult<PyConfigurationImprovementExplanation> {
        let api = Arc::clone(&self.api);
        let inspection = Arc::clone(&inspection.inner);
        let profile = Arc::clone(&profile.inner);
        py.detach(move || {
            let inspection = take(&inspection, "configuration-improvement inspection")?;
            api.explain_configuration_improvement(inspection, &profile)
                .map_err(BridgeFailure::Api)
        })
        .map(|inner| PyConfigurationImprovementExplanation {
            inner: retain(inner),
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Attach one bounded source candidate without evaluating it.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] for rejected capture and a
    /// value error if the explanation was already consumed.
    pub fn propose_configuration_improvement(
        &self,
        py: Python<'_>,
        explanation: &PyConfigurationImprovementExplanation,
        request: PyConfigurationImprovementProposalRequest,
    ) -> PyResult<PyConfigurationImprovementProposal> {
        let api = Arc::clone(&self.api);
        let explanation = Arc::clone(&explanation.inner);
        let request = request.into_api();
        py.detach(move || {
            let explanation = take(&explanation, "configuration-improvement explanation")?;
            api.propose_configuration_improvement(explanation, request)
                .map_err(BridgeFailure::Api)
        })
        .map(|inner| PyConfigurationImprovementProposal {
            inner: retain(inner),
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Admit one proposal through the canonical workspace compiler.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] for rejected admission and a
    /// value error if the proposal was already consumed.
    pub fn evaluate_configuration_improvement(
        &self,
        py: Python<'_>,
        proposal: &PyConfigurationImprovementProposal,
    ) -> PyResult<PyConfigurationImprovementEvaluation> {
        let api = Arc::clone(&self.api);
        let proposal = Arc::clone(&proposal.inner);
        py.detach(move || {
            let proposal = take(&proposal, "configuration-improvement proposal")?;
            api.evaluate_configuration_improvement(proposal)
                .map_err(BridgeFailure::Api)
        })
        .map(|inner| PyConfigurationImprovementEvaluation {
            inner: retain(inner),
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Derive the exact source-change and impact proof.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] for a contradictory admitted
    /// owner and a value error if the evaluation was already consumed.
    pub fn diagnose_configuration_improvement(
        &self,
        py: Python<'_>,
        evaluation: &PyConfigurationImprovementEvaluation,
    ) -> PyResult<PyConfigurationImprovementDiagnosis> {
        let api = Arc::clone(&self.api);
        let evaluation = Arc::clone(&evaluation.inner);
        py.detach(move || {
            let evaluation = take(&evaluation, "configuration-improvement evaluation")?;
            api.diagnose_configuration_improvement(evaluation)
                .map_err(BridgeFailure::Api)
        })
        .map(|inner| PyConfigurationImprovementDiagnosis {
            inner: retain(inner),
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Freeze one diagnosis into a reviewable preview.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] for a failed canonical
    /// projection and a value error if the diagnosis was already consumed.
    pub fn preview_configuration_improvement(
        &self,
        py: Python<'_>,
        diagnosis: &PyConfigurationImprovementDiagnosis,
    ) -> PyResult<PyConfigurationImprovementPreview> {
        let api = Arc::clone(&self.api);
        let diagnosis = Arc::clone(&diagnosis.inner);
        py.detach(move || {
            let diagnosis = take(&diagnosis, "configuration-improvement diagnosis")?;
            api.preview_configuration_improvement(diagnosis)
                .map_err(BridgeFailure::Api)
        })
        .map(PyConfigurationImprovementPreview::from_api)
        .map_err(|error| bridge_error(py, error))
    }

    /// Prepare a preview from an already admitted candidate generation.
    ///
    /// The explanation is borrowed. Every failure therefore leaves it
    /// available for a corrected retry without compiling portable source again.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] for rejected private/profile,
    /// context, or derivation input and a value error if the explanation was
    /// already consumed.
    pub fn prepare_configuration_improvement_preview(
        &self,
        py: Python<'_>,
        explanation: &PyConfigurationImprovementExplanation,
        candidate: &PyPortableWorkspaceCompilation,
        request: PyConfigurationImprovementCandidateRequest,
    ) -> PyResult<PyPreparedConfigurationImprovementPreview> {
        let api = Arc::clone(&self.api);
        let explanation = Arc::clone(&explanation.inner);
        let candidate = Arc::clone(&candidate.inner);
        let request = request.into_api();
        py.detach(move || {
            with(
                &explanation,
                "configuration-improvement explanation",
                |value| api.prepare_configuration_improvement_preview(value, &candidate, request),
            )?
            .map_err(BridgeFailure::Api)
        })
        .map(|inner| PyPreparedConfigurationImprovementPreview {
            inner: retain(inner),
        })
        .map_err(|error| bridge_error(py, error))
    }

    /// Commit a preparation through the exact explanation that created it.
    ///
    /// Both handles are consumed together before the API owner validates their
    /// private lifecycle token.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] for an owner splice and a value
    /// error if either handle was already consumed.
    pub fn commit_configuration_improvement_preview(
        &self,
        py: Python<'_>,
        explanation: &PyConfigurationImprovementExplanation,
        prepared: &PyPreparedConfigurationImprovementPreview,
    ) -> PyResult<PyConfigurationImprovementPreview> {
        let api = Arc::clone(&self.api);
        let explanation = Arc::clone(&explanation.inner);
        let prepared = Arc::clone(&prepared.inner);
        py.detach(move || {
            let mut explanation_guard = explanation
                .lock()
                .map_err(|_| BridgeFailure::Poisoned("configuration-improvement explanation"))?;
            let mut prepared_guard = prepared
                .lock()
                .map_err(|_| BridgeFailure::Poisoned("configuration-improvement preparation"))?;
            if explanation_guard.is_none() {
                return Err(BridgeFailure::Consumed(
                    "configuration-improvement explanation",
                ));
            }
            if prepared_guard.is_none() {
                return Err(BridgeFailure::Consumed(
                    "configuration-improvement preparation",
                ));
            }
            let explanation = explanation_guard.take().ok_or(BridgeFailure::Consumed(
                "configuration-improvement explanation",
            ))?;
            let prepared = prepared_guard.take().ok_or(BridgeFailure::Consumed(
                "configuration-improvement preparation",
            ))?;
            drop(prepared_guard);
            drop(explanation_guard);
            api.commit_configuration_improvement_preview(explanation, prepared)
                .map_err(BridgeFailure::Api)
        })
        .map(PyConfigurationImprovementPreview::from_api)
        .map_err(|error| bridge_error(py, error))
    }
}
