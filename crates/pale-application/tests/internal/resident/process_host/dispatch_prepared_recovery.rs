//! Exact cold-restart completion of retained dispatch preparations.

use super::provider_recovery::{AppendAnchor, AppendCustody};
use super::restart::{association, host_with_generation, restart_input};
use super::*;
use crate::local_operation_start_fixture::{record_ref, schema_digest, signature};
use crate::provider_invocation::{
    ProviderInvocationAdmissionContext, ProviderInvocationSafeFailureProof,
};
use crate::resident::operation_dispatch::{
    prepare_recovered_provider_operation_call, OperationAttemptError,
    PreparedRecoveredOperationDispatch, PreparedRecoveredProviderDispatch,
    RecoveredDispatchCustodyFailure,
};
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_process_host_prepared_restart_fixtures, ProcessHostRestartFixture,
};
use crate::resident::process_host::dispatch_prepared_recovery::*;
use crate::resident::process_host::restart::*;
use crate::resident::task_owner::{ResidentOperationTask, ResidentOperationTaskCompletion};
use crate::resident::{
    OperationObservationAuthorityStoreRejection, OperationObservationAuthorityStoreResolver,
    ProviderOperationUncertainty,
};
use crate::session_admission_fixture::ProvingAuthority;
use pale_core::records::{
    AdapterFailureAdmissionContextV1, AdapterFailureDigest, AdapterFailurePreimageV1,
    ArtifactDigest, BoundedResidentTextV1, CanonicalSignatureV1, CapabilityAttemptIdV1,
    CapabilityResultAdmissionContextV1, CapabilityResultDigest, CapabilityResultPreimageV1,
    CapabilitySuccessAdmissionContextV1, ControlCheckpointIdV1, InvalidResidentRecordV1,
    OperationActivePhaseV1, OperationObservationStateV1, ReconciliationFrontierDigest,
    ReconciliationFrontierReferenceV1, RequestBindingV1, ResidentRecordRefV1, SchemaDigest,
    SchemaId, SignedResidentAdmissionContextV1, UnixTimeV1, ADAPTER_FAILURE_V1_SCHEMA_ID,
    CAPABILITY_RESULT_V1_SCHEMA_ID, CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    AccumulatorPreparationLimits, AuthorityStoreCoordinate, ControlRecord, ControlRecordBytes,
    ControlStoreErrorKind, OperationCustodyReadBackOutcome, WallClockSourceError,
    WallClockSourcePort,
};
use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

enum PreparedDispatchFixture {
    Native(Arc<PreparedRecoveredOperationDispatch>),
    Provider(Box<PreparedRecoveredProviderDispatch>),
}

impl PreparedDispatchFixture {
    fn shared_arc(&self) -> &Arc<PreparedRecoveredOperationDispatch> {
        match self {
            Self::Native(dispatch) => dispatch,
            Self::Provider(dispatch) => dispatch.dispatch(),
        }
    }

    fn shared(&self) -> &PreparedRecoveredOperationDispatch {
        self.shared_arc()
    }
}

pub(in crate::resident::process_host) struct RecoveryReleaseClock(
    pub(in crate::resident::process_host) AtomicU64,
);

impl WallClockSourcePort for RecoveryReleaseClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        let next = self.0.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(UnixTimeV1::new(next).expect("positive recovered-release time"))
    }
}

#[derive(Default)]
struct RecoveredProviderExecutor {
    calls: usize,
    accepted: VecDeque<ResidentOperationTask<OwnedProviderInvocationRequest>>,
    completions: VecDeque<
        ResidentOperationTaskCompletion<OwnedProviderInvocationRequest, ProcessProviderTaskOutput>,
    >,
    refuse: bool,
}

impl RecoveredProviderExecutor {
    fn complete_next_unavailable(&mut self) {
        let task = self
            .accepted
            .pop_front()
            .expect("one accepted provider task");
        self.completions
            .push_back(ResidentOperationTaskCompletion::Completed {
                task,
                output: Ok(pale_ports::ProviderInvocationObservation::Unavailable(
                    pale_ports::ProviderInvocationUnavailability::EvidenceUnknown,
                )),
            });
    }

    fn lose_next(&mut self) {
        let task = self
            .accepted
            .pop_front()
            .expect("one accepted provider task");
        self.completions.push_back(task.into_lost());
    }
}

#[derive(Clone, Copy)]
enum RecoveredProviderReturn {
    CompletedUnavailable,
    ExecutorLost,
}

impl ResidentOperationTaskExecutor<OwnedProviderInvocationRequest, ProcessProviderTaskOutput>
    for RecoveredProviderExecutor
{
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<OwnedProviderInvocationRequest>,
    ) -> Result<(), ResidentOperationTask<OwnedProviderInvocationRequest>> {
        self.calls += 1;
        if self.refuse {
            return Err(task);
        }
        self.accepted.push_back(task);
        Ok(())
    }

    fn try_completion(
        &mut self,
    ) -> Option<
        ResidentOperationTaskCompletion<OwnedProviderInvocationRequest, ProcessProviderTaskOutput>,
    > {
        self.completions.pop_front()
    }
}

