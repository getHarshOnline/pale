//! Provider resolution and dispatch use the real application custody phases.
//! Test signing and accumulator callbacks remain models, not native proof.

use super::*;
use crate::local_operation_start_fixture::{
    artifact_digest, checkpoint_authority, checkpoint_preimage, issue_observation, sha, signature,
    store_coordinate,
};
use crate::resident::consultation::ResidentConsultationBatchCoordinate;
use crate::resident::local_operation_coordinator::ResidentLocalOperationCoordinator;
use crate::resident::operation_dispatch::provider::{
    prepare_provider_resolution, ProviderOperationDispatchContext, ProviderOperationDispatchInput,
    ProviderOperationInput, ProviderOperationInvokeFailure, ProviderOperationInvokeInput,
    ProviderOperationInvokeResult, ProviderOperationRecordingPreparationError,
    ProviderOperationUncertainty,
};
use crate::resident::operation_dispatch::OperationAttemptContext;
use crate::resident::operation_host::ResidentOperationHost;
use crate::resident::operation_observation_custody::tests::native_execution as custody;
use crate::resident::operation_service::{
    CancellationAppendAdmission, CancellationAppendInput, ResidentOperationCancellationAdmission,
    ResidentOperationCancellationFailure, ResidentOperationCancellationInput,
    ResidentOperationCancellationOutcome, ResidentOperationService, ResidentOperationTaskState,
};
use crate::resident::provider_task_service::{
    ResidentProviderDispatchOwnerSelection, ResidentProviderTaskAcceptanceFailure,
    ResidentProviderTaskPoll, ResidentProviderTaskRecoveryFailure, ResidentProviderTaskService,
};
use crate::resident::task_owner::{
    execute_provider_task, ResidentOperationTask, ResidentOperationTaskCancellationNotification,
    ResidentOperationTaskCancellationState, ResidentOperationTaskCompletion,
    ResidentOperationTaskExecutor, ResidentOperationTaskJoin, ResidentOperationTaskLimits,
    ResidentOperationTaskOwner, ResidentOperationTaskOwnerInstance, ResidentProviderTaskRefusal,
};
use crate::resident::task_reservation::{
    ResidentAggregateTaskLimits, ResidentTaskReservationPhase, ResidentTaskReservationRoute,
    ResidentTaskReservationTransitionFailure,
};
use crate::resident::{
    prepare_operation_context_binding, prepare_operation_observation_append,
    AmbientSessionAdmissionContext, AuthorizedOperationCancellation,
    LocalOperationHandleRecoveryInput, LocalOperationRecoveryContext, OperationAttemptError,
    OperationAttemptHistoryInput, OperationCheckpointAncestryLimits, OperationCheckpointInput,
    OperationContextBindingInput, OperationControlAuthorizationContext,
    OperationControlRecoveryAdmission, OperationControlRecoveryInput,
    OperationCustodyDatabaseEvidence, OperationObservationAuthorityStoreRejection,
    OperationObservationAuthorityStoreResolver, OperationObservationRecoveryLimits,
    OperationObservationSelection, OperationObservationTransition,
    ProviderOperationRetentionFailure, RecoveredLocalOperation, RecoveredOperationObservation,
    RetainedProviderOperationDispatch, OPERATION_CANCEL_CAPABILITY_ID,
};
use crate::resident_fixture::RecoveryResolver;
use crate::session_admission_fixture::{LocalStartContext, ProvingAuthority};
use pale_ports::{
    AccumulatorMembershipLimits, AccumulatorPreparationLimits, AntiRollbackAnchor,
    AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError, AntiRollbackAnchorErrorKind,
    AntiRollbackAnchorKey, AntiRollbackAnchorObservation, AntiRollbackAnchorPort,
    AuthorityStoreCoordinate, ControlRecordBytes, ControlStoreError, OperationCustodyPort,
    OperationCustodyRecord, PreparedAntiRollbackAnchorTransition, ProviderInvocationObservation,
    ProviderInvocationPort, ProviderInvocationRequest, ProviderInvocationSuccessWire,
    ProviderInvocationUnavailability, SessionLeaseBody, SessionLeaseBodyRead,
    SessionLeaseBodyRetention, SessionLeaseCustodyPort, WallClockSourceError, WallClockSourcePort,
};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

struct Clock {
    times: Vec<u64>,
    next: AtomicUsize,
}

impl WallClockSourcePort for Clock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        let index = self.next.fetch_add(1, Ordering::SeqCst);
        Ok(UnixTimeV1::new(self.times[index]).unwrap())
    }
}

struct Store;

pub(in crate::resident) fn cancellation_request(
    original: &CommonRequestEnvelopeV1,
    handle: &OperationHandleV1,
) -> CommonRequestEnvelopeV1 {
    let mut preimage = original.to_preimage();
    preimage.request_id = ResidentRequestIdV1::parse("provider-service-cancel").unwrap();
    preimage.correlation_id = ResidentCorrelationIdV1::parse("provider-service-cancel").unwrap();
    preimage.capability_id = CapabilityIdV1::parse(OPERATION_CANCEL_CAPABILITY_ID).unwrap();
    preimage.idempotency_key = RequestIdempotencyKeyV1::parse("provider-service-cancel").unwrap();
    preimage.issued_at = UnixTimeV1::new(69).unwrap();
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

struct SessionRead(SessionLeaseBody);

impl SessionLeaseCustodyPort for SessionRead {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("cancellation recovery does not renew its historical session")
    }

    fn read(
        &self,
        query: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        assert_eq!(query.store(), self.0.store());
        assert_eq!(query.digest(), self.0.digest());
        Ok(Some(self.0.clone()))
    }
}

fn session_read(
    parents: &crate::local_operation_start_fixture::RecoveryParents<'_>,
) -> SessionRead {
    let session = parents.session.lease();
    SessionRead(SessionLeaseBody::new(
        store_coordinate(),
        session.session_admission_lease_digest().clone(),
        session.schema_id().clone(),
        session.schema_sha256().clone(),
        ControlRecordBytes::new(
            pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(session))
                .unwrap()
                .into_bytes(),
        )
        .unwrap(),
    ))
}

fn recovery_resolver(
    parents: &crate::local_operation_start_fixture::RecoveryParents<'_>,
) -> RecoveryResolver {
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(parents.current.generation.compiled_generation);
    resolver.insert_active(parents.current.generation.active_head);
    resolver.insert_fence(parents.current.generation.activation_fence);
    resolver.insert_effective_profile(parents.current.generation.effective_profile);
    resolver
}

#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn request_composed_provider_cancellation(
    service: &mut ResidentOperationService<'_, ProvingAuthority>,
    operation: &crate::resident::IssuedLocalOperation,
    original: &CommonRequestEnvelopeV1,
    parents: &crate::local_operation_start_fixture::RecoveryParents<'_>,
    parent_request: Option<&(pale_ports::ControlRecordCoordinate, CommonRequestEnvelopeV1)>,
    initial_candidate: &OperationCustodyCandidate,
    dispatched: (&ResidentRecordRefV1, &OperationObservationV1),
    dispatched_checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    allow: bool,
) -> Result<ResidentOperationCancellationOutcome, ResidentOperationCancellationFailure> {
    let handle_record = initial_candidate
        .record(pale_ports::OperationCustodyRecordKind::OperationHandle)
        .expect("initial custody retains the operation handle");
    let request_record = initial_candidate
        .record(pale_ports::OperationCustodyRecordKind::CommonRequest)
        .expect("initial custody retains the original request");
    let expected_handle = handle_record.coordinate().clone();
    let (handle_ref, handle) = operation.handle();
    assert_eq!(handle_record.coordinate().record_ref(), handle_ref);
    assert!(matches!(
        dispatched.1.state(),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Dispatched { .. }
        }
    ));

    let store = store_coordinate();
    let profile = crate::local_operation_start_fixture::profile();
    let resolver = recovery_resolver(parents);
    let sessions = session_read(parents);
    let checkpoint_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
        authority_ref: dispatched_checkpoint.1.checkpoint_signing_authority_ref(),
        authority_sha256: dispatched_checkpoint
            .1
            .checkpoint_signing_authority_sha256(),
        principal_id: dispatched_checkpoint.1.issuer_principal_id(),
        control_store_id: dispatched_checkpoint.1.control_store_id(),
        control_incarnation_id: dispatched_checkpoint.1.control_incarnation_id(),
        checkpoint_domain_id: dispatched_checkpoint.1.checkpoint_domain_id(),
        checkpoint_profile_ref: dispatched_checkpoint.1.checkpoint_profile_ref(),
        checkpoint_profile_sha256: dispatched_checkpoint.1.checkpoint_profile_sha256(),
        subject_key_id: dispatched_checkpoint.1.issuer_key_id(),
        subject_key_epoch: *dispatched_checkpoint.1.issuer_key_epoch(),
    };
    let historical = original.to_preimage();
    let request_context = CommonRequestAdmissionContextV1 {
        expected_schema_sha256: &historical.envelope_schema_sha256,
        expected_body_schema_id: &historical.body.schema_id,
        expected_body_schema_sha256: &historical.body.schema_sha256,
        caller: &historical.caller,
        signing_authority: &historical.signing_authority_binding,
        route: &historical.route,
        session_admission: &historical.session_admission,
        operation_target: historical.operation_target.as_ref(),
        execution_owner_fence: None,
        federation_claim: None,
        workspace_id: &historical.workspace_id,
        capability_id: &historical.capability_id,
        operation: &historical.operation,
        scope: &historical.scope,
        risk_class: &historical.risk_class,
        resource_constraints: &historical.resource_constraints,
        expected_precondition: None,
        idempotency_key: &historical.idempotency_key,
        issued_at: historical.issued_at,
        expires_at: historical.expires_at,
        generation: historical.generation.as_ref(),
        fence_token: historical.fence_token.as_ref(),
    };
    let mut admit_request = |reference: &ResidentRecordRefV1, wire: CommonRequestEnvelopeWireV1| {
        if let Some((coordinate, request)) = parent_request {
            if coordinate.record_ref() == reference {
                return Ok(request.clone());
            }
        }
        assert_eq!(reference, request_record.coordinate().record_ref());
        CommonRequestEnvelopeV1::admit_contextual_wire(wire, &request_context, |_, _, _| Ok(()))
    };
    let mut admit_handle_authority =
        |_: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
            if parent_request.is_none() {
                assert!(parent.is_none());
            }
            Ok(())
        };
    let mut verify_handle_signature =
        |_: &[u8],
         preimage: &OperationHandlePreimageV1,
         digest: &OperationHandleDigest,
         _: &CanonicalSignatureV1| {
            assert_eq!(
                digest,
                &OperationHandleDigest::hash_canonical(preimage).unwrap()
            );
            Ok(())
        };
    let mut admit_checkpoint = |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
        ControlStateCheckpointV1::admit_signed_wire(
            wire,
            &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
            &checkpoint_authority,
            |_, _, _, _| Ok(()),
        )
    };
    let mut resolve_observation_signer =
        |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| Ok(parents.signing.clone());
    let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
    let mut admit_rotation = |_: &ResidentRecordRefV1,
                              _: &ArtifactDigest,
                              _: &ResidentRecordRefV1,
                              _: &ArtifactDigest| { Ok(()) };
    let mut verify_observation_signature =
        |_: &[u8],
         _: &OperationObservationPreimageV1,
         _: &OperationObservationDigest,
         _: &CanonicalSignatureV1| { Ok(()) };
    let mut verify_checkpoint_signature =
        |_: &[u8],
         _: &ControlStateCheckpointPreimageV1,
         _: &ControlStateCheckpointDigest,
         _: &CanonicalSignatureV1| { Ok(()) };
    let mut authorize = |context: OperationControlAuthorizationContext<'_>| {
        assert_eq!(
            context.request.capability_id().as_str(),
            OPERATION_CANCEL_CAPABILITY_ID
        );
        assert_eq!(context.original_request, original);
        assert_eq!(context.operation.0, handle_ref);
        assert_eq!(context.observation.0, dispatched.0);
        if allow {
            Ok(())
        } else {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "operation cancellation",
                invariant: "synthetic current authority denied cancellation",
            })
        }
    };
    let cancellation = cancellation_request(original, handle);
    let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let recovery = OperationControlRecoveryInput {
        operation: LocalOperationHandleRecoveryInput {
            context: LocalOperationRecoveryContext {
                store: &store,
                profile: &profile,
                handle_signing_authority: parents.signing,
                checkpoint_authority: &checkpoint_authority,
                historical: &resolver,
                maximum_record_bytes: 1024 * 1024,
                maximum_operation_lease_bytes: 1024 * 1024,
                maximum_session_lease_bytes: 1024 * 1024,
                membership_limits: AccumulatorMembershipLimits::new(8, 10_000, 100_000).unwrap(),
                observation_limits: OperationObservationRecoveryLimits::new(
                    8,
                    1024 * 1024,
                    1024 * 1024,
                )
                .unwrap(),
                checkpoint_limits: OperationCheckpointAncestryLimits::new(8, 1024 * 1024, 8)
                    .unwrap(),
            },
            expected_handle: &expected_handle,
            maximum_parent_depth: 1,
        },
        observation: OperationObservationSelection::SelectedHead,
        observation_limits: OperationObservationRecoveryLimits::new(8, 1024 * 1024, 1024 * 1024)
            .unwrap(),
        checkpoint_limits: OperationCheckpointAncestryLimits::new(8, 1024 * 1024, 8).unwrap(),
        membership_limits: AccumulatorMembershipLimits::new(8, 10_000, 100_000).unwrap(),
    };
    let mut append_rotation = |_: &ResidentRecordRefV1,
                               _: &ArtifactDigest,
                               _: &ResidentRecordRefV1,
                               _: &ArtifactDigest| { Ok(()) };
    let mut sign_observation =
        |_: &[u8], _: &OperationObservationPreimageV1, _: &OperationObservationDigest| {
            Ok(signature())
        };
    let mut sign_checkpoint =
        |_: &[u8], _: &ControlStateCheckpointPreimageV1, _: &ControlStateCheckpointDigest| {
            Ok(signature())
        };
    service.request_cancellation(
        ResidentOperationCancellationInput {
            recovery,
            request: &cancellation,
            authorized_at: UnixTimeV1::new(70).unwrap(),
            request_ref: record_ref("provider-service/composed-cancellation-request"),
            observation_ref: record_ref("provider-service/composed-cancellation-requested"),
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &observation_schema,
                signing_authority: parents.signing,
            },
            checkpoint: OperationCheckpointInput {
                reference: record_ref("provider-service/checkpoint-composed-cancellation"),
                id: ControlCheckpointIdV1::parse("provider-service-composed-cancellation").unwrap(),
            },
            checkpoint_issued_at: UnixTimeV1::new(70).unwrap(),
            checkpoint_schema: dispatched_checkpoint.1.schema_sha256(),
            checkpoint_authority: &checkpoint_authority,
            accumulator_limits: AccumulatorPreparationLimits::new(1, 1024, 1024, 512, 65_536)
                .unwrap(),
        },
        None,
        &sessions,
        ResidentOperationCancellationAdmission {
            recovery: OperationControlRecoveryAdmission {
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
            authorize: &mut authorize,
            resolve_store: &Store,
            admit_rotation: &mut append_rotation,
            sign_observation: &mut sign_observation,
            sign_checkpoint: &mut sign_checkpoint,
        },
    )
}

#[derive(Default)]
struct BlockingProviderState {
    started: bool,
    release: bool,
}

struct BlockingProvider {
    calls: AtomicUsize,
    panic_after_release: bool,
    state: Mutex<BlockingProviderState>,
    changed: Condvar,
}

impl BlockingProvider {
    fn new(panic_after_release: bool) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            panic_after_release,
            state: Mutex::new(BlockingProviderState::default()),
            changed: Condvar::new(),
        }
    }

    fn wait_until_started(&self) {
        let guard = self.state.lock().expect("provider state lock");
        let (guard, timeout) = self
            .changed
            .wait_timeout_while(guard, Duration::from_secs(2), |state| !state.started)
            .expect("provider start wait");
        assert!(guard.started, "provider task did not start");
        assert!(!timeout.timed_out(), "provider start wait elapsed");
    }

    fn release(&self) {
        let mut state = self.state.lock().expect("provider state lock");
        state.release = true;
        self.changed.notify_all();
    }
}

