//! Behavioral laws for routed A2A `CancelTask` ownership.

use super::*;
use crate::local_operation_start_fixture::{record_ref, schema_digest, signature, RecoveryParents};
use crate::resident::a2a_task_custody::{
    A2aCancelTaskDispositionCommitInput, A2aCancelTaskReadbackCommitInput,
};
use crate::resident::operation_dispatch::selection::tests::a2a_readback_service::{
    exchange_limits, recover_a2a_task_history_after_local_commit, with_recovered_a2a_task_history,
    A2aTaskHistoryFixture, Clock,
};
use crate::resident::process_host::a2a_cancel_task::*;
use crate::resident::process_host::a2a_task_routes::{
    ResidentProcessA2aTaskRouteInsertError, ResidentProcessA2aTaskRoutes,
};
use crate::resident::{
    a2a_cancel_task_owner::{
        a2a_cancel_task_key, ResidentA2aCancelTaskDurableCustody, ResidentA2aCancelTaskStartContext,
    },
    A2aCancelTaskDispositionInput, A2aCancelTaskInput, A2aCancelTaskObservationInput,
    OperationAttemptHistoryInput, OperationObservationAuthorityStoreRejection,
    RecoveredA2aTaskHistory,
};
use pale_core::records::{
    A2aJsonRpcRequestIdV1, A2aRemoteContextIdV1, A2aRemoteTaskIdV1, A2aRemoteTaskStateV1,
    A2aTaskBindingDigest, A2aTaskBindingIdV1, CapabilityIdV1, CommonRequestEnvelopeV1,
    ControlCheckpointIdV1, ControlCheckpointSigningAuthorityAdmissionContextV1,
    ControlIncarnationIdV1, ControlStateCheckpointV1, ControlStoreIdV1, OperationObservationV1,
    ReconciliationFrontierDigest, ReconciliationFrontierReferenceV1, RequestIdempotencyKeyV1,
    ResidentCorrelationIdV1, ResidentRequestIdV1, SchemaId, SignedResidentAdmissionContextV1,
    UnixTimeV1, A2A_TASK_OBSERVATION_V1_SCHEMA_ID, OPERATION_HANDLE_V1_SCHEMA,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    A2aExchange, A2aExchangeCoordinate, A2aExchangeError, A2aExchangeEvidence, A2aExchangeFacts,
    A2aExchangeInput, A2aExchangePort, A2aExchangeRequest, A2aPreparedExchange,
    AccumulatorMembershipLimits, AccumulatorPreparationLimits, AuthorityStoreCoordinate,
    ControlStoreError, SessionLeaseBody, SessionLeaseBodyRead, SessionLeaseBodyRetention,
    SessionLeaseCustodyPort, WallClockSourceError, WallClockSourcePort,
};
use std::{
    cell::{Cell, RefCell},
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

struct UnusedSessions;

fn admit_cancel_task(
    _: crate::resident::A2aCancelTaskAuthorizationContext<'_>,
) -> Result<(), pale_core::records::InvalidResidentRecordV1> {
    Ok(())
}

impl SessionLeaseCustodyPort for UnusedSessions {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("CancelTask uses the admitted historical session")
    }

    fn read(
        &self,
        _: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        panic!("CancelTask uses the admitted historical session")
    }
}

pub(super) struct SelectedStore(pub(super) AuthorityStoreCoordinate);

impl crate::resident::OperationObservationAuthorityStoreResolver for SelectedStore {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        Ok(self.0.clone())
    }
}

struct FailingClock;

impl WallClockSourcePort for FailingClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Err(WallClockSourceError::from_source(std::io::Error::other(
            "fresh preflight unavailable",
        )))
    }
}

struct PanicClock;

impl WallClockSourcePort for PanicClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        panic!("route admission must finish before the clock is read")
    }
}

struct PendingCancelExchange {
    coordinate: A2aExchangeCoordinate,
    stopped: Arc<AtomicBool>,
}

struct CompletedCancelExchange {
    coordinate: A2aExchangeCoordinate,
    request_id: String,
    response_sent: bool,
}

impl A2aExchange for CompletedCancelExchange {
    fn coordinate(&self) -> &A2aExchangeCoordinate {
        &self.coordinate
    }

    fn poll_next(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<A2aExchangeEvidence, A2aExchangeError>>> {
        if self.response_sent {
            return Poll::Ready(None);
        }
        self.response_sent = true;
        Poll::Ready(Some(Ok(A2aExchangeEvidence {
            facts: A2aExchangeFacts::Task {
                task: A2aRemoteTaskIdV1::parse("task-1").unwrap(),
                context: A2aRemoteContextIdV1::parse("context-1").unwrap(),
                state: A2aRemoteTaskStateV1::Canceled,
            },
            bytes: format!(
                r#"{{"jsonrpc":"2.0","id":"{}","result":{{"id":"task-1","contextId":"context-1","status":{{"state":"TASK_STATE_CANCELED"}}}}}}"#,
                self.request_id,
            )
            .into_bytes(),
            sse: None,
        })))
    }

    fn cancel(&mut self) {}
}

impl A2aExchange for PendingCancelExchange {
    fn coordinate(&self) -> &A2aExchangeCoordinate {
        &self.coordinate
    }

    fn poll_next(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<pale_ports::A2aExchangeEvidence, A2aExchangeError>>> {
        Poll::Pending
    }

    fn cancel(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
    }
}

#[derive(Default)]
pub(super) struct CountingCancelPort {
    preparations: Cell<usize>,
    starts: Cell<usize>,
    reject_start: Cell<bool>,
    complete_response: Cell<bool>,
    request_bytes: RefCell<Option<Vec<u8>>>,
    stopped: Arc<AtomicBool>,
}

impl CountingCancelPort {
    fn reject_next_start(&self) {
        self.reject_start.set(true);
    }

    fn complete_next_response(&self) {
        self.complete_response.set(true);
    }

    pub(super) fn starts(&self) -> usize {
        self.starts.get()
    }

