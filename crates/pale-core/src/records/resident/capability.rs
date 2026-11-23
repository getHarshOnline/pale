//! Signed capability-result and bounded adapter-failure contracts.
//!
//! [`CapabilityResultV1`] owns the exact success/failure and payload-representation
//! sum law. [`AdapterFailureV1`] owns the corresponding non-effect evidence law.
//!
//! External-effect ambiguity remains the responsibility of [`EffectReceiptV1`]
//! and reconciliation. These records neither dispatch an implementation nor
//! turn a timeout, response, or missing receipt into proof of effect absence.

use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

/// Public schema root for signed capability results.
pub const CAPABILITY_RESULT_V1_SCHEMA_ID: &str = "capability-result.v1";
/// Wire schema carried by every signed capability result.
pub const CAPABILITY_RESULT_V1_SCHEMA: &str = "pale/capability-result.v1";
/// Public schema root for signed bounded adapter failures.
pub const ADAPTER_FAILURE_V1_SCHEMA_ID: &str = "adapter-failure.v1";
/// Wire schema carried by every signed bounded adapter failure.
pub const ADAPTER_FAILURE_V1_SCHEMA: &str = "pale/adapter-failure.v1";
/// Signature domain for [`CapabilityResultV1`].
pub const CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_CAPABILITY_RESULT_V1\0";
/// Signature domain for [`AdapterFailureV1`].
pub const ADAPTER_FAILURE_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_ADAPTER_FAILURE_V1\0";

/// Terminal branch of one capability transport attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityResultStatusV1 {
    /// One admitted result payload is available.
    Succeeded,
    /// One independently admitted safe-failure record is available.
    Failed,
}

/// Admitted payload coordinates carried by a successful capability result.
///
/// Exactly one of [`Self::inline_payload`] and [`Self::immutable_result_ref`]
/// is present. The schema coordinate must belong to the request's exact
/// [`ContractSetV1`]; carrying its ID and digest does not establish admission.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(transform = capability_success_schema_v1)]
#[serde(deny_unknown_fields)]
pub struct CapabilitySuccessV1 {
    result_schema_id: SchemaId,
    result_schema_sha256: SchemaDigest,
    result_payload_sha256: ArtifactDigest,
    inline_payload: Option<serde_json::Value>,
    immutable_result_ref: Option<ResidentRecordRefV1>,
}

impl_resident_wire_deserialize!(CapabilitySuccessV1 {
    result_schema_id: SchemaId,
    result_schema_sha256: SchemaDigest,
    result_payload_sha256: ArtifactDigest,
    inline_payload: Option<serde_json::Value>,
    immutable_result_ref: Option<ResidentRecordRefV1>,
});

impl CapabilitySuccessV1 {
    /// Construct bounded inline payload coordinates, without admitting a result.
    ///
    /// The payload digest covers its canonical JSON bytes. This does not prove
    /// that the schema is selected or that the payload satisfies it. Pass these
    /// coordinates to [`CapabilityResultV1::issue_success_contextual`], or use
    /// its wire-admission counterpart, to establish those obligations.
    ///
    /// # Errors
    /// Rejects noncanonical JSON numbers or exceeded resident byte, depth or
    /// member limits. The immutable-reference branch is always absent.
    pub fn inline(
        schema_id: SchemaId,
        schema_sha256: SchemaDigest,
        payload: serde_json::Value,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let bytes = validate_bounded_json("capability inline result", &payload)?;
        Ok(Self {
            result_schema_id: schema_id,
            result_schema_sha256: schema_sha256,
            result_payload_sha256: ArtifactDigest::hash_bytes(bytes.as_bytes()),
            inline_payload: Some(payload),
            immutable_result_ref: None,
        })
    }

    /// Borrow the selected result-schema identity.
    #[must_use]
    pub const fn result_schema_id(&self) -> &SchemaId {
        &self.result_schema_id
    }

    /// Borrow the digest of the exact selected result-schema bytes.
    #[must_use]
    pub const fn result_schema_sha256(&self) -> &SchemaDigest {
        &self.result_schema_sha256
    }

    /// Borrow the digest of the canonical inline or resolved immutable bytes.
    #[must_use]
    pub const fn result_payload_sha256(&self) -> &ArtifactDigest {
        &self.result_payload_sha256
    }

