//! Local completion of a retained pre-call cancellation request.

use super::restart::{association, host_with_generation, request_admission_context, restart_input};
use super::*;
use crate::local_operation_start_fixture::{record_ref, schema_digest, signature};
use crate::resident::operation_observation_custody::tests::local_recovery::{
    cancellation_accumulator_response, with_process_host_local_cancellation_restart_fixtures,
    ProcessHostRestartFixture, RecoveryAppendAnchor, RecoveryAppendStore, SequencedAppendAnchor,
};
use crate::resident::operation_observation_custody::tests::RecordingStore;
use crate::resident::process_host::cancellation_recovery::*;
use crate::resident::process_host::operation_recovery_sweep::*;
use crate::resident::process_host::restart::*;
use crate::resident::{
    LocalOperationHandleRecoveryInput, OperationAppendFailure, OperationAttemptError,
    OperationCheckpointInput, OperationControlRecoveryAdmission, OperationControlRecoveryInput,
    OperationDispatchCustodyError, OperationObservationAuthorityStoreResolver,
    OperationObservationCheckpointConstructionRejection,
};
use pale_core::canon;
use pale_core::records::{
    CommonRequestEnvelopeV1, CommonRequestEnvelopeWireV1, ControlCheckpointIdV1,
    ControlCheckpointSigningAuthorityAdmissionContextV1, DiagnosticEffectCertaintyV1,
    InvalidResidentRecordV1, OperationObservationStateV1, ResidentRecordRefV1,
    SignedResidentAdmissionContextV1, SigningKeyEpochV1, UnixTimeV1,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID, OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    AccumulatorPreparationLimits, AntiRollbackAnchorErrorKind, ControlStoreError,
    ControlStoreErrorKind, OperationCustodyCommitOutcome, OperationCustodyReadBackOutcome,
    OperationCustodyRecordKind, SessionLeaseCustodyPort,
};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Copy)]
enum CompletionMode {
    Commit,
    DatabaseExactSuccessor,
    DatabaseUnresolved,
    DatabaseRetry,
    AnchorUnavailable,
    AnchorRetry,
    PhaseContinuity,
    FinalHeadChanged,
    FinalAnchorUnavailable,
    ForeignStore,
}

struct CompletionRun {
    result: Result<
        CompletedResidentProcessLocalCancellation,
        ResidentProcessLocalCancellationCompletionFailure,
    >,
    database: (usize, usize),
    anchor: (usize, usize),
    provider_reservations: usize,
    provider_checkouts: usize,
    provider_active: usize,
    task_reservations: usize,
    cancellation_head: ResidentRecordRefV1,
    recovery_reads: RecoveryReadCounts,
    observation_signs: usize,
    checkpoint_signs: usize,
}

struct RecoveryReadCounts {
    before: (usize, usize, usize, usize),
    after: (usize, usize, usize, usize),
}

fn reject_accumulator_preparation(
    _: &pale_ports::OperationAccumulatorPreparation,
) -> Result<
    pale_ports::PreparedOperationAccumulator,
    pale_ports::OperationAccumulatorPreparationError,
> {
    Err(pale_ports::OperationAccumulatorPreparationError::Limit)
}

fn continuation_access<'a>(
    store: &'a pale_ports::AuthorityStoreCoordinate,
    custody: &'a mut dyn pale_ports::OperationCustodyPort,
    sessions: &'a dyn SessionLeaseCustodyPort,
    anchor: &'a mut dyn pale_ports::AntiRollbackAnchorPort,
) -> ResidentDeploymentOperationAccess<'a> {
    ResidentDeploymentOperationAccess {
        store,
        custody,
        sessions,
        anchor,
    }
}

fn reborrow_recovery_admission<'short>(
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

