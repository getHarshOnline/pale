//! Test-only transitions for the process-local A2A `GetTask` owner.

use super::*;
use crate::resident::start_a2a_get_task_readback;

impl ResidentA2aGetTaskOwner {
    /// Start one fresh read and retain its exact in-flight phase.
    pub(in crate::resident) fn start<Authorize>(
        &mut self,
        key: ResidentA2aTaskWorkKey,
        input: A2aGetTaskReadbackInput<'_>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        reservations: &mut ResidentAggregateTaskReservations,
        authorize: Authorize,
    ) -> Result<(), ResidentA2aGetTaskStartFailure>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), pale_core::records::InvalidResidentRecordV1>,
    {
        if key != a2a_get_task_key(&input) || self.active.iter().any(|active| active.key == key) {
            return Err(ResidentA2aGetTaskStartFailure::Association);
        }
        let request_checkpoint = {
            let (reference, checkpoint) = input.history.local_observation().current_checkpoint();
            Box::new((reference.clone(), checkpoint.clone()))
        };
        let offer = reservations
            .begin_a2a_task_offer(key.clone(), ResidentTaskReservationRoute::A2aGetTask)
            .map_err(ResidentA2aGetTaskStartFailure::Reservation)?;
        let in_flight = match start_a2a_get_task_readback(input, exchange, clock, authorize) {
            Ok(in_flight) => in_flight,
            Err(reason) => {
                return match reservations.refuse_offer(offer) {
                    Ok(()) => Err(ResidentA2aGetTaskStartFailure::Closed(reason)),
                    Err(reservation) => Err(ResidentA2aGetTaskStartFailure::Rollback {
                        reason: Box::new(reason),
                        reservation: Box::new(reservation),
                    }),
                };
            }
        };
        if let Err(reservation) = reservations.accept_offer(offer) {
            return Err(ResidentA2aGetTaskStartFailure::Acceptance {
                key: Box::new(key),
                in_flight: Box::new(in_flight),
                request_checkpoint,
                reservation: Box::new(reservation),
            });
        }
        self.active.push(ActiveA2aGetTask {
            key,
            phase: ActiveA2aGetTaskPhase::AwaitingResponse {
                in_flight: Box::new(in_flight),
                request_checkpoint,
                request_custody: None,
                disposition_basis: None,
            },
        });
        Ok(())
    }

    /// Continue only the aggregate transition after the exchange already started.
    pub(in crate::resident) fn retry_acceptance(
        &mut self,
        failure: ResidentA2aGetTaskStartFailure,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentA2aGetTaskStartFailure> {
        let ResidentA2aGetTaskStartFailure::Acceptance {
            key,
            in_flight,
            request_checkpoint,
            reservation,
        } = failure
        else {
            return Err(failure);
        };
        match reservations.retry_accept_offer(*reservation) {
            Ok(()) => {
                self.active.push(ActiveA2aGetTask {
                    key: *key,
                    phase: ActiveA2aGetTaskPhase::AwaitingResponse {
                        in_flight,
                        request_checkpoint,
                        request_custody: None,
                        disposition_basis: None,
                    },
                });
                Ok(())
            }
            Err(reservation) => Err(ResidentA2aGetTaskStartFailure::Acceptance {
                key,
                in_flight,
                request_checkpoint,
                reservation: Box::new(reservation),
            }),
        }
    }

    /// Accept and immediately stop a carrier whose aggregate transition failed.
    pub(in crate::resident) fn stop_after_acceptance_failure(
        &mut self,
        failure: ResidentA2aGetTaskStartFailure,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentA2aGetTaskPoll, ResidentA2aGetTaskStartFailure> {
        let key = match &failure {
            ResidentA2aGetTaskStartFailure::Acceptance { key, .. } => key.as_ref().clone(),
            _ => return Err(failure),
        };
        self.retry_acceptance(failure, reservations)?;
        Ok(self.stop_response_read(&key, reservations))
    }

    /// Retry only release of a closed start whose carrier never opened.
    pub(in crate::resident) fn retry_start_rollback(
        &self,
        failure: ResidentA2aGetTaskStartFailure,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<A2aGetTaskReadbackError, ResidentA2aGetTaskStartFailure> {
        let ResidentA2aGetTaskStartFailure::Rollback {
            reason,
            reservation,
        } = failure
        else {
            return Err(failure);
        };
        match reservations.retry_refuse_offer(*reservation) {
            Ok(()) => Ok(*reason),
            Err(reservation) => Err(ResidentA2aGetTaskStartFailure::Rollback {
                reason,
                reservation: Box::new(reservation),
            }),
        }
    }
}
