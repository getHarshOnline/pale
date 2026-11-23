//! Process ownership laws for bounded Event publication.

use super::*;
use crate::local_operation_start_fixture::{
    checkpoint_preimage, record_ref, signature, store_coordinate,
};
use crate::resident::event_custody::local_operation_projection::tests::{
    producer, writer, PublicationStore,
};
use crate::resident::event_custody::{
    EventPublicationRaceClosure, EventPublicationState, LocalOperationPublicationCurrent,
    LocalOperationPublicationIssuance, LocalOperationPublicationReadmission,
};
use crate::resident::operation_observation_custody::tests::{
    admit_initial_checkpoint, OperationFixture,
};
use crate::resident::process_host::publication::{
    ResidentDeploymentPublicationAccess, ResidentProcessPublicationTurn,
    ResidentProcessPublicationTurnError,
};
use crate::resident::record_projection;
use crate::resident::{
    A2aTaskPublicationRecoveryLimits, A2aTaskPublicationRecoveryRequest,
    A2aTaskPublicationSourceRecoveryError, A2aTaskRecoveryLimits,
    LocalOperationPublicationRecoveryLimits, LocalOperationPublicationRecoveryRequest,
    LocalOperationPublicationSigningContexts, LocalOperationPublicationSourceRecoveryError,
    LocalOperationPublicationSweepError, LocalOperationPublicationSweepLimits,
    LocalOperationPublicationSweepProgress, OperationCheckpointAncestryLimits,
    OperationObservationRecoveryLimits,
};
use pale_core::records::*;
use pale_ports::*;

pub(in crate::resident::process_host) struct PublicationSources {
    store: AuthorityStoreCoordinate,
    through: ControlTransactionSequenceV1,
    source: Option<OperationPublicationSource>,
    reads: usize,
}

impl PublicationSources {
    fn empty(store: AuthorityStoreCoordinate) -> Self {
        Self {
            store,
            through: ControlTransactionSequenceV1::new(1).expect("positive transaction"),
            source: None,
            reads: 0,
        }
    }

    pub(in crate::resident::process_host) fn one(
        store: AuthorityStoreCoordinate,
        source: OperationPublicationSource,
    ) -> Self {
        Self {
            store,
            through: source.transaction(),
            source: Some(source),
            reads: 0,
        }
    }

    pub(in crate::resident::process_host) const fn reads(&self) -> usize {
        self.reads
    }
}

impl ControlTransactionPort for PublicationSources {
    fn prepare_transaction(
        &mut self,
        _: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        panic!("publication source discovery performs no transaction")
    }
}

impl OperationCustodyPort for PublicationSources {
    fn read_a2a_task_bindings(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &OperationA2aBindingScope,
        _: Option<&OperationA2aBindingCursor>,
        _: usize,
        _: usize,
    ) -> Result<OperationA2aBindings, ControlStoreError> {
        panic!("publication source discovery does not inspect task bindings")
    }

    fn read_publication_sources(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&OperationPublicationCursor>,
        maximum_sources: usize,
        maximum_body_bytes: usize,
    ) -> Result<OperationPublicationSources, OperationPublicationReadError> {
        self.reads += 1;
        assert_eq!(store, &self.store);
        assert!(cursor.is_none());
        assert_eq!(maximum_sources, 1);
        assert!(maximum_body_bytes > 0);
        let Some(source) = self.source.clone() else {
            return Ok(OperationPublicationSources::new(
                self.through,
                Vec::new(),
                None,
            ));
        };
        let source_bytes = source.observation().canonical_record_jcs().as_slice().len()
            + source.checkpoint().canonical_record_jcs().as_slice().len();
        if source_bytes > maximum_body_bytes {
            return Err(OperationPublicationReadError::Limit);
        }
        Ok(OperationPublicationSources::new(
            self.through,
            vec![source],
            None,
        ))
    }

    fn read_checkpoint_head(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlCheckpointDomainIdV1,
        _: usize,
    ) -> Result<Option<(ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
        panic!("empty publication discovery does not recover checkpoints")
    }

