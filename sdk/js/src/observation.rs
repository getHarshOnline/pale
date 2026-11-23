//! Node projection of API-owned bounded observation.
//!
//! Every class in this module retains or translates an owned [`pale_sdk`]
//! value. Paging, deltas, causal proof, cancellation, backpressure, and lens
//! construction remain API-owned; this leaf neither traverses a graph nor
//! derives readiness, freshness, or effect certainty.

mod profile;
pub use profile::{
    ObservationProfileAssociation, ObservationProfileAssociationResult,
    ObservationProfileAssociationResultKind, ObservationProfileContinuation,
    ObservationProfileError, ObservationProfileMapping, ObservationProfilePage,
    ObservationProfileResult, ObservationProfileResultKind,
};

use napi::bindgen_prelude::BigInt;
use napi_derive::napi;
use pale_sdk::{
    DiagnosticEffectCertainty as ApiDiagnosticEffectCertainty,
    ObservationBackpressureRecovery as ApiObservationBackpressureRecovery,
    ObservationCancellation as ApiObservationCancellation,
    ObservationCategory as ApiObservationCategory,
    ObservationCausalPath as ApiObservationCausalPath, ObservationCausalSelector,
    ObservationCausalStep as ApiObservationCausalStep, ObservationChange as ApiObservationChange,
    ObservationCursor as ApiObservationCursor,
    ObservationDeltaOutcome as ApiObservationDeltaOutcome, ObservationDeltaRequest,
    ObservationFact as ApiObservationFact, ObservationFilter as ApiObservationFilter,
    ObservationFreshness as ApiObservationFreshness,
    ObservationFreshnessClass as ApiObservationFreshnessClass,
    ObservationGeneration as ApiObservationGeneration,
    ObservationHealthState as ApiObservationHealthState, ObservationLens as ApiObservationLens,
    ObservationLensCursor as ApiObservationLensCursor,
    ObservationLensDisposition as ApiObservationLensDisposition,
    ObservationLensLimits as ApiObservationLensLimits,
    ObservationLensNode as ApiObservationLensNode, ObservationLensRequest,
    ObservationNativeState as ApiObservationNativeState, ObservationPage as ApiObservationPage,
    ObservationPageCursor as ApiObservationPageCursor,
    ObservationPageDisposition as ApiObservationPageDisposition,
    ObservationPageLimits as ApiObservationPageLimits, ObservationPageRequest,
    ObservationProcessIntent as ApiObservationProcessIntent,
    ObservationReadinessState as ApiObservationReadinessState,
    ObservationReconciliationState as ApiObservationReconciliationState,
    ObservationResynchronization as ApiObservationResynchronization,
    ObservationResynchronizationReason as ApiObservationResynchronizationReason,
    ObservationRevision as ApiObservationRevision, ObservationSnapshot as ApiObservationSnapshot,
    ObservationStepCompletion as ApiObservationStepCompletion,
    ObservationStepLimit as ApiObservationStepLimit, ObservationSubject as ApiObservationSubject,
    ObservationSubjectKind as ApiObservationSubjectKind,
};

use crate::{
    DiagnosticEffectCertainty, EffectiveProfileCompilation, OperationExposure, Pale,
    RequirementDisposition,
};

/// Admitted generation kind carried by an observation cursor.
#[napi(js_name = "ObservationGenerationKind")]
pub enum ObservationGenerationKind {
    /// Immutable effective-profile generation.
    EffectiveProfile,
    /// Resident authority generation.
    Resident,
}

/// Work and copy limits for one observation-lens step.
///
/// A returned cursor binds these limits to the exact snapshot and filter.
/// Reaching a limit does not prove that later subjects are absent.
#[napi(object, js_name = "ObservationLensLimits")]
pub struct ObservationLensLimits {
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

/// Work and output limits for one observation-page step.
///
/// Zero selects the application's default. The application caps larger values
/// at its owned ceilings.
/// A returned cursor binds these values to the exact snapshot and filter.
/// Reaching a limit does not prove that later facts are absent.
#[napi(object, js_name = "ObservationPageLimits")]
pub struct ObservationPageLimits {
    /// Maximum retained facts this step can inspect.
    pub maximum_inspected: u16,
    /// Maximum matching facts this step can copy.
    pub maximum_returned: u16,
    /// Maximum UTF-8 string bytes this step can copy.
    pub maximum_copied_string_bytes: u32,
}

/// Owned generation coordinate with variant-specific accessors.
#[napi(js_name = "ObservationGeneration")]
pub struct ObservationGeneration {
    inner: ApiObservationGeneration,
}

#[napi]
impl ObservationGeneration {
    /// Return the closed generation kind.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationGenerationKind {
        match self.inner {
            ApiObservationGeneration::EffectiveProfile { .. } => {
                ObservationGenerationKind::EffectiveProfile
            }
            ApiObservationGeneration::Resident { .. } => ObservationGenerationKind::Resident,
        }
    }

    /// Return the profile digest for an effective-profile generation.
    #[napi(getter)]
    pub fn profile_digest(&self) -> Option<String> {
        match &self.inner {
            ApiObservationGeneration::EffectiveProfile { profile_digest, .. } => {
                Some(profile_digest.clone())
            }
            ApiObservationGeneration::Resident { .. } => None,
        }
    }

    /// Return the machine-resolved instance digest for an effective profile.
    #[napi(getter)]
    pub fn instance_digest(&self) -> Option<String> {
        match &self.inner {
            ApiObservationGeneration::EffectiveProfile {
                instance_digest, ..
            } => Some(instance_digest.clone()),
            ApiObservationGeneration::Resident { .. } => None,
        }
    }

    /// Return the composition proof digest that binds an effective profile's causal paths.
    #[napi(getter)]
    pub fn composition_plan_digest(&self) -> Option<String> {
        match &self.inner {
            ApiObservationGeneration::EffectiveProfile {
                composition_plan_digest,
                ..
            } => Some(composition_plan_digest.clone()),
            ApiObservationGeneration::Resident { .. } => None,
        }
    }

    /// Return the stable authority-store identity for a resident generation.
    #[napi(getter)]
    pub fn control_store_id(&self) -> Option<String> {
        match &self.inner {
            ApiObservationGeneration::Resident {
                control_store_id, ..
            } => Some(control_store_id.clone()),
            ApiObservationGeneration::EffectiveProfile { .. } => None,
        }
    }

    /// Return the authority-store incarnation for a resident generation.
    #[napi(getter)]
    pub fn control_incarnation_id(&self) -> Option<String> {
        match &self.inner {
            ApiObservationGeneration::Resident {
                control_incarnation_id,
                ..
            } => Some(control_incarnation_id.clone()),
            ApiObservationGeneration::EffectiveProfile { .. } => None,
        }
    }

    /// Return the active digest for a resident generation, when selected.
    #[napi(getter)]
    pub fn active_generation_digest(&self) -> Option<String> {
        match &self.inner {
            ApiObservationGeneration::Resident {
                active_generation_digest,
                ..
            } => active_generation_digest.clone(),
            ApiObservationGeneration::EffectiveProfile { .. } => None,
        }
    }
}

/// Revision kind within one admitted generation.
#[napi(js_name = "ObservationRevisionKind")]
pub enum ObservationRevisionKind {
    /// Immutable compiler phase.
    Static,
    /// Durable resident-journal head.
    ResidentJournal,
}

