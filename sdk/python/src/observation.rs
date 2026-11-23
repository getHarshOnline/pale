//! Python projection of API-owned bounded observation.
//!
//! The classes retain owned [`pale_sdk`] values and expose closed variant
//! accessors. Paging, deltas, causal proof, cancellation, backpressure, and
//! lens construction run through the façade while Python is detached; this
//! leaf owns no graph traversal or diagnostic/readiness policy.

mod profile;
pub use profile::{
    PyObservationProfileContinuation, PyObservationProfileError, PyObservationProfileMapping,
    PyObservationProfilePage, PyObservationProfileResult, PyObservationProfileResultKind,
};

use std::sync::Arc;

use pale_sdk::{
    DiagnosticEffectCertainty, ObservationBackpressureRecovery, ObservationCancellation,
    ObservationCategory, ObservationCausalPath, ObservationCausalSelector, ObservationCausalStep,
    ObservationChange, ObservationCursor, ObservationDeltaOutcome, ObservationDeltaRequest,
    ObservationFact, ObservationFilter, ObservationFreshness, ObservationFreshnessClass,
    ObservationGeneration, ObservationHealthState, ObservationLens, ObservationLensCursor,
    ObservationLensDisposition, ObservationLensLimits as ApiObservationLensLimits,
    ObservationLensNode, ObservationLensRequest, ObservationNativeState, ObservationPage,
    ObservationPageCursor, ObservationPageDisposition,
    ObservationPageLimits as ApiObservationPageLimits, ObservationPageRequest,
    ObservationProcessIntent, ObservationReadinessState, ObservationReconciliationState,
    ObservationResynchronization, ObservationResynchronizationReason, ObservationRevision,
    ObservationSnapshot, ObservationStepCompletion, ObservationStepLimit, ObservationSubject,
    ObservationSubjectKind,
};
use pyo3::prelude::*;

use crate::{
    PalePython, PyDiagnosticEffectCertainty, PyEffectiveProfileCompilation, PyOperationExposure,
    PyRequirementDisposition,
};

/// Admitted generation kind carried by an observation cursor.
#[pyclass(
    name = "ObservationGenerationKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationGenerationKind {
    /// Immutable effective-profile generation.
    EffectiveProfile,
    /// Resident authority generation.
    Resident,
}

/// Owned generation coordinate with variant-specific accessors.
#[pyclass(name = "ObservationGeneration", frozen)]
pub struct PyObservationGeneration {
    inner: ObservationGeneration,
}

#[pymethods]
impl PyObservationGeneration {
    /// Return the closed generation kind.
    #[getter]
    pub fn kind(&self) -> PyObservationGenerationKind {
        match self.inner {
            ObservationGeneration::EffectiveProfile { .. } => {
                PyObservationGenerationKind::EffectiveProfile
            }
            ObservationGeneration::Resident { .. } => PyObservationGenerationKind::Resident,
        }
    }

    /// Return the profile digest for an effective-profile generation.
    #[getter]
    pub fn profile_digest(&self) -> Option<String> {
        match &self.inner {
            ObservationGeneration::EffectiveProfile { profile_digest, .. } => {
                Some(profile_digest.clone())
            }
            ObservationGeneration::Resident { .. } => None,
        }
    }

    /// Return the machine-resolved instance digest for an effective profile.
    #[getter]
    pub fn instance_digest(&self) -> Option<String> {
        match &self.inner {
            ObservationGeneration::EffectiveProfile {
                instance_digest, ..
            } => Some(instance_digest.clone()),
            ObservationGeneration::Resident { .. } => None,
        }
    }

    /// Return the composition proof digest that binds an effective profile's causal paths.
    #[getter]
    pub fn composition_plan_digest(&self) -> Option<String> {
        match &self.inner {
            ObservationGeneration::EffectiveProfile {
                composition_plan_digest,
                ..
            } => Some(composition_plan_digest.clone()),
            ObservationGeneration::Resident { .. } => None,
        }
    }

    /// Return the stable authority-store identity for a resident generation.
    #[getter]
    pub fn control_store_id(&self) -> Option<String> {
        match &self.inner {
            ObservationGeneration::Resident {
                control_store_id, ..
            } => Some(control_store_id.clone()),
            ObservationGeneration::EffectiveProfile { .. } => None,
        }
    }

    /// Return the authority-store incarnation for a resident generation.
    #[getter]
    pub fn control_incarnation_id(&self) -> Option<String> {
        match &self.inner {
            ObservationGeneration::Resident {
                control_incarnation_id,
                ..
            } => Some(control_incarnation_id.clone()),
            ObservationGeneration::EffectiveProfile { .. } => None,
        }
    }

    /// Return the active digest for a resident generation, when selected.
    #[getter]
    pub fn active_generation_digest(&self) -> Option<String> {
        match &self.inner {
            ObservationGeneration::Resident {
                active_generation_digest,
                ..
            } => active_generation_digest.clone(),
            ObservationGeneration::EffectiveProfile { .. } => None,
        }
    }
}

/// Revision kind within one admitted generation.
#[pyclass(
    name = "ObservationRevisionKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationRevisionKind {
    /// Immutable compiler phase.
    Static,
    /// Durable resident-journal head.
    ResidentJournal,
}

/// Owned revision coordinate.
#[pyclass(name = "ObservationRevision", frozen)]
pub struct PyObservationRevision {
    inner: ObservationRevision,
}

#[pymethods]
impl PyObservationRevision {
    /// Return the closed revision kind.
    #[getter]
    pub fn kind(&self) -> PyObservationRevisionKind {
        match self.inner {
            ObservationRevision::Static => PyObservationRevisionKind::Static,
            ObservationRevision::ResidentJournal { .. } => {
                PyObservationRevisionKind::ResidentJournal
            }
        }
    }

    /// Return the durable resident sequence when it exists.
    #[getter]
    pub fn sequence(&self) -> Option<u64> {
        match self.inner {
            ObservationRevision::ResidentJournal { sequence } => sequence,
            ObservationRevision::Static => None,
        }
    }
}

/// Opaque generation/revision cursor issued by the API.
#[pyclass(name = "ObservationCursor", frozen)]
pub struct PyObservationCursor {
    inner: ObservationCursor,
}

#[pymethods]
impl PyObservationCursor {
    /// Return the admitted generation coordinate.
    #[getter]
    pub fn generation(&self) -> PyObservationGeneration {
        PyObservationGeneration {
            inner: self.inner.generation().clone(),
        }
    }

    /// Return the revision within [`Self::generation`].
    #[getter]
    pub fn revision(&self) -> PyObservationRevision {
        PyObservationRevision {
            inner: self.inner.revision().clone(),
        }
    }
}

/// Independent observation axis.
#[pyclass(name = "ObservationCategory", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationCategory {
    /// Exact operation exposure.
    OperationExposure,
    /// Portable capability-requirement disposition.
    CapabilityRequirementDisposition,
    /// Health evidence.
    Health,
    /// Readiness evidence.
    Readiness,
    /// External-effect certainty.
    Effect,
    /// Native-owner evidence.
    NativeOwner,
    /// Reconciliation state.
    Reconciliation,
    /// Desired process state.
    ProcessIntent,
}

impl From<PyObservationCategory> for ObservationCategory {
    fn from(value: PyObservationCategory) -> Self {
        match value {
            PyObservationCategory::OperationExposure => Self::OperationExposure,
            PyObservationCategory::CapabilityRequirementDisposition => {
                Self::CapabilityRequirementDisposition
            }
            PyObservationCategory::Health => Self::Health,
            PyObservationCategory::Readiness => Self::Readiness,
            PyObservationCategory::Effect => Self::Effect,
            PyObservationCategory::NativeOwner => Self::NativeOwner,
            PyObservationCategory::Reconciliation => Self::Reconciliation,
            PyObservationCategory::ProcessIntent => Self::ProcessIntent,
        }
    }
}

