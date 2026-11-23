//! Neutral byte custody for the resident authority journal.
//!
//! The application owns the journal entry and recovery-checkpoint algebra. An
//! adapter stores their already-canonical bytes under one control-store
//! coordinate, performs a store-global compare-and-append, and returns bounded
//! recovery slices. Physical framing, checksums, synchronization, and storage
//! profiles remain adapter-owned mechanics rather than public semantic records.
//!
//! A successful append proves the store-global successor law. The empty store
//! admits sequence one; every non-empty store admits only the exact successor
//! of its current head:

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/durable-authority-sequence.display.mathml"
))]
//!
//! [`DurableAuthorityStoreErrorKind::AmbiguousCommit`] proves
//! neither success nor failure; callers must close the current writer and recover
//! from durable bytes instead of retrying or inferring the outcome.
use pale_core::records::{ControlIncarnationIdV1, ControlStoreIdV1, JournalSequenceV1};

/// Stable coordinate of one incarnation of a durable authority store.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuthorityStoreCoordinate {
    store_id: ControlStoreIdV1,
    incarnation_id: ControlIncarnationIdV1,
}

impl AuthorityStoreCoordinate {
    /// Bind a stable store identity to its current physical incarnation.
    #[must_use]
    pub fn new(store_id: ControlStoreIdV1, incarnation_id: ControlIncarnationIdV1) -> Self {
        Self {
            store_id,
            incarnation_id,
        }
    }

    /// Borrow the stable control-store identity.
    #[must_use]
    pub fn store_id(&self) -> &ControlStoreIdV1 {
        &self.store_id
    }

    /// Borrow the current control-store incarnation identity.
    #[must_use]
    pub fn incarnation_id(&self) -> &ControlIncarnationIdV1 {
        &self.incarnation_id
    }
}

/// Class of opaque bytes whose configured bound was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityByteClass {
    /// One application-owned journal-entry byte string.
    JournalEntry,
    /// One application-owned recovery-checkpoint byte string.
    RecoveryCheckpoint,
}

/// Non-empty bounded bytes for exactly one authority-journal entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedAuthorityEntryBytes(Vec<u8>);

impl BoundedAuthorityEntryBytes {
    /// Admit non-empty entry bytes under the caller's explicit hard bound.
    ///
    /// Canonicality remains an application proof; this type proves only byte
    /// ownership and size at the neutral adapter boundary.
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`DurableAuthorityStoreErrorKind::Corrupt`] for
    /// empty bytes or [`DurableAuthorityStoreErrorKind::BoundExceeded`] when
    /// `bytes` is too large.
    pub fn new(bytes: Vec<u8>, maximum_bytes: usize) -> Result<Self, DurableAuthorityStoreError> {
        admit_bytes(bytes, maximum_bytes, AuthorityByteClass::JournalEntry).map(Self)
    }

    /// Borrow the complete opaque entry bytes.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

/// Non-empty bounded bytes for one handle-free recovery checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedRecoveryCheckpointBytes(Vec<u8>);

impl BoundedRecoveryCheckpointBytes {
    /// Admit non-empty checkpoint bytes under the caller's explicit hard bound.
    ///
    /// Canonicality remains an application proof; this type proves only byte
    /// ownership and size at the neutral adapter boundary.
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`DurableAuthorityStoreErrorKind::Corrupt`] for
    /// empty bytes or [`DurableAuthorityStoreErrorKind::BoundExceeded`] when
    /// `bytes` is too large.
    pub fn new(bytes: Vec<u8>, maximum_bytes: usize) -> Result<Self, DurableAuthorityStoreError> {
        admit_bytes(bytes, maximum_bytes, AuthorityByteClass::RecoveryCheckpoint).map(Self)
    }

    /// Borrow the complete opaque checkpoint bytes.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

