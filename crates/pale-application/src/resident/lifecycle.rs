//! Pure lifecycle algebra for immutable compiled generations.
//!
//! Publication, advertisement, and activation are separate monotone heads. For
//! compiled generation `G` and activation fence `F_n`, their order is:

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/generation-lifecycle-order.display.mathml"
))]
//!
//! Each head family is a sparse projection over one store-global journal.
//!
//! Each successor binds the exact prior family digest. Exact global adjacency
//! is a journal-append property, so unrelated events may occur between two
//! records from the same family.
//!
//! A rollback is not a decrement. It is a later activation of retained content
//! under the successor fence `F[n+1]`. Sessions retain the exact generation and
//! fence observed at admission. Reclamation is conservative: incomplete
//! recovery knowledge can never become permission to collect.
//!
//! This module proves transition policy and returns a pure
//! [`GenerationLifecycleTransition`]. The resident journal coordinator owns
//! durable append and recovery; the one-shot path may consume the same
//! transition without pretending that persistence occurred. This module proves
//! no physical deletion, resident-process readiness, or native effect.
use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    ActivationFenceAdmissionContextV1, ActivationFenceDigest, ActivationFenceV1,
    ActivationFenceWireV1, ActiveGenerationHeadAdmissionContextV1, ActiveGenerationHeadDigest,
    ActiveGenerationHeadV1, ActiveGenerationHeadWireV1, AdvertisedGenerationHeadAdmissionContextV1,
    AdvertisedGenerationHeadDigest, AdvertisedGenerationHeadV1, AdvertisedGenerationHeadWireV1,
    ArtifactDigest, CanonicalSignatureV1, CompiledGenerationDigest, CompiledGenerationV1,
    EffectiveProjectProfileV1, InvalidResidentRecordV1, JournalSequenceV1, ProfileDigest,
    PublishedGenerationHeadAdmissionContextV1, PublishedGenerationHeadDigest,
    PublishedGenerationHeadV1, PublishedGenerationHeadWireV1, ResidentKeyIdV1,
    ResidentPrincipalIdV1, ResidentRecordRefV1, SessionAdmissionIdV1, SessionAdmissionLeaseDigest,
    SessionAdmissionLeasePreimageV1, SessionAdmissionLeaseV1, SigningAuthorityBindingV1,
    SigningKeyEpochV1, ACTIVATION_FENCE_V1_SCHEMA_ID, ACTIVE_GENERATION_HEAD_V1_SCHEMA_ID,
    ADVERTISED_GENERATION_HEAD_V1_SCHEMA_ID, PUBLISHED_GENERATION_HEAD_V1_SCHEMA_ID,
};
use serde::{Deserialize, Serialize};

use super::session_lease_custody::SessionLeaseRecoveryContext;
use super::GenerationActivationCompatibility;

/// Whether every recovery-held generation reference is currently observable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryReferenceKnowledge {
    /// The supplied set is the complete recovery-held reference frontier.
    Known(BTreeSet<CompiledGenerationDigest>),
    /// Recovery custody cannot currently prove a complete reference frontier.
    Unknown,
}

/// Stable semantic image of one locally undrained session-to-generation pin.
///
/// The surrounding map is keyed by [`SessionAdmissionIdV1`]. The lease digest
/// preserves the exact admitted bytes, while the independently typed generation
/// identifies the immutable body retained for that logical session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionGenerationPinSnapshot {
    /// Canonical digest of the exact admitted session lease.
    pub lease_digest: SessionAdmissionLeaseDigest,
    /// Immutable compiled generation retained by the session.
    pub generation: CompiledGenerationDigest,
}

/// Complete semantic state compared before one lifecycle transition is folded.
///
/// Head digests remain independently typed. Session entries bind immutable
/// lease identities to their pinned compiled generation. This is application
/// state, not a public durable record or an adapter serialization format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationLifecycleSnapshot {
    /// Current publication head in this semantic projection.
    pub published: Option<PublishedGenerationHeadDigest>,
    /// Current generation advertised for admission.
    pub advertised: Option<AdvertisedGenerationHeadDigest>,
    /// Head selected for new sessions.
    pub active: Option<pale_core::records::ActiveGenerationHeadDigest>,
    /// Deployment activation fence paired with [`Self::active`].
    pub activation_fence: Option<ActivationFenceDigest>,
    /// Immutable generation bodies retained by the lifecycle.
    pub retained_generations: BTreeSet<CompiledGenerationDigest>,
    /// Locally undrained session identities and exact lease/generation pins.
    pub session_pins: BTreeMap<SessionAdmissionIdV1, SessionGenerationPinSnapshot>,
    /// Generations retained as explicit rollback targets.
    pub rollback_targets: BTreeSet<CompiledGenerationDigest>,
    /// Complete or unavailable recovery-held generation references.
    pub recovery_references: RecoveryReferenceKnowledge,
}

/// One immutable session-to-generation retention binding against an active head.
///
/// This value proves only the exact lease/generation/head/fence relationship
/// needed to retain a compiled generation. It does not prove ambient grant or
/// runtime authority, session-floor freshness, revocation state, or time validity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionGenerationPin {
    lease: SessionAdmissionLeaseV1,
}

impl SessionGenerationPin {
    /// Validate a retention pin against the resolved generation head and fence.
    ///
    /// The active-head digest is retained explicitly because a session must
    /// never be retargeted when the deployment later activates another head.
    /// This is not full ambient-session admission; that requires the independent
    /// authority, floor, revocation, and time proofs absent from this function.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationLifecycleError`] for an invalid record or any mixed
    /// generation/fence coordinate.
    pub fn bind_generation_retention(
        lease: &SessionAdmissionLeaseV1,
        active: &ActiveGenerationHeadV1,
        fence: &ActivationFenceV1,
    ) -> Result<Self, GenerationLifecycleError> {
        lease.verify()?;
        active.verify()?;
        fence.verify()?;
        if lease.compiled_generation_digest() != active.compiled_generation_digest()
            || lease.compiled_generation_digest() != fence.compiled_generation_digest()
            || lease.active_generation_head_digest() != active.active_generation_head_digest()
            || lease.activation_fence_digest() != active.activation_fence_digest()
            || lease.activation_fence_digest() != fence.activation_fence_digest()
        {
            return Err(GenerationLifecycleError::CoordinateMismatch(
                "session pin generation/fence",
            ));
        }
        Ok(Self {
            lease: lease.clone(),
        })
    }

    /// Borrow the logical session identity that must remain unique while live.
    #[must_use]
    pub fn session_admission_id(&self) -> &SessionAdmissionIdV1 {
        self.lease.session_admission_id()
    }

    /// Borrow the immutable session lease identity.
    #[must_use]
    pub fn lease_digest(&self) -> &SessionAdmissionLeaseDigest {
        self.lease.session_admission_lease_digest()
    }

    /// Borrow the compiled generation pinned by this session.
    #[must_use]
    pub fn generation(&self) -> &CompiledGenerationDigest {
        self.lease.compiled_generation_digest()
    }

    /// Borrow the exact active head observed at session admission.
    #[must_use]
    pub fn active_head(&self) -> &pale_core::records::ActiveGenerationHeadDigest {
        self.lease.active_generation_head_digest()
    }