fn category_from_api(value: ObservationCategory) -> PyObservationCategory {
    match value {
        ObservationCategory::OperationExposure => PyObservationCategory::OperationExposure,
        ObservationCategory::CapabilityRequirementDisposition => {
            PyObservationCategory::CapabilityRequirementDisposition
        }
        ObservationCategory::Health => PyObservationCategory::Health,
        ObservationCategory::Readiness => PyObservationCategory::Readiness,
        ObservationCategory::Effect => PyObservationCategory::Effect,
        ObservationCategory::NativeOwner => PyObservationCategory::NativeOwner,
        ObservationCategory::Reconciliation => PyObservationCategory::Reconciliation,
        ObservationCategory::ProcessIntent => PyObservationCategory::ProcessIntent,
    }
}

/// Exact API-issued observation subject.
#[pyclass(name = "ObservationSubject", frozen)]
pub struct PyObservationSubject {
    inner: ObservationSubject,
}

#[pymethods]
impl PyObservationSubject {
    /// Return the closed subject kind.
    #[getter]
    pub fn kind(&self) -> PyObservationSubjectKind {
        self.inner.kind().into()
    }

    /// Return the profile digest when applicable.
    #[getter]
    pub fn profile_digest(&self) -> Option<&str> {
        self.inner.profile_digest()
    }

    /// Return the machine identity when applicable.
    #[getter]
    pub fn machine_id(&self) -> Option<&str> {
        self.inner.machine_id()
    }

    /// Return the repository identity when applicable.
    #[getter]
    pub fn repository_id(&self) -> Option<&str> {
        self.inner.repository_id()
    }

    /// Return the operation identity when applicable.
    #[getter]
    pub fn operation_id(&self) -> Option<&str> {
        self.inner.operation_id()
    }

    /// Return the requirement identity when applicable.
    #[getter]
    pub fn requirement_id(&self) -> Option<&str> {
        self.inner.requirement_id()
    }

    /// Return the capability identity when applicable.
    #[getter]
    pub fn capability_id(&self) -> Option<&str> {
        self.inner.capability_id()
    }

    /// Return the execution-path identity when applicable.
    #[getter]
    pub fn execution_path_id(&self) -> Option<&str> {
        self.inner.execution_path_id()
    }

    /// Return the compiled-generation digest when applicable.
    #[getter]
    pub fn compiled_generation_digest(&self) -> Option<&str> {
        self.inner.compiled_generation_digest()
    }
}

/// Closed kind of an API-issued subject.
#[pyclass(
    name = "ObservationSubjectKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationSubjectKind {
    /// Complete effective profile.
    EffectiveProfile,
    /// Selected machine.
    Machine,
    /// Logical repository.
    Repository,
    /// Repository-scoped operation.
    Operation,
    /// Capability-requirement occurrence.
    CapabilityRequirement,
    /// Selected execution path.
    ExecutionPath,
    /// Immutable compiled generation.
    CompiledGeneration,
    /// Resident process intent.
    ResidentProcess,
}

impl From<ObservationSubjectKind> for PyObservationSubjectKind {
    fn from(value: ObservationSubjectKind) -> Self {
        match value {
            ObservationSubjectKind::EffectiveProfile => Self::EffectiveProfile,
            ObservationSubjectKind::Machine => Self::Machine,
            ObservationSubjectKind::Repository => Self::Repository,
            ObservationSubjectKind::Operation => Self::Operation,
            ObservationSubjectKind::CapabilityRequirement => Self::CapabilityRequirement,
            ObservationSubjectKind::ExecutionPath => Self::ExecutionPath,
            ObservationSubjectKind::CompiledGeneration => Self::CompiledGeneration,
            ObservationSubjectKind::ResidentProcess => Self::ResidentProcess,
        }
    }
}

/// Freshness kind for one observation fact.
#[pyclass(
    name = "ObservationFreshnessKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationFreshnessKind {
    /// Current for the snapshot.
    Fresh,
    /// Retained from an older revision.
    Stale,
    /// Currency is not established.
    Unknown,
}

/// Temporal qualification of one fact.
#[pyclass(name = "ObservationFreshness", frozen)]
pub struct PyObservationFreshness {
    inner: ObservationFreshness,
}

#[pymethods]
impl PyObservationFreshness {
    /// Return the closed freshness kind.
    #[getter]
    pub fn kind(&self) -> PyObservationFreshnessKind {
        match self.inner {
            ObservationFreshness::Fresh => PyObservationFreshnessKind::Fresh,
            ObservationFreshness::Stale { .. } => PyObservationFreshnessKind::Stale,
            ObservationFreshness::Unknown => PyObservationFreshnessKind::Unknown,
        }
    }

    /// Return the exact stale distance when applicable.
    #[getter]
    pub fn revisions_behind(&self) -> Option<u64> {
        match self.inner {
            ObservationFreshness::Stale { revisions_behind } => Some(revisions_behind),
            ObservationFreshness::Fresh | ObservationFreshness::Unknown => None,
        }
    }
}

/// Freshness filter accepted by bounded observation requests.
#[pyclass(name = "ObservationFreshnessClass", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationFreshnessClass {
    /// Current facts only.
    Fresh,
    /// Stale facts only.
    Stale,
    /// Unknown-currency facts only.
    Unknown,
}

impl From<PyObservationFreshnessClass> for ObservationFreshnessClass {
    fn from(value: PyObservationFreshnessClass) -> Self {
        match value {
            PyObservationFreshnessClass::Fresh => Self::Fresh,
            PyObservationFreshnessClass::Stale => Self::Stale,
            PyObservationFreshnessClass::Unknown => Self::Unknown,
        }
    }
}

/// Health state independent of readiness.
#[pyclass(
    name = "ObservationHealthState",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationHealthState {
    /// Applicable health predicate passed.
    Healthy,
    /// Reduced but usable.
    Degraded,
    /// Unusable.
    Unhealthy,
    /// Owner evidence absent.
    Unknown,
    /// Carrier absent.
    Unavailable,
}

/// Readiness state independent of health and native evidence.
#[pyclass(
    name = "ObservationReadinessState",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationReadinessState {
    /// Applicable readiness predicate passed.
    Ready,
    /// Policy forbids the subject.
    Blocked,
    /// Positive policy omitted the subject.
    Narrowed,
    /// Native evidence is absent.
    Unproven,
    /// Required binding or carrier is absent.
    Unavailable,
}

/// Native-owner evidence kind.
#[pyclass(
    name = "ObservationNativeStateKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationNativeStateKind {
    /// No carrier exists.
    Unavailable,
    /// Owner selected without qualifying evidence.
    Unknown,
    /// Exact owner evidence admitted.
    Observed,
}

/// Native-owner evidence without copied native payload.
#[pyclass(name = "ObservationNativeState", frozen)]
pub struct PyObservationNativeState {
    inner: ObservationNativeState,
}

#[pymethods]
impl PyObservationNativeState {
    /// Return the closed native-evidence kind.
    #[getter]
    pub fn kind(&self) -> PyObservationNativeStateKind {
        match self.inner {
            ObservationNativeState::Unavailable => PyObservationNativeStateKind::Unavailable,
            ObservationNativeState::Unknown => PyObservationNativeStateKind::Unknown,
            ObservationNativeState::Observed { .. } => PyObservationNativeStateKind::Observed,
        }
    }

    /// Return the native system retaining source truth when observed.
    #[getter]
    pub fn owner_system_id(&self) -> Option<&str> {
        match &self.inner {
            ObservationNativeState::Observed {
                owner_system_id, ..
            } => Some(owner_system_id),
            _ => None,
        }
    }

