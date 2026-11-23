//! Fair, caller-driven scheduling for one resident work turn.
//!
//! The coordinator does not run a thread or discover an endpoint. A caller
//! offers a ready frame and/or a recovery opportunity. The resident service
//! owns interrupted phases. One call advances at most one source.

use std::time::Instant;

use pale_ports::LocalIpcFramePort;

use super::{
    frame_service::{ResidentFrameOutcome, ResidentFrameService, ResidentFrameServiceError},
    program::ResidentProgramRetainedPhaseTurn,
};

/// A connected frame that the caller has established is ready for one turn.
pub struct ResidentReadyFrame<'carrier> {
    carrier: &'carrier mut dyn LocalIpcFramePort,
    carrier_deadline: Instant,
    operation_response_deadline: Instant,
}

impl<'carrier> ResidentReadyFrame<'carrier> {
    /// Bind one ready carrier to its existing absolute deadlines.
    #[must_use]
    pub fn new(
        carrier: &'carrier mut dyn LocalIpcFramePort,
        carrier_deadline: Instant,
        operation_response_deadline: Instant,
    ) -> Self {
        Self {
            carrier,
            carrier_deadline,
            operation_response_deadline,
        }
    }
}

/// Work made ready by the caller for one bounded scheduling decision.
pub struct ResidentWorkTurnInput<'carrier> {
    frame: Option<ResidentReadyFrame<'carrier>>,
    recovery: ResidentRecoveryOpportunity,
}

enum ResidentRecoveryOpportunity {
    NotReady,
    Ready,
}

impl<'carrier> ResidentWorkTurnInput<'carrier> {
    /// Construct an input with no ready external work.
    #[must_use]
    pub const fn idle() -> Self {
        Self {
            frame: None,
            recovery: ResidentRecoveryOpportunity::NotReady,
        }
    }

    /// Offer one already-ready frame.
    #[must_use]
    pub fn with_frame(mut self, frame: ResidentReadyFrame<'carrier>) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Report that one bounded task-completion or recovery attempt is ready.
    #[must_use]
    pub const fn with_recovery(mut self) -> Self {
        self.recovery = ResidentRecoveryOpportunity::Ready;
        self
    }
}

impl<'carrier> Default for ResidentWorkTurnInput<'carrier> {
    fn default() -> Self {
        Self::idle()
    }
}

/// Source selected for one resident work turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentWorkSource {
    /// A ready local request frame.
    Frame,
    /// One exact runnable phase retained by the operation owner.
    RetainedPhase,
    /// One task-completion or durable-recovery opportunity.
    Recovery,
}

impl ResidentWorkSource {
    const fn next(self) -> Self {
        match self {
            Self::Frame => Self::RetainedPhase,
            Self::RetainedPhase => Self::Recovery,
            Self::Recovery => Self::Frame,
        }
    }
}

/// Result of one bounded scheduling decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentWorkTurnOutcome {
    /// One ready frame reached its existing application owner.
    Frame(ResidentFrameOutcome),
    /// One exact retained operation phase advanced. A final result or
    /// continuation can remain unavailable.
    RetainedPhaseAdvanced,
    /// The selected retained phase could not advance. The exact phase remains
    /// owned by the resident service.
    RetainedPhaseDeferred,
    /// One task-completion or recovery source reached a safe handoff.
    RecoveryAdvanced,
    /// The selected recovery owner was unavailable. Its exact cursor or
    /// handoff remains retained by that owner.
    RecoveryDeferred,
    /// No frame, retained phase, or recovery opportunity was ready.
    Idle,
}

/// Failure of a selected work turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ResidentWorkTurnError {
    /// A ready frame failed at its existing bounded request boundary.
    #[error(transparent)]
    Frame(#[from] ResidentFrameServiceError),
}

/// Round-robin selector for caller-driven resident work.
///
/// A selected source always moves to the back of the rotation, including a
/// deferred retained phase or recovery attempt. A source that is not ready is
/// skipped. Consequently, an interrupted Start cannot permanently prevent a
/// ready Advance or Cancel frame from reaching the existing request owner.
pub struct ResidentWorkTurnCoordinator {
    next_source: ResidentWorkSource,
}

impl ResidentWorkTurnCoordinator {
    /// Start with frame ingress, then retained phases, then recovery.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next_source: ResidentWorkSource::Frame,
        }
    }

    /// Read the first source considered on the next turn.
    #[must_use]
    pub const fn next_source(&self) -> ResidentWorkSource {
        self.next_source
    }

    /// Advance at most one ready source.
    ///
    /// The caller decides when a carrier or recovery opportunity is ready.
    /// The resident service alone decides whether it owns an interrupted
    /// phase. This method performs no wait, retry, discovery, or background
    /// work.
    ///
    /// # Errors
    ///
    /// Returns the existing frame-service error when the selected frame cannot
    /// reach a complete response boundary. Retained-phase and recovery
    /// unavailability are explicit nonterminal outcomes because their owners
    /// keep the unfinished state.
    pub fn run_one(
        &mut self,
        service: &mut ResidentFrameService<'_>,
        mut input: ResidentWorkTurnInput<'_>,
    ) -> Result<ResidentWorkTurnOutcome, ResidentWorkTurnError> {
        let mut source = self.next_source;
        for _ in 0..3 {
            match source {
                ResidentWorkSource::Frame => {
                    if let Some(frame) = input.frame.take() {
                        self.next_source = source.next();
                        return service
                            .serve_frame(
                                frame.carrier,
                                frame.carrier_deadline,
                                frame.operation_response_deadline,
                            )
                            .map(ResidentWorkTurnOutcome::Frame)
                            .map_err(Into::into);
                    }
                }
                ResidentWorkSource::RetainedPhase => {
                    if service.has_retained_phase() {
                        match service.continue_one_retained_phase() {
                            ResidentProgramRetainedPhaseTurn::Advanced => {
                                self.next_source = source.next();
                                return Ok(ResidentWorkTurnOutcome::RetainedPhaseAdvanced);
                            }
                            ResidentProgramRetainedPhaseTurn::Deferred => {
                                self.next_source = source.next();
                                return Ok(ResidentWorkTurnOutcome::RetainedPhaseDeferred);
                            }
                            ResidentProgramRetainedPhaseTurn::Idle => {}
                        }
                    }
                }
                ResidentWorkSource::Recovery => {
                    if matches!(input.recovery, ResidentRecoveryOpportunity::Ready) {
                        self.next_source = source.next();
                        return Ok(match service.recover_one_bounded_source() {
                            Ok(()) => ResidentWorkTurnOutcome::RecoveryAdvanced,
                            Err(ResidentFrameServiceError::RecoveryUnavailable) => {
                                ResidentWorkTurnOutcome::RecoveryDeferred
                            }
                            Err(ResidentFrameServiceError::TurnUnavailable) => {
                                ResidentWorkTurnOutcome::RecoveryDeferred
                            }
                        });
                    }
                }
            }
            source = source.next();
        }
        Ok(ResidentWorkTurnOutcome::Idle)
    }
}

impl Default for ResidentWorkTurnCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/work_turn.rs"]
mod tests;