impl ProviderInvocationPort for BlockingProvider {
    fn invoke(&self, _: ProviderInvocationRequest<'_>) -> ProviderInvocationObservation {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut state = self.state.lock().expect("provider state lock");
        state.started = true;
        self.changed.notify_all();
        while !state.release {
            state = self.changed.wait(state).expect("provider release wait");
        }
        assert!(!self.panic_after_release, "synthetic provider panic");
        ProviderInvocationObservation::Unavailable(
            ProviderInvocationUnavailability::EvidenceUnknown,
        )
    }
}

type ProviderTaskOutput = Result<
    ProviderInvocationObservation,
    crate::provider_invocation::ProviderInvocationAdmissionError,
>;

struct BlockingProviderExecutor {
    provider: Arc<BlockingProvider>,
    sender: mpsc::Sender<
        ResidentOperationTaskCompletion<
            crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
    >,
    receiver: mpsc::Receiver<
        ResidentOperationTaskCompletion<
            crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
    >,
}

impl BlockingProviderExecutor {
    fn new(provider: Arc<BlockingProvider>) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            provider,
            sender,
            receiver,
        }
    }
}

impl
    ResidentOperationTaskExecutor<
        crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
        ProviderTaskOutput,
    > for BlockingProviderExecutor
{
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<
            crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
        >,
    ) -> Result<
        (),
        ResidentOperationTask<crate::resident::operation_dispatch::OwnedProviderInvocationRequest>,
    > {
        let provider = Arc::clone(&self.provider);
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                execute_provider_task(&task, provider.as_ref())
            }));
            let completion = match outcome {
                Ok(output) => ResidentOperationTaskCompletion::Completed { task, output },
                Err(_) => task.into_lost(),
            };
            let _ = sender.send(completion);
        });
        Ok(())
    }

    fn try_completion(
        &mut self,
    ) -> Option<
        ResidentOperationTaskCompletion<
            crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
    > {
        self.receiver.try_recv().ok()
    }
}

#[derive(Default)]
struct DeferredProviderExecutor {
    accept_calls: usize,
    refuse_next: bool,
    accepted: VecDeque<
        ResidentOperationTask<crate::resident::operation_dispatch::OwnedProviderInvocationRequest>,
    >,
    completions: VecDeque<
        ResidentOperationTaskCompletion<
            crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
    >,
}

impl DeferredProviderExecutor {
    fn invoke_next(&mut self, provider: &impl ProviderInvocationPort) {
        let task = self
            .accepted
            .pop_front()
            .expect("one accepted provider task");
        let output = execute_provider_task(&task, provider);
        self.completions
            .push_back(ResidentOperationTaskCompletion::Completed { task, output });
    }

    fn lose_next(&mut self) {
        let task = self
            .accepted
            .pop_front()
            .expect("one accepted provider task");
        self.completions.push_back(task.into_lost());
    }
}

impl
    ResidentOperationTaskExecutor<
        crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
        ProviderTaskOutput,
    > for DeferredProviderExecutor
{
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<
            crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
        >,
    ) -> Result<
        (),
        ResidentOperationTask<crate::resident::operation_dispatch::OwnedProviderInvocationRequest>,
    > {
        self.accept_calls += 1;
        if std::mem::take(&mut self.refuse_next) {
            return Err(task);
        }
        self.accepted.push_back(task);
        Ok(())
    }

    fn try_completion(
        &mut self,
    ) -> Option<
        ResidentOperationTaskCompletion<
            crate::resident::operation_dispatch::OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
    > {
        self.completions.pop_front()
    }
}

impl OperationObservationAuthorityStoreResolver for Store {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        Ok(store_coordinate())
    }
}

struct AmbiguousFinalAnchor<'a> {
    inner: &'a mut dyn AntiRollbackAnchorPort,
    observations: usize,
}

impl AntiRollbackAnchorPort for AmbiguousFinalAnchor<'_> {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.inner.read(key)
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.inner.compare_and_advance(transition)?;
        Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::OutcomeUnknown,
        ))
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.observations += 1;
        if self.observations == 1 {
            return self.inner.observe(transition);
        }
        Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::Unavailable,
        ))
    }
}

#[test]
fn provider_resolution_preparation_and_dispatch_enter_custody_in_order() {
    with_selection_custody_context(
        LocalStartContext::SelfContained,
        |selection, selected, admission, parents, initial_checkpoint, initial_record| {
            let (handle_ref, handle) = selected.operation.handle();
            let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let initial = issue_observation(
                parents.signing,
                handle_ref,
                handle,
                schema.clone(),
                OperationObservationRevisionV1::new(1).unwrap(),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                60,
            );
            assert_eq!(
                initial_record.canonical_record_jcs().as_slice(),
                canon::to_canonical_string(&initial).unwrap().as_bytes()
            );
            let observation = OperationObservationAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &schema,
                    signing_authority: parents.signing,
                },
                handle_ref,
                handle,
                predecessor: Some((initial_record.coordinate().record_ref(), &initial)),
            };
            let signs = Cell::new(0);
            let resolution = prepare_provider_resolution(
                ProviderOperationInput {
                    context: OperationAttemptContext {
                        operation: selected.operation,
                        request: selected.request,
                        session: parents.session,
                        current: parents.current,
                        lease: selected.lease,
                        context_binding: None,
                        generation: selected.generation,
                        selection_ref: record_ref("provider-dispatch/selection"),
                        maximum_selection_bytes: 65_536,
                        observation_ref: record_ref("provider-dispatch/preparation"),
                        observation,
                    },
                    invocation: selection.provider_request(selected).unwrap(),
                    result_admission: admission,
                    resolution_ref: record_ref("provider-dispatch/resolution"),
                },
                &mut ProvingAuthority,
                |_, _, _, _| Ok(()),
                |_, _, _| {
                    signs.set(signs.get() + 1);
                    Ok(signature())
                },
            )
            .unwrap();
            assert_eq!(signs.get(), 1);
            assert!(matches!(
                resolution.observation().1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ProviderResolved { provider_binding }
                } if provider_binding == selection.provider().unwrap()
            ));
            let resolution_append = resolution.prepare_append(&Store).unwrap();
            assert_eq!(resolution_append.associated_record(), None);
            let (resolution_done, resolution_checkpoint) = custody::complete(
                resolution_append,
                selected.request,
                (&record_ref("operation/checkpoint-1"), initial_checkpoint),
                OperationCustodyDatabaseEvidence::New,
                61,
            );

            let preparation_current = parents.current.at_time(UnixTimeV1::new(62).unwrap());
            let attempt = resolution
                .prepare_attempt(
                    &resolution_done,
                    &preparation_current,
                    &Store,
                    &mut ProvingAuthority,
                    |_, _, _, _| Ok(()),
                    |_, _, _| {
                        signs.set(signs.get() + 1);
                        Ok(signature())
                    },
                )
                .unwrap();
            assert_eq!(signs.get(), 2);
            assert!(matches!(
                attempt.observation().1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::DispatchPrepared { attempt_id, .. }
                } if attempt_id == selection.attempt_id()
            ));
            let attempt_append = attempt.prepare_append(&Store).unwrap();
            assert!(matches!(
                attempt_append.associated_record(),
                Some(OperationCustodyRecord::AttemptSelection(_))
            ));
            let (attempt_done, attempt_checkpoint) = custody::complete(
                attempt_append,
                selected.request,
                (
                    resolution_done
                        .candidate()
                        .checkpoint()
                        .successor()
                        .coordinate()
                        .record_ref(),
                    &resolution_checkpoint,
                ),
                OperationCustodyDatabaseEvidence::New,
                62,
            );

            let dispatch_current = parents.current.at_time(UnixTimeV1::new(63).unwrap());
            let dispatch = attempt
                .prepare_dispatch(
                    ProviderOperationDispatchContext {
                        completed: &attempt_done,
                        current: &dispatch_current,
                        signing: observation.signed,
                        observation_ref: record_ref("provider-dispatch/dispatched"),
                        resolve_store: &Store,
                    },
                    &mut ProvingAuthority,
                    |_, _, _, _| Ok(()),
                    |_, _, _| {
                        signs.set(signs.get() + 1);
                        Ok(signature())
                    },
                )
                .unwrap();
            assert_eq!(signs.get(), 3);
            assert!(matches!(
                dispatch.observation().1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Dispatched { attempt_id, .. }
                } if attempt_id == selection.attempt_id()
            ));
            assert_eq!(
                dispatch.provider_request().unwrap().provider_binding(),
                selection
                    .provider_request(selected)
                    .unwrap()
                    .provider_binding()
            );
            let dispatch_append = dispatch.prepare_append(&Store).unwrap();
            assert_eq!(dispatch_append.associated_record(), None);
            let (dispatch_done, _) = custody::complete(
                dispatch_append,
                selected.request,
                (
                    attempt_done
                        .candidate()
                        .checkpoint()
                        .successor()
                        .coordinate()
                        .record_ref(),
                    &attempt_checkpoint,
                ),
                OperationCustodyDatabaseEvidence::New,
                63,
            );
            assert_eq!(
                dispatch_done.database_evidence(),
                OperationCustodyDatabaseEvidence::New
            );
        },
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderServiceOutcome {
    AggregateSaturated,
    AdmissionRejected,
    ReconciliationAlias,
    Succeeded,
    ComposedSucceeded,
    ComposedRefusalContinuation,
    ComposedNonRetryableRefusals,
    ComposedCancellation,
    NotificationOnly,
    ParallelServiceIsolation,
    Failed,
    Rejected,
    ResultSignatureRejected,
    FailureSignatureRejected,
    AssociationRejected,
    Unavailable,
    AnchorUnknown,
    RecordingRetry,
    ConcurrentSuccess,
    ConcurrentFailure,
    ConcurrentUncertainty,
    ComposedConcurrentSuccess,
    ComposedConcurrentFailure,
    ComposedConcurrentUncertainty,
    ConcurrentHeadTwice,
    ColdRestart,
    LostAfterAcceptance,
}

impl ProviderServiceOutcome {
    const fn has_concurrent_head(self) -> bool {
        matches!(
            self,
            Self::ConcurrentSuccess
                | Self::ConcurrentFailure
                | Self::ConcurrentUncertainty
                | Self::ConcurrentHeadTwice
        )
    }

    const fn is_composed_concurrent(self) -> bool {
        matches!(
            self,
            Self::ComposedConcurrentSuccess
                | Self::ComposedConcurrentFailure
                | Self::ComposedConcurrentUncertainty
        )
    }

    const fn has_cancellation_notification(self) -> bool {
        self.has_concurrent_head()
            || self.is_composed_concurrent()
            || matches!(self, Self::NotificationOnly)
    }

    const fn cancellation_state(self) -> ResidentOperationTaskCancellationState {
        if self.has_cancellation_notification() {
            ResidentOperationTaskCancellationState::Notified
        } else {
            ResidentOperationTaskCancellationState::Unnotified
        }
    }
}

/// Phase-specific physical outcomes for direct A2A task-owner tests.
pub(in crate::resident) struct A2aTaskOwnerCustodyModel<'a> {
    pub(in crate::resident) clock_times: Vec<u64>,
    pub(in crate::resident) expected_preparations: usize,
    pub(in crate::resident) expected_commits: usize,
    pub(in crate::resident) expected_read_backs: usize,
    pub(in crate::resident) expected_anchor_advances: usize,
    pub(in crate::resident) expected_anchor_observations: usize,
    pub(in crate::resident) unknown_exact_commits: &'a [usize],
    pub(in crate::resident) unavailable_read_backs: &'a [usize],
    pub(in crate::resident) unavailable_anchor_advances: &'a [usize],
    pub(in crate::resident) unknown_exact_anchor_advances: &'a [usize],
    pub(in crate::resident) unknown_failed_observation_anchor_advances: &'a [usize],
    pub(in crate::resident) later_anchor_observations: Vec<custody::ServiceAnchorLaterObservation>,
    pub(in crate::resident) fail_a2a_preflight_once: bool,
}

/// Retain one real Delegate provider dispatch before handing it to an A2A owner test.
#[allow(clippy::too_many_lines)]
pub(in crate::resident) fn with_retained_delegate_dispatch<R>(
    model: A2aTaskOwnerCustodyModel<'_>,
    run: impl FnOnce(
        RetainedProviderOperationDispatch<'_>,
        &AmbientSessionAdmissionContext<'_>,
        SignedResidentAdmissionContextV1<'_>,
        &SchemaDigest,
        &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        &dyn WallClockSourcePort,
        &dyn OperationObservationAuthorityStoreResolver,
        &mut ProvingAuthority,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
        (pale_ports::ControlRecordCoordinate, CommonRequestEnvelopeV1),
        AdmittedCompiledGeneration,
        &OperationCustodyCandidate,
    ) -> R,
) -> R {
    with_retained_delegate_dispatch_named(model, "a2a-task-owner", run)
}

pub(in crate::resident) fn with_retained_delegate_dispatch_named<R>(
    model: A2aTaskOwnerCustodyModel<'_>,
    label: &str,
    run: impl FnOnce(
        RetainedProviderOperationDispatch<'_>,
        &AmbientSessionAdmissionContext<'_>,
        SignedResidentAdmissionContextV1<'_>,
        &SchemaDigest,
        &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        &dyn WallClockSourcePort,
        &dyn OperationObservationAuthorityStoreResolver,
        &mut ProvingAuthority,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
        (pale_ports::ControlRecordCoordinate, CommonRequestEnvelopeV1),
        AdmittedCompiledGeneration,
        &OperationCustodyCandidate,
    ) -> R,
) -> R {
    with_delegate_dispatch_input_named(
        model,
        label,
        |dispatch,
         _recovered,
         _completed_context,
         checkpoint_schema,
         checkpoint_authority,
         clock,
         resolve_store,
         authority,
         custody,
         anchor,
         parents,
         parent_request,
         restart_generation,
         initial_candidate| {
            let signing = dispatch.signing;
            let retained_result =
                ResidentLocalOperationCoordinator::new(authority, custody, anchor)
                    .retain_provider_dispatch(
                        dispatch,
                        |_, _, _, _| Ok(()),
                        |_, _, _| Ok(signature()),
                        |_, _, _| Ok(signature()),
                    );
            let retained = match retained_result {
                Ok(retained) => retained,
                Err(ProviderOperationRetentionFailure::Admission(reason)) => {
                    panic!("Delegate provider dispatch admission: {reason:?}")
                }
                Err(other) => panic!("Delegate provider dispatch retention: {other:?}"),
            };
            let dispatch_current = parents
                .current
                .at_time(*retained.observation().1.observed_at());
            run(
                retained,
                &dispatch_current,
                signing,
                checkpoint_schema,
                checkpoint_authority,
                clock,
                resolve_store,
                authority,
                custody,
                anchor,
                parents,
                parent_request,
                restart_generation,
                initial_candidate,
            )
        },
    )
}

