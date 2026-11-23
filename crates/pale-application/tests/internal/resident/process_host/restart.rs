//! Read-only cold-restart classification through one exact process host.

use super::provider::{
    with_retained_process_provider_cancellation, RetainedProcessProviderCancellationFixture,
};
use super::*;
use crate::local_operation_start_fixture::signature;
use crate::resident::consultation::tests::with_issued_delegate_child_named;
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_process_host_local_cancellation_restart_fixtures,
    with_process_host_restart_dispatch_fixtures, with_process_host_restart_fixtures,
    ProcessHostRestartFixture,
};
use crate::resident::process_host::restart::*;
use crate::resident::{
    IssuedLocalOperation, OperationCheckpointAncestryLimits, OperationObservationRecoveryLimits,
    OperationObservationSelection, RecoveredLocalOperation, RecoveredOperationControl,
    RecoveredOperationObservation,
};
use pale_core::records::{
    OperationActivePhaseV1, OperationObservationAdmissionContextV1, OperationObservationV1,
    OperationObservationWireV1, ParticipantRefV1, ProviderBindingRefV1,
    SessionAdmissionLeaseWireV1, SignedResidentAdmissionContextV1,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    OperationCustodyCandidate, OperationCustodyRecordKind, SessionLeaseBody, SessionLeaseBodyRead,
    SessionLeaseBodyRetention, SessionLeaseCustodyPort,
};
use std::cell::Cell;
use std::rc::Rc;

pub(super) fn host_with_generation<'store>(
    port: &'store MemoryAuthorityStore,
    selector: &ResidentDeploymentAuthoritySelector,
    selected_store: &AuthorityStoreCoordinate,
    generation: crate::resident::AdmittedCompiledGeneration,
) -> ResidentProcessHost<'store, UnprovedHeldWriter> {
    let mut application = ready_application(port, selected_store.clone(), 71);
    application.fixture_retain_generation(generation);
    let deployment =
        unproved_deployment_for_selector(selector, selected_store.clone(), application);
    ResidentProcessHost::new(
        unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
        vec![deployment],
        NonZeroUsize::new(1).expect("one deployment"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one exact restart deployment")
}

pub(super) fn host_without_generation<'store>(
    port: &'store MemoryAuthorityStore,
    selector: &ResidentDeploymentAuthoritySelector,
    selected_store: &AuthorityStoreCoordinate,
) -> ResidentProcessHost<'store, UnprovedHeldWriter> {
    let application = ready_application(port, selected_store.clone(), 72);
    let deployment =
        unproved_deployment_for_selector(selector, selected_store.clone(), application);
    ResidentProcessHost::new(
        unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
        vec![deployment],
        NonZeroUsize::new(1).expect("one deployment"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one exact restart deployment")
}

pub(super) fn restart_input<'a>(
    recovery: OperationControlRecoveryInput<'a>,
    maximum_selection_bytes: usize,
) -> ResidentProcessOperationRestartInput<'a> {
    ResidentProcessOperationRestartInput {
        committed_observation_limits: None,
        operation: recovery.operation.context,
        maximum_parent_depth: recovery.operation.maximum_parent_depth,
        current_head: ResidentProcessRestartReadLimits {
            observation: recovery.observation_limits,
            checkpoints: recovery.checkpoint_limits,
            membership: recovery.membership_limits,
        },
        maximum_selection_bytes,
        a2a_task: None,
        a2a_cancel_task: None,
    }
}

pub(super) fn association<'a>(
    signing: &'a SigningAuthorityBindingV1,
    input: &ResidentProcessOperationRestartInput<'a>,
) -> ResidentOperationControlAssociation<'a> {
    ResidentOperationControlAssociation {
        current_request_authority: signing,
        historical_handle_authority: input.operation.handle_signing_authority,
        recovery_store: input.operation.store,
        checkpoint_authority: input.operation.checkpoint_authority,
    }
}

pub(in crate::resident) fn request_admission_context(
    request: &pale_core::records::CommonRequestEnvelopePreimageV1,
) -> pale_core::records::CommonRequestAdmissionContextV1<'_> {
    pale_core::records::CommonRequestAdmissionContextV1 {
        expected_schema_sha256: &request.envelope_schema_sha256,
        expected_body_schema_id: &request.body.schema_id,
        expected_body_schema_sha256: &request.body.schema_sha256,
        caller: &request.caller,
        signing_authority: &request.signing_authority_binding,
        route: &request.route,
        session_admission: &request.session_admission,
        operation_target: request.operation_target.as_ref(),
        execution_owner_fence: None,
        federation_claim: None,
        workspace_id: &request.workspace_id,
        capability_id: &request.capability_id,
        operation: &request.operation,
        scope: &request.scope,
        risk_class: &request.risk_class,
        resource_constraints: &request.resource_constraints,
        expected_precondition: None,
        idempotency_key: &request.idempotency_key,
        issued_at: request.issued_at,
        expires_at: request.expires_at,
        generation: request.generation.as_ref(),
        fence_token: request.fence_token.as_ref(),
    }
}