/// Apply the shared nonempty and hard-size laws without claiming canonicality.
fn admit_bytes(
    bytes: Vec<u8>,
    maximum_bytes: usize,
    class: AuthorityByteClass,
) -> Result<Vec<u8>, DurableAuthorityStoreError> {
    if bytes.is_empty() {
        return Err(DurableAuthorityStoreErrorKind::Corrupt(class).into());
    }
    if bytes.len() > maximum_bytes {
        return Err(DurableAuthorityStoreErrorKind::BoundExceeded { class }.into());
    }
    Ok(bytes)
}

/// Explicit allocation and input bounds for one recovery pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityRecoveryReadLimits {
    maximum_entry_bytes: usize,
    maximum_checkpoint_bytes: usize,
    maximum_checkpoint_candidates: usize,
    maximum_tail_entries: usize,
}

impl AuthorityRecoveryReadLimits {
    /// Construct a recovery budget in bytes and record counts.
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`DurableAuthorityStoreErrorKind::BoundExceeded`]
    /// when any bound is zero, because a zero budget cannot admit even one record.
    pub fn new(
        maximum_entry_bytes: usize,
        maximum_checkpoint_bytes: usize,
        maximum_checkpoint_candidates: usize,
        maximum_tail_entries: usize,
    ) -> Result<Self, DurableAuthorityStoreError> {
        if maximum_entry_bytes == 0 || maximum_tail_entries == 0 {
            return Err(DurableAuthorityStoreErrorKind::BoundExceeded {
                class: AuthorityByteClass::JournalEntry,
            }
            .into());
        }
        if maximum_checkpoint_bytes == 0 || maximum_checkpoint_candidates == 0 {
            return Err(DurableAuthorityStoreErrorKind::BoundExceeded {
                class: AuthorityByteClass::RecoveryCheckpoint,
            }
            .into());
        }
        Ok(Self {
            maximum_entry_bytes,
            maximum_checkpoint_bytes,
            maximum_checkpoint_candidates,
            maximum_tail_entries,
        })
    }

    /// Return the hard byte bound for one journal entry.
    #[must_use]
    pub const fn maximum_entry_bytes(self) -> usize {
        self.maximum_entry_bytes
    }

    /// Return the hard byte bound for one recovery checkpoint.
    #[must_use]
    pub const fn maximum_checkpoint_bytes(self) -> usize {
        self.maximum_checkpoint_bytes
    }

    /// Return the maximum checkpoint candidates supplied to the application.
    #[must_use]
    pub const fn maximum_checkpoint_candidates(self) -> usize {
        self.maximum_checkpoint_candidates
    }

    /// Return the maximum entries in one requested tail observation.
    ///
    /// The count includes the first entry after the request's exclusive
    /// frontier, whether that frontier came from a checkpoint or an earlier
    /// bounded tail read.
    #[must_use]
    pub const fn maximum_tail_entries(self) -> usize {
        self.maximum_tail_entries
    }
}

/// One opaque journal entry paired with its store-global sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequencedAuthorityEntry {
    sequence: JournalSequenceV1,
    bytes: BoundedAuthorityEntryBytes,
}

impl SequencedAuthorityEntry {
    /// Pair a validated sequence coordinate with its opaque bounded bytes.
    #[must_use]
    pub fn new(sequence: JournalSequenceV1, bytes: BoundedAuthorityEntryBytes) -> Self {
        Self { sequence, bytes }
    }

    /// Return the store-global append coordinate.
    #[must_use]
    pub const fn sequence(&self) -> JournalSequenceV1 {
        self.sequence
    }

    /// Borrow the complete opaque entry bytes.
    #[must_use]
    pub fn bytes(&self) -> &BoundedAuthorityEntryBytes {
        &self.bytes
    }
}

/// One append prepared against the exact current store-global journal head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedAuthorityAppend {
    store: AuthorityStoreCoordinate,
    expected_head: Option<JournalSequenceV1>,
    entry: SequencedAuthorityEntry,
}

