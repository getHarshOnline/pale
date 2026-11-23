//! Bounded cold-start discovery and phase dispatch through one process host.

use super::restart::{host_with_generation, restart_input};
use super::*;
use crate::local_operation_start_fixture::signature;
use crate::resident::consultation::tests::with_issued_delegate_child_named;
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_process_host_local_cancellation_restart_fixtures,
    with_process_host_operation_resolution_restart_fixtures,
    with_process_host_prepared_restart_fixtures,
    with_process_host_provider_resolved_restart_fixture, with_process_host_recovery_sweep_fixture,
    with_process_host_restart_dispatch_fixtures, ProcessHostRestartFixture,
};
use crate::resident::process_host::delegate_recovery_sweep::*;
use crate::resident::process_host::operation_recovery_sweep::*;
use crate::resident::process_host::restart::{
    ResidentProcessRecoveredA2aCancelTaskRequest, ResidentProcessRecoveredA2aTask,
    ResidentProcessRecoveredAttempt, ResidentProcessRecoveredLocalCancellation,
    ResidentProcessRecoveredOperationResolution, ResidentProcessRecoveredPreparedAttempt,
    ResidentProcessRecoveredProviderResolution, ResidentProcessRestartReadLimits,
};
use crate::resident::{
    LocalOperationHandleRecoveryInput, OperationCheckpointAncestryLimits,
    OperationControlRecoveryAdmission, OperationControlRecoveryInput,
    OperationObservationRecoveryLimits, ResidentServiceA2aCancelRecoveryInputs,
    ResidentServiceA2aDelegateLaunchInputs, ResidentServiceA2aPreTaskRecoveryInputs,
    ResidentServiceA2aTaskObservationInputs, ResidentServiceDelegateInputPort,
    ResidentServiceDeploymentSelector,
};
use crate::session_admission_fixture::ProvingAuthority;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveredPhase {
    LocalCancellation,
    OperationResolution,
    ProviderResolved,
    ProviderPrepared,
    ProviderDispatched,
    NativePrepared,
    NativeDispatched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObservedSweepTurn {
    Scheduled(RecoveredPhase),
    Deferred(ResidentProcessOperationRecoveryDeferred),
    Terminal,
    RecoveryOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObservedDelegateRoute {
    Scheduled(RecoveredPhase),
    DeferredResolution,
    Terminal,
    RecoveryOnly,
    ConsultRejected,
    PairwiseRequired,
}

#[derive(Debug, Clone, Copy)]
enum HostileSource {
    Store,
    Basis,
    Request,
    Lease,
}

struct InitialDelegateLaunchProbe(Rc<Cell<usize>>);

impl<'service> ResidentServiceDelegateInputPort<'service> for InitialDelegateLaunchProbe {
    type Authority = ProvingAuthority;

    fn with_initial_launch_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        locator: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs:
            FnOnce(ResidentServiceA2aDelegateLaunchInputs<'service, Self::Authority>) -> Output,
    {
        assert!(!locator.operation_handle_record_ref().is_empty());
        assert!(!locator.operation_handle_sha256().is_empty());
        self.0.set(self.0.get() + 1);
        Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_pre_task_recovery_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs:
            FnOnce(ResidentServiceA2aPreTaskRecoveryInputs<'service, Self::Authority>) -> Output,
    {
        panic!("an operation-resolution phase cannot select pre-Task recovery")
    }

    fn with_task_observation_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs:
            FnOnce(ResidentServiceA2aTaskObservationInputs<'service, Self::Authority>) -> Output,
    {
        panic!("an operation-resolution phase cannot select Task observation")
    }

    fn with_cancel_recovery_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(ResidentServiceA2aCancelRecoveryInputs<'service>) -> Output,
    {
        panic!("an operation-resolution phase cannot select CancelTask recovery")
    }
}

#[derive(Default)]
struct RecordingRecoveryOwner {
    calls: Vec<RecoveredPhase>,
    fail_next: bool,
}

impl RecordingRecoveryOwner {
    fn finish(&mut self, phase: RecoveredPhase) -> Result<RecoveredPhase, ()> {
        self.calls.push(phase);
        if std::mem::take(&mut self.fail_next) {
            Err(())
        } else {
            Ok(phase)
        }
    }
}

impl ResidentProcessRootProviderRecoveryOwner<UnprovedHeldWriter> for RecordingRecoveryOwner {
    type Output = RecoveredPhase;
    type Error = ();

    fn complete_local_cancellation<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredLocalCancellation>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.finish(RecoveredPhase::LocalCancellation)
    }

    fn recover_operation_resolution<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredOperationResolution>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<ResidentProcessRootProviderResolution<Self::Output>, Self::Error> {
        self.finish(RecoveredPhase::OperationResolution)
            .map(ResidentProcessRootProviderResolution::Provider)
    }

    fn recover_provider_resolved<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredProviderResolution>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.finish(RecoveredPhase::ProviderResolved)
    }

    fn recover_provider_prepared<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredPreparedAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.finish(RecoveredPhase::ProviderPrepared)
    }

    fn recover_provider_dispatched<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.finish(RecoveredPhase::ProviderDispatched)
    }

    fn recover_native_prepared<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredPreparedAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.finish(RecoveredPhase::NativePrepared)
    }

    fn recover_native_dispatched<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.finish(RecoveredPhase::NativeDispatched)
    }
}

impl ResidentProcessDelegateRecoveryOwner<UnprovedHeldWriter> for RecordingRecoveryOwner {
    fn recover_delegate_operation_resolution<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredOperationResolution>,
        _: ResidentDeploymentOperationAccess<'_>,
        input: crate::resident::process_host::program::ResidentProgramDelegateResolutionInput<
            '_,
            '_,
        >,
    ) -> Result<Self::Output, Self::Error> {
        assert!(input.maximum_selection_bytes() > 0);
        assert!(input.maximum_record_bytes() > 0);
        self.finish(RecoveredPhase::OperationResolution)
    }

    fn recover_a2a_pre_task<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        panic!("resolving Delegate fixture cannot become an A2A send")
    }

    fn recover_a2a_task<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredA2aTask>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        panic!("resolving Delegate fixture cannot become an A2A Task")
    }

    fn recover_a2a_cancel_task<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredA2aCancelTaskRequest>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        panic!("resolving Delegate fixture cannot become A2A cancellation")
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

