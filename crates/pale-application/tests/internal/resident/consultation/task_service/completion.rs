//! Completion routing preserves sibling work, cancellation evidence and ambiguous results.

use super::*;

#[test]
fn partial_completion_keeps_siblings_in_flight_and_attributable() {
    let first = candidate("partial-first");
    let second = candidate("partial-second");
    let mut service = ResidentConsultationTaskService::new(
        batch(vec![first.clone(), second.clone()]),
        limits(2, 2),
    )
    .expect("bounded service");
    let mut executor = Executor::default();
    assert!(service
        .try_accept_retained(
            retained(first.clone(), "dispatch/partial-first", 1),
            &mut executor,
        )
        .is_ok());
    assert!(service
        .try_accept_retained(
            retained(second, "dispatch/partial-second", 2),
            &mut executor,
        )
        .is_ok());

    executor.complete(0, 11);
    let Some(ResidentConsultationTaskPoll::Completed {
        candidate,
        dispatch,
        work,
        output,
        cancellation,
        ..
    }) = service.try_completion(&mut executor)
    else {
        panic!("first completion remains attributable")
    };
    assert_eq!(candidate, first);
    assert_eq!(dispatch, task_key("dispatch/partial-first"));
    assert_eq!((work, output), (1, 11));
    assert_eq!(
        cancellation,
        ResidentOperationTaskCancellationState::Unnotified
    );
    assert_eq!(service.in_flight(), 1);
}

#[test]
fn cancellation_notification_does_not_discard_late_completion() {
    let first = candidate("cancel-race");
    let mut service =
        ResidentConsultationTaskService::new(batch(vec![first.clone()]), limits(1, 1))
            .expect("bounded service");
    let mut executor = Executor::default();
    let Ok(dispatch) =
        service.try_accept_retained(retained(first, "dispatch/cancel-race", 7), &mut executor)
    else {
        panic!("accepted")
    };
    assert_eq!(
        service.note_cancellation_notification(&dispatch),
        ResidentOperationTaskCancellationNotification::Notified
    );
    executor.complete(0, 17);
    assert!(matches!(
        service.try_completion(&mut executor),
        Some(ResidentConsultationTaskPoll::Completed {
            work: 7,
            output: 17,
            cancellation: ResidentOperationTaskCancellationState::Notified,
            ..
        })
    ));
}

#[test]
fn owner_notifies_only_the_exact_retained_consultation_batch() {
    let child = candidate("cancel-owner");
    let mut owner = ResidentConsultationTaskOwner::new(owner_limits(1)).expect("bounded owner");
    let coordinate = owner
        .open_batch(batch(vec![child.clone()]), limits(1, 1))
        .expect("exact retained batch");
    let mut executor = Executor::default();
    owner
        .batch_mut(&coordinate)
        .expect("retained batch")
        .try_accept_retained(
            retained(child.clone(), "dispatch/cancel-owner", 23),
            &mut executor,
        )
        .expect("accepted child");

    let mut wrong_owner = coordinate.clone();
    wrong_owner.owner_instance = ResidentOperationTaskOwnerInstance::new();
    let mut wrong_store = coordinate.clone();
    wrong_store.store = store("other");
    let mut wrong_parent = coordinate.clone();
    wrong_parent.parent = immutable("operation/other-parent");
    let mut wrong_frontier = coordinate.clone();
    wrong_frontier.collection_frontier = immutable("collection/other-frontier");
    for wrong in [&wrong_owner, &wrong_store, &wrong_parent, &wrong_frontier] {
        assert_eq!(
            owner.note_cancellation_notification(wrong),
            ResidentConsultationCancellationNotification::NoLocalOwner
        );
    }

    let mut restarted_owner =
        ResidentConsultationTaskOwner::new(owner_limits(1)).expect("restarted owner");
    assert_eq!(
        restarted_owner.note_cancellation_notification(&coordinate),
        ResidentConsultationCancellationNotification::NoLocalOwner
    );
    assert_eq!(
        owner.note_cancellation_notification(&coordinate),
        ResidentConsultationCancellationNotification::BatchMarked {
            newly_marked: 1,
            already_marked: 0,
            no_longer_accepted: 0,
        }
    );
    assert_eq!(
        owner.note_cancellation_notification(&coordinate),
        ResidentConsultationCancellationNotification::BatchMarked {
            newly_marked: 0,
            already_marked: 1,
            no_longer_accepted: 0,
        }
    );

    executor.complete(0, 29);
    assert!(matches!(
        owner
            .batch_mut(&coordinate)
            .expect("retained batch")
            .try_completion(&mut executor),
        Some(ResidentConsultationTaskPoll::Completed {
            candidate,
            work: 23,
            output: 29,
            cancellation: ResidentOperationTaskCancellationState::Notified,
            ..
        }) if candidate == child
    ));
}

