//! Read a recorded operation through the caller's resident connection.

use std::fmt;

use pale_core::records::{
    ArtifactDigest, CommonRequestEnvelopeDigest, DiagnosticDispositionV1,
    DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1, OperationHandleDigest,
    OperationObservationDigest, ResidentRecordRefV1,
};
use pale_ports::{
    ResidentObserveFrameFailure, ResidentObserveFrameRequest, ResidentObserveFrameResponse,
};
use serde::Serialize;

use crate::error::{ApiError, FailureSpecification};
use crate::PaleApi;

use crate::resident_attachment::{
    resident_may_have_been_sent_error, resident_not_sent_error, response_admission_error,
    FramedResidentClient, ResidentAttachment, ResidentAttachmentClientError,
    ResidentRequestTransmissionCertainty, RESIDENT_RECORD_BYTE_LIMIT,
};

/// Locate one stored operation handle and identify its contents.
///
/// The record reference names its storage location; the digest identifies the
/// handle's unsigned canonical bytes. Verifying its signature also requires
/// checking the signer's authority for that context. Knowing the location and
/// digest does not grant permission to read the operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct OperationHandleLocator {
    /// Immutable storage reference for the admitted operation handle.
    operation_handle_record_ref: String,
    /// SHA-256 of the unsigned canonical operation-handle preimage.
    operation_handle_sha256: String,
}

impl OperationHandleLocator {
    /// Combine the storage reference and digest for the resident to check.
    #[must_use]
    pub fn new(operation_handle_record_ref: String, operation_handle_sha256: String) -> Self {
        Self {
            operation_handle_record_ref,
            operation_handle_sha256,
        }
    }

    /// Borrow the immutable operation-handle record reference.
    #[must_use]
    pub fn operation_handle_record_ref(&self) -> &str {
        &self.operation_handle_record_ref
    }

    /// Borrow the canonical operation-handle digest.
    #[must_use]
    pub fn operation_handle_sha256(&self) -> &str {
        &self.operation_handle_sha256
    }
}

/// Request to read one operation already recorded by the resident.
///
/// The caller supplies the common request with its authority's signature already
/// attached. The SDK checks canonical formatting, the public contract and the
/// digest carried by the request. Signature verification still needs a trusted
/// verification key. This request contains no permission decision, path or secret.
pub struct ResidentOperationObservationRequest {
    locator: OperationHandleLocator,
    signed_common_request: Vec<u8>,
}

impl ResidentOperationObservationRequest {
    /// Assemble one owned resident observation request.
    #[must_use]
    pub fn new(locator: OperationHandleLocator, signed_common_request: Vec<u8>) -> Self {
        Self {
            locator,
            signed_common_request,
        }
    }

    /// Borrow the exact durable operation locator.
    #[must_use]
    pub const fn locator(&self) -> &OperationHandleLocator {
        &self.locator
    }

    /// Borrow the exact pre-signed common-request bytes.
    #[must_use]
    pub fn signed_common_request(&self) -> &[u8] {
        &self.signed_common_request
    }
}

impl fmt::Debug for ResidentOperationObservationRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResidentOperationObservationRequest")
            .field("locator", &self.locator)
            .field(
                "signed_common_request_byte_len",
                &self.signed_common_request.len(),
            )
            .finish()
    }
}

/// An observation request that passed the SDK's checks before connection.
///
/// Those checks cover canonical formatting, the public contract, embedded digests,
/// the `operation.observe` capability and agreement with the operation locator.
/// Signature verification, resident selection, peer authentication, session and
/// operation recovery, and permission to disclose data remain separate checks.
/// This value grants no retry or permission to perform work.
///
/// Interfaces can reject malformed input before finding a resident connection.
/// The private state keeps a checked request from being constructed as if it were
/// proof of resident authority.
pub struct ResidentOperationObservationPreflight {
    request: ResidentOperationObservationRequest,
}

impl fmt::Debug for ResidentOperationObservationPreflight {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResidentOperationObservationPreflight")
            .field("locator", &self.request.locator)
            .field(
                "signed_common_request_byte_len",
                &self.request.signed_common_request.len(),
            )
            .finish()
    }
}

