//! Physical relation closure only; opaque fixture records prove no task meaning.

use super::*;
use pale_ports::{ControlTransactionPort, OperationA2aDependencies, OperationCustodyRecordKind};

fn local_after_binding(start: &OperationCustodyCandidate) -> OperationCustodyCandidate {
    let template = observation_candidate(
        start
            .record(OperationCustodyRecordKind::OperationObservation)
            .unwrap()
            .clone(),
        "one",
        '4',
    );
    let checkpoint = checkpoint_transition(
        10,
        "checkpoint-10-task",
        'a',
        11,
        "checkpoint-11-local",
        'e',
    );
    OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        template.records().to_vec(),
        template
            .indexes()
            .iter()
            .filter(|index| {
                matches!(
                    index,
                    OperationCustodyIndexMutation::LatestObservation { .. }
                )
            })
            .cloned()
            .collect(),
        checkpoint.clone(),
        physical_accumulator(store(), &checkpoint),
    )
    .unwrap()
}

#[test]
fn relation_only_partial_is_not_an_unrelated_winner_after_head_advance() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let selected = session_basis::a2a_successor(&start);
    let local = observation_candidate(
        start
            .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
            .unwrap()
            .clone(),
        "one",
        '4',
    );
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    port.compare_and_commit(&start).unwrap();
    port.compare_and_commit(&local).unwrap();
    assert_eq!(
        port.compare_and_commit(&selected).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    let (operation_key, binding_key, binding) = selected
        .indexes()
        .iter()
        .find_map(|index| match index {
            OperationCustodyIndexMutation::A2aTaskBinding {
                operation_key,
                key,
                successor,
                ..
            } => Some((operation_key, key, successor)),
            _ => None,
        })
        .unwrap();
    // Model a damaged image, not a permitted writer: no binding/body/index/raw
    // rows exist. Turning FK checks off only creates this deliberate orphan.
    port.connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    port.connection.execute("INSERT INTO pale_deployment_operation_a2a_binding(operation_key,binding_key,record_ref,record_sha256,inserted_transaction_sequence) VALUES (?1,?2,?3,?4,?5)", params![operation_key.as_str(),binding_key.as_str(),binding.record_ref().as_str(),binding.record_sha256().as_str(),i64::try_from(selected.accumulator().binding().transaction().successor().get()).unwrap()]).unwrap();
    port.connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    let writes = port.connection.total_changes();
    let transaction = port.prepare_transaction(selected.store()).unwrap();
    assert_eq!(
        port.compare_and_commit(&selected).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        port.observe_exact(&selected).unwrap(),
        OperationCustodyReadBackOutcome::ConflictingOrPartial
    );
    assert_eq!(
        port.prepare_transaction(selected.store()).unwrap(),
        transaction
    );
    assert_eq!(port.connection.total_changes(), writes);
}

