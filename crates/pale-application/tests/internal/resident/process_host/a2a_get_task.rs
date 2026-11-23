//! Behavioral laws for binding-qualified A2A `Task` routes.

use super::*;
use crate::local_operation_start_fixture::{
    record_ref, schema_digest, signature, store_coordinate,
};
use crate::resident::a2a_get_task_owner::{a2a_get_task_key, ResidentA2aGetTaskStartFailure};
use crate::resident::operation_dispatch::selection::tests::a2a_readback_service::{
    exchange_limits, poll_context, with_recovered_a2a_task_history, A2aTaskHistoryFixture, Clock,
    GetTaskPort,
};
use crate::resident::process_host::a2a_get_task::*;
use crate::resident::process_host::a2a_task_routes::{
    ResidentProcessA2aTaskRouteInsertError, ResidentProcessA2aTaskRoutes,
};
use crate::resident::task_reservation::{ResidentA2aTaskWorkKey, ResidentTaskReservationPhase};
use crate::resident::{
    start_a2a_get_task_readback, A2aGetTaskCommitInput, A2aGetTaskObservationInput,
    A2aGetTaskReadbackInput, RecoveredA2aTaskHistory,
};
use std::{
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
};

use pale_core::records::{
    A2aJsonRpcRequestIdV1, A2aTaskBindingDigest, A2aTaskBindingIdV1, ArtifactDigest,
    ControlCheckpointIdV1, ControlCheckpointSigningAuthorityAdmissionContextV1,
    ControlIncarnationIdV1, ControlStateCheckpointV1, ControlStoreIdV1, DeploymentBindingDigest,
    DeploymentRuntimeAuthorityDigest, OperationHandleDigest, ResidentPrincipalIdV1,
    ResidentRecordRefV1, SignedResidentAdmissionContextV1, UnixTimeV1,
    A2A_TASK_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    AccumulatorMembershipLimits, AccumulatorPreparationLimits, AuthorityStoreCoordinate,
    ControlStoreError, OperationCustodyIndexPredecessor, SessionLeaseBody, SessionLeaseBodyRead,
    SessionLeaseBodyRetention, SessionLeaseCustodyPort, WallClockSourceError, WallClockSourcePort,
};

struct UnusedSessions;

impl SessionLeaseCustodyPort for UnusedSessions {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("GetTask does not renew a historical session")
    }

    fn read(
        &self,
        _: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        panic!("GetTask uses the already admitted historical session")
    }
}

struct CountingClock {
    observations: AtomicUsize,
}

impl CountingClock {
    fn new() -> Self {
        Self {
            observations: AtomicUsize::new(0),
        }
    }
}

impl WallClockSourcePort for CountingClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.observations.fetch_add(1, Ordering::SeqCst);
        Ok(UnixTimeV1::new(65).expect("fixture time"))
    }
}

fn store(name: &str) -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse(format!("store-{name}")).expect("store"),
        ControlIncarnationIdV1::parse("incarnation-1").expect("incarnation"),
    )
}

fn reference(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("record reference")
}

fn checkpoint_signer(
    checkpoint: &ControlStateCheckpointV1,
) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
    ControlCheckpointSigningAuthorityAdmissionContextV1 {
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
    }
}

fn selector(name: &str) -> ResidentDeploymentAuthoritySelector {
    let digest = ArtifactDigest::hash_bytes(name.as_bytes()).to_string();
    ResidentDeploymentAuthoritySelector {
        principal_id: ResidentPrincipalIdV1::parse(format!("principal-{name}")).expect("principal"),
        deployment_binding_ref: reference(&format!("binding/{name}")),
        deployment_binding_sha256: DeploymentBindingDigest::parse(digest.clone())
            .expect("deployment digest"),
        runtime_authority_ref: reference(&format!("runtime/{name}")),
        runtime_authority_sha256: DeploymentRuntimeAuthorityDigest::parse(digest)
            .expect("runtime digest"),
    }
}

fn key(name: &str) -> ResidentA2aTaskWorkKey {
    let digest = ArtifactDigest::hash_bytes(name.as_bytes()).to_string();
    ResidentA2aTaskWorkKey {
        store: store(name),
        operation_handle_ref: reference(&format!("operation/{name}")),
        operation_handle_sha256: OperationHandleDigest::parse(digest.clone())
            .expect("operation digest"),
        task_binding_id: A2aTaskBindingIdV1::parse(format!("task-binding-{name}"))
            .expect("task binding"),
        task_binding_ref: reference(&format!("task-binding/{name}")),
        task_binding_sha256: A2aTaskBindingDigest::parse(digest).expect("task binding digest"),
    }
}

fn routes(
    owner: ResidentOperationTaskOwnerInstance,
    maximum: usize,
) -> ResidentProcessA2aTaskRoutes {
    ResidentProcessA2aTaskRoutes::new(
        owner,
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(maximum).expect("positive limit")),
    )
    .expect("route ledger")
}

fn with_get_task_process(
    fixture: A2aTaskHistoryFixture,
    run: impl FnOnce(
        &RecoveredA2aTaskHistory,
        crate::resident::OperationAttemptHistoryInput<'_>,
        &SigningAuthorityBindingV1,
        &ControlStateCheckpointV1,
        &mut ProvingAuthority,
        &mut dyn pale_ports::OperationCustodyPort,
        &mut dyn pale_ports::AntiRollbackAnchorPort,
        &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
        &AuthorityStoreCoordinate,
        &ResidentDeploymentAuthoritySelector,
        &UnusedSessions,
    ) -> bool,
) {
    with_recovered_a2a_task_history(
        fixture,
        |history, attempt, signing, checkpoint, authority, custody, anchor, _parents, _| {
            let selected_store = history.local_observation().store().clone();
            let selector = selector_from_signing_authority(signing);
            let application_store = MemoryAuthorityStore::default();
            let deployment = unproved_deployment_for_selector(
                &selector,
                selected_store.clone(),
                ready_application(&application_store, selected_store.clone(), 1),
            );
            let mut host = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                vec![deployment],
                NonZeroUsize::new(1).expect("one deployment"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one ready GetTask deployment");
            let sessions = UnusedSessions;
            run(
                history,
                attempt,
                signing,
                checkpoint,
                authority,
                custody,
                anchor,
                &mut host,
                &selected_store,
                &selector,
                &sessions,
            )
        },
    );
}

fn copied_attempt<'a>(
    input: &crate::resident::OperationAttemptHistoryInput<'a>,
) -> crate::resident::OperationAttemptHistoryInput<'a> {
    crate::resident::OperationAttemptHistoryInput {
        operation: input.operation,
        request: input.request,
        session: input.session,
        lease: input.lease,
        generation: input.generation,
        dispatched: input.dispatched,
        selection: input.selection,
        maximum_selection_bytes: input.maximum_selection_bytes,
    }
}

fn read_input<'a>(
    history: &'a RecoveredA2aTaskHistory,
    attempt: &crate::resident::OperationAttemptHistoryInput<'a>,
    request_id: &'a A2aJsonRpcRequestIdV1,
    name: &str,
) -> A2aGetTaskReadbackInput<'a> {
    A2aGetTaskReadbackInput {
        history,
        attempt: copied_attempt(attempt),
        request_id,
        request_record_ref: reference(&format!("process/{name}-request")),
        response_record_ref: reference(&format!("process/{name}-response")),
        limits: exchange_limits(),
        maximum_evidence_bytes: 8_192,
    }
}

fn process_access<'a>(
    store: &'a AuthorityStoreCoordinate,
    custody: &'a mut dyn pale_ports::OperationCustodyPort,
    sessions: &'a UnusedSessions,
    anchor: &'a mut dyn pale_ports::AntiRollbackAnchorPort,
) -> ResidentDeploymentOperationAccess<'a> {
    ResidentDeploymentOperationAccess {
        store,
        custody,
        sessions,
        anchor,
    }
}

fn preparation_input<'a>(
    history: &'a RecoveredA2aTaskHistory,
    attempt: &crate::resident::OperationAttemptHistoryInput<'a>,
    request_id: &'a A2aJsonRpcRequestIdV1,
    name: &str,
    maximum_record_bytes: usize,
) -> crate::resident::a2a_task_custody::A2aGetTaskPreparationInput<'a> {
    crate::resident::a2a_task_custody::A2aGetTaskPreparationInput {
        readback: read_input(history, attempt, request_id, name),
        preparation_ref: record_ref(&format!("process/{name}-preparation")),
        current_predecessor: OperationCustodyIndexPredecessor::Absent,
        maximum_record_bytes,
    }
}

fn request_checkpoint(name: &str) -> OperationCheckpointInput {
    OperationCheckpointInput {
        reference: record_ref(&format!("process/{name}-checkpoint")),
        id: ControlCheckpointIdV1::parse(format!("process-{name}-checkpoint"))
            .expect("checkpoint identity"),
    }
}

fn accumulator_limits() -> AccumulatorPreparationLimits {
    AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000)
        .expect("accumulator limits")
}

fn membership_limits() -> AccumulatorMembershipLimits {
    AccumulatorMembershipLimits::new(8, 20_000, 200_000).expect("membership limits")
}

fn local_disposition_input<'a>(
    checkpoint: &'a ControlStateCheckpointV1,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    name: &str,
) -> crate::resident::operation_service::ResidentA2aGetTaskLocalDispositionInput<'a> {
    crate::resident::operation_service::ResidentA2aGetTaskLocalDispositionInput {
        disposition_ref: record_ref(&format!("process/{name}-disposition")),
        recorded_at: UnixTimeV1::new(67).expect("disposition time"),
        checkpoint: OperationCheckpointInput {
            reference: record_ref(&format!("process/{name}-disposition-checkpoint")),
            id: ControlCheckpointIdV1::parse(format!("process-{name}-disposition-checkpoint"))
                .expect("checkpoint identity"),
        },
        checkpoint_issued_at: UnixTimeV1::new(67).expect("checkpoint time"),
        checkpoint_schema: checkpoint.schema_sha256(),
        checkpoint_authority,
        accumulator_limits: accumulator_limits(),
    }
}

