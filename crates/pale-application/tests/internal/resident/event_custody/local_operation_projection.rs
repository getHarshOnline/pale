//! Behavioral laws for exact local-operation Event projection.

use super::super::{
    advance_local_operation_publication as advance_stage_lazy, AdmittedEventRecord,
    LocalOperationPublicationAdvance, LocalOperationPublicationIssuance,
    LocalOperationPublicationReadmission, LocalOperationPublicationStage,
};
use super::*;
use crate::local_operation_start_fixture::{
    artifact_digest, checkpoint_preimage, publication_writer, record_ref, signature,
    store_coordinate,
};
use crate::producer_record_fixture::{admit_allocation, producer_preimage};
use crate::resident::application::tests::empty_application;
use crate::resident::journal::tests::MemoryAuthorityStore;
use crate::resident::operation_observation_custody::tests::{
    admit_initial_checkpoint, admit_successor_checkpoint, operation_publication_case,
    OperationFixture,
};
use crate::resident::record_projection;
use crate::resident::ResidentApplication;
use pale_ports::{
    ControlRecord, ControlRecordCoordinate, ControlStoreError, ControlStoreErrorKind,
    ControlTransactionPort, ControlTransactionPreparation, DurableAuthorityStorePort,
    EventCustodyCommitOutcome, EventCustodyHistoryObservation, EventCustodyHistoryRead,
    EventCustodyPort, EventCustodyReadBackOutcome, EventCustodyTransition, EventInboxPage,
    EventInboxPageRead, EventIndexObservation, EventIndexPredecessor, EventIndexQuery,
    EventRecordObservation, PaleDerivedOutboxPage, PaleDerivedOutboxPageRead,
};
use std::cell::Cell;
use std::collections::BTreeMap;

pub(in crate::resident) struct PublicationStore<'a> {
    authority: &'a MemoryAuthorityStore,
    coordinate: AuthorityStoreCoordinate,
    transaction: ControlTransactionSequenceV1,
    indexes: BTreeMap<EventIndexQuery, ControlRecord>,
    records: BTreeMap<ControlRecordCoordinate, ControlRecord>,
    fail_next_commit_schema: Option<SchemaId>,
}

impl<'a> PublicationStore<'a> {
    pub(in crate::resident) fn new(authority: &'a MemoryAuthorityStore) -> Self {
        Self {
            authority,
            coordinate: store_coordinate(),
            transaction: ControlTransactionSequenceV1::new(1).unwrap(),
            indexes: BTreeMap::new(),
            records: BTreeMap::new(),
            fail_next_commit_schema: None,
        }
    }

    pub(in crate::resident) fn fail_next_commit_for_schema(&mut self, schema: &str) {
        self.fail_next_commit_schema = Some(SchemaId::parse(schema).expect("fixture schema"));
    }

    pub(in crate::resident) fn record_count_for_schema(&self, schema: &str) -> usize {
        self.records
            .values()
            .filter(|record| record.schema_id().as_str() == schema)
            .count()
    }

    pub(in crate::resident) fn records_for_schema(&self, schema: &str) -> Vec<ControlRecord> {
        self.records
            .values()
            .filter(|record| record.schema_id().as_str() == schema)
            .cloned()
            .collect()
    }

    pub(in crate::resident) const fn committed_transition_count(&self) -> u64 {
        self.transaction.get() - 1
    }

    pub(in crate::resident) fn remove_only_occurrence_index(
        &mut self,
    ) -> Option<(EventIndexQuery, ControlRecord)> {
        let query = self
            .indexes
            .keys()
            .find(|query| matches!(query, EventIndexQuery::InteractionOccurrence(_)))?
            .clone();
        self.indexes.remove(&query).map(|record| (query, record))
    }

    pub(in crate::resident) fn restore_index(
        &mut self,
        query: EventIndexQuery,
        record: ControlRecord,
    ) {
        self.indexes.insert(query, record);
    }

    fn predecessor_matches(
        &self,
        query: &EventIndexQuery,
        expected: &EventIndexPredecessor,
    ) -> bool {
        match (expected, self.indexes.get(query)) {
            (EventIndexPredecessor::Absent, None) => true,
            (EventIndexPredecessor::Exact(expected), Some(observed)) => expected == observed,
            _ => false,
        }
    }
}

impl ControlTransactionPort for PublicationStore<'_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        if store != &self.coordinate {
            return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
        }
        ControlTransactionPreparation::new(store.clone(), self.transaction)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::TransactionExhausted))
    }
}

impl EventCustodyPort for PublicationStore<'_> {
    fn read_committed_members(
        &mut self,
        _request: &EventCustodyHistoryRead,
    ) -> Result<EventCustodyHistoryObservation, ControlStoreError> {
        unimplemented!("publication-stage test does not recover journal history")
    }

    fn compare_and_commit(
        &mut self,
        transition: &EventCustodyTransition,
        _maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyCommitOutcome, ControlStoreError> {
        if self.fail_next_commit_schema.as_ref().is_some_and(|schema| {
            transition
                .members()
                .iter()
                .any(|member| member.record().schema_id() == schema)
        }) {
            self.fail_next_commit_schema = None;
            return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
        }
        if transition.transaction().expected() != self.transaction
            || transition
                .index_assertions()
                .iter()
                .any(|assertion| !self.predecessor_matches(assertion.query(), assertion.expected()))
            || transition
                .index_mutations()
                .iter()
                .any(|mutation| !self.predecessor_matches(mutation.query(), mutation.expected()))
        {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::CandidateConflict,
            ));
        }
        self.authority
            .compare_and_append(transition.journal_append())
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::Storage))?;
        for member in transition.members() {
            self.records.insert(
                member.record().coordinate().clone(),
                member.record().clone(),
            );
        }
        for mutation in transition.index_mutations() {
            let successor = self
                .records
                .get(mutation.successor())
                .expect("candidate closes every successor")
                .clone();
            self.indexes.insert(mutation.query().clone(), successor);
        }
        self.transaction = transition.transaction().successor();
        Ok(EventCustodyCommitOutcome::Committed)
    }

    fn observe_exact(
        &mut self,
        _transition: &EventCustodyTransition,
        _maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyReadBackOutcome, ControlStoreError> {
        Ok(EventCustodyReadBackOutcome::ConflictingOrPartial)
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &EventIndexQuery,
        maximum_body_bytes: usize,
    ) -> Result<EventIndexObservation, ControlStoreError> {
        if maximum_body_bytes == 0 {
            return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
        }
        if store != &self.coordinate {
            return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
        }
        self.indexes.get(query).cloned().map_or_else(
            || Ok(EventIndexObservation::absent(query.clone())),
            |record| {
                if record.canonical_record_jcs().as_slice().len() > maximum_body_bytes {
                    return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
                }
                EventIndexObservation::exact_record(query.clone(), record)
                    .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::Storage))
            },
        )
    }

    fn read_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<EventRecordObservation, ControlStoreError> {
        if maximum_body_bytes == 0 {
            return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
        }
        if store != &self.coordinate {
            return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
        }
        self.records.get(coordinate).cloned().map_or_else(
            || Ok(EventRecordObservation::absent(coordinate.clone())),
            |record| {
                if record.canonical_record_jcs().as_slice().len() > maximum_body_bytes {
                    return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
                }
                EventRecordObservation::exact(coordinate.clone(), record)
                    .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::Storage))
            },
        )
    }

    fn read_inbox_page(
        &mut self,
        _request: &EventInboxPageRead,
    ) -> Result<EventInboxPage, ControlStoreError> {
        unimplemented!("publication-stage test does not read delivery inboxes")
    }

    fn read_pale_derived_outbox_page(
        &mut self,
        _request: &PaleDerivedOutboxPageRead,
    ) -> Result<PaleDerivedOutboxPage, ControlStoreError> {
        unimplemented!("publication-stage test does not sweep the outbox")
    }
}

fn signing_authority() -> SigningAuthorityBindingV1 {
    signing_authority_at("key-1", 1)
}

fn signing_authority_at(key: &str, epoch: u64) -> SigningAuthorityBindingV1 {
    signing_authority_for("principal-1", key, epoch)
}

fn signing_authority_for(principal: &str, key: &str, epoch: u64) -> SigningAuthorityBindingV1 {
    signing_authority_for_runtime(
        principal,
        key,
        epoch,
        "runtime-authority",
        artifact_digest('1'),
    )
}

fn signing_authority_for_runtime(
    principal: &str,
    key: &str,
    epoch: u64,
    runtime_authority_ref: &str,
    runtime_authority_sha256: ArtifactDigest,
) -> SigningAuthorityBindingV1 {
    SigningAuthorityBindingV1::integrated_runtime(
        ResidentPrincipalIdV1::parse(principal).unwrap(),
        ResidentKeyIdV1::parse(key).unwrap(),
        SigningKeyEpochV1::new(epoch).unwrap(),
        IntegratedRuntimeAuthorityV1 {
            deployment_runtime_authority_ref: record_ref(runtime_authority_ref),
            deployment_runtime_authority_sha256: runtime_authority_sha256,
            deployment_binding_ref: record_ref("deployment-binding"),
            deployment_binding_sha256: artifact_digest('2'),
            checked_principal_trust_checkpoint_ref: record_ref("trust-checkpoint"),
            checked_principal_trust_checkpoint_sha256: artifact_digest('3'),
        },
    )
    .unwrap()
}

fn integrated_owner_authority() -> SigningAuthorityBindingV1 {
    integrated_owner_authority_at("principal-1", "owner-key-1", 1)
}

fn integrated_owner_authority_at(
    principal: &str,
    key: &str,
    epoch: u64,
) -> SigningAuthorityBindingV1 {
    SigningAuthorityBindingV1::integrated_owner(
        ResidentPrincipalIdV1::parse(principal).unwrap(),
        ResidentKeyIdV1::parse(key).unwrap(),
        SigningKeyEpochV1::new(epoch).unwrap(),
        IntegratedOwnerAuthorityV1 {
            owner_component_id: OwnerComponentIdV1::parse("owner-component").unwrap(),
            owner_record_signing_authority_ref: record_ref("owner-signing-authority"),
            owner_record_signing_authority_sha256: artifact_digest('a'),
            owner_contract_manifest_ref: record_ref("owner-contract-manifest"),
            owner_contract_manifest_sha256: artifact_digest('b'),
            owner_signer_record_sequence: OwnerSignerRecordSequenceV1::new(1).unwrap(),
            previous_owner_record_sha256: None,
            checked_principal_trust_checkpoint_ref: record_ref("owner-trust-checkpoint"),
            checked_principal_trust_checkpoint_sha256: artifact_digest('c'),
        },
    )
    .unwrap()
}

pub(in crate::resident) fn producer() -> ProducerIncarnationV1 {
    crate::producer_custody_fixture::InitialProducerFixture::new()
        .0
        .allocation
}

fn producer_at(identity_fill: char, allocated_at: u64) -> ProducerIncarnationV1 {
    let fixture = crate::producer_custody_fixture::InitialProducerFixture::new().0;
    let atom = fixture.fixture.admitted.atom_view();
    let authorities = atom.authorities();
    let preimage = producer_preimage(
        authorities.producer_authority_ref(),
        authorities.producer_authority(),
        identity_fill,
        allocated_at,
    );
    admit_allocation(
        &preimage,
        authorities.producer_authority_ref(),
        authorities.producer_authority(),
        &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
    )
}