    /// Borrow the activation fence observed at session admission.
    #[must_use]
    pub fn activation_fence(&self) -> &ActivationFenceDigest {
        self.lease.activation_fence_digest()
    }

    /// Borrow the exact admitted lease retained by this semantic relation.
    pub(super) const fn lease(&self) -> &SessionAdmissionLeaseV1 {
        &self.lease
    }
}

/// Typed reason why one retained generation cannot be reclaimed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GenerationRetentionReason {
    /// The generation is the latest published head.
    Published,
    /// The generation is the latest advertised head.
    Advertised,
    /// The generation is selected for new sessions.
    Active,
    /// At least one locally undrained session-retention pin remains.
    LiveSession,
    /// The generation is an explicit rollback target.
    RollbackTarget,
    /// A known recovery record retains the generation.
    RecoveryReference,
}

/// Conservative answer to a generation-reclamation query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReclamationDisposition {
    /// No lifecycle or recovery reference retains the generation.
    Eligible,
    /// Complete reference knowledge proves the listed retention reasons.
    Retained(BTreeSet<GenerationRetentionReason>),
    /// Recovery custody cannot prove that its reference frontier is complete.
    Unknown,
}

/// One pure transition awaiting atomic compare-and-commit.
#[derive(Debug, Clone)]
pub struct GenerationLifecycleTransition {
    expected: GenerationLifecycleSnapshot,
    delta: GenerationLifecycleDelta,
    successor: GenerationLifecycle,
}

impl GenerationLifecycleTransition {
    /// Borrow the derived successor lifecycle.
    #[must_use]
    pub fn successor(&self) -> &GenerationLifecycle {
        &self.successor
    }

    /// Consume the pure transition without claiming durable persistence.
    ///
    /// This is the one-shot path. Resident callers requiring durable authority
    /// must pass the transition to the journal coordinator instead.
    #[must_use]
    pub fn into_successor(self) -> GenerationLifecycle {
        self.successor
    }

    pub(super) fn into_journal_parts(
        self,
    ) -> (
        GenerationLifecycleSnapshot,
        GenerationLifecycleDelta,
        GenerationLifecycle,
    ) {
        (self.expected, self.delta, self.successor)
    }
}

/// Closed sparse mutation algebra for the generation lifecycle.
///
/// Every variant carries only the record wire or stable identity required to
/// reapply one ordinary [`GenerationLifecycle`] law. The journal stores this
/// delta between typed predecessor and successor state fingerprints; complete
/// recovery images remain checkpoint-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body", rename_all = "snake_case")]
pub(super) enum GenerationLifecycleDelta {
    /// Publish one already sealed immutable generation.
    Publish {
        /// Exact publication-head wire re-admitted through owner custody.
        head: PublishedGenerationHeadWireV1,
    },
    /// Advertise the current published generation for admission.
    Advertise {
        /// Exact advertisement-head wire re-admitted through owner custody.
        head: AdvertisedGenerationHeadWireV1,
    },
    /// Activate an advertised retained generation under a successor fence.
    ///
    /// Ordinary activation and a later rollback share this delta because
    /// rollback is another forward activation after its target was explicitly
    /// retained; neither journal replay nor this variant rewinds coordinates.
    Activate {
        /// Exact active-head wire re-admitted through owner custody.
        head: ActiveGenerationHeadWireV1,
        /// Exact paired activation-fence wire re-admitted through owner custody.
        fence: ActivationFenceWireV1,
    },
    /// Add one already retained generation to the rollback-target set.
    RetainRollback {
        /// Stable retained generation identity.
        generation: CompiledGenerationDigest,
    },
    /// Add one owner-resolved live-session lease pin.
    PinSession {
        /// Stable lease identity resolved during replay.
        lease: SessionAdmissionLeaseDigest,
    },
    /// Remove one exact live-session lease pin.
    DrainSession {
        /// Stable logical session identity removed by the transition.
        session_admission_id: SessionAdmissionIdV1,
        /// Exact immutable lease identity authorized for removal.
        lease_digest: SessionAdmissionLeaseDigest,
    },
    /// Remove one generation whose complete reference analysis permits collection.
    Reclaim {
        /// Stable retained generation identity.
        generation: CompiledGenerationDigest,
    },
}

impl GenerationLifecycleDelta {
    /// Prove that any sparse head introduced by this delta belongs to `sequence`.
    pub(super) fn validate_envelope(
        &self,
        sequence: JournalSequenceV1,
    ) -> Result<(), GenerationLifecycleError> {
        let observed = match self {
            Self::Publish { head } => Some(head.journal_sequence()),
            Self::Advertise { head } => Some(head.journal_sequence()),
            Self::Activate { head, .. } => Some(head.journal_sequence()),
            Self::RetainRollback { .. }
            | Self::PinSession { .. }
            | Self::DrainSession { .. }
            | Self::Reclaim { .. } => None,
        };
        if observed.is_none_or(|observed| *observed == sequence) {
            Ok(())
        } else {
            Err(GenerationLifecycleError::CoordinateMismatch(
                "journal lifecycle-delta sequence",
            ))
        }
    }

    /// Reapply this delta through the ordinary lifecycle laws and exact owner context.
    pub(super) fn apply(
        self,
        current: &GenerationLifecycle,
        resolver: &dyn GenerationLifecycleRecoveryResolver,
        session_leases: &SessionLeaseRecoveryContext<'_>,
    ) -> Result<GenerationLifecycle, GenerationLifecycleError> {
        let transition = match self {
            Self::Publish { head } => current.publish(admit_published(head, resolver)?)?,
            Self::Advertise { head } => current.advertise(admit_advertised(head, resolver)?)?,
            Self::Activate { head, fence } => {
                let fence = admit_fence(fence, resolver)?;
                let head = admit_active(head, resolver)?;
                current.activate_checked(head, fence, None)?
            }
            Self::RetainRollback { generation } => current.retain_for_rollback(generation)?,
            Self::PinSession { lease } => {
                let resolved = session_leases.recover(&lease, resolver)?;
                current.pin_session(&resolved)?
            }
            Self::DrainSession {
                session_admission_id,
                lease_digest,
            } => current.drain_session(&session_admission_id, &lease_digest)?,
            Self::Reclaim { generation } => current.reclaim(&generation)?,
        };
        Ok(transition.into_successor())
    }
}

/// In-memory semantic state consumed and replaced by pure lifecycle transitions.
#[derive(Debug, Clone)]
pub struct GenerationLifecycle {
    published: Option<PublishedGenerationHeadV1>,
    advertised: Option<AdvertisedGenerationHeadV1>,
    active: Option<ActiveGenerationHeadV1>,
    fence: Option<ActivationFenceV1>,
    retained_generations: BTreeSet<CompiledGenerationDigest>,
    session_pins: BTreeMap<SessionAdmissionIdV1, SessionGenerationPin>,
    rollback_targets: BTreeSet<CompiledGenerationDigest>,
    recovery_references: RecoveryReferenceKnowledge,
}

