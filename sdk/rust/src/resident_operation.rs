//! Start or advance an Invoke, Consult or Delegate through the resident.
//!
//! Each request has defined limits and needs a caller-supplied authenticated
//! [`ResidentAttachment`]. The SDK checks canonical records and that they belong
//! to the exact operation. Consultation policy, providers, A2A routes, recovery
//! ownership and deployment endpoints must already have been selected elsewhere.

use std::error::Error;
use std::fmt;

use pale_application::resident::OPERATION_CANCEL_CAPABILITY_ID;
use pale_core::records::{
    ArtifactDigest, AuthorityEvidenceV1, CommonRequestEnvelopeDigest, DiagnosticDispositionV1,
    DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1, OperationTargetReferenceV1,
};
use pale_ports::{
    ResidentOperationFrameCancellationDisposition, ResidentOperationFrameFailure,
    ResidentOperationFrameLocator, ResidentOperationFrameRecords, ResidentOperationFrameRequest,
    ResidentOperationFrameResponse, ResidentOperationFrameUnsupportedReason,
};
use serde::Serialize;

use crate::error::{ApiError, FailureSpecification};
use crate::resident_attachment::{
    resident_may_have_been_sent_error, FramedResidentClient, ResidentAttachmentClientError,
    ResidentRequestTransmissionCertainty,
};
use crate::resident_observation::{
    json_string, parse_canonical_record, project_response, unsigned_preimage, validate_response,
    verified_operation_handle_digest, ResidentObservationResponse,
};
use crate::{OperationHandleLocator, PaleApi, ResidentAttachment, ResidentOperationObservation};

/// Start one durable root Invoke.
#[derive(Clone)]
pub struct ResidentInvokeStartRequest {
    signed_common_request: Vec<u8>,
}

impl ResidentInvokeStartRequest {
    /// Preserve the exact pre-signed common request.
    #[must_use]
    pub fn new(signed_common_request: Vec<u8>) -> Self {
        Self {
            signed_common_request,
        }
    }
}

impl fmt::Debug for ResidentInvokeStartRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResidentInvokeStartRequest")
            .field(
                "signed_common_request_byte_len",
                &self.signed_common_request.len(),
            )
            .finish()
    }
}

/// Advance one retained Invoke by at most one owner-defined turn.
#[derive(Debug)]
pub struct ResidentInvokeAdvanceRequest {
    locator: OperationHandleLocator,
    signed_common_request: Vec<u8>,
}

impl ResidentInvokeAdvanceRequest {
    /// Bind one exact retained Invoke to a pre-signed request.
    #[must_use]
    pub fn new(locator: OperationHandleLocator, signed_common_request: Vec<u8>) -> Self {
        Self {
            locator,
            signed_common_request,
        }
    }
}

/// Start one already-selected bounded Consult.
pub struct ResidentConsultStartRequest {
    signed_common_request: Vec<u8>,
}

impl ResidentConsultStartRequest {
    /// Preserve the exact pre-signed common request.
    #[must_use]
    pub fn new(signed_common_request: Vec<u8>) -> Self {
        Self {
            signed_common_request,
        }
    }
}

impl fmt::Debug for ResidentConsultStartRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResidentConsultStartRequest")
            .field(
                "signed_common_request_byte_len",
                &self.signed_common_request.len(),
            )
            .finish()
    }
}

/// Advance one retained Consult by at most one mechanical turn.
#[derive(Debug)]
pub struct ResidentConsultAdvanceRequest {
    locator: OperationHandleLocator,
    signed_common_request: Vec<u8>,
}

impl ResidentConsultAdvanceRequest {
    /// Bind one exact retained Consult to a pre-signed request.
    #[must_use]
    pub fn new(locator: OperationHandleLocator, signed_common_request: Vec<u8>) -> Self {
        Self {
            locator,
            signed_common_request,
        }
    }
}

/// Start one already-authorized pairwise Delegate child.
#[derive(Debug)]
pub struct ResidentDelegateStartRequest {
    parent: OperationHandleLocator,
    signed_common_request: Vec<u8>,
}

impl ResidentDelegateStartRequest {
    /// Bind the exact parent to a pre-signed Delegate request.
    #[must_use]
    pub fn new(parent: OperationHandleLocator, signed_common_request: Vec<u8>) -> Self {
        Self {
            parent,
            signed_common_request,
        }
    }
}

/// Advance one retained Delegate through its already-selected dispatch owner.
#[derive(Debug)]
pub struct ResidentDelegateAdvanceRequest {
    locator: OperationHandleLocator,
    signed_common_request: Vec<u8>,
}

impl ResidentDelegateAdvanceRequest {
    /// Bind one exact retained Delegate to a pre-signed continuation request.
    #[must_use]
    pub fn new(locator: OperationHandleLocator, signed_common_request: Vec<u8>) -> Self {
        Self {
            locator,
            signed_common_request,
        }
    }
}

/// Send one fresh Cancel request for a retained operation.
///
/// The signed common request must carry `operation.cancel`, omit the separate
/// continuation target and include the exact, internally consistent operation
/// handle identified by the locator digest. The resident still checks the caller's
/// signature, recovery, current authority, replay protection and operation state.
#[derive(Debug)]
pub struct ResidentOperationCancellationRequest {
    locator: OperationHandleLocator,
    signed_common_request: Vec<u8>,
}

impl ResidentOperationCancellationRequest {
    /// Bind one exact retained operation to a pre-signed Cancel request.
    #[must_use]
    pub fn new(locator: OperationHandleLocator, signed_common_request: Vec<u8>) -> Self {
        Self {
            locator,
            signed_common_request,
        }
    }
}

/// Why a retained operation cannot advance through this call.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ResidentOperationUnsupportedReason {
    /// The retained Delegate has no already-selected provider or A2A route.
    DelegateRouteNotSelected,
}

/// Result of one bounded Invoke continuation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ResidentInvokeAdvanceOutcome {
    /// The resident returned the exact current signed records.
    Observed {
        /// Exact handle and observation returned by the resident.
        observation: ResidentOperationObservation,
    },
    /// The Invoke remains valid and needs another bounded turn.
    Pending {
        /// Exact retained Invoke coordinate.
        locator: OperationHandleLocator,
    },
}

