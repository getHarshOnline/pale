//! Bounded cold-recovery laws for root Consult operations.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::rc::Rc;

use super::restart::{host_with_generation, restart_input};
use super::start::CountingAuthority;
use super::*;
use crate::resident::consultation::tests::{
    consultation_supervision_fixture_in_store, consultation_supervision_fixture_with_child_in_store,
};
use crate::resident::consultation::{
    ConsultationChildRetentionOutcome, ConsultationCollectionLimits,
    ResidentConsultationPolicyFrontier, ResidentConsultationPrePolicyNeedsRecovery,
    ResidentConsultationPrePolicyTurn, ResidentConsultationTaskLimits,
    ResidentConsultationTurnStopped,
};
use crate::resident::local_operation_start::{
    discover_local_operation_start, LocalOperationStartDiscoveryTurn,
};
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_process_host_recovery_sweep_fixture, with_process_host_restart_fixtures,
    ProcessHostRestartFixture,
};
use crate::resident::operation_observation_custody::tests::{
    admit_initial_checkpoint, OperationFixture,
};
use crate::resident::process_host::consult_recovery::*;
use crate::resident::process_host::operation_recovery_sweep::*;
use crate::resident::process_host::operation_request_host::{
    ResidentOperationRequestHost, ResidentOperationRequestKind, ResidentOperationRequestResolution,
};
use crate::resident::process_host::program::ResidentProgramConsultTurn;
use crate::resident::process_host::restart::{
    ResidentProcessRecoveredAttempt, ResidentProcessRecoveredLocalCancellation,
    ResidentProcessRecoveredOperationResolution, ResidentProcessRecoveredPreparedAttempt,
    ResidentProcessRecoveredProviderResolution,
};
use crate::resident::task_owner::{
    ResidentOperationTask, ResidentOperationTaskCompletion, ResidentOperationTaskExecutor,
};
use crate::resident::{
    LocalOperationHandleRecoveryInput, OperationControlRecoveryAdmission,
    OperationControlRecoveryInput, RecoveredOperationObservation,
};
use pale_core::records::{
    ArtifactDigest, CapabilityAttemptIdV1, ControlIncarnationIdV1, ControlStoreIdV1,
    ImmutableRecordReferenceV1, OperationActivePhaseV1, OperationObservationRevisionV1,
    OperationObservationStateV1, OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    ControlRecordCoordinate, ControlRecordDigest, ResidentOperationFrameLocator,
    ResidentOperationFrameRequest, ResidentOperationFrameResponse, WallClockSourceError,
    WallClockSourcePort,
};

#[derive(Default)]
struct ProbeExecutor {
    accepts: usize,
    polls: usize,
}

impl ResidentOperationTaskExecutor<u8, u16> for ProbeExecutor {
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<u8>,
    ) -> Result<(), ResidentOperationTask<u8>> {
        self.accepts += 1;
        Err(task)
    }

    fn try_completion(&mut self) -> Option<ResidentOperationTaskCompletion<u8, u16>> {
        self.polls += 1;
        None
    }
}

#[derive(Default)]
struct ReadyExecutor {
    accepts: usize,
    polls: usize,
    accepted: Vec<ResidentOperationTask<u8>>,
    completions: VecDeque<ResidentOperationTaskCompletion<u8, u16>>,
}

impl ReadyExecutor {
    fn complete_first(&mut self, output: u16) {
        let task = self.accepted.remove(0);
        self.completions
            .push_back(ResidentOperationTaskCompletion::Completed { task, output });
    }
}

impl ResidentOperationTaskExecutor<u8, u16> for ReadyExecutor {
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
struct FixedClock(UnixTimeV1);

impl WallClockSourcePort for FixedClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Ok(self.0)
    }
}

#[derive(Default)]
struct RootOwner {
    calls: usize,
}

impl ResidentProcessRootProviderRecoveryOwner<UnprovedHeldWriter> for RootOwner {
    type Output = ();
    type Error = ();

    fn complete_local_cancellation<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredLocalCancellation>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.calls += 1;
        Ok(())
    }

    fn recover_operation_resolution<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredOperationResolution>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<ResidentProcessRootProviderResolution<Self::Output>, Self::Error> {
        self.calls += 1;
        Ok(ResidentProcessRootProviderResolution::Provider(()))
    }

    fn recover_provider_resolved<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredProviderResolution>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.calls += 1;
        Ok(())
    }

    fn recover_provider_prepared<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredPreparedAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.calls += 1;
        Ok(())
    }

    fn recover_provider_dispatched<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.calls += 1;
        Ok(())
    }

    fn recover_native_prepared<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredPreparedAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.calls += 1;
        Ok(())
    }

    fn recover_native_dispatched<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.calls += 1;
        Ok(())
    }
}

