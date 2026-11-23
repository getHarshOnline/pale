//! Phase-typed physical custody and one-use continuation evidence.

use pale_ports::{
    ControlStoreError, EventCustodyCommitOutcome, EventCustodyPort, EventCustodyReadBackOutcome,
    EventCustodyTransition, EventCustodyTransitionKind,
};

use super::super::custody_phase::{
    commit_with_exact_readback, DatabaseCommitDisposition, DatabaseCommitEvidence,
    DatabaseCommitFailureReason, DatabaseReadBackDisposition,
};

/// Evidence establishing exact database custody for one Event transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCustodyCommitEvidence {
    /// The adapter committed the complete transition from its predecessor.
    New,
    /// The complete transition already existed before this call.
    ExactReplay,
    /// Same-call read-back after an ambiguous commit found the exact successor.
    RecoveredExactAfterAmbiguousCommit,
}

/// One-use permission to continue beyond newly proven durable custody.
///
/// This is deliberately not `Clone` or `Copy`. Ordinary exact replay never
/// produces a value, so replay cannot advertise an event, duplicate fanout, or
/// repeat a transport send.
#[derive(Debug, PartialEq, Eq)]
pub struct EventCustodyContinuation {
    kind: EventCustodyContinuationKind,
}

/// Closed description of the action permitted by an Event continuation.
///
/// This value describes a continuation but does not grant one. Only an opaque
/// [`EventCustodyContinuation`] returned after newly proven custody authorizes
/// the action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCustodyContinuationKind {
    /// Project and fan out a newly admitted owner reference.
    ProjectOwnerReference,
    /// Attempt one newly retained inbox delivery.
    DeliverInboxAttempt,
    /// Expose one newly retained Pale-derived publication.
    ExposePaleDerivedPublication,
}

impl EventCustodyContinuation {
    const fn new(kind: EventCustodyContinuationKind) -> Self {
        Self { kind }
    }

    /// Return the action authorized by this one-use continuation.
    #[must_use]
    pub const fn kind(&self) -> EventCustodyContinuationKind {
        self.kind
    }
}

/// Prepared semantic transition with no physical-custody proof.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "prepared Event custody must be committed or retained for recovery"]
pub struct PreparedEventCustodyTransition {
    candidate: EventCustodyTransition,
}

impl PreparedEventCustodyTransition {
    pub(in crate::resident) const fn new(candidate: EventCustodyTransition) -> Self {
        Self { candidate }
    }

    /// Borrow the exact physical candidate without granting commit authority.
    #[must_use]
    pub const fn candidate(&self) -> &EventCustodyTransition {
        &self.candidate
    }

    /// Consume the prepared state and attempt one atomic database transition.
    /// The positive payload limit applies to journal comparison and to the
    /// single read-back after an uncertain commit. It does not change candidate
    /// identity. Exceeding the limit during that read-back leaves custody unknown.
    ///
    /// A [`pale_ports::ControlStoreErrorKind::OutcomeUnknown`] result is never retried.
    /// This method performs exactly one [`EventCustodyPort::observe_exact`]
    /// read-back and advances only for the exact successor.
    ///
    /// # Errors
    ///
    /// Returns [`EventCustodyCommitFailure`] with the complete prepared value
    /// whenever exact custody remains unproven.
    pub fn commit(
        self,
        store: &mut dyn EventCustodyPort,
        maximum_journal_payload_bytes: usize,
    ) -> Result<CompletedEventCustodyTransition, EventCustodyCommitFailure> {
        let evidence = match commit_with_exact_readback(
            store,
            &self.candidate,
            |store, candidate| {
                store
                    .compare_and_commit(candidate, maximum_journal_payload_bytes)
                    .map(|outcome| match outcome {
                        EventCustodyCommitOutcome::Committed => {
                            DatabaseCommitDisposition::Committed
                        }
                        EventCustodyCommitOutcome::ExactReplay => {
                            DatabaseCommitDisposition::ExactReplay
                        }
                    })
            },
            |store, candidate| {
                store
                    .observe_exact(candidate, maximum_journal_payload_bytes)
                    .map(|outcome| match outcome {
                        EventCustodyReadBackOutcome::Predecessor => {
                            DatabaseReadBackDisposition::Predecessor
                        }
                        EventCustodyReadBackOutcome::ExactSuccessor => {
                            DatabaseReadBackDisposition::ExactSuccessor
                        }
                        EventCustodyReadBackOutcome::ConflictingOrPartial => {
                            DatabaseReadBackDisposition::ConflictingOrPartial
                        }
                    })
            },
        ) {
            Ok(evidence) => EventCustodyCommitEvidence::from(evidence),
            Err(reason) => return Err(self.failed(reason.into())),
        };

        Ok(self.completed(evidence))
    }

    fn completed(self, evidence: EventCustodyCommitEvidence) -> CompletedEventCustodyTransition {
        let continuation = (!matches!(evidence, EventCustodyCommitEvidence::ExactReplay))
            .then(|| continuation_for(self.candidate.kind()))
            .flatten();
        CompletedEventCustodyTransition {
            candidate: self.candidate,
            evidence,
            continuation,
        }
    }

    fn failed(self, reason: EventCustodyCommitFailureReason) -> EventCustodyCommitFailure {
        EventCustodyCommitFailure {
            prepared: Box::new(self),
            reason,
        }
    }
}