/// Lend one exact pairwise Delegate provider input before durable dispatch retention.
#[allow(clippy::too_many_lines, clippy::type_complexity)]
pub(in crate::resident) fn with_delegate_dispatch_input_named<R>(
    model: A2aTaskOwnerCustodyModel<'_>,
    label: &str,
    run: impl FnOnce(
        ProviderOperationDispatchInput<'_, '_>,
        &crate::resident::process_host::restart::ResidentProcessRecoveredOperationResolution,
        &crate::resident::process_host::operation_resolution::CompletedResidentProcessOperationContext<'_>,
        &SchemaDigest,
        &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        &dyn WallClockSourcePort,
        &dyn OperationObservationAuthorityStoreResolver,
        &mut ProvingAuthority,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
        (pale_ports::ControlRecordCoordinate, CommonRequestEnvelopeV1),
        AdmittedCompiledGeneration,
        &OperationCustodyCandidate,
    ) -> R,
) -> R {
    let (mut generations, profile) =
        crate::resident::operation_observation_custody::tests::local_start::
            host_source_generation_copies(2);
    let restart_generation = generations.pop().expect("restart generation copy");
    let generation = generations.pop().expect("selected generation copy");
    crate::resident::consultation::tests::with_issued_delegate_child_named(
        &generation,
        &profile,
        label,
        |child,
         _parent,
         parent_candidate,
         _parent_checkpoint,
         lease_custody,
         generation,
         parents| {
            let binding = child.request.local_request_binding().unwrap();
            let attempt = CapabilityAttemptIdV1::parse("attempt-1").unwrap();
            let kind = BoundedResidentTextV1::parse("host_compute").unwrap();
            let version = BoundedResidentTextV1::parse("1").unwrap();
            let contracts = crate::session_admission_fixture::local_start_contract_set();
            let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
            let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
            let admission = super::admission(
                CapabilitySuccessAdmissionContextV1 {
                    result: CapabilityResultAdmissionContextV1 {
                        expected_schema_sha256: &result_schema,
                        expected_request_binding: &binding,
                        expected_attempt_id: &attempt,
                        expected_implementation_kind: &kind,
                        expected_implementation_version: &version,
                        signing_authority: parents.signing,
                    },
                    contract_set_ref: binding.contract_set_ref(),
                    contract_set: &contracts,
                },
                &failure_schema,
            );
            let invocation = ProviderInvocationRequest::new(
                ProviderInvocationCoordinate::new(
                    generation.record().compiled_generation_digest(),
                    generation.record().instance_digest(),
                    parents.lease.operation(),
                    &binding,
                    &attempt,
                ),
                generation
                    .provider_binding("provider-binding-claude-code")
                    .unwrap(),
                &kind,
                &version,
            )
            .unwrap();
            let selected = AttemptSelectionContext {
                operation: &child.operation,
                request_ref: &child.request_ref,
                request: &child.request,
                session: parents.session.lease(),
                lease: parents.lease,
                generation,
            };
            let selection = AttemptSelection::capture(AttemptSelectionInput {
                parents: selected,
                invocation,
                admission,
            })
            .unwrap();
            let initial_checkpoint = &child.checkpoint;
            let initial_record = &child.initial_record;
            let initial_candidate = &child.initial_candidate;
            let (handle_ref, handle) = selected.operation.handle();
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let initial = issue_observation(
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
                initial_record.canonical_record_jcs().as_slice(),
                canon::to_canonical_string(&initial).unwrap().as_bytes()
            );
            let initial_admission = OperationObservationAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority: parents.signing,
                },
                handle_ref,
                handle,
                predecessor: Some((initial_record.coordinate().record_ref(), &initial)),
            };
            let mut checkpoint_fields = checkpoint_preimage(
                initial_checkpoint.domain_sequence().get() + 1,
                Some((
                    child.checkpoint_ref.clone(),
                    initial_checkpoint
                        .canonical_control_checkpoint_sha256()
                        .clone(),
                )),
            );
            checkpoint_fields.checkpoint_domain_id =
                initial_checkpoint.checkpoint_domain_id().clone();
            checkpoint_fields.checkpoint_profile_ref =
                initial_checkpoint.checkpoint_profile_ref().clone();
            checkpoint_fields.checkpoint_profile_sha256 =
                initial_checkpoint.checkpoint_profile_sha256().clone();
            let checkpoint_signing = checkpoint_authority(&checkpoint_fields);
            let clock = Clock {
                times: model.clock_times,
                next: AtomicUsize::new(0),
            };
            let checkpoint_input = |suffix: &str| OperationCheckpointInput {
                reference: record_ref(&format!("{label}/checkpoint-{suffix}")),
                id: ControlCheckpointIdV1::parse(format!("{label}-checkpoint-{suffix}")).unwrap(),
            };
            let handle_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    handle_ref,
                    handle.canonical_operation_handle_sha256().as_str(),
                )
                .unwrap(),
                handle,
            )
            .unwrap();
            let evidence = ImmutableRecordReferenceV1 {
                record_ref: record_ref(&format!("{label}/context")),
                record_sha256: artifact_digest('8'),
            };
            let context_binding = prepare_operation_context_binding(
                OperationContextBindingInput {
                    operation: selected.operation,
                    request: selected.request,
                    session: parents.session,
                    current: parents.current,
                    lease: selected.lease,
                    evidence: evidence.clone(),
                    observation_ref: record_ref(&format!("{label}/context-bound")),
                    observation: &initial_admission,
                },
                &mut ProvingAuthority,
                |actual, obligations| {
                    assert_eq!(actual, &evidence);
                    assert_eq!(obligations, selected.lease.evidence_obligations());
                    Ok(())
                },
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
            )
            .expect("Delegate context binding preparation");
            custody::with_service_custody_faults(
                initial_checkpoint,
                Some(initial_candidate),
                vec![parent_candidate],
                None,
                vec![lease_custody.parent, lease_custody.child],
                model.expected_preparations,
                model.expected_commits,
                model.expected_read_backs,
                model.expected_anchor_advances,
                model.expected_anchor_observations,
                &[],
                model.unknown_exact_commits,
                model.unavailable_read_backs,
                model.unavailable_anchor_advances,
                model.unknown_exact_anchor_advances,
                model.unknown_failed_observation_anchor_advances,
                model.later_anchor_observations,
                model.fail_a2a_preflight_once,
                Some(handle_record),
                Default::default(),
                |custody, anchor| {
                    let mut authority = ProvingAuthority;
                    let (context_done, context_checkpoint) =
                        crate::resident::operation_dispatch::custody::append_checkpoint(
                            context_binding.prepare_append(&Store).unwrap(),
                            selected.operation,
                            (&child.checkpoint_ref, initial_checkpoint),
                            checkpoint_input("context-bound"),
                            UnixTimeV1::new(63).unwrap(),
                            &checkpoint_fields.schema_sha256,
                            &checkpoint_signing,
                            AccumulatorPreparationLimits::new(1, 1024, 1024, 512, 65_536).unwrap(),
                            custody,
                            anchor,
                            |_, _, _| Ok(signature()),
                        )
                        .expect("Delegate context binding custody");
                    let context_binding = context_binding
                        .finish(&context_done, &Store)
                        .expect("Delegate context binding completion");
                    let completed_context = crate::resident::process_host::operation_resolution::CompletedResidentProcessOperationContext::fixture(
                        context_binding,
                        (context_checkpoint.0.clone(), context_checkpoint.1.clone()),
                        context_done,
                    );
                    let observation = OperationObservationAdmissionContextV1 {
                        signed: initial_admission.signed,
                        handle_ref,
                        handle,
                        predecessor: Some(completed_context.binding().observation()),
                    };
                    let dispatch = ProviderOperationDispatchInput {
                        operation: ProviderOperationInput {
                            context: OperationAttemptContext {
                                operation: selected.operation,
                                request: selected.request,
                                session: parents.session,
                                current: parents.current,
                                lease: selected.lease,
                                context_binding: Some(completed_context.binding()),
                                generation: selected.generation,
                                selection_ref: record_ref(&format!("{label}/selection")),
                                maximum_selection_bytes: 65_536,
                                observation_ref: record_ref(&format!("{label}/preparation")),
                                observation,
                            },
                            invocation: selection.provider_request(selected).unwrap(),
                            result_admission: admission,
                            resolution_ref: record_ref(&format!("{label}/resolution")),
                        },
                        checkpoint: (&context_checkpoint.0, &context_checkpoint.1),
                        checkpoint_schema: &checkpoint_fields.schema_sha256,
                        checkpoint_authority: &checkpoint_signing,
                        signing: observation.signed,
                        resolution_checkpoint: checkpoint_input("resolved"),
                        preparation_checkpoint: checkpoint_input("prepared"),
                        dispatch_checkpoint: checkpoint_input("dispatched"),
                        dispatch_ref: record_ref(&format!("{label}/dispatched")),
                        clock: &clock,
                        resolve_store: &Store,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            1, 1024, 1024, 512, 65_536,
                        )
                        .unwrap(),
                    };
                    let recovered =
                        crate::resident::process_host::tests::pairwise_resolution_fixture(
                            selected.operation,
                            selected.request.clone(),
                            initial_candidate,
                            initial_checkpoint.clone(),
                            parents.signing,
                            selected
                                .generation
                                .record()
                                .compiled_generation_digest()
                                .clone(),
                        );
                    run(
                        dispatch,
                        &recovered,
                        &completed_context,
                        &checkpoint_fields.schema_sha256,
                        &checkpoint_signing,
                        &clock,
                        &Store,
                        &mut authority,
                        custody,
                        anchor,
                        parents,
                        child.parent_request.clone(),
                        restart_generation,
                        initial_candidate,
                    )
                },
            )
        },
    )
}

