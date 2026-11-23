//! Phase-typed database and anti-rollback custody.

use pale_ports::{
    AntiRollbackAnchorError, AntiRollbackAnchorObservation, AntiRollbackAnchorPort,
    ControlRecordCoordinate, ControlStoreError, EffectCaseCommitOutcome, EffectCaseCustodyPort,
    EffectCaseReadBackOutcome, EffectCaseTransitionCandidate, PreparedAntiRollbackAnchorTransition,
};

use super::super::custody_phase::{
    advance_anchor_with_exact_observation, commit_with_exact_readback, AnchorAdvanceEvidence,
    AnchorAdvanceFailureReason, DatabaseCommitDisposition, DatabaseCommitEvidence,
    DatabaseCommitFailureReason, DatabaseReadBackDisposition,
};

/// Database custody evidence retained independently of anchor evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectCaseDatabaseCommitEvidence {
    /// The database advanced from the exact asserted predecessor.
    New,
    /// The database reported that the exact successor already existed.
    ExactReplay,
    /// One read-back proved the exact successor after an ambiguous commit.
    RecoveredExactAfterAmbiguousCommit,
}

/// External hard-domain anchor evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectCaseAnchorEvidence {
    /// The independent anchor advanced from the exact predecessor.
    Advanced,
    /// The anchor reported that the exact successor already existed.
    ExactReplay,
    /// One observation proved the exact successor after an ambiguous advance.
    RecoveredExactAfterAmbiguousAdvance,
}

/// Prepared semantic transition before any physical mutation is attempted.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "prepared effect-case custody must be committed or retained"]
pub struct PreparedEffectCaseTransition {
    pub(super) candidate: EffectCaseTransitionCandidate,
    pub(super) anchor: Option<PreparedAntiRollbackAnchorTransition>,
    pub(super) dispatch_intent: Option<ControlRecordCoordinate>,
}

/// A committed transition either completes immediately or still requires its anchor.
pub enum EffectCaseCommitSuccess {
    /// An alias transaction completed without a hard-domain checkpoint change.
    Complete(CompletedEffectCaseTransition),
    /// A database commit succeeded and its hard-domain anchor remains pending.
    AnchorPending(Box<DatabaseCommittedEffectCaseAnchorPending>),
}

/// Database-proven state that cannot dispatch until its external anchor is proven.
#[derive(Debug)]
#[must_use = "database custody is not dispatch eligibility"]
pub struct DatabaseCommittedEffectCaseAnchorPending {
    candidate: EffectCaseTransitionCandidate,
    anchor: PreparedAntiRollbackAnchorTransition,
    dispatch_intent: Option<ControlRecordCoordinate>,
    database_evidence: EffectCaseDatabaseCommitEvidence,
}

/// Fully rollback-fenced effect-case transition.
pub struct CompletedEffectCaseTransition {
    candidate: EffectCaseTransitionCandidate,
    database_evidence: EffectCaseDatabaseCommitEvidence,
    anchor_evidence: Option<EffectCaseAnchorEvidence>,
    dispatch: Option<EffectDispatchAuthorization>,
}

/// Affine permission to perform one not-yet-dispatched native attempt.
///
/// The value is deliberately neither [`Clone`] nor [`Copy`]. Consuming it does
/// not prove that a provider accepted bytes or that any native effect occurred.
pub struct EffectDispatchAuthorization {
    intent: ControlRecordCoordinate,
}

impl EffectDispatchAuthorization {
    /// Consume the authorization into the exact durable intent coordinate.
    pub fn into_intent(self) -> ControlRecordCoordinate {
        self.intent
    }
}

impl PreparedEffectCaseTransition {
    /// Borrow the closed physical transaction without mutating custody.
    #[must_use]
    pub const fn candidate(&self) -> &EffectCaseTransitionCandidate {
        &self.candidate
    }

