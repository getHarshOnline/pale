//! Behavioral tests for deployment Event/Genealogy custody.

use pale_core::records::{
    ControlIncarnationIdV1, ControlStoreIdV1, JournalSequenceV1, ResidentRecordRefV1, SchemaDigest,
    SchemaId, EVENT_REFERENCE_V1_SCHEMA,
};
use pale_ports::{
    AuthorityStoreCoordinate, BoundedAuthorityEntryBytes, ControlRecord, ControlRecordBytes,
    ControlRecordCoordinate, ControlRecordDigest, ControlStoreErrorKind, EventCustodyCommitOutcome,
    EventCustodyPort, EventCustodyReadBackOutcome, EventCustodyTransition,
    EventCustodyTransitionKind, EventInboxPageBoundary, EventInboxPageRead, EventIndexAssertion,
    EventIndexKey, EventIndexMutation, EventIndexPredecessor, EventIndexQuery,
    EventTransitionMember, EventTransitionMemberKind, PaleDerivedOutboxPageBoundary,
    PaleDerivedOutboxPageRead, PreparedAuthorityAppend, SequencedAuthorityEntry,
};
use rusqlite::{params, Connection};

use super::{read_index_coordinate, SqliteEventCustody};
use crate::deployment::{insert_record, read_record};

#[path = "event/publication.rs"]
mod publication;

#[path = "event/history.rs"]
mod history;

#[path = "event/journal_comparison.rs"]
mod journal_comparison;

const MIGRATION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/deployment-control/0001_initial_v1.sql"
));

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn digest(byte: char) -> ControlRecordDigest {
    ControlRecordDigest::parse(sha(byte)).expect("record digest")
}

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record reference"),
        digest(byte),
    )
}

fn record(reference: &str, byte: char, schema: &str) -> ControlRecord {
    ControlRecord::new(
        coordinate(reference, byte),
        SchemaId::parse(schema).expect("schema identity"),
        SchemaDigest::parse(sha('f')).expect("schema digest"),
        ControlRecordBytes::new(format!(r#"{{"record":"{reference}"}}"#).into_bytes())
            .expect("record bytes"),
    )
}

fn private_record(reference: &str, byte: char, kind: EventTransitionMemberKind) -> ControlRecord {
    record(reference, byte, kind.required_schema_id())
}

fn key(byte: char) -> EventIndexKey {
    EventIndexKey::new(digest(byte))
}

fn store() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("deployment-store-1").expect("store identity"),
        ControlIncarnationIdV1::parse("deployment-incarnation-1").expect("incarnation"),
    )
}

fn append(sequence: u64) -> PreparedAuthorityAppend {
    let sequence = JournalSequenceV1::new(sequence).expect("journal sequence");
    PreparedAuthorityAppend::new(
        store(),
        (sequence.get() > 1)
            .then(|| JournalSequenceV1::new(sequence.get() - 1).expect("predecessor sequence")),
        SequencedAuthorityEntry::new(
            sequence,
            BoundedAuthorityEntryBytes::new(vec![u8::try_from(sequence.get()).unwrap()], 8)
                .expect("journal entry"),
        ),
    )
    .expect("prepared authority append")
}

fn owner_event(
    connection: &mut Connection,
    sequence: u64,
    reference: &str,
    record_byte: char,
    identity_key: char,
) -> (ControlRecord, EventCustodyTransition) {
    let event = record(reference, record_byte, EVENT_REFERENCE_V1_SCHEMA);
    let witness = private_record(
        &format!("event-commit-witness-{reference}"),
        identity_key,
        EventTransitionMemberKind::EventCommitWitness,
    );
    let transition = EventCustodyTransition::new(
        store(),
        prepare_transaction(connection),
        EventCustodyTransitionKind::AdmitOwnerReference,
        append(sequence),
        vec![
            EventTransitionMember::new(EventTransitionMemberKind::EventReference, event.clone()),
            EventTransitionMember::new(
                EventTransitionMemberKind::EventCommitWitness,
                witness.clone(),
            ),
        ],
        vec![
            EventIndexMutation::new(
                EventIndexQuery::EventIdentity(key(identity_key)),
                EventIndexPredecessor::Absent,
                event.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::OwnerSource(key(identity_key)),
                EventIndexPredecessor::Absent,
                event.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::EventCommitWitness(key(identity_key)),
                EventIndexPredecessor::Absent,
                witness.coordinate().clone(),
            ),
        ],
        Vec::new(),
    )
    .expect("owner Event transition");
    (event, transition)
}

fn subscription(
    connection: &mut Connection,
    sequence: u64,
) -> (ControlRecord, EventCustodyTransition) {
    let subscription = private_record(
        "subscription-1",
        '2',
        EventTransitionMemberKind::Subscription,
    );
    let transition = EventCustodyTransition::new(
        store(),
        prepare_transaction(connection),
        EventCustodyTransitionKind::AdmitSubscription,
        append(sequence),
        vec![EventTransitionMember::new(
            EventTransitionMemberKind::Subscription,
            subscription.clone(),
        )],
        vec![EventIndexMutation::new(
            EventIndexQuery::Subscription(key('2')),
            EventIndexPredecessor::Absent,
            subscription.coordinate().clone(),
        )],
        Vec::new(),
    )
    .expect("subscription transition");
    (subscription, transition)
}

