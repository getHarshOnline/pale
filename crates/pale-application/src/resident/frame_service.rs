//! Public frame and recovery façade over the assembled resident program.
//!
//! This type adds no request parsing, admission, recovery, or operation
//! semantics. Each call delegates to the existing process-lifetime
//! [`ResidentProgram`](super::program::ResidentProgram) owner.
//! Deployment composition still supplies the connected carrier and deadlines;
//! endpoint discovery and listener lifecycle remain outside the application.

use std::time::Instant;

use pale_ports::{LocalIpcFramePort, PrincipalWriterGuardPort};

use super::{
    consult_recovery, program,
    resident_frame_ingress::{ResidentFrameIngressError, ResidentProgramFrameOutcome},
    ResidentProcessInvokePolicyEvaluator,
};

/// Opaque callable resident assembled by the Pale application.
///
/// The façade owns the process-lifetime program. It exposes one bounded frame
/// turn and one bounded recovery turn without exposing stores, keys, trust
/// frontiers, policy evaluators, or private continuations.
pub struct ResidentFrameService<'service> {
    program: Box<dyn ResidentFrameProgram + 'service>,
}

impl<'service> ResidentFrameService<'service> {
    pub(super) fn from_program(program: impl ResidentFrameProgram + 'service) -> Self {
        Self {
            program: Box::new(program),
        }
    }

    /// Read, classify, and serve one resident request frame.
    ///
    /// The resident ingress admits exactly one hook, Observe or operation
    /// family. It never tries a second family after a failed decode.
    ///
    /// # Errors
    ///
    /// Returns a bounded turn failure when framing, peer admission, protected
    /// application admission, or response delivery cannot complete.
    pub fn serve_frame(
        &mut self,
        carrier: &mut dyn LocalIpcFramePort,
        carrier_deadline: Instant,
        operation_response_deadline: Instant,
    ) -> Result<ResidentFrameOutcome, ResidentFrameServiceError> {
        self.program
            .serve_frame(carrier, carrier_deadline, operation_response_deadline)
            .map(Into::into)
            .map_err(|_| ResidentFrameServiceError::TurnUnavailable)
    }

    /// Advance at most one retained recovery source.
    ///
    /// # Errors
    ///
    /// Returns a bounded recovery failure. The underlying program retains its
    /// fair source cursor even when the selected recovery owner fails.
    pub fn recover_one_bounded_source(&mut self) -> Result<(), ResidentFrameServiceError> {
        self.program
            .recover_one_bounded_source()
            .map_err(|_| ResidentFrameServiceError::RecoveryUnavailable)
    }

    pub(super) fn has_retained_phase(&self) -> bool {
        self.program.has_retained_phase()
    }

    pub(super) fn continue_one_retained_phase(
        &mut self,
    ) -> program::ResidentProgramRetainedPhaseTurn {
        self.program.continue_one_retained_phase()
    }
}

/// Existing resident request family selected for one completed frame turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentFrameOutcome {
    /// A native hook reached ingress; inspect its response for availability.
    Hook,
    /// The frame reached the read-only Observe owner.
    Observe,
    /// The frame reached an Invoke, Consult, Delegate, or Cancel owner.
    Operation,
}

impl From<ResidentProgramFrameOutcome> for ResidentFrameOutcome {
    fn from(outcome: ResidentProgramFrameOutcome) -> Self {
        match outcome {
            ResidentProgramFrameOutcome::Hook => Self::Hook,
            ResidentProgramFrameOutcome::Observe => Self::Observe,
            ResidentProgramFrameOutcome::Operation => Self::Operation,
        }
    }
}

/// Failure of one public resident service turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ResidentFrameServiceError {
    /// One framed request could not reach a complete response boundary.
    #[error("resident frame turn is unavailable")]
    TurnUnavailable,
    /// The selected bounded recovery owner did not reach a safe handoff.
    #[error("resident recovery turn is unavailable")]
    RecoveryUnavailable,
}

/// Object-safe projection of the existing generic resident program.
pub(super) trait ResidentFrameProgram {
    fn serve_frame(
        &mut self,
        carrier: &mut dyn LocalIpcFramePort,
        carrier_deadline: Instant,
        operation_response_deadline: Instant,
    ) -> Result<ResidentProgramFrameOutcome, ResidentFrameIngressError>;

    fn has_retained_phase(&self) -> bool;

    fn continue_one_retained_phase(&mut self) -> program::ResidentProgramRetainedPhaseTurn;

    fn recover_one_bounded_source(&mut self) -> Result<(), program::ResidentProgramTurnError>;
}

impl<'store, WriterGuard, PolicyEvaluator, TurnOwner, StopInput> ResidentFrameProgram
    for program::ResidentProgram<'store, WriterGuard, PolicyEvaluator, TurnOwner, StopInput>
where
    WriterGuard: PrincipalWriterGuardPort,
    PolicyEvaluator: consult_recovery::ResidentConsultationPolicyEvaluator<StopInput>
        + ResidentProcessInvokePolicyEvaluator,
    TurnOwner: program::ResidentProgramTurnOwner<WriterGuard, PolicyEvaluator, StopInput>
        + program::ResidentProgramObserveTurnOwner<WriterGuard>,
{
    fn serve_frame(
        &mut self,
        carrier: &mut dyn LocalIpcFramePort,
        carrier_deadline: Instant,
        operation_response_deadline: Instant,
    ) -> Result<ResidentProgramFrameOutcome, ResidentFrameIngressError> {
        program::ResidentProgram::serve_resident_frame(
            self,
            carrier,
            carrier_deadline,
            operation_response_deadline,
        )
    }

    fn has_retained_phase(&self) -> bool {
        program::ResidentProgram::has_retained_phase(self)
    }

    fn continue_one_retained_phase(&mut self) -> program::ResidentProgramRetainedPhaseTurn {
        program::ResidentProgram::continue_one_retained_phase(self)
    }

    fn recover_one_bounded_source(&mut self) -> Result<(), program::ResidentProgramTurnError> {
        program::ResidentProgram::recover_one_bounded_source(self)
    }
}

impl<Program> ResidentFrameProgram for &mut Program
where
    Program: ResidentFrameProgram + ?Sized,
{
    fn serve_frame(
        &mut self,
        carrier: &mut dyn LocalIpcFramePort,
        carrier_deadline: Instant,
        operation_response_deadline: Instant,
    ) -> Result<ResidentProgramFrameOutcome, ResidentFrameIngressError> {
        (**self).serve_frame(carrier, carrier_deadline, operation_response_deadline)
    }

    fn has_retained_phase(&self) -> bool {
        (**self).has_retained_phase()
    }

    fn continue_one_retained_phase(&mut self) -> program::ResidentProgramRetainedPhaseTurn {
        (**self).continue_one_retained_phase()
    }

    fn recover_one_bounded_source(&mut self) -> Result<(), program::ResidentProgramTurnError> {
        (**self).recover_one_bounded_source()
    }
}
