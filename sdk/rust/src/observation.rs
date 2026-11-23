//! Read retained configuration facts and compare observation snapshots.
//!
//! [`PaleApi::observe_effective_profile`] observes compiled configuration,
//! not a running machine or provider. Use [`ObservationSnapshot::page`] to read
//! its facts and [`ObservationSnapshot::causal_path`] to inspect retained
//! explanations. Omitted counts identify facts excluded by snapshot bounds.
//!
//! Snapshot, page, resume, and subject handles preserve the identities supplied
//! by the application. They are not interchangeable strings or proof that a
//! client has applied an update. [`ObservationSnapshot::delta_from`] describes
//! how to compare retained snapshots without accepting a partial update.
//! These types expose neither source bodies nor internal graphs, journal
//! cursors, or store handles. Product-specific display remains client-owned.

use std::sync::Arc;

mod profile;
pub use profile::{
    ObservationProfileAssociation, ObservationProfileContinuation, ObservationProfileError,
    ObservationProfileMapping, ObservationProfilePage,
};

use pale_application::observation as application;

use crate::{
    DiagnosticEffectCertainty, EffectiveProfileCompilation, EffectiveProfileOperationExposure,
    EffectiveProfileRequirementDisposition, PaleApi,
};

/// Exact admitted generation namespace of an observation snapshot.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ObservationGeneration {
    /// Immutable effective-profile compiler phase.
    EffectiveProfile {
        /// Canonical profile digest.
        profile_digest: String,
        /// Exact machine-resolved instance represented by the snapshot.
        instance_digest: String,
        /// Composition proof used by the retained causal paths.
        composition_plan_digest: String,
    },
    /// Resident authority at one exact store and optional active generation.
    Resident {
        /// Stable authority-store identity.
        control_store_id: String,
        /// Physical incarnation of the authority store.
        control_incarnation_id: String,
        /// Active compiled-generation digest, absent before activation.
        active_generation_digest: Option<String>,
    },
}

impl From<&application::ObservationGeneration> for ObservationGeneration {
    fn from(value: &application::ObservationGeneration) -> Self {
        match value {
            application::ObservationGeneration::EffectiveProfile {
                profile_digest,
                instance_digest,
                composition_plan_digest,
            } => Self::EffectiveProfile {
                profile_digest: profile_digest.as_str().to_owned(),
                instance_digest: instance_digest.as_str().to_owned(),
                composition_plan_digest: composition_plan_digest.as_str().to_owned(),
            },
            application::ObservationGeneration::Resident {
                authority_store,
                active_generation,
            } => Self::Resident {
                control_store_id: authority_store.store_id().as_str().to_owned(),
                control_incarnation_id: authority_store.incarnation_id().as_str().to_owned(),
                active_generation_digest: active_generation
                    .as_ref()
                    .map(|digest| digest.as_str().to_owned()),
            },
        }
    }
}

/// Revision domain within one [`ObservationGeneration`].
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ObservationRevision {
    /// Immutable effective-profile state.
    Static,
    /// Resident authority at an optional durable journal sequence.
    ResidentJournal {
        /// Durable journal head, absent before the first append.
        sequence: Option<u64>,
    },
}

impl From<&application::ObservationRevision> for ObservationRevision {
    fn from(value: &application::ObservationRevision) -> Self {
        match value {
            application::ObservationRevision::Static => Self::Static,
            application::ObservationRevision::ResidentJournal(sequence) => Self::ResidentJournal {
                sequence: sequence.map(|sequence| sequence.get()),
            },
        }
    }
}

/// Opaque generation/revision cursor issued by one snapshot.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationCursor {
    generation: ObservationGeneration,
    revision: ObservationRevision,
}

impl ObservationCursor {
    fn from_application(cursor: application::ObservationCursor) -> Self {
        Self {
            generation: cursor.generation().into(),
            revision: cursor.revision().into(),
        }
    }

    /// Borrow the admitted generation coordinate.
    #[must_use]
    pub const fn generation(&self) -> &ObservationGeneration {
        &self.generation
    }

    /// Borrow the revision within [`Self::generation`].
    #[must_use]
    pub const fn revision(&self) -> &ObservationRevision {
        &self.revision
    }
}

/// Opaque stream-resume coordinate, distinct from page continuation.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationResumeCoordinate {
    cursor: ObservationCursor,
    #[serde(skip)]
    application: application::ObservationResumeCoordinate,
}

impl ObservationResumeCoordinate {
    fn from_application(value: application::ObservationResumeCoordinate) -> Self {
        Self {
            cursor: ObservationCursor::from_application(value.cursor().clone()),
            application: value,
        }
    }

    /// Borrow the last completely applied cursor.
    #[must_use]
    pub const fn cursor(&self) -> &ObservationCursor {
        &self.cursor
    }
}

/// Independent observation axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationCategory {
    /// Exact three-state operation exposure.
    OperationExposure,
    /// Portable capability-requirement disposition.
    CapabilityRequirementDisposition,
    /// Health evidence.
    Health,
    /// Admission or operational readiness.
    Readiness,
    /// External-effect certainty.
    Effect,
    /// Native-owner evidence.
    NativeOwner,
    /// Reconciliation state.
    Reconciliation,
    /// Desired resident process state.
    ProcessIntent,
}

impl From<application::ObservationCategory> for ObservationCategory {
    fn from(value: application::ObservationCategory) -> Self {
        match value {
            application::ObservationCategory::OperationExposure => Self::OperationExposure,
            application::ObservationCategory::CapabilityRequirementDisposition => {
                Self::CapabilityRequirementDisposition
            }
            application::ObservationCategory::Health => Self::Health,
            application::ObservationCategory::Readiness => Self::Readiness,
            application::ObservationCategory::Effect => Self::Effect,
            application::ObservationCategory::NativeOwner => Self::NativeOwner,
            application::ObservationCategory::Reconciliation => Self::Reconciliation,
            application::ObservationCategory::ProcessIntent => Self::ProcessIntent,
        }
    }
}

impl From<ObservationCategory> for application::ObservationCategory {
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

/// Exact typed subject copied from an admitted application projection.
///
/// The inner nominal coordinate is private, so consumers cannot fabricate a
/// Package, repository, generation, or capability identity from arbitrary text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationSubject {
    projection: ObservationSubjectProjection,
    application: application::ObservationSubject,
}

/// Closed kind of an opaque [`ObservationSubject`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationSubjectKind {
    /// Complete effective profile.
    EffectiveProfile,
    /// Selected private machine identity.
    Machine,
    /// One logical repository.
    Repository,
    /// One Package-owned operation in one repository.
    Operation,
    /// One capability-requirement occurrence.
    CapabilityRequirement,
    /// One selected execution path.
    ExecutionPath,
    /// One immutable compiled generation.
    CompiledGeneration,
    /// Complete resident process intent.
    ResidentProcess,
}