fn reborrow_admission<'short>(
    admission: &'short mut OperationControlRecoveryAdmission<'_>,
) -> OperationControlRecoveryAdmission<'short> {
    OperationControlRecoveryAdmission {
        admit_request: &mut *admission.admit_request,
        admit_handle_authority: &mut *admission.admit_handle_authority,
        verify_handle_signature: &mut *admission.verify_handle_signature,
        admit_checkpoint: &mut *admission.admit_checkpoint,
        resolve_observation_signer: &mut *admission.resolve_observation_signer,
        admit_observation_state: &mut *admission.admit_observation_state,
        admit_rotation: &mut *admission.admit_rotation,
        verify_observation_signature: &mut *admission.verify_observation_signature,
        verify_checkpoint_signature: &mut *admission.verify_checkpoint_signature,
    }
}

fn restart_from<'a>(
    recovery: &'a OperationControlRecoveryInput<'a>,
) -> ResidentProcessOperationRestartInput<'a> {
    let LocalOperationHandleRecoveryInput {
        context,
        expected_handle,
        maximum_parent_depth,
    } = recovery.operation;
    restart_input(
        OperationControlRecoveryInput {
            operation: LocalOperationHandleRecoveryInput {
                context,
                expected_handle,
                maximum_parent_depth,
            },
            observation: recovery.observation,
            observation_limits: recovery.observation_limits,
            checkpoint_limits: recovery.checkpoint_limits,
            membership_limits: recovery.membership_limits,
        },
        1 << 20,
    )
}

fn collection_limits() -> ConsultationCollectionLimits {
    ConsultationCollectionLimits::new(1, 65_536, 65_536).expect("collection limits")
}

fn task_limits() -> ResidentConsultationTaskLimits {
    ResidentConsultationTaskLimits::new(
        NonZeroUsize::new(1).expect("one in-flight child"),
        NonZeroUsize::new(1).expect("one batch member"),
    )
}