fn restart_inputs(
    recovery: OperationControlRecoveryInput<'_>,
) -> (
    ResidentProcessOperationRestartInput<'_>,
    ResidentProcessOperationRestartInput<'_>,
    ResidentProcessOperationRestartInput<'_>,
) {
    let OperationControlRecoveryInput {
        operation,
        observation,
        observation_limits,
        checkpoint_limits,
        membership_limits,
    } = recovery;
    let LocalOperationHandleRecoveryInput {
        context,
        expected_handle,
        maximum_parent_depth,
    } = operation;
    let make = || {
        restart_input(
            OperationControlRecoveryInput {
                operation: LocalOperationHandleRecoveryInput {
                    context,
                    expected_handle,
                    maximum_parent_depth,
                },
                observation,
                observation_limits,
                checkpoint_limits,
                membership_limits,
            },
            1024 * 1024,
        )
    };
    (make(), make(), make())
}

fn run_one(
    fixture: ProcessHostRestartFixture<'_>,
    group: &'static str,
) -> (ObservedSweepTurn, usize, usize) {
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
    let (first_restart, second_restart, _) = restart_inputs(recovery);
    let mut owner = RecordingRecoveryOwner::default();
    let first = host
        .recover_one_root_provider_start(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            first_restart,
            reborrow_admission(&mut admission),
            1024 * 1024,
            &mut owner,
        )
        .unwrap_or_else(|error| panic!("{group} source must classify: {error:?}"));
    let selected = match first {
        ResidentProcessOperationRecoveryTurn::Scheduled(phase) => {
            ObservedSweepTurn::Scheduled(phase)
        }
        ResidentProcessOperationRecoveryTurn::DeferredUnsupported { owner, .. } => {
            ObservedSweepTurn::Deferred(owner)
        }
        ResidentProcessOperationRecoveryTurn::ReportedTerminal => ObservedSweepTurn::Terminal,
        ResidentProcessOperationRecoveryTurn::ReportedRecoveryOnly(_) => {
            ObservedSweepTurn::RecoveryOnly
        }
        ResidentProcessOperationRecoveryTurn::Exhausted => {
            panic!("fixture retains one committed start")
        }
    };
    let reads_after_first = custody.start_reads();
    let calls_after_first = owner.calls.len();
    let second = host
        .recover_one_root_provider_start(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            second_restart,
            reborrow_admission(&mut admission),
            1024 * 1024,
            &mut owner,
        )
        .unwrap_or_else(|error| panic!("{group} closed source must exhaust: {error:?}"));
    let keeps_source = matches!(
        selected,
        ObservedSweepTurn::Deferred(
            ResidentProcessOperationRecoveryDeferred::A2a
                | ResidentProcessOperationRecoveryDeferred::Native
        )
    );
    if keeps_source {
        assert!(matches!(
            second,
            ResidentProcessOperationRecoveryTurn::ReportedRecoveryOnly(_)
                | ResidentProcessOperationRecoveryTurn::DeferredUnsupported {
                    owner: ResidentProcessOperationRecoveryDeferred::A2a
                        | ResidentProcessOperationRecoveryDeferred::Native,
                    ..
                }
        ));
        assert!(custody.start_reads() > reads_after_first);
    } else {
        assert!(matches!(
            second,
            ResidentProcessOperationRecoveryTurn::Exhausted
        ));
        assert_eq!(custody.start_reads(), reads_after_first);
    }
    assert_eq!(owner.calls.len(), calls_after_first);
    (selected, reads_after_first, calls_after_first)
}

