//! Aggregate reservations survive protocol loss, rejected settlement and dropped continuations.

use super::*;

#[test]
fn provider_occupancy_blocks_consult_before_lookup_clock_retention_or_executor() {
    let planned_candidate = candidate("aggregate-no-io");
    let mut state = task_state_with_aggregate(1);
    let coordinate = state
        .open_consultation_batch(batch(vec![planned_candidate.clone()]), limits(1, 1))
        .expect("bounded Consult batch");
    state.accept_provider_reservation_for_test(task_key("dispatch/provider-occupied"));

    let clock_calls = AtomicUsize::new(0);
    let lookup_calls = AtomicUsize::new(0);
    let child_calls = AtomicUsize::new(0);
    let retention_calls = AtomicUsize::new(0);
    let mut executor = Executor::default();
    let result: Result<
        ResidentOperationTaskKey,
        Box<ResidentConsultationTaskOwnerAcceptanceFailure<u8, (), ()>>,
    > = state.retain_and_accept_consultation_task(
        &coordinate,
        &planned_candidate,
        &CountingClock(&clock_calls),
        || -> Result<RecoveredConsultationCollection, ()> {
            lookup_calls.fetch_add(1, Ordering::Relaxed);
            panic!("aggregate saturation must precede parent lookup")
        },
        |_| -> Result<RecoveredOperationObservation, ()> {
            child_calls.fetch_add(1, Ordering::Relaxed);
            panic!("aggregate saturation must precede child lookup")
        },
        |_, _| -> ConsultationChildRetentionOutcome<u8, ()> {
            retention_calls.fetch_add(1, Ordering::Relaxed);
            panic!("aggregate saturation must precede retention")
        },
        &mut executor,
    );
    let failure = result.expect_err("the shared provider reservation fills the ceiling");
    assert!(matches!(
        *failure,
        ResidentConsultationTaskOwnerAcceptanceFailure::Acceptance(inner)
            if matches!(
                *inner,
                ResidentConsultationTaskAcceptanceFailure::AggregateSaturated(_)
            )
    ));
    assert_eq!(clock_calls.load(Ordering::Relaxed), 0);
    assert_eq!(lookup_calls.load(Ordering::Relaxed), 0);
    assert_eq!(child_calls.load(Ordering::Relaxed), 0);
    assert_eq!(retention_calls.load(Ordering::Relaxed), 0);
    assert_eq!(executor.accept_calls, 0);
    assert_eq!(state.outstanding_task_reservations(), 1);
}

#[test]
fn consult_executor_refusal_returns_shared_capacity() {
    let planned_candidate = candidate("aggregate-refusal");
    let mut state = task_state_with_aggregate(1);
    let coordinate = state
        .open_consultation_batch(batch(vec![planned_candidate.clone()]), limits(1, 1))
        .expect("bounded Consult batch");
    let mut executor = Executor::refusing();

    let failure = state
        .accept_retained_consultation_for_test(
            &coordinate,
            retained(planned_candidate, "dispatch/aggregate-refusal", 1),
            &mut executor,
        )
        .expect("retained batch")
        .expect_err("executor refusal returns retry-safe work");
    assert!(matches!(
        failure,
        ResidentConsultationTaskRefusal::Executor(_)
    ));
    assert_eq!(executor.accept_calls, 1);
    assert_eq!(state.outstanding_task_reservations(), 0);
}