#[test]
fn complete_competing_binding_is_conflict_but_missing_winner_relation_requires_recovery() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let selected = session_basis::a2a_successor(&start);
    let winner = session_basis::a2a_successor_named(&start, "other-task", key('e'));
    assert_eq!(
        selected.accumulator().binding().transaction(),
        winner.accumulator().binding().transaction()
    );
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    port.compare_and_commit(&start).unwrap();
    assert_eq!(
        port.compare_and_commit(&winner).unwrap(),
        OperationCustodyCommitOutcome::Committed
    );
    assert_eq!(
        port.observe_exact(&winner).unwrap(),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    for record in selected.records() {
        assert!(
            port.read_operation_record(selected.store(), record.record().coordinate(), 4096)
                .unwrap()
                .is_none(),
            "distinct loser has no signed candidate material"
        );
    }
    for raw in selected.a2a_evidence() {
        assert!(port
            .read_a2a_evidence(selected.store(), raw.coordinate(), 4096)
            .unwrap()
            .is_none());
    }
    let transaction = port.prepare_transaction(selected.store()).unwrap();
    let writes = port.connection.total_changes();
    assert_eq!(
        port.compare_and_commit(&selected).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict,
        "a complete different Bind atom at the proposed T is an ordinary winner"
    );
    assert_eq!(port.connection.total_changes(), writes);
    assert_eq!(
        port.prepare_transaction(selected.store()).unwrap(),
        transaction
    );
    let binding = winner
        .record(OperationCustodyRecordKind::A2aTaskBinding)
        .unwrap();
    let reference = binding.coordinate().record_ref().as_str();
    let (operation_key, binding_key, digest, stamp): (String, String, String, i64) = port.connection.query_row("SELECT operation_key,binding_key,record_sha256,inserted_transaction_sequence FROM pale_deployment_operation_a2a_binding WHERE record_ref=?1", [reference], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap();
    assert_eq!(
        port.connection
            .execute(
                "DELETE FROM pale_deployment_operation_a2a_binding WHERE record_ref=?1",
                [reference]
            )
            .unwrap(),
        1
    );
    let writes = port.connection.total_changes();
    assert_eq!(
        port.compare_and_commit(&selected).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired,
        "the same loser cannot classify damaged winning custody as an ordinary conflict"
    );
    assert_eq!(port.connection.total_changes(), writes);
    assert_eq!(
        port.prepare_transaction(selected.store()).unwrap(),
        transaction
    );
    port.connection.execute("INSERT INTO pale_deployment_operation_a2a_binding(operation_key,binding_key,record_ref,record_sha256,inserted_transaction_sequence) VALUES (?1,?2,?3,?4,?5)", params![operation_key,binding_key,reference,digest,stamp]).unwrap();
    let writes = port.connection.total_changes();
    assert_eq!(
        port.compare_and_commit(&selected).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    assert_eq!(port.connection.total_changes(), writes);
}

#[test]
fn local_observation_cannot_move_a_retained_binding_relation_into_its_atom() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let binding = session_basis::a2a_successor(&start);
    let selected = local_after_binding(&start);
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    port.compare_and_commit(&start).unwrap();
    port.compare_and_commit(&binding).unwrap();
    let stamp: i64 = port
        .connection
        .query_row(
            "SELECT inserted_transaction_sequence FROM pale_deployment_operation_a2a_binding",
            [],
            |row| row.get(0),
        )
        .unwrap();
    // The referenced real binding remains present and every FK still holds.
    // Only the transaction's closed relation-membership check can reject this.
    port.connection.execute_batch("CREATE TEMP TRIGGER move_relation AFTER UPDATE ON pale_deployment_operation_checkpoint_head BEGIN UPDATE pale_deployment_operation_a2a_binding SET inserted_transaction_sequence=(SELECT inserted_transaction_sequence FROM pale_deployment_control_record WHERE record_ref=NEW.control_checkpoint_ref); END;").unwrap();
    assert_eq!(
        port.compare_and_commit(&selected).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        port.observe_exact(&selected).unwrap(),
        OperationCustodyReadBackOutcome::Predecessor
    );
    let after: i64 = port
        .connection
        .query_row(
            "SELECT inserted_transaction_sequence FROM pale_deployment_operation_a2a_binding",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        after, stamp,
        "relation move rolls back with the rejected atom"
    );
    port.connection
        .execute_batch("DROP TRIGGER move_relation")
        .unwrap();
    assert_eq!(
        port.compare_and_commit(&selected).unwrap(),
        OperationCustodyCommitOutcome::Committed
    );
    assert_eq!(
        port.compare_and_commit(&selected).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
}

#[test]
fn stale_uncommitted_task_observation_still_requires_its_retained_relation() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let binding = session_basis::a2a_successor(&start);
    let body = binding
        .record(OperationCustodyRecordKind::A2aTaskBinding)
        .unwrap();
    let prior = binding
        .record(OperationCustodyRecordKind::A2aTaskObservation)
        .unwrap();
    let next = record(
        "task-observation-losing",
        'f',
        pale_core::records::A2A_TASK_OBSERVATION_V1_SCHEMA,
    );
    let checkpoint = checkpoint_transition(
        10,
        "checkpoint-10-task",
        'a',
        11,
        "checkpoint-11-losing",
        'f',
    );
    let (operation_key, binding_key) = binding
        .indexes()
        .iter()
        .find_map(|index| match index {
            OperationCustodyIndexMutation::A2aTaskBinding {
                operation_key, key, ..
            } => Some((operation_key.clone(), key.clone())),
            _ => None,
        })
        .unwrap();
    let mut retained = binding.a2a_retained_records().to_vec();
    retained.push(body.clone());
    retained.push(
        binding
            .record(OperationCustodyRecordKind::A2aRouteAdmission)
            .unwrap()
            .clone(),
    );
    let selected = OperationCustodyCandidate::new_a2a_custody(
        store(),
        OperationCustodyTransitionKind::ObserveA2aTask,
        vec![OperationCustodyRecord::A2aTaskObservation(next.clone())],
        vec![
            OperationCustodyIndexMutation::A2aTaskBinding {
                operation_key: operation_key.clone(),
                key: binding_key.clone(),
                expected: OperationCustodyIndexPredecessor::Exact(body.clone()),
                successor: body.coordinate().clone(),
            },
            OperationCustodyIndexMutation::LatestA2aTaskObservation {
                key: binding_key.clone(),
                expected: OperationCustodyIndexPredecessor::Exact(prior.clone()),
                successor: next.coordinate().clone(),
            },
        ],
        checkpoint.clone(),
        physical_accumulator(store(), &checkpoint),
        OperationA2aDependencies {
            new_evidence: vec![],
            retained_evidence: binding.a2a_evidence().to_vec(),
            retained_records: retained,
            associated_route: None,
        },
    )
    .unwrap();
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    port.compare_and_commit(&start).unwrap();
    port.compare_and_commit(&binding).unwrap();
    port.compare_and_commit(&local_after_binding(&start))
        .unwrap();
    assert_eq!(
        port.compare_and_commit(&selected).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    assert!(
        port.read_operation_record(selected.store(), next.coordinate(), 4096)
            .unwrap()
            .is_none(),
        "the losing candidate has no own material"
    );
    let stamp: i64 = port.connection.query_row("SELECT inserted_transaction_sequence FROM pale_deployment_operation_a2a_binding WHERE record_ref=?1", [body.coordinate().record_ref().as_str()], |row| row.get(0)).unwrap();
    port.connection
        .execute(
            "DELETE FROM pale_deployment_operation_a2a_binding WHERE record_ref=?1",
            [body.coordinate().record_ref().as_str()],
        )
        .unwrap();
    let writes = port.connection.total_changes();
    assert_eq!(
        port.compare_and_commit(&selected).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired,
        "staleness cannot hide a missing retained dependency"
    );
    assert_eq!(port.connection.total_changes(), writes);
    port.connection.execute("INSERT INTO pale_deployment_operation_a2a_binding(operation_key,binding_key,record_ref,record_sha256,inserted_transaction_sequence) VALUES (?1,?2,?3,?4,?5)", params![operation_key.as_str(), binding_key.as_str(), body.coordinate().record_ref().as_str(), body.coordinate().record_sha256().as_str(), stamp]).unwrap();
    assert_eq!(
        port.compare_and_commit(&selected).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
}