fn route_one_delegate_disposition(
    fixture: ProcessHostRestartFixture<'_>,
    group: &'static str,
) -> (ObservedDelegateRoute, Vec<RecoveredPhase>) {
    let ProcessHostRestartFixture {
        recovery,
        mut admission,
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
    let mut host = host_with_generation(&port, &selector, &selected_store, generation);
    let (restart, _, _) = restart_inputs(recovery);
    let association = ResidentOperationControlAssociation {
        current_request_authority: signing_authority,
        historical_handle_authority: restart.operation.handle_signing_authority,
        recovery_store: restart.operation.store,
        checkpoint_authority: restart.operation.checkpoint_authority,
    };
    let disposition = host
        .classify_existing_operation(
            ResidentProcessExistingOperationStart {
                selector: selector.clone(),
                evidence: Box::new(evidence),
            },
            association,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            restart,
            reborrow_admission(&mut admission),
        )
        .unwrap_or_else(|error| panic!("{group} must classify before routing: {error:?}"));
    let mut owner = RecordingRecoveryOwner::default();
    let routed = host.route_delegate_restart_disposition(
        disposition,
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut *custody,
            sessions,
            anchor: &mut *anchor,
        },
        &mut owner,
    );
    let observed = match routed {
        Ok(ResidentProcessDelegateRecoveryTurn::Scheduled(phase)) => {
            ObservedDelegateRoute::Scheduled(phase)
        }
        Ok(ResidentProcessDelegateRecoveryTurn::ReportedTerminal(_)) => {
            ObservedDelegateRoute::Terminal
        }
        Ok(ResidentProcessDelegateRecoveryTurn::ReportedRecoveryOnly(_)) => {
            ObservedDelegateRoute::RecoveryOnly
        }
        Ok(ResidentProcessDelegateRecoveryTurn::DeferredResolution(_)) => {
            ObservedDelegateRoute::DeferredResolution
        }
        Err(ResidentProcessDelegateRecoveryError::State(
            ResidentProcessDelegateRecoveryStateError::ConsultDisposition,
        )) => ObservedDelegateRoute::ConsultRejected,
        Err(ResidentProcessDelegateRecoveryError::State(
            ResidentProcessDelegateRecoveryStateError::PairwiseLineage,
        )) => ObservedDelegateRoute::PairwiseRequired,
        Ok(ResidentProcessDelegateRecoveryTurn::Exhausted)
        | Ok(ResidentProcessDelegateRecoveryTurn::SkippedOtherOwner(_)) => {
            panic!("{group} route helper received no classified disposition")
        }
        Err(error) => panic!("{group} failed during direct Delegate routing: {error:?}"),
    };
    (observed, owner.calls)
}