pub(in crate::resident) fn writer() -> PrincipalResidentWriterFenceV1 {
    writer_on("node-1", "process-1", 10)
}

fn writer_on(node: &str, process: &str, acquired_at: u64) -> PrincipalResidentWriterFenceV1 {
    let schema =
        pale_contracts::record_schema_identity(PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA_ID)
            .unwrap();
    let preimage = PrincipalResidentWriterFencePreimageV1 {
        schema_id: schema.schema,
        schema_sha256: schema.schema_sha256,
        writer_fence_id: PrincipalResidentWriterFenceIdV1::parse("writer-1").unwrap(),
        principal_id: ResidentPrincipalIdV1::parse("principal-1").unwrap(),
        writer_generation: PrincipalResidentWriterGenerationV1::new(1).unwrap(),
        previous_writer_fence_ref: None,
        previous_writer_fence_sha256: None,
        prior_terminal_receipt_ref: None,
        prior_terminal_receipt_sha256: None,
        takeover_receipt_ref: None,
        takeover_receipt_sha256: None,
        primary_node_id: ResidentNodeIdV1::parse(node).unwrap(),
        process_instance_id: ResidentProcessInstanceIdV1::parse(process).unwrap(),
        boot_incarnation_id: ResidentBootIncarnationIdV1::parse("boot-1").unwrap(),
        trust_store_id: ControlStoreIdV1::parse("trust-store").unwrap(),
        trust_store_incarnation_id: ControlIncarnationIdV1::parse("trust-1").unwrap(),
        opened_deployment_store_heads_root_sha256: artifact_digest('6'),
        principal_session_floor_epoch: PrincipalSessionFloorEpochV1::new(1).unwrap(),
        state: PrincipalResidentWriterFenceStateV1::Active,
        acquired_at: UnixTimeV1::new(acquired_at).unwrap(),
        retired_at: None,
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").unwrap(),
        issuer_key_id: ResidentKeyIdV1::parse("principal-key-1").unwrap(),
        issuer_key_epoch: PrincipalKeyEpochV1::new(1).unwrap(),
    };
    let digest = PrincipalResidentWriterFenceDigest::hash_canonical(&preimage).unwrap();
    let wire = crate::principal_trust_test_support::signed_wire(
        &preimage,
        "canonical_principal_resident_writer_fence_sha256",
        &digest,
    );
    let principal = ResidentPrincipalIdV1::parse("principal-1").unwrap();
    let key = ResidentKeyIdV1::parse("principal-key-1").unwrap();
    PrincipalResidentWriterFenceV1::admit_initial_active_wire(
        wire,
        &PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated,
        &preimage.schema_sha256,
        &PrincipalKeyAdmissionContextV1::new(
            &principal,
            &key,
            PrincipalKeyEpochV1::new(1).unwrap(),
        ),
        |_, _, _, _| Ok(()),
    )
    .unwrap()
}

fn control_record<R: Serialize + record_projection::ResidentRecordSchema>(
    reference: &ResidentRecordRefV1,
    digest: &str,
    value: &R,
) -> pale_ports::ControlRecord {
    record_projection::record(
        record_projection::coordinate(reference, digest).unwrap(),
        value,
    )
    .unwrap()
}

pub(in crate::resident) fn source(
    observation: (&ResidentRecordRefV1, &OperationObservationV1),
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
) -> OperationPublicationSource {
    OperationPublicationSource::new(
        *checkpoint.1.committed_transaction_sequence(),
        control_record(
            observation.0,
            observation
                .1
                .canonical_operation_observation_sha256()
                .as_str(),
            observation.1,
        ),
        control_record(
            checkpoint.0,
            checkpoint.1.canonical_control_checkpoint_sha256().as_str(),
            checkpoint.1,
        ),
    )
    .unwrap()
}

pub(in crate::resident) struct ProjectionFixture {
    scope: EventScopeV1,
    classification: DataClassificationV1,
    visibility: InteractionVisibilityV1,
    disclosure: InteractionDisclosureCeilingV1,
    payload_policy: PayloadResolutionPolicyV1,
    actor: OwnerRecordCoordinateV1,
    authority: SigningAuthorityBindingV1,
    occurrence_schema: pale_core::records::RecordSchemaIdentity,
    edge_schema: pale_core::records::RecordSchemaIdentity,
    frontier_schema: pale_core::records::RecordSchemaIdentity,
}

impl ProjectionFixture {
    pub(in crate::resident) fn new() -> Self {
        Self {
            scope: EventScopeV1 {
                principal_scope: ResidentPrincipalIdV1::parse("principal-1").unwrap(),
                controller_scope: ResidentControllerIdV1::parse("controller-1").unwrap(),
                relationship_scope: None,
                environment: DeploymentEnvironmentV1::parse("test").unwrap(),
            },
            classification: DataClassificationV1::parse("confidential").unwrap(),
            visibility: InteractionVisibilityV1::parse("private").unwrap(),
            disclosure: InteractionDisclosureCeilingV1::parse("private").unwrap(),
            payload_policy: PayloadResolutionPolicyV1::parse("reference-only").unwrap(),
            actor: OwnerRecordCoordinateV1 {
                owner_system_id: OwnerSystemIdV1::parse("pale").unwrap(),
                record_ref: record_ref("actor-1"),
                record_schema_id: SchemaId::parse("pale/actor.v1").unwrap(),
                record_schema_sha256: SchemaDigest::parse("4".repeat(64)).unwrap(),
                record_content_sha256: artifact_digest('5'),
                native_revision: NativeRevisionV1 {
                    native_revision_owner: NativeRevisionOwnerV1::parse("pale").unwrap(),
                    native_revision_token: NativeRevisionTokenV1::parse("actor-revision-1")
                        .unwrap(),
                },
            },
            authority: signing_authority(),
            occurrence_schema: pale_contracts::record_schema_identity(
                INTERACTION_OCCURRENCE_V1_SCHEMA_ID,
            )
            .unwrap(),
            edge_schema: pale_contracts::record_schema_identity(GENEALOGY_EDGE_V1_SCHEMA_ID)
                .unwrap(),
            frontier_schema: pale_contracts::record_schema_identity(
                RECONCILIATION_FRONTIER_V1_SCHEMA_ID,
            )
            .unwrap(),
        }
    }

    fn policy(&self) -> LocalOperationPublicationPolicy {
        LocalOperationPublicationPolicy {
            target_scope: self.scope.clone(),
            classification: self.classification.clone(),
            visibility: self.visibility.clone(),
            disclosure_ceiling: self.disclosure.clone(),
            payload_resolution_policy: self.payload_policy.clone(),
            actor_provenance: self.actor.clone(),
        }
    }

    pub(in crate::resident) fn contexts(&self) -> LocalOperationPublicationSigningContexts<'_> {
        self.contexts_with(&self.authority)
    }

    pub(in crate::resident) fn contexts_with<'a>(
        &'a self,
        authority: &'a SigningAuthorityBindingV1,
    ) -> LocalOperationPublicationSigningContexts<'a> {
        LocalOperationPublicationSigningContexts {
            occurrence: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &self.occurrence_schema.schema_sha256,
                signing_authority: authority,
            },
            edge: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &self.edge_schema.schema_sha256,
                signing_authority: authority,
            },
            frontier: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &self.frontier_schema.schema_sha256,
                signing_authority: authority,
            },
        }
    }
}

fn references(
    event: &ResidentRecordRefV1,
    occurrence: &ResidentRecordRefV1,
    edge: Option<&ResidentRecordRefV1>,
    frontier: &ResidentRecordRefV1,
) -> LocalOperationPublicationReferences {
    LocalOperationPublicationReferences {
        event: event.clone(),
        occurrence: occurrence.clone(),
        occurrence_retention: record_ref("occurrence-retention"),
        edge: edge.cloned(),
        edge_retention: edge.map(|_| record_ref("edge-retention")),
        frontier: frontier.clone(),
        frontier_retention: record_ref("frontier-retention"),
    }
}

#[allow(clippy::too_many_arguments)]
fn advance_with_authorities(
    resident: &mut ResidentApplication<'_>,
    store: &mut PublicationStore<'_>,
    plan: &AdmittedLocalOperationPublicationPlan,
    producer: &ProducerIncarnationV1,
    writer: &PrincipalResidentWriterFenceV1,
    config: &ProjectionFixture,
    current: &SigningAuthorityBindingV1,
    historical_occurrence: &SigningAuthorityBindingV1,
    historical_frontier: &SigningAuthorityBindingV1,
    signed: &Cell<usize>,
) -> LocalOperationPublicationAdvance {
    try_advance_with_authorities(
        resident,
        store,
        plan,
        producer,
        writer,
        config,
        current,
        historical_occurrence,
        historical_occurrence,
        historical_frontier,
        signed,
        false,
    )
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn try_advance_with_authorities(
    resident: &mut ResidentApplication<'_>,
    store: &mut PublicationStore<'_>,
    plan: &AdmittedLocalOperationPublicationPlan,
    producer: &ProducerIncarnationV1,
    writer: &PrincipalResidentWriterFenceV1,
    config: &ProjectionFixture,
    current: &SigningAuthorityBindingV1,
    historical_occurrence: &SigningAuthorityBindingV1,
    historical_edge: &SigningAuthorityBindingV1,
    historical_frontier: &SigningAuthorityBindingV1,
    signed: &Cell<usize>,
    reject_occurrence_signature: bool,
) -> Result<LocalOperationPublicationAdvance, super::super::LocalOperationPublicationAdvanceError> {
    let contexts = LocalOperationPublicationSigningContexts {
        occurrence: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &config.occurrence_schema.schema_sha256,
            signing_authority: current,
        },
        edge: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &config.edge_schema.schema_sha256,
            signing_authority: current,
        },
        frontier: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &config.frontier_schema.schema_sha256,
            signing_authority: current,
        },
    };
    let mut sign_occurrence =
        |_: &[u8], _: &InteractionOccurrencePreimageV1, _: &InteractionOccurrenceDigest| {
            signed.set(signed.get() + 1);
            Ok(signature())
        };
    let mut sign_edge = |_: &[u8], _: &GenealogyEdgePreimageV1, _: &GenealogyEdgeDigest| {
        signed.set(signed.get() + 1);
        Ok(signature())
    };
    let mut sign_frontier =
        |_: &[u8], _: &ReconciliationFrontierPreimageV1, _: &ReconciliationFrontierDigest| {
            signed.set(signed.get() + 1);
            Ok(signature())
        };
    let mut issuance = LocalOperationPublicationIssuance {
        sign_occurrence: &mut sign_occurrence,
        sign_edge: &mut sign_edge,
        sign_frontier: &mut sign_frontier,
    };
    let mut resolve_occurrence = |_: &ControlRecord| Ok(historical_occurrence.clone());
    let mut resolve_edge = |_: &ControlRecord| Ok(historical_edge.clone());
    let mut resolve_frontier = |_: &ControlRecord| Ok(historical_frontier.clone());
    let mut admit_event_source = |_: &EventReferencePreimageV1| Ok(());
    let mut verify_occurrence = |_: &[u8],
                                 _: &InteractionOccurrencePreimageV1,
                                 _: &InteractionOccurrenceDigest,
                                 _: &CanonicalSignatureV1| {
        if reject_occurrence_signature {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "interaction occurrence",
                invariant: "test rejected historical signature",
            })
        } else {
            Ok(())
        }
    };
    let mut verify_edge = |_: &[u8],
                           _: &GenealogyEdgePreimageV1,
                           _: &GenealogyEdgeDigest,
                           _: &CanonicalSignatureV1| { Ok(()) };
    let mut verify_frontier = |_: &[u8],
                               _: &ReconciliationFrontierPreimageV1,
                               _: &ReconciliationFrontierDigest,
                               _: &CanonicalSignatureV1| { Ok(()) };
    let mut readmission = LocalOperationPublicationReadmission {
        admit_event_source: &mut admit_event_source,
        resolve_occurrence_authority: &mut resolve_occurrence,
        verify_occurrence_signature: &mut verify_occurrence,
        resolve_edge_authority: &mut resolve_edge,
        verify_edge_signature: &mut verify_edge,
        resolve_frontier_authority: &mut resolve_frontier,
        verify_frontier_signature: &mut verify_frontier,
    };
    let verifier = |_: &super::super::EventPublicationRaceClosure<'_>| Ok(());
    advance_stage_lazy(
        resident,
        store,
        plan,
        &record_ref("producer-allocation-1"),
        producer,
        &record_ref("writer-fence-current"),
        writer,
        super::super::EventPublicationState::Active,
        &verifier,
        contexts,
        &mut issuance,
        &mut readmission,
        32_768,
    )
}

