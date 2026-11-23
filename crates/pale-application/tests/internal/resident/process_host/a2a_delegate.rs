//! Behavioral checks for private A2A deployment routing.

use super::*;
use crate::local_operation_start_fixture::{
    record_ref, schema_digest, signature, store_coordinate,
};
use crate::resident::operation_dispatch::selection::tests::a2a_readback_service::recover_retained_a2a_task_history;
use crate::resident::operation_dispatch::selection::tests::provider_dispatch::{
    with_delegate_dispatch_input_named, with_retained_delegate_dispatch,
    with_retained_delegate_dispatch_named, A2aTaskOwnerCustodyModel,
};
use crate::resident::operation_observation_custody::tests::a2a_exchange;
use crate::resident::process_host::a2a_delegate::test_support::{
    cross_a2a_retained_outcomes_for_test, cross_a2a_retention_failures_for_test,
};
use crate::resident::process_host::a2a_delegate::ResidentProcessA2aDelegateRoutes;
use crate::resident::process_host::a2a_delegate::{
    ResidentProcessA2aDelegateCompletion, ResidentProcessA2aDelegateLaunch,
    ResidentProcessA2aDelegateLaunchFailure, ResidentProcessA2aDelegateLaunchPlan,
    ResidentProcessA2aDelegatePoll, ResidentProcessA2aDelegateRecoveryFailure,
    ResidentProcessA2aDelegateRestartFailure, ResidentProcessA2aDelegateRetention,
    ResidentProcessA2aDelegateRetentionFailure, ResidentProcessA2aDelegateRouteCommitFailure,
    ResidentProcessA2aDelegateRouteCustody, ResidentProcessPairwiseA2aDelegateLaunchFailure,
    ResidentProcessRecoveredA2aDelegate,
};
use crate::resident::process_host::operation_request_host::{
    ResidentOperationRequestHost, ResidentOperationRequestKind, ResidentOperationRequestOutcome,
    ResidentOperationRequestResolution,
};
use crate::resident::process_host::restart::{
    ResidentProcessAttemptObservation, ResidentProcessRecoveredOperationStart,
};
use crate::resident::task_owner::{ResidentOperationTaskKey, ResidentOperationTaskOwnerInstance};
use crate::resident::{
    A2aDelegateLaunchInput, A2aFailedSendObservationInput, A2aInitialTaskCommitInput,
    A2aInitialTaskCustodyInput, A2aNoTaskOutcomeRetention, A2aNotStartedOutcomeRetention,
    A2aOutcomeUnknownObservationInput, A2aRecoveredUnknownOutcomeRetention, A2aRouteAssociation,
    A2aSendDispatch, A2aSendInput, A2aSendObservationCommitInput, A2aSendRecoveryAdmission,
    A2aSendRecoveryInput, A2aSendRecoveryLimits, A2aTaskOutcomeRetention,
    A2aUnknownOutcomeRetention, AmbientSessionAdmissionContext, IssuedLocalOperation,
    OperationAttemptHistoryInput, OperationCheckpointAncestryLimits,
    OperationObservationAuthorityStoreResolver, RecoveredA2aTaskHistory,
    RecoveredOperationObservation,
};
use pale_core::records::{
    A2aJsonRpcRequestIdV1, A2aRemoteContextIdV1, A2aRemoteTaskIdV1, A2aRemoteTaskStateV1,
    A2aRouteAdmissionContextV1, A2aRouteAdmissionV1, A2aRouteAdmissionWireV1, A2aTaskBindingDigest,
    A2aTaskBindingIdV1, A2aTaskBindingPreimageV1, A2aTaskObservationPreimageV1,
    A2aTaskObservationRevisionV1, A2aTaskObservationSourceV1, ArtifactDigest,
    ControlCheckpointIdV1, ControlStateCheckpointV1, ControlStateCheckpointWireV1,
    ImmutableRecordReferenceV1, ReconciliationFrontierReferenceV1, SchemaId, UnixTimeV1,
    A2A_TASK_BINDING_V1_SCHEMA, A2A_TASK_BINDING_V1_SCHEMA_ID, A2A_TASK_OBSERVATION_V1_SCHEMA,
    A2A_TASK_OBSERVATION_V1_SCHEMA_ID, OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    A2aExchange, A2aExchangeCoordinate, A2aExchangeError, A2aExchangeEvidence, A2aExchangeFacts,
    A2aExchangePort, A2aExchangeRequest, A2aPreparedExchange, A2aSendMessageEvidencePort,
    AccumulatorPreparationLimits, AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition,
    AntiRollbackAnchorError, AntiRollbackAnchorErrorKind, AntiRollbackAnchorKey,
    AntiRollbackAnchorObservation, AntiRollbackAnchorPort, ControlStoreError, OperationA2aEvidence,
    OperationCustodyRecordKind, PreparedAntiRollbackAnchorTransition,
    ResidentOperationFrameResponse, SessionLeaseBody, SessionLeaseBodyRead,
    SessionLeaseBodyRetention, SessionLeaseCustodyPort, WallClockSourceError, WallClockSourcePort,
};
use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::task::{Context, Poll};

fn task_key(store: AuthorityStoreCoordinate, label: &str, byte: char) -> ResidentOperationTaskKey {
    ResidentOperationTaskKey::new(
        store,
        ImmutableRecordReferenceV1 {
            record_ref: ResidentRecordRefV1::parse(label).expect("A2A dispatch reference"),
            record_sha256: ArtifactDigest::parse(byte.to_string().repeat(64))
                .expect("A2A dispatch digest"),
        },
    )
}

fn context_only_custody_model() -> A2aTaskOwnerCustodyModel<'static> {
    A2aTaskOwnerCustodyModel {
        clock_times: Vec::new(),
        expected_preparations: 1,
        expected_commits: 1,
        expected_read_backs: 1,
        expected_anchor_advances: 1,
        expected_anchor_observations: 1,
        unknown_exact_commits: &[],
        unavailable_read_backs: &[],
        unavailable_anchor_advances: &[],
        unknown_exact_anchor_advances: &[],
        unknown_failed_observation_anchor_advances: &[],
        later_anchor_observations: Vec::new(),
        fail_a2a_preflight_once: false,
    }
}

/// Compile-time witness that Delegate launch failures borrow their operation
/// inputs, not the temporary recovery-resolution loan used to validate them.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn launch_pairwise_with_temporary_resolution_borrow<
    'resolution,
    'operation,
    'call,
    'store,
    'access,
>(
    host: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
    recovered: &'resolution crate::resident::process_host::restart::ResidentProcessRecoveredOperationResolution,
    access: ResidentDeploymentOperationAccess<'access>,
    resolution: crate::resident::process_host::operation_resolution::ResidentProcessOperationResolutionInput<'call>,
    completed_context: Option<
        &crate::resident::process_host::operation_resolution::CompletedResidentProcessOperationContext<'_>,
    >,
    authority: &mut ProvingAuthority,
    dispatch: crate::resident::ProviderOperationDispatchInput<'operation, 'call>,
    exchange: &dyn A2aExchangePort,
) -> Result<
    Result<
        ResidentProcessA2aDelegateLaunch,
        ResidentProcessPairwiseA2aDelegateLaunchFailure<'operation, 'call>,
    >,
    crate::resident::process_host::operation_resolution::ResidentProcessOperationResolutionFailure,
> {
    host.launch_recovered_pairwise_a2a_delegate(
        recovered,
        access,
        resolution,
        completed_context,
        authority,
        dispatch,
        || -> ResidentProcessA2aDelegateLaunchPlan<'call> {
            unreachable!("the selection refusal must retain the supplied dispatch")
        },
        exchange,
        |_, _, _, _| unreachable!("provider retention must not start"),
        |_, _, _| unreachable!("provider observation must not be signed"),
        |_, _, _| unreachable!("provider checkpoint must not be signed"),
        |_, _, _| unreachable!("A2A disclosure must not run"),
        |_, _, _| unreachable!("A2A transport admission must not run"),
        |_, _, _| unreachable!("A2A checkpoint must not be signed"),
    )
}

#[test]
fn recovered_pairwise_delegate_retains_the_exact_dispatch_before_a2a_start() {
    let mut model = a2a_process_custody_model(
        None,
        A2aProcessRetentionMode::Normal,
        RetainedTaskCallbackCustody::ReadOnly,
    );
    model.expected_preparations -= 1;
    model.expected_commits -= 1;
    model.expected_read_backs -= 1;
    model.expected_anchor_advances -= 1;
    model.expected_anchor_observations -= 1;
    with_delegate_dispatch_input_named(
        model,
        "pairwise-a2a-composition",
        |dispatch,
         recovered,
         completed_context,
         checkpoint_schema,
         checkpoint_authority,
         _provider_clock,
         resolve_store,
         authority,
         custody,
         anchor,
         parents,
         _parent_request,
         restart_generation,
         _initial_candidate| {
            let selected_store = recovered.start.control.observation().store().clone();
            let selector = recovered.start.selector.clone();
            let (sessions, _) = super::restart::cold_restart_sessions(parents, &selected_store);
            let application_store = MemoryAuthorityStore::default();
            let host = super::restart::host_with_generation(
                &application_store,
                &selector,
                &selected_store,
                restart_generation,
            );
            let provider = dispatch.operation.invocation.provider_binding().clone();
            let route = a2a_exchange::route_for_handle_at_with_authority(
                &provider,
                dispatch.operation.context.operation.handle().1,
                "https://agent.example.test/a2a",
                true,
                10,
                100,
                110,
                parents.signing,
            );
            let route_ref = record_ref("pairwise-a2a-composition/route");
            let request_id = A2aJsonRpcRequestIdV1::parse("pairwise-a2a-rpc").unwrap();
            let launch_current = parents.current.at_time(UnixTimeV1::new(65).unwrap());
            let response_clock = A2aProcessClock::new([Ok(66), Ok(67), Ok(68)]);
            let exchange = A2aProcessExchangePort::new(A2aProcessExchangeMode::Response(
                A2aExchangeFacts::DirectMessage,
            ));
            let selection_ref = dispatch.operation.context.selection_ref.clone();
            let observation_ref = dispatch.operation.context.observation_ref.clone();
            let maximum_selection_bytes = dispatch.operation.context.maximum_selection_bytes;
            let signing = dispatch.signing;
            let plan = ResidentProcessA2aDelegateLaunchPlan {
                current: &launch_current,
                route: (&route_ref, &route),
                route_predecessor: pale_ports::OperationCustodyIndexPredecessor::Absent,
                route_association: A2aRouteAssociation::Insert,
                request_id: &request_id,
                message_id: "pairwise-a2a-message",
                text: "Inspect the selected workspace.",
                request_evidence_ref: record_ref("pairwise-a2a-composition/request"),
                preparation_ref: record_ref("pairwise-a2a-composition/preparation"),
                exchange_limits: a2a_exchange::limits(),
                maximum_evidence_bytes: 8_192,
                maximum_record_bytes: 65_536,
                maximum_dispatch_record_bytes: 65_536,
                resolve_store,
                checkpoint: OperationCheckpointInput {
                    reference: record_ref("pairwise-a2a-composition/send-checkpoint"),
                    id: ControlCheckpointIdV1::parse("pairwise-a2a-composition-send-checkpoint")
                        .unwrap(),
                },
                checkpoint_schema,
                checkpoint_authority,
                response_ref: record_ref("pairwise-a2a-composition/response"),
                clock: &response_clock,
                accumulator_limits: AccumulatorPreparationLimits::new(
                    8, 20_000, 20_000, 8_192, 200_000,
                )
                .unwrap(),
                membership_limits: pale_ports::AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                    .unwrap(),
            };
            let child = recovered.start.control.operation().operation();
            let locator = operation_frame_locator(child);
            let request_frame = ResidentOperationFrameRequest::AdvanceDelegate {
                locator: locator.clone(),
                signed_common_request: pale_core::canon::to_canonical_string(
                    dispatch.operation.context.request,
                )
                .expect("signed Delegate request is canonical"),
            };
            let mut frame_port = OperationFramePort::new(&request_frame);
            let deadline = std::time::Instant::now()
                .checked_add(std::time::Duration::from_secs(2))
                .expect("test deadline");
            let mut request_host = ResidentOperationRequestHost::new(&mut frame_port, deadline);
            let received = request_host
                .receive()
                .expect("Delegate continuation frame is exact");
            let mut host = host;
            let response = operation_request_host::request_host_test_support::answer(
                    &mut request_host,
                    received,
                    &mut host,
                    ResidentOperationRequestResolution::retained(
                        ResidentOperationRequestKind::AdvanceDelegate,
                        dispatch.operation.context.request,
                        locator.clone(),
                    ),
                    &mut |_, actual| {
                        assert_eq!(actual, &request_frame);
                        Ok(())
                    },
                    deadline,
                    |host, admitted_request| {
                        assert!(std::ptr::eq(
                            admitted_request,
                            dispatch.operation.context.request
                        ));
                        let result = host
                            .launch_recovered_pairwise_a2a_delegate(
                                recovered,
                                ResidentDeploymentOperationAccess {
                                    store: &selected_store,
                                    custody,
                                    sessions: &sessions,
                                    anchor,
                                },
                                crate::resident::process_host::operation_resolution::ResidentProcessOperationResolutionInput {
                                    session: parents.session,
                                    current: parents.current,
                                    signing,
                                    selection_ref,
                                    observation_ref,
                                    maximum_selection_bytes,
                                    maximum_record_bytes: 65_536,
                                },
                                Some(completed_context),
                                authority,
                                dispatch,
                                || plan,
                                &exchange,
                                |_, _, _, _| Ok(()),
                                |_, _, _| Ok(signature()),
                                |_, _, _| Ok(signature()),
                                |_, _, _| Ok(()),
                                |_, _, _| Ok(()),
                                |_, _, _| Ok(signature()),
                            )
                            .map_err(|_| {
                                pale_ports::ResidentOperationFrameFailure::ResidentUnavailable
                            })?;
                        assert!(matches!(
                            result,
                            Ok(ResidentProcessA2aDelegateLaunch::Awaiting(_))
                        ));
                        Ok(ResidentOperationRequestOutcome::Pending(locator.clone()))
                    },
                )
                .expect("private request host advances the exact A2A route once");
            assert_eq!(
                response,
                ResidentOperationFrameResponse::Pending {
                    locator: locator.clone()
                }
            );
            assert_eq!(exchange.starts.get(), 1);
        },
    );
}

#[test]
fn recovered_pairwise_launch_failure_is_independent_of_temporary_resolution_borrow() {
    let model = context_only_custody_model();
    with_delegate_dispatch_input_named(
        model,
        "pairwise-a2a-substituted-selection",
        |mut dispatch,
         recovered,
         completed_context,
         _checkpoint_schema,
         _checkpoint_authority,
         _provider_clock,
         _resolve_store,
         authority,
         custody,
         anchor,
         parents,
         _parent_request,
         restart_generation,
         _initial_candidate| {
            let selected_store = recovered.start.control.observation().store().clone();
            let selector = recovered.start.selector.clone();
            let (sessions, _) = super::restart::cold_restart_sessions(parents, &selected_store);
            let application_store = MemoryAuthorityStore::default();
            let mut host = super::restart::host_with_generation(
                &application_store,
                &selector,
                &selected_store,
                restart_generation,
            );
            let exchange = A2aProcessExchangePort::new(A2aProcessExchangeMode::Response(
                A2aExchangeFacts::DirectMessage,
            ));
            let selection_ref = dispatch.operation.context.selection_ref.clone();
            let observation_ref = dispatch.operation.context.observation_ref.clone();
            let maximum_selection_bytes = dispatch.operation.context.maximum_selection_bytes;
            let signing = dispatch.signing;
            dispatch.operation.context.selection_ref =
                record_ref("pairwise-a2a-substituted-selection/foreign-selection");
            let result = launch_pairwise_with_temporary_resolution_borrow(
                    &mut host,
                    recovered,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody,
                        sessions: &sessions,
                        anchor,
                    },
                    crate::resident::process_host::operation_resolution::ResidentProcessOperationResolutionInput {
                        session: parents.session,
                        current: parents.current,
                        signing,
                        selection_ref,
                        observation_ref,
                        maximum_selection_bytes,
                        maximum_record_bytes: 65_536,
                    },
                    Some(completed_context),
                    authority,
                    dispatch,
                    &exchange,
                )
                .expect("the recovered operation itself remains valid");
            assert!(matches!(
                result,
                Err(ResidentProcessPairwiseA2aDelegateLaunchFailure::Selection {
                    reason: ResidentProcessHostSelectionError::A2aOperationAssociation,
                    ..
                })
            ));
            assert_eq!(exchange.preparations.get(), 0);
            assert_eq!(exchange.starts.get(), 0);
            assert_eq!(host.a2a_delegate_routes.reserved_count(), 0);
            assert_eq!(host.operations.outstanding_task_reservations(), 0);
        },
    );
}

#[test]
fn recovered_pairwise_delegate_rejects_foreign_completed_context_before_retention() {
    with_delegate_dispatch_input_named(
        context_only_custody_model(),
        "pairwise-a2a-context-owner",
        |dispatch,
         recovered,
         _completed_context,
         _checkpoint_schema,
         _checkpoint_authority,
         _provider_clock,
         _resolve_store,
         authority,
         custody,
         anchor,
         parents,
         _parent_request,
         restart_generation,
         _initial_candidate| {
            let selected_store = recovered.start.control.observation().store().clone();
            let selector = recovered.start.selector.clone();
            let (sessions, _) = super::restart::cold_restart_sessions(parents, &selected_store);
            let application_store = MemoryAuthorityStore::default();
            let mut host = super::restart::host_with_generation(
                &application_store,
                &selector,
                &selected_store,
                restart_generation,
            );
            let exchange = A2aProcessExchangePort::new(A2aProcessExchangeMode::Response(
                A2aExchangeFacts::DirectMessage,
            ));
            let selection_ref = dispatch.operation.context.selection_ref.clone();
            let observation_ref = dispatch.operation.context.observation_ref.clone();
            let maximum_selection_bytes = dispatch.operation.context.maximum_selection_bytes;
            let signing = dispatch.signing;
            with_delegate_dispatch_input_named(
                context_only_custody_model(),
                "pairwise-a2a-foreign-context",
                |_foreign_dispatch,
                 _foreign_recovered,
                 foreign_completed_context,
                 _foreign_checkpoint_schema,
                 _foreign_checkpoint_authority,
                 _foreign_provider_clock,
                 _foreign_resolve_store,
                 _foreign_authority,
                 _foreign_custody,
                 _foreign_anchor,
                 _foreign_parents,
                 _foreign_parent_request,
                 _foreign_restart_generation,
                 _foreign_initial_candidate| {
                    let result = host.launch_recovered_pairwise_a2a_delegate(
                        recovered,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions: &sessions,
                            anchor,
                        },
                        crate::resident::process_host::operation_resolution::ResidentProcessOperationResolutionInput {
                            session: parents.session,
                            current: parents.current,
                            signing,
                            selection_ref,
                            observation_ref,
                            maximum_selection_bytes,
                            maximum_record_bytes: 65_536,
                        },
                        Some(foreign_completed_context),
                        authority,
                        dispatch,
                        || -> ResidentProcessA2aDelegateLaunchPlan<'_> {
                            unreachable!("a foreign context cannot select an A2A route")
                        },
                        &exchange,
                        |_, _, _, _| unreachable!("provider retention must not start"),
                        |_, _, _| unreachable!("provider observation must not be signed"),
                        |_, _, _| unreachable!("provider checkpoint must not be signed"),
                        |_, _, _| unreachable!("A2A disclosure must not run"),
                        |_, _, _| unreachable!("A2A transport admission must not run"),
                        |_, _, _| unreachable!("A2A checkpoint must not be signed"),
                    );
                    assert!(matches!(
                        result,
                        Err(crate::resident::process_host::operation_resolution::ResidentProcessOperationResolutionFailure::Admission(
                            crate::resident::OperationAttemptError::Association
                        ))
                    ));
                    assert_eq!(exchange.preparations.get(), 0);
                    assert_eq!(exchange.starts.get(), 0);
                    assert_eq!(host.a2a_delegate_routes.reserved_count(), 0);
                    assert_eq!(host.operations.outstanding_task_reservations(), 0);
                },
            );
        },
    );
}

