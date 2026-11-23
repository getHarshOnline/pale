//! Physical attachment conformance. Opaque fixture bytes do not prove admission.

use super::*;
use pale_ports::{OperationA2aDependencies, OperationA2aEvidence, OperationCustodyRecordKind};

#[test]
fn exact_session_basis_is_same_transaction_bounded_and_reopen_readable() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("session-basis.sqlite");
    let candidate = start_candidate("one", ['1', '2', '3', '9']);
    let basis = candidate.session_basis().unwrap();
    {
        let mut connection = Connection::open(&path).unwrap();
        initialize_store(&connection);
        SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .compare_and_commit(&candidate)
        .unwrap();
        let stamps: (i64,i64,i64,i64) = connection.query_row(
            "SELECT b.inserted_transaction_sequence,h.inserted_transaction_sequence,r.inserted_transaction_sequence,c.inserted_transaction_sequence \
             FROM pale_deployment_operation_session_basis b JOIN pale_deployment_control_record h ON h.record_ref=b.handle_ref \
             JOIN pale_deployment_control_record r ON r.record_ref=b.request_ref JOIN pale_deployment_control_record c ON c.record_ref=?1 WHERE b.handle_ref=?2",
            params![candidate.checkpoint().successor().coordinate().record_ref().as_str(), basis.handle().record_ref().as_str()],
            |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
        ).unwrap();
        let expected = i64::try_from(
            candidate
                .accumulator()
                .binding()
                .transaction()
                .successor()
                .get(),
        )
        .unwrap();
        assert_eq!(stamps, (expected, expected, expected, expected));
    }
    let mut connection = Connection::open(&path).unwrap();
    connection.pragma_update(None, "query_only", true).unwrap();
    let writes = connection.total_changes();
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        port.read_operation_session_basis(
            candidate.store(),
            basis.handle(),
            basis.canonical_basis_jcs().len()
        )
        .unwrap()
        .as_ref(),
        Some(basis)
    );
    for limit in [0, basis.canonical_basis_jcs().len() - 1] {
        assert_eq!(
            port.read_operation_session_basis(candidate.store(), basis.handle(), limit)
                .unwrap_err()
                .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
    }
    assert_eq!(
        port.read_operation_session_basis(&other_store(), basis.handle(), 4096)
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    let unlike_digest = ControlRecordCoordinate::new(
        basis.handle().record_ref().clone(),
        ControlRecordDigest::parse(digest('f')).unwrap(),
    );
    assert_eq!(
        port.read_operation_session_basis(candidate.store(), &unlike_digest, 4096)
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        port.observe_exact(&candidate).unwrap(),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    // Even exact compare-and-commit opens IMMEDIATE, which query_only forbids.
    // Its successful replay must still leave the database unchanged.
    port.connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    assert_eq!(
        port.compare_and_commit(&candidate).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(connection.total_changes(), writes);
}

#[test]
fn missing_changed_oversized_or_retimed_basis_never_replays_or_repairs() {
    for (mutation, missing) in [
        ("DELETE FROM pale_deployment_operation_session_basis", true),
        (
            "UPDATE pale_deployment_operation_session_basis SET canonical_basis_jcs=x'7b7d'",
            false,
        ),
        (
            "UPDATE pale_deployment_operation_session_basis SET basis_sha256=printf('%064d',0)",
            false,
        ),
        (
            "UPDATE pale_deployment_operation_session_basis SET inserted_transaction_sequence=8",
            false,
        ),
        (
            "PRAGMA ignore_check_constraints=ON; UPDATE pale_deployment_operation_session_basis SET canonical_basis_jcs=zeroblob(65537)",
            false,
        ),
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
        connection.execute_batch(mutation).unwrap();
        let writes = connection.total_changes();
        let mut port = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        let basis = candidate.session_basis().unwrap();
        let read = port.read_operation_session_basis(candidate.store(), basis.handle(), usize::MAX);
        if missing {
            assert!(read.unwrap().is_none());
        } else {
            assert_eq!(
                read.unwrap_err().kind(),
                ControlStoreErrorKind::RecoveryRequired,
                "{mutation}"
            );
        }
        assert_eq!(
            port.observe_exact(&candidate).unwrap(),
            OperationCustodyReadBackOutcome::ConflictingOrPartial,
            "{mutation}"
        );
        assert_eq!(
            port.compare_and_commit(&candidate).unwrap_err().kind(),
            ControlStoreErrorKind::RecoveryRequired,
            "{mutation}"
        );
        assert_eq!(connection.total_changes(), writes);
    }
}

#[test]
fn internally_consistent_substitution_is_not_the_selected_basis() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let candidate = start_candidate("one", ['1', '2', '3', '9']);
    SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .compare_and_commit(&candidate)
    .unwrap();
    let original = candidate.session_basis().unwrap();
    let changed = pale_ports::OperationSessionBasis::new(
        original.handle().clone(),
        original.request().clone(),
        original.session_lease_digest().clone(),
        br#"{"physical_fixture":"substituted-basis"}"#.to_vec(),
        4096,
    )
    .unwrap();
    connection.execute("UPDATE pale_deployment_operation_session_basis SET canonical_basis_jcs=?1,basis_sha256=?2",
        params![changed.canonical_basis_jcs(),changed.basis_sha256().as_str()]).unwrap();
    let writes = connection.total_changes();
    let mut port = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    // Point read is physical only; protected application recovery still needs membership.
    assert_eq!(
        port.read_operation_session_basis(candidate.store(), original.handle(), 4096)
            .unwrap()
            .as_ref(),
        Some(&changed)
    );
    assert_eq!(
        port.observe_exact(&candidate).unwrap(),
        OperationCustodyReadBackOutcome::ConflictingOrPartial
    );
    assert_eq!(
        port.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(connection.total_changes(), writes);
}

// This is the same opaque physical accumulator fixture, not remote-task admission.
pub(super) fn a2a_successor(start: &OperationCustodyCandidate) -> OperationCustodyCandidate {
    a2a_successor_named(start, "task", key('b'))
}

pub(super) fn a2a_successor_named(
    start: &OperationCustodyCandidate,
    namespace: &str,
    binding_key: OperationCustodyIndexKey,
) -> OperationCustodyCandidate {
    let binding = record(
        &format!("{namespace}-binding"),
        'b',
        pale_core::records::A2A_TASK_BINDING_V1_SCHEMA,
    );
    let observation = record(
        &format!("{namespace}-observation"),
        'c',
        pale_core::records::A2A_TASK_OBSERVATION_V1_SCHEMA,
    );
    let checkpoint = checkpoint_transition(
        9,
        "checkpoint-9-one",
        '9',
        10,
        &format!("checkpoint-10-{namespace}"),
        'a',
    );
    let bytes = b"opaque protocol fixture".to_vec();
    OperationCustodyCandidate::new_a2a_custody(
        store(),
        OperationCustodyTransitionKind::BindA2aTask,
        vec![
            OperationCustodyRecord::A2aRouteAdmission(record(
                &format!("{namespace}-route"),
                'd',
                pale_core::records::A2A_ROUTE_ADMISSION_V1_SCHEMA,
            )),
            OperationCustodyRecord::A2aTaskBinding(binding.clone()),
            OperationCustodyRecord::A2aTaskObservation(observation.clone()),
        ],
        vec![
            OperationCustodyIndexMutation::A2aTaskBinding {
                operation_key: start
                    .indexes()
                    .iter()
                    .find_map(|index| match index {
                        OperationCustodyIndexMutation::OperationHandle { key, .. } => {
                            Some(key.clone())
                        }
                        _ => None,
                    })
                    .unwrap(),
                key: binding_key.clone(),
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: binding.coordinate().clone(),
            },
            OperationCustodyIndexMutation::LatestA2aTaskObservation {
                key: binding_key,
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: observation.coordinate().clone(),
            },
        ],
        checkpoint.clone(),
        physical_accumulator(store(), &checkpoint),
        OperationA2aDependencies {
            new_evidence: vec![OperationA2aEvidence::new(
                pale_core::records::ImmutableRecordReferenceV1 {
                    record_ref: ResidentRecordRefV1::parse(format!("{namespace}-raw")).unwrap(),
                    record_sha256: ArtifactDigest::hash_bytes(&bytes),
                },
                bytes,
                None,
                4096,
            )
            .unwrap()],
            retained_evidence: vec![],
            associated_route: None,
            retained_records: vec![
                start
                    .record(OperationCustodyRecordKind::OperationHandle)
                    .unwrap()
                    .clone(),
                start
                    .record(OperationCustodyRecordKind::OperationObservation)
                    .unwrap()
                    .clone(),
            ],
        },
    )
    .unwrap()
}

#[test]
fn observe_and_remote_history_cannot_inject_a_session_basis() {
    for remote in [false, true] {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_store(&connection);
        let start = start_candidate("one", ['1', '2', '3', '9']);
        SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .compare_and_commit(&start)
        .unwrap();
        let successor = if remote {
            a2a_successor(&start)
        } else {
            observation_candidate(
                start
                    .record(OperationCustodyRecordKind::OperationObservation)
                    .unwrap()
                    .clone(),
                "one",
                '4',
            )
        };
        assert!(successor.session_basis().is_none());
        // Both swapped coordinates exist: foreign-key failure cannot satisfy this test.
        connection.execute_batch("CREATE TEMP TRIGGER inject_basis AFTER UPDATE ON pale_deployment_operation_checkpoint_head BEGIN \
            INSERT INTO pale_deployment_operation_session_basis SELECT request_ref,request_sha256,handle_ref,handle_sha256,session_lease_sha256,basis_sha256,canonical_basis_jcs, \
            (SELECT inserted_transaction_sequence FROM pale_deployment_control_record WHERE record_ref=NEW.control_checkpoint_ref) FROM pale_deployment_operation_session_basis; END;").unwrap();
        let mut port = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        assert_eq!(
            port.compare_and_commit(&successor).unwrap_err().kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        assert_eq!(
            port.observe_exact(&successor).unwrap(),
            OperationCustodyReadBackOutcome::Predecessor
        );
        assert_eq!(
            port.observe_exact(&start).unwrap(),
            OperationCustodyReadBackOutcome::ExactSuccessor
        );
        port.connection
            .execute_batch("DROP TRIGGER inject_basis")
            .unwrap();
        assert_eq!(
            port.compare_and_commit(&successor).unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
        let basis = start.session_basis().unwrap();
        assert_eq!(
            port.read_operation_session_basis(start.store(), basis.handle(), 4096)
                .unwrap()
                .as_ref(),
            Some(basis)
        );
        assert_eq!(
            port.connection
                .query_row(
                    "SELECT COUNT(*) FROM pale_deployment_operation_session_basis",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
    }
}
