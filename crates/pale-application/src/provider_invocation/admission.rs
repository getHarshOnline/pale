//! Contextual admission of provider-neutral invocation evidence.

use pale_core::records::{
    AdapterFailureAdmissionContextV1, AdapterFailureClassV1, AdapterFailureDigest,
    AdapterFailurePreimageV1, AdapterFailureV1, ArtifactDigest, BoundedResidentTextV1,
    CanonicalSignatureV1, CapabilityAttemptIdV1, CapabilityIdV1,
    CapabilityResultAdmissionContextV1, CapabilityResultDigest, CapabilityResultPreimageV1,
    CapabilityResultV1, CapabilitySuccessAdmissionContextV1, CompiledGenerationDigest,
    ContractSetV1, EffectClass, InstanceDigest, InvalidResidentRecordV1, PackageResourceRefV1,
    ProviderBindingIdentityV1, RequestBindingV1, ResidentRecordRefV1, SchemaDigest, SchemaId,
    SigningAuthorityBindingV1, UnixTimeV1,
};
use pale_ports::{
    ProviderInvocationCoordinateError, ProviderInvocationEvidenceFrontier,
    ProviderInvocationObservation, ProviderInvocationObservationCoordinate, ProviderInvocationPort,
    ProviderInvocationRequest, ProviderInvocationUnavailability,
};

/// Contextual trust services required to admit untrusted provider wire evidence.
///
/// Implementations resolve retained bytes and verify independently selected
/// schemas, failure policy, and signer authority. The provider adapter cannot
/// implement this trait for its own response and thereby self-admit it.
pub trait ProviderInvocationEvidenceAdmission {
    /// Admit the exact owner-evidence frontier carried by a positive observation.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the owner-selected evidence
    /// frontier is not admissible.
    fn admit_evidence_frontier(
        &self,
        frontier: &ProviderInvocationEvidenceFrontier,
    ) -> Result<(), InvalidResidentRecordV1>;

    /// Resolve an immutable payload or absence-evidence reference.
    fn resolve_immutable(&self, reference: &ResidentRecordRefV1) -> Option<Vec<u8>>;

    /// Admit exact payload bytes against their independently selected schema.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the payload does not satisfy
    /// the independently selected schema.
    fn admit_payload(
        &self,
        schema_id: &SchemaId,
        schema_sha256: &SchemaDigest,
        bytes: &[u8],
    ) -> Result<(), InvalidResidentRecordV1>;

    /// Apply capability-specific policy to one structurally safe failure class.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when policy rejects the exact
    /// failure class, request, or attempt.
    fn admit_failure_class(
        &self,
        class: AdapterFailureClassV1,
        retry_at: Option<UnixTimeV1>,
        request: &RequestBindingV1,
        attempt: &CapabilityAttemptIdV1,
    ) -> Result<(), InvalidResidentRecordV1>;

    /// Admit independent bytes proving that the exact attempt applied no effect.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the evidence bytes do not prove
    /// non-application for the exact request and attempt.
    fn admit_absence_evidence(
        &self,
        reference: &ResidentRecordRefV1,
        expected_sha256: &ArtifactDigest,
        bytes: &[u8],
        request: &RequestBindingV1,
        attempt: &CapabilityAttemptIdV1,
    ) -> Result<(), InvalidResidentRecordV1>;

