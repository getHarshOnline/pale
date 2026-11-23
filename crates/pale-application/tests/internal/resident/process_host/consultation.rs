//! Process-host laws for one bounded Consult supervision turn.

use std::collections::VecDeque;

use pale_core::canon;
use pale_core::records::{
    ArtifactDigest, CapabilityAttemptIdV1, ControlCheckpointIdV1, ControlStateCheckpointPreimageV1,
    ImmutableRecordReferenceV1, OperationActivePhaseV1, OperationObservationAdmissionContextV1,
    OperationObservationDigest, OperationObservationPreimageV1, OperationObservationRevisionV1,
    OperationObservationStateV1, OperationObservationV1, SignedResidentAdmissionContextV1,
};
use pale_ports::{
    AccumulatorPreparationLimits, AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition,
    AntiRollbackAnchorError, AntiRollbackAnchorErrorKind, AntiRollbackAnchorKey,
    AntiRollbackAnchorObservation, AntiRollbackAnchorPort, ControlRecordDigest,
    ControlStoreErrorKind, OperationCustodyPort, PreparedAntiRollbackAnchorTransition,
    ResidentOperationFrameCancellationDisposition, ResidentOperationFrameFailure,
    ResidentOperationFrameResponse, WallClockSourceError, WallClockSourcePort,
};

use super::*;
use crate::resident::consultation::service::task::ResidentConsultationTaskPoll;
use crate::resident::consultation::tests::{
    consultation_supervision_fixture_for_operation_in_store,
    consultation_supervision_fixture_in_store,
    consultation_supervision_fixture_with_child_in_store, prepare_collection_accumulator,
    CollectionCommitPort,
};
use crate::resident::consultation::{
    ConsultationChildRetentionOutcome, ConsultationCollectionLimits, ConsultationStopCause,
    ConsultationStopInput, ConsultationSuccessorCommitInput,
    ResidentConsultationCancellationNotification, ResidentConsultationSupervisionDecision,
    ResidentConsultationTurnDisposition, ResidentConsultationTurnNeedsRecovery,
    ResidentConsultationTurnProgress,
};
use crate::resident::operation_observation_custody::tests::{
    admit_initial_checkpoint, signing_authority, OperationFixture,
};
use crate::resident::operation_service::ResidentConsultationNotCollecting;
use crate::resident::process_host::operation_request_host::{
    ResidentOperationRequestHost, ResidentOperationRequestKind, ResidentOperationRequestOutcome,
    ResidentOperationRequestResolution,
};
use crate::resident::task_owner::{
    ResidentOperationTask, ResidentOperationTaskCompletion, ResidentOperationTaskExecutor,
};

#[derive(Default)]
struct CountingConsultationExecutor {
    accepts: usize,
    polls: usize,
    accepted: Vec<ResidentOperationTask<u8>>,
    completions: VecDeque<ResidentOperationTaskCompletion<u8, u16>>,
}

impl CountingConsultationExecutor {
    fn complete_first(&mut self, output: u16) {
        let task = self.accepted.remove(0);
        self.completions
            .push_back(ResidentOperationTaskCompletion::Completed { task, output });
    }

    fn lose_first(&mut self) {
        let task = self.accepted.remove(0);
        self.completions.push_back(task.into_lost());
    }
}

impl ResidentOperationTaskExecutor<u8, u16> for CountingConsultationExecutor {
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<u8>,
    ) -> Result<(), ResidentOperationTask<u8>> {
        self.accepts += 1;
        self.accepted.push(task);
        Ok(())
    }

    fn try_completion(&mut self) -> Option<ResidentOperationTaskCompletion<u8, u16>> {
        self.polls += 1;
        self.completions.pop_front()
    }
}

#[derive(Clone, Copy)]
struct FixedConsultationClock(UnixTimeV1);

impl WallClockSourcePort for FixedConsultationClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Ok(self.0)
    }
}

#[derive(Default)]
struct ConsultationAnchorReborrowProbe {
    reads: usize,
}

