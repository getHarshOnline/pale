//! Operation-scoped provider readiness over independently typed dimensions.

use std::collections::BTreeSet;

use pale_core::records::{
    ArtifactDigest, CapabilityIdV1, CapabilityRequirementIdV1, CommunicationActorHandleV1,
    CompiledGenerationDigest, EffectClass, InstanceDigest, LogicalProviderInstanceRefV1,
    PackageResourceRefV1, ProviderAuthorityCoordinatesV1, ProviderBindingIdentityV1,
    ProviderBindingV1, ResidentRecordRefV1, UnixTimeV1,
};
use pale_ports::{
    ProviderInvocationEvidenceFrontier, ProviderInvocationEvidenceSubject,
    ProviderInvocationImplementationCoordinate,
};

use crate::{
    compile::{
        EffectiveProfileNarrowingReason, EffectiveProfilePolicyBlock,
        EffectiveProfileRequirementDisposition,
    },
    observation::ObservationReadinessState,
    provider_invocation::{
        AdmittedProviderInvocationOutcome, AdmittedProviderInvocationSuccess,
        ProviderInvocationUnavailableAttempt,
    },
};

/// Conservative state of one independently evidenced readiness dimension.
///
/// [`Self::Stale`] retains the exact last-known coordinate for explanation but
/// cannot satisfy readiness. No branch is a provider-wide health Boolean.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderInvocationComponent<T> {
    /// Current admitted evidence satisfies this exact dimension.
    Satisfied(T),
    /// An admitted authority or policy ceiling rejects this dimension.
    Blocked,
    /// The selected component or evidence carrier does not exist.
    Unavailable,
    /// No qualifying current evidence establishes a disposition.
    Unknown,
    /// The retained coordinate exists but its evidence is no longer current.
    Stale(T),
}

impl<T> ProviderInvocationComponent<T> {
    /// Project this dimension to the common conservative readiness lattice.
    #[must_use]
    pub const fn state(&self) -> ProviderInvocationReadinessState {
        match self {
            Self::Satisfied(_) => ProviderInvocationReadinessState::Ready,
            Self::Blocked => ProviderInvocationReadinessState::Blocked,
            Self::Unavailable => ProviderInvocationReadinessState::Unavailable,
            Self::Unknown => ProviderInvocationReadinessState::Unknown,
            Self::Stale(_) => ProviderInvocationReadinessState::Stale,
        }
    }
}

impl ProviderInvocationComponent<ProviderInvocationEvidenceFrontier> {
    /// Classify one evidence frontier at an exact checked time.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationReadinessError::EvidenceNotYetObserved`] when
    /// `checked_at` precedes the observation itself.
    pub fn at(
        frontier: ProviderInvocationEvidenceFrontier,
        checked_at: UnixTimeV1,
    ) -> Result<Self, ProviderInvocationReadinessError> {
        if checked_at < frontier.observed_at() {
            return Err(ProviderInvocationReadinessError::EvidenceNotYetObserved);
        }
        if checked_at > frontier.valid_until() {
            Ok(Self::Stale(frontier))
        } else {
            Ok(Self::Satisfied(frontier))
        }
    }
}

/// Conservative result of accumulating independent readiness dimensions.
///
/// Under the information order rendered below, [`Self::meet`] is associative,
/// commutative, and idempotent with [`Self::Ready`] as identity.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/provider-invocation-readiness-meet.display.mathml"
))]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProviderInvocationReadinessState {
    /// Every constitutive dimension has current admitted evidence.
    Ready,
    /// A constitutive dimension has no qualifying evidence.
    Unknown,
    /// A constitutive dimension has only expired evidence.
    Stale,
    /// A required binding, dependency, or carrier is absent.
    Unavailable,
    /// Admitted policy or authority forbids the operation.
    Blocked,
}