fn cancellation_restart_inputs(
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
    // Initial recovery sees three revisions. A fresh read after terminal custody
    // must admit that retained successor as the fourth revision and checkpoint.
    let make = |includes_terminal_successor| {
        let observation_limits = if includes_terminal_successor {
            crate::resident::OperationObservationRecoveryLimits::new(4, 1024 * 1024, 1024 * 1024)
                .expect("expanded observation bounds")
        } else {
            observation_limits
        };
        let checkpoint_limits = if includes_terminal_successor {
            crate::resident::OperationCheckpointAncestryLimits::new(4, 1024 * 1024, 4)
                .expect("expanded checkpoint bounds")
        } else {
            checkpoint_limits
        };
        let context = if includes_terminal_successor {
            LocalOperationRecoveryContext {
                store: context.store,
                profile: context.profile,
                handle_signing_authority: context.handle_signing_authority,
                checkpoint_authority: context.checkpoint_authority,
                historical: context.historical,
                maximum_record_bytes: context.maximum_record_bytes,
                maximum_operation_lease_bytes: context.maximum_operation_lease_bytes,
                maximum_session_lease_bytes: context.maximum_session_lease_bytes,
                membership_limits: context.membership_limits,
                observation_limits,
                checkpoint_limits,
            }
        } else {
            context
        };
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
    (make(false), make(true), make(true))
}

struct TerminalOnlyRecoveryOwner;

impl<WriterGuard> ResidentProcessRootProviderRecoveryOwner<WriterGuard>
    for TerminalOnlyRecoveryOwner
{
    type Output = ();
    type Error = ();

    fn complete_local_cancellation<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<ResidentProcessRecoveredLocalCancellation>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<(), ()> {
        panic!("fresh rediscovery must classify the retained terminal state")
    }

    fn recover_operation_resolution<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<ResidentProcessRecoveredOperationResolution>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<ResidentProcessRootProviderResolution<()>, ()> {
        panic!("fresh rediscovery must not schedule resolution")
    }

    fn recover_provider_resolved<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<ResidentProcessRecoveredProviderResolution>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<(), ()> {
        panic!("fresh rediscovery must not schedule provider selection")
    }

    fn recover_provider_prepared<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<ResidentProcessRecoveredPreparedAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<(), ()> {
        panic!("fresh rediscovery must not schedule provider preparation")
    }

    fn recover_provider_dispatched<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<ResidentProcessRecoveredAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<(), ()> {
        panic!("fresh rediscovery must not schedule a crossed provider call")
    }

    fn recover_native_prepared<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<ResidentProcessRecoveredPreparedAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<(), ()> {
        panic!("fresh rediscovery must not schedule native preparation")
    }

    fn recover_native_dispatched<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, WriterGuard>,
        _: Box<ResidentProcessRecoveredAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<(), ()> {
        panic!("fresh rediscovery must not schedule crossed native work")
    }
}

fn admit_retained_request(
    expected: &CommonRequestEnvelopeV1,
    reference: &ResidentRecordRefV1,
    wire: CommonRequestEnvelopeWireV1,
) -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1> {
    let preimage = expected.to_preimage();
    CommonRequestEnvelopeV1::admit_contextual_wire(
        wire,
        &request_admission_context(&preimage),
        |actual_preimage, digest, signature| {
            if reference.as_str() != "restart/local-cancellation-request"
                || actual_preimage != &preimage
                || digest != expected.canonical_request_sha256()
                || signature != expected.caller_signature()
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "common request envelope",
                    invariant: "exact retained cancellation request",
                });
            }
            Ok(())
        },
    )
}