    pub(super) fn with_request_bytes(bytes: Vec<u8>) -> Self {
        Self {
            request_bytes: RefCell::new(Some(bytes)),
            ..Self::default()
        }
    }
}

impl A2aExchangePort for CountingCancelPort {
    fn prepare(
        &self,
        request: A2aExchangeRequest,
    ) -> Result<A2aPreparedExchange, A2aExchangeError> {
        self.preparations.set(self.preparations.get() + 1);
        assert!(matches!(
            request.input(),
            A2aExchangeInput::CancelTask { task, context }
                if task.as_str() == "task-1" && context.as_str() == "context-1"
        ));
        let request_id = request.coordinate().request_id().as_str().to_owned();
        let bytes = self.request_bytes.borrow_mut().take().unwrap_or_else(|| {
            format!(
                r#"{{"jsonrpc":"2.0","id":"{request_id}","method":"CancelTask","params":{{"id":"task-1"}}}}"#,
            )
            .into_bytes()
        });
        A2aPreparedExchange::from_encoded(
            request,
            bytes,
            ArtifactDigest::hash_bytes(b"CancelTask fixture TLS trust"),
        )
    }

    fn start(
        &self,
        prepared: A2aPreparedExchange,
    ) -> Result<Box<dyn A2aExchange>, A2aExchangeError> {
        self.starts.set(self.starts.get() + 1);
        if self.reject_start.replace(false) {
            return Err(A2aExchangeError::ServiceUnavailable);
        }
        if self.complete_response.replace(false) {
            return Ok(Box::new(CompletedCancelExchange {
                request_id: prepared
                    .coordinate()
                    .context()
                    .request_id()
                    .as_str()
                    .to_owned(),
                coordinate: prepared.coordinate().clone(),
                response_sent: false,
            }));
        }
        Ok(Box::new(PendingCancelExchange {
            coordinate: prepared.coordinate().clone(),
            stopped: Arc::clone(&self.stopped),
        }))
    }
}

pub(in crate::resident) fn checkpoint_signer(
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

fn copied_attempt<'a>(
    input: &OperationAttemptHistoryInput<'a>,
) -> OperationAttemptHistoryInput<'a> {
    OperationAttemptHistoryInput {
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

pub(super) fn cancellation_request(
    original: &CommonRequestEnvelopeV1,
    handle: &pale_core::records::OperationHandleV1,
) -> CommonRequestEnvelopeV1 {
    let mut preimage = original.to_preimage();
    preimage.request_id = ResidentRequestIdV1::parse("process-cancel-request").unwrap();
    preimage.correlation_id = ResidentCorrelationIdV1::parse("process-cancel-request").unwrap();
    preimage.capability_id =
        CapabilityIdV1::parse(crate::resident::OPERATION_CANCEL_CAPABILITY_ID).unwrap();
    preimage.idempotency_key = RequestIdempotencyKeyV1::parse("process-cancel-request").unwrap();
    preimage.issued_at = UnixTimeV1::new(handle.created_at().get() + 2).unwrap();
    preimage.expires_at = *handle.retain_until();
    preimage.generation = Some(handle.compiled_generation_sha256().clone());
    preimage.fence_token = Some(handle.activation_fence_sha256().clone());
    preimage.body.schema_id = SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).unwrap();
    preimage.body.schema_sha256 = handle.schema_sha256().clone();
    preimage.body.payload = serde_json::to_value(handle).unwrap();
    preimage.body.payload_sha256 = ArtifactDigest::hash_bytes(
        pale_core::canon::to_canonical_string(&preimage.body.payload)
            .unwrap()
            .as_bytes(),
    );
    crate::effect_case_lineage_fixture::request(&preimage)
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

fn other_store() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("cancel-other-store").expect("other store"),
        ControlIncarnationIdV1::parse("cancel-other-incarnation").expect("other incarnation"),
    )
}

pub(super) fn cancel_input<'a>(
    request: &'a CommonRequestEnvelopeV1,
    history: &'a RecoveredA2aTaskHistory,
    attempt: &OperationAttemptHistoryInput<'a>,
    request_id: &'a A2aJsonRpcRequestIdV1,
    observation_schema: &'a SchemaDigest,
    signing: &'a SigningAuthorityBindingV1,
) -> A2aCancelTaskInput<'a> {
    A2aCancelTaskInput {
        request,
        history,
        attempt: copied_attempt(attempt),
        request_id,
        request_record_ref: record_ref("process/cancel-request"),
        response_record_ref: record_ref("process/cancel-response"),
        additional_response_record_ref: record_ref("process/cancel-additional-response"),
        cancellation_observation_ref: record_ref("process/cancel-observation"),
        signing: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: observation_schema,
            signing_authority: signing,
        },
        limits: exchange_limits(),
        maximum_evidence_bytes: 8_192,
    }
}

struct PreparedCancelRequestFixture<'a, 'store> {
    host: &'a mut ResidentProcessHost<'store, UnprovedHeldWriter>,
    selector: ResidentDeploymentAuthoritySelector,
    store: AuthorityStoreCoordinate,
    sessions: UnusedSessions,
    port: CountingCancelPort,
    prepared: ResidentProcessPreparedA2aCancelTask<'a>,
    history: &'a RecoveredA2aTaskHistory,
    attempt: OperationAttemptHistoryInput<'a>,
    signing: &'a SigningAuthorityBindingV1,
    observation_schema: &'a SchemaDigest,
    resolver: SelectedStore,
    authority: &'a mut ProvingAuthority,
    custody: &'a mut dyn pale_ports::OperationCustodyPort,
    anchor: &'a mut dyn pale_ports::AntiRollbackAnchorPort,
    parents: RecoveryParents<'a>,
}

