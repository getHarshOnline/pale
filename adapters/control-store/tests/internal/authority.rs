//! Deployment-only durable authority adapter tests.

use super::*;
use crate::test_support as support;
use pale_core::records::{ControlIncarnationIdV1, ControlStoreIdV1};
use std::error::Error as _;

fn coordinate(identity: &ControlStoreIdentity) -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    )
}

fn limits(
    maximum_entry_bytes: usize,
    maximum_checkpoint_bytes: usize,
    maximum_checkpoint_candidates: usize,
    maximum_tail_entries: usize,
) -> pale_ports::AuthorityRecoveryReadLimits {
    pale_ports::AuthorityRecoveryReadLimits::new(
        maximum_entry_bytes,
        maximum_checkpoint_bytes,
        maximum_checkpoint_candidates,
        maximum_tail_entries,
    )
    .unwrap()
}

fn append(
    store: &DeploymentControlSqliteStore,
    coordinate: &AuthorityStoreCoordinate,
    expected_head: Option<JournalSequenceV1>,
    sequence: u64,
    payload: &[u8],
) -> Result<(), DurableAuthorityStoreError> {
    let bytes = BoundedAuthorityEntryBytes::new(payload.to_vec(), payload.len()).unwrap();
    let entry = SequencedAuthorityEntry::new(JournalSequenceV1::new(sequence).unwrap(), bytes);
    let prepared = PreparedAuthorityAppend::new(coordinate.clone(), expected_head, entry).unwrap();
    store.compare_and_append(&prepared)
}

fn checkpoint(
    store: &DeploymentControlSqliteStore,
    coordinate: &AuthorityStoreCoordinate,
    sequence: u64,
    payload: &[u8],
) -> Result<(), DurableAuthorityStoreError> {
    let sequence = JournalSequenceV1::new(sequence).unwrap();
    let bytes = BoundedRecoveryCheckpointBytes::new(payload.to_vec(), payload.len()).unwrap();
    let stored = StoredRecoveryCheckpoint::new(sequence, bytes);
    let prepared = PreparedRecoveryCheckpoint::new(coordinate.clone(), sequence, stored).unwrap();
    store.compare_and_store_checkpoint(&prepared)
}

fn deployment_store(
    directory: &tempfile::TempDir,
    suffix: &str,
) -> (
    DeploymentControlSqliteStore,
    AuthorityStoreCoordinate,
    crate::location::ControlStoreLocation,
) {
    let (location, identity, _) =
        support::initialize(directory, ControlStoreKind::DeploymentControl, suffix);
    let coordinate = coordinate(&identity);
    let store =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
            .unwrap();
    (store, coordinate, location)
}

fn read_page(
    store: &DeploymentControlSqliteStore,
    coordinate: AuthorityStoreCoordinate,
    after: Option<JournalSequenceV1>,
    through: u64,
    recovery_limits: pale_ports::AuthorityRecoveryReadLimits,
    maximum_page_entries: usize,
) -> Result<JournalTailPage, DurableAuthorityStoreError> {
    store.read_journal_page(
        &JournalTailPageRead::new(
            coordinate,
            after,
            JournalSequenceV1::new(through).unwrap(),
            recovery_limits,
            maximum_page_entries,
        )
        .unwrap(),
    )
}

#[test]
fn compare_and_append_is_an_exact_atomic_head_cas() {
    let directory = support::private_tempdir();
    let (store, coordinate, _) = deployment_store(&directory, "append");
    append(&store, &coordinate, None, 1, b"one").unwrap();
    assert_eq!(
        append(&store, &coordinate, None, 1, b"duplicate").map_err(|error| error.kind()),
        Err(DurableAuthorityStoreErrorKind::Conflict)
    );
    append(
        &store,
        &coordinate,
        Some(JournalSequenceV1::new(1).unwrap()),
        2,
        b"two",
    )
    .unwrap();

    assert_eq!(
        store.read_journal_head(&coordinate).unwrap(),
        Some(JournalSequenceV1::new(2).unwrap())
    );
    let page = read_page(&store, coordinate.clone(), None, 2, limits(16, 16, 4, 4), 4).unwrap();
    assert_eq!(
        page.entries()
            .iter()
            .map(|entry| (entry.sequence().get(), entry.bytes().as_slice()))
            .collect::<Vec<_>>(),
        vec![(1, b"one".as_slice()), (2, b"two".as_slice())]
    );
    assert_eq!(page.boundary(), JournalTailPageBoundary::Complete);
}