#[test]
fn restart_readmits_historical_stage_before_signing_with_rotated_key() {
    let fixture = OperationFixture::exact();
    let checkpoint_ref = record_ref("publication-checkpoint-1");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let publication_source = source(fixture.predecessor(), (&checkpoint_ref, &checkpoint));
    let config = ProjectionFixture::new();
    let policy = config.policy();
    let references = LocalOperationPublicationReferences::derive(
        &store_coordinate(),
        fixture.predecessor().0,
        fixture.predecessor().1,
    )
    .unwrap();
    let original_writer = writer_on("node-1", "process-1", 10);
    let plan = admit_local_operation_publication_plan_from_parts(
        &store_coordinate(),
        &publication_source,
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        (&checkpoint_ref, &checkpoint),
        &policy,
        &references,
        None,
    )
    .unwrap();
    let authority_store = MemoryAuthorityStore::default();
    let mut resident = empty_application(&authority_store, RingCursorV1::new(0));
    let mut publication_store = PublicationStore::new(&authority_store);
    let producer_one = producer_at('a', 50);
    let key_one = signing_authority_at("key-1", 1);
    let first_signatures = Cell::new(0);

    for expected in [
        LocalOperationPublicationStage::Event,
        LocalOperationPublicationStage::Occurrence,
    ] {
        let result = advance_with_authorities(
            &mut resident,
            &mut publication_store,
            &plan,
            &producer_one,
            &original_writer,
            &config,
            &key_one,
            &key_one,
            &key_one,
            &first_signatures,
        );
        assert!(matches!(
            result,
            LocalOperationPublicationAdvance::Established { stage, .. } if stage == expected
        ));
    }
    assert_eq!(first_signatures.get(), 1, "the Event is not signed");

    let producer_two = producer_at('b', 80);
    let takeover_writer = writer_on("node-2", "process-2", 99);
    let restarted_plan = admit_local_operation_publication_plan_from_parts(
        &store_coordinate(),
        &publication_source,
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        (&checkpoint_ref, &checkpoint),
        &policy,
        &references,
        None,
    )
    .unwrap();
    let key_two = signing_authority_at("key-2", 2);
    let rotated_signatures = Cell::new(0);
    let result = advance_with_authorities(
        &mut resident,
        &mut publication_store,
        &restarted_plan,
        &producer_two,
        &takeover_writer,
        &config,
        &key_two,
        &key_one,
        &key_two,
        &rotated_signatures,
    );
    assert!(matches!(
        result,
        LocalOperationPublicationAdvance::Established {
            stage: LocalOperationPublicationStage::ReconciliationFrontier,
            ..
        }
    ));
    assert_eq!(rotated_signatures.get(), 1);
    let occurrence_record = publication_store
        .records
        .values()
        .find(|record| record.schema_id().as_str() == INTERACTION_OCCURRENCE_V1_SCHEMA)
        .expect("occurrence is retained");
    let frontier_record = publication_store
        .records
        .values()
        .find(|record| record.schema_id().as_str() == RECONCILIATION_FRONTIER_V1_SCHEMA)
        .expect("frontier is retained");
    let frontier_wire: ReconciliationFrontierWireV1 = serde_json::from_value(
        canon::parse_strict(frontier_record.canonical_record_jcs().as_slice()).unwrap(),
    )
    .unwrap();
    let frontier_context = SignedResidentAdmissionContextV1 {
        expected_schema_sha256: &config.frontier_schema.schema_sha256,
        signing_authority: &key_two,
    };
    let frontier = ReconciliationFrontierV1::admit_contextual_wire(
        frontier_wire,
        &frontier_context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .unwrap();
    assert_eq!(
        frontier.source_occurrence_refs()[0].record_sha256.as_str(),
        sha256_hex(occurrence_record.canonical_record_jcs().as_slice()),
        "the rotated frontier binds the exact retained historical occurrence bytes"
    );

    let key_three = signing_authority_at("key-3", 3);
    let producer_three = producer_at('c', 120);
    let writer_three = writer_on("node-3", "process-3", 130);
    let completion_signatures = Cell::new(0);
    let result = advance_with_authorities(
        &mut resident,
        &mut publication_store,
        &restarted_plan,
        &producer_three,
        &writer_three,
        &config,
        &key_three,
        &key_one,
        &key_two,
        &completion_signatures,
    );
    assert!(matches!(result, LocalOperationPublicationAdvance::Complete));
    assert_eq!(completion_signatures.get(), 0);
    assert_eq!(resident.journal_head().unwrap().get(), 3);
    let event_record = publication_store
        .records
        .values()
        .find(|record| record.schema_id().as_str() == EVENT_REFERENCE_V1_SCHEMA)
        .expect("the original Event remains retained");
    let event_wire: EventReferenceWireV1 = serde_json::from_value(
        canon::parse_strict(event_record.canonical_record_jcs().as_slice()).unwrap(),
    )
    .unwrap();
    let event_schema =
        pale_contracts::record_schema_identity(EVENT_REFERENCE_V1_SCHEMA_ID).expect("Event schema");
    let event = EventReferenceV1::admit_contextual_wire(
        event_wire,
        &event_schema.schema_sha256,
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(event.producer_epoch(), producer_one.producer_epoch());
    assert_eq!(event.origin_node_id().as_str(), "node-1");
}

#[test]
fn retained_owner_source_rejects_policy_drift_before_signing() {
    let fixture = OperationFixture::exact();
    let checkpoint_ref = record_ref("publication-checkpoint-1");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let publication_source = source(fixture.predecessor(), (&checkpoint_ref, &checkpoint));
    let config = ProjectionFixture::new();
    let references = LocalOperationPublicationReferences::derive(
        &store_coordinate(),
        fixture.predecessor().0,
        fixture.predecessor().1,
    )
    .unwrap();
    let original_plan = admit_local_operation_publication_plan_from_parts(
        &store_coordinate(),
        &publication_source,
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        (&checkpoint_ref, &checkpoint),
        &config.policy(),
        &references,
        None,
    )
    .unwrap();
    let mut changed_policy = config.policy();
    changed_policy.classification = DataClassificationV1::parse("restricted").unwrap();
    let conflicting_plan = admit_local_operation_publication_plan_from_parts(
        &store_coordinate(),
        &publication_source,
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        (&checkpoint_ref, &checkpoint),
        &changed_policy,
        &references,
        None,
    )
    .unwrap();
    let authority_store = MemoryAuthorityStore::default();
    let mut resident = empty_application(&authority_store, RingCursorV1::new(0));
    let mut publication_store = PublicationStore::new(&authority_store);
    let current_producer = producer_at('a', 50);
    let current_writer = writer();
    let key = signing_authority();
    let signed = Cell::new(0);
    assert!(matches!(
        advance_with_authorities(
            &mut resident,
            &mut publication_store,
            &original_plan,
            &current_producer,
            &current_writer,
            &config,
            &key,
            &key,
            &key,
            &signed,
        ),
        LocalOperationPublicationAdvance::Established {
            stage: LocalOperationPublicationStage::Event,
            ..
        }
    ));

    let result = try_advance_with_authorities(
        &mut resident,
        &mut publication_store,
        &conflicting_plan,
        &producer_at('b', 80),
        &writer_on("node-2", "process-2", 90),
        &config,
        &key,
        &key,
        &key,
        &key,
        &signed,
        false,
    );
    assert!(matches!(
        result,
        Err(super::super::LocalOperationPublicationAdvanceError::Conflict)
    ));
    assert_eq!(signed.get(), 0);
    assert_eq!(resident.journal_head().unwrap().get(), 1);
}

#[test]
fn retained_stage_rejects_wrong_authority_kind_principal_and_signature() {
    let fixture = OperationFixture::exact();
    let checkpoint_ref = record_ref("publication-checkpoint-1");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let publication_source = source(fixture.predecessor(), (&checkpoint_ref, &checkpoint));
    let config = ProjectionFixture::new();
    let policy = config.policy();
    let references = LocalOperationPublicationReferences::derive(
        &store_coordinate(),
        fixture.predecessor().0,
        fixture.predecessor().1,
    )
    .unwrap();
    let producer_one = producer_at('a', 50);
    let writer_one = writer();
    let plan = admit_local_operation_publication_plan_from_parts(
        &store_coordinate(),
        &publication_source,
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        (&checkpoint_ref, &checkpoint),
        &policy,
        &references,
        None,
    )
    .unwrap();
    let authority_store = MemoryAuthorityStore::default();
    let mut resident = empty_application(&authority_store, RingCursorV1::new(0));
    let mut publication_store = PublicationStore::new(&authority_store);
    let key_one = signing_authority_at("key-1", 1);
    let key_two = signing_authority_at("key-2", 2);
    let signed = Cell::new(0);
    for expected in [
        LocalOperationPublicationStage::Event,
        LocalOperationPublicationStage::Occurrence,
    ] {
        assert!(matches!(
            advance_with_authorities(
                &mut resident,
                &mut publication_store,
                &plan,
                &producer_one,
                &writer_one,
                &config,
                &key_one,
                &key_one,
                &key_one,
                &signed,
            ),
            LocalOperationPublicationAdvance::Established { stage, .. } if stage == expected
        ));
    }

    let owner_authority = integrated_owner_authority_at("principal-1", "key-1", 1);
    let result = try_advance_with_authorities(
        &mut resident,
        &mut publication_store,
        &plan,
        &producer_one,
        &writer_one,
        &config,
        &key_two,
        &owner_authority,
        &owner_authority,
        &key_two,
        &signed,
        false,
    );
    assert!(matches!(
        result,
        Err(
            super::super::LocalOperationPublicationAdvanceError::Projection(
                LocalOperationPublicationProjectionError::Association
            )
        )
    ));

    let other_runtime = signing_authority_for_runtime(
        "principal-1",
        "key-1",
        1,
        "other-runtime-authority",
        artifact_digest('9'),
    );
    let result = try_advance_with_authorities(
        &mut resident,
        &mut publication_store,
        &plan,
        &producer_one,
        &writer_one,
        &config,
        &key_two,
        &other_runtime,
        &other_runtime,
        &key_two,
        &signed,
        false,
    );
    assert!(matches!(
        result,
        Err(
            super::super::LocalOperationPublicationAdvanceError::Projection(
                LocalOperationPublicationProjectionError::Association
            )
        )
    ));

    let other_principal = signing_authority_for("principal-2", "key-1", 1);
    let result = try_advance_with_authorities(
        &mut resident,
        &mut publication_store,
        &plan,
        &producer_one,
        &writer_one,
        &config,
        &key_two,
        &other_principal,
        &other_principal,
        &key_two,
        &signed,
        false,
    );
    assert!(matches!(
        result,
        Err(
            super::super::LocalOperationPublicationAdvanceError::Projection(
                LocalOperationPublicationProjectionError::Association
            )
        )
    ));

    let result = try_advance_with_authorities(
        &mut resident,
        &mut publication_store,
        &plan,
        &producer_one,
        &writer_one,
        &config,
        &key_two,
        &key_one,
        &key_one,
        &key_two,
        &signed,
        true,
    );
    assert!(matches!(
        result,
        Err(
            super::super::LocalOperationPublicationAdvanceError::Admission(
                InvalidResidentRecordV1::ContextualAdmission { .. }
            )
        )
    ));
    assert_eq!(resident.journal_head().unwrap().get(), 2);
}

#[test]
fn successor_uses_retained_occurrence_across_edge_and_frontier_key_rotation() {
    let fixture = OperationFixture::exact();
    let first_checkpoint_ref = record_ref("publication-checkpoint-1");
    let first_checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let first_source = source(
        fixture.predecessor(),
        (&first_checkpoint_ref, &first_checkpoint),
    );
    let config = ProjectionFixture::new();
    let first = project_initial(
        &fixture,
        (&first_checkpoint_ref, &first_checkpoint),
        &first_source,
        &config,
        &Cell::new(0),
        &Cell::new(0),
    );
    let authority_store = MemoryAuthorityStore::default();
    let mut resident = empty_application(&authority_store, RingCursorV1::new(0));
    let mut publication_store = PublicationStore::new(&authority_store);
    let producer_one = producer_at('a', 50);
    let writer_one = writer();
    let key_one = signing_authority_at("key-1", 1);
    let key_two = signing_authority_at("key-2", 2);
    let signed = Cell::new(0);

    for expected in [
        LocalOperationPublicationStage::Event,
        LocalOperationPublicationStage::Occurrence,
        LocalOperationPublicationStage::ReconciliationFrontier,
    ] {
        assert!(matches!(
            advance_with_authorities(
                &mut resident,
                &mut publication_store,
                &first.plan,
                &producer_one,
                &writer_one,
                &config,
                &key_one,
                &key_one,
                &key_one,
                &signed,
            ),
            LocalOperationPublicationAdvance::Established { stage, .. } if stage == expected
        ));
    }

    let second_checkpoint_ref = record_ref("publication-checkpoint-2");
    let second_checkpoint = admit_successor_checkpoint(
        &checkpoint_preimage(
            2,
            Some((
                first_checkpoint_ref.clone(),
                first_checkpoint
                    .canonical_control_checkpoint_sha256()
                    .clone(),
            )),
        ),
        &first_checkpoint_ref,
        &first_checkpoint,
    );
    let second_source = source(
        fixture.successor(),
        (&second_checkpoint_ref, &second_checkpoint),
    );
    let second_references = LocalOperationPublicationReferences::derive(
        &store_coordinate(),
        fixture.successor().0,
        fixture.successor().1,
    )
    .unwrap();
    let predecessor = admit_predecessor(
        &fixture,
        (&first_checkpoint_ref, &first_checkpoint),
        &config,
        &first,
    );
    let second_plan = admit_local_operation_publication_plan_from_parts(
        &store_coordinate(),
        &second_source,
        &store_coordinate(),
        fixture.handle(),
        fixture.successor(),
        (&second_checkpoint_ref, &second_checkpoint),
        &config.policy(),
        &second_references,
        Some(predecessor),
    )
    .unwrap();
    let producer_two = producer_at('b', 80);
    let writer_two = writer_on("node-2", "process-2", 90);

    for expected in [
        LocalOperationPublicationStage::Event,
        LocalOperationPublicationStage::Occurrence,
    ] {
        assert!(matches!(
            advance_with_authorities(
                &mut resident,
                &mut publication_store,
                &second_plan,
                &producer_two,
                &writer_two,
                &config,
                &key_one,
                &key_one,
                &key_one,
                &signed,
            ),
            LocalOperationPublicationAdvance::Established { stage, .. } if stage == expected
        ));
    }
    assert!(matches!(
        try_advance_with_authorities(
            &mut resident,
            &mut publication_store,
            &second_plan,
            &producer_two,
            &writer_two,
            &config,
            &key_two,
            &key_one,
            &key_two,
            &key_two,
            &signed,
            false,
        )
        .unwrap(),
        LocalOperationPublicationAdvance::Established {
            stage: LocalOperationPublicationStage::GenealogyEdge,
            ..
        }
    ));
    assert!(matches!(
        try_advance_with_authorities(
            &mut resident,
            &mut publication_store,
            &second_plan,
            &producer_two,
            &writer_two,
            &config,
            &key_two,
            &key_one,
            &key_two,
            &key_two,
            &signed,
            false,
        )
        .unwrap(),
        LocalOperationPublicationAdvance::Established {
            stage: LocalOperationPublicationStage::ReconciliationFrontier,
            ..
        }
    ));

    let occurrence_record = publication_store
        .records
        .values()
        .find(|record| record.coordinate().record_ref() == second_references.occurrence())
        .expect("successor occurrence");
    let edge_record = publication_store
        .records
        .values()
        .find(|record| {
            second_references
                .edge()
                .is_some_and(|reference| record.coordinate().record_ref() == reference)
        })
        .expect("successor edge");
    let frontier_record = publication_store
        .records
        .values()
        .find(|record| record.coordinate().record_ref() == second_references.frontier())
        .expect("successor frontier");
    let occurrence: InteractionOccurrenceV1 = InteractionOccurrenceV1::admit_contextual_wire(
        serde_json::from_value(
            canon::parse_strict(occurrence_record.canonical_record_jcs().as_slice()).unwrap(),
        )
        .unwrap(),
        &config.contexts().occurrence,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .unwrap();
    let edge_context = SignedResidentAdmissionContextV1 {
        expected_schema_sha256: &config.edge_schema.schema_sha256,
        signing_authority: &key_two,
    };
    let edge: GenealogyEdgeV1 = GenealogyEdgeV1::admit_contextual_wire(
        serde_json::from_value(
            canon::parse_strict(edge_record.canonical_record_jcs().as_slice()).unwrap(),
        )
        .unwrap(),
        &edge_context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .unwrap();
    assert_eq!(
        edge.destination_occurrence_sha256(),
        occurrence.canonical_occurrence_sha256()
    );
    assert_eq!(edge.issuer_key_id().as_str(), "key-2");
    let frontier_context = SignedResidentAdmissionContextV1 {
        expected_schema_sha256: &config.frontier_schema.schema_sha256,
        signing_authority: &key_two,
    };
    let frontier: ReconciliationFrontierV1 = ReconciliationFrontierV1::admit_contextual_wire(
        serde_json::from_value(
            canon::parse_strict(frontier_record.canonical_record_jcs().as_slice()).unwrap(),
        )
        .unwrap(),
        &frontier_context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .unwrap();
    assert_eq!(
        frontier.source_occurrence_refs()[0].record_sha256.as_str(),
        sha256_hex(occurrence_record.canonical_record_jcs().as_slice())
    );
    let event_schema =
        pale_contracts::record_schema_identity(EVENT_REFERENCE_V1_SCHEMA_ID).expect("Event schema");
    let first_event_record = publication_store
        .records
        .values()
        .find(|record| record.coordinate().record_ref() == first.plan.event_ref())
        .expect("predecessor Event");
    let second_event_record = publication_store
        .records
        .values()
        .find(|record| record.coordinate().record_ref() == second_plan.event_ref())
        .expect("successor Event");
    let admit_event = |record: &ControlRecord| {
        EventReferenceV1::admit_contextual_wire(
            serde_json::from_value(
                canon::parse_strict(record.canonical_record_jcs().as_slice()).unwrap(),
            )
            .unwrap(),
            &event_schema.schema_sha256,
            |_| Ok(()),
        )
        .unwrap()
    };
    let first_event = admit_event(first_event_record);
    let second_event = admit_event(second_event_record);
    assert_eq!(first_event.producer_epoch(), producer_one.producer_epoch());
    assert_eq!(second_event.producer_epoch(), producer_two.producer_epoch());
    assert_eq!(
        second_event
            .causation_id()
            .as_ref()
            .map(EventCausationIdV1::as_str),
        Some(first_event.event_id().as_str())
    );
}

struct LocalOperationPublicationProjection {
    plan: AdmittedLocalOperationPublicationPlan,
    event: (ResidentRecordRefV1, EventReferenceV1),
    occurrence: (ResidentRecordRefV1, InteractionOccurrenceV1),
    edge: Option<(ResidentRecordRefV1, GenealogyEdgeV1)>,
    frontier: (ResidentRecordRefV1, ReconciliationFrontierV1),
    occurrence_authority: SigningAuthorityBindingV1,
    edge_authority: SigningAuthorityBindingV1,
    frontier_authority: SigningAuthorityBindingV1,
}

impl LocalOperationPublicationProjection {
    fn event(&self) -> (&ResidentRecordRefV1, &EventReferenceV1) {
        (&self.event.0, &self.event.1)
    }

    fn occurrence(&self) -> (&ResidentRecordRefV1, &InteractionOccurrenceV1) {
        (&self.occurrence.0, &self.occurrence.1)
    }

    fn edge(&self) -> Option<(&ResidentRecordRefV1, &GenealogyEdgeV1)> {
        self.edge
            .as_ref()
            .map(|(reference, value)| (reference, value))
    }

    fn frontier(&self) -> (&ResidentRecordRefV1, &ReconciliationFrontierV1) {
        (&self.frontier.0, &self.frontier.1)
    }
}

#[allow(clippy::too_many_arguments)]
fn project_local_operation_publication_from_parts<EA, OS, ES, FS>(
    store: &AuthorityStoreCoordinate,
    source: &OperationPublicationSource,
    recovered_store: &AuthorityStoreCoordinate,
    operation: (&ResidentRecordRefV1, &OperationHandleV1),
    observation: (&ResidentRecordRefV1, &OperationObservationV1),
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    producer: &ProducerIncarnationV1,
    writer_fence: &PrincipalResidentWriterFenceV1,
    policy: &LocalOperationPublicationPolicy,
    references: &LocalOperationPublicationReferences,
    predecessor: Option<LocalOperationPredecessorPublication>,
    contexts: LocalOperationPublicationSigningContexts<'_>,
    admit_event_source: EA,
    sign_occurrence: OS,
    sign_edge: ES,
    sign_frontier: FS,
) -> Result<LocalOperationPublicationProjection, LocalOperationPublicationProjectionError>
where
    EA: FnOnce(&EventReferencePreimageV1) -> Result<(), InvalidResidentRecordV1>,
    OS: FnOnce(
        &[u8],
        &InteractionOccurrencePreimageV1,
        &InteractionOccurrenceDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    ES: FnOnce(
        &[u8],
        &GenealogyEdgePreimageV1,
        &GenealogyEdgeDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    FS: FnOnce(
        &[u8],
        &ReconciliationFrontierPreimageV1,
        &ReconciliationFrontierDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let plan = admit_local_operation_publication_plan_from_parts(
        store,
        source,
        recovered_store,
        operation,
        observation,
        checkpoint,
        policy,
        references,
        predecessor,
    )?;
    let event = plan.issue_event(producer, writer_fence, admit_event_source)?;
    let occurrence = plan.issue_occurrence(&event, contexts.occurrence, sign_occurrence)?;
    let edge = plan
        .issue_edge(&occurrence, contexts.edge, sign_edge)?
        .map(|value| {
            (
                references
                    .edge()
                    .expect("an issued edge has a reference")
                    .clone(),
                value,
            )
        });
    let frontier = plan.issue_frontier(&occurrence, contexts.frontier, sign_frontier)?;
    Ok(LocalOperationPublicationProjection {
        plan,
        event: (references.event().clone(), event),
        occurrence: (references.occurrence().clone(), occurrence),
        edge,
        frontier: (references.frontier().clone(), frontier),
        occurrence_authority: contexts.occurrence.signing_authority.clone(),
        edge_authority: contexts.edge.signing_authority.clone(),
        frontier_authority: contexts.frontier.signing_authority.clone(),
    })
}

#[test]
fn child_start_projects_exact_parent_placement_and_one_delegate_edge() {
    let (mut generations, profile) =
        crate::resident::operation_observation_custody::tests::local_start::
            host_source_generation_copies(1);
    let generation = generations.pop().expect("one source generation");
    crate::resident::consultation::tests::with_issued_delegate_child_named(
        &generation,
        &profile,
        "publication-lineage",
        |child,
         parent,
         parent_candidate,
         parent_checkpoint,
         _lease_custody,
         _generation,
         parents| {
            let observation_schema =
                pale_contracts::record_schema_identity(OPERATION_OBSERVATION_V1_SCHEMA_ID).unwrap();
            let admit_initial = |record: &ControlRecord, operation: &IssuedLocalOperation| {
                let wire: OperationObservationWireV1 = serde_json::from_value(
                    canon::parse_strict(record.canonical_record_jcs().as_slice()).unwrap(),
                )
                .unwrap();
                OperationObservationV1::admit_contextual_wire(
                    wire,
                    &OperationObservationAdmissionContextV1 {
                        signed: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema.schema_sha256,
                            signing_authority: parents.signing,
                        },
                        handle_ref: operation.handle().0,
                        handle: operation.handle().1,
                        predecessor: None,
                    },
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _, _| Ok(()),
                )
                .unwrap()
            };
            let parent_record = parent_candidate
                .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
                .unwrap();
            let parent_observation = admit_initial(parent_record, parent);
            let parent_observation_ref = parent_record.coordinate().record_ref().clone();
            let parent_checkpoint_ref = parent_candidate
                .checkpoint()
                .successor_record()
                .coordinate()
                .record_ref()
                .clone();
            let parent_source = source(
                (&parent_observation_ref, &parent_observation),
                (&parent_checkpoint_ref, &parent_checkpoint),
            );
            let parent_references = LocalOperationPublicationReferences::derive_for_operation(
                &store_coordinate(),
                &parent_observation_ref,
                &parent_observation,
                parent.handle().1,
            )
            .unwrap();
            assert!(parent_references.edge().is_none());
            let occurrence_schema =
                pale_contracts::record_schema_identity(INTERACTION_OCCURRENCE_V1_SCHEMA_ID)
                    .unwrap();
            let edge_schema =
                pale_contracts::record_schema_identity(GENEALOGY_EDGE_V1_SCHEMA_ID).unwrap();
            let parent_plan = admit_local_operation_publication_plan_from_parts(
                &store_coordinate(),
                &parent_source,
                &store_coordinate(),
                parent.handle(),
                (&parent_observation_ref, &parent_observation),
                (&parent_checkpoint_ref, &parent_checkpoint),
                parent.publication_policy(),
                &parent_references,
                None,
            )
            .expect("parent publication plan");
            let parent_event = parent_plan
                .issue_event(&producer(), &writer(), |_| Ok(()))
                .expect("parent Event");
            let parent_occurrence = parent_plan
                .issue_occurrence(
                    &parent_event,
                    SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &occurrence_schema.schema_sha256,
                        signing_authority: parents.signing,
                    },
                    |_, _, _| Ok(signature()),
                )
                .expect("parent occurrence");
            let parent_start = || LocalOperationPredecessorPublication {
                observation: (parent_observation_ref.clone(), parent_observation.clone()),
                event: (parent_references.event().clone(), parent_event.clone()),
                occurrence: (
                    parent_references.occurrence().clone(),
                    parent_occurrence.clone(),
                ),
            };

            let child_observation = admit_initial(&child.initial_record, &child.operation);
            let child_observation_ref = child.initial_record.coordinate().record_ref().clone();
            let child_source = source(
                (&child_observation_ref, &child_observation),
                (&child.checkpoint_ref, &child.checkpoint),
            );
            let child_references = LocalOperationPublicationReferences::derive_for_operation(
                &store_coordinate(),
                &child_observation_ref,
                &child_observation,
                child.operation.handle().1,
            )
            .unwrap();
            assert!(child_references.edge().is_some());
            let parent_coordinate =
                operation_handle_coordinate(parent.handle().0, parent.handle().1).unwrap();
            assert!(matches!(
                admit_local_operation_publication_plan_with_parent(
                    &store_coordinate(),
                    &child_source,
                    &store_coordinate(),
                    child.operation.handle(),
                    (&child_observation_ref, &child_observation),
                    (&child.checkpoint_ref, &child.checkpoint),
                    child.operation.publication_policy(),
                    &child_references,
                    OwnerRecordKnowledgeV1::Known {
                        coordinate: parent_coordinate.clone(),
                    },
                    None,
                    None,
                ),
                Err(LocalOperationPublicationProjectionError::MissingPredecessor)
            ));
            assert!(matches!(
                admit_local_operation_publication_plan_with_parent(
                    &store_coordinate(),
                    &child_source,
                    &store_coordinate(),
                    child.operation.handle(),
                    (&child_observation_ref, &child_observation),
                    (&child.checkpoint_ref, &child.checkpoint),
                    child.operation.publication_policy(),
                    &child_references,
                    OwnerRecordKnowledgeV1::Known {
                        coordinate: operation_handle_coordinate(
                            child.operation.handle().0,
                            child.operation.handle().1,
                        )
                        .unwrap(),
                    },
                    None,
                    Some(parent_start()),
                ),
                Err(LocalOperationPublicationProjectionError::Association)
            ));
            let plan = admit_local_operation_publication_plan_with_parent(
                &store_coordinate(),
                &child_source,
                &store_coordinate(),
                child.operation.handle(),
                (&child_observation_ref, &child_observation),
                (&child.checkpoint_ref, &child.checkpoint),
                child.operation.publication_policy(),
                &child_references,
                OwnerRecordKnowledgeV1::Known {
                    coordinate: parent_coordinate.clone(),
                },
                None,
                Some(parent_start()),
            )
            .unwrap();
            let event = plan
                .issue_event(&producer(), &writer(), |_| Ok(()))
                .unwrap();
            assert!(event.causation_id().is_none());
            let occurrence = plan
                .issue_occurrence(
                    &event,
                    SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &occurrence_schema.schema_sha256,
                        signing_authority: parents.signing,
                    },
                    |_, _, _| Ok(signature()),
                )
                .unwrap();
            assert_eq!(
                occurrence.parent_placement(),
                &OwnerRecordKnowledgeV1::Known {
                    coordinate: parent_coordinate
                }
            );
            let edge = plan
                .issue_edge(
                    &occurrence,
                    SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &edge_schema.schema_sha256,
                        signing_authority: parents.signing,
                    },
                    |_, _, _| Ok(signature()),
                )
                .unwrap()
                .expect("child start has one structural edge");
            assert_eq!(edge.source_occurrence_ref(), parent_references.occurrence());
            assert_eq!(
                edge.destination_occurrence_ref(),
                child_references.occurrence()
            );
            assert_eq!(
                edge.proof(),
                &GenealogyProofV1::NativeStructural {
                    relation: GenealogyRelationV1::parse("delegate").unwrap(),
                    native_owner_evidence: operation_handle_coordinate(
                        child.operation.handle().0,
                        child.operation.handle().1,
                    )
                    .unwrap(),
                }
            );

            let successor_ref = record_ref("observation/child-operation-publication-lineage-2");
            let successor = crate::local_operation_start_fixture::issue_observation(
                parents.signing,
                child.operation.handle().0,
                child.operation.handle().1,
                observation_schema.schema_sha256.clone(),
                OperationObservationRevisionV1::new(2).unwrap(),
                Some((&child_observation_ref, &child_observation)),
                child_observation.state().clone(),
                child_observation.observed_at().get() + 1,
            );
            let successor_source = source(
                (&successor_ref, &successor),
                (&child.checkpoint_ref, &child.checkpoint),
            );
            let successor_references = LocalOperationPublicationReferences::derive_for_operation(
                &store_coordinate(),
                &successor_ref,
                &successor,
                child.operation.handle().1,
            )
            .unwrap();
            let predecessor = LocalOperationPredecessorPublication {
                observation: (child_observation_ref.clone(), child_observation.clone()),
                event: (child_references.event().clone(), event.clone()),
                occurrence: (child_references.occurrence().clone(), occurrence.clone()),
            };
            let successor_plan = admit_local_operation_publication_plan_with_parent(
                &store_coordinate(),
                &successor_source,
                &store_coordinate(),
                child.operation.handle(),
                (&successor_ref, &successor),
                (&child.checkpoint_ref, &child.checkpoint),
                child.operation.publication_policy(),
                &successor_references,
                occurrence.parent_placement().clone(),
                Some(predecessor),
                None,
            )
            .unwrap();
            let successor_event = successor_plan
                .issue_event(&producer(), &writer(), |_| Ok(()))
                .unwrap();
            assert_eq!(
                successor_event
                    .causation_id()
                    .as_ref()
                    .map(EventCausationIdV1::as_str),
                Some(event.event_id().as_str())
            );
            let successor_occurrence = successor_plan
                .issue_occurrence(
                    &successor_event,
                    SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &occurrence_schema.schema_sha256,
                        signing_authority: parents.signing,
                    },
                    |_, _, _| Ok(signature()),
                )
                .unwrap();
            let successor_edge = successor_plan
                .issue_edge(
                    &successor_occurrence,
                    SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &edge_schema.schema_sha256,
                        signing_authority: parents.signing,
                    },
                    |_, _, _| Ok(signature()),
                )
                .unwrap()
                .unwrap();
            assert!(matches!(
                successor_edge.proof(),
                GenealogyProofV1::NativeStructural { relation, native_owner_evidence }
                    if relation.as_str() == "successor"
                        && native_owner_evidence.record_ref == successor_ref
            ));
        },
    );
}

#[test]
fn operation_classes_have_closed_structural_relation_tokens() {
    assert_eq!(operation_class_relation(OperationClassV1::Invoke), "invoke");
    assert_eq!(
        operation_class_relation(OperationClassV1::Consult),
        "consult"
    );
    assert_eq!(
        operation_class_relation(OperationClassV1::Delegate),
        "delegate"
    );
}

#[allow(clippy::too_many_arguments)]
fn advance_local_operation_publication(
    application: &mut ResidentApplication<'_>,
    port: &mut PublicationStore<'_>,
    references: &LocalOperationPublicationReferences,
    projection: &LocalOperationPublicationProjection,
    producer_allocation_ref: &ResidentRecordRefV1,
    producer: &ProducerIncarnationV1,
    writer_fence_ref: &ResidentRecordRefV1,
    writer_fence: &PrincipalResidentWriterFenceV1,
    publication_state: super::super::EventPublicationState<'_>,
    race_verifier: &dyn super::super::EventPublicationRaceVerifier,
    _predecessor_occurrence: Option<AdmittedEventRecord<'_, InteractionOccurrenceV1>>,
    maximum_publication_bytes: usize,
) -> Result<LocalOperationPublicationAdvance, super::super::LocalOperationPublicationAdvanceError> {
    if projection.plan.references() != references {
        return Err(super::super::LocalOperationPublicationAdvanceError::ReferenceMismatch);
    }
    let occurrence_schema =
        pale_contracts::record_schema_identity(INTERACTION_OCCURRENCE_V1_SCHEMA_ID).unwrap();
    let edge_schema = pale_contracts::record_schema_identity(GENEALOGY_EDGE_V1_SCHEMA_ID).unwrap();
    let frontier_schema =
        pale_contracts::record_schema_identity(RECONCILIATION_FRONTIER_V1_SCHEMA_ID).unwrap();
    let contexts = LocalOperationPublicationSigningContexts {
        occurrence: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &occurrence_schema.schema_sha256,
            signing_authority: &projection.occurrence_authority,
        },
        edge: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &edge_schema.schema_sha256,
            signing_authority: &projection.edge_authority,
        },
        frontier: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &frontier_schema.schema_sha256,
            signing_authority: &projection.frontier_authority,
        },
    };
    let mut sign_occurrence =
        |_: &[u8], _: &InteractionOccurrencePreimageV1, _: &InteractionOccurrenceDigest| {
            Ok(signature())
        };
    let mut sign_edge =
        |_: &[u8], _: &GenealogyEdgePreimageV1, _: &GenealogyEdgeDigest| Ok(signature());
    let mut sign_frontier =
        |_: &[u8], _: &ReconciliationFrontierPreimageV1, _: &ReconciliationFrontierDigest| {
            Ok(signature())
        };
    let mut issuance = LocalOperationPublicationIssuance {
        sign_occurrence: &mut sign_occurrence,
        sign_edge: &mut sign_edge,
        sign_frontier: &mut sign_frontier,
    };
    let mut admit_event_source = |_: &EventReferencePreimageV1| Ok(());
    let mut resolve_occurrence = |_: &ControlRecord| Ok(projection.occurrence_authority.clone());
    let mut resolve_edge = |_: &ControlRecord| Ok(projection.edge_authority.clone());
    let mut resolve_frontier = |_: &ControlRecord| Ok(projection.frontier_authority.clone());
    let mut verify_occurrence =
        |_: &[u8],
         _: &InteractionOccurrencePreimageV1,
         _: &InteractionOccurrenceDigest,
         _: &CanonicalSignatureV1| { Ok(()) };
    let mut verify_edge = |_: &[u8],
                           _: &GenealogyEdgePreimageV1,
                           _: &GenealogyEdgeDigest,
                           _: &CanonicalSignatureV1| { Ok(()) };
    let mut verify_frontier = |_: &[u8],
                               _: &ReconciliationFrontierPreimageV1,
                               _: &ReconciliationFrontierDigest,
                               _: &CanonicalSignatureV1| { Ok(()) };
    let mut readmission = LocalOperationPublicationReadmission {
        admit_event_source: &mut admit_event_source,
        resolve_occurrence_authority: &mut resolve_occurrence,
        verify_occurrence_signature: &mut verify_occurrence,
        resolve_edge_authority: &mut resolve_edge,
        verify_edge_signature: &mut verify_edge,
        resolve_frontier_authority: &mut resolve_frontier,
        verify_frontier_signature: &mut verify_frontier,
    };
    advance_stage_lazy(
        application,
        port,
        &projection.plan,
        producer_allocation_ref,
        producer,
        writer_fence_ref,
        writer_fence,
        publication_state,
        race_verifier,
        contexts,
        &mut issuance,
        &mut readmission,
        maximum_publication_bytes,
    )
}