impl AntiRollbackAnchorPort for ConsultationAnchorReborrowProbe {
    fn read(
        &mut self,
        _: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.reads += 1;
        Ok(None)
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::Unavailable,
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

fn recovered_consultation_child(
    operation: &OperationFixture,
    store: AuthorityStoreCoordinate,
    dispatched: bool,
) -> RecoveredOperationObservation {
    let (handle_ref, handle) = operation.handle();
    let selected = if dispatched {
        let reference = local_operation_start_fixture::record_ref("operation/process-consult");
        let observation = local_operation_start_fixture::issue_observation(
            &signing_authority(),
            handle_ref,
            handle,
            local_operation_start_fixture::schema_digest(
                pale_core::records::OPERATION_OBSERVATION_V1_SCHEMA_ID,
            ),
            OperationObservationRevisionV1::new(3).expect("dispatch revision"),
            Some(operation.successor()),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched {
                    attempt_id: CapabilityAttemptIdV1::parse("process-consult-attempt")
                        .expect("attempt identity"),
                    evidence: ImmutableRecordReferenceV1 {
                        record_ref: local_operation_start_fixture::record_ref(
                            "attempt/process-consult",
                        ),
                        record_sha256: ArtifactDigest::hash_bytes(b"process-consult-attempt"),
                    },
                },
            },
            32,
        );
        (reference, observation)
    } else {
        (
            operation.successor().0.clone(),
            operation.successor().1.clone(),
        )
    };
    let handle = pale_ports::ControlRecordCoordinate::new(
        handle_ref.clone(),
        ControlRecordDigest::parse(handle.canonical_operation_handle_sha256().as_str())
            .expect("fixture handle coordinate"),
    );
    RecoveredOperationObservation::fixture_for_handle(
        handle,
        selected.clone(),
        selected,
        (
            local_operation_start_fixture::record_ref("checkpoint/process-consult"),
            admit_initial_checkpoint(&local_operation_start_fixture::checkpoint_preimage(1, None)),
        ),
        store,
    )
}

fn operation_handle_coordinate(operation: &OperationFixture) -> ImmutableRecordReferenceV1 {
    let (reference, handle) = operation.handle();
    ImmutableRecordReferenceV1 {
        record_ref: reference.clone(),
        record_sha256: ArtifactDigest::parse(handle.canonical_operation_handle_sha256().as_str())
            .expect("fixture handle digest"),
    }
}

fn task_state_with_consultation_capacity(
    maximum_active_batches: usize,
    maximum_outstanding: usize,
) -> ResidentOperationTaskState {
    ResidentOperationTaskState::new(
        ResidentOperationTaskLimits::new(
            NonZeroUsize::new(1).expect("positive provider-task bound"),
        ),
        NonZeroUsize::new(maximum_active_batches).expect("positive consultation-batch bound"),
        ResidentAggregateTaskLimits::new(
            NonZeroUsize::new(maximum_outstanding).expect("positive aggregate-task bound"),
        ),
    )
    .expect("bounded operation task state")
}

fn run_nonmutating_process_consult_cancel(
    fixture: ProcessHostCancellationFixture<'_>,
) -> Result<ResidentOperationCancellationOutcome, ResidentOperationCancellationFailure> {
    let ProcessHostCancellationFixture {
        input,
        admission,
        signing_authority,
        operation_handle: _,
        custody,
        sessions,
        anchor,
    } = fixture;
    let selected_store = input.recovery.operation.context.store.clone();
    let selector = selector_from_signing_authority(signing_authority);
    let deployment_port = MemoryAuthorityStore::default();
    let deployment = unproved_deployment_for_selector(
        &selector,
        selected_store.clone(),
        ready_application(&deployment_port, selected_store.clone(), 1),
    );
    let mut host = ResidentProcessHost::new(
        unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
        vec![deployment],
        NonZeroUsize::new(1).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one exact Consult deployment");
    let mut authority = ProvingAuthority;
    let outcome = host
        .request_consultation_cancellation(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &mut authority,
            input,
            admission,
            ConsultationCollectionLimits::new(1, 16_384, 4096).expect("bounded collection"),
            |_, _| panic!("non-Consult state cannot admit a consultation stop"),
            |_| panic!("non-Consult state cannot admit a consultation collection"),
        )
        .unwrap_or_else(|failure| match failure {
            ResidentProcessConsultationCancellationError::Selection(reason) => {
                panic!("exact disposition selection failed: {reason:?}")
            }
            ResidentProcessConsultationCancellationError::Cancellation(reason) => {
                panic!("exact disposition failed: {reason:?}")
            }
            ResidentProcessConsultationCancellationError::Custody(_) => {
                panic!("non-Consult disposition cannot create custody")
            }
            ResidentProcessConsultationCancellationError::Preparation(_) => {
                panic!("non-Consult disposition cannot prepare Consult custody")
            }
        });
    match outcome {
        ResidentConsultationCancellationDisposition::Established(existing) => {
            Ok(ResidentOperationCancellationOutcome::Established(existing))
        }
        ResidentConsultationCancellationDisposition::AlreadyRequested(request) => Ok(
            ResidentOperationCancellationOutcome::AlreadyRequested(request),
        ),
        ResidentConsultationCancellationDisposition::ConsultationCollected(evidence) => Ok(
            ResidentOperationCancellationOutcome::ConsultationCollected(evidence),
        ),
        ResidentConsultationCancellationDisposition::ReconciliationOnly(existing) => Ok(
            ResidentOperationCancellationOutcome::ReconciliationOnly(existing),
        ),
        ResidentConsultationCancellationDisposition::NotCollecting(
            ResidentConsultationNotCollecting::Dispatched(existing)
            | ResidentConsultationNotCollecting::BeforeDispatch(existing),
        ) => Ok(ResidentOperationCancellationOutcome::DispatchOwnerRequired(
            existing,
        )),
        ResidentConsultationCancellationDisposition::Retained(_) => {
            panic!("non-Consult disposition cannot append")
        }
    }
}

#[test]
fn process_consult_cancel_preserves_closed_and_non_consult_states_without_append() {
    with_process_host_consultation_cancellation_dispositions(
        &mut run_nonmutating_process_consult_cancel,
    );
}

#[test]
fn process_consult_cancel_reports_before_dispatch_without_using_another_owner() {
    with_process_host_before_dispatch_consultation_cancellation_fixture(
        &mut run_nonmutating_process_consult_cancel,
    );
}

#[test]
fn exact_process_consult_cancel_reaches_custody_before_marking_local_work() {
    with_process_host_collecting_cancellation_fixture(
        &mut |fixture: ProcessHostCancellationFixture<'_>| {
            let ProcessHostCancellationFixture {
                input,
                admission,
                signing_authority,
                operation_handle,
                custody,
                sessions,
                anchor,
            } = fixture;
            let selected_store = input.recovery.operation.context.store.clone();
            let selector = selector_from_signing_authority(signing_authority);
            let deployment_port = MemoryAuthorityStore::default();
            let deployment = unproved_deployment_for_selector(
                &selector,
                selected_store.clone(),
                ready_application(&deployment_port, selected_store.clone(), 1),
            );
            let mut host = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                vec![deployment],
                NonZeroUsize::new(1).expect("positive deployment bound"),
                ResidentOperationHost::new(task_state_with_consultation_capacity(1, 1)),
            )
            .expect("one exact Consult deployment");
            let child_operation = OperationFixture::exact();
            let current = crate::resident::consultation::tests::
                consultation_supervision_fixture_for_operation_in_store(
                &operation_handle.0,
                &operation_handle.1,
                "cancel-child",
                operation_handle_coordinate(&child_operation),
                false,
                selected_store.clone(),
            );
            let dispatch_time =
                UnixTimeV1::new(operation_handle.1.created_at().get() + 1).expect("dispatch time");
            let batch = current
                .plan_child_dispatch(dispatch_time)
                .expect("one exact child batch");
            let candidate = batch.candidates()[0].clone();
            let initial_child =
                recovered_consultation_child(&child_operation, selected_store.clone(), false);
            let retained_child =
                recovered_consultation_child(&child_operation, selected_store.clone(), true);
            let mut executor = CountingConsultationExecutor::default();
            let mut authority = ProvingAuthority;
            let coordinate = with_unproved_operation_service_for_test(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                |_application, service| {
                    let coordinate = service
                        .open_consultation_batch(batch, consultation_limits())
                        .expect("exact process-local batch");
                    service
                        .retain_and_accept_consultation_task(
                            &coordinate,
                            &candidate,
                            &FixedConsultationClock(dispatch_time),
                            || Ok::<_, std::convert::Infallible>(current.clone()),
                            |_| Ok::<_, std::convert::Infallible>(initial_child),
                            |_, _| ConsultationChildRetentionOutcome::<
                                u8,
                                std::convert::Infallible,
                            >::Retained {
                                child: Box::new(retained_child),
                                work: 7,
                            },
                            &mut executor,
                        )
                        .expect("child reaches process-local execution once");
                    coordinate
                },
            )
            .expect("exact deployment selection");
            let collection_admissions = std::cell::Cell::new(0);
            let locator = ResidentOperationFrameLocator::new(
                operation_handle.0.as_str().to_owned(),
                operation_handle
                    .1
                    .canonical_operation_handle_sha256()
                    .as_str()
                    .to_owned(),
            );
            let request_frame = ResidentOperationFrameRequest::Cancel {
                locator: locator.clone(),
                signed_common_request: pale_core::canon::to_canonical_string(input.request)
                    .expect("signed cancellation request is canonical"),
            };
            let mut frame_port = OperationFramePort::new(&request_frame);
            let deadline = std::time::Instant::now()
                .checked_add(std::time::Duration::from_secs(2))
                .expect("test deadline");
            let mut request_host = ResidentOperationRequestHost::new(&mut frame_port, deadline);
            let received = request_host
                .receive()
                .expect("Consult cancellation frame is exact");
            let retained_outcome = std::cell::RefCell::new(None);
            let response = operation_request_host::request_host_test_support::answer(
                &mut request_host,
                received,
                &mut host,
                ResidentOperationRequestResolution::retained(
                    ResidentOperationRequestKind::Cancel,
                    input.request,
                    locator.clone(),
                ),
                &mut |_, actual| {
                    assert_eq!(actual, &request_frame);
                    Ok(())
                },
                deadline,
                |host, admitted_request| {
                    assert!(std::ptr::eq(admitted_request, input.request));
                    let mut input = input;
                    input.request = admitted_request;
                    let outcome = host
                        .request_consultation_cancellation(
                            &selector,
                            ResidentDeploymentOperationAccess {
                                store: &selected_store,
                                custody: &mut *custody,
                                sessions,
                                anchor: &mut *anchor,
                            },
                            &mut authority,
                            input,
                            admission,
                            ConsultationCollectionLimits::new(1, 16_384, 4096)
                                .expect("bounded collection"),
                            |collection, summary| {
                                assert_eq!(collection.parent(), coordinate.parent());
                                assert_eq!(summary.required_outstanding().len(), 1);
                                Ok(())
                            },
                            |collection| {
                                collection_admissions.set(collection_admissions.get() + 1);
                                if collection_admissions.get() == 2 {
                                    assert!(collection.stopped().is_some());
                                }
                                Ok(())
                            },
                        )
                        .map_err(|_| {
                            pale_ports::ResidentOperationFrameFailure::ResidentUnavailable
                        })?;
                    if !matches!(
                        outcome,
                        ResidentConsultationCancellationDisposition::Retained(_)
                    ) {
                        return Err(pale_ports::ResidentOperationFrameFailure::ResidentUnavailable);
                    }
                    retained_outcome.replace(Some(outcome));
                    Ok(ResidentOperationRequestOutcome::CancellationDisposition {
                        locator: locator.clone(),
                        disposition:
                            ResidentOperationFrameCancellationDisposition::ConsultationStopped,
                    })
                },
            )
            .expect("private request host returns the real cancellation disposition");
            assert_eq!(
                response,
                ResidentOperationFrameResponse::CancellationDisposition {
                    locator,
                    disposition: ResidentOperationFrameCancellationDisposition::ConsultationStopped,
                }
            );
            let outcome = retained_outcome
                .into_inner()
                .expect("the real Consult cancellation outcome was retained");
            assert_eq!(collection_admissions.get(), 2);
            let ResidentConsultationCancellationDisposition::Retained(outcome) = outcome else {
                panic!("collecting consultation must retain its stopped roster")
            };
            assert_eq!(
                outcome.notification,
                ResidentConsultationCancellationNotification::BatchMarked {
                    newly_marked: 1,
                    already_marked: 0,
                    no_longer_accepted: 0,
                }
            );

            executor.lose_first();
            let late = with_unproved_operation_service_for_test(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                |_application, service| {
                    service
                        .try_consultation_completion(&mut executor)
                        .expect("late loss remains attributable")
                        .expect("late loss keeps its exact batch owner")
                },
            )
            .expect("late evidence uses the same deployment owner");
            assert_eq!(late.coordinate, coordinate);
            assert!(matches!(
                late.outcome,
                ResidentConsultationTaskPoll::Lost(lost)
                    if lost.cancellation_state()
                        == ResidentOperationTaskCancellationState::Notified
            ));

            Ok(ResidentOperationCancellationOutcome::LocalRequestRetained {
                custody: outcome.custody,
                checkpoint: outcome.checkpoint,
            })
        },
    );
}

#[test]
fn process_consult_cancel_retains_signed_preparation_before_returning_unavailable() {
    with_process_host_collecting_cancellation_preparation_failure_fixture(
        &mut |fixture: ProcessHostCancellationFixture<'_>| {
            let ProcessHostCancellationFixture {
                input,
                admission,
                signing_authority,
                operation_handle: _,
                custody,
                sessions,
                anchor,
            } = fixture;
            let selected_store = input.recovery.operation.context.store.clone();
            let selector = selector_from_signing_authority(signing_authority);
            let deployment_port = MemoryAuthorityStore::default();
            let deployment = unproved_deployment_for_selector(
                &selector,
                selected_store.clone(),
                ready_application(&deployment_port, selected_store.clone(), 1),
            );
            let mut host = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                vec![deployment],
                NonZeroUsize::new(1).expect("positive deployment bound"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one exact Consult deployment");
            let mut authority = ProvingAuthority;

            let pending = match host.request_consultation_cancellation(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                input,
                admission,
                ConsultationCollectionLimits::new(1, 16_384, 4096).expect("bounded collection"),
                |_, _| Ok(()),
                |_| Ok(()),
            ) {
                Err(ResidentProcessConsultationCancellationError::Preparation(pending)) => pending,
                _ => panic!("preparation refusal must return its signed Consult cancellation"),
            };
            let mut retention = ProgramCancellationRetentionProbe::default();
            let failure = match crate::resident::process_host::program::settle_cancellation_handoff(
                &mut retention,
                (),
                crate::resident::process_host::program::ResidentProgramCancellationHandoff::ConsultationPreparation(
                    pending,
                ),
            ) {
                Ok(_) => panic!("retained preparation is not a completed Cancel response"),
                Err(failure) => failure,
            };
            assert_eq!(failure, ResidentOperationFrameFailure::ResidentUnavailable);
            assert_eq!(retention.retention_calls, 1);
            assert_eq!(retention.release_calls, 0);

            let pending = retention
                .consultation_preparation
                .take()
                .expect("the exact signed preparation remains retained");
            match host.continue_consultation_cancellation_preparation(
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                pending,
            ) {
                Err(ResidentProcessConsultationCancellationPreparationContinuationFailure {
                    reason: ResidentProcessConsultationCancellationContinuationError::Custody,
                    ..
                }) => {}
                _ => panic!("a pre-write refusal must remain the same deferred phase"),
            }

            Err(ResidentOperationCancellationFailure::ConsultationRouteRequired)
        },
    );
}

