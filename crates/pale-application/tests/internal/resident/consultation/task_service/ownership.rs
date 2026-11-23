//! Retries and continuations stay with their original batch, store and process owner.

use super::*;

#[test]
fn owner_keeps_concurrent_batches_separate_by_parent_and_frontier() {
    let first = candidate("owner-first");
    let second = candidate("owner-second");
    let mut owner = ResidentConsultationTaskOwner::new(owner_limits(2)).expect("bounded owner");
    let first_coordinate = owner
        .open_batch(
            batch_at(
                "operation/consult-first",
                "collection/frontier-first",
                vec![first.clone()],
            ),
            limits(1, 1),
        )
        .expect("first batch");
    let second_coordinate = owner
        .open_batch(
            batch_at(
                "operation/consult-second",
                "collection/frontier-second",
                vec![second.clone()],
            ),
            limits(1, 1),
        )
        .expect("second batch");

    assert_eq!(owner.active_batches(), 2);
    assert_ne!(first_coordinate, second_coordinate);
    assert_eq!(
        first_coordinate.parent(),
        &immutable("operation/consult-first")
    );
    assert_eq!(
        second_coordinate.collection_frontier(),
        &immutable("collection/frontier-second")
    );

    let mut executor = Executor::default();
    owner
        .batch_mut(&first_coordinate)
        .expect("exact first batch")
        .try_accept_retained(retained(first, "dispatch/owner-first", 1), &mut executor)
        .expect("first accepted task");
    owner
        .batch_mut(&second_coordinate)
        .expect("exact second batch")
        .try_accept_retained(retained(second, "dispatch/owner-second", 2), &mut executor)
        .expect("second accepted task");
    assert_eq!(owner.in_flight(), 2);

    let mut unrelated = first_coordinate.clone();
    unrelated.collection_frontier = immutable("collection/not-the-frontier");
    assert!(owner.batch_mut(&unrelated).is_none());
}

#[test]
fn owner_refuses_duplicate_conflicting_and_excess_batches_before_work() {
    let planned = batch_at(
        "operation/consult-one",
        "collection/frontier-one",
        vec![candidate("owner-one")],
    );
    let mut owner = ResidentConsultationTaskOwner::new(owner_limits(1)).expect("bounded owner");
    owner
        .open_batch(planned.clone(), limits(1, 1))
        .expect("initial batch");
    assert_eq!(
        owner.open_batch(planned, limits(1, 1)),
        Err(ResidentConsultationBatchOpenError::Duplicate)
    );
    assert_eq!(
        owner.open_batch(
            batch_at(
                "operation/consult-one",
                "collection/frontier-successor",
                vec![candidate("owner-one")],
            ),
            limits(1, 1),
        ),
        Err(ResidentConsultationBatchOpenError::ParentAlreadyActive)
    );
    assert_eq!(
        owner.open_batch(
            batch_at(
                "operation/consult-two",
                "collection/frontier-two",
                vec![candidate("owner-two")],
            ),
            limits(1, 1),
        ),
        Err(ResidentConsultationBatchOpenError::Saturated)
    );
    assert_eq!(owner.active_batches(), 1);
    assert_eq!(owner.in_flight(), 0);
}

#[test]
fn retry_continuation_cannot_move_between_batches_or_release_its_owner() {
    let mut owner = ResidentConsultationTaskOwner::new(owner_limits(2)).expect("bounded owner");
    let first = candidate("retry-first");
    let first_coordinate = owner
        .open_batch(
            batch_at(
                "operation/retry-first",
                "collection/retry-first",
                vec![first.clone()],
            ),
            limits(1, 1),
        )
        .expect("first batch");
    let second_coordinate = owner
        .open_batch(
            batch_at(
                "operation/retry-second",
                "collection/retry-second",
                vec![candidate("retry-second")],
            ),
            limits(1, 1),
        )
        .expect("second batch");
    let mut executor = Executor::refusing();
    let Err(ResidentConsultationTaskRefusal::Executor(retry)) = owner
        .batch_mut(&first_coordinate)
        .expect("first batch")
        .try_accept_retained(retained(first, "dispatch/retry-first", 5), &mut executor)
    else {
        panic!("executor refusal must return one linear retry")
    };
    assert_eq!(
        owner.retire_quiescent(&first_coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::RetryPending
        )
    );

    let retry = owner
        .retry_accept(&first_coordinate, *retry, &mut executor)
        .expect_err("a second executor refusal keeps the retry pending")
        .into_retry()
        .expect("refusal preserves the only retry");
    assert_eq!(executor.accept_calls, 2);
    assert_eq!(
        owner.retire_quiescent(&first_coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::RetryPending
        )
    );

    let retry = owner
        .retry_accept(&second_coordinate, retry, &mut executor)
        .expect_err("another batch cannot accept the token")
        .into_retry()
        .expect("batch mismatch preserves the linear retry");
    assert_eq!(executor.accept_calls, 2, "mismatch precedes executor use");
    assert_eq!(
        owner.retire_quiescent(&first_coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::RetryPending
        )
    );

    executor.accept_next();
    owner
        .retry_accept(&first_coordinate, retry, &mut executor)
        .expect("only the exact batch can consume its retry");
    assert_eq!(executor.accept_calls, 3);
    assert_eq!(
        owner.retire_quiescent(&first_coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::Accepted
        )
    );
}