impl serde::Serialize for ObservationSubject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(&self.projection, serializer)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum ObservationSubjectProjection {
    EffectiveProfile {
        profile_digest: String,
    },
    Machine {
        machine_id: String,
    },
    Repository {
        repository_id: String,
    },
    Operation {
        repository_id: String,
        operation_id: String,
    },
    CapabilityRequirement {
        repository_id: String,
        operation_id: String,
        requirement_id: String,
        capability_id: String,
    },
    ExecutionPath {
        execution_path_id: String,
    },
    CompiledGeneration {
        compiled_generation_digest: String,
    },
    ResidentProcess,
}

impl From<&application::ObservationSubject> for ObservationSubject {
    fn from(value: &application::ObservationSubject) -> Self {
        let projection = match value {
            application::ObservationSubject::EffectiveProfile(digest) => {
                ObservationSubjectProjection::EffectiveProfile {
                    profile_digest: digest.as_str().to_owned(),
                }
            }
            application::ObservationSubject::Machine(machine_id) => {
                ObservationSubjectProjection::Machine {
                    machine_id: machine_id.clone(),
                }
            }
            application::ObservationSubject::Repository(repository_id) => {
                ObservationSubjectProjection::Repository {
                    repository_id: repository_id.as_str().to_owned(),
                }
            }
            application::ObservationSubject::Operation {
                repository_id,
                operation_id,
            } => ObservationSubjectProjection::Operation {
                repository_id: repository_id.as_str().to_owned(),
                operation_id: operation_id.to_string(),
            },
            application::ObservationSubject::CapabilityRequirement {
                repository_id,
                operation_id,
                requirement_id,
                capability_id,
            } => ObservationSubjectProjection::CapabilityRequirement {
                repository_id: repository_id.as_str().to_owned(),
                operation_id: operation_id.to_string(),
                requirement_id: requirement_id.as_str().to_owned(),
                capability_id: capability_id.as_str().to_owned(),
            },
            application::ObservationSubject::ExecutionPath(path) => {
                ObservationSubjectProjection::ExecutionPath {
                    execution_path_id: path.as_str().to_owned(),
                }
            }
            application::ObservationSubject::CompiledGeneration(digest) => {
                ObservationSubjectProjection::CompiledGeneration {
                    compiled_generation_digest: digest.as_str().to_owned(),
                }
            }
            application::ObservationSubject::ResidentProcess => {
                ObservationSubjectProjection::ResidentProcess
            }
        };
        Self {
            projection,
            application: value.clone(),
        }
    }
}

impl ObservationSubject {
    /// Return the closed subject kind without exposing its nominal coordinate.
    #[must_use]
    pub const fn kind(&self) -> ObservationSubjectKind {
        match &self.projection {
            ObservationSubjectProjection::EffectiveProfile { .. } => {
                ObservationSubjectKind::EffectiveProfile
            }
            ObservationSubjectProjection::Machine { .. } => ObservationSubjectKind::Machine,
            ObservationSubjectProjection::Repository { .. } => ObservationSubjectKind::Repository,
            ObservationSubjectProjection::Operation { .. } => ObservationSubjectKind::Operation,
            ObservationSubjectProjection::CapabilityRequirement { .. } => {
                ObservationSubjectKind::CapabilityRequirement
            }
            ObservationSubjectProjection::ExecutionPath { .. } => {
                ObservationSubjectKind::ExecutionPath
            }
            ObservationSubjectProjection::CompiledGeneration { .. } => {
                ObservationSubjectKind::CompiledGeneration
            }
            ObservationSubjectProjection::ResidentProcess => {
                ObservationSubjectKind::ResidentProcess
            }
        }
    }

    /// Borrow the profile digest when [`Self::kind`] is effective profile.
    #[must_use]
    pub fn profile_digest(&self) -> Option<&str> {
        match &self.projection {
            ObservationSubjectProjection::EffectiveProfile { profile_digest } => {
                Some(profile_digest)
            }
            _ => None,
        }
    }

    /// Borrow the machine identity when [`Self::kind`] is machine.
    #[must_use]
    pub fn machine_id(&self) -> Option<&str> {
        match &self.projection {
            ObservationSubjectProjection::Machine { machine_id } => Some(machine_id),
            _ => None,
        }
    }

    /// Borrow the repository identity carried by a repository-scoped subject.
    #[must_use]
    pub fn repository_id(&self) -> Option<&str> {
        match &self.projection {
            ObservationSubjectProjection::Repository { repository_id }
            | ObservationSubjectProjection::Operation { repository_id, .. }
            | ObservationSubjectProjection::CapabilityRequirement { repository_id, .. } => {
                Some(repository_id)
            }
            _ => None,
        }
    }

    /// Borrow the operation identity carried by an operation-scoped subject.
    #[must_use]
    pub fn operation_id(&self) -> Option<&str> {
        match &self.projection {
            ObservationSubjectProjection::Operation { operation_id, .. }
            | ObservationSubjectProjection::CapabilityRequirement { operation_id, .. } => {
                Some(operation_id)
            }
            _ => None,
        }
    }

    /// Borrow the requirement identity when this subject is a requirement occurrence.
    #[must_use]
    pub fn requirement_id(&self) -> Option<&str> {
        match &self.projection {
            ObservationSubjectProjection::CapabilityRequirement { requirement_id, .. } => {
                Some(requirement_id)
            }
            _ => None,
        }
    }

    /// Borrow the capability identity when this subject is a requirement occurrence.
    #[must_use]
    pub fn capability_id(&self) -> Option<&str> {
        match &self.projection {
            ObservationSubjectProjection::CapabilityRequirement { capability_id, .. } => {
                Some(capability_id)
            }
            _ => None,
        }
    }

    /// Borrow the execution-path identity when [`Self::kind`] is execution path.
    #[must_use]
    pub fn execution_path_id(&self) -> Option<&str> {
        match &self.projection {
            ObservationSubjectProjection::ExecutionPath { execution_path_id } => {
                Some(execution_path_id)
            }
            _ => None,
        }
    }

    /// Borrow the compiled-generation digest when that is the subject kind.
    #[must_use]
    pub fn compiled_generation_digest(&self) -> Option<&str> {
        match &self.projection {
            ObservationSubjectProjection::CompiledGeneration {
                compiled_generation_digest,
            } => Some(compiled_generation_digest),
            _ => None,
        }
    }
}

/// Temporal qualification of one fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ObservationFreshness {
    /// Current for the exact snapshot coordinate.
    Fresh,
    /// Retained from an older source revision.
    Stale {
        /// Number of source revisions by which this evidence trails.
        revisions_behind: u64,
    },
    /// Currency is not established by owner evidence.
    Unknown,
}

impl From<application::ObservationFreshness> for ObservationFreshness {
    fn from(value: application::ObservationFreshness) -> Self {
        match value {
            application::ObservationFreshness::Fresh => Self::Fresh,
            application::ObservationFreshness::Stale { revisions_behind } => {
                Self::Stale { revisions_behind }
            }
            application::ObservationFreshness::Unknown => Self::Unknown,
        }
    }
}

/// Freshness class accepted by [`ObservationFilter`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationFreshnessClass {
    /// Current for the snapshot.
    Fresh,
    /// Retained from an older revision.
    Stale,
    /// Currency is not established.
    Unknown,
}