/// Owned revision coordinate.
#[napi(js_name = "ObservationRevision")]
pub struct ObservationRevision {
    inner: ApiObservationRevision,
}

#[napi]
impl ObservationRevision {
    /// Return the closed revision kind.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationRevisionKind {
        match self.inner {
            ApiObservationRevision::Static => ObservationRevisionKind::Static,
            ApiObservationRevision::ResidentJournal { .. } => {
                ObservationRevisionKind::ResidentJournal
            }
        }
    }

    /// Return the durable resident sequence when it exists.
    #[napi(getter)]
    pub fn sequence(&self) -> Option<BigInt> {
        match self.inner {
            ApiObservationRevision::ResidentJournal { sequence } => sequence.map(BigInt::from),
            ApiObservationRevision::Static => None,
        }
    }
}

/// Opaque generation/revision cursor issued by the API.
#[napi(js_name = "ObservationCursor")]
pub struct ObservationCursor {
    inner: ApiObservationCursor,
}

#[napi]
impl ObservationCursor {
    /// Return the admitted generation coordinate.
    #[napi(getter)]
    pub fn generation(&self) -> ObservationGeneration {
        ObservationGeneration {
            inner: self.inner.generation().clone(),
        }
    }

    /// Return the revision within this cursor's `generation`.
    #[napi(getter)]
    pub fn revision(&self) -> ObservationRevision {
        ObservationRevision {
            inner: self.inner.revision().clone(),
        }
    }
}

/// Independent observation axis.
#[napi(js_name = "ObservationCategory")]
pub enum ObservationCategory {
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

impl From<ObservationCategory> for ApiObservationCategory {
    fn from(value: ObservationCategory) -> Self {
        match value {
            ObservationCategory::OperationExposure => Self::OperationExposure,
            ObservationCategory::CapabilityRequirementDisposition => {
                Self::CapabilityRequirementDisposition
            }
            ObservationCategory::Health => Self::Health,
            ObservationCategory::Readiness => Self::Readiness,
            ObservationCategory::Effect => Self::Effect,
            ObservationCategory::NativeOwner => Self::NativeOwner,
            ObservationCategory::Reconciliation => Self::Reconciliation,
            ObservationCategory::ProcessIntent => Self::ProcessIntent,
        }
    }
}

fn category_from_api(value: ApiObservationCategory) -> ObservationCategory {
    match value {
        ApiObservationCategory::OperationExposure => ObservationCategory::OperationExposure,
        ApiObservationCategory::CapabilityRequirementDisposition => {
            ObservationCategory::CapabilityRequirementDisposition
        }
        ApiObservationCategory::Health => ObservationCategory::Health,
        ApiObservationCategory::Readiness => ObservationCategory::Readiness,
        ApiObservationCategory::Effect => ObservationCategory::Effect,
        ApiObservationCategory::NativeOwner => ObservationCategory::NativeOwner,
        ApiObservationCategory::Reconciliation => ObservationCategory::Reconciliation,
        ApiObservationCategory::ProcessIntent => ObservationCategory::ProcessIntent,
    }
}

/// Exact API-issued observation subject.
#[napi(js_name = "ObservationSubject")]
pub struct ObservationSubject {
    inner: ApiObservationSubject,
}

#[napi]
impl ObservationSubject {
    /// Return the closed subject kind.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationSubjectKind {
        self.inner.kind().into()
    }

    /// Return the profile digest when applicable.
    #[napi(getter)]
    pub fn profile_digest(&self) -> Option<String> {
        self.inner.profile_digest().map(str::to_owned)
    }

    /// Return the machine identity when applicable.
    #[napi(getter)]
    pub fn machine_id(&self) -> Option<String> {
        self.inner.machine_id().map(str::to_owned)
    }

    /// Return the repository identity when applicable.
    #[napi(getter)]
    pub fn repository_id(&self) -> Option<String> {
        self.inner.repository_id().map(str::to_owned)
    }

    /// Return the operation identity when applicable.
    #[napi(getter)]
    pub fn operation_id(&self) -> Option<String> {
        self.inner.operation_id().map(str::to_owned)
    }

    /// Return the requirement identity when applicable.
    #[napi(getter)]
    pub fn requirement_id(&self) -> Option<String> {
        self.inner.requirement_id().map(str::to_owned)
    }

    /// Return the capability identity when applicable.
    #[napi(getter)]
    pub fn capability_id(&self) -> Option<String> {
        self.inner.capability_id().map(str::to_owned)
    }

    /// Return the execution-path identity when applicable.
    #[napi(getter)]
    pub fn execution_path_id(&self) -> Option<String> {
        self.inner.execution_path_id().map(str::to_owned)
    }

    /// Return the compiled-generation digest when applicable.
    #[napi(getter)]
    pub fn compiled_generation_digest(&self) -> Option<String> {
        self.inner.compiled_generation_digest().map(str::to_owned)
    }
}

/// Closed kind of an API-issued subject.
#[napi(js_name = "ObservationSubjectKind")]
pub enum ObservationSubjectKind {
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

impl From<ApiObservationSubjectKind> for ObservationSubjectKind {
    fn from(value: ApiObservationSubjectKind) -> Self {
        match value {
            ApiObservationSubjectKind::EffectiveProfile => Self::EffectiveProfile,
            ApiObservationSubjectKind::Machine => Self::Machine,
            ApiObservationSubjectKind::Repository => Self::Repository,
            ApiObservationSubjectKind::Operation => Self::Operation,
            ApiObservationSubjectKind::CapabilityRequirement => Self::CapabilityRequirement,
            ApiObservationSubjectKind::ExecutionPath => Self::ExecutionPath,
            ApiObservationSubjectKind::CompiledGeneration => Self::CompiledGeneration,
            ApiObservationSubjectKind::ResidentProcess => Self::ResidentProcess,
        }
    }
}

/// Freshness kind for one observation fact.
#[napi(js_name = "ObservationFreshnessKind")]
pub enum ObservationFreshnessKind {
    /// Current for the snapshot.
    Fresh,
    /// Retained from an older revision.
    Stale,
    /// Currency is not established.
    Unknown,
}

/// Temporal qualification of one fact.
#[napi(js_name = "ObservationFreshness")]
pub struct ObservationFreshness {
    inner: ApiObservationFreshness,
}

#[napi]
impl ObservationFreshness {
    /// Return the closed freshness kind.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationFreshnessKind {
        match self.inner {
            ApiObservationFreshness::Fresh => ObservationFreshnessKind::Fresh,
            ApiObservationFreshness::Stale { .. } => ObservationFreshnessKind::Stale,
            ApiObservationFreshness::Unknown => ObservationFreshnessKind::Unknown,
        }
    }

    /// Return the exact stale distance when applicable.
    #[napi(getter)]
    pub fn revisions_behind(&self) -> Option<BigInt> {
        match self.inner {
            ApiObservationFreshness::Stale { revisions_behind } => {
                Some(BigInt::from(revisions_behind))
            }
            ApiObservationFreshness::Fresh | ApiObservationFreshness::Unknown => None,
        }
    }
}

/// Freshness filter accepted by bounded observation requests.
#[napi(js_name = "ObservationFreshnessClass")]
pub enum ObservationFreshnessClass {
    /// Current facts only.
    Fresh,
    /// Stale facts only.
    Stale,
    /// Unknown-currency facts only.
    Unknown,
}

