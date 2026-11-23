//! Closed coordinates and result carriers for [`super::ObservationSnapshot`].

use pale_core::records::{
    CapabilityIdV1, CapabilityRequirementIdV1, CompiledGenerationDigest, CompositionReasonStepV1,
    DiagnosticEffectCertaintyV1, ExecutionPathSelectionIdV1, ExperienceCompositionPlanDigest,
    InstanceDigest, JournalSequenceV1, LogicalRepositoryIdV1, ObservationKindV1, OwnerSystemIdV1,
    PackageResourceRefV1, ProfileDigest, ReconciliationCaseDigest, ReconciliationResultDigest,
};
use pale_ports::AuthorityStoreCoordinate;

use crate::compile::{EffectiveProfileOperationExposure, EffectiveProfileRequirementDisposition};

/// Default number of facts returned when a page request supplies zero.
pub const DEFAULT_OBSERVATION_PAGE_LIMIT: u16 = 64;
/// Maximum number of facts returned by one page.
pub const MAX_OBSERVATION_PAGE_LIMIT: u16 = 256;
/// Default number of retained facts or subjects inspected by one query step.
pub const DEFAULT_OBSERVATION_INSPECTION_LIMIT: u16 = 64;
/// Maximum number of retained facts or subjects inspected by one query step.
pub const MAX_OBSERVATION_INSPECTION_LIMIT: u16 = 1_024;
/// Default variable-length UTF-8 bytes copied by one query step.
pub const DEFAULT_OBSERVATION_COPIED_STRING_BYTES: u32 = 64 * 1_024;
/// Maximum variable-length UTF-8 bytes copied by one query step.
pub const MAX_OBSERVATION_COPIED_STRING_BYTES: u32 = 4 * 1_024 * 1_024;
/// Maximum number of facts retained in one observation snapshot.
pub const MAX_OBSERVATION_FACTS: usize = 4_096;
/// Maximum number of changes emitted in one contiguous delta.
pub const MAX_OBSERVATION_DELTA_ITEMS: u16 = 256;
/// Maximum number of nodes returned by one disposable lens.
pub const MAX_OBSERVATION_LENS_NODES: u16 = 512;
/// Default causal-path lookups performed by one lens step.
pub const DEFAULT_OBSERVATION_CAUSAL_PATH_LOOKUPS: u16 = 64;
/// Maximum causal-path lookups performed by one lens step.
pub const MAX_OBSERVATION_CAUSAL_PATH_LOOKUPS: u16 = 256;
/// Default causal steps copied by one lens step.
pub const DEFAULT_OBSERVATION_CAUSAL_STEPS: u16 = 256;
/// Maximum causal steps copied by one lens step.
pub const MAX_OBSERVATION_CAUSAL_STEPS: u16 = 4_096;
/// Maximum causal paths retained by one snapshot.
pub(super) const MAX_OBSERVATION_CAUSAL_PATHS: usize = 256;

/// Exact admitted generation namespace of one observation snapshot.
///
/// Profile and resident coordinates remain nominally separate. A profile
/// digest is not accepted where a compiled generation is required, and a
/// resident authority with no active generation remains explicitly unactivated.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservationGeneration {
    /// Immutable effective-profile compiler phase.
    EffectiveProfile {
        /// Canonical portable profile identity.
        profile_digest: ProfileDigest,
        /// Exact machine-resolved instance projected into observed facts.
        instance_digest: InstanceDigest,
        /// Composition proof used by the retained causal paths.
        composition_plan_digest: ExperienceCompositionPlanDigest,
    },
    /// Resident authority at one exact store and optional active generation.
    Resident {
        /// Durable store incarnation that owns the observed journal frontier.
        authority_store: AuthorityStoreCoordinate,
        /// Generation selected for new sessions, when activation has occurred.
        active_generation: Option<CompiledGenerationDigest>,
    },
}

/// Revision domain within one [`ObservationGeneration`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationRevision {
    /// Immutable effective-profile state has no journal successor coordinate.
    Static,
    /// Resident authority at an exact optional durable journal frontier.
    ResidentJournal(Option<JournalSequenceV1>),
}

/// Exact generation and revision consumed by snapshot and delta operations.
///
/// This coordinate is not an execution identity, owner receipt, Genealogy
/// edge, protocol request ID, or visualization node ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationCursor {
    pub(super) generation: ObservationGeneration,
    pub(super) revision: ObservationRevision,
}