#[test]
fn consult_protocol_loss_retains_shared_capacity() {
    let planned_candidate = candidate("aggregate-protocol-loss");
    let planned = batch(vec![planned_candidate.clone()]);
    let mut original = task_state_with_aggregate(1);
    let mut replacement = task_state_with_aggregate(1);
    let original_coordinate = original
        .open_consultation_batch(planned.clone(), limits(1, 1))
        .expect("original batch");
    let replacement_coordinate = replacement
        .open_consultation_batch(planned, limits(1, 1))
        .expect("replacement batch");
    let mut replacement_executor = Executor::default();
    replacement
        .accept_retained_consultation_for_test(
            &replacement_coordinate,
            retained(
                planned_candidate.clone(),
                "dispatch/aggregate-protocol-loss",
                2,
            ),
            &mut replacement_executor,
        )
        .expect("replacement batch")
        .expect("replacement task");
    let foreign_task = replacement_executor
        .accepted
        .pop()
        .expect("replacement executor owns one task");
    let mut substituting = SubstitutingExecutor {
        accept_calls: 0,
        returned: Some(foreign_task),
        retained: Vec::new(),
    };

    let failure = original
        .accept_retained_consultation_for_test(
            &original_coordinate,
            retained(planned_candidate, "dispatch/aggregate-protocol-loss", 1),
            &mut substituting,
        )
        .expect("original batch")
        .expect_err("substitution is executor protocol loss");
    assert!(matches!(
        failure,
        ResidentConsultationTaskRefusal::ExecutorProtocolLoss { .. }
    ));
    assert_eq!(substituting.accept_calls, 1);
    assert_eq!(original.outstanding_task_reservations(), 1);
    assert_eq!(replacement.outstanding_task_reservations(), 1);
}

#[test]
fn consultation_settlement_rejections_preserve_the_exact_completion_capability() {
    let owner_instance = ResidentOperationTaskOwnerInstance::new();
    let planned_candidate = candidate("settlement-hostile");
    let planned = batch(vec![planned_candidate.clone()]);
    let mut owner = ResidentConsultationTaskOwner::new_in(owner_instance.clone(), owner_limits(1))
        .expect("bounded owner");
    let mut reservations = ResidentAggregateTaskReservations::new(
        owner_instance,
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).expect("aggregate bound")),
    )
    .expect("aggregate reservations");
    let coordinate = owner
        .open_batch(planned, limits(1, 1))
        .expect("consultation batch");
    let mut executor = Executor::default();
    owner
        .accept_retained_for_test(
            &coordinate,
            retained(planned_candidate, "dispatch/settlement-hostile", 11),
            &mut reservations,
            &mut executor,
        )
        .expect("batch exists")
        .expect("task accepted");
    executor.complete(0, 110);
    let Some(Ok(ResidentConsultationTaskOwnerPoll {
        mut continuation,
        outcome: ResidentConsultationTaskPoll::Completed { .. },
        ..
    })) = owner.try_completion_reserved(&mut reservations, &mut executor)
    else {
        panic!("completion must retain an exact settlement capability")
    };
    let dispatch = continuation.dispatch.clone();
    let route = ResidentTaskReservationRoute::Consultation(coordinate.clone());

    continuation.kind = ResidentConsultationContinuationKind::Lost;
    continuation = owner
        .resolve_continuation_reserved(continuation, &mut reservations)
        .into_rejected_token()
        .expect("kind substitution preserves the continuation");
    continuation.kind = ResidentConsultationContinuationKind::Completed;
    assert_eq!(reservations.outstanding(), 1);

    reservations.set_phase_for_test(&dispatch, &route, ResidentTaskReservationPhase::Accepted);
    let resolution = owner.resolve_continuation_reserved(continuation, &mut reservations);
    let ResidentConsultationTaskOwnerResolution::Continuation(
        ResidentConsultationContinuationResolution::AggregateReleaseRejected {
            continuation: retained,
            reason:
                ResidentTaskReservationTransitionFailure::Phase {
                    expected: ResidentTaskReservationPhase::CompletionPendingDurableHandling,
                    actual: ResidentTaskReservationPhase::Accepted,
                },
        },
    ) = resolution
    else {
        panic!("phase substitution must retain the exact continuation")
    };
    continuation = retained;
    reservations.set_phase_for_test(
        &dispatch,
        &route,
        ResidentTaskReservationPhase::CompletionPendingDurableHandling,
    );

    let substituted_key = task_key("dispatch/settlement-substituted");
    reservations.replace_key_for_test(&dispatch, substituted_key.clone());
    let resolution = owner.resolve_continuation_reserved(continuation, &mut reservations);
    let ResidentConsultationTaskOwnerResolution::Continuation(
        ResidentConsultationContinuationResolution::AggregateReleaseRejected {
            continuation: retained,
            reason: ResidentTaskReservationTransitionFailure::NotFound,
        },
    ) = resolution
    else {
        panic!("key substitution must retain the exact continuation")
    };
    continuation = retained;
    reservations.replace_key_for_test(&substituted_key, dispatch.clone());

    reservations.replace_route_for_test(&dispatch, ResidentTaskReservationRoute::Provider);
    let resolution = owner.resolve_continuation_reserved(continuation, &mut reservations);
    let ResidentConsultationTaskOwnerResolution::Continuation(
        ResidentConsultationContinuationResolution::AggregateReleaseRejected {
            continuation: retained,
            reason: ResidentTaskReservationTransitionFailure::RouteMismatch,
        },
    ) = resolution
    else {
        panic!("route substitution must retain the exact continuation")
    };
    continuation = retained;
    reservations.replace_route_for_test(&dispatch, route);

    assert_eq!(reservations.outstanding(), 1);
    assert_eq!(
        owner.retire_quiescent(&coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::CompletionPendingJoin
        )
    );
    assert_continuation_resolved(
        owner.resolve_continuation_reserved(continuation, &mut reservations),
    );
    assert_eq!(reservations.outstanding(), 0);
    assert_eq!(
        owner.retire_quiescent(&coordinate),
        ResidentConsultationBatchRetirement::Retired
    );
}