/// Result of one bounded Consult continuation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ResidentConsultAdvanceOutcome {
    /// The resident returned the exact current signed records.
    Observed {
        /// Exact handle and observation returned by the resident.
        observation: ResidentOperationObservation,
    },
    /// The Consult remains valid and needs another bounded turn.
    Pending {
        /// Exact retained Consult coordinate.
        locator: OperationHandleLocator,
    },
}

/// Result of one bounded Delegate continuation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ResidentDelegateAdvanceOutcome {
    /// The resident returned the exact current signed records.
    Observed {
        /// Exact handle and observation returned by the resident.
        observation: ResidentOperationObservation,
    },
    /// The Delegate remains valid and needs another bounded turn.
    Pending {
        /// Exact retained Delegate coordinate.
        locator: OperationHandleLocator,
    },
    /// The compiled constitution selected no provider or A2A route.
    Unsupported {
        /// Exact retained Delegate coordinate.
        locator: OperationHandleLocator,
        /// Why no Delegate route can be used for this retained child.
        reason: ResidentOperationUnsupportedReason,
    },
}

/// Exact result of one resident Cancel request.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ResidentOperationCancellationDisposition {
    /// The operation had already reached a final state before this request.
    TerminalAlreadyEstablished,
    /// The same Cancel request was already recorded.
    AlreadyRequested,
    /// The Cancel request was durably recorded.
    RequestRetained,
    /// Final cancellation of the local operation was durably recorded.
    LocallyCancelled,
    /// A dispatched operation retained the request and received local notification.
    DispatchedRequestRetained,
    /// The local A2A response read stopped without proving remote cancellation.
    A2aResponseReadStopped,
    /// Consultation collection had already completed.
    ConsultationCollected,
    /// Recovery or uncertainty requires reconciliation.
    ReconciliationOnly,
    /// The selected dispatch needs another exact process owner.
    DispatchOwnerRequired,
    /// The consultation request and its stopped roster were durably recorded.
    ConsultationStopped,
    /// The target is not an open consultation.
    NotCollecting,
}

/// Cancel result joined to the exact target operation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ResidentOperationCancellationOutcome {
    /// Exact retained operation targeted by Cancel.
    locator: OperationHandleLocator,
    /// Resident-owned disposition without an implied terminal state.
    disposition: ResidentOperationCancellationDisposition,
}

impl ResidentOperationCancellationOutcome {
    /// Borrow the exact Cancel target.
    #[must_use]
    pub const fn locator(&self) -> &OperationHandleLocator {
        &self.locator
    }

    /// Return the result of this Cancel request.
    #[must_use]
    pub const fn disposition(&self) -> ResidentOperationCancellationDisposition {
        self.disposition
    }
}

/// Private request client implemented only by SDK-owned attachment constructors.
pub(crate) trait ResidentOperationClient: Send + Sync {
    fn request(
        &self,
        request: &ResidentOperationFrameRequest,
    ) -> Result<ResidentOperationFrameResponse, ResidentOperationClientError>;
}

/// Opaque attachment client failure retained behind an API diagnostic.
#[derive(Debug, thiserror::Error)]
#[error("resident operation client failed")]
pub(crate) struct ResidentOperationClientError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
    transmission: ResidentRequestTransmissionCertainty,
}

/// Common-request coordinates that every newly issued handle must retain.
///
/// The application may add a separately admitted federation coordinate, but it
/// cannot change any coordinate carried by the caller-signed request.
#[derive(Debug)]
struct StartedRequestBinding {
    members: Vec<(&'static str, serde_json::Value)>,
    execution_owner_fence: Option<(serde_json::Value, serde_json::Value)>,
    federation_claimed: bool,
}

/// One exact canonical request and the binding it requires from a new handle.
#[derive(Debug)]
struct ValidatedSignedRequest {
    canonical: String,
    value: serde_json::Value,
    started_binding: StartedRequestBinding,
    operation_target: Option<OperationTargetReferenceV1>,
    authority_evidence: Box<[AuthorityEvidenceV1]>,
}

impl ResidentOperationClientError {
    pub(crate) fn not_sent(message: &'static str) -> Self {
        Self {
            source: std::io::Error::other(message).into(),
            transmission: ResidentRequestTransmissionCertainty::NotSent,
        }
    }

    pub(crate) fn resident(source: ResidentAttachmentClientError) -> Self {
        Self {
            transmission: source.transmission(),
            source: Box::new(source),
        }
    }

    const fn transmission(&self) -> ResidentRequestTransmissionCertainty {
        self.transmission
    }
}

impl ResidentOperationClient for FramedResidentClient {
    fn request(
        &self,
        request: &ResidentOperationFrameRequest,
    ) -> Result<ResidentOperationFrameResponse, ResidentOperationClientError> {
        let deadline = self
            .deadline()
            .map_err(ResidentOperationClientError::resident)?;
        let frame = pale_core::canon::to_canonical_string(request).map_err(|_| {
            ResidentOperationClientError::not_sent(
                "resident operation request could not be encoded",
            )
        })?;
        self.exchange_with(frame.as_bytes(), deadline, |bytes| {
            let response: ResidentOperationFrameResponse = self.decode(bytes)?;
            validate_frame_response(request, &response).map_err(|_| {
                resident_may_have_been_sent_error("resident operation response is misbound")
            })?;
            match &response {
                ResidentOperationFrameResponse::Started { records }
                | ResidentOperationFrameResponse::Observed { records } => {
                    self.admit_records(records, deadline)?;
                }
                ResidentOperationFrameResponse::CancellationDisposition { .. }
                | ResidentOperationFrameResponse::Pending { .. }
                | ResidentOperationFrameResponse::Unsupported { .. }
                | ResidentOperationFrameResponse::Rejected { .. } => {}
            }
            Ok(response)
        })
        .map_err(ResidentOperationClientError::resident)
    }
}

impl ResidentAttachment {
    /// Report whether this attachment can route bounded operation requests.
    #[must_use]
    pub fn supports_operation_requests(&self) -> bool {
        self.operation_client.is_some()
    }
}

impl PaleApi {
    /// Start one durable root Invoke and return its handle before dispatch.
    ///
    /// # Errors
    ///
    /// Returns an API diagnostic when the signed request is invalid, the
    /// attachment cannot carry operation requests, or the resident returns an
    /// invalid or rejected start result.
    pub fn start_resident_invoke(
        &self,
        attachment: Option<&ResidentAttachment>,
        request: ResidentInvokeStartRequest,
    ) -> Result<ResidentOperationObservation, ApiError> {
        self.start_resident_invoke_with_body(attachment, request, None)
    }

