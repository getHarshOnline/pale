//! Resident authority-journal coordination and recovery-law tests.
//!
//! The suite proves exact canonical replay, checkpoint projection, fail-closed
//! custody, and a read-only validated prefix at the first quarantined coordinate.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error as _;
use std::sync::Mutex;

#[path = "journal/event_custody.rs"]
pub(in crate::resident) mod event_custody;

use pale_core::records::{
    ActivationFenceV1, ActiveGenerationHeadV1, CompiledGenerationV1, ControlIncarnationIdV1,
    ControlStoreIdV1, JournalSequenceV1, SchemaDigest, SchemaId, SessionAdmissionLeaseDigest,
    SessionAdmissionLeaseV1, SessionAdmissionLeaseWireV1,
};
use pale_ports::{
    AuthorityRecoveryReadLimits, AuthorityStoreCoordinate, BoundedAuthorityEntryBytes,
    BoundedRecoveryCheckpointBytes, DurableAuthorityStoreError, DurableAuthorityStoreErrorKind,
    DurableAuthorityStorePort, JournalTailPage, JournalTailPageBoundary, JournalTailPageRead,
    PreparedAuthorityAppend, PreparedRecoveryCheckpoint, RecoveryCheckpointCandidate,
    RecoveryCheckpointPhysicalRejection, RecoveryCheckpointRead, SequencedAuthorityEntry,
    SessionLeaseBody, SessionLeaseBodyRead, SessionLeaseBodyRetention, SessionLeaseCustodyPort,
    StoredRecoveryCheckpoint,
};

use super::recovery::{
    recover_authority as recover_composite_authority, AuthorityRecoveryOutcome,
    AuthorityRecoveryQuarantine, AuthorityRecoveryQuarantineReason, RecoveredAuthorityState,
};
use super::*;
use crate::resident::authority::ResidentAuthorityState;
use crate::resident::lifecycle::tests::{compiled_generation, effective_profile};
use crate::resident::lifecycle::{GenerationLifecycleRecoveryResolver, RecoveryReferenceKnowledge};
use crate::resident::session_lease_custody::SessionLeaseRecoveryContext;
use crate::resident::session_lease_custody::{
    recover_session_lease_body, retain_session_lease_body,
};
use crate::resident::DesiredProcessState;
use crate::resident_fixture::{
    active, advertised, fence, published, session_lease_for_profile, RecoveryResolver,
};

#[derive(Default)]
pub(in crate::resident) struct MemoryAuthorityStore {
    entries: Mutex<Vec<SequencedAuthorityEntry>>,
    checkpoints: Mutex<Vec<StoredRecoveryCheckpoint>>,
    checkpoint_prefix: Mutex<Vec<RecoveryCheckpointCandidate>>,
    append_results: Mutex<VecDeque<TestAppendDisposition>>,
    checkpoint_results: Mutex<VecDeque<Result<(), DurableAuthorityStoreError>>>,
    append_calls: Mutex<usize>,
    tail_take: Mutex<Option<usize>>,
    tail_reads: Mutex<Vec<Option<JournalSequenceV1>>>,
    head_results: Mutex<VecDeque<Result<Option<JournalSequenceV1>, DurableAuthorityStoreError>>>,
    page_errors: Mutex<VecDeque<DurableAuthorityStoreError>>,
    physical_quarantine_from: Mutex<Option<JournalSequenceV1>>,
    session_lease_bodies: Mutex<BTreeMap<SessionAdmissionLeaseDigest, SessionLeaseBody>>,
    session_lease_body_results: Mutex<VecDeque<TestSessionLeaseBodyDisposition>>,
}

/// Closed physical outcomes exercised by the lease-body custody test double.
enum TestSessionLeaseBodyDisposition {
    /// Return this result before retaining any body.
    BeforeRetain(Result<(), pale_ports::ControlStoreError>),
    /// Retain the complete body, then report the outcome as ambiguous.
    RetainThenAmbiguous,
}

/// Read-only hostile custody seam for application-owned re-admission tests.
struct ObservedSessionLeaseBody(Option<SessionLeaseBody>);

impl SessionLeaseCustodyPort for ObservedSessionLeaseBody {
    fn compare_and_retain(
        &self,
        _body: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, pale_ports::ControlStoreError> {
        Err(pale_ports::ControlStoreError::new(
            pale_ports::ControlStoreErrorKind::Storage,
        ))
    }

    fn read(
        &self,
        _request: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, pale_ports::ControlStoreError> {
        Ok(self.0.clone())
    }
}

impl SessionLeaseCustodyPort for MemoryAuthorityStore {
    fn compare_and_retain(
        &self,
        body: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, pale_ports::ControlStoreError> {
        if body.store() != &store() {
            return Err(pale_ports::ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::Storage,
            ));
        }
        let disposition = self
            .session_lease_body_results
            .lock()
            .expect("session body results")
            .pop_front();
        let retain_then_ambiguous = match disposition {
            Some(TestSessionLeaseBodyDisposition::BeforeRetain(result)) => {
                result?;
                false
            }
            Some(TestSessionLeaseBodyDisposition::RetainThenAmbiguous) => true,
            None => false,
        };
        let mut bodies = self.session_lease_bodies.lock().expect("session bodies");
        let retained = match bodies.get(body.digest()) {
            None => {
                bodies.insert(body.digest().clone(), body.clone());
                SessionLeaseBodyRetention::Retained
            }
            Some(existing) if existing == body => SessionLeaseBodyRetention::ExactReplay,
            Some(_) => {
                return Err(pale_ports::ControlStoreError::new(
                    pale_ports::ControlStoreErrorKind::RecoveryRequired,
                ));
            }
        };
        if retain_then_ambiguous {
            Err(pale_ports::ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::OutcomeUnknown,
            ))
        } else {
            Ok(retained)
        }
    }

    fn read(
        &self,
        request: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, pale_ports::ControlStoreError> {
        if request.store() != &store() {
            return Err(pale_ports::ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::Storage,
            ));
        }
        let body = self
            .session_lease_bodies
            .lock()
            .expect("session bodies")
            .get(request.digest())
            .cloned();
        if body.as_ref().is_some_and(|body| {
            body.canonical_wire_jcs().as_slice().len() > request.maximum_body_bytes()
        }) {
            return Err(pale_ports::ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        Ok(body)
    }
}

/// Closed physical outcomes exercised by the in-memory durability test double.
enum TestAppendDisposition {
    /// Return this result before any durable append occurs.
    BeforeAppend(Result<(), DurableAuthorityStoreError>),
    /// Append durably, then report the outcome as ambiguous to the caller.
    CommitThenAmbiguous,
}

impl MemoryAuthorityStore {
    pub(in crate::resident) fn entry_count(&self) -> usize {
        self.entries.lock().expect("entries").len()
    }

    pub(in crate::resident) fn enqueue(&self, result: Result<(), DurableAuthorityStoreError>) {
        self.append_results
            .lock()
            .expect("append results")
            .push_back(TestAppendDisposition::BeforeAppend(result));
    }

    pub(in crate::resident) fn enqueue_commit_then_ambiguous(&self) {
        self.append_results
            .lock()
            .expect("append results")
            .push_back(TestAppendDisposition::CommitThenAmbiguous);
    }

    pub(in crate::resident) fn enqueue_session_body(
        &self,
        result: Result<(), pale_ports::ControlStoreError>,
    ) {
        self.session_lease_body_results
            .lock()
            .expect("session body results")
            .push_back(TestSessionLeaseBodyDisposition::BeforeRetain(result));
    }

    pub(in crate::resident) fn enqueue_session_body_retain_then_ambiguous(&self) {
        self.session_lease_body_results
            .lock()
            .expect("session body results")
            .push_back(TestSessionLeaseBodyDisposition::RetainThenAmbiguous);
    }

    pub(in crate::resident) fn session_body_count(&self) -> usize {
        self.session_lease_bodies
            .lock()
            .expect("session bodies")
            .len()
    }

    fn enqueue_checkpoint(&self, result: Result<(), DurableAuthorityStoreError>) {
        self.checkpoint_results
            .lock()
            .expect("checkpoint results")
            .push_back(result);
    }

    fn reject_checkpoint_at(&self, sequence: u64, reason: RecoveryCheckpointPhysicalRejection) {
        self.checkpoint_prefix
            .lock()
            .expect("checkpoint prefix")
            .push(RecoveryCheckpointCandidate::Rejected {
                through_sequence: JournalSequenceV1::new(sequence).expect("checkpoint sequence"),
                reason,
            });
    }

    pub(in crate::resident) fn append_calls(&self) -> usize {
        *self.append_calls.lock().expect("append calls")
    }

    fn truncate_tail_to(&self, entries: usize) {
        *self.tail_take.lock().expect("tail bound") = Some(entries);
    }

    pub(in crate::resident) fn quarantine_from(&self, sequence: u64) {
        *self
            .physical_quarantine_from
            .lock()
            .expect("physical quarantine") =
            Some(JournalSequenceV1::new(sequence).expect("quarantine sequence"));
    }
}

impl DurableAuthorityStorePort for MemoryAuthorityStore {
    fn compare_and_append(
        &self,
        append: &PreparedAuthorityAppend,
    ) -> Result<(), DurableAuthorityStoreError> {
        *self.append_calls.lock().expect("append calls") += 1;
        let disposition = self
            .append_results
            .lock()
            .expect("append results")
            .pop_front();
        let commit_then_ambiguous = match disposition {
            Some(TestAppendDisposition::BeforeAppend(result)) => {
                result?;
                false
            }
            Some(TestAppendDisposition::CommitThenAmbiguous) => true,
            None => false,
        };
        let mut entries = self.entries.lock().expect("entries");
        let observed = entries.last().map(SequencedAuthorityEntry::sequence);
        if observed != append.expected_head() {
            return Err(DurableAuthorityStoreErrorKind::Conflict.into());
        }
        entries.push(append.entry().clone());
        if commit_then_ambiguous {
            Err(DurableAuthorityStoreErrorKind::AmbiguousCommit.into())
        } else {
            Ok(())
        }
    }

    fn compare_and_store_checkpoint(
        &self,
        prepared: &PreparedRecoveryCheckpoint,
    ) -> Result<(), DurableAuthorityStoreError> {
        if let Some(result) = self
            .checkpoint_results
            .lock()
            .expect("checkpoint results")
            .pop_front()
        {
            result?;
        }
        let observed = self
            .entries
            .lock()
            .expect("entries")
            .last()
            .map(SequencedAuthorityEntry::sequence);
        if observed != Some(prepared.expected_head()) {
            return Err(DurableAuthorityStoreErrorKind::Conflict.into());
        }
        self.checkpoints
            .lock()
            .expect("checkpoints")
            .insert(0, prepared.checkpoint().clone());
        Ok(())
    }

    fn read_recovery_checkpoints(
        &self,
        _request: &RecoveryCheckpointRead,
    ) -> Result<Vec<RecoveryCheckpointCandidate>, DurableAuthorityStoreError> {
        let mut candidates = self
            .checkpoint_prefix
            .lock()
            .expect("checkpoint prefix")
            .clone();
        candidates.extend(
            self.checkpoints
                .lock()
                .expect("checkpoints")
                .iter()
                .cloned()
                .map(RecoveryCheckpointCandidate::Trusted),
        );
        Ok(candidates)
    }

    fn read_journal_head(
        &self,
        _store: &AuthorityStoreCoordinate,
    ) -> Result<Option<JournalSequenceV1>, DurableAuthorityStoreError> {
        if let Some(result) = self.head_results.lock().expect("head results").pop_front() {
            return result;
        }
        Ok(self
            .entries
            .lock()
            .expect("entries")
            .last()
            .map(SequencedAuthorityEntry::sequence))
    }

    fn read_journal_page(
        &self,
        request: &JournalTailPageRead,
    ) -> Result<JournalTailPage, DurableAuthorityStoreError> {
        self.tail_reads
            .lock()
            .expect("tail reads")
            .push(request.after());
        if let Some(error) = self.page_errors.lock().expect("page errors").pop_front() {
            return Err(error);
        }
        let entries = self.entries.lock().expect("entries");
        let take = self
            .tail_take
            .lock()
            .expect("tail bound")
            .unwrap_or(request.maximum_page_entries())
            .min(request.maximum_page_entries());
        let quarantine_from = *self
            .physical_quarantine_from
            .lock()
            .expect("physical quarantine");
        let suffix = entries
            .iter()
            .filter(|entry| {
                request
                    .after()
                    .is_none_or(|head| entry.sequence().get() > head.get())
            })
            .filter(|entry| entry.sequence() <= request.through())
            .filter(|entry| {
                quarantine_from.is_none_or(|first| entry.sequence().get() < first.get())
            })
            .take(take)
            .cloned()
            .collect::<Vec<_>>();
        let next = suffix
            .last()
            .map(SequencedAuthorityEntry::sequence)
            .or(request.after())
            .map_or_else(
                || JournalSequenceV1::new(1).ok(),
                JournalSequenceV1::checked_successor,
            );
        let boundary = if let Some(first_untrusted_sequence) =
            quarantine_from.filter(|first| next == Some(*first))
        {
            JournalTailPageBoundary::QuarantinedFrom {
                first_untrusted_sequence,
            }
        } else if suffix
            .last()
            .is_some_and(|entry| entry.sequence() == request.through())
        {
            JournalTailPageBoundary::Complete
        } else if let Some(resume_after) = suffix.last().map(SequencedAuthorityEntry::sequence) {
            JournalTailPageBoundary::More { resume_after }
        } else {
            return Err(DurableAuthorityStoreErrorKind::Corrupt(
                pale_ports::AuthorityByteClass::JournalEntry,
            )
            .into());
        };
        JournalTailPage::new(request, suffix, boundary)
    }
}

pub(in crate::resident) fn store() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("control-store").expect("store id"),
        ControlIncarnationIdV1::parse("incarnation-1").expect("incarnation id"),
    )
}

pub(in crate::resident) fn limits() -> AuthorityRecoveryReadLimits {
    AuthorityRecoveryReadLimits::new(1_000_000, 1_000_000, 8, 64).expect("limits")
}

pub(in crate::resident) fn genesis() -> GenerationLifecycle {
    GenerationLifecycle::empty(RecoveryReferenceKnowledge::Known(BTreeSet::new()))
}

fn session_lease_recovery(port: &MemoryAuthorityStore) -> SessionLeaseRecoveryContext<'_> {
    SessionLeaseRecoveryContext::new(port, store(), limits().maximum_entry_bytes())
}

