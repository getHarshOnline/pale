//! Bind a caller-signed operation start to the exact native event delivered later.

use pale_application::resident::{ResidentHookIngressAdmissionError, ResidentNativeToolStart};
use std::time::Instant;

use pale_core::records::{
    CanonicalSignatureV1, CommonRequestBodyV1, CommonRequestEnvelopePreimageV1,
    CommonRequestEnvelopeWireV1, DiagnosticDispositionV1, DiagnosticEffectCertaintyV1,
    DiagnosticRetryV1, DiagnosticStageV1, InvalidResidentRecordV1,
};

use super::{ingress_failure, ResidentHookAdmission};
use crate::{
    ApiError, PaleApi, ResidentAttachment, ResidentInvokeStartRequest, ResidentOperationObservation,
};

/// Exact request claim presented to an independently authorized signing owner.
/// The canonical preimage includes the caller, key coordinate and authority
/// binding that owner must check against its own configuration.
pub struct ResidentNativeRequestSigningInput<'a> {
    preimage: &'a [u8],
    purpose: &'a [u8],
    digest: &'a str,
}

impl ResidentNativeRequestSigningInput<'_> {
    /// Complete canonical unsigned request. These are claims, not authority.
    #[must_use]
    pub const fn canonical_preimage(&self) -> &[u8] {
        self.preimage
    }

    /// Fixed request domain, including its terminating NUL.
    /// Sign this followed by the decoded 32-byte request digest.
    #[must_use]
    pub const fn purpose(&self) -> &[u8] {
        self.purpose
    }

    /// Lowercase hexadecimal SHA-256 of the exact canonical preimage.
    #[must_use]
    pub const fn digest(&self) -> &str {
        self.digest
    }
}

impl std::fmt::Debug for ResidentNativeRequestSigningInput<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ResidentNativeRequestSigningInput(<private request>)")
    }
}

/// An independently selected owner refused or could not complete request signing.
#[derive(Debug, thiserror::Error)]
#[error("native request signing is unavailable or refused")]
pub struct ResidentNativeRequestSigningFailure;

/// Request-signing authority, separate from connection authentication.
/// Implementations must authorize the claimed caller, key and purpose against
/// independently provisioned evidence before using a key. The returned signature
/// remains an untrusted claim until the resident verifies it and current authority.
pub trait ResidentNativeRequestSigner {
    /// Sign the fixed domain and digest once within the supplied deadline.
    /// Return the canonical unpadded base64url Ed25519 signature.
    ///
    /// # Errors
    /// Refuse an unauthorized caller, key or purpose, expired deadline, or any
    /// unavailable credential. Do not fall back to a transport-signing key.
    fn sign(
        &mut self,
        input: ResidentNativeRequestSigningInput<'_>,
        deadline: Instant,
    ) -> Result<String, ResidentNativeRequestSigningFailure>;
}

#[derive(Debug, thiserror::Error)]
enum NativeSigningError {
    #[error("native request claim is invalid or does not match the prepared event")]
    Request,
    #[error("native request signer refused or exceeded its deadline")]
    Signing,
}

/// Original native event and the body its independently provisioned caller must sign.
/// Preparation selects no caller, grant, session, capability or operation.
pub struct PreparedResidentNativeTool {
    admission: ResidentHookAdmission,
    body: CommonRequestBodyV1,
    body_json: Vec<u8>,
}

impl PreparedResidentNativeTool {
    /// Canonical JSON for the common request's `body` field, including digests.
    /// The host's request signer must preserve this whole value while supplying
    /// its independently admitted session, grant and operation coordinates.
    #[must_use]
    pub fn request_body_json(&self) -> &[u8] {
        &self.body_json
    }
}

impl std::fmt::Debug for PreparedResidentNativeTool {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PreparedResidentNativeTool(<private event>)")
    }
}

/// A verified durable start response and the original event still awaiting a decision.
/// The issued operation does not permit native execution. The host must keep the
/// tool blocked until the resident's current, enforced hook response allows it.
pub struct StartedResidentNativeTool {
    observation: ResidentOperationObservation,
    admission: ResidentHookAdmission,
}

impl StartedResidentNativeTool {
    /// Retained issued handle and initial observation for later reconciliation.
    #[must_use]
    pub const fn observation(&self) -> &ResidentOperationObservation {
        &self.observation
    }

    /// Move the exact original event to the hook boundary after durable start.
    /// This is not permission to execute the native tool.
    pub fn into_hook_admission(self) -> ResidentHookAdmission {
        self.admission
    }
}

impl std::fmt::Debug for StartedResidentNativeTool {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StartedResidentNativeTool(<awaiting native decision>)")
    }
}