fn complete_fixture(fixture: ProcessHostRestartFixture<'_>, mode: CompletionMode) -> CompletionRun {
    let ProcessHostRestartFixture {
        recovery,
        mut admission,
        signing_authority,
        current,
        evidence,
        generation,
        rediscovery_generation,
        custody,
        sessions,
        anchor,
        ..
    } = fixture;
    let retained_cancellation_request = custody
        .retained_cancellation_request()
        .expect("local cancellation fixture retains its admitted request")
        .clone();
    let selected_store = recovery.operation.context.store.clone();
    let checkpoint_authority = recovery.operation.context.checkpoint_authority;
    let selector = selector_from_signing_authority(signing_authority);
    let port = MemoryAuthorityStore::default();
    let mut host = host_with_generation(&port, &selector, &selected_store, generation);
    let (restart, rediscovery_restart, exhausted_restart) = cancellation_restart_inputs(recovery);
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
            reborrow_recovery_admission(&mut admission),
        )
        .expect("retained local cancellation classifies");
    let ResidentProcessOperationRestartDisposition::LocalCancellationRequested(recovered) =
        disposition
    else {
        panic!("pre-call cancellation has one local completion owner")
    };
    let operation = recovered.start.control.operation().operation();
    let (head_ref, head) = recovered.start.control.observation().selected_head();
    retained_cancellation_request
        .verify_operation_handle_body(operation.handle().1)
        .expect("retained cancellation names the recovered handle");
    assert_eq!(
        retained_cancellation_request.workspace_id(),
        recovered.start.control.operation().request().workspace_id()
    );
    assert_eq!(
        retained_cancellation_request.generation(),
        Some(operation.handle().1.compiled_generation_sha256())
    );
    assert_eq!(
        retained_cancellation_request.fence_token(),
        Some(operation.handle().1.activation_fence_sha256())
    );
    assert!(retained_cancellation_request.issued_at() >= *operation.handle().1.created_at());
    assert!(retained_cancellation_request.issued_at() <= *head.observed_at());
    assert!(retained_cancellation_request.expires_at() >= *head.observed_at());
    assert_eq!(
        recovered.request.record_ref.as_str(),
        "restart/local-cancellation-request"
    );
    assert_eq!(
        recovered.request.record_sha256.as_str(),
        retained_cancellation_request
            .canonical_request_sha256()
            .as_str()
    );
    assert_eq!(head.operation_handle_ref(), operation.handle().0);
    assert_eq!(
        head.operation_handle_sha256(),
        operation.handle().1.canonical_operation_handle_sha256()
    );
    assert_ne!(head_ref, operation.handle().0);
    let transaction = *recovered
        .start
        .control
        .observation()
        .current_checkpoint()
        .1
        .committed_transaction_sequence();
    let cancellation_head = recovered
        .start
        .control
        .observation()
        .selected_head()
        .0
        .clone();
    if matches!(mode, CompletionMode::FinalHeadChanged) {
        custody.remove_current_observation();
    }
    if matches!(mode, CompletionMode::FinalAnchorUnavailable) {
        anchor.make_unavailable();
    }
    let mut database = match mode {
        CompletionMode::DatabaseExactSuccessor => RecordingStore::failing_commit(
            ControlStoreError::new(ControlStoreErrorKind::Storage),
            [Ok(OperationCustodyReadBackOutcome::ExactSuccessor)],
        ),
        CompletionMode::DatabaseUnresolved | CompletionMode::DatabaseRetry => {
            RecordingStore::failing_commit(
                ControlStoreError::new(ControlStoreErrorKind::Storage),
                [Err(ControlStoreError::new(ControlStoreErrorKind::Storage))],
            )
        }
        CompletionMode::PhaseContinuity => {
            RecordingStore::committing(OperationCustodyCommitOutcome::Committed)
        }
        _ => RecordingStore::committing(OperationCustodyCommitOutcome::Committed),
    }
    .with_preparation(
        if matches!(mode, CompletionMode::PhaseContinuity) {
            reject_accumulator_preparation
        } else {
            cancellation_accumulator_response
        },
        transaction,
    );
    let mut append_anchor = match mode {
        CompletionMode::AnchorUnavailable => SequencedAppendAnchor::unavailable(),
        CompletionMode::AnchorRetry => SequencedAppendAnchor::unavailable_then_advancing(),
        _ => SequencedAppendAnchor::advancing(),
    };
    let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
    let resolver = |_: &pale_core::records::OperationObservationV1| Ok(selected_store.clone());
    let established_at = UnixTimeV1::new(current.now.get() + 20).expect("terminal time");
    let checkpoint_issued_at = UnixTimeV1::new(current.now.get() + 21).expect("checkpoint time");
    let foreign_store = store("foreign-store", "foreign-incarnation");
    let access_store = if matches!(mode, CompletionMode::ForeignStore) {
        &foreign_store
    } else {
        &selected_store
    };
    let reads_before_completion = custody.access_counts();
    let observation_signs = Rc::new(Cell::new(0));
    let checkpoint_signs = Rc::new(Cell::new(0));
    let count_observation_signs = Rc::clone(&observation_signs);
    let count_checkpoint_signs = Rc::clone(&checkpoint_signs);
    let result = {
        let mut joined_store = RecoveryAppendStore {
            recovery: &mut *custody,
            append: &mut database,
        };
        let mut joined_anchor = RecoveryAppendAnchor {
            recovery: anchor,
            append: &mut append_anchor,
        };
        let initial = host.complete_recovered_local_cancellation(
            *recovered,
            ResidentDeploymentOperationAccess {
                store: access_store,
                custody: &mut joined_store,
                sessions,
                anchor: &mut joined_anchor,
            },
            ResidentProcessLocalCancellationCompletionInput {
                current,
                maximum_request_bytes: 1024 * 1024,
                maximum_record_bytes: 1024 * 1024,
                diagnostic_ref: record_ref("restart/local-cancellation-diagnostic"),
                observation_ref: record_ref("restart/local-cancellation-terminal"),
                established_at,
                checkpoint_issued_at,
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority,
                },
                maximum_diagnostic_bytes: 1024 * 1024,
                resolve_store: &resolver,
                checkpoint: OperationCheckpointInput {
                    reference: record_ref("restart/local-cancellation-checkpoint"),
                    id: ControlCheckpointIdV1::parse("restart-local-cancellation")
                        .expect("checkpoint identity"),
                },
                checkpoint_schema: &checkpoint_schema,
                checkpoint_authority,
                accumulator_limits: AccumulatorPreparationLimits::new(
                    1, 10_000, 10_000, 4096, 100_000,
                )
                .expect("bounded cancellation accumulator"),
            },
            move |reference, wire| {
                admit_retained_request(&retained_cancellation_request, reference, wire)
            },
            |_, _, _, _| Ok(()),
            move |_, _, _| {
                count_observation_signs.set(count_observation_signs.get() + 1);
                Ok(signature())
            },
            move |_, _, _| {
                count_checkpoint_signs.set(count_checkpoint_signs.get() + 1);
                Ok(signature())
            },
        );
        continue_completion_mode(
            initial,
            mode,
            &mut host,
            &selector,
            &selected_store,
            &foreign_store,
            &resolver,
            checkpoint_authority,
            sessions,
            &mut joined_store,
            &mut joined_anchor,
            transaction,
            &checkpoint_signs,
            &mut admission,
            rediscovery_restart,
            exhausted_restart,
            &port,
            rediscovery_generation
                .expect("local cancellation retains one fresh rediscovery generation"),
        )
    };
    CompletionRun {
        result,
        database: database.mutation_counts(),
        anchor: append_anchor.access_counts(),
        provider_reservations: host.provider_routes.reserved,
        provider_checkouts: host.provider_routes.checked_out,
        provider_active: host.provider_routes.active.len(),
        task_reservations: host.operations.outstanding_task_reservations(),
        cancellation_head,
        recovery_reads: RecoveryReadCounts {
            before: reads_before_completion,
            after: custody.access_counts(),
        },
        observation_signs: observation_signs.get(),
        checkpoint_signs: checkpoint_signs.get(),
    }
}