fn with_retained_get_task(
    run: impl FnOnce(
        &RecoveredA2aTaskHistory,
        &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
        &AuthorityStoreCoordinate,
        &ResidentDeploymentAuthoritySelector,
        &mut ProvingAuthority,
        &mut dyn pale_ports::OperationCustodyPort,
        &mut dyn pale_ports::AntiRollbackAnchorPort,
        &UnusedSessions,
        ResidentProcessRetainedA2aGetTask,
        ResidentA2aTaskWorkKey,
        &GetTaskPort,
    ) -> bool,
) {
    with_get_task_process(
        A2aTaskHistoryFixture {
            expected_callback_appends: 1,
            ..A2aTaskHistoryFixture::default()
        },
        |history,
         attempt,
         signing,
         checkpoint,
         authority,
         custody,
         anchor,
         host,
         store,
         selector,
         sessions| {
            let port = GetTaskPort::default();
            let request =
                A2aJsonRpcRequestIdV1::parse("process-get-task-retained").expect("request");
            let input = read_input(history, &attempt, &request, "get-task-retained");
            let key = a2a_get_task_key(&input);
            assert!(host
                .start_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    input,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                )
                .is_ok());
            let mut cx = poll_context();
            for expected in [
                ResidentProcessA2aGetTaskPoll::PendingResponse,
                ResidentProcessA2aGetTaskPoll::ResponseReceived,
                ResidentProcessA2aGetTaskPoll::PendingEof,
            ] {
                let actual = host.poll_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    &key,
                    &mut cx,
                    &Clock::at(66),
                );
                assert_eq!(
                    std::mem::discriminant(&actual),
                    std::mem::discriminant(&expected)
                );
            }
            let awaiting = match host.poll_a2a_get_task(
                selector,
                process_access(store, custody, sessions, anchor),
                authority,
                &key,
                &mut cx,
                &Clock::at(66),
            ) {
                ResidentProcessA2aGetTaskPoll::AwaitingCustody(awaiting) => awaiting,
                _ => panic!("completed response must await durable custody"),
            };
            let observation_schema = schema_digest(A2A_TASK_OBSERVATION_V1_SCHEMA_ID);
            let checkpoint_authority = checkpoint_signer(checkpoint);
            let retained = host
                .retain_a2a_get_task(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    awaiting,
                    A2aGetTaskCommitInput {
                        observation: A2aGetTaskObservationInput {
                            history,
                            operation: attempt.operation,
                            observation_ref: record_ref("process/get-task-retained-observation"),
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &observation_schema,
                                signing_authority: signing,
                            },
                            maximum_evidence_bytes: 8_192,
                            maximum_record_bytes: 65_536,
                        },
                        checkpoint: OperationCheckpointInput {
                            reference: record_ref("process/get-task-retained-checkpoint"),
                            id: ControlCheckpointIdV1::parse(
                                "process-get-task-retained-checkpoint",
                            )
                            .expect("checkpoint identity"),
                        },
                        issued_at: UnixTimeV1::new(67).expect("fixture time"),
                        checkpoint_schema: checkpoint.schema_sha256(),
                        checkpoint_authority: &checkpoint_authority,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            8, 20_000, 20_000, 8_192, 200_000,
                        )
                        .expect("accumulator limits"),
                        membership_limits: AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                            .expect("membership limits"),
                    },
                    &pale_adapter_a2a::A2aGetTaskDecoder,
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_| Ok(store_coordinate()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("valid completed response must reach durable custody"));
            run(
                history, host, store, selector, authority, custody, anchor, sessions, retained,
                key, &port,
            )
        },
    );
}

#[test]
fn binding_key_holds_capacity_through_checked_out_custody() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut routes = routes(owner, 1);
    let exact_key = key("one");
    let exact_selector = selector("one");

    assert!(routes
        .insert(exact_key.clone(), exact_selector.clone())
        .is_ok());
    assert!(matches!(
        routes.insert(exact_key.clone(), exact_selector.clone()),
        Err(ResidentProcessA2aTaskRouteInsertError::Conflict)
    ));
    let custody = routes.check_out(&exact_key).expect("checked-out route");
    assert!(matches!(
        routes.insert(key("two"), selector("two")),
        Err(ResidentProcessA2aTaskRouteInsertError::Saturated)
    ));
    assert!(routes.release(custody).is_ok());
    assert!(routes.insert(key("two"), selector("two")).is_ok());
}

#[test]
fn same_owner_replacement_ledger_rejects_crossed_route_custody() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut first = routes(owner.clone(), 1);
    let mut replacement = routes(owner, 1);
    let exact_key = key("one");
    let exact_selector = selector("one");
    assert!(first
        .insert(exact_key.clone(), exact_selector.clone())
        .is_ok());
    assert!(replacement
        .insert(exact_key.clone(), exact_selector)
        .is_ok());
    let custody = first.check_out(&exact_key).expect("checked-out route");
    let replacement_custody = replacement
        .check_out(&exact_key)
        .expect("identically populated replacement route");

    assert!(!replacement.owns_custody(&custody));
    let custody = match replacement.release(custody) {
        Ok(()) => panic!("replacement ledger must reject route custody"),
        Err(custody) => custody,
    };
    assert!(matches!(
        first.insert(
            key("first-capacity-probe"),
            selector("first-capacity-probe")
        ),
        Err(ResidentProcessA2aTaskRouteInsertError::Saturated)
    ));
    assert!(first.release(custody).is_ok());
    assert!(first
        .insert(
            key("first-capacity-probe"),
            selector("first-capacity-probe")
        )
        .is_ok());
    assert!(replacement.release(replacement_custody).is_ok());
}

#[test]
fn route_lookup_distinguishes_binding_and_selector_mismatch() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut routes = routes(owner, 2);
    let exact_key = key("one");
    let exact_selector = selector("one");
    assert!(routes
        .insert(exact_key.clone(), exact_selector.clone())
        .is_ok());

    assert!(routes.matches(&exact_key, &exact_selector));
    assert!(!routes.matches(&key("two"), &exact_selector));
    assert!(!routes.matches(&exact_key, &selector("two")));
}

#[test]
fn route_selection_reports_store_and_selector_failures_separately() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut routes = routes(owner, 1);
    let exact_key = key("one");
    let exact_selector = selector("one");
    assert!(routes
        .insert(exact_key.clone(), exact_selector.clone())
        .is_ok());

    assert!(matches!(
        routes.require_active(&exact_key, &exact_selector, &store("two")),
        Err(ResidentProcessHostSelectionError::AccessStoreAssociation)
    ));
    assert!(matches!(
        routes.require_active(&exact_key, &selector("two"), &exact_key.store),
        Err(ResidentProcessHostSelectionError::A2aTaskRouteAssociation)
    ));
    assert!(routes
        .require_active(&exact_key, &exact_selector, &exact_key.store)
        .is_ok());
}

#[test]
fn crossed_selector_cannot_remove_or_check_out_the_active_route() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut routes = routes(owner, 1);
    let exact_key = key("one");
    let exact_selector = selector("one");
    assert!(routes
        .insert(exact_key.clone(), exact_selector.clone())
        .is_ok());

    assert!(!routes.remove_active(&exact_key, &selector("two")));
    assert!(routes.matches(&exact_key, &exact_selector));
    let custody = routes
        .check_out(&exact_key)
        .expect("the exact active route remains available");
    assert!(routes.release(custody).is_ok());
}

#[test]
fn checked_out_route_is_not_an_active_poll_or_read_stop_selection() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut routes = routes(owner, 1);
    let exact_key = key("one");
    let exact_selector = selector("one");
    assert!(routes
        .insert(exact_key.clone(), exact_selector.clone())
        .is_ok());
    let custody = routes.check_out(&exact_key).expect("checked-out route");

    assert!(matches!(
        routes.require_active(&exact_key, &exact_selector, &exact_key.store),
        Err(ResidentProcessHostSelectionError::A2aTaskRouteAssociation)
    ));
    assert!(routes.release(custody).is_ok());
}

#[test]
fn checked_out_binding_rejects_another_route_even_when_capacity_remains() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut routes = routes(owner, 2);
    let exact_key = key("one");
    let exact_selector = selector("one");
    assert!(routes
        .insert(exact_key.clone(), exact_selector.clone())
        .is_ok());
    let custody = routes.check_out(&exact_key).expect("checked-out route");

    assert!(matches!(
        routes.insert(exact_key, exact_selector),
        Err(ResidentProcessA2aTaskRouteInsertError::Conflict)
    ));
    assert!(routes.insert(key("two"), selector("two")).is_ok());
    assert!(routes.release(custody).is_ok());
}

#[test]
fn active_route_is_released_only_by_its_exact_checked_out_custody() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut routes = routes(owner, 2);
    let first_key = key("one");
    let second_key = key("two");
    assert!(routes.insert(first_key.clone(), selector("one")).is_ok());
    assert!(routes.insert(second_key.clone(), selector("two")).is_ok());
    let first = routes.check_out(&first_key).expect("first route custody");
    let second = routes.check_out(&second_key).expect("second route custody");

    assert!(routes.owns_custody(&first));
    assert!(routes.owns_custody(&second));
    assert!(routes.release(first).is_ok());
    assert!(routes.owns_custody(&second));
    assert!(routes.insert(key("three"), selector("three")).is_ok());
    assert!(matches!(
        routes.insert(key("four"), selector("four")),
        Err(ResidentProcessA2aTaskRouteInsertError::Saturated)
    ));
    assert!(routes.release(second).is_ok());
    assert!(routes.insert(key("four"), selector("four")).is_ok());
}