    fn verify_accumulator_membership(
        &mut self,
        _: &OperationAccumulatorMembership,
    ) -> Result<VerifiedOperationAccumulatorMembership, OperationAccumulatorPreparationError> {
        panic!("empty publication discovery does not verify membership")
    }

    fn prepare_accumulator(
        &mut self,
        _: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        panic!("empty publication discovery does not prepare accumulators")
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("empty publication discovery does not retain leases")
    }

    fn read_operation_record(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecordCoordinate,
        _: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        panic!("empty publication discovery does not recover records")
    }

    fn compare_and_commit(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("empty publication discovery does not commit operation custody")
    }

    fn observe_exact(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        panic!("empty publication discovery does not read operation custody")
    }

    fn read_index(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &OperationCustodyIndexQuery,
        _: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        panic!("empty publication discovery does not read operation indexes")
    }
}

pub(in crate::resident::process_host) fn publication_source() -> OperationPublicationSource {
    let operation = OperationFixture::exact();
    let checkpoint_ref = record_ref("publication/checkpoint-1");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let observation = record_projection::record(
        record_projection::coordinate(
            operation.predecessor().0,
            operation
                .predecessor()
                .1
                .canonical_operation_observation_sha256()
                .as_str(),
        )
        .expect("observation coordinate"),
        operation.predecessor().1,
    )
    .expect("observation record");
    let checkpoint = record_projection::record(
        record_projection::coordinate(
            &checkpoint_ref,
            checkpoint.canonical_control_checkpoint_sha256().as_str(),
        )
        .expect("checkpoint coordinate"),
        &checkpoint,
    )
    .expect("checkpoint record");
    OperationPublicationSource::new(
        ControlTransactionSequenceV1::new(1).expect("positive transaction"),
        observation,
        checkpoint,
    )
    .expect("supported publication source")
}

pub(in crate::resident::process_host) fn publication_contexts<'a>(
    authority: &'a SigningAuthorityBindingV1,
    occurrence: &'a RecordSchemaIdentity,
    edge: &'a RecordSchemaIdentity,
    frontier: &'a RecordSchemaIdentity,
) -> LocalOperationPublicationSigningContexts<'a> {
    LocalOperationPublicationSigningContexts {
        occurrence: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &occurrence.schema_sha256,
            signing_authority: authority,
        },
        edge: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &edge.schema_sha256,
            signing_authority: authority,
        },
        frontier: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &frontier.schema_sha256,
            signing_authority: authority,
        },
    }
}

pub(in crate::resident::process_host) fn publication_schemas() -> (
    RecordSchemaIdentity,
    RecordSchemaIdentity,
    RecordSchemaIdentity,
) {
    (
        pale_contracts::record_schema_identity(INTERACTION_OCCURRENCE_V1_SCHEMA_ID)
            .expect("occurrence schema"),
        pale_contracts::record_schema_identity(GENEALOGY_EDGE_V1_SCHEMA_ID).expect("edge schema"),
        pale_contracts::record_schema_identity(RECONCILIATION_FRONTIER_V1_SCHEMA_ID)
            .expect("frontier schema"),
    )
}