fn project_initial(
    fixture: &OperationFixture,
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    source: &OperationPublicationSource,
    config: &ProjectionFixture,
    event_admissions: &Cell<usize>,
    signed: &Cell<usize>,
) -> LocalOperationPublicationProjection {
    let policy = config.policy();
    let references = LocalOperationPublicationReferences::derive(
        &store_coordinate(),
        fixture.predecessor().0,
        fixture.predecessor().1,
    )
    .unwrap();
    project_local_operation_publication_from_parts(
        &store_coordinate(),
        source,
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        checkpoint,
        &producer(),
        &writer(),
        &policy,
        &references,
        None,
        config.contexts(),
        |_| {
            event_admissions.set(event_admissions.get() + 1);
            Ok(())
        },
        |_, _, _| {
            signed.set(signed.get() + 1);
            Ok(signature())
        },
        |_, _, _| panic!("revision one must not sign an edge"),
        |_, _, _| {
            signed.set(signed.get() + 1);
            Ok(signature())
        },
    )
    .unwrap()
}

fn admit_predecessor(
    fixture: &OperationFixture,
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    config: &ProjectionFixture,
    projection: &LocalOperationPublicationProjection,
) -> LocalOperationPredecessorPublication {
    admit_local_operation_predecessor_publication_from_parts(
        &store_coordinate(),
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        checkpoint,
        &config.policy(),
        projection.event(),
        projection.occurrence(),
        config.contexts().occurrence,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .unwrap()
}

#[test]
fn initial_projection_preserves_exact_bytes_and_claims_only_owner_publication() {
    let fixture = OperationFixture::exact();
    let checkpoint_ref = record_ref("publication-checkpoint-1");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let publication_source = source(fixture.predecessor(), (&checkpoint_ref, &checkpoint));
    let config = ProjectionFixture::new();
    let event_admissions = Cell::new(0);
    let signed = Cell::new(0);
    let projected = project_initial(
        &fixture,
        (&checkpoint_ref, &checkpoint),
        &publication_source,
        &config,
        &event_admissions,
        &signed,
    );

    assert_eq!(event_admissions.get(), 1);
    assert_eq!(signed.get(), 2);
    assert_eq!(projected.event().1.event_type().as_str(), EVENT_TYPE);
    assert_eq!(projected.event().1.payload_ref(), &None);
    assert_ne!(
        projected.event().1.source_record_content_sha256().as_str(),
        publication_source
            .observation()
            .coordinate()
            .record_sha256()
            .as_str(),
        "the Event hashes the complete signed source bytes, not its unsigned identity"
    );
    assert_eq!(
        projected.occurrence().1.lifecycle(),
        &InteractionLifecycleProjectionV1::Unknown
    );
    assert!(projected.edge().is_none());
    assert_eq!(projected.frontier().1.dimensions().len(), 1);
    assert_eq!(
        projected.frontier().1.dimensions()[0].dimension,
        ReconciliationDimensionV1::OwnerPublication
    );
    assert_eq!(
        projected.frontier().1.dimensions()[0].status,
        ReconciliationDimensionStatusV1::Complete
    );
    let expected_projection_time = [
        *fixture.predecessor().1.observed_at(),
        *checkpoint.issued_at(),
        *producer().allocated_at(),
        *writer().acquired_at(),
    ]
    .into_iter()
    .max()
    .unwrap();
    assert_eq!(projected.event().1.recorded_at(), &expected_projection_time);
    assert_eq!(
        projected.occurrence().1.projected_at(),
        &expected_projection_time
    );
    assert_eq!(
        projected.frontier().1.generated_at(),
        &expected_projection_time
    );
    let signed_occurrence_bytes = canon::to_canonical_string(projected.occurrence().1).unwrap();
    let signed_occurrence_sha256 =
        ArtifactDigest::parse(sha256_hex(signed_occurrence_bytes.as_bytes())).unwrap();
    assert_eq!(
        projected.frontier().1.source_occurrence_refs()[0].record_sha256,
        signed_occurrence_sha256
    );
    assert_ne!(
        projected.frontier().1.source_occurrence_refs()[0]
            .record_sha256
            .as_str(),
        projected
            .occurrence()
            .1
            .canonical_occurrence_sha256()
            .as_str(),
        "the frontier references complete signed occurrence bytes"
    );
}

#[test]
fn operation_classes_and_states_share_one_body_free_projection() {
    let checkpoint_ref = record_ref("publication-matrix-checkpoint");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let config = ProjectionFixture::new();
    let immutable = |name: &str, fill: char| ImmutableRecordReferenceV1 {
        record_ref: record_ref(name),
        record_sha256: artifact_digest(fill),
    };
    let reconciliation = |name: &str, fill: char| ReconciliationFrontierReferenceV1 {
        record_ref: record_ref(name),
        record_sha256: ReconciliationFrontierDigest::parse(fill.to_string().repeat(64)).unwrap(),
    };
    let cases = vec![
        (
            "active",
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Resolving,
            },
            Vec::new(),
        ),
        (
            "cancellation-requested",
            OperationObservationStateV1::CancellationRequested {
                request: immutable("matrix/cancel-request-body", '1'),
            },
            vec!["matrix/cancel-request-body"],
        ),
        (
            "recovering",
            OperationObservationStateV1::Recovering {
                reconciliation: reconciliation("matrix/recovery-body", '2'),
            },
            vec!["matrix/recovery-body"],
        ),
        (
            "succeeded",
            OperationObservationStateV1::Succeeded {
                result: CapabilityResultReferenceV1 {
                    record_ref: record_ref("matrix/success-result-body"),
                    record_sha256: CapabilityResultDigest::parse("3".repeat(64)).unwrap(),
                },
                effect_receipt: Some(EffectReceiptReferenceV1 {
                    record_ref: record_ref("matrix/success-effect-body"),
                    record_sha256: EffectReceiptDigest::parse("4".repeat(64)).unwrap(),
                }),
            },
            vec!["matrix/success-result-body", "matrix/success-effect-body"],
        ),
        (
            "failed",
            OperationObservationStateV1::Failed {
                result: Some(CapabilityResultReferenceV1 {
                    record_ref: record_ref("matrix/failure-result-body"),
                    record_sha256: CapabilityResultDigest::parse("5".repeat(64)).unwrap(),
                }),
                diagnostic: immutable("matrix/failure-diagnostic-body", '6'),
            },
            vec![
                "matrix/failure-result-body",
                "matrix/failure-diagnostic-body",
            ],
        ),
        (
            "cancelled",
            OperationObservationStateV1::Cancelled {
                diagnostic: immutable("matrix/cancelled-body", '7'),
            },
            vec!["matrix/cancelled-body"],
        ),
        (
            "outcome-unknown",
            OperationObservationStateV1::OutcomeUnknown {
                diagnostic: immutable("matrix/unknown-diagnostic-body", '8'),
                reconciliation: reconciliation("matrix/unknown-frontier-body", '9'),
                effect_receipt: Some(EffectReceiptReferenceV1 {
                    record_ref: record_ref("matrix/unknown-effect-body"),
                    record_sha256: EffectReceiptDigest::parse("a".repeat(64)).unwrap(),
                }),
            },
            vec![
                "matrix/unknown-diagnostic-body",
                "matrix/unknown-frontier-body",
                "matrix/unknown-effect-body",
            ],
        ),
        (
            "expired",
            OperationObservationStateV1::Expired {
                diagnostic: immutable("matrix/expired-body", 'b'),
            },
            vec!["matrix/expired-body"],
        ),
    ];

    for operation_class in [
        OperationClassV1::Invoke,
        OperationClassV1::Consult,
        OperationClassV1::Delegate,
    ] {
        for (case_name, state, excluded) in &cases {
            let suffix = format!("{operation_class:?}-{case_name}").to_ascii_lowercase();
            let ((handle_ref, handle), (observation_ref, observation)) =
                operation_publication_case(&suffix, operation_class, state.clone());
            let publication_source = source(
                (&observation_ref, &observation),
                (&checkpoint_ref, &checkpoint),
            );
            let references = LocalOperationPublicationReferences::derive(
                &store_coordinate(),
                &observation_ref,
                &observation,
            )
            .unwrap();
            let projected = project_local_operation_publication_from_parts(
                &store_coordinate(),
                &publication_source,
                &store_coordinate(),
                (&handle_ref, &handle),
                (&observation_ref, &observation),
                (&checkpoint_ref, &checkpoint),
                &producer(),
                &publication_writer(),
                &config.policy(),
                &references,
                None,
                config.contexts(),
                |_| Ok(()),
                |_, _, _| Ok(signature()),
                |_, _, _| panic!("revision one has no predecessor edge"),
                |_, _, _| Ok(signature()),
            )
            .unwrap();

            assert_eq!(handle.operation_class(), &operation_class);
            assert_eq!(projected.event().1.payload_ref(), &None);
            assert_eq!(projected.event().1.payload_sha256(), &None);
            assert_eq!(
                projected.occurrence().1.lifecycle(),
                &InteractionLifecycleProjectionV1::Unknown
            );
            assert_eq!(projected.frontier().1.dimensions().len(), 1);
            assert_eq!(
                projected.frontier().1.dimensions()[0].dimension,
                ReconciliationDimensionV1::OwnerPublication
            );
            assert_eq!(
                projected.frontier().1.dimensions()[0].status,
                ReconciliationDimensionStatusV1::Complete
            );
            let projected_bytes = [
                canon::to_canonical_string(projected.event().1).unwrap(),
                canon::to_canonical_string(projected.occurrence().1).unwrap(),
                canon::to_canonical_string(projected.frontier().1).unwrap(),
            ]
            .concat();
            for inferred in ["acceptance", "accepted", "settlement", "settled"] {
                assert!(
                    !projected_bytes.contains(inferred),
                    "{operation_class:?} {case_name} inferred {inferred}"
                );
            }
            for state_body in excluded {
                assert!(
                    !projected_bytes.contains(state_body),
                    "{operation_class:?} {case_name} copied state-owned body {state_body}"
                );
            }
        }
    }
}