fn provider_service_case(
    stale_release: bool,
    outcome: ProviderServiceOutcome,
    port_slot: &mut Option<provider::ResultPort>,
) -> u32 {
    with_selection_recovery_context(
        LocalStartContext::SelfContained,
        |selection,
         selected,
         admission,
         parents,
         initial_checkpoint,
         initial_record,
         initial_candidate,
         _restart_generation| {
            let (handle_ref, handle) = selected.operation.handle();
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let initial = issue_observation(
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
                initial_record.canonical_record_jcs().as_slice(),
                canon::to_canonical_string(&initial).unwrap().as_bytes()
            );
            let observation = OperationObservationAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority: parents.signing,
                },
                handle_ref,
                handle,
                predecessor: Some((initial_record.coordinate().record_ref(), &initial)),
            };
            let mut checkpoint_fields = checkpoint_preimage(2, None);
            checkpoint_fields.checkpoint_domain_id =
                initial_checkpoint.checkpoint_domain_id().clone();
            checkpoint_fields.checkpoint_profile_ref =
                initial_checkpoint.checkpoint_profile_ref().clone();
            checkpoint_fields.checkpoint_profile_sha256 =
                initial_checkpoint.checkpoint_profile_sha256().clone();
            let checkpoint_signing = checkpoint_authority(&checkpoint_fields);
            let clock = Clock {
                times: if stale_release {
                    vec![61, 62, 63, 10_000]
                } else {
                    vec![61, 62, 63, 64, 65, 66, 71, 72]
                },
                next: AtomicUsize::new(0),
            };
            let observation_signs = Cell::new(0);
            let checkpoint_signs = Cell::new(0);
            let checkpoint_input = |suffix: &str| OperationCheckpointInput {
                reference: record_ref(&format!("provider-service/checkpoint-{suffix}")),
                id: ControlCheckpointIdV1::parse(format!("provider-service-checkpoint-{suffix}"))
                    .unwrap(),
            };
            let admitted_success = provider::success(
                selected.request,
                selected.generation,
                parents.signing,
                selection.attempt_id(),
                "valid",
            );
            let success_wire =
                serde_json::from_value(serde_json::to_value(admitted_success.result()).unwrap())
                    .unwrap();
            port_slot.get_or_insert_with(|| match outcome {
                ProviderServiceOutcome::AggregateSaturated
                | ProviderServiceOutcome::AdmissionRejected
                | ProviderServiceOutcome::ReconciliationAlias
                | ProviderServiceOutcome::Succeeded
                | ProviderServiceOutcome::ComposedSucceeded
                | ProviderServiceOutcome::ComposedRefusalContinuation
                | ProviderServiceOutcome::ComposedNonRetryableRefusals
                | ProviderServiceOutcome::ComposedCancellation
                | ProviderServiceOutcome::NotificationOnly
                | ProviderServiceOutcome::ParallelServiceIsolation
                | ProviderServiceOutcome::AnchorUnknown
                | ProviderServiceOutcome::RecordingRetry
                | ProviderServiceOutcome::ConcurrentSuccess
                | ProviderServiceOutcome::ComposedConcurrentSuccess
                | ProviderServiceOutcome::ConcurrentHeadTwice
                | ProviderServiceOutcome::ColdRestart
                | ProviderServiceOutcome::LostAfterAcceptance
                | ProviderServiceOutcome::ResultSignatureRejected => {
                    provider::ResultPort::success(ProviderInvocationSuccessWire::new(
                        admitted_success.coordinate().clone(),
                        success_wire,
                    ))
                }
                ProviderServiceOutcome::Failed
                | ProviderServiceOutcome::FailureSignatureRejected
                | ProviderServiceOutcome::AssociationRejected
                | ProviderServiceOutcome::ConcurrentFailure
                | ProviderServiceOutcome::ComposedConcurrentFailure => {
                    provider::ResultPort::failure(provider::safe_failure(
                        selected.request,
                        parents.signing,
                        selection.attempt_id(),
                        admitted_success.coordinate().clone(),
                    ))
                }
                ProviderServiceOutcome::Rejected => {
                    let rejected = provider::success(
                        selected.request,
                        selected.generation,
                        parents.signing,
                        selection.attempt_id(),
                        "wrong-generation",
                    );
                    let rejected_wire =
                        serde_json::from_value(serde_json::to_value(rejected.result()).unwrap())
                            .unwrap();
                    provider::ResultPort::success(ProviderInvocationSuccessWire::new(
                        rejected.coordinate().clone(),
                        rejected_wire,
                    ))
                }
                ProviderServiceOutcome::Unavailable
                | ProviderServiceOutcome::ConcurrentUncertainty
                | ProviderServiceOutcome::ComposedConcurrentUncertainty => {
                    provider::ResultPort::unavailable(
                        ProviderInvocationUnavailability::EvidenceUnknown,
                    )
                }
            });
            let port = port_slot.as_ref().unwrap();
            let stale_current = parents.current.at_time(UnixTimeV1::new(10_000).unwrap());
            let release_current = if stale_release {
                &stale_current
            } else {
                parents.current
            };
            let expected_appends = if outcome == ProviderServiceOutcome::AggregateSaturated {
                0
            } else if outcome.is_composed_concurrent() || outcome.has_concurrent_head() {
                5
            } else if matches!(
                outcome,
                ProviderServiceOutcome::ColdRestart | ProviderServiceOutcome::LostAfterAcceptance
            ) {
                4
            } else if stale_release
                || matches!(
                    outcome,
                    ProviderServiceOutcome::AdmissionRejected
                        | ProviderServiceOutcome::ReconciliationAlias
                        | ProviderServiceOutcome::ComposedNonRetryableRefusals
                )
            {
                3
            } else {
                4
            };
            let expected_anchor_advances = if outcome.is_composed_concurrent() {
                5
            } else if outcome == ProviderServiceOutcome::ConcurrentHeadTwice {
                3
            } else if outcome.has_concurrent_head() {
                4
            } else {
                expected_appends
            };
            let conflicting_commits = if outcome == ProviderServiceOutcome::ConcurrentHeadTwice {
                vec![4, 5]
            } else if outcome.has_concurrent_head() {
                vec![4]
            } else {
                Vec::new()
            };
            let handle_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    handle_ref,
                    handle.canonical_operation_handle_sha256().as_str(),
                )
                .unwrap(),
                handle,
            )
            .unwrap();
            let lease_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    handle.operation_lease_ref(),
                    parents.lease.operation_lease_digest().as_str(),
                )
                .unwrap(),
                parents.lease,
            )
            .unwrap();
            let competing_head = Rc::new(RefCell::new(None));
            custody::with_service_custody_and_handle(
                initial_checkpoint,
                Some(initial_candidate),
                Some(lease_record),
                expected_appends,
                expected_anchor_advances,
                false,
                &conflicting_commits,
                Some(handle_record),
                Rc::clone(&competing_head),
                |custody, anchor| {
                    let mut authority = ProvingAuthority;
                    let initial_checkpoint_ref = record_ref("operation/checkpoint-1");
                    let dispatch_input = || ProviderOperationDispatchInput {
                        operation: ProviderOperationInput {
                            context: OperationAttemptContext {
                                operation: selected.operation,
                                request: selected.request,
                                session: parents.session,
                                current: parents.current,
                                lease: selected.lease,
                                context_binding: None,
                                generation: selected.generation,
                                selection_ref: record_ref("provider-service/selection"),
                                maximum_selection_bytes: 65_536,
                                observation_ref: record_ref("provider-service/preparation"),
                                observation,
                            },
                            invocation: selection.provider_request(selected).unwrap(),
                            result_admission: admission,
                            resolution_ref: record_ref("provider-service/resolution"),
                        },
                        checkpoint: (&initial_checkpoint_ref, initial_checkpoint),
                        checkpoint_schema: &checkpoint_fields.schema_sha256,
                        checkpoint_authority: &checkpoint_signing,
                        signing: observation.signed,
                        resolution_checkpoint: checkpoint_input("resolved"),
                        preparation_checkpoint: checkpoint_input("prepared"),
                        dispatch_checkpoint: checkpoint_input("dispatched"),
                        dispatch_ref: record_ref("provider-service/dispatched"),
                        clock: &clock,
                        resolve_store: &Store,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            1, 1024, 1024, 512, 65_536,
                        )
                        .unwrap(),
                    };
                    let invoke_input = || ProviderOperationInvokeInput {
                        current: release_current,
                        signing: observation.signed,
                        terminal_checkpoint: checkpoint_input("terminal"),
                        result_ref: record_ref("provider-service/result"),
                        diagnostic_ref: record_ref(
                            if matches!(outcome, ProviderServiceOutcome::AssociationRejected) {
                                "provider-service/adapter-failure"
                            } else {
                                "provider-service/diagnostic"
                            },
                        ),
                        reconciliation: ReconciliationFrontierReferenceV1 {
                            record_ref: record_ref(
                                if matches!(outcome, ProviderServiceOutcome::ReconciliationAlias) {
                                    "provider-service/result"
                                } else {
                                    "provider-service/reconciliation"
                                },
                            ),
                            record_sha256: ReconciliationFrontierDigest::parse(sha('d')).unwrap(),
                        },
                        terminal_ref: record_ref(
                            if matches!(outcome, ProviderServiceOutcome::AdmissionRejected) {
                                "provider-service/dispatched"
                            } else {
                                "provider-service/terminal"
                            },
                        ),
                        checkpoint_schema: &checkpoint_fields.schema_sha256,
                        checkpoint_authority: &checkpoint_signing,
                        clock: &clock,
                        resolve_store: &Store,
                        maximum_record_bytes: 65_536,
                        maximum_diagnostic_bytes: 65_536,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            1, 1024, 1024, 512, 65_536,
                        )
                        .unwrap(),
                    };
                    let evidence = match outcome {
                        ProviderServiceOutcome::ResultSignatureRejected => {
                            provider::Evidence::rejecting_result_signature()
                        }
                        ProviderServiceOutcome::FailureSignatureRejected => {
                            provider::Evidence::rejecting_failure_signature()
                        }
                        _ => provider::Evidence::valid(),
                    };
                    if outcome == ProviderServiceOutcome::AggregateSaturated {
                        let mut tasks = ResidentOperationTaskState::new(
                            ResidentOperationTaskLimits::new(
                                NonZeroUsize::new(1).expect("positive provider-task limit"),
                            ),
                            NonZeroUsize::new(1).expect("consultation batch bound"),
                            ResidentAggregateTaskLimits::new(
                                NonZeroUsize::new(1).expect("aggregate task bound"),
                            ),
                        )
                        .expect("bounded operation task state");
                        tasks.accept_provider_reservation_for_test(
                            crate::resident::task_owner::ResidentOperationTaskKey::new(
                                store_coordinate(),
                                ImmutableRecordReferenceV1 {
                                    record_ref: record_ref("provider-service/occupied"),
                                    record_sha256: ArtifactDigest::hash_bytes(
                                        b"provider-service/occupied",
                                    ),
                                },
                            ),
                        );
                        let rotation_calls = Cell::new(0);
                        let mut executor = DeferredProviderExecutor::default();
                        let mut service = ResidentOperationService::new(
                            &mut authority,
                            custody,
                            anchor,
                            &mut tasks,
                        );
                        let failure = service
                            .retain_and_accept_provider(
                                dispatch_input(),
                                &invoke_input(),
                                &mut executor,
                                |_, _, _, _| {
                                    rotation_calls.set(rotation_calls.get() + 1);
                                    Ok(())
                                },
                                |_, _, _| {
                                    observation_signs.set(observation_signs.get() + 1);
                                    Ok(signature())
                                },
                                |_, _, _| {
                                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                                    Ok(signature())
                                },
                            )
                            .expect_err("the filled aggregate ledger must reject before retention");
                        assert!(matches!(
                            failure,
                            crate::resident::operation_service::ResidentProviderTaskLaunchFailure::AggregateSaturated(_)
                        ));
                        assert_eq!(service.outstanding_task_reservations(), 1);
                        assert_eq!(service.in_flight_provider_tasks(), 0);
                        assert_eq!(rotation_calls.get(), 0);
                        assert_eq!(observation_signs.get(), 0);
                        assert_eq!(checkpoint_signs.get(), 0);
                        assert_eq!(clock.next.load(Ordering::SeqCst), 0);
                        assert_eq!(executor.accept_calls, 0);
                        assert_eq!(port.calls(), 0);
                        return;
                    }
                    if outcome == ProviderServiceOutcome::Rejected {
                        let result =
                            ResidentLocalOperationCoordinator::new(&mut authority, custody, anchor)
                                .invoke_provider(
                                    dispatch_input(),
                                    invoke_input(),
                                    port,
                                    &evidence,
                                    |frontier| {
                                        assert_eq!(
                                            frontier.record_ref,
                                            record_ref("provider-service/reconciliation")
                                        );
                                        Ok(())
                                    },
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| {
                                        observation_signs.set(observation_signs.get() + 1);
                                        Ok(signature())
                                    },
                                    |_, _, _| {
                                        checkpoint_signs.set(checkpoint_signs.get() + 1);
                                        Ok(signature())
                                    },
                                )
                                .expect("the combined path must retain uncertainty without replay");
                        let ProviderOperationInvokeResult::OutcomeUnknown(uncertain) = result
                        else {
                            panic!("a rejected provider response must remain outcome unknown")
                        };
                        assert_eq!(
                            uncertain.observation().1.predecessor_ref(),
                            &Some(record_ref("provider-service/dispatched"))
                        );
                        assert!(matches!(
                            uncertain.uncertainty(),
                            ProviderOperationUncertainty::ResponseRejected(_)
                        ));
                        assert_eq!(port.calls(), 1);
                        return;
                    }
                    let retained =
                        ResidentLocalOperationCoordinator::new(&mut authority, custody, anchor)
                            .retain_provider_dispatch(
                                dispatch_input(),
                                |_, _, _, _| Ok(()),
                                |_, _, _| {
                                    observation_signs.set(observation_signs.get() + 1);
                                    Ok(signature())
                                },
                                |_, _, _| {
                                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                                    Ok(signature())
                                },
                            )
                            .unwrap();
                    assert_eq!(
                        retained.provider_request().unwrap().provider_binding(),
                        selection
                            .provider_request(selected)
                            .unwrap()
                            .provider_binding()
                    );
                    assert!(matches!(
                        retained.observation().1.state(),
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::Dispatched { attempt_id, .. }
                        } if *attempt_id == *selection.attempt_id()
                    ));
                    assert_eq!(
                        retained.custody().database_evidence(),
                        OperationCustodyDatabaseEvidence::New
                    );
                    let dispatched = (
                        retained.observation().0.clone(),
                        retained.observation().1.clone(),
                    );
                    let dispatched_checkpoint = (
                        retained.checkpoint().0.clone(),
                        retained.checkpoint().1.clone(),
                    );
                    let selection_record = selection
                        .record(record_ref("provider-service/selection"), 65_536)
                        .unwrap();
                    let selection_evidence = ImmutableRecordReferenceV1 {
                        record_ref: record_ref("provider-service/selection"),
                        record_sha256: ArtifactDigest::parse(
                            selection_record.coordinate().record_sha256().as_str(),
                        )
                        .unwrap(),
                    };
                    let resolution_ref = record_ref("provider-service/resolution");
                    let resolution = issue_observation(
                        parents.signing,
                        handle_ref,
                        handle,
                        observation_schema.clone(),
                        OperationObservationRevisionV1::new(2).unwrap(),
                        Some((initial_record.coordinate().record_ref(), &initial)),
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::ProviderResolved {
                                provider_binding: selection.provider().unwrap().clone(),
                            },
                        },
                        61,
                    );
                    let preparation_ref = record_ref("provider-service/preparation");
                    let preparation = issue_observation(
                        parents.signing,
                        handle_ref,
                        handle,
                        observation_schema.clone(),
                        OperationObservationRevisionV1::new(3).unwrap(),
                        Some((&resolution_ref, &resolution)),
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::DispatchPrepared {
                                attempt_id: selection.attempt_id().clone(),
                                evidence: selection_evidence,
                            },
                        },
                        62,
                    );
                    dispatched
                        .1
                        .verify_admitted_successor_of(
                            handle_ref,
                            handle,
                            &preparation_ref,
                            &preparation,
                        )
                        .expect("retained dispatch must name the actual preparation");
                    let history = || OperationAttemptHistoryInput {
                        operation: selected.operation,
                        request: selected.request,
                        session: selected.operation.historical_session_lease(),
                        lease: selected.lease,
                        generation: selected.generation,
                        dispatched: &dispatched.1,
                        selection: &selection_record,
                        maximum_selection_bytes: 65_536,
                    };
                    let recovered_dispatch =
                        RecoveredOperationObservation::history_fixture_for_handle(
                            crate::resident::record_projection::coordinate(
                                handle_ref,
                                handle.canonical_operation_handle_sha256().as_str(),
                            )
                            .unwrap(),
                            dispatched.clone(),
                            Some((preparation_ref.clone(), preparation.clone())),
                            None,
                            dispatched.clone(),
                            dispatched_checkpoint.clone(),
                            store_coordinate(),
                        );
                    if outcome == ProviderServiceOutcome::ComposedSucceeded {
                        let mut executor = DeferredProviderExecutor::default();
                        let tasks = ResidentOperationTaskState::new(
                            ResidentOperationTaskLimits::new(
                                NonZeroUsize::new(1).expect("positive provider-task limit"),
                            ),
                            NonZeroUsize::new(1).expect("consultation batch bound"),
                            ResidentAggregateTaskLimits::new(
                                NonZeroUsize::new(1).expect("aggregate task bound"),
                            ),
                        )
                        .expect("bounded operation task state");
                        let mut operation_host = ResidentOperationHost::new(tasks);
                        let task_key = operation_host.with_operation_service(
                            &mut authority,
                            &mut *custody,
                            &mut *anchor,
                            |service| {
                                let task_key = service
                                    .accept_retained_provider(
                                        retained,
                                        &invoke_input(),
                                        &mut executor,
                                    )
                                    .unwrap_or_else(|_| {
                                        panic!("composed service must accept retained work")
                                    });
                                assert_eq!(service.in_flight_provider_tasks(), 1);
                                task_key
                            },
                        );
                        assert_eq!(task_key.dispatch().record_ref, dispatched.0);
                        assert_eq!(
                            task_key.store(),
                            recovered_dispatch.store(),
                            "accepted provider work keeps the custody store incarnation"
                        );
                        assert_eq!(executor.accept_calls, 1);
                        operation_host.with_operation_service(
                            &mut authority,
                            &mut *custody,
                            &mut *anchor,
                            |service| assert_eq!(service.in_flight_provider_tasks(), 1),
                        );
                        assert_eq!(port.calls(), 0, "retention must precede release");
                        executor.invoke_next(port);
                        let substituted_dispatch =
                            RecoveredOperationObservation::history_fixture_for_handle(
                                crate::resident::record_projection::coordinate(
                                    handle_ref,
                                    handle.canonical_operation_handle_sha256().as_str(),
                                )
                                .unwrap(),
                                (
                                    record_ref("provider-service/substituted-dispatch"),
                                    dispatched.1.clone(),
                                ),
                                Some((preparation_ref.clone(), preparation.clone())),
                                None,
                                (
                                    record_ref("provider-service/substituted-dispatch"),
                                    dispatched.1.clone(),
                                ),
                                dispatched_checkpoint.clone(),
                                store_coordinate(),
                            );
                        operation_host.with_operation_service(
                            &mut authority,
                            &mut *custody,
                            &mut *anchor,
                            |service| {
                                let rejected = service
                                    .poll_provider_completion(
                                        &mut executor,
                                        history(),
                                        &substituted_dispatch,
                                        admission,
                                        &invoke_input(),
                                        &evidence,
                                        |_| Ok(()),
                                        |_, _, _, _| Ok(()),
                                        |_, _, _| {
                                            observation_signs.set(observation_signs.get() + 1);
                                            Ok(signature())
                                        },
                                        |_, _, _| {
                                            checkpoint_signs.set(checkpoint_signs.get() + 1);
                                            Ok(signature())
                                        },
                                    )
                                    .expect("accepted task must produce one completion");
                                let ResidentProviderTaskPoll::RecoveryRejected {
                                    completed,
                                    reason: OperationAttemptError::Association,
                                } = rejected
                                else {
                                    panic!(
                                        "a substituted dispatch must retain the accepted completion"
                                    )
                                };
                                assert_eq!(service.in_flight_provider_tasks(), 0);
                                assert_eq!(port.calls(), 1);
                                let poll = service.continue_provider_completion_recovery(
                                    *completed,
                                    history(),
                                    &recovered_dispatch,
                                    admission,
                                    &invoke_input(),
                                    &evidence,
                                    |_| Ok(()),
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| {
                                        observation_signs.set(observation_signs.get() + 1);
                                        Ok(signature())
                                    },
                                    |_, _, _| {
                                        checkpoint_signs.set(checkpoint_signs.get() + 1);
                                        Ok(signature())
                                    },
                                );
                                let Ok(ResidentProviderTaskPoll::Recorded {
                                    result: ProviderOperationInvokeResult::Completed(completed),
                                    cancellation: ResidentOperationTaskCancellationState::Unnotified,
                                }) = poll
                                else {
                                    panic!("composed service must record the exact provider result")
                                };
                                assert!(matches!(
                                    completed.observation().1.state(),
                                    OperationObservationStateV1::Succeeded { .. }
                                ));
                                assert_eq!(service.in_flight_provider_tasks(), 0);
                            },
                        );
                        return;
                    }
                    let competing_cancellation = outcome.has_concurrent_head().then(|| {
                        let reference = record_ref("provider-service/cancellation-requested");
                        let observation = issue_observation(
                            parents.signing,
                            handle_ref,
                            handle,
                            observation_schema.clone(),
                            dispatched.1.revision().checked_successor().unwrap(),
                            Some((&dispatched.0, &dispatched.1)),
                            OperationObservationStateV1::CancellationRequested {
                                request: ImmutableRecordReferenceV1 {
                                    record_ref: record_ref("provider-service/cancellation-request"),
                                    record_sha256: ArtifactDigest::hash_bytes(
                                        b"provider cancellation request",
                                    ),
                                },
                            },
                            70,
                        );
                        let record = crate::resident::record_projection::coordinate(
                            &reference,
                            observation
                                .canonical_operation_observation_sha256()
                                .as_str(),
                        )
                        .and_then(|coordinate| {
                            crate::resident::record_projection::record(coordinate, &observation)
                        })
                        .unwrap();
                        *competing_head.borrow_mut() = Some(record);
                        (reference, observation)
                    });

                    if outcome == ProviderServiceOutcome::ComposedCancellation {
                        let mut tasks = ResidentOperationTaskState::new(
                            ResidentOperationTaskLimits::new(
                                NonZeroUsize::new(1).expect("positive provider-task limit"),
                            ),
                            NonZeroUsize::new(1).expect("consultation batch bound"),
                            ResidentAggregateTaskLimits::new(
                                NonZeroUsize::new(1).expect("aggregate task bound"),
                            ),
                        )
                        .expect("bounded operation task state");
                        let mut service = ResidentOperationService::new(
                            &mut authority,
                            custody,
                            anchor,
                            &mut tasks,
                        );
                        let mut executor = DeferredProviderExecutor::default();
                        let task_key = service
                            .accept_retained_provider(retained, &invoke_input(), &mut executor)
                            .unwrap_or_else(|_| {
                                panic!("retained provider work must enter the owned task table")
                            });
                        assert_eq!(task_key.dispatch().record_ref, dispatched.0);
                        assert_eq!(service.in_flight_provider_tasks(), 1);
                        assert_eq!(executor.accept_calls, 1);
                        assert_eq!(port.calls(), 0);

                        let recovered_operation = RecoveredLocalOperation::fixture(
                            selected.operation,
                            selected.request.clone(),
                            store_coordinate(),
                        );
                        let recovered_observation = RecoveredOperationObservation::fixture(
                            selected.operation,
                            dispatched.clone(),
                            dispatched.clone(),
                            dispatched_checkpoint.clone(),
                            store_coordinate(),
                        );
                        let cancellation = cancellation_request(selected.request, handle);
                        let authorized = AuthorizedOperationCancellation::fixture(
                            &cancellation,
                            &recovered_operation,
                            &recovered_observation,
                            UnixTimeV1::new(70).unwrap(),
                        );
                        let mut admit_rotation =
                            |_: &ResidentRecordRefV1,
                             _: &ArtifactDigest,
                             _: &ResidentRecordRefV1,
                             _: &ArtifactDigest| { Ok(()) };
                        let mut sign_observation =
                            |_: &[u8],
                             _: &OperationObservationPreimageV1,
                             _: &OperationObservationDigest| {
                                observation_signs.set(observation_signs.get() + 1);
                                Ok(signature())
                            };
                        let mut sign_checkpoint =
                            |_: &[u8],
                             _: &ControlStateCheckpointPreimageV1,
                             _: &ControlStateCheckpointDigest| {
                                checkpoint_signs.set(checkpoint_signs.get() + 1);
                                Ok(signature())
                            };
                        let outcome = match service.retain_authorized_cancellation(
                                authorized,
                                CancellationAppendInput {
                                    request_ref: record_ref(
                                        "provider-service/cancellation-request",
                                    ),
                                    observation_ref: record_ref(
                                        "provider-service/cancellation-requested",
                                    ),
                                    signing: observation.signed,
                                    checkpoint: checkpoint_input("cancellation"),
                                    checkpoint_issued_at: UnixTimeV1::new(70).unwrap(),
                                    checkpoint_schema: &checkpoint_fields.schema_sha256,
                                    checkpoint_authority: &checkpoint_signing,
                                    accumulator_limits: AccumulatorPreparationLimits::new(
                                        2, 1024, 1024, 512, 65_536,
                                    )
                                    .unwrap(),
                                },
                                &mut CancellationAppendAdmission {
                                    resolve_store: &Store,
                                    admit_rotation: &mut admit_rotation,
                                    sign_observation: &mut sign_observation,
                                    sign_checkpoint: &mut sign_checkpoint,
                                },
                            ) {
                            Ok(outcome) => outcome,
                            Err(crate::resident::operation_service::ResidentOperationCancellationFailure::Recovery(_)) => panic!("fixture already completed protected recovery"),
                            Err(crate::resident::operation_service::ResidentOperationCancellationFailure::Authorization(_)) => panic!("fixture already completed fresh authorization"),
                            Err(crate::resident::operation_service::ResidentOperationCancellationFailure::Request(_)) => panic!("accepted provider cancellation request must project"),
                            Err(crate::resident::operation_service::ResidentOperationCancellationFailure::Completion(_)) => panic!("dispatched provider cancellation cannot enter local completion"),
                            Err(crate::resident::operation_service::ResidentOperationCancellationFailure::Custody(failure)) => panic!("accepted provider cancellation must reach custody: {failure:?}"),
                            Err(crate::resident::operation_service::ResidentOperationCancellationFailure::DispatchedCustody(failure)) => panic!("accepted provider cancellation custody must complete: {failure:?}"),
                            Err(crate::resident::operation_service::ResidentOperationCancellationFailure::Association) => panic!("accepted provider task must retain its exact dispatch association"),
                            Err(crate::resident::operation_service::ResidentOperationCancellationFailure::DispatchOwnerConflict) => panic!("accepted provider task cannot share its exact dispatch owner"),
                            Err(crate::resident::operation_service::ResidentOperationCancellationFailure::ConsultationRouteRequired) => panic!("provider cancellation cannot route to consultation custody"),
                        };
                        assert!(matches!(
                            outcome,
                            ResidentOperationCancellationOutcome::DispatchedRequestRetained {
                                notification:
                                    ResidentOperationTaskCancellationNotification::Notified,
                                ..
                            }
                        ));
                        assert_eq!(service.in_flight_provider_tasks(), 1);
                        assert_eq!(port.calls(), 0);
                        return;
                    }

                    if outcome.is_composed_concurrent() {
                        let mut tasks = ResidentOperationTaskState::new(
                            ResidentOperationTaskLimits::new(
                                NonZeroUsize::new(1).expect("positive provider-task limit"),
                            ),
                            NonZeroUsize::new(1).expect("consultation batch bound"),
                            ResidentAggregateTaskLimits::new(
                                NonZeroUsize::new(1).expect("aggregate task bound"),
                            ),
                        )
                        .expect("bounded operation task state");
                        let mut executor = DeferredProviderExecutor::default();
                        let task_key = {
                            let mut service = ResidentOperationService::new(
                                &mut authority,
                                &mut *custody,
                                &mut *anchor,
                                &mut tasks,
                            );
                            service
                                .accept_retained_provider(retained, &invoke_input(), &mut executor)
                                .unwrap_or_else(|_| {
                                    panic!("retained provider work must enter the owned task table")
                                })
                        };
                        assert_eq!(task_key.dispatch().record_ref, dispatched.0);
                        assert_eq!(executor.accept_calls, 1);
                        assert_eq!(port.calls(), 0);
                        executor.invoke_next(port);
                        assert_eq!(port.calls(), 1);

                        let (cancellation_custody, cancellation_checkpoint) = {
                            let mut service = ResidentOperationService::new(
                                &mut authority,
                                &mut *custody,
                                &mut *anchor,
                                &mut tasks,
                            );

                            let denied = request_composed_provider_cancellation(
                                &mut service,
                                selected.operation,
                                selected.request,
                                &parents,
                                None,
                                initial_candidate,
                                (&dispatched.0, &dispatched.1),
                                (&dispatched_checkpoint.0, &dispatched_checkpoint.1),
                                false,
                            );
                            match denied {
                                Err(ResidentOperationCancellationFailure::Authorization(_)) => {}
                                Err(ResidentOperationCancellationFailure::Recovery(failure)) => {
                                    panic!(
                                        "protected dispatch recovery failed before denial: {failure:?}"
                                    )
                                }
                                Err(ResidentOperationCancellationFailure::Request(failure)) => {
                                    panic!("denied request projected unexpectedly: {failure:?}")
                                }
                                Err(ResidentOperationCancellationFailure::Completion(failure)) => {
                                    panic!("denied request reached local completion: {failure:?}")
                                }
                                Err(ResidentOperationCancellationFailure::Custody(failure)) => {
                                    panic!(
                                        "denied request reached custody unexpectedly: {failure:?}"
                                    )
                                }
                                Err(ResidentOperationCancellationFailure::DispatchedCustody(
                                    failure,
                                )) => {
                                    panic!(
                                        "denied request reached dispatched custody unexpectedly: {failure:?}"
                                    )
                                }
                                Err(ResidentOperationCancellationFailure::Association) => {
                                    panic!("denied request lost provider association")
                                }
                                Err(
                                    ResidentOperationCancellationFailure::DispatchOwnerConflict,
                                ) => {
                                    panic!("denied request found conflicting dispatch owners")
                                }
                                Err(
                                    ResidentOperationCancellationFailure::ConsultationRouteRequired,
                                ) => panic!("provider denial cannot route to consultation custody"),
                                Ok(_) => panic!("denied authority accepted cancellation"),
                            }
                            assert_eq!(service.in_flight_provider_tasks(), 1);

                            let retained_cancellation = request_composed_provider_cancellation(
                                &mut service,
                                selected.operation,
                                selected.request,
                                &parents,
                                None,
                                initial_candidate,
                                (&dispatched.0, &dispatched.1),
                                (&dispatched_checkpoint.0, &dispatched_checkpoint.1),
                                true,
                            )
                            .unwrap_or_else(|_| {
                                panic!("fresh cancellation authority must reach exact custody")
                            });
                            let ResidentOperationCancellationOutcome::DispatchedRequestRetained {
                                custody: cancellation_custody,
                                checkpoint: cancellation_checkpoint,
                                notification:
                                    ResidentOperationTaskCancellationNotification::Notified,
                            } = retained_cancellation
                            else {
                                panic!("accepted provider cancellation must notify after custody")
                            };
                            (cancellation_custody, cancellation_checkpoint)
                        };
                        assert_eq!(port.calls(), 1);
                        let cancellation_request =
                            cancellation_request(selected.request, selected.operation.handle().1);
                        let cancellation_ref =
                            record_ref("provider-service/composed-cancellation-requested");
                        let cancellation = issue_observation(
                            parents.signing,
                            handle_ref,
                            handle,
                            observation_schema.clone(),
                            dispatched.1.revision().checked_successor().unwrap(),
                            Some((&dispatched.0, &dispatched.1)),
                            OperationObservationStateV1::CancellationRequested {
                                request: ImmutableRecordReferenceV1 {
                                    record_ref: record_ref(
                                        "provider-service/composed-cancellation-request",
                                    ),
                                    record_sha256: ArtifactDigest::parse(
                                        cancellation_request.canonical_request_sha256().as_str(),
                                    )
                                    .unwrap(),
                                },
                            },
                            70,
                        );
                        let retained_observation = cancellation_custody
                            .candidate()
                            .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
                            .expect("cancellation custody retains its observation");
                        assert_eq!(
                            retained_observation.canonical_record_jcs().as_slice(),
                            pale_core::canon::to_canonical_string(&cancellation)
                                .unwrap()
                                .as_bytes()
                        );

                        let recovered = RecoveredOperationObservation::history_fixture_for_handle(
                            crate::resident::record_projection::coordinate(
                                handle_ref,
                                handle.canonical_operation_handle_sha256().as_str(),
                            )
                            .unwrap(),
                            dispatched.clone(),
                            Some((preparation_ref.clone(), preparation.clone())),
                            None,
                            (cancellation_ref.clone(), cancellation.clone()),
                            (
                                cancellation_checkpoint.0.clone(),
                                cancellation_checkpoint.1.clone(),
                            ),
                            store_coordinate(),
                        );

                        let mut service = ResidentOperationService::new(
                            &mut authority,
                            &mut *custody,
                            &mut *anchor,
                            &mut tasks,
                        );
                        let poll = service
                            .poll_provider_completion(
                                &mut executor,
                                history(),
                                &recovered,
                                admission,
                                &invoke_input(),
                                &evidence,
                                |_| Ok(()),
                                |_, _, _, _| Ok(()),
                                |_, _, _| Ok(signature()),
                                |_, _, _| Ok(signature()),
                            )
                            .expect("late provider evidence must join without waiting");
                        let ResidentProviderTaskPoll::Recorded {
                            result,
                            cancellation: ResidentOperationTaskCancellationState::Notified,
                        } = poll
                        else {
                            panic!("late evidence must retain its cancellation notification")
                        };
                        match (outcome, result) {
                            (
                                ProviderServiceOutcome::ComposedConcurrentSuccess,
                                ProviderOperationInvokeResult::Completed(completed),
                            ) => assert!(matches!(
                                completed.observation().1.state(),
                                OperationObservationStateV1::Succeeded { .. }
                            )),
                            (
                                ProviderServiceOutcome::ComposedConcurrentFailure,
                                ProviderOperationInvokeResult::Completed(completed),
                            ) => assert!(matches!(
                                completed.observation().1.state(),
                                OperationObservationStateV1::Failed { .. }
                            )),
                            (
                                ProviderServiceOutcome::ComposedConcurrentUncertainty,
                                ProviderOperationInvokeResult::OutcomeUnknown(uncertain),
                            ) => assert!(matches!(
                                uncertain.observation().1.state(),
                                OperationObservationStateV1::OutcomeUnknown { .. }
                            )),
                            _ => panic!("protected continuation changed the provider disposition"),
                        }
                        assert_eq!(service.in_flight_provider_tasks(), 0);
                        assert_eq!(port.calls(), 1);
                        return;
                    }

                    if matches!(
                        outcome,
                        ProviderServiceOutcome::ComposedRefusalContinuation
                            | ProviderServiceOutcome::ComposedNonRetryableRefusals
                    ) {
                        let mut tasks = ResidentOperationTaskState::new(
                            ResidentOperationTaskLimits::new(
                                NonZeroUsize::new(1).expect("positive provider-task limit"),
                            ),
                            NonZeroUsize::new(1).expect("consultation batch bound"),
                            ResidentAggregateTaskLimits::new(
                                NonZeroUsize::new(1).expect("aggregate task bound"),
                            ),
                        )
                        .expect("bounded operation task state");
                        let mut executor = DeferredProviderExecutor {
                            refuse_next: true,
                            ..DeferredProviderExecutor::default()
                        };
                        let refusal = {
                            let mut service = ResidentOperationService::new(
                                &mut authority,
                                &mut *custody,
                                &mut *anchor,
                                &mut tasks,
                            );
                            let refusal = match service.accept_retained_provider(
                                retained,
                                &invoke_input(),
                                &mut executor,
                            ) {
                                Err(ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal)) => {
                                    refusal
                                }
                                Err(ResidentProviderTaskAcceptanceFailure::Preflight(_))
                                | Err(ResidentProviderTaskAcceptanceFailure::AggregateSaturated(
                                    _,
                                ))
                                | Ok(_) => {
                                    panic!("fixture must stop only at process-local acceptance")
                                }
                            };
                            assert!(matches!(
                                &refusal,
                                ResidentProviderTaskRefusal::Executor { .. }
                            ));
                            assert_eq!(executor.accept_calls, 1);
                            assert_eq!(service.in_flight_provider_tasks(), 0);
                            assert_eq!(port.calls(), 0);
                            if outcome == ProviderServiceOutcome::ComposedNonRetryableRefusals {
                                let call = match refusal {
                                    ResidentProviderTaskRefusal::Executor { call, .. } => call,
                                    _ => {
                                        panic!("fixture must return one unopened executor refusal")
                                    }
                                };
                                let preparation = ResidentProviderTaskRefusal::Preparation {
                                    call,
                                    reason: OperationAttemptError::Association,
                                };
                                let preparation = service
                                    .continue_provider_acceptance(preparation, &mut executor)
                                    .expect_err("preparation refusal is not retryable");
                                let call = match preparation {
                                    ResidentProviderTaskRefusal::Preparation {
                                        call,
                                        reason: OperationAttemptError::Association,
                                    } => call,
                                    _ => panic!("preparation refusal must return unchanged"),
                                };
                                let duplicate = service
                                    .continue_provider_acceptance(
                                        ResidentProviderTaskRefusal::Duplicate(call),
                                        &mut executor,
                                    )
                                    .expect_err("duplicate refusal is not retryable");
                                assert!(matches!(
                                    duplicate,
                                    ResidentProviderTaskRefusal::Duplicate(_)
                                ));
                                assert_eq!(executor.accept_calls, 1);
                                assert_eq!(service.in_flight_provider_tasks(), 0);
                                assert_eq!(port.calls(), 0);
                                return;
                            }
                            refusal
                        };
                        let mut unrelated_tasks = ResidentOperationTaskState::new(
                            ResidentOperationTaskLimits::new(
                                NonZeroUsize::new(1).expect("positive provider-task limit"),
                            ),
                            NonZeroUsize::new(1).expect("consultation batch bound"),
                            ResidentAggregateTaskLimits::new(
                                NonZeroUsize::new(1).expect("aggregate task bound"),
                            ),
                        )
                        .expect("bounded unrelated operation task state");
                        let refusal = {
                            let mut unrelated_service = ResidentOperationService::new(
                                &mut authority,
                                &mut *custody,
                                &mut *anchor,
                                &mut unrelated_tasks,
                            );
                            let refusal = unrelated_service
                                .continue_provider_acceptance(refusal, &mut executor)
                                .expect_err("another process owner must reject the refusal intact");
                            assert_eq!(executor.accept_calls, 1);
                            assert_eq!(unrelated_service.in_flight_provider_tasks(), 0);
                            assert_eq!(port.calls(), 0);
                            refusal
                        };
                        let mut service = ResidentOperationService::new(
                            &mut authority,
                            &mut *custody,
                            &mut *anchor,
                            &mut tasks,
                        );
                        let accepted = service
                            .continue_provider_acceptance(refusal, &mut executor)
                            .unwrap_or_else(|_| {
                                panic!("the unopened call must be accepted without re-retention")
                            });
                        assert_eq!(
                            accepted.dispatch().record_ref,
                            record_ref("provider-service/dispatched")
                        );
                        assert_eq!(executor.accept_calls, 2);
                        assert_eq!(service.in_flight_provider_tasks(), 1);
                        assert_eq!(port.calls(), 0);
                        executor.invoke_next(port);
                        let poll = service
                            .poll_provider_completion(
                                &mut executor,
                                history(),
                                &recovered_dispatch,
                                admission,
                                &invoke_input(),
                                &evidence,
                                |_| Ok(()),
                                |_, _, _, _| Ok(()),
                                |_, _, _| {
                                    observation_signs.set(observation_signs.get() + 1);
                                    Ok(signature())
                                },
                                |_, _, _| {
                                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                                    Ok(signature())
                                },
                            )
                            .expect("accepted continuation must return one completion");
                        assert!(matches!(
                            poll,
                            ResidentProviderTaskPoll::Recorded {
                                result: ProviderOperationInvokeResult::Completed(_),
                                cancellation: ResidentOperationTaskCancellationState::Unnotified
                            }
                        ));
                        assert_eq!(service.in_flight_provider_tasks(), 0);
                        assert_eq!(port.calls(), 1);
                        return;
                    }

                    if matches!(
                        outcome,
                        ProviderServiceOutcome::ColdRestart
                            | ProviderServiceOutcome::LostAfterAcceptance
                    ) {
                        let recovered = &recovered_dispatch;
                        let recovered = if outcome == ProviderServiceOutcome::LostAfterAcceptance {
                            let mut tasks = ResidentOperationTaskState::new(
                                ResidentOperationTaskLimits::new(
                                    NonZeroUsize::new(1).expect("positive provider-task limit"),
                                ),
                                NonZeroUsize::new(1).expect("consultation batch bound"),
                                ResidentAggregateTaskLimits::new(
                                    NonZeroUsize::new(1).expect("aggregate task bound"),
                                ),
                            )
                            .expect("bounded operation task state");
                            let mut executor = DeferredProviderExecutor::default();
                            let task_key = {
                                let mut service = ResidentOperationService::new(
                                    &mut authority,
                                    &mut *custody,
                                    &mut *anchor,
                                    &mut tasks,
                                );
                                match service.accept_retained_provider(
                                    retained,
                                    &invoke_input(),
                                    &mut executor,
                                ) {
                                    Ok(task_key) => task_key,
                                    Err(_) => panic!(
                                        "lost-work fixture must accept retained provider work"
                                    ),
                                }
                            };
                            assert_eq!(executor.accept_calls, 1);
                            assert_eq!(port.calls(), 0);
                            assert_eq!(tasks.in_flight_provider_tasks(), 1);
                            executor.lose_next();
                            let lost = {
                                let mut service = ResidentOperationService::new(
                                    &mut authority,
                                    &mut *custody,
                                    &mut *anchor,
                                    &mut tasks,
                                );
                                let poll = service
                                    .poll_provider_completion(
                                        &mut executor,
                                        history(),
                                        recovered,
                                        admission,
                                        &invoke_input(),
                                        &evidence,
                                        |_| Ok(()),
                                        |_, _, _, _| Ok(()),
                                        |_, _, _| Ok(signature()),
                                        |_, _, _| Ok(signature()),
                                    )
                                    .expect("lost executor work must be observed");
                                let ResidentProviderTaskPoll::Lost(lost) = poll else {
                                    panic!(
                                        "lost executor work must retain protected recovery basis"
                                    )
                                };
                                lost
                            };
                            assert_eq!(
                                lost.cancellation_state(),
                                ResidentOperationTaskCancellationState::Unnotified
                            );
                            assert_eq!(task_key.dispatch().record_ref, dispatched.0);
                            let substituted = RecoveredOperationObservation::fixture(
                                selected.operation,
                                (
                                    record_ref("provider-service/substituted-dispatch"),
                                    dispatched.1.clone(),
                                ),
                                dispatched.clone(),
                                dispatched_checkpoint.clone(),
                                store_coordinate(),
                            );
                            let lost = {
                                let mut service = ResidentOperationService::new(
                                    &mut authority,
                                    &mut *custody,
                                    &mut *anchor,
                                    &mut tasks,
                                );
                                match service.recover_lost_provider(
                                    *lost,
                                    history(),
                                    &substituted,
                                    &invoke_input(),
                                    |_| Ok(()),
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| Ok(signature()),
                                    |_, _, _| Ok(signature()),
                                ) {
                                    Err(ResidentProviderTaskRecoveryFailure::Association(lost)) => {
                                        lost
                                    }
                                    Err(ResidentProviderTaskRecoveryFailure::Recording {
                                        ..
                                    })
                                    | Err(ResidentProviderTaskRecoveryFailure::Settlement {
                                        ..
                                    })
                                    | Ok(_) => panic!(
                                        "a substituted protected dispatch must not record lost work"
                                    ),
                                }
                            };
                            let mut service = ResidentOperationService::new(
                                &mut authority,
                                &mut *custody,
                                &mut *anchor,
                                &mut tasks,
                            );
                            match service.recover_lost_provider(
                                *lost,
                                history(),
                                recovered,
                                &invoke_input(),
                                |frontier| {
                                    assert_eq!(
                                        frontier.record_ref,
                                        record_ref("provider-service/reconciliation")
                                    );
                                    Ok(())
                                },
                                |_, _, _, _| Ok(()),
                                |_, _, _| {
                                    observation_signs.set(observation_signs.get() + 1);
                                    Ok(signature())
                                },
                                |_, _, _| {
                                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                                    Ok(signature())
                                },
                            ) {
                                Ok(ResidentProviderTaskPoll::Recorded {
                                    result,
                                    cancellation: ResidentOperationTaskCancellationState::Unnotified,
                                }) => result,
                                Ok(_) => {
                                    panic!("lost work must finish protected uncertainty custody")
                                }
                                Err(_) => panic!("lost work must enter protected unknown custody"),
                            }
                        } else {
                            ResidentLocalOperationCoordinator::new(&mut authority, custody, anchor)
                                .recover_dispatched(
                                    history(),
                                    recovered,
                                    &invoke_input(),
                                    |frontier| {
                                        assert_eq!(
                                            frontier.record_ref,
                                            record_ref("provider-service/reconciliation")
                                        );
                                        Ok(())
                                    },
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| {
                                        observation_signs.set(observation_signs.get() + 1);
                                        Ok(signature())
                                    },
                                    |_, _, _| {
                                        checkpoint_signs.set(checkpoint_signs.get() + 1);
                                        Ok(signature())
                                    },
                                )
                                .expect(
                                    "cold restart must retain uncertainty without provider replay",
                                )
                        };
                        let ProviderOperationInvokeResult::OutcomeUnknown(uncertain) = recovered
                        else {
                            panic!("cold restart must remain outcome unknown")
                        };
                        assert!(matches!(
                            uncertain.uncertainty(),
                            ProviderOperationUncertainty::ColdRestartAfterDispatch
                        ));
                        assert!(matches!(
                            uncertain.observation().1.state(),
                            OperationObservationStateV1::OutcomeUnknown { .. }
                        ));
                        assert_eq!(port.calls(), 0);
                        return;
                    }
                    let mut service =
                        ResidentProviderTaskService::new(ResidentOperationTaskLimits::new(
                            NonZeroUsize::new(1).expect("positive provider-task limit"),
                        ))
                        .expect("provider task service");
                    let mut task_reservations = service.test_reservations();
                    let mut executor = DeferredProviderExecutor::default();
                    let accepted = service.try_accept(
                        &mut ResidentLocalOperationCoordinator::new(
                            &mut authority,
                            custody,
                            anchor,
                        ),
                        retained,
                        &invoke_input(),
                        &mut task_reservations,
                        &mut executor,
                    );
                    let mut accepted_key = None;
                    let result = match accepted {
                        Err(ResidentProviderTaskAcceptanceFailure::Preflight(failure)) => {
                            Err(failure)
                        }
                        Err(ResidentProviderTaskAcceptanceFailure::TaskOwner(_)) => {
                            panic!("the empty bounded task owner must accept the provider call")
                        }
                        Err(ResidentProviderTaskAcceptanceFailure::AggregateSaturated(_)) => {
                            panic!("the empty aggregate owner must accept the provider call")
                        }
                        Ok(task_key) => {
                            accepted_key = Some(task_key.clone());
                            assert_eq!(executor.accept_calls, 1);
                            assert_eq!(service.in_flight(), 1);
                            assert_eq!(port.calls(), 0, "retention must precede release");
                            if outcome == ProviderServiceOutcome::ParallelServiceIsolation {
                                let parallel_service = ResidentProviderTaskService::new(
                                    ResidentOperationTaskLimits::new(
                                        NonZeroUsize::new(1).expect("positive provider-task limit"),
                                    ),
                                )
                                .expect("parallel provider task service");
                                assert_eq!(parallel_service.in_flight(), 0);
                                assert!(matches!(
                                    parallel_service.select_dispatch_owner(&task_key),
                                    ResidentProviderDispatchOwnerSelection::Missing
                                ));
                            }
                            if outcome.has_cancellation_notification() {
                                let ResidentProviderDispatchOwnerSelection::Owned(owner) =
                                    service.select_dispatch_owner(&task_key)
                                else {
                                    panic!("accepted provider task must retain its exact owner")
                                };
                                assert_eq!(
                                    service.note_dispatch_cancellation_notification(owner),
                                    ResidentOperationTaskCancellationNotification::Notified
                                );
                            }
                            executor.invoke_next(port);
                            let mut record = |selected_anchor: &mut dyn AntiRollbackAnchorPort| {
                                let poll = service
                                    .try_record_completion(
                                        &mut ResidentLocalOperationCoordinator::new(
                                            &mut authority,
                                            custody,
                                            selected_anchor,
                                        ),
                                        &mut task_reservations,
                                        &mut executor,
                                        history(),
                                        &recovered_dispatch,
                                        admission,
                                        &invoke_input(),
                                        &evidence,
                                        |frontier| {
                                            assert_eq!(
                                                frontier.record_ref,
                                                record_ref("provider-service/reconciliation")
                                            );
                                            Ok(())
                                        },
                                        |_, _, _, _| Ok(()),
                                        |_, _, _| {
                                            observation_signs.set(observation_signs.get() + 1);
                                            if matches!(
                                                outcome,
                                                ProviderServiceOutcome::RecordingRetry
                                            ) && observation_signs.get() == 4
                                            {
                                                return Err(
                                                    InvalidResidentRecordV1::ContextualAdmission {
                                                        record: "provider terminal observation",
                                                        invariant:
                                                            "test signer accepts the retry only",
                                                    },
                                                );
                                            }
                                            Ok(signature())
                                        },
                                        |_, _, _| {
                                            checkpoint_signs.set(checkpoint_signs.get() + 1);
                                            Ok(signature())
                                        },
                                    )
                                    .expect("the accepted provider task must complete");
                                if let ResidentProviderTaskPoll::Recorded { cancellation, .. } =
                                    &poll
                                {
                                    assert_eq!(*cancellation, outcome.cancellation_state());
                                }
                                assert_eq!(service.in_flight(), 0);
                                if matches!(
                                    outcome,
                                    ProviderServiceOutcome::RecordingRetry
                                        | ProviderServiceOutcome::AnchorUnknown
                                ) {
                                    assert_eq!(
                                        task_reservations.outstanding(),
                                        1,
                                        "recording failure must retain aggregate occupancy",
                                    );
                                    assert_eq!(
                                        executor.accept_calls, 1,
                                        "recording failure grants no provider replay",
                                    );
                                }
                                Ok(poll)
                            };
                            if matches!(outcome, ProviderServiceOutcome::AnchorUnknown) {
                                let mut ambiguous = AmbiguousFinalAnchor {
                                    inner: anchor,
                                    observations: 0,
                                };
                                record(&mut ambiguous)
                            } else {
                                record(anchor)
                            }
                        }
                    };
                    if matches!(
                        outcome,
                        ProviderServiceOutcome::AdmissionRejected
                            | ProviderServiceOutcome::ReconciliationAlias
                    ) {
                        let failure = match result {
                            Err(failure @ ProviderOperationInvokeFailure::Admission { .. }) => {
                                failure
                            }
                            _ => panic!(
                                "inconsistent output references must refuse before the provider call"
                            ),
                        };
                        let dispatch = failure
                            .into_unopened_dispatch()
                            .expect("pre-call refusal must return the exact unopened dispatch");
                        assert_eq!(
                            dispatch.custody().database_evidence(),
                            OperationCustodyDatabaseEvidence::New
                        );
                        assert!(matches!(
                            dispatch.observation().1.state(),
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::Dispatched { attempt_id, .. },
                            } if attempt_id == selection.attempt_id()
                        ));
                    } else if stale_release {
                        let Err(ProviderOperationInvokeFailure::Release { dispatch, .. }) = result
                        else {
                            panic!("stale authority must refuse before the provider call")
                        };
                        assert_eq!(
                            dispatch.custody().database_evidence(),
                            OperationCustodyDatabaseEvidence::New
                        );
                    } else {
                        if matches!(outcome, ProviderServiceOutcome::RecordingRetry) {
                            let task_key = accepted_key
                                .clone()
                                .expect("recording continuation follows accepted provider work");
                            let Ok(ResidentProviderTaskPoll::Preparation {
                                continuation,
                                reason,
                            }) = result
                            else {
                                panic!("pre-commit recording failure must return the consumed call")
                            };
                            assert!(matches!(
                                *reason,
                                ProviderOperationRecordingPreparationError::Operation(_)
                            ));
                            let mut continuation = *continuation;
                            let clock_calls = clock.next.load(Ordering::SeqCst);
                            let observation_sign_calls = observation_signs.get();
                            let checkpoint_sign_calls = checkpoint_signs.get();

                            let mut replacement_service =
                                ResidentProviderTaskService::new(ResidentOperationTaskLimits::new(
                                    NonZeroUsize::new(1)
                                        .expect("positive replacement provider-task limit"),
                                ))
                                .expect("replacement provider task service");
                            let mut replacement_reservations =
                                replacement_service.test_reservations();
                            let rejection = match replacement_service.continue_recording(
                                &mut ResidentLocalOperationCoordinator::new(
                                    &mut authority,
                                    custody,
                                    anchor,
                                ),
                                continuation,
                                &mut replacement_reservations,
                                &invoke_input(),
                                |_| panic!("foreign continuation reached reconciliation"),
                                |_, _, _, _| panic!("foreign continuation reached rotation"),
                                |_, _, _| panic!("foreign continuation reached signing"),
                                |_, _, _| panic!("foreign continuation reached checkpoint signing"),
                            ) {
                                Err(rejection) => rejection,
                                Ok(_) => panic!("a replacement owner accepted the continuation"),
                            };
                            assert_eq!(
                                rejection.reason,
                                ResidentTaskReservationTransitionFailure::OwnerMismatch
                            );
                            continuation = *rejection.continuation;
                            assert_eq!(replacement_reservations.outstanding(), 0);

                            let provider_route = ResidentTaskReservationRoute::Provider;
                            task_reservations.set_phase_for_test(
                                &task_key,
                                &provider_route,
                                ResidentTaskReservationPhase::Accepted,
                            );
                            let rejection = match service.continue_recording(
                                &mut ResidentLocalOperationCoordinator::new(
                                    &mut authority,
                                    custody,
                                    anchor,
                                ),
                                continuation,
                                &mut task_reservations,
                                &invoke_input(),
                                |_| panic!("wrong-phase continuation reached reconciliation"),
                                |_, _, _, _| panic!("wrong-phase continuation reached rotation"),
                                |_, _, _| panic!("wrong-phase continuation reached signing"),
                                |_, _, _| {
                                    panic!("wrong-phase continuation reached checkpoint signing")
                                },
                            ) {
                                Err(rejection) => rejection,
                                Ok(_) => panic!("the wrong aggregate phase accepted continuation"),
                            };
                            assert!(matches!(
                                rejection.reason,
                                ResidentTaskReservationTransitionFailure::Phase {
                                    expected:
                                        ResidentTaskReservationPhase::CompletionPendingDurableHandling,
                                    actual: ResidentTaskReservationPhase::Accepted,
                                }
                            ));
                            continuation = *rejection.continuation;
                            task_reservations.set_phase_for_test(
                                &task_key,
                                &provider_route,
                                ResidentTaskReservationPhase::CompletionPendingDurableHandling,
                            );

                            let consultation_route = ResidentTaskReservationRoute::Consultation(
                                ResidentConsultationBatchCoordinate::hostile_fixture(
                                    ResidentOperationTaskOwnerInstance::new(),
                                    store_coordinate(),
                                    ImmutableRecordReferenceV1 {
                                        record_ref: record_ref(
                                            "provider-service/hostile-consult-parent",
                                        ),
                                        record_sha256: ArtifactDigest::hash_bytes(
                                            b"provider hostile consult parent",
                                        ),
                                    },
                                    ImmutableRecordReferenceV1 {
                                        record_ref: record_ref(
                                            "provider-service/hostile-consult-frontier",
                                        ),
                                        record_sha256: ArtifactDigest::hash_bytes(
                                            b"provider hostile consult frontier",
                                        ),
                                    },
                                ),
                            );
                            task_reservations.replace_route_for_test(&task_key, consultation_route);
                            let rejection = match service.continue_recording(
                                &mut ResidentLocalOperationCoordinator::new(
                                    &mut authority,
                                    custody,
                                    anchor,
                                ),
                                continuation,
                                &mut task_reservations,
                                &invoke_input(),
                                |_| panic!("wrong-route continuation reached reconciliation"),
                                |_, _, _, _| panic!("wrong-route continuation reached rotation"),
                                |_, _, _| panic!("wrong-route continuation reached signing"),
                                |_, _, _| {
                                    panic!("wrong-route continuation reached checkpoint signing")
                                },
                            ) {
                                Err(rejection) => rejection,
                                Ok(_) => panic!("the wrong aggregate route accepted continuation"),
                            };
                            assert_eq!(
                                rejection.reason,
                                ResidentTaskReservationTransitionFailure::RouteMismatch
                            );
                            continuation = *rejection.continuation;
                            task_reservations
                                .replace_route_for_test(&task_key, provider_route.clone());

                            let substituted_key =
                                crate::resident::task_owner::ResidentOperationTaskKey::new(
                                    store_coordinate(),
                                    ImmutableRecordReferenceV1 {
                                        record_ref: record_ref(
                                            "provider-service/substituted-task-key",
                                        ),
                                        record_sha256: ArtifactDigest::hash_bytes(
                                            b"provider substituted task key",
                                        ),
                                    },
                                );
                            task_reservations
                                .replace_key_for_test(&task_key, substituted_key.clone());
                            let rejection = match service.continue_recording(
                                &mut ResidentLocalOperationCoordinator::new(
                                    &mut authority,
                                    custody,
                                    anchor,
                                ),
                                continuation,
                                &mut task_reservations,
                                &invoke_input(),
                                |_| panic!("wrong-key continuation reached reconciliation"),
                                |_, _, _, _| panic!("wrong-key continuation reached rotation"),
                                |_, _, _| panic!("wrong-key continuation reached signing"),
                                |_, _, _| {
                                    panic!("wrong-key continuation reached checkpoint signing")
                                },
                            ) {
                                Err(rejection) => rejection,
                                Ok(_) => panic!("the wrong aggregate key accepted continuation"),
                            };
                            assert_eq!(
                                rejection.reason,
                                ResidentTaskReservationTransitionFailure::NotFound
                            );
                            continuation = *rejection.continuation;
                            task_reservations
                                .replace_key_for_test(&substituted_key, task_key.clone());

                            assert_eq!(clock.next.load(Ordering::SeqCst), clock_calls);
                            assert_eq!(observation_signs.get(), observation_sign_calls);
                            assert_eq!(checkpoint_signs.get(), checkpoint_sign_calls);
                            assert_eq!(task_reservations.outstanding(), 1);
                            let retry_input = invoke_input();
                            let retried = service.continue_recording(
                                &mut ResidentLocalOperationCoordinator::new(
                                    &mut authority,
                                    custody,
                                    anchor,
                                ),
                                continuation,
                                &mut task_reservations,
                                &retry_input,
                                |_| Ok(()),
                                |_, _, _, _| Ok(()),
                                |_, _, _| {
                                    observation_signs.set(observation_signs.get() + 1);
                                    Ok(signature())
                                },
                                |_, _, _| {
                                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                                    Ok(signature())
                                },
                            )
                            .expect(
                                "the consumed provider result must be recordable without replay",
                            );
                            let ResidentProviderTaskPoll::Recorded {
                                result: ProviderOperationInvokeResult::Completed(completed),
                                cancellation,
                            } = retried
                            else {
                                panic!("the admitted success must retain terminal custody")
                            };
                            assert_eq!(cancellation, outcome.cancellation_state());
                            assert!(matches!(
                                completed.observation().1.state(),
                                OperationObservationStateV1::Succeeded { .. }
                            ));
                            assert_eq!(
                                task_reservations.outstanding(),
                                0,
                                "exact durable settlement must release aggregate custody once",
                            );
                            assert_eq!(
                                executor.accept_calls, 1,
                                "recording continuation must not replay provider work",
                            );
                            assert_eq!(port.calls(), 1);
                            return;
                        }
                        if matches!(outcome, ProviderServiceOutcome::AnchorUnknown) {
                            let Ok(ResidentProviderTaskPoll::Custody(custody_continuation)) =
                                result
                            else {
                                panic!("ambiguous terminal anchoring must retain recording custody")
                            };
                            let mut replacement_service =
                                ResidentProviderTaskService::new(ResidentOperationTaskLimits::new(
                                    NonZeroUsize::new(1)
                                        .expect("positive replacement provider-task limit"),
                                ))
                                .expect("replacement provider task service");
                            let mut replacement_reservations =
                                replacement_service.test_reservations();
                            let rejection = match replacement_service.continue_custody(
                                &mut ResidentLocalOperationCoordinator::new(
                                    &mut authority,
                                    custody,
                                    anchor,
                                ),
                                *custody_continuation,
                                &mut replacement_reservations,
                            ) {
                                Err(rejection) => rejection,
                                Ok(_) => panic!("a replacement owner accepted provider custody"),
                            };
                            assert_eq!(
                                rejection.reason,
                                ResidentTaskReservationTransitionFailure::OwnerMismatch
                            );
                            let _retained_custody = rejection.custody;
                            assert_eq!(replacement_reservations.outstanding(), 0);
                            assert_eq!(task_reservations.outstanding(), 1);
                            return;
                        }
                        if outcome.has_concurrent_head() {
                            let Ok(ResidentProviderTaskPoll::ConcurrentHead(conflict)) = result
                            else {
                                panic!("a definite competing head must retain the consumed outcome")
                            };
                            let mut conflict = *conflict;
                            let mut replacement_service =
                                ResidentProviderTaskService::new(ResidentOperationTaskLimits::new(
                                    NonZeroUsize::new(1)
                                        .expect("positive replacement provider-task limit"),
                                ))
                                .expect("replacement provider task service");
                            let mut replacement_reservations =
                                replacement_service.test_reservations();
                            let rejection = match replacement_service
                                .continue_after_concurrent_head(
                                    &mut ResidentLocalOperationCoordinator::new(
                                        &mut authority,
                                        custody,
                                        anchor,
                                    ),
                                    conflict,
                                    &mut replacement_reservations,
                                    &recovered_dispatch,
                                    &invoke_input(),
                                    |_| panic!("foreign conflict reached reconciliation"),
                                    |_, _, _, _| panic!("foreign conflict reached rotation"),
                                    |_, _, _| panic!("foreign conflict reached signing"),
                                    |_, _, _| panic!("foreign conflict reached checkpoint signing"),
                                ) {
                                Err(rejection) => rejection,
                                Ok(_) => panic!("a replacement owner accepted provider conflict"),
                            };
                            assert_eq!(
                                rejection.reason,
                                ResidentTaskReservationTransitionFailure::OwnerMismatch
                            );
                            conflict = *rejection.conflict;
                            assert_eq!(replacement_reservations.outstanding(), 0);
                            assert_eq!(task_reservations.outstanding(), 1);

                            let (cancellation_ref, cancellation) = competing_cancellation
                                .clone()
                                .expect("concurrent cases install one exact competing head");
                            let cancellation_append = prepare_operation_observation_append(
                                OperationObservationTransition::new(
                                    handle_ref,
                                    handle,
                                    &dispatched.0,
                                    &dispatched.1,
                                    &cancellation_ref,
                                    &cancellation,
                                ),
                                &Store,
                            )
                            .unwrap();
                            let (_, cancellation_checkpoint) = custody::complete(
                                cancellation_append,
                                selected.request,
                                (&dispatched_checkpoint.0, &dispatched_checkpoint.1),
                                OperationCustodyDatabaseEvidence::New,
                                70,
                            );
                            let cancellation_checkpoint_ref = record_ref(&format!(
                                "operation/checkpoint-{}",
                                cancellation_checkpoint.domain_sequence().get()
                            ));
                            let second_competing_head = (outcome
                                == ProviderServiceOutcome::ConcurrentHeadTwice)
                                .then(|| {
                                    let reference =
                                        record_ref("provider-service/cancelled-after-recovery");
                                    let observation = issue_observation(
                                        parents.signing,
                                        handle_ref,
                                        handle,
                                        observation_schema.clone(),
                                        cancellation.revision().checked_successor().unwrap(),
                                        Some((&cancellation_ref, &cancellation)),
                                        OperationObservationStateV1::Cancelled {
                                            diagnostic: ImmutableRecordReferenceV1 {
                                                record_ref: record_ref(
                                                    "provider-service/cancellation-diagnostic",
                                                ),
                                                record_sha256: ArtifactDigest::hash_bytes(
                                                    b"provider cancellation diagnostic",
                                                ),
                                            },
                                        },
                                        71,
                                    );
                                    crate::resident::record_projection::coordinate(
                                        &reference,
                                        observation
                                            .canonical_operation_observation_sha256()
                                            .as_str(),
                                    )
                                    .and_then(|coordinate| {
                                        crate::resident::record_projection::record(
                                            coordinate,
                                            &observation,
                                        )
                                    })
                                    .unwrap()
                                });

                            let substituted = RecoveredOperationObservation::fixture(
                                selected.operation,
                                (cancellation_ref.clone(), cancellation.clone()),
                                (cancellation_ref.clone(), cancellation.clone()),
                                (
                                    cancellation_checkpoint_ref.clone(),
                                    cancellation_checkpoint.clone(),
                                ),
                                store_coordinate(),
                            );
                            let substituted_input = invoke_input();
                            let conflict = match service.continue_after_concurrent_head(
                                &mut ResidentLocalOperationCoordinator::new(
                                    &mut authority,
                                    custody,
                                    anchor,
                                ),
                                conflict,
                                &mut task_reservations,
                                &substituted,
                                &substituted_input,
                                |_| Ok(()),
                                |_, _, _, _| Ok(()),
                                |_, _, _| Ok(signature()),
                                |_, _, _| Ok(signature()),
                            ) {
                                Ok(ResidentProviderTaskPoll::ConcurrentHead(conflict)) => *conflict,
                                Err(_) | Ok(_) => {
                                    panic!("recovery must reject a substituted selected dispatch")
                                }
                            };
                            let recovered = RecoveredOperationObservation::fixture(
                                selected.operation,
                                dispatched.clone(),
                                (cancellation_ref, cancellation),
                                (cancellation_checkpoint_ref, cancellation_checkpoint),
                                store_coordinate(),
                            );
                            if let Some(record) = second_competing_head {
                                *competing_head.borrow_mut() = Some(record);
                            }
                            let retry_input = invoke_input();
                            let continued = service.continue_after_concurrent_head(
                                &mut ResidentLocalOperationCoordinator::new(
                                    &mut authority,
                                    custody,
                                    anchor,
                                ),
                                conflict,
                                &mut task_reservations,
                                &recovered,
                                &retry_input,
                                |_| Ok(()),
                                |_, _, _, _| Ok(()),
                                |_, _, _| {
                                    observation_signs.set(observation_signs.get() + 1);
                                    Ok(signature())
                                },
                                |_, _, _| {
                                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                                    Ok(signature())
                                },
                            );
                            if outcome == ProviderServiceOutcome::ConcurrentHeadTwice {
                                let Ok(ResidentProviderTaskPoll::ConcurrentHead(_conflict)) =
                                    continued
                                else {
                                    panic!("a second changed head must retain the consumed outcome")
                                };
                                assert_eq!(task_reservations.outstanding(), 1);
                                return;
                            }
                            match (outcome, continued) {
                                (
                                    ProviderServiceOutcome::ConcurrentSuccess,
                                    Ok(ResidentProviderTaskPoll::Recorded {
                                        result: ProviderOperationInvokeResult::Completed(completed),
                                        ..
                                    }),
                                ) => assert!(matches!(
                                    completed.observation().1.state(),
                                    OperationObservationStateV1::Succeeded { .. }
                                )),
                                (
                                    ProviderServiceOutcome::ConcurrentFailure,
                                    Ok(ResidentProviderTaskPoll::Recorded {
                                        result: ProviderOperationInvokeResult::Completed(completed),
                                        ..
                                    }),
                                ) => {
                                    assert!(matches!(
                                        completed.observation().1.state(),
                                        OperationObservationStateV1::Failed { .. }
                                    ));
                                    assert!(completed.diagnostic().is_some());
                                }
                                (
                                    ProviderServiceOutcome::ConcurrentUncertainty,
                                    Ok(ResidentProviderTaskPoll::Recorded {
                                        result:
                                            ProviderOperationInvokeResult::OutcomeUnknown(uncertain),
                                        ..
                                    }),
                                ) => assert!(matches!(
                                    uncertain.observation().1.state(),
                                    OperationObservationStateV1::OutcomeUnknown { .. }
                                )),
                                (
                                    outcome,
                                    Ok(ResidentProviderTaskPoll::Preparation { reason, .. }),
                                ) => {
                                    panic!("{outcome:?} did not follow cancellation: {reason:?}")
                                }
                                (outcome, Err(failure)) => {
                                    panic!("{outcome:?} did not follow cancellation: {failure:?}")
                                }
                                (
                                    outcome,
                                    Ok(ResidentProviderTaskPoll::Recorded {
                                        result: ProviderOperationInvokeResult::Completed(value),
                                        ..
                                    }),
                                ) => {
                                    panic!(
                                        "{outcome:?} produced {:?}",
                                        value.observation().1.state()
                                    )
                                }
                                (
                                    outcome,
                                    Ok(ResidentProviderTaskPoll::Recorded {
                                        result: ProviderOperationInvokeResult::OutcomeUnknown(value),
                                        ..
                                    }),
                                ) => panic!(
                                    "{outcome:?} produced {:?}",
                                    value.observation().1.state()
                                ),
                                (outcome, Ok(_)) => {
                                    panic!("{outcome:?} did not finish its retained settlement")
                                }
                            }
                            return;
                        }
                        let Ok(ResidentProviderTaskPoll::Recorded {
                            result: ProviderOperationInvokeResult::Completed(completed),
                            cancellation,
                        }) = result
                        else {
                            if matches!(
                                outcome,
                                ProviderServiceOutcome::Rejected
                                    | ProviderServiceOutcome::ResultSignatureRejected
                                    | ProviderServiceOutcome::FailureSignatureRejected
                                    | ProviderServiceOutcome::AssociationRejected
                                    | ProviderServiceOutcome::Unavailable
                            ) {
                                let Ok(ResidentProviderTaskPoll::Recorded {
                                    result: ProviderOperationInvokeResult::OutcomeUnknown(uncertain),
                                    cancellation,
                                }) = result
                                else {
                                    panic!(
                                        "inconclusive provider result must reach uncertainty custody"
                                    )
                                };
                                assert_eq!(cancellation, outcome.cancellation_state());
                                assert!(matches!(
                                    uncertain.observation().1.state(),
                                    OperationObservationStateV1::OutcomeUnknown {
                                        reconciliation,
                                        ..
                                    } if reconciliation.record_ref == record_ref("provider-service/reconciliation")
                                ));
                                assert!(!uncertain.observation().1.state().is_terminal());
                                assert_eq!(
                                    uncertain.diagnostic().wire().retry,
                                    DiagnosticRetryV1::ReconcileFirst
                                );
                                assert_eq!(
                                    uncertain.diagnostic().wire().effect,
                                    DiagnosticEffectCertaintyV1::NotApplicable
                                );
                                assert_eq!(
                                    uncertain.custody().database_evidence(),
                                    OperationCustodyDatabaseEvidence::New
                                );
                                assert!(uncertain
                                    .custody()
                                    .candidate()
                                    .record(
                                        pale_ports::OperationCustodyRecordKind::CapabilityResult
                                    )
                                    .is_none());
                                assert!(uncertain
                                    .custody()
                                    .candidate()
                                    .record(pale_ports::OperationCustodyRecordKind::AdapterFailure)
                                    .is_none());
                                assert!(match outcome {
                                    ProviderServiceOutcome::Rejected => matches!(
                                        uncertain.uncertainty(),
                                        crate::resident::ProviderOperationUncertainty::ResponseRejected(_)
                                    ),
                                    ProviderServiceOutcome::ResultSignatureRejected => matches!(
                                        uncertain.uncertainty(),
                                        crate::resident::ProviderOperationUncertainty::ResponseRejected(_)
                                    ),
                                    ProviderServiceOutcome::FailureSignatureRejected => matches!(
                                        uncertain.uncertainty(),
                                        crate::resident::ProviderOperationUncertainty::ResponseRejected(_)
                                    ),
                                    ProviderServiceOutcome::AssociationRejected => matches!(
                                        uncertain.uncertainty(),
                                        crate::resident::ProviderOperationUncertainty::ResponseAssociation { .. }
                                    ),
                                    ProviderServiceOutcome::Unavailable => matches!(
                                        uncertain.uncertainty(),
                                        crate::resident::ProviderOperationUncertainty::Unavailable(_)
                                    ),
                                    _ => false,
                                });
                                return;
                            }
                            panic!("fresh provider result must reach terminal custody")
                        };
                        assert_eq!(cancellation, outcome.cancellation_state());
                        match outcome {
                            ProviderServiceOutcome::AggregateSaturated
                            | ProviderServiceOutcome::AdmissionRejected
                            | ProviderServiceOutcome::ReconciliationAlias => unreachable!(),
                            ProviderServiceOutcome::Succeeded
                            | ProviderServiceOutcome::ComposedSucceeded
                            | ProviderServiceOutcome::ComposedRefusalContinuation
                            | ProviderServiceOutcome::ComposedCancellation
                            | ProviderServiceOutcome::NotificationOnly
                            | ProviderServiceOutcome::ParallelServiceIsolation => {
                                assert!(matches!(
                                    completed.observation().1.state(),
                                    OperationObservationStateV1::Succeeded { .. }
                                ));
                                assert_eq!(
                                    completed.observation().1.predecessor_ref(),
                                    &Some(record_ref("provider-service/dispatched"))
                                );
                                assert!(completed.diagnostic().is_none());
                            }
                            ProviderServiceOutcome::Failed => {
                                assert!(matches!(
                                    completed.observation().1.state(),
                                    OperationObservationStateV1::Failed {
                                        result: Some(_),
                                        ..
                                    }
                                ));
                                assert!(completed.diagnostic().is_some());
                                assert!(matches!(
                                    completed.outcome(),
                                    crate::provider_invocation::AdmittedProviderInvocationOutcome::Failed(
                                        _
                                    )
                                ));
                            }
                            ProviderServiceOutcome::Rejected
                            | ProviderServiceOutcome::ResultSignatureRejected
                            | ProviderServiceOutcome::FailureSignatureRejected
                            | ProviderServiceOutcome::AssociationRejected
                            | ProviderServiceOutcome::Unavailable
                            | ProviderServiceOutcome::AnchorUnknown
                            | ProviderServiceOutcome::RecordingRetry
                            | ProviderServiceOutcome::ConcurrentSuccess
                            | ProviderServiceOutcome::ConcurrentFailure
                            | ProviderServiceOutcome::ConcurrentUncertainty
                            | ProviderServiceOutcome::ConcurrentHeadTwice
                            | ProviderServiceOutcome::ComposedNonRetryableRefusals
                            | ProviderServiceOutcome::ComposedConcurrentSuccess
                            | ProviderServiceOutcome::ComposedConcurrentFailure
                            | ProviderServiceOutcome::ComposedConcurrentUncertainty
                            | ProviderServiceOutcome::ColdRestart
                            | ProviderServiceOutcome::LostAfterAcceptance => unreachable!(),
                        }
                        assert_eq!(
                            completed.custody().database_evidence(),
                            OperationCustodyDatabaseEvidence::New
                        );
                    }
                },
            );
            assert_eq!(
                observation_signs.get(),
                if outcome == ProviderServiceOutcome::AggregateSaturated {
                    0
                } else if outcome.is_composed_concurrent() {
                    3
                } else if matches!(outcome, ProviderServiceOutcome::RecordingRetry) {
                    expected_appends + 1
                } else {
                    expected_appends
                }
            );
            assert_eq!(
                checkpoint_signs.get(),
                if outcome == ProviderServiceOutcome::AggregateSaturated {
                    0
                } else if outcome.is_composed_concurrent() {
                    3
                } else {
                    expected_appends
                }
            );
            assert_eq!(
                clock.next.load(Ordering::SeqCst),
                if outcome == ProviderServiceOutcome::AggregateSaturated {
                    0
                } else if matches!(
                    outcome,
                    ProviderServiceOutcome::AdmissionRejected
                        | ProviderServiceOutcome::ReconciliationAlias
                ) {
                    3
                } else if matches!(
                    outcome,
                    ProviderServiceOutcome::ComposedCancellation
                        | ProviderServiceOutcome::ComposedNonRetryableRefusals
                ) {
                    6
                } else if stale_release || outcome == ProviderServiceOutcome::ColdRestart {
                    4
                } else if outcome == ProviderServiceOutcome::LostAfterAcceptance
                    || outcome.is_composed_concurrent()
                {
                    7
                } else if matches!(outcome, ProviderServiceOutcome::RecordingRetry)
                    || outcome.has_concurrent_head()
                {
                    8
                } else {
                    7
                }
            );
            port.calls()
        },
    )
}