#[allow(clippy::too_many_lines)]
fn with_prepared_cancel_request(
    faults: A2aTaskHistoryFixture,
    run: impl for<'a, 'store> FnOnce(PreparedCancelRequestFixture<'a, 'store>) -> bool,
) {
    with_recovered_a2a_task_history(
        faults,
        |history,
         attempt,
         signing,
         checkpoint,
         authority,
         custody,
         anchor,
         parents,
         restart_generation| {
            let selected_store = history.local_observation().store().clone();
            let selector = selector_from_signing_authority(signing);
            let application_store = MemoryAuthorityStore::default();
            assert_eq!(
                restart_generation.record().compiled_generation_digest(),
                attempt.generation.record().compiled_generation_digest()
            );
            let mut application = ready_application(&application_store, selected_store.clone(), 1);
            application.fixture_retain_generation(restart_generation);
            let deployment =
                unproved_deployment_for_selector(&selector, selected_store.clone(), application);
            let mut host = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                vec![deployment],
                NonZeroUsize::new(1).expect("one deployment"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one ready CancelTask deployment");
            let sessions = UnusedSessions;
            let port = CountingCancelPort::default();
            let request = cancellation_request(attempt.request, attempt.operation.handle().1);
            let request_id = A2aJsonRpcRequestIdV1::parse("process-cancel-rpc").unwrap();
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let checkpoint_authority = checkpoint_signer(checkpoint);
            let resolver = SelectedStore(selected_store.clone());
            let input = cancel_input(
                &request,
                history,
                &attempt,
                &request_id,
                &observation_schema,
                signing,
            );
            let prepared = host
                .prepare_a2a_cancel_task(
                    &selector,
                    process_access(&selected_store, custody, &sessions, anchor),
                    authority,
                    input,
                    &port,
                    &Clock::at(65),
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    OperationCheckpointInput {
                        reference: record_ref("process/cancel-checkpoint"),
                        id: ControlCheckpointIdV1::parse("process-cancel-checkpoint").unwrap(),
                    },
                    UnixTimeV1::new(66).unwrap(),
                    checkpoint.schema_sha256(),
                    &checkpoint_authority,
                    AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000).unwrap(),
                    &resolver,
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("CancelTask preparation"));
            run(PreparedCancelRequestFixture {
                host: &mut host,
                selector,
                store: selected_store,
                sessions,
                port,
                prepared,
                history,
                attempt,
                signing,
                observation_schema: &observation_schema,
                resolver,
                authority,
                custody,
                anchor,
                parents,
            })
        },
    );
}

#[test]
fn process_host_rejects_conflicting_and_saturated_routes_before_lower_io() {
    with_recovered_a2a_task_history(
        A2aTaskHistoryFixture::default(),
        |history, attempt, signing, checkpoint, authority, custody, anchor, _parents, _| {
            let store = history.local_observation().store().clone();
            let selector = selector_from_signing_authority(signing);
            let application_store = MemoryAuthorityStore::default();
            let deployment = unproved_deployment_for_selector(
                &selector,
                store.clone(),
                ready_application(&application_store, store.clone(), 1),
            );
            let mut host = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
                vec![deployment],
                NonZeroUsize::new(1).expect("one deployment"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one ready CancelTask deployment");
            let sessions = UnusedSessions;
            let port = CountingCancelPort::default();
            let request = cancellation_request(attempt.request, attempt.operation.handle().1);
            let request_id = A2aJsonRpcRequestIdV1::parse("process-cancel-rpc").unwrap();
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let checkpoint_authority = checkpoint_signer(checkpoint);
            let resolver = SelectedStore(store.clone());
            let key = a2a_cancel_task_key(history);

            host.a2a_task_routes
                .insert(key.clone(), selector.clone())
                .unwrap_or_else(|_| panic!("exact active binding route"));
            let conflict = host.prepare_a2a_cancel_task(
                &selector,
                process_access(&store, custody, &sessions, anchor),
                authority,
                cancel_input(
                    &request,
                    history,
                    &attempt,
                    &request_id,
                    &observation_schema,
                    signing,
                ),
                &port,
                &PanicClock,
                |_| panic!("route conflict must precede authorization"),
                |_, _, _, _| panic!("route conflict must precede rotation admission"),
                |_, _, _| panic!("route conflict must precede observation signing"),
                OperationCheckpointInput {
                    reference: record_ref("process/cancel-conflict-checkpoint"),
                    id: ControlCheckpointIdV1::parse("process-cancel-conflict-checkpoint").unwrap(),
                },
                UnixTimeV1::new(66).unwrap(),
                checkpoint.schema_sha256(),
                &checkpoint_authority,
                AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000).unwrap(),
                &resolver,
                |_, _, _| panic!("route conflict must precede checkpoint signing"),
            );
            assert!(matches!(
                conflict,
                Err(ResidentProcessA2aCancelTaskPreparationFailure::RouteConflict)
            ));
            assert_eq!(port.preparations.get(), 0);
            assert_eq!(port.starts.get(), 0);
            assert!(host.a2a_task_routes.remove_active(&key, &selector));

            let mut occupied = key.clone();
            occupied.task_binding_id =
                A2aTaskBindingIdV1::parse("process-occupied-task").expect("occupied task");
            occupied.task_binding_ref = record_ref("process/occupied-task-binding");
            occupied.task_binding_sha256 = A2aTaskBindingDigest::parse(
                ArtifactDigest::hash_bytes(b"process occupied task binding").to_string(),
            )
            .expect("occupied binding digest");
            host.a2a_task_routes
                .insert(occupied.clone(), selector.clone())
                .unwrap_or_else(|_| panic!("one unlike route fills capacity"));
            let saturated = host.prepare_a2a_cancel_task(
                &selector,
                process_access(&store, custody, &sessions, anchor),
                authority,
                cancel_input(
                    &request,
                    history,
                    &attempt,
                    &request_id,
                    &observation_schema,
                    signing,
                ),
                &port,
                &PanicClock,
                |_| panic!("route saturation must precede authorization"),
                |_, _, _, _| panic!("route saturation must precede rotation admission"),
                |_, _, _| panic!("route saturation must precede observation signing"),
                OperationCheckpointInput {
                    reference: record_ref("process/cancel-saturated-checkpoint"),
                    id: ControlCheckpointIdV1::parse("process-cancel-saturated-checkpoint")
                        .unwrap(),
                },
                UnixTimeV1::new(66).unwrap(),
                checkpoint.schema_sha256(),
                &checkpoint_authority,
                AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000).unwrap(),
                &resolver,
                |_, _, _| panic!("route saturation must precede checkpoint signing"),
            );
            assert!(matches!(
                saturated,
                Err(ResidentProcessA2aCancelTaskPreparationFailure::RouteSaturated)
            ));
            assert_eq!(port.preparations.get(), 0);
            assert_eq!(port.starts.get(), 0);
            assert!(host.a2a_task_routes.remove_active(&occupied, &selector));
            true
        },
    );
}

