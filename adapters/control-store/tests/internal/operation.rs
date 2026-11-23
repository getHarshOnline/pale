//! Behavioral tests for deployment-bound operation custody.

use pale_application::resident::{
    prepare_operation_start, read_operation_start, OperationCustodyAnchorFailureReason,
    OperationStartCommitFailureReason,
};
use pale_core::records::{
    ArtifactDigest, CheckpointProfileDigest, ControlCheckpointDomainIdV1, ControlDomainEpochV1,
    ControlDomainSequenceV1, ControlIncarnationIdV1, ControlStoreIdV1, ResidentRecordRefV1,
    SchemaDigest, SchemaId, COMMON_REQUEST_ENVELOPE_V1_SCHEMA, CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
    OPERATION_HANDLE_V1_SCHEMA, OPERATION_LEASE_V1_SCHEMA, OPERATION_OBSERVATION_V1_SCHEMA,
};
use pale_ports::{
    AccumulatorCheckpointBinding, AccumulatorMutation, AccumulatorPosition, AntiRollbackAnchor,
    AntiRollbackAnchorObservation, AuthorityStoreCoordinate, ControlCheckpointHead,
    ControlDomainAntiRollbackAnchor, ControlRecord, ControlRecordBytes, ControlRecordCoordinate,
    ControlRecordDigest, ControlStoreErrorKind, OperationAccumulatorBatch,
    OperationCustodyCandidate, OperationCustodyCheckpoint, OperationCustodyCommitOutcome,
    OperationCustodyIndexKey, OperationCustodyIndexMutation, OperationCustodyIndexPredecessor,
    OperationCustodyIndexQuery, OperationCustodyPort, OperationCustodyReadBackOutcome,
    OperationCustodyRecord, OperationCustodyTransitionKind, StateAccumulatorBatch,
    UnresolvedCustodyAccumulatorBatch,
};
use rusqlite::{params, Connection};

use super::SqliteOperationCustody;
use crate::test_support::RetainedAntiRollbackAnchors;

#[path = "operation/local_recovery.rs"]
mod local_recovery;

pub(crate) use local_recovery::{with_generation, IndependentAnchor};

#[path = "operation/native_invocation.rs"]
mod native_invocation;

#[path = "operation/consult_service_restart.rs"]
mod consult_service_restart;

#[path = "operation/publication.rs"]
mod publication;

#[path = "operation/request_for_handle.rs"]
mod request_for_handle;

#[path = "operation/collection.rs"]
mod collection;
#[path = "operation/reconciliation_association.rs"]
mod reconciliation_association;
#[path = "operation/result.rs"]
mod result;
#[path = "operation/store_view.rs"]
mod store_view;

#[path = "operation/a2a_discovery.rs"]
mod a2a_discovery;
#[path = "operation/a2a_get_task.rs"]
mod a2a_get_task;
#[path = "operation/session_basis.rs"]
mod session_basis;
#[path = "operation/start_discovery.rs"]
mod start_discovery;

const MIGRATION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/deployment-control/0001_initial_v1.sql"
));

#[test]
fn accumulator_records_and_checkpoint_rollback_and_commit_as_one_disk_transaction() {
    let directory = tempfile::tempdir().unwrap();
    for (index, trigger) in [
        "CREATE TRIGGER abort_batch BEFORE INSERT ON pale_deployment_operation_accumulator_position BEGIN SELECT RAISE(ABORT,'injected node write'); END;",
        "CREATE TRIGGER abort_batch BEFORE INSERT ON pale_deployment_operation_index BEGIN SELECT RAISE(ABORT,'injected index write'); END;",
        "CREATE TRIGGER abort_batch BEFORE UPDATE ON pale_deployment_operation_checkpoint_head BEGIN SELECT RAISE(ABORT,'injected head write'); END;",
    ].iter().enumerate() {
        let path = directory.path().join(format!("atomic-{index}.sqlite"));
        let candidate = start_candidate("one", ['1','2','3','9']);
        {
            let mut connection = Connection::open(&path).unwrap();
            connection.pragma_update(None,"journal_mode","WAL").unwrap();
            connection.pragma_update(None,"synchronous","FULL").unwrap();
            initialize_store(&connection);
            connection.execute_batch(trigger).unwrap();
            assert!(SqliteOperationCustody { connection: &mut connection, coordinate: store() }.compare_and_commit(&candidate).is_err());
            assert_eq!(connection.query_row("SELECT COUNT(*) FROM pale_deployment_operation_accumulator_position",[],|row| row.get::<_,i64>(0)).unwrap(),0);
            assert_eq!(connection.query_row("SELECT COUNT(*) FROM pale_deployment_control_record",[],|row| row.get::<_,i64>(0)).unwrap(),1);
        }
        {
            let mut connection = Connection::open(&path).unwrap();
            assert_eq!(SqliteOperationCustody { connection: &mut connection, coordinate: store() }.observe_exact(&candidate).unwrap(),OperationCustodyReadBackOutcome::Predecessor);
            connection.execute_batch("DROP TRIGGER abort_batch").unwrap();
            connection.pragma_update(None,"foreign_keys",true).unwrap();
            connection.pragma_update(None,"synchronous","FULL").unwrap();
            assert_eq!(SqliteOperationCustody { connection: &mut connection, coordinate: store() }.compare_and_commit(&candidate).unwrap(),OperationCustodyCommitOutcome::Committed);
        }
        {
            let mut connection = Connection::open(&path).unwrap();
            let mut custody = SqliteOperationCustody { connection: &mut connection, coordinate: store() };
            assert_eq!(custody.observe_exact(&candidate).unwrap(),OperationCustodyReadBackOutcome::ExactSuccessor);
            assert_eq!(custody.compare_and_commit(&candidate).unwrap(),OperationCustodyCommitOutcome::ExactReplay);
            // Actual opaque record rows, indexes and the changed leaf are present;
            // this is physical custody, not admitted signature/root proof.
            assert_eq!(connection.query_row("SELECT COUNT(*) FROM pale_deployment_control_record",[],|row| row.get::<_,i64>(0)).unwrap(),5);
            assert_eq!(connection.query_row("SELECT COUNT(*) FROM pale_deployment_operation_accumulator_position",[],|row| row.get::<_,i64>(0)).unwrap(),1);
        }
    }
}

#[test]
fn missing_or_corrupt_accumulator_material_cannot_be_exact_replay() {
    for corruption in [
        "DELETE FROM pale_deployment_operation_accumulator_position",
        "UPDATE pale_deployment_operation_accumulator_position SET node_bytes=zeroblob(32)",
        "DELETE FROM pale_deployment_operation_accumulator_frontier",
        "UPDATE pale_deployment_operation_accumulator_frontier SET checkpoint_profile_ref='unlike-profile'",
        "UPDATE pale_deployment_operation_accumulator_frontier SET domain_epoch=5",
    ] {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_store(&connection);
        let candidate = start_candidate("one", ['1', '2', '3', '9']);
        SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .compare_and_commit(&candidate)
        .unwrap();
        connection.execute_batch(corruption).unwrap();
        let before = connection.total_changes();
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        assert_eq!(
            custody.observe_exact(&candidate).unwrap(),
            OperationCustodyReadBackOutcome::ConflictingOrPartial
        );
        assert_eq!(
            custody.compare_and_commit(&candidate).unwrap_err().kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        assert_eq!(connection.total_changes(), before);
    }
}

#[test]
fn existing_node_upsert_update_failure_rolls_back_observation_records() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let first = start_candidate("one", ['1', '2', '3', '9']);
    SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .compare_and_commit(&first)
    .unwrap();
    let second = observation_candidate(
        first
            .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
            .unwrap()
            .clone(),
        "one",
        '4',
    );
    connection.execute_batch("CREATE TRIGGER abort_update BEFORE UPDATE ON pale_deployment_operation_accumulator_position BEGIN SELECT RAISE(ABORT,'injected existing node update'); END;").unwrap();
    assert!(SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store()
    }
    .compare_and_commit(&second)
    .is_err());
    assert_eq!(
        SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store()
        }
        .observe_exact(&first)
        .unwrap(),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    assert_eq!(
        SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store()
        }
        .observe_exact(&second)
        .unwrap(),
        OperationCustodyReadBackOutcome::Predecessor
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM pale_deployment_control_record",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        5
    );
    connection
        .execute_batch("DROP TRIGGER abort_update")
        .unwrap();
    assert_eq!(
        SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store()
        }
        .compare_and_commit(&second)
        .unwrap(),
        OperationCustodyCommitOutcome::Committed
    );
}