#[test]
fn journal_and_recovery_projection_share_one_store_clock_without_number_aliasing() {
    let directory = support::private_tempdir();
    let (store, coordinate, location) = deployment_store(&directory, "transaction-clock");
    let connection = Connection::open(location.path()).unwrap();
    let head = || {
        crate::store::transaction::read(&connection, &coordinate)
            .unwrap()
            .get()
    };
    assert_eq!(head(), 1);
    append(&store, &coordinate, None, 1, b"one").unwrap();
    assert_eq!(head(), 2);
    assert_eq!(
        append(&store, &coordinate, None, 1, b"one")
            .unwrap_err()
            .kind(),
        DurableAuthorityStoreErrorKind::Conflict
    );
    assert_eq!(head(), 2);
    checkpoint(&store, &coordinate, 1, b"projection").unwrap();
    assert_eq!(head(), 3);
    checkpoint(&store, &coordinate, 1, b"projection").unwrap();
    assert_eq!(head(), 3);
    assert_eq!(
        store.read_journal_head(&coordinate).unwrap().unwrap().get(),
        1
    );
    assert_eq!(
        head(),
        3,
        "ordinary read and exact projection replay do not mutate"
    );
    assert_eq!(connection.execute("UPDATE pale_control_store_header SET logical_transaction_sequence=9007199254740991 WHERE singleton=1", []).unwrap(), 1);
    checkpoint(&store, &coordinate, 1, b"projection").unwrap();
    let error = append(
        &store,
        &coordinate,
        Some(JournalSequenceV1::new(1).unwrap()),
        2,
        b"two",
    )
    .unwrap_err();
    assert_eq!(error.kind(), DurableAuthorityStoreErrorKind::Unavailable);
    assert_eq!(
        store.read_journal_head(&coordinate).unwrap().unwrap().get(),
        1
    );
    assert_eq!(head(), 9_007_199_254_740_991);
}

#[test]
fn fixed_recovery_head_excludes_later_appends_across_pages() {
    let directory = support::private_tempdir();
    let (store, coordinate, _) = deployment_store(&directory, "fixed-head");
    append(&store, &coordinate, None, 1, b"one").unwrap();
    append(
        &store,
        &coordinate,
        Some(JournalSequenceV1::new(1).unwrap()),
        2,
        b"two",
    )
    .unwrap();
    let request = JournalTailPageRead::new(
        coordinate.clone(),
        None,
        JournalSequenceV1::new(2).unwrap(),
        limits(16, 16, 4, 4),
        1,
    )
    .unwrap();
    let first = store.read_journal_page(&request).unwrap();
    assert_eq!(first.entries()[0].sequence().get(), 1);
    assert!(matches!(
        first.boundary(),
        JournalTailPageBoundary::More { .. }
    ));

    append(
        &store,
        &coordinate,
        Some(JournalSequenceV1::new(2).unwrap()),
        3,
        b"three",
    )
    .unwrap();
    let continuation = request.continue_after(&first).unwrap().unwrap();
    let second = store.read_journal_page(&continuation).unwrap();
    assert_eq!(second.entries()[0].sequence().get(), 2);
    assert_eq!(second.boundary(), JournalTailPageBoundary::Complete);
    assert_eq!(
        store.read_journal_head(&coordinate).unwrap().unwrap().get(),
        3
    );
}

