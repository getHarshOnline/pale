//! Behavioral laws for application-owned Event custody.

use std::collections::VecDeque;

use pale_core::records::*;
use pale_ports::{
    AuthorityStoreCoordinate, BoundedAuthorityEntryBytes, ControlRecord, ControlStoreError,
    ControlStoreErrorKind, EventCustodyCommitOutcome, EventCustodyPort,
    EventCustodyReadBackOutcome, EventCustodyTransitionKind, EventInboxPage, EventInboxPageRead,
    EventIndexObservation, EventIndexPredecessor, EventIndexQuery, EventPublicationRole,
    EventRecordObservation, PaleDerivedOutboxCursor, PaleDerivedOutboxPage,
    PaleDerivedOutboxPageBoundary, PaleDerivedOutboxPageEntry, PaleDerivedOutboxPageRead,
    PreparedAuthorityAppend, SequencedAuthorityEntry,
};
use serde::{de::DeserializeOwned, Serialize};

use super::*;

#[path = "event_custody/initial_delivery.rs"]
mod initial_delivery;
#[path = "event_custody/publication.rs"]
mod publication;

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("record reference")
}

fn artifact(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(sha(byte)).expect("artifact digest")
}

fn assert_private_role_coordinate(prepared: &PreparedEventCustodyTransition) {
    for member in prepared.candidate().members() {
        let role = member.kind().required_schema_id();
        if role.starts_with("pale/internal-") {
            assert_eq!(member.record().schema_id().as_str(), role);
            assert_eq!(
                member.record().schema_sha256().as_str(),
                pale_core::sha256_hex(role.as_bytes())
            );
        }
    }
}

fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("signature")
}

fn signed_wire<P, D, W>(preimage: &P, digest_name: &str, digest: &D) -> W
where
    P: Serialize,
    D: Serialize,
    W: DeserializeOwned,
{
    let mut value = serde_json::to_value(preimage).expect("preimage serialization");
    let object = value.as_object_mut().expect("signed-record object");
    object.insert(
        digest_name.to_owned(),
        serde_json::to_value(digest).expect("digest serialization"),
    );
    object.insert(
        "signature".to_owned(),
        serde_json::to_value(signature()).expect("signature serialization"),
    );
    serde_json::from_value(value).expect("signed-record wire")
}

fn signing_authority() -> SigningAuthorityBindingV1 {
    SigningAuthorityBindingV1::integrated_runtime(
        ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        ResidentKeyIdV1::parse("key-1").expect("key"),
        SigningKeyEpochV1::new(1).expect("key epoch"),
        IntegratedRuntimeAuthorityV1 {
            deployment_runtime_authority_ref: record_ref("runtime-authority"),
            deployment_runtime_authority_sha256: artifact('1'),
            deployment_binding_ref: record_ref("deployment-binding"),
            deployment_binding_sha256: artifact('2'),
            checked_principal_trust_checkpoint_ref: record_ref("trust-checkpoint"),
            checked_principal_trust_checkpoint_sha256: artifact('3'),
        },
    )
    .expect("signing authority")
}

fn owner_coordinate(label: &str, digest: char) -> OwnerRecordCoordinateV1 {
    OwnerRecordCoordinateV1 {
        owner_system_id: OwnerSystemIdV1::parse("owner-system").expect("owner system"),
        record_ref: record_ref(label),
        record_schema_id: SchemaId::parse("owner/record.v1").expect("owner schema"),
        record_schema_sha256: SchemaDigest::parse(sha('4')).expect("owner schema digest"),
        record_content_sha256: artifact(digest),
        native_revision: NativeRevisionV1 {
            native_revision_owner: NativeRevisionOwnerV1::parse("owner-system")
                .expect("native revision owner"),
            native_revision_token: NativeRevisionTokenV1::parse(format!("revision-{label}"))
                .expect("native revision token"),
        },
    }
}