#[test]
fn access_store_mismatch_fails_before_clock_authorization_or_carrier() {
    with_get_task_process(
        A2aTaskHistoryFixture::default(),
        |history,
         attempt,
         _,
         _,
         authority,
         custody,
         anchor,
         _,
         selected_store,
         exact_selector,
         sessions| {
            let unlike_store = store("unlike-deployment");
            let application_store = MemoryAuthorityStore::default();
            let deployment = unproved_deployment_for_selector(
                exact_selector,
                unlike_store.clone(),
                ready_application(&application_store, unlike_store.clone(), 1),
            );
            let mut host = ResidentProcessHost::new(
                unproved_writer(exact_selector.principal_id.clone(), vec![unlike_store]),
                vec![deployment],
                NonZeroUsize::new(1).expect("one deployment"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one unlike-store deployment");
            let request_id = A2aJsonRpcRequestIdV1::parse("get-task-store-fence").expect("request");
            let input = read_input(history, &attempt, &request_id, "get-task-store-fence");
            let key = a2a_get_task_key(&input);
            let clock = CountingClock::new();
            let port = GetTaskPort::default();
            let authorizations = AtomicUsize::new(0);

            assert!(matches!(
                host.start_a2a_get_task(
                    exact_selector,
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    input,
                    &port,
                    &clock,
                    |_| {
                        authorizations.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    },
                ),
                Err(ResidentProcessA2aGetTaskStartFailure::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation
                ))
            ));
            assert_eq!(clock.observations.load(Ordering::SeqCst), 0);
            assert_eq!(authorizations.load(Ordering::SeqCst), 0);
            assert_eq!(port.prepares(), 0);
            assert_eq!(port.starts(), 0);
            assert!(!host.a2a_task_routes.matches(&key, exact_selector));
            true
        },
    );
}

#[test]
fn route_saturation_fails_before_clock_authorization_or_carrier() {
    with_get_task_process(
        A2aTaskHistoryFixture::default(),
        |history,
         attempt,
         _,
         _,
         authority,
         custody,
         anchor,
         host,
         selected_store,
         exact_selector,
         sessions| {
            let mut occupied = key("occupied");
            occupied.store = selected_store.clone();
            assert!(host
                .a2a_task_routes
                .insert(occupied, exact_selector.clone())
                .is_ok());
            let request_id =
                A2aJsonRpcRequestIdV1::parse("get-task-route-saturated").expect("request");
            let input = read_input(history, &attempt, &request_id, "get-task-route-saturated");
            let clock = CountingClock::new();
            let port = GetTaskPort::default();
            let authorizations = AtomicUsize::new(0);

            assert!(matches!(
                host.start_a2a_get_task(
                    exact_selector,
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    input,
                    &port,
                    &clock,
                    |_| {
                        authorizations.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    },
                ),
                Err(ResidentProcessA2aGetTaskStartFailure::RouteSaturated)
            ));
            assert_eq!(clock.observations.load(Ordering::SeqCst), 0);
            assert_eq!(authorizations.load(Ordering::SeqCst), 0);
            assert_eq!(port.prepares(), 0);
            assert_eq!(port.starts(), 0);
            true
        },
    );
}

#[test]
fn duplicate_binding_start_does_not_start_a_second_carrier() {
    with_get_task_process(
        A2aTaskHistoryFixture::default(),
        |history,
         attempt,
         _,
         _,
         authority,
         custody,
         anchor,
         host,
         selected_store,
         selector,
         sessions| {
            let request_id =
                A2aJsonRpcRequestIdV1::parse("get-task-duplicate-binding").expect("request");
            let port = GetTaskPort::default();
            assert!(host
                .start_a2a_get_task(
                    selector,
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    read_input(history, &attempt, &request_id, "get-task-duplicate-binding"),
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                )
                .is_ok());
            assert!(matches!(
                host.start_a2a_get_task(
                    selector,
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    read_input(history, &attempt, &request_id, "get-task-duplicate-binding"),
                    &port,
                    &Clock::at(65),
                    |_| panic!("duplicate binding must fail before authorization"),
                ),
                Err(ResidentProcessA2aGetTaskStartFailure::RouteConflict)
            ));
            assert_eq!(port.starts(), 1);
            true
        },
    );
}

#[test]
fn local_response_read_stop_rejects_wrong_store_and_selector() {
    with_get_task_process(
        A2aTaskHistoryFixture::default(),
        |history,
         attempt,
         _,
         _,
         authority,
         custody,
         anchor,
         host,
         selected_store,
         exact_selector,
         sessions| {
            let request_id = A2aJsonRpcRequestIdV1::parse("get-task-stop-fences").expect("request");
            let input = read_input(history, &attempt, &request_id, "get-task-stop-fences");
            let key = a2a_get_task_key(&input);
            let port = GetTaskPort::default();
            assert!(host
                .start_a2a_get_task(
                    exact_selector,
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    input,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                )
                .is_ok());

            let unlike_store = store("unlike");
            assert!(matches!(
                host.stop_a2a_get_task_response_read(
                    exact_selector,
                    process_access(&unlike_store, custody, sessions, anchor),
                    authority,
                    &key,
                ),
                ResidentProcessA2aGetTaskPoll::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation
                )
            ));
            assert!(matches!(
                host.stop_a2a_get_task_response_read(
                    &selector("unlike"),
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    &key,
                ),
                ResidentProcessA2aGetTaskPoll::Selection(
                    ResidentProcessHostSelectionError::A2aTaskRouteAssociation
                )
            ));
            let stopped = match host.stop_a2a_get_task_response_read(
                exact_selector,
                process_access(selected_store, custody, sessions, anchor),
                authority,
                &key,
            ) {
                ResidentProcessA2aGetTaskPoll::ReadStopped(stopped) => stopped,
                _ => panic!("the exact route must enter local read-stop custody"),
            };
            assert!(host
                .finish_stopped_a2a_get_task_response_read(
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    stopped,
                )
                .is_ok());
            assert_eq!(port.starts(), 1);
            true
        },
    );
}