fn reject_hostile_source(fixture: ProcessHostRestartFixture<'_>, hostile: HostileSource) {
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
    let foreign_store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("foreign-recovery-store").unwrap(),
        ControlIncarnationIdV1::parse("foreign-recovery-incarnation").unwrap(),
    );
    match hostile {
        HostileSource::Store => {}
        HostileSource::Basis => custody.substitute_discovered_session_basis(),
        HostileSource::Request => custody.remove_discovered_request(),
        HostileSource::Lease => custody.remove_operation_lease(),
    }
    let access_store = match hostile {
        HostileSource::Store => &foreign_store,
        _ => &selected_store,
    };
    let selector = selector_from_signing_authority(signing_authority);
    let port = MemoryAuthorityStore::default();
    let mut host = host_with_generation(&port, &selector, &selected_store, generation);
    let (restart, _, _) = restart_inputs(recovery);
    let mut owner = RecordingRecoveryOwner::default();
    let result = host.recover_one_root_provider_start(
        &selector,
        ResidentDeploymentOperationAccess {
            store: access_store,
            custody: &mut *custody,
            sessions,
            anchor: &mut *anchor,
        },
        restart,
        reborrow_admission(&mut admission),
        1024 * 1024,
        &mut owner,
    );
    match hostile {
        HostileSource::Store => assert!(matches!(
            result,
            Err(ResidentProcessOperationRecoverySweepError::Restart(
                ResidentProcessOperationRestartError::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation
                )
            ))
        )),
        HostileSource::Basis | HostileSource::Request => assert!(matches!(
            result,
            Err(ResidentProcessOperationRecoverySweepError::Source(
                ResidentProcessOperationRecoverySourceError::Association
            ))
        )),
        HostileSource::Lease => assert!(matches!(
            result,
            Err(ResidentProcessOperationRecoverySweepError::Restart(_))
        )),
    }
    assert!(owner.calls.is_empty(), "{hostile:?} reached a phase owner");
    assert_eq!(
        custody.start_reads(),
        usize::from(!matches!(hostile, HostileSource::Store)),
        "{hostile:?} crossed an unexpected discovery boundary"
    );
}