#[allow(clippy::too_many_arguments)]
fn continue_completion_mode<WriterGuard>(
    initial: Result<
        CompletedResidentProcessLocalCancellation,
        ResidentProcessLocalCancellationCompletionFailure,
    >,
    mode: CompletionMode,
    host: &mut ResidentProcessHost<'_, WriterGuard>,
    selector: &ResidentDeploymentAuthoritySelector,
    selected_store: &pale_ports::AuthorityStoreCoordinate,
    foreign_store: &pale_ports::AuthorityStoreCoordinate,
    resolver: &dyn OperationObservationAuthorityStoreResolver,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    sessions: &dyn SessionLeaseCustodyPort,
    custody: &mut RecoveryAppendStore<'_>,
    anchor: &mut RecoveryAppendAnchor<'_, SequencedAppendAnchor>,
    transaction: pale_core::records::ControlTransactionSequenceV1,
    checkpoint_signs: &Rc<Cell<usize>>,
    admission: &mut OperationControlRecoveryAdmission<'_>,
    rediscovery_restart: ResidentProcessOperationRestartInput<'_>,
    exhausted_restart: ResidentProcessOperationRestartInput<'_>,
    port: &MemoryAuthorityStore,
    rediscovery_generation: crate::resident::AdmittedCompiledGeneration,
) -> Result<
    CompletedResidentProcessLocalCancellation,
    ResidentProcessLocalCancellationCompletionFailure,
