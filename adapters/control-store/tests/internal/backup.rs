//! Bounded logical backup and fresh inactive-restore qualification tests.

use std::fs;
use std::os::unix::fs::symlink;
use std::time::{Duration, Instant};

use pale_core::records::{ControlStoreIdV1, JournalSequenceV1};
use pale_ports::{
    AuthorityRecoveryReadLimits, AuthorityStoreCoordinate, BoundedAuthorityEntryBytes,
    BoundedRecoveryCheckpointBytes, DurableAuthorityStoreError, DurableAuthorityStorePort,
    JournalTailPageRead, PreparedAuthorityAppend, PreparedRecoveryCheckpoint,
    RecoveryCheckpointRead, SequencedAuthorityEntry, StoredRecoveryCheckpoint,
};
use rusqlite::backup::Backup;
use rusqlite::Connection;

use super::*;
use crate::authority::DeploymentControlSqliteStore;
use crate::candidate::{sqlite_sidecar, CandidateReservation};
use crate::test_support as support;
use crate::ControlStoreKind;

/// Generous qualification budget used outside explicit bound tests.
fn generous_budget() -> BackupQualificationBudget {
    BackupQualificationBudget::new(16, 4_096, Duration::from_secs(10)).unwrap()
}

/// Project one storage identity into the neutral authority coordinate.
fn coordinate(identity: &ControlStoreIdentity) -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    )
}

/// Append one bounded opaque authority entry.
fn append(
    store: &DeploymentControlSqliteStore,
    coordinate: &AuthorityStoreCoordinate,
    expected_head: Option<JournalSequenceV1>,
    sequence: u64,
    payload: &[u8],
) -> Result<(), DurableAuthorityStoreError> {
    let bytes = BoundedAuthorityEntryBytes::new(payload.to_vec(), payload.len()).unwrap();
    let entry = SequencedAuthorityEntry::new(JournalSequenceV1::new(sequence).unwrap(), bytes);
    let append = PreparedAuthorityAppend::new(coordinate.clone(), expected_head, entry).unwrap();
    store.compare_and_append(&append)
}

/// Store one bounded opaque recovery checkpoint at the exact current head.
fn checkpoint(
    store: &DeploymentControlSqliteStore,
    coordinate: &AuthorityStoreCoordinate,
    sequence: u64,
    payload: &[u8],
) -> Result<(), DurableAuthorityStoreError> {
    let sequence = JournalSequenceV1::new(sequence).unwrap();
    let bytes = BoundedRecoveryCheckpointBytes::new(payload.to_vec(), payload.len()).unwrap();
    let checkpoint = StoredRecoveryCheckpoint::new(sequence, bytes);
    let prepared =
        PreparedRecoveryCheckpoint::new(coordinate.clone(), sequence, checkpoint).unwrap();
    store.compare_and_store_checkpoint(&prepared)
}

/// Current candidate read limits, deliberately local to synthetic qualification.
fn recovery_limits() -> AuthorityRecoveryReadLimits {
    AuthorityRecoveryReadLimits::new(32, 32, 4, 4).unwrap()
}

#[test]
fn backup_and_fresh_restore_preserve_both_store_kinds_without_admission() {
    for (kind, suffix) in [
        (ControlStoreKind::PrincipalTrust, "backup-trust"),
        (ControlStoreKind::DeploymentControl, "backup-deployment"),
    ] {
        let source_root = support::private_tempdir();
        let backup_root = support::private_tempdir();
        let restore_root = support::private_tempdir();
        let (source, identity, _) = support::initialize(&source_root, kind, suffix);
        let backup = support::location(&backup_root, kind, suffix);
        let restore = support::location(&restore_root, kind, suffix);

        qualify_inactive_store_copy(&source, &backup, &identity, generous_budget()).unwrap();
        qualify_inactive_store_copy(&backup, &restore, &identity, generous_budget()).unwrap();

        for candidate in [&backup, &restore] {
            assert!(matches!(
                crate::store::classify_store(candidate, &identity).unwrap(),
                StoreOpenClassification::ProfileUnselected { .. }
            ));
            assert_ne!(candidate.path(), source.path());
        }
        assert_ne!(backup.path(), restore.path());
    }
}

