//! Real built-in execution through the application custody phases.
//! Signing, authority callbacks and finite accumulator responses are explicit
//! test models, not proof of native storage, Merkle computation or cryptography.

use super::*;
use crate::actions::NativeActionRegistry;
use crate::local_operation_start_fixture::{issue_observation, signature, store_coordinate};
use crate::resident::operation_dispatch::{
    prepare_native_operation_attempt, NativeOperationAttemptInput, NativeOperationCompletionInput,
    NativeOperationDispatchContext, NativeOperationExecutionContext, NativeOperationOutcome,
    OperationAttemptContext, OperationAttemptError,
};
use crate::resident::operation_observation_custody::tests::native_execution as custody;
use crate::resident::{
    OperationCustodyDatabaseEvidence, OperationObservationAuthorityStoreRejection,
    OperationObservationAuthorityStoreResolver,
};
use crate::session_admission_fixture::{LocalStartContext, ProvingAuthority};
use pale_ports::{
    AuthorityStoreCoordinate, OperationCustodyRecord, WallClockSourceError, WallClockSourcePort,
};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "cancellation.rs"]
mod cancellation;
#[path = "failure.rs"]
mod failure;
#[path = "refusal_cancellation.rs"]
mod refusal_cancellation;
#[path = "refusal_expiry.rs"]
mod refusal_expiry;
#[path = "release_refusal.rs"]
mod release_refusal;

struct Store;

impl OperationObservationAuthorityStoreResolver for Store {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        Ok(store_coordinate())
    }
}

struct Clock {
    times: [u64; 3],
    reads: AtomicUsize,
}

impl WallClockSourcePort for Clock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        let index = self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(UnixTimeV1::new(*self.times.get(index).expect("at most three clock reads")).unwrap())
    }
}

#[derive(Clone, Copy, Debug)]
enum Case {
    Expiry(refusal_expiry::Case),
    Complete,
    UnusedReadAuthority,
    EvidenceOnly,
    Cancel,
    CancelUnusedReadAuthority,
    Inherited,
    MalformedInput,
    Replay,
    RecoveredCommit,
    WrongCompletion,
    ShiftedHead,
    ExpiredBeforeRead,
    BackwardBeforeRead,
    ExpiredAfterAuthority,
    BackwardAfterAuthority,
    LateRecording,
    HandlerFailure,
    HandlerFailureLateRecording,
    HandlerFailureReplay,
    ReleaseClockRecovery,
    ReleaseProjectionRecovery,
    ReleaseHighWater,
    ReleaseTimeFloor,
    ReleaseElapsedBudget,
    ReleaseCurrentAuthority,
    CancelRefused,
    CancelRefusedDeadline,
    CancelRefusedExpired,
    CancelRefusedExpiresAfterReads,
    CancelRefusedHighwater,
    CancelRefusedReplay,
    CancelRefusedRecovered,
    CancelRefusedChangedHead,
    CancelRefusedMissingAnchor,
}