    /// Atomically compare and commit, with mandatory exact read-back on ambiguity.
    ///
    /// # Errors
    ///
    /// Returns [`EffectCaseCommitFailure`] when the store rejects the comparison,
    /// exact read-back excludes the commit, or ambiguity remains unresolved.
    pub fn commit(
        self,
        store: &mut dyn EffectCaseCustodyPort,
    ) -> Result<EffectCaseCommitSuccess, EffectCaseCommitFailure> {
        let evidence = match commit_with_exact_readback(
            store,
            &self.candidate,
            |store, candidate| {
                store
                    .compare_and_commit(candidate)
                    .map(|outcome| match outcome {
                        EffectCaseCommitOutcome::Committed => DatabaseCommitDisposition::Committed,
                        EffectCaseCommitOutcome::ExactReplay => {
                            DatabaseCommitDisposition::ExactReplay
                        }
                    })
            },
            |store, candidate| {
                store.observe_exact(candidate).map(|outcome| match outcome {
                    EffectCaseReadBackOutcome::Predecessor => {
                        DatabaseReadBackDisposition::Predecessor
                    }
                    EffectCaseReadBackOutcome::ExactSuccessor => {
                        DatabaseReadBackDisposition::ExactSuccessor
                    }
                    EffectCaseReadBackOutcome::ConflictingOrPartial => {
                        DatabaseReadBackDisposition::ConflictingOrPartial
                    }
                })
            },
        ) {
            Ok(evidence) => EffectCaseDatabaseCommitEvidence::from(evidence),
            Err(reason) => return Err(EffectCaseCommitFailure::new(self, reason.into())),
        };

        match self.anchor {
            None => Ok(EffectCaseCommitSuccess::Complete(
                CompletedEffectCaseTransition {
                    candidate: self.candidate,
                    database_evidence: evidence,
                    anchor_evidence: None,
                    dispatch: None,
                },
            )),
            Some(anchor) => Ok(EffectCaseCommitSuccess::AnchorPending(Box::new(
                DatabaseCommittedEffectCaseAnchorPending {
                    candidate: self.candidate,
                    anchor,
                    dispatch_intent: self.dispatch_intent,
                    database_evidence: evidence,
                },
            ))),
        }
    }
}

impl DatabaseCommittedEffectCaseAnchorPending {
    /// Advance the independently retained anchor; ambiguous writes are observed once.
    ///
    /// # Errors
    ///
    /// Returns [`EffectCaseAnchorFailure`] when the anchor does not advance to
    /// the exact successor or an ambiguous advance cannot be resolved by one
    /// observation. The returned failure retains the pending phase.
    pub fn advance_anchor(
        self,
        port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedEffectCaseTransition, EffectCaseAnchorFailure> {
        let evidence = match advance_anchor_with_exact_observation(port, &self.anchor) {
            Ok(evidence) => EffectCaseAnchorEvidence::from(evidence),
            Err(reason) => return Err(self.failed(reason.into())),
        };

        Ok(self.completed(evidence))
    }

    fn completed(self, evidence: EffectCaseAnchorEvidence) -> CompletedEffectCaseTransition {
        let dispatch = matches!(
            (self.database_evidence, evidence),
            (
                EffectCaseDatabaseCommitEvidence::New
                    | EffectCaseDatabaseCommitEvidence::RecoveredExactAfterAmbiguousCommit,
                EffectCaseAnchorEvidence::Advanced
                    | EffectCaseAnchorEvidence::RecoveredExactAfterAmbiguousAdvance
            )
        )
        .then(|| self.dispatch_intent.clone())
        .flatten()
        .map(|intent| EffectDispatchAuthorization { intent });
        CompletedEffectCaseTransition {
            candidate: self.candidate,
            database_evidence: self.database_evidence,
            anchor_evidence: Some(evidence),
            dispatch,
        }
    }

    fn failed(self, reason: EffectCaseAnchorFailureReason) -> EffectCaseAnchorFailure {
        EffectCaseAnchorFailure::new(self, reason)
    }
}

impl CompletedEffectCaseTransition {
    /// Borrow the closed physical transaction proven by this phase.
    #[must_use]
    pub const fn candidate(&self) -> &EffectCaseTransitionCandidate {
        &self.candidate
    }

    /// Return how exact database custody was established.
    #[must_use]
    pub const fn database_evidence(&self) -> EffectCaseDatabaseCommitEvidence {
        self.database_evidence
    }

    /// Return how hard-domain anchoring was established, if required.
    #[must_use]
    pub const fn anchor_evidence(&self) -> Option<EffectCaseAnchorEvidence> {
        self.anchor_evidence
    }

    /// Remove the sole affine dispatch permission, when this was a new fenced intent.
    pub fn take_dispatch_authorization(&mut self) -> Option<EffectDispatchAuthorization> {
        self.dispatch.take()
    }
}

/// Failed database custody with the complete prepared value retained.
#[derive(Debug, thiserror::Error)]
#[error("effect-case database custody did not advance: {reason}")]
pub struct EffectCaseCommitFailure {
    prepared: Box<PreparedEffectCaseTransition>,
    reason: EffectCaseCommitFailureReason,
}

impl EffectCaseCommitFailure {
    fn new(prepared: PreparedEffectCaseTransition, reason: EffectCaseCommitFailureReason) -> Self {
        Self {
            prepared: Box::new(prepared),
            reason,
        }
    }

    /// Borrow the closed reason why database custody did not complete.
    #[must_use]
    pub const fn reason(&self) -> &EffectCaseCommitFailureReason {
        &self.reason
    }