#[test]
fn publication_references_are_stable_and_source_specific() {
    let fixture = OperationFixture::exact();
    let store = store_coordinate();
    let first = LocalOperationPublicationReferences::derive(
        &store,
        fixture.predecessor().0,
        fixture.predecessor().1,
    )
    .unwrap();
    let replay = LocalOperationPublicationReferences::derive(
        &store,
        fixture.predecessor().0,
        fixture.predecessor().1,
    )
    .unwrap();

    assert_eq!(first.event, replay.event);
    assert_eq!(first.occurrence, replay.occurrence);
    assert_eq!(first.occurrence_retention, replay.occurrence_retention);
    assert_eq!(first.edge, replay.edge);
    assert_eq!(first.edge_retention, replay.edge_retention);
    assert_eq!(first.frontier, replay.frontier);
    assert_eq!(first.frontier_retention, replay.frontier_retention);
    assert!(first.edge.is_none());
    assert!(first.edge_retention.is_none());
    assert_ne!(first.event, first.occurrence);
    assert_ne!(first.occurrence, first.occurrence_retention);
    assert_ne!(first.event, first.frontier);
    assert_ne!(first.occurrence, first.frontier);
    assert_ne!(first.frontier, first.frontier_retention);

    let successor = LocalOperationPublicationReferences::derive(
        &store,
        fixture.successor().0,
        fixture.successor().1,
    )
    .unwrap();
    assert!(successor.edge.is_some());
    assert!(successor.edge_retention.is_some());
    assert_ne!(first.event, successor.event);

    let other_store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("other-store").unwrap(),
        ControlIncarnationIdV1::parse("other-incarnation").unwrap(),
    );
    let other = LocalOperationPublicationReferences::derive(
        &other_store,
        fixture.predecessor().0,
        fixture.predecessor().1,
    )
    .unwrap();
    assert_ne!(first.event, other.event);
    assert_ne!(first.occurrence, other.occurrence);
    assert_ne!(first.occurrence_retention, other.occurrence_retention);
    assert_ne!(first.frontier, other.frontier);
    assert_ne!(first.frontier_retention, other.frontier_retention);
}