impl PreparedAuthorityAppend {
    /// Admit an initial append at one or an exact successor append.
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`DurableAuthorityStoreErrorKind::Corrupt`]
    /// unless the entry is at sequence one when `expected_head` is absent, or
    /// exactly one greater than the supplied head otherwise.
    pub fn new(
        store: AuthorityStoreCoordinate,
        expected_head: Option<JournalSequenceV1>,
        entry: SequencedAuthorityEntry,
    ) -> Result<Self, DurableAuthorityStoreError> {
        let expected_sequence = match expected_head {
            None => JournalSequenceV1::new(1).ok(),
            Some(head) => head.checked_successor(),
        };
        if expected_sequence != Some(entry.sequence()) {
            return Err(
                DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry).into(),
            );
        }
        Ok(Self {
            store,
            expected_head,
            entry,
        })
    }

    /// Borrow the selected store incarnation.
    #[must_use]
    pub fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Return the journal head that must still be current.
    #[must_use]
    pub const fn expected_head(&self) -> Option<JournalSequenceV1> {
        self.expected_head
    }

    /// Borrow the exact successor entry.
    #[must_use]
    pub fn entry(&self) -> &SequencedAuthorityEntry {
        &self.entry
    }
}

/// One handle-free checkpoint paired with the final sequence it folds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRecoveryCheckpoint {
    through_sequence: JournalSequenceV1,
    bytes: BoundedRecoveryCheckpointBytes,
}

/// One physically inspected recovery-checkpoint candidate.
///
/// The adapter owns frame and checksum validation, while the application owns
/// canonical decoding and semantic admission. A physically corrupt candidate
/// therefore retains only its ordered journal coordinate so recovery can skip
/// it and continue to an older candidate without treating corrupt bytes as an
/// application record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryCheckpointCandidate {
    /// The physical frame admitted and exposes bounded opaque application bytes.
    Trusted(StoredRecoveryCheckpoint),
    /// The frame at this coordinate failed adapter-owned integrity validation.
    Rejected {
        /// Inclusive journal frontier claimed by the rejected candidate row.
        through_sequence: JournalSequenceV1,
        /// Adapter-owned reason the application bytes cannot be exposed.
        reason: RecoveryCheckpointPhysicalRejection,
    },
}

/// Physical reason one checkpoint candidate cannot reach semantic admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryCheckpointPhysicalRejection {
    /// The stored frame or checksum is corrupt.
    Integrity,
    /// The candidate exceeds the configured physical recovery budget.
    BoundExceeded,
}

impl RecoveryCheckpointCandidate {
    /// Return the candidate's inclusive journal coordinate for order checks.
    #[must_use]
    pub const fn through_sequence(&self) -> JournalSequenceV1 {
        match self {
            Self::Trusted(checkpoint) => checkpoint.through_sequence(),
            Self::Rejected {
                through_sequence, ..
            } => *through_sequence,
        }
    }

    /// Borrow trusted application bytes, excluding a corrupt physical frame.
    #[must_use]
    pub const fn trusted(&self) -> Option<&StoredRecoveryCheckpoint> {
        match self {
            Self::Trusted(checkpoint) => Some(checkpoint),
            Self::Rejected { .. } => None,
        }
    }

    /// Return the adapter-owned rejection reason, when no bytes are exposed.
    #[must_use]
    pub const fn rejection(&self) -> Option<RecoveryCheckpointPhysicalRejection> {
        match self {
            Self::Trusted(_) => None,
            Self::Rejected { reason, .. } => Some(*reason),
        }
    }
}

/// Reconstructible checkpoint prepared against the exact journal frontier it folds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRecoveryCheckpoint {
    store: AuthorityStoreCoordinate,
    expected_head: JournalSequenceV1,
    checkpoint: StoredRecoveryCheckpoint,
}

