//! Append-before-notify coordination for resident authority.
//!
//! One admitted authority transition crosses the durable boundary before its
//! prepared process-local projection is applied and its wake token becomes
//! visible:
//!
//! The noncommutative prepare-then-append-then-apply-then-publish transition is
//! rendered on [`crate::resident::ResidentApplication`].
//! [`ResidentAuthorityCoordinator`] admits only the exact pending journal
//! coordinate for final publication.
//!
//! An append without proven success leaves the ring unchanged. Once the port
//! proves append success, a failed publication cannot undo the installed
//! authority or its applied projection; it produces
//! [`AuthorityCommitNotification::DurableReplayRequired`] so consumers recover
//! the exact sequence from durable custody. This module selects no storage
//! profile. It invokes the Pale-owned durable-custody port, but authorizes no
//! provider action or other native consequential effect.

use pale_core::records::JournalSequenceV1;
use pale_kernel::{WakeConsumerCursor, WakePublishError, WakeToken, WakeTokenRing};

use super::journal::{AuthorityJournalError, DurableAuthorityJournal};
use super::lifecycle::{GenerationLifecycle, GenerationLifecycleTransition};

/// Notification disposition after one authority transition is durably committed.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthorityCommitNotification {
    /// The exact durable journal coordinate was published to the bounded ring.
    Published(WakeToken),
    /// Authority committed, but consumers must replay this coordinate durably.
    DurableReplayRequired {
        /// Exact committed sequence whose token could not be published.
        journal_sequence: JournalSequenceV1,
        /// Process-local ring failure; never evidence that the append failed.
        reason: WakePublishError,
    },
}

impl AuthorityCommitNotification {
    /// Return the durable journal coordinate committed by either disposition.
    #[must_use]
    pub(crate) const fn journal_sequence(self) -> JournalSequenceV1 {
        match self {
            Self::Published(token) => token.journal_sequence(),
            Self::DurableReplayRequired {
                journal_sequence, ..
            } => journal_sequence,
        }
    }
}

/// Affine proof that one exact authority transition was appended but not published.
///
/// Only this coordinator can construct the token. Publication consumes it, while
/// an unconsumed token leaves the coordinator closed to every later append.
#[must_use = "an appended authority transition must be projected or recovered"]
pub(crate) struct AppendedAuthorityTransition {
    journal_sequence: JournalSequenceV1,
}

/// Reader-visibility phase of the current durable authority frontier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthorityProjectionState {
    /// No committed sequence awaits mechanical projection.
    Ready,
    /// The named append awaits application-owned mechanical apply.
    AwaitingMechanicalApply(JournalSequenceV1),
    /// Apply failed after append; only cold recovery may resume mutation.
    RecoveryRequired(JournalSequenceV1),
}

/// Application-owned join between one durable writer and its wake-token projection.
pub(crate) struct ResidentAuthorityCoordinator<'a> {
    journal: DurableAuthorityJournal<'a>,
    wake_ring: WakeTokenRing,
    projection_state: AuthorityProjectionState,
}

impl<'a> ResidentAuthorityCoordinator<'a> {
    /// Resume coordination only when the journal authority and wake-ring projection
    /// name the same durable frontier.
    ///
    /// The caller reconstructs both inputs and separately supplies the current
    /// process epoch when it builds the ring. Equality here proves only that
    /// the journal-authority and ring-projection frontiers agree; it proves no
    /// epoch provenance or freshness.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentAuthorityCoordinationError::FrontierMismatch`] when
    /// the journal and ring do not name the same last durable sequence.
    pub(crate) fn resume(
        journal: DurableAuthorityJournal<'a>,
        wake_ring: WakeTokenRing,
    ) -> Result<Self, ResidentAuthorityCoordinationError> {
        let journal_head = journal.head();
        let wake_head = wake_ring.current_cursor().journal_sequence();
        if journal_head != wake_head {
            return Err(ResidentAuthorityCoordinationError::FrontierMismatch {
                journal_head,
                wake_head,
            });
        }
        Ok(Self {
            journal,
            wake_ring,
            projection_state: AuthorityProjectionState::Ready,
        })
    }

