//! Native-local Python projections over API-owned host composition.
//!
//! This module translates owned Python values into the `native-local`
//! [`pale_sdk`] façade. Filesystem/process composition, planning, saved-plan
//! admission, settlement, reconciliation, and diagnostic policy stay behind
//! that façade; this leaf neither imports adapters nor recreates their laws.

use std::sync::{Arc, Mutex};

use pale_sdk::{
    CompiledInterfaceIdentity, DurablePublicationMechanism, LocalConfigurationImprovementRequest,
    LocalMaterializationAction, LocalMaterializationAdmissionRequest,
    LocalMaterializationAdmittedRun, LocalMaterializationDrift,
    LocalMaterializationMachineEffectKind, LocalMaterializationOwnershipChange,
    LocalMaterializationOwnershipObservation, LocalMaterializationReceiptObservation,
    LocalMaterializationReconciliation, LocalMaterializationRunOutcome,
    LocalMaterializationRunResult, LocalMaterializationTargetAdmission,
    LocalMaterializationTargetOutcome, LocalMaterializationTargetResult,
    LocalNativeActionInputRequest, LocalOneShotCompilation, LocalOneShotCompilationRequest,
    LocalOneShotPlan, LocalOneShotPlanCounts, LocalOneShotPlanRequest,
    LocalPackageDistributionRequest, LocalRepositoryStatus, LocalWorkspaceLocator,
    NativeActionRequest,
};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::error::diagnostic_error;
use crate::{PalePython, PyOneShotPlanKind, PyOneShotSummary, PyPackageDistributionInspection};

/// Two local workspace entries to compare without changing either source tree.
#[pyclass(name = "LocalConfigurationImprovementRequest", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyLocalConfigurationImprovementRequest {
    baseline_manifest: String,
    candidate_manifest: String,
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
}

#[pymethods]
impl PyLocalConfigurationImprovementRequest {
    /// Collect locators and selectors; the preview call captures and validates them.
    #[new]
    #[pyo3(signature = (*, baseline_manifest, candidate_manifest, project_id, execution_workspace_id, machine_id))]
    pub fn new(
        baseline_manifest: String,
        candidate_manifest: String,
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
    ) -> Self {
        Self {
            baseline_manifest,
            candidate_manifest,
            project_id,
            execution_workspace_id,
            machine_id,
        }
    }
}

/// Local source selection for one complete one-shot compilation.
#[pyclass(name = "LocalOneShotCompilationRequest", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyLocalOneShotCompilationRequest {
    workspace_manifest: String,
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
}

#[pymethods]
impl PyLocalOneShotCompilationRequest {
    /// Bind one absolute workspace entry and its exact selectors.
    #[new]
    #[pyo3(signature = (*, workspace_manifest, project_id, execution_workspace_id, machine_id))]
    pub fn new(
        workspace_manifest: String,
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
    ) -> Self {
        Self {
            workspace_manifest,
            project_id,
            execution_workspace_id,
            machine_id,
        }
    }
}

/// Local source selection for Package distribution inspection.
#[pyclass(name = "LocalPackageDistributionRequest", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyLocalPackageDistributionRequest {
    workspace_manifest: String,
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
}

#[pymethods]
impl PyLocalPackageDistributionRequest {
    /// Bind one absolute workspace entry and its exact selectors.
    #[new]
    #[pyo3(signature = (*, workspace_manifest, project_id, execution_workspace_id, machine_id))]
    pub fn new(
        workspace_manifest: String,
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
    ) -> Self {
        Self {
            workspace_manifest,
            project_id,
            execution_workspace_id,
            machine_id,
        }
    }
}

/// Confined local source for one effect-free native-action request.
#[pyclass(name = "LocalNativeActionInputRequest", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyLocalNativeActionInputRequest {
    action_id: String,
    root: String,
    relative_path: String,
}

#[pymethods]
impl PyLocalNativeActionInputRequest {
    /// Bind an action to one confined file under an absolute retained root.
    #[new]
    pub fn new(action_id: String, root: String, relative_path: String) -> Self {
        Self {
            action_id,
            root,
            relative_path,
        }
    }
}

/// Opaque path-free native-action request captured by the API owner.
#[pyclass(name = "CapturedNativeActionRequest", frozen)]
pub struct PyCapturedNativeActionRequest {
    inner: NativeActionRequest,
}

/// Opaque local compilation retaining API-private host capabilities.
///
/// The handle itself grants no plan admission, mutation, runtime activation,
/// or resident authority. Python can pass it only to the explicit local
/// operations exposed by [`PalePython`].
#[pyclass(name = "LocalOneShotCompilation", frozen)]
pub struct PyLocalOneShotCompilation {
    inner: Arc<LocalOneShotCompilation>,
}

#[pymethods]
impl PyLocalOneShotCompilation {
    /// Return the bounded summary shared with captured one-shot compilation.
    pub fn summary(&self, py: Python<'_>) -> PyOneShotSummary {
        let compilation = Arc::clone(&self.inner);
        py.detach(move || PyOneShotSummary::from_api(compilation.summary()))
    }
}

