//! Private fold and write coordination for the resident authority journal.
//!
//! Each admitted entry applies one deterministic transition and binds both
//! semantic boundaries according to the journal recurrence owned by
//! [`crate::resident::ResidentApplication`].
//!
//! The entry carries the predecessor fingerprint, closed typed delta, and
//! successor fingerprint. Replay verifies the predecessor, reapplies the
//! ordinary owning law, then verifies the complete successor product state.
//! Complete authority images are reserved for checkpoints. Opaque byte custody
//! is delegated to [`pale_ports::DurableAuthorityStorePort`]; adapter framing,
//! checksums, and persistence profiles never enter this algebra.
//!
//! A conflict or ambiguous append closes the current writer pending explicit
//! recovery. Neither is retried or turned into authority by querying storage
//! and guessing which outcome occurred.

mod recovery;

use pale_core::canon;
use pale_core::records::{ControlIncarnationIdV1, ControlStoreIdV1, JournalSequenceV1};
use pale_ports::{
    AuthorityRecoveryReadLimits, AuthorityStoreCoordinate, BoundedAuthorityEntryBytes,
    DurableAuthorityAppendFailure, DurableAuthorityCheckpointFailure, DurableAuthorityStoreError,
    DurableAuthorityStoreErrorKind, DurableAuthorityStorePort, EventCustodyHistoryMember,
    EventCustodyPort, JournalTailPageBoundary, JournalTailPageRead, PreparedAuthorityAppend,
    PreparedRecoveryCheckpoint, SequencedAuthorityEntry, StoredRecoveryCheckpoint,
};
use serde::{Deserialize, Serialize};

use super::authority::{
    ResidentAuthorityDelta, ResidentAuthorityState, ResidentAuthorityStateDigest,
    ResidentAuthorityTransition, ResidentAuthorityTransitionError,
};
use super::lifecycle::{
    GenerationLifecycle, GenerationLifecycleError, GenerationLifecycleTransition,
};
use super::session_lease_custody::SessionLeaseRecoveryContext;

pub(crate) use recovery::encode_recovery_checkpoint;
pub(super) use recovery::{
    recover_authority, AuthorityRecoveryCustody, AuthorityRecoveryOutcome,
    AuthorityRecoveryQuarantine, AuthorityRecoveryQuarantineReason, DegradedAuthorityState,
    RecoveredAuthorityState,
};

/// Private discriminator that prevents journal bytes from decoding as another record family.
///
/// The marker selects this application's initial decoder only. It is not a
/// public schema identity, integrity checksum, admission proof, or release.
const AUTHORITY_JOURNAL_ENTRY_FORMAT_V1: &str = "pale-authority-journal-entry.v1";
/// Private discriminator that separates reconstructible checkpoints from journal entries.
///
/// The marker selects this application's initial decoder only. It does not
/// make a checkpoint authoritative or specify adapter framing and checksums.
const AUTHORITY_RECOVERY_IMAGE_FORMAT_V1: &str = "pale-authority-recovery-image.v1";

/// One application-owned canonical transition envelope in the authority journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AuthorityJournalEntry {
    /// Private decoder discriminator; not a public schema identity.
    format: String,
    /// Stable store identity bound into the canonical entry bytes.
    control_store_id: ControlStoreIdV1,
    /// Physical store incarnation that accepted this entry.
    control_incarnation_id: ControlIncarnationIdV1,
    /// Store-global sequence at which this transition must fold.
    journal_sequence: JournalSequenceV1,
    /// Complete semantic product-state fingerprint before the sparse delta.
    predecessor: ResidentAuthorityStateDigest,
    /// Exact closed sparse transition carried by this entry.
    delta: ResidentAuthorityDelta,
    /// Complete semantic product-state fingerprint after the sparse delta.
    successor: ResidentAuthorityStateDigest,
}