    /// Return the owner-defined observation kind when observed.
    #[getter]
    pub fn observation_kind(&self) -> Option<&str> {
        match &self.inner {
            ObservationNativeState::Observed {
                observation_kind, ..
            } => Some(observation_kind),
            _ => None,
        }
    }
}

/// Reconciliation-state kind independent of effect certainty.
#[pyclass(
    name = "ObservationReconciliationStateKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationReconciliationStateKind {
    /// No reconciliation obligation.
    NotRequired,
    /// Unresolved case retained.
    Required,
    /// Case in progress.
    InProgress,
    /// Terminal result admitted.
    Settled,
    /// Frontier unobserved.
    Unknown,
    /// Carrier absent.
    Unavailable,
}

/// Reconciliation state and exact case/result coordinate.
#[pyclass(name = "ObservationReconciliationState", frozen)]
pub struct PyObservationReconciliationState {
    inner: ObservationReconciliationState,
}

#[pymethods]
impl PyObservationReconciliationState {
    /// Return the closed reconciliation kind.
    #[getter]
    pub fn kind(&self) -> PyObservationReconciliationStateKind {
        match self.inner {
            ObservationReconciliationState::NotRequired => {
                PyObservationReconciliationStateKind::NotRequired
            }
            ObservationReconciliationState::Required { .. } => {
                PyObservationReconciliationStateKind::Required
            }
            ObservationReconciliationState::InProgress { .. } => {
                PyObservationReconciliationStateKind::InProgress
            }
            ObservationReconciliationState::Settled { .. } => {
                PyObservationReconciliationStateKind::Settled
            }
            ObservationReconciliationState::Unknown => {
                PyObservationReconciliationStateKind::Unknown
            }
            ObservationReconciliationState::Unavailable => {
                PyObservationReconciliationStateKind::Unavailable
            }
        }
    }

    /// Return the active reconciliation case digest, when any.
    #[getter]
    pub fn case_digest(&self) -> Option<&str> {
        match &self.inner {
            ObservationReconciliationState::Required { case_digest }
            | ObservationReconciliationState::InProgress { case_digest } => Some(case_digest),
            _ => None,
        }
    }

    /// Return the terminal result digest when settled.
    #[getter]
    pub fn result_digest(&self) -> Option<&str> {
        match &self.inner {
            ObservationReconciliationState::Settled { result_digest } => Some(result_digest),
            _ => None,
        }
    }
}

/// Desired resident process state, not observed liveness.
#[pyclass(
    name = "ObservationProcessIntent",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationProcessIntent {
    /// Desired running.
    Running,
    /// Desired stopped.
    Stopped,
}

/// One independently typed observation fact.
#[pyclass(name = "ObservationFact", frozen)]
pub struct PyObservationFact {
    inner: ObservationFact,
}

#[pymethods]
impl PyObservationFact {
    /// Return the independent fact axis.
    #[getter]
    pub fn category(&self) -> PyObservationCategory {
        category_from_api(match self.inner {
            ObservationFact::OperationExposure { .. } => ObservationCategory::OperationExposure,
            ObservationFact::CapabilityRequirementDisposition { .. } => {
                ObservationCategory::CapabilityRequirementDisposition
            }
            ObservationFact::Health { .. } => ObservationCategory::Health,
            ObservationFact::Readiness { .. } => ObservationCategory::Readiness,
            ObservationFact::Effect { .. } => ObservationCategory::Effect,
            ObservationFact::NativeOwner { .. } => ObservationCategory::NativeOwner,
            ObservationFact::Reconciliation { .. } => ObservationCategory::Reconciliation,
            ObservationFact::ProcessIntent { .. } => ObservationCategory::ProcessIntent,
        })
    }

    /// Return the exact subject when this axis is subject-scoped.
    #[getter]
    pub fn subject(&self) -> Option<PyObservationSubject> {
        match &self.inner {
            ObservationFact::OperationExposure { subject, .. }
            | ObservationFact::CapabilityRequirementDisposition { subject, .. }
            | ObservationFact::Health { subject, .. }
            | ObservationFact::Readiness { subject, .. }
            | ObservationFact::Effect { subject, .. }
            | ObservationFact::NativeOwner { subject, .. }
            | ObservationFact::Reconciliation { subject, .. } => Some(PyObservationSubject {
                inner: subject.clone(),
            }),
            ObservationFact::ProcessIntent { .. } => None,
        }
    }

    /// Return the fact's temporal qualification.
    #[getter]
    pub fn freshness(&self) -> PyObservationFreshness {
        let inner = match self.inner {
            ObservationFact::OperationExposure { freshness, .. }
            | ObservationFact::CapabilityRequirementDisposition { freshness, .. }
            | ObservationFact::Health { freshness, .. }
            | ObservationFact::Readiness { freshness, .. }
            | ObservationFact::Effect { freshness, .. }
            | ObservationFact::NativeOwner { freshness, .. }
            | ObservationFact::Reconciliation { freshness, .. }
            | ObservationFact::ProcessIntent { freshness, .. } => freshness,
        };
        PyObservationFreshness { inner }
    }

    /// Return operation exposure only on that axis.
    #[getter]
    pub fn operation_exposure(&self) -> Option<PyOperationExposure> {
        match self.inner {
            ObservationFact::OperationExposure { state, .. } => {
                Some(PyOperationExposure::from_api(state))
            }
            _ => None,
        }
    }

    /// Return capability disposition only on that independent axis.
    #[getter]
    pub(crate) fn requirement_disposition(&self) -> Option<PyRequirementDisposition> {
        match self.inner {
            ObservationFact::CapabilityRequirementDisposition { state, .. } => {
                Some(PyRequirementDisposition::from_api(state))
            }
            _ => None,
        }
    }

    /// Return health state only on the health axis.
    #[getter]
    pub fn health_state(&self) -> Option<PyObservationHealthState> {
        match self.inner {
            ObservationFact::Health { state, .. } => Some(match state {
                ObservationHealthState::Healthy => PyObservationHealthState::Healthy,
                ObservationHealthState::Degraded => PyObservationHealthState::Degraded,
                ObservationHealthState::Unhealthy => PyObservationHealthState::Unhealthy,
                ObservationHealthState::Unknown => PyObservationHealthState::Unknown,
                ObservationHealthState::Unavailable => PyObservationHealthState::Unavailable,
            }),
            _ => None,
        }
    }

    /// Return readiness only on the readiness axis.
    #[getter]
    pub fn readiness_state(&self) -> Option<PyObservationReadinessState> {
        match self.inner {
            ObservationFact::Readiness { state, .. } => Some(match state {
                ObservationReadinessState::Ready => PyObservationReadinessState::Ready,
                ObservationReadinessState::Blocked => PyObservationReadinessState::Blocked,
                ObservationReadinessState::Narrowed => PyObservationReadinessState::Narrowed,
                ObservationReadinessState::Unproven => PyObservationReadinessState::Unproven,
                ObservationReadinessState::Unavailable => PyObservationReadinessState::Unavailable,
            }),
            _ => None,
        }
    }

    /// Return effect certainty only on the effect axis.
    #[getter]
    pub fn effect_certainty(&self) -> Option<PyDiagnosticEffectCertainty> {
        match self.inner {
            ObservationFact::Effect { certainty, .. } => Some(match certainty {
                DiagnosticEffectCertainty::NotApplicable => {
                    PyDiagnosticEffectCertainty::NotApplicable
                }
                DiagnosticEffectCertainty::NotStarted => PyDiagnosticEffectCertainty::NotStarted,
                DiagnosticEffectCertainty::Confirmed => PyDiagnosticEffectCertainty::Confirmed,
                DiagnosticEffectCertainty::OutcomeUnknown => {
                    PyDiagnosticEffectCertainty::OutcomeUnknown
                }
            }),
            _ => None,
        }
    }