/// Owner-resolved bodies required to re-admit lifecycle wires during replay.
///
/// Implementations are views over retained resident custody. Returning `None`
/// closes recovery; this interface never performs ambient lookup or copies
/// Package, graph, arena, or adapter state into a journal delta or checkpoint.
pub trait GenerationLifecycleRecoveryResolver {
    /// Resolve the exact historical runtime authority that signed a retained session lease.
    fn session_signing_authority(
        &self,
        authority_ref: &ResidentRecordRefV1,
        authority_sha256: &ArtifactDigest,
        issuer_principal_id: &ResidentPrincipalIdV1,
        issuer_key_id: &ResidentKeyIdV1,
        issuer_key_epoch: SigningKeyEpochV1,
    ) -> Option<SigningAuthorityBindingV1>;

    /// Verify a retained session lease with its resolved historical authority.
    ///
    /// # Errors
    ///
    /// Returns the exact resident-record error when the retained signature is
    /// not valid for this authority, domain, preimage, and digest.
    fn verify_session_admission_lease_signature(
        &self,
        authority: &SigningAuthorityBindingV1,
        domain: &[u8],
        preimage: &SessionAdmissionLeasePreimageV1,
        digest: &SessionAdmissionLeaseDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>;

    /// Resolve one retained compiled-generation body by its canonical digest.
    fn compiled_generation(
        &self,
        digest: &CompiledGenerationDigest,
    ) -> Option<&CompiledGenerationV1>;

    /// Resolve one admitted publication head, including historical predecessors.
    fn published_head(
        &self,
        digest: &PublishedGenerationHeadDigest,
    ) -> Option<&PublishedGenerationHeadV1>;

    /// Resolve one admitted advertisement head, including historical predecessors.
    fn advertised_head(
        &self,
        digest: &AdvertisedGenerationHeadDigest,
    ) -> Option<&AdvertisedGenerationHeadV1>;

    /// Resolve one admitted active head, including heads retained by live sessions.
    fn active_head(&self, digest: &ActiveGenerationHeadDigest) -> Option<&ActiveGenerationHeadV1>;

    /// Resolve one admitted activation fence, including fences retained by live sessions.
    fn activation_fence(&self, digest: &ActivationFenceDigest) -> Option<&ActivationFenceV1>;

    /// Resolve one admitted effective profile, including profiles retained by sessions.
    fn effective_profile(&self, digest: &ProfileDigest) -> Option<&EffectiveProjectProfileV1>;
}

/// Handle-free lifecycle image encoded only inside a private recovery checkpoint.
///
/// Untrusted head wires retain the exact identities needed to request owner
/// context during replay. Live session pins retain only their stable admitted
/// lease identities; the resolver returns the exact lease, active-head, and
/// fence bodies from resident custody. The image contains no duplicated session
/// coordinates, compiled-generation bodies, graph handles, arena positions,
/// ring cursors, or adapter framing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct GenerationLifecycleRecoveryStateV1 {
    /// Untrusted current publication-head wire, when publication has begun.
    published: Option<PublishedGenerationHeadWireV1>,
    /// Untrusted current advertisement-head wire, when admission is advertised.
    advertised: Option<AdvertisedGenerationHeadWireV1>,
    /// Untrusted current active-head wire, when activation has occurred.
    active: Option<ActiveGenerationHeadWireV1>,
    /// Untrusted current deployment fence paired with the active head.
    fence: Option<ActivationFenceWireV1>,
    /// Stable identities of every immutable generation body that must resolve.
    retained_generations: BTreeSet<CompiledGenerationDigest>,
    /// Strictly ordered logical sessions and their exact admitted lease identities.
    session_pins: Vec<SessionGenerationPinRecoveryV1>,
    /// Stable identities retained as explicit rollback targets.
    rollback_targets: BTreeSet<CompiledGenerationDigest>,
    /// Complete or unavailable recovery-held generation identities.
    recovery_references: RecoveryReferenceKnowledge,
}

/// Minimal owner-custody coordinate for one live session pin in a checkpoint.
///
/// The logical identity is the uniqueness key. The digest independently binds
/// the exact lease bytes that the recovery resolver must return. Keeping the pair
/// avoids copying the lease body while preventing a different lease for the same
/// logical session from being substituted during recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SessionGenerationPinRecoveryV1 {
    /// Logical session identity whose generation retention remains live.
    session_admission_id: SessionAdmissionIdV1,
    /// Canonical digest of the exact admitted lease held by owner custody.
    lease_digest: SessionAdmissionLeaseDigest,
}

impl GenerationLifecycleRecoveryStateV1 {
    /// Project one admitted lifecycle into wires and stable owner-custody identities.
    pub(super) fn from_lifecycle(lifecycle: &GenerationLifecycle) -> Self {
        Self {
            published: lifecycle
                .published
                .as_ref()
                .map(PublishedGenerationHeadWireV1::from),
            advertised: lifecycle
                .advertised
                .as_ref()
                .map(AdvertisedGenerationHeadWireV1::from),
            active: lifecycle
                .active
                .as_ref()
                .map(ActiveGenerationHeadWireV1::from),
            fence: lifecycle.fence.as_ref().map(ActivationFenceWireV1::from),
            retained_generations: lifecycle.retained_generations.clone(),
            session_pins: lifecycle
                .session_pins
                .values()
                .map(|pin| SessionGenerationPinRecoveryV1 {
                    session_admission_id: pin.session_admission_id().clone(),
                    lease_digest: pin.lease_digest().clone(),
                })
                .collect(),
            rollback_targets: lifecycle.rollback_targets.clone(),
            recovery_references: lifecycle.recovery_references.clone(),
        }
    }

