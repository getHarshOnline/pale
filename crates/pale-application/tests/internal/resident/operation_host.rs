//! Process-lifetime laws for the private resident operation aggregate.

use std::num::NonZeroUsize;

use pale_ports::OperationCustodyCommitOutcome;

use super::*;

impl ResidentOperationHost {
    pub(in crate::resident) fn set_consultation_reservation_phase_for_test(
        &mut self,
        key: &crate::resident::task_owner::ResidentOperationTaskKey,
        coordinate: &crate::resident::consultation::ResidentConsultationBatchCoordinate,
        phase: crate::resident::task_reservation::ResidentTaskReservationPhase,
    ) {
        self.operation_tasks
            .set_consultation_reservation_phase_for_test(key, coordinate, phase);
    }

    pub(in crate::resident) fn accepted_provider_offer_failure_for_test(
        &mut self,
        key: &crate::resident::task_owner::ResidentOperationTaskKey,
    ) -> crate::resident::task_reservation::ResidentTaskReservationOfferTransitionFailure {
        self.operation_tasks
            .accepted_provider_offer_failure_for_test(key)
    }

    pub(in crate::resident) fn refused_provider_offer_failure_for_test(
        &mut self,
        key: &crate::resident::task_owner::ResidentOperationTaskKey,
    ) -> crate::resident::task_reservation::ResidentTaskReservationOfferTransitionFailure {
        self.operation_tasks
            .refused_provider_offer_failure_for_test(key)
    }

    pub(in crate::resident) fn provider_release_failure_for_test(
        &mut self,
        key: &crate::resident::task_owner::ResidentOperationTaskKey,
    ) -> crate::resident::task_reservation::ResidentTaskReservationSettlementTransitionFailure {
        self.operation_tasks.provider_release_failure_for_test(key)
    }

    pub(in crate::resident) fn set_provider_reservation_phase_for_test(
        &mut self,
        key: &crate::resident::task_owner::ResidentOperationTaskKey,
        phase: crate::resident::task_reservation::ResidentTaskReservationPhase,
    ) {
        self.operation_tasks
            .set_provider_reservation_phase_for_test(key, phase);
    }
}
use crate::resident::consultation::tests::consultation_task_batch_fixture;
use crate::resident::consultation::{
    ResidentConsultationBatchOpenError, ResidentConsultationTaskLimits,
};
use crate::resident::operation_observation_custody::tests::{RecordingAnchor, RecordingStore};
use crate::resident::operation_service::ResidentOperationTaskState;
use crate::resident::task_owner::ResidentOperationTaskLimits;
use crate::resident::task_reservation::ResidentAggregateTaskLimits;
use crate::session_admission_fixture::ProvingAuthority;

fn task_state(maximum_active_consultation_batches: usize) -> ResidentOperationTaskState {
    ResidentOperationTaskState::new(
        ResidentOperationTaskLimits::new(
            NonZeroUsize::new(1).expect("positive provider-task bound"),
        ),
        NonZeroUsize::new(maximum_active_consultation_batches)
            .expect("positive consultation-batch bound"),
        ResidentAggregateTaskLimits::new(
            NonZeroUsize::new(1).expect("positive aggregate-task bound"),
        ),
    )
    .expect("bounded operation task state")
}

fn consultation_limits() -> ResidentConsultationTaskLimits {
    ResidentConsultationTaskLimits::new(
        NonZeroUsize::new(1).expect("positive in-flight child bound"),
        NonZeroUsize::new(1).expect("positive batch-member bound"),
    )
}

fn ports() -> (ProvingAuthority, RecordingStore, RecordingAnchor) {
    (
        ProvingAuthority,
        RecordingStore::committing(OperationCustodyCommitOutcome::Committed),
        RecordingAnchor::unavailable_after_preflight(),
    )
}

#[test]
fn unused_service_borrow_has_no_port_effects() {
    let mut host = ResidentOperationHost::new(task_state(1));
    let (mut authority, mut custody, mut anchor) = ports();

    host.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
        assert_eq!(service.in_flight_provider_tasks(), 0);
        assert_eq!(service.in_flight_consultation_tasks(), 0);
        assert_eq!(service.active_consultation_batches(), 0);
    });

    assert_eq!(custody.mutation_counts(), (0, 0));
    assert_eq!(anchor.access_counts(), (0, 0));
}

#[test]
fn consultation_state_and_its_bound_survive_operation_service_borrows() {
    let mut host = ResidentOperationHost::new(task_state(1));
    let (mut authority, mut custody, mut anchor) = ports();
    let coordinate =
        host.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
            service
                .open_consultation_batch(
                    consultation_task_batch_fixture(
                        "operation/host-first",
                        "collection/host-first",
                        "host-first",
                    ),
                    consultation_limits(),
                )
                .expect("first bounded batch")
        });

    host.with_operation_service(&mut authority, &mut custody, &mut anchor, |service| {
        assert_eq!(service.active_consultation_batches(), 1);
        assert!(service.consultation_batch_mut(&coordinate).is_some());
        assert_eq!(
            service.open_consultation_batch(
                consultation_task_batch_fixture(
                    "operation/host-second",
                    "collection/host-second",
                    "host-second",
                ),
                consultation_limits(),
            ),
            Err(ResidentConsultationBatchOpenError::Saturated)
        );
    });

    assert_eq!(custody.mutation_counts(), (0, 0));
    assert_eq!(anchor.access_counts(), (0, 0));
}
