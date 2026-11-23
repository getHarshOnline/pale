//! Cohesive application assembly for resident authority.
//!
//! The root keeps immutable generation bodies, generation lifecycle, and
//! explicit desired process state, and last Event custody basis together. Every authority
//! mutation derives a pure sparse transition from the installed product state.
//! Where the transition changes a reconstructible working projection, the
//! application prepares that edit before append, appends exactly once, applies
//! the prepared edit, and only then publishes a process-local wake token. The
//! exact ordering law is rendered on [`ResidentApplication`].
//!
//! A failed append changes neither semantic projection nor the ring. A failed
//! notification after append is an owned durable-replay disposition, never a
//! failed commit. This module starts no process, opens no native store, and
//! performs no provider or application-native effect.

use std::collections::BTreeMap;

use pale_core::records::{
    ActivationFenceV1, ActiveGenerationHeadV1, AdvertisedGenerationHeadV1,
    CompiledGenerationDigest, HarnessActivityIdentity, JournalSequenceV1,
    NativeSessionCorrelationV1, PublishedGenerationHeadV1, SessionAdmissionIdV1,
    SessionAdmissionLeaseDigest, SessionAdmissionLeaseV1, SessionAdmissionLeaseWireV1,
};
use pale_kernel::{
    AdaptiveNodeSetPolicy, WakeBatch, WakeConsumerCursor, WakePublishError, WakeReadError,
    WakeToken,
};
use pale_ports::{
    AuthorityRecoveryReadLimits, AuthorityStoreCoordinate, ControlStoreErrorKind,
    DurableAuthorityStoreErrorKind, DurableAuthorityStorePort, SessionLeaseCustodyPort,
};

use super::authority::ResidentAuthorityState;
use super::coordination::{
    AuthorityCommitNotification, ResidentAuthorityCoordinationError, ResidentAuthorityCoordinator,
};
use super::generation::{
    AdmittedCompiledGeneration, AppliedRepositoryMaterialization, CompiledGenerationAdmissionError,
};
use super::journal::{
    recover_authority, AuthorityJournalError, AuthorityRecoveryCustody,
    AuthorityRecoveryQuarantine, AuthorityRecoveryQuarantineReason, DurableAuthorityJournal,
};
use super::lifecycle::{
    GenerationLifecycle, GenerationLifecycleError, GenerationLifecycleRecoveryResolver,
    GenerationLifecycleSnapshot, ReclamationDisposition, RecoveryReferenceKnowledge,
};
use super::process_reconciliation::{
    DesiredProcessState, PendingSupervisorReconciliation, SupervisorReconciliation,
};
use super::reconstruction::{
    reconstruct_resident, ReconstructedResident, ResidentColdRecoveryOutcome,
    ResidentGenerationInputSource, ResidentReconstructionError, ResidentReconstructionParameters,
};
use super::session_admission::{
    admit_ambient_session, AdmittedAmbientSession, AmbientSessionAdmissionContext,
    AmbientSessionAdmissionError, AmbientSessionAuthorityPort, RetainedAmbientSession,
};
use super::session_lease_custody::{
    recover_retained_session, retain_admitted_session_body, retain_session_lease_body,
    RetainedSessionRecoveryError,
};
use super::working_set::{
    ResidentNativeSessionLookup, ResidentSessionProjection, ResidentSessionWorkingSetError,
};
use crate::compile::Compilation;

/// Mutation root for one recovered resident authority store.
///
/// The generation map and authority coordinator cannot be separated. This keeps
/// lifecycle, desired-state and Event custody transitions, checkpointing, and cold
/// reconstruction over one exact journal frontier. This type proves neither
/// admission of the physical storage profile nor readiness of resident IPC,
/// providers, operations, or native effects.
///
/// In the law below, `A` is the installed journal state: `L` is generation
/// lifecycle, `D` is desired process state, and `E` is the last Event custody
/// basis. `P` is the reconstructible working projection and `R` is the wake
/// ring. Event custody retains record coordinates and byte hashes, not bodies
/// or permission to publish again.
///
/// The rendered law relates installed journal state, its
/// reconstructible process-local projection, and the wake ring at one live
/// journal coordinate. A semantic successor has one fully prepared mechanical
/// edit. Recovery uses an independent replay index and a checkpoint coordinate;
/// each entry carries a typed authority delta and complete predecessor/successor
/// fingerprints. Preparation precedes append; mechanical apply precedes wake
/// publication:
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/resident-authority-journal.display.mathml"
))]
pub struct ResidentApplication<'store> {
    coordinator: ResidentAuthorityCoordinator<'store>,
    session_lease_port: &'store dyn SessionLeaseCustodyPort,
    maximum_session_lease_body_bytes: usize,
    generations: BTreeMap<CompiledGenerationDigest, AdmittedCompiledGeneration>,
    sessions: ResidentSessionProjection,
}

/// Closed cold-open result.
///
/// Only [`Self::Ready`] grants authority mutation. [`Self::Degraded`]
/// carries a validated read-only prefix and its quarantine boundary, but no
/// coordinator, generation bodies, wake ring, Pale Runtime assembly, or effect surface.
#[must_use = "only the ready branch grants resident authority mutation"]
pub enum ResidentColdOpenOutcome<'store> {
    /// Journal recovery and every retained generation reconstructed exactly.
    Ready(Box<ResidentApplication<'store>>),
    /// Recovery retained a readable prefix while quarantining the durable suffix.
    Degraded(Box<ResidentReadOnlyDegradedState>),
}

/// Read-only semantic prefix recovered before one unresolved durable suffix.
///
/// The lifecycle and desired-state projections are the exact state through
/// [`ResidentRecoveryQuarantine::valid_through`]. It supports diagnosis and
/// bounded display only: this type deliberately owns no store port, journal
/// writer, generation bodies, wake ring, admission handle, or mutation method.
#[derive(Debug)]
pub struct ResidentReadOnlyDegradedState {
    lifecycle: GenerationLifecycleSnapshot,
    desired_process_state: Option<DesiredProcessState>,
    quarantine: ResidentRecoveryQuarantine,
}

impl ResidentReadOnlyDegradedState {
    /// Borrow the complete semantic snapshot of the validated historical prefix.
    #[must_use]
    pub const fn lifecycle(&self) -> &GenerationLifecycleSnapshot {
        &self.lifecycle
    }

    /// Return the desired process state retained in the validated prefix.
    #[must_use]
    pub const fn desired_process_state(&self) -> Option<DesiredProcessState> {
        self.desired_process_state
    }

    /// Borrow the exact boundary and reason that closed the unresolved suffix.
    #[must_use]
    pub const fn quarantine(&self) -> &ResidentRecoveryQuarantine {
        &self.quarantine
    }
}

/// Public classification of the first recovery layer that failed closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentRecoveryQuarantineClass {
    /// Adapter framing or integrity stopped before the entry.
    PhysicalIntegrity,
    /// The bytes violate strict canonical JSON grammar.
    CanonicalGrammar,
    /// Canonical JSON does not decode as the private typed journal entry.
    TypedDecode,
    /// Typed bytes are not their unique canonical JCS representation.
    NonCanonical,
    /// The private journal format is not the supported initial-v1 format.
    UnsupportedFormat,
    /// A delta's predecessor or successor fingerprint contradicts the accepted fold.
    FoldContradiction,
    /// The successor violates lifecycle semantics.
    Lifecycle,
}