#[test]
fn restored_deployment_recovers_the_exact_checkpoint_and_tail() {
    let source_root = support::private_tempdir();
    let backup_root = support::private_tempdir();
    let restore_root = support::private_tempdir();
    let (source, identity, _) = support::initialize(
        &source_root,
        ControlStoreKind::DeploymentControl,
        "checkpoint-tail",
    );
    let coordinate = coordinate(&identity);
    let source_store =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&source, coordinate.clone())
            .unwrap();
    append(&source_store, &coordinate, None, 1, b"one").unwrap();
    checkpoint(&source_store, &coordinate, 1, b"checkpoint-one").unwrap();
    append(
        &source_store,
        &coordinate,
        Some(JournalSequenceV1::new(1).unwrap()),
        2,
        b"two",
    )
    .unwrap();
    drop(source_store);

    let backup = support::location(
        &backup_root,
        ControlStoreKind::DeploymentControl,
        "checkpoint-tail",
    );
    let restore = support::location(
        &restore_root,
        ControlStoreKind::DeploymentControl,
        "checkpoint-tail",
    );
    qualify_inactive_store_copy(&source, &backup, &identity, generous_budget()).unwrap();
    qualify_inactive_store_copy(&backup, &restore, &identity, generous_budget()).unwrap();

    let restored =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&restore, coordinate.clone())
            .unwrap();
    let checkpoints = restored
        .read_recovery_checkpoints(&RecoveryCheckpointRead::new(
            coordinate.clone(),
            recovery_limits(),
        ))
        .unwrap();
    assert_eq!(checkpoints.len(), 1);
    assert_eq!(
        checkpoints[0]
            .trusted()
            .expect("copied checkpoint remains physically trusted")
            .bytes()
            .as_slice(),
        b"checkpoint-one"
    );
    let page = restored
        .read_journal_page(
            &JournalTailPageRead::new(
                coordinate,
                Some(JournalSequenceV1::new(1).unwrap()),
                JournalSequenceV1::new(2).unwrap(),
                recovery_limits(),
                2,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(page.entries().len(), 1);
    assert_eq!(page.entries()[0].sequence().get(), 2);
    assert_eq!(page.entries()[0].bytes().as_slice(), b"two");
}

#[test]
fn a_read_transaction_pins_the_snapshot_consumed_by_native_backup() {
    let source_root = support::private_tempdir();
    let backup_root = support::private_tempdir();
    let (source, identity, _) = support::initialize(
        &source_root,
        ControlStoreKind::DeploymentControl,
        "snapshot",
    );
    let sources = verify_embedded_storage_sources().unwrap();
    let (snapshot, _) = open_snapshot_source(&source, &identity, &sources).unwrap();

    let coordinate = coordinate(&identity);
    let writer =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&source, coordinate.clone())
            .unwrap();
    append(&writer, &coordinate, None, 1, b"after-snapshot").unwrap();
    assert_eq!(
        writer
            .read_journal_head(&coordinate)
            .unwrap()
            .unwrap()
            .get(),
        1
    );

    let destination = support::location(
        &backup_root,
        ControlStoreKind::DeploymentControl,
        "snapshot",
    );
    let reservation = CandidateReservation::reserve_store(&destination).unwrap();
    let mut destination_connection = reservation
        .open_sqlite("open snapshot test destination")
        .unwrap();
    let backup = Backup::new(&snapshot, &mut destination_connection).unwrap();
    run_backup_steps(&backup, Instant::now(), generous_budget()).unwrap();
    drop(backup);
    close_copy_connections(destination_connection, snapshot).unwrap();
    reservation
        .sync_closed_main("synchronize snapshot test destination")
        .unwrap();
    reservation.seal_identity_and_parent().unwrap();
    drop(reservation);

    let copied =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&destination, coordinate.clone())
            .unwrap();
    assert_eq!(copied.read_journal_head(&coordinate).unwrap(), None);
}

