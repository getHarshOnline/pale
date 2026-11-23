//! Provider-neutral transport of one exact capability attempt.
//!
//! [`ProviderInvocationPort`] receives an application-prepared request and returns
//! either untrusted owner wire evidence or one closed unavailability state. The
//! port never admits a [`CapabilityResultWireV1`] or [`AdapterFailureWireV1`],
//! infers whole-provider readiness, or turns transport success into native
//! effect proof.
//!
//! The port is the single owner-neutral request/observation spine for every
//! adapter class whose admitted contract is a [`ProviderBindingV1`]. A native
//! application, responsibility provider, and replaceable Pale capability may
//! have different class contracts without acquiring parallel invocation traits.
//! Their adapters still own native transactions and read-back; this crate owns
//! only the non-substitutable Pale coordinates crossing the boundary.

use pale_core::records::{
    AdapterFailureWireV1, AdapterPluginRefV1, ArtifactDigest, BoundedResidentTextV1,
    CapabilityAttemptIdV1, CapabilityIdV1, CapabilityResultWireV1, CompiledGenerationDigest,
    InstanceDigest, LogicalProviderInstanceRefV1, PackageResourceRefV1,
    ProviderArtifactCoordinatesV1, ProviderBindingIdentityV1, ProviderBindingV1,
    ProviderGenerationCoordinatesV1, ProviderProtocolCoordinatesV1, RequestBindingV1,
    ResidentRecordRefV1, UnixTimeV1,
};

/// One exact implementation selected by an admitted provider binding.
///
/// The coordinate deliberately retains the complete artifact, protocol,
/// adapter, and provider-generation axes. Comparing only an executable or
/// version string would permit source, configuration, or admission-generation
/// substitution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationImplementationCoordinate {
    adapter_plugin: AdapterPluginRefV1,
    artifacts: ProviderArtifactCoordinatesV1,
    protocol: ProviderProtocolCoordinatesV1,
    provider_generations: ProviderGenerationCoordinatesV1,
}

impl ProviderInvocationImplementationCoordinate {
    /// Derive the implementation coordinate from its canonical provider binding.
    #[must_use]
    pub fn from_binding(binding: &ProviderBindingV1) -> Self {
        Self {
            adapter_plugin: binding.adapter_plugin.clone(),
            artifacts: binding.artifact_coordinates.clone(),
            protocol: binding.protocol_coordinates.clone(),
            provider_generations: binding.generation_coordinates.clone(),
        }
    }

    /// Borrow the selected adapter or plugin identity.
    #[must_use]
    pub const fn adapter_plugin(&self) -> &AdapterPluginRefV1 {
        &self.adapter_plugin
    }

    /// Borrow the exact source, build, package, image, and executable coordinates.
    #[must_use]
    pub const fn artifacts(&self) -> &ProviderArtifactCoordinatesV1 {
        &self.artifacts
    }

    /// Borrow the exact protocol, configuration, and extension coordinates.
    #[must_use]
    pub const fn protocol(&self) -> &ProviderProtocolCoordinatesV1 {
        &self.protocol
    }

    /// Borrow the independently typed provider generations.
    #[must_use]
    pub const fn provider_generations(&self) -> &ProviderGenerationCoordinatesV1 {
        &self.provider_generations
    }
}

/// Immutable evidence frontier qualifying one provider invocation observation.
///
/// The reference and digest identify owner evidence without copying its body.
/// Freshness is a separate interval and does not follow from record presence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationEvidenceFrontier {
    subject: ProviderInvocationEvidenceSubject,
    evidence_ref: ResidentRecordRefV1,
    evidence_sha256: ArtifactDigest,
    observed_at: UnixTimeV1,
    valid_until: UnixTimeV1,
}

impl ProviderInvocationEvidenceFrontier {
    /// Construct a non-inverted evidence interval.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationCoordinateError::InvalidEvidenceInterval`] when
    /// `valid_until` precedes `observed_at`.
    pub fn new(
        subject: ProviderInvocationEvidenceSubject,
        evidence_ref: ResidentRecordRefV1,
        evidence_sha256: ArtifactDigest,
        observed_at: UnixTimeV1,
        valid_until: UnixTimeV1,
    ) -> Result<Self, ProviderInvocationCoordinateError> {
        if valid_until < observed_at {
            return Err(ProviderInvocationCoordinateError::InvalidEvidenceInterval);
        }
        Ok(Self {
            subject,
            evidence_ref,
            evidence_sha256,
            observed_at,
            valid_until,
        })
    }

