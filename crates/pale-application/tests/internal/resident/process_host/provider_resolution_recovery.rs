//! Exact first-attempt recovery from a retained provider resolution.

use super::provider_recovery::{AppendAnchor, AppendCustody};
use super::restart::{association, host_with_generation, restart_input};
use super::start::CountingAuthority;
use super::*;
use crate::local_operation_start_fixture::{record_ref, schema_digest, signature};
use crate::provider_invocation::{
    ProviderInvocationAttemptAdmission, ProviderInvocationSafeFailureProof,
};
use crate::resident::operation_dispatch::RecoveredProviderAttemptMeaning;
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_process_host_foreign_provider_predecessor_restart_fixtures,
    with_process_host_native_provider_resolved_restart_fixture,
    with_process_host_provider_resolved_restart_fixture, ProcessHostRestartFixture,
};
use crate::resident::process_host::provider_resolution_recovery::*;
use crate::resident::process_host::restart::*;
use crate::resident::{
    OperationObservationAuthorityStoreRejection, OperationObservationAuthorityStoreResolver,
};
use crate::session_admission_fixture::ProvingAuthority;
use pale_core::records::{
    BoundedResidentTextV1, CompiledGenerationDigest,
    ControlCheckpointSigningAuthorityAdmissionContextV1, OperationActivePhaseV1,
    OperationObservationStateV1, SignedResidentAdmissionContextV1, UnixTimeV1,
    ADAPTER_FAILURE_V1_SCHEMA_ID, CAPABILITY_RESULT_V1_SCHEMA_ID,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID, OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    AccumulatorPreparationLimits, AuthorityStoreCoordinate, CryptographicRandomSourceError,
    CryptographicRandomSourcePort,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct SelectedStore {
    coordinate: AuthorityStoreCoordinate,
    reject: bool,
}

impl OperationObservationAuthorityStoreResolver for SelectedStore {
    fn resolve(
        &self,
        _: &pale_core::records::OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        if self.reject {
            return Err(OperationObservationAuthorityStoreRejection);
        }
        Ok(self.coordinate.clone())
    }
}

struct OrderedRandom {
    calls: AtomicUsize,
    fail: bool,
}

impl OrderedRandom {
    fn succeeding() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            fail: false,
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn failing() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            fail: true,
        }
    }
}

struct ConstantRandom(AtomicUsize);

impl CryptographicRandomSourcePort for ConstantRandom {
    fn fill_bytes(&self, destination: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        destination.fill(1);
        Ok(())
    }
}

impl CryptographicRandomSourcePort for OrderedRandom {
    fn fill_bytes(&self, destination: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if self.fail {
            return Err(CryptographicRandomSourceError::unavailable());
        }
        destination.fill(u8::try_from(call).expect("bounded provider-attempt draw"));
        Ok(())
    }
}

fn accumulator_limits() -> AccumulatorPreparationLimits {
    AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000)
        .expect("bounded provider-attempt accumulator")
}

#[test]
fn provider_resolved_allocation_rechecks_physical_head_and_anchor_before_entropy() {
    let cases = Rc::new(Cell::new(0));
    let counted_cases = Rc::clone(&cases);
    with_process_host_provider_resolved_restart_fixture(&mut move |fixture| {
        let case = counted_cases.replace(counted_cases.get() + 1);
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            session,
            current,
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
        let restart = restart_input(recovery, 1 << 20);
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
            .expect("the exact provider resolution classifies before the hostile change");
        let ResidentProcessOperationRestartDisposition::ProviderResolved(recovered) = disposition
        else {
            panic!("provider resolution has one restart owner")
        };
        if case % 2 == 0 {
            custody.remove_current_observation();
        } else {
            anchor.make_unavailable();
        }

        let now = current.at_time(UnixTimeV1::new(current.now.get() + 10).unwrap());
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
        let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
        let contracts = crate::session_admission_fixture::local_start_contract_set();
        let kind = BoundedResidentTextV1::parse("host_compute").unwrap();
        let version = BoundedResidentTextV1::parse("1").unwrap();
        let resolver = SelectedStore {
            coordinate: selected_store.clone(),
            reject: false,
        };
        let input = ResidentProcessProviderResolvedInput {
            session,
            current: &now,
            meaning: RecoveredProviderAttemptMeaning {
                implementation_kind: &kind,
                implementation_version: &version,
                admission: ProviderInvocationAttemptAdmission {
                    result_schema_sha256: &result_schema,
                    result_signing_authority: signing_authority,
                    contract_set: &contracts,
                    failure_schema_sha256: &failure_schema,
                    failure_signing_authority: signing_authority,
                    safe_failure_proof: ProviderInvocationSafeFailureProof::BeforeDispatch,
                },
            },
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &observation_schema,
                signing_authority,
            },
            checkpoint_schema: &checkpoint_schema,
            checkpoint_authority,
            resolve_store: &resolver,
            accumulator_limits: accumulator_limits(),
            maximum_selection_bytes: 1 << 20,
            maximum_record_bytes: 1 << 20,
        };
        let random = OrderedRandom::succeeding();
        let rejected = host.allocate_provider_resolved_attempt(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &input,
            &mut ProvingAuthority,
            &random,
        );
        assert!(matches!(
            rejected,
            Err(ResidentProcessProviderResolvedPreparationError::Selection(
                ResidentProcessHostSelectionError::ProviderOperationAuthority
            ))
        ));
        assert_eq!(random.calls(), 0);
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
    });
    assert_eq!(cases.get(), 3);
}

