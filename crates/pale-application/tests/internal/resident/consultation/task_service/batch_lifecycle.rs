//! Batch retirement and frontier reconciliation preserve pending work.

use super::*;

#[test]
fn owner_retires_only_after_completed_result_context_is_released() {
    let planned = batch_at(
        "operation/consult-retire",
        "collection/frontier-retire",
        vec![candidate("owner-retire")],
    );
    let mut owner = ResidentConsultationTaskOwner::new(owner_limits(1)).expect("bounded owner");
    let coordinate = owner
        .open_batch(planned, limits(1, 1))
        .expect("retained batch");
    let mut executor = Executor::default();
    owner
        .batch_mut(&coordinate)
        .expect("exact batch")
        .try_accept_retained(
            retained(candidate("owner-retire"), "dispatch/owner-retire", 3),
            &mut executor,
        )
        .expect("accepted task");
    assert_eq!(
        owner.retire_quiescent(&coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::Accepted
        )
    );
    executor.complete(0, 30);
    let Some(Ok(ResidentConsultationTaskOwnerPoll {
        coordinate: completed_coordinate,
        continuation,
        outcome: ResidentConsultationTaskPoll::Completed { output: 30, .. },
    })) = owner.try_completion(&mut executor)
    else {
        panic!("completion must route to its exact batch")
    };
    assert_eq!(completed_coordinate, coordinate);
    assert_eq!(
        owner.retire_quiescent(&coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::CompletionPendingJoin
        )
    );
    assert_continuation_resolved(owner.resolve_continuation(continuation));
    assert_eq!(
        owner.retire_quiescent(&coordinate),
        ResidentConsultationBatchRetirement::Retired
    );
    assert_eq!(
        owner.retire_quiescent(&coordinate),
        ResidentConsultationBatchRetirement::NotFound
    );
    assert_eq!(owner.active_batches(), 0);
}

#[test]
fn lost_result_context_blocks_retirement_until_explicit_recovery() {
    let planned_candidate = candidate("owner-lost");
    let mut owner = ResidentConsultationTaskOwner::new(owner_limits(1)).expect("bounded owner");
    let coordinate = owner
        .open_batch(
            batch_at(
                "operation/owner-lost",
                "collection/owner-lost",
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
            retained(planned_candidate, "dispatch/owner-lost", 4),
            &mut executor,
        )
        .expect("accepted task");
    executor.lose(0);
    let Some(Ok(ResidentConsultationTaskOwnerPoll {
        coordinate: routed,
        continuation,
        outcome: ResidentConsultationTaskPoll::Lost(_),
    })) = owner.try_completion(&mut executor)
    else {
        panic!("loss must route to its exact batch")
    };
    assert_eq!(routed, coordinate);
    assert_eq!(
        owner.retire_quiescent(&coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::LossPendingRecovery
        )
    );
    assert_continuation_resolved(owner.resolve_continuation(continuation));
    assert_eq!(
        owner.retire_quiescent(&coordinate),
        ResidentConsultationBatchRetirement::Retired
    );
}

#[test]
fn dropped_continuation_tokens_leave_each_batch_fail_closed() {
    let completed = candidate("dropped-completion");
    let lost = candidate("dropped-loss");
    let mut owner = ResidentConsultationTaskOwner::new(owner_limits(2)).expect("bounded owner");
    let completed_coordinate = owner
        .open_batch(
            batch_at(
                "operation/dropped-completion",
                "collection/dropped-completion",
                vec![completed.clone()],
            ),
            limits(1, 1),
        )
        .expect("completion batch");
    let lost_coordinate = owner
        .open_batch(
            batch_at(
                "operation/dropped-loss",
                "collection/dropped-loss",
                vec![lost.clone()],
            ),
            limits(1, 1),
        )
        .expect("loss batch");
    let mut executor = Executor::default();
    owner
        .batch_mut(&completed_coordinate)
        .expect("completion batch")
        .try_accept_retained(
            retained(completed, "dispatch/dropped-completion", 9),
            &mut executor,
        )
        .expect("accepted completion task");
    owner
        .batch_mut(&lost_coordinate)
        .expect("loss batch")
        .try_accept_retained(retained(lost, "dispatch/dropped-loss", 10), &mut executor)
        .expect("accepted loss task");

    executor.complete(0, 90);
    executor.lose(0);
    let Some(Ok(ResidentConsultationTaskOwnerPoll {
        continuation: completed_continuation,
        outcome: ResidentConsultationTaskPoll::Completed { .. },
        ..
    })) = owner.try_completion(&mut executor)
    else {
        panic!("completion must route")
    };
    drop(completed_continuation);
    let Some(Ok(ResidentConsultationTaskOwnerPoll {
        continuation: lost_continuation,
        outcome: ResidentConsultationTaskPoll::Lost(_),
        ..
    })) = owner.try_completion(&mut executor)
    else {
        panic!("loss must route")
    };
    drop(lost_continuation);

    assert_eq!(
        owner.retire_quiescent(&completed_coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::CompletionPendingJoin
        )
    );
    assert_eq!(
        owner.retire_quiescent(&lost_coordinate),
        ResidentConsultationBatchRetirement::Busy(
            ResidentConsultationBatchRetentionReason::LossPendingRecovery
        )
    );
}

#[test]
fn batch_reconciliation_reuses_exact_and_advances_only_quiescent_frontiers() {
    let owner_instance = ResidentOperationTaskOwnerInstance::new();
    let mut owner = ResidentConsultationTaskOwner::new_in(
        owner_instance.clone(),
        ResidentConsultationTaskOwnerLimits::new(NonZeroUsize::new(2).unwrap()),
    )
    .expect("batch owner");
    let reservations = ResidentAggregateTaskReservations::new(
        owner_instance,
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(2).unwrap()),
    )
    .expect("aggregate owner");
    let first = batch_at(
        "operation/reconcile",
        "collection/frontier-1",
        vec![candidate("reconcile")],
    );
    let first_coordinate = match owner
        .reconcile_batch(first.clone(), limits(1, 1), &reservations)
        .expect("first batch")
    {
        ResidentConsultationBatchReconciliation::Opened(coordinate) => coordinate,
        _ => panic!("first reconciliation opens the batch"),
    };
    assert!(matches!(
        owner
            .reconcile_batch(first, limits(1, 1), &reservations)
            .expect("exact batch reuse"),
        ResidentConsultationBatchReconciliation::Reused(ref coordinate)
            if coordinate == &first_coordinate
    ));

    let successor = batch_at(
        "operation/reconcile",
        "collection/frontier-2",
        vec![candidate("reconcile")],
    );
    let advanced = owner
        .reconcile_batch(successor, limits(1, 1), &reservations)
        .expect("quiescent frontier advance");
    let ResidentConsultationBatchReconciliation::Advanced { retired, opened } = advanced else {
        panic!("a quiescent predecessor must advance")
    };
    assert_eq!(retired, first_coordinate);
    assert_ne!(opened.collection_frontier(), retired.collection_frontier());
    assert_eq!(owner.active_batches(), 1);
}

