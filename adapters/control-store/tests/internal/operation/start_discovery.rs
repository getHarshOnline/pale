//! Physical enumeration of committed operation starts.

use super::*;
use pale_ports::{
    OperationStartCursor, OperationStartReadError, OperationStartSources,
    MAX_OPERATION_START_SOURCES, MAX_OPERATION_START_SOURCE_BYTES,
};

fn read(
    connection: &mut Connection,
    cursor: Option<&OperationStartCursor>,
    maximum_sources: usize,
    maximum_bytes: usize,
) -> Result<OperationStartSources, OperationStartReadError> {
    SqliteOperationCustody {
        connection,
        coordinate: store(),
    }
    .read_operation_starts(&store(), cursor, maximum_sources, maximum_bytes)
}

fn commit_first(connection: &mut Connection) -> OperationCustodyCandidate {
    let first = start_candidate("one", ['1', '2', '3', '9']);
    SqliteOperationCustody {
        connection,
        coordinate: store(),
    }
    .compare_and_commit(&first)
    .unwrap();
    first
}

fn commit_second(connection: &mut Connection) -> OperationCustodyCandidate {
    let checkpoint =
        checkpoint_transition(9, "checkpoint-9-one", '9', 10, "checkpoint-10-two", 'a');
    let second = start_candidate_at_checkpoint("two", ['4', '5', '6', 'a'], key('5'), checkpoint);
    SqliteOperationCustody {
        connection,
        coordinate: store(),
    }
    .compare_and_commit(&second)
    .unwrap();
    second
}

fn commit_third(connection: &mut Connection) -> OperationCustodyCandidate {
    let checkpoint =
        checkpoint_transition(10, "checkpoint-10-two", 'a', 11, "checkpoint-11-three", 'b');
    let third = start_candidate_at_checkpoint("three", ['7', '8', '9', 'b'], key('8'), checkpoint);
    SqliteOperationCustody {
        connection,
        coordinate: store(),
    }
    .compare_and_commit(&third)
    .unwrap();
    third
}

fn expect_recovery(result: Result<OperationStartSources, OperationStartReadError>, context: &str) {
    assert!(
        matches!(
            result,
            Err(OperationStartReadError::Storage(ref error))
                if error.kind() == ControlStoreErrorKind::RecoveryRequired
        ),
        "{context}: {result:?}"
    );
}

#[test]
fn pages_are_stable_ordered_and_emit_continuation_only_for_lookahead() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let first = commit_first(&mut connection);
    let second = commit_second(&mut connection);

    let first_page = read(&mut connection, None, 1, 4096).unwrap();
    assert_eq!(first_page.sources.len(), 1);
    assert_eq!(
        first_page.sources[0].session_basis(),
        first.session_basis().unwrap()
    );
    let frozen = first_page.through();
    let cursor = first_page.continuation.unwrap();

    let third = commit_third(&mut connection);
    let second_page = read(&mut connection, Some(&cursor), 1, 4096).unwrap();
    assert_eq!(second_page.through(), frozen);
    assert_eq!(second_page.sources.len(), 1);
    assert_eq!(
        second_page.sources[0].session_basis(),
        second.session_basis().unwrap()
    );
    assert!(second_page.continuation.is_none());

    let fresh = read(&mut connection, None, 8, 4096).unwrap();
    assert_eq!(fresh.sources.len(), 3);
    assert_eq!(
        fresh
            .sources
            .iter()
            .map(|source| source.session_basis().handle().record_ref().as_str())
            .collect::<Vec<_>>(),
        ["handle-one", "handle-two", "handle-three"]
    );
    assert_eq!(
        fresh.sources[2].session_basis(),
        third.session_basis().unwrap()
    );
    assert!(fresh.continuation.is_none());

    let final_page = read(&mut connection, None, 3, 4096).unwrap();
    assert_eq!(final_page.sources.len(), 3);
    assert!(final_page.continuation.is_none());

    let mut empty = Connection::open_in_memory().unwrap();
    initialize_store(&empty);
    let empty_page = read(&mut empty, None, 3, 4096).unwrap();
    assert!(empty_page.sources.is_empty());
    assert!(empty_page.continuation.is_none());
}

#[test]
fn limits_fail_without_returning_a_partial_page() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let first = commit_first(&mut connection);
    let second = commit_second(&mut connection);
    let exact = first.session_basis().unwrap().canonical_basis_jcs().len()
        + second.session_basis().unwrap().canonical_basis_jcs().len();
    assert_eq!(
        read(&mut connection, None, 2, exact).unwrap().sources.len(),
        2
    );
    for (count, bytes) in [
        (0, exact),
        (MAX_OPERATION_START_SOURCES + 1, exact),
        (2, 0),
        (2, MAX_OPERATION_START_SOURCE_BYTES + 1),
        (2, exact - 1),
    ] {
        assert!(matches!(
            read(&mut connection, None, count, bytes),
            Err(OperationStartReadError::Limit)
        ));
    }
}