impl PreparedRecoveryCheckpoint {
    /// Bind one checkpoint to the exact current journal head.
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`DurableAuthorityStoreErrorKind::Corrupt`]
    /// unless the checkpoint's inclusive frontier equals `expected_head` exactly.
    pub fn new(
        store: AuthorityStoreCoordinate,
        expected_head: JournalSequenceV1,
        checkpoint: StoredRecoveryCheckpoint,
    ) -> Result<Self, DurableAuthorityStoreError> {
        if checkpoint.through_sequence() != expected_head {
            return Err(DurableAuthorityStoreErrorKind::Corrupt(
                AuthorityByteClass::RecoveryCheckpoint,
            )
            .into());
        }
        Ok(Self {
            store,
            expected_head,
            checkpoint,
        })
    }

    /// Borrow the selected store incarnation.
    #[must_use]
    pub fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Return the exact journal head against which the adapter must compare.
    #[must_use]
    pub const fn expected_head(&self) -> JournalSequenceV1 {
        self.expected_head
    }

    /// Borrow the reconstructible checkpoint bytes and frontier.
    #[must_use]
    pub fn checkpoint(&self) -> &StoredRecoveryCheckpoint {
        &self.checkpoint
    }
}

impl StoredRecoveryCheckpoint {
    /// Pair a checkpoint's inclusive journal frontier with its opaque bytes.
    #[must_use]
    pub fn new(through_sequence: JournalSequenceV1, bytes: BoundedRecoveryCheckpointBytes) -> Self {
        Self {
            through_sequence,
            bytes,
        }
    }

    /// Return the inclusive journal frontier represented by this checkpoint.
    #[must_use]
    pub const fn through_sequence(&self) -> JournalSequenceV1 {
        self.through_sequence
    }

    /// Borrow the complete opaque checkpoint bytes.
    #[must_use]
    pub fn bytes(&self) -> &BoundedRecoveryCheckpointBytes {
        &self.bytes
    }
}

/// Bounded request for newest-first recovery-checkpoint candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryCheckpointRead {
    store: AuthorityStoreCoordinate,
    limits: AuthorityRecoveryReadLimits,
}

impl RecoveryCheckpointRead {
    /// Select one store and explicit recovery budget.
    #[must_use]
    pub fn new(store: AuthorityStoreCoordinate, limits: AuthorityRecoveryReadLimits) -> Self {
        Self { store, limits }
    }

    /// Borrow the selected store incarnation.
    #[must_use]
    pub fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Return the complete recovery budget.
    #[must_use]
    pub const fn limits(&self) -> AuthorityRecoveryReadLimits {
        self.limits
    }
}

/// Bounded page request against one fixed recovery head.
///
/// The fixed `through` coordinate turns a growing authority journal into one
/// finite recovery snapshot. Later appends remain outside this pass and cannot
/// make pagination chase a moving head indefinitely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalTailPageRead {
    store: AuthorityStoreCoordinate,
    after: Option<JournalSequenceV1>,
    through: JournalSequenceV1,
    limits: AuthorityRecoveryReadLimits,
    maximum_page_entries: usize,
    remaining_entries: usize,
}