pub(in crate::resident::process_host) struct UnusedRecoveredProviderEvidence;

impl crate::provider_invocation::ProviderInvocationEvidenceAdmission
    for UnusedRecoveredProviderEvidence
{
    fn admit_evidence_frontier(
        &self,
        _: &pale_ports::ProviderInvocationEvidenceFrontier,
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("unavailable provider output carries no positive evidence")
    }

    fn resolve_immutable(&self, _: &ResidentRecordRefV1) -> Option<Vec<u8>> {
        panic!("unavailable provider output carries no immutable evidence")
    }

    fn admit_payload(
        &self,
        _: &SchemaId,
        _: &SchemaDigest,
        _: &[u8],
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("unavailable provider output carries no payload")
    }

    fn admit_failure_class(
        &self,
        _: pale_core::records::AdapterFailureClassV1,
        _: Option<UnixTimeV1>,
        _: &RequestBindingV1,
        _: &CapabilityAttemptIdV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("unavailable provider output is not a safe failure")
    }

    fn admit_absence_evidence(
        &self,
        _: &ResidentRecordRefV1,
        _: &ArtifactDigest,
        _: &[u8],
        _: &RequestBindingV1,
        _: &CapabilityAttemptIdV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("unavailable provider output carries no absence proof")
    }

    fn verify_result_signature(
        &self,
        _: &[u8],
        _: &CapabilityResultPreimageV1,
        _: &CapabilityResultDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("unavailable provider output carries no result signature")
    }

    fn verify_failure_signature(
        &self,
        _: &[u8],
        _: &AdapterFailurePreimageV1,
        _: &AdapterFailureDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("unavailable provider output carries no failure signature")
    }
}

struct SelectedStore {
    coordinate: AuthorityStoreCoordinate,
    resolutions: Cell<usize>,
    fail_at: Option<usize>,
}

impl SelectedStore {
    fn available(coordinate: AuthorityStoreCoordinate) -> Self {
        Self {
            coordinate,
            resolutions: Cell::new(0),
            fail_at: None,
        }
    }

    fn failing_at(coordinate: AuthorityStoreCoordinate, fail_at: usize) -> Self {
        Self {
            coordinate,
            resolutions: Cell::new(0),
            fail_at: Some(fail_at),
        }
    }
}

impl OperationObservationAuthorityStoreResolver for SelectedStore {
    fn resolve(
        &self,
        _: &pale_core::records::OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        let resolution = self.resolutions.get() + 1;
        self.resolutions.set(resolution);
        if self.fail_at == Some(resolution) {
            return Err(OperationObservationAuthorityStoreRejection);
        }
        Ok(self.coordinate.clone())
    }
}

pub(in crate::resident::process_host) fn accumulator_limits() -> AccumulatorPreparationLimits {
    AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000)
        .expect("recovered dispatch accumulator limits")
}

pub(in crate::resident::process_host) struct ProviderAdmissionFixture<'a> {
    pub(in crate::resident::process_host) binding: &'a pale_core::records::RequestBindingV1,
    pub(in crate::resident::process_host) attempt_id: &'a pale_core::records::CapabilityAttemptIdV1,
    pub(in crate::resident::process_host) signing:
        &'a pale_core::records::SigningAuthorityBindingV1,
    pub(in crate::resident::process_host) result_schema: &'a pale_core::records::SchemaDigest,
    pub(in crate::resident::process_host) failure_schema: &'a pale_core::records::SchemaDigest,
    pub(in crate::resident::process_host) contracts: &'a pale_core::records::ContractSetV1,
    pub(in crate::resident::process_host) kind: &'a BoundedResidentTextV1,
    pub(in crate::resident::process_host) version: &'a BoundedResidentTextV1,
}

pub(in crate::resident::process_host) fn provider_admission(
    fixture: ProviderAdmissionFixture<'_>,
) -> ProviderInvocationAdmissionContext<'_> {
    let ProviderAdmissionFixture {
        binding,
        attempt_id,
        signing,
        result_schema,
        failure_schema,
        contracts,
        kind,
        version,
    } = fixture;
    let success = CapabilitySuccessAdmissionContextV1 {
        result: CapabilityResultAdmissionContextV1 {
            expected_schema_sha256: result_schema,
            expected_request_binding: binding,
            expected_attempt_id: attempt_id,
            expected_implementation_kind: kind,
            expected_implementation_version: version,
            signing_authority: signing,
        },
        contract_set_ref: binding.contract_set_ref(),
        contract_set: contracts,
    };
    ProviderInvocationAdmissionContext::new(
        success,
        AdapterFailureAdmissionContextV1 {
            expected_schema_sha256: failure_schema,
            expected_request_binding: binding,
            expected_attempt_id: attempt_id,
            expected_implementation_kind: kind,
            expected_implementation_version: version,
            signing_authority: signing,
        },
        ProviderInvocationSafeFailureProof::BeforeDispatch,
    )
}