struct PreparedCancelTaskFixture<'a, 'store> {
    host: &'a mut ResidentProcessHost<'store, UnprovedHeldWriter>,
    selector: ResidentDeploymentAuthoritySelector,
    store: AuthorityStoreCoordinate,
    sessions: UnusedSessions,
    port: CountingCancelPort,
    retained: ResidentProcessRetainedUnopenedA2aCancelTask<'a>,
    history: RecoveredA2aTaskHistory,
    attempt: OperationAttemptHistoryInput<'a>,
    signing: &'a SigningAuthorityBindingV1,
    checkpoint: ControlStateCheckpointV1,
    observation_schema: &'a SchemaDigest,
    resolver: SelectedStore,
    authority: &'a mut ProvingAuthority,
    custody: &'a mut dyn pale_ports::OperationCustodyPort,
    anchor: &'a mut dyn pale_ports::AntiRollbackAnchorPort,
    parents: RecoveryParents<'a>,
}

#[allow(clippy::too_many_lines)]
fn with_prepared_cancel_task_with_faults(
    faults: A2aTaskHistoryFixture,
    run: impl for<'a, 'store> FnOnce(PreparedCancelTaskFixture<'a, 'store>) -> bool,
) {
    with_prepared_cancel_request(
        faults,
        |PreparedCancelRequestFixture {
             host,
             selector,
             store: selected_store,
             sessions,
             port,
             prepared,
             history,
             attempt,
             signing,
             observation_schema,
             resolver,
             authority,
             custody,
             anchor,
             parents,
         }| {
            let retained = host
                .retain_prepared_a2a_cancel_task(
                    process_access(&selected_store, custody, &sessions, anchor),
                    authority,
                    prepared,
                )
                .unwrap_or_else(|_| panic!("CancelTask request custody"));
            let cancellation_checkpoint = retained.request_checkpoint().1.clone();
            let fresh_history = recover_a2a_task_history_after_local_commit(
                history,
                attempt.operation,
                &cancellation_checkpoint,
                signing,
                authority,
                custody,
                anchor,
            );
            run(PreparedCancelTaskFixture {
                host,
                selector,
                store: selected_store,
                sessions,
                port,
                retained,
                history: fresh_history,
                attempt,
                signing,
                checkpoint: cancellation_checkpoint,
                observation_schema,
                resolver,
                authority,
                custody,
                anchor,
                parents,
            })
        },
    );
}

fn with_prepared_cancel_task(
    run: impl for<'a, 'store> FnOnce(PreparedCancelTaskFixture<'a, 'store>) -> bool,
) {
    with_prepared_cancel_task_with_faults(
        A2aTaskHistoryFixture {
            expected_callback_appends: 2,
            ..A2aTaskHistoryFixture::default()
        },
        run,
    );
}

#[test]
fn checked_out_binding_blocks_cross_class_reuse_and_holds_route_capacity() {
    with_recovered_a2a_task_history(
        A2aTaskHistoryFixture::default(),
        |history, _, signing, _, _, _, _, _, _| {
            let operation_host = ResidentOperationHost::new(task_state());
            let mut routes = ResidentProcessA2aTaskRoutes::new(
                operation_host.operation_task_owner_instance().clone(),
                operation_host.aggregate_task_limits(),
            )
            .expect("bounded route ledger");
            let key = a2a_cancel_task_key(history);
            let selector = selector_from_signing_authority(signing);
            routes
                .insert(key.clone(), selector.clone())
                .unwrap_or_else(|_| panic!("first request class reserves the binding route"));
            let custody = routes
                .check_out(&key)
                .expect("terminal evidence keeps the binding checked out");
            assert!(matches!(
                routes.insert(key.clone(), selector.clone()),
                Err(ResidentProcessA2aTaskRouteInsertError::Conflict)
            ));

            let mut unlike = key;
            unlike.task_binding_id =
                A2aTaskBindingIdV1::parse("process-unlike-task").expect("unlike task");
            unlike.task_binding_ref = record_ref("process/unlike-task-binding");
            unlike.task_binding_sha256 = A2aTaskBindingDigest::parse(
                ArtifactDigest::hash_bytes(b"process unlike task binding").to_string(),
            )
            .expect("unlike binding digest");
            assert!(matches!(
                routes.insert(unlike, selector),
                Err(ResidentProcessA2aTaskRouteInsertError::Saturated)
            ));
            routes
                .release(custody)
                .unwrap_or_else(|_| panic!("exact checked-out route release"));
            true
        },
    );
}