#[test]
fn unresolved_write_failure_rolls_back_the_already_written_state_tree() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("both-trees.sqlite");
    let original = start_candidate("one", ['1', '2', '3', '9']);
    let batch = original.accumulator();
    let both = OperationAccumulatorBatch::new(
        batch.binding().clone(),
        batch.state().clone(),
        UnresolvedCustodyAccumulatorBatch::new(
            batch.unresolved().predecessor_root().clone(),
            ArtifactDigest::parse(digest('7')).unwrap(),
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([2; 32]),
                None,
                Some(vec![7; 32]),
                132,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
    )
    .unwrap();
    let indexes = original
        .indexes()
        .iter()
        .filter(|index| {
            !matches!(
                index,
                OperationCustodyIndexMutation::RequestForHandle { .. }
                    | OperationCustodyIndexMutation::StartObservation { .. }
                    | OperationCustodyIndexMutation::ObservationCheckpoint { .. }
            )
        })
        .cloned()
        .collect();
    let candidate = OperationCustodyCandidate::new_start(
        store(),
        original.kind(),
        original.records().to_vec(),
        indexes,
        original.checkpoint().clone(),
        both,
        original.session_basis().unwrap().clone(),
    )
    .unwrap();
    {
        let mut connection = Connection::open(&path).unwrap();
        initialize_store(&connection);
        connection.execute_batch("CREATE TRIGGER abort_unresolved BEFORE INSERT ON pale_deployment_operation_accumulator_position WHEN NEW.tree_kind='unresolved' BEGIN SELECT RAISE(ABORT,'injected unresolved insertion'); END;").unwrap();
        assert!(SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store()
        }
        .compare_and_commit(&candidate)
        .is_err());
    }
    {
        let mut connection = Connection::open(&path).unwrap();
        assert_eq!(
            SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store()
            }
            .observe_exact(&candidate)
            .unwrap(),
            OperationCustodyReadBackOutcome::Predecessor
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM pale_deployment_operation_accumulator_position",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        connection
            .execute_batch("DROP TRIGGER abort_unresolved")
            .unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .compare_and_commit(&candidate)
        .unwrap();
    }
    {
        let mut connection = Connection::open(&path).unwrap();
        assert_eq!(
            SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store()
            }
            .observe_exact(&candidate)
            .unwrap(),
            OperationCustodyReadBackOutcome::ExactSuccessor
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM pale_deployment_operation_accumulator_position",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            2
        );
        assert_eq!(
            SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store()
            }
            .compare_and_commit(&candidate)
            .unwrap(),
            OperationCustodyCommitOutcome::ExactReplay
        );
    }
}

#[test]
fn malformed_native_branch_is_rejected_before_database_access() {
    let mut connection = Connection::open_in_memory().unwrap();
    // No tables exist: a storage read would itself fail. Codec rejection is first.
    let original = start_candidate("one", ['1', '2', '3', '9']);
    let batch = original.accumulator();
    let malformed = OperationAccumulatorBatch::new(
        batch.binding().clone(),
        StateAccumulatorBatch::new(
            batch.state().predecessor_root().clone(),
            batch.state().successor_root().clone(),
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Branch {
                    height: 255,
                    path: [0; 32],
                },
                None,
                Some(vec![0; 66]),
                132,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        batch.unresolved().clone(),
    )
    .unwrap();
    let indexes = original
        .indexes()
        .iter()
        .filter(|index| {
            !matches!(
                index,
                OperationCustodyIndexMutation::RequestForHandle { .. }
                    | OperationCustodyIndexMutation::StartObservation { .. }
                    | OperationCustodyIndexMutation::ObservationCheckpoint { .. }
            )
        })
        .cloned()
        .collect();
    let candidate = OperationCustodyCandidate::new_start(
        store(),
        original.kind(),
        original.records().to_vec(),
        indexes,
        original.checkpoint().clone(),
        malformed,
        original.session_basis().unwrap().clone(),
    )
    .unwrap();
    assert_eq!(
        SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store()
        }
        .compare_and_commit(&candidate)
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(connection.total_changes(), 0);
}

#[test]
fn later_complete_checkpoint_does_not_require_historical_mutable_node_bytes() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let first = start_candidate("one", ['1', '2', '3', '9']);
    SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .compare_and_commit(&first)
    .unwrap();
    let second = observation_candidate(
        first
            .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
            .unwrap()
            .clone(),
        "one",
        '4',
    );
    assert_ne!(
        first.accumulator().state().mutations()[0].successor(),
        second.accumulator().state().mutations()[0].successor()
    );
    SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .compare_and_commit(&second)
    .unwrap();
    assert_eq!(
        SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store()
        }
        .compare_and_commit(&first)
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    for corruption in [
        "UPDATE pale_deployment_operation_accumulator_frontier SET checkpoint_profile_ref='changed-profile'",
        "UPDATE pale_deployment_operation_accumulator_frontier SET checkpoint_profile_ref='checkpoint-profile',checkpoint_profile_sha256=printf('%064d',0)",
    ] {
        connection.execute_batch(corruption).unwrap();
        let before = connection.total_changes();
        assert_eq!(
            SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store()
            }
            .compare_and_commit(&first)
            .unwrap_err()
            .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        assert_eq!(connection.total_changes(), before);
    }
}

