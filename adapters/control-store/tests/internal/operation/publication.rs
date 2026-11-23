//! Physical discovery of operation sources; not signature or Event admission.

use super::*;
use pale_ports::{
    OperationPublicationCursor, OperationPublicationReadError, OperationPublicationSourceKind,
    OperationPublicationSources, MAX_OPERATION_PUBLICATION_BODY_BYTES,
    MAX_OPERATION_PUBLICATION_SOURCES,
};

fn read(
    connection: &mut Connection,
    cursor: Option<&OperationPublicationCursor>,
    count: usize,
    bytes: usize,
) -> Result<OperationPublicationSources, OperationPublicationReadError> {
    SqliteOperationCustody {
        connection,
        coordinate: store(),
    }
    .read_publication_sources(&store(), cursor, count, bytes)
}

fn commit_pair(
    connection: &mut Connection,
) -> (OperationCustodyCandidate, OperationCustodyCandidate) {
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let append = observation_candidate(start.records()[2].record().clone(), "one", '4');
    let mut custody = SqliteOperationCustody {
        connection,
        coordinate: store(),
    };
    custody.compare_and_commit(&start).unwrap();
    custody.compare_and_commit(&append).unwrap();
    (start, append)
}

#[test]
fn initial_and_successor_sources_survive_restart_without_another_marker() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("publication.sqlite");
    let (start, append) = {
        let mut connection = Connection::open(&path).unwrap();
        initialize_store(&connection);
        commit_pair(&mut connection)
    };
    let mut connection = Connection::open(&path).unwrap();
    let before = connection.total_changes();
    let page = read(&mut connection, None, 8, 4096).unwrap();
    assert!(page.continuation.is_none());
    assert_eq!(
        page.through(),
        append
            .checkpoint()
            .successor()
            .committed_transaction_sequence()
    );
    assert_eq!(page.sources.len(), 2);
    for (source, candidate) in page.sources.iter().zip([&start, &append]) {
        assert_eq!(
            source.kind(),
            OperationPublicationSourceKind::OperationObservation
        );
        assert_eq!(
            source.observation(),
            candidate
                .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
                .unwrap()
        );
        assert_eq!(
            source.checkpoint(),
            candidate.checkpoint().successor_record()
        );
        assert_eq!(
            source.transaction(),
            candidate
                .checkpoint()
                .successor()
                .committed_transaction_sequence()
        );
    }
    assert_eq!(connection.total_changes(), before);
}

#[test]
fn mixed_family_coordinate_merge_orders_same_transaction_ties_and_bounds_only_returned_bodies() {
    use pale_core::records::A2A_TASK_OBSERVATION_V1_SCHEMA;
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let (start, append) = commit_pair(&mut connection);
    let checkpoint = append.checkpoint().successor_record();
    let remote_first = record("a-remote-observation", 'c', A2A_TASK_OBSERVATION_V1_SCHEMA);
    let remote_last = record("z-remote-observation", 'd', A2A_TASK_OBSERVATION_V1_SCHEMA);
    // Physical-reader fixture only: explicitly seed same-transaction siblings
    // to exercise tie order that current one-observation producers do not emit.
    // Contextually admitted A2A custody and restart are covered by the native journey.
    for remote in [&remote_first, &remote_last] {
        insert_record(&connection, remote);
        connection.execute(
            "INSERT INTO pale_deployment_operation_index(index_role,index_key,record_ref,record_sha256) VALUES ('a2a_task_observation_checkpoint',?1,?2,?3)",
            params![OperationCustodyIndexKey::for_observation(remote.coordinate()).as_str(), checkpoint.coordinate().record_ref().as_str(), checkpoint.coordinate().record_sha256().as_str()],
        ).unwrap();
    }
    let expected = [
        start.records()[2].record(),
        &remote_first,
        append.records()[0].record(),
        &remote_last,
    ];
    let kinds = [
        OperationPublicationSourceKind::OperationObservation,
        OperationPublicationSourceKind::A2aTaskObservation,
        OperationPublicationSourceKind::OperationObservation,
        OperationPublicationSourceKind::A2aTaskObservation,
    ];
    let first_bytes = expected[0].canonical_record_jcs().as_slice().len()
        + start
            .checkpoint()
            .successor_record()
            .canonical_record_jcs()
            .as_slice()
            .len();
    let first = read(&mut connection, None, 1, first_bytes).unwrap();
    assert_eq!(first.sources[0].observation(), expected[0]);
    assert!(
        first.continuation.is_some(),
        "look-ahead must not consume body budget"
    );
    assert_eq!(
        first.through(),
        append
            .checkpoint()
            .successor()
            .committed_transaction_sequence()
    );
    for count in [1, 2, 3, 4] {
        let mut actual = Vec::new();
        let mut cursor = None;
        for _ in 0..expected.len() {
            let page = read(&mut connection, cursor.as_ref(), count, 4096).unwrap();
            assert!(page.sources.len() <= count);
            actual.extend(page.sources);
            cursor = page.continuation;
            if cursor.is_none() {
                break;
            }
        }
        assert!(cursor.is_none());
        assert_eq!(actual.len(), expected.len());
        for ((source, expected), kind) in actual.iter().zip(expected).zip(kinds) {
            assert_eq!(source.observation(), expected);
            assert_eq!(source.kind(), kind);
        }
        assert!(actual[1..].iter().all(|source| {
            source.transaction()
                == append
                    .checkpoint()
                    .successor()
                    .committed_transaction_sequence()
        }));
    }
}