#[test]
fn prepared_restart_classifies_each_owner_without_releasing_work() {
    let native = Rc::new(Cell::new(0));
    let provider = Rc::new(Cell::new(0));
    let native_cases = Rc::clone(&native);
    let provider_cases = Rc::clone(&provider);
    with_process_host_prepared_restart_fixtures(&mut move |fixture| {
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
        let input = restart_input(recovery, 1024 * 1024);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &input),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
            .expect("exact preparation classifies");
        match disposition {
            ResidentProcessOperationRestartDisposition::NativePrepared(attempt) => {
                native_cases.set(native_cases.get() + 1);
                assert_eq!(
                    attempt
                        .start
                        .control
                        .observation()
                        .selected_head()
                        .1
                        .state(),
                    attempt.start.control.observation().selected().1.state()
                );
            }
            ResidentProcessOperationRestartDisposition::ProviderPrepared(attempt) => {
                provider_cases.set(provider_cases.get() + 1);
                assert_eq!(
                    attempt
                        .start
                        .control
                        .observation()
                        .selected_head()
                        .1
                        .state(),
                    attempt.start.control.observation().selected().1.state()
                );
            }
            _ => panic!("retained preparation has one owner-specific disposition"),
        }
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
    });
    assert_eq!((native.get(), provider.get()), (1, 1));
}

