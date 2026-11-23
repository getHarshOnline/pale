//! Hostile laws for process-local provider route custody.

use super::*;
use crate::local_operation_start_fixture::{
    checkpoint_authority, checkpoint_preimage, checkpoint_preimage_at_store, issue_observation,
    record_ref, schema_digest, signature,
};
use crate::resident::operation_dispatch::selection::tests::provider as provider_fixture;
use crate::resident::operation_dispatch::selection::tests::provider_dispatch::{
    cancellation_request as retained_provider_cancellation_request,
    request_composed_provider_cancellation, with_delegate_dispatch_input_named,
    A2aTaskOwnerCustodyModel,
};
use crate::resident::operation_dispatch::selection::tests::{
    with_provider_selection_recovery_context, with_provider_selection_recovery_context_at_store,
    with_provider_selection_restart_context,
};
use crate::resident::operation_dispatch::OperationAttemptContext;
use crate::resident::operation_dispatch::PreparedProviderOperationCall;
use crate::resident::operation_dispatch::ProviderOperationInput;
use crate::resident::operation_dispatch::ProviderOperationRetentionFailure;
use crate::resident::operation_observation_custody::tests::native_execution as custody;
use crate::resident::process_host::operation_request_host::{
    ResidentOperationRequestHost, ResidentOperationRequestKind, ResidentOperationRequestOutcome,
    ResidentOperationRequestResolution,
};
use crate::resident::task_owner::{
    ResidentOperationTask, ResidentOperationTaskCompletion, ResidentOperationTaskKey,
    ResidentOperationTaskLimits, ResidentOperationTaskOwnerInstance,
};
use crate::resident::task_reservation::ResidentTaskReservationPhase;
use crate::resident::{
    prepare_operation_observation_append, OperationCustodyAnchorEvidence,
    OperationCustodyDatabaseEvidence, OperationObservationTransition,
};
use crate::resident::{
    AdmittedCompiledGeneration, IssuedLocalOperation, OperationAttemptError,
    OperationCheckpointInput, OperationObservationAuthorityStoreRejection,
    OperationObservationAuthorityStoreResolver,
};
use crate::session_admission_fixture::{LocalStartContext, ProvingAuthority};
use pale_core::records::{
    CapabilityAttemptIdV1, CapabilityResultAdmissionContextV1, CapabilitySuccessAdmissionContextV1,
    ControlCheckpointIdV1, ControlStateCheckpointWireV1, ImmutableRecordReferenceV1,
    OperationActivePhaseV1, OperationObservationAdmissionContextV1, OperationObservationRevisionV1,
    OperationObservationStateV1, SignedResidentAdmissionContextV1, ADAPTER_FAILURE_V1_SCHEMA_ID,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    AccumulatorPreparationLimits, AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition,
    AntiRollbackAnchorError, AntiRollbackAnchorErrorKind, AntiRollbackAnchorKey,
    AntiRollbackAnchorObservation, AntiRollbackAnchorPort, ControlRecord, ControlRecordBytes,
    ControlStoreError, OperationCustodyIndexKey, OperationCustodyIndexQuery, OperationCustodyPort,
    PreparedAntiRollbackAnchorTransition, ProviderInvocationObservation, ProviderInvocationPort,
    ProviderInvocationRequest, ProviderInvocationSuccessWire, ResidentOperationFrameRequest,
    ResidentOperationFrameResponse, SessionLeaseBody, SessionLeaseBodyRead,
    SessionLeaseBodyRetention, SessionLeaseCustodyPort, WallClockSourceError, WallClockSourcePort,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

type ProviderTaskOutput = Result<
    ProviderInvocationObservation,
    crate::provider_invocation::ProviderInvocationAdmissionError,
>;

impl<WriterGuard> ResidentProcessHost<'_, WriterGuard> {
    fn continue_provider_custody_with_finalization_mismatch_for_test<Authority>(
        &mut self,
        recording: ResidentProcessProviderRecording<'static>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        substituted_store: AuthorityStoreCoordinate,
    ) -> ResidentProcessProviderRecording<'static> {
        let ResidentProcessProviderRecording::Pending {
            route,
            progress: ResidentProviderTaskPoll::Custody(custody),
        } = recording
        else {
            panic!("test fixture requires provider recording custody")
        };
        let selector = route.selector.clone();
        assert!(self
            .validate_provider_route_access(&route, access.store)
            .is_ok());
        let progress = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.continue_provider_custody_with_finalization_mismatch_for_test(
                        *custody,
                        substituted_store,
                    )
                },
            )
            .expect("test fixture selects the exact provider deployment")
            .unwrap_or_else(|_| panic!("test fixture retains exact provider custody"));
        ResidentProcessProviderRecording::Pending { route, progress }
    }

    fn restore_provider_finalization_for_test<Authority>(
        &mut self,
        recording: ResidentProcessProviderRecording<'static>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
    ) -> ResidentProcessProviderRecording<'static> {
        let ResidentProcessProviderRecording::Pending {
            route,
            progress:
                ResidentProviderTaskPoll::Finalization {
                    finalization,
                    reason: _,
                },
        } = recording
        else {
            panic!("test fixture requires provider finalization")
        };
        let selector = route.selector.clone();
        assert!(self
            .validate_provider_route_access(&route, access.store)
            .is_ok());
        let progress = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.restore_provider_finalization_for_test(*finalization)
                },
            )
            .expect("test fixture selects the exact provider deployment")
            .unwrap_or_else(|_| panic!("test fixture retains exact provider finalization"));
        self.finish_provider_progress(route, progress)
    }
}

fn replace_recording_selector<'operation>(
    recording: ResidentProcessProviderRecording<'operation>,
    selector: ResidentDeploymentAuthoritySelector,
) -> (
    ResidentProcessProviderRecording<'operation>,
    ResidentDeploymentAuthoritySelector,
) {
    let ResidentProcessProviderRecording::Pending {
        mut route,
        progress,
    } = recording
    else {
        panic!("test fixture requires pending provider recording")
    };
    let previous = std::mem::replace(&mut route.selector, selector);
    (
        ResidentProcessProviderRecording::Pending { route, progress },
        previous,
    )
}

struct UnusedProviderEvidence;

impl crate::provider_invocation::ProviderInvocationEvidenceAdmission for UnusedProviderEvidence {
    fn admit_evidence_frontier(
        &self,
        _: &pale_ports::ProviderInvocationEvidenceFrontier,
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("wrong-store selection must precede evidence admission")
    }

    fn resolve_immutable(&self, _: &ResidentRecordRefV1) -> Option<Vec<u8>> {
        panic!("wrong-store selection must precede evidence resolution")
    }

    fn admit_payload(
        &self,
        _: &SchemaId,
        _: &SchemaDigest,
        _: &[u8],
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("wrong-store selection must precede payload admission")
    }

    fn admit_failure_class(
        &self,
        _: pale_core::records::AdapterFailureClassV1,
        _: Option<UnixTimeV1>,
        _: &pale_core::records::RequestBindingV1,
        _: &pale_core::records::CapabilityAttemptIdV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("wrong-store selection must precede failure admission")
    }

    fn admit_absence_evidence(
        &self,
        _: &ResidentRecordRefV1,
        _: &ArtifactDigest,
        _: &[u8],
        _: &pale_core::records::RequestBindingV1,
        _: &pale_core::records::CapabilityAttemptIdV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("wrong-store selection must precede absence admission")
    }

    fn verify_result_signature(
        &self,
        _: &[u8],
        _: &pale_core::records::CapabilityResultPreimageV1,
        _: &pale_core::records::CapabilityResultDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("wrong-store selection must precede result authentication")
    }

    fn verify_failure_signature(
        &self,
        _: &[u8],
        _: &pale_core::records::AdapterFailurePreimageV1,
        _: &pale_core::records::AdapterFailureDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("wrong-store selection must precede failure authentication")
    }
}

pub(super) struct ProviderClock {
    times: Vec<u64>,
    next: AtomicUsize,
}

impl ProviderClock {
    pub(super) fn reads(&self) -> usize {
        self.next.load(Ordering::SeqCst)
    }
}

impl WallClockSourcePort for ProviderClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        let index = self.next.fetch_add(1, Ordering::SeqCst);
        Ok(UnixTimeV1::new(self.times[index]).expect("provider fixture time"))
    }
}

pub(super) struct ProviderStore {
    store: AuthorityStoreCoordinate,
    calls: AtomicUsize,
}

impl ProviderStore {
    fn new(store: AuthorityStoreCoordinate) -> Self {
        Self {
            store,
            calls: AtomicUsize::new(0),
        }
    }

    pub(super) fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl OperationObservationAuthorityStoreResolver for ProviderStore {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.store.clone())
    }
}

struct ProviderSessionRead(SessionLeaseBody);

impl SessionLeaseCustodyPort for ProviderSessionRead {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("provider launch does not renew its historical session")
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

fn provider_session_read(
    parents: &crate::local_operation_start_fixture::RecoveryParents<'_>,
    store: AuthorityStoreCoordinate,
) -> ProviderSessionRead {
    let session = parents.session.lease();
    ProviderSessionRead(SessionLeaseBody::new(
        store,
        session.session_admission_lease_digest().clone(),
        session.schema_id().clone(),
        session.schema_sha256().clone(),
        ControlRecordBytes::new(
            pale_core::canon::to_canonical_string(
                &pale_core::records::SessionAdmissionLeaseWireV1::from(session),
            )
            .expect("session lease encoding")
            .into_bytes(),
        )
        .expect("session lease bytes"),
    ))
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum ProviderNextAcceptance {
    #[default]
    Accept,
    Refuse,
}

#[derive(Default)]
struct ProviderExecutor {
    accepted: VecDeque<ResidentOperationTask<OwnedProviderInvocationRequest>>,
    completions: VecDeque<
        ResidentOperationTaskCompletion<OwnedProviderInvocationRequest, ProviderTaskOutput>,
    >,
    next_acceptance: ProviderNextAcceptance,
    substitute_next_key: Option<ResidentOperationTaskKey>,
    completion_polls: usize,
}

impl ProviderExecutor {
    fn refuse_next(&mut self) {
        self.next_acceptance = ProviderNextAcceptance::Refuse;
    }

    fn lose_next(&mut self) {
        let task = self.accepted.pop_front().expect("accepted provider task");
        self.completions.push_back(task.into_lost());
    }

    fn complete_next_unavailable(&mut self) {
        self.complete_next(Ok(ProviderInvocationObservation::Unavailable(
            pale_ports::ProviderInvocationUnavailability::EvidenceUnknown,
        )));
    }

    fn complete_next(&mut self, output: ProviderTaskOutput) {
        let task = self.accepted.pop_front().expect("accepted provider task");
        self.completions
            .push_back(ResidentOperationTaskCompletion::Completed { task, output });
    }

    fn execute_next<Provider: ProviderInvocationPort>(&mut self, provider: &Provider) {
        let task = self.accepted.pop_front().expect("accepted provider task");
        let output = crate::resident::task_owner::execute_provider_task(&task, provider);
        self.completions
            .push_back(ResidentOperationTaskCompletion::Completed { task, output });
    }

    fn complete_next_with_substituted_key(&mut self, key: ResidentOperationTaskKey) {
        let task = self.accepted.pop_front().expect("accepted provider task");
        let task = crate::resident::task_owner::tests::substitute_task_key(task, key);
        self.completions
            .push_back(ResidentOperationTaskCompletion::Completed {
                task,
                output: Ok(ProviderInvocationObservation::Unavailable(
                    pale_ports::ProviderInvocationUnavailability::EvidenceUnknown,
                )),
            });
    }
}

impl ResidentOperationTaskExecutor<OwnedProviderInvocationRequest, ProviderTaskOutput>
    for ProviderExecutor
{
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<OwnedProviderInvocationRequest>,
    ) -> Result<(), ResidentOperationTask<OwnedProviderInvocationRequest>> {
        if self.next_acceptance == ProviderNextAcceptance::Refuse {
            self.next_acceptance = ProviderNextAcceptance::Accept;
            return Err(task);
        }
        if let Some(key) = self.substitute_next_key.take() {
            return Err(crate::resident::task_owner::tests::substitute_task_key(
                task, key,
            ));
        }
        self.accepted.push_back(task);
        Ok(())
    }

    fn try_completion(
        &mut self,
    ) -> Option<ResidentOperationTaskCompletion<OwnedProviderInvocationRequest, ProviderTaskOutput>>
    {
        self.completion_polls += 1;
        self.completions.pop_front()
    }
}

struct AmbiguousProviderAnchor<'a> {
    inner: &'a mut dyn AntiRollbackAnchorPort,
}

impl AntiRollbackAnchorPort for AmbiguousProviderAnchor<'_> {
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
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::Unavailable,
        ))
    }
}

fn provider_task_key(
    store: AuthorityStoreCoordinate,
    label: &str,
    digest_byte: char,
) -> ResidentOperationTaskKey {
    ResidentOperationTaskKey::new(
        store,
        ImmutableRecordReferenceV1 {
            record_ref: ResidentRecordRefV1::parse(label).expect("provider dispatch reference"),
            record_sha256: ArtifactDigest::parse(digest_byte.to_string().repeat(64))
                .expect("provider dispatch digest"),
        },
    )
}

struct LaunchedProviderEvidence {
    key: ResidentOperationTaskKey,
    resolution: (ResidentRecordRefV1, OperationObservationV1),
    preparation: (ResidentRecordRefV1, OperationObservationV1),
    dispatched: (ResidentRecordRefV1, OperationObservationV1),
}

#[test]
fn recovered_pairwise_delegate_launches_only_its_exact_provider_dispatch() {
    let model = A2aTaskOwnerCustodyModel {
        clock_times: vec![63, 64, 65, 66, 67, 68, 69, 70],
        expected_preparations: 4,
        expected_commits: 4,
        expected_read_backs: 4,
        expected_anchor_advances: 4,
        expected_anchor_observations: 4,
        unknown_exact_commits: &[],
        unavailable_read_backs: &[],
        unavailable_anchor_advances: &[],
        unknown_exact_anchor_advances: &[],
        unknown_failed_observation_anchor_advances: &[],
        later_anchor_observations: Vec::new(),
        fail_a2a_preflight_once: false,
    };
    with_delegate_dispatch_input_named(
        model,
        "pairwise-provider-composition",
        |dispatch,
         recovered,
         completed_context,
         checkpoint_schema,
         checkpoint_authority,
         clock,
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
            let mut host = super::restart::host_with_generation(
                &application_store,
                &selector,
                &selected_store,
                restart_generation,
            );
            let selection_ref = dispatch.operation.context.selection_ref.clone();
            let observation_ref = dispatch.operation.context.observation_ref.clone();
            let maximum_selection_bytes = dispatch.operation.context.maximum_selection_bytes;
            let signing = dispatch.signing;
            let checkpoint_input = |phase: &str| OperationCheckpointInput {
                reference: record_ref(&format!("pairwise-provider-composition/checkpoint-{phase}")),
                id: ControlCheckpointIdV1::parse(format!(
                    "pairwise-provider-composition-checkpoint-{phase}"
                ))
                .expect("provider checkpoint identity"),
            };
            let invocation = ProviderOperationInvokeInput {
                current: parents.current,
                signing,
                terminal_checkpoint: checkpoint_input("terminal"),
                result_ref: record_ref("pairwise-provider-composition/result"),
                diagnostic_ref: record_ref("pairwise-provider-composition/diagnostic"),
                reconciliation: pale_core::records::ReconciliationFrontierReferenceV1 {
                    record_ref: record_ref("pairwise-provider-composition/reconciliation"),
                    record_sha256: pale_core::records::ReconciliationFrontierDigest::parse(
                        "d".repeat(64),
                    )
                    .expect("reconciliation digest"),
                },
                terminal_ref: record_ref("pairwise-provider-composition/terminal"),
                checkpoint_schema,
                checkpoint_authority,
                clock,
                resolve_store,
                maximum_record_bytes: 65_536,
                maximum_diagnostic_bytes: 65_536,
                accumulator_limits: AccumulatorPreparationLimits::new(1, 1024, 1024, 512, 65_536)
                    .expect("provider invocation limits"),
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
            let mut executor = ProviderExecutor::default();
            let launched_result = RefCell::new(None);
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
                        let launched = host
                            .launch_recovered_pairwise_provider(
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
                                &invocation,
                                &mut executor,
                                |_, _, _, _| Ok(()),
                                |_, _, _| Ok(signature()),
                                |_, _, _| Ok(signature()),
                            )
                            .map_err(|_| {
                                pale_ports::ResidentOperationFrameFailure::ResidentUnavailable
                            })?;
                        if launched.is_err() {
                            return Err(
                                pale_ports::ResidentOperationFrameFailure::ResidentUnavailable,
                            );
                        }
                        launched_result.replace(Some(launched));
                        Ok(ResidentOperationRequestOutcome::Pending(locator.clone()))
                    },
                )
                .expect("private request host advances the selected provider once");
            assert_eq!(
                response,
                ResidentOperationFrameResponse::Pending {
                    locator: locator.clone()
                }
            );
            let launched = launched_result
                .into_inner()
                .expect("the real provider continuation was retained");
            let result = match launched {
                Ok(key) => key,
                Err(failure) => match *failure {
                    ResidentProcessProviderLaunchFailure::Selection { reason, .. } => {
                        panic!("exact provider dispatch selection: {reason:?}")
                    }
                    ResidentProcessProviderLaunchFailure::RouteSaturated(_) => {
                        panic!("exact provider route saturated")
                    }
                    ResidentProcessProviderLaunchFailure::Retention { .. } => {
                        panic!("exact provider dispatch retention")
                    }
                    ResidentProcessProviderLaunchFailure::RetainedStoreAssociation { .. } => {
                        panic!("exact retained provider store association")
                    }
                    ResidentProcessProviderLaunchFailure::Acceptance { failure, .. } => {
                        match failure {
                            ResidentProviderTaskAcceptanceFailure::AggregateSaturated(_) => {
                                panic!("exact provider aggregate saturated")
                            }
                            ResidentProviderTaskAcceptanceFailure::Preflight(reason) => {
                                panic!("exact provider preflight: {reason:?}")
                            }
                            ResidentProviderTaskAcceptanceFailure::TaskOwner(_) => {
                                panic!("exact provider task owner refused the call")
                            }
                        }
                    }
                    ResidentProcessProviderLaunchFailure::AcceptedWithFailure { .. } => {
                        panic!("accepted provider task returned a process failure")
                    }
                },
            };
            assert_eq!(executor.accepted.len(), 1);
            assert_eq!(executor.accepted.front().unwrap().key(), &result);
        },
    );
}

