//! Physical `GetTask` preparation custody and exact point-read recovery.

use super::*;
use pale_core::records::{
    ImmutableRecordReferenceV1, A2A_ROUTE_ADMISSION_V1_SCHEMA, A2A_TASK_BINDING_V1_SCHEMA,
    A2A_TASK_OBSERVATION_V1_SCHEMA,
};
use pale_ports::{
    OperationA2aDependencies, OperationA2aEvidence,
    PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA,
};

fn get_task_candidate() -> (OperationCustodyCandidate, ControlRecord) {
    let preparation = record(
        "get-task-preparation",
        '7',
        PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA,
    );
    let checkpoint = checkpoint_transition(8, "checkpoint-8", '8', 9, "checkpoint-get-task-9", '9');
    let bytes = b"get-task-request".to_vec();
    let evidence = OperationA2aEvidence::new(
        ImmutableRecordReferenceV1 {
            record_ref: ResidentRecordRefV1::parse("get-task-request").unwrap(),
            record_sha256: ArtifactDigest::hash_bytes(&bytes),
        },
        bytes,
        None,
        16,
    )
    .unwrap();
    let candidate = OperationCustodyCandidate::new_a2a_custody(
        store(),
        OperationCustodyTransitionKind::PrepareA2aGetTask,
        vec![OperationCustodyRecord::A2aGetTaskPreparation(
            preparation.clone(),
        )],
        vec![OperationCustodyIndexMutation::CurrentA2aGetTask {
            key: key('a'),
            expected: OperationCustodyIndexPredecessor::Absent,
            successor: preparation.coordinate().clone(),
        }],
        checkpoint.clone(),
        physical_accumulator(store(), &checkpoint),
        OperationA2aDependencies {
            new_evidence: vec![evidence],
            retained_evidence: Vec::new(),
            retained_records: vec![
                record("get-task-handle", '1', OPERATION_HANDLE_V1_SCHEMA),
                record("get-task-dispatch", '2', OPERATION_OBSERVATION_V1_SCHEMA),
                record("get-task-route", '3', A2A_ROUTE_ADMISSION_V1_SCHEMA),
                record("get-task-binding", '4', A2A_TASK_BINDING_V1_SCHEMA),
                record("get-task-remote-head", '5', A2A_TASK_OBSERVATION_V1_SCHEMA),
            ],
            associated_route: None,
        },
    )
    .unwrap();
    (candidate, preparation)
}

fn seed_retained_parents(connection: &Connection, candidate: &OperationCustodyCandidate) {
    for record in candidate.a2a_retained_records() {
        insert_record(connection, record);
    }
}

#[test]
fn get_task_preparation_and_checkpoint_are_atomic_restart_readable_point_indexes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("get-task.sqlite3");
    let (candidate, preparation) = get_task_candidate();
    let checkpoint_key =
        OperationCustodyIndexKey::for_a2a_get_task_checkpoint(preparation.coordinate());
    {
        let mut connection = Connection::open(&path).unwrap();
        initialize_store(&connection);
        seed_retained_parents(&connection, &candidate);
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        assert_eq!(
            custody.compare_and_commit(&candidate).unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
        assert_eq!(
            custody.compare_and_commit(&candidate).unwrap(),
            OperationCustodyCommitOutcome::ExactReplay
        );
    }
    let mut connection = Connection::open(&path).unwrap();
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    let current = custody
        .read_index(
            &store(),
            &OperationCustodyIndexQuery::CurrentA2aGetTask(key('a')),
            16 * 1024,
        )
        .unwrap()
        .into_record()
        .unwrap();
    assert_eq!(current.coordinate(), preparation.coordinate());
    let checkpoint = custody
        .read_index(
            &store(),
            &OperationCustodyIndexQuery::A2aGetTaskCheckpoint(checkpoint_key),
            16 * 1024,
        )
        .unwrap()
        .into_record()
        .unwrap();
    assert_eq!(
        checkpoint.coordinate(),
        candidate.checkpoint().successor_record().coordinate()
    );
}

#[test]
fn failed_get_task_index_write_retains_no_preparation_or_checkpoint_index() {
    let (candidate, preparation) = get_task_candidate();
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    seed_retained_parents(&connection, &candidate);
    connection
        .execute_batch(
            "CREATE TEMP TRIGGER reject_get_task_index BEFORE INSERT \
             ON pale_deployment_operation_index \
             BEGIN SELECT RAISE(ABORT, 'injected get-task index failure'); END;",
        )
        .unwrap();
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::Storage
    );
    assert!(custody
        .read_index(
            &store(),
            &OperationCustodyIndexQuery::CurrentA2aGetTask(key('a')),
            16 * 1024,
        )
        .unwrap()
        .record()
        .is_none());
    assert!(custody
        .read_index(
            &store(),
            &OperationCustodyIndexQuery::A2aGetTaskCheckpoint(
                OperationCustodyIndexKey::for_a2a_get_task_checkpoint(preparation.coordinate()),
            ),
            16 * 1024,
        )
        .unwrap()
        .record()
        .is_none());
    assert_eq!(
        custody.observe_exact(&candidate).unwrap(),
        OperationCustodyReadBackOutcome::Predecessor
    );
}