/// Closed repository or machine-local transition shown by local status.
#[pyclass(
    name = "LocalMaterializationAction",
    eq,
    eq_int,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyLocalMaterializationAction {
    /// Create a previously absent file.
    Create,
    /// Adopt matching pre-existing bytes under explicit adoption policy.
    Adopt,
    /// Replace an owned file whose desired bytes changed.
    Update,
    /// Delete a previously owned file.
    Delete,
    /// Replace only Pale's managed block while preserving other bytes.
    Splice,
}

impl From<LocalMaterializationAction> for PyLocalMaterializationAction {
    fn from(value: LocalMaterializationAction) -> Self {
        match value {
            LocalMaterializationAction::Create => Self::Create,
            LocalMaterializationAction::Adopt => Self::Adopt,
            LocalMaterializationAction::Update => Self::Update,
            LocalMaterializationAction::Delete => Self::Delete,
            LocalMaterializationAction::Splice => Self::Splice,
        }
    }
}

/// Ownership-record transition required by local status.
#[pyclass(
    name = "LocalMaterializationOwnershipChange",
    eq,
    eq_int,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyLocalMaterializationOwnershipChange {
    /// Establish the first trusted ownership state.
    Initialize,
    /// Advance an existing trusted ownership state.
    Advance,
}

impl From<LocalMaterializationOwnershipChange> for PyLocalMaterializationOwnershipChange {
    fn from(value: LocalMaterializationOwnershipChange) -> Self {
        match value {
            LocalMaterializationOwnershipChange::Initialize => Self::Initialize,
            LocalMaterializationOwnershipChange::Advance => Self::Advance,
        }
    }
}

/// Closed machine-local policy effect visible through local status.
#[pyclass(
    name = "LocalMaterializationMachineEffectKind",
    eq,
    eq_int,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyLocalMaterializationMachineEffectKind {
    /// Managed protection in the verified Git exclude file.
    GitExcludeProtection,
}

impl From<LocalMaterializationMachineEffectKind> for PyLocalMaterializationMachineEffectKind {
    fn from(value: LocalMaterializationMachineEffectKind) -> Self {
        match value {
            LocalMaterializationMachineEffectKind::GitExcludeProtection => {
                Self::GitExcludeProtection
            }
        }
    }
}

/// Non-interchangeable local-drift variant.
#[pyclass(
    name = "LocalMaterializationDriftKind",
    eq,
    eq_int,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyLocalMaterializationDriftKind {
    /// One generated repository surface differs.
    Surface,
    /// The trusted ownership record must change.
    Ownership,
    /// One machine-local policy file differs.
    MachineLocal,
}

/// One exact API-derived reason a repository differs from its projection.
#[pyclass(name = "LocalMaterializationDrift", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct PyLocalMaterializationDrift {
    kind: PyLocalMaterializationDriftKind,
    status_label: String,
    relative_path: Option<String>,
    action: Option<PyLocalMaterializationAction>,
    ownership_change: Option<PyLocalMaterializationOwnershipChange>,
    machine_effect_kind: Option<PyLocalMaterializationMachineEffectKind>,
}

impl PyLocalMaterializationDrift {
    fn from_api(value: &LocalMaterializationDrift) -> Self {
        let status_label = value.status_label();
        match value {
            LocalMaterializationDrift::Surface {
                relative_path,
                action,
            } => Self {
                kind: PyLocalMaterializationDriftKind::Surface,
                status_label,
                relative_path: Some(relative_path.clone()),
                action: Some((*action).into()),
                ownership_change: None,
                machine_effect_kind: None,
            },
            LocalMaterializationDrift::Ownership { change } => Self {
                kind: PyLocalMaterializationDriftKind::Ownership,
                status_label,
                relative_path: None,
                action: None,
                ownership_change: Some((*change).into()),
                machine_effect_kind: None,
            },
            LocalMaterializationDrift::MachineLocal {
                kind,
                relative_path,
                action,
            } => Self {
                kind: PyLocalMaterializationDriftKind::MachineLocal,
                status_label,
                relative_path: Some(relative_path.clone()),
                action: Some((*action).into()),
                ownership_change: None,
                machine_effect_kind: Some((*kind).into()),
            },
        }
    }
}

#[pymethods]
impl PyLocalMaterializationDrift {
    /// Closed variant retaining the drift's semantic class.
    #[getter]
    pub fn kind(&self) -> PyLocalMaterializationDriftKind {
        self.kind
    }

    /// Canonical presentation label owned by the API.
    #[getter]
    pub fn status_label(&self) -> &str {
        &self.status_label
    }

    /// Confined relative path when the variant identifies one file.
    #[getter]
    pub fn relative_path(&self) -> Option<String> {
        self.relative_path.clone()
    }

    /// Required file transition for surface or machine-local drift.
    #[getter]
    pub fn action(&self) -> Option<PyLocalMaterializationAction> {
        self.action
    }