impl From<ObservationFreshnessClass> for application::ObservationFreshnessClass {
    fn from(value: ObservationFreshnessClass) -> Self {
        match value {
            ObservationFreshnessClass::Fresh => Self::Fresh,
            ObservationFreshnessClass::Stale => Self::Stale,
            ObservationFreshnessClass::Unknown => Self::Unknown,
        }
    }
}

/// Health state independent of readiness and effect certainty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationHealthState {
    /// Owner evidence satisfies the applicable bounded health predicate.
    Healthy,
    /// Owner evidence reports reduced but usable operation.
    Degraded,
    /// Owner evidence reports an unusable health predicate.
    Unhealthy,
    /// A selected owner supplied no qualifying health evidence.
    Unknown,
    /// No admitted carrier can obtain this health class.
    Unavailable,
}

/// Readiness state independent of health and native evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationReadinessState {
    /// The applicable readiness predicate is satisfied by admitted evidence.
    Ready,
    /// An admitted policy ceiling rejects the subject.
    Blocked,
    /// A positive admitted ceiling omitted the subject.
    Narrowed,
    /// Portable meaning is coherent but owner-native evidence is absent.
    Unproven,
    /// A required binding or carrier is absent.
    Unavailable,
}

/// Native-owner evidence without copied native payload.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ObservationNativeState {
    /// No selected native observation carrier exists.
    Unavailable,
    /// A native owner is selected, but no qualifying observation was admitted.
    Unknown,
    /// Exact owner-native evidence was admitted without copying its body.
    Observed {
        /// Native system retaining source truth.
        owner_system_id: String,
        /// Owner-defined observation kind.
        observation_kind: String,
    },
}

/// Reconciliation state independent of effect certainty.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ObservationReconciliationState {
    /// No effect uncertainty created a reconciliation obligation.
    NotRequired,
    /// An exact case retains unresolved custody.
    Required {
        /// Reconciliation case digest.
        case_digest: String,
    },
    /// An exact case is being reconciled without a terminal result.
    InProgress {
        /// Reconciliation case digest.
        case_digest: String,
    },
    /// An exact terminal reconciliation result was admitted.
    Settled {
        /// Terminal result digest.
        result_digest: String,
    },
    /// A relevant owner exists, but its reconciliation frontier is unobserved.
    Unknown,
    /// No admitted reconciliation carrier exists.
    Unavailable,
}

/// Desired resident process state, not observed liveness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationProcessIntent {
    /// All admitted services are desired running.
    Running,
    /// All admitted services are desired stopped.
    Stopped,
}

/// One independently typed observation fact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "category",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ObservationFact {
    /// Exact operation exposure without collapsing readiness.
    OperationExposure {
        /// Exact repository-scoped operation.
        subject: ObservationSubject,
        /// Unbound, semantic-only, or capability-bearing state.
        state: EffectiveProfileOperationExposure,
        /// Temporal qualification of the compiler-owned state.
        freshness: ObservationFreshness,
    },
    /// Exact portable requirement disposition without native inference.
    CapabilityRequirementDisposition {
        /// Exact capability-requirement occurrence.
        subject: ObservationSubject,
        /// Satisfied, missing-provider, narrowed, or blocked state.
        state: EffectiveProfileRequirementDisposition,
        /// Temporal qualification of the compiler-owned state.
        freshness: ObservationFreshness,
    },
    /// Bounded health evidence.
    Health {
        /// Exact typed subject.
        subject: ObservationSubject,
        /// Health classification only.
        state: ObservationHealthState,
        /// Temporal qualification of the health evidence.
        freshness: ObservationFreshness,
    },
    /// Admission or operational readiness.
    Readiness {
        /// Exact typed subject.
        subject: ObservationSubject,
        /// Readiness classification only.
        state: ObservationReadinessState,
        /// Temporal qualification of the readiness evidence.
        freshness: ObservationFreshness,
    },
    /// External-effect certainty using the existing diagnostic algebra.
    Effect {
        /// Exact typed subject.
        subject: ObservationSubject,
        /// What admitted evidence proves about effect execution.
        certainty: DiagnosticEffectCertainty,
        /// Temporal qualification of the effect evidence.
        freshness: ObservationFreshness,
    },
    /// Owner-native evidence without copied native payload.
    NativeOwner {
        /// Exact typed subject.
        subject: ObservationSubject,
        /// Native observation classification.
        state: ObservationNativeState,
        /// Temporal qualification of the native evidence.
        freshness: ObservationFreshness,
    },
    /// Reconciliation custody or terminal settlement.
    Reconciliation {
        /// Exact typed subject.
        subject: ObservationSubject,
        /// Reconciliation classification.
        state: ObservationReconciliationState,
        /// Temporal qualification of the reconciliation evidence.
        freshness: ObservationFreshness,
    },
    /// Desired resident process state.
    ProcessIntent {
        /// Desired state; never observed liveness.
        state: ObservationProcessIntent,
        /// Temporal qualification of the durable desired-state projection.
        freshness: ObservationFreshness,
    },
}

impl From<&application::ObservationFact> for ObservationFact {
    fn from(value: &application::ObservationFact) -> Self {
        match value {
            application::ObservationFact::OperationExposure {
                subject,
                state,
                freshness,
            } => Self::OperationExposure {
                subject: subject.into(),
                state: (*state).into(),
                freshness: (*freshness).into(),
            },
            application::ObservationFact::CapabilityRequirementDisposition {
                subject,
                state,
                freshness,
            } => Self::CapabilityRequirementDisposition {
                subject: subject.into(),
                state: (*state).into(),
                freshness: (*freshness).into(),
            },
            application::ObservationFact::Health {
                subject,
                state,
                freshness,
            } => Self::Health {
                subject: subject.into(),
                state: (*state).into(),
                freshness: (*freshness).into(),
            },
            application::ObservationFact::Readiness {
                subject,
                state,
                freshness,
            } => Self::Readiness {
                subject: subject.into(),
                state: (*state).into(),
                freshness: (*freshness).into(),
            },
            application::ObservationFact::Effect {
                subject,
                certainty,
                freshness,
            } => Self::Effect {
                subject: subject.into(),
                certainty: DiagnosticEffectCertainty::from_wire(*certainty),
                freshness: (*freshness).into(),
            },
            application::ObservationFact::NativeOwner {
                subject,
                state,
                freshness,
            } => Self::NativeOwner {
                subject: subject.into(),
                state: state.into(),
                freshness: (*freshness).into(),
            },
            application::ObservationFact::Reconciliation {
                subject,
                state,
                freshness,
            } => Self::Reconciliation {
                subject: subject.into(),
                state: state.into(),
                freshness: (*freshness).into(),
            },
            application::ObservationFact::ProcessIntent { state, freshness } => {
                Self::ProcessIntent {
                    state: (*state).into(),
                    freshness: (*freshness).into(),
                }
            }
        }
    }
}