    /// Return native evidence only on the native-owner axis.
    #[getter]
    pub fn native_state(&self) -> Option<PyObservationNativeState> {
        match &self.inner {
            ObservationFact::NativeOwner { state, .. } => Some(PyObservationNativeState {
                inner: state.clone(),
            }),
            _ => None,
        }
    }

    /// Return reconciliation state only on that axis.
    #[getter]
    pub fn reconciliation_state(&self) -> Option<PyObservationReconciliationState> {
        match &self.inner {
            ObservationFact::Reconciliation { state, .. } => {
                Some(PyObservationReconciliationState {
                    inner: state.clone(),
                })
            }
            _ => None,
        }
    }

    /// Return process intent only on that axis.
    #[getter]
    pub fn process_intent(&self) -> Option<PyObservationProcessIntent> {
        match self.inner {
            ObservationFact::ProcessIntent { state, .. } => Some(match state {
                ObservationProcessIntent::Running => PyObservationProcessIntent::Running,
                ObservationProcessIntent::Stopped => PyObservationProcessIntent::Stopped,
            }),
            _ => None,
        }
    }
}

/// Opaque API-owned filter; setters compose the canonical filter algebra.
#[pyclass(name = "ObservationFilter")]
pub struct PyObservationFilter {
    inner: ObservationFilter,
}

#[pymethods]
impl PyObservationFilter {
    /// Construct an unrestricted filter.
    #[new]
    pub fn new() -> Self {
        Self {
            inner: ObservationFilter::all(),
        }
    }

    /// Restrict this filter to one independent category.
    pub fn select_category(&mut self, category: PyObservationCategory) {
        self.inner = self.inner.clone().with_category(category.into());
    }

    /// Restrict this filter to one API-issued subject.
    pub fn select_subject(&mut self, subject: &PyObservationSubject) {
        self.inner = self.inner.clone().with_subject(&subject.inner);
    }

    /// Restrict this filter to one freshness class.
    pub fn select_freshness(&mut self, freshness: PyObservationFreshnessClass) {
        self.inner = self.inner.clone().with_freshness(freshness.into());
    }
}

/// Work and copy limits for one observation-page step.
///
/// Zero selects the application's default. The application caps larger values
/// at its owned ceilings.
/// A returned cursor binds these limits to the exact snapshot and filter.
/// Reaching a limit does not prove that later facts are absent.
#[pyclass(name = "ObservationPageLimits", frozen, get_all)]
pub struct PyObservationPageLimits {
    /// Maximum facts this step can inspect.
    pub maximum_inspected: u16,
    /// Maximum facts this step can copy.
    pub maximum_returned: u16,
    /// Maximum UTF-8 string bytes this step can copy.
    pub maximum_copied_string_bytes: u32,
}

#[pymethods]
impl PyObservationPageLimits {
    /// Select the independent work and output limits for one page step.
    #[new]
    pub fn new(
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_string_bytes: u32,
    ) -> Self {
        Self {
            maximum_inspected,
            maximum_returned,
            maximum_copied_string_bytes,
        }
    }
}

/// Work and copy limits for one observation-lens step.
///
/// A returned cursor binds these limits to the exact snapshot and filter.
/// Reaching a limit does not prove that later subjects are absent.
#[pyclass(name = "ObservationLensLimits", frozen, get_all)]
pub struct PyObservationLensLimits {
    /// Maximum subjects this step can inspect.
    pub maximum_inspected: u16,
    /// Maximum lens nodes this step can copy.
    pub maximum_nodes: u16,
    /// Maximum UTF-8 string bytes this step can copy.
    pub maximum_copied_string_bytes: u32,
    /// Maximum retained causal paths this step can look up.
    pub maximum_causal_path_lookups: u16,
    /// Maximum causal steps this step can copy.
    pub maximum_causal_steps: u16,
}

#[pymethods]
impl PyObservationLensLimits {
    /// Select the independent work and output limits for one lens step.
    #[new]
    pub fn new(
        maximum_inspected: u16,
        maximum_nodes: u16,
        maximum_copied_string_bytes: u32,
        maximum_causal_path_lookups: u16,
        maximum_causal_steps: u16,
    ) -> Self {
        Self {
            maximum_inspected,
            maximum_nodes,
            maximum_copied_string_bytes,
            maximum_causal_path_lookups,
            maximum_causal_steps,
        }
    }
}

impl Default for PyObservationFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Opaque continuation for one exact filtered snapshot page.
///
/// It binds the snapshot, filter, raw limits, and next fact. Pass it only to
/// the same query. Its position does not prove that later facts are absent.
#[pyclass(name = "ObservationPageCursor", frozen)]
pub struct PyObservationPageCursor {
    inner: ObservationPageCursor,
}

#[pymethods]
impl PyObservationPageCursor {
    /// Return the exact snapshot cursor.
    #[getter]
    pub fn snapshot(&self) -> PyObservationCursor {
        PyObservationCursor {
            inner: self.inner.snapshot().clone(),
        }
    }

    /// Return how many retained facts precede the next step.
    #[getter]
    pub fn inspected_position(&self) -> u32 {
        self.inner.inspected_position()
    }
}

/// Completion state of one bounded observation step.
#[pyclass(
    name = "ObservationStepCompletionKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationStepCompletionKind {
    /// The selected input for this snapshot and filter was exhausted.
    Complete,
    /// More input can remain; continue with the returned cursor.
    Stopped,
    /// The next indivisible result cannot fit, so these limits cannot continue.
    Incomplete,
}

/// Resource that bounded an observation step.
#[pyclass(
    name = "ObservationStepLimitKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationStepLimitKind {
    /// Retained-input inspection.
    Inspected,
    /// Returned items.
    Returned,
    /// Copied variable-length text.
    CopiedStringBytes,
    /// Causal-path lookups.
    CausalPathLookups,
    /// Causal steps.
    CausalSteps,
}

fn completion_kind(value: ObservationStepCompletion) -> PyObservationStepCompletionKind {
    match value {
        ObservationStepCompletion::Complete => PyObservationStepCompletionKind::Complete,
        ObservationStepCompletion::Stopped(_) => PyObservationStepCompletionKind::Stopped,
        ObservationStepCompletion::Incomplete(_) => PyObservationStepCompletionKind::Incomplete,
    }
}

fn completion_limit(value: ObservationStepCompletion) -> Option<ObservationStepLimit> {
    match value {
        ObservationStepCompletion::Complete => None,
        ObservationStepCompletion::Stopped(limit)
        | ObservationStepCompletion::Incomplete(limit) => Some(limit),
    }
}

fn limit_kind(value: ObservationStepLimit) -> PyObservationStepLimitKind {
    match value {
        ObservationStepLimit::Inspected => PyObservationStepLimitKind::Inspected,
        ObservationStepLimit::Returned => PyObservationStepLimitKind::Returned,
        ObservationStepLimit::CopiedStringBytes { .. } => {
            PyObservationStepLimitKind::CopiedStringBytes
        }
        ObservationStepLimit::CausalPathLookups => PyObservationStepLimitKind::CausalPathLookups,
        ObservationStepLimit::CausalSteps { .. } => PyObservationStepLimitKind::CausalSteps,
    }
}

fn minimum_required(value: ObservationStepLimit) -> Option<u32> {
    match value {
        ObservationStepLimit::CopiedStringBytes { minimum_required } => Some(minimum_required),
        ObservationStepLimit::CausalSteps { minimum_required } => Some(u32::from(minimum_required)),
        ObservationStepLimit::Inspected
        | ObservationStepLimit::Returned
        | ObservationStepLimit::CausalPathLookups => None,
    }
}

/// One valid bounded observation-page step.
#[pyclass(name = "ObservationPage", frozen)]
pub struct PyObservationPage {
    inner: ObservationPage,
}