    /// Required ownership transition for ownership drift.
    #[getter]
    pub fn ownership_change(&self) -> Option<PyLocalMaterializationOwnershipChange> {
        self.ownership_change
    }

    /// Closed machine-local effect kind for machine-local drift.
    #[getter]
    pub fn machine_effect_kind(&self) -> Option<PyLocalMaterializationMachineEffectKind> {
        self.machine_effect_kind
    }
}

/// Complete read-only comparison for one local repository.
#[pyclass(name = "LocalRepositoryStatus", frozen)]
pub struct PyLocalRepositoryStatus {
    repository_id: String,
    is_clean: bool,
    drift: Vec<PyLocalMaterializationDrift>,
}

impl PyLocalRepositoryStatus {
    fn from_api(value: LocalRepositoryStatus) -> Self {
        Self {
            repository_id: value.repository_id().to_owned(),
            is_clean: value.is_clean(),
            drift: value
                .drift()
                .iter()
                .map(PyLocalMaterializationDrift::from_api)
                .collect(),
        }
    }
}

#[pymethods]
impl PyLocalRepositoryStatus {
    /// Logical repository selected from the admitted compilation.
    #[getter]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Whether every observed surface and ownership fact matches.
    #[getter]
    pub fn is_clean(&self) -> bool {
        self.is_clean
    }

    /// Clone the deterministically ordered drift partition.
    #[getter]
    pub fn drift(&self) -> Vec<PyLocalMaterializationDrift> {
        self.drift.clone()
    }
}

/// Local plan request over one retained compilation.
#[pyclass(name = "LocalOneShotPlanRequest", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyLocalOneShotPlanRequest {
    repository_id: String,
    kind: PyOneShotPlanKind,
}

#[pymethods]
impl PyLocalOneShotPlanRequest {
    /// Bind one repository to the apply, explicit-adoption, or removal law.
    #[new]
    pub fn new(repository_id: String, kind: PyOneShotPlanKind) -> Self {
        Self {
            repository_id,
            kind,
        }
    }
}

/// Flat transition counts derived from one complete local plan.
#[pyclass(name = "LocalOneShotPlanCounts", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct PyLocalOneShotPlanCounts {
    baseline: u32,
    operations: u32,
    total_operations: u32,
    create: u32,
    adopt: u32,
    update: u32,
    unchanged: u32,
    delete: u32,
    splice: u32,
    deferred_effects: u32,
    machine_local_effects: u32,
}

impl From<LocalOneShotPlanCounts> for PyLocalOneShotPlanCounts {
    fn from(value: LocalOneShotPlanCounts) -> Self {
        Self {
            baseline: value.baseline(),
            operations: value.operations(),
            total_operations: value.total_operations(),
            create: value.create(),
            adopt: value.adopt(),
            update: value.update(),
            unchanged: value.unchanged(),
            delete: value.delete(),
            splice: value.splice(),
            deferred_effects: value.deferred_effects(),
            machine_local_effects: value.machine_local_effects(),
        }
    }
}

#[pymethods]
impl PyLocalOneShotPlanCounts {
    /// Number of observed repository targets.
    #[getter]
    pub fn baseline(&self) -> u32 {
        self.baseline
    }

    /// Number of repository-surface operations.
    #[getter]
    pub fn operations(&self) -> u32 {
        self.operations
    }

    /// Total repository-surface and machine-local operations.
    #[getter]
    pub fn total_operations(&self) -> u32 {
        self.total_operations
    }

    /// Number of create transitions.
    #[getter]
    pub fn create(&self) -> u32 {
        self.create
    }

    /// Number of explicit adoption transitions.
    #[getter]
    pub fn adopt(&self) -> u32 {
        self.adopt
    }

    /// Number of replacement transitions.
    #[getter]
    pub fn update(&self) -> u32 {
        self.update
    }

    /// Number of unchanged surfaces.
    #[getter]
    pub fn unchanged(&self) -> u32 {
        self.unchanged
    }

    /// Number of deletion transitions.
    #[getter]
    pub fn delete(&self) -> u32 {
        self.delete
    }

    /// Number of managed-block splice transitions.
    #[getter]
    pub fn splice(&self) -> u32 {
        self.splice
    }

    /// Number of deliberately deferred effects.
    #[getter]
    pub fn deferred_effects(&self) -> u32 {
        self.deferred_effects
    }

    /// Number of bound machine-local effects.
    #[getter]
    pub fn machine_local_effects(&self) -> u32 {
        self.machine_local_effects
    }
}

/// Complete deterministic local plan with privately retained host context.
#[pyclass(name = "LocalOneShotPlan", frozen)]
pub struct PyLocalOneShotPlan {
    inner: Arc<LocalOneShotPlan>,
}

#[pymethods]
impl PyLocalOneShotPlan {
    /// Requested plan operation.
    #[getter]
    pub fn kind(&self) -> PyOneShotPlanKind {
        self.inner.document().kind().into()
    }