#[test]
fn rejected_completion_transition_retries_without_polling_or_losing_cancellation_evidence() {
    let owner_instance = ResidentOperationTaskOwnerInstance::new();
    let selected = candidate("completion-transition-retry");
    let mut owner = ResidentConsultationTaskOwner::new_in(owner_instance.clone(), owner_limits(1))
        .expect("bounded owner");
    let mut reservations = ResidentAggregateTaskReservations::new(
        owner_instance,
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).expect("aggregate bound")),
    )
    .expect("aggregate reservations");
    let coordinate = owner
        .open_batch(batch(vec![selected.clone()]), limits(1, 1))
        .expect("consultation batch");
    let dispatch = task_key("dispatch/completion-transition-retry");
    let route = ResidentTaskReservationRoute::Consultation(coordinate.clone());
    let mut executor = Executor::default();
    owner
        .accept_retained_for_test(
            &coordinate,
            retained(selected.clone(), "dispatch/completion-transition-retry", 41),
            &mut reservations,
            &mut executor,
        )
        .expect("batch exists")
        .expect("task accepted");
    assert_eq!(
        owner.note_cancellation_notification(&coordinate),
        ResidentConsultationCancellationNotification::BatchMarked {
            newly_marked: 1,
            already_marked: 0,
            no_longer_accepted: 0,
        }
    );
    executor.complete(0, 410);

    reservations.set_phase_for_test(&dispatch, &route, ResidentTaskReservationPhase::Offered);
    let Some(Err(ResidentConsultationTaskOwnerPollError::ReservationRejected {
        completion,
        reason:
            ResidentTaskReservationTransitionFailure::Phase {
                expected: ResidentTaskReservationPhase::Accepted,
                actual: ResidentTaskReservationPhase::Offered,
            },
    })) = owner.try_completion_reserved(&mut reservations, &mut executor)
    else {
        panic!("the exact aggregate mismatch must preserve the returned completion")
    };
    assert_eq!(completion.key(), &dispatch);
    assert_eq!(executor.completion_calls, 1);
    assert_eq!(owner.in_flight(), 1, "rejection keeps the task active");
    assert_eq!(reservations.outstanding(), 1);

    let completion = match owner.retry_completion_reserved(completion, &mut reservations) {
        Err(ResidentConsultationTaskOwnerPollError::ReservationRejected {
            completion,
            reason:
                ResidentTaskReservationTransitionFailure::Phase {
                    expected: ResidentTaskReservationPhase::Accepted,
                    actual: ResidentTaskReservationPhase::Offered,
                },
        }) => completion,
        _ => panic!("repeated rejection must remain retryable"),
    };
    assert_eq!(completion.key(), &dispatch);
    assert_eq!(
        executor.completion_calls, 1,
        "retry never polls the executor"
    );
    assert_eq!(owner.in_flight(), 1);
    assert_eq!(reservations.outstanding(), 1);

    reservations.set_phase_for_test(&dispatch, &route, ResidentTaskReservationPhase::Accepted);
    let ResidentConsultationTaskOwnerPoll {
        coordinate: completed_coordinate,
        continuation,
        outcome:
            ResidentConsultationTaskPoll::Completed {
                candidate,
                dispatch: completed_dispatch,
                work,
                output,
                cancellation,
            },
    } = owner
        .retry_completion_reserved(completion, &mut reservations)
        .expect("the repaired aggregate transition routes once")
    else {
        panic!("completion phase is preserved across retry")
    };
    assert_eq!(completed_coordinate, coordinate);
    assert_eq!(candidate, selected);
    assert_eq!(completed_dispatch, dispatch);
    assert_eq!((work, output), (41, 410));
    assert_eq!(
        cancellation,
        ResidentOperationTaskCancellationState::Notified
    );
    assert_eq!(executor.completion_calls, 1);
    assert_eq!(owner.in_flight(), 0);
    assert_eq!(reservations.outstanding(), 1);
    assert_continuation_resolved(
        owner.resolve_continuation_reserved(continuation, &mut reservations),
    );
    assert_eq!(reservations.outstanding(), 0);
}