#[test]
fn resident_operation_service_rejects_aggregate_saturation_before_provider_retention() {
    let mut port = None;
    assert_eq!(
        provider_service_case(false, ProviderServiceOutcome::AggregateSaturated, &mut port,),
        0
    );
}

#[test]
fn resident_local_operation_coordinator_returns_unopened_custody_after_input_rejection() {
    let mut port = None;
    assert_eq!(
        provider_service_case(false, ProviderServiceOutcome::AdmissionRejected, &mut port,),
        0
    );
}

#[test]
fn resident_local_operation_coordinator_rejects_a_reconciliation_alias_before_the_provider_call() {
    let mut port = None;
    assert_eq!(
        provider_service_case(
            false,
            ProviderServiceOutcome::ReconciliationAlias,
            &mut port,
        ),
        0
    );
}

#[test]
fn resident_local_operation_coordinator_calls_once_and_stale_release_adds_no_call() {
    let mut port = None;
    assert_eq!(
        provider_service_case(false, ProviderServiceOutcome::Succeeded, &mut port),
        1
    );
    assert_eq!(
        provider_service_case(true, ProviderServiceOutcome::Succeeded, &mut port),
        1
    );
}

#[test]
fn accepted_provider_task_survives_resident_operation_host_service_reborrows() {
    let mut port = None;
    assert_eq!(
        provider_service_case(false, ProviderServiceOutcome::ComposedSucceeded, &mut port,),
        1
    );
}