#[test]
fn supported_root_provider_phases_dispatch_once_and_do_not_replay() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let resolution = Rc::clone(&observed);
    let resolution_case = Rc::new(Cell::new(0));
    let counted_resolution = Rc::clone(&resolution_case);
    with_process_host_operation_resolution_restart_fixtures(&mut move |fixture| {
        let ordinal = counted_resolution.replace(counted_resolution.get() + 1);
        if ordinal == 3 {
            // The source fixture deliberately combines a self-contained lease
            // with `ContextBound`; the restart owner must reject that state.
            return;
        }
        resolution
            .borrow_mut()
            .push(run_one(fixture, "operation-resolution"));
    });
    assert_eq!(resolution_case.get(), 4);
    let provider_resolved = Rc::clone(&observed);
    with_process_host_provider_resolved_restart_fixture(&mut move |fixture| {
        provider_resolved
            .borrow_mut()
            .push(run_one(fixture, "provider-resolved"));
    });
    let prepared = Rc::clone(&observed);
    with_process_host_prepared_restart_fixtures(&mut move |fixture| {
        prepared
            .borrow_mut()
            .push(run_one(fixture, "dispatch-prepared"));
    });
    let dispatched = Rc::clone(&observed);
    with_process_host_restart_dispatch_fixtures(&mut move |fixture| {
        dispatched
            .borrow_mut()
            .push(run_one(fixture, "provider-dispatched"));
    });
    let observed = observed.borrow();
    assert!(observed.contains(&(
        ObservedSweepTurn::Scheduled(RecoveredPhase::OperationResolution),
        1,
        1
    )));
    assert!(observed.contains(&(
        ObservedSweepTurn::Scheduled(RecoveredPhase::ProviderResolved),
        1,
        1
    )));
    assert!(observed.contains(&(
        ObservedSweepTurn::Scheduled(RecoveredPhase::ProviderPrepared),
        1,
        1
    )));
    assert!(observed.contains(&(
        ObservedSweepTurn::Scheduled(RecoveredPhase::ProviderDispatched),
        1,
        1
    )));
    assert!(
        observed.iter().any(|(turn, reads, calls)| {
            matches!(
                turn,
                ObservedSweepTurn::Scheduled(
                    RecoveredPhase::NativePrepared | RecoveredPhase::NativeDispatched
                )
            ) && *reads == 1
                && *calls == 1
        }),
        "observed root recovery turns: {observed:?}"
    );
}

#[test]
fn retained_predispatch_cancellation_uses_only_the_local_owner_once() {
    let count = Rc::new(Cell::new(0));
    let completed = Rc::clone(&count);
    with_process_host_local_cancellation_restart_fixtures(&mut move |fixture| {
        let result = run_one(fixture, "local-cancellation");
        assert_eq!(
            result,
            (
                ObservedSweepTurn::Scheduled(RecoveredPhase::LocalCancellation),
                1,
                1
            )
        );
        completed.set(completed.get() + 1);
    });
    assert_eq!(count.get(), 4);
}

#[test]
fn delegate_router_reports_closed_states_and_hands_native_work_to_its_owner() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let reported = Rc::clone(&observed);
    crate::resident::operation_observation_custody::tests::local_recovery::with_process_host_restart_fixtures(
        &mut move |fixture| {
            reported
                .borrow_mut()
                .push(route_one_delegate_disposition(fixture, "closed Delegate route"));
        },
    );
    let prepared = Rc::clone(&observed);
    with_process_host_prepared_restart_fixtures(&mut move |fixture| {
        prepared.borrow_mut().push(route_one_delegate_disposition(
            fixture,
            "prepared Delegate route",
        ));
    });
    let dispatched = Rc::clone(&observed);
    with_process_host_restart_dispatch_fixtures(&mut move |fixture| {
        dispatched.borrow_mut().push(route_one_delegate_disposition(
            fixture,
            "dispatched Delegate route",
        ));
    });

    let observed = observed.borrow();
    assert!(observed.contains(&(ObservedDelegateRoute::Terminal, Vec::new())));
    assert!(observed.contains(&(ObservedDelegateRoute::RecoveryOnly, Vec::new())));
    assert!(observed.contains(&(ObservedDelegateRoute::ConsultRejected, Vec::new())));
    assert!(observed.contains(&(
        ObservedDelegateRoute::Scheduled(RecoveredPhase::NativePrepared),
        vec![RecoveredPhase::NativePrepared]
    )));
    assert!(observed.contains(&(
        ObservedDelegateRoute::Scheduled(RecoveredPhase::NativeDispatched),
        vec![RecoveredPhase::NativeDispatched]
    )));
    assert!(observed.contains(&(
        ObservedDelegateRoute::Scheduled(RecoveredPhase::ProviderPrepared),
        vec![RecoveredPhase::ProviderPrepared]
    )));
    assert!(observed.contains(&(
        ObservedDelegateRoute::Scheduled(RecoveredPhase::ProviderDispatched),
        vec![RecoveredPhase::ProviderDispatched]
    )));
}

