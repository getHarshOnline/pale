//! Recovered losses retain their exact owner across capacity and offer failures.

use super::*;

#[test]
fn recovered_loss_bypasses_executor_capacity_and_enters_exact_pending_owner() {
    let accepted = candidate("recovered-loss-accepted");
    let lost_candidate = candidate("recovered-loss-cold");
    let mut service = ResidentConsultationTaskService::new(
        batch(vec![accepted.clone(), lost_candidate.clone()]),
        limits(1, 2),
    )
    .expect("bounded service");
    let coordinate = service.coordinate().clone();
    let mut executor = Executor::default();
    assert!(
        service
            .tasks
            .try_accept(
                task_key("dispatch/recovered-loss-accepted"),
                ConsultationTaskBasis {
                    candidate: accepted,
                    dispatch: task_key("dispatch/recovered-loss-accepted"),
                },
                7,
                &mut executor,
            )
            .is_ok(),
        "fill the executor-only Consult ceiling"
    );

    let basis = ConsultationTaskBasis {
        candidate: lost_candidate.clone(),
        dispatch: task_key("dispatch/recovered-loss-cold"),
    };
    let mut reservations = ResidentAggregateTaskReservations::new(
        service.tasks.owner_instance().clone(),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(2).expect("positive aggregate limit")),
    )
    .expect("aggregate ledger");
    let poll = ResidentConsultationTaskOwner {
        owner_instance: service.tasks.owner_instance().clone(),
        limits: owner_limits(1),
        batches: vec![ResidentConsultationOwnedBatch {
            coordinate: coordinate.clone(),
            tasks: service,
        }],
    }
    .commit_recovered_loss::<u8, u16>(
        PreparedResidentConsultationTaskLoss {
            coordinate: coordinate.clone(),
            basis,
        },
        &mut reservations,
    )
    .expect("cold loss does not consult executor capacity");
    assert_eq!(executor.accept_calls, 1);
    assert_eq!(reservations.outstanding(), 1);
    assert_eq!(poll.coordinate, coordinate);
    let ResidentConsultationTaskPoll::Lost(lost) = &poll.outcome else {
        panic!("cold dispatch becomes an exact lost task")
    };
    assert_eq!(lost.candidate(), &lost_candidate);
    assert_eq!(
        lost.cancellation_state(),
        ResidentOperationTaskCancellationState::Unnotified
    );
}

#[test]
fn recovered_loss_offered_transition_retains_one_offer_for_retry() {
    let candidate = candidate("recovered-loss-retry");
    let service =
        ResidentConsultationTaskService::new(batch(vec![candidate.clone()]), limits(1, 1))
            .expect("bounded service");
    let coordinate = service.coordinate().clone();
    let owner_instance = service.tasks.owner_instance().clone();
    let dispatch = task_key("dispatch/recovered-loss-retry");
    let route = ResidentTaskReservationRoute::Consultation(coordinate.clone());
    let mut reservations = ResidentAggregateTaskReservations::new(
        owner_instance.clone(),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).expect("positive aggregate limit")),
    )
    .expect("aggregate ledger");
    let offer = reservations
        .begin_offer(dispatch.clone(), route.clone())
        .expect("one exact offer");
    reservations.set_phase_for_test(&dispatch, &route, ResidentTaskReservationPhase::Accepted);
    let pending = ResidentConsultationTaskLossOffer {
        prepared: PreparedResidentConsultationTaskLoss {
            coordinate: coordinate.clone(),
            basis: ConsultationTaskBasis {
                candidate,
                dispatch: dispatch.clone(),
            },
        },
        offer,
    };
    let mut owner = ResidentConsultationTaskOwner {
        owner_instance,
        limits: owner_limits(1),
        batches: vec![ResidentConsultationOwnedBatch {
            coordinate,
            tasks: service,
        }],
    };
    let failure = match owner.retry_recovered_loss::<u8, u16>(pending, &mut reservations) {
        Err(failure) => failure,
        Ok(_) => panic!("wrong aggregate phase preserves the exact offer"),
    };
    let ResidentConsultationTaskLossCommitFailure::Offered {
        pending,
        reason:
            ResidentTaskReservationTransitionFailure::Phase {
                expected: ResidentTaskReservationPhase::Offered,
                actual: ResidentTaskReservationPhase::Accepted,
            },
    } = failure
    else {
        panic!("loss retry must preserve its offered owner")
    };
    assert_eq!(reservations.outstanding(), 1);
    reservations.set_phase_for_test(&dispatch, &route, ResidentTaskReservationPhase::Offered);
    let poll = owner
        .retry_recovered_loss::<u8, u16>(*pending, &mut reservations)
        .expect("the same offer reaches loss pending after repair");
    assert!(matches!(
        poll.outcome,
        ResidentConsultationTaskPoll::Lost(_)
    ));
    assert_eq!(reservations.outstanding(), 1);
}