#[test]
fn resident_operation_service_continues_unopened_executor_refusal_without_reretention() {
    let mut port = None;
    assert_eq!(
        provider_service_case(
            false,
            ProviderServiceOutcome::ComposedRefusalContinuation,
            &mut port,
        ),
        1
    );
}

#[test]
fn resident_operation_service_does_not_retry_preparation_or_duplicate_refusals() {
    let mut port = None;
    assert_eq!(
        provider_service_case(
            false,
            ProviderServiceOutcome::ComposedNonRetryableRefusals,
            &mut port,
        ),
        0
    );
}

#[test]
fn resident_operation_service_notifies_only_its_accepted_provider_after_custody() {
    let mut port = None;
    assert_eq!(
        provider_service_case(
            false,
            ProviderServiceOutcome::ComposedCancellation,
            &mut port,
        ),
        0
    );
}

#[test]
fn durable_cancellation_and_late_completion_cross_facade_reborrow() {
    for outcome in [
        ProviderServiceOutcome::ComposedConcurrentSuccess,
        ProviderServiceOutcome::ComposedConcurrentFailure,
        ProviderServiceOutcome::ComposedConcurrentUncertainty,
    ] {
        let mut port = None;
        assert_eq!(provider_service_case(false, outcome, &mut port), 1);
    }
}