#[test]
fn delegate_router_sends_predispatch_cancellation_only_to_the_local_owner() {
    let count = Rc::new(Cell::new(0));
    let routed = Rc::clone(&count);
    with_process_host_local_cancellation_restart_fixtures(&mut move |fixture| {
        let result = route_one_delegate_disposition(fixture, "local Delegate cancellation");
        assert_eq!(
            result,
            (
                ObservedDelegateRoute::Scheduled(RecoveredPhase::LocalCancellation),
                vec![RecoveredPhase::LocalCancellation]
            )
        );
        routed.set(routed.get() + 1);
    });
    assert_eq!(count.get(), 4);
}

#[test]
fn owner_error_keeps_the_source_selected_until_one_closed_retry() {
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
        let (first_restart, retry_restart, exhausted_restart) = restart_inputs(recovery);
        let mut owner = RecordingRecoveryOwner {
            fail_next: true,
            ..RecordingRecoveryOwner::default()
        };

        let first = host.recover_one_root_provider_start(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            first_restart,
            reborrow_admission(&mut admission),
            1024 * 1024,
            &mut owner,
        );
        assert!(matches!(
            first,
            Err(ResidentProcessOperationRecoverySweepError::Route(()))
        ));
        assert_eq!(custody.start_reads(), 1);
        assert_eq!(owner.calls, [RecoveredPhase::OperationResolution]);

        let retry = host
            .recover_one_root_provider_start(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                retry_restart,
                reborrow_admission(&mut admission),
                1024 * 1024,
                &mut owner,
            )
            .expect("the same source remains selected after an owner error");
        assert!(matches!(
            retry,
            ResidentProcessOperationRecoveryTurn::Scheduled(RecoveredPhase::OperationResolution)
        ));
        assert_eq!(custody.start_reads(), 2);
        assert_eq!(
            owner.calls,
            [
                RecoveredPhase::OperationResolution,
                RecoveredPhase::OperationResolution
            ]
        );

        let exhausted = host
            .recover_one_root_provider_start(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                exhausted_restart,
                reborrow_admission(&mut admission),
                1024 * 1024,
                &mut owner,
            )
            .expect("a closed retry exhausts the frozen prefix");
        assert!(matches!(
            exhausted,
            ResidentProcessOperationRecoveryTurn::Exhausted
        ));
        assert_eq!(custody.start_reads(), 2);
        assert_eq!(owner.calls.len(), 2);
    });
}

#[test]
fn terminal_recovery_only_and_consult_sources_are_reported_without_scheduling() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let collected = Rc::clone(&observed);
    crate::resident::operation_observation_custody::tests::local_recovery::with_process_host_restart_fixtures(
        &mut move |fixture| collected.borrow_mut().push(run_one(fixture, "reported")),
    );
    let observed = observed.borrow();
    assert!(observed.contains(&(ObservedSweepTurn::Terminal, 1, 0)));
    assert!(observed.contains(&(ObservedSweepTurn::RecoveryOnly, 1, 0)));
    assert!(observed.contains(&(
        ObservedSweepTurn::Deferred(ResidentProcessOperationRecoveryDeferred::Consult),
        1,
        0
    )));
}

#[test]
fn hostile_store_basis_request_and_lease_fail_before_phase_dispatch() {
    for hostile in [
        HostileSource::Store,
        HostileSource::Basis,
        HostileSource::Request,
        HostileSource::Lease,
    ] {
        with_process_host_recovery_sweep_fixture(&mut move |fixture| {
            reject_hostile_source(fixture, hostile)
        });
    }
}