#[test]
fn checked_out_a2a_route_holds_capacity_until_durable_handling_finishes() {
    let selected_store = store("control-store", "incarnation-1");
    let selector = unproved_selector(principal("principal-a"), selected_store.clone());
    let key = task_key(selected_store, "a2a/dispatch-1", 'a');
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut routes = ResidentProcessA2aDelegateRoutes::new(
        owner,
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
    )
    .expect("preallocated A2A routes");

    let reservation = routes.reserve(selector.clone()).expect("first route slot");
    assert!(routes.commit(reservation, key.clone()).is_ok());
    assert_eq!(routes.selector(&key), Some(&selector));

    let custody = routes.check_out(&key).expect("terminal route custody");
    assert!(routes.selector(&key).is_none());
    assert!(routes.reserve(selector.clone()).is_err());

    assert!(routes.release(custody).is_ok());
    assert!(routes.reserve(selector).is_ok());
}

#[test]
fn unrouted_a2a_cancel_relinquishes_only_its_exact_process_route_for_recovery() {
    let selected_store = store("control-store", "incarnation-1");
    let selector = unproved_selector(principal("principal-a"), selected_store.clone());
    let other_selector = unproved_selector(
        principal("principal-a"),
        store("control-store-b", "incarnation-1"),
    );
    let key = task_key(selected_store, "a2a/unrouted-cancel", 'a');
    let mut routes = ResidentProcessA2aDelegateRoutes::new(
        ResidentOperationTaskOwnerInstance::new(),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
    )
    .expect("preallocated A2A routes");

    let reservation = routes.reserve(selector.clone()).expect("exact route slot");
    assert!(routes.commit(reservation, key.clone()).is_ok());
    assert!(!routes.relinquish_unrouted_to_recovery(&key, &other_selector));
    assert_eq!(routes.selector(&key), Some(&selector));

    assert!(routes.relinquish_unrouted_to_recovery(&key, &selector));
    assert_eq!(routes.active_count(), 0);
    assert_eq!(routes.reserved_count(), 0);
    assert_eq!(routes.checked_out_count(), 0);
    assert!(routes.reserve(selector).is_ok());
}

#[test]
fn a2a_routes_keep_store_qualified_dispatches_on_their_exact_deployments() {
    let first_store = store("control-store-a", "incarnation-1");
    let second_store = store("control-store-b", "incarnation-1");
    let first_selector = unproved_selector(principal("principal-a"), first_store.clone());
    let second_selector = unproved_selector(principal("principal-a"), second_store.clone());
    let first_key = task_key(first_store, "a2a/dispatch", 'a');
    let second_key = task_key(second_store, "a2a/dispatch", 'a');
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut routes = ResidentProcessA2aDelegateRoutes::new(
        owner,
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(2).unwrap()),
    )
    .expect("preallocated A2A routes");

    let first = routes.reserve(first_selector.clone()).expect("first route");
    assert!(routes.commit(first, first_key.clone()).is_ok());
    let second = routes
        .reserve(second_selector.clone())
        .expect("second route");
    assert!(routes.commit(second, second_key.clone()).is_ok());

    assert_eq!(routes.selector(&first_key), Some(&first_selector));
    assert_eq!(routes.selector(&second_key), Some(&second_selector));
    assert_ne!(first_key, second_key);
}

#[test]
fn foreign_a2a_route_reservation_is_returned_without_changing_either_table() {
    let selected_store = store("control-store", "incarnation-1");
    let selector = unproved_selector(principal("principal-a"), selected_store.clone());
    let key = task_key(selected_store, "a2a/dispatch-1", 'a');
    let mut first = ResidentProcessA2aDelegateRoutes::new(
        ResidentOperationTaskOwnerInstance::new(),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
    )
    .unwrap();
    let mut second = ResidentProcessA2aDelegateRoutes::new(
        ResidentOperationTaskOwnerInstance::new(),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
    )
    .unwrap();

    let reservation = first.reserve(selector.clone()).unwrap();
    let failure = second.commit(reservation, key).unwrap_err();
    let ResidentProcessA2aDelegateRouteCommitFailure::ForeignReservation(reservation) = failure
    else {
        panic!("the foreign reservation must remain returnable")
    };
    assert!(first.release_reservation(*reservation).is_ok());
    assert!(first.reserve(selector.clone()).is_ok());
    assert!(second.reserve(selector).is_ok());
}

pub(super) struct A2aProcessClock {
    values: Mutex<VecDeque<Result<UnixTimeV1, WallClockSourceError>>>,
}

impl A2aProcessClock {
    pub(super) fn new(values: impl IntoIterator<Item = Result<u64, WallClockSourceError>>) -> Self {
        Self {
            values: Mutex::new(
                values
                    .into_iter()
                    .map(|value| value.map(|value| UnixTimeV1::new(value).unwrap()))
                    .collect(),
            ),
        }
    }
}

impl WallClockSourcePort for A2aProcessClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.values
            .lock()
            .unwrap()
            .pop_front()
            .expect("one modeled A2A process-host time")
    }
}

struct A2aProcessExchange {
    coordinate: A2aExchangeCoordinate,
    response: Option<A2aExchangeEvidence>,
    poll_error: Option<A2aExchangeError>,
}

impl A2aExchange for A2aProcessExchange {
    fn coordinate(&self) -> &A2aExchangeCoordinate {
        &self.coordinate
    }

    fn poll_next(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<A2aExchangeEvidence, A2aExchangeError>>> {
        if let Some(error) = self.poll_error.take() {
            return Poll::Ready(Some(Err(error)));
        }
        Poll::Ready(self.response.take().map(Ok))
    }

    fn cancel(&mut self) {}
}

#[derive(Clone)]
enum A2aProcessExchangeMode {
    Response(A2aExchangeFacts),
    Reject(A2aExchangeError),
    PollError(A2aExchangeError),
}

pub(super) struct A2aProcessExchangePort {
    preparations: Cell<usize>,
    starts: Cell<usize>,
    mode: A2aProcessExchangeMode,
}

impl A2aProcessExchangePort {
    fn new(mode: A2aProcessExchangeMode) -> Self {
        Self {
            preparations: Cell::new(0),
            starts: Cell::new(0),
            mode,
        }
    }

    pub(super) fn direct_message() -> Self {
        Self::new(A2aProcessExchangeMode::Response(
            A2aExchangeFacts::DirectMessage,
        ))
    }

    pub(super) fn start_count(&self) -> usize {
        self.starts.get()
    }
}

impl A2aExchangePort for A2aProcessExchangePort {
    fn prepare(
        &self,
        request: A2aExchangeRequest,
    ) -> Result<A2aPreparedExchange, A2aExchangeError> {
        self.preparations.set(self.preparations.get() + 1);
        A2aPreparedExchange::from_encoded(
            request,
            br#"{"jsonrpc":"2.0","id":"rpc-1","method":"message/send"}"#.to_vec(),
            ArtifactDigest::hash_bytes(b"process-host TLS trust"),
        )
    }

    fn start(
        &self,
        prepared: A2aPreparedExchange,
    ) -> Result<Box<dyn A2aExchange>, A2aExchangeError> {
        self.starts.set(self.starts.get() + 1);
        if let A2aProcessExchangeMode::Reject(reason) = self.mode {
            return Err(reason);
        }
        let (response, poll_error) = match &self.mode {
            A2aProcessExchangeMode::Response(facts) => (
                Some(A2aExchangeEvidence {
                    facts: facts.clone(),
                    bytes: br#"{"jsonrpc":"2.0","id":"rpc-1","result":{}}"#.to_vec(),
                    sse: None,
                }),
                None,
            ),
            A2aProcessExchangeMode::PollError(error) => (None, Some(*error)),
            A2aProcessExchangeMode::Reject(_) => unreachable!("handled before exchange creation"),
        };
        Ok(Box::new(A2aProcessExchange {
            coordinate: prepared.coordinate().clone(),
            response,
            poll_error,
        }))
    }
}

struct ExactA2aProcessProtocol(A2aExchangeFacts);

impl A2aSendMessageEvidencePort for ExactA2aProcessProtocol {
    fn read_initial_response(
        &self,
        _: &[u8],
        _: &[u8],
        _: &A2aJsonRpcRequestIdV1,
        _: NonZeroUsize,
    ) -> Result<A2aExchangeFacts, A2aExchangeError> {
        Ok(self.0.clone())
    }
}

pub(super) struct UnusedA2aSessions;

impl SessionLeaseCustodyPort for UnusedA2aSessions {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("A2A ProcessHost composition does not renew a session")
    }

    fn read(
        &self,
        _: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        panic!("A2A ProcessHost composition uses its admitted session context")
    }
}

#[derive(Clone, Copy, Debug)]
enum A2aProcessTerminalCase {
    Task,
    NoTask,
    NotStarted,
    OutcomeUnknown,
}

#[derive(Clone, Copy)]
enum A2aProcessLaunchFence {
    Selector,
    Store,
    Operation,
    Session,
    Checkpoint,
    Route,
    DuplicateRoute,
}

#[derive(Clone, Copy)]
enum A2aProcessRetentionMode {
    Normal,
    PublicInputRefusal,
    LaunchCustodyContinuation,
    RejectFinalization,
    RejectRouteRelease,
    DatabaseCustodyContinuation,
    AnchorCustodyContinuation,
    CrossedCustodyContinuation,
    CrossedRoute,
    CrossedFinalization,
    CrossedRouteRelease,
    LaunchRollback,
    StartedAssociation,
    OfferedCompletion,
    PollSettlement,
    DiagnosticRecovery,
    RecoveredLoss,
}

#[derive(Default)]
struct CompletedDelegateInputCalls {
    task: Cell<usize>,
    no_task: Cell<usize>,
    not_started: Cell<usize>,
    outcome_unknown: Cell<usize>,
}

impl CompletedDelegateInputCalls {
    fn counts(&self) -> [usize; 4] {
        [
            self.task.get(),
            self.no_task.get(),
            self.not_started.get(),
            self.outcome_unknown.get(),
        ]
    }
}

struct RefusingCompletedDelegateInputs(Rc<CompletedDelegateInputCalls>);

struct ExactExpiryClock {
    now: UnixTimeV1,
    reads: AtomicUsize,
}

impl WallClockSourcePort for ExactExpiryClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(self.now)
    }
}

struct ExpiredDelegateResolutionInputs<'a> {
    clock: &'a ExactExpiryClock,
    signing_authority: &'a SigningAuthorityBindingV1,
    observation_schema: &'a SchemaDigest,
    checkpoint_schema: &'a SchemaDigest,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    resolver: &'a dyn OperationObservationAuthorityStoreResolver,
    expiry_loans: Rc<Cell<usize>>,
    launch_loans: Rc<Cell<usize>>,
    observation_signs: Rc<Cell<usize>>,
    checkpoint_signs: Rc<Cell<usize>>,
}

impl<'service> crate::resident::ResidentServiceDelegateInputPort<'service>
    for ExpiredDelegateResolutionInputs<'service>
{
    type Authority = ProvingAuthority;

    fn with_pre_dispatch_expiry_preparation_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(
            crate::resident::ResidentServicePreDispatchExpiryPreparationInputs<'input>,
        ) -> Output,
    {
        self.expiry_loans.set(self.expiry_loans.get() + 1);
        let observation_signs = Rc::clone(&self.observation_signs);
        let checkpoint_signs = Rc::clone(&self.checkpoint_signs);
        let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
        let mut sign_observation = move |_: &[u8], _: &_, _: &_| {
            observation_signs.set(observation_signs.get() + 1);
            Ok(signature())
        };
        let mut sign_checkpoint = move |_: &[u8], _: &_, _: &_| {
            checkpoint_signs.set(checkpoint_signs.get() + 1);
            Ok(signature())
        };
        Ok(use_inputs(
            crate::resident::ResidentServicePreDispatchExpiryPreparationInputs {
                clock: self.clock,
                diagnostic_ref: record_ref("pairwise-a2a-expiry/diagnostic"),
                observation_ref: record_ref("pairwise-a2a-expiry/observation"),
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: self.observation_schema,
                    signing_authority: self.signing_authority,
                },
                checkpoint: crate::resident::OperationCheckpointInput {
                    reference: record_ref("pairwise-a2a-expiry/checkpoint"),
                    id: ControlCheckpointIdV1::parse("pairwise-a2a-expiry-checkpoint")
                        .expect("bounded checkpoint identity"),
                },
                checkpoint_schema: self.checkpoint_schema,
                checkpoint_authority: self.checkpoint_authority,
                resolve_store: self.resolver,
                maximum_diagnostic_bytes: 65_536,
                maximum_record_bytes: 1 << 20,
                accumulator_limits: AccumulatorPreparationLimits::new(
                    8, 20_000, 20_000, 8_192, 200_000,
                )
                .expect("expiry accumulator limits"),
                admit_rotation: &mut admit_rotation,
                sign_observation: &mut sign_observation,
                sign_checkpoint: &mut sign_checkpoint,
            },
        ))
    }

    fn with_pre_dispatch_expiry_continuation_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        use_inputs: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(
            crate::resident::ResidentServicePreDispatchExpiryContinuationInputs<'input>,
        ) -> Output,
    {
        let checkpoint_signs = Rc::clone(&self.checkpoint_signs);
        let mut sign_checkpoint = move |_: &[u8], _: &_, _: &_| {
            checkpoint_signs.set(checkpoint_signs.get() + 1);
            Ok(signature())
        };
        Ok(use_inputs(
            crate::resident::ResidentServicePreDispatchExpiryContinuationInputs {
                checkpoint_schema: self.checkpoint_schema,
                checkpoint_authority: self.checkpoint_authority,
                resolve_store: self.resolver,
                accumulator_limits: AccumulatorPreparationLimits::new(
                    8, 20_000, 20_000, 8_192, 200_000,
                )
                .expect("expiry accumulator limits"),
                sign_checkpoint: &mut sign_checkpoint,
            },
        ))
    }

    fn with_initial_launch_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::ResidentServiceA2aDelegateLaunchInputs<'service, Self::Authority>,
        ) -> Output,
    {
        self.launch_loans.set(self.launch_loans.get() + 1);
        Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_pre_task_recovery_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::ResidentServiceA2aPreTaskRecoveryInputs<'service, Self::Authority>,
        ) -> Output,
    {
        Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_task_observation_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::ResidentServiceA2aTaskObservationInputs<'service, Self::Authority>,
        ) -> Output,
    {
        Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_cancel_recovery_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs:
            FnOnce(crate::resident::ResidentServiceA2aCancelRecoveryInputs<'service>) -> Output,
    {
        Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
    }
}

#[test]
fn pre_dispatch_expiry_blocks_a2a_send_for_delegate_resolution() {
    with_delegate_dispatch_input_named(
        A2aTaskOwnerCustodyModel {
            clock_times: Vec::new(),
            expected_preparations: 2,
            expected_commits: 2,
            expected_read_backs: 2,
            expected_anchor_advances: 2,
            expected_anchor_observations: 2,
            unknown_exact_commits: &[],
            unavailable_read_backs: &[],
            unavailable_anchor_advances: &[],
            unknown_exact_anchor_advances: &[],
            unknown_failed_observation_anchor_advances: &[],
            later_anchor_observations: Vec::new(),
            fail_a2a_preflight_once: false,
        },
        "pairwise-a2a-expiry",
        |dispatch,
         recovered,
         completed_context,
         checkpoint_schema,
         checkpoint_authority,
         _provider_clock,
         resolve_store,
         _authority,
         custody,
         anchor,
         parents,
         _parent_request,
         restart_generation,
         _initial_candidate| {
            let operation = dispatch.operation.context.operation;
            let selected_store = recovered.start.control.observation().store().clone();
            let selector = recovered.start.selector.clone();
            let (sessions, _) = super::restart::cold_restart_sessions(parents, &selected_store);
            let application_store = MemoryAuthorityStore::default();
            let mut host = super::restart::host_with_generation(
                &application_store,
                &selector,
                &selected_store,
                restart_generation,
            );
            let checkpoint_record = completed_context
                .custody()
                .candidate()
                .checkpoint()
                .successor_record();
            let checkpoint_wire: ControlStateCheckpointWireV1 = serde_json::from_value(
                pale_core::canon::parse_strict(checkpoint_record.canonical_record_jcs().as_slice())
                    .expect("context checkpoint JSON"),
            )
            .expect("context checkpoint wire");
            let context_checkpoint = ControlStateCheckpointV1::admit_signed_wire(
                checkpoint_wire,
                checkpoint_schema,
                checkpoint_authority,
                |_, _, _, actual| {
                    assert_eq!(actual, &signature());
                    Ok(())
                },
            )
            .expect("the context checkpoint admits");
            let context_observation = completed_context.observation();
            let recovered_operation = crate::resident::RecoveredLocalOperation::fixture(
                operation,
                dispatch.operation.context.request.clone(),
                selected_store.clone(),
            );
            let recovered_observation = RecoveredOperationObservation::fixture(
                operation,
                (context_observation.0.clone(), context_observation.1.clone()),
                (context_observation.0.clone(), context_observation.1.clone()),
                (
                    checkpoint_record.coordinate().record_ref().clone(),
                    context_checkpoint,
                ),
                selected_store.clone(),
            );
            let recovered = crate::resident::process_host::restart::ResidentProcessRecoveredOperationResolution::pairwise_fixture(
                selector.clone(),
                crate::resident::RecoveredOperationControl::fixture(
                    recovered_operation,
                    recovered_observation,
                ),
                parents
                    .current
                    .generation
                    .compiled_generation
                    .compiled_generation_digest()
                    .clone(),
                operation.handle().0,
                operation.handle().1,
                crate::resident::process_host::restart::ResidentProcessOperationResolutionPhase::ContextBound,
            );
            let expiry_clock = ExactExpiryClock {
                now: *operation.handle().1.expires_at(),
                reads: AtomicUsize::new(0),
            };
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let expiry_loans = Rc::new(Cell::new(0));
            let launch_loans = Rc::new(Cell::new(0));
            let observation_signs = Rc::new(Cell::new(0));
            let checkpoint_signs = Rc::new(Cell::new(0));
            let mut adapter = crate::resident::process_host::service_assembly::ResidentServiceDelegateRecoveryAdapter::new(
                ExpiredDelegateResolutionInputs {
                    clock: &expiry_clock,
                    signing_authority: parents.signing,
                    observation_schema: &observation_schema,
                    checkpoint_schema,
                    checkpoint_authority,
                    resolver: resolve_store,
                    expiry_loans: Rc::clone(&expiry_loans),
                    launch_loans: Rc::clone(&launch_loans),
                    observation_signs: Rc::clone(&observation_signs),
                    checkpoint_signs: Rc::clone(&checkpoint_signs),
                },
                1,
            );
            let locator = operation_frame_locator(operation);
            let outcome = crate::resident::process_host::a2a_delegate_recovery_owner::ResidentA2aDelegateRecoveryPort::recover_delegate_resolution(
                &mut adapter,
                &mut host,
                Box::new(recovered),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions: &sessions,
                    anchor,
                },
                crate::resident::process_host::program::ResidentProgramDelegateResolutionInput::new(
                    dispatch.operation.context.request,
                    dispatch.operation.context.session,
                    dispatch.operation.context.current,
                    dispatch.operation.context.maximum_selection_bytes,
                    1 << 20,
                ),
            )
            .expect("the expired Delegate reaches durable local custody");

            let ResidentOperationRequestOutcome::Pending(actual_locator) = outcome else {
                panic!("expiry keeps the operation point-addressable")
            };
            assert_eq!(actual_locator, locator);
            assert_eq!(expiry_loans.get(), 1);
            assert_eq!(expiry_clock.reads.load(Ordering::SeqCst), 1);
            assert_eq!(observation_signs.get(), 1);
            assert_eq!(checkpoint_signs.get(), 1);
            assert_eq!(launch_loans.get(), 0, "expiry must precede A2A access");
            assert_eq!(host.operations.outstanding_task_reservations(), 0);
        },
    );
}