/// Non-authoritative evidence for one quarantined journal suffix.
pub struct ResidentRecoveryQuarantine(AuthorityRecoveryQuarantine);

impl ResidentRecoveryQuarantine {
    /// Borrow the exact durable store incarnation that was inspected.
    #[must_use]
    pub fn store(&self) -> &AuthorityStoreCoordinate {
        self.0.store()
    }

    /// Return the final sequence validated physically and semantically.
    #[must_use]
    pub fn valid_through(&self) -> Option<JournalSequenceV1> {
        self.0.valid_through()
    }

    /// Return the first sequence excluded from the trusted prefix.
    #[must_use]
    pub fn first_untrusted_sequence(&self) -> JournalSequenceV1 {
        self.0.first_untrusted_sequence()
    }

    /// Return the durable head co-observed with the quarantined prefix.
    #[must_use]
    pub fn durable_head(&self) -> JournalSequenceV1 {
        self.0.durable_head()
    }

    /// Classify the validation layer that rejected the first untrusted entry.
    #[must_use]
    pub const fn class(&self) -> ResidentRecoveryQuarantineClass {
        match self.0.reason() {
            AuthorityRecoveryQuarantineReason::PhysicalIntegrity => {
                ResidentRecoveryQuarantineClass::PhysicalIntegrity
            }
            AuthorityRecoveryQuarantineReason::CanonicalGrammar(_) => {
                ResidentRecoveryQuarantineClass::CanonicalGrammar
            }
            AuthorityRecoveryQuarantineReason::TypedDecode(_) => {
                ResidentRecoveryQuarantineClass::TypedDecode
            }
            AuthorityRecoveryQuarantineReason::NonCanonical => {
                ResidentRecoveryQuarantineClass::NonCanonical
            }
            AuthorityRecoveryQuarantineReason::UnsupportedFormat => {
                ResidentRecoveryQuarantineClass::UnsupportedFormat
            }
            AuthorityRecoveryQuarantineReason::FoldContradiction => {
                ResidentRecoveryQuarantineClass::FoldContradiction
            }
            AuthorityRecoveryQuarantineReason::Lifecycle(_) => {
                ResidentRecoveryQuarantineClass::Lifecycle
            }
        }
    }
}

impl std::fmt::Debug for ResidentRecoveryQuarantine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentRecoveryQuarantine")
            .field("store", self.store())
            .field("valid_through", &self.valid_through())
            .field("first_untrusted_sequence", &self.first_untrusted_sequence())
            .field("durable_head", &self.durable_head())
            .field("class", &self.class())
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for ResidentRecoveryQuarantine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "resident recovery quarantined at journal sequence {:?} after {:?}: {}",
            self.first_untrusted_sequence(),
            self.valid_through(),
            self.0.reason()
        )
    }
}

impl std::error::Error for ResidentRecoveryQuarantine {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.reason())
    }
}

/// Wake-token disposition coupled to one proven durable authority commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentWakeDisposition {
    /// The exact durable coordinate entered the bounded process-local ring.
    Published(WakeToken),
    /// Authority committed, but consumers must replay the durable coordinate.
    DurableReplayRequired {
        /// Exact committed sequence that was not published.
        journal_sequence: JournalSequenceV1,
        /// Process-local publication failure; never evidence of append failure.
        reason: WakePublishError,
    },
}

/// Owned receipt for one durable resident authority transition.
#[derive(Debug)]
#[must_use = "a committed replay obligation must not be discarded implicitly"]
pub struct ResidentCommitReceipt {
    journal_sequence: JournalSequenceV1,
    wake: ResidentWakeDisposition,
    lifecycle: GenerationLifecycleSnapshot,
    desired_process_state: Option<DesiredProcessState>,
}

/// Failure to admit a session and retain its generation in this resident.
#[derive(Debug, thiserror::Error)]
pub enum ResidentSessionAdmissionError {
    /// Current authority, scope, time or lease admission failed before retention.
    #[error(transparent)]
    Admission(#[from] AmbientSessionAdmissionError),
    /// The admitted runtime names a different control store or incarnation.
    #[error("session runtime authority names a different resident store")]
    StoreMismatch,
    /// The resident could not prove retention; inspect the underlying custody state.
    #[error(transparent)]
    Retention(#[from] ResidentApplicationError),
}

impl ResidentCommitReceipt {
    /// Return the store-global sequence committed by this transition.
    #[must_use]
    pub const fn journal_sequence(&self) -> JournalSequenceV1 {
        self.journal_sequence
    }

    /// Return whether the process-local wake projection published successfully.
    #[must_use]
    pub const fn wake(&self) -> ResidentWakeDisposition {
        self.wake
    }

    /// Borrow the complete lifecycle installed by the durable append.
    #[must_use]
    pub const fn lifecycle(&self) -> &GenerationLifecycleSnapshot {
        &self.lifecycle
    }

    /// Return the desired process state installed at the committed frontier.
    #[must_use]
    pub const fn desired_process_state(&self) -> Option<DesiredProcessState> {
        self.desired_process_state
    }
}

/// Whether one desired-state request appended or reused its durable frontier.
#[derive(Debug)]
pub enum ResidentDesiredStatePersistence {
    /// The desired state changed and this exact transition committed.
    Committed(Box<ResidentCommitReceipt>),
    /// The same state was already installed, so no journal coordinate was spent.
    AlreadyCurrent {
        /// Current store-global frontier proving the retained state.
        journal_sequence: JournalSequenceV1,
    },
}

/// Post-persistence supervisor intents plus their durable-state disposition.
///
/// Only this carrier exposes sentinel and process directives. It is returned
/// after [`ResidentApplication::commit_supervisor_reconciliation`] either
/// commits the requested state or proves it already current.
#[derive(Debug)]
pub struct ResidentSupervisorReconciliation {
    persistence: ResidentDesiredStatePersistence,
    reconciliation: SupervisorReconciliation,
}

impl ResidentSupervisorReconciliation {
    /// Borrow the exact append or idempotent reuse disposition.
    #[must_use]
    pub const fn persistence(&self) -> &ResidentDesiredStatePersistence {
        &self.persistence
    }

    /// Borrow post-persistence sentinel and process directives.
    #[must_use]
    pub const fn reconciliation(&self) -> &SupervisorReconciliation {
        &self.reconciliation
    }
}

/// Stable failure class exposed without leaking private journal encoding types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentApplicationErrorClass {
    /// One immutable generation or applied proof did not admit.
    GenerationAdmission,
    /// A requested lifecycle transition violated the pure state algebra.
    Lifecycle,
    /// Durable custody proved that no requested mutation became authoritative.
    DurabilityNoMutation,
    /// Another writer owns a later durable frontier; explicit recovery is required.
    DurabilityConflict,
    /// A prior mutation may have committed; retry is unsafe before recovery.
    DurabilityOutcomeUnknown,
    /// A reconstructible checkpoint projection may have committed; journal
    /// authority remains unchanged and no effect retry question is implied.
    CheckpointProjectionOutcomeUnknown,
    /// Durable bytes or application replay semantics are untrustworthy.
    DurabilityIntegrity,
    /// A configured durable byte or record-count ceiling was exceeded.
    DurabilityBoundExceeded,
    /// Complete generation reconstruction failed.
    Reconstruction,
    /// Journal and wake-ring coordination failed.
    Coordination,
    /// A bounded process-local projection cannot admit another body.
    ProjectionBoundExceeded,
    /// Durable authority committed but its reconstructible projection requires recovery.
    ProjectionRecoveryRequired,
    /// A requested generation body is absent from resident custody.
    MissingGeneration,
    /// An immutable generation identity is already present.
    DuplicateGeneration,
}

/// Failure before an application operation can return its promised disposition.
#[derive(Debug)]
pub struct ResidentApplicationError(ResidentApplicationErrorKind);

impl std::fmt::Display for ResidentApplicationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, formatter)
    }
}