fn derived_publication(
    connection: &mut Connection,
    sequence: u64,
    reference: &str,
    record_byte: char,
    publication_key: char,
    predecessor: Option<&ControlRecord>,
) -> (ControlRecord, EventCustodyTransition) {
    let retention = private_record(
        reference,
        record_byte,
        EventTransitionMemberKind::PaleDerivedOutbox,
    );
    // These are opaque physical role fixtures, not signed frontier admission.
    let mut members = vec![EventTransitionMember::new(
        EventTransitionMemberKind::PaleDerivedOutbox,
        retention.clone(),
    )];
    let mut mutations = vec![EventIndexMutation::new(
        EventIndexQuery::PaleDerivedOutbox(key(publication_key)),
        predecessor.map_or(EventIndexPredecessor::Absent, |record| {
            EventIndexPredecessor::Exact(record.clone())
        }),
        retention.coordinate().clone(),
    )];
    if predecessor.is_none() {
        let publication = private_record(
            &format!("{reference}-frontier"),
            record_byte,
            EventTransitionMemberKind::ReconciliationFrontier,
        );
        mutations.push(EventIndexMutation::new(
            EventIndexQuery::ReconciliationFrontier(key(publication_key)),
            EventIndexPredecessor::Absent,
            publication.coordinate().clone(),
        ));
        members.push(EventTransitionMember::new(
            EventTransitionMemberKind::ReconciliationFrontier,
            publication,
        ));
    }
    let transition = EventCustodyTransition::new(
        store(),
        prepare_transaction(connection),
        if predecessor.is_some() {
            EventCustodyTransitionKind::SettlePaleDerivedPublication
        } else {
            EventCustodyTransitionKind::AdvanceReconciliationFrontier
        },
        append(sequence),
        members,
        mutations,
        Vec::new(),
    )
    .expect("Pale-derived publication transition");
    (retention, transition)
}

struct Delivery<'event> {
    sequence: u64,
    event: &'event ControlRecord,
    event_key: char,
    subscription: &'event ControlRecord,
    delivery_key: char,
    attempt_key: char,
    suffix: &'event str,
    record_byte: char,
    previous_inbox: Option<&'event ControlRecord>,
}

fn delivery(
    connection: &mut Connection,
    input: Delivery<'_>,
) -> (ControlRecord, ControlRecord, EventCustodyTransition) {
    let inbox = private_record(
        &format!("inbox-{}", input.suffix),
        input.record_byte,
        EventTransitionMemberKind::InboxDelivery,
    );
    let attempt = private_record(
        &format!("attempt-{}", input.suffix),
        input.attempt_key,
        EventTransitionMemberKind::DeliveryAttempt,
    );
    let stream = private_record(
        &format!("delivery-stream-{}", input.suffix),
        input.attempt_key,
        EventTransitionMemberKind::DeliveryStream,
    );
    let witness = private_record(
        &format!("delivery-transition-witness-{}", input.suffix),
        input.attempt_key,
        EventTransitionMemberKind::TransitionWitness,
    );
    let stream_query = EventIndexQuery::DeliveryStream(key(input.delivery_key));
    let stream_predecessor = if input.previous_inbox.is_some() {
        read_index_coordinate(connection, &stream_query)
            .expect("read replay stream predecessor")
            .and_then(|coordinate| read_record(connection, &coordinate).expect("read stream"))
            .map(EventIndexPredecessor::Exact)
            .expect("replay stream predecessor")
    } else {
        EventIndexPredecessor::Absent
    };
    let transition = EventCustodyTransition::new(
        store(),
        prepare_transaction(connection),
        if input.previous_inbox.is_some() {
            EventCustodyTransitionKind::EnqueueReplay
        } else {
            EventCustodyTransitionKind::EnqueueDelivery
        },
        append(input.sequence),
        vec![
            EventTransitionMember::new(EventTransitionMemberKind::InboxDelivery, inbox.clone()),
            EventTransitionMember::new(EventTransitionMemberKind::DeliveryAttempt, attempt.clone()),
            EventTransitionMember::new(EventTransitionMemberKind::DeliveryStream, stream.clone()),
            EventTransitionMember::new(
                EventTransitionMemberKind::TransitionWitness,
                witness.clone(),
            ),
        ],
        vec![
            EventIndexMutation::new(
                EventIndexQuery::InboxDelivery(key(input.delivery_key)),
                input
                    .previous_inbox
                    .map_or(EventIndexPredecessor::Absent, |record| {
                        EventIndexPredecessor::Exact(record.clone())
                    }),
                inbox.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::DeliveryAttempt(key(input.attempt_key)),
                EventIndexPredecessor::Absent,
                attempt.coordinate().clone(),
            ),
            EventIndexMutation::new(
                stream_query,
                stream_predecessor,
                stream.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::TransitionWitness(key(input.attempt_key)),
                EventIndexPredecessor::Absent,
                witness.coordinate().clone(),
            ),
        ],
        vec![
            EventIndexAssertion::new(
                EventIndexQuery::EventIdentity(key(input.event_key)),
                EventIndexPredecessor::Exact(input.event.clone()),
            ),
            EventIndexAssertion::new(
                EventIndexQuery::Subscription(key('2')),
                EventIndexPredecessor::Exact(input.subscription.clone()),
            ),
        ],
    )
    .expect("delivery transition");
    (inbox, attempt, transition)
}