impl ProviderInvocationReadinessState {
    /// Accumulate two independently derived readiness states conservatively.
    #[must_use]
    pub const fn meet(self, other: Self) -> Self {
        use ProviderInvocationReadinessState::{Blocked, Ready, Stale, Unavailable, Unknown};
        match (self, other) {
            (Blocked, _) | (_, Blocked) => Blocked,
            (Unavailable, _) | (_, Unavailable) => Unavailable,
            (Stale, _) | (_, Stale) => Stale,
            (Unknown, _) | (_, Unknown) => Unknown,
            (Ready, Ready) => Ready,
        }
    }

    /// Project detailed provider invocation readiness into the coarse observation view.
    ///
    /// Both unknown and stale remain unproven; this projection is intentionally
    /// lossy and must never be used to reconstruct provider invocation state.
    #[must_use]
    pub const fn observation_projection(self) -> ObservationReadinessState {
        match self {
            Self::Ready => ObservationReadinessState::Ready,
            Self::Blocked => ObservationReadinessState::Blocked,
            Self::Unavailable => ObservationReadinessState::Unavailable,
            Self::Unknown | Self::Stale => ObservationReadinessState::Unproven,
        }
    }
}

/// Exact provider binding and machine-resolved instance selected for one operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationBindingCoordinate {
    binding_identity: ProviderBindingIdentityV1,
    logical_instance: LogicalProviderInstanceRefV1,
    instance_digest: InstanceDigest,
}

impl ProviderInvocationBindingCoordinate {
    /// Capture the canonical binding identity and its separately resolved instance.
    #[must_use]
    fn from_binding(binding: &ProviderBindingV1, instance_digest: InstanceDigest) -> Self {
        Self {
            binding_identity: binding.binding_identity.clone(),
            logical_instance: binding.logical_provider_instance.clone(),
            instance_digest,
        }
    }

    /// Borrow the canonical identity of the complete provider binding.
    #[must_use]
    pub const fn binding_identity(&self) -> &ProviderBindingIdentityV1 {
        &self.binding_identity
    }

    /// Borrow the provider-native logical instance.
    #[must_use]
    pub const fn logical_instance(&self) -> &LogicalProviderInstanceRefV1 {
        &self.logical_instance
    }

    /// Borrow the exact machine-resolved private profile instance.
    #[must_use]
    pub const fn instance_digest(&self) -> &InstanceDigest {
        &self.instance_digest
    }
}

/// Actor, native authority contract, and admitted effect ceiling for one capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationAuthorityCoordinate {
    actor: Option<CommunicationActorHandleV1>,
    authority: ProviderAuthorityCoordinatesV1,
    effect_ceiling: BTreeSet<EffectClass>,
}

impl ProviderInvocationAuthorityCoordinate {
    /// Capture independently admitted actor, authority, and effect-ceiling axes.
    fn from_binding(
        binding: &ProviderBindingV1,
        effect_ceiling: impl IntoIterator<Item = EffectClass>,
    ) -> Self {
        Self {
            actor: binding.communication_actor_handle.clone(),
            authority: binding.authority_coordinates.clone(),
            effect_ceiling: effect_ceiling.into_iter().collect(),
        }
    }

    /// Borrow the optional provider-native actor without inferring a person.
    #[must_use]
    pub const fn actor(&self) -> Option<&CommunicationActorHandleV1> {
        self.actor.as_ref()
    }

    /// Borrow the exact native and field-authority contract coordinates.
    #[must_use]
    pub const fn authority(&self) -> &ProviderAuthorityCoordinatesV1 {
        &self.authority
    }

    /// Borrow the canonical set of effects admitted for this capability.
    #[must_use]
    pub const fn effect_ceiling(&self) -> &BTreeSet<EffectClass> {
        &self.effect_ceiling
    }
}

/// One exact constitutive dependency of an operation-capability binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationDependencyCoordinate {
    record_ref: ResidentRecordRefV1,
    record_sha256: ArtifactDigest,
}

impl ProviderInvocationDependencyCoordinate {
    /// Bind an immutable dependency reference to its exact bytes.
    #[must_use]
    pub const fn new(record_ref: ResidentRecordRefV1, record_sha256: ArtifactDigest) -> Self {
        Self {
            record_ref,
            record_sha256,
        }
    }