    /// Borrow the exact durable authority-store incarnation.
    #[must_use]
    pub(crate) const fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        self.journal.store()
    }

    pub(crate) fn verify_event_custody_members_at(
        &self,
        events: &mut dyn pale_ports::EventCustodyPort,
        sequence: JournalSequenceV1,
        required: &[pale_ports::EventCustodyHistoryMember],
        maximum_event_bytes: usize,
    ) -> Result<(), ResidentAuthorityCoordinationError> {
        self.require_current_projection()?;
        self.journal.verify_event_custody_members_at(
            events,
            sequence,
            required,
            maximum_event_bytes,
        )?;
        Ok(())
    }

    /// Require one current durable frontier with no pending mechanical apply.
    ///
    /// # Errors
    ///
    /// Returns the owning journal uncertainty or projection-recovery error.
    pub(crate) fn require_current_projection(
        &self,
    ) -> Result<(), ResidentAuthorityCoordinationError> {
        if let Err(source) = self.journal.require_current() {
            return Err(ResidentAuthorityCoordinationError::Journal(source));
        }
        match self.projection_state {
            AuthorityProjectionState::Ready => Ok(()),
            AuthorityProjectionState::AwaitingMechanicalApply(journal_sequence) => {
                Err(ResidentAuthorityCoordinationError::PublicationPending { journal_sequence })
            }
            AuthorityProjectionState::RecoveryRequired(journal_sequence) => Err(
                ResidentAuthorityCoordinationError::PublicationRecoveryRequired {
                    journal_sequence,
                },
            ),
        }
    }

    /// Commit one lifecycle transition and then publish its wake coordinate.
    ///
    /// Durable append is attempted exactly once. Any append error returns
    /// before ring mutation. A later [`WakePublishError`] is represented as a
    /// committed replay obligation rather than a failed authority transition.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentAuthorityCoordinationError::Journal`] when the durable
    /// journal rejects or cannot prove the append. After append success, the
    /// method has no ordinary error branch: publication either succeeds or
    /// returns a committed durable-replay disposition.
    pub(crate) fn commit_transition(
        &mut self,
        transition: GenerationLifecycleTransition,
    ) -> Result<AuthorityCommitNotification, ResidentAuthorityCoordinationError> {
        let appended = self.append_transition(transition)?;
        self.publish_appended(appended)
    }

    /// Install joint Event custody before exposing its wake or continuation.
    pub(crate) fn commit_event_custody(
        &mut self,
        store: &mut dyn pale_ports::EventCustodyPort,
        transaction: pale_ports::ControlTransactionPreparation,
        input: super::event_custody::EventCustodyTransitionInput<'_>,
    ) -> Result<
        (
            super::event_custody::CompletedEventCustodyTransition,
            AuthorityCommitNotification,
        ),
        ResidentAuthorityCoordinationError,
    > {
        self.require_current_projection()?;
        let completed = self
            .journal
            .commit_event_custody(store, transaction, input)?;
        let journal_sequence = completed.candidate().journal_append().entry().sequence();
        self.projection_state = AuthorityProjectionState::AwaitingMechanicalApply(journal_sequence);
        let notification =
            self.publish_appended(AppendedAuthorityTransition { journal_sequence })?;
        Ok((completed, notification))
    }

    /// Append and install one lifecycle transition without publishing its wake token.
    ///
    /// This phase exists so an application-owned reconstructible projection can
    /// apply its already-prepared mechanical edit after durable authority but
    /// before reader visibility. The returned sequence is the only coordinate
    /// accepted by [`Self::publish_appended`].
    ///
    /// # Errors
    ///
    /// Returns [`ResidentAuthorityCoordinationError::Journal`] when the durable
    /// journal rejects or cannot prove the append. No wake token is published.
    pub(crate) fn append_transition(
        &mut self,
        transition: GenerationLifecycleTransition,
    ) -> Result<AppendedAuthorityTransition, ResidentAuthorityCoordinationError> {
        match self.projection_state {
            AuthorityProjectionState::Ready => {}
            AuthorityProjectionState::AwaitingMechanicalApply(journal_sequence) => {
                return Err(ResidentAuthorityCoordinationError::PublicationPending {
                    journal_sequence,
                });
            }
            AuthorityProjectionState::RecoveryRequired(journal_sequence) => {
                return Err(
                    ResidentAuthorityCoordinationError::PublicationRecoveryRequired {
                        journal_sequence,
                    },
                );
            }
        }
        let committed = self.journal.commit_transition(transition)?;
        let (journal_sequence, _) = committed.into_parts();
        self.projection_state = AuthorityProjectionState::AwaitingMechanicalApply(journal_sequence);
        Ok(AppendedAuthorityTransition { journal_sequence })
    }

    /// Commit one desired process state and then publish its wake coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentAuthorityCoordinationError::Journal`] when the durable
    /// append is not proven. Callers must avoid invoking this for an unchanged
    /// desired state because idempotent requests consume no journal coordinate.
    pub(crate) fn commit_desired_process_state(
        &mut self,
        desired: super::process_reconciliation::DesiredProcessState,
    ) -> Result<AuthorityCommitNotification, ResidentAuthorityCoordinationError> {
        match self.projection_state {
            AuthorityProjectionState::Ready => {}
            AuthorityProjectionState::AwaitingMechanicalApply(journal_sequence) => {
                return Err(ResidentAuthorityCoordinationError::PublicationPending {
                    journal_sequence,
                });
            }
            AuthorityProjectionState::RecoveryRequired(journal_sequence) => {
                return Err(
                    ResidentAuthorityCoordinationError::PublicationRecoveryRequired {
                        journal_sequence,
                    },
                );
            }
        }
        let committed = self.journal.commit_desired_process_state(desired)?;
        let (journal_sequence, _) = committed.into_parts();
        self.projection_state = AuthorityProjectionState::AwaitingMechanicalApply(journal_sequence);
        self.publish_appended(AppendedAuthorityTransition { journal_sequence })
    }

    /// Publish one exact sequence returned by [`Self::append_transition`].
    ///
    /// A caller may perform only infallible, already-prepared projection work
    /// between append and this publication. Failure in that phase must suppress
    /// this call and require cold reconstruction.
    pub(crate) fn publish_appended(
        &mut self,
        appended: AppendedAuthorityTransition,
    ) -> Result<AuthorityCommitNotification, ResidentAuthorityCoordinationError> {
        let journal_sequence = appended.journal_sequence;
        if self.projection_state
            != AuthorityProjectionState::AwaitingMechanicalApply(journal_sequence)
        {
            return Err(
                ResidentAuthorityCoordinationError::PublicationCoordinateMismatch {
                    pending: self.projection_state.sequence(),
                    token: journal_sequence,
                },
            );
        }
        self.projection_state = AuthorityProjectionState::Ready;
        Ok(
            match self
                .wake_ring
                .publish_after_durable_append(journal_sequence)
            {
                Ok(token) => AuthorityCommitNotification::Published(token),
                Err(reason) => AuthorityCommitNotification::DurableReplayRequired {
                    journal_sequence,
                    reason,
                },
            },
        )
    }

    /// Close this coordinator after mechanical apply fails for a proven append.
    ///
    /// The consumed token preserves the exact durable sequence that cold
    /// recovery must replay. No wake token is published from this path.
    pub(crate) fn require_recovery_after_apply(
        &mut self,
        appended: AppendedAuthorityTransition,
    ) -> JournalSequenceV1 {
        let journal_sequence = appended.journal_sequence;
        self.projection_state = AuthorityProjectionState::RecoveryRequired(journal_sequence);
        journal_sequence
    }

    /// Persist a reconstructible checkpoint at the current durable frontier.
    ///
    /// The checkpoint is a cache of already-committed authority. It advances no
    /// semantic state and publishes no wake token.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentAuthorityCoordinationError::Journal`] when the journal
    /// has no committed head or durable custody rejects the bounded checkpoint.
    pub(crate) fn store_checkpoint(&mut self) -> Result<(), ResidentAuthorityCoordinationError> {
        self.journal.store_checkpoint()?;
        Ok(())
    }

    /// Return the latest journal sequence proven durable by this coordinator.
    #[must_use]
    pub(crate) const fn journal_head(&self) -> Option<JournalSequenceV1> {
        self.journal.head()
    }

    /// Borrow the semantic authority installed at the current durable frontier.
    ///
    /// Successor transitions must be derived from this journal-owned state, not
    /// from a caller-retained shadow of an earlier lifecycle.
    #[must_use]
    pub(crate) const fn lifecycle(&self) -> &GenerationLifecycle {
        self.journal.lifecycle()
    }

    /// Return the desired process state installed at the durable frontier.
    #[must_use]
    pub(crate) const fn desired_process_state(
        &self,
    ) -> Option<super::process_reconciliation::DesiredProcessState> {
        self.journal.desired_process_state()
    }

    /// Borrow the process-local ring without granting publication authority.
    #[must_use]
    pub(crate) const fn wake_ring(&self) -> &WakeTokenRing {
        &self.wake_ring
    }

    /// Return the current process-local consumer frontier for diagnostics.
    #[must_use]
    pub(crate) const fn wake_cursor(&self) -> WakeConsumerCursor {
        self.wake_ring.current_cursor()
    }
}