#[cfg(feature = "bench-memory")]
#[test]
fn index_completeness_does_not_copy_the_opaque_body() {
    let mut connection = Connection::open_in_memory().expect("store");
    initialize_store(&connection);
    let candidate = start_candidate("one", ['1', '2', '3', '9']);
    SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .compare_and_commit(&candidate)
    .expect("start");
    let body_bytes = 1024 * 1024;
    connection
        .execute(
            "UPDATE pale_deployment_control_record SET canonical_record_jcs = zeroblob(?1) \
             WHERE record_ref = 'observation-one-1'",
            [body_bytes],
        )
        .expect("large opaque body");
    let query_key = key('2');
    let allocations = allocation_counter::measure(|| {
        assert!(super::index_is_complete(
            &connection,
            super::IndexRole::LatestObservation,
            &query_key,
        )
        .expect("structural record completeness"));
    });
    // This measures Rust-owned copies, not SQLite page reads or C allocations.
    assert!(
        allocations.bytes_total < body_bytes as u64,
        "metadata inspection copied the opaque body: {allocations:?}"
    );
    let query = OperationCustodyIndexQuery::LatestObservation(query_key);
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    let rejected = allocation_counter::measure(|| {
        assert_eq!(
            custody
                .read_index(&store(), &query, 1)
                .expect_err("body exceeds read ceiling")
                .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
    });
    assert!(
        rejected.bytes_total < body_bytes as u64,
        "rejected read copied the oversized body: {rejected:?}"
    );
}

#[test]
fn every_index_role_obeys_its_explicit_body_ceiling_without_mutation() {
    let mut connection = Connection::open_in_memory().expect("store");
    initialize_store(&connection);
    let candidate = start_candidate("one", ['1', '2', '3', '9']);
    SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .compare_and_commit(&candidate)
    .expect("start");
    connection.execute(
        "INSERT INTO pale_deployment_operation_index (index_role, index_key, record_ref, record_sha256) \
         VALUES ('parent_child_membership', ?1, 'handle-one', ?2)",
        params![key('f').as_str(), digest('2')],
    ).expect("physical child-membership fixture");
    let before = connection.total_changes();
    let queries = [
        OperationCustodyIndexQuery::RequestForHandle(key('2')),
        OperationCustodyIndexQuery::RequestEvidence(key('1')),
        OperationCustodyIndexQuery::RequestOperation(key('1')),
        OperationCustodyIndexQuery::OperationHandle(key('2')),
        OperationCustodyIndexQuery::LatestObservation(key('2')),
        OperationCustodyIndexQuery::StartObservation(key('1')),
        OperationCustodyIndexQuery::ParentChildMembership(key('f')),
        OperationCustodyIndexQuery::ObservationCheckpoint(
            OperationCustodyIndexKey::for_observation(&coordinate("observation-one-1", '3')),
        ),
    ];
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    for query in queries {
        let full = custody
            .read_index(&store(), &query, 16 * 1024)
            .expect("bounded baseline");
        let expected = full.record().expect("present role");
        let size = expected.canonical_record_jcs().as_slice().len();
        assert_eq!(
            custody
                .read_index(&store(), &query, size)
                .expect("exact limit")
                .record(),
            Some(expected)
        );
        for maximum in [0, size - 1] {
            assert_eq!(
                custody
                    .read_index(&store(), &query, maximum)
                    .expect_err("over limit")
                    .kind(),
                ControlStoreErrorKind::RecoveryRequired
            );
        }
    }
    assert!(custody
        .read_index(
            &store(),
            &OperationCustodyIndexQuery::ParentChildMembership(key('e')),
            0
        )
        .expect("absent membership")
        .record()
        .is_none());
    assert_eq!(connection.total_changes(), before);
}

#[test]
fn bounded_record_queries_share_a_wal_snapshot_across_a_writer_commit() {
    let directory = tempfile::tempdir().expect("temporary database");
    let path = directory.path().join("snapshot.sqlite3");
    let mut reader = Connection::open(&path).expect("reader");
    reader
        .pragma_update(None, "journal_mode", "WAL")
        .expect("WAL");
    initialize_store(&reader);
    let candidate = start_candidate("one", ['1', '2', '3', '9']);
    SqliteOperationCustody {
        connection: &mut reader,
        coordinate: store(),
    }
    .compare_and_commit(&candidate)
    .expect("start");
    let expected = record("observation-one-1", '3', OPERATION_OBSERVATION_V1_SCHEMA);
    let snapshot = reader
        .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
        .expect("read transaction");
    super::require_store_binding(&snapshot, &store()).expect("header establishes snapshot");
    let indexed =
        super::read_index_coordinate(&snapshot, super::IndexRole::LatestObservation, &key('2'))
            .expect("index")
            .expect("present coordinate");
    let mut writer = Connection::open(&path).expect("independent writer");
    let change = writer.transaction().expect("write transaction");
    change
        .execute(
            "UPDATE pale_control_store_header SET control_incarnation_id = 'replacement'",
            [],
        )
        .expect("replace live header");
    change.execute("UPDATE pale_deployment_control_record SET canonical_record_jcs = zeroblob(1048576) WHERE record_ref = 'observation-one-1'", [])
        .expect("replace live body");
    change
        .commit()
        .expect("writer commits while snapshot remains open");
    let observed = super::read_record_bounded(
        &snapshot,
        &indexed,
        expected.canonical_record_jcs().as_slice().len(),
    )
    .expect("old snapshot remains internally consistent")
    .expect("original body");
    assert_eq!(observed, expected);
    snapshot.commit().expect("release snapshot");
    assert_eq!(
        SqliteOperationCustody {
            connection: &mut reader,
            coordinate: store()
        }
        .read_index(
            &store(),
            &OperationCustodyIndexQuery::LatestObservation(key('2')),
            16 * 1024
        )
        .expect_err("next read must reject replacement incarnation")
        .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn metadata_only_completeness_preserves_recovery_rejections() {
    for change in [
        "DELETE FROM pale_deployment_control_record WHERE record_ref = 'observation-one-1'",
        "UPDATE pale_deployment_control_record SET record_sha256 = replace(record_sha256, '3', '4') WHERE record_ref = 'observation-one-1'",
        "UPDATE pale_deployment_control_record SET schema_id = 'Pale/invalid' WHERE record_ref = 'observation-one-1'",
        "UPDATE pale_deployment_control_record SET schema_sha256 = 'invalid' WHERE record_ref = 'observation-one-1'",
        "UPDATE pale_deployment_control_record SET canonical_record_jcs = x'' WHERE record_ref = 'observation-one-1'",
    ] {
        let mut connection = Connection::open_in_memory().expect("store");
        initialize_store(&connection);
        let candidate = start_candidate("one", ['1', '2', '3', '9']);
        SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .compare_and_commit(&candidate)
        .expect("start");
        connection
            .pragma_update(None, "foreign_keys", false)
            .expect("fixture corruption");
        connection
            .pragma_update(None, "ignore_check_constraints", true)
            .expect("fixture corruption");
        connection
            .execute_batch(change)
            .expect("inject incomplete metadata");
        let observed =
            super::index_is_complete(&connection, super::IndexRole::LatestObservation, &key('2'));
        assert!(
            matches!(observed, Ok(false)) || observed.is_err(),
            "invalid metadata must not be complete"
        );
        assert_eq!(
            SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store()
            }
            .compare_and_commit(&candidate)
            .expect_err("corrupt recovery state")
            .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
    }
}

#[test]
fn index_and_checkpoint_coordinates_are_bounded_before_owned_decoding() {
    let valid_digest = digest('a');
    for reference in ["é".repeat(256), "x".repeat(512)] {
        assert!(super::decode_record_coordinate(
            rusqlite::types::ValueRef::Text(reference.as_bytes()),
            rusqlite::types::ValueRef::Text(valid_digest.as_bytes()),
        )
        .is_ok());
    }
    for reference in [
        Vec::new(),
        vec![b'x'; 513],
        "é".repeat(257).into_bytes(),
        vec![0xff],
    ] {
        assert_eq!(
            super::decode_record_coordinate(
                rusqlite::types::ValueRef::Text(&reference),
                rusqlite::types::ValueRef::Text(valid_digest.as_bytes()),
            )
            .expect_err("invalid reference")
            .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
    }
    for bytes in [
        vec![b'a'; 63],
        vec![b'a'; 65],
        vec![b'G'; 64],
        vec![0xff; 64],
    ] {
        assert_eq!(
            super::decode_record_coordinate(
                rusqlite::types::ValueRef::Text(b"reference"),
                rusqlite::types::ValueRef::Text(&bytes),
            )
            .expect_err("invalid digest")
            .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
    }
}

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record reference"),
        ControlRecordDigest::parse(digest(byte)).expect("record digest"),
    )
}

fn record(reference: &str, byte: char, schema_id: &str) -> ControlRecord {
    ControlRecord::new(
        coordinate(reference, byte),
        SchemaId::parse(schema_id).expect("schema identity"),
        SchemaDigest::parse(digest('f')).expect("schema digest"),
        ControlRecordBytes::new(format!(r#"{{"record":"{reference}"}}"#).into_bytes())
            .expect("record bytes"),
    )
}

fn key(byte: char) -> OperationCustodyIndexKey {
    OperationCustodyIndexKey::parse(digest(byte)).expect("index key")
}

#[test]
fn unrelated_lease_retention_fences_a_prepared_operation_without_implicit_retry() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let candidate = start_candidate("counter-race", ['1', '2', '3', '9']);
    let lease = record("intervening-lease", 'e', OPERATION_LEASE_V1_SCHEMA);
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody
            .compare_and_retain_operation_lease(&store(), &lease)
            .unwrap(),
        OperationCustodyCommitOutcome::Committed
    );
    let changes = custody.connection.total_changes();
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    assert_eq!(custody.connection.total_changes(), changes);
    assert_eq!(
        custody.observe_exact(&candidate).unwrap(),
        OperationCustodyReadBackOutcome::Predecessor
    );
    assert_eq!(
        crate::store::transaction::read(custody.connection, &store())
            .unwrap()
            .get(),
        29
    );
    assert_eq!(
        custody
            .connection
            .query_row(
                "SELECT COUNT(*) FROM pale_deployment_control_record",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );
}

#[test]
fn exact_operation_and_lease_replay_survive_later_transactions_and_saturation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("transaction-replay.sqlite");
    let candidate = start_candidate("counter-replay", ['1', '2', '3', '9']);
    let lease = record("later-lease", 'e', OPERATION_LEASE_V1_SCHEMA);
    {
        let mut connection = Connection::open(&path).unwrap();
        initialize_store(&connection);
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        assert_eq!(
            custody.compare_and_commit(&candidate).unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
        assert_eq!(
            custody
                .compare_and_retain_operation_lease(&store(), &lease)
                .unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
        assert_eq!(
            crate::store::transaction::read(custody.connection, &store())
                .unwrap()
                .get(),
            30
        );
        let changes = custody.connection.total_changes();
        assert_eq!(
            custody.compare_and_commit(&candidate).unwrap(),
            OperationCustodyCommitOutcome::ExactReplay
        );
        assert_eq!(custody.connection.total_changes(), changes);
        assert_eq!(custody.connection.execute("UPDATE pale_control_store_header SET logical_transaction_sequence=9007199254740991 WHERE singleton=1", []).unwrap(), 1);
    }
    let mut connection = Connection::open(&path).unwrap();
    let changes = connection.total_changes();
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(
        custody
            .compare_and_retain_operation_lease(&store(), &lease)
            .unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(
        custody
            .compare_and_retain_operation_lease(
                &store(),
                &record("new-lease", 'd', OPERATION_LEASE_V1_SCHEMA)
            )
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::TransactionExhausted
    );
    assert_eq!(custody.connection.total_changes(), changes);
    assert_eq!(
        crate::store::transaction::read(custody.connection, &store())
            .unwrap()
            .get(),
        9_007_199_254_740_991
    );
}

#[test]
fn unlike_atom_member_insertion_stamp_is_recovery_not_exact_replay() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let candidate = start_candidate("counter-stamp", ['1', '2', '3', '9']);
    {
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        custody.compare_and_commit(&candidate).unwrap();
    }
    let member = candidate.records()[0].record();
    assert_eq!(connection.execute(
        "UPDATE pale_deployment_control_record SET inserted_transaction_sequence=28 WHERE record_ref=?1 AND record_sha256=?2",
        params![member.coordinate().record_ref().as_str(), member.coordinate().record_sha256().as_str()],
    ).unwrap(), 1);
    let changes = connection.total_changes();
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(custody.connection.total_changes(), changes);
}

pub(crate) fn store() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("deployment-store-1").expect("store identity"),
        ControlIncarnationIdV1::parse("deployment-incarnation-1").expect("store incarnation"),
    )
}

fn other_store() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("deployment-store-1").expect("store identity"),
        ControlIncarnationIdV1::parse("deployment-incarnation-2").expect("store incarnation"),
    )
}

pub(crate) fn checkpoint(sequence: u64, reference: &str, byte: char) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("operation:principal-1").expect("checkpoint domain"),
        ControlDomainEpochV1::new(4).expect("domain epoch"),
        ControlDomainSequenceV1::new(sequence).expect("domain sequence"),
        pale_core::records::ControlTransactionSequenceV1::new(sequence + 20).unwrap(),
        coordinate(reference, byte),
    )
}