/// Build provider dispatch inputs for an already issued operation.
#[allow(dead_code, clippy::too_many_arguments)]
pub(super) fn with_provider_dispatch_inputs<'operation, R>(
    prefix: &str,
    store: &AuthorityStoreCoordinate,
    provider_request: ProviderInvocationRequest<'operation>,
    operation: &'operation IssuedLocalOperation,
    request: &'operation CommonRequestEnvelopeV1,
    generation: &'operation AdmittedCompiledGeneration,
    admission: ProviderInvocationAdmissionContext<'operation>,
    parents: &'operation crate::local_operation_start_fixture::RecoveryParents<'operation>,
    initial_checkpoint: (&ResidentRecordRefV1, &'operation ControlStateCheckpointV1),
    initial_record: &'operation ControlRecord,
    selection_record: &'operation ControlRecord,
    run: impl FnOnce(
        ProviderOperationDispatchInput<'_, '_>,
        ProviderOperationInvokeInput<'_>,
        ProviderOperationInvokeInput<'_>,
        &ProviderClock,
        &ProviderStore,
    ) -> R,
) -> R {
    let (handle_ref, handle) = operation.handle();
    let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let initial = issue_observation(
        parents.signing,
        handle_ref,
        handle,
        observation_schema.clone(),
        OperationObservationRevisionV1::new(1).expect("initial revision"),
        None,
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving,
        },
        60,
    );
    assert_eq!(
        initial_record.canonical_record_jcs().as_slice(),
        pale_core::canon::to_canonical_string(&initial)
            .expect("initial observation encoding")
            .as_bytes(),
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
    let mut checkpoint_fields = checkpoint_preimage_at_store(2, None, store);
    checkpoint_fields.checkpoint_domain_id = initial_checkpoint.1.checkpoint_domain_id().clone();
    checkpoint_fields.checkpoint_profile_ref =
        initial_checkpoint.1.checkpoint_profile_ref().clone();
    checkpoint_fields.checkpoint_profile_sha256 =
        initial_checkpoint.1.checkpoint_profile_sha256().clone();
    let checkpoint_signing = checkpoint_authority(&checkpoint_fields);
    let checkpoint_input = |phase: &str| OperationCheckpointInput {
        reference: record_ref(&format!("{prefix}/checkpoint-{phase}")),
        id: ControlCheckpointIdV1::parse(format!("{prefix}-checkpoint-{phase}"))
            .expect("provider checkpoint identity"),
    };
    let first_dispatch_time = parents
        .current
        .now
        .max(*initial_checkpoint.1.issued_at())
        .get();
    let clock = ProviderClock {
        times: [
            0, 1, 2, 3, 4, 5, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        ]
        .map(|offset| first_dispatch_time + offset)
        .into(),
        next: AtomicUsize::new(0),
    };
    let store_resolver = ProviderStore::new(store.clone());
    let invocation = || ProviderOperationInvokeInput {
        current: parents.current,
        signing: observation.signed,
        terminal_checkpoint: checkpoint_input("terminal"),
        result_ref: record_ref(&format!("{prefix}/result")),
        diagnostic_ref: record_ref(&format!("{prefix}/diagnostic")),
        reconciliation: pale_core::records::ReconciliationFrontierReferenceV1 {
            record_ref: record_ref(&format!("{prefix}/reconciliation")),
            record_sha256: pale_core::records::ReconciliationFrontierDigest::parse("d".repeat(64))
                .expect("reconciliation digest"),
        },
        terminal_ref: record_ref(&format!("{prefix}/terminal")),
        checkpoint_schema: &checkpoint_fields.schema_sha256,
        checkpoint_authority: &checkpoint_signing,
        clock: &clock,
        resolve_store: &store_resolver,
        maximum_record_bytes: 65_536,
        maximum_diagnostic_bytes: 65_536,
        accumulator_limits: AccumulatorPreparationLimits::new(1, 1024, 1024, 512, 65_536)
            .expect("provider invocation limits"),
    };
    let dispatch = ProviderOperationDispatchInput {
        operation: ProviderOperationInput {
            context: OperationAttemptContext {
                operation,
                request,
                session: parents.session,
                current: parents.current,
                lease: operation.historical_operation_lease(),
                context_binding: None,
                generation,
                selection_ref: selection_record.coordinate().record_ref().clone(),
                maximum_selection_bytes: 65_536,
                observation_ref: record_ref(&format!("{prefix}/preparation")),
                observation,
            },
            invocation: provider_request,
            result_admission: admission,
            resolution_ref: record_ref(&format!("{prefix}/resolution")),
        },
        checkpoint: initial_checkpoint,
        checkpoint_schema: &checkpoint_fields.schema_sha256,
        checkpoint_authority: &checkpoint_signing,
        signing: observation.signed,
        resolution_checkpoint: checkpoint_input("resolved"),
        preparation_checkpoint: checkpoint_input("prepared"),
        dispatch_checkpoint: checkpoint_input("dispatched"),
        dispatch_ref: record_ref(&format!("{prefix}/dispatched")),
        clock: &clock,
        resolve_store: &store_resolver,
        accumulator_limits: AccumulatorPreparationLimits::new(1, 1024, 1024, 512, 65_536)
            .expect("provider accumulator limits"),
    };
    run(
        dispatch,
        invocation(),
        invocation(),
        &clock,
        &store_resolver,
    )
}

#[allow(clippy::too_many_arguments)]
fn launch_fixture_provider<'operation, WriterGuard>(
    host: &mut ResidentProcessHost<'_, WriterGuard>,
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    custody: &mut dyn OperationCustodyPort,
    sessions: &ProviderSessionRead,
    anchor: &mut dyn AntiRollbackAnchorPort,
    authority: &mut ProvingAuthority,
    executor: &mut ProviderExecutor,
    prefix: &str,
    provider_request: ProviderInvocationRequest<'operation>,
    operation: &'operation IssuedLocalOperation,
    request: &'operation CommonRequestEnvelopeV1,
    generation: &'operation AdmittedCompiledGeneration,
    admission: ProviderInvocationAdmissionContext<'operation>,
    parents: &'operation crate::local_operation_start_fixture::RecoveryParents<'operation>,
    initial_checkpoint: &'operation ControlStateCheckpointV1,
    initial_record: &'operation ControlRecord,
    selection_record: &'operation ControlRecord,
) -> LaunchedProviderEvidence {
    let provider_binding = pale_core::records::ProviderBindingRefV1 {
        provider_binding_id: provider_request.provider_binding().id.clone(),
        provider_binding_identity: provider_request.provider_binding().binding_identity.clone(),
    };
    let (handle_ref, handle) = operation.handle();
    let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let initial = issue_observation(
        parents.signing,
        handle_ref,
        handle,
        observation_schema.clone(),
        OperationObservationRevisionV1::new(1).expect("initial revision"),
        None,
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving,
        },
        60,
    );
    assert_eq!(
        initial_record.canonical_record_jcs().as_slice(),
        pale_core::canon::to_canonical_string(&initial)
            .expect("initial observation encoding")
            .as_bytes(),
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
    let mut checkpoint_fields = checkpoint_preimage_at_store(2, None, store);
    checkpoint_fields.checkpoint_domain_id = initial_checkpoint.checkpoint_domain_id().clone();
    checkpoint_fields.checkpoint_profile_ref = initial_checkpoint.checkpoint_profile_ref().clone();
    checkpoint_fields.checkpoint_profile_sha256 =
        initial_checkpoint.checkpoint_profile_sha256().clone();
    let checkpoint_signing = checkpoint_authority(&checkpoint_fields);
    let checkpoint_input = |phase: &str| OperationCheckpointInput {
        reference: record_ref(&format!("{prefix}/checkpoint-{phase}")),
        id: ControlCheckpointIdV1::parse(format!("{prefix}-checkpoint-{phase}"))
            .expect("provider checkpoint identity"),
    };
    let clock = ProviderClock {
        times: vec![61, 62, 63, 64, 65, 66, 67, 68],
        next: AtomicUsize::new(0),
    };
    let store_resolver = ProviderStore::new(store.clone());
    let initial_checkpoint_ref = record_ref("operation/checkpoint-1");
    let invocation = ProviderOperationInvokeInput {
        current: parents.current,
        signing: observation.signed,
        terminal_checkpoint: checkpoint_input("terminal"),
        result_ref: record_ref(&format!("{prefix}/result")),
        diagnostic_ref: record_ref(&format!("{prefix}/diagnostic")),
        reconciliation: pale_core::records::ReconciliationFrontierReferenceV1 {
            record_ref: record_ref(&format!("{prefix}/reconciliation")),
            record_sha256: pale_core::records::ReconciliationFrontierDigest::parse("d".repeat(64))
                .expect("reconciliation digest"),
        },
        terminal_ref: record_ref(&format!("{prefix}/terminal")),
        checkpoint_schema: &checkpoint_fields.schema_sha256,
        checkpoint_authority: &checkpoint_signing,
        clock: &clock,
        resolve_store: &store_resolver,
        maximum_record_bytes: 65_536,
        maximum_diagnostic_bytes: 65_536,
        accumulator_limits: AccumulatorPreparationLimits::new(1, 1024, 1024, 512, 65_536)
            .expect("provider invocation limits"),
    };
    let dispatch = ProviderOperationDispatchInput {
        operation: ProviderOperationInput {
            context: OperationAttemptContext {
                operation,
                request,
                session: parents.session,
                current: parents.current,
                lease: parents.lease,
                context_binding: None,
                generation,
                selection_ref: selection_record.coordinate().record_ref().clone(),
                maximum_selection_bytes: 65_536,
                observation_ref: record_ref(&format!("{prefix}/preparation")),
                observation,
            },
            invocation: provider_request,
            result_admission: admission,
            resolution_ref: record_ref(&format!("{prefix}/resolution")),
        },
        checkpoint: (&initial_checkpoint_ref, initial_checkpoint),
        checkpoint_schema: &checkpoint_fields.schema_sha256,
        checkpoint_authority: &checkpoint_signing,
        signing: observation.signed,
        resolution_checkpoint: checkpoint_input("resolved"),
        preparation_checkpoint: checkpoint_input("prepared"),
        dispatch_checkpoint: checkpoint_input("dispatched"),
        dispatch_ref: record_ref(&format!("{prefix}/dispatched")),
        clock: &clock,
        resolve_store: &store_resolver,
        accumulator_limits: AccumulatorPreparationLimits::new(1, 1024, 1024, 512, 65_536)
            .expect("provider accumulator limits"),
    };
    let key = match host.launch_provider(
        selector,
        ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        },
        authority,
        dispatch,
        &invocation,
        executor,
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(signature()),
        |_, _, _| Ok(signature()),
    ) {
        Ok(key) => key,
        Err(failure) => match *failure {
            ResidentProcessProviderLaunchFailure::Selection { reason, .. } => {
                panic!("exact deployment selection failed: {reason:?}")
            }
            ResidentProcessProviderLaunchFailure::RouteSaturated(_) => {
                panic!("the exact deployment route was saturated")
            }
            ResidentProcessProviderLaunchFailure::Retention { .. } => {
                panic!("exact deployment retention failed")
            }
            ResidentProcessProviderLaunchFailure::RetainedStoreAssociation { .. } => {
                panic!("retained dispatch resolved to another store")
            }
            ResidentProcessProviderLaunchFailure::Acceptance { failure, .. }
            | ResidentProcessProviderLaunchFailure::AcceptedWithFailure { failure, .. } => {
                match failure {
                    ResidentProviderTaskAcceptanceFailure::AggregateSaturated(_) => {
                        panic!("provider aggregate was saturated")
                    }
                    ResidentProviderTaskAcceptanceFailure::Preflight(_) => {
                        panic!("provider release preflight failed")
                    }
                    ResidentProviderTaskAcceptanceFailure::TaskOwner(_) => {
                        panic!("provider task owner refused the call")
                    }
                }
            }
        },
    };
    let evidence = ImmutableRecordReferenceV1 {
        record_ref: selection_record.coordinate().record_ref().clone(),
        record_sha256: ArtifactDigest::parse(
            selection_record.coordinate().record_sha256().as_str(),
        )
        .expect("selection digest"),
    };
    let resolution = (
        record_ref(&format!("{prefix}/resolution")),
        issue_observation(
            parents.signing,
            handle_ref,
            handle,
            observation_schema.clone(),
            OperationObservationRevisionV1::new(2).expect("resolution revision"),
            Some((initial_record.coordinate().record_ref(), &initial)),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ProviderResolved { provider_binding },
            },
            61,
        ),
    );
    let preparation = (
        record_ref(&format!("{prefix}/preparation")),
        issue_observation(
            parents.signing,
            handle_ref,
            handle,
            observation_schema.clone(),
            OperationObservationRevisionV1::new(3).expect("preparation revision"),
            Some((&resolution.0, &resolution.1)),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::DispatchPrepared {
                    attempt_id: CapabilityAttemptIdV1::parse("attempt-1").unwrap(),
                    evidence: evidence.clone(),
                },
            },
            62,
        ),
    );
    let dispatched = (
        record_ref(&format!("{prefix}/dispatched")),
        issue_observation(
            parents.signing,
            handle_ref,
            handle,
            observation_schema,
            OperationObservationRevisionV1::new(4).expect("dispatch revision"),
            Some((&preparation.0, &preparation.1)),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched {
                    attempt_id: CapabilityAttemptIdV1::parse("attempt-1").unwrap(),
                    evidence,
                },
            },
            63,
        ),
    );
    assert_eq!(key.dispatch().record_ref, dispatched.0);
    assert_eq!(
        key.dispatch().record_sha256.as_str(),
        dispatched
            .1
            .canonical_operation_observation_sha256()
            .as_str()
    );
    LaunchedProviderEvidence {
        key,
        resolution,
        preparation,
        dispatched,
    }
}

fn read_observation_checkpoint(
    custody: &mut dyn OperationCustodyPort,
    store: &AuthorityStoreCoordinate,
    observation: &(ResidentRecordRefV1, OperationObservationV1),
    predecessor: &(ResidentRecordRefV1, ControlStateCheckpointV1),
) -> (ResidentRecordRefV1, ControlStateCheckpointV1) {
    let coordinate = pale_ports::ControlRecordCoordinate::new(
        observation.0.clone(),
        pale_ports::ControlRecordDigest::parse(
            observation
                .1
                .canonical_operation_observation_sha256()
                .as_str(),
        )
        .expect("observation record digest"),
    );
    let query = OperationCustodyIndexQuery::ObservationCheckpoint(
        OperationCustodyIndexKey::for_observation(&coordinate),
    );
    let record = custody
        .read_index(store, &query, 65_536)
        .expect("checkpoint index read")
        .into_record()
        .expect("retained observation checkpoint");
    let reference = record.coordinate().record_ref().clone();
    let wire: ControlStateCheckpointWireV1 =
        serde_json::from_slice(record.canonical_record_jcs().as_slice()).expect("checkpoint wire");
    let mut authority_fields = checkpoint_preimage_at_store(2, None, store);
    authority_fields.checkpoint_domain_id = predecessor.1.checkpoint_domain_id().clone();
    authority_fields.checkpoint_profile_ref = predecessor.1.checkpoint_profile_ref().clone();
    authority_fields.checkpoint_profile_sha256 = predecessor.1.checkpoint_profile_sha256().clone();
    let authority = checkpoint_authority(&authority_fields);
    let checkpoint = ControlStateCheckpointV1::admit_successor_wire(
        wire,
        &predecessor.0,
        &predecessor.1,
        predecessor.1.schema_sha256(),
        &authority,
        |_, _, _, actual| {
            assert_eq!(actual, &signature());
            Ok(())
        },
    )
    .expect("retained successor checkpoint admits");
    assert_eq!(
        record.canonical_record_jcs().as_slice(),
        pale_core::canon::to_canonical_string(&checkpoint)
            .expect("admitted checkpoint encoding")
            .as_bytes()
    );
    (reference, checkpoint)
}

pub(super) struct RetainedProcessProviderCancellationFixture<'a, 'store> {
    pub(super) host: &'a mut ResidentProcessHost<'store, UnprovedHeldWriter>,
    pub(super) selector: ResidentDeploymentAuthoritySelector,
    pub(super) store: AuthorityStoreCoordinate,
    pub(super) operation: &'a IssuedLocalOperation,
    pub(super) request: &'a CommonRequestEnvelopeV1,
    pub(super) generation: &'a AdmittedCompiledGeneration,
    pub(super) selection: &'a ControlRecord,
    pub(super) dispatched: (ResidentRecordRefV1, OperationObservationV1),
    pub(super) cancellation_request: (pale_ports::ControlRecordCoordinate, CommonRequestEnvelopeV1),
    pub(super) checkpoint: ControlStateCheckpointV1,
    pub(super) signing: &'a SigningAuthorityBindingV1,
    pub(super) custody: &'a mut dyn OperationCustodyPort,
    pub(super) sessions: &'a dyn SessionLeaseCustodyPort,
    pub(super) anchor: &'a mut dyn AntiRollbackAnchorPort,
    pub(super) parents: crate::local_operation_start_fixture::RecoveryParents<'a>,
}