#[test]
fn bounded_publication_commits_one_missing_stage_per_call() {
    let fixture = OperationFixture::exact();
    let checkpoint_ref = record_ref("publication-checkpoint-1");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let publication_source = source(fixture.predecessor(), (&checkpoint_ref, &checkpoint));
    let config = ProjectionFixture::new();
    let policy = config.policy();
    let references = LocalOperationPublicationReferences::derive(
        &store_coordinate(),
        fixture.predecessor().0,
        fixture.predecessor().1,
    )
    .unwrap();
    let projection = project_local_operation_publication_from_parts(
        &store_coordinate(),
        &publication_source,
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        (&checkpoint_ref, &checkpoint),
        &producer(),
        &writer(),
        &policy,
        &references,
        None,
        config.contexts(),
        |_| Ok(()),
        |_, _, _| Ok(signature()),
        |_, _, _| panic!("revision one must not sign an edge"),
        |_, _, _| Ok(signature()),
    )
    .unwrap();
    let authority = MemoryAuthorityStore::default();
    let mut resident = empty_application(&authority, RingCursorV1::new(0));
    let mut publication_store = PublicationStore::new(&authority);
    let producer = producer();
    let writer = writer();
    let producer_ref = record_ref("producer-allocation-1");
    let writer_ref = record_ref("writer-fence-1");
    let verifier = |_: &super::super::EventPublicationRaceClosure<'_>| Ok(());

    for (expected_stage, expected_sequence) in [
        (LocalOperationPublicationStage::Event, 1),
        (LocalOperationPublicationStage::Occurrence, 2),
        (LocalOperationPublicationStage::ReconciliationFrontier, 3),
    ] {
        let result = advance_local_operation_publication(
            &mut resident,
            &mut publication_store,
            &references,
            &projection,
            &producer_ref,
            &producer,
            &writer_ref,
            &writer,
            super::super::EventPublicationState::Active,
            &verifier,
            None,
            32_768,
        )
        .unwrap();
        match result {
            LocalOperationPublicationAdvance::Established { stage, receipt, .. } => {
                assert_eq!(stage, expected_stage);
                assert_eq!(receipt.journal_sequence().get(), expected_sequence);
            }
            LocalOperationPublicationAdvance::Complete => {
                panic!("a missing stage must commit before completion")
            }
        }
    }

    assert!(matches!(
        advance_local_operation_publication(
            &mut resident,
            &mut publication_store,
            &references,
            &projection,
            &producer_ref,
            &producer,
            &writer_ref,
            &writer,
            super::super::EventPublicationState::Active,
            &verifier,
            None,
            32_768,
        )
        .unwrap(),
        LocalOperationPublicationAdvance::Complete
    ));
    assert_eq!(resident.journal_head().unwrap().get(), 3);

    let successor_checkpoint_ref = record_ref("publication-checkpoint-2");
    let successor_checkpoint = admit_successor_checkpoint(
        &checkpoint_preimage(
            2,
            Some((
                checkpoint_ref.clone(),
                checkpoint.canonical_control_checkpoint_sha256().clone(),
            )),
        ),
        &checkpoint_ref,
        &checkpoint,
    );
    let successor_source = source(
        fixture.successor(),
        (&successor_checkpoint_ref, &successor_checkpoint),
    );
    let successor_references = LocalOperationPublicationReferences::derive(
        &store_coordinate(),
        fixture.successor().0,
        fixture.successor().1,
    )
    .unwrap();
    let successor_projection = project_local_operation_publication_from_parts(
        &store_coordinate(),
        &successor_source,
        &store_coordinate(),
        fixture.handle(),
        fixture.successor(),
        (&successor_checkpoint_ref, &successor_checkpoint),
        &producer,
        &writer,
        &policy,
        &successor_references,
        Some(admit_predecessor(
            &fixture,
            (&checkpoint_ref, &checkpoint),
            &config,
            &projection,
        )),
        config.contexts(),
        |_| Ok(()),
        |_, _, _| Ok(signature()),
        |_, _, _| Ok(signature()),
        |_, _, _| Ok(signature()),
    )
    .unwrap();
    let predecessor_occurrence =
        AdmittedEventRecord::new(projection.occurrence().0, projection.occurrence().1);

    for (expected_stage, expected_sequence) in [
        (LocalOperationPublicationStage::Event, 4),
        (LocalOperationPublicationStage::Occurrence, 5),
        (LocalOperationPublicationStage::GenealogyEdge, 6),
        (LocalOperationPublicationStage::ReconciliationFrontier, 7),
    ] {
        let result = advance_local_operation_publication(
            &mut resident,
            &mut publication_store,
            &successor_references,
            &successor_projection,
            &producer_ref,
            &producer,
            &writer_ref,
            &writer,
            super::super::EventPublicationState::Active,
            &verifier,
            Some(predecessor_occurrence),
            32_768,
        )
        .unwrap();
        match result {
            LocalOperationPublicationAdvance::Established { stage, receipt, .. } => {
                assert_eq!(stage, expected_stage);
                assert_eq!(receipt.journal_sequence().get(), expected_sequence);
            }
            LocalOperationPublicationAdvance::Complete => {
                panic!("a missing successor stage must commit before completion")
            }
        }
    }
    assert!(matches!(
        advance_local_operation_publication(
            &mut resident,
            &mut publication_store,
            &successor_references,
            &successor_projection,
            &producer_ref,
            &producer,
            &writer_ref,
            &writer,
            super::super::EventPublicationState::Active,
            &verifier,
            Some(predecessor_occurrence),
            32_768,
        )
        .unwrap(),
        LocalOperationPublicationAdvance::Complete
    ));
    assert_eq!(resident.journal_head().unwrap().get(), 7);
}