#[test]
fn native_retryable_results_cannot_cross_the_step_budget() {
    let directory = support::private_tempdir();
    let source_path = directory.path().join("source.sqlite3");
    let destination_path = directory.path().join("destination.sqlite3");
    let source = Connection::open(&source_path).unwrap();
    source
        .execute_batch(
            "CREATE TABLE payload (value BLOB NOT NULL); \
             INSERT INTO payload(value) VALUES (zeroblob(32768));",
        )
        .unwrap();
    let one_step = BackupQualificationBudget::new(1, 1, Duration::from_secs(10)).unwrap();

    let mut destination = Connection::open(&destination_path).unwrap();
    let backup = Backup::new(&source, &mut destination).unwrap();
    assert!(matches!(
        run_backup_steps(&backup, Instant::now(), one_step),
        Err(BackupQualificationError::StepBudgetExhausted {
            completed_steps: 1,
            last_step: Some(BackupStepObservation::More),
        })
    ));
    drop(backup);
    drop(destination);
    fs::remove_file(&destination_path).unwrap();

    let mut destination = Connection::open(&destination_path).unwrap();
    destination
        .execute_batch("CREATE TABLE lock_holder (value INTEGER NOT NULL);")
        .unwrap();
    let blocker = Connection::open(&destination_path).unwrap();
    blocker.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let backup = Backup::new(&source, &mut destination).unwrap();
    assert!(matches!(
        run_backup_steps(&backup, Instant::now(), one_step),
        Err(BackupQualificationError::StepBudgetExhausted {
            completed_steps: 1,
            last_step: Some(BackupStepObservation::Busy),
        })
    ));
    drop(backup);
    blocker.execute_batch("ROLLBACK").unwrap();
    drop(blocker);
    drop(destination);
    fs::remove_file(&destination_path).unwrap();

    source
        .execute_batch(
            "BEGIN IMMEDIATE; \
             INSERT INTO payload(value) VALUES (zeroblob(1));",
        )
        .unwrap();
    let mut destination = Connection::open(&destination_path).unwrap();
    let backup = Backup::new(&source, &mut destination).unwrap();
    let source_write = run_backup_steps(&backup, Instant::now(), one_step);
    assert!(
        matches!(
            source_write,
            Err(BackupQualificationError::StepBudgetExhausted {
                completed_steps: 1,
                last_step: Some(BackupStepObservation::Busy),
            })
        ),
        "unexpected source-write backup result: {source_write:?}"
    );
    drop(backup);
    drop(destination);
    source.execute_batch("ROLLBACK").unwrap();

    assert_eq!(
        observe_native_step(rusqlite::backup::StepResult::Locked).unwrap(),
        Some(BackupStepObservation::Locked)
    );
}

#[test]
fn exhausted_step_and_elapsed_budgets_never_leave_a_verified_candidate() {
    let source_root = support::private_tempdir();
    let destination_root = support::private_tempdir();
    let (source, identity, _) =
        support::initialize(&source_root, ControlStoreKind::PrincipalTrust, "bounded");
    let destination = support::location(
        &destination_root,
        ControlStoreKind::PrincipalTrust,
        "bounded",
    );
    let one_step = BackupQualificationBudget::new(1, 1, Duration::from_secs(10)).unwrap();
    assert!(matches!(
        qualify_inactive_store_copy(&source, &destination, &identity, one_step),
        Err(BackupQualificationError::StepBudgetExhausted { .. })
    ));
    assert!(!destination.path().exists());

    let expired = BackupQualificationBudget::new(1, 1, Duration::from_nanos(1)).unwrap();
    let elapsed = ensure_deadline(Instant::now() - Duration::from_secs(1), expired, 0, None);
    assert!(matches!(
        elapsed,
        Err(BackupQualificationError::AcceptanceDeadlineExceeded {
            completed_steps: 0,
            last_step: None,
        })
    ));
}