fn acknowledgement(
    connection: &mut Connection,
    sequence: u64,
    delivery_key: char,
    attempt_key: char,
    previous_inbox: &ControlRecord,
    attempt: &ControlRecord,
) -> (ControlRecord, EventCustodyTransition) {
    let inbox = private_record(
        &format!("inbox-acknowledged-{delivery_key}"),
        '0',
        EventTransitionMemberKind::InboxDelivery,
    );
    let acknowledgement = private_record(
        &format!("acknowledgement-{delivery_key}"),
        '6',
        EventTransitionMemberKind::EventAcknowledgement,
    );
    let witness = private_record(
        &format!("acknowledgement-transition-witness-{delivery_key}"),
        delivery_key,
        EventTransitionMemberKind::TransitionWitness,
    );
    let transition = EventCustodyTransition::new(
        store(),
        prepare_transaction(connection),
        EventCustodyTransitionKind::RecordAcknowledgement,
        append(sequence),
        vec![
            EventTransitionMember::new(
                EventTransitionMemberKind::EventAcknowledgement,
                acknowledgement.clone(),
            ),
            EventTransitionMember::new(EventTransitionMemberKind::InboxDelivery, inbox.clone()),
            EventTransitionMember::new(
                EventTransitionMemberKind::TransitionWitness,
                witness.clone(),
            ),
        ],
        vec![
            EventIndexMutation::new(
                EventIndexQuery::Acknowledgement(key('6')),
                EventIndexPredecessor::Absent,
                acknowledgement.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::InboxDelivery(key(delivery_key)),
                EventIndexPredecessor::Exact(previous_inbox.clone()),
                inbox.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::TransitionWitness(key(delivery_key)),
                EventIndexPredecessor::Absent,
                witness.coordinate().clone(),
            ),
        ],
        vec![EventIndexAssertion::new(
            EventIndexQuery::DeliveryAttempt(key(attempt_key)),
            EventIndexPredecessor::Exact(attempt.clone()),
        )],
    )
    .expect("acknowledgement transition");
    (inbox, transition)
}

fn initialize_store() -> Connection {
    initialize_connection(Connection::open_in_memory().expect("in-memory SQLite"))
}

fn initialize_connection(connection: Connection) -> Connection {
    connection
        .pragma_update(None, "foreign_keys", true)
        .expect("foreign keys");
    connection.execute_batch(MIGRATION).expect("migration");
    connection
        .execute(
            "INSERT INTO pale_control_store_header \
             (singleton, store_kind, logical_store_id, control_incarnation_id, store_schema_id, \
              store_schema_sha256, store_schema_version, storage_manifest_sha256, migration_head_sha256, logical_transaction_sequence) \
             VALUES (1, 'deployment_control', ?1, ?2, \
                     'pale/control-store/deployment-control.v1', ?3, 1, ?4, ?5, 1)",
            params![
                store().store_id().as_str(),
                store().incarnation_id().as_str(),
                sha('a'),
                sha('b'),
                sha('c'),
            ],
        )
        .expect("store header");
    connection
}

fn commit(connection: &mut Connection, transition: &EventCustodyTransition) {
    assert_eq!(
        SqliteEventCustody {
            connection,
            coordinate: store(),
        }
        .compare_and_commit(transition, 8)
        .expect("Event transition commit"),
        EventCustodyCommitOutcome::Committed
    );
}

fn prepare_transaction(connection: &mut Connection) -> pale_ports::ControlTransactionPreparation {
    use pale_ports::ControlTransactionPort;
    SqliteEventCustody {
        connection,
        coordinate: store(),
    }
    .prepare_transaction(&store())
    .expect("actual transaction predecessor")
}