    /// Borrow the immutable dependency reference.
    #[must_use]
    pub const fn record_ref(&self) -> &ResidentRecordRefV1 {
        &self.record_ref
    }

    /// Borrow the digest of the referenced dependency bytes.
    #[must_use]
    pub const fn record_sha256(&self) -> &ArtifactDigest {
        &self.record_sha256
    }
}

/// Deterministically ordered exact constitutive dependencies.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProviderInvocationDependencySet(Vec<ProviderInvocationDependencyCoordinate>);

impl ProviderInvocationDependencySet {
    /// Sort and deduplicate exact dependency ref/hash pairs.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationReadinessError::DependencyReferenceConflict`]
    /// when one immutable reference is paired with two different byte digests.
    pub fn new(
        mut dependencies: Vec<ProviderInvocationDependencyCoordinate>,
    ) -> Result<Self, ProviderInvocationReadinessError> {
        dependencies.sort_by(|left, right| {
            left.record_ref
                .as_str()
                .cmp(right.record_ref.as_str())
                .then_with(|| {
                    left.record_sha256
                        .as_str()
                        .cmp(right.record_sha256.as_str())
                })
        });
        if dependencies.windows(2).any(|pair| {
            pair[0].record_ref == pair[1].record_ref
                && pair[0].record_sha256 != pair[1].record_sha256
        }) {
            return Err(ProviderInvocationReadinessError::DependencyReferenceConflict);
        }
        dependencies.dedup();
        Ok(Self(dependencies))
    }

    /// Borrow the exact deterministic dependency sequence.
    #[must_use]
    pub fn as_slice(&self) -> &[ProviderInvocationDependencyCoordinate] {
        &self.0
    }
}

/// Invocation dimensions present only for a portably satisfied provider requirement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationBoundCapability {
    binding: ProviderInvocationComponent<ProviderInvocationBindingCoordinate>,
    implementation: ProviderInvocationComponent<ProviderInvocationImplementationCoordinate>,
    authority: ProviderInvocationComponent<ProviderInvocationAuthorityCoordinate>,
    dependencies: ProviderInvocationComponent<ProviderInvocationDependencySet>,
    evidence: ProviderInvocationComponent<ProviderInvocationEvidenceFrontier>,
}

impl ProviderInvocationBoundCapability {
    /// Assemble the five independently evidenced provider invocation dimensions.
    const fn new(
        binding: ProviderInvocationComponent<ProviderInvocationBindingCoordinate>,
        implementation: ProviderInvocationComponent<ProviderInvocationImplementationCoordinate>,
        authority: ProviderInvocationComponent<ProviderInvocationAuthorityCoordinate>,
        dependencies: ProviderInvocationComponent<ProviderInvocationDependencySet>,
        evidence: ProviderInvocationComponent<ProviderInvocationEvidenceFrontier>,
    ) -> Self {
        Self {
            binding,
            implementation,
            authority,
            dependencies,
            evidence,
        }
    }

    /// Conservatively accumulate every constitutive dimension.
    #[must_use]
    pub fn state(&self) -> ProviderInvocationReadinessState {
        [
            self.binding.state(),
            self.implementation.state(),
            self.authority.state(),
            self.dependencies.state(),
            self.evidence.state(),
        ]
        .into_iter()
        .fold(
            ProviderInvocationReadinessState::Ready,
            ProviderInvocationReadinessState::meet,
        )
    }

    /// Borrow the exact provider-binding and profile-instance dimension.
    #[must_use]
    pub const fn binding(
        &self,
    ) -> &ProviderInvocationComponent<ProviderInvocationBindingCoordinate> {
        &self.binding
    }

    /// Borrow the exact source-to-executable and protocol/configuration dimension.
    #[must_use]
    pub const fn implementation(
        &self,
    ) -> &ProviderInvocationComponent<ProviderInvocationImplementationCoordinate> {
        &self.implementation
    }