/// Adapt lifecycle-only fixtures to the complete private authority product.
fn recover_authority<P>(
    port: &P,
    store: AuthorityStoreCoordinate,
    limits: AuthorityRecoveryReadLimits,
    genesis: GenerationLifecycle,
    resolver: &dyn GenerationLifecycleRecoveryResolver,
) -> Result<AuthorityRecoveryOutcome, AuthorityJournalError>
where
    P: DurableAuthorityStorePort + SessionLeaseCustodyPort,
{
    recover_composite_authority(
        super::recovery::AuthorityRecoveryCustody {
            journal: port,
            session_leases: port,
            events: &mut event_custody::UnavailableEventCustody,
            maximum_session_lease_bytes: limits.maximum_entry_bytes(),
            maximum_event_bytes: limits.maximum_entry_bytes(),
        },
        store,
        limits,
        ResidentAuthorityState::new(genesis),
        resolver,
    )
}

/// Re-export the lifecycle-fixture adapter to sibling resident tests.
pub(in crate::resident) fn recover_for_resident_test<P>(
    port: &P,
    store: AuthorityStoreCoordinate,
    limits: AuthorityRecoveryReadLimits,
    genesis: GenerationLifecycle,
    resolver: &dyn GenerationLifecycleRecoveryResolver,
) -> Result<AuthorityRecoveryOutcome, AuthorityJournalError>
where
    P: DurableAuthorityStorePort + SessionLeaseCustodyPort,
{
    recover_authority(port, store, limits, genesis, resolver)
}

/// Encode lifecycle-only fixtures with no invented desired process state.
fn encode_recovery_checkpoint(
    store: &AuthorityStoreCoordinate,
    through_sequence: JournalSequenceV1,
    lifecycle: &GenerationLifecycle,
    maximum_bytes: usize,
) -> Result<BoundedRecoveryCheckpointBytes, AuthorityJournalError> {
    super::encode_recovery_checkpoint(
        store,
        through_sequence,
        &ResidentAuthorityState::new(lifecycle.clone()),
        maximum_bytes,
    )
}

/// Encode a checkpoint from an already folded composite authority fixture.
fn encode_authority_recovery_checkpoint(
    store: &AuthorityStoreCoordinate,
    through_sequence: JournalSequenceV1,
    authority: &ResidentAuthorityState,
    maximum_bytes: usize,
) -> Result<BoundedRecoveryCheckpointBytes, AuthorityJournalError> {
    super::encode_recovery_checkpoint(store, through_sequence, authority, maximum_bytes)
}

/// Insert a derived cache independently of journal bytes to exercise substitution.
fn cache_lifecycle(port: &MemoryAuthorityStore, sequence: u64, state: &GenerationLifecycle) {
    let sequence = JournalSequenceV1::new(sequence).expect("checkpoint sequence");
    let bytes = encode_recovery_checkpoint(
        &store(),
        sequence,
        state,
        limits().maximum_checkpoint_bytes(),
    )
    .expect("checkpoint encodes");
    port.checkpoints
        .lock()
        .expect("checkpoints")
        .push(StoredRecoveryCheckpoint::new(sequence, bytes));
}

fn complete_recovery(outcome: AuthorityRecoveryOutcome) -> RecoveredAuthorityState {
    match outcome {
        AuthorityRecoveryOutcome::Complete(recovered) => *recovered,
        AuthorityRecoveryOutcome::Degraded(degraded) => {
            let (_, quarantine) = (*degraded).into_parts();
            panic!(
                "unexpected quarantine at {} after {:?}",
                quarantine.first_untrusted_sequence().get(),
                quarantine.valid_through().map(JournalSequenceV1::get)
            )
        }
    }
}

fn degraded_recovery(
    outcome: AuthorityRecoveryOutcome,
) -> (RecoveredAuthorityState, AuthorityRecoveryQuarantine) {
    match outcome {
        AuthorityRecoveryOutcome::Degraded(degraded) => (*degraded).into_parts(),
        AuthorityRecoveryOutcome::Complete(_) => panic!("expected degraded recovery"),
    }
}

pub(in crate::resident) fn empty_journal(
    port: &MemoryAuthorityStore,
) -> DurableAuthorityJournal<'_> {
    let recovered = complete_recovery(
        recover_authority(
            port,
            store(),
            limits(),
            genesis(),
            &RecoveryResolver::default(),
        )
        .expect("empty store recovers genesis"),
    );
    DurableAuthorityJournal::resume(port, limits(), recovered)
}

pub(in crate::resident) fn session_lease(
    generation: &CompiledGenerationV1,
    active: &ActiveGenerationHeadV1,
    fence: &ActivationFenceV1,
) -> SessionAdmissionLeaseV1 {
    session_lease_with_budget(generation, active, fence, 10)
}

fn session_lease_with_budget(
    generation: &CompiledGenerationV1,
    active: &ActiveGenerationHeadV1,
    fence: &ActivationFenceV1,
    budget_ceiling_units: u64,
) -> SessionAdmissionLeaseV1 {
    session_lease_for(generation, active, fence, "session-1", budget_ceiling_units)
}

pub(in crate::resident) fn session_lease_for(
    generation: &CompiledGenerationV1,
    active: &ActiveGenerationHeadV1,
    fence: &ActivationFenceV1,
    session_admission_id: &str,
    budget_ceiling_units: u64,
) -> SessionAdmissionLeaseV1 {
    let profile = effective_profile();
    session_lease_for_profile(
        generation,
        active,
        fence,
        &profile,
        session_admission_id,
        budget_ceiling_units,
    )
}

fn committed_initial_lifecycle(
    port: &MemoryAuthorityStore,
) -> (GenerationLifecycle, RecoveryResolver) {
    let generation = compiled_generation('4');
    let p = published(&generation, 1, None);
    let a = advertised(&p, 2, None);
    let f = fence(&generation, 1, None);
    let h = active(&a, &f, 3, None);
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(&generation);
    resolver.insert_effective_profile(&effective_profile());
    resolver.insert_published(&p);
    resolver.insert_advertised(&a);
    resolver.insert_fence(&f);
    resolver.insert_active(&h);

    let mut journal = empty_journal(port);
    let state = journal
        .commit_transition(genesis().publish(p).expect("publish transition"))
        .expect("publish commit")
        .into_parts()
        .1
        .lifecycle()
        .clone();
    let state = journal
        .commit_transition(state.advertise(a).expect("advertise transition"))
        .expect("advertise commit")
        .into_parts()
        .1
        .lifecycle()
        .clone();
    let state = journal
        .commit_transition(
            state
                .activate(
                    h,
                    f,
                    &crate::resident::generation::tests::activation_compatibility(
                        None,
                        generation.compiled_generation_digest().clone(),
                    ),
                )
                .expect("activate transition"),
        )
        .expect("activate commit")
        .into_parts()
        .1
        .lifecycle()
        .clone();
    assert_eq!(journal.head().map(JournalSequenceV1::get), Some(3));
    (state, resolver)
}