#[test]
fn rejected_loss_transition_retries_without_polling_or_releasing_capacity() {
    let owner_instance = ResidentOperationTaskOwnerInstance::new();
    let selected = candidate("loss-transition-retry");
    let mut owner = ResidentConsultationTaskOwner::new_in(owner_instance.clone(), owner_limits(1))
        .expect("bounded owner");
    let mut reservations = ResidentAggregateTaskReservations::new(
        owner_instance,
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).expect("aggregate bound")),
    )
    .expect("aggregate reservations");
    let coordinate = owner
        .open_batch(batch(vec![selected.clone()]), limits(1, 1))
        .expect("consultation batch");
    let dispatch = task_key("dispatch/loss-transition-retry");
    let route = ResidentTaskReservationRoute::Consultation(coordinate.clone());
    let mut executor = Executor::default();
    owner
        .accept_retained_for_test(
            &coordinate,
            retained(selected.clone(), "dispatch/loss-transition-retry", 42),
            &mut reservations,
            &mut executor,
        )
        .expect("batch exists")
        .expect("task accepted");
    executor.lose(0);

    reservations.replace_route_for_test(&dispatch, ResidentTaskReservationRoute::Provider);
    let Some(Err(ResidentConsultationTaskOwnerPollError::ReservationRejected {
        completion,
        reason: ResidentTaskReservationTransitionFailure::RouteMismatch,
    })) = owner.try_completion_reserved(&mut reservations, &mut executor)
    else {
        panic!("route substitution must preserve the exact lost completion")
    };
    assert_eq!(executor.completion_calls, 1);
    assert_eq!(owner.in_flight(), 1);
    assert_eq!(reservations.outstanding(), 1);

    let completion = match owner.retry_completion_reserved(completion, &mut reservations) {
        Err(ResidentConsultationTaskOwnerPollError::ReservationRejected {
            completion,
            reason: ResidentTaskReservationTransitionFailure::RouteMismatch,
        }) => completion,
        _ => panic!("wrong route must remain fail closed with the intact loss"),
    };
    assert_eq!(completion.key(), &dispatch);
    assert_eq!(executor.completion_calls, 1);
    assert_eq!(owner.in_flight(), 1);
    assert_eq!(reservations.outstanding(), 1);

    reservations.replace_route_for_test(&dispatch, route);
    let poll = owner
        .retry_completion_reserved(completion, &mut reservations)
        .expect("the repaired loss transition routes once");
    assert_eq!(poll.coordinate, coordinate);
    let ResidentConsultationTaskPoll::Lost(lost) = &poll.outcome else {
        panic!("loss phase is preserved across retry")
    };
    assert_eq!(lost.candidate(), &selected);
    assert_eq!(lost.dispatch(), &dispatch);
    assert_eq!(executor.completion_calls, 1);
    assert_eq!(owner.in_flight(), 0);
    assert_eq!(reservations.outstanding(), 1);
}

