//! One strict ingress for resident requests and native hook observations.
//!
//! The ingress reads one physical frame, classifies it without trying another
//! family after a failed decode, and lends the exact bytes to the selected
//! hook, Observe or operation boundary. It owns no endpoint,
//! authentication policy, disclosure decision, retry, or semantic operation.

use std::{io, time::Instant};

use pale_core::canon;
use pale_ports::{
    LocalIpcCarrierError, LocalIpcCarrierOperation, LocalIpcFramePort, LocalIpcPeerCredentials,
    PrincipalWriterGuardPort, ResidentHookFrameRequest, ResidentHookFrameResponse,
    ResidentObserveFrameRequest, ResidentOperationFrameRequest,
};

use super::{
    consult_recovery, program::ResidentProgram, program::ResidentProgramObserveTurnOwner,
    program::ResidentProgramTurnError, program::ResidentProgramTurnOwner,
    ResidentProcessInvokePolicyEvaluator,
};

/// Which existing request family owns one strictly decoded frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResidentRequestFrameKind {
    Hook,
    Observe,
    Operation,
}

/// Result of one application-owned request-frame turn.
pub(in crate::resident) enum ResidentProgramFrameOutcome {
    /// One native hook reached its application ingress without granting work.
    Hook,
    /// One read-only Observe request reached the existing Observe owner.
    Observe,
    /// One operation request reached the existing operation owner.
    Operation,
}

/// Failure before or during one application-owned request-frame turn.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum ResidentFrameIngressError {
    /// The physical carrier failed before a complete frame was retained.
    #[error(transparent)]
    Carrier(#[from] LocalIpcCarrierError),
    /// The payload was absent, oversized, non-canonical, or not exactly one
    /// existing resident request family.
    #[error("resident request is not one exact bounded canonical frame")]
    Frame,
    /// The selected resident owner failed after strict frame classification.
    #[error(transparent)]
    Program(#[from] ResidentProgramTurnError),
}

/// One physical frame retained beside its supplementary peer coordinates.
struct PrefetchedResidentFrame {
    peer: LocalIpcPeerCredentials,
    caller_key: Option<[u8; 32]>,
    bytes: Box<[u8]>,
    kind: ResidentRequestFrameKind,
}

impl PrefetchedResidentFrame {
    fn receive(
        carrier: &mut dyn LocalIpcFramePort,
        deadline: Instant,
    ) -> Result<Self, ResidentFrameIngressError> {
        let peer = carrier.peer_credentials()?;
        let bytes = carrier.receive_frame(deadline)?;
        let caller_key = carrier.authenticated_caller_key();
        let kind = classify(&bytes)?;
        Ok(Self {
            peer,
            caller_key,
            bytes,
            kind,
        })
    }

    fn port<'carrier>(
        self,
        carrier: &'carrier mut dyn LocalIpcFramePort,
    ) -> PrefetchedResidentFramePort<'carrier> {
        PrefetchedResidentFramePort {
            carrier,
            peer: self.peer,
            caller_key: self.caller_key,
            bytes: Some(self.bytes),
            response_sent: false,
        }
    }
}

/// Lend one already-read frame to the existing request host.
///
/// A second read or write is a programming error made fail-closed before the
/// physical carrier is touched again.
struct PrefetchedResidentFramePort<'carrier> {
    carrier: &'carrier mut dyn LocalIpcFramePort,
    peer: LocalIpcPeerCredentials,
    caller_key: Option<[u8; 32]>,
    bytes: Option<Box<[u8]>>,
    response_sent: bool,
}

impl LocalIpcFramePort for PrefetchedResidentFramePort<'_> {
    fn authenticated_caller_key(&self) -> Option<[u8; 32]> {
        self.caller_key
    }

    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        Ok(self.peer)
    }

    fn send_frame(
        &mut self,
        payload: &[u8],
        deadline: Instant,
    ) -> Result<(), LocalIpcCarrierError> {
        if self.response_sent {
            return Err(closed_ingress_error(LocalIpcCarrierOperation::SendFrame));
        }
        self.response_sent = true;
        self.carrier.send_frame(payload, deadline)
    }

    fn receive_frame(&mut self, _: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        self.bytes
            .take()
            .ok_or_else(|| closed_ingress_error(LocalIpcCarrierOperation::ReceiveFrame))
    }
}