/// Signed records returned after a resident request passes its response checks.
///
/// Storage references and canonical bytes stay separate: a digest identifies
/// content, not its storage location. Observe, durable Start and advances that
/// return current records all use this type. These records do not fetch the
/// evidence they reference or prove that an external action happened or finished.
#[derive(Clone, Eq, PartialEq)]
pub struct ResidentOperationObservation {
    operation_handle_record_ref: String,
    operation_handle_canonical_bytes: Vec<u8>,
    operation_observation_record_ref: String,
    operation_observation_canonical_bytes: Vec<u8>,
}

impl fmt::Debug for ResidentOperationObservation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResidentOperationObservation")
            .field(
                "operation_handle_record_ref",
                &self.operation_handle_record_ref,
            )
            .field(
                "operation_handle_canonical_byte_len",
                &self.operation_handle_canonical_bytes.len(),
            )
            .field(
                "operation_observation_record_ref",
                &self.operation_observation_record_ref,
            )
            .field(
                "operation_observation_canonical_byte_len",
                &self.operation_observation_canonical_bytes.len(),
            )
            .finish()
    }
}

impl ResidentOperationObservation {
    /// Borrow the immutable operation-handle record reference.
    #[must_use]
    pub fn operation_handle_record_ref(&self) -> &str {
        &self.operation_handle_record_ref
    }

    /// Borrow the exact admitted canonical `OperationHandleV1` JSON bytes.
    #[must_use]
    pub fn operation_handle_canonical_bytes(&self) -> &[u8] {
        &self.operation_handle_canonical_bytes
    }

    /// Borrow the exact admitted `OperationHandleV1` record as canonical JSON text.
    #[must_use]
    pub fn operation_handle_canonical_json(&self) -> &str {
        admitted_canonical_json(&self.operation_handle_canonical_bytes)
    }

    /// Borrow the immutable operation-observation record reference.
    #[must_use]
    pub fn operation_observation_record_ref(&self) -> &str {
        &self.operation_observation_record_ref
    }

    /// Borrow the exact admitted canonical `OperationObservationV1` JSON bytes.
    #[must_use]
    pub fn operation_observation_canonical_bytes(&self) -> &[u8] {
        &self.operation_observation_canonical_bytes
    }

    /// Borrow the exact admitted `OperationObservationV1` record as canonical JSON text.
    #[must_use]
    pub fn operation_observation_canonical_json(&self) -> &str {
        admitted_canonical_json(&self.operation_observation_canonical_bytes)
    }
}

/// Recover the UTF-8 form already proved by strict canonical JSON admission.
fn admitted_canonical_json(bytes: &[u8]) -> &str {
    match std::str::from_utf8(bytes) {
        Ok(json) => json,
        Err(_) => unreachable!("admitted canonical JSON is always UTF-8"),
    }
}

/// Readable wire projection over the byte-exact admitted records.
#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
struct ResidentOperationObservationProjection<'a> {
    /// Immutable storage reference for the admitted operation handle.
    operation_handle_record_ref: &'a str,
    /// Exact admitted `OperationHandleV1` record as canonical JSON text.
    operation_handle_canonical_json: &'a str,
    /// Immutable storage reference for the admitted operation observation.
    operation_observation_record_ref: &'a str,
    /// Exact admitted `OperationObservationV1` record as canonical JSON text.
    operation_observation_canonical_json: &'a str,
}

impl Serialize for ResidentOperationObservation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ResidentOperationObservationProjection {
            operation_handle_record_ref: &self.operation_handle_record_ref,
            operation_handle_canonical_json: self.operation_handle_canonical_json(),
            operation_observation_record_ref: &self.operation_observation_record_ref,
            operation_observation_canonical_json: self.operation_observation_canonical_json(),
        }
        .serialize(serializer)
    }
}

#[cfg(feature = "wire-schema")]
impl schemars::JsonSchema for ResidentOperationObservation {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ResidentOperationObservation".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::ResidentOperationObservation").into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        <ResidentOperationObservationProjection<'static> as schemars::JsonSchema>::json_schema(
            generator,
        )
    }
}

