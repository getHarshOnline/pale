//! Durable, application-owned consultation collection.
//!
//! A consultation keeps its own bounded roster. Pairwise operation membership
//! proves one child relationship; it is not an enumerable roster or a join.
//! The private collection bytes are committed with the signed parent
//! observation and checkpoint. No public contract family is introduced here.

use std::collections::BTreeSet;

use pale_core::{canon, records::*};
use pale_ports::{
    AccumulatorPreparationLimits, BoundedAuthorityEntryBytes, ControlRecord,
    ControlRecordCoordinate, ControlRecordDigest, DurableAuthorityStoreError,
    OperationCollectionJournal, OperationCollectionJournalError, OperationCollectionSnapshot,
    OperationCustodyPort, OperationCustodyRecord, PreparedAuthorityAppend, SequencedAuthorityEntry,
};
use serde::{Deserialize, Serialize};

mod recovery;
pub use recovery::{recover_committed_consultation_expiry, HistoricalConsultationAdmission};
mod initial_recovery;
pub(in crate::resident) use initial_recovery::recover_committed_initial_collection;

#[allow(
    dead_code,
    reason = "private consultation-start construction awaits the assembled process host"
)]
mod start_intent;
#[allow(
    unused_imports,
    reason = "the process-owned Consult start consumes these private inputs"
)]
pub(in crate::resident) use start_intent::{
    ConsultationFreshChildPlan, ConsultationRosterTemplate, ConsultationRosterTemplateSlot,
    ConsultationRosterTemplateSource, ConsultationStartIntent, ConsultationStartIntentAdmission,
    ConsultationStartIntentError, ConsultationStartIntentSlot, ConsultationStartIntentSource,
};

use super::{
    operation_dispatch::{signing, OperationAttemptError},
    prepare_operation_observation_append, AuthorizedOperationCancellation, IssuedLocalOperation,
    OperationCancellationDisposition, OperationCancellationRequestError,
    OperationObservationAuthorityStoreResolver, OperationObservationCustodyProjectionError,
    OperationObservationTransition, PreparedOperationObservationCheckpoint,
    PreparedOperationObservationCustody, RecoveredOperationObservation,
};

#[allow(
    dead_code,
    reason = "non-join consultation transitions await the assembled resident host"
)]
pub(in crate::resident) mod coordinator;
pub(in crate::resident) use coordinator::ProtectedConsultationChildPhase;
pub(in crate::resident) use coordinator::{
    ResidentConsultationOpenFailure, ResidentConsultationOpenInput,
};
pub(in crate::resident) mod service;
pub use service::task::ConsultationChildRetentionOutcome;
pub(in crate::resident) use service::task::{
    PreparedResidentConsultationTaskLoss, PreparedResidentConsultationTaskResume,
    ResidentConsultationBatchCoordinate, ResidentConsultationBatchOpenError,
    ResidentConsultationBatchRetentionReason, ResidentConsultationBatchRetirement,
    ResidentConsultationCancellationNotification, ResidentConsultationDispatchOwner,
    ResidentConsultationDispatchOwnerSelection, ResidentConsultationPreparedResumeFailure,
    ResidentConsultationTaskLimits, ResidentConsultationTaskLossCommitFailure,
    ResidentConsultationTaskLossOffer, ResidentConsultationTaskLossPreparationFailure,
    ResidentConsultationTaskOwner, ResidentConsultationTaskOwnerAcceptanceFailure,
    ResidentConsultationTaskOwnerBuildError, ResidentConsultationTaskOwnerLimits,
    ResidentConsultationTaskOwnerPoll, ResidentConsultationTaskOwnerPollError,
    ResidentConsultationTaskResumePreparationFailure, ResidentConsultationTaskRetry,
    ResidentConsultationTaskRetryFailure,
};
#[allow(
    unused_imports,
    reason = "private Consult turn dispositions are consumed by resident-host composition"
)]
pub(in crate::resident) use service::{
    apply_consultation_supervision_decision, commit_selected_consultation_stop,
    continue_consultation_stop_custody, continue_consultation_terminal_custody,
    join_consultation_terminal, retry_consultation_terminal_release,
    settle_consultation_nonterminal, supervise_consultation_pre_policy_turn,
    supervise_consultation_turn, supervise_recovered_consultation_pre_policy_turn,
    supervise_recovered_consultation_turn, CompletedResidentConsultationNonterminal,
    CompletedResidentConsultationTerminal, ResidentConsultationCompletionReady,
    ResidentConsultationJoinedTerminal, ResidentConsultationNonterminalSettlementFailure,
    ResidentConsultationOfferRecovery, ResidentConsultationPolicyFrontier,
    ResidentConsultationPrePolicyNeedsRecovery, ResidentConsultationPrePolicyTurn,
    ResidentConsultationStopCommitFailure, ResidentConsultationStopCustody,
    ResidentConsultationStopSelected, ResidentConsultationSupervisionDecision,
    ResidentConsultationTerminalCustody, ResidentConsultationTerminalJoinFailure,
    ResidentConsultationTurnDisposition, ResidentConsultationTurnNeedsRecovery,
    ResidentConsultationTurnPending, ResidentConsultationTurnProgress,
    ResidentConsultationTurnStopped,
};
pub use service::{
    commit_consultation_collection, commit_recovered_consultation_cancellation,
    commit_recovered_consultation_child, commit_recovered_consultation_disagreement,
    commit_recovered_consultation_stop, CompletedConsultationCollection,
    ConsultationCancellationFailure, ConsultationCancellationInput, ConsultationChildJoinFailure,
    ConsultationChildJoinInput, ConsultationCollectionCommitFailure,
    ConsultationCollectionCommitInput, ConsultationDisagreementCommitFailure,
    ConsultationDisagreementCommitInput, ConsultationStopFailure, ConsultationStopInput,
    ConsultationSuccessorCommitInput,
};
/// Requiredness of one admitted consultation contribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConsultationRequirement {
    /// The selected policy requires this contribution or an admitted substitute.
    Required,
    /// The selected policy may use this contribution without requiring it.
    Optional,
}

/// Stable private identity of one consultation slot.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(transparent)]
pub struct ConsultationSlotId(String);

impl ConsultationSlotId {
    /// Admit a short, non-empty private slot identity.
    ///
    /// # Errors
    /// Rejects empty values, surrounding whitespace, control characters, or
    /// more than 128 Unicode scalar values.
    pub fn parse(value: impl Into<String>) -> Result<Self, ConsultationCollectionError> {
        let value = value.into();
        if value.is_empty()
            || value.trim() != value
            || value.chars().count() > 128
            || value.chars().any(char::is_control)
        {
            return Err(ConsultationCollectionError::Bounds);
        }
        Ok(Self(value))
    }

    /// Borrow the admitted spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Initial source of one attributable consultation contribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConsultationContributionSource {
    /// Existing evidence satisfied this slot under the selected policy.
    EvidenceResolved {
        /// Exact admitted evidence; lookup alone does not construct this state.
        evidence: ImmutableRecordReferenceV1,
    },
    /// A fresh child operation is responsible for producing the contribution.
    FreshChild {
        /// Exact immutable child handle already started under this Consult.
        child: ImmutableRecordReferenceV1,
    },
}

/// Current collection state of one admitted consultation slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConsultationSlotDisposition {
    /// A fresh contribution is still required.
    Outstanding,
    /// Exact attributable evidence fulfilled this slot under the selected policy.
    Fulfilled {
        /// Evidence admitted for this exact slot.
        evidence: ImmutableRecordReferenceV1,
        /// Time at which the collection admitted the evidence.
        recorded_at: UnixTimeV1,
    },
    /// The selected contribution cannot presently be obtained.
    Unfulfillable {
        /// Exact evidence explaining why the requirement cannot be fulfilled now.
        reason: ImmutableRecordReferenceV1,
        /// Time at which the collection admitted this disposition.
        recorded_at: UnixTimeV1,
    },
}

/// One bounded, attributable slot in a consultation roster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsultationRosterSlot {
    slot_id: ConsultationSlotId,
    requirement: ConsultationRequirement,
    requirement_evidence: ImmutableRecordReferenceV1,
    participant: ParticipantRefV1,
    provider_binding: Option<ProviderBindingRefV1>,
    context: Option<ImmutableRecordReferenceV1>,
    source: ConsultationContributionSource,
}

impl ConsultationRosterSlot {
    /// Construct one slot after its policy, participant, provider, and context
    /// coordinates have been selected by their existing owners.
    #[must_use]
    pub fn new(
        slot_id: ConsultationSlotId,
        requirement: ConsultationRequirement,
        requirement_evidence: ImmutableRecordReferenceV1,
        participant: ParticipantRefV1,
        provider_binding: Option<ProviderBindingRefV1>,
        context: Option<ImmutableRecordReferenceV1>,
        source: ConsultationContributionSource,
    ) -> Self {
        Self {
            slot_id,
            requirement,
            requirement_evidence,
            participant,
            provider_binding,
            context,
            source,
        }
    }

    /// Borrow the stable private slot identity.
    #[must_use]
    pub const fn slot_id(&self) -> &ConsultationSlotId {
        &self.slot_id
    }

    /// Return whether the selected policy requires this slot.
    #[must_use]
    pub const fn requirement(&self) -> ConsultationRequirement {
        self.requirement
    }

    /// Borrow the exact contribution source.
    #[must_use]
    pub const fn source(&self) -> &ConsultationContributionSource {
        &self.source
    }

    /// Borrow the evidence that established this contribution requirement.
    #[must_use]
    pub const fn requirement_evidence(&self) -> &ImmutableRecordReferenceV1 {
        &self.requirement_evidence
    }

    /// Borrow the selected contributor identity.
    #[must_use]
    pub const fn participant(&self) -> &ParticipantRefV1 {
        &self.participant
    }

    /// Borrow the selected provider binding, when this contribution needs one.
    #[must_use]
    pub const fn provider_binding(&self) -> Option<&ProviderBindingRefV1> {
        self.provider_binding.as_ref()
    }