fn admitted_occurrence(
    label: &str,
    event_ref: &ResidentRecordRefV1,
    event_sha256: &EventReferenceDigest,
) -> InteractionOccurrenceV1 {
    let authority = signing_authority();
    let runtime_authority = authority
        .integrated_runtime_authority()
        .expect("integrated runtime authority");
    let preimage = InteractionOccurrencePreimageV1 {
        schema_id: SchemaId::parse(INTERACTION_OCCURRENCE_V1_SCHEMA).expect("occurrence schema"),
        schema_sha256: SchemaDigest::parse(sha('5')).expect("occurrence schema digest"),
        deployment_runtime_authority_ref: runtime_authority
            .deployment_runtime_authority_ref
            .clone(),
        deployment_runtime_authority_sha256: runtime_authority
            .deployment_runtime_authority_sha256
            .clone(),
        occurrence_id: InteractionOccurrenceIdV1::parse(label).expect("occurrence identity"),
        event_reference_ref: event_ref.clone(),
        event_reference_sha256: event_sha256.clone(),
        native_container: owner_coordinate(&format!("container-{label}"), '6'),
        container_genre: NativeContainerGenreV1::parse("discussion").expect("container genre"),
        root_placement: OwnerRecordKnowledgeV1::Unknown,
        thread_placement: OwnerRecordKnowledgeV1::Unknown,
        parent_placement: OwnerRecordKnowledgeV1::Unknown,
        referenced_records: Vec::new(),
        actor_provenance: owner_coordinate(&format!("actor-{label}"), '7'),
        participant_snapshot: OwnerRecordKnowledgeV1::Unknown,
        audience_snapshot: OwnerRecordKnowledgeV1::Unknown,
        visibility: InteractionVisibilityV1::parse("private").expect("visibility"),
        disclosure_ceiling: InteractionDisclosureCeilingV1::parse("private")
            .expect("disclosure ceiling"),
        mention_and_cc_records: Vec::new(),
        response_expectation: InteractionResponseExpectationV1::Unknown,
        lifecycle: InteractionLifecycleProjectionV1::Current,
        classification: DataClassificationV1::parse("confidential").expect("classification"),
        payload_resolution_policy: PayloadResolutionPolicyV1::parse("reference-only")
            .expect("payload policy"),
        projected_at: UnixTimeV1::new(3).expect("projection time"),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("issuer epoch"),
    };
    let digest = InteractionOccurrenceDigest::hash_canonical(&preimage).expect("occurrence digest");
    let wire = signed_wire(&preimage, "canonical_occurrence_sha256", &digest);
    InteractionOccurrenceV1::admit_contextual_wire(
        wire,
        &SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            signing_authority: &authority,
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("admitted occurrence")
}

fn admitted_acknowledgement(
    event: &EventReferenceV1,
    attempt: &EventDeliveryAttempt,
    subscription: &AdmittedEventSubscription,
) -> EventAcknowledgementV1 {
    let destination_node = ResidentNodeIdV1::parse("node-2").expect("destination node");
    let delegation_ref = record_ref("acknowledging-node-delegation");
    let delegation_sha256 = artifact('8');
    let preimage = EventAcknowledgementPreimageV1 {
        schema_id: SchemaId::parse(EVENT_ACKNOWLEDGEMENT_V1_SCHEMA).expect("ack schema"),
        schema_sha256: SchemaDigest::parse(sha('9')).expect("ack schema digest"),
        canonical_event_identity: projection::event_identity(event),
        canonical_event_sha256: event.canonical_event_sha256().clone(),
        delivery_attempt_id: attempt.attempt_id().clone(),
        subscription_id: subscription.subscription_id().clone(),
        destination_principal_id: ResidentPrincipalIdV1::parse("principal-1")
            .expect("destination principal"),
        destination_node_id: None,
        disposition: EventAcknowledgementDispositionV1::Accepted,
        reason_code: None,
        consumer_state_version: Some(
            ConsumerStateVersionV1::parse("consumer-state-1").expect("consumer state"),
        ),
        duplicate_of_ack_ref: None,
        duplicate_of_ack_sha256: None,
        quarantine_ref: None,
        quarantine_sha256: None,
        acknowledged_at: UnixTimeV1::new(4).expect("acknowledgement time"),
        acknowledging_principal_id: ResidentPrincipalIdV1::parse("principal-1")
            .expect("acknowledging principal"),
        acknowledging_node_id: destination_node.clone(),
        acknowledging_principal_key_epoch: PrincipalKeyEpochV1::new(1)
            .expect("principal key epoch"),
        acknowledging_node_delegation_ref: delegation_ref.clone(),
        acknowledging_node_delegation_sha256: delegation_sha256.clone(),
    };
    let digest = EventAcknowledgementDigest::hash_canonical(&preimage).expect("ack digest");
    let wire = signed_wire(&preimage, "canonical_acknowledgement_sha256", &digest);
    EventAcknowledgementV1::admit_contextual_wire(
        wire,
        &EventAcknowledgementAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            destination_principal_id: &preimage.destination_principal_id,
            destination_node_id: None,
            acknowledging_node_id: &destination_node,
            principal_key_epoch: preimage.acknowledging_principal_key_epoch,
            node_delegation_ref: &delegation_ref,
            node_delegation_sha256: &delegation_sha256,
        },
        |_, _, _, _| Ok(()),
    )
    .expect("admitted acknowledgement")
}

