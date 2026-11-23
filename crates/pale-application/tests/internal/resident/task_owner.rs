//! Behavioral laws for process-local resident operation task ownership.

use super::*;
use crate::resident::consultation::ResidentConsultationBatchCoordinate;
use crate::resident::task_reservation::{
    ResidentAggregateTaskLimits, ResidentAggregateTaskReservations, ResidentTaskReservationPhase,
    ResidentTaskReservationRoute, ResidentTaskReservationTransitionFailure,
};
use pale_core::records::{
    ArtifactDigest, ControlIncarnationIdV1, ControlStoreIdV1, ImmutableRecordReferenceV1,
    ResidentRecordRefV1,
};
use pale_ports::AuthorityStoreCoordinate;
use std::collections::VecDeque;

pub(in crate::resident) fn substitute_task_key<Work>(
    mut task: ResidentOperationTask<Work>,
    key: ResidentOperationTaskKey,
) -> ResidentOperationTask<Work> {
    task.key = key;
    task
}

impl ResidentOperationTaskOwnerInstance {
    fn exhaust_offer_ids_for_test(&self) {
        self.0.last_offer_id.store(u64::MAX, Ordering::Relaxed);
    }
}

impl<Work> ResidentOperationTask<Work> {
    pub(in crate::resident) fn hostile_fixture(
        owner_instance: ResidentOperationTaskOwnerInstance,
        key: ResidentOperationTaskKey,
        work: Work,
    ) -> Self {
        Self::new(
            owner_instance,
            ResidentOperationTaskOfferId(
                NonZeroU64::new(u64::MAX).expect("positive fixture offer"),
            ),
            key,
            work,
        )
    }
}

impl<Basis> ResidentOperationTaskOwner<Basis> {
    pub(in crate::resident) fn new(
        limits: ResidentOperationTaskLimits,
    ) -> Result<Self, ResidentOperationTaskOwnerBuildError> {
        Self::new_in(ResidentOperationTaskOwnerInstance::new(), limits)
    }
}

fn store(value: &str) -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse(format!("store-{value}")).expect("store identity"),
        ControlIncarnationIdV1::parse("incarnation-1").expect("store incarnation"),
    )
}

fn store_incarnation(store_id: &str, incarnation_id: &str) -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse(store_id).expect("store identity"),
        ControlIncarnationIdV1::parse(incarnation_id).expect("store incarnation"),
    )
}

fn key(value: &str) -> ResidentOperationTaskKey {
    ResidentOperationTaskKey::new(
        store("primary"),
        ImmutableRecordReferenceV1 {
            record_ref: ResidentRecordRefV1::parse(value).expect("task record reference"),
            record_sha256: ArtifactDigest::hash_bytes(value.as_bytes()),
        },
    )
}

fn reference(value: &str) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse(value).expect("record reference"),
        record_sha256: ArtifactDigest::hash_bytes(value.as_bytes()),
    }
}

#[derive(Default)]
struct Executor {
    accept_calls: usize,
    refuse: bool,
    accepted: Vec<ResidentOperationTask<u8>>,
    completions: VecDeque<ResidentOperationTaskCompletion<u8, u16>>,
}

impl Executor {
    fn complete(&mut self, index: usize, output: u16) {
        let task = self.accepted.remove(index);
        self.completions
            .push_back(ResidentOperationTaskCompletion::Completed { task, output });
    }

    fn lose(&mut self, index: usize) {
        let task = self.accepted.remove(index);
        self.completions.push_back(task.into_lost());
    }
}

impl ResidentOperationTaskExecutor<u8, u16> for Executor {
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<u8>,
    ) -> Result<(), ResidentOperationTask<u8>> {
        self.accept_calls += 1;
        if self.refuse {
            Err(task)
        } else {
            self.accepted.push(task);
            Ok(())
        }
    }

    fn try_completion(&mut self) -> Option<ResidentOperationTaskCompletion<u8, u16>> {
        self.completions.pop_front()
    }
}