pub(super) fn with_retained_process_provider_cancellation<R>(
    run: impl for<'a, 'store> FnOnce(RetainedProcessProviderCancellationFixture<'a, 'store>) -> R,
) -> R {
    with_provider_selection_restart_context(
        LocalStartContext::SelfContained,
        |provider_request,
         operation,
         request,
         generation,
         admission,
         parents,
         initial_checkpoint,
         initial_record,
         initial_candidate,
         selection,
         restart_generation| {
            let selected_store = initial_candidate.store().clone();
            let selector = selector_from_signing_authority(parents.signing);
            let (handle_ref, handle) = operation.handle();
            let handle_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    handle_ref,
                    handle.canonical_operation_handle_sha256().as_str(),
                )
                .expect("provider handle coordinate"),
                handle,
            )
            .expect("provider handle record");
            let lease_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    handle.operation_lease_ref(),
                    parents.lease.operation_lease_digest().as_str(),
                )
                .expect("provider lease coordinate"),
                parents.lease,
            )
            .expect("provider lease record");
            let competing_head = Rc::new(RefCell::new(None));
            custody::with_service_custody_and_handle(
                initial_checkpoint,
                Some(initial_candidate),
                Some(lease_record),
                4,
                4,
                false,
                &[],
                Some(handle_record),
                competing_head,
                |custody, anchor| {
                    let application_store = MemoryAuthorityStore::default();
                    let mut application =
                        ready_application(&application_store, selected_store.clone(), 73);
                    application.fixture_retain_generation(restart_generation);
                    let deployment = unproved_deployment_for_selector(
                        &selector,
                        selected_store.clone(),
                        application,
                    );
                    let mut host = ResidentProcessHost::new(
                        unproved_writer(
                            selector.principal_id.clone(),
                            vec![selected_store.clone()],
                        ),
                        vec![deployment],
                        NonZeroUsize::new(1).expect("one provider deployment"),
                        ResidentOperationHost::new(task_state()),
                    )
                    .expect("one provider process host");
                    let sessions = provider_session_read(&parents, selected_store.clone());
                    let mut executor = ProviderExecutor::default();
                    let mut authority = ProvingAuthority;
                    let launched = launch_fixture_provider(
                        &mut host,
                        &selector,
                        &selected_store,
                        custody,
                        &sessions,
                        anchor,
                        &mut authority,
                        &mut executor,
                        "provider-cancellation-restart",
                        provider_request,
                        operation,
                        request,
                        generation,
                        admission,
                        &parents,
                        initial_checkpoint,
                        initial_record,
                        selection,
                    );
                    let dispatched_checkpoint = {
                        let resolved = read_observation_checkpoint(
                            custody,
                            &selected_store,
                            &launched.resolution,
                            &(
                                record_ref("operation/checkpoint-1"),
                                initial_checkpoint.clone(),
                            ),
                        );
                        let prepared = read_observation_checkpoint(
                            custody,
                            &selected_store,
                            &launched.preparation,
                            &resolved,
                        );
                        read_observation_checkpoint(
                            custody,
                            &selected_store,
                            &launched.dispatched,
                            &prepared,
                        )
                    };
                    let outcome = host.operations.with_operation_service(
                        &mut authority,
                        custody,
                        anchor,
                        |service| {
                            request_composed_provider_cancellation(
                                service,
                                operation,
                                request,
                                &parents,
                                None,
                                initial_candidate,
                                (&launched.dispatched.0, &launched.dispatched.1),
                                (&dispatched_checkpoint.0, &dispatched_checkpoint.1),
                                true,
                            )
                            .unwrap_or_else(|_| panic!("provider cancellation request custody"))
                        },
                    );
                    let ResidentOperationCancellationOutcome::DispatchedRequestRetained {
                        custody: cancellation_custody,
                        checkpoint: cancellation_checkpoint,
                        ..
                    } = outcome
                    else {
                        panic!("accepted provider task must retain its cancellation request")
                    };
                    let cancellation_record = cancellation_custody
                        .candidate()
                        .record(pale_ports::OperationCustodyRecordKind::CommonRequest)
                        .expect("provider cancellation retains its exact request");
                    let cancellation_request = (
                        cancellation_record.coordinate().clone(),
                        retained_provider_cancellation_request(request, operation.handle().1),
                    );
                    run(RetainedProcessProviderCancellationFixture {
                        host: &mut host,
                        selector,
                        store: selected_store,
                        operation,
                        request,
                        generation,
                        selection,
                        dispatched: launched.dispatched,
                        cancellation_request,
                        checkpoint: cancellation_checkpoint.1,
                        signing: parents.signing,
                        custody,
                        sessions: &sessions,
                        anchor,
                        parents,
                    })
                },
            )
        },
    )
}

#[derive(Clone, Copy)]
enum ProviderEvidenceProofCase {
    RejectedSignature,
    SubstitutedResultSchema,
}

fn provider_evidence_uncertainty_after_owned_recording_retry(case: ProviderEvidenceProofCase) {
    with_provider_selection_restart_context(
        LocalStartContext::SelfContained,
        |provider_request,
         operation,
         request,
         generation,
         admission,
         parents,
         initial_checkpoint,
         initial_record,
         initial_candidate,
         selection,
         restart_generation| {
            let selected_store = initial_candidate.store().clone();
            let selector = selector_from_signing_authority(parents.signing);
            let (handle_ref, handle) = operation.handle();
            let handle_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    handle_ref,
                    handle.canonical_operation_handle_sha256().as_str(),
                )
                .expect("provider evidence handle coordinate"),
                handle,
            )
            .expect("provider evidence handle record");
            let lease_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    handle.operation_lease_ref(),
                    parents.lease.operation_lease_digest().as_str(),
                )
                .expect("provider evidence lease coordinate"),
                parents.lease,
            )
            .expect("provider evidence lease record");
            let competing_head = Rc::new(RefCell::new(None));
            custody::with_service_custody_and_handle(
                initial_checkpoint,
                Some(initial_candidate),
                Some(lease_record),
                4,
                4,
                false,
                &[],
                Some(handle_record),
                competing_head,
                |custody, anchor| {
                    let application_store = MemoryAuthorityStore::default();
                    let mut application =
                        ready_application(&application_store, selected_store.clone(), 83);
                    application.fixture_retain_generation(restart_generation);
                    let deployment = unproved_deployment_for_selector(
                        &selector,
                        selected_store.clone(),
                        application,
                    );
                    let mut host = ResidentProcessHost::new(
                        unproved_writer(
                            selector.principal_id.clone(),
                            vec![selected_store.clone()],
                        ),
                        vec![deployment],
                        NonZeroUsize::new(1).expect("one provider deployment"),
                        ResidentOperationHost::new(task_state()),
                    )
                    .expect("provider evidence process host");
                    let sessions = provider_session_read(&parents, selected_store.clone());
                    let mut executor = ProviderExecutor::default();
                    let mut authority = ProvingAuthority;
                    let label = match case {
                        ProviderEvidenceProofCase::RejectedSignature => {
                            "provider-evidence-rejected"
                        }
                        ProviderEvidenceProofCase::SubstitutedResultSchema => {
                            "provider-evidence-substituted"
                        }
                    };
                    let launched = launch_fixture_provider(
                        &mut host,
                        &selector,
                        &selected_store,
                        custody,
                        &sessions,
                        anchor,
                        &mut authority,
                        &mut executor,
                        label,
                        provider_request,
                        operation,
                        request,
                        generation,
                        admission,
                        &parents,
                        initial_checkpoint,
                        initial_record,
                        selection,
                    );
                    assert_eq!(executor.accepted.len(), 1);

                    let dispatched_checkpoint = {
                        let resolved = read_observation_checkpoint(
                            custody,
                            &selected_store,
                            &launched.resolution,
                            &(
                                record_ref("operation/checkpoint-1"),
                                initial_checkpoint.clone(),
                            ),
                        );
                        let prepared = read_observation_checkpoint(
                            custody,
                            &selected_store,
                            &launched.preparation,
                            &resolved,
                        );
                        read_observation_checkpoint(
                            custody,
                            &selected_store,
                            &launched.dispatched,
                            &prepared,
                        )
                    };

                    let attempt = CapabilityAttemptIdV1::parse("attempt-1")
                        .expect("provider attempt identity");
                    let response_case = match case {
                        ProviderEvidenceProofCase::RejectedSignature => "valid",
                        ProviderEvidenceProofCase::SubstitutedResultSchema => "wrong-outer-schema",
                    };
                    let returned = provider_fixture::success(
                        request,
                        generation,
                        parents.signing,
                        &attempt,
                        response_case,
                    );
                    let returned_coordinate = returned.coordinate().clone();
                    returned_coordinate
                        .validate_for(provider_request)
                        .expect("provider response echoes the exact released request");
                    let returned_schema = returned.result().schema_sha256().clone();
                    let returned_wire = serde_json::from_value(
                        serde_json::to_value(returned.result()).expect("provider result value"),
                    )
                    .expect("provider result wire");
                    let port =
                        provider_fixture::ResultPort::success(ProviderInvocationSuccessWire::new(
                            returned_coordinate.clone(),
                            returned_wire,
                        ));
                    executor.execute_next(&port);
                    assert_eq!(port.calls(), 1, "the provider port must be invoked once");
                    let progress = host
                        .poll_provider_task(&mut executor)
                        .expect("provider evidence completion poll");
                    let ResidentProcessProviderCompletionPoll::Routed(progress) = progress else {
                        panic!("provider evidence must retain its selected route")
                    };
                    let progress = host.continue_provider_aggregate_transition(*progress);

                    let history = OperationAttemptHistoryInput {
                        operation,
                        request,
                        session: parents.session.lease(),
                        lease: parents.lease,
                        generation,
                        dispatched: &launched.dispatched.1,
                        selection,
                        maximum_selection_bytes: 65_536,
                    };
                    let recovered = RecoveredOperationObservation::history_fixture_for_handle(
                        crate::resident::record_projection::coordinate(
                            handle_ref,
                            handle.canonical_operation_handle_sha256().as_str(),
                        )
                        .expect("provider evidence recovery handle"),
                        launched.dispatched.clone(),
                        Some(launched.preparation.clone()),
                        None,
                        launched.dispatched.clone(),
                        dispatched_checkpoint,
                        selected_store.clone(),
                    );
                    let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
                    let mut checkpoint_fields =
                        checkpoint_preimage_at_store(5, None, &selected_store);
                    checkpoint_fields.checkpoint_domain_id =
                        initial_checkpoint.checkpoint_domain_id().clone();
                    checkpoint_fields.checkpoint_profile_ref =
                        initial_checkpoint.checkpoint_profile_ref().clone();
                    checkpoint_fields.checkpoint_profile_sha256 =
                        initial_checkpoint.checkpoint_profile_sha256().clone();
                    let checkpoint_signing = checkpoint_authority(&checkpoint_fields);
                    let clock = ProviderClock {
                        times: vec![64, 65, 66, 67],
                        next: AtomicUsize::new(0),
                    };
                    let resolver = ProviderStore::new(selected_store.clone());
                    let invocation = ProviderOperationInvokeInput {
                        current: parents.current,
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: parents.signing,
                        },
                        terminal_checkpoint: OperationCheckpointInput {
                            reference: record_ref(&format!("{label}/terminal-checkpoint")),
                            id: ControlCheckpointIdV1::parse(format!(
                                "{label}-terminal-checkpoint"
                            ))
                            .expect("provider evidence checkpoint identity"),
                        },
                        result_ref: record_ref(&format!("{label}/result")),
                        diagnostic_ref: record_ref(&format!("{label}/diagnostic")),
                        reconciliation: pale_core::records::ReconciliationFrontierReferenceV1 {
                            record_ref: record_ref(&format!("{label}/reconciliation")),
                            record_sha256: pale_core::records::ReconciliationFrontierDigest::parse(
                                "d".repeat(64),
                            )
                            .expect("provider evidence reconciliation digest"),
                        },
                        terminal_ref: record_ref(&format!("{label}/terminal")),
                        checkpoint_schema: &checkpoint_fields.schema_sha256,
                        checkpoint_authority: &checkpoint_signing,
                        clock: &clock,
                        resolve_store: &resolver,
                        maximum_record_bytes: 65_536,
                        maximum_diagnostic_bytes: 65_536,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            1, 1024, 1024, 512, 65_536,
                        )
                        .expect("provider evidence recording limits"),
                    };
                    let contract_set = crate::session_admission_fixture::local_start_contract_set();
                    let returned_result = returned.result();
                    let returned_success = CapabilitySuccessAdmissionContextV1 {
                        result: CapabilityResultAdmissionContextV1 {
                            expected_schema_sha256: returned_result.schema_sha256(),
                            expected_request_binding: returned_result.request_binding(),
                            expected_attempt_id: returned_result.attempt_id(),
                            expected_implementation_kind: returned_result.implementation_kind(),
                            expected_implementation_version: returned_result
                                .implementation_version(),
                            signing_authority: parents.signing,
                        },
                        contract_set_ref: returned_result.request_binding().contract_set_ref(),
                        contract_set: &contract_set,
                    };
                    let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
                    let returned_admission =
                        crate::resident::operation_dispatch::selection::tests::admission(
                            returned_success,
                            &failure_schema,
                        );
                    let recording_admission = match case {
                        ProviderEvidenceProofCase::RejectedSignature => admission,
                        ProviderEvidenceProofCase::SubstitutedResultSchema => returned_admission,
                    };
                    let valid_evidence = provider_fixture::Evidence::valid();
                    let rejecting_evidence =
                        provider_fixture::Evidence::rejecting_result_signature();
                    let recording_evidence: &dyn crate::provider_invocation::ProviderInvocationEvidenceAdmission =
                        match case {
                            ProviderEvidenceProofCase::RejectedSignature => &rejecting_evidence,
                            ProviderEvidenceProofCase::SubstitutedResultSchema => &valid_evidence,
                        };
                    let recording: ResidentProcessProviderRecording<'static> = {
                        // The static recording type requires a preparation failure to retain
                        // only its immutable basis, never these borrowed recovery inputs.
                        host.record_completed_provider(
                            progress,
                            ResidentDeploymentOperationAccess {
                                store: &selected_store,
                                custody,
                                sessions: &sessions,
                                anchor,
                            },
                            &mut authority,
                            history,
                            &recovered,
                            recording_admission,
                            &invocation,
                            recording_evidence,
                            |_| Ok(()),
                            |_, _, _, _| Ok(()),
                            |_, _, _| {
                                Err(InvalidResidentRecordV1::ContextualAdmission {
                                    record: "provider terminal observation",
                                    invariant: "test retains the owned recording continuation",
                                })
                            },
                            |_, _, _| Ok(signature()),
                        )
                        .unwrap_or_else(|_| panic!("provider evidence reaches its recording owner"))
                    };
                    assert!(matches!(
                        &recording,
                        ResidentProcessProviderRecording::Pending {
                            progress: ResidentProviderTaskPoll::Preparation { .. },
                            ..
                        }
                    ));
                    let polls_before_retry = executor.completion_polls;
                    let recording = host
                        .continue_provider_recording(
                            recording,
                            ResidentDeploymentOperationAccess {
                                store: &selected_store,
                                custody,
                                sessions: &sessions,
                                anchor,
                            },
                            &mut authority,
                            &invocation,
                            |_| Ok(()),
                            |_, _, _, _| Ok(()),
                            |_, _, _| Ok(signature()),
                            |_, _, _| Ok(signature()),
                        )
                        .unwrap_or_else(|_| {
                            panic!("owned provider recording continuation retries directly")
                        });
                    assert_eq!(
                        executor.completion_polls, polls_before_retry,
                        "recording retry must not poll the executor again",
                    );
                    assert_eq!(
                        port.calls(),
                        1,
                        "recording retry must not invoke the provider"
                    );
                    let ResidentProcessProviderRecording::Recorded {
                        result: ProviderOperationInvokeResult::OutcomeUnknown(unknown),
                        cancellation: ResidentOperationTaskCancellationState::Unnotified,
                    } = recording
                    else {
                        panic!("untrusted provider evidence must settle as outcome unknown")
                    };
                    match (case, unknown.uncertainty()) {
                        (
                            ProviderEvidenceProofCase::RejectedSignature,
                            crate::resident::operation_dispatch::ProviderOperationUncertainty::ResponseRejected(
                                crate::provider_invocation::ProviderInvocationAdmissionError::InvalidEvidence(
                                    InvalidResidentRecordV1::ContextualAdmission { record, invariant },
                                ),
                            ),
                        ) => {
                            assert_eq!(record, &"capability result");
                            assert_eq!(
                                invariant,
                                &"test result signature verifier rejects the provider evidence"
                            );
                            assert_eq!(
                                returned_schema,
                                schema_digest(pale_core::records::CAPABILITY_RESULT_V1_SCHEMA_ID)
                            );
                        }
                        (
                            ProviderEvidenceProofCase::SubstitutedResultSchema,
                            crate::resident::operation_dispatch::ProviderOperationUncertainty::ResponseAssociation {
                                outcome,
                                reason,
                            },
                        ) => {
                            assert!(matches!(reason.as_ref(), OperationAttemptError::Association));
                            let crate::provider_invocation::AdmittedProviderInvocationOutcome::Succeeded(
                                admitted,
                            ) = outcome.as_ref()
                            else {
                                panic!("the substituted response must remain an admitted success")
                            };
                            let substituted_schema = SchemaDigest::parse("0".repeat(64))
                                .expect("substituted result-schema digest");
                            assert_eq!(&returned_schema, &substituted_schema);
                            assert_ne!(
                                &returned_schema,
                                &schema_digest(pale_core::records::CAPABILITY_RESULT_V1_SCHEMA_ID)
                            );
                            assert_eq!(admitted.coordinate(), &returned_coordinate);
                            admitted
                                .coordinate()
                                .validate_for(provider_request)
                                .expect("all provider observation coordinates remain exact");
                            assert_eq!(admitted.result().schema_sha256(), &substituted_schema);
                            assert_eq!(
                                admitted.result().request_binding(),
                                &request.local_request_binding().expect("local request binding")
                            );
                            assert_eq!(admitted.result().attempt_id(), &attempt);
                        }
                        _ => panic!("provider evidence reached the wrong uncertainty branch"),
                    }
                    let diagnostic_digest =
                        ArtifactDigest::hash_bytes(unknown.diagnostic().canonical_bytes());
                    match unknown.observation().1.state() {
                        OperationObservationStateV1::OutcomeUnknown {
                            diagnostic,
                            reconciliation,
                            effect_receipt,
                        } => {
                            assert_eq!(diagnostic.record_ref, invocation.diagnostic_ref);
                            assert_eq!(diagnostic.record_sha256, diagnostic_digest);
                            assert_eq!(reconciliation, &invocation.reconciliation);
                            assert!(effect_receipt.is_none());
                        }
                        state => panic!("provider uncertainty recorded unexpected state {state:?}"),
                    }
                    assert!(
                        !unknown.observation().1.state().is_terminal(),
                        "outcome unknown remains open for reconciliation"
                    );
                    assert_eq!(
                        unknown.observation().1.predecessor_ref(),
                        &Some(launched.dispatched.0.clone())
                    );
                    assert_eq!(
                        unknown.diagnostic().wire().code,
                        match case {
                            ProviderEvidenceProofCase::RejectedSignature => {
                                "provider_response_rejected"
                            }
                            ProviderEvidenceProofCase::SubstitutedResultSchema => {
                                "provider_response_mismatch"
                            }
                        }
                    );
                    assert_eq!(
                        unknown.custody().database_evidence(),
                        OperationCustodyDatabaseEvidence::New
                    );
                    assert_eq!(
                        unknown.custody().anchor_evidence(),
                        OperationCustodyAnchorEvidence::Advanced
                    );
                    let retained_observation = unknown
                        .custody()
                        .candidate()
                        .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
                        .expect("uncertainty custody retains its observation");
                    assert_eq!(
                        retained_observation.coordinate().record_ref(),
                        &invocation.terminal_ref
                    );
                    assert_eq!(
                        retained_observation.canonical_record_jcs().as_slice(),
                        pale_core::canon::to_canonical_string(unknown.observation().1)
                            .expect("uncertainty observation encoding")
                            .as_bytes()
                    );
                    let retained_diagnostic = unknown
                        .custody()
                        .candidate()
                        .record(pale_ports::OperationCustodyRecordKind::Diagnostic)
                        .expect("uncertainty custody retains its diagnostic");
                    assert_eq!(
                        retained_diagnostic.coordinate().record_ref(),
                        &invocation.diagnostic_ref
                    );
                    assert_eq!(
                        retained_diagnostic.coordinate().record_sha256().as_str(),
                        diagnostic_digest.as_str()
                    );
                    assert_eq!(
                        retained_diagnostic.canonical_record_jcs().as_slice(),
                        unknown.diagnostic().canonical_bytes()
                    );
                    assert_eq!(
                        port.calls(),
                        1,
                        "recording must not call the provider again"
                    );
                    assert!(executor.accepted.is_empty());
                    assert!(executor.completions.is_empty());
                    assert!(host.poll_provider_task(&mut executor).is_none());
                    assert_eq!(host.provider_routes.reserved, 0);
                    assert_eq!(host.provider_routes.checked_out, 0);
                    assert!(host.provider_routes.active.is_empty());
                },
            )
        },
    )
}

