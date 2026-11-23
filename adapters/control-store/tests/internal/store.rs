//! Initial-v1 creation and fail-closed classification tests.

#[path = "../support/fixtures.rs"]
mod fixtures;

use super::*;
use crate::test_support as support;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, Barrier};

#[test]
fn classifies_missing_empty_and_unsafe_entries_without_opening_them() {
    let directory = support::private_tempdir();
    let identity = support::identity(ControlStoreKind::PrincipalTrust, "classify");
    let missing = support::location(&directory, ControlStoreKind::PrincipalTrust, "classify");
    assert_eq!(
        classify_store(&missing, &identity).unwrap(),
        StoreOpenClassification::Missing
    );
    fixtures::materialize(&missing.path(), fixtures::FixtureState::Empty);
    assert_eq!(
        classify_store(&missing, &identity).unwrap(),
        StoreOpenClassification::Empty
    );
    fs::remove_file(missing.path()).unwrap();
    fs::create_dir(missing.path()).unwrap();
    assert_eq!(
        classify_store(&missing, &identity).unwrap(),
        StoreOpenClassification::Rejected(StoreOpenRejection::UnsafeEntryKind)
    );
}

#[test]
fn initializes_both_store_kinds_without_claiming_runtime_admission() {
    for (kind, suffix) in [
        (ControlStoreKind::PrincipalTrust, "trust"),
        (ControlStoreKind::DeploymentControl, "deployment"),
    ] {
        let directory = support::private_tempdir();
        let (location, identity, initialized) = support::initialize(&directory, kind, suffix);
        assert_eq!(initialized.path(), location.path());
        assert_eq!(initialized.identity(), &identity);
        let connection = Connection::open(location.path()).unwrap();
        let coordinate = pale_ports::AuthorityStoreCoordinate::new(
            identity.logical_store_id().clone(),
            identity.control_incarnation_id().clone(),
        );
        assert_eq!(
            crate::store::transaction::read(&connection, &coordinate)
                .unwrap()
                .get(),
            1
        );
        assert_eq!(initialized.manifest_sha256().len(), 64);
        assert_eq!(initialized.compile_options_sha256().len(), 64);
        assert_eq!(
            fs::metadata(location.path()).unwrap().permissions().mode() & 0o077,
            0
        );
        assert!(matches!(
            classify_store(&location, &identity).unwrap(),
            StoreOpenClassification::ProfileUnselected { unresolved }
                if unresolved.contains(&ControlStorageProfileRequirement::SqliteCompileOptions)
                    && unresolved.contains(&ControlStorageProfileRequirement::SqliteSynchronous)
        ));
    }
}