impl AuthorityJournalEntry {
    /// Borrow the custody basis only when this entry commits Event members.
    fn event_custody_basis(&self) -> Option<&super::event_custody::journal::EventCustodyBasis> {
        match &self.delta {
            ResidentAuthorityDelta::EventCustody { basis } => Some(basis),
            _ => None,
        }
    }
    /// Bind one pure authority transition to an exact store-global sequence.
    fn from_transition(
        store: &AuthorityStoreCoordinate,
        sequence: JournalSequenceV1,
        current: &ResidentAuthorityState,
        transition: ResidentAuthorityTransition,
    ) -> Result<(Self, ResidentAuthorityState), AuthorityJournalError> {
        let predecessor = ResidentAuthorityStateDigest::from_state(current)?;
        let (delta, successor) = transition.into_journal_parts(current, sequence)?;
        let successor_digest = ResidentAuthorityStateDigest::from_state(&successor)?;
        Ok((
            Self {
                format: AUTHORITY_JOURNAL_ENTRY_FORMAT_V1.to_owned(),
                control_store_id: store.store_id().clone(),
                control_incarnation_id: store.incarnation_id().clone(),
                journal_sequence: sequence,
                predecessor,
                delta,
                successor: successor_digest,
            },
            successor,
        ))
    }

    /// Encode the entry once as strict canonical JCS under the configured bound.
    fn encode(
        &self,
        maximum_bytes: usize,
    ) -> Result<BoundedAuthorityEntryBytes, AuthorityJournalError> {
        let canonical = canon::to_canonical_string(self)?;
        BoundedAuthorityEntryBytes::new(canonical.into_bytes(), maximum_bytes)
            .map_err(AuthorityJournalError::Store)
    }

    /// Bind a re-admitted checkpoint to this retained journal entry's successor.
    ///
    /// The caller has already proved canonical bytes and exact store/sequence
    /// custody with [`Self::decode`]. This checks the delta's own sequence law
    /// before comparing the existing semantic fingerprint; it neither replays
    /// the predecessor nor authenticates the journal's origin or freshness.
    fn matches_checkpoint(
        &self,
        authority: &ResidentAuthorityState,
    ) -> Result<bool, AuthorityJournalError> {
        self.delta.validate_envelope(self.journal_sequence)?;
        Ok(ResidentAuthorityStateDigest::from_state(authority)? == self.successor)
    }

    /// Decode and re-encode one opaque entry before admitting its coordinates.
    fn decode(
        entry: &SequencedAuthorityEntry,
        store: &AuthorityStoreCoordinate,
        maximum_bytes: usize,
    ) -> Result<Self, AuthorityJournalError> {
        if entry.bytes().as_slice().len() > maximum_bytes {
            return Err(AuthorityJournalError::Store(
                DurableAuthorityStoreErrorKind::BoundExceeded {
                    class: pale_ports::AuthorityByteClass::JournalEntry,
                }
                .into(),
            ));
        }
        let value = canon::parse_strict(entry.bytes().as_slice())?;
        let decoded: Self = serde_json::from_value(value)?;
        if decoded.encode(maximum_bytes)?.as_slice() != entry.bytes().as_slice() {
            return Err(AuthorityJournalError::NonCanonical("journal entry"));
        }
        if decoded.format != AUTHORITY_JOURNAL_ENTRY_FORMAT_V1 {
            return Err(AuthorityJournalError::WrongFormat("journal entry"));
        }
        if decoded.control_store_id != *store.store_id()
            || decoded.control_incarnation_id != *store.incarnation_id()
        {
            return Err(AuthorityJournalError::StoreCoordinateMismatch);
        }
        if decoded.journal_sequence != entry.sequence() {
            return Err(AuthorityJournalError::SequenceMismatch {
                expected: entry.sequence().get(),
                observed: decoded.journal_sequence.get(),
            });
        }
        Ok(decoded)
    }

    /// Verify, apply, and verify one sparse authority transition atomically.
    fn fold(
        self,
        current: &ResidentAuthorityState,
        resolver: &dyn super::lifecycle::GenerationLifecycleRecoveryResolver,
        session_leases: &SessionLeaseRecoveryContext<'_>,
    ) -> Result<ResidentAuthorityState, AuthorityJournalError> {
        if ResidentAuthorityStateDigest::from_state(current)? != self.predecessor {
            return Err(AuthorityJournalError::FoldContradiction);
        }
        let admitted =
            self.delta
                .apply(current, self.journal_sequence, resolver, session_leases)?;
        if ResidentAuthorityStateDigest::from_state(&admitted)? != self.successor {
            return Err(AuthorityJournalError::FoldContradiction);
        }
        Ok(admitted)
    }
}

/// Whether one coordinator may attempt another append without explicit recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthorityWriterState {
    /// The installed lifecycle and journal head remain current.
    Ready,
    /// Another writer advanced the store-global head; recovery must refresh state.
    KnownStale,
    /// Retained custody is contradictory; explicit recovery must repair it.
    RecoveryRequired,
    /// This writer's prior append may have committed; recovery must resolve it.
    CommitOutcomeUnknown,
}