fn admitted_edge(
    label: &str,
    source_ref: &ResidentRecordRefV1,
    source: &InteractionOccurrenceV1,
    destination_ref: &ResidentRecordRefV1,
    destination: &InteractionOccurrenceV1,
) -> GenealogyEdgeV1 {
    let authority = signing_authority();
    let runtime_authority = authority
        .integrated_runtime_authority()
        .expect("integrated runtime authority");
    let preimage = GenealogyEdgePreimageV1 {
        schema_id: SchemaId::parse(GENEALOGY_EDGE_V1_SCHEMA).expect("edge schema"),
        schema_sha256: SchemaDigest::parse(sha('a')).expect("edge schema digest"),
        deployment_runtime_authority_ref: runtime_authority
            .deployment_runtime_authority_ref
            .clone(),
        deployment_runtime_authority_sha256: runtime_authority
            .deployment_runtime_authority_sha256
            .clone(),
        edge_id: GenealogyEdgeIdV1::parse(label).expect("edge identity"),
        source_occurrence_ref: source_ref.clone(),
        source_occurrence_sha256: source.canonical_occurrence_sha256().clone(),
        destination_occurrence_ref: destination_ref.clone(),
        destination_occurrence_sha256: destination.canonical_occurrence_sha256().clone(),
        proof: GenealogyProofV1::NativeStructural {
            relation: GenealogyRelationV1::parse("promoted-into").expect("relation"),
            native_owner_evidence: owner_coordinate("genealogy-proof", 'b'),
        },
        admitted_at: UnixTimeV1::new(5).expect("admission time"),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("issuer epoch"),
    };
    let digest = GenealogyEdgeDigest::hash_canonical(&preimage).expect("edge digest");
    let wire = signed_wire(&preimage, "canonical_genealogy_edge_sha256", &digest);
    GenealogyEdgeV1::admit_contextual_wire(
        wire,
        &SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            signing_authority: &authority,
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("admitted genealogy edge")
}

fn admitted_frontier(
    label: &str,
    subject_ref: &str,
    status: ReconciliationDimensionStatusV1,
) -> ReconciliationFrontierV1 {
    let authority = signing_authority();
    let runtime_authority = authority
        .integrated_runtime_authority()
        .expect("integrated runtime authority");
    let preimage = ReconciliationFrontierPreimageV1 {
        schema_id: SchemaId::parse(RECONCILIATION_FRONTIER_V1_SCHEMA).expect("frontier schema"),
        schema_sha256: SchemaDigest::parse(sha('c')).expect("frontier schema digest"),
        deployment_runtime_authority_ref: runtime_authority
            .deployment_runtime_authority_ref
            .clone(),
        deployment_runtime_authority_sha256: runtime_authority
            .deployment_runtime_authority_sha256
            .clone(),
        frontier_id: ReconciliationFrontierIdV1::parse(label).expect("frontier identity"),
        subject: ReconciliationSubjectV1::Projection {
            projection_ref: record_ref(subject_ref),
            projection_sha256: artifact('d'),
        },
        dimensions: vec![ReconciliationDimensionEntryV1 {
            dimension: ReconciliationDimensionV1::Delivery,
            status,
            evidence: None,
        }],
        source_occurrence_refs: Vec::new(),
        owner_record_revisions: Vec::new(),
        generated_at: UnixTimeV1::new(6).expect("frontier time"),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("issuer epoch"),
    };
    let digest = ReconciliationFrontierDigest::hash_canonical(&preimage).expect("frontier digest");
    let wire = signed_wire(
        &preimage,
        "canonical_reconciliation_frontier_sha256",
        &digest,
    );
    ReconciliationFrontierV1::admit_contextual_wire(
        wire,
        &SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            signing_authority: &authority,
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("admitted reconciliation frontier")
}

fn event(event_id: &str, source_version: &str) -> EventReferenceV1 {
    let preimage = EventReferencePreimageV1 {
        schema_id: SchemaId::parse(EVENT_REFERENCE_V1_SCHEMA).expect("event schema"),
        schema_sha256: SchemaDigest::parse(sha('a')).expect("event schema digest"),
        event_id: EventIdV1::parse(event_id).expect("event identity"),
        event_type: EventTypeV1::parse("owner.record.committed").expect("event type"),
        event_version: 1,
        lifecycle_namespace: LifecycleNamespaceV1::parse("owner.record").expect("lifecycle"),
        origin_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        origin_node_id: ResidentNodeIdV1::parse("node-1").expect("node"),
        source_owner: SourceOwnerV1::parse("owner-system").expect("source owner"),
        source_record_ref: record_ref("owner-record-1"),
        source_record_schema_id: SchemaId::parse("owner/record.v1").expect("source schema"),
        source_record_schema_sha256: SchemaDigest::parse(sha('b')).expect("source schema digest"),
        source_record_content_sha256: ArtifactDigest::parse(sha('c')).expect("source digest"),
        source_version: SourceVersionV1::parse(source_version).expect("source version"),
        target_scope: EventScopeV1 {
            principal_scope: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
            controller_scope: ResidentControllerIdV1::parse("controller-1").expect("controller"),
            relationship_scope: None,
            environment: DeploymentEnvironmentV1::parse("test").expect("environment"),
        },
        payload_ref: None,
        payload_sha256: None,
        classification: DataClassificationV1::parse("confidential").expect("classification"),
        ordering_key: None,
        sequence: None,
        occurred_at: UnixTimeV1::new(1).expect("occurrence time"),
        recorded_at: UnixTimeV1::new(2).expect("record time"),
        correlation_id: EventCorrelationIdV1::parse("correlation-1").expect("correlation"),
        causation_id: None,
        producer_epoch: ProducerEpochV1::parse("1".repeat(32)).expect("producer epoch"),
        expires_at: None,
    };
    let digest = EventReferenceDigest::hash_canonical(&preimage).expect("event digest");
    let mut wire = serde_json::to_value(&preimage).expect("event preimage");
    wire.as_object_mut().expect("event object").insert(
        "canonical_event_sha256".to_owned(),
        serde_json::to_value(digest).expect("event digest serialization"),
    );
    EventReferenceV1::admit_contextual_wire(
        serde_json::from_value(wire).expect("event wire"),
        &preimage.schema_sha256,
        |_| Ok(()),
    )
    .expect("valid event")
}

fn subscription(id: &str, reference: &str, frontier: u64) -> AdmittedEventSubscription {
    AdmittedEventSubscription::admit(
        record_ref(reference),
        EventSubscriptionDeclaration {
            subscription_id: EventSubscriptionIdV1::parse(id).expect("subscription"),
            source_owner: SourceOwnerV1::parse("owner-system").expect("source owner"),
            event_types: vec![EventTypeV1::parse("owner.record.committed").expect("event type")],
            destination_principal_id: ResidentPrincipalIdV1::parse("principal-1")
                .expect("principal"),
            destination_node_id: None,
            ingress_frontier: JournalSequenceV1::new(frontier).expect("frontier"),
        },
        SubscriptionLifecycle::Active,
    )
    .expect("valid subscription")
}

fn authority_store() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("control-store").expect("store"),
        ControlIncarnationIdV1::parse("incarnation-1").expect("incarnation"),
    )
}

fn append(store: AuthorityStoreCoordinate) -> PreparedAuthorityAppend {
    PreparedAuthorityAppend::new(
        store,
        None,
        SequencedAuthorityEntry::new(
            JournalSequenceV1::new(1).expect("journal sequence"),
            BoundedAuthorityEntryBytes::new(b"{}".to_vec(), 2).expect("journal bytes"),
        ),
    )
    .expect("prepared append")
}

fn retained_transition() -> PreparedEventCustodyTransition {
    let event = event("derived-event", "revision-1");
    let event_ref = record_ref("derived-event-record");
    let occurrence = admitted_occurrence(
        "derived-occurrence",
        &event_ref,
        event.canonical_event_sha256(),
    );
    prepare(EventCustodyTransitionInput::ProjectInteractionOccurrence {
        event: AdmittedEventRecord::new(&event_ref, &event),
        occurrence: AdmittedEventRecord::new(&record_ref("derived-record-1"), &occurrence),
        retention_record_ref: &record_ref("derived-outbox-1"),
        maximum_publication_bytes: 16_384,
    })
    .expect("prepared derived publication")
}

fn derived_recovery_page(
    prepared: &PreparedEventCustodyTransition,
) -> (PaleDerivedOutboxPageRead, PaleDerivedOutboxPage) {
    let mutation = prepared
        .candidate()
        .index_mutations()
        .iter()
        .find(|mutation| matches!(mutation.query(), EventIndexQuery::PaleDerivedOutbox(_)))
        .expect("outbox index");
    let EventIndexQuery::PaleDerivedOutbox(publication_key) = mutation.query() else {
        panic!("derived transition must use the derived outbox index")
    };
    let cursor = PaleDerivedOutboxCursor::new(publication_key.clone());
    let request = PaleDerivedOutboxPageRead::new(authority_store(), None, 1)
        .expect("derived recovery request");
    let page = PaleDerivedOutboxPage::new(
        &request,
        vec![PaleDerivedOutboxPageEntry::new(
            cursor,
            prepared
                .candidate()
                .members()
                .iter()
                .find(|member| {
                    member.kind() == pale_ports::EventTransitionMemberKind::PaleDerivedOutbox
                })
                .expect("outbox record")
                .record()
                .clone(),
        )],
        PaleDerivedOutboxPageBoundary::Complete,
    )
    .expect("derived recovery page");
    (request, page)
}