#[test]
fn transaction_fence_is_store_bound_and_rolls_back_with_real_records() {
    use crate::store::transaction;
    use pale_core::records::{ResidentRecordRefV1, SchemaDigest, SchemaId};
    use pale_ports::{
        ControlRecord, ControlRecordBytes, ControlRecordCoordinate, ControlRecordDigest,
        ControlStoreErrorKind,
    };
    for kind in [
        ControlStoreKind::PrincipalTrust,
        ControlStoreKind::DeploymentControl,
    ] {
        let directory = support::private_tempdir();
        let (location, identity, _) = support::initialize(&directory, kind, "transaction-rollback");
        let coordinate = pale_ports::AuthorityStoreCoordinate::new(
            identity.logical_store_id().clone(),
            identity.control_incarnation_id().clone(),
        );
        // An opaque physical fixture record, not a signed authority claim.
        let record = ControlRecord::new(
            ControlRecordCoordinate::new(
                ResidentRecordRefV1::parse("transaction-member").unwrap(),
                ControlRecordDigest::parse("a".repeat(64)).unwrap(),
            ),
            SchemaId::parse("pale/common-request-envelope.v1").unwrap(),
            SchemaDigest::parse("b".repeat(64)).unwrap(),
            ControlRecordBytes::new(br#"{"physical":"member"}"#.to_vec()).unwrap(),
        );
        let mut connection = Connection::open(location.path()).unwrap();
        let prepared = transaction::prepare_snapshot(&mut connection, &coordinate, kind).unwrap();
        assert!(connection.is_autocommit());
        assert_eq!(prepared.expected().get(), 1);
        assert_eq!(prepared.successor().get(), 2);
        for commit in [false, true] {
            let write = connection
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .unwrap();
            transaction::advance(&write, &prepared).unwrap();
            match kind {
                ControlStoreKind::PrincipalTrust => {
                    let ControlStoreScope::Principal(scope) = identity.scope() else {
                        panic!("principal fixture scope")
                    };
                    let binding = pale_ports::PrincipalTrustStoreBinding::new(
                        identity.logical_store_id().clone(),
                        identity.control_incarnation_id().clone(),
                        scope.principal_id().clone(),
                    );
                    crate::principal_trust::insert_record(&write, &binding, &record).unwrap();
                }
                ControlStoreKind::DeploymentControl => {
                    crate::deployment::insert_record(&write, &record).unwrap()
                }
            }
            assert_eq!(
                transaction::record_stamp(&write, kind, record.coordinate()).unwrap(),
                Some(prepared.successor())
            );
            if commit {
                write.commit().unwrap();
            } else {
                drop(write);
            }
            drop(connection);
            connection = Connection::open(location.path()).unwrap();
            assert_eq!(
                transaction::read(&connection, &coordinate).unwrap().get(),
                if commit { 2 } else { 1 }
            );
            assert_eq!(
                transaction::record_stamp(&connection, kind, record.coordinate()).unwrap(),
                commit.then_some(prepared.successor())
            );
        }
        let write = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        assert_eq!(
            transaction::advance(&write, &prepared).unwrap_err().kind(),
            ControlStoreErrorKind::CandidateConflict
        );
        write.rollback().unwrap();
        assert_eq!(
            transaction::read(&connection, &coordinate).unwrap().get(),
            2
        );
        let wrong = pale_ports::AuthorityStoreCoordinate::new(
            coordinate.store_id().clone(),
            pale_core::records::ControlIncarnationIdV1::parse("different-incarnation").unwrap(),
        );
        assert_eq!(
            transaction::prepare_snapshot(&mut connection, &wrong, kind)
                .unwrap_err()
                .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        let other = match kind {
            ControlStoreKind::PrincipalTrust => ControlStoreKind::DeploymentControl,
            ControlStoreKind::DeploymentControl => ControlStoreKind::PrincipalTrust,
        };
        assert_eq!(
            transaction::prepare_snapshot(&mut connection, &coordinate, other)
                .unwrap_err()
                .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
    }
}

#[test]
fn saturated_transaction_head_is_valid_for_reads_but_cannot_prepare_new_mutation() {
    use crate::store::transaction;
    use pale_ports::ControlStoreErrorKind;
    let directory = support::private_tempdir();
    let (location, identity, _) = support::initialize(
        &directory,
        ControlStoreKind::DeploymentControl,
        "transaction-maximum",
    );
    let coordinate = pale_ports::AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    );
    let mut connection = Connection::open(location.path()).unwrap();
    // Deliberately position the physical fixture at the exact admitted integer limit.
    assert_eq!(connection.execute("UPDATE pale_control_store_header SET logical_transaction_sequence=9007199254740991 WHERE singleton=1", []).unwrap(), 1);
    assert_eq!(
        transaction::read(&connection, &coordinate).unwrap().get(),
        9_007_199_254_740_991
    );
    let changes = connection.total_changes();
    assert_eq!(
        transaction::prepare_snapshot(
            &mut connection,
            &coordinate,
            ControlStoreKind::DeploymentControl
        )
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::TransactionExhausted
    );
    assert_eq!(connection.total_changes(), changes);
    assert!(connection.is_autocommit());
}

#[test]
fn malformed_transaction_head_is_recovery_not_initialization_or_exhaustion() {
    use crate::store::transaction;
    use pale_ports::ControlStoreErrorKind;
    for invalid in ["0", "-1", "9007199254740992"] {
        let directory = support::private_tempdir();
        let (location, identity, _) = support::initialize(
            &directory,
            ControlStoreKind::DeploymentControl,
            "transaction-corrupt",
        );
        let coordinate = pale_ports::AuthorityStoreCoordinate::new(
            identity.logical_store_id().clone(),
            identity.control_incarnation_id().clone(),
        );
        let connection = Connection::open(location.path()).unwrap();
        // Scoped fault injection bypasses only the fixture CHECK, then restores it.
        connection
            .pragma_update(None, "ignore_check_constraints", true)
            .unwrap();
        assert_eq!(connection.execute(&format!("UPDATE pale_control_store_header SET logical_transaction_sequence={invalid} WHERE singleton=1"), []).unwrap(), 1);
        connection
            .pragma_update(None, "ignore_check_constraints", false)
            .unwrap();
        let changes = connection.total_changes();
        assert_eq!(
            transaction::read(&connection, &coordinate)
                .unwrap_err()
                .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        assert_eq!(
            transaction::prepare(&connection, &coordinate)
                .unwrap_err()
                .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        assert_eq!(connection.total_changes(), changes);
    }
    // A deliberately malformed non-STRICT decoder fixture, not an initialized
    // store: actual initial DDL rejects BLOB values even with CHECKs disabled.
    let malformed = Connection::open_in_memory().unwrap();
    malformed.execute_batch("CREATE TABLE pale_control_store_header (singleton,logical_store_id,control_incarnation_id,logical_transaction_sequence)").unwrap();
    assert_eq!(malformed.execute("INSERT INTO pale_control_store_header VALUES(1,'malformed','incarnation',zeroblob(1048576))", []).unwrap(), 1);
    let coordinate = pale_ports::AuthorityStoreCoordinate::new(
        pale_core::records::ControlStoreIdV1::parse("malformed").unwrap(),
        pale_core::records::ControlIncarnationIdV1::parse("incarnation").unwrap(),
    );
    assert_eq!(
        transaction::read(&malformed, &coordinate)
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn failed_initialization_cleans_only_its_owned_main_file() {
    let directory = support::private_tempdir();
    let location = support::location(&directory, ControlStoreKind::PrincipalTrust, "failed");
    let identity = support::identity(ControlStoreKind::PrincipalTrust, "failed");
    let sources = crate::verify_embedded_storage_sources().unwrap();
    let error = initialize_unadmitted_store_with_sql(
        &location,
        identity.clone(),
        sources,
        "THIS IS NOT VALID SQLITE DDL;",
    )
    .unwrap_err();
    assert!(matches!(error, ControlSqliteError::Sqlite { .. }));
    assert!(!location.path().exists());
    assert_eq!(
        classify_store(&location, &identity).unwrap(),
        StoreOpenClassification::Missing
    );
}

#[test]
fn reserves_initialization_atomically_and_preserves_existing_sidecars() {
    let directory = support::private_tempdir();
    let location = support::location(&directory, ControlStoreKind::PrincipalTrust, "contended");
    let identity = support::identity(ControlStoreKind::PrincipalTrust, "contended");
    let barrier = Arc::new(Barrier::new(3));
    let mut attempts = Vec::new();
    for _ in 0..2 {
        let location = location.clone();
        let barrier = Arc::clone(&barrier);
        let identity = identity.clone();
        attempts.push(std::thread::spawn(move || {
            barrier.wait();
            initialize_unadmitted_store(&location, identity)
        }));
    }
    barrier.wait();
    let outcomes = attempts
        .into_iter()
        .map(|attempt| attempt.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(
                outcome,
                Err(ControlSqliteError::InitializationTargetExists(_))
            ))
            .count(),
        1
    );

    let blocked = support::location(&directory, ControlStoreKind::PrincipalTrust, "blocked");
    let wal = crate::candidate::sqlite_sidecar(&blocked.path(), "-wal");
    fs::write(&wal, b"caller-owned").unwrap();
    assert!(matches!(
        initialize_unadmitted_store(
            &blocked,
            support::identity(ControlStoreKind::PrincipalTrust, "blocked")
        ),
        Err(ControlSqliteError::InitializationTargetExists(found)) if found == wal
    ));
    assert_eq!(fs::read(&wal).unwrap(), b"caller-owned");
}

#[test]
fn rejects_identity_header_and_missing_migration_receipt() {
    let directory = support::private_tempdir();
    let (location, identity, _) =
        support::initialize(&directory, ControlStoreKind::DeploymentControl, "identity");
    assert!(matches!(
        classify_store(
            &location,
            &support::identity(ControlStoreKind::PrincipalTrust, "identity")
        ),
        Err(ControlSqliteError::StoreLocationScopeMismatch)
    ));
    let wrong_store = ControlStoreIdentity::new(
        identity.scope().clone(),
        pale_core::records::ControlStoreIdV1::parse("another-logical-store").unwrap(),
        identity.control_incarnation_id().clone(),
    );
    assert_eq!(
        classify_store(&location, &wrong_store).unwrap(),
        StoreOpenClassification::Rejected(StoreOpenRejection::WrongLogicalStoreId)
    );
    let wrong_incarnation = ControlStoreIdentity::new(
        identity.scope().clone(),
        identity.logical_store_id().clone(),
        pale_core::records::ControlIncarnationIdV1::parse("another-incarnation").unwrap(),
    );
    assert_eq!(
        classify_store(&location, &wrong_incarnation).unwrap(),
        StoreOpenClassification::Rejected(StoreOpenRejection::WrongControlIncarnation)
    );
    fixtures::materialize(
        &location.path(),
        fixtures::FixtureState::IncompleteMigration,
    );
    assert_eq!(
        classify_store(&location, &identity).unwrap(),
        StoreOpenClassification::Rejected(StoreOpenRejection::SourceIdentityMismatch)
    );
}

#[test]
fn rejects_kind_specific_raw_scope_binding_substitution() {
    let principal_directory = support::private_tempdir();
    let (principal_location, principal_identity, _) = support::initialize(
        &principal_directory,
        ControlStoreKind::PrincipalTrust,
        "principal-binding",
    );
    Connection::open(principal_location.path())
        .unwrap()
        .execute(
            "UPDATE pale_principal_trust_store_binding SET principal_id = 'substituted'",
            [],
        )
        .unwrap();
    assert_eq!(
        classify_store(&principal_location, &principal_identity).unwrap(),
        StoreOpenClassification::Rejected(StoreOpenRejection::WrongPrincipalScope)
    );

    let deployment_directory = support::private_tempdir();
    let (deployment_location, deployment_identity, _) = support::initialize(
        &deployment_directory,
        ControlStoreKind::DeploymentControl,
        "deployment-binding",
    );
    for (column, value) in [
        ("workspace_id", "substituted-workspace".to_owned()),
        ("deployment_binding_ref", "substituted-binding".to_owned()),
        ("deployment_binding_sha256", "f".repeat(64)),
    ] {
        let connection = Connection::open(deployment_location.path()).unwrap();
        let original: String = connection
            .query_row(
                &format!(
                    "SELECT {column} FROM pale_deployment_control_store_binding WHERE singleton = 1"
                ),
                [],
                |row| row.get(0),
            )
            .unwrap();
        connection
            .execute(
                &format!(
                    "UPDATE pale_deployment_control_store_binding SET {column} = ?1 WHERE singleton = 1"
                ),
                [&value],
            )
            .unwrap();
        assert_eq!(
            classify_store(&deployment_location, &deployment_identity).unwrap(),
            StoreOpenClassification::Rejected(StoreOpenRejection::WrongDeploymentScope),
            "raw deployment coordinate {column} must be causal"
        );
        connection
            .execute(
                &format!(
                    "UPDATE pale_deployment_control_store_binding SET {column} = ?1 WHERE singleton = 1"
                ),
                [original],
            )
            .unwrap();
    }
}

#[test]
fn resultant_schema_drift_is_rejected_even_when_header_hash_is_untouched() {
    for (suffix, ddl) in [
        (
            "extra-table",
            "CREATE TABLE injected (id INTEGER PRIMARY KEY) STRICT;",
        ),
        (
            "extra-index",
            "CREATE INDEX injected_header_kind ON pale_control_store_header(store_kind);",
        ),
        (
            "extra-view",
            "CREATE VIEW injected_header AS SELECT store_kind FROM pale_control_store_header;",
        ),
        (
            "extra-trigger",
            "CREATE TRIGGER injected_guard BEFORE DELETE ON pale_control_store_header BEGIN SELECT RAISE(ABORT, 'no'); END;",
        ),
    ] {
        let directory = support::private_tempdir();
        let (location, identity, _) =
            support::initialize(&directory, ControlStoreKind::PrincipalTrust, suffix);
        Connection::open(location.path())
            .unwrap()
            .execute_batch(ddl)
            .unwrap();
        assert_eq!(
            classify_store(&location, &identity).unwrap(),
            StoreOpenClassification::Rejected(StoreOpenRejection::SourceIdentityMismatch),
            "persistent object drift must be causal for {suffix}"
        );
    }
}

#[test]
fn data_changes_do_not_change_resultant_schema_identity() {
    let directory = support::private_tempdir();
    let (location, identity, _) =
        support::initialize(&directory, ControlStoreKind::PrincipalTrust, "data");
    Connection::open(location.path())
        .unwrap()
        .execute(
            "UPDATE pale_control_store_profile_observation SET sqlite_version = sqlite_version",
            [],
        )
        .unwrap();
    assert!(matches!(
        classify_store(&location, &identity).unwrap(),
        StoreOpenClassification::ProfileUnselected { .. }
    ));
}

#[test]
fn corrupt_bytes_and_non_private_files_have_distinct_rejections() {
    let directory = support::private_tempdir();
    let identity = support::identity(ControlStoreKind::PrincipalTrust, "failure-kind");
    let location = support::location(&directory, ControlStoreKind::PrincipalTrust, "failure-kind");
    fixtures::materialize(&location.path(), fixtures::FixtureState::Corrupt);
    assert_eq!(
        classify_store(&location, &identity).unwrap(),
        StoreOpenClassification::Rejected(StoreOpenRejection::CorruptOrPartial)
    );

    fs::set_permissions(location.path(), fs::Permissions::from_mode(0o000)).unwrap();
    let result = classify_store(&location, &identity);
    fs::set_permissions(location.path(), fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        result.unwrap(),
        StoreOpenClassification::Rejected(StoreOpenRejection::UnsafeEntryKind)
    );
}

#[test]
fn database_lock_contention_remains_an_operational_error() {
    let directory = support::private_tempdir();
    let (location, identity, _) =
        support::initialize(&directory, ControlStoreKind::PrincipalTrust, "busy");
    let blocker = Connection::open(location.path()).unwrap();
    let journal: String = blocker
        .query_row("PRAGMA journal_mode = DELETE", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal, "delete");
    blocker.execute_batch("BEGIN EXCLUSIVE;").unwrap();
    let result = classify_store(&location, &identity);
    blocker.execute_batch("ROLLBACK;").unwrap();
    assert!(matches!(result, Err(ControlSqliteError::Sqlite { .. })));
}

#[test]
fn newer_partial_source_mismatch_and_non_wal_remain_distinct() {
    let directory = support::private_tempdir();
    let (location, identity, _) =
        support::initialize(&directory, ControlStoreKind::PrincipalTrust, "matrix");
    fixtures::materialize(&location.path(), fixtures::FixtureState::NewerVersion);
    assert_eq!(
        classify_store(&location, &identity).unwrap(),
        StoreOpenClassification::Rejected(StoreOpenRejection::NewerVersion {
            observed: 2,
            supported: 1,
        })
    );

    let partial = support::location(&directory, ControlStoreKind::PrincipalTrust, "partial");
    let partial_identity = support::identity(ControlStoreKind::PrincipalTrust, "partial");
    fixtures::materialize(&partial.path(), fixtures::FixtureState::Partial);
    assert_eq!(
        classify_store(&partial, &partial_identity).unwrap(),
        StoreOpenClassification::Rejected(StoreOpenRejection::SourceIdentityMismatch)
    );

    let source_directory = support::private_tempdir();
    let (source_location, source_identity, _) = support::initialize(
        &source_directory,
        ControlStoreKind::PrincipalTrust,
        "source-mismatch",
    );
    fixtures::materialize(
        &source_location.path(),
        fixtures::FixtureState::SourceManifestMismatch,
    );
    assert_eq!(
        classify_store(&source_location, &source_identity).unwrap(),
        StoreOpenClassification::Rejected(StoreOpenRejection::SourceIdentityMismatch)
    );

    let wal_directory = support::private_tempdir();
    let (wal_location, wal_identity, _) =
        support::initialize(&wal_directory, ControlStoreKind::PrincipalTrust, "non-wal");
    fixtures::materialize(&wal_location.path(), fixtures::FixtureState::NonWal);
    assert_eq!(
        classify_store(&wal_location, &wal_identity).unwrap(),
        StoreOpenClassification::Rejected(StoreOpenRejection::StructuralProfileMismatch)
    );
}

#[path = "store_collection_upgrade.rs"]
mod collection_upgrade;