#[test]
fn each_projection_stage_accepts_its_current_runtime_key() {
    let fixture = OperationFixture::exact();
    let checkpoint_ref = record_ref("publication-checkpoint-1");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let publication_source = source(fixture.predecessor(), (&checkpoint_ref, &checkpoint));
    let config = ProjectionFixture::new();
    let occurrence_authority = signing_authority_at("occurrence-key-2", 2);
    let edge_authority = signing_authority_at("edge-key-3", 3);
    let frontier_authority = signing_authority_at("frontier-key-4", 4);
    let contexts = LocalOperationPublicationSigningContexts {
        occurrence: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &config.occurrence_schema.schema_sha256,
            signing_authority: &occurrence_authority,
        },
        edge: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &config.edge_schema.schema_sha256,
            signing_authority: &edge_authority,
        },
        frontier: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &config.frontier_schema.schema_sha256,
            signing_authority: &frontier_authority,
        },
    };
    let projected = project_local_operation_publication_from_parts(
        &store_coordinate(),
        &publication_source,
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        (&checkpoint_ref, &checkpoint),
        &producer(),
        &writer(),
        &config.policy(),
        &references(
            &record_ref("event-1"),
            &record_ref("occurrence-1"),
            None,
            &record_ref("frontier-1"),
        ),
        None,
        contexts,
        |_| Ok(()),
        |_, _, _| Ok(signature()),
        |_, _, _| panic!("revision one must not sign an edge"),
        |_, _, _| Ok(signature()),
    )
    .unwrap();

    assert_eq!(
        projected.occurrence().1.issuer_key_id().as_str(),
        "occurrence-key-2"
    );
    assert_eq!(
        projected.frontier().1.issuer_key_id().as_str(),
        "frontier-key-4"
    );
}