impl std::error::Error for ResidentApplicationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl ResidentApplicationError {
    /// Return the stable application-level failure class.
    #[must_use]
    pub fn class(&self) -> ResidentApplicationErrorClass {
        match &self.0 {
            ResidentApplicationErrorKind::Generation(_) => {
                ResidentApplicationErrorClass::GenerationAdmission
            }
            ResidentApplicationErrorKind::Lifecycle(_) => ResidentApplicationErrorClass::Lifecycle,
            ResidentApplicationErrorKind::Durability(source) => durability_class(source),
            ResidentApplicationErrorKind::SessionLeaseCustody(source) => {
                session_lease_custody_class(source)
            }
            ResidentApplicationErrorKind::Reconstruction(_) => {
                ResidentApplicationErrorClass::Reconstruction
            }
            ResidentApplicationErrorKind::Coordination(
                ResidentAuthorityCoordinationError::Journal(source),
            ) => durability_class(source),
            ResidentApplicationErrorKind::Coordination(
                ResidentAuthorityCoordinationError::FrontierMismatch { .. },
            ) => ResidentApplicationErrorClass::Coordination,
            ResidentApplicationErrorKind::Coordination(
                ResidentAuthorityCoordinationError::PublicationPending { .. }
                | ResidentAuthorityCoordinationError::PublicationRecoveryRequired { .. }
                | ResidentAuthorityCoordinationError::PublicationCoordinateMismatch { .. },
            ) => ResidentApplicationErrorClass::ProjectionRecoveryRequired,
            ResidentApplicationErrorKind::ResidentSessions(source) => {
                if source.requires_recovery() {
                    ResidentApplicationErrorClass::ProjectionRecoveryRequired
                } else {
                    ResidentApplicationErrorClass::ProjectionBoundExceeded
                }
            }
            ResidentApplicationErrorKind::CommittedProjection { .. } => {
                ResidentApplicationErrorClass::ProjectionRecoveryRequired
            }
            ResidentApplicationErrorKind::MissingGeneration { .. } => {
                ResidentApplicationErrorClass::MissingGeneration
            }
            ResidentApplicationErrorKind::DuplicateGeneration { .. } => {
                ResidentApplicationErrorClass::DuplicateGeneration
            }
        }
    }
}

/// Preserve the retry- and effect-relevant durable failure distinction.
fn durability_class(source: &AuthorityJournalError) -> ResidentApplicationErrorClass {
    match source {
        AuthorityJournalError::EventProjection(_) => {
            ResidentApplicationErrorClass::DurabilityNoMutation
        }
        AuthorityJournalError::EventCustody(error) => {
            use super::event_custody::journal::EventJournalCustodyError;
            match error.as_ref() {
                EventJournalCustodyError::BoundExceeded
                | EventJournalCustodyError::Candidate(
                    pale_ports::EventCustodyCandidateError::HistoryByteLimit,
                ) => ResidentApplicationErrorClass::DurabilityBoundExceeded,
                EventJournalCustodyError::Store(error) => control_store_class(error.kind()),
                EventJournalCustodyError::Correspondence
                | EventJournalCustodyError::Candidate(_) => {
                    ResidentApplicationErrorClass::DurabilityIntegrity
                }
            }
        }
        AuthorityJournalError::EventCommit(failure) => {
            use super::event_custody::EventCustodyCommitFailureReason;
            match failure.reason() {
                EventCustodyCommitFailureReason::Store(error) => control_store_class(error.kind()),
                EventCustodyCommitFailureReason::CommitExcludedByReadBack { .. } => {
                    ResidentApplicationErrorClass::DurabilityNoMutation
                }
                EventCustodyCommitFailureReason::RecoveryRequired { .. } => {
                    ResidentApplicationErrorClass::DurabilityIntegrity
                }
                EventCustodyCommitFailureReason::ReadBackFailed { .. } => {
                    ResidentApplicationErrorClass::DurabilityOutcomeUnknown
                }
            }
        }
        AuthorityJournalError::Store(source) => match source.kind() {
            DurableAuthorityStoreErrorKind::Conflict => {
                ResidentApplicationErrorClass::DurabilityConflict
            }
            DurableAuthorityStoreErrorKind::AmbiguousCommit => {
                ResidentApplicationErrorClass::DurabilityOutcomeUnknown
            }
            DurableAuthorityStoreErrorKind::BoundExceeded { .. } => {
                ResidentApplicationErrorClass::DurabilityBoundExceeded
            }
            DurableAuthorityStoreErrorKind::Unavailable => {
                ResidentApplicationErrorClass::DurabilityNoMutation
            }
            DurableAuthorityStoreErrorKind::Corrupt(_) => {
                ResidentApplicationErrorClass::DurabilityIntegrity
            }
        },
        AuthorityJournalError::WriterStale => ResidentApplicationErrorClass::DurabilityConflict,
        AuthorityJournalError::WriterRecoveryRequired => {
            ResidentApplicationErrorClass::DurabilityIntegrity
        }
        AuthorityJournalError::WriterCommitOutcomeUnknown => {
            ResidentApplicationErrorClass::DurabilityOutcomeUnknown
        }
        AuthorityJournalError::CheckpointProjectionOutcomeUnknown(_) => {
            ResidentApplicationErrorClass::CheckpointProjectionOutcomeUnknown
        }
        AuthorityJournalError::MissingJournalHead => {
            ResidentApplicationErrorClass::DurabilityNoMutation
        }
        AuthorityJournalError::Canonical(_)
        | AuthorityJournalError::Decode(_)
        | AuthorityJournalError::NonCanonical(_)
        | AuthorityJournalError::WrongFormat(_)
        | AuthorityJournalError::StoreCoordinateMismatch
        | AuthorityJournalError::SequenceMismatch { .. }
        | AuthorityJournalError::DurableHeadContradiction { .. }
        | AuthorityJournalError::FoldContradiction
        | AuthorityJournalError::UnchangedDesiredProcessState
        | AuthorityJournalError::CheckpointOrder
        | AuthorityJournalError::SequenceExhausted
        | AuthorityJournalError::Lifecycle(_) => ResidentApplicationErrorClass::DurabilityIntegrity,
    }
}

