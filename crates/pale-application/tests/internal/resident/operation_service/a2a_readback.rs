//! Hostile support for exact `GetTask` aggregate-release custody.

use super::*;
use crate::resident::a2a_get_task_owner::ResidentA2aGetTaskStartFailure;
use crate::resident::task_owner::ResidentOperationTaskOwnerInstance;
use crate::resident::task_reservation::{
    ResidentAggregateTaskLimits, ResidentAggregateTaskReservations,
};
use std::num::NonZeroUsize;

impl<'ports, Authority> ResidentOperationService<'ports, Authority> {
    pub(in crate::resident) fn start_owned_a2a_get_task<Authorize>(
        &mut self,
        key: ResidentA2aTaskWorkKey,
        input: A2aGetTaskReadbackInput<'_>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
    ) -> Result<(), ResidentA2aGetTaskStartFailure>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        self.tasks.a2a_get_task_owner.start(
            key,
            input,
            exchange,
            clock,
            &mut self.tasks.reservations,
            authorize,
        )
    }

    pub(in crate::resident) fn retry_owned_a2a_get_task_acceptance(
        &mut self,
        failure: ResidentA2aGetTaskStartFailure,
    ) -> Result<(), ResidentA2aGetTaskStartFailure> {
        self.tasks
            .a2a_get_task_owner
            .retry_acceptance(failure, &mut self.tasks.reservations)
    }

    pub(in crate::resident) fn stop_owned_a2a_get_task_after_acceptance_failure(
        &mut self,
        failure: ResidentA2aGetTaskStartFailure,
    ) -> Result<ResidentA2aGetTaskPoll, ResidentA2aGetTaskStartFailure> {
        self.tasks
            .a2a_get_task_owner
            .stop_after_acceptance_failure(failure, &mut self.tasks.reservations)
    }

    pub(in crate::resident) fn retry_owned_a2a_get_task_start_rollback(
        &mut self,
        failure: ResidentA2aGetTaskStartFailure,
    ) -> Result<A2aGetTaskReadbackError, ResidentA2aGetTaskStartFailure> {
        self.tasks
            .a2a_get_task_owner
            .retry_start_rollback(failure, &mut self.tasks.reservations)
    }

    pub(in crate::resident) fn finish_closed_a2a_get_task(
        &mut self,
        closed: ResidentA2aGetTaskClosedDisposition,
    ) -> Result<A2aGetTaskReadbackError, ResidentA2aGetTaskFinishFailure> {
        self.tasks
            .a2a_get_task_owner
            .finish_closed(closed, &mut self.tasks.reservations)
    }

    pub(in crate::resident) fn finish_stopped_a2a_get_task_response_read(
        &mut self,
        stopped: ResidentA2aGetTaskReadStoppedDisposition,
    ) -> Result<(), ResidentA2aGetTaskReadStopFinishFailure> {
        self.tasks
            .a2a_get_task_owner
            .finish_stopped_response_read(stopped, &mut self.tasks.reservations)
    }

    pub(in crate::resident) fn reject_get_task_release_with_replacement_for_test(
        &mut self,
        retained: ResidentRetainedA2aGetTask,
    ) -> ResidentA2aGetTaskReleaseFailure {
        let ResidentRetainedA2aGetTask {
            key,
            settlement,
            custody,
        } = retained;
        let mut replacement = ResidentAggregateTaskReservations::new(
            ResidentOperationTaskOwnerInstance::new(),
            ResidentAggregateTaskLimits::new(
                NonZeroUsize::new(1).expect("positive replacement-ledger bound"),
            ),
        )
        .expect("replacement hostile ledger");
        let release = replacement
            .release_completion(*settlement)
            .expect_err("a replacement ledger must preserve the exact settlement");
        ResidentA2aGetTaskReleaseFailure::Settlement {
            key,
            custody,
            release: Box::new(release),
        }
    }
}