impl ObservationCursor {
    /// Borrow the independently typed admitted generation coordinate.
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

/// Caller-held position from which ordered delta delivery may resume.
///
/// The distinct type prevents a page continuation from becoming stream state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationResumeCoordinate(pub(super) ObservationCursor);

impl ObservationResumeCoordinate {
    /// Borrow the last completely applied observation cursor.
    #[must_use]
    pub const fn cursor(&self) -> &ObservationCursor {
        &self.0
    }
}

/// Continuation for one filtered snapshot query.
///
/// The cursor binds the exact snapshot, filter, raw limits, and next retained
/// fact. It can continue only that query. Its position does not prove that
/// omitted or later facts are absent, and it never substitutes for
/// [`ObservationResumeCoordinate`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationPageCursor {
    pub(super) snapshot: ObservationCursor,
    pub(super) filter: ObservationFilter,
    pub(super) after_key: Option<ObservationFactKey>,
    pub(super) inspected_position: u32,
    pub(super) maximum_inspected: u16,
    pub(super) maximum_returned: u16,
    pub(super) maximum_copied_string_bytes: u32,
}

impl ObservationPageCursor {
    /// Borrow the exact snapshot coordinate that issued this continuation.
    #[must_use]
    pub const fn snapshot(&self) -> &ObservationCursor {
        &self.snapshot
    }

    /// Borrow the filter bound into this continuation.
    #[must_use]
    pub const fn filter(&self) -> &ObservationFilter {
        &self.filter
    }

    /// Return how many retained facts precede the next bounded step.
    #[must_use]
    pub const fn inspected_position(&self) -> u32 {
        self.inspected_position
    }

    /// Return the raw inspection bound carried by this continuation.
    #[must_use]
    pub const fn maximum_inspected(&self) -> u16 {
        self.maximum_inspected
    }

    /// Return the raw result bound carried by this continuation.
    #[must_use]
    pub const fn maximum_returned(&self) -> u16 {
        self.maximum_returned
    }

    /// Return the raw copied-string bound carried by this continuation.
    #[must_use]
    pub const fn maximum_copied_string_bytes(&self) -> u32 {
        self.maximum_copied_string_bytes
    }
}

/// Independent observation axes available for filtering and presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservationCategory {
    /// Exact three-state operation exposure.
    OperationExposure,
    /// Portable capability-requirement disposition.
    CapabilityRequirementDisposition,
    /// Bounded health evidence.
    Health,
    /// Admission or operational-readiness evidence.
    Readiness,
    /// External-effect certainty.
    Effect,
    /// Native-owner observation state.
    NativeOwner,
    /// Reconciliation custody and settlement state.
    Reconciliation,
    /// Desired resident process state, not observed liveness.
    ProcessIntent,
}

/// Exact typed subject of one observation fact.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservationSubject {
    /// Complete effective profile.
    EffectiveProfile(ProfileDigest),
    /// Selected private machine identity.
    Machine(String),
    /// One logical repository.
    Repository(LogicalRepositoryIdV1),
    /// One owner-qualified operation in a repository.
    Operation {
        /// Owning logical repository.
        repository_id: LogicalRepositoryIdV1,
        /// Package-owned operation identity.
        operation_id: PackageResourceRefV1,
    },
    /// One Package-owned capability requirement occurrence.
    CapabilityRequirement {
        /// Owning logical repository.
        repository_id: LogicalRepositoryIdV1,
        /// Package-owned operation identity.
        operation_id: PackageResourceRefV1,
        /// Operation-local requirement identity.
        requirement_id: CapabilityRequirementIdV1,
        /// Required portable capability.
        capability_id: CapabilityIdV1,
    },
    /// One selected execution path.
    ExecutionPath(ExecutionPathSelectionIdV1),
    /// One immutable compiled generation.
    CompiledGeneration(CompiledGenerationDigest),
    /// Complete resident process intent.
    ResidentProcess,
}

/// Temporal qualification of one observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationFreshness {
    /// Evidence is current for the exact snapshot generation and revision.
    Fresh,
    /// Evidence was current at an older revision and remains displayable only as stale.
    Stale {
        /// Number of source revisions by which the evidence trails.
        revisions_behind: u64,
    },
    /// No owner evidence establishes temporal currency.
    Unknown,
}