#[test]
fn process_consult_cancel_retries_only_its_retained_database_candidate() {
    with_process_host_collecting_cancellation_database_retry_fixture(
        &mut |fixture: ProcessHostCancellationFixture<'_>| {
            let ProcessHostCancellationFixture {
                input,
                admission,
                signing_authority,
                operation_handle,
                custody,
                sessions,
                anchor,
            } = fixture;
            let selected_store = input.recovery.operation.context.store.clone();
            let selector = selector_from_signing_authority(signing_authority);
            let deployment_port = MemoryAuthorityStore::default();
            let deployment = unproved_deployment_for_selector(
                &selector,
                selected_store.clone(),
                ready_application(&deployment_port, selected_store.clone(), 1),
            );
            let mut host = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                vec![deployment],
                NonZeroUsize::new(1).expect("positive deployment bound"),
                ResidentOperationHost::new(task_state_with_consultation_capacity(1, 1)),
            )
            .expect("one exact Consult deployment");
            let child_operation = OperationFixture::exact();
            let current = crate::resident::consultation::tests::
                consultation_supervision_fixture_for_operation_in_store(
                    &operation_handle.0,
                    &operation_handle.1,
                    "cancel-database-retry-child",
                    operation_handle_coordinate(&child_operation),
                    false,
                    selected_store.clone(),
                );
            let dispatch_time =
                UnixTimeV1::new(operation_handle.1.created_at().get() + 1).expect("dispatch time");
            let batch = current
                .plan_child_dispatch(dispatch_time)
                .expect("one exact child batch");
            let candidate = batch.candidates()[0].clone();
            let initial_child =
                recovered_consultation_child(&child_operation, selected_store.clone(), false);
            let retained_child =
                recovered_consultation_child(&child_operation, selected_store.clone(), true);
            let mut executor = CountingConsultationExecutor::default();
            let mut authority = ProvingAuthority;
            let coordinate = with_unproved_operation_service_for_test(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                |_application, service| {
                    let coordinate = service
                        .open_consultation_batch(batch, consultation_limits())
                        .expect("exact process-local batch");
                    service
                        .retain_and_accept_consultation_task(
                            &coordinate,
                            &candidate,
                            &FixedConsultationClock(dispatch_time),
                            || Ok::<_, std::convert::Infallible>(current.clone()),
                            |_| Ok::<_, std::convert::Infallible>(initial_child),
                            |_, _| ConsultationChildRetentionOutcome::<
                                u8,
                                std::convert::Infallible,
                            >::Retained {
                                child: Box::new(retained_child),
                                work: 7,
                            },
                            &mut executor,
                        )
                        .expect("child reaches process-local execution once");
                    coordinate
                },
            )
            .expect("exact deployment selection");

            let pending = match host.request_consultation_cancellation(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                input,
                admission,
                ConsultationCollectionLimits::new(1, 16_384, 4096).expect("bounded collection"),
                |_, _| Ok(()),
                |_| Ok(()),
            ) {
                Err(ResidentProcessConsultationCancellationError::Custody(pending)) => pending,
                _ => panic!("database uncertainty must retain process custody"),
            };
            let mut retention = ProgramCancellationRetentionProbe::default();
            assert!(crate::resident::process_host::program::settle_cancellation_handoff(
                &mut retention,
                (),
                crate::resident::process_host::program::ResidentProgramCancellationHandoff::ConsultationCustody(pending),
            )
            .is_err());
            assert_eq!(retention.retention_calls, 1);
            let pending = retention
                .consultation
                .take()
                .expect("the exact Consult custody is retained once");

            executor.lose_first();
            let before_custody = with_unproved_operation_service_for_test(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                |_application, service| {
                    service
                        .try_consultation_completion(&mut executor)
                        .expect("completion poll remains bounded")
                        .expect("late completion remains attributable")
                },
            )
            .expect("exact deployment selection");
            assert_eq!(before_custody.coordinate, coordinate);
            assert!(matches!(
                before_custody.outcome,
                ResidentConsultationTaskPoll::Lost(lost)
                    if lost.cancellation_state()
                        == ResidentOperationTaskCancellationState::Unnotified
            ));

            let foreign_store = AuthorityStoreCoordinate::new(
                ControlStoreIdV1::parse("foreign-consult-custody").unwrap(),
                selected_store.incarnation_id().clone(),
            );
            let pending = match host.continue_consultation_cancellation_custody(
                ResidentDeploymentOperationAccess {
                    store: &foreign_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                pending,
            ) {
                Err(ResidentProcessConsultationCancellationContinuationFailure {
                    pending,
                    reason:
                        ResidentProcessConsultationCancellationContinuationError::Selection(
                            ResidentProcessHostSelectionError::AccessStoreAssociation,
                        ),
                }) => pending,
                _ => panic!("foreign access must return the exact process custody wrapper"),
            };
            let pending = match host.continue_consultation_cancellation_custody(
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                pending,
            ) {
                Err(ResidentProcessConsultationCancellationContinuationFailure {
                    pending,
                    reason: ResidentProcessConsultationCancellationContinuationError::Custody,
                }) => pending,
                _ => panic!("first exact read-back remains uncertain"),
            };
            let outcome = match host.continue_consultation_cancellation_custody(
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                pending,
            ) {
                Ok(outcome) => outcome,
                Err(_) => panic!("second exact read-back must prove the retained candidate"),
            };
            assert_eq!(
                outcome.notification,
                ResidentConsultationCancellationNotification::NoLocalOwner
            );
            Ok(ResidentOperationCancellationOutcome::LocalRequestRetained {
                custody: outcome.custody,
                checkpoint: outcome.checkpoint,
            })
        },
    );
}

#[test]
fn rebuilt_process_host_continues_anchor_custody_without_claiming_an_old_local_batch() {
    with_process_host_collecting_cancellation_anchor_retry_fixture(
        &mut |fixture: ProcessHostCancellationFixture<'_>| {
            let ProcessHostCancellationFixture {
                input,
                admission,
                signing_authority,
                operation_handle,
                custody,
                sessions,
                anchor,
            } = fixture;
            let selected_store = input.recovery.operation.context.store.clone();
            let selector = selector_from_signing_authority(signing_authority);
            let deployment_port = MemoryAuthorityStore::default();
            let deployment = unproved_deployment_for_selector(
                &selector,
                selected_store.clone(),
                ready_application(&deployment_port, selected_store.clone(), 1),
            );
            let mut original_host = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                vec![deployment],
                NonZeroUsize::new(1).expect("positive deployment bound"),
                ResidentOperationHost::new(task_state_with_consultation_capacity(1, 1)),
            )
            .expect("one exact Consult deployment");
            let child_operation = OperationFixture::exact();
            let current = crate::resident::consultation::tests::
                consultation_supervision_fixture_for_operation_in_store(
                    &operation_handle.0,
                    &operation_handle.1,
                    "cancel-anchor-retry-child",
                    operation_handle_coordinate(&child_operation),
                    false,
                    selected_store.clone(),
                );
            let dispatch_time =
                UnixTimeV1::new(operation_handle.1.created_at().get() + 1).expect("dispatch time");
            let batch = current
                .plan_child_dispatch(dispatch_time)
                .expect("one exact child batch");
            let candidate = batch.candidates()[0].clone();
            let initial_child =
                recovered_consultation_child(&child_operation, selected_store.clone(), false);
            let retained_child =
                recovered_consultation_child(&child_operation, selected_store.clone(), true);
            let mut executor = CountingConsultationExecutor::default();
            let mut authority = ProvingAuthority;
            let coordinate = with_unproved_operation_service_for_test(
                &mut original_host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                |_application, service| {
                    let coordinate = service
                        .open_consultation_batch(batch, consultation_limits())
                        .expect("exact process-local batch");
                    service
                        .retain_and_accept_consultation_task(
                            &coordinate,
                            &candidate,
                            &FixedConsultationClock(dispatch_time),
                            || Ok::<_, std::convert::Infallible>(current.clone()),
                            |_| Ok::<_, std::convert::Infallible>(initial_child),
                            |_, _| ConsultationChildRetentionOutcome::<
                                u8,
                                std::convert::Infallible,
                            >::Retained {
                                child: Box::new(retained_child),
                                work: 7,
                            },
                            &mut executor,
                        )
                        .expect("child reaches process-local execution once");
                    coordinate
                },
            )
            .expect("exact deployment selection");
            let pending = match original_host.request_consultation_cancellation(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                input,
                admission,
                ConsultationCollectionLimits::new(1, 16_384, 4096).expect("bounded collection"),
                |_, _| Ok(()),
                |_| Ok(()),
            ) {
                Err(ResidentProcessConsultationCancellationError::Custody(pending)) => pending,
                _ => panic!("anchor unavailability must retain process custody"),
            };

            let rebuilt_port = MemoryAuthorityStore::default();
            let rebuilt_deployment = unproved_deployment_for_selector(
                &selector,
                selected_store.clone(),
                ready_application(&rebuilt_port, selected_store.clone(), 2),
            );
            let mut rebuilt_host = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                vec![rebuilt_deployment],
                NonZeroUsize::new(1).expect("positive deployment bound"),
                ResidentOperationHost::new(task_state_with_consultation_capacity(1, 1)),
            )
            .expect("rebuilt host selects the same deployment authority");
            let outcome = match rebuilt_host.continue_consultation_cancellation_custody(
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                pending,
            ) {
                Ok(outcome) => outcome,
                Err(_) => panic!("exact retained anchor candidate must complete once"),
            };
            assert_eq!(
                outcome.notification,
                ResidentConsultationCancellationNotification::NoLocalOwner
            );

            executor.lose_first();
            let untouched = with_unproved_operation_service_for_test(
                &mut original_host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &mut authority,
                |_application, service| {
                    service
                        .try_consultation_completion(&mut executor)
                        .expect("completion poll remains bounded")
                        .expect("late completion remains attributable")
                },
            )
            .expect("original deployment remains selectable");
            assert_eq!(untouched.coordinate, coordinate);
            assert!(matches!(
                untouched.outcome,
                ResidentConsultationTaskPoll::Lost(lost)
                    if lost.cancellation_state()
                        == ResidentOperationTaskCancellationState::Unnotified
            ));
            Ok(ResidentOperationCancellationOutcome::LocalRequestRetained {
                custody: outcome.custody,
                checkpoint: outcome.checkpoint,
            })
        },
    );
}