    /// Borrow the actor, authority, and effect-ceiling dimension.
    #[must_use]
    pub const fn authority(
        &self,
    ) -> &ProviderInvocationComponent<ProviderInvocationAuthorityCoordinate> {
        &self.authority
    }

    /// Borrow the exact constitutive dependency-set dimension.
    #[must_use]
    pub const fn dependencies(
        &self,
    ) -> &ProviderInvocationComponent<ProviderInvocationDependencySet> {
        &self.dependencies
    }

    /// Borrow the exact evidence-frontier and freshness dimension.
    #[must_use]
    pub const fn evidence(
        &self,
    ) -> &ProviderInvocationComponent<ProviderInvocationEvidenceFrontier> {
        &self.evidence
    }
}

/// Portable requirement disposition plus any separately derived invocation dimensions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderInvocationRequirementState {
    /// Portable meaning selected a provider; invocation dimensions remain independent.
    Bound(Box<ProviderInvocationBoundCapability>),
    /// The capability is absent from a positive private ceiling.
    Narrowed(EffectiveProfileNarrowingReason),
    /// No private provider binding exists for the requirement.
    ProviderBindingAbsent,
    /// An explicit private policy dimension rejects the requirement.
    PolicyBlocked(EffectiveProfilePolicyBlock),
    /// The exact bound invocation returned one closed unavailability branch.
    InvocationUnavailable(ProviderInvocationUnavailableAttempt),
}

/// Readiness of one Package-owned capability requirement within one operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationCapabilityReadiness {
    generation: CompiledGenerationDigest,
    operation: PackageResourceRefV1,
    requirement_id: CapabilityRequirementIdV1,
    capability_id: CapabilityIdV1,
    state: ProviderInvocationRequirementState,
}

impl ProviderInvocationCapabilityReadiness {
    /// Derive one requirement row directly from its admitted invocation outcome.
    ///
    /// A safe failed result remains valid failure evidence but cannot establish
    /// positive invocation readiness. Unavailability is retained as its exact
    /// operation-bound branch rather than widened to a provider-wide state.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationReadinessError`] when a positive coordinate is
    /// substituted, its evidence is stale-invalid, its effect lies outside the
    /// ceiling, or the outcome is a failed result rather than readiness proof.
    pub fn from_admitted_outcome(
        requirement_id: CapabilityRequirementIdV1,
        required_effect: EffectClass,
        binding: &ProviderBindingV1,
        effect_ceiling: impl IntoIterator<Item = EffectClass>,
        dependencies: ProviderInvocationComponent<ProviderInvocationDependencySet>,
        outcome: AdmittedProviderInvocationOutcome,
        checked_at: UnixTimeV1,
    ) -> Result<Self, ProviderInvocationReadinessError> {
        match outcome {
            AdmittedProviderInvocationOutcome::Succeeded(observation) => {
                Self::from_admitted_success(
                    requirement_id,
                    required_effect,
                    binding,
                    effect_ceiling,
                    dependencies,
                    &observation,
                    checked_at,
                )
            }
            AdmittedProviderInvocationOutcome::Failed(_) => {
                Err(ProviderInvocationReadinessError::FailedResultIsNotReadiness)
            }
            AdmittedProviderInvocationOutcome::Unavailable(attempt) => {
                Ok(Self::from_unavailable(requirement_id, attempt))
            }
        }
    }