    /// Re-admit every untrusted wire and stable identity through exact owner context.
    pub(super) fn admit(
        self,
        resolver: &dyn GenerationLifecycleRecoveryResolver,
        session_leases: &SessionLeaseRecoveryContext<'_>,
    ) -> Result<GenerationLifecycle, GenerationLifecycleError> {
        let published = self
            .published
            .map(|value| admit_published(value, resolver))
            .transpose()?;
        let advertised = self
            .advertised
            .map(|value| admit_advertised(value, resolver))
            .transpose()?;
        let fence = self
            .fence
            .map(|value| admit_fence(value, resolver))
            .transpose()?;
        let active = self
            .active
            .map(|value| admit_active(value, resolver))
            .transpose()?;

        for digest in &self.retained_generations {
            let body = resolver.compiled_generation(digest).ok_or(
                GenerationLifecycleError::MissingRecoveryContext(
                    "retained compiled generation body",
                ),
            )?;
            verify_resolved_record(body.verify(), "retained compiled generation body")?;
            if body.compiled_generation_digest() != digest {
                return Err(GenerationLifecycleError::RecoveryContextSubstitution(
                    "retained compiled generation body",
                ));
            }
        }

        match (&advertised, &published) {
            (None, _) => {}
            (Some(advertised), Some(published))
                if advertised.compiled_generation_digest()
                    == published.compiled_generation_digest()
                    && advertised.published_generation_head_digest()
                        == published.published_generation_head_digest() => {}
            (Some(_), None) => {
                return Err(GenerationLifecycleError::MissingPrecondition(
                    "recovered publication for advertisement",
                ));
            }
            _ => {
                return Err(GenerationLifecycleError::CoordinateMismatch(
                    "recovered advertisement and publication",
                ));
            }
        }
        match (&active, &fence, &advertised) {
            (None, None, _) => {}
            (Some(active), Some(fence), Some(advertised))
                if active.compiled_generation_digest() == fence.compiled_generation_digest()
                    && active.activation_fence_digest() == fence.activation_fence_digest()
                    && active.compiled_generation_digest()
                        == advertised.compiled_generation_digest()
                    && active.advertised_generation_head_digest()
                        == advertised.advertised_generation_head_digest() => {}
            _ => {
                return Err(GenerationLifecycleError::CoordinateMismatch(
                    "recovered active head, fence, and advertisement",
                ));
            }
        }
        for generation in published
            .iter()
            .map(PublishedGenerationHeadV1::compiled_generation_digest)
            .chain(
                advertised
                    .iter()
                    .map(AdvertisedGenerationHeadV1::compiled_generation_digest),
            )
            .chain(
                active
                    .iter()
                    .map(ActiveGenerationHeadV1::compiled_generation_digest),
            )
        {
            if !self.retained_generations.contains(generation) {
                return Err(GenerationLifecycleError::GenerationNotRetained);
            }
        }
        if !self.rollback_targets.is_subset(&self.retained_generations) {
            return Err(GenerationLifecycleError::GenerationNotRetained);
        }
        if let RecoveryReferenceKnowledge::Known(references) = &self.recovery_references {
            if !references.is_subset(&self.retained_generations) {
                return Err(GenerationLifecycleError::GenerationNotRetained);
            }
        }
        let mut session_pins = BTreeMap::new();
        let mut prior_session = None;
        for recorded in self.session_pins {
            if prior_session
                .as_ref()
                .is_some_and(|prior| prior >= &recorded.session_admission_id)
            {
                return Err(GenerationLifecycleError::NonCanonicalRecovery(
                    "session pins",
                ));
            }
            let lease = session_leases.recover(&recorded.lease_digest, resolver)?;
            if lease.session_admission_lease_digest() != &recorded.lease_digest
                || lease.session_admission_id() != &recorded.session_admission_id
            {
                return Err(GenerationLifecycleError::RecoveryContextSubstitution(
                    "live session lease body",
                ));
            }
            let session_active = resolver
                .active_head(lease.active_generation_head_digest())
                .ok_or(GenerationLifecycleError::MissingRecoveryContext(
                    "live session active head",
                ))?;
            let session_fence = resolver
                .activation_fence(lease.activation_fence_digest())
                .ok_or(GenerationLifecycleError::MissingRecoveryContext(
                    "live session activation fence",
                ))?;
            verify_resolved_record(session_active.verify(), "live session active head")?;
            verify_resolved_record(session_fence.verify(), "live session activation fence")?;
            let pin = SessionGenerationPin::bind_generation_retention(
                &lease,
                session_active,
                session_fence,
            )?;
            if !self.retained_generations.contains(pin.generation()) {
                return Err(GenerationLifecycleError::GenerationNotRetained);
            }
            prior_session = Some(recorded.session_admission_id.clone());
            if session_pins
                .insert(recorded.session_admission_id, pin)
                .is_some()
            {
                return Err(GenerationLifecycleError::DuplicateSession);
            }
        }
        Ok(GenerationLifecycle {
            published,
            advertised,
            active,
            fence,
            retained_generations: self.retained_generations,
            session_pins,
            rollback_targets: self.rollback_targets,
            recovery_references: self.recovery_references,
        })
    }

    /// Return the greatest sparse head sequence embedded in this image.
    pub(super) fn maximum_journal_sequence(&self) -> Option<JournalSequenceV1> {
        [
            self.published
                .as_ref()
                .map(|value| *value.journal_sequence()),
            self.advertised
                .as_ref()
                .map(|value| *value.journal_sequence()),
            self.active.as_ref().map(|value| *value.journal_sequence()),
        ]
        .into_iter()
        .flatten()
        .max_by_key(|value| value.get())
    }
}

/// Bind changed sparse head families to one enclosing journal coordinate.
fn validate_lifecycle_head_sequences(
    published: Option<(&PublishedGenerationHeadDigest, &JournalSequenceV1)>,
    advertised: Option<(&AdvertisedGenerationHeadDigest, &JournalSequenceV1)>,
    active: Option<(&ActiveGenerationHeadDigest, &JournalSequenceV1)>,
    expected: &GenerationLifecycleSnapshot,
    sequence: JournalSequenceV1,
) -> Result<(), GenerationLifecycleError> {
    validate_changed_head_sequence(
        published,
        expected.published.as_ref(),
        sequence,
        "journal publication sequence",
    )?;
    validate_changed_head_sequence(
        advertised,
        expected.advertised.as_ref(),
        sequence,
        "journal advertisement sequence",
    )?;
    validate_changed_head_sequence(
        active,
        expected.active.as_ref(),
        sequence,
        "journal activation sequence",
    )
}

/// Re-admit one publication wire with exact generation and predecessor custody.
fn admit_published(
    value: PublishedGenerationHeadWireV1,
    resolver: &dyn GenerationLifecycleRecoveryResolver,
) -> Result<PublishedGenerationHeadV1, GenerationLifecycleError> {
    let claimed_digest = value.published_generation_head_digest().clone();
    let generation_digest = value.compiled_generation_digest().clone();
    let sequence = *value.journal_sequence();
    let predecessor_digest = value.predecessor_published_generation_head_digest().clone();
    let schema = recovery_schema(PUBLISHED_GENERATION_HEAD_V1_SCHEMA_ID)?;
    let generation = resolver.compiled_generation(&generation_digest).ok_or(
        GenerationLifecycleError::MissingRecoveryContext("compiled generation for published head"),
    )?;
    verify_resolved_record(
        generation.verify(),
        "compiled generation for published head",
    )?;
    if generation.compiled_generation_digest() != &generation_digest {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "compiled generation for published head",
        ));
    }
    let predecessor =
        predecessor_digest
            .as_ref()
            .map(|digest| {
                resolver.published_head(digest).ok_or(
                    GenerationLifecycleError::MissingRecoveryContext("published head predecessor"),
                )
            })
            .transpose()?;
    if let Some(predecessor) = predecessor {
        verify_resolved_record(predecessor.verify(), "published head predecessor")?;
    }
    if predecessor.as_ref().is_some_and(|resolved| {
        Some(resolved.published_generation_head_digest()) != predecessor_digest.as_ref()
    }) {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "published head predecessor",
        ));
    }
    let admitted = PublishedGenerationHeadV1::admit_contextual_wire(
        value,
        &PublishedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: &schema,
            compiled_generation: generation,
            predecessor,
        },
    )?;
    if admitted.published_generation_head_digest() != &claimed_digest
        || admitted.journal_sequence() != &sequence
    {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "published head recovery coordinates",
        ));
    }
    Ok(admitted)
}

