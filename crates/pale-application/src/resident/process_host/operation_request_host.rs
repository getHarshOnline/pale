//! One injected framed host for bounded Invoke, Consult, Delegate, and Cancel requests.
//!
//! This private ingress handles one request and one bounded application turn.
//! It does not select a roster, parent policy, provider, A2A route, deployment,
//! endpoint, signer, replay owner, or recovery owner. The injected handler must
//! call the existing resident operation owners and return their honest boundary.

use std::time::Instant;

use pale_core::canon;
use pale_ports::{
    LocalIpcCarrierError, LocalIpcFramePort, LocalIpcPeerCredentials,
    ResidentOperationFrameCancellationDisposition, ResidentOperationFrameFailure,
    ResidentOperationFrameLocator, ResidentOperationFrameRecords, ResidentOperationFrameRequest,
    ResidentOperationFrameResponse, ResidentOperationFrameUnsupportedReason,
};

use super::*;

/// Maximum canonical bytes accepted for one private local operation frame.
type PeerAdmission<'a, Witness> = dyn FnMut(
        LocalIpcPeerCredentials,
        &ResidentOperationFrameRequest,
    ) -> Result<Witness, InvalidResidentRecordV1>
    + 'a;

/// One decoded frame and its supplementary kernel peer coordinates.
#[derive(Debug)]
pub(super) struct ReceivedResidentOperationFrame {
    peer: LocalIpcPeerCredentials,
    caller_key: Option<[u8; 32]>,
    request: ResidentOperationFrameRequest,
}

/// One decoded frame whose supplementary kernel peer evidence was admitted.
///
/// Only this witness exposes the request to semantic resolution. The type makes
/// protected resolver or store work structurally unavailable before peer
/// admission.
#[derive(Debug)]
pub(super) struct AdmittedResidentOperationFrame<Witness> {
    peer: Witness,
    caller_key: Option<[u8; 32]>,
    request: ResidentOperationFrameRequest,
}

impl<Witness> AdmittedResidentOperationFrame<Witness> {
    /// Verified transport key captured after this exact frame was read.
    pub(super) const fn caller_key(&self) -> Option<[u8; 32]> {
        self.caller_key
    }

    /// Borrow the application-owned peer admission witness.
    pub(super) const fn peer(&self) -> &Witness {
        &self.peer
    }

    /// Borrow the exact decoded request after peer admission.
    pub(super) const fn request(&self) -> &ResidentOperationFrameRequest {
        &self.request
    }
}

/// One private host over an exclusively borrowed framed connection.
pub(super) struct ResidentOperationRequestHost<'carrier> {
    carrier: &'carrier mut dyn LocalIpcFramePort,
    carrier_deadline: Instant,
}

/// Failure before one complete request or response frame crosses the host.
#[derive(Debug, thiserror::Error)]
pub(super) enum ResidentOperationRequestHostError {
    /// The physical local carrier failed or lost frame alignment.
    #[error(transparent)]
    Carrier(#[from] LocalIpcCarrierError),
    /// Frame bytes were absent, oversized, malformed, or non-canonical.
    #[error("resident operation frame is not exact bounded canonical JSON")]
    Frame,
    /// Supplementary peer evidence was rejected by the injected resolver.
    #[error("resident operation peer evidence was rejected")]
    PeerAdmission(#[source] InvalidResidentRecordV1),
    /// The admitted semantic request or owner coordinate differs from the frame.
    #[error("resident operation frame does not match its admitted owner resolution")]
    ResolutionAssociation,
}

/// Exact bounded journey selected by an already-admitted application owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResidentOperationRequestKind {
    StartInvoke,
    AdvanceInvoke,
    StartConsult,
    AdvanceConsult,
    StartDelegate,
    AdvanceDelegate,
    Cancel,
}

/// Admitted semantic request and owner coordinate for one received frame.
///
/// Construction does not admit bytes, select a route, or choose policy. The
/// caller supplies a contextually admitted request plus the exact coordinate
/// already selected by the corresponding start, recovery, dispatch, or
/// cancellation owner. [`ResidentOperationRequestHost::answer_admitted`] then
/// proves that the received bytes identify this same resolution before any
/// owner runs.
pub(super) struct ResidentOperationRequestResolution<'request> {
    kind: ResidentOperationRequestKind,
    request: &'request CommonRequestEnvelopeV1,
    locator: Option<ResidentOperationFrameLocator>,
}