    /// Authenticate one capability result under the selected producer authority.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the digest or signature is not
    /// authentic under the independently selected result authority.
    fn verify_result_signature(
        &self,
        domain: &[u8],
        preimage: &CapabilityResultPreimageV1,
        digest: &CapabilityResultDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>;

    /// Authenticate one adapter failure under the selected producer authority.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the digest or signature is not
    /// authentic under the independently selected failure authority.
    fn verify_failure_signature(
        &self,
        domain: &[u8],
        preimage: &AdapterFailurePreimageV1,
        digest: &AdapterFailureDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>;
}

/// Exact safe non-effect branch selected for adapter-failure admission.
///
/// No variant represents an unknown or partially observed external effect.
/// Such evidence belongs to the effect-case reconciliation path rather
/// than an ordinary provider invocation result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderInvocationSafeFailureProof {
    /// The adapter proves that external dispatch never began.
    BeforeDispatch,
    /// Independent bounded evidence proves that the effect was not applied.
    IndependentlyProvenNotApplied,
}

/// Owner-resolved context for one provider invocation observation.
///
/// Schema selection and signer authority stay in the existing signed-record
/// contexts. Positive outcomes carry their own exact evidence frontier for
/// independent admission; unavailable outcomes carry none.
#[derive(Clone, Copy)]
pub struct ProviderInvocationAdmissionContext<'a> {
    success: CapabilitySuccessAdmissionContextV1<'a>,
    failure: AdapterFailureAdmissionContextV1<'a>,
    safe_failure_proof: ProviderInvocationSafeFailureProof,
}

/// Owned copy of one already selected provider-result admission.
///
/// Recovery continuations use this value only after the semantic owner has
/// selected and validated the attempt. It creates no provider request, signer,
/// contract-set, or replay authority.
#[derive(Clone)]
pub(crate) struct OwnedProviderInvocationAdmission {
    result_schema_sha256: SchemaDigest,
    request_binding: RequestBindingV1,
    attempt_id: CapabilityAttemptIdV1,
    implementation_kind: BoundedResidentTextV1,
    implementation_version: BoundedResidentTextV1,
    result_signing_authority: SigningAuthorityBindingV1,
    contract_set_ref: ResidentRecordRefV1,
    contract_set: ContractSetV1,
    failure_schema_sha256: SchemaDigest,
    failure_signing_authority: SigningAuthorityBindingV1,
    safe_failure_proof: ProviderInvocationSafeFailureProof,
}

/// Provider-result meaning selected before a fresh attempt identity exists.
///
/// This private input carries schemas, producer authorities, and safe-failure
/// policy only. The operation owner supplies request lineage, implementation
/// coordinates, and the freshly allocated attempt when it builds the exact
/// provider invocation admission context.
#[derive(Clone, Copy)]
pub(crate) struct ProviderInvocationAttemptAdmission<'a> {
    pub(crate) result_schema_sha256: &'a SchemaDigest,
    pub(crate) result_signing_authority: &'a SigningAuthorityBindingV1,
    pub(crate) contract_set: &'a ContractSetV1,
    pub(crate) failure_schema_sha256: &'a SchemaDigest,
    pub(crate) failure_signing_authority: &'a SigningAuthorityBindingV1,
    pub(crate) safe_failure_proof: ProviderInvocationSafeFailureProof,
}

impl<'a> ProviderInvocationAttemptAdmission<'a> {
    pub(crate) const fn result_signing_authority(&self) -> &SigningAuthorityBindingV1 {
        self.result_signing_authority
    }

    pub(crate) const fn failure_signing_authority(&self) -> &SigningAuthorityBindingV1 {
        self.failure_signing_authority
    }
}

impl<'a> ProviderInvocationAdmissionContext<'a> {
    pub(crate) fn to_owned_selection(self) -> OwnedProviderInvocationAdmission {
        OwnedProviderInvocationAdmission {
            result_schema_sha256: self.success.result.expected_schema_sha256.clone(),
            request_binding: self.success.result.expected_request_binding.clone(),
            attempt_id: self.success.result.expected_attempt_id.clone(),
            implementation_kind: self.success.result.expected_implementation_kind.clone(),
            implementation_version: self.success.result.expected_implementation_version.clone(),
            result_signing_authority: self.success.result.signing_authority.clone(),
            contract_set_ref: self.success.contract_set_ref.clone(),
            contract_set: self.success.contract_set.clone(),
            failure_schema_sha256: self.failure.expected_schema_sha256.clone(),
            failure_signing_authority: self.failure.signing_authority.clone(),
            safe_failure_proof: self.safe_failure_proof,
        }
    }