#[test]
fn only_originating_host_finishes_local_read_stop() {
    with_get_task_process(
        A2aTaskHistoryFixture::default(),
        |history,
         attempt,
         _,
         _,
         authority,
         custody,
         anchor,
         host,
         selected_store,
         selector,
         sessions| {
            let replacement_store = MemoryAuthorityStore::default();
            let replacement_deployment = unproved_deployment_for_selector(
                selector,
                selected_store.clone(),
                ready_application(&replacement_store, selected_store.clone(), 1),
            );
            let mut replacement = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                vec![replacement_deployment],
                NonZeroUsize::new(1).expect("one deployment"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one replacement deployment");
            let request_id = A2aJsonRpcRequestIdV1::parse("get-task-stop-owner").expect("request");
            let input = read_input(history, &attempt, &request_id, "get-task-stop-owner");
            let key = a2a_get_task_key(&input);
            let port = GetTaskPort::default();
            assert!(host
                .start_a2a_get_task(
                    selector,
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    input,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                )
                .is_ok());
            let stopped = match host.stop_a2a_get_task_response_read(
                selector,
                process_access(selected_store, custody, sessions, anchor),
                authority,
                &key,
            ) {
                ResidentProcessA2aGetTaskPoll::ReadStopped(stopped) => stopped,
                _ => panic!("the exact route must enter local read-stop custody"),
            };
            let stopped = match replacement.finish_stopped_a2a_get_task_response_read(
                process_access(selected_store, custody, sessions, anchor),
                authority,
                stopped,
            ) {
                Err(ResidentProcessA2aGetTaskTerminalReleaseFailure::SelectionReadStopped(
                    stopped,
                )) => stopped,
                _ => panic!("replacement host must preserve exact read-stop custody"),
            };
            assert!(host
                .finish_stopped_a2a_get_task_response_read(
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    stopped,
                )
                .is_ok());
            assert_eq!(port.starts(), 1);
            true
        },
    );
}

#[test]
fn only_originating_host_finishes_closed_response_failure() {
    with_get_task_process(
        A2aTaskHistoryFixture::default(),
        |history,
         attempt,
         _,
         _,
         authority,
         custody,
         anchor,
         host,
         selected_store,
         selector,
         sessions| {
            let replacement_store = MemoryAuthorityStore::default();
            let replacement_deployment = unproved_deployment_for_selector(
                selector,
                selected_store.clone(),
                ready_application(&replacement_store, selected_store.clone(), 1),
            );
            let mut replacement = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                vec![replacement_deployment],
                NonZeroUsize::new(1).expect("one deployment"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one replacement deployment");
            let request_id =
                A2aJsonRpcRequestIdV1::parse("get-task-closed-owner").expect("request");
            let input = read_input(history, &attempt, &request_id, "get-task-closed-owner");
            let key = a2a_get_task_key(&input);
            let port = GetTaskPort::response_failure();
            assert!(host
                .start_a2a_get_task(
                    selector,
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    input,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                )
                .is_ok());
            let mut cx = poll_context();
            assert!(matches!(
                host.poll_a2a_get_task(
                    selector,
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    &key,
                    &mut cx,
                    &Clock::at(66),
                ),
                ResidentProcessA2aGetTaskPoll::PendingResponse
            ));
            let closed = match host.poll_a2a_get_task(
                selector,
                process_access(selected_store, custody, sessions, anchor),
                authority,
                &key,
                &mut cx,
                &Clock::at(66),
            ) {
                ResidentProcessA2aGetTaskPoll::Closed(closed) => closed,
                _ => panic!("carrier response failure must enter closed custody"),
            };
            let closed = match replacement.finish_closed_a2a_get_task(
                process_access(selected_store, custody, sessions, anchor),
                authority,
                closed,
            ) {
                Err(ResidentProcessA2aGetTaskTerminalReleaseFailure::SelectionClosed(closed)) => {
                    closed
                }
                _ => panic!("replacement host must preserve exact closed custody"),
            };
            assert!(host
                .finish_closed_a2a_get_task(
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    closed,
                )
                .is_ok());
            assert_eq!(port.starts(), 1);
            true
        },
    );
}

#[test]
fn aggregate_acceptance_retry_does_not_replay_get_task() {
    with_get_task_process(
        A2aTaskHistoryFixture::default(),
        |history,
         attempt,
         _,
         _,
         authority,
         custody,
         anchor,
         host,
         selected_store,
         selector,
         sessions| {
            let request_id =
                A2aJsonRpcRequestIdV1::parse("get-task-accept-retry").expect("request");
            let input = read_input(history, &attempt, &request_id, "get-task-accept-retry");
            let key = a2a_get_task_key(&input);
            let port = GetTaskPort::default();
            assert!(host
                .a2a_task_routes
                .insert(key.clone(), selector.clone())
                .is_ok());
            let lower = host
                .with_selected_operation_service(
                    selector,
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    |_application, service, _sessions| {
                        let offer = service.begin_a2a_get_task_offer_for_test(key.clone());
                        let request_checkpoint = {
                            let (reference, checkpoint) =
                                input.history.local_observation().current_checkpoint();
                            Box::new((reference.clone(), checkpoint.clone()))
                        };
                        let in_flight =
                            start_a2a_get_task_readback(input, &port, &Clock::at(65), |_| Ok(()))
                                .expect("the fixture carrier starts once");
                        let reservation =
                            service.reject_a2a_get_task_acceptance_for_test(&key, offer);
                        ResidentA2aGetTaskStartFailure::Acceptance {
                            key: Box::new(key.clone()),
                            in_flight: Box::new(in_flight),
                            request_checkpoint,
                            reservation: Box::new(reservation),
                        }
                    },
                )
                .expect("the exact deployment remains selected");
            let failure = ResidentProcessA2aGetTaskStartFailure::Unresolved {
                key: Box::new(key.clone()),
                selector: Box::new(selector.clone()),
                failure: Box::new(lower),
            };
            assert!(matches!(
                host.retry_a2a_get_task_start(
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    failure,
                ),
                Ok(ResidentProcessA2aGetTaskStartRetry::Started)
            ));
            assert_eq!(port.starts(), 1);
            let stopped = match host.stop_a2a_get_task_response_read(
                selector,
                process_access(selected_store, custody, sessions, anchor),
                authority,
                &key,
            ) {
                ResidentProcessA2aGetTaskPoll::ReadStopped(stopped) => stopped,
                _ => panic!("retried start must remain locally stoppable"),
            };
            assert!(host
                .finish_stopped_a2a_get_task_response_read(
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    stopped,
                )
                .is_ok());
            assert_eq!(port.starts(), 1);
            true
        },
    );
}

#[test]
fn failed_aggregate_acceptance_can_stop_without_replay() {
    with_get_task_process(
        A2aTaskHistoryFixture::default(),
        |history,
         attempt,
         _,
         _,
         authority,
         custody,
         anchor,
         host,
         selected_store,
         selector,
         sessions| {
            let request_id = A2aJsonRpcRequestIdV1::parse("get-task-accept-stop").expect("request");
            let input = read_input(history, &attempt, &request_id, "get-task-accept-stop");
            let key = a2a_get_task_key(&input);
            let port = GetTaskPort::default();
            assert!(host
                .a2a_task_routes
                .insert(key.clone(), selector.clone())
                .is_ok());
            let lower = host
                .with_selected_operation_service(
                    selector,
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    |_application, service, _sessions| {
                        let offer = service.begin_a2a_get_task_offer_for_test(key.clone());
                        let request_checkpoint = {
                            let (reference, checkpoint) =
                                input.history.local_observation().current_checkpoint();
                            Box::new((reference.clone(), checkpoint.clone()))
                        };
                        let in_flight =
                            start_a2a_get_task_readback(input, &port, &Clock::at(65), |_| Ok(()))
                                .expect("the fixture carrier starts once");
                        let reservation =
                            service.reject_a2a_get_task_acceptance_for_test(&key, offer);
                        ResidentA2aGetTaskStartFailure::Acceptance {
                            key: Box::new(key.clone()),
                            in_flight: Box::new(in_flight),
                            request_checkpoint,
                            reservation: Box::new(reservation),
                        }
                    },
                )
                .expect("the exact deployment remains selected");
            let failure = ResidentProcessA2aGetTaskStartFailure::Unresolved {
                key: Box::new(key),
                selector: Box::new(selector.clone()),
                failure: Box::new(lower),
            };
            let stopped = match host.stop_a2a_get_task_after_acceptance_failure(
                process_access(selected_store, custody, sessions, anchor),
                authority,
                failure,
            ) {
                Ok(ResidentProcessA2aGetTaskPoll::ReadStopped(stopped)) => stopped,
                _ => panic!("local stop must retain its exact disposition"),
            };
            assert!(host
                .finish_stopped_a2a_get_task_response_read(
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    stopped,
                )
                .is_ok());
            assert_eq!(port.starts(), 1);
            assert_eq!(port.read_stops(), 1);
            true
        },
    );
}

#[test]
fn unopened_start_rollback_retry_never_starts_a_carrier() {
    with_get_task_process(
        A2aTaskHistoryFixture::default(),
        |history,
         attempt,
         _,
         _,
         authority,
         custody,
         anchor,
         host,
         selected_store,
         selector,
         sessions| {
            let request_id = A2aJsonRpcRequestIdV1::parse("get-task-rollback").expect("request");
            let input = read_input(history, &attempt, &request_id, "get-task-rollback");
            let key = a2a_get_task_key(&input);
            let port = GetTaskPort::default();
            assert!(host
                .a2a_task_routes
                .insert(key.clone(), selector.clone())
                .is_ok());
            let lower = host
                .with_selected_operation_service(
                    selector,
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    |_application, service, _sessions| {
                        let offer = service.begin_a2a_get_task_offer_for_test(key.clone());
                        let reason =
                            match start_a2a_get_task_readback(input, &port, &Clock::at(65), |_| {
                                Err(InvalidResidentRecordV1::ContextualAdmission {
                                    record: "GetTask authorization fixture",
                                    invariant: "refused",
                                })
                            }) {
                                Ok(_) => panic!("authorization must stop before carrier start"),
                                Err(reason) => reason,
                            };
                        let reservation =
                            service.reject_a2a_get_task_rollback_for_test(&key, offer);
                        ResidentA2aGetTaskStartFailure::Rollback {
                            reason: Box::new(reason),
                            reservation: Box::new(reservation),
                        }
                    },
                )
                .expect("the exact deployment remains selected");
            let failure = ResidentProcessA2aGetTaskStartFailure::Unresolved {
                key: Box::new(key),
                selector: Box::new(selector.clone()),
                failure: Box::new(lower),
            };
            assert!(matches!(
                host.retry_a2a_get_task_start(
                    process_access(selected_store, custody, sessions, anchor),
                    authority,
                    failure,
                ),
                Ok(ResidentProcessA2aGetTaskStartRetry::Closed(_))
            ));
            assert_eq!(port.starts(), 0);
            true
        },
    );
}

#[test]
fn read_stop_aggregate_release_retries_without_replay() {
    with_get_task_process(
        A2aTaskHistoryFixture::default(),
        |history, attempt, _, _, authority, custody, anchor, host, store, selector, sessions| {
            let port = GetTaskPort::default();
            let request =
                A2aJsonRpcRequestIdV1::parse("process-get-task-stop-release").expect("request");
            let input = read_input(history, &attempt, &request, "get-task-stop-release");
            let key = a2a_get_task_key(&input);
            assert!(host
                .start_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    input,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                )
                .is_ok());
            let stopped = match host.stop_a2a_get_task_response_read(
                selector,
                process_access(store, custody, sessions, anchor),
                authority,
                &key,
            ) {
                ResidentProcessA2aGetTaskPoll::ReadStopped(stopped) => stopped,
                _ => panic!("local read stop must retain its terminal disposition"),
            };
            host.with_selected_operation_service(
                selector,
                process_access(store, custody, sessions, anchor),
                authority,
                |_application, service, _sessions| {
                    service.set_a2a_get_task_reservation_phase_for_test(
                        &key,
                        ResidentTaskReservationPhase::Accepted,
                    );
                },
            )
            .expect("the exact test deployment must remain selected");
            let failure = match host.finish_stopped_a2a_get_task_response_read(
                process_access(store, custody, sessions, anchor),
                authority,
                stopped,
            ) {
                Err(
                    failure @ ResidentProcessA2aGetTaskTerminalReleaseFailure::ReadStopped {
                        ..
                    },
                ) => failure,
                _ => panic!("aggregate release fault must preserve read-stop custody"),
            };
            host.with_selected_operation_service(
                selector,
                process_access(store, custody, sessions, anchor),
                authority,
                |_application, service, _sessions| {
                    service.set_a2a_get_task_reservation_phase_for_test(
                        &key,
                        ResidentTaskReservationPhase::CompletionPendingDurableHandling,
                    );
                },
            )
            .expect("the exact test deployment must remain selected");
            assert!(matches!(
                host.retry_a2a_get_task_terminal_release(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    failure,
                ),
                Ok(ResidentProcessA2aGetTaskTerminalRelease::ReadStopped)
            ));
            assert_eq!(port.starts(), 1);
            assert_eq!(port.read_stops(), 1);
            true
        },
    );
}

#[test]
fn closed_aggregate_release_retries_without_replay() {
    with_get_task_process(
        A2aTaskHistoryFixture::default(),
        |history, attempt, _, _, authority, custody, anchor, host, store, selector, sessions| {
            let port = GetTaskPort::response_failure();
            let request =
                A2aJsonRpcRequestIdV1::parse("process-get-task-closed-release").expect("request");
            let input = read_input(history, &attempt, &request, "get-task-closed-release");
            let key = a2a_get_task_key(&input);
            assert!(host
                .start_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    input,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                )
                .is_ok());
            let mut cx = poll_context();
            assert!(matches!(
                host.poll_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    &key,
                    &mut cx,
                    &Clock::at(66),
                ),
                ResidentProcessA2aGetTaskPoll::PendingResponse
            ));
            let closed = match host.poll_a2a_get_task(
                selector,
                process_access(store, custody, sessions, anchor),
                authority,
                &key,
                &mut cx,
                &Clock::at(66),
            ) {
                ResidentProcessA2aGetTaskPoll::Closed(closed) => closed,
                _ => panic!("carrier read failure must retain a closed disposition"),
            };
            host.with_selected_operation_service(
                selector,
                process_access(store, custody, sessions, anchor),
                authority,
                |_application, service, _sessions| {
                    service.set_a2a_get_task_reservation_phase_for_test(
                        &key,
                        ResidentTaskReservationPhase::Accepted,
                    );
                },
            )
            .expect("the exact test deployment must remain selected");
            let failure = match host.finish_closed_a2a_get_task(
                process_access(store, custody, sessions, anchor),
                authority,
                closed,
            ) {
                Err(failure @ ResidentProcessA2aGetTaskTerminalReleaseFailure::Closed { .. }) => {
                    failure
                }
                _ => panic!("aggregate release fault must preserve closed custody"),
            };
            host.with_selected_operation_service(
                selector,
                process_access(store, custody, sessions, anchor),
                authority,
                |_application, service, _sessions| {
                    service.set_a2a_get_task_reservation_phase_for_test(
                        &key,
                        ResidentTaskReservationPhase::CompletionPendingDurableHandling,
                    );
                },
            )
            .expect("the exact test deployment must remain selected");
            assert!(matches!(
                host.retry_a2a_get_task_terminal_release(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    failure,
                ),
                Ok(ResidentProcessA2aGetTaskTerminalRelease::Closed(_))
            ));
            assert_eq!(port.starts(), 1);
            true
        },
    );
}