impl From<application::ObservationHealthState> for ObservationHealthState {
    fn from(value: application::ObservationHealthState) -> Self {
        match value {
            application::ObservationHealthState::Healthy => Self::Healthy,
            application::ObservationHealthState::Degraded => Self::Degraded,
            application::ObservationHealthState::Unhealthy => Self::Unhealthy,
            application::ObservationHealthState::Unknown => Self::Unknown,
            application::ObservationHealthState::Unavailable => Self::Unavailable,
        }
    }
}

impl From<application::ObservationReadinessState> for ObservationReadinessState {
    fn from(value: application::ObservationReadinessState) -> Self {
        match value {
            application::ObservationReadinessState::Ready => Self::Ready,
            application::ObservationReadinessState::Blocked => Self::Blocked,
            application::ObservationReadinessState::Narrowed => Self::Narrowed,
            application::ObservationReadinessState::Unproven => Self::Unproven,
            application::ObservationReadinessState::Unavailable => Self::Unavailable,
        }
    }
}

impl From<&application::ObservationNativeState> for ObservationNativeState {
    fn from(value: &application::ObservationNativeState) -> Self {
        match value {
            application::ObservationNativeState::Unavailable => Self::Unavailable,
            application::ObservationNativeState::Unknown => Self::Unknown,
            application::ObservationNativeState::Observed { owner, kind } => Self::Observed {
                owner_system_id: owner.as_str().to_owned(),
                observation_kind: kind.as_str().to_owned(),
            },
        }
    }
}

impl From<&application::ObservationReconciliationState> for ObservationReconciliationState {
    fn from(value: &application::ObservationReconciliationState) -> Self {
        match value {
            application::ObservationReconciliationState::NotRequired => Self::NotRequired,
            application::ObservationReconciliationState::Required(digest) => Self::Required {
                case_digest: digest.as_str().to_owned(),
            },
            application::ObservationReconciliationState::InProgress(digest) => Self::InProgress {
                case_digest: digest.as_str().to_owned(),
            },
            application::ObservationReconciliationState::Settled(digest) => Self::Settled {
                result_digest: digest.as_str().to_owned(),
            },
            application::ObservationReconciliationState::Unknown => Self::Unknown,
            application::ObservationReconciliationState::Unavailable => Self::Unavailable,
        }
    }
}

impl From<application::ObservationProcessIntent> for ObservationProcessIntent {
    fn from(value: application::ObservationProcessIntent) -> Self {
        match value {
            application::ObservationProcessIntent::Running => Self::Running,
            application::ObservationProcessIntent::Stopped => Self::Stopped,
        }
    }
}

/// Exact bounded filter constructed without parsing identities.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationFilter {
    category: Option<ObservationCategory>,
    subject: Option<ObservationSubject>,
    freshness: Option<ObservationFreshnessClass>,
    #[serde(skip)]
    application: application::ObservationFilter,
}

impl ObservationFilter {
    /// Select every fact.
    #[must_use]
    pub fn all() -> Self {
        Self {
            category: None,
            subject: None,
            freshness: None,
            application: application::ObservationFilter::all(),
        }
    }

    /// Restrict to one independent category.
    #[must_use]
    pub fn with_category(mut self, category: ObservationCategory) -> Self {
        self.application = self.application.with_category(category.into());
        self.category = Some(category);
        self
    }

    /// Restrict to one exact subject obtained from an admitted result.
    #[must_use]
    pub fn with_subject(mut self, subject: &ObservationSubject) -> Self {
        let application_subject = subject.to_application();
        self.application = self.application.with_subject(application_subject);
        self.subject = Some(subject.clone());
        self
    }

    /// Restrict to one freshness class.
    #[must_use]
    pub fn with_freshness(mut self, freshness: ObservationFreshnessClass) -> Self {
        self.application = self.application.with_freshness(freshness.into());
        self.freshness = Some(freshness);
        self
    }
}

impl ObservationSubject {
    fn to_application(&self) -> application::ObservationSubject {
        self.application.clone()
    }
}

/// Query request for one bounded snapshot page.
///
/// The limits bound one step, not the size of the snapshot. A returned
/// continuation carries the exact snapshot, filter, and raw limits required for
/// the next step.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationPageRequest {
    filter: ObservationFilter,
    maximum_inspected: u16,
    maximum_returned: u16,
    maximum_copied_string_bytes: u32,
    after: Option<ObservationPageCursor>,
    cancellation: ObservationCancellation,
}

/// Independent work and output limits for one observation-page step.
///
/// Zero asks the application to use its default. The application caps larger
/// values at its owned ceilings.
/// These values bound one request. A returned cursor binds the raw values to
/// the snapshot and filter. Reaching a limit does not prove that later facts
/// are absent.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationPageLimits {
    maximum_inspected: u16,
    maximum_returned: u16,
    maximum_copied_string_bytes: u32,
}

impl ObservationPageLimits {
    /// Capture the raw limits that one bounded page step must enforce.
    #[must_use]
    pub const fn new(
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

impl ObservationPageRequest {
    /// Construct an active first-page request with default work and copy bounds.
    #[must_use]
    pub const fn new(filter: ObservationFilter, maximum_returned: u16) -> Self {
        Self::bounded(
            filter,
            ObservationPageLimits::new(0, maximum_returned, 0),
            ObservationCancellation::Active,
        )
    }

    /// Construct a first-page request with independent raw bounds.
    #[must_use]
    pub const fn bounded(
        filter: ObservationFilter,
        limits: ObservationPageLimits,
        cancellation: ObservationCancellation,
    ) -> Self {
        Self {
            filter,
            maximum_inspected: limits.maximum_inspected,
            maximum_returned: limits.maximum_returned,
            maximum_copied_string_bytes: limits.maximum_copied_string_bytes,
            after: None,
            cancellation,
        }
    }

    /// Continue from a cursor issued by the same snapshot and filter.
    #[must_use]
    pub fn after(mut self, cursor: ObservationPageCursor) -> Self {
        self.after = Some(cursor);
        self
    }

    fn into_application(self) -> application::ObservationPageRequest {
        let mut request = application::ObservationPageRequest::bounded(
            self.filter.application,
            self.maximum_inspected,
            self.maximum_returned,
            self.maximum_copied_string_bytes,
            self.cancellation.into(),
        );
        if let Some(after) = self.after {
            request = request.after(after.application);
        }
        request
    }
}

/// Opaque continuation for one filtered snapshot page.
///
/// The application binds its exact snapshot, filter, raw limits, and next fact.
/// Pass it only to the same query. Its position does not prove that omitted or
/// later facts are absent.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationPageCursor {
    snapshot: ObservationCursor,
    inspected_position: u32,
    maximum_inspected: u16,
    maximum_returned: u16,
    maximum_copied_string_bytes: u32,
    #[serde(skip)]
    application: application::ObservationPageCursor,
}

impl ObservationPageCursor {
    fn from_application(value: application::ObservationPageCursor) -> Self {
        Self {
            snapshot: ObservationCursor::from_application(value.snapshot().clone()),
            inspected_position: value.inspected_position(),
            maximum_inspected: value.maximum_inspected(),
            maximum_returned: value.maximum_returned(),
            maximum_copied_string_bytes: value.maximum_copied_string_bytes(),
            application: value,
        }
    }