struct SubstitutingExecutor {
    accept_calls: usize,
    returned: Option<ResidentOperationTask<u8>>,
    retained: Vec<ResidentOperationTask<u8>>,
}

impl ResidentOperationTaskExecutor<u8, u16> for SubstitutingExecutor {
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<u8>,
    ) -> Result<(), ResidentOperationTask<u8>> {
        self.accept_calls += 1;
        self.retained.push(task);
        Err(self.returned.take().expect("one substituted task"))
    }

    fn try_completion(&mut self) -> Option<ResidentOperationTaskCompletion<u8, u16>> {
        None
    }
}

fn owner(limit: usize) -> ResidentOperationTaskOwner<&'static str> {
    ResidentOperationTaskOwner::new_in(
        ResidentOperationTaskOwnerInstance::new(),
        ResidentOperationTaskLimits::new(NonZeroUsize::new(limit).expect("positive test limit")),
    )
    .expect("task owner")
}

#[test]
fn cancellation_before_acceptance_has_no_executor_call() {
    let mut owner = owner(1);
    let mut executor = Executor::default();

    assert_eq!(
        owner.note_cancellation_notification(&key("task-before-acceptance")),
        ResidentOperationTaskCancellationNotification::NotAccepted
    );
    assert_eq!(executor.accept_calls, 0);
    assert_eq!(owner.in_flight(), 0);
    assert!(owner.try_completion(&mut executor).is_none());
}

#[test]
fn accepted_work_retains_a_cancellation_notification_and_late_completion() {
    let mut owner = owner(1);
    let mut executor = Executor::default();
    let task_key = key("task-late-completion");

    let Ok(accepted) = owner.try_accept(task_key.clone(), "basis", 7, &mut executor) else {
        panic!("task must be accepted")
    };
    assert_eq!(accepted, task_key);
    assert_eq!(executor.accept_calls, 1);
    assert_eq!(owner.in_flight(), 1);
    assert_eq!(
        owner.note_cancellation_notification(&task_key),
        ResidentOperationTaskCancellationNotification::Notified
    );
    assert_eq!(
        owner.note_cancellation_notification(&task_key),
        ResidentOperationTaskCancellationNotification::AlreadyNotified
    );

    executor.complete(0, 11);
    let Some(ResidentOperationTaskJoin::Completed {
        basis,
        task,
        output,
        cancellation,
    }) = owner.try_completion(&mut executor)
    else {
        panic!("late completion must return to its exact accepted basis")
    };
    assert_eq!(basis, "basis");
    assert_eq!(task.into_parts(), (task_key, 7));
    assert_eq!(output, 11);
    assert_eq!(
        cancellation,
        ResidentOperationTaskCancellationState::Notified
    );
    assert_eq!(owner.in_flight(), 0);
}

#[test]
fn duplicate_and_saturated_work_never_reach_the_executor() {
    let mut owner = owner(1);
    let mut executor = Executor::default();
    let first = key("task-first");

    assert!(owner
        .try_accept(first.clone(), "first", 1, &mut executor)
        .is_ok());
    assert_eq!(executor.accept_calls, 1);
    assert!(matches!(
        owner.try_accept(first, "duplicate", 2, &mut executor),
        Err(ResidentOperationTaskRefusal::Duplicate {
            basis: "duplicate",
            work: 2
        })
    ));
    assert!(matches!(
        owner.try_accept(key("task-second"), "second", 3, &mut executor),
        Err(ResidentOperationTaskRefusal::Saturated {
            basis: "second",
            work: 3
        })
    ));
    assert_eq!(executor.accept_calls, 1);
}

#[test]
fn executor_refusal_returns_unopened_basis_and_work() {
    let mut owner = owner(1);
    let mut executor = Executor {
        refuse: true,
        ..Executor::default()
    };

    assert!(matches!(
        owner.try_accept(key("task-refused"), "basis", 5, &mut executor),
        Err(ResidentOperationTaskRefusal::Executor {
            basis: "basis",
            work: 5
        })
    ));
    assert_eq!(executor.accept_calls, 1);
    assert_eq!(owner.in_flight(), 0);
}