/// Application-owned coordinator for one store-global authority writer.
pub(crate) struct DurableAuthorityJournal<'a> {
    store_port: &'a dyn DurableAuthorityStorePort,
    store: AuthorityStoreCoordinate,
    limits: AuthorityRecoveryReadLimits,
    head: Option<JournalSequenceV1>,
    authority: ResidentAuthorityState,
    writer_state: AuthorityWriterState,
}

/// One proven durable append and the complete authority state installed by it.
///
/// The sequence and semantic successor are returned as one success value so a
/// caller never has to reconstruct or rediscover the committed coordinate.
#[must_use]
pub(crate) struct CommittedAuthorityTransition<'a> {
    journal_sequence: JournalSequenceV1,
    authority: &'a ResidentAuthorityState,
}

impl<'a> CommittedAuthorityTransition<'a> {
    /// Consume the success carrier as the preserved coordinate/state pair.
    pub(crate) const fn into_parts(self) -> (JournalSequenceV1, &'a ResidentAuthorityState) {
        (self.journal_sequence, self.authority)
    }
}

impl<'a> DurableAuthorityJournal<'a> {
    /// Resume a ready writer by consuming one proven recovered frontier.
    #[must_use]
    pub(crate) fn resume(
        store_port: &'a dyn DurableAuthorityStorePort,
        limits: AuthorityRecoveryReadLimits,
        recovered: RecoveredAuthorityState,
    ) -> Self {
        let (store, authority, head) = recovered.into_parts();
        Self {
            store_port,
            store,
            limits,
            head,
            authority,
            writer_state: AuthorityWriterState::Ready,
        }
    }

    /// Borrow the exact durable store incarnation owned by this writer.
    #[must_use]
    pub(crate) const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Verify one historical Event transaction and its selected members.
    pub(crate) fn verify_event_custody_members_at(
        &self,
        events: &mut dyn EventCustodyPort,
        sequence: JournalSequenceV1,
        required: &[EventCustodyHistoryMember],
        maximum_event_bytes: usize,
    ) -> Result<(), AuthorityJournalError> {
        self.require_current()?;
        let after = sequence
            .get()
            .checked_sub(1)
            .and_then(|value| JournalSequenceV1::new(value).ok());
        let request =
            JournalTailPageRead::new(self.store.clone(), after, sequence, self.limits, 1)?;
        let page = self.store_port.read_journal_page(&request)?;
        let [entry] = page.entries() else {
            return Err(AuthorityJournalError::FoldContradiction);
        };
        if !matches!(page.boundary(), JournalTailPageBoundary::Complete)
            || entry.sequence() != sequence
        {
            return Err(AuthorityJournalError::FoldContradiction);
        }
        let decoded =
            AuthorityJournalEntry::decode(entry, &self.store, self.limits.maximum_entry_bytes())?;
        let basis = decoded
            .event_custody_basis()
            .filter(|basis| basis.sequence() == sequence && basis.contains_members(required))
            .ok_or(AuthorityJournalError::FoldContradiction)?;
        basis
            .verify_members(&self.store, entry, events, maximum_event_bytes)
            .map_err(|error| AuthorityJournalError::EventCustody(Box::new(error)))
    }

    /// Require this writer's installed frontier to remain current.
    ///
    /// # Errors
    ///
    /// Returns [`AuthorityJournalError::WriterStale`] when another writer has
    /// advanced the store, or [`AuthorityJournalError::WriterCommitOutcomeUnknown`]
    /// when this writer's last append may have committed. Contradictory custody
    /// returns [`AuthorityJournalError::WriterRecoveryRequired`].
    pub(crate) const fn require_current(&self) -> Result<(), AuthorityJournalError> {
        match self.writer_state {
            AuthorityWriterState::Ready => Ok(()),
            AuthorityWriterState::KnownStale => Err(AuthorityJournalError::WriterStale),
            AuthorityWriterState::RecoveryRequired => {
                Err(AuthorityJournalError::WriterRecoveryRequired)
            }
            AuthorityWriterState::CommitOutcomeUnknown => {
                Err(AuthorityJournalError::WriterCommitOutcomeUnknown)
            }
        }
    }