#[test]
fn prepared_restart_signs_and_retains_dispatched_without_execution() {
    let completed = Rc::new(Cell::new(0));
    let completed_cases = Rc::clone(&completed);
    with_process_host_prepared_restart_fixtures(&mut move |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            current,
            request,
            native_registry,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            session: _,
            rediscovery_generation: _,
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let checkpoint_authority = recovery.operation.context.checkpoint_authority;
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let restart = restart_input(recovery, 1024 * 1024);
        let disposition = host
            .classify_existing_operation(
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
            )
            .expect("exact preparation classifies");
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let recovery_current = current.at_time(UnixTimeV1::new(current.now.get() + 10).unwrap());
        let preparation_input = |reference| ResidentProcessPreparedDispatchInput {
            current: &recovery_current,
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &observation_schema,
                signing_authority,
            },
            observation_ref: record_ref(reference),
            maximum_selection_bytes: 1024 * 1024,
            maximum_record_bytes: 1024 * 1024,
        };
        let mut authority = ProvingAuthority;
        let contracts = crate::session_admission_fixture::local_start_contract_set();
        let native_owner = matches!(
            disposition,
            ResidentProcessOperationRestartDisposition::NativePrepared(_)
        );
        let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
        let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
        let kind = BoundedResidentTextV1::parse("host_compute").unwrap();
        let version = BoundedResidentTextV1::parse("1").unwrap();
        let dispatched = match &disposition {
            ResidentProcessOperationRestartDisposition::NativePrepared(attempt) => {
                PreparedDispatchFixture::Native(
                    host.recover_native_prepared_dispatch(
                        attempt,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions,
                            anchor,
                        },
                        preparation_input("restart/native-dispatched"),
                        native_registry,
                        &contracts,
                        &mut authority,
                        |_, _, _, _| Ok(()),
                        |_, _, _| Ok(signature()),
                    )
                    .expect("native preparation recovers"),
                )
            }
            ResidentProcessOperationRestartDisposition::ProviderPrepared(attempt) => {
                let OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::DispatchPrepared { attempt_id, .. },
                } = attempt.start.control.observation().selected().1.state()
                else {
                    panic!("provider fixture retains dispatch preparation")
                };
                let binding = attempt
                    .start
                    .control
                    .operation()
                    .operation()
                    .handle()
                    .1
                    .request_binding();
                assert_eq!(
                    &request
                        .local_request_binding()
                        .expect("provider request binding"),
                    binding
                );
                let provider = provider_admission(ProviderAdmissionFixture {
                    binding,
                    attempt_id,
                    signing: signing_authority,
                    result_schema: &result_schema,
                    failure_schema: &failure_schema,
                    contracts: &contracts,
                    kind: &kind,
                    version: &version,
                });
                PreparedDispatchFixture::Provider(Box::new(
                    host.recover_provider_prepared_dispatch(
                        attempt,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions,
                            anchor,
                        },
                        preparation_input("restart/provider-dispatched"),
                        provider,
                        &mut authority,
                        |_, _, _, _| Ok(()),
                        |_, _, _| Ok(signature()),
                    )
                    .expect("provider preparation recovers"),
                ))
            }
            _ => panic!("fixture is an exact retained preparation"),
        };
        let OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Dispatched { .. },
        } = dispatched.shared().observation().1.state()
        else {
            panic!("semantic recovery signs only Dispatched")
        };
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
        assert_eq!(host.operations.outstanding_task_reservations(), 0);

        let resolver = SelectedStore::available(selected_store.clone());
        let mut append_custody = AppendCustody::committing(custody);
        let mut append_anchor = AppendAnchor::available(anchor);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let checkpoint_now = UnixTimeV1::new(recovery_current.now.get() + 1).unwrap();
        let rejected_signs = Cell::new(0);
        let alias_prepared = host
            .prepare_recovered_dispatch_checkpoint(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                dispatched.shared_arc(),
                &resolver,
                accumulator_limits(),
            )
            .expect("selected alias preparation")
            .expect("alias checkpoint roots");
        assert!(host
            .issue_recovered_dispatch_checkpoint(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                alias_prepared,
                OperationCheckpointInput {
                    reference: dispatched.shared().observation().0.clone(),
                    id: ControlCheckpointIdV1::parse("restart-alias-checkpoint").unwrap(),
                },
                checkpoint_now,
                &checkpoint_schema,
                checkpoint_authority,
                |_, _, _| {
                    rejected_signs.set(rejected_signs.get() + 1);
                    Ok(signature())
                },
            )
            .is_err());
        let expired_prepared = host
            .prepare_recovered_dispatch_checkpoint(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                dispatched.shared_arc(),
                &resolver,
                accumulator_limits(),
            )
            .expect("selected expiry preparation")
            .expect("expiry checkpoint roots");
        assert!(host
            .issue_recovered_dispatch_checkpoint(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                expired_prepared,
                OperationCheckpointInput {
                    reference: record_ref("restart/expired-checkpoint"),
                    id: ControlCheckpointIdV1::parse("restart-expired-checkpoint").unwrap(),
                },
                dispatched.shared().request_expires_at(),
                &checkpoint_schema,
                checkpoint_authority,
                |_, _, _| {
                    rejected_signs.set(rejected_signs.get() + 1);
                    Ok(signature())
                },
            )
            .is_err());
        assert_eq!(rejected_signs.get(), 0);
        let prepared = host
            .prepare_recovered_dispatch_checkpoint(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                dispatched.shared_arc(),
                &resolver,
                accumulator_limits(),
            )
            .expect("selected preparation phase")
            .expect("dispatch checkpoint roots");
        let identity = OperationCheckpointInput {
            reference: record_ref("restart/dispatched-checkpoint"),
            id: ControlCheckpointIdV1::parse("restart-dispatched-checkpoint").unwrap(),
        };
        let checkpoint_signs = Cell::new(0);
        let issued = host
            .issue_recovered_dispatch_checkpoint(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                prepared,
                identity,
                checkpoint_now,
                &checkpoint_schema,
                checkpoint_authority,
                |_, _, _| {
                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                    Ok(signature())
                },
            )
            .expect("selected checkpoint phase")
            .expect("checkpoint signs once");
        if native_owner {
            append_anchor.fail_next_advance();
        } else {
            append_custody.fail_next_commit(ControlStoreErrorKind::OutcomeUnknown);
            append_custody.push_read_back(OperationCustodyReadBackOutcome::Predecessor);
            append_custody.fail_next_read_back(ControlStoreErrorKind::Storage);
            append_custody.push_read_back(OperationCustodyReadBackOutcome::ExactSuccessor);
        }
        let retention_resolver = if native_owner {
            SelectedStore::failing_at(selected_store.clone(), 1)
        } else {
            SelectedStore::available(selected_store.clone())
        };
        let first = match host.retain_recovered_dispatch(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append_custody,
                sessions,
                anchor: &mut append_anchor,
            },
            issued,
            &retention_resolver,
            accumulator_limits(),
        ) {
            Ok(result) => result,
            Err(_) => panic!("selected retention phase"),
        };
        let completed_dispatch = if native_owner {
            let Err(RecoveredDispatchCustodyFailure::Issued { continuation, .. }) = first else {
                panic!("attachment interruption retains the issued checkpoint")
            };
            let retried = match host.retain_recovered_dispatch(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                *continuation,
                &resolver,
                accumulator_limits(),
            ) {
                Ok(result) => result,
                Err(_) => panic!("selected attachment continuation"),
            };
            let Err(RecoveredDispatchCustodyFailure::Anchor(pending)) = retried else {
                panic!("anchor interruption retains exact database custody")
            };
            match host.continue_recovered_dispatch_anchor(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                *pending,
            ) {
                Ok(result) => result,
                Err(_) => panic!("selected anchor continuation"),
            }
            .unwrap_or_else(|_| panic!("anchor continuation completes"))
        } else {
            let Err(RecoveredDispatchCustodyFailure::Commit(failure)) = first else {
                panic!("unknown commit retains the exact physical candidate")
            };
            let pending = match host.reconcile_recovered_dispatch(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                *failure,
            ) {
                Ok(result) => result,
                Err(_) => panic!("selected commit reconciliation"),
            }
            .unwrap_or_else(|_| panic!("exact read-back resolves the commit"));
            match host.continue_recovered_dispatch_anchor(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                pending,
            ) {
                Ok(result) => result,
                Err(_) => panic!("selected anchor continuation"),
            }
            .unwrap_or_else(|_| panic!("provider anchor continuation completes"))
        };
        assert_eq!(
            completed_dispatch.observation().0,
            dispatched.shared().observation().0
        );
        assert_eq!(checkpoint_signs.get(), 1);
        if native_owner {
            assert_eq!(append_custody.counts(), (1, 1));
            assert_eq!(append_anchor.counts(), (2, 1));
        } else {
            assert_eq!(append_custody.counts(), (1, 3));
            assert_eq!(append_anchor.counts(), (1, 1));
            let PreparedDispatchFixture::Provider(provider_dispatch) = dispatched else {
                unreachable!("provider ownership was classified above")
            };
            let release = host
                .complete_recovered_provider_release(
                    &selector,
                    &selected_store,
                    *provider_dispatch,
                    completed_dispatch,
                )
                .unwrap_or_else(|_| panic!("exact recovered provider release joins"));
            let clock = RecoveryReleaseClock(AtomicU64::new(checkpoint_now.get()));
            let invocation = ProviderOperationInvokeInput {
                current: &recovery_current,
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority,
                },
                terminal_checkpoint: OperationCheckpointInput {
                    reference: record_ref("restart/provider-replay-terminal-checkpoint"),
                    id: ControlCheckpointIdV1::parse("restart-provider-replay-terminal-checkpoint")
                        .unwrap(),
                },
                result_ref: record_ref("restart/provider-replay-result"),
                diagnostic_ref: record_ref("restart/provider-replay-diagnostic"),
                reconciliation: ReconciliationFrontierReferenceV1 {
                    record_ref: record_ref("restart/provider-replay-reconciliation"),
                    record_sha256: ReconciliationFrontierDigest::parse("d".repeat(64)).unwrap(),
                },
                terminal_ref: record_ref("restart/provider-replay-terminal"),
                checkpoint_schema: &checkpoint_schema,
                checkpoint_authority,
                clock: &clock,
                resolve_store: &resolver,
                maximum_record_bytes: 1024 * 1024,
                maximum_diagnostic_bytes: 1024 * 1024,
                accumulator_limits: accumulator_limits(),
            };
            let failure = match prepare_recovered_provider_operation_call(
                release,
                &invocation,
                &mut ProvingAuthority,
                &mut append_custody,
                &mut append_anchor,
            ) {
                Ok(_) => panic!("reconciled dispatch custody cannot replay a provider call"),
                Err(failure) => failure,
            };
            let (_, reason) = failure.into_parts();
            assert!(matches!(
                reason,
                OperationAttemptError::Custody(
                    crate::resident::OperationDispatchCustodyError::NotNewCommit
                )
            ));
        }
        completed_cases.set(completed_cases.get() + 1);
    });
    assert_eq!(completed.get(), 2);
}