fn prepare(
    input: EventCustodyTransitionInput<'_>,
) -> Result<PreparedEventCustodyTransition, EventCustodyProjectionError> {
    let selected_store = authority_store();
    // This fixture tests semantic projection and phase classification, not native custody.
    let transaction = pale_ports::ControlTransactionPreparation::new(
        selected_store.clone(),
        ControlTransactionSequenceV1::new(1).unwrap(),
    )
    .unwrap();
    prepare_event_custody_transition(
        selected_store.clone(),
        transaction,
        append(selected_store),
        input,
    )
}

enum CommitStep {
    New,
    Replay,
    Unknown,
}

struct FakeEventStore {
    commits: VecDeque<CommitStep>,
    read_back: EventCustodyReadBackOutcome,
    observations: usize,
    index_record: Option<ControlRecord>,
    derived_page: Option<PaleDerivedOutboxPage>,
}

impl FakeEventStore {
    fn with_commit(step: CommitStep, read_back: EventCustodyReadBackOutcome) -> Self {
        Self {
            commits: VecDeque::from([step]),
            read_back,
            observations: 0,
            index_record: None,
            derived_page: None,
        }
    }

    fn with_derived_page(mut self, page: PaleDerivedOutboxPage) -> Self {
        self.derived_page = Some(page);
        self
    }
}

impl EventCustodyPort for FakeEventStore {
    fn read_committed_members(
        &mut self,
        _: &pale_ports::EventCustodyHistoryRead,
    ) -> Result<pale_ports::EventCustodyHistoryObservation, ControlStoreError> {
        unreachable!("history reads are outside these controlled phase laws")
    }