fn closed_ingress_error(operation: LocalIpcCarrierOperation) -> LocalIpcCarrierError {
    LocalIpcCarrierError::io(
        operation,
        io::Error::other("resident ingress permits one request and one response"),
    )
}

fn classify(bytes: &[u8]) -> Result<ResidentRequestFrameKind, ResidentFrameIngressError> {
    if bytes.is_empty() || bytes.len() > super::MAX_RESIDENT_REQUEST_FRAME_BYTES {
        return Err(ResidentFrameIngressError::Frame);
    }
    let value = canon::parse_strict(bytes).map_err(|_| ResidentFrameIngressError::Frame)?;
    let canonical =
        canon::to_canonical_string(&value).map_err(|_| ResidentFrameIngressError::Frame)?;
    if canonical.as_bytes() != bytes {
        return Err(ResidentFrameIngressError::Frame);
    }
    let Some(object) = value.as_object() else {
        return Err(ResidentFrameIngressError::Frame);
    };
    if object.get("kind").and_then(serde_json::Value::as_str) == Some("hook") {
        serde_json::from_value::<ResidentHookFrameRequest>(value)
            .map(|_| ResidentRequestFrameKind::Hook)
            .map_err(|_| ResidentFrameIngressError::Frame)
    } else if object.contains_key("kind") {
        serde_json::from_value::<ResidentOperationFrameRequest>(value)
            .map(|_| ResidentRequestFrameKind::Operation)
            .map_err(|_| ResidentFrameIngressError::Frame)
    } else {
        serde_json::from_value::<ResidentObserveFrameRequest>(value)
            .map(|_| ResidentRequestFrameKind::Observe)
            .map_err(|_| ResidentFrameIngressError::Frame)
    }
}

impl<'store, WriterGuard, PolicyEvaluator, TurnOwner, StopInput>
    ResidentProgram<'store, WriterGuard, PolicyEvaluator, TurnOwner, StopInput>
where
    WriterGuard: PrincipalWriterGuardPort,
    PolicyEvaluator: consult_recovery::ResidentConsultationPolicyEvaluator<StopInput>
        + ResidentProcessInvokePolicyEvaluator,
    TurnOwner: ResidentProgramTurnOwner<WriterGuard, PolicyEvaluator, StopInput>
        + ResidentProgramObserveTurnOwner<WriterGuard>,
{
    /// Read, classify, and serve one existing resident request frame.
    ///
    /// A valid request reaches exactly one existing semantic owner and can
    /// produce at most one response. Invalid or ambiguous bytes produce no
    /// response because no response family has been established.
    pub(in crate::resident) fn serve_resident_frame(
        &mut self,
        carrier: &mut dyn LocalIpcFramePort,
        carrier_deadline: Instant,
        operation_response_deadline: Instant,
    ) -> Result<ResidentProgramFrameOutcome, ResidentFrameIngressError> {
        let frame = PrefetchedResidentFrame::receive(carrier, carrier_deadline)?;
        let kind = frame.kind;
        let mut port = frame.port(carrier);
        match kind {
            ResidentRequestFrameKind::Hook => {
                let request = crate::resident::AuthenticatedResidentHookRequest::receive(
                    &mut port,
                    carrier_deadline,
                )
                .map_err(|_| ResidentFrameIngressError::Frame)?;
                // A response is released only after exact dispatch custody and
                // fresh permission/disclosure admission. Sending is attempted once.
                let response = match self.admit_native_hook(port.peer, &request) {
                    Ok(response) => response,
                    Err(failure) => ResidentHookFrameResponse::Rejected { failure },
                };
                let response = canon::to_canonical_string(&response)
                    .map_err(|_| ResidentFrameIngressError::Frame)?;
                port.send_frame(response.as_bytes(), carrier_deadline)?;
                Ok(ResidentProgramFrameOutcome::Hook)
            }
            ResidentRequestFrameKind::Observe => self
                .serve_observe_frame(&mut port, carrier_deadline)
                .map(|_| ResidentProgramFrameOutcome::Observe)
                .map_err(Into::into),
            ResidentRequestFrameKind::Operation => self
                .serve_operation_frame(&mut port, carrier_deadline, operation_response_deadline)
                .map(|_| ResidentProgramFrameOutcome::Operation)
                .map_err(Into::into),
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/resident_frame_ingress.rs"]
mod tests;