#[test]
fn retry_and_completion_continuations_cannot_cross_stores_within_one_owner() {
    let mut owner = ResidentConsultationTaskOwner::new(owner_limits(2)).expect("bounded owner");
    let first = candidate("cross-store-first");
    let first_coordinate = owner
        .open_batch(
            batch_at(
                "operation/cross-store-first",
                "collection/cross-store-first",
                vec![first.clone()],
            ),
            limits(1, 1),
        )
        .expect("first batch");
    let mut second_batch = batch_at(
        "operation/cross-store-second",
        "collection/cross-store-second",
        vec![candidate("cross-store-second")],
    );
    second_batch.store = store("secondary");
    let second_coordinate = owner
        .open_batch(second_batch, limits(1, 1))
        .expect("second-store batch");
    let mut executor = Executor::refusing();

    let Err(ResidentConsultationTaskRefusal::Executor(retry)) = owner
        .batch_mut(&first_coordinate)
        .expect("first batch")
        .try_accept_retained(
            retained(first, "dispatch/cross-store-first", 5),
            &mut executor,
        )
    else {
        panic!("executor refusal must preserve one retry")
    };
    let retry = owner
        .retry_accept(&second_coordinate, *retry, &mut executor)
        .expect_err("another store cannot consume the retry")
        .into_retry()
        .expect("store mismatch preserves the retry");
    assert_eq!(executor.accept_calls, 1, "store mismatch precedes I/O");

    executor.accept_next();
    owner
        .retry_accept(&first_coordinate, retry, &mut executor)
        .expect("the original store consumes its retry");
    executor.complete(0, 25);
    let Some(Ok(ResidentConsultationTaskOwnerPoll {
        continuation,
        outcome: ResidentConsultationTaskPoll::Completed { output: 25, .. },
        ..
    })) = owner.try_completion(&mut executor)
    else {
        panic!("exact completion must return one continuation")
    };

    let original_coordinate = continuation.coordinate.clone();
    let mut transplanted = continuation;
    transplanted.coordinate = second_coordinate.clone();
    let mut rejected = owner
        .resolve_continuation(transplanted)
        .into_rejected_token()
        .expect("store mismatch preserves the continuation");
    assert_eq!(
        owner.retire_quiescent(&second_coordinate),
        ResidentConsultationBatchRetirement::Retired,
        "rejection cannot mutate the unrelated store"
    );
    rejected.coordinate = original_coordinate;
    assert_continuation_resolved(owner.resolve_continuation(rejected));
}