/// Health state that does not imply operation admission or effect success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationHealthState {
    /// Owner evidence satisfies the applicable bounded health predicate.
    Healthy,
    /// Owner evidence reports reduced but usable operation.
    Degraded,
    /// Owner evidence reports an unusable health predicate.
    Unhealthy,
    /// The selected owner has not supplied qualifying health evidence.
    Unknown,
    /// No admitted carrier can currently obtain this health class.
    Unavailable,
}

/// Readiness state independent of health, native observation, and effect certainty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationReadinessState {
    /// The applicable readiness predicate is satisfied by admitted evidence.
    Ready,
    /// An admitted policy ceiling rejects the subject.
    Blocked,
    /// A positive admitted ceiling omitted the subject.
    Narrowed,
    /// Portable configuration is coherent but owner-native evidence is absent.
    Unproven,
    /// A required binding or carrier is absent.
    Unavailable,
}

/// Native-owner evidence state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationNativeState {
    /// No selected native observation carrier exists.
    Unavailable,
    /// A native owner is selected, but no qualifying observation was admitted.
    Unknown,
    /// Exact owner-native evidence was admitted without copying its body.
    Observed {
        /// Native system retaining source truth.
        owner: OwnerSystemIdV1,
        /// Owner-defined observation kind.
        kind: ObservationKindV1,
    },
}

/// Reconciliation state independent of effect certainty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationReconciliationState {
    /// No effect uncertainty created a reconciliation obligation.
    NotRequired,
    /// An exact case retains unresolved custody.
    Required(ReconciliationCaseDigest),
    /// An exact case is being reconciled without a terminal result.
    InProgress(ReconciliationCaseDigest),
    /// An exact terminal reconciliation result was admitted.
    Settled(ReconciliationResultDigest),
    /// A relevant owner exists, but its reconciliation frontier is unobserved.
    Unknown,
    /// No admitted reconciliation carrier exists.
    Unavailable,
}

/// Desired process state, explicitly not an observation of process liveness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationProcessIntent {
    /// All admitted services are desired running.
    Running,
    /// All admitted services are desired stopped.
    Stopped,
}