    /// Selected logical repository.
    #[getter]
    pub fn repository_id(&self) -> &str {
        self.inner.document().repository_id()
    }

    /// Copy the canonical complete materialization-plan bytes.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, self.inner.document().canonical_bytes())
    }

    /// Return counts derived from the same API-owned plan.
    pub fn counts(&self) -> PyLocalOneShotPlanCounts {
        self.inner.counts().into()
    }
}

/// Binding-safe facts for one atomically published saved plan.
#[pyclass(name = "LocalOneShotPlanPublication", frozen)]
pub struct PyLocalOneShotPlanPublication {
    repository_id: String,
    relative_target: String,
    display_target: String,
}

#[pymethods]
impl PyLocalOneShotPlanPublication {
    /// Logical repository named by the published plan.
    #[getter]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Confined filename under the selected output directory.
    #[getter]
    pub fn relative_target(&self) -> &str {
        &self.relative_target
    }

    /// Ephemeral display spelling of the published target.
    #[getter]
    pub fn display_target(&self) -> &str {
        &self.display_target
    }
}

/// Local directories and plan class admitted for one materialization run.
#[pyclass(name = "LocalMaterializationAdmissionRequest", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyLocalMaterializationAdmissionRequest {
    kind: PyOneShotPlanKind,
    saved_plan_directory: String,
    evidence_directory: String,
}

#[pymethods]
impl PyLocalMaterializationAdmissionRequest {
    /// Bind one plan class to the exact saved-plan and evidence directories.
    #[new]
    pub fn new(
        kind: PyOneShotPlanKind,
        saved_plan_directory: String,
        evidence_directory: String,
    ) -> Self {
        Self {
            kind,
            saved_plan_directory,
            evidence_directory,
        }
    }
}

/// Path-free identity of one repository plan admitted before effects.
#[pyclass(
    name = "LocalMaterializationTargetAdmission",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyLocalMaterializationTargetAdmission {
    repository_id: String,
    plan_digest: String,
}

impl PyLocalMaterializationTargetAdmission {
    fn from_api(value: &LocalMaterializationTargetAdmission) -> Self {
        Self {
            repository_id: value.repository_id().to_owned(),
            plan_digest: value.plan_digest().to_owned(),
        }
    }
}

#[pymethods]
impl PyLocalMaterializationTargetAdmission {
    /// Logical repository whose plan was admitted.
    #[getter]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Digest of the exact saved plan.
    #[getter]
    pub fn plan_digest(&self) -> &str {
        &self.plan_digest
    }
}

/// Opaque admitted local run awaiting settlement or non-effectful drop.
#[pyclass(name = "LocalMaterializationAdmittedRun", frozen)]
pub struct PyLocalMaterializationAdmittedRun {
    inner: Arc<Mutex<LocalMaterializationAdmittedRun>>,
}

#[pymethods]
impl PyLocalMaterializationAdmittedRun {
    /// Requested apply, explicit-adoption, or removal journey.
    ///
    /// # Errors
    ///
    /// Raises [`pyo3::exceptions::PyRuntimeError`] only if a prior panic
    /// poisoned the retained native handle.
    #[getter]
    pub fn kind(&self) -> PyResult<PyOneShotPlanKind> {
        self.inner
            .lock()
            .map(|run| run.kind().into())
            .map_err(|_| poisoned_run())
    }

    /// Ordered repository-plan identities admitted before effects.
    ///
    /// # Errors
    ///
    /// Raises [`pyo3::exceptions::PyRuntimeError`] only if a prior panic
    /// poisoned the retained native handle.
    pub fn targets(&self) -> PyResult<Vec<PyLocalMaterializationTargetAdmission>> {
        self.inner
            .lock()
            .map(|run| {
                run.targets()
                    .iter()
                    .map(PyLocalMaterializationTargetAdmission::from_api)
                    .collect()
            })
            .map_err(|_| poisoned_run())
    }
}

/// Aggregate result of one application-admitted settlement.
#[pyclass(
    name = "LocalMaterializationRunOutcome",
    eq,
    eq_int,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyLocalMaterializationRunOutcome {
    /// Every selected repository settled with an immutable receipt.
    Succeeded,
    /// A successful prefix settled before a later no-effect failure.
    Partial,
    /// The first repository retained no materialization effect.
    FailedNoEffect,
}

impl From<LocalMaterializationRunOutcome> for PyLocalMaterializationRunOutcome {
    fn from(value: LocalMaterializationRunOutcome) -> Self {
        match value {
            LocalMaterializationRunOutcome::Succeeded => Self::Succeeded,
            LocalMaterializationRunOutcome::Partial => Self::Partial,
            LocalMaterializationRunOutcome::FailedNoEffect => Self::FailedNoEffect,
        }
    }
}

/// Outcome of one repository settlement in project order.
#[pyclass(
    name = "LocalMaterializationTargetOutcome",
    eq,
    eq_int,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyLocalMaterializationTargetOutcome {
    /// The immutable receipt preceded repository commit.
    Succeeded,
    /// The repository retained no materialization effect.
    FailedNoEffect,
}

