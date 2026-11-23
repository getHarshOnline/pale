//! Ordinary provider execution accepts only `Invoke` operations.

use super::provider_dispatch::{with_retained_delegate_dispatch, A2aTaskOwnerCustodyModel};
use super::*;
use crate::provider_invocation::ProviderInvocationAdmissionError;
use crate::resident::operation_dispatch::OwnedProviderInvocationRequest;
use crate::resident::provider_task_service::{
    ResidentProviderTaskAcceptanceFailure, ResidentProviderTaskService,
    ResidentRecoveredProviderTaskAcceptanceFailure,
};
use crate::resident::task_owner::{
    ResidentOperationTask, ResidentOperationTaskCompletion, ResidentOperationTaskExecutor,
    ResidentOperationTaskLimits,
};
use crate::resident::{
    AmbientSessionAdmissionContext, OperationAttemptError, OperationCheckpointInput,
    OperationObservationAuthorityStoreResolver, ProviderOperationInvokeFailure,
    ProviderOperationInvokeInput,
};
use pale_ports::{
    AccumulatorPreparationLimits, ProviderInvocationObservation, WallClockSourceError,
    WallClockSourcePort,
};
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingClock(AtomicUsize);

impl WallClockSourcePort for CountingClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(UnixTimeV1::new(70).unwrap())
    }
}

#[derive(Default)]
struct CountingExecutor {
    accepts: usize,
}

impl
    ResidentOperationTaskExecutor<
        OwnedProviderInvocationRequest,
        Result<ProviderInvocationObservation, ProviderInvocationAdmissionError>,
    > for CountingExecutor
{
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<OwnedProviderInvocationRequest>,
    ) -> Result<(), ResidentOperationTask<OwnedProviderInvocationRequest>> {
        self.accepts += 1;
        Err(task)
    }

    fn try_completion(
        &mut self,
    ) -> Option<
        ResidentOperationTaskCompletion<
            OwnedProviderInvocationRequest,
            Result<ProviderInvocationObservation, ProviderInvocationAdmissionError>,
        >,
    > {
        None
    }
}

fn custody_model() -> A2aTaskOwnerCustodyModel<'static> {
    A2aTaskOwnerCustodyModel {
        clock_times: vec![63, 64, 65],
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
    }
}

fn invoke_input<'a>(
    current: &'a AmbientSessionAdmissionContext<'a>,
    signing: SignedResidentAdmissionContextV1<'a>,
    checkpoint_schema: &'a SchemaDigest,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    clock: &'a dyn WallClockSourcePort,
    resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    label: &str,
) -> ProviderOperationInvokeInput<'a> {
    ProviderOperationInvokeInput {
        current,
        signing,
        terminal_checkpoint: OperationCheckpointInput {
            reference: record_ref(&format!("{label}/terminal-checkpoint")),
            id: ControlCheckpointIdV1::parse(format!("{label}-terminal-checkpoint")).unwrap(),
        },
        result_ref: record_ref(&format!("{label}/result")),
        diagnostic_ref: record_ref(&format!("{label}/diagnostic")),
        reconciliation: ReconciliationFrontierReferenceV1 {
            record_ref: record_ref(&format!("{label}/reconciliation")),
            record_sha256: ReconciliationFrontierDigest::parse("d".repeat(64)).unwrap(),
        },
        terminal_ref: record_ref(&format!("{label}/terminal")),
        checkpoint_schema,
        checkpoint_authority,
        clock,
        resolve_store,
        maximum_record_bytes: 1024 * 1024,
        maximum_diagnostic_bytes: 1024 * 1024,
        accumulator_limits: AccumulatorPreparationLimits::new(1, 1024, 1024, 512, 65_536).unwrap(),
    }
}