    /// Borrow the exact snapshot cursor.
    #[must_use]
    pub const fn snapshot(&self) -> &ObservationCursor {
        &self.snapshot
    }

    /// Return how many retained facts precede the next step.
    #[must_use]
    pub const fn inspected_position(&self) -> u32 {
        self.inspected_position
    }
}

/// Resource that bounded one observation step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ObservationStepLimit {
    /// Retained-input inspection bound.
    Inspected,
    /// Returned-item bound.
    Returned,
    /// Copied-string bound, including the minimum needed for the next item.
    CopiedStringBytes {
        /// Minimum bound that can admit the next indivisible result.
        minimum_required: u32,
    },
    /// Causal-path lookup bound.
    CausalPathLookups,
    /// Causal-step bound, including the minimum needed for the next path.
    CausalSteps {
        /// Minimum bound that can admit the next complete causal path.
        minimum_required: u16,
    },
}

impl From<application::ObservationStepLimit> for ObservationStepLimit {
    fn from(value: application::ObservationStepLimit) -> Self {
        match value {
            application::ObservationStepLimit::Inspected => Self::Inspected,
            application::ObservationStepLimit::Returned => Self::Returned,
            application::ObservationStepLimit::CopiedStringBytes { minimum_required } => {
                Self::CopiedStringBytes { minimum_required }
            }
            application::ObservationStepLimit::CausalPathLookups => Self::CausalPathLookups,
            application::ObservationStepLimit::CausalSteps { minimum_required } => {
                Self::CausalSteps { minimum_required }
            }
        }
    }
}

/// Whether a bounded step exhausted its selected input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "limit", rename_all = "kebab-case")]
pub enum ObservationStepCompletion {
    /// The selected input for this snapshot and filter was exhausted.
    Complete,
    /// More input can remain after an ordinary resource stop. Continue with the
    /// returned cursor; this state does not prove absence.
    Stopped(ObservationStepLimit),
    /// The next indivisible result does not fit the raw request limits. No
    /// continuation is available because the same limits cannot make progress;
    /// later input can still exist.
    Incomplete(ObservationStepLimit),
}

impl From<application::ObservationStepCompletion> for ObservationStepCompletion {
    fn from(value: application::ObservationStepCompletion) -> Self {
        match value {
            application::ObservationStepCompletion::Complete => Self::Complete,
            application::ObservationStepCompletion::Stopped(limit) => Self::Stopped(limit.into()),
            application::ObservationStepCompletion::Incomplete(limit) => {
                Self::Incomplete(limit.into())
            }
        }
    }
}

/// One bounded filtered observation page.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationPage {
    /// Exact snapshot cursor.
    pub cursor: ObservationCursor,
    /// Deterministically ordered facts.
    pub facts: Vec<ObservationFact>,
    /// Continuation when matching facts remain.
    pub continuation: Option<ObservationPageCursor>,
    /// Whether this step exhausted its selected input.
    pub completion: ObservationStepCompletion,
    /// Retained facts examined, including nonmatches.
    pub inspected: u16,
    /// Variable-length UTF-8 bytes copied into returned facts.
    pub copied_string_bytes: u32,
    /// Owner facts omitted when the bounded snapshot was constructed.
    pub snapshot_omitted_count: u32,
}

/// Result of one page step.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ObservationPageDisposition {
    /// One valid bounded step. Inspect its completion before treating the
    /// selected input as exhausted.
    Ready {
        /// Complete result of this bounded step.
        page: ObservationPage,
    },
    /// Cancellation was observed before this bounded step began.
    Cancelled {
        /// Unchanged continuation that the caller may retry, if one was supplied.
        continuation: Option<ObservationPageCursor>,
    },
}

/// Opaque source-free snapshot handle.
#[derive(Clone)]
pub struct ObservationSnapshot {
    application: Arc<application::ObservationSnapshot>,
}

impl std::fmt::Debug for ObservationSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ObservationSnapshot")
            .field("cursor", &self.cursor())
            .field("fact_count", &self.application.fact_count())
            .finish_non_exhaustive()
    }
}

impl ObservationSnapshot {
    /// Return the exact complete-snapshot cursor.
    #[must_use]
    pub fn cursor(&self) -> ObservationCursor {
        ObservationCursor::from_application(self.application.cursor().clone())
    }

    /// Identify this complete snapshot for subsequent delta requests.
    ///
    /// The caller records this position only after applying the snapshot. Merely
    /// obtaining the handle does not acknowledge delivery or start a stream.
    #[must_use]
    pub fn resume_coordinate(&self) -> ObservationResumeCoordinate {
        ObservationResumeCoordinate::from_application(self.application.resume_coordinate())
    }

    /// Return bounded snapshot cardinality and coordinates.
    #[must_use]
    pub fn summary(&self) -> ObservationSnapshotSummary {
        ObservationSnapshotSummary {
            cursor: self.cursor(),
            fact_count: u32::try_from(self.application.fact_count()).unwrap_or(u32::MAX),
            omitted_fact_count: self.application.omitted_fact_count(),
        }
    }

    /// Query one bounded page from this exact handle.
    ///
    /// # Errors
    ///
    /// Returns [`ObservationResynchronization`] when the continuation belongs
    /// to another snapshot coordinate or filter.
    pub fn page(
        &self,
        request: ObservationPageRequest,
    ) -> Result<ObservationPageDisposition, ObservationResynchronization> {
        self.application
            .page(request.into_application())
            .map(page_disposition_from_application)
            .map_err(ObservationResynchronization::from_application)
    }

    /// Compare `prior` with this snapshot and return changes or a recovery instruction.
    ///
    /// Use the resume position for the snapshot already applied by the client,
    /// not a [`ObservationPageCursor`] from a partially read page sequence.
    /// [`ObservationDeltaOutcome::Delta`] contains the complete ordered update:
    /// apply all changes and the successor omitted-fact count before replacing
    /// the client's cursor with [`ObservationDelta::to`].
    ///
    /// `CaughtUp` requires no changes. `Resynchronize` and `Backpressured`
    /// require a fresh snapshot, not a partial delta. `Cancelled` preserves the
    /// caller's resume position; it reports cancellation checked before the
    /// comparison, not interruption of work already running. This call compares
    /// retained data and does not subscribe to resident or provider events.
    #[must_use]
    pub fn delta_from(
        &self,
        prior: &Self,
        request: ObservationDeltaRequest,
    ) -> ObservationDeltaOutcome {
        self.application
            .delta_from(&prior.application, request.into_application())
            .into()
    }

    /// Return one retained exact causal path for a subject from this snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`ObservationResynchronization`] when the selector belongs to a
    /// different admitted generation.
    pub fn causal_path(
        &self,
        selector: &ObservationCausalSelector,
    ) -> Result<Option<ObservationCausalPath>, ObservationResynchronization> {
        self.application
            .causal_path(&selector.application)
            .map(|path| path.as_ref().map(ObservationCausalPath::from_application))
            .map_err(ObservationResynchronization::from_application)
    }

    /// Build one bounded non-authoritative lens for native client rendering.
    #[must_use]
    pub fn visual_lens(&self, request: ObservationLensRequest) -> ObservationLensDisposition {
        self.application
            .visual_lens(request.into_application())
            .into()
    }
}