impl<'service> crate::resident::ResidentServiceDelegateInputPort<'service>
    for RefusingCompletedDelegateInputs
{
    type Authority = ProvingAuthority;

    fn with_initial_launch_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::ResidentServiceA2aDelegateLaunchInputs<'service, Self::Authority>,
        ) -> Output,
    {
        Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_pre_task_recovery_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::ResidentServiceA2aPreTaskRecoveryInputs<'service, Self::Authority>,
        ) -> Output,
    {
        Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_task_observation_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::ResidentServiceA2aTaskObservationInputs<'service, Self::Authority>,
        ) -> Output,
    {
        Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_cancel_recovery_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs:
            FnOnce(crate::resident::ResidentServiceA2aCancelRecoveryInputs<'service>) -> Output,
    {
        Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_task_outcome_inputs<Output, UseInputs>(
        self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        response: Box<crate::resident::CompletedA2aTaskResponse>,
        _: UseInputs,
    ) -> (
        Self,
        Result<
            Output,
            (
                pale_ports::ResidentOperationFrameFailure,
                Box<crate::resident::CompletedA2aTaskResponse>,
            ),
        >,
    )
    where
        UseInputs: FnOnce(crate::resident::A2aTaskOutcomeRetention<'service>) -> Output,
    {
        self.0.task.set(self.0.task.get() + 1);
        (
            self,
            Err((
                pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable,
                response,
            )),
        )
    }

    fn with_no_task_outcome_inputs<Output, UseInputs>(
        self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        response: Box<crate::resident::CompletedA2aNoTaskResponse>,
        _: UseInputs,
    ) -> (
        Self,
        Result<
            Output,
            (
                pale_ports::ResidentOperationFrameFailure,
                Box<crate::resident::CompletedA2aNoTaskResponse>,
            ),
        >,
    )
    where
        UseInputs: FnOnce(crate::resident::A2aNoTaskOutcomeRetention<'service>) -> Output,
    {
        self.0.no_task.set(self.0.no_task.get() + 1);
        (
            self,
            Err((
                pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable,
                response,
            )),
        )
    }

    fn with_not_started_outcome_inputs<Output, UseInputs>(
        self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        response: Box<crate::resident::A2aSendNotStarted>,
        _: UseInputs,
    ) -> (
        Self,
        Result<
            Output,
            (
                pale_ports::ResidentOperationFrameFailure,
                Box<crate::resident::A2aSendNotStarted>,
            ),
        >,
    )
    where
        UseInputs: FnOnce(crate::resident::A2aNotStartedOutcomeRetention<'service>) -> Output,
    {
        self.0.not_started.set(self.0.not_started.get() + 1);
        (
            self,
            Err((
                pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable,
                response,
            )),
        )
    }

    fn with_unknown_outcome_inputs<Output, UseInputs>(
        self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        outcome: Box<crate::resident::A2aSendOutcomeUnknown>,
        _: UseInputs,
    ) -> (
        Self,
        Result<
            Output,
            (
                pale_ports::ResidentOperationFrameFailure,
                Box<crate::resident::A2aSendOutcomeUnknown>,
            ),
        >,
    )
    where
        UseInputs: FnOnce(crate::resident::A2aUnknownOutcomeRetention<'service>) -> Output,
    {
        self.0.outcome_unknown.set(self.0.outcome_unknown.get() + 1);
        (
            self,
            Err((
                pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable,
                outcome,
            )),
        )
    }
}

enum A2aProcessTerminal<'a> {
    Carrier(ResidentProcessA2aDelegateCompletion),
    Recovered(ResidentProcessRecoveredA2aDelegate<'a>),
}

enum A2aProcessLaunch<'a> {
    Routed(ResidentProcessA2aDelegateLaunch),
    Recovered(ResidentProcessRecoveredA2aDelegate<'a>),
}

enum A2aProcessOutcome<'a> {
    Carrier(crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion),
    Recovered(crate::resident::a2a_delegate_task_service::ResidentRecoveredA2aDelegate<'a>),
}

struct A2aProcessAccess<'a> {
    selected_store: &'a AuthorityStoreCoordinate,
    unlike_store: &'a AuthorityStoreCoordinate,
    selector: &'a ResidentDeploymentAuthoritySelector,
    sessions: &'a UnusedA2aSessions,
    custody: &'a mut dyn pale_ports::OperationCustodyPort,
    anchor: &'a mut dyn pale_ports::AntiRollbackAnchorPort,
}

impl A2aProcessAccess<'_> {
    fn selected(&mut self) -> ResidentDeploymentOperationAccess<'_> {
        ResidentDeploymentOperationAccess {
            store: self.selected_store,
            custody: &mut *self.custody,
            sessions: self.sessions,
            anchor: &mut *self.anchor,
        }
    }

    fn unlike(&mut self) -> ResidentDeploymentOperationAccess<'_> {
        ResidentDeploymentOperationAccess {
            store: self.unlike_store,
            custody: &mut *self.custody,
            sessions: self.sessions,
            anchor: &mut *self.anchor,
        }
    }
}

enum A2aProcessRoutedTerminal {
    Completed(Box<ResidentProcessA2aDelegateCompletion>),
    Released,
}

struct A2aProcessRetentionAccess<'fixture, 'store, WriterGuard> {
    host: &'fixture mut ResidentProcessHost<'store, WriterGuard>,
    selected_store: &'fixture AuthorityStoreCoordinate,
    unlike_store: &'fixture AuthorityStoreCoordinate,
    selector: &'fixture ResidentDeploymentAuthoritySelector,
    sessions: &'fixture UnusedA2aSessions,
    authority: &'fixture mut ProvingAuthority,
    custody: &'fixture mut dyn pale_ports::OperationCustodyPort,
    anchor: &'fixture mut dyn pale_ports::AntiRollbackAnchorPort,
    exchange: &'fixture A2aProcessExchangePort,
    outcome_observation_signs: &'fixture Cell<usize>,
    outcome_checkpoint_signs: &'fixture Cell<usize>,
    mode: A2aProcessRetentionMode,
    crossed_route: Option<ResidentProcessA2aDelegateRouteCustody>,
    correct_route: Option<ResidentProcessA2aDelegateRouteCustody>,
}

fn a2a_process_operation_access<'a>(
    store: &'a AuthorityStoreCoordinate,
    custody: &'a mut dyn pale_ports::OperationCustodyPort,
    sessions: &'a UnusedA2aSessions,
    anchor: &'a mut dyn pale_ports::AntiRollbackAnchorPort,
) -> ResidentDeploymentOperationAccess<'a> {
    ResidentDeploymentOperationAccess {
        store,
        custody,
        sessions,
        anchor,
    }
}

impl<WriterGuard> A2aProcessRetentionAccess<'_, '_, WriterGuard> {
    fn retain<'input>(
        &mut self,
        pending: ResidentProcessA2aDelegateRetention<'input>,
    ) -> Option<crate::resident::RetainedA2aOutcome> {
        macro_rules! access {
            ($store:expr) => {
                a2a_process_operation_access(
                    $store,
                    &mut *self.custody,
                    self.sessions,
                    &mut *self.anchor,
                )
            };
        }
        if matches!(
            self.mode,
            A2aProcessRetentionMode::RejectFinalization
                | A2aProcessRetentionMode::CrossedFinalization
        ) {
            let failure = self
                .host
                .retain_a2a_delegate_with_rejected_finalization_for_test(
                    access!(self.selected_store),
                    self.authority,
                    pending,
                );
            assert!(self
                .host
                .a2a_delegate_routes
                .reserve(self.selector.clone())
                .is_err());
            let failure = if matches!(self.mode, A2aProcessRetentionMode::CrossedFinalization) {
                let ResidentProcessA2aDelegateRetentionFailure::Finalization {
                    route: correct,
                    rejection,
                } = failure
                else {
                    panic!("the fixture must stop after durable retention")
                };
                let crossed = ResidentProcessA2aDelegateRetentionFailure::Finalization {
                    route: self
                        .crossed_route
                        .take()
                        .expect("the hostile route remains checked out"),
                    rejection,
                };
                let rejected = match self.host.retry_a2a_delegate_finalization(
                    access!(self.selected_store),
                    self.authority,
                    crossed,
                ) {
                    Ok(_) => panic!("crossed finalization must be rejected before I/O"),
                    Err(failure) => failure,
                };
                let ResidentProcessA2aDelegateRetentionFailure::Finalization {
                    route: crossed,
                    rejection,
                } = *rejected
                else {
                    panic!("crossed finalization must retain both capabilities")
                };
                assert!(self.host.a2a_delegate_routes.release(crossed).is_ok());
                ResidentProcessA2aDelegateRetentionFailure::Finalization {
                    route: correct,
                    rejection,
                }
            } else {
                failure
            };
            return Some(
                self.host
                    .retry_a2a_delegate_finalization(
                        access!(self.selected_store),
                        self.authority,
                        failure,
                    )
                    .unwrap_or_else(|_| panic!("the exact finalization retry must settle")),
            );
        }

        if matches!(
            self.mode,
            A2aProcessRetentionMode::RejectRouteRelease
                | A2aProcessRetentionMode::CrossedRouteRelease
        ) {
            let failure = self
                .host
                .retain_a2a_delegate_with_rejected_route_release_for_test(
                    access!(self.selected_store),
                    self.authority,
                    pending,
                );
            assert!(self
                .host
                .a2a_delegate_routes
                .reserve(self.selector.clone())
                .is_err());
            let failure = if matches!(self.mode, A2aProcessRetentionMode::CrossedRouteRelease) {
                let ResidentProcessA2aDelegateRetentionFailure::Route {
                    route: correct,
                    key,
                    kind,
                    retained,
                } = failure
                else {
                    panic!("the fixture must stop at process-route release")
                };
                let crossed = ResidentProcessA2aDelegateRetentionFailure::Route {
                    route: Box::new(
                        self.crossed_route
                            .take()
                            .expect("the hostile route remains checked out"),
                    ),
                    key,
                    kind,
                    retained,
                };
                let rejected = match self.host.retry_a2a_delegate_route_release(crossed) {
                    Ok(_) => panic!("crossed route release must retain both routes"),
                    Err(failure) => failure,
                };
                let ResidentProcessA2aDelegateRetentionFailure::Route {
                    route: crossed,
                    key,
                    kind,
                    retained,
                } = *rejected
                else {
                    panic!("crossed route release must remain retryable")
                };
                assert!(self.host.a2a_delegate_routes.release(*crossed).is_ok());
                ResidentProcessA2aDelegateRetentionFailure::Route {
                    route: correct,
                    key,
                    kind,
                    retained,
                }
            } else {
                failure
            };
            return Some(
                self.host
                    .retry_a2a_delegate_route_release(failure)
                    .unwrap_or_else(|_| panic!("the exact route release retry must settle")),
            );
        }

        if matches!(
            self.mode,
            A2aProcessRetentionMode::DatabaseCustodyContinuation
                | A2aProcessRetentionMode::AnchorCustodyContinuation
                | A2aProcessRetentionMode::CrossedCustodyContinuation
        ) {
            let failure = match self.host.retain_a2a_delegate_outcome(
                access!(self.selected_store),
                self.authority,
                pending,
            ) {
                Ok(_) => panic!("modeled custody uncertainty must stop retention"),
                Err(failure) => failure,
            };
            assert!(matches!(
                *failure,
                ResidentProcessA2aDelegateRetentionFailure::Retention { .. }
            ));
            assert!(self
                .host
                .a2a_delegate_routes
                .reserve(self.selector.clone())
                .is_err());
            let observation_signs = self.outcome_observation_signs.get();
            let checkpoint_signs = self.outcome_checkpoint_signs.get();
            let failure = if matches!(
                self.mode,
                A2aProcessRetentionMode::CrossedCustodyContinuation
            ) {
                let ResidentProcessA2aDelegateRetentionFailure::Retention {
                    route: correct,
                    key,
                    kind,
                    finalization,
                    failure,
                    ..
                } = *failure
                else {
                    panic!("the fixture must stop in physical custody")
                };
                let crossed = ResidentProcessA2aDelegateRetentionFailure::Retention {
                    route: self
                        .crossed_route
                        .take()
                        .expect("the hostile route remains checked out"),
                    key,
                    kind,
                    finalization,
                    failure,
                    reason: None,
                };
                let rejected = match self.host.continue_a2a_delegate_outcome_custody(
                    access!(self.selected_store),
                    self.authority,
                    crossed,
                ) {
                    Ok(_) => panic!("crossed custody must be rejected before I/O"),
                    Err(failure) => failure,
                };
                let ResidentProcessA2aDelegateRetentionFailure::Retention {
                    route: crossed,
                    key,
                    kind,
                    finalization,
                    failure,
                    ..
                } = *rejected
                else {
                    panic!("crossed custody must retain both capabilities")
                };
                assert!(self.host.a2a_delegate_routes.release(crossed).is_ok());
                Box::new(ResidentProcessA2aDelegateRetentionFailure::Retention {
                    route: correct,
                    key,
                    kind,
                    finalization,
                    failure,
                    reason: None,
                })
            } else {
                failure
            };
            let failure = match self.host.continue_a2a_delegate_outcome_custody(
                access!(self.unlike_store),
                self.authority,
                *failure,
            ) {
                Ok(_) => panic!("an unlike store cannot continue outcome custody"),
                Err(failure) => failure,
            };
            assert_eq!(self.outcome_observation_signs.get(), observation_signs);
            assert_eq!(self.outcome_checkpoint_signs.get(), checkpoint_signs);
            assert_eq!(self.exchange.preparations.get(), 1);
            assert_eq!(self.exchange.starts.get(), 1);
            return Some(
                self.host
                    .continue_a2a_delegate_outcome_custody(
                        access!(self.selected_store),
                        self.authority,
                        *failure,
                    )
                    .unwrap_or_else(|_| panic!("the exact outcome custody must continue")),
            );
        }

        if matches!(self.mode, A2aProcessRetentionMode::CrossedRoute) {
            let failure = match self.host.retain_a2a_delegate_outcome(
                access!(self.selected_store),
                self.authority,
                pending,
            ) {
                Ok(_) => panic!("crossed route custody must fail before retention"),
                Err(failure) => failure,
            };
            assert_eq!(self.outcome_observation_signs.get(), 0);
            assert_eq!(self.outcome_checkpoint_signs.get(), 0);
            let ResidentProcessA2aDelegateRetentionFailure::Selection { pending, reason } =
                *failure
            else {
                panic!("crossed route custody must fail as a process association")
            };
            assert!(matches!(
                reason,
                ResidentProcessHostSelectionError::A2aProcessRouteAssociation
            ));
            let ResidentProcessA2aDelegateRetention::Task {
                route: crossed_route,
                finalization,
                input,
            } = *pending
            else {
                panic!("the crossed Task branch must return unchanged")
            };
            assert!(self.host.a2a_delegate_routes.release(crossed_route).is_ok());
            let pending = ResidentProcessA2aDelegateRetention::Task {
                route: self
                    .correct_route
                    .take()
                    .expect("the original route remains recoverable"),
                finalization,
                input,
            };
            return Some(
                self.host
                    .retain_a2a_delegate_outcome(
                        access!(self.selected_store),
                        self.authority,
                        pending,
                    )
                    .unwrap_or_else(|_| panic!("the restored route must settle")),
            );
        }

        Some(
            self.host
                .retain_a2a_delegate_outcome(access!(self.selected_store), self.authority, pending)
                .unwrap_or_else(|_| panic!("the exact durable branch must settle")),
        )
    }
}

struct A2aProcessRetentionBuilder<'fixture, 'operation> {
    case: A2aProcessTerminalCase,
    operation: &'operation IssuedLocalOperation,
    recover_operation: &'fixture dyn Fn(
        (ResidentRecordRefV1, ControlStateCheckpointV1),
    ) -> RecoveredOperationObservation,
    route_admission: &'fixture A2aRouteAdmissionV1,
    route_ref: &'fixture ResidentRecordRefV1,
    request_id: &'fixture A2aJsonRpcRequestIdV1,
    retention_clock: &'fixture A2aProcessClock,
    signing: SignedResidentAdmissionContextV1<'fixture>,
    checkpoint_schema: &'fixture SchemaDigest,
    checkpoint_authority: &'fixture ControlCheckpointSigningAuthorityAdmissionContextV1<'fixture>,
    resolve_store: &'fixture dyn OperationObservationAuthorityStoreResolver,
    outcome_observation_signs: &'fixture Cell<usize>,
    outcome_checkpoint_signs: &'fixture Cell<usize>,
}