/// One independently typed fact in an observation snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
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
        /// Exact subject.
        subject: ObservationSubject,
        /// Health classification only.
        state: ObservationHealthState,
        /// Temporal qualification of the health evidence.
        freshness: ObservationFreshness,
    },
    /// Admission or operational readiness.
    Readiness {
        /// Exact subject.
        subject: ObservationSubject,
        /// Readiness classification only.
        state: ObservationReadinessState,
        /// Temporal qualification of the readiness evidence.
        freshness: ObservationFreshness,
    },
    /// External-effect certainty using the existing diagnostic algebra.
    Effect {
        /// Exact subject.
        subject: ObservationSubject,
        /// What admitted evidence proves about effect execution.
        certainty: DiagnosticEffectCertaintyV1,
        /// Temporal qualification of the effect evidence.
        freshness: ObservationFreshness,
    },
    /// Owner-native evidence without copied native payload.
    NativeOwner {
        /// Exact subject.
        subject: ObservationSubject,
        /// Native observation classification.
        state: ObservationNativeState,
        /// Temporal qualification of the native evidence.
        freshness: ObservationFreshness,
    },
    /// Reconciliation custody or terminal settlement.
    Reconciliation {
        /// Exact subject.
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

impl ObservationFact {
    /// Return the independent fact category.
    #[must_use]
    pub const fn category(&self) -> ObservationCategory {
        match self {
            Self::OperationExposure { .. } => ObservationCategory::OperationExposure,
            Self::CapabilityRequirementDisposition { .. } => {
                ObservationCategory::CapabilityRequirementDisposition
            }
            Self::Health { .. } => ObservationCategory::Health,
            Self::Readiness { .. } => ObservationCategory::Readiness,
            Self::Effect { .. } => ObservationCategory::Effect,
            Self::NativeOwner { .. } => ObservationCategory::NativeOwner,
            Self::Reconciliation { .. } => ObservationCategory::Reconciliation,
            Self::ProcessIntent { .. } => ObservationCategory::ProcessIntent,
        }
    }

    /// Borrow the exact subject, when this category has one.
    #[must_use]
    pub(super) const fn subject(&self) -> ObservationSubjectRef<'_> {
        match self {
            Self::OperationExposure { subject, .. }
            | Self::CapabilityRequirementDisposition { subject, .. }
            | Self::Health { subject, .. }
            | Self::Readiness { subject, .. }
            | Self::Effect { subject, .. }
            | Self::NativeOwner { subject, .. }
            | Self::Reconciliation { subject, .. } => ObservationSubjectRef::Subject(subject),
            Self::ProcessIntent { .. } => ObservationSubjectRef::ResidentProcess,
        }
    }

    /// Clone the normalized subject used by fact keys and owned projections.
    pub(super) fn owned_subject(&self) -> ObservationSubject {
        match self.subject() {
            ObservationSubjectRef::Subject(subject) => subject.clone(),
            ObservationSubjectRef::ResidentProcess => ObservationSubject::ResidentProcess,
        }
    }

    /// Return the temporal qualification carried by this fact.
    #[must_use]
    pub const fn freshness(&self) -> ObservationFreshness {
        match self {
            Self::OperationExposure { freshness, .. }
            | Self::CapabilityRequirementDisposition { freshness, .. }
            | Self::Health { freshness, .. }
            | Self::Readiness { freshness, .. }
            | Self::Effect { freshness, .. }
            | Self::NativeOwner { freshness, .. }
            | Self::Reconciliation { freshness, .. }
            | Self::ProcessIntent { freshness, .. } => *freshness,
        }
    }

    pub(super) fn key(&self) -> ObservationFactKey {
        ObservationFactKey {
            category: self.category(),
            subject: self.owned_subject(),
        }
    }

    pub(super) fn copied_string_bytes(&self) -> u32 {
        let subject = self.owned_subject().copied_string_bytes();
        let state = match self {
            Self::NativeOwner {
                state: ObservationNativeState::Observed { owner, kind },
                ..
            } => string_bytes(owner.as_str()).saturating_add(string_bytes(kind.as_str())),
            Self::Reconciliation { state, .. } => match state {
                ObservationReconciliationState::Required(digest)
                | ObservationReconciliationState::InProgress(digest) => {
                    string_bytes(digest.as_str())
                }
                ObservationReconciliationState::Settled(digest) => string_bytes(digest.as_str()),
                ObservationReconciliationState::NotRequired
                | ObservationReconciliationState::Unknown
                | ObservationReconciliationState::Unavailable => 0,
            },
            Self::OperationExposure { .. }
            | Self::CapabilityRequirementDisposition { .. }
            | Self::Health { .. }
            | Self::Readiness { .. }
            | Self::Effect { .. }
            | Self::NativeOwner { .. }
            | Self::ProcessIntent { .. } => 0,
        };
        subject.saturating_add(state)
    }
}

impl ObservationSubject {
    pub(super) fn copied_string_bytes(&self) -> u32 {
        match self {
            Self::EffectiveProfile(digest) => string_bytes(digest.as_str()),
            Self::Machine(machine) => string_bytes(machine),
            Self::Repository(repository_id) => string_bytes(repository_id.as_str()),
            Self::Operation {
                repository_id,
                operation_id,
            } => string_bytes(repository_id.as_str())
                .saturating_add(resource_ref_bytes(operation_id)),
            Self::CapabilityRequirement {
                repository_id,
                operation_id,
                requirement_id,
                capability_id,
            } => string_bytes(repository_id.as_str())
                .saturating_add(resource_ref_bytes(operation_id))
                .saturating_add(string_bytes(requirement_id.as_str()))
                .saturating_add(string_bytes(capability_id.as_str())),
            Self::ExecutionPath(path_id) => string_bytes(path_id.as_str()),
            Self::CompiledGeneration(digest) => string_bytes(digest.as_str()),
            Self::ResidentProcess => 0,
        }
    }
}

fn resource_ref_bytes(resource: &PackageResourceRefV1) -> u32 {
    string_bytes(resource.package_id.as_str())
        .saturating_add(string_bytes(resource.resource_id.as_str()))
}

fn string_bytes(value: &str) -> u32 {
    u32::try_from(value.len()).unwrap_or(u32::MAX)
}

