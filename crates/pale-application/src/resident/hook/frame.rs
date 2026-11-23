//! Decode a hook and retain the authentication of the frame that supplied it.

use std::time::Instant;

use pale_core::canon;
use pale_ports::{LocalIpcCarrierError, LocalIpcFramePort, ResidentHookFrameRequest};

use crate::hooks::RepositoryHookRoutingV1;

use super::{
    CallerBoundResidentHookRequest, ResidentHookCallerBinding, ResidentHookIngressRequest,
    ResidentHookSessionAdmissionError,
};

/// A parsed native event inseparable from its authenticated frame's caller key.
///
/// Construction reads one frame and captures its verified key after that read.
/// It accepts no separately parsed event or caller-supplied key. Connection
/// authentication still grants no session, operation or disclosure permission.
#[must_use = "an authenticated hook must reach current admission or a rejection"]
pub struct AuthenticatedResidentHookRequest {
    pub(super) request: ResidentHookIngressRequest,
    pub(super) caller_key: [u8; 32],
}

impl AuthenticatedResidentHookRequest {
    /// Borrow the captured event for exact matching before protected reads.
    /// Authentication alone supplies no session or operation authority.
    pub(in crate::resident) const fn request(&self) -> &ResidentHookIngressRequest {
        &self.request
    }

    /// Exact key retained from the authenticated native occurrence.
    pub(in crate::resident) const fn caller_key(&self) -> [u8; 32] {
        self.caller_key
    }

    /// Receive one canonical hook frame from the selected authenticated carrier.
    ///
    /// # Errors
    /// Rejects missing authentication, malformed routing or events, duplicate
    /// keys, unknown frame fields, noncanonical frames and oversized input.
    /// No failure reads session custody or starts a native operation.
    pub fn receive(
        carrier: &mut dyn LocalIpcFramePort,
        deadline: Instant,
    ) -> Result<Self, ResidentHookFrameAdmissionError> {
        let bytes = carrier.receive_frame(deadline)?;
        let caller_key = carrier
            .authenticated_caller_key()
            .ok_or(ResidentHookFrameAdmissionError::Unauthenticated)?;
        let request = decode(&bytes)?;
        Ok(Self {
            request,
            caller_key,
        })
    }

    /// Join this exact event to a separately selected admitted-session binding.
    ///
    /// # Errors
    /// Rejects a different caller or native activity. Current session recovery,
    /// operation registration and response permission remain separate checks.
    pub fn bind_to<'binding>(
        &self,
        binding: &'binding ResidentHookCallerBinding,
    ) -> Result<CallerBoundResidentHookRequest<'binding, '_>, ResidentHookSessionAdmissionError>
    {
        binding.admit_authenticated_request(Some(self.caller_key), &self.request)
    }
}

impl std::fmt::Debug for AuthenticatedResidentHookRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AuthenticatedResidentHookRequest(<private event>)")
    }
}

/// A hook frame failed before current session or operation admission.
#[derive(Debug, thiserror::Error)]
pub enum ResidentHookFrameAdmissionError {
    /// The physical carrier did not yield one complete frame.
    #[error(transparent)]
    Carrier(#[from] LocalIpcCarrierError),
    /// This exact frame has no verified connection key.
    #[error("resident hook frame has no authenticated caller")]
    Unauthenticated,
    /// The bounded frame, command or event did not match the strict grammar.
    #[error("resident hook frame is malformed or exceeds its bound")]
    InvalidFrame,
}

pub(in crate::resident) fn decode(
    bytes: &[u8],
) -> Result<ResidentHookIngressRequest, ResidentHookFrameAdmissionError> {
    use ResidentHookFrameAdmissionError::InvalidFrame;
    if bytes.is_empty()
        || bytes.len() > crate::resident::process_host::MAX_RESIDENT_REQUEST_FRAME_BYTES
    {
        return Err(InvalidFrame);
    }
    let value = canon::parse_strict(bytes).map_err(|_| InvalidFrame)?;
    if canon::to_canonical_string(&value)
        .map_err(|_| InvalidFrame)?
        .as_bytes()
        != bytes
    {
        return Err(InvalidFrame);
    }
    let frame: ResidentHookFrameRequest =
        serde_json::from_value(value).map_err(|_| InvalidFrame)?;
    // Preserve native formatting while rejecting repeated keys that another
    // consumer could interpret as a different event.
    canon::parse_strict(frame.native_event()).map_err(|_| InvalidFrame)?;
    let routing = RepositoryHookRoutingV1::parse(frame.arguments()).map_err(|_| InvalidFrame)?;
    ResidentHookIngressRequest::admit(routing, frame.native_event()).map_err(|_| InvalidFrame)
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/hook/frame.rs"]
mod tests;