#[test]
fn desired_process_state_is_one_sparse_global_authority_projection() {
    let port = MemoryAuthorityStore::default();
    let mut journal = empty_journal(&port);
    assert_eq!(journal.desired_process_state(), None);

    let committed = journal
        .commit_desired_process_state(DesiredProcessState::Running)
        .expect("first explicit desired state commits");
    let (sequence, authority) = committed.into_parts();
    assert_eq!(sequence.get(), 1);
    assert_eq!(
        authority.desired_process_state(),
        Some(DesiredProcessState::Running)
    );
    assert_eq!(authority.lifecycle().snapshot(), genesis().snapshot());
    assert_eq!(port.append_calls(), 1);

    assert!(matches!(
        journal.commit_desired_process_state(DesiredProcessState::Running),
        Err(AuthorityJournalError::UnchangedDesiredProcessState)
    ));
    assert_eq!(port.append_calls(), 1, "idempotence spends no sequence");

    let generation = compiled_generation('4');
    let head = published(&generation, 2, None);
    let published_digest = head.published_generation_head_digest().clone();
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(&generation);
    resolver.insert_published(&head);
    let committed = journal
        .commit_transition(genesis().publish(head).expect("publication transition"))
        .expect("lifecycle and desired state share the global journal");
    assert_eq!(committed.into_parts().0.get(), 2);
    assert_eq!(
        journal.desired_process_state(),
        Some(DesiredProcessState::Running)
    );
    journal
        .store_checkpoint()
        .expect("product state checkpoints");
    drop(journal);

    let stored = port.checkpoints.lock().expect("checkpoints")[0].clone();
    let mut substituted =
        pale_core::canon::parse_strict(stored.bytes().as_slice()).expect("checkpoint parses");
    substituted["authority"]["desired_process_state"] =
        serde_json::json!({"kind": "set", "state": "stopped"});
    let substituted = BoundedRecoveryCheckpointBytes::new(
        pale_core::canon::to_canonical_string(&substituted)
            .expect("substituted checkpoint canonicalizes")
            .into_bytes(),
        limits().maximum_checkpoint_bytes(),
    )
    .expect("substituted checkpoint remains bounded");
    port.checkpoints.lock().expect("checkpoints")[0] = StoredRecoveryCheckpoint::new(
        JournalSequenceV1::new(2).expect("checkpoint sequence"),
        substituted,
    );

    let recovered = complete_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("substituted cache falls back to the journal"),
    );
    assert_eq!(
        recovered.journal_head().map(JournalSequenceV1::get),
        Some(2)
    );
    assert_eq!(
        recovered.desired_process_state(),
        Some(DesiredProcessState::Running)
    );
    assert_eq!(
        recovered.lifecycle().snapshot().published.as_ref(),
        Some(&published_digest)
    );
}

#[test]
fn canonical_entries_replay_the_exact_global_interleaving() {
    let port = MemoryAuthorityStore::default();
    let (expected, resolver) = committed_initial_lifecycle(&port);
    for entry in port.entries.lock().expect("entries").iter() {
        let parsed = pale_core::canon::parse_strict(entry.bytes().as_slice()).expect("strict JCS");
        assert_eq!(
            pale_core::canon::to_canonical_string(&parsed)
                .expect("canonical")
                .as_bytes(),
            entry.bytes().as_slice()
        );
    }
    let recovered = complete_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("recovery succeeds"),
    );
    assert_eq!(
        recovered.journal_head().map(JournalSequenceV1::get),
        Some(3)
    );
    assert_eq!(recovered.lifecycle().snapshot(), expected.snapshot());
}

#[test]
fn every_sparse_lifecycle_delta_replays_through_the_ordinary_laws() {
    let port = MemoryAuthorityStore::default();
    let mut journal = empty_journal(&port);
    let first_generation = compiled_generation('4');
    let second_generation = compiled_generation('5');
    let first_published = published(&first_generation, 1, None);
    let first_advertised = advertised(&first_published, 2, None);
    let first_fence = fence(&first_generation, 1, None);
    let first_active = active(&first_advertised, &first_fence, 3, None);
    let lease = session_lease(&first_generation, &first_active, &first_fence);
    let second_published = published(&second_generation, 6, Some(&first_published));
    let second_advertised = advertised(&second_published, 7, Some(&first_advertised));
    let second_fence = fence(&second_generation, 2, Some(&first_fence));
    let second_active = active(&second_advertised, &second_fence, 8, Some(&first_active));
    let rollback_published = published(&second_generation, 11, Some(&second_published));
    let rollback_advertised = advertised(&rollback_published, 12, Some(&second_advertised));
    let rollback_fence = fence(&second_generation, 3, Some(&second_fence));
    let rollback_active = active(
        &rollback_advertised,
        &rollback_fence,
        13,
        Some(&second_active),
    );
    let mut resolver = RecoveryResolver::default();
    for generation in [&first_generation, &second_generation] {
        resolver.insert_generation(generation);
    }
    for head in [&first_published, &second_published, &rollback_published] {
        resolver.insert_published(head);
    }
    for head in [&first_advertised, &second_advertised, &rollback_advertised] {
        resolver.insert_advertised(head);
    }
    for fence in [&first_fence, &second_fence, &rollback_fence] {
        resolver.insert_fence(fence);
    }
    for head in [&first_active, &second_active, &rollback_active] {
        resolver.insert_active(head);
    }
    resolver.insert_effective_profile(&effective_profile());
    retain_session_lease_body(&port, &store(), &lease, limits().maximum_entry_bytes())
        .expect("session lease body retains before its journal pin");

    let mut direct = genesis();
    let transition = direct.publish(first_published.clone()).expect("publish");
    direct = transition.clone().into_successor();
    let _committed = journal
        .commit_transition(transition)
        .expect("publish commit");
    let transition = direct
        .advertise(first_advertised.clone())
        .expect("advertise");
    direct = transition.clone().into_successor();
    let _committed = journal
        .commit_transition(transition)
        .expect("advertise commit");
    let transition = direct
        .activate(
            first_active.clone(),
            first_fence.clone(),
            &crate::resident::generation::tests::activation_compatibility(
                None,
                first_generation.compiled_generation_digest().clone(),
            ),
        )
        .expect("activate");
    direct = transition.clone().into_successor();
    let _committed = journal
        .commit_transition(transition)
        .expect("activate commit");
    let transition = direct.pin_session(&lease).expect("pin session");
    direct = transition.clone().into_successor();
    let _committed = journal.commit_transition(transition).expect("pin commit");
    let transition = direct
        .drain_session(
            lease.session_admission_id(),
            lease.session_admission_lease_digest(),
        )
        .expect("drain session");
    direct = transition.clone().into_successor();
    let _committed = journal.commit_transition(transition).expect("drain commit");
    let transition = direct
        .publish(second_published.clone())
        .expect("successor publish");
    direct = transition.clone().into_successor();
    let _committed = journal
        .commit_transition(transition)
        .expect("successor publish commit");
    let transition = direct
        .advertise(second_advertised.clone())
        .expect("successor advertise");
    direct = transition.clone().into_successor();
    let _committed = journal
        .commit_transition(transition)
        .expect("successor advertise commit");
    let transition = direct
        .activate(
            second_active.clone(),
            second_fence.clone(),
            &crate::resident::generation::tests::activation_compatibility(
                Some(first_generation.compiled_generation_digest().clone()),
                second_generation.compiled_generation_digest().clone(),
            ),
        )
        .expect("successor activate");
    direct = transition.clone().into_successor();
    let _committed = journal
        .commit_transition(transition)
        .expect("successor activate commit");
    let transition = direct
        .reclaim(first_generation.compiled_generation_digest())
        .expect("reclaim predecessor");
    direct = transition.clone().into_successor();
    let _committed = journal
        .commit_transition(transition)
        .expect("reclaim commit");
    let transition = direct
        .retain_for_rollback(second_generation.compiled_generation_digest().clone())
        .expect("retain rollback");
    direct = transition.clone().into_successor();
    let _committed = journal
        .commit_transition(transition)
        .expect("retain commit");
    let transition = direct
        .publish(rollback_published)
        .expect("rollback publication");
    direct = transition.clone().into_successor();
    let _committed = journal
        .commit_transition(transition)
        .expect("rollback publication commit");
    let transition = direct
        .advertise(rollback_advertised)
        .expect("rollback advertisement");
    direct = transition.clone().into_successor();
    let _committed = journal
        .commit_transition(transition)
        .expect("rollback advertisement commit");
    let transition = direct
        .rollback(rollback_active, rollback_fence)
        .expect("forward rollback activation");
    direct = transition.clone().into_successor();
    let _committed = journal
        .commit_transition(transition)
        .expect("rollback activation commit");

    let recovered = complete_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("all lifecycle deltas replay"),
    );
    assert_eq!(
        recovered.journal_head().map(JournalSequenceV1::get),
        Some(13)
    );
    assert_eq!(recovered.lifecycle().snapshot(), direct.snapshot());
}

#[test]
fn successor_fingerprint_tamper_degrades_at_the_last_matching_prefix() {
    let port = MemoryAuthorityStore::default();
    let (_, resolver) = committed_initial_lifecycle(&port);
    let first_entry = port.entries.lock().expect("entries")[0].clone();
    let genesis_authority = ResidentAuthorityState::new(genesis());
    let prefix =
        AuthorityJournalEntry::decode(&first_entry, &store(), limits().maximum_entry_bytes())
            .expect("first entry decodes")
            .fold(
                &genesis_authority,
                &resolver,
                &session_lease_recovery(&port),
            )
            .expect("first entry folds");
    let original = port.entries.lock().expect("entries")[1].clone();
    let mut value = pale_core::canon::parse_strict(original.bytes().as_slice())
        .expect("entry is canonical JSON");
    value["successor"] = serde_json::json!(std::iter::repeat_n('f', 64).collect::<String>());
    let bytes = pale_core::canon::to_canonical_string(&value)
        .expect("tampered entry canonicalizes")
        .into_bytes();
    port.entries.lock().expect("entries")[1] = SequencedAuthorityEntry::new(
        original.sequence(),
        BoundedAuthorityEntryBytes::new(bytes, limits().maximum_entry_bytes())
            .expect("tampered entry remains bounded"),
    );

    let outcome = recover_authority(&port, store(), limits(), genesis(), &resolver)
        .expect("semantic tamper produces degraded recovery");
    let (recovered, quarantine) = degraded_recovery(outcome);
    assert_eq!(
        recovered.lifecycle().snapshot(),
        prefix.lifecycle().snapshot()
    );
    assert_eq!(
        recovered.journal_head().map(JournalSequenceV1::get),
        Some(1)
    );
    assert!(matches!(
        quarantine.reason(),
        AuthorityRecoveryQuarantineReason::FoldContradiction
    ));
}