#[test]
fn provider_resolved_allocation_rejects_foreign_predecessor_before_entropy_or_effects() {
    let cases = Rc::new(Cell::new(0));
    let counted_cases = Rc::clone(&cases);
    with_process_host_foreign_provider_predecessor_restart_fixtures(&mut move |fixture| {
        counted_cases.set(counted_cases.get() + 1);
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            session,
            current,
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
        let restart = restart_input(recovery, 1 << 20);
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
            .expect("provider resolution retains its admitted historical predecessor");
        let ResidentProcessOperationRestartDisposition::ProviderResolved(recovered) = disposition
        else {
            panic!("provider resolution has one restart owner")
        };

        let now = current.at_time(UnixTimeV1::new(current.now.get() + 10).unwrap());
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
        let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
        let contracts = crate::session_admission_fixture::local_start_contract_set();
        let kind = BoundedResidentTextV1::parse("host_compute").unwrap();
        let version = BoundedResidentTextV1::parse("1").unwrap();
        let resolver = SelectedStore {
            coordinate: selected_store.clone(),
            reject: false,
        };
        let input = ResidentProcessProviderResolvedInput {
            session,
            current: &now,
            meaning: RecoveredProviderAttemptMeaning {
                implementation_kind: &kind,
                implementation_version: &version,
                admission: ProviderInvocationAttemptAdmission {
                    result_schema_sha256: &result_schema,
                    result_signing_authority: signing_authority,
                    contract_set: &contracts,
                    failure_schema_sha256: &failure_schema,
                    failure_signing_authority: signing_authority,
                    safe_failure_proof: ProviderInvocationSafeFailureProof::BeforeDispatch,
                },
            },
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &observation_schema,
                signing_authority,
            },
            checkpoint_schema: &checkpoint_schema,
            checkpoint_authority,
            resolve_store: &resolver,
            accumulator_limits: accumulator_limits(),
            maximum_selection_bytes: 1 << 20,
            maximum_record_bytes: 1 << 20,
        };
        let random = OrderedRandom::succeeding();
        let rejected = host.allocate_provider_resolved_attempt(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &input,
            &mut ProvingAuthority,
            &random,
        );
        assert!(matches!(
            rejected,
            Err(ResidentProcessProviderResolvedPreparationError::Selection(
                ResidentProcessHostSelectionError::ProviderObservationAuthority
            ))
        ));
        assert_eq!(random.calls(), 0);
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
    });
    assert_eq!(cases.get(), 2);
}