    pub(crate) fn start_resident_invoke_with_body(
        &self,
        attachment: Option<&ResidentAttachment>,
        request: ResidentInvokeStartRequest,
        expected_body: Option<&pale_core::records::CommonRequestBodyV1>,
    ) -> Result<ResidentOperationObservation, ApiError> {
        let signed = validate_signed_request(&request.signed_common_request)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        if let Some(expected) = expected_body {
            let expected = serde_json::to_value(expected).map_err(|_| {
                invalid_request(
                    &self.diagnostic_identity,
                    ResidentOperationBoundaryError::SignedRequestAssociation,
                )
            })?;
            if signed.value.get("body") != Some(&expected) {
                return Err(invalid_request(
                    &self.diagnostic_identity,
                    ResidentOperationBoundaryError::SignedRequestAssociation,
                ));
            }
        }
        let frame = ResidentOperationFrameRequest::StartInvoke {
            signed_common_request: signed.canonical,
        };
        let response = call(self, attachment, &frame)?;
        project_started(self, response, "invoke", None, &signed.started_binding)
    }

    /// Advance one Invoke through the implementation owner selected by the application.
    ///
    /// # Errors
    ///
    /// Returns an API diagnostic when the locator or signed request is invalid,
    /// the attachment cannot carry the request, or the resident returns an
    /// invalid or rejected continuation result.
    pub fn advance_resident_invoke(
        &self,
        attachment: Option<&ResidentAttachment>,
        request: ResidentInvokeAdvanceRequest,
    ) -> Result<ResidentInvokeAdvanceOutcome, ApiError> {
        validate_locator(&request.locator)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        let signed = validate_signed_request(&request.signed_common_request)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        validate_signed_operation_target(&signed, &request.locator)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        let frame = ResidentOperationFrameRequest::AdvanceInvoke {
            locator: frame_locator(&request.locator),
            signed_common_request: signed.canonical,
        };
        match call(self, attachment, &frame)? {
            ResidentOperationFrameResponse::Observed { records } => {
                Ok(ResidentInvokeAdvanceOutcome::Observed {
                    observation: project_records(self, Some(&request.locator), records, "invoke")?,
                })
            }
            ResidentOperationFrameResponse::Pending { locator } => {
                exact_locator(self, &request.locator, &locator)?;
                Ok(ResidentInvokeAdvanceOutcome::Pending {
                    locator: request.locator,
                })
            }
            other => Err(invalid_response(self, other)),
        }
    }

    /// Start one already-selected Consult and return its durable handle.
    ///
    /// # Errors
    ///
    /// Returns an API diagnostic when the signed request is invalid, the
    /// attachment cannot carry operation requests, or the resident returns an
    /// invalid or rejected start result.
    pub fn start_resident_consult(
        &self,
        attachment: Option<&ResidentAttachment>,
        request: ResidentConsultStartRequest,
    ) -> Result<ResidentOperationObservation, ApiError> {
        let signed = validate_signed_request(&request.signed_common_request)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        let frame = ResidentOperationFrameRequest::StartConsult {
            signed_common_request: signed.canonical,
        };
        let response = call(self, attachment, &frame)?;
        project_started(self, response, "consult", None, &signed.started_binding)
    }

    /// Advance one Consult through its selected completion policy.
    ///
    /// # Errors
    ///
    /// Returns an API diagnostic when the locator or signed request is invalid,
    /// the attachment cannot carry the request, or the resident returns an
    /// invalid or rejected continuation result.
    pub fn advance_resident_consult(
        &self,
        attachment: Option<&ResidentAttachment>,
        request: ResidentConsultAdvanceRequest,
    ) -> Result<ResidentConsultAdvanceOutcome, ApiError> {
        validate_locator(&request.locator)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        let signed = validate_signed_request(&request.signed_common_request)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        validate_signed_operation_target(&signed, &request.locator)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        let locator = frame_locator(&request.locator);
        let frame = ResidentOperationFrameRequest::AdvanceConsult {
            locator,
            signed_common_request: signed.canonical,
        };
        match call(self, attachment, &frame)? {
            ResidentOperationFrameResponse::Observed { records } => {
                Ok(ResidentConsultAdvanceOutcome::Observed {
                    observation: project_records(self, Some(&request.locator), records, "consult")?,
                })
            }
            ResidentOperationFrameResponse::Pending { locator } => {
                exact_locator(self, &request.locator, &locator)?;
                Ok(ResidentConsultAdvanceOutcome::Pending {
                    locator: request.locator,
                })
            }
            other => Err(invalid_response(self, other)),
        }
    }

    /// Start one pairwise Delegate child and return its handle before dispatch.
    ///
    /// # Errors
    ///
    /// Returns an API diagnostic when the parent locator or signed request is
    /// invalid, the attachment cannot carry operation requests, or the resident
    /// returns an invalid or rejected start result.
    pub fn start_resident_delegate(
        &self,
        attachment: Option<&ResidentAttachment>,
        request: ResidentDelegateStartRequest,
    ) -> Result<ResidentOperationObservation, ApiError> {
        validate_locator(&request.parent)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        let signed = validate_signed_request(&request.signed_common_request)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        validate_signed_operation_target(&signed, &request.parent)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        let frame = ResidentOperationFrameRequest::StartDelegate {
            parent: frame_locator(&request.parent),
            signed_common_request: signed.canonical,
        };
        let response = call(self, attachment, &frame)?;
        project_started(
            self,
            response,
            "delegate",
            Some(&request.parent),
            &signed.started_binding,
        )
    }