#[test]
fn sparse_entry_size_is_independent_of_unchanged_retained_state() {
    let small_generation = compiled_generation('a');
    let small_head = published(&small_generation, 1, None);
    let small = genesis()
        .publish(small_head)
        .expect("small publication")
        .into_successor();
    let small_transition = small
        .retain_for_rollback(small_generation.compiled_generation_digest().clone())
        .expect("small retention");

    let mut large = genesis();
    let mut predecessor = None;
    let mut last_generation = None;
    for (index, runtime_byte) in "0123456789abcdef".chars().enumerate() {
        let generation = compiled_generation(runtime_byte);
        let head = published(&generation, index as u64 + 1, predecessor.as_ref());
        large = large
            .publish(head.clone())
            .expect("large-state publication")
            .into_successor();
        predecessor = Some(head);
        last_generation = Some(generation);
    }
    let last_generation = last_generation.expect("large state has a generation");
    let large_transition = large
        .retain_for_rollback(last_generation.compiled_generation_digest().clone())
        .expect("large retention");
    let sequence = JournalSequenceV1::new(100).expect("comparison sequence");
    let small_authority = ResidentAuthorityState::from_recovery(
        small,
        None,
        crate::resident::event_custody::journal::EventCustodyFrontier::Empty,
    );
    let large_authority = ResidentAuthorityState::from_recovery(
        large,
        None,
        crate::resident::event_custody::journal::EventCustodyFrontier::Empty,
    );
    let (small_entry, _) = AuthorityJournalEntry::from_transition(
        &store(),
        sequence,
        &small_authority,
        ResidentAuthorityState::lifecycle_transition(small_transition),
    )
    .expect("small entry");
    let (large_entry, _) = AuthorityJournalEntry::from_transition(
        &store(),
        sequence,
        &large_authority,
        ResidentAuthorityState::lifecycle_transition(large_transition),
    )
    .expect("large entry");
    let small_bytes = small_entry
        .encode(limits().maximum_entry_bytes())
        .expect("small entry encodes");
    let large_bytes = large_entry
        .encode(limits().maximum_entry_bytes())
        .expect("large entry encodes");
    assert_eq!(small_bytes.as_slice().len(), large_bytes.as_slice().len());
    let text = std::str::from_utf8(large_bytes.as_slice()).expect("entry is UTF-8 JSON");
    assert!(!text.contains("retained_generations"));
    assert!(!text.contains("session_pins"));
    let value = pale_core::canon::parse_strict(large_bytes.as_slice()).expect("entry parses");
    let body = value.as_object().expect("authority journal envelope");
    assert!(body.contains_key("predecessor"));
    assert!(body.contains_key("delta"));
    assert!(body.contains_key("successor"));
}

#[test]
fn ambiguous_commit_requires_recovery_without_retry_or_query_and_guess() {
    let port = MemoryAuthorityStore::default();
    port.enqueue(Err(DurableAuthorityStoreErrorKind::AmbiguousCommit.into()));
    let generation = compiled_generation('4');
    let head = published(&generation, 1, None);
    let mut journal = empty_journal(&port);
    let transition = genesis().publish(head.clone()).expect("transition");
    assert!(matches!(
        journal.commit_transition(transition),
        Err(AuthorityJournalError::Store(error))
            if error.kind() == DurableAuthorityStoreErrorKind::AmbiguousCommit
    ));
    let second = genesis().publish(head).expect("second transition");
    assert!(matches!(
        journal.commit_transition(second),
        Err(AuthorityJournalError::WriterCommitOutcomeUnknown)
    ));
    assert_eq!(port.append_calls(), 1);
}

#[test]
fn competing_writer_conflict_requires_recovery_before_another_append() {
    let port = MemoryAuthorityStore::default();
    let generation = compiled_generation('4');
    let head = published(&generation, 1, None);
    let mut winner = empty_journal(&port);
    let mut stale = empty_journal(&port);
    let winning_transition = genesis().publish(head.clone()).expect("winning transition");
    let _ = winner
        .commit_transition(winning_transition)
        .expect("competing writer advances the durable head");
    let stale_transition = genesis().publish(head.clone()).expect("stale transition");
    assert!(matches!(
        stale.commit_transition(stale_transition),
        Err(AuthorityJournalError::Store(error))
            if error.kind() == DurableAuthorityStoreErrorKind::Conflict
    ));
    assert_eq!(port.append_calls(), 2);
    let stale_retry = genesis()
        .publish(head.clone())
        .expect("stale retry transition");
    assert!(matches!(
        stale.commit_transition(stale_retry),
        Err(AuthorityJournalError::WriterStale)
    ));
    assert_eq!(port.append_calls(), 2);
    drop(stale);
    drop(winner);

    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(&generation);
    resolver.insert_published(&head);
    let recovered = complete_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("explicit recovery refreshes the competing frontier"),
    );
    let mut resumed = DurableAuthorityJournal::resume(&port, limits(), recovered);
    let transition = resumed
        .lifecycle()
        .retain_for_rollback(generation.compiled_generation_digest().clone())
        .expect("recovered generation is retained");
    let committed = resumed
        .commit_transition(transition)
        .expect("recovered writer advances normally");
    assert_eq!(committed.into_parts().0.get(), 2);
    assert_eq!(port.append_calls(), 3);
}

#[test]
fn writer_rejects_stale_semantic_state_before_durable_append() {
    let port = MemoryAuthorityStore::default();
    let first_generation = compiled_generation('4');
    let first_head = published(&first_generation, 1, None);
    let mut journal = empty_journal(&port);
    let _ = journal
        .commit_transition(genesis().publish(first_head).expect("first transition"))
        .expect("first commit");
    let second_generation = compiled_generation('5');
    let stale = genesis()
        .publish(published(&second_generation, 2, None))
        .expect("stale transition");
    assert!(matches!(
        journal.commit_transition(stale),
        Err(AuthorityJournalError::FoldContradiction)
    ));
    journal
        .store_checkpoint()
        .expect("current state checkpoints");
    assert_eq!(port.append_calls(), 1);
}

#[test]
fn checkpoint_plus_tail_rebuilds_exact_state() {
    let port = MemoryAuthorityStore::default();
    let (expected, resolver) = committed_initial_lifecycle(&port);
    let first_entry = port.entries.lock().expect("entries")[0].clone();
    let first =
        AuthorityJournalEntry::decode(&first_entry, &store(), limits().maximum_entry_bytes())
            .expect("first decodes")
            .fold(
                &ResidentAuthorityState::new(genesis()),
                &resolver,
                &session_lease_recovery(&port),
            )
            .expect("first folds");
    let checkpoint = encode_authority_recovery_checkpoint(
        &store(),
        JournalSequenceV1::new(1).expect("checkpoint sequence"),
        &first,
        limits().maximum_checkpoint_bytes(),
    )
    .expect("checkpoint encodes");
    port.checkpoints
        .lock()
        .expect("checkpoints")
        .push(StoredRecoveryCheckpoint::new(
            JournalSequenceV1::new(1).expect("checkpoint sequence"),
            checkpoint,
        ));
    let recovered = complete_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("checkpoint recovery"),
    );
    assert_eq!(recovered.lifecycle().snapshot(), expected.snapshot());
}

#[test]
fn physically_rejected_newest_checkpoint_falls_back_to_older_checkpoint_and_tail() {
    let port = MemoryAuthorityStore::default();
    let (expected, resolver) = committed_initial_lifecycle(&port);
    let first_entry = port.entries.lock().expect("entries")[0].clone();
    let first =
        AuthorityJournalEntry::decode(&first_entry, &store(), limits().maximum_entry_bytes())
            .expect("first decodes")
            .fold(
                &ResidentAuthorityState::new(genesis()),
                &resolver,
                &session_lease_recovery(&port),
            )
            .expect("first folds");
    let checkpoint = encode_authority_recovery_checkpoint(
        &store(),
        JournalSequenceV1::new(1).expect("checkpoint sequence"),
        &first,
        limits().maximum_checkpoint_bytes(),
    )
    .expect("checkpoint encodes");
    port.reject_checkpoint_at(3, RecoveryCheckpointPhysicalRejection::Integrity);
    port.checkpoints
        .lock()
        .expect("checkpoints")
        .push(StoredRecoveryCheckpoint::new(
            JournalSequenceV1::new(1).expect("checkpoint sequence"),
            checkpoint,
        ));

    let recovered = complete_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("older checkpoint plus exact tail recovers"),
    );
    assert_eq!(recovered.lifecycle().snapshot(), expected.snapshot());
}

#[test]
fn a_semantically_valid_checkpoint_cannot_replace_the_committed_journal_state() {
    let port = MemoryAuthorityStore::default();
    let (expected, resolver) = committed_initial_lifecycle(&port);
    let empty = genesis();
    let checkpoint = encode_recovery_checkpoint(
        &store(),
        JournalSequenceV1::new(3).expect("current durable head"),
        &empty,
        limits().maximum_checkpoint_bytes(),
    )
    .expect("empty lifecycle is a valid canonical checkpoint image");
    assert!(
        crate::resident::lifecycle::GenerationLifecycleRecoveryStateV1::from_lifecycle(&empty)
            .admit(&resolver, &session_lease_recovery(&port))
            .is_ok()
    );
    port.checkpoints
        .lock()
        .expect("checkpoints")
        .push(StoredRecoveryCheckpoint::new(
            JournalSequenceV1::new(3).expect("current durable head"),
            checkpoint,
        ));
    port.tail_reads.lock().expect("tail reads").clear();

    let recovered = complete_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("an unrelated valid cache cannot suppress authoritative replay"),
    );
    assert_eq!(
        recovered.journal_head().map(JournalSequenceV1::get),
        Some(3)
    );
    assert_eq!(recovered.lifecycle().snapshot(), expected.snapshot());
    assert_eq!(
        *port.tail_reads.lock().expect("tail reads"),
        vec![JournalSequenceV1::new(2).ok(), None],
        "a mismatched candidate falls back to one independent genesis observation",
    );
}

