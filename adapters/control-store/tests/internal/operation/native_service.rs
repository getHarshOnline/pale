//! One application-owned invocation over real `SQLite` custody.
//! Signing, current authority and the independent anchor remain explicit fixtures.

use super::*;
use std::cell::Cell;
#[path = "native_hook_response.rs"]
pub(super) mod hook_response;
#[path = "native_service/ports.rs"]
mod ports;
pub(super) use hook_response::{
    hook_request_body, recover as recover_hook_response, run as run_hook_response,
    Case as HookResponseCase,
};

#[derive(Clone, Copy, Debug)]
pub(super) enum Case {
    Success,
    HandlerFailure,
    ChangedSession,
    AliasedCheckpoint,
    PreparationCheckpointFailure,
    DispatchSignerFailure,
    ReleaseClockMissing,
    CompletionClockMissing,
    CompletionClockBackwards,
    ResultSignerFailure,
    TerminalCheckpointFailure,
    TerminalCommitFailure,
    DispatchLostConfirmation,
    DispatchUnknownReadBack,
    OutcomeUnknownReadBack,
    OutcomeAnchorFailure,
    TerminalAnchorFailure,
    PreparationAnchorFailure,
    DispatchAnchorFailure,
    EvidenceObligations,
    UnusedReadAuthority,
}

struct ServiceClock {
    calls: AtomicUsize,
    case: Case,
    first: u64,
}

impl WallClockSourcePort for ServiceClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        let count = self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(
            count < 16,
            "one bounded service invocation, not a retry loop"
        );
        if matches!(
            (self.case, count),
            (Case::ReleaseClockMissing, 2) | (Case::CompletionClockMissing, 5)
        ) {
            return Err(WallClockSourceError::from_source(std::io::Error::other(
                "fixture clock unavailable",
            )));
        }
        if matches!((self.case, count), (Case::CompletionClockBackwards, 5)) {
            return Ok(UnixTimeV1::new(65).unwrap());
        }
        Ok(UnixTimeV1::new(self.first + count as u64).unwrap())
    }
}

pub(super) fn run(
    case: Case,
    attempt: NativeOperationAttemptInput<'_, '_>,
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) -> Option<CompletedNativeOperation> {
    if matches!(case, Case::ChangedSession) {
        let current = attempt.context.current;
        let other_ref = record_ref("grant/service-independent-other-reference");
        let changed = AmbientSessionAdmissionContext {
            now: current.now,
            subject: current.subject,
            grant: Some(CapabilityGrantAuthority {
                grant_ref: &other_ref,
                ..current.grant.unwrap()
            }),
            current_principal_trust: current.current_principal_trust,
            capability_revocations: current.capability_revocations,
            deployment_binding_ref: current.deployment_binding_ref,
            deployment_binding: current.deployment_binding,
            deployment_revocations: current.deployment_revocations,
            runtime_authority: current.runtime_authority,
            session_revocations: current.session_revocations,
            route_use: current.route_use,
            generation: SessionAdmissionLeaseContextV1 {
                active_head: current.generation.active_head,
                compiled_generation: current.generation.compiled_generation,
                effective_profile: current.generation.effective_profile,
                activation_fence: current.generation.activation_fence,
                signed: current.generation.signed,
            },
        };
        let session = admit_ambient_session(
            SessionAdmissionLeaseWireV1::from(attempt.context.session.lease()),
            &changed,
            &mut ProvingAuthority,
        )
        .expect("the replacement is independently admissible, not malformed");
        assert_eq!(session.lease(), attempt.context.session.lease());
        assert_eq!(
            session.grant_sha256(),
            attempt.context.session.grant_sha256()
        );
        let selected = NativeOperationAttemptInput {
            context: OperationAttemptContext {
                current: &changed,
                session: &session,
                ..attempt.context
            },
            registry: attempt.registry,
            attempt_id: attempt.attempt_id,
            result_admission: attempt.result_admission,
        };
        let writes = port.connection.total_changes();
        let direct = prepare_native_operation_attempt(
            NativeOperationAttemptInput {
                context: selected.context.clone(),
                registry: selected.registry,
                attempt_id: selected.attempt_id,
                result_admission: selected.result_admission,
            },
            &mut ProvingAuthority,
            |_, _, _, _| panic!("replacement session cannot reach rotation admission"),
            |_, _, _| panic!("replacement session cannot sign"),
        );
        assert!(matches!(
            direct,
            Err(OperationAttemptError::Session(
                AmbientSessionAdmissionError::SessionAuthorityChanged
            ))
        ));
        assert_eq!(port.connection.total_changes(), writes);
        return run_selected(case, selected, checkpoint, port, anchor);
    }
    run_selected(case, attempt, checkpoint, port, anchor)
}