#[test]
fn terminal_join_preflight_rejects_every_substituted_process_coordinate() {
    let owner_instance = ResidentOperationTaskOwnerInstance::new();
    let planned_candidate = candidate("terminal-preflight");
    let mut owner = ResidentConsultationTaskOwner::new_in(owner_instance.clone(), owner_limits(1))
        .expect("bounded owner");
    let mut reservations = ResidentAggregateTaskReservations::new(
        owner_instance,
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).expect("aggregate bound")),
    )
    .expect("aggregate reservations");
    let coordinate = owner
        .open_batch(batch(vec![planned_candidate.clone()]), limits(1, 1))
        .expect("consultation batch");
    let dispatch = task_key("dispatch/terminal-preflight");
    let mut executor = Executor::default();
    owner
        .accept_retained_for_test(
            &coordinate,
            retained(planned_candidate.clone(), "dispatch/terminal-preflight", 14),
            &mut reservations,
            &mut executor,
        )
        .expect("batch exists")
        .expect("task accepted");
    executor.complete(0, 140);
    let Some(Ok(mut poll)) = owner.try_completion_reserved(&mut reservations, &mut executor) else {
        panic!("completion must retain exact terminal-join custody")
    };
    owner
        .require_pending_terminal(&poll, &reservations)
        .expect("unmodified poll owns its aggregate reservation");

    let replacement = ResidentConsultationTaskOwner::new(owner_limits(1)).expect("replacement");
    assert!(matches!(
        replacement.require_pending_terminal(&poll, &reservations),
        Err(ResidentConsultationTerminalJoinPreflightFailure::OwnerMismatch)
    ));

    poll.continuation.coordinate = ResidentConsultationBatchCoordinate::hostile_fixture(
        ResidentOperationTaskOwnerInstance::new(),
        coordinate.store().clone(),
        coordinate.parent().clone(),
        coordinate.collection_frontier().clone(),
    );
    assert!(matches!(
        owner.require_pending_terminal(&poll, &reservations),
        Err(ResidentConsultationTerminalJoinPreflightFailure::OwnerMismatch)
    ));
    poll.continuation.coordinate = coordinate.clone();

    poll.continuation.dispatch = ResidentOperationTaskKey::new(
        store("terminal-preflight-substituted"),
        dispatch.dispatch().clone(),
    );
    assert!(matches!(
        owner.require_pending_terminal(&poll, &reservations),
        Err(ResidentConsultationTerminalJoinPreflightFailure::StoreMismatch)
    ));
    poll.continuation.dispatch = dispatch.clone();

    {
        let ResidentConsultationTaskPoll::Completed {
            candidate: outcome_candidate,
            ..
        } = &mut poll.outcome
        else {
            panic!("fixture completion")
        };
        *outcome_candidate = candidate("terminal-preflight-substituted");
    }
    assert!(matches!(
        owner.require_pending_terminal(&poll, &reservations),
        Err(ResidentConsultationTerminalJoinPreflightFailure::CandidateMismatch)
    ));
    {
        let ResidentConsultationTaskPoll::Completed {
            candidate: outcome_candidate,
            dispatch: outcome_dispatch,
            ..
        } = &mut poll.outcome
        else {
            panic!("fixture completion")
        };
        *outcome_candidate = planned_candidate;
        *outcome_dispatch = task_key("dispatch/terminal-preflight-substituted");
    }
    assert!(matches!(
        owner.require_pending_terminal(&poll, &reservations),
        Err(ResidentConsultationTerminalJoinPreflightFailure::DispatchMismatch)
    ));
    {
        let ResidentConsultationTaskPoll::Completed {
            dispatch: outcome_dispatch,
            ..
        } = &mut poll.outcome
        else {
            panic!("fixture completion")
        };
        *outcome_dispatch = dispatch.clone();
    }

    let route = ResidentTaskReservationRoute::Consultation(coordinate.clone());
    reservations.set_phase_for_test(&dispatch, &route, ResidentTaskReservationPhase::Accepted);
    assert!(matches!(
        owner.require_pending_terminal(&poll, &reservations),
        Err(ResidentConsultationTerminalJoinPreflightFailure::Aggregate(
            ResidentTaskReservationTransitionFailure::Phase { .. }
        ))
    ));
    reservations.set_phase_for_test(
        &dispatch,
        &route,
        ResidentTaskReservationPhase::CompletionPendingDurableHandling,
    );
    assert_eq!(reservations.outstanding(), 1);
    drop(poll);
    assert_eq!(reservations.outstanding(), 1);
}