    /// Borrow the exact binding, operation, and capability evidenced.
    #[must_use]
    pub const fn subject(&self) -> &ProviderInvocationEvidenceSubject {
        &self.subject
    }

    /// Borrow the immutable owner-evidence reference.
    #[must_use]
    pub const fn evidence_ref(&self) -> &ResidentRecordRefV1 {
        &self.evidence_ref
    }

    /// Borrow the digest of the exact owner-evidence bytes.
    #[must_use]
    pub const fn evidence_sha256(&self) -> &ArtifactDigest {
        &self.evidence_sha256
    }

    /// Return the observation time.
    #[must_use]
    pub const fn observed_at(&self) -> UnixTimeV1 {
        self.observed_at
    }

    /// Return the last time at which the observation remains fresh.
    #[must_use]
    pub const fn valid_until(&self) -> UnixTimeV1 {
        self.valid_until
    }
}

/// A provider invocation coordinate is malformed before any adapter call.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProviderInvocationCoordinateError {
    /// The provider-binding body no longer reproduces its sealed identity.
    #[error("provider invocation request carries an invalid provider-binding identity")]
    InvalidProviderBindingIdentity,
    /// An evidence frontier expires before its observation time.
    #[error("provider invocation evidence interval is inverted")]
    InvalidEvidenceInterval,
    /// The evidence frontier names another binding, operation, or capability.
    #[error("provider invocation evidence subject differs from its invocation")]
    EvidenceSubjectMismatch,
    /// An observation echoes a different request, binding, attempt, generation, or implementation.
    #[error("provider invocation observation coordinate differs from its invocation")]
    ObservationRequestMismatch,
}

/// Exact admitted invocation lineage before selecting a provider implementation.
///
/// The capability is owned by [`RequestBindingV1::capability_id`], so this
/// coordinate does not accept a second copy that could disagree. Provider
/// selection and result implementation remain separate axes on
/// [`ProviderInvocationRequest`].
#[derive(Clone, Copy)]
pub struct ProviderInvocationCoordinate<'a> {
    compiled_generation_digest: &'a CompiledGenerationDigest,
    instance_digest: &'a InstanceDigest,
    operation: &'a PackageResourceRefV1,
    request_binding: &'a RequestBindingV1,
    attempt_id: &'a CapabilityAttemptIdV1,
}

impl<'a> ProviderInvocationCoordinate<'a> {
    /// Capture the immutable application-admitted lineage for one attempt.
    #[must_use]
    pub const fn new(
        compiled_generation_digest: &'a CompiledGenerationDigest,
        instance_digest: &'a InstanceDigest,
        operation: &'a PackageResourceRefV1,
        request_binding: &'a RequestBindingV1,
        attempt_id: &'a CapabilityAttemptIdV1,
    ) -> Self {
        Self {
            compiled_generation_digest,
            instance_digest,
            operation,
            request_binding,
            attempt_id,
        }
    }
}

/// Borrowed admitted coordinates for one provider invocation attempt.
///
/// This is the minimum adapter-facing projection of admitted application state.
/// It carries the complete provider binding and full request lineage, while
/// omitting session, compiler, and path bodies that the adapter does not need.
/// Construction grants no provider authority and permits no external effect.
#[derive(Clone, Copy)]
pub struct ProviderInvocationRequest<'a> {
    compiled_generation_digest: &'a CompiledGenerationDigest,
    instance_digest: &'a InstanceDigest,
    operation: &'a PackageResourceRefV1,
    provider_binding: &'a ProviderBindingV1,
    request_binding: &'a RequestBindingV1,
    attempt_id: &'a CapabilityAttemptIdV1,
    implementation_kind: &'a BoundedResidentTextV1,
    implementation_version: &'a BoundedResidentTextV1,
}

impl<'a> ProviderInvocationRequest<'a> {
    /// Bind one admitted attempt to its exact generation and provider selection.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationCoordinateError`] when the provider-binding
    /// body does not reproduce its sealed identity.
    pub fn new(
        invocation: ProviderInvocationCoordinate<'a>,
        provider_binding: &'a ProviderBindingV1,
        implementation_kind: &'a BoundedResidentTextV1,
        implementation_version: &'a BoundedResidentTextV1,
    ) -> Result<Self, ProviderInvocationCoordinateError> {
        if provider_binding.recompute_identity().ok().as_ref()
            != Some(&provider_binding.binding_identity)
        {
            return Err(ProviderInvocationCoordinateError::InvalidProviderBindingIdentity);
        }
        Ok(Self {
            compiled_generation_digest: invocation.compiled_generation_digest,
            instance_digest: invocation.instance_digest,
            operation: invocation.operation,
            provider_binding,
            request_binding: invocation.request_binding,
            attempt_id: invocation.attempt_id,
            implementation_kind,
            implementation_version,
        })
    }