#[pymethods]
impl PyObservationPage {
    /// Return the exact snapshot cursor.
    #[getter]
    pub fn cursor(&self) -> PyObservationCursor {
        PyObservationCursor {
            inner: self.inner.cursor.clone(),
        }
    }

    /// Return deterministically ordered typed facts.
    #[getter]
    pub fn facts(&self) -> Vec<PyObservationFact> {
        self.inner
            .facts
            .iter()
            .cloned()
            .map(|inner| PyObservationFact { inner })
            .collect()
    }

    /// Return the continuation when matching facts remain.
    #[getter]
    pub fn continuation(&self) -> Option<PyObservationPageCursor> {
        self.inner
            .continuation
            .clone()
            .map(|inner| PyObservationPageCursor { inner })
    }

    /// Return whether this step exhausted its selected input.
    #[getter]
    pub fn completion(&self) -> PyObservationStepCompletionKind {
        completion_kind(self.inner.completion)
    }

    /// Return the resource that stopped this step, if any.
    #[getter]
    pub fn stopped_by(&self) -> Option<PyObservationStepLimitKind> {
        completion_limit(self.inner.completion).map(limit_kind)
    }

    /// Return the minimum bound needed by an indivisible result, if applicable.
    #[getter]
    pub fn minimum_required(&self) -> Option<u32> {
        completion_limit(self.inner.completion).and_then(minimum_required)
    }

    /// Return retained facts examined by this step.
    #[getter]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected
    }

    /// Return variable-length UTF-8 bytes copied by this step.
    #[getter]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes
    }

    /// Return facts omitted when the bounded snapshot was constructed.
    #[getter]
    pub fn snapshot_omitted_count(&self) -> u32 {
        self.inner.snapshot_omitted_count
    }
}

/// Full-snapshot recovery reason.
#[pyclass(
    name = "ObservationResynchronizationReason",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationResynchronizationReason {
    /// Generation mismatch.
    GenerationMismatch,
    /// Revision gap.
    RevisionGap,
    /// Page or lens continuation mismatch.
    ContinuationMismatch,
}

/// Typed instruction to request a new complete snapshot.
#[pyclass(name = "ObservationResynchronization", frozen)]
pub struct PyObservationResynchronization {
    inner: ObservationResynchronization,
}

#[pymethods]
impl PyObservationResynchronization {
    /// Return the exact recovery reason.
    #[getter]
    pub fn reason(&self) -> PyObservationResynchronizationReason {
        match self.inner.reason {
            ObservationResynchronizationReason::GenerationMismatch { .. } => {
                PyObservationResynchronizationReason::GenerationMismatch
            }
            ObservationResynchronizationReason::RevisionGap { .. } => {
                PyObservationResynchronizationReason::RevisionGap
            }
            ObservationResynchronizationReason::ContinuationMismatch => {
                PyObservationResynchronizationReason::ContinuationMismatch
            }
        }
    }

    /// Return the current cursor from which to restart.
    #[getter]
    pub fn current(&self) -> PyObservationCursor {
        PyObservationCursor {
            inner: self.inner.current.as_ref().clone(),
        }
    }

    /// Return the caller-held generation when a generation mismatch occurred.
    #[getter]
    pub fn requested_generation(&self) -> Option<PyObservationGeneration> {
        match &self.inner.reason {
            ObservationResynchronizationReason::GenerationMismatch { requested, .. } => {
                Some(PyObservationGeneration {
                    inner: requested.as_ref().clone(),
                })
            }
            _ => None,
        }
    }

    /// Return the current generation when a generation mismatch occurred.
    #[getter]
    pub fn current_generation(&self) -> Option<PyObservationGeneration> {
        match &self.inner.reason {
            ObservationResynchronizationReason::GenerationMismatch { current, .. } => {
                Some(PyObservationGeneration {
                    inner: current.as_ref().clone(),
                })
            }
            _ => None,
        }
    }

    /// Return the caller-held revision when a revision gap occurred.
    #[getter]
    pub fn requested_revision(&self) -> Option<PyObservationRevision> {
        match &self.inner.reason {
            ObservationResynchronizationReason::RevisionGap { requested, .. } => {
                Some(PyObservationRevision {
                    inner: requested.clone(),
                })
            }
            _ => None,
        }
    }

    /// Return the current revision when a revision gap occurred.
    #[getter]
    pub fn current_revision(&self) -> Option<PyObservationRevision> {
        match &self.inner.reason {
            ObservationResynchronizationReason::RevisionGap { current, .. } => {
                Some(PyObservationRevision {
                    inner: current.clone(),
                })
            }
            _ => None,
        }
    }
}

/// Page query disposition.
#[pyclass(
    name = "ObservationPageResultKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationPageResultKind {
    /// A valid page returned; inspect its completion before assuming exhaustion.
    Ready,
    /// Cancellation was observed before the step began.
    Cancelled,
    /// The supplied cursor cannot continue; obtain a fresh snapshot.
    Resynchronize,
}

/// Typed page query result; resynchronization is data, not an exception.
#[pyclass(name = "ObservationPageResult", frozen)]
pub struct PyObservationPageResult {
    page: Option<ObservationPage>,
    cancelled_continuation: Option<ObservationPageCursor>,
    resynchronization: Option<ObservationResynchronization>,
}

#[pymethods]
impl PyObservationPageResult {
    /// Return the closed result kind.
    #[getter]
    pub fn kind(&self) -> PyObservationPageResultKind {
        if self.page.is_some() {
            PyObservationPageResultKind::Ready
        } else if self.cancelled_continuation.is_some() {
            PyObservationPageResultKind::Cancelled
        } else {
            PyObservationPageResultKind::Resynchronize
        }
    }

    /// Return the bounded page when ready.
    #[getter]
    pub fn page(&self) -> Option<PyObservationPage> {
        self.page.clone().map(|inner| PyObservationPage { inner })
    }

    /// Return the unchanged continuation after cancellation, if any.
    #[getter]
    pub fn cancelled_continuation(&self) -> Option<PyObservationPageCursor> {
        self.cancelled_continuation
            .clone()
            .map(|inner| PyObservationPageCursor { inner })
    }

    /// Return the full-snapshot instruction when resynchronization is required.
    #[getter]
    pub fn resynchronization(&self) -> Option<PyObservationResynchronization> {
        self.resynchronization
            .clone()
            .map(|inner| PyObservationResynchronization { inner })
    }
}

/// Cooperative cancellation state forwarded unchanged to the API.
#[pyclass(name = "ObservationCancellation", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationCancellation {
    /// Projection may proceed.
    Active,
    /// Caller cancelled before projection.
    Cancelled,
}

impl From<PyObservationCancellation> for ObservationCancellation {
    fn from(value: PyObservationCancellation) -> Self {
        match value {
            PyObservationCancellation::Active => Self::Active,
            PyObservationCancellation::Cancelled => Self::Cancelled,
        }
    }
}

/// Delta outcome kind.
#[pyclass(
    name = "ObservationDeltaOutcomeKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationDeltaOutcomeKind {
    /// Caller is current.
    CaughtUp,
    /// Complete adjacent delta returned.
    Delta,
    /// Full snapshot required.
    Resynchronize,
    /// Cancelled before projection.
    Cancelled,
    /// Complete delta exceeded the declared bound.
    Backpressured,
}

/// Deterministic change kind.
#[pyclass(
    name = "ObservationChangeKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationChangeKind {
    /// Insert or replace one fact.
    Upsert,
    /// Remove one fact key.
    Remove,
}

/// One deterministic fact change.
#[pyclass(name = "ObservationChange", frozen)]
pub struct PyObservationChange {
    inner: ObservationChange,
}

