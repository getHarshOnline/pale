//! Append-before-notify causal-law tests.

use pale_core::records::{JournalSequenceV1, RingCursorV1, WakeEpochV1};
use pale_kernel::{WakePublishError, WakeReadError, WakeTokenRing};
use pale_ports::{AuthorityByteClass, AuthorityRecoveryReadLimits, DurableAuthorityStoreErrorKind};

use super::*;
use crate::resident::journal::tests::{
    empty_journal, genesis, limits, recover_for_resident_test, store, MemoryAuthorityStore,
};
use crate::resident::journal::{AuthorityJournalError, AuthorityRecoveryOutcome};
use crate::resident::lifecycle::tests::compiled_generation;
use crate::resident_fixture::{published, RecoveryResolver};

fn empty_ring(starting_cursor: u64, epoch: u64) -> WakeTokenRing {
    WakeTokenRing::new(
        4,
        WakeEpochV1::new(epoch),
        RingCursorV1::new(starting_cursor),
        None,
    )
    .expect("bounded wake ring")
}

fn first_publish_transition(tag: char) -> GenerationLifecycleTransition {
    let generation = compiled_generation(tag);
    let published = published(&generation, 1, None);
    genesis()
        .publish(published)
        .expect("initial publish transition")
}

fn empty_coordinator(port: &MemoryAuthorityStore) -> ResidentAuthorityCoordinator<'_> {
    ResidentAuthorityCoordinator::resume(empty_journal(port), empty_ring(0, 1))
        .expect("empty frontiers align")
}

#[test]
fn successful_append_publishes_the_exact_durable_sequence() {
    let port = MemoryAuthorityStore::default();
    let mut coordinator = empty_coordinator(&port);

    let notification = coordinator
        .commit_transition(first_publish_transition('4'))
        .expect("append and notification succeed");

    assert!(matches!(
        notification,
        AuthorityCommitNotification::Published(token)
            if token.journal_sequence().get() == 1
                && token.cursor().get() == 1
    ));
    assert_eq!(notification.journal_sequence().get(), 1);
    assert_eq!(
        coordinator.journal_head().map(JournalSequenceV1::get),
        Some(1)
    );
    assert_eq!(
        coordinator
            .wake_cursor()
            .journal_sequence()
            .map(JournalSequenceV1::get),
        Some(1)
    );
    assert_eq!(port.append_calls(), 1);
}

#[test]
fn unpublished_append_blocks_every_follow_on_transition() {
    let port = MemoryAuthorityStore::default();
    let mut coordinator = empty_coordinator(&port);
    let generation = compiled_generation('4');
    let appended = coordinator
        .append_transition(first_publish_transition('4'))
        .expect("first append is proven");
    let follow_on = coordinator
        .lifecycle()
        .retain_for_rollback(generation.compiled_generation_digest().clone())
        .expect("installed authority is the successor basis");

    assert!(matches!(
        coordinator.commit_transition(follow_on),
        Err(ResidentAuthorityCoordinationError::PublicationPending { journal_sequence })
            if journal_sequence.get() == 1
    ));
    assert_eq!(port.append_calls(), 1);
    assert!(matches!(
        coordinator
            .publish_appended(appended)
            .expect("the affine append token publishes once"),
        AuthorityCommitNotification::Published(token)
            if token.journal_sequence().get() == 1
    ));
}

#[test]
fn proven_append_failures_publish_nothing() {
    for failure in [
        DurableAuthorityStoreErrorKind::Conflict,
        DurableAuthorityStoreErrorKind::Unavailable,
        DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry),
        DurableAuthorityStoreErrorKind::BoundExceeded {
            class: AuthorityByteClass::JournalEntry,
        },
    ] {
        let port = MemoryAuthorityStore::default();
        port.enqueue(Err(failure.into()));
        let mut coordinator = empty_coordinator(&port);
        let starting_cursor = coordinator.wake_cursor();

        let result = coordinator.commit_transition(first_publish_transition('4'));

        match result {
            Err(ResidentAuthorityCoordinationError::Journal(AuthorityJournalError::Store(
                observed,
            ))) => assert_eq!(observed.kind(), failure),
            _ => panic!("unexpected append result"),
        }
        assert_eq!(coordinator.journal_head(), None);
        assert_eq!(coordinator.wake_cursor(), starting_cursor);
        assert_eq!(port.append_calls(), 1);
    }
}