#[test]
fn retry_cannot_move_between_resident_operation_hosts() {
    let planned_candidate = candidate("cross-owner-retry");
    let planned = batch_at(
        "operation/cross-owner-retry",
        "collection/cross-owner-retry",
        vec![planned_candidate.clone()],
    );
    let mut original = operation_host();
    let mut unrelated = operation_host();
    let (mut authority, mut custody, mut anchor) = host_ports();
    let original_coordinate =
        original.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            service
                .open_consultation_batch(planned.clone(), limits(1, 1))
                .expect("original batch")
        });
    let unrelated_coordinate =
        unrelated.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            service
                .open_consultation_batch(planned, limits(1, 1))
                .expect("byte-identical unrelated batch")
        });
    assert_ne!(original_coordinate, unrelated_coordinate);

    let mut executor = Executor::refusing();
    let retry =
        original.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            let Err(ResidentConsultationTaskRefusal::Executor(retry)) = service
                .consultation_batch_mut(&original_coordinate)
                .expect("original batch")
                .try_accept_retained(
                    retained(planned_candidate, "dispatch/cross-owner-retry", 5),
                    &mut executor,
                )
            else {
                panic!("executor refusal must return one linear retry")
            };
            *retry
        });
    assert_eq!(executor.accept_calls, 1);

    let retry =
        unrelated.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            service
                .retry_consultation_task(&unrelated_coordinate, retry, &mut executor)
                .expect_err("unrelated owner must reject the retry")
                .into_retry()
                .expect("owner mismatch preserves the linear retry")
        });
    assert_eq!(
        executor.accept_calls, 1,
        "owner check precedes executor use"
    );
    unrelated.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
        assert_eq!(service.in_flight_consultation_tasks(), 0)
    });

    executor.accept_next();
    original.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
        service
            .retry_consultation_task(&original_coordinate, retry, &mut executor)
            .expect("original owner still consumes its retry");
        assert_eq!(service.in_flight_consultation_tasks(), 1);
    });
    assert_eq!(executor.accept_calls, 2);
    assert_eq!(custody.mutation_counts(), (0, 0));
    assert_eq!(anchor.access_counts(), (0, 0));
}

#[test]
fn retry_cannot_replay_after_its_resident_operation_host_is_dropped() {
    let planned_candidate = candidate("dropped-host-retry");
    let planned = batch_at(
        "operation/dropped-host-retry",
        "collection/dropped-host-retry",
        vec![planned_candidate.clone()],
    );
    let (retry, original_coordinate, executor_calls) = {
        let mut original = operation_host();
        let (mut authority, mut custody, mut anchor) = host_ports();
        let coordinate =
            original.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
                service
                    .open_consultation_batch(planned.clone(), limits(1, 1))
                    .expect("original batch")
            });
        let mut executor = Executor::refusing();
        let retry =
            original.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
                let Err(ResidentConsultationTaskRefusal::Executor(retry)) = service
                    .consultation_batch_mut(&coordinate)
                    .expect("original batch")
                    .try_accept_retained(
                        retained(planned_candidate, "dispatch/dropped-host-retry", 7),
                        &mut executor,
                    )
                else {
                    panic!("executor refusal must return one linear retry")
                };
                *retry
            });
        assert_eq!(custody.mutation_counts(), (0, 0));
        assert_eq!(anchor.access_counts(), (0, 0));
        (retry, coordinate, executor.accept_calls)
    };

    let mut replacement = operation_host();
    let (mut authority, mut custody, mut anchor) = host_ports();
    let replacement_coordinate =
        replacement.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            service
                .open_consultation_batch(planned, limits(1, 1))
                .expect("byte-identical replacement batch")
        });
    assert_ne!(replacement_coordinate, original_coordinate);
    let mut executor = Executor::default();
    let failure =
        replacement.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            service
                .retry_consultation_task(&replacement_coordinate, retry, &mut executor)
                .expect_err("replacement operation host must reject the old linear retry")
        });
    assert!(matches!(
        failure,
        ResidentConsultationTaskRetryFailure::OwnerMismatch(_)
    ));
    assert_eq!(executor_calls, 1);
    assert_eq!(executor.accept_calls, 0, "rejection precedes executor use");
    assert_eq!(custody.mutation_counts(), (0, 0));
    assert_eq!(anchor.access_counts(), (0, 0));
}