#[test]
fn provider_routes_preserve_two_full_deployment_selectors_out_of_order() {
    let limits = ResidentOperationTaskLimits::new(
        NonZeroUsize::new(2).expect("positive provider task bound"),
    );
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut routes =
        ResidentProcessProviderRoutes::new(owner, limits).expect("preallocated routes");
    let first_store = store("process-provider-first", "incarnation-1");
    let second_store = store("process-provider-second", "incarnation-1");
    let first_selector = unproved_selector(principal("principal-a"), first_store.clone());
    let second_selector = unproved_selector(principal("principal-a"), second_store.clone());
    let first_key = provider_task_key(first_store, "provider/dispatch-first", 'a');
    let second_key = provider_task_key(second_store, "provider/dispatch-second", 'b');

    let first_slot = routes
        .reserve(first_selector.clone())
        .expect("first route slot");
    assert!(routes.commit(first_slot, first_key.clone()).is_ok());
    let second_slot = routes
        .reserve(second_selector.clone())
        .expect("second route slot");
    assert!(routes.commit(second_slot, second_key.clone()).is_ok());

    let second = routes.check_out(&second_key).expect("second route custody");
    assert_eq!(second.key, second_key);
    assert_eq!(second.selector, second_selector);
    let first = routes.check_out(&first_key).expect("first route custody");
    assert_eq!(first.key, first_key);
    assert_eq!(first.selector, first_selector);
    assert!(routes
        .reserve(unproved_selector(
            principal("principal-a"),
            store("process-provider-third", "incarnation-1"),
        ))
        .is_err());
    assert!(routes.release_custody(second).is_ok());
    assert!(routes.release_custody(first).is_ok());
    assert!(routes
        .reserve(unproved_selector(
            principal("principal-a"),
            store("process-provider-third", "incarnation-1"),
        ))
        .is_ok());
}