#[test]
fn replayed_request_custody_never_regains_carrier_authority_after_anchor_retry() {
    with_prepared_cancel_request(
        A2aTaskHistoryFixture {
            expected_callback_appends: 1,
            additional_read_backs: 1,
            additional_anchor_advances: 1,
            unknown_exact_commits: vec![2],
            unavailable_advances: vec![2],
            ..A2aTaskHistoryFixture::default()
        },
        |PreparedCancelRequestFixture {
             host,
             store,
             sessions,
             port,
             prepared,
             authority,
             custody,
             anchor,
             ..
         }| {
            let first = match host.retain_prepared_a2a_cancel_task(
                process_access(&store, custody, &sessions, anchor),
                authority,
                prepared,
            ) {
                Err(pending) => pending,
                Ok(_) => panic!("unknown database commit must require reconciliation"),
            };
            assert_eq!(port.starts.get(), 0);

            let second = match host.continue_a2a_cancel_task_request_custody(
                process_access(&store, custody, &sessions, anchor),
                authority,
                first,
            ) {
                Err(pending) => pending,
                Ok(_) => panic!("first independent-anchor advance is unavailable"),
            };
            assert_eq!(port.starts.get(), 0);

            let not_new = match host.continue_a2a_cancel_task_request_custody(
                process_access(&store, custody, &sessions, anchor),
                authority,
                second,
            ) {
                Err(pending) => pending,
                Ok(_) => panic!("replayed database custody must remain non-dispatchable"),
            };
            let classified = host
                .classify_non_dispatchable_replay_a2a_cancel_task(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    not_new,
                )
                .unwrap_or_else(|_| panic!("exact replay must classify without carrier release"));
            host.finish_non_dispatchable_replay_a2a_cancel_task(
                process_access(&store, custody, &sessions, anchor),
                authority,
                classified,
            )
            .unwrap_or_else(|_| panic!("non-dispatchable replay capacity release"));
            assert_eq!(port.preparations.get(), 1);
            assert_eq!(port.starts.get(), 0);
            true
        },
    );
}

#[test]
fn replacement_process_host_cannot_continue_original_request_custody_before_io() {
    with_prepared_cancel_request(
        A2aTaskHistoryFixture {
            expected_callback_appends: 1,
            ..A2aTaskHistoryFixture::default()
        },
        |PreparedCancelRequestFixture {
             host,
             selector,
             store,
             sessions,
             port,
             prepared,
             authority,
             custody,
             anchor,
             ..
         }| {
            let replacement_application_store = MemoryAuthorityStore::default();
            let replacement_deployment = unproved_deployment_for_selector(
                &selector,
                store.clone(),
                ready_application(&replacement_application_store, store.clone(), 1),
            );
            let mut replacement = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
                vec![replacement_deployment],
                NonZeroUsize::new(1).expect("one deployment"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("replacement CancelTask host");
            let pending = match replacement.retain_prepared_a2a_cancel_task(
                process_access(&store, custody, &sessions, anchor),
                authority,
                prepared,
            ) {
                Err(pending) => pending,
                Ok(_) => panic!("replacement host must reject before request custody I/O"),
            };
            with_unproved_operation_service_for_test(
                &mut replacement,
                &selector,
                process_access(&store, custody, &sessions, anchor),
                authority,
                |_store, service| assert_eq!(service.outstanding_task_reservations(), 0),
            )
            .expect("replacement deployment remains inspectable");
            let _retained = host
                .continue_a2a_cancel_task_request_custody(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    pending,
                )
                .unwrap_or_else(|_| panic!("the original owner continues exact request custody"));
            with_unproved_operation_service_for_test(
                host,
                &selector,
                process_access(&store, custody, &sessions, anchor),
                authority,
                |_store, service| assert_eq!(service.outstanding_task_reservations(), 1),
            )
            .expect("original deployment remains inspectable");
            assert_eq!(port.preparations.get(), 1);
            assert_eq!(port.starts.get(), 0);
            true
        },
    );
}

#[test]
fn proved_not_started_is_durably_retained_without_remote_state_promotion() {
    with_prepared_cancel_task_with_faults(
        A2aTaskHistoryFixture {
            expected_callback_appends: 2,
            additional_read_backs: 2,
            additional_anchor_advances: 1,
            unknown_exact_commits: vec![3],
            unavailable_read_backs: vec![4],
            unavailable_advances: vec![3],
            ..A2aTaskHistoryFixture::default()
        },
        |PreparedCancelTaskFixture {
             host,
             selector: _,
             store,
             sessions,
             port,
             retained,
             history,
             attempt,
             signing,
             checkpoint,
             observation_schema,
             resolver,
             authority,
             custody,
             anchor,
             parents: _,
         }| {
            port.reject_next_start();
            let awaiting = match host
                .start_a2a_cancel_task(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    retained,
                    ResidentA2aCancelTaskStartContext::new(
                        &history,
                        &port,
                        &Clock::at(65),
                        admit_cancel_task,
                    ),
                )
                .unwrap_or_else(|_| panic!("aggregate acceptance before carrier rejection"))
            {
                ResidentProcessA2aCancelTaskStart::AwaitingCustody(awaiting) => awaiting,
                _ => panic!("proved carrier rejection must retain a NotStarted disposition"),
            };
            assert_eq!(port.preparations.get(), 1);
            assert_eq!(port.starts.get(), 1);
            assert!(matches!(
                history.local_observation().selected_head().1.state(),
                pale_core::records::OperationObservationStateV1::CancellationRequested { .. }
            ));

            let selected = host
                .select_a2a_cancel_task_disposition(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    awaiting,
                )
                .unwrap_or_else(|_| panic!("NotStarted disposition selection"));
            let checkpoint_authority = checkpoint_signer(&checkpoint);
            let first = match host.retain_a2a_cancel_task_disposition(
                process_access(&store, custody, &sessions, anchor),
                authority,
                selected,
                A2aCancelTaskDispositionCommitInput {
                    disposition: A2aCancelTaskDispositionInput {
                        history: &history,
                        operation: attempt.operation,
                        diagnostic_ref: record_ref("process/cancel-not-started-diagnostic"),
                        observation_ref: record_ref("process/cancel-not-started-observation"),
                        reconciliation: ReconciliationFrontierReferenceV1 {
                            record_ref: record_ref("process/cancel-not-started-reconciliation"),
                            record_sha256: ReconciliationFrontierDigest::parse(
                                ArtifactDigest::hash_bytes(b"cancel not started").to_string(),
                            )
                            .unwrap(),
                        },
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: observation_schema,
                            signing_authority: signing,
                        },
                        observed_at: UnixTimeV1::new(66).unwrap(),
                        maximum_diagnostic_bytes: 65_536,
                    },
                    checkpoint: OperationCheckpointInput {
                        reference: record_ref("process/cancel-not-started-checkpoint"),
                        id: ControlCheckpointIdV1::parse("process-cancel-not-started-checkpoint")
                            .unwrap(),
                    },
                    checkpoint_issued_at: UnixTimeV1::new(67).unwrap(),
                    checkpoint_schema: checkpoint.schema_sha256(),
                    checkpoint_authority: &checkpoint_authority,
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        8, 20_000, 20_000, 8_192, 200_000,
                    )
                    .unwrap(),
                },
                &resolver,
                |_| Ok(()),
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
                |_, _, _| Ok(signature()),
            ) {
                Err(pending) => pending,
                Ok(_) => panic!("unknown disposition commit must require reconciliation"),
            };
            let second = match host.continue_a2a_cancel_task_disposition_custody(
                process_access(&store, custody, &sessions, anchor),
                authority,
                first,
            ) {
                Err(pending) => pending,
                Ok(_) => panic!("first disposition anchor advance is unavailable"),
            };
            let retained = host
                .continue_a2a_cancel_task_disposition_custody(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    second,
                )
                .unwrap_or_else(|_| panic!("continued NotStarted disposition custody"));
            let durable = host
                .finish_a2a_cancel_task(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    retained,
                )
                .unwrap_or_else(|_| panic!("NotStarted aggregate then route release"));
            assert!(matches!(
                durable,
                ResidentA2aCancelTaskDurableCustody::Disposition(_)
            ));
            assert_eq!(port.starts.get(), 1, "carrier rejection is never replayed");
            true
        },
    );
}