#[test]
fn stale_and_duplicate_completions_cannot_advance_an_active_task() {
    let mut owner = owner(1);
    let mut executor = Executor::default();
    let active_key = key("task-active");
    assert!(owner
        .try_accept(active_key.clone(), "active", 9, &mut executor)
        .is_ok());

    executor
        .completions
        .push_back(ResidentOperationTaskCompletion::Completed {
            task: ResidentOperationTask::hostile_fixture(
                owner.owner_instance().clone(),
                key("task-stale"),
                4,
            ),
            output: 12,
        });
    assert!(matches!(
        owner.try_completion(&mut executor),
        Some(ResidentOperationTaskJoin::Stale(_))
    ));
    assert_eq!(owner.in_flight(), 1);

    executor.complete(0, 13);
    assert!(matches!(
        owner.try_completion(&mut executor),
        Some(ResidentOperationTaskJoin::Completed {
            basis: "active",
            output: 13,
            ..
        })
    ));
    executor.completions.push_back(
        ResidentOperationTask::hostile_fixture(owner.owner_instance().clone(), active_key, 0)
            .into_lost(),
    );
    assert!(matches!(
        owner.try_completion(&mut executor),
        Some(ResidentOperationTaskJoin::Stale(_))
    ));
}

#[test]
fn lost_executor_work_returns_basis_for_durable_recovery_without_replay() {
    let mut owner = owner(1);
    let mut executor = Executor::default();
    let task_key = key("task-lost");
    assert!(owner
        .try_accept(task_key.clone(), "basis", 6, &mut executor)
        .is_ok());
    executor.lose(0);

    let Some(ResidentOperationTaskJoin::Lost {
        basis,
        key,
        cancellation,
    }) = owner.try_completion(&mut executor)
    else {
        panic!("lost work must return its basis for protected recovery")
    };
    assert_eq!(basis, "basis");
    assert_eq!(key, task_key);
    assert_eq!(
        cancellation,
        ResidentOperationTaskCancellationState::Unnotified
    );
    assert_eq!(owner.in_flight(), 0);
    assert_eq!(executor.accept_calls, 1);
}

fn transition_failure_preserves_completion_and_active_task(
    route_for: impl FnOnce(&ResidentOperationTaskOwnerInstance) -> ResidentTaskReservationRoute,
) {
    let mut owner = owner(1);
    let owner_instance = owner.owner_instance().clone();
    let route = route_for(&owner_instance);
    let task_key = key("task-reservation-transition");
    let mut reservations = ResidentAggregateTaskReservations::new(
        owner_instance,
        ResidentAggregateTaskLimits::new(
            NonZeroUsize::new(1).expect("positive aggregate test limit"),
        ),
    )
    .expect("aggregate reservations");
    let offer = reservations
        .begin_offer(task_key.clone(), route.clone())
        .expect("aggregate offer");
    reservations.accept_offer(offer).expect("accepted offer");
    let mut executor = Executor::default();
    assert!(owner
        .try_accept(task_key.clone(), "basis", 7, &mut executor)
        .is_ok());
    executor.complete(0, 11);
    let completion = executor
        .completions
        .pop_front()
        .expect("one exact completion");
    reservations.set_phase_for_test(&task_key, &route, ResidentTaskReservationPhase::Offered);

    let ResidentReservedOperationTaskJoin::ReservationRejected { completion, reason } =
        owner.join_reserved_completion(completion, &mut reservations, route.clone())
    else {
        panic!("the wrong aggregate phase must return the exact completion")
    };
    assert!(matches!(
        reason,
        ResidentTaskReservationTransitionFailure::Phase {
            expected: ResidentTaskReservationPhase::Accepted,
            actual: ResidentTaskReservationPhase::Offered,
        }
    ));
    assert_eq!(owner.in_flight(), 1);
    assert_eq!(reservations.outstanding(), 1);

    reservations.set_phase_for_test(&task_key, &route, ResidentTaskReservationPhase::Accepted);
    let ResidentReservedOperationTaskJoin::Completed {
        basis,
        key,
        output,
        settlement,
        ..
    } = owner.join_reserved_completion(completion, &mut reservations, route)
    else {
        panic!("restoring the expected phase must admit the same completion")
    };
    assert_eq!(basis, "basis");
    assert_eq!(key, task_key);
    assert_eq!(output, 11);
    assert_eq!(owner.in_flight(), 0);
    assert_eq!(reservations.outstanding(), 1);
    reservations
        .release_completion(settlement)
        .expect("durable handling releases the exact reservation");
    assert_eq!(reservations.outstanding(), 0);
}