fn journey(case: Case) {
    let (compiled, materializations, profile) = source_generation_inputs_with(|_| {}, |_| {});
    let generation = AdmittedCompiledGeneration::admit(
        compiled,
        materializations,
        crate::resident_fixture::synthetic_set_policy(),
    )
    .unwrap();
    let context = match case {
        Case::Inherited => LocalStartContext::Inherited,
        Case::UnusedReadAuthority | Case::CancelUnusedReadAuthority => {
            LocalStartContext::ReadAuthorityOnly
        }
        Case::EvidenceOnly => LocalStartContext::EvidenceOnly,
        _ => LocalStartContext::SelfContained,
    };
    local_start::with_issued_context(
        &generation,
        &profile,
        "native-shell",
        |request| {
            let descriptor = pale_contracts::schema_descriptor("action-input.v1").unwrap();
            request.body.schema_id = SchemaId::parse(descriptor.schema_id).unwrap();
            request.body.schema_sha256 = SchemaDigest::parse(descriptor.schema_sha256).unwrap();
            request.body.payload = native::input();
            if matches!(case, Case::MalformedInput) {
                request.body.payload["extra"] = serde_json::json!(true);
            }
            if matches!(
                case,
                Case::HandlerFailure
                    | Case::HandlerFailureLateRecording
                    | Case::HandlerFailureReplay
            ) {
                // Valid action schema; only the real handler rejects this lexical escape.
                request.body.payload["payload"]["paths"] = serde_json::json!(["../token=private"]);
            }
            request.body.payload_sha256 = ArtifactDigest::hash_bytes(
                canon::to_canonical_string(&request.body.payload)
                    .unwrap()
                    .as_bytes(),
            );
        },
        context,
        |operation, request, parents, initial_checkpoint, initial_record, _| {
            assert_eq!(
                parents.lease.read_set().is_empty(),
                !context.has_read_authority()
            );
            assert_eq!(
                parents.lease.evidence_obligations().is_empty(),
                !context.has_evidence_obligations()
            );
            let (handle_ref, handle) = operation.handle();
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
                canon::to_canonical_string(&initial).unwrap().as_bytes(),
                "reuse the exact initial observation retained by the start"
            );
            let selected = AttemptSelectionContext {
                operation,
                request_ref: operation.request_coordinate().record_ref(),
                request,
                session: parents.session.lease(),
                lease: parents.lease,
                generation: &generation,
            };
            assert_eq!(
                operation.request_coordinate().record_sha256().as_str(),
                request.canonical_request_sha256().as_str()
            );
            assert_eq!(
                handle.request_binding(),
                &request.local_request_binding().unwrap()
            );
            native::with_admission(selected, parents.signing, |attempt_id, admission| {
                let registry = NativeActionRegistry::new().unwrap();
                let observation = OperationObservationAdmissionContextV1 {
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &schema,
                        signing_authority: parents.signing,
                    },
                    handle_ref,
                    handle,
                    predecessor: Some((initial_record.coordinate().record_ref(), &initial)),
                };
                let prepare_signs = Cell::new(0);
                let prepared = prepare_native_operation_attempt(
                    NativeOperationAttemptInput {
                        context: OperationAttemptContext {
                            operation,
                            request,
                            session: parents.session,
                            current: parents.current,
                            lease: parents.lease,
                            context_binding: None,
                            generation: &generation,
                            selection_ref: record_ref("native-shell/selection"),
                            maximum_selection_bytes: 65_536,
                            observation_ref: record_ref("native-shell/preparation"),
                            observation,
                        },
                        registry: &registry,
                        attempt_id,
                        result_admission: admission,
                    },
                    &mut ProvingAuthority,
                    |_, _, _, _| Ok(()),
                    |_, _, _| {
                        prepare_signs.set(prepare_signs.get() + 1);
                        Ok(signature())
                    },
                );
                if matches!(case, Case::MalformedInput) {
                    match prepared {
                        Err(OperationAttemptError::NativeAction(_)) => {}
                        Err(error) => panic!("malformed input returned {error:?}"),
                        Ok(_) => panic!("malformed input produced a prepared action"),
                    }
                    assert_eq!(prepare_signs.get(), 0);
                    return;
                }
                if context.has_evidence_obligations() {
                    assert!(matches!(
                        prepared,
                        Err(OperationAttemptError::ContextRequired)
                    ));
                    assert_eq!(prepare_signs.get(), 0);
                    return;
                }
                let prepared = prepared.unwrap();
                assert_eq!(prepare_signs.get(), 1);
                let append = prepared.prepare_append(&Store).unwrap();
                assert!(matches!(
                    append.associated_record(),
                    Some(OperationCustodyRecord::AttemptSelection(_))
                ));
                let (preparation_done, preparation_checkpoint) = custody::complete(
                    append,
                    request,
                    (&record_ref("operation/checkpoint-1"), initial_checkpoint),
                    OperationCustodyDatabaseEvidence::New,
                    63,
                );
                let dispatch_current = parents.current.at_time(UnixTimeV1::new(64).unwrap());
                if matches!(case, Case::Cancel | Case::CancelUnusedReadAuthority) {
                    cancellation::check(
                        &prepared,
                        crate::resident::NativeOperationCancellationInput {
                            completed: &preparation_done,
                            current: &dispatch_current,
                            signing: observation.signed,
                            diagnostic_ref: record_ref("native-shell/cancel-diagnostic"),
                            observation_ref: record_ref("native-shell/cancelled"),
                            maximum_diagnostic_bytes: 65_536,
                            resolve_store: &Store,
                        },
                        request,
                        &preparation_checkpoint,
                        context.has_evidence_obligations(),
                    );
                    return;
                }
                let dispatch_signs = Cell::new(0);
                let dispatch = prepared.prepare_dispatch(
                    NativeOperationDispatchContext {
                        completed: &preparation_done,
                        current: &dispatch_current,
                        signing: observation.signed,
                        observation_ref: record_ref("native-shell/dispatch"),
                        resolve_store: &Store,
                    },
                    &mut ProvingAuthority,
                    |_, _, _, _| Ok(()),
                    |_, _, _| {
                        dispatch_signs.set(dispatch_signs.get() + 1);
                        Ok(signature())
                    },
                );
                let dispatch = dispatch.unwrap();
                assert_eq!(dispatch_signs.get(), 1);
                assert!(matches!(
                    dispatch.observation().1.state(),
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::Dispatched { .. }
                    }
                ));
                let database = match case {
                    Case::Replay
                    | Case::HandlerFailureReplay
                    | Case::CancelRefusedReplay
                    | Case::Expiry(refusal_expiry::Case::Replay) => {
                        OperationCustodyDatabaseEvidence::ExactReplay
                    }
                    Case::RecoveredCommit
                    | Case::CancelRefusedRecovered
                    | Case::Expiry(refusal_expiry::Case::Recovered) => {
                        OperationCustodyDatabaseEvidence::RecoveredExactAfterAmbiguousCommit
                    }
                    _ => OperationCustodyDatabaseEvidence::New,
                };
                let (dispatch_done, dispatch_checkpoint) = custody::complete(
                    dispatch.prepare_append(&Store).unwrap(),
                    request,
                    (
                        preparation_done
                            .candidate()
                            .checkpoint()
                            .successor()
                            .coordinate()
                            .record_ref(),
                        &preparation_checkpoint,
                    ),
                    database,
                    64,
                );
                let times = match case {
                    Case::ExpiredBeforeRead => [80, 80, 80],
                    Case::BackwardBeforeRead => [63, 65, 66],
                    Case::ExpiredAfterAuthority => [65, 66, 80],
                    Case::BackwardAfterAuthority => [65, 66, 65],
                    _ => [65, 66, 67],
                };
                let selection_record = preparation_done
                    .candidate()
                    .records()
                    .iter()
                    .find_map(|record| match record {
                        OperationCustodyRecord::AttemptSelection(record) => Some(record.clone()),
                        _ => None,
                    })
                    .unwrap();
                let dispatch_done = if matches!(case, Case::WrongCompletion) {
                    preparation_done
                } else {
                    dispatch_done
                };
                let clock = Clock {
                    times,
                    reads: AtomicUsize::new(0),
                };
                let dispatch_checkpoint_ref = dispatch_done
                    .candidate()
                    .checkpoint()
                    .successor()
                    .coordinate()
                    .record_ref()
                    .clone();
                if let Case::Expiry(expiry) = case {
                    custody::with_reads(
                        operation,
                        dispatch_done,
                        matches!(expiry, refusal_expiry::Case::ChangedHead),
                        |completed, store, anchor| {
                            refusal_expiry::check(
                                expiry,
                                dispatch,
                                completed,
                                NativeOperationExecutionContext {
                                    current: &dispatch_current,
                                    resolve_store: &Store,
                                    clock: &clock,
                                    maximum_record_bytes: 65_536,
                                },
                                observation.signed,
                                store,
                                anchor,
                            );
                        },
                    );
                    return;
                }
                if matches!(
                    case,
                    Case::CancelRefused
                        | Case::CancelRefusedDeadline
                        | Case::CancelRefusedExpired
                        | Case::CancelRefusedExpiresAfterReads
                        | Case::CancelRefusedHighwater
                        | Case::CancelRefusedReplay
                        | Case::CancelRefusedRecovered
                        | Case::CancelRefusedChangedHead
                        | Case::CancelRefusedMissingAnchor
                ) {
                    custody::with_reads(
                        operation,
                        dispatch_done,
                        matches!(case, Case::CancelRefusedChangedHead),
                        |completed, store, anchor| {
                            refusal_cancellation::check(
                                case,
                                dispatch,
                                completed,
                                NativeOperationExecutionContext {
                                    current: &dispatch_current,
                                    resolve_store: &Store,
                                    clock: &clock,
                                    maximum_record_bytes: 65_536,
                                },
                                observation.signed,
                                store,
                                anchor,
                            );
                        },
                    );
                    return;
                }
                let executed = custody::with_reads(
                    operation,
                    dispatch_done,
                    matches!(case, Case::ShiftedHead),
                    |dispatch_done, store, anchor| {
                        release_refusal::invoke(
                            case,
                            dispatch,
                            dispatch_done,
                            NativeOperationExecutionContext {
                                current: &dispatch_current,
                                resolve_store: &Store,
                                clock: &clock,
                                maximum_record_bytes: 65_536,
                            },
                            store,
                            anchor,
                        )
                    },
                );
                match case {
                    Case::Replay
                    | Case::HandlerFailureReplay
                    | Case::RecoveredCommit
                    | Case::WrongCompletion
                    | Case::ShiftedHead => {
                        assert!(matches!(
                            executed.as_ref().err().unwrap().reason(),
                            OperationAttemptError::Custody(_)
                        ));
                        assert_eq!(clock.reads.load(Ordering::SeqCst), 1);
                        return;
                    }
                    Case::ExpiredBeforeRead => {
                        assert!(executed.is_err());
                        assert_eq!(clock.reads.load(Ordering::SeqCst), 1);
                        return;
                    }
                    Case::BackwardBeforeRead => {
                        assert!(matches!(
                            executed.as_ref().err().unwrap().reason(),
                            OperationAttemptError::TimeOrder
                        ));
                        assert_eq!(clock.reads.load(Ordering::SeqCst), 1);
                        return;
                    }
                    Case::ExpiredAfterAuthority => {
                        assert!(executed.is_err());
                        assert_eq!(clock.reads.load(Ordering::SeqCst), 3);
                        return;
                    }
                    Case::BackwardAfterAuthority => {
                        assert!(matches!(
                            executed.as_ref().err().unwrap().reason(),
                            OperationAttemptError::TimeOrder
                        ));
                        assert_eq!(clock.reads.load(Ordering::SeqCst), 3);
                        return;
                    }
                    Case::ReleaseHighWater
                    | Case::ReleaseTimeFloor
                    | Case::ReleaseElapsedBudget => {
                        assert!(executed.is_err());
                        return;
                    }
                    _ => {}
                }
                let executed = executed.unwrap();
                assert_eq!(clock.reads.load(Ordering::SeqCst), 3);
                if matches!(
                    case,
                    Case::HandlerFailure | Case::HandlerFailureLateRecording
                ) {
                    let NativeOperationOutcome::Failed(failed) = executed.outcome() else {
                        panic!("the admitted path reaches the handler and fails there")
                    };
                    let observed_at = if matches!(case, Case::HandlerFailureLateRecording) {
                        95
                    } else {
                        69
                    };
                    let aliases = [
                        handle_ref.clone(),
                        handle.operation_lease_ref().clone(),
                        operation.request_coordinate().record_ref().clone(),
                        initial_record.coordinate().record_ref().clone(),
                        record_ref("native-shell/selection"),
                        record_ref("native-shell/preparation"),
                        failed.dispatch().0.clone(),
                    ];
                    let prepared =
                        failure::prepare(failed, observation.signed, observed_at, &aliases);
                    let append = prepared.prepare_append(&Store).unwrap();
                    failure::assert_append(failed, &prepared, &append);
                    let (settled, _) = custody::complete(
                        append,
                        request,
                        (&dispatch_checkpoint_ref, &dispatch_checkpoint),
                        OperationCustodyDatabaseEvidence::New,
                        observed_at,
                    );
                    assert_eq!(settled.candidate().records().len(), 2);
                    assert_eq!(
                        clock.reads.load(Ordering::SeqCst),
                        3,
                        "failure signing and custody do not release another handler"
                    );
                    return;
                }
                let NativeOperationOutcome::Succeeded(executed) = executed.outcome() else {
                    panic!("the admitted relative path must complete")
                };
                let value = serde_json::to_value(executed.execution()).unwrap();
                assert_eq!(
                    executed.execution().request.action_id,
                    PATH_INVENTORY_ACTION_ID
                );
                let output = &value["result"]["output"];
                assert_eq!(output["payload"]["input_count"], 1);
                assert_eq!(output["payload"]["unique_count"], 1);
                assert_eq!(
                    output["payload"]["entries"][0]["normalized_path"],
                    "src/lib.rs"
                );
                assert!(executed
                    .issue_result(UnixTimeV1::new(68).unwrap(), |_, _, _| {
                        Err(InvalidResidentRecordV1::ContextualAdmission {
                            record: "test signer",
                            invariant: "unavailable",
                        })
                    })
                    .is_err());
                assert_eq!(
                    serde_json::to_value(executed.execution()).unwrap(),
                    value,
                    "failed signing retains the actual computation"
                );
                let observed_at = if matches!(case, Case::LateRecording) {
                    95
                } else {
                    69
                };
                let completed = executed
                    .prepare_completion(
                        NativeOperationCompletionInput {
                            recovered_history: None,
                            result_ref: record_ref("native-shell/result"),
                            observation_ref: record_ref("native-shell/succeeded"),
                            completed_at: UnixTimeV1::new(68).unwrap(),
                            observed_at: UnixTimeV1::new(observed_at).unwrap(),
                            signing: observation.signed,
                        },
                        |_, _, _| Ok(signature()),
                        |_, _, _, _| Ok(()),
                        |_, _, _| Ok(signature()),
                    )
                    .unwrap();
                let append = completed.prepare_append(&Store).unwrap();
                let Some(OperationCustodyRecord::CapabilityResult(result_record)) =
                    append.associated_record()
                else {
                    panic!("actual signed result belongs to the terminal append")
                };
                assert_eq!(
                    result_record.canonical_record_jcs().as_slice(),
                    canon::to_canonical_string(completed.result())
                        .unwrap()
                        .as_bytes()
                );
                let OperationObservationStateV1::Succeeded {
                    result,
                    effect_receipt,
                } = completed.observation().1.state()
                else {
                    panic!("derived success")
                };
                assert!(effect_receipt.is_none());
                assert_eq!(&result.record_ref, result_record.coordinate().record_ref());
                assert_eq!(
                    result.record_sha256,
                    *completed.result().canonical_result_sha256()
                );
                let selection_record = &selection_record;
                let early_terminal = issue_observation(
                    parents.signing,
                    handle_ref,
                    handle,
                    schema.clone(),
                    *completed.observation().1.revision(),
                    Some(executed.dispatch()),
                    completed.observation().1.state().clone(),
                    67,
                );
                let history = || crate::resident::OperationAttemptHistoryInput {
                    operation,
                    request,
                    session: parents.session.lease(),
                    lease: parents.lease,
                    generation: &generation,
                    dispatched: executed.dispatch().1,
                    selection: selection_record,
                    maximum_selection_bytes: 65_536,
                };
                assert!(matches!(
                    history().classify_owner().unwrap(),
                    crate::resident::operation_dispatch::OperationAttemptOwner::Native
                ));
                let wrong_dispatch_ref = record_ref("other/dispatch");
                for failure in [
                    "none",
                    "byte-limit",
                    "reference",
                    "digest",
                    "terminal-time",
                    "dispatch-reference",
                    "nonterminal",
                ] {
                    let mut expected = result.clone();
                    if failure == "reference" {
                        expected.record_ref = record_ref("other/result");
                    }
                    if failure == "digest" {
                        expected.record_sha256 =
                            CapabilityResultDigest::hash_canonical(&"other result").unwrap();
                    }
                    let stored = ControlRecord::new(
                        pale_ports::ControlRecordCoordinate::new(
                            expected.record_ref,
                            pale_ports::ControlRecordDigest::parse(expected.record_sha256.as_str())
                                .unwrap(),
                        ),
                        result_record.schema_id().clone(),
                        result_record.schema_sha256().clone(),
                        pale_ports::ControlRecordBytes::new(
                            result_record.canonical_record_jcs().as_slice().to_vec(),
                        )
                        .unwrap(),
                    );
                    let admitted = crate::resident::admit_native_operation_result(
                        crate::resident::NativeOperationResultInput {
                            history: history(),
                            registry: &registry,
                            contract_set: admission.contract_set,
                            terminal: crate::resident::NativeOperationTerminalEvidence::Direct {
                                dispatched_ref: if failure == "dispatch-reference" {
                                    &wrong_dispatch_ref
                                } else {
                                    executed.dispatch().0
                                },
                                terminal: match failure {
                                    "terminal-time" => &early_terminal,
                                    "nonterminal" => executed.dispatch().1,
                                    _ => completed.observation().1,
                                },
                            },
                            result: &stored,
                            maximum_result_bytes: if failure == "byte-limit" { 1 } else { 65_536 },
                        },
                        |domain, preimage, _, supplied_signature| {
                            assert_eq!(domain, CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN);
                            assert_eq!(&preimage.signing_authority_binding, parents.signing);
                            assert_eq!(supplied_signature, &signature());
                            Ok(())
                        },
                    );
                    if failure == "none" {
                        assert_eq!(&admitted.unwrap(), completed.result());
                    } else {
                        assert!(admitted.is_err(), "{failure}");
                    }
                }
                let (settled, _) = custody::complete(
                    append,
                    request,
                    (&dispatch_checkpoint_ref, &dispatch_checkpoint),
                    OperationCustodyDatabaseEvidence::New,
                    observed_at,
                );
                assert_eq!(settled.candidate().records().len(), 2);
                assert_eq!(
                    clock.reads.load(Ordering::SeqCst),
                    3,
                    "completion does not execute again"
                );
            });
        },
    );
}