fn run_selected(
    case: Case,
    attempt: NativeOperationAttemptInput<'_, '_>,
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) -> Option<CompletedNativeOperation> {
    let original_current = attempt.context.current;
    let expected_terminal_revision = attempt
        .context
        .observation
        .predecessor
        .expect("service input has one durable predecessor")
        .1
        .revision()
        .get()
        + 4;
    let namespace = if attempt
        .context
        .operation
        .handle()
        .1
        .parent_operation()
        .is_some()
    {
        "native/child-service"
    } else {
        "native/service"
    };
    let signing = attempt.context.observation.signed;
    let coordinate = port.coordinate.clone();
    let resolve = |_: &OperationObservationV1| Ok(coordinate.clone());
    let mut fields = checkpoint_preimage(2, None);
    fields.checkpoint_domain_id = checkpoint.1.checkpoint_domain_id().clone();
    fields.checkpoint_profile_ref = checkpoint.1.checkpoint_profile_ref().clone();
    fields.checkpoint_profile_sha256 = checkpoint.1.checkpoint_profile_sha256().clone();
    let observations = Cell::new(0);
    let checkpoints = Cell::new(0);
    let results = Cell::new(0);
    let checkpoint_input = |stage: &str| OperationCheckpointInput {
        reference: record_ref(&format!("{namespace}-checkpoint-{stage}")),
        id: ControlCheckpointIdV1::parse(format!(
            "{}-checkpoint-{stage}",
            namespace.replace('/', "-"),
        ))
        .unwrap(),
    };
    let writes = port.connection.total_changes();
    if matches!(case, Case::TerminalCommitFailure) {
        port.connection.execute_batch("CREATE TEMP TRIGGER reject_service_terminal BEFORE INSERT ON pale_deployment_control_record WHEN NEW.record_ref = 'native/service-terminal' BEGIN SELECT RAISE(ABORT, 'fixture terminal storage unavailable'); END;").unwrap();
    }
    let (completed, clock_calls, commits, last_candidate) = {
        // The returned failure may retain the operation, never these temporary
        // clock, resolver or checkpoint-authority borrows.
        let clock = ServiceClock {
            calls: AtomicUsize::new(0),
            case,
            // Root fixtures still start at 62; an issued child may inherit a
            // later checkpoint without moving any existing fault-case times.
            first: checkpoint
                .1
                .issued_at()
                .get()
                .checked_add(1)
                .unwrap()
                .max(62),
        };
        let authority = checkpoint_authority(&fields);
        let resolve = |_: &OperationObservationV1| Ok(coordinate.clone());
        let mut custody = ports::Custody {
            inner: port,
            case,
            commits: 0,
            last: None,
        };
        let mut anchors = ports::Anchor {
            inner: anchor,
            case,
            advances: 0,
        };
        let mut input = NativeOperationInvokeInput {
            attempt,
            checkpoint,
            checkpoint_schema: &fields.schema_sha256,
            checkpoint_authority: &authority,
            signing,
            preparation_checkpoint: checkpoint_input("prepared"),
            dispatch_checkpoint: checkpoint_input("dispatched"),
            outcome_checkpoint: checkpoint_input("outcome"),
            terminal_checkpoint: checkpoint_input("terminal"),
            dispatch_ref: record_ref(&format!("{namespace}-dispatch")),
            outcome_ref: record_ref(&format!("{namespace}-outcome")),
            outcome_observation_ref: record_ref(&format!("{namespace}-outcome-observation")),
            result_ref: record_ref(&format!("{namespace}-result")),
            diagnostic_ref: record_ref(&format!("{namespace}-diagnostic")),
            terminal_ref: record_ref(&format!("{namespace}-terminal")),
            clock: &clock,
            resolve_store: &resolve,
            maximum_record_bytes: 65_536,
            maximum_diagnostic_bytes: 65_536,
            maximum_outcome_bytes: 65_536,
            accumulator_limits: AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000)
                .unwrap(),
        };
        if matches!(case, Case::AliasedCheckpoint) {
            input.terminal_checkpoint.reference = input.preparation_checkpoint.reference.clone();
        }
        let completed = invoke_native_operation(
            input,
            &mut ProvingAuthority,
            &mut custody,
            &mut anchors,
            |_, _, _, _| Ok(()),
            |_, _, _| {
                observations.set(observations.get() + 1);
                if matches!(case, Case::DispatchSignerFailure) && observations.get() == 2 {
                    return Err(signer_failure());
                }
                Ok(signature())
            },
            |_, _, _| {
                checkpoints.set(checkpoints.get() + 1);
                if matches!(case, Case::PreparationCheckpointFailure) && checkpoints.get() == 1 {
                    return Err(signer_failure());
                }
                if matches!(case, Case::TerminalCheckpointFailure) && checkpoints.get() == 4 {
                    return Err(signer_failure());
                }
                Ok(signature())
            },
            |_, _, _| {
                results.set(results.get() + 1);
                if matches!(case, Case::ResultSignerFailure) {
                    return Err(signer_failure());
                }
                Ok(signature())
            },
        );
        (
            completed,
            clock.calls.load(Ordering::SeqCst),
            custody.commits,
            custody.last,
        )
    };
    if matches!(
        case,
        Case::ChangedSession | Case::AliasedCheckpoint | Case::EvidenceObligations
    ) {
        assert!(
            matches!(
                (case, completed),
                (
                    Case::ChangedSession,
                    Err(NativeOperationInvokeFailure::Admission(
                        OperationAttemptError::Session(
                            AmbientSessionAdmissionError::SessionAuthorityChanged,
                        )
                    )),
                ) | (
                    Case::AliasedCheckpoint,
                    Err(NativeOperationInvokeFailure::Admission(
                        OperationAttemptError::Association,
                    )),
                ) | (
                    Case::EvidenceObligations,
                    Err(NativeOperationInvokeFailure::Admission(
                        OperationAttemptError::ContextRequired,
                    )),
                )
            ),
            "{case:?} must reject its exact association before starting"
        );
        assert_eq!(
            (observations.get(), checkpoints.get(), results.get()),
            (0, 0, 0)
        );
        assert_eq!(port.connection.total_changes(), writes);
        assert_eq!(commits, 0);
        assert_eq!(clock_calls, 0);
        return None;
    }
    if !matches!(
        case,
        Case::Success | Case::HandlerFailure | Case::UnusedReadAuthority
    ) {
        let failed = completed.expect_err("the injected fault must return its exact owned phase");
        match (case, failed) {
            (
                Case::PreparationAnchorFailure,
                NativeOperationInvokeFailure::Preparation { attempt, failure },
            ) => {
                let OperationAppendFailure::Anchor {
                    failure,
                    checkpoint,
                } = *failure
                else {
                    panic!("preparation anchor failure");
                };
                let completed = failure
                    .try_into_retryable()
                    .unwrap()
                    .advance_anchor(anchor)
                    .unwrap();
                assert_eq!(
                    attempt.prepare_append(&resolve).unwrap().successor_record(),
                    completed
                        .candidate()
                        .record(OperationCustodyRecordKind::OperationObservation)
                        .unwrap()
                );
                assert_eq!(
                    checkpoint.1.canonical_control_checkpoint_sha256().as_str(),
                    completed
                        .candidate()
                        .checkpoint()
                        .successor()
                        .coordinate()
                        .record_sha256()
                        .as_str()
                );
                assert_eq!((commits, clock_calls, results.get()), (1, 1, 0));
            }
            (
                Case::DispatchAnchorFailure,
                NativeOperationInvokeFailure::Dispatch { dispatch, failure },
            ) => {
                let OperationAppendFailure::Anchor {
                    failure,
                    checkpoint,
                } = *failure
                else {
                    panic!("dispatch anchor failure");
                };
                let completed = failure
                    .try_into_retryable()
                    .unwrap()
                    .advance_anchor(anchor)
                    .unwrap();
                assert_eq!(
                    dispatch
                        .prepare_append(&resolve)
                        .unwrap()
                        .successor_record(),
                    completed
                        .candidate()
                        .record(OperationCustodyRecordKind::OperationObservation)
                        .unwrap()
                );
                assert_eq!(
                    checkpoint.1.canonical_control_checkpoint_sha256().as_str(),
                    completed
                        .candidate()
                        .checkpoint()
                        .successor()
                        .coordinate()
                        .record_sha256()
                        .as_str()
                );
                assert_eq!((commits, clock_calls, results.get()), (2, 2, 0));
            }
            (
                Case::PreparationCheckpointFailure,
                NativeOperationInvokeFailure::Preparation { attempt, failure },
            ) => {
                assert!(matches!(
                    *failure,
                    OperationAppendFailure::Projection(
                        OperationObservationCustodyProjectionError::Semantic(
                            InvalidResidentRecordV1::ContextualAdmission {
                                invariant: "signer unavailable",
                                ..
                            }
                        )
                    )
                ));
                assert_eq!(port.connection.total_changes(), writes);
                assert!(attempt
                    .prepare_append(&resolve)
                    .unwrap()
                    .associated_record()
                    .is_some());
                assert_eq!(
                    (observations.get(), checkpoints.get(), results.get()),
                    (1, 1, 0)
                );
            }
            (
                Case::DispatchSignerFailure,
                NativeOperationInvokeFailure::DispatchPreparation {
                    attempt,
                    completed,
                    checkpoint,
                    reason,
                },
            ) => {
                assert!(matches!(
                    *reason,
                    OperationAttemptError::Admission(
                        InvalidResidentRecordV1::ContextualAdmission {
                            invariant: "signer unavailable",
                            ..
                        }
                    )
                ));
                assert_eq!(
                    port.observe_exact(completed.candidate()).unwrap(),
                    OperationCustodyReadBackOutcome::ExactSuccessor
                );
                assert_eq!(
                    checkpoint.1.canonical_control_checkpoint_sha256().as_str(),
                    completed
                        .candidate()
                        .checkpoint()
                        .successor()
                        .coordinate()
                        .record_sha256()
                        .as_str()
                );
                assert_eq!(
                    attempt.prepare_append(&resolve).unwrap().successor_record(),
                    completed
                        .candidate()
                        .record(OperationCustodyRecordKind::OperationObservation)
                        .unwrap()
                );
                assert_eq!(
                    (observations.get(), checkpoints.get(), results.get()),
                    (2, 1, 0)
                );
            }
            (Case::ReleaseClockMissing, NativeOperationInvokeFailure::Invocation(refusal)) => {
                assert!(matches!(refusal.reason(), OperationAttemptError::Clock(_)));
                assert_eq!(
                    (observations.get(), checkpoints.get(), results.get()),
                    (2, 2, 0)
                );
                assert_eq!(clock_calls, 3);
                // Re-enter only the retained unopened call, never the service or
                // dispatch append. Its original New evidence remains selected.
                let next_clock = super::Clock(AtomicUsize::new(0));
                let writes = port.connection.total_changes();
                let outcome = refusal.recheck(
                    NativeOperationExecutionContext {
                        current: original_current,
                        resolve_store: &resolve,
                        clock: &next_clock,
                        maximum_record_bytes: 65_536,
                    },
                    &mut ProvingAuthority,
                    port,
                    anchor,
                );
                assert!(outcome.is_ok());
                assert_eq!(next_clock.0.load(Ordering::SeqCst), 3);
                assert_eq!(port.connection.total_changes(), writes);
            }
            (Case::DispatchLostConfirmation, NativeOperationInvokeFailure::Invocation(refusal)) => {
                assert!(matches!(
                    refusal.reason(),
                    OperationAttemptError::Custody(OperationDispatchCustodyError::NotNewCommit)
                ));
                assert_eq!(
                    (observations.get(), checkpoints.get(), results.get()),
                    (2, 2, 0)
                );
                assert_eq!(commits, 2);
                assert_eq!(clock_calls, 3);
                let candidate = last_candidate.unwrap();
                assert_eq!(
                    port.observe_exact(&candidate).unwrap(),
                    OperationCustodyReadBackOutcome::ExactSuccessor
                );
                let writes = port.connection.total_changes();
                let retry_clock = super::Clock(AtomicUsize::new(0));
                let refusal = refusal
                    .recheck(
                        NativeOperationExecutionContext {
                            current: original_current,
                            resolve_store: &resolve,
                            clock: &retry_clock,
                            maximum_record_bytes: 65_536,
                        },
                        &mut ProvingAuthority,
                        port,
                        anchor,
                    )
                    .expect_err("re-entry cannot manufacture New evidence");
                assert!(matches!(
                    refusal.reason(),
                    OperationAttemptError::Custody(OperationDispatchCustodyError::NotNewCommit)
                ));
                assert_eq!(port.connection.total_changes(), writes);
            }
            (
                Case::DispatchUnknownReadBack,
                NativeOperationInvokeFailure::Dispatch { dispatch, failure },
            ) => {
                let OperationAppendFailure::Commit { failure, .. } = *failure else {
                    panic!("failed ambiguity readback retains commit phase");
                };
                assert!(
                    failure.try_into_retryable().is_err(),
                    "uncertainty is not permission to retry"
                );
                assert_eq!(commits, 2);
                assert_eq!(clock_calls, 2);
                assert_eq!(results.get(), 0);
                let candidate = last_candidate.unwrap();
                assert_eq!(
                    dispatch
                        .prepare_append(&resolve)
                        .unwrap()
                        .successor_record(),
                    candidate
                        .record(OperationCustodyRecordKind::OperationObservation)
                        .unwrap()
                );
                assert_eq!(
                    port.observe_exact(&candidate).unwrap(),
                    OperationCustodyReadBackOutcome::ExactSuccessor
                );
            }
            (
                Case::OutcomeUnknownReadBack,
                NativeOperationInvokeFailure::Recording(
                    NativeOperationRecordingFailure::StagingCommit(failure),
                ),
            ) => {
                assert!(matches!(
                    failure.reason(),
                    OperationObservationCustodyCommitFailureReason::Database(
                        OperationCustodyDatabaseFailureReason::ReadBackFailed { .. }
                    )
                ));
                assert_eq!((commits, clock_calls, results.get()), (3, 6, 0));
                let candidate = last_candidate.unwrap();
                assert!(candidate
                    .record(OperationCustodyRecordKind::NativeOutcome)
                    .is_some());
                assert_eq!(
                    port.observe_exact(&candidate).unwrap(),
                    OperationCustodyReadBackOutcome::ExactSuccessor
                );
                let writes = port.connection.total_changes();
                let pending = failure
                    .reconcile_exact(port)
                    .expect("later exact read-back retains the original stage candidate");
                let finalization = pending
                    .continue_anchor(anchor)
                    .expect("only the exact staged anchor continuation is exposed");
                assert_eq!(
                    finalization
                        .checkpoint()
                        .1
                        .canonical_control_checkpoint_sha256()
                        .as_str(),
                    candidate
                        .checkpoint()
                        .successor()
                        .coordinate()
                        .record_sha256()
                        .as_str()
                );
                let staged = finalization
                    .finish()
                    .expect("exact custody admits the retained staged head");
                assert!(matches!(
                    staged.outcome(),
                    NativeOperationOutcome::Succeeded(_)
                ));
                assert_eq!(
                    port.connection.total_changes(),
                    writes,
                    "read-back, anchor continuation, and stage finalization do not restage"
                );

                let continuation_clock = ServiceClock {
                    calls: AtomicUsize::new(0),
                    case: Case::Success,
                    first: 68,
                };
                let completed = staged
                    .record_terminal(
                        NativeOperationTerminalRecordingInput {
                            result_ref: record_ref(&format!("{namespace}-result")),
                            diagnostic_ref: record_ref(&format!("{namespace}-diagnostic")),
                            terminal_ref: record_ref(&format!("{namespace}-terminal")),
                            terminal_checkpoint: checkpoint_input("terminal"),
                            clock: &continuation_clock,
                            resolve_store: &resolve,
                            checkpoint_schema: &fields.schema_sha256,
                            checkpoint_authority: &checkpoint_authority(&fields),
                            signing,
                            maximum_diagnostic_bytes: 65_536,
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                1, 10_000, 10_000, 4096, 100_000,
                            )
                            .unwrap(),
                        },
                        port,
                        anchor,
                        |_, _, _, _| Ok(()),
                        |_, _, _| {
                            observations.set(observations.get() + 1);
                            Ok(signature())
                        },
                        |_, _, _| {
                            checkpoints.set(checkpoints.get() + 1);
                            Ok(signature())
                        },
                        |_, _, _| {
                            results.set(results.get() + 1);
                            Ok(signature())
                        },
                    )
                    .expect("the admitted staged head continues directly to terminal custody");
                assert_payload(completed.result().unwrap());
                assert_eq!(continuation_clock.calls.load(Ordering::SeqCst), 1);
                assert_eq!(
                    (observations.get(), checkpoints.get(), results.get()),
                    (4, 4, 1)
                );
                let staged_records: i64 = port
                    .connection
                    .query_row(
                        "SELECT COUNT(*) FROM pale_deployment_control_record WHERE schema_id=?1",
                        [PRIVATE_OPERATION_NATIVE_OUTCOME_V1_SCHEMA],
                        |row| row.get(0),
                    )
                    .unwrap();
                let staged_indexes: i64 = port
                    .connection
                    .query_row(
                        "SELECT COUNT(*) FROM pale_deployment_operation_index WHERE index_role='native_outcome'",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(
                    (staged_records, staged_indexes),
                    (1, 1),
                    "read-back and terminal continuation cannot create an alternate stage"
                );
            }
            (
                Case::OutcomeAnchorFailure,
                NativeOperationInvokeFailure::Recording(
                    NativeOperationRecordingFailure::StagingAnchor(pending),
                ),
            ) => {
                let writes = port.connection.total_changes();
                let finalization = pending
                    .continue_anchor(anchor)
                    .expect("database-retained stage exposes exact anchor continuation");
                let checkpoint = finalization.checkpoint().1.clone();
                let staged = finalization
                    .finish()
                    .expect("completed anchor admits the original staged head");
                assert!(matches!(
                    staged.outcome(),
                    NativeOperationOutcome::Succeeded(_)
                ));
                assert_eq!((commits, clock_calls, results.get()), (3, 6, 0));
                let candidate = last_candidate.unwrap();
                assert!(candidate
                    .record(OperationCustodyRecordKind::NativeOutcome)
                    .is_some());
                assert_eq!(
                    candidate
                        .checkpoint()
                        .successor()
                        .coordinate()
                        .record_sha256()
                        .as_str(),
                    checkpoint.canonical_control_checkpoint_sha256().as_str()
                );
                assert_eq!(port.connection.total_changes(), writes);
            }
            (
                Case::TerminalAnchorFailure | Case::TerminalCommitFailure,
                NativeOperationInvokeFailure::Recording(
                    NativeOperationRecordingFailure::TerminalCustody { failure },
                ),
            ) => {
                assert_eq!(clock_calls, 7);
                if matches!(case, Case::TerminalAnchorFailure) {
                    let OperationAppendFailure::Anchor {
                        checkpoint,
                        failure,
                    } = *failure
                    else {
                        panic!("database committed but anchor remains pending");
                    };
                    let pending = failure
                        .try_into_retryable()
                        .expect("unavailable anchor with exact predecessor is retryable");
                    let writes = port.connection.total_changes();
                    let completed = pending.advance_anchor(anchor).unwrap();
                    assert_eq!(
                        completed
                            .candidate()
                            .checkpoint()
                            .successor()
                            .coordinate()
                            .record_sha256()
                            .as_str(),
                        checkpoint.1.canonical_control_checkpoint_sha256().as_str()
                    );
                    assert_eq!(
                        port.observe_exact(completed.candidate()).unwrap(),
                        OperationCustodyReadBackOutcome::ExactSuccessor
                    );
                    assert_eq!(
                        port.connection.total_changes(),
                        writes,
                        "anchor continuation never re-appends or invokes"
                    );
                } else {
                    let OperationAppendFailure::Commit {
                        checkpoint,
                        failure,
                    } = *failure
                    else {
                        panic!("physical terminal failure");
                    };
                    port.connection
                        .execute_batch("DROP TRIGGER reject_service_terminal")
                        .unwrap();
                    let retry = failure
                        .try_into_retryable()
                        .expect("definite storage failure retains exact physical candidate");
                    let completed = retry
                        .commit(port, anchor)
                        .unwrap()
                        .advance_anchor(anchor)
                        .unwrap();
                    assert_eq!(
                        completed
                            .candidate()
                            .checkpoint()
                            .successor()
                            .coordinate()
                            .record_sha256()
                            .as_str(),
                        checkpoint.1.canonical_control_checkpoint_sha256().as_str()
                    );
                    assert_eq!(
                        port.observe_exact(completed.candidate()).unwrap(),
                        OperationCustodyReadBackOutcome::ExactSuccessor
                    );
                    assert_eq!(
                        (observations.get(), checkpoints.get(), results.get()),
                        (4, 4, 1)
                    );
                }
            }
            (
                Case::CompletionClockMissing,
                NativeOperationInvokeFailure::Recording(
                    NativeOperationRecordingFailure::Unstaged { outcome, reason },
                ),
            ) => {
                assert_eq!(clock_calls, 6);
                assert!(matches!(
                    outcome.outcome(),
                    NativeOperationOutcome::Succeeded(_)
                ));
                assert!(matches!(
                    (&case, &reason),
                    (
                        Case::CompletionClockMissing,
                        OperationAttemptError::Clock(_),
                    )
                ));
                assert_eq!(
                    (observations.get(), checkpoints.get(), results.get()),
                    (2, 2, 0)
                );
            }
            (
                Case::CompletionClockBackwards,
                NativeOperationInvokeFailure::Recording(
                    NativeOperationRecordingFailure::StagePreparation {
                        continuation: _,
                        reason,
                    },
                ),
            ) => {
                assert_eq!(clock_calls, 6);
                assert!(matches!(reason, OperationAttemptError::TimeOrder));
                assert_eq!(
                    (observations.get(), checkpoints.get(), results.get()),
                    (2, 2, 0)
                );
            }
            (
                Case::ResultSignerFailure | Case::TerminalCheckpointFailure,
                NativeOperationInvokeFailure::Recording(
                    NativeOperationRecordingFailure::TerminalPreparation { staged, failure },
                ),
            ) => {
                assert_eq!(clock_calls, 7);
                assert!(matches!(
                    staged.outcome(),
                    NativeOperationOutcome::Succeeded(_)
                ));
                assert!(matches!(
                    (case, &*failure),
                    (
                        Case::ResultSignerFailure,
                        OperationAppendFailure::Operation(OperationAttemptError::Admission(_)),
                    ) | (
                        Case::TerminalCheckpointFailure,
                        OperationAppendFailure::Projection(
                            OperationObservationCustodyProjectionError::Semantic(_),
                        ),
                    )
                ));
                let writes = port.connection.total_changes();
                let continuation_clock = ServiceClock {
                    calls: AtomicUsize::new(0),
                    case: Case::Success,
                    first: 68,
                };
                let continued = staged
                    .record_terminal(
                        NativeOperationTerminalRecordingInput {
                            result_ref: record_ref("native/recovered-service-result"),
                            diagnostic_ref: record_ref("native/recovered-service-diagnostic"),
                            terminal_ref: record_ref("native/recovered-service-terminal"),
                            terminal_checkpoint: OperationCheckpointInput {
                                reference: record_ref(
                                    "native/recovered-service-terminal-checkpoint",
                                ),
                                id: ControlCheckpointIdV1::parse(
                                    "native-recovered-service-terminal-checkpoint",
                                )
                                .unwrap(),
                            },
                            clock: &continuation_clock,
                            resolve_store: &resolve,
                            checkpoint_schema: &fields.schema_sha256,
                            checkpoint_authority: &checkpoint_authority(&fields),
                            signing,
                            maximum_diagnostic_bytes: 65_536,
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                1, 10_000, 10_000, 4096, 100_000,
                            )
                            .unwrap(),
                        },
                        port,
                        anchor,
                        |_, _, _, _| Ok(()),
                        |_, _, _| {
                            observations.set(observations.get() + 1);
                            Ok(signature())
                        },
                        |_, _, _| {
                            checkpoints.set(checkpoints.get() + 1);
                            Ok(signature())
                        },
                        |_, _, _| {
                            results.set(results.get() + 1);
                            Ok(signature())
                        },
                    )
                    .unwrap();
                assert_payload(continued.result().unwrap());
                assert!(port.connection.total_changes() > writes);
            }
            (_, failure) => panic!("wrong stopped phase for {case:?}: {failure:?}"),
        }
        return None;
    }
    let completed = match completed {
        Ok(completed) => completed,
        Err(NativeOperationInvokeFailure::Admission(failure)) => {
            panic!("native service admission failed: {failure:?}")
        }
        Err(failure) => {
            panic!("one application service completes actual native custody: {failure:?}")
        }
    };
    assert_eq!(observations.get(), 4);
    assert_eq!(checkpoints.get(), 4);
    assert_eq!(clock_calls, 7);
    assert_eq!(
        completed.observation().1.revision().get(),
        expected_terminal_revision
    );
    match case {
        Case::Success | Case::UnusedReadAuthority => {
            assert_eq!(results.get(), 1);
            assert_payload(completed.result().unwrap());
            assert!(matches!(
                completed.observation().1.state(),
                OperationObservationStateV1::Succeeded { .. }
            ));
        }
        Case::HandlerFailure => {
            assert_eq!(results.get(), 0);
            let diagnostic = completed.result().unwrap_err();
            assert_eq!(diagnostic.wire().code, "invalid_path");
            assert!(
                !String::from_utf8_lossy(diagnostic.canonical_bytes()).contains("token=private")
            );
            assert!(matches!(
                completed.observation().1.state(),
                OperationObservationStateV1::Failed { .. }
            ));
        }
        _ => unreachable!(),
    }
    assert_eq!(commits, 4);
    Some(completed)
}

