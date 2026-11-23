//! Restart-safe local operation publication over real `SQLite` custody.
//!
//! The signer, current producer, writer fence, and independent anti-rollback
//! anchor remain explicit test models. The operation source, protected recovery,

//! resident journal, Event records, and Event indexes use the native adapter.

use super::*;
use crate::location::{ControlStorageRoot, ControlStoreScope, DeploymentStoreScope};
use crate::operation::tests::{with_generation, IndependentAnchor};
use crate::operation::SqliteOperationCustody;
use crate::resident_fixture::local_operation_start_fixture::{
    checkpoint_authority, checkpoint_preimage, issue_observation, profile, publication_writer,
    record_ref, signature, store_coordinate, with_start, RecoveryParents, StartAdmission,
};
use crate::resident_fixture::producer_custody_fixture::InitialProducerFixture;
use crate::store::{initialize_unadmitted_store, ControlStoreIdentity};
use pale_application::resident::*;
use pale_core::records::*;
use pale_ports::*;
use rusqlite::Connection;
use std::cell::Cell;

pub(super) fn writer() -> PrincipalResidentWriterFenceV1 {
    publication_writer()
}

fn recover_source(
    request: &LocalOperationPublicationRecoveryRequest<'_>,
    original_request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    checkpoint_fields: &ControlStateCheckpointPreimageV1,
    operation_port: &mut SqliteOperationCustody<'_>,
    sessions: &dyn SessionLeaseCustodyPort,
    anchor: &mut IndependentAnchor,
) -> Result<RecoveredOperationControl, LocalOperationPublicationSourceRecoveryError> {
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(parents.current.generation.compiled_generation);
    resolver.insert_active(parents.current.generation.active_head);
    resolver.insert_fence(parents.current.generation.activation_fence);
    resolver.insert_effective_profile(parents.current.generation.effective_profile);
    let checkpoint_context = checkpoint_authority(checkpoint_fields);
    let request_preimage = original_request.to_preimage();
    let request_context = CommonRequestAdmissionContextV1 {
        expected_schema_sha256: &request_preimage.envelope_schema_sha256,
        expected_body_schema_id: &request_preimage.body.schema_id,
        expected_body_schema_sha256: &request_preimage.body.schema_sha256,
        caller: &request_preimage.caller,
        signing_authority: &request_preimage.signing_authority_binding,
        route: &request_preimage.route,
        session_admission: &request_preimage.session_admission,
        operation_target: request_preimage.operation_target.as_ref(),
        execution_owner_fence: None,
        federation_claim: None,
        workspace_id: &request_preimage.workspace_id,
        capability_id: &request_preimage.capability_id,
        operation: &request_preimage.operation,
        scope: &request_preimage.scope,
        risk_class: &request_preimage.risk_class,
        resource_constraints: &request_preimage.resource_constraints,
        expected_precondition: None,
        idempotency_key: &request_preimage.idempotency_key,
        issued_at: request_preimage.issued_at,
        expires_at: request_preimage.expires_at,
        generation: request_preimage.generation.as_ref(),
        fence_token: request_preimage.fence_token.as_ref(),
    };
    let mut admit_request = |reference: &ResidentRecordRefV1, wire| {
        assert_eq!(reference, &record_ref("request/publication-sweep"));
        CommonRequestEnvelopeV1::admit_contextual_wire(wire, &request_context, |_, _, _| Ok(()))
    };
    let mut admit_handle = |_: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
        assert!(parent.is_none());
        Ok(())
    };
    let mut verify_handle = |domain: &[u8],
                             _: &OperationHandlePreimageV1,
                             _: &OperationHandleDigest,
                             actual: &CanonicalSignatureV1| {
        assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
        assert_eq!(actual, &signature());
        Ok(())
    };
    let mut admit_checkpoint = |_: &ResidentRecordRefV1, wire| {
        ControlStateCheckpointV1::admit_signed_wire(
            wire,
            &checkpoint_fields.schema_sha256,
            &checkpoint_context,
            |_, _, _, actual| {
                assert_eq!(actual, &signature());
                Ok(())
            },
        )
    };
    let mut resolve_observation =
        |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| Ok(parents.signing.clone());
    let mut admit_state = |_: &OperationObservationStateV1| Ok(());
    let mut admit_rotation = |_: &ResidentRecordRefV1,
                              _: &ArtifactDigest,
                              _: &ResidentRecordRefV1,
                              _: &ArtifactDigest| { Ok(()) };
    let mut verify_observation = |domain: &[u8],
                                  _: &OperationObservationPreimageV1,
                                  _: &OperationObservationDigest,
                                  actual: &CanonicalSignatureV1| {
        assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
        assert_eq!(actual, &signature());
        Ok(())
    };
    let mut verify_checkpoint = |domain: &[u8],
                                 _: &ControlStateCheckpointPreimageV1,
                                 _: &ControlStateCheckpointDigest,
                                 actual: &CanonicalSignatureV1| {
        assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
        assert_eq!(actual, &signature());
        Ok(())
    };
    let observation_digest =
        OperationObservationDigest::parse(request.observation().record_sha256().as_str()).unwrap();
    let supported_profile = profile();
    recover_operation_control(
        OperationControlRecoveryInput {
            operation: LocalOperationHandleRecoveryInput {
                context: LocalOperationRecoveryContext {
                    store: request.store(),
                    profile: &supported_profile,
                    handle_signing_authority: parents.signing,
                    checkpoint_authority: &checkpoint_context,
                    historical: &resolver,
                    maximum_record_bytes: 1 << 20,
                    maximum_operation_lease_bytes: 1 << 20,
                    maximum_session_lease_bytes: 1 << 20,
                    membership_limits: request.limits().membership(),
                    observation_limits: request.limits().observation(),
                    checkpoint_limits: request.limits().checkpoint(),
                },
                expected_handle: request.operation_handle(),
                maximum_parent_depth: request.limits().maximum_parent_depth(),
            },
            observation: OperationObservationSelection::HistoryMember {
                reference: request.observation().record_ref(),
                digest: &observation_digest,
            },
            observation_limits: request.limits().observation(),
            checkpoint_limits: request.limits().checkpoint(),
            membership_limits: request.limits().membership(),
        },
        operation_port,
        sessions,
        anchor,
        OperationControlRecoveryAdmission {
            admit_request: &mut admit_request,
            admit_handle_authority: &mut admit_handle,
            verify_handle_signature: &mut verify_handle,
            admit_checkpoint: &mut admit_checkpoint,
            resolve_observation_signer: &mut resolve_observation,
            admit_observation_state: &mut admit_state,
            admit_rotation: &mut admit_rotation,
            verify_observation_signature: &mut verify_observation,
            verify_checkpoint_signature: &mut verify_checkpoint,
        },
    )
    .map_err(Into::into)
}