#[allow(clippy::too_many_arguments)]
fn run_unavailable_publication_turn(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    source_custody: &mut PublicationSources,
    event_custody: &mut PublicationStore<'_>,
    publication_authority: &SigningAuthorityBindingV1,
    schemas: &(
        RecordSchemaIdentity,
        RecordSchemaIdentity,
        RecordSchemaIdentity,
    ),
    sweep_limits: LocalOperationPublicationSweepLimits,
    recoveries: &std::cell::Cell<usize>,
    signatures: &std::cell::Cell<usize>,
) -> Result<LocalOperationPublicationSweepProgress, ResidentProcessPublicationTurnError> {
    let current_producer = producer();
    let current_writer = writer();
    let producer_ref = record_ref("publication/producer");
    let writer_ref = record_ref("publication/writer");
    let race_verifier = |_: &EventPublicationRaceClosure<'_>| Ok(());
    let mut sign_occurrence =
        |_: &[u8], _: &InteractionOccurrencePreimageV1, _: &InteractionOccurrenceDigest| {
            signatures.set(signatures.get() + 1);
            Ok(signature())
        };
    let mut sign_edge = |_: &[u8], _: &GenealogyEdgePreimageV1, _: &GenealogyEdgeDigest| {
        signatures.set(signatures.get() + 1);
        Ok(signature())
    };
    let mut sign_frontier =
        |_: &[u8], _: &ReconciliationFrontierPreimageV1, _: &ReconciliationFrontierDigest| {
            signatures.set(signatures.get() + 1);
            Ok(signature())
        };
    let historical = publication_authority.clone();
    let mut admit_event_source = |_: &EventReferencePreimageV1| Ok(());
    let mut resolve_occurrence = |_: &ControlRecord| Ok(historical.clone());
    let mut resolve_edge = |_: &ControlRecord| Ok(historical.clone());
    let mut resolve_frontier = |_: &ControlRecord| Ok(historical.clone());
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
    let mut recover_local = |_: &LocalOperationPublicationRecoveryRequest<'_>| {
        recoveries.set(recoveries.get() + 1);
        Err(LocalOperationPublicationSourceRecoveryError::Unavailable)
    };
    let mut recover_a2a = |_: &A2aTaskPublicationRecoveryRequest<'_>| {
        recoveries.set(recoveries.get() + 1);
        Err(A2aTaskPublicationSourceRecoveryError::Unavailable)
    };
    let source_store = source_custody.store.clone();

    host.run_event_publication_turn(
        selector,
        ResidentDeploymentPublicationAccess {
            source_store: &source_store,
            source_custody,
            event_custody,
        },
        ResidentProcessPublicationTurn {
            sweep_limits,
            local_recovery_limits: recovery_limits(),
            a2a_recovery_limits: a2a_recovery_limits(),
            current: LocalOperationPublicationCurrent {
                producer_allocation_ref: &producer_ref,
                producer: &current_producer,
                writer_fence_ref: &writer_ref,
                writer_fence: &current_writer,
                publication_state: EventPublicationState::Active,
                race_verifier: &race_verifier,
                signing_contexts: publication_contexts(
                    publication_authority,
                    &schemas.0,
                    &schemas.1,
                    &schemas.2,
                ),
            },
            issuance: LocalOperationPublicationIssuance {
                sign_occurrence: &mut sign_occurrence,
                sign_edge: &mut sign_edge,
                sign_frontier: &mut sign_frontier,
            },
            readmission: LocalOperationPublicationReadmission {
                admit_event_source: &mut admit_event_source,
                resolve_occurrence_authority: &mut resolve_occurrence,
                verify_occurrence_signature: &mut verify_occurrence,
                resolve_edge_authority: &mut resolve_edge,
                verify_edge_signature: &mut verify_edge,
                resolve_frontier_authority: &mut resolve_frontier,
                verify_frontier_signature: &mut verify_frontier,
            },
        },
        &mut recover_local,
        &mut recover_a2a,
    )
}

pub(in crate::resident::process_host) fn recovery_limits() -> LocalOperationPublicationRecoveryLimits
{
    LocalOperationPublicationRecoveryLimits::new(
        4,
        OperationObservationRecoveryLimits::new(4, 65_536, 65_536)
            .expect("positive observation limits"),
        OperationCheckpointAncestryLimits::new(4, 65_536, 8).expect("positive checkpoint limits"),
        AccumulatorMembershipLimits::new(8, 8, 65_536).expect("positive membership limits"),
    )
}

pub(in crate::resident::process_host) fn a2a_recovery_limits() -> A2aTaskPublicationRecoveryLimits {
    A2aTaskPublicationRecoveryLimits::new(
        recovery_limits(),
        A2aTaskRecoveryLimits::new(4, 65_536, 65_536, 8, 65_536)
            .expect("positive A2A recovery limits"),
    )
}