#[test]
fn native_action_crosses_both_checkpoint_commits_and_retains_actual_result_with_success() {
    journey(Case::Complete);
    journey(Case::UnusedReadAuthority);
    journey(Case::LateRecording);
}

#[test]
fn native_cancellation_preparation_retains_call_on_refusal_and_binds_exact_predecessor() {
    journey(Case::Cancel);
    journey(Case::CancelUnusedReadAuthority);
}

#[test]
fn native_handler_failure_retains_diagnostic_and_terminal_observation_without_reexecution() {
    journey(Case::HandlerFailure);
    journey(Case::HandlerFailureLateRecording);
    journey(Case::HandlerFailureReplay);
}

#[test]
fn native_preparation_validates_input_and_refuses_unbound_context() {
    journey(Case::MalformedInput);
    journey(Case::Inherited);
    journey(Case::EvidenceOnly);
}

#[test]
fn native_execution_refuses_replay_uncertain_commit_and_changed_current_custody() {
    for case in [
        Case::Replay,
        Case::RecoveredCommit,
        Case::WrongCompletion,
        Case::ShiftedHead,
    ] {
        journey(case);
    }
}

#[test]
fn native_execution_rechecks_time_after_the_last_authority_callback() {
    for case in [
        Case::ExpiredBeforeRead,
        Case::BackwardBeforeRead,
        Case::ExpiredAfterAuthority,
        Case::BackwardAfterAuthority,
    ] {
        journey(case);
    }
}