impl<'request> ResidentOperationRequestResolution<'request> {
    /// Resolve a root start that carries no pre-existing operation coordinate.
    #[must_use]
    pub(super) const fn root_start(
        kind: ResidentOperationRequestKind,
        request: &'request CommonRequestEnvelopeV1,
    ) -> Self {
        Self {
            kind,
            request,
            locator: None,
        }
    }

    /// Resolve a child start against its exact admitted parent coordinate.
    #[must_use]
    pub(super) const fn child_start(
        request: &'request CommonRequestEnvelopeV1,
        parent: ResidentOperationFrameLocator,
    ) -> Self {
        Self {
            kind: ResidentOperationRequestKind::StartDelegate,
            request,
            locator: Some(parent),
        }
    }

    /// Resolve one continuation or cancellation against its retained handle.
    #[must_use]
    pub(super) const fn retained(
        kind: ResidentOperationRequestKind,
        request: &'request CommonRequestEnvelopeV1,
        locator: ResidentOperationFrameLocator,
    ) -> Self {
        Self {
            kind,
            request,
            locator: Some(locator),
        }
    }

    pub(super) fn admits(
        &self,
        request: &ResidentOperationFrameRequest,
    ) -> Result<bool, canon::CanonError> {
        let canonical_request = canon::to_canonical_string(self.request)?;
        if canonical_request != request.signed_common_request() {
            return Ok(false);
        }
        Ok(match (self.kind, request, self.locator.as_ref()) {
            (
                ResidentOperationRequestKind::StartInvoke,
                ResidentOperationFrameRequest::StartInvoke { .. },
                None,
            )
            | (
                ResidentOperationRequestKind::StartConsult,
                ResidentOperationFrameRequest::StartConsult { .. },
                None,
            ) => true,
            (
                ResidentOperationRequestKind::AdvanceInvoke,
                ResidentOperationFrameRequest::AdvanceInvoke { locator, .. },
                Some(expected),
            )
            | (
                ResidentOperationRequestKind::AdvanceConsult,
                ResidentOperationFrameRequest::AdvanceConsult { locator, .. },
                Some(expected),
            )
            | (
                ResidentOperationRequestKind::StartDelegate,
                ResidentOperationFrameRequest::StartDelegate {
                    parent: locator, ..
                },
                Some(expected),
            )
            | (
                ResidentOperationRequestKind::AdvanceDelegate,
                ResidentOperationFrameRequest::AdvanceDelegate { locator, .. },
                Some(expected),
            )
            | (
                ResidentOperationRequestKind::Cancel,
                ResidentOperationFrameRequest::Cancel { locator, .. },
                Some(expected),
            ) => locator == expected,
            _ => false,
        })
    }
}

/// Closed application result of one bounded resident request.
///
/// This is deliberately private. It prevents the transport frame from becoming
/// a second semantic owner while preserving every honest stopping condition.
pub(super) enum ResidentOperationRequestOutcome {
    /// A new operation and initial observation reached durable custody.
    Started(ResidentOperationFrameRecords),
    /// Return the exact current records after this bounded turn.
    Observed(ResidentOperationFrameRecords),
    /// An existing cancellation owner returned one exact disposition.
    CancellationDisposition {
        /// Exact cancellation target.
        locator: ResidentOperationFrameLocator,
        /// What the cancellation owner established.
        disposition: ResidentOperationFrameCancellationDisposition,
    },
    /// The operation remains valid but needs another bounded turn.
    Pending(ResidentOperationFrameLocator),
    /// The compiled constitution selected no Delegate route.
    Unsupported {
        /// Exact retained operation coordinate.
        locator: ResidentOperationFrameLocator,
        /// Owner boundary that prevented progress.
        reason: ResidentOperationFrameUnsupportedReason,
    },
}