fn continuation_for(kind: EventCustodyTransitionKind) -> Option<EventCustodyContinuation> {
    match kind {
        EventCustodyTransitionKind::AdmitOwnerReference => Some(EventCustodyContinuation::new(
            EventCustodyContinuationKind::ProjectOwnerReference,
        )),
        EventCustodyTransitionKind::EnqueueDelivery | EventCustodyTransitionKind::EnqueueReplay => {
            Some(EventCustodyContinuation::new(
                EventCustodyContinuationKind::DeliverInboxAttempt,
            ))
        }
        EventCustodyTransitionKind::ProjectInteractionOccurrence
        | EventCustodyTransitionKind::ProjectGenealogyEdge
        | EventCustodyTransitionKind::AdvanceReconciliationFrontier => {
            Some(EventCustodyContinuation::new(
                EventCustodyContinuationKind::ExposePaleDerivedPublication,
            ))
        }
        EventCustodyTransitionKind::QuarantineIdentityConflict
        | EventCustodyTransitionKind::AdmitSubscription
        | EventCustodyTransitionKind::RetireSubscription
        | EventCustodyTransitionKind::RecordAcknowledgement
        | EventCustodyTransitionKind::AdvanceDeliveryCursor
        | EventCustodyTransitionKind::AdvanceSourceFrontier
        | EventCustodyTransitionKind::SettlePaleDerivedPublication => None,
    }
}

/// Completed exact database custody and optional affine continuation.
#[derive(Debug, PartialEq, Eq)]
pub struct CompletedEventCustodyTransition {
    candidate: EventCustodyTransition,
    evidence: EventCustodyCommitEvidence,
    continuation: Option<EventCustodyContinuation>,
}

impl CompletedEventCustodyTransition {
    /// Borrow the exact committed transition.
    #[must_use]
    pub const fn candidate(&self) -> &EventCustodyTransition {
        &self.candidate
    }

    /// Return how exact custody was established.
    #[must_use]
    pub const fn evidence(&self) -> EventCustodyCommitEvidence {
        self.evidence
    }

    /// Remove the sole newly earned continuation permission.
    pub fn take_continuation(&mut self) -> Option<EventCustodyContinuation> {
        self.continuation.take()
    }
}

/// Failed commit with the complete prepared value retained.
#[derive(Debug, thiserror::Error)]
#[error("Event custody did not advance: {reason}")]
pub struct EventCustodyCommitFailure {
    prepared: Box<PreparedEventCustodyTransition>,
    reason: EventCustodyCommitFailureReason,
}

impl EventCustodyCommitFailure {
    /// Borrow the exact closed failure reason.
    #[must_use]
    pub const fn reason(&self) -> &EventCustodyCommitFailureReason {
        &self.reason
    }

    /// Recover the unchanged prepared state for explicit policy or recovery.
    pub fn into_prepared(self) -> PreparedEventCustodyTransition {
        *self.prepared
    }
}

/// Closed classification of database custody failure.
#[derive(Debug, thiserror::Error)]
pub enum EventCustodyCommitFailureReason {
    /// Storage rejected the transition before an ambiguous commit boundary.
    #[error(transparent)]
    Store(ControlStoreError),
    /// Exact read-back proved that the predecessor remains current.
    #[error("ambiguous Event commit was excluded by exact read-back")]
    CommitExcludedByReadBack {
        /// Original outcome-unknown error.
        uncertain: ControlStoreError,
    },
    /// Exact read-back found partial or contradictory state.
    #[error("ambiguous Event commit requires explicit recovery")]
    RecoveryRequired {
        /// Original outcome-unknown error.
        uncertain: ControlStoreError,
    },
    /// Mandatory read-back itself failed.
    #[error("exact Event read-back failed after ambiguous commit")]
    ReadBackFailed {
        /// Original outcome-unknown error.
        uncertain: ControlStoreError,
        /// Failure that prevented exact read-back classification.
        #[source]
        read_back: ControlStoreError,
    },
}

impl From<DatabaseCommitEvidence> for EventCustodyCommitEvidence {
    fn from(value: DatabaseCommitEvidence) -> Self {
        match value {
            DatabaseCommitEvidence::New => Self::New,
            DatabaseCommitEvidence::ExactReplay => Self::ExactReplay,
            DatabaseCommitEvidence::RecoveredExactAfterAmbiguousCommit => {
                Self::RecoveredExactAfterAmbiguousCommit
            }
        }
    }
}

impl From<DatabaseCommitFailureReason> for EventCustodyCommitFailureReason {
    fn from(value: DatabaseCommitFailureReason) -> Self {
        match value {
            DatabaseCommitFailureReason::Store(error) => Self::Store(error),
            DatabaseCommitFailureReason::CommitExcludedByReadBack { uncertain } => {
                Self::CommitExcludedByReadBack { uncertain }
            }
            DatabaseCommitFailureReason::RecoveryRequired { uncertain } => {
                Self::RecoveryRequired { uncertain }
            }
            DatabaseCommitFailureReason::ReadBackFailed {
                uncertain,
                read_back,
            } => Self::ReadBackFailed {
                uncertain,
                read_back,
            },
        }
    }
}
