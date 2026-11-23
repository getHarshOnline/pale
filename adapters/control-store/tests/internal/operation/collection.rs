//! Real database atomicity for a physical collection frontier and journal pair.
//!
//! The inherited opaque observation/checkpoint and accumulator fixtures do not
//! admit a Consult roster, signed evidence, stopping policy, or production profile.

use super::*;
use pale_core::records::JournalSequenceV1;
use pale_ports::{
    BoundedAuthorityEntryBytes, OperationCollectionJournal, OperationCustodyRecordKind,
    PreparedAuthorityAppend, SequencedAuthorityEntry,
};

fn collection_handle() -> ControlRecordCoordinate {
    candidates()
        .0
        .record(OperationCustodyRecordKind::OperationHandle)
        .unwrap()
        .coordinate()
        .clone()
}

fn journal(payload: &[u8]) -> PreparedAuthorityAppend {
    PreparedAuthorityAppend::new(
        store(),
        None,
        SequencedAuthorityEntry::new(
            JournalSequenceV1::new(1).unwrap(),
            BoundedAuthorityEntryBytes::new(payload.to_vec(), 64).unwrap(),
        ),
    )
    .unwrap()
}

fn successor_journal(payload: &[u8]) -> PreparedAuthorityAppend {
    PreparedAuthorityAppend::new(
        store(),
        Some(JournalSequenceV1::new(1).unwrap()),
        SequencedAuthorityEntry::new(
            JournalSequenceV1::new(2).unwrap(),
            BoundedAuthorityEntryBytes::new(payload.to_vec(), 64).unwrap(),
        ),
    )
    .unwrap()
}

fn later_observation(predecessor: ControlRecord) -> OperationCustodyCandidate {
    let successor = record("observation-one-3", '5', OPERATION_OBSERVATION_V1_SCHEMA);
    let checkpoint =
        checkpoint_transition(10, "checkpoint-10-one", 'a', 11, "checkpoint-11-one", 'b');
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
    .unwrap()
}

fn candidates() -> (
    OperationCustodyCandidate,
    OperationCustodyCandidate,
    OperationCustodyCandidate,
) {
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let observation = observation_candidate(
        start
            .record(OperationCustodyRecordKind::OperationObservation)
            .unwrap()
            .clone(),
        "one",
        '4',
    );
    let pair = OperationCollectionJournal::prepare(
        start
            .record(OperationCustodyRecordKind::OperationHandle)
            .unwrap()
            .coordinate(),
        observation.accumulator().binding().transaction().clone(),
        journal(b"physical journal payload"),
        4096,
        64,
    )
    .unwrap();
    let joined = observation.clone().with_collection_journal(pair).unwrap();
    (start, observation, joined)
}

fn cancellation_candidate(
    start: &OperationCustodyCandidate,
) -> (OperationCustodyCandidate, OperationCustodyCandidate) {
    let ordinary = observation_candidate(
        start
            .record(OperationCustodyRecordKind::OperationObservation)
            .unwrap()
            .clone(),
        "one",
        '4',
    );
    let request = record(
        "cancel-consultation",
        '5',
        COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
    );
    let cancellation = OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        vec![
            OperationCustodyRecord::OperationObservation(
                ordinary
                    .record(OperationCustodyRecordKind::OperationObservation)
                    .unwrap()
                    .clone(),
            ),
            OperationCustodyRecord::CommonRequest(request),
        ],
        ordinary
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
        ordinary.checkpoint().clone(),
        ordinary.accumulator().clone(),
    )
    .unwrap();
    let pair = OperationCollectionJournal::prepare(
        start
            .record(OperationCustodyRecordKind::OperationHandle)
            .unwrap()
            .coordinate(),
        cancellation.accumulator().binding().transaction().clone(),
        journal(b"cancelled consultation roster"),
        4096,
        64,
    )
    .unwrap();
    let joined = cancellation.clone().with_collection_journal(pair).unwrap();
    (cancellation, joined)
}