#[test]
fn real_provider_launches_preserve_two_deployments_when_completion_order_reverses() {
    let first_store = store("process-provider-first", "incarnation-1");
    let second_store = store("process-provider-second", "incarnation-1");
    with_provider_selection_recovery_context_at_store(
        LocalStartContext::SelfContained,
        "first-selection",
        &first_store,
        |first_request,
         first_operation,
         first_envelope,
         first_generation,
         first_admission,
         first_parents,
         first_checkpoint,
         first_initial_record,
         first_candidate,
         first_selection| {
            let first_selector = selector_from_signing_authority(first_parents.signing);
            let first_handle_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    first_operation.handle().0,
                    first_operation
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .as_str(),
                )
                .expect("first handle coordinate"),
                first_operation.handle().1,
            )
            .expect("first handle record");
            let first_lease_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    first_operation.handle().1.operation_lease_ref(),
                    first_parents.lease.operation_lease_digest().as_str(),
                )
                .expect("first lease coordinate"),
                first_parents.lease,
            )
            .expect("first lease record");
            custody::with_service_custody_faults(
                first_checkpoint,
                Some(first_candidate),
                Vec::new(),
                Some(first_lease_record),
                Vec::new(),
                4,
                4,
                5,
                4,
                4,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                Vec::new(),
                false,
                Some(first_handle_record),
                Rc::new(RefCell::new(None)),
                |first_custody, first_anchor| {
                    with_provider_selection_recovery_context_at_store(
                        LocalStartContext::SelfContained,
                        "second-selection",
                        &second_store,
                        |second_request,
                         second_operation,
                         second_envelope,
                         second_generation,
                         second_admission,
                         second_parents,
                         second_checkpoint,
                         second_initial_record,
                         second_candidate,
                         second_selection| {
                            let second_selector =
                                selector_from_signing_authority(second_parents.signing);
                            let second_handle_record = crate::resident::record_projection::record(
                                crate::resident::record_projection::coordinate(
                                    second_operation.handle().0,
                                    second_operation
                                        .handle()
                                        .1
                                        .canonical_operation_handle_sha256()
                                        .as_str(),
                                )
                                .expect("second handle coordinate"),
                                second_operation.handle().1,
                            )
                            .expect("second handle record");
                            let second_lease_record = crate::resident::record_projection::record(
                                crate::resident::record_projection::coordinate(
                                    second_operation.handle().1.operation_lease_ref(),
                                    second_parents.lease.operation_lease_digest().as_str(),
                                )
                                .expect("second lease coordinate"),
                                second_parents.lease,
                            )
                            .expect("second lease record");
                            let second_competing_head = Rc::new(RefCell::new(None));
                            custody::with_service_custody_faults(
                                second_checkpoint,
                                Some(second_candidate),
                                Vec::new(),
                                Some(second_lease_record),
                                Vec::new(),
                                4,
                                4,
                                4,
                                3,
                                4,
                                &[4],
                                &[],
                                &[],
                                &[],
                                &[],
                                &[],
                                Vec::new(),
                                false,
                                Some(second_handle_record),
                                Rc::clone(&second_competing_head),
                                |second_custody, second_anchor| {
                                    let first_port = MemoryAuthorityStore::default();
                                    let second_port = MemoryAuthorityStore::default();
                                    let first_deployment = unproved_deployment_for_selector(
                                        &first_selector,
                                        first_store.clone(),
                                        ready_application(&first_port, first_store.clone(), 1),
                                    );
                                    let second_deployment = unproved_deployment_for_selector(
                                        &second_selector,
                                        second_store.clone(),
                                        ready_application(&second_port, second_store.clone(), 2),
                                    );
                                    let mut host = ResidentProcessHost::new(
                                        unproved_writer(
                                            first_selector.principal_id.clone(),
                                            vec![first_store.clone(), second_store.clone()],
                                        ),
                                        vec![first_deployment, second_deployment],
                                        NonZeroUsize::new(2).expect("two deployment bound"),
                                        ResidentOperationHost::new(
                                            task_state_with_provider_capacity(2),
                                        ),
                                    )
                                    .expect("two-deployment process host");
                                    let first_sessions =
                                        provider_session_read(&first_parents, first_store.clone());
                                    let second_sessions = provider_session_read(
                                        &second_parents,
                                        second_store.clone(),
                                    );
                                    let mut first_executor = ProviderExecutor::default();
                                    let mut second_executor = ProviderExecutor::default();
                                    let mut authority = ProvingAuthority;
                                    let first = launch_fixture_provider(
                                        &mut host,
                                        &first_selector,
                                        &first_store,
                                        first_custody,
                                        &first_sessions,
                                        first_anchor,
                                        &mut authority,
                                        &mut first_executor,
                                        "two-deployment/first",
                                        first_request,
                                        first_operation,
                                        first_envelope,
                                        first_generation,
                                        first_admission,
                                        &first_parents,
                                        first_checkpoint,
                                        first_initial_record,
                                        first_selection,
                                    );
                                    let second = launch_fixture_provider(
                                        &mut host,
                                        &second_selector,
                                        &second_store,
                                        second_custody,
                                        &second_sessions,
                                        second_anchor,
                                        &mut authority,
                                        &mut second_executor,
                                        "two-deployment/second",
                                        second_request,
                                        second_operation,
                                        second_envelope,
                                        second_generation,
                                        second_admission,
                                        &second_parents,
                                        second_checkpoint,
                                        second_initial_record,
                                        second_selection,
                                    );
                                    assert_eq!(host.provider_routes.active.len(), 2);

                                    second_executor.complete_next_unavailable();
                                    let second_progress = host
                                        .poll_provider_task(&mut second_executor)
                                        .expect("second completion arrives first");
                                    let ResidentProcessProviderCompletionPoll::Routed(
                                        second_progress,
                                    ) = second_progress
                                    else {
                                        panic!("second completion keeps its deployment route")
                                    };
                                    let ResidentProcessProviderCompletionRoute::CheckedOut(
                                        second_route,
                                    ) = &second_progress.route
                                    else {
                                        panic!("ordinary completion checks out its route")
                                    };
                                    assert_eq!(&second_route.selector, &second_selector);
                                    assert_eq!(&second_route.key, &second.key);
                                    assert_eq!(
                                        host.provider_routes.selector(&first.key),
                                        Some(&first_selector)
                                    );

                                    first_executor.lose_next();
                                    let first_progress = host
                                        .poll_provider_task(&mut first_executor)
                                        .expect("first completion arrives second");
                                    let ResidentProcessProviderCompletionPoll::Routed(
                                        first_progress,
                                    ) = first_progress
                                    else {
                                        panic!("first completion keeps its deployment route")
                                    };
                                    let ResidentProcessProviderCompletionRoute::CheckedOut(
                                        first_route,
                                    ) = &first_progress.route
                                    else {
                                        panic!("ordinary completion checks out its route")
                                    };
                                    assert_eq!(&first_route.selector, &first_selector);
                                    assert_eq!(&first_route.key, &first.key);

                                    let first_progress = host
                                        .continue_provider_aggregate_transition(*first_progress);
                                    let second_progress = host
                                        .continue_provider_aggregate_transition(*second_progress);
                                    assert_eq!(host.provider_routes.checked_out, 2);

                                    let first_dispatch_checkpoint = {
                                        let resolved = read_observation_checkpoint(
                                            first_custody,
                                            &first_store,
                                            &first.resolution,
                                            &(
                                                record_ref("operation/checkpoint-1"),
                                                first_checkpoint.clone(),
                                            ),
                                        );
                                        let prepared = read_observation_checkpoint(
                                            first_custody,
                                            &first_store,
                                            &first.preparation,
                                            &resolved,
                                        );
                                        read_observation_checkpoint(
                                            first_custody,
                                            &first_store,
                                            &first.dispatched,
                                            &prepared,
                                        )
                                    };
                                    let second_dispatch_checkpoint = {
                                        let resolved = read_observation_checkpoint(
                                            second_custody,
                                            &second_store,
                                            &second.resolution,
                                            &(
                                                record_ref("operation/checkpoint-1"),
                                                second_checkpoint.clone(),
                                            ),
                                        );
                                        let prepared = read_observation_checkpoint(
                                            second_custody,
                                            &second_store,
                                            &second.preparation,
                                            &resolved,
                                        );
                                        read_observation_checkpoint(
                                            second_custody,
                                            &second_store,
                                            &second.dispatched,
                                            &prepared,
                                        )
                                    };

                                    let second_cancellation_ref =
                                        record_ref("two-deployment/second/cancellation-requested");
                                    let second_cancellation = issue_observation(
                                        second_parents.signing,
                                        second_operation.handle().0,
                                        second_operation.handle().1,
                                        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                                        second.dispatched.1.revision().checked_successor().unwrap(),
                                        Some((&second.dispatched.0, &second.dispatched.1)),
                                        OperationObservationStateV1::CancellationRequested {
                                            request: ImmutableRecordReferenceV1 {
                                                record_ref: record_ref(
                                                    "two-deployment/second/cancellation-request",
                                                ),
                                                record_sha256: ArtifactDigest::hash_bytes(
                                                    b"second deployment cancellation request",
                                                ),
                                            },
                                        },
                                        70,
                                    );
                                    let second_cancellation_record =
                                        crate::resident::record_projection::coordinate(
                                            &second_cancellation_ref,
                                            second_cancellation
                                                .canonical_operation_observation_sha256()
                                                .as_str(),
                                        )
                                        .and_then(|coordinate| {
                                            crate::resident::record_projection::record(
                                                coordinate,
                                                &second_cancellation,
                                            )
                                        })
                                        .expect("second competing head record");
                                    *second_competing_head.borrow_mut() =
                                        Some(second_cancellation_record);
                                    let second_cancellation_append =
                                        prepare_operation_observation_append(
                                            OperationObservationTransition::new(
                                                second_operation.handle().0,
                                                second_operation.handle().1,
                                                &second.dispatched.0,
                                                &second.dispatched.1,
                                                &second_cancellation_ref,
                                                &second_cancellation,
                                            ),
                                            &ProviderStore::new(second_store.clone()),
                                        )
                                        .expect("second competing head append");
                                    let (_, second_cancellation_checkpoint) = custody::complete(
                                        second_cancellation_append,
                                        second_envelope,
                                        (
                                            &second_dispatch_checkpoint.0,
                                            &second_dispatch_checkpoint.1,
                                        ),
                                        OperationCustodyDatabaseEvidence::New,
                                        70,
                                    );
                                    let second_cancellation_checkpoint_ref = record_ref(&format!(
                                        "operation/checkpoint-{}",
                                        second_cancellation_checkpoint.domain_sequence().get()
                                    ));

                                    let first_recovered =
                                        RecoveredOperationObservation::history_fixture_for_handle(
                                            crate::resident::record_projection::coordinate(
                                                first_operation.handle().0,
                                                first_operation
                                                    .handle()
                                                    .1
                                                    .canonical_operation_handle_sha256()
                                                    .as_str(),
                                            )
                                            .expect("first recovery handle"),
                                            first.dispatched.clone(),
                                            Some(first.preparation.clone()),
                                            None,
                                            first.dispatched.clone(),
                                            first_dispatch_checkpoint,
                                            first_store.clone(),
                                        );
                                    let first_history = || OperationAttemptHistoryInput {
                                        operation: first_operation,
                                        request: first_envelope,
                                        session: first_operation.historical_session_lease(),
                                        lease: first_parents.lease,
                                        generation: first_generation,
                                        dispatched: &first.dispatched.1,
                                        selection: first_selection,
                                        maximum_selection_bytes: 65_536,
                                    };
                                    let second_rejected_recovered =
                                        RecoveredOperationObservation::history_fixture_for_handle(
                                            crate::resident::record_projection::coordinate(
                                                second_operation.handle().0,
                                                second_operation
                                                    .handle()
                                                    .1
                                                    .canonical_operation_handle_sha256()
                                                    .as_str(),
                                            )
                                            .expect("second recovery handle"),
                                            second.dispatched.clone(),
                                            None,
                                            None,
                                            second.dispatched.clone(),
                                            second_dispatch_checkpoint.clone(),
                                            second_store.clone(),
                                        );
                                    let second_dispatch_recovered =
                                        RecoveredOperationObservation::history_fixture_for_handle(
                                            crate::resident::record_projection::coordinate(
                                                second_operation.handle().0,
                                                second_operation
                                                    .handle()
                                                    .1
                                                    .canonical_operation_handle_sha256()
                                                    .as_str(),
                                            )
                                            .expect("second dispatch recovery handle"),
                                            second.dispatched.clone(),
                                            Some(second.preparation.clone()),
                                            None,
                                            second.dispatched.clone(),
                                            second_dispatch_checkpoint.clone(),
                                            second_store.clone(),
                                        );
                                    let second_recovered =
                                        RecoveredOperationObservation::history_fixture_for_handle(
                                            crate::resident::record_projection::coordinate(
                                                second_operation.handle().0,
                                                second_operation
                                                    .handle()
                                                    .1
                                                    .canonical_operation_handle_sha256()
                                                    .as_str(),
                                            )
                                            .expect("second recovery handle"),
                                            second.dispatched.clone(),
                                            Some(second.preparation.clone()),
                                            None,
                                            (
                                                second_cancellation_ref.clone(),
                                                second_cancellation.clone(),
                                            ),
                                            (
                                                second_cancellation_checkpoint_ref.clone(),
                                                second_cancellation_checkpoint.clone(),
                                            ),
                                            second_store.clone(),
                                        );
                                    let second_wrong_store_recovered =
                                        RecoveredOperationObservation::history_fixture_for_handle(
                                            crate::resident::record_projection::coordinate(
                                                second_operation.handle().0,
                                                second_operation
                                                    .handle()
                                                    .1
                                                    .canonical_operation_handle_sha256()
                                                    .as_str(),
                                            )
                                            .expect("second wrong-store recovery handle"),
                                            second.dispatched.clone(),
                                            Some(second.preparation.clone()),
                                            None,
                                            (second_cancellation_ref, second_cancellation),
                                            (
                                                second_cancellation_checkpoint_ref,
                                                second_cancellation_checkpoint,
                                            ),
                                            first_store.clone(),
                                        );
                                    let second_history = || OperationAttemptHistoryInput {
                                        operation: second_operation,
                                        request: second_envelope,
                                        session: second_operation.historical_session_lease(),
                                        lease: second_parents.lease,
                                        generation: second_generation,
                                        dispatched: &second.dispatched.1,
                                        selection: second_selection,
                                        maximum_selection_bytes: 65_536,
                                    };
                                    let first_clock = ProviderClock {
                                        times: vec![64, 65, 66, 67],
                                        next: AtomicUsize::new(0),
                                    };
                                    let mut first_checkpoint_fields =
                                        checkpoint_preimage_at_store(5, None, &first_store);
                                    first_checkpoint_fields.checkpoint_domain_id =
                                        first_checkpoint.checkpoint_domain_id().clone();
                                    first_checkpoint_fields.checkpoint_profile_ref =
                                        first_checkpoint.checkpoint_profile_ref().clone();
                                    first_checkpoint_fields.checkpoint_profile_sha256 =
                                        first_checkpoint.checkpoint_profile_sha256().clone();
                                    let first_checkpoint_signing =
                                        checkpoint_authority(&first_checkpoint_fields);
                                    let first_resolver = ProviderStore::new(first_store.clone());
                                    let first_observation_schema =
                                        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
                                    let first_invoke = ProviderOperationInvokeInput {
                                        current: first_parents.current,
                                        signing: SignedResidentAdmissionContextV1 {
                                            expected_schema_sha256: &first_observation_schema,
                                            signing_authority: first_parents.signing,
                                        },
                                        terminal_checkpoint: OperationCheckpointInput {
                                            reference: record_ref(
                                                "two-deployment/first/checkpoint-terminal",
                                            ),
                                            id: ControlCheckpointIdV1::parse(
                                                "two-deployment-first-checkpoint-terminal",
                                            )
                                            .expect("terminal checkpoint"),
                                        },
                                        result_ref: record_ref("two-deployment/first/result"),
                                        diagnostic_ref: record_ref(
                                            "two-deployment/first/diagnostic",
                                        ),
                                        reconciliation: pale_core::records::ReconciliationFrontierReferenceV1 {
                                            record_ref: record_ref(
                                                "two-deployment/first/reconciliation",
                                            ),
                                            record_sha256: pale_core::records::ReconciliationFrontierDigest::parse(
                                                "d".repeat(64),
                                            )
                                            .expect("reconciliation digest"),
                                        },
                                        terminal_ref: record_ref("two-deployment/first/terminal"),
                                        checkpoint_schema: &first_checkpoint_fields.schema_sha256,
                                        checkpoint_authority: &first_checkpoint_signing,
                                        clock: &first_clock,
                                        resolve_store: &first_resolver,
                                        maximum_record_bytes: 65_536,
                                        maximum_diagnostic_bytes: 65_536,
                                        accumulator_limits: AccumulatorPreparationLimits::new(
                                            1, 1024, 1024, 512, 65_536,
                                        )
                                        .expect("recording limits"),
                                    };
                                    let second_clock = ProviderClock {
                                        times: vec![64, 65, 66, 67],
                                        next: AtomicUsize::new(0),
                                    };
                                    let mut second_checkpoint_fields =
                                        checkpoint_preimage_at_store(5, None, &second_store);
                                    second_checkpoint_fields.checkpoint_domain_id =
                                        second_checkpoint.checkpoint_domain_id().clone();
                                    second_checkpoint_fields.checkpoint_profile_ref =
                                        second_checkpoint.checkpoint_profile_ref().clone();
                                    second_checkpoint_fields.checkpoint_profile_sha256 =
                                        second_checkpoint.checkpoint_profile_sha256().clone();
                                    let second_checkpoint_signing =
                                        checkpoint_authority(&second_checkpoint_fields);
                                    let second_resolver = ProviderStore::new(second_store.clone());
                                    let second_observation_schema =
                                        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
                                    let second_invoke = ProviderOperationInvokeInput {
                                        current: second_parents.current,
                                        signing: SignedResidentAdmissionContextV1 {
                                            expected_schema_sha256: &second_observation_schema,
                                            signing_authority: second_parents.signing,
                                        },
                                        terminal_checkpoint: OperationCheckpointInput {
                                            reference: record_ref(
                                                "two-deployment/second/checkpoint-terminal",
                                            ),
                                            id: ControlCheckpointIdV1::parse(
                                                "two-deployment-second-checkpoint-terminal",
                                            )
                                            .expect("terminal checkpoint"),
                                        },
                                        result_ref: record_ref("two-deployment/second/result"),
                                        diagnostic_ref: record_ref(
                                            "two-deployment/second/diagnostic",
                                        ),
                                        reconciliation: pale_core::records::ReconciliationFrontierReferenceV1 {
                                            record_ref: record_ref(
                                                "two-deployment/second/reconciliation",
                                            ),
                                            record_sha256: pale_core::records::ReconciliationFrontierDigest::parse(
                                                "d".repeat(64),
                                            )
                                            .expect("reconciliation digest"),
                                        },
                                        terminal_ref: record_ref("two-deployment/second/terminal"),
                                        checkpoint_schema: &second_checkpoint_fields.schema_sha256,
                                        checkpoint_authority: &second_checkpoint_signing,
                                        clock: &second_clock,
                                        resolve_store: &second_resolver,
                                        maximum_record_bytes: 65_536,
                                        maximum_diagnostic_bytes: 65_536,
                                        accumulator_limits: AccumulatorPreparationLimits::new(
                                            1, 1024, 1024, 512, 65_536,
                                        )
                                        .expect("recording limits"),
                                    };
                                    let recording_observation_signs = AtomicUsize::new(0);
                                    let recording_checkpoint_signs = AtomicUsize::new(0);
                                    let first_progress = match host.record_lost_provider(
                                        first_progress,
                                        ResidentDeploymentOperationAccess {
                                            store: &second_store,
                                            custody: second_custody,
                                            sessions: &second_sessions,
                                            anchor: second_anchor,
                                        },
                                        &mut authority,
                                        first_history(),
                                        &first_recovered,
                                        &first_invoke,
                                        |_| Ok(()),
                                        |_, _, _, _| Ok(()),
                                        |_, _, _| {
                                            recording_observation_signs
                                                .fetch_add(1, Ordering::SeqCst);
                                            Ok(signature())
                                        },
                                        |_, _, _| {
                                            recording_checkpoint_signs
                                                .fetch_add(1, Ordering::SeqCst);
                                            Ok(signature())
                                        },
                                    ) {
                                        Err(failure) => {
                                            let ResidentProcessProviderLossRecordingFailure::Selection {
                                                route,
                                                lost,
                                                reason: ResidentProcessHostSelectionError::RecoveryStoreAssociation,
                                            } = *failure
                                            else {
                                                panic!("foreign access must preserve first progress before I/O")
                                            };
                                            ResidentProcessProviderCompletion {
                                                route: ResidentProcessProviderCompletionRoute::CheckedOut(route),
                                                completion: ResidentProviderCompletionPoll::Lost(lost),
                                            }
                                        }
                                        Ok(_) => {
                                            panic!("foreign access cannot consume first progress")
                                        }
                                    };
                                    let second_progress = match host.record_completed_provider(
                                        second_progress,
                                        ResidentDeploymentOperationAccess {
                                            store: &first_store,
                                            custody: first_custody,
                                            sessions: &first_sessions,
                                            anchor: first_anchor,
                                        },
                                        &mut authority,
                                        second_history(),
                                        &second_recovered,
                                        second_admission,
                                        &second_invoke,
                                        &UnusedProviderEvidence,
                                        |_| Ok(()),
                                        |_, _, _, _| Ok(()),
                                        |_, _, _| {
                                            recording_observation_signs
                                                .fetch_add(1, Ordering::SeqCst);
                                            Ok(signature())
                                        },
                                        |_, _, _| {
                                            recording_checkpoint_signs
                                                .fetch_add(1, Ordering::SeqCst);
                                            Ok(signature())
                                        },
                                    ) {
                                        Err(failure) => {
                                            let ResidentProcessProviderCompletionRecordingFailure::Selection {
                                                route,
                                                completed,
                                                reason: ResidentProcessHostSelectionError::RecoveryStoreAssociation,
                                            } = *failure
                                            else {
                                                panic!("foreign access must preserve second progress before I/O")
                                            };
                                            ResidentProcessProviderCompletion {
                                                route: ResidentProcessProviderCompletionRoute::CheckedOut(route),
                                                completion: ResidentProviderCompletionPoll::Completed(completed),
                                            }
                                        }
                                        Ok(_) => {
                                            panic!("foreign access cannot consume second progress")
                                        }
                                    };
                                    assert_eq!(
                                        recording_observation_signs.load(Ordering::SeqCst),
                                        0
                                    );
                                    assert_eq!(
                                        recording_checkpoint_signs.load(Ordering::SeqCst),
                                        0
                                    );
                                    let first_recording = {
                                        let mut ambiguous_anchor = AmbiguousProviderAnchor {
                                            inner: first_anchor,
                                        };
                                        match host.record_lost_provider(
                                            first_progress,
                                            ResidentDeploymentOperationAccess {
                                                store: &first_store,
                                                custody: first_custody,
                                                sessions: &first_sessions,
                                                anchor: &mut ambiguous_anchor,
                                            },
                                            &mut authority,
                                            first_history(),
                                            &first_recovered,
                                            &first_invoke,
                                            |_| Ok(()),
                                            |_, _, _, _| Ok(()),
                                            |_, _, _| Ok(signature()),
                                            |_, _, _| Ok(signature()),
                                        ) {
                                            Ok(recording) => recording,
                                            Err(_) => panic!("first loss records exactly once"),
                                        }
                                    };
                                    assert!(matches!(
                                        first_recording,
                                        ResidentProcessProviderRecording::Pending {
                                            progress: ResidentProviderTaskPoll::Custody(_),
                                            ..
                                        }
                                    ));
                                    let first_recording = host
                                        .continue_provider_custody_with_finalization_mismatch_for_test(
                                            first_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &first_store,
                                                custody: first_custody,
                                                sessions: &first_sessions,
                                                anchor: first_anchor,
                                            },
                                            &mut authority,
                                            second_store.clone(),
                                        );
                                    let signs_before_finalization = (
                                        recording_observation_signs.load(Ordering::SeqCst),
                                        recording_checkpoint_signs.load(Ordering::SeqCst),
                                    );
                                    let first_recording = match host.finish_provider_recording(
                                        first_recording,
                                        ResidentDeploymentOperationAccess {
                                            store: &second_store,
                                            custody: second_custody,
                                            sessions: &second_sessions,
                                            anchor: second_anchor,
                                        },
                                        &mut authority,
                                    ) {
                                        Err(failure) => {
                                            let ResidentProcessProviderRecordingContinuationFailure::Selection {
                                                recording,
                                                reason: ResidentProcessHostSelectionError::AccessStoreAssociation,
                                            } = *failure
                                            else {
                                                panic!("foreign access preserves finalization")
                                            };
                                            recording
                                        }
                                        Ok(_) => {
                                            panic!("foreign access cannot finish finalization")
                                        }
                                    };
                                    let (first_recording, exact_selector) =
                                        replace_recording_selector(
                                            first_recording,
                                            second_selector.clone(),
                                        );
                                    let first_recording = match host.finish_provider_recording(
                                        first_recording,
                                        ResidentDeploymentOperationAccess {
                                            store: &first_store,
                                            custody: first_custody,
                                            sessions: &first_sessions,
                                            anchor: first_anchor,
                                        },
                                        &mut authority,
                                    ) {
                                        Err(failure) => {
                                            let ResidentProcessProviderRecordingContinuationFailure::Selection {
                                                recording,
                                                reason: ResidentProcessHostSelectionError::AccessStoreAssociation,
                                            } = *failure
                                            else {
                                                panic!("wrong selector preserves finalization")
                                            };
                                            recording
                                        }
                                        Ok(_) => {
                                            panic!("wrong selector cannot finish finalization")
                                        }
                                    };
                                    let (first_recording, _) =
                                        replace_recording_selector(first_recording, exact_selector);
                                    assert_eq!(host.provider_routes.checked_out, 2);
                                    assert_eq!(
                                        (
                                            recording_observation_signs.load(Ordering::SeqCst),
                                            recording_checkpoint_signs.load(Ordering::SeqCst),
                                        ),
                                        signs_before_finalization,
                                    );
                                    let first_recording = host
                                        .finish_provider_recording(
                                            first_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &first_store,
                                                custody: first_custody,
                                                sessions: &first_sessions,
                                                anchor: first_anchor,
                                            },
                                            &mut authority,
                                        )
                                        .unwrap_or_else(|_| {
                                            panic!("exact access reaches semantic finalization")
                                        });
                                    assert!(matches!(
                                        first_recording,
                                        ResidentProcessProviderRecording::Pending {
                                            progress: ResidentProviderTaskPoll::Finalization {
                                                reason: OperationAttemptError::Association,
                                                ..
                                            },
                                            ..
                                        }
                                    ));
                                    let first_recording = host
                                        .finish_provider_recording(
                                            first_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &first_store,
                                                custody: first_custody,
                                                sessions: &first_sessions,
                                                anchor: first_anchor,
                                            },
                                            &mut authority,
                                        )
                                        .unwrap_or_else(|_| {
                                            panic!("repeated mismatch retains semantic phase")
                                        });
                                    assert!(matches!(
                                        first_recording,
                                        ResidentProcessProviderRecording::Pending {
                                            progress: ResidentProviderTaskPoll::Finalization {
                                                reason: OperationAttemptError::Association,
                                                ..
                                            },
                                            ..
                                        }
                                    ));
                                    assert_eq!(host.provider_routes.checked_out, 2);
                                    assert_eq!(
                                        (
                                            recording_observation_signs.load(Ordering::SeqCst),
                                            recording_checkpoint_signs.load(Ordering::SeqCst),
                                        ),
                                        signs_before_finalization,
                                    );
                                    host.provider_routes.checked_out = 0;
                                    let first_recording = host
                                        .restore_provider_finalization_for_test(
                                            first_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &first_store,
                                                custody: first_custody,
                                                sessions: &first_sessions,
                                                anchor: first_anchor,
                                            },
                                            &mut authority,
                                        );
                                    assert!(matches!(
                                        first_recording,
                                        ResidentProcessProviderRecording::RouteReleaseRejected { .. }
                                    ));
                                    host.provider_routes.checked_out = 2;
                                    let first_recording = host
                                        .continue_provider_route_release(first_recording)
                                        .unwrap_or_else(|_| {
                                            panic!("the owning host releases the first exact route")
                                        });
                                    assert!(matches!(
                                        first_recording,
                                        ResidentProcessProviderRecording::Recorded { .. }
                                    ));
                                    assert_eq!(host.provider_routes.checked_out, 1);

                                    let second_recording = match host.record_completed_provider(
                                        second_progress,
                                        ResidentDeploymentOperationAccess {
                                            store: &second_store,
                                            custody: second_custody,
                                            sessions: &second_sessions,
                                            anchor: second_anchor,
                                        },
                                        &mut authority,
                                        second_history(),
                                        &second_rejected_recovered,
                                        second_admission,
                                        &second_invoke,
                                        &UnusedProviderEvidence,
                                        |_| Ok(()),
                                        |_, _, _, _| Ok(()),
                                        |_, _, _| Ok(signature()),
                                        |_, _, _| Ok(signature()),
                                    ) {
                                        Ok(recording) => recording,
                                        Err(_) => {
                                            panic!("second completion reaches protected recovery")
                                        }
                                    };
                                    assert!(matches!(
                                        second_recording,
                                        ResidentProcessProviderRecording::Pending {
                                            progress: ResidentProviderTaskPoll::RecoveryRejected { .. },
                                            ..
                                        }
                                    ));
                                    let second_recording = match host
                                        .continue_provider_completion_recovery(
                                            second_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &first_store,
                                                custody: first_custody,
                                                sessions: &first_sessions,
                                                anchor: first_anchor,
                                            },
                                            &mut authority,
                                            second_history(),
                                            &second_dispatch_recovered,
                                            second_admission,
                                            &second_invoke,
                                            &UnusedProviderEvidence,
                                            |_| Ok(()),
                                            |_, _, _, _| Ok(()),
                                            |_, _, _| Ok(signature()),
                                            |_, _, _| Ok(signature()),
                                        ) {
                                        Err(failure) => {
                                            let ResidentProcessProviderCompletionRecordingFailure::Selection {
                                                route,
                                                completed,
                                                reason: ResidentProcessHostSelectionError::RecoveryStoreAssociation,
                                            } = *failure
                                            else {
                                                panic!("foreign recovery access preserves completion")
                                            };
                                            ResidentProcessProviderRecording::Pending {
                                                route,
                                                progress:
                                                    ResidentProviderTaskPoll::RecoveryRejected {
                                                        completed,
                                                        reason: OperationAttemptError::Association,
                                                    },
                                            }
                                        }
                                        Ok(_) => panic!("foreign access cannot continue recovery"),
                                    };
                                    let second_recording = host
                                        .continue_provider_completion_recovery(
                                            second_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &second_store,
                                                custody: second_custody,
                                                sessions: &second_sessions,
                                                anchor: second_anchor,
                                            },
                                            &mut authority,
                                            second_history(),
                                            &second_dispatch_recovered,
                                            second_admission,
                                            &second_invoke,
                                            &UnusedProviderEvidence,
                                            |_| Ok(()),
                                            |_, _, _, _| Ok(()),
                                            |_, _, _| {
                                                Err(InvalidResidentRecordV1::ContextualAdmission {
                                                    record: "provider terminal observation",
                                                    invariant: "hostile signer accepts only retry",
                                                })
                                            },
                                            |_, _, _| Ok(signature()),
                                        )
                                        .unwrap_or_else(|_| {
                                            panic!("the owning deployment continues exact recovery")
                                        });
                                    assert!(matches!(
                                        second_recording,
                                        ResidentProcessProviderRecording::Pending {
                                            progress: ResidentProviderTaskPoll::Preparation { .. },
                                            ..
                                        }
                                    ));
                                    let second_recording = match host.continue_provider_recording(
                                        second_recording,
                                        ResidentDeploymentOperationAccess {
                                            store: &first_store,
                                            custody: first_custody,
                                            sessions: &first_sessions,
                                            anchor: first_anchor,
                                        },
                                        &mut authority,
                                        &second_invoke,
                                        |_| Ok(()),
                                        |_, _, _, _| Ok(()),
                                        |_, _, _| Ok(signature()),
                                        |_, _, _| Ok(signature()),
                                    ) {
                                        Err(failure) => {
                                            let ResidentProcessProviderRecordingContinuationFailure::Selection {
                                                recording,
                                                reason: ResidentProcessHostSelectionError::AccessStoreAssociation,
                                            } = *failure
                                            else {
                                                panic!("foreign access preserves preparation")
                                            };
                                            recording
                                        }
                                        Ok(_) => {
                                            panic!("foreign access cannot continue preparation")
                                        }
                                    };
                                    let second_recording = host
                                        .continue_provider_recording(
                                            second_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &second_store,
                                                custody: second_custody,
                                                sessions: &second_sessions,
                                                anchor: second_anchor,
                                            },
                                            &mut authority,
                                            &second_invoke,
                                            |_| Ok(()),
                                            |_, _, _, _| Ok(()),
                                            |_, _, _| Ok(signature()),
                                            |_, _, _| Ok(signature()),
                                        )
                                        .unwrap_or_else(|_| {
                                            panic!("the owning deployment continues preparation")
                                        });
                                    assert!(matches!(
                                        second_recording,
                                        ResidentProcessProviderRecording::Pending {
                                            progress: ResidentProviderTaskPoll::ConcurrentHead(_),
                                            ..
                                        }
                                    ));
                                    let signs_before_head = (
                                        recording_observation_signs.load(Ordering::SeqCst),
                                        recording_checkpoint_signs.load(Ordering::SeqCst),
                                    );
                                    let second_recording = match host
                                        .continue_provider_after_concurrent_head(
                                            second_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &first_store,
                                                custody: first_custody,
                                                sessions: &first_sessions,
                                                anchor: first_anchor,
                                            },
                                            &mut authority,
                                            &second_recovered,
                                            &second_invoke,
                                            |_| panic!("foreign access reached reconciliation"),
                                            |_, _, _, _| panic!("foreign access reached rotation"),
                                            |_, _, _| panic!("foreign access reached signing"),
                                            |_, _, _| {
                                                panic!("foreign access reached checkpoint signing")
                                            },
                                        ) {
                                        Err(failure) => {
                                            let ResidentProcessProviderRecordingContinuationFailure::Selection {
                                                recording,
                                                reason: ResidentProcessHostSelectionError::AccessStoreAssociation,
                                            } = *failure
                                            else {
                                                panic!("foreign access preserves concurrent head")
                                            };
                                            recording
                                        }
                                        Ok(_) => panic!("foreign access cannot continue head"),
                                    };
                                    let (second_recording, exact_selector) =
                                        replace_recording_selector(
                                            second_recording,
                                            first_selector.clone(),
                                        );
                                    let second_recording = match host
                                        .continue_provider_after_concurrent_head(
                                            second_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &second_store,
                                                custody: second_custody,
                                                sessions: &second_sessions,
                                                anchor: second_anchor,
                                            },
                                            &mut authority,
                                            &second_recovered,
                                            &second_invoke,
                                            |_| panic!("wrong selector reached reconciliation"),
                                            |_, _, _, _| panic!("wrong selector reached rotation"),
                                            |_, _, _| panic!("wrong selector reached signing"),
                                            |_, _, _| {
                                                panic!("wrong selector reached checkpoint signing")
                                            },
                                        ) {
                                        Err(failure) => {
                                            let ResidentProcessProviderRecordingContinuationFailure::Selection {
                                                recording,
                                                reason: ResidentProcessHostSelectionError::AccessStoreAssociation,
                                            } = *failure
                                            else {
                                                panic!("wrong selector preserves concurrent head")
                                            };
                                            recording
                                        }
                                        Ok(_) => panic!("wrong selector cannot continue head"),
                                    };
                                    let (second_recording, _) = replace_recording_selector(
                                        second_recording,
                                        exact_selector,
                                    );
                                    let second_recording = match host
                                        .continue_provider_after_concurrent_head(
                                            second_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &second_store,
                                                custody: second_custody,
                                                sessions: &second_sessions,
                                                anchor: second_anchor,
                                            },
                                            &mut authority,
                                            &second_wrong_store_recovered,
                                            &second_invoke,
                                            |_| panic!("wrong recovery store reached reconciliation"),
                                            |_, _, _, _| panic!("wrong recovery store reached rotation"),
                                            |_, _, _| panic!("wrong recovery store reached signing"),
                                            |_, _, _| {
                                                panic!("wrong recovery store reached checkpoint signing")
                                            },
                                        ) {
                                        Err(failure) => {
                                            let ResidentProcessProviderRecordingContinuationFailure::Selection {
                                                recording,
                                                reason: ResidentProcessHostSelectionError::RecoveryStoreAssociation,
                                            } = *failure
                                            else {
                                                panic!("wrong recovery store preserves concurrent head")
                                            };
                                            recording
                                        }
                                        Ok(_) => panic!("wrong recovery store cannot continue head"),
                                    };
                                    let mut wrong_second_current =
                                        second_parents.current.at_time(second_parents.current.now);
                                    wrong_second_current.runtime_authority = None;
                                    let wrong_current_invoke = ProviderOperationInvokeInput {
                                        current: &wrong_second_current,
                                        signing: second_invoke.signing,
                                        terminal_checkpoint: OperationCheckpointInput {
                                            reference: second_invoke
                                                .terminal_checkpoint
                                                .reference
                                                .clone(),
                                            id: second_invoke.terminal_checkpoint.id.clone(),
                                        },
                                        result_ref: second_invoke.result_ref.clone(),
                                        diagnostic_ref: second_invoke.diagnostic_ref.clone(),
                                        reconciliation: second_invoke.reconciliation.clone(),
                                        terminal_ref: second_invoke.terminal_ref.clone(),
                                        checkpoint_schema: second_invoke.checkpoint_schema,
                                        checkpoint_authority: second_invoke.checkpoint_authority,
                                        clock: second_invoke.clock,
                                        resolve_store: second_invoke.resolve_store,
                                        maximum_record_bytes: second_invoke.maximum_record_bytes,
                                        maximum_diagnostic_bytes: second_invoke
                                            .maximum_diagnostic_bytes,
                                        accumulator_limits: second_invoke.accumulator_limits,
                                    };
                                    let second_recording = match host
                                        .continue_provider_after_concurrent_head(
                                            second_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &second_store,
                                                custody: second_custody,
                                                sessions: &second_sessions,
                                                anchor: second_anchor,
                                            },
                                            &mut authority,
                                            &second_recovered,
                                            &wrong_current_invoke,
                                            |_| panic!("wrong current reached reconciliation"),
                                            |_, _, _, _| panic!("wrong current reached rotation"),
                                            |_, _, _| panic!("wrong current reached signing"),
                                            |_, _, _| {
                                                panic!("wrong current reached checkpoint signing")
                                            },
                                        ) {
                                        Err(failure) => {
                                            let ResidentProcessProviderRecordingContinuationFailure::Selection {
                                                recording,
                                                reason: ResidentProcessHostSelectionError::ProviderCurrentAuthority,
                                            } = *failure
                                            else {
                                                panic!("wrong current preserves concurrent head")
                                            };
                                            recording
                                        }
                                        Ok(_) => panic!("wrong current cannot continue head"),
                                    };
                                    let wrong_signing_invoke = ProviderOperationInvokeInput {
                                        current: second_parents.current,
                                        signing: SignedResidentAdmissionContextV1 {
                                            expected_schema_sha256: &second_observation_schema,
                                            signing_authority: first_parents.signing,
                                        },
                                        terminal_checkpoint: OperationCheckpointInput {
                                            reference: second_invoke
                                                .terminal_checkpoint
                                                .reference
                                                .clone(),
                                            id: second_invoke.terminal_checkpoint.id.clone(),
                                        },
                                        result_ref: second_invoke.result_ref.clone(),
                                        diagnostic_ref: second_invoke.diagnostic_ref.clone(),
                                        reconciliation: second_invoke.reconciliation.clone(),
                                        terminal_ref: second_invoke.terminal_ref.clone(),
                                        checkpoint_schema: second_invoke.checkpoint_schema,
                                        checkpoint_authority: second_invoke.checkpoint_authority,
                                        clock: second_invoke.clock,
                                        resolve_store: second_invoke.resolve_store,
                                        maximum_record_bytes: second_invoke.maximum_record_bytes,
                                        maximum_diagnostic_bytes: second_invoke
                                            .maximum_diagnostic_bytes,
                                        accumulator_limits: second_invoke.accumulator_limits,
                                    };
                                    let second_recording = match host
                                        .continue_provider_after_concurrent_head(
                                            second_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &second_store,
                                                custody: second_custody,
                                                sessions: &second_sessions,
                                                anchor: second_anchor,
                                            },
                                            &mut authority,
                                            &second_recovered,
                                            &wrong_signing_invoke,
                                            |_| panic!("wrong signer reached reconciliation"),
                                            |_, _, _, _| panic!("wrong signer reached rotation"),
                                            |_, _, _| panic!("wrong signer reached signing"),
                                            |_, _, _| {
                                                panic!("wrong signer reached checkpoint signing")
                                            },
                                        ) {
                                        Err(failure) => {
                                            let ResidentProcessProviderRecordingContinuationFailure::Selection {
                                                recording,
                                                reason: ResidentProcessHostSelectionError::ProviderCurrentAuthority,
                                            } = *failure
                                            else {
                                                panic!("wrong signer preserves concurrent head")
                                            };
                                            recording
                                        }
                                        Ok(_) => panic!("wrong signer cannot continue head"),
                                    };
                                    let wrong_checkpoint_invoke = ProviderOperationInvokeInput {
                                        current: second_parents.current,
                                        signing: second_invoke.signing,
                                        terminal_checkpoint: OperationCheckpointInput {
                                            reference: second_invoke
                                                .terminal_checkpoint
                                                .reference
                                                .clone(),
                                            id: second_invoke.terminal_checkpoint.id.clone(),
                                        },
                                        result_ref: second_invoke.result_ref.clone(),
                                        diagnostic_ref: second_invoke.diagnostic_ref.clone(),
                                        reconciliation: second_invoke.reconciliation.clone(),
                                        terminal_ref: second_invoke.terminal_ref.clone(),
                                        checkpoint_schema: second_invoke.checkpoint_schema,
                                        checkpoint_authority: &first_checkpoint_signing,
                                        clock: second_invoke.clock,
                                        resolve_store: second_invoke.resolve_store,
                                        maximum_record_bytes: second_invoke.maximum_record_bytes,
                                        maximum_diagnostic_bytes: second_invoke
                                            .maximum_diagnostic_bytes,
                                        accumulator_limits: second_invoke.accumulator_limits,
                                    };
                                    let second_recording = match host
                                        .continue_provider_after_concurrent_head(
                                            second_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &second_store,
                                                custody: second_custody,
                                                sessions: &second_sessions,
                                                anchor: second_anchor,
                                            },
                                            &mut authority,
                                            &second_recovered,
                                            &wrong_checkpoint_invoke,
                                            |_| panic!("wrong checkpoint reached reconciliation"),
                                            |_, _, _, _| {
                                                panic!("wrong checkpoint reached rotation")
                                            },
                                            |_, _, _| panic!("wrong checkpoint reached signing"),
                                            |_, _, _| {
                                                panic!(
                                                    "wrong checkpoint reached checkpoint signing"
                                                )
                                            },
                                        ) {
                                        Err(failure) => {
                                            let ResidentProcessProviderRecordingContinuationFailure::Selection {
                                                recording,
                                                reason: ResidentProcessHostSelectionError::ProviderCurrentAuthority,
                                            } = *failure
                                            else {
                                                panic!("wrong checkpoint preserves concurrent head")
                                            };
                                            recording
                                        }
                                        Ok(_) => panic!("wrong checkpoint cannot continue head"),
                                    };
                                    assert_eq!(
                                        (
                                            recording_observation_signs.load(Ordering::SeqCst),
                                            recording_checkpoint_signs.load(Ordering::SeqCst),
                                        ),
                                        signs_before_head,
                                    );
                                    host.operations.set_provider_reservation_phase_for_test(
                                        &second.key,
                                        ResidentTaskReservationPhase::Accepted,
                                    );
                                    let second_recording = match host
                                        .continue_provider_after_concurrent_head(
                                            second_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &second_store,
                                                custody: second_custody,
                                                sessions: &second_sessions,
                                                anchor: second_anchor,
                                            },
                                            &mut authority,
                                            &second_recovered,
                                            &second_invoke,
                                            |_| panic!("phase refusal reached reconciliation"),
                                            |_, _, _, _| panic!("phase refusal reached rotation"),
                                            |_, _, _| panic!("phase refusal reached signing"),
                                            |_, _, _| {
                                                panic!("phase refusal reached checkpoint signing")
                                            },
                                        ) {
                                        Err(failure) => {
                                            let ResidentProcessProviderRecordingContinuationFailure::ConcurrentHead {
                                                route,
                                                rejection,
                                            } = *failure
                                            else {
                                                panic!("lower phase refusal retains concurrent head")
                                            };
                                            assert_eq!(
                                                rejection.reason,
                                                crate::resident::task_reservation::ResidentTaskReservationTransitionFailure::Phase {
                                                    expected: ResidentTaskReservationPhase::CompletionPendingDurableHandling,
                                                    actual: ResidentTaskReservationPhase::Accepted,
                                                },
                                            );
                                            ResidentProcessProviderRecording::Pending {
                                                route,
                                                progress: ResidentProviderTaskPoll::ConcurrentHead(
                                                    rejection.conflict,
                                                ),
                                            }
                                        }
                                        Ok(_) => {
                                            panic!("wrong aggregate phase cannot continue head")
                                        }
                                    };
                                    assert_eq!(
                                        (
                                            recording_observation_signs.load(Ordering::SeqCst),
                                            recording_checkpoint_signs.load(Ordering::SeqCst),
                                        ),
                                        signs_before_head,
                                    );
                                    host.operations.set_provider_reservation_phase_for_test(
                                        &second.key,
                                        ResidentTaskReservationPhase::CompletionPendingDurableHandling,
                                    );
                                    let second_recording = host
                                        .continue_provider_after_concurrent_head(
                                            second_recording,
                                            ResidentDeploymentOperationAccess {
                                                store: &second_store,
                                                custody: second_custody,
                                                sessions: &second_sessions,
                                                anchor: second_anchor,
                                            },
                                            &mut authority,
                                            &second_recovered,
                                            &second_invoke,
                                            |_| Ok(()),
                                            |_, _, _, _| Ok(()),
                                            |_, _, _| {
                                                recording_observation_signs
                                                    .fetch_add(1, Ordering::SeqCst);
                                                Ok(signature())
                                            },
                                            |_, _, _| {
                                                recording_checkpoint_signs
                                                    .fetch_add(1, Ordering::SeqCst);
                                                Ok(signature())
                                            },
                                        )
                                        .unwrap_or_else(|_| {
                                            panic!(
                                                "the owning deployment continues concurrent head"
                                            )
                                        });
                                    assert!(matches!(
                                        second_recording,
                                        ResidentProcessProviderRecording::Pending {
                                            progress: ResidentProviderTaskPoll::Preparation { .. },
                                            ..
                                        }
                                    ));
                                    assert_eq!(host.provider_routes.checked_out, 1);
                                    let reusable = host
                                        .provider_routes
                                        .reserve(first_selector.clone())
                                        .expect("the first released slot remains reusable");
                                    assert!(host
                                        .provider_routes
                                        .release_reservation(reusable)
                                        .is_ok());
                                    let _retained_second_recording = second_recording;
                                },
                            );
                        },
                    );
                },
            );
        },
    );
}