impl<'fixture> A2aProcessRetentionBuilder<'fixture, '_> {
    fn commit_input(&self) -> A2aSendObservationCommitInput<'fixture> {
        A2aSendObservationCommitInput {
            checkpoint: OperationCheckpointInput {
                reference: record_ref(match self.case {
                    A2aProcessTerminalCase::Task => "process-a2a/task-checkpoint",
                    A2aProcessTerminalCase::NoTask => "process-a2a/no-task-checkpoint",
                    A2aProcessTerminalCase::NotStarted => "process-a2a/not-started-checkpoint",
                    A2aProcessTerminalCase::OutcomeUnknown => {
                        "process-a2a/outcome-unknown-checkpoint"
                    }
                }),
                id: ControlCheckpointIdV1::parse(match self.case {
                    A2aProcessTerminalCase::Task => "process-a2a-task-checkpoint",
                    A2aProcessTerminalCase::NoTask => "process-a2a-no-task-checkpoint",
                    A2aProcessTerminalCase::NotStarted => "process-a2a-not-started-checkpoint",
                    A2aProcessTerminalCase::OutcomeUnknown => {
                        "process-a2a-outcome-unknown-checkpoint"
                    }
                })
                .unwrap(),
            },
            clock: self.retention_clock,
            checkpoint_schema: self.checkpoint_schema,
            checkpoint_authority: self.checkpoint_authority,
            accumulator_limits: AccumulatorPreparationLimits::new(
                8, 20_000, 20_000, 8_192, 200_000,
            )
            .unwrap(),
        }
    }

    fn failed_observation(&self, label: &str) -> A2aFailedSendObservationInput<'fixture> {
        A2aFailedSendObservationInput {
            diagnostic_ref: record_ref(&format!("process-a2a/{label}-diagnostic")),
            observation_ref: record_ref(&format!("process-a2a/{label}-observation")),
            clock: self.retention_clock,
            signing: self.signing,
            maximum_diagnostic_bytes: 65_536,
        }
    }

    fn unknown_observation(&self, recovered: bool) -> A2aOutcomeUnknownObservationInput<'fixture> {
        let (diagnostic, observation, reconciliation, digest) = if recovered {
            (
                "process-a2a/recovered-diagnostic",
                "process-a2a/recovered-observation",
                "process-a2a/recovered-reconciliation",
                'e',
            )
        } else {
            (
                "process-a2a/unknown-diagnostic",
                "process-a2a/unknown-observation",
                "process-a2a/reconciliation",
                'd',
            )
        };
        A2aOutcomeUnknownObservationInput {
            diagnostic_ref: record_ref(diagnostic),
            observation_ref: record_ref(observation),
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref(reconciliation),
                record_sha256: pale_core::records::ReconciliationFrontierDigest::parse(
                    digest.to_string().repeat(64),
                )
                .unwrap(),
            },
            clock: self.retention_clock,
            signing: self.signing,
            maximum_diagnostic_bytes: 65_536,
        }
    }

    fn retain<'input, WriterGuard>(
        &'input self,
        access: &mut A2aProcessRetentionAccess<'_, '_, WriterGuard>,
        route: ResidentProcessA2aDelegateRouteCustody,
        completion: A2aProcessOutcome<'input>,
    ) -> Option<crate::resident::RetainedA2aOutcome>
    where
        'fixture: 'input,
    {
        match (self.case, completion) {
            (
                A2aProcessTerminalCase::Task,
                A2aProcessOutcome::Carrier(
                    super::super::super::a2a_delegate_task_service::ResidentA2aDelegateCompletion::Task {
                        response,
                        finalization,
                    },
                ),
            ) => {
                let operation_custody = (self.recover_operation)((
                    response.send_checkpoint().0.clone(),
                    response.send_checkpoint().1.clone(),
                ));
                let (dispatch_ref, dispatch) = response.operation_observation();
                let (task, remote_context, remote_state) = response.task();
                let (request_evidence, response_evidence) = response.evidence();
                let binding_ref = record_ref("process-a2a/task-binding");
                let binding_schema = schema_digest(A2A_TASK_BINDING_V1_SCHEMA_ID);
                let binding = A2aTaskBindingPreimageV1 {
                    schema_id: SchemaId::parse(A2A_TASK_BINDING_V1_SCHEMA).unwrap(),
                    schema_sha256: binding_schema.clone(),
                    task_binding_id: A2aTaskBindingIdV1::parse("process-a2a-task").unwrap(),
                    operation_handle_id: self
                        .operation
                        .handle()
                        .1
                        .operation_handle_id()
                        .clone(),
                    operation_handle_ref: self.operation.handle().0.clone(),
                    operation_handle_sha256: self
                        .operation
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .clone(),
                    operation_observation_ref: dispatch_ref.clone(),
                    operation_observation_sha256: dispatch
                        .canonical_operation_observation_sha256()
                        .clone(),
                    route_admission_id: self.route_admission.route_admission_id().clone(),
                    route_admission_ref: self.route_ref.clone(),
                    route_admission_sha256: self
                        .route_admission
                        .canonical_a2a_route_admission_sha256()
                        .clone(),
                    attempt_id: dispatch.state().dispatched_attempt_id().unwrap().clone(),
                    json_rpc_request_id: self.request_id.clone(),
                    request_mode: pale_core::records::A2aTaskRequestModeV1::SendMessage,
                    remote_task_id: task.clone(),
                    remote_context_id: remote_context.clone(),
                    request_evidence: immutable_a2a_evidence(request_evidence),
                    initial_task_evidence: immutable_a2a_evidence(response_evidence),
                    bound_at: response.observed_at(),
                    deployment_runtime_authority_ref: self
                        .operation
                        .handle()
                        .1
                        .deployment_runtime_authority_ref()
                        .clone(),
                    deployment_runtime_authority_sha256: self
                        .operation
                        .handle()
                        .1
                        .deployment_runtime_authority_sha256()
                        .clone(),
                    issuer_principal_id: self
                        .operation
                        .handle()
                        .1
                        .issuer_principal_id()
                        .clone(),
                    issuer_key_id: self.operation.handle().1.issuer_key_id().clone(),
                    issuer_key_epoch: *self.operation.handle().1.issuer_key_epoch(),
                };
                let observation_schema = schema_digest(A2A_TASK_OBSERVATION_V1_SCHEMA_ID);
                let observation = A2aTaskObservationPreimageV1 {
                    schema_id: SchemaId::parse(A2A_TASK_OBSERVATION_V1_SCHEMA).unwrap(),
                    schema_sha256: observation_schema.clone(),
                    task_binding_id: binding.task_binding_id.clone(),
                    task_binding_ref: binding_ref.clone(),
                    task_binding_sha256: A2aTaskBindingDigest::hash_canonical(&binding).unwrap(),
                    revision: A2aTaskObservationRevisionV1::new(1).unwrap(),
                    predecessor_ref: None,
                    predecessor_sha256: None,
                    source: A2aTaskObservationSourceV1::InitialSendMessageResponse {
                        evidence: immutable_a2a_evidence(response_evidence),
                    },
                    remote_state: *remote_state,
                    reconciliation: None,
                    observed_at: response.observed_at(),
                    deployment_runtime_authority_ref: self
                        .operation
                        .handle()
                        .1
                        .deployment_runtime_authority_ref()
                        .clone(),
                    deployment_runtime_authority_sha256: self
                        .operation
                        .handle()
                        .1
                        .deployment_runtime_authority_sha256()
                        .clone(),
                    issuer_principal_id: self
                        .operation
                        .handle()
                        .1
                        .issuer_principal_id()
                        .clone(),
                    issuer_key_id: self.operation.handle().1.issuer_key_id().clone(),
                    issuer_key_epoch: *self.operation.handle().1.issuer_key_epoch(),
                };
                access.retain(ResidentProcessA2aDelegateRetention::Task {
                    route,
                    finalization,
                    input: Box::new(A2aTaskOutcomeRetention::new(
                        response,
                        A2aInitialTaskCommitInput {
                            task: A2aInitialTaskCustodyInput {
                                operation_custody: &operation_custody,
                                binding_ref,
                                binding,
                                binding_signing: SignedResidentAdmissionContextV1 {
                                    expected_schema_sha256: &binding_schema,
                                    signing_authority: self.signing.signing_authority,
                                },
                                observation_ref: record_ref("process-a2a/task-observation"),
                                observation,
                                observation_signing: SignedResidentAdmissionContextV1 {
                                    expected_schema_sha256: &observation_schema,
                                    signing_authority: self.signing.signing_authority,
                                },
                                maximum_evidence_bytes: 8_192,
                                maximum_record_bytes: 65_536,
                            },
                            checkpoint: self.commit_input().checkpoint,
                            clock: self.retention_clock,
                            checkpoint_schema: self.checkpoint_schema,
                            checkpoint_authority: self.checkpoint_authority,
                            accumulator_limits: self.commit_input().accumulator_limits,
                            membership_limits: pale_ports::AccumulatorMembershipLimits::new(
                                8, 20_000, 200_000,
                            )
                            .unwrap(),
                        },
                        |_, _, new, retained| {
                            assert_eq!(new.len(), 1);
                            assert_eq!(retained.len(), 1);
                            Ok(())
                        },
                        |_, _, _| Ok(signature()),
                        |_, _, _| {
                            self.outcome_observation_signs
                                .set(self.outcome_observation_signs.get() + 1);
                            Ok(signature())
                        },
                        |_| Ok(store_coordinate()),
                        |_, _, _| {
                            self.outcome_checkpoint_signs
                                .set(self.outcome_checkpoint_signs.get() + 1);
                            Ok(signature())
                        },
                    )),
                })
            }
            (
                A2aProcessTerminalCase::NoTask,
                A2aProcessOutcome::Carrier(
                    super::super::super::a2a_delegate_task_service::ResidentA2aDelegateCompletion::NoTask {
                        response,
                        finalization,
                    },
                ),
            ) => access.retain(ResidentProcessA2aDelegateRetention::NoTask {
                route,
                finalization,
                input: A2aNoTaskOutcomeRetention::new(
                    response,
                    self.failed_observation("no-task"),
                    self.commit_input(),
                    self.resolve_store,
                    |_, _, _, _| Ok(()),
                    |_, _, _| {
                        self.outcome_observation_signs
                            .set(self.outcome_observation_signs.get() + 1);
                        Ok(signature())
                    },
                    |_, _, _| {
                        self.outcome_checkpoint_signs
                            .set(self.outcome_checkpoint_signs.get() + 1);
                        Ok(signature())
                    },
                ),
            }),
            (
                A2aProcessTerminalCase::NotStarted,
                A2aProcessOutcome::Carrier(
                    super::super::super::a2a_delegate_task_service::ResidentA2aDelegateCompletion::NotStarted {
                        response,
                        finalization,
                    },
                ),
            ) => access.retain(ResidentProcessA2aDelegateRetention::NotStarted {
                route,
                finalization,
                input: A2aNotStartedOutcomeRetention::new(
                    response,
                    self.failed_observation("not-started"),
                    self.commit_input(),
                    self.resolve_store,
                    |_, _, _, _| Ok(()),
                    |_, _, _| {
                        self.outcome_observation_signs
                            .set(self.outcome_observation_signs.get() + 1);
                        Ok(signature())
                    },
                    |_, _, _| {
                        self.outcome_checkpoint_signs
                            .set(self.outcome_checkpoint_signs.get() + 1);
                        Ok(signature())
                    },
                ),
            }),
            (
                A2aProcessTerminalCase::OutcomeUnknown,
                A2aProcessOutcome::Carrier(
                    super::super::super::a2a_delegate_task_service::ResidentA2aDelegateCompletion::OutcomeUnknown {
                        outcome,
                        finalization,
                    },
                ),
            ) => access.retain(ResidentProcessA2aDelegateRetention::OutcomeUnknown {
                route,
                finalization,
                input: A2aUnknownOutcomeRetention::new(
                    outcome,
                    self.unknown_observation(false),
                    self.commit_input(),
                    self.resolve_store,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| {
                        self.outcome_observation_signs
                            .set(self.outcome_observation_signs.get() + 1);
                        Ok(signature())
                    },
                    |_, _, _| {
                        self.outcome_checkpoint_signs
                            .set(self.outcome_checkpoint_signs.get() + 1);
                        Ok(signature())
                    },
                ),
            }),
            (
                A2aProcessTerminalCase::OutcomeUnknown,
                A2aProcessOutcome::Recovered(
                    super::super::super::a2a_delegate_task_service::ResidentRecoveredA2aDelegate {
                        outcome,
                        finalization,
                    },
                ),
            ) => access.retain(ResidentProcessA2aDelegateRetention::RecoveredOutcomeUnknown {
                route,
                finalization,
                input: A2aRecoveredUnknownOutcomeRetention::new(
                    outcome,
                    self.unknown_observation(true),
                    self.commit_input(),
                    self.resolve_store,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| {
                        self.outcome_observation_signs
                            .set(self.outcome_observation_signs.get() + 1);
                        Ok(signature())
                    },
                    |_, _, _| {
                        self.outcome_checkpoint_signs
                            .set(self.outcome_checkpoint_signs.get() + 1);
                        Ok(signature())
                    },
                ),
            }),
            _ => panic!("the terminal carrier branch changed meaning"),
        }
    }
}

struct A2aProcessLaunchAccess<'fixture, 'store, WriterGuard> {
    host: &'fixture mut ResidentProcessHost<'store, WriterGuard>,
    selector: &'fixture ResidentDeploymentAuthoritySelector,
    selected_store: &'fixture AuthorityStoreCoordinate,
    unlike_store: &'fixture AuthorityStoreCoordinate,
    sessions: &'fixture UnusedA2aSessions,
    authority: &'fixture mut ProvingAuthority,
    custody: &'fixture mut dyn pale_ports::OperationCustodyPort,
    anchor: &'fixture mut dyn pale_ports::AntiRollbackAnchorPort,
    exchange: &'fixture A2aProcessExchangePort,
    response_clock: &'fixture A2aProcessClock,
    checkpoint_signs: &'fixture Cell<usize>,
    fence: Option<A2aProcessLaunchFence>,
    mode: A2aProcessRetentionMode,
}

impl<WriterGuard> A2aProcessLaunchAccess<'_, '_, WriterGuard> {
    fn launch<'operation, 'call>(
        &mut self,
        launch_selector: &ResidentDeploymentAuthoritySelector,
        access_store: &AuthorityStoreCoordinate,
        launch: A2aDelegateLaunchInput<'operation, 'call>,
    ) -> Option<ResidentProcessA2aDelegateLaunch> {
        let launched = self.host.launch_a2a_delegate(
            launch_selector,
            a2a_process_operation_access(
                access_store,
                &mut *self.custody,
                self.sessions,
                &mut *self.anchor,
            ),
            self.authority,
            launch,
            self.exchange,
            |_, _, _| Ok(()),
            |_, _, _| Ok(()),
            |_, _, _| {
                self.checkpoint_signs.set(self.checkpoint_signs.get() + 1);
                Ok(signature())
            },
        );
        match launched {
            Ok(launched) => Some(launched),
            Err(failure) if self.fence.is_some() => {
                if matches!(self.fence, Some(A2aProcessLaunchFence::DuplicateRoute)) {
                    assert!(matches!(
                        *failure,
                        ResidentProcessA2aDelegateLaunchFailure::RouteConflict(_)
                    ));
                } else {
                    let ResidentProcessA2aDelegateLaunchFailure::Selection { reason, .. } =
                        *failure
                    else {
                        panic!("a launch fence failed after route or carrier ownership")
                    };
                    assert!(
                        launch_fence_matches(self.fence.unwrap(), &reason),
                        "the exact requested A2A launch fence must fail"
                    );
                }
                assert_eq!(self.exchange.preparations.get(), 0);
                assert_eq!(self.exchange.starts.get(), 0);
                assert!(self
                    .host
                    .a2a_delegate_routes
                    .reserve(self.selector.clone())
                    .is_ok());
                None
            }
            Err(failure)
                if matches!(
                    self.mode,
                    A2aProcessRetentionMode::LaunchCustodyContinuation
                ) =>
            {
                assert_eq!(self.exchange.preparations.get(), 1);
                assert_eq!(self.exchange.starts.get(), 0);
                let pending = match self.host.continue_a2a_delegate_custody(
                    a2a_process_operation_access(
                        self.unlike_store,
                        &mut *self.custody,
                        self.sessions,
                        &mut *self.anchor,
                    ),
                    self.authority,
                    *failure,
                ) {
                    Ok(_) => panic!("an unlike store cannot continue A2A custody"),
                    Err(failure) => *failure,
                };
                let unopened = self
                    .host
                    .continue_a2a_delegate_custody(
                        a2a_process_operation_access(
                            self.selected_store,
                            &mut *self.custody,
                            self.sessions,
                            &mut *self.anchor,
                        ),
                        self.authority,
                        pending,
                    )
                    .unwrap_or_else(|_| panic!("the exact process route continues custody"));
                assert!(matches!(
                    unopened,
                    ResidentProcessA2aDelegateLaunchFailure::Unopened { .. }
                ));
                let unopened = match self.host.retry_unopened_a2a_delegate(
                    a2a_process_operation_access(
                        self.unlike_store,
                        &mut *self.custody,
                        self.sessions,
                        &mut *self.anchor,
                    ),
                    self.authority,
                    unopened,
                    self.exchange,
                    self.response_clock,
                    |_, _, _| Ok(()),
                ) {
                    Ok(_) => panic!("an unlike store cannot consume unopened custody"),
                    Err(failure) => *failure,
                };
                assert_eq!(self.exchange.preparations.get(), 1);
                assert_eq!(self.exchange.starts.get(), 0);
                Some(
                    self.host
                        .retry_unopened_a2a_delegate(
                            a2a_process_operation_access(
                                self.selected_store,
                                &mut *self.custody,
                                self.sessions,
                                &mut *self.anchor,
                            ),
                            self.authority,
                            unopened,
                            self.exchange,
                            self.response_clock,
                            |_, _, _| Ok(()),
                        )
                        .unwrap_or_else(|_| panic!("the exact unopened send starts once")),
                )
            }
            Err(failure) => match *failure {
                ResidentProcessA2aDelegateLaunchFailure::Selection { reason, .. } => {
                    panic!("process A2A selection failed: {reason}")
                }
                ResidentProcessA2aDelegateLaunchFailure::RouteConflict(_) => {
                    panic!("the process A2A route unexpectedly conflicted")
                }
                _ => panic!("the process host must launch exactly once"),
            },
        }
    }
}

fn settle_started_a2a_association<WriterGuard>(
    host: &mut ResidentProcessHost<'_, WriterGuard>,
    access: &mut A2aProcessAccess<'_>,
    authority: &mut ProvingAuthority,
    key: ResidentOperationTaskKey,
) -> ResidentProcessA2aDelegateCompletion {
    let failure = host.a2a_started_association_failure_for_test(access.selected(), authority, key);
    let failure =
        match host.settle_a2a_delegate_started_association(access.unlike(), authority, failure) {
            Ok(_) => panic!("an unlike store cannot settle the started join"),
            Err(failure) => *failure,
        };
    match host
        .settle_a2a_delegate_started_association(access.selected(), authority, failure)
        .unwrap_or_else(|_| panic!("the exact started join must settle"))
    {
        ResidentProcessA2aDelegateLaunch::Completed(completed) => completed,
        _ => panic!("settled started association must be terminal"),
    }
}