#[test]
fn cold_root_consult_preserves_exact_start_across_policy_and_stop() {
    with_process_host_root_consult_policy_fixture(&mut move |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            mut admission,
            signing_authority,
            request,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let mut executor = ProbeExecutor::default();
        let mut authority = CountingAuthority::default();
        let resolver = |_: &OperationObservationV1| Ok(selected_store.clone());

        let retained_locator = ResidentOperationFrameLocator::new(
            recovery
                .operation
                .expected_handle
                .record_ref()
                .as_str()
                .to_owned(),
            recovery
                .operation
                .expected_handle
                .record_sha256()
                .as_str()
                .to_owned(),
        );
        let request_frame = ResidentOperationFrameRequest::AdvanceConsult {
            locator: retained_locator.clone(),
            signed_common_request: pale_core::canon::to_canonical_string(request)
                .expect("signed Consult request is canonical"),
        };
        let mut frame_port = OperationFramePort::new(&request_frame);
        let deadline = std::time::Instant::now()
            .checked_add(std::time::Duration::from_secs(2))
            .expect("test deadline");
        let mut request_host = ResidentOperationRequestHost::new(&mut frame_port, deadline);
        let received = request_host
            .receive()
            .expect("Consult continuation frame is exact");
        let retained_turn = RefCell::new(None);
        let response = operation_request_host::request_host_test_support::answer(
            &mut request_host,
            received,
            &mut host,
            ResidentOperationRequestResolution::retained(
                ResidentOperationRequestKind::AdvanceConsult,
                request,
                retained_locator.clone(),
            ),
            &mut |_, actual| {
                assert_eq!(actual, &request_frame);
                Ok(())
            },
            deadline,
            |host, admitted_request| {
                assert!(std::ptr::eq(admitted_request, request));
                let turn = host
                    .recover_one_root_consult_start(
                        &selector,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody: &mut *custody,
                            sessions,
                            anchor: &mut *anchor,
                        },
                        restart_from(&recovery),
                        reborrow_admission(&mut admission),
                        1 << 20,
                        &mut authority,
                        collection_limits(),
                        task_limits(),
                        UnixTimeV1::new(70).expect("policy time"),
                        &resolver,
                        |_| Ok(()),
                        &mut executor,
                    )
                    .map_err(|_| pale_ports::ResidentOperationFrameFailure::ResidentUnavailable)?;
                if !matches!(turn, ResidentProcessConsultRecoveryTurn::PolicyRequired(_)) {
                    return Err(pale_ports::ResidentOperationFrameFailure::ResidentUnavailable);
                }
                retained_turn.replace(Some(turn));
                Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
            },
        )
        .expect("private request host reaches the real Consult policy boundary");
        assert_eq!(
            response,
            ResidentOperationFrameResponse::Rejected {
                failure: pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable
            }
        );
        let turn = retained_turn
            .into_inner()
            .expect("the real Consult recovery turn was retained");
        let ResidentProcessConsultRecoveryTurn::PolicyRequired(handoff) = turn else {
            panic!(
                "the rebuilt Consult must preserve its start at policy: {}",
                match turn {
                    ResidentProcessConsultRecoveryTurn::Exhausted => "exhausted",
                    ResidentProcessConsultRecoveryTurn::SkippedOtherOwner(_) => "other owner",
                    ResidentProcessConsultRecoveryTurn::ReportedTerminal(_) => "terminal",
                    ResidentProcessConsultRecoveryTurn::Progress(_) => "progress",
                    ResidentProcessConsultRecoveryTurn::Pending(_) => "pending",
                    ResidentProcessConsultRecoveryTurn::StoppedEstablished(_) => "stopped",
                    ResidentProcessConsultRecoveryTurn::PolicyRequired(_) => unreachable!(),
                    ResidentProcessConsultRecoveryTurn::NeedsRecovery(reason) => match *reason {
                        ResidentConsultationPrePolicyNeedsRecovery::Collection(_) => {
                            "needs collection recovery"
                        }
                        ResidentConsultationPrePolicyNeedsRecovery::Planning(_) => {
                            "needs planning recovery"
                        }
                        ResidentConsultationPrePolicyNeedsRecovery::Batch(_) => {
                            "needs batch recovery"
                        }
                        ResidentConsultationPrePolicyNeedsRecovery::Poll(_) => {
                            "needs poll recovery"
                        }
                        ResidentConsultationPrePolicyNeedsRecovery::ForeignCompletion(_) => {
                            "foreign completion"
                        }
                    },
                }
            )
        };
        assert_eq!(handoff.started().selector(), &selector);
        assert_eq!(handoff.started().store(), &selected_store);
        assert!(handoff.started().children().is_empty());
        assert_eq!(
            handoff.frontier().observed_at(),
            UnixTimeV1::new(70).unwrap()
        );
        assert_eq!(
            handoff.started().parent().handle().0,
            &handoff
                .frontier()
                .current()
                .collection()
                .parent()
                .record_ref
        );
        assert!(handoff
            .started()
            .matches_collection(handoff.frontier().current().collection()));
        assert_eq!((executor.accepts, executor.polls), (0, 1));
        assert_eq!(custody.start_reads(), 1);

        let expected_policy = handoff.completion_policy().clone();
        let failure = match host
            .evaluate_recovered_consultation_policy(handoff, &mut UnavailablePolicyEvaluator)
        {
            Ok(_) => panic!("an unavailable selected policy cannot produce a decision"),
            Err(failure) => failure,
        };
        let (handoff, error) = failure.into_parts();
        assert_eq!(error, "selected Package policy is unavailable");
        let retained = ResidentProgramConsultTurn::<
            u8,
            u16,
            std::convert::Infallible,
            std::convert::Infallible,
            u8,
        >::PolicyUnavailable(handoff);
        let ResidentProgramConsultTurn::PolicyUnavailable(handoff) = retained else {
            unreachable!("the unavailable policy retains its exact handoff")
        };
        assert_eq!(handoff.completion_policy(), &expected_policy);
        let mut policy = ExactStopPolicyEvaluator {
            expected: expected_policy,
            calls: 0,
            stop: 7,
        };
        let (handoff, decision) = host
            .evaluate_recovered_consultation_policy(handoff, &mut policy)
            .unwrap_or_else(|_| panic!("the selected Package policy is available"));
        assert_eq!(policy.calls, 1);

        let child_recoveries = Cell::new(0);
        let retentions = Cell::new(0);
        let continued = host
            .continue_recovered_consultation_policy(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                handoff,
                decision,
                &mut executor,
                |_| -> Result<RecoveredOperationObservation, &'static str> {
                    child_recoveries.set(child_recoveries.get() + 1);
                    panic!("a selected stop cannot recover a child")
                },
                |_, _, _, _| -> ConsultationChildRetentionOutcome<u8, &'static str> {
                    retentions.set(retentions.get() + 1);
                    panic!("a selected stop cannot retain a child")
                },
            )
            .unwrap_or_else(|_| panic!("the exact handoff admits its selected stop"));
        let (started, disposition) = continued.into_parts();
        assert_eq!(started.selector(), &selector);
        assert_eq!(started.store(), &selected_store);
        assert!(started.children().is_empty());
        let ResidentConsultationTurnDisposition::Stopped(
            ResidentConsultationTurnStopped::Selected(selected),
        ) = disposition
        else {
            panic!("the application decision owner returns the selected stop")
        };
        let (current, stop) = selected.into_parts();
        assert_eq!(stop, 7);
        assert!(started.matches_collection(current.collection()));
        assert_eq!((child_recoveries.get(), retentions.get()), (0, 0));
        assert_eq!((executor.accepts, executor.polls), (0, 1));

        let exhausted = host
            .recover_one_root_consult_start(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                restart_from(&recovery),
                reborrow_admission(&mut admission),
                1 << 20,
                &mut authority,
                collection_limits(),
                task_limits(),
                UnixTimeV1::new(71).expect("later time"),
                &resolver,
                |_| Ok(()),
                &mut executor,
            )
            .unwrap_or_else(|_| panic!("the transferred source closes its frozen prefix"));
        assert!(matches!(
            exhausted,
            ResidentProcessConsultRecoveryTurn::Exhausted
        ));
        assert_eq!(custody.start_reads(), 1);
        assert_eq!((executor.accepts, executor.polls), (0, 1));
    });
}