/// Re-admit one advertisement wire with exact publication and predecessor custody.
fn admit_advertised(
    value: AdvertisedGenerationHeadWireV1,
    resolver: &dyn GenerationLifecycleRecoveryResolver,
) -> Result<AdvertisedGenerationHeadV1, GenerationLifecycleError> {
    let claimed_digest = value.advertised_generation_head_digest().clone();
    let generation_digest = value.compiled_generation_digest().clone();
    let published_digest = value.published_generation_head_digest().clone();
    let sequence = *value.journal_sequence();
    let predecessor_digest = value
        .predecessor_advertised_generation_head_digest()
        .clone();
    let schema = recovery_schema(ADVERTISED_GENERATION_HEAD_V1_SCHEMA_ID)?;
    let published = resolver.published_head(&published_digest).ok_or(
        GenerationLifecycleError::MissingRecoveryContext("publication for advertised head"),
    )?;
    verify_resolved_record(published.verify(), "publication for advertised head")?;
    if published.published_generation_head_digest() != &published_digest
        || published.compiled_generation_digest() != &generation_digest
    {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "publication for advertised head",
        ));
    }
    let predecessor = predecessor_digest
        .as_ref()
        .map(|digest| {
            resolver.advertised_head(digest).ok_or(
                GenerationLifecycleError::MissingRecoveryContext("advertised head predecessor"),
            )
        })
        .transpose()?;
    if let Some(predecessor) = predecessor {
        verify_resolved_record(predecessor.verify(), "advertised head predecessor")?;
    }
    if predecessor.as_ref().is_some_and(|resolved| {
        Some(resolved.advertised_generation_head_digest()) != predecessor_digest.as_ref()
    }) {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "advertised head predecessor",
        ));
    }
    let admitted = AdvertisedGenerationHeadV1::admit_contextual_wire(
        value,
        &AdvertisedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: &schema,
            published_head: published,
            predecessor,
        },
    )?;
    if admitted.advertised_generation_head_digest() != &claimed_digest
        || admitted.journal_sequence() != &sequence
    {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "advertised head recovery coordinates",
        ));
    }
    Ok(admitted)
}

/// Re-admit one fence wire with exact generation and predecessor custody.
fn admit_fence(
    value: ActivationFenceWireV1,
    resolver: &dyn GenerationLifecycleRecoveryResolver,
) -> Result<ActivationFenceV1, GenerationLifecycleError> {
    let claimed_digest = value.activation_fence_digest().clone();
    let generation_digest = value.compiled_generation_digest().clone();
    let predecessor_digest = value.predecessor_activation_fence_digest().clone();
    let schema = recovery_schema(ACTIVATION_FENCE_V1_SCHEMA_ID)?;
    let generation = resolver.compiled_generation(&generation_digest).ok_or(
        GenerationLifecycleError::MissingRecoveryContext(
            "compiled generation for activation fence",
        ),
    )?;
    verify_resolved_record(
        generation.verify(),
        "compiled generation for activation fence",
    )?;
    if generation.compiled_generation_digest() != &generation_digest {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "compiled generation for activation fence",
        ));
    }
    let predecessor = predecessor_digest
        .as_ref()
        .map(|digest| {
            resolver.activation_fence(digest).ok_or(
                GenerationLifecycleError::MissingRecoveryContext("activation fence predecessor"),
            )
        })
        .transpose()?;
    if let Some(predecessor) = predecessor {
        verify_resolved_record(predecessor.verify(), "activation fence predecessor")?;
    }
    if predecessor.as_ref().is_some_and(|resolved| {
        Some(resolved.activation_fence_digest()) != predecessor_digest.as_ref()
    }) {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "activation fence predecessor",
        ));
    }
    let admitted = ActivationFenceV1::admit_contextual_wire(
        value,
        &ActivationFenceAdmissionContextV1 {
            expected_schema_sha256: &schema,
            compiled_generation: generation,
            predecessor,
        },
    )?;
    if admitted.activation_fence_digest() != &claimed_digest {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "activation fence recovery coordinates",
        ));
    }
    Ok(admitted)
}

/// Re-admit one active-head wire with exact advertisement, fence, and predecessor custody.
fn admit_active(
    value: ActiveGenerationHeadWireV1,
    resolver: &dyn GenerationLifecycleRecoveryResolver,
) -> Result<ActiveGenerationHeadV1, GenerationLifecycleError> {
    let claimed_digest = value.active_generation_head_digest().clone();
    let generation_digest = value.compiled_generation_digest().clone();
    let advertised_digest = value.advertised_generation_head_digest().clone();
    let fence_digest = value.activation_fence_digest().clone();
    let sequence = *value.journal_sequence();
    let predecessor_digest = value.predecessor_active_generation_head_digest().clone();
    let schema = recovery_schema(ACTIVE_GENERATION_HEAD_V1_SCHEMA_ID)?;
    let advertised = resolver.advertised_head(&advertised_digest).ok_or(
        GenerationLifecycleError::MissingRecoveryContext("advertisement for active head"),
    )?;
    let fence = resolver.activation_fence(&fence_digest).ok_or(
        GenerationLifecycleError::MissingRecoveryContext("activation fence for active head"),
    )?;
    verify_resolved_record(advertised.verify(), "advertisement for active head")?;
    verify_resolved_record(fence.verify(), "activation fence for active head")?;
    if advertised.advertised_generation_head_digest() != &advertised_digest
        || fence.activation_fence_digest() != &fence_digest
        || advertised.compiled_generation_digest() != &generation_digest
        || fence.compiled_generation_digest() != &generation_digest
    {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "advertisement and fence for active head",
        ));
    }
    let predecessor =
        predecessor_digest
            .as_ref()
            .map(|digest| {
                resolver.active_head(digest).ok_or(
                    GenerationLifecycleError::MissingRecoveryContext("active head predecessor"),
                )
            })
            .transpose()?;
    if let Some(predecessor) = predecessor {
        verify_resolved_record(predecessor.verify(), "active head predecessor")?;
    }
    if predecessor.as_ref().is_some_and(|resolved| {
        Some(resolved.active_generation_head_digest()) != predecessor_digest.as_ref()
    }) {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "active head predecessor",
        ));
    }
    let admitted = ActiveGenerationHeadV1::admit_contextual_wire(
        value,
        &ActiveGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: &schema,
            advertised_head: advertised,
            activation_fence: fence,
            predecessor,
        },
    )?;
    if admitted.active_generation_head_digest() != &claimed_digest
        || admitted.journal_sequence() != &sequence
    {
        return Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "active head recovery coordinates",
        ));
    }
    Ok(admitted)
}

/// Resolve one lifecycle schema from the embedded initial-v1 registry SSOT.
fn recovery_schema(
    registry_id: &'static str,
) -> Result<pale_core::records::SchemaDigest, GenerationLifecycleError> {
    pale_contracts::record_schema_identity(registry_id)
        .map(|identity| identity.schema_sha256)
        .map_err(GenerationLifecycleError::ContractRegistry)
}

/// Convert an invalid owner-resolved body into a typed substitution failure.
fn verify_resolved_record(
    verification: Result<(), InvalidResidentRecordV1>,
    context: &'static str,
) -> Result<(), GenerationLifecycleError> {
    verification.map_err(|_| GenerationLifecycleError::RecoveryContextSubstitution(context))
}

/// Require changed sparse heads at the envelope sequence and retained heads no later.
fn validate_changed_head_sequence<D: PartialEq>(
    successor: Option<(&D, &JournalSequenceV1)>,
    expected_digest: Option<&D>,
    envelope_sequence: JournalSequenceV1,
    coordinate: &'static str,
) -> Result<(), GenerationLifecycleError> {
    match successor {
        Some((digest, sequence)) if Some(digest) != expected_digest => {
            if *sequence == envelope_sequence {
                Ok(())
            } else {
                Err(GenerationLifecycleError::CoordinateMismatch(coordinate))
            }
        }
        Some((_, sequence)) if sequence.get() <= envelope_sequence.get() => Ok(()),
        None if expected_digest.is_none() => Ok(()),
        _ => Err(GenerationLifecycleError::CoordinateMismatch(coordinate)),
    }
}