fn retained_operation_anchor(head: &ControlCheckpointHead) -> AntiRollbackAnchor {
    AntiRollbackAnchor::from(ControlDomainAntiRollbackAnchor::new(
        store().store_id().clone(),
        store().incarnation_id().clone(),
        head.checkpoint_domain_id().clone(),
        head.domain_epoch(),
        head.domain_sequence(),
        head.coordinate().clone(),
    ))
}

fn checkpoint_transition(
    predecessor_sequence: u64,
    predecessor_reference: &str,
    predecessor_byte: char,
    successor_sequence: u64,
    successor_reference: &str,
    successor_byte: char,
) -> OperationCustodyCheckpoint {
    OperationCustodyCheckpoint::new(
        pale_ports::OperationCheckpointPhase::Successor {
            predecessor: checkpoint(
                predecessor_sequence,
                predecessor_reference,
                predecessor_byte,
            ),
            predecessor_record: Box::new(record(
                predecessor_reference,
                predecessor_byte,
                CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            )),
        },
        checkpoint(successor_sequence, successor_reference, successor_byte),
        record(
            successor_reference,
            successor_byte,
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
        ),
    )
    .expect("checkpoint transition")
}

pub(crate) fn start_candidate(tag: &str, bytes: [char; 4]) -> OperationCustodyCandidate {
    let checkpoint = checkpoint_transition(
        8,
        "checkpoint-8",
        '8',
        9,
        &format!("checkpoint-9-{tag}"),
        bytes[3],
    );
    start_candidate_at_checkpoint(tag, bytes, key(bytes[1]), checkpoint)
}

fn start_candidate_at_checkpoint(
    tag: &str,
    bytes: [char; 4],
    operation_key: OperationCustodyIndexKey,
    checkpoint: OperationCustodyCheckpoint,
) -> OperationCustodyCandidate {
    let request_reference = format!("request-{tag}");
    let handle_reference = format!("handle-{tag}");
    let observation_reference = format!("observation-{tag}-1");
    let request = record(
        &request_reference,
        bytes[0],
        COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
    );
    let handle = record(&handle_reference, bytes[1], OPERATION_HANDLE_V1_SCHEMA);
    let observation = record(
        &observation_reference,
        bytes[2],
        OPERATION_OBSERVATION_V1_SCHEMA,
    );
    // Opaque physical conformance bytes, not an admitted session authority.
    let basis = pale_ports::OperationSessionBasis::new(
        handle.coordinate().clone(),
        request.coordinate().clone(),
        pale_core::records::SessionAdmissionLeaseDigest::parse(digest('a')).unwrap(),
        br#"{"physical_fixture":"session-basis"}"#.to_vec(),
        4096,
    )
    .unwrap();
    OperationCustodyCandidate::new_start(
        store(),
        OperationCustodyTransitionKind::StartRoot,
        vec![
            OperationCustodyRecord::CommonRequest(request.clone()),
            OperationCustodyRecord::OperationHandle(handle.clone()),
            OperationCustodyRecord::OperationObservation(observation.clone()),
        ],
        vec![
            OperationCustodyIndexMutation::RequestEvidence {
                key: key(bytes[0]),
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: request.coordinate().clone(),
            },
            OperationCustodyIndexMutation::RequestOperation {
                key: key(bytes[0]),
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: handle.coordinate().clone(),
            },
            OperationCustodyIndexMutation::OperationHandle {
                key: operation_key.clone(),
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: handle.coordinate().clone(),
            },
            OperationCustodyIndexMutation::LatestObservation {
                key: operation_key,
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: observation.coordinate().clone(),
            },
        ],
        checkpoint.clone(),
        physical_accumulator(store(), &checkpoint),
        basis,
    )
    .expect("start candidate")
}

fn observation_candidate(
    predecessor: ControlRecord,
    tag: &str,
    observation_byte: char,
) -> OperationCustodyCandidate {
    let successor = record(
        &format!("observation-{tag}-2"),
        observation_byte,
        OPERATION_OBSERVATION_V1_SCHEMA,
    );
    let checkpoint = checkpoint_transition(
        9,
        &format!("checkpoint-9-{tag}"),
        '9',
        10,
        &format!("checkpoint-10-{tag}"),
        'a',
    );
    OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        vec![OperationCustodyRecord::OperationObservation(
            successor.clone(),
        )],
        vec![OperationCustodyIndexMutation::LatestObservation {
            key: key('2'),
            expected: OperationCustodyIndexPredecessor::Exact(predecessor),
            successor: successor.coordinate().clone(),
        }],
        checkpoint.clone(),
        physical_accumulator(store(), &checkpoint),
    )
    .expect("observation candidate")
}

