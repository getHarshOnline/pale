//! Operation hosts retain consultation work across service borrows and transition retries.

use super::*;

#[test]
fn operation_host_retries_a_returned_consult_completion_without_executor_or_port_io() {
    let selected = candidate("host-completion-transition-retry");
    let planned = batch_at(
        "operation/host-completion-transition-retry",
        "collection/host-completion-transition-retry",
        vec![selected.clone()],
    );
    let mut host = operation_host();
    let (mut authority, mut custody, mut anchor) = host_ports();
    let coordinate =
        host.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            service
                .open_consultation_batch(planned, limits(1, 1))
                .expect("consultation batch")
        });
    let dispatch = task_key("dispatch/host-completion-transition-retry");
    let mut executor = Executor::default();
    host.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
        service
            .accept_retained_consultation_for_test(
                &coordinate,
                retained(
                    selected.clone(),
                    "dispatch/host-completion-transition-retry",
                    43,
                ),
                &mut executor,
            )
            .expect("batch exists")
            .expect("task accepted");
    });
    executor.complete(0, 430);

    let completion =
        host.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            service.set_consultation_reservation_phase_for_test(
                &dispatch,
                &coordinate,
                ResidentTaskReservationPhase::Offered,
            );
            let Some(Err(ResidentConsultationTaskOwnerPollError::ReservationRejected {
                completion,
                reason: ResidentTaskReservationTransitionFailure::Phase { .. },
            })) = service.try_consultation_completion(&mut executor)
            else {
                panic!("host must return the rejected completion")
            };
            completion
        });
    assert_eq!(executor.completion_calls, 1);
    assert_eq!(custody.mutation_counts(), (0, 0));
    assert_eq!(anchor.access_counts(), (0, 0));

    let completion =
        host.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            let Err(ResidentConsultationTaskOwnerPollError::ReservationRejected {
                completion,
                reason: ResidentTaskReservationTransitionFailure::Phase { .. },
            }) = service.retry_consultation_completion_transition(completion)
            else {
                panic!("the service must preserve a repeated aggregate rejection")
            };
            service.set_consultation_reservation_phase_for_test(
                &dispatch,
                &coordinate,
                ResidentTaskReservationPhase::Accepted,
            );
            completion
        });
    let poll = host
        .retry_consultation_completion_transition(completion)
        .expect("same host routes the repaired transition");
    assert_eq!(poll.coordinate, coordinate);
    assert!(matches!(
        poll.outcome,
        ResidentConsultationTaskPoll::Completed {
            candidate,
            dispatch: completed_dispatch,
            work: 43,
            output: 430,
            cancellation: ResidentOperationTaskCancellationState::Unnotified,
        } if candidate == selected && completed_dispatch == dispatch
    ));
    assert_eq!(
        executor.completion_calls, 1,
        "retry never polls the executor"
    );
    assert_eq!(custody.mutation_counts(), (0, 0));
    assert_eq!(anchor.access_counts(), (0, 0));
}

#[test]
fn operation_task_state_keeps_consultation_work_across_short_lived_borrows() {
    let mut state = ResidentOperationTaskState::new(
        ResidentOperationTaskLimits::new(NonZeroUsize::new(1).expect("provider bound")),
        NonZeroUsize::new(2).expect("active consultation bound"),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(8).expect("aggregate-task bound")),
    )
    .expect("bounded operation task state");
    let coordinate = state
        .open_consultation_batch(
            batch_at(
                "operation/consult-state",
                "collection/frontier-state",
                vec![candidate("state")],
            ),
            limits(1, 1),
        )
        .expect("state-owned batch");
    let mut executor = Executor::default();
    state
        .accept_retained_consultation_for_test(
            &coordinate,
            retained(candidate("state"), "dispatch/state", 4),
            &mut executor,
        )
        .expect("first short-lived borrow")
        .expect("accepted state-owned work");
    assert_eq!(state.in_flight_consultation_tasks(), 1);
    assert_eq!(state.outstanding_task_reservations(), 1);

    executor.complete(0, 40);
    let Some(Ok(ResidentConsultationTaskOwnerPoll {
        coordinate: completed_coordinate,
        continuation,
        outcome:
            ResidentConsultationTaskPoll::Completed {
                work: 4,
                output: 40,
                ..
            },
    })) = state.try_consultation_completion(&mut executor)
    else {
        panic!("state owner must route the completion")
    };
    assert_eq!(completed_coordinate, coordinate);
    assert_eq!(state.in_flight_consultation_tasks(), 0);
    assert_eq!(state.outstanding_task_reservations(), 1);
    assert_continuation_resolved(state.resolve_consultation_continuation(continuation));
    assert_eq!(state.outstanding_task_reservations(), 0);
    assert_eq!(
        state.retire_consultation_batch(&coordinate),
        ResidentConsultationBatchRetirement::Retired
    );
}