pub(in crate::resident) fn initial_operation_control(
    operation: &IssuedLocalOperation,
    request: CommonRequestEnvelopeV1,
    candidate: &OperationCustodyCandidate,
    checkpoint: ControlStateCheckpointV1,
    signing: &SigningAuthorityBindingV1,
) -> RecoveredOperationControl {
    let store = candidate.store().clone();
    let record = candidate
        .record(OperationCustodyRecordKind::OperationObservation)
        .expect("issued start retains its initial observation");
    let wire: OperationObservationWireV1 = serde_json::from_value(
        pale_core::canon::parse_strict(record.canonical_record_jcs().as_slice())
            .expect("initial observation JSON"),
    )
    .expect("initial observation wire");
    let observation_schema =
        crate::local_operation_start_fixture::schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let observation = OperationObservationV1::admit_contextual_wire(
        wire,
        &OperationObservationAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &observation_schema,
                signing_authority: signing,
            },
            handle_ref: operation.handle().0,
            handle: operation.handle().1,
            predecessor: None,
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("initial observation admission");
    let observation_ref = record.coordinate().record_ref().clone();
    let checkpoint_ref = candidate
        .checkpoint()
        .successor_record()
        .coordinate()
        .record_ref()
        .clone();
    RecoveredOperationControl::fixture(
        RecoveredLocalOperation::fixture(operation, request, store.clone()),
        RecoveredOperationObservation::fixture(
            operation,
            (observation_ref.clone(), observation.clone()),
            (observation_ref, observation),
            (checkpoint_ref, checkpoint),
            store,
        ),
    )
}

pub(in crate::resident) struct ColdRestartSessions(SessionLeaseBody);

impl SessionLeaseCustodyPort for ColdRestartSessions {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, pale_ports::ControlStoreError> {
        panic!("restart recovery cannot retain or renew a session")
    }

    fn read(
        &self,
        query: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, pale_ports::ControlStoreError> {
        assert_eq!(query.store(), self.0.store());
        Ok(Some(self.0.clone()))
    }
}

pub(in crate::resident) fn cold_restart_sessions(
    parents: crate::local_operation_start_fixture::RecoveryParents<'_>,
    store: &AuthorityStoreCoordinate,
) -> (ColdRestartSessions, RecoveryResolver) {
    let session = parents.session.lease();
    let body = SessionLeaseBody::new(
        store.clone(),
        session.session_admission_lease_digest().clone(),
        session.schema_id().clone(),
        session.schema_sha256().clone(),
        pale_ports::ControlRecordBytes::new(
            pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(session))
                .expect("session lease wire")
                .into_bytes(),
        )
        .expect("bounded session lease"),
    );
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(parents.current.generation.compiled_generation);
    resolver.insert_active(parents.current.generation.active_head);
    resolver.insert_fence(parents.current.generation.activation_fence);
    resolver.insert_effective_profile(parents.current.generation.effective_profile);
    (ColdRestartSessions(body), resolver)
}

#[test]
fn pairwise_delegate_resolving_requires_its_exact_parent_without_consult_state() {
    let (mut generations, profile) =
        crate::resident::operation_observation_custody::tests::local_start::
            host_source_generation_copies(2);
    let restart_generation = generations.pop().expect("restart generation copy");
    let generation = generations.pop().expect("source generation copy");
    with_issued_delegate_child_named(
        &generation,
        &profile,
        "pairwise-resolving-reentry",
        |child,
         parent,
         parent_candidate,
         parent_checkpoint,
         _lease_custody,
         _generation,
         parents| {
            let store = child.initial_candidate.store().clone();
            let selector = selector_from_signing_authority(parents.signing);
            let parent_control = initial_operation_control(
                parent,
                child.parent_request.1.clone(),
                &parent_candidate,
                parent_checkpoint,
                parents.signing,
            );
            let child_control = initial_operation_control(
                &child.operation,
                child.request.clone(),
                &child.initial_candidate,
                child.checkpoint.clone(),
                parents.signing,
            );
            assert!(matches!(
                child_control.observation().selected_head().1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving
                }
            ));
            let parent_start = ResidentProcessRecoveredOperationStart {
                selector: selector.clone(),
                control: parent_control,
                generation_digest: parent.handle().1.compiled_generation_sha256().clone(),
                lineage: ResidentProcessOperationLineage::Root,
            };
            let application_store = MemoryAuthorityStore::default();
            let mut application = ready_application(&application_store, store.clone(), 84);
            application.fixture_retain_generation(restart_generation);

            assert!(matches!(
                admit_restart_lineage(
                    &application,
                    ResidentProcessRestartLineageInput::Root,
                    &selector,
                    &store,
                    &child_control,
                ),
                Err(ResidentProcessOperationRestartError::State)
            ));
            let admitted = admit_restart_lineage(
                &application,
                ResidentProcessRestartLineageInput::PairwiseDelegate {
                    parent: &parent_start,
                },
                &selector,
                &store,
                &child_control,
            )
            .expect("the exact pairwise Delegate lineage");
            assert!(admitted.admits(child.operation.handle().0, child.operation.handle().1));
            assert!(admitted.consult_child().is_none());

            let mut wrong_selector = selector.clone();
            wrong_selector.runtime_authority_ref =
                crate::local_operation_start_fixture::record_ref("other-runtime-authority");
            assert!(matches!(
                admit_restart_lineage(
                    &application,
                    ResidentProcessRestartLineageInput::PairwiseDelegate {
                        parent: &parent_start,
                    },
                    &wrong_selector,
                    &store,
                    &child_control,
                ),
                Err(ResidentProcessOperationRestartError::State)
            ));
        },
    );
}