#[test]
fn mismatched_newest_checkpoint_falls_back_once_to_an_older_journal_bound_state() {
    let port = MemoryAuthorityStore::default();
    let (expected, resolver) = committed_initial_lifecycle(&port);
    let first = AuthorityJournalEntry::decode(
        &port.entries.lock().expect("entries")[0],
        &store(),
        limits().maximum_entry_bytes(),
    )
    .expect("first decodes")
    .fold(
        &ResidentAuthorityState::new(genesis()),
        &resolver,
        &session_lease_recovery(&port),
    )
    .expect("first folds");
    cache_lifecycle(&port, 3, &genesis());
    cache_lifecycle(&port, 1, first.lifecycle());
    port.tail_reads.lock().expect("tail reads").clear();

    let recovered = complete_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("older corresponding cache and its observed suffix recover"),
    );
    assert_eq!(recovered.lifecycle().snapshot(), expected.snapshot());
    assert_eq!(
        *port.tail_reads.lock().expect("tail reads"),
        vec![JournalSequenceV1::new(2).ok(), None],
        "reuse the older candidate's observation without fetching its suffix again",
    );
}

#[test]
fn checkpoint_at_head_uses_exactly_one_binding_entry_and_one_observation() {
    let port = MemoryAuthorityStore::default();
    let (expected, resolver) = committed_initial_lifecycle(&port);
    cache_lifecycle(&port, 3, &expected);
    port.tail_reads.lock().expect("tail reads").clear();
    let bounded =
        AuthorityRecoveryReadLimits::new(1_000_000, 1_000_000, 8, 1).expect("one-entry budget");
    let recovered = complete_recovery(
        recover_authority(&port, store(), bounded, genesis(), &resolver)
            .expect("binding entry alone proves the current checkpoint frontier"),
    );
    assert_eq!(recovered.lifecycle().snapshot(), expected.snapshot());
    assert_eq!(
        *port.tail_reads.lock().expect("tail reads"),
        vec![JournalSequenceV1::new(2).ok()]
    );
}

#[test]
fn binding_entry_counts_toward_the_exact_tail_budget_from_sequence_one() {
    let port = MemoryAuthorityStore::default();
    let (expected, resolver) = committed_initial_lifecycle(&port);
    let first = AuthorityJournalEntry::decode(
        &port.entries.lock().expect("entries")[0],
        &store(),
        limits().maximum_entry_bytes(),
    )
    .expect("first decodes")
    .fold(
        &ResidentAuthorityState::new(genesis()),
        &resolver,
        &session_lease_recovery(&port),
    )
    .expect("first folds");
    cache_lifecycle(&port, 1, first.lifecycle());
    port.tail_reads.lock().expect("tail reads").clear();
    let exact = AuthorityRecoveryReadLimits::new(1_000_000, 1_000_000, 8, 3)
        .expect("exact binding plus tail budget");
    let recovered = complete_recovery(
        recover_authority(&port, store(), exact, genesis(), &resolver).expect("exact bound"),
    );
    assert_eq!(recovered.lifecycle().snapshot(), expected.snapshot());
    assert_eq!(*port.tail_reads.lock().expect("tail reads"), vec![None]);
    let short = AuthorityRecoveryReadLimits::new(1_000_000, 1_000_000, 8, 2).expect("short budget");
    assert!(matches!(
        recover_authority(&port, store(), short, genesis(), &resolver),
        Err(AuthorityJournalError::Store(error)) if error.kind()
            == DurableAuthorityStoreErrorKind::BoundExceeded {
                class: pale_ports::AuthorityByteClass::JournalEntry,
            }
    ));
}

#[test]
fn physically_rejected_binding_entry_cannot_be_bypassed_by_its_checkpoint() {
    let port = MemoryAuthorityStore::default();
    let (expected, resolver) = committed_initial_lifecycle(&port);
    cache_lifecycle(&port, 3, &expected);
    port.quarantine_from(3);
    let (prefix, quarantine) = degraded_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("recover the prefix before the rejected binding"),
    );
    assert_eq!(prefix.journal_head().map(JournalSequenceV1::get), Some(2));
    assert!(prefix.lifecycle().snapshot().active.is_none());
    assert_eq!(quarantine.first_untrusted_sequence().get(), 3);
    assert!(matches!(
        quarantine.reason(),
        AuthorityRecoveryQuarantineReason::PhysicalIntegrity
    ));
}

#[test]
fn missing_binding_entry_is_a_physical_page_failure_not_a_cache_rejection() {
    let port = MemoryAuthorityStore::default();
    let (_, resolver) = committed_initial_lifecycle(&port);
    cache_lifecycle(&port, 1, &genesis());
    port.entries.lock().expect("entries").remove(0);
    assert!(matches!(
        recover_authority(&port, store(), limits(), genesis(), &resolver),
        Err(AuthorityJournalError::Store(error))
            if error.kind() == DurableAuthorityStoreErrorKind::Corrupt(
                pale_ports::AuthorityByteClass::JournalEntry
            )
    ));
}

#[test]
fn malformed_binding_entry_preserves_typed_quarantine_cause() {
    let port = MemoryAuthorityStore::default();
    let (expected, resolver) = committed_initial_lifecycle(&port);
    cache_lifecycle(&port, 3, &expected);
    port.entries.lock().expect("entries")[2] = SequencedAuthorityEntry::new(
        JournalSequenceV1::new(3).expect("head"),
        BoundedAuthorityEntryBytes::new(b"{}".to_vec(), limits().maximum_entry_bytes())
            .expect("bounded malformed entry"),
    );
    let (prefix, quarantine) = degraded_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("malformed binding requires earlier recovery"),
    );
    assert_eq!(prefix.journal_head().map(JournalSequenceV1::get), Some(2));
    assert!(quarantine
        .reason()
        .source()
        .expect("typed decode cause")
        .is::<serde_json::Error>());
}

#[test]
fn matching_checkpoint_fingerprint_does_not_excuse_a_wrong_delta_sequence() {
    let port = MemoryAuthorityStore::default();
    let (expected, resolver) = committed_initial_lifecycle(&port);
    cache_lifecycle(&port, 3, &expected);
    let original = port.entries.lock().expect("entries")[2].clone();
    let mut value =
        pale_core::canon::parse_strict(original.bytes().as_slice()).expect("entry parses");
    let sequence = &mut value["delta"]["body"]["delta"]["body"]["head"]["journal_sequence"];
    assert_eq!(*sequence, serde_json::json!(3));
    *sequence = serde_json::json!(2);
    port.entries.lock().expect("entries")[2] = SequencedAuthorityEntry::new(
        original.sequence(),
        BoundedAuthorityEntryBytes::new(
            pale_core::canon::to_canonical_string(&value)
                .expect("canonical tamper")
                .into_bytes(),
            limits().maximum_entry_bytes(),
        )
        .expect("bounded tamper"),
    );
    let (prefix, quarantine) = degraded_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("invalid delta cannot be hidden by a matching successor"),
    );
    assert_eq!(prefix.journal_head().map(JournalSequenceV1::get), Some(2));
    assert!(quarantine
        .reason()
        .source()
        .expect("lifecycle cause")
        .is::<GenerationLifecycleError>());
}

#[test]
fn checkpoint_fallback_cannot_regress_an_observed_durable_head() {
    let port = MemoryAuthorityStore::default();
    let (_, resolver) = committed_initial_lifecycle(&port);
    cache_lifecycle(&port, 3, &genesis());
    port.head_results.lock().expect("head results").extend([
        Ok(JournalSequenceV1::new(3).ok()),
        Ok(JournalSequenceV1::new(2).ok()),
    ]);
    assert!(matches!(
        recover_authority(&port, store(), limits(), genesis(), &resolver),
        Err(AuthorityJournalError::DurableHeadContradiction {
            required: 3,
            observed: Some(2)
        })
    ));
}

#[test]
fn checkpoint_journal_read_failure_preserves_cause_without_fallback() {
    let port = MemoryAuthorityStore::default();
    let (_, resolver) = committed_initial_lifecycle(&port);
    cache_lifecycle(&port, 3, &genesis());
    port.tail_reads.lock().expect("tail reads").clear();
    port.page_errors.lock().expect("page errors").push_back(
        DurableAuthorityStoreError::with_source(
            DurableAuthorityStoreErrorKind::Unavailable,
            std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        ),
    );
    let result = recover_authority(&port, store(), limits(), genesis(), &resolver);
    let Err(AuthorityJournalError::Store(error)) = result else {
        panic!("custody failure must not be retried as cache rejection");
    };
    assert_eq!(error.kind(), DurableAuthorityStoreErrorKind::Unavailable);
    assert_eq!(
        error
            .source()
            .expect("source")
            .downcast_ref::<std::io::Error>()
            .expect("native source")
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        *port.tail_reads.lock().expect("tail reads"),
        vec![JournalSequenceV1::new(2).ok()]
    );
}

#[test]
fn mixed_current_head_checkpoints_are_skipped_before_full_journal_rebuild() {
    let port = MemoryAuthorityStore::default();
    let (state, resolver) = committed_initial_lifecycle(&port);
    let valid = encode_recovery_checkpoint(
        &store(),
        JournalSequenceV1::new(3).expect("sequence"),
        &state,
        limits().maximum_checkpoint_bytes(),
    )
    .expect("valid checkpoint");
    for absent_member in ["published", "advertised", "fence"] {
        let mut invalid =
            pale_core::canon::parse_strict(valid.as_slice()).expect("checkpoint parses");
        invalid["authority"]["lifecycle"][absent_member] = serde_json::Value::Null;
        let invalid =
            pale_core::canon::to_canonical_string(&invalid).expect("canonical contradiction");
        *port.checkpoints.lock().expect("checkpoints") = vec![StoredRecoveryCheckpoint::new(
            JournalSequenceV1::new(3).expect("sequence"),
            BoundedRecoveryCheckpointBytes::new(
                invalid.into_bytes(),
                limits().maximum_checkpoint_bytes(),
            )
            .expect("bounded contradiction"),
        )];
        let recovered = complete_recovery(
            recover_authority(&port, store(), limits(), genesis(), &resolver)
                .expect("invalid projection cannot suppress authoritative full replay"),
        );
        assert_eq!(recovered.lifecycle().snapshot(), state.snapshot());
    }
}

#[test]
fn coordinator_stores_a_checkpoint_without_advancing_authority() {
    let port = MemoryAuthorityStore::default();
    let generation = compiled_generation('4');
    let head = published(&generation, 1, None);
    let mut journal = empty_journal(&port);
    assert!(matches!(
        journal.store_checkpoint(),
        Err(AuthorityJournalError::MissingJournalHead)
    ));
    let _ = journal
        .commit_transition(genesis().publish(head).expect("transition"))
        .expect("commit");
    journal.store_checkpoint().expect("checkpoint stored");
    assert_eq!(journal.head().map(JournalSequenceV1::get), Some(1));
    assert_eq!(port.entries.lock().expect("entries").len(), 1);
    assert_eq!(port.checkpoints.lock().expect("checkpoints").len(), 1);
}