#[test]
fn replacement_route_cannot_release_durable_cancel_task_custody() {
    with_prepared_cancel_task(
        |PreparedCancelTaskFixture {
             host,
             selector: _,
             store,
             sessions,
             port,
             retained,
             history,
             attempt,
             signing,
             checkpoint,
             observation_schema,
             resolver,
             authority,
             custody,
             anchor,
             parents: _,
         }| {
            port.reject_next_start();
            let awaiting = match host
                .start_a2a_cancel_task(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    retained,
                    ResidentA2aCancelTaskStartContext::new(
                        &history,
                        &port,
                        &Clock::at(65),
                        admit_cancel_task,
                    ),
                )
                .unwrap_or_else(|_| panic!("aggregate acceptance before carrier rejection"))
            {
                ResidentProcessA2aCancelTaskStart::AwaitingCustody(awaiting) => awaiting,
                _ => panic!("proved carrier rejection must retain a NotStarted disposition"),
            };
            let selected = host
                .select_a2a_cancel_task_disposition(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    awaiting,
                )
                .unwrap_or_else(|_| panic!("NotStarted disposition selection"));
            let checkpoint_authority = checkpoint_signer(&checkpoint);
            let retained = host
                .retain_a2a_cancel_task_disposition(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    selected,
                    A2aCancelTaskDispositionCommitInput {
                        disposition: A2aCancelTaskDispositionInput {
                            history: &history,
                            operation: attempt.operation,
                            diagnostic_ref: record_ref("process/cancel-route-diagnostic"),
                            observation_ref: record_ref("process/cancel-route-observation"),
                            reconciliation: ReconciliationFrontierReferenceV1 {
                                record_ref: record_ref("process/cancel-route-reconciliation"),
                                record_sha256: ReconciliationFrontierDigest::parse(
                                    ArtifactDigest::hash_bytes(b"cancel route release").to_string(),
                                )
                                .unwrap(),
                            },
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: observation_schema,
                                signing_authority: signing,
                            },
                            observed_at: UnixTimeV1::new(66).unwrap(),
                            maximum_diagnostic_bytes: 65_536,
                        },
                        checkpoint: OperationCheckpointInput {
                            reference: record_ref("process/cancel-route-checkpoint"),
                            id: ControlCheckpointIdV1::parse("process-cancel-route-checkpoint")
                                .unwrap(),
                        },
                        checkpoint_issued_at: UnixTimeV1::new(67).unwrap(),
                        checkpoint_schema: checkpoint.schema_sha256(),
                        checkpoint_authority: &checkpoint_authority,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            8, 20_000, 20_000, 8_192, 200_000,
                        )
                        .unwrap(),
                    },
                    &resolver,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("NotStarted disposition custody"));
            let (exact_route, custody_result) = host
                .release_a2a_cancel_task_aggregate_without_route_for_test(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    retained,
                );
            let key = exact_route.key.as_ref().clone();
            let selector = selector_from_signing_authority(signing);
            let replacement_application_store = MemoryAuthorityStore::default();
            let replacement_deployment = unproved_deployment_for_selector(
                &selector,
                store.clone(),
                ready_application(&replacement_application_store, store.clone(), 1),
            );
            let mut replacement = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
                vec![replacement_deployment],
                NonZeroUsize::new(1).expect("one deployment"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("replacement CancelTask host");
            replacement
                .a2a_task_routes
                .insert(key.clone(), selector.clone())
                .unwrap_or_else(|_| panic!("replacement route"));
            let crossed_route = replacement
                .a2a_task_routes
                .check_out(&key)
                .expect("replacement checked-out route");
            let crossed = host.retry_a2a_cancel_task_release(
                process_access(&store, custody, &sessions, anchor),
                authority,
                ResidentProcessA2aCancelTaskReleaseFailure::Route {
                    route: crossed_route,
                    custody: Box::new(custody_result),
                },
            );
            let (crossed_route, custody_result) = match crossed {
                Err(ResidentProcessA2aCancelTaskReleaseFailure::Route { route, custody }) => {
                    (route, *custody)
                }
                _ => panic!("replacement route must not release exact durable custody"),
            };
            replacement
                .a2a_task_routes
                .release(crossed_route)
                .unwrap_or_else(|_| panic!("replacement route cleanup"));
            let durable = host
                .retry_a2a_cancel_task_release(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    ResidentProcessA2aCancelTaskReleaseFailure::Route {
                        route: exact_route,
                        custody: Box::new(custody_result),
                    },
                )
                .unwrap_or_else(|_| panic!("exact route release retry"));
            assert!(matches!(
                durable,
                ResidentA2aCancelTaskDurableCustody::Disposition(_)
            ));
            assert_eq!(port.starts.get(), 1, "carrier rejection is never replayed");
            true
        },
    );
}

