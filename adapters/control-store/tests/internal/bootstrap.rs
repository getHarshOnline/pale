//! Production `SQLite` principal-bootstrap custody tests.

use pale_core::records::{
    ArtifactDigest, CheckpointProfileDigest, ControlCheckpointDomainIdV1, ControlDomainEpochV1,
    ControlDomainSequenceV1, ControlIncarnationIdV1, ControlStoreIdV1,
    ControlTransactionSequenceV1, PrincipalBootstrapIdV1, PrincipalTrustCheckpointEpochV1,
    ResidentPrincipalIdV1, ResidentRecordRefV1, SchemaDigest, SchemaId, UnixTimeV1,
};
use pale_ports::{
    AccumulatorCheckpointBinding, AccumulatorCheckpointPhase, AccumulatorMutation,
    AccumulatorPosition, AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecord,
    ControlRecordBytes, ControlRecordCoordinate, ControlRecordDigest, ControlStoreErrorKind,
    ControlTransactionPreparation, OperationAccumulatorBatch, PreparedPrincipalBootstrapAtom,
    PrincipalBootstrapCommitDisposition, PrincipalBootstrapControlCheckpoint,
    PrincipalBootstrapCustodyPort, PrincipalBootstrapDatabaseObservation,
    PrincipalBootstrapFixedRecords, PrincipalBootstrapHead, PrincipalTrustCheckpointHead,
    PrincipalTrustStoreBinding, StateAccumulatorBatch, UnresolvedCustodyAccumulatorBatch,
};
use rusqlite::{params, Connection};

use crate::bootstrap::SqlitePrincipalBootstrapCustody;

#[path = "bootstrap/accumulator.rs"]
mod accumulator;

#[path = "bootstrap/finalization.rs"]
mod finalization;

const MIGRATION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/principal-trust/0001_initial_v1.sql"
));

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record reference"),
        ControlRecordDigest::parse(sha(byte)).expect("record digest"),
    )
}

fn record(coordinate: ControlRecordCoordinate, schema: &str, content: &str) -> ControlRecord {
    ControlRecord::new(
        coordinate,
        SchemaId::parse(schema).expect("schema id"),
        SchemaDigest::parse(sha('a')).expect("schema digest"),
        ControlRecordBytes::new(content.as_bytes().to_vec()).expect("record bytes"),
    )
}

struct CandidateParts {
    binding: PrincipalTrustStoreBinding,
    records: Vec<ControlRecord>,
    checkpoints: Vec<PrincipalBootstrapControlCheckpoint>,
    head: PrincipalBootstrapHead,
}

