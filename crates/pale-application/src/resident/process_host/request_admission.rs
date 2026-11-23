//! Shared bounded admission for one caller-signed common request.
//!
//! This private primitive proves bytes and owner-resolved context only. It does
//! not authenticate a transport peer, select a deployment, choose policy,
//! admit replay, disclose state, or execute an operation.

use pale_core::{canon, records::*};

pub(super) type CallerSignatureVerification<'a> = dyn FnMut(
        &CommonRequestEnvelopePreimageV1,
        &CommonRequestEnvelopeDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;

/// One bounded canonical wire value decoded before owner-context selection.
///
/// Its preimage remains untrusted. Callers may use immutable coordinates only
/// to locate independently owned evidence; they must not use them as admitted
/// policy or authority.
pub(super) struct DecodedCommonRequestWire {
    wire: CommonRequestEnvelopeWireV1,
    preimage: CommonRequestEnvelopePreimageV1,
    canonical: Box<[u8]>,
}

impl DecodedCommonRequestWire {
    /// Borrow the unsigned coordinates claimed by the untrusted wire value.
    pub(super) const fn untrusted_preimage(&self) -> &CommonRequestEnvelopePreimageV1 {
        &self.preimage
    }

    /// Complete contextual and caller-signature admission.
    pub(super) fn admit(
        self,
        context: &CommonRequestAdmissionContextV1<'_>,
        verify_signature: &mut CallerSignatureVerification<'_>,
    ) -> Result<CommonRequestEnvelopeV1, ResidentCommonRequestAdmissionError> {
        let request = CommonRequestEnvelopeV1::admit_contextual_wire(
            self.wire,
            context,
            |preimage, digest, signature| verify_signature(preimage, digest, signature),
        )
        .map_err(ResidentCommonRequestAdmissionError::Admission)?;
        if canon::to_canonical_string(&request)
            .map_err(ResidentCommonRequestAdmissionError::Canonical)?
            .as_bytes()
            != self.canonical.as_ref()
        {
            return Err(ResidentCommonRequestAdmissionError::NotCanonical);
        }
        Ok(request)
    }
}

/// Why common-request bytes failed before any use-case owner ran.
#[derive(Debug, thiserror::Error)]
pub(super) enum ResidentCommonRequestAdmissionError {
    /// Bytes were absent or exceeded the explicit ceiling.
    #[error("common request bytes are absent or exceed their bound")]
    Bounds,
    /// JSON was malformed or could not be represented canonically.
    #[error("common request bytes are not valid canonical JSON")]
    Canonical(#[source] canon::CanonError),
    /// Valid JSON bytes were not their exact canonical encoding.
    #[error("common request bytes are not exact canonical JSON")]
    NotCanonical,
    /// Bytes did not satisfy the selected public contract.
    #[error("common request bytes do not satisfy their selected public contract")]
    Contract(#[source] pale_contracts::SchemaError),
    /// Contract-valid JSON did not decode into the exact wire type.
    #[error("common request bytes do not decode as a common request")]
    Wire(#[source] serde_json::Error),
    /// Owner context or caller-signature admission rejected the request.
    #[error("common request did not pass owner admission")]
    Admission(#[source] InvalidResidentRecordV1),
}

/// Admit exact bounded canonical request bytes against owner-resolved context.
///
/// # Errors
///
/// Returns a typed failure for bounds, canonical form, contract shape, wire
/// decoding, contextual association, or caller-signature verification.
pub(super) fn admit_common_request_bytes(
    bytes: &[u8],
    maximum_bytes: usize,
    context: &CommonRequestAdmissionContextV1<'_>,
    verify_signature: &mut CallerSignatureVerification<'_>,
) -> Result<CommonRequestEnvelopeV1, ResidentCommonRequestAdmissionError> {
    decode_common_request_bytes(bytes, maximum_bytes)?.admit(context, verify_signature)
}

/// Check that one signed request names the exact operation in its outer frame.
///
/// The dedicated target coordinate preserves operation-specific body
/// semantics while preventing an unsigned outer frame from retargeting them.
pub(super) fn request_binds_operation_target(
    request: &CommonRequestEnvelopePreimageV1,
    target: &OperationTargetReferenceV1,
) -> bool {
    request.operation_target.as_ref() == Some(target)
        && !request
            .scope
            .authority_evidence
            .iter()
            .any(|evidence| evidence.authority_ref() == &target.operation_handle_ref)
}

/// Decode one exact bounded canonical common-request wire value.
///
/// This is the only pre-admission stage exposed to the resident program. The
/// returned fields can select independently owned evidence, but cannot enter a
/// use-case owner until [`DecodedCommonRequestWire::admit`] succeeds.
pub(super) fn decode_common_request_bytes(
    bytes: &[u8],
    maximum_bytes: usize,
) -> Result<DecodedCommonRequestWire, ResidentCommonRequestAdmissionError> {
    if bytes.is_empty() || maximum_bytes == 0 || bytes.len() > maximum_bytes {
        return Err(ResidentCommonRequestAdmissionError::Bounds);
    }
    let value =
        canon::parse_strict(bytes).map_err(ResidentCommonRequestAdmissionError::Canonical)?;
    let canonical = canon::to_canonical_string(&value)
        .map_err(ResidentCommonRequestAdmissionError::Canonical)?;
    if canonical.as_bytes() != bytes {
        return Err(ResidentCommonRequestAdmissionError::NotCanonical);
    }
    pale_contracts::validate_value(&value, COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID)
        .map_err(ResidentCommonRequestAdmissionError::Contract)?;
    let wire: CommonRequestEnvelopeWireV1 =
        serde_json::from_value(value).map_err(ResidentCommonRequestAdmissionError::Wire)?;
    let preimage = wire.untrusted_preimage();
    Ok(DecodedCommonRequestWire {
        wire,
        preimage,
        canonical: bytes.into(),
    })
}