impl GenerationLifecycle {
    /// Create an empty lifecycle with explicit recovery-reference knowledge.
    #[must_use]
    pub fn empty(recovery_references: RecoveryReferenceKnowledge) -> Self {
        Self {
            published: None,
            advertised: None,
            active: None,
            fence: None,
            retained_generations: BTreeSet::new(),
            session_pins: BTreeMap::new(),
            rollback_targets: BTreeSet::new(),
            recovery_references,
        }
    }

    /// Derive publication of one already sealed immutable generation.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationLifecycleError`] when the record is invalid or its
    /// publication lineage does not advance in the store-global journal order.
    pub fn publish(
        &self,
        head: PublishedGenerationHeadV1,
    ) -> Result<GenerationLifecycleTransition, GenerationLifecycleError> {
        head.verify()?;
        let delta = GenerationLifecycleDelta::Publish {
            head: PublishedGenerationHeadWireV1::from(&head),
        };
        let generation = head.compiled_generation_digest().clone();
        validate_published_successor(self.published.as_ref(), &head)?;
        let mut successor = self.clone();
        successor.retained_generations.insert(generation);
        successor.published = Some(head);
        Ok(self.transition(delta, successor))
    }

    /// Derive advertisement of the exact current published generation.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationLifecycleError`] unless the advertisement names the
    /// current publication and advances its own lineage in global journal order.
    pub fn advertise(
        &self,
        head: AdvertisedGenerationHeadV1,
    ) -> Result<GenerationLifecycleTransition, GenerationLifecycleError> {
        head.verify()?;
        let delta = GenerationLifecycleDelta::Advertise {
            head: AdvertisedGenerationHeadWireV1::from(&head),
        };
        let published =
            self.published
                .as_ref()
                .ok_or(GenerationLifecycleError::MissingPrecondition(
                    "published head",
                ))?;
        if head.compiled_generation_digest() != published.compiled_generation_digest()
            || head.published_generation_head_digest()
                != published.published_generation_head_digest()
        {
            return Err(GenerationLifecycleError::CoordinateMismatch(
                "advertisement publication",
            ));
        }
        validate_advertised_successor(self.advertised.as_ref(), &head)?;
        let mut successor = self.clone();
        successor.advertised = Some(head);
        Ok(self.transition(delta, successor))
    }

    /// Derive activation of the exact current advertisement under a successor fence.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationLifecycleError`] for an unretained generation or
    /// any advertisement, active-head, or fence mismatch.
    pub fn activate(
        &self,
        head: ActiveGenerationHeadV1,
        fence: ActivationFenceV1,
        compatibility: &GenerationActivationCompatibility,
    ) -> Result<GenerationLifecycleTransition, GenerationLifecycleError> {
        self.activate_checked(head, fence, Some(compatibility))
    }

    fn activate_checked(
        &self,
        head: ActiveGenerationHeadV1,
        fence: ActivationFenceV1,
        compatibility: Option<&GenerationActivationCompatibility>,
    ) -> Result<GenerationLifecycleTransition, GenerationLifecycleError> {
        head.verify()?;
        fence.verify()?;
        let delta = GenerationLifecycleDelta::Activate {
            head: ActiveGenerationHeadWireV1::from(&head),
            fence: ActivationFenceWireV1::from(&fence),
        };
        if compatibility.is_some_and(|proof| {
            !proof.admits(
                self.active
                    .as_ref()
                    .map(ActiveGenerationHeadV1::compiled_generation_digest),
                head.compiled_generation_digest(),
            )
        }) {
            return Err(GenerationLifecycleError::CoordinateMismatch(
                "activation compatibility",
            ));
        }
        let advertised =
            self.advertised
                .as_ref()
                .ok_or(GenerationLifecycleError::MissingPrecondition(
                    "advertised head",
                ))?;
        if head.compiled_generation_digest() != advertised.compiled_generation_digest()
            || head.advertised_generation_head_digest()
                != advertised.advertised_generation_head_digest()
            || head.compiled_generation_digest() != fence.compiled_generation_digest()
            || head.activation_fence_digest() != fence.activation_fence_digest()
        {
            return Err(GenerationLifecycleError::CoordinateMismatch(
                "activation advertisement/fence",
            ));
        }
        validate_fence_successor(self.fence.as_ref(), &fence)?;
        validate_active_successor(self.active.as_ref(), &head)?;
        if !self
            .retained_generations
            .contains(head.compiled_generation_digest())
        {
            return Err(GenerationLifecycleError::GenerationNotRetained);
        }
        let mut successor = self.clone();
        successor.active = Some(head);
        successor.fence = Some(fence);
        Ok(self.transition(delta, successor))
    }

    /// Derive a later activation of retained prior content.
    ///
    /// This is the rollback operation: callers must first publish and advertise
    /// the target through the ordinary heads, then activate it under the next
    /// fence. No sequence, owner fact, or native revision is rewound.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationLifecycleError`] unless the generation is an
    /// explicit rollback target and satisfies the ordinary activation laws.
    pub fn rollback(
        &self,
        head: ActiveGenerationHeadV1,
        fence: ActivationFenceV1,
    ) -> Result<GenerationLifecycleTransition, GenerationLifecycleError> {
        if !self
            .rollback_targets
            .contains(head.compiled_generation_digest())
        {
            return Err(GenerationLifecycleError::NotRollbackTarget);
        }
        self.activate_checked(head, fence, None)
    }

    /// Retain a generation as an explicit rollback target.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationLifecycleError::GenerationNotRetained`] when the
    /// immutable generation body is not already retained.
    pub fn retain_for_rollback(
        &self,
        generation: CompiledGenerationDigest,
    ) -> Result<GenerationLifecycleTransition, GenerationLifecycleError> {
        if !self.retained_generations.contains(&generation) {
            return Err(GenerationLifecycleError::GenerationNotRetained);
        }
        let delta = GenerationLifecycleDelta::RetainRollback {
            generation: generation.clone(),
        };
        let mut successor = self.clone();
        successor.rollback_targets.insert(generation);
        Ok(self.transition(delta, successor))
    }

    /// Add one immutable generation-retention pin against the current active state.
    ///
    /// This transition keeps the named generation from reclamation. It does not
    /// establish current ambient authority, floor, revocation, or time validity.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationLifecycleError`] when no active state exists, the
    /// lease mixes coordinates, or its logical identity is already pinned.
    pub fn pin_session(
        &self,
        lease: &SessionAdmissionLeaseV1,
    ) -> Result<GenerationLifecycleTransition, GenerationLifecycleError> {
        let active = self
            .active
            .as_ref()
            .ok_or(GenerationLifecycleError::MissingPrecondition("active head"))?;
        let fence = self
            .fence
            .as_ref()
            .ok_or(GenerationLifecycleError::MissingPrecondition(
                "activation fence",
            ))?;
        let pin = SessionGenerationPin::bind_generation_retention(lease, active, fence)?;
        if let Some(existing) = self.session_pins.get(pin.session_admission_id()) {
            return if existing.lease_digest() == pin.lease_digest() {
                Err(GenerationLifecycleError::DuplicateSession)
            } else {
                Err(GenerationLifecycleError::SessionIdentityConflict)
            };
        }
        let delta = GenerationLifecycleDelta::PinSession {
            lease: pin.lease_digest().clone(),
        };
        let mut successor = self.clone();
        successor
            .session_pins
            .insert(pin.session_admission_id().clone(), pin);
        Ok(self.transition(delta, successor))
    }

