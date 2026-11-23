//! A journal sequence is not candidate identity; comparison remains byte-bounded.

use super::*;

fn with_payload(candidate: &EventCustodyTransition, payload: Vec<u8>) -> EventCustodyTransition {
    let append = candidate.journal_append();
    let entry = SequencedAuthorityEntry::new(
        append.entry().sequence(),
        BoundedAuthorityEntryBytes::new(payload, 8).unwrap(),
    );
    EventCustodyTransition::new(
        candidate.store().clone(),
        candidate.transaction().clone(),
        candidate.kind(),
        PreparedAuthorityAppend::new(candidate.store().clone(), append.expected_head(), entry)
            .unwrap(),
        candidate.members().to_vec(),
        candidate.index_mutations().to_vec(),
        candidate.index_assertions().to_vec(),
    )
    .unwrap()
}

#[test]
fn a_valid_competing_entry_can_have_a_different_payload_length() {
    let mut connection = initialize_store();
    let (_, loser) = owner_event(&mut connection, 1, "loser", '1', '1');
    let (_, winner) = owner_event(&mut connection, 1, "winner", '2', '2');
    let winner = with_payload(&winner, vec![2, 3]);
    let mut custody = SqliteEventCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody.compare_and_commit(&winner, 2).unwrap(),
        EventCustodyCommitOutcome::Committed
    );
    assert_eq!(
        custody.compare_and_commit(&loser, 1).unwrap_err().kind(),
        ControlStoreErrorKind::BoundExceeded
    );
    assert_eq!(
        custody.observe_exact(&loser, 1).unwrap_err().kind(),
        ControlStoreErrorKind::BoundExceeded
    );
    assert_eq!(
        custody.compare_and_commit(&loser, 2).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    assert_eq!(
        custody.observe_exact(&loser, 2).unwrap(),
        EventCustodyReadBackOutcome::ConflictingOrPartial
    );
    assert_eq!(
        custody.compare_and_commit(&winner, 2).unwrap(),
        EventCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(table_counts(&connection), (1, 2, 3, 0, 0));
}

#[test]
fn zero_and_exact_payload_limits_do_not_change_candidate_identity() {
    let mut connection = initialize_store();
    let (_, candidate) = owner_event(&mut connection, 1, "bounded", '1', '1');
    {
        let mut custody = SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        assert_eq!(
            custody
                .compare_and_commit(&candidate, 0)
                .unwrap_err()
                .kind(),
            ControlStoreErrorKind::BoundExceeded
        );
        assert_eq!(
            custody.observe_exact(&candidate, 0).unwrap_err().kind(),
            ControlStoreErrorKind::BoundExceeded
        );
    }
    assert_eq!(table_counts(&connection), (0, 0, 0, 0, 0));
    let mut custody = SqliteEventCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody.compare_and_commit(&candidate, 1).unwrap(),
        EventCustodyCommitOutcome::Committed
    );
    assert_eq!(
        custody.observe_exact(&candidate, 1).unwrap(),
        EventCustodyReadBackOutcome::ExactSuccessor
    );
    assert_eq!(
        custody.compare_and_commit(&candidate, 8).unwrap(),
        EventCustodyCommitOutcome::ExactReplay
    );
}

#[test]
fn corrupt_and_oversized_competing_frames_are_not_ordinary_conflicts() {
    for oversized in [false, true] {
        let mut connection = initialize_store();
        let (_, loser) = owner_event(&mut connection, 1, "loser", '1', '1');
        let (_, winner) = owner_event(&mut connection, 1, "winner", '2', '2');
        let winner = with_payload(&winner, vec![2, 3]);
        assert_eq!(
            SqliteEventCustody {
                connection: &mut connection,
                coordinate: store()
            }
            .compare_and_commit(&winner, 8)
            .unwrap(),
            EventCustodyCommitOutcome::Committed
        );
        let mut damaged =
            crate::frame::encode(&[2, 3], pale_ports::AuthorityByteClass::JournalEntry).unwrap();
        damaged[0] ^= 1;
        if oversized {
            damaged.resize(4096, 0);
        }
        connection.execute("UPDATE pale_deployment_authority_journal SET physical_frame = ?1 WHERE journal_sequence = 1", [&damaged]).unwrap();
        let expected = if oversized {
            ControlStoreErrorKind::BoundExceeded
        } else {
            ControlStoreErrorKind::RecoveryRequired
        };
        let mut custody = SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        assert_eq!(
            custody.compare_and_commit(&loser, 8).unwrap_err().kind(),
            expected
        );
        if oversized {
            assert_eq!(
                custody.observe_exact(&loser, 8).unwrap_err().kind(),
                expected
            );
        } else {
            assert_eq!(
                custody.observe_exact(&loser, 8).unwrap(),
                EventCustodyReadBackOutcome::ConflictingOrPartial
            );
        }
        let stored: Vec<u8> = connection.query_row("SELECT physical_frame FROM pale_deployment_authority_journal WHERE journal_sequence = 1", [], |row| row.get(0)).unwrap();
        assert_eq!(
            stored, damaged,
            "failed comparison must not repair the winning entry"
        );
        assert_eq!(table_counts(&connection), (1, 2, 3, 0, 0));
    }
}