    /// Append and then install one already-derived lifecycle transition.
    ///
    /// The port is called exactly once. A conflict proves this writer's installed
    /// frontier stale; an ambiguous outcome leaves its own commit unknown. Both
    /// states require explicit recovery before another append. Corruption, bounds,
    /// and unavailability prove no append and leave the installed frontier usable.
    ///
    /// # Errors
    ///
    /// Returns [`AuthorityJournalError`] for sequence exhaustion, semantic
    /// contradiction, storage failure, or any call after recovery became required.
    pub(crate) fn commit_transition(
        &mut self,
        transition: GenerationLifecycleTransition,
    ) -> Result<CommittedAuthorityTransition<'_>, AuthorityJournalError> {
        self.commit_authority_transition(ResidentAuthorityState::lifecycle_transition(transition))
    }

    /// Append and install one explicit desired process state.
    ///
    /// Callers must avoid this path when the requested state is already current;
    /// no-op desired-state transitions deliberately consume no journal sequence.
    pub(crate) fn commit_desired_process_state(
        &mut self,
        desired: super::process_reconciliation::DesiredProcessState,
    ) -> Result<CommittedAuthorityTransition<'_>, AuthorityJournalError> {
        let transition = self
            .authority
            .desired_process_state_transition(desired)
            .ok_or(AuthorityJournalError::UnchangedDesiredProcessState)?;
        self.commit_authority_transition(transition)
    }

    /// Append one already-derived product-state transition exactly once.
    fn commit_authority_transition(
        &mut self,
        transition: ResidentAuthorityTransition,
    ) -> Result<CommittedAuthorityTransition<'_>, AuthorityJournalError> {
        let (append, successor) = self.prepare_authority_transition(transition)?;
        let next = append.entry().sequence();
        match self.store_port.compare_and_append(&append) {
            Ok(()) => {
                self.head = Some(next);
                self.authority = successor;
                Ok(CommittedAuthorityTransition {
                    journal_sequence: next,
                    authority: &self.authority,
                })
            }
            Err(error) => {
                match error.append_failure() {
                    DurableAuthorityAppendFailure::RejectedNoAppend => {}
                    DurableAuthorityAppendFailure::Conflict => {
                        self.writer_state = AuthorityWriterState::KnownStale;
                    }
                    DurableAuthorityAppendFailure::OutcomeUnknown => {
                        self.writer_state = AuthorityWriterState::CommitOutcomeUnknown;
                    }
                }
                Err(AuthorityJournalError::Store(error))
            }
        }
    }

    /// Derive bytes and successor state without calling durable custody.
    fn prepare_authority_transition(
        &self,
        transition: ResidentAuthorityTransition,
    ) -> Result<(PreparedAuthorityAppend, ResidentAuthorityState), AuthorityJournalError> {
        self.require_current()?;
        let next = self
            .head
            .map_or_else(
                || JournalSequenceV1::new(1).ok(),
                JournalSequenceV1::checked_successor,
            )
            .ok_or(AuthorityJournalError::SequenceExhausted)?;
        let (entry, successor) =
            AuthorityJournalEntry::from_transition(&self.store, next, &self.authority, transition)?;
        let bytes = entry.encode(self.limits.maximum_entry_bytes())?;
        let append = PreparedAuthorityAppend::new(
            self.store.clone(),
            self.head,
            SequencedAuthorityEntry::new(next, bytes),
        )
        .map_err(AuthorityJournalError::Store)?;
        Ok((append, successor))
    }

    /// Commit the prepared Event members and this journal entry together.
    pub(crate) fn commit_event_custody(
        &mut self,
        store: &mut dyn pale_ports::EventCustodyPort,
        transaction: pale_ports::ControlTransactionPreparation,
        input: super::event_custody::EventCustodyTransitionInput<'_>,
    ) -> Result<super::event_custody::CompletedEventCustodyTransition, AuthorityJournalError> {
        use super::event_custody::journal::EventCustodyBasis;
        use super::event_custody::{
            EventCustodyCommitFailureReason, PreparedEventCustodyTransition,
        };
        self.require_current()?;
        let next = self
            .head
            .map_or_else(
                || JournalSequenceV1::new(1).ok(),
                JournalSequenceV1::checked_successor,
            )
            .ok_or(AuthorityJournalError::SequenceExhausted)?;
        let content =
            super::event_custody::projection::project_content(self.store.clone(), next, input)
                .map_err(|error| AuthorityJournalError::EventProjection(Box::new(error)))?;
        let basis = EventCustodyBasis::from_content(next, &transaction, &content);
        let transition = self.authority.event_custody_transition(basis);
        let (append, successor) = self.prepare_authority_transition(transition)?;
        let candidate = content
            .attach(transaction, append)
            .map_err(|error| AuthorityJournalError::EventProjection(Box::new(error)))?;
        let completed = match PreparedEventCustodyTransition::new(candidate)
            .commit(store, self.limits.maximum_entry_bytes())
        {
            Ok(completed) => completed,
            Err(error) => {
                match error.reason() {
                    EventCustodyCommitFailureReason::Store(error) => match error.kind() {
                        pale_ports::ControlStoreErrorKind::CandidateConflict => {
                            self.writer_state = AuthorityWriterState::KnownStale;
                        }
                        pale_ports::ControlStoreErrorKind::RecoveryRequired => {
                            self.writer_state = AuthorityWriterState::RecoveryRequired;
                        }
                        pale_ports::ControlStoreErrorKind::OutcomeUnknown => {
                            self.writer_state = AuthorityWriterState::CommitOutcomeUnknown;
                        }
                        pale_ports::ControlStoreErrorKind::BoundExceeded
                        | pale_ports::ControlStoreErrorKind::Storage
                        | pale_ports::ControlStoreErrorKind::TransactionExhausted => {}
                    },
                    EventCustodyCommitFailureReason::CommitExcludedByReadBack { .. } => {}
                    EventCustodyCommitFailureReason::RecoveryRequired { .. } => {
                        self.writer_state = AuthorityWriterState::RecoveryRequired;
                    }
                    EventCustodyCommitFailureReason::ReadBackFailed { .. } => {
                        self.writer_state = AuthorityWriterState::CommitOutcomeUnknown;
                    }
                }
                return Err(AuthorityJournalError::EventCommit(Box::new(error)));
            }
        };
        // The existing Event owner has proved the exact joint successor. Until
        // this installation completes, no continuation escapes to the caller.
        self.head = Some(next);
        self.authority = successor;
        Ok(completed)
    }

    /// Persist a reconstructible checkpoint at the exact proven journal head.
    ///
    /// This does not advance authority and selects no cadence or retention
    /// policy. An ambiguous checkpoint write does not close the journal writer
    /// because replay remains defined by the authoritative journal prefix. A
    /// compare conflict does close it because the conflict proves this
    /// coordinator's installed journal head is stale.
    ///
    /// # Errors
    ///
    /// Returns [`AuthorityJournalError`] when no entry exists, checkpoint
    /// encoding fails, or durable custody rejects the bounded projection.
    pub(crate) fn store_checkpoint(&mut self) -> Result<(), AuthorityJournalError> {
        self.require_current()?;
        let head = self.head.ok_or(AuthorityJournalError::MissingJournalHead)?;
        let bytes = encode_recovery_checkpoint(
            &self.store,
            head,
            &self.authority,
            self.limits.maximum_checkpoint_bytes(),
        )?;
        let prepared = PreparedRecoveryCheckpoint::new(
            self.store.clone(),
            head,
            StoredRecoveryCheckpoint::new(head, bytes),
        )?;
        match self.store_port.compare_and_store_checkpoint(&prepared) {
            Ok(()) => Ok(()),
            Err(error) => match error.checkpoint_failure() {
                DurableAuthorityCheckpointFailure::RejectedNoWrite => {
                    Err(AuthorityJournalError::Store(error))
                }
                DurableAuthorityCheckpointFailure::Conflict => {
                    self.writer_state = AuthorityWriterState::KnownStale;
                    Err(AuthorityJournalError::Store(error))
                }
                DurableAuthorityCheckpointFailure::OutcomeUnknown => Err(
                    AuthorityJournalError::CheckpointProjectionOutcomeUnknown(error),
                ),
            },
        }
    }

    /// Return the last store-global sequence proven by this coordinator.
    #[must_use]
    pub(crate) const fn head(&self) -> Option<JournalSequenceV1> {
        self.head
    }

    /// Borrow the exact lifecycle installed at the durable journal frontier.
    #[must_use]
    pub(crate) const fn lifecycle(&self) -> &GenerationLifecycle {
        self.authority.lifecycle()
    }

    /// Return the desired process state installed at this durable frontier.
    #[must_use]
    pub(crate) const fn desired_process_state(
        &self,
    ) -> Option<super::process_reconciliation::DesiredProcessState> {
        self.authority.desired_process_state()
    }
}

