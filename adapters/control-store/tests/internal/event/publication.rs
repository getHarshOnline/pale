//! Native physical atomicity; records remain opaque and unadmitted here.

use super::*;

#[test]
fn derived_publication_failures_leave_neither_record_nor_outbox() {
    for trigger in [
        "CREATE TEMP TRIGGER reject_publication BEFORE INSERT ON pale_deployment_control_record WHEN NEW.record_ref = 'derived-frontier' BEGIN SELECT RAISE(ABORT, 'publication failure'); END;",
        "CREATE TEMP TRIGGER reject_publication BEFORE INSERT ON pale_deployment_event_index WHEN NEW.index_role = 'reconciliation_frontier' BEGIN SELECT RAISE(ABORT, 'second index failure'); END;",
        "CREATE TEMP TRIGGER reject_publication BEFORE INSERT ON pale_deployment_authority_journal BEGIN SELECT RAISE(ABORT, 'journal failure'); END;",
        "CREATE TEMP TRIGGER reject_publication BEFORE UPDATE ON pale_deployment_authority_head BEGIN SELECT RAISE(ABORT, 'head failure'); END;",
    ] {
        let mut connection = initialize_store();
        connection.execute_batch(trigger).unwrap();
        let (_, transition) = derived_publication(&mut connection, 1, "derived", '1', '2', None);
        let mut custody = SqliteEventCustody { connection: &mut connection, coordinate: store() };
        assert_eq!(custody.compare_and_commit(&transition, 8).unwrap_err().kind(), ControlStoreErrorKind::Storage);
        assert_eq!(custody.observe_exact(&transition, 8).unwrap(), EventCustodyReadBackOutcome::Predecessor);
        assert!(custody.read_pale_derived_outbox_page(&PaleDerivedOutboxPageRead::new(store(), None, 1).unwrap()).unwrap().entries().is_empty());
        assert_eq!(table_counts(&connection), (0, 0, 0, 0, 0));
        let sequence: i64 = connection.query_row("SELECT logical_transaction_sequence FROM pale_control_store_header WHERE singleton = 1", [], |row| row.get(0)).unwrap();
        assert_eq!(sequence, 1);
    }
}

#[test]
fn either_orphan_member_is_not_a_recoverable_atomic_publication() {
    for member_index in 0..2 {
        let mut connection = initialize_store();
        let (_, transition) = derived_publication(&mut connection, 1, "derived", '1', '2', None);
        insert_record(&connection, transition.members()[member_index].record()).unwrap();
        let before = table_counts(&connection);
        let mut custody = SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        assert_eq!(
            custody.observe_exact(&transition, 8).unwrap(),
            EventCustodyReadBackOutcome::ConflictingOrPartial
        );
        assert_eq!(
            custody
                .compare_and_commit(&transition, 8)
                .unwrap_err()
                .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        assert!(custody
            .read_pale_derived_outbox_page(
                &PaleDerivedOutboxPageRead::new(store(), None, 1).unwrap()
            )
            .unwrap()
            .entries()
            .is_empty());
        assert_eq!(table_counts(&connection), before);
    }
}