#[test]
fn ambiguous_physical_outcomes_recover_without_retry_or_duplicate_sequence() {
    for commit_before_error in [false, true] {
        let port = MemoryAuthorityStore::default();
        if commit_before_error {
            port.enqueue_commit_then_ambiguous();
        } else {
            port.enqueue(Err(DurableAuthorityStoreErrorKind::AmbiguousCommit.into()));
        }
        let generation = compiled_generation('4');
        let head = published(&generation, 1, None);
        let first = genesis()
            .publish(head.clone())
            .expect("initial publish transition");
        let mut resolver = RecoveryResolver::default();
        resolver.insert_generation(&generation);
        resolver.insert_published(&head);
        let mut coordinator = empty_coordinator(&port);
        let starting_cursor = coordinator.wake_cursor();

        assert!(matches!(
            coordinator.commit_transition(first),
            Err(ResidentAuthorityCoordinationError::Journal(
                AuthorityJournalError::Store(error)
            )) if error.kind() == DurableAuthorityStoreErrorKind::AmbiguousCommit
        ));
        assert_eq!(coordinator.wake_cursor(), starting_cursor);
        assert!(matches!(
            coordinator.commit_transition(first_publish_transition('5')),
            Err(ResidentAuthorityCoordinationError::Journal(
                AuthorityJournalError::WriterCommitOutcomeUnknown
            ))
        ));
        assert_eq!(port.append_calls(), 1);
        drop(coordinator);

        let recovery = recover_for_resident_test(&port, store(), limits(), genesis(), &resolver)
            .expect("explicit recovery resolves the ambiguous physical outcome");
        let AuthorityRecoveryOutcome::Complete(recovered) = recovery else {
            panic!("valid ambiguous outcome must recover completely");
        };
        let durable_head = recovered.journal_head();
        assert_eq!(
            durable_head.map(JournalSequenceV1::get),
            commit_before_error.then_some(1)
        );
        let journal = DurableAuthorityJournal::resume(&port, limits(), *recovered);
        let ring = WakeTokenRing::new(4, WakeEpochV1::new(2), RingCursorV1::new(0), durable_head)
            .expect("fresh recovery ring");
        let mut coordinator =
            ResidentAuthorityCoordinator::resume(journal, ring).expect("recovered frontiers align");
        let transition = if commit_before_error {
            coordinator
                .lifecycle()
                .retain_for_rollback(generation.compiled_generation_digest().clone())
                .expect("committed generation is retained")
        } else {
            coordinator
                .lifecycle()
                .publish(head)
                .expect("uncommitted publication remains the next transition")
        };
        let notification = coordinator
            .commit_transition(transition)
            .expect("recovered writer advances exactly once");
        let expected = if commit_before_error { 2 } else { 1 };
        assert!(matches!(
            notification,
            AuthorityCommitNotification::Published(token)
                if token.journal_sequence().get() == expected
        ));
        assert_eq!(port.append_calls(), 2);
    }
}

#[test]
fn wake_failure_after_append_remains_a_committed_replay_obligation() {
    let port = MemoryAuthorityStore::default();
    let ring = empty_ring(u64::MAX, 1);
    let starting_cursor = ring.current_cursor();
    let mut coordinator = ResidentAuthorityCoordinator::resume(empty_journal(&port), ring)
        .expect("empty durable frontiers align");
    let generation = compiled_generation('4');
    let head = published(&generation, 1, None);

    let notification = coordinator
        .commit_transition(
            genesis()
                .publish(head.clone())
                .expect("initial publish transition"),
        )
        .expect("wake failure is a committed disposition");

    assert!(matches!(
        notification,
        AuthorityCommitNotification::DurableReplayRequired {
            journal_sequence,
            reason: WakePublishError::CursorExhausted { cursor }
        } if journal_sequence.get() == 1 && cursor.get() == u64::MAX
    ));
    assert_eq!(
        coordinator.journal_head().map(JournalSequenceV1::get),
        Some(1)
    );
    assert_eq!(coordinator.wake_cursor(), starting_cursor);
    assert_eq!(port.append_calls(), 1);
    drop(coordinator);

    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(&generation);
    resolver.insert_published(&head);
    let recovery = recover_for_resident_test(&port, store(), limits(), genesis(), &resolver)
        .expect("committed authority recovers after wake failure");
    let AuthorityRecoveryOutcome::Complete(recovered) = recovery else {
        panic!("valid committed prefix must recover completely");
    };
    let durable_head = recovered.journal_head();
    let journal = DurableAuthorityJournal::resume(&port, limits(), *recovered);
    let ring = WakeTokenRing::new(4, WakeEpochV1::new(2), RingCursorV1::new(0), durable_head)
        .expect("fresh ring at recovered durable frontier");
    let mut coordinator =
        ResidentAuthorityCoordinator::resume(journal, ring).expect("recovered frontiers align");
    let transition = coordinator
        .lifecycle()
        .retain_for_rollback(generation.compiled_generation_digest().clone())
        .expect("recovered generation is retained");
    let notification = coordinator
        .commit_transition(transition)
        .expect("fresh writer resumes ordinary publication");
    assert!(matches!(
        notification,
        AuthorityCommitNotification::Published(token)
            if token.journal_sequence().get() == 2
    ));
    assert_eq!(port.append_calls(), 2);
}