#[test]
fn cold_restart_recovers_pairwise_delegate_resolving_without_consult_state() {
    let (mut generations, profile) =
        crate::resident::operation_observation_custody::tests::local_start::
            host_source_generation_copies(2);
    let restart_generation = generations.pop().expect("restart generation copy");
    let generation = generations.pop().expect("source generation copy");
    with_issued_delegate_child_named(
        &generation,
        &profile,
        "pairwise-resolving-cold-reentry",
        |child,
         parent,
         parent_candidate,
         parent_checkpoint,
         lease_custody,
         _generation,
         parents| {
            let store = child.initial_candidate.store().clone();
            let selector = selector_from_signing_authority(parents.signing);
            let (sessions, resolver) = cold_restart_sessions(parents, &store);
            let checkpoint_authority = super::a2a_cancel_task::checkpoint_signer(&child.checkpoint);
            let port = MemoryAuthorityStore::default();
            let host = host_with_generation(&port, &selector, &store, restart_generation);
            crate::resident::operation_observation_custody::tests::native_execution::
                with_service_custody_faults(
                    &parent_checkpoint,
                    Some(&child.initial_candidate),
                    vec![parent_candidate],
                    None,
                    vec![lease_custody.parent, lease_custody.child],
                    0,
                    0,
                    0,
                    0,
                    0,
                    &[],
                    &[],
                    &[],
                    &[],
                    &[],
                    &[],
                    Vec::new(),
                    false,
                    None,
                    Rc::new(std::cell::RefCell::new(None)),
                    |custody, anchor| {
                        let profile = local_operation_start_fixture::profile();
                        let observation_limits = OperationObservationRecoveryLimits::new(
                            16,
                            1 << 20,
                            1 << 20,
                        )
                        .expect("bounded observation recovery");
                        let checkpoint_limits =
                            OperationCheckpointAncestryLimits::new(16, 1 << 20, 16)
                                .expect("bounded checkpoint recovery");
                        let membership_limits = pale_ports::AccumulatorMembershipLimits::new(
                            64, 100_000, 1_000_000,
                        )
                        .expect("bounded membership recovery");
                        let recovery_context = LocalOperationRecoveryContext {
                            store: &store,
                            profile: &profile,
                            handle_signing_authority: parents.signing,
                            checkpoint_authority: &checkpoint_authority,
                            historical: &resolver,
                            maximum_record_bytes: 1 << 20,
                            maximum_operation_lease_bytes: 1 << 20,
                            maximum_session_lease_bytes: 1 << 20,
                            membership_limits,
                            observation_limits,
                            checkpoint_limits,
                        };
                        let limits = ResidentProcessRestartReadLimits {
                            observation: observation_limits,
                            checkpoints: checkpoint_limits,
                            membership: membership_limits,
                        };
                        let parent_evidence =
                            crate::resident::operation_custody::read_operation_start_for_request(
                                custody,
                                &store,
                                &child.parent_request.1,
                                1 << 20,
                            )
                            .expect("parent start read")
                            .expect("retained parent start");
                        let child_evidence =
                            crate::resident::operation_custody::read_operation_start_for_request(
                                custody,
                                &store,
                                &child.request,
                                1 << 20,
                            )
                            .expect("child start read")
                            .expect("retained child start");
                        let child_handle = child.operation.handle().1;
                        let parent_handle = parent.handle().1;
                        let mut admit_request =
                            |reference: &ResidentRecordRefV1,
                             wire: CommonRequestEnvelopeWireV1| {
                                let expected = if reference
                                    == child.operation.request_coordinate().record_ref()
                                {
                                    &child.request
                                } else if reference == child.parent_request.0.record_ref() {
                                    &child.parent_request.1
                                } else {
                                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                                        record: "common request envelope",
                                        invariant: "known retained request coordinate",
                                    });
                                };
                                let preimage = expected.to_preimage();
                                CommonRequestEnvelopeV1::admit_contextual_wire(
                                    wire,
                                    &request_admission_context(&preimage),
                                    |_, _, actual| {
                                        assert_eq!(actual, expected.caller_signature());
                                        Ok(())
                                    },
                                )
                            };
                        let mut admit_handle_authority =
                            |preimage: &OperationHandlePreimageV1,
                             recovered_parent: Option<&OperationHandleV1>| {
                                if preimage.operation_handle_id
                                    == *child_handle.operation_handle_id()
                                {
                                    let recovered_parent = recovered_parent.ok_or(
                                        InvalidResidentRecordV1::ContextualAdmission {
                                            record: "operation handle",
                                            invariant: "exact recovered Delegate parent",
                                        },
                                    )?;
                                    if recovered_parent != parent_handle {
                                        return Err(
                                            InvalidResidentRecordV1::ContextualAdmission {
                                                record: "operation handle",
                                                invariant: "exact recovered Delegate parent",
                                            },
                                        );
                                    }
                                } else if preimage.operation_handle_id
                                    != *parent_handle.operation_handle_id()
                                    || recovered_parent.is_some()
                                {
                                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                                        record: "operation handle",
                                        invariant: "one admitted Consult parent",
                                    });
                                }
                                Ok(())
                            };
                        let mut verify_handle_signature =
                            |_: &[u8],
                             _: &OperationHandlePreimageV1,
                             _: &OperationHandleDigest,
                             actual: &CanonicalSignatureV1| {
                                assert_eq!(actual, &signature());
                                Ok(())
                            };
                        let mut admit_checkpoint =
                            |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                                ControlStateCheckpointV1::admit_signed_wire(
                                    wire,
                                    child.checkpoint.schema_sha256(),
                                    &checkpoint_authority,
                                    |_, _, _, actual| {
                                        assert_eq!(actual, &signature());
                                        Ok(())
                                    },
                                )
                            };
                        let mut resolve_observation_signer =
                            |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
                                Ok(parents.signing.clone())
                            };
                        let mut admit_observation_state =
                            |_: &OperationObservationStateV1| Ok(());
                        let mut admit_rotation =
                            |_: &ResidentRecordRefV1,
                             _: &ArtifactDigest,
                             _: &ResidentRecordRefV1,
                             _: &ArtifactDigest| { Ok(()) };
                        let mut verify_observation_signature =
                            |_: &[u8],
                             _: &OperationObservationPreimageV1,
                             _: &OperationObservationDigest,
                             actual: &CanonicalSignatureV1| {
                                assert_eq!(actual, &signature());
                                Ok(())
                            };
                        let mut verify_checkpoint_signature =
                            |_: &[u8],
                             _: &ControlStateCheckpointPreimageV1,
                             _: &ControlStateCheckpointDigest,
                             actual: &CanonicalSignatureV1| {
                                assert_eq!(actual, &signature());
                                Ok(())
                            };
                        let mut admission = OperationControlRecoveryAdmission {
                            admit_request: &mut admit_request,
                            admit_handle_authority: &mut admit_handle_authority,
                            verify_handle_signature: &mut verify_handle_signature,
                            admit_checkpoint: &mut admit_checkpoint,
                            resolve_observation_signer: &mut resolve_observation_signer,
                            admit_observation_state: &mut admit_observation_state,
                            admit_rotation: &mut admit_rotation,
                            verify_observation_signature: &mut verify_observation_signature,
                            verify_checkpoint_signature: &mut verify_checkpoint_signature,
                        };
                        let parent = host
                            .classify_existing_operation(
                                ResidentProcessExistingOperationStart {
                                    selector: selector.clone(),
                                    evidence: Box::new(parent_evidence),
                                },
                                ResidentOperationControlAssociation {
                                    current_request_authority: parents.signing,
                                    historical_handle_authority: parents.signing,
                                    recovery_store: &store,
                                    checkpoint_authority: &checkpoint_authority,
                                },
                                ResidentDeploymentOperationAccess {
                                    store: &store,
                                    custody,
                                    sessions: &sessions,
                                    anchor,
                                },
                                ResidentProcessOperationRestartInput {
                                    committed_observation_limits: None,
                                    operation: recovery_context,
                                    maximum_parent_depth: 0,
                                    current_head: limits,
                                    maximum_selection_bytes: 1 << 20,
                                    a2a_task: None,
                                    a2a_cancel_task: None,
                                },
                                OperationControlRecoveryAdmission {
                                    admit_request: &mut *admission.admit_request,
                                    admit_handle_authority: &mut *admission
                                        .admit_handle_authority,
                                    verify_handle_signature: &mut *admission
                                        .verify_handle_signature,
                                    admit_checkpoint: &mut *admission.admit_checkpoint,
                                    resolve_observation_signer: &mut *admission
                                        .resolve_observation_signer,
                                    admit_observation_state: &mut *admission
                                        .admit_observation_state,
                                    admit_rotation: &mut *admission.admit_rotation,
                                    verify_observation_signature: &mut *admission
                                        .verify_observation_signature,
                                    verify_checkpoint_signature: &mut *admission
                                        .verify_checkpoint_signature,
                                },
                            )
                            .expect("cold parent classification");
                        let parent = match parent {
                            ResidentProcessOperationRestartDisposition::Consult(parent) => parent,
                            ResidentProcessOperationRestartDisposition::OperationResolution(
                                parent,
                            ) => Box::new(parent.start),
                            other => {
                                drop(other);
                                panic!("pairwise Delegate has one recovered Consult parent")
                            }
                        };
                        let child = host
                            .classify_delegate_child_operation(
                                ResidentProcessExistingOperationStart {
                                    selector: selector.clone(),
                                    evidence: Box::new(child_evidence),
                                },
                                ResidentOperationControlAssociation {
                                    current_request_authority: parents.signing,
                                    historical_handle_authority: parents.signing,
                                    recovery_store: &store,
                                    checkpoint_authority: &checkpoint_authority,
                                },
                                ResidentDeploymentOperationAccess {
                                    store: &store,
                                    custody,
                                    sessions: &sessions,
                                    anchor,
                                },
                                ResidentProcessOperationRestartInput {
                                    committed_observation_limits: None,
                                    operation: recovery_context,
                                    maximum_parent_depth: 0,
                                    current_head: limits,
                                    maximum_selection_bytes: 1 << 20,
                                    a2a_task: None,
                                    a2a_cancel_task: None,
                                },
                                &mut admission,
                                &parent,
                            )
                            .expect("cold pairwise Delegate classification");
                        let ResidentProcessOperationRestartDisposition::OperationResolution(
                            recovered,
                        ) = child
                        else {
                            panic!("Resolving Delegate has one pre-attempt continuation")
                        };
                        assert_eq!(
                            recovered.phase,
                            ResidentProcessOperationResolutionPhase::Resolving
                        );
                        assert!(recovered.start.lineage.consult_child().is_none());
                        assert_eq!(host.provider_routes.reserved, 0);
                        assert_eq!(host.operations.outstanding_task_reservations(), 0);
                    },
                );
        },
    );
}