    /// Derive positive readiness from one admitted observation and canonical binding.
    ///
    /// The method replays every join needed for a positive claim: exact binding
    /// identity and logical instance, source/build/executable/configuration/
    /// protocol coordinate, evidence subject, capability, and required effect.
    /// Dependencies may only narrow the resulting state.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationReadinessError`] when any positive coordinate is
    /// substituted or the required effect lies outside the admitted ceiling.
    ///
    pub(super) fn from_admitted_success(
        requirement_id: CapabilityRequirementIdV1,
        required_effect: EffectClass,
        binding: &ProviderBindingV1,
        effect_ceiling: impl IntoIterator<Item = EffectClass>,
        dependencies: ProviderInvocationComponent<ProviderInvocationDependencySet>,
        observation: &AdmittedProviderInvocationSuccess,
        checked_at: UnixTimeV1,
    ) -> Result<Self, ProviderInvocationReadinessError> {
        let coordinate = observation.coordinate();
        let recomputed = binding
            .recompute_identity()
            .map_err(|_| ProviderInvocationReadinessError::InvalidProviderBindingIdentity)?;
        let implementation = ProviderInvocationImplementationCoordinate::from_binding(binding);
        let subject = ProviderInvocationEvidenceSubject::from_parts(
            binding.binding_identity.clone(),
            coordinate.operation().clone(),
            coordinate.capability_id().clone(),
        );
        if recomputed != binding.binding_identity {
            return Err(ProviderInvocationReadinessError::InvalidProviderBindingIdentity);
        }
        if coordinate.provider_binding_identity() != &binding.binding_identity
            || coordinate.logical_provider_instance() != &binding.logical_provider_instance
            || coordinate.implementation() != &implementation
        {
            return Err(ProviderInvocationReadinessError::ProviderCoordinateMismatch);
        }
        if coordinate.evidence_frontier().subject() != &subject {
            return Err(ProviderInvocationReadinessError::EvidenceSubjectMismatch);
        }
        let effect_ceiling = effect_ceiling.into_iter().collect::<BTreeSet<_>>();
        if !effect_ceiling.contains(&required_effect) {
            return Err(ProviderInvocationReadinessError::RequiredEffectOutsideCeiling);
        }
        let evidence =
            ProviderInvocationComponent::at(coordinate.evidence_frontier().clone(), checked_at)?;
        let capability_id = coordinate.capability_id().clone();
        let readiness = ProviderInvocationBoundCapability::new(
            ProviderInvocationComponent::Satisfied(
                ProviderInvocationBindingCoordinate::from_binding(
                    binding,
                    coordinate.instance_digest().clone(),
                ),
            ),
            ProviderInvocationComponent::Satisfied(implementation),
            ProviderInvocationComponent::Satisfied(
                ProviderInvocationAuthorityCoordinate::from_binding(binding, effect_ceiling),
            ),
            dependencies,
            evidence,
        );
        Ok(Self {
            generation: coordinate.compiled_generation_digest().clone(),
            operation: coordinate.operation().clone(),
            requirement_id,
            capability_id,
            state: ProviderInvocationRequirementState::Bound(Box::new(readiness)),
        })
    }

    /// Preserve one non-satisfied portable provider disposition without invocation inference.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationReadinessError::SatisfiedRequirementNeedsInvocationEvidence`]
    /// when called for [`EffectiveProfileRequirementDisposition::Satisfied`].
    pub fn from_portable_disposition(
        generation: CompiledGenerationDigest,
        operation: PackageResourceRefV1,
        requirement_id: CapabilityRequirementIdV1,
        capability_id: CapabilityIdV1,
        disposition: EffectiveProfileRequirementDisposition,
    ) -> Result<Self, ProviderInvocationReadinessError> {
        let state = match disposition {
            EffectiveProfileRequirementDisposition::Satisfied => {
                return Err(
                    ProviderInvocationReadinessError::SatisfiedRequirementNeedsInvocationEvidence,
                );
            }
            EffectiveProfileRequirementDisposition::Narrowed(reason) => {
                ProviderInvocationRequirementState::Narrowed(reason)
            }
            EffectiveProfileRequirementDisposition::ProviderBindingAbsent => {
                ProviderInvocationRequirementState::ProviderBindingAbsent
            }
            EffectiveProfileRequirementDisposition::PolicyBlocked(reason) => {
                ProviderInvocationRequirementState::PolicyBlocked(reason)
            }
        };
        Ok(Self {
            generation,
            operation,
            requirement_id,
            capability_id,
            state,
        })
    }

