//! Fresh caller admission for read-only operation observation.
//!
//! This private seam does not construct an attachment, discover an endpoint,
//! select a verification key, or implement a transport.

use pale_core::{canon, records::*};
use pale_ports::ControlRecordCoordinate;

use super::*;
use crate::resident::operation_control::validate_fresh_operation_control_request_time;
use crate::resident::{
    OperationControlAuthorizationContext, OperationControlRecoveryInput,
    OperationObservationSelection,
};

use super::request_admission::{
    admit_common_request_bytes, CallerSignatureVerification, ResidentCommonRequestAdmissionError,
};
type DisclosureAuthorization<'a> =
    dyn FnMut(OperationControlAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1> + 'a;
type ObserveReplayAdmission<'a> = dyn FnMut(
        &ResidentRequestIdV1,
        &CommonRequestEnvelopeDigest,
        &[u8],
    ) -> Result<ResidentObserveReplayDisposition, ResidentObserveReplayAdmissionError>
    + 'a;

/// Result of consulting the mandatory owner of Observe request replay state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResidentObserveReplayDisposition {
    /// This exact request identity and byte sequence was admitted for the first time.
    FirstAdmission,
    /// The same request identity was admitted previously with exactly these bytes.
    ExactReplay,
}

/// Replay admission failed before resident custody was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum ResidentObserveReplayAdmissionError {
    /// The request identity was retained with different signed bytes.
    #[error("Observe request identity was retained with different signed bytes")]
    ChangedBytes,
    /// The injected durable replay owner could not establish an answer.
    #[error("Observe replay admission is unavailable")]
    Unavailable,
}

/// Untrusted bytes and owner-resolved coordinates for one Observe request.
///
/// The saved locator and recovery input are separate so this boundary can
/// reject substitution before it lends custody, session, or anchor access.
pub(super) struct ResidentProcessObserveRequestInput<'a> {
    /// Exact caller-signed canonical request bytes.
    pub(super) request_bytes: &'a [u8],
    /// Maximum accepted request bytes.
    pub(super) maximum_request_bytes: usize,
    /// Locator supplied by the future authenticated attachment owner.
    pub(super) locator: &'a ControlRecordCoordinate,
    /// Owner-resolved protected recovery and current-head bounds.
    pub(super) recovery: OperationControlRecoveryInput<'a>,
    /// Time at which present disclosure authority is checked.
    pub(super) authorized_at: UnixTimeV1,
}

/// Existing owners needed to admit and disclose one Observe result.
pub(super) struct ResidentProcessObserveRequestAdmission<'request, 'context, 'recovery> {
    /// Complete owner-resolved admission context for the fresh request.
    pub(super) request: &'request CommonRequestAdmissionContextV1<'context>,
    /// Externally resolved caller-signature verifier; this seam selects no key.
    pub(super) verify_caller_signature: &'request mut CallerSignatureVerification<'request>,
    /// Mandatory durable replay admission; this seam owns no fallback cache.
    pub(super) admit_replay: &'request mut ObserveReplayAdmission<'request>,
    /// Existing historical operation and observation admission owners.
    pub(super) recovery: OperationControlRecoveryAdmission<'recovery>,
    /// Present disclosure decision for the recovered current head.
    pub(super) authorize_disclosure: &'recovery mut DisclosureAuthorization<'recovery>,
}

