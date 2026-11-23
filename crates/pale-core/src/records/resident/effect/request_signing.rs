//! Contextual issuance before invoking the selected common-request signer.

use super::*;

/// Signature purpose for a Pale common request, distinct from transport proofs.
/// The signer signs this domain, including its NUL, followed by the decoded
/// 32-byte canonical request digest. This constant selects no credential.
pub const COMMON_REQUEST_ENVELOPE_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_COMMON_REQUEST_ENVELOPE_V1\0";

impl CommonRequestEnvelopeWireV1 {
    /// Sign a structurally valid request claim without admitting its authority.
    ///
    /// The signer must select a key authorized for the claimed caller and the
    /// supplied request purpose. The resulting wire remains untrusted: its
    /// receiver must independently admit the complete context and verify the
    /// signature. This path never derives authority from the candidate itself.
    ///
    /// # Errors
    /// Rejects invalid structure or payload digests before signing, or returns
    /// the selected signer's refusal without creating a signed candidate.
    pub fn sign_candidate<Sign>(
        preimage: CommonRequestEnvelopePreimageV1,
        sign: Sign,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Sign: FnOnce(
            &[u8],
            &CommonRequestEnvelopePreimageV1,
            &CommonRequestEnvelopeDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        preimage.validate()?;
        let digest = CommonRequestEnvelopeDigest::hash_canonical(&preimage)?;
        let signature = sign(
            COMMON_REQUEST_ENVELOPE_V1_SIGNATURE_DOMAIN,
            &preimage,
            &digest,
        )?;
        Ok(Self {
            envelope_schema_id: preimage.envelope_schema_id,
            envelope_schema_sha256: preimage.envelope_schema_sha256,
            request_id: preimage.request_id,
            correlation_id: preimage.correlation_id,
            workspace_id: preimage.workspace_id,
            capability_id: preimage.capability_id,
            operation: preimage.operation,
            caller: preimage.caller,
            signing_authority_binding: preimage.signing_authority_binding,
            route: preimage.route,
            session_admission: preimage.session_admission,
            operation_target: preimage.operation_target,
            execution_owner_fence: preimage.execution_owner_fence,
            federation_claim: preimage.federation_claim,
            scope: preimage.scope,
            risk_class: preimage.risk_class,
            resource_constraints: preimage.resource_constraints,
            expected_precondition: preimage.expected_precondition,
            idempotency_key: preimage.idempotency_key,
            issued_at: preimage.issued_at,
            expires_at: preimage.expires_at,
            generation: preimage.generation,
            fence_token: preimage.fence_token,
            body: preimage.body,
            canonical_request_sha256: digest,
            caller_signature: signature,
        })
    }
}

impl CommonRequestEnvelopeV1 {
    /// Validate the complete owner-resolved request before asking its signer.
    ///
    /// The context must come from independently admitted caller, session, route
    /// and authority evidence. Matching a context copied from untrusted input
    /// supplies no authority. The signing owner must select an authorized key
    /// for this caller and purpose; a transport credential alone is insufficient.
    /// The receiver still checks current authority and expiry on admission.
    ///
    /// # Errors
    /// Rejects an invalid preimage, a mismatch with the complete context or an
    /// execution fence that does not contain the request. These checks precede
    /// the only signing call. A refused signature produces no issued request.
    pub fn issue_contextual<Sign>(
        preimage: CommonRequestEnvelopePreimageV1,
        context: &CommonRequestAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Sign: FnOnce(
            &[u8],
            &CommonRequestEnvelopePreimageV1,
            &CommonRequestEnvelopeDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        preimage.validate()?;
        context.signing_authority.verify()?;
        validate_common_request_context(&preimage, context)?;
        let digest = CommonRequestEnvelopeDigest::hash_canonical(&preimage)?;
        let signature = sign(
            COMMON_REQUEST_ENVELOPE_V1_SIGNATURE_DOMAIN,
            &preimage,
            &digest,
        )?;
        Self::seal(preimage, signature)
    }
}