#[test]
fn completed_readback_retains_remote_history_without_promoting_pale_state() {
    with_prepared_cancel_task_with_faults(
        A2aTaskHistoryFixture {
            expected_callback_appends: 2,
            additional_read_backs: 2,
            additional_anchor_advances: 1,
            unknown_exact_commits: vec![3],
            unavailable_read_backs: vec![4],
            unavailable_advances: vec![3],
            ..A2aTaskHistoryFixture::default()
        },
        |PreparedCancelTaskFixture {
             host,
             selector,
             store,
             sessions,
             port,
             retained,
             history,
             attempt,
             signing,
             checkpoint,
             observation_schema: _,
             resolver: _,
             authority,
             custody,
             anchor,
             parents: _,
         }| {
            port.complete_next_response();
            assert!(matches!(
                host.start_a2a_cancel_task(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    retained,
                    ResidentA2aCancelTaskStartContext::new(
                        &history,
                        &port,
                        &Clock::at(65),
                        admit_cancel_task,
                    ),
                )
                .unwrap_or_else(|_| panic!("exact read-back start")),
                ResidentProcessA2aCancelTaskStart::Active
            ));
            let key = a2a_cancel_task_key(&history);
            let protocol = pale_adapter_a2a::A2aCancelTaskDecoder;
            let mut cx = crate::resident::operation_dispatch::selection::tests::a2a_readback_service::poll_context();
            assert!(matches!(
                host.poll_a2a_cancel_task(
                    &selector,
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    &key,
                    ResidentProcessA2aCancelTaskPollContext::new(
                        &mut cx,
                        &protocol,
                        &Clock::at(66),
                    ),
                ),
                ResidentProcessA2aCancelTaskPoll::ResponseReceived
            ));
            let awaiting = match host.poll_a2a_cancel_task(
                &selector,
                process_access(&store, custody, &sessions, anchor),
                authority,
                &key,
                ResidentProcessA2aCancelTaskPollContext::new(&mut cx, &protocol, &Clock::at(66)),
            ) {
                ResidentProcessA2aCancelTaskPoll::AwaitingCustody(awaiting) => awaiting,
                _ => panic!("unary EOF must retain completed read-back evidence"),
            };
            let selected = host
                .select_a2a_cancel_task_readback(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    awaiting,
                )
                .unwrap_or_else(|_| panic!("completed read-back selection"));
            let task_schema = schema_digest(A2A_TASK_OBSERVATION_V1_SCHEMA_ID);
            let checkpoint_authority = checkpoint_signer(&checkpoint);
            let first = match host.retain_a2a_cancel_task_readback(
                process_access(&store, custody, &sessions, anchor),
                authority,
                selected,
                A2aCancelTaskReadbackCommitInput {
                    observation: A2aCancelTaskObservationInput {
                        history: &history,
                        operation: attempt.operation,
                        observation_ref: record_ref("process/cancel-readback-observation"),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &task_schema,
                            signing_authority: signing,
                        },
                        maximum_evidence_bytes: 8_192,
                        maximum_record_bytes: 65_536,
                    },
                    checkpoint: OperationCheckpointInput {
                        reference: record_ref("process/cancel-readback-checkpoint"),
                        id: ControlCheckpointIdV1::parse("process-cancel-readback-checkpoint")
                            .unwrap(),
                    },
                    issued_at: UnixTimeV1::new(67).unwrap(),
                    checkpoint_schema: checkpoint.schema_sha256(),
                    checkpoint_authority: &checkpoint_authority,
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        8, 20_000, 20_000, 8_192, 200_000,
                    )
                    .unwrap(),
                    membership_limits: AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                        .unwrap(),
                },
                &protocol,
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
                |_| Ok(store.clone()),
                |_, _, _| Ok(signature()),
            ) {
                Err(pending) => pending,
                Ok(_) => panic!("unknown read-back commit must require reconciliation"),
            };
            let second = match host.continue_a2a_cancel_task_readback_custody(
                process_access(&store, custody, &sessions, anchor),
                authority,
                first,
            ) {
                Err(pending) => pending,
                Ok(_) => panic!("first read-back anchor advance is unavailable"),
            };
            let retained = host
                .continue_a2a_cancel_task_readback_custody(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    second,
                )
                .unwrap_or_else(|_| panic!("continued completed read-back custody"));
            let release = host.reject_a2a_cancel_task_aggregate_release_with_replacement_for_test(
                process_access(&store, custody, &sessions, anchor),
                authority,
                retained,
            );
            assert!(matches!(
                release,
                ResidentProcessA2aCancelTaskReleaseFailure::Lower { .. }
            ));
            let durable = host
                .retry_a2a_cancel_task_release(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    release,
                )
                .unwrap_or_else(|_| panic!("read-back aggregate then route release retry"));
            let ResidentA2aCancelTaskDurableCustody::ReadbackCustody(readback) = durable else {
                panic!("completed Task evidence must remain remote-history custody");
            };
            assert_eq!(
                readback.observation().1.remote_state(),
                &A2aRemoteTaskStateV1::Canceled
            );
            assert!(matches!(
                history.local_observation().selected_head().1.state(),
                pale_core::records::OperationObservationStateV1::CancellationRequested { .. }
            ));
            assert_eq!(
                port.starts.get(),
                1,
                "completed read-back is never replayed"
            );
            true
        },
    );
}