#[test]
fn exact_process_consult_turn_recovers_then_polls_decides_and_offers_once() {
    let deployment_port = MemoryAuthorityStore::default();
    let principal_id = principal("principal-consult");
    let selected_store = store("consult-store", "consult-incarnation");
    let selector = unproved_selector(principal_id.clone(), selected_store.clone());
    let deployment = unproved_deployment_for_selector(
        &selector,
        selected_store.clone(),
        ready_application(&deployment_port, selected_store.clone(), 1),
    );
    let mut host = ResidentProcessHost::new(
        unproved_writer(principal_id.clone(), vec![selected_store.clone()]),
        vec![deployment],
        NonZeroUsize::new(1).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state_with_consultation_capacity(1, 1)),
    )
    .expect("one exact Consult deployment");
    let integrated = integrated_signing_authority(&selector);
    let checkpoint = local_operation_start_fixture::checkpoint_preimage(1, None);
    let base_checkpoint_authority =
        local_operation_start_fixture::checkpoint_authority(&checkpoint);
    let checkpoint_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
        principal_id: &principal_id,
        control_store_id: selected_store.store_id(),
        control_incarnation_id: selected_store.incarnation_id(),
        ..base_checkpoint_authority
    };
    let operation = OperationFixture::exact();
    let current = consultation_supervision_fixture_with_child_in_store(
        "operation/process-consult-parent",
        "process-consult-child",
        operation_handle_coordinate(&operation),
        100,
        false,
        selected_store.clone(),
    );
    let candidate = current
        .plan_child_dispatch(UnixTimeV1::new(2).expect("turn time"))
        .expect("one exact candidate")
        .candidates()[0]
        .clone();
    let recovered_child = recovered_consultation_child(&operation, selected_store.clone(), false);
    let retained_child = recovered_consultation_child(&operation, selected_store.clone(), true);
    let recoveries = std::cell::Cell::new(0);
    let decisions = std::cell::Cell::new(0);
    let child_reads = std::cell::Cell::new(0);
    let retentions = std::cell::Cell::new(0);
    let mut executor = CountingConsultationExecutor::default();
    let (mut authority, mut custody, _) = operation_ports();
    let mut anchor = ConsultationAnchorReborrowProbe::default();
    let custody_identity = std::ptr::from_mut(&mut custody).cast::<()>();
    let anchor_identity = std::ptr::from_mut(&mut anchor).cast::<()>();
    let custody_probe = pale_ports::ControlRecordCoordinate::new(
        local_operation_start_fixture::record_ref("operation/process-consult-probe"),
        ControlRecordDigest::parse(
            "abababababababababababababababababababababababababababababababab",
        )
        .expect("probe digest"),
    );

    let disposition = host
        .supervise_consultation_turn(
            &selector,
            ResidentOperationControlAssociation {
                current_request_authority: &integrated,
                historical_handle_authority: &integrated,
                recovery_store: &selected_store,
                checkpoint_authority: &checkpoint_authority,
            },
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut custody,
                sessions: &deployment_port,
                anchor: &mut anchor,
            },
            &mut authority,
            consultation_limits(),
            UnixTimeV1::new(2).expect("turn time"),
            |_application, _service, _sessions| {
                recoveries.set(recoveries.get() + 1);
                Ok::<_, &'static str>(current)
            },
            &mut executor,
            |_, _| {
                decisions.set(decisions.get() + 1);
                ResidentConsultationSupervisionDecision::<()>::Continue(Some(Box::new(candidate)))
            },
            |_| {
                child_reads.set(child_reads.get() + 1);
                Ok::<_, &'static str>(recovered_child)
            },
            |_, _, custody, anchor| {
                retentions.set(retentions.get() + 1);
                assert_eq!(
                    custody as *mut dyn OperationCustodyPort as *mut (),
                    custody_identity
                );
                assert_eq!(
                    anchor as *mut dyn AntiRollbackAnchorPort as *mut (),
                    anchor_identity
                );
                let custody_error = custody
                    .read_collection_snapshot(&selected_store, &custody_probe, 1, 1)
                    .expect_err("probe store has no collection reader");
                assert_eq!(
                    custody_error.kind(),
                    ControlStoreErrorKind::RecoveryRequired
                );
                assert!(anchor
                    .read(&AntiRollbackAnchorKey::PrincipalTrustMinimum(
                        principal_id.clone(),
                    ))
                    .expect("retention anchor read")
                    .is_none());
                ConsultationChildRetentionOutcome::<u8, &'static str>::Retained {
                    child: Box::new(retained_child),
                    work: 7,
                }
            },
        )
        .expect("exact process Consult selection");

    assert!(matches!(
        disposition,
        ResidentConsultationTurnDisposition::Progress(
            ResidentConsultationTurnProgress::Offered { .. }
        )
    ));
    assert_eq!(recoveries.get(), 1);
    assert_eq!(executor.polls, 1);
    assert_eq!(executor.accepts, 1);
    assert_eq!(decisions.get(), 1);
    assert_eq!(child_reads.get(), 1);
    assert_eq!(retentions.get(), 1);
    assert_eq!(anchor.reads, 1);

    with_unproved_operation_service_for_test(
        &mut host,
        &selector,
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut custody,
            sessions: &deployment_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_store, service| {
            assert_eq!(service.active_consultation_batches(), 1);
            assert_eq!(service.outstanding_task_reservations(), 1);
        },
    )
    .expect("custody and anchor ports can be reborrowed after the task turn");
}

#[test]
fn recovered_consultation_from_another_store_is_rejected_before_task_io() {
    let deployment_port = MemoryAuthorityStore::default();
    let principal_id = principal("principal-consult-store");
    let selected_store = store("consult-selected", "consult-selected-incarnation");
    let foreign_store = store("consult-foreign", "consult-foreign-incarnation");
    let selector = unproved_selector(principal_id.clone(), selected_store.clone());
    let deployment = unproved_deployment_for_selector(
        &selector,
        selected_store.clone(),
        ready_application(&deployment_port, selected_store.clone(), 1),
    );
    let mut host = ResidentProcessHost::new(
        unproved_writer(principal_id.clone(), vec![selected_store.clone()]),
        vec![deployment],
        NonZeroUsize::new(1).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one exact Consult deployment");
    let integrated = integrated_signing_authority(&selector);
    let checkpoint = local_operation_start_fixture::checkpoint_preimage(1, None);
    let base_checkpoint_authority =
        local_operation_start_fixture::checkpoint_authority(&checkpoint);
    let checkpoint_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
        principal_id: &principal_id,
        control_store_id: selected_store.store_id(),
        control_incarnation_id: selected_store.incarnation_id(),
        ..base_checkpoint_authority
    };
    let current = consultation_supervision_fixture_in_store(
        "operation/foreign-consult-parent",
        "foreign-consult-child",
        100,
        false,
        foreign_store,
    );
    let decisions = std::cell::Cell::new(0);
    let child_reads = std::cell::Cell::new(0);
    let retentions = std::cell::Cell::new(0);
    let mut executor = CountingConsultationExecutor::default();
    let (mut authority, mut custody, mut anchor) = operation_ports();

    let result = host.supervise_consultation_turn(
        &selector,
        ResidentOperationControlAssociation {
            current_request_authority: &integrated,
            historical_handle_authority: &integrated,
            recovery_store: &selected_store,
            checkpoint_authority: &checkpoint_authority,
        },
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut custody,
            sessions: &deployment_port,
            anchor: &mut anchor,
        },
        &mut authority,
        consultation_limits(),
        UnixTimeV1::new(2).expect("turn time"),
        |_application, _service, _sessions| Ok::<_, &'static str>(current),
        &mut executor,
        |_, _| {
            decisions.set(decisions.get() + 1);
            ResidentConsultationSupervisionDecision::<()>::Continue(None)
        },
        |_| {
            child_reads.set(child_reads.get() + 1);
            Err::<RecoveredOperationObservation, _>("child recovery must not run")
        },
        |_, _, _, _| {
            retentions.set(retentions.get() + 1);
            ConsultationChildRetentionOutcome::<u8, &'static str>::Unopened(
                "retention must not run",
            )
        },
    );

    assert!(matches!(
        result,
        Err(ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation)
    ));
    assert_eq!(executor.polls, 0);
    assert_eq!(executor.accepts, 0);
    assert_eq!(decisions.get(), 0);
    assert_eq!(child_reads.get(), 0);
    assert_eq!(retentions.get(), 0);
}