#[test]
fn pre_call_cancellation_keeps_its_exact_request_and_predecessor_phase() {
    let phases = Rc::new(std::cell::RefCell::new(BTreeSet::new()));
    let observed_phases = Rc::clone(&phases);
    with_process_host_local_cancellation_restart_fixtures(&mut move |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let input = restart_input(recovery, 1024 * 1024);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &input),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
            .expect("retained pre-call cancellation classifies");
        let ResidentProcessOperationRestartDisposition::LocalCancellationRequested(recovered) =
            disposition
        else {
            panic!("pre-call cancellation has one local continuation")
        };
        let observation = recovered.start.control.observation();
        let (head_ref, head) = observation.selected_head();
        let (predecessor_ref, predecessor) = observation
            .selected_predecessor()
            .expect("cancellation retains its exact predecessor");
        let OperationObservationStateV1::CancellationRequested { request } = head.state() else {
            panic!("current head remains the cancellation request")
        };
        assert_eq!(request, &recovered.request);
        assert_eq!(head.predecessor_ref().as_ref(), Some(predecessor_ref));
        assert_eq!(
            head.predecessor_sha256().as_ref(),
            Some(predecessor.canonical_operation_observation_sha256())
        );
        assert_ne!(head_ref, predecessor_ref);
        let phase = match predecessor.state() {
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Resolving,
            } => "resolving",
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ContextBound { .. },
            } => "context_bound",
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ProviderResolved { .. },
            } => "provider_resolved",
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::DispatchPrepared { .. },
            } => "dispatch_prepared",
            _ => panic!("fixture must retain one pre-call active predecessor"),
        };
        assert!(observed_phases.borrow_mut().insert(phase));
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
    });
    assert_eq!(
        phases.borrow().iter().copied().collect::<Vec<_>>(),
        [
            "context_bound",
            "dispatch_prepared",
            "provider_resolved",
            "resolving"
        ]
    );
}