#[test]
fn provider_completion_transition_failure_keeps_task_and_completion_intact() {
    transition_failure_preserves_completion_and_active_task(|_| {
        ResidentTaskReservationRoute::Provider
    });
}

#[test]
fn consult_completion_transition_failure_keeps_task_and_completion_intact() {
    transition_failure_preserves_completion_and_active_task(|owner_instance| {
        ResidentTaskReservationRoute::Consultation(
            ResidentConsultationBatchCoordinate::hostile_fixture(
                owner_instance.clone(),
                store("primary"),
                reference("consult/parent"),
                reference("consult/frontier"),
            ),
        )
    });
}

#[test]
fn identical_dispatches_in_distinct_stores_are_distinct_tasks() {
    let dispatch = ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse("task/shared-dispatch")
            .expect("task record reference"),
        record_sha256: ArtifactDigest::hash_bytes(b"shared dispatch"),
    };
    let first = ResidentOperationTaskKey::new(store("first"), dispatch.clone());
    let second = ResidentOperationTaskKey::new(store("second"), dispatch);
    let mut owner = owner(2);
    let mut executor = Executor::default();

    assert!(owner
        .try_accept(first.clone(), "first", 1, &mut executor)
        .is_ok());
    assert!(owner
        .try_accept(second.clone(), "second", 2, &mut executor)
        .is_ok());
    assert_eq!(executor.accept_calls, 2);
    assert_eq!(owner.in_flight(), 2);
    assert!(owner.owns(&first));
    assert!(owner.owns(&second));
}

#[test]
fn identical_dispatches_in_distinct_store_incarnations_are_distinct_tasks() {
    let dispatch = ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse("task/shared-incarnation-dispatch")
            .expect("task record reference"),
        record_sha256: ArtifactDigest::hash_bytes(b"shared incarnation dispatch"),
    };
    let first = ResidentOperationTaskKey::new(
        store_incarnation("shared-store", "incarnation-1"),
        dispatch.clone(),
    );
    let second =
        ResidentOperationTaskKey::new(store_incarnation("shared-store", "incarnation-2"), dispatch);
    let mut owner = owner(2);
    let mut executor = Executor::default();

    assert!(owner
        .try_accept(first.clone(), "first", 1, &mut executor)
        .is_ok());
    assert!(owner
        .try_accept(second.clone(), "second", 2, &mut executor)
        .is_ok());
    assert!(owner.owns(&first));
    assert!(owner.owns(&second));
    assert_eq!(owner.in_flight(), 2);
}