/// Failure before one authenticated read-only observation exists.
#[derive(Debug, thiserror::Error)]
pub(super) enum ResidentProcessObserveRequestError {
    /// Request bytes are absent or exceed the caller's explicit ceiling.
    #[error("Observe request bytes are absent or exceed their bound")]
    RequestBounds,
    /// Request bytes are malformed or cannot be represented canonically.
    #[error("Observe request bytes are not valid canonical JSON")]
    RequestCanonical(#[source] canon::CanonError),
    /// Request bytes are valid JSON but are not their exact canonical encoding.
    #[error("Observe request bytes are not exact canonical JSON")]
    RequestNotCanonical,
    /// Request or handle-body bytes do not satisfy their selected public contract.
    #[error("Observe request bytes do not satisfy their selected public contract")]
    RequestContract(#[source] pale_contracts::SchemaError),
    /// The contract-valid JSON could not decode into its exact wire type.
    #[error("Observe request bytes do not decode as a common request")]
    RequestWire(#[source] serde_json::Error),
    /// Owner context or caller-signature admission rejected the fresh request.
    #[error("Observe request did not pass owner admission")]
    RequestAdmission(#[source] InvalidResidentRecordV1),
    /// The durable replay owner rejected or could not classify this request.
    #[error("Observe request did not pass replay admission")]
    ReplayAdmission(#[source] ResidentObserveReplayAdmissionError),
    /// The request body, saved locator, and recovery input do not identify one handle.
    #[error("Observe request does not identify the selected operation locator")]
    LocatorAssociation,
    /// Request time, deployment selection, protected recovery, or disclosure failed.
    #[error(transparent)]
    Observation(#[from] ResidentProcessObservationError),
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Admit fresh request bytes, then run the existing read-only Observe owner.
    ///
    /// Parsing, contract checks, caller admission, the current-head requirement,
    /// locator association, and the fresh request interval all finish before
    /// custody, session, or anchor access. This method performs no write, provider
    /// call, effect, retry, discovery, or authority renewal.
    pub(super) fn observe_request<Authority>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentProcessObserveRequestInput<'_>,
        admission: ResidentProcessObserveRequestAdmission<'_, '_, '_>,
    ) -> Result<OperationControlObservation, ResidentProcessObserveRequestError> {
        let request = admit_common_request_bytes(
            input.request_bytes,
            input.maximum_request_bytes,
            admission.request,
            admission.verify_caller_signature,
        )
        .map_err(map_request_admission_error)?;
        require_current_head_and_locator(&request, input.locator, &input.recovery)?;
        validate_fresh_operation_control_request_time(&request, input.authorized_at).map_err(
            |reason| {
                ResidentProcessObserveRequestError::Observation(
                    ResidentProcessObservationError::Observe(
                        LocalOperationObservationError::Authorization(reason),
                    ),
                )
            },
        )?;
        (admission.admit_replay)(
            request.request_id(),
            request.canonical_request_sha256(),
            input.request_bytes,
        )
        .map_err(ResidentProcessObserveRequestError::ReplayAdmission)?;

        self.observe(
            selector,
            access,
            authority,
            LocalOperationObservationInput {
                recovery: input.recovery,
                request: &request,
                authorized_at: input.authorized_at,
            },
            LocalOperationObservationAdmission {
                recovery: admission.recovery,
                authorize: admission.authorize_disclosure,
            },
        )
        .map_err(Into::into)
    }
}

fn map_request_admission_error(
    error: ResidentCommonRequestAdmissionError,
) -> ResidentProcessObserveRequestError {
    match error {
        ResidentCommonRequestAdmissionError::Bounds => {
            ResidentProcessObserveRequestError::RequestBounds
        }
        ResidentCommonRequestAdmissionError::Canonical(error) => {
            ResidentProcessObserveRequestError::RequestCanonical(error)
        }
        ResidentCommonRequestAdmissionError::NotCanonical => {
            ResidentProcessObserveRequestError::RequestNotCanonical
        }
        ResidentCommonRequestAdmissionError::Contract(error) => {
            ResidentProcessObserveRequestError::RequestContract(error)
        }
        ResidentCommonRequestAdmissionError::Wire(error) => {
            ResidentProcessObserveRequestError::RequestWire(error)
        }
        ResidentCommonRequestAdmissionError::Admission(error) => {
            ResidentProcessObserveRequestError::RequestAdmission(error)
        }
    }
}

fn require_current_head_and_locator(
    request: &CommonRequestEnvelopeV1,
    locator: &ControlRecordCoordinate,
    recovery: &OperationControlRecoveryInput<'_>,
) -> Result<(), ResidentProcessObserveRequestError> {
    if !matches!(
        recovery.observation,
        OperationObservationSelection::SelectedHead
    ) || recovery.operation.expected_handle != locator
    {
        return Err(ResidentProcessObserveRequestError::LocatorAssociation);
    }

    let body = request.body();
    let schema = pale_contracts::record_schema_identity(OPERATION_HANDLE_V1_SCHEMA_ID)
        .map_err(ResidentProcessObserveRequestError::RequestContract)?;
    if body.schema_id != schema.schema || body.schema_sha256 != schema.schema_sha256 {
        return Err(ResidentProcessObserveRequestError::LocatorAssociation);
    }
    pale_contracts::validate_value(&body.payload, OPERATION_HANDLE_V1_SCHEMA_ID)
        .map_err(ResidentProcessObserveRequestError::RequestContract)?;

    let claimed = body
        .payload
        .get("canonical_operation_handle_sha256")
        .and_then(serde_json::Value::as_str)
        .and_then(|digest| OperationHandleDigest::parse(digest.to_owned()).ok())
        .ok_or(ResidentProcessObserveRequestError::LocatorAssociation)?;
    let mut preimage = body.payload.clone();
    let members = preimage
        .as_object_mut()
        .ok_or(ResidentProcessObserveRequestError::LocatorAssociation)?;
    if members
        .remove("canonical_operation_handle_sha256")
        .is_none()
        || members.remove("signature").is_none()
    {
        return Err(ResidentProcessObserveRequestError::LocatorAssociation);
    }
    let observed = OperationHandleDigest::hash_canonical(&preimage)
        .map_err(ResidentProcessObserveRequestError::RequestCanonical)?;
    if observed != claimed || claimed.as_str() != locator.record_sha256().as_str() {
        return Err(ResidentProcessObserveRequestError::LocatorAssociation);
    }
    Ok(())
}
