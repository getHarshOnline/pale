//! Exercise service scheduling after native-response classification.
//! Protected response decoding has separate application and `SQLite` tests.

use super::*;
use crate::resident::operation_dispatch::hook_response::{
    recovery::{RecoveredHookResponsePhase, RecoveredNativeHookResponse},
    response_record,
};
use crate::resident::operation_observation_custody::tests::local_recovery::with_process_host_local_cancellation_restart_fixtures;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    Expired,
    NotExpired,
    Dispatched,
    Cancelled,
    AnchorFailure,
}

/// Supply the already-classified response at the service boundary. The start
/// retains the fixture's real protected head; these tests do not admit a frame.
fn classified_response(
    start: restart::ResidentProcessRecoveredOperationStart,
    phase: RecoveredHookResponsePhase,
) -> restart::ResidentProcessOperationRestartDisposition {
    let operation = start.control.operation().operation();
    let evidence = ImmutableRecordReferenceV1 {
        record_ref: record_ref("fixture/hook-permission"),
        record_sha256: ArtifactDigest::hash_bytes(b"fixture permission"),
    };
    let response = pale_ports::ResidentHookResponseRecord {
        schema_id: SchemaId::parse(pale_ports::PRIVATE_OPERATION_HOOK_RESPONSE_V1_SCHEMA).unwrap(),
        handle_sha256: operation
            .handle()
            .1
            .canonical_operation_handle_sha256()
            .clone(),
        native_request_sha256: start
            .control
            .operation()
            .request()
            .body()
            .payload_sha256
            .clone(),
        permission: evidence.clone(),
        disclosure: evidence,
        stdout: "saved output must not be released".into(),
        stderr: String::new(),
        exit_code: 0,
    };
    let canonical = pale_core::canon::to_canonical_string(&response).unwrap();
    let record = response_record(
        record_ref("fixture/saved-hook-response"),
        &response,
        &canonical,
    )
    .unwrap();
    restart::ResidentProcessOperationRestartDisposition::NativeHookResponse(Box::new(
        restart::ResidentProcessRecoveredNativeHookResponse {
            start,
            response: RecoveredNativeHookResponse { phase, record },
        },
    ))
}