fn candidate_parts(bootstrap_id: &str, receipt_content: &str) -> CandidateParts {
    let coordinates = [
        coordinate("principal-record-1", '1'),
        coordinate("recovery-policy-1", '2'),
        coordinate("admin-authority-1", '3'),
        coordinate("producer-authority-1", '4'),
        coordinate("checkpoint-authority-1", '5'),
        coordinate("trust-checkpoint-1", '6'),
        coordinate("bootstrap-receipt-1", '9'),
        coordinate("control-checkpoint-1", '7'),
        coordinate("control-checkpoint-2", '8'),
    ];
    let records = vec![
        record(
            coordinates[0].clone(),
            "pale/principal-record.v1",
            r#"{"record":"principal"}"#,
        ),
        record(
            coordinates[1].clone(),
            "pale/principal-recovery-policy.v1",
            r#"{"record":"recovery"}"#,
        ),
        record(
            coordinates[2].clone(),
            "pale/local-admin-authority.v1",
            r#"{"record":"admin"}"#,
        ),
        record(
            coordinates[3].clone(),
            "pale/producer-incarnation-authority.v1",
            r#"{"record":"producer"}"#,
        ),
        record(
            coordinates[4].clone(),
            "pale/checkpoint-signing-authority.v1",
            r#"{"record":"checkpoint-authority"}"#,
        ),
        record(
            coordinates[5].clone(),
            "pale/principal-trust-checkpoint.v1",
            r#"{"record":"trust"}"#,
        ),
        record(
            coordinates[7].clone(),
            "pale/control-state-checkpoint.v1",
            r#"{"record":"control-1"}"#,
        ),
        record(
            coordinates[8].clone(),
            "pale/control-state-checkpoint.v1",
            r#"{"record":"control-2"}"#,
        ),
        record(
            coordinates[6].clone(),
            "pale/principal-bootstrap-receipt.v1",
            receipt_content,
        ),
    ];
    let checkpoints = vec![
        PrincipalBootstrapControlCheckpoint::new(
            1,
            ControlCheckpointDomainIdV1::parse("admin-authority").unwrap(),
            ControlDomainEpochV1::new(1).unwrap(),
            ControlDomainSequenceV1::new(1).unwrap(),
            pale_core::records::ControlTransactionSequenceV1::new(2).unwrap(),
            coordinates[7].clone(),
        )
        .unwrap(),
        PrincipalBootstrapControlCheckpoint::new(
            2,
            ControlCheckpointDomainIdV1::parse("principal-head").unwrap(),
            ControlDomainEpochV1::new(1).unwrap(),
            ControlDomainSequenceV1::new(1).unwrap(),
            pale_core::records::ControlTransactionSequenceV1::new(2).unwrap(),
            coordinates[8].clone(),
        )
        .unwrap(),
    ];
    let head = PrincipalBootstrapHead::new(
        PrincipalBootstrapIdV1::parse(bootstrap_id).unwrap(),
        PrincipalBootstrapFixedRecords::new(
            coordinates[0].clone(),
            coordinates[1].clone(),
            coordinates[2].clone(),
            coordinates[3].clone(),
            coordinates[4].clone(),
            coordinates[6].clone(),
        ),
        PrincipalTrustCheckpointHead::new(
            ResidentPrincipalIdV1::parse("principal-1").unwrap(),
            PrincipalTrustCheckpointEpochV1::new(1).unwrap(),
            coordinates[5].clone(),
        ),
        2,
        UnixTimeV1::new(50).unwrap(),
    );
    CandidateParts {
        binding: PrincipalTrustStoreBinding::new(
            ControlStoreIdV1::parse("trust-store-1").unwrap(),
            ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap(),
            ResidentPrincipalIdV1::parse("principal-1").unwrap(),
        ),
        records,
        checkpoints,
        head,
    }
}

fn synthetic_accumulator_binding(
    binding: &PrincipalTrustStoreBinding,
    checkpoints: &[PrincipalBootstrapControlCheckpoint],
    index: usize,
) -> AccumulatorCheckpointBinding {
    let checkpoint = &checkpoints[index];
    let store = AuthorityStoreCoordinate::new(
        binding.trust_store_id().clone(),
        binding.trust_store_incarnation_id().clone(),
    );
    let transaction = ControlTransactionPreparation::new(
        store.clone(),
        ControlTransactionSequenceV1::new(1).unwrap(),
    )
    .unwrap();
    AccumulatorCheckpointBinding::new(
        store,
        ResidentRecordRefV1::parse("synthetic-profile").unwrap(),
        CheckpointProfileDigest::parse("d".repeat(64)).unwrap(),
        AccumulatorCheckpointPhase::Initial,
        ControlCheckpointHead::new(
            checkpoint.checkpoint_domain_id().clone(),
            checkpoint.domain_epoch(),
            checkpoint.domain_sequence(),
            checkpoint.committed_transaction_sequence(),
            checkpoint.coordinate().clone(),
        ),
        transaction,
    )
    .unwrap()
}