pub(super) fn read_back(
    completed: &CompletedNativeOperation,
    port: &mut SqliteOperationCustody<'_>,
) {
    let candidate = completed.custody().candidate();
    assert_eq!(
        pale_core::canon::to_canonical_string(completed.observation().1)
            .unwrap()
            .as_bytes(),
        candidate
            .record(OperationCustodyRecordKind::OperationObservation)
            .unwrap()
            .canonical_record_jcs()
            .as_slice()
    );
    match completed.result() {
        Ok(result) => {
            assert_payload(result);
            assert_eq!(
                pale_core::canon::to_canonical_string(result)
                    .unwrap()
                    .as_bytes(),
                candidate
                    .record(OperationCustodyRecordKind::CapabilityResult)
                    .unwrap()
                    .canonical_record_jcs()
                    .as_slice()
            );
            assert!(candidate
                .record(OperationCustodyRecordKind::Diagnostic)
                .is_none());
        }
        Err(diagnostic) => {
            assert_eq!(
                diagnostic.canonical_bytes(),
                candidate
                    .record(OperationCustodyRecordKind::Diagnostic)
                    .unwrap()
                    .canonical_record_jcs()
                    .as_slice()
            );
            assert!(candidate
                .record(OperationCustodyRecordKind::CapabilityResult)
                .is_none());
        }
    }
    assert_eq!(
        completed.custody().database_evidence(),
        OperationCustodyDatabaseEvidence::New
    );
    assert_eq!(
        port.observe_exact(candidate).unwrap(),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    let staged: i64 = port
        .connection
        .query_row(
            "SELECT COUNT(*) FROM pale_deployment_control_record WHERE schema_id=?1",
            [PRIVATE_OPERATION_NATIVE_OUTCOME_V1_SCHEMA],
            |row| row.get(0),
        )
        .unwrap();
    let staged_indexes: i64 = port
        .connection
        .query_row(
            "SELECT COUNT(*) FROM pale_deployment_operation_index WHERE index_role='native_outcome'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!((staged, staged_indexes), (1, 1));
    let writes = port.connection.total_changes();
    for record in candidate.records() {
        let exact = port
            .read_operation_record(candidate.store(), record.record().coordinate(), 65_536)
            .unwrap()
            .unwrap();
        assert_eq!(&exact, record.record());
    }
    assert_eq!(
        port.connection.total_changes(),
        writes,
        "query-only reopened readback never writes"
    );
}

fn assert_payload(result: &CapabilityResultV1) {
    let execution: ActionExecution = serde_json::from_value(
        result
            .success()
            .as_ref()
            .unwrap()
            .inline_payload()
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert_eq!(execution.request.action_id, PATH_INVENTORY_ACTION_ID);
    assert_eq!(execution.result.action_id, PATH_INVENTORY_ACTION_ID);
    let ActionInput::PathInventory(input) = &execution.request.input else {
        panic!("the admitted native input survives service result plumbing");
    };
    assert_eq!(input.paths, ["src/./lib.rs", "src/lib.rs"]);
    let ActionOutput::PathInventory(output) = &execution.result.output else {
        panic!("the actual path inventory output survives service result plumbing");
    };
    assert_eq!((output.input_count, output.unique_count), (2, 1));
    assert_eq!(output.entries.len(), 1);
    assert_eq!(output.entries[0].normalized_path, "src/lib.rs");
    assert_eq!(output.entries[0].occurrences, 2);
    assert_eq!(
        execution.result.request_digest,
        execution.request.request_digest
    );
}

fn signer_failure() -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission {
        record: "native service fixture",
        invariant: "signer unavailable",
    }
}