#[test]
fn completion_wins_cleanly_when_it_precedes_local_cancellation_notification() {
    let child = candidate("completion-first");
    let mut owner = ResidentConsultationTaskOwner::new(owner_limits(1)).expect("bounded owner");
    let coordinate = owner
        .open_batch(batch(vec![child.clone()]), limits(1, 1))
        .expect("exact retained batch");
    let mut executor = Executor::default();
    owner
        .batch_mut(&coordinate)
        .expect("retained batch")
        .try_accept_retained(
            retained(child.clone(), "dispatch/completion-first", 31),
            &mut executor,
        )
        .expect("accepted child");
    executor.complete(0, 37);
    assert!(matches!(
        owner
            .batch_mut(&coordinate)
            .expect("retained batch")
            .try_completion(&mut executor),
        Some(ResidentConsultationTaskPoll::Completed {
            candidate,
            work: 31,
            output: 37,
            cancellation: ResidentOperationTaskCancellationState::Unnotified,
            ..
        }) if candidate == child
    ));
    assert_eq!(
        owner.note_cancellation_notification(&coordinate),
        ResidentConsultationCancellationNotification::NoAcceptedChildren
    );
}

#[test]
fn lost_work_returns_exact_recovery_basis_without_replay() {
    let first = candidate("lost");
    let mut service =
        ResidentConsultationTaskService::new(batch(vec![first.clone()]), limits(1, 1))
            .expect("bounded service");
    let mut executor = Executor::default();
    let Ok(dispatch) =
        service.try_accept_retained(retained(first.clone(), "dispatch/lost", 9), &mut executor)
    else {
        panic!("accepted")
    };
    executor.lose(0);
    let Some(ResidentConsultationTaskPoll::Lost(lost)) = service.try_completion(&mut executor)
    else {
        panic!("lost task must retain exact recovery basis")
    };
    assert_eq!(lost.candidate(), &first);
    assert_eq!(lost.dispatch(), &dispatch);
    assert_eq!(
        lost.cancellation_state(),
        ResidentOperationTaskCancellationState::Unnotified
    );
    assert_eq!(executor.accept_calls, 1, "polling never replays lost work");
    assert_eq!(service.in_flight(), 0);
}

#[test]
fn shared_executor_routes_reverse_completions_to_their_exact_batches() {
    let mut owner = ResidentConsultationTaskOwner::new(owner_limits(2)).expect("bounded owner");
    let first = candidate("reverse-first");
    let second = candidate("reverse-second");
    let first_coordinate = owner
        .open_batch(
            batch_at(
                "operation/reverse-first",
                "collection/reverse-first",
                vec![first.clone()],
            ),
            limits(1, 1),
        )
        .expect("first batch");
    let second_coordinate = owner
        .open_batch(
            batch_at(
                "operation/reverse-second",
                "collection/reverse-second",
                vec![second.clone()],
            ),
            limits(1, 1),
        )
        .expect("second batch");
    let mut executor = Executor::default();
    owner
        .batch_mut(&first_coordinate)
        .expect("first batch")
        .try_accept_retained(retained(first, "dispatch/reverse-first", 1), &mut executor)
        .expect("first accepted");
    owner
        .batch_mut(&second_coordinate)
        .expect("second batch")
        .try_accept_retained(
            retained(second, "dispatch/reverse-second", 2),
            &mut executor,
        )
        .expect("second accepted");

    executor.complete(1, 22);
    executor.complete(0, 11);
    let Some(Ok(ResidentConsultationTaskOwnerPoll {
        coordinate,
        continuation: second_continuation,
        outcome: ResidentConsultationTaskPoll::Completed { output: 22, .. },
    })) = owner.try_completion(&mut executor)
    else {
        panic!("second completion must route without touching the first batch")
    };
    assert_eq!(coordinate, second_coordinate);
    assert_eq!(owner.in_flight(), 1);
    assert_continuation_resolved(owner.resolve_continuation(second_continuation));

    let Some(Ok(ResidentConsultationTaskOwnerPoll {
        coordinate,
        continuation: first_continuation,
        outcome: ResidentConsultationTaskPoll::Completed { output: 11, .. },
    })) = owner.try_completion(&mut executor)
    else {
        panic!("first completion remains attributable after reverse delivery")
    };
    assert_eq!(coordinate, first_coordinate);
    assert_continuation_resolved(owner.resolve_continuation(first_continuation));
}