#[test]
fn newly_committed_provider_recovery_releases_once_through_the_process_task_owner() {
    recovered_provider_root_progression(RecoveredProviderReturn::CompletedUnavailable);
}

#[test]
fn recovered_provider_executor_loss_uses_the_exact_dispatch_without_replay() {
    recovered_provider_root_progression(RecoveredProviderReturn::ExecutorLost);
}

fn recovered_provider_root_progression(returned: RecoveredProviderReturn) {
    let provider_cases = Rc::new(Cell::new(0));
    let counted = Rc::clone(&provider_cases);
    with_process_host_prepared_restart_fixtures(&mut move |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            current,
            request,
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
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let restart = restart_input(recovery, 1024 * 1024);
        let disposition = host
            .classify_existing_operation(
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
            )
            .expect("exact preparation classifies");
        let ResidentProcessOperationRestartDisposition::ProviderPrepared(attempt) = disposition
        else {
            return;
        };
        counted.set(counted.get() + 1);

        let issued_operation = attempt.start.control.operation().operation();
        let recovered_operation = crate::resident::RecoveredLocalOperation::fixture(
            issued_operation,
            request.clone(),
            selected_store.clone(),
        );
        let (handle_ref, handle) = issued_operation.handle();
        let recovered_handle = crate::resident::record_projection::coordinate(
            handle_ref,
            handle.canonical_operation_handle_sha256().as_str(),
        )
        .expect("provider handle recovery coordinate");
        let prepared_observation = (
            attempt.start.control.observation().selected().0.clone(),
            attempt.start.control.observation().selected().1.clone(),
        );
        let context_binding = attempt
            .start
            .control
            .observation()
            .context_binding()
            .map(|(reference, observation)| (reference.clone(), observation.clone()));
        let generation_digest = attempt.start.generation_digest.clone();
        let selection = attempt.selection.clone();

        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
        let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
        let contracts = crate::session_admission_fixture::local_start_contract_set();
        let kind = BoundedResidentTextV1::parse("host_compute").unwrap();
        let version = BoundedResidentTextV1::parse("1").unwrap();
        let OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::DispatchPrepared { attempt_id, .. },
        } = attempt.start.control.observation().selected().1.state()
        else {
            panic!("provider fixture retains dispatch preparation")
        };
        let binding = attempt
            .start
            .control
            .operation()
            .operation()
            .handle()
            .1
            .request_binding();
        assert_eq!(
            &request.local_request_binding().expect("provider binding"),
            binding
        );
        let provider = provider_admission(ProviderAdmissionFixture {
            binding,
            attempt_id,
            signing: signing_authority,
            result_schema: &result_schema,
            failure_schema: &failure_schema,
            contracts: &contracts,
            kind: &kind,
            version: &version,
        });
        let recovery_current = current.at_time(UnixTimeV1::new(current.now.get() + 10).unwrap());
        let mut driver = host
            .provider_recovery_driver(&selector)
            .expect("one exact provider deployment");
        assert_eq!(driver.application().store(), &selected_store);
        let dispatch = driver
            .recover_prepared_dispatch(
                &attempt,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                ResidentProcessPreparedDispatchInput {
                    current: &recovery_current,
                    signing: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority,
                    },
                    observation_ref: record_ref("restart/provider-fresh-dispatched"),
                    maximum_selection_bytes: 1024 * 1024,
                    maximum_record_bytes: 1024 * 1024,
                },
                provider,
                &mut ProvingAuthority,
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
            )
            .expect("provider preparation recovers through the split driver");
        let resolver = SelectedStore::available(selected_store.clone());
        let mut append_custody = AppendCustody::committing(custody);
        let mut append_anchor = AppendAnchor::available(anchor);
        let prepared = dispatch
            .dispatch()
            .prepare_checkpoint(&resolver, accumulator_limits(), &mut append_custody)
            .expect("fresh dispatch checkpoint roots");
        let checkpoint_now = UnixTimeV1::new(recovery_current.now.get() + 1).unwrap();
        let issued = prepared
            .issue(
                OperationCheckpointInput {
                    reference: record_ref("restart/provider-fresh-checkpoint"),
                    id: ControlCheckpointIdV1::parse("restart-provider-fresh-checkpoint").unwrap(),
                },
                checkpoint_now,
                &checkpoint_schema,
                checkpoint_authority,
                |_, _, _| Ok(signature()),
            )
            .expect("fresh checkpoint signs once");
        let completed = issued
            .attach_and_commit(
                &resolver,
                accumulator_limits(),
                &mut append_custody,
                &mut append_anchor,
            )
            .unwrap_or_else(|_| panic!("fresh dispatch reaches new database and anchor custody"));
        let dispatched_observation = (
            completed.observation().0.clone(),
            completed.observation().1.clone(),
        );
        let dispatch_checkpoint = (
            completed.checkpoint().0.clone(),
            completed.checkpoint().1.clone(),
        );
        let recovered_dispatch =
            crate::resident::RecoveredOperationObservation::history_fixture_for_handle(
                recovered_handle,
                dispatched_observation.clone(),
                Some(prepared_observation),
                context_binding,
                dispatched_observation,
                dispatch_checkpoint,
                selected_store.clone(),
            );
        let recovered_attempt = ResidentProcessRecoveredAttempt {
            start: ResidentProcessRecoveredOperationStart {
                selector: selector.clone(),
                control: crate::resident::RecoveredOperationControl::fixture(
                    recovered_operation,
                    recovered_dispatch,
                ),
                generation_digest,
                lineage: ResidentProcessOperationLineage::Root,
            },
            dispatch: ResidentProcessAttemptObservation::CurrentHead,
            selection,
        };
        let release = driver
            .complete_release(dispatch, completed)
            .unwrap_or_else(|_| panic!("same driver joins the new dispatch custody"));
        let clock = RecoveryReleaseClock(AtomicU64::new(checkpoint_now.get()));
        let invocation = ProviderOperationInvokeInput {
            current: &recovery_current,
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &observation_schema,
                signing_authority,
            },
            terminal_checkpoint: OperationCheckpointInput {
                reference: record_ref("restart/provider-fresh-terminal-checkpoint"),
                id: ControlCheckpointIdV1::parse("restart-provider-fresh-terminal-checkpoint")
                    .unwrap(),
            },
            result_ref: record_ref("restart/provider-fresh-result"),
            diagnostic_ref: record_ref("restart/provider-fresh-diagnostic"),
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref("restart/provider-fresh-reconciliation"),
                record_sha256: ReconciliationFrontierDigest::parse("d".repeat(64)).unwrap(),
            },
            terminal_ref: record_ref("restart/provider-fresh-terminal"),
            checkpoint_schema: &checkpoint_schema,
            checkpoint_authority,
            clock: &clock,
            resolve_store: &resolver,
            maximum_record_bytes: 1024 * 1024,
            maximum_diagnostic_bytes: 1024 * 1024,
            accumulator_limits: accumulator_limits(),
        };
        let mut executor = RecoveredProviderExecutor::default();
        let foreign_store = store("restart-provider-foreign", "restart-provider-foreign-1");
        let release = match driver.launch(
            ResidentDeploymentOperationAccess {
                store: &foreign_store,
                custody: &mut append_custody,
                sessions,
                anchor: &mut append_anchor,
            },
            &mut ProvingAuthority,
            release,
            &invocation,
            &mut executor,
        ) {
            Ok(_) => panic!("a foreign store cannot release provider work"),
            Err(failure) => match *failure {
                ResidentProcessRecoveredProviderLaunchFailure::Selection {
                    release,
                    reason: ResidentProcessHostSelectionError::ProviderOperationAuthority,
                } => *release,
                _ => panic!("store substitution must preserve the exact release token"),
            },
        };
        assert_eq!(executor.calls, 0);
        assert_eq!(driver.routes.reserved, 0);
        assert_eq!(driver.operations.outstanding_task_reservations(), 0);
        let key = driver
            .launch(
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                &mut ProvingAuthority,
                release,
                &invocation,
                &mut executor,
            )
            .unwrap_or_else(|_| panic!("new recovered dispatch transfers once"));
        assert_eq!(executor.calls, 1);
        assert_eq!(executor.accepted.len(), 1);
        assert_eq!(executor.accepted.front().unwrap().key(), &key);
        assert_eq!(driver.routes.reserved, 0);
        assert_eq!(driver.routes.checked_out, 0);
        assert_eq!(driver.routes.active.len(), 1);
        assert_eq!(driver.operations.outstanding_task_reservations(), 1);

        match returned {
            RecoveredProviderReturn::CompletedUnavailable => executor.complete_next_unavailable(),
            RecoveredProviderReturn::ExecutorLost => executor.lose_next(),
        }
        let progress = driver
            .poll(&mut executor)
            .expect("accepted provider task returns one completion");
        let ResidentProcessProviderCompletionPoll::Routed(progress) = progress else {
            panic!("accepted completion remains on its exact deployment route")
        };
        let recording = match returned {
            RecoveredProviderReturn::CompletedUnavailable => driver
                .record_completed(
                    *progress,
                    &recovered_attempt,
                    1024 * 1024,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut append_custody,
                        sessions,
                        anchor: &mut append_anchor,
                    },
                    &mut ProvingAuthority,
                    provider,
                    &invocation,
                    &UnusedRecoveredProviderEvidence,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("exact dispatch history records the completion")),
            RecoveredProviderReturn::ExecutorLost => driver
                .record_lost(
                    *progress,
                    &recovered_attempt,
                    1024 * 1024,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody: &mut append_custody,
                        sessions,
                        anchor: &mut append_anchor,
                    },
                    &mut ProvingAuthority,
                    &invocation,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("exact dispatch history records the executor loss")),
        };
        let ResidentProcessProviderRecording::Recorded {
            result,
            cancellation,
        } = recording
        else {
            panic!("accepted recovered provider work reaches durable terminal custody")
        };
        assert_eq!(
            cancellation,
            ResidentOperationTaskCancellationState::Unnotified
        );
        let ProviderOperationInvokeResult::OutcomeUnknown(unknown) = result else {
            panic!("provider unavailability must remain outcome-unknown")
        };
        match returned {
            RecoveredProviderReturn::CompletedUnavailable => assert!(matches!(
                unknown.uncertainty(),
                ProviderOperationUncertainty::Unavailable(_)
            )),
            RecoveredProviderReturn::ExecutorLost => assert!(matches!(
                unknown.uncertainty(),
                ProviderOperationUncertainty::ColdRestartAfterDispatch
            )),
        }
        assert_eq!(executor.calls, 1);
        assert_eq!(driver.routes.reserved, 0);
        assert_eq!(driver.routes.checked_out, 0);
        assert!(driver.routes.active.is_empty());
        assert_eq!(driver.operations.outstanding_task_reservations(), 0);
    });
    assert_eq!(provider_cases.get(), 1);
}