impl ResidentOperationRequestOutcome {
    fn into_frame(self) -> ResidentOperationFrameResponse {
        match self {
            Self::Started(records) => ResidentOperationFrameResponse::Started { records },
            Self::Observed(records) => ResidentOperationFrameResponse::Observed { records },
            Self::CancellationDisposition {
                locator,
                disposition,
            } => ResidentOperationFrameResponse::CancellationDisposition {
                locator,
                disposition,
            },
            Self::Pending(locator) => ResidentOperationFrameResponse::Pending { locator },
            Self::Unsupported { locator, reason } => {
                ResidentOperationFrameResponse::Unsupported { locator, reason }
            }
        }
    }
}

/// Project one durably issued start into the exact local response records.
///
/// Both records already passed the operation start's database and independent
/// anchor custody. This projection performs no recovery read and does not imply
/// a later operation state.
pub(super) fn issued_start_records(
    operation: &crate::resident::IssuedLocalOperation,
) -> Result<ResidentOperationFrameRecords, canon::CanonError> {
    let (handle_ref, handle) = operation.handle();
    let (observation_ref, observation) = operation.initial_observation();
    Ok(ResidentOperationFrameRecords::new(
        handle_ref.as_str().to_owned(),
        canon::to_canonical_string(handle)?,
        observation_ref.as_str().to_owned(),
        canon::to_canonical_string(observation)?,
    ))
}

/// Project one protected recovery result into exact response records.
///
/// Recovery already admitted both records and selected the current observation.
/// This projection performs no additional read, disclosure decision, or state
/// transition.
pub(super) fn recovered_control_records(
    control: &crate::resident::RecoveredOperationControl,
) -> Result<ResidentOperationFrameRecords, canon::CanonError> {
    let (handle_ref, handle) = control.operation().operation().handle();
    let (observation_ref, observation) = control.observation().selected();
    Ok(ResidentOperationFrameRecords::new(
        handle_ref.as_str().to_owned(),
        canon::to_canonical_string(handle)?,
        observation_ref.as_str().to_owned(),
        canon::to_canonical_string(observation)?,
    ))
}

impl<'carrier> ResidentOperationRequestHost<'carrier> {
    fn send_response(
        &mut self,
        response: &ResidentOperationFrameResponse,
        response_deadline: Instant,
    ) -> Result<(), ResidentOperationRequestHostError> {
        let bytes = canon::to_canonical_string(response)
            .map_err(|_| ResidentOperationRequestHostError::Frame)?;
        if bytes.len() > super::MAX_RESIDENT_REQUEST_FRAME_BYTES {
            return Err(ResidentOperationRequestHostError::Frame);
        }
        self.carrier.send_frame(
            bytes.as_bytes(),
            self.carrier_deadline.min(response_deadline),
        )?;
        Ok(())
    }

    /// Borrow one already connected frame port under an absolute host ceiling.
    #[must_use]
    pub(super) fn new(
        carrier: &'carrier mut dyn LocalIpcFramePort,
        carrier_deadline: Instant,
    ) -> Self {
        Self {
            carrier,
            carrier_deadline,
        }
    }

    /// Receive and decode one request without selecting any semantic owner.
    ///
    /// # Errors
    ///
    /// Returns a carrier failure or rejects absent, oversized, malformed, and
    /// non-canonical payloads. No resident state is read.
    pub(super) fn receive(
        &mut self,
    ) -> Result<ReceivedResidentOperationFrame, ResidentOperationRequestHostError> {
        let peer = self.carrier.peer_credentials()?;
        let bytes = self.carrier.receive_frame(self.carrier_deadline)?;
        let caller_key = self.carrier.authenticated_caller_key();
        if bytes.is_empty() || bytes.len() > super::MAX_RESIDENT_REQUEST_FRAME_BYTES {
            return Err(ResidentOperationRequestHostError::Frame);
        }
        let value =
            canon::parse_strict(&bytes).map_err(|_| ResidentOperationRequestHostError::Frame)?;
        let canonical = canon::to_canonical_string(&value)
            .map_err(|_| ResidentOperationRequestHostError::Frame)?;
        if canonical.as_bytes() != bytes.as_ref() {
            return Err(ResidentOperationRequestHostError::Frame);
        }
        let request =
            serde_json::from_value(value).map_err(|_| ResidentOperationRequestHostError::Frame)?;
        if request_signed_bytes(&request).is_empty() {
            return Err(ResidentOperationRequestHostError::Frame);
        }
        Ok(ReceivedResidentOperationFrame {
            peer,
            caller_key,
            request,
        })
    }