fn settle_offered_a2a_completion<WriterGuard>(
    host: &mut ResidentProcessHost<'_, WriterGuard>,
    access: &mut A2aProcessAccess<'_>,
    authority: &mut ProvingAuthority,
    key: ResidentOperationTaskKey,
) -> ResidentProcessA2aDelegateCompletion {
    let failure = host.a2a_offered_completion_failure_for_test(access.selected(), authority, key);
    let failure =
        match host.retry_a2a_delegate_offered_completion(access.unlike(), authority, failure) {
            Ok(_) => panic!("an unlike store cannot settle the offered completion"),
            Err(failure) => *failure,
        };
    match host
        .retry_a2a_delegate_offered_completion(access.selected(), authority, failure)
        .unwrap_or_else(|_| panic!("the exact offered completion must settle"))
    {
        ResidentProcessA2aDelegatePoll::Completed(completed) => completed,
        _ => panic!("settled offered completion must be terminal"),
    }
}

fn poll_a2a_process_terminal<WriterGuard>(
    host: &mut ResidentProcessHost<'_, WriterGuard>,
    access: &mut A2aProcessAccess<'_>,
    authority: &mut ProvingAuthority,
    key: ResidentOperationTaskKey,
    mode: A2aProcessRetentionMode,
    response_facts: A2aExchangeFacts,
    clock: &dyn WallClockSourcePort,
) -> ResidentProcessA2aDelegateCompletion {
    let selector = access.selector.clone();
    let waker = std::task::Waker::noop();
    let mut context = Context::from_waker(waker);
    if matches!(mode, A2aProcessRetentionMode::PollSettlement) {
        host.set_a2a_reservation_phase_for_test(
            access.selected(),
            authority,
            &key,
            crate::resident::task_reservation::ResidentTaskReservationPhase::Offered,
        );
    }
    let polled = host
        .poll_a2a_delegate(
            &selector,
            access.selected(),
            authority,
            &key,
            &mut context,
            &ExactA2aProcessProtocol(response_facts),
            clock,
        )
        .expect("the exact process route remains selected");
    let polled = if matches!(mode, A2aProcessRetentionMode::PollSettlement) {
        host.set_a2a_reservation_phase_for_test(
            access.selected(),
            authority,
            &key,
            crate::resident::task_reservation::ResidentTaskReservationPhase::Accepted,
        );
        let rejected = match host.retry_a2a_delegate_settlement(access.unlike(), authority, polled)
        {
            Ok(_) => panic!("an unlike store cannot retry poll settlement"),
            Err(rejected) => *rejected,
        };
        host.retry_a2a_delegate_settlement(access.selected(), authority, rejected)
            .unwrap_or_else(|_| panic!("the exact poll settlement must retry"))
    } else {
        polled
    };
    match polled {
        ResidentProcessA2aDelegatePoll::Completed(completed) => completed,
        _ => panic!("the modeled exchange must reach one terminal branch"),
    }
}

fn release_a2a_launch_rollback<WriterGuard>(
    host: &mut ResidentProcessHost<'_, WriterGuard>,
    access: &mut A2aProcessAccess<'_>,
    authority: &mut ProvingAuthority,
    completed: ResidentProcessA2aDelegateCompletion,
) {
    let failure = host.a2a_rollback_failure_for_test(access.selected(), authority, completed);
    let failure = match host.retry_a2a_delegate_launch_rollback(access.unlike(), authority, failure)
    {
        Ok(()) => panic!("an unlike store cannot release rollback capacity"),
        Err(failure) => *failure,
    };
    host.retry_a2a_delegate_launch_rollback(access.selected(), authority, failure)
        .unwrap_or_else(|_| panic!("the exact rollback must release capacity"));
}

fn settle_routed_a2a_process_launch<WriterGuard>(
    host: &mut ResidentProcessHost<'_, WriterGuard>,
    access: &mut A2aProcessAccess<'_>,
    authority: &mut ProvingAuthority,
    launched: ResidentProcessA2aDelegateLaunch,
    mode: A2aProcessRetentionMode,
    response_facts: A2aExchangeFacts,
    clock: &dyn WallClockSourcePort,
) -> A2aProcessRoutedTerminal {
    match launched {
        ResidentProcessA2aDelegateLaunch::Awaiting(key)
            if matches!(mode, A2aProcessRetentionMode::StartedAssociation) =>
        {
            A2aProcessRoutedTerminal::Completed(Box::new(settle_started_a2a_association(
                host, access, authority, key,
            )))
        }
        ResidentProcessA2aDelegateLaunch::Awaiting(key)
            if matches!(mode, A2aProcessRetentionMode::OfferedCompletion) =>
        {
            A2aProcessRoutedTerminal::Completed(Box::new(settle_offered_a2a_completion(
                host, access, authority, key,
            )))
        }
        ResidentProcessA2aDelegateLaunch::Awaiting(key) => {
            A2aProcessRoutedTerminal::Completed(Box::new(poll_a2a_process_terminal(
                host,
                access,
                authority,
                key,
                mode,
                response_facts,
                clock,
            )))
        }
        ResidentProcessA2aDelegateLaunch::Completed(completed)
            if matches!(mode, A2aProcessRetentionMode::LaunchRollback) =>
        {
            release_a2a_launch_rollback(host, access, authority, completed);
            A2aProcessRoutedTerminal::Released
        }
        ResidentProcessA2aDelegateLaunch::Completed(completed) => {
            A2aProcessRoutedTerminal::Completed(Box::new(completed))
        }
        ResidentProcessA2aDelegateLaunch::Diagnostic(_) => {
            panic!("the modeled diagnostic must be admissible")
        }
    }
}

fn launch_fence_matches(
    fence: A2aProcessLaunchFence,
    reason: &ResidentProcessHostSelectionError,
) -> bool {
    match fence {
        A2aProcessLaunchFence::Selector => matches!(
            reason,
            ResidentProcessHostSelectionError::UnknownDeploymentAuthority
        ),
        A2aProcessLaunchFence::Store => matches!(
            reason,
            ResidentProcessHostSelectionError::AccessStoreAssociation
        ),
        A2aProcessLaunchFence::Operation => matches!(
            reason,
            ResidentProcessHostSelectionError::A2aOperationAssociation
        ),
        A2aProcessLaunchFence::Session => matches!(
            reason,
            ResidentProcessHostSelectionError::A2aCurrentAuthority
        ),
        A2aProcessLaunchFence::Checkpoint => matches!(
            reason,
            ResidentProcessHostSelectionError::CheckpointAuthorityAssociation
        ),
        A2aProcessLaunchFence::Route => {
            matches!(reason, ResidentProcessHostSelectionError::A2aRouteAuthority)
        }
        A2aProcessLaunchFence::DuplicateRoute => false,
    }
}

fn immutable_a2a_evidence(evidence: &OperationA2aEvidence) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: evidence.coordinate().record_ref().clone(),
        record_sha256: ArtifactDigest::parse(evidence.coordinate().record_sha256().as_str())
            .unwrap(),
    }
}

#[derive(Clone, Copy)]
enum RetainedTaskCallbackCustody {
    ReadOnly,
    OneCompletedAppend,
    TwoCompletedAppends,
}

impl RetainedTaskCallbackCustody {
    const fn completed_appends(self) -> usize {
        match self {
            Self::ReadOnly => 0,
            Self::OneCompletedAppend => 1,
            Self::TwoCompletedAppends => 2,
        }
    }
}

fn a2a_process_custody_model(
    fence: Option<A2aProcessLaunchFence>,
    retention_mode: A2aProcessRetentionMode,
    callback_custody: RetainedTaskCallbackCustody,
) -> A2aTaskOwnerCustodyModel<'static> {
    let expected_callback_appends = callback_custody.completed_appends();
    let expected_preparations = if fence.is_some() {
        4
    } else if matches!(retention_mode, A2aProcessRetentionMode::LaunchRollback) {
        5
    } else {
        6
    };
    let expected_read_backs = if fence.is_some() {
        4
    } else if matches!(retention_mode, A2aProcessRetentionMode::RecoveredLoss) {
        7
    } else if matches!(
        retention_mode,
        A2aProcessRetentionMode::DatabaseCustodyContinuation
    ) {
        8
    } else if matches!(retention_mode, A2aProcessRetentionMode::LaunchRollback) {
        5
    } else {
        6
    };
    let expected_anchor_advances = if fence.is_some() {
        4
    } else if matches!(
        retention_mode,
        A2aProcessRetentionMode::LaunchCustodyContinuation
            | A2aProcessRetentionMode::AnchorCustodyContinuation
            | A2aProcessRetentionMode::CrossedCustodyContinuation
    ) {
        7
    } else if matches!(retention_mode, A2aProcessRetentionMode::LaunchRollback) {
        5
    } else {
        6
    };
    let expected_anchor_observations = if fence.is_some() {
        4
    } else if matches!(retention_mode, A2aProcessRetentionMode::LaunchRollback) {
        5
    } else {
        6
    };
    A2aTaskOwnerCustodyModel {
        clock_times: vec![63, 64, 65],
        expected_preparations: expected_preparations + expected_callback_appends,
        expected_commits: expected_preparations + expected_callback_appends,
        expected_read_backs: expected_read_backs + expected_callback_appends,
        expected_anchor_advances: expected_anchor_advances + expected_callback_appends,
        expected_anchor_observations: expected_anchor_observations + expected_callback_appends,
        unknown_exact_commits: match retention_mode {
            A2aProcessRetentionMode::DatabaseCustodyContinuation => &[6],
            A2aProcessRetentionMode::RecoveredLoss => &[5],
            _ => &[],
        },
        unavailable_read_backs: match retention_mode {
            A2aProcessRetentionMode::DatabaseCustodyContinuation => &[6],
            _ => &[],
        },
        unavailable_anchor_advances: match retention_mode {
            A2aProcessRetentionMode::LaunchCustodyContinuation => &[5],
            A2aProcessRetentionMode::AnchorCustodyContinuation => &[6],
            A2aProcessRetentionMode::CrossedCustodyContinuation => &[6],
            _ => &[],
        },
        unknown_exact_anchor_advances: &[],
        unknown_failed_observation_anchor_advances: &[],
        later_anchor_observations: Vec::new(),
        fail_a2a_preflight_once: false,
    }
}

pub(super) fn joined_program_a2a_custody_model() -> A2aTaskOwnerCustodyModel<'static> {
    let mut model = a2a_process_custody_model(
        None,
        A2aProcessRetentionMode::Normal,
        RetainedTaskCallbackCustody::ReadOnly,
    );
    model.expected_preparations -= 1;
    model.expected_commits -= 1;
    model.expected_read_backs -= 1;
    model.expected_anchor_advances -= 1;
    model.expected_anchor_observations -= 1;
    model
}

/// Retained Task history produced by the real process-host A2A send path.
pub(in crate::resident) struct RetainedProcessA2aTaskFixture<'a, 'store> {
    pub(super) host: &'a mut ResidentProcessHost<'store, UnprovedHeldWriter>,
    pub(super) selector: ResidentDeploymentAuthoritySelector,
    pub(super) store: AuthorityStoreCoordinate,
    pub(super) sessions: &'a UnusedA2aSessions,
    pub(in crate::resident) history: RecoveredA2aTaskHistory,
    pub(in crate::resident) provider_recovery:
        &'a crate::resident::operation_dispatch::test_support::RetainedProviderRecoveryFixture<'a>,
    pub(in crate::resident) attempt: OperationAttemptHistoryInput<'a>,
    pub(in crate::resident) signing: &'a SigningAuthorityBindingV1,
    pub(in crate::resident) checkpoint: ControlStateCheckpointV1,
    pub(in crate::resident) checkpoint_ref: ResidentRecordRefV1,
    pub(super) authority: &'a mut ProvingAuthority,
    pub(in crate::resident) custody: &'a mut dyn pale_ports::OperationCustodyPort,
    pub(in crate::resident) anchor: &'a mut dyn pale_ports::AntiRollbackAnchorPort,
    pub(super) parents: crate::local_operation_start_fixture::RecoveryParents<'a>,
    pub(super) parent_request: (pale_ports::ControlRecordCoordinate, CommonRequestEnvelopeV1),
    pub(super) send_preparations: usize,
    pub(super) send_starts: usize,
}