struct ExactStopPolicyEvaluator {
    expected: ImmutableRecordReferenceV1,
    calls: usize,
    stop: u8,
}

struct UnavailablePolicyEvaluator;

impl ResidentConsultationPolicyEvaluator<u8> for UnavailablePolicyEvaluator {
    type Error = &'static str;

    fn evaluate(
        &mut self,
        _: &ImmutableRecordReferenceV1,
        _: &ResidentConsultationPolicyFrontier,
    ) -> Result<ResidentConsultationSupervisionDecision<u8>, Self::Error> {
        Err("selected Package policy is unavailable")
    }
}

impl ResidentConsultationPolicyEvaluator<u8> for ExactStopPolicyEvaluator {
    type Error = &'static str;

    fn evaluate(
        &mut self,
        completion_policy: &ImmutableRecordReferenceV1,
        _frontier: &ResidentConsultationPolicyFrontier,
    ) -> Result<ResidentConsultationSupervisionDecision<u8>, Self::Error> {
        if completion_policy != &self.expected {
            return Err("unexpected completion policy");
        }
        self.calls += 1;
        Ok(ResidentConsultationSupervisionDecision::Stop(self.stop))
    }
}

#[derive(Clone, Copy)]
enum PolicyRefusalCase {
    Selection,
    HandoffSelector,
    AccessStore,
    StartedStore,
    Collection,
}

fn assert_policy_refusal_before_callbacks(case: PolicyRefusalCase) {
    with_process_host_root_consult_policy_fixture(&mut move |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            mut admission,
            signing_authority,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let foreign_store = store(
            "foreign-policy-continuation-store",
            "foreign-policy-continuation-incarnation",
        );
        let selector = selector_from_signing_authority(signing_authority);
        let mut foreign_selector = selector.clone();
        foreign_selector.runtime_authority_ref =
            crate::local_operation_start_fixture::record_ref("foreign-policy-runtime");
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let mut executor = ProbeExecutor::default();
        let mut authority = CountingAuthority::default();
        let resolver = |_: &OperationObservationV1| Ok(selected_store.clone());

        let recovered = host
            .recover_one_root_consult_start(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                restart_from(&recovery),
                reborrow_admission(&mut admission),
                1 << 20,
                &mut authority,
                collection_limits(),
                task_limits(),
                UnixTimeV1::new(70).expect("policy time"),
                &resolver,
                |_| Ok(()),
                &mut executor,
            )
            .unwrap_or_else(|_| panic!("the durable root Consult reaches policy"));
        let ResidentProcessConsultRecoveryTurn::PolicyRequired(mut handoff) = recovered else {
            panic!("the durable root Consult must produce a policy handoff")
        };

        match case {
            PolicyRefusalCase::Selection => {}
            PolicyRefusalCase::HandoffSelector => {
                handoff.replace_started_selector_for_test(foreign_selector.clone());
            }
            PolicyRefusalCase::AccessStore => {}
            PolicyRefusalCase::StartedStore => {
                handoff.replace_started_store_for_test(foreign_store.clone());
            }
            PolicyRefusalCase::Collection => {
                let foreign_current = consultation_supervision_fixture_in_store(
                    "operation/foreign-policy-parent",
                    "foreign-policy-child",
                    100,
                    false,
                    selected_store.clone(),
                );
                let mut frontier_executor = ProbeExecutor::default();
                let mut foreign_tasks = task_state();
                let foreign_frontier = foreign_tasks
                    .supervise_recovered_consultation_pre_policy_turn::<
                        u8,
                        u16,
                        std::convert::Infallible,
                        _,
                    >(
                        task_limits(),
                        UnixTimeV1::new(70).expect("foreign policy time"),
                        foreign_current,
                        &mut frontier_executor,
                    );
                let ResidentConsultationPrePolicyTurn::PolicyRequired(frontier) = foreign_frontier
                else {
                    panic!("the foreign fixture reaches its policy frontier")
                };
                handoff.replace_frontier_for_test(frontier);
            }
        }

        let requested_selector = if matches!(case, PolicyRefusalCase::Selection) {
            &foreign_selector
        } else {
            &selector
        };
        let claimed_store = if matches!(case, PolicyRefusalCase::AccessStore) {
            &foreign_store
        } else {
            &selected_store
        };
        let child_recoveries = Cell::new(0);
        let retentions = Cell::new(0);
        let calls_before = (executor.accepts, executor.polls);
        let refused = match host.continue_recovered_consultation_policy(
            requested_selector,
            ResidentDeploymentOperationAccess {
                store: claimed_store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            handoff,
            ResidentConsultationSupervisionDecision::Stop(11_u8),
            &mut executor,
            |_| -> Result<RecoveredOperationObservation, &'static str> {
                child_recoveries.set(child_recoveries.get() + 1);
                panic!("association refusal cannot recover a child")
            },
            |_, _, _, _| -> ConsultationChildRetentionOutcome<u8, &'static str> {
                retentions.set(retentions.get() + 1);
                panic!("association refusal cannot retain a child")
            },
        ) {
            Err(refused) => refused,
            Ok(_) => panic!("the unlike policy continuation must be refused"),
        };
        let (handoff, decision, reason) = refused.into_parts();
        assert!(matches!(
            decision,
            ResidentConsultationSupervisionDecision::Stop(11)
        ));
        assert_eq!(
            handoff.frontier().observed_at(),
            UnixTimeV1::new(70).unwrap()
        );
        assert!(matches!(
            (case, reason),
            (
                PolicyRefusalCase::Selection,
                ResidentProcessConsultationPolicyRefusalReason::Selection(
                    ResidentProcessHostSelectionError::UnknownDeploymentAuthority,
                ),
            ) | (
                PolicyRefusalCase::HandoffSelector,
                ResidentProcessConsultationPolicyRefusalReason::HandoffSelectorAssociation,
            ) | (
                PolicyRefusalCase::AccessStore,
                ResidentProcessConsultationPolicyRefusalReason::AccessStoreAssociation,
            ) | (
                PolicyRefusalCase::StartedStore,
                ResidentProcessConsultationPolicyRefusalReason::StartedStoreAssociation,
            ) | (
                PolicyRefusalCase::Collection,
                ResidentProcessConsultationPolicyRefusalReason::CollectionAssociation,
            )
        ));
        assert_eq!((child_recoveries.get(), retentions.get()), (0, 0));
        assert_eq!((executor.accepts, executor.polls), calls_before);
    });
}