#[test]
fn substituted_owner_never_becomes_retry_safe_but_the_offered_task_stays_owned() {
    let offered_key = key("task/substituted-offer");
    let mut owner = owner(1);
    let returned = ResidentOperationTask::hostile_fixture(
        ResidentOperationTaskOwnerInstance::new(),
        offered_key.clone(),
        41,
    );
    let mut executor = SubstitutingExecutor {
        accept_calls: 0,
        returned: Some(returned),
        retained: Vec::new(),
    };

    let Err(ResidentOperationTaskRefusal::ExecutorProtocolLoss {
        unopened_basis,
        expected_key,
        returned,
    }) = owner.try_accept(offered_key.clone(), "basis", 7, &mut executor)
    else {
        panic!("substitution must become non-retry-safe protocol loss")
    };
    assert_eq!(unopened_basis, None);
    assert_eq!(expected_key, offered_key);
    assert_eq!(returned.into_parts().1, 41);
    assert_eq!(executor.accept_calls, 1);
    assert_eq!(executor.retained.len(), 1, "offered work may have escaped");
    assert_eq!(owner.in_flight(), 1);

    let original = executor.retained.pop().expect("retained original offer");
    let mut completion_executor = Executor::default();
    completion_executor
        .completions
        .push_back(ResidentOperationTaskCompletion::Completed {
            task: original,
            output: 71,
        });
    assert!(matches!(
        owner.try_completion(&mut completion_executor),
        Some(ResidentOperationTaskJoin::Completed {
            basis: "basis",
            output: 71,
            ..
        })
    ));
    assert_eq!(owner.in_flight(), 0);
}

#[test]
fn substituted_store_never_becomes_retry_safe_but_the_offered_task_stays_owned() {
    let offered_key = key("task/substituted-store");
    let mut owner = owner(1);
    let returned = ResidentOperationTask::hostile_fixture(
        owner.owner_instance().clone(),
        ResidentOperationTaskKey::new(store("substituted"), offered_key.dispatch().clone()),
        42,
    );
    let mut executor = SubstitutingExecutor {
        accept_calls: 0,
        returned: Some(returned),
        retained: Vec::new(),
    };

    let Err(ResidentOperationTaskRefusal::ExecutorProtocolLoss {
        unopened_basis,
        expected_key,
        returned,
    }) = owner.try_accept(offered_key.clone(), "basis", 7, &mut executor)
    else {
        panic!("store substitution must become non-retry-safe protocol loss")
    };
    assert_eq!(unopened_basis, None);
    assert_eq!(expected_key, offered_key);
    assert_eq!(returned.into_parts().1, 42);
    assert_eq!(executor.accept_calls, 1);
    assert_eq!(owner.in_flight(), 1);

    let original = executor.retained.pop().expect("retained original offer");
    let mut completion_executor = Executor::default();
    completion_executor
        .completions
        .push_back(ResidentOperationTaskCompletion::Completed {
            task: original,
            output: 72,
        });
    assert!(matches!(
        owner.try_completion(&mut completion_executor),
        Some(ResidentOperationTaskJoin::Completed {
            basis: "basis",
            output: 72,
            ..
        })
    ));
    assert_eq!(owner.in_flight(), 0);
}

#[test]
fn substituted_work_with_the_same_owner_and_key_is_protocol_loss() {
    let mut owner = owner(1);
    let task_key = key("task/substituted-work");
    let returned = ResidentOperationTask::hostile_fixture(
        owner.owner_instance().clone(),
        task_key.clone(),
        99,
    );
    let mut executor = SubstitutingExecutor {
        accept_calls: 0,
        returned: Some(returned),
        retained: Vec::new(),
    };

    let Err(ResidentOperationTaskRefusal::ExecutorProtocolLoss {
        unopened_basis,
        expected_key,
        returned,
    }) = owner.try_accept(task_key.clone(), "basis", 7, &mut executor)
    else {
        panic!("an unlike offer identity must expose altered work as protocol loss")
    };
    assert_eq!(unopened_basis, None);
    assert_eq!(expected_key, task_key);
    assert_eq!(returned.into_parts().1, 99);
    assert_eq!(executor.retained[0].work(), &7);
    assert_eq!(owner.in_flight(), 1);

    let original = executor.retained.pop().expect("retained original offer");
    let mut completion_executor = Executor::default();
    completion_executor
        .completions
        .push_back(ResidentOperationTaskCompletion::Completed {
            task: original,
            output: 73,
        });
    assert!(matches!(
        owner.try_completion(&mut completion_executor),
        Some(ResidentOperationTaskJoin::Completed {
            basis: "basis",
            output: 73,
            ..
        })
    ));
    assert_eq!(owner.in_flight(), 0);
}