    /// Return whether two private admission selections are exactly alike.
    ///
    /// This comparison grants no admission authority. Resident composition
    /// uses it only to prevent a separately supplied recording context from
    /// replacing the one retained by an unopened provider call.
    pub(crate) fn is_same_selection(&self, other: &Self) -> bool {
        let left_result = &self.success.result;
        let right_result = &other.success.result;
        self.safe_failure_proof == other.safe_failure_proof
            && self.success.contract_set_ref == other.success.contract_set_ref
            && self.success.contract_set == other.success.contract_set
            && left_result.expected_schema_sha256 == right_result.expected_schema_sha256
            && left_result.expected_request_binding == right_result.expected_request_binding
            && left_result.expected_attempt_id == right_result.expected_attempt_id
            && left_result.expected_implementation_kind == right_result.expected_implementation_kind
            && left_result.expected_implementation_version
                == right_result.expected_implementation_version
            && left_result.signing_authority == right_result.signing_authority
            && self.failure.expected_schema_sha256 == other.failure.expected_schema_sha256
            && self.failure.expected_request_binding == other.failure.expected_request_binding
            && self.failure.expected_attempt_id == other.failure.expected_attempt_id
            && self.failure.expected_implementation_kind
                == other.failure.expected_implementation_kind
            && self.failure.expected_implementation_version
                == other.failure.expected_implementation_version
            && self.failure.signing_authority == other.failure.signing_authority
    }

    /// Check the selected attempt before retaining its private dispatch basis.
    pub(crate) fn validate_for(
        self,
        request: ProviderInvocationRequest<'_>,
    ) -> Result<(), ProviderInvocationAdmissionError> {
        validate_context(request, self)
    }

    /// Borrow the existing result owner context without selecting new authority.
    pub(crate) const fn success_context(&self) -> &CapabilitySuccessAdmissionContextV1<'a> {
        &self.success
    }

    /// Assemble independently resolved result and safe-failure admission policy.
    #[must_use]
    pub const fn new(
        success: CapabilitySuccessAdmissionContextV1<'a>,
        failure: AdapterFailureAdmissionContextV1<'a>,
        safe_failure_proof: ProviderInvocationSafeFailureProof,
    ) -> Self {
        Self {
            success,
            failure,
            safe_failure_proof,
        }
    }
}

impl OwnedProviderInvocationAdmission {
    pub(crate) const fn result_signing_authority(&self) -> &SigningAuthorityBindingV1 {
        &self.result_signing_authority
    }

    pub(crate) fn borrowed(&self) -> ProviderInvocationAdmissionContext<'_> {
        ProviderInvocationAdmissionContext::new(
            CapabilitySuccessAdmissionContextV1 {
                result: CapabilityResultAdmissionContextV1 {
                    expected_schema_sha256: &self.result_schema_sha256,
                    expected_request_binding: &self.request_binding,
                    expected_attempt_id: &self.attempt_id,
                    expected_implementation_kind: &self.implementation_kind,
                    expected_implementation_version: &self.implementation_version,
                    signing_authority: &self.result_signing_authority,
                },
                contract_set_ref: &self.contract_set_ref,
                contract_set: &self.contract_set,
            },
            AdapterFailureAdmissionContextV1 {
                expected_schema_sha256: &self.failure_schema_sha256,
                expected_request_binding: &self.request_binding,
                expected_attempt_id: &self.attempt_id,
                expected_implementation_kind: &self.implementation_kind,
                expected_implementation_version: &self.implementation_version,
                signing_authority: &self.failure_signing_authority,
            },
            self.safe_failure_proof,
        )
    }
}

