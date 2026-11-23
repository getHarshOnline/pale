//! Expiry of authenticated root Invoke heads before dispatch.

use super::provider_recovery::{AppendAnchor, AppendCustody};
use super::restart::{association, host_with_generation, restart_input};
use super::*;
use crate::local_operation_start_fixture::{record_ref, schema_digest, signature};
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_process_host_local_cancellation_restart_fixtures,
    with_process_host_operation_resolution_restart_fixtures,
    with_process_host_prepared_restart_fixtures,
    with_process_host_provider_resolved_restart_fixture, ProcessHostRestartFixture,
};
use crate::resident::process_host::expiry::*;
use crate::resident::process_host::restart::*;
use crate::resident::{
    OperationObservationAuthorityStoreRejection, OperationObservationAuthorityStoreResolver,
};
use pale_core::records::{
    ControlCheckpointIdV1, DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1,
    DiagnosticStageV1, OperationActivePhaseV1, OperationObservationStateV1,
    SignedResidentAdmissionContextV1, UnixTimeV1, CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    AccumulatorPreparationLimits, AuthorityStoreCoordinate, WallClockSourceError,
    WallClockSourcePort,
};
use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct ExactClock {
    now: UnixTimeV1,
    calls: AtomicUsize,
}

impl ExactClock {
    fn new(now: UnixTimeV1) -> Self {
        Self {
            now,
            calls: AtomicUsize::new(0),
        }
    }
}

impl WallClockSourcePort for ExactClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.now)
    }
}

struct SelectedStore(AuthorityStoreCoordinate);

impl OperationObservationAuthorityStoreResolver for SelectedStore {
    fn resolve(
        &self,
        _: &pale_core::records::OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        Ok(self.0.clone())
    }
}

fn accumulator_limits() -> AccumulatorPreparationLimits {
    AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000)
        .expect("expiry accumulator limits")
}

fn phase_name(start: &ResidentProcessRecoveredOperationStart) -> &'static str {
    match start.control.observation().selected_head().1.state() {
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
        _ => panic!("fixture must retain one pre-dispatch active head"),
    }
}