#[test]
fn failed_source_transaction_exposes_neither_observation_nor_obligation() {
    for successor in [false, true] {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_store(&connection);
        let start = start_candidate("one", ['1', '2', '3', '9']);
        let candidate = if successor {
            SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store(),
            }
            .compare_and_commit(&start)
            .unwrap();
            observation_candidate(start.records()[2].record().clone(), "one", '4')
        } else {
            start
        };
        connection.execute_batch("CREATE TRIGGER abort_source BEFORE INSERT ON pale_deployment_operation_index WHEN NEW.index_role='observation_checkpoint' BEGIN SELECT RAISE(ABORT,'source join'); END;").unwrap();
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        assert!(custody.compare_and_commit(&candidate).is_err());
        assert_eq!(
            custody.observe_exact(&candidate).unwrap(),
            OperationCustodyReadBackOutcome::Predecessor
        );
        assert_eq!(
            read(&mut connection, None, 8, 4096).unwrap().sources.len(),
            usize::from(successor)
        );
        connection
            .execute_batch("DROP TRIGGER abort_source")
            .unwrap();
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        custody.compare_and_commit(&candidate).unwrap();
        assert_eq!(
            custody.compare_and_commit(&candidate).unwrap(),
            OperationCustodyCommitOutcome::ExactReplay
        );
        assert_eq!(
            read(&mut connection, None, 8, 4096).unwrap().sources.len(),
            usize::from(successor) + 1
        );
    }
}

#[test]
fn missing_malformed_or_different_transaction_checkpoint_fails_the_whole_page() {
    for corruption in [
        "DELETE FROM pale_deployment_operation_index WHERE index_role='observation_checkpoint'",
        "UPDATE pale_deployment_operation_index SET record_ref='checkpoint-8', record_sha256=replace(hex(zeroblob(32)),'0','8') WHERE index_role='observation_checkpoint'",
        "UPDATE pale_deployment_control_record SET inserted_transaction_sequence=28 WHERE record_ref='checkpoint-9-one'",
        "UPDATE pale_deployment_control_record SET schema_id='wrong-schema' WHERE record_ref='checkpoint-9-one'",
        "UPDATE pale_deployment_control_record SET schema_id=printf('%0500d',1)||'ééééééé' WHERE record_ref='checkpoint-9-one'",
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
        let error = read(&mut connection, None, 8, 4096).unwrap_err();
        assert!(
            matches!(error, OperationPublicationReadError::Storage(ref error) if error.kind()==ControlStoreErrorKind::RecoveryRequired),
            "{corruption}: {error:?}"
        );
        assert_eq!(connection.total_changes(), before);
    }
}