    /// Borrow the immutable compiled-generation identity.
    #[must_use]
    pub const fn compiled_generation_digest(&self) -> &CompiledGenerationDigest {
        self.compiled_generation_digest
    }

    /// Borrow the machine-resolved instance identity.
    #[must_use]
    pub const fn instance_digest(&self) -> &InstanceDigest {
        self.instance_digest
    }

    /// Borrow the exact Package-owned operation identity.
    #[must_use]
    pub const fn operation(&self) -> &PackageResourceRefV1 {
        self.operation
    }

    /// Borrow the exact capability selected for this attempt.
    #[must_use]
    pub const fn capability_id(&self) -> &CapabilityIdV1 {
        self.request_binding.capability_id()
    }

    /// Borrow the complete admitted provider binding.
    #[must_use]
    pub const fn provider_binding(&self) -> &ProviderBindingV1 {
        self.provider_binding
    }

    /// Borrow the full admitted caller-request lineage.
    #[must_use]
    pub const fn request_binding(&self) -> &RequestBindingV1 {
        self.request_binding
    }

    /// Borrow the distinct transport-attempt identity.
    #[must_use]
    pub const fn attempt_id(&self) -> &CapabilityAttemptIdV1 {
        self.attempt_id
    }

    /// Borrow the selected result implementation class.
    #[must_use]
    pub const fn implementation_kind(&self) -> &BoundedResidentTextV1 {
        self.implementation_kind
    }

    /// Borrow the selected result implementation version.
    #[must_use]
    pub const fn implementation_version(&self) -> &BoundedResidentTextV1 {
        self.implementation_version
    }
}

/// Exact subject whose provider invocation evidence may qualify one invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationEvidenceSubject {
    provider_binding_identity: ProviderBindingIdentityV1,
    operation: PackageResourceRefV1,
    capability_id: CapabilityIdV1,
}

impl ProviderInvocationEvidenceSubject {
    /// Construct an explicit subject for independently resolved owner evidence.
    #[must_use]
    pub const fn from_parts(
        provider_binding_identity: ProviderBindingIdentityV1,
        operation: PackageResourceRefV1,
        capability_id: CapabilityIdV1,
    ) -> Self {
        Self {
            provider_binding_identity,
            operation,
            capability_id,
        }
    }

    /// Derive the only evidence subject accepted for an exact provider request.
    #[must_use]
    pub fn from_request(request: ProviderInvocationRequest<'_>) -> Self {
        Self::from_parts(
            request.provider_binding().binding_identity.clone(),
            request.operation().clone(),
            request.capability_id().clone(),
        )
    }

    /// Borrow the canonical provider-binding identity.
    #[must_use]
    pub const fn provider_binding_identity(&self) -> &ProviderBindingIdentityV1 {
        &self.provider_binding_identity
    }

    /// Borrow the exact semantic operation identity.
    #[must_use]
    pub const fn operation(&self) -> &PackageResourceRefV1 {
        &self.operation
    }

    /// Borrow the exact capability identity.
    #[must_use]
    pub const fn capability_id(&self) -> &CapabilityIdV1 {
        &self.capability_id
    }
}

/// Exact coordinate echoed beside untrusted provider wire evidence.
///
/// An adapter may return a response retained from another call. The
/// application therefore compares this complete coordinate before admitting
/// either signed wire record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationObservationCoordinate {
    compiled_generation_digest: CompiledGenerationDigest,
    instance_digest: InstanceDigest,
    operation: PackageResourceRefV1,
    capability_id: CapabilityIdV1,
    provider_binding_identity: ProviderBindingIdentityV1,
    logical_provider_instance: LogicalProviderInstanceRefV1,
    request_binding: RequestBindingV1,
    attempt_id: CapabilityAttemptIdV1,
    implementation_kind: BoundedResidentTextV1,
    implementation_version: BoundedResidentTextV1,
    implementation: ProviderInvocationImplementationCoordinate,
    evidence_frontier: ProviderInvocationEvidenceFrontier,
}