#[test]
fn continuation_binds_store_prefix_and_every_prior_row_coordinate() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    commit_first(&mut connection);
    commit_second(&mut connection);
    let cursor = read(&mut connection, None, 1, 4096)
        .unwrap()
        .continuation
        .unwrap();

    let foreign = OperationStartCursor::new(
        other_store(),
        cursor.through(),
        cursor.after_transaction(),
        cursor.handle().clone(),
        cursor.request().clone(),
        cursor.session_lease_digest().clone(),
        cursor.session_basis_digest().clone(),
    )
    .unwrap();
    expect_recovery(read(&mut connection, Some(&foreign), 1, 4096), "foreign");

    let future = OperationStartCursor::new(
        store(),
        pale_core::records::ControlTransactionSequenceV1::new(cursor.through().get() + 1).unwrap(),
        cursor.after_transaction(),
        cursor.handle().clone(),
        cursor.request().clone(),
        cursor.session_lease_digest().clone(),
        cursor.session_basis_digest().clone(),
    )
    .unwrap();
    expect_recovery(read(&mut connection, Some(&future), 1, 4096), "future");
    assert!(
        OperationStartCursor::new(
            store(),
            cursor.after_transaction(),
            pale_core::records::ControlTransactionSequenceV1::new(
                cursor.after_transaction().get() + 1,
            )
            .unwrap(),
            cursor.handle().clone(),
            cursor.request().clone(),
            cursor.session_lease_digest().clone(),
            cursor.session_basis_digest().clone(),
        )
        .is_err()
    );

    let unlike_handle = record("handle-unlike", 'c', OPERATION_HANDLE_V1_SCHEMA);
    let substituted = OperationStartCursor::new(
        store(),
        cursor.through(),
        cursor.after_transaction(),
        unlike_handle.coordinate().clone(),
        cursor.request().clone(),
        cursor.session_lease_digest().clone(),
        cursor.session_basis_digest().clone(),
    )
    .unwrap();
    expect_recovery(
        read(&mut connection, Some(&substituted), 1, 4096),
        "substituted",
    );

    let unlike_request = record("request-unlike", 'd', COMMON_REQUEST_ENVELOPE_V1_SCHEMA);
    for substituted in [
        OperationStartCursor::new(
            store(),
            cursor.through(),
            cursor.after_transaction(),
            cursor.handle().clone(),
            unlike_request.coordinate().clone(),
            cursor.session_lease_digest().clone(),
            cursor.session_basis_digest().clone(),
        )
        .unwrap(),
        OperationStartCursor::new(
            store(),
            cursor.through(),
            cursor.after_transaction(),
            cursor.handle().clone(),
            cursor.request().clone(),
            pale_core::records::SessionAdmissionLeaseDigest::parse(digest('e')).unwrap(),
            cursor.session_basis_digest().clone(),
        )
        .unwrap(),
        OperationStartCursor::new(
            store(),
            cursor.through(),
            cursor.after_transaction(),
            cursor.handle().clone(),
            cursor.request().clone(),
            cursor.session_lease_digest().clone(),
            ArtifactDigest::parse(digest('f')).unwrap(),
        )
        .unwrap(),
    ] {
        expect_recovery(
            read(&mut connection, Some(&substituted), 1, 4096),
            "substituted continuation coordinate",
        );
    }

    connection
        .execute(
            "UPDATE pale_deployment_operation_session_basis SET session_lease_sha256=?1 \
             WHERE handle_ref=?2",
            params![digest('f'), cursor.handle().record_ref().as_str()],
        )
        .unwrap();
    expect_recovery(read(&mut connection, Some(&cursor), 1, 4096), "changed");
}

#[test]
fn missing_or_inconsistent_start_material_fails_closed() {
    for corruption in [
        "UPDATE pale_deployment_operation_session_basis SET canonical_basis_jcs=x'7b7d' WHERE handle_ref='handle-one'",
        "UPDATE pale_deployment_operation_session_basis SET basis_sha256=printf('%064d',0) WHERE handle_ref='handle-one'",
        "UPDATE pale_deployment_operation_session_basis SET inserted_transaction_sequence=10 WHERE handle_ref='handle-one'",
        "UPDATE pale_deployment_control_record SET inserted_transaction_sequence=10 WHERE record_ref='request-one'",
        "UPDATE pale_deployment_control_record SET schema_id='unsupported' WHERE record_ref='handle-one'",
        "DELETE FROM pale_deployment_control_record WHERE record_ref='handle-one'",
    ] {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_store(&connection);
        commit_first(&mut connection);
        if corruption.starts_with("DELETE FROM pale_deployment_control_record") {
            connection.pragma_update(None, "foreign_keys", false).unwrap();
        }
        connection.execute_batch(corruption).unwrap();
        expect_recovery(read(&mut connection, None, 8, 4096), corruption);
    }
}

#[test]
fn a_missing_seek_index_does_not_fall_back_to_a_table_scan() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    commit_first(&mut connection);
    connection
        .execute_batch("DROP INDEX pale_deployment_operation_session_basis_transaction")
        .unwrap();
    assert!(matches!(
        read(&mut connection, None, 1, 4096),
        Err(OperationStartReadError::Storage(_))
    ));
}