#[test]
fn ambiguous_checkpoint_is_not_effect_uncertainty_and_does_not_close_writer() {
    let port = MemoryAuthorityStore::default();
    let generation = compiled_generation('4');
    let mut journal = empty_journal(&port);
    let _ = journal
        .commit_transition(
            genesis()
                .publish(published(&generation, 1, None))
                .expect("publish transition"),
        )
        .expect("publish commit");
    port.enqueue_checkpoint(Err(DurableAuthorityStoreError::with_source(
        DurableAuthorityStoreErrorKind::AmbiguousCommit,
        std::io::Error::other("checkpoint finalization failed"),
    )));

    let Err(AuthorityJournalError::CheckpointProjectionOutcomeUnknown(error)) =
        journal.store_checkpoint()
    else {
        panic!("checkpoint outcome must remain separately unknown");
    };
    assert!(error
        .source()
        .expect("native checkpoint cause")
        .is::<std::io::Error>());
    let retained = journal
        .lifecycle()
        .retain_for_rollback(generation.compiled_generation_digest().clone())
        .expect("retain transition");
    assert_eq!(
        journal
            .commit_transition(retained)
            .expect("checkpoint ambiguity leaves journal writer ready")
            .into_parts()
            .0
            .get(),
        2
    );
}

#[test]
fn checkpoint_head_conflict_closes_the_stale_writer() {
    let port = MemoryAuthorityStore::default();
    let generation = compiled_generation('4');
    let mut journal = empty_journal(&port);
    let _ = journal
        .commit_transition(
            genesis()
                .publish(published(&generation, 1, None))
                .expect("publish transition"),
        )
        .expect("publish commit");
    port.enqueue_checkpoint(Err(DurableAuthorityStoreErrorKind::Conflict.into()));

    assert!(matches!(
        journal.store_checkpoint(),
        Err(AuthorityJournalError::Store(error))
            if error.kind() == DurableAuthorityStoreErrorKind::Conflict
    ));
    let retained = journal
        .lifecycle()
        .retain_for_rollback(generation.compiled_generation_digest().clone())
        .expect("retain transition");
    assert!(matches!(
        journal.commit_transition(retained),
        Err(AuthorityJournalError::WriterStale)
    ));
}

#[test]
fn physical_gap_fails_while_noncanonical_tail_quarantines_at_the_first_bad_entry() {
    let gap_port = MemoryAuthorityStore::default();
    let (_, resolver) = committed_initial_lifecycle(&gap_port);
    gap_port.entries.lock().expect("entries").remove(1);
    assert!(matches!(
        recover_authority(&gap_port, store(), limits(), genesis(), &resolver),
        Err(AuthorityJournalError::Store(error))
            if error.kind() == DurableAuthorityStoreErrorKind::Corrupt(
                pale_ports::AuthorityByteClass::JournalEntry
            )
    ));

    let corrupt_port = MemoryAuthorityStore::default();
    let (_, resolver) = committed_initial_lifecycle(&corrupt_port);
    let original = corrupt_port.entries.lock().expect("entries")[0].clone();
    let mut bytes = original.bytes().as_slice().to_vec();
    bytes.push(b' ');
    corrupt_port.entries.lock().expect("entries")[0] = SequencedAuthorityEntry::new(
        original.sequence(),
        BoundedAuthorityEntryBytes::new(bytes, limits().maximum_entry_bytes())
            .expect("bounded corrupt bytes"),
    );
    let outcome = recover_authority(&corrupt_port, store(), limits(), genesis(), &resolver)
        .expect("noncanonical application bytes quarantine");
    let (_, quarantine) = degraded_recovery(outcome);
    assert!(quarantine.valid_through().is_none());
    assert_eq!(quarantine.first_untrusted_sequence().get(), 1);
    assert!(matches!(
        quarantine.reason(),
        AuthorityRecoveryQuarantineReason::NonCanonical
    ));
}

#[test]
fn small_physical_pages_continue_to_the_fixed_recovery_head() {
    let port = MemoryAuthorityStore::default();
    let (expected, resolver) = committed_initial_lifecycle(&port);
    port.truncate_tail_to(1);
    let recovered = complete_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("bounded pages recover through the fixed head"),
    );
    assert_eq!(recovered.lifecycle().snapshot(), expected.snapshot());
    assert_eq!(
        *port.tail_reads.lock().expect("tail reads"),
        vec![
            None,
            JournalSequenceV1::new(1).ok(),
            JournalSequenceV1::new(2).ok()
        ]
    );
}

#[test]
fn physical_suffix_quarantine_retains_the_exact_read_only_prefix() {
    let port = MemoryAuthorityStore::default();
    let (complete, resolver) = committed_initial_lifecycle(&port);
    port.quarantine_from(3);

    let outcome = recover_authority(&port, store(), limits(), genesis(), &resolver)
        .expect("physical quarantine is an observed recovery outcome");
    let (prefix, quarantine) = degraded_recovery(outcome);
    assert_eq!(quarantine.store(), &store());
    assert_eq!(
        quarantine.valid_through().map(JournalSequenceV1::get),
        Some(2)
    );
    assert_eq!(quarantine.first_untrusted_sequence().get(), 3);
    assert_eq!(quarantine.durable_head().get(), 3);
    assert!(matches!(
        quarantine.reason(),
        AuthorityRecoveryQuarantineReason::PhysicalIntegrity
    ));
    assert_eq!(prefix.journal_head().map(JournalSequenceV1::get), Some(2));
    assert_eq!(
        prefix.lifecycle().snapshot().published,
        complete.snapshot().published
    );
    assert_eq!(
        prefix.lifecycle().snapshot().advertised,
        complete.snapshot().advertised
    );
    assert!(prefix.lifecycle().snapshot().active.is_none());
}

#[test]
fn initial_physical_quarantine_retains_read_only_genesis_prefix() {
    let port = MemoryAuthorityStore::default();
    let (_, resolver) = committed_initial_lifecycle(&port);
    port.quarantine_from(1);

    let outcome = recover_authority(&port, store(), limits(), genesis(), &resolver)
        .expect("initial physical quarantine is represented");
    let (prefix, quarantine) = degraded_recovery(outcome);
    assert_eq!(quarantine.valid_through(), None);
    assert_eq!(quarantine.first_untrusted_sequence().get(), 1);
    assert_eq!(quarantine.durable_head().get(), 3);
    assert_eq!(prefix.journal_head(), None);
    assert_eq!(prefix.lifecycle().snapshot(), genesis().snapshot());
}

#[test]
fn application_quarantine_stops_before_later_entries_are_decoded() {
    let port = MemoryAuthorityStore::default();
    let (_, resolver) = committed_initial_lifecycle(&port);
    let mut entries = port.entries.lock().expect("entries");
    for index in [1, 2] {
        let original = &entries[index];
        let mut bytes = original.bytes().as_slice().to_vec();
        bytes.push(b' ');
        entries[index] = SequencedAuthorityEntry::new(
            original.sequence(),
            BoundedAuthorityEntryBytes::new(bytes, limits().maximum_entry_bytes())
                .expect("bounded noncanonical bytes"),
        );
    }
    drop(entries);

    let outcome = recover_authority(&port, store(), limits(), genesis(), &resolver)
        .expect("application-invalid entry quarantines");
    let (_, quarantine) = degraded_recovery(outcome);
    assert_eq!(
        quarantine.valid_through().map(JournalSequenceV1::get),
        Some(1)
    );
    assert_eq!(quarantine.first_untrusted_sequence().get(), 2);
    assert_eq!(quarantine.durable_head().get(), 3);
    assert!(matches!(
        quarantine.reason(),
        AuthorityRecoveryQuarantineReason::NonCanonical
    ));
}

#[test]
fn canonical_grammar_and_typed_decode_failures_are_distinct_quarantines() {
    for (bytes, typed_decode) in [(b"{".as_slice(), false), (b"{}".as_slice(), true)] {
        let port = MemoryAuthorityStore::default();
        let (_, resolver) = committed_initial_lifecycle(&port);
        let sequence = port.entries.lock().expect("entries")[1].sequence();
        port.entries.lock().expect("entries")[1] = SequencedAuthorityEntry::new(
            sequence,
            BoundedAuthorityEntryBytes::new(bytes.to_vec(), limits().maximum_entry_bytes())
                .expect("bounded invalid application bytes"),
        );

        let outcome = recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("application-invalid entry quarantines");
        let (_, quarantine) = degraded_recovery(outcome);
        assert_eq!(
            quarantine.valid_through().map(JournalSequenceV1::get),
            Some(1)
        );
        assert_eq!(quarantine.first_untrusted_sequence().get(), 2);
        match (quarantine.reason(), typed_decode) {
            (AuthorityRecoveryQuarantineReason::CanonicalGrammar(error), false) => {
                assert!(!error.to_string().is_empty());
                assert!(quarantine
                    .reason()
                    .source()
                    .expect("canonical cause")
                    .is::<pale_core::canon::CanonError>());
            }
            (AuthorityRecoveryQuarantineReason::TypedDecode(error), true) => {
                assert!(!error.to_string().is_empty());
                assert!(quarantine
                    .reason()
                    .source()
                    .expect("decode cause")
                    .is::<serde_json::Error>());
            }
            (reason, _) => panic!("unexpected quarantine reason: {reason}"),
        }
    }
}

#[test]
fn a_quarantine_boundary_remains_exact_across_small_physical_pages() {
    let boundary_port = MemoryAuthorityStore::default();
    let (_, resolver) = committed_initial_lifecycle(&boundary_port);
    boundary_port.quarantine_from(3);
    boundary_port.truncate_tail_to(1);
    let (prefix, quarantine) = degraded_recovery(
        recover_authority(&boundary_port, store(), limits(), genesis(), &resolver)
            .expect("paged quarantine retains its exact prefix"),
    );
    assert_eq!(prefix.journal_head().map(JournalSequenceV1::get), Some(2));
    assert_eq!(quarantine.first_untrusted_sequence().get(), 3);
    assert_eq!(
        *boundary_port.tail_reads.lock().expect("tail reads"),
        vec![None, JournalSequenceV1::new(1).ok()]
    );
}