#[test]
fn an_unrouted_completion_cannot_consume_another_deployments_route() {
    let limits = ResidentOperationTaskLimits::new(
        NonZeroUsize::new(2).expect("positive provider task bound"),
    );
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut routes =
        ResidentProcessProviderRoutes::new(owner, limits).expect("preallocated routes");
    let first_store = store("process-provider-first", "incarnation-1");
    let second_store = store("process-provider-second", "incarnation-1");
    let first_selector = unproved_selector(principal("principal-a"), first_store.clone());
    let second_selector = unproved_selector(principal("principal-a"), second_store.clone());
    let first_key = provider_task_key(first_store.clone(), "provider/dispatch-first", 'a');
    let second_key = provider_task_key(second_store.clone(), "provider/dispatch-second", 'b');
    let first_slot = routes
        .reserve(first_selector.clone())
        .expect("first route slot");
    assert!(routes.commit(first_slot, first_key.clone()).is_ok());
    let second_slot = routes
        .reserve(second_selector.clone())
        .expect("second route slot");
    assert!(routes.commit(second_slot, second_key.clone()).is_ok());
    let foreign = provider_task_key(
        store("process-provider-foreign", "incarnation-1"),
        "provider/dispatch-foreign",
        'c',
    );

    assert!(routes.check_out(&foreign).is_none());
    assert_eq!(routes.selector(&first_key), Some(&first_selector));
    assert_eq!(routes.selector(&second_key), Some(&second_selector));
}