#[test]
fn committed_delegate_defers_initial_launch_and_retries_exact_parent_admission() {
    let (mut generations, profile) =
        crate::resident::operation_observation_custody::tests::local_start::
            host_source_generation_copies(2);
    let restart_generation = generations.pop().expect("restart generation copy");
    let generation = generations.pop().expect("source generation copy");
    with_issued_delegate_child_named(
        &generation,
        &profile,
        "root-sweep-delegate-deferral",
        |child, parent, parent_candidate, parent_checkpoint, lease_custody, _, parents| {
            let store = child.initial_candidate.store().clone();
            let selector = selector_from_signing_authority(parents.signing);
            let (sessions, resolver) = super::restart::cold_restart_sessions(parents, &store);
            let checkpoint_authority = super::a2a_cancel_task::checkpoint_signer(&child.checkpoint);
            let port = MemoryAuthorityStore::default();
            let mut host = host_with_generation(&port, &selector, &store, restart_generation);
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
                    Rc::new(RefCell::new(None)),
                    |custody, anchor| {
                        let checkpoint_profile = local_operation_start_fixture::profile();
                        let observation_limits = OperationObservationRecoveryLimits::new(
                            16,
                            1 << 20,
                            1 << 20,
                        )
                        .expect("bounded observation recovery");
                        let checkpoint_limits = OperationCheckpointAncestryLimits::new(
                            16,
                            1 << 20,
                            16,
                        )
                        .expect("bounded checkpoint recovery");
                        let membership_limits = pale_ports::AccumulatorMembershipLimits::new(
                            64,
                            100_000,
                            1_000_000,
                        )
                        .expect("bounded membership recovery");
                        let recovery_context = LocalOperationRecoveryContext {
                            store: &store,
                            profile: &checkpoint_profile,
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
                        let restart = || ResidentProcessOperationRestartInput {
                            committed_observation_limits: None,
                            operation: recovery_context,
                            maximum_parent_depth: 1,
                            current_head: ResidentProcessRestartReadLimits {
                                observation: observation_limits,
                                checkpoints: checkpoint_limits,
                                membership: membership_limits,
                            },
                            maximum_selection_bytes: 1 << 20,
                            a2a_task: None,
                            a2a_cancel_task: None,
                        };
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
                                        invariant: "known committed recovery request",
                                    });
                                };
                                let preimage = expected.to_preimage();
                                CommonRequestEnvelopeV1::admit_contextual_wire(
                                    wire,
                                    &super::restart::request_admission_context(&preimage),
                                    |_, _, actual| {
                                        assert_eq!(actual, expected.caller_signature());
                                        Ok(())
                                    },
                                )
                            };
                        let reject_child_parent_once = Cell::new(true);
                        let child_parent_attempts = Cell::new(0);
                        let mut admit_handle_authority =
                            |preimage: &OperationHandlePreimageV1,
                             recovered_parent: Option<&OperationHandleV1>| {
                                if preimage.operation_handle_id
                                    == *child_handle.operation_handle_id()
                                {
                                    child_parent_attempts.set(child_parent_attempts.get() + 1);
                                    let recovered_parent = recovered_parent.ok_or(
                                        InvalidResidentRecordV1::ContextualAdmission {
                                            record: "operation handle",
                                            invariant: "exact recovered Delegate parent",
                                        },
                                    )?;
                                    if recovered_parent != parent_handle
                                        || reject_child_parent_once.replace(false)
                                    {
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
                        let mut owner = RecordingRecoveryOwner::default();
                        {
                            let mut turn = || {
                                host.recover_one_root_provider_start(
                                    &selector,
                                    ResidentDeploymentOperationAccess {
                                        store: &store,
                                        custody: &mut *custody,
                                        sessions: &sessions,
                                        anchor: &mut *anchor,
                                    },
                                    restart(),
                                    reborrow_admission(&mut admission),
                                    1 << 20,
                                    &mut owner,
                                )
                            };

                            assert!(matches!(
                                turn().expect("the parent source is admitted"),
                                ResidentProcessOperationRecoveryTurn::DeferredUnsupported {
                                    owner: ResidentProcessOperationRecoveryDeferred::Consult,
                                    ..
                                }
                            ));
                            assert!(matches!(
                                turn(),
                                Err(ResidentProcessOperationRecoverySweepError::Restart(
                                    ResidentProcessOperationRestartError::Recovery(_)
                                ))
                            ));
                            assert_eq!(child_parent_attempts.get(), 1);
                            assert!(matches!(
                                turn().expect("the exact failed Delegate source is retried"),
                                ResidentProcessOperationRecoveryTurn::DeferredUnsupported {
                                    owner: ResidentProcessOperationRecoveryDeferred::Delegate,
                                    ..
                                }
                            ));
                            assert_eq!(child_parent_attempts.get(), 2);
                            assert!(matches!(
                                turn().expect("the closed Delegate source exhausts the prefix"),
                                ResidentProcessOperationRecoveryTurn::Exhausted
                            ));
                        }
                        assert!(owner.calls.is_empty());

                        // Delegate recovery owns an independent cursor. It must
                        // reclassify the parent as another owner, retry the same
                        // child after one owner refusal, then exhaust without a
                        // third handoff.
                        assert!(matches!(
                            host.recover_one_delegate_start(
                                &selector,
                                ResidentDeploymentOperationAccess {
                                    store: &store,
                                    custody: &mut *custody,
                                    sessions: &sessions,
                                    anchor: &mut *anchor,
                                },
                                restart(),
                                reborrow_admission(&mut admission),
                                1 << 20,
                                &mut owner,
                            )
                            .expect("Delegate cursor admits the same parent source"),
                            ResidentProcessDelegateRecoveryTurn::SkippedOtherOwner(
                                ResidentProcessDelegateRecoveryOtherOwner::Root
                            )
                        ));
                        let initial_launch_calls = Rc::new(Cell::new(0));
                        let mut public = crate::resident::process_host::service_assembly::
                            ResidentServiceDelegateRecoveryAdapter::new(
                                InitialDelegateLaunchProbe(Rc::clone(&initial_launch_calls)),
                                1,
                            );
                        let mut public_owner = crate::resident::process_host::service_assembly::
                            resident_service_delegate_recovery_owner::<_, UnprovedHeldWriter>(
                                &mut public,
                            );
                        assert!(matches!(
                            host.recover_one_delegate_start(
                                &selector,
                                ResidentDeploymentOperationAccess {
                                    store: &store,
                                    custody: &mut *custody,
                                    sessions: &sessions,
                                    anchor: &mut *anchor,
                                },
                                restart(),
                                reborrow_admission(&mut admission),
                                1 << 20,
                                &mut public_owner,
                            )
                            .expect("background recovery retains the admitted Delegate phase"),
                            ResidentProcessDelegateRecoveryTurn::DeferredResolution(_)
                        ));
                        assert_eq!(
                            initial_launch_calls.get(),
                            0,
                            "background recovery cannot borrow an interactive A2A carrier"
                        );
                        assert!(matches!(
                            host.recover_one_delegate_start(
                                &selector,
                                ResidentDeploymentOperationAccess {
                                    store: &store,
                                    custody: &mut *custody,
                                    sessions: &sessions,
                                    anchor: &mut *anchor,
                                },
                                restart(),
                                reborrow_admission(&mut admission),
                                1 << 20,
                                &mut owner,
                            )
                            .expect("retained Delegate custody advances the frozen cursor"),
                            ResidentProcessDelegateRecoveryTurn::Exhausted
                        ));
                        assert!(owner.calls.is_empty());
                    },
                );
        },
    );
}
