//! Checkpoint-plus-contiguous-tail reconstruction for resident authority.
//!
//! A checkpoint is usable only when its re-admitted state matches the successor
//! fingerprint in the retained journal entry at the same frontier. The
//! checkpoint and contiguous-tail recurrence is rendered on
//! [`crate::resident::ResidentApplication`].
//!
//! The binding entry and suffix come from one bounded storage observation.
//! Cache-local validity alone cannot authorize skipping journal history.
//!
//! Each fold must satisfy the embedded expected-state equality. A complete
//! result reconstructs lifecycle, desired process state, and last Event custody;
//! a quarantined result retains the validated product-state
//! prefix while quarantining the suffix. Neither contains graph or arena
//! handles, index positions, ring cursors, or proof of runtime readiness.

use pale_core::canon;
use pale_core::records::{ControlIncarnationIdV1, ControlStoreIdV1, JournalSequenceV1};
use pale_ports::{
    AuthorityByteClass, AuthorityRecoveryReadLimits, AuthorityStoreCoordinate,
    BoundedRecoveryCheckpointBytes, DurableAuthorityStoreErrorKind, DurableAuthorityStorePort,
    JournalTailPageBoundary, JournalTailPageRead, RecoveryCheckpointRead, SequencedAuthorityEntry,
    SessionLeaseCustodyPort,
};
use serde::{Deserialize, Serialize};

use super::{AuthorityJournalEntry, AuthorityJournalError, AUTHORITY_RECOVERY_IMAGE_FORMAT_V1};
use crate::resident::authority::{ResidentAuthorityRecoveryStateV1, ResidentAuthorityState};
use crate::resident::lifecycle::{
    GenerationLifecycle, GenerationLifecycleError, GenerationLifecycleRecoveryResolver,
};
use crate::resident::session_lease_custody::SessionLeaseRecoveryContext;

/// Actual custody readers and byte limits used by one recovery attempt.
pub(crate) struct AuthorityRecoveryCustody<'a> {
    pub(crate) journal: &'a dyn DurableAuthorityStorePort,
    pub(crate) session_leases: &'a dyn SessionLeaseCustodyPort,
    pub(crate) events: &'a mut dyn pale_ports::EventCustodyPort,
    pub(crate) maximum_session_lease_bytes: usize,
    pub(crate) maximum_event_bytes: usize,
}

struct RecoveryFoldCustody<'a> {
    sessions: SessionLeaseRecoveryContext<'a>,
    events: &'a mut dyn pale_ports::EventCustodyPort,
    maximum_event_bytes: usize,
}

/// Maximum authority entries requested from physical custody in one page.
const AUTHORITY_RECOVERY_PAGE_ENTRIES: usize = 256;

/// Private handle-free checkpoint image reconstructed from retained owner custody.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RecoveryImageV1 {
    /// Private decoder discriminator; not a public schema identity.
    format: String,
    /// Stable store identity bound into this projection.
    control_store_id: ControlStoreIdV1,
    /// Physical store incarnation from which replay must continue.
    control_incarnation_id: ControlIncarnationIdV1,
    /// Inclusive authoritative journal frontier folded into this image.
    through_sequence: JournalSequenceV1,
    /// Untrusted composite authority projection to re-admit.
    authority: ResidentAuthorityRecoveryStateV1,
}

/// Complete semantic authority and exact durable frontier reconstructed by recovery.
pub(crate) struct RecoveredAuthorityState {
    store: AuthorityStoreCoordinate,
    authority: ResidentAuthorityState,
    journal_head: Option<JournalSequenceV1>,
}

impl RecoveredAuthorityState {
    /// Borrow the completely recovered lifecycle without weakening its journal frontier.
    pub(in crate::resident) const fn lifecycle(&self) -> &GenerationLifecycle {
        self.authority.lifecycle()
    }