> {
    match mode {
        CompletionMode::DatabaseRetry => {
            let Err(ResidentProcessLocalCancellationCompletionFailure::Pending(
                ResidentProcessPendingLocalCancellation::Custody(pending),
            )) = initial
            else {
                panic!("database retry starts from one exact custody token")
            };
            custody
                .append
                .replace_read_back_results([Ok(OperationCustodyReadBackOutcome::Predecessor)]);
            custody
                .append
                .replace_commit_result(Ok(OperationCustodyCommitOutcome::Committed));
            host.continue_local_cancellation_custody(
                pending,
                selector,
                continuation_access(selected_store, custody, sessions, anchor),
            )
            .map_err(|failure| {
                let pending = match failure {
                    ResidentProcessLocalCancellationCustodyFailure::Refused { pending, .. }
                    | ResidentProcessLocalCancellationCustodyFailure::Custody(pending) => pending,
                };
                ResidentProcessLocalCancellationCompletionFailure::Pending(
                    ResidentProcessPendingLocalCancellation::Custody(pending),
                )
            })
        }
        CompletionMode::AnchorRetry => {
            let Err(ResidentProcessLocalCancellationCompletionFailure::Pending(
                ResidentProcessPendingLocalCancellation::Custody(pending),
            )) = initial
            else {
                panic!("anchor retry starts from one exact custody token")
            };
            host.continue_local_cancellation_custody(
                pending,
                selector,
                continuation_access(selected_store, custody, sessions, anchor),
            )
            .map_err(|failure| {
                let pending = match failure {
                    ResidentProcessLocalCancellationCustodyFailure::Refused { pending, .. }
                    | ResidentProcessLocalCancellationCustodyFailure::Custody(pending) => pending,
                };
                ResidentProcessLocalCancellationCompletionFailure::Pending(
                    ResidentProcessPendingLocalCancellation::Custody(pending),
                )
            })
        }
        CompletionMode::PhaseContinuity => {
            let Err(ResidentProcessLocalCancellationCompletionFailure::Pending(
                ResidentProcessPendingLocalCancellation::Prepared(pending),
            )) = initial
            else {
                panic!("projection refusal retains the prepared phase")
            };
            let terminal_bytes = canon::to_canonical_string(pending.observation().1)
                .expect("terminal cancellation is canonical");
            let observation_ref = pending.observation().0.clone();
            let checkpoint_ref = pending.checkpoint_input().reference.clone();
            let checkpoint_id = pending.checkpoint_input().id.clone();
            assert_eq!(checkpoint_signs.get(), 0);

            let substituted_epoch = SigningKeyEpochV1::new(
                checkpoint_authority
                    .subject_key_epoch
                    .get()
                    .checked_add(1)
                    .expect("fixture epoch has a successor"),
            )
            .expect("positive substituted epoch");
            let substituted_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
                subject_key_epoch: substituted_epoch,
                ..*checkpoint_authority
            };
            let pending = match host.continue_prepared_local_cancellation(
                pending,
                selector,
                continuation_access(selected_store, custody, sessions, anchor),
                resolver,
                &substituted_authority,
                |_, _, _| -> Result<_, InvalidResidentRecordV1> {
                    panic!("substituted authority must fail before signing")
                },
            ) {
                Err(ResidentProcessPreparedLocalCancellationFailure::Refused {
                    pending,
                    reason: ResidentProcessLocalCancellationContinuationRefusal::CheckpointAuthority,
                }) => pending,
                _ => panic!("unexpected substituted-authority result"),
            };
            assert_eq!(
                canon::to_canonical_string(pending.observation().1).unwrap(),
                terminal_bytes
            );

            let foreign_selector =
                selector.with_test_runtime_authority_ref(record_ref("foreign-runtime-authority"));
            let pending = match host.continue_prepared_local_cancellation(
                pending,
                &foreign_selector,
                continuation_access(selected_store, custody, sessions, anchor),
                resolver,
                checkpoint_authority,
                |_, _, _| -> Result<_, InvalidResidentRecordV1> {
                    panic!("foreign selector must fail before signing")
                },
            ) {
                Err(ResidentProcessPreparedLocalCancellationFailure::Refused {
                    pending,
                    reason: ResidentProcessLocalCancellationContinuationRefusal::Selector,
                }) => pending,
                _ => panic!("unexpected foreign-selector result"),
            };
            let pending = match host.continue_prepared_local_cancellation(
                pending,
                selector,
                continuation_access(foreign_store, custody, sessions, anchor),
                resolver,
                checkpoint_authority,
                |_, _, _| -> Result<_, InvalidResidentRecordV1> {
                    panic!("foreign store must fail before signing")
                },
            ) {
                Err(ResidentProcessPreparedLocalCancellationFailure::Refused {
                    pending,
                    reason: ResidentProcessLocalCancellationContinuationRefusal::AccessStore,
                }) => pending,
                _ => panic!("unexpected foreign-store result"),
            };
            assert_eq!(pending.observation().0, &observation_ref);
            assert_eq!(pending.checkpoint_input().reference, checkpoint_ref);
            assert_eq!(pending.checkpoint_input().id, checkpoint_id);
            assert_eq!(checkpoint_signs.get(), 0);

            custody
                .append
                .replace_preparation(cancellation_accumulator_response, transaction);
            let count_checkpoint_signs = Rc::clone(checkpoint_signs);
            let issued = match host.continue_prepared_local_cancellation_with_test_attachment(
                pending,
                selector,
                continuation_access(selected_store, custody, sessions, anchor),
                resolver,
                checkpoint_authority,
                move |_, _, _| {
                    count_checkpoint_signs.set(count_checkpoint_signs.get() + 1);
                    Ok(signature())
                },
                |_, _, _| Err(OperationObservationCheckpointConstructionRejection.into()),
            ) {
                Err(ResidentProcessPreparedLocalCancellationFailure::Issued(issued)) => issued,
                _ => panic!("unexpected checkpoint-attachment result"),
            };
            assert_eq!(checkpoint_signs.get(), 1);
            assert_eq!(issued.observation().0, &observation_ref);
            assert_eq!(
                canon::to_canonical_string(issued.observation().1).unwrap(),
                terminal_bytes
            );
            let checkpoint_bytes = canon::to_canonical_string(issued.issued_checkpoint())
                .expect("issued cancellation checkpoint is canonical");

            let issued = match host.continue_issued_local_cancellation(
                issued,
                &foreign_selector,
                continuation_access(selected_store, custody, sessions, anchor),
                resolver,
            ) {
                Err(ResidentProcessIssuedLocalCancellationFailure::Refused {
                    pending,
                    reason: ResidentProcessLocalCancellationContinuationRefusal::Selector,
                }) => pending,
                _ => panic!("unexpected issued selector refusal"),
            };
            let issued = match host.continue_issued_local_cancellation(
                issued,
                selector,
                continuation_access(foreign_store, custody, sessions, anchor),
                resolver,
            ) {
                Err(ResidentProcessIssuedLocalCancellationFailure::Refused {
                    pending,
                    reason: ResidentProcessLocalCancellationContinuationRefusal::AccessStore,
                }) => pending,
                _ => panic!("unexpected issued store refusal"),
            };
            assert_eq!(
                canon::to_canonical_string(issued.issued_checkpoint()).unwrap(),
                checkpoint_bytes
            );
            let completed = host
                .continue_issued_local_cancellation(
                    issued,
                    selector,
                    continuation_access(selected_store, custody, sessions, anchor),
                    resolver,
                )
                .expect("issued checkpoint attaches without a signer or live authority");
            assert_eq!(checkpoint_signs.get(), 1);
            assert_eq!(completed.observation.0, observation_ref);
            assert_eq!(
                canon::to_canonical_string(&completed.observation.1).unwrap(),
                terminal_bytes
            );
            assert_eq!(
                canon::to_canonical_string(&completed.checkpoint.1).unwrap(),
                checkpoint_bytes
            );
            let terminal_candidate = completed.custody.candidate().clone();
            custody.recovery.adopt_completed_observation(
                selected_store,
                &terminal_candidate,
                &completed.observation.0,
                &completed.observation.1,
                &completed.checkpoint.1,
            );
            anchor.recovery.replace_with_control_head(
                selected_store,
                terminal_candidate.checkpoint().successor(),
                16,
            );
            let mut fresh_host =
                host_with_generation(port, selector, selected_store, rediscovery_generation);
            let mut owner = TerminalOnlyRecoveryOwner;
            let rediscovered = fresh_host
                .recover_one_root_provider_start(
                    selector,
                    continuation_access(
                        selected_store,
                        &mut *custody.recovery,
                        sessions,
                        &mut *anchor.recovery,
                    ),
                    rediscovery_restart,
                    reborrow_recovery_admission(admission),
                    1024 * 1024,
                    &mut owner,
                )
                .expect("fresh recovery admits the retained terminal state");
            assert!(matches!(
                rediscovered,
                ResidentProcessOperationRecoveryTurn::ReportedTerminal
            ));
            let exhausted = fresh_host
                .recover_one_root_provider_start(
                    selector,
                    continuation_access(
                        selected_store,
                        &mut *custody.recovery,
                        sessions,
                        &mut *anchor.recovery,
                    ),
                    exhausted_restart,
                    reborrow_recovery_admission(admission),
                    1024 * 1024,
                    &mut owner,
                )
                .expect("the terminal source advances the frozen recovery cursor once");
            assert!(matches!(
                exhausted,
                ResidentProcessOperationRecoveryTurn::Exhausted
            ));
            assert_eq!(
                (
                    fresh_host.provider_routes.reserved,
                    fresh_host.provider_routes.checked_out,
                    fresh_host.provider_routes.active.len(),
                    fresh_host.operations.outstanding_task_reservations(),
                ),
                (0, 0, 0, 0),
                "terminal rediscovery creates no provider route or operation task"
            );
            Ok(completed)
        }
        _ => initial,
    }
}