#[test]
fn completed_read_aggregate_transition_retries_without_replay() {
    with_get_task_process(
        A2aTaskHistoryFixture::default(),
        |history, attempt, _, _, authority, custody, anchor, host, store, selector, sessions| {
            let port = GetTaskPort::default();
            let request =
                A2aJsonRpcRequestIdV1::parse("process-get-task-transition-retry").expect("request");
            let input = read_input(history, &attempt, &request, "get-task-transition-retry");
            let key = a2a_get_task_key(&input);
            assert!(host
                .start_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    input,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                )
                .is_ok());
            let mut cx = poll_context();
            for expected in [
                ResidentProcessA2aGetTaskPoll::PendingResponse,
                ResidentProcessA2aGetTaskPoll::ResponseReceived,
                ResidentProcessA2aGetTaskPoll::PendingEof,
            ] {
                let actual = host.poll_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    &key,
                    &mut cx,
                    &Clock::at(66),
                );
                assert_eq!(
                    std::mem::discriminant(&actual),
                    std::mem::discriminant(&expected)
                );
            }
            host.with_selected_operation_service(
                selector,
                process_access(store, custody, sessions, anchor),
                authority,
                |_application, service, _sessions| {
                    service.set_a2a_get_task_reservation_phase_for_test(
                        &key,
                        ResidentTaskReservationPhase::Offered,
                    );
                },
            )
            .expect("the exact test deployment must remain selected");
            let (retry_selector, rejection) = match host.poll_a2a_get_task(
                selector,
                process_access(store, custody, sessions, anchor),
                authority,
                &key,
                &mut cx,
                &Clock::at(66),
            ) {
                ResidentProcessA2aGetTaskPoll::AggregateTransitionRejected {
                    key: actual_key,
                    selector,
                    rejection,
                } => {
                    assert_eq!(*actual_key, key);
                    (*selector, *rejection)
                }
                _ => panic!("aggregate transition rejection must preserve the completed response"),
            };
            host.with_selected_operation_service(
                selector,
                process_access(store, custody, sessions, anchor),
                authority,
                |_application, service, _sessions| {
                    service.set_a2a_get_task_reservation_phase_for_test(
                        &key,
                        ResidentTaskReservationPhase::Accepted,
                    );
                },
            )
            .expect("the exact test deployment must remain selected");
            assert!(matches!(
                host.retry_a2a_get_task_aggregate_transition(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    key,
                    retry_selector,
                    rejection,
                ),
                ResidentProcessA2aGetTaskPoll::AwaitingCustody(_)
            ));
            assert_eq!(port.starts(), 1);
            true
        },
    );
}

#[test]
fn prephysical_signing_failure_retries_without_replay() {
    with_get_task_process(
        A2aTaskHistoryFixture {
            expected_callback_appends: 1,
            ..A2aTaskHistoryFixture::default()
        },
        |history,
         attempt,
         signing,
         checkpoint,
         authority,
         custody,
         anchor,
         host,
         store,
         selector,
         sessions| {
            let port = GetTaskPort::default();
            let request =
                A2aJsonRpcRequestIdV1::parse("process-get-task-signing-retry").expect("request");
            let input = read_input(history, &attempt, &request, "get-task-signing-retry");
            let key = a2a_get_task_key(&input);
            assert!(host
                .start_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    input,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                )
                .is_ok());
            let mut cx = poll_context();
            for expected in [
                ResidentProcessA2aGetTaskPoll::PendingResponse,
                ResidentProcessA2aGetTaskPoll::ResponseReceived,
                ResidentProcessA2aGetTaskPoll::PendingEof,
            ] {
                let actual = host.poll_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    &key,
                    &mut cx,
                    &Clock::at(66),
                );
                assert_eq!(
                    std::mem::discriminant(&actual),
                    std::mem::discriminant(&expected)
                );
            }
            let awaiting = match host.poll_a2a_get_task(
                selector,
                process_access(store, custody, sessions, anchor),
                authority,
                &key,
                &mut cx,
                &Clock::at(66),
            ) {
                ResidentProcessA2aGetTaskPoll::AwaitingCustody(awaiting) => awaiting,
                _ => panic!("completed response must await durable custody"),
            };
            let observation_schema = schema_digest(A2A_TASK_OBSERVATION_V1_SCHEMA_ID);
            let checkpoint_authority = checkpoint_signer(checkpoint);
            let commit_input = || A2aGetTaskCommitInput {
                observation: A2aGetTaskObservationInput {
                    history,
                    operation: attempt.operation,
                    observation_ref: record_ref("process/get-task-signing-retry-observation"),
                    signing: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: signing,
                    },
                    maximum_evidence_bytes: 8_192,
                    maximum_record_bytes: 65_536,
                },
                checkpoint: OperationCheckpointInput {
                    reference: record_ref("process/get-task-signing-retry-checkpoint"),
                    id: ControlCheckpointIdV1::parse("process-get-task-signing-retry-checkpoint")
                        .expect("checkpoint identity"),
                },
                issued_at: UnixTimeV1::new(67).expect("fixture time"),
                checkpoint_schema: checkpoint.schema_sha256(),
                checkpoint_authority: &checkpoint_authority,
                accumulator_limits: AccumulatorPreparationLimits::new(
                    8, 20_000, 20_000, 8_192, 200_000,
                )
                .expect("accumulator limits"),
                membership_limits: AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                    .expect("membership limits"),
            };
            let failure = match host.retain_a2a_get_task(
                process_access(store, custody, sessions, anchor),
                authority,
                awaiting,
                commit_input(),
                &pale_adapter_a2a::A2aGetTaskDecoder,
                |_, _, _, _| Ok(()),
                |_, _, _| {
                    Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "GetTask observation signer fixture",
                        invariant: "unavailable",
                    })
                },
                |_| Ok(store_coordinate()),
                |_, _, _| Ok(signature()),
            ) {
                Err(failure @ ResidentProcessA2aGetTaskRetentionFailure::Lower { .. }) => failure,
                _ => panic!("signer failure must preserve pre-physical retention"),
            };
            let retained = host
                .retry_a2a_get_task_retention(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    failure,
                    commit_input(),
                    &pale_adapter_a2a::A2aGetTaskDecoder,
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_| Ok(store_coordinate()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("corrected signing input must reach durable custody"));
            let completed = host
                .finish_a2a_get_task(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    retained,
                )
                .unwrap_or_else(|_| panic!("corrected retention must release exact capacity"));
            assert_eq!(completed.binding(), history.binding());
            assert_eq!(port.starts(), 1);
            true
        },
    );
}