fn with_a2a_send_through_process_route<R>(
    case: A2aProcessTerminalCase,
    fence: Option<A2aProcessLaunchFence>,
    retention_mode: A2aProcessRetentionMode,
    callback_custody: RetainedTaskCallbackCustody,
    run: impl for<'a, 'store> FnOnce(RetainedProcessA2aTaskFixture<'a, 'store>) -> R,
) -> Option<R> {
    with_retained_delegate_dispatch(
        a2a_process_custody_model(fence, retention_mode, callback_custody),
        |retained,
         current,
         signing,
         checkpoint_schema,
         checkpoint_authority,
         _provider_clock,
         resolve_store,
         authority,
         custody,
         anchor,
         parents,
         parent_request,
         restart_generation,
         _initial_candidate| {
            let operation = retained.operation();
            let recovery = retained.recovery_fixture();
            let selected_store = retained.custody().candidate().store().clone();
            let selector = selector_from_signing_authority(signing.signing_authority);
            let host_selector = if matches!(fence, Some(A2aProcessLaunchFence::Operation)) {
                unproved_selector(selector.principal_id.clone(), selected_store.clone())
            } else {
                selector.clone()
            };
            let provider = retained
                .provider_request()
                .unwrap()
                .provider_binding()
                .clone();
            let route_admission = a2a_exchange::route_for_handle_at_with_authority(
                &provider,
                operation.handle().1,
                "https://agent.example.test/a2a",
                true,
                10,
                100,
                110,
                signing.signing_authority,
            );
            let unlike_route = a2a_exchange::route_for_handle_at(
                &provider,
                operation.handle().1,
                "https://agent.example.test/a2a",
                false,
                10,
                100,
                110,
            );
            let route_ref = record_ref("process-a2a/route");
            let request_id = A2aJsonRpcRequestIdV1::parse("rpc-1").unwrap();
            let response_facts = match case {
                A2aProcessTerminalCase::Task => A2aExchangeFacts::Task {
                    task: A2aRemoteTaskIdV1::parse("task-1").unwrap(),
                    context: A2aRemoteContextIdV1::parse("context-1").unwrap(),
                    state: A2aRemoteTaskStateV1::Working,
                },
                A2aProcessTerminalCase::NoTask
                | A2aProcessTerminalCase::NotStarted
                | A2aProcessTerminalCase::OutcomeUnknown => A2aExchangeFacts::DirectMessage,
            };
            let exchange = A2aProcessExchangePort::new(match case {
                A2aProcessTerminalCase::Task | A2aProcessTerminalCase::NoTask => {
                    A2aProcessExchangeMode::Response(response_facts.clone())
                }
                A2aProcessTerminalCase::NotStarted => {
                    A2aProcessExchangeMode::Reject(A2aExchangeError::ServiceUnavailable)
                }
                A2aProcessTerminalCase::OutcomeUnknown => {
                    A2aProcessExchangeMode::PollError(A2aExchangeError::Timeout)
                }
            });
            let response_clock = A2aProcessClock::new([Ok(66), Ok(67), Ok(68)]);
            let retention_clock = A2aProcessClock::new([Ok(69), Ok(70)]);
            let send_checkpoint_signs = Cell::new(0_usize);
            let outcome_observation_signs = Cell::new(0_usize);
            let outcome_checkpoint_signs = Cell::new(0_usize);
            let application_store = MemoryAuthorityStore::default();
            let mut application = ready_application(&application_store, selected_store.clone(), 1);
            application.fixture_retain_generation(restart_generation);
            let deployment = unproved_deployment_for_selector(
                &host_selector,
                selected_store.clone(),
                application,
            );
            let mut host = ResidentProcessHost::new(
                unproved_writer(
                    host_selector.principal_id.clone(),
                    vec![selected_store.clone()],
                ),
                vec![deployment],
                NonZeroUsize::new(1).unwrap(),
                ResidentOperationHost::new(task_state_with_provider_capacity(
                    if matches!(fence, Some(A2aProcessLaunchFence::DuplicateRoute))
                        || matches!(
                            retention_mode,
                            A2aProcessRetentionMode::CrossedRoute
                                | A2aProcessRetentionMode::CrossedFinalization
                                | A2aProcessRetentionMode::CrossedRouteRelease
                                | A2aProcessRetentionMode::CrossedCustodyContinuation
                        )
                    {
                        2
                    } else {
                        1
                    },
                )),
            )
            .expect("one-deployment A2A process host");
            let sessions = UnusedA2aSessions;
            let unlike_store = store("unlike-a2a-store", "incarnation-1");
            let unlike_selector =
                unproved_selector(host_selector.principal_id.clone(), selected_store.clone());
            let unlike_principal = principal("unlike-a2a-principal");
            let unlike_checkpoint_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
                principal_id: &unlike_principal,
                ..*checkpoint_authority
            };
            let unlike_current = AmbientSessionAdmissionContext {
                now: current.now,
                subject: current.subject,
                grant: current.grant,
                current_principal_trust: current.current_principal_trust,
                capability_revocations: current.capability_revocations,
                deployment_binding_ref: current.deployment_binding_ref,
                deployment_binding: current.deployment_binding,
                deployment_revocations: current.deployment_revocations,
                runtime_authority: None,
                session_revocations: current.session_revocations,
                route_use: current.route_use,
                generation: pale_core::records::SessionAdmissionLeaseContextV1 {
                    active_head: current.generation.active_head,
                    compiled_generation: current.generation.compiled_generation,
                    effective_profile: current.generation.effective_profile,
                    activation_fence: current.generation.activation_fence,
                    signed: current.generation.signed,
                },
            };
            let launch_current = if matches!(fence, Some(A2aProcessLaunchFence::Session)) {
                &unlike_current
            } else {
                current
            };
            let launch_route = if matches!(fence, Some(A2aProcessLaunchFence::Route)) {
                &unlike_route
            } else {
                &route_admission
            };
            let launch_checkpoint_authority =
                if matches!(fence, Some(A2aProcessLaunchFence::Checkpoint)) {
                    &unlike_checkpoint_authority
                } else {
                    checkpoint_authority
                };
            let launch = A2aDelegateLaunchInput {
                send: A2aSendInput {
                    dispatch: A2aSendDispatch::fresh(retained),
                    current: launch_current,
                    route: (&route_ref, launch_route),
                    route_predecessor: pale_ports::OperationCustodyIndexPredecessor::Absent,
                    route_association: A2aRouteAssociation::Insert,
                    request_id: &request_id,
                    message_id: "message-1",
                    text: "Inspect the selected workspace.",
                    request_evidence_ref: record_ref("process-a2a/request"),
                    preparation_ref: record_ref("process-a2a/preparation"),
                    exchange_limits: a2a_exchange::limits(),
                    maximum_evidence_bytes: 8_192,
                    maximum_record_bytes: 65_536,
                    maximum_dispatch_record_bytes: 65_536,
                    resolve_store,
                },
                checkpoint: OperationCheckpointInput {
                    reference: record_ref("process-a2a/send-checkpoint"),
                    id: ControlCheckpointIdV1::parse("process-a2a-send-checkpoint").unwrap(),
                },
                checkpoint_schema,
                checkpoint_authority: launch_checkpoint_authority,
                response_ref: record_ref("process-a2a/response"),
                clock: &response_clock,
                accumulator_limits: AccumulatorPreparationLimits::new(
                    8, 20_000, 20_000, 8_192, 200_000,
                )
                .unwrap(),
                membership_limits: pale_ports::AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                    .unwrap(),
            };
            let launch_selector = if matches!(fence, Some(A2aProcessLaunchFence::Selector)) {
                &unlike_selector
            } else {
                &host_selector
            };
            let access_store = if matches!(fence, Some(A2aProcessLaunchFence::Store)) {
                &unlike_store
            } else {
                &selected_store
            };
            if matches!(fence, Some(A2aProcessLaunchFence::DuplicateRoute)) {
                let key = launch.send.dispatch.task_key().unwrap();
                let route = host
                    .a2a_delegate_routes
                    .reserve(host_selector.clone())
                    .expect("the first route fits the bounded table");
                assert!(host.a2a_delegate_routes.commit(route, key).is_ok());
            }
            let recovery_fixture_slot;
            let recovered_observation_slot;
            let launched = if matches!(retention_mode, A2aProcessRetentionMode::RecoveredLoss) {
                let failure = match host.launch_a2a_delegate(
                    launch_selector,
                    a2a_process_operation_access(access_store, custody, &sessions, anchor),
                    authority,
                    launch,
                    &exchange,
                    |_, _, _| Ok(()),
                    |_, _, _| Ok(()),
                    |_, _, _| {
                        send_checkpoint_signs.set(send_checkpoint_signs.get() + 1);
                        Ok(signature())
                    },
                ) {
                    Ok(_) => panic!("modeled request custody must remain uncertain"),
                    Err(failure) => failure,
                };
                assert_eq!(exchange.preparations.get(), 1);
                assert_eq!(exchange.starts.get(), 0);
                recovery_fixture_slot = match &*failure {
                    ResidentProcessA2aDelegateLaunchFailure::Custody { failure, .. } => {
                        let crate::resident::a2a_delegate_task_service::ResidentA2aDelegateLaunchFailure::Custody(
                            unresolved,
                        ) = &**failure
                        else {
                            panic!("the routed failure must retain unresolved send custody")
                        };
                        unresolved
                            .recovery_fixture()
                            .expect("ambiguous request custody must retain exact recovery evidence")
                    }
                    _ => panic!("the modeled commit uncertainty must stop in send custody"),
                };
                let recovery_fixture = &recovery_fixture_slot;
                recovered_observation_slot = recovery_fixture.recovered();
                let recovered_observation = &recovered_observation_slot;
                let recovery_limits = A2aSendRecoveryLimits::new(
                    65_536,
                    8_192,
                    OperationCheckpointAncestryLimits::new(16, 1 << 20, 16).unwrap(),
                    pale_ports::AccumulatorMembershipLimits::new(32, 100_000, 1_000_000).unwrap(),
                )
                .unwrap();
                let mut admit_checkpoint =
                    |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                        ControlStateCheckpointV1::admit_signed_wire(
                            wire,
                            checkpoint_schema,
                            checkpoint_authority,
                            |_, _, _, actual| {
                                assert_eq!(actual, &signature());
                                Ok(())
                            },
                        )
                    };
                let mut admit_route =
                    |reference: &ResidentRecordRefV1, wire: A2aRouteAdmissionWireV1| {
                        assert_eq!(reference, &route_ref);
                        A2aRouteAdmissionV1::admit_contextual_wire(
                            wire,
                            &A2aRouteAdmissionContextV1 {
                                signed: SignedResidentAdmissionContextV1 {
                                    expected_schema_sha256: route_admission.schema_sha256(),
                                    signing_authority: signing.signing_authority,
                                },
                                provider_binding: route_admission.provider_binding(),
                                route: route_admission.route(),
                            },
                            |_| Ok(()),
                            |_, _, _, actual| {
                                assert_eq!(actual, &signature());
                                Ok(())
                            },
                        )
                    };
                let rejected = host.recover_unresolved_a2a_delegate_with_rejected_loss_for_test(
                    a2a_process_operation_access(&selected_store, custody, &sessions, anchor),
                    authority,
                    *failure,
                    A2aSendRecoveryInput {
                        history: recovery_fixture.history(),
                        recovered: recovered_observation,
                        limits: recovery_limits,
                    },
                    A2aSendRecoveryAdmission {
                        admit_checkpoint: &mut admit_checkpoint,
                        admit_route: &mut admit_route,
                    },
                );
                let key = match &rejected {
                    ResidentProcessA2aDelegateRecoveryFailure::Loss { key, .. } => key.clone(),
                    _ => panic!("the fixture must stop at recovered loss settlement"),
                };
                let rejected = match host.retry_recovered_a2a_delegate_loss(
                    a2a_process_operation_access(&unlike_store, custody, &sessions, anchor),
                    authority,
                    rejected,
                ) {
                    Ok(_) => panic!("an unlike store cannot settle recovered loss"),
                    Err(rejected) => *rejected,
                };
                host.set_a2a_reservation_phase_for_test(
                    a2a_process_operation_access(&selected_store, custody, &sessions, anchor),
                    authority,
                    &key,
                    crate::resident::task_reservation::ResidentTaskReservationPhase::Offered,
                );
                let recovered = host
                    .retry_recovered_a2a_delegate_loss(
                        a2a_process_operation_access(&selected_store, custody, &sessions, anchor),
                        authority,
                        rejected,
                    )
                    .unwrap_or_else(|_| panic!("the exact recovered loss must settle"));
                assert_eq!(exchange.preparations.get(), 1);
                assert_eq!(exchange.starts.get(), 0);
                A2aProcessLaunch::Recovered(recovered)
            } else {
                let mut launch_access = A2aProcessLaunchAccess {
                    host: &mut host,
                    selector: &host_selector,
                    selected_store: &selected_store,
                    unlike_store: &unlike_store,
                    sessions: &sessions,
                    authority,
                    custody,
                    anchor,
                    exchange: &exchange,
                    response_clock: &response_clock,
                    checkpoint_signs: &send_checkpoint_signs,
                    fence,
                    mode: retention_mode,
                };
                let launched = launch_access.launch(launch_selector, access_store, launch)?;
                A2aProcessLaunch::Routed(launched)
            };
            let terminal = match launched {
                A2aProcessLaunch::Recovered(recovered) => A2aProcessTerminal::Recovered(recovered),
                A2aProcessLaunch::Routed(launched) => {
                    let mut process_access = A2aProcessAccess {
                        selected_store: &selected_store,
                        unlike_store: &unlike_store,
                        selector: &selector,
                        sessions: &sessions,
                        custody,
                        anchor,
                    };
                    match settle_routed_a2a_process_launch(
                        &mut host,
                        &mut process_access,
                        authority,
                        launched,
                        retention_mode,
                        response_facts,
                        &response_clock,
                    ) {
                        A2aProcessRoutedTerminal::Completed(completed) => {
                            A2aProcessTerminal::Carrier(*completed)
                        }
                        A2aProcessRoutedTerminal::Released => {
                            assert_eq!(exchange.preparations.get(), 1);
                            assert_eq!(exchange.starts.get(), 1);
                            assert!(host.a2a_delegate_routes.reserve(selector).is_ok());
                            return None;
                        }
                    }
                }
            };
            let terminal = if matches!(retention_mode, A2aProcessRetentionMode::DiagnosticRecovery)
            {
                let A2aProcessTerminal::Carrier(completed) = terminal else {
                    panic!("diagnostic recovery requires one carrier terminal")
                };
                let pending = host.a2a_diagnostic_from_unknown_for_test(completed);
                let pending = match host.recover_a2a_delegate_diagnostic(
                    ResidentDeploymentOperationAccess {
                        store: &unlike_store,
                        custody,
                        sessions: &sessions,
                        anchor,
                    },
                    authority,
                    pending,
                ) {
                    Ok(_) => panic!("an unlike store cannot recover the diagnostic"),
                    Err(pending) => *pending,
                };
                A2aProcessTerminal::Carrier(
                    host.recover_a2a_delegate_diagnostic(
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions: &sessions,
                            anchor,
                        },
                        authority,
                        pending,
                    )
                    .unwrap_or_else(|_| panic!("the exact diagnostic must recover")),
                )
            } else {
                terminal
            };
            let terminal = if matches!(retention_mode, A2aProcessRetentionMode::PublicInputRefusal)
            {
                let A2aProcessTerminal::Carrier(completed) = terminal else {
                    panic!("the public input test requires one carrier completion")
                };
                let calls = Rc::new(CompletedDelegateInputCalls::default());
                let mut adapter = crate::resident::process_host::service_assembly::ResidentServiceDelegateRecoveryAdapter::new(
                    RefusingCompletedDelegateInputs(Rc::clone(&calls)),
                    1,
                );
                let (handle_ref, handle) = operation.handle();
                let locator = pale_ports::ResidentOperationFrameLocator::new(
                    handle_ref.as_str().to_owned(),
                    handle
                        .canonical_operation_handle_sha256()
                        .as_str()
                        .to_owned(),
                );
                crate::resident::process_host::service_assembly::delegate_recovery_test_support::retain_completed(
                    &mut adapter,
                    selector.clone(),
                    locator,
                    completed,
                )
                    .expect("one completed branch fits bounded service custody");
                let turn = crate::resident::process_host::service_assembly::ResidentServiceRetainedDelegatePhasePort::continue_one_retained_delegate(
                    &mut adapter,
                    &mut host,
                    a2a_process_operation_access(
                        &selected_store,
                        custody,
                        &sessions,
                        anchor,
                    ),
                    authority,
                );
                assert_eq!(
                    turn,
                    crate::resident::process_host::program::ResidentProgramRetainedPhaseTurn::Deferred
                );
                let expected_calls = match case {
                    A2aProcessTerminalCase::Task => [1, 0, 0, 0],
                    A2aProcessTerminalCase::NoTask => [0, 1, 0, 0],
                    A2aProcessTerminalCase::NotStarted => [0, 0, 1, 0],
                    A2aProcessTerminalCase::OutcomeUnknown => [0, 0, 0, 1],
                };
                assert_eq!(calls.counts(), expected_calls);
                A2aProcessTerminal::Carrier(
                    crate::resident::process_host::service_assembly::delegate_recovery_test_support::take_retained_completed(&mut adapter)
                        .expect("a refused lender preserves the exact completed branch"),
                )
            } else {
                terminal
            };
            if !matches!(
                retention_mode,
                A2aProcessRetentionMode::CrossedRoute
                    | A2aProcessRetentionMode::CrossedCustodyContinuation
                    | A2aProcessRetentionMode::CrossedFinalization
                    | A2aProcessRetentionMode::CrossedRouteRelease
            ) {
                assert!(host.a2a_delegate_routes.reserve(selector.clone()).is_err());
            }

            let (route, completion) = match terminal {
                A2aProcessTerminal::Carrier(ResidentProcessA2aDelegateCompletion {
                    route,
                    completion,
                }) => (route, A2aProcessOutcome::Carrier(completion)),
                A2aProcessTerminal::Recovered(ResidentProcessRecoveredA2aDelegate {
                    route,
                    recovered,
                }) => (route, A2aProcessOutcome::Recovered(recovered)),
            };
            let mut crossed_route = None;
            let (route, correct_route) = if matches!(
                retention_mode,
                A2aProcessRetentionMode::CrossedRoute
                    | A2aProcessRetentionMode::CrossedFinalization
                    | A2aProcessRetentionMode::CrossedRouteRelease
                    | A2aProcessRetentionMode::CrossedCustodyContinuation
            ) {
                let crossed_key =
                    task_key(selected_store.clone(), "process-a2a/crossed-dispatch", 'b');
                let crossed = host
                    .a2a_delegate_routes
                    .reserve(selector.clone())
                    .expect("the second bounded route fits");
                assert!(host
                    .a2a_delegate_routes
                    .commit(crossed, crossed_key.clone())
                    .is_ok());
                let crossed = host
                    .a2a_delegate_routes
                    .check_out(&crossed_key)
                    .expect("the crossed route is held linearly");
                if matches!(retention_mode, A2aProcessRetentionMode::CrossedRoute) {
                    (crossed, Some(route))
                } else {
                    crossed_route = Some(crossed);
                    (route, None)
                }
            } else {
                (route, None)
            };
            let mut retention_access = A2aProcessRetentionAccess {
                host: &mut host,
                selected_store: &selected_store,
                unlike_store: &unlike_store,
                selector: &selector,
                sessions: &sessions,
                authority,
                custody,
                anchor,
                exchange: &exchange,
                outcome_observation_signs: &outcome_observation_signs,
                outcome_checkpoint_signs: &outcome_checkpoint_signs,
                mode: retention_mode,
                crossed_route,
                correct_route,
            };
            let recover_operation = |checkpoint| recovery.recovered(checkpoint);
            let retention_builder = A2aProcessRetentionBuilder {
                case,
                operation,
                recover_operation: &recover_operation,
                route_admission: &route_admission,
                route_ref: &route_ref,
                request_id: &request_id,
                retention_clock: &retention_clock,
                signing,
                checkpoint_schema,
                checkpoint_authority,
                resolve_store,
                outcome_observation_signs: &outcome_observation_signs,
                outcome_checkpoint_signs: &outcome_checkpoint_signs,
            };
            let retained = retention_builder.retain(&mut retention_access, route, completion);
            let Some(retained) = retained else {
                assert_eq!(exchange.preparations.get(), 1);
                assert_eq!(exchange.starts.get(), 1);
                return None;
            };
            let result = match (case, retained) {
                (A2aProcessTerminalCase::Task, crate::resident::RetainedA2aOutcome::Task(task)) => {
                    assert_eq!(task.binding().1.remote_task_id().as_str(), "task-1");
                    let checkpoint = task.checkpoint().1.clone();
                    let checkpoint_ref = task.checkpoint().0.clone();
                    let attempt = recovery.history();
                    let history = recover_retained_a2a_task_history(
                        &task,
                        (&route_ref, &route_admission),
                        &attempt,
                        signing.signing_authority,
                        authority,
                        custody,
                        anchor,
                    );
                    Some(run(RetainedProcessA2aTaskFixture {
                        host: &mut host,
                        selector: selector.clone(),
                        store: selected_store.clone(),
                        sessions: &sessions,
                        history,
                        provider_recovery: &recovery,
                        attempt,
                        signing: signing.signing_authority,
                        checkpoint,
                        checkpoint_ref,
                        authority,
                        custody,
                        anchor,
                        parents,
                        parent_request,
                        send_preparations: exchange.preparations.get(),
                        send_starts: exchange.starts.get(),
                    }))
                }
                (
                    A2aProcessTerminalCase::NoTask,
                    crate::resident::RetainedA2aOutcome::NoTask(value),
                ) => {
                    assert!(value
                        .custody()
                        .candidate()
                        .record(OperationCustodyRecordKind::A2aTaskBinding)
                        .is_none());
                    None
                }
                (
                    A2aProcessTerminalCase::NotStarted,
                    crate::resident::RetainedA2aOutcome::NotStarted(_),
                )
                | (
                    A2aProcessTerminalCase::OutcomeUnknown,
                    crate::resident::RetainedA2aOutcome::OutcomeUnknown(_),
                ) => None,
                _ => panic!("durable retention changed the carrier branch"),
            };
            assert_eq!(outcome_observation_signs.get(), 1);
            assert_eq!(outcome_checkpoint_signs.get(), 1);
            assert_eq!(exchange.preparations.get(), 1);
            assert_eq!(
                exchange.starts.get(),
                usize::from(!matches!(
                    retention_mode,
                    A2aProcessRetentionMode::RecoveredLoss
                ))
            );
            assert_eq!(send_checkpoint_signs.get(), 1);
            assert!(host.a2a_delegate_routes.reserve(selector).is_ok());
            result
        },
    )
}

fn run_a2a_send_through_process_route(
    case: A2aProcessTerminalCase,
    fence: Option<A2aProcessLaunchFence>,
    retention_mode: A2aProcessRetentionMode,
) {
    let _ = with_a2a_send_through_process_route(
        case,
        fence,
        retention_mode,
        RetainedTaskCallbackCustody::ReadOnly,
        |_| (),
    );
}

#[derive(Clone, Copy, Debug)]
pub(super) enum CommittedA2aSendRestartCase {
    Exact,
    Selector,
    Store,
    RouteSaturated,
    RouteConflict,
    Bounds,
    Request,
    Route,
    DeploymentAuthority,
    Checkpoint,
    Anchor,
}

struct UnavailableA2aRestartAnchor<'a>(&'a mut dyn AntiRollbackAnchorPort);

impl AntiRollbackAnchorPort for UnavailableA2aRestartAnchor<'_> {
    fn read(
        &mut self,
        _: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::Unavailable,
        ))
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.0.compare_and_advance(transition)
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.0.observe(transition)
    }
}

