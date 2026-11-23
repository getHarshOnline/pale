//! Exact read-back uses one database snapshot and never changes the store.

use super::*;

#[test]
fn readback_rejects_invalid_binding_and_transaction_head_without_writes() {
    for committed in [false, true] {
        for fault in [
            "DELETE FROM pale_control_store_header",
            "DELETE FROM pale_principal_trust_store_binding",
            "UPDATE pale_principal_trust_store_binding SET principal_id = 'other-principal'",
            "UPDATE pale_control_store_header SET logical_store_id = 'other-store'",
            "UPDATE pale_control_store_header SET control_incarnation_id = 'other-incarnation'",
            "UPDATE pale_control_store_header SET logical_transaction_sequence = 0",
            "UPDATE pale_control_store_header SET logical_transaction_sequence = -1",
        ] {
            let mut connection = initialize_store();
            let candidate = initial_candidate();
            if committed {
                commit(&mut connection, &candidate);
            }
            // Inject damaged stored state, then restore normal enforcement
            // before exercising the production reader.
            connection
                .pragma_update(None, "foreign_keys", false)
                .unwrap();
            connection
                .pragma_update(None, "ignore_check_constraints", true)
                .unwrap();
            connection
                .execute_batch(fault)
                .unwrap_or_else(|error| panic!("inject {fault} (committed={committed}): {error}"));
            connection
                .pragma_update(None, "ignore_check_constraints", false)
                .unwrap();
            connection
                .pragma_update(None, "foreign_keys", true)
                .unwrap();
            assert!(connection
                .pragma_query_value(None, "foreign_keys", |row| row.get::<_, bool>(0))
                .unwrap());
            assert!(!connection
                .pragma_query_value(None, "ignore_check_constraints", |row| row
                    .get::<_, bool>(0))
                .unwrap());
            let changes = connection.total_changes();
            let rows = transition_row_counts(&connection);
            let error = SqlitePrincipalProducerIncarnationCustody {
                connection: &mut connection,
            }
            .observe_exact(&candidate)
            .unwrap_err();
            assert_eq!(
                error.kind(),
                ControlStoreErrorKind::RecoveryRequired,
                "{fault}"
            );
            assert!(
                connection.is_autocommit(),
                "failed read-back releases its snapshot"
            );
            assert_eq!(connection.total_changes(), changes);
            assert_eq!(transition_row_counts(&connection), rows);
        }
    }
}

#[test]
fn readback_preserves_predecessor_successor_and_partial_dispositions() {
    for state in ["predecessor", "successor", "partial"] {
        let mut connection = initialize_store();
        let candidate = initial_candidate();
        let expected = match state {
            "successor" => {
                commit(&mut connection, &candidate);
                PrincipalProducerIncarnationDatabaseObservation::ExactSuccessor
            }
            "partial" => {
                insert_record(&connection, candidate.binding(), &candidate.records()[0]).unwrap();
                PrincipalProducerIncarnationDatabaseObservation::ConflictingOrPartial
            }
            _ => PrincipalProducerIncarnationDatabaseObservation::Predecessor,
        };
        let changes = connection.total_changes();
        let rows = transition_row_counts(&connection);
        assert_eq!(
            SqlitePrincipalProducerIncarnationCustody {
                connection: &mut connection
            }
            .observe_exact(&candidate)
            .unwrap(),
            expected
        );
        assert!(connection.is_autocommit());
        assert_eq!(connection.total_changes(), changes);
        assert_eq!(transition_row_counts(&connection), rows);
    }
}

#[test]
fn concurrent_commit_does_not_mix_predecessor_and_successor_reads() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("producer-readback.sqlite3");
    let mut reader = initialize_connection(Connection::open(&path).unwrap());
    reader.pragma_update(None, "journal_mode", "WAL").unwrap();
    assert_eq!(
        reader
            .pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    let mut writer = Connection::open(&path).unwrap();
    writer.pragma_update(None, "foreign_keys", true).unwrap();
    let candidate = initial_candidate();
    let changes = reader.total_changes();

    let snapshot = reader
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();
    assert_eq!(
        observe(&snapshot, &candidate).unwrap(),
        PrincipalProducerIncarnationDatabaseObservation::Predecessor
    );
    commit(&mut writer, &candidate);
    assert_eq!(
        observe(&snapshot, &candidate).unwrap(),
        PrincipalProducerIncarnationDatabaseObservation::Predecessor
    );
    snapshot.rollback().unwrap();
    assert_eq!(
        SqlitePrincipalProducerIncarnationCustody {
            connection: &mut reader
        }
        .observe_exact(&candidate)
        .unwrap(),
        PrincipalProducerIncarnationDatabaseObservation::ExactSuccessor
    );
    assert!(reader.is_autocommit());
    assert_eq!(reader.total_changes(), changes);
}

#[test]
fn concurrent_retirement_does_not_change_an_existing_successor_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory
        .path()
        .join("producer-retirement-readback.sqlite3");
    let mut reader = initialize_connection(Connection::open(&path).unwrap());
    reader.pragma_update(None, "journal_mode", "WAL").unwrap();
    assert_eq!(
        reader
            .pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    let candidate = initial_candidate();
    commit(&mut reader, &candidate);
    let mut writer = Connection::open(&path).unwrap();
    writer.pragma_update(None, "foreign_keys", true).unwrap();
    let changes = reader.total_changes();

    let snapshot = reader
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();
    assert_eq!(
        observe(&snapshot, &candidate).unwrap(),
        PrincipalProducerIncarnationDatabaseObservation::ExactSuccessor
    );
    commit(&mut writer, &retirement_candidate());
    assert_eq!(
        observe(&snapshot, &candidate).unwrap(),
        PrincipalProducerIncarnationDatabaseObservation::ExactSuccessor
    );
    snapshot.rollback().unwrap();
    assert_eq!(
        SqlitePrincipalProducerIncarnationCustody {
            connection: &mut reader
        }
        .observe_exact(&candidate)
        .unwrap(),
        PrincipalProducerIncarnationDatabaseObservation::ConflictingOrPartial
    );
    assert!(reader.is_autocommit());
    assert_eq!(reader.total_changes(), changes);
}

#[test]
fn saturated_transaction_head_does_not_require_a_new_transaction_for_readback() {
    for committed in [false, true] {
        let mut connection = initialize_store();
        let candidate = initial_candidate();
        if committed {
            commit(&mut connection, &candidate);
        }
        connection
            .execute(
                "UPDATE pale_control_store_header SET logical_transaction_sequence = 9007199254740991",
                [],
            )
            .unwrap();
        let changes = connection.total_changes();
        assert_eq!(
            SqlitePrincipalProducerIncarnationCustody {
                connection: &mut connection
            }
            .observe_exact(&candidate)
            .unwrap(),
            if committed {
                PrincipalProducerIncarnationDatabaseObservation::ExactSuccessor
            } else {
                PrincipalProducerIncarnationDatabaseObservation::Predecessor
            }
        );
        assert!(connection.is_autocommit());
        assert_eq!(connection.total_changes(), changes);
    }
}