#[test]
fn aggregate_release_retries_only_for_the_exact_owner() {
    with_retained_get_task(
        |history,
         host,
         store,
         _selector,
         authority,
         custody,
         anchor,
         sessions,
         retained,
         _key,
         port| {
            let failure = host.reject_a2a_get_task_release_with_replacement_for_test(
                process_access(store, custody, sessions, anchor),
                authority,
                retained,
            );
            let completed = host
                .retry_a2a_get_task_release(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    failure,
                )
                .unwrap_or_else(|_| panic!("exact aggregate release must return custody"));
            assert_eq!(completed.binding(), history.binding());
            assert_eq!(port.starts(), 1);
            true
        },
    );
}

#[test]
fn crossed_route_and_custody_release_neither_capability() {
    with_retained_get_task(
        |history,
         host,
         store,
         selector,
         authority,
         custody,
         anchor,
         sessions,
         retained,
         exact_key,
         port| {
            let completed = host
                .finish_a2a_get_task(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    retained,
                )
                .unwrap_or_else(|_| panic!("exact retained custody must release"));
            let mut unlike_key = exact_key.clone();
            unlike_key.task_binding_id = A2aTaskBindingIdV1::parse("task-binding-crossed-custody")
                .expect("unlike task-binding identity");
            unlike_key.task_binding_ref = record_ref("process/task-binding-crossed-custody");
            unlike_key.task_binding_sha256 = A2aTaskBindingDigest::parse(
                ArtifactDigest::hash_bytes(b"crossed task binding").to_string(),
            )
            .expect("unlike task-binding digest");
            assert!(host
                .a2a_task_routes
                .insert(unlike_key.clone(), selector.clone())
                .is_ok());
            let unlike_route = host
                .a2a_task_routes
                .check_out(&unlike_key)
                .expect("unlike checked-out route");
            let (unlike_route, completed) = match host.retry_a2a_get_task_release(
                process_access(store, custody, sessions, anchor),
                authority,
                ResidentProcessA2aGetTaskReleaseFailure::Route {
                    route: unlike_route,
                    custody: Box::new(completed),
                },
            ) {
                Err(ResidentProcessA2aGetTaskReleaseFailure::Route { route, custody }) => {
                    (route, *custody)
                }
                _ => panic!("crossed binding and custody must remain unreleased"),
            };
            assert!(host.a2a_task_routes.owns_custody(&unlike_route));
            assert!(host.a2a_task_routes.release(unlike_route).is_ok());
            assert!(host
                .a2a_task_routes
                .insert(exact_key.clone(), selector.clone())
                .is_ok());
            let exact_route = host
                .a2a_task_routes
                .check_out(&exact_key)
                .expect("exact checked-out route");
            let completed = host
                .retry_a2a_get_task_release(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    ResidentProcessA2aGetTaskReleaseFailure::Route {
                        route: exact_route,
                        custody: Box::new(completed),
                    },
                )
                .unwrap_or_else(|_| panic!("restored exact route and custody must release"));
            assert_eq!(completed.binding(), history.binding());
            assert_eq!(port.starts(), 1);
            true
        },
    );
}

#[derive(Clone, Copy)]
enum PhysicalCustodyFault {
    DatabaseOutcomeUnknown,
    AnchorUnavailable,
}

fn prove_process_host_physical_custody_continuation(fault: PhysicalCustodyFault) {
    let (
        additional_read_backs,
        additional_anchor_advances,
        unknown_commits,
        unavailable_read_backs,
        unavailable_advances,
    ) = match fault {
        PhysicalCustodyFault::DatabaseOutcomeUnknown => (2, 0, &[2][..], &[3][..], &[][..]),
        PhysicalCustodyFault::AnchorUnavailable => (0, 1, &[][..], &[][..], &[2][..]),
    };
    with_recovered_a2a_task_history(
        A2aTaskHistoryFixture {
            expected_callback_appends: 1,
            additional_read_backs,
            additional_anchor_advances,
            unknown_exact_commits: unknown_commits.to_vec(),
            unavailable_read_backs: unavailable_read_backs.to_vec(),
            unavailable_advances: unavailable_advances.to_vec(),
            ..A2aTaskHistoryFixture::default()
        },
        |history, attempt_input, signing, checkpoint, authority, custody, anchor, _parents, _| {
            let selected_store = history.local_observation().store().clone();
            let exact_selector = selector_from_signing_authority(signing);
            let application_store = MemoryAuthorityStore::default();
            let deployment = unproved_deployment_for_selector(
                &exact_selector,
                selected_store.clone(),
                ready_application(&application_store, selected_store.clone(), 1),
            );
            let mut host = ResidentProcessHost::new(
                unproved_writer(
                    exact_selector.principal_id.clone(),
                    vec![selected_store.clone()],
                ),
                vec![deployment],
                NonZeroUsize::new(1).expect("one deployment"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one ready GetTask deployment");
            let sessions = UnusedSessions;
            let port = GetTaskPort::default();
            let request_id = match fault {
                PhysicalCustodyFault::DatabaseOutcomeUnknown => {
                    A2aJsonRpcRequestIdV1::parse("process-get-task-database-continuation")
                        .expect("request")
                }
                PhysicalCustodyFault::AnchorUnavailable => {
                    A2aJsonRpcRequestIdV1::parse("process-get-task-anchor-continuation")
                        .expect("request")
                }
            };
            let input = A2aGetTaskReadbackInput {
                history,
                attempt: crate::resident::OperationAttemptHistoryInput {
                    operation: attempt_input.operation,
                    request: attempt_input.request,
                    session: attempt_input.session,
                    lease: attempt_input.lease,
                    generation: attempt_input.generation,
                    dispatched: attempt_input.dispatched,
                    selection: attempt_input.selection,
                    maximum_selection_bytes: attempt_input.maximum_selection_bytes,
                },
                request_id: &request_id,
                request_record_ref: record_ref("process/get-task-physical-request"),
                response_record_ref: record_ref("process/get-task-physical-response"),
                limits: exchange_limits(),
                maximum_evidence_bytes: 8_192,
            };
            let exact_key = a2a_get_task_key(&input);
            assert!(host
                .start_a2a_get_task(
                    &exact_selector,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut *custody,
                        sessions: &sessions,
                        anchor: &mut *anchor,
                    },
                    authority,
                    input,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                )
                .is_ok());
            let mut cx = poll_context();
            assert!(matches!(
                host.poll_a2a_get_task(
                    &exact_selector,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut *custody,
                        sessions: &sessions,
                        anchor: &mut *anchor,
                    },
                    authority,
                    &exact_key,
                    &mut cx,
                    &Clock::at(66),
                ),
                ResidentProcessA2aGetTaskPoll::PendingResponse
            ));
            assert!(matches!(
                host.poll_a2a_get_task(
                    &exact_selector,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut *custody,
                        sessions: &sessions,
                        anchor: &mut *anchor,
                    },
                    authority,
                    &exact_key,
                    &mut cx,
                    &Clock::at(66),
                ),
                ResidentProcessA2aGetTaskPoll::ResponseReceived
            ));
            assert!(matches!(
                host.poll_a2a_get_task(
                    &exact_selector,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut *custody,
                        sessions: &sessions,
                        anchor: &mut *anchor,
                    },
                    authority,
                    &exact_key,
                    &mut cx,
                    &Clock::at(66),
                ),
                ResidentProcessA2aGetTaskPoll::PendingEof
            ));
            let awaiting = match host.poll_a2a_get_task(
                &exact_selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions: &sessions,
                    anchor: &mut *anchor,
                },
                authority,
                &exact_key,
                &mut cx,
                &Clock::at(66),
            ) {
                ResidentProcessA2aGetTaskPoll::AwaitingCustody(awaiting) => awaiting,
                _ => panic!("completed response must await durable custody"),
            };
            let observation_schema = schema_digest(A2A_TASK_OBSERVATION_V1_SCHEMA_ID);
            let checkpoint_authority = checkpoint_signer(checkpoint);
            let retained = match host.retain_a2a_get_task(
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions: &sessions,
                    anchor: &mut *anchor,
                },
                authority,
                awaiting,
                A2aGetTaskCommitInput {
                    observation: A2aGetTaskObservationInput {
                        history,
                        operation: attempt_input.operation,
                        observation_ref: record_ref("process/get-task-physical-observation"),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: signing,
                        },
                        maximum_evidence_bytes: 8_192,
                        maximum_record_bytes: 65_536,
                    },
                    checkpoint: OperationCheckpointInput {
                        reference: record_ref("process/get-task-physical-checkpoint"),
                        id: ControlCheckpointIdV1::parse("process-get-task-physical-checkpoint")
                            .expect("checkpoint identity"),
                    },
                    issued_at: UnixTimeV1::new(67).expect("fixture time"),
                    checkpoint_schema: checkpoint.schema_sha256(),
                    checkpoint_authority: &checkpoint_authority,
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        8, 20_000, 20_000, 8_192, 200_000,
                    )
                    .expect("accumulator limits"),
                    membership_limits: AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                        .expect("membership limits"),
                },
                &pale_adapter_a2a::A2aGetTaskDecoder,
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
                |_| Ok(store_coordinate()),
                |_, _, _| Ok(signature()),
            ) {
                Err(failure @ ResidentProcessA2aGetTaskRetentionFailure::Lower { .. }) => failure,
                _ => panic!("physical fault must preserve continuable custody"),
            };
            let retained = host
                .continue_a2a_get_task_custody(
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut *custody,
                        sessions: &sessions,
                        anchor: &mut *anchor,
                    },
                    authority,
                    retained,
                )
                .unwrap_or_else(|_| panic!("exact physical continuation must complete custody"));
            let completed = host
                .finish_a2a_get_task(
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut *custody,
                        sessions: &sessions,
                        anchor: &mut *anchor,
                    },
                    authority,
                    retained,
                )
                .unwrap_or_else(|_| panic!("durable custody must release aggregate and route"));
            assert_eq!(completed.binding(), history.binding());
            assert_eq!(
                port.starts(),
                1,
                "physical continuation must not replay GetTask"
            );
            true
        },
    );
}