    /// Borrow the recipient-specific context selected for this contribution.
    #[must_use]
    pub const fn context(&self) -> Option<&ImmutableRecordReferenceV1> {
        self.context.as_ref()
    }
}

/// One roster slot and its current, attributable collection disposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsultationCollectionSlot {
    roster: ConsultationRosterSlot,
    disposition: ConsultationSlotDisposition,
}

impl ConsultationCollectionSlot {
    /// Borrow the immutable roster declaration.
    #[must_use]
    pub const fn roster(&self) -> &ConsultationRosterSlot {
        &self.roster
    }

    /// Borrow the current collection disposition.
    #[must_use]
    pub const fn disposition(&self) -> &ConsultationSlotDisposition {
        &self.disposition
    }
}

/// Why collection stopped. The selected policy still owns sufficiency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsultationStopCause {
    /// The selected completion policy chose to stop.
    CompletionPolicy,
    /// The admitted deadline was reached.
    Deadline,
    /// Cancellation policy stopped further collection.
    Cancellation,
    /// The parent operation reached its expiry boundary.
    OperationExpiry,
}

/// Exact unresolved sets computed from the roster when collection stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsultationRequirementSummary {
    requirements_met: bool,
    required_outstanding: Vec<ConsultationSlotId>,
    required_unfulfillable: Vec<ConsultationSlotId>,
    optional_outstanding: Vec<ConsultationSlotId>,
    optional_unfulfillable: Vec<ConsultationSlotId>,
}

impl ConsultationRequirementSummary {
    /// Return whether every required slot was fulfilled.
    #[must_use]
    pub const fn requirements_met(&self) -> bool {
        self.requirements_met
    }

    /// Borrow required slots that were still awaiting a contribution.
    #[must_use]
    pub fn required_outstanding(&self) -> &[ConsultationSlotId] {
        &self.required_outstanding
    }

    /// Borrow required slots that could not presently be fulfilled.
    #[must_use]
    pub fn required_unfulfillable(&self) -> &[ConsultationSlotId] {
        &self.required_unfulfillable
    }
}

/// Final collection decision retained separately from operation acceptance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsultationCollectionStop {
    cause: ConsultationStopCause,
    reason: ImmutableRecordReferenceV1,
    stopped_at: UnixTimeV1,
    summary: ConsultationRequirementSummary,
}

impl ConsultationCollectionStop {
    /// Return the mechanical cause that ended collection.
    #[must_use]
    pub const fn cause(&self) -> ConsultationStopCause {
        self.cause
    }

    /// Borrow the exact policy or boundary evidence for the stop.
    #[must_use]
    pub const fn reason(&self) -> &ImmutableRecordReferenceV1 {
        &self.reason
    }

    /// Borrow the exact unresolved sets frozen by the stop.
    #[must_use]
    pub const fn summary(&self) -> &ConsultationRequirementSummary {
        &self.summary
    }
}

/// Terminal disposition of one child received after collection had stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case", deny_unknown_fields)]
pub enum LateConsultationDisposition {
    /// The child produced evidence that could have fulfilled its slot.
    Fulfilled {
        /// Exact admitted result evidence from the child.
        evidence: ImmutableRecordReferenceV1,
    },
    /// The child failed or was cancelled with an attributable diagnostic.
    Unfulfillable {
        /// Exact admitted diagnostic explaining the child outcome.
        reason: ImmutableRecordReferenceV1,
    },
}

/// Whether this exact child terminal is absent or already retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum ConsultationChildTerminalRetention {
    /// The exact terminal evidence has not entered the current collection.
    Absent,
    /// The exact terminal evidence is already in its slot or late-outcome log.
    Exact,
}

/// Attributable child outcome received after collection had stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LateConsultationOutcome {
    slot_id: ConsultationSlotId,
    child: ImmutableRecordReferenceV1,
    disposition: LateConsultationDisposition,
    recorded_at: UnixTimeV1,
}

impl LateConsultationOutcome {
    /// Borrow the slot that the late outcome addressed.
    #[must_use]
    pub const fn slot_id(&self) -> &ConsultationSlotId {
        &self.slot_id
    }

    /// Borrow the child operation that produced this late outcome.
    #[must_use]
    pub const fn child(&self) -> &ImmutableRecordReferenceV1 {
        &self.child
    }

    /// Borrow the late outcome without changing the frozen stop result.
    #[must_use]
    pub const fn disposition(&self) -> &LateConsultationDisposition {
        &self.disposition
    }

    /// Return when the collection admitted the late outcome.
    #[must_use]
    pub const fn recorded_at(&self) -> UnixTimeV1 {
        self.recorded_at
    }
}

/// Explicit disagreement between attributable fulfilled contributions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsultationDisagreement {
    slots: Vec<ConsultationSlotId>,
    evidence: ImmutableRecordReferenceV1,
    recorded_at: UnixTimeV1,
}

impl ConsultationDisagreement {
    /// Borrow the distinct fulfilled slots in stable order.
    #[must_use]
    pub fn slots(&self) -> &[ConsultationSlotId] {
        &self.slots
    }

    /// Borrow evidence that establishes the disagreement.
    #[must_use]
    pub const fn evidence(&self) -> &ImmutableRecordReferenceV1 {
        &self.evidence
    }
}

/// Explicit hard limits for one private consultation snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsultationCollectionLimits {
    maximum_slots: usize,
    maximum_journal_bytes: usize,
    maximum_frontier_bytes: usize,
}

/// Application-admitted inputs for one initial consultation roster.
pub struct ConsultationCollectionAdmission {
    /// Exact completion-policy evidence selected for this consultation.
    pub completion_policy: ImmutableRecordReferenceV1,
    /// Time at which collection begins.
    pub opened_at: UnixTimeV1,
    /// Latest time at which ordinary collection may continue.
    pub deadline: UnixTimeV1,
    /// Complete bounded roster in stable application-selected order.
    pub slots: Vec<ConsultationRosterSlot>,
}

/// One attributable fresh-child contribution to an existing collection.
pub struct ConsultationContribution {
    /// Roster slot addressed by the contribution.
    pub slot_id: ConsultationSlotId,
    /// Exact child operation that produced the evidence.
    pub child: ImmutableRecordReferenceV1,
    /// Exact admitted evidence produced by that child.
    pub evidence: ImmutableRecordReferenceV1,
    /// Time at which the collection admitted the evidence.
    pub recorded_at: UnixTimeV1,
}

/// One outstanding child selected from an exact recovered consultation roster.
///
/// This is a finite dispatch input, not renewed authority or proof that a
/// provider ran. The child dispatch path must still recheck its operation,
/// lease, provider, context, time, and current constitutional authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsultationChildDispatchCandidate {
    slot_id: ConsultationSlotId,
    child: ImmutableRecordReferenceV1,
    requirement: ConsultationRequirement,
    requirement_evidence: ImmutableRecordReferenceV1,
    participant: ParticipantRefV1,
    provider_binding: Option<ProviderBindingRefV1>,
    context: Option<ImmutableRecordReferenceV1>,
}

impl ConsultationChildDispatchCandidate {
    /// Borrow the exact roster slot that owns this child.
    #[must_use]
    pub const fn slot_id(&self) -> &ConsultationSlotId {
        &self.slot_id
    }

    /// Borrow the already durable child operation handle.
    #[must_use]
    pub const fn child(&self) -> &ImmutableRecordReferenceV1 {
        &self.child
    }

    /// Return whether the selected policy requires this contribution.
    #[must_use]
    pub const fn requirement(&self) -> ConsultationRequirement {
        self.requirement
    }

    /// Borrow the evidence that established the contribution requirement.
    #[must_use]
    pub const fn requirement_evidence(&self) -> &ImmutableRecordReferenceV1 {
        &self.requirement_evidence
    }

    /// Borrow the selected contributor identity.
    #[must_use]
    pub const fn participant(&self) -> &ParticipantRefV1 {
        &self.participant
    }

    /// Borrow the selected provider binding, when present.
    #[must_use]
    pub const fn provider_binding(&self) -> Option<&ProviderBindingRefV1> {
        self.provider_binding.as_ref()
    }

    /// Borrow the recipient-specific context, when selected.
    #[must_use]
    pub const fn context(&self) -> Option<&ImmutableRecordReferenceV1> {
        self.context.as_ref()
    }
}

/// Finite child-dispatch candidates from one exact recovered roster frontier.
///
/// The caller may execute these independent candidates concurrently. This
/// batch does not select a concurrency implementation or a completion policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsultationChildDispatchBatch {
    store: pale_ports::AuthorityStoreCoordinate,
    parent: ImmutableRecordReferenceV1,
    collection_frontier: ImmutableRecordReferenceV1,
    deadline: UnixTimeV1,
    candidates: Vec<ConsultationChildDispatchCandidate>,
}

impl ConsultationChildDispatchBatch {
    /// Borrow the exact parent-store incarnation used to recover this batch.
    #[must_use]
    pub(in crate::resident) const fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the Consult operation that owns this roster.
    #[must_use]
    pub(in crate::resident) const fn parent(&self) -> &ImmutableRecordReferenceV1 {
        &self.parent
    }

    /// Borrow the exact roster frontier from which this batch was derived.
    #[must_use]
    pub const fn collection_frontier(&self) -> &ImmutableRecordReferenceV1 {
        &self.collection_frontier
    }

    /// Return the last admitted time for ordinary child collection.
    #[must_use]
    pub const fn deadline(&self) -> UnixTimeV1 {
        self.deadline
    }

    /// Borrow outstanding child candidates in stable roster order.
    #[must_use]
    pub fn candidates(&self) -> &[ConsultationChildDispatchCandidate] {
        &self.candidates
    }
}

