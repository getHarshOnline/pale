//! Physical handle-to-request custody. Record admission remains with application owners.

use super::*;

#[test]
fn unlike_request_with_the_right_schema_is_not_exact_start_replay() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let candidate = start_candidate("one", ['1', '2', '3', '9']);
    SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .compare_and_commit(&candidate)
    .unwrap();
    let foreign = record("foreign-request", 'f', COMMON_REQUEST_ENVELOPE_V1_SCHEMA);
    insert_record(&connection, &foreign);
    connection
        .execute(
            "UPDATE pale_deployment_operation_index SET record_ref=?1, record_sha256=?2 \
         WHERE index_role='request_for_handle' AND index_key=?3",
            params![
                foreign.coordinate().record_ref().as_str(),
                foreign.coordinate().record_sha256().as_str(),
                key('2').as_str()
            ],
        )
        .unwrap();
    let before = connection.total_changes();
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    // A physical lookup returns retained bytes; it cannot admit their association.
    let observed = custody
        .read_index(
            &store(),
            &OperationCustodyIndexQuery::RequestForHandle(key('2')),
            16 * 1024,
        )
        .unwrap();
    assert_eq!(observed.record(), Some(&foreign));
    assert_ne!(
        observed.record(),
        candidate.record(pale_ports::OperationCustodyRecordKind::CommonRequest)
    );
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

#[test]
fn failed_handle_request_insert_rolls_back_the_complete_start() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let candidate = start_candidate("one", ['1', '2', '3', '9']);
    let query = OperationCustodyIndexQuery::RequestForHandle(key('2'));
    connection.execute_batch(
        "CREATE TEMP TRIGGER reject_handle_request BEFORE INSERT ON pale_deployment_operation_index \
         WHEN NEW.index_role='request_for_handle' \
         BEGIN SELECT RAISE(ABORT, 'injected handle request failure'); END;",
    ).unwrap();
    {
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        assert!(custody
            .read_index(&store(), &query, 16 * 1024)
            .unwrap()
            .record()
            .is_none());
        assert_eq!(
            custody.compare_and_commit(&candidate).unwrap_err().kind(),
            ControlStoreErrorKind::Storage
        );
        assert_eq!(
            custody.observe_exact(&candidate).unwrap(),
            OperationCustodyReadBackOutcome::Predecessor
        );
        assert!(custody
            .read_index(&store(), &query, 16 * 1024)
            .unwrap()
            .record()
            .is_none());
    }
    let counts: (i64, i64, i64) = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM pale_deployment_control_record), \
         (SELECT COUNT(*) FROM pale_deployment_operation_index), \
         (SELECT domain_sequence FROM pale_deployment_operation_checkpoint_head \
          WHERE checkpoint_domain_id='operation:principal-1')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(counts, (1, 0, 8));
    connection
        .execute_batch("DROP TRIGGER reject_handle_request")
        .unwrap();
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
    let observed = custody.read_index(&store(), &query, 16 * 1024).unwrap();
    assert_eq!(
        observed.record(),
        candidate.record(pale_ports::OperationCustodyRecordKind::CommonRequest)
    );
}