impl JournalTailPageRead {
    /// Select the first exclusive page frontier below an exact recovery head.
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`DurableAuthorityStoreErrorKind::Corrupt`]
    /// when `after` is at or beyond `through`, or an error of kind
    /// [`DurableAuthorityStoreErrorKind::BoundExceeded`] when the per-page
    /// bound is zero.
    pub fn new(
        store: AuthorityStoreCoordinate,
        after: Option<JournalSequenceV1>,
        through: JournalSequenceV1,
        limits: AuthorityRecoveryReadLimits,
        maximum_page_entries: usize,
    ) -> Result<Self, DurableAuthorityStoreError> {
        if after.is_some_and(|after| after >= through) {
            return Err(
                DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry).into(),
            );
        }
        if maximum_page_entries == 0 {
            return Err(DurableAuthorityStoreErrorKind::BoundExceeded {
                class: AuthorityByteClass::JournalEntry,
            }
            .into());
        }
        Ok(Self {
            store,
            after,
            through,
            limits,
            maximum_page_entries,
            remaining_entries: limits.maximum_tail_entries(),
        })
    }

    /// Borrow the selected store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Return the exclusive sequence frontier for this page.
    #[must_use]
    pub const fn after(&self) -> Option<JournalSequenceV1> {
        self.after
    }

    /// Return the inclusive durable head fixed for the complete recovery pass.
    #[must_use]
    pub const fn through(&self) -> JournalSequenceV1 {
        self.through
    }

    /// Return the complete byte and cumulative recovery budget.
    #[must_use]
    pub const fn limits(&self) -> AuthorityRecoveryReadLimits {
        self.limits
    }

    /// Return the maximum entries the adapter may return in this page.
    #[must_use]
    pub const fn maximum_page_entries(&self) -> usize {
        if self.maximum_page_entries < self.remaining_entries {
            self.maximum_page_entries
        } else {
            self.remaining_entries
        }
    }

    /// Return the entries still available under the whole-pass tail budget.
    #[must_use]
    pub const fn remaining_entries(&self) -> usize {
        self.remaining_entries
    }

    /// Construct the next request from this page's exact continuation boundary.
    ///
    /// `None` means the page completed or quarantined the fixed recovery pass.
    /// The cumulative allowance decreases by the exact returned entry count, so
    /// repeated bounded reads cannot exceed [`AuthorityRecoveryReadLimits::maximum_tail_entries`].
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`DurableAuthorityStoreErrorKind::BoundExceeded`]
    /// if a continuation would exceed the whole-pass entry budget.
    pub fn continue_after(
        &self,
        page: &JournalTailPage,
    ) -> Result<Option<Self>, DurableAuthorityStoreError> {
        let JournalTailPageBoundary::More { resume_after } = page.boundary else {
            return Ok(None);
        };
        let remaining_entries = self
            .remaining_entries
            .checked_sub(page.entries.len())
            .filter(|remaining| *remaining > 0);
        let Some(remaining_entries) = remaining_entries else {
            return Err(DurableAuthorityStoreErrorKind::BoundExceeded {
                class: AuthorityByteClass::JournalEntry,
            }
            .into());
        };
        Ok(Some(Self {
            store: self.store.clone(),
            after: Some(resume_after),
            through: self.through,
            limits: self.limits,
            maximum_page_entries: self.maximum_page_entries,
            remaining_entries,
        }))
    }
}

/// Physical boundary reached by one fixed-head journal page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalTailPageBoundary {
    /// The page reached the request's inclusive recovery head.
    Complete,
    /// More physically valid entries remain before the fixed recovery head.
    More {
        /// Exact last sequence returned; use it as the next exclusive frontier.
        resume_after: JournalSequenceV1,
    },
    /// Physical validation stopped before this first untrusted sequence.
    QuarantinedFrom {
        /// First store-global coordinate whose physical bytes are not trusted.
        first_untrusted_sequence: JournalSequenceV1,
    },
}

/// One validated physical page from a fixed authority-journal snapshot.
///
/// This value proves pagination shape, bounds, and physical sequence closure.
/// It does not prove application canonicality, semantic folding, checkpoint
/// admission, or anti-rollback anchor state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalTailPage {
    entries: Box<[SequencedAuthorityEntry]>,
    boundary: JournalTailPageBoundary,
}