#[test]
fn an_unaccepted_launch_releases_its_exact_preallocated_route_slot() {
    let limits = ResidentOperationTaskLimits::new(
        NonZeroUsize::new(1).expect("positive provider task bound"),
    );
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut routes =
        ResidentProcessProviderRoutes::new(owner, limits).expect("preallocated routes");
    let first = unproved_selector(
        principal("principal-a"),
        store("process-provider-first", "incarnation-1"),
    );
    let second = unproved_selector(
        principal("principal-a"),
        store("process-provider-second", "incarnation-1"),
    );

    let pending = routes.reserve(first).expect("first route slot");
    assert!(routes.reserve(second).is_err());
    assert!(routes.release_reservation(pending).is_ok());
    assert!(routes
        .reserve(unproved_selector(
            principal("principal-a"),
            store("process-provider-second", "incarnation-1"),
        ))
        .is_ok());
}

#[test]
fn abandoning_a_retention_failure_releases_only_its_host_route_slot() {
    let selected_principal = principal("principal-a");
    let selected = unproved_selector(
        selected_principal.clone(),
        store("process-provider-retention", "incarnation-1"),
    );
    let mut host = ResidentProcessHost::new(
        unproved_writer(selected_principal, Vec::new()),
        Vec::new(),
        NonZeroUsize::new(1).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("empty process host");
    let reservation = host
        .provider_routes
        .reserve(selected.clone())
        .expect("provider route reservation");
    let failure = ResidentProcessProviderLaunchFailure::Retention {
        reservation,
        failure: ResidentProviderDispatchRetentionFailure::Retention(
            ProviderOperationRetentionFailure::Admission(OperationAttemptError::Association),
        ),
    };

    let abandoned = host
        .abandon_unaccepted_provider_launch(failure)
        .unwrap_or_else(|_| panic!("retention failure remained unopened"));
    assert!(matches!(
        abandoned,
        ResidentProcessProviderAbandonedLaunch::Retention(
            ResidentProviderDispatchRetentionFailure::Retention(
                ProviderOperationRetentionFailure::Admission(OperationAttemptError::Association)
            )
        )
    ));
    assert!(host.provider_routes.reserve(selected).is_ok());
}

#[test]
fn route_tokens_reject_a_rebuilt_host_and_remain_usable_by_the_owner() {
    let limits = ResidentOperationTaskLimits::new(
        NonZeroUsize::new(1).expect("positive provider task bound"),
    );
    let mut original =
        ResidentProcessProviderRoutes::new(ResidentOperationTaskOwnerInstance::new(), limits)
            .expect("original routes");
    let mut replacement =
        ResidentProcessProviderRoutes::new(ResidentOperationTaskOwnerInstance::new(), limits)
            .expect("replacement routes");
    let selected_store = store("process-provider-first", "incarnation-1");
    let selector = unproved_selector(principal("principal-a"), selected_store.clone());
    let key = provider_task_key(selected_store, "provider/dispatch-first", 'a');
    let reservation = original.reserve(selector).expect("original route slot");
    let reservation = replacement
        .release_reservation(reservation)
        .expect_err("replacement host rejects route reservation");
    assert_eq!(replacement.reserved, 0);
    assert_eq!(original.reserved, 1);
    assert!(original.commit(*reservation, key.clone()).is_ok());

    let custody = original.check_out(&key).expect("checked-out route custody");
    let custody = replacement
        .release_custody(custody)
        .expect_err("replacement host rejects checked-out route custody");
    assert_eq!(replacement.checked_out, 0);
    assert_eq!(original.checked_out, 1);
    assert!(original.release_custody(*custody).is_ok());
    assert_eq!(original.checked_out, 0);
}

#[derive(Clone, Copy)]
enum ProviderProcessCase {
    RefusedThenLost,
    RefusedAggregateTransition,
    DuplicateAggregateTransition,
    Completed,
    AcceptedAggregateTransition,
    ProtocolLoss,
    ProtocolLossAggregateTransition,
    Stale,
}

fn provider_launch_and_poll_case(case: ProviderProcessCase) {
    with_provider_selection_recovery_context(
        LocalStartContext::SelfContained,
        |provider_request,
         operation,
         request,
         generation,
         admission,
         parents,
         initial_checkpoint,
         initial_record,
         initial_candidate| {
            let selected_store = initial_candidate.store().clone();
            let selected_selector = selector_from_signing_authority(parents.signing);
            let other_store = store("process-provider-other", "incarnation-1");
            let other_selector =
                unproved_selector(selected_selector.principal_id.clone(), other_store.clone());
            let selected_port = MemoryAuthorityStore::default();
            let other_port = MemoryAuthorityStore::default();
            let selected_deployment = unproved_deployment_for_selector(
                &selected_selector,
                selected_store.clone(),
                ready_application(&selected_port, selected_store.clone(), 1),
            );
            let other_deployment = unproved_deployment_for_selector(
                &other_selector,
                other_store.clone(),
                ready_application(&other_port, other_store.clone(), 2),
            );
            let mut host = ResidentProcessHost::new(
                unproved_writer(
                    selected_selector.principal_id.clone(),
                    vec![selected_store.clone(), other_store],
                ),
                vec![selected_deployment, other_deployment],
                NonZeroUsize::new(2).expect("two deployment bound"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("two deployment process host");

            let (handle_ref, handle) = operation.handle();
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let initial = issue_observation(
                parents.signing,
                handle_ref,
                handle,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(1).expect("initial revision"),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                60,
            );
            assert_eq!(
                initial_record.canonical_record_jcs().as_slice(),
                pale_core::canon::to_canonical_string(&initial)
                    .expect("initial observation encoding")
                    .as_bytes(),
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
            let checkpoint_input = |suffix: &str| OperationCheckpointInput {
                reference: record_ref(&format!("process-provider/checkpoint-{suffix}")),
                id: ControlCheckpointIdV1::parse(format!("process-provider-checkpoint-{suffix}"))
                    .expect("provider checkpoint identity"),
            };
            let clock = ProviderClock {
                times: vec![61, 62, 63, 64, 65, 66, 67, 68],
                next: AtomicUsize::new(0),
            };
            let store_resolver = ProviderStore::new(selected_store.clone());
            let initial_checkpoint_ref = record_ref("operation/checkpoint-1");
            let handle_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    handle_ref,
                    handle.canonical_operation_handle_sha256().as_str(),
                )
                .expect("provider handle coordinate"),
                handle,
            )
            .expect("provider handle record");
            let lease_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    handle.operation_lease_ref(),
                    parents.lease.operation_lease_digest().as_str(),
                )
                .expect("provider lease coordinate"),
                parents.lease,
            )
            .expect("provider lease record");
            let session = parents.session.lease();
            let sessions = ProviderSessionRead(SessionLeaseBody::new(
                selected_store.clone(),
                session.session_admission_lease_digest().clone(),
                session.schema_id().clone(),
                session.schema_sha256().clone(),
                ControlRecordBytes::new(
                    pale_core::canon::to_canonical_string(
                        &pale_core::records::SessionAdmissionLeaseWireV1::from(session),
                    )
                    .expect("session lease encoding")
                    .into_bytes(),
                )
                .expect("session lease bytes"),
            ));
            let competing_head = Rc::new(RefCell::new(None));
            custody::with_service_custody_and_handle(
                initial_checkpoint,
                Some(initial_candidate),
                Some(lease_record),
                3,
                3,
                false,
                &[],
                Some(handle_record),
                competing_head,
                |custody, anchor| {
                    let mut authority = ProvingAuthority;
                    let mut executor = ProviderExecutor::default();
                    let dispatch = ProviderOperationDispatchInput {
                        operation: ProviderOperationInput {
                            context: OperationAttemptContext {
                                operation,
                                request,
                                session: parents.session,
                                current: parents.current,
                                lease: parents.lease,
                                context_binding: None,
                                generation,
                                selection_ref: record_ref("process-provider/selection"),
                                maximum_selection_bytes: 65_536,
                                observation_ref: record_ref("process-provider/preparation"),
                                observation,
                            },
                            invocation: provider_request,
                            result_admission: admission,
                            resolution_ref: record_ref("process-provider/resolution"),
                        },
                        checkpoint: (&initial_checkpoint_ref, initial_checkpoint),
                        checkpoint_schema: &checkpoint_fields.schema_sha256,
                        checkpoint_authority: &checkpoint_signing,
                        signing: observation.signed,
                        resolution_checkpoint: checkpoint_input("resolved"),
                        preparation_checkpoint: checkpoint_input("prepared"),
                        dispatch_checkpoint: checkpoint_input("dispatched"),
                        dispatch_ref: record_ref("process-provider/dispatched"),
                        clock: &clock,
                        resolve_store: &store_resolver,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            1, 1024, 1024, 512, 65_536,
                        )
                        .expect("provider accumulator limits"),
                    };
                    let invocation = ProviderOperationInvokeInput {
                        current: parents.current,
                        signing: observation.signed,
                        terminal_checkpoint: checkpoint_input("terminal"),
                        result_ref: record_ref("process-provider/result"),
                        diagnostic_ref: record_ref("process-provider/diagnostic"),
                        reconciliation: pale_core::records::ReconciliationFrontierReferenceV1 {
                            record_ref: record_ref("process-provider/reconciliation"),
                            record_sha256: pale_core::records::ReconciliationFrontierDigest::parse(
                                "d".repeat(64),
                            )
                            .expect("reconciliation digest"),
                        },
                        terminal_ref: record_ref("process-provider/terminal"),
                        checkpoint_schema: &checkpoint_fields.schema_sha256,
                        checkpoint_authority: &checkpoint_signing,
                        clock: &clock,
                        resolve_store: &store_resolver,
                        maximum_record_bytes: 65_536,
                        maximum_diagnostic_bytes: 65_536,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            1, 1024, 1024, 512, 65_536,
                        )
                        .expect("provider invocation limits"),
                    };
                    let observation_signatures = AtomicUsize::new(0);
                    let checkpoint_signatures = AtomicUsize::new(0);
                    let dispatch = match host.launch_provider(
                        &other_selector,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody: &mut *custody,
                            sessions: &sessions,
                            anchor: &mut *anchor,
                        },
                        &mut authority,
                        dispatch,
                        &invocation,
                        &mut executor,
                        |_, _, _, _| Ok(()),
                        |_, _, _| {
                            observation_signatures.fetch_add(1, Ordering::SeqCst);
                            Ok(signature())
                        },
                        |_, _, _| {
                            checkpoint_signatures.fetch_add(1, Ordering::SeqCst);
                            Ok(signature())
                        },
                    ) {
                        Err(failure) => match *failure {
                            ResidentProcessProviderLaunchFailure::Selection {
                                dispatch,
                                reason: ResidentProcessHostSelectionError::AccessStoreAssociation,
                            } => *dispatch,
                            _ => panic!("wrong deployment must preserve the unopened dispatch"),
                        },
                        Ok(_) => panic!("wrong deployment must preserve the unopened dispatch"),
                    };
                    assert_eq!(clock.next.load(Ordering::SeqCst), 0);
                    assert!(executor.accepted.is_empty());
                    assert_eq!(observation_signatures.load(Ordering::SeqCst), 0);
                    assert_eq!(checkpoint_signatures.load(Ordering::SeqCst), 0);

                    let mut wrong_current = parents.current.at_time(parents.current.now);
                    wrong_current.runtime_authority = None;
                    let wrong_current_invocation = ProviderOperationInvokeInput {
                        current: &wrong_current,
                        signing: observation.signed,
                        terminal_checkpoint: checkpoint_input("terminal"),
                        result_ref: record_ref("process-provider/result"),
                        diagnostic_ref: record_ref("process-provider/diagnostic"),
                        reconciliation: invocation.reconciliation.clone(),
                        terminal_ref: record_ref("process-provider/terminal"),
                        checkpoint_schema: &checkpoint_fields.schema_sha256,
                        checkpoint_authority: &checkpoint_signing,
                        clock: &clock,
                        resolve_store: &store_resolver,
                        maximum_record_bytes: 65_536,
                        maximum_diagnostic_bytes: 65_536,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            1, 1024, 1024, 512, 65_536,
                        )
                        .expect("provider invocation limits"),
                    };
                    let dispatch = match host.launch_provider(
                        &selected_selector,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions: &sessions,
                            anchor,
                        },
                        &mut authority,
                        dispatch,
                        &wrong_current_invocation,
                        &mut executor,
                        |_, _, _, _| Ok(()),
                        |_, _, _| {
                            observation_signatures.fetch_add(1, Ordering::SeqCst);
                            Ok(signature())
                        },
                        |_, _, _| {
                            checkpoint_signatures.fetch_add(1, Ordering::SeqCst);
                            Ok(signature())
                        },
                    ) {
                        Err(failure) => match *failure {
                            ResidentProcessProviderLaunchFailure::Selection {
                                dispatch,
                                reason: ResidentProcessHostSelectionError::ProviderCurrentAuthority,
                            } => *dispatch,
                            _ => panic!("wrong current authority must preserve the dispatch"),
                        },
                        Ok(_) => panic!("wrong current authority must fail before I/O"),
                    };
                    let mut wrong_checkpoint_fields = checkpoint_fields.clone();
                    wrong_checkpoint_fields.control_store_id =
                        ControlStoreIdV1::parse("process-provider-other")
                            .expect("other checkpoint store");
                    let wrong_checkpoint = checkpoint_authority(&wrong_checkpoint_fields);
                    let wrong_checkpoint_invocation = ProviderOperationInvokeInput {
                        current: parents.current,
                        signing: observation.signed,
                        terminal_checkpoint: checkpoint_input("terminal"),
                        result_ref: record_ref("process-provider/result"),
                        diagnostic_ref: record_ref("process-provider/diagnostic"),
                        reconciliation: invocation.reconciliation.clone(),
                        terminal_ref: record_ref("process-provider/terminal"),
                        checkpoint_schema: &checkpoint_fields.schema_sha256,
                        checkpoint_authority: &wrong_checkpoint,
                        clock: &clock,
                        resolve_store: &store_resolver,
                        maximum_record_bytes: 65_536,
                        maximum_diagnostic_bytes: 65_536,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            1, 1024, 1024, 512, 65_536,
                        )
                        .expect("provider invocation limits"),
                    };
                    let dispatch = match host.launch_provider(
                        &selected_selector,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions: &sessions,
                            anchor,
                        },
                        &mut authority,
                        dispatch,
                        &wrong_checkpoint_invocation,
                        &mut executor,
                        |_, _, _, _| Ok(()),
                        |_, _, _| {
                            observation_signatures.fetch_add(1, Ordering::SeqCst);
                            Ok(signature())
                        },
                        |_, _, _| {
                            checkpoint_signatures.fetch_add(1, Ordering::SeqCst);
                            Ok(signature())
                        },
                    ) {
                        Err(failure) => match *failure {
                            ResidentProcessProviderLaunchFailure::Selection {
                                dispatch,
                                reason:
                                    ResidentProcessHostSelectionError::ProviderCheckpointAuthority,
                            } => *dispatch,
                            _ => panic!("wrong checkpoint authority must preserve the dispatch"),
                        },
                        Ok(_) => panic!("wrong checkpoint authority must fail before I/O"),
                    };
                    assert_eq!(clock.next.load(Ordering::SeqCst), 0);
                    assert_eq!(observation_signatures.load(Ordering::SeqCst), 0);
                    assert_eq!(checkpoint_signatures.load(Ordering::SeqCst), 0);
                    assert!(executor.accepted.is_empty());

                    match case {
                        ProviderProcessCase::RefusedThenLost
                        | ProviderProcessCase::RefusedAggregateTransition
                        | ProviderProcessCase::DuplicateAggregateTransition => {
                            executor.refuse_next()
                        }
                        _ => {}
                    }
                    if matches!(
                        case,
                        ProviderProcessCase::ProtocolLoss
                            | ProviderProcessCase::ProtocolLossAggregateTransition
                    ) {
                        executor.substitute_next_key = Some(provider_task_key(
                            selected_store.clone(),
                            "process-provider/substituted-task",
                            'f',
                        ));
                    }
                    let launched = host.launch_provider(
                        &selected_selector,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions: &sessions,
                            anchor,
                        },
                        &mut authority,
                        dispatch,
                        &invocation,
                        &mut executor,
                        |_, _, _, _| Ok(()),
                        |_, _, _| {
                            observation_signatures.fetch_add(1, Ordering::SeqCst);
                            Ok(signature())
                        },
                        |_, _, _| {
                            checkpoint_signatures.fetch_add(1, Ordering::SeqCst);
                            Ok(signature())
                        },
                    );
                    let key = match case {
                        ProviderProcessCase::RefusedAggregateTransition
                        | ProviderProcessCase::DuplicateAggregateTransition => {
                            let failure = match launched {
                                Err(failure) => *failure,
                                Ok(_) => panic!("the executor must refuse the first offer"),
                            };
                            let ResidentProcessProviderLaunchFailure::Acceptance {
                                reservation,
                                failure:
                                    ResidentProviderTaskAcceptanceFailure::TaskOwner(
                                        ResidentProviderTaskRefusal::Executor {
                                            owner_instance,
                                            call,
                                        },
                                    ),
                            } = failure
                            else {
                                panic!("the real launch must retain its exact refusal")
                            };
                            let (returned_store, dispatch, _continuation, basis, request) =
                                match call.into_task_parts() {
                                    Ok(parts) => parts,
                                    Err(_) => panic!("retained launch has a durable task key"),
                                };
                            assert_eq!(returned_store, selected_store);
                            let call = Box::new(PreparedProviderOperationCall::from_task_parts(
                                basis, request,
                            ));
                            let original = if matches!(
                                case,
                                ProviderProcessCase::DuplicateAggregateTransition
                            ) {
                                ResidentProviderTaskRefusal::Duplicate(call)
                            } else {
                                ResidentProviderTaskRefusal::Executor {
                                    owner_instance,
                                    call,
                                }
                            };
                            let transition =
                                host.operations.refused_provider_offer_failure_for_test(
                                    &ResidentOperationTaskKey::new(
                                        selected_store.clone(),
                                        dispatch,
                                    ),
                                );
                            let failure = ResidentProcessProviderLaunchFailure::Acceptance {
                                reservation,
                                failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(
                                    ResidentProviderTaskRefusal::AggregateTransition {
                                        transition,
                                        outcome: Box::new(
                                            ResidentProviderTaskAggregateTransitionOutcome::Refused(
                                                original,
                                            ),
                                        ),
                                    },
                                ),
                            };
                            let calls = executor.accepted.len();
                            let failure = match host.continue_refused_provider_aggregate_transition(
                                failure,
                                ResidentDeploymentOperationAccess {
                                    store: &store("process-provider-other", "incarnation-1"),
                                    custody,
                                    sessions: &sessions,
                                    anchor,
                                },
                            ) {
                                Err(failure) => failure,
                                Ok(_) => panic!("foreign access keeps the exact refusal wrapper"),
                            };
                            assert_eq!(executor.accepted.len(), calls);
                            let original = match host
                                .continue_refused_provider_aggregate_transition(
                                    *failure,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &sessions,
                                        anchor,
                                    },
                                ) {
                                Ok(original) => original,
                                Err(_) => {
                                    panic!("the exact lower refusal transition must be repaired")
                                }
                            };
                            assert!(matches!(
                                (&original, case),
                                (
                                    ResidentProviderTaskRefusal::Executor { .. },
                                    ProviderProcessCase::RefusedAggregateTransition
                                ) | (
                                    ResidentProviderTaskRefusal::Duplicate(_),
                                    ProviderProcessCase::DuplicateAggregateTransition
                                )
                            ));
                            assert_eq!(executor.accepted.len(), calls);
                            assert_eq!(host.provider_routes.reserved, 0);
                            assert!(host
                                .provider_routes
                                .reserve(selected_selector.clone())
                                .is_ok());
                            return;
                        }
                        ProviderProcessCase::RefusedThenLost => {
                            let failure = match launched {
                                Err(failure) => *failure,
                                Ok(_) => panic!("the executor must refuse the first offer"),
                            };
                            assert!(matches!(
                                &failure,
                                ResidentProcessProviderLaunchFailure::Acceptance {
                                    failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(
                                        ResidentProviderTaskRefusal::Executor { .. }
                                    ),
                                    ..
                                }
                            ));
                            let observation_signatures_before_retry =
                                observation_signatures.load(Ordering::SeqCst);
                            let checkpoint_signatures_before_retry =
                                checkpoint_signatures.load(Ordering::SeqCst);
                            let key = match host.continue_refused_provider_acceptance(
                                failure,
                                ResidentDeploymentOperationAccess {
                                    store: &selected_store,
                                    custody,
                                    sessions: &sessions,
                                    anchor,
                                },
                                &mut authority,
                                &mut executor,
                            ) {
                                Ok(key) => key,
                                Err(_) => panic!("retry only the unopened executor offer"),
                            };
                            assert_eq!(
                                observation_signatures.load(Ordering::SeqCst),
                                observation_signatures_before_retry,
                                "acceptance retry must not re-sign an observation",
                            );
                            assert_eq!(
                                checkpoint_signatures.load(Ordering::SeqCst),
                                checkpoint_signatures_before_retry,
                                "acceptance retry must not re-sign a checkpoint",
                            );
                            key
                        }
                        ProviderProcessCase::Completed
                        | ProviderProcessCase::AcceptedAggregateTransition => match launched {
                            Ok(key) => key,
                            Err(_) => panic!("the executor must accept the retained dispatch"),
                        },
                        ProviderProcessCase::Stale => match launched {
                            Ok(key) => key,
                            Err(_) => panic!("the executor must accept the retained dispatch"),
                        },
                        ProviderProcessCase::ProtocolLoss => {
                            let failure = match launched {
                                Err(failure) => *failure,
                                Ok(_) => panic!("substituted executor return must be rejected"),
                            };
                            let ResidentProcessProviderLaunchFailure::AcceptedWithFailure {
                                selector: _,
                                key,
                                failure:
                                    ResidentProviderTaskAcceptanceFailure::TaskOwner(
                                        ResidentProviderTaskRefusal::ExecutorProtocolLoss {
                                            ..
                                        },
                                    ),
                            } = failure
                            else {
                                panic!("protocol loss must retain the accepted route")
                            };
                            assert_eq!(
                                host.provider_routes.selector(&key),
                                Some(&selected_selector)
                            );
                            assert!(host
                                .provider_routes
                                .reserve(selected_selector.clone())
                                .is_err());
                            return;
                        }
                        ProviderProcessCase::ProtocolLossAggregateTransition => {
                            let failure = match launched {
                                Err(failure) => *failure,
                                Ok(_) => panic!("substituted executor return must be rejected"),
                            };
                            let ResidentProcessProviderLaunchFailure::AcceptedWithFailure {
                                selector,
                                key,
                                failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
                            } = failure
                            else {
                                panic!("protocol loss must retain its accepted route")
                            };
                            assert!(matches!(
                                refusal,
                                ResidentProviderTaskRefusal::ExecutorProtocolLoss { .. }
                            ));
                            let transition = host
                                .operations
                                .accepted_provider_offer_failure_for_test(&key);
                            let failure =
                                ResidentProcessProviderLaunchFailure::AcceptedWithFailure {
                                    selector: selector.clone(),
                                    key: key.clone(),
                                    failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(
                                        ResidentProviderTaskRefusal::AggregateTransition {
                                            transition,
                                            outcome: Box::new(
                                                ResidentProviderTaskAggregateTransitionOutcome::Refused(
                                                    refusal,
                                                ),
                                            ),
                                        },
                                    ),
                                };
                            let signs = (
                                observation_signatures.load(Ordering::SeqCst),
                                checkpoint_signatures.load(Ordering::SeqCst),
                            );
                            let failure = host
                                .continue_accepted_provider_aggregate_transition(
                                    failure,
                                    ResidentDeploymentOperationAccess {
                                        store: &store("process-provider-other", "incarnation-1"),
                                        custody,
                                        sessions: &sessions,
                                        anchor,
                                    },
                                )
                                .expect_err("wrong deployment access keeps the exact wrapper");
                            let failure = host
                                .continue_accepted_provider_aggregate_transition(
                                    *failure,
                                    ResidentDeploymentOperationAccess {
                                        store: &selected_store,
                                        custody,
                                        sessions: &sessions,
                                        anchor,
                                    },
                                )
                                .expect_err("protocol loss remains after aggregate repair");
                            assert!(matches!(
                                *failure,
                                ResidentProcessProviderLaunchFailure::AcceptedWithFailure {
                                    selector: ref retained_selector,
                                    key: ref retained_key,
                                    failure:
                                        ResidentProviderTaskAcceptanceFailure::TaskOwner(
                                            ResidentProviderTaskRefusal::ExecutorProtocolLoss {
                                                ..
                                            }
                                        ),
                                } if retained_selector == &selector && retained_key == &key
                            ));
                            assert_eq!(
                                (
                                    observation_signatures.load(Ordering::SeqCst),
                                    checkpoint_signatures.load(Ordering::SeqCst),
                                ),
                                signs,
                            );
                            assert_eq!(host.provider_routes.selector(&key), Some(&selector));
                            return;
                        }
                    };
                    if matches!(case, ProviderProcessCase::AcceptedAggregateTransition) {
                        let transition = host
                            .operations
                            .accepted_provider_offer_failure_for_test(&key);
                        let failure = ResidentProcessProviderLaunchFailure::AcceptedWithFailure {
                            selector: selected_selector.clone(),
                            key: key.clone(),
                            failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(
                                ResidentProviderTaskRefusal::AggregateTransition {
                                    transition,
                                    outcome: Box::new(
                                        ResidentProviderTaskAggregateTransitionOutcome::Accepted {
                                            key: key.clone(),
                                        },
                                    ),
                                },
                            ),
                        };
                        let accepted_before = executor.accepted.len();
                        let signs = (
                            observation_signatures.load(Ordering::SeqCst),
                            checkpoint_signatures.load(Ordering::SeqCst),
                        );
                        let failure = host
                            .continue_accepted_provider_aggregate_transition(
                                failure,
                                ResidentDeploymentOperationAccess {
                                    store: &store("process-provider-other", "incarnation-1"),
                                    custody,
                                    sessions: &sessions,
                                    anchor,
                                },
                            )
                            .expect_err("wrong deployment access keeps the exact wrapper");
                        let accepted = match host.continue_accepted_provider_aggregate_transition(
                            *failure,
                            ResidentDeploymentOperationAccess {
                                store: &selected_store,
                                custody,
                                sessions: &sessions,
                                anchor,
                            },
                        ) {
                            Ok(accepted) => accepted,
                            Err(_) => panic!("the exact Offered entry must advance to Accepted"),
                        };
                        assert_eq!(accepted, key);
                        assert_eq!(executor.accepted.len(), accepted_before);
                        assert_eq!(
                            (
                                observation_signatures.load(Ordering::SeqCst),
                                checkpoint_signatures.load(Ordering::SeqCst),
                            ),
                            signs,
                        );
                    }
                    assert_eq!(key.store(), &selected_store);
                    match case {
                        ProviderProcessCase::RefusedThenLost => executor.lose_next(),
                        ProviderProcessCase::RefusedAggregateTransition
                        | ProviderProcessCase::DuplicateAggregateTransition => unreachable!(),
                        ProviderProcessCase::Completed
                        | ProviderProcessCase::AcceptedAggregateTransition => {
                            executor.complete_next_unavailable()
                        }
                        ProviderProcessCase::Stale => {
                            executor.complete_next_with_substituted_key(provider_task_key(
                                selected_store.clone(),
                                "process-provider/stale-task",
                                'e',
                            ))
                        }
                        ProviderProcessCase::ProtocolLoss
                        | ProviderProcessCase::ProtocolLossAggregateTransition => unreachable!(),
                    }
                    let progress = host
                        .poll_provider_task(&mut executor)
                        .expect("provider completion poll");
                    if matches!(case, ProviderProcessCase::Stale) {
                        assert!(matches!(
                            progress,
                            ResidentProcessProviderCompletionPoll::Stale(_)
                        ));
                        assert_eq!(
                            host.provider_routes.selector(&key),
                            Some(&selected_selector)
                        );
                        return;
                    }
                    let ResidentProcessProviderCompletionPoll::Routed(progress) = progress else {
                        panic!("accepted provider progress must remain deployment-routed");
                    };
                    let progress = Box::new(host.continue_provider_aggregate_transition(*progress));
                    let dummy_history = || OperationAttemptHistoryInput {
                        operation,
                        request,
                        session: parents.session.lease(),
                        lease: parents.lease,
                        generation,
                        dispatched: &initial,
                        selection: initial_record,
                        maximum_selection_bytes: 65_536,
                    };
                    let recovered = RecoveredOperationObservation::history_fixture_for_handle(
                        crate::resident::record_projection::coordinate(
                            handle_ref,
                            handle.canonical_operation_handle_sha256().as_str(),
                        )
                        .expect("provider handle recovery coordinate"),
                        (
                            initial_record.coordinate().record_ref().clone(),
                            initial.clone(),
                        ),
                        None,
                        None,
                        (
                            initial_record.coordinate().record_ref().clone(),
                            initial.clone(),
                        ),
                        (initial_checkpoint_ref.clone(), initial_checkpoint.clone()),
                        selected_store.clone(),
                    );
                    let recording_observation_signs = AtomicUsize::new(0);
                    let recording_checkpoint_signs = AtomicUsize::new(0);
                    let progress = match (case, *progress) {
                        (
                            ProviderProcessCase::Completed
                            | ProviderProcessCase::AcceptedAggregateTransition,
                            ResidentProcessProviderCompletion {
                                route: ResidentProcessProviderCompletionRoute::CheckedOut(route),
                                completion: ResidentProviderCompletionPoll::Completed(completed),
                            },
                        ) => {
                            let wrong_phase = ResidentProcessProviderCompletion {
                                route: ResidentProcessProviderCompletionRoute::CheckedOut(route),
                                completion: ResidentProviderCompletionPoll::Completed(completed),
                            };
                            let failure = match host.record_completed_provider(
                                wrong_phase,
                                ResidentDeploymentOperationAccess {
                                    store: &store("process-provider-other", "incarnation-1"),
                                    custody,
                                    sessions: &sessions,
                                    anchor,
                                },
                                &mut authority,
                                dummy_history(),
                                &recovered,
                                admission,
                                &invocation,
                                &UnusedProviderEvidence,
                                |_| Ok(()),
                                |_, _, _, _| Ok(()),
                                |_, _, _| {
                                    recording_observation_signs.fetch_add(1, Ordering::SeqCst);
                                    Ok(signature())
                                },
                                |_, _, _| {
                                    recording_checkpoint_signs.fetch_add(1, Ordering::SeqCst);
                                    Ok(signature())
                                },
                            ) {
                                Err(failure) => failure,
                                Ok(_) => {
                                    panic!("wrong store must retain completed provider progress")
                                }
                            };
                            let ResidentProcessProviderCompletionRecordingFailure::Selection {
                                route,
                                completed,
                                reason: ResidentProcessHostSelectionError::RecoveryStoreAssociation,
                            } = *failure
                            else {
                                panic!("wrong store must fail before provider recording I/O")
                            };
                            ResidentProcessProviderCompletion {
                                route: ResidentProcessProviderCompletionRoute::CheckedOut(route),
                                completion: ResidentProviderCompletionPoll::Completed(completed),
                            }
                        }
                        (
                            ProviderProcessCase::RefusedThenLost,
                            ResidentProcessProviderCompletion {
                                route: ResidentProcessProviderCompletionRoute::CheckedOut(route),
                                completion: ResidentProviderCompletionPoll::Lost(lost),
                            },
                        ) => {
                            let wrong_phase = ResidentProcessProviderCompletion {
                                route: ResidentProcessProviderCompletionRoute::CheckedOut(route),
                                completion: ResidentProviderCompletionPoll::Lost(lost),
                            };
                            let failure = match host.record_lost_provider(
                                wrong_phase,
                                ResidentDeploymentOperationAccess {
                                    store: &store("process-provider-other", "incarnation-1"),
                                    custody,
                                    sessions: &sessions,
                                    anchor,
                                },
                                &mut authority,
                                dummy_history(),
                                &recovered,
                                &invocation,
                                |_| Ok(()),
                                |_, _, _, _| Ok(()),
                                |_, _, _| {
                                    recording_observation_signs.fetch_add(1, Ordering::SeqCst);
                                    Ok(signature())
                                },
                                |_, _, _| {
                                    recording_checkpoint_signs.fetch_add(1, Ordering::SeqCst);
                                    Ok(signature())
                                },
                            ) {
                                Err(failure) => failure,
                                Ok(_) => panic!("wrong store must retain lost provider progress"),
                            };
                            let ResidentProcessProviderLossRecordingFailure::Selection {
                                route,
                                lost,
                                reason: ResidentProcessHostSelectionError::RecoveryStoreAssociation,
                            } = *failure
                            else {
                                panic!("wrong store must fail before provider loss I/O")
                            };
                            ResidentProcessProviderCompletion {
                                route: ResidentProcessProviderCompletionRoute::CheckedOut(route),
                                completion: ResidentProviderCompletionPoll::Lost(lost),
                            }
                        }
                        (_, progress) => progress,
                    };
                    assert_eq!(recording_observation_signs.load(Ordering::SeqCst), 0);
                    assert_eq!(recording_checkpoint_signs.load(Ordering::SeqCst), 0);
                    let ResidentProcessProviderCompletionRoute::CheckedOut(route) = progress.route
                    else {
                        panic!("lost work must carry checked-out route custody");
                    };
                    assert_eq!(route.selector, selected_selector);
                    match (case, progress.completion) {
                        (
                            ProviderProcessCase::RefusedThenLost,
                            ResidentProviderCompletionPoll::Lost(lost),
                        ) => {
                            assert_eq!(lost.key().store(), &selected_store);
                        }
                        (
                            ProviderProcessCase::Completed
                            | ProviderProcessCase::AcceptedAggregateTransition,
                            ResidentProviderCompletionPoll::Completed(completed),
                        ) => {
                            assert_eq!(completed.key().store(), &selected_store);
                        }
                        (ProviderProcessCase::Stale, _) => unreachable!(),
                        _ => panic!("provider progress must preserve its mechanical disposition"),
                    }
                },
            );
        },
    );
}