fn table_counts(connection: &Connection) -> (i64, i64, i64, i64, i64) {
    connection
        .query_row(
            "SELECT \
             (SELECT COUNT(*) FROM pale_deployment_authority_journal), \
             (SELECT COUNT(*) FROM pale_deployment_control_record), \
             (SELECT COUNT(*) FROM pale_deployment_event_index), \
             (SELECT COUNT(*) FROM pale_deployment_event_inbox_order), \
             (SELECT COUNT(*) FROM pale_deployment_event_inbox_version)",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("Event custody row counts")
}

#[test]
fn commit_replay_and_same_identity_conflict_keep_one_authority_history() {
    let mut connection = initialize_store();
    let (event, transition) = owner_event(&mut connection, 1, "event-1", '1', '1');
    assert_eq!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .observe_exact(&transition, 8)
        .unwrap(),
        EventCustodyReadBackOutcome::Predecessor
    );
    assert_eq!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .compare_and_commit(&transition, 8)
        .unwrap(),
        EventCustodyCommitOutcome::Committed
    );
    assert_eq!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .compare_and_commit(&transition, 8)
        .unwrap(),
        EventCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(table_counts(&connection), (1, 2, 3, 0, 0));
    assert_eq!(
        crate::store::transaction::read(&connection, &store())
            .unwrap()
            .get(),
        2,
        "the Event and its journal append share one mutation transaction"
    );

    let (_, conflicting) = owner_event(&mut connection, 2, "event-conflicting", '3', '1');
    assert_eq!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .compare_and_commit(&conflicting, 8)
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::CandidateConflict
    );

    let conflict = private_record(
        "event-conflict-1",
        '4',
        EventTransitionMemberKind::IdentityConflict,
    );
    let quarantine = EventCustodyTransition::new(
        store(),
        prepare_transaction(&mut connection),
        EventCustodyTransitionKind::QuarantineIdentityConflict,
        append(2),
        vec![EventTransitionMember::new(
            EventTransitionMemberKind::IdentityConflict,
            conflict.clone(),
        )],
        vec![EventIndexMutation::new(
            EventIndexQuery::IdentityConflict(key('4')),
            EventIndexPredecessor::Absent,
            conflict.coordinate().clone(),
        )],
        vec![EventIndexAssertion::new(
            EventIndexQuery::EventIdentity(key('1')),
            EventIndexPredecessor::Exact(event),
        )],
    )
    .expect("quarantine transition");
    commit(&mut connection, &quarantine);
    assert_eq!(table_counts(&connection), (2, 3, 4, 0, 0));
    assert_eq!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .compare_and_commit(&transition, 8)
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::CandidateConflict
    );
}

#[test]
fn committed_event_survives_reopen_for_exact_recovery_observation() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("deployment-control.sqlite3");
    let (event, transition) = {
        let mut connection =
            initialize_connection(Connection::open(&path).expect("file-backed connection"));
        let (event, transition) = owner_event(&mut connection, 1, "event-1", '1', '1');
        commit(&mut connection, &transition);
        (event, transition)
    };

    let mut reopened = Connection::open(&path).expect("reopen deployment store");
    reopened
        .pragma_update(None, "foreign_keys", true)
        .expect("foreign keys");
    let changes_before = reopened.total_changes();
    let mut custody = SqliteEventCustody {
        connection: &mut reopened,
        coordinate: store(),
    };
    assert_eq!(
        custody.observe_exact(&transition, 8).unwrap(),
        EventCustodyReadBackOutcome::ExactSuccessor
    );
    assert_eq!(
        custody
            .read_record(&store(), event.coordinate(), 16 * 1024)
            .unwrap()
            .record(),
        Some(&event)
    );
    for query in [
        EventIndexQuery::EventIdentity(key('1')),
        EventIndexQuery::OwnerSource(key('1')),
    ] {
        assert_eq!(
            custody
                .read_index(&store(), &query, 16 * 1024)
                .unwrap()
                .record(),
            Some(&event)
        );
    }
    assert_eq!(reopened.total_changes(), changes_before);
}

#[test]
fn event_point_reads_require_a_positive_limit_before_loading_the_body() {
    let mut connection = initialize_store();
    let (event, transition) = owner_event(&mut connection, 1, "event-1", '1', '1');
    commit(&mut connection, &transition);
    let exact = event.canonical_record_jcs().as_slice().len();
    let query = EventIndexQuery::OwnerSource(key('1'));
    let mut custody = SqliteEventCustody {
        connection: &mut connection,
        coordinate: store(),
    };

    assert_eq!(
        custody
            .read_index(&store(), &query, exact)
            .unwrap()
            .record(),
        Some(&event)
    );
    assert_eq!(
        custody
            .read_record(&store(), event.coordinate(), exact)
            .unwrap()
            .record(),
        Some(&event)
    );
    for maximum_body_bytes in [0, exact - 1] {
        assert_eq!(
            custody
                .read_index(&store(), &query, maximum_body_bytes)
                .unwrap_err()
                .kind(),
            ControlStoreErrorKind::BoundExceeded
        );
        assert_eq!(
            custody
                .read_record(&store(), event.coordinate(), maximum_body_bytes)
                .unwrap_err()
                .kind(),
            ControlStoreErrorKind::BoundExceeded
        );
    }
    assert_eq!(
        custody
            .read_index(&store(), &EventIndexQuery::EventIdentity(key('f')), 0,)
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::BoundExceeded,
        "zero is invalid even when the index is absent"
    );
}