#[test]
fn process_host_continues_unknown_database_custody_without_replay() {
    prove_process_host_physical_custody_continuation(PhysicalCustodyFault::DatabaseOutcomeUnknown);
}

#[test]
fn process_host_continues_unavailable_anchor_custody_without_replay() {
    prove_process_host_physical_custody_continuation(PhysicalCustodyFault::AnchorUnavailable);
}

#[test]
fn durable_request_custody_precedes_one_carrier_release() {
    with_get_task_process(
        A2aTaskHistoryFixture {
            expected_callback_appends: 2,
            ..A2aTaskHistoryFixture::default()
        },
        |history,
         attempt,
         signing,
         checkpoint,
         authority,
         custody,
         anchor,
         host,
         store,
         selector,
         sessions| {
            let port = GetTaskPort::default();
            let request_id =
                A2aJsonRpcRequestIdV1::parse("process-durable-get-task").expect("request");
            let checkpoint_authority = checkpoint_signer(checkpoint);
            let preparation =
                preparation_input(history, &attempt, &request_id, "durable-get-task", 65_536);
            let key = a2a_get_task_key(&preparation.readback);
            let prepared = host
                .prepare_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    preparation,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                    request_checkpoint("durable-get-task"),
                    UnixTimeV1::new(66).expect("checkpoint time"),
                    checkpoint.schema_sha256(),
                    &checkpoint_authority,
                    accumulator_limits(),
                    membership_limits(),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("exact GetTask preparation"));
            assert_eq!(port.prepares(), 1);
            assert_eq!(port.starts(), 0);

            let retained = host
                .retain_prepared_a2a_get_task(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    prepared,
                )
                .unwrap_or_else(|_| panic!("database and anchor custody"));
            assert_eq!(port.starts(), 0);
            assert!(matches!(
                host.start_retained_a2a_get_task(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    retained,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                ),
                Ok(ResidentProcessA2aGetTaskDurableStart::Active)
            ));
            assert_eq!(port.starts(), 1);

            assert!(matches!(
                host.start_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    read_input(history, &attempt, &request_id, "durable-get-task-replay"),
                    &port,
                    &Clock::at(65),
                    |_| panic!("an active route must reject before authorization"),
                ),
                Err(ResidentProcessA2aGetTaskStartFailure::RouteConflict)
            ));
            assert_eq!(port.starts(), 1, "the retained carrier is released once");

            let mut cx = poll_context();
            for expected in [
                ResidentProcessA2aGetTaskPoll::PendingResponse,
                ResidentProcessA2aGetTaskPoll::ResponseReceived,
                ResidentProcessA2aGetTaskPoll::PendingEof,
            ] {
                let actual = host.poll_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    &key,
                    &mut cx,
                    &Clock::at(66),
                );
                assert_eq!(
                    std::mem::discriminant(&actual),
                    std::mem::discriminant(&expected)
                );
            }
            let awaiting = match host.poll_a2a_get_task(
                selector,
                process_access(store, custody, sessions, anchor),
                authority,
                &key,
                &mut cx,
                &Clock::at(66),
            ) {
                ResidentProcessA2aGetTaskPoll::AwaitingCustody(awaiting) => awaiting,
                _ => panic!("completed response must retain its request checkpoint"),
            };
            let observation_schema = schema_digest(A2A_TASK_OBSERVATION_V1_SCHEMA_ID);
            let retained = host
                .retain_a2a_get_task(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    awaiting,
                    A2aGetTaskCommitInput {
                        observation: A2aGetTaskObservationInput {
                            history,
                            operation: attempt.operation,
                            observation_ref: record_ref("process/durable-get-task-observation"),
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &observation_schema,
                                signing_authority: signing,
                            },
                            maximum_evidence_bytes: 8_192,
                            maximum_record_bytes: 65_536,
                        },
                        checkpoint: OperationCheckpointInput {
                            reference: record_ref("process/durable-get-task-response-checkpoint"),
                            id: ControlCheckpointIdV1::parse(
                                "durable-get-task-response-checkpoint",
                            )
                            .expect("checkpoint identity"),
                        },
                        issued_at: UnixTimeV1::new(67).expect("fixture time"),
                        checkpoint_schema: checkpoint.schema_sha256(),
                        checkpoint_authority: &checkpoint_authority,
                        accumulator_limits: accumulator_limits(),
                        membership_limits: membership_limits(),
                    },
                    &pale_adapter_a2a::A2aGetTaskDecoder,
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_| Ok(store_coordinate()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("response must follow retained request custody"));
            let completed = host
                .finish_a2a_get_task(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    retained,
                )
                .unwrap_or_else(|_| panic!("response custody must release exact capacity"));
            assert_eq!(
                completed
                    .checkpoint_preimage()
                    .previous_control_checkpoint_ref
                    .as_ref(),
                Some(&record_ref("process/durable-get-task-checkpoint")),
                "the response checkpoint must follow the durable request checkpoint"
            );
            assert_eq!(port.starts(), 1, "response custody must not replay GetTask");
            true
        },
    );
}

#[derive(Clone, Copy)]
enum DurableLocalDispositionCase {
    CarrierStartFailure,
    ResponseReadFailure,
    ExplicitReadStop,
}

fn prove_durable_local_disposition(
    case: DurableLocalDispositionCase,
    custody_fault: Option<PhysicalCustodyFault>,
) {
    let (
        additional_read_backs,
        additional_anchor_advances,
        unknown_commits,
        unavailable_reads,
        unavailable_advances,
    ) = match custody_fault {
        None => (0, 0, Vec::new(), Vec::new(), Vec::new()),
        Some(PhysicalCustodyFault::DatabaseOutcomeUnknown) => (2, 0, vec![3], vec![4], Vec::new()),
        Some(PhysicalCustodyFault::AnchorUnavailable) => (0, 1, Vec::new(), Vec::new(), vec![3]),
    };
    with_get_task_process(
        A2aTaskHistoryFixture {
            expected_callback_appends: 2,
            additional_read_backs,
            additional_anchor_advances,
            unknown_exact_commits: unknown_commits,
            unavailable_read_backs: unavailable_reads,
            unavailable_advances,
            ..A2aTaskHistoryFixture::default()
        },
        |history,
         attempt,
         _signing,
         checkpoint,
         authority,
         custody,
         anchor,
         host,
         store,
         selector,
         sessions| {
            let (name, port) = match case {
                DurableLocalDispositionCase::CarrierStartFailure => {
                    let name = match custody_fault {
                        None => "start-failure",
                        Some(PhysicalCustodyFault::DatabaseOutcomeUnknown) => {
                            "start-failure-database-continuation"
                        }
                        Some(PhysicalCustodyFault::AnchorUnavailable) => {
                            "start-failure-anchor-continuation"
                        }
                    };
                    (name, GetTaskPort::start_failure())
                }
                DurableLocalDispositionCase::ResponseReadFailure => {
                    assert!(custody_fault.is_none());
                    ("response-failure", GetTaskPort::response_failure())
                }
                DurableLocalDispositionCase::ExplicitReadStop => {
                    assert!(custody_fault.is_none());
                    ("read-stopped", GetTaskPort::default())
                }
            };
            let request_id =
                A2aJsonRpcRequestIdV1::parse(format!("process-{name}")).expect("request identity");
            let checkpoint_authority = checkpoint_signer(checkpoint);
            let preparation = preparation_input(history, &attempt, &request_id, name, 65_536);
            let key = a2a_get_task_key(&preparation.readback);
            let prepared = host
                .prepare_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    preparation,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                    request_checkpoint(name),
                    UnixTimeV1::new(66).expect("checkpoint time"),
                    checkpoint.schema_sha256(),
                    &checkpoint_authority,
                    accumulator_limits(),
                    membership_limits(),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("exact GetTask preparation"));
            let retained = host
                .retain_prepared_a2a_get_task(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    prepared,
                )
                .unwrap_or_else(|_| panic!("durable request custody"));
            let started = host.start_retained_a2a_get_task(
                process_access(store, custody, sessions, anchor),
                authority,
                retained,
                &port,
                &Clock::at(65),
                |_| Ok(()),
            );
            assert_eq!(port.starts(), 1);

            let released = match case {
                DurableLocalDispositionCase::CarrierStartFailure => {
                    let closed = match started {
                        Ok(ResidentProcessA2aGetTaskDurableStart::Terminal(
                            ResidentProcessA2aGetTaskPoll::Closed(closed),
                        )) => closed,
                        _ => panic!("carrier-start failure must retain a closed owner phase"),
                    };
                    let retained = match host.retain_closed_a2a_get_task_local_disposition(
                            process_access(store, custody, sessions, anchor),
                            authority,
                            closed,
                            local_disposition_input(checkpoint, &checkpoint_authority, name),
                            |_, _, _| Ok(signature()),
                        ) {
                            Ok(retained) if custody_fault.is_none() => retained,
                            Err(
                                failure
                                @ ResidentProcessA2aGetTaskLocalDispositionCustodyFailure::Lower {
                                    ..
                                },
                            ) if custody_fault.is_some() => host
                                .continue_a2a_get_task_local_disposition_custody(
                                    process_access(store, custody, sessions, anchor),
                                    authority,
                                    failure,
                                )
                                .unwrap_or_else(|_| {
                                    panic!("exact local-disposition custody continuation")
                                }),
                            _ => panic!("unexpected local-disposition custody phase"),
                        };
                    host.finish_a2a_get_task_local_disposition(
                        process_access(store, custody, sessions, anchor),
                        authority,
                        retained,
                    )
                }
                DurableLocalDispositionCase::ResponseReadFailure => {
                    assert!(matches!(
                        started,
                        Ok(ResidentProcessA2aGetTaskDurableStart::Active)
                    ));
                    let mut cx = poll_context();
                    assert!(matches!(
                        host.poll_a2a_get_task(
                            selector,
                            process_access(store, custody, sessions, anchor),
                            authority,
                            &key,
                            &mut cx,
                            &Clock::at(66),
                        ),
                        ResidentProcessA2aGetTaskPoll::PendingResponse
                    ));
                    let closed = match host.poll_a2a_get_task(
                        selector,
                        process_access(store, custody, sessions, anchor),
                        authority,
                        &key,
                        &mut cx,
                        &Clock::at(66),
                    ) {
                        ResidentProcessA2aGetTaskPoll::Closed(closed) => closed,
                        _ => panic!("response failure must retain a closed owner phase"),
                    };
                    let retained = host
                        .retain_closed_a2a_get_task_local_disposition(
                            process_access(store, custody, sessions, anchor),
                            authority,
                            closed,
                            local_disposition_input(checkpoint, &checkpoint_authority, name),
                            |_, _, _| Ok(signature()),
                        )
                        .unwrap_or_else(|_| panic!("response-failure disposition custody"));
                    host.finish_a2a_get_task_local_disposition(
                        process_access(store, custody, sessions, anchor),
                        authority,
                        retained,
                    )
                }
                DurableLocalDispositionCase::ExplicitReadStop => {
                    assert!(matches!(
                        started,
                        Ok(ResidentProcessA2aGetTaskDurableStart::Active)
                    ));
                    let stopped = match host.stop_a2a_get_task_response_read(
                        selector,
                        process_access(store, custody, sessions, anchor),
                        authority,
                        &key,
                    ) {
                        ResidentProcessA2aGetTaskPoll::ReadStopped(stopped) => stopped,
                        _ => panic!("local read stop must retain its owner phase"),
                    };
                    assert_eq!(port.read_stops(), 1);
                    let retained = host
                        .retain_stopped_a2a_get_task_local_disposition(
                            process_access(store, custody, sessions, anchor),
                            authority,
                            stopped,
                            local_disposition_input(checkpoint, &checkpoint_authority, name),
                            |_, _, _| Ok(signature()),
                        )
                        .unwrap_or_else(|_| panic!("read-stop disposition custody"));
                    host.finish_a2a_get_task_local_disposition(
                        process_access(store, custody, sessions, anchor),
                        authority,
                        retained,
                    )
                }
            };
            assert!(matches!(
                released,
                Ok(ResidentProcessReleasedA2aGetTaskLocalDisposition::Closed { .. })
                    | Ok(ResidentProcessReleasedA2aGetTaskLocalDisposition::ReadStopped { .. })
            ));
            assert_eq!(
                port.starts(),
                1,
                "durable local disposition must not restart the carrier"
            );
            assert!(matches!(
                crate::resident::a2a_task_custody::recover_a2a_get_task_custody(
                    history,
                    attempt.operation,
                    65_536,
                    custody,
                ),
                Ok(
                    crate::resident::a2a_task_custody::RecoveredA2aGetTaskCustody::LocalDisposition(
                        _,
                    )
                )
            ));
            true
        },
    );
}