#[test]
fn every_operation_rejects_cross_incarnation_substitution() {
    let directory = support::private_tempdir();
    let (store, coordinate, _) = deployment_store(&directory, "coordinate");
    let wrong = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("other-store").unwrap(),
        ControlIncarnationIdV1::parse("other-incarnation").unwrap(),
    );
    assert_eq!(
        append(&store, &wrong, None, 1, b"wrong").map_err(|error| error.kind()),
        Err(DurableAuthorityStoreErrorKind::Unavailable)
    );
    assert_eq!(
        store
            .read_journal_head(&wrong)
            .map_err(|error| error.kind()),
        Err(DurableAuthorityStoreErrorKind::Unavailable)
    );
    append(&store, &coordinate, None, 1, b"right").unwrap();
}

#[test]
fn checkpoints_compare_the_head_and_are_returned_newest_first() {
    let directory = support::private_tempdir();
    let (store, coordinate, _) = deployment_store(&directory, "checkpoint");
    append(&store, &coordinate, None, 1, b"one").unwrap();
    checkpoint(&store, &coordinate, 1, b"checkpoint-one").unwrap();
    append(
        &store,
        &coordinate,
        Some(JournalSequenceV1::new(1).unwrap()),
        2,
        b"two",
    )
    .unwrap();
    checkpoint(&store, &coordinate, 2, b"checkpoint-two").unwrap();
    assert_eq!(
        checkpoint(&store, &coordinate, 1, b"stale").map_err(|error| error.kind()),
        Err(DurableAuthorityStoreErrorKind::Conflict)
    );

    let checkpoints = store
        .read_recovery_checkpoints(&RecoveryCheckpointRead::new(
            coordinate,
            limits(16, 32, 2, 2),
        ))
        .unwrap();
    assert_eq!(
        checkpoints
            .iter()
            .map(|value| value.through_sequence().get())
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
}

#[test]
fn corrupt_newest_checkpoint_does_not_hide_an_older_trusted_candidate() {
    let directory = support::private_tempdir();
    let (store, coordinate, _) = deployment_store(&directory, "checkpoint-fallback");
    append(&store, &coordinate, None, 1, b"one").unwrap();
    checkpoint(&store, &coordinate, 1, b"checkpoint-one").unwrap();
    append(
        &store,
        &coordinate,
        Some(JournalSequenceV1::new(1).unwrap()),
        2,
        b"two",
    )
    .unwrap();
    checkpoint(&store, &coordinate, 2, b"checkpoint-two").unwrap();
    store
        .connection
        .lock()
        .unwrap()
        .execute(
            "UPDATE pale_deployment_recovery_checkpoint SET physical_frame = x'00' \
             WHERE through_journal_sequence = 2",
            [],
        )
        .unwrap();

    let candidates = store
        .read_recovery_checkpoints(&RecoveryCheckpointRead::new(
            coordinate,
            limits(16, 32, 2, 2),
        ))
        .unwrap();
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates[0].rejection(),
        Some(RecoveryCheckpointPhysicalRejection::Integrity)
    );
    assert_eq!(candidates[0].through_sequence().get(), 2);
    assert_eq!(
        candidates[1]
            .trusted()
            .expect("older checkpoint remains trusted")
            .bytes()
            .as_slice(),
        b"checkpoint-one"
    );
}

#[test]
fn restart_reopens_the_same_exact_coordinate_without_losing_bytes() {
    let directory = support::private_tempdir();
    let (store, coordinate, location) = deployment_store(&directory, "restart");
    append(&store, &coordinate, None, 1, b"durable").unwrap();
    drop(store);

    let reopened =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
            .unwrap();
    let page = read_page(&reopened, coordinate, None, 1, limits(16, 16, 1, 1), 1).unwrap();
    assert_eq!(page.entries()[0].bytes().as_slice(), b"durable");
}