impl From<LocalMaterializationTargetOutcome> for PyLocalMaterializationTargetOutcome {
    fn from(value: LocalMaterializationTargetOutcome) -> Self {
        match value {
            LocalMaterializationTargetOutcome::Succeeded => Self::Succeeded,
            LocalMaterializationTargetOutcome::FailedNoEffect => Self::FailedNoEffect,
        }
    }
}

/// Presentation-neutral facts for one attempted repository settlement.
#[pyclass(name = "LocalMaterializationTargetResult", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct PyLocalMaterializationTargetResult {
    repository_id: String,
    outcome: PyLocalMaterializationTargetOutcome,
    changed_surface_count: u32,
    deferred_effect_count: u32,
    local_git_exclude_changed: bool,
    failure_diagnostic: Option<Vec<u8>>,
}

impl PyLocalMaterializationTargetResult {
    fn from_api(value: &LocalMaterializationTargetResult) -> Self {
        Self {
            repository_id: value.repository_id().to_owned(),
            outcome: value.outcome().into(),
            changed_surface_count: value.changed_surface_count(),
            deferred_effect_count: value.deferred_effect_count(),
            local_git_exclude_changed: value.local_git_exclude_changed(),
            failure_diagnostic: value
                .failure()
                .map(|diagnostic| diagnostic.canonical_bytes().to_vec()),
        }
    }
}

#[pymethods]
impl PyLocalMaterializationTargetResult {
    /// Logical repository attempted by this row.
    #[getter]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Proven per-repository outcome.
    #[getter]
    pub fn outcome(&self) -> PyLocalMaterializationTargetOutcome {
        self.outcome
    }

    /// Number of repository surfaces changed on success.
    #[getter]
    pub fn changed_surface_count(&self) -> u32 {
        self.changed_surface_count
    }

    /// Number of effects kept outside the transaction.
    #[getter]
    pub fn deferred_effect_count(&self) -> u32 {
        self.deferred_effect_count
    }

    /// Whether the managed Git-exclude block changed.
    #[getter]
    pub fn local_git_exclude_changed(&self) -> bool {
        self.local_git_exclude_changed
    }

    /// Copy canonical diagnostic bytes for a no-effect failure, when present.
    #[getter]
    pub fn failure_diagnostic(&self, py: Python<'_>) -> Option<Py<PyBytes>> {
        self.failure_diagnostic
            .as_deref()
            .map(|bytes| crate::python_bytes(py, bytes))
    }
}

/// Binding-safe result of one settled repository-scoped run.
#[pyclass(name = "LocalMaterializationRunResult", frozen)]
pub struct PyLocalMaterializationRunResult {
    kind: PyOneShotPlanKind,
    outcome: PyLocalMaterializationRunOutcome,
    target_count: u32,
    succeeded_target_count: u32,
    targets: Vec<PyLocalMaterializationTargetResult>,
    canonical_bytes: Vec<u8>,
}

impl PyLocalMaterializationRunResult {
    fn from_api(value: LocalMaterializationRunResult) -> Self {
        Self {
            kind: value.kind().into(),
            outcome: value.outcome().into(),
            target_count: value.target_count(),
            succeeded_target_count: value.succeeded_target_count(),
            targets: value
                .targets()
                .iter()
                .map(PyLocalMaterializationTargetResult::from_api)
                .collect(),
            canonical_bytes: value.canonical_bytes().to_vec(),
        }
    }
}

#[pymethods]
impl PyLocalMaterializationRunResult {
    /// Requested plan operation.
    #[getter]
    pub fn kind(&self) -> PyOneShotPlanKind {
        self.kind
    }

    /// Aggregate repository-scoped outcome.
    #[getter]
    pub fn outcome(&self) -> PyLocalMaterializationRunOutcome {
        self.outcome
    }

    /// Number of repository results included.
    #[getter]
    pub fn target_count(&self) -> u32 {
        self.target_count
    }

    /// Number of repositories fully settled.
    #[getter]
    pub fn succeeded_target_count(&self) -> u32 {
        self.succeeded_target_count
    }

    /// Clone ordered result rows for attempted repositories.
    #[getter]
    pub fn targets(&self) -> Vec<PyLocalMaterializationTargetResult> {
        self.targets.clone()
    }

    /// Copy canonical application-admitted `run-result.v1` bytes.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, &self.canonical_bytes)
    }
}

/// Observation of the immutable receipt slot after an unknown outcome.
#[pyclass(
    name = "LocalMaterializationReceiptObservation",
    eq,
    eq_int,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyLocalMaterializationReceiptObservation {
    /// Receipt bytes were not constructed before failure.
    NotConstructed,
    /// Exact admitted receipt bytes and mode are visible.
    Exact,
    /// No receipt is visible at the admitted identity.
    Absent,
    /// Different bytes or mode occupy the admitted identity.
    Different,
}