#[test]
fn stale_completion_cannot_consume_another_batch_task() {
    let planned_candidate = candidate("stale-owner");
    let mut owner = ResidentConsultationTaskOwner::new(owner_limits(1)).expect("bounded owner");
    let coordinate = owner
        .open_batch(
            batch_at(
                "operation/stale-owner",
                "collection/stale-owner",
                vec![planned_candidate.clone()],
            ),
            limits(1, 1),
        )
        .expect("retained batch");
    let mut executor = Executor::default();
    owner
        .batch_mut(&coordinate)
        .expect("batch")
        .try_accept_retained(
            retained(planned_candidate, "dispatch/stale-owner", 3),
            &mut executor,
        )
        .expect("accepted real task");
    executor
        .completions
        .push_back(ResidentOperationTaskCompletion::Completed {
            task: ResidentOperationTask::hostile_fixture(
                owner.owner_instance.clone(),
                task_key("dispatch/foreign"),
                99,
            ),
            output: 999,
        });
    let Some(Err(ResidentConsultationTaskOwnerPollError::Stale(completion))) =
        owner.try_completion(&mut executor)
    else {
        panic!("foreign result must remain intact")
    };
    assert_eq!(owner.in_flight(), 1, "stale result removes no owned task");
    let mut reservations = ResidentAggregateTaskReservations::new(
        owner.owner_instance.clone(),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).expect("aggregate bound")),
    )
    .expect("aggregate reservations");
    let Err(ResidentConsultationTaskOwnerPollError::Stale(completion)) =
        owner.retry_completion_reserved(completion, &mut reservations)
    else {
        panic!("retry must preserve the stale completion")
    };
    assert_eq!(completion.key(), &task_key("dispatch/foreign"));
    assert_eq!(owner.in_flight(), 1);
    assert_eq!(
        executor.completion_calls, 1,
        "retry never polls the executor"
    );

    executor.complete(0, 33);
    let Some(Ok(ResidentConsultationTaskOwnerPoll {
        coordinate: routed,
        continuation,
        outcome: ResidentConsultationTaskPoll::Completed { output: 33, .. },
    })) = owner.try_completion(&mut executor)
    else {
        panic!("the real task must remain routable")
    };
    assert_eq!(routed, coordinate);
    assert_continuation_resolved(owner.resolve_continuation(continuation));
}

#[test]
fn ambiguous_completion_preserves_the_result_and_both_claimants() {
    let shared = candidate("ambiguous");
    let mut owner = ResidentConsultationTaskOwner::new(owner_limits(2)).expect("bounded owner");
    let first_coordinate = owner
        .open_batch(
            batch_at(
                "operation/ambiguous-first",
                "collection/ambiguous-first",
                vec![shared.clone()],
            ),
            limits(1, 1),
        )
        .expect("first batch");
    let second_coordinate = owner
        .open_batch(
            batch_at(
                "operation/ambiguous-second",
                "collection/ambiguous-second",
                vec![shared.clone()],
            ),
            limits(1, 1),
        )
        .expect("second batch");
    let mut executor = Executor::default();
    owner
        .batch_mut(&first_coordinate)
        .expect("first batch")
        .try_accept_retained(
            retained(shared.clone(), "dispatch/ambiguous", 7),
            &mut executor,
        )
        .expect("first hostile claimant");
    owner
        .batch_mut(&second_coordinate)
        .expect("second batch")
        .try_accept_retained(retained(shared, "dispatch/ambiguous", 8), &mut executor)
        .expect("second hostile claimant");

    executor.complete(0, 70);
    let Some(Err(ResidentConsultationTaskOwnerPollError::Ambiguous(completion))) =
        owner.try_completion(&mut executor)
    else {
        panic!("duplicate ownership must return the intact completion")
    };
    let ResidentOperationTaskCompletion::Completed { task, output } = &completion else {
        panic!("fixture completion")
    };
    assert_eq!(task.key(), &task_key("dispatch/ambiguous"));
    assert_eq!(task.work(), &7);
    assert_eq!(*output, 70);
    assert_eq!(owner.in_flight(), 2, "ambiguity removes neither claimant");
    let mut reservations = ResidentAggregateTaskReservations::new(
        owner.owner_instance.clone(),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).expect("aggregate bound")),
    )
    .expect("aggregate reservations");
    let Err(ResidentConsultationTaskOwnerPollError::Ambiguous(completion)) =
        owner.retry_completion_reserved(completion, &mut reservations)
    else {
        panic!("retry must preserve ambiguous ownership")
    };
    assert_eq!(completion.key(), &task_key("dispatch/ambiguous"));
    assert_eq!(owner.in_flight(), 2, "ambiguity removes neither claimant");
    assert_eq!(
        executor.completion_calls, 1,
        "retry never polls the executor"
    );
    assert_eq!(
        owner.retire_quiescent(&first_coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::Accepted
        )
    );
    assert_eq!(
        owner.retire_quiescent(&second_coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::Accepted
        )
    );
}