/// Borrowed subject view that makes subjectless wire conventions impossible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ObservationSubjectRef<'a> {
    /// Exact fact subject.
    Subject(&'a ObservationSubject),
    /// The singleton resident-process subject.
    ResidentProcess,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ObservationFactKey {
    category: ObservationCategory,
    subject: ObservationSubject,
}

/// One exact filter over independent fact axes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObservationFilter {
    category: Option<ObservationCategory>,
    subject: Option<ObservationSubject>,
    freshness: Option<ObservationFreshnessClass>,
}

/// Filterable freshness class without a stale-age payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationFreshnessClass {
    /// Current for the named generation and revision.
    Fresh,
    /// Retained from an older revision.
    Stale,
    /// Currency is not established.
    Unknown,
}

impl ObservationFilter {
    /// Select every fact.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            category: None,
            subject: None,
            freshness: None,
        }
    }

    /// Restrict the filter to one independent fact category.
    #[must_use]
    pub const fn with_category(mut self, category: ObservationCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Restrict the filter to one exact typed subject.
    #[must_use]
    pub fn with_subject(mut self, subject: ObservationSubject) -> Self {
        self.subject = Some(subject);
        self
    }

    /// Restrict the filter to one temporal class.
    #[must_use]
    pub const fn with_freshness(mut self, freshness: ObservationFreshnessClass) -> Self {
        self.freshness = Some(freshness);
        self
    }

    pub(super) fn matches(&self, fact: &ObservationFact) -> bool {
        let subject = fact.subject();
        self.category.is_none_or(|value| value == fact.category())
            && self.subject.as_ref().is_none_or(|value| {
                matches!(subject, ObservationSubjectRef::Subject(subject) if subject == value)
                    || matches!(
                        (value, subject),
                        (
                            ObservationSubject::ResidentProcess,
                            ObservationSubjectRef::ResidentProcess
                        )
                    )
            })
            && self.freshness.is_none_or(|value| {
                value
                    == match fact.freshness() {
                        ObservationFreshness::Fresh => ObservationFreshnessClass::Fresh,
                        ObservationFreshness::Stale { .. } => ObservationFreshnessClass::Stale,
                        ObservationFreshness::Unknown => ObservationFreshnessClass::Unknown,
                    }
            })
    }
}

/// Bounded query over one exact snapshot.
///
/// The limits bound one step, not the size of the snapshot. A returned
/// continuation carries the exact filter and raw limits required for the next
/// step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationPageRequest {
    /// Fact filter.
    pub filter: ObservationFilter,
    /// Maximum retained facts examined; zero selects the default.
    pub maximum_inspected: u16,
    /// Maximum matching facts returned; zero selects the default page limit.
    pub maximum_returned: u16,
    /// Maximum variable-length UTF-8 bytes copied; zero selects the default.
    pub maximum_copied_string_bytes: u32,
    /// Optional continuation issued by a prior page.
    pub after: Option<ObservationPageCursor>,
    /// Cancellation checked before this bounded step begins.
    pub cancellation: ObservationCancellation,
}

impl ObservationPageRequest {
    /// Construct an active first-page request with default work and copy bounds.
    #[must_use]
    pub const fn new(filter: ObservationFilter, maximum_returned: u16) -> Self {
        Self::bounded(
            filter,
            0,
            maximum_returned,
            0,
            ObservationCancellation::Active,
        )
    }

    /// Construct a first-page request with independent raw bounds.
    #[must_use]
    pub const fn bounded(
        filter: ObservationFilter,
        maximum_inspected: u16,
        maximum_returned: u16,
        maximum_copied_string_bytes: u32,
        cancellation: ObservationCancellation,
    ) -> Self {
        Self {
            filter,
            maximum_inspected,
            maximum_returned,
            maximum_copied_string_bytes,
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
}

/// Resource that ended one bounded observation step before exhaustion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationStepLimit {
    /// The step reached its retained-input inspection bound.
    Inspected,
    /// The step reached its returned-item bound.
    Returned,
    /// The copied-string byte bound ended the step.
    CopiedStringBytes {
        /// Minimum bound that can admit the next indivisible result by itself.
        minimum_required: u32,
    },
    /// The step reached its causal-path lookup bound.
    CausalPathLookups,
    /// The causal-step bound ended the step.
    CausalSteps {
        /// Minimum bound that can admit the next complete causal path by itself.
        minimum_required: u16,
    },
}

/// Whether one bounded observation step exhausted its selected input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationStepCompletion {
    /// The selected input for this snapshot and filter was exhausted.
    Complete,
    /// More selected input can exist after a named resource stopped this step.
    /// Continue with the returned cursor; this state does not prove absence.
    Stopped(ObservationStepLimit),
    /// The next indivisible result does not fit the request's raw limits.
    ///
    /// No continuation is issued because repeating it under the same limits
    /// could not make progress. The result does not imply that later input is
    /// absent.
    Incomplete(ObservationStepLimit),
}