impl From<LocalMaterializationReceiptObservation> for PyLocalMaterializationReceiptObservation {
    fn from(value: LocalMaterializationReceiptObservation) -> Self {
        match value {
            LocalMaterializationReceiptObservation::NotConstructed => Self::NotConstructed,
            LocalMaterializationReceiptObservation::Exact => Self::Exact,
            LocalMaterializationReceiptObservation::Absent => Self::Absent,
            LocalMaterializationReceiptObservation::Different => Self::Different,
        }
    }
}

/// Trusted ownership-record relation to an admitted transition.
#[pyclass(
    name = "LocalMaterializationOwnershipObservation",
    eq,
    eq_int,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyLocalMaterializationOwnershipObservation {
    /// No trusted ownership record is present.
    Unowned,
    /// The trusted record matches the admitted pre-state.
    MatchesPrior,
    /// The trusted record matches the admitted post-state.
    MatchesPost,
    /// A trusted record matches neither admitted endpoint.
    Other,
}

impl From<LocalMaterializationOwnershipObservation> for PyLocalMaterializationOwnershipObservation {
    fn from(value: LocalMaterializationOwnershipObservation) -> Self {
        match value {
            LocalMaterializationOwnershipObservation::Unowned => Self::Unowned,
            LocalMaterializationOwnershipObservation::MatchesPrior => Self::MatchesPrior,
            LocalMaterializationOwnershipObservation::MatchesPost => Self::MatchesPost,
            LocalMaterializationOwnershipObservation::Other => Self::Other,
        }
    }
}

/// Bounded read-back after settlement reported outcome-unknown.
#[pyclass(name = "LocalMaterializationReconciliation", frozen)]
pub struct PyLocalMaterializationReconciliation {
    repository_id: String,
    plan_digest: String,
    receipt: PyLocalMaterializationReceiptObservation,
    ownership: PyLocalMaterializationOwnershipObservation,
}

impl From<LocalMaterializationReconciliation> for PyLocalMaterializationReconciliation {
    fn from(value: LocalMaterializationReconciliation) -> Self {
        Self {
            repository_id: value.repository_id().to_owned(),
            plan_digest: value.plan_digest().to_owned(),
            receipt: value.receipt().into(),
            ownership: value.ownership().into(),
        }
    }
}

#[pymethods]
impl PyLocalMaterializationReconciliation {
    /// Repository whose unknown settlement is inspected.
    #[getter]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Exact admitted plan identity.
    #[getter]
    pub fn plan_digest(&self) -> &str {
        &self.plan_digest
    }

    /// Current immutable receipt-slot observation.
    #[getter]
    pub fn receipt(&self) -> PyLocalMaterializationReceiptObservation {
        self.receipt
    }

    /// Current trusted ownership-record relation.
    #[getter]
    pub fn ownership(&self) -> PyLocalMaterializationOwnershipObservation {
        self.ownership
    }
}

/// Durable-publication implementation compiled for this native target.
#[pyclass(
    name = "DurablePublicationMechanism",
    eq,
    eq_int,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyDurablePublicationMechanism {
    /// Apple `F_FULLFSYNC`, then parent-directory `fsync`.
    AppleFullFsync,
    /// POSIX file and parent-directory `fsync`.
    UnixFsync,
    /// Rust `sync_all` for the file and parent directory.
    StandardSyncAll,
}

impl From<DurablePublicationMechanism> for PyDurablePublicationMechanism {
    fn from(value: DurablePublicationMechanism) -> Self {
        match value {
            DurablePublicationMechanism::AppleFullFsync => Self::AppleFullFsync,
            DurablePublicationMechanism::UnixFsync => Self::UnixFsync,
            DurablePublicationMechanism::StandardSyncAll => Self::StandardSyncAll,
        }
    }
}

/// Compiled SDK and interface claims, not resident state or process health.
#[pyclass(name = "CompiledInterfaceIdentity", frozen)]
pub struct PyCompiledInterfaceIdentity {
    canonical_bytes: Vec<u8>,
    version: String,
    protocol: String,
    os: String,
    arch: String,
    durable_publication: PyDurablePublicationMechanism,
}

impl From<CompiledInterfaceIdentity> for PyCompiledInterfaceIdentity {
    fn from(value: CompiledInterfaceIdentity) -> Self {
        Self {
            canonical_bytes: value.canonical_bytes().to_vec(),
            version: value.version().to_owned(),
            protocol: value.protocol().to_owned(),
            os: value.os().to_owned(),
            arch: value.arch().to_owned(),
            durable_publication: value.durable_publication().into(),
        }
    }
}

#[pymethods]
impl PyCompiledInterfaceIdentity {
    /// Copy the canonical JCS executable-identity record.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, &self.canonical_bytes)
    }

    /// SDK package version compiled into this extension.
    #[getter]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Interface protocol compiled into this extension.
    #[getter]
    pub fn protocol(&self) -> &str {
        &self.protocol
    }

    /// Operating-system identity compiled for this process.
    #[getter]
    pub fn os(&self) -> &str {
        &self.os
    }

    /// Architecture identity compiled for this process.
    #[getter]
    pub fn arch(&self) -> &str {
        &self.arch
    }

    /// Durable-publication implementation compiled for this process.
    #[getter]
    pub fn durable_publication(&self) -> PyDurablePublicationMechanism {
        self.durable_publication
    }
}