#[test]
fn generation_independent_restart_states_classify_after_generation_eviction() {
    let terminal = Rc::new(Cell::new(0));
    let recovery = Rc::new(Cell::new(0));
    let consultation = Rc::new(Cell::new(0));
    let operation_resolution = Rc::new(Cell::new(0));
    let provider_resolved = Rc::new(Cell::new(0));
    let cancellation_predecessor = Rc::new(Cell::new(0));
    let counted_terminal = Rc::clone(&terminal);
    let counted_recovery = Rc::clone(&recovery);
    let counted_consultation = Rc::clone(&consultation);
    let counted_operation_resolution = Rc::clone(&operation_resolution);
    let counted_provider_resolved = Rc::clone(&provider_resolved);
    let counted_cancellation_predecessor = Rc::clone(&cancellation_predecessor);
    with_process_host_restart_fixtures(&mut move |fixture: ProcessHostRestartFixture<'_>| {
        let ProcessHostRestartFixture {
            recovery: recovery_input,
            admission,
            signing_authority,
            current: _,
            evidence,
            generation: _,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery_input.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_without_generation(&port, &selector, &selected_store);
        let input = restart_input(recovery_input, 1024 * 1024);
        let result = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &input),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
            .unwrap_or_else(|error| panic!("exact restart classification: {error}"));
        let start = match result {
            ResidentProcessOperationRestartDisposition::NativeHookResponse(_) => {
                panic!("action/provider fixture cannot become native hook response recovery")
            }
            ResidentProcessOperationRestartDisposition::Terminal(start) => {
                counted_terminal.set(counted_terminal.get() + 1);
                start
            }
            ResidentProcessOperationRestartDisposition::RecoveryOnly(start) => {
                counted_recovery.set(counted_recovery.get() + 1);
                start
            }
            ResidentProcessOperationRestartDisposition::LocalCancellationRequested(recovered) => {
                Box::new(recovered.start)
            }
            ResidentProcessOperationRestartDisposition::OperationResolution(recovered) => {
                counted_operation_resolution.set(counted_operation_resolution.get() + 1);
                Box::new(recovered.start)
            }
            ResidentProcessOperationRestartDisposition::NativePrepared(attempt)
            | ResidentProcessOperationRestartDisposition::ProviderPrepared(attempt) => {
                Box::new(attempt.start)
            }
            ResidentProcessOperationRestartDisposition::ProviderResolved(recovered) => {
                counted_provider_resolved.set(counted_provider_resolved.get() + 1);
                Box::new(recovered.start)
            }
            ResidentProcessOperationRestartDisposition::Consult(start) => {
                counted_consultation.set(counted_consultation.get() + 1);
                start
            }
            ResidentProcessOperationRestartDisposition::Native(attempt)
            | ResidentProcessOperationRestartDisposition::Provider(attempt)
            | ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(attempt) => {
                if attempt.dispatch == ResidentProcessAttemptObservation::CancellationPredecessor {
                    counted_cancellation_predecessor
                        .set(counted_cancellation_predecessor.get() + 1);
                }
                Box::new(attempt.start)
            }
            ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(
                recovered,
            ) => Box::new(recovered.attempt.start),
            ResidentProcessOperationRestartDisposition::A2aTask(recovered) => {
                Box::new(recovered.attempt.start)
            }
        };
        if matches!(
            start.control.observation().selected_head().1.state(),
            OperationObservationStateV1::CancellationRequested { .. }
        ) {
            counted_cancellation_predecessor.set(counted_cancellation_predecessor.get() + 1);
        }
        assert_eq!(start.selector, selector);
        assert_eq!(
            &start.generation_digest,
            start
                .control
                .operation()
                .operation()
                .handle()
                .1
                .compiled_generation_sha256()
        );
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
    });
    assert_eq!(terminal.get(), 1);
    assert_eq!(consultation.get(), 1);
    assert_eq!(operation_resolution.get(), 1);
    assert_eq!(recovery.get(), 1);
    assert_eq!(provider_resolved.get(), 0);
    assert_eq!(cancellation_predecessor.get(), 1);
}

#[test]
fn dispatched_restart_never_falls_back_to_another_generation() {
    let calls = Rc::new(Cell::new(0));
    let counted_calls = Rc::clone(&calls);
    with_process_host_restart_dispatch_fixtures(&mut move |fixture: ProcessHostRestartFixture<
        '_,
    >| {
        if counted_calls.replace(counted_calls.get() + 1) != 0 {
            return;
        }
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            evidence,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let application = ready_application(&port, selected_store.clone(), 72);
        let deployment =
            unproved_deployment_for_selector(&selector, selected_store.clone(), application);
        let host = ResidentProcessHost::new(
            unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
            vec![deployment],
            NonZeroUsize::new(1).expect("one deployment"),
            ResidentOperationHost::new(task_state()),
        )
        .expect("empty restart deployment");
        let input = restart_input(recovery, 1024 * 1024);
        let result = host.classify_existing_operation(
            ResidentProcessExistingOperationStart {
                selector,
                evidence: Box::new(evidence),
            },
            association(signing_authority, &input),
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            input,
            admission,
        );
        assert!(matches!(
            result,
            Err(ResidentProcessOperationRestartError::GenerationUnavailable)
        ));
    });
    assert_eq!(calls.get(), 3);
}