    /// Preserve one exact operation-bound invocation unavailability outcome.
    #[must_use]
    fn from_unavailable(
        requirement_id: CapabilityRequirementIdV1,
        attempt: ProviderInvocationUnavailableAttempt,
    ) -> Self {
        Self {
            generation: attempt.generation().clone(),
            operation: attempt.operation().clone(),
            capability_id: attempt.capability_id().clone(),
            requirement_id,
            state: ProviderInvocationRequirementState::InvocationUnavailable(attempt),
        }
    }

    /// Borrow the exact generation whose operation owns this requirement.
    #[must_use]
    pub const fn generation(&self) -> &CompiledGenerationDigest {
        &self.generation
    }

    /// Borrow the exact operation that owns this requirement occurrence.
    #[must_use]
    pub const fn operation(&self) -> &PackageResourceRefV1 {
        &self.operation
    }

    /// Borrow the Package-local requirement identity.
    #[must_use]
    pub const fn requirement_id(&self) -> &CapabilityRequirementIdV1 {
        &self.requirement_id
    }

    /// Borrow the portable capability identity.
    #[must_use]
    pub const fn capability_id(&self) -> &CapabilityIdV1 {
        &self.capability_id
    }

    /// Borrow the non-flattened provider requirement disposition.
    #[must_use]
    pub const fn requirement_state(&self) -> &ProviderInvocationRequirementState {
        &self.state
    }

    /// Derive readiness for this exact requirement occurrence.
    #[must_use]
    pub fn readiness(&self) -> ProviderInvocationReadinessState {
        match &self.state {
            ProviderInvocationRequirementState::Bound(readiness) => readiness.state(),
            ProviderInvocationRequirementState::Narrowed(_)
            | ProviderInvocationRequirementState::PolicyBlocked(_) => {
                ProviderInvocationReadinessState::Blocked
            }
            ProviderInvocationRequirementState::ProviderBindingAbsent => {
                ProviderInvocationReadinessState::Unavailable
            }
            ProviderInvocationRequirementState::InvocationUnavailable(_) => {
                ProviderInvocationReadinessState::Unavailable
            }
        }
    }
}

/// Non-empty, deterministic capability-requirement set for one operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationCapabilitySet(Vec<ProviderInvocationCapabilityReadiness>);

impl ProviderInvocationCapabilitySet {
    /// Sort a non-empty set and reject duplicate requirement identities.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationReadinessError`] for an empty set or duplicate
    /// requirement identity within the same operation.
    pub fn new(
        mut requirements: Vec<ProviderInvocationCapabilityReadiness>,
    ) -> Result<Self, ProviderInvocationReadinessError> {
        if requirements.is_empty() {
            return Err(ProviderInvocationReadinessError::EmptyCapabilityBearingOperation);
        }
        requirements.sort_by(|left, right| {
            left.requirement_id
                .as_str()
                .cmp(right.requirement_id.as_str())
                .then_with(|| {
                    left.capability_id
                        .as_str()
                        .cmp(right.capability_id.as_str())
                })
        });
        if requirements
            .windows(2)
            .any(|pair| pair[0].requirement_id == pair[1].requirement_id)
        {
            return Err(ProviderInvocationReadinessError::DuplicateRequirement);
        }
        Ok(Self(requirements))
    }

    /// Borrow the deterministic requirement sequence.
    #[must_use]
    pub fn as_slice(&self) -> &[ProviderInvocationCapabilityReadiness] {
        &self.0
    }
}

/// Exact operation exposure, kept distinct from provider and native readiness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderInvocationOperationExposure {
    /// No private operation binding exists.
    Unbound,
    /// A binding exists and the operation declares zero Package requirements.
    SemanticOnly,
    /// A binding exists with one or more Package-owned capability requirements.
    CapabilityBearing(ProviderInvocationCapabilitySet),
}

/// Generation-bound readiness vector for exactly one semantic operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderInvocationOperationReadiness {
    generation: CompiledGenerationDigest,
    operation: PackageResourceRefV1,
    exposure: ProviderInvocationOperationExposure,
    state: ProviderInvocationReadinessState,
}