#[test]
fn association_and_preflight_retries_reuse_one_unopened_request() {
    with_prepared_cancel_task_with_faults(
        A2aTaskHistoryFixture {
            expected_callback_appends: 1,
            ..A2aTaskHistoryFixture::default()
        },
        |PreparedCancelTaskFixture {
             host,
             selector: _,
             store: selected_store,
             sessions,
             port,
             retained,
             history: fresh_history,
             attempt: _,
             signing: _,
             checkpoint: _,
             observation_schema: _,
             resolver: _,
             authority,
             custody,
             anchor,
             parents: _,
         }| {
            let crossed_store = other_store();
            let association = match host.start_a2a_cancel_task(
                process_access(&crossed_store, custody, &sessions, anchor),
                authority,
                retained,
                ResidentA2aCancelTaskStartContext::new(
                    &fresh_history,
                    &port,
                    &Clock::at(65),
                    admit_cancel_task,
                ),
            ) {
                Err(failure) => failure,
                Ok(_) => panic!("crossed physical store must reject before carrier use"),
            };
            assert_eq!(port.preparations.get(), 1);
            assert_eq!(port.starts.get(), 0);

            let preflight = match host
                .retry_a2a_cancel_task_start_association(
                    process_access(&selected_store, custody, &sessions, anchor),
                    authority,
                    association,
                    ResidentA2aCancelTaskStartContext::new(
                        &fresh_history,
                        &port,
                        &FailingClock,
                        admit_cancel_task,
                    ),
                )
                .unwrap_or_else(|_| panic!("corrected association must accept aggregate capacity"))
            {
                ResidentProcessA2aCancelTaskStart::PreflightRejected(failure) => failure,
                _ => panic!("failed clock must preserve the accepted unopened request"),
            };
            assert_eq!(port.preparations.get(), 1);
            assert_eq!(port.starts.get(), 0);

            let started = host.retry_a2a_cancel_task_preflight(
                process_access(&selected_store, custody, &sessions, anchor),
                authority,
                preflight,
                ResidentA2aCancelTaskStartContext::new(
                    &fresh_history,
                    &port,
                    &Clock::at(65),
                    admit_cancel_task,
                ),
            );
            match started {
                ResidentProcessA2aCancelTaskStart::Active => {}
                ResidentProcessA2aCancelTaskStart::PreflightRejected(_) => {
                    panic!("fresh preflight remained rejected")
                }
                ResidentProcessA2aCancelTaskStart::AwaitingCustody(_) => {
                    panic!("carrier start closed before response polling")
                }
                ResidentProcessA2aCancelTaskStart::AggregateTransitionRejected { .. } => {
                    panic!("aggregate transition was rejected")
                }
            }
            assert_eq!(port.preparations.get(), 1);
            assert_eq!(port.starts.get(), 1);
            true
        },
    );
}

#[test]
fn local_response_read_stop_retains_durable_uncertainty_before_release() {
    with_prepared_cancel_task(
        |PreparedCancelTaskFixture {
             host,
             selector,
             store,
             sessions,
             port,
             retained,
             history,
             attempt,
             signing,
             checkpoint,
             observation_schema,
             resolver,
             authority,
             custody,
             anchor,
             parents: _,
         }| {
            assert!(matches!(
                host.start_a2a_cancel_task(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    retained,
                    ResidentA2aCancelTaskStartContext::new(
                        &history,
                        &port,
                        &Clock::at(65),
                        admit_cancel_task,
                    ),
                ),
                Ok(ResidentProcessA2aCancelTaskStart::Active)
            ));
            let key = a2a_cancel_task_key(&history);
            let awaiting = match host.stop_a2a_cancel_task_response_read(
                &selector,
                process_access(&store, custody, &sessions, anchor),
                authority,
                &key,
            ) {
                ResidentProcessA2aCancelTaskPoll::AwaitingCustody(awaiting) => awaiting,
                _ => panic!("local response-read stop must retain unknown disposition"),
            };
            assert!(port.stopped.load(Ordering::SeqCst));

            let selected = host
                .select_a2a_cancel_task_disposition(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    awaiting,
                )
                .unwrap_or_else(|_| panic!("unknown disposition selection"));
            let checkpoint_authority = checkpoint_signer(&checkpoint);
            let retained = host
                .retain_a2a_cancel_task_disposition(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    selected,
                    A2aCancelTaskDispositionCommitInput {
                        disposition: A2aCancelTaskDispositionInput {
                            history: &history,
                            operation: attempt.operation,
                            diagnostic_ref: record_ref("process/cancel-read-stop-diagnostic"),
                            observation_ref: record_ref("process/cancel-read-stop-observation"),
                            reconciliation: ReconciliationFrontierReferenceV1 {
                                record_ref: record_ref("process/cancel-read-stop-reconciliation"),
                                record_sha256: ReconciliationFrontierDigest::parse(
                                    ArtifactDigest::hash_bytes(b"cancel response-read stop")
                                        .to_string(),
                                )
                                .unwrap(),
                            },
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: observation_schema,
                                signing_authority: signing,
                            },
                            observed_at: UnixTimeV1::new(66).unwrap(),
                            maximum_diagnostic_bytes: 65_536,
                        },
                        checkpoint: OperationCheckpointInput {
                            reference: record_ref("process/cancel-read-stop-checkpoint"),
                            id: ControlCheckpointIdV1::parse("process-cancel-read-stop-checkpoint")
                                .unwrap(),
                        },
                        checkpoint_issued_at: UnixTimeV1::new(67).unwrap(),
                        checkpoint_schema: checkpoint.schema_sha256(),
                        checkpoint_authority: &checkpoint_authority,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            8, 20_000, 20_000, 8_192, 200_000,
                        )
                        .unwrap(),
                    },
                    &resolver,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("unknown disposition custody"));
            let durable = host
                .finish_a2a_cancel_task(
                    process_access(&store, custody, &sessions, anchor),
                    authority,
                    retained,
                )
                .unwrap_or_else(|_| panic!("aggregate then route release"));
            assert!(matches!(
                durable,
                ResidentA2aCancelTaskDurableCustody::Disposition(_)
            ));
            assert!(matches!(
                history.local_observation().selected_head().1.state(),
                pale_core::records::OperationObservationStateV1::CancellationRequested { .. }
            ));
            assert_eq!(
                port.starts.get(),
                1,
                "local read stop never replays CancelTask"
            );
            true
        },
    );
}