    /// Remove exactly one terminated session pin without retargeting another.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationLifecycleError::UnknownSession`] when the logical
    /// session identity is absent, or
    /// [`GenerationLifecycleError::SessionIdentityConflict`] when it is bound to
    /// different immutable lease bytes.
    pub fn drain_session(
        &self,
        session_admission_id: &SessionAdmissionIdV1,
        lease_digest: &SessionAdmissionLeaseDigest,
    ) -> Result<GenerationLifecycleTransition, GenerationLifecycleError> {
        match self.session_pins.get(session_admission_id) {
            None => return Err(GenerationLifecycleError::UnknownSession),
            Some(pin) if pin.lease_digest() != lease_digest => {
                return Err(GenerationLifecycleError::SessionIdentityConflict);
            }
            Some(_) => {}
        }
        let delta = GenerationLifecycleDelta::DrainSession {
            session_admission_id: session_admission_id.clone(),
            lease_digest: lease_digest.clone(),
        };
        let mut successor = self.clone();
        successor.session_pins.remove(session_admission_id);
        Ok(self.transition(delta, successor))
    }

    /// Derive removal of one retained body after complete reference analysis.
    ///
    /// Reclamation is a monotone authority transition: it removes only the
    /// reconstructible resident body coordinate and never rewinds a head,
    /// journal entry, owner fact, or native revision. Call
    /// [`Self::reclamation_disposition`] to inspect the exact denial reasons.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationLifecycleError::GenerationNotRetained`] when the body
    /// is absent, or [`GenerationLifecycleError::GenerationNotReclaimable`] while
    /// any live or unknown reference prevents collection.
    pub fn reclaim(
        &self,
        generation: &CompiledGenerationDigest,
    ) -> Result<GenerationLifecycleTransition, GenerationLifecycleError> {
        if !self.retained_generations.contains(generation) {
            return Err(GenerationLifecycleError::GenerationNotRetained);
        }
        if self.reclamation_disposition(generation) != ReclamationDisposition::Eligible {
            return Err(GenerationLifecycleError::GenerationNotReclaimable);
        }
        let delta = GenerationLifecycleDelta::Reclaim {
            generation: generation.clone(),
        };
        let mut successor = self.clone();
        successor.retained_generations.remove(generation);
        Ok(self.transition(delta, successor))
    }

    /// Classify whether a retained generation is safe for later physical collection.
    #[must_use]
    pub fn reclamation_disposition(
        &self,
        generation: &CompiledGenerationDigest,
    ) -> ReclamationDisposition {
        if matches!(
            self.recovery_references,
            RecoveryReferenceKnowledge::Unknown
        ) {
            return ReclamationDisposition::Unknown;
        }
        let mut reasons = BTreeSet::new();
        if self
            .published
            .as_ref()
            .is_some_and(|head| head.compiled_generation_digest() == generation)
        {
            reasons.insert(GenerationRetentionReason::Published);
        }
        if self
            .advertised
            .as_ref()
            .is_some_and(|head| head.compiled_generation_digest() == generation)
        {
            reasons.insert(GenerationRetentionReason::Advertised);
        }
        if self
            .active
            .as_ref()
            .is_some_and(|head| head.compiled_generation_digest() == generation)
        {
            reasons.insert(GenerationRetentionReason::Active);
        }
        if self
            .session_pins
            .values()
            .any(|pin| pin.generation() == generation)
        {
            reasons.insert(GenerationRetentionReason::LiveSession);
        }
        if self.rollback_targets.contains(generation) {
            reasons.insert(GenerationRetentionReason::RollbackTarget);
        }
        if let RecoveryReferenceKnowledge::Known(references) = &self.recovery_references {
            if references.contains(generation) {
                reasons.insert(GenerationRetentionReason::RecoveryReference);
            }
        }
        if reasons.is_empty() {
            ReclamationDisposition::Eligible
        } else {
            ReclamationDisposition::Retained(reasons)
        }
    }

    /// Return the exact state compared by the atomic lifecycle port.
    #[must_use]
    pub fn snapshot(&self) -> GenerationLifecycleSnapshot {
        GenerationLifecycleSnapshot {
            published: self
                .published
                .as_ref()
                .map(|head| head.published_generation_head_digest().clone()),
            advertised: self
                .advertised
                .as_ref()
                .map(|head| head.advertised_generation_head_digest().clone()),
            active: self
                .active
                .as_ref()
                .map(|head| head.active_generation_head_digest().clone()),
            activation_fence: self
                .fence
                .as_ref()
                .map(|fence| fence.activation_fence_digest().clone()),
            retained_generations: self.retained_generations.clone(),
            session_pins: self
                .session_pins
                .iter()
                .map(|(session_admission_id, pin)| {
                    (
                        session_admission_id.clone(),
                        SessionGenerationPinSnapshot {
                            lease_digest: pin.lease_digest().clone(),
                            generation: pin.generation().clone(),
                        },
                    )
                })
                .collect(),
            rollback_targets: self.rollback_targets.clone(),
            recovery_references: self.recovery_references.clone(),
        }
    }

    /// Borrow generation identities retained by this lifecycle in digest order.
    pub(super) fn retained_generation_digests(
        &self,
    ) -> impl ExactSizeIterator<Item = &CompiledGenerationDigest> {
        self.retained_generations.iter()
    }

    /// Iterate the exact semantic pins used to rebuild process-local mechanics.
    ///
    /// This crate-private seam supplies the resident working-set projection. It
    /// grants no arena handle, mutation capability, or independently derived
    /// session authority.
    pub(super) fn session_pins(&self) -> impl ExactSizeIterator<Item = &SessionGenerationPin> {
        self.session_pins.values()
    }

    /// Borrow one exact semantic pin for prepared mechanical projection.
    pub(super) fn session_pin(
        &self,
        session_admission_id: &SessionAdmissionIdV1,
    ) -> Option<&SessionGenerationPin> {
        self.session_pins.get(session_admission_id)
    }

    /// Bind changed sparse heads to the sequence of their enclosing journal delta.
    pub(super) fn validate_envelope(
        &self,
        sequence: JournalSequenceV1,
        expected: &GenerationLifecycleSnapshot,
    ) -> Result<(), GenerationLifecycleError> {
        validate_lifecycle_head_sequences(
            self.published.as_ref().map(|value| {
                (
                    value.published_generation_head_digest(),
                    value.journal_sequence(),
                )
            }),
            self.advertised.as_ref().map(|value| {
                (
                    value.advertised_generation_head_digest(),
                    value.journal_sequence(),
                )
            }),
            self.active.as_ref().map(|value| {
                (
                    value.active_generation_head_digest(),
                    value.journal_sequence(),
                )
            }),
            expected,
            sequence,
        )
    }