#[test]
fn completed_prefix_starts_a_fresh_bounded_turn_without_changing_operation_state() {
    let authority = MemoryAuthorityStore::default();
    let selected_store = store_coordinate();
    let principal_id = principal("principal-1");
    let selector = unproved_selector(principal_id.clone(), selected_store.clone());
    let deployment = unproved_deployment(
        principal_id.clone(),
        &selected_store,
        ready_application(&authority, selected_store.clone(), 1),
    )
    .expect("one admitted deployment");
    let writer_owner = unproved_writer(principal_id, vec![selected_store.clone()]);
    let mut host = ResidentProcessHost::new(
        writer_owner,
        vec![deployment],
        NonZeroUsize::new(1).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one deployment host");
    let mut source_custody = PublicationSources::empty(selected_store.clone());
    let mut event_custody = PublicationStore::new(&authority);
    let publication_authority = integrated_signing_authority(&selector);
    let (occurrence_schema, edge_schema, frontier_schema) = publication_schemas();
    let producer = producer();
    let resident_writer = writer();
    let producer_ref = record_ref("publication/producer");
    let writer_ref = record_ref("publication/writer");
    let race_verifier = |_: &EventPublicationRaceClosure<'_>| Ok(());
    let recoveries = std::cell::Cell::new(0);
    let signatures = std::cell::Cell::new(0);

    for expected_reads in 1..=2 {
        let mut sign_occurrence =
            |_: &[u8], _: &InteractionOccurrencePreimageV1, _: &InteractionOccurrenceDigest| {
                signatures.set(signatures.get() + 1);
                Ok(signature())
            };
        let mut sign_edge = |_: &[u8], _: &GenealogyEdgePreimageV1, _: &GenealogyEdgeDigest| {
            signatures.set(signatures.get() + 1);
            Ok(signature())
        };
        let mut sign_frontier =
            |_: &[u8], _: &ReconciliationFrontierPreimageV1, _: &ReconciliationFrontierDigest| {
                signatures.set(signatures.get() + 1);
                Ok(signature())
            };
        let historical = publication_authority.clone();
        let mut admit_event_source = |_: &EventReferencePreimageV1| Ok(());
        let mut resolve_occurrence = |_: &ControlRecord| Ok(historical.clone());
        let mut resolve_edge = |_: &ControlRecord| Ok(historical.clone());
        let mut resolve_frontier = |_: &ControlRecord| Ok(historical.clone());
        let mut verify_occurrence =
            |_: &[u8],
             _: &InteractionOccurrencePreimageV1,
             _: &InteractionOccurrenceDigest,
             _: &CanonicalSignatureV1| { Ok(()) };
        let mut verify_edge = |_: &[u8],
                               _: &GenealogyEdgePreimageV1,
                               _: &GenealogyEdgeDigest,
                               _: &CanonicalSignatureV1| { Ok(()) };
        let mut verify_frontier =
            |_: &[u8],
             _: &ReconciliationFrontierPreimageV1,
             _: &ReconciliationFrontierDigest,
             _: &CanonicalSignatureV1| { Ok(()) };
        let mut recover_local = |_: &LocalOperationPublicationRecoveryRequest<'_>| {
            recoveries.set(recoveries.get() + 1);
            Err(LocalOperationPublicationSourceRecoveryError::Unavailable)
        };
        let mut recover_a2a = |_: &A2aTaskPublicationRecoveryRequest<'_>| {
            recoveries.set(recoveries.get() + 1);
            Err(A2aTaskPublicationSourceRecoveryError::Unavailable)
        };

        let result = host
            .run_event_publication_turn(
                &selector,
                ResidentDeploymentPublicationAccess {
                    source_store: &selected_store,
                    source_custody: &mut source_custody,
                    event_custody: &mut event_custody,
                },
                ResidentProcessPublicationTurn {
                    sweep_limits: LocalOperationPublicationSweepLimits::new(1, 1, 1024, 65_536)
                        .expect("positive sweep limits"),
                    local_recovery_limits: recovery_limits(),
                    a2a_recovery_limits: a2a_recovery_limits(),
                    current: LocalOperationPublicationCurrent {
                        producer_allocation_ref: &producer_ref,
                        producer: &producer,
                        writer_fence_ref: &writer_ref,
                        writer_fence: &resident_writer,
                        publication_state: EventPublicationState::Active,
                        race_verifier: &race_verifier,
                        signing_contexts: publication_contexts(
                            &publication_authority,
                            &occurrence_schema,
                            &edge_schema,
                            &frontier_schema,
                        ),
                    },
                    issuance: LocalOperationPublicationIssuance {
                        sign_occurrence: &mut sign_occurrence,
                        sign_edge: &mut sign_edge,
                        sign_frontier: &mut sign_frontier,
                    },
                    readmission: LocalOperationPublicationReadmission {
                        admit_event_source: &mut admit_event_source,
                        resolve_occurrence_authority: &mut resolve_occurrence,
                        verify_occurrence_signature: &mut verify_occurrence,
                        resolve_edge_authority: &mut resolve_edge,
                        verify_edge_signature: &mut verify_edge,
                        resolve_frontier_authority: &mut resolve_frontier,
                        verify_frontier_signature: &mut verify_frontier,
                    },
                },
                &mut recover_local,
                &mut recover_a2a,
            )
            .expect("empty frozen prefix is complete");

        assert!(matches!(
            result,
            LocalOperationPublicationSweepProgress::SupportedLocalPrefixComplete {
                sources_examined: 0,
                ..
            }
        ));
        assert_eq!(source_custody.reads, expected_reads);
    }

    assert_eq!(recoveries.get(), 0);
    assert_eq!(signatures.get(), 0);
    assert!(host.deployments[0].application.journal_head().is_none());
}