#[test]
fn zero_selection_limit_and_wrong_authority_fail_before_recovery() {
    let calls = Rc::new(Cell::new(0));
    let counted_calls = Rc::clone(&calls);
    with_process_host_restart_fixtures(&mut move |fixture: ProcessHostRestartFixture<'_>| {
        let ordinal = counted_calls.replace(counted_calls.get() + 1);
        if ordinal > 1 {
            return;
        }
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            current: _,
            evidence,
            generation: _,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(
            &port,
            &selector,
            &selected_store,
            crate::resident::operation_observation_custody::tests::local_start::source_generation()
                .0,
        );
        let input = restart_input(recovery, if ordinal == 0 { 0 } else { 1024 * 1024 });
        let exact = association(signing_authority, &input);
        let result = if ordinal == 0 {
            host.classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                exact,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
        } else {
            let foreign = owner_native_signing_authority(selector.principal_id.clone());
            host.classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                ResidentOperationControlAssociation {
                    current_request_authority: &foreign,
                    historical_handle_authority: input.operation.handle_signing_authority,
                    recovery_store: input.operation.store,
                    checkpoint_authority: input.operation.checkpoint_authority,
                },
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
        };
        if ordinal == 0 {
            assert!(matches!(
                result,
                Err(ResidentProcessOperationRestartError::Bounds)
            ));
        } else {
            assert!(matches!(
                result,
                Err(ResidentProcessOperationRestartError::Selection(
                    ResidentProcessHostSelectionError::CurrentRequestAuthority
                ))
            ));
        }
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
    });
    assert_eq!(calls.get(), 5);
}

#[test]
fn cancellation_predecessor_uses_the_original_checkpoint_walk_once() {
    let calls = Rc::new(Cell::new(0));
    let counted_calls = Rc::clone(&calls);
    with_process_host_restart_fixtures(&mut move |fixture: ProcessHostRestartFixture<'_>| {
        let ordinal = counted_calls.replace(counted_calls.get() + 1);
        if ordinal != 4 {
            return;
        }
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            current: _,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        custody.change_head_after_memberships(3);
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let input = restart_input(recovery, 1024 * 1024);
        host.classify_existing_operation(
            ResidentProcessExistingOperationStart {
                selector,
                evidence: Box::new(evidence),
            },
            association(signing_authority, &input),
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            input,
            admission,
        )
        .expect("the retained cancellation predecessor needs no second checkpoint walk");
        assert_eq!(custody.access_counts(), (20, 2, 3, 3));
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
    });
    assert_eq!(calls.get(), 5);
}

#[test]
fn cancellation_predecessor_does_not_repeat_start_evidence_reads() {
    let calls = Rc::new(Cell::new(0));
    let counted_calls = Rc::clone(&calls);
    with_process_host_restart_fixtures(&mut move |fixture: ProcessHostRestartFixture<'_>| {
        let ordinal = counted_calls.replace(counted_calls.get() + 1);
        if ordinal != 4 {
            return;
        }
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            current: _,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        custody.change_start_after_memberships(3);
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let input = restart_input(recovery, 1024 * 1024);
        host.classify_existing_operation(
            ResidentProcessExistingOperationStart {
                selector,
                evidence: Box::new(evidence),
            },
            association(signing_authority, &input),
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            input,
            admission,
        )
        .expect("the retained cancellation predecessor needs no second start-evidence read");
        assert_eq!(custody.access_counts(), (20, 2, 3, 3));
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
    });
    assert_eq!(calls.get(), 5);
}

#[test]
fn dispatched_restart_requires_cancellation_request_custody() {
    let calls = Rc::new(Cell::new(0));
    let counted_calls = Rc::clone(&calls);
    with_process_host_restart_dispatch_fixtures(&mut move |fixture| {
        let ordinal = counted_calls.replace(counted_calls.get() + 1);
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            current: _,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let input = restart_input(recovery, 1024 * 1024);
        let result = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &input),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
            .unwrap_or_else(|error| {
                panic!("exact dispatched restart classification {ordinal}: {error}")
            });
        let attempt = match (ordinal, result) {
            (0, ResidentProcessOperationRestartDisposition::Native(attempt)) => {
                assert_eq!(
                    attempt.dispatch,
                    ResidentProcessAttemptObservation::CurrentHead
                );
                Some(attempt)
            }
            (1, ResidentProcessOperationRestartDisposition::Provider(attempt)) => {
                assert_eq!(
                    attempt.dispatch,
                    ResidentProcessAttemptObservation::CurrentHead
                );
                Some(attempt)
            }
            (2, ResidentProcessOperationRestartDisposition::RecoveryOnly(_)) => None,
            (_, _) => panic!("wrong restart disposition for dispatch fixture {ordinal}"),
        };
        if let Some(attempt) = attempt {
            assert_eq!(
                &attempt.selection,
                custody
                    .attempt_selection()
                    .expect("dispatch fixture selection")
            );
        }
        assert_eq!(custody.selection_reads(), 1);
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
    });
    assert_eq!(calls.get(), 3);
}