fn poisoned_run() -> PyErr {
    PyRuntimeError::new_err("local materialization-run handle is poisoned")
}

#[pymethods]
impl PalePython {
    /// Capture two local configurations and retain an effect-free review preview.
    ///
    /// Only the baseline supplies Package content. Accepting the returned preview
    /// returns a disposition; it does not persist the decision, write files, or
    /// authorize an effect.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when capture or compilation fails.
    pub fn preview_local_configuration_improvement(
        &self,
        py: Python<'_>,
        request: PyLocalConfigurationImprovementRequest,
    ) -> PyResult<crate::PyConfigurationImprovementPreview> {
        let api = Arc::clone(&self.api);
        let request = LocalConfigurationImprovementRequest::new(
            LocalWorkspaceLocator::new(request.baseline_manifest),
            LocalWorkspaceLocator::new(request.candidate_manifest),
            request.project_id,
            request.execution_workspace_id,
            request.machine_id,
        );
        py.detach(move || api.preview_local_configuration_improvement(request))
            .map(crate::PyConfigurationImprovementPreview::from_api)
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Resolve, stably capture, and compile one local workspace selection.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when the host locator, source
    /// capture, selectors, or canonical compiler rejects input.
    pub fn compile_local_one_shot(
        &self,
        py: Python<'_>,
        request: PyLocalOneShotCompilationRequest,
    ) -> PyResult<PyLocalOneShotCompilation> {
        let api = Arc::clone(&self.api);
        let request = LocalOneShotCompilationRequest::new(
            LocalWorkspaceLocator::new(request.workspace_manifest),
            request.project_id,
            request.execution_workspace_id,
            request.machine_id,
        );
        py.detach(move || api.compile_local_one_shot(request))
            .map(|inner| PyLocalOneShotCompilation {
                inner: Arc::new(inner),
            })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Inspect Package releases selected by one local workspace entry.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when local selection, bounded
    /// Package capture, or Package admission fails.
    pub fn inspect_local_package_distribution(
        &self,
        py: Python<'_>,
        request: PyLocalPackageDistributionRequest,
    ) -> PyResult<PyPackageDistributionInspection> {
        let api = Arc::clone(&self.api);
        let request = LocalPackageDistributionRequest::new(
            LocalWorkspaceLocator::new(request.workspace_manifest),
            request.project_id,
            request.execution_workspace_id,
            request.machine_id,
        );
        py.detach(move || api.inspect_local_package_distribution(request))
            .map(PyPackageDistributionInspection::from_api)
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Read one confined local file into a path-free action request.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when the root, confined relative
    /// path, file type, or byte ceiling is rejected.
    pub fn capture_local_native_action_request(
        &self,
        py: Python<'_>,
        request: PyLocalNativeActionInputRequest,
    ) -> PyResult<PyCapturedNativeActionRequest> {
        let api = Arc::clone(&self.api);
        let request = LocalNativeActionInputRequest::new(
            request.action_id,
            request.root,
            request.relative_path,
        );
        py.detach(move || api.capture_local_native_action_request(request))
            .map(|inner| PyCapturedNativeActionRequest { inner })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Invoke one captured request through the same pure native-action owner.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when the action, tagged input,
    /// or pure execution fails API-owned admission.
    pub fn invoke_captured_native_action(
        &self,
        py: Python<'_>,
        request: &PyCapturedNativeActionRequest,
    ) -> PyResult<Py<PyBytes>> {
        let api = Arc::clone(&self.api);
        let request = request.inner.clone();
        py.detach(move || api.invoke_native_action(request))
            .map(|execution| crate::python_bytes(py, execution.canonical_bytes()))
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Compare one local repository with its admitted desired projection.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when selection, lineage,
    /// baseline, Git authority, or comparison admission fails.
    pub fn inspect_local_repository_status(
        &self,
        py: Python<'_>,
        compilation: &PyLocalOneShotCompilation,
        repository_id: String,
    ) -> PyResult<PyLocalRepositoryStatus> {
        let api = Arc::clone(&self.api);
        let compilation = Arc::clone(&compilation.inner);
        py.detach(move || api.inspect_local_repository_status(&compilation, repository_id))
            .map(PyLocalRepositoryStatus::from_api)
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Reconstruct one complete local plan from trusted observations.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when repository, lineage,
    /// baseline, Git authority, or plan admission fails.
    pub fn plan_local_one_shot(
        &self,
        py: Python<'_>,
        compilation: &PyLocalOneShotCompilation,
        request: PyLocalOneShotPlanRequest,
    ) -> PyResult<PyLocalOneShotPlan> {
        let api = Arc::clone(&self.api);
        let compilation = Arc::clone(&compilation.inner);
        let request = LocalOneShotPlanRequest::new(request.repository_id, request.kind.into());
        py.detach(move || api.plan_local_one_shot(&compilation, request))
            .map(|inner| PyLocalOneShotPlan {
                inner: Arc::new(inner),
            })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Publish one complete plan into a confined local directory.
    ///
    /// This is the explicit native-effect boundary for saved-plan publication.
    /// Constructing the request or plan does not invoke it.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when directory admission,
    /// retained identity, or atomic publication fails.
    pub fn publish_local_one_shot_plan(
        &self,
        py: Python<'_>,
        plan: &PyLocalOneShotPlan,
        output_directory: String,
    ) -> PyResult<PyLocalOneShotPlanPublication> {
        let api = Arc::clone(&self.api);
        let plan = Arc::clone(&plan.inner);
        py.detach(move || api.publish_local_one_shot_plan(&plan, output_directory))
            .map(|value| PyLocalOneShotPlanPublication {
                repository_id: value.repository_id().to_owned(),
                relative_target: value.relative_target().to_owned(),
                display_target: value.display_target().to_owned(),
            })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Admit saved plans and target effects without writing.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when directory spelling,
    /// saved-plan equality, observations, or Git-local preparation fails.
    pub fn admit_local_materialization_run(
        &self,
        py: Python<'_>,
        compilation: &PyLocalOneShotCompilation,
        request: PyLocalMaterializationAdmissionRequest,
    ) -> PyResult<PyLocalMaterializationAdmittedRun> {
        let api = Arc::clone(&self.api);
        let compilation = Arc::clone(&compilation.inner);
        let request = LocalMaterializationAdmissionRequest::new(
            request.kind.into(),
            request.saved_plan_directory,
            request.evidence_directory,
        );
        py.detach(move || api.admit_local_materialization_run(&compilation, request))
            .map(|inner| PyLocalMaterializationAdmittedRun {
                inner: Arc::new(Mutex::new(inner)),
            })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Settle one admitted run in repository order.
    ///
    /// This is an explicit native-effect boundary. Executable identity and every
    /// receipt slot are admitted before the first repository mutation.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] for pre-effect rejection or
    /// outcome-unknown settlement requiring read-back. A poisoned host handle
    /// raises [`pyo3::exceptions::PyRuntimeError`].
    pub fn settle_local_materialization_run(
        &self,
        py: Python<'_>,
        run: &PyLocalMaterializationAdmittedRun,
    ) -> PyResult<PyLocalMaterializationRunResult> {
        let api = Arc::clone(&self.api);
        let run = Arc::clone(&run.inner);
        py.detach(move || {
            let mut run = run.lock().map_err(|_| None)?;
            api.settle_local_materialization_run(&mut run).map_err(Some)
        })
        .map(PyLocalMaterializationRunResult::from_api)
        .map_err(|error| match error {
            Some(error) => diagnostic_error(py, error),
            None => poisoned_run(),
        })
    }

    /// Inspect bounded read-back after an outcome-unknown settlement.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when the run is not unknown or
    /// retained receipt/ownership evidence cannot be observed. A poisoned host
    /// handle raises [`pyo3::exceptions::PyRuntimeError`].
    pub fn inspect_local_materialization_reconciliation(
        &self,
        py: Python<'_>,
        run: &PyLocalMaterializationAdmittedRun,
    ) -> PyResult<PyLocalMaterializationReconciliation> {
        let api = Arc::clone(&self.api);
        let run = Arc::clone(&run.inner);
        py.detach(move || {
            let run = run.lock().map_err(|_| None)?;
            api.inspect_local_materialization_reconciliation(&run)
                .map_err(Some)
        })
        .map(Into::into)
        .map_err(|error| match error {
            Some(error) => diagnostic_error(py, error),
            None => poisoned_run(),
        })
    }

    /// Report compiled SDK and interface claims without reading an executable.
    ///
    /// The record establishes neither executable-byte identity nor resident state.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] if the embedded runtime schema
    /// or canonical record is incoherent.
    pub fn compiled_interface_identity(
        &self,
        py: Python<'_>,
    ) -> PyResult<PyCompiledInterfaceIdentity> {
        let api = Arc::clone(&self.api);
        py.detach(move || api.compiled_interface_identity())
            .map(Into::into)
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Verify the current host executable against one captured workspace binding.
    ///
    /// This checks the Python executable, not the loaded extension's bytes.
    /// Success returns the compiled interface claims; it establishes no resident
    /// state or process health and starts no service.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when no runtime is bound or
    /// executable bytes, version, protocol, or identity disagree.
    pub fn verify_current_executable_binding(
        &self,
        py: Python<'_>,
        compilation: &PyLocalOneShotCompilation,
    ) -> PyResult<PyCompiledInterfaceIdentity> {
        let api = Arc::clone(&self.api);
        let compilation = Arc::clone(&compilation.inner);
        py.detach(move || api.verify_current_executable_binding(&compilation))
            .map(Into::into)
            .map_err(|error| diagnostic_error(py, error))
    }
}