#[test]
fn exact_join_rejects_substitution_before_any_signing_callback() {
    let fixture = OperationFixture::exact();
    let checkpoint_ref = record_ref("publication-checkpoint-1");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let exact_source = source(fixture.predecessor(), (&checkpoint_ref, &checkpoint));
    let changed_source = OperationPublicationSource::new(
        ControlTransactionSequenceV1::new(1).unwrap(),
        pale_ports::ControlRecord::new(
            pale_ports::ControlRecordCoordinate::new(
                record_ref("substituted-observation"),
                exact_source
                    .observation()
                    .coordinate()
                    .record_sha256()
                    .clone(),
            ),
            exact_source.observation().schema_id().clone(),
            exact_source.observation().schema_sha256().clone(),
            exact_source.observation().canonical_record_jcs().clone(),
        ),
        exact_source.checkpoint().clone(),
    )
    .unwrap();
    let config = ProjectionFixture::new();
    let policy = config.policy();
    let event_admissions = Cell::new(0);
    let signed = Cell::new(0);
    let result = project_local_operation_publication_from_parts(
        &store_coordinate(),
        &changed_source,
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        (&checkpoint_ref, &checkpoint),
        &producer(),
        &writer(),
        &policy,
        &references(
            &record_ref("event-1"),
            &record_ref("occurrence-1"),
            None,
            &record_ref("frontier-1"),
        ),
        None,
        config.contexts(),
        |_| {
            event_admissions.set(event_admissions.get() + 1);
            Ok(())
        },
        |_, _, _| {
            signed.set(signed.get() + 1);
            Ok(signature())
        },
        |_, _, _| {
            signed.set(signed.get() + 1);
            Ok(signature())
        },
        |_, _, _| {
            signed.set(signed.get() + 1);
            Ok(signature())
        },
    );
    assert!(matches!(
        result,
        Err(LocalOperationPublicationProjectionError::Association)
    ));
    assert_eq!(event_admissions.get(), 0);
    assert_eq!(signed.get(), 0);
}

#[test]
fn successor_edge_requires_and_binds_the_exact_predecessor_publication() {
    let fixture = OperationFixture::exact();
    let first_checkpoint_ref = record_ref("publication-checkpoint-1");
    let first_checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let first_source = source(
        fixture.predecessor(),
        (&first_checkpoint_ref, &first_checkpoint),
    );
    let config = ProjectionFixture::new();
    let first = project_initial(
        &fixture,
        (&first_checkpoint_ref, &first_checkpoint),
        &first_source,
        &config,
        &Cell::new(0),
        &Cell::new(0),
    );
    let second_checkpoint_ref = record_ref("publication-checkpoint-2");
    let second_preimage = checkpoint_preimage(
        2,
        Some((
            first_checkpoint_ref.clone(),
            first_checkpoint
                .canonical_control_checkpoint_sha256()
                .clone(),
        )),
    );
    let second_checkpoint =
        admit_successor_checkpoint(&second_preimage, &first_checkpoint_ref, &first_checkpoint);
    let second_source = source(
        fixture.successor(),
        (&second_checkpoint_ref, &second_checkpoint),
    );
    let event_ref = record_ref("event-2");
    let occurrence_ref = record_ref("occurrence-2");
    let edge_ref = record_ref("edge-2");
    let frontier_ref = record_ref("frontier-2");
    let first_occurrence_ref = first.occurrence().0.clone();
    let policy = config.policy();
    let projected = project_local_operation_publication_from_parts(
        &store_coordinate(),
        &second_source,
        &store_coordinate(),
        fixture.handle(),
        fixture.successor(),
        (&second_checkpoint_ref, &second_checkpoint),
        &producer(),
        &writer(),
        &policy,
        &references(&event_ref, &occurrence_ref, Some(&edge_ref), &frontier_ref),
        Some(admit_predecessor(
            &fixture,
            (&first_checkpoint_ref, &first_checkpoint),
            &config,
            &first,
        )),
        config.contexts(),
        |_| Ok(()),
        |_, _, _| Ok(signature()),
        |_, _, _| Ok(signature()),
        |_, _, _| Ok(signature()),
    )
    .unwrap();

    let edge = projected.edge().expect("successor edge");
    assert_eq!(edge.1.source_occurrence_ref(), &first_occurrence_ref);
    assert_eq!(edge.1.destination_occurrence_ref(), &occurrence_ref);
    assert!(matches!(
        edge.1.proof(),
        GenealogyProofV1::NativeStructural { relation, .. }
            if relation.as_str() == SUCCESSOR_RELATION
    ));
    assert_eq!(
        projected
            .event()
            .1
            .causation_id()
            .as_ref()
            .map(EventCausationIdV1::as_str),
        Some(first.event().1.event_id().as_str())
    );
}

#[test]
fn predecessor_admission_rejects_policy_tampering_and_invalid_signature() {
    let fixture = OperationFixture::exact();
    let checkpoint_ref = record_ref("publication-checkpoint-1");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let first_source = source(fixture.predecessor(), (&checkpoint_ref, &checkpoint));
    let config = ProjectionFixture::new();
    let first = project_initial(
        &fixture,
        (&checkpoint_ref, &checkpoint),
        &first_source,
        &config,
        &Cell::new(0),
        &Cell::new(0),
    );
    let mut changed_policy = config.policy();
    changed_policy.visibility = InteractionVisibilityV1::parse("public").unwrap();
    let result = admit_local_operation_predecessor_publication_from_parts(
        &store_coordinate(),
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        (&checkpoint_ref, &checkpoint),
        &changed_policy,
        first.event(),
        first.occurrence(),
        config.contexts().occurrence,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    );
    assert!(matches!(
        result,
        Err(LocalOperationPublicationProjectionError::Admission(
            InvalidResidentRecordV1::ContextualAdmission { .. }
        ))
    ));

    let result = admit_local_operation_predecessor_publication_from_parts(
        &store_coordinate(),
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        (&checkpoint_ref, &checkpoint),
        &config.policy(),
        first.event(),
        first.occurrence(),
        config.contexts().occurrence,
        |_| Ok(()),
        |_, _, _, _| {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "interaction occurrence",
                invariant: "test rejected predecessor signature",
            })
        },
    );
    assert!(matches!(
        result,
        Err(LocalOperationPublicationProjectionError::Admission(
            InvalidResidentRecordV1::ContextualAdmission { .. }
        ))
    ));
}

#[test]
fn known_reference_collision_is_rejected_before_callbacks() {
    let fixture = OperationFixture::exact();
    let checkpoint_ref = record_ref("publication-checkpoint-1");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let publication_source = source(fixture.predecessor(), (&checkpoint_ref, &checkpoint));
    let config = ProjectionFixture::new();
    let policy = config.policy();
    let callbacks = Cell::new(0);
    let result = project_local_operation_publication_from_parts(
        &store_coordinate(),
        &publication_source,
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        (&checkpoint_ref, &checkpoint),
        &producer(),
        &writer(),
        &policy,
        &references(
            fixture.handle().0,
            &record_ref("occurrence-1"),
            None,
            &record_ref("frontier-1"),
        ),
        None,
        config.contexts(),
        |_| {
            callbacks.set(callbacks.get() + 1);
            Ok(())
        },
        |_, _, _| {
            callbacks.set(callbacks.get() + 1);
            Ok(signature())
        },
        |_, _, _| panic!("revision one must not sign an edge"),
        |_, _, _| {
            callbacks.set(callbacks.get() + 1);
            Ok(signature())
        },
    );

    assert!(matches!(
        result,
        Err(LocalOperationPublicationProjectionError::Association)
    ));
    assert_eq!(callbacks.get(), 0);
}

#[test]
fn current_runtime_key_may_differ_from_the_historical_observation_key() {
    let fixture = OperationFixture::exact();
    let checkpoint_ref = record_ref("publication-checkpoint-1");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let publication_source = source(fixture.predecessor(), (&checkpoint_ref, &checkpoint));
    let mut config = ProjectionFixture::new();
    config.authority = signing_authority_at("current-key-2", 2);
    let projected = project_initial(
        &fixture,
        (&checkpoint_ref, &checkpoint),
        &publication_source,
        &config,
        &Cell::new(0),
        &Cell::new(0),
    );

    assert_eq!(
        projected.occurrence().1.issuer_key_id().as_str(),
        "current-key-2"
    );
    assert_eq!(projected.occurrence().1.issuer_key_epoch().get(), 2);
    assert_ne!(
        projected.occurrence().1.issuer_key_id(),
        fixture.predecessor().1.issuer_key_id()
    );
}

#[test]
fn owner_native_signer_cannot_issue_runtime_projection() {
    let fixture = OperationFixture::exact();
    let checkpoint_ref = record_ref("publication-checkpoint-1");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let publication_source = source(fixture.predecessor(), (&checkpoint_ref, &checkpoint));
    let mut config = ProjectionFixture::new();
    config.authority = integrated_owner_authority();
    let policy = config.policy();
    let event_admissions = Cell::new(0);
    let signatures = Cell::new(0);
    let result = project_local_operation_publication_from_parts(
        &store_coordinate(),
        &publication_source,
        &store_coordinate(),
        fixture.handle(),
        fixture.predecessor(),
        (&checkpoint_ref, &checkpoint),
        &producer(),
        &writer(),
        &policy,
        &references(
            &record_ref("event-1"),
            &record_ref("occurrence-1"),
            None,
            &record_ref("frontier-1"),
        ),
        None,
        config.contexts(),
        |_| {
            event_admissions.set(event_admissions.get() + 1);
            Ok(())
        },
        |_, _, _| {
            signatures.set(signatures.get() + 1);
            Ok(signature())
        },
        |_, _, _| panic!("revision one must not sign an edge"),
        |_, _, _| {
            signatures.set(signatures.get() + 1);
            Ok(signature())
        },
    );

    assert!(matches!(
        result,
        Err(LocalOperationPublicationProjectionError::Association)
    ));
    assert_eq!(event_admissions.get(), 1);
    assert_eq!(signatures.get(), 0);
}
