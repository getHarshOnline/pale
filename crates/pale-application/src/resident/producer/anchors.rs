//! Complete the two external anchors of an already committed producer transition.

use pale_ports::{
    AntiRollbackAnchorError, AntiRollbackAnchorKey, AntiRollbackAnchorPort,
    AntiRollbackAnchorTransitionError, ControlDomainAntiRollbackAnchor,
    PreparedAntiRollbackAnchorTransition, PreparedPrincipalProducerIncarnationTransition,
    PrincipalTrustMinimumAnchor,
};

use super::{
    DatabaseCommittedPrincipalProducerIncarnationAnchorsPending,
    PrincipalProducerIncarnationCommitEvidence, PrincipalProducerIncarnationCustodyPhase,
};
use crate::resident::custody_phase::{AnchorAdvanceEvidence, AnchorAdvanceFailureReason};

pub(super) use crate::resident::anchor_progress::AnchorProgress;
use crate::resident::anchor_progress::{AnchorCompletionFailure, ObservationReason};

impl AnchorProgress {
    pub(super) const fn after_database(
        evidence: PrincipalProducerIncarnationCommitEvidence,
    ) -> Self {
        match evidence {
            PrincipalProducerIncarnationCommitEvidence::New => Self::Ready,
            PrincipalProducerIncarnationCommitEvidence::ExactReplay
            | PrincipalProducerIncarnationCommitEvidence::RecoveredExactAfterAmbiguousCommit => {
                Self::ObserveFirst(ObservationReason::DatabaseReplay)
            }
        }
    }
}

impl DatabaseCommittedPrincipalProducerIncarnationAnchorsPending {
    /// Return the first anchor's evidence if only the hard-domain advance remains.
    #[must_use]
    pub const fn trust_anchor_evidence(&self) -> Option<AnchorAdvanceEvidence> {
        match self.trust_anchor {
            AnchorProgress::Established(evidence) => Some(evidence),
            AnchorProgress::Ready | AnchorProgress::ObserveFirst(_) => None,
        }
    }

    /// Advance the trust minimum, then the retained hard-domain checkpoint.
    ///
    /// Both comparisons use the retained candidate and its admitted control
    /// store. The caller supplies only the selected anchor port. Each advance occurs once per
    /// attempt. An unknown outcome permits one exact observation, not a retry.
    /// If the second advance fails, the returned state retains the first one's
    /// evidence. Resuming that state attempts only the remaining anchor.
    /// A missing, conflicting or later anchor does not establish this successor.
    /// After failure or database replay, observe the retained transition first:
    /// an exact successor completes the leg; an exact predecessor permits one
    /// new comparison. Every other observation keeps the leg pending.
    ///
    /// Completion records historical custody, not current authority, epoch
    /// non-reuse, permission to publish an Event or permission to dispatch work.
    /// Later consumers must independently check current authority, heads and fences.
    ///
    /// # Errors
    /// Returns the complete pending state and the failed comparison when either
    /// transition cannot be constructed or an exact anchor cannot be established.
    ///
    /// A merely prepared transition has no anchor-advance method:
    ///
    /// ```compile_fail
    /// use pale_application::resident::PreparedPrincipalProducerIncarnationCustody;
    /// use pale_ports::AntiRollbackAnchorPort;
    /// fn premature(prepared: PreparedPrincipalProducerIncarnationCustody, port: &mut dyn AntiRollbackAnchorPort) {
    ///     prepared.advance_anchors(port);
    /// }
    /// ```
    pub fn advance_anchors(
        mut self,
        port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<
        CompletedPrincipalProducerIncarnationCustody,
        PrincipalProducerIncarnationAnchorFailure,
    > {
        let (trust, control) = match transitions(&self.candidate, &self.control_store) {
            Ok(pair) => pair,
            Err(error) => {
                return Err(self.failed(
                    PrincipalProducerIncarnationAnchorFailureReason::Transition(error),
                ));
            }
        };
        let trust_evidence = match complete_anchor(&mut self.trust_anchor, port, &trust) {
            Ok(evidence) => evidence,
            Err(reason) => return Err(self.failed(reason)),
        };
        match complete_anchor(&mut self.control_anchor, port, &control) {
            Ok(control_evidence) => Ok(CompletedPrincipalProducerIncarnationCustody {
                candidate: self.candidate,
                control_store: self.control_store,
                database_evidence: self.evidence,
                trust_evidence,
                control_evidence,
            }),
            Err(reason) => Err(self.failed(reason)),
        }
    }

    fn failed(
        self,
        reason: PrincipalProducerIncarnationAnchorFailureReason,
    ) -> PrincipalProducerIncarnationAnchorFailure {
        PrincipalProducerIncarnationAnchorFailure {
            state: Box::new(AnchorFailureState {
                pending: self,
                reason,
            }),
        }
    }
}

/// Historical proof of one database transition and its two exact anchor advances.
///
/// This is not a current-head witness, owner-outbox admission or dispatch permission.
/// No caller-supplied coordinates can construct this state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedPrincipalProducerIncarnationCustody {
    candidate: PreparedPrincipalProducerIncarnationTransition,
    control_store: pale_ports::AuthorityStoreCoordinate,
    database_evidence: PrincipalProducerIncarnationCommitEvidence,
    trust_evidence: AnchorAdvanceEvidence,
    control_evidence: AnchorAdvanceEvidence,
}

impl CompletedPrincipalProducerIncarnationCustody {
    /// Return the historical completion phase represented by this type.
    #[must_use]
    pub const fn phase(&self) -> PrincipalProducerIncarnationCustodyPhase {
        PrincipalProducerIncarnationCustodyPhase::Completed
    }

