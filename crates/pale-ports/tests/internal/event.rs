//! Physical Event/Genealogy custody laws.

use super::*;
use crate::{BoundedAuthorityEntryBytes, ControlRecordBytes};
use pale_core::records::{
    ControlIncarnationIdV1, ControlStoreIdV1, ResidentRecordRefV1, SchemaDigest, SchemaId,
};

fn digest(byte: char) -> ControlRecordDigest {
    ControlRecordDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>())
        .expect("record digest")
}

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record ref"),
        digest(byte),
    )
}

fn record(reference: &str, byte: char, schema: &str) -> ControlRecord {
    ControlRecord::new(
        coordinate(reference, byte),
        SchemaId::parse(schema).expect("schema"),
        SchemaDigest::parse("a".repeat(64)).expect("schema digest"),
        ControlRecordBytes::new(format!(r#"{{"record":"{reference}"}}"#).into_bytes())
            .expect("record bytes"),
    )
}

fn private_record(reference: &str, byte: char, kind: EventTransitionMemberKind) -> ControlRecord {
    record(reference, byte, kind.required_schema_id())
}

fn store(incarnation: &str) -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("control-store").expect("store id"),
        ControlIncarnationIdV1::parse(incarnation).expect("incarnation id"),
    )
}

fn append(store: AuthorityStoreCoordinate) -> PreparedAuthorityAppend {
    PreparedAuthorityAppend::new(
        store,
        None,
        crate::SequencedAuthorityEntry::new(
            JournalSequenceV1::new(1).expect("sequence"),
            BoundedAuthorityEntryBytes::new(vec![b'{', b'}'], 2).expect("entry bytes"),
        ),
    )
    .expect("authority append")
}

fn key(byte: char) -> EventIndexKey {
    EventIndexKey::new(digest(byte))
}

// Controlled physical shape fixture; native tests obtain the actual store head.
fn transaction(store: AuthorityStoreCoordinate) -> crate::ControlTransactionPreparation {
    crate::ControlTransactionPreparation::new(
        store,
        pale_core::records::ControlTransactionSequenceV1::new(1).unwrap(),
    )
    .unwrap()
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    let value = pale_core::canon::parse_strict(bytes).map_err(|error| error.to_string())?;
    T::deserialize(value).map_err(|error| error.to_string())
}

fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug>(
    value: T,
) {
    let encoded = pale_core::canon::to_canonical_string(&value).unwrap();
    assert_eq!(decode::<T>(encoded.as_bytes()).unwrap(), value);
}