    /// Return the desired process state recovered at the exact durable frontier.
    pub(in crate::resident) const fn desired_process_state(
        &self,
    ) -> Option<crate::resident::process_reconciliation::DesiredProcessState> {
        self.authority.desired_process_state()
    }

    /// Consume the verified store, product state, and inclusive frontier together.
    pub(in crate::resident) fn into_parts(
        self,
    ) -> (
        AuthorityStoreCoordinate,
        ResidentAuthorityState,
        Option<JournalSequenceV1>,
    ) {
        (self.store, self.authority, self.journal_head)
    }

    /// Return the inclusive durable journal frontier, if any entry exists.
    #[must_use]
    pub(crate) const fn journal_head(&self) -> Option<JournalSequenceV1> {
        self.journal_head
    }
}

/// Closed result of attempting to reconstruct resident authority.
///
/// Only [`Self::Complete`] may resume a writer. [`Self::Degraded`] retains the
/// validated historical prefix for read-only inspection while withholding all
/// mutation authority from the unresolved suffix.
pub(crate) enum AuthorityRecoveryOutcome {
    /// The exact tail through the co-observed durable head admitted completely.
    Complete(Box<RecoveredAuthorityState>),
    /// Recovery retained a validated prefix and quarantined the unresolved suffix.
    Degraded(Box<DegradedAuthorityState>),
}

/// Validated historical prefix plus the exact suffix quarantine boundary.
///
/// This value exposes no journal writer, generation bodies, wake ring, runtime
/// admission, or effect surface. Its prefix is safe for bounded inspection but
/// is not the current durable head while the suffix remains unresolved.
pub(crate) struct DegradedAuthorityState {
    prefix: RecoveredAuthorityState,
    quarantine: AuthorityRecoveryQuarantine,
}

impl DegradedAuthorityState {
    /// Consume the inseparable read-only prefix and quarantine evidence.
    pub(crate) fn into_parts(self) -> (RecoveredAuthorityState, AuthorityRecoveryQuarantine) {
        (self.prefix, self.quarantine)
    }
}

/// Non-authoritative evidence for one quarantined recovery suffix.
pub(crate) struct AuthorityRecoveryQuarantine {
    store: AuthorityStoreCoordinate,
    valid_through: Option<JournalSequenceV1>,
    first_untrusted_sequence: JournalSequenceV1,
    durable_head: JournalSequenceV1,
    reason: AuthorityRecoveryQuarantineReason,
}

impl AuthorityRecoveryQuarantine {
    /// Borrow the exact store incarnation whose prefix was observed.
    #[must_use]
    pub(crate) const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Return the final sequence that validated physically and semantically.
    #[must_use]
    pub(crate) const fn valid_through(&self) -> Option<JournalSequenceV1> {
        self.valid_through
    }

    /// Return the first sequence excluded from the valid prefix.
    #[must_use]
    pub(crate) const fn first_untrusted_sequence(&self) -> JournalSequenceV1 {
        self.first_untrusted_sequence
    }

    /// Return the durable head observed with the quarantined prefix.
    #[must_use]
    pub(crate) const fn durable_head(&self) -> JournalSequenceV1 {
        self.durable_head
    }

    /// Borrow the typed reason why recovery stopped.
    #[must_use]
    pub(crate) const fn reason(&self) -> &AuthorityRecoveryQuarantineReason {
        &self.reason
    }
}