/// Preserve commit ambiguity and integrity distinctions for lease-body custody.
fn session_lease_custody_class(
    source: &super::custody_phase::DatabaseCommitFailureReason,
) -> ResidentApplicationErrorClass {
    use super::custody_phase::DatabaseCommitFailureReason;
    match source {
        DatabaseCommitFailureReason::Store(error) => control_store_class(error.kind()),
        DatabaseCommitFailureReason::CommitExcludedByReadBack { .. } => {
            ResidentApplicationErrorClass::DurabilityNoMutation
        }
        DatabaseCommitFailureReason::RecoveryRequired { .. } => {
            ResidentApplicationErrorClass::DurabilityIntegrity
        }
        DatabaseCommitFailureReason::ReadBackFailed { .. } => {
            ResidentApplicationErrorClass::DurabilityOutcomeUnknown
        }
    }
}

/// Preserve the same physical failure meaning for every custody owner.
fn control_store_class(kind: ControlStoreErrorKind) -> ResidentApplicationErrorClass {
    match kind {
        ControlStoreErrorKind::BoundExceeded => {
            ResidentApplicationErrorClass::DurabilityBoundExceeded
        }
        ControlStoreErrorKind::CandidateConflict => {
            ResidentApplicationErrorClass::DurabilityConflict
        }
        ControlStoreErrorKind::RecoveryRequired => {
            ResidentApplicationErrorClass::DurabilityIntegrity
        }
        ControlStoreErrorKind::OutcomeUnknown => {
            ResidentApplicationErrorClass::DurabilityOutcomeUnknown
        }
        ControlStoreErrorKind::Storage | ControlStoreErrorKind::TransactionExhausted => {
            ResidentApplicationErrorClass::DurabilityNoMutation
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ResidentApplicationErrorKind {
    /// Immutable compilation/materialization/generation admission failed.
    #[error(transparent)]
    Generation(CompiledGenerationAdmissionError),
    /// The pure lifecycle algebra rejected a requested transition.
    #[error(transparent)]
    Lifecycle(GenerationLifecycleError),
    /// Durable recovery or journal custody failed before coordination.
    #[error(transparent)]
    Durability(AuthorityJournalError),
    /// Exact session-lease body custody failed before its digest was journaled.
    #[error("session lease body was not proven durable: {0}")]
    SessionLeaseCustody(#[source] super::custody_phase::DatabaseCommitFailureReason),
    /// Complete retained-generation reconstruction failed.
    #[error(transparent)]
    Reconstruction(ResidentReconstructionError),
    /// Journal/wake coordination failed; journal causes remain nested and typed.
    #[error(transparent)]
    Coordination(ResidentAuthorityCoordinationError),
    /// Process-local resident-session mechanics rejected or lost coherence.
    #[error(transparent)]
    ResidentSessions(ResidentSessionWorkingSetError),
    /// Durable authority committed, but mechanical apply failed before visibility.
    #[error(
        "resident authority sequence {journal_sequence:?} committed but its projection requires recovery: {source}"
    )]
    CommittedProjection {
        journal_sequence: JournalSequenceV1,
        #[source]
        source: ResidentSessionWorkingSetError,
    },
    /// Resident custody has no immutable body for the requested identity.
    #[error("resident generation body is unavailable: {generation:?}")]
    MissingGeneration {
        generation: CompiledGenerationDigest,
    },
    /// Resident custody already contains the immutable identity.
    #[error("resident generation identity is already admitted: {generation:?}")]
    DuplicateGeneration {
        generation: CompiledGenerationDigest,
    },
}

impl From<CompiledGenerationAdmissionError> for ResidentApplicationError {
    fn from(source: CompiledGenerationAdmissionError) -> Self {
        Self(ResidentApplicationErrorKind::Generation(source))
    }
}

impl From<GenerationLifecycleError> for ResidentApplicationError {
    fn from(source: GenerationLifecycleError) -> Self {
        Self(ResidentApplicationErrorKind::Lifecycle(source))
    }
}

impl From<AuthorityJournalError> for ResidentApplicationError {
    fn from(source: AuthorityJournalError) -> Self {
        Self(ResidentApplicationErrorKind::Durability(source))
    }
}

impl From<ResidentReconstructionError> for ResidentApplicationError {
    fn from(source: ResidentReconstructionError) -> Self {
        Self(ResidentApplicationErrorKind::Reconstruction(source))
    }
}

impl From<ResidentAuthorityCoordinationError> for ResidentApplicationError {
    fn from(source: ResidentAuthorityCoordinationError) -> Self {
        Self(ResidentApplicationErrorKind::Coordination(source))
    }
}

impl From<ResidentSessionWorkingSetError> for ResidentApplicationError {
    fn from(source: ResidentSessionWorkingSetError) -> Self {
        Self(ResidentApplicationErrorKind::ResidentSessions(source))
    }
}

/// Complete bounded inputs for one resident cold-open attempt.
///
/// The value groups semantic recovery custody and explicit physical projection
/// policy without owning the durable byte port. It is consumed exactly once by
/// [`ResidentApplication::cold_open`].
pub struct ResidentColdOpenInput<'input> {
    store: AuthorityStoreCoordinate,
    limits: AuthorityRecoveryReadLimits,
    maximum_session_lease_body_bytes: usize,
    recovery_references: RecoveryReferenceKnowledge,
    resolver: &'input dyn GenerationLifecycleRecoveryResolver,
    generation_source: &'input mut dyn ResidentGenerationInputSource,
    parameters: ResidentReconstructionParameters,
}

impl<'input> ResidentColdOpenInput<'input> {
    /// Bind one store coordinate to its semantic and generation recovery custody.
    #[must_use]
    pub fn new(
        store: AuthorityStoreCoordinate,
        limits: AuthorityRecoveryReadLimits,
        maximum_session_lease_body_bytes: usize,
        recovery_references: RecoveryReferenceKnowledge,
        resolver: &'input dyn GenerationLifecycleRecoveryResolver,
        generation_source: &'input mut dyn ResidentGenerationInputSource,
        parameters: ResidentReconstructionParameters,
    ) -> Self {
        Self {
            store,
            limits,
            maximum_session_lease_body_bytes,
            recovery_references,
            resolver,
            generation_source,
            parameters,
        }
    }

    /// Borrow the exact store incarnation selected for this cold-open attempt.
    ///
    /// This coordinate lets an outer process owner prove its complete opened
    /// store set before any deployment recovery reads begin. It grants no store
    /// access and does not classify the recovered application.
    #[must_use]
    pub(in crate::resident) const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }
}