    /// Borrow the exact candidate whose custody was completed.
    #[must_use]
    pub const fn candidate(&self) -> &PreparedPrincipalProducerIncarnationTransition {
        &self.candidate
    }

    /// Borrow the admitted control-store incarnation whose anchor was established.
    /// This is distinct from the candidate's physical trust-store binding.
    #[must_use]
    pub const fn control_store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        &self.control_store
    }

    /// Return how the database leg was established.
    #[must_use]
    pub const fn database_evidence(&self) -> PrincipalProducerIncarnationCommitEvidence {
        self.database_evidence
    }

    /// Return how the principal trust minimum was established.
    #[must_use]
    pub const fn trust_anchor_evidence(&self) -> AnchorAdvanceEvidence {
        self.trust_evidence
    }

    /// Return how the hard-domain successor was established.
    #[must_use]
    pub const fn control_anchor_evidence(&self) -> AnchorAdvanceEvidence {
        self.control_evidence
    }
}

/// Anchor failure with the committed candidate and any first-anchor proof retained.
#[derive(Debug, thiserror::Error)]
#[error("producer custody anchors remain incomplete: {reason}", reason = .state.reason)]
pub struct PrincipalProducerIncarnationAnchorFailure {
    state: Box<AnchorFailureState>,
}

/// Keep both retained values in one allocation, not in the returned error itself.
#[derive(Debug)]
struct AnchorFailureState {
    pending: DatabaseCommittedPrincipalProducerIncarnationAnchorsPending,
    reason: PrincipalProducerIncarnationAnchorFailureReason,
}

impl PrincipalProducerIncarnationAnchorFailure {
    /// Borrow the exact failed comparison or transition error.
    #[must_use]
    pub const fn reason(&self) -> &PrincipalProducerIncarnationAnchorFailureReason {
        &self.state.reason
    }

    /// Borrow the pending state without losing established first-anchor progress.
    pub fn pending(&self) -> &DatabaseCommittedPrincipalProducerIncarnationAnchorsPending {
        &self.state.pending
    }

    /// Recover the same committed state for an explicit continuation attempt.
    pub fn into_pending(self) -> DatabaseCommittedPrincipalProducerIncarnationAnchorsPending {
        self.state.pending
    }
}

/// Failure to establish the retained producer candidate's external anchors.
#[derive(Debug, thiserror::Error)]
pub enum PrincipalProducerIncarnationAnchorFailureReason {
    /// Retained coordinates did not form an exact successor before any port call.
    #[error(transparent)]
    Transition(AntiRollbackAnchorTransitionError),
    /// A mandatory continuation observation could not be read.
    #[error("producer anchor {anchor:?} could not be observed: {error}")]
    Observation {
        /// Exact retained key whose prior outcome remains unresolved.
        anchor: AntiRollbackAnchorKey,
        /// Native observation failure; no new comparison was attempted.
        #[source]
        error: AntiRollbackAnchorError,
    },
    /// One named anchor could not be established exactly.
    #[error("producer anchor {anchor:?} did not complete: {reason}")]
    Advance {
        /// Exact key derived from retained admitted coordinates, not replaced by a caller.
        anchor: AntiRollbackAnchorKey,
        /// Original port and ambiguity-observation evidence.
        #[source]
        reason: AnchorAdvanceFailureReason,
    },
}

fn complete_anchor(
    progress: &mut AnchorProgress,
    port: &mut dyn AntiRollbackAnchorPort,
    transition: &PreparedAntiRollbackAnchorTransition,
) -> Result<AnchorAdvanceEvidence, PrincipalProducerIncarnationAnchorFailureReason> {
    progress
        .complete(port, transition)
        .map_err(|failure| match failure {
            AnchorCompletionFailure::Observation(error) => {
                PrincipalProducerIncarnationAnchorFailureReason::Observation {
                    anchor: transition.key().clone(),
                    error,
                }
            }
            AnchorCompletionFailure::Advance(reason) => {
                PrincipalProducerIncarnationAnchorFailureReason::Advance {
                    anchor: transition.key().clone(),
                    reason,
                }
            }
        })
}

fn transitions(
    candidate: &PreparedPrincipalProducerIncarnationTransition,
    control_store: &pale_ports::AuthorityStoreCoordinate,
) -> Result<
    (
        PreparedAntiRollbackAnchorTransition,
        PreparedAntiRollbackAnchorTransition,
    ),
    AntiRollbackAnchorTransitionError,
> {
    let trust = |head: &pale_ports::PrincipalTrustCheckpointHead| {
        PrincipalTrustMinimumAnchor::new(
            head.principal_id().clone(),
            head.checkpoint_epoch(),
            head.coordinate().clone(),
        )
        .into()
    };
    let control = |head: &pale_ports::ControlCheckpointHead| {
        ControlDomainAntiRollbackAnchor::new(
            control_store.store_id().clone(),
            control_store.incarnation_id().clone(),
            head.checkpoint_domain_id().clone(),
            head.domain_epoch(),
            head.domain_sequence(),
            head.coordinate().clone(),
        )
        .into()
    };
    Ok((
        PreparedAntiRollbackAnchorTransition::successor(
            trust(candidate.expected_trust()),
            trust(candidate.next_trust()),
        )?,
        PreparedAntiRollbackAnchorTransition::successor(
            control(candidate.expected_checkpoint()),
            control(candidate.next_checkpoint()),
        )?,
    ))
}