/// Private carrier result before SDK projection.
pub(crate) struct ResidentObservationResponse {
    pub(crate) operation_handle_record_ref: String,
    pub(crate) operation_handle_canonical_bytes: Vec<u8>,
    pub(crate) operation_observation_record_ref: String,
    pub(crate) operation_observation_canonical_bytes: Vec<u8>,
}

/// Private client implemented only by SDK-owned attachment constructors.
pub(crate) trait ResidentObservationClient: Send + Sync {
    fn observe(
        &self,
        request: &ResidentOperationObservationRequest,
    ) -> Result<ResidentObservationResponse, ResidentAttachmentClientError>;
}

impl ResidentObservationClient for FramedResidentClient {
    fn observe(
        &self,
        request: &ResidentOperationObservationRequest,
    ) -> Result<ResidentObservationResponse, ResidentAttachmentClientError> {
        let request_json = std::str::from_utf8(request.signed_common_request())
            .map_err(|_| resident_not_sent_error("signed common request is not UTF-8"))?;
        let frame = ResidentObserveFrameRequest::new(
            request.locator().operation_handle_record_ref().to_owned(),
            request.locator().operation_handle_sha256().to_owned(),
            request_json.to_owned(),
        );
        let deadline = self.deadline()?;
        let frame = pale_core::canon::to_canonical_string(&frame).map_err(|_| {
            resident_not_sent_error("resident Observe request could not be encoded")
        })?;
        if frame.len() > RESIDENT_RECORD_BYTE_LIMIT.saturating_mul(4) {
            return Err(resident_not_sent_error(
                "resident Observe request frame exceeds its bound",
            ));
        }

        let response: ResidentObserveFrameResponse =
            self.exchange_with(frame.as_bytes(), deadline, |bytes| {
                let response: ResidentObserveFrameResponse = self.decode(bytes)?;
                if let ResidentObserveFrameResponse::Observed {
                    operation_handle_record_ref,
                    operation_handle_canonical,
                    operation_observation_record_ref,
                    operation_observation_canonical,
                } = &response
                {
                    let projected = ResidentObservationResponse {
                        operation_handle_record_ref: operation_handle_record_ref.clone(),
                        operation_handle_canonical_bytes: operation_handle_canonical
                            .as_bytes()
                            .to_vec(),
                        operation_observation_record_ref: operation_observation_record_ref.clone(),
                        operation_observation_canonical_bytes: operation_observation_canonical
                            .as_bytes()
                            .to_vec(),
                    };
                    validate_response(Some(request.locator()), &projected).map_err(|_| {
                        resident_may_have_been_sent_error("resident Observe response is misbound")
                    })?;
                    self.response_admission()?
                        .admit(
                            operation_handle_record_ref,
                            operation_handle_canonical.as_bytes(),
                            operation_observation_record_ref,
                            operation_observation_canonical.as_bytes(),
                            deadline,
                        )
                        .map_err(response_admission_error)?;
                }
                Ok(response)
            })?;
        match response {
            ResidentObserveFrameResponse::Observed {
                operation_handle_record_ref,
                operation_handle_canonical,
                operation_observation_record_ref,
                operation_observation_canonical,
            } => Ok(ResidentObservationResponse {
                operation_handle_record_ref,
                operation_handle_canonical_bytes: operation_handle_canonical.into_bytes(),
                operation_observation_record_ref,
                operation_observation_canonical_bytes: operation_observation_canonical.into_bytes(),
            }),
            ResidentObserveFrameResponse::Rejected { failure } => {
                Err(rejected_client_error(failure))
            }
        }
    }
}

fn rejected_client_error(failure: ResidentObserveFrameFailure) -> ResidentAttachmentClientError {
    let message = match failure {
        ResidentObserveFrameFailure::InvalidRequest => "resident rejected the Observe request",
        ResidentObserveFrameFailure::ReplayConflict => {
            "resident rejected changed bytes for an Observe request identity"
        }
        ResidentObserveFrameFailure::AdmissionDenied => "resident denied Observe request admission",
        ResidentObserveFrameFailure::ResidentUnavailable => {
            "resident could not complete Observe recovery"
        }
    };
    resident_may_have_been_sent_error(message)
}

