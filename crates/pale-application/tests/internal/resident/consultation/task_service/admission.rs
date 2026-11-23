//! Bounded admission, duplicate dispatch and unopened executor refusals.

use super::*;

#[test]
fn finite_batch_and_in_flight_ceilings_bound_all_process_local_work() {
    let first = candidate("first");
    let second = candidate("second");
    assert!(matches!(
        ResidentConsultationTaskService::new(
            batch(vec![first.clone(), second.clone()]),
            limits(1, 1),
        ),
        Err(ResidentConsultationTaskBuildError::BatchTooLarge)
    ));

    let mut service = ResidentConsultationTaskService::new(
        batch(vec![first.clone(), second.clone()]),
        limits(1, 2),
    )
    .expect("bounded service");
    let mut executor = Executor::default();
    assert!(service
        .try_accept_retained(retained(first, "dispatch/first", 1), &mut executor)
        .is_ok());
    let Err(ResidentConsultationTaskRefusal::Saturated(retry)) =
        service.try_accept_retained(retained(second, "dispatch/second", 2), &mut executor)
    else {
        panic!("the occupied bound must return unopened retry-safe work")
    };
    assert_eq!(executor.accept_calls, 1);
    assert_eq!(service.in_flight(), 1);
    executor.complete(0, 10);
    assert!(matches!(
        service.try_completion(&mut executor),
        Some(ResidentConsultationTaskPoll::Completed { output: 10, .. })
    ));
    let coordinate = service.coordinate().clone();
    assert!(service
        .retry_accept(&coordinate, *retry, &mut executor)
        .is_ok());
    assert_eq!(executor.accept_calls, 2);
    assert_eq!(service.in_flight(), 1);
}

#[test]
fn duplicate_dispatch_never_reaches_the_executor_twice() {
    let first = candidate("duplicate");
    let mut service =
        ResidentConsultationTaskService::new(batch(vec![first.clone()]), limits(2, 1))
            .expect("bounded service");
    let mut executor = Executor::default();
    assert!(service
        .try_accept_retained(
            retained(first.clone(), "dispatch/duplicate", 1),
            &mut executor
        )
        .is_ok());
    let second = retained(first, "dispatch/duplicate", 2);
    let expected_candidate = second.basis.candidate.clone();
    let expected_dispatch = second.basis.dispatch.clone();
    let Err(duplicate) = service.try_accept_retained(second, &mut executor) else {
        panic!("duplicate dispatch must be refused")
    };
    assert!(matches!(
        &duplicate,
        ResidentConsultationTaskRefusal::Duplicate(_)
    ));
    assert_eq!(executor.accept_calls, 1);
    executor.complete(0, 10);
    assert!(matches!(
        service.try_completion(&mut executor),
        Some(ResidentConsultationTaskPoll::Completed { output: 10, .. })
    ));
    assert_eq!(service.in_flight(), 0);
    let (returned_candidate, returned_dispatch, returned_work) = duplicate
        .into_duplicate()
        .expect("duplicate refusal returns its exact unopened work");
    assert_eq!(returned_candidate, expected_candidate);
    assert_eq!(returned_dispatch, expected_dispatch);
    assert_eq!(returned_work, 2);
}

#[test]
fn resumed_duplicate_returns_exact_work_without_a_second_executor_offer() {
    let selected = candidate("resumed-duplicate");
    let mut service =
        ResidentConsultationTaskService::new(batch(vec![selected.clone()]), limits(2, 1))
            .expect("bounded service");
    let coordinate = service.coordinate().clone();
    let first = retained(selected.clone(), "dispatch/resumed-duplicate", 1);
    let basis = first.basis.clone();
    let mut executor = Executor::default();
    assert!(service
        .tasks
        .try_accept(
            basis.dispatch.clone(),
            first.basis,
            first.work,
            &mut executor,
        )
        .is_ok());
    assert_eq!(executor.accept_calls, 1);

    let expected_candidate = basis.candidate.clone();
    let expected_dispatch = basis.dispatch.clone();
    let prepared = PreparedResidentConsultationTaskResume { coordinate, basis };
    let mut reservations = service.legacy_reservations();
    let failure = service
        .accept_prepared_resume_reserved(prepared, &mut reservations, 2, &mut executor)
        .expect_err("resumed duplicate must remain non-retryable");
    let ResidentConsultationPreparedResumeFailure::Refused(refusal) = failure else {
        panic!("exact prepared ownership must reach duplicate classification")
    };
    let (returned_candidate, returned_dispatch, returned_work) = refusal
        .into_duplicate()
        .expect("resumed duplicate returns its exact unopened work");
    assert_eq!(returned_candidate, expected_candidate);
    assert_eq!(returned_dispatch, expected_dispatch);
    assert_eq!(returned_work, 2);
    assert_eq!(
        executor.accept_calls, 1,
        "duplicate work never executes twice"
    );
}

