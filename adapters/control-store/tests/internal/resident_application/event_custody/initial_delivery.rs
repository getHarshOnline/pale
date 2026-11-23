//! Restart and replay laws for deterministic private initial delivery custody.

use super::local_operation::writer;
use super::*;
use crate::resident_fixture::producer_custody_fixture::InitialProducerFixture;
use pale_application::resident::*;
use pale_core::records::*;

fn rewrite_record_body(
    store: &DeploymentControlSqliteStore,
    schema_id: &str,
    rewrite: impl FnOnce(&mut serde_json::Value),
) {
    let connection = store.connection().unwrap();
    let (record_ref, body): (String, Vec<u8>) = connection
        .query_row(
            "SELECT record_ref, canonical_record_jcs \
               FROM pale_deployment_control_record \
              WHERE schema_id=?1 ORDER BY inserted_transaction_sequence DESC LIMIT 1",
            [schema_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    rewrite(&mut value);
    let canonical = pale_core::canon::to_canonical_string(&value).unwrap();
    let digest = ArtifactDigest::hash_bytes(canonical.as_bytes());
    connection
        .execute_batch("PRAGMA defer_foreign_keys=ON; BEGIN IMMEDIATE;")
        .unwrap();
    connection
        .execute(
            "UPDATE pale_deployment_control_record \
                SET record_sha256=?1, canonical_record_jcs=?2 WHERE record_ref=?3",
            rusqlite::params![digest.as_str(), canonical.as_bytes(), record_ref],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE pale_deployment_event_index SET record_sha256=?1 WHERE record_ref=?2",
            rusqlite::params![digest.as_str(), record_ref],
        )
        .unwrap();
    connection.execute_batch("COMMIT;").unwrap();
}

fn replace_stream_head_with_equal_ordinal_other_attempt(store: &DeploymentControlSqliteStore) {
    let connection = store.connection().unwrap();
    let (old_ref, schema_id, schema_sha256, inserted, body): (
        String,
        String,
        String,
        i64,
        Vec<u8>,
    ) = connection
        .query_row(
            "SELECT record_ref, schema_id, schema_sha256, inserted_transaction_sequence, \
                    canonical_record_jcs \
               FROM pale_deployment_control_record \
              WHERE schema_id='pale/internal-event-delivery-stream.v1' \
              ORDER BY inserted_transaction_sequence DESC LIMIT 1",
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
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let other_attempt = "a".repeat(64);
    value["last_attempt_id"] = serde_json::Value::String(other_attempt.clone());
    let canonical = pale_core::canon::to_canonical_string(&value).unwrap();
    let identity = serde_json::json!({
        "attempt_id": other_attempt,
        "initial_v1_domain": "event_delivery_stream_successor",
        "ordinal": value["through_ordinal"].clone(),
        "subscription_id": value["subscription_id"].clone(),
    });
    let identity = pale_core::canon::to_canonical_string(&identity).unwrap();
    let mut preimage = b"PALE_EVENT_DELIVERY_STREAM_SUCCESSOR_V1\0".to_vec();
    preimage.extend_from_slice(identity.as_bytes());
    let new_ref = format!("event-delivery-stream-{}", pale_core::sha256_hex(&preimage));
    let digest = ArtifactDigest::hash_bytes(canonical.as_bytes());
    connection
        .execute(
            "INSERT INTO pale_deployment_control_record \
                (record_ref, inserted_transaction_sequence, record_sha256, schema_id, \
                 schema_sha256, canonical_record_jcs) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                new_ref,
                inserted,
                digest.as_str(),
                schema_id,
                schema_sha256,
                canonical.as_bytes()
            ],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE pale_deployment_event_index SET record_ref=?1, record_sha256=?2 \
              WHERE index_role='delivery_stream' AND record_ref=?3",
            rusqlite::params![new_ref, digest.as_str(), old_ref],
        )
        .unwrap();
}

fn event(suffix: &str) -> EventReferenceV1 {
    let schema = pale_contracts::record_schema_identity(EVENT_REFERENCE_V1_SCHEMA_ID).unwrap();
    let expected_schema_sha256 = schema.schema_sha256.clone();
    let preimage = EventReferencePreimageV1 {
        schema_id: schema.schema,
        schema_sha256: schema.schema_sha256,
        event_id: EventIdV1::parse(format!("initial-delivery-event-{suffix}")).unwrap(),
        event_type: EventTypeV1::parse("owner.record.committed").unwrap(),
        event_version: 1,
        lifecycle_namespace: LifecycleNamespaceV1::parse("owner.record").unwrap(),
        origin_principal_id: ResidentPrincipalIdV1::parse("principal-1").unwrap(),
        origin_node_id: ResidentNodeIdV1::parse("publication-node").unwrap(),
        source_owner: SourceOwnerV1::parse("native-owner").unwrap(),
        source_record_ref: ResidentRecordRefV1::parse(format!(
            "initial-delivery-owner-record-{suffix}"
        ))
        .unwrap(),
        source_record_schema_id: SchemaId::parse("owner/record.v1").unwrap(),
        source_record_schema_sha256: SchemaDigest::parse("1".repeat(64)).unwrap(),
        source_record_content_sha256: ArtifactDigest::parse("2".repeat(64)).unwrap(),
        source_version: SourceVersionV1::parse("owner-revision-1").unwrap(),
        target_scope: EventScopeV1 {
            principal_scope: ResidentPrincipalIdV1::parse("principal-1").unwrap(),
            controller_scope: ResidentControllerIdV1::parse("controller-1").unwrap(),
            relationship_scope: None,
            environment: DeploymentEnvironmentV1::parse("test").unwrap(),
        },
        payload_ref: None,
        payload_sha256: None,
        classification: DataClassificationV1::parse("confidential").unwrap(),
        ordering_key: None,
        sequence: None,
        occurred_at: UnixTimeV1::new(51).unwrap(),
        recorded_at: UnixTimeV1::new(52).unwrap(),
        correlation_id: EventCorrelationIdV1::parse(format!(
            "initial-delivery-correlation-{suffix}"
        ))
        .unwrap(),
        causation_id: None,
        producer_epoch: ProducerEpochV1::parse("a".repeat(32)).unwrap(),
        expires_at: None,
    };
    EventReferenceV1::issue_contextual(preimage, &expected_schema_sha256, |_| Ok(())).unwrap()
}

fn acknowledgement_wire(
    event: &EventReferenceV1,
    plan: &InitialEventDeliveryPlan,
) -> (
    EventAcknowledgementWireV1,
    ResidentNodeIdV1,
    PrincipalKeyEpochV1,
    ResidentRecordRefV1,
    ArtifactDigest,
) {
    let schema =
        pale_contracts::record_schema_identity(EVENT_ACKNOWLEDGEMENT_V1_SCHEMA_ID).unwrap();
    let node = ResidentNodeIdV1::parse("destination-node").unwrap();
    let key_epoch = PrincipalKeyEpochV1::new(1).unwrap();
    let delegation_ref = ResidentRecordRefV1::parse("destination-node-delegation").unwrap();
    let delegation_sha256 = ArtifactDigest::parse("8".repeat(64)).unwrap();
    let preimage = EventAcknowledgementPreimageV1 {
        schema_id: schema.schema,
        schema_sha256: schema.schema_sha256,
        canonical_event_identity: CanonicalEventIdentityV1 {
            origin_principal_id: event.origin_principal_id().clone(),
            producer_epoch: event.producer_epoch().clone(),
            event_id: event.event_id().clone(),
        },
        canonical_event_sha256: event.canonical_event_sha256().clone(),
        delivery_attempt_id: plan.attempt_id().clone(),
        subscription_id: EventSubscriptionIdV1::parse("resident-subscription").unwrap(),
        destination_principal_id: ResidentPrincipalIdV1::parse("principal-1").unwrap(),
        destination_node_id: None,
        disposition: EventAcknowledgementDispositionV1::Accepted,
        reason_code: None,
        consumer_state_version: Some(ConsumerStateVersionV1::parse("consumer-state-1").unwrap()),
        duplicate_of_ack_ref: None,
        duplicate_of_ack_sha256: None,
        quarantine_ref: None,
        quarantine_sha256: None,
        acknowledged_at: UnixTimeV1::new(60).unwrap(),
        acknowledging_principal_id: ResidentPrincipalIdV1::parse("principal-1").unwrap(),
        acknowledging_node_id: node.clone(),
        acknowledging_principal_key_epoch: key_epoch,
        acknowledging_node_delegation_ref: delegation_ref.clone(),
        acknowledging_node_delegation_sha256: delegation_sha256.clone(),
    };
    let digest = EventAcknowledgementDigest::hash_canonical(&preimage).unwrap();
    let mut wire = serde_json::to_value(&preimage).unwrap();
    wire["canonical_acknowledgement_sha256"] = serde_json::json!(digest.as_str());
    wire["signature"] = serde_json::json!("A".repeat(86));
    (
        serde_json::from_value(wire).unwrap(),
        node,
        key_epoch,
        delegation_ref,
        delegation_sha256,
    )
}

#[test]
fn initial_delivery_recovers_after_restart_and_replay_mints_no_continuation() {
    let directory = support::private_tempdir();
    let (location, identity, _) = support::initialize(
        &directory,
        ControlStoreKind::DeploymentControl,
        "resident-initial-event-delivery",
    );
    let coordinate = AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    );
    let resolver = RecoveryResolver::default();
    let active = subscription(
        "initial-delivery-subscription",
        SubscriptionLifecycle::Active,
    );
    let admitted_event = event("first");
    let event_ref = ResidentRecordRefV1::parse("initial-delivery-event-record").unwrap();
    let producer = InitialProducerFixture::new().0;
    let current_writer = writer();
    {
        let store =
            DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
                .unwrap();
        let mut source = FixedInputSource::default();
        let mut resident = open_resident(&store, coordinate.clone(), &resolver, &mut source, 1);
        commit_subscription(
            &mut resident,
            &store,
            &coordinate,
            EventCustodyTransitionInput::AdmitSubscription(&active),
        );
        let verifier = |_: &EventPublicationRaceClosure<'_>| Ok(());
        let writer_ref = ResidentRecordRefV1::parse("initial-delivery-writer-record").unwrap();
        let owner = AdmittedOwnerPublication::new(
            AdmittedEventRecord::new(&event_ref, &admitted_event),
            &producer.allocation_ref,
            &producer.allocation,
            &writer_ref,
            &current_writer,
            EventPublicationState::Active,
            &verifier,
        );
        let transaction = (&store).prepare_transaction(&coordinate).unwrap();
        let (mut completed, receipt) = resident
            .commit_event_custody(
                &mut &store,
                transaction,
                EventCustodyTransitionInput::AdmitOwnerReference(owner),
            )
            .unwrap();
        assert_eq!(
            completed.take_continuation().map(|value| value.kind()),
            Some(EventCustodyContinuationKind::ProjectOwnerReference)
        );
        assert_eq!(receipt.journal_sequence().get(), 2);
    }

    let first_event = admitted_event.clone();
    let plan = InitialEventDeliveryPlan::admit(event_ref, admitted_event, active.clone()).unwrap();
    let second_event = event("second");
    let second_ref = ResidentRecordRefV1::parse("initial-delivery-event-record-second").unwrap();
    let second_plan =
        InitialEventDeliveryPlan::admit(second_ref.clone(), second_event.clone(), active.clone())
            .unwrap();
    let limits = InitialEventDeliveryRecoveryLimits::new(8, 8, 1 << 20, 1 << 22).unwrap();
    let commit_limits = InitialEventDeliveryRecoveryLimits::new(8, 8, 1 << 20, 1).unwrap();

    {
        let store =
            DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
                .unwrap();
        let mut source = FixedInputSource::default();
        let mut resident = open_resident(&store, coordinate.clone(), &resolver, &mut source, 2);
        let result =
            advance_initial_event_delivery(&mut resident, &mut &store, &plan, commit_limits)
                .expect("retain one initial attempt");
        match result {
            InitialEventDeliveryAdvance::Established {
                ordinal,
                evidence,
                continuation,
                receipt,
            } => {
                assert_eq!(evidence, EventCustodyCommitEvidence::New);
                assert_eq!(ordinal.get(), 1);
                assert_eq!(
                    continuation.kind(),
                    EventCustodyContinuationKind::DeliverInboxAttempt
                );
                assert_eq!(receipt.journal_sequence().get(), 3);
            }
            InitialEventDeliveryAdvance::Existing { .. } => {
                panic!("first enqueue must establish custody")
            }
        }

        let verifier = |_: &EventPublicationRaceClosure<'_>| Ok(());
        let writer_ref = ResidentRecordRefV1::parse("initial-delivery-writer-record").unwrap();
        let owner = AdmittedOwnerPublication::new(
            AdmittedEventRecord::new(&second_ref, &second_event),
            &producer.allocation_ref,
            &producer.allocation,
            &writer_ref,
            &current_writer,
            EventPublicationState::Active,
            &verifier,
        );
        let transaction = (&store).prepare_transaction(&coordinate).unwrap();
        let (mut completed, receipt) = resident
            .commit_event_custody(
                &mut &store,
                transaction,
                EventCustodyTransitionInput::AdmitOwnerReference(owner),
            )
            .unwrap();
        assert_eq!(
            completed.take_continuation().map(|value| value.kind()),
            Some(EventCustodyContinuationKind::ProjectOwnerReference)
        );
        assert_eq!(receipt.journal_sequence().get(), 4);
        let second =
            advance_initial_event_delivery(&mut resident, &mut &store, &second_plan, limits)
                .expect("allocate the next subscription-stream ordinal");
        assert!(matches!(
            second,
            InitialEventDeliveryAdvance::Established { ordinal, .. } if ordinal.get() == 2
        ));
        assert_eq!(resident.journal_head().unwrap().get(), 5);

        let (acknowledgement, node, key_epoch, delegation_ref, delegation_sha256) =
            acknowledgement_wire(&second_event, &second_plan);
        let writes_before_rejection = store.connection().unwrap().total_changes();
        let rejected = record_initial_event_acknowledgement(
            &mut resident,
            &mut &store,
            &second_plan,
            EventDeliveryOrdinal::new(2).unwrap(),
            InitialEventAcknowledgementInput {
                acknowledgement_ref: ResidentRecordRefV1::parse(
                    "initial-delivery-acknowledgement-second",
                )
                .unwrap(),
                delivery_state_ref: ResidentRecordRefV1::parse(
                    "initial-delivery-acknowledged-second",
                )
                .unwrap(),
                acknowledgement: acknowledgement.clone(),
            },
            InitialEventAcknowledgementAuthority {
                acknowledging_node_id: &ResidentNodeIdV1::parse("substituted-node").unwrap(),
                principal_key_epoch: key_epoch,
                node_delegation_ref: &delegation_ref,
                node_delegation_sha256: &delegation_sha256,
            },
            1 << 20,
            |_, _, _, _| Ok(()),
        );
        assert!(matches!(
            rejected,
            Err(InitialEventDeliveryError::Acknowledgement(
                InvalidResidentRecordV1::ContextualAdmission { .. }
            ))
        ));
        assert_eq!(resident.journal_head().unwrap().get(), 5);
        assert_eq!(
            store.connection().unwrap().total_changes(),
            writes_before_rejection
        );
        let acknowledged = record_initial_event_acknowledgement(
            &mut resident,
            &mut &store,
            &second_plan,
            EventDeliveryOrdinal::new(2).unwrap(),
            InitialEventAcknowledgementInput {
                acknowledgement_ref: ResidentRecordRefV1::parse(
                    "initial-delivery-acknowledgement-second",
                )
                .unwrap(),
                delivery_state_ref: ResidentRecordRefV1::parse(
                    "initial-delivery-acknowledged-second",
                )
                .unwrap(),
                acknowledgement,
            },
            InitialEventAcknowledgementAuthority {
                acknowledging_node_id: &node,
                principal_key_epoch: key_epoch,
                node_delegation_ref: &delegation_ref,
                node_delegation_sha256: &delegation_sha256,
            },
            1 << 20,
            |_, _, _, _| Ok(()),
        )
        .unwrap();
        let second_recorded = match acknowledged {
            InitialEventAcknowledgementAdvance::Established {
                recorded,
                evidence: EventCustodyCommitEvidence::New,
                ..
            } => recorded,
            _ => panic!("first acknowledgement must establish exact custody"),
        };
        assert_eq!(resident.journal_head().unwrap().get(), 6);
        assert!(matches!(
            advance_initial_event_delivery_cursor(
                &mut resident,
                &mut &store,
                &second_recorded,
                1 << 20,
            )
            .unwrap(),
            InitialEventDeliveryCursorAdvance::WaitingForEarlier {
                current_through: None,
                acknowledged_ordinal,
            } if acknowledged_ordinal.get() == 2
        ));
        assert_eq!(resident.journal_head().unwrap().get(), 6);
    }

    {
        let store =
            DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
                .unwrap();
        let mut source = FixedInputSource::default();
        let mut resident = open_resident(&store, coordinate.clone(), &resolver, &mut source, 3);
        let retired = subscription(
            "initial-delivery-subscription-retired",
            SubscriptionLifecycle::Retired {
                retired_at: UnixTimeV1::new(60).unwrap(),
            },
        );
        commit_subscription(
            &mut resident,
            &store,
            &coordinate,
            EventCustodyTransitionInput::RetireSubscription {
                predecessor: &active,
                successor: &retired,
            },
        );
        assert_eq!(resident.journal_head().unwrap().get(), 7);
    }

    let store =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
            .unwrap();
    let before_changes = store.connection().unwrap().total_changes();
    let before_records: i64 = store
        .connection()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM pale_deployment_control_record",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut source = FixedInputSource::default();
    let mut resident = open_resident(&store, coordinate.clone(), &resolver, &mut source, 4);
    assert_eq!(
        recover_initial_event_delivery(&resident, &mut &store, &plan, limits,).unwrap(),
        InitialEventDeliveryRecovery::Pending {
            ordinal: EventDeliveryOrdinal::new(1).unwrap()
        }
    );
    let alias_plan = InitialEventDeliveryPlan::admit(
        ResidentRecordRefV1::parse("initial-delivery-event-record-alias").unwrap(),
        event("first"),
        active,
    )
    .unwrap();
    assert!(matches!(
        recover_initial_event_delivery(&resident, &mut &store, &alias_plan, limits,),
        Err(InitialEventDeliveryError::RetainedStateConflict)
    ));
    assert!(matches!(
        advance_initial_event_delivery(&mut resident, &mut &store, &plan, limits).unwrap(),
        InitialEventDeliveryAdvance::Existing { ordinal } if ordinal.get() == 1
    ));
    assert_eq!(resident.journal_head().unwrap().get(), 7);
    assert_eq!(store.connection().unwrap().total_changes(), before_changes);
    let after_records: i64 = store
        .connection()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM pale_deployment_control_record",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(after_records, before_records);

    let (acknowledgement, node, key_epoch, delegation_ref, delegation_sha256) =
        acknowledgement_wire(&first_event, &plan);
    let first_acknowledgement = record_initial_event_acknowledgement(
        &mut resident,
        &mut &store,
        &plan,
        EventDeliveryOrdinal::new(1).unwrap(),
        InitialEventAcknowledgementInput {
            acknowledgement_ref: ResidentRecordRefV1::parse(
                "initial-delivery-acknowledgement-first",
            )
            .unwrap(),
            delivery_state_ref: ResidentRecordRefV1::parse("initial-delivery-acknowledged-first")
                .unwrap(),
            acknowledgement,
        },
        InitialEventAcknowledgementAuthority {
            acknowledging_node_id: &node,
            principal_key_epoch: key_epoch,
            node_delegation_ref: &delegation_ref,
            node_delegation_sha256: &delegation_sha256,
        },
        1 << 20,
        |_, _, _, _| Ok(()),
    )
    .unwrap();
    let first_recorded = match first_acknowledgement {
        InitialEventAcknowledgementAdvance::Established {
            recorded,
            evidence: EventCustodyCommitEvidence::New,
            ..
        } => recorded,
        _ => panic!("first ordinal acknowledgement must establish exact custody"),
    };
    assert_eq!(resident.journal_head().unwrap().get(), 8);
    assert!(matches!(
        advance_initial_event_delivery_cursor(
            &mut resident,
            &mut &store,
            &first_recorded,
            1 << 20,
        )
        .unwrap(),
        InitialEventDeliveryCursorAdvance::Established {
            cursor,
            evidence: EventCustodyCommitEvidence::New,
            ..
        } if cursor.through_ordinal().get() == 1
    ));
    assert_eq!(resident.journal_head().unwrap().get(), 9);

    let (acknowledgement, node, key_epoch, delegation_ref, delegation_sha256) =
        acknowledgement_wire(&second_event, &second_plan);
    let before_replay = store.connection().unwrap().total_changes();
    let replay = record_initial_event_acknowledgement(
        &mut resident,
        &mut &store,
        &second_plan,
        EventDeliveryOrdinal::new(2).unwrap(),
        InitialEventAcknowledgementInput {
            acknowledgement_ref: ResidentRecordRefV1::parse(
                "initial-delivery-acknowledgement-second",
            )
            .unwrap(),
            delivery_state_ref: ResidentRecordRefV1::parse("initial-delivery-acknowledged-second")
                .unwrap(),
            acknowledgement,
        },
        InitialEventAcknowledgementAuthority {
            acknowledging_node_id: &node,
            principal_key_epoch: key_epoch,
            node_delegation_ref: &delegation_ref,
            node_delegation_sha256: &delegation_sha256,
        },
        1 << 20,
        |_, _, _, _| Ok(()),
    )
    .unwrap();
    assert_eq!(store.connection().unwrap().total_changes(), before_replay);
    let replay_recorded = match replay {
        InitialEventAcknowledgementAdvance::Existing(recorded) => recorded,
        InitialEventAcknowledgementAdvance::Established { .. } => {
            panic!("replayed acknowledgement must not append")
        }
    };
    assert!(matches!(
        advance_initial_event_delivery_cursor(
            &mut resident,
            &mut &store,
            &replay_recorded,
            1 << 20,
        )
        .unwrap(),
        InitialEventDeliveryCursorAdvance::Established {
            cursor,
            evidence: EventCustodyCommitEvidence::New,
            ..
        }
            if cursor.through_ordinal().get() == 2
    ));
    assert_eq!(resident.journal_head().unwrap().get(), 10);
    let changes_after_cursor = store.connection().unwrap().total_changes();
    assert!(matches!(
        advance_initial_event_delivery_cursor(
            &mut resident,
            &mut &store,
            &replay_recorded,
            1 << 20,
        )
        .unwrap(),
        InitialEventDeliveryCursorAdvance::Existing(cursor)
            if cursor.through_ordinal().get() == 2
    ));
    assert_eq!(resident.journal_head().unwrap().get(), 10);
    assert_eq!(
        store.connection().unwrap().total_changes(),
        changes_after_cursor
    );
    assert!(matches!(
        advance_initial_event_delivery_cursor(
            &mut resident,
            &mut &store,
            &first_recorded,
            1 << 20,
        )
        .unwrap(),
        InitialEventDeliveryCursorAdvance::AlreadyBeyond {
            cursor,
            acknowledged_ordinal,
        } if cursor.through_ordinal().get() == 2 && acknowledged_ordinal.get() == 1
    ));
    assert_eq!(resident.journal_head().unwrap().get(), 10);
    assert_eq!(
        store.connection().unwrap().total_changes(),
        changes_after_cursor
    );
    store
        .connection()
        .unwrap()
        .execute_batch(
            "INSERT INTO pale_deployment_control_record \
         SELECT 'event-delivery-cursor-alias', inserted_transaction_sequence, \
                record_sha256, schema_id, schema_sha256, canonical_record_jcs \
           FROM pale_deployment_control_record \
          WHERE schema_id='pale/internal-event-delivery-cursor.v1' \
          ORDER BY inserted_transaction_sequence DESC LIMIT 1; \
         UPDATE pale_deployment_event_index SET record_ref='event-delivery-cursor-alias' \
          WHERE index_role='delivery_cursor';",
        )
        .unwrap();
    let writes_after_corruption = store.connection().unwrap().total_changes();
    assert!(matches!(
        advance_initial_event_delivery_cursor(&mut resident, &mut &store, &first_recorded, 1 << 20,),
        Err(InitialEventDeliveryError::RetainedStateConflict)
    ));
    assert_eq!(resident.journal_head().unwrap().get(), 10);
    assert_eq!(
        store.connection().unwrap().total_changes(),
        writes_after_corruption
    );
    store
        .connection()
        .unwrap()
        .execute(
            "DELETE FROM pale_deployment_event_index \
              WHERE index_role='transition_witness' AND record_ref IN ( \
                    SELECT record_ref FROM pale_deployment_control_record \
                     WHERE schema_id='pale/internal-event-transition-witness.v1' \
                       AND CAST(canonical_record_jcs AS TEXT) \
                           LIKE '%\"transition_kind\":\"record_acknowledgement\"%' \
              )",
            [],
        )
        .unwrap();
    let (acknowledgement, node, key_epoch, delegation_ref, delegation_sha256) =
        acknowledgement_wire(&first_event, &plan);
    let writes_after_witness_removal = store.connection().unwrap().total_changes();
    assert!(matches!(
        record_initial_event_acknowledgement(
            &mut resident,
            &mut &store,
            &plan,
            EventDeliveryOrdinal::new(1).unwrap(),
            InitialEventAcknowledgementInput {
                acknowledgement_ref: ResidentRecordRefV1::parse(
                    "initial-delivery-acknowledgement-first",
                )
                .unwrap(),
                delivery_state_ref: ResidentRecordRefV1::parse(
                    "initial-delivery-acknowledged-first",
                )
                .unwrap(),
                acknowledgement,
            },
            InitialEventAcknowledgementAuthority {
                acknowledging_node_id: &node,
                principal_key_epoch: key_epoch,
                node_delegation_ref: &delegation_ref,
                node_delegation_sha256: &delegation_sha256,
            },
            1 << 20,
            |_, _, _, _| Ok(()),
        ),
        Err(InitialEventDeliveryError::RetainedStateConflict)
    ));
    assert_eq!(resident.journal_head().unwrap().get(), 10);
    assert_eq!(
        store.connection().unwrap().total_changes(),
        writes_after_witness_removal
    );
}

#[test]
fn initial_delivery_rejects_corrupt_or_partial_witness_and_stream_custody() {
    #[derive(Debug)]
    enum Fault<'a> {
        Sql(&'a str),
        CoherentWitnessRewrite,
        EqualOrdinalOtherAttempt,
    }

    let faults = [
        Fault::Sql(
            "INSERT INTO pale_deployment_control_record \
         SELECT 'event-witness-alias', inserted_transaction_sequence, record_sha256, schema_id, \
                schema_sha256, canonical_record_jcs \
           FROM pale_deployment_control_record \
          WHERE schema_id='pale/internal-event-commit-witness.v1' \
          ORDER BY inserted_transaction_sequence DESC LIMIT 1; \
         UPDATE pale_deployment_event_index SET record_ref='event-witness-alias' \
          WHERE index_role='event_commit_witness' AND record_ref=( \
              SELECT record_ref FROM pale_deployment_control_record \
               WHERE schema_id='pale/internal-event-commit-witness.v1' \
                 AND record_ref<>'event-witness-alias' \
               ORDER BY inserted_transaction_sequence DESC LIMIT 1);",
        ),
        Fault::Sql(
            "UPDATE pale_deployment_control_record SET canonical_record_jcs=x'7b7d' \
          WHERE record_ref=(SELECT record_ref FROM pale_deployment_event_index \
                             WHERE index_role='event_commit_witness' \
                             ORDER BY rowid DESC LIMIT 1);",
        ),
        Fault::Sql(
            "INSERT INTO pale_deployment_control_record \
         SELECT 'event-stream-alias', inserted_transaction_sequence, record_sha256, schema_id, \
                schema_sha256, canonical_record_jcs \
           FROM pale_deployment_control_record \
          WHERE schema_id='pale/internal-event-delivery-stream.v1' \
          ORDER BY inserted_transaction_sequence DESC LIMIT 1; \
         UPDATE pale_deployment_event_index SET record_ref='event-stream-alias' \
          WHERE index_role='delivery_stream';",
        ),
        Fault::Sql(
            "UPDATE pale_deployment_control_record SET canonical_record_jcs=x'7b7d' \
          WHERE record_ref=(SELECT record_ref FROM pale_deployment_event_index \
                             WHERE index_role='delivery_stream');",
        ),
        Fault::Sql("DELETE FROM pale_deployment_event_index WHERE index_role='delivery_stream';"),
        Fault::Sql(
            "UPDATE pale_deployment_event_index \
            SET record_ref=(SELECT record_ref FROM pale_deployment_control_record \
                             WHERE schema_id='pale/internal-event-delivery-stream.v1' \
                             ORDER BY inserted_transaction_sequence ASC LIMIT 1), \
                record_sha256=(SELECT record_sha256 FROM pale_deployment_control_record \
                                WHERE schema_id='pale/internal-event-delivery-stream.v1' \
                                ORDER BY inserted_transaction_sequence ASC LIMIT 1) \
          WHERE index_role='delivery_stream';",
        ),
        Fault::Sql(
            "DELETE FROM pale_deployment_event_index \
              WHERE index_role='transition_witness' \
                AND rowid=(SELECT MAX(rowid) FROM pale_deployment_event_index \
                            WHERE index_role='transition_witness');",
        ),
        Fault::Sql(
            "INSERT INTO pale_deployment_control_record \
         SELECT 'event-transition-witness-alias', inserted_transaction_sequence, \
                record_sha256, schema_id, schema_sha256, canonical_record_jcs \
           FROM pale_deployment_control_record \
          WHERE schema_id='pale/internal-event-transition-witness.v1' \
          ORDER BY inserted_transaction_sequence DESC LIMIT 1; \
         UPDATE pale_deployment_event_index SET record_ref='event-transition-witness-alias' \
          WHERE index_role='transition_witness' \
            AND rowid=(SELECT MAX(rowid) FROM pale_deployment_event_index \
                        WHERE index_role='transition_witness');",
        ),
        Fault::CoherentWitnessRewrite,
        Fault::EqualOrdinalOtherAttempt,
    ];

    for (fault_index, fault) in faults.iter().enumerate() {
        let directory = support::private_tempdir();
        let (location, identity, _) = support::initialize(
            &directory,
            ControlStoreKind::DeploymentControl,
            &format!("resident-initial-event-corruption-{fault_index}"),
        );
        let coordinate = AuthorityStoreCoordinate::new(
            identity.logical_store_id().clone(),
            identity.control_incarnation_id().clone(),
        );
        let resolver = RecoveryResolver::default();
        let active = subscription(
            "initial-delivery-corruption-subscription",
            SubscriptionLifecycle::Active,
        );
        let producer = InitialProducerFixture::new().0;
        let current_writer = writer();
        let store =
            DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
                .unwrap();
        let mut source = FixedInputSource::default();
        let mut resident = open_resident(&store, coordinate.clone(), &resolver, &mut source, 1);
        commit_subscription(
            &mut resident,
            &store,
            &coordinate,
            EventCustodyTransitionInput::AdmitSubscription(&active),
        );

        let mut second_plan = None;
        for suffix in ["corruption-first", "corruption-second"] {
            let admitted_event = event(suffix);
            let event_ref =
                ResidentRecordRefV1::parse(format!("initial-delivery-event-record-{suffix}"))
                    .unwrap();
            let verifier = |_: &EventPublicationRaceClosure<'_>| Ok(());
            let writer_ref = ResidentRecordRefV1::parse("initial-delivery-writer-record").unwrap();
            let owner = AdmittedOwnerPublication::new(
                AdmittedEventRecord::new(&event_ref, &admitted_event),
                &producer.allocation_ref,
                &producer.allocation,
                &writer_ref,
                &current_writer,
                EventPublicationState::Active,
                &verifier,
            );
            let transaction = (&store).prepare_transaction(&coordinate).unwrap();
            let (mut completed, _receipt) = resident
                .commit_event_custody(
                    &mut &store,
                    transaction,
                    EventCustodyTransitionInput::AdmitOwnerReference(owner),
                )
                .unwrap();
            assert_eq!(
                completed.take_continuation().map(|value| value.kind()),
                Some(EventCustodyContinuationKind::ProjectOwnerReference)
            );
            let plan =
                InitialEventDeliveryPlan::admit(event_ref, admitted_event, active.clone()).unwrap();
            let limits = InitialEventDeliveryRecoveryLimits::new(8, 8, 1 << 20, 1 << 22).unwrap();
            advance_initial_event_delivery(&mut resident, &mut &store, &plan, limits).unwrap();
            second_plan = Some(plan);
        }
        let plan = second_plan.unwrap();
        match fault {
            Fault::Sql(sql) => store.connection().unwrap().execute_batch(sql).unwrap(),
            Fault::CoherentWitnessRewrite => {
                rewrite_record_body(&store, "pale/internal-event-commit-witness.v1", |value| {
                    value["committed_at"] = serde_json::json!(2)
                })
            }
            Fault::EqualOrdinalOtherAttempt => {
                replace_stream_head_with_equal_ordinal_other_attempt(&store)
            }
        }
        let limits = InitialEventDeliveryRecoveryLimits::new(8, 8, 1 << 20, 1 << 22).unwrap();
        assert!(
            recover_initial_event_delivery(&resident, &mut &store, &plan, limits,).is_err(),
            "fault {fault_index} ({fault:?}) must fail closed"
        );
    }
}