/// Run one real committed send across a process restart.
pub(super) fn run_committed_a2a_send_cold_restart(case: CommittedA2aSendRestartCase) {
    let reaches_recovery = matches!(case, CommittedA2aSendRestartCase::Exact);
    with_retained_delegate_dispatch(
        A2aTaskOwnerCustodyModel {
            clock_times: vec![63, 64, 65],
            expected_preparations: if reaches_recovery { 6 } else { 5 },
            expected_commits: if reaches_recovery { 6 } else { 5 },
            expected_read_backs: if reaches_recovery { 6 } else { 5 },
            expected_anchor_advances: if reaches_recovery { 6 } else { 5 },
            expected_anchor_observations: if reaches_recovery { 6 } else { 5 },
            unknown_exact_commits: &[],
            unavailable_read_backs: &[],
            unavailable_anchor_advances: &[],
            unknown_exact_anchor_advances: &[],
            unknown_failed_observation_anchor_advances: &[],
            later_anchor_observations: Vec::new(),
            fail_a2a_preflight_once: false,
        },
        |retained,
         current,
         signing,
         checkpoint_schema,
         checkpoint_authority,
         _provider_clock,
         resolve_store,
         authority,
         custody,
         anchor,
         _parents,
         parent_request,
         restart_generation,
         _initial_candidate| {
            let operation = retained.operation();
            let recovery = retained.recovery_fixture();
            let history = recovery.history();
            let selected_store = retained.custody().candidate().store().clone();
            let selector = selector_from_signing_authority(signing.signing_authority);
            let provider = retained
                .provider_request()
                .expect("Delegate dispatch keeps its provider request")
                .provider_binding()
                .clone();
            let foreign_selector =
                unproved_selector(principal("foreign-route-principal"), selected_store.clone());
            let foreign_route_signing = integrated_signing_authority(&foreign_selector);
            let route_signing = if matches!(case, CommittedA2aSendRestartCase::DeploymentAuthority)
            {
                &foreign_route_signing
            } else {
                signing.signing_authority
            };
            let route_admission = a2a_exchange::route_for_handle_at_with_authority(
                &provider,
                operation.handle().1,
                "https://agent.example.test/a2a",
                true,
                10,
                100,
                110,
                route_signing,
            );
            let route_ref = record_ref("process-a2a/restart-route");
            let request_id = A2aJsonRpcRequestIdV1::parse("restart-rpc-1").unwrap();
            let exchange = A2aProcessExchangePort::new(A2aProcessExchangeMode::PollError(
                A2aExchangeError::Timeout,
            ));
            let response_clock = A2aProcessClock::new([Ok(66), Ok(67), Ok(68)]);
            let retention_clock = A2aProcessClock::new([Ok(69), Ok(70)]);
            let send_checkpoint_signs = Cell::new(0_usize);
            let outcome_observation_signs = Cell::new(0_usize);
            let outcome_checkpoint_signs = Cell::new(0_usize);
            let application_store = MemoryAuthorityStore::default();
            let application = ready_application(&application_store, selected_store.clone(), 1);
            let deployment =
                unproved_deployment_for_selector(&selector, selected_store.clone(), application);
            let mut host = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                vec![deployment],
                NonZeroUsize::new(1).unwrap(),
                ResidentOperationHost::new(task_state_with_provider_capacity(1)),
            )
            .expect("one-deployment A2A process host");
            let sessions = UnusedA2aSessions;
            let launch = A2aDelegateLaunchInput {
                send: A2aSendInput {
                    dispatch: A2aSendDispatch::fresh(retained),
                    current,
                    route: (&route_ref, &route_admission),
                    route_predecessor: pale_ports::OperationCustodyIndexPredecessor::Absent,
                    route_association: A2aRouteAssociation::Insert,
                    request_id: &request_id,
                    message_id: "restart-message-1",
                    text: "Inspect the selected workspace.",
                    request_evidence_ref: record_ref("process-a2a/restart-request"),
                    preparation_ref: record_ref("process-a2a/restart-preparation"),
                    exchange_limits: a2a_exchange::limits(),
                    maximum_evidence_bytes: 8_192,
                    maximum_record_bytes: 65_536,
                    maximum_dispatch_record_bytes: 65_536,
                    resolve_store,
                },
                checkpoint: OperationCheckpointInput {
                    reference: record_ref("process-a2a/restart-send-checkpoint"),
                    id: ControlCheckpointIdV1::parse("process-a2a-restart-send-checkpoint")
                        .unwrap(),
                },
                checkpoint_schema,
                checkpoint_authority,
                response_ref: record_ref("process-a2a/restart-response"),
                clock: &response_clock,
                accumulator_limits: AccumulatorPreparationLimits::new(
                    8, 20_000, 20_000, 8_192, 200_000,
                )
                .unwrap(),
                membership_limits: pale_ports::AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                    .unwrap(),
            };
            let (key, send_checkpoint) = if matches!(
                case,
                CommittedA2aSendRestartCase::DeploymentAuthority
            ) {
                let mut tasks = task_state_with_provider_capacity(1);
                let mut service =
                    ResidentOperationService::new(authority, custody, anchor, &mut tasks);
                let key = match service
                    .launch_a2a_delegate(
                        launch,
                        &exchange,
                        |_, _, _| Ok(()),
                        |_, _, _| Ok(()),
                        |_, _, _| {
                            send_checkpoint_signs.set(send_checkpoint_signs.get() + 1);
                            Ok(signature())
                        },
                    )
                    .unwrap_or_else(|_| panic!("the foreign-route send starts exactly once"))
                {
                    crate::resident::a2a_delegate_task_service::ResidentA2aDelegateLaunch::Awaiting(
                        key,
                    ) => key,
                    crate::resident::a2a_delegate_task_service::ResidentA2aDelegateLaunch::Completed(
                        _,
                    ) => panic!("the recovery fixture must stop before a response"),
                    crate::resident::a2a_delegate_task_service::ResidentA2aDelegateLaunch::Diagnostic(
                        _,
                    ) => panic!("the recovery fixture must not synthesize a terminal diagnostic"),
                };
                let checkpoint = service
                    .active_a2a_send_checkpoint_for_test(&key)
                    .expect("the foreign-route fixture needs one active send");
                (key, checkpoint)
            } else {
                let key = match host
                    .launch_a2a_delegate(
                        &selector,
                        a2a_process_operation_access(&selected_store, custody, &sessions, anchor),
                        authority,
                        launch,
                        &exchange,
                        |_, _, _| Ok(()),
                        |_, _, _| Ok(()),
                        |_, _, _| {
                            send_checkpoint_signs.set(send_checkpoint_signs.get() + 1);
                            Ok(signature())
                        },
                    )
                    .unwrap_or_else(|_| panic!("the selected send starts exactly once"))
                {
                    ResidentProcessA2aDelegateLaunch::Awaiting(key) => key,
                    ResidentProcessA2aDelegateLaunch::Completed(_) => {
                        panic!("the recovery fixture must stop before a response")
                    }
                    ResidentProcessA2aDelegateLaunch::Diagnostic(_) => {
                        panic!("the recovery fixture must not synthesize a terminal diagnostic")
                    }
                };
                let checkpoint = host.a2a_send_checkpoint_for_test(
                    a2a_process_operation_access(&selected_store, custody, &sessions, anchor),
                    authority,
                    &key,
                );
                (key, checkpoint)
            };
            assert_eq!(exchange.preparations.get(), 1);
            assert_eq!(exchange.starts.get(), 1);
            assert_eq!(send_checkpoint_signs.get(), 1);
            drop(host);

            let recovered_observation = recovery.recovered(send_checkpoint.clone());
            let send_recovery_observation =
                recovery.recovered(if matches!(case, CommittedA2aSendRestartCase::Checkpoint) {
                    (
                        record_ref("process-a2a/substituted-restart-checkpoint"),
                        send_checkpoint.1,
                    )
                } else {
                    send_checkpoint
                });
            let recovered_operation = crate::resident::RecoveredLocalOperation::fixture(
                operation,
                history.request.clone(),
                selected_store.clone(),
            );
            let mut attempt = ResidentProcessRecoveredAttempt {
                start: ResidentProcessRecoveredOperationStart {
                    selector: selector.clone(),
                    control: crate::resident::RecoveredOperationControl::fixture(
                        recovered_operation,
                        recovered_observation,
                    ),
                    generation_digest: operation.handle().1.compiled_generation_sha256().clone(),
                    lineage: ResidentProcessOperationLineage::pairwise_delegate_fixture(
                        operation.handle().0,
                        operation.handle().1,
                    ),
                },
                dispatch: ResidentProcessAttemptObservation::CurrentHead,
                selection: history.selection.clone(),
            };
            if matches!(case, CommittedA2aSendRestartCase::Selector) {
                attempt.start.selector = unproved_selector(
                    principal("unlike-restart-principal"),
                    selected_store.clone(),
                );
            }

            let restarted_store = MemoryAuthorityStore::default();
            let mut restarted_application =
                ready_application(&restarted_store, selected_store.clone(), 2);
            restarted_application.fixture_retain_generation(restart_generation);
            let restarted_deployment = unproved_deployment_for_selector(
                &selector,
                selected_store.clone(),
                restarted_application,
            );
            let mut restarted = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                vec![restarted_deployment],
                NonZeroUsize::new(1).unwrap(),
                ResidentOperationHost::new(task_state_with_provider_capacity(
                    if matches!(case, CommittedA2aSendRestartCase::RouteConflict) {
                        2
                    } else {
                        1
                    },
                )),
            )
            .expect("fresh process host for A2A recovery");
            if matches!(
                case,
                CommittedA2aSendRestartCase::RouteSaturated
                    | CommittedA2aSendRestartCase::RouteConflict
            ) {
                let occupied_key = if matches!(case, CommittedA2aSendRestartCase::RouteConflict) {
                    key.clone()
                } else {
                    task_key(selected_store.clone(), "process-a2a/restart-occupied", 'f')
                };
                let occupied = restarted
                    .a2a_delegate_routes
                    .reserve(selector.clone())
                    .expect("the hostile fixture occupies one route");
                assert!(restarted
                    .a2a_delegate_routes
                    .commit(occupied, occupied_key)
                    .is_ok());
            }
            let recovery_limits = A2aSendRecoveryLimits::new(
                if matches!(case, CommittedA2aSendRestartCase::Bounds) {
                    1
                } else {
                    65_536
                },
                if matches!(case, CommittedA2aSendRestartCase::Bounds) {
                    1
                } else {
                    8_192
                },
                OperationCheckpointAncestryLimits::new(16, 1 << 20, 16).unwrap(),
                pale_ports::AccumulatorMembershipLimits::new(32, 100_000, 1_000_000).unwrap(),
            )
            .unwrap();
            let unlike_route = a2a_exchange::route_for_handle_at(
                &provider,
                operation.handle().1,
                "https://unlike-agent.example.test/a2a",
                false,
                10,
                100,
                110,
            );
            let mut unlike_route = Some(unlike_route);
            let mut admit_checkpoint =
                |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                    ControlStateCheckpointV1::admit_signed_wire(
                        wire,
                        checkpoint_schema,
                        checkpoint_authority,
                        |_, _, _, actual| {
                            assert_eq!(actual, &signature());
                            Ok(())
                        },
                    )
                };
            let mut admit_route =
                |reference: &ResidentRecordRefV1, wire: A2aRouteAdmissionWireV1| {
                    assert_eq!(reference, &route_ref);
                    if matches!(case, CommittedA2aSendRestartCase::Route) {
                        return Ok(unlike_route
                            .take()
                            .expect("route admission occurs once during recovery"));
                    }
                    A2aRouteAdmissionV1::admit_contextual_wire(
                        wire,
                        &A2aRouteAdmissionContextV1 {
                            signed: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: route_admission.schema_sha256(),
                                signing_authority: route_signing,
                            },
                            provider_binding: route_admission.provider_binding(),
                            route: route_admission.route(),
                        },
                        |_| Ok(()),
                        |_, _, _, actual| {
                            assert_eq!(actual, &signature());
                            Ok(())
                        },
                    )
                };
            let recovery_history = if matches!(case, CommittedA2aSendRestartCase::Request) {
                OperationAttemptHistoryInput {
                    operation: history.operation,
                    request: &parent_request.1,
                    session: history.session,
                    lease: history.lease,
                    generation: history.generation,
                    dispatched: history.dispatched,
                    selection: history.selection,
                    maximum_selection_bytes: history.maximum_selection_bytes,
                }
            } else {
                history
            };
            let unlike_store = store("unlike-a2a-store", "incarnation-1");
            let access_store = if matches!(case, CommittedA2aSendRestartCase::Store) {
                &unlike_store
            } else {
                &selected_store
            };
            let mut unavailable_anchor = UnavailableA2aRestartAnchor(anchor);
            let recovery_anchor: &mut dyn AntiRollbackAnchorPort =
                if matches!(case, CommittedA2aSendRestartCase::Anchor) {
                    &mut unavailable_anchor
                } else {
                    unavailable_anchor.0
                };
            let recovered = restarted.recover_restarted_a2a_delegate(
                attempt,
                a2a_process_operation_access(access_store, custody, &sessions, recovery_anchor),
                authority,
                A2aSendRecoveryInput {
                    history: recovery_history,
                    recovered: &send_recovery_observation,
                    limits: recovery_limits,
                },
                A2aSendRecoveryAdmission {
                    admit_checkpoint: &mut admit_checkpoint,
                    admit_route: &mut admit_route,
                },
            );
            assert_eq!(exchange.preparations.get(), 1);
            assert_eq!(exchange.starts.get(), 1);
            if !matches!(case, CommittedA2aSendRestartCase::Exact) {
                let failure = match recovered {
                    Ok(_) => panic!("the {case:?} restart must be rejected"),
                    Err(failure) => failure,
                };
                match (case, *failure) {
                    (
                        CommittedA2aSendRestartCase::Selector | CommittedA2aSendRestartCase::Store,
                        ResidentProcessA2aDelegateRestartFailure::Selection(_),
                    )
                    | (
                        CommittedA2aSendRestartCase::RouteSaturated,
                        ResidentProcessA2aDelegateRestartFailure::RouteSaturated,
                    )
                    | (
                        CommittedA2aSendRestartCase::RouteConflict,
                        ResidentProcessA2aDelegateRestartFailure::RouteConflict,
                    )
                    | (
                        CommittedA2aSendRestartCase::Bounds,
                        ResidentProcessA2aDelegateRestartFailure::Rejected(_),
                    )
                    | (
                        CommittedA2aSendRestartCase::Request
                        | CommittedA2aSendRestartCase::Route
                        | CommittedA2aSendRestartCase::DeploymentAuthority
                        | CommittedA2aSendRestartCase::Anchor,
                        ResidentProcessA2aDelegateRestartFailure::Rejected(_),
                    )
                    | (
                        CommittedA2aSendRestartCase::Checkpoint,
                        ResidentProcessA2aDelegateRestartFailure::Selection(_)
                        | ResidentProcessA2aDelegateRestartFailure::Rejected(_),
                    ) => {}
                    (case, failure) => {
                        let boundary = match failure {
                            ResidentProcessA2aDelegateRestartFailure::Selection(_) => "selection",
                            ResidentProcessA2aDelegateRestartFailure::RouteSaturated => {
                                "route-saturated"
                            }
                            ResidentProcessA2aDelegateRestartFailure::RouteConflict => {
                                "route-conflict"
                            }
                            ResidentProcessA2aDelegateRestartFailure::Rejected(_) => "rejected",
                            ResidentProcessA2aDelegateRestartFailure::Rollback { .. } => "rollback",
                            ResidentProcessA2aDelegateRestartFailure::Loss { .. } => "loss",
                        };
                        panic!("{case:?} failed at the {boundary} boundary")
                    }
                }
                assert_eq!(restarted.operations.outstanding_task_reservations(), 0);
                if matches!(case, CommittedA2aSendRestartCase::DeploymentAuthority) {
                    assert!(restarted.a2a_delegate_routes.reserve(selector).is_ok());
                }
                return;
            }
            let recovered =
                recovered.unwrap_or_else(|_| panic!("the exact committed send must recover"));
            assert_eq!(restarted.operations.outstanding_task_reservations(), 1);
            assert!(restarted
                .a2a_delegate_routes
                .reserve(selector.clone())
                .is_err());

            let ResidentProcessRecoveredA2aDelegate { route, recovered } = recovered;
            let recover_operation = |checkpoint| recovery.recovered(checkpoint);
            let retention_builder = A2aProcessRetentionBuilder {
                case: A2aProcessTerminalCase::OutcomeUnknown,
                operation,
                recover_operation: &recover_operation,
                route_admission: &route_admission,
                route_ref: &route_ref,
                request_id: &request_id,
                retention_clock: &retention_clock,
                signing,
                checkpoint_schema,
                checkpoint_authority,
                resolve_store,
                outcome_observation_signs: &outcome_observation_signs,
                outcome_checkpoint_signs: &outcome_checkpoint_signs,
            };
            let mut retention_access = A2aProcessRetentionAccess {
                host: &mut restarted,
                selected_store: &selected_store,
                unlike_store: &unlike_store,
                selector: &selector,
                sessions: &sessions,
                authority,
                custody,
                anchor,
                exchange: &exchange,
                outcome_observation_signs: &outcome_observation_signs,
                outcome_checkpoint_signs: &outcome_checkpoint_signs,
                mode: A2aProcessRetentionMode::Normal,
                crossed_route: None,
                correct_route: None,
            };
            let retained = retention_builder
                .retain(
                    &mut retention_access,
                    route,
                    A2aProcessOutcome::Recovered(recovered),
                )
                .expect("recovered uncertainty must be retained");
            let crate::resident::RetainedA2aOutcome::OutcomeUnknown(retained) = retained else {
                panic!("restart recovery cannot fabricate a remote Task")
            };
            assert!(retained
                .custody()
                .candidate()
                .record(OperationCustodyRecordKind::A2aTaskBinding)
                .is_none());
            let OperationObservationStateV1::OutcomeUnknown { reconciliation, .. } =
                retained.observation().1.state()
            else {
                panic!("restart recovery must retain explicit outcome uncertainty")
            };
            assert_eq!(
                reconciliation.record_ref.as_str(),
                "process-a2a/recovered-reconciliation"
            );
            assert_eq!(outcome_observation_signs.get(), 1);
            assert_eq!(outcome_checkpoint_signs.get(), 1);
            assert_eq!(restarted.operations.outstanding_task_reservations(), 0);
            assert!(restarted.a2a_delegate_routes.reserve(selector).is_ok());
            assert_eq!(exchange.preparations.get(), 1);
            assert_eq!(exchange.starts.get(), 1);
        },
    );
}

/// Lend one real retained Task to a callback that completes one custody append.
pub(super) fn with_retained_process_a2a_task_and_one_completed_append<R>(
    run: impl for<'a, 'store> FnOnce(RetainedProcessA2aTaskFixture<'a, 'store>) -> R,
) -> R {
    with_a2a_send_through_process_route(
        A2aProcessTerminalCase::Task,
        None,
        A2aProcessRetentionMode::Normal,
        RetainedTaskCallbackCustody::OneCompletedAppend,
        run,
    )
    .expect("the real A2A send path must retain one Task")
}

/// Lend one real retained Task to a callback that completes two custody appends.
pub(super) fn with_retained_process_a2a_task_and_two_completed_appends<R>(
    run: impl for<'a, 'store> FnOnce(RetainedProcessA2aTaskFixture<'a, 'store>) -> R,
) -> R {
    with_a2a_send_through_process_route(
        A2aProcessTerminalCase::Task,
        None,
        A2aProcessRetentionMode::Normal,
        RetainedTaskCallbackCustody::TwoCompletedAppends,
        run,
    )
    .expect("the real A2A send path must retain one Task")
}

/// Lend one real retained Task to a read-only callback.
pub(in crate::resident) fn with_retained_process_a2a_task<R>(
    run: impl for<'a, 'store> FnOnce(RetainedProcessA2aTaskFixture<'a, 'store>) -> R,
) -> R {
    with_a2a_send_through_process_route(
        A2aProcessTerminalCase::Task,
        None,
        A2aProcessRetentionMode::Normal,
        RetainedTaskCallbackCustody::ReadOnly,
        run,
    )
    .expect("the real A2A send path must retain one Task")
}

#[test]
fn process_host_retains_task_before_releasing_its_route() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::Task,
        None,
        A2aProcessRetentionMode::Normal,
    );
}