impl From<ObservationFreshnessClass> for ApiObservationFreshnessClass {
    fn from(value: ObservationFreshnessClass) -> Self {
        match value {
            ObservationFreshnessClass::Fresh => Self::Fresh,
            ObservationFreshnessClass::Stale => Self::Stale,
            ObservationFreshnessClass::Unknown => Self::Unknown,
        }
    }
}

/// Health state independent of readiness.
#[napi(js_name = "ObservationHealthState")]
pub enum ObservationHealthState {
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
#[napi(js_name = "ObservationReadinessState")]
pub enum ObservationReadinessState {
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
#[napi(js_name = "ObservationNativeStateKind")]
pub enum ObservationNativeStateKind {
    /// No carrier exists.
    Unavailable,
    /// Owner selected without qualifying evidence.
    Unknown,
    /// Exact owner evidence admitted.
    Observed,
}

/// Native-owner evidence without copied native payload.
#[napi(js_name = "ObservationNativeState")]
pub struct ObservationNativeState {
    inner: ApiObservationNativeState,
}

#[napi]
impl ObservationNativeState {
    /// Return the closed native-evidence kind.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationNativeStateKind {
        match self.inner {
            ApiObservationNativeState::Unavailable => ObservationNativeStateKind::Unavailable,
            ApiObservationNativeState::Unknown => ObservationNativeStateKind::Unknown,
            ApiObservationNativeState::Observed { .. } => ObservationNativeStateKind::Observed,
        }
    }

    /// Return the native system retaining source truth when observed.
    #[napi(getter)]
    pub fn owner_system_id(&self) -> Option<String> {
        match &self.inner {
            ApiObservationNativeState::Observed {
                owner_system_id, ..
            } => Some(owner_system_id.clone()),
            _ => None,
        }
    }

    /// Return the owner-defined observation kind when observed.
    #[napi(getter)]
    pub fn observation_kind(&self) -> Option<String> {
        match &self.inner {
            ApiObservationNativeState::Observed {
                observation_kind, ..
            } => Some(observation_kind.clone()),
            _ => None,
        }
    }
}

/// Reconciliation-state kind independent of effect certainty.
#[napi(js_name = "ObservationReconciliationStateKind")]
pub enum ObservationReconciliationStateKind {
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
#[napi(js_name = "ObservationReconciliationState")]
pub struct ObservationReconciliationState {
    inner: ApiObservationReconciliationState,
}

#[napi]
impl ObservationReconciliationState {
    /// Return the closed reconciliation kind.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationReconciliationStateKind {
        match self.inner {
            ApiObservationReconciliationState::NotRequired => {
                ObservationReconciliationStateKind::NotRequired
            }
            ApiObservationReconciliationState::Required { .. } => {
                ObservationReconciliationStateKind::Required
            }
            ApiObservationReconciliationState::InProgress { .. } => {
                ObservationReconciliationStateKind::InProgress
            }
            ApiObservationReconciliationState::Settled { .. } => {
                ObservationReconciliationStateKind::Settled
            }
            ApiObservationReconciliationState::Unknown => {
                ObservationReconciliationStateKind::Unknown
            }
            ApiObservationReconciliationState::Unavailable => {
                ObservationReconciliationStateKind::Unavailable
            }
        }
    }

    /// Return the active reconciliation case digest, when any.
    #[napi(getter)]
    pub fn case_digest(&self) -> Option<String> {
        match &self.inner {
            ApiObservationReconciliationState::Required { case_digest }
            | ApiObservationReconciliationState::InProgress { case_digest } => {
                Some(case_digest.clone())
            }
            _ => None,
        }
    }

    /// Return the terminal result digest when settled.
    #[napi(getter)]
    pub fn result_digest(&self) -> Option<String> {
        match &self.inner {
            ApiObservationReconciliationState::Settled { result_digest } => {
                Some(result_digest.clone())
            }
            _ => None,
        }
    }
}

/// Desired resident process state, not observed liveness.
#[napi(js_name = "ObservationProcessIntent")]
pub enum ObservationProcessIntent {
    /// Desired running.
    Running,
    /// Desired stopped.
    Stopped,
}

/// One independently typed observation fact.
#[napi(js_name = "ObservationFact")]
pub struct ObservationFact {
    inner: ApiObservationFact,
}

#[napi]
impl ObservationFact {
    /// Return the independent fact axis.
    #[napi(getter)]
    pub fn category(&self) -> ObservationCategory {
        category_from_api(match self.inner {
            ApiObservationFact::OperationExposure { .. } => {
                ApiObservationCategory::OperationExposure
            }
            ApiObservationFact::CapabilityRequirementDisposition { .. } => {
                ApiObservationCategory::CapabilityRequirementDisposition
            }
            ApiObservationFact::Health { .. } => ApiObservationCategory::Health,
            ApiObservationFact::Readiness { .. } => ApiObservationCategory::Readiness,
            ApiObservationFact::Effect { .. } => ApiObservationCategory::Effect,
            ApiObservationFact::NativeOwner { .. } => ApiObservationCategory::NativeOwner,
            ApiObservationFact::Reconciliation { .. } => ApiObservationCategory::Reconciliation,
            ApiObservationFact::ProcessIntent { .. } => ApiObservationCategory::ProcessIntent,
        })
    }

    /// Return the exact subject when this axis is subject-scoped.
    #[napi(getter)]
    pub fn subject(&self) -> Option<ObservationSubject> {
        match &self.inner {
            ApiObservationFact::OperationExposure { subject, .. }
            | ApiObservationFact::CapabilityRequirementDisposition { subject, .. }
            | ApiObservationFact::Health { subject, .. }
            | ApiObservationFact::Readiness { subject, .. }
            | ApiObservationFact::Effect { subject, .. }
            | ApiObservationFact::NativeOwner { subject, .. }
            | ApiObservationFact::Reconciliation { subject, .. } => Some(ObservationSubject {
                inner: subject.clone(),
            }),
            ApiObservationFact::ProcessIntent { .. } => None,
        }
    }

    /// Return the fact's temporal qualification.
    #[napi(getter)]
    pub fn freshness(&self) -> ObservationFreshness {
        let inner = match self.inner {
            ApiObservationFact::OperationExposure { freshness, .. }
            | ApiObservationFact::CapabilityRequirementDisposition { freshness, .. }
            | ApiObservationFact::Health { freshness, .. }
            | ApiObservationFact::Readiness { freshness, .. }
            | ApiObservationFact::Effect { freshness, .. }
            | ApiObservationFact::NativeOwner { freshness, .. }
            | ApiObservationFact::Reconciliation { freshness, .. }
            | ApiObservationFact::ProcessIntent { freshness, .. } => freshness,
        };
        ObservationFreshness { inner }
    }

    /// Return operation exposure only on that axis.
    #[napi(getter)]
    pub fn operation_exposure(&self) -> Option<OperationExposure> {
        match self.inner {
            ApiObservationFact::OperationExposure { state, .. } => {
                Some(OperationExposure::from_api(state))
            }
            _ => None,
        }
    }

    /// Return capability disposition only on that independent axis.
    #[napi(getter)]
    pub fn requirement_disposition(&self) -> Option<RequirementDisposition> {
        match self.inner {
            ApiObservationFact::CapabilityRequirementDisposition { state, .. } => {
                Some(RequirementDisposition::from_api(state))
            }
            _ => None,
        }
    }