#[test]
fn native_release_refusal_retains_call_and_rechecks_without_forgetting_time_or_authority() {
    for case in [
        Case::ReleaseClockRecovery,
        Case::ReleaseProjectionRecovery,
        Case::ReleaseHighWater,
        Case::ReleaseTimeFloor,
        Case::ReleaseElapsedBudget,
        Case::ReleaseCurrentAuthority,
    ] {
        journey(case);
    }
}

#[test]
fn refused_native_call_cancellation_preserves_custody_time_and_single_use() {
    for case in [
        Case::CancelRefused,
        Case::CancelRefusedDeadline,
        Case::CancelRefusedExpired,
        Case::CancelRefusedExpiresAfterReads,
        Case::CancelRefusedHighwater,
        Case::CancelRefusedReplay,
        Case::CancelRefusedRecovered,
        Case::CancelRefusedChangedHead,
        Case::CancelRefusedMissingAnchor,
    ] {
        journey(case);
    }
}

#[test]
fn unopened_expiry_requires_retained_expiry_evidence_and_original_new_custody() {
    use refusal_expiry::Case as Expiry;
    for case in [
        Expiry::Handle,
        Expiry::LeaseAndHandle,
        Expiry::Deadline,
        Expiry::CancellationCrossing,
        Expiry::NoExpiry,
        Expiry::Clock,
        Expiry::Backward,
        Expiry::CallerFloor,
        Expiry::RejectedCrossing,
        Expiry::Revocation,
        Expiry::Replay,
        Expiry::Recovered,
        Expiry::ChangedHead,
    ] {
        journey(Case::Expiry(case));
    }
}