#[test]
fn committed_derived_publication_survives_restart_as_one_current_recovery_head() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("deployment-control.sqlite3");
    let (retained, retain) = {
        let mut connection =
            initialize_connection(Connection::open(&path).expect("file-backed connection"));
        let (retained, retain) =
            derived_publication(&mut connection, 1, "derived-retained", '1', '2', None);
        commit(&mut connection, &retain);
        (retained, retain)
    };

    let mut reopened = Connection::open(&path).expect("reopen deployment store");
    reopened
        .pragma_update(None, "foreign_keys", true)
        .expect("foreign keys");
    let request =
        PaleDerivedOutboxPageRead::new(store(), None, 1).expect("derived recovery request");
    let mut custody = SqliteEventCustody {
        connection: &mut reopened,
        coordinate: store(),
    };
    let page = custody
        .read_pale_derived_outbox_page(&request)
        .expect("recovered derived head");
    assert_eq!(page.entries().len(), 1);
    assert_eq!(page.entries()[0].record(), &retained);
    assert_eq!(page.boundary(), &PaleDerivedOutboxPageBoundary::Complete);
    for member in retain.members() {
        assert_eq!(
            custody
                .read_record(&store(), member.record().coordinate(), 16 * 1024)
                .unwrap()
                .record(),
            Some(member.record())
        );
    }
    assert_eq!(
        custody.observe_exact(&retain, 8).unwrap(),
        EventCustodyReadBackOutcome::ExactSuccessor
    );
    assert_eq!(
        custody
            .compare_and_commit(&retain, 8)
            .expect("exact replay"),
        EventCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(
        custody
            .read_pale_derived_outbox_page(&request)
            .expect("recovery after exact replay")
            .entries(),
        page.entries()
    );
    let stamps: (i64, i64, i64) = reopened.query_row(
        "SELECT COUNT(*), MIN(inserted_transaction_sequence), MAX(inserted_transaction_sequence) FROM pale_deployment_control_record",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    assert_eq!(stamps, (2, 2, 2));

    let (settled, settle) = derived_publication(
        &mut reopened,
        2,
        "derived-settled",
        '3',
        '2',
        Some(&retained),
    );
    commit(&mut reopened, &settle);
    let after_settlement = SqliteEventCustody {
        connection: &mut reopened,
        coordinate: store(),
    }
    .read_pale_derived_outbox_page(&request)
    .expect("current derived head after settlement");
    assert_eq!(after_settlement.entries().len(), 1);
    assert_eq!(after_settlement.entries()[0].record(), &settled);
    assert_eq!(table_counts(&reopened), (2, 3, 2, 0, 0));
}

#[test]
fn derived_recovery_pages_follow_strict_key_order_and_exact_continuation() {
    let mut connection = initialize_store();
    for (sequence, reference, record_byte, publication_key) in [
        (1, "derived-three", '4', '3'),
        (2, "derived-one", '5', '1'),
        (3, "derived-two", '6', '2'),
    ] {
        let (_, transition) = derived_publication(
            &mut connection,
            sequence,
            reference,
            record_byte,
            publication_key,
            None,
        );
        commit(&mut connection, &transition);
    }

    let first_request =
        PaleDerivedOutboxPageRead::new(store(), None, 2).expect("first recovery page");
    let first = SqliteEventCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .read_pale_derived_outbox_page(&first_request)
    .expect("first derived recovery page");
    assert_eq!(
        first
            .entries()
            .iter()
            .map(|entry| entry.cursor().publication_key().as_str())
            .collect::<Vec<_>>(),
        [sha('1'), sha('2')]
    );
    let PaleDerivedOutboxPageBoundary::More { resume_after } = first.boundary() else {
        panic!("two bounded rows must expose continuation")
    };

    let second_request = PaleDerivedOutboxPageRead::new(store(), Some(resume_after.clone()), 2)
        .expect("continued recovery page");
    let second = SqliteEventCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .read_pale_derived_outbox_page(&second_request)
    .expect("continued derived recovery page");
    assert_eq!(second.entries().len(), 1);
    assert_eq!(
        second.entries()[0].cursor().publication_key().as_str(),
        sha('3')
    );
    assert_eq!(second.boundary(), &PaleDerivedOutboxPageBoundary::Complete);
}

#[test]
fn every_failed_boundary_rolls_back_records_indexes_and_journal_together() {
    for trigger in [
        "CREATE TEMP TRIGGER reject_boundary BEFORE INSERT \
         ON pale_deployment_control_record \
         BEGIN SELECT RAISE(ABORT, 'injected Event record failure'); END;",
        "CREATE TEMP TRIGGER reject_boundary BEFORE INSERT \
         ON pale_deployment_event_index \
         BEGIN SELECT RAISE(ABORT, 'injected Event index failure'); END;",
        "CREATE TEMP TRIGGER reject_owner_source BEFORE INSERT \
         ON pale_deployment_event_index WHEN NEW.index_role = 'owner_source' \
         BEGIN SELECT RAISE(ABORT, 'injected owner-source index failure'); END;",
        "CREATE TEMP TRIGGER reject_boundary BEFORE INSERT \
         ON pale_deployment_authority_journal \
         BEGIN SELECT RAISE(ABORT, 'injected Event journal failure'); END;",
        "CREATE TEMP TRIGGER reject_boundary BEFORE UPDATE \
         ON pale_deployment_authority_head \
         BEGIN SELECT RAISE(ABORT, 'injected Event head failure'); END;",
    ] {
        let mut connection = initialize_store();
        connection
            .execute_batch(trigger)
            .expect("failure injection");
        let (_, transition) = owner_event(&mut connection, 1, "event-1", '1', '1');
        assert_eq!(
            SqliteEventCustody {
                connection: &mut connection,
                coordinate: store(),
            }
            .compare_and_commit(&transition, 8)
            .unwrap_err()
            .kind(),
            ControlStoreErrorKind::Storage
        );
        assert_eq!(
            SqliteEventCustody {
                connection: &mut connection,
                coordinate: store(),
            }
            .observe_exact(&transition, 8)
            .unwrap(),
            EventCustodyReadBackOutcome::Predecessor
        );
        assert_eq!(table_counts(&connection), (0, 0, 0, 0, 0));
    }

    let mut connection = initialize_store();
    let (_, transition) = owner_event(&mut connection, 1, "event-1", '1', '1');
    insert_record(&connection, transition.members()[0].record()).expect("partial record");
    assert_eq!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .compare_and_commit(&transition, 8)
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(table_counts(&connection), (0, 1, 0, 0, 0));
}

#[test]
fn inbox_projection_failures_roll_back_delivery_records_and_indexes() {
    for trigger in [
        "CREATE TEMP TRIGGER reject_inbox_projection BEFORE INSERT \
         ON pale_deployment_event_inbox_order \
         BEGIN SELECT RAISE(ABORT, 'injected Event inbox-order failure'); END;",
        "CREATE TEMP TRIGGER reject_inbox_projection BEFORE INSERT \
         ON pale_deployment_event_inbox_version \
         BEGIN SELECT RAISE(ABORT, 'injected Event inbox-version failure'); END;",
    ] {
        let mut connection = initialize_store();
        let (event, event_transition) = owner_event(&mut connection, 1, "event-1", '1', '1');
        commit(&mut connection, &event_transition);
        let (subscription, subscription_transition) = subscription(&mut connection, 2);
        commit(&mut connection, &subscription_transition);
        let (_, _, delivery) = delivery(
            &mut connection,
            Delivery {
                sequence: 3,
                event: &event,
                event_key: '1',
                subscription: &subscription,
                delivery_key: '3',
                attempt_key: '4',
                suffix: "one",
                record_byte: '5',
                previous_inbox: None,
            },
        );
        let before = table_counts(&connection);
        connection
            .execute_batch(trigger)
            .expect("inbox projection failure injection");
        assert_eq!(
            SqliteEventCustody {
                connection: &mut connection,
                coordinate: store(),
            }
            .compare_and_commit(&delivery, 8)
            .unwrap_err()
            .kind(),
            ControlStoreErrorKind::Storage
        );
        assert_eq!(table_counts(&connection), before);
        assert_eq!(
            SqliteEventCustody {
                connection: &mut connection,
                coordinate: store(),
            }
            .observe_exact(&delivery, 8)
            .unwrap(),
            EventCustodyReadBackOutcome::Predecessor
        );
    }
}

#[test]
fn commit_failure_reports_unknown_outcome_for_exact_read_back() {
    let mut connection = initialize_store();
    connection
        .execute_batch(
            "CREATE TEMP TABLE commit_parent (id INTEGER PRIMARY KEY); \
             CREATE TEMP TABLE commit_child (parent_id INTEGER REFERENCES commit_parent(id) \
                 DEFERRABLE INITIALLY DEFERRED); \
             CREATE TEMP TRIGGER reject_commit AFTER INSERT \
                 ON pale_deployment_authority_journal \
             BEGIN INSERT INTO commit_child(parent_id) VALUES (1); END;",
        )
        .expect("deferred commit failure injection");
    let (_, transition) = owner_event(&mut connection, 1, "event-1", '1', '1');
    assert_eq!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .compare_and_commit(&transition, 8)
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::OutcomeUnknown
    );
    assert_eq!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .observe_exact(&transition, 8)
        .unwrap(),
        EventCustodyReadBackOutcome::Predecessor
    );
    assert_eq!(table_counts(&connection), (0, 0, 0, 0, 0));
}