impl ProviderInvocationOperationReadiness {
    /// Derive one operation's readiness without promoting sibling operations.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderInvocationReadinessError::RequirementCoordinateMismatch`]
    /// when a capability requirement belongs to another operation or generation.
    pub fn derive(
        generation: CompiledGenerationDigest,
        operation: PackageResourceRefV1,
        exposure: ProviderInvocationOperationExposure,
    ) -> Result<Self, ProviderInvocationReadinessError> {
        if let ProviderInvocationOperationExposure::CapabilityBearing(requirements) = &exposure {
            if requirements.as_slice().iter().any(|requirement| {
                requirement.generation() != &generation || requirement.operation() != &operation
            }) {
                return Err(ProviderInvocationReadinessError::RequirementCoordinateMismatch);
            }
        }
        let state = match &exposure {
            ProviderInvocationOperationExposure::Unbound => {
                ProviderInvocationReadinessState::Unavailable
            }
            ProviderInvocationOperationExposure::SemanticOnly => {
                ProviderInvocationReadinessState::Ready
            }
            ProviderInvocationOperationExposure::CapabilityBearing(requirements) => requirements
                .as_slice()
                .iter()
                .map(ProviderInvocationCapabilityReadiness::readiness)
                .fold(
                    ProviderInvocationReadinessState::Ready,
                    ProviderInvocationReadinessState::meet,
                ),
        };
        Ok(Self {
            generation,
            operation,
            exposure,
            state,
        })
    }

    /// Borrow the exact compiled generation that owns this vector.
    #[must_use]
    pub const fn generation(&self) -> &CompiledGenerationDigest {
        &self.generation
    }

    /// Borrow the exact semantic operation identity.
    #[must_use]
    pub const fn operation(&self) -> &PackageResourceRefV1 {
        &self.operation
    }

    /// Borrow operation exposure without substituting invocation readiness.
    #[must_use]
    pub const fn exposure(&self) -> &ProviderInvocationOperationExposure {
        &self.exposure
    }

    /// Return the conservative meet of this operation's constitutive dimensions.
    #[must_use]
    pub const fn state(&self) -> ProviderInvocationReadinessState {
        self.state
    }
}

/// Invalid construction of one operation-scoped readiness vector.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProviderInvocationReadinessError {
    /// A capability-bearing exposure omitted all Package requirements.
    #[error("capability-bearing provider invocation operation has no requirements")]
    EmptyCapabilityBearingOperation,
    /// Two requirements in one operation used the same local requirement identity.
    #[error("provider invocation operation contains a duplicate requirement identity")]
    DuplicateRequirement,
    /// Portable satisfaction was asserted without the independent invocation dimensions.
    #[error("satisfied provider requirement needs separately typed invocation evidence")]
    SatisfiedRequirementNeedsInvocationEvidence,
    /// A checked time predates the evidence observation itself.
    #[error("provider invocation evidence was checked before it was observed")]
    EvidenceNotYetObserved,
    /// One immutable dependency reference was paired with unlike bytes.
    #[error("provider invocation dependency reference has conflicting byte digests")]
    DependencyReferenceConflict,
    /// The selected provider binding no longer reproduces its canonical identity.
    #[error("provider invocation binding identity is invalid")]
    InvalidProviderBindingIdentity,
    /// The admitted observation belongs to another binding or implementation.
    #[error("provider invocation observation differs from the canonical provider binding")]
    ProviderCoordinateMismatch,
    /// The admitted evidence frontier describes another readiness subject.
    #[error("provider invocation evidence frontier describes another subject")]
    EvidenceSubjectMismatch,
    /// The Package-required effect is absent from the admitted effect ceiling.
    #[error("provider invocation required effect lies outside the admitted ceiling")]
    RequiredEffectOutsideCeiling,
    /// One admitted failed result was incorrectly offered as readiness proof.
    #[error("a failed provider invocation result cannot establish readiness")]
    FailedResultIsNotReadiness,
    /// A capability requirement belongs to another operation or compiled generation.
    #[error("provider invocation requirement differs from its operation coordinate")]
    RequirementCoordinateMismatch,
}