#[test]
fn completed_task_and_continuation_cannot_move_between_resident_operation_hosts() {
    let planned_candidate = candidate("cross-owner-completed");
    let planned = batch_at(
        "operation/cross-owner-completed",
        "collection/cross-owner-completed",
        vec![planned_candidate.clone()],
    );
    let mut original = operation_host();
    let mut unrelated = operation_host();
    let (mut authority, mut custody, mut anchor) = host_ports();
    let original_coordinate =
        original.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            service
                .open_consultation_batch(planned.clone(), limits(1, 1))
                .expect("original batch")
        });
    let unrelated_coordinate =
        unrelated.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            service
                .open_consultation_batch(planned, limits(1, 1))
                .expect("byte-identical unrelated batch")
        });
    let mut original_executor = Executor::default();
    let mut unrelated_executor = Executor::default();
    original.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
        service
            .accept_retained_consultation_for_test(
                &original_coordinate,
                retained(
                    planned_candidate.clone(),
                    "dispatch/cross-owner-completed",
                    5,
                ),
                &mut original_executor,
            )
            .expect("original batch")
            .expect("original task");
    });
    unrelated.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
        service
            .accept_retained_consultation_for_test(
                &unrelated_coordinate,
                retained(planned_candidate, "dispatch/cross-owner-completed", 6),
                &mut unrelated_executor,
            )
            .expect("unrelated batch")
            .expect("unrelated task");
    });

    original_executor.complete(0, 15);
    unrelated_executor.completions.push_back(
        original_executor
            .completions
            .pop_front()
            .expect("original completion"),
    );
    let completion =
        unrelated.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            let Some(Err(ResidentConsultationTaskOwnerPollError::Stale(completion))) =
                service.try_consultation_completion(&mut unrelated_executor)
            else {
                panic!("unrelated owner must return the intact completion")
            };
            assert_eq!(service.in_flight_consultation_tasks(), 1);
            completion
        });

    original_executor.completions.push_back(completion);
    let continuation =
        original.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            let Some(Ok(ResidentConsultationTaskOwnerPoll { continuation, .. })) =
                service.try_consultation_completion(&mut original_executor)
            else {
                panic!("original owner must route its returned completion")
            };
            continuation
        });
    let continuation =
        unrelated.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            let continuation = service
                .resolve_consultation_continuation(continuation)
                .into_rejected_token()
                .expect("owner mismatch preserves the continuation");
            assert_eq!(service.in_flight_consultation_tasks(), 1);
            continuation
        });
    original.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
        assert_continuation_resolved(service.resolve_consultation_continuation(continuation));
    });
    assert_eq!(custody.mutation_counts(), (0, 0));
    assert_eq!(anchor.access_counts(), (0, 0));
}

#[test]
fn lost_task_and_continuation_cannot_move_between_process_owner_instances() {
    let planned_candidate = candidate("cross-owner-lost");
    let planned = batch_at(
        "operation/cross-owner-lost",
        "collection/cross-owner-lost",
        vec![planned_candidate.clone()],
    );
    let mut original = task_state();
    let mut unrelated = task_state();
    let original_coordinate = original
        .open_consultation_batch(planned.clone(), limits(1, 1))
        .expect("original batch");
    let unrelated_coordinate = unrelated
        .open_consultation_batch(planned, limits(1, 1))
        .expect("byte-identical unrelated batch");
    let mut original_executor = Executor::default();
    let mut unrelated_executor = Executor::default();
    original
        .accept_retained_consultation_for_test(
            &original_coordinate,
            retained(planned_candidate.clone(), "dispatch/cross-owner-lost", 5),
            &mut original_executor,
        )
        .expect("original batch")
        .expect("original task");
    unrelated
        .accept_retained_consultation_for_test(
            &unrelated_coordinate,
            retained(planned_candidate, "dispatch/cross-owner-lost", 6),
            &mut unrelated_executor,
        )
        .expect("unrelated batch")
        .expect("unrelated task");

    original_executor.lose(0);
    unrelated_executor.completions.push_back(
        original_executor
            .completions
            .pop_front()
            .expect("original lost completion"),
    );
    let Some(Err(ResidentConsultationTaskOwnerPollError::Stale(completion))) =
        unrelated.try_consultation_completion(&mut unrelated_executor)
    else {
        panic!("unrelated owner must return the intact loss")
    };
    assert_eq!(unrelated.in_flight_consultation_tasks(), 1);

    original_executor.completions.push_back(completion);
    let Some(Ok(ResidentConsultationTaskOwnerPoll { continuation, .. })) =
        original.try_consultation_completion(&mut original_executor)
    else {
        panic!("original owner must route its returned loss")
    };
    let continuation = unrelated
        .resolve_consultation_continuation(continuation)
        .into_rejected_token()
        .expect("owner mismatch preserves the loss continuation");
    assert_eq!(unrelated.in_flight_consultation_tasks(), 1);
    assert_continuation_resolved(original.resolve_consultation_continuation(continuation));
}
