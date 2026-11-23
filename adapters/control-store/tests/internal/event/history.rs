//! Historical physical reads do not reapply today's mutable index comparisons.

use super::*;
use pale_ports::{
    EventCustodyHistoryMember, EventCustodyHistoryObservation, EventCustodyHistoryRead,
};

fn request(transition: &EventCustodyTransition, maximum_bytes: usize) -> EventCustodyHistoryRead {
    EventCustodyHistoryRead::new(
        transition.store().clone(),
        transition.transaction().successor(),
        transition.journal_append().entry().clone(),
        transition.kind(),
        transition
            .members()
            .iter()
            .map(|member| {
                EventCustodyHistoryMember::new(member.kind(), member.record().coordinate().clone())
            })
            .collect(),
        maximum_bytes,
    )
    .unwrap()
}

fn body_bytes(transition: &EventCustodyTransition) -> usize {
    transition.journal_append().entry().bytes().as_slice().len()
        + transition
            .members()
            .iter()
            .map(|member| member.record().canonical_record_jcs().as_slice().len())
            .sum::<usize>()
}

fn observe(
    connection: &mut Connection,
    request: &EventCustodyHistoryRead,
) -> Result<EventCustodyHistoryObservation, pale_ports::ControlStoreError> {
    SqliteEventCustody {
        connection,
        coordinate: store(),
    }
    .read_committed_members(request)
}

#[test]
fn historical_members_survive_outbox_settlement_and_reopening() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("event-history.db");
    let mut connection = initialize_connection(Connection::open(&path).unwrap());
    let (retained, retain) =
        derived_publication(&mut connection, 1, "history-retained", '1', '2', None);
    let read = request(&retain, body_bytes(&retain));
    assert_eq!(
        observe(&mut connection, &read).unwrap(),
        EventCustodyHistoryObservation::Absent
    );
    commit(&mut connection, &retain);
    let (_, settle) = derived_publication(
        &mut connection,
        2,
        "history-settled",
        '3',
        '2',
        Some(&retained),
    );
    commit(&mut connection, &settle);
    drop(connection);
    let mut connection = Connection::open(&path).unwrap();
    let before = table_counts(&connection);
    let EventCustodyHistoryObservation::Exact { members } =
        observe(&mut connection, &read).unwrap()
    else {
        panic!("retained historical member read");
    };
    assert_eq!(members.as_ref(), retain.members());
    assert_eq!(table_counts(&connection), before);
    assert_ne!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store()
        }
        .observe_exact(&retain, 8)
        .unwrap(),
        EventCustodyReadBackOutcome::ExactSuccessor,
        "current-index replay and immutable history are different queries",
    );
}

#[test]
fn historical_reads_reject_partial_custody_and_split_insertion_stamps() {
    for fault in 0..6 {
        let mut connection = initialize_store();
        let (_, retain) = derived_publication(&mut connection, 1, "fault-retained", '1', '2', None);
        commit(&mut connection, &retain);
        let read = request(&retain, body_bytes(&retain));
        match fault {
            0 => {
                connection
                    .execute(
                        "DELETE FROM pale_deployment_authority_journal WHERE journal_sequence=1",
                        [],
                    )
                    .unwrap();
            }
            1 => {
                connection.execute("UPDATE pale_deployment_authority_journal SET physical_frame=x'01' WHERE journal_sequence=1", []).unwrap();
            }
            2 | 3 => {
                let member = &retain.members()[fault - 2];
                connection.execute("UPDATE pale_deployment_control_record SET inserted_transaction_sequence=1 WHERE record_ref=?1", [member.record().coordinate().record_ref().as_str()]).unwrap();
            }
            4 => {
                let member = &retain.members()[0];
                let reference = member.record().coordinate().record_ref().as_str();
                connection
                    .execute(
                        "DELETE FROM pale_deployment_event_index WHERE record_ref=?1",
                        [reference],
                    )
                    .unwrap();
                connection
                    .execute(
                        "DELETE FROM pale_deployment_control_record WHERE record_ref=?1",
                        [reference],
                    )
                    .unwrap();
            }
            5 => {
                connection
                    .execute("DELETE FROM pale_deployment_event_index", [])
                    .unwrap();
                connection
                    .execute("DELETE FROM pale_deployment_control_record", [])
                    .unwrap();
                connection
                    .execute("DELETE FROM pale_deployment_authority_journal", [])
                    .unwrap();
                // Committed frontiers still cover this missing history: it is not absence.
            }
            _ => unreachable!(),
        }
        let before = table_counts(&connection);
        assert_eq!(
            observe(&mut connection, &read).unwrap(),
            EventCustodyHistoryObservation::ConflictingOrPartial,
            "fault {fault}"
        );
        assert_eq!(table_counts(&connection), before);
        assert!(
            connection.is_autocommit(),
            "read transaction released for fault {fault}"
        );
    }
}