#[test]
fn executor_refusal_returns_the_only_unopened_token_for_retry() {
    let first = candidate("executor-refusal");
    let mut service =
        ResidentConsultationTaskService::new(batch(vec![first.clone()]), limits(1, 1))
            .expect("bounded service");
    let mut executor = Executor::refusing();
    let Err(ResidentConsultationTaskRefusal::Executor(refusal)) = service.try_accept_retained(
        retained(first, "dispatch/executor-refusal", 5),
        &mut executor,
    ) else {
        panic!("executor must return the retained task unopened")
    };
    assert_eq!(executor.accept_calls, 1);
    assert_eq!(service.in_flight(), 0);

    executor.accept_next();
    let coordinate = service.coordinate().clone();
    assert!(service
        .retry_accept(&coordinate, *refusal, &mut executor)
        .is_ok());
    assert_eq!(executor.accept_calls, 2);
    assert_eq!(service.in_flight(), 1);
}

#[test]
fn first_offers_from_shared_owner_batches_cannot_be_swapped() {
    let owner_instance = ResidentOperationTaskOwnerInstance::new();
    let first_candidate = candidate("shared-offer-first");
    let second_candidate = candidate("shared-offer-second");
    let mut first = ResidentConsultationTaskService::new_in(
        batch_at(
            "operation/shared-offer-first",
            "collection/shared-offer-first",
            vec![first_candidate.clone()],
        ),
        limits(1, 1),
        owner_instance.clone(),
    )
    .expect("first shared-owner batch");
    let mut second = ResidentConsultationTaskService::new_in(
        batch_at(
            "operation/shared-offer-second",
            "collection/shared-offer-second",
            vec![second_candidate.clone()],
        ),
        limits(1, 1),
        owner_instance,
    )
    .expect("second shared-owner batch");
    let shared_dispatch = "dispatch/shared-first-offer";
    let mut second_executor = Executor::default();
    second
        .try_accept_retained(
            retained(second_candidate.clone(), shared_dispatch, 2),
            &mut second_executor,
        )
        .expect("second batch first offer");
    let returned_second = second_executor
        .accepted
        .pop()
        .expect("second batch accepted task");
    let mut substituting = SubstitutingExecutor {
        accept_calls: 0,
        returned: Some(returned_second),
        retained: Vec::new(),
    };

    let Err(ResidentConsultationTaskRefusal::ExecutorProtocolLoss {
        expected_key,
        returned,
    }) = first.try_accept_retained(
        retained(first_candidate.clone(), shared_dispatch, 1),
        &mut substituting,
    )
    else {
        panic!("a first-offer swap must be protocol loss")
    };
    assert_eq!(expected_key, task_key(shared_dispatch));
    assert_eq!(first.in_flight(), 1);
    assert_eq!(second.in_flight(), 1);
    assert_eq!(substituting.accept_calls, 1);

    let mut second_completion = Executor::default();
    second_completion
        .completions
        .push_back(ResidentOperationTaskCompletion::Completed {
            task: *returned,
            output: 22,
        });
    let Some(ResidentConsultationTaskPoll::Completed {
        candidate,
        work: 2,
        output: 22,
        ..
    }) = second.try_completion(&mut second_completion)
    else {
        panic!("returned second task must join only its original batch")
    };
    assert_eq!(candidate, second_candidate);
    assert_eq!(second.in_flight(), 0);

    let first_task = substituting
        .retained
        .pop()
        .expect("first offered task remains retained");
    let mut first_completion = Executor::default();
    first_completion
        .completions
        .push_back(first_task.into_lost());
    let Some(ResidentConsultationTaskPoll::Lost(lost)) =
        first.try_completion(&mut first_completion)
    else {
        panic!("lost first task must join only its original batch")
    };
    assert_eq!(lost.candidate(), &first_candidate);
    assert_eq!(first.in_flight(), 0);
}