#[test]
fn cold_policy_continuation_refuses_every_unlike_association_before_io() {
    for case in [
        PolicyRefusalCase::Selection,
        PolicyRefusalCase::HandoffSelector,
        PolicyRefusalCase::AccessStore,
        PolicyRefusalCase::StartedStore,
        PolicyRefusalCase::Collection,
    ] {
        assert_policy_refusal_before_callbacks(case);
    }
}

fn recovered_child(
    operation: &OperationFixture,
    store: pale_ports::AuthorityStoreCoordinate,
    dispatched: bool,
) -> RecoveredOperationObservation {
    let (handle_ref, handle) = operation.handle();
    let selected = if dispatched {
        let reference = crate::local_operation_start_fixture::record_ref(
            "observation/consult-recovery-dispatched",
        );
        let observation = crate::local_operation_start_fixture::issue_observation(
            &crate::resident::operation_observation_custody::tests::signing_authority(),
            handle_ref,
            handle,
            crate::local_operation_start_fixture::schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
            OperationObservationRevisionV1::new(3).expect("dispatch revision"),
            Some(operation.successor()),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched {
                    attempt_id: CapabilityAttemptIdV1::parse("consult-recovery-attempt")
                        .expect("attempt identity"),
                    evidence: ImmutableRecordReferenceV1 {
                        record_ref: crate::local_operation_start_fixture::record_ref(
                            "attempt/consult-recovery",
                        ),
                        record_sha256: ArtifactDigest::hash_bytes(b"consult-recovery-attempt"),
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
    let handle = ControlRecordCoordinate::new(
        handle_ref.clone(),
        ControlRecordDigest::parse(handle.canonical_operation_handle_sha256().as_str())
            .expect("fixture handle coordinate"),
    );
    RecoveredOperationObservation::fixture_for_handle(
        handle,
        selected.clone(),
        selected,
        (
            crate::local_operation_start_fixture::record_ref("checkpoint/consult-recovery-child"),
            admit_initial_checkpoint(&crate::local_operation_start_fixture::checkpoint_preimage(
                1, None,
            )),
        ),
        store,
    )
}

#[test]
fn consult_and_root_provider_sweeps_keep_independent_cursors() {
    with_process_host_recovery_sweep_fixture(&mut |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            mut admission,
            signing_authority,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let mut executor = ProbeExecutor::default();
        let mut authority = CountingAuthority::default();
        let resolver = |_: &OperationObservationV1| Ok(selected_store.clone());

        let consult = match host.recover_one_root_consult_start(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            restart_from(&recovery),
            reborrow_admission(&mut admission),
            1 << 20,
            &mut authority,
            collection_limits(),
            task_limits(),
            UnixTimeV1::new(2).expect("turn time"),
            &resolver,
            |_| Ok(()),
            &mut executor,
        ) {
            Ok(turn) => turn,
            Err(_) => panic!("the admitted root Invoke is handed to its owner"),
        };
        assert!(matches!(
            consult,
            ResidentProcessConsultRecoveryTurn::SkippedOtherOwner(
                ResidentProcessConsultRecoveryOtherOwner::Invoke
            )
        ));
        assert_eq!((executor.accepts, executor.polls), (0, 0));

        let mut root_owner = RootOwner::default();
        let root = host
            .recover_one_root_provider_start(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                restart_from(&recovery),
                reborrow_admission(&mut admission),
                1 << 20,
                &mut root_owner,
            )
            .expect("the root-provider cursor still owns the same source");
        assert!(matches!(
            root,
            ResidentProcessOperationRecoveryTurn::Scheduled(())
        ));
        assert_eq!(root_owner.calls, 1);
        assert_eq!(custody.start_reads(), 2);

        let consult_exhausted = match host.recover_one_root_consult_start(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            restart_from(&recovery),
            reborrow_admission(&mut admission),
            1 << 20,
            &mut authority,
            collection_limits(),
            task_limits(),
            UnixTimeV1::new(2).expect("turn time"),
            &resolver,
            |_| Ok(()),
            &mut executor,
        ) {
            Ok(turn) => turn,
            Err(_) => panic!("the closed Consult cursor exhausts"),
        };
        assert!(matches!(
            consult_exhausted,
            ResidentProcessConsultRecoveryTurn::Exhausted
        ));
        assert_eq!(custody.start_reads(), 2);
    });
}

#[test]
fn retained_consult_start_error_retries_the_same_source_without_task_io() {
    let consult_cases = Rc::new(Cell::new(0));
    let observed_consult_cases = Rc::clone(&consult_cases);
    with_process_host_restart_fixtures(&mut move |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            mut admission,
            signing_authority,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let mut executor = ProbeExecutor::default();
        let mut authority = CountingAuthority::default();
        let resolver = |_: &OperationObservationV1| Ok(selected_store.clone());
        let first = host.recover_one_root_consult_start(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            restart_from(&recovery),
            reborrow_admission(&mut admission),
            1 << 20,
            &mut authority,
            collection_limits(),
            task_limits(),
            UnixTimeV1::new(2).expect("turn time"),
            &resolver,
            |_| Ok(()),
            &mut executor,
        );

        if matches!(first, Err(ResidentProcessConsultRecoveryError::Start(_))) {
            observed_consult_cases.set(observed_consult_cases.get() + 1);
            let reads = custody.start_reads();
            let second = host.recover_one_root_consult_start(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                restart_from(&recovery),
                reborrow_admission(&mut admission),
                1 << 20,
                &mut authority,
                collection_limits(),
                task_limits(),
                UnixTimeV1::new(2).expect("turn time"),
                &resolver,
                |_| Ok(()),
                &mut executor,
            );
            assert!(matches!(
                second,
                Err(ResidentProcessConsultRecoveryError::Start(_))
            ));
            assert_eq!(custody.start_reads(), reads + 1);
            assert_eq!((executor.accepts, executor.polls), (0, 0));
        }
    });
    assert_eq!(
        consult_cases.get(),
        1,
        "one retained Consult fixture retries"
    );
}

#[test]
fn consult_retention_admission_precedes_collection_and_executor_io() {
    with_process_host_root_consult_policy_fixture(&mut |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            mut admission,
            signing_authority,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let mut executor = ProbeExecutor::default();
        let mut authority = CountingAuthority::default();
        let resolver = |_: &OperationObservationV1| {
            panic!("a refused retention reservation must precede collection recovery")
        };

        let result = host.recover_one_root_consult_start_with_retention(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            restart_from(&recovery),
            reborrow_admission(&mut admission),
            1 << 20,
            &mut authority,
            collection_limits(),
            task_limits(),
            UnixTimeV1::new(2).expect("turn time"),
            &resolver,
            |_| panic!("a refused retention reservation must precede collection admission"),
            None,
            |_| Err(()),
            &mut executor,
        );

        assert!(matches!(
            result,
            Err(ResidentProcessConsultRecoveryError::RetentionUnavailable)
        ));
        assert_eq!((executor.accepts, executor.polls), (0, 0));
    });
}

#[test]
fn pre_policy_projection_polls_once_and_never_selects_policy() {
    with_process_host_recovery_sweep_fixture(&mut |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            signing_authority,
            generation,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let current = consultation_supervision_fixture_in_store(
            "operation/consult-recovery-parent",
            "consult-recovery-child",
            100,
            false,
            selected_store,
        );
        let mut executor = ProbeExecutor::default();

        let turn = host
            .operations
            .supervise_recovered_consultation_pre_policy_turn::<
                u8,
                u16,
                std::convert::Infallible,
                _,
            >(
            task_limits(),
            UnixTimeV1::new(2).expect("turn time"),
            current,
            &mut executor,
        );
        assert!(matches!(
            turn,
            ResidentConsultationPrePolicyTurn::PolicyRequired(_)
        ));
        assert_eq!((executor.accepts, executor.polls), (0, 1));
    });
}