    /// Recover the unchanged prepared transition for inspection or later policy.
    pub fn into_prepared(self) -> PreparedEffectCaseTransition {
        *self.prepared
    }
}

/// Closed database failure classification.
#[derive(Debug, thiserror::Error)]
pub enum EffectCaseCommitFailureReason {
    /// The database rejected the comparison or failed before an ambiguous result.
    #[error(transparent)]
    Store(ControlStoreError),
    /// Read-back proved that the transaction did not commit.
    #[error("ambiguous commit read-back proved the predecessor")]
    CommitExcludedByReadBack {
        /// Original outcome-unknown database error.
        uncertain: ControlStoreError,
    },
    /// Read-back observed neither the exact predecessor nor exact successor.
    #[error("ambiguous commit read-back found conflicting or partial state")]
    RecoveryRequired {
        /// Original outcome-unknown database error.
        uncertain: ControlStoreError,
    },
    /// The mandatory read-back itself failed.
    #[error("exact read-back failed after ambiguous commit")]
    ReadBackFailed {
        /// Original outcome-unknown database error.
        uncertain: ControlStoreError,
        /// Read-back failure that prevents classification.
        #[source]
        read_back: ControlStoreError,
    },
}

/// Failed external-anchor advance with database custody retained.
#[derive(Debug, thiserror::Error)]
#[error("effect-case anti-rollback anchor did not advance: {reason}")]
pub struct EffectCaseAnchorFailure {
    pending: Box<DatabaseCommittedEffectCaseAnchorPending>,
    reason: Box<EffectCaseAnchorFailureReason>,
}

impl EffectCaseAnchorFailure {
    fn new(
        pending: DatabaseCommittedEffectCaseAnchorPending,
        reason: EffectCaseAnchorFailureReason,
    ) -> Self {
        Self {
            pending: Box::new(pending),
            reason: Box::new(reason),
        }
    }

    /// Borrow the closed reason why independent anchoring did not complete.
    #[must_use]
    pub fn reason(&self) -> &EffectCaseAnchorFailureReason {
        self.reason.as_ref()
    }

    /// Recover the committed-but-unanchored phase for observe-only handling.
    pub fn into_pending(self) -> DatabaseCommittedEffectCaseAnchorPending {
        *self.pending
    }
}

/// Closed anchor failure classification; every variant remains observe-only.
#[derive(Debug, thiserror::Error)]
pub enum EffectCaseAnchorFailureReason {
    /// The anchor rejected the comparison or failed before ambiguity.
    #[error(transparent)]
    Port(AntiRollbackAnchorError),
    /// The anchor explicitly retained another observable state.
    #[error("anti-rollback anchor did not match the prepared comparison")]
    NotAdvanced(AntiRollbackAnchorObservation),
    /// An ambiguous advance did not observe the exact successor.
    #[error("ambiguous anchor advance did not observe the exact successor")]
    AmbiguousNotExact {
        /// Original outcome-unknown anchor error.
        uncertain: AntiRollbackAnchorError,
        /// Non-successor state found by the mandatory observation.
        observation: AntiRollbackAnchorObservation,
    },
    /// Observation failed after an ambiguous anchor advance.
    #[error("anti-rollback observation failed after ambiguous advance")]
    AmbiguousObservationFailed {
        /// Original outcome-unknown anchor error.
        uncertain: AntiRollbackAnchorError,
        /// Observation failure that prevents classification.
        #[source]
        observe: AntiRollbackAnchorError,
    },
}

impl From<DatabaseCommitEvidence> for EffectCaseDatabaseCommitEvidence {
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

impl From<DatabaseCommitFailureReason> for EffectCaseCommitFailureReason {
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

impl From<AnchorAdvanceEvidence> for EffectCaseAnchorEvidence {
    fn from(value: AnchorAdvanceEvidence) -> Self {
        match value {
            AnchorAdvanceEvidence::Advanced => Self::Advanced,
            AnchorAdvanceEvidence::ExactReplay => Self::ExactReplay,
            AnchorAdvanceEvidence::RecoveredExactAfterAmbiguousAdvance => {
                Self::RecoveredExactAfterAmbiguousAdvance
            }
        }
    }
}

impl From<AnchorAdvanceFailureReason> for EffectCaseAnchorFailureReason {
    fn from(value: AnchorAdvanceFailureReason) -> Self {
        match value {
            AnchorAdvanceFailureReason::Port(error) => Self::Port(error),
            AnchorAdvanceFailureReason::NotAdvanced(observation) => Self::NotAdvanced(*observation),
            AnchorAdvanceFailureReason::AmbiguousNotExact {
                uncertain,
                observation,
            } => Self::AmbiguousNotExact {
                uncertain,
                observation: *observation,
            },
            AnchorAdvanceFailureReason::AmbiguousObservationFailed { uncertain, observe } => {
                Self::AmbiguousObservationFailed { uncertain, observe }
            }
        }
    }
}