#[test]
fn body_and_count_limits_fail_without_a_partial_success() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    commit_pair(&mut connection);
    let page = read(&mut connection, None, 2, 4096).unwrap();
    let exact: usize = page
        .sources
        .iter()
        .map(|source| {
            source.observation().canonical_record_jcs().as_slice().len()
                + source.checkpoint().canonical_record_jcs().as_slice().len()
        })
        .sum();
    assert_eq!(
        read(&mut connection, None, 2, exact).unwrap().sources.len(),
        2
    );
    for (count, bytes) in [
        (0, exact),
        (MAX_OPERATION_PUBLICATION_SOURCES + 1, exact),
        (2, 0),
        (2, MAX_OPERATION_PUBLICATION_BODY_BYTES + 1),
        (2, exact - 1),
    ] {
        assert!(matches!(
            read(&mut connection, None, count, bytes),
            Err(OperationPublicationReadError::Limit)
        ));
    }
    connection.execute("UPDATE pale_deployment_control_record SET canonical_record_jcs=zeroblob(?1) WHERE record_ref='observation-one-1'", [i64::try_from(MAX_OPERATION_PUBLICATION_BODY_BYTES + 1).unwrap()]).unwrap();
    assert!(matches!(
        read(
            &mut connection,
            None,
            2,
            MAX_OPERATION_PUBLICATION_BODY_BYTES
        ),
        Err(OperationPublicationReadError::Limit)
    ));
}

#[test]
fn continuation_keeps_the_original_prefix_and_does_not_revisit_prior_bodies() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let (_, append) = commit_pair(&mut connection);
    let first = read(&mut connection, None, 1, 4096).unwrap();
    let cursor = first.continuation.unwrap();
    let successor = record("observation-one-3", '5', OPERATION_OBSERVATION_V1_SCHEMA);
    let checkpoint =
        checkpoint_transition(10, "checkpoint-10-one", 'a', 11, "checkpoint-11-one", 'b');
    let next = OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        vec![OperationCustodyRecord::OperationObservation(
            successor.clone(),
        )],
        vec![OperationCustodyIndexMutation::LatestObservation {
            key: key('2'),
            expected: OperationCustodyIndexPredecessor::Exact(append.records()[0].record().clone()),
            successor: successor.coordinate().clone(),
        }],
        checkpoint.clone(),
        physical_accumulator(store(), &checkpoint),
    )
    .unwrap();
    SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .compare_and_commit(&next)
    .unwrap();
    // An earlier oversized body would fail if continuation read it again.
    connection.execute("UPDATE pale_deployment_control_record SET canonical_record_jcs=zeroblob(8192) WHERE record_ref='observation-one-1'", []).unwrap();
    let second = read(&mut connection, Some(&cursor), 8, 4096).unwrap();
    assert_eq!(second.through(), cursor.through());
    assert_eq!(second.sources.len(), 1);
    assert_eq!(
        second.sources[0].observation(),
        append.records()[0].record()
    );
    assert!(second.continuation.is_none());
    let fresh = read(&mut connection, None, 8, 16384).unwrap();
    assert_eq!(fresh.sources.len(), 3);
}

#[test]
fn continuation_rejects_another_incarnation_or_a_rewound_store_head() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    commit_pair(&mut connection);
    let cursor = read(&mut connection, None, 1, 4096)
        .unwrap()
        .continuation
        .unwrap();
    let wrong = OperationPublicationCursor::new(
        other_store(),
        cursor.through(),
        cursor.after_transaction(),
        cursor.after_reference().clone(),
    )
    .unwrap();
    assert!(
        matches!(read(&mut connection, Some(&wrong), 1, 4096), Err(OperationPublicationReadError::Storage(ref error)) if error.kind()==ControlStoreErrorKind::RecoveryRequired)
    );
    connection
        .execute(
            "UPDATE pale_control_store_header SET logical_transaction_sequence=28",
            [],
        )
        .unwrap();
    assert!(
        matches!(read(&mut connection, Some(&cursor), 1, 4096), Err(OperationPublicationReadError::Storage(ref error)) if error.kind()==ControlStoreErrorKind::RecoveryRequired)
    );
}

#[test]
fn a_missing_seek_index_fails_instead_of_falling_back_to_a_full_scan() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    commit_pair(&mut connection);
    connection
        .execute_batch("DROP INDEX pale_deployment_control_record_source_order")
        .unwrap();
    assert!(matches!(
        read(&mut connection, None, 1, 4096),
        Err(OperationPublicationReadError::Storage(_))
    ));
}