#[test]
fn established_stop_bypasses_policy_after_one_empty_poll() {
    with_process_host_recovery_sweep_fixture(&mut |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            signing_authority,
            generation,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let current = consultation_supervision_fixture_in_store(
            "operation/consult-recovery-stopped",
            "consult-recovery-child",
            10,
            true,
            selected_store,
        );
        let mut executor = ProbeExecutor::default();

        let turn = host
            .operations
            .supervise_recovered_consultation_pre_policy_turn::<
                u8,
                u16,
                std::convert::Infallible,
                _,
            >(
            task_limits(),
            UnixTimeV1::new(11).expect("post-stop time"),
            current,
            &mut executor,
        );
        let ResidentConsultationPrePolicyTurn::Stopped(_stop) = turn else {
            panic!("the durable stop remains established")
        };
        assert_eq!((executor.accepts, executor.polls), (0, 1));
    });
}

#[test]
fn mechanical_owner_does_not_poll_when_planning_rejects_the_collection() {
    with_process_host_recovery_sweep_fixture(&mut |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            signing_authority,
            generation,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let foreign_store = pale_ports::AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("foreign-consult-recovery-store").expect("store"),
            ControlIncarnationIdV1::parse("foreign-consult-recovery-incarnation")
                .expect("incarnation"),
        );
        let foreign = consultation_supervision_fixture_in_store(
            "operation/foreign-consult-recovery",
            "foreign-consult-recovery-child",
            10,
            false,
            foreign_store,
        );
        let mut executor = ProbeExecutor::default();
        let first = host
            .operations
            .supervise_recovered_consultation_pre_policy_turn::<
                u8,
                u16,
                std::convert::Infallible,
                _,
            >(
                task_limits(),
                UnixTimeV1::new(0).expect("time before collection open"),
                foreign,
                &mut executor,
            );
        assert!(matches!(
            first,
            ResidentConsultationPrePolicyTurn::NeedsRecovery(_)
        ));

        let selected = consultation_supervision_fixture_in_store(
            "operation/selected-consult-recovery",
            "selected-consult-recovery-child",
            10,
            false,
            selected_store,
        );
        let second = host
            .operations
            .supervise_recovered_consultation_pre_policy_turn::<
                u8,
                u16,
                std::convert::Infallible,
                _,
            >(
                task_limits(),
                UnixTimeV1::new(0).expect("time before collection open"),
                selected,
                &mut executor,
            );
        assert!(matches!(
            second,
            ResidentConsultationPrePolicyTurn::NeedsRecovery(_)
        ));
        assert_eq!((executor.accepts, executor.polls), (0, 0));
    });
}