/// One bounded filtered snapshot page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationPage {
    /// Exact snapshot coordinate.
    pub cursor: ObservationCursor,
    /// Returned facts in deterministic category/subject order.
    pub facts: Vec<ObservationFact>,
    /// Continuation when matching facts remain.
    pub continuation: Option<ObservationPageCursor>,
    /// Whether this step exhausted the selected snapshot input.
    pub completion: ObservationStepCompletion,
    /// Retained facts examined by this step, including nonmatches.
    pub inspected: u16,
    /// Variable-length UTF-8 bytes copied into returned facts.
    pub copied_string_bytes: u32,
    /// Owner facts omitted when the bounded snapshot was constructed.
    ///
    /// This count is snapshot-wide because omitted facts are not available for
    /// category-specific classification.
    pub snapshot_omitted_count: u32,
}

/// Result of one bounded snapshot-page step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationPageDisposition {
    /// One valid bounded step; inspect [`ObservationPage::completion`] before
    /// treating its selected input as exhausted.
    Ready(ObservationPage),
    /// Cancellation was observed before this bounded step began.
    Cancelled {
        /// Unchanged continuation that the caller may retry, if one was supplied.
        continuation: Option<ObservationPageCursor>,
    },
}

/// Cooperative request state checked before bounded observation work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationCancellation {
    /// Work may proceed.
    Active,
    /// The caller cancelled before this projection began.
    Cancelled,
}

/// Request for the next complete ordered delta.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationDeltaRequest {
    /// Last completely applied cursor.
    pub resume: ObservationResumeCoordinate,
    /// Maximum changes accepted atomically; zero selects the default page limit.
    pub maximum_changes: u16,
    /// Cooperative cancellation state.
    pub cancellation: ObservationCancellation,
}

/// One exact fact mutation in deterministic key order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationChange {
    /// Insert or replace one independently typed fact.
    Upsert(ObservationFact),
    /// Remove one fact key that no longer exists in the successor.
    Remove {
        /// Removed category.
        category: ObservationCategory,
        /// Removed exact subject.
        subject: ObservationSubject,
    },
}

/// Complete contiguous change set between adjacent compatible revisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationDelta {
    /// Cursor the client must already have applied.
    pub from: ObservationCursor,
    /// Cursor installed after every change is applied.
    pub to: ObservationCursor,
    /// Deterministically ordered complete changes.
    pub changes: Vec<ObservationChange>,
    /// Owner facts omitted from the complete bounded successor snapshot.
    ///
    /// Applying a delta replaces the predecessor count with this value even
    /// when every changed fact lies beyond the retained snapshot bound.
    pub successor_omitted_fact_count: u32,
}

/// Why full snapshot resynchronization is mandatory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationResynchronizationReason {
    /// The requested generation differs from the current generation.
    GenerationMismatch {
        /// Client-held generation.
        requested: Box<ObservationGeneration>,
        /// Current generation.
        current: Box<ObservationGeneration>,
    },
    /// One or more source revisions are absent from this bounded delivery path.
    RevisionGap {
        /// Client-held revision.
        requested: ObservationRevision,
        /// Current source revision.
        current: ObservationRevision,
    },
    /// A page or lens continuation belongs to another query.
    ContinuationMismatch,
}

/// Full-resynchronization instruction bound to the current snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationResynchronization {
    /// Exact reason no delta or page may be applied.
    pub reason: ObservationResynchronizationReason,
    /// Current cursor from which a new snapshot must be requested.
    pub current: Box<ObservationCursor>,
}

/// Recovery offered when one atomic delta exceeds the consumer's bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationBackpressureRecovery {
    /// Request a new bounded full snapshot; no partial delta was emitted.
    RequestSnapshot,
}