#[test]
fn consult_authority_and_checkpoint_substitutions_fail_before_recovery_or_poll() {
    let deployment_port = MemoryAuthorityStore::default();
    let principal_id = principal("principal-consult-axis");
    let selected_store = store("consult-axis", "consult-axis-incarnation");
    let selector = unproved_selector(principal_id.clone(), selected_store.clone());
    let deployment = unproved_deployment_for_selector(
        &selector,
        selected_store.clone(),
        ready_application(&deployment_port, selected_store.clone(), 1),
    );
    let mut host = ResidentProcessHost::new(
        unproved_writer(principal_id.clone(), vec![selected_store.clone()]),
        vec![deployment],
        NonZeroUsize::new(1).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one exact Consult deployment");
    let integrated = integrated_signing_authority(&selector);
    let mut foreign_selector = selector.clone();
    foreign_selector.runtime_authority_ref =
        ResidentRecordRefV1::parse("authority/consult-foreign")
            .expect("foreign authority reference");
    let foreign_historical = integrated_signing_authority(&foreign_selector);
    let checkpoint = local_operation_start_fixture::checkpoint_preimage(1, None);
    let base_checkpoint_authority =
        local_operation_start_fixture::checkpoint_authority(&checkpoint);
    let checkpoint_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
        principal_id: &principal_id,
        control_store_id: selected_store.store_id(),
        control_incarnation_id: selected_store.incarnation_id(),
        ..base_checkpoint_authority
    };
    let foreign_incarnation = ControlIncarnationIdV1::parse("consult-axis-foreign-incarnation")
        .expect("foreign checkpoint incarnation");
    let foreign_store_id =
        ControlStoreIdV1::parse("consult-axis-foreign-store").expect("foreign checkpoint store");
    let foreign_store_checkpoint_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
        control_store_id: &foreign_store_id,
        ..checkpoint_authority
    };
    let foreign_checkpoint_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
        control_incarnation_id: &foreign_incarnation,
        ..checkpoint_authority
    };
    let recoveries = std::cell::Cell::new(0);
    let decisions = std::cell::Cell::new(0);
    let mut executor = CountingConsultationExecutor::default();
    let (mut authority, mut custody, mut anchor) = operation_ports();

    let historical_result = host.supervise_consultation_turn(
        &selector,
        ResidentOperationControlAssociation {
            current_request_authority: &integrated,
            historical_handle_authority: &foreign_historical,
            recovery_store: &selected_store,
            checkpoint_authority: &checkpoint_authority,
        },
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut custody,
            sessions: &deployment_port,
            anchor: &mut anchor,
        },
        &mut authority,
        consultation_limits(),
        UnixTimeV1::new(2).expect("turn time"),
        |_application, _service, _sessions| {
            recoveries.set(recoveries.get() + 1);
            Ok::<_, &'static str>(consultation_supervision_fixture_in_store(
                "operation/unreachable-historical",
                "unreachable-historical",
                100,
                false,
                selected_store.clone(),
            ))
        },
        &mut executor,
        |_, _| {
            decisions.set(decisions.get() + 1);
            ResidentConsultationSupervisionDecision::<()>::Continue(None)
        },
        |_| Err::<RecoveredOperationObservation, _>("unreachable"),
        |_, _, _, _| ConsultationChildRetentionOutcome::<u8, &'static str>::Unopened("unreachable"),
    );
    assert!(matches!(
        historical_result,
        Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority)
    ));

    let checkpoint_result = host.supervise_consultation_turn(
        &selector,
        ResidentOperationControlAssociation {
            current_request_authority: &integrated,
            historical_handle_authority: &integrated,
            recovery_store: &selected_store,
            checkpoint_authority: &foreign_checkpoint_authority,
        },
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut custody,
            sessions: &deployment_port,
            anchor: &mut anchor,
        },
        &mut authority,
        consultation_limits(),
        UnixTimeV1::new(2).expect("turn time"),
        |_application, _service, _sessions| {
            recoveries.set(recoveries.get() + 1);
            Ok::<_, &'static str>(consultation_supervision_fixture_in_store(
                "operation/unreachable-checkpoint",
                "unreachable-checkpoint",
                100,
                false,
                selected_store.clone(),
            ))
        },
        &mut executor,
        |_, _| {
            decisions.set(decisions.get() + 1);
            ResidentConsultationSupervisionDecision::<()>::Continue(None)
        },
        |_| Err::<RecoveredOperationObservation, _>("unreachable"),
        |_, _, _, _| ConsultationChildRetentionOutcome::<u8, &'static str>::Unopened("unreachable"),
    );
    assert!(matches!(
        checkpoint_result,
        Err(ResidentProcessHostSelectionError::CheckpointAuthorityAssociation)
    ));

    let checkpoint_store_result = host.supervise_consultation_turn(
        &selector,
        ResidentOperationControlAssociation {
            current_request_authority: &integrated,
            historical_handle_authority: &integrated,
            recovery_store: &selected_store,
            checkpoint_authority: &foreign_store_checkpoint_authority,
        },
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut custody,
            sessions: &deployment_port,
            anchor: &mut anchor,
        },
        &mut authority,
        consultation_limits(),
        UnixTimeV1::new(2).expect("turn time"),
        |_application, _service, _sessions| {
            recoveries.set(recoveries.get() + 1);
            Ok::<_, &'static str>(consultation_supervision_fixture_in_store(
                "operation/unreachable-checkpoint-store",
                "unreachable-checkpoint-store",
                100,
                false,
                selected_store.clone(),
            ))
        },
        &mut executor,
        |_, _| {
            decisions.set(decisions.get() + 1);
            ResidentConsultationSupervisionDecision::<()>::Continue(None)
        },
        |_| Err::<RecoveredOperationObservation, _>("unreachable"),
        |_, _, _, _| ConsultationChildRetentionOutcome::<u8, &'static str>::Unopened("unreachable"),
    );
    assert!(matches!(
        checkpoint_store_result,
        Err(ResidentProcessHostSelectionError::CheckpointAuthorityAssociation)
    ));
    assert_eq!(recoveries.get(), 0);
    assert_eq!(decisions.get(), 0);
    assert_eq!(executor.polls, 0);
    assert_eq!(custody.mutation_counts(), (0, 0));
    assert_eq!(anchor.access_counts(), (0, 0));
}