#[test]
fn checkpoint_ahead_of_the_co_observed_durable_head_is_a_fatal_contradiction() {
    let port = MemoryAuthorityStore::default();
    let (state, resolver) = committed_initial_lifecycle(&port);
    let checkpoint = encode_recovery_checkpoint(
        &store(),
        JournalSequenceV1::new(3).expect("checkpoint sequence"),
        &state,
        limits().maximum_checkpoint_bytes(),
    )
    .expect("checkpoint encodes");
    port.checkpoints
        .lock()
        .expect("checkpoints")
        .push(StoredRecoveryCheckpoint::new(
            JournalSequenceV1::new(3).expect("checkpoint sequence"),
            checkpoint,
        ));
    port.entries.lock().expect("entries").pop();

    assert!(matches!(
        recover_authority(&port, store(), limits(), genesis(), &resolver),
        Err(AuthorityJournalError::DurableHeadContradiction {
            required: 3,
            observed: Some(2)
        })
    ));

    port.entries.lock().expect("entries").clear();
    assert!(matches!(
        recover_authority(&port, store(), limits(), genesis(), &resolver),
        Err(AuthorityJournalError::DurableHeadContradiction {
            required: 3,
            observed: None
        })
    ));
}

#[test]
fn recovery_tail_count_bounds_remain_fatal_not_quarantined() {
    let port = MemoryAuthorityStore::default();
    let (_, resolver) = committed_initial_lifecycle(&port);
    let bounded = AuthorityRecoveryReadLimits::new(1_000_000, 1_000_000, 8, 2)
        .expect("bounded recovery limits");
    assert!(matches!(
        recover_authority(&port, store(), bounded, genesis(), &resolver),
        Err(AuthorityJournalError::Store(error))
            if error.kind() == DurableAuthorityStoreErrorKind::BoundExceeded {
                class: pale_ports::AuthorityByteClass::JournalEntry
            }
    ));
}

#[test]
fn replay_quarantines_a_transition_built_against_another_semantic_state() {
    let port = MemoryAuthorityStore::default();
    let first_generation = compiled_generation('4');
    let first_head = published(&first_generation, 1, None);
    let mut journal = empty_journal(&port);
    let first_state = journal
        .commit_transition(
            genesis()
                .publish(first_head.clone())
                .expect("first transition"),
        )
        .expect("first commit")
        .into_parts()
        .1
        .lifecycle()
        .clone();
    assert_eq!(first_state.snapshot().retained_generations.len(), 1);

    let second_generation = compiled_generation('5');
    let second_head = published(&second_generation, 2, None);
    let contradictory = genesis()
        .publish(second_head.clone())
        .expect("independent transition");
    let contradictory_current = ResidentAuthorityState::new(genesis());
    let (entry, _) = AuthorityJournalEntry::from_transition(
        &store(),
        JournalSequenceV1::new(2).expect("sequence"),
        &contradictory_current,
        ResidentAuthorityState::lifecycle_transition(contradictory),
    )
    .expect("entry builds");
    port.entries
        .lock()
        .expect("entries")
        .push(SequencedAuthorityEntry::new(
            JournalSequenceV1::new(2).expect("sequence"),
            entry
                .encode(limits().maximum_entry_bytes())
                .expect("entry encodes"),
        ));
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(&first_generation);
    resolver.insert_generation(&second_generation);
    resolver.insert_published(&first_head);
    resolver.insert_published(&second_head);
    let outcome = recover_authority(&port, store(), limits(), genesis(), &resolver)
        .expect("fold contradiction quarantines");
    let (_, quarantine) = degraded_recovery(outcome);
    assert_eq!(
        quarantine.valid_through().map(JournalSequenceV1::get),
        Some(1)
    );
    assert_eq!(quarantine.first_untrusted_sequence().get(), 2);
    assert!(matches!(
        quarantine.reason(),
        AuthorityRecoveryQuarantineReason::FoldContradiction
    ));
}

#[test]
fn invalid_newest_checkpoint_falls_back_to_the_greatest_valid_candidate() {
    let port = MemoryAuthorityStore::default();
    let (expected, resolver) = committed_initial_lifecycle(&port);
    let first_entry = port.entries.lock().expect("entries")[0].clone();
    let first =
        AuthorityJournalEntry::decode(&first_entry, &store(), limits().maximum_entry_bytes())
            .expect("first decodes")
            .fold(
                &ResidentAuthorityState::new(genesis()),
                &resolver,
                &session_lease_recovery(&port),
            )
            .expect("first folds");
    let valid = encode_authority_recovery_checkpoint(
        &store(),
        JournalSequenceV1::new(1).expect("sequence"),
        &first,
        limits().maximum_checkpoint_bytes(),
    )
    .expect("valid checkpoint");
    *port.checkpoints.lock().expect("checkpoints") = vec![
        StoredRecoveryCheckpoint::new(
            JournalSequenceV1::new(2).expect("sequence"),
            BoundedRecoveryCheckpointBytes::new(
                br#"{"not":"a checkpoint"}"#.to_vec(),
                limits().maximum_checkpoint_bytes(),
            )
            .expect("bounded invalid checkpoint"),
        ),
        StoredRecoveryCheckpoint::new(JournalSequenceV1::new(1).expect("sequence"), valid),
    ];
    let recovered = complete_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver).expect("falls back"),
    );
    assert_eq!(recovered.lifecycle().snapshot(), expected.snapshot());
}

#[test]
fn checkpoint_order_is_validated_before_any_candidate_is_selected() {
    let port = MemoryAuthorityStore::default();
    let (_, resolver) = committed_initial_lifecycle(&port);
    let first_entry = port.entries.lock().expect("entries")[0].clone();
    let first =
        AuthorityJournalEntry::decode(&first_entry, &store(), limits().maximum_entry_bytes())
            .expect("first decodes")
            .fold(
                &ResidentAuthorityState::new(genesis()),
                &resolver,
                &session_lease_recovery(&port),
            )
            .expect("first folds");
    let valid = encode_authority_recovery_checkpoint(
        &store(),
        JournalSequenceV1::new(1).expect("sequence"),
        &first,
        limits().maximum_checkpoint_bytes(),
    )
    .expect("valid checkpoint");
    *port.checkpoints.lock().expect("checkpoints") = vec![
        StoredRecoveryCheckpoint::new(JournalSequenceV1::new(1).expect("sequence"), valid),
        StoredRecoveryCheckpoint::new(
            JournalSequenceV1::new(2).expect("sequence"),
            BoundedRecoveryCheckpointBytes::new(
                br#"{"not":"a checkpoint"}"#.to_vec(),
                limits().maximum_checkpoint_bytes(),
            )
            .expect("bounded invalid checkpoint"),
        ),
    ];
    assert!(matches!(
        recover_authority(&port, store(), limits(), genesis(), &resolver),
        Err(AuthorityJournalError::CheckpointOrder)
    ));
}

#[test]
fn semantically_invalid_newest_checkpoint_falls_back_but_missing_custody_does_not() {
    let port = MemoryAuthorityStore::default();
    let (expected, resolver) = committed_initial_lifecycle(&port);
    let first_entry = port.entries.lock().expect("entries")[0].clone();
    let first =
        AuthorityJournalEntry::decode(&first_entry, &store(), limits().maximum_entry_bytes())
            .expect("first decodes")
            .fold(
                &ResidentAuthorityState::new(genesis()),
                &resolver,
                &session_lease_recovery(&port),
            )
            .expect("first folds");
    let valid = encode_authority_recovery_checkpoint(
        &store(),
        JournalSequenceV1::new(1).expect("sequence"),
        &first,
        limits().maximum_checkpoint_bytes(),
    )
    .expect("valid checkpoint");
    let mut invalid: serde_json::Value =
        pale_core::canon::parse_strict(valid.as_slice()).expect("checkpoint parses");
    invalid["through_sequence"] = serde_json::json!(2);
    invalid["authority"]["lifecycle"]["retained_generations"] = serde_json::json!([]);
    let invalid = pale_core::canon::to_canonical_string(&invalid).expect("canonical invalid state");
    *port.checkpoints.lock().expect("checkpoints") = vec![
        StoredRecoveryCheckpoint::new(
            JournalSequenceV1::new(2).expect("sequence"),
            BoundedRecoveryCheckpointBytes::new(
                invalid.into_bytes(),
                limits().maximum_checkpoint_bytes(),
            )
            .expect("bounded semantic contradiction"),
        ),
        StoredRecoveryCheckpoint::new(JournalSequenceV1::new(1).expect("sequence"), valid),
    ];

    let recovered = complete_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("older fully valid checkpoint is selected"),
    );
    assert_eq!(recovered.lifecycle().snapshot(), expected.snapshot());

    assert!(matches!(
        recover_authority(
            &port,
            store(),
            limits(),
            genesis(),
            &RecoveryResolver::default(),
        ),
        Err(AuthorityJournalError::Lifecycle(
            GenerationLifecycleError::MissingRecoveryContext(_)
        ))
    ));
}

#[test]
fn missing_owner_context_fails_closed() {
    let port = MemoryAuthorityStore::default();
    let _ = committed_initial_lifecycle(&port);
    assert!(matches!(
        recover_authority(
            &port,
            store(),
            limits(),
            genesis(),
            &RecoveryResolver::default(),
        ),
        Err(AuthorityJournalError::Lifecycle(
            GenerationLifecycleError::MissingRecoveryContext(_)
        ))
    ));
}

#[test]
fn checkpoint_requires_bodies_for_every_retained_generation() {
    let port = MemoryAuthorityStore::default();
    let first_generation = compiled_generation('4');
    let first_head = published(&first_generation, 1, None);
    let second_generation = compiled_generation('5');
    let second_head = published(&second_generation, 2, Some(&first_head));
    let first_state = genesis()
        .publish(first_head.clone())
        .expect("first transition")
        .into_successor();
    let second_state = first_state
        .publish(second_head.clone())
        .expect("second transition")
        .into_successor();
    let checkpoint = encode_recovery_checkpoint(
        &store(),
        JournalSequenceV1::new(2).expect("sequence"),
        &second_state,
        limits().maximum_checkpoint_bytes(),
    )
    .expect("checkpoint");
    port.checkpoints
        .lock()
        .expect("checkpoints")
        .push(StoredRecoveryCheckpoint::new(
            JournalSequenceV1::new(2).expect("sequence"),
            checkpoint,
        ));
    let mut incomplete = RecoveryResolver::default();
    incomplete.insert_generation(&second_generation);
    incomplete.insert_published(&first_head);
    incomplete.insert_published(&second_head);
    assert!(matches!(
        recover_authority(&port, store(), limits(), genesis(), &incomplete),
        Err(AuthorityJournalError::Lifecycle(
            GenerationLifecycleError::MissingRecoveryContext("retained compiled generation body")
        ))
    ));
}

