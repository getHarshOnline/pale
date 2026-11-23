//! Complete bootstrap's retained initial anchors after database commitment.

use super::{
    DatabaseCommittedPrincipalBootstrapAnchorsPending, PrincipalBootstrapCommitEvidence,
    PrincipalBootstrapCustodyPhase,
};
use crate::resident::{
    anchor_progress::AnchorCompletionFailure,
    custody_phase::{AnchorAdvanceEvidence, AnchorAdvanceFailureReason},
};
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorError, AntiRollbackAnchorKey, AntiRollbackAnchorPort,
    PreparedPrincipalBootstrapAtom,
};

impl DatabaseCommittedPrincipalBootstrapAnchorsPending {
    /// Establish the trust minimum, then each admitted initial domain anchor.
    ///
    /// The coordinates come from the signed bootstrap records, not from the
    /// caller or the physical database identity. Each leg uses the shared
    /// exact-observation protocol. A failure retains completed legs and the
    /// uncertain leg; resumption observes that leg before any new CAS.
    /// Database replay also observes first. Missing state permits only the
    /// retained initial CAS. A later or conflicting anchor requires recovery.
    ///
    /// The result proves historical custody under the selected port's contract.
    /// It does not qualify the backend's restore isolation, establish current
    /// authority, activate the resident, or permit dispatch.
    ///
    /// # Errors
    /// Returns the committed candidate and partial progress when a leg cannot
    /// be established. An ambiguous advance is never retried in the same call.
    ///
    /// ```compile_fail
    /// use pale_application::resident::PreparedPrincipalBootstrapCustody;
    /// use pale_ports::AntiRollbackAnchorPort;
    /// fn before_commit(prepared: PreparedPrincipalBootstrapCustody, port: &mut dyn AntiRollbackAnchorPort) {
    ///     prepared.advance_anchors(port);
    /// }
    /// ```
    pub fn advance_anchors(
        mut self,
        port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedPrincipalBootstrapCustody, PrincipalBootstrapAnchorFailure> {
        let mut evidence = Vec::with_capacity(self.anchors.len());
        for (transition, progress) in &mut self.anchors {
            match progress.complete(port, transition) {
                Ok(completed) => evidence.push((transition.next().clone(), completed)),
                Err(failure) => {
                    let reason = match failure {
                        AnchorCompletionFailure::Observation(error) => {
                            PrincipalBootstrapAnchorFailureReason::Observation {
                                anchor: transition.key().clone(),
                                error,
                            }
                        }
                        AnchorCompletionFailure::Advance(reason) => {
                            PrincipalBootstrapAnchorFailureReason::Advance {
                                anchor: transition.key().clone(),
                                reason,
                            }
                        }
                    };
                    return Err(PrincipalBootstrapAnchorFailure {
                        state: Box::new(AnchorFailureState {
                            pending: self,
                            reason,
                        }),
                    });
                }
            }
        }
        Ok(CompletedPrincipalBootstrapCustody {
            candidate: self.candidate,
            database_evidence: self.evidence,
            anchors: evidence,
        })
    }
}

/// Historical database and anchor custody for the exact bootstrap candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedPrincipalBootstrapCustody {
    candidate: PreparedPrincipalBootstrapAtom,
    database_evidence: PrincipalBootstrapCommitEvidence,
    anchors: Vec<(AntiRollbackAnchor, AnchorAdvanceEvidence)>,
}

impl CompletedPrincipalBootstrapCustody {
    /// Return the completed historical custody phase, not resident readiness.
    #[must_use]
    pub const fn phase(&self) -> PrincipalBootstrapCustodyPhase {
        PrincipalBootstrapCustodyPhase::Complete
    }
    /// Borrow the candidate whose database and anchors were established.
    #[must_use]
    pub const fn candidate(&self) -> &PreparedPrincipalBootstrapAtom {
        &self.candidate
    }
    /// Return the evidence for database commitment.
    #[must_use]
    pub const fn database_evidence(&self) -> PrincipalBootstrapCommitEvidence {
        self.database_evidence
    }
    /// Borrow every exact anchor and its evidence, trust minimum first.
    #[must_use]
    pub fn anchors(&self) -> &[(AntiRollbackAnchor, AnchorAdvanceEvidence)] {
        &self.anchors
    }
}

/// An unfinished anchor leg with the committed candidate and progress retained.
#[derive(Debug, thiserror::Error)]
#[error("principal bootstrap anchors remain incomplete: {reason}", reason = .state.reason)]
pub struct PrincipalBootstrapAnchorFailure {
    state: Box<AnchorFailureState>,
}

#[derive(Debug)]
struct AnchorFailureState {
    pending: DatabaseCommittedPrincipalBootstrapAnchorsPending,
    reason: PrincipalBootstrapAnchorFailureReason,
}

impl PrincipalBootstrapAnchorFailure {
    /// Borrow the failed observation or advance.
    #[must_use]
    pub const fn reason(&self) -> &PrincipalBootstrapAnchorFailureReason {
        &self.state.reason
    }
    /// Borrow the retained state without discarding completed legs.
    pub fn pending(&self) -> &DatabaseCommittedPrincipalBootstrapAnchorsPending {
        &self.state.pending
    }
    /// Recover that same state for an explicit continuation.
    pub fn into_pending(self) -> DatabaseCommittedPrincipalBootstrapAnchorsPending {
        self.state.pending
    }
}

/// Why one exact bootstrap anchor remains unproved.
#[derive(Debug, thiserror::Error)]
pub enum PrincipalBootstrapAnchorFailureReason {
    /// The required read before resumption failed; no CAS was attempted.
    #[error("bootstrap anchor {anchor:?} could not be observed: {error}")]
    Observation {
        /// Retained anchor key, not caller-supplied replacement coordinates.
        anchor: AntiRollbackAnchorKey,
        /// Failure returned by the external mechanism.
        #[source]
        error: AntiRollbackAnchorError,
    },
    /// The anchor did not establish the exact retained successor.
    #[error("bootstrap anchor {anchor:?} did not complete: {reason}")]
    Advance {
        /// Retained anchor key.
        anchor: AntiRollbackAnchorKey,
        /// The comparison and any immediate ambiguity observation.
        #[source]
        reason: AnchorAdvanceFailureReason,
    },
}