    /// Advance one Delegate only through its already-selected provider or A2A route.
    ///
    /// # Errors
    ///
    /// Returns an API diagnostic when the locator or signed request is invalid,
    /// the attachment cannot carry the request, or the resident returns an
    /// invalid or rejected continuation result.
    pub fn advance_resident_delegate(
        &self,
        attachment: Option<&ResidentAttachment>,
        request: ResidentDelegateAdvanceRequest,
    ) -> Result<ResidentDelegateAdvanceOutcome, ApiError> {
        validate_locator(&request.locator)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        let signed = validate_signed_request(&request.signed_common_request)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        validate_signed_operation_target(&signed, &request.locator)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        let frame = ResidentOperationFrameRequest::AdvanceDelegate {
            locator: frame_locator(&request.locator),
            signed_common_request: signed.canonical,
        };
        match call(self, attachment, &frame)? {
            ResidentOperationFrameResponse::Observed { records } => {
                Ok(ResidentDelegateAdvanceOutcome::Observed {
                    observation: project_records(
                        self,
                        Some(&request.locator),
                        records,
                        "delegate",
                    )?,
                })
            }
            ResidentOperationFrameResponse::Pending { locator } => {
                exact_locator(self, &request.locator, &locator)?;
                Ok(ResidentDelegateAdvanceOutcome::Pending {
                    locator: request.locator,
                })
            }
            ResidentOperationFrameResponse::Unsupported { locator, reason } => {
                exact_locator(self, &request.locator, &locator)?;
                Ok(ResidentDelegateAdvanceOutcome::Unsupported {
                    locator: request.locator,
                    reason: delegate_unsupported_reason(self, reason)?,
                })
            }
            other => Err(invalid_response(self, other)),
        }
    }