impl<'store> ResidentApplication<'store> {
    /// Recover durable authority and rebuild every retained generation projection.
    ///
    /// Degraded recovery returns its validated prefix before
    /// [`ResidentGenerationInputSource`] is invoked. Complete recovery consumes
    /// the exact one-shot inputs for the recovered retained set, rebuilds the
    /// bounded session-pin runtime projection, and resumes one inseparable
    /// generation/coordinator aggregate. Scratch projections remain outside this
    /// reconstruction surface.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] for durable read or replay failure,
    /// missing/substituted generation inputs, generation admission failure,
    /// invalid wake-ring parameters, a session-pin capacity, allocation, or
    /// integrity failure, or a reconstructed frontier mismatch. Event recovery
    /// requires the original journal entry and exact retained member bytes;
    /// `maximum_event_custody_bytes` bounds their combined bytes per read.
    pub fn cold_open(
        store_port: &'store dyn DurableAuthorityStorePort,
        session_lease_port: &'store dyn SessionLeaseCustodyPort,
        event_custody_port: &mut dyn pale_ports::EventCustodyPort,
        maximum_event_custody_bytes: usize,
        input: ResidentColdOpenInput<'_>,
    ) -> Result<ResidentColdOpenOutcome<'store>, ResidentApplicationError> {
        let ResidentColdOpenInput {
            store,
            limits,
            maximum_session_lease_body_bytes,
            recovery_references,
            resolver,
            generation_source,
            parameters,
        } = input;
        let genesis = ResidentAuthorityState::new(GenerationLifecycle::empty(recovery_references));
        let recovery = recover_authority(
            AuthorityRecoveryCustody {
                journal: store_port,
                session_leases: session_lease_port,
                events: event_custody_port,
                maximum_session_lease_bytes: maximum_session_lease_body_bytes,
                maximum_event_bytes: maximum_event_custody_bytes,
            },
            store,
            limits,
            genesis,
            resolver,
        )?;
        match reconstruct_resident(recovery, generation_source, parameters)? {
            ResidentColdRecoveryOutcome::Complete(reconstructed) => {
                let ReconstructedResident {
                    authority,
                    generations,
                    sessions,
                    wake_ring,
                } = *reconstructed;
                let journal = DurableAuthorityJournal::resume(store_port, limits, authority);
                let coordinator = ResidentAuthorityCoordinator::resume(journal, wake_ring)?;
                Ok(ResidentColdOpenOutcome::Ready(Box::new(Self {
                    coordinator,
                    session_lease_port,
                    maximum_session_lease_body_bytes,
                    generations,
                    sessions: ResidentSessionProjection::ready(sessions),
                })))
            }
            ResidentColdRecoveryOutcome::Degraded(degraded) => {
                let (prefix, quarantine) = degraded.into_parts();
                Ok(ResidentColdOpenOutcome::Degraded(Box::new(
                    ResidentReadOnlyDegradedState {
                        lifecycle: prefix.lifecycle().snapshot(),
                        desired_process_state: prefix.desired_process_state(),
                        quarantine: ResidentRecoveryQuarantine(quarantine),
                    },
                )))
            }
        }
    }

    /// Borrow the exact durable authority-store incarnation recovered at open.
    ///
    /// This coordinate distinguishes otherwise identical semantic frontiers in
    /// separate stores; it grants no access to the underlying storage port.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        self.coordinator.store()
    }

    pub(in crate::resident) fn verify_event_custody_members_at(
        &self,
        events: &mut dyn pale_ports::EventCustodyPort,
        sequence: JournalSequenceV1,
        required: &[pale_ports::EventCustodyHistoryMember],
        maximum_event_bytes: usize,
    ) -> Result<(), ResidentApplicationError> {
        self.coordinator.verify_event_custody_members_at(
            events,
            sequence,
            required,
            maximum_event_bytes,
        )?;
        Ok(())
    }

    /// Commit admitted Event content with its journal entry and publish a wake.
    ///
    /// The transaction and journal counters are checked independently. The
    /// returned continuation belongs to the existing Event custody owner; an
    /// ordinary exact replay grants none. This call does not send an event or
    /// perform a native application effect.
    ///
    /// # Errors
    ///
    /// Returns a projection or custody error before a proven commit. Conflict
    /// and unresolved commit outcomes close the writer until explicit recovery.
    pub fn commit_event_custody(
        &mut self,
        store: &mut dyn pale_ports::EventCustodyPort,
        transaction: pale_ports::ControlTransactionPreparation,
        input: super::event_custody::EventCustodyTransitionInput<'_>,
    ) -> Result<
        (
            super::event_custody::CompletedEventCustodyTransition,
            ResidentCommitReceipt,
        ),
        ResidentApplicationError,
    > {
        self.ensure_current_projection()?;
        let (completed, notification) =
            self.coordinator
                .commit_event_custody(store, transaction, input)?;
        Ok((completed, self.commit_receipt(notification)))
    }

    /// Admit one immutable genesis or active-successor candidate.
    ///
    /// The active generation, when present, is the sole successor basis. The
    /// candidate enters resident custody only after complete generation admission;
    /// this operation does not publish, advertise, activate, or durably retain it.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] when active predecessor custody is
    /// missing, generation admission fails, or the identity is already present.
    pub fn admit_generation(
        &mut self,
        compilation: Compilation,
        materializations: Vec<AppliedRepositoryMaterialization>,
        set_policy: AdaptiveNodeSetPolicy,
    ) -> Result<CompiledGenerationDigest, ResidentApplicationError> {
        self.ensure_current_projection()?;
        let admitted = if let Some(prior) = self.coordinator.lifecycle().active_generation_digest()
        {
            let predecessor = self.generations.get(prior).ok_or_else(|| {
                ResidentApplicationError(ResidentApplicationErrorKind::MissingGeneration {
                    generation: prior.clone(),
                })
            })?;
            AdmittedCompiledGeneration::admit_successor(
                predecessor,
                compilation,
                materializations,
                set_policy,
            )?
        } else {
            AdmittedCompiledGeneration::admit(compilation, materializations, set_policy)?
        };
        let digest = admitted.record().compiled_generation_digest().clone();
        if self.generations.contains_key(&digest) {
            return Err(ResidentApplicationError(
                ResidentApplicationErrorKind::DuplicateGeneration { generation: digest },
            ));
        }
        self.generations.insert(digest.clone(), admitted);
        Ok(digest)
    }

    /// Durably publish one already-admitted immutable generation.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] when the head names no resident body,
    /// violates publication lineage, or cannot be appended exactly once. A stale
    /// or ambiguous writer requires a fresh [`Self::cold_open`] before mutation.
    pub fn publish(
        &mut self,
        head: PublishedGenerationHeadV1,
    ) -> Result<ResidentCommitReceipt, ResidentApplicationError> {
        let generation = head.compiled_generation_digest().clone();
        let transition = self.coordinator.lifecycle().publish(head)?;
        self.require_generation(&generation)?;
        self.commit(transition)
    }

    /// Durably advertise the exact current publication.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] when the head names no resident body,
    /// disagrees with current publication, or cannot be appended exactly once. A
    /// stale or ambiguous writer requires a fresh [`Self::cold_open`].
    pub fn advertise(
        &mut self,
        head: AdvertisedGenerationHeadV1,
    ) -> Result<ResidentCommitReceipt, ResidentApplicationError> {
        self.require_generation(head.compiled_generation_digest())?;
        let transition = self.coordinator.lifecycle().advertise(head)?;
        self.commit(transition)
    }

    /// Durably activate the current advertisement with candidate compatibility.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] for missing generation custody,
    /// incompatible derivation, advertisement/fence mismatch, or an unproven
    /// append. Stale or ambiguous append outcomes close the writer for recovery.
    pub fn activate(
        &mut self,
        head: ActiveGenerationHeadV1,
        fence: ActivationFenceV1,
    ) -> Result<ResidentCommitReceipt, ResidentApplicationError> {
        let generation = self.require_generation(head.compiled_generation_digest())?;
        let transition = self.coordinator.lifecycle().activate(
            head,
            fence,
            generation.activation_compatibility(),
        )?;
        self.commit(transition)
    }

    /// Durably activate retained prior content through the rollback-only path.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] when the body or rollback retention
    /// is absent, lifecycle coordinates disagree, or append is not proven. An
    /// ambiguous append requires a fresh [`Self::cold_open`].
    pub fn rollback(
        &mut self,
        head: ActiveGenerationHeadV1,
        fence: ActivationFenceV1,
    ) -> Result<ResidentCommitReceipt, ResidentApplicationError> {
        self.require_generation(head.compiled_generation_digest())?;
        let transition = self.coordinator.lifecycle().rollback(head, fence)?;
        self.commit(transition)
    }

    /// Durably retain one resident body as an explicit rollback target.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] when body/lifecycle custody is absent
    /// or append is not proven. A stale or ambiguous writer remains recovery-only.
    pub fn retain_for_rollback(
        &mut self,
        generation: CompiledGenerationDigest,
    ) -> Result<ResidentCommitReceipt, ResidentApplicationError> {
        self.require_generation(&generation)?;
        let transition = self
            .coordinator
            .lifecycle()
            .retain_for_rollback(generation)?;
        self.commit(transition)
    }

    /// Admit a caller's session and retain its exact generation in this resident.
    ///
    /// Checks current ambient authority first, joins its runtime to this store,
    /// then persists the lease and generation pin through [`Self::pin_session`].
    /// Returns the admitted session only after the pin is committed and the
    /// resident projection is coherent. Later operation admission, dispatch and
    /// release must still revalidate authority; this does not extend any lease.
    /// It neither authenticates a transport peer nor grants source disclosure.
    ///
    /// # Errors
    /// Authority or store mismatch changes no retention state. Retention failure
    /// can leave durable lease bytes or an uncertain journal append: preserve its
    /// typed custody outcome and use existing recovery, not a blind retry.
    pub fn admit_session<P: AmbientSessionAuthorityPort>(
        &mut self,
        wire: SessionAdmissionLeaseWireV1,
        context: &AmbientSessionAdmissionContext<'_>,
        authority: &mut P,
    ) -> Result<(AdmittedAmbientSession, ResidentCommitReceipt), ResidentSessionAdmissionError>
    {
        self.ensure_current_projection()?;
        let admitted = admit_ambient_session(wire, context, authority)?;
        let runtime = context
            .runtime_authority
            .as_ref()
            .ok_or(AmbientSessionAdmissionError::RuntimeAuthorityAbsent)?
            .authority;
        if runtime.control_store_id() != self.store().store_id()
            || runtime.control_incarnation_id() != self.store().incarnation_id()
        {
            return Err(ResidentSessionAdmissionError::StoreMismatch);
        }
        let receipt = self.pin_admitted_session(&admitted)?;
        Ok((admitted, receipt))
    }

    /// Durably retain one lease's exact current generation, head, and fence.
    ///
    /// This lower-level path has no private admission basis to preserve. A
    /// native hook cannot later recover ambient authority from this pin. Use
    /// [`Self::admit_session`] when the application performed the admission.
    ///
    /// This persists generation-retention evidence only. It does not establish
    /// current ambient grant or runtime authority, session-floor freshness,
    /// revocation state, time validity, or readiness to execute the session.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] for missing body or active state,
    /// mixed/duplicate lease coordinates, or an unproven append. A stale or
    /// ambiguous writer requires a fresh [`Self::cold_open`].
    pub fn pin_session(
        &mut self,
        lease: &SessionAdmissionLeaseV1,
    ) -> Result<ResidentCommitReceipt, ResidentApplicationError> {
        self.pin_session_with_optional_basis(lease, None)
    }

    /// Retain an application-admitted session with its private recovery basis.
    fn pin_admitted_session(
        &mut self,
        session: &AdmittedAmbientSession,
    ) -> Result<ResidentCommitReceipt, ResidentApplicationError> {
        self.pin_session_with_optional_basis(session.lease(), Some(session))
    }

    /// Apply one session pin after retaining every supplied recovery input.
    fn pin_session_with_optional_basis(
        &mut self,
        lease: &SessionAdmissionLeaseV1,
        admitted: Option<&AdmittedAmbientSession>,
    ) -> Result<ResidentCommitReceipt, ResidentApplicationError> {
        self.ensure_current_projection()?;
        self.require_generation(lease.compiled_generation_digest())?;
        let transition = self.coordinator.lifecycle().pin_session(lease)?;
        let pin = transition
            .successor()
            .session_pin(lease.session_admission_id())
            .cloned()
            .ok_or_else(|| {
                self.sessions
                    .require_recovery(ResidentSessionWorkingSetError::Contradiction)
            })?;
        let prepared = self.sessions.prepare_pin(pin)?;
        let retention = match admitted {
            Some(session) => retain_admitted_session_body(
                self.session_lease_port,
                self.coordinator.store(),
                session,
                self.maximum_session_lease_body_bytes,
            ),
            None => retain_session_lease_body(
                self.session_lease_port,
                self.coordinator.store(),
                lease,
                self.maximum_session_lease_body_bytes,
            ),
        };
        retention.map_err(|source| {
            ResidentApplicationError(ResidentApplicationErrorKind::SessionLeaseCustody(source))
        })?;
        self.commit_session_pin_projection(transition, prepared)
    }

    /// Durably drain the exact lease bound to one logical session identity.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] when the logical session identity is
    /// absent, its retained lease digest differs, or the append is not proven. A
    /// stale or ambiguous writer remains recovery-only.
    pub fn drain_session(
        &mut self,
        session_admission_id: &SessionAdmissionIdV1,
        lease_digest: &SessionAdmissionLeaseDigest,
    ) -> Result<ResidentCommitReceipt, ResidentApplicationError> {
        self.ensure_current_projection()?;
        let transition = self
            .coordinator
            .lifecycle()
            .drain_session(session_admission_id, lease_digest)?;
        let prepared = self
            .sessions
            .prepare_drain(session_admission_id, lease_digest)?;
        self.commit_session_pin_projection(transition, prepared)
    }

    /// Persist one pending desired state before exposing its process directives.
    ///
    /// An idempotent request reuses the current durable frontier without
    /// appending or publishing another wake token. A changed request is appended
    /// exactly once; only proven append success releases the stop-sentinel and
    /// service directives carried by the returned reconciliation.
    /// Running plans must originate from the current active generation in resident
    /// custody, including when the desired state is already running. Stopped
    /// plans retain their origin without requiring it to remain active: cleanup
    /// of older children still needs independent native identity and authority.
    /// This check binds compiled content, not a particular activation of that
    /// content, and the returned intents do not authorize later process effects.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] when a running plan lacks its exact
    /// active generation or body, or durable custody cannot prove the changed
    /// desired state. A stale or ambiguous writer requires a fresh
    /// [`Self::cold_open`] before another reconciliation attempt.
    pub fn commit_supervisor_reconciliation(
        &mut self,
        pending: PendingSupervisorReconciliation,
    ) -> Result<ResidentSupervisorReconciliation, ResidentApplicationError> {
        self.ensure_current_projection()?;
        let desired = pending.desired_process_state();
        if desired == DesiredProcessState::Running {
            let active = self
                .coordinator
                .lifecycle()
                .active_generation_digest()
                .ok_or(GenerationLifecycleError::MissingPrecondition(
                    "active compiled generation for supervisor running state",
                ))?;
            if active != pending.compiled_generation() {
                return Err(GenerationLifecycleError::CoordinateMismatch(
                    "supervisor plan does not name the active compiled generation",
                )
                .into());
            }
            self.require_generation(active)?;
        }
        let persistence = if self.coordinator.desired_process_state() == Some(desired) {
            let journal_sequence = self
                .coordinator
                .journal_head()
                .ok_or(AuthorityJournalError::MissingJournalHead)?;
            ResidentDesiredStatePersistence::AlreadyCurrent { journal_sequence }
        } else {
            let notification = self.coordinator.commit_desired_process_state(desired)?;
            ResidentDesiredStatePersistence::Committed(Box::new(self.commit_receipt(notification)))
        };
        Ok(ResidentSupervisorReconciliation {
            persistence,
            reconciliation: pending.into_committed(),
        })
    }

    /// Durably remove an eligible retained coordinate, then drop its body.
    ///
    /// On append failure the body is restored. On append success it is dropped
    /// even when wake publication requires durable replay, because authority has
    /// already committed the reclamation.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] for absent body custody, live or
    /// unknown references, or an unproven append. Ambiguous append restores the
    /// body conservatively and closes the writer until [`Self::cold_open`].
    pub fn reclaim_generation(
        &mut self,
        generation: &CompiledGenerationDigest,
    ) -> Result<ResidentCommitReceipt, ResidentApplicationError> {
        self.ensure_current_projection()?;
        self.require_generation(generation)?;
        let transition = self.coordinator.lifecycle().reclaim(generation)?;
        let retained = self.generations.remove(generation).ok_or_else(|| {
            ResidentApplicationError(ResidentApplicationErrorKind::MissingGeneration {
                generation: generation.clone(),
            })
        })?;
        match self.commit(transition) {
            Ok(receipt) => Ok(receipt),
            Err(error) => {
                self.generations.insert(generation.clone(), retained);
                Err(error)
            }
        }
    }

    /// Persist a reconstructible checkpoint of the current durable lifecycle.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] before the first commit, when bounded
    /// checkpoint encoding fails, or when durable custody rejects the projection.
    /// Checkpoints never advance authority, so ambiguity does not authorize state.
    /// A head conflict nevertheless closes this stale writer until
    /// [`Self::cold_open`] reconstructs the current journal frontier.
    pub fn checkpoint(&mut self) -> Result<(), ResidentApplicationError> {
        self.ensure_current_projection()?;
        self.coordinator.store_checkpoint()?;
        Ok(())
    }

    /// Borrow one immutable generation body by canonical digest.
    #[must_use]
    pub fn generation(
        &self,
        digest: &CompiledGenerationDigest,
    ) -> Option<&AdmittedCompiledGeneration> {
        self.generations.get(digest)
    }

    /// Borrow the body selected at this writer's last proven active frontier.
    ///
    /// This recovery-oriented view may be a prior prefix after a stale or
    /// ambiguous append; [`ObservationSnapshot::from_resident`](crate::observation::ObservationSnapshot::from_resident)
    /// gates current-state projection on writer certainty.
    #[must_use]
    pub fn active_generation(&self) -> Option<&AdmittedCompiledGeneration> {
        self.coordinator
            .lifecycle()
            .active_generation_digest()
            .and_then(|digest| self.generations.get(digest))
    }

    /// Iterate every body currently held by resident custody in digest order.
    ///
    /// Includes unpublished candidates. These bodies are not all durably
    /// retained and must not supply journal-bound observation facts.
    #[must_use]
    pub fn generation_digests(&self) -> impl ExactSizeIterator<Item = &CompiledGenerationDigest> {
        self.generations.keys()
    }

    /// Borrow the committed retention set without copying lifecycle or session state.
    ///
    /// Current observations must first pass [`Self::ensure_current_projection`].
    pub(crate) fn retained_generation_digests(
        &self,
    ) -> impl ExactSizeIterator<Item = &CompiledGenerationDigest> {
        self.coordinator.lifecycle().retained_generation_digests()
    }

    /// Return the semantic lifecycle at this writer's last proven frontier.
    ///
    /// This recovery-oriented snapshot may be a prior prefix after a stale or
    /// ambiguous append. Use coherence-gated observations for current-state claims.
    #[must_use]
    pub fn lifecycle(&self) -> GenerationLifecycleSnapshot {
        self.coordinator.lifecycle().snapshot()
    }

    /// Return the exact owned process-local projection of all durable session pins.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] when the durable writer is stale or
    /// uncertain, or projection integrity requires cold reconstruction.
    pub fn session_pins(
        &self,
    ) -> Result<
        BTreeMap<SessionAdmissionIdV1, super::SessionGenerationPinSnapshot>,
        ResidentApplicationError,
    > {
        self.ensure_current_projection()?;
        self.sessions.snapshot().map_err(Into::into)
    }

    /// Return the ordered session identities retaining one compiled generation.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentApplicationError`] when the durable writer is stale or
    /// uncertain, or projection integrity requires cold reconstruction.
    pub fn sessions_retaining_generation(
        &self,
        generation: &CompiledGenerationDigest,
    ) -> Result<Vec<SessionAdmissionIdV1>, ResidentApplicationError> {
        self.ensure_current_projection()?;
        self.sessions
            .sessions_retaining(generation)
            .map_err(Into::into)
    }

    /// Return desired process state at this writer's last proven durable frontier.
    ///
    /// This recovery-oriented value may be a prior prefix after a stale or
    /// ambiguous append; coherence-gated observation must precede current claims.
    #[must_use]
    pub const fn desired_process_state(&self) -> Option<DesiredProcessState> {
        self.coordinator.desired_process_state()
    }

    /// Classify references at this writer's last proven semantic frontier.
    ///
    /// Mutation rechecks writer and projection certainty before using this
    /// recovery-oriented classification.
    #[must_use]
    pub fn reclamation_disposition(
        &self,
        generation: &CompiledGenerationDigest,
    ) -> ReclamationDisposition {
        self.coordinator
            .lifecycle()
            .reclamation_disposition(generation)
    }

    /// Return the latest store-global sequence proven durable by this writer.
    #[must_use]
    pub const fn journal_head(&self) -> Option<JournalSequenceV1> {
        self.coordinator.journal_head()
    }

    /// Return the current process-local wake cursor for a new diagnostic read.
    #[must_use]
    pub const fn wake_cursor(&self) -> WakeConsumerCursor {
        self.coordinator.wake_cursor()
    }

    /// Read unread wake tokens without mutating another consumer's cursor.
    ///
    /// # Errors
    ///
    /// Returns [`WakeReadError`] when the cursor is stale, overwritten, ahead,
    /// or inconsistent; the error names the exact durable replay frontier.
    pub fn wake_after(&self, cursor: WakeConsumerCursor) -> Result<WakeBatch<'_>, WakeReadError> {
        self.coordinator.wake_ring().read_after(cursor)
    }

    fn require_generation(
        &self,
        generation: &CompiledGenerationDigest,
    ) -> Result<&AdmittedCompiledGeneration, ResidentApplicationError> {
        self.generations.get(generation).ok_or_else(|| {
            ResidentApplicationError(ResidentApplicationErrorKind::MissingGeneration {
                generation: generation.clone(),
            })
        })
    }

    fn commit(
        &mut self,
        transition: super::GenerationLifecycleTransition,
    ) -> Result<ResidentCommitReceipt, ResidentApplicationError> {
        self.ensure_current_projection()?;
        let notification = self.coordinator.commit_transition(transition)?;
        Ok(self.commit_receipt(notification))
    }

    /// Append, mechanically apply, and only then publish one prepared pin edit.
    fn commit_session_pin_projection(
        &mut self,
        transition: super::GenerationLifecycleTransition,
        prepared: super::working_set::PreparedResidentSessionMutation,
    ) -> Result<ResidentCommitReceipt, ResidentApplicationError> {
        let appended = self.coordinator.append_transition(transition)?;
        if let Err(source) = self.sessions.apply(prepared) {
            let journal_sequence = self.coordinator.require_recovery_after_apply(appended);
            return Err(ResidentApplicationError(
                ResidentApplicationErrorKind::CommittedProjection {
                    journal_sequence,
                    source,
                },
            ));
        }
        let notification = self.coordinator.publish_appended(appended)?;
        Ok(self.commit_receipt(notification))
    }

    /// Project one proven append and its wake disposition into the public receipt.
    fn commit_receipt(&self, notification: AuthorityCommitNotification) -> ResidentCommitReceipt {
        let journal_sequence = notification.journal_sequence();
        let wake = match notification {
            AuthorityCommitNotification::Published(token) => {
                ResidentWakeDisposition::Published(token)
            }
            AuthorityCommitNotification::DurableReplayRequired {
                journal_sequence,
                reason,
            } => ResidentWakeDisposition::DurableReplayRequired {
                journal_sequence,
                reason,
            },
        };
        let lifecycle = self.coordinator.lifecycle().snapshot();
        let desired_process_state = self.coordinator.desired_process_state();
        ResidentCommitReceipt {
            journal_sequence,
            wake,
            lifecycle,
            desired_process_state,
        }
    }

    /// Keep every resident mutation behind a coherent process-local projection.
    pub(crate) fn ensure_current_projection(&self) -> Result<(), ResidentApplicationError> {
        self.coordinator.require_current_projection()?;
        self.sessions.require_ready().map_err(Into::into)
    }

    /// Recover one exact live session named by its signed request coordinates.
    ///
    /// The lifecycle pin is checked before session custody is read. This method
    /// selects no default session and performs no current-authority admission;
    /// the caller must still rebuild the current context and call
    /// [`RetainedAmbientSession::revalidate`].
    pub(in crate::resident) fn recover_exact_retained_session(
        &self,
        session_admission_id: &SessionAdmissionIdV1,
        lease_digest: &SessionAdmissionLeaseDigest,
        historical: &dyn GenerationLifecycleRecoveryResolver,
    ) -> Result<RetainedAmbientSession, ExactRetainedSessionRecoveryError> {
        self.ensure_current_projection()?;
        let pin = self
            .coordinator
            .lifecycle()
            .session_pin(session_admission_id)
            .ok_or(ExactRetainedSessionRecoveryError::Missing)?;
        if pin.lease_digest() != lease_digest {
            return Err(ExactRetainedSessionRecoveryError::LeaseSubstitution);
        }
        recover_retained_session(
            self.session_lease_port,
            self.coordinator.store(),
            lease_digest,
            self.maximum_session_lease_body_bytes,
            historical,
        )
        .map_err(Into::into)
    }

    /// Recover and currently re-admit the unique live session for one host activity.
    ///
    /// This is an application-internal ingress fence. It selects no default
    /// session, creates no operation, and performs no native effect.
    pub(in crate::resident) fn admit_native_activity_session<P: AmbientSessionAuthorityPort>(
        &self,
        activity: &HarnessActivityIdentity,
        historical: &dyn GenerationLifecycleRecoveryResolver,
        context: &AmbientSessionAdmissionContext<'_>,
        authority: &mut P,
    ) -> Result<AdmittedAmbientSession, NativeActivitySessionAdmissionError> {
        self.ensure_current_projection()?;
        let digest = match self.sessions.session_by_native_activity(activity)? {
            ResidentNativeSessionLookup::Missing => {
                return Err(NativeActivitySessionAdmissionError::Missing);
            }
            ResidentNativeSessionLookup::Exact(digest) => digest,
            ResidentNativeSessionLookup::Ambiguous => {
                return Err(NativeActivitySessionAdmissionError::Ambiguous);
            }
        };
        let retained = recover_retained_session(
            self.session_lease_port,
            self.coordinator.store(),
            &digest,
            self.maximum_session_lease_body_bytes,
            historical,
        )?;
        match retained.lease().native_session_correlation() {
            NativeSessionCorrelationV1::Observed {
                native_session_identity,
            } if native_session_identity == activity => {}
            _ => return Err(NativeActivitySessionAdmissionError::CorrelationSubstitution),
        }
        retained.revalidate(context, authority).map_err(Into::into)
    }
}