/// Why an ordinary provider invocation attempt was rejected before or after I/O.
#[derive(Debug, thiserror::Error)]
pub enum ProviderInvocationAdmissionError {
    /// This effect class requires separately admitted dispatch intent.
    #[error("provider invocation effect {effect:?} requires dispatch-intent authorization")]
    DispatchIntentRequired {
        /// Effect class excluded from the ordinary provider invocation path.
        effect: EffectClass,
    },
    /// The supplied admission context is not for the prepared request.
    #[error("provider invocation admission context differs from the prepared attempt")]
    AdmissionContextMismatch,
    /// A provider invocation request or evidence coordinate was internally inconsistent.
    #[error(transparent)]
    InvalidCoordinate(#[from] ProviderInvocationCoordinateError),
    /// A signed initial-v1 wire record failed contextual admission.
    #[error(transparent)]
    InvalidEvidence(#[from] InvalidResidentRecordV1),
}

/// Admitted signed success retaining its exact provider invocation proof coordinate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedProviderInvocationSuccess {
    coordinate: ProviderInvocationObservationCoordinate,
    result: CapabilityResultV1,
}

impl AdmittedProviderInvocationSuccess {
    /// Borrow the exact binding, generation, implementation, and evidence join.
    #[must_use]
    pub const fn coordinate(&self) -> &ProviderInvocationObservationCoordinate {
        &self.coordinate
    }

    /// Borrow the contextually admitted signed result.
    #[must_use]
    pub const fn result(&self) -> &CapabilityResultV1 {
        &self.result
    }
}

/// One admitted safe failure and its corresponding terminal failed result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedProviderInvocationFailure {
    coordinate: ProviderInvocationObservationCoordinate,
    failure_ref: ResidentRecordRefV1,
    failure: AdapterFailureV1,
    result: CapabilityResultV1,
}

impl AdmittedProviderInvocationFailure {
    /// Borrow the exact binding, generation, implementation, and evidence join.
    #[must_use]
    pub const fn coordinate(&self) -> &ProviderInvocationObservationCoordinate {
        &self.coordinate
    }

    /// Borrow the independently admitted safe failure.
    #[must_use]
    pub const fn failure(&self) -> &AdapterFailureV1 {
        &self.failure
    }

    /// Borrow the immutable reference carried by the failed result.
    #[must_use]
    pub const fn failure_ref(&self) -> &ResidentRecordRefV1 {
        &self.failure_ref
    }

    /// Borrow the failed result that references [`Self::failure`].
    #[must_use]
    pub const fn result(&self) -> &CapabilityResultV1 {
        &self.result
    }
}

/// Operation-bound unavailable outcome carrying no invented positive frontier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationUnavailableAttempt {
    generation: CompiledGenerationDigest,
    instance: InstanceDigest,
    operation: PackageResourceRefV1,
    capability_id: CapabilityIdV1,
    provider_binding_identity: ProviderBindingIdentityV1,
    reason: ProviderInvocationUnavailability,
}

impl ProviderInvocationUnavailableAttempt {
    fn capture(
        request: ProviderInvocationRequest<'_>,
        reason: ProviderInvocationUnavailability,
    ) -> Self {
        Self {
            generation: request.compiled_generation_digest().clone(),
            instance: request.instance_digest().clone(),
            operation: request.operation().clone(),
            capability_id: request.capability_id().clone(),
            provider_binding_identity: request.provider_binding().binding_identity.clone(),
            reason,
        }
    }

    /// Borrow the exact compiled generation of the failed attempt.
    #[must_use]
    pub const fn generation(&self) -> &CompiledGenerationDigest {
        &self.generation
    }

    /// Borrow the exact machine-resolved profile instance.
    #[must_use]
    pub const fn instance(&self) -> &InstanceDigest {
        &self.instance
    }

    /// Borrow the exact operation whose provider attempt was unavailable.
    #[must_use]
    pub const fn operation(&self) -> &PackageResourceRefV1 {
        &self.operation
    }

    /// Borrow the exact unavailable capability.
    #[must_use]
    pub const fn capability_id(&self) -> &CapabilityIdV1 {
        &self.capability_id
    }

    /// Borrow the canonical provider-binding identity selected for the attempt.
    #[must_use]
    pub const fn provider_binding_identity(&self) -> &ProviderBindingIdentityV1 {
        &self.provider_binding_identity
    }

    /// Return the exact unavailability branch.
    #[must_use]
    pub const fn reason(&self) -> ProviderInvocationUnavailability {
        self.reason
    }
}