impl ProviderInvocationObservationCoordinate {
    /// Capture one complete observation echo for a request and evidence frontier.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationCoordinateError::EvidenceSubjectMismatch`]
    /// when the frontier describes another binding, operation, or capability.
    pub fn capture(
        request: ProviderInvocationRequest<'_>,
        evidence_frontier: ProviderInvocationEvidenceFrontier,
    ) -> Result<Self, ProviderInvocationCoordinateError> {
        if evidence_frontier.subject() != &ProviderInvocationEvidenceSubject::from_request(request)
        {
            return Err(ProviderInvocationCoordinateError::EvidenceSubjectMismatch);
        }
        Ok(Self {
            compiled_generation_digest: request.compiled_generation_digest.clone(),
            instance_digest: request.instance_digest.clone(),
            operation: request.operation.clone(),
            capability_id: request.capability_id().clone(),
            provider_binding_identity: request.provider_binding.binding_identity.clone(),
            logical_provider_instance: request.provider_binding.logical_provider_instance.clone(),
            request_binding: request.request_binding.clone(),
            attempt_id: request.attempt_id.clone(),
            implementation_kind: request.implementation_kind.clone(),
            implementation_version: request.implementation_version.clone(),
            implementation: ProviderInvocationImplementationCoordinate::from_binding(
                request.provider_binding,
            ),
            evidence_frontier,
        })
    }

    /// Verify that this observation echoes one exact prepared invocation.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationCoordinateError`] when any request, provider,
    /// generation, implementation, or evidence-subject axis differs.
    pub fn validate_for(
        &self,
        request: ProviderInvocationRequest<'_>,
    ) -> Result<(), ProviderInvocationCoordinateError> {
        if self.compiled_generation_digest != *request.compiled_generation_digest()
            || self.instance_digest != *request.instance_digest()
            || self.operation != *request.operation()
            || self.capability_id != *request.capability_id()
            || self.provider_binding_identity != request.provider_binding().binding_identity
            || self.logical_provider_instance
                != request.provider_binding().logical_provider_instance
            || self.request_binding != *request.request_binding()
            || self.attempt_id != *request.attempt_id()
            || self.implementation_kind != *request.implementation_kind()
            || self.implementation_version != *request.implementation_version()
            || self.implementation
                != ProviderInvocationImplementationCoordinate::from_binding(
                    request.provider_binding(),
                )
        {
            return Err(ProviderInvocationCoordinateError::ObservationRequestMismatch);
        }
        if self.evidence_frontier.subject()
            != &ProviderInvocationEvidenceSubject::from_request(request)
        {
            return Err(ProviderInvocationCoordinateError::EvidenceSubjectMismatch);
        }
        Ok(())
    }

    /// Borrow the immutable compiled-generation identity.
    #[must_use]
    pub const fn compiled_generation_digest(&self) -> &CompiledGenerationDigest {
        &self.compiled_generation_digest
    }

    /// Borrow the machine-resolved instance identity.
    #[must_use]
    pub const fn instance_digest(&self) -> &InstanceDigest {
        &self.instance_digest
    }

    /// Borrow the exact operation identity.
    #[must_use]
    pub const fn operation(&self) -> &PackageResourceRefV1 {
        &self.operation
    }

    /// Borrow the exact capability identity.
    #[must_use]
    pub const fn capability_id(&self) -> &CapabilityIdV1 {
        &self.capability_id
    }

    /// Borrow the canonical provider-binding identity.
    #[must_use]
    pub const fn provider_binding_identity(&self) -> &ProviderBindingIdentityV1 {
        &self.provider_binding_identity
    }

    /// Borrow the provider-native logical instance coordinate.
    #[must_use]
    pub const fn logical_provider_instance(&self) -> &LogicalProviderInstanceRefV1 {
        &self.logical_provider_instance
    }

    /// Borrow the complete admitted request lineage.
    #[must_use]
    pub const fn request_binding(&self) -> &RequestBindingV1 {
        &self.request_binding
    }

    /// Borrow the distinct transport-attempt identity.
    #[must_use]
    pub const fn attempt_id(&self) -> &CapabilityAttemptIdV1 {
        &self.attempt_id
    }

    /// Borrow the selected result implementation class.
    #[must_use]
    pub const fn implementation_kind(&self) -> &BoundedResidentTextV1 {
        &self.implementation_kind
    }