#[test]
fn pre_append_bound_failure_never_reaches_storage_or_notification() {
    let port = MemoryAuthorityStore::default();
    let bounded =
        AuthorityRecoveryReadLimits::new(1, 1_000_000, 8, 64).expect("nonzero restrictive limits");
    let recovery = recover_for_resident_test(
        &port,
        store(),
        bounded,
        genesis(),
        &RecoveryResolver::default(),
    )
    .expect("empty store recovers under restrictive limits");
    let AuthorityRecoveryOutcome::Complete(recovered) = recovery else {
        panic!("empty recovery must be complete");
    };
    let journal = DurableAuthorityJournal::resume(&port, bounded, *recovered);
    let mut coordinator = ResidentAuthorityCoordinator::resume(journal, empty_ring(0, 1))
        .expect("empty frontiers align");
    let starting_cursor = coordinator.wake_cursor();

    assert!(matches!(
        coordinator.commit_transition(first_publish_transition('4')),
        Err(ResidentAuthorityCoordinationError::Journal(
            AuthorityJournalError::Store(error)
        )) if error.kind() == (DurableAuthorityStoreErrorKind::BoundExceeded {
            class: AuthorityByteClass::JournalEntry
        })
    ));
    assert_eq!(port.append_calls(), 0);
    assert_eq!(coordinator.wake_cursor(), starting_cursor);
}

#[test]
fn follow_on_transition_derives_from_committed_state_after_wake_failure() {
    let port = MemoryAuthorityStore::default();
    let ring = empty_ring(u64::MAX, 1);
    let starting_cursor = ring.current_cursor();
    let mut coordinator = ResidentAuthorityCoordinator::resume(empty_journal(&port), ring)
        .expect("empty frontiers align");
    let first_generation = compiled_generation('4');
    let first_head = published(&first_generation, 1, None);
    let first_notification = coordinator
        .commit_transition(
            genesis()
                .publish(first_head.clone())
                .expect("first transition"),
        )
        .expect("first authority commits");
    assert!(matches!(
        first_notification,
        AuthorityCommitNotification::DurableReplayRequired {
            journal_sequence,
            reason: WakePublishError::CursorExhausted { .. }
        } if journal_sequence.get() == 1
    ));
    let second_generation = compiled_generation('5');
    let second_head = published(&second_generation, 2, Some(&first_head));
    let second_transition = coordinator
        .lifecycle()
        .publish(second_head)
        .expect("follow-on derives from committed lifecycle");
    let second_notification = coordinator
        .commit_transition(second_transition)
        .expect("second authority also commits");
    assert!(matches!(
        second_notification,
        AuthorityCommitNotification::DurableReplayRequired {
            journal_sequence,
            reason: WakePublishError::Gap { expected, supplied }
        } if journal_sequence.get() == 2
            && expected.get() == 1
            && supplied.get() == 2
    ));
    assert_eq!(
        coordinator
            .lifecycle()
            .snapshot()
            .retained_generations
            .len(),
        2
    );
    assert_eq!(
        coordinator.journal_head().map(JournalSequenceV1::get),
        Some(2)
    );
    assert_eq!(coordinator.wake_cursor(), starting_cursor);
    assert_eq!(port.append_calls(), 2);
}

#[test]
fn resume_rejects_a_substituted_wake_frontier() {
    let port = MemoryAuthorityStore::default();
    let wake_head = JournalSequenceV1::new(1).expect("wake head");
    let ring = WakeTokenRing::new(
        4,
        WakeEpochV1::new(1),
        RingCursorV1::new(0),
        Some(wake_head),
    )
    .expect("wake ring");

    assert!(matches!(
        ResidentAuthorityCoordinator::resume(empty_journal(&port), ring),
        Err(ResidentAuthorityCoordinationError::FrontierMismatch {
            journal_head: None,
            wake_head: Some(observed),
        }) if observed == wake_head
    ));
}

#[test]
fn crash_after_append_before_publish_recovers_the_exact_replay_interval() {
    let port = MemoryAuthorityStore::default();
    let old_ring = empty_ring(0, 1);
    let old_cursor = old_ring.starting_cursor();
    let generation = compiled_generation('4');
    let published = published(&generation, 1, None);
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(&generation);
    resolver.insert_published(&published);

    let mut journal = empty_journal(&port);
    let _ = journal
        .commit_transition(genesis().publish(published).expect("publish transition"))
        .expect("durable append before simulated crash");
    drop(journal);

    let recovery = recover_for_resident_test(&port, store(), limits(), genesis(), &resolver)
        .expect("durable journal recovers");
    let AuthorityRecoveryOutcome::Complete(recovered) = recovery else {
        panic!("valid committed prefix must recover completely");
    };
    let durable_head = recovered.journal_head();
    let journal = DurableAuthorityJournal::resume(&port, limits(), *recovered);
    let ring = WakeTokenRing::new(4, WakeEpochV1::new(2), RingCursorV1::new(0), durable_head)
        .expect("reconstructed ring");
    let coordinator =
        ResidentAuthorityCoordinator::resume(journal, ring).expect("recovered frontiers align");

    assert!(matches!(
        coordinator.wake_ring().read_after(old_cursor),
        Err(WakeReadError::StaleEpoch {
            replay_after: None,
            durable_through: Some(sequence),
            ..
        }) if sequence.get() == 1
    ));
}