// Physical adapter conformance only. Existing opaque ControlRecord fixtures do
// not prove admitted signatures, application root construction, or a profile.
fn physical_accumulator(
    store: AuthorityStoreCoordinate,
    checkpoint: &OperationCustodyCheckpoint,
) -> OperationAccumulatorBatch {
    let pale_ports::OperationCheckpointPhase::Successor { predecessor, .. } = checkpoint.phase()
    else {
        panic!("existing replacement fixture requires a predecessor")
    };
    let sequence = predecessor.domain_sequence().get();
    let previous = (sequence != 8).then(|| vec![u8::try_from(sequence).unwrap(); 32]);
    let transaction = pale_ports::ControlTransactionPreparation::new(
        store.clone(),
        pale_core::records::ControlTransactionSequenceV1::new(
            checkpoint
                .successor()
                .committed_transaction_sequence()
                .get()
                - 1,
        )
        .unwrap(),
    )
    .unwrap();
    OperationAccumulatorBatch::new(
        AccumulatorCheckpointBinding::new(
            store,
            ResidentRecordRefV1::parse("checkpoint-profile").unwrap(),
            CheckpointProfileDigest::parse(digest('b')).unwrap(),
            pale_ports::AccumulatorCheckpointPhase::Successor(predecessor.clone()),
            checkpoint.successor().clone(),
            transaction,
        )
        .unwrap(),
        StateAccumulatorBatch::new(
            ArtifactDigest::parse(predecessor.coordinate().record_sha256().as_str()).unwrap(),
            ArtifactDigest::parse(checkpoint.successor().coordinate().record_sha256().as_str())
                .unwrap(),
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([1; 32]),
                previous,
                Some(vec![u8::try_from(sequence + 1).unwrap(); 32]),
                132,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        UnresolvedCustodyAccumulatorBatch::new(
            ArtifactDigest::parse(digest('0')).unwrap(),
            ArtifactDigest::parse(digest('0')).unwrap(),
            vec![],
            1,
        )
        .unwrap(),
    )
    .unwrap()
}

pub(crate) fn initialize_store(connection: &Connection) {
    connection
        .pragma_update(None, "foreign_keys", true)
        .expect("foreign keys");
    connection.execute_batch(MIGRATION).expect("migration");
    connection
        .execute(
            "INSERT INTO pale_control_store_header \
             (singleton, store_kind, logical_store_id, control_incarnation_id, store_schema_id, \
              store_schema_sha256, store_schema_version, storage_manifest_sha256, migration_head_sha256, logical_transaction_sequence) \
             VALUES (1, 'deployment_control', ?1, ?2, \
                     'pale/control-store/deployment-control.v1', ?3, 1, ?4, ?5, 28)",
            params![
                store().store_id().as_str(),
                store().incarnation_id().as_str(),
                digest('a'),
                digest('b'),
                digest('c'),
            ],
        )
        .expect("store header");
    let predecessor = record("checkpoint-8", '8', CONTROL_STATE_CHECKPOINT_V1_SCHEMA);
    insert_record(connection, &predecessor);
    connection
        .execute(
            "INSERT INTO pale_deployment_operation_checkpoint_head \
             (checkpoint_domain_id, domain_epoch, domain_sequence, \
              control_checkpoint_ref, control_checkpoint_sha256) VALUES (?1, 4, 8, ?2, ?3)",
            params![
                "operation:principal-1",
                predecessor.coordinate().record_ref().as_str(),
                predecessor.coordinate().record_sha256().as_str(),
            ],
        )
        .expect("checkpoint head");
    connection.execute(
        "INSERT INTO pale_deployment_operation_accumulator_frontier \
         (checkpoint_domain_id,domain_epoch,checkpoint_profile_ref,checkpoint_profile_sha256,control_checkpoint_ref,control_checkpoint_sha256,state_root_sha256,unresolved_root_sha256) \
         VALUES(?1,4,'checkpoint-profile',?2,'checkpoint-8',?3,?3,?4)",
        params!["operation:principal-1",digest('b'),digest('8'),digest('0')]
    ).expect("physical accumulator predecessor");
}

#[test]
fn restored_sqlite_predecessor_cannot_recommit_below_the_retained_anchor() {
    let directory = tempfile::tempdir().expect("temporary stores");
    let live_path = directory.path().join("live.sqlite3");
    let restored_path = directory.path().join("restored.sqlite3");
    let connection = Connection::open(&live_path).expect("live store");
    initialize_store(&connection);
    drop(connection);
    std::fs::copy(&live_path, &restored_path).expect("pre-start database snapshot");

    let candidate = start_candidate("one", ['1', '2', '3', '9']);
    let pale_ports::OperationCheckpointPhase::Successor { predecessor, .. } =
        candidate.checkpoint().phase()
    else {
        panic!("restoration fixture is a successor")
    };
    let mut anchor =
        RetainedAntiRollbackAnchors::with_anchor(retained_operation_anchor(predecessor));
    let mut live = Connection::open(&live_path).expect("reopen live store");
    let completed = prepare_operation_start(candidate.clone())
        .expect("prepared start")
        .commit(
            &mut SqliteOperationCustody {
                connection: &mut live,
                coordinate: store(),
            },
            &mut anchor,
            16 * 1024,
        )
        .expect("live database commit")
        .advance_anchor(&mut anchor)
        .expect("independent anchor advance");
    assert_eq!(completed.candidate(), &candidate);
    drop(live);

    let mut restored = Connection::open(&restored_path).expect("reopen restored store");
    let changes_before = restored.total_changes();
    let failure = prepare_operation_start(candidate.clone())
        .expect("same prepared start")
        .commit(
            &mut SqliteOperationCustody {
                connection: &mut restored,
                coordinate: store(),
            },
            &mut anchor,
            16 * 1024,
        )
        .expect_err("retained successor rejects restored database predecessor");
    assert!(matches!(
        failure.reason(),
        OperationStartCommitFailureReason::AnchorPrecondition(
            OperationCustodyAnchorFailureReason::NotAdvanced(observation)
        ) if matches!(observation.as_ref(), AntiRollbackAnchorObservation::ExactSuccessor)
    ));
    assert_eq!(restored.total_changes(), changes_before);
    let stored_request = candidate
        .record(pale_ports::OperationCustodyRecordKind::CommonRequest)
        .expect("request record");
    assert!(read_operation_start(
        &mut SqliteOperationCustody {
            connection: &mut restored,
            coordinate: store(),
        },
        &store(),
        &key('1'),
        stored_request,
        16 * 1024,
    )
    .expect("restored start lookup")
    .is_none());
}

fn insert_record(connection: &Connection, record: &ControlRecord) {
    connection
        .execute(
            "INSERT INTO pale_deployment_control_record \
             (record_ref, record_sha256, schema_id, schema_sha256, canonical_record_jcs, inserted_transaction_sequence) \
             VALUES (?1, ?2, ?3, ?4, ?5, (SELECT logical_transaction_sequence FROM pale_control_store_header WHERE singleton=1))",
            params![
                record.coordinate().record_ref().as_str(),
                record.coordinate().record_sha256().as_str(),
                record.schema_id().as_str(),
                record.schema_sha256().as_str(),
                record.canonical_record_jcs().as_slice(),
            ],
        )
        .expect("seed record");
}

#[test]
fn atomic_start_replay_point_reads_and_restart_preserve_one_operation() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("operation.sqlite3");
    let candidate = start_candidate("one", ['1', '2', '3', '9']);
    {
        let mut connection = Connection::open(&path).expect("open SQLite");
        initialize_store(&connection);
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        assert_eq!(
            custody.observe_exact(&candidate).expect("predecessor"),
            OperationCustodyReadBackOutcome::Predecessor
        );
        assert_eq!(
            custody.compare_and_commit(&candidate).expect("commit"),
            OperationCustodyCommitOutcome::Committed
        );
        assert_eq!(
            custody.compare_and_commit(&candidate).expect("replay"),
            OperationCustodyCommitOutcome::ExactReplay
        );
        let request = custody
            .read_index(
                candidate.store(),
                &OperationCustodyIndexQuery::RequestEvidence(key('1')),
                16 * 1024,
            )
            .expect("request evidence");
        assert_eq!(
            request.record().expect("request record").coordinate(),
            &coordinate("request-one", '1')
        );
        let handle = custody
            .read_index(
                candidate.store(),
                &OperationCustodyIndexQuery::RequestOperation(key('1')),
                16 * 1024,
            )
            .expect("request handle");
        assert_eq!(
            handle.record().expect("handle record").coordinate(),
            &coordinate("handle-one", '2')
        );
    }
    let mut connection = Connection::open(&path).expect("reopen SQLite");
    connection
        .pragma_update(None, "foreign_keys", true)
        .expect("foreign keys");
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody
            .observe_exact(&candidate)
            .expect("restart read-back"),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
}