/// Contextually admitted outcome of one provider invocation attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmittedProviderInvocationOutcome {
    /// One signed success passed exact schema, authority, and coordinate checks.
    Succeeded(Box<AdmittedProviderInvocationSuccess>),
    /// One signed safe failure and its failed result passed contextual checks.
    Failed(Box<AdmittedProviderInvocationFailure>),
    /// No signed terminal wire could presently be obtained.
    Unavailable(ProviderInvocationUnavailableAttempt),
}

/// Effect-screened provider invocation request ready for one adapter invocation.
///
/// Construction excludes local execution and local or remote mutation because
/// the ordinary provider invocation boundary carries no dispatch-intent authorization.
pub struct PreparedProviderInvocation<'a> {
    request: ProviderInvocationRequest<'a>,
}

/// One effect-screened request bound to its independently selected admission context.
///
/// Construction performs every check that can reject the request before I/O.
/// The value is linear: invoking it consumes the only ordinary call authority.
pub struct AdmittedProviderInvocation<'request, 'admission> {
    request: ProviderInvocationRequest<'request>,
    context: ProviderInvocationAdmissionContext<'admission>,
}

impl<'a> PreparedProviderInvocation<'a> {
    /// Admit a pure or read-only request to the ordinary provider invocation path.
    ///
    /// [`EffectClass::Network`] is a remote read in the canonical effect
    /// algebra. [`EffectClass::Execute`], [`EffectClass::Write`], and
    /// [`EffectClass::Publish`] require separately admitted dispatch
    /// intent and are therefore rejected here.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationAdmissionError::DispatchIntentRequired`] for an
    /// effectful request that this boundary is not authorized to dispatch.
    pub fn prepare(
        request: ProviderInvocationRequest<'a>,
        effect: EffectClass,
    ) -> Result<Self, ProviderInvocationAdmissionError> {
        match effect {
            EffectClass::Compute | EffectClass::Read | EffectClass::Network => Ok(Self { request }),
            EffectClass::Execute | EffectClass::Write | EffectClass::Publish => {
                Err(ProviderInvocationAdmissionError::DispatchIntentRequired { effect })
            }
        }
    }

    /// Bind independently selected result and failure authority before provider I/O.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationAdmissionError::AdmissionContextMismatch`] when
    /// the context selects another request, binding, attempt, implementation,
    /// or contract set. A rejected binding does not produce an invocable value.
    pub fn bind<'admission>(
        self,
        context: ProviderInvocationAdmissionContext<'admission>,
    ) -> Result<AdmittedProviderInvocation<'a, 'admission>, ProviderInvocationAdmissionError> {
        context.validate_for(self.request)?;
        Ok(AdmittedProviderInvocation {
            request: self.request,
            context,
        })
    }
}

impl AdmittedProviderInvocation<'_, '_> {
    /// Invoke one provider port and contextually admit its untrusted wire evidence.
    ///
    /// The complete echoed coordinate is compared before either signed record
    /// is admitted. Transport success alone therefore cannot substitute another
    /// request, binding, attempt, generation, implementation, or evidence
    /// frontier.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationAdmissionError`] when the context or echoed
    /// coordinate differs, or when signed wire admission fails.
    pub fn invoke<P: ProviderInvocationPort>(
        self,
        port: &P,
        admission: &dyn ProviderInvocationEvidenceAdmission,
    ) -> Result<AdmittedProviderInvocationOutcome, ProviderInvocationAdmissionError> {
        let observation = port.invoke(self.request);
        self.admit_observation(observation, admission)
    }