#[test]
fn fresh_provider_preflight_returns_a_delegate_dispatch_without_touching_io() {
    with_retained_delegate_dispatch(
        custody_model(),
        |retained,
         current,
         signing,
         checkpoint_schema,
         checkpoint_authority,
         _,
         resolve_store,
         authority,
         custody,
         anchor,
         _,
         _,
         _,
         _| {
            let expected_ref = retained.observation().0.clone();
            let expected_digest = retained
                .observation()
                .1
                .canonical_operation_observation_sha256()
                .clone();
            let expected_store = retained.custody().candidate().store().clone();
            let clock = CountingClock(AtomicUsize::new(0));
            let input = invoke_input(
                current,
                signing,
                checkpoint_schema,
                checkpoint_authority,
                &clock,
                resolve_store,
                "provider-preflight-fresh-delegate",
            );
            let mut service = ResidentProviderTaskService::new(ResidentOperationTaskLimits::new(
                NonZeroUsize::new(1).unwrap(),
            ))
            .unwrap();
            let mut reservations = service.test_reservations();
            let mut executor = CountingExecutor::default();
            let failure = match service.try_accept(
                &mut crate::resident::local_operation_coordinator::ResidentLocalOperationCoordinator::new(
                    authority, custody, anchor,
                ),
                retained,
                &input,
                &mut reservations,
                &mut executor,
            ) {
                Err(ResidentProviderTaskAcceptanceFailure::Preflight(failure)) => failure,
                _ => panic!("Delegate work must stop at ordinary provider preflight"),
            };
            let ProviderOperationInvokeFailure::Admission { dispatch, reason } = failure else {
                panic!("operation class must be rejected as admission")
            };
            assert!(matches!(*reason, OperationAttemptError::Association));
            assert_eq!(dispatch.observation().0, &expected_ref);
            assert_eq!(
                dispatch
                    .observation()
                    .1
                    .canonical_operation_observation_sha256(),
                &expected_digest
            );
            assert_eq!(dispatch.custody().candidate().store(), &expected_store);
            assert_eq!(
                dispatch.operation().handle().1.operation_class(),
                &OperationClassV1::Delegate
            );
            assert_eq!(clock.0.load(Ordering::SeqCst), 0);
            assert_eq!(executor.accepts, 0);
            assert_eq!(service.in_flight(), 0);
            assert_eq!(reservations.outstanding(), 0);
        },
    );
}

#[test]
fn recovered_provider_preflight_returns_a_delegate_release_without_touching_io() {
    with_retained_delegate_dispatch(
        custody_model(),
        |retained,
         current,
         signing,
         checkpoint_schema,
         checkpoint_authority,
         _,
         resolve_store,
         authority,
         custody,
         anchor,
         _,
         _,
         _,
         _| {
            let release = retained.into_recovered_release_fixture();
            let expected_ref = release.observation().0.clone();
            let expected_digest = release
                .observation()
                .1
                .canonical_operation_observation_sha256()
                .clone();
            let expected_store = release.store().clone();
            let clock = CountingClock(AtomicUsize::new(0));
            let input = invoke_input(
                current,
                signing,
                checkpoint_schema,
                checkpoint_authority,
                &clock,
                resolve_store,
                "provider-preflight-recovered-delegate",
            );
            let mut service = ResidentProviderTaskService::new(ResidentOperationTaskLimits::new(
                NonZeroUsize::new(1).unwrap(),
            ))
            .unwrap();
            let mut reservations = service.test_reservations();
            let mut executor = CountingExecutor::default();
            let failure = match service.try_accept_recovered(
                &mut crate::resident::local_operation_coordinator::ResidentLocalOperationCoordinator::new(
                    authority, custody, anchor,
                ),
                release,
                &input,
                &mut reservations,
                &mut executor,
            ) {
                Err(ResidentRecoveredProviderTaskAcceptanceFailure::Preflight(failure)) => failure,
                _ => panic!("recovered Delegate work must stop at ordinary provider preflight"),
            };
            let (release, reason) = failure.into_parts();
            assert!(matches!(reason, OperationAttemptError::Association));
            assert_eq!(release.observation().0, &expected_ref);
            assert_eq!(
                release
                    .observation()
                    .1
                    .canonical_operation_observation_sha256(),
                &expected_digest
            );
            assert_eq!(release.store(), &expected_store);
            assert_eq!(
                release.operation().handle().1.operation_class(),
                &OperationClassV1::Delegate
            );
            assert_eq!(clock.0.load(Ordering::SeqCst), 0);
            assert_eq!(executor.accepts, 0);
            assert_eq!(service.in_flight(), 0);
            assert_eq!(reservations.outstanding(), 0);
        },
    );
}