#[test]
fn operation_identity_collision_preserves_the_original_and_does_not_consume_the_new_request() {
    let mut connection = Connection::open_in_memory().expect("in-memory SQLite");
    initialize_store(&connection);
    let original = start_candidate("one", ['1', '2', '3', '9']);
    let next_checkpoint =
        checkpoint_transition(9, "checkpoint-9-one", '9', 10, "checkpoint-10-two", 'a');
    let reused_operation_identity = start_candidate_at_checkpoint(
        "two",
        ['4', '5', '6', 'a'],
        key('2'),
        next_checkpoint.clone(),
    );
    let distinct_operation_identity =
        start_candidate_at_checkpoint("two", ['4', '5', '6', 'a'], key('5'), next_checkpoint);
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };

    assert_eq!(
        custody.compare_and_commit(&original).expect("first start"),
        OperationCustodyCommitOutcome::Committed
    );
    assert_eq!(
        custody
            .compare_and_commit(&reused_operation_identity)
            .expect_err("an operation identity cannot be rebound to another handle")
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    let retained_original = custody
        .read_index(
            original.store(),
            &OperationCustodyIndexQuery::OperationHandle(key('2')),
            16 * 1024,
        )
        .expect("original operation identity");
    assert_eq!(
        retained_original
            .record()
            .expect("original handle")
            .coordinate(),
        &coordinate("handle-one", '2')
    );
    assert!(custody
        .read_index(
            original.store(),
            &OperationCustodyIndexQuery::RequestOperation(key('4')),
            16 * 1024,
        )
        .expect("unconsumed new request identity")
        .record()
        .is_none());
    assert_eq!(
        custody
            .read_operation_record(original.store(), &coordinate("handle-two", '5'), 16 * 1024,)
            .expect("rolled-back handle lookup"),
        None
    );

    assert_eq!(
        custody
            .compare_and_commit(&distinct_operation_identity)
            .expect("distinct operation identity"),
        OperationCustodyCommitOutcome::Committed
    );
    assert_eq!(
        custody
            .compare_and_commit(&distinct_operation_identity)
            .expect("exact new start replay"),
        OperationCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(
        custody
            .read_index(
                original.store(),
                &OperationCustodyIndexQuery::RequestOperation(key('4')),
                16 * 1024,
            )
            .expect("new request mapping")
            .record()
            .expect("new operation handle")
            .coordinate(),
        &coordinate("handle-two", '5')
    );
}

#[test]
fn recovery_indexes_survive_reopen_after_the_latest_head_advances() {
    let directory = tempfile::tempdir().expect("temporary store");
    let path = directory.path().join("history.sqlite3");
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let append = observation_candidate(start.records()[2].record().clone(), "one", '4');
    {
        let mut connection = Connection::open(&path).expect("open");
        initialize_store(&connection);
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        custody.compare_and_commit(&start).expect("start");
        custody.compare_and_commit(&append).expect("append");
    }
    let mut connection = Connection::open(&path).expect("reopen");
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    for (query, expected) in [
        (
            OperationCustodyIndexQuery::RequestForHandle(key('2')),
            coordinate("request-one", '1'),
        ),
        (
            OperationCustodyIndexQuery::StartObservation(key('1')),
            coordinate("observation-one-1", '3'),
        ),
        (
            OperationCustodyIndexQuery::ObservationCheckpoint(
                OperationCustodyIndexKey::for_observation(&coordinate("observation-one-1", '3')),
            ),
            coordinate("checkpoint-9-one", '9'),
        ),
        (
            OperationCustodyIndexQuery::ObservationCheckpoint(
                OperationCustodyIndexKey::for_observation(&coordinate("observation-one-2", '4')),
            ),
            coordinate("checkpoint-10-one", 'a'),
        ),
        (
            OperationCustodyIndexQuery::LatestObservation(key('2')),
            coordinate("observation-one-2", '4'),
        ),
    ] {
        let observed = custody
            .read_index(&store(), &query, 16 * 1024)
            .expect("historical point read");
        assert_eq!(
            observed.record().expect("retained record").coordinate(),
            &expected
        );
    }
    assert_eq!(
        custody
            .compare_and_commit(&start)
            .expect_err("never replay stale start")
            .kind(),
        ControlStoreErrorKind::CandidateConflict
    );
}

#[test]
fn application_request_lookup_recovers_original_start_after_reopen_and_latest_advance() {
    let directory = tempfile::tempdir().expect("temporary store");
    let path = directory.path().join("application-recovery.sqlite3");
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let append = observation_candidate(start.records()[2].record().clone(), "one", '4');
    let expected_request = start.records()[0].record().clone();
    let expected_handle = start.records()[1].record().clone();
    let expected_initial_observation = start.records()[2].record().clone();
    let expected_start_checkpoint = start.checkpoint().successor_record().clone();
    let expected_latest_observation = append.records()[0].record().clone();

    {
        let mut connection = Connection::open(&path).expect("open");
        initialize_store(&connection);
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        custody.compare_and_commit(&start).expect("start");
        custody.compare_and_commit(&append).expect("append");
    }
    drop(start);
    drop(append);

    let mut connection = Connection::open(&path).expect("reopen");
    let changes_before = connection.total_changes();
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    let evidence = read_operation_start(
        &mut custody,
        &store(),
        &key('1'),
        &expected_request,
        16 * 1024,
    )
    .expect("read original start without a candidate")
    .expect("existing operation");

    assert_eq!(evidence.store(), &store());
    assert_eq!(evidence.request(), &expected_request);
    assert_eq!(evidence.handle(), &expected_handle);
    assert_eq!(
        evidence.initial_observation(),
        &expected_initial_observation
    );
    assert_eq!(evidence.checkpoint(), &expected_start_checkpoint);

    let latest = custody
        .read_index(
            &store(),
            &OperationCustodyIndexQuery::LatestObservation(key('2')),
            16 * 1024,
        )
        .expect("latest observation after duplicate recovery");
    assert_eq!(latest.record(), Some(&expected_latest_observation));
    assert_eq!(connection.total_changes(), changes_before);
}

#[test]
fn selected_observation_keeps_its_immutable_checkpoint_across_advance_and_restart() {
    // These opaque physical fixtures prove the port's immutable index join,
    // not contextual operation admission or a public Observe result.
    let directory = tempfile::tempdir().expect("temporary store");
    let path = directory.path().join("selected-observation.sqlite3");
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let append = observation_candidate(start.records()[2].record().clone(), "one", '4');
    let mut reader = Connection::open(&path).expect("reader");
    reader
        .pragma_update(None, "journal_mode", "WAL")
        .expect("WAL");
    initialize_store(&reader);
    SqliteOperationCustody {
        connection: &mut reader,
        coordinate: store(),
    }
    .compare_and_commit(&start)
    .expect("start");

    let selected = SqliteOperationCustody {
        connection: &mut reader,
        coordinate: store(),
    }
    .read_index(
        &store(),
        &OperationCustodyIndexQuery::LatestObservation(key('2')),
        16 * 1024,
    )
    .expect("select latest once")
    .into_record()
    .expect("initial observation");
    let mut writer = Connection::open(&path).expect("independent writer");
    SqliteOperationCustody {
        connection: &mut writer,
        coordinate: store(),
    }
    .compare_and_commit(&append)
    .expect("advance between recovery reads");
    let selected_checkpoint = SqliteOperationCustody {
        connection: &mut reader,
        coordinate: store(),
    }
    .read_index(
        &store(),
        &OperationCustodyIndexQuery::ObservationCheckpoint(
            OperationCustodyIndexKey::for_observation(selected.coordinate()),
        ),
        16 * 1024,
    )
    .expect("immutable checkpoint join")
    .into_record()
    .expect("original checkpoint");
    assert_eq!(&selected, start.records()[2].record());
    assert_eq!(&selected_checkpoint, start.checkpoint().successor_record());
    assert_ne!(&selected_checkpoint, append.checkpoint().successor_record());
    drop(writer);
    drop(reader);

    let mut reopened = Connection::open(&path).expect("restart");
    let changes_before = reopened.total_changes();
    let mut custody = SqliteOperationCustody {
        connection: &mut reopened,
        coordinate: store(),
    };
    let current = custody
        .read_index(
            &store(),
            &OperationCustodyIndexQuery::LatestObservation(key('2')),
            16 * 1024,
        )
        .expect("new selection")
        .into_record()
        .expect("successor observation");
    let current_checkpoint = custody
        .read_index(
            &store(),
            &OperationCustodyIndexQuery::ObservationCheckpoint(
                OperationCustodyIndexKey::for_observation(current.coordinate()),
            ),
            16 * 1024,
        )
        .expect("successor checkpoint join")
        .into_record()
        .expect("successor checkpoint");
    assert_eq!(&current, append.records()[0].record());
    assert_eq!(&current_checkpoint, append.checkpoint().successor_record());
    assert_ne!(selected.coordinate(), current.coordinate());
    let original_start = read_operation_start(
        &mut custody,
        &store(),
        &key('1'),
        start.records()[0].record(),
        16 * 1024,
    )
    .expect("original request recovery")
    .expect("retained start");
    assert_eq!(original_start.initial_observation(), &selected);
    assert_eq!(original_start.checkpoint(), &selected_checkpoint);
    assert_eq!(reopened.total_changes(), changes_before);
}

#[test]
fn missing_or_wrong_schema_recovery_indexes_require_recovery() {
    for role in [
        "request_for_handle",
        "start_observation",
        "observation_checkpoint",
    ] {
        for corrupt in [false, true] {
            let mut connection = Connection::open_in_memory().expect("store");
            initialize_store(&connection);
            let start = start_candidate("one", ['1', '2', '3', '9']);
            SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store(),
            }
            .compare_and_commit(&start)
            .expect("start");
            if corrupt {
                connection.execute("UPDATE pale_deployment_operation_index SET record_ref = 'handle-one', record_sha256 = ?1 WHERE index_role = ?2",
                    params![digest('2'), role]).expect("inject wrong schema");
            } else {
                connection
                    .execute(
                        "DELETE FROM pale_deployment_operation_index WHERE index_role = ?1",
                        [role],
                    )
                    .expect("remove recovery index");
            }
            let query = if role == "request_for_handle" {
                OperationCustodyIndexQuery::RequestForHandle(key('2'))
            } else if role == "start_observation" {
                OperationCustodyIndexQuery::StartObservation(key('1'))
            } else {
                OperationCustodyIndexQuery::ObservationCheckpoint(
                    OperationCustodyIndexKey::for_observation(&coordinate(
                        "observation-one-1",
                        '3',
                    )),
                )
            };
            let mut custody = SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store(),
            };
            if corrupt {
                assert_eq!(
                    custody
                        .read_index(&store(), &query, 16 * 1024,)
                        .expect_err("wrong schema")
                        .kind(),
                    ControlStoreErrorKind::RecoveryRequired
                );
            } else {
                assert!(custody
                    .read_index(&store(), &query, 16 * 1024,)
                    .expect("absent")
                    .record()
                    .is_none());
            }
            assert_eq!(
                custody
                    .compare_and_commit(&start)
                    .expect_err("incomplete history")
                    .kind(),
                ControlStoreErrorKind::RecoveryRequired
            );
        }
    }
}