impl PaleApi {
    /// Project one admitted effective-profile handle into observation state.
    ///
    /// Native machine/provider evidence remains `unknown` or `unavailable`.
    /// This method reads no filesystem, resident store, provider, or native body.
    #[must_use]
    pub fn observe_effective_profile(
        &self,
        compilation: &EffectiveProfileCompilation,
    ) -> ObservationSnapshot {
        let _ = self;
        ObservationSnapshot {
            application: Arc::new(application::ObservationSnapshot::from_effective_profile(
                &compilation.compilation,
            )),
        }
    }
}

/// Cardinality and exact coordinates of one bounded snapshot.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationSnapshotSummary {
    /// Complete-snapshot cursor.
    pub cursor: ObservationCursor,
    /// Number of retained facts.
    pub fact_count: u32,
    /// Number of owner facts omitted by the bounded snapshot.
    pub omitted_fact_count: u32,
}

/// Cooperative request state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationCancellation {
    /// Work may proceed.
    Active,
    /// The caller cancelled before bounded projection began.
    Cancelled,
}

impl From<ObservationCancellation> for application::ObservationCancellation {
    fn from(value: ObservationCancellation) -> Self {
        match value {
            ObservationCancellation::Active => Self::Active,
            ObservationCancellation::Cancelled => Self::Cancelled,
        }
    }
}

/// Request for one complete ordered delta.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationDeltaRequest {
    resume: ObservationResumeCoordinate,
    maximum_changes: u16,
    cancellation: ObservationCancellation,
}

impl ObservationDeltaRequest {
    /// Bind a caller-held resume coordinate and atomic change bound.
    #[must_use]
    pub const fn new(
        resume: ObservationResumeCoordinate,
        maximum_changes: u16,
        cancellation: ObservationCancellation,
    ) -> Self {
        Self {
            resume,
            maximum_changes,
            cancellation,
        }
    }

    fn into_application(self) -> application::ObservationDeltaRequest {
        application::ObservationDeltaRequest {
            resume: self.resume.application,
            maximum_changes: self.maximum_changes,
            cancellation: self.cancellation.into(),
        }
    }
}

/// One deterministic fact change.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ObservationChange {
    /// Insert or replace one independently typed fact.
    Upsert {
        /// Complete successor fact.
        fact: ObservationFact,
    },
    /// Remove one fact key that is absent from the successor.
    Remove {
        /// Removed fact category.
        category: ObservationCategory,
        /// Removed exact subject.
        subject: ObservationSubject,
    },
}

/// One complete adjacent delta.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationDelta {
    /// Cursor the client must already have applied.
    pub from: ObservationCursor,
    /// Cursor installed after every change is applied.
    pub to: ObservationCursor,
    /// Deterministically ordered complete changes.
    pub changes: Vec<ObservationChange>,
    /// Omitted-fact count installed with the successor snapshot.
    pub successor_omitted_fact_count: u32,
}

/// Safe recovery after slow-consumer backpressure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationBackpressureRecovery {
    /// Request a fresh bounded snapshot; no partial delta was emitted.
    RequestSnapshot,
}

/// Result of one delta request.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ObservationDeltaOutcome {
    /// The caller already names the current cursor.
    CaughtUp {
        /// Current observation cursor.
        cursor: ObservationCursor,
    },
    /// One complete adjacent delta is safe to apply atomically.
    Delta {
        /// Complete ordered delta.
        delta: ObservationDelta,
    },
    /// A gap or generation mismatch forbids delta application.
    Resynchronize {
        /// Exact full-snapshot recovery instruction.
        resynchronization: ObservationResynchronization,
    },
    /// Cancellation was observed before projection.
    Cancelled {
        /// Caller-held position that remains completely applied.
        resume: ObservationResumeCoordinate,
    },
    /// The complete adjacent delta exceeds the consumer's declared bound.
    Backpressured {
        /// Number of changes required for atomic application.
        pending_changes: u32,
        /// Effective bounded maximum accepted by the consumer.
        maximum_changes: u16,
        /// Safe recovery that emits no partial delta.
        recovery: ObservationBackpressureRecovery,
        /// Current server cursor available through a new snapshot.
        current: ObservationCursor,
    },
}

impl From<application::ObservationDeltaOutcome> for ObservationDeltaOutcome {
    fn from(value: application::ObservationDeltaOutcome) -> Self {
        match value {
            application::ObservationDeltaOutcome::CaughtUp(cursor) => Self::CaughtUp {
                cursor: ObservationCursor::from_application(cursor),
            },
            application::ObservationDeltaOutcome::Delta(delta) => Self::Delta {
                delta: ObservationDelta {
                    from: ObservationCursor::from_application(delta.from),
                    to: ObservationCursor::from_application(delta.to),
                    changes: delta
                        .changes
                        .into_iter()
                        .map(change_from_application)
                        .collect(),
                    successor_omitted_fact_count: delta.successor_omitted_fact_count,
                },
            },
            application::ObservationDeltaOutcome::Resynchronize(value) => Self::Resynchronize {
                resynchronization: ObservationResynchronization::from_application(value),
            },
            application::ObservationDeltaOutcome::Cancelled(resume) => Self::Cancelled {
                resume: ObservationResumeCoordinate::from_application(resume),
            },
            application::ObservationDeltaOutcome::Backpressured {
                pending_changes,
                maximum_changes,
                recovery,
                current,
            } => Self::Backpressured {
                pending_changes,
                maximum_changes,
                recovery: match recovery {
                    application::ObservationBackpressureRecovery::RequestSnapshot => {
                        ObservationBackpressureRecovery::RequestSnapshot
                    }
                },
                current: ObservationCursor::from_application(current),
            },
        }
    }
}

fn change_from_application(value: application::ObservationChange) -> ObservationChange {
    match value {
        application::ObservationChange::Upsert(fact) => ObservationChange::Upsert {
            fact: (&fact).into(),
        },
        application::ObservationChange::Remove { category, subject } => ObservationChange::Remove {
            category: category.into(),
            subject: (&subject).into(),
        },
    }
}

/// Exact reason a full snapshot is mandatory.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ObservationResynchronizationReason {
    /// The caller-held generation differs from the current generation.
    GenerationMismatch {
        /// Caller-held generation.
        requested: Box<ObservationGeneration>,
        /// Current generation.
        current: Box<ObservationGeneration>,
    },
    /// One or more source revisions are absent from bounded delivery.
    RevisionGap {
        /// Caller-held revision.
        requested: ObservationRevision,
        /// Current source revision.
        current: ObservationRevision,
    },
    /// A page or lens continuation belongs to another query.
    ContinuationMismatch,
}

/// Full-snapshot recovery bound to the current coordinate.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationResynchronization {
    /// Exact reason no delta or page may be applied.
    pub reason: ObservationResynchronizationReason,
    /// Current cursor from which a new snapshot must be requested.
    pub current: Box<ObservationCursor>,
}