#[pymethods]
impl PyObservationChange {
    /// Return the closed change kind.
    #[getter]
    pub fn kind(&self) -> PyObservationChangeKind {
        match self.inner {
            ObservationChange::Upsert { .. } => PyObservationChangeKind::Upsert,
            ObservationChange::Remove { .. } => PyObservationChangeKind::Remove,
        }
    }

    /// Return the complete successor fact for an upsert.
    #[getter]
    pub fn fact(&self) -> Option<PyObservationFact> {
        match &self.inner {
            ObservationChange::Upsert { fact } => Some(PyObservationFact {
                inner: fact.clone(),
            }),
            ObservationChange::Remove { .. } => None,
        }
    }

    /// Return the removed category for a removal.
    #[getter]
    pub fn category(&self) -> Option<PyObservationCategory> {
        match self.inner {
            ObservationChange::Remove { category, .. } => Some(category_from_api(category)),
            ObservationChange::Upsert { .. } => None,
        }
    }

    /// Return the removed subject for a removal.
    #[getter]
    pub fn subject(&self) -> Option<PyObservationSubject> {
        match &self.inner {
            ObservationChange::Remove { subject, .. } => Some(PyObservationSubject {
                inner: subject.clone(),
            }),
            ObservationChange::Upsert { .. } => None,
        }
    }
}

/// Safe recovery after backpressure.
#[pyclass(
    name = "ObservationBackpressureRecovery",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationBackpressureRecovery {
    /// Request a fresh bounded snapshot.
    RequestSnapshot,
}

/// Typed delta outcome retaining all recovery coordinates.
#[pyclass(name = "ObservationDeltaOutcome", frozen)]
pub struct PyObservationDeltaOutcome {
    inner: ObservationDeltaOutcome,
}

#[pymethods]
impl PyObservationDeltaOutcome {
    /// Return the closed outcome kind.
    #[getter]
    pub fn kind(&self) -> PyObservationDeltaOutcomeKind {
        match self.inner {
            ObservationDeltaOutcome::CaughtUp { .. } => PyObservationDeltaOutcomeKind::CaughtUp,
            ObservationDeltaOutcome::Delta { .. } => PyObservationDeltaOutcomeKind::Delta,
            ObservationDeltaOutcome::Resynchronize { .. } => {
                PyObservationDeltaOutcomeKind::Resynchronize
            }
            ObservationDeltaOutcome::Cancelled { .. } => PyObservationDeltaOutcomeKind::Cancelled,
            ObservationDeltaOutcome::Backpressured { .. } => {
                PyObservationDeltaOutcomeKind::Backpressured
            }
        }
    }

    /// Return the current cursor for caught-up or backpressured outcomes.
    #[getter]
    pub fn cursor(&self) -> Option<PyObservationCursor> {
        match &self.inner {
            ObservationDeltaOutcome::CaughtUp { cursor }
            | ObservationDeltaOutcome::Backpressured {
                current: cursor, ..
            } => Some(PyObservationCursor {
                inner: cursor.clone(),
            }),
            _ => None,
        }
    }

    /// Return the predecessor cursor for a complete delta.
    #[getter]
    pub fn from_cursor(&self) -> Option<PyObservationCursor> {
        match &self.inner {
            ObservationDeltaOutcome::Delta { delta } => Some(PyObservationCursor {
                inner: delta.from.clone(),
            }),
            _ => None,
        }
    }

    /// Return the successor cursor for a complete delta.
    #[getter]
    pub fn to_cursor(&self) -> Option<PyObservationCursor> {
        match &self.inner {
            ObservationDeltaOutcome::Delta { delta } => Some(PyObservationCursor {
                inner: delta.to.clone(),
            }),
            _ => None,
        }
    }

    /// Return the complete ordered change batch for a delta.
    #[getter]
    pub fn changes(&self) -> Vec<PyObservationChange> {
        match &self.inner {
            ObservationDeltaOutcome::Delta { delta } => delta
                .changes
                .iter()
                .cloned()
                .map(|inner| PyObservationChange { inner })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Return the omitted-fact count installed with a complete delta.
    #[getter]
    pub fn successor_omitted_fact_count(&self) -> Option<u32> {
        match &self.inner {
            ObservationDeltaOutcome::Delta { delta } => Some(delta.successor_omitted_fact_count),
            _ => None,
        }
    }

    /// Return the recovery instruction when resynchronization is required.
    #[getter]
    pub fn resynchronization(&self) -> Option<PyObservationResynchronization> {
        match &self.inner {
            ObservationDeltaOutcome::Resynchronize { resynchronization } => {
                Some(PyObservationResynchronization {
                    inner: resynchronization.clone(),
                })
            }
            _ => None,
        }
    }

    /// Return the completely applied cursor retained after cancellation.
    #[getter]
    pub fn cancelled_at(&self) -> Option<PyObservationCursor> {
        match &self.inner {
            ObservationDeltaOutcome::Cancelled { resume } => Some(PyObservationCursor {
                inner: resume.cursor().clone(),
            }),
            _ => None,
        }
    }

    /// Return the required atomic change count after backpressure.
    #[getter]
    pub fn pending_changes(&self) -> Option<u32> {
        match self.inner {
            ObservationDeltaOutcome::Backpressured {
                pending_changes, ..
            } => Some(pending_changes),
            _ => None,
        }
    }

    /// Return the caller-declared change ceiling after backpressure.
    #[getter]
    pub fn maximum_changes(&self) -> Option<u16> {
        match self.inner {
            ObservationDeltaOutcome::Backpressured {
                maximum_changes, ..
            } => Some(maximum_changes),
            _ => None,
        }
    }

    /// Return the safe recovery action after backpressure.
    #[getter]
    pub fn recovery(&self) -> Option<PyObservationBackpressureRecovery> {
        match self.inner {
            ObservationDeltaOutcome::Backpressured {
                recovery: ObservationBackpressureRecovery::RequestSnapshot,
                ..
            } => Some(PyObservationBackpressureRecovery::RequestSnapshot),
            _ => None,
        }
    }
}

/// Causal proof-step kind.
#[pyclass(
    name = "ObservationCausalStepKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationCausalStepKind {
    /// Origin receipt.
    OriginReceipt,
    /// Entry contract.
    EntryContract,
    /// Responsibility contract.
    ResponsibilityContract,
    /// Provider binding.
    ProviderBinding,
    /// Delegation edge.
    DelegationEdge,
    /// Termination contract.
    TerminationContract,
    /// Settlement contract.
    SettlementContract,
    /// Continuity contract.
    ContinuityContract,
}

/// One exact API-owned causal step.
#[pyclass(name = "ObservationCausalStep", frozen)]
pub struct PyObservationCausalStep {
    inner: ObservationCausalStep,
}

#[pymethods]
impl PyObservationCausalStep {
    /// Return the closed proof-step kind.
    #[getter]
    pub fn kind(&self) -> PyObservationCausalStepKind {
        match self.inner {
            ObservationCausalStep::OriginReceipt { .. } => {
                PyObservationCausalStepKind::OriginReceipt
            }
            ObservationCausalStep::EntryContract { .. } => {
                PyObservationCausalStepKind::EntryContract
            }
            ObservationCausalStep::ResponsibilityContract { .. } => {
                PyObservationCausalStepKind::ResponsibilityContract
            }
            ObservationCausalStep::ProviderBinding { .. } => {
                PyObservationCausalStepKind::ProviderBinding
            }
            ObservationCausalStep::DelegationEdge { .. } => {
                PyObservationCausalStepKind::DelegationEdge
            }
            ObservationCausalStep::TerminationContract { .. } => {
                PyObservationCausalStepKind::TerminationContract
            }
            ObservationCausalStep::SettlementContract { .. } => {
                PyObservationCausalStepKind::SettlementContract
            }
            ObservationCausalStep::ContinuityContract { .. } => {
                PyObservationCausalStepKind::ContinuityContract
            }
        }
    }