fn assert_no_external_work(run: &CompletionRun) {
    assert_eq!(run.provider_reservations, 0);
    assert_eq!(run.provider_checkouts, 0);
    assert_eq!(run.provider_active, 0);
    assert_eq!(run.task_reservations, 0);
}

#[test]
fn all_pre_call_phases_complete_as_cancelled_without_external_work() {
    let cases = Rc::new(Cell::new(0));
    let counted = Rc::clone(&cases);
    with_process_host_local_cancellation_restart_fixtures(&mut move |fixture| {
        let run = complete_fixture(fixture, CompletionMode::Commit);
        assert_no_external_work(&run);
        let completed = run
            .result
            .expect("local cancellation reaches terminal custody");
        let OperationObservationStateV1::Cancelled { diagnostic } = completed.observation.1.state()
        else {
            panic!("pre-call cancellation terminates as Cancelled")
        };
        assert_eq!(
            completed.observation.1.predecessor_ref().as_ref(),
            Some(&run.cancellation_head)
        );
        let record = completed
            .custody
            .candidate()
            .record(OperationCustodyRecordKind::Diagnostic)
            .expect("terminal cancellation retains its diagnostic");
        assert!(completed
            .custody
            .candidate()
            .record(OperationCustodyRecordKind::CommonRequest)
            .is_none());
        assert!(completed
            .custody
            .candidate()
            .record(OperationCustodyRecordKind::OperationHandle)
            .is_none());
        assert_eq!(diagnostic.record_ref, *record.coordinate().record_ref());
        let value: serde_json::Value =
            serde_json::from_slice(record.canonical_record_jcs().as_slice())
                .expect("terminal diagnostic JSON");
        assert_eq!(value["code"], "operation_cancelled_before_dispatch");
        assert_eq!(
            value["effect"],
            serde_json::to_value(DiagnosticEffectCertaintyV1::NotStarted).unwrap()
        );
        assert_eq!(run.database, (1, 1));
        assert_eq!(run.anchor, (1, 1));
        counted.set(counted.get() + 1);
    });
    assert_eq!(cases.get(), 4);
}