#[test]
fn replay_requires_the_original_stable_inbox_order() {
    let mut connection = initialize_store();
    let (event, event_transition) = owner_event(&mut connection, 1, "event-1", '1', '1');
    commit(&mut connection, &event_transition);
    let (subscription, subscription_transition) = subscription(&mut connection, 2);
    commit(&mut connection, &subscription_transition);
    let (inbox, _, initial_delivery) = delivery(
        &mut connection,
        Delivery {
            sequence: 3,
            event: &event,
            event_key: '1',
            subscription: &subscription,
            delivery_key: '3',
            attempt_key: '4',
            suffix: "one",
            record_byte: '5',
            previous_inbox: None,
        },
    );
    commit(&mut connection, &initial_delivery);
    connection
        .execute("DELETE FROM pale_deployment_event_inbox_version", [])
        .unwrap();
    connection
        .execute("DELETE FROM pale_deployment_event_inbox_order", [])
        .unwrap();
    let (_, _, replay) = delivery(
        &mut connection,
        Delivery {
            sequence: 4,
            event: &event,
            event_key: '1',
            subscription: &subscription,
            delivery_key: '3',
            attempt_key: '6',
            suffix: "one-replay",
            record_byte: '7',
            previous_inbox: Some(&inbox),
        },
    );
    assert_eq!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .compare_and_commit(&replay, 8)
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(table_counts(&connection), (3, 7, 8, 0, 0));
}