    /// Borrow the inline JSON representation when selected.
    #[must_use]
    pub const fn inline_payload(&self) -> Option<&serde_json::Value> {
        self.inline_payload.as_ref()
    }

    /// Borrow the immutable result reference when selected.
    #[must_use]
    pub const fn immutable_result_ref(&self) -> Option<&ResidentRecordRefV1> {
        self.immutable_result_ref.as_ref()
    }
}

/// Exact safe-failure coordinate carried by a failed capability result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityFailureV1 {
    failure_ref: ResidentRecordRefV1,
    failure_sha256: AdapterFailureDigest,
}

impl CapabilityFailureV1 {
    /// Borrow the immutable safe-failure reference.
    #[must_use]
    pub const fn failure_ref(&self) -> &ResidentRecordRefV1 {
        &self.failure_ref
    }

    /// Borrow the canonical identity of the referenced safe failure.
    #[must_use]
    pub const fn failure_sha256(&self) -> &AdapterFailureDigest {
        &self.failure_sha256
    }
}

/// Stable, provider-neutral safe-failure class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdapterFailureClassV1 {
    /// The request lacks sufficient authority.
    Denied,
    /// The admitted input is invalid for the selected capability.
    Invalid,
    /// The selected implementation cannot satisfy the exact schema.
    UnsupportedSchema,
    /// The request conflicts with an admitted identity or expected state.
    Conflict,
    /// A required bounded service, credential, or owner path is unavailable.
    Unavailable,
    /// The implementation declared a later retry coordinate.
    RateLimited,
    /// The bounded call timed out without external-effect ambiguity.
    Timeout,
    /// A safe non-effect failure requiring capability-specific reconciliation.
    ///
    /// This class is reconciliation-only; it never means that an external
    /// effect has an unknown outcome, which is outside [`AdapterFailureV1`].
    Unknown,
    /// A declared finite capability-specific non-effect computation completed partially.
    ///
    /// The caller may reconcile it or resume only the declared finite
    /// composite. A partially observed external effect belongs to
    /// [`EffectReceiptV1`] and reconciliation instead.
    Partial,
    /// The implementation encountered a bounded internal failure.
    Internal,
}

/// Effect-absence proof carried by one adapter failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdapterFailureEffectStateV1 {
    /// Dispatch was never released to an external owner.
    NotStarted,
    /// Independent evidence established that no external effect was applied.
    IndependentlyProvenNotApplied,
}

resident_signed_record!(
    /// Signed terminal disposition for one capability transport attempt.
    ///
    /// Admission preserves the complete [`RequestBindingV1`] and independently
    /// verifies the producer's [`SigningAuthorityBindingV1`]. A result does not
    /// inherit caller authority and does not establish provider execution availability.
    ///
    /// In the law below, `S` and `F` mean that the success and failure members
    /// are present; `I` and `M` mean that success uses the inline-payload or
    /// immutable-reference representation. Success admits exactly one of `I`
    /// and `M`, while failure admits `F` alone. Thus no admitted result can be
    /// both successful and failed or carry two success representations.
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/capability-result.display.mathml"
    ))]
    CapabilityResultV1,
    CapabilityResultWireV1,
    CapabilityResultPreimageV1,
    canonical_result_sha256: CapabilityResultDigest,
    CAPABILITY_RESULT_V1_SCHEMA,
    "capability result",
    {
        /// Immutable lineage of the admitted caller request.
        request_binding: RequestBindingV1,
        /// Distinct transport attempt within the logical request.
        attempt_id: CapabilityAttemptIdV1,
        /// Explicit terminal result branch.
        status: CapabilityResultStatusV1,
        /// Successful result coordinates, present only for [`CapabilityResultStatusV1::Succeeded`].
        success: Option<CapabilitySuccessV1>,
        /// Safe-failure coordinates, present only for [`CapabilityResultStatusV1::Failed`].
        failure: Option<CapabilityFailureV1>,
        /// Selected implementation class, not a provider response body.
        implementation_kind: BoundedResidentTextV1,
        /// Selected implementation version.
        implementation_version: BoundedResidentTextV1,
        /// Explicit attempt start time.
        started_at: UnixTimeV1,
        /// Explicit terminal result time.
        completed_at: UnixTimeV1,
        /// Exact authority regime of the result producer.
        signing_authority_binding: SigningAuthorityBindingV1,
        /// Producer principal matched against [`Self::signing_authority_binding`].
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Producer signing key matched against [`Self::signing_authority_binding`].
        issuer_key_id: ResidentKeyIdV1,
        /// Producer signing-key epoch matched against [`Self::signing_authority_binding`].
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_capability_result_shape_v1,
    wire_transform = capability_result_schema_v1
);