    /// Return health state only on the health axis.
    #[napi(getter)]
    pub fn health_state(&self) -> Option<ObservationHealthState> {
        match self.inner {
            ApiObservationFact::Health { state, .. } => Some(match state {
                ApiObservationHealthState::Healthy => ObservationHealthState::Healthy,
                ApiObservationHealthState::Degraded => ObservationHealthState::Degraded,
                ApiObservationHealthState::Unhealthy => ObservationHealthState::Unhealthy,
                ApiObservationHealthState::Unknown => ObservationHealthState::Unknown,
                ApiObservationHealthState::Unavailable => ObservationHealthState::Unavailable,
            }),
            _ => None,
        }
    }

    /// Return readiness only on the readiness axis.
    #[napi(getter)]
    pub fn readiness_state(&self) -> Option<ObservationReadinessState> {
        match self.inner {
            ApiObservationFact::Readiness { state, .. } => Some(match state {
                ApiObservationReadinessState::Ready => ObservationReadinessState::Ready,
                ApiObservationReadinessState::Blocked => ObservationReadinessState::Blocked,
                ApiObservationReadinessState::Narrowed => ObservationReadinessState::Narrowed,
                ApiObservationReadinessState::Unproven => ObservationReadinessState::Unproven,
                ApiObservationReadinessState::Unavailable => ObservationReadinessState::Unavailable,
            }),
            _ => None,
        }
    }

    /// Return effect certainty only on the effect axis.
    #[napi(getter)]
    pub fn effect_certainty(&self) -> Option<DiagnosticEffectCertainty> {
        match self.inner {
            ApiObservationFact::Effect { certainty, .. } => Some(match certainty {
                ApiDiagnosticEffectCertainty::NotApplicable => {
                    DiagnosticEffectCertainty::NotApplicable
                }
                ApiDiagnosticEffectCertainty::NotStarted => DiagnosticEffectCertainty::NotStarted,
                ApiDiagnosticEffectCertainty::Confirmed => DiagnosticEffectCertainty::Confirmed,
                ApiDiagnosticEffectCertainty::OutcomeUnknown => {
                    DiagnosticEffectCertainty::OutcomeUnknown
                }
            }),
            _ => None,
        }
    }

    /// Return native evidence only on the native-owner axis.
    #[napi(getter)]
    pub fn native_state(&self) -> Option<ObservationNativeState> {
        match &self.inner {
            ApiObservationFact::NativeOwner { state, .. } => Some(ObservationNativeState {
                inner: state.clone(),
            }),
            _ => None,
        }
    }

    /// Return reconciliation state only on that axis.
    #[napi(getter)]
    pub fn reconciliation_state(&self) -> Option<ObservationReconciliationState> {
        match &self.inner {
            ApiObservationFact::Reconciliation { state, .. } => {
                Some(ObservationReconciliationState {
                    inner: state.clone(),
                })
            }
            _ => None,
        }
    }

    /// Return process intent only on that axis.
    #[napi(getter)]
    pub fn process_intent(&self) -> Option<ObservationProcessIntent> {
        match self.inner {
            ApiObservationFact::ProcessIntent { state, .. } => Some(match state {
                ApiObservationProcessIntent::Running => ObservationProcessIntent::Running,
                ApiObservationProcessIntent::Stopped => ObservationProcessIntent::Stopped,
            }),
            _ => None,
        }
    }
}

/// Opaque API-owned filter; setters only compose the canonical filter algebra.
#[napi(js_name = "ObservationFilter")]
pub struct ObservationFilter {
    inner: ApiObservationFilter,
}

#[napi]
impl ObservationFilter {
    /// Construct an unrestricted filter.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: ApiObservationFilter::all(),
        }
    }

    /// Restrict this filter to one independent category.
    #[napi]
    pub fn select_category(&mut self, category: ObservationCategory) {
        self.inner = self.inner.clone().with_category(category.into());
    }

    /// Restrict this filter to one API-issued subject.
    #[napi]
    pub fn select_subject(&mut self, subject: &ObservationSubject) {
        self.inner = self.inner.clone().with_subject(&subject.inner);
    }

    /// Restrict this filter to one freshness class.
    #[napi]
    pub fn select_freshness(&mut self, freshness: ObservationFreshnessClass) {
        self.inner = self.inner.clone().with_freshness(freshness.into());
    }
}

impl Default for ObservationFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Opaque continuation for one exact filtered snapshot page.
///
/// It binds the snapshot, filter, raw limits, and next fact. Pass it only to
/// the same query. Its position does not prove that later facts are absent.
#[napi(js_name = "ObservationPageCursor")]
pub struct ObservationPageCursor {
    inner: ApiObservationPageCursor,
}

#[napi]
impl ObservationPageCursor {
    /// Return the exact snapshot cursor.
    #[napi(getter)]
    pub fn snapshot(&self) -> ObservationCursor {
        ObservationCursor {
            inner: self.inner.snapshot().clone(),
        }
    }

    /// Return how many retained facts precede the next step.
    #[napi(getter)]
    pub fn inspected_position(&self) -> u32 {
        self.inner.inspected_position()
    }
}

/// Completion state of one bounded observation step.
#[napi(js_name = "ObservationStepCompletionKind")]
pub enum ObservationStepCompletionKind {
    /// The selected input for this snapshot and filter was exhausted.
    Complete,
    /// More input can remain; continue with the returned cursor.
    Stopped,
    /// The next indivisible result cannot fit, so these limits cannot continue.
    Incomplete,
}

/// Resource that bounded an observation step.
#[napi(js_name = "ObservationStepLimitKind")]
pub enum ObservationStepLimitKind {
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

fn completion_kind(value: ApiObservationStepCompletion) -> ObservationStepCompletionKind {
    match value {
        ApiObservationStepCompletion::Complete => ObservationStepCompletionKind::Complete,
        ApiObservationStepCompletion::Stopped(_) => ObservationStepCompletionKind::Stopped,
        ApiObservationStepCompletion::Incomplete(_) => ObservationStepCompletionKind::Incomplete,
    }
}

fn completion_limit(value: ApiObservationStepCompletion) -> Option<ApiObservationStepLimit> {
    match value {
        ApiObservationStepCompletion::Complete => None,
        ApiObservationStepCompletion::Stopped(limit)
        | ApiObservationStepCompletion::Incomplete(limit) => Some(limit),
    }
}

fn limit_kind(value: ApiObservationStepLimit) -> ObservationStepLimitKind {
    match value {
        ApiObservationStepLimit::Inspected => ObservationStepLimitKind::Inspected,
        ApiObservationStepLimit::Returned => ObservationStepLimitKind::Returned,
        ApiObservationStepLimit::CopiedStringBytes { .. } => {
            ObservationStepLimitKind::CopiedStringBytes
        }
        ApiObservationStepLimit::CausalPathLookups => ObservationStepLimitKind::CausalPathLookups,
        ApiObservationStepLimit::CausalSteps { .. } => ObservationStepLimitKind::CausalSteps,
    }
}

fn minimum_required(value: ApiObservationStepLimit) -> Option<u32> {
    match value {
        ApiObservationStepLimit::CopiedStringBytes { minimum_required } => Some(minimum_required),
        ApiObservationStepLimit::CausalSteps { minimum_required } => {
            Some(u32::from(minimum_required))
        }
        ApiObservationStepLimit::Inspected
        | ApiObservationStepLimit::Returned
        | ApiObservationStepLimit::CausalPathLookups => None,
    }
}

/// One valid bounded observation-page step.
#[napi(js_name = "ObservationPage")]
pub struct ObservationPage {
    inner: ApiObservationPage,
}

#[napi]
impl ObservationPage {
    /// Return the exact snapshot cursor.
    #[napi(getter)]
    pub fn cursor(&self) -> ObservationCursor {
        ObservationCursor {
            inner: self.inner.cursor.clone(),
        }
    }