/// Result of one ordered delta request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationDeltaOutcome {
    /// Client already names the current cursor.
    CaughtUp(ObservationCursor),
    /// One complete adjacent delta is safe to apply atomically.
    Delta(ObservationDelta),
    /// A gap or generation mismatch forbids delta application.
    Resynchronize(ObservationResynchronization),
    /// Caller cancellation was observed before projection.
    Cancelled(ObservationResumeCoordinate),
    /// The complete adjacent delta exceeds the consumer's declared bound.
    Backpressured {
        /// Number of changes required for atomic application.
        pending_changes: u32,
        /// Effective bounded maximum accepted by the consumer.
        maximum_changes: u16,
        /// Safe recovery; no partial delta was emitted.
        recovery: ObservationBackpressureRecovery,
        /// Current server cursor available through a fresh snapshot.
        current: ObservationCursor,
    },
}

/// Exact subject selector for a retained causal proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationCausalSelector {
    /// Snapshot generation in which the subject is meaningful.
    pub generation: ObservationGeneration,
    /// Exact subject whose retained reason path is requested.
    pub subject: ObservationSubject,
}

/// One application-owned causal step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationCausalStep {
    /// Exact composition reason already admitted by the compiler.
    Composition(CompositionReasonStepV1),
}

/// Bounded exact causal proof for one observation subject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationCausalPath {
    /// Exact subject and generation selected.
    pub selector: ObservationCausalSelector,
    /// Ordered admitted steps; never inferred from display adjacency.
    pub steps: Vec<ObservationCausalStep>,
}

/// Request for one bounded disposable observation lens step.
///
/// The request selects typed facts and optional retained causal paths. It does
/// not request layout, infer relationships, or establish that omitted subjects
/// are absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationLensRequest {
    /// Facts eligible for the lens.
    pub filter: ObservationFilter,
    /// Maximum retained subjects examined; zero selects the default.
    pub maximum_inspected: u16,
    /// Maximum returned nodes; zero selects the default page limit.
    pub maximum_nodes: u16,
    /// Maximum variable-length UTF-8 bytes copied; zero selects the default.
    pub maximum_copied_string_bytes: u32,
    /// Whether retained exact causal paths should be included.
    pub include_causal_paths: bool,
    /// Maximum causal-path lookups; zero selects the default when paths are requested.
    pub maximum_causal_path_lookups: u16,
    /// Maximum causal steps copied; zero selects the default when paths are requested.
    pub maximum_causal_steps: u16,
    /// Optional continuation issued by a prior lens step.
    pub after: Option<ObservationLensCursor>,
    /// Cooperative cancellation state.
    pub cancellation: ObservationCancellation,
}

impl ObservationLensRequest {
    /// Continue from a cursor issued by the same snapshot, filter and raw limits.
    #[must_use]
    pub fn after(mut self, cursor: ObservationLensCursor) -> Self {
        self.after = Some(cursor);
        self
    }
}

/// Opaque position for one bounded lens query.
///
/// The cursor binds the exact snapshot, filter, causal-path choice, raw limits,
/// and next retained subject. It can continue only that query. Its position does
/// not prove that omitted or later subjects are absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationLensCursor {
    pub(super) snapshot: ObservationCursor,
    pub(super) filter: ObservationFilter,
    pub(super) after_subject: Option<ObservationSubject>,
    pub(super) inspected_position: u32,
    pub(super) maximum_inspected: u16,
    pub(super) maximum_nodes: u16,
    pub(super) maximum_copied_string_bytes: u32,
    pub(super) include_causal_paths: bool,
    pub(super) maximum_causal_path_lookups: u16,
    pub(super) maximum_causal_steps: u16,
}

impl ObservationLensCursor {
    /// Borrow the exact snapshot coordinate that issued this continuation.
    #[must_use]
    pub const fn snapshot(&self) -> &ObservationCursor {
        &self.snapshot
    }

    /// Return how many retained subjects precede the next bounded step.
    #[must_use]
    pub const fn inspected_position(&self) -> u32 {
        self.inspected_position
    }

    /// Borrow the filter bound into this continuation.
    #[must_use]
    pub const fn filter(&self) -> &ObservationFilter {
        &self.filter
    }