    /// Borrow the generation selected for new sessions, when one is active.
    pub(crate) fn active_generation_digest(&self) -> Option<&CompiledGenerationDigest> {
        self.active
            .as_ref()
            .map(ActiveGenerationHeadV1::compiled_generation_digest)
    }

    fn transition(
        &self,
        delta: GenerationLifecycleDelta,
        successor: Self,
    ) -> GenerationLifecycleTransition {
        GenerationLifecycleTransition {
            expected: self.snapshot(),
            delta,
            successor,
        }
    }
}

/// A lifecycle transition violated the exact generation algebra.
#[derive(Debug, thiserror::Error)]
pub enum GenerationLifecycleError {
    /// One resident record failed its owning structural or digest validation.
    #[error(transparent)]
    InvalidRecord(#[from] InvalidResidentRecordV1),
    /// The embedded initial-v1 registry cannot resolve an owning lifecycle schema.
    #[error("generation lifecycle contract registry is invalid: {0}")]
    ContractRegistry(#[source] pale_contracts::SchemaError),
    /// A required prior lifecycle coordinate does not exist.
    #[error("generation lifecycle requires {0}")]
    MissingPrecondition(&'static str),
    /// Independently typed lifecycle coordinates do not identify one transition.
    #[error("generation lifecycle coordinate mismatch: {0}")]
    CoordinateMismatch(&'static str),
    /// A generation body is not retained by this lifecycle.
    #[error("compiled generation is not retained")]
    GenerationNotRetained,
    /// The generation remains referenced or recovery knowledge is incomplete.
    #[error("compiled generation is not eligible for reclamation")]
    GenerationNotReclaimable,
    /// A rollback target was not explicitly retained for rollback.
    #[error("compiled generation is not an explicit rollback target")]
    NotRollbackTarget,
    /// The same logical session and immutable lease were already pinned.
    #[error("session admission identity and lease are already pinned")]
    DuplicateSession,
    /// One logical session identity was presented with different immutable lease bytes.
    #[error("session admission identity is already bound to a different lease")]
    SessionIdentityConflict,
    /// A drain named no live logical session pin.
    #[error("session admission identity is not pinned")]
    UnknownSession,
    /// A private recovery projection has more than one byte representation.
    #[error("generation lifecycle recovery projection is noncanonical: {0}")]
    NonCanonicalRecovery(&'static str),
    /// Retained resident custody cannot resolve an exact recovery dependency.
    #[error("generation lifecycle recovery context is missing: {0}")]
    MissingRecoveryContext(&'static str),
    /// A resolved recovery dependency does not match the requested typed identity.
    #[error("generation lifecycle recovery context was substituted: {0}")]
    RecoveryContextSubstitution(&'static str),
    /// Physical custody could not return the requested exact historical lease bytes.
    #[error("session lease body custody failed: {0}")]
    SessionLeaseCustody(#[source] pale_ports::ControlStoreError),
    /// Retained lease bytes violate strict canonical JSON grammar.
    #[error("session lease body violates canonical JSON grammar: {0}")]
    SessionLeaseCanonical(#[source] pale_core::canon::CanonError),
    /// Retained lease bytes do not decode as the exact typed wire.
    #[error("session lease body does not decode as its typed wire: {0}")]
    SessionLeaseDecode(#[source] serde_json::Error),
    /// Retained lease bytes have another valid JSON representation.
    #[error("session lease body is not canonical JCS")]
    SessionLeaseNonCanonical,
    /// Retained lease schema coordinates cannot be resolved from the registry.
    #[error("session lease schema registry is invalid: {0}")]
    SessionLeaseContract(#[source] pale_contracts::SchemaError),
    /// Retained lease bytes exceed the explicit recovery ceiling.
    #[error("session lease body exceeds the recovery byte ceiling")]
    SessionLeaseBoundExceeded,
    /// Retained lease fields do not re-admit against their historical context.
    #[error("session lease body does not admit against historical context: {0}")]
    SessionLeaseAdmission(#[source] InvalidResidentRecordV1),
}

fn validate_published_successor(
    prior: Option<&PublishedGenerationHeadV1>,
    next: &PublishedGenerationHeadV1,
) -> Result<(), GenerationLifecycleError> {
    validate_head_successor(
        prior.map(PublishedGenerationHeadV1::journal_sequence),
        next.journal_sequence(),
        prior.map(PublishedGenerationHeadV1::published_generation_head_digest),
        next.predecessor_published_generation_head_digest().as_ref(),
        "published head predecessor",
    )
}

fn validate_advertised_successor(
    prior: Option<&AdvertisedGenerationHeadV1>,
    next: &AdvertisedGenerationHeadV1,
) -> Result<(), GenerationLifecycleError> {
    validate_head_successor(
        prior.map(AdvertisedGenerationHeadV1::journal_sequence),
        next.journal_sequence(),
        prior.map(AdvertisedGenerationHeadV1::advertised_generation_head_digest),
        next.predecessor_advertised_generation_head_digest()
            .as_ref(),
        "advertised head predecessor",
    )
}

fn validate_active_successor(
    prior: Option<&ActiveGenerationHeadV1>,
    next: &ActiveGenerationHeadV1,
) -> Result<(), GenerationLifecycleError> {
    validate_head_successor(
        prior.map(ActiveGenerationHeadV1::journal_sequence),
        next.journal_sequence(),
        prior.map(ActiveGenerationHeadV1::active_generation_head_digest),
        next.predecessor_active_generation_head_digest().as_ref(),
        "active head predecessor",
    )
}

fn validate_head_successor<D: PartialEq>(
    prior_sequence: Option<&pale_core::records::JournalSequenceV1>,
    next_sequence: &pale_core::records::JournalSequenceV1,
    prior_digest: Option<&D>,
    claimed_predecessor: Option<&D>,
    coordinate: &'static str,
) -> Result<(), GenerationLifecycleError> {
    match (prior_sequence, prior_digest, claimed_predecessor) {
        (None, None, None) => Ok(()),
        (Some(prior_sequence), Some(prior_digest), Some(claimed))
            if prior_digest == claimed && next_sequence.get() > prior_sequence.get() =>
        {
            Ok(())
        }
        _ => Err(GenerationLifecycleError::CoordinateMismatch(coordinate)),
    }
}

fn validate_fence_successor(
    prior: Option<&ActivationFenceV1>,
    next: &ActivationFenceV1,
) -> Result<(), GenerationLifecycleError> {
    match prior {
        None if next.activation_fence_generation().get() == 1
            && next.predecessor_activation_fence_digest().is_none() =>
        {
            Ok(())
        }
        Some(prior)
            if prior.activation_fence_id() == next.activation_fence_id()
                && next.predecessor_activation_fence_digest().as_ref()
                    == Some(prior.activation_fence_digest())
                && prior.activation_fence_generation().checked_successor()
                    == Some(*next.activation_fence_generation()) =>
        {
            Ok(())
        }
        _ => Err(GenerationLifecycleError::CoordinateMismatch(
            "activation fence predecessor",
        )),
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/lifecycle.rs"]
pub(super) mod tests;