#[test]
fn retained_session_body_re_admission_rejects_every_hostile_custody_axis() {
    let generation = compiled_generation('4');
    let published = published(&generation, 1, None);
    let advertised = advertised(&published, 2, None);
    let fence = fence(&generation, 1, None);
    let active = active(&advertised, &fence, 3, None);
    let profile = effective_profile();
    let lease = session_lease_for_profile(
        &generation,
        &active,
        &fence,
        &profile,
        "session-hostile-custody",
        10,
    );
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(&generation);
    resolver.insert_effective_profile(&profile);
    resolver.insert_published(&published);
    resolver.insert_advertised(&advertised);
    resolver.insert_fence(&fence);
    resolver.insert_active(&active);
    let canonical =
        pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(&lease))
            .expect("admitted lease has canonical wire bytes");
    let exact_body = SessionLeaseBody::new(
        store(),
        lease.session_admission_lease_digest().clone(),
        lease.schema_id().clone(),
        lease.schema_sha256().clone(),
        pale_ports::ControlRecordBytes::new(canonical.as_bytes().to_vec())
            .expect("canonical wire is nonempty"),
    );
    let recover =
        |body: SessionLeaseBody, maximum_body_bytes: usize, resolver: &RecoveryResolver| {
            recover_session_lease_body(
                &ObservedSessionLeaseBody(Some(body)),
                &store(),
                lease.session_admission_lease_digest(),
                maximum_body_bytes,
                resolver,
            )
        };

    assert_eq!(
        recover(exact_body.clone(), canonical.len(), &resolver).unwrap(),
        lease
    );

    let other_store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("other-control-store").expect("other store id"),
        ControlIncarnationIdV1::parse("other-incarnation").expect("other incarnation"),
    );
    let wrong_store = SessionLeaseBody::new(
        other_store,
        exact_body.digest().clone(),
        exact_body.schema_id().clone(),
        exact_body.schema_sha256().clone(),
        exact_body.canonical_wire_jcs().clone(),
    );
    assert!(matches!(
        recover(wrong_store, canonical.len(), &resolver),
        Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "retained session lease body"
        ))
    ));

    let other_digest =
        SessionAdmissionLeaseDigest::parse("d".repeat(64)).expect("other lease digest");
    let wrong_digest = SessionLeaseBody::new(
        store(),
        other_digest.clone(),
        exact_body.schema_id().clone(),
        exact_body.schema_sha256().clone(),
        exact_body.canonical_wire_jcs().clone(),
    );
    assert!(matches!(
        recover(wrong_digest, canonical.len(), &resolver),
        Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "retained session lease body"
        ))
    ));

    let wrong_schema = SessionLeaseBody::new(
        store(),
        exact_body.digest().clone(),
        SchemaId::parse("pale/other-session-lease.v1").expect("other schema id"),
        SchemaDigest::parse("e".repeat(64)).expect("other schema digest"),
        exact_body.canonical_wire_jcs().clone(),
    );
    assert!(matches!(
        recover(wrong_schema, canonical.len(), &resolver),
        Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "retained session lease schema"
        ))
    ));

    assert!(matches!(
        recover(exact_body.clone(), canonical.len() - 1, &resolver),
        Err(GenerationLifecycleError::SessionLeaseBoundExceeded)
    ));

    let duplicate_key = SessionLeaseBody::new(
        store(),
        exact_body.digest().clone(),
        exact_body.schema_id().clone(),
        exact_body.schema_sha256().clone(),
        pale_ports::ControlRecordBytes::new(br#"{"schema":"a","schema":"b"}"#.to_vec())
            .expect("duplicate-key body is nonempty"),
    );
    assert!(matches!(
        recover(duplicate_key, canonical.len(), &resolver),
        Err(GenerationLifecycleError::SessionLeaseCanonical(_))
    ));

    let noncanonical = SessionLeaseBody::new(
        store(),
        exact_body.digest().clone(),
        exact_body.schema_id().clone(),
        exact_body.schema_sha256().clone(),
        pale_ports::ControlRecordBytes::new(format!(" {canonical}").into_bytes())
            .expect("noncanonical body is nonempty"),
    );
    assert!(matches!(
        recover(noncanonical, canonical.len() + 1, &resolver),
        Err(GenerationLifecycleError::SessionLeaseNonCanonical)
    ));

    let mut mismatched_wire = pale_core::canon::parse_strict(canonical.as_bytes())
        .expect("valid lease wire parses for substitution");
    mismatched_wire["session_admission_lease_digest"] =
        serde_json::to_value(&other_digest).expect("other digest serializes");
    let mismatched_wire = pale_core::canon::to_canonical_string(&mismatched_wire)
        .expect("substituted wire canonicalizes");
    let mismatched_self_digest = SessionLeaseBody::new(
        store(),
        exact_body.digest().clone(),
        exact_body.schema_id().clone(),
        exact_body.schema_sha256().clone(),
        pale_ports::ControlRecordBytes::new(mismatched_wire.as_bytes().to_vec())
            .expect("substituted wire is nonempty"),
    );
    assert!(matches!(
        recover(mismatched_self_digest, mismatched_wire.len(), &resolver),
        Err(GenerationLifecycleError::RecoveryContextSubstitution(
            "retained session lease body"
        ))
    ));

    resolver.profiles.clear();
    let mut unrelated_profile = effective_profile().to_preimage();
    unrelated_profile.project_id =
        pale_core::records::LogicalProjectIdV1::parse("project-2").expect("unrelated project id");
    let unrelated_profile = pale_core::records::EffectiveProjectProfileV1::seal(unrelated_profile)
        .expect("unrelated effective profile seals");
    resolver.insert_effective_profile(&unrelated_profile);
    assert!(matches!(
        recover(exact_body.clone(), canonical.len(), &resolver),
        Err(GenerationLifecycleError::MissingRecoveryContext(
            "session effective profile"
        ))
    ));

    resolver.profiles.clear();
    resolver
        .profiles
        .insert(lease.profile_digest().clone(), unrelated_profile);
    assert!(matches!(
        recover(exact_body, canonical.len(), &resolver),
        Err(GenerationLifecycleError::SessionLeaseAdmission(_))
    ));
}

#[test]
fn checkpoint_session_pins_are_stable_identities_re_admitted_from_owner_custody() {
    let port = MemoryAuthorityStore::default();
    let (state, mut resolver) = committed_initial_lifecycle(&port);
    let snapshot = state.snapshot();
    let generation = resolver
        .generations
        .get(
            resolver
                .active
                .get(snapshot.active.as_ref().expect("active digest"))
                .expect("active head")
                .compiled_generation_digest(),
        )
        .expect("compiled generation")
        .clone();
    let active = resolver
        .active
        .get(snapshot.active.as_ref().expect("active digest"))
        .expect("active head")
        .clone();
    let fence = resolver
        .fences
        .get(snapshot.activation_fence.as_ref().expect("fence digest"))
        .expect("activation fence")
        .clone();
    let lease = session_lease(&generation, &active, &fence);
    resolver.insert_effective_profile(&effective_profile());
    retain_session_lease_body(&port, &store(), &lease, limits().maximum_entry_bytes())
        .expect("session lease body retains before its journal pin");

    let recovered = complete_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("current frontier recovers before writer resume"),
    );
    let mut journal = DurableAuthorityJournal::resume(&port, limits(), recovered);
    let pinned = journal
        .commit_transition(state.pin_session(&lease).expect("pin transition"))
        .expect("pin commit")
        .into_parts()
        .1
        .lifecycle()
        .clone();
    journal.store_checkpoint().expect("checkpoint stored");

    let checkpoint = port.checkpoints.lock().expect("checkpoints")[0].clone();
    let value =
        pale_core::canon::parse_strict(checkpoint.bytes().as_slice()).expect("checkpoint parses");
    assert_eq!(
        value["authority"]["lifecycle"]["session_pins"],
        serde_json::json!([{
            "session_admission_id": lease.session_admission_id().as_str(),
            "lease_digest": lease.session_admission_lease_digest().as_str(),
        }])
    );
    let recovered = complete_recovery(
        recover_authority(&port, store(), limits(), genesis(), &resolver)
            .expect("owner-resolved lease recovers"),
    );
    assert_eq!(recovered.lifecycle().snapshot(), pinned.snapshot());

    let mut substituted = value;
    substituted["authority"]["lifecycle"]["session_pins"][0]["session_admission_id"] =
        serde_json::json!("session-substituted");
    let substituted = pale_core::canon::to_canonical_string(&substituted)
        .expect("substituted checkpoint canonicalizes");
    port.checkpoints.lock().expect("checkpoints")[0] = StoredRecoveryCheckpoint::new(
        checkpoint.through_sequence(),
        BoundedRecoveryCheckpointBytes::new(
            substituted.into_bytes(),
            limits().maximum_checkpoint_bytes(),
        )
        .expect("substituted checkpoint is bounded"),
    );
    assert!(matches!(
        recover_authority(&port, store(), limits(), genesis(), &resolver),
        Err(AuthorityJournalError::Lifecycle(
            GenerationLifecycleError::RecoveryContextSubstitution("live session lease body")
        ))
    ));

    port.checkpoints.lock().expect("checkpoints")[0] = checkpoint;

    port.session_lease_bodies
        .lock()
        .expect("session bodies")
        .clear();
    assert!(matches!(
        recover_authority(&port, store(), limits(), genesis(), &resolver),
        Err(AuthorityJournalError::Lifecycle(
            GenerationLifecycleError::MissingRecoveryContext("retained session lease body")
        ))
    ));
}

#[test]
fn one_logical_session_cannot_be_rebound_to_different_lease_bytes() {
    let port = MemoryAuthorityStore::default();
    let (state, resolver) = committed_initial_lifecycle(&port);
    let snapshot = state.snapshot();
    let active = resolver
        .active
        .get(snapshot.active.as_ref().expect("active digest"))
        .expect("active head");
    let generation = resolver
        .generations
        .get(active.compiled_generation_digest())
        .expect("compiled generation");
    let fence = resolver
        .fences
        .get(snapshot.activation_fence.as_ref().expect("fence digest"))
        .expect("activation fence");
    let first = session_lease_with_budget(generation, active, fence, 10);
    let conflicting = session_lease_with_budget(generation, active, fence, 11);
    assert_eq!(
        first.session_admission_id(),
        conflicting.session_admission_id()
    );
    assert_ne!(
        first.session_admission_lease_digest(),
        conflicting.session_admission_lease_digest()
    );

    let pinned = state
        .pin_session(&first)
        .expect("first lease pins")
        .into_successor();
    assert!(matches!(
        pinned.pin_session(&conflicting),
        Err(GenerationLifecycleError::SessionIdentityConflict)
    ));
}