impl PaleApi {
    /// Check an observation request's structure before connecting to the resident.
    ///
    /// The check enforces input limits and rejects malformed or substituted bytes.
    /// Success establishes neither authentication nor permission, a usable connection
    /// nor the existence of the operation. The result contains only the checked
    /// request, with no client, store, callback, signer, credential, continuation
    /// or permission to perform work.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the locator or signed request is malformed,
    /// not canonical, incompatible with its contract, inconsistent with itself,
    /// or associated with another operation. The check contacts no connection,
    /// storage or authority service.
    pub fn preflight_resident_operation_observation(
        &self,
        request: ResidentOperationObservationRequest,
    ) -> Result<ResidentOperationObservationPreflight, ApiError> {
        validate_request(&request)
            .map_err(|error| invalid_request(&self.diagnostic_identity, error))?;
        Ok(ResidentOperationObservationPreflight { request })
    }

    /// Read one recorded operation through the supplied resident connection.
    ///
    /// The caller's attachment provides the connection and checks response authority.
    /// The SDK checks the request's canonical formatting, public contract and embedded
    /// digest. Returned records must pass the same checks and match the exact requested
    /// locator. Peer authentication and signature verification belong to the caller's
    /// connection and response checker.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the attachment is absent, the locator or signed
    /// request is malformed, the client fails, or its response substitutes a
    /// different handle or non-canonical record. The call does not change the operation,
    /// retry the request, cancel work or renew authority.
    pub fn observe_resident_operation(
        &self,
        attachment: Option<&ResidentAttachment>,
        request: ResidentOperationObservationRequest,
    ) -> Result<ResidentOperationObservation, ApiError> {
        let client = attachment
            .and_then(|attachment| attachment.client.as_ref())
            .ok_or_else(|| attachment_unavailable(&self.diagnostic_identity))?;
        let preflight = self.preflight_resident_operation_observation(request)?;
        let response = client
            .observe(&preflight.request)
            .map_err(|error| client_failure(&self.diagnostic_identity, error))?;
        project_response(Some(preflight.request.locator()), response)
            .map_err(|error| invalid_response(&self.diagnostic_identity, error))
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ResidentObservationBoundaryError {
    #[error("operation record reference is invalid")]
    OperationRecordReference,
    #[error("operation-handle digest is invalid")]
    OperationHandleDigest,
    #[error("signed common request is absent or exceeds its byte bound")]
    SignedRequestBounds,
    #[error("signed common request is not exact canonical JSON")]
    SignedRequestCanonicalRecord,
    #[error("signed common request does not satisfy its public contract")]
    SignedRequestContract,
    #[error("signed common request does not reproduce its claimed digest")]
    SignedRequestDigest,
    #[error("signed common request does not identify the exact observation target")]
    SignedRequestAssociation,
    #[error("resident response record reference is invalid")]
    ResponseRecordReference,
    #[error("resident response record is not exact canonical JSON")]
    ResponseCanonicalRecord,
    #[error("resident response record does not satisfy its public contract")]
    ResponseContract,
    #[error("resident response record does not reproduce its claimed digest")]
    ResponseDigest,
    #[error("resident response does not identify the requested operation")]
    ResponseAssociation,
}

fn validate_request(
    request: &ResidentOperationObservationRequest,
) -> Result<(), ResidentObservationBoundaryError> {
    ResidentRecordRefV1::parse(request.locator.operation_handle_record_ref.clone())
        .map_err(|_| ResidentObservationBoundaryError::OperationRecordReference)?;
    OperationHandleDigest::parse(request.locator.operation_handle_sha256.clone())
        .map_err(|_| ResidentObservationBoundaryError::OperationHandleDigest)?;
    if request.signed_common_request.is_empty()
        || request.signed_common_request.len() > RESIDENT_RECORD_BYTE_LIMIT
    {
        return Err(ResidentObservationBoundaryError::SignedRequestBounds);
    }
    let common_request = parse_canonical_record(
        &request.signed_common_request,
        "common-request-envelope.v1",
        "envelope_schema_sha256",
    )
    .map_err(|error| match error {
        CanonicalRecordError::Canonical => {
            ResidentObservationBoundaryError::SignedRequestCanonicalRecord
        }
        CanonicalRecordError::Contract => ResidentObservationBoundaryError::SignedRequestContract,
    })?;
    verify_common_request_digest(&common_request)?;
    verify_common_request_association(&common_request, request.locator())?;
    Ok(())
}

pub(crate) fn project_response(
    expected_locator: Option<&OperationHandleLocator>,
    response: ResidentObservationResponse,
) -> Result<ResidentOperationObservation, ResidentObservationBoundaryError> {
    validate_response(expected_locator, &response)?;

    Ok(ResidentOperationObservation {
        operation_handle_record_ref: response.operation_handle_record_ref,
        operation_handle_canonical_bytes: response.operation_handle_canonical_bytes,
        operation_observation_record_ref: response.operation_observation_record_ref,
        operation_observation_canonical_bytes: response.operation_observation_canonical_bytes,
    })
}

pub(crate) fn validate_response(
    expected_locator: Option<&OperationHandleLocator>,
    response: &ResidentObservationResponse,
) -> Result<(), ResidentObservationBoundaryError> {
    ResidentRecordRefV1::parse(response.operation_handle_record_ref.clone())
        .map_err(|_| ResidentObservationBoundaryError::ResponseRecordReference)?;
    ResidentRecordRefV1::parse(response.operation_observation_record_ref.clone())
        .map_err(|_| ResidentObservationBoundaryError::ResponseRecordReference)?;

    let handle = parse_canonical_record(
        &response.operation_handle_canonical_bytes,
        "operation-handle.v1",
        "schema_sha256",
    )
    .map_err(response_record_error)?;
    let observation = parse_canonical_record(
        &response.operation_observation_canonical_bytes,
        "operation-observation.v1",
        "schema_sha256",
    )
    .map_err(response_record_error)?;
    verify_operation_handle_digest(&handle)?;
    verify_operation_observation_digest(&observation)?;
    if response.operation_observation_record_ref == response.operation_handle_record_ref
        || json_string(&handle, "canonical_operation_handle_sha256")
            != json_string(&observation, "operation_handle_sha256")
        || json_string(&observation, "operation_handle_ref")
            != Some(response.operation_handle_record_ref.as_str())
        || json_string(&observation, "operation_handle_sha256")
            != json_string(&handle, "canonical_operation_handle_sha256")
        || json_string(&observation, "operation_handle_id")
            != json_string(&handle, "operation_handle_id")
    {
        return Err(ResidentObservationBoundaryError::ResponseAssociation);
    }
    if expected_locator.is_some_and(|locator| {
        response.operation_handle_record_ref != locator.operation_handle_record_ref
            || json_string(&handle, "canonical_operation_handle_sha256")
                != Some(locator.operation_handle_sha256())
    }) {
        return Err(ResidentObservationBoundaryError::ResponseAssociation);
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CanonicalRecordError {
    Canonical,
    Contract,
}

pub(crate) fn parse_canonical_record(
    bytes: &[u8],
    schema_id: &'static str,
    schema_digest_member: &'static str,
) -> Result<serde_json::Value, CanonicalRecordError> {
    if bytes.is_empty() || bytes.len() > RESIDENT_RECORD_BYTE_LIMIT {
        return Err(CanonicalRecordError::Canonical);
    }
    let value =
        pale_core::canon::parse_strict(bytes).map_err(|_| CanonicalRecordError::Canonical)?;
    let canonical = pale_core::canon::to_canonical_string(&value)
        .map_err(|_| CanonicalRecordError::Canonical)?;
    if canonical.as_bytes() != bytes {
        return Err(CanonicalRecordError::Canonical);
    }
    pale_contracts::validate_value(&value, schema_id)
        .map_err(|_| CanonicalRecordError::Contract)?;
    let expected_schema_sha256 = pale_contracts::schema_descriptor(schema_id)
        .map_err(|_| CanonicalRecordError::Contract)?
        .schema_sha256;
    if json_string(&value, schema_digest_member) != Some(expected_schema_sha256) {
        return Err(CanonicalRecordError::Contract);
    }
    Ok(value)
}

fn response_record_error(error: CanonicalRecordError) -> ResidentObservationBoundaryError {
    match error {
        CanonicalRecordError::Canonical => {
            ResidentObservationBoundaryError::ResponseCanonicalRecord
        }
        CanonicalRecordError::Contract => ResidentObservationBoundaryError::ResponseContract,
    }
}

pub(crate) fn unsigned_preimage(
    value: &serde_json::Value,
    digest_member: &'static str,
    signature_member: &'static str,
) -> Option<serde_json::Value> {
    let mut preimage = value.as_object()?.clone();
    preimage.remove(digest_member)?;
    preimage.remove(signature_member)?;
    Some(serde_json::Value::Object(preimage))
}

pub(crate) fn verify_common_request_digest(
    value: &serde_json::Value,
) -> Result<(), ResidentObservationBoundaryError> {
    let claimed = json_string(value, "canonical_request_sha256")
        .and_then(|digest| CommonRequestEnvelopeDigest::parse(digest.to_owned()).ok())
        .ok_or(ResidentObservationBoundaryError::SignedRequestDigest)?;
    let preimage = unsigned_preimage(value, "canonical_request_sha256", "caller_signature")
        .ok_or(ResidentObservationBoundaryError::SignedRequestDigest)?;
    let observed = CommonRequestEnvelopeDigest::hash_canonical(&preimage)
        .map_err(|_| ResidentObservationBoundaryError::SignedRequestDigest)?;
    if claimed == observed {
        Ok(())
    } else {
        Err(ResidentObservationBoundaryError::SignedRequestDigest)
    }
}

fn verify_common_request_association(
    value: &serde_json::Value,
    locator: &OperationHandleLocator,
) -> Result<(), ResidentObservationBoundaryError> {
    if json_string(value, "capability_id") != Some("operation.observe") {
        return Err(ResidentObservationBoundaryError::SignedRequestAssociation);
    }
    let body = value
        .get("body")
        .and_then(serde_json::Value::as_object)
        .ok_or(ResidentObservationBoundaryError::SignedRequestAssociation)?;
    let handle_schema = pale_contracts::schema_descriptor("operation-handle.v1")
        .map_err(|_| ResidentObservationBoundaryError::SignedRequestAssociation)?;
    if body.get("schema_id").and_then(serde_json::Value::as_str) != Some("pale/operation-handle.v1")
        || body
            .get("schema_sha256")
            .and_then(serde_json::Value::as_str)
            != Some(handle_schema.schema_sha256)
    {
        return Err(ResidentObservationBoundaryError::SignedRequestAssociation);
    }
    let payload = body
        .get("payload")
        .ok_or(ResidentObservationBoundaryError::SignedRequestAssociation)?;
    let payload_bytes = pale_core::canon::to_canonical_string(payload)
        .map_err(|_| ResidentObservationBoundaryError::SignedRequestAssociation)?;
    let claimed_payload_sha256 = body
        .get("payload_sha256")
        .and_then(serde_json::Value::as_str)
        .and_then(|digest| ArtifactDigest::parse(digest.to_owned()).ok())
        .ok_or(ResidentObservationBoundaryError::SignedRequestAssociation)?;
    if ArtifactDigest::hash_bytes(payload_bytes.as_bytes()) != claimed_payload_sha256 {
        return Err(ResidentObservationBoundaryError::SignedRequestAssociation);
    }
    let handle = parse_canonical_record(
        payload_bytes.as_bytes(),
        "operation-handle.v1",
        "schema_sha256",
    )
    .map_err(|_| ResidentObservationBoundaryError::SignedRequestAssociation)?;
    verify_operation_handle_digest(&handle)
        .map_err(|_| ResidentObservationBoundaryError::SignedRequestAssociation)?;
    if json_string(&handle, "canonical_operation_handle_sha256")
        != Some(locator.operation_handle_sha256())
    {
        return Err(ResidentObservationBoundaryError::SignedRequestAssociation);
    }
    Ok(())
}

fn verify_operation_handle_digest(
    value: &serde_json::Value,
) -> Result<(), ResidentObservationBoundaryError> {
    verified_operation_handle_digest(value)
        .ok_or(ResidentObservationBoundaryError::ResponseDigest)
        .map(|_| ())
}

/// Return an operation handle's exact self-carried digest after reproducing it.
pub(crate) fn verified_operation_handle_digest(value: &serde_json::Value) -> Option<&str> {
    let claimed_text = json_string(value, "canonical_operation_handle_sha256")?;
    let claimed = OperationHandleDigest::parse(claimed_text.to_owned()).ok()?;
    let preimage = unsigned_preimage(value, "canonical_operation_handle_sha256", "signature")?;
    let observed = OperationHandleDigest::hash_canonical(&preimage).ok()?;
    if claimed == observed {
        Some(claimed_text)
    } else {
        None
    }
}

fn verify_operation_observation_digest(
    value: &serde_json::Value,
) -> Result<(), ResidentObservationBoundaryError> {
    let claimed = json_string(value, "canonical_operation_observation_sha256")
        .and_then(|digest| OperationObservationDigest::parse(digest.to_owned()).ok())
        .ok_or(ResidentObservationBoundaryError::ResponseDigest)?;
    let preimage = unsigned_preimage(value, "canonical_operation_observation_sha256", "signature")
        .ok_or(ResidentObservationBoundaryError::ResponseDigest)?;
    let observed = OperationObservationDigest::hash_canonical(&preimage)
        .map_err(|_| ResidentObservationBoundaryError::ResponseDigest)?;
    if claimed == observed {
        Ok(())
    } else {
        Err(ResidentObservationBoundaryError::ResponseDigest)
    }
}

pub(crate) fn json_string<'a>(value: &'a serde_json::Value, member: &str) -> Option<&'a str> {
    value.get(member).and_then(serde_json::Value::as_str)
}

#[derive(Debug, thiserror::Error)]
#[error("no resident attachment was supplied")]
struct ResidentAttachmentUnavailable;

fn attachment_unavailable(identity: &pale_core::records::RecordSchemaIdentity) -> ApiError {
    ApiError::new_with_opaque_source(
        identity,
        ResidentAttachmentUnavailable,
        FailureSpecification {
            code: "resident_observation_unavailable",
            stage: DiagnosticStageV1::ResidentAdmission,
            disposition: DiagnosticDispositionV1::Blocked,
            summary: "no resident attachment was supplied",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::AfterStateChange,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: vec![
                "No resident lookup or disclosure decision started.",
                "A trusted native composition root must provision the carrier and response-admission ports.",
            ],
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}

fn invalid_request(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: ResidentObservationBoundaryError,
) -> ApiError {
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: "resident_observation_request_invalid",
            stage: DiagnosticStageV1::InputAdmission,
            disposition: DiagnosticDispositionV1::InvalidInput,
            summary: "resident Observe request is malformed",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::AfterCorrection,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: vec!["Use an exact handle reference, digest, and pre-signed common request."],
            facts: Vec::new(),
        },
    )
}

fn client_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: ResidentAttachmentClientError,
) -> ApiError {
    let (code, summary, retry, help) = match error.transmission() {
        ResidentRequestTransmissionCertainty::NotSent => (
            "resident_observation_not_sent",
            "resident Observe request was not sent",
            DiagnosticRetryV1::AfterStateChange,
            vec!["Restore the caller-provisioned carrier source, then issue a new request."],
        ),
        ResidentRequestTransmissionCertainty::MayHaveBeenSent => (
            "resident_observation_failed",
            "resident Observe did not complete",
            DiagnosticRetryV1::NotIndicated,
            vec!["Inspect the resident attachment and reconcile before retrying."],
        ),
    };
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code,
            stage: DiagnosticStageV1::HostObservation,
            disposition: DiagnosticDispositionV1::OperationalFailure,
            summary,
            subject: None,
            schema_coordinate: None,
            location: None,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help,
            facts: Vec::new(),
        },
    )
}

fn invalid_response(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: ResidentObservationBoundaryError,
) -> ApiError {
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: "resident_observation_response_invalid",
            stage: DiagnosticStageV1::HostObservation,
            disposition: DiagnosticDispositionV1::InternalFailure,
            summary: "resident Observe response is not an exact canonical projection",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::NotIndicated,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: vec!["The SDK did not expose the rejected resident response."],
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}

#[cfg(test)]
#[path = "../tests/internal/resident_observation.rs"]
mod tests;