#[test]
fn completion_for_another_selected_deployment_remains_an_owned_foreign_phase() {
    let first_port = MemoryAuthorityStore::default();
    let second_port = MemoryAuthorityStore::default();
    let principal_id = principal("principal-consult-switch");
    let first_store = store("consult-first", "consult-first-incarnation");
    let second_store = store("consult-second", "consult-second-incarnation");
    let first_selector = unproved_selector(principal_id.clone(), first_store.clone());
    let second_selector = unproved_selector(principal_id.clone(), second_store.clone());
    let first = unproved_deployment_for_selector(
        &first_selector,
        first_store.clone(),
        ready_application(&first_port, first_store.clone(), 1),
    );
    let second = unproved_deployment_for_selector(
        &second_selector,
        second_store.clone(),
        ready_application(&second_port, second_store.clone(), 2),
    );
    let mut host = ResidentProcessHost::new(
        unproved_writer(
            principal_id.clone(),
            vec![first_store.clone(), second_store.clone()],
        ),
        vec![first, second],
        NonZeroUsize::new(2).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state_with_consultation_capacity(2, 1)),
    )
    .expect("two exact Consult deployments");
    let first_authority = integrated_signing_authority(&first_selector);
    let second_authority = integrated_signing_authority(&second_selector);
    let checkpoint = local_operation_start_fixture::checkpoint_preimage(1, None);
    let base_checkpoint_authority =
        local_operation_start_fixture::checkpoint_authority(&checkpoint);
    let first_checkpoint_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
        principal_id: &principal_id,
        control_store_id: first_store.store_id(),
        control_incarnation_id: first_store.incarnation_id(),
        ..base_checkpoint_authority
    };
    let second_checkpoint_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
        principal_id: &principal_id,
        control_store_id: second_store.store_id(),
        control_incarnation_id: second_store.incarnation_id(),
        ..base_checkpoint_authority
    };
    let second_operation = OperationFixture::exact();
    let second_current = consultation_supervision_fixture_with_child_in_store(
        "operation/second-parent",
        "second-child",
        operation_handle_coordinate(&second_operation),
        100,
        false,
        second_store.clone(),
    );
    let second_candidate = second_current
        .plan_child_dispatch(UnixTimeV1::new(2).expect("turn time"))
        .expect("second candidate")
        .candidates()[0]
        .clone();
    let second_recovered =
        recovered_consultation_child(&second_operation, second_store.clone(), false);
    let second_retained =
        recovered_consultation_child(&second_operation, second_store.clone(), true);
    let mut executor = CountingConsultationExecutor::default();
    let (mut authority, mut custody, mut anchor) = operation_ports();

    let second_disposition = host
        .supervise_consultation_turn(
            &second_selector,
            ResidentOperationControlAssociation {
                current_request_authority: &second_authority,
                historical_handle_authority: &second_authority,
                recovery_store: &second_store,
                checkpoint_authority: &second_checkpoint_authority,
            },
            ResidentDeploymentOperationAccess {
                store: &second_store,
                custody: &mut custody,
                sessions: &second_port,
                anchor: &mut anchor,
            },
            &mut authority,
            consultation_limits(),
            UnixTimeV1::new(2).expect("turn time"),
            |_application, _service, _sessions| Ok::<_, &'static str>(second_current.clone()),
            &mut executor,
            |_, _| {
                ResidentConsultationSupervisionDecision::<()>::Continue(Some(Box::new(
                    second_candidate,
                )))
            },
            |_| Ok::<_, &'static str>(second_recovered),
            |_, _, _, _| ConsultationChildRetentionOutcome::<u8, &'static str>::Retained {
                child: Box::new(second_retained),
                work: 9,
            },
        )
        .expect("second deployment turn");
    assert!(matches!(
        second_disposition,
        ResidentConsultationTurnDisposition::Progress(
            ResidentConsultationTurnProgress::Offered { .. }
        )
    ));
    executor.complete_first(90);

    let first_current = consultation_supervision_fixture_in_store(
        "operation/first-parent",
        "first-child",
        100,
        false,
        first_store.clone(),
    );
    let first_decisions = std::cell::Cell::new(0);
    let first_child_reads = std::cell::Cell::new(0);
    let first_retentions = std::cell::Cell::new(0);
    let first_disposition = host
        .supervise_consultation_turn(
            &first_selector,
            ResidentOperationControlAssociation {
                current_request_authority: &first_authority,
                historical_handle_authority: &first_authority,
                recovery_store: &first_store,
                checkpoint_authority: &first_checkpoint_authority,
            },
            ResidentDeploymentOperationAccess {
                store: &first_store,
                custody: &mut custody,
                sessions: &first_port,
                anchor: &mut anchor,
            },
            &mut authority,
            consultation_limits(),
            UnixTimeV1::new(3).expect("turn time"),
            |_application, _service, _sessions| Ok::<_, &'static str>(first_current),
            &mut executor,
            |_, _| {
                first_decisions.set(first_decisions.get() + 1);
                ResidentConsultationSupervisionDecision::<()>::Continue(None)
            },
            |_| {
                first_child_reads.set(first_child_reads.get() + 1);
                Err::<RecoveredOperationObservation, _>("foreign completion wins")
            },
            |_, _, _, _| {
                first_retentions.set(first_retentions.get() + 1);
                ConsultationChildRetentionOutcome::<u8, &'static str>::Unopened(
                    "foreign completion wins",
                )
            },
        )
        .expect("first deployment selection");

    let ResidentConsultationTurnDisposition::NeedsRecovery(reason) = first_disposition else {
        panic!("second deployment completion must remain an owned recovery phase")
    };
    let ResidentConsultationTurnNeedsRecovery::ForeignCompletion(poll) = *reason else {
        panic!("second deployment completion must not be consumed by the first")
    };
    assert_eq!(poll.coordinate.store(), &second_store);
    assert_eq!(first_decisions.get(), 0);
    assert_eq!(first_child_reads.get(), 0);
    assert_eq!(first_retentions.get(), 0);
    assert_eq!(executor.polls, 2);

    let failure = match host.bind_consultation_completion(
        first_selector.clone(),
        ResidentConsultationCompletionReady::from_parts(second_current, *poll),
    ) {
        Ok(_) => panic!("a completion cannot bind to another deployment"),
        Err(failure) => failure,
    };
    assert!(matches!(
        failure.reason,
        ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation
    ));
    assert_eq!(failure.selector, first_selector);
    assert_eq!(failure.ready.store(), &second_store);

    with_unproved_operation_service_for_test(
        &mut host,
        &first_selector,
        ResidentDeploymentOperationAccess {
            store: &first_store,
            custody: &mut custody,
            sessions: &first_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_store, service| {
            assert_eq!(service.active_consultation_batches(), 2);
            assert_eq!(service.outstanding_task_reservations(), 1);
        },
    )
    .expect("one task owner remains visible through either selected deployment");
}

#[test]
fn consult_completion_and_stop_phases_outlive_recovery_port_borrows() {
    let deployment_port = MemoryAuthorityStore::default();
    let principal_id = principal("principal-consult-phases");
    let selected_store = store("consult-phases", "consult-phases-incarnation");
    let selector = unproved_selector(principal_id.clone(), selected_store.clone());
    let deployment = unproved_deployment_for_selector(
        &selector,
        selected_store.clone(),
        ready_application(&deployment_port, selected_store.clone(), 1),
    );
    let mut host = ResidentProcessHost::new(
        unproved_writer(principal_id.clone(), vec![selected_store.clone()]),
        vec![deployment],
        NonZeroUsize::new(1).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state_with_consultation_capacity(2, 1)),
    )
    .expect("one exact Consult deployment");
    let integrated = integrated_signing_authority(&selector);
    let checkpoint = local_operation_start_fixture::checkpoint_preimage(1, None);
    let base_checkpoint_authority =
        local_operation_start_fixture::checkpoint_authority(&checkpoint);
    let checkpoint_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
        principal_id: &principal_id,
        control_store_id: selected_store.store_id(),
        control_incarnation_id: selected_store.incarnation_id(),
        ..base_checkpoint_authority
    };
    let operation = OperationFixture::exact();
    let completion_current = consultation_supervision_fixture_with_child_in_store(
        "operation/completion-phase-parent",
        "completion-phase-child",
        operation_handle_coordinate(&operation),
        100,
        false,
        selected_store.clone(),
    );
    let candidate = completion_current
        .plan_child_dispatch(UnixTimeV1::new(2).expect("turn time"))
        .expect("one exact candidate")
        .candidates()[0]
        .clone();
    let recovered_child = recovered_consultation_child(&operation, selected_store.clone(), false);
    let retained_child = recovered_consultation_child(&operation, selected_store.clone(), true);
    let mut executor = CountingConsultationExecutor::default();
    let (mut authority, mut custody, mut anchor) = operation_ports();

    let offered = host
        .supervise_consultation_turn(
            &selector,
            ResidentOperationControlAssociation {
                current_request_authority: &integrated,
                historical_handle_authority: &integrated,
                recovery_store: &selected_store,
                checkpoint_authority: &checkpoint_authority,
            },
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut custody,
                sessions: &deployment_port,
                anchor: &mut anchor,
            },
            &mut authority,
            consultation_limits(),
            UnixTimeV1::new(2).expect("turn time"),
            |_application, _service, _sessions| Ok::<_, &'static str>(completion_current.clone()),
            &mut executor,
            |_, _| {
                ResidentConsultationSupervisionDecision::<()>::Continue(Some(Box::new(candidate)))
            },
            |_| Ok::<_, &'static str>(recovered_child),
            |_, _, _, _| ConsultationChildRetentionOutcome::<u8, &'static str>::Retained {
                child: Box::new(retained_child),
                work: 11,
            },
        )
        .expect("exact offer selection");
    assert!(matches!(
        offered,
        ResidentConsultationTurnDisposition::Progress(
            ResidentConsultationTurnProgress::Offered { .. }
        )
    ));
    executor.complete_first(110);

    let ready = host
        .supervise_consultation_turn(
            &selector,
            ResidentOperationControlAssociation {
                current_request_authority: &integrated,
                historical_handle_authority: &integrated,
                recovery_store: &selected_store,
                checkpoint_authority: &checkpoint_authority,
            },
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut custody,
                sessions: &deployment_port,
                anchor: &mut anchor,
            },
            &mut authority,
            consultation_limits(),
            UnixTimeV1::new(3).expect("turn time"),
            |_application, _service, _sessions| Ok::<_, &'static str>(completion_current),
            &mut executor,
            |_, _| -> ResidentConsultationSupervisionDecision<()> {
                panic!("ready completion must precede policy")
            },
            |_| -> Result<RecoveredOperationObservation, &'static str> {
                panic!("ready completion must precede child recovery")
            },
            |_, _, _, _| -> ConsultationChildRetentionOutcome<u8, &'static str> {
                panic!("ready completion must precede retention")
            },
        )
        .expect("exact completion selection");
    let ResidentConsultationTurnDisposition::Progress(
        ResidentConsultationTurnProgress::CompletionReady(ready),
    ) = ready
    else {
        panic!("exact completion must remain an owned phase")
    };

    with_unproved_operation_service_for_test(
        &mut host,
        &selector,
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut custody,
            sessions: &deployment_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_store, service| assert_eq!(service.outstanding_task_reservations(), 1),
    )
    .expect("completion carrier does not retain a port borrow");
    let bound = match host.bind_consultation_completion(selector.clone(), *ready) {
        Ok(bound) => bound,
        Err(_) => panic!("ready completion binds to its exact deployment"),
    };
    let ResidentProcessConsultationCompletionReady {
        selector: bound_selector,
        ready,
    } = bound;
    assert_eq!(bound_selector, selector);
    let (completion_parent, completion_poll) = ready.into_parts();
    assert_eq!(completion_parent.store(), &selected_store);
    assert_eq!(completion_poll.coordinate.store(), &selected_store);

    let stop_current = consultation_supervision_fixture_in_store(
        "operation/stop-phase-parent",
        "stop-phase-child",
        100,
        false,
        selected_store.clone(),
    );
    let stopped = host
        .supervise_consultation_turn(
            &selector,
            ResidentOperationControlAssociation {
                current_request_authority: &integrated,
                historical_handle_authority: &integrated,
                recovery_store: &selected_store,
                checkpoint_authority: &checkpoint_authority,
            },
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut custody,
                sessions: &deployment_port,
                anchor: &mut anchor,
            },
            &mut authority,
            consultation_limits(),
            UnixTimeV1::new(4).expect("turn time"),
            |_application, _service, _sessions| Ok::<_, &'static str>(stop_current),
            &mut executor,
            |_, _| ResidentConsultationSupervisionDecision::Stop("selected-stop"),
            |_| -> Result<RecoveredOperationObservation, &'static str> {
                panic!("selected stop performs no child recovery")
            },
            |_, _, _, _| -> ConsultationChildRetentionOutcome<u8, &'static str> {
                panic!("selected stop performs no child retention")
            },
        )
        .expect("exact stop selection");
    let ResidentConsultationTurnDisposition::Stopped(
        crate::resident::consultation::ResidentConsultationTurnStopped::Selected(stopped),
    ) = stopped
    else {
        panic!("Package stop must remain an owned phase")
    };

    with_unproved_operation_service_for_test(
        &mut host,
        &selector,
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut custody,
            sessions: &deployment_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_store, service| assert_eq!(service.active_consultation_batches(), 2),
    )
    .expect("stop carrier does not retain a port borrow");
    let (stop_parent, stop_input) = stopped.into_parts();
    assert_eq!(stop_parent.store(), &selected_store);
    assert_eq!(stop_input, "selected-stop");
}