/// Exact validation layer that first rejected a quarantined sequence.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AuthorityRecoveryQuarantineReason {
    /// Adapter-owned framing or integrity validation stopped before this entry.
    #[error("adapter physical integrity stopped before the entry")]
    PhysicalIntegrity,
    /// Entry bytes are outside the strict canonical JSON grammar.
    #[error("entry violates the canonical JSON grammar: {0}")]
    CanonicalGrammar(#[source] pale_core::canon::CanonError),
    /// Canonical JSON does not decode as the private typed journal entry.
    #[error("entry does not decode as the typed journal record: {0}")]
    TypedDecode(#[source] serde_json::Error),
    /// Typed entry bytes are not their unique canonical JCS representation.
    #[error("entry bytes are not their canonical JCS representation")]
    NonCanonical,
    /// The entry names no supported private journal format.
    #[error("entry names an unsupported private journal format")]
    UnsupportedFormat,
    /// A delta's predecessor or successor fingerprint contradicts the accepted fold.
    #[error("entry contradicts the accepted prefix or derived successor state")]
    FoldContradiction,
    /// The successor violates lifecycle semantics under complete owner custody.
    #[error("entry successor violates lifecycle semantics: {0}")]
    Lifecycle(#[source] GenerationLifecycleError),
}

/// Semantic fold state for one physically and semantically valid journal prefix.
///
/// The lifecycle and inclusive head move together so recovery cannot advance
/// one coordinate without the other. A failed admission seals the current
/// prefix into [`RecoveryStep::Degraded`] together with the first rejected
/// coordinate; the degraded branch exposes no writer.
struct RecoveryFoldState {
    head: Option<JournalSequenceV1>,
    authority: ResidentAuthorityState,
}

impl RecoveryFoldState {
    /// Start from either an admitted checkpoint frontier or genesis.
    fn new(head: Option<JournalSequenceV1>, authority: ResidentAuthorityState) -> Self {
        Self { head, authority }
    }

    /// Fold one entry after its batch's sequence and physical boundary checks.
    fn admit_entry(
        self,
        entry: pale_ports::SequencedAuthorityEntry,
        store: &AuthorityStoreCoordinate,
        maximum_entry_bytes: usize,
        durable_head: Option<JournalSequenceV1>,
        resolver: &dyn GenerationLifecycleRecoveryResolver,
        custody: &mut RecoveryFoldCustody<'_>,
    ) -> Result<RecoveryStep, AuthorityJournalError> {
        let durable = require_durable_at_least(durable_head, entry.sequence())?;
        let decoded = match AuthorityJournalEntry::decode(&entry, store, maximum_entry_bytes) {
            Ok(decoded) => decoded,
            Err(error) => {
                return Ok(RecoveryStep::Degraded(self.degrade(
                    store,
                    entry.sequence(),
                    durable,
                    quarantine_reason(error)?,
                )));
            }
        };
        if let Some(basis) = decoded.event_custody_basis() {
            basis
                .verify_members(store, &entry, custody.events, custody.maximum_event_bytes)
                .map_err(|error| AuthorityJournalError::EventCustody(Box::new(error)))?;
        }
        let authority = match decoded.fold(&self.authority, resolver, &custody.sessions) {
            Ok(authority) => authority,
            Err(error) => {
                return Ok(RecoveryStep::Degraded(self.degrade(
                    store,
                    entry.sequence(),
                    durable,
                    quarantine_reason(error)?,
                )));
            }
        };
        Ok(RecoveryStep::Admitted(Self {
            head: Some(entry.sequence()),
            authority,
        }))
    }

    /// Project one rejected suffix without exposing the transient lifecycle.
    fn quarantine(
        &self,
        store: &AuthorityStoreCoordinate,
        first_untrusted_sequence: JournalSequenceV1,
        durable_head: JournalSequenceV1,
        reason: AuthorityRecoveryQuarantineReason,
    ) -> AuthorityRecoveryQuarantine {
        AuthorityRecoveryQuarantine {
            store: store.clone(),
            valid_through: self.head,
            first_untrusted_sequence,
            durable_head,
            reason,
        }
    }

    /// Seal the accepted prefix and rejected-suffix boundary without a writer.
    fn degrade(
        self,
        store: &AuthorityStoreCoordinate,
        first_untrusted_sequence: JournalSequenceV1,
        durable_head: JournalSequenceV1,
        reason: AuthorityRecoveryQuarantineReason,
    ) -> DegradedAuthorityState {
        let quarantine = self.quarantine(store, first_untrusted_sequence, durable_head, reason);
        DegradedAuthorityState {
            prefix: self.complete(store.clone()),
            quarantine,
        }
    }

    /// Seal the completely recovered semantic prefix with its store coordinate.
    fn complete(self, store: AuthorityStoreCoordinate) -> RecoveredAuthorityState {
        RecoveredAuthorityState {
            store,
            authority: self.authority,
            journal_head: self.head,
        }
    }
}

/// Result of admitting one physically valid journal entry.
enum RecoveryStep {
    /// The exact semantic prefix advanced by one entry.
    Admitted(RecoveryFoldState),
    /// The accepted prefix is readable but the suffix is quarantined.
    Degraded(DegradedAuthorityState),
}

/// Encode one handle-free private recovery image as exact canonical JCS bytes.
pub(crate) fn encode_recovery_checkpoint(
    store: &AuthorityStoreCoordinate,
    through_sequence: JournalSequenceV1,
    authority: &ResidentAuthorityState,
    maximum_bytes: usize,
) -> Result<BoundedRecoveryCheckpointBytes, AuthorityJournalError> {
    let state = ResidentAuthorityRecoveryStateV1::from_authority(authority);
    if let Some(sequence) = state
        .maximum_journal_sequence()
        .filter(|sequence| sequence.get() > through_sequence.get())
    {
        return Err(AuthorityJournalError::SequenceMismatch {
            expected: through_sequence.get(),
            observed: sequence.get(),
        });
    }
    let image = RecoveryImageV1 {
        format: AUTHORITY_RECOVERY_IMAGE_FORMAT_V1.to_owned(),
        control_store_id: store.store_id().clone(),
        control_incarnation_id: store.incarnation_id().clone(),
        through_sequence,
        authority: state,
    };
    let canonical = canon::to_canonical_string(&image)?;
    BoundedRecoveryCheckpointBytes::new(canonical.into_bytes(), maximum_bytes)
        .map_err(AuthorityJournalError::Store)
}

/// Reconstruct resident authority from a journal-bound checkpoint and suffix.
///
/// Candidates are decoded and re-admitted before reading their journal
/// frontier. A candidate at `k` must match the successor fingerprint of entry
/// `k`; the same read supplies every later entry to replay. That binding entry
/// consumes one slot of [`AuthorityRecoveryReadLimits::maximum_tail_entries`].
/// A checkpoint's earlier Event entry consumes one additional slot under that
/// same count; it is reused when it is already the checkpoint binding entry.
/// Invalid cache bytes, semantics, or correspondence may fall back to an older
/// candidate, then genesis. Missing owner context and custody contradictions
/// remain fatal. Fallback reads cannot regress an already observed durable head.
///
/// Each locally admissible candidate fixes one durable head and reads bounded
/// pages through that head. Pages from different fixed-head observations are
/// never merged. Compaction must retain a checkpoint's binding entry; a cache
/// hash cannot replace it.
///
/// A physically or semantically invalid entry retains only the read-only prefix
/// defined by the fold recurrence on [`crate::resident::ResidentApplication`].
/// Quarantine starts at the following
/// entry, and the prefix cannot resume a writer.
///
/// This proves correspondence to retained journal custody, not independent
/// anti-rollback, origin authentication, selected durability, or Pale Runtime assembly.
///
/// # Errors
///
/// Returns [`AuthorityJournalError`] when custody, bounds, sequence coordinates,
/// the durable head, or required owner context cannot be proven. Invalid entry
/// encoding or fold semantics produce a typed quarantine outcome instead.
pub(crate) fn recover_authority(
    custody: AuthorityRecoveryCustody<'_>,
    store: AuthorityStoreCoordinate,
    limits: AuthorityRecoveryReadLimits,
    genesis: ResidentAuthorityState,
    resolver: &dyn GenerationLifecycleRecoveryResolver,
) -> Result<AuthorityRecoveryOutcome, AuthorityJournalError> {
    let port = custody.journal;
    let session_leases = SessionLeaseRecoveryContext::new(
        custody.session_leases,
        store.clone(),
        custody.maximum_session_lease_bytes,
    );
    let mut fold_custody = RecoveryFoldCustody {
        sessions: session_leases,
        events: custody.events,
        maximum_event_bytes: custody.maximum_event_bytes,
    };
    let checkpoints =
        port.read_recovery_checkpoints(&RecoveryCheckpointRead::new(store.clone(), limits))?;
    if checkpoints.len() > limits.maximum_checkpoint_candidates() {
        return Err(AuthorityJournalError::Store(
            DurableAuthorityStoreErrorKind::BoundExceeded {
                class: AuthorityByteClass::RecoveryCheckpoint,
            }
            .into(),
        ));
    }
    let mut prior_sequence = None;
    for candidate in &checkpoints {
        if prior_sequence.is_some_and(|prior: u64| prior <= candidate.through_sequence().get()) {
            return Err(AuthorityJournalError::CheckpointOrder);
        }
        prior_sequence = Some(candidate.through_sequence().get());
        if candidate.trusted().is_some_and(|checkpoint| {
            checkpoint.bytes().as_slice().len() > limits.maximum_checkpoint_bytes()
        }) {
            return Err(AuthorityJournalError::Store(
                DurableAuthorityStoreErrorKind::BoundExceeded {
                    class: AuthorityByteClass::RecoveryCheckpoint,
                }
                .into(),
            ));
        }
    }
    let mut observed_head = None;
    let mut selected = None;
    for checkpoint in checkpoints
        .iter()
        .filter_map(|candidate| candidate.trusted())
    {
        let Ok(image) = decode_checkpoint(
            checkpoint.bytes().as_slice(),
            checkpoint.through_sequence(),
            &store,
            limits.maximum_checkpoint_bytes(),
        ) else {
            continue;
        };
        let authority = match image.authority.admit(resolver, &fold_custody.sessions) {
            Ok(authority) => authority,
            Err(
                error @ (GenerationLifecycleError::MissingRecoveryContext(_)
                | GenerationLifecycleError::RecoveryContextSubstitution(_)
                | GenerationLifecycleError::SessionLeaseCustody(_)
                | GenerationLifecycleError::SessionLeaseCanonical(_)
                | GenerationLifecycleError::SessionLeaseDecode(_)
                | GenerationLifecycleError::SessionLeaseNonCanonical
                | GenerationLifecycleError::SessionLeaseContract(_)
                | GenerationLifecycleError::SessionLeaseBoundExceeded
                | GenerationLifecycleError::SessionLeaseAdmission(_)),
            ) => {
                return Err(AuthorityJournalError::Lifecycle(error));
            }
            Err(_) => continue,
        };
        // Sequence one has no predecessor; a portable sequence zero is invalid.
        let after = JournalSequenceV1::new(image.through_sequence.get() - 1).ok();
        let minimum_head = observed_head.max(Some(image.through_sequence));
        let tail = read_checked_tail(port, &store, after, minimum_head, limits)?;
        observed_head = tail.durable_head;
        let Some(binding) = tail.entries.first() else {
            // A consistent physical quarantine at k cannot be skipped by C_k.
            continue;
        };
        let corresponds =
            match AuthorityJournalEntry::decode(binding, &store, limits.maximum_entry_bytes())
                .and_then(|entry| entry.matches_checkpoint(&authority))
            {
                Ok(corresponds) => corresponds,
                Err(error) => {
                    // Only application-invalid entries permit an earlier recovery basis.
                    // Custody/coordinate errors remain fatal through this classifier.
                    quarantine_reason(error)?;
                    false
                }
            };
        if corresponds {
            // The binding entry may be a later lifecycle or process change.
            // The last Event basis still needs its own original entry and members.
            if let Some(basis) = authority.event_custody().basis() {
                let extra_entry = usize::from(basis.sequence() != binding.sequence());
                if tail
                    .entries
                    .len()
                    .checked_add(extra_entry)
                    .is_none_or(|count| count > limits.maximum_tail_entries())
                {
                    return Err(AuthorityJournalError::Store(
                        DurableAuthorityStoreErrorKind::BoundExceeded {
                            class: AuthorityByteClass::JournalEntry,
                        }
                        .into(),
                    ));
                }
                let historical;
                let original = if extra_entry == 0 {
                    binding
                } else {
                    let after = JournalSequenceV1::new(basis.sequence().get() - 1).ok();
                    let request = JournalTailPageRead::new(
                        store.clone(),
                        after,
                        basis.sequence(),
                        limits,
                        1,
                    )?;
                    historical = port.read_journal_page(&request)?;
                    let [original] = historical.entries() else {
                        return Err(AuthorityJournalError::FoldContradiction);
                    };
                    if !matches!(historical.boundary(), JournalTailPageBoundary::Complete)
                        || original.sequence() != basis.sequence()
                    {
                        return Err(AuthorityJournalError::FoldContradiction);
                    }
                    // A historical point read must not erase the greater head that
                    // selected this checkpoint/tail observation.
                    if let Some(required) = tail.durable_head {
                        require_durable_at_least(port.read_journal_head(&store)?, required)?;
                    }
                    original
                };
                let decoded =
                    AuthorityJournalEntry::decode(original, &store, limits.maximum_entry_bytes())?;
                if decoded.event_custody_basis() != Some(basis) {
                    return Err(AuthorityJournalError::FoldContradiction);
                }
                basis
                    .verify_members(
                        &store,
                        original,
                        fold_custody.events,
                        fold_custody.maximum_event_bytes,
                    )
                    .map_err(|error| AuthorityJournalError::EventCustody(Box::new(error)))?;
            }
            selected = Some((
                RecoveryFoldState::new(Some(image.through_sequence), authority),
                tail,
            ));
            break;
        }
    }
    let (mut state, tail) = match selected {
        Some(selected) => selected,
        None => (
            RecoveryFoldState::new(None, genesis),
            read_checked_tail(port, &store, None, observed_head, limits)?,
        ),
    };
    let CheckedJournalTail {
        entries,
        durable_head,
        boundary,
    } = tail;
    // A selected checkpoint already represents its journal binding entry.
    let skip_binding = usize::from(state.head.is_some());
    for entry in entries.into_iter().skip(skip_binding) {
        state = match state.admit_entry(
            entry,
            &store,
            limits.maximum_entry_bytes(),
            durable_head,
            resolver,
            &mut fold_custody,
        )? {
            RecoveryStep::Admitted(state) => state,
            RecoveryStep::Degraded(degraded) => {
                return Ok(AuthorityRecoveryOutcome::Degraded(Box::new(degraded)));
            }
        };
    }
    match boundary {
        CheckedJournalTailBoundary::Complete => Ok(AuthorityRecoveryOutcome::Complete(Box::new(
            state.complete(store),
        ))),
        CheckedJournalTailBoundary::QuarantinedFrom {
            first_untrusted_sequence,
        } => {
            let durable = require_durable_at_least(durable_head, first_untrusted_sequence)?;
            Ok(AuthorityRecoveryOutcome::Degraded(Box::new(state.degrade(
                &store,
                first_untrusted_sequence,
                durable,
                AuthorityRecoveryQuarantineReason::PhysicalIntegrity,
            ))))
        }
    }
}

/// Final physical boundary of one completely consumed fixed-head page sequence.
enum CheckedJournalTailBoundary {
    /// Every physical row through the fixed head was returned.
    Complete,
    /// Physical validation stopped immediately before this row.
    QuarantinedFrom {
        /// First sequence whose physical bytes are not trusted.
        first_untrusted_sequence: JournalSequenceV1,
    },
}

/// Completely consumed bounded page sequence from one fixed durable head.
struct CheckedJournalTail {
    /// Physically valid entries in exact ascending order.
    entries: Vec<SequencedAuthorityEntry>,
    /// Fixed store-global head observed before the first page.
    durable_head: Option<JournalSequenceV1>,
    /// Exact terminal boundary reached by the final page.
    boundary: CheckedJournalTailBoundary,
}

/// Read one bounded fixed-head page sequence before trusting a cache.
///
/// `minimum_head` retains the greatest prior observation across fallback reads;
/// it is not an independently retained anti-rollback anchor. This function
/// consumes pages only through the first observed head, retains the cumulative
/// recovery budget across continuations, and does not decode entries or infer
/// semantic validity from adapter checksums.
fn read_checked_tail(
    port: &dyn DurableAuthorityStorePort,
    store: &AuthorityStoreCoordinate,
    after: Option<JournalSequenceV1>,
    minimum_head: Option<JournalSequenceV1>,
    limits: AuthorityRecoveryReadLimits,
) -> Result<CheckedJournalTail, AuthorityJournalError> {
    let durable_head = port.read_journal_head(store)?;
    for required in [minimum_head, after].into_iter().flatten() {
        require_durable_at_least(durable_head, required)?;
    }
    let Some(through) = durable_head else {
        return Ok(CheckedJournalTail {
            entries: Vec::new(),
            durable_head,
            boundary: CheckedJournalTailBoundary::Complete,
        });
    };
    if after == Some(through) {
        return Ok(CheckedJournalTail {
            entries: Vec::new(),
            durable_head,
            boundary: CheckedJournalTailBoundary::Complete,
        });
    }

    let maximum_page_entries = limits
        .maximum_tail_entries()
        .min(AUTHORITY_RECOVERY_PAGE_ENTRIES);
    let mut request =
        JournalTailPageRead::new(store.clone(), after, through, limits, maximum_page_entries)?;
    let mut entries = Vec::new();
    loop {
        let page = port.read_journal_page(&request)?;
        for entry in page.entries() {
            if entry.bytes().as_slice().len() > limits.maximum_entry_bytes() {
                return Err(AuthorityJournalError::Store(
                    DurableAuthorityStoreErrorKind::BoundExceeded {
                        class: AuthorityByteClass::JournalEntry,
                    }
                    .into(),
                ));
            }
        }
        entries.extend_from_slice(page.entries());
        match page.boundary() {
            JournalTailPageBoundary::Complete => {
                return Ok(CheckedJournalTail {
                    entries,
                    durable_head,
                    boundary: CheckedJournalTailBoundary::Complete,
                });
            }
            JournalTailPageBoundary::QuarantinedFrom {
                first_untrusted_sequence,
            } => {
                return Ok(CheckedJournalTail {
                    entries,
                    durable_head,
                    boundary: CheckedJournalTailBoundary::QuarantinedFrom {
                        first_untrusted_sequence,
                    },
                });
            }
            JournalTailPageBoundary::More { .. } => {
                request = request.continue_after(&page)?.ok_or_else(|| {
                    AuthorityJournalError::Store(
                        DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry)
                            .into(),
                    )
                })?;
            }
        }
    }
}

/// Prove that one co-observed durable head contains a supplied tail coordinate.
fn require_durable_at_least(
    durable_head: Option<JournalSequenceV1>,
    required: JournalSequenceV1,
) -> Result<JournalSequenceV1, AuthorityJournalError> {
    match durable_head {
        Some(durable) if durable.get() >= required.get() => Ok(durable),
        observed => Err(AuthorityJournalError::DurableHeadContradiction {
            required: required.get(),
            observed: observed.map(JournalSequenceV1::get),
        }),
    }
}

/// Separate entry quarantine from fatal failures of custody or coordinates.
fn quarantine_reason(
    error: AuthorityJournalError,
) -> Result<AuthorityRecoveryQuarantineReason, AuthorityJournalError> {
    match error {
        AuthorityJournalError::Canonical(error) => {
            Ok(AuthorityRecoveryQuarantineReason::CanonicalGrammar(error))
        }
        AuthorityJournalError::Decode(error) => {
            Ok(AuthorityRecoveryQuarantineReason::TypedDecode(error))
        }
        AuthorityJournalError::NonCanonical(_) => {
            Ok(AuthorityRecoveryQuarantineReason::NonCanonical)
        }
        AuthorityJournalError::WrongFormat(_) => {
            Ok(AuthorityRecoveryQuarantineReason::UnsupportedFormat)
        }
        AuthorityJournalError::FoldContradiction => {
            Ok(AuthorityRecoveryQuarantineReason::FoldContradiction)
        }
        AuthorityJournalError::UnchangedDesiredProcessState => {
            Ok(AuthorityRecoveryQuarantineReason::FoldContradiction)
        }
        AuthorityJournalError::Lifecycle(
            error @ (GenerationLifecycleError::MissingRecoveryContext(_)
            | GenerationLifecycleError::RecoveryContextSubstitution(_)
            | GenerationLifecycleError::SessionLeaseCustody(_)
            | GenerationLifecycleError::SessionLeaseCanonical(_)
            | GenerationLifecycleError::SessionLeaseDecode(_)
            | GenerationLifecycleError::SessionLeaseNonCanonical
            | GenerationLifecycleError::SessionLeaseContract(_)
            | GenerationLifecycleError::SessionLeaseBoundExceeded
            | GenerationLifecycleError::SessionLeaseAdmission(_)),
        ) => Err(AuthorityJournalError::Lifecycle(error)),
        AuthorityJournalError::Lifecycle(error) => {
            Ok(AuthorityRecoveryQuarantineReason::Lifecycle(error))
        }
        fatal => Err(fatal),
    }
}

/// Decode one candidate checkpoint and prove its canonical envelope coordinates.
fn decode_checkpoint(
    bytes: &[u8],
    supplied_sequence: JournalSequenceV1,
    store: &AuthorityStoreCoordinate,
    maximum_bytes: usize,
) -> Result<RecoveryImageV1, AuthorityJournalError> {
    let value = canon::parse_strict(bytes)?;
    let image: RecoveryImageV1 = serde_json::from_value(value)?;
    let canonical = canon::to_canonical_string(&image)?;
    if canonical.as_bytes() != bytes {
        return Err(AuthorityJournalError::NonCanonical("recovery checkpoint"));
    }
    if canonical.len() > maximum_bytes {
        return Err(AuthorityJournalError::Store(
            DurableAuthorityStoreErrorKind::BoundExceeded {
                class: AuthorityByteClass::RecoveryCheckpoint,
            }
            .into(),
        ));
    }
    if image.format != AUTHORITY_RECOVERY_IMAGE_FORMAT_V1 {
        return Err(AuthorityJournalError::WrongFormat("recovery checkpoint"));
    }
    if image.control_store_id != *store.store_id()
        || image.control_incarnation_id != *store.incarnation_id()
    {
        return Err(AuthorityJournalError::StoreCoordinateMismatch);
    }
    if image.through_sequence != supplied_sequence {
        return Err(AuthorityJournalError::SequenceMismatch {
            expected: supplied_sequence.get(),
            observed: image.through_sequence.get(),
        });
    }
    if let Some(sequence) = image
        .authority
        .maximum_journal_sequence()
        .filter(|sequence| sequence.get() > image.through_sequence.get())
    {
        return Err(AuthorityJournalError::SequenceMismatch {
            expected: image.through_sequence.get(),
            observed: sequence.get(),
        });
    }
    Ok(image)
}