#[test]
fn provider_resolution_requires_exact_generation_binding_and_participant() {
    let calls = Rc::new(Cell::new(0));
    let counted_calls = Rc::clone(&calls);
    with_process_host_restart_dispatch_fixtures(&mut move |fixture| {
        let ordinal = counted_calls.replace(counted_calls.get() + 1);
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let input = restart_input(recovery, 1024 * 1024);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &input),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
            .expect("exact retained dispatch classifies");
        let ResidentProcessOperationRestartDisposition::Provider(attempt) = disposition else {
            assert_ne!(ordinal, 1, "provider dispatch keeps its provider owner");
            return;
        };
        assert_eq!(ordinal, 1, "only the provider fixture enters this proof");
        let operation = attempt.start.control.operation().operation();
        let generation = host.deployments[0]
            .application
            .generation(operation.handle().1.compiled_generation_sha256())
            .expect("operation generation remains retained");
        let provider = generation
            .provider_binding("provider-binding-claude-code")
            .expect("operation provider remains retained");
        let exact = ProviderBindingRefV1 {
            provider_binding_id: provider.id.clone(),
            provider_binding_identity: provider.binding_identity.clone(),
        };
        assert!(provider_resolution_matches_generation(
            generation,
            operation,
            operation.historical_operation_lease(),
            &exact,
            Some(&provider.participant),
        ));

        let mut wrong_digest = exact.clone();
        wrong_digest
            .provider_binding_identity
            .provider_binding_sha256 =
            ArtifactDigest::hash_bytes(b"same provider ID with another digest");
        assert!(!provider_resolution_matches_generation(
            generation,
            operation,
            operation.historical_operation_lease(),
            &wrong_digest,
            Some(&provider.participant),
        ));
        let wrong_participant = ParticipantRefV1::Application {
            application_id: "foreign-provider-participant".into(),
        };
        assert!(!provider_resolution_matches_generation(
            generation,
            operation,
            operation.historical_operation_lease(),
            &exact,
            Some(&wrong_participant),
        ));
    });
    assert_eq!(calls.get(), 3);
}