#[test]
fn process_local_cancellation_notification_does_not_fabricate_a_durable_head() {
    let mut port = None;
    assert_eq!(
        provider_service_case(false, ProviderServiceOutcome::NotificationOnly, &mut port),
        1
    );
}

#[test]
fn parallel_live_provider_service_cannot_claim_or_recover_another_live_task() {
    let mut port = None;
    assert_eq!(
        provider_service_case(
            false,
            ProviderServiceOutcome::ParallelServiceIsolation,
            &mut port,
        ),
        1
    );
}

#[test]
fn resident_local_operation_coordinator_commits_safe_failure_result_diagnostic_and_terminal_state()
{
    let mut port = None;
    assert_eq!(
        provider_service_case(false, ProviderServiceOutcome::Failed, &mut port),
        1
    );
}

#[test]
fn resident_local_operation_coordinator_commits_reconciliation_for_rejected_response_without_replay(
) {
    let mut port = None;
    assert_eq!(
        provider_service_case(false, ProviderServiceOutcome::Rejected, &mut port),
        1
    );
}

#[test]
fn resident_local_operation_coordinator_commits_reconciliation_for_rejected_result_signature_without_replay(
) {
    let mut port = None;
    assert_eq!(
        provider_service_case(
            false,
            ProviderServiceOutcome::ResultSignatureRejected,
            &mut port,
        ),
        1
    );
}