#[test]
fn committed_local_source_projects_every_stage_across_resident_restarts() {
    with_generation(|generation, effective| {
        with_start(
            generation,
            effective,
            "publication-sweep",
            |_| {},
            StartAdmission::AcceptedOwnerModel,
            |prepared, request, _, _, parents| {
                let directory = crate::test_support::private_tempdir();
                let binding = parents.current.deployment_binding;
                let scope = ControlStoreScope::Deployment(DeploymentStoreScope::new(
                    ResidentWorkspaceIdV1::parse(
                        binding.deployment_terms().workspace_id().as_str(),
                    )
                    .unwrap(),
                    parents.current.deployment_binding_ref.clone(),
                    ArtifactDigest::parse(binding.canonical_binding_sha256().as_str()).unwrap(),
                ));
                let location = ControlStorageRoot::retain(directory.path())
                    .unwrap()
                    .prepare_store_location(scope.clone())
                    .unwrap();
                let coordinate = store_coordinate();
                let initialized = initialize_unadmitted_store(
                    &location,
                    ControlStoreIdentity::new(
                        scope,
                        coordinate.store_id().clone(),
                        coordinate.incarnation_id().clone(),
                    ),
                )
                .unwrap();
                let file = initialized.path();
                let sessions = DeploymentControlSqliteStore::open_unadmitted_candidate(
                    &location,
                    coordinate.clone(),
                )
                .unwrap();
                let session = parents.session.lease();
                sessions
                    .compare_and_retain(&SessionLeaseBody::new(
                        coordinate.clone(),
                        session.session_admission_lease_digest().clone(),
                        session.schema_id().clone(),
                        session.schema_sha256().clone(),
                        ControlRecordBytes::new(
                            pale_core::canon::to_canonical_string(
                                &SessionAdmissionLeaseWireV1::from(session),
                            )
                            .unwrap()
                            .into_bytes(),
                        )
                        .unwrap(),
                    ))
                    .unwrap();

                let mut operation_connection = Connection::open(file).unwrap();
                let mut operation_port = SqliteOperationCustody {
                    connection: &mut operation_connection,
                    coordinate: coordinate.clone(),
                };
                retain_operation_lease_body(
                    &mut operation_port,
                    &coordinate,
                    &record_ref("start-lease"),
                    parents.lease,
                    1 << 20,
                )
                .unwrap();
                let prepared = prepared
                    .unwrap()
                    .prepare_checkpoint(
                        AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4096, 100_000)
                            .unwrap(),
                        &mut operation_port,
                    )
                    .unwrap();
                let initial_source_transaction = prepared.transaction_sequence();
                let mut checkpoint = checkpoint_preimage(1, None);
                checkpoint.checkpoint_profile_sha256 =
                    SupportedOperationCheckpointProfile::content_digest().unwrap();
                checkpoint.checkpoint_domain_id =
                    SupportedOperationCheckpointProfile::conflict_domain(request).unwrap();
                checkpoint.committed_transaction_sequence = prepared.transaction_sequence();
                checkpoint.state_root_sha256 = prepared.state_root().clone();
                checkpoint.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
                checkpoint.changed_leaf_keys = prepared.changed_keys().to_vec();
                checkpoint.leaf_count = ControlLeafCountV1::new(4).unwrap();
                checkpoint.issued_at = UnixTimeV1::new(61).unwrap();
                let (physical, pending) = prepared
                    .issue_checkpoint(
                        &record_ref("publication/checkpoint-1"),
                        checkpoint.clone(),
                        &checkpoint.schema_sha256,
                        &checkpoint_authority(&checkpoint),
                        |_, _, _| Ok(signature()),
                    )
                    .unwrap();
                let mut anchor = IndependentAnchor::default();
                let completed = physical
                    .commit(&mut operation_port, &mut anchor, 1 << 20)
                    .unwrap()
                    .advance_anchor(&mut anchor)
                    .unwrap();
                let initial_candidate = completed.candidate().clone();
                let initial_checkpoint = ControlStateCheckpointV1::admit_signed_wire(
                    serde_json::from_slice(
                        initial_candidate
                            .checkpoint()
                            .successor_record()
                            .canonical_record_jcs()
                            .as_slice(),
                    )
                    .unwrap(),
                    &checkpoint.schema_sha256,
                    &checkpoint_authority(&checkpoint),
                    |_, _, _, actual| {
                        assert_eq!(actual, &signature());
                        Ok(())
                    },
                )
                .unwrap();
                let issued = pending
                    .finish(completed)
                    .unwrap_or_else(|_| panic!("matching committed start and pending issuance"));
                let (handle_ref, handle) = issued.handle();
                let initial_observation_record = initial_candidate
                    .record(OperationCustodyRecordKind::OperationObservation)
                    .unwrap();
                let observation_schema =
                    crate::resident_fixture::local_operation_start_fixture::schema_digest(
                        OPERATION_OBSERVATION_V1_SCHEMA_ID,
                    );
                let initial_observation = issue_observation(
                    parents.signing,
                    handle_ref,
                    handle,
                    observation_schema.clone(),
                    OperationObservationRevisionV1::new(1).unwrap(),
                    None,
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::Resolving,
                    },
                    60,
                );
                assert_eq!(
                    initial_observation_record.canonical_record_jcs().as_slice(),
                    pale_core::canon::to_canonical_string(&initial_observation)
                        .unwrap()
                        .as_bytes()
                );

                let successor_ref = record_ref("observation/publication-sweep-context-bound");
                let successor = issue_observation(
                    parents.signing,
                    handle_ref,
                    handle,
                    observation_schema,
                    OperationObservationRevisionV1::new(2).unwrap(),
                    Some((
                        initial_observation_record.coordinate().record_ref(),
                        &initial_observation,
                    )),
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::ContextBound {
                            evidence: ImmutableRecordReferenceV1 {
                                record_ref: record_ref("publication/context-evidence"),
                                record_sha256: ArtifactDigest::hash_bytes(
                                    b"publication context evidence",
                                ),
                            },
                        },
                    },
                    62,
                );
                let append = prepare_operation_observation_append(
                    OperationObservationTransition::new(
                        handle_ref,
                        handle,
                        initial_observation_record.coordinate().record_ref(),
                        &initial_observation,
                        &successor_ref,
                        &successor,
                    ),
                    &|actual: &OperationObservationV1| {
                        assert_eq!(
                            actual.deployment_runtime_authority_ref(),
                            successor.deployment_runtime_authority_ref()
                        );
                        assert_eq!(
                            actual.deployment_runtime_authority_sha256(),
                            successor.deployment_runtime_authority_sha256()
                        );
                        Ok(coordinate.clone())
                    },
                )
                .unwrap();
                let prepared_successor = append
                    .prepare_checkpoint(
                        issued.checkpoint_selection(),
                        initial_candidate
                            .checkpoint()
                            .successor_record()
                            .coordinate()
                            .record_ref(),
                        &initial_checkpoint,
                        AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000)
                            .unwrap(),
                        &mut operation_port,
                    )
                    .unwrap();
                let successor_source_transaction = prepared_successor.transaction_sequence();
                let mut successor_checkpoint = checkpoint_preimage(
                    2,
                    Some((
                        initial_candidate
                            .checkpoint()
                            .successor_record()
                            .coordinate()
                            .record_ref()
                            .clone(),
                        initial_checkpoint
                            .canonical_control_checkpoint_sha256()
                            .clone(),
                    )),
                );
                successor_checkpoint.checkpoint_profile_ref =
                    initial_checkpoint.checkpoint_profile_ref().clone();
                successor_checkpoint.checkpoint_profile_sha256 =
                    initial_checkpoint.checkpoint_profile_sha256().clone();
                successor_checkpoint.checkpoint_domain_id =
                    initial_checkpoint.checkpoint_domain_id().clone();
                successor_checkpoint.committed_transaction_sequence =
                    prepared_successor.transaction_sequence();
                successor_checkpoint.state_root_sha256 = prepared_successor.state_root().clone();
                successor_checkpoint.unresolved_custody_root_sha256 =
                    prepared_successor.unresolved_root().clone();
                successor_checkpoint.changed_leaf_keys = prepared_successor.changed_keys().to_vec();
                successor_checkpoint.leaf_count = prepared_successor.leaf_count();
                successor_checkpoint.issued_at = UnixTimeV1::new(63).unwrap();
                let (successor_physical, _) = prepared_successor
                    .issue_checkpoint(
                        &record_ref("publication/checkpoint-2"),
                        successor_checkpoint.clone(),
                        &successor_checkpoint.schema_sha256,
                        &checkpoint_authority(&successor_checkpoint),
                        |_, _, _| Ok(signature()),
                    )
                    .unwrap();
                successor_physical
                    .commit(&mut operation_port, &mut anchor)
                    .unwrap()
                    .advance_anchor(&mut anchor)
                    .unwrap();
                drop(issued);
                drop(operation_port);
                drop(operation_connection);

                let (producer_fixture, _) = InitialProducerFixture::new();
                let current_writer = writer();
                let occurrence_schema =
                    pale_contracts::record_schema_identity(INTERACTION_OCCURRENCE_V1_SCHEMA_ID)
                        .unwrap();
                let edge_schema =
                    pale_contracts::record_schema_identity(GENEALOGY_EDGE_V1_SCHEMA_ID).unwrap();
                let frontier_schema =
                    pale_contracts::record_schema_identity(RECONCILIATION_FRONTIER_V1_SCHEMA_ID)
                        .unwrap();
                let limits =
                    LocalOperationPublicationSweepLimits::new(32, 32, 1 << 20, 1 << 20).unwrap();
                let recovery_limits = LocalOperationPublicationRecoveryLimits::new(
                    0,
                    OperationObservationRecoveryLimits::new(8, 1 << 20, 1 << 20).unwrap(),
                    OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).unwrap(),
                    AccumulatorMembershipLimits::new(3, 10_000, 100_000).unwrap(),
                );
                let mut resident_epoch = 0_u64;
                for replay in [false, true] {
                    let mut attempt =
                        LocalOperationPublicationSweepAttempt::new(coordinate.clone());
                    let mut established = Vec::new();
                    let signatures_issued = Cell::new(0_usize);

                    loop {
                        resident_epoch += 1;
                        let store = DeploymentControlSqliteStore::open_unadmitted_candidate(
                            &location,
                            coordinate.clone(),
                        )
                        .unwrap();
                        let mut source = FixedInputSource::default();
                        let mut resident = open_resident(
                            &store,
                            coordinate.clone(),
                            &RecoveryResolver::default(),
                            &mut source,
                            resident_epoch,
                        );
                        let mut source_connection = Connection::open(file).unwrap();
                        let mut source_port = SqliteOperationCustody {
                            connection: &mut source_connection,
                            coordinate: coordinate.clone(),
                        };
                        let mut recovery_connection = Connection::open(file).unwrap();
                        let mut recovery_port = SqliteOperationCustody {
                            connection: &mut recovery_connection,
                            coordinate: coordinate.clone(),
                        };
                        let mut sign_occurrence =
                            |_: &[u8],
                             _: &InteractionOccurrencePreimageV1,
                             _: &InteractionOccurrenceDigest| {
                                signatures_issued.set(signatures_issued.get() + 1);
                                Ok(signature())
                            };
                        let mut sign_edge =
                            |_: &[u8], _: &GenealogyEdgePreimageV1, _: &GenealogyEdgeDigest| {
                                signatures_issued.set(signatures_issued.get() + 1);
                                Ok(signature())
                            };
                        let mut sign_frontier =
                            |_: &[u8],
                             _: &ReconciliationFrontierPreimageV1,
                             _: &ReconciliationFrontierDigest| {
                                signatures_issued.set(signatures_issued.get() + 1);
                                Ok(signature())
                            };
                        let mut issuance = LocalOperationPublicationIssuance {
                            sign_occurrence: &mut sign_occurrence,
                            sign_edge: &mut sign_edge,
                            sign_frontier: &mut sign_frontier,
                        };
                        let mut admit_event = |_: &EventReferencePreimageV1| Ok(());
                        let mut resolve_occurrence =
                            |_: &ControlRecord| Ok(parents.signing.clone());
                        let mut resolve_edge = |_: &ControlRecord| Ok(parents.signing.clone());
                        let mut resolve_frontier = |_: &ControlRecord| Ok(parents.signing.clone());
                        let mut verify_occurrence =
                            |_: &[u8],
                             _: &InteractionOccurrencePreimageV1,
                             _: &InteractionOccurrenceDigest,
                             _: &CanonicalSignatureV1| Ok(());
                        let mut verify_edge =
                            |_: &[u8],
                             _: &GenealogyEdgePreimageV1,
                             _: &GenealogyEdgeDigest,
                             _: &CanonicalSignatureV1| Ok(());
                        let mut verify_frontier =
                            |_: &[u8],
                             _: &ReconciliationFrontierPreimageV1,
                             _: &ReconciliationFrontierDigest,
                             _: &CanonicalSignatureV1| Ok(());
                        let mut readmission = LocalOperationPublicationReadmission {
                            admit_event_source: &mut admit_event,
                            resolve_occurrence_authority: &mut resolve_occurrence,
                            verify_occurrence_signature: &mut verify_occurrence,
                            resolve_edge_authority: &mut resolve_edge,
                            verify_edge_signature: &mut verify_edge,
                            resolve_frontier_authority: &mut resolve_frontier,
                            verify_frontier_signature: &mut verify_frontier,
                        };
                        let mut recover =
                            |source: &LocalOperationPublicationRecoveryRequest<'_>| {
                                recover_source(
                                    source,
                                    request,
                                    &parents,
                                    &checkpoint,
                                    &mut recovery_port,
                                    &store,
                                    &mut anchor,
                                )
                            };
                        let verifier = |_: &EventPublicationRaceClosure<'_>| Ok(());
                        let contexts = LocalOperationPublicationSigningContexts {
                            occurrence: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &occurrence_schema.schema_sha256,
                                signing_authority: parents.signing,
                            },
                            edge: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &edge_schema.schema_sha256,
                                signing_authority: parents.signing,
                            },
                            frontier: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &frontier_schema.schema_sha256,
                                signing_authority: parents.signing,
                            },
                        };
                        let mut event_port = &store;
                        let a2a_recovery_limits = A2aTaskPublicationRecoveryLimits::new(
                            recovery_limits,
                            A2aTaskRecoveryLimits::new(8, 65_536, 1 << 20, 16, 1 << 20).unwrap(),
                        );
                        let mut recover_a2a = |_: &A2aTaskPublicationRecoveryRequest<'_>| {
                            Err(A2aTaskPublicationSourceRecoveryError::Unavailable)
                        };
                        match sweep_local_operation_publications(
                        &mut attempt,
                        limits,
                        recovery_limits,
                        a2a_recovery_limits,
                        &mut source_port,
                        &mut resident,
                        &mut event_port,
                        &mut recover,
                        &mut recover_a2a,
                        LocalOperationPublicationCurrent {
                            producer_allocation_ref: &producer_fixture.allocation_ref,
                            producer: &producer_fixture.allocation,
                            writer_fence_ref: &record_ref("publication-writer-record"),
                            writer_fence: &current_writer,
                            publication_state: EventPublicationState::Active,
                            race_verifier: &verifier,
                            signing_contexts: contexts,
                        },
                        &mut issuance,
                        &mut readmission,
                    )
                    .unwrap()
                    {
                        LocalOperationPublicationSweepProgress::StageEstablished {
                            source_transaction: observed_source_transaction,
                            stage,
                            evidence,
                            ..
                        } => {
                            assert!(
                                observed_source_transaction == initial_source_transaction
                                    || observed_source_transaction == successor_source_transaction
                            );
                            assert_eq!(evidence, EventCustodyCommitEvidence::New);
                            established.push((observed_source_transaction, stage));
                        }
                        LocalOperationPublicationSweepProgress::SupportedLocalPrefixComplete {
                            sources_examined,
                            ..
                        } => {
                            assert_eq!(sources_examined, 2);
                            break;
                        }
                        LocalOperationPublicationSweepProgress::ContinueScan => {}
                        LocalOperationPublicationSweepProgress::BudgetExhausted => {
                            panic!("fixed two-source sweep cannot exhaust its aggregate budget")
                        }
                    }
                    }
                    if replay {
                        assert!(
                            established.is_empty(),
                            "a cold replay must read the exact completed prefix without another commit"
                        );
                        assert_eq!(
                            signatures_issued.get(),
                            0,
                            "a cold replay must use historical custody without current signing"
                        );
                    } else {
                        assert_eq!(
                            established,
                            [
                                (
                                    initial_source_transaction,
                                    LocalOperationPublicationStage::Event,
                                ),
                                (
                                    initial_source_transaction,
                                    LocalOperationPublicationStage::Occurrence,
                                ),
                                (
                                    initial_source_transaction,
                                    LocalOperationPublicationStage::ReconciliationFrontier,
                                ),
                                (
                                    successor_source_transaction,
                                    LocalOperationPublicationStage::Event,
                                ),
                                (
                                    successor_source_transaction,
                                    LocalOperationPublicationStage::Occurrence,
                                ),
                                (
                                    successor_source_transaction,
                                    LocalOperationPublicationStage::GenealogyEdge,
                                ),
                                (
                                    successor_source_transaction,
                                    LocalOperationPublicationStage::ReconciliationFrontier,
                                ),
                            ]
                        );
                        assert_eq!(
                            signatures_issued.get(),
                            5,
                            "two occurrences, two frontiers, and one successor edge require signatures"
                        );
                    }
                }
                let reopened = DeploymentControlSqliteStore::open_unadmitted_candidate(
                    &location,
                    coordinate.clone(),
                )
                .unwrap();
                let mut reopened_event_port = &reopened;
                let pending = read_pending_pale_derived_publications(
                    &mut reopened_event_port,
                    &PaleDerivedOutboxPageRead::new(coordinate.clone(), None, 8).unwrap(),
                )
                .unwrap();
                assert_eq!(
                    pending.entries().len(),
                    5,
                    "two occurrences, two frontiers, and one edge remain exact pending publications"
                );
                for (index, entry) in pending.entries().iter().enumerate() {
                    assert!(pending.entries()[..index].iter().all(|prior| {
                        prior.publication_ref() != entry.publication_ref()
                            || prior.publication_sha256() != entry.publication_sha256()
                    }));
                }
            },
        )
    });
}