    /// Return deterministically ordered typed facts.
    #[napi(getter)]
    pub fn facts(&self) -> Vec<ObservationFact> {
        self.inner
            .facts
            .iter()
            .cloned()
            .map(|inner| ObservationFact { inner })
            .collect()
    }

    /// Return the continuation when matching facts remain.
    #[napi(getter)]
    pub fn continuation(&self) -> Option<ObservationPageCursor> {
        self.inner
            .continuation
            .clone()
            .map(|inner| ObservationPageCursor { inner })
    }

    /// Return whether this step exhausted its selected input.
    #[napi(getter)]
    pub fn completion(&self) -> ObservationStepCompletionKind {
        completion_kind(self.inner.completion)
    }

    /// Return the resource that stopped this step, if any.
    #[napi(getter)]
    pub fn stopped_by(&self) -> Option<ObservationStepLimitKind> {
        completion_limit(self.inner.completion).map(limit_kind)
    }

    /// Return the minimum bound needed by an indivisible result, if applicable.
    #[napi(getter)]
    pub fn minimum_required(&self) -> Option<u32> {
        completion_limit(self.inner.completion).and_then(minimum_required)
    }

    /// Return retained facts examined by this step.
    #[napi(getter)]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected
    }

    /// Return variable-length UTF-8 bytes copied by this step.
    #[napi(getter)]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes
    }

    /// Return facts omitted when the bounded snapshot was constructed.
    #[napi(getter)]
    pub fn snapshot_omitted_count(&self) -> u32 {
        self.inner.snapshot_omitted_count
    }
}

/// Full-snapshot recovery reason.
#[napi(js_name = "ObservationResynchronizationReason")]
pub enum ObservationResynchronizationReason {
    /// Generation mismatch.
    GenerationMismatch,
    /// Revision gap.
    RevisionGap,
    /// Page or lens continuation mismatch.
    ContinuationMismatch,
}

/// Typed instruction to request a new complete snapshot.
#[napi(js_name = "ObservationResynchronization")]
pub struct ObservationResynchronization {
    inner: ApiObservationResynchronization,
}

#[napi]
impl ObservationResynchronization {
    /// Return the exact recovery reason.
    #[napi(getter)]
    pub fn reason(&self) -> ObservationResynchronizationReason {
        match self.inner.reason {
            ApiObservationResynchronizationReason::GenerationMismatch { .. } => {
                ObservationResynchronizationReason::GenerationMismatch
            }
            ApiObservationResynchronizationReason::RevisionGap { .. } => {
                ObservationResynchronizationReason::RevisionGap
            }
            ApiObservationResynchronizationReason::ContinuationMismatch => {
                ObservationResynchronizationReason::ContinuationMismatch
            }
        }
    }

    /// Return the current cursor from which to restart.
    #[napi(getter)]
    pub fn current(&self) -> ObservationCursor {
        ObservationCursor {
            inner: self.inner.current.as_ref().clone(),
        }
    }

    /// Return the caller-held generation when a generation mismatch occurred.
    #[napi(getter)]
    pub fn requested_generation(&self) -> Option<ObservationGeneration> {
        match &self.inner.reason {
            ApiObservationResynchronizationReason::GenerationMismatch { requested, .. } => {
                Some(ObservationGeneration {
                    inner: requested.as_ref().clone(),
                })
            }
            _ => None,
        }
    }

    /// Return the current generation when a generation mismatch occurred.
    #[napi(getter)]
    pub fn current_generation(&self) -> Option<ObservationGeneration> {
        match &self.inner.reason {
            ApiObservationResynchronizationReason::GenerationMismatch { current, .. } => {
                Some(ObservationGeneration {
                    inner: current.as_ref().clone(),
                })
            }
            _ => None,
        }
    }

    /// Return the caller-held revision when a revision gap occurred.
    #[napi(getter)]
    pub fn requested_revision(&self) -> Option<ObservationRevision> {
        match &self.inner.reason {
            ApiObservationResynchronizationReason::RevisionGap { requested, .. } => {
                Some(ObservationRevision {
                    inner: requested.clone(),
                })
            }
            _ => None,
        }
    }

    /// Return the current revision when a revision gap occurred.
    #[napi(getter)]
    pub fn current_revision(&self) -> Option<ObservationRevision> {
        match &self.inner.reason {
            ApiObservationResynchronizationReason::RevisionGap { current, .. } => {
                Some(ObservationRevision {
                    inner: current.clone(),
                })
            }
            _ => None,
        }
    }
}

/// Page query disposition.
#[napi(js_name = "ObservationPageResultKind")]
pub enum ObservationPageResultKind {
    /// A valid page returned; inspect its completion before assuming exhaustion.
    Ready,
    /// Cancellation was observed before the step began.
    Cancelled,
    /// The supplied cursor cannot continue; obtain a fresh snapshot.
    Resynchronize,
}

/// Typed page query result; resynchronization is data, not an exception.
#[napi(js_name = "ObservationPageResult")]
pub struct ObservationPageResult {
    page: Option<ApiObservationPage>,
    cancelled_continuation: Option<ApiObservationPageCursor>,
    resynchronization: Option<ApiObservationResynchronization>,
}

#[napi]
impl ObservationPageResult {
    /// Return the closed result kind.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationPageResultKind {
        if self.page.is_some() {
            ObservationPageResultKind::Ready
        } else if self.cancelled_continuation.is_some() {
            ObservationPageResultKind::Cancelled
        } else {
            ObservationPageResultKind::Resynchronize
        }
    }

    /// Return the bounded page when ready.
    #[napi(getter)]
    pub fn page(&self) -> Option<ObservationPage> {
        self.page.clone().map(|inner| ObservationPage { inner })
    }

    /// Return the unchanged continuation after cancellation, if any.
    #[napi(getter)]
    pub fn cancelled_continuation(&self) -> Option<ObservationPageCursor> {
        self.cancelled_continuation
            .clone()
            .map(|inner| ObservationPageCursor { inner })
    }

    /// Return the full-snapshot instruction when resynchronization is required.
    #[napi(getter)]
    pub fn resynchronization(&self) -> Option<ObservationResynchronization> {
        self.resynchronization
            .clone()
            .map(|inner| ObservationResynchronization { inner })
    }
}

/// Cooperative cancellation state forwarded unchanged to the API.
#[napi(js_name = "ObservationCancellation")]
pub enum ObservationCancellation {
    /// Projection may proceed.
    Active,
    /// Caller cancelled before projection.
    Cancelled,
}

impl From<ObservationCancellation> for ApiObservationCancellation {
    fn from(value: ObservationCancellation) -> Self {
        match value {
            ObservationCancellation::Active => Self::Active,
            ObservationCancellation::Cancelled => Self::Cancelled,
        }
    }
}