#[test]
fn substituted_split_basis_keeps_only_the_accepted_basis_active() {
    let mut owner = owner(1);
    let task_key = key("task/substituted-split-basis");
    let returned = ResidentOperationTask::hostile_fixture(
        owner.owner_instance().clone(),
        task_key.clone(),
        99,
    );
    let mut executor = SubstitutingExecutor {
        accept_calls: 0,
        returned: Some(returned),
        retained: Vec::new(),
    };

    let Err(ResidentOperationTaskRefusal::ExecutorProtocolLoss {
        unopened_basis,
        expected_key,
        returned,
    }) = owner.try_accept_with(
        task_key.clone(),
        "accepted-basis",
        "unopened-basis",
        7,
        &mut executor,
    )
    else {
        panic!("a substituted split-basis offer must fail closed")
    };
    assert_eq!(unopened_basis, Some("unopened-basis"));
    assert_eq!(expected_key, task_key);
    assert_eq!(returned.into_parts().1, 99);
    assert_eq!(owner.in_flight(), 1);

    let original = executor.retained.pop().expect("retained original offer");
    let mut completion_executor = Executor::default();
    completion_executor
        .completions
        .push_back(ResidentOperationTaskCompletion::Completed {
            task: original,
            output: 74,
        });
    assert!(matches!(
        owner.try_completion(&mut completion_executor),
        Some(ResidentOperationTaskJoin::Completed {
            basis: "accepted-basis",
            output: 74,
            ..
        })
    ));
    assert_eq!(owner.in_flight(), 0);
}

#[test]
fn exhausted_offer_identity_fails_before_executor_access() {
    let owner_instance = ResidentOperationTaskOwnerInstance::new();
    let mut owner = ResidentOperationTaskOwner::new_in(
        owner_instance.clone(),
        ResidentOperationTaskLimits::new(NonZeroUsize::new(1).expect("positive test limit")),
    )
    .expect("task owner");
    owner_instance.exhaust_offer_ids_for_test();
    let mut executor = Executor::default();

    assert!(matches!(
        owner.try_accept(key("task/exhausted-offer"), "basis", 7, &mut executor),
        Err(ResidentOperationTaskRefusal::OfferIdentityExhausted {
            basis: "basis",
            work: 7
        })
    ));
    assert_eq!(executor.accept_calls, 0);
    assert_eq!(owner.in_flight(), 0);
}

#[test]
fn wrong_owner_completion_is_returned_intact_for_the_original_owner() {
    let mut original = owner(1);
    let mut unrelated = owner(1);
    let mut original_executor = Executor::default();
    let mut unrelated_executor = Executor::default();
    let task_key = key("task/owner-confined");

    assert!(original
        .try_accept(task_key.clone(), "original", 7, &mut original_executor)
        .is_ok());
    assert!(unrelated
        .try_accept(task_key, "unrelated", 8, &mut unrelated_executor)
        .is_ok());
    original_executor.complete(0, 17);
    let completion = original_executor
        .completions
        .pop_front()
        .expect("original completion");
    unrelated_executor.completions.push_back(completion);

    let Some(ResidentOperationTaskJoin::Stale(completion)) =
        unrelated.try_completion(&mut unrelated_executor)
    else {
        panic!("wrong owner must return the intact completion")
    };
    assert_eq!(unrelated.in_flight(), 1);
    original_executor.completions.push_back(completion);
    assert!(matches!(
        original.try_completion(&mut original_executor),
        Some(ResidentOperationTaskJoin::Completed {
            basis: "original",
            output: 17,
            ..
        })
    ));
    assert_eq!(original.in_flight(), 0);
    assert_eq!(unrelated.in_flight(), 1);
}