impl JournalTailPage {
    /// Bind an ascending contiguous page to its exact request and boundary.
    ///
    /// # Errors
    ///
    /// Returns [`DurableAuthorityStoreError`] when entry count, ordering,
    /// contiguity, target head, or continuation coordinates disagree.
    pub fn new(
        request: &JournalTailPageRead,
        entries: Vec<SequencedAuthorityEntry>,
        boundary: JournalTailPageBoundary,
    ) -> Result<Self, DurableAuthorityStoreError> {
        if entries.len() > request.maximum_page_entries() {
            return Err(DurableAuthorityStoreErrorKind::BoundExceeded {
                class: AuthorityByteClass::JournalEntry,
            }
            .into());
        }

        let first = request.after.map_or_else(
            || JournalSequenceV1::new(1).ok(),
            JournalSequenceV1::checked_successor,
        );
        let mut expected = first;
        let contiguous = entries.iter().all(|entry| {
            if expected != Some(entry.sequence()) || entry.sequence() > request.through {
                return false;
            }
            expected = entry.sequence().checked_successor();
            true
        });
        if !contiguous {
            return Err(
                DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry).into(),
            );
        }

        let last = entries.last().map(SequencedAuthorityEntry::sequence);
        let next = last.or(request.after).map_or_else(
            || JournalSequenceV1::new(1).ok(),
            JournalSequenceV1::checked_successor,
        );
        if matches!(boundary, JournalTailPageBoundary::More { .. })
            && entries.len() >= request.remaining_entries
        {
            return Err(DurableAuthorityStoreErrorKind::BoundExceeded {
                class: AuthorityByteClass::JournalEntry,
            }
            .into());
        }
        let boundary_is_exact = match boundary {
            JournalTailPageBoundary::Complete => last == Some(request.through),
            JournalTailPageBoundary::More { resume_after } => {
                !entries.is_empty() && last == Some(resume_after) && resume_after < request.through
            }
            JournalTailPageBoundary::QuarantinedFrom {
                first_untrusted_sequence,
            } => {
                next == Some(first_untrusted_sequence)
                    && first_untrusted_sequence <= request.through
            }
        };
        if !boundary_is_exact {
            return Err(
                DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry).into(),
            );
        }

        Ok(Self {
            entries: entries.into_boxed_slice(),
            boundary,
        })
    }

    /// Borrow the physically validated contiguous entries.
    #[must_use]
    pub const fn entries(&self) -> &[SequencedAuthorityEntry] {
        &self.entries
    }

    /// Return the exact physical completion, continuation, or quarantine state.
    #[must_use]
    pub const fn boundary(&self) -> JournalTailPageBoundary {
        self.boundary
    }
}

/// Stable failure meaning at the durable-authority boundary.
///
/// This value determines recovery and retry behavior. A
/// [`DurableAuthorityStoreError`] retains it separately from any adapter-native
/// cause, so storage implementation details cannot change its meaning.
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum DurableAuthorityStoreErrorKind {
    /// Another writer changed the expected store-global journal head.
    #[error("durable authority journal compare-and-append conflict")]
    Conflict,
    /// Bytes, sequence coordinates, or physical integrity are invalid.
    #[error("durable authority {0:?} is corrupt")]
    Corrupt(AuthorityByteClass),
    /// A configured hard byte or record-count bound was exceeded.
    #[error("durable authority {class:?} exceeds its configured bound")]
    BoundExceeded {
        /// Byte family whose hard bound was exceeded.
        class: AuthorityByteClass,
    },
    /// The store could not perform the operation and proved no append occurred.
    #[error("durable authority store is unavailable")]
    Unavailable,
    /// The store cannot prove whether the requested mutation committed.
    #[error("durable authority store mutation outcome is ambiguous")]
    AmbiguousCommit,
}