/// Delta outcome kind.
#[napi(js_name = "ObservationDeltaOutcomeKind")]
pub enum ObservationDeltaOutcomeKind {
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
#[napi(js_name = "ObservationChangeKind")]
pub enum ObservationChangeKind {
    /// Insert or replace one fact.
    Upsert,
    /// Remove one fact key.
    Remove,
}

/// One deterministic fact change.
#[napi(js_name = "ObservationChange")]
pub struct ObservationChange {
    inner: ApiObservationChange,
}

#[napi]
impl ObservationChange {
    /// Return the closed change kind.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationChangeKind {
        match self.inner {
            ApiObservationChange::Upsert { .. } => ObservationChangeKind::Upsert,
            ApiObservationChange::Remove { .. } => ObservationChangeKind::Remove,
        }
    }

    /// Return the complete successor fact for an upsert.
    #[napi(getter)]
    pub fn fact(&self) -> Option<ObservationFact> {
        match &self.inner {
            ApiObservationChange::Upsert { fact } => Some(ObservationFact {
                inner: fact.clone(),
            }),
            ApiObservationChange::Remove { .. } => None,
        }
    }

    /// Return the removed category for a removal.
    #[napi(getter)]
    pub fn category(&self) -> Option<ObservationCategory> {
        match self.inner {
            ApiObservationChange::Remove { category, .. } => Some(category_from_api(category)),
            ApiObservationChange::Upsert { .. } => None,
        }
    }

    /// Return the removed subject for a removal.
    #[napi(getter)]
    pub fn subject(&self) -> Option<ObservationSubject> {
        match &self.inner {
            ApiObservationChange::Remove { subject, .. } => Some(ObservationSubject {
                inner: subject.clone(),
            }),
            ApiObservationChange::Upsert { .. } => None,
        }
    }
}

/// Safe recovery after backpressure.
#[napi(js_name = "ObservationBackpressureRecovery")]
pub enum ObservationBackpressureRecovery {
    /// Request a fresh bounded snapshot.
    RequestSnapshot,
}

/// Typed delta outcome retaining all recovery coordinates.
#[napi(js_name = "ObservationDeltaOutcome")]
pub struct ObservationDeltaOutcome {
    inner: ApiObservationDeltaOutcome,
}

#[napi]
impl ObservationDeltaOutcome {
    /// Return the closed outcome kind.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationDeltaOutcomeKind {
        match self.inner {
            ApiObservationDeltaOutcome::CaughtUp { .. } => ObservationDeltaOutcomeKind::CaughtUp,
            ApiObservationDeltaOutcome::Delta { .. } => ObservationDeltaOutcomeKind::Delta,
            ApiObservationDeltaOutcome::Resynchronize { .. } => {
                ObservationDeltaOutcomeKind::Resynchronize
            }
            ApiObservationDeltaOutcome::Cancelled { .. } => ObservationDeltaOutcomeKind::Cancelled,
            ApiObservationDeltaOutcome::Backpressured { .. } => {
                ObservationDeltaOutcomeKind::Backpressured
            }
        }
    }

    /// Return the current cursor for caught-up or backpressured outcomes.
    #[napi(getter)]
    pub fn cursor(&self) -> Option<ObservationCursor> {
        match &self.inner {
            ApiObservationDeltaOutcome::CaughtUp { cursor }
            | ApiObservationDeltaOutcome::Backpressured {
                current: cursor, ..
            } => Some(ObservationCursor {
                inner: cursor.clone(),
            }),
            _ => None,
        }
    }

    /// Return the predecessor cursor for a complete delta.
    #[napi(getter)]
    pub fn from(&self) -> Option<ObservationCursor> {
        match &self.inner {
            ApiObservationDeltaOutcome::Delta { delta } => Some(ObservationCursor {
                inner: delta.from.clone(),
            }),
            _ => None,
        }
    }

    /// Return the successor cursor for a complete delta.
    #[napi(getter)]
    pub fn to(&self) -> Option<ObservationCursor> {
        match &self.inner {
            ApiObservationDeltaOutcome::Delta { delta } => Some(ObservationCursor {
                inner: delta.to.clone(),
            }),
            _ => None,
        }
    }

    /// Return the complete ordered change batch for a delta.
    #[napi(getter)]
    pub fn changes(&self) -> Vec<ObservationChange> {
        match &self.inner {
            ApiObservationDeltaOutcome::Delta { delta } => delta
                .changes
                .iter()
                .cloned()
                .map(|inner| ObservationChange { inner })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Return the omitted-fact count installed with a complete delta.
    #[napi(getter)]
    pub fn successor_omitted_fact_count(&self) -> Option<u32> {
        match &self.inner {
            ApiObservationDeltaOutcome::Delta { delta } => Some(delta.successor_omitted_fact_count),
            _ => None,
        }
    }

    /// Return the recovery instruction when resynchronization is required.
    #[napi(getter)]
    pub fn resynchronization(&self) -> Option<ObservationResynchronization> {
        match &self.inner {
            ApiObservationDeltaOutcome::Resynchronize { resynchronization } => {
                Some(ObservationResynchronization {
                    inner: resynchronization.clone(),
                })
            }
            _ => None,
        }
    }

    /// Return the completely applied cursor retained after cancellation.
    #[napi(getter)]
    pub fn cancelled_at(&self) -> Option<ObservationCursor> {
        match &self.inner {
            ApiObservationDeltaOutcome::Cancelled { resume } => Some(ObservationCursor {
                inner: resume.cursor().clone(),
            }),
            _ => None,
        }
    }

    /// Return the required atomic change count after backpressure.
    #[napi(getter)]
    pub fn pending_changes(&self) -> Option<u32> {
        match self.inner {
            ApiObservationDeltaOutcome::Backpressured {
                pending_changes, ..
            } => Some(pending_changes),
            _ => None,
        }
    }

    /// Return the caller-declared change ceiling after backpressure.
    #[napi(getter)]
    pub fn maximum_changes(&self) -> Option<u16> {
        match self.inner {
            ApiObservationDeltaOutcome::Backpressured {
                maximum_changes, ..
            } => Some(maximum_changes),
            _ => None,
        }
    }

    /// Return the safe recovery action after backpressure.
    #[napi(getter)]
    pub fn recovery(&self) -> Option<ObservationBackpressureRecovery> {
        match self.inner {
            ApiObservationDeltaOutcome::Backpressured {
                recovery: ApiObservationBackpressureRecovery::RequestSnapshot,
                ..
            } => Some(ObservationBackpressureRecovery::RequestSnapshot),
            _ => None,
        }
    }
}

/// Causal proof-step kind.
#[napi(js_name = "ObservationCausalStepKind")]
pub enum ObservationCausalStepKind {
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
#[napi(js_name = "ObservationCausalStep")]
pub struct ObservationCausalStep {
    inner: ApiObservationCausalStep,
}

#[napi]
impl ObservationCausalStep {
    /// Return the closed proof-step kind.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationCausalStepKind {
        match self.inner {
            ApiObservationCausalStep::OriginReceipt { .. } => {
                ObservationCausalStepKind::OriginReceipt
            }
            ApiObservationCausalStep::EntryContract { .. } => {
                ObservationCausalStepKind::EntryContract
            }
            ApiObservationCausalStep::ResponsibilityContract { .. } => {
                ObservationCausalStepKind::ResponsibilityContract
            }
            ApiObservationCausalStep::ProviderBinding { .. } => {
                ObservationCausalStepKind::ProviderBinding
            }
            ApiObservationCausalStep::DelegationEdge { .. } => {
                ObservationCausalStepKind::DelegationEdge
            }
            ApiObservationCausalStep::TerminationContract { .. } => {
                ObservationCausalStepKind::TerminationContract
            }
            ApiObservationCausalStep::SettlementContract { .. } => {
                ObservationCausalStepKind::SettlementContract
            }
            ApiObservationCausalStep::ContinuityContract { .. } => {
                ObservationCausalStepKind::ContinuityContract
            }
        }
    }

    /// Return the exact referenced contract, binding, or edge identity.
    #[napi(getter)]
    pub fn reference_id(&self) -> String {
        match &self.inner {
            ApiObservationCausalStep::OriginReceipt { contract_id }
            | ApiObservationCausalStep::EntryContract { contract_id }
            | ApiObservationCausalStep::ResponsibilityContract { contract_id }
            | ApiObservationCausalStep::TerminationContract { contract_id }
            | ApiObservationCausalStep::SettlementContract { contract_id }
            | ApiObservationCausalStep::ContinuityContract { contract_id } => contract_id.clone(),
            ApiObservationCausalStep::ProviderBinding { binding_id } => binding_id.clone(),
            ApiObservationCausalStep::DelegationEdge { edge_id } => edge_id.clone(),
        }
    }
}

/// One bounded exact causal proof.
#[napi(js_name = "ObservationCausalPath")]
pub struct ObservationCausalPath {
    inner: ApiObservationCausalPath,
}

#[napi]
impl ObservationCausalPath {
    /// Return the generation in which the proof is meaningful.
    #[napi(getter)]
    pub fn generation(&self) -> ObservationGeneration {
        ObservationGeneration {
            inner: self.inner.generation.clone(),
        }
    }

    /// Return the exact selected subject.
    #[napi(getter)]
    pub fn subject(&self) -> ObservationSubject {
        ObservationSubject {
            inner: self.inner.subject.clone(),
        }
    }

    /// Return the ordered compiler-owned proof steps.
    #[napi(getter)]
    pub fn steps(&self) -> Vec<ObservationCausalStep> {
        self.inner
            .steps
            .iter()
            .cloned()
            .map(|inner| ObservationCausalStep { inner })
            .collect()
    }
}

/// Causal query disposition.
#[napi(js_name = "ObservationCausalResultKind")]
pub enum ObservationCausalResultKind {
    /// Exact proof found.
    Found,
    /// Subject has no retained proof.
    NotFound,
    /// Selector generation is no longer current.
    Resynchronize,
}

/// Typed causal query result.
#[napi(js_name = "ObservationCausalPathResult")]
pub struct ObservationCausalPathResult {
    path: Option<ApiObservationCausalPath>,
    resynchronization: Option<ApiObservationResynchronization>,
}

#[napi]
impl ObservationCausalPathResult {
    /// Return the closed query disposition.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationCausalResultKind {
        if self.resynchronization.is_some() {
            ObservationCausalResultKind::Resynchronize
        } else if self.path.is_some() {
            ObservationCausalResultKind::Found
        } else {
            ObservationCausalResultKind::NotFound
        }
    }

    /// Return the exact proof when found.
    #[napi(getter)]
    pub fn path(&self) -> Option<ObservationCausalPath> {
        self.path
            .clone()
            .map(|inner| ObservationCausalPath { inner })
    }

    /// Return the recovery instruction for a stale selector generation.
    #[napi(getter)]
    pub fn resynchronization(&self) -> Option<ObservationResynchronization> {
        self.resynchronization
            .clone()
            .map(|inner| ObservationResynchronization { inner })
    }
}