#[test]
fn process_host_retains_no_task_without_fabricating_a_binding() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NoTask,
        None,
        A2aProcessRetentionMode::Normal,
    );
}

#[test]
fn process_host_retains_not_started_before_releasing_its_route() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NotStarted,
        None,
        A2aProcessRetentionMode::Normal,
    );
}

#[test]
fn process_host_retains_ambiguous_send_without_replay() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::OutcomeUnknown,
        None,
        A2aProcessRetentionMode::Normal,
    );
}

#[test]
fn public_delegate_task_input_refusal_preserves_binding_custody_for_retry() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::Task,
        None,
        A2aProcessRetentionMode::PublicInputRefusal,
    );
}

#[test]
fn public_delegate_no_task_input_refusal_preserves_non_binding_custody_for_retry() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NoTask,
        None,
        A2aProcessRetentionMode::PublicInputRefusal,
    );
}

#[test]
fn public_delegate_not_started_input_refusal_preserves_custody_for_retry() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NotStarted,
        None,
        A2aProcessRetentionMode::PublicInputRefusal,
    );
}

#[test]
fn public_delegate_unknown_outcome_input_refusal_preserves_custody_without_replay() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::OutcomeUnknown,
        None,
        A2aProcessRetentionMode::PublicInputRefusal,
    );
}

#[test]
fn process_host_holds_route_through_finalization_retry() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NoTask,
        None,
        A2aProcessRetentionMode::RejectFinalization,
    );
}

#[test]
fn process_host_retries_exact_route_release_without_replaying_send() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NoTask,
        None,
        A2aProcessRetentionMode::RejectRouteRelease,
    );
}

#[test]
fn process_host_rejects_crossed_terminal_routes_before_durable_io() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::Task,
        None,
        A2aProcessRetentionMode::CrossedRoute,
    );
}

#[test]
fn process_host_rejects_crossed_finalization_before_releasing_capacity() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NoTask,
        None,
        A2aProcessRetentionMode::CrossedFinalization,
    );
}

#[test]
fn process_host_rejects_crossed_route_release_without_releasing_either_route() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NoTask,
        None,
        A2aProcessRetentionMode::CrossedRouteRelease,
    );
}

#[test]
fn process_host_holds_route_through_durable_custody_uncertainty() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NoTask,
        None,
        A2aProcessRetentionMode::AnchorCustodyContinuation,
    );
}

#[test]
fn process_host_continues_exact_database_custody_without_replaying_send() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NoTask,
        None,
        A2aProcessRetentionMode::DatabaseCustodyContinuation,
    );
}

#[test]
fn process_host_rejects_crossed_physical_custody_before_database_or_anchor_io() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NoTask,
        None,
        A2aProcessRetentionMode::CrossedCustodyContinuation,
    );
}

#[test]
fn process_host_continues_exact_send_custody_without_replaying_the_carrier() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NoTask,
        None,
        A2aProcessRetentionMode::LaunchCustodyContinuation,
    );
}

#[test]
fn process_host_retries_exact_launch_rollback_without_replaying_the_carrier() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NotStarted,
        None,
        A2aProcessRetentionMode::LaunchRollback,
    );
}

#[test]
fn retained_post_send_failure_continues_without_replaying_the_carrier() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::OutcomeUnknown,
        None,
        A2aProcessRetentionMode::StartedAssociation,
    );
}

#[test]
fn process_host_retries_exact_offered_completion_without_replaying_the_carrier() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::OutcomeUnknown,
        None,
        A2aProcessRetentionMode::OfferedCompletion,
    );
}

#[test]
fn process_host_retries_exact_poll_settlement_without_replaying_the_carrier() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::NoTask,
        None,
        A2aProcessRetentionMode::PollSettlement,
    );
}

#[test]
fn process_host_recovers_exact_diagnostic_without_replaying_the_carrier() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::OutcomeUnknown,
        None,
        A2aProcessRetentionMode::DiagnosticRecovery,
    );
}

#[test]
fn process_host_recovers_unresolved_send_and_retries_exact_loss_without_carrier_replay() {
    run_a2a_send_through_process_route(
        A2aProcessTerminalCase::OutcomeUnknown,
        None,
        A2aProcessRetentionMode::RecoveredLoss,
    );
}

#[test]
fn process_host_rejects_unlike_launch_coordinates_before_carrier_io() {
    for fence in [
        A2aProcessLaunchFence::Selector,
        A2aProcessLaunchFence::Store,
        A2aProcessLaunchFence::Operation,
        A2aProcessLaunchFence::Session,
        A2aProcessLaunchFence::Checkpoint,
        A2aProcessLaunchFence::Route,
        A2aProcessLaunchFence::DuplicateRoute,
    ] {
        run_a2a_send_through_process_route(
            A2aProcessTerminalCase::NoTask,
            Some(fence),
            A2aProcessRetentionMode::Normal,
        );
    }
}

#[derive(Clone, Copy)]
enum NoTaskProcessStop {
    PhysicalCustody,
    RouteRelease,
}

struct StoppedNoTaskProcess<'host, 'store, 'input> {
    host: &'host mut ResidentProcessHost<'store, UnprovedHeldWriter>,
    store: &'host AuthorityStoreCoordinate,
    selector: &'host ResidentDeploymentAuthoritySelector,
    sessions: &'host UnusedA2aSessions,
    authority: &'host mut ProvingAuthority,
    custody: &'host mut dyn pale_ports::OperationCustodyPort,
    anchor: &'host mut dyn pale_ports::AntiRollbackAnchorPort,
    exchange: &'host A2aProcessExchangePort,
    observation_signs: &'host Cell<usize>,
    checkpoint_signs: &'host Cell<usize>,
    failure: ResidentProcessA2aDelegateRetentionFailure<'input>,
}

fn with_stopped_no_task_process<R>(
    label: &str,
    stop: NoTaskProcessStop,
    run: impl FnOnce(StoppedNoTaskProcess<'_, '_, '_>) -> R,
) -> R {
    let mode = match stop {
        NoTaskProcessStop::PhysicalCustody => A2aProcessRetentionMode::AnchorCustodyContinuation,
        NoTaskProcessStop::RouteRelease => A2aProcessRetentionMode::Normal,
    };
    with_retained_delegate_dispatch_named(
        a2a_process_custody_model(None, mode, RetainedTaskCallbackCustody::ReadOnly),
        label,
        |retained,
         current,
         signing,
         checkpoint_schema,
         checkpoint_authority,
         _provider_clock,
         resolve_store,
         authority,
         custody,
         anchor,
         _parents,
         _parent_request,
         _restart_generation,
         _initial_candidate| {
            let operation = retained.operation();
            let selected_store = retained.custody().candidate().store().clone();
            let selector = selector_from_signing_authority(signing.signing_authority);
            let provider = retained
                .provider_request()
                .expect("Delegate dispatch keeps its provider request")
                .provider_binding()
                .clone();
            let route_admission = a2a_exchange::route_for_handle_at_with_authority(
                &provider,
                operation.handle().1,
                "https://agent.example.test/a2a",
                false,
                10,
                100,
                110,
                signing.signing_authority,
            );
            let route_ref = record_ref(&format!("{label}/route"));
            let request_id = A2aJsonRpcRequestIdV1::parse(format!("{label}-rpc")).unwrap();
            let message_id = format!("{label}-message");
            let exchange = A2aProcessExchangePort::new(A2aProcessExchangeMode::Response(
                A2aExchangeFacts::DirectMessage,
            ));
            let response_clock = A2aProcessClock::new([Ok(66), Ok(67), Ok(68)]);
            let retention_clock = A2aProcessClock::new([Ok(69), Ok(70)]);
            let observation_signs = Cell::new(0_usize);
            let checkpoint_signs = Cell::new(0_usize);
            let application_store = MemoryAuthorityStore::default();
            let deployment = unproved_deployment_for_selector(
                &selector,
                selected_store.clone(),
                ready_application(&application_store, selected_store.clone(), 1),
            );
            let mut host = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                vec![deployment],
                NonZeroUsize::new(1).unwrap(),
                ResidentOperationHost::new(task_state_with_provider_capacity(1)),
            )
            .expect("one-deployment A2A process host");
            let sessions = UnusedA2aSessions;
            let launch = A2aDelegateLaunchInput {
                send: A2aSendInput {
                    dispatch: A2aSendDispatch::fresh(retained),
                    current,
                    route: (&route_ref, &route_admission),
                    route_predecessor: pale_ports::OperationCustodyIndexPredecessor::Absent,
                    route_association: A2aRouteAssociation::Insert,
                    request_id: &request_id,
                    message_id: &message_id,
                    text: "Inspect the selected workspace.",
                    request_evidence_ref: record_ref(&format!("{label}/request")),
                    preparation_ref: record_ref(&format!("{label}/preparation")),
                    exchange_limits: a2a_exchange::limits(),
                    maximum_evidence_bytes: 8_192,
                    maximum_record_bytes: 65_536,
                    maximum_dispatch_record_bytes: 65_536,
                    resolve_store,
                },
                checkpoint: OperationCheckpointInput {
                    reference: record_ref(&format!("{label}/send-checkpoint")),
                    id: ControlCheckpointIdV1::parse(format!("{label}-send-checkpoint")).unwrap(),
                },
                checkpoint_schema,
                checkpoint_authority,
                response_ref: record_ref(&format!("{label}/response")),
                clock: &response_clock,
                accumulator_limits: AccumulatorPreparationLimits::new(
                    8, 20_000, 20_000, 8_192, 200_000,
                )
                .unwrap(),
                membership_limits: pale_ports::AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                    .unwrap(),
            };
            let launched = host
                .launch_a2a_delegate(
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody,
                        sessions: &sessions,
                        anchor,
                    },
                    authority,
                    launch,
                    &exchange,
                    |_, _, _| Ok(()),
                    |_, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("the NoTask fixture must launch"));
            let mut process_access = A2aProcessAccess {
                selected_store: &selected_store,
                unlike_store: &selected_store,
                selector: &selector,
                sessions: &sessions,
                custody,
                anchor,
            };
            let completed = match settle_routed_a2a_process_launch(
                &mut host,
                &mut process_access,
                authority,
                launched,
                A2aProcessRetentionMode::Normal,
                A2aExchangeFacts::DirectMessage,
                &response_clock,
            ) {
                A2aProcessRoutedTerminal::Completed(completed) => *completed,
                A2aProcessRoutedTerminal::Released => {
                    panic!("the NoTask fixture must keep terminal route custody")
                }
            };
            let ResidentProcessA2aDelegateCompletion { route, completion } = completed;
            let crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion::NoTask {
                response,
                finalization,
            } = completion
            else {
                panic!("the fixture carrier must return a direct Message")
            };
            let pending = ResidentProcessA2aDelegateRetention::NoTask {
                route,
                finalization,
                input: A2aNoTaskOutcomeRetention::new(
                    response,
                    A2aFailedSendObservationInput {
                        diagnostic_ref: record_ref(&format!("{label}/diagnostic")),
                        observation_ref: record_ref(&format!("{label}/observation")),
                        clock: &retention_clock,
                        signing,
                        maximum_diagnostic_bytes: 65_536,
                    },
                    A2aSendObservationCommitInput {
                        checkpoint: OperationCheckpointInput {
                            reference: record_ref(&format!("{label}/outcome-checkpoint")),
                            id: ControlCheckpointIdV1::parse(format!("{label}-outcome-checkpoint"))
                                .unwrap(),
                        },
                        clock: &retention_clock,
                        checkpoint_schema,
                        checkpoint_authority,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            8, 20_000, 20_000, 8_192, 200_000,
                        )
                        .unwrap(),
                    },
                    resolve_store,
                    |_, _, _, _| Ok(()),
                    |_, _, _| {
                        observation_signs.set(observation_signs.get() + 1);
                        Ok(signature())
                    },
                    |_, _, _| {
                        checkpoint_signs.set(checkpoint_signs.get() + 1);
                        Ok(signature())
                    },
                ),
            };
            let failure = match stop {
                NoTaskProcessStop::PhysicalCustody => {
                    match host.retain_a2a_delegate_outcome(
                        a2a_process_operation_access(&selected_store, custody, &sessions, anchor),
                        authority,
                        pending,
                    ) {
                        Ok(_) => panic!("the modeled anchor must stop physical custody"),
                        Err(failure) => failure,
                    }
                }
                NoTaskProcessStop::RouteRelease => Box::new(
                    host.retain_a2a_delegate_with_rejected_route_release_for_test(
                        a2a_process_operation_access(&selected_store, custody, &sessions, anchor),
                        authority,
                        pending,
                    ),
                ),
            };
            run(StoppedNoTaskProcess {
                host: &mut host,
                store: &selected_store,
                selector: &selector,
                sessions: &sessions,
                authority,
                custody,
                anchor,
                exchange: &exchange,
                observation_signs: &observation_signs,
                checkpoint_signs: &checkpoint_signs,
                failure: *failure,
            })
        },
    )
}

#[test]
fn process_host_rejects_crossed_same_kind_physical_failures_before_io() {
    with_stopped_no_task_process(
        "crossed-custody-first",
        NoTaskProcessStop::PhysicalCustody,
        |first| {
            with_stopped_no_task_process(
                "crossed-custody-second",
                NoTaskProcessStop::PhysicalCustody,
                |second| {
                    let StoppedNoTaskProcess {
                        host: first_host,
                        store: first_store,
                        selector: first_selector,
                        sessions: first_sessions,
                        authority: first_authority,
                        custody: first_custody,
                        anchor: first_anchor,
                        exchange: first_exchange,
                        observation_signs: first_observation_signs,
                        checkpoint_signs: first_checkpoint_signs,
                        failure: first_failure,
                    } = first;
                    let StoppedNoTaskProcess {
                        host: second_host,
                        store: second_store,
                        selector: second_selector,
                        sessions: second_sessions,
                        authority: second_authority,
                        custody: second_custody,
                        anchor: second_anchor,
                        exchange: second_exchange,
                        observation_signs: second_observation_signs,
                        checkpoint_signs: second_checkpoint_signs,
                        failure: second_failure,
                    } = second;
                    let (first_crossed, second_crossed) =
                        cross_a2a_retention_failures_for_test(first_failure, second_failure);
                    let first_counts =
                        (first_observation_signs.get(), first_checkpoint_signs.get());
                    let second_counts = (
                        second_observation_signs.get(),
                        second_checkpoint_signs.get(),
                    );
                    let first_crossed = match first_host.continue_a2a_delegate_outcome_custody(
                        a2a_process_operation_access(
                            first_store,
                            first_custody,
                            first_sessions,
                            first_anchor,
                        ),
                        first_authority,
                        first_crossed,
                    ) {
                        Ok(_) => panic!("the first host must reject the second lower failure"),
                        Err(failure) => failure,
                    };
                    let second_crossed = match second_host.continue_a2a_delegate_outcome_custody(
                        a2a_process_operation_access(
                            second_store,
                            second_custody,
                            second_sessions,
                            second_anchor,
                        ),
                        second_authority,
                        second_crossed,
                    ) {
                        Ok(_) => panic!("the second host must reject the first lower failure"),
                        Err(failure) => failure,
                    };
                    assert_eq!(
                        (first_observation_signs.get(), first_checkpoint_signs.get()),
                        first_counts
                    );
                    assert_eq!(
                        (
                            second_observation_signs.get(),
                            second_checkpoint_signs.get()
                        ),
                        second_counts
                    );
                    assert_eq!(first_exchange.preparations.get(), 1);
                    assert_eq!(first_exchange.starts.get(), 1);
                    assert_eq!(second_exchange.preparations.get(), 1);
                    assert_eq!(second_exchange.starts.get(), 1);
                    let (first_exact, second_exact) =
                        cross_a2a_retention_failures_for_test(*first_crossed, *second_crossed);
                    let _ = first_host
                        .continue_a2a_delegate_outcome_custody(
                            a2a_process_operation_access(
                                first_store,
                                first_custody,
                                first_sessions,
                                first_anchor,
                            ),
                            first_authority,
                            first_exact,
                        )
                        .unwrap_or_else(|_| panic!("the first exact failure must continue"));
                    let _ = second_host
                        .continue_a2a_delegate_outcome_custody(
                            a2a_process_operation_access(
                                second_store,
                                second_custody,
                                second_sessions,
                                second_anchor,
                            ),
                            second_authority,
                            second_exact,
                        )
                        .unwrap_or_else(|_| panic!("the second exact failure must continue"));
                    assert!(first_host
                        .a2a_delegate_routes
                        .reserve(first_selector.clone())
                        .is_ok());
                    assert!(second_host
                        .a2a_delegate_routes
                        .reserve(second_selector.clone())
                        .is_ok());
                },
            )
        },
    );
}

#[test]
fn process_host_rejects_crossed_same_kind_retained_outcomes_before_route_release() {
    with_stopped_no_task_process(
        "crossed-retained-first",
        NoTaskProcessStop::RouteRelease,
        |first| {
            with_stopped_no_task_process(
                "crossed-retained-second",
                NoTaskProcessStop::RouteRelease,
                |second| {
                    let StoppedNoTaskProcess {
                        host: first_host,
                        selector: first_selector,
                        failure: first_failure,
                        ..
                    } = first;
                    let StoppedNoTaskProcess {
                        host: second_host,
                        selector: second_selector,
                        failure: second_failure,
                        ..
                    } = second;
                    let (first_crossed, second_crossed) =
                        cross_a2a_retained_outcomes_for_test(first_failure, second_failure);
                    let first_crossed =
                        match first_host.retry_a2a_delegate_route_release(first_crossed) {
                            Ok(_) => {
                                panic!("the first route must reject the second retained outcome")
                            }
                            Err(failure) => failure,
                        };
                    let second_crossed =
                        match second_host.retry_a2a_delegate_route_release(second_crossed) {
                            Ok(_) => {
                                panic!("the second route must reject the first retained outcome")
                            }
                            Err(failure) => failure,
                        };
                    assert!(first_host
                        .a2a_delegate_routes
                        .reserve(first_selector.clone())
                        .is_err());
                    assert!(second_host
                        .a2a_delegate_routes
                        .reserve(second_selector.clone())
                        .is_err());
                    let (first_exact, second_exact) =
                        cross_a2a_retained_outcomes_for_test(*first_crossed, *second_crossed);
                    let _ = first_host
                        .retry_a2a_delegate_route_release(first_exact)
                        .unwrap_or_else(|_| {
                            panic!("the first exact retained outcome must release")
                        });
                    let _ = second_host
                        .retry_a2a_delegate_route_release(second_exact)
                        .unwrap_or_else(|_| {
                            panic!("the second exact retained outcome must release")
                        });
                    assert!(first_host
                        .a2a_delegate_routes
                        .reserve(first_selector.clone())
                        .is_ok());
                    assert!(second_host
                        .a2a_delegate_routes
                        .reserve(second_selector.clone())
                        .is_ok());
                },
            )
        },
    );
}