impl ConsultationCollectionLimits {
    /// Admit nonzero collection limits.
    ///
    /// # Errors
    /// Rejects any zero ceiling.
    pub fn new(
        maximum_slots: usize,
        maximum_journal_bytes: usize,
        maximum_frontier_bytes: usize,
    ) -> Result<Self, ConsultationCollectionError> {
        if maximum_slots == 0 || maximum_journal_bytes == 0 || maximum_frontier_bytes == 0 {
            return Err(ConsultationCollectionError::Bounds);
        }
        Ok(Self {
            maximum_slots,
            maximum_journal_bytes,
            maximum_frontier_bytes,
        })
    }
}

/// Initial durable consultation roster selected by the application owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsultationCollection {
    initial_v1_domain: String,
    parent: ImmutableRecordReferenceV1,
    completion_policy: ImmutableRecordReferenceV1,
    opened_at: UnixTimeV1,
    updated_at: UnixTimeV1,
    deadline: UnixTimeV1,
    expires_at: UnixTimeV1,
    retain_until: UnixTimeV1,
    predecessor_frontier: Option<ImmutableRecordReferenceV1>,
    frozen_stop_frontier: Option<ImmutableRecordReferenceV1>,
    slots: Vec<ConsultationCollectionSlot>,
    disagreements: Vec<ConsultationDisagreement>,
    stop: Option<ConsultationCollectionStop>,
    late_outcomes: Vec<LateConsultationOutcome>,
}

impl ConsultationCollection {
    /// Admit an initial roster for one exact Consult operation.
    ///
    /// Existing evidence must already have passed the selected policy's
    /// substitution rule. Fresh children must be exact Invoke or Consult
    /// descendants of this parent; Delegate would transfer responsibility and
    /// therefore cannot stand in for a consultation contribution.
    ///
    /// # Errors
    /// Rejects an unlike parent, time range, duplicate slot or child, excessive
    /// roster, or child outside the exact parent relationship.
    pub fn admit_initial(
        parent_ref: &ResidentRecordRefV1,
        parent: &OperationHandleV1,
        admission: ConsultationCollectionAdmission,
        child_handles: &[(&ResidentRecordRefV1, &OperationHandleV1)],
        limits: ConsultationCollectionLimits,
    ) -> Result<Self, ConsultationCollectionError> {
        let ConsultationCollectionAdmission {
            completion_policy,
            opened_at,
            deadline,
            slots,
        } = admission;
        if parent.operation_class() != &OperationClassV1::Consult
            || opened_at < *parent.created_at()
            || opened_at > deadline
            || deadline > *parent.expires_at()
            || slots.is_empty()
            || slots.len() > limits.maximum_slots
        {
            return Err(ConsultationCollectionError::Association);
        }
        let mut slot_ids = BTreeSet::new();
        let mut selected_children = BTreeSet::new();
        let expected_parent = ParentOperationReferenceV1 {
            operation_handle_id: parent.operation_handle_id().clone(),
            operation_handle_ref: parent_ref.clone(),
            operation_handle_sha256: parent.canonical_operation_handle_sha256().clone(),
        };
        for slot in &slots {
            if !slot_ids.insert(slot.slot_id.clone()) {
                return Err(ConsultationCollectionError::DuplicateSlot);
            }
            let ConsultationContributionSource::FreshChild { child } = &slot.source else {
                continue;
            };
            if !selected_children.insert((child.record_ref.clone(), child.record_sha256.clone())) {
                return Err(ConsultationCollectionError::DuplicateChild);
            }
            let Some((_, handle)) = child_handles.iter().find(|(reference, handle)| {
                *reference == &child.record_ref
                    && handle.canonical_operation_handle_sha256().as_str()
                        == child.record_sha256.as_str()
            }) else {
                return Err(ConsultationCollectionError::Child);
            };
            if !matches!(
                handle.operation_class(),
                OperationClassV1::Invoke | OperationClassV1::Consult
            ) || handle.parent_operation().as_ref() != Some(&expected_parent)
            {
                return Err(ConsultationCollectionError::Child);
            }
        }
        let slots = slots
            .into_iter()
            .map(|roster| {
                let disposition = match &roster.source {
                    ConsultationContributionSource::EvidenceResolved { evidence } => {
                        ConsultationSlotDisposition::Fulfilled {
                            evidence: evidence.clone(),
                            recorded_at: opened_at,
                        }
                    }
                    ConsultationContributionSource::FreshChild { .. } => {
                        ConsultationSlotDisposition::Outstanding
                    }
                };
                ConsultationCollectionSlot {
                    roster,
                    disposition,
                }
            })
            .collect();
        Ok(Self {
            initial_v1_domain: "PALE_PRIVATE_CONSULTATION_COLLECTION_V1".into(),
            parent: immutable_operation_reference(parent_ref, parent)?,
            completion_policy,
            opened_at,
            updated_at: opened_at,
            deadline,
            expires_at: *parent.expires_at(),
            retain_until: *parent.retain_until(),
            predecessor_frontier: None,
            frozen_stop_frontier: None,
            slots,
            disagreements: Vec::new(),
            stop: None,
            late_outcomes: Vec::new(),
        })
    }

    /// Borrow the exact parent handle coordinate.
    #[must_use]
    pub const fn parent(&self) -> &ImmutableRecordReferenceV1 {
        &self.parent
    }

    /// Borrow the selected completion-policy evidence.
    #[must_use]
    pub const fn completion_policy(&self) -> &ImmutableRecordReferenceV1 {
        &self.completion_policy
    }

    pub(in crate::resident) const fn opened_at(&self) -> UnixTimeV1 {
        self.opened_at
    }

    pub(in crate::resident) const fn deadline(&self) -> UnixTimeV1 {
        self.deadline
    }

    /// Borrow the complete bounded roster.
    #[must_use]
    pub fn slots(&self) -> &[ConsultationCollectionSlot] {
        &self.slots
    }

    /// Borrow the collection stop, if policy or a boundary ended collection.
    #[must_use]
    pub const fn stopped(&self) -> Option<&ConsultationCollectionStop> {
        self.stop.as_ref()
    }

    /// Borrow disagreements without merging or ranking their contributions.
    #[must_use]
    pub fn disagreements(&self) -> &[ConsultationDisagreement] {
        &self.disagreements
    }

    /// Borrow child outcomes received after the historical stop result was frozen.
    #[must_use]
    pub fn late_outcomes(&self) -> &[LateConsultationOutcome] {
        &self.late_outcomes
    }