/// Failure before the coordinator can return a committed notification disposition.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ResidentAuthorityCoordinationError {
    /// Reconstructed journal and process-local ring frontiers disagree.
    #[error("resident journal and wake-ring durable frontiers differ")]
    FrontierMismatch {
        /// Durable frontier owned by the journal coordinator.
        journal_head: Option<JournalSequenceV1>,
        /// Durable frontier carried by the wake ring.
        wake_head: Option<JournalSequenceV1>,
    },
    /// A prior durable append has not completed its process-local projection.
    #[error("resident authority sequence {journal_sequence:?} awaits projection recovery")]
    PublicationPending {
        /// Exact durable append whose reader-visible projection did not complete.
        journal_sequence: JournalSequenceV1,
    },
    /// A committed sequence has a broken reconstructible projection.
    #[error("resident authority sequence {journal_sequence:?} requires projection recovery")]
    PublicationRecoveryRequired {
        /// Exact committed sequence that cold recovery must reconstruct.
        journal_sequence: JournalSequenceV1,
    },
    /// An affine publication token did not match this coordinator's pending append.
    #[error("resident authority publication token does not match its pending append")]
    PublicationCoordinateMismatch {
        /// Append currently awaiting publication, when present.
        pending: Option<JournalSequenceV1>,
        /// Sequence carried by the consumed publication token.
        token: JournalSequenceV1,
    },
    /// The authority journal rejected or could not prove the append.
    #[error(transparent)]
    Journal(#[from] AuthorityJournalError),
}

impl AuthorityProjectionState {
    /// Return the sequence preventing ordinary projection, when any.
    const fn sequence(self) -> Option<JournalSequenceV1> {
        match self {
            Self::Ready => None,
            Self::AwaitingMechanicalApply(sequence) | Self::RecoveryRequired(sequence) => {
                Some(sequence)
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/coordination.rs"]
mod tests;