fn retain_at_exact_expiry(
    fixture: ProcessHostRestartFixture<'_>,
    observed: &Rc<std::cell::RefCell<BTreeSet<&'static str>>>,
) {
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
    let checkpoint_authority = recovery.operation.context.checkpoint_authority;
    let selector = selector_from_signing_authority(signing_authority);
    let port = MemoryAuthorityStore::default();
    let host = host_with_generation(&port, &selector, &selected_store, generation);
    let restart = restart_input(recovery, 1024 * 1024);
    let disposition = match host.classify_existing_operation(
        ResidentProcessExistingOperationStart {
            selector: selector.clone(),
            evidence: Box::new(evidence),
        },
        association(signing_authority, &restart),
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody,
            sessions,
            anchor,
        },
        restart,
        admission,
    ) {
        Ok(disposition) => disposition,
        Err(ResidentProcessOperationRestartError::State) => return,
        Err(error) => panic!("pre-dispatch head classifies: {error:?}"),
    };
    let start = match &disposition {
        ResidentProcessOperationRestartDisposition::OperationResolution(value) => &value.start,
        ResidentProcessOperationRestartDisposition::ProviderResolved(value) => &value.start,
        ResidentProcessOperationRestartDisposition::NativePrepared(value)
        | ResidentProcessOperationRestartDisposition::ProviderPrepared(value) => &value.start,
        _ => panic!("fixture must retain an eligible pre-dispatch head"),
    };
    observed.borrow_mut().insert(phase_name(start));
    let expiry = *start
        .control
        .operation()
        .operation()
        .handle()
        .1
        .expires_at();
    let clock = ExactClock::new(expiry);
    let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let prepared = host
        .prepare_recovered_pre_dispatch_expiry(
            start,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            ResidentProcessPreDispatchExpiryInput {
                clock: &clock,
                diagnostic_ref: record_ref("restart/pre-dispatch-expiry-diagnostic"),
                observation_ref: record_ref("restart/pre-dispatch-expiry-observation"),
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority,
                },
                maximum_diagnostic_bytes: 1024 * 1024,
                maximum_record_bytes: 1024 * 1024,
            },
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .expect("expiry successor prepares at the exact boundary");
    assert_eq!(clock.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        prepared.diagnostic().wire().code,
        "operation_expired_before_dispatch"
    );
    assert_eq!(
        prepared.diagnostic().wire().stage,
        DiagnosticStageV1::OperationAdmission
    );
    assert_eq!(
        prepared.diagnostic().wire().disposition,
        DiagnosticDispositionV1::Blocked
    );
    assert_eq!(
        prepared.diagnostic().wire().retry,
        DiagnosticRetryV1::NotIndicated
    );
    assert_eq!(
        prepared.diagnostic().wire().effect,
        DiagnosticEffectCertaintyV1::NotStarted
    );
    assert!(matches!(
        prepared.observation().1.state(),
        OperationObservationStateV1::Expired { .. }
    ));

    let resolver = SelectedStore(selected_store.clone());
    let mut append_custody = AppendCustody::committing(custody);
    let mut append_anchor = AppendAnchor::available(anchor);
    let aliased_roots = host
        .prepare_pre_dispatch_expiry_checkpoint(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append_custody,
                sessions,
                anchor: &mut append_anchor,
            },
            &prepared,
            &resolver,
            accumulator_limits(),
        )
        .expect("aliased expiry roots use the selected deployment")
        .expect("aliased expiry roots prepare");
    let rejected_checkpoint_signs = Cell::new(0);
    let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
    assert!(host
        .issue_pre_dispatch_expiry_checkpoint(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append_custody,
                sessions,
                anchor: &mut append_anchor,
            },
            aliased_roots,
            OperationCheckpointInput {
                reference: prepared.observation().0.clone(),
                id: ControlCheckpointIdV1::parse("restart-aliased-expiry")
                    .expect("checkpoint identity"),
            },
            &checkpoint_schema,
            checkpoint_authority,
            |_, _, _| {
                rejected_checkpoint_signs.set(rejected_checkpoint_signs.get() + 1);
                Ok(signature())
            },
        )
        .is_err());
    assert_eq!(rejected_checkpoint_signs.get(), 0);
    let diagnostic_aliased_roots = host
        .prepare_pre_dispatch_expiry_checkpoint(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append_custody,
                sessions,
                anchor: &mut append_anchor,
            },
            &prepared,
            &resolver,
            accumulator_limits(),
        )
        .expect("diagnostic-aliased expiry roots use the selected deployment")
        .expect("diagnostic-aliased expiry roots prepare");
    assert!(host
        .issue_pre_dispatch_expiry_checkpoint(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append_custody,
                sessions,
                anchor: &mut append_anchor,
            },
            diagnostic_aliased_roots,
            OperationCheckpointInput {
                reference: prepared.diagnostic_ref().clone(),
                id: ControlCheckpointIdV1::parse("restart-diagnostic-aliased-expiry")
                    .expect("checkpoint identity"),
            },
            &checkpoint_schema,
            checkpoint_authority,
            |_, _, _| {
                rejected_checkpoint_signs.set(rejected_checkpoint_signs.get() + 1);
                Ok(signature())
            },
        )
        .is_err());
    assert_eq!(rejected_checkpoint_signs.get(), 0);
    let roots = host
        .prepare_pre_dispatch_expiry_checkpoint(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append_custody,
                sessions,
                anchor: &mut append_anchor,
            },
            &prepared,
            &resolver,
            accumulator_limits(),
        )
        .expect("expiry roots use the selected deployment")
        .expect("expiry roots prepare");
    let issued = host
        .issue_pre_dispatch_expiry_checkpoint(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append_custody,
                sessions,
                anchor: &mut append_anchor,
            },
            roots,
            OperationCheckpointInput {
                reference: record_ref("restart/pre-dispatch-expiry-checkpoint"),
                id: ControlCheckpointIdV1::parse("restart-pre-dispatch-expiry")
                    .expect("checkpoint identity"),
            },
            &checkpoint_schema,
            checkpoint_authority,
            |_, _, _| Ok(signature()),
        )
        .expect("expiry checkpoint uses selected authority")
        .expect("expiry checkpoint signs");
    let completed = match host
        .retain_pre_dispatch_expiry(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append_custody,
                sessions,
                anchor: &mut append_anchor,
            },
            issued,
            &resolver,
            accumulator_limits(),
        )
        .expect("expiry custody uses selected deployment")
    {
        Ok(completed) => completed,
        Err(_) => panic!("expiry reaches database and anchor custody"),
    };
    assert!(matches!(
        completed.expiry().observation().1.state(),
        OperationObservationStateV1::Expired { .. }
    ));
    assert_eq!(append_custody.counts(), (1, 1));
    assert_eq!(append_anchor.counts(), (1, 1));
    assert_eq!(host.provider_routes.reserved, 0);
    assert_eq!(host.provider_routes.checked_out, 0);
    assert!(host.provider_routes.active.is_empty());
    assert_eq!(host.operations.outstanding_task_reservations(), 0);
}

#[test]
fn each_pre_dispatch_head_expires_at_the_exact_handle_boundary_without_effects() {
    let observed = Rc::new(std::cell::RefCell::new(BTreeSet::new()));
    let phases = Rc::clone(&observed);
    with_process_host_operation_resolution_restart_fixtures(&mut move |fixture| {
        retain_at_exact_expiry(fixture, &phases)
    });
    let phases = Rc::clone(&observed);
    with_process_host_provider_resolved_restart_fixture(&mut move |fixture| {
        retain_at_exact_expiry(fixture, &phases)
    });
    let phases = Rc::clone(&observed);
    with_process_host_prepared_restart_fixtures(&mut move |fixture| {
        retain_at_exact_expiry(fixture, &phases)
    });
    assert_eq!(
        observed.borrow().iter().copied().collect::<Vec<_>>(),
        vec![
            "context_bound",
            "dispatch_prepared",
            "provider_resolved",
            "resolving"
        ]
    );
}