#[test]
fn provider_refusal_retry_and_loss_keep_the_selected_deployment() {
    provider_launch_and_poll_case(ProviderProcessCase::RefusedThenLost);
}

#[test]
fn refused_provider_aggregate_transition_repairs_before_route_release() {
    provider_launch_and_poll_case(ProviderProcessCase::RefusedAggregateTransition);
}

#[test]
fn duplicate_provider_aggregate_transition_repairs_before_route_release() {
    provider_launch_and_poll_case(ProviderProcessCase::DuplicateAggregateTransition);
}

#[test]
fn completed_provider_work_keeps_the_selected_deployment() {
    provider_launch_and_poll_case(ProviderProcessCase::Completed);
}

#[test]
fn rejected_provider_evidence_retries_owned_recording_without_repoll_or_replay() {
    provider_evidence_uncertainty_after_owned_recording_retry(
        ProviderEvidenceProofCase::RejectedSignature,
    );
}

#[test]
fn mismatched_provider_evidence_retries_owned_recording_without_repoll_or_replay() {
    provider_evidence_uncertainty_after_owned_recording_retry(
        ProviderEvidenceProofCase::SubstitutedResultSchema,
    );
}

#[test]
fn accepted_provider_aggregate_transition_is_repaired_without_replaying_work() {
    provider_launch_and_poll_case(ProviderProcessCase::AcceptedAggregateTransition);
}

#[test]
fn provider_protocol_loss_retains_the_non_replayable_route() {
    provider_launch_and_poll_case(ProviderProcessCase::ProtocolLoss);
}

#[test]
fn protocol_loss_aggregate_transition_is_repaired_without_replaying_work() {
    provider_launch_and_poll_case(ProviderProcessCase::ProtocolLossAggregateTransition);
}

#[test]
fn stale_provider_completion_does_not_consume_the_active_route() {
    provider_launch_and_poll_case(ProviderProcessCase::Stale);
}