/// Durable storage failure with its neutral meaning and original native cause.
///
/// Inspect [`Self::kind`] for control flow and [`std::error::Error::source`] for
/// diagnosis. The cause is intentionally neither comparable nor cloneable;
/// callers must not infer retry safety from its text or expose it directly to
/// an untrusted consumer. Adapters use [`Self::with_source`] when a native
/// operation fails; pure contract checks convert their kind with [`From`].
#[derive(Debug, thiserror::Error)]
#[error("{kind}")]
pub struct DurableAuthorityStoreError {
    /// Stable recovery meaning, independent of the storage implementation.
    kind: DurableAuthorityStoreErrorKind,
    /// Original native failure, absent for a pure contract rejection.
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl From<DurableAuthorityStoreErrorKind> for DurableAuthorityStoreError {
    fn from(kind: DurableAuthorityStoreErrorKind) -> Self {
        Self { kind, source: None }
    }
}

/// Effect-relevant result class for a failed authority-journal append.
///
/// The classification applies specifically to
/// [`DurableAuthorityStorePort::compare_and_append`]. Read and checkpoint
/// operations retain their ordinary error meaning. An adapter must report
/// [`Self::OutcomeUnknown`] whenever it cannot exclude a commit; returning
/// [`Self::RejectedNoAppend`] is a proof that retry cannot duplicate this
/// attempted append.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurableAuthorityAppendFailure {
    /// The adapter proved that the requested entry was not appended.
    RejectedNoAppend,
    /// Another writer changed the expected head and this append did not occur.
    Conflict,
    /// The adapter cannot prove whether the requested entry committed.
    OutcomeUnknown,
}

/// Authority-relevant result class for a failed checkpoint projection write.
///
/// Unlike a journal append, a checkpoint never advances semantic authority.
/// An ambiguous write can therefore leave an unknown cache projection without
/// making the journal head or an external effect uncertain. A head conflict is
/// still decisive: it proves the caller's installed authority frontier is stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurableAuthorityCheckpointFailure {
    /// The adapter proved that no checkpoint projection was written.
    RejectedNoWrite,
    /// Another writer changed the journal head before this projection committed.
    Conflict,
    /// The adapter cannot prove whether an equivalent projection was written.
    OutcomeUnknown,
}

impl DurableAuthorityStoreError {
    /// Retain a native cause without changing the selected failure meaning.
    ///
    /// The adapter must classify the operation before crossing this boundary:
    /// a failure after a possibly committed mutation requires
    /// [`DurableAuthorityStoreErrorKind::AmbiguousCommit`], regardless of its
    /// native error code.
    #[must_use]
    pub fn with_source(
        kind: DurableAuthorityStoreErrorKind,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind,
            source: Some(Box::new(source)),
        }
    }

    /// Return the implementation-independent classification of this failure.
    #[must_use]
    pub const fn kind(&self) -> DurableAuthorityStoreErrorKind {
        self.kind
    }

    /// Classify this error under the append outcome contract.
    ///
    /// Corruption, bounds, and unavailability are permitted append rejections
    /// only when the adapter proves that no append occurred. If that proof is
    /// unavailable after any attempted mutation, the adapter must return
    /// [`DurableAuthorityStoreErrorKind::AmbiguousCommit`] instead.
    #[must_use]
    pub const fn append_failure(&self) -> DurableAuthorityAppendFailure {
        match self.kind {
            DurableAuthorityStoreErrorKind::Conflict => DurableAuthorityAppendFailure::Conflict,
            DurableAuthorityStoreErrorKind::AmbiguousCommit => {
                DurableAuthorityAppendFailure::OutcomeUnknown
            }
            DurableAuthorityStoreErrorKind::Corrupt(_)
            | DurableAuthorityStoreErrorKind::BoundExceeded { .. }
            | DurableAuthorityStoreErrorKind::Unavailable => {
                DurableAuthorityAppendFailure::RejectedNoAppend
            }
        }
    }

    /// Classify this error under the non-authoritative checkpoint contract.
    ///
    /// This classification must not be reused for journal append or native
    /// effect decisions; an unknown checkpoint write leaves authority defined
    /// by the unchanged journal prefix.
    #[must_use]
    pub const fn checkpoint_failure(&self) -> DurableAuthorityCheckpointFailure {
        match self.kind {
            DurableAuthorityStoreErrorKind::Conflict => DurableAuthorityCheckpointFailure::Conflict,
            DurableAuthorityStoreErrorKind::AmbiguousCommit => {
                DurableAuthorityCheckpointFailure::OutcomeUnknown
            }
            DurableAuthorityStoreErrorKind::Corrupt(_)
            | DurableAuthorityStoreErrorKind::BoundExceeded { .. }
            | DurableAuthorityStoreErrorKind::Unavailable => {
                DurableAuthorityCheckpointFailure::RejectedNoWrite
            }
        }
    }
}