#[test]
fn observation_cas_advances_only_the_latest_observation_and_checkpoint() {
    let mut connection = Connection::open_in_memory().expect("in-memory SQLite");
    initialize_store(&connection);
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let predecessor = start.records()[2].record().clone();
    let observation = observation_candidate(predecessor, "one", '4');
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    custody.compare_and_commit(&start).expect("durable start");
    assert_eq!(
        custody
            .compare_and_commit(&observation)
            .expect("observation commit"),
        OperationCustodyCommitOutcome::Committed
    );
    let latest = custody
        .read_index(
            observation.store(),
            &OperationCustodyIndexQuery::LatestObservation(key('2')),
            16 * 1024,
        )
        .expect("latest observation");
    assert_eq!(
        latest.record().expect("observation record").coordinate(),
        &coordinate("observation-one-2", '4')
    );
    assert_eq!(
        custody
            .compare_and_commit(&start)
            .expect_err("stale start cannot replay")
            .kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    let existing = custody
        .read_index(
            start.store(),
            &OperationCustodyIndexQuery::RequestOperation(key('1')),
            16 * 1024,
        )
        .expect("existing request operation");
    assert_eq!(
        existing
            .record()
            .expect("existing operation handle")
            .coordinate(),
        &coordinate("handle-one", '2')
    );
}

#[test]
fn missing_latest_observation_predecessor_requires_recovery() {
    let mut connection = Connection::open_in_memory().expect("in-memory SQLite");
    initialize_store(&connection);
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let predecessor = start.records()[2].record().clone();
    let observation = observation_candidate(predecessor, "one", '4');
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    custody.compare_and_commit(&start).expect("durable start");
    drop(custody);
    connection
        .execute(
            "DELETE FROM pale_deployment_operation_index \
             WHERE index_role = 'latest_observation' AND index_key = ?1",
            [key('2').as_str()],
        )
        .expect("remove latest-observation predecessor");
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };

    assert_eq!(
        custody.observe_exact(&observation).expect("read-back"),
        OperationCustodyReadBackOutcome::ConflictingOrPartial
    );
    assert_eq!(
        custody
            .compare_and_commit(&observation)
            .expect_err("missing exact predecessor index is partial state")
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn mismatched_latest_observation_predecessor_requires_recovery() {
    let mut connection = Connection::open_in_memory().expect("in-memory SQLite");
    initialize_store(&connection);
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let predecessor = start.records()[2].record().clone();
    let observation = observation_candidate(predecessor, "one", '4');
    let mismatched = record(
        "observation-another-lineage",
        '7',
        OPERATION_OBSERVATION_V1_SCHEMA,
    );
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    custody.compare_and_commit(&start).expect("durable start");
    drop(custody);
    insert_record(&connection, &mismatched);
    connection
        .execute(
            "UPDATE pale_deployment_operation_index \
             SET record_ref = ?1, record_sha256 = ?2 \
             WHERE index_role = 'latest_observation' AND index_key = ?3",
            params![
                mismatched.coordinate().record_ref().as_str(),
                mismatched.coordinate().record_sha256().as_str(),
                key('2').as_str(),
            ],
        )
        .expect("replace latest-observation predecessor");
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };

    assert_eq!(
        custody.observe_exact(&observation).expect("read-back"),
        OperationCustodyReadBackOutcome::ConflictingOrPartial
    );
    assert_eq!(
        custody
            .compare_and_commit(&observation)
            .expect_err("mismatched predecessor index contradicts the checkpoint")
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn stale_complete_candidate_is_a_conflict_not_recovery_state() {
    let mut connection = Connection::open_in_memory().expect("in-memory SQLite");
    initialize_store(&connection);
    let first = start_candidate("one", ['1', '2', '3', '9']);
    let stale = start_candidate("two", ['4', '5', '6', 'a']);
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    custody.compare_and_commit(&first).expect("first commit");
    assert_eq!(
        custody
            .compare_and_commit(&stale)
            .expect_err("stale candidate")
            .kind(),
        ControlStoreErrorKind::CandidateConflict
    );
}

#[test]
fn orphaned_candidate_record_requires_recovery() {
    let mut connection = Connection::open_in_memory().expect("in-memory SQLite");
    initialize_store(&connection);
    let candidate = start_candidate("one", ['1', '2', '3', '9']);
    insert_record(&connection, candidate.records()[0].record());
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody
            .compare_and_commit(&candidate)
            .expect_err("partial candidate")
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn checkpoint_successor_with_a_missing_index_requires_recovery() {
    let mut connection = Connection::open_in_memory().expect("in-memory SQLite");
    initialize_store(&connection);
    let candidate = start_candidate("one", ['1', '2', '3', '9']);
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    custody.compare_and_commit(&candidate).expect("commit");
    drop(custody);
    connection
        .execute(
            "DELETE FROM pale_deployment_operation_index \
             WHERE index_role = 'request_evidence' AND index_key = ?1",
            [key('1').as_str()],
        )
        .expect("remove one candidate index");
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };

    assert_eq!(
        custody.observe_exact(&candidate).expect("read-back"),
        OperationCustodyReadBackOutcome::ConflictingOrPartial
    );
    assert_eq!(
        custody
            .compare_and_commit(&candidate)
            .expect_err("partial successor")
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn store_incarnation_is_checked_against_both_custody_and_database_header() {
    let candidate = start_candidate("one", ['1', '2', '3', '9']);

    let mut connection = Connection::open_in_memory().expect("in-memory SQLite");
    initialize_store(&connection);
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: other_store(),
    };
    assert_eq!(
        custody
            .observe_exact(&candidate)
            .expect_err("custody coordinate mismatch")
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    let candidate = OperationCustodyCandidate::new_start(
        other_store(),
        candidate.kind(),
        candidate.records().to_vec(),
        candidate
            .indexes()
            .iter()
            .filter(|index| {
                !matches!(
                    index,
                    OperationCustodyIndexMutation::RequestForHandle { .. }
                        | OperationCustodyIndexMutation::StartObservation { .. }
                        | OperationCustodyIndexMutation::ObservationCheckpoint { .. }
                )
            })
            .cloned()
            .collect(),
        candidate.checkpoint().clone(),
        physical_accumulator(other_store(), candidate.checkpoint()),
        candidate.session_basis().unwrap().clone(),
    )
    .expect("other-incarnation candidate");
    assert_eq!(
        custody
            .observe_exact(&candidate)
            .expect_err("database header mismatch")
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn failed_index_write_rolls_back_records_and_checkpoint() {
    let mut connection = Connection::open_in_memory().expect("in-memory SQLite");
    initialize_store(&connection);
    connection
        .execute_batch(
            "CREATE TEMP TRIGGER reject_operation_index BEFORE INSERT \
             ON pale_deployment_operation_index \
             BEGIN SELECT RAISE(ABORT, 'injected index failure'); END;",
        )
        .expect("failure trigger");
    let candidate = start_candidate("one", ['1', '2', '3', '9']);
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody
            .compare_and_commit(&candidate)
            .expect_err("injected failure")
            .kind(),
        ControlStoreErrorKind::Storage
    );
    drop(custody);

    let counts = connection
        .query_row(
            "SELECT \
             (SELECT COUNT(*) FROM pale_deployment_control_record), \
            (SELECT COUNT(*) FROM pale_deployment_operation_index), \
             (SELECT COUNT(*) FROM pale_deployment_operation_session_basis), \
             (SELECT domain_sequence FROM pale_deployment_operation_checkpoint_head \
              WHERE checkpoint_domain_id = 'operation:principal-1')",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .expect("post-failure state");
    assert_eq!(counts, (1, 0, 0, 8));
}

#[test]
fn commit_failure_reports_unknown_outcome_for_exact_read_back() {
    let mut connection = Connection::open_in_memory().expect("in-memory SQLite");
    initialize_store(&connection);
    connection
        .execute_batch(
            "CREATE TEMP TABLE commit_parent (id INTEGER PRIMARY KEY); \
             CREATE TEMP TABLE commit_child (parent_id INTEGER REFERENCES commit_parent(id) \
                 DEFERRABLE INITIALLY DEFERRED); \
             CREATE TEMP TRIGGER reject_operation_commit AFTER UPDATE \
                 ON pale_deployment_operation_checkpoint_head \
             BEGIN INSERT INTO commit_child(parent_id) VALUES (1); END;",
        )
        .expect("deferred commit failure injection");
    let candidate = start_candidate("one", ['1', '2', '3', '9']);
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };

    assert_eq!(
        custody
            .compare_and_commit(&candidate)
            .expect_err("ambiguous commit")
            .kind(),
        ControlStoreErrorKind::OutcomeUnknown
    );
    assert_eq!(
        custody
            .observe_exact(&candidate)
            .expect("exact non-mutating read-back"),
        OperationCustodyReadBackOutcome::Predecessor
    );
    let basis = candidate.session_basis().unwrap();
    assert!(custody
        .read_operation_session_basis(
            candidate.store(),
            basis.handle(),
            basis.canonical_basis_jcs().len()
        )
        .unwrap()
        .is_none());
}

#[test]
fn operation_lease_body_is_exact_bounded_unindexed_and_restart_readable() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("operation-lease.sqlite3");
    let lease = record("operation-lease-1", 'd', OPERATION_LEASE_V1_SCHEMA);
    let body_len = lease.canonical_record_jcs().as_slice().len();

    {
        let mut connection = Connection::open(&path).expect("open SQLite");
        initialize_store(&connection);
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        assert_eq!(
            custody
                .compare_and_retain_operation_lease(&store(), &lease)
                .expect("lease body retains"),
            OperationCustodyCommitOutcome::Committed
        );
        assert_eq!(
            custody
                .compare_and_retain_operation_lease(&store(), &lease)
                .expect("exact lease replay"),
            OperationCustodyCommitOutcome::ExactReplay
        );
        assert_eq!(
            custody
                .read_operation_record(&store(), lease.coordinate(), body_len)
                .expect("bounded lease read"),
            Some(lease.clone())
        );
        assert_eq!(
            custody
                .read_operation_record(&store(), lease.coordinate(), body_len - 1)
                .expect_err("body above caller ceiling")
                .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        let wrong_digest = ControlRecordCoordinate::new(
            lease.coordinate().record_ref().clone(),
            ControlRecordDigest::parse(digest('e')).expect("wrong digest"),
        );
        assert_eq!(
            custody
                .read_operation_record(&store(), &wrong_digest, body_len)
                .expect_err("reference cannot be rebound to another digest")
                .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        let conflicting = record("operation-lease-1", 'e', OPERATION_LEASE_V1_SCHEMA);
        assert_eq!(
            custody
                .compare_and_retain_operation_lease(&store(), &conflicting)
                .expect_err("immutable reference reuse")
                .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        assert_eq!(
            custody
                .compare_and_retain_operation_lease(&other_store(), &lease)
                .expect_err("other store incarnation")
                .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
    }

    let mut connection = Connection::open(&path).expect("reopen SQLite");
    let counts = connection
        .query_row(
            "SELECT \
             (SELECT COUNT(*) FROM pale_deployment_operation_index), \
             (SELECT domain_sequence FROM pale_deployment_operation_checkpoint_head \
              WHERE checkpoint_domain_id = 'operation:principal-1')",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .expect("operation authority rows");
    assert_eq!(counts, (0, 8), "lease retention creates no operation start");
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody
            .read_operation_record(&store(), lease.coordinate(), body_len)
            .expect("restart read"),
        Some(lease.clone())
    );
    drop(custody);

    connection
        .execute(
            "UPDATE pale_deployment_control_record SET schema_id = ?1 WHERE record_ref = ?2",
            params!["é".repeat(300), lease.coordinate().record_ref().as_str()],
        )
        .expect("inject metadata whose character and byte bounds differ");
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody
            .read_operation_record(&store(), lease.coordinate(), body_len)
            .expect_err("schema metadata exceeds its byte ceiling")
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[path = "operation/consult_child_service_restart.rs"]
mod consult_child_service_restart;