#[test]
fn mechanical_needs_recovery_retries_the_same_discovered_source() {
    with_process_host_recovery_sweep_fixture(&mut |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            signing_authority,
            generation,
            custody,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let initial = host.consult_recovery_attempts[0].clone();
        let discovered = discover_local_operation_start(initial.clone(), &mut *custody, 1 << 20)
            .expect("one committed start is physically discoverable");
        let LocalOperationStartDiscoveryTurn::Pending(pending) = discovered else {
            panic!("the first Consult recovery source must be pending")
        };
        let source_handle = pending.source().session_basis().handle().clone();
        let successor = pending.advance();
        let current = consultation_supervision_fixture_in_store(
            "operation/consult-recovery-needs-repair",
            "consult-recovery-child",
            100,
            false,
            selected_store,
        );
        let mut executor = ProbeExecutor::default();
        let turn = host
            .operations
            .supervise_recovered_consultation_pre_policy_turn::<
                u8,
                u16,
                std::convert::Infallible,
                _,
            >(
            task_limits(),
            UnixTimeV1::new(0).expect("pre-collection time"),
            current,
            &mut executor,
        );
        assert!(matches!(
            turn,
            ResidentConsultationPrePolicyTurn::NeedsRecovery(ref reason)
                if matches!(
                    reason.as_ref(),
                    ResidentConsultationPrePolicyNeedsRecovery::Planning(_)
                )
        ));
        let retained = host.finish_consult_recovery_turn::<u8, u16>(
            0,
            successor,
            ResidentProcessConsultRecoveryTurn::NeedsRecovery(match turn {
                ResidentConsultationPrePolicyTurn::NeedsRecovery(reason) => reason,
                _ => unreachable!("the asserted recovery reason remains owned"),
            }),
        );
        assert!(matches!(
            retained,
            ResidentProcessConsultRecoveryTurn::NeedsRecovery(_)
        ));
        assert_eq!(host.consult_recovery_attempts[0], initial);

        let retry = discover_local_operation_start(
            host.consult_recovery_attempts[0].clone(),
            &mut *custody,
            1 << 20,
        )
        .expect("repairable work remains selected");
        let LocalOperationStartDiscoveryTurn::Pending(retry) = retry else {
            panic!("mechanical recovery must retry the same source")
        };
        assert_eq!(retry.source().session_basis().handle(), &source_handle);
        assert_eq!((executor.accepts, executor.polls), (0, 0));
    });
}