#[test]
fn batch_reconciliation_preserves_busy_stale_owner_and_rejects_coordinate_alias() {
    let owner_instance = ResidentOperationTaskOwnerInstance::new();
    let mut owner = ResidentConsultationTaskOwner::new_in(
        owner_instance.clone(),
        ResidentConsultationTaskOwnerLimits::new(NonZeroUsize::new(2).unwrap()),
    )
    .expect("batch owner");
    let mut reservations = ResidentAggregateTaskReservations::new(
        owner_instance,
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(2).unwrap()),
    )
    .expect("aggregate owner");
    let planned_candidate = candidate("busy-reconcile");
    let first = batch_at(
        "operation/busy-reconcile",
        "collection/frontier-1",
        vec![planned_candidate.clone()],
    );
    let coordinate = owner
        .reconcile_batch(first.clone(), limits(1, 1), &reservations)
        .expect("first batch")
        .usable()
        .expect("usable first batch")
        .clone();
    let mut executor = Executor::default();
    owner
        .seed_retained_for_supervision_test(
            &coordinate,
            planned_candidate,
            task_key("dispatch/busy-reconcile"),
            1,
            &mut reservations,
            &mut executor,
        )
        .expect("busy accepted task");

    let successor = batch_at(
        "operation/busy-reconcile",
        "collection/frontier-2",
        vec![candidate("busy-reconcile")],
    );
    assert!(matches!(
        owner
            .reconcile_batch(successor, limits(1, 1), &reservations)
            .expect("busy is a valid preserved disposition"),
        ResidentConsultationBatchReconciliation::Busy {
            ref retained,
            reason: ResidentConsultationBatchRetentionReason::AggregateReservation,
        } if retained == &coordinate
    ));
    assert_eq!(owner.active_batches(), 1);

    let alias = batch_at(
        "operation/busy-reconcile",
        "collection/frontier-1",
        vec![candidate("different-roster")],
    );
    assert!(matches!(
        owner.reconcile_batch(alias, limits(1, 1), &reservations),
        Err(ResidentConsultationBatchReconcileError::CoordinateConflict)
    ));
    assert_eq!(owner.active_batches(), 1);
}