#[test]
fn private_journal_value_encoding_preserves_roles_and_validates_digests() {
    for kind in [
        EventCustodyTransitionKind::AdmitOwnerReference,
        EventCustodyTransitionKind::QuarantineIdentityConflict,
        EventCustodyTransitionKind::AdmitSubscription,
        EventCustodyTransitionKind::RetireSubscription,
        EventCustodyTransitionKind::EnqueueDelivery,
        EventCustodyTransitionKind::EnqueueReplay,
        EventCustodyTransitionKind::RecordAcknowledgement,
        EventCustodyTransitionKind::AdvanceDeliveryCursor,
        EventCustodyTransitionKind::AdvanceSourceFrontier,
        EventCustodyTransitionKind::ProjectInteractionOccurrence,
        EventCustodyTransitionKind::ProjectGenealogyEdge,
        EventCustodyTransitionKind::AdvanceReconciliationFrontier,
        EventCustodyTransitionKind::SettlePaleDerivedPublication,
    ] {
        round_trip(kind);
    }
    for member in [
        EventTransitionMemberKind::EventReference,
        EventTransitionMemberKind::EventCommitWitness,
        EventTransitionMemberKind::IdentityConflict,
        EventTransitionMemberKind::Subscription,
        EventTransitionMemberKind::InboxDelivery,
        EventTransitionMemberKind::DeliveryAttempt,
        EventTransitionMemberKind::DeliveryStream,
        EventTransitionMemberKind::EventAcknowledgement,
        EventTransitionMemberKind::DeliveryCursor,
        EventTransitionMemberKind::SourceFrontier,
        EventTransitionMemberKind::InteractionOccurrence,
        EventTransitionMemberKind::GenealogyEdge,
        EventTransitionMemberKind::ReconciliationFrontier,
        EventTransitionMemberKind::PaleDerivedOutbox,
    ] {
        round_trip(member);
    }
    for query in [
        EventIndexQuery::EventIdentity(key('a')),
        EventIndexQuery::OwnerSource(key('a')),
        EventIndexQuery::EventCommitWitness(key('a')),
        EventIndexQuery::IdentityConflict(key('a')),
        EventIndexQuery::Subscription(key('a')),
        EventIndexQuery::InboxDelivery(key('a')),
        EventIndexQuery::DeliveryAttempt(key('a')),
        EventIndexQuery::DeliveryStream(key('a')),
        EventIndexQuery::Acknowledgement(key('a')),
        EventIndexQuery::DeliveryCursor(key('a')),
        EventIndexQuery::SourceFrontier(key('a')),
        EventIndexQuery::InteractionOccurrence(key('a')),
        EventIndexQuery::GenealogyEdge(key('a')),
        EventIndexQuery::ReconciliationFrontier(key('a')),
        EventIndexQuery::PaleDerivedOutbox(key('a')),
    ] {
        round_trip(query);
    }
    round_trip(coordinate("journal-member", 'a'));
    round_trip(digest('a'));
    round_trip(key('a'));
    round_trip(EventCustodyHistoryMember::new(
        EventTransitionMemberKind::EventReference,
        coordinate("encoded-member", 'a'),
    ));
    for invalid in [
        String::new(),
        "A".repeat(64),
        "a".repeat(63),
        "g".repeat(64),
    ] {
        let encoded = format!("\"{invalid}\"");
        assert!(decode::<ControlRecordDigest>(encoded.as_bytes()).is_err());
        assert!(decode::<EventIndexKey>(encoded.as_bytes()).is_err());
        let coordinate = format!(r#"{{"record_ref":"member","record_sha256":{encoded}}}"#);
        assert!(decode::<ControlRecordCoordinate>(coordinate.as_bytes()).is_err());
    }
    assert!(decode::<EventCustodyTransitionKind>(b"\"unknown\"").is_err());
    assert!(decode::<EventTransitionMemberKind>(b"\"unknown\"").is_err());
    assert!(decode::<EventIndexQuery>(b"{\"unknown\":\"a\"}").is_err());
    let extra = format!(
        r#"{{"record_ref":"member","record_sha256":"{}","unexpected":true}}"#,
        "a".repeat(64)
    );
    assert!(decode::<ControlRecordCoordinate>(extra.as_bytes()).is_err());
}

#[test]
fn history_requests_require_exact_new_member_roles_and_combined_byte_limit() {
    let entry = append(store("incarnation-1")).entry().clone();
    let event = EventCustodyHistoryMember::new(
        EventTransitionMemberKind::EventReference,
        coordinate("history-event", '1'),
    );
    let witness = EventCustodyHistoryMember::new(
        EventTransitionMemberKind::EventCommitWitness,
        coordinate("history-event-witness", '2'),
    );
    let build = |members, limit| {
        EventCustodyHistoryRead::new(
            store("incarnation-1"),
            pale_core::records::ControlTransactionSequenceV1::new(2).unwrap(),
            entry.clone(),
            EventCustodyTransitionKind::AdmitOwnerReference,
            members,
            limit,
        )
    };
    assert!(build(vec![event.clone(), witness.clone()], 2).is_ok());
    assert_eq!(
        build(vec![event.clone(), witness.clone()], 1),
        Err(EventCustodyCandidateError::HistoryByteLimit)
    );
    assert_eq!(
        build(Vec::new(), 2),
        Err(EventCustodyCandidateError::MemberShape)
    );
    assert_eq!(
        build(vec![event.clone(), event], 2),
        Err(EventCustodyCandidateError::MemberShape)
    );
    assert_eq!(
        build(
            vec![EventCustodyHistoryMember::new(
                EventTransitionMemberKind::Subscription,
                coordinate("subscription", '2')
            )],
            2
        ),
        Err(EventCustodyCandidateError::MemberShape)
    );
}

#[test]
fn owner_reference_admission_uses_one_authority_append_and_all_event_indexes() {
    let selected_store = store("incarnation-1");
    let event = record("event-1", '1', EVENT_REFERENCE_V1_SCHEMA);
    let witness = private_record(
        "event-commit-witness-1",
        '2',
        EventTransitionMemberKind::EventCommitWitness,
    );
    let transition = EventCustodyTransition::new(
        selected_store.clone(),
        transaction(selected_store.clone()),
        EventCustodyTransitionKind::AdmitOwnerReference,
        append(selected_store),
        vec![
            EventTransitionMember::new(EventTransitionMemberKind::EventReference, event.clone()),
            EventTransitionMember::new(
                EventTransitionMemberKind::EventCommitWitness,
                witness.clone(),
            ),
        ],
        vec![
            EventIndexMutation::new(
                EventIndexQuery::EventIdentity(key('a')),
                EventIndexPredecessor::Absent,
                event.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::OwnerSource(key('b')),
                EventIndexPredecessor::Absent,
                event.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::EventCommitWitness(key('c')),
                EventIndexPredecessor::Absent,
                witness.coordinate().clone(),
            ),
        ],
        Vec::new(),
    )
    .expect("closed owner-reference transition");

    assert_eq!(
        transition.kind().publication_role(),
        Some(EventPublicationRole::OwnerIngress)
    );
    assert_eq!(transition.journal_append().entry().sequence().get(), 1);
    assert_eq!(transition.index_mutations().len(), 3);
}

#[test]
fn owner_reference_requires_absent_identity_source_and_commit_witness_indexes() {
    let selected_store = store("incarnation-1");
    let event = record("event-1", '1', EVENT_REFERENCE_V1_SCHEMA);
    let witness = private_record(
        "event-commit-witness-1",
        '2',
        EventTransitionMemberKind::EventCommitWitness,
    );
    let build = |mutations| {
        EventCustodyTransition::new(
            selected_store.clone(),
            transaction(selected_store.clone()),
            EventCustodyTransitionKind::AdmitOwnerReference,
            append(selected_store.clone()),
            vec![
                EventTransitionMember::new(
                    EventTransitionMemberKind::EventReference,
                    event.clone(),
                ),
                EventTransitionMember::new(
                    EventTransitionMemberKind::EventCommitWitness,
                    witness.clone(),
                ),
            ],
            mutations,
            Vec::new(),
        )
    };
    let identity = EventIndexMutation::new(
        EventIndexQuery::EventIdentity(key('a')),
        EventIndexPredecessor::Absent,
        event.coordinate().clone(),
    );
    let source = EventIndexMutation::new(
        EventIndexQuery::OwnerSource(key('b')),
        EventIndexPredecessor::Absent,
        event.coordinate().clone(),
    );
    let commit_witness = EventIndexMutation::new(
        EventIndexQuery::EventCommitWitness(key('c')),
        EventIndexPredecessor::Absent,
        witness.coordinate().clone(),
    );

    assert_eq!(
        build(vec![identity.clone()]),
        Err(EventCustodyCandidateError::IndexMutationShape)
    );
    assert_eq!(
        build(vec![identity.clone(), identity]),
        Err(EventCustodyCandidateError::DuplicateIndexMutation)
    );
    assert_eq!(
        build(vec![
            source,
            EventIndexMutation::new(
                EventIndexQuery::EventIdentity(key('a')),
                EventIndexPredecessor::Exact(event.clone()),
                event.coordinate().clone(),
            ),
            commit_witness.clone(),
        ]),
        Err(EventCustodyCandidateError::IndexPredecessorShape)
    );
    assert_eq!(
        build(vec![
            EventIndexMutation::new(
                EventIndexQuery::EventIdentity(key('a')),
                EventIndexPredecessor::Absent,
                event.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::OwnerSource(key('b')),
                EventIndexPredecessor::Absent,
                coordinate("another-event", '2'),
            ),
            commit_witness,
        ]),
        Err(EventCustodyCandidateError::IndexSuccessorClosure)
    );
}

#[test]
fn event_transition_rejects_another_journal_or_public_schema() {
    let selected_store = store("incarnation-1");
    let event = record("event-1", '1', EVENT_REFERENCE_V1_SCHEMA);
    let witness = private_record(
        "event-commit-witness-1",
        '2',
        EventTransitionMemberKind::EventCommitWitness,
    );
    let wrong_store = EventCustodyTransition::new(
        selected_store.clone(),
        transaction(selected_store.clone()),
        EventCustodyTransitionKind::AdmitOwnerReference,
        append(store("incarnation-2")),
        vec![
            EventTransitionMember::new(EventTransitionMemberKind::EventReference, event.clone()),
            EventTransitionMember::new(
                EventTransitionMemberKind::EventCommitWitness,
                witness.clone(),
            ),
        ],
        vec![
            EventIndexMutation::new(
                EventIndexQuery::EventIdentity(key('a')),
                EventIndexPredecessor::Absent,
                event.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::OwnerSource(key('b')),
                EventIndexPredecessor::Absent,
                event.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::EventCommitWitness(key('c')),
                EventIndexPredecessor::Absent,
                witness.coordinate().clone(),
            ),
        ],
        Vec::new(),
    );
    assert_eq!(
        wrong_store,
        Err(EventCustodyCandidateError::AuthorityJournal)
    );

    let wrong_schema = private_record("event-1", '1', EventTransitionMemberKind::IdentityConflict);
    let successor = wrong_schema.coordinate().clone();
    let rejected = EventCustodyTransition::new(
        selected_store.clone(),
        transaction(selected_store.clone()),
        EventCustodyTransitionKind::AdmitOwnerReference,
        append(selected_store),
        vec![
            EventTransitionMember::new(EventTransitionMemberKind::EventReference, wrong_schema),
            EventTransitionMember::new(
                EventTransitionMemberKind::EventCommitWitness,
                witness.clone(),
            ),
        ],
        vec![
            EventIndexMutation::new(
                EventIndexQuery::EventIdentity(key('a')),
                EventIndexPredecessor::Absent,
                successor.clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::OwnerSource(key('b')),
                EventIndexPredecessor::Absent,
                successor,
            ),
            EventIndexMutation::new(
                EventIndexQuery::EventCommitWitness(key('c')),
                EventIndexPredecessor::Absent,
                witness.coordinate().clone(),
            ),
        ],
        Vec::new(),
    );
    assert_eq!(rejected, Err(EventCustodyCandidateError::MemberSchema));
}

#[test]
fn transition_rejects_repeating_one_typed_index_mutation() {
    let selected_store = store("incarnation-1");
    let event = record("event-1", '1', EVENT_REFERENCE_V1_SCHEMA);
    let witness = private_record(
        "event-commit-witness-1",
        '2',
        EventTransitionMemberKind::EventCommitWitness,
    );
    let mutation = EventIndexMutation::new(
        EventIndexQuery::EventIdentity(key('a')),
        EventIndexPredecessor::Absent,
        event.coordinate().clone(),
    );
    let rejected = EventCustodyTransition::new(
        selected_store.clone(),
        transaction(selected_store.clone()),
        EventCustodyTransitionKind::AdmitOwnerReference,
        append(selected_store),
        vec![
            EventTransitionMember::new(EventTransitionMemberKind::EventReference, event.clone()),
            EventTransitionMember::new(
                EventTransitionMemberKind::EventCommitWitness,
                witness.clone(),
            ),
        ],
        vec![
            mutation.clone(),
            mutation,
            EventIndexMutation::new(
                EventIndexQuery::OwnerSource(key('b')),
                EventIndexPredecessor::Absent,
                event.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::EventCommitWitness(key('c')),
                EventIndexPredecessor::Absent,
                witness.coordinate().clone(),
            ),
        ],
        Vec::new(),
    );

    assert_eq!(
        rejected,
        Err(EventCustodyCandidateError::DuplicateIndexMutation)
    );
}

#[test]
fn identity_conflict_requires_exact_event_assertion_and_separate_quarantine_index() {
    let selected_store = store("incarnation-1");
    let admitted = record("event-1", '1', EVENT_REFERENCE_V1_SCHEMA);
    let conflict = private_record(
        "event-conflict-1",
        '2',
        EventTransitionMemberKind::IdentityConflict,
    );
    let transition = EventCustodyTransition::new(
        selected_store.clone(),
        transaction(selected_store.clone()),
        EventCustodyTransitionKind::QuarantineIdentityConflict,
        append(selected_store),
        vec![EventTransitionMember::new(
            EventTransitionMemberKind::IdentityConflict,
            conflict.clone(),
        )],
        vec![EventIndexMutation::new(
            EventIndexQuery::IdentityConflict(key('b')),
            EventIndexPredecessor::Absent,
            conflict.coordinate().clone(),
        )],
        vec![EventIndexAssertion::new(
            EventIndexQuery::EventIdentity(key('a')),
            EventIndexPredecessor::Exact(admitted),
        )],
    )
    .expect("identity conflict quarantine");
    assert_eq!(transition.index_assertions().len(), 1);

    let incomplete_conflict = private_record(
        "event-conflict-2",
        '3',
        EventTransitionMemberKind::IdentityConflict,
    );
    let missing_admitted_event = EventCustodyTransition::new(
        store("incarnation-1"),
        transaction(store("incarnation-1")),
        EventCustodyTransitionKind::QuarantineIdentityConflict,
        append(store("incarnation-1")),
        vec![EventTransitionMember::new(
            EventTransitionMemberKind::IdentityConflict,
            incomplete_conflict.clone(),
        )],
        vec![EventIndexMutation::new(
            EventIndexQuery::IdentityConflict(key('c')),
            EventIndexPredecessor::Absent,
            incomplete_conflict.coordinate().clone(),
        )],
        vec![EventIndexAssertion::new(
            EventIndexQuery::EventIdentity(key('a')),
            EventIndexPredecessor::Absent,
        )],
    );
    assert_eq!(
        missing_admitted_event,
        Err(EventCustodyCandidateError::IndexPredecessorShape)
    );
}

#[test]
fn genealogy_edge_requires_two_distinct_occurrence_observations() {
    let selected_store = store("incarnation-1");
    let edge = record("edge-1", '3', GENEALOGY_EDGE_V1_SCHEMA);
    let retention = private_record(
        "edge-outbox",
        '6',
        EventTransitionMemberKind::PaleDerivedOutbox,
    );
    let occurrence_a = record("occurrence-1", '4', INTERACTION_OCCURRENCE_V1_SCHEMA);
    let occurrence_b = record("occurrence-2", '5', INTERACTION_OCCURRENCE_V1_SCHEMA);
    let transition = EventCustodyTransition::new(
        selected_store.clone(),
        transaction(selected_store.clone()),
        EventCustodyTransitionKind::ProjectGenealogyEdge,
        append(selected_store),
        vec![
            EventTransitionMember::new(EventTransitionMemberKind::GenealogyEdge, edge.clone()),
            EventTransitionMember::new(
                EventTransitionMemberKind::PaleDerivedOutbox,
                retention.clone(),
            ),
        ],
        vec![
            EventIndexMutation::new(
                EventIndexQuery::GenealogyEdge(key('c')),
                EventIndexPredecessor::Absent,
                edge.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::PaleDerivedOutbox(key('f')),
                EventIndexPredecessor::Absent,
                retention.coordinate().clone(),
            ),
        ],
        vec![
            EventIndexAssertion::new(
                EventIndexQuery::InteractionOccurrence(key('d')),
                EventIndexPredecessor::Exact(occurrence_a),
            ),
            EventIndexAssertion::new(
                EventIndexQuery::InteractionOccurrence(key('e')),
                EventIndexPredecessor::Exact(occurrence_b),
            ),
        ],
    )
    .expect("truth-classed edge projection");
    assert_eq!(transition.index_assertions().len(), 2);
}

#[test]
fn derived_transition_shapes_require_both_records_and_exact_index_closure() {
    for (kind, family, query, assertions) in [
        (
            EventCustodyTransitionKind::ProjectInteractionOccurrence,
            EventTransitionMemberKind::InteractionOccurrence,
            EventIndexQuery::InteractionOccurrence(key('a')),
            vec![EventIndexAssertion::new(
                EventIndexQuery::EventIdentity(key('c')),
                EventIndexPredecessor::Exact(record("event", '3', EVENT_REFERENCE_V1_SCHEMA)),
            )],
        ),
        (
            EventCustodyTransitionKind::ProjectGenealogyEdge,
            EventTransitionMemberKind::GenealogyEdge,
            EventIndexQuery::GenealogyEdge(key('a')),
            vec![
                EventIndexAssertion::new(
                    EventIndexQuery::InteractionOccurrence(key('c')),
                    EventIndexPredecessor::Exact(record(
                        "source",
                        '3',
                        INTERACTION_OCCURRENCE_V1_SCHEMA,
                    )),
                ),
                EventIndexAssertion::new(
                    EventIndexQuery::InteractionOccurrence(key('d')),
                    EventIndexPredecessor::Exact(record(
                        "destination",
                        '4',
                        INTERACTION_OCCURRENCE_V1_SCHEMA,
                    )),
                ),
            ],
        ),
        (
            EventCustodyTransitionKind::AdvanceReconciliationFrontier,
            EventTransitionMemberKind::ReconciliationFrontier,
            EventIndexQuery::ReconciliationFrontier(key('a')),
            Vec::new(),
        ),
    ] {
        let publication = private_record("publication", '1', family);
        let retention = private_record("outbox", '2', EventTransitionMemberKind::PaleDerivedOutbox);
        let members = vec![
            EventTransitionMember::new(family, publication.clone()),
            EventTransitionMember::new(
                EventTransitionMemberKind::PaleDerivedOutbox,
                retention.clone(),
            ),
        ];
        let mutations = vec![
            EventIndexMutation::new(
                query,
                EventIndexPredecessor::Absent,
                publication.coordinate().clone(),
            ),
            EventIndexMutation::new(
                EventIndexQuery::PaleDerivedOutbox(key('b')),
                EventIndexPredecessor::Absent,
                retention.coordinate().clone(),
            ),
        ];
        let build = |members, mutations| {
            EventCustodyTransition::new(
                store("incarnation-1"),
                transaction(store("incarnation-1")),
                kind,
                append(store("incarnation-1")),
                members,
                mutations,
                assertions.clone(),
            )
        };
        assert!(build(members.clone(), mutations.clone()).is_ok());
        for missing in 0..2 {
            let mut incomplete = members.clone();
            incomplete.remove(missing);
            assert_eq!(
                build(incomplete, mutations.clone()),
                Err(EventCustodyCandidateError::MemberShape)
            );
            let mut incomplete = mutations.clone();
            incomplete.remove(missing);
            assert_eq!(
                build(members.clone(), incomplete),
                Err(EventCustodyCandidateError::IndexMutationShape)
            );
        }
        let mut wrong_schema = members.clone();
        wrong_schema[0] = EventTransitionMember::new(
            family,
            record("publication", '1', EVENT_REFERENCE_V1_SCHEMA),
        );
        assert_eq!(
            build(wrong_schema, mutations.clone()),
            Err(EventCustodyCandidateError::MemberSchema)
        );
        let mut foreign = mutations.clone();
        foreign[1] = EventIndexMutation::new(
            EventIndexQuery::PaleDerivedOutbox(key('b')),
            EventIndexPredecessor::Absent,
            coordinate("foreign-outbox", '2'),
        );
        assert_eq!(
            build(members.clone(), foreign),
            Err(EventCustodyCandidateError::IndexSuccessorClosure)
        );
        let mut existing = mutations.clone();
        existing[1] = EventIndexMutation::new(
            EventIndexQuery::PaleDerivedOutbox(key('b')),
            EventIndexPredecessor::Exact(private_record(
                "old-outbox",
                '5',
                EventTransitionMemberKind::PaleDerivedOutbox,
            )),
            retention.coordinate().clone(),
        );
        assert_eq!(
            build(members, existing),
            Err(EventCustodyCandidateError::IndexPredecessorShape)
        );
    }
}

#[test]
fn typed_point_observations_reject_schema_and_coordinate_drift() {
    let event_query = EventIndexQuery::EventIdentity(key('a'));
    assert_eq!(
        EventIndexObservation::exact_record(
            event_query,
            private_record(
                "not-an-event",
                '1',
                EventTransitionMemberKind::IdentityConflict,
            )
        ),
        Err(EventObservationError::RecordSchema)
    );
    assert_eq!(
        EventIndexObservation::exact_record(
            EventIndexQuery::OwnerSource(key('b')),
            private_record(
                "not-an-owner-event",
                '2',
                EventTransitionMemberKind::IdentityConflict,
            ),
        ),
        Err(EventObservationError::RecordSchema)
    );

    let expected = coordinate("event-1", '1');
    assert_eq!(
        EventRecordObservation::exact(expected, record("event-2", '2', EVENT_REFERENCE_V1_SCHEMA)),
        Err(EventObservationError::RecordCoordinate)
    );

    let subscription_query = EventIndexQuery::Subscription(key('b'));
    assert_eq!(
        EventIndexObservation::exact_record(
            subscription_query,
            private_record(
                "delivery-attempt-1",
                '3',
                EventTransitionMemberKind::DeliveryAttempt,
            ),
        ),
        Err(EventObservationError::RecordSchema)
    );
}

#[test]
fn inbox_pages_are_bounded_and_continue_from_the_exact_last_cursor() {
    let request = EventInboxPageRead::new(
        store("incarnation-1"),
        key('a'),
        None,
        JournalSequenceV1::new(5).expect("frontier"),
        2,
        1 << 20,
        1 << 20,
    )
    .expect("page request");
    let first_cursor =
        EventInboxCursor::new(JournalSequenceV1::new(2).expect("sequence"), key('b'));
    let second_cursor =
        EventInboxCursor::new(JournalSequenceV1::new(4).expect("sequence"), key('c'));
    let page = EventInboxPage::new(
        &request,
        vec![
            EventInboxPageEntry::new(
                first_cursor,
                private_record(
                    "inbox-delivery-1",
                    '6',
                    EventTransitionMemberKind::InboxDelivery,
                ),
            ),
            EventInboxPageEntry::new(
                second_cursor.clone(),
                private_record(
                    "inbox-delivery-2",
                    '7',
                    EventTransitionMemberKind::InboxDelivery,
                ),
            ),
        ],
        EventInboxPageBoundary::More {
            resume_after: second_cursor,
        },
    )
    .expect("bounded page");
    assert_eq!(page.entries().len(), 2);

    let wrong_resume = EventInboxPage::new(
        &request,
        vec![EventInboxPageEntry::new(
            EventInboxCursor::new(JournalSequenceV1::new(2).expect("sequence"), key('b')),
            private_record(
                "inbox-delivery-1",
                '6',
                EventTransitionMemberKind::InboxDelivery,
            ),
        )],
        EventInboxPageBoundary::More {
            resume_after: EventInboxCursor::new(
                JournalSequenceV1::new(3).expect("sequence"),
                key('d'),
            ),
        },
    );
    assert_eq!(wrong_resume, Err(EventPageError::Continuation));
}

#[test]
fn derived_outbox_pages_are_bounded_typed_and_strictly_continuable() {
    assert_eq!(
        PaleDerivedOutboxPageRead::new(store("incarnation-1"), None, 0),
        Err(PaleDerivedOutboxPageError::ZeroBound)
    );
    let request = PaleDerivedOutboxPageRead::new(store("incarnation-1"), None, 2)
        .expect("derived outbox page request");
    let first = PaleDerivedOutboxCursor::new(key('1'));
    let second = PaleDerivedOutboxCursor::new(key('2'));
    let page = PaleDerivedOutboxPage::new(
        &request,
        vec![
            PaleDerivedOutboxPageEntry::new(
                first,
                private_record(
                    "derived-retention-1",
                    '3',
                    EventTransitionMemberKind::PaleDerivedOutbox,
                ),
            ),
            PaleDerivedOutboxPageEntry::new(
                second.clone(),
                private_record(
                    "derived-retention-2",
                    '4',
                    EventTransitionMemberKind::PaleDerivedOutbox,
                ),
            ),
        ],
        PaleDerivedOutboxPageBoundary::More {
            resume_after: second,
        },
    )
    .expect("bounded derived outbox page");
    assert_eq!(page.entries().len(), 2);

    let wrong_schema = PaleDerivedOutboxPage::new(
        &request,
        vec![PaleDerivedOutboxPageEntry::new(
            PaleDerivedOutboxCursor::new(key('1')),
            private_record(
                "not-derived-retention",
                '5',
                EventTransitionMemberKind::InboxDelivery,
            ),
        )],
        PaleDerivedOutboxPageBoundary::Complete,
    );
    assert_eq!(wrong_schema, Err(PaleDerivedOutboxPageError::RecordSchema));
}