#[test]
fn database_and_anchor_ambiguity_keep_the_exact_issued_successor() {
    let ordinal = Rc::new(Cell::new(0));
    let current = Rc::clone(&ordinal);
    with_process_host_local_cancellation_restart_fixtures(&mut move |fixture| {
        let case = current.get();
        current.set(case + 1);
        let mode = match case {
            0 => CompletionMode::DatabaseExactSuccessor,
            1 => CompletionMode::DatabaseUnresolved,
            2 => CompletionMode::AnchorUnavailable,
            _ => return,
        };
        let run = complete_fixture(fixture, mode);
        assert_no_external_work(&run);
        match (mode, run.result) {
            (CompletionMode::DatabaseExactSuccessor, Ok(completed)) => {
                assert_eq!(
                    completed.custody.database_evidence(),
                    crate::resident::OperationCustodyDatabaseEvidence::ExactReplay
                );
                assert_eq!(run.database, (0, 1));
                assert_eq!(run.anchor, (1, 1));
            }
            (
                CompletionMode::DatabaseUnresolved,
                Err(ResidentProcessLocalCancellationCompletionFailure::Pending(
                    ResidentProcessPendingLocalCancellation::Custody(pending),
                )),
            ) => {
                let OperationAppendFailure::Commit {
                    checkpoint,
                    failure,
                } = pending.failure()
                else {
                    panic!("database ambiguity retains the commit phase")
                };
                assert_eq!(
                    checkpoint.0,
                    record_ref("restart/local-cancellation-checkpoint")
                );
                assert_eq!(
                    failure
                        .candidate()
                        .checkpoint()
                        .successor_record()
                        .coordinate()
                        .record_ref(),
                    &checkpoint.0
                );
                assert_eq!(run.database, (0, 1));
                assert_eq!(run.anchor, (0, 0));
            }
            (
                CompletionMode::AnchorUnavailable,
                Err(ResidentProcessLocalCancellationCompletionFailure::Pending(
                    ResidentProcessPendingLocalCancellation::Custody(pending),
                )),
            ) => {
                let OperationAppendFailure::Anchor {
                    checkpoint,
                    failure,
                } = pending.failure()
                else {
                    panic!("anchor ambiguity retains the anchor phase")
                };
                assert_eq!(
                    checkpoint.0,
                    record_ref("restart/local-cancellation-checkpoint")
                );
                assert_eq!(
                    failure
                        .candidate()
                        .checkpoint()
                        .successor_record()
                        .coordinate()
                        .record_ref(),
                    &checkpoint.0
                );
                assert_eq!(run.database, (1, 1));
                assert_eq!(run.anchor, (1, 1));
            }
            _ => panic!("unexpected cancellation ambiguity result"),
        }
    });
    assert_eq!(ordinal.get(), 4);
}