    /// Return the exact referenced contract, binding, or edge identity.
    #[getter]
    pub fn reference_id(&self) -> String {
        match &self.inner {
            ObservationCausalStep::OriginReceipt { contract_id }
            | ObservationCausalStep::EntryContract { contract_id }
            | ObservationCausalStep::ResponsibilityContract { contract_id }
            | ObservationCausalStep::TerminationContract { contract_id }
            | ObservationCausalStep::SettlementContract { contract_id }
            | ObservationCausalStep::ContinuityContract { contract_id } => contract_id.clone(),
            ObservationCausalStep::ProviderBinding { binding_id } => binding_id.clone(),
            ObservationCausalStep::DelegationEdge { edge_id } => edge_id.clone(),
        }
    }
}

/// One bounded exact causal proof.
#[pyclass(name = "ObservationCausalPath", frozen)]
pub struct PyObservationCausalPath {
    inner: ObservationCausalPath,
}

#[pymethods]
impl PyObservationCausalPath {
    /// Return the generation in which the proof is meaningful.
    #[getter]
    pub fn generation(&self) -> PyObservationGeneration {
        PyObservationGeneration {
            inner: self.inner.generation.clone(),
        }
    }

    /// Return the exact selected subject.
    #[getter]
    pub fn subject(&self) -> PyObservationSubject {
        PyObservationSubject {
            inner: self.inner.subject.clone(),
        }
    }

    /// Return the ordered compiler-owned proof steps.
    #[getter]
    pub fn steps(&self) -> Vec<PyObservationCausalStep> {
        self.inner
            .steps
            .iter()
            .cloned()
            .map(|inner| PyObservationCausalStep { inner })
            .collect()
    }
}

/// Causal query disposition.
#[pyclass(
    name = "ObservationCausalResultKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationCausalResultKind {
    /// Exact proof found.
    Found,
    /// Subject has no retained proof.
    NotFound,
    /// Selector generation is no longer current.
    Resynchronize,
}

/// Typed causal query result.
#[pyclass(name = "ObservationCausalPathResult", frozen)]
pub struct PyObservationCausalPathResult {
    path: Option<ObservationCausalPath>,
    resynchronization: Option<ObservationResynchronization>,
}

#[pymethods]
impl PyObservationCausalPathResult {
    /// Return the closed query disposition.
    #[getter]
    pub fn kind(&self) -> PyObservationCausalResultKind {
        if self.resynchronization.is_some() {
            PyObservationCausalResultKind::Resynchronize
        } else if self.path.is_some() {
            PyObservationCausalResultKind::Found
        } else {
            PyObservationCausalResultKind::NotFound
        }
    }

    /// Return the exact proof when found.
    #[getter]
    pub fn path(&self) -> Option<PyObservationCausalPath> {
        self.path
            .clone()
            .map(|inner| PyObservationCausalPath { inner })
    }

    /// Return the recovery instruction for a stale selector generation.
    #[getter]
    pub fn resynchronization(&self) -> Option<PyObservationResynchronization> {
        self.resynchronization
            .clone()
            .map(|inner| PyObservationResynchronization { inner })
    }
}

/// One lens node and its complete selected facts.
#[pyclass(name = "ObservationLensNode", frozen)]
pub struct PyObservationLensNode {
    inner: ObservationLensNode,
}

/// Opaque continuation for one exact bounded lens query.
///
/// It binds the snapshot, filter, causal-path choice, raw limits, and next
/// subject. Pass it only to the same query. Its position does not prove that
/// later subjects are absent.
#[pyclass(name = "ObservationLensCursor", frozen)]
pub struct PyObservationLensCursor {
    inner: ObservationLensCursor,
}

#[pymethods]
impl PyObservationLensCursor {
    /// Return the exact snapshot cursor.
    #[getter]
    pub fn snapshot(&self) -> PyObservationCursor {
        PyObservationCursor {
            inner: self.inner.snapshot().clone(),
        }
    }

    /// Return how many retained subjects precede the next step.
    #[getter]
    pub fn inspected_position(&self) -> u32 {
        self.inner.inspected_position()
    }
}

#[pymethods]
impl PyObservationLensNode {
    /// Return the exact admitted subject.
    #[getter]
    pub fn subject(&self) -> PyObservationSubject {
        PyObservationSubject {
            inner: self.inner.subject.clone(),
        }
    }

    /// Return the complete typed facts selected for this subject.
    #[getter]
    pub fn facts(&self) -> Vec<PyObservationFact> {
        self.inner
            .facts
            .iter()
            .cloned()
            .map(|inner| PyObservationFact { inner })
            .collect()
    }
}

/// Bounded generation-bound visualization substrate.
#[pyclass(name = "ObservationLens", frozen)]
pub struct PyObservationLens {
    inner: ObservationLens,
}

#[pymethods]
impl PyObservationLens {
    /// Return the exact source snapshot coordinate.
    #[getter]
    pub fn cursor(&self) -> PyObservationCursor {
        PyObservationCursor {
            inner: self.inner.cursor.clone(),
        }
    }

    /// Return deterministically ordered subject nodes.
    #[getter]
    pub fn nodes(&self) -> Vec<PyObservationLensNode> {
        self.inner
            .nodes
            .iter()
            .cloned()
            .map(|inner| PyObservationLensNode { inner })
            .collect()
    }

    /// Return retained causal paths when requested.
    #[getter]
    pub fn causal_paths(&self) -> Vec<PyObservationCausalPath> {
        self.inner
            .causal_paths
            .iter()
            .cloned()
            .map(|inner| PyObservationCausalPath { inner })
            .collect()
    }

    /// Return the continuation when selected subjects may remain.
    #[getter]
    pub fn continuation(&self) -> Option<PyObservationLensCursor> {
        self.inner
            .continuation
            .clone()
            .map(|inner| PyObservationLensCursor { inner })
    }

    /// Return whether this step exhausted its selected input.
    #[getter]
    pub fn completion(&self) -> PyObservationStepCompletionKind {
        completion_kind(self.inner.completion)
    }

    /// Return the resource that stopped this step, if any.
    #[getter]
    pub fn stopped_by(&self) -> Option<PyObservationStepLimitKind> {
        completion_limit(self.inner.completion).map(limit_kind)
    }

    /// Return the minimum bound needed by an indivisible result, if applicable.
    #[getter]
    pub fn minimum_required(&self) -> Option<u32> {
        completion_limit(self.inner.completion).and_then(minimum_required)
    }

    /// Return retained subjects examined by this step.
    #[getter]
    pub fn inspected_subjects(&self) -> u16 {
        self.inner.inspected_subjects
    }

    /// Return variable-length UTF-8 bytes copied by this step.
    #[getter]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes
    }

    /// Return causal-path lookups performed by this step.
    ///
    /// A stopped step can count a lookup for an unreturned subject.
    #[getter]
    pub fn causal_path_lookups(&self) -> u16 {
        self.inner.causal_path_lookups
    }

    /// Return causal steps copied by this step.
    #[getter]
    pub fn causal_steps(&self) -> u16 {
        self.inner.causal_steps
    }

    /// Return owner facts omitted when the bounded snapshot was constructed.
    #[getter]
    pub fn snapshot_omitted_count(&self) -> u32 {
        self.inner.snapshot_omitted_count
    }
}

/// Cooperative visual-lens disposition.
#[pyclass(
    name = "ObservationLensDispositionKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationLensDispositionKind {
    /// A valid lens step returned; inspect its completion before assuming exhaustion.
    Ready,
    /// The supplied cursor cannot continue; obtain a fresh snapshot.
    Resynchronize,
    /// Cancelled before this step began; the supplied cursor is unchanged.
    Cancelled,
}