#[test]
fn resident_local_operation_coordinator_commits_reconciliation_for_rejected_failure_signature_without_replay(
) {
    let mut port = None;
    assert_eq!(
        provider_service_case(
            false,
            ProviderServiceOutcome::FailureSignatureRejected,
            &mut port,
        ),
        1
    );
}

#[test]
fn resident_local_operation_coordinator_commits_reconciliation_for_unavailable_evidence() {
    let mut port = None;
    assert_eq!(
        provider_service_case(false, ProviderServiceOutcome::Unavailable, &mut port),
        1
    );
}

#[test]
fn resident_local_operation_coordinator_commits_reconciliation_for_unassociated_evidence() {
    let mut port = None;
    assert_eq!(
        provider_service_case(
            false,
            ProviderServiceOutcome::AssociationRejected,
            &mut port,
        ),
        1
    );
}

#[test]
fn resident_local_operation_coordinator_preserves_ambiguous_terminal_anchor_without_replay_grant() {
    let mut port = None;
    assert_eq!(
        provider_service_case(false, ProviderServiceOutcome::AnchorUnknown, &mut port),
        1
    );
}

#[test]
fn resident_local_operation_coordinator_retries_recording_without_reopening_the_provider() {
    let mut port = None;
    assert_eq!(
        provider_service_case(false, ProviderServiceOutcome::RecordingRetry, &mut port),
        1
    );
}

#[test]
fn resident_local_operation_coordinator_records_consumed_outcomes_after_cancellation() {
    for outcome in [
        ProviderServiceOutcome::ConcurrentSuccess,
        ProviderServiceOutcome::ConcurrentFailure,
        ProviderServiceOutcome::ConcurrentUncertainty,
        ProviderServiceOutcome::ConcurrentHeadTwice,
    ] {
        let mut port = None;
        assert_eq!(provider_service_case(false, outcome, &mut port), 1);
    }
}

#[test]
fn resident_local_operation_coordinator_records_cold_restart_without_replaying_the_provider() {
    let mut port = None;
    assert_eq!(
        provider_service_case(false, ProviderServiceOutcome::ColdRestart, &mut port),
        0
    );
}

#[test]
fn lost_provider_task_crosses_facade_reborrow_without_replay() {
    let mut port = None;
    assert_eq!(
        provider_service_case(
            false,
            ProviderServiceOutcome::LostAfterAcceptance,
            &mut port,
        ),
        0
    );
}

#[test]
fn accepted_provider_work_does_not_block_the_resident_and_calls_exactly_once() {
    with_selection_context(|selection, selected, _, _| {
        let request = crate::resident::operation_dispatch::OwnedProviderInvocationRequest::capture(
            selection
                .provider_request(selected)
                .expect("provider request"),
        );
        let task_key = crate::resident::task_owner::ResidentOperationTaskKey::new(
            store_coordinate(),
            ImmutableRecordReferenceV1 {
                record_ref: record_ref("provider-task-owner/dispatched"),
                record_sha256: ArtifactDigest::hash_bytes(b"provider task owner dispatched"),
            },
        );
        let mut owner = ResidentOperationTaskOwner::new(ResidentOperationTaskLimits::new(
            NonZeroUsize::new(1).expect("positive task limit"),
        ))
        .expect("task owner");
        let provider = Arc::new(BlockingProvider::new(false));
        let mut executor = BlockingProviderExecutor::new(Arc::clone(&provider));

        let accepted = owner.try_accept(
            task_key.clone(),
            "retained application basis",
            request,
            &mut executor,
        );
        assert!(accepted.is_ok());
        provider.wait_until_started();
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
        assert_eq!(owner.in_flight(), 1);
        assert!(owner.try_completion(&mut executor).is_none());
        assert_eq!(
            owner.note_cancellation_notification(&task_key),
            ResidentOperationTaskCancellationNotification::Notified
        );

        provider.release();
        let deadline = Instant::now() + Duration::from_secs(2);
        let completion = loop {
            if let Some(completion) = owner.try_completion(&mut executor) {
                break completion;
            }
            assert!(
                Instant::now() < deadline,
                "provider completion wait elapsed"
            );
            std::thread::yield_now();
        };
        let ResidentOperationTaskJoin::Completed {
            basis,
            output,
            cancellation,
            ..
        } = completion
        else {
            panic!("accepted provider work must join to its exact basis")
        };
        assert_eq!(basis, "retained application basis");
        assert!(matches!(
            output,
            Ok(ProviderInvocationObservation::Unavailable(
                ProviderInvocationUnavailability::EvidenceUnknown
            ))
        ));
        assert_eq!(
            cancellation,
            ResidentOperationTaskCancellationState::Notified
        );
        assert_eq!(owner.in_flight(), 0);
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
        assert!(owner.try_completion(&mut executor).is_none());
    });
}

#[test]
fn provider_panic_returns_lost_basis_without_replay_or_stuck_occupancy() {
    with_selection_context(|selection, selected, _, _| {
        let request = crate::resident::operation_dispatch::OwnedProviderInvocationRequest::capture(
            selection
                .provider_request(selected)
                .expect("provider request"),
        );
        let task_key = crate::resident::task_owner::ResidentOperationTaskKey::new(
            store_coordinate(),
            ImmutableRecordReferenceV1 {
                record_ref: record_ref("provider-task-owner/panic"),
                record_sha256: ArtifactDigest::hash_bytes(b"provider task owner panic"),
            },
        );
        let mut owner = ResidentOperationTaskOwner::new(ResidentOperationTaskLimits::new(
            NonZeroUsize::new(1).expect("positive task limit"),
        ))
        .expect("task owner");
        let provider = Arc::new(BlockingProvider::new(true));
        let mut executor = BlockingProviderExecutor::new(Arc::clone(&provider));

        assert!(owner
            .try_accept(
                task_key.clone(),
                "retained application basis",
                request,
                &mut executor,
            )
            .is_ok());
        provider.wait_until_started();
        provider.release();

        let deadline = Instant::now() + Duration::from_secs(2);
        let completion = loop {
            if let Some(completion) = owner.try_completion(&mut executor) {
                break completion;
            }
            assert!(Instant::now() < deadline, "provider loss wait elapsed");
            std::thread::yield_now();
        };
        let ResidentOperationTaskJoin::Lost { basis, key, .. } = completion else {
            panic!("a panicked provider task must return its basis as lost work")
        };
        assert_eq!(basis, "retained application basis");
        assert_eq!(key, task_key);
        assert_eq!(owner.in_flight(), 0);
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
        assert!(owner.try_completion(&mut executor).is_none());
    });
}