#[test]
fn replay_does_not_duplicate_fanout_and_fixed_frontier_pages_preserve_gaps() {
    let mut connection = initialize_store();
    let (event_one, event_one_transition) = owner_event(&mut connection, 1, "event-1", '1', '1');
    commit(&mut connection, &event_one_transition);
    let (subscription, subscription_transition) = subscription(&mut connection, 2);
    commit(&mut connection, &subscription_transition);
    let (inbox_one, _, delivery_one) = delivery(
        &mut connection,
        Delivery {
            sequence: 3,
            event: &event_one,
            event_key: '1',
            subscription: &subscription,
            delivery_key: '3',
            attempt_key: '4',
            suffix: "one",
            record_byte: '5',
            previous_inbox: None,
        },
    );
    commit(&mut connection, &delivery_one);
    assert_eq!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .compare_and_commit(&delivery_one, 8)
        .unwrap(),
        EventCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(table_counts(&connection).3, 1);
    assert_eq!(table_counts(&connection).4, 1);

    let (replayed_inbox, _, replay) = delivery(
        &mut connection,
        Delivery {
            sequence: 4,
            event: &event_one,
            event_key: '1',
            subscription: &subscription,
            delivery_key: '3',
            attempt_key: '6',
            suffix: "one-replay",
            record_byte: '7',
            previous_inbox: Some(&inbox_one),
        },
    );
    commit(&mut connection, &replay);
    assert_eq!(table_counts(&connection).3, 1);
    assert_eq!(table_counts(&connection).4, 2);

    let (event_two, event_two_transition) = owner_event(&mut connection, 5, "event-2", '8', '8');
    commit(&mut connection, &event_two_transition);
    let (_, _, delivery_two) = delivery(
        &mut connection,
        Delivery {
            sequence: 6,
            event: &event_two,
            event_key: '8',
            subscription: &subscription,
            delivery_key: '9',
            attempt_key: 'a',
            suffix: "two",
            record_byte: 'b',
            previous_inbox: None,
        },
    );
    commit(&mut connection, &delivery_two);
    let (event_three, event_three_transition) =
        owner_event(&mut connection, 7, "event-3", 'c', 'c');
    commit(&mut connection, &event_three_transition);
    let (_, _, delivery_three) = delivery(
        &mut connection,
        Delivery {
            sequence: 8,
            event: &event_three,
            event_key: 'c',
            subscription: &subscription,
            delivery_key: 'd',
            attempt_key: 'e',
            suffix: "three",
            record_byte: 'f',
            previous_inbox: None,
        },
    );
    commit(&mut connection, &delivery_three);

    let first_request = EventInboxPageRead::new(
        store(),
        key('2'),
        None,
        JournalSequenceV1::new(8).unwrap(),
        2,
        1 << 20,
        1 << 20,
    )
    .unwrap();
    let first = SqliteEventCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .read_inbox_page(&first_request)
    .expect("first page");
    let undersized = EventInboxPageRead::new(
        store(),
        key('2'),
        None,
        JournalSequenceV1::new(8).unwrap(),
        2,
        1,
        2,
    )
    .unwrap();
    assert_eq!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .read_inbox_page(&undersized)
        .expect_err("row metadata must reject the body before allocation")
        .kind(),
        ControlStoreErrorKind::BoundExceeded
    );
    assert_eq!(
        first
            .entries()
            .iter()
            .map(|entry| entry.cursor().journal_sequence().get())
            .collect::<Vec<_>>(),
        [3, 6]
    );
    assert_eq!(
        first.entries()[0].record().coordinate(),
        replayed_inbox.coordinate()
    );
    let EventInboxPageBoundary::More { resume_after } = first.boundary() else {
        panic!("two-item page must expose continuation");
    };

    let (event_four, event_four_transition) = owner_event(&mut connection, 9, "event-4", '0', '0');
    commit(&mut connection, &event_four_transition);
    let (_, _, delivery_four) = delivery(
        &mut connection,
        Delivery {
            sequence: 10,
            event: &event_four,
            event_key: '0',
            subscription: &subscription,
            delivery_key: '7',
            attempt_key: '5',
            suffix: "four",
            record_byte: '6',
            previous_inbox: None,
        },
    );
    commit(&mut connection, &delivery_four);

    let second_request = EventInboxPageRead::new(
        store(),
        key('2'),
        Some(resume_after.clone()),
        JournalSequenceV1::new(8).unwrap(),
        2,
        1 << 20,
        1 << 20,
    )
    .unwrap();
    let second = SqliteEventCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .read_inbox_page(&second_request)
    .expect("second page");
    assert_eq!(second.entries().len(), 1);
    assert_eq!(second.entries()[0].cursor().journal_sequence().get(), 8);
    assert_eq!(second.boundary(), &EventInboxPageBoundary::Complete);

    let future = EventInboxPageRead::new(
        store(),
        key('2'),
        None,
        JournalSequenceV1::new(11).unwrap(),
        2,
        1 << 20,
        1 << 20,
    )
    .unwrap();
    assert_eq!(
        SqliteEventCustody {
            connection: &mut connection,
            coordinate: store(),
        }
        .read_inbox_page(&future)
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn fixed_frontier_pages_select_the_latest_inbox_version_not_the_current_head() {
    let mut connection = initialize_store();
    let (event_one, event_one_transition) = owner_event(&mut connection, 1, "event-1", '1', '1');
    commit(&mut connection, &event_one_transition);
    let (subscription, subscription_transition) = subscription(&mut connection, 2);
    commit(&mut connection, &subscription_transition);
    let (inbox_one, _, delivery_one) = delivery(
        &mut connection,
        Delivery {
            sequence: 3,
            event: &event_one,
            event_key: '1',
            subscription: &subscription,
            delivery_key: '3',
            attempt_key: '4',
            suffix: "one",
            record_byte: '5',
            previous_inbox: None,
        },
    );
    commit(&mut connection, &delivery_one);
    let (event_two, event_two_transition) = owner_event(&mut connection, 4, "event-2", '7', '7');
    commit(&mut connection, &event_two_transition);
    let (inbox_two, attempt_two, delivery_two) = delivery(
        &mut connection,
        Delivery {
            sequence: 5,
            event: &event_two,
            event_key: '7',
            subscription: &subscription,
            delivery_key: '8',
            attempt_key: '9',
            suffix: "two",
            record_byte: 'a',
            previous_inbox: None,
        },
    );
    commit(&mut connection, &delivery_two);

    let first_request = EventInboxPageRead::new(
        store(),
        key('2'),
        None,
        JournalSequenceV1::new(5).unwrap(),
        1,
        1 << 20,
        1 << 20,
    )
    .unwrap();
    let first = SqliteEventCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .read_inbox_page(&first_request)
    .expect("first fixed-frontier page");
    assert_eq!(first.entries()[0].record(), &inbox_one);
    let EventInboxPageBoundary::More { resume_after } = first.boundary() else {
        panic!("one-item first page must expose continuation");
    };

    let (acknowledged_inbox, acknowledgement) =
        acknowledgement(&mut connection, 6, '8', '9', &inbox_two, &attempt_two);
    commit(&mut connection, &acknowledgement);

    let second_request = EventInboxPageRead::new(
        store(),
        key('2'),
        Some(resume_after.clone()),
        JournalSequenceV1::new(5).unwrap(),
        1,
        1 << 20,
        1 << 20,
    )
    .unwrap();
    let second = SqliteEventCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .read_inbox_page(&second_request)
    .expect("second fixed-frontier page after head advance");
    assert_eq!(second.entries()[0].record(), &inbox_two);
    assert_eq!(second.boundary(), &EventInboxPageBoundary::Complete);

    let advanced_request = EventInboxPageRead::new(
        store(),
        key('2'),
        Some(resume_after.clone()),
        JournalSequenceV1::new(6).unwrap(),
        1,
        1 << 20,
        1 << 20,
    )
    .unwrap();
    let advanced = SqliteEventCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .read_inbox_page(&advanced_request)
    .expect("advanced-frontier page");
    assert_eq!(advanced.entries()[0].record(), &acknowledged_inbox);
}

#[test]
fn typed_index_read_rejects_cross_role_schema_substitution() {
    let mut connection = initialize_store();
    let (_, transition) = owner_event(&mut connection, 1, "event-1", '1', '1');
    commit(&mut connection, &transition);
    let wrong = private_record(
        "subscription-wrong",
        '2',
        EventTransitionMemberKind::Subscription,
    );
    insert_record(&connection, &wrong).unwrap();
    connection
        .execute(
            "UPDATE pale_deployment_event_index SET record_ref = ?1, record_sha256 = ?2 \
             WHERE index_role = 'event_identity' AND index_key = ?3",
            params![
                wrong.coordinate().record_ref().as_str(),
                wrong.coordinate().record_sha256().as_str(),
                key('1').as_str(),
            ],
        )
        .unwrap();
    let mut custody = SqliteEventCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody
            .read_index(
                &store(),
                &EventIndexQuery::EventIdentity(key('1')),
                16 * 1024,
            )
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    connection
        .execute(
            "UPDATE pale_deployment_event_index SET record_ref = ?1, record_sha256 = ?2 \
             WHERE index_role = 'owner_source' AND index_key = ?3",
            params![
                wrong.coordinate().record_ref().as_str(),
                wrong.coordinate().record_sha256().as_str(),
                key('1').as_str(),
            ],
        )
        .unwrap();
    let mut custody = SqliteEventCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody
            .read_index(&store(), &EventIndexQuery::OwnerSource(key('1')), 16 * 1024,)
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}