#[test]
fn provider_resolved_restart_retains_one_fresh_dispatch_preparation_without_dispatch() {
    let cases = Rc::new(Cell::new(0));
    let counted_cases = Rc::clone(&cases);
    with_process_host_provider_resolved_restart_fixture(&mut move |fixture| {
        let case = counted_cases.replace(counted_cases.get() + 1);
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            session,
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
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let restart = restart_input(recovery, 1 << 20);
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
            .expect("exact provider resolution classifies");
        let ResidentProcessOperationRestartDisposition::ProviderResolved(mut recovered) =
            disposition
        else {
            panic!("provider-resolved head has one private restart disposition")
        };
        let selected = recovered.start.control.observation().selected_head();
        let selected_ref = selected.0.clone();
        assert!(matches!(
            selected.1.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ProviderResolved { provider_binding }
            } if provider_binding == &recovered.provider_binding
        ));
        let predecessor = recovered
            .start
            .control
            .observation()
            .selected_predecessor()
            .expect("provider resolution retains its exact predecessor");
        if recovered
            .start
            .control
            .operation()
            .operation()
            .historical_operation_lease()
            .evidence_obligations()
            .is_empty()
        {
            assert!(matches!(
                predecessor.1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving
                }
            ));
        } else {
            assert!(matches!(
                predecessor.1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ContextBound { .. }
                }
            ));
        }

        let now = current.at_time(UnixTimeV1::new(current.now.get() + 10).unwrap());
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
        let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
        let contracts = crate::session_admission_fixture::local_start_contract_set();
        let kind = BoundedResidentTextV1::parse("host_compute").unwrap();
        let version = BoundedResidentTextV1::parse("1").unwrap();
        let resolver = SelectedStore {
            coordinate: selected_store.clone(),
            reject: case == 0,
        };
        let meaning = RecoveredProviderAttemptMeaning {
            implementation_kind: &kind,
            implementation_version: &version,
            admission: ProviderInvocationAttemptAdmission {
                result_schema_sha256: &result_schema,
                result_signing_authority: signing_authority,
                contract_set: &contracts,
                failure_schema_sha256: &failure_schema,
                failure_signing_authority: signing_authority,
                safe_failure_proof: ProviderInvocationSafeFailureProof::BeforeDispatch,
            },
        };
        let input = ResidentProcessProviderResolvedInput {
            session,
            current: &now,
            meaning,
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &observation_schema,
                signing_authority,
            },
            checkpoint_schema: &checkpoint_schema,
            checkpoint_authority,
            resolve_store: &resolver,
            accumulator_limits: accumulator_limits(),
            maximum_selection_bytes: 1 << 20,
            maximum_record_bytes: 1 << 20,
        };
        let random = OrderedRandom::succeeding();
        let mut authority = ProvingAuthority;
        let foreign_store = store("foreign-provider-store", "foreign-provider-incarnation");
        let rejected = host.allocate_provider_resolved_attempt(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &foreign_store,
                custody,
                sessions,
                anchor,
            },
            &input,
            &mut authority,
            &random,
        );
        assert!(matches!(
            rejected,
            Err(ResidentProcessProviderResolvedPreparationError::Selection(
                ResidentProcessHostSelectionError::AccessStoreAssociation
            ))
        ));
        assert_eq!(random.calls(), 0);

        let exact_provider_binding = recovered.provider_binding.clone();
        recovered.provider_binding.provider_binding_id = "foreign-provider-binding".into();
        let rejected = host.allocate_provider_resolved_attempt(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &input,
            &mut authority,
            &random,
        );
        assert!(matches!(
            rejected,
            Err(ResidentProcessProviderResolvedPreparationError::Selection(
                ResidentProcessHostSelectionError::ProviderOperationAuthority
            ))
        ));
        assert_eq!(random.calls(), 0);
        recovered.provider_binding = exact_provider_binding;

        let expired = current.at_time(request.expires_at());
        let expired_input = ResidentProcessProviderResolvedInput {
            current: &expired,
            ..input
        };
        let rejected = host.allocate_provider_resolved_attempt(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &expired_input,
            &mut authority,
            &random,
        );
        assert!(matches!(
            rejected,
            Err(ResidentProcessProviderResolvedPreparationError::Selection(
                ResidentProcessHostSelectionError::ProviderOperationAuthority
            ))
        ));
        assert_eq!(random.calls(), 0);

        let exact_generation = recovered.start.generation_digest.clone();
        recovered.start.generation_digest =
            CompiledGenerationDigest::parse("f".repeat(64)).unwrap();
        let rejected = host.allocate_provider_resolved_attempt(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &input,
            &mut authority,
            &random,
        );
        assert!(matches!(
            rejected,
            Err(ResidentProcessProviderResolvedPreparationError::Selection(
                ResidentProcessHostSelectionError::ProviderOperationAuthority
            ))
        ));
        assert_eq!(random.calls(), 0);
        recovered.start.generation_digest = exact_generation;

        let foreign_signing = owner_native_signing_authority(selector.principal_id.clone());
        let foreign_meaning = RecoveredProviderAttemptMeaning {
            implementation_kind: &kind,
            implementation_version: &version,
            admission: ProviderInvocationAttemptAdmission {
                result_schema_sha256: &result_schema,
                result_signing_authority: &foreign_signing,
                contract_set: &contracts,
                failure_schema_sha256: &failure_schema,
                failure_signing_authority: &foreign_signing,
                safe_failure_proof: ProviderInvocationSafeFailureProof::BeforeDispatch,
            },
        };
        let foreign_admission = ResidentProcessProviderResolvedInput {
            meaning: foreign_meaning,
            ..input
        };
        let rejected = host.allocate_provider_resolved_attempt(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &foreign_admission,
            &mut authority,
            &random,
        );
        assert!(matches!(
            rejected,
            Err(ResidentProcessProviderResolvedPreparationError::Selection(
                ResidentProcessHostSelectionError::ProviderOperationAuthority
            ))
        ));
        assert_eq!(random.calls(), 0);

        let foreign_checkpoint_ref = record_ref("authority/foreign-checkpoint");
        let foreign_checkpoint = ControlCheckpointSigningAuthorityAdmissionContextV1 {
            authority_ref: &foreign_checkpoint_ref,
            authority_sha256: checkpoint_authority.authority_sha256,
            principal_id: checkpoint_authority.principal_id,
            control_store_id: checkpoint_authority.control_store_id,
            control_incarnation_id: checkpoint_authority.control_incarnation_id,
            checkpoint_domain_id: checkpoint_authority.checkpoint_domain_id,
            checkpoint_profile_ref: checkpoint_authority.checkpoint_profile_ref,
            checkpoint_profile_sha256: checkpoint_authority.checkpoint_profile_sha256,
            subject_key_id: checkpoint_authority.subject_key_id,
            subject_key_epoch: checkpoint_authority.subject_key_epoch,
        };
        let foreign_checkpoint_input = ResidentProcessProviderResolvedInput {
            checkpoint_authority: &foreign_checkpoint,
            ..input
        };
        let rejected = host.allocate_provider_resolved_attempt(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &foreign_checkpoint_input,
            &mut authority,
            &random,
        );
        assert!(matches!(
            rejected,
            Err(ResidentProcessProviderResolvedPreparationError::Selection(
                ResidentProcessHostSelectionError::ProviderOperationAuthority
            ))
        ));
        assert_eq!(random.calls(), 0);

        let mut rejecting_authority = CountingAuthority::rejecting_signature();
        let rejected = host.allocate_provider_resolved_attempt(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &input,
            &mut rejecting_authority,
            &random,
        );
        assert!(matches!(
            rejected,
            Err(ResidentProcessProviderResolvedPreparationError::Selection(
                ResidentProcessHostSelectionError::ProviderCurrentAuthority
            ))
        ));
        assert_eq!(rejecting_authority.calls, 1);
        assert_eq!(random.calls(), 0);

        let failing_random = OrderedRandom::failing();
        let rejected = host.allocate_provider_resolved_attempt(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &input,
            &mut authority,
            &failing_random,
        );
        assert!(matches!(
            rejected,
            Err(ResidentProcessProviderResolvedPreparationError::Random(_))
        ));
        assert_eq!(failing_random.calls(), 1);

        let repeated_random = ConstantRandom(AtomicUsize::new(0));
        let rejected = host.allocate_provider_resolved_attempt(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &input,
            &mut authority,
            &repeated_random,
        );
        assert!(matches!(
            rejected,
            Err(ResidentProcessProviderResolvedPreparationError::RandomDrawsExhausted)
        ));
        assert_eq!(repeated_random.0.load(Ordering::SeqCst), 10);

        let allocation = host
            .allocate_provider_resolved_attempt(
                &recovered,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &input,
                &mut authority,
                &random,
            )
            .expect("all immutable recovery joins precede allocation");
        assert_eq!(random.calls(), 5);
        let observation_signs = Cell::new(0);
        let preparation = host
            .prepare_provider_resolved_attempt(
                &recovered,
                &input,
                allocation,
                &mut authority,
                |_, _, _, _| Ok(()),
                |_, _, _| {
                    observation_signs.set(observation_signs.get() + 1);
                    Ok(signature())
                },
            )
            .expect("the exact provider binding rebuilds one private selection");
        assert_eq!(observation_signs.get(), 1);

        let mut append = AppendCustody::committing(custody);
        let mut append_anchor = AppendAnchor::available(anchor);
        if case == 2 {
            append_anchor.fail_next_advance();
        } else if case == 1 {
            append.fail_next_commit(pale_ports::ControlStoreErrorKind::OutcomeUnknown);
            append.push_read_back(pale_ports::OperationCustodyReadBackOutcome::Predecessor);
            append.fail_next_read_back(pale_ports::ControlStoreErrorKind::Storage);
            append.push_read_back(pale_ports::OperationCustodyReadBackOutcome::ExactSuccessor);
        }
        let checkpoint_signs = Cell::new(0);
        let retained = host.retain_provider_resolved_attempt(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &input,
            preparation,
            |_, _, _| {
                checkpoint_signs.set(checkpoint_signs.get() + 1);
                Ok(signature())
            },
        );
        let completed = if case == 0 {
            let Err(ResidentProcessProviderResolvedRetentionFailure::Custody(
                crate::resident::operation_dispatch::RecoveredProviderAttemptCustodyFailure::Prepared {
                    attempt,
                    checkpoint,
                    failure,
                },
            )) = retained
            else {
                panic!("pre-commit rejection preserves the exact signed preparation")
            };
            assert!(matches!(
                failure.as_ref(),
                crate::resident::OperationAppendFailure::Operation(_)
                    | crate::resident::OperationAppendFailure::Projection(_)
            ));
            assert!(matches!(
                attempt.observation().1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::DispatchPrepared { .. }
                }
            ));
            assert!(checkpoint
                .reference
                .as_str()
                .starts_with("operation-checkpoint-record:v1:"));
            assert_eq!(checkpoint_signs.get(), 0);
            assert_eq!(append.counts(), (0, 0));
            assert_eq!(append_anchor.counts(), (0, 0));
            assert_eq!(host.provider_routes.reserved, 0);
            assert_eq!(host.provider_routes.checked_out, 0);
            assert!(host.provider_routes.active.is_empty());
            return;
        } else if case == 2 {
            let Err(ResidentProcessProviderResolvedRetentionFailure::Custody(
                crate::resident::operation_dispatch::RecoveredProviderAttemptCustodyFailure::Anchor(
                    pending,
                ),
            )) = retained
            else {
                panic!("anchor interruption preserves exact committed custody")
            };
            match host.continue_provider_resolved_attempt_anchor(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append,
                    sessions,
                    anchor: &mut append_anchor,
                },
                *pending,
            ) {
                Ok(result) => result,
                Err(_) => panic!("the exact deployment owns anchor continuation"),
            }
            .unwrap_or_else(|_| panic!("anchor continuation completes without rebuilding"))
        } else {
            let Err(ResidentProcessProviderResolvedRetentionFailure::Custody(
                crate::resident::operation_dispatch::RecoveredProviderAttemptCustodyFailure::Commit(
                    failure,
                ),
            )) = retained
            else {
                panic!("unknown database outcome preserves one exact candidate")
            };
            let pending = match host.reconcile_provider_resolved_attempt(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append,
                    sessions,
                    anchor: &mut append_anchor,
                },
                *failure,
            ) {
                Ok(result) => result,
                Err(_) => panic!("the exact deployment owns commit reconciliation"),
            }
            .unwrap_or_else(|_| panic!("exact read-back resolves the retained candidate"));
            match host.continue_provider_resolved_attempt_anchor(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append,
                    sessions,
                    anchor: &mut append_anchor,
                },
                pending,
            ) {
                Ok(result) => result,
                Err(_) => panic!("the exact deployment owns anchor continuation"),
            }
            .unwrap_or_else(|_| panic!("anchor continuation completes without rebuilding"))
        };
        let OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::DispatchPrepared {
                    attempt_id,
                    evidence,
                },
        } = completed.observation().1.state()
        else {
            panic!("recovery stops at DispatchPrepared")
        };
        assert!(attempt_id.as_str().starts_with("provider-attempt:v1:"));
        assert!(evidence
            .record_ref
            .as_str()
            .starts_with("provider-attempt-selection-record:v1:"));
        assert_eq!(
            completed.observation().1.predecessor_ref().as_ref(),
            Some(&selected_ref)
        );
        assert!(completed
            .checkpoint()
            .0
            .as_str()
            .starts_with("operation-checkpoint-record:v1:"));
        assert_eq!(completed.custody().candidate().store(), &selected_store);
        assert_eq!(checkpoint_signs.get(), 1);
        assert_eq!(append.counts(), if case == 2 { (1, 1) } else { (1, 3) });
        assert_eq!(
            append_anchor.counts(),
            if case == 2 { (2, 1) } else { (1, 1) }
        );
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
        assert_eq!(request, recovered.start.control.operation().request());
    });
    assert_eq!(cases.get(), 3);
}

#[test]
fn native_provider_resolved_head_never_acquires_the_provider_restart_owner() {
    with_process_host_native_provider_resolved_restart_fixture(&mut |fixture| {
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
        let restart = restart_input(recovery, 1 << 20);
        let disposition = host.classify_existing_operation(
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
        );
        assert!(matches!(
            disposition,
            Err(ResidentProcessOperationRestartError::State)
        ));
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
    });
}