#[test]
fn selected_consult_stop_keeps_exact_process_custody_until_anchor_completion() {
    let (generation, profile) =
        crate::resident::operation_observation_custody::tests::local_start::source_generation();
    crate::local_operation_start_fixture::with_consult_start(
        &generation,
        &profile,
        "process-host-selected-stop",
        |_| {},
        crate::local_operation_start_fixture::StartAdmission::AcceptedOwnerModel,
        |prepared, request, _, _, retained| {
            let prepared = crate::resident::operation_observation_custody::tests::local_start::prepared_initial(
                prepared.expect("Consult parent start"),
            );
            let (physical, pending, parent_checkpoint) =
                crate::resident::operation_observation_custody::tests::local_start::issue_initial(
                    prepared,
                    request,
                    |_| {},
                    &std::cell::Cell::new(0),
                )
                .expect("Consult parent checkpoint");
            let mut start_store =
                crate::resident::operation_observation_custody::tests::local_start::absent_start_store(
                    request,
                );
            let mut start_anchor =
                crate::resident::operation_observation_custody::tests::local_start::initial_anchor(
                );
            let database = physical
                .commit(&mut start_store, &mut start_anchor, 1_048_576)
                .expect("Consult parent database custody");
            let initial_record = database
                .candidate()
                .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
                .expect("Consult parent resolving observation")
                .clone();
            let parent = pending
                .finish(
                    database
                        .advance_anchor(&mut start_anchor)
                        .expect("Consult parent anchor custody"),
                )
                .unwrap_or_else(|_| panic!("Consult parent locator"));

            let selected_store = crate::local_operation_start_fixture::store_coordinate();
            let principal_id = retained.signing.principal_id().clone();
            let selector = selector_from_signing_authority(retained.signing);
            let deployment_port = MemoryAuthorityStore::default();
            let deployment = unproved_deployment_for_selector(
                &selector,
                selected_store.clone(),
                ready_application(&deployment_port, selected_store.clone(), 1),
            );
            let mut host = ResidentProcessHost::new(
                unproved_writer(principal_id, vec![selected_store.clone()]),
                vec![deployment],
                NonZeroUsize::new(1).expect("positive deployment bound"),
                ResidentOperationHost::new(task_state_with_consultation_capacity(1, 1)),
            )
            .expect("one exact Consult deployment");

            let child = ImmutableRecordReferenceV1 {
                record_ref: local_operation_start_fixture::record_ref(
                    "operation/process-host-selected-stop-child",
                ),
                record_sha256: ArtifactDigest::hash_bytes(b"process-host-selected-stop-child"),
            };
            let current = consultation_supervision_fixture_for_operation_in_store(
                parent.handle().0,
                parent.handle().1,
                "process-host-selected-stop-child",
                child,
                false,
                selected_store.clone(),
            );
            let parent_checkpoint_ref = local_operation_start_fixture::record_ref(
                "operation/process-host-selected-stop-parent-checkpoint",
            );
            let parent_checkpoint_digest = parent_checkpoint
                .canonical_control_checkpoint_sha256()
                .clone();
            let mut successor_authority_source = local_operation_start_fixture::checkpoint_preimage(
                2,
                Some((parent_checkpoint_ref.clone(), parent_checkpoint_digest)),
            );
            successor_authority_source.checkpoint_domain_id =
                parent_checkpoint.checkpoint_domain_id().clone();
            successor_authority_source.checkpoint_profile_ref =
                parent_checkpoint.checkpoint_profile_ref().clone();
            successor_authority_source.checkpoint_profile_sha256 =
                parent_checkpoint.checkpoint_profile_sha256().clone();
            successor_authority_source.domain_epoch = *parent_checkpoint.domain_epoch();
            let checkpoint_authority =
                local_operation_start_fixture::checkpoint_authority(&successor_authority_source);
            let observation_schema = local_operation_start_fixture::schema_digest(
                pale_core::records::OPERATION_OBSERVATION_V1_SCHEMA_ID,
            );
            let parent_observation = OperationObservationV1::admit_contextual_wire(
                serde_json::from_value(
                    canon::parse_strict(initial_record.canonical_record_jcs().as_slice())
                        .expect("strict parent observation JSON"),
                )
                .expect("parent observation wire"),
                &OperationObservationAdmissionContextV1 {
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: retained.signing,
                    },
                    handle_ref: parent.handle().0,
                    handle: parent.handle().1,
                    predecessor: None,
                },
                |_| Ok(()),
                |_, _, _, _| panic!("revision one cannot rotate authority"),
                |_, _, _, _| Ok(()),
            )
            .expect("parent resolving observation admits");
            let parent_observation_ref = initial_record.coordinate().record_ref().clone();
            let collection_frontier = current.snapshot().frontier().coordinate();
            let collecting_evidence = ImmutableRecordReferenceV1 {
                record_ref: collection_frontier.record_ref().clone(),
                record_sha256: ArtifactDigest::parse(collection_frontier.record_sha256().as_str())
                    .expect("collection frontier digest"),
            };
            let collecting_observation_ref = local_operation_start_fixture::record_ref(
                "observation/process-host-selected-stop-collecting",
            );
            let collecting_observation = local_operation_start_fixture::issue_observation(
                retained.signing,
                parent.handle().0,
                parent.handle().1,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(2).expect("collecting revision"),
                Some((&parent_observation_ref, &parent_observation)),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::CollectingConsultation {
                        evidence: collecting_evidence,
                    },
                },
                64,
            );
            let checkpoint_schema = local_operation_start_fixture::schema_digest(
                pale_core::records::CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
            );
            let stop_input = ConsultationStopInput {
                cause: ConsultationStopCause::CompletionPolicy,
                reason: ImmutableRecordReferenceV1 {
                    record_ref: local_operation_start_fixture::record_ref(
                        "policy/process-host-selected-stop",
                    ),
                    record_sha256: ArtifactDigest::hash_bytes(b"process-host-selected-stop"),
                },
                stopped_at: UnixTimeV1::new(65).expect("stop time"),
                parent: &parent,
                parent_observation: (&collecting_observation_ref, &collecting_observation),
                parent_checkpoint: (&parent_checkpoint_ref, &parent_checkpoint),
                observation_ref: local_operation_start_fixture::record_ref(
                    "observation/process-host-selected-stop",
                ),
                observed_at: UnixTimeV1::new(65).expect("observation time"),
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority: retained.signing,
                },
                limits: ConsultationCollectionLimits::new(1, 16_384, 4096)
                    .expect("bounded collection"),
            };
            let mut executor = CountingConsultationExecutor::default();
            let mut authority = ProvingAuthority;
            let mut stop_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: pale_core::records::ControlTransactionSequenceV1::new(2)
                    .expect("successor transaction"),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let mut unavailable_anchor = RecordingAnchor::unavailable_after_preflight();

            let selected = host
                .supervise_consultation_turn(
                    &selector,
                    ResidentOperationControlAssociation {
                        current_request_authority: retained.signing,
                        historical_handle_authority: retained.signing,
                        recovery_store: &selected_store,
                        checkpoint_authority: &checkpoint_authority,
                    },
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut stop_custody,
                        sessions: &deployment_port,
                        anchor: &mut unavailable_anchor,
                    },
                    &mut authority,
                    consultation_limits(),
                    UnixTimeV1::new(65).expect("turn time"),
                    |_application, _service, _sessions| Ok::<_, &'static str>(current),
                    &mut executor,
                    |_, _| ResidentConsultationSupervisionDecision::Stop(stop_input),
                    |_| -> Result<RecoveredOperationObservation, &'static str> {
                        panic!("a selected stop performs no child recovery")
                    },
                    |_, _, _, _| -> ConsultationChildRetentionOutcome<u8, &'static str> {
                        panic!("a selected stop performs no child retention")
                    },
                )
                .expect("exact stop selection");
            let ResidentConsultationTurnDisposition::Stopped(
                crate::resident::consultation::ResidentConsultationTurnStopped::Selected(selected),
            ) = selected
            else {
                panic!("Package stop must remain an owned selected phase")
            };

            let mut foreign_selector = selector.clone();
            foreign_selector.runtime_authority_ref =
                ResidentRecordRefV1::parse("authority/process-host-selected-stop-foreign")
                    .expect("foreign authority reference");
            let binding_failure =
                match host.bind_consultation_stop(foreign_selector.clone(), *selected) {
                    Ok(_) => panic!("an unknown deployment selector cannot bind the stop"),
                    Err(failure) => failure,
                };
            assert!(matches!(
                binding_failure.reason,
                ResidentProcessHostSelectionError::UnknownDeploymentAuthority
            ));
            assert_eq!(binding_failure.selector, foreign_selector);
            assert_eq!(binding_failure.selected.store(), &selected_store);
            assert_eq!(stop_custody.physical.mutation_counts(), (0, 0));
            assert_eq!(unavailable_anchor.access_counts(), (0, 0));

            let selected = host
                .bind_consultation_stop(selector.clone(), *binding_failure.selected)
                .unwrap_or_else(|_| panic!("exact deployment binds the selected stop"));
            prove_selected_stop_process_custody(
                &mut host,
                &selector,
                &selected_store,
                &deployment_port,
                &mut authority,
                &mut stop_custody,
                &mut unavailable_anchor,
                selected,
                retained.signing,
                &checkpoint_authority,
                &checkpoint_schema,
                retained.session,
            );
            assert_eq!(stop_custody.physical.mutation_counts(), (1, 1));
            assert_eq!(unavailable_anchor.access_counts(), (1, 1));
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn reject_selected_stop_before_io<'operation>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    access_store: &AuthorityStoreCoordinate,
    sessions: &dyn pale_ports::SessionLeaseCustodyPort,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    authority: &mut ProvingAuthority,
    selected: ResidentProcessConsultationStopSelected<'operation>,
    association: ResidentOperationControlAssociation<'_>,
    checkpoint: ConsultationSuccessorCommitInput<'_>,
    resolver: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
) -> (
    ResidentProcessConsultationStopSelected<'operation>,
    ResidentProcessHostSelectionError,
) {
    match host.commit_consultation_stop(
        association,
        ResidentDeploymentOperationAccess {
            store: access_store,
            custody,
            sessions,
            anchor,
        },
        authority,
        selected,
        checkpoint,
        resolver,
        |_, _| panic!("selection must precede stop admission"),
        |_| panic!("selection must precede collection admission"),
        |_, _, _, _| panic!("selection must precede rotation admission"),
        |_: &[u8], _: &OperationObservationPreimageV1, _: &OperationObservationDigest| {
            panic!("selection must precede observation signing")
        },
        |_: &[u8],
         _: &ControlStateCheckpointPreimageV1,
         _: &pale_core::records::ControlStateCheckpointDigest| {
            panic!("selection must precede checkpoint signing")
        },
    ) {
        Err(ResidentProcessConsultationStopCommitFailure::Selection { selected, reason }) => {
            (*selected, reason)
        }
        Ok(_) => panic!("selection failure cannot commit a stop"),
        Err(_) => panic!("selection failure cannot enter physical custody"),
    }
}