#[test]
fn completion_handoff_advances_once_and_prevents_a_second_poll() {
    with_process_host_recovery_sweep_fixture(&mut |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            signing_authority,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let discovered = discover_local_operation_start(
            host.consult_recovery_attempts[0].clone(),
            &mut *custody,
            1 << 20,
        )
        .expect("one committed start is physically discoverable");
        let LocalOperationStartDiscoveryTurn::Pending(pending) = discovered else {
            panic!("the first Consult recovery source must be pending")
        };
        let successor = pending.advance();

        let child = OperationFixture::exact();
        let current = consultation_supervision_fixture_with_child_in_store(
            "operation/consult-recovery-completion",
            "consult-recovery-child",
            ImmutableRecordReferenceV1 {
                record_ref: child.handle().0.clone(),
                record_sha256: ArtifactDigest::parse(
                    child
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .as_str(),
                )
                .expect("child handle digest"),
            },
            100,
            false,
            selected_store.clone(),
        );
        let planned = current
            .plan_child_dispatch(UnixTimeV1::new(2).expect("dispatch time"))
            .expect("one exact child batch");
        let candidate = planned.candidates()[0].clone();
        let initial_child = recovered_child(&child, selected_store.clone(), false);
        let retained_child = recovered_child(&child, selected_store.clone(), true);
        let mut executor = ReadyExecutor::default();
        let mut authority = CountingAuthority::default();
        with_unproved_operation_service_for_test(
            &mut host,
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            &mut authority,
            |_store, service| {
                let coordinate = service
                    .open_consultation_batch(planned, task_limits())
                    .expect("one exact process-local batch");
                service
                    .retain_and_accept_consultation_task(
                        &coordinate,
                        &candidate,
                        &FixedClock(UnixTimeV1::new(2).expect("dispatch time")),
                        || Ok::<_, std::convert::Infallible>(current.clone()),
                        |_| Ok::<_, std::convert::Infallible>(initial_child),
                        |_, _| {
                            ConsultationChildRetentionOutcome::<
                            u8,
                            std::convert::Infallible,
                        >::Retained {
                            child: Box::new(retained_child),
                            work: 7,
                        }
                        },
                        &mut executor,
                    )
                    .expect("the child reaches its executor once");
            },
        )
        .expect("the exact deployment supplies its task owner");
        executor.complete_first(70);

        let turn = host
            .operations
            .supervise_recovered_consultation_pre_policy_turn::<
                u8,
                u16,
                std::convert::Infallible,
                _,
            >(
            task_limits(),
            UnixTimeV1::new(3).expect("completion time"),
            current,
            &mut executor,
        );
        assert_eq!((executor.accepts, executor.polls), (1, 1));
        let ResidentConsultationPrePolicyTurn::CompletionReady(ready) = turn else {
            panic!("the existing join owner must receive the exact completion")
        };
        let mut foreign_selector = selector.clone();
        foreign_selector.runtime_authority_ref =
            crate::local_operation_start_fixture::record_ref("foreign-completion-runtime");
        let failure = match host.bind_consultation_completion(foreign_selector.clone(), *ready) {
            Ok(_) => panic!("a foreign selector cannot consume the exact completion"),
            Err(failure) => failure,
        };
        let retained = ResidentProgramConsultTurn::<
            u8,
            u16,
            std::convert::Infallible,
            std::convert::Infallible,
            (),
        >::CompletionBinding(failure);
        let ResidentProgramConsultTurn::CompletionBinding(failure) = retained else {
            unreachable!("the selector refusal retains the exact completion")
        };
        assert_eq!(failure.selector, foreign_selector);
        let ready = match host.bind_consultation_completion(selector.clone(), *failure.ready) {
            Ok(ready) => ready,
            Err(_) => panic!("the retained completion binds to its admitted deployment"),
        };
        let handed_off = host.finish_consult_recovery_turn::<u8, u16>(
            0,
            successor,
            ResidentProcessConsultRecoveryTurn::Progress(Box::new(ready)),
        );
        let ResidentProcessConsultRecoveryTurn::Progress(ready) = handed_off else {
            panic!("the existing join owner must receive the exact completion")
        };
        let ResidentProcessConsultationCompletionReady {
            selector: ready_selector,
            ready,
        } = *ready;
        assert_eq!(ready_selector, selector);
        let (_current, _poll) = ready.into_parts();

        let next = discover_local_operation_start(
            host.consult_recovery_attempts[0].clone(),
            &mut *custody,
            1 << 20,
        )
        .expect("the transferred source closes its frozen prefix");
        assert!(matches!(
            next,
            LocalOperationStartDiscoveryTurn::Exhausted(_)
        ));
        assert_eq!(executor.polls, 1, "the completion is not polled twice");
    });
}