resident_signed_record!(
    /// Signed, bounded and provider-neutral evidence of a safe adapter failure.
    ///
    /// This record cannot represent an ambiguous or observed external effect.
    /// Those states require [`EffectReceiptV1`] and reconciliation custody.
    ///
    /// In the law below, `R` and `D` mean that [`Self::absence_evidence_ref`]
    /// and its paired digest are present. `request` and `attempt` are the exact
    /// [`Self::request_binding`] and [`Self::attempt_id`]. A not-started failure
    /// carries neither absence member; independently proven non-application
    /// carries and admits both. Neither branch can encode an unknown effect
    /// outcome.
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/adapter-failure.display.mathml"
    ))]
    AdapterFailureV1,
    AdapterFailureWireV1,
    AdapterFailurePreimageV1,
    canonical_failure_sha256: AdapterFailureDigest,
    ADAPTER_FAILURE_V1_SCHEMA,
    "adapter failure",
    {
        /// Immutable lineage of the admitted caller request.
        request_binding: RequestBindingV1,
        /// Stable identity of this failure record.
        failure_id: AdapterFailureIdV1,
        /// Exact capability transport attempt that failed safely.
        attempt_id: CapabilityAttemptIdV1,
        /// Stable provider-neutral failure class.
        failure_class: AdapterFailureClassV1,
        /// Bounded owner-defined reason code.
        stable_code: ResidentReasonV1,
        /// Optional safe human explanation; never a raw provider body.
        bounded_message: Option<BoundedResidentTextV1>,
        /// Optional bounded provider status class.
        provider_status: Option<BoundedResidentTextV1>,
        /// Optional owner-declared retry time.
        retry_at: Option<UnixTimeV1>,
        /// Exact non-effect proof branch.
        effect_state: AdapterFailureEffectStateV1,
        /// Independent absence-evidence reference when required by [`Self::effect_state`].
        absence_evidence_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::absence_evidence_ref`].
        absence_evidence_sha256: Option<ArtifactDigest>,
        /// Selected implementation class.
        implementation_kind: BoundedResidentTextV1,
        /// Selected implementation version.
        implementation_version: BoundedResidentTextV1,
        /// Explicit time at which the failure was recorded.
        recorded_at: UnixTimeV1,
        /// Exact authority regime of the failure producer.
        signing_authority_binding: SigningAuthorityBindingV1,
        /// Producer principal matched against [`Self::signing_authority_binding`].
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Producer signing key matched against [`Self::signing_authority_binding`].
        issuer_key_id: ResidentKeyIdV1,
        /// Producer signing-key epoch matched against [`Self::signing_authority_binding`].
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_adapter_failure_shape_v1,
    wire_transform = adapter_failure_schema_v1
);

/// Owner-resolved coordinates shared by both capability-result branches.
#[derive(Clone, Copy)]
pub struct CapabilityResultAdmissionContextV1<'a> {
    /// Digest of the exact selected capability-result schema bytes.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Complete request lineage expected in the result.
    pub expected_request_binding: &'a RequestBindingV1,
    /// Exact transport attempt expected in the result.
    pub expected_attempt_id: &'a CapabilityAttemptIdV1,
    /// Exact selected implementation class.
    pub expected_implementation_kind: &'a BoundedResidentTextV1,
    /// Exact selected implementation version.
    pub expected_implementation_version: &'a BoundedResidentTextV1,
    /// Independently resolved producer authority.
    pub signing_authority: &'a SigningAuthorityBindingV1,
}

/// Success-only proof that the result schema belongs to the request's contract set.
#[derive(Clone, Copy)]
pub struct CapabilitySuccessAdmissionContextV1<'a> {
    /// Common outer-result coordinates and producer authority.
    pub result: CapabilityResultAdmissionContextV1<'a>,
    /// Immutable reference assigned to [`Self::contract_set`].
    pub contract_set_ref: &'a ResidentRecordRefV1,
    /// Admitted contract set selected by the request.
    pub contract_set: &'a ContractSetV1,
}

impl CapabilitySuccessAdmissionContextV1<'_> {
    /// Check that an exact result schema belongs to this request's admitted set.
    ///
    /// Use before dispatch when an implementation has one known return format.
    /// This joins the request's set reference and digest to the supplied set;
    /// it does not admit payloads, producer authority or execution outcomes.
    ///
    /// # Errors
    /// Rejects an invalid set, another request's set, or an unselected schema
    /// identity/digest pair.
    pub fn validate_result_schema(
        &self,
        schema_id: &SchemaId,
        schema_sha256: &SchemaDigest,
    ) -> Result<(), InvalidResidentRecordV1> {
        self.contract_set.verify()?;
        if self.result.expected_request_binding.contract_set_ref() != self.contract_set_ref
            || self
                .result
                .expected_request_binding
                .contract_set_sha256()
                .as_str()
                != self.contract_set.canonical_contract_set_sha256().as_str()
            || !self.contract_set.entries().iter().any(|entry| {
                entry.contract_id() == schema_id && entry.contract_schema_sha256() == schema_sha256
            })
        {
            return Err(contextual(
                "capability result",
                "result schema selected by exact contract set",
            ));
        }
        Ok(())
    }
}

/// Owner-resolved context for one bounded adapter failure.
#[derive(Clone, Copy)]
pub struct AdapterFailureAdmissionContextV1<'a> {
    /// Digest of the exact selected adapter-failure schema bytes.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Complete request lineage expected in the failure.
    pub expected_request_binding: &'a RequestBindingV1,
    /// Exact transport attempt expected in the failure.
    pub expected_attempt_id: &'a CapabilityAttemptIdV1,
    /// Exact selected implementation class.
    pub expected_implementation_kind: &'a BoundedResidentTextV1,
    /// Exact selected implementation version.
    pub expected_implementation_version: &'a BoundedResidentTextV1,
    /// Independently resolved producer authority.
    pub signing_authority: &'a SigningAuthorityBindingV1,
}

impl CapabilityResultV1 {
    /// Issue a successful result only after its context and payload admit.
    ///
    /// Uses the same request, attempt, implementation, signer, contract-set and
    /// payload laws as [`Self::admit_success_wire`]. `sign` runs after every
    /// check, with that owner's signature domain and canonical preimage digest.
    /// The result is not evidence of durable operation custody or settlement.
    ///
    /// `admit_payload` validates the exact selected schema. Inline JSON uses its
    /// canonical encoding; immutable content uses the exact resolved bytes.
    ///
    /// # Errors
    /// Rejects malformed result shape, unlike context, missing or oversized
    /// payload, wrong payload digest, schema rejection or signing failure.
    pub fn issue_success_contextual<Resolve, Payload, Sign>(
        preimage: CapabilityResultPreimageV1,
        context: &CapabilitySuccessAdmissionContextV1<'_>,
        resolve_immutable: Resolve,
        admit_payload: Payload,
        sign: Sign,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Resolve: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
        Payload: FnOnce(&SchemaId, &SchemaDigest, &[u8]) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &CapabilityResultPreimageV1,
            &CapabilityResultDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if preimage.schema_id.as_str() != CAPABILITY_RESULT_V1_SCHEMA {
            return Err(InvalidResidentRecordV1::SchemaCoordinate {
                family: "capability result",
            });
        }
        validate_capability_result_shape_v1(&preimage)?;
        let success = validate_success_context_v1(&preimage, context)?;
        admit_success_payload_v1(success, resolve_immutable, admit_payload)?;
        let digest = CapabilityResultDigest::hash_canonical(&preimage)?;
        let signature = sign(CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN, &preimage, &digest)?;
        Self::seal(preimage, signature)
    }

    /// Admit a successful result with one schema-qualified payload.
    ///
    /// `admit_payload` must validate the supplied bytes against the exact
    /// schema selected by `context.contract_set`. Inline JSON is passed in its
    /// RFC 8785 encoding; immutable bytes are passed exactly as resolved.
    /// `verify_signature` must authenticate the supplied signature domain and
    /// canonical digest under the independently resolved producer authority.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for schema, branch, contract-set,
    /// payload, digest, request, attempt, implementation, producer-authority,
    /// chronology, or signature failure.
    pub fn admit_success_wire<Resolve, Payload, Signature>(
        wire: CapabilityResultWireV1,
        context: &CapabilitySuccessAdmissionContextV1<'_>,
        resolve_immutable: Resolve,
        admit_payload: Payload,
        verify_signature: Signature,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Resolve: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
        Payload: FnOnce(&SchemaId, &SchemaDigest, &[u8]) -> Result<(), InvalidResidentRecordV1>,
        Signature: FnOnce(
            &[u8],
            &CapabilityResultPreimageV1,
            &CapabilityResultDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            let success = validate_success_context_v1(preimage, context)?;
            verify_signature(
                CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )?;
            admit_success_payload_v1(success, resolve_immutable, admit_payload)
        })
    }

    /// Admit a failed result referencing one independently admitted safe failure.
    ///
    /// The resolved [`AdapterFailureV1`] must preserve the complete request
    /// binding and transport-attempt identity. This function does not turn the
    /// failure into an effect receipt or authorize a retry.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for schema, branch, unresolved or
    /// transposed failure evidence, request, attempt, implementation,
    /// producer-authority, chronology, digest, or signature failure.
    pub fn admit_failure_wire<Resolve, Signature>(
        wire: CapabilityResultWireV1,
        context: &CapabilityResultAdmissionContextV1<'_>,
        resolve_failure: Resolve,
        verify_signature: Signature,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Resolve: FnOnce(&ResidentRecordRefV1) -> Option<AdapterFailureV1>,
        Signature: FnOnce(
            &[u8],
            &CapabilityResultPreimageV1,
            &CapabilityResultDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_result_context_v1(preimage, context)?;
            let failure = match (preimage.status, &preimage.success, &preimage.failure) {
                (CapabilityResultStatusV1::Failed, None, Some(failure)) => failure,
                _ => return Err(contextual("capability result", "exclusive failure branch")),
            };
            verify_signature(
                CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )?;
            let admitted = resolve_failure(&failure.failure_ref)
                .ok_or_else(|| contextual("capability result", "safe-failure resolution"))?;
            admitted.verify()?;
            if admitted.canonical_failure_sha256() != &failure.failure_sha256
                || admitted.request_binding() != &preimage.request_binding
                || admitted.attempt_id() != &preimage.attempt_id
            {
                return Err(contextual(
                    "capability result",
                    "exact safe-failure request and attempt",
                ));
            }
            Ok(())
        })
    }
}

impl AdapterFailureV1 {
    /// Admit a safe failure established before external dispatch.
    ///
    /// `admit_class` owns capability-specific restrictions such as when
    /// `timeout` or `partial` is meaningful. It receives the exact request and
    /// attempt so the decision is evidence-bound rather than ambient closure
    /// state. The generic contract enforces the stable class and retry
    /// coordinate but does not invent capability policy.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for schema, state, request,
    /// attempt, implementation, class-policy, producer-authority, digest, or
    /// signature failure.
    pub fn admit_before_dispatch_wire<Class, Signature>(
        wire: AdapterFailureWireV1,
        context: &AdapterFailureAdmissionContextV1<'_>,
        admit_class: Class,
        verify_signature: Signature,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Class: FnOnce(
            AdapterFailureClassV1,
            Option<UnixTimeV1>,
            &RequestBindingV1,
            &CapabilityAttemptIdV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        Signature: FnOnce(
            &[u8],
            &AdapterFailurePreimageV1,
            &AdapterFailureDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_failure_context_v1(preimage, context)?;
            if preimage.effect_state != AdapterFailureEffectStateV1::NotStarted
                || preimage.absence_evidence_ref.is_some()
                || preimage.absence_evidence_sha256.is_some()
            {
                return Err(contextual(
                    "adapter failure",
                    "pre-dispatch non-effect branch",
                ));
            }
            verify_signature(
                ADAPTER_FAILURE_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )?;
            admit_class(
                preimage.failure_class,
                preimage.retry_at,
                &preimage.request_binding,
                &preimage.attempt_id,
            )?;
            Ok(())
        })
    }

    /// Admit a safe failure supported by independent no-effect evidence.
    ///
    /// Resolving an opaque reference is mandatory: the reference and claimed
    /// digest alone are not proof. `admit_absence_evidence` must establish that
    /// the exact bytes independently prove no effect for this request and
    /// attempt. Unknown, partial, or observed external effects instead require
    /// [`EffectReceiptV1`] and reconciliation.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for schema, state, missing or
    /// transposed absence evidence, request, attempt, implementation,
    /// class-policy, producer-authority, digest, or signature failure.
    pub fn admit_proven_not_applied_wire<Resolve, Evidence, Class, Signature>(
        wire: AdapterFailureWireV1,
        context: &AdapterFailureAdmissionContextV1<'_>,
        resolve_absence_evidence: Resolve,
        admit_absence_evidence: Evidence,
        admit_class: Class,
        verify_signature: Signature,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Resolve: FnOnce(&ResidentRecordRefV1) -> Option<Vec<u8>>,
        Evidence: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &[u8],
            &RequestBindingV1,
            &CapabilityAttemptIdV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        Class: FnOnce(
            AdapterFailureClassV1,
            Option<UnixTimeV1>,
            &RequestBindingV1,
            &CapabilityAttemptIdV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        Signature: FnOnce(
            &[u8],
            &AdapterFailurePreimageV1,
            &AdapterFailureDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_failure_context_v1(preimage, context)?;
            let (reference, expected_digest) = match (
                preimage.effect_state,
                &preimage.absence_evidence_ref,
                &preimage.absence_evidence_sha256,
            ) {
                (
                    AdapterFailureEffectStateV1::IndependentlyProvenNotApplied,
                    Some(reference),
                    Some(digest),
                ) => (reference, digest),
                _ => {
                    return Err(contextual(
                        "adapter failure",
                        "independently proven no-effect branch",
                    ));
                }
            };
            verify_signature(
                ADAPTER_FAILURE_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )?;
            let evidence = resolve_absence_evidence(reference)
                .ok_or_else(|| contextual("adapter failure", "absence-evidence resolution"))?;
            validate_bounded_content_bytes("adapter absence evidence", &evidence)?;
            if ArtifactDigest::hash_bytes(&evidence) != *expected_digest {
                return Err(InvalidResidentRecordV1::DigestMismatch {
                    family: "adapter absence evidence",
                });
            }
            admit_absence_evidence(
                reference,
                expected_digest,
                &evidence,
                &preimage.request_binding,
                &preimage.attempt_id,
            )?;
            admit_class(
                preimage.failure_class,
                preimage.retry_at,
                &preimage.request_binding,
                &preimage.attempt_id,
            )?;
            Ok(())
        })
    }
}

fn validate_capability_result_shape_v1(
    value: &CapabilityResultPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    value.request_binding.verify()?;
    value.signing_authority_binding.verify()?;
    let branch_matches = matches!(
        (value.status, &value.success, &value.failure),
        (CapabilityResultStatusV1::Succeeded, Some(_), None)
            | (CapabilityResultStatusV1::Failed, None, Some(_))
    );
    let representation_matches = value.success.as_ref().is_none_or(|success| {
        success.inline_payload.is_some() != success.immutable_result_ref.is_some()
    });
    if !branch_matches || !representation_matches {
        return Err(InvalidResidentRecordV1::EffectState {
            record: "capability result",
            state: "exclusive terminal branch and result representation",
        });
    }
    if value.started_at > value.completed_at {
        return Err(InvalidResidentRecordV1::InvalidCoordinate {
            coordinate: "capability result chronology",
        });
    }
    Ok(())
}

fn validate_adapter_failure_shape_v1(
    value: &AdapterFailurePreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    value.request_binding.verify()?;
    value.signing_authority_binding.verify()?;
    let evidence_matches = match value.effect_state {
        AdapterFailureEffectStateV1::NotStarted => {
            value.absence_evidence_ref.is_none() && value.absence_evidence_sha256.is_none()
        }
        AdapterFailureEffectStateV1::IndependentlyProvenNotApplied => {
            value.absence_evidence_ref.is_some() && value.absence_evidence_sha256.is_some()
        }
    };
    if !evidence_matches
        || (value.failure_class == AdapterFailureClassV1::RateLimited && value.retry_at.is_none())
    {
        return Err(InvalidResidentRecordV1::EffectState {
            record: "adapter failure",
            state: "effect evidence or retry coordinate",
        });
    }
    Ok(())
}

fn validate_result_context_v1(
    value: &CapabilityResultPreimageV1,
    context: &CapabilityResultAdmissionContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    validate_capability_transport_context_v1(
        "capability result",
        CapabilityTransportCoordinatesV1::from_result(value),
        ExpectedCapabilityTransportV1::from_result(context),
    )
}

fn validate_success_context_v1<'a>(
    preimage: &'a CapabilityResultPreimageV1,
    context: &CapabilitySuccessAdmissionContextV1<'_>,
) -> Result<&'a CapabilitySuccessV1, InvalidResidentRecordV1> {
    validate_result_context_v1(preimage, &context.result)?;
    let success = match (preimage.status, &preimage.success, &preimage.failure) {
        (CapabilityResultStatusV1::Succeeded, Some(success), None) => success,
        _ => return Err(contextual("capability result", "exclusive success branch")),
    };
    context.validate_result_schema(&success.result_schema_id, &success.result_schema_sha256)?;
    Ok(success)
}

fn admit_success_payload_v1<Resolve, Payload>(
    success: &CapabilitySuccessV1,
    mut resolve_immutable: Resolve,
    admit_payload: Payload,
) -> Result<(), InvalidResidentRecordV1>
where
    Resolve: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
    Payload: FnOnce(&SchemaId, &SchemaDigest, &[u8]) -> Result<(), InvalidResidentRecordV1>,
{
    let bytes = match (&success.inline_payload, &success.immutable_result_ref) {
        (Some(inline), None) => {
            validate_bounded_json("capability inline result", inline)?.into_bytes()
        }
        (None, Some(reference)) => {
            let bytes = resolve_immutable(reference)
                .ok_or_else(|| contextual("capability result", "immutable result resolution"))?;
            validate_bounded_content_bytes("capability immutable result", &bytes)?;
            bytes
        }
        _ => return Err(contextual("capability result", "one result representation")),
    };
    if ArtifactDigest::hash_bytes(&bytes) != success.result_payload_sha256 {
        return Err(InvalidResidentRecordV1::DigestMismatch {
            family: "capability result payload",
        });
    }
    admit_payload(
        &success.result_schema_id,
        &success.result_schema_sha256,
        &bytes,
    )
}

fn validate_failure_context_v1(
    value: &AdapterFailurePreimageV1,
    context: &AdapterFailureAdmissionContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    validate_capability_transport_context_v1(
        "adapter failure",
        CapabilityTransportCoordinatesV1::from_failure(value),
        ExpectedCapabilityTransportV1::from_failure(context),
    )
}

struct CapabilityTransportCoordinatesV1<'a> {
    schema_sha256: &'a SchemaDigest,
    request_binding: &'a RequestBindingV1,
    attempt_id: &'a CapabilityAttemptIdV1,
    implementation_kind: &'a BoundedResidentTextV1,
    implementation_version: &'a BoundedResidentTextV1,
    signing_authority_binding: &'a SigningAuthorityBindingV1,
    issuer_principal_id: &'a ResidentPrincipalIdV1,
    issuer_key_id: &'a ResidentKeyIdV1,
    issuer_key_epoch: SigningKeyEpochV1,
}

impl<'a> CapabilityTransportCoordinatesV1<'a> {
    fn from_result(value: &'a CapabilityResultPreimageV1) -> Self {
        Self {
            schema_sha256: &value.schema_sha256,
            request_binding: &value.request_binding,
            attempt_id: &value.attempt_id,
            implementation_kind: &value.implementation_kind,
            implementation_version: &value.implementation_version,
            signing_authority_binding: &value.signing_authority_binding,
            issuer_principal_id: &value.issuer_principal_id,
            issuer_key_id: &value.issuer_key_id,
            issuer_key_epoch: value.issuer_key_epoch,
        }
    }

    fn from_failure(value: &'a AdapterFailurePreimageV1) -> Self {
        Self {
            schema_sha256: &value.schema_sha256,
            request_binding: &value.request_binding,
            attempt_id: &value.attempt_id,
            implementation_kind: &value.implementation_kind,
            implementation_version: &value.implementation_version,
            signing_authority_binding: &value.signing_authority_binding,
            issuer_principal_id: &value.issuer_principal_id,
            issuer_key_id: &value.issuer_key_id,
            issuer_key_epoch: value.issuer_key_epoch,
        }
    }
}

struct ExpectedCapabilityTransportV1<'a> {
    schema_sha256: &'a SchemaDigest,
    request_binding: &'a RequestBindingV1,
    attempt_id: &'a CapabilityAttemptIdV1,
    implementation_kind: &'a BoundedResidentTextV1,
    implementation_version: &'a BoundedResidentTextV1,
    signing_authority: &'a SigningAuthorityBindingV1,
}

impl<'a> ExpectedCapabilityTransportV1<'a> {
    fn from_result(value: &'a CapabilityResultAdmissionContextV1<'a>) -> Self {
        Self {
            schema_sha256: value.expected_schema_sha256,
            request_binding: value.expected_request_binding,
            attempt_id: value.expected_attempt_id,
            implementation_kind: value.expected_implementation_kind,
            implementation_version: value.expected_implementation_version,
            signing_authority: value.signing_authority,
        }
    }

    fn from_failure(value: &'a AdapterFailureAdmissionContextV1<'a>) -> Self {
        Self {
            schema_sha256: value.expected_schema_sha256,
            request_binding: value.expected_request_binding,
            attempt_id: value.expected_attempt_id,
            implementation_kind: value.expected_implementation_kind,
            implementation_version: value.expected_implementation_version,
            signing_authority: value.signing_authority,
        }
    }
}

fn validate_capability_transport_context_v1(
    family: &'static str,
    candidate: CapabilityTransportCoordinatesV1<'_>,
    expected: ExpectedCapabilityTransportV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    validate_exact_request_binding(candidate.request_binding, expected.request_binding, family)?;
    if candidate.attempt_id != expected.attempt_id
        || candidate.implementation_kind != expected.implementation_kind
        || candidate.implementation_version != expected.implementation_version
        || candidate.signing_authority_binding != expected.signing_authority
    {
        return Err(contextual(
            family,
            "exact attempt, implementation, and producer binding",
        ));
    }
    validate_signed_coordinates(
        family,
        candidate.schema_sha256,
        expected.schema_sha256,
        candidate.issuer_principal_id,
        candidate.issuer_key_id,
        candidate.issuer_key_epoch,
        expected.signing_authority,
    )
}

fn contextual(record: &'static str, invariant: &'static str) -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission { record, invariant }
}

fn capability_success_schema_v1(schema: &mut schemars::Schema) {
    forbid_explicit_null_properties_v1(schema);
    let root = schema.ensure_object();
    if let Some(inline_payload) = root
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .and_then(|properties| properties.get_mut("inline_payload"))
    {
        *inline_payload = serde_json::json!({ "not": { "type": "null" } });
    }
    root.insert(
        "oneOf".into(),
        serde_json::json!([
            {
                "required": ["inline_payload"],
                "not": { "required": ["immutable_result_ref"] }
            },
            {
                "required": ["immutable_result_ref"],
                "not": { "required": ["inline_payload"] }
            }
        ]),
    );
}

fn capability_result_schema_v1(schema: &mut schemars::Schema) {
    forbid_explicit_null_properties_v1(schema);
    schema.ensure_object().insert(
        "oneOf".into(),
        serde_json::json!([
            {
                "properties": { "status": { "const": "succeeded" } },
                "required": ["success"],
                "not": { "required": ["failure"] }
            },
            {
                "properties": { "status": { "const": "failed" } },
                "required": ["failure"],
                "not": { "required": ["success"] }
            }
        ]),
    );
}

fn adapter_failure_schema_v1(schema: &mut schemars::Schema) {
    forbid_explicit_null_properties_v1(schema);
    require_all_or_none_properties_v1(schema, &["absence_evidence_ref", "absence_evidence_sha256"]);
    schema.ensure_object().insert(
        "oneOf".into(),
        serde_json::json!([
            {
                "properties": { "effect_state": { "const": "not_started" } },
                "not": { "anyOf": [
                    { "required": ["absence_evidence_ref"] },
                    { "required": ["absence_evidence_sha256"] }
                ] }
            },
            {
                "properties": {
                    "effect_state": { "const": "independently_proven_not_applied" }
                },
                "required": ["absence_evidence_ref", "absence_evidence_sha256"]
            }
        ]),
    );
    schema.ensure_object().insert(
        "allOf".into(),
        serde_json::json!([{
            "if": {
                "properties": { "failure_class": { "const": "rate_limited" } },
                "required": ["failure_class"]
            },
            "then": { "required": ["retry_at"] }
        }]),
    );
}

#[cfg(test)]
#[path = "../../../tests/internal/records/resident/capability.rs"]
mod tests;