impl ObservationResynchronization {
    fn from_application(value: application::ObservationResynchronization) -> Self {
        Self {
            reason: match value.reason {
                application::ObservationResynchronizationReason::GenerationMismatch {
                    requested,
                    current,
                } => ObservationResynchronizationReason::GenerationMismatch {
                    requested: Box::new(requested.as_ref().into()),
                    current: Box::new(current.as_ref().into()),
                },
                application::ObservationResynchronizationReason::RevisionGap {
                    requested,
                    current,
                } => ObservationResynchronizationReason::RevisionGap {
                    requested: (&requested).into(),
                    current: (&current).into(),
                },
                application::ObservationResynchronizationReason::ContinuationMismatch => {
                    ObservationResynchronizationReason::ContinuationMismatch
                }
            },
            current: Box::new(ObservationCursor::from_application(*value.current)),
        }
    }
}

/// Selector for one retained exact causal path.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationCausalSelector {
    generation: ObservationGeneration,
    subject: ObservationSubject,
    #[serde(skip)]
    application: application::ObservationCausalSelector,
}

impl ObservationCausalSelector {
    /// Bind a subject to the exact snapshot generation that exposed it.
    #[must_use]
    pub fn new(snapshot: &ObservationSnapshot, subject: &ObservationSubject) -> Self {
        let generation = snapshot.application.cursor().generation().clone();
        Self {
            generation: (&generation).into(),
            subject: subject.clone(),
            application: application::ObservationCausalSelector {
                generation,
                subject: subject.to_application(),
            },
        }
    }
}

/// One exact application-owned causal reason.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ObservationCausalStep {
    /// Origin-receipt proof step.
    OriginReceipt {
        /// Referenced contract identity.
        contract_id: String,
    },
    /// Entry-contract proof step.
    EntryContract {
        /// Referenced contract identity.
        contract_id: String,
    },
    /// Responsibility-contract proof step.
    ResponsibilityContract {
        /// Referenced contract identity.
        contract_id: String,
    },
    /// Provider-binding proof step.
    ProviderBinding {
        /// Referenced private binding identity.
        binding_id: String,
    },
    /// Delegation-edge proof step.
    DelegationEdge {
        /// Referenced delegation edge identity.
        edge_id: String,
    },
    /// Termination-contract proof step.
    TerminationContract {
        /// Referenced contract identity.
        contract_id: String,
    },
    /// Settlement-contract proof step.
    SettlementContract {
        /// Referenced contract identity.
        contract_id: String,
    },
    /// Continuity-contract proof step.
    ContinuityContract {
        /// Referenced contract identity.
        contract_id: String,
    },
}

/// Bounded exact causal proof.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationCausalPath {
    /// Exact generation in which the proof is meaningful.
    pub generation: ObservationGeneration,
    /// Exact subject selected from the admitted snapshot.
    pub subject: ObservationSubject,
    /// Ordered compiler-owned proof steps.
    pub steps: Vec<ObservationCausalStep>,
}

impl ObservationCausalPath {
    fn from_application(value: &application::ObservationCausalPath) -> Self {
        Self {
            generation: (&value.selector.generation).into(),
            subject: (&value.selector.subject).into(),
            steps: value
                .steps
                .iter()
                .map(causal_step_from_application)
                .collect(),
        }
    }
}

fn causal_step_from_application(
    value: &application::ObservationCausalStep,
) -> ObservationCausalStep {
    let application::ObservationCausalStep::Composition(value) = value;
    match value {
        pale_core::records::CompositionReasonStepV1::OriginReceipt { contract_id } => {
            ObservationCausalStep::OriginReceipt {
                contract_id: contract_id.clone(),
            }
        }
        pale_core::records::CompositionReasonStepV1::EntryContract { contract_id } => {
            ObservationCausalStep::EntryContract {
                contract_id: contract_id.clone(),
            }
        }
        pale_core::records::CompositionReasonStepV1::ResponsibilityContract { contract_id } => {
            ObservationCausalStep::ResponsibilityContract {
                contract_id: contract_id.clone(),
            }
        }
        pale_core::records::CompositionReasonStepV1::ProviderBinding { binding_id } => {
            ObservationCausalStep::ProviderBinding {
                binding_id: binding_id.clone(),
            }
        }
        pale_core::records::CompositionReasonStepV1::DelegationEdge { edge_id } => {
            ObservationCausalStep::DelegationEdge {
                edge_id: edge_id.clone(),
            }
        }
        pale_core::records::CompositionReasonStepV1::TerminationContract { contract_id } => {
            ObservationCausalStep::TerminationContract {
                contract_id: contract_id.clone(),
            }
        }
        pale_core::records::CompositionReasonStepV1::SettlementContract { contract_id } => {
            ObservationCausalStep::SettlementContract {
                contract_id: contract_id.clone(),
            }
        }
        pale_core::records::CompositionReasonStepV1::ContinuityContract { contract_id } => {
            ObservationCausalStep::ContinuityContract {
                contract_id: contract_id.clone(),
            }
        }
    }
}

/// Request for one bounded disposable observation lens step.
///
/// The request selects typed facts and optional retained causal paths. It does
/// not request layout, infer relationships, or prove that omitted subjects are
/// absent.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationLensRequest {
    filter: ObservationFilter,
    maximum_inspected: u16,
    maximum_nodes: u16,
    maximum_copied_string_bytes: u32,
    include_causal_paths: bool,
    maximum_causal_path_lookups: u16,
    maximum_causal_steps: u16,
    after: Option<ObservationLensCursor>,
    cancellation: ObservationCancellation,
}

/// Independent work and copy limits for one observation-lens step.
///
/// These values bound work within one request. The issued cursor binds them to
/// the snapshot and filter. Reaching a limit does not prove that later subjects
/// are absent.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationLensLimits {
    maximum_inspected: u16,
    maximum_nodes: u16,
    maximum_copied_string_bytes: u32,
    maximum_causal_path_lookups: u16,
    maximum_causal_steps: u16,
}