#[test]
fn prepared_restart_rejects_changed_selection_generation_contract_authority_head_and_anchor() {
    let case = Rc::new(Cell::new(0));
    for _ in 0..4 {
        let next_case = Rc::clone(&case);
        with_process_host_prepared_restart_fixtures(&mut move |fixture| {
            let ordinal = next_case.replace(next_case.get() + 1);
            let ProcessHostRestartFixture {
                recovery,
                admission,
                signing_authority,
                current,
                request,
                native_registry,
                evidence,
                generation,
                custody,
                sessions,
                anchor,
                session: _,
                rediscovery_generation: _,
            } = fixture;
            let selected_store = recovery.operation.context.store.clone();
            let selector = selector_from_signing_authority(signing_authority);
            let port = MemoryAuthorityStore::default();
            let host = if ordinal == 1 {
                let application = ready_application(&port, selected_store.clone(), 74);
                let deployment = unproved_deployment_for_selector(
                    &selector,
                    selected_store.clone(),
                    application,
                );
                ResidentProcessHost::new(
                    unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                    vec![deployment],
                    NonZeroUsize::new(1).unwrap(),
                    ResidentOperationHost::new(task_state()),
                )
                .unwrap()
            } else {
                host_with_generation(&port, &selector, &selected_store, generation)
            };
            if ordinal == 0 {
                let selected = custody
                    .attempt_selection()
                    .expect("prepared fixture selection");
                custody.replace_attempt_selection(ControlRecord::new(
                    selected.coordinate().clone(),
                    selected.schema_id().clone(),
                    selected.schema_sha256().clone(),
                    ControlRecordBytes::new(br#"{}"#.to_vec()).unwrap(),
                ));
            }
            let input = restart_input(recovery, 1024 * 1024);
            let classified = host.classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &input),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            );
            if ordinal == 0 {
                assert!(matches!(
                    classified,
                    Err(ResidentProcessOperationRestartError::Attempt(_))
                ));
                return;
            }
            if ordinal == 1 {
                assert!(matches!(
                    classified,
                    Err(ResidentProcessOperationRestartError::GenerationUnavailable)
                ));
                return;
            }
            let disposition = classified.expect("unchanged preparation classifies");
            if ordinal == 2 {
                custody.remove_current_observation();
            } else if ordinal == 3 {
                anchor.make_unavailable();
            }
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let recovery_current =
                current.at_time(UnixTimeV1::new(current.now.get() + 10).unwrap());
            let foreign_signing = (ordinal == 4)
                .then(|| owner_native_signing_authority(selector.principal_id.clone()));
            let dispatch_signing = if ordinal == 4 {
                foreign_signing
                    .as_ref()
                    .expect("foreign observation signer")
            } else {
                signing_authority
            };
            let observation_ref = if ordinal == 7 {
                match &disposition {
                    ResidentProcessOperationRestartDisposition::NativePrepared(attempt)
                    | ResidentProcessOperationRestartDisposition::ProviderPrepared(attempt) => {
                        attempt.start.control.observation().selected().0.clone()
                    }
                    _ => panic!("prepared disposition"),
                }
            } else {
                record_ref("restart/rejected-dispatched")
            };
            let input = ResidentProcessPreparedDispatchInput {
                current: &recovery_current,
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority: dispatch_signing,
                },
                observation_ref,
                maximum_selection_bytes: 1024 * 1024,
                maximum_record_bytes: 1024 * 1024,
            };
            let signs = Cell::new(0);
            let contracts = if ordinal == 6 {
                crate::session_admission_fixture::substituted_local_start_contract_set()
            } else {
                crate::session_admission_fixture::local_start_contract_set()
            };
            let rejected = match &disposition {
                ResidentProcessOperationRestartDisposition::NativePrepared(attempt) => host
                    .recover_native_prepared_dispatch(
                        attempt,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions,
                            anchor,
                        },
                        input,
                        native_registry,
                        &contracts,
                        &mut ProvingAuthority,
                        |_, _, _, _| Ok(()),
                        |_, _, _| {
                            signs.set(signs.get() + 1);
                            Ok(signature())
                        },
                    )
                    .is_err(),
                ResidentProcessOperationRestartDisposition::ProviderPrepared(attempt) => {
                    let OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::DispatchPrepared { attempt_id, .. },
                    } = attempt.start.control.observation().selected().1.state()
                    else {
                        panic!("provider fixture preparation")
                    };
                    let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
                    let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
                    let kind = BoundedResidentTextV1::parse(if ordinal == 5 {
                        "host_compute_substitute"
                    } else {
                        "host_compute"
                    })
                    .unwrap();
                    let version = BoundedResidentTextV1::parse("1").unwrap();
                    let binding = attempt
                        .start
                        .control
                        .operation()
                        .operation()
                        .handle()
                        .1
                        .request_binding();
                    assert_eq!(
                        &request.local_request_binding().expect("request binding"),
                        binding
                    );
                    host.recover_provider_prepared_dispatch(
                        attempt,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions,
                            anchor,
                        },
                        input,
                        provider_admission(ProviderAdmissionFixture {
                            binding,
                            attempt_id,
                            signing: signing_authority,
                            result_schema: &result_schema,
                            failure_schema: &failure_schema,
                            contracts: &contracts,
                            kind: &kind,
                            version: &version,
                        }),
                        &mut ProvingAuthority,
                        |_, _, _, _| Ok(()),
                        |_, _, _| {
                            signs.set(signs.get() + 1);
                            Ok(signature())
                        },
                    )
                    .is_err()
                }
                _ => panic!("prepared disposition"),
            };
            assert!(rejected, "hostile recovery case {ordinal}");
            assert_eq!(signs.get(), 0);
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.operations.outstanding_task_reservations(), 0);
        });
    }
    assert_eq!(case.get(), 8);
}