    /// Borrow the selected result implementation version.
    #[must_use]
    pub const fn implementation_version(&self) -> &BoundedResidentTextV1 {
        &self.implementation_version
    }

    /// Borrow the complete selected implementation coordinate.
    #[must_use]
    pub const fn implementation(&self) -> &ProviderInvocationImplementationCoordinate {
        &self.implementation
    }

    /// Borrow the owner-evidence frontier qualifying the observation.
    #[must_use]
    pub const fn evidence_frontier(&self) -> &ProviderInvocationEvidenceFrontier {
        &self.evidence_frontier
    }
}

/// Untrusted successful result paired with its complete coordinate echo.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationSuccessWire {
    coordinate: ProviderInvocationObservationCoordinate,
    result: CapabilityResultWireV1,
}

impl ProviderInvocationSuccessWire {
    /// Pair untrusted result bytes with the coordinate observed by the adapter.
    #[must_use]
    pub const fn new(
        coordinate: ProviderInvocationObservationCoordinate,
        result: CapabilityResultWireV1,
    ) -> Self {
        Self { coordinate, result }
    }

    /// Borrow the adapter's claimed complete observation coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &ProviderInvocationObservationCoordinate {
        &self.coordinate
    }

    /// Consume the untrusted signed capability-result wire.
    #[must_use]
    pub fn into_result(self) -> CapabilityResultWireV1 {
        self.result
    }
}

/// Untrusted safe-failure records paired with their exact immutable reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationFailureWire {
    coordinate: ProviderInvocationObservationCoordinate,
    failure_ref: ResidentRecordRefV1,
    failure: AdapterFailureWireV1,
    result: CapabilityResultWireV1,
}

impl ProviderInvocationFailureWire {
    /// Pair one safe failure and its failed result with the observed coordinate.
    #[must_use]
    pub const fn new(
        coordinate: ProviderInvocationObservationCoordinate,
        failure_ref: ResidentRecordRefV1,
        failure: AdapterFailureWireV1,
        result: CapabilityResultWireV1,
    ) -> Self {
        Self {
            coordinate,
            failure_ref,
            failure,
            result,
        }
    }

    /// Borrow the adapter's claimed complete observation coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &ProviderInvocationObservationCoordinate {
        &self.coordinate
    }

    /// Consume the immutable reference and both untrusted signed wires.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        ResidentRecordRefV1,
        AdapterFailureWireV1,
        CapabilityResultWireV1,
    ) {
        (self.failure_ref, self.failure, self.result)
    }
}

/// Why the exact selected provider attempt cannot currently proceed.
///
/// These states remain distinct from
/// [`pale_core::records::AdapterFailureClassV1::Unavailable`]. Application
/// policy may project an observation into safe failure evidence, but this port
/// cannot manufacture a signature or collapse missing owner law, native health,
/// reachability, removal, and evidence freshness.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProviderInvocationUnavailability {
    /// The native owner has not published an admissible execution contract.
    OwnerContractUnavailable,
    /// Current owner-native evidence reports the selected provider as down.
    Down,
    /// The observation path cannot reach the selected provider or evidence owner.
    Partitioned,
    /// Admitted owner evidence says the exact provider binding was removed.
    Removed,
    /// No sufficiently fresh owner evidence proves a more specific state.
    EvidenceUnknown,
}

/// Closed provider response before application-owned contextual admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderInvocationObservation {
    /// One untrusted successful capability-result wire was returned.
    Succeeded(Box<ProviderInvocationSuccessWire>),
    /// One untrusted signed safe failure and failed result were returned.
    Failed(Box<ProviderInvocationFailureWire>),
    /// No provider call can presently produce signed terminal evidence.
    Unavailable(ProviderInvocationUnavailability),
}

/// Provider-neutral invocation boundary for one exact capability attempt.
///
/// Implementations may translate the request to an owner-native contract, but
/// must return untrusted initial-v1 wires. Only the application may compare the
/// complete coordinate and admit those wires. This port does not authorize an
/// external effect, activate a provider, or infer readiness for another
/// operation. Adapter-class membership is independent conformance metadata; it
/// neither changes this request algebra nor grants native-owner authority.
pub trait ProviderInvocationPort {
    /// Invoke the exact application-prepared capability attempt.
    fn invoke(&self, request: ProviderInvocationRequest<'_>) -> ProviderInvocationObservation;
}

#[cfg(test)]
#[path = "../tests/internal/provider_invocation.rs"]
mod tests;