#[test]
fn recovered_loss_saturation_preserves_exact_prepared_owner() {
    let selected = candidate("recovered-loss-saturated");
    let service = ResidentConsultationTaskService::new(batch(vec![selected.clone()]), limits(1, 1))
        .expect("bounded service");
    let coordinate = service.coordinate().clone();
    let owner_instance = service.tasks.owner_instance().clone();
    let dispatch = task_key("dispatch/recovered-loss-saturated");
    let mut reservations = ResidentAggregateTaskReservations::new(
        owner_instance.clone(),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).expect("positive aggregate limit")),
    )
    .expect("aggregate ledger");
    let _occupied = reservations
        .begin_offer(
            task_key("dispatch/recovered-loss-occupied"),
            ResidentTaskReservationRoute::Provider,
        )
        .expect("occupy the aggregate ledger");
    let mut owner = ResidentConsultationTaskOwner {
        owner_instance,
        limits: owner_limits(1),
        batches: vec![ResidentConsultationOwnedBatch {
            coordinate: coordinate.clone(),
            tasks: service,
        }],
    };
    let prepared = PreparedResidentConsultationTaskLoss {
        coordinate,
        basis: ConsultationTaskBasis {
            candidate: selected.clone(),
            dispatch,
        },
    };
    let failure = match owner.commit_recovered_loss::<u8, u16>(prepared, &mut reservations) {
        Err(failure) => failure,
        Ok(_) => panic!("occupied aggregate ledger refuses recovered loss"),
    };
    let ResidentConsultationTaskLossCommitFailure::Prepared { prepared, reason } = failure else {
        panic!("begin-offer refusal preserves prepared ownership")
    };
    assert_eq!(prepared.basis.candidate, selected);
    assert_eq!(reason, ResidentTaskReservationOfferFailure::Saturated);
    assert_eq!(reservations.outstanding(), 1);
}

#[test]
fn recovered_loss_duplicate_preserves_exact_prepared_owner() {
    let selected = candidate("recovered-loss-duplicate");
    let service = ResidentConsultationTaskService::new(batch(vec![selected.clone()]), limits(1, 1))
        .expect("bounded service");
    let coordinate = service.coordinate().clone();
    let owner_instance = service.tasks.owner_instance().clone();
    let dispatch = task_key("dispatch/recovered-loss-duplicate");
    let mut reservations = ResidentAggregateTaskReservations::new(
        owner_instance.clone(),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(2).expect("positive aggregate limit")),
    )
    .expect("aggregate ledger");
    let _occupied = reservations
        .begin_offer(dispatch.clone(), ResidentTaskReservationRoute::Provider)
        .expect("occupy the exact dispatch identity");
    let mut owner = ResidentConsultationTaskOwner {
        owner_instance,
        limits: owner_limits(1),
        batches: vec![ResidentConsultationOwnedBatch {
            coordinate: coordinate.clone(),
            tasks: service,
        }],
    };
    let prepared = PreparedResidentConsultationTaskLoss {
        coordinate,
        basis: ConsultationTaskBasis {
            candidate: selected.clone(),
            dispatch,
        },
    };
    let failure = match owner.commit_recovered_loss::<u8, u16>(prepared, &mut reservations) {
        Err(failure) => failure,
        Ok(_) => panic!("duplicate dispatch identity must preserve recovered-loss ownership"),
    };
    let ResidentConsultationTaskLossCommitFailure::Prepared { prepared, reason } = failure else {
        panic!("duplicate offer refusal preserves the prepared owner")
    };
    assert_eq!(prepared.basis.candidate, selected);
    assert_eq!(reason, ResidentTaskReservationOfferFailure::Duplicate);
    assert_eq!(reservations.outstanding(), 1);
}