#[test]
fn carrier_start_failure_is_durable_before_capacity_is_released() {
    prove_durable_local_disposition(DurableLocalDispositionCase::CarrierStartFailure, None);
}

#[test]
fn response_read_failure_is_durable_before_capacity_is_released() {
    prove_durable_local_disposition(DurableLocalDispositionCase::ResponseReadFailure, None);
}

#[test]
fn local_read_stop_is_durable_before_capacity_is_released() {
    prove_durable_local_disposition(DurableLocalDispositionCase::ExplicitReadStop, None);
}

#[test]
fn local_disposition_continues_unknown_database_custody_without_replay() {
    prove_durable_local_disposition(
        DurableLocalDispositionCase::CarrierStartFailure,
        Some(PhysicalCustodyFault::DatabaseOutcomeUnknown),
    );
}

#[test]
fn local_disposition_continues_unavailable_anchor_custody_without_replay() {
    prove_durable_local_disposition(
        DurableLocalDispositionCase::CarrierStartFailure,
        Some(PhysicalCustodyFault::AnchorUnavailable),
    );
}

#[test]
fn hostile_store_cannot_advance_prepared_request_custody() {
    with_get_task_process(
        A2aTaskHistoryFixture {
            expected_callback_appends: 1,
            ..A2aTaskHistoryFixture::default()
        },
        |history,
         attempt,
         _,
         checkpoint,
         authority,
         custody,
         anchor,
         host,
         store,
         selector,
         sessions| {
            let port = GetTaskPort::default();
            let request_id =
                A2aJsonRpcRequestIdV1::parse("process-hostile-get-task").expect("request");
            let checkpoint_authority = checkpoint_signer(checkpoint);
            let prepared = host
                .prepare_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    preparation_input(history, &attempt, &request_id, "hostile-get-task", 65_536),
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                    request_checkpoint("hostile-get-task"),
                    UnixTimeV1::new(66).expect("checkpoint time"),
                    checkpoint.schema_sha256(),
                    &checkpoint_authority,
                    accumulator_limits(),
                    membership_limits(),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("exact GetTask preparation"));
            let unlike_store = super::store("hostile-get-task", "incarnation-hostile");
            let pending = match host.retain_prepared_a2a_get_task(
                process_access(&unlike_store, custody, sessions, anchor),
                authority,
                prepared,
            ) {
                Err(pending) => pending,
                Ok(_) => panic!("crossed store cannot reach physical custody"),
            };
            assert_eq!(port.starts(), 0);
            let _retained = host
                .continue_a2a_get_task_request_custody(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    pending,
                )
                .unwrap_or_else(|_| panic!("exact store resumes the same prepared request"));
            assert_eq!(port.starts(), 0);
            true
        },
    );
}

#[test]
fn failed_preparation_releases_route_without_carrier_authority() {
    with_get_task_process(
        A2aTaskHistoryFixture::default(),
        |history,
         attempt,
         _,
         checkpoint,
         authority,
         custody,
         anchor,
         host,
         store,
         selector,
         sessions| {
            let port = GetTaskPort::default();
            let request_id =
                A2aJsonRpcRequestIdV1::parse("process-invalid-get-task").expect("request");
            let checkpoint_authority = checkpoint_signer(checkpoint);
            let input = preparation_input(history, &attempt, &request_id, "invalid-get-task", 0);
            let key = a2a_get_task_key(&input.readback);
            assert!(matches!(
                host.prepare_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    input,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                    request_checkpoint("invalid-get-task"),
                    UnixTimeV1::new(66).expect("checkpoint time"),
                    checkpoint.schema_sha256(),
                    &checkpoint_authority,
                    accumulator_limits(),
                    membership_limits(),
                    |_, _, _| Ok(signature()),
                ),
                Err(ResidentProcessA2aGetTaskPreparationFailure::Closed(_))
            ));
            assert_eq!(port.prepares(), 0);
            assert_eq!(port.starts(), 0);
            assert!(host
                .a2a_task_routes
                .insert(key.clone(), selector.clone())
                .is_ok());
            assert!(host.a2a_task_routes.remove_active(&key, selector));
            true
        },
    );
}

#[test]
fn anchor_failure_preserves_request_without_replaying_carrier() {
    with_get_task_process(
        A2aTaskHistoryFixture {
            expected_callback_appends: 1,
            additional_anchor_advances: 1,
            unavailable_advances: vec![2],
            ..A2aTaskHistoryFixture::default()
        },
        |history,
         attempt,
         _,
         checkpoint,
         authority,
         custody,
         anchor,
         host,
         store,
         selector,
         sessions| {
            let port = GetTaskPort::default();
            let request_id =
                A2aJsonRpcRequestIdV1::parse("process-anchor-get-task").expect("request");
            let checkpoint_authority = checkpoint_signer(checkpoint);
            let prepared = host
                .prepare_a2a_get_task(
                    selector,
                    process_access(store, custody, sessions, anchor),
                    authority,
                    preparation_input(history, &attempt, &request_id, "anchor-get-task", 65_536),
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                    request_checkpoint("anchor-get-task"),
                    UnixTimeV1::new(66).expect("checkpoint time"),
                    checkpoint.schema_sha256(),
                    &checkpoint_authority,
                    accumulator_limits(),
                    membership_limits(),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("exact GetTask preparation"));
            let pending = match host.retain_prepared_a2a_get_task(
                process_access(store, custody, sessions, anchor),
                authority,
                prepared,
            ) {
                Err(pending) => pending,
                Ok(_) => panic!("first independent-anchor advance is unavailable"),
            };
            assert_eq!(port.starts(), 0);
            let retained = host
                .continue_a2a_get_task_request_custody(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    pending,
                )
                .unwrap_or_else(|_| panic!("exact anchor continuation"));
            assert_eq!(port.starts(), 0);
            assert!(matches!(
                host.start_retained_a2a_get_task(
                    process_access(store, custody, sessions, anchor),
                    authority,
                    retained,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                ),
                Ok(ResidentProcessA2aGetTaskDurableStart::Active)
            ));
            assert_eq!(port.starts(), 1);
            true
        },
    );
}