    /// Recover the current bounded roster through its handle-scoped point index.
    ///
    /// The supplied observation must already be admitted for `operation`. While
    /// collection is open, its evidence names the indexed frontier. After a stop,
    /// the observation keeps naming the frontier that froze the stop result; the
    /// handle-scoped point index may advance only to retain late outcomes.
    /// `admit` must re-admit policy, participants, child handles, contexts, and
    /// evidence under the current compiled generation.
    ///
    /// # Errors
    /// Rejects absent or unlike evidence, malformed canonical bytes, exceeded
    /// bounds, or a collection rejected by its semantic owner.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn recover_current<Admit>(
        operation: &IssuedLocalOperation,
        observation: &OperationObservationV1,
        limits: ConsultationCollectionLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        admit: Admit,
    ) -> Result<Option<RecoveredConsultationCollection>, ConsultationCollectionError>
    where
        Admit: FnOnce(&Self) -> Result<(), InvalidResidentRecordV1>,
    {
        let (handle_ref, handle) = operation.handle();
        if observation.operation_handle_ref() != handle_ref
            || observation.operation_handle_sha256() != handle.canonical_operation_handle_sha256()
        {
            return Err(ConsultationCollectionError::Association);
        }
        let expected_frontier = match observation.state() {
            OperationObservationStateV1::Active {
                phase:
                    OperationActivePhaseV1::CollectingConsultation { evidence }
                    | OperationActivePhaseV1::ConsultationCollected { evidence },
            } => Some(evidence),
            _ => None,
        };
        let cancellation_request = match observation.state() {
            OperationObservationStateV1::CancellationRequested { request } => Some(request),
            _ => None,
        };
        let store = resolve_store.resolve(observation)?;
        let handle_coordinate = ControlRecordCoordinate::new(
            handle_ref.clone(),
            ControlRecordDigest::parse(handle.canonical_operation_handle_sha256().as_str())?,
        );
        let snapshot = custody.read_collection_snapshot(
            &store,
            &handle_coordinate,
            limits.maximum_frontier_bytes,
            limits.maximum_journal_bytes,
        )?;
        if expected_frontier.is_none() && cancellation_request.is_none() {
            return if snapshot.is_none() {
                Ok(None)
            } else {
                Err(ConsultationCollectionError::Association)
            };
        }
        let snapshot = snapshot.ok_or(ConsultationCollectionError::Association)?;
        let expected_head_query = pale_ports::OperationCustodyIndexQuery::CollectionHead(
            pale_ports::OperationCustodyIndexKey::for_collection_head(&handle_coordinate),
        );
        if snapshot.head().query() != &expected_head_query
            || snapshot.head().current_frontier() != snapshot.frontier().coordinate()
        {
            return Err(ConsultationCollectionError::Association);
        }
        let bytes = snapshot.journal_payload().as_slice();
        let collection = Self::deserialize(canon::parse_strict(bytes)?)
            .map_err(|_| ConsultationCollectionError::Decoding)?;
        if canon::to_canonical_string(&collection)?.as_bytes() != bytes {
            return Err(ConsultationCollectionError::Decoding);
        }
        collection.validate_recovered(handle_ref, handle, limits)?;
        let stopped_phase = cancellation_request.is_some()
            || matches!(
                observation.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ConsultationCollected { .. }
                }
            );
        if stopped_phase != collection.stop.is_some() {
            return Err(ConsultationCollectionError::Association);
        }
        let indexed_frontier = immutable_frontier_reference(snapshot.frontier())?;
        let frontier_matches = if let Some(request) = cancellation_request {
            collection.stop.as_ref().is_some_and(|stop| {
                stop.cause == ConsultationStopCause::Cancellation && &stop.reason == request
            }) && collection
                .frozen_stop_frontier
                .as_ref()
                .is_none_or(|frozen| frozen != &indexed_frontier)
        } else {
            let expected_frontier =
                expected_frontier.ok_or(ConsultationCollectionError::Association)?;
            match &collection.frozen_stop_frontier {
                None => &indexed_frontier == expected_frontier,
                Some(frozen) => {
                    stopped_phase && frozen == expected_frontier && frozen != &indexed_frontier
                }
            }
        };
        if !frontier_matches {
            return Err(ConsultationCollectionError::Association);
        }
        admit(&collection)?;
        Ok(Some(RecoveredConsultationCollection {
            collection,
            snapshot,
            store,
        }))
    }

    fn validate_recovered(
        &self,
        handle_ref: &ResidentRecordRefV1,
        handle: &OperationHandleV1,
        limits: ConsultationCollectionLimits,
    ) -> Result<(), ConsultationCollectionError> {
        if self.initial_v1_domain != "PALE_PRIVATE_CONSULTATION_COLLECTION_V1"
            || self.parent != immutable_operation_reference(handle_ref, handle)?
            || handle.operation_class() != &OperationClassV1::Consult
            || self.opened_at < *handle.created_at()
            || self.opened_at > self.updated_at
            || self.updated_at >= self.retain_until
            || self.deadline < self.opened_at
            || self.deadline > self.expires_at
            || self.expires_at != *handle.expires_at()
            || self.retain_until != *handle.retain_until()
            || self.slots.is_empty()
            || self.slots.len() > limits.maximum_slots
            || self.disagreements.len() > limits.maximum_slots
            || self.late_outcomes.len() > limits.maximum_slots
            || self.frozen_stop_frontier.is_some() == self.late_outcomes.is_empty()
        {
            return Err(ConsultationCollectionError::Association);
        }
        let mut slot_ids = BTreeSet::new();
        let mut fresh_children = BTreeSet::new();
        for slot in &self.slots {
            if ConsultationSlotId::parse(slot.roster.slot_id.as_str())? != slot.roster.slot_id
                || !slot_ids.insert(slot.roster.slot_id.clone())
            {
                return Err(ConsultationCollectionError::DuplicateSlot);
            }
            if let ConsultationContributionSource::FreshChild { child } = &slot.roster.source {
                if !fresh_children.insert((child.record_ref.clone(), child.record_sha256.clone())) {
                    return Err(ConsultationCollectionError::DuplicateChild);
                }
            } else if !matches!(
                (&slot.roster.source, &slot.disposition),
                (
                    ConsultationContributionSource::EvidenceResolved { evidence: selected },
                    ConsultationSlotDisposition::Fulfilled { evidence, recorded_at }
                ) if selected == evidence && *recorded_at == self.opened_at
            ) {
                return Err(ConsultationCollectionError::Contribution);
            }
            let recorded_at = match &slot.disposition {
                ConsultationSlotDisposition::Outstanding => continue,
                ConsultationSlotDisposition::Fulfilled { recorded_at, .. }
                | ConsultationSlotDisposition::Unfulfillable { recorded_at, .. } => *recorded_at,
            };
            if recorded_at < self.opened_at
                || recorded_at > self.updated_at
                || recorded_at > self.expires_at
            {
                return Err(ConsultationCollectionError::Contribution);
            }
        }
        for disagreement in &self.disagreements {
            if disagreement.slots.len() < 2
                || !disagreement.slots.windows(2).all(|pair| pair[0] < pair[1])
                || disagreement.recorded_at < self.opened_at
                || disagreement.recorded_at > self.updated_at
                || disagreement.recorded_at > self.expires_at
                || disagreement.slots.iter().any(|id| {
                    !self.slots.iter().any(|slot| {
                        slot.roster.slot_id() == id
                            && matches!(
                                slot.disposition,
                                ConsultationSlotDisposition::Fulfilled { .. }
                            )
                    })
                })
            {
                return Err(ConsultationCollectionError::Disagreement);
            }
        }
        let mut late_slots = BTreeSet::new();
        for late in &self.late_outcomes {
            let Some(slot) = self
                .slots
                .iter()
                .find(|slot| slot.roster.slot_id() == &late.slot_id)
            else {
                return Err(ConsultationCollectionError::Slot);
            };
            if !matches!(
                &slot.roster.source,
                ConsultationContributionSource::FreshChild { child } if child == &late.child
            ) || !late_slots.insert((late.slot_id.clone(), late.child.clone()))
                || self.stop.as_ref().is_none_or(|stop| {
                    late.recorded_at < stop.stopped_at
                        || late.recorded_at > self.updated_at
                        || late.recorded_at >= self.retain_until
                })
            {
                return Err(ConsultationCollectionError::Contribution);
            }
        }
        if let Some(stop) = &self.stop {
            if stop.stopped_at < self.opened_at
                || stop.stopped_at > self.updated_at
                || stop.summary != self.requirement_summary()
                || (stop.cause == ConsultationStopCause::Deadline
                    && stop.stopped_at < self.deadline)
                || (stop.cause == ConsultationStopCause::OperationExpiry
                    && stop.stopped_at < self.expires_at)
                || (stop.cause != ConsultationStopCause::OperationExpiry
                    && stop.stopped_at > self.expires_at)
                || stop.stopped_at >= self.retain_until
            {
                return Err(ConsultationCollectionError::Stopped);
            }
        } else if !self.late_outcomes.is_empty() {
            return Err(ConsultationCollectionError::Stopped);
        }
        Ok(())
    }

    /// Admit one fresh child contribution against its exact roster slot.
    ///
    /// The admission callback owns freshness, disclosure, subject, and evidence
    /// suitability. If collection already stopped, the contribution is retained
    /// as a late outcome until the parent handle's retention boundary. It cannot
    /// rewrite the frozen stop summary or restart collection.
    ///
    /// # Errors
    /// Rejects unlike frontier, time, slot, child, duplicate contribution,
    /// exceeded bounds, or evidence rejected by its semantic owner.
    pub fn admit_child_contribution<Admit>(
        &self,
        current: &OperationCollectionJournal,
        contribution: ConsultationContribution,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<Self, ConsultationCollectionError>
    where
        Admit: FnOnce(
            &ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        self.admit_child_contribution_from(
            ConsultationCollectionCurrent::Prepared(current),
            contribution,
            limits,
            admit,
        )
    }

    fn admit_child_contribution_from<Admit>(
        &self,
        current: ConsultationCollectionCurrent<'_>,
        contribution: ConsultationContribution,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<Self, ConsultationCollectionError>
    where
        Admit: FnOnce(
            &ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let ConsultationContribution {
            slot_id,
            child,
            evidence,
            recorded_at,
        } = contribution;
        if self.stop.is_none() && recorded_at > self.deadline {
            return Err(ConsultationCollectionError::Stopped);
        }
        let horizon = if self.stop.is_some() {
            ConsultationSuccessorHorizon::Retention
        } else {
            ConsultationSuccessorHorizon::Execution
        };
        let mut successor = self.successor(current, recorded_at, limits, horizon)?;
        let Some(slot) = successor
            .slots
            .iter_mut()
            .find(|slot| slot.roster.slot_id() == &slot_id)
        else {
            return Err(ConsultationCollectionError::Slot);
        };
        let ConsultationContributionSource::FreshChild { child: expected } = &slot.roster.source
        else {
            return Err(ConsultationCollectionError::Contribution);
        };
        if expected != &child {
            return Err(ConsultationCollectionError::Child);
        }
        admit(&slot.roster, &evidence)?;
        if successor.stop.is_some() {
            if successor.late_outcomes.len() >= limits.maximum_slots
                || successor
                    .late_outcomes
                    .iter()
                    .any(|late| late.slot_id == slot_id && late.child == child)
            {
                return Err(ConsultationCollectionError::Contribution);
            }
            if successor.frozen_stop_frontier.is_none() {
                successor.frozen_stop_frontier =
                    Some(immutable_frontier_reference(current.frontier())?);
            }
            successor.late_outcomes.push(LateConsultationOutcome {
                slot_id,
                child,
                disposition: LateConsultationDisposition::Fulfilled { evidence },
                recorded_at,
            });
            return Ok(successor);
        }
        if !matches!(slot.disposition, ConsultationSlotDisposition::Outstanding) {
            return Err(ConsultationCollectionError::Contribution);
        }
        slot.disposition = ConsultationSlotDisposition::Fulfilled {
            evidence,
            recorded_at,
        };
        Ok(successor)
    }

    /// Record that one requirement cannot presently be fulfilled.
    ///
    /// While collection is open, this changes one outstanding slot. After a
    /// stop, it retains the exact fresh child's late diagnostic without
    /// changing the frozen slot, summary, or stop decision.
    ///
    /// # Errors
    /// Rejects unlike frontier, time, slot, non-outstanding state, or reason.
    pub fn mark_unfulfillable<Admit>(
        &self,
        current: &OperationCollectionJournal,
        slot_id: &ConsultationSlotId,
        reason: ImmutableRecordReferenceV1,
        recorded_at: UnixTimeV1,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<Self, ConsultationCollectionError>
    where
        Admit: FnOnce(
            &ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        self.mark_unfulfillable_from(
            ConsultationCollectionCurrent::Prepared(current),
            slot_id,
            reason,
            recorded_at,
            limits,
            admit,
        )
    }

    fn mark_unfulfillable_from<Admit>(
        &self,
        current: ConsultationCollectionCurrent<'_>,
        slot_id: &ConsultationSlotId,
        reason: ImmutableRecordReferenceV1,
        recorded_at: UnixTimeV1,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<Self, ConsultationCollectionError>
    where
        Admit: FnOnce(
            &ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        if self.stop.is_none() && recorded_at > self.deadline {
            return Err(ConsultationCollectionError::Stopped);
        }
        let horizon = if self.stop.is_some() {
            ConsultationSuccessorHorizon::Retention
        } else {
            ConsultationSuccessorHorizon::Execution
        };
        let mut successor = self.successor(current, recorded_at, limits, horizon)?;
        let Some(slot) = successor
            .slots
            .iter_mut()
            .find(|slot| slot.roster.slot_id() == slot_id)
        else {
            return Err(ConsultationCollectionError::Slot);
        };
        if successor.stop.is_some() {
            let ConsultationContributionSource::FreshChild { child } = &slot.roster.source else {
                return Err(ConsultationCollectionError::Contribution);
            };
            let child = child.clone();
            admit(&slot.roster, &reason)?;
            if successor.late_outcomes.len() >= limits.maximum_slots
                || successor
                    .late_outcomes
                    .iter()
                    .any(|late| late.slot_id == *slot_id && late.child == child)
            {
                return Err(ConsultationCollectionError::Contribution);
            }
            if successor.frozen_stop_frontier.is_none() {
                successor.frozen_stop_frontier =
                    Some(immutable_frontier_reference(current.frontier())?);
            }
            successor.late_outcomes.push(LateConsultationOutcome {
                slot_id: slot_id.clone(),
                child,
                disposition: LateConsultationDisposition::Unfulfillable { reason },
                recorded_at,
            });
            return Ok(successor);
        }
        if !matches!(slot.disposition, ConsultationSlotDisposition::Outstanding) {
            return Err(ConsultationCollectionError::Contribution);
        }
        admit(&slot.roster, &reason)?;
        slot.disposition = ConsultationSlotDisposition::Unfulfillable {
            reason,
            recorded_at,
        };
        Ok(successor)
    }

    /// Preserve an admitted disagreement without voting or synthesis.
    ///
    /// # Errors
    /// Rejects unlike frontier, time, duplicate/unknown/unfulfilled slots,
    /// fewer than two sides, repeated disagreement, or exceeded bounds.
    pub fn record_disagreement<Admit>(
        &self,
        current: &OperationCollectionJournal,
        slots: Vec<ConsultationSlotId>,
        evidence: ImmutableRecordReferenceV1,
        recorded_at: UnixTimeV1,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<Self, ConsultationCollectionError>
    where
        Admit: FnOnce(
            &[ConsultationCollectionSlot],
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        self.record_disagreement_from(
            ConsultationCollectionCurrent::Prepared(current),
            slots,
            evidence,
            recorded_at,
            limits,
            admit,
        )
    }

    fn record_disagreement_from<Admit>(
        &self,
        current: ConsultationCollectionCurrent<'_>,
        mut slots: Vec<ConsultationSlotId>,
        evidence: ImmutableRecordReferenceV1,
        recorded_at: UnixTimeV1,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<Self, ConsultationCollectionError>
    where
        Admit: FnOnce(
            &[ConsultationCollectionSlot],
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        if self.stop.is_some() || recorded_at > self.deadline {
            return Err(ConsultationCollectionError::Stopped);
        }
        slots.sort();
        slots.dedup();
        if slots.len() < 2 || slots.len() > limits.maximum_slots {
            return Err(ConsultationCollectionError::Disagreement);
        }
        let selected = slots
            .iter()
            .map(|id| {
                self.slots
                    .iter()
                    .find(|slot| slot.roster.slot_id() == id)
                    .filter(|slot| {
                        matches!(
                            slot.disposition,
                            ConsultationSlotDisposition::Fulfilled { .. }
                        )
                    })
                    .cloned()
                    .ok_or(ConsultationCollectionError::Disagreement)
            })
            .collect::<Result<Vec<_>, _>>()?;
        admit(&selected, &evidence)?;
        let mut successor = self.successor(
            current,
            recorded_at,
            limits,
            ConsultationSuccessorHorizon::Execution,
        )?;
        if successor.disagreements.len() >= limits.maximum_slots
            || successor
                .disagreements
                .iter()
                .any(|existing| existing.slots == slots)
        {
            return Err(ConsultationCollectionError::Disagreement);
        }
        successor.disagreements.push(ConsultationDisagreement {
            slots,
            evidence,
            recorded_at,
        });
        Ok(successor)
    }

    /// Stop collection under one explicitly admitted policy or boundary reason.
    ///
    /// The runtime computes missing sets mechanically. The callback decides
    /// whether the selected higher-level policy permits this stop. Stopping does
    /// not accept advice, synthesize an answer, or make missing evidence present.
    /// An operation-expiry stop may record the observed boundary after execution
    /// expired, but only before the parent handle's retention boundary.
    ///
    /// # Errors
    /// Rejects unlike frontier, time, repeated stop, premature deadline/expiry,
    /// a non-expiry stop after execution expired, or a stop rejected by the
    /// selected policy owner.
    pub fn stop<Admit>(
        &self,
        current: &OperationCollectionJournal,
        cause: ConsultationStopCause,
        reason: ImmutableRecordReferenceV1,
        stopped_at: UnixTimeV1,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<Self, ConsultationCollectionError>
    where
        Admit: FnOnce(
            &ConsultationCollection,
            &ConsultationRequirementSummary,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        self.stop_from(
            ConsultationCollectionCurrent::Prepared(current),
            cause,
            reason,
            stopped_at,
            limits,
            admit,
        )
    }

    fn stop_from<Admit>(
        &self,
        current: ConsultationCollectionCurrent<'_>,
        cause: ConsultationStopCause,
        reason: ImmutableRecordReferenceV1,
        stopped_at: UnixTimeV1,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<Self, ConsultationCollectionError>
    where
        Admit: FnOnce(
            &ConsultationCollection,
            &ConsultationRequirementSummary,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        if self.stop.is_some()
            || (cause == ConsultationStopCause::Deadline && stopped_at < self.deadline)
            || (cause == ConsultationStopCause::OperationExpiry && stopped_at < self.expires_at)
            || (cause != ConsultationStopCause::OperationExpiry && stopped_at > self.expires_at)
        {
            return Err(ConsultationCollectionError::Stopped);
        }
        let horizon = if cause == ConsultationStopCause::OperationExpiry {
            ConsultationSuccessorHorizon::Retention
        } else {
            ConsultationSuccessorHorizon::Execution
        };
        let mut successor = self.successor(current, stopped_at, limits, horizon)?;
        let summary = self.requirement_summary();
        admit(self, &summary)?;
        successor.stop = Some(ConsultationCollectionStop {
            cause,
            reason,
            stopped_at,
            summary,
        });
        Ok(successor)
    }

    fn requirement_summary(&self) -> ConsultationRequirementSummary {
        let mut summary = ConsultationRequirementSummary {
            requirements_met: true,
            required_outstanding: Vec::new(),
            required_unfulfillable: Vec::new(),
            optional_outstanding: Vec::new(),
            optional_unfulfillable: Vec::new(),
        };
        for slot in &self.slots {
            let target = match (slot.roster.requirement, &slot.disposition) {
                (_, ConsultationSlotDisposition::Fulfilled { .. }) => continue,
                (ConsultationRequirement::Required, ConsultationSlotDisposition::Outstanding) => {
                    summary.requirements_met = false;
                    &mut summary.required_outstanding
                }
                (
                    ConsultationRequirement::Required,
                    ConsultationSlotDisposition::Unfulfillable { .. },
                ) => {
                    summary.requirements_met = false;
                    &mut summary.required_unfulfillable
                }
                (ConsultationRequirement::Optional, ConsultationSlotDisposition::Outstanding) => {
                    &mut summary.optional_outstanding
                }
                (
                    ConsultationRequirement::Optional,
                    ConsultationSlotDisposition::Unfulfillable { .. },
                ) => &mut summary.optional_unfulfillable,
            };
            target.push(slot.roster.slot_id.clone());
        }
        summary
    }

    fn successor(
        &self,
        current: ConsultationCollectionCurrent<'_>,
        updated_at: UnixTimeV1,
        limits: ConsultationCollectionLimits,
        horizon: ConsultationSuccessorHorizon,
    ) -> Result<Self, ConsultationCollectionError> {
        let predecessor_frontier = current.frontier();
        let exceeds_time_horizon = match horizon {
            ConsultationSuccessorHorizon::Execution => updated_at > self.expires_at,
            ConsultationSuccessorHorizon::Retention => updated_at >= self.retain_until,
        };
        if current.journal_payload() != self.canonical_bytes(limits)?.as_slice()
            || predecessor_frontier.coordinate().record_ref() == &self.parent.record_ref
            || predecessor_frontier.schema_id().as_str()
                != pale_ports::PRIVATE_OPERATION_COLLECTION_FRONTIER_V1_SCHEMA
            || updated_at < self.updated_at
            || exceeds_time_horizon
        {
            return Err(ConsultationCollectionError::Association);
        }
        let predecessor = immutable_frontier_reference(predecessor_frontier)?;
        if self.predecessor_frontier.as_ref() == Some(&predecessor) {
            return Err(ConsultationCollectionError::Association);
        }
        let mut successor = self.clone();
        successor.predecessor_frontier = Some(predecessor);
        successor.updated_at = updated_at;
        Ok(successor)
    }

    fn canonical_bytes(
        &self,
        limits: ConsultationCollectionLimits,
    ) -> Result<BoundedAuthorityEntryBytes, ConsultationCollectionError> {
        BoundedAuthorityEntryBytes::new(
            canon::to_canonical_string(self)?.into_bytes(),
            limits.maximum_journal_bytes,
        )
        .map_err(Into::into)
    }
}

fn immutable_frontier_reference(
    frontier: &ControlRecord,
) -> Result<ImmutableRecordReferenceV1, ConsultationCollectionError> {
    Ok(ImmutableRecordReferenceV1 {
        record_ref: frontier.coordinate().record_ref().clone(),
        record_sha256: ArtifactDigest::parse(frontier.coordinate().record_sha256().as_str())
            .map_err(|_| ConsultationCollectionError::Association)?,
    })
}

/// Exact retained collection head used for one successor compare-and-swap.
#[derive(Debug, Clone, Copy)]
pub struct ConsultationCollectionPredecessor<'a> {
    frontier: &'a ControlRecord,
    journal_sequence: JournalSequenceV1,
    store: &'a pale_ports::AuthorityStoreCoordinate,
}

/// Current application-admitted roster and its exact physical recovery witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredConsultationCollection {
    collection: ConsultationCollection,
    snapshot: OperationCollectionSnapshot,
    store: pale_ports::AuthorityStoreCoordinate,
}

#[derive(Clone, Copy)]
enum ConsultationCollectionCurrent<'a> {
    Prepared(&'a OperationCollectionJournal),
    Recovered(&'a OperationCollectionSnapshot),
}

#[derive(Clone, Copy)]
enum ConsultationSuccessorHorizon {
    Execution,
    Retention,
}

impl ConsultationCollectionCurrent<'_> {
    const fn frontier(&self) -> &ControlRecord {
        match self {
            Self::Prepared(journal) => journal.frontier(),
            Self::Recovered(snapshot) => snapshot.frontier(),
        }
    }

    fn journal_payload(&self) -> &[u8] {
        match self {
            Self::Prepared(journal) => journal.append().entry().bytes().as_slice(),
            Self::Recovered(snapshot) => snapshot.journal_payload().as_slice(),
        }
    }
}

impl RecoveredConsultationCollection {
    /// Borrow the re-admitted application-owned collection.
    #[must_use]
    pub const fn collection(&self) -> &ConsultationCollection {
        &self.collection
    }

    /// Borrow the exact native point-read witness for successor custody.
    #[must_use]
    pub const fn snapshot(&self) -> &OperationCollectionSnapshot {
        &self.snapshot
    }

    /// Return the exact immutable identity of this recovered roster head.
    ///
    /// This derives identity from the admitted snapshot. It does not plan work,
    /// apply consultation policy, or read storage.
    pub(in crate::resident) fn frontier_reference(
        &self,
    ) -> Result<ImmutableRecordReferenceV1, ConsultationCollectionError> {
        immutable_frontier_reference(self.snapshot.frontier())
    }

    /// Borrow the exact parent store incarnation used for roster recovery.
    #[must_use]
    pub const fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        &self.store
    }

    /// Select the outstanding fresh children from this exact recovered roster.
    ///
    /// The result preserves roster order and all recipient-specific coordinates.
    /// It performs no dispatch and grants no new authority. The actual child
    /// service must recheck its current operation authority immediately before
    /// release. An empty batch is valid; the admitted completion policy decides
    /// whether collection should continue or stop.
    ///
    /// # Errors
    /// Rejects time reversal, stopped collection, or planning at or after the
    /// consultation deadline.
    pub fn plan_child_dispatch(
        &self,
        now: UnixTimeV1,
    ) -> Result<ConsultationChildDispatchBatch, ConsultationCollectionError> {
        if now < self.collection.updated_at {
            return Err(ConsultationCollectionError::Association);
        }
        if self.collection.stop.is_some() || now >= self.collection.deadline {
            return Err(ConsultationCollectionError::Stopped);
        }
        let candidates = self
            .collection
            .slots
            .iter()
            .filter_map(|slot| {
                if !matches!(slot.disposition, ConsultationSlotDisposition::Outstanding) {
                    return None;
                }
                let ConsultationContributionSource::FreshChild { child } = &slot.roster.source
                else {
                    return None;
                };
                Some(ConsultationChildDispatchCandidate {
                    slot_id: slot.roster.slot_id.clone(),
                    child: child.clone(),
                    requirement: slot.roster.requirement,
                    requirement_evidence: slot.roster.requirement_evidence.clone(),
                    participant: slot.roster.participant.clone(),
                    provider_binding: slot.roster.provider_binding.clone(),
                    context: slot.roster.context.clone(),
                })
            })
            .collect();
        Ok(ConsultationChildDispatchBatch {
            store: self.store.clone(),
            parent: self.collection.parent.clone(),
            collection_frontier: immutable_frontier_reference(self.snapshot.frontier())?,
            deadline: self.collection.deadline,
            candidates,
        })
    }

    /// Recheck a previously planned child batch against this recovered head.
    ///
    /// Planning does not reserve a roster or grant dispatch authority. A
    /// caller that keeps a batch while the parent changes must recover the
    /// current collection and call this method before it releases any child.
    /// The returned batch is reconstructed from the current roster; it carries
    /// no authority beyond the individual child services' own admission.
    ///
    /// # Errors
    /// Rejects a changed collection frontier, candidate set or deadline, and
    /// returns the same time and stop failures as [`Self::plan_child_dispatch`].
    pub fn revalidate_child_dispatch(
        &self,
        planned: &ConsultationChildDispatchBatch,
        now: UnixTimeV1,
    ) -> Result<ConsultationChildDispatchBatch, ConsultationCollectionError> {
        let current = self.plan_child_dispatch(now)?;
        if current != *planned {
            return Err(ConsultationCollectionError::Association);
        }
        Ok(current)
    }

    /// Admit one fresh child contribution against this recovered exact head.
    ///
    /// # Errors
    /// Returns the same semantic rejections as
    /// [`ConsultationCollection::admit_child_contribution`].
    pub fn admit_child_contribution<Admit>(
        &self,
        contribution: ConsultationContribution,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<ConsultationCollection, ConsultationCollectionError>
    where
        Admit: FnOnce(
            &ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        self.collection.admit_child_contribution_from(
            ConsultationCollectionCurrent::Recovered(&self.snapshot),
            contribution,
            limits,
            admit,
        )
    }

    /// Join one conclusively recovered child into its exact consultation slot.
    ///
    /// A successful child contributes the result named by its terminal
    /// observation. A failed or cancelled child makes the slot unfulfillable
    /// with that observation's diagnostic. Expiry, recovery and an unknown
    /// outcome remain open histories and cannot be promoted into either state.
    /// The callback re-admits the selected result or diagnostic under the
    /// consultation policy; this method does not read its body or synthesize a
    /// parent result.
    ///
    /// # Errors
    /// Rejects a child outside the slot, a substituted recovery, a non-terminal
    /// head, time reversal, an invalid collection successor, or evidence refused
    /// by the policy owner.
    pub fn admit_recovered_child_terminal<Admit>(
        &self,
        slot_id: &ConsultationSlotId,
        child: &RecoveredOperationObservation,
        recorded_at: UnixTimeV1,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<ConsultationCollection, ConsultationCollectionError>
    where
        Admit: FnOnce(
            &ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (_slot, child, disposition) =
            self.recovered_child_terminal(slot_id, child, recorded_at)?;
        match disposition {
            LateConsultationDisposition::Fulfilled { evidence } => {
                self.collection.admit_child_contribution_from(
                    ConsultationCollectionCurrent::Recovered(&self.snapshot),
                    ConsultationContribution {
                        slot_id: slot_id.clone(),
                        child,
                        evidence,
                        recorded_at,
                    },
                    limits,
                    admit,
                )
            }
            LateConsultationDisposition::Unfulfillable { reason } => {
                self.collection.mark_unfulfillable_from(
                    ConsultationCollectionCurrent::Recovered(&self.snapshot),
                    slot_id,
                    reason,
                    recorded_at,
                    limits,
                    admit,
                )
            }
        }
    }

    /// Determine whether one exact recovered child terminal is already retained.
    ///
    /// This check is mechanical. The current collection was already admitted by
    /// its semantic owner. An unlike terminal disposition is a conflict, not an
    /// absent contribution that can be replayed.
    pub(in crate::resident) fn child_terminal_retention(
        &self,
        slot_id: &ConsultationSlotId,
        child: &RecoveredOperationObservation,
    ) -> Result<ConsultationChildTerminalRetention, ConsultationCollectionError> {
        let recorded_at = child.selected_head().1.observed_at();
        let (slot, expected_child, expected_disposition) =
            self.recovered_child_terminal(slot_id, child, *recorded_at)?;
        let retained = match slot.disposition() {
            ConsultationSlotDisposition::Outstanding => self
                .collection
                .late_outcomes
                .iter()
                .find(|late| late.slot_id() == slot_id && late.child() == &expected_child)
                .map(LateConsultationOutcome::disposition),
            ConsultationSlotDisposition::Fulfilled { evidence, .. } => {
                let retained = LateConsultationDisposition::Fulfilled {
                    evidence: evidence.clone(),
                };
                if retained == expected_disposition {
                    return Ok(ConsultationChildTerminalRetention::Exact);
                }
                return Err(ConsultationCollectionError::Contribution);
            }
            ConsultationSlotDisposition::Unfulfillable { reason, .. } => {
                let retained = LateConsultationDisposition::Unfulfillable {
                    reason: reason.clone(),
                };
                if retained == expected_disposition {
                    return Ok(ConsultationChildTerminalRetention::Exact);
                }
                return Err(ConsultationCollectionError::Contribution);
            }
        };
        match retained {
            None => Ok(ConsultationChildTerminalRetention::Absent),
            Some(retained) if retained == &expected_disposition => {
                Ok(ConsultationChildTerminalRetention::Exact)
            }
            Some(_) => Err(ConsultationCollectionError::Contribution),
        }
    }

    fn recovered_child_terminal<'collection>(
        &'collection self,
        slot_id: &ConsultationSlotId,
        child: &RecoveredOperationObservation,
        recorded_at: UnixTimeV1,
    ) -> Result<
        (
            &'collection ConsultationCollectionSlot,
            ImmutableRecordReferenceV1,
            LateConsultationDisposition,
        ),
        ConsultationCollectionError,
    > {
        let slot = self
            .collection
            .slots
            .iter()
            .find(|slot| slot.roster.slot_id() == slot_id)
            .ok_or(ConsultationCollectionError::Slot)?;
        let ConsultationContributionSource::FreshChild { child: expected } = &slot.roster.source
        else {
            return Err(ConsultationCollectionError::Contribution);
        };
        let (_, terminal) = child.selected_head();
        if child.store() != &self.store
            || child.handle.record_ref() != &expected.record_ref
            || child.handle.record_sha256().as_str() != expected.record_sha256.as_str()
            || terminal.operation_handle_ref() != &expected.record_ref
            || terminal.operation_handle_sha256().as_str() != expected.record_sha256.as_str()
            || recorded_at < *terminal.observed_at()
        {
            return Err(ConsultationCollectionError::Child);
        }
        let disposition = match terminal.state() {
            OperationObservationStateV1::Succeeded { result, .. } => {
                LateConsultationDisposition::Fulfilled {
                    evidence: ImmutableRecordReferenceV1 {
                        record_ref: result.record_ref.clone(),
                        record_sha256: ArtifactDigest::parse(result.record_sha256.as_str())
                            .map_err(|_| ConsultationCollectionError::Contribution)?,
                    },
                }
            }
            OperationObservationStateV1::Failed { diagnostic, .. }
            | OperationObservationStateV1::Cancelled { diagnostic } => {
                LateConsultationDisposition::Unfulfillable {
                    reason: diagnostic.clone(),
                }
            }
            OperationObservationStateV1::Active { .. }
            | OperationObservationStateV1::CancellationRequested { .. }
            | OperationObservationStateV1::Recovering { .. }
            | OperationObservationStateV1::OutcomeUnknown { .. }
            | OperationObservationStateV1::Expired { .. } => {
                return Err(ConsultationCollectionError::Contribution);
            }
        };
        Ok((slot, expected.clone(), disposition))
    }

    /// Mark one outstanding slot unfulfillable against this recovered exact head.
    ///
    /// # Errors
    /// Returns the same semantic rejections as [`ConsultationCollection::mark_unfulfillable`].
    pub fn mark_unfulfillable<Admit>(
        &self,
        slot_id: &ConsultationSlotId,
        reason: ImmutableRecordReferenceV1,
        recorded_at: UnixTimeV1,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<ConsultationCollection, ConsultationCollectionError>
    where
        Admit: FnOnce(
            &ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        self.collection.mark_unfulfillable_from(
            ConsultationCollectionCurrent::Recovered(&self.snapshot),
            slot_id,
            reason,
            recorded_at,
            limits,
            admit,
        )
    }

    /// Record disagreement against this recovered exact head.
    ///
    /// # Errors
    /// Returns the same semantic rejections as [`ConsultationCollection::record_disagreement`].
    pub fn record_disagreement<Admit>(
        &self,
        slots: Vec<ConsultationSlotId>,
        evidence: ImmutableRecordReferenceV1,
        recorded_at: UnixTimeV1,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<ConsultationCollection, ConsultationCollectionError>
    where
        Admit: FnOnce(
            &[ConsultationCollectionSlot],
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        self.collection.record_disagreement_from(
            ConsultationCollectionCurrent::Recovered(&self.snapshot),
            slots,
            evidence,
            recorded_at,
            limits,
            admit,
        )
    }

    /// Stop collection against this recovered exact head.
    ///
    /// # Errors
    /// Returns the same semantic rejections as [`ConsultationCollection::stop`].
    pub fn stop<Admit>(
        &self,
        cause: ConsultationStopCause,
        reason: ImmutableRecordReferenceV1,
        stopped_at: UnixTimeV1,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<ConsultationCollection, ConsultationCollectionError>
    where
        Admit: FnOnce(
            &ConsultationCollection,
            &ConsultationRequirementSummary,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        self.collection.stop_from(
            ConsultationCollectionCurrent::Recovered(&self.snapshot),
            cause,
            reason,
            stopped_at,
            limits,
            admit,
        )
    }
}

impl<'a> ConsultationCollectionPredecessor<'a> {
    /// Borrow a collection head prepared or committed in this process.
    #[must_use]
    pub fn prepared(journal: &'a OperationCollectionJournal) -> Self {
        Self {
            frontier: journal.frontier(),
            journal_sequence: journal.append().entry().sequence(),
            store: journal.append().store(),
        }
    }

    /// Borrow a collection head recovered through native point custody.
    #[must_use]
    pub fn recovered(recovered: &'a RecoveredConsultationCollection) -> Self {
        Self {
            frontier: recovered.snapshot.frontier(),
            journal_sequence: recovered.snapshot.evidence().journal_sequence(),
            store: &recovered.store,
        }
    }
}

/// Fresh signed-observation input for one collection state.
///
/// Physical custody derives the private frontier reference from its canonical
/// bytes; callers select only the public observation reference.
pub struct ConsultationCollectionObservationInput<'a> {
    /// Fresh immutable signed observation reference.
    pub observation_ref: ResidentRecordRefV1,
    /// Exact current collection head; `None` is valid only for the first roster.
    pub predecessor_collection: Option<ConsultationCollectionPredecessor<'a>>,
    /// Time at which this collection state was established.
    pub observed_at: UnixTimeV1,
    /// Current admitted runtime signer for the observation.
    pub signing: SignedResidentAdmissionContextV1<'a>,
}

/// Signed collection observation retaining its exact private journal pair.
#[must_use = "prepare, issue, and commit the collection checkpoint before reporting it"]
pub struct PreparedConsultationCollection {
    basis: ConsultationCollectionPreparationBasis,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    associated_request: Option<ControlRecord>,
    predecessor_collection_frontier: Option<ControlRecord>,
    journal: OperationCollectionJournal,
}

/// Immutable operation facts needed after semantic preparation completes.
///
/// This private basis excludes current session authority and every mutable
/// operation phase. It retains only the exact signed handle coordinate and
/// checkpoint selection admitted by the issued local operation. Custody can
/// therefore continue after operation recovery leaves scope without cloning
/// the broader authority-bearing locator.
struct ConsultationCollectionPreparationBasis {
    handle_ref: ResidentRecordRefV1,
    handle: OperationHandleV1,
    checkpoint_selection: super::checkpoint_profile::OperationCheckpointSelection,
}

impl ConsultationCollectionPreparationBasis {
    fn from_operation(operation: &IssuedLocalOperation) -> Self {
        let (handle_ref, handle) = operation.handle();
        Self {
            handle_ref: handle_ref.clone(),
            handle: handle.clone(),
            checkpoint_selection: operation.checkpoint_selection(),
        }
    }
}

enum ConsultationObservationMode {
    Collection,
    Cancellation {
        request: ImmutableRecordReferenceV1,
        record: ControlRecord,
    },
}

impl ConsultationCollection {
    /// Prepare one predecessor-linked collection frontier and signed observation.
    ///
    /// The admission callback owns the selected completion policy and confirms
    /// that existing evidence may satisfy its named slots. This function does
    /// not start children, judge sufficiency, or synthesize. An initial snapshot
    /// follows `Resolving`; every successor names the exact previous collection
    /// frontier in its private payload. Before a stop, the signed observation
    /// also names the current frontier. After a stop, retained successors keep
    /// the stopped observation payload unchanged while the private point index
    /// advances to late outcomes. They are unavailable at or beyond retention.
    ///
    /// # Errors
    /// Rejects unlike operation, predecessor, store, signer, journal sequence,
    /// policy admission, bounds, or signature. No write occurs.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_observation<Admit, Rotation, Sign>(
        &self,
        operation: &IssuedLocalOperation,
        checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
        input: ConsultationCollectionObservationInput<'_>,
        limits: ConsultationCollectionLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        admit_collection: Admit,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedConsultationCollection, ConsultationCollectionError>
    where
        Admit: FnOnce(&Self) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.prepare_observation_with_mode(
            operation,
            checkpoint,
            predecessor,
            input,
            ConsultationObservationMode::Collection,
            limits,
            resolve_store,
            custody,
            admit_collection,
            admit_rotation,
            sign,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn prepare_cancellation_observation<Admit, Rotation, Sign>(
        &self,
        operation: &IssuedLocalOperation,
        checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
        input: ConsultationCollectionObservationInput<'_>,
        request: ImmutableRecordReferenceV1,
        request_record: ControlRecord,
        limits: ConsultationCollectionLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        admit_collection: Admit,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedConsultationCollection, ConsultationCollectionError>
    where
        Admit: FnOnce(&Self) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.prepare_observation_with_mode(
            operation,
            checkpoint,
            predecessor,
            input,
            ConsultationObservationMode::Cancellation {
                request,
                record: request_record,
            },
            limits,
            resolve_store,
            custody,
            admit_collection,
            admit_rotation,
            sign,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_observation_with_mode<Admit, Rotation, Sign>(
        &self,
        operation: &IssuedLocalOperation,
        checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
        input: ConsultationCollectionObservationInput<'_>,
        mode: ConsultationObservationMode,
        limits: ConsultationCollectionLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        admit_collection: Admit,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedConsultationCollection, ConsultationCollectionError>
    where
        Admit: FnOnce(&Self) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let handle = operation.handle();
        let cancellation_predecessor = match predecessor.1.state() {
            OperationObservationStateV1::CancellationRequested { request } => Some(request),
            _ => None,
        };
        let predecessor_matches = match &self.predecessor_frontier {
            None => matches!(
                predecessor.1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving
                }
            ),
            Some(immediate_predecessor) => {
                matches!(
                    predecessor.1.state(),
                    OperationObservationStateV1::Active {
                        phase:
                            OperationActivePhaseV1::CollectingConsultation { evidence }
                            | OperationActivePhaseV1::ConsultationCollected { evidence }
                    } if evidence == self
                        .frozen_stop_frontier
                        .as_ref()
                        .unwrap_or(immediate_predecessor)
                ) || cancellation_predecessor.is_some_and(|request| {
                    self.stop.as_ref().is_some_and(|stop| {
                        stop.cause == ConsultationStopCause::Cancellation && &stop.reason == request
                    })
                })
            }
        };
        if self.parent != immutable_operation_reference(handle.0, handle.1)?
            || handle.1.operation_class() != &OperationClassV1::Consult
            || !predecessor_matches
            || self.predecessor_frontier.is_some() != input.predecessor_collection.is_some()
            || input.observed_at < *predecessor.1.observed_at()
            || input.observed_at < self.updated_at
            || if self.stop.is_some() {
                input.observed_at >= *handle.1.retain_until()
            } else {
                input.observed_at > *handle.1.expires_at()
            }
        {
            return Err(ConsultationCollectionError::Association);
        }
        if let Some(previous) = input.predecessor_collection {
            let expected = self
                .predecessor_frontier
                .as_ref()
                .ok_or(ConsultationCollectionError::Association)?;
            if previous.frontier.coordinate().record_ref() != &expected.record_ref
                || previous.frontier.coordinate().record_sha256().as_str()
                    != expected.record_sha256.as_str()
            {
                return Err(ConsultationCollectionError::Association);
            }
        }
        let store = resolve_store.resolve(predecessor.1)?;
        if input
            .predecessor_collection
            .is_some_and(|previous| previous.store != &store)
        {
            return Err(ConsultationCollectionError::Association);
        }
        admit_collection(self)?;
        let transaction = custody.prepare_transaction(&store)?;
        let expected_journal_head = input
            .predecessor_collection
            .map(|previous| previous.journal_sequence);
        let sequence = match expected_journal_head {
            None => JournalSequenceV1::new(1).map_err(|_| ConsultationCollectionError::Bounds)?,
            Some(head) => head
                .checked_successor()
                .ok_or(ConsultationCollectionError::Bounds)?,
        };
        let append = PreparedAuthorityAppend::new(
            store,
            expected_journal_head,
            SequencedAuthorityEntry::new(sequence, self.canonical_bytes(limits)?),
        )?;
        let handle_coordinate = ControlRecordCoordinate::new(
            handle.0.clone(),
            ControlRecordDigest::parse(handle.1.canonical_operation_handle_sha256().as_str())?,
        );
        let journal = OperationCollectionJournal::prepare(
            &handle_coordinate,
            transaction,
            append,
            limits.maximum_frontier_bytes,
            limits.maximum_journal_bytes,
        )?;
        let evidence = immutable_frontier_reference(journal.frontier())?;
        let (state, associated_request) = match mode {
            ConsultationObservationMode::Cancellation { request, record } => {
                if !matches!(
                    predecessor.1.state(),
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::CollectingConsultation { .. }
                    }
                ) || self.stop.as_ref().is_none_or(|stop| {
                    stop.cause != ConsultationStopCause::Cancellation || stop.reason != request
                }) {
                    return Err(ConsultationCollectionError::Association);
                }
                (
                    OperationObservationStateV1::CancellationRequested { request },
                    Some(record),
                )
            }
            ConsultationObservationMode::Collection => {
                let state = if let Some(request) = cancellation_predecessor {
                    if self.stop.as_ref().is_none_or(|stop| {
                        stop.cause != ConsultationStopCause::Cancellation || &stop.reason != request
                    }) {
                        return Err(ConsultationCollectionError::Association);
                    }
                    OperationObservationStateV1::CancellationRequested {
                        request: request.clone(),
                    }
                } else {
                    let phase = if self.stop.is_some() {
                        OperationActivePhaseV1::ConsultationCollected {
                            evidence: self.frozen_stop_frontier.clone().unwrap_or(evidence),
                        }
                    } else {
                        OperationActivePhaseV1::CollectingConsultation { evidence }
                    };
                    OperationObservationStateV1::Active { phase }
                };
                (state, None)
            }
        };
        let observation = signing::issue_observation(
            &OperationObservationAdmissionContextV1 {
                signed: input.signing,
                handle_ref: handle.0,
                handle: handle.1,
                predecessor: Some(predecessor),
            },
            state,
            input.observed_at,
            admit_rotation,
            sign,
        )?;
        Ok(PreparedConsultationCollection {
            basis: ConsultationCollectionPreparationBasis::from_operation(operation),
            checkpoint: (checkpoint.0.clone(), checkpoint.1.clone()),
            predecessor: (predecessor.0.clone(), predecessor.1.clone()),
            observation_ref: input.observation_ref,
            observation,
            associated_request,
            predecessor_collection_frontier: input
                .predecessor_collection
                .map(|previous| previous.frontier.clone()),
            journal,
        })
    }
}

impl PreparedConsultationCollection {
    /// Borrow the signed collection observation before custody.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Borrow the exact private frontier and journal pair.
    #[must_use]
    pub const fn journal(&self) -> &OperationCollectionJournal {
        &self.journal
    }

    /// Prepare roots with the same transaction that derived the frontier.
    ///
    /// # Errors
    /// Rejects unlike lineage, checkpoint, store, transaction, roots, or bounds.
    fn prepare_checkpoint<'a>(
        &'a self,
        limits: AccumulatorPreparationLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<PreparedConsultationCollectionCheckpoint<'a>, ConsultationCollectionError> {
        let prepared = prepare_operation_observation_append(
            OperationObservationTransition::new(
                &self.basis.handle_ref,
                &self.basis.handle,
                &self.predecessor.0,
                &self.predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?;
        let prepared = match &self.associated_request {
            Some(request) => prepared
                .with_associated_record(OperationCustodyRecord::CommonRequest(request.clone())),
            None => prepared,
        }
        .prepare_checkpoint_with_transaction(
            self.basis.checkpoint_selection.clone(),
            &self.checkpoint.0,
            &self.checkpoint.1,
            limits,
            self.journal.transaction().clone(),
            custody,
        )?;
        Ok(PreparedConsultationCollectionCheckpoint {
            prepared,
            journal: &self.journal,
            predecessor_collection_frontier: self.predecessor_collection_frontier.as_ref(),
        })
    }
}

/// Prepared collection roots retaining the journal that must join custody.
struct PreparedConsultationCollectionCheckpoint<'a> {
    prepared: PreparedOperationObservationCheckpoint<'a>,
    journal: &'a OperationCollectionJournal,
    predecessor_collection_frontier: Option<&'a ControlRecord>,
}

impl PreparedConsultationCollectionCheckpoint<'_> {
    /// Borrow the ordinary observation-checkpoint preparation.
    const fn prepared(&self) -> &PreparedOperationObservationCheckpoint<'_> {
        &self.prepared
    }

    /// Issue the checkpoint and attach the exact journal/frontier pair.
    ///
    /// # Errors
    /// Preserves checkpoint issuance, candidate, and anchor-preparation failures.
    fn issue_checkpoint<Sign>(
        self,
        successor_ref: &ResidentRecordRefV1,
        preimage: ControlStateCheckpointPreimageV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<
        (
            PreparedOperationObservationCustody,
            ControlStateCheckpointV1,
        ),
        ConsultationCollectionError,
    >
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let (custody, checkpoint) =
            self.prepared
                .issue_checkpoint(successor_ref, preimage, schema, authority, sign)?;
        let custody = match self.predecessor_collection_frontier {
            Some(predecessor) => custody
                .with_collection_journal_after(self.journal.clone(), Some(predecessor.clone()))?,
            None => custody.with_collection_journal(self.journal.clone())?,
        };
        Ok((custody, checkpoint))
    }
}

fn immutable_operation_reference(
    reference: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
) -> Result<ImmutableRecordReferenceV1, ConsultationCollectionError> {
    Ok(ImmutableRecordReferenceV1 {
        record_ref: reference.clone(),
        record_sha256: ArtifactDigest::parse(handle.canonical_operation_handle_sha256().as_str())
            .map_err(|_| ConsultationCollectionError::Association)?,
    })
}

/// Failure before an exact durable consultation collection exists.
#[derive(Debug, thiserror::Error)]
pub enum ConsultationCollectionError {
    /// A configured count, text, time, sequence, or byte ceiling is invalid.
    #[error("consultation collection exceeds an explicit bound")]
    Bounds,
    /// The parent, predecessor, deadline, checkpoint, or store differs.
    #[error("consultation collection does not belong to the exact Consult operation")]
    Association,
    /// Two slots use one private identity.
    #[error("consultation collection repeats a slot identity")]
    DuplicateSlot,
    /// Two slots claim one fresh child.
    #[error("consultation collection repeats a child operation")]
    DuplicateChild,
    /// A requested slot is not part of the admitted roster.
    #[error("consultation collection does not contain the requested slot")]
    Slot,
    /// A fresh child is absent, unlike, delegated, or belongs to another parent.
    #[error("consultation child does not belong to the exact Consult parent")]
    Child,
    /// A contribution repeats or contradicts the slot's retained disposition.
    #[error("consultation contribution conflicts with the retained slot state")]
    Contribution,
    /// Disagreement evidence does not name distinct fulfilled contributions.
    #[error("consultation disagreement does not match fulfilled roster slots")]
    Disagreement,
    /// Collection is already stopped or the requested boundary has not occurred.
    #[error("consultation collection cannot stop or change in its current state")]
    Stopped,
    /// Canonical private encoding failed.
    #[error(transparent)]
    Canonical(#[from] canon::CanonError),
    /// Private bytes were valid JSON but not the exact collection encoding.
    #[error("consultation collection bytes are not the exact canonical encoding")]
    Decoding,
    /// A typed digest, sequence, signature, or record invariant failed.
    #[error(transparent)]
    Semantic(#[from] InvalidResidentRecordV1),
    /// Physical bounded journal bytes or sequencing were rejected.
    #[error(transparent)]
    Journal(#[from] DurableAuthorityStoreError),
    /// Private frontier derivation rejected the pair.
    #[error(transparent)]
    CollectionJournal(#[from] OperationCollectionJournalError),
    /// Storage preparation failed before a write.
    #[error(transparent)]
    Store(#[from] pale_ports::ControlStoreError),
    /// Observation issuance rejected current semantics or signing.
    #[error(transparent)]
    Attempt(#[from] OperationAttemptError),
    /// Checkpoint and closed-candidate preparation failed.
    #[error(transparent)]
    Projection(#[from] OperationObservationCustodyProjectionError),
    /// A physical record coordinate was malformed.
    #[error(transparent)]
    Record(#[from] pale_ports::ControlRecordError),
    /// The runtime-authority owner could not resolve one store.
    #[error(transparent)]
    RuntimeAuthority(#[from] super::OperationObservationAuthorityStoreRejection),
}

#[cfg(test)]
#[path = "../../tests/internal/resident/consultation.rs"]
pub(in crate::resident) mod tests;
