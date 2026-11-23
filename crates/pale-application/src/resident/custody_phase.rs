//! Domain-neutral closure of atomic database-commit ambiguity.

use pale_ports::{
    AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError, AntiRollbackAnchorErrorKind,
    AntiRollbackAnchorObservation, AntiRollbackAnchorPort, ControlStoreError,
    ControlStoreErrorKind, PreparedAntiRollbackAnchorTransition,
};

/// Store disposition after one atomic compare-and-commit call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DatabaseCommitDisposition {
    /// The complete candidate advanced from its exact predecessor.
    Committed,
    /// The complete candidate already existed byte-for-byte.
    ExactReplay,
}

/// Exact read-back classification shared by atomic custody ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DatabaseReadBackDisposition {
    /// The exact predecessor remains and no candidate material exists.
    Predecessor,
    /// The complete exact successor exists.
    ExactSuccessor,
    /// Storage is partial, contradictory, or contains another transition.
    ConflictingOrPartial,
}

/// Evidence establishing exact database custody without semantic side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DatabaseCommitEvidence {
    /// The database advanced during this call.
    New,
    /// The database already contained the exact candidate.
    ExactReplay,
    /// One read-back after an ambiguous commit found the exact successor.
    RecoveredExactAfterAmbiguousCommit,
}

/// Exact database-custody failure with ambiguity preserved.
#[derive(Debug, thiserror::Error)]
pub(super) enum DatabaseCommitFailureReason {
    /// Storage rejected the candidate without an ambiguous commit boundary.
    #[error(transparent)]
    Store(ControlStoreError),
    /// Read-back proved that an ambiguous commit did not advance.
    #[error("ambiguous commit was excluded by exact read-back")]
    CommitExcludedByReadBack {
        /// Original outcome-unknown error.
        uncertain: ControlStoreError,
    },
    /// Read-back found partial, contradictory, or competing custody.
    #[error("ambiguous commit requires explicit recovery")]
    RecoveryRequired {
        /// Original outcome-unknown error.
        uncertain: ControlStoreError,
    },
    /// Mandatory read-back failed after an ambiguous commit.
    #[error("exact read-back failed after ambiguous commit")]
    ReadBackFailed {
        /// Original outcome-unknown error.
        uncertain: ControlStoreError,
        /// Error preventing exact read-back classification.
        #[source]
        read_back: ControlStoreError,
    },
}

/// Evidence establishing one exact external-anchor successor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorAdvanceEvidence {
    /// The external anchor advanced during this call.
    Advanced,
    /// The external anchor already contained the exact successor.
    ExactReplay,
    /// An exact observation found the successor after an ambiguous advance.
    RecoveredExactAfterAmbiguousAdvance,
}

/// External-anchor failure with uncertainty and observation evidence retained.
#[derive(Debug, thiserror::Error)]
pub enum AnchorAdvanceFailureReason {
    /// The anchor rejected the comparison or failed before ambiguity.
    #[error(transparent)]
    Port(AntiRollbackAnchorError),
    /// The anchor explicitly retained another observable state.
    #[error("anti-rollback anchor did not match the prepared comparison")]
    NotAdvanced(Box<AntiRollbackAnchorObservation>),
    /// An ambiguous advance did not observe the exact successor.
    #[error("ambiguous anchor advance did not observe the exact successor")]
    AmbiguousNotExact {
        /// Original outcome-unknown anchor error.
        uncertain: AntiRollbackAnchorError,
        /// Non-successor state found by the mandatory observation.
        observation: Box<AntiRollbackAnchorObservation>,
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

/// Perform exactly one commit and the sole allowed ambiguity read-back.
///
/// `observe` is called only after [`ControlStoreErrorKind::OutcomeUnknown`].
/// The helper never retries a commit and grants no domain-specific continuation.
pub(super) fn commit_with_exact_readback<S, C, Commit, Observe>(
    store: &mut S,
    candidate: &C,
    commit: Commit,
    observe: Observe,
) -> Result<DatabaseCommitEvidence, DatabaseCommitFailureReason>
where
    S: ?Sized,
    Commit: FnOnce(&mut S, &C) -> Result<DatabaseCommitDisposition, ControlStoreError>,
    Observe: FnOnce(&mut S, &C) -> Result<DatabaseReadBackDisposition, ControlStoreError>,
{
    match commit(store, candidate) {
        Ok(DatabaseCommitDisposition::Committed) => Ok(DatabaseCommitEvidence::New),
        Ok(DatabaseCommitDisposition::ExactReplay) => Ok(DatabaseCommitEvidence::ExactReplay),
        Err(error) if error.kind() == ControlStoreErrorKind::OutcomeUnknown => {
            match observe(store, candidate) {
                Ok(DatabaseReadBackDisposition::ExactSuccessor) => {
                    Ok(DatabaseCommitEvidence::RecoveredExactAfterAmbiguousCommit)
                }
                Ok(DatabaseReadBackDisposition::Predecessor) => {
                    Err(DatabaseCommitFailureReason::CommitExcludedByReadBack { uncertain: error })
                }
                Ok(DatabaseReadBackDisposition::ConflictingOrPartial) => {
                    Err(DatabaseCommitFailureReason::RecoveryRequired { uncertain: error })
                }
                Err(read_back) => Err(DatabaseCommitFailureReason::ReadBackFailed {
                    uncertain: error,
                    read_back,
                }),
            }
        }
        Err(error) => Err(DatabaseCommitFailureReason::Store(error)),
    }
}

/// Perform one anchor comparison and the sole allowed ambiguity observation.
///
/// The helper never retries an advance. Only an exact successor recovers an
/// ambiguous outcome; domain-specific policy decides whether an explicitly
/// observed later anchor can establish historical custody.
pub(super) fn advance_anchor_with_exact_observation(
    port: &mut dyn AntiRollbackAnchorPort,
    transition: &PreparedAntiRollbackAnchorTransition,
) -> Result<AnchorAdvanceEvidence, AnchorAdvanceFailureReason> {
    match port.compare_and_advance(transition) {
        Ok(AntiRollbackAnchorAdvanceDisposition::Advanced) => Ok(AnchorAdvanceEvidence::Advanced),
        Ok(AntiRollbackAnchorAdvanceDisposition::ExactReplay) => {
            Ok(AnchorAdvanceEvidence::ExactReplay)
        }
        Ok(AntiRollbackAnchorAdvanceDisposition::NotAdvanced(observation)) => Err(
            AnchorAdvanceFailureReason::NotAdvanced(Box::new(observation)),
        ),
        Err(error) if error.kind() == AntiRollbackAnchorErrorKind::OutcomeUnknown => {
            match port.observe(transition) {
                Ok(AntiRollbackAnchorObservation::ExactSuccessor) => {
                    Ok(AnchorAdvanceEvidence::RecoveredExactAfterAmbiguousAdvance)
                }
                Ok(observation) => Err(AnchorAdvanceFailureReason::AmbiguousNotExact {
                    uncertain: error,
                    observation: Box::new(observation),
                }),
                Err(observe) => Err(AnchorAdvanceFailureReason::AmbiguousObservationFailed {
                    uncertain: error,
                    observe,
                }),
            }
        }
        Err(error) => Err(AnchorAdvanceFailureReason::Port(error)),
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/custody_phase.rs"]
mod tests;