#[test]
fn first_corrupt_tail_frame_quarantines_only_the_exact_valid_prefix() {
    let directory = support::private_tempdir();
    let (store, coordinate, _) = deployment_store(&directory, "quarantine");
    append(&store, &coordinate, None, 1, b"one").unwrap();
    append(
        &store,
        &coordinate,
        Some(JournalSequenceV1::new(1).unwrap()),
        2,
        b"two",
    )
    .unwrap();
    store
        .connection
        .lock()
        .unwrap()
        .execute(
            "UPDATE pale_deployment_authority_journal SET physical_frame = x'00' \
             WHERE journal_sequence = 2",
            [],
        )
        .unwrap();

    let page = read_page(&store, coordinate, None, 2, limits(16, 16, 2, 2), 2).unwrap();
    assert_eq!(page.entries().len(), 1);
    assert_eq!(page.entries()[0].sequence().get(), 1);
    assert_eq!(
        page.boundary(),
        JournalTailPageBoundary::QuarantinedFrom {
            first_untrusted_sequence: JournalSequenceV1::new(2).unwrap()
        }
    );
}

#[test]
fn missing_mandatory_head_is_physical_corruption_not_unavailability() {
    let directory = support::private_tempdir();
    let (store, coordinate, _) = deployment_store(&directory, "missing-head");
    store
        .connection
        .lock()
        .unwrap()
        .execute("DELETE FROM pale_deployment_authority_head", [])
        .unwrap();

    let error = store
        .read_journal_head(&coordinate)
        .expect_err("missing singleton rejects recovery");
    assert_eq!(
        error.kind(),
        DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry)
    );
    assert!(matches!(
        sqlite_cause(&error),
        rusqlite::Error::QueryReturnedNoRows
    ));
}

#[test]
fn read_budgets_fail_before_returning_an_oversized_result() {
    let directory = support::private_tempdir();
    let (store, coordinate, _) = deployment_store(&directory, "bounds");
    append(&store, &coordinate, None, 1, b"one").unwrap();
    append(
        &store,
        &coordinate,
        Some(JournalSequenceV1::new(1).unwrap()),
        2,
        b"two",
    )
    .unwrap();
    assert_eq!(
        store
            .read_journal_page(
                &JournalTailPageRead::new(
                    coordinate,
                    None,
                    JournalSequenceV1::new(2).unwrap(),
                    limits(16, 16, 1, 1),
                    1,
                )
                .unwrap(),
            )
            .map_err(|error| error.kind()),
        Err(DurableAuthorityStoreErrorKind::BoundExceeded {
            class: AuthorityByteClass::JournalEntry
        })
    );
}

fn sqlite_cause(error: &DurableAuthorityStoreError) -> &rusqlite::Error {
    error
        .source()
        .and_then(|source| source.downcast_ref::<rusqlite::Error>())
        .expect("the original SQLite error remains the port error source")
}

#[test]
fn native_uncommitted_classification_retains_each_sqlite_cause() {
    for (code, expected) in [
        (
            rusqlite::ffi::SQLITE_CORRUPT,
            DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry),
        ),
        (
            rusqlite::ffi::SQLITE_CONSTRAINT,
            DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry),
        ),
        (
            rusqlite::ffi::SQLITE_BUSY,
            DurableAuthorityStoreErrorKind::Unavailable,
        ),
    ] {
        let error = map_uncommitted(
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(code),
                Some("native marker".to_owned()),
            ),
            AuthorityByteClass::JournalEntry,
        );
        assert_eq!(error.kind(), expected);
        assert_eq!(
            error.append_failure(),
            pale_ports::DurableAuthorityAppendFailure::RejectedNoAppend
        );
        let rusqlite::Error::SqliteFailure(native, message) = sqlite_cause(&error) else {
            panic!("the native SQLite failure changed type");
        };
        assert_eq!(native.extended_code, code);
        assert_eq!(message.as_deref(), Some("native marker"));
    }
}