fn run(case: Case) {
    let visited = Rc::new(Cell::new(0));
    let counted = Rc::clone(&visited);
    let mut check = move |fixture: ProcessHostRestartFixture<'_>| {
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
        let mut host = host_with_generation(&port, &selector, &selected_store, generation);
        let restart = restart_input(recovery);
        let disposition = host
            .classify_existing_operation(
                super::super::super::ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                super::super::super::ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .expect("fixture head recovers");
        let start = match disposition {
            restart::ResidentProcessOperationRestartDisposition::NativePrepared(prepared) => {
                prepared.start
            }
            restart::ResidentProcessOperationRestartDisposition::LocalCancellationRequested(
                cancelled,
            ) if case == Case::Cancelled => cancelled.start,
            _ => return,
        };
        counted.set(counted.get() + 1);
        let expires_at = *start
            .control
            .operation()
            .operation()
            .handle()
            .1
            .expires_at();
        let now = if case == Case::NotExpired {
            UnixTimeV1::new(expires_at.get() - 1).unwrap()
        } else {
            expires_at
        };
        let phase = if case == Case::Dispatched {
            RecoveredHookResponsePhase::Dispatched
        } else {
            RecoveredHookResponsePhase::Prepared
        };
        let disposition = classified_response(start, phase);
        let expected = match &disposition {
            restart::ResidentProcessOperationRestartDisposition::NativeHookResponse(value) => {
                value.response.record.clone()
            }
            _ => unreachable!(),
        };
        let clock = ExactExpiryClock::new(now);
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let resolver = SelectedStore(selected_store.clone());
        let expiry_loans = Rc::new(Cell::new(0));
        let continuation_loans = Rc::new(Cell::new(0));
        let external_work_loans = Rc::new(Cell::new(0));
        let observation_signs = Rc::new(Cell::new(0));
        let checkpoint_signs = Rc::new(Cell::new(0));
        let identity_loans = Rc::new(RefCell::new(Vec::new()));
        let mut retained = InvokeRecoveryAdapter::new(
            ExpiredInvokeInputs {
                clock: &clock,
                signing_authority,
                observation_schema: &observation_schema,
                checkpoint_schema: &checkpoint_schema,
                checkpoint_authority,
                resolver: &resolver,
                expiry_loans: Rc::clone(&expiry_loans),
                continuation_loans: Rc::clone(&continuation_loans),
                external_work_loans: Rc::clone(&external_work_loans),
                observation_signs: Rc::clone(&observation_signs),
                checkpoint_signs: Rc::clone(&checkpoint_signs),
                interruption: None,
                identity_loans,
            },
            Box::new(UnavailableResidentServicePublicationInputs),
            UnavailableResidentServiceDelegateInputs,
            (),
            (),
            1,
        );
        retained.retain_without_response(RetainedInvokePhase::Deferred(Box::new(disposition)));
        let mut append = AppendCustody::committing(custody);
        let mut append_anchor = AppendAnchor::available(anchor);
        if case == Case::AnchorFailure {
            append_anchor.fail_next_advance();
        }
        let turns = if case == Case::AnchorFailure { 2 } else { 1 };
        for index in 0..turns {
            let turn = program::ResidentProgramRootRecoveryInputPort::continue_one_retained_invoke(
                &mut retained,
                &mut host,
                super::super::super::ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append,
                    sessions,
                    anchor: &mut append_anchor,
                },
                &mut ProvingAuthority,
            );
            match case {
                Case::Expired | Case::AnchorFailure => {
                    assert_eq!(turn, program::ResidentProgramRetainedPhaseTurn::Advanced);
                    if case == Case::AnchorFailure && index == 0 {
                        assert!(matches!(
                            retained.retained.iter().next(),
                            Some(RetainedServicePhase::Invoke(
                                RetainedInvokePhase::PreDispatchExpiry(_)
                            ))
                        ));
                    } else {
                        assert_eq!(retained.retained.iter().count(), 0);
                    }
                }
                Case::NotExpired | Case::Dispatched | Case::Cancelled => {
                    assert_eq!(
                        turn,
                        if case == Case::NotExpired {
                            program::ResidentProgramRetainedPhaseTurn::Deferred
                        } else {
                            program::ResidentProgramRetainedPhaseTurn::Idle
                        }
                    );
                    let Some(RetainedServicePhase::Invoke(RetainedInvokePhase::Deferred(saved))) =
                        retained.retained.iter().next()
                    else {
                        panic!("response remains deferred")
                    };
                    let restart::ResidentProcessOperationRestartDisposition::NativeHookResponse(
                        saved,
                    ) = saved.as_ref()
                    else {
                        panic!("classification cannot change")
                    };
                    assert_eq!(saved.response.record, expected);
                }
            }
            assert_eq!(
                external_work_loans.get(),
                0,
                "expiry cannot borrow an executor"
            );
            assert_eq!(host.operations.outstanding_task_reservations(), 0);
        }
        let expires = matches!(case, Case::Expired | Case::AnchorFailure);
        let examines = !matches!(case, Case::Dispatched | Case::Cancelled);
        assert_eq!(expiry_loans.get(), usize::from(examines));
        assert_eq!(clock.reads(), usize::from(examines));
        assert_eq!(observation_signs.get(), usize::from(expires));
        assert_eq!(checkpoint_signs.get(), usize::from(expires));
        assert_eq!(
            continuation_loans.get(),
            usize::from(case == Case::AnchorFailure)
        );
        assert_eq!(
            append.counts().0,
            usize::from(expires),
            "continuation cannot re-commit"
        );
    };
    if case == Case::Cancelled {
        with_process_host_local_cancellation_restart_fixtures(&mut check);
    } else {
        with_process_host_prepared_restart_fixtures(&mut check);
    }
    assert!(visited.get() > 0, "the fixture exercised {case:?}");
}

#[test]
fn recovered_hook_response_preparation_expires_without_dispatch() {
    run(Case::Expired);
    run(Case::AnchorFailure);
}

#[test]
fn recovered_hook_response_other_states_remain_deferred() {
    run(Case::NotExpired);
    run(Case::Dispatched);
    run(Case::Cancelled);
}

#[test]
fn deferred_expiry_requeue_preserves_capacity_and_allows_later_work() {
    let mut retained = BoundedRetainedPhases::new(2);
    assert!(matches!(
        retained.retain(1, "waiting for expiry"),
        Ok(RetainedPhaseAdmission::Retained)
    ));
    assert!(matches!(
        retained.retain(2, "ready recovery"),
        Ok(RetainedPhaseAdmission::Retained)
    ));
    let (reservation, phase) = retained.take_first_where(|_| true).unwrap();
    let key = reservation.key;
    retained.retain_reserved(reservation, phase).unwrap();
    retained.move_to_back(&key);
    assert!(
        retained.is_full(),
        "requeue must not free the protected reservation"
    );
    assert!(matches!(
        retained.retain(1, "replacement"),
        Ok(RetainedPhaseAdmission::KeyConflict(_))
    ));
    let (ready, phase) = retained.take_first_where(|_| true).unwrap();
    assert_eq!(phase, "ready recovery");
    assert!(retained.release(ready));
    let (_, original) = retained.take_first_where(|_| true).unwrap();
    assert_eq!(original, "waiting for expiry");
}