    /// Return the raw subject-inspection bound.
    #[must_use]
    pub const fn maximum_inspected(&self) -> u16 {
        self.maximum_inspected
    }

    /// Return the raw node bound.
    #[must_use]
    pub const fn maximum_nodes(&self) -> u16 {
        self.maximum_nodes
    }

    /// Return the raw copied-string bound.
    #[must_use]
    pub const fn maximum_copied_string_bytes(&self) -> u32 {
        self.maximum_copied_string_bytes
    }

    /// Return whether causal paths are part of this query.
    #[must_use]
    pub const fn includes_causal_paths(&self) -> bool {
        self.include_causal_paths
    }

    /// Return the raw causal-path lookup bound.
    #[must_use]
    pub const fn maximum_causal_path_lookups(&self) -> u16 {
        self.maximum_causal_path_lookups
    }

    /// Return the raw causal-step bound.
    #[must_use]
    pub const fn maximum_causal_steps(&self) -> u16 {
        self.maximum_causal_steps
    }
}

impl ObservationCausalPath {
    pub(super) fn copied_string_bytes(&self) -> u32 {
        let selector = self
            .selector
            .generation
            .copied_string_bytes()
            .saturating_add(self.selector.subject.copied_string_bytes());
        self.steps.iter().fold(selector, |total, step| {
            let ObservationCausalStep::Composition(reason) = step;
            total.saturating_add(string_bytes(reason.identifier()))
        })
    }
}

impl ObservationGeneration {
    fn copied_string_bytes(&self) -> u32 {
        match self {
            Self::EffectiveProfile {
                profile_digest,
                instance_digest,
                composition_plan_digest,
            } => string_bytes(profile_digest.as_str())
                .saturating_add(string_bytes(instance_digest.as_str()))
                .saturating_add(string_bytes(composition_plan_digest.as_str())),
            Self::Resident {
                authority_store,
                active_generation,
            } => string_bytes(authority_store.store_id().as_str())
                .saturating_add(string_bytes(authority_store.incarnation_id().as_str()))
                .saturating_add(
                    active_generation
                        .as_ref()
                        .map_or(0, |digest| string_bytes(digest.as_str())),
                ),
        }
    }
}

/// One disposable lens node backed by an exact observation subject and facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationLensNode {
    /// Exact subject shared with snapshot queries.
    pub subject: ObservationSubject,
    /// Complete typed facts selected for this subject.
    pub facts: Vec<ObservationFact>,
}

/// Bounded non-authoritative projection for native client rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationLens {
    /// Exact snapshot generation and revision used by every node and path.
    pub cursor: ObservationCursor,
    /// Deduplicated deterministic subject nodes.
    pub nodes: Vec<ObservationLensNode>,
    /// Exact retained causal paths, when requested.
    pub causal_paths: Vec<ObservationCausalPath>,
    /// Continuation when selected subjects may remain.
    pub continuation: Option<ObservationLensCursor>,
    /// Whether this step exhausted the selected snapshot input.
    pub completion: ObservationStepCompletion,
    /// Retained subjects examined by this step, including nonmatches.
    pub inspected_subjects: u16,
    /// Variable-length UTF-8 bytes copied into nodes and causal paths.
    pub copied_string_bytes: u32,
    /// Causal-path lookups performed by this step.
    ///
    /// A lookup is counted before its path passes the causal-step and copied-byte
    /// limits, so a stopped step can count a lookup for an unreturned subject.
    pub causal_path_lookups: u16,
    /// Causal steps copied into returned paths.
    pub causal_steps: u16,
    /// Owner facts omitted when the bounded snapshot was constructed.
    pub snapshot_omitted_count: u32,
}

/// Result of one cooperative lens request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationLensDisposition {
    /// One valid bounded lens step; inspect [`ObservationLens::completion`]
    /// before treating its selected input as exhausted.
    Ready(ObservationLens),
    /// The continuation belongs to another snapshot, filter, or raw limit set.
    /// Obtain a fresh snapshot; this outcome makes no claim about subject state.
    Resynchronize(ObservationResynchronization),
    /// Caller cancellation was observed before this bounded step began.
    Cancelled {
        /// Unchanged continuation that the caller may retry, if one was supplied.
        continuation: Option<ObservationLensCursor>,
    },
}
