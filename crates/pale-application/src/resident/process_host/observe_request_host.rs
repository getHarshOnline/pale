//! One injected framed host for authenticated read-only Observe requests.
//!
//! This private composition seam owns neither endpoint discovery nor any
//! resident, store, key, deployment, replay database, or disclosure policy.
//! All semantic owners are supplied for one already selected request.

use std::time::Instant;

use pale_core::{canon, records::*};
use pale_ports::{
    LocalIpcCarrierError, LocalIpcFramePort, LocalIpcPeerCredentials, ResidentObserveFrameFailure,
    ResidentObserveFrameRequest, ResidentObserveFrameResponse,
};

use super::observe_request::{
    ResidentObserveReplayAdmissionError, ResidentProcessObserveRequestAdmission,
    ResidentProcessObserveRequestError, ResidentProcessObserveRequestInput,
};
use super::*;

/// Maximum canonical bytes accepted for the private local Observe projection.
type PeerAdmission<'a, Witness> = dyn FnMut(
        LocalIpcPeerCredentials,
        &ResidentObserveFrameRequest,
    ) -> Result<Witness, InvalidResidentRecordV1>
    + 'a;

/// One received frame and its supplementary kernel peer coordinates.
#[derive(Debug)]
pub(super) struct ReceivedResidentObserveFrame {
    peer: LocalIpcPeerCredentials,
    request: ResidentObserveFrameRequest,
}

/// One Observe frame exposed only after supplementary peer admission.
#[derive(Debug)]
pub(super) struct AdmittedResidentObserveFrame<Witness> {
    peer: Witness,
    request: ResidentObserveFrameRequest,
}

impl<Witness> AdmittedResidentObserveFrame<Witness> {
    /// Borrow the application-owned peer admission witness.
    pub(super) const fn peer(&self) -> &Witness {
        &self.peer
    }

    /// Borrow the decoded request after peer admission.
    pub(super) const fn request(&self) -> &ResidentObserveFrameRequest {
        &self.request
    }
}

/// Already resolved application owners for one framed Observe request.
///
/// Grouping these borrows makes the request boundary explicit without giving
/// the host authority to discover or manufacture any of them.
pub(super) struct ResidentObserveRequestResolution<
    'selector,
    'access,
    'authority,
    'input,
    'request,
    'recovery,
    'authorization,
    Authority,
> {
    pub(super) selector: &'selector ResidentDeploymentAuthoritySelector,
    pub(super) access: ResidentDeploymentOperationAccess<'access>,
    pub(super) authority: &'authority mut Authority,
    pub(super) input: ResidentProcessObserveRequestInput<'input>,
    pub(super) admission:
        ResidentProcessObserveRequestAdmission<'request, 'recovery, 'authorization>,
    /// Absolute monotonic deadline already clamped by the trusted request-time owner.
    pub(super) response_deadline: Instant,
}

/// One private host over an exclusively borrowed framed connection.
///
/// Construction requires the caller to supply the absolute carrier ceiling.
/// It deliberately cannot discover an endpoint or manufacture a replay owner.
pub(super) struct ResidentObserveRequestHost<'carrier> {
    carrier: &'carrier mut dyn LocalIpcFramePort,
    carrier_deadline: Instant,
}