#[test]
fn begin_failure_preserves_native_cause_without_claiming_an_append() {
    let directory = support::private_tempdir();
    let (store, coordinate, _) = deployment_store(&directory, "begin-source");
    store
        .connection
        .lock()
        .unwrap()
        .execute_batch("BEGIN IMMEDIATE")
        .unwrap();
    let error = append(&store, &coordinate, None, 1, b"not-appended")
        .expect_err("a nested transaction cannot begin");
    assert_eq!(error.kind(), DurableAuthorityStoreErrorKind::Unavailable);
    assert_eq!(
        error.append_failure(),
        pale_ports::DurableAuthorityAppendFailure::RejectedNoAppend
    );
    assert!(matches!(
        sqlite_cause(&error),
        rusqlite::Error::SqliteFailure(..)
    ));
    let connection = store.connection.lock().unwrap();
    connection.execute_batch("ROLLBACK").unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM pale_deployment_authority_journal",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn insert_failure_preserves_constraint_source_and_rolls_back() {
    let directory = support::private_tempdir();
    let (store, coordinate, _) = deployment_store(&directory, "insert-source");
    store
        .connection
        .lock()
        .unwrap()
        .execute_batch(
            "CREATE TEMP TRIGGER reject_append BEFORE INSERT ON pale_deployment_authority_journal \
         BEGIN SELECT RAISE(ABORT, 'injected append rejection'); END;",
        )
        .unwrap();
    let error = append(&store, &coordinate, None, 1, b"not-appended")
        .expect_err("the trigger rejects before commit");
    assert_eq!(
        error.kind(),
        DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry)
    );
    assert_eq!(
        sqlite_cause(&error).sqlite_error_code(),
        Some(rusqlite::ErrorCode::ConstraintViolation)
    );
    assert_eq!(store.read_journal_head(&coordinate).unwrap(), None);
}

#[test]
fn append_and_checkpoint_commit_failures_retain_native_cause_and_uncertainty() {
    for checkpoint_write in [false, true] {
        let directory = support::private_tempdir();
        let (store, coordinate, _) = deployment_store(&directory, "commit-source");
        if checkpoint_write {
            append(&store, &coordinate, None, 1, b"one").unwrap();
        }
        let table = if checkpoint_write {
            "pale_deployment_recovery_checkpoint"
        } else {
            "pale_deployment_authority_journal"
        };
        store
            .connection
            .lock()
            .unwrap()
            .execute_batch(&format!(
                "CREATE TEMP TABLE commit_parent (id INTEGER PRIMARY KEY); \
             CREATE TEMP TABLE commit_child (parent_id INTEGER REFERENCES commit_parent(id) \
                 DEFERRABLE INITIALLY DEFERRED); \
             CREATE TEMP TRIGGER reject_commit AFTER INSERT ON {table} \
             BEGIN INSERT INTO commit_child(parent_id) VALUES (1); END;",
            ))
            .unwrap();
        let error = if checkpoint_write {
            checkpoint(&store, &coordinate, 1, b"projection")
        } else {
            append(&store, &coordinate, None, 1, b"one")
        }
        .expect_err("the deferred constraint rejects at commit");
        assert_eq!(
            error.kind(),
            DurableAuthorityStoreErrorKind::AmbiguousCommit
        );
        assert_eq!(
            sqlite_cause(&error).sqlite_error_code(),
            Some(rusqlite::ErrorCode::ConstraintViolation)
        );
        assert_eq!(
            error.append_failure(),
            pale_ports::DurableAuthorityAppendFailure::OutcomeUnknown
        );
        assert_eq!(
            error.checkpoint_failure(),
            pale_ports::DurableAuthorityCheckpointFailure::OutcomeUnknown
        );
        assert_eq!(
            store
                .read_journal_head(&coordinate)
                .unwrap()
                .map(JournalSequenceV1::get),
            checkpoint_write.then_some(1)
        );
    }
}