#[test]
fn foreign_store_fails_before_request_recovery_or_effects() {
    let cases = Rc::new(Cell::new(0));
    let counted = Rc::clone(&cases);
    with_process_host_local_cancellation_restart_fixtures(&mut move |fixture| {
        if counted.replace(counted.get() + 1) != 0 {
            return;
        }
        let run = complete_fixture(fixture, CompletionMode::ForeignStore);
        assert!(matches!(
            run.result,
            Err(
                ResidentProcessLocalCancellationCompletionFailure::Selection {
                    error: ResidentProcessHostSelectionError::AccessStoreAssociation,
                    ..
                }
            )
        ));
        assert_eq!(run.database, (0, 0));
        assert_eq!(run.anchor, (0, 0));
        assert_eq!(run.recovery_reads.before, run.recovery_reads.after);
        assert_no_external_work(&run);
    });
    assert_eq!(cases.get(), 4);
}

#[test]
fn final_head_and_anchor_ambiguity_return_the_exact_admission_failure() {
    let ordinal = Rc::new(Cell::new(0));
    let current = Rc::clone(&ordinal);
    with_process_host_local_cancellation_restart_fixtures(&mut move |fixture| {
        let case = current.get();
        current.set(case + 1);
        let mode = match case {
            0 => CompletionMode::FinalHeadChanged,
            1 => CompletionMode::FinalAnchorUnavailable,
            _ => return,
        };
        let run = complete_fixture(fixture, mode);
        assert_no_external_work(&run);
        assert_eq!(run.database, (0, 0));
        assert_eq!(run.anchor, (0, 0));
        match (mode, run.result) {
            (
                CompletionMode::FinalHeadChanged,
                Err(ResidentProcessLocalCancellationCompletionFailure::Admission {
                    error:
                        OperationAttemptError::Custody(OperationDispatchCustodyError::Recovery(_)),
                    ..
                }),
            ) => {}
            (
                CompletionMode::FinalAnchorUnavailable,
                Err(ResidentProcessLocalCancellationCompletionFailure::Admission {
                    error:
                        OperationAttemptError::Custody(OperationDispatchCustodyError::Anchor(error)),
                    ..
                }),
            ) => assert_eq!(error.kind(), AntiRollbackAnchorErrorKind::Unavailable),
            _ => panic!("unexpected final cancellation admission result"),
        }
    });
    assert_eq!(ordinal.get(), 4);
}

#[test]
fn signed_terminal_and_issued_checkpoint_survive_every_continuation_refusal() {
    let cases = Rc::new(Cell::new(0));
    let counted = Rc::clone(&cases);
    with_process_host_local_cancellation_restart_fixtures(&mut move |fixture| {
        let run = complete_fixture(fixture, CompletionMode::PhaseContinuity);
        assert_no_external_work(&run);
        let completed = run
            .result
            .expect("the retained issued checkpoint reaches custody");
        assert!(matches!(
            completed.observation.1.state(),
            OperationObservationStateV1::Cancelled { .. }
        ));
        assert_eq!(run.observation_signs, 1);
        assert_eq!(run.checkpoint_signs, 1);
        assert_eq!(run.database, (1, 1));
        assert_eq!(run.anchor, (1, 1));
        counted.set(counted.get() + 1);
    });
    assert_eq!(cases.get(), 4);
}

#[test]
fn database_and_anchor_custody_resume_without_reissuing_semantic_values() {
    let ordinal = Rc::new(Cell::new(0));
    let current = Rc::clone(&ordinal);
    with_process_host_local_cancellation_restart_fixtures(&mut move |fixture| {
        let case = current.get();
        current.set(case + 1);
        let mode = match case {
            0 => CompletionMode::DatabaseRetry,
            1 => CompletionMode::AnchorRetry,
            _ => return,
        };
        let run = complete_fixture(fixture, mode);
        assert_no_external_work(&run);
        let completed = run
            .result
            .expect("retained physical custody completes on retry");
        assert!(matches!(
            completed.observation.1.state(),
            OperationObservationStateV1::Cancelled { .. }
        ));
        assert_eq!(run.observation_signs, 1);
        assert_eq!(run.checkpoint_signs, 1);
        match mode {
            CompletionMode::DatabaseRetry => {
                assert_eq!(run.database, (1, 2));
                assert_eq!(run.anchor, (1, 1));
            }
            CompletionMode::AnchorRetry => {
                assert_eq!(run.database, (1, 1));
                assert_eq!(run.anchor, (2, 1));
            }
            _ => unreachable!(),
        }
    });
    assert_eq!(ordinal.get(), 4);
}