#[test]
fn expiry_rejects_time_beyond_retention_after_one_clock_sample() {
    with_process_host_provider_resolved_restart_fixture(&mut |fixture| {
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
        let restart = restart_input(recovery, 1024 * 1024);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .unwrap();
        let ResidentProcessOperationRestartDisposition::ProviderResolved(recovered) = disposition
        else {
            panic!("fixture is provider resolved")
        };
        let retain_until = recovered
            .start
            .control
            .operation()
            .operation()
            .handle()
            .1
            .retain_until()
            .get();
        let clock = ExactClock::new(UnixTimeV1::new(retain_until + 1).unwrap());
        let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let result = host.prepare_recovered_pre_dispatch_expiry(
            &recovered.start,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            ResidentProcessPreDispatchExpiryInput {
                clock: &clock,
                diagnostic_ref: record_ref("restart/late-expiry-diagnostic"),
                observation_ref: record_ref("restart/late-expiry-observation"),
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &schema,
                    signing_authority,
                },
                maximum_diagnostic_bytes: 1024 * 1024,
                maximum_record_bytes: 1024 * 1024,
            },
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        );
        assert!(matches!(
            result,
            Err(ResidentProcessPreDispatchExpiryFailure::Expiry(
                crate::resident::operation_expiry::PreDispatchOperationExpiryError::RetentionElapsed
            ))
        ));
        assert_eq!(clock.calls.load(Ordering::SeqCst), 1);
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
    });
}

#[test]
fn cancellation_precedes_expiry_and_never_samples_the_clock() {
    with_process_host_local_cancellation_restart_fixtures(&mut |fixture| {
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
        let restart = restart_input(recovery, 1024 * 1024);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .unwrap();
        let ResidentProcessOperationRestartDisposition::LocalCancellationRequested(recovered) =
            disposition
        else {
            panic!("cancellation retains precedence")
        };
        let now = *recovered
            .start
            .control
            .operation()
            .operation()
            .handle()
            .1
            .expires_at();
        let clock = ExactClock::new(now);
        let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let reads_before_expiry = custody.access_counts();
        let result = host.prepare_recovered_pre_dispatch_expiry(
            &recovered.start,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            ResidentProcessPreDispatchExpiryInput {
                clock: &clock,
                diagnostic_ref: record_ref("restart/cancelled-expiry-diagnostic"),
                observation_ref: record_ref("restart/cancelled-expiry-observation"),
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &schema,
                    signing_authority,
                },
                maximum_diagnostic_bytes: 1024 * 1024,
                maximum_record_bytes: 1024 * 1024,
            },
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        );
        assert!(matches!(
            result,
            Err(ResidentProcessPreDispatchExpiryFailure::Selection(
                ResidentProcessHostSelectionError::PreDispatchExpiryOperationAuthority
            ))
        ));
        assert_eq!(clock.calls.load(Ordering::SeqCst), 0);
        assert_eq!(custody.access_counts(), reads_before_expiry);
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
    });
}

#[test]
fn substituted_store_is_rejected_before_clock_or_signing() {
    with_process_host_provider_resolved_restart_fixture(&mut |fixture| {
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
        let restart = restart_input(recovery, 1024 * 1024);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .unwrap();
        let ResidentProcessOperationRestartDisposition::ProviderResolved(recovered) = disposition
        else {
            panic!("fixture is provider resolved")
        };
        let clock = ExactClock::new(
            *recovered
                .start
                .control
                .operation()
                .operation()
                .handle()
                .1
                .expires_at(),
        );
        let signs = Cell::new(0);
        let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let foreign_store = store("foreign-expiry-store", "foreign-expiry-incarnation");
        let result = host.prepare_recovered_pre_dispatch_expiry(
            &recovered.start,
            ResidentDeploymentOperationAccess {
                store: &foreign_store,
                custody,
                sessions,
                anchor,
            },
            ResidentProcessPreDispatchExpiryInput {
                clock: &clock,
                diagnostic_ref: record_ref("restart/foreign-expiry-diagnostic"),
                observation_ref: record_ref("restart/foreign-expiry-observation"),
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &schema,
                    signing_authority,
                },
                maximum_diagnostic_bytes: 1024 * 1024,
                maximum_record_bytes: 1024 * 1024,
            },
            |_, _, _, _| Ok(()),
            |_, _, _| {
                signs.set(signs.get() + 1);
                Ok(signature())
            },
        );
        assert!(matches!(
            result,
            Err(ResidentProcessPreDispatchExpiryFailure::Selection(
                ResidentProcessHostSelectionError::AccessStoreAssociation
            ))
        ));
        assert_eq!((clock.calls.load(Ordering::SeqCst), signs.get()), (0, 0));
    });
}