/// Typed visual-lens result.
#[pyclass(name = "ObservationLensDisposition", frozen)]
pub struct PyObservationLensDisposition {
    inner: ObservationLensDisposition,
}

#[pymethods]
impl PyObservationLensDisposition {
    /// Return the closed disposition kind.
    #[getter]
    pub fn kind(&self) -> PyObservationLensDispositionKind {
        match self.inner {
            ObservationLensDisposition::Ready { .. } => PyObservationLensDispositionKind::Ready,
            ObservationLensDisposition::Resynchronize { .. } => {
                PyObservationLensDispositionKind::Resynchronize
            }
            ObservationLensDisposition::Cancelled { .. } => {
                PyObservationLensDispositionKind::Cancelled
            }
        }
    }

    /// Return the bounded lens step when ready.
    #[getter]
    pub fn lens(&self) -> Option<PyObservationLens> {
        match &self.inner {
            ObservationLensDisposition::Ready { lens } => Some(PyObservationLens {
                inner: lens.clone(),
            }),
            ObservationLensDisposition::Resynchronize { .. }
            | ObservationLensDisposition::Cancelled { .. } => None,
        }
    }

    /// Return the recovery instruction for a mismatched continuation.
    #[getter]
    pub fn resynchronization(&self) -> Option<PyObservationResynchronization> {
        match &self.inner {
            ObservationLensDisposition::Resynchronize { resynchronization } => {
                Some(PyObservationResynchronization {
                    inner: resynchronization.clone(),
                })
            }
            ObservationLensDisposition::Ready { .. }
            | ObservationLensDisposition::Cancelled { .. } => None,
        }
    }

    /// Return the unchanged continuation after cancellation, if any.
    #[getter]
    pub fn cancelled_continuation(&self) -> Option<PyObservationLensCursor> {
        match &self.inner {
            ObservationLensDisposition::Cancelled { continuation } => continuation
                .clone()
                .map(|inner| PyObservationLensCursor { inner }),
            ObservationLensDisposition::Ready { .. }
            | ObservationLensDisposition::Resynchronize { .. } => None,
        }
    }
}

/// Opaque source-free observation snapshot.
#[pyclass(name = "ObservationSnapshot", frozen)]
pub struct PyObservationSnapshot {
    inner: Arc<ObservationSnapshot>,
}

#[pymethods]
impl PyObservationSnapshot {
    /// Return the exact complete-snapshot cursor.
    #[getter]
    pub fn cursor(&self) -> PyObservationCursor {
        PyObservationCursor {
            inner: self.inner.cursor(),
        }
    }

    /// Return the retained fact count.
    #[getter]
    pub fn fact_count(&self) -> u32 {
        self.inner.summary().fact_count
    }

    /// Return the count omitted by snapshot construction.
    #[getter]
    pub fn omitted_fact_count(&self) -> u32 {
        self.inner.summary().omitted_fact_count
    }

    /// Query one bounded page. A ready result can still stop before exhaustion.
    pub fn page(
        &self,
        py: Python<'_>,
        filter: &PyObservationFilter,
        limits: &PyObservationPageLimits,
        cancellation: PyObservationCancellation,
        after: Option<&PyObservationPageCursor>,
    ) -> PyObservationPageResult {
        let snapshot = Arc::clone(&self.inner);
        let filter = filter.inner.clone();
        let maximum_inspected = limits.maximum_inspected;
        let maximum_returned = limits.maximum_returned;
        let maximum_copied_string_bytes = limits.maximum_copied_string_bytes;
        let after = after.map(|cursor| cursor.inner.clone());
        py.detach(move || {
            let mut request = ObservationPageRequest::bounded(
                filter,
                ApiObservationPageLimits::new(
                    maximum_inspected,
                    maximum_returned,
                    maximum_copied_string_bytes,
                ),
                cancellation.into(),
            );
            if let Some(after) = after {
                request = request.after(after);
            }
            match snapshot.page(request) {
                Ok(ObservationPageDisposition::Ready { page }) => PyObservationPageResult {
                    page: Some(page),
                    cancelled_continuation: None,
                    resynchronization: None,
                },
                Ok(ObservationPageDisposition::Cancelled { continuation }) => {
                    PyObservationPageResult {
                        page: None,
                        cancelled_continuation: continuation,
                        resynchronization: None,
                    }
                }
                Err(resynchronization) => PyObservationPageResult {
                    page: None,
                    cancelled_continuation: None,
                    resynchronization: Some(resynchronization),
                },
            }
        })
    }

    /// Derive one adjacent atomic delta from an earlier API snapshot.
    pub fn delta_from(
        &self,
        py: Python<'_>,
        prior: &PyObservationSnapshot,
        maximum_changes: u16,
        cancellation: PyObservationCancellation,
    ) -> PyObservationDeltaOutcome {
        let snapshot = Arc::clone(&self.inner);
        let prior = Arc::clone(&prior.inner);
        py.detach(move || {
            let request = ObservationDeltaRequest::new(
                prior.resume_coordinate(),
                maximum_changes,
                cancellation.into(),
            );
            PyObservationDeltaOutcome {
                inner: snapshot.delta_from(&prior, request),
            }
        })
    }

    /// Return one retained exact causal path for an API-issued subject.
    pub fn causal_path(
        &self,
        py: Python<'_>,
        subject: &PyObservationSubject,
    ) -> PyObservationCausalPathResult {
        let snapshot = Arc::clone(&self.inner);
        let subject = subject.inner.clone();
        py.detach(move || {
            let selector = ObservationCausalSelector::new(&snapshot, &subject);
            match snapshot.causal_path(&selector) {
                Ok(path) => PyObservationCausalPathResult {
                    path,
                    resynchronization: None,
                },
                Err(resynchronization) => PyObservationCausalPathResult {
                    path: None,
                    resynchronization: Some(resynchronization),
                },
            }
        })
    }

    /// Build one bounded non-authoritative lens step.
    ///
    /// The result carries snapshot-bound facts and optional retained causal
    /// paths, not layout or proof that omitted subjects are absent.
    pub fn visual_lens(
        &self,
        py: Python<'_>,
        filter: &PyObservationFilter,
        limits: &PyObservationLensLimits,
        include_causal_paths: bool,
        cancellation: PyObservationCancellation,
        after: Option<&PyObservationLensCursor>,
    ) -> PyObservationLensDisposition {
        let snapshot = Arc::clone(&self.inner);
        let filter = filter.inner.clone();
        let maximum_inspected = limits.maximum_inspected;
        let maximum_nodes = limits.maximum_nodes;
        let maximum_copied_string_bytes = limits.maximum_copied_string_bytes;
        let maximum_causal_path_lookups = limits.maximum_causal_path_lookups;
        let maximum_causal_steps = limits.maximum_causal_steps;
        let after = after.map(|cursor| cursor.inner.clone());
        py.detach(move || {
            let mut request = ObservationLensRequest::bounded(
                filter,
                ApiObservationLensLimits::new(
                    maximum_inspected,
                    maximum_nodes,
                    maximum_copied_string_bytes,
                    maximum_causal_path_lookups,
                    maximum_causal_steps,
                ),
                include_causal_paths,
                cancellation.into(),
            );
            if let Some(after) = after {
                request = request.after(after);
            }
            PyObservationLensDisposition {
                inner: snapshot.visual_lens(request),
            }
        })
    }
}

#[pymethods]
impl PalePython {
    /// Project an admitted effective profile into a source-free snapshot.
    pub fn observe_effective_profile(
        &self,
        py: Python<'_>,
        compilation: &PyEffectiveProfileCompilation,
    ) -> PyObservationSnapshot {
        let api = Arc::clone(&self.api);
        let compilation = Arc::clone(&compilation.inner);
        py.detach(move || PyObservationSnapshot {
            inner: Arc::new(api.observe_effective_profile(&compilation)),
        })
    }
}