    fn compare_and_commit(
        &mut self,
        _: &pale_ports::EventCustodyTransition,
        maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyCommitOutcome, ControlStoreError> {
        assert_eq!(maximum_journal_payload_bytes, 2);
        match self.commits.pop_front().expect("one expected commit") {
            CommitStep::New => Ok(EventCustodyCommitOutcome::Committed),
            CommitStep::Replay => Ok(EventCustodyCommitOutcome::ExactReplay),
            CommitStep::Unknown => Err(ControlStoreError::new(
                ControlStoreErrorKind::OutcomeUnknown,
            )),
        }
    }

    fn observe_exact(
        &mut self,
        _: &pale_ports::EventCustodyTransition,
        maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyReadBackOutcome, ControlStoreError> {
        assert_eq!(maximum_journal_payload_bytes, 2);
        self.observations += 1;
        Ok(self.read_back)
    }

    fn read_index(
        &mut self,
        _: &AuthorityStoreCoordinate,
        query: &EventIndexQuery,
        maximum_body_bytes: usize,
    ) -> Result<EventIndexObservation, ControlStoreError> {
        if maximum_body_bytes == 0
            || self.index_record.as_ref().is_some_and(|record| {
                record.canonical_record_jcs().as_slice().len() > maximum_body_bytes
            })
        {
            return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
        }
        self.index_record.clone().map_or_else(
            || Ok(EventIndexObservation::absent(query.clone())),
            |record| {
                EventIndexObservation::exact_record(query.clone(), record)
                    .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
            },
        )
    }

    fn read_record(
        &mut self,
        _: &AuthorityStoreCoordinate,
        coordinate: &pale_ports::ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<EventRecordObservation, ControlStoreError> {
        if maximum_body_bytes == 0 {
            return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
        }
        Ok(EventRecordObservation::absent(coordinate.clone()))
    }

    fn read_inbox_page(
        &mut self,
        _: &EventInboxPageRead,
    ) -> Result<EventInboxPage, ControlStoreError> {
        unreachable!("page reads are outside these phase laws")
    }

    fn read_pale_derived_outbox_page(
        &mut self,
        _: &PaleDerivedOutboxPageRead,
    ) -> Result<PaleDerivedOutboxPage, ControlStoreError> {
        self.derived_page
            .clone()
            .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::Storage))
    }
}

#[test]
fn delivery_ordinals_are_positive_exact_jcs_integers() {
    assert!(EventDeliveryOrdinal::new(0).is_err());
    assert_eq!(
        EventDeliveryOrdinal::new(1)
            .expect("positive ordinal")
            .get(),
        1
    );
    assert!(EventDeliveryOrdinal::new(9_007_199_254_740_992).is_err());
}

#[test]
fn fanout_is_stable_and_never_retroactive() {
    let admitted_event = event("event-1", "revision-1");
    let subscriptions = vec![
        subscription("subscription-b", "subscription-b-active", 1),
        subscription("subscription-a", "subscription-a-active", 1),
        subscription("subscription-old", "subscription-old-active", 3),
    ];
    let selected = plan_event_fanout(
        &admitted_event,
        JournalSequenceV1::new(2).expect("commit sequence"),
        &subscriptions,
    );
    assert_eq!(
        selected
            .iter()
            .map(|target| target.subscription_id().as_str())
            .collect::<Vec<_>>(),
        ["subscription-a", "subscription-b"]
    );
}

#[test]
fn subscription_event_types_must_be_nonempty_unique_and_sorted() {
    let make = |event_types| {
        AdmittedEventSubscription::admit(
            record_ref("subscription-invalid"),
            EventSubscriptionDeclaration {
                subscription_id: EventSubscriptionIdV1::parse("subscription-invalid")
                    .expect("subscription"),
                source_owner: SourceOwnerV1::parse("owner-system").expect("source owner"),
                event_types,
                destination_principal_id: ResidentPrincipalIdV1::parse("principal-1")
                    .expect("principal"),
                destination_node_id: None,
                ingress_frontier: JournalSequenceV1::new(1).expect("frontier"),
            },
            SubscriptionLifecycle::Active,
        )
    };
    assert!(make(Vec::new()).is_err());
    let repeated = EventTypeV1::parse("owner.record.committed").expect("event type");
    assert!(make(vec![repeated.clone(), repeated]).is_err());
}

#[test]
fn ordinary_exact_replay_never_mints_a_continuation() {
    let mut store = FakeEventStore::with_commit(
        CommitStep::Replay,
        EventCustodyReadBackOutcome::ConflictingOrPartial,
    );
    let mut completed = retained_transition()
        .commit(&mut store, 2)
        .expect("exact replay");
    assert_eq!(
        completed.evidence(),
        EventCustodyCommitEvidence::ExactReplay
    );
    assert_eq!(completed.take_continuation(), None);
    assert_eq!(store.observations, 0);
}

#[test]
fn new_or_same_call_recovery_mints_exactly_one_continuation() {
    let mut new_store =
        FakeEventStore::with_commit(CommitStep::New, EventCustodyReadBackOutcome::Predecessor);
    let mut newly_committed = retained_transition()
        .commit(&mut new_store, 2)
        .expect("new commit");
    assert_eq!(
        newly_committed
            .take_continuation()
            .map(|value| value.kind()),
        Some(EventCustodyContinuationKind::ExposePaleDerivedPublication)
    );
    assert_eq!(newly_committed.take_continuation(), None);

    let mut ambiguous_store = FakeEventStore::with_commit(
        CommitStep::Unknown,
        EventCustodyReadBackOutcome::ExactSuccessor,
    );
    let mut recovered = retained_transition()
        .commit(&mut ambiguous_store, 2)
        .expect("same-call recovery");
    assert_eq!(
        recovered.evidence(),
        EventCustodyCommitEvidence::RecoveredExactAfterAmbiguousCommit
    );
    assert_eq!(
        recovered.take_continuation().map(|value| value.kind()),
        Some(EventCustodyContinuationKind::ExposePaleDerivedPublication)
    );
    assert_eq!(ambiguous_store.observations, 1);
}

#[test]
fn restart_after_commit_before_continuation_recovers_one_stable_exposure_candidate() {
    let prepared = retained_transition();
    let (request, page) = derived_recovery_page(&prepared);
    let expected_retention = prepared
        .candidate()
        .members()
        .iter()
        .find(|member| member.kind() == pale_ports::EventTransitionMemberKind::PaleDerivedOutbox)
        .expect("outbox record")
        .record()
        .coordinate()
        .clone();
    let expected_publication = prepared.candidate().members()[0]
        .record()
        .coordinate()
        .clone();
    let mut store = FakeEventStore::with_commit(
        CommitStep::New,
        EventCustodyReadBackOutcome::ConflictingOrPartial,
    )
    .with_derived_page(page);

    let committed = prepared.commit(&mut store, 2).expect("durable commit");
    drop(committed);
    let recovered = read_pending_pale_derived_publications(&mut store, &request)
        .expect("restart recovery page");
    assert_eq!(recovered.entries().len(), 1);
    assert_eq!(
        recovered.entries()[0].retention_record(),
        &expected_retention
    );
    assert_eq!(
        recovered.entries()[0].publication_ref(),
        &record_ref("derived-record-1")
    );
    assert_eq!(
        recovered.entries()[0].publication_sha256().as_str(),
        expected_publication.record_sha256().as_str()
    );

    store.commits.push_back(CommitStep::Replay);
    let mut replayed = retained_transition()
        .commit(&mut store, 2)
        .expect("exact replay after restart");
    assert_eq!(replayed.take_continuation(), None);
    let repeated = read_pending_pale_derived_publications(&mut store, &request)
        .expect("idempotent recovery retry");
    assert_eq!(repeated, recovered);
}

#[test]
fn recovery_decodes_settled_heads_without_returning_them_as_pending() {
    let retained = PaleDerivedPublicationRetention::retained(
        record_ref("derived-retained"),
        record_ref("derived-record"),
        artifact('1'),
    );
    let settled = PaleDerivedPublicationRetention::settled(
        record_ref("derived-settled"),
        record_ref("derived-record"),
        artifact('1'),
        record_ref("derived-acknowledgement"),
        artifact('2'),
    );
    let prepared = prepare(EventCustodyTransitionInput::SettlePaleDerivedPublication {
        predecessor: &retained,
        successor: &settled,
    })
    .expect("settled derived publication");
    let (request, page) = derived_recovery_page(&prepared);
    let mut store = FakeEventStore::with_commit(
        CommitStep::Replay,
        EventCustodyReadBackOutcome::ConflictingOrPartial,
    )
    .with_derived_page(page);

    let recovered = read_pending_pale_derived_publications(&mut store, &request)
        .expect("settled head admission");
    assert!(recovered.entries().is_empty());
    assert_eq!(
        recovered.boundary(),
        &PaleDerivedOutboxPageBoundary::Complete
    );
}

#[test]
fn ambiguous_predecessor_is_not_blindly_retried() {
    let mut store = FakeEventStore::with_commit(
        CommitStep::Unknown,
        EventCustodyReadBackOutcome::Predecessor,
    );
    let failure = retained_transition()
        .commit(&mut store, 2)
        .expect_err("predecessor excludes commit");
    assert!(matches!(
        failure.reason(),
        EventCustodyCommitFailureReason::CommitExcludedByReadBack { .. }
    ));
    assert!(store.commits.is_empty());
    assert_eq!(store.observations, 1);
}

#[test]
fn acknowledgement_and_cursor_preserve_attempt_identity_and_contiguity() {
    let admitted_event = event("event-delivery", "revision-1");
    let event_ref = record_ref("event-delivery-record");
    let event_identity = projection::event_identity(&admitted_event);
    let subscription = subscription("subscription-delivery", "subscription-delivery-active", 1);
    let ordinal = EventDeliveryOrdinal::new(1).expect("first delivery ordinal");
    let attempt = EventDeliveryAttempt::initial(
        record_ref("attempt-1"),
        DeliveryAttemptIdV1::parse("attempt-1").expect("attempt identity"),
        subscription.subscription_id().clone(),
        event_identity.clone(),
        admitted_event.canonical_event_sha256().clone(),
        ordinal,
    );
    let pending = EventDeliveryState::new(
        record_ref("delivery-pending"),
        subscription.subscription_id().clone(),
        event_identity,
        admitted_event.canonical_event_sha256().clone(),
        attempt.attempt_id().clone(),
        ordinal,
        EventDeliveryDisposition::Pending,
    );
    let replay_ordinal = EventDeliveryOrdinal::new(2).expect("replay ordinal");
    let replay_attempt = EventDeliveryAttempt::replay(
        record_ref("attempt-2"),
        DeliveryAttemptIdV1::parse("attempt-2").expect("replay attempt identity"),
        subscription.subscription_id().clone(),
        pending.event_identity.clone(),
        pending.event_sha256.clone(),
        replay_ordinal,
    );
    let replay_delivery = EventDeliveryState::new(
        record_ref("delivery-replay-pending"),
        pending.subscription_id.clone(),
        pending.event_identity.clone(),
        pending.event_sha256.clone(),
        replay_attempt.attempt_id().clone(),
        replay_ordinal,
        EventDeliveryDisposition::Pending,
    );
    let stream_predecessor = EventDeliveryStream::new(
        record_ref("delivery-stream-1"),
        subscription.subscription_id().clone(),
        ordinal,
        attempt.attempt_id().clone(),
    );
    let stream_successor = EventDeliveryStream::new(
        record_ref("delivery-stream-2"),
        subscription.subscription_id().clone(),
        replay_ordinal,
        replay_attempt.attempt_id().clone(),
    );
    let _prepared = prepare(EventCustodyTransitionInput::EnqueueReplay {
        event: AdmittedEventRecord::new(&event_ref, &admitted_event),
        subscription: &subscription,
        predecessor: &pending,
        successor: &replay_delivery,
        attempt: &replay_attempt,
        stream_predecessor: &stream_predecessor,
        stream_successor: &stream_successor,
    })
    .expect("explicit replay has a distinct attempt and successor ordinal");

    let repeated_attempt = EventDeliveryAttempt::replay(
        record_ref("attempt-1-repeated"),
        attempt.attempt_id().clone(),
        subscription.subscription_id().clone(),
        pending.event_identity.clone(),
        pending.event_sha256.clone(),
        replay_ordinal,
    );
    assert!(prepare(EventCustodyTransitionInput::EnqueueReplay {
        event: AdmittedEventRecord::new(&event_ref, &admitted_event),
        subscription: &subscription,
        predecessor: &pending,
        successor: &replay_delivery,
        attempt: &repeated_attempt,
        stream_predecessor: &stream_predecessor,
        stream_successor: &stream_successor,
    })
    .is_err());

    let acknowledgement = admitted_acknowledgement(&admitted_event, &attempt, &subscription);
    let acknowledgement_ref = record_ref("acknowledgement-1");
    let acknowledged = EventDeliveryState::new(
        record_ref("delivery-acknowledged"),
        pending.subscription_id.clone(),
        pending.event_identity.clone(),
        pending.event_sha256.clone(),
        pending.attempt_id.clone(),
        pending.ordinal,
        EventDeliveryDisposition::Acknowledged {
            acknowledgement_ref: acknowledgement_ref.clone(),
            acknowledgement_sha256: acknowledgement.canonical_acknowledgement_sha256().clone(),
            disposition: *acknowledgement.disposition(),
        },
    );
    let acknowledgement_transition = prepare(EventCustodyTransitionInput::RecordAcknowledgement {
        attempt: &attempt,
        predecessor: &pending,
        successor: &acknowledged,
        acknowledgement: AdmittedEventRecord::new(&acknowledgement_ref, &acknowledgement),
    })
    .expect("delivery-only acknowledgement transition");
    assert_eq!(
        acknowledgement_transition.candidate().kind(),
        EventCustodyTransitionKind::RecordAcknowledgement
    );

    let cursor = EventDeliveryCursor::new(
        record_ref("delivery-cursor-1"),
        subscription.subscription_id().clone(),
        ordinal,
        acknowledgement_ref.clone(),
        acknowledgement.canonical_acknowledgement_sha256().clone(),
    );
    let cursor_transition = prepare(EventCustodyTransitionInput::AdvanceDeliveryCursor {
        attempt: &attempt,
        acknowledgement: AdmittedEventRecord::new(&acknowledgement_ref, &acknowledgement),
        predecessor: None,
        successor: &cursor,
    })
    .expect("first contiguous cursor transition");
    assert!(matches!(
        cursor_transition.candidate().index_mutations()[0].expected(),
        EventIndexPredecessor::Absent
    ));

    let replay_acknowledgement =
        admitted_acknowledgement(&admitted_event, &replay_attempt, &subscription);
    let replay_acknowledgement_ref = record_ref("acknowledgement-2");
    let second_cursor = EventDeliveryCursor::new(
        record_ref("delivery-cursor-2"),
        subscription.subscription_id().clone(),
        replay_ordinal,
        replay_acknowledgement_ref.clone(),
        replay_acknowledgement
            .canonical_acknowledgement_sha256()
            .clone(),
    );
    let second_cursor_transition = prepare(EventCustodyTransitionInput::AdvanceDeliveryCursor {
        attempt: &replay_attempt,
        acknowledgement: AdmittedEventRecord::new(
            &replay_acknowledgement_ref,
            &replay_acknowledgement,
        ),
        predecessor: Some(&cursor),
        successor: &second_cursor,
    })
    .expect("second stream ordinal is cursor-compatible");
    assert!(matches!(
        second_cursor_transition.candidate().index_mutations()[0].expected(),
        EventIndexPredecessor::Exact(_)
    ));

    let skipped_acknowledgement_ref = acknowledgement_ref.clone();
    let skipped = EventDeliveryCursor::new(
        record_ref("delivery-cursor-3"),
        subscription.subscription_id().clone(),
        EventDeliveryOrdinal::new(3).expect("third ordinal"),
        skipped_acknowledgement_ref.clone(),
        acknowledgement.canonical_acknowledgement_sha256().clone(),
    );
    assert!(prepare(EventCustodyTransitionInput::AdvanceDeliveryCursor {
        attempt: &attempt,
        acknowledgement: AdmittedEventRecord::new(&skipped_acknowledgement_ref, &acknowledgement,),
        predecessor: Some(&cursor),
        successor: &skipped,
    })
    .is_err());
}

#[test]
fn source_frontiers_advance_independently_and_monotonically() {
    let first = event("event-frontier-1", "revision-1");
    let second = event("event-frontier-2", "revision-2");
    let subscription_id =
        EventSubscriptionIdV1::parse("subscription-frontier").expect("subscription identity");
    let prior = EventSourceFrontier::new(
        record_ref("source-frontier-1"),
        subscription_id.clone(),
        first.source_owner().clone(),
        projection::event_identity(&first),
        first.canonical_event_sha256().clone(),
        Some(OwnerEventSequenceV1::new(1).expect("owner sequence")),
    );
    let successor = EventSourceFrontier::new(
        record_ref("source-frontier-2"),
        subscription_id,
        second.source_owner().clone(),
        projection::event_identity(&second),
        second.canonical_event_sha256().clone(),
        Some(OwnerEventSequenceV1::new(2).expect("owner sequence")),
    );
    let transition = prepare(EventCustodyTransitionInput::AdvanceSourceFrontier {
        predecessor: Some(&prior),
        successor: &successor,
    })
    .expect("monotone source frontier");
    assert!(matches!(
        transition.candidate().index_mutations()[0].expected(),
        EventIndexPredecessor::Exact(_)
    ));

    let repeated = EventSourceFrontier::new(
        record_ref("source-frontier-repeat"),
        successor.subscription_id.clone(),
        successor.source_owner.clone(),
        successor.event_identity.clone(),
        successor.event_sha256.clone(),
        successor.owner_sequence,
    );
    assert!(prepare(EventCustodyTransitionInput::AdvanceSourceFrontier {
        predecessor: Some(&successor),
        successor: &repeated,
    })
    .is_err());
}

#[test]
fn occurrence_and_genealogy_projection_require_exact_directed_endpoints() {
    let source_event = event("event-source", "revision-source");
    let destination_event = event("event-destination", "revision-destination");
    let source_event_ref = record_ref("event-source-record");
    let destination_event_ref = record_ref("event-destination-record");
    let source_ref = record_ref("occurrence-source-record");
    let destination_ref = record_ref("occurrence-destination-record");
    let source = admitted_occurrence(
        "occurrence-source",
        &source_event_ref,
        source_event.canonical_event_sha256(),
    );
    let destination = admitted_occurrence(
        "occurrence-destination",
        &destination_event_ref,
        destination_event.canonical_event_sha256(),
    );
    let _source_prepared = prepare(EventCustodyTransitionInput::ProjectInteractionOccurrence {
        event: AdmittedEventRecord::new(&source_event_ref, &source_event),
        occurrence: AdmittedEventRecord::new(&source_ref, &source),
        retention_record_ref: &record_ref("source-outbox"),
        maximum_publication_bytes: 16_384,
    })
    .expect("source occurrence projection");
    let _destination_prepared =
        prepare(EventCustodyTransitionInput::ProjectInteractionOccurrence {
            event: AdmittedEventRecord::new(&destination_event_ref, &destination_event),
            occurrence: AdmittedEventRecord::new(&destination_ref, &destination),
            retention_record_ref: &record_ref("destination-outbox"),
            maximum_publication_bytes: 16_384,
        })
        .expect("destination occurrence projection");

    let edge = admitted_edge(
        "edge-source-destination",
        &source_ref,
        &source,
        &destination_ref,
        &destination,
    );
    let edge_ref = record_ref("edge-source-destination-record");
    let transition = prepare(EventCustodyTransitionInput::ProjectGenealogyEdge {
        source: AdmittedEventRecord::new(&source_ref, &source),
        destination: AdmittedEventRecord::new(&destination_ref, &destination),
        edge: AdmittedEventRecord::new(&edge_ref, &edge),
        retention_record_ref: &record_ref("edge-outbox"),
        maximum_publication_bytes: 16_384,
    })
    .expect("directed truth-classed edge");
    assert_eq!(transition.candidate().index_assertions().len(), 2);

    assert!(prepare(EventCustodyTransitionInput::ProjectGenealogyEdge {
        source: AdmittedEventRecord::new(&destination_ref, &destination),
        destination: AdmittedEventRecord::new(&source_ref, &source),
        edge: AdmittedEventRecord::new(&edge_ref, &edge),
        retention_record_ref: &record_ref("edge-outbox"),
        maximum_publication_bytes: 16_384,
    })
    .is_err());
}

#[test]
fn reconciliation_uses_exact_subject_head_cas_and_permits_explicit_reopen() {
    let complete = admitted_frontier(
        "frontier-complete",
        "profile-subject",
        ReconciliationDimensionStatusV1::Complete,
    );
    let reopened = admitted_frontier(
        "frontier-reopened",
        "profile-subject",
        ReconciliationDimensionStatusV1::Pending,
    );
    let complete_ref = record_ref("frontier-complete-record");
    let reopened_ref = record_ref("frontier-reopened-record");
    let transition = prepare(EventCustodyTransitionInput::AdvanceReconciliationFrontier {
        predecessor: Some(AdmittedEventRecord::new(&complete_ref, &complete)),
        successor: AdmittedEventRecord::new(&reopened_ref, &reopened),
        retention_record_ref: &record_ref("frontier-outbox"),
        maximum_publication_bytes: 16_384,
    })
    .expect("dimension may reopen in a new immutable vector");
    assert!(matches!(
        transition.candidate().index_mutations()[0].expected(),
        EventIndexPredecessor::Exact(_)
    ));

    let another_subject = admitted_frontier(
        "frontier-other-subject",
        "other-profile-subject",
        ReconciliationDimensionStatusV1::Pending,
    );
    let another_ref = record_ref("frontier-other-subject-record");
    assert!(
        prepare(EventCustodyTransitionInput::AdvanceReconciliationFrontier {
            predecessor: Some(AdmittedEventRecord::new(&complete_ref, &complete)),
            successor: AdmittedEventRecord::new(&another_ref, &another_subject),
            retention_record_ref: &record_ref("frontier-outbox"),
            maximum_publication_bytes: 16_384,
        },)
        .is_err()
    );
}

#[test]
fn derived_outbox_retain_then_settle_without_replay_authority() {
    let pale_retained = PaleDerivedPublicationRetention::retained(
        record_ref("pale-outbox-retained"),
        record_ref("pale-publication"),
        artifact('e'),
    );
    let pale_settled = PaleDerivedPublicationRetention::settled(
        record_ref("pale-outbox-settled"),
        record_ref("pale-publication"),
        artifact('e'),
        record_ref("pale-settlement"),
        artifact('f'),
    );
    let pale_transition = prepare(EventCustodyTransitionInput::SettlePaleDerivedPublication {
        predecessor: &pale_retained,
        successor: &pale_settled,
    })
    .expect("Pale-derived retain-to-settle lineage");
    assert_eq!(
        pale_transition.candidate().kind().publication_role(),
        Some(EventPublicationRole::PaleDerivedOutbox)
    );

    let mismatched = PaleDerivedPublicationRetention::settled(
        record_ref("pale-outbox-invalid"),
        record_ref("pale-publication"),
        artifact('3'),
        record_ref("pale-settlement"),
        artifact('2'),
    );
    assert!(
        prepare(EventCustodyTransitionInput::SettlePaleDerivedPublication {
            predecessor: &pale_retained,
            successor: &mismatched,
        })
        .is_err()
    );

    let prepared = retained_transition();
    assert_private_role_coordinate(&prepared);
    let mut store = FakeEventStore::with_commit(
        CommitStep::Replay,
        EventCustodyReadBackOutcome::ConflictingOrPartial,
    );
    let mut replayed = prepared
        .commit(&mut store, 2)
        .expect("ordinary exact replay");
    assert_eq!(replayed.evidence(), EventCustodyCommitEvidence::ExactReplay);
    assert_eq!(replayed.take_continuation(), None);
    assert_eq!(store.observations, 0);
}

#[test]
fn event_identity_lookup_has_exact_three_way_classification() {
    let admitted_event = event("event-1", "revision-1");
    let event_ref = record_ref("event-record-1");
    let admitted = AdmittedEventRecord::new(&event_ref, &admitted_event);
    let mut resident_store =
        FakeEventStore::with_commit(CommitStep::New, EventCustodyReadBackOutcome::Predecessor);
    assert_eq!(
        classify_event_identity(&mut resident_store, &authority_store(), admitted, 16 * 1024)
            .expect("new identity"),
        EventIdentityDisposition::New
    );

    resident_store.index_record = Some(
        projection::public_record(
            AdmittedEventRecord::new(&event_ref, &admitted_event),
            admitted_event.canonical_event_sha256().as_str(),
        )
        .expect("projected event"),
    );
    assert!(matches!(
        classify_event_identity(&mut resident_store, &authority_store(), admitted, 16 * 1024)
            .expect("exact replay"),
        EventIdentityDisposition::ExactReplay { .. }
    ));

    let alias_ref = record_ref("event-record-alias");
    resident_store.index_record = Some(
        projection::public_record(
            AdmittedEventRecord::new(&alias_ref, &admitted_event),
            admitted_event.canonical_event_sha256().as_str(),
        )
        .expect("projected event alias"),
    );
    assert!(matches!(
        classify_event_identity(&mut resident_store, &authority_store(), admitted, 16 * 1024)
            .expect("same bytes at another reference"),
        EventIdentityDisposition::Conflict { .. }
    ));

    let unlike = event("event-1", "revision-2");
    let unlike_ref = record_ref("event-record-conflict");
    let EventIdentityDisposition::Conflict { incumbent } = classify_event_identity(
        &mut resident_store,
        &authority_store(),
        AdmittedEventRecord::new(&unlike_ref, &unlike),
        16 * 1024,
    )
    .expect("conflicting identity bytes") else {
        panic!("same identity with different canonical bytes must conflict");
    };
    let conflict_ref = record_ref("event-identity-conflict");
    let quarantine = prepare(EventCustodyTransitionInput::QuarantineIdentityConflict {
        incumbent: &incumbent,
        candidate: AdmittedEventRecord::new(&unlike_ref, &unlike),
        conflict_ref: &conflict_ref,
    })
    .expect("bounded conflict quarantine");
    assert!(matches!(
        quarantine.candidate().index_assertions()[0].expected(),
        EventIndexPredecessor::Exact(record) if record == &incumbent
    ));
}

#[test]
fn event_identity_lookup_propagates_the_selected_body_limit() {
    let admitted_event = event("bounded-event", "revision-1");
    let event_ref = record_ref("bounded-event-record");
    let admitted = AdmittedEventRecord::new(&event_ref, &admitted_event);
    let record =
        projection::public_record(admitted, admitted_event.canonical_event_sha256().as_str())
            .expect("projected event");
    let exact = record.canonical_record_jcs().as_slice().len();
    let mut resident_store =
        FakeEventStore::with_commit(CommitStep::New, EventCustodyReadBackOutcome::Predecessor);
    resident_store.index_record = Some(record);

    assert!(matches!(
        classify_event_identity(&mut resident_store, &authority_store(), admitted, exact),
        Ok(EventIdentityDisposition::ExactReplay { .. })
    ));
    for maximum_body_bytes in [0, exact - 1] {
        assert!(matches!(
            classify_event_identity(
                &mut resident_store,
                &authority_store(),
                admitted,
                maximum_body_bytes,
            ),
            Err(EventIdentityLookupError::Store(ref error))
                if error.kind() == ControlStoreErrorKind::BoundExceeded
        ));
    }
}