fn commit(
    connection: &mut Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<OperationCustodyCommitOutcome, pale_ports::ControlStoreError> {
    SqliteOperationCustody {
        connection,
        coordinate: store(),
    }
    .compare_and_commit(candidate)
}

fn observed(
    connection: &mut Connection,
    candidate: &OperationCustodyCandidate,
) -> OperationCustodyReadBackOutcome {
    SqliteOperationCustody {
        connection,
        coordinate: store(),
    }
    .observe_exact(candidate)
    .unwrap()
}

#[test]
fn joined_frontier_and_journal_reopen_exactly_without_subset_replay() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("collection.sqlite");
    let (start, omitted, joined) = candidates();
    {
        let mut connection = Connection::open(&path).unwrap();
        initialize_store(&connection);
        assert_eq!(
            commit(&mut connection, &start).unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
        assert_eq!(
            observed(&mut connection, &joined),
            OperationCustodyReadBackOutcome::Predecessor
        );
        assert_eq!(
            commit(&mut connection, &joined).unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
    }
    let mut connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    let changes = connection.total_changes();
    assert_eq!(
        observed(&mut connection, &joined),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    let handle = start
        .record(OperationCustodyRecordKind::OperationHandle)
        .unwrap()
        .coordinate()
        .clone();
    let snapshot = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .read_collection_snapshot(&store(), &handle, 4096, 64)
    .unwrap()
    .expect("handle-scoped collection head");
    assert_eq!(
        snapshot.frontier(),
        joined
            .record(OperationCustodyRecordKind::CollectionFrontier)
            .unwrap()
    );
    assert_eq!(
        snapshot.head().query(),
        &pale_ports::OperationCustodyIndexQuery::CollectionHead(
            pale_ports::OperationCustodyIndexKey::for_collection_head(&handle)
        )
    );
    assert_eq!(
        snapshot.head().current_frontier(),
        snapshot.frontier().coordinate()
    );
    assert_eq!(
        snapshot.journal_payload().as_slice(),
        b"physical journal payload"
    );
    assert_eq!(
        commit(&mut connection, &joined).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(
        observed(&mut connection, &omitted),
        OperationCustodyReadBackOutcome::ConflictingOrPartial
    );
    assert_eq!(
        commit(&mut connection, &omitted).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    let pair = joined.collection_journal().unwrap();
    assert!(crate::operation::collection_journal::journal_entry_matches(
        &connection,
        &collection_handle(),
        pair.append(),
        64
    )
    .unwrap());
    assert_eq!(
        crate::operation::collection_journal::read_journal_head(&connection, &collection_handle())
            .unwrap(),
        Some(pair.append().entry().sequence())
    );
    for member in joined
        .records()
        .iter()
        .map(OperationCustodyRecord::record)
        .chain(std::iter::once(joined.checkpoint().successor_record()))
    {
        assert_eq!(
            SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store()
            }
            .read_operation_record(&store(), member.coordinate(), 4096)
            .unwrap(),
            Some(member.clone())
        );
        let inserted:i64=connection.query_row("SELECT inserted_transaction_sequence FROM pale_deployment_control_record WHERE record_ref=?1",[member.coordinate().record_ref().as_str()],|row|row.get(0)).unwrap();
        assert_eq!(
            u64::try_from(inserted).unwrap(),
            pair.transaction().successor().get()
        );
    }
    assert_eq!(connection.total_changes(), changes);
}

#[test]
fn cancellation_request_and_collection_pair_reopen_as_one_exact_transaction() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("consultation-cancellation.sqlite");
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let (without_collection, cancellation) = cancellation_candidate(&start);
    {
        let mut connection = Connection::open(&path).unwrap();
        initialize_store(&connection);
        assert_eq!(
            commit(&mut connection, &start).unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
        assert_eq!(
            commit(&mut connection, &cancellation).unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
    }

    let mut connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    assert_eq!(
        observed(&mut connection, &cancellation),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    assert_eq!(
        observed(&mut connection, &without_collection),
        OperationCustodyReadBackOutcome::ConflictingOrPartial,
        "the signed cancellation request cannot replay without its roster frontier"
    );
    let handle = start
        .record(OperationCustodyRecordKind::OperationHandle)
        .unwrap()
        .coordinate()
        .clone();
    let snapshot = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .read_collection_snapshot(&store(), &handle, 4096, 64)
    .unwrap()
    .expect("cancelled roster remains point-readable");
    assert_eq!(
        snapshot.frontier(),
        cancellation
            .record(OperationCustodyRecordKind::CollectionFrontier)
            .unwrap()
    );
    assert_eq!(
        snapshot.journal_payload().as_slice(),
        b"cancelled consultation roster"
    );
    for kind in [
        OperationCustodyRecordKind::OperationObservation,
        OperationCustodyRecordKind::CommonRequest,
        OperationCustodyRecordKind::CollectionFrontier,
    ] {
        let record = cancellation.record(kind).unwrap();
        assert_eq!(
            SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store(),
            }
            .read_operation_record(&store(), record.coordinate(), 4096)
            .unwrap(),
            Some(record.clone())
        );
    }
    assert_eq!(
        commit(&mut connection, &cancellation).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
}

#[test]
fn successor_collection_advances_one_handle_scoped_head_and_recovers_without_a_scan() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let (start, _, joined) = candidates();
    commit(&mut connection, &start).unwrap();
    commit(&mut connection, &joined).unwrap();

    let later = later_observation(
        joined
            .record(OperationCustodyRecordKind::OperationObservation)
            .unwrap()
            .clone(),
    );
    let previous_frontier = joined
        .record(OperationCustodyRecordKind::CollectionFrontier)
        .unwrap()
        .clone();
    let handle = start
        .record(OperationCustodyRecordKind::OperationHandle)
        .unwrap()
        .coordinate()
        .clone();
    let pair = OperationCollectionJournal::prepare(
        &handle,
        later.accumulator().binding().transaction().clone(),
        successor_journal(b"second collection payload"),
        4096,
        64,
    )
    .unwrap();
    let successor = later
        .with_collection_journal_after(pair, Some(previous_frontier.clone()))
        .unwrap();
    assert_eq!(
        commit(&mut connection, &successor).unwrap(),
        OperationCustodyCommitOutcome::Committed
    );

    let snapshot = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .read_collection_snapshot(&store(), &handle, 4096, 64)
    .unwrap()
    .expect("successor collection head");
    assert_eq!(
        snapshot.frontier(),
        successor
            .record(OperationCustodyRecordKind::CollectionFrontier)
            .unwrap()
    );
    assert_eq!(
        snapshot.head().query(),
        &pale_ports::OperationCustodyIndexQuery::CollectionHead(
            pale_ports::OperationCustodyIndexKey::for_collection_head(&handle)
        )
    );
    assert_eq!(
        snapshot.head().current_frontier(),
        snapshot.frontier().coordinate()
    );
    assert_eq!(
        snapshot.journal_payload().as_slice(),
        b"second collection payload"
    );
    assert_eq!(
        SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .read_operation_record(&store(), previous_frontier.coordinate(), 4096)
        .unwrap(),
        Some(previous_frontier)
    );
    assert_eq!(
        commit(&mut connection, &joined).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
}

#[test]
fn collection_point_recovery_rejects_bounds_and_changed_journal_bytes() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let (start, _, joined) = candidates();
    commit(&mut connection, &start).unwrap();
    commit(&mut connection, &joined).unwrap();
    let handle = start
        .record(OperationCustodyRecordKind::OperationHandle)
        .unwrap()
        .coordinate()
        .clone();

    let missing = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .read_collection_snapshot(&store(), &coordinate("unrelated-handle", 'f'), 4096, 64)
    .unwrap();
    assert!(
        missing.is_none(),
        "point lookup must not scan other collections"
    );
    for (frontier_limit, payload_limit) in [(0, 64), (4096, 0), (4096, 1)] {
        let error = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .read_collection_snapshot(&store(), &handle, frontier_limit, payload_limit)
        .unwrap_err();
        assert_eq!(error.kind(), ControlStoreErrorKind::BoundExceeded);
    }
    let error = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .read_collection_snapshot(&store(), &handle, 1, 64)
    .unwrap_err();
    assert_eq!(error.kind(), ControlStoreErrorKind::RecoveryRequired);

    let changed = crate::frame::encode(
        b"changed collection payload",
        pale_ports::AuthorityByteClass::JournalEntry,
    )
    .unwrap();
    connection
        .execute(
            "UPDATE pale_deployment_collection_journal SET physical_frame=?1 WHERE journal_sequence=1",
            [changed],
        )
        .unwrap();
    let error = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .read_collection_snapshot(&store(), &handle, 4096, 64)
    .unwrap_err();
    assert_eq!(error.kind(), ControlStoreErrorKind::RecoveryRequired);
}

#[test]
fn every_joined_write_failure_restores_both_predecessors() {
    let failures = [
        "BEFORE INSERT ON pale_deployment_control_record WHEN NEW.schema_id='pale/private-operation-collection-frontier.v1'",
        "BEFORE INSERT ON pale_deployment_collection_journal",
        "BEFORE UPDATE ON pale_deployment_collection_head",
        "BEFORE INSERT ON pale_deployment_operation_accumulator_position",
        "BEFORE UPDATE ON pale_deployment_operation_index",
        "BEFORE UPDATE ON pale_deployment_operation_checkpoint_head",
    ];
    for fault in failures {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_store(&connection);
        let (start, _, joined) = candidates();
        commit(&mut connection, &start).unwrap();
        connection.execute_batch(&format!("CREATE TRIGGER abort_join {fault} BEGIN SELECT RAISE(ABORT,'injected joined write'); END;")).unwrap();
        assert!(commit(&mut connection, &joined).is_err(), "{fault}");
        assert_eq!(
            observed(&mut connection, &joined),
            OperationCustodyReadBackOutcome::Predecessor,
            "{fault}"
        );
        assert_eq!(
            observed(&mut connection, &start),
            OperationCustodyReadBackOutcome::ExactSuccessor,
            "{fault}"
        );
        assert_eq!(
            crate::operation::collection_journal::read_journal_head(
                &connection,
                &collection_handle()
            )
            .unwrap(),
            None
        );
        assert!(!crate::operation::collection_journal::journal_entry_exists(
            &connection,
            &collection_handle(),
            JournalSequenceV1::new(1).unwrap()
        )
        .unwrap());
        let transaction: i64 = connection
            .query_row(
                "SELECT logical_transaction_sequence FROM pale_control_store_header ",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            u64::try_from(transaction).unwrap(),
            joined
                .accumulator()
                .binding()
                .transaction()
                .expected()
                .get()
        );
        connection.execute_batch("DROP TRIGGER abort_join").unwrap();
        assert_eq!(
            commit(&mut connection, &joined).unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
    }
}

#[test]
fn changed_pair_or_retained_material_never_becomes_exact_replay() {
    for fault in 0..6 {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_store(&connection);
        let (start, observation, joined) = candidates();
        commit(&mut connection, &start).unwrap();
        commit(&mut connection, &joined).unwrap();
        let mut selected = joined.clone();
        let frontier_ref = joined
            .record(OperationCustodyRecordKind::CollectionFrontier)
            .unwrap()
            .coordinate()
            .record_ref()
            .as_str()
            .to_owned();
        match fault {
            0 => {
                let changed = OperationCollectionJournal::prepare(
                    start
                        .record(OperationCustodyRecordKind::OperationHandle)
                        .unwrap()
                        .coordinate(),
                    observation.accumulator().binding().transaction().clone(),
                    journal(b"other physical payload"),
                    4096,
                    64,
                )
                .unwrap();
                selected = observation.with_collection_journal(changed).unwrap();
            }
            1 => {
                connection
                    .execute(
                        "DELETE FROM pale_deployment_collection_journal WHERE journal_sequence=1",
                        [],
                    )
                    .unwrap();
            }
            2 => {
                let frame = crate::frame::encode(
                    b"other physical payload",
                    pale_ports::AuthorityByteClass::JournalEntry,
                )
                .unwrap();
                connection.execute("UPDATE pale_deployment_collection_journal SET physical_frame=?1 WHERE journal_sequence=1",[frame]).unwrap();
            }
            3 => {
                connection.execute("UPDATE pale_deployment_control_record SET canonical_record_jcs=x'7b7d' WHERE record_ref=?1",[frontier_ref.as_str()]).unwrap();
            }
            4 => {
                connection.execute("UPDATE pale_deployment_control_record SET schema_sha256=?1 WHERE record_ref=?2",rusqlite::params![digest('e'), frontier_ref]).unwrap();
            }
            5 => {
                connection.execute("UPDATE pale_deployment_control_record SET inserted_transaction_sequence=29 WHERE record_ref=?1",[frontier_ref.as_str()]).unwrap();
            }
            _ => unreachable!(),
        }
        let changes = connection.total_changes();
        assert_eq!(
            observed(&mut connection, &selected),
            OperationCustodyReadBackOutcome::ConflictingOrPartial,
            "fault {fault}"
        );
        assert_eq!(
            commit(&mut connection, &selected).unwrap_err().kind(),
            ControlStoreErrorKind::RecoveryRequired,
            "fault {fault}"
        );
        assert_eq!(connection.total_changes(), changes);
    }
}

#[test]
fn an_unrelated_complete_journal_winner_stales_the_pair_without_operation_writes() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let (start, _, joined) = candidates();
    commit(&mut connection, &start).unwrap();
    // This competing physical journal writer is a model, not an admitted roster.
    let transaction = connection.transaction().unwrap();
    crate::operation::collection_journal::append_collection_entry(
        &transaction,
        &collection_handle(),
        &journal(b"unrelated journal winner"),
    )
    .unwrap();
    transaction.commit().unwrap();
    let changes = connection.total_changes();
    assert_eq!(
        observed(&mut connection, &joined),
        OperationCustodyReadBackOutcome::ConflictingOrPartial
    );
    assert_eq!(
        commit(&mut connection, &joined).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    assert_eq!(
        observed(&mut connection, &start),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    assert_eq!(connection.total_changes(), changes);
}

#[test]
fn later_journal_progress_preserves_bytes_but_cannot_be_replayed_backwards() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let (start, _, joined) = candidates();
    commit(&mut connection, &start).unwrap();
    commit(&mut connection, &joined).unwrap();
    let later = PreparedAuthorityAppend::new(
        store(),
        Some(JournalSequenceV1::new(1).unwrap()),
        SequencedAuthorityEntry::new(
            JournalSequenceV1::new(2).unwrap(),
            BoundedAuthorityEntryBytes::new(b"later journal entry".to_vec(), 64).unwrap(),
        ),
    )
    .unwrap();
    let transaction = connection.transaction().unwrap();
    crate::operation::collection_journal::append_collection_entry(
        &transaction,
        &collection_handle(),
        &later,
    )
    .unwrap();
    transaction.commit().unwrap();
    let changes = connection.total_changes();
    assert_eq!(
        observed(&mut connection, &joined),
        OperationCustodyReadBackOutcome::ConflictingOrPartial
    );
    assert_eq!(
        commit(&mut connection, &joined).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    assert!(crate::operation::collection_journal::journal_entry_matches(
        &connection,
        &collection_handle(),
        joined.collection_journal().unwrap().append(),
        64
    )
    .unwrap());
    assert_eq!(
        crate::operation::collection_journal::read_journal_head(&connection, &collection_handle())
            .unwrap(),
        Some(later.entry().sequence())
    );
    assert_eq!(connection.total_changes(), changes);
}

#[test]
fn retained_frame_bound_and_extra_intact_role_are_not_exact_custody() {
    for oversized in [false, true] {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_store(&connection);
        let (start, _, joined) = candidates();
        commit(&mut connection, &start).unwrap();
        commit(&mut connection, &joined).unwrap();
        if oversized {
            let frame =
                crate::frame::encode(&[b'x'; 65], pale_ports::AuthorityByteClass::JournalEntry)
                    .unwrap();
            connection.execute("UPDATE pale_deployment_collection_journal SET physical_frame=?1 WHERE journal_sequence=1",[frame]).unwrap();
        } else {
            let extra = OperationCollectionJournal::prepare(
                start
                    .record(OperationCustodyRecordKind::OperationHandle)
                    .unwrap()
                    .coordinate(),
                joined.accumulator().binding().transaction().clone(),
                journal(b"other physical payload"),
                4096,
                64,
            )
            .unwrap();
            insert_record(&connection, extra.frontier());
        }
        let changes = connection.total_changes();
        let outcome = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .observe_exact(&joined);
        let kind = if oversized {
            assert_eq!(
                outcome.unwrap_err().kind(),
                ControlStoreErrorKind::BoundExceeded
            );
            ControlStoreErrorKind::BoundExceeded
        } else {
            assert_eq!(
                outcome.unwrap(),
                OperationCustodyReadBackOutcome::ConflictingOrPartial
            );
            ControlStoreErrorKind::RecoveryRequired
        };
        assert_eq!(commit(&mut connection, &joined).unwrap_err().kind(), kind);
        assert_eq!(connection.total_changes(), changes);
    }
}

#[test]
fn historical_collection_requires_its_exact_journal_payload_after_operation_progress() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let (start, _, joined) = candidates();
    commit(&mut connection, &start).unwrap();
    commit(&mut connection, &joined).unwrap();
    let successor = record("observation-one-3", '5', OPERATION_OBSERVATION_V1_SCHEMA);
    let checkpoint =
        checkpoint_transition(10, "checkpoint-10-one", 'a', 11, "checkpoint-11-one", 'b');
    let later = OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        vec![OperationCustodyRecord::OperationObservation(
            successor.clone(),
        )],
        vec![OperationCustodyIndexMutation::LatestObservation {
            key: key('2'),
            expected: OperationCustodyIndexPredecessor::Exact(
                joined
                    .record(OperationCustodyRecordKind::OperationObservation)
                    .unwrap()
                    .clone(),
            ),
            successor: successor.coordinate().clone(),
        }],
        checkpoint.clone(),
        physical_accumulator(store(), &checkpoint),
    )
    .unwrap();
    commit(&mut connection, &later).unwrap();
    assert_eq!(
        commit(&mut connection, &joined).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );

    let substituted = crate::frame::encode(
        b"substituted physical payload",
        pale_ports::AuthorityByteClass::JournalEntry,
    )
    .unwrap();
    connection.execute(
        "UPDATE pale_deployment_collection_journal SET physical_frame=?1 WHERE journal_sequence=1",
        [substituted],
    ).unwrap();
    let changes = connection.total_changes();
    assert_eq!(
        observed(&mut connection, &joined),
        OperationCustodyReadBackOutcome::ConflictingOrPartial
    );
    assert_eq!(
        commit(&mut connection, &joined).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        observed(&mut connection, &later),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    assert_eq!(connection.total_changes(), changes);
}

#[test]
fn journal_only_collection_material_requires_recovery_after_an_unrelated_operation_wins() {
    for exact_payload in [false, true] {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_store(&connection);
        let (start, _, joined) = candidates();
        commit(&mut connection, &start).unwrap();
        // Model a partial atom containing only its journal entry. The different
        // payload control is an unrelated journal winner, not candidate material.
        let append = if exact_payload {
            joined.collection_journal().unwrap().append().clone()
        } else {
            journal(b"unrelated journal winner")
        };
        let transaction = connection.transaction().unwrap();
        crate::operation::collection_journal::append_collection_entry(
            &transaction,
            &collection_handle(),
            &append,
        )
        .unwrap();
        transaction.commit().unwrap();

        // A separate operation advances the shared checkpoint with complete
        // native custody. None of the selected collection's records are present.
        let other = start_candidate("winner", ['5', '6', '7', 'b']);
        let checkpoint =
            checkpoint_transition(9, "checkpoint-9-one", '9', 10, "checkpoint-10-winner", 'b');
        let indexes = other
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
        let winner = OperationCustodyCandidate::new_start(
            store(),
            OperationCustodyTransitionKind::StartRoot,
            other.records().to_vec(),
            indexes,
            checkpoint.clone(),
            physical_accumulator(store(), &checkpoint),
            other.session_basis().unwrap().clone(),
        )
        .unwrap();
        assert_eq!(
            commit(&mut connection, &winner).unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
        for member in joined.records() {
            assert!(SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store(),
            }
            .read_operation_record(&store(), member.record().coordinate(), 4096)
            .unwrap()
            .is_none());
        }
        let changes = connection.total_changes();
        assert_eq!(
            observed(&mut connection, &joined),
            OperationCustodyReadBackOutcome::ConflictingOrPartial
        );
        assert_eq!(
            commit(&mut connection, &joined).unwrap_err().kind(),
            if exact_payload {
                ControlStoreErrorKind::RecoveryRequired
            } else {
                ControlStoreErrorKind::CandidateConflict
            }
        );
        assert_eq!(
            observed(&mut connection, &winner),
            OperationCustodyReadBackOutcome::ExactSuccessor
        );
        assert_eq!(connection.total_changes(), changes);
    }
}

#[test]
fn uncertain_collection_commit_reopens_at_both_predecessors_before_exact_retry() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("uncertain-collection.sqlite");
    let (start, _, joined) = candidates();
    {
        let mut connection = Connection::open(&path).unwrap();
        initialize_store(&connection);
        commit(&mut connection, &start).unwrap();
        // Reuse the native deferred-constraint seam: all candidate writes run,
        // but SQLite rejects their commit and the adapter must report uncertainty.
        connection
            .execute_batch(
                "CREATE TEMP TABLE commit_parent (id INTEGER PRIMARY KEY); \
             CREATE TEMP TABLE commit_child (parent_id INTEGER REFERENCES commit_parent(id) \
                 DEFERRABLE INITIALLY DEFERRED); \
             CREATE TEMP TRIGGER reject_collection_commit AFTER UPDATE \
                 ON pale_deployment_operation_checkpoint_head \
             BEGIN INSERT INTO commit_child(parent_id) VALUES (1); END;",
            )
            .unwrap();
        assert_eq!(
            commit(&mut connection, &joined).unwrap_err().kind(),
            ControlStoreErrorKind::OutcomeUnknown
        );
        assert_eq!(
            observed(&mut connection, &joined),
            OperationCustodyReadBackOutcome::Predecessor
        );
    }
    {
        let mut connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        assert_eq!(
            observed(&mut connection, &joined),
            OperationCustodyReadBackOutcome::Predecessor
        );
        assert_eq!(
            observed(&mut connection, &start),
            OperationCustodyReadBackOutcome::ExactSuccessor
        );
        assert_eq!(
            crate::operation::collection_journal::read_journal_head(
                &connection,
                &collection_handle()
            )
            .unwrap(),
            None
        );
        assert!(!crate::operation::collection_journal::journal_entry_exists(
            &connection,
            &collection_handle(),
            joined
                .collection_journal()
                .unwrap()
                .append()
                .entry()
                .sequence(),
        )
        .unwrap());
        assert_eq!(
            crate::store::transaction::read(&connection, &store()).unwrap(),
            joined.accumulator().binding().transaction().expected()
        );
        assert_eq!(
            commit(&mut connection, &joined).unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
        assert_eq!(
            crate::store::transaction::read(&connection, &store()).unwrap(),
            joined.accumulator().binding().transaction().successor()
        );
    }
    let mut connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    let changes = connection.total_changes();
    assert_eq!(
        observed(&mut connection, &joined),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    assert_eq!(
        commit(&mut connection, &joined).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(connection.total_changes(), changes);
}

#[path = "collection_journal_integrity.rs"]
mod journal_integrity;