/// Failure at the private framed-host boundary.
#[derive(Debug, thiserror::Error)]
pub(super) enum ResidentObserveRequestHostError {
    /// The physical local carrier failed or lost frame alignment.
    #[error(transparent)]
    Carrier(#[from] LocalIpcCarrierError),
    /// Frame bytes were absent, oversized, malformed, or non-canonical.
    #[error("resident Observe frame is not exact bounded canonical JSON")]
    Frame,
    /// The resolved request does not match the received frame.
    #[error("resident Observe frame does not match the resolved request")]
    ResolutionAssociation,
    /// Supplementary peer evidence was rejected by the injected resolver.
    #[error("resident Observe peer evidence was rejected")]
    PeerAdmission(#[source] InvalidResidentRecordV1),
}

impl<'carrier> ResidentObserveRequestHost<'carrier> {
    fn send_response(
        &mut self,
        response: &ResidentObserveFrameResponse,
        response_deadline: Instant,
    ) -> Result<(), ResidentObserveRequestHostError> {
        let bytes = canon::to_canonical_string(response)
            .map_err(|_| ResidentObserveRequestHostError::Frame)?;
        if bytes.len() > super::MAX_RESIDENT_REQUEST_FRAME_BYTES {
            return Err(ResidentObserveRequestHostError::Frame);
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
    /// non-canonical frame payloads. No resident custody is read.
    pub(super) fn receive(
        &mut self,
    ) -> Result<ReceivedResidentObserveFrame, ResidentObserveRequestHostError> {
        let peer = self.carrier.peer_credentials()?;
        let bytes = self.carrier.receive_frame(self.carrier_deadline)?;
        if bytes.is_empty() || bytes.len() > super::MAX_RESIDENT_REQUEST_FRAME_BYTES {
            return Err(ResidentObserveRequestHostError::Frame);
        }
        let value =
            canon::parse_strict(&bytes).map_err(|_| ResidentObserveRequestHostError::Frame)?;
        let canonical = canon::to_canonical_string(&value)
            .map_err(|_| ResidentObserveRequestHostError::Frame)?;
        if canonical.as_bytes() != bytes.as_ref() {
            return Err(ResidentObserveRequestHostError::Frame);
        }
        let request =
            serde_json::from_value(value).map_err(|_| ResidentObserveRequestHostError::Frame)?;
        Ok(ReceivedResidentObserveFrame { peer, request })
    }

    /// Admit supplementary peer evidence before semantic resolution or store access.
    pub(super) fn admit_peer<Witness>(
        &mut self,
        received: ReceivedResidentObserveFrame,
        admit_peer: &mut PeerAdmission<'_, Witness>,
    ) -> Result<AdmittedResidentObserveFrame<Witness>, ResidentObserveRequestHostError> {
        let peer = admit_peer(received.peer, &received.request)
            .map_err(ResidentObserveRequestHostError::PeerAdmission)?;
        Ok(AdmittedResidentObserveFrame {
            peer,
            request: received.request,
        })
    }

    /// Admit and answer one already resolved request through the existing owner.
    ///
    /// A resolver must supply all application authorities, protected recovery
    /// inputs, and the mandatory durable replay admission. Kernel peer
    /// coordinates are supplementary evidence only. A rejection is returned as
    /// one bounded frame without exposing application internals.
    ///
    /// # Errors
    ///
    /// Returns only framing, association, peer-admission, or carrier failures.
    /// Application admission failures are encoded as a typed rejection frame.
    pub(super) fn answer_admitted<Authority, Witness>(
        &mut self,
        admitted: &AdmittedResidentObserveFrame<Witness>,
        resident: &mut ResidentProcessHost<'_, impl Sized>,
        resolution: ResidentObserveRequestResolution<'_, '_, '_, '_, '_, '_, '_, Authority>,
    ) -> Result<Option<OperationControlObservation>, ResidentObserveRequestHostError> {
        let ResidentObserveRequestResolution {
            selector,
            access,
            authority,
            input,
            admission,
            response_deadline,
        } = resolution;
        if admitted.request.operation_handle_record_ref() != input.locator.record_ref().as_str()
            || admitted.request.operation_handle_sha256() != input.locator.record_sha256().as_str()
            || admitted.request.signed_common_request().as_bytes() != input.request_bytes
        {
            return Err(ResidentObserveRequestHostError::ResolutionAssociation);
        }
        let response_deadline = self.carrier_deadline.min(response_deadline);

        self.answer_admitted_with(admitted, response_deadline, |_| {
            resident
                .observe_request(selector, access, authority, input, admission)
                .map(|observed| (observed, response_deadline))
                .map_err(|error| frame_failure(&error))
        })
    }

    /// Project one application-owned Observe answer through the existing frame.
    ///
    /// The callback must use the admitted request. This method owns only the
    /// stable response projection and the one-send carrier law.
    pub(super) fn answer_admitted_with<Witness>(
        &mut self,
        admitted: &AdmittedResidentObserveFrame<Witness>,
        response_deadline: Instant,
        answer: impl FnOnce(
            &ResidentObserveFrameRequest,
        ) -> Result<
            (OperationControlObservation, Instant),
            ResidentObserveFrameFailure,
        >,
    ) -> Result<Option<OperationControlObservation>, ResidentObserveRequestHostError> {
        let (response, observed, selected_deadline) = match answer(admitted.request()) {
            Ok((observed, selected_deadline)) => (
                observed_response(&observed).map_err(|_| ResidentObserveRequestHostError::Frame)?,
                Some(observed),
                selected_deadline,
            ),
            Err(failure) => (
                ResidentObserveFrameResponse::Rejected { failure },
                None,
                response_deadline,
            ),
        };
        self.send_response(&response, self.carrier_deadline.min(selected_deadline))?;
        Ok(observed)
    }

    /// Return one bounded application rejection after peer admission.
    ///
    /// Resolution failures use the same closed frame family as failures from
    /// the Observe owner. No fallback deployment or retry is attempted.
    pub(super) fn reject_admitted<Witness>(
        &mut self,
        _admitted: &AdmittedResidentObserveFrame<Witness>,
        response_deadline: Instant,
        failure: ResidentObserveFrameFailure,
    ) -> Result<(), ResidentObserveRequestHostError> {
        self.send_response(
            &ResidentObserveFrameResponse::Rejected { failure },
            response_deadline,
        )
    }
}

fn observed_response(
    observed: &OperationControlObservation,
) -> Result<ResidentObserveFrameResponse, canon::CanonError> {
    let (operation_ref, operation) = observed.operation();
    let (observation_ref, observation) = observed.observation();
    Ok(ResidentObserveFrameResponse::Observed {
        operation_handle_record_ref: operation_ref.as_str().to_owned(),
        operation_handle_canonical: canon::to_canonical_string(operation)?,
        operation_observation_record_ref: observation_ref.as_str().to_owned(),
        operation_observation_canonical: canon::to_canonical_string(observation)?,
    })
}

pub(super) fn frame_failure(
    error: &ResidentProcessObserveRequestError,
) -> ResidentObserveFrameFailure {
    match error {
        ResidentProcessObserveRequestError::RequestBounds
        | ResidentProcessObserveRequestError::RequestCanonical(_)
        | ResidentProcessObserveRequestError::RequestNotCanonical
        | ResidentProcessObserveRequestError::RequestContract(_)
        | ResidentProcessObserveRequestError::RequestWire(_)
        | ResidentProcessObserveRequestError::LocatorAssociation => {
            ResidentObserveFrameFailure::InvalidRequest
        }
        ResidentProcessObserveRequestError::ReplayAdmission(
            ResidentObserveReplayAdmissionError::ChangedBytes,
        ) => ResidentObserveFrameFailure::ReplayConflict,
        ResidentProcessObserveRequestError::RequestAdmission(_) => {
            ResidentObserveFrameFailure::AdmissionDenied
        }
        ResidentProcessObserveRequestError::Observation(
            ResidentProcessObservationError::Selection(_)
            | ResidentProcessObservationError::Observe(
                LocalOperationObservationError::Authorization(_),
            ),
        ) => ResidentObserveFrameFailure::AdmissionDenied,
        ResidentProcessObserveRequestError::ReplayAdmission(
            ResidentObserveReplayAdmissionError::Unavailable,
        )
        | ResidentProcessObserveRequestError::Observation(
            ResidentProcessObservationError::Observe(LocalOperationObservationError::Recovery(_)),
        ) => ResidentObserveFrameFailure::ResidentUnavailable,
    }
}
