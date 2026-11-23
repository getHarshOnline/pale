//! Laws for domain-neutral custody ambiguity closure.

use std::collections::VecDeque;

use pale_ports::{ControlStoreError, ControlStoreErrorKind};

use super::*;

struct Database {
    commit: Option<Result<DatabaseCommitDisposition, ControlStoreError>>,
    reads: VecDeque<Result<DatabaseReadBackDisposition, ControlStoreError>>,
    commits: usize,
    observations: usize,
}

fn database(
    commit: Result<DatabaseCommitDisposition, ControlStoreError>,
    reads: impl IntoIterator<Item = Result<DatabaseReadBackDisposition, ControlStoreError>>,
) -> Database {
    Database {
        commit: Some(commit),
        reads: reads.into_iter().collect(),
        commits: 0,
        observations: 0,
    }
}

fn commit(store: &mut Database, _: &()) -> Result<DatabaseCommitDisposition, ControlStoreError> {
    store.commits += 1;
    store.commit.take().expect("one database commit")
}

fn observe(store: &mut Database, _: &()) -> Result<DatabaseReadBackDisposition, ControlStoreError> {
    store.observations += 1;
    store.reads.pop_front().expect("one database read-back")
}

#[test]
fn direct_database_outcomes_never_read_back_or_retry() {
    for (disposition, evidence) in [
        (
            DatabaseCommitDisposition::Committed,
            DatabaseCommitEvidence::New,
        ),
        (
            DatabaseCommitDisposition::ExactReplay,
            DatabaseCommitEvidence::ExactReplay,
        ),
    ] {
        let mut store = database(Ok(disposition), []);
        let observed = commit_with_exact_readback(&mut store, &(), commit, observe)
            .expect("direct commit outcome");

        assert_eq!(observed, evidence);
        assert_eq!(store.commits, 1);
        assert_eq!(store.observations, 0);
    }
}

#[test]
fn ambiguous_database_commit_observes_once_and_only_exact_successor_continues() {
    let unknown = || ControlStoreError::new(ControlStoreErrorKind::OutcomeUnknown);
    let cases = [
        DatabaseReadBackDisposition::ExactSuccessor,
        DatabaseReadBackDisposition::Predecessor,
        DatabaseReadBackDisposition::ConflictingOrPartial,
    ];
    for read_back in cases {
        let mut store = database(Err(unknown()), [Ok(read_back)]);
        let outcome = commit_with_exact_readback(&mut store, &(), commit, observe);

        match read_back {
            DatabaseReadBackDisposition::ExactSuccessor => assert_eq!(
                outcome.expect("exact successor recovers"),
                DatabaseCommitEvidence::RecoveredExactAfterAmbiguousCommit
            ),
            DatabaseReadBackDisposition::Predecessor => assert!(matches!(
                outcome,
                Err(DatabaseCommitFailureReason::CommitExcludedByReadBack { .. })
            )),
            DatabaseReadBackDisposition::ConflictingOrPartial => assert!(matches!(
                outcome,
                Err(DatabaseCommitFailureReason::RecoveryRequired { .. })
            )),
        }
        assert_eq!(store.commits, 1);
        assert_eq!(store.observations, 1);
    }
}

#[test]
fn ambiguous_database_commit_with_failed_read_back_never_retries() {
    let mut store = database(
        Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        )),
        [Err(ControlStoreError::new(ControlStoreErrorKind::Storage))],
    );

    assert!(matches!(
        commit_with_exact_readback(&mut store, &(), commit, observe),
        Err(DatabaseCommitFailureReason::ReadBackFailed { .. })
    ));
    assert_eq!(store.commits, 1);
    assert_eq!(store.observations, 1);
}

#[test]
fn nonambiguous_database_failure_does_not_read_back() {
    let mut store = database(
        Err(ControlStoreError::new(ControlStoreErrorKind::Storage)),
        [],
    );
    assert!(matches!(
        commit_with_exact_readback(&mut store, &(), commit, observe),
        Err(DatabaseCommitFailureReason::Store(_))
    ));
    assert_eq!(store.commits, 1);
    assert_eq!(store.observations, 0);
}