/// One lens node and its complete selected facts.
#[napi(js_name = "ObservationLensNode")]
pub struct ObservationLensNode {
    inner: ApiObservationLensNode,
}

/// Opaque continuation for one exact bounded lens query.
///
/// It binds the snapshot, filter, causal-path choice, raw limits, and next
/// subject. Pass it only to the same query. Its position does not prove that
/// later subjects are absent.
#[napi(js_name = "ObservationLensCursor")]
pub struct ObservationLensCursor {
    inner: ApiObservationLensCursor,
}

#[napi]
impl ObservationLensCursor {
    /// Return the exact snapshot cursor.
    #[napi(getter)]
    pub fn snapshot(&self) -> ObservationCursor {
        ObservationCursor {
            inner: self.inner.snapshot().clone(),
        }
    }

    /// Return how many retained subjects precede the next step.
    #[napi(getter)]
    pub fn inspected_position(&self) -> u32 {
        self.inner.inspected_position()
    }
}

#[napi]
impl ObservationLensNode {
    /// Return the exact admitted subject.
    #[napi(getter)]
    pub fn subject(&self) -> ObservationSubject {
        ObservationSubject {
            inner: self.inner.subject.clone(),
        }
    }

    /// Return the complete typed facts selected for this subject.
    #[napi(getter)]
    pub fn facts(&self) -> Vec<ObservationFact> {
        self.inner
            .facts
            .iter()
            .cloned()
            .map(|inner| ObservationFact { inner })
            .collect()
    }
}

/// Bounded generation-bound visualization substrate.
#[napi(js_name = "ObservationLens")]
pub struct ObservationLens {
    inner: ApiObservationLens,
}

#[napi]
impl ObservationLens {
    /// Return the exact source snapshot coordinate.
    #[napi(getter)]
    pub fn cursor(&self) -> ObservationCursor {
        ObservationCursor {
            inner: self.inner.cursor.clone(),
        }
    }

    /// Return deterministically ordered subject nodes.
    #[napi(getter)]
    pub fn nodes(&self) -> Vec<ObservationLensNode> {
        self.inner
            .nodes
            .iter()
            .cloned()
            .map(|inner| ObservationLensNode { inner })
            .collect()
    }

    /// Return retained causal paths when requested.
    #[napi(getter)]
    pub fn causal_paths(&self) -> Vec<ObservationCausalPath> {
        self.inner
            .causal_paths
            .iter()
            .cloned()
            .map(|inner| ObservationCausalPath { inner })
            .collect()
    }

    /// Return the continuation when selected subjects may remain.
    #[napi(getter)]
    pub fn continuation(&self) -> Option<ObservationLensCursor> {
        self.inner
            .continuation
            .clone()
            .map(|inner| ObservationLensCursor { inner })
    }

    /// Return whether this step exhausted its selected input.
    #[napi(getter)]
    pub fn completion(&self) -> ObservationStepCompletionKind {
        completion_kind(self.inner.completion)
    }

    /// Return the resource that stopped this step, if any.
    #[napi(getter)]
    pub fn stopped_by(&self) -> Option<ObservationStepLimitKind> {
        completion_limit(self.inner.completion).map(limit_kind)
    }

    /// Return the minimum bound needed by an indivisible result, if applicable.
    #[napi(getter)]
    pub fn minimum_required(&self) -> Option<u32> {
        completion_limit(self.inner.completion).and_then(minimum_required)
    }

    /// Return retained subjects examined by this step.
    #[napi(getter)]
    pub fn inspected_subjects(&self) -> u16 {
        self.inner.inspected_subjects
    }