impl ObservationLensLimits {
    /// Capture the raw limits that one bounded step must enforce.
    #[must_use]
    pub const fn new(
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

impl ObservationLensRequest {
    /// Construct an active lens with default work, copy, and causal bounds.
    #[must_use]
    pub const fn new(
        filter: ObservationFilter,
        maximum_nodes: u16,
        include_causal_paths: bool,
        cancellation: ObservationCancellation,
    ) -> Self {
        Self::bounded(
            filter,
            ObservationLensLimits::new(0, maximum_nodes, 0, 0, 0),
            include_causal_paths,
            cancellation,
        )
    }

    /// Construct a lens request with independent raw bounds.
    #[must_use]
    pub const fn bounded(
        filter: ObservationFilter,
        limits: ObservationLensLimits,
        include_causal_paths: bool,
        cancellation: ObservationCancellation,
    ) -> Self {
        Self {
            filter,
            maximum_inspected: limits.maximum_inspected,
            maximum_nodes: limits.maximum_nodes,
            maximum_copied_string_bytes: limits.maximum_copied_string_bytes,
            include_causal_paths,
            maximum_causal_path_lookups: limits.maximum_causal_path_lookups,
            maximum_causal_steps: limits.maximum_causal_steps,
            after: None,
            cancellation,
        }
    }

    /// Continue from a cursor issued by the same snapshot and raw limits.
    #[must_use]
    pub fn after(mut self, cursor: ObservationLensCursor) -> Self {
        self.after = Some(cursor);
        self
    }

    fn into_application(self) -> application::ObservationLensRequest {
        let mut request = application::ObservationLensRequest {
            filter: self.filter.application,
            maximum_inspected: self.maximum_inspected,
            maximum_nodes: self.maximum_nodes,
            maximum_copied_string_bytes: self.maximum_copied_string_bytes,
            include_causal_paths: self.include_causal_paths,
            maximum_causal_path_lookups: self.maximum_causal_path_lookups,
            maximum_causal_steps: self.maximum_causal_steps,
            after: None,
            cancellation: self.cancellation.into(),
        };
        if let Some(after) = self.after {
            request = request.after(after.application);
        }
        request
    }
}

/// Opaque continuation for one bounded lens query.
///
/// The application binds its exact snapshot, filter, causal-path choice, raw
/// limits, and next subject. Pass it only to the same query. Its position does
/// not prove that omitted or later subjects are absent.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationLensCursor {
    snapshot: ObservationCursor,
    inspected_position: u32,
    maximum_inspected: u16,
    maximum_nodes: u16,
    maximum_copied_string_bytes: u32,
    include_causal_paths: bool,
    maximum_causal_path_lookups: u16,
    maximum_causal_steps: u16,
    #[serde(skip)]
    application: application::ObservationLensCursor,
}

impl ObservationLensCursor {
    fn from_application(value: application::ObservationLensCursor) -> Self {
        Self {
            snapshot: ObservationCursor::from_application(value.snapshot().clone()),
            inspected_position: value.inspected_position(),
            maximum_inspected: value.maximum_inspected(),
            maximum_nodes: value.maximum_nodes(),
            maximum_copied_string_bytes: value.maximum_copied_string_bytes(),
            include_causal_paths: value.includes_causal_paths(),
            maximum_causal_path_lookups: value.maximum_causal_path_lookups(),
            maximum_causal_steps: value.maximum_causal_steps(),
            application: value,
        }
    }

    /// Borrow the exact snapshot cursor.
    #[must_use]
    pub const fn snapshot(&self) -> &ObservationCursor {
        &self.snapshot
    }

    /// Return how many retained subjects precede the next step.
    #[must_use]
    pub const fn inspected_position(&self) -> u32 {
        self.inspected_position
    }
}

/// One exact subject node and its complete selected facts.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationLensNode {
    /// Exact admitted observation subject.
    pub subject: ObservationSubject,
    /// Complete typed facts selected for the subject.
    pub facts: Vec<ObservationFact>,
}

/// Bounded snapshot-bound lens with no authoritative geometry.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationLens {
    /// Exact snapshot generation and revision used by all lens data.
    pub cursor: ObservationCursor,
    /// Deterministically ordered, deduplicated subject nodes.
    pub nodes: Vec<ObservationLensNode>,
    /// Retained exact causal paths, when requested.
    pub causal_paths: Vec<ObservationCausalPath>,
    /// Continuation when selected subjects may remain.
    pub continuation: Option<ObservationLensCursor>,
    /// Whether this step exhausted its selected input.
    pub completion: ObservationStepCompletion,
    /// Retained subjects examined, including nonmatches.
    pub inspected_subjects: u16,
    /// Variable-length UTF-8 bytes copied into this result.
    pub copied_string_bytes: u32,
    /// Causal-path lookups performed by this step.
    ///
    /// A lookup is charged before its path passes the causal-step and copied-byte
    /// limits, so the count can include an unreturned subject.
    pub causal_path_lookups: u16,
    /// Causal steps copied into this result.
    pub causal_steps: u16,
    /// Owner facts omitted when the bounded snapshot was constructed.
    pub snapshot_omitted_count: u32,
}

/// Cooperative lens result.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ObservationLensDisposition {
    /// One valid bounded lens step. Inspect its completion before treating the
    /// selected input as exhausted.
    Ready {
        /// Generation-bound lens result.
        lens: ObservationLens,
    },
    /// The continuation belongs to another snapshot, filter, or raw limits.
    /// Obtain a fresh snapshot; this outcome makes no claim about subject state.
    Resynchronize {
        /// Exact full-snapshot recovery instruction.
        resynchronization: ObservationResynchronization,
    },
    /// Caller cancellation was observed before this bounded step began.
    Cancelled {
        /// Unchanged continuation that the caller may retry, if one was supplied.
        continuation: Option<ObservationLensCursor>,
    },
}

impl From<application::ObservationLensDisposition> for ObservationLensDisposition {
    fn from(value: application::ObservationLensDisposition) -> Self {
        match value {
            application::ObservationLensDisposition::Cancelled { continuation } => {
                Self::Cancelled {
                    continuation: continuation.map(ObservationLensCursor::from_application),
                }
            }
            application::ObservationLensDisposition::Resynchronize(value) => Self::Resynchronize {
                resynchronization: ObservationResynchronization::from_application(value),
            },
            application::ObservationLensDisposition::Ready(lens) => Self::Ready {
                lens: ObservationLens {
                    cursor: ObservationCursor::from_application(lens.cursor),
                    nodes: lens
                        .nodes
                        .into_iter()
                        .map(|node| ObservationLensNode {
                            subject: (&node.subject).into(),
                            facts: node.facts.iter().map(Into::into).collect(),
                        })
                        .collect(),
                    causal_paths: lens
                        .causal_paths
                        .iter()
                        .map(ObservationCausalPath::from_application)
                        .collect(),
                    continuation: lens
                        .continuation
                        .map(ObservationLensCursor::from_application),
                    completion: lens.completion.into(),
                    inspected_subjects: lens.inspected_subjects,
                    copied_string_bytes: lens.copied_string_bytes,
                    causal_path_lookups: lens.causal_path_lookups,
                    causal_steps: lens.causal_steps,
                    snapshot_omitted_count: lens.snapshot_omitted_count,
                },
            },
        }
    }
}

fn page_from_application(value: application::ObservationPage) -> ObservationPage {
    ObservationPage {
        cursor: ObservationCursor::from_application(value.cursor),
        facts: value.facts.iter().map(Into::into).collect(),
        continuation: value
            .continuation
            .map(ObservationPageCursor::from_application),
        completion: value.completion.into(),
        inspected: value.inspected,
        copied_string_bytes: value.copied_string_bytes,
        snapshot_omitted_count: value.snapshot_omitted_count,
    }
}

fn page_disposition_from_application(
    value: application::ObservationPageDisposition,
) -> ObservationPageDisposition {
    match value {
        application::ObservationPageDisposition::Ready(page) => ObservationPageDisposition::Ready {
            page: page_from_application(page),
        },
        application::ObservationPageDisposition::Cancelled { continuation } => {
            ObservationPageDisposition::Cancelled {
                continuation: continuation.map(ObservationPageCursor::from_application),
            }
        }
    }
}