#[test]
fn unlike_deployment_is_rejected_before_source_or_event_custody() {
    let authority = MemoryAuthorityStore::default();
    let selected_store = store_coordinate();
    let principal_id = principal("principal-1");
    let selector = unproved_selector(principal_id.clone(), selected_store.clone());
    let deployment = unproved_deployment(
        principal_id.clone(),
        &selected_store,
        ready_application(&authority, selected_store.clone(), 1),
    )
    .expect("one admitted deployment");
    let mut host = ResidentProcessHost::new(
        unproved_writer(principal_id, vec![selected_store.clone()]),
        vec![deployment],
        NonZeroUsize::new(1).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one deployment host");
    let foreign = unproved_selector(principal("principal-foreign"), selected_store.clone());
    let mut source_custody = PublicationSources::empty(selected_store.clone());
    let mut event_custody = PublicationStore::new(&authority);
    let publication_authority = integrated_signing_authority(&selector);
    let (occurrence_schema, edge_schema, frontier_schema) = publication_schemas();
    let producer = producer();
    let resident_writer = writer();
    let producer_ref = record_ref("publication/producer");
    let writer_ref = record_ref("publication/writer");
    let race_verifier = |_: &EventPublicationRaceClosure<'_>| Ok(());
    let mut unreachable_occurrence =
        |_: &[u8], _: &InteractionOccurrencePreimageV1, _: &InteractionOccurrenceDigest| {
            panic!("selection failure must not sign")
        };
    let mut unreachable_edge = |_: &[u8], _: &GenealogyEdgePreimageV1, _: &GenealogyEdgeDigest| {
        panic!("selection failure must not sign")
    };
    let mut unreachable_frontier =
        |_: &[u8], _: &ReconciliationFrontierPreimageV1, _: &ReconciliationFrontierDigest| {
            panic!("selection failure must not sign")
        };
    let historical = publication_authority.clone();
    let mut admit_event_source = |_: &EventReferencePreimageV1| Ok(());
    let mut resolve_occurrence = |_: &ControlRecord| Ok(historical.clone());
    let mut resolve_edge = |_: &ControlRecord| Ok(historical.clone());
    let mut resolve_frontier = |_: &ControlRecord| Ok(historical.clone());
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
    let mut recover_local = |_: &LocalOperationPublicationRecoveryRequest<'_>| {
        panic!("selection failure must not recover a source")
    };
    let mut recover_a2a = |_: &A2aTaskPublicationRecoveryRequest<'_>| {
        panic!("selection failure must not recover a remote source")
    };

    let result = host.run_event_publication_turn(
        &foreign,
        ResidentDeploymentPublicationAccess {
            source_store: &selected_store,
            source_custody: &mut source_custody,
            event_custody: &mut event_custody,
        },
        ResidentProcessPublicationTurn {
            sweep_limits: LocalOperationPublicationSweepLimits::new(1, 1, 1024, 65_536)
                .expect("positive sweep limits"),
            local_recovery_limits: recovery_limits(),
            a2a_recovery_limits: a2a_recovery_limits(),
            current: LocalOperationPublicationCurrent {
                producer_allocation_ref: &producer_ref,
                producer: &producer,
                writer_fence_ref: &writer_ref,
                writer_fence: &resident_writer,
                publication_state: EventPublicationState::Active,
                race_verifier: &race_verifier,
                signing_contexts: publication_contexts(
                    &publication_authority,
                    &occurrence_schema,
                    &edge_schema,
                    &frontier_schema,
                ),
            },
            issuance: LocalOperationPublicationIssuance {
                sign_occurrence: &mut unreachable_occurrence,
                sign_edge: &mut unreachable_edge,
                sign_frontier: &mut unreachable_frontier,
            },
            readmission: LocalOperationPublicationReadmission {
                admit_event_source: &mut admit_event_source,
                resolve_occurrence_authority: &mut resolve_occurrence,
                verify_occurrence_signature: &mut verify_occurrence,
                resolve_edge_authority: &mut resolve_edge,
                verify_edge_signature: &mut verify_edge,
                resolve_frontier_authority: &mut resolve_frontier,
                verify_frontier_signature: &mut verify_frontier,
            },
        },
        &mut recover_local,
        &mut recover_a2a,
    );

    assert!(matches!(
        result,
        Err(ResidentProcessPublicationTurnError::Selection(
            ResidentProcessHostSelectionError::UnknownDeploymentAuthority
        ))
    ));
    assert_eq!(source_custody.reads, 0);
    assert!(host.deployments[0].application.journal_head().is_none());
    assert_eq!(
        selector,
        unproved_selector(principal("principal-1"), selected_store)
    );
}