    /// Request cancellation of the exact operation named by the checked request.
    /// Acknowledgement alone does not establish that the operation has ended.
    ///
    /// # Errors
    ///
    /// Returns an API diagnostic when the locator, `operation.cancel` capability,
    /// exact handle body, or signed request is invalid; the attachment cannot
    /// carry the request; or the resident returns an invalid or rejected Cancel
    /// result.
    pub fn cancel_resident_operation(
        &self,
        attachment: Option<&ResidentAttachment>,
        request: ResidentOperationCancellationRequest,
    ) -> Result<ResidentOperationCancellationOutcome, ApiError> {
        validate_locator(&request.locator)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        let signed = validate_signed_request(&request.signed_common_request)
            .and_then(|signed| {
                validate_signed_cancellation_request(&signed, &request.locator)?;
                Ok(signed)
            })
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        let frame = ResidentOperationFrameRequest::Cancel {
            locator: frame_locator(&request.locator),
            signed_common_request: signed.canonical,
        };
        match call(self, attachment, &frame)? {
            ResidentOperationFrameResponse::CancellationDisposition {
                locator,
                disposition,
            } => {
                exact_locator(self, &request.locator, &locator)?;
                Ok(ResidentOperationCancellationOutcome {
                    locator: request.locator,
                    disposition: cancellation_disposition(disposition),
                })
            }
            other => Err(invalid_response(self, other)),
        }
    }
}

fn call(
    api: &PaleApi,
    attachment: Option<&ResidentAttachment>,
    request: &ResidentOperationFrameRequest,
) -> Result<ResidentOperationFrameResponse, ApiError> {
    let client = attachment
        .and_then(|attachment| attachment.operation_client.as_ref())
        .ok_or_else(|| attachment_unavailable(&api.diagnostic_identity))?;
    match client
        .request(request)
        .map_err(|error| client_failure(&api.diagnostic_identity, error))?
    {
        ResidentOperationFrameResponse::Rejected { failure } => {
            Err(rejected(&api.diagnostic_identity, failure))
        }
        response => Ok(response),
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ResidentOperationBoundaryError {
    #[error("operation locator is malformed")]
    Locator,
    #[error("signed common request is not an exact self-consistent public record")]
    SignedRequest,
    #[error("signed common request does not identify the exact operation target once")]
    SignedRequestAssociation,
    #[error("resident operation response is incompatible with the request")]
    Response,
}

fn validate_signed_request(
    bytes: &[u8],
) -> Result<ValidatedSignedRequest, ResidentOperationBoundaryError> {
    let value = parse_canonical_record(
        bytes,
        "common-request-envelope.v1",
        "envelope_schema_sha256",
    )
    .map_err(|_| ResidentOperationBoundaryError::SignedRequest)?;
    let claimed = json_string(&value, "canonical_request_sha256")
        .and_then(|digest| CommonRequestEnvelopeDigest::parse(digest.to_owned()).ok())
        .ok_or(ResidentOperationBoundaryError::SignedRequest)?;
    let preimage = unsigned_preimage(&value, "canonical_request_sha256", "caller_signature")
        .ok_or(ResidentOperationBoundaryError::SignedRequest)?;
    let observed = CommonRequestEnvelopeDigest::hash_canonical(&preimage)
        .map_err(|_| ResidentOperationBoundaryError::SignedRequest)?;
    if claimed != observed {
        return Err(ResidentOperationBoundaryError::SignedRequest);
    }
    let authority_evidence = value
        .get("scope")
        .and_then(|scope| scope.get("authority_evidence"))
        .cloned()
        .and_then(|evidence| serde_json::from_value::<Vec<AuthorityEvidenceV1>>(evidence).ok())
        .ok_or(ResidentOperationBoundaryError::SignedRequest)?
        .into_boxed_slice();
    let operation_target = value
        .get("operation_target")
        .cloned()
        .map(serde_json::from_value::<OperationTargetReferenceV1>)
        .transpose()
        .map_err(|_| ResidentOperationBoundaryError::SignedRequest)?;
    let started_binding = started_request_binding(&value)?;
    let canonical = String::from_utf8(bytes.to_vec())
        .map_err(|_| ResidentOperationBoundaryError::SignedRequest)?;
    Ok(ValidatedSignedRequest {
        canonical,
        value,
        started_binding,
        operation_target,
        authority_evidence,
    })
}

/// Verify the fixed SDK-owned portion of one fresh Cancel request.
///
/// The application still owns caller-signature, current authority, recovery,
/// replay, disclosure, and operation-state admission. This preflight only keeps
/// a malformed, substituted, or continuation-shaped control request from
/// reaching the carrier.
fn validate_signed_cancellation_request(
    request: &ValidatedSignedRequest,
    locator: &OperationHandleLocator,
) -> Result<(), ResidentOperationBoundaryError> {
    if json_string(&request.value, "capability_id") != Some(OPERATION_CANCEL_CAPABILITY_ID)
        || request.operation_target.is_some()
    {
        return Err(ResidentOperationBoundaryError::SignedRequestAssociation);
    }
    let body = request
        .value
        .get("body")
        .and_then(serde_json::Value::as_object)
        .ok_or(ResidentOperationBoundaryError::SignedRequestAssociation)?;
    let handle_schema = pale_contracts::schema_descriptor("operation-handle.v1")
        .map_err(|_| ResidentOperationBoundaryError::SignedRequestAssociation)?;
    if body.get("schema_id").and_then(serde_json::Value::as_str) != Some("pale/operation-handle.v1")
        || body
            .get("schema_sha256")
            .and_then(serde_json::Value::as_str)
            != Some(handle_schema.schema_sha256)
    {
        return Err(ResidentOperationBoundaryError::SignedRequestAssociation);
    }
    let payload = body
        .get("payload")
        .ok_or(ResidentOperationBoundaryError::SignedRequestAssociation)?;
    let payload_canonical = pale_core::canon::to_canonical_string(payload)
        .map_err(|_| ResidentOperationBoundaryError::SignedRequestAssociation)?;
    let payload_sha256 = body
        .get("payload_sha256")
        .and_then(serde_json::Value::as_str)
        .and_then(|digest| ArtifactDigest::parse(digest.to_owned()).ok())
        .ok_or(ResidentOperationBoundaryError::SignedRequestAssociation)?;
    if ArtifactDigest::hash_bytes(payload_canonical.as_bytes()) != payload_sha256 {
        return Err(ResidentOperationBoundaryError::SignedRequestAssociation);
    }
    let handle = parse_canonical_record(
        payload_canonical.as_bytes(),
        "operation-handle.v1",
        "schema_sha256",
    )
    .map_err(|_| ResidentOperationBoundaryError::SignedRequestAssociation)?;
    if verified_operation_handle_digest(&handle) != Some(locator.operation_handle_sha256()) {
        return Err(ResidentOperationBoundaryError::SignedRequestAssociation);
    }
    Ok(())
}

fn validate_signed_operation_target(
    request: &ValidatedSignedRequest,
    locator: &OperationHandleLocator,
) -> Result<(), ResidentOperationBoundaryError> {
    let exact_target = request.operation_target.as_ref().is_some_and(|target| {
        target.operation_handle_ref.as_str() == locator.operation_handle_record_ref()
            && target.operation_handle_sha256.as_str() == locator.operation_handle_sha256()
    });
    let duplicated_as_authority = request.authority_evidence.iter().any(|candidate| {
        candidate.authority_ref().as_str() == locator.operation_handle_record_ref()
    });
    if exact_target && !duplicated_as_authority {
        Ok(())
    } else {
        Err(ResidentOperationBoundaryError::SignedRequestAssociation)
    }
}

fn started_request_binding(
    request: &serde_json::Value,
) -> Result<StartedRequestBinding, ResidentOperationBoundaryError> {
    let route = request
        .get("route")
        .and_then(serde_json::Value::as_object)
        .ok_or(ResidentOperationBoundaryError::SignedRequest)?;
    let session = request
        .get("session_admission")
        .and_then(serde_json::Value::as_object)
        .ok_or(ResidentOperationBoundaryError::SignedRequest)?;
    let required = |object: &serde_json::Map<String, serde_json::Value>, name: &str| {
        object
            .get(name)
            .cloned()
            .ok_or(ResidentOperationBoundaryError::SignedRequest)
    };
    let root = request
        .as_object()
        .ok_or(ResidentOperationBoundaryError::SignedRequest)?;
    let members = vec![
        ("request_id", required(root, "request_id")?),
        ("correlation_id", required(root, "correlation_id")?),
        ("capability_id", required(root, "capability_id")?),
        (
            "canonical_request_sha256",
            required(root, "canonical_request_sha256")?,
        ),
        ("session_grant_id", required(session, "session_grant_id")?),
        (
            "session_grant_sha256",
            required(session, "session_grant_sha256")?,
        ),
        (
            "deployment_binding_ref",
            required(session, "deployment_binding_ref")?,
        ),
        (
            "deployment_binding_sha256",
            required(session, "deployment_binding_sha256")?,
        ),
        (
            "route_binding_schema_id",
            required(route, "route_binding_schema_id")?,
        ),
        (
            "route_binding_schema_sha256",
            required(route, "route_binding_schema_sha256")?,
        ),
        ("route_binding_ref", required(route, "route_binding_ref")?),
        (
            "route_binding_sha256",
            required(route, "route_binding_sha256")?,
        ),
        ("route_generation", required(route, "route_generation")?),
        ("contract_set_ref", required(route, "contract_set_ref")?),
        (
            "contract_set_sha256",
            required(route, "contract_set_sha256")?,
        ),
    ];
    let execution_owner_fence = match root.get("execution_owner_fence") {
        None => None,
        Some(value) => {
            let fence = value
                .as_object()
                .ok_or(ResidentOperationBoundaryError::SignedRequest)?;
            Some((
                required(fence, "execution_fence_ref")?,
                required(fence, "execution_fence_sha256")?,
            ))
        }
    };
    Ok(StartedRequestBinding {
        members,
        execution_owner_fence,
        federation_claimed: root.contains_key("federation_claim"),
    })
}

fn validate_locator(
    locator: &OperationHandleLocator,
) -> Result<(), ResidentOperationBoundaryError> {
    pale_core::records::ResidentRecordRefV1::parse(
        locator.operation_handle_record_ref().to_owned(),
    )
    .map_err(|_| ResidentOperationBoundaryError::Locator)?;
    pale_core::records::OperationHandleDigest::parse(locator.operation_handle_sha256().to_owned())
        .map_err(|_| ResidentOperationBoundaryError::Locator)?;
    Ok(())
}

fn frame_locator(locator: &OperationHandleLocator) -> ResidentOperationFrameLocator {
    ResidentOperationFrameLocator::new(
        locator.operation_handle_record_ref().to_owned(),
        locator.operation_handle_sha256().to_owned(),
    )
}

pub(crate) fn validate_frame_response(
    request: &ResidentOperationFrameRequest,
    response: &ResidentOperationFrameResponse,
) -> Result<(), ResidentOperationBoundaryError> {
    if matches!(response, ResidentOperationFrameResponse::Rejected { .. }) {
        return Ok(());
    }
    match (request, response) {
        (
            ResidentOperationFrameRequest::StartInvoke {
                signed_common_request,
            },
            ResidentOperationFrameResponse::Started { records },
        ) => validate_started_records(signed_common_request, None, records, "invoke"),
        (
            ResidentOperationFrameRequest::StartConsult {
                signed_common_request,
            },
            ResidentOperationFrameResponse::Started { records },
        ) => validate_started_records(signed_common_request, None, records, "consult"),
        (
            ResidentOperationFrameRequest::StartDelegate {
                parent,
                signed_common_request,
            },
            ResidentOperationFrameResponse::Started { records },
        ) => validate_started_records(
            signed_common_request,
            Some(&observation_locator(parent)),
            records,
            "delegate",
        ),
        (
            ResidentOperationFrameRequest::AdvanceInvoke { locator, .. },
            ResidentOperationFrameResponse::Observed { records },
        ) => validate_frame_records(&observation_locator(locator), records, "invoke"),
        (
            ResidentOperationFrameRequest::AdvanceConsult { locator, .. },
            ResidentOperationFrameResponse::Observed { records },
        ) => validate_frame_records(&observation_locator(locator), records, "consult"),
        (
            ResidentOperationFrameRequest::AdvanceDelegate { locator, .. },
            ResidentOperationFrameResponse::Observed { records },
        ) => validate_frame_records(&observation_locator(locator), records, "delegate"),
        (
            ResidentOperationFrameRequest::AdvanceInvoke { locator, .. }
            | ResidentOperationFrameRequest::AdvanceConsult { locator, .. }
            | ResidentOperationFrameRequest::AdvanceDelegate { locator, .. },
            ResidentOperationFrameResponse::Pending { locator: returned },
        ) if same_frame_locator(locator, returned) => Ok(()),
        (
            ResidentOperationFrameRequest::AdvanceDelegate { locator, .. },
            ResidentOperationFrameResponse::Unsupported {
                locator: returned,
                reason: ResidentOperationFrameUnsupportedReason::DelegateRouteNotSelected,
            },
        ) if same_frame_locator(locator, returned) => Ok(()),
        (
            ResidentOperationFrameRequest::Cancel { locator, .. },
            ResidentOperationFrameResponse::CancellationDisposition {
                locator: returned, ..
            },
        ) if same_frame_locator(locator, returned) => Ok(()),
        _ => Err(ResidentOperationBoundaryError::Response),
    }
}

fn observation_locator(locator: &ResidentOperationFrameLocator) -> OperationHandleLocator {
    OperationHandleLocator::new(
        locator.operation_handle_record_ref().to_owned(),
        locator.operation_handle_sha256().to_owned(),
    )
}

fn same_frame_locator(
    expected: &ResidentOperationFrameLocator,
    actual: &ResidentOperationFrameLocator,
) -> bool {
    expected.operation_handle_record_ref() == actual.operation_handle_record_ref()
        && expected.operation_handle_sha256() == actual.operation_handle_sha256()
}

fn validate_started_records(
    signed_common_request: &str,
    parent: Option<&OperationHandleLocator>,
    records: &ResidentOperationFrameRecords,
    operation_class: &'static str,
) -> Result<(), ResidentOperationBoundaryError> {
    let expected = validate_signed_request(signed_common_request.as_bytes())?;
    let handle = validate_records(None, records, operation_class)?;
    verify_started_request_binding(&handle, &expected.started_binding)?;
    if let Some(parent) = parent {
        let parent_record = handle
            .get("parent_operation")
            .and_then(serde_json::Value::as_object)
            .ok_or(ResidentOperationBoundaryError::Response)?;
        if parent_record
            .get("operation_handle_ref")
            .and_then(serde_json::Value::as_str)
            != Some(parent.operation_handle_record_ref())
            || parent_record
                .get("operation_handle_sha256")
                .and_then(serde_json::Value::as_str)
                != Some(parent.operation_handle_sha256())
        {
            return Err(ResidentOperationBoundaryError::Response);
        }
    }
    Ok(())
}

fn validate_frame_records(
    expected: &OperationHandleLocator,
    records: &ResidentOperationFrameRecords,
    operation_class: &'static str,
) -> Result<(), ResidentOperationBoundaryError> {
    validate_records(Some(expected), records, operation_class).map(|_| ())
}

fn validate_records(
    expected: Option<&OperationHandleLocator>,
    records: &ResidentOperationFrameRecords,
    operation_class: &'static str,
) -> Result<serde_json::Value, ResidentOperationBoundaryError> {
    let response = ResidentObservationResponse {
        operation_handle_record_ref: records.operation_handle_record_ref().to_owned(),
        operation_handle_canonical_bytes: records.operation_handle_canonical().as_bytes().to_vec(),
        operation_observation_record_ref: records.operation_observation_record_ref().to_owned(),
        operation_observation_canonical_bytes: records
            .operation_observation_canonical()
            .as_bytes()
            .to_vec(),
    };
    validate_response(expected, &response).map_err(|_| ResidentOperationBoundaryError::Response)?;
    let handle = parse_canonical_record(
        &response.operation_handle_canonical_bytes,
        "operation-handle.v1",
        "schema_sha256",
    )
    .map_err(|_| ResidentOperationBoundaryError::Response)?;
    if json_string(&handle, "operation_class") != Some(operation_class) {
        return Err(ResidentOperationBoundaryError::Response);
    }
    Ok(handle)
}

fn exact_locator(
    api: &PaleApi,
    expected: &OperationHandleLocator,
    actual: &ResidentOperationFrameLocator,
) -> Result<(), ApiError> {
    if expected.operation_handle_record_ref() == actual.operation_handle_record_ref()
        && expected.operation_handle_sha256() == actual.operation_handle_sha256()
    {
        Ok(())
    } else {
        Err(invalid_response_boundary(&api.diagnostic_identity))
    }
}

fn project_started(
    api: &PaleApi,
    response: ResidentOperationFrameResponse,
    operation_class: &'static str,
    parent: Option<&OperationHandleLocator>,
    expected_binding: &StartedRequestBinding,
) -> Result<ResidentOperationObservation, ApiError> {
    let ResidentOperationFrameResponse::Started { records } = response else {
        return Err(invalid_response(api, response));
    };
    let observation = project_records(api, None, records, operation_class)?;
    let handle = parse_canonical_record(
        observation.operation_handle_canonical_bytes(),
        "operation-handle.v1",
        "schema_sha256",
    )
    .map_err(|_| invalid_response_boundary(&api.diagnostic_identity))?;
    verify_started_request_binding(&handle, expected_binding)
        .map_err(|_| invalid_response_boundary(&api.diagnostic_identity))?;
    if let Some(parent) = parent {
        let parent_record = handle
            .get("parent_operation")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| invalid_response_boundary(&api.diagnostic_identity))?;
        if parent_record
            .get("operation_handle_ref")
            .and_then(serde_json::Value::as_str)
            != Some(parent.operation_handle_record_ref())
            || parent_record
                .get("operation_handle_sha256")
                .and_then(serde_json::Value::as_str)
                != Some(parent.operation_handle_sha256())
        {
            return Err(invalid_response_boundary(&api.diagnostic_identity));
        }
    }
    Ok(observation)
}

fn verify_started_request_binding(
    handle: &serde_json::Value,
    expected: &StartedRequestBinding,
) -> Result<(), ResidentOperationBoundaryError> {
    let binding = handle
        .get("request_binding")
        .and_then(serde_json::Value::as_object)
        .ok_or(ResidentOperationBoundaryError::Response)?;
    if expected
        .members
        .iter()
        .any(|(name, value)| binding.get(*name) != Some(value))
    {
        return Err(ResidentOperationBoundaryError::Response);
    }
    match &expected.execution_owner_fence {
        Some((reference, digest))
            if binding.get("execution_owner_fence_ref") == Some(reference)
                && binding.get("execution_owner_fence_sha256") == Some(digest) => {}
        None if !binding.contains_key("execution_owner_fence_ref")
            && !binding.contains_key("execution_owner_fence_sha256") => {}
        _ => return Err(ResidentOperationBoundaryError::Response),
    }
    let federation_reference = binding.get("federation_request_admission_ref");
    let federation_digest = binding.get("federation_request_admission_sha256");
    if expected.federation_claimed {
        if federation_reference.is_none() || federation_digest.is_none() {
            return Err(ResidentOperationBoundaryError::Response);
        }
    } else if federation_reference.is_some() || federation_digest.is_some() {
        return Err(ResidentOperationBoundaryError::Response);
    }
    Ok(())
}

fn project_records(
    api: &PaleApi,
    expected: Option<&OperationHandleLocator>,
    records: ResidentOperationFrameRecords,
    operation_class: &'static str,
) -> Result<ResidentOperationObservation, ApiError> {
    let response = ResidentObservationResponse {
        operation_handle_record_ref: records.operation_handle_record_ref().to_owned(),
        operation_handle_canonical_bytes: records.operation_handle_canonical().as_bytes().to_vec(),
        operation_observation_record_ref: records.operation_observation_record_ref().to_owned(),
        operation_observation_canonical_bytes: records
            .operation_observation_canonical()
            .as_bytes()
            .to_vec(),
    };
    let observation = project_response(expected, response)
        .map_err(|_| invalid_response_boundary(&api.diagnostic_identity))?;
    let handle = parse_canonical_record(
        observation.operation_handle_canonical_bytes(),
        "operation-handle.v1",
        "schema_sha256",
    )
    .map_err(|_| invalid_response_boundary(&api.diagnostic_identity))?;
    if json_string(&handle, "operation_class") != Some(operation_class) {
        return Err(invalid_response_boundary(&api.diagnostic_identity));
    }
    Ok(observation)
}

fn delegate_unsupported_reason(
    _api: &PaleApi,
    reason: ResidentOperationFrameUnsupportedReason,
) -> Result<ResidentOperationUnsupportedReason, ApiError> {
    match reason {
        ResidentOperationFrameUnsupportedReason::DelegateRouteNotSelected => {
            Ok(ResidentOperationUnsupportedReason::DelegateRouteNotSelected)
        }
    }
}

fn cancellation_disposition(
    value: ResidentOperationFrameCancellationDisposition,
) -> ResidentOperationCancellationDisposition {
    match value {
        ResidentOperationFrameCancellationDisposition::TerminalAlreadyEstablished => {
            ResidentOperationCancellationDisposition::TerminalAlreadyEstablished
        }
        ResidentOperationFrameCancellationDisposition::AlreadyRequested => {
            ResidentOperationCancellationDisposition::AlreadyRequested
        }
        ResidentOperationFrameCancellationDisposition::RequestRetained => {
            ResidentOperationCancellationDisposition::RequestRetained
        }
        ResidentOperationFrameCancellationDisposition::LocallyCancelled => {
            ResidentOperationCancellationDisposition::LocallyCancelled
        }
        ResidentOperationFrameCancellationDisposition::DispatchedRequestRetained => {
            ResidentOperationCancellationDisposition::DispatchedRequestRetained
        }
        ResidentOperationFrameCancellationDisposition::A2aResponseReadStopped => {
            ResidentOperationCancellationDisposition::A2aResponseReadStopped
        }
        ResidentOperationFrameCancellationDisposition::ConsultationCollected => {
            ResidentOperationCancellationDisposition::ConsultationCollected
        }
        ResidentOperationFrameCancellationDisposition::ReconciliationOnly => {
            ResidentOperationCancellationDisposition::ReconciliationOnly
        }
        ResidentOperationFrameCancellationDisposition::DispatchOwnerRequired => {
            ResidentOperationCancellationDisposition::DispatchOwnerRequired
        }
        ResidentOperationFrameCancellationDisposition::ConsultationStopped => {
            ResidentOperationCancellationDisposition::ConsultationStopped
        }
        ResidentOperationFrameCancellationDisposition::NotCollecting => {
            ResidentOperationCancellationDisposition::NotCollecting
        }
    }
}

fn invalid_response(api: &PaleApi, _response: ResidentOperationFrameResponse) -> ApiError {
    invalid_response_boundary(&api.diagnostic_identity)
}

fn attachment_unavailable(identity: &pale_core::records::RecordSchemaIdentity) -> ApiError {
    failure(
        identity,
        ResidentOperationFailure {
            code: "resident_operation_unavailable",
            stage: DiagnosticStageV1::ResidentAdmission,
            disposition: DiagnosticDispositionV1::Blocked,
            retry: DiagnosticRetryV1::AfterStateChange,
            effect: DiagnosticEffectCertaintyV1::NotStarted,
            summary: "no authenticated resident operation attachment was supplied",
            help: Vec::new(),
        },
        ResidentOperationBoundaryError::Response,
    )
}

fn invalid_request(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: ResidentOperationBoundaryError,
) -> ApiError {
    failure(
        identity,
        ResidentOperationFailure {
            code: "resident_operation_request_invalid",
            stage: DiagnosticStageV1::InputAdmission,
            disposition: DiagnosticDispositionV1::InvalidInput,
            retry: DiagnosticRetryV1::AfterCorrection,
            effect: DiagnosticEffectCertaintyV1::NotStarted,
            summary: "resident operation request is malformed",
            help: vec!["Use an exact locator and a self-consistent pre-signed common request."],
        },
        error,
    )
}

fn invalid_response_boundary(identity: &pale_core::records::RecordSchemaIdentity) -> ApiError {
    failure(
        identity,
        ResidentOperationFailure {
            code: "resident_operation_response_invalid",
            stage: DiagnosticStageV1::HostObservation,
            disposition: DiagnosticDispositionV1::EffectOutcomeUnknown,
            retry: DiagnosticRetryV1::ReconcileFirst,
            effect: DiagnosticEffectCertaintyV1::OutcomeUnknown,
            summary: "resident operation response does not match the request",
            help: vec![
                "Observe and reconcile the retained operation before deciding whether to retry.",
            ],
        },
        ResidentOperationBoundaryError::Response,
    )
}

fn client_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: ResidentOperationClientError,
) -> ApiError {
    let specification = match error.transmission() {
        ResidentRequestTransmissionCertainty::NotSent => ResidentOperationFailure {
            code: "resident_operation_not_sent",
            stage: DiagnosticStageV1::HostObservation,
            disposition: DiagnosticDispositionV1::OperationalFailure,
            retry: DiagnosticRetryV1::AfterStateChange,
            effect: DiagnosticEffectCertaintyV1::NotStarted,
            summary: "resident operation request was not sent",
            help: vec!["Restore the authenticated carrier source, then issue a new request."],
        },
        ResidentRequestTransmissionCertainty::MayHaveBeenSent => ResidentOperationFailure {
            code: "resident_operation_failed",
            stage: DiagnosticStageV1::HostObservation,
            disposition: DiagnosticDispositionV1::EffectOutcomeUnknown,
            retry: DiagnosticRetryV1::ReconcileFirst,
            effect: DiagnosticEffectCertaintyV1::OutcomeUnknown,
            summary: "resident operation request did not complete",
            help: vec![
                "Observe with the returned handle when one exists. If Start returned no handle, resubmit the exact signed Start request bytes; do not mint a new request identity.",
            ],
        },
    };
    failure(identity, specification, error)
}

fn rejected(
    identity: &pale_core::records::RecordSchemaIdentity,
    failure_code: ResidentOperationFrameFailure,
) -> ApiError {
    let (code, stage, disposition, retry, effect, summary, help) = match failure_code {
        ResidentOperationFrameFailure::InvalidRequest => (
            "resident_operation_request_rejected",
            DiagnosticStageV1::ResidentAdmission,
            DiagnosticDispositionV1::InvalidInput,
            DiagnosticRetryV1::AfterCorrection,
            DiagnosticEffectCertaintyV1::NotStarted,
            "resident rejected the operation request",
            vec!["Correct the signed request before retrying."],
        ),
        ResidentOperationFrameFailure::ReplayConflict => (
            "resident_operation_replay_conflict",
            DiagnosticStageV1::ResidentAdmission,
            DiagnosticDispositionV1::InvalidInput,
            DiagnosticRetryV1::AfterCorrection,
            DiagnosticEffectCertaintyV1::NotStarted,
            "resident rejected changed bytes for a retained request identity",
            vec!["Use the exact retained request bytes or issue a new request identity."],
        ),
        ResidentOperationFrameFailure::AdmissionDenied => (
            "resident_operation_admission_denied",
            DiagnosticStageV1::ResidentAdmission,
            DiagnosticDispositionV1::Blocked,
            DiagnosticRetryV1::AfterStateChange,
            DiagnosticEffectCertaintyV1::NotStarted,
            "resident denied operation request admission",
            vec![
                "Correct the caller, session, authority, or disclosure admission before retrying.",
            ],
        ),
        ResidentOperationFrameFailure::ConfigurationUnavailable => (
            "resident_operation_configuration_unavailable",
            DiagnosticStageV1::ResidentAdmission,
            DiagnosticDispositionV1::Blocked,
            DiagnosticRetryV1::AfterStateChange,
            DiagnosticEffectCertaintyV1::NotStarted,
            "the selected resident configuration is unavailable",
            vec![
                "Restore the exact compiled policy or configuration selected for this operation before advancing it.",
            ],
        ),
        ResidentOperationFrameFailure::ResidentUnavailable => (
            "resident_operation_outcome_unknown",
            DiagnosticStageV1::ResidentRecovery,
            DiagnosticDispositionV1::EffectOutcomeUnknown,
            DiagnosticRetryV1::ReconcileFirst,
            DiagnosticEffectCertaintyV1::OutcomeUnknown,
            "resident could not establish the bounded operation result",
            vec![
                "Observe with the returned handle when one exists. If Start returned no handle, resubmit the exact signed Start request bytes; do not mint a new request identity.",
            ],
        ),
    };
    failure(
        identity,
        ResidentOperationFailure {
            code,
            stage,
            disposition,
            retry,
            effect,
            summary,
            help,
        },
        ResidentOperationBoundaryError::Response,
    )
}

#[cfg(test)]
#[path = "../tests/internal/resident_operation.rs"]
mod tests;

struct ResidentOperationFailure {
    code: &'static str,
    stage: DiagnosticStageV1,
    disposition: DiagnosticDispositionV1,
    retry: DiagnosticRetryV1,
    effect: DiagnosticEffectCertaintyV1,
    summary: &'static str,
    help: Vec<&'static str>,
}

fn failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    failure: ResidentOperationFailure,
    source: impl Error + Send + Sync + 'static,
) -> ApiError {
    let ResidentOperationFailure {
        code,
        stage,
        disposition,
        retry,
        effect,
        summary,
        help,
    } = failure;
    ApiError::new_with_opaque_source(
        identity,
        source,
        FailureSpecification {
            code,
            stage,
            disposition,
            summary,
            subject: None,
            schema_coordinate: None,
            location: None,
            retry,
            effect,
            request: None,
            notes: Vec::new(),
            help,
            facts: Vec::new(),
        },
    )
}