#[test]
fn dropped_reserved_consultation_completion_retains_aggregate_capacity() {
    let planned_candidate = candidate("dropped-reserved");
    let mut state = task_state_with_aggregate(1);
    let coordinate = state
        .open_consultation_batch(batch(vec![planned_candidate.clone()]), limits(1, 1))
        .expect("consultation batch");
    let mut executor = Executor::default();
    state
        .accept_retained_consultation_for_test(
            &coordinate,
            retained(planned_candidate, "dispatch/dropped-reserved", 12),
            &mut executor,
        )
        .expect("batch exists")
        .expect("task accepted");
    executor.complete(0, 120);
    let Some(Ok(ResidentConsultationTaskOwnerPoll { continuation, .. })) =
        state.try_consultation_completion(&mut executor)
    else {
        panic!("completion must retain its settlement capability")
    };

    drop(continuation);

    assert_eq!(state.outstanding_task_reservations(), 1);
    assert_eq!(
        state.retire_consultation_batch(&coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::AggregateReservation
        )
    );
}

#[test]
fn consultation_loss_uses_its_distinct_linear_settlement_path() {
    let owner_instance = ResidentOperationTaskOwnerInstance::new();
    let planned_candidate = candidate("loss-settlement");
    let mut owner = ResidentConsultationTaskOwner::new_in(owner_instance.clone(), owner_limits(1))
        .expect("bounded owner");
    let mut reservations = ResidentAggregateTaskReservations::new(
        owner_instance,
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).expect("aggregate bound")),
    )
    .expect("aggregate reservations");
    let coordinate = owner
        .open_batch(batch(vec![planned_candidate.clone()]), limits(1, 1))
        .expect("consultation batch");
    let mut executor = Executor::default();
    owner
        .accept_retained_for_test(
            &coordinate,
            retained(planned_candidate, "dispatch/loss-settlement", 13),
            &mut reservations,
            &mut executor,
        )
        .expect("batch exists")
        .expect("task accepted");
    executor.lose(0);
    let Some(Ok(ResidentConsultationTaskOwnerPoll {
        continuation,
        outcome: ResidentConsultationTaskPoll::Lost(lost),
        ..
    })) = owner.try_completion_reserved(&mut reservations, &mut executor)
    else {
        panic!("loss must retain an exact settlement capability")
    };
    assert_eq!(lost.dispatch(), &task_key("dispatch/loss-settlement"));
    assert_eq!(reservations.outstanding(), 1);
    assert_eq!(
        owner.retire_quiescent(&coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::LossPendingRecovery
        )
    );

    assert_continuation_resolved(
        owner.resolve_continuation_reserved(continuation, &mut reservations),
    );
    assert_eq!(reservations.outstanding(), 0);
    assert_eq!(
        owner.retire_quiescent(&coordinate),
        ResidentConsultationBatchRetirement::Retired
    );
}