#[test]
fn unlike_publication_signers_are_rejected_before_source_or_event_custody() {
    let authority = MemoryAuthorityStore::default();
    let selected_store = store_coordinate();
    let principal_id = principal("principal-1");
    let selector = unproved_selector(principal_id.clone(), selected_store.clone());
    let deployment = unproved_deployment(
        principal_id.clone(),
        &selected_store,
        ready_application(&authority, selected_store.clone(), 1),
    )
    .expect("one admitted deployment");
    let mut host = ResidentProcessHost::new(
        unproved_writer(principal_id.clone(), vec![selected_store.clone()]),
        vec![deployment],
        NonZeroUsize::new(1).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one deployment host");
    let foreign_selector = unproved_selector(
        principal_id,
        store(
            "foreign-publication-store",
            "foreign-publication-incarnation",
        ),
    );
    let foreign_authority = integrated_signing_authority(&foreign_selector);
    let schemas = publication_schemas();
    let mut source_custody = PublicationSources::one(selected_store, publication_source());
    let mut event_custody = PublicationStore::new(&authority);
    let recoveries = std::cell::Cell::new(0);
    let signatures = std::cell::Cell::new(0);

    let result = run_unavailable_publication_turn(
        &mut host,
        &selector,
        &mut source_custody,
        &mut event_custody,
        &foreign_authority,
        &schemas,
        LocalOperationPublicationSweepLimits::new(1, 1, 65_536, 65_536)
            .expect("positive sweep limits"),
        &recoveries,
        &signatures,
    );

    assert!(matches!(
        result,
        Err(ResidentProcessPublicationTurnError::Selection(
            ResidentProcessHostSelectionError::PublicationCurrentAuthority
        ))
    ));
    assert_eq!(source_custody.reads, 0);
    assert_eq!(recoveries.get(), 0);
    assert_eq!(signatures.get(), 0);
    assert!(host.deployments[0].application.journal_head().is_none());
}

#[test]
fn budget_and_recovery_failure_keep_one_pending_source_without_rescan() {
    let authority = MemoryAuthorityStore::default();
    let selected_store = store_coordinate();
    let principal_id = principal("principal-1");
    let selector = unproved_selector(principal_id.clone(), selected_store.clone());
    let deployment = unproved_deployment(
        principal_id.clone(),
        &selected_store,
        ready_application(&authority, selected_store.clone(), 1),
    )
    .expect("one admitted deployment");
    let mut host = ResidentProcessHost::new(
        unproved_writer(principal_id, vec![selected_store.clone()]),
        vec![deployment],
        NonZeroUsize::new(1).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one deployment host");
    let publication_authority = integrated_signing_authority(&selector);
    let schemas = publication_schemas();
    let source = publication_source();
    let source_bytes = source.observation().canonical_record_jcs().as_slice().len()
        + source.checkpoint().canonical_record_jcs().as_slice().len();
    let mut source_custody = PublicationSources::one(selected_store, source);
    let mut event_custody = PublicationStore::new(&authority);
    let recoveries = std::cell::Cell::new(0);
    let signatures = std::cell::Cell::new(0);

    let budget = run_unavailable_publication_turn(
        &mut host,
        &selector,
        &mut source_custody,
        &mut event_custody,
        &publication_authority,
        &schemas,
        LocalOperationPublicationSweepLimits::new(1, 1, source_bytes - 1, 65_536)
            .expect("positive sweep limits"),
        &recoveries,
        &signatures,
    )
    .expect("a source above the selected byte ceiling is bounded");
    assert!(matches!(
        budget,
        LocalOperationPublicationSweepProgress::BudgetExhausted
    ));
    assert_eq!(source_custody.reads, 1);
    assert_eq!(recoveries.get(), 0);

    let first_recovery = run_unavailable_publication_turn(
        &mut host,
        &selector,
        &mut source_custody,
        &mut event_custody,
        &publication_authority,
        &schemas,
        LocalOperationPublicationSweepLimits::new(1, 1, source_bytes, 65_536)
            .expect("positive sweep limits"),
        &recoveries,
        &signatures,
    );
    assert!(matches!(
        first_recovery,
        Err(ResidentProcessPublicationTurnError::Sweep(
            LocalOperationPublicationSweepError::Recovery(
                LocalOperationPublicationSourceRecoveryError::Unavailable
            )
        ))
    ));
    assert_eq!(source_custody.reads, 2);
    assert_eq!(recoveries.get(), 1);
    assert_eq!(host.publication_attempts[0].pages_examined(), 1);
    assert_eq!(host.publication_attempts[0].sources_examined(), 1);
    assert_eq!(
        host.publication_attempts[0].source_body_bytes_examined(),
        source_bytes
    );

    let repeated_recovery = run_unavailable_publication_turn(
        &mut host,
        &selector,
        &mut source_custody,
        &mut event_custody,
        &publication_authority,
        &schemas,
        LocalOperationPublicationSweepLimits::new(1, 1, source_bytes, 65_536)
            .expect("positive sweep limits"),
        &recoveries,
        &signatures,
    );
    assert!(matches!(
        repeated_recovery,
        Err(ResidentProcessPublicationTurnError::Sweep(
            LocalOperationPublicationSweepError::Recovery(
                LocalOperationPublicationSourceRecoveryError::Unavailable
            )
        ))
    ));
    assert_eq!(
        source_custody.reads, 2,
        "the retained source is not enumerated again"
    );
    assert_eq!(recoveries.get(), 2);
    assert_eq!(signatures.get(), 0);
    assert!(host.deployments[0].application.journal_head().is_none());
}