    /// Return variable-length UTF-8 bytes copied by this step.
    #[napi(getter)]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes
    }

    /// Return causal-path lookups performed by this step.
    ///
    /// A stopped step can count a lookup for an unreturned subject.
    #[napi(getter)]
    pub fn causal_path_lookups(&self) -> u16 {
        self.inner.causal_path_lookups
    }

    /// Return causal steps copied by this step.
    #[napi(getter)]
    pub fn causal_steps(&self) -> u16 {
        self.inner.causal_steps
    }

    /// Return owner facts omitted when the bounded snapshot was constructed.
    #[napi(getter)]
    pub fn snapshot_omitted_count(&self) -> u32 {
        self.inner.snapshot_omitted_count
    }
}

/// Cooperative visual-lens disposition.
#[napi(js_name = "ObservationLensDispositionKind")]
pub enum ObservationLensDispositionKind {
    /// A valid lens step returned; inspect its completion before assuming exhaustion.
    Ready,
    /// The supplied cursor cannot continue; obtain a fresh snapshot.
    Resynchronize,
    /// Cancelled before this step began; the supplied cursor is unchanged.
    Cancelled,
}

/// Typed visual-lens result.
#[napi(js_name = "ObservationLensDisposition")]
pub struct ObservationLensDisposition {
    inner: ApiObservationLensDisposition,
}

#[napi]
impl ObservationLensDisposition {
    /// Return the closed disposition kind.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationLensDispositionKind {
        match self.inner {
            ApiObservationLensDisposition::Ready { .. } => ObservationLensDispositionKind::Ready,
            ApiObservationLensDisposition::Resynchronize { .. } => {
                ObservationLensDispositionKind::Resynchronize
            }
            ApiObservationLensDisposition::Cancelled { .. } => {
                ObservationLensDispositionKind::Cancelled
            }
        }
    }

    /// Return the bounded lens step when ready.
    #[napi(getter)]
    pub fn lens(&self) -> Option<ObservationLens> {
        match &self.inner {
            ApiObservationLensDisposition::Ready { lens } => Some(ObservationLens {
                inner: lens.clone(),
            }),
            ApiObservationLensDisposition::Resynchronize { .. }
            | ApiObservationLensDisposition::Cancelled { .. } => None,
        }
    }

    /// Return the recovery instruction for a mismatched continuation.
    #[napi(getter)]
    pub fn resynchronization(&self) -> Option<ObservationResynchronization> {
        match &self.inner {
            ApiObservationLensDisposition::Resynchronize { resynchronization } => {
                Some(ObservationResynchronization {
                    inner: resynchronization.clone(),
                })
            }
            ApiObservationLensDisposition::Ready { .. }
            | ApiObservationLensDisposition::Cancelled { .. } => None,
        }
    }

    /// Return the unchanged continuation after cancellation, if any.
    #[napi(getter)]
    pub fn cancelled_continuation(&self) -> Option<ObservationLensCursor> {
        match &self.inner {
            ApiObservationLensDisposition::Cancelled { continuation } => continuation
                .clone()
                .map(|inner| ObservationLensCursor { inner }),
            ApiObservationLensDisposition::Ready { .. }
            | ApiObservationLensDisposition::Resynchronize { .. } => None,
        }
    }
}

/// Opaque source-free observation snapshot.
#[napi(js_name = "ObservationSnapshot")]
pub struct ObservationSnapshot {
    pub(crate) inner: ApiObservationSnapshot,
}

#[napi]
impl ObservationSnapshot {
    /// Return the exact complete-snapshot cursor.
    #[napi(getter)]
    pub fn cursor(&self) -> ObservationCursor {
        ObservationCursor {
            inner: self.inner.cursor(),
        }
    }

    /// Return the retained fact count.
    #[napi(getter)]
    pub fn fact_count(&self) -> u32 {
        self.inner.summary().fact_count
    }

    /// Return the count omitted by snapshot construction.
    #[napi(getter)]
    pub fn omitted_fact_count(&self) -> u32 {
        self.inner.summary().omitted_fact_count
    }

    /// Query one bounded page. A ready result can still stop before exhaustion.
    #[napi]
    pub fn page(
        &self,
        filter: &ObservationFilter,
        limits: ObservationPageLimits,
        cancellation: ObservationCancellation,
        after: Option<&ObservationPageCursor>,
    ) -> ObservationPageResult {
        let mut request = ObservationPageRequest::bounded(
            filter.inner.clone(),
            ApiObservationPageLimits::new(
                limits.maximum_inspected,
                limits.maximum_returned,
                limits.maximum_copied_string_bytes,
            ),
            cancellation.into(),
        );
        if let Some(after) = after {
            request = request.after(after.inner.clone());
        }
        match self.inner.page(request) {
            Ok(ApiObservationPageDisposition::Ready { page }) => ObservationPageResult {
                page: Some(page),
                cancelled_continuation: None,
                resynchronization: None,
            },
            Ok(ApiObservationPageDisposition::Cancelled { continuation }) => {
                ObservationPageResult {
                    page: None,
                    cancelled_continuation: continuation,
                    resynchronization: None,
                }
            }
            Err(resynchronization) => ObservationPageResult {
                page: None,
                cancelled_continuation: None,
                resynchronization: Some(resynchronization),
            },
        }
    }

    /// Derive one adjacent atomic delta from an earlier API snapshot.
    #[napi]
    pub fn delta_from(
        &self,
        prior: &ObservationSnapshot,
        maximum_changes: u16,
        cancellation: ObservationCancellation,
    ) -> ObservationDeltaOutcome {
        let request = ObservationDeltaRequest::new(
            prior.inner.resume_coordinate(),
            maximum_changes,
            cancellation.into(),
        );
        ObservationDeltaOutcome {
            inner: self.inner.delta_from(&prior.inner, request),
        }
    }

    /// Return one retained exact causal path for an API-issued subject.
    #[napi]
    pub fn causal_path(&self, subject: &ObservationSubject) -> ObservationCausalPathResult {
        let selector = ObservationCausalSelector::new(&self.inner, &subject.inner);
        match self.inner.causal_path(&selector) {
            Ok(path) => ObservationCausalPathResult {
                path,
                resynchronization: None,
            },
            Err(resynchronization) => ObservationCausalPathResult {
                path: None,
                resynchronization: Some(resynchronization),
            },
        }
    }

    /// Build one bounded non-authoritative lens step.
    ///
    /// The result carries snapshot-bound facts and optional retained causal
    /// paths, not layout or proof that omitted subjects are absent.
    #[napi]
    pub fn visual_lens(
        &self,
        filter: &ObservationFilter,
        limits: ObservationLensLimits,
        include_causal_paths: bool,
        cancellation: ObservationCancellation,
        after: Option<&ObservationLensCursor>,
    ) -> ObservationLensDisposition {
        let mut request = ObservationLensRequest::bounded(
            filter.inner.clone(),
            ApiObservationLensLimits::new(
                limits.maximum_inspected,
                limits.maximum_nodes,
                limits.maximum_copied_string_bytes,
                limits.maximum_causal_path_lookups,
                limits.maximum_causal_steps,
            ),
            include_causal_paths,
            cancellation.into(),
        );
        if let Some(after) = after {
            request = request.after(after.inner.clone());
        }
        ObservationLensDisposition {
            inner: self.inner.visual_lens(request),
        }
    }
}

#[napi]
impl Pale {
    /// Project an admitted effective profile into a source-free snapshot.
    #[napi]
    pub fn observe_effective_profile(
        &self,
        compilation: &EffectiveProfileCompilation,
    ) -> ObservationSnapshot {
        ObservationSnapshot {
            inner: self.api.observe_effective_profile(&compilation.inner),
        }
    }
}