#[test]
fn occupied_symlink_and_sidecar_destinations_are_preserved() {
    let source_root = support::private_tempdir();
    let (source, identity, _) =
        support::initialize(&source_root, ControlStoreKind::PrincipalTrust, "occupied");

    let occupied_root = support::private_tempdir();
    let occupied = support::location(&occupied_root, ControlStoreKind::PrincipalTrust, "occupied");
    fs::write(occupied.path(), b"caller-owned-main").unwrap();
    assert!(matches!(
        qualify_inactive_store_copy(&source, &occupied, &identity, generous_budget()),
        Err(BackupQualificationError::Storage(
            ControlSqliteError::InitializationTargetExists(_)
        ))
    ));
    assert_eq!(fs::read(occupied.path()).unwrap(), b"caller-owned-main");

    let symlink_root = support::private_tempdir();
    let symlinked = support::location(&symlink_root, ControlStoreKind::PrincipalTrust, "occupied");
    symlink("caller-target", symlinked.path()).unwrap();
    assert!(matches!(
        qualify_inactive_store_copy(&source, &symlinked, &identity, generous_budget()),
        Err(BackupQualificationError::Storage(
            ControlSqliteError::InitializationTargetExists(_)
        ))
    ));
    assert_eq!(
        fs::read_link(symlinked.path()).unwrap(),
        std::path::Path::new("caller-target")
    );

    let sidecar_root = support::private_tempdir();
    let sidecar_location =
        support::location(&sidecar_root, ControlStoreKind::PrincipalTrust, "occupied");
    let sidecar = sqlite_sidecar(&sidecar_location.path(), "-wal");
    fs::write(&sidecar, b"caller-owned-sidecar").unwrap();
    assert!(matches!(
        qualify_inactive_store_copy(
            &source,
            &sidecar_location,
            &identity,
            generous_budget()
        ),
        Err(BackupQualificationError::Storage(
            ControlSqliteError::InitializationTargetExists(found)
        )) if found == sidecar
    ));
    assert_eq!(fs::read(&sidecar).unwrap(), b"caller-owned-sidecar");
}

#[test]
fn wrong_identity_and_source_aliases_fail_before_copying() {
    let source_root = support::private_tempdir();
    let destination_root = support::private_tempdir();
    let (source, identity, _) =
        support::initialize(&source_root, ControlStoreKind::PrincipalTrust, "identity");
    let destination = support::location(
        &destination_root,
        ControlStoreKind::PrincipalTrust,
        "identity",
    );
    let wrong = ControlStoreIdentity::new(
        identity.scope().clone(),
        ControlStoreIdV1::parse("another-store").unwrap(),
        identity.control_incarnation_id().clone(),
    );
    assert!(matches!(
        qualify_inactive_store_copy(&source, &destination, &wrong, generous_budget()),
        Err(BackupQualificationError::SourceClassification(
            StoreOpenClassification::Rejected(StoreOpenRejection::WrongLogicalStoreId)
        ))
    ));
    assert!(!destination.path().exists());

    assert!(matches!(
        qualify_inactive_store_copy(&source, &source, &identity, generous_budget()),
        Err(BackupQualificationError::DestinationAliasesSource)
    ));
    assert!(matches!(
        crate::store::classify_store(&source, &identity).unwrap(),
        StoreOpenClassification::ProfileUnselected { .. }
    ));
}

#[test]
fn budget_constructor_rejects_zero_and_native_page_overflow() {
    assert_eq!(
        BackupQualificationBudget::new(0, 1, Duration::from_secs(1)),
        None
    );
    assert_eq!(
        BackupQualificationBudget::new(1, 0, Duration::from_secs(1)),
        None
    );
    assert_eq!(BackupQualificationBudget::new(1, 1, Duration::ZERO), None);
    assert_eq!(
        BackupQualificationBudget::new(i32::MAX as u32 + 1, 1, Duration::from_secs(1)),
        None
    );
}