#[test]
fn retained_local_provider_cancellation_remains_provider_recovery_work() {
    with_retained_process_provider_cancellation(
        |RetainedProcessProviderCancellationFixture {
             host,
             selector,
             store,
             operation,
             request,
             generation: _,
             selection,
             dispatched: _,
             cancellation_request,
             checkpoint,
             signing,
             custody,
             sessions,
             anchor,
             parents,
         }| {
            let evidence = crate::resident::operation_custody::read_operation_start_for_request(
                custody,
                &store,
                request,
                1 << 20,
            )
            .expect("operation start read")
            .expect("retained operation start");
            let expected_handle = crate::resident::record_projection::coordinate(
                operation.handle().0,
                operation
                    .handle()
                    .1
                    .canonical_operation_handle_sha256()
                    .as_str(),
            )
            .expect("provider operation handle coordinate");
            let checkpoint_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
                authority_ref: checkpoint.checkpoint_signing_authority_ref(),
                authority_sha256: checkpoint.checkpoint_signing_authority_sha256(),
                principal_id: checkpoint.issuer_principal_id(),
                control_store_id: checkpoint.control_store_id(),
                control_incarnation_id: checkpoint.control_incarnation_id(),
                checkpoint_domain_id: checkpoint.checkpoint_domain_id(),
                checkpoint_profile_ref: checkpoint.checkpoint_profile_ref(),
                checkpoint_profile_sha256: checkpoint.checkpoint_profile_sha256(),
                subject_key_id: checkpoint.issuer_key_id(),
                subject_key_epoch: *checkpoint.issuer_key_epoch(),
            };
            let mut resolver = RecoveryResolver::default();
            resolver.insert_generation(parents.current.generation.compiled_generation);
            resolver.insert_active(parents.current.generation.active_head);
            resolver.insert_fence(parents.current.generation.activation_fence);
            resolver.insert_effective_profile(parents.current.generation.effective_profile);
            let profile = local_operation_start_fixture::profile();
            let recovery = OperationControlRecoveryInput {
                operation: LocalOperationHandleRecoveryInput {
                    context: LocalOperationRecoveryContext {
                        store: &store,
                        profile: &profile,
                        handle_signing_authority: signing,
                        checkpoint_authority: &checkpoint_authority,
                        historical: &resolver,
                        maximum_record_bytes: 1 << 20,
                        maximum_operation_lease_bytes: 1 << 20,
                        maximum_session_lease_bytes: 1 << 20,
                        membership_limits: pale_ports::AccumulatorMembershipLimits::new(
                            64, 100_000, 1_000_000,
                        )
                        .expect("bounded membership recovery"),
                        observation_limits: OperationObservationRecoveryLimits::new(
                            16,
                            1 << 20,
                            1 << 20,
                        )
                        .expect("bounded observation recovery"),
                        checkpoint_limits: OperationCheckpointAncestryLimits::new(16, 1 << 20, 16)
                            .expect("bounded checkpoint recovery"),
                    },
                    expected_handle: &expected_handle,
                    maximum_parent_depth: 0,
                },
                observation: OperationObservationSelection::SelectedHead,
                observation_limits: OperationObservationRecoveryLimits::new(16, 1 << 20, 1 << 20)
                    .expect("bounded observation recovery"),
                checkpoint_limits: OperationCheckpointAncestryLimits::new(16, 1 << 20, 16)
                    .expect("bounded checkpoint recovery"),
                membership_limits: pale_ports::AccumulatorMembershipLimits::new(
                    64, 100_000, 1_000_000,
                )
                .expect("bounded membership recovery"),
            };
            let original_preimage = request.to_preimage();
            let cancellation_preimage = cancellation_request.1.to_preimage();
            let mut admit_request =
                |reference: &ResidentRecordRefV1, wire: CommonRequestEnvelopeWireV1| {
                    let (expected, context) =
                        if reference == operation.request_coordinate().record_ref() {
                            (request, request_admission_context(&original_preimage))
                        } else {
                            assert_eq!(reference, cancellation_request.0.record_ref());
                            (
                                &cancellation_request.1,
                                request_admission_context(&cancellation_preimage),
                            )
                        };
                    CommonRequestEnvelopeV1::admit_contextual_wire(
                        wire,
                        &context,
                        |_, _, actual| {
                            assert_eq!(actual, expected.caller_signature());
                            Ok(())
                        },
                    )
                };
            let mut admit_handle_authority =
                |_: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
                    assert!(parent.is_none());
                    Ok(())
                };
            let mut verify_handle_signature =
                |_: &[u8],
                 _: &OperationHandlePreimageV1,
                 digest: &OperationHandleDigest,
                 actual: &CanonicalSignatureV1| {
                    assert_eq!(
                        digest,
                        operation.handle().1.canonical_operation_handle_sha256()
                    );
                    assert_eq!(actual, &signature());
                    Ok(())
                };
            let mut admit_checkpoint =
                |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                    ControlStateCheckpointV1::admit_signed_wire(
                        wire,
                        checkpoint.schema_sha256(),
                        &checkpoint_authority,
                        |_, _, _, actual| {
                            assert_eq!(actual, &signature());
                            Ok(())
                        },
                    )
                };
            let mut resolve_observation_signer =
                |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| Ok(signing.clone());
            let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
            let mut admit_rotation =
                |_: &ResidentRecordRefV1,
                 _: &ArtifactDigest,
                 _: &ResidentRecordRefV1,
                 _: &ArtifactDigest| { Ok(()) };
            let mut verify_observation_signature =
                |_: &[u8],
                 _: &OperationObservationPreimageV1,
                 _: &OperationObservationDigest,
                 actual: &CanonicalSignatureV1| {
                    assert_eq!(actual, &signature());
                    Ok(())
                };
            let mut verify_checkpoint_signature =
                |_: &[u8],
                 _: &ControlStateCheckpointPreimageV1,
                 _: &ControlStateCheckpointDigest,
                 actual: &CanonicalSignatureV1| {
                    assert_eq!(actual, &signature());
                    Ok(())
                };
            let input = restart_input(recovery, 1 << 20);
            let provider_routes = (
                host.provider_routes.reserved,
                host.provider_routes.checked_out,
                host.provider_routes.active.len(),
            );
            let task_reservations = host.operations.outstanding_task_reservations();
            let result = host
                .classify_existing_operation(
                    ResidentProcessExistingOperationStart {
                        selector: selector.clone(),
                        evidence: Box::new(evidence),
                    },
                    association(signing, &input),
                    ResidentDeploymentOperationAccess {
                        store: &store,
                        custody,
                        sessions,
                        anchor,
                    },
                    input,
                    OperationControlRecoveryAdmission {
                        admit_request: &mut admit_request,
                        admit_handle_authority: &mut admit_handle_authority,
                        verify_handle_signature: &mut verify_handle_signature,
                        admit_checkpoint: &mut admit_checkpoint,
                        resolve_observation_signer: &mut resolve_observation_signer,
                        admit_observation_state: &mut admit_observation_state,
                        admit_rotation: &mut admit_rotation,
                        verify_observation_signature: &mut verify_observation_signature,
                        verify_checkpoint_signature: &mut verify_checkpoint_signature,
                    },
                )
                .expect("retained provider cancellation restart classification");
            let attempt = match result {
                ResidentProcessOperationRestartDisposition::NativeHookResponse(_) => {
                    panic!("action/provider fixture cannot become native hook response recovery")
                }
                ResidentProcessOperationRestartDisposition::Provider(attempt) => attempt,
                ResidentProcessOperationRestartDisposition::Terminal(_) => {
                    panic!("retained local provider cancellation became terminal")
                }
                ResidentProcessOperationRestartDisposition::RecoveryOnly(_) => {
                    panic!("retained local provider cancellation became recovery-only")
                }
                ResidentProcessOperationRestartDisposition::LocalCancellationRequested(_) => {
                    panic!("post-dispatch cancellation became local cancellation work")
                }
                ResidentProcessOperationRestartDisposition::OperationResolution(_) => {
                    panic!("post-dispatch cancellation became pre-attempt resolution")
                }
                ResidentProcessOperationRestartDisposition::Consult(_) => {
                    panic!("retained local provider cancellation became consultation work")
                }
                ResidentProcessOperationRestartDisposition::Native(_) => {
                    panic!("retained local provider cancellation became native work")
                }
                ResidentProcessOperationRestartDisposition::NativePrepared(_)
                | ResidentProcessOperationRestartDisposition::ProviderPrepared(_)
                | ResidentProcessOperationRestartDisposition::ProviderResolved(_) => {
                    panic!("post-dispatch cancellation became preparation recovery")
                }
                ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(_) => {
                    panic!("retained local provider cancellation became A2A recovery work")
                }
                ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(_) => {
                    panic!("retained root Invoke became A2A Delegate recovery work")
                }
                ResidentProcessOperationRestartDisposition::A2aTask(_) => {
                    panic!("retained root Invoke became known A2A Task work")
                }
            };
            assert_eq!(
                attempt.dispatch,
                ResidentProcessAttemptObservation::CancellationPredecessor
            );
            assert_eq!(&attempt.selection, selection);
            assert_eq!(
                (
                    host.provider_routes.reserved,
                    host.provider_routes.checked_out,
                    host.provider_routes.active.len(),
                ),
                provider_routes
            );
            assert_eq!(
                host.operations.outstanding_task_reservations(),
                task_reservations
            );
        },
    );
}