/// Opaque durable custody for one application-owned authority journal.
///
/// Implementations must apply [`PreparedAuthorityAppend`] atomically against the
/// store-global head. Recovery candidates are returned newest first; journal
/// tails are returned in exact ascending sequence order. Every returned vector
/// and byte string must respect the request's [`AuthorityRecoveryReadLimits`].
/// The application independently checks order, bounds, canonical bytes, and
/// fold laws before accepting any recovered state.
pub trait DurableAuthorityStorePort: Send + Sync {
    /// Compare the exact global head and append one successor entry atomically.
    ///
    /// # Errors
    ///
    /// Returns [`DurableAuthorityStoreError`] without retry. In particular,
    /// [`DurableAuthorityStoreErrorKind::Conflict`] proves no append but requires
    /// recovery to refresh the stale head.
    /// [`DurableAuthorityStoreErrorKind::AmbiguousCommit`] means the adapter cannot
    /// exclude a commit and therefore requires recovery before any retry.
    /// Every other error kind proves no append. An implementation must select
    /// `AmbiguousCommit` after any failed mutation whose commit outcome cannot
    /// be excluded, and retain the native cause when one exists.
    fn compare_and_append(
        &self,
        append: &PreparedAuthorityAppend,
    ) -> Result<(), DurableAuthorityStoreError>;

    /// Store one reconstructible checkpoint only if its journal head is current.
    ///
    /// A checkpoint is not authority and cannot advance the journal. An
    /// ambiguous outcome may leave either zero or one equivalent projection,
    /// so callers may recover without closing the authority-journal writer.
    ///
    /// # Errors
    ///
    /// Returns [`DurableAuthorityStoreError`] when the head is stale or storage
    /// cannot prove the bounded checkpoint operation.
    fn compare_and_store_checkpoint(
        &self,
        checkpoint: &PreparedRecoveryCheckpoint,
    ) -> Result<(), DurableAuthorityStoreError>;

    /// Read bounded newest-first checkpoint candidates for recovery.
    ///
    /// A candidate whose own frame or checksum fails physical validation must
    /// be returned as [`RecoveryCheckpointCandidate::Rejected`] so an older
    /// candidate can still be considered. The whole read fails only when the
    /// adapter cannot prove the store-level read, ordering coordinates, or
    /// configured result bounds.
    ///
    /// # Errors
    ///
    /// Returns [`DurableAuthorityStoreError`] when custody, availability, the
    /// candidate ordering coordinates, or a requested aggregate bound cannot
    /// be proven.
    fn read_recovery_checkpoints(
        &self,
        request: &RecoveryCheckpointRead,
    ) -> Result<Vec<RecoveryCheckpointCandidate>, DurableAuthorityStoreError>;

    /// Read the current store-global durable head, or `None` for an empty store.
    ///
    /// # Errors
    ///
    /// Returns [`DurableAuthorityStoreError`] when physical custody or
    /// availability cannot be proven.
    fn read_journal_head(
        &self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<Option<JournalSequenceV1>, DurableAuthorityStoreError>;

    /// Read one bounded page through the request's fixed recovery head.
    ///
    /// # Errors
    ///
    /// Returns [`DurableAuthorityStoreError`] when custody, physical integrity,
    /// ordering, availability, or a requested bound cannot be proven.
    fn read_journal_page(
        &self,
        request: &JournalTailPageRead,
    ) -> Result<JournalTailPage, DurableAuthorityStoreError>;
}

#[cfg(test)]
#[path = "../tests/internal/durability.rs"]
mod tests;
