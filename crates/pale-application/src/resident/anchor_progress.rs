//! Resume one retained anchor without repeating an uncertain advance.

use super::custody_phase::{
    advance_anchor_with_exact_observation, AnchorAdvanceEvidence, AnchorAdvanceFailureReason,
};
use pale_ports::{
    AntiRollbackAnchorError, AntiRollbackAnchorObservation, AntiRollbackAnchorPort,
    PreparedAntiRollbackAnchorTransition,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ObservationReason {
    DatabaseReplay,
    AdvanceUncertain,
    Refused,
}

/// Per-anchor progress; completed legs are historical evidence, not current authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AnchorProgress {
    Ready,
    ObserveFirst(ObservationReason),
    Established(AnchorAdvanceEvidence),
}

#[derive(Debug, thiserror::Error)]
pub(super) enum AnchorCompletionFailure {
    #[error(transparent)]
    Observation(#[from] AntiRollbackAnchorError),
    #[error(transparent)]
    Advance(#[from] AnchorAdvanceFailureReason),
}

impl AnchorProgress {
    /// Observe before resuming a failed or replayed leg. Missing state permits an
    /// initial CAS only; it never permits reconstruction of a successor's anchor.
    pub(super) fn complete(
        &mut self,
        port: &mut dyn AntiRollbackAnchorPort,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AnchorAdvanceEvidence, AnchorCompletionFailure> {
        match *self {
            Self::Established(evidence) => return Ok(evidence),
            Self::Ready => {}
            Self::ObserveFirst(reason) => {
                let observed = port
                    .observe(transition)
                    .map_err(AnchorCompletionFailure::Observation)?;
                match observed {
                    AntiRollbackAnchorObservation::ExactSuccessor => {
                        let evidence = match reason {
                            ObservationReason::AdvanceUncertain => {
                                AnchorAdvanceEvidence::RecoveredExactAfterAmbiguousAdvance
                            }
                            ObservationReason::DatabaseReplay | ObservationReason::Refused => {
                                AnchorAdvanceEvidence::ExactReplay
                            }
                        };
                        *self = Self::Established(evidence);
                        return Ok(evidence);
                    }
                    AntiRollbackAnchorObservation::ExactPredecessor => {}
                    AntiRollbackAnchorObservation::Missing if transition.expected().is_none() => {}
                    other => {
                        return Err(AnchorCompletionFailure::Advance(
                            AnchorAdvanceFailureReason::NotAdvanced(Box::new(other)),
                        ));
                    }
                }
            }
        }
        match advance_anchor_with_exact_observation(port, transition) {
            Ok(evidence) => {
                *self = Self::Established(evidence);
                Ok(evidence)
            }
            Err(reason) => {
                *self = Self::ObserveFirst(match &reason {
                    AnchorAdvanceFailureReason::AmbiguousNotExact { .. }
                    | AnchorAdvanceFailureReason::AmbiguousObservationFailed { .. } => {
                        ObservationReason::AdvanceUncertain
                    }
                    AnchorAdvanceFailureReason::Port(_)
                    | AnchorAdvanceFailureReason::NotAdvanced(_) => ObservationReason::Refused,
                });
                Err(AnchorCompletionFailure::Advance(reason))
            }
        }
    }
}
