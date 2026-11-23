//! Collection journal scope and partial-write refusal.

use super::*;
use crate::operation::collection_journal;

#[test]
fn collection_rejects_missing_or_malformed_head_and_rows_beyond_the_head() {
    for fault in 0..6 {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_store(&connection);
        let (start, observation, initial) = candidates();
        commit(&mut connection, &start).unwrap();
        // Model one already retained opaque collection entry. This tests physical
        // head consistency only, not semantic recovery of the historical prefix.
        let transaction = connection.transaction().unwrap();
        crate::operation::collection_journal::append_collection_entry(
            &transaction,
            &collection_handle(),
            &journal(b"predecessor journal payload"),
        )
        .unwrap();
        transaction.commit().unwrap();
        let append = PreparedAuthorityAppend::new(
            store(),
            Some(JournalSequenceV1::new(1).unwrap()),
            SequencedAuthorityEntry::new(
                JournalSequenceV1::new(2).unwrap(),
                BoundedAuthorityEntryBytes::new(b"collection journal payload".to_vec(), 64)
                    .unwrap(),
            ),
        )
        .unwrap();
        let pair = OperationCollectionJournal::prepare(
            start
                .record(OperationCustodyRecordKind::OperationHandle)
                .unwrap()
                .coordinate(),
            observation.accumulator().binding().transaction().clone(),
            append,
            4096,
            64,
        )
        .unwrap();
        let mut selected = observation.with_collection_journal(pair).unwrap();
        assert_eq!(
            observed(&mut connection, &selected),
            OperationCustodyReadBackOutcome::Predecessor
        );
        match fault {
            0 => {
                connection
                    .execute(
                        "DELETE FROM pale_deployment_collection_journal WHERE journal_sequence=1",
                        [],
                    )
                    .unwrap();
            }
            1 => {
                connection.execute("UPDATE pale_deployment_collection_journal SET physical_frame=x'00' WHERE journal_sequence=1", []).unwrap();
            }
            2 | 4 | 5 => {
                if fault == 4 {
                    connection
                        .execute(
                            "UPDATE pale_deployment_collection_head SET journal_sequence=NULL ",
                            [],
                        )
                        .unwrap();
                    connection.execute("DELETE FROM pale_deployment_collection_journal WHERE journal_sequence=1", []).unwrap();
                    selected = initial;
                } else if fault == 5 {
                    commit(&mut connection, &selected).unwrap();
                    assert_eq!(
                        observed(&mut connection, &selected),
                        OperationCustodyReadBackOutcome::ExactSuccessor
                    );
                }
                let frame = crate::frame::encode(
                    b"stray later entry",
                    pale_ports::AuthorityByteClass::JournalEntry,
                )
                .unwrap();
                connection.execute("INSERT INTO pale_deployment_collection_journal (handle_ref,handle_sha256,journal_sequence,physical_frame) VALUES (?2,?3,3,?1)", params![frame, collection_handle().record_ref().as_str(), collection_handle().record_sha256().as_str()]).unwrap();
            }
            3 => {
                let frame =
                    crate::frame::encode(&[b'x'; 65], pale_ports::AuthorityByteClass::JournalEntry)
                        .unwrap();
                connection.execute("UPDATE pale_deployment_collection_journal SET physical_frame=?1 WHERE journal_sequence=1", [frame]).unwrap();
            }
            _ => unreachable!(),
        }
        let changes = connection.total_changes();
        let read_back = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .observe_exact(&selected);
        let expected = if fault == 3 {
            assert_eq!(
                read_back.unwrap_err().kind(),
                ControlStoreErrorKind::BoundExceeded
            );
            ControlStoreErrorKind::BoundExceeded
        } else {
            assert_eq!(
                read_back.unwrap(),
                OperationCustodyReadBackOutcome::ConflictingOrPartial
            );
            ControlStoreErrorKind::RecoveryRequired
        };
        assert_eq!(
            commit(&mut connection, &selected).unwrap_err().kind(),
            expected,
            "fault {fault}"
        );
        assert_eq!(connection.total_changes(), changes, "fault {fault}");
    }
}

#[test]
fn collections_and_authority_keep_independent_sequences_and_payloads() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let (start, _, joined) = candidates();
    commit(&mut connection, &start).unwrap();
    let handle = collection_handle();
    let same_ref = ControlRecordCoordinate::new(
        handle.record_ref().clone(),
        ControlRecordDigest::parse(digest('e')).unwrap(),
    );
    let same_digest = ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse("another-operation-handle").unwrap(),
        handle.record_sha256().clone(),
    );
    let authority = journal(b"authority state");
    let first = journal(b"other collection sharing the reference");
    let second = journal(b"other collection sharing the digest");
    {
        let transaction = connection.transaction().unwrap();
        crate::store::journal::append_authority_entry(&transaction, &authority).unwrap();
        collection_journal::append_collection_entry(&transaction, &same_ref, &first).unwrap();
        collection_journal::append_collection_entry(&transaction, &same_digest, &second).unwrap();
        transaction.commit().unwrap();
    }
    // All four histories can start at one. Authority and the other handles
    // cannot make this operation's first collection stale.
    assert_eq!(
        commit(&mut connection, &joined).unwrap(),
        OperationCustodyCommitOutcome::Committed
    );
    let pair = joined.collection_journal().unwrap();
    for (selected, entry) in [
        (&handle, pair.append()),
        (&same_ref, &first),
        (&same_digest, &second),
    ] {
        assert_eq!(
            collection_journal::read_journal_head(&connection, selected).unwrap(),
            Some(entry.entry().sequence())
        );
        assert!(
            collection_journal::journal_entry_matches(&connection, selected, entry, 64).unwrap()
        );
    }
    let next = successor_journal(b"second collection sharing the reference");
    {
        let transaction = connection.transaction().unwrap();
        collection_journal::append_collection_entry(&transaction, &same_ref, &next).unwrap();
        transaction.commit().unwrap();
    }
    let changes = connection.total_changes();
    assert_eq!(
        commit(&mut connection, &joined).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(connection.total_changes(), changes);
    assert!(crate::store::journal::journal_entry_matches(&connection, &authority, 64).unwrap());
    assert_eq!(
        crate::store::journal::read_journal_head(&connection).unwrap(),
        Some(JournalSequenceV1::new(1).unwrap())
    );
    assert_eq!(
        collection_journal::read_journal_entry(
            &connection,
            &same_digest,
            JournalSequenceV1::new(2).unwrap(),
            64
        )
        .unwrap(),
        None
    );
}