fn synthetic_accumulator(
    binding: &PrincipalTrustStoreBinding,
    checkpoints: &[PrincipalBootstrapControlCheckpoint],
    index: usize,
    leaf: u8,
) -> OperationAccumulatorBatch {
    let zero = ArtifactDigest::parse("0".repeat(64)).unwrap();
    OperationAccumulatorBatch::new(
        synthetic_accumulator_binding(binding, checkpoints, index),
        StateAccumulatorBatch::new(
            zero.clone(),
            ArtifactDigest::parse(
                std::iter::repeat_n(char::from(b'a' + leaf), 64).collect::<String>(),
            )
            .unwrap(),
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([leaf; 32]),
                None,
                Some(vec![leaf; 32]),
                32,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        UnresolvedCustodyAccumulatorBatch::new(zero.clone(), zero, vec![], 1).unwrap(),
    )
    .unwrap()
}

fn synthetic_empty_accumulator(
    binding: &PrincipalTrustStoreBinding,
    checkpoints: &[PrincipalBootstrapControlCheckpoint],
    index: usize,
) -> OperationAccumulatorBatch {
    let zero = ArtifactDigest::parse("0".repeat(64)).unwrap();
    OperationAccumulatorBatch::new(
        synthetic_accumulator_binding(binding, checkpoints, index),
        StateAccumulatorBatch::new(zero.clone(), zero.clone(), vec![], 1).unwrap(),
        UnresolvedCustodyAccumulatorBatch::new(zero.clone(), zero, vec![], 1).unwrap(),
    )
    .unwrap()
}

fn candidate(bootstrap_id: &str, receipt_content: &str) -> PreparedPrincipalBootstrapAtom {
    let parts = candidate_parts(bootstrap_id, receipt_content);
    let accumulators = (0..parts.checkpoints.len())
        .map(|index| {
            synthetic_accumulator(
                &parts.binding,
                &parts.checkpoints,
                index,
                u8::try_from(index + 1).unwrap(),
            )
        })
        .collect();
    PreparedPrincipalBootstrapAtom::new(
        parts.binding,
        parts.records,
        parts.checkpoints,
        parts.head,
        accumulators,
    )
    .unwrap()
}

fn initialize_store(connection: Connection) -> Connection {
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    connection.execute_batch(MIGRATION).unwrap();
    connection
        .execute(
            "INSERT INTO pale_control_store_header \
             (singleton, store_kind, logical_store_id, control_incarnation_id, store_schema_id, \
              store_schema_sha256, store_schema_version, storage_manifest_sha256, migration_head_sha256, logical_transaction_sequence) \
             VALUES (1, 'principal_trust', 'trust-store-1', 'trust-incarnation-1', \
                     'pale/control-store/principal-trust.v1', ?1, 1, ?2, ?3, 1)",
            params![sha('a'), sha('b'), sha('c')],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO pale_principal_trust_store_binding (singleton, principal_id) \
             VALUES (1, 'principal-1')",
            [],
        )
        .unwrap();
    connection
}

fn in_memory_store() -> Connection {
    initialize_store(Connection::open_in_memory().unwrap())
}

fn row_counts(connection: &Connection) -> (i64, i64, i64, i64, i64) {
    connection
        .query_row(
            "SELECT \
             (SELECT COUNT(*) FROM pale_principal_trust_record), \
             (SELECT COUNT(*) FROM pale_principal_trust_bootstrap_head), \
             (SELECT COUNT(*) FROM pale_principal_trust_checkpoint_head), \
             (SELECT COUNT(*) FROM pale_principal_bootstrap_control_checkpoint), \
             (SELECT COUNT(*) FROM pale_principal_control_checkpoint_head)",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap()
}

#[test]
fn empty_commit_exact_replay_and_read_back_preserve_one_atom() {
    let mut connection = in_memory_store();
    let candidate = candidate("bootstrap-1", r#"{"record":"receipt"}"#);
    let mut store = SqlitePrincipalBootstrapCustody {
        connection: &mut connection,
    };
    assert_eq!(
        store.compare_and_commit(&candidate).unwrap(),
        PrincipalBootstrapCommitDisposition::New
    );
    assert_eq!(
        store.observe_exact(&candidate).unwrap(),
        PrincipalBootstrapDatabaseObservation::Exact
    );
    assert_eq!(
        store.compare_and_commit(&candidate).unwrap(),
        PrincipalBootstrapCommitDisposition::ExactReplay
    );
    assert_eq!(row_counts(&connection), (9, 1, 1, 2, 2));
    let trust_head = candidate.head().trust_checkpoint();
    let retained: (String, i64, String, String) = connection
        .query_row(
            "SELECT principal_id, checkpoint_epoch, trust_checkpoint_ref, \
             trust_checkpoint_sha256 FROM pale_principal_trust_checkpoint_head \
             WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(retained.0, trust_head.principal_id().as_str());
    assert_eq!(retained.1, trust_head.checkpoint_epoch().get() as i64);
    assert_eq!(retained.2, trust_head.coordinate().record_ref().as_str());
    assert_eq!(retained.3, trust_head.coordinate().record_sha256().as_str());
}

#[test]
fn bootstrap_domains_share_one_stamp_and_exact_replay_needs_no_successor() {
    let mut connection = in_memory_store();
    let candidate = candidate("bootstrap-clock", r#"{"record":"receipt"}"#);
    {
        let mut custody = SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        };
        custody.compare_and_commit(&candidate).unwrap();
    }
    assert_eq!(connection.query_row("SELECT logical_transaction_sequence FROM pale_control_store_header WHERE singleton=1", [], |row| row.get::<_, i64>(0)).unwrap(), 2);
    for checkpoint in candidate.control_checkpoints() {
        assert_eq!(checkpoint.committed_transaction_sequence().get(), 2);
    }
    assert_eq!(connection.query_row("SELECT COUNT(*) FROM pale_principal_trust_record WHERE inserted_transaction_sequence=2", [], |row| row.get::<_, i64>(0)).unwrap(), 9);
    assert_eq!(connection.execute("UPDATE pale_control_store_header SET logical_transaction_sequence=9007199254740991 WHERE singleton=1", []).unwrap(), 1);
    let changes = connection.total_changes();
    let mut custody = SqlitePrincipalBootstrapCustody {
        connection: &mut connection,
    };
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap(),
        PrincipalBootstrapCommitDisposition::ExactReplay
    );
    assert_eq!(custody.connection.total_changes(), changes);
}

#[test]
fn byte_identical_bootstrap_with_unlike_member_stamp_requires_recovery() {
    let mut connection = in_memory_store();
    let candidate = candidate("bootstrap-clock-corrupt", r#"{"record":"receipt"}"#);
    SqlitePrincipalBootstrapCustody {
        connection: &mut connection,
    }
    .compare_and_commit(&candidate)
    .unwrap();
    let record = &candidate.records()[0];
    assert_eq!(connection.execute("UPDATE pale_principal_trust_record SET inserted_transaction_sequence=1 WHERE record_ref=?1 AND record_sha256=?2",
        params![record.coordinate().record_ref().as_str(), record.coordinate().record_sha256().as_str()]).unwrap(), 1);
    let changes = connection.total_changes();
    assert_eq!(
        SqlitePrincipalBootstrapCustody {
            connection: &mut connection
        }
        .compare_and_commit(&candidate)
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(connection.total_changes(), changes);
}

#[test]
fn contradictory_bytes_and_partial_rows_require_recovery() {
    let original = candidate("bootstrap-1", r#"{"record":"receipt"}"#);
    let changed = candidate("bootstrap-1", r#"{"record":"changed"}"#);
    let mut connection = in_memory_store();
    let mut store = SqlitePrincipalBootstrapCustody {
        connection: &mut connection,
    };
    store.compare_and_commit(&original).unwrap();
    // The fixture retains the same receipt reference, digest and closing head.
    // Different bytes under that identity are not another complete candidate.
    assert_eq!(
        store.compare_and_commit(&changed).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    let mut partial = in_memory_store();
    partial
        .execute(
            "INSERT INTO pale_principal_trust_record \
         (record_ref, record_sha256, schema_id, schema_sha256, principal_id, canonical_record_jcs, inserted_transaction_sequence) \
         VALUES ('orphan', ?1, 'pale/test.v1', ?2, 'principal-1', X'7b7d', 1)",
            params![sha('1'), sha('2')],
        )
        .unwrap();
    let mut partial_store = SqlitePrincipalBootstrapCustody {
        connection: &mut partial,
    };
    assert_eq!(
        partial_store
            .compare_and_commit(&original)
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        partial_store.observe_exact(&original).unwrap(),
        PrincipalBootstrapDatabaseObservation::ConflictingOrPartial
    );

    let mut missing_trust_head = in_memory_store();
    {
        let mut store = SqlitePrincipalBootstrapCustody {
            connection: &mut missing_trust_head,
        };
        store.compare_and_commit(&original).unwrap();
    }
    missing_trust_head
        .execute("DELETE FROM pale_principal_trust_checkpoint_head", [])
        .unwrap();
    let mut missing_head_store = SqlitePrincipalBootstrapCustody {
        connection: &mut missing_trust_head,
    };
    assert_eq!(
        missing_head_store.observe_exact(&original).unwrap(),
        PrincipalBootstrapDatabaseObservation::ConflictingOrPartial
    );
}

#[test]
fn committed_atom_survives_reopen_and_replays_without_writes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("principal-trust.sqlite3");
    let candidate = candidate("bootstrap-1", r#"{"record":"receipt"}"#);
    {
        let mut connection = initialize_store(Connection::open(&path).unwrap());
        let mut store = SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        };
        assert_eq!(
            store.compare_and_commit(&candidate).unwrap(),
            PrincipalBootstrapCommitDisposition::New
        );
    }
    let mut reopened = Connection::open(&path).unwrap();
    reopened.pragma_update(None, "foreign_keys", true).unwrap();
    let changes_before = reopened.total_changes();
    let mut store = SqlitePrincipalBootstrapCustody {
        connection: &mut reopened,
    };
    assert_eq!(
        store.observe_exact(&candidate).unwrap(),
        PrincipalBootstrapDatabaseObservation::Exact
    );
    assert_eq!(
        store.compare_and_commit(&candidate).unwrap(),
        PrincipalBootstrapCommitDisposition::ExactReplay
    );
    assert_eq!(reopened.total_changes(), changes_before);
}

#[test]
fn each_injected_insert_failure_rolls_back_to_empty() {
    let candidate = candidate("bootstrap-1", r#"{"record":"receipt"}"#);
    let record_triggers = candidate.records().iter().map(|record| {
        format!(
            "CREATE TEMP TRIGGER reject_boundary BEFORE INSERT ON \
         pale_principal_trust_record WHEN NEW.record_ref = '{}' \
         BEGIN SELECT RAISE(ABORT, 'injected record failure'); END;",
            record.coordinate().record_ref().as_str()
        )
    });
    let checkpoint_triggers = candidate.control_checkpoints().iter().map(|checkpoint| {
        format!(
            "CREATE TEMP TRIGGER reject_boundary BEFORE INSERT ON \
         pale_principal_bootstrap_control_checkpoint WHEN NEW.checkpoint_ordinal = {} \
         BEGIN SELECT RAISE(ABORT, 'injected checkpoint failure'); END;",
            checkpoint.ordinal()
        )
    });
    let checkpoint_head_triggers = candidate.control_checkpoints().iter().map(|checkpoint| {
        format!(
            "CREATE TEMP TRIGGER reject_boundary BEFORE INSERT ON \
         pale_principal_control_checkpoint_head WHEN NEW.checkpoint_domain_id = '{}' \
         BEGIN SELECT RAISE(ABORT, 'injected checkpoint-head failure'); END;",
            checkpoint.checkpoint_domain_id().as_str()
        )
    });
    let trust_head_trigger = std::iter::once(
        "CREATE TEMP TRIGGER reject_boundary BEFORE INSERT ON \
         pale_principal_trust_checkpoint_head \
         BEGIN SELECT RAISE(ABORT, 'injected trust-head failure'); END;"
            .to_owned(),
    );
    let head_trigger = std::iter::once(
        "CREATE TEMP TRIGGER reject_boundary BEFORE INSERT ON \
         pale_principal_trust_bootstrap_head \
         BEGIN SELECT RAISE(ABORT, 'injected head failure'); END;"
            .to_owned(),
    );
    for trigger in record_triggers
        .chain(checkpoint_triggers)
        .chain(checkpoint_head_triggers)
        .chain(trust_head_trigger)
        .chain(head_trigger)
    {
        let mut connection = in_memory_store();
        connection.execute_batch(&trigger).unwrap();
        let mut store = SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        };
        assert_eq!(
            store.compare_and_commit(&candidate).unwrap_err().kind(),
            ControlStoreErrorKind::Storage
        );
        assert_eq!(row_counts(&connection), (0, 0, 0, 0, 0));
    }
}

#[test]
fn store_binding_mismatch_requires_recovery_without_writes() {
    let candidate = candidate("bootstrap-1", r#"{"record":"receipt"}"#);
    let mut connection = in_memory_store();
    connection
        .execute(
            "UPDATE pale_control_store_header SET logical_store_id = 'other-store'",
            [],
        )
        .unwrap();
    let mut store = SqlitePrincipalBootstrapCustody {
        connection: &mut connection,
    };
    assert_eq!(
        store.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(row_counts(&connection), (0, 0, 0, 0, 0));
}

#[test]
fn an_empty_unrelated_store_cannot_exclude_an_ambiguous_commit() {
    let candidate = candidate("bootstrap-uncertain", r#"{"record":"receipt"}"#);
    for change in [
        "UPDATE pale_control_store_header SET logical_store_id = 'other-store'",
        "UPDATE pale_control_store_header SET control_incarnation_id = 'other-incarnation'",
        "UPDATE pale_principal_trust_store_binding SET principal_id = 'other-principal'",
        "DELETE FROM pale_principal_trust_store_binding",
    ] {
        let mut connection = in_memory_store();
        connection.execute(change, []).unwrap();
        let changes_before = connection.total_changes();
        let error = SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        }
        .observe_exact(&candidate)
        .unwrap_err();
        // The application must retain uncertainty on a failed read-back. Only
        // Empty from the selected store can exclude its attempted commit.
        assert_eq!(error.kind(), ControlStoreErrorKind::RecoveryRequired);
        assert!(connection.is_autocommit());
        assert_eq!(connection.total_changes(), changes_before);
        assert_eq!(row_counts(&connection), (0, 0, 0, 0, 0));
    }
}

#[test]
fn an_empty_bound_store_requires_a_readable_valid_transaction_head() {
    let candidate = candidate("bootstrap-damaged-head", r#"{"record":"receipt"}"#);
    for (damage, expected) in [
        (
            "PRAGMA ignore_check_constraints = ON; \
             UPDATE pale_control_store_header SET logical_transaction_sequence = 0; \
             PRAGMA ignore_check_constraints = OFF;",
            ControlStoreErrorKind::RecoveryRequired,
        ),
        (
            "ALTER TABLE pale_control_store_header DROP COLUMN logical_transaction_sequence;",
            ControlStoreErrorKind::Storage,
        ),
    ] {
        let mut connection = in_memory_store();
        // Corrupt only the throwaway transaction head. The selected store and
        // principal still match, so binding checks alone cannot reject it.
        connection.execute_batch(damage).unwrap();
        assert!(crate::principal_trust::binding_matches(&connection, candidate.binding()).unwrap());
        let changes_before = connection.total_changes();
        assert_eq!(
            SqlitePrincipalBootstrapCustody {
                connection: &mut connection,
            }
            .observe_exact(&candidate)
            .unwrap_err()
            .kind(),
            expected
        );
        assert!(connection.is_autocommit());
        assert_eq!(connection.total_changes(), changes_before);
        assert_eq!(row_counts(&connection), (0, 0, 0, 0, 0));
    }
}

#[test]
fn read_back_releases_its_snapshot_on_empty_exact_and_storage_failure() {
    let candidate = candidate("bootstrap-read-release", r#"{"record":"receipt"}"#);
    let mut connection = in_memory_store();
    assert_eq!(
        SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        }
        .observe_exact(&candidate)
        .unwrap(),
        PrincipalBootstrapDatabaseObservation::Empty
    );
    assert!(connection.is_autocommit());
    SqlitePrincipalBootstrapCustody {
        connection: &mut connection,
    }
    .compare_and_commit(&candidate)
    .unwrap();
    let changes_before = connection.total_changes();
    assert_eq!(
        SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        }
        .observe_exact(&candidate)
        .unwrap(),
        PrincipalBootstrapDatabaseObservation::Exact
    );
    assert!(connection.is_autocommit());
    assert_eq!(connection.total_changes(), changes_before);

    connection
        .execute_batch("DROP TABLE pale_principal_control_checkpoint_head")
        .unwrap();
    let changes_before = connection.total_changes();
    assert_eq!(
        SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        }
        .observe_exact(&candidate)
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::Storage
    );
    assert!(connection.is_autocommit());
    assert_eq!(connection.total_changes(), changes_before);
}

#[test]
fn read_back_keeps_one_snapshot_across_another_connection_commit() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("principal-trust.sqlite3");
    let candidate = candidate("bootstrap-snapshot", r#"{"record":"receipt"}"#);
    let mut reader = initialize_store(Connection::open(&path).unwrap());
    reader.pragma_update(None, "journal_mode", "WAL").unwrap();
    let mut writer = Connection::open(&path).unwrap();
    writer.pragma_update(None, "foreign_keys", true).unwrap();

    let snapshot = reader
        .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
        .unwrap();
    // The first read fixes the WAL snapshot before the second connection commits.
    assert!(crate::principal_trust::binding_matches(&snapshot, candidate.binding()).unwrap());
    SqlitePrincipalBootstrapCustody {
        connection: &mut writer,
    }
    .compare_and_commit(&candidate)
    .unwrap();
    assert_eq!(
        super::observe_exact_in(&snapshot, &candidate).unwrap(),
        PrincipalBootstrapDatabaseObservation::Empty
    );
    snapshot.rollback().unwrap();
    assert_eq!(
        SqlitePrincipalBootstrapCustody {
            connection: &mut reader,
        }
        .observe_exact(&candidate)
        .unwrap(),
        PrincipalBootstrapDatabaseObservation::Exact
    );

    let snapshot = reader
        .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
        .unwrap();
    assert!(crate::principal_trust::binding_matches(&snapshot, candidate.binding()).unwrap());
    writer
        .execute("DELETE FROM pale_principal_trust_checkpoint_head", [])
        .unwrap();
    assert_eq!(
        super::observe_exact_in(&snapshot, &candidate).unwrap(),
        PrincipalBootstrapDatabaseObservation::Exact
    );
    snapshot.rollback().unwrap();
    assert_eq!(
        SqlitePrincipalBootstrapCustody {
            connection: &mut reader,
        }
        .observe_exact(&candidate)
        .unwrap(),
        PrincipalBootstrapDatabaseObservation::ConflictingOrPartial
    );
    assert!(reader.is_autocommit());
}