impl PaleApi {
    /// Sign a bounded canonical request claim for this exact prepared event.
    ///
    /// The host supplies all authority coordinates and the independent signing
    /// owner. This call selects no key or grant and sends nothing to the resident.
    /// The returned request must still pass receiver admission.
    ///
    /// # Errors
    /// Rejects malformed, noncanonical, substituted or continuation-shaped
    /// claims before signing. Signing failure or deadline expiry returns no
    /// request and is never retried automatically.
    pub fn sign_resident_native_tool_request(
        &self,
        prepared: &PreparedResidentNativeTool,
        unsigned_request: &[u8],
        signer: &mut dyn ResidentNativeRequestSigner,
        deadline: Instant,
    ) -> Result<ResidentInvokeStartRequest, ApiError> {
        let failure = |error| {
            let (code, summary, disposition, retry, help) = match error {
                NativeSigningError::Request => (
                    "resident_native_request_invalid",
                    "native request claim does not match the exact prepared event and contract",
                    DiagnosticDispositionV1::InvalidInput,
                    DiagnosticRetryV1::AfterCorrection,
                    "Supply the exact prepared body and current canonical request contract.",
                ),
                NativeSigningError::Signing => (
                    "resident_native_request_signing_unavailable",
                    "the selected request signer refused or exceeded its deadline",
                    DiagnosticDispositionV1::Blocked,
                    DiagnosticRetryV1::AfterStateChange,
                    "Resolve the selected request-signing owner's refusal before trying again.",
                ),
            };
            ApiError::new_with_opaque_source(
                &self.diagnostic_identity,
                error,
                crate::error::FailureSpecification {
                    code,
                    summary,
                    disposition,
                    retry,
                    stage: DiagnosticStageV1::ResidentAdmission,
                    effect: DiagnosticEffectCertaintyV1::NotStarted,
                    subject: None,
                    schema_coordinate: None,
                    location: None,
                    request: None,
                    notes: vec!["No resident connection or native effect started."],
                    help: vec![help],
                    facts: Vec::new(),
                },
            )
        };
        let invalid = || failure(NativeSigningError::Request);
        if unsigned_request.is_empty() || unsigned_request.len() > crate::RESIDENT_RECORD_BYTE_LIMIT
        {
            return Err(invalid());
        }
        let value = pale_core::canon::parse_strict(unsigned_request).map_err(|_| invalid())?;
        let preimage: CommonRequestEnvelopePreimageV1 =
            serde_json::from_value(value).map_err(|_| invalid())?;
        let canonical = pale_core::canon::to_canonical_string(&preimage).map_err(|_| invalid())?;
        let schema = pale_contracts::schema_descriptor("common-request-envelope.v1")
            .map_err(|_| invalid())?;
        if canonical.as_bytes() != unsigned_request
            || preimage.envelope_schema_sha256.as_str() != schema.schema_sha256
            || preimage.body != prepared.body
            || preimage.operation_target.is_some()
        {
            return Err(invalid());
        }
        let mut signing_attempted = false;
        let signed = CommonRequestEnvelopeWireV1::sign_candidate(preimage, |purpose, _, digest| {
            signing_attempted = true;
            let refused = || InvalidResidentRecordV1::ContextualAdmission {
                record: "common request",
                invariant: "request signing refused",
            };
            if Instant::now() >= deadline {
                return Err(refused());
            }
            let signature = signer
                .sign(
                    ResidentNativeRequestSigningInput {
                        preimage: canonical.as_bytes(),
                        purpose,
                        digest: digest.as_str(),
                    },
                    deadline,
                )
                .map_err(|_| refused())?;
            if Instant::now() >= deadline {
                return Err(refused());
            }
            CanonicalSignatureV1::parse(signature).map_err(|_| refused())
        })
        .map_err(|_| {
            failure(if signing_attempted {
                NativeSigningError::Signing
            } else {
                NativeSigningError::Request
            })
        })?;
        let signed = pale_core::canon::to_canonical_string(&signed).map_err(|_| invalid())?;
        if signed.len() > crate::RESIDENT_RECORD_BYTE_LIMIT {
            return Err(invalid());
        }
        Ok(ResidentInvokeStartRequest::new(signed.into_bytes()))
    }

    /// Prepare a pre-effect native event for the host's independently supplied signer.
    ///
    /// # Errors
    /// Rejects unsupported events, malformed or missing native identities and
    /// events that cannot be represented by the canonical body contract.
    pub fn prepare_resident_native_tool(
        &self,
        admission: ResidentHookAdmission,
    ) -> Result<PreparedResidentNativeTool, ApiError> {
        let invalid = || {
            ingress_failure(
                &self.diagnostic_identity,
                ResidentHookIngressAdmissionError::Malformed("native tool start body is invalid"),
            )
        };
        let prepared = ResidentNativeToolStart::prepare(
            admission._request.routing().clone(),
            admission.frame.native_event(),
        )
        .map_err(|_| invalid())?;
        let body = prepared.body().clone();
        let body_json = pale_core::canon::to_canonical_string(&body)
            .map_err(|_| invalid())?
            .into_bytes();
        Ok(PreparedResidentNativeTool {
            admission,
            body,
            body_json,
        })
    }

    /// Submit a separately signed common request for this exact native occurrence.
    ///
    /// # Errors
    /// Rejects a substituted body before opening a connection, or any malformed,
    /// rejected, uncertain or mismatched operation response. No start result is
    /// an allow decision. The caller must preserve uncertain starts for recovery.
    pub fn start_resident_native_tool(
        &self,
        attachment: Option<&ResidentAttachment>,
        prepared: PreparedResidentNativeTool,
        request: ResidentInvokeStartRequest,
    ) -> Result<StartedResidentNativeTool, ApiError> {
        let observation =
            self.start_resident_invoke_with_body(attachment, request, Some(&prepared.body))?;
        Ok(StartedResidentNativeTool {
            observation,
            admission: prepared.admission,
        })
    }
}