#[allow(clippy::too_many_arguments)]
fn prove_selected_stop_process_custody<'operation>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    selected_store: &AuthorityStoreCoordinate,
    sessions: &dyn pale_ports::SessionLeaseCustodyPort,
    authority: &mut ProvingAuthority,
    custody: &mut dyn OperationCustodyPort,
    unavailable_anchor: &mut RecordingAnchor,
    mut selected: ResidentProcessConsultationStopSelected<'operation>,
    signing: &SigningAuthorityBindingV1,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    checkpoint_schema: &SchemaDigest,
    session: &crate::resident::AdmittedAmbientSession,
) {
    let checkpoint_input = |authority| ConsultationSuccessorCommitInput {
        accumulator_limits: AccumulatorPreparationLimits::new(1, 1024, 1024, 512, 65_536)
            .expect("bounded successor accumulator"),
        identity: OperationCheckpointInput {
            reference: local_operation_start_fixture::record_ref(
                "operation/process-host-selected-stop-checkpoint",
            ),
            id: ControlCheckpointIdV1::parse("process-host-selected-stop-checkpoint")
                .expect("checkpoint identity"),
        },
        issued_at: UnixTimeV1::new(66).expect("checkpoint time"),
        schema: checkpoint_schema,
        authority,
    };
    let resolver =
        crate::resident::operation_observation_custody::tests::local_start::StartStore(session);
    let exact_association = || ResidentOperationControlAssociation {
        current_request_authority: signing,
        historical_handle_authority: signing,
        recovery_store: selected_store,
        checkpoint_authority,
    };

    let foreign_access_store = store(
        "process-host-stop-foreign-store",
        "process-host-stop-foreign-incarnation",
    );
    let rejected = reject_selected_stop_before_io(
        host,
        &foreign_access_store,
        sessions,
        custody,
        unavailable_anchor,
        authority,
        selected,
        exact_association(),
        checkpoint_input(checkpoint_authority),
        &resolver,
    );
    selected = rejected.0;
    assert!(matches!(
        rejected.1,
        ResidentProcessHostSelectionError::AccessStoreAssociation
    ));

    let mut foreign_selector = selector.clone();
    foreign_selector.runtime_authority_ref =
        ResidentRecordRefV1::parse("authority/process-host-selected-stop-foreign-signing")
            .expect("foreign signing authority reference");
    let foreign_signing = integrated_signing_authority(&foreign_selector);
    let rejected = reject_selected_stop_before_io(
        host,
        selected_store,
        sessions,
        custody,
        unavailable_anchor,
        authority,
        selected,
        ResidentOperationControlAssociation {
            current_request_authority: &foreign_signing,
            ..exact_association()
        },
        checkpoint_input(checkpoint_authority),
        &resolver,
    );
    selected = rejected.0;
    assert!(matches!(
        rejected.1,
        ResidentProcessHostSelectionError::CurrentRequestAuthority
    ));

    let rejected = reject_selected_stop_before_io(
        host,
        selected_store,
        sessions,
        custody,
        unavailable_anchor,
        authority,
        selected,
        ResidentOperationControlAssociation {
            historical_handle_authority: &foreign_signing,
            ..exact_association()
        },
        checkpoint_input(checkpoint_authority),
        &resolver,
    );
    selected = rejected.0;
    assert!(matches!(
        rejected.1,
        ResidentProcessHostSelectionError::HistoricalHandleAuthority
    ));

    let foreign_checkpoint_store = store(
        "process-host-stop-checkpoint-store",
        "process-host-stop-checkpoint-incarnation",
    );
    let foreign_checkpoint_source = local_operation_start_fixture::checkpoint_preimage_at_store(
        2,
        None,
        &foreign_checkpoint_store,
    );
    let foreign_checkpoint_authority =
        local_operation_start_fixture::checkpoint_authority(&foreign_checkpoint_source);
    let rejected = reject_selected_stop_before_io(
        host,
        selected_store,
        sessions,
        custody,
        unavailable_anchor,
        authority,
        selected,
        ResidentOperationControlAssociation {
            checkpoint_authority: &foreign_checkpoint_authority,
            ..exact_association()
        },
        checkpoint_input(checkpoint_authority),
        &resolver,
    );
    selected = rejected.0;
    assert!(matches!(
        rejected.1,
        ResidentProcessHostSelectionError::CheckpointAuthorityAssociation
    ));

    let rejected = reject_selected_stop_before_io(
        host,
        selected_store,
        sessions,
        custody,
        unavailable_anchor,
        authority,
        selected,
        exact_association(),
        checkpoint_input(&foreign_checkpoint_authority),
        &resolver,
    );
    selected = rejected.0;
    assert!(matches!(
        rejected.1,
        ResidentProcessHostSelectionError::ConsultationCheckpointAuthority
    ));

    let pending = match host.commit_consultation_stop(
        exact_association(),
        ResidentDeploymentOperationAccess {
            store: selected_store,
            custody,
            sessions,
            anchor: unavailable_anchor,
        },
        authority,
        selected,
        checkpoint_input(checkpoint_authority),
        &resolver,
        |_, summary| {
            assert_eq!(summary.required_outstanding().len(), 1);
            Ok(())
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(local_operation_start_fixture::signature()),
        |_, _, _| Ok(local_operation_start_fixture::signature()),
    ) {
        Err(ResidentProcessConsultationStopCommitFailure::Custody(pending)) => *pending,
        Ok(_) => panic!("unavailable anchor must retain exact stop custody"),
        Err(ResidentProcessConsultationStopCommitFailure::Selection { reason, .. }) => {
            panic!("exact stop selection failed: {reason:?}")
        }
        Err(ResidentProcessConsultationStopCommitFailure::Admission { reason, .. }) => {
            panic!("exact stop admission failed: {reason:?}")
        }
    };

    let mut recovery_anchor =
        crate::resident::operation_observation_custody::tests::local_start::successor_anchor();
    let completed = host
        .continue_consultation_stop_custody(
            ResidentDeploymentOperationAccess {
                store: selected_store,
                custody,
                sessions,
                anchor: &mut recovery_anchor,
            },
            authority,
            pending,
        )
        .unwrap_or_else(|_| panic!("retained anchor continuation completes once"));
    assert_eq!(completed.custody().candidate().store(), selected_store);
    assert_eq!(recovery_anchor.access_counts(), (1, 0));
}