    /// Admit one untrusted observation returned by the mechanical provider call.
    ///
    /// This split lets the resident execute a synchronous provider port away
    /// from its single-writer event path. It does not create another release
    /// authority: callers must already own this linear admitted invocation.
    pub(crate) fn admit_observation(
        self,
        observation: ProviderInvocationObservation,
        admission: &dyn ProviderInvocationEvidenceAdmission,
    ) -> Result<AdmittedProviderInvocationOutcome, ProviderInvocationAdmissionError> {
        let context = self.context;
        match observation {
            ProviderInvocationObservation::Succeeded(wire) => {
                wire.coordinate().validate_for(self.request)?;
                admission.admit_evidence_frontier(wire.coordinate().evidence_frontier())?;
                let coordinate = wire.coordinate().clone();
                let admitted = CapabilityResultV1::admit_success_wire(
                    wire.into_result(),
                    &context.success,
                    |reference| admission.resolve_immutable(reference),
                    |schema_id, schema_sha256, bytes| {
                        admission.admit_payload(schema_id, schema_sha256, bytes)
                    },
                    |domain, preimage, digest, signature| {
                        admission.verify_result_signature(domain, preimage, digest, signature)
                    },
                )?;
                Ok(AdmittedProviderInvocationOutcome::Succeeded(Box::new(
                    AdmittedProviderInvocationSuccess {
                        coordinate,
                        result: admitted,
                    },
                )))
            }
            ProviderInvocationObservation::Failed(wire) => {
                wire.coordinate().validate_for(self.request)?;
                admission.admit_evidence_frontier(wire.coordinate().evidence_frontier())?;
                let coordinate = wire.coordinate().clone();
                let (failure_ref, failure_wire, result_wire) = wire.into_parts();
                let admitted_failure = match context.safe_failure_proof {
                    ProviderInvocationSafeFailureProof::BeforeDispatch => {
                        AdapterFailureV1::admit_before_dispatch_wire(
                            failure_wire,
                            &context.failure,
                            |class, retry_at, request, attempt| {
                                admission.admit_failure_class(class, retry_at, request, attempt)
                            },
                            |domain, preimage, digest, signature| {
                                admission
                                    .verify_failure_signature(domain, preimage, digest, signature)
                            },
                        )?
                    }
                    ProviderInvocationSafeFailureProof::IndependentlyProvenNotApplied => {
                        AdapterFailureV1::admit_proven_not_applied_wire(
                            failure_wire,
                            &context.failure,
                            |reference| admission.resolve_immutable(reference),
                            |reference, digest, bytes, request, attempt| {
                                admission.admit_absence_evidence(
                                    reference, digest, bytes, request, attempt,
                                )
                            },
                            |class, retry_at, request, attempt| {
                                admission.admit_failure_class(class, retry_at, request, attempt)
                            },
                            |domain, preimage, digest, signature| {
                                admission
                                    .verify_failure_signature(domain, preimage, digest, signature)
                            },
                        )?
                    }
                };
                let result_context = context.success.result;
                let resolved_failure = admitted_failure.clone();
                let admitted_result = CapabilityResultV1::admit_failure_wire(
                    result_wire,
                    &result_context,
                    |reference| (reference == &failure_ref).then_some(resolved_failure),
                    |domain, preimage, digest, signature| {
                        admission.verify_result_signature(domain, preimage, digest, signature)
                    },
                )?;
                Ok(AdmittedProviderInvocationOutcome::Failed(Box::new(
                    AdmittedProviderInvocationFailure {
                        coordinate,
                        failure_ref,
                        failure: admitted_failure,
                        result: admitted_result,
                    },
                )))
            }
            ProviderInvocationObservation::Unavailable(reason) => {
                Ok(AdmittedProviderInvocationOutcome::Unavailable(
                    ProviderInvocationUnavailableAttempt::capture(self.request, reason),
                ))
            }
        }
    }
}

fn validate_context(
    request: ProviderInvocationRequest<'_>,
    context: ProviderInvocationAdmissionContext<'_>,
) -> Result<(), ProviderInvocationAdmissionError> {
    let result = context.success.result;
    let failure = context.failure;
    let matches = result.expected_request_binding == request.request_binding()
        && failure.expected_request_binding == request.request_binding()
        && result.expected_attempt_id == request.attempt_id()
        && failure.expected_attempt_id == request.attempt_id()
        && result.expected_implementation_kind == request.implementation_kind()
        && failure.expected_implementation_kind == request.implementation_kind()
        && result.expected_implementation_version == request.implementation_version()
        && failure.expected_implementation_version == request.implementation_version()
        && context.success.contract_set_ref == request.request_binding().contract_set_ref();
    if matches {
        Ok(())
    } else {
        Err(ProviderInvocationAdmissionError::AdmissionContextMismatch)
    }
}