    /// Admit supplementary peer evidence before exposing the request.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentOperationRequestHostError::PeerAdmission`] when the
    /// injected local-transport owner rejects the peer/request pair.
    pub(super) fn admit_peer<Witness>(
        &mut self,
        received: ReceivedResidentOperationFrame,
        admit_peer: &mut PeerAdmission<'_, Witness>,
    ) -> Result<AdmittedResidentOperationFrame<Witness>, ResidentOperationRequestHostError> {
        let peer = admit_peer(received.peer, &received.request)
            .map_err(ResidentOperationRequestHostError::PeerAdmission)?;
        Ok(AdmittedResidentOperationFrame {
            peer,
            caller_key: received.caller_key,
            request: received.request,
        })
    }

    /// Admit and answer one request through its already-resolved application owner.
    ///
    /// The handler receives the process host so it cannot bypass deployment,
    /// task, recovery, or custody ownership. It must execute at most one bounded
    /// turn and return a closed application result. This method does not retry
    /// it or reinterpret `Pending` or `Unsupported`.
    ///
    /// # Errors
    ///
    /// Returns a frame-association, peer-admission, canonical encoding, or
    /// carrier failure. Application rejections are returned as a bounded
    /// rejection frame.
    pub(super) fn answer_admitted<WriterGuard, Witness, Handle>(
        &mut self,
        admitted: AdmittedResidentOperationFrame<Witness>,
        resident: &mut ResidentProcessHost<'_, WriterGuard>,
        resolution: ResidentOperationRequestResolution<'_>,
        response_deadline: Instant,
        handle: Handle,
    ) -> Result<ResidentOperationFrameResponse, ResidentOperationRequestHostError>
    where
        Handle: FnOnce(
            &mut ResidentProcessHost<'_, WriterGuard>,
            &CommonRequestEnvelopeV1,
        )
            -> Result<ResidentOperationRequestOutcome, ResidentOperationFrameFailure>,
    {
        if !resolution
            .admits(&admitted.request)
            .map_err(|_| ResidentOperationRequestHostError::ResolutionAssociation)?
        {
            return Err(ResidentOperationRequestHostError::ResolutionAssociation);
        }
        let response = match handle(resident, resolution.request) {
            Ok(outcome) => outcome.into_frame(),
            Err(failure) => ResidentOperationFrameResponse::Rejected { failure },
        };
        self.send_response(&response, response_deadline)?;
        Ok(response)
    }

    /// Return one bounded application rejection after peer admission.
    ///
    /// This is used when application-owned semantic resolution cannot produce
    /// an admitted owner coordinate. It sends exactly one existing response
    /// frame and performs no retry or fallback selection.
    pub(super) fn reject_admitted<Witness>(
        &mut self,
        _admitted: AdmittedResidentOperationFrame<Witness>,
        response_deadline: Instant,
        failure: ResidentOperationFrameFailure,
    ) -> Result<ResidentOperationFrameResponse, ResidentOperationRequestHostError> {
        let response = ResidentOperationFrameResponse::Rejected { failure };
        self.send_response(&response, response_deadline)?;
        Ok(response)
    }
}

#[cfg(test)]
#[path = "../../../tests/support/resident_operation_request_host.rs"]
pub(super) mod request_host_test_support;

fn request_signed_bytes(request: &ResidentOperationFrameRequest) -> &[u8] {
    request.signed_common_request().as_bytes()
}