#[test]
fn historical_reads_enforce_aggregate_bytes_and_exact_requested_transaction() {
    let mut connection = initialize_store();
    let (_, retain) = derived_publication(&mut connection, 1, "bounded-retained", '1', '2', None);
    commit(&mut connection, &retain);
    let exact = request(&retain, body_bytes(&retain));
    assert!(matches!(
        observe(&mut connection, &exact).unwrap(),
        EventCustodyHistoryObservation::Exact { .. }
    ));
    for limit in [
        retain.journal_append().entry().bytes().as_slice().len(),
        body_bytes(&retain) - 1,
    ] {
        let short = request(&retain, limit);
        let before = table_counts(&connection);
        assert_eq!(
            observe(&mut connection, &short).unwrap(),
            EventCustodyHistoryObservation::BoundExceeded
        );
        assert_eq!(table_counts(&connection), before);
        assert!(connection.is_autocommit());
    }
    // The length check still validates the requested coordinate. A zero body
    // budget must not turn a known digest substitution into a size refusal.
    let mut substituted_members = exact.members().to_vec();
    substituted_members[0] = EventCustodyHistoryMember::new(
        substituted_members[0].kind(),
        coordinate(
            substituted_members[0].coordinate().record_ref().as_str(),
            'f',
        ),
    );
    let wrong_digest = EventCustodyHistoryRead::new(
        exact.store().clone(),
        exact.transaction(),
        exact.entry().clone(),
        exact.kind(),
        substituted_members,
        exact.entry().bytes().as_slice().len(),
    )
    .unwrap();
    let before = table_counts(&connection);
    assert_eq!(
        observe(&mut connection, &wrong_digest).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(table_counts(&connection), before);
    assert!(connection.is_autocommit());

    let wrong_transaction = EventCustodyHistoryRead::new(
        exact.store().clone(),
        pale_core::records::ControlTransactionSequenceV1::new(1).unwrap(),
        exact.entry().clone(),
        exact.kind(),
        exact.members().to_vec(),
        exact.maximum_bytes(),
    )
    .unwrap();
    assert_eq!(
        observe(&mut connection, &wrong_transaction).unwrap(),
        EventCustodyHistoryObservation::ConflictingOrPartial
    );
    let wrong_entry = EventCustodyHistoryRead::new(
        exact.store().clone(),
        exact.transaction(),
        append(2).entry().clone(),
        exact.kind(),
        exact.members().to_vec(),
        exact.maximum_bytes(),
    )
    .unwrap();
    assert_eq!(
        observe(&mut connection, &wrong_entry).unwrap(),
        EventCustodyHistoryObservation::ConflictingOrPartial
    );
}

#[test]
fn transaction_preparation_cannot_be_replaced_after_commit_or_raced_before_commit() {
    let mut connection = initialize_store();
    let (_, original) = owner_event(&mut connection, 1, "transaction-event", '1', '1');
    commit(&mut connection, &original);
    let substituted = EventCustodyTransition::new(
        store(),
        prepare_transaction(&mut connection),
        original.kind(),
        original.journal_append().clone(),
        original.members().to_vec(),
        original.index_mutations().to_vec(),
        original.index_assertions().to_vec(),
    )
    .unwrap();
    let mut custody = SqliteEventCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody.compare_and_commit(&original, 8).unwrap(),
        EventCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(
        custody
            .compare_and_commit(&substituted, 8)
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    let mut connection = initialize_store();
    let (_, raced) = owner_event(&mut connection, 1, "raced-event", '1', '1');
    let transaction = connection.transaction().unwrap();
    crate::store::transaction::advance(&transaction, raced.transaction()).unwrap();
    transaction.commit().unwrap();
    let before = table_counts(&connection);
    assert_eq!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store()
        }
        .compare_and_commit(&raced, 8)
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    assert_eq!(table_counts(&connection), before);
}