/// Private journal encoding, replay, or durable-custody failure.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AuthorityJournalError {
    /// A previous custody contradiction must be resolved before another write.
    #[error("authority writer requires recovery of contradictory custody")]
    WriterRecoveryRequired,
    /// Event semantics rejected the candidate before any commit.
    #[error("Event custody projection failed: {0}")]
    EventProjection(Box<super::event_custody::EventCustodyProjectionError>),
    /// The joint Event commit did not establish its exact successor.
    #[error("Event custody commit failed: {0}")]
    EventCommit(Box<super::event_custody::EventCustodyCommitFailure>),
    /// Retained Event members do not establish the journal's custody basis.
    #[error("Event journal custody failed: {0}")]
    EventCustody(Box<super::event_custody::journal::EventJournalCustodyError>),
    /// Typed state could not be represented under the canonical JSON grammar.
    #[error("authority journal canonicalization failed: {0}")]
    Canonical(#[from] pale_core::canon::CanonError),
    /// Strict canonical JSON did not decode into the private typed record.
    #[error("authority journal typed decode failed: {0}")]
    Decode(#[from] serde_json::Error),
    /// Input bytes decode but are not their exact canonical JCS representation.
    #[error("authority {0} bytes are not canonical")]
    NonCanonical(&'static str),
    /// Input bytes name an unsupported private record format.
    #[error("authority {0} format is not supported")]
    WrongFormat(&'static str),
    /// Input bytes belong to another store or physical incarnation.
    #[error("authority record store coordinate does not match recovery")]
    StoreCoordinateMismatch,
    /// Envelope and supplied sequence coordinates disagree.
    #[error("authority sequence mismatch: expected {expected}, observed {observed}")]
    SequenceMismatch {
        /// Sequence supplied by durable custody.
        expected: u64,
        /// Sequence encoded inside the application record.
        observed: u64,
    },
    /// The co-observed durable head does not contain a returned tail coordinate.
    #[error("authority durable head contradiction: requires {required}, observed {observed:?}")]
    DurableHeadContradiction {
        /// Minimum durable sequence required by the returned observation.
        required: u64,
        /// Co-observed durable head, when custody reported one.
        observed: Option<u64>,
    },
    /// A delta's predecessor or successor fingerprint contradicts the replay fold.
    #[error("authority journal transition contradicts the recovered state")]
    FoldContradiction,
    /// A caller attempted to journal a desired state already installed durably.
    #[error("desired process state is already current and requires no journal append")]
    UnchangedDesiredProcessState,
    /// Checkpoint candidates were not supplied in strict newest-first order.
    #[error("authority recovery checkpoints are not strictly newest first")]
    CheckpointOrder,
    /// A configured journal sequence cannot advance.
    #[error("authority journal sequence is exhausted")]
    SequenceExhausted,
    /// Another writer advanced the store-global head; this writer must recover.
    #[error("authority journal writer frontier is stale; explicit recovery is required")]
    WriterStale,
    /// This writer's prior append may have committed; this writer must recover.
    #[error("authority journal writer commit outcome is unknown; explicit recovery is required")]
    WriterCommitOutcomeUnknown,
    /// A checkpoint was requested before any authoritative journal entry exists.
    #[error("authority checkpoint requires a nonempty journal")]
    MissingJournalHead,
    /// A non-authoritative checkpoint projection may or may not have committed.
    #[error("authority checkpoint projection outcome is unknown; journal authority is unchanged")]
    CheckpointProjectionOutcomeUnknown(#[source] DurableAuthorityStoreError),
    /// Recovered lifecycle semantics violated their owning algebra.
    #[error("recovered generation lifecycle is invalid: {0}")]
    Lifecycle(#[from] GenerationLifecycleError),
    /// Durable byte custody could not prove the requested operation.
    #[error(transparent)]
    Store(#[from] DurableAuthorityStoreError),
}

impl From<ResidentAuthorityTransitionError> for AuthorityJournalError {
    fn from(error: ResidentAuthorityTransitionError) -> Self {
        match error {
            ResidentAuthorityTransitionError::PredecessorMismatch => Self::FoldContradiction,
            ResidentAuthorityTransitionError::Lifecycle(error) => Self::Lifecycle(error),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/journal.rs"]
pub(super) mod tests;