/// One exact signed session coordinate could not recover its retained basis.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum ExactRetainedSessionRecoveryError {
    /// The resident projection is stale or inconsistent.
    #[error(transparent)]
    Application(#[from] ResidentApplicationError),
    /// No live lifecycle pin carries the supplied session identity.
    #[error("exact retained session is absent")]
    Missing,
    /// The live session pin carries another immutable lease digest.
    #[error("session identity and lease digest do not identify one retained session")]
    LeaseSubstitution,
    /// Exact retained-session custody or historical admission failed.
    #[error(transparent)]
    Recovery(#[from] RetainedSessionRecoveryError),
}

/// One native activity could not recover a unique, currently admitted session.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum NativeActivitySessionAdmissionError {
    /// The resident projection is stale or inconsistent.
    #[error(transparent)]
    Application(#[from] ResidentApplicationError),
    /// No live session carries the supplied native activity identity.
    #[error("native activity has no live admitted session")]
    Missing,
    /// More than one live session carries the supplied native activity identity.
    #[error("native activity identifies more than one live admitted session")]
    Ambiguous,
    /// The recovered lease differs from the live projection correlation.
    #[error("recovered session changed its native activity correlation")]
    CorrelationSubstitution,
    /// Exact retained-session custody or historical admission failed.
    #[error(transparent)]
    Recovery(#[from] RetainedSessionRecoveryError),
    /// Current authority no longer admits the historically retained session.
    #[error(transparent)]
    Admission(#[from] AmbientSessionAdmissionError),
    /// The reconstructible session projection is not coherent.
    #[error(transparent)]
    Projection(#[from] ResidentSessionWorkingSetError),
}

#[cfg(test)]
#[path = "../../tests/internal/resident/application.rs"]
pub(super) mod tests;
