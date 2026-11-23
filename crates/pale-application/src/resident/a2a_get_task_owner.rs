//! Process-local ownership for one-shot A2A `GetTask` reads.
//!
//! Durable task history remains authoritative. This owner only keeps a started
//! read and its shared capacity until the completed response reaches durable
//! local remote-history custody or an explicit closed disposition.

use std::{num::NonZeroUsize, task::Context};

use pale_core::records::{ControlStateCheckpointV1, ResidentRecordRefV1};
use pale_ports::{A2aExchangePort, WallClockSourcePort};

use super::a2a_task_custody::{
    A2aGetTaskLocalDispositionBasis, A2aGetTaskReadbackError, A2aGetTaskReadbackInput,
    A2aGetTaskReadbackPoll, A2aGetTaskStartFailure as DurableA2aGetTaskStartFailure,
    CompletedA2aGetTaskReadback, InFlightA2aGetTaskReadback, ReceivedA2aGetTaskReadback,
    RetainedA2aGetTask,
};
use super::task_owner::{ResidentOperationTaskOwnerBuildError, ResidentOperationTaskOwnerInstance};
use super::task_reservation::{
    ResidentA2aTaskReservationOfferFailure, ResidentA2aTaskWorkKey,
    ResidentAggregateTaskReservations, ResidentTaskReservationOffer,
    ResidentTaskReservationOfferTransitionFailure, ResidentTaskReservationRoute,
    ResidentTaskReservationSettlement, ResidentTaskReservationTransitionFailure,
};
use super::A2aGetTaskReadbackAuthorizationContext;

enum ActiveA2aGetTaskPhase {
    AwaitingResponse {
        in_flight: Box<InFlightA2aGetTaskReadback>,
        request_checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        request_custody: Option<Box<super::CompletedOperationObservationCustody>>,
        disposition_basis: Option<Box<A2aGetTaskLocalDispositionBasis>>,
    },
    AwaitingEof {
        received: Box<ReceivedA2aGetTaskReadback>,
        request_checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        request_custody: Option<Box<super::CompletedOperationObservationCustody>>,
        disposition_basis: Option<Box<A2aGetTaskLocalDispositionBasis>>,
    },
}

/// Derive the private work key from one binding-qualified read input.
pub(in crate::resident) fn a2a_get_task_key(
    input: &A2aGetTaskReadbackInput<'_>,
) -> ResidentA2aTaskWorkKey {
    let binding = input.history.binding();
    ResidentA2aTaskWorkKey {
        store: input.history.local_observation().store().clone(),
        operation_handle_ref: binding.1.operation_handle_ref().clone(),
        operation_handle_sha256: binding.1.operation_handle_sha256().clone(),
        task_binding_id: binding.1.task_binding_id().clone(),
        task_binding_ref: binding.0.clone(),
        task_binding_sha256: binding.1.canonical_a2a_task_binding_sha256().clone(),
    }
}

struct ActiveA2aGetTask {
    key: ResidentA2aTaskWorkKey,
    phase: ActiveA2aGetTaskPhase,
}

/// Offered aggregate capacity retained while one request gains physical custody.
#[must_use = "retain through physical custody or release before carrier use"]
pub(in crate::resident) struct ResidentA2aGetTaskReservation {
    owner_instance: ResidentOperationTaskOwnerInstance,
    pub(in crate::resident) key: Box<ResidentA2aTaskWorkKey>,
    offer: Box<ResidentTaskReservationOffer>,
}

/// Aggregate-offer release stopped before any carrier call.
#[must_use = "retry exact unopened capacity release"]
pub(in crate::resident) enum ResidentA2aGetTaskReservationReleaseFailure {
    Association(ResidentA2aGetTaskReservation),
    Transition {
        owner_instance: ResidentOperationTaskOwnerInstance,
        key: Box<ResidentA2aTaskWorkKey>,
        failure: Box<ResidentTaskReservationOfferTransitionFailure>,
    },
}

impl ResidentA2aGetTaskReservationReleaseFailure {
    /// Borrow the binding-qualified key retained by the failed release.
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        match self {
            Self::Association(reservation) => &reservation.key,
            Self::Transition { key, .. } => key,
        }
    }
}

/// Durable unopened request whose aggregate acceptance needs a retry.
#[must_use = "retry aggregate acceptance before carrier release"]
pub(in crate::resident) struct ResidentA2aGetTaskAggregateAcceptanceFailure<'a> {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: Box<ResidentA2aTaskWorkKey>,
    retained: Box<RetainedA2aGetTask<'a>>,
    failure: Box<ResidentTaskReservationOfferTransitionFailure>,
}

impl ResidentA2aGetTaskAggregateAcceptanceFailure<'_> {
    /// Borrow the key whose unopened aggregate acceptance needs a retry.
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.key
    }
}

/// Aggregate-accepted durable request that has not called the carrier.
#[must_use = "apply fresh preflight before carrier release"]
pub(in crate::resident) struct ResidentA2aGetTaskAggregateAccepted<'a> {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: Box<ResidentA2aTaskWorkKey>,
    retained: Box<RetainedA2aGetTask<'a>>,
}

impl ResidentA2aGetTaskAggregateAccepted<'_> {
    /// Borrow the key whose aggregate capacity is already accepted.
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.key
    }
}

/// Fresh authority refused one durable unopened request.
#[must_use = "retry only this accepted unopened request"]
pub(in crate::resident) struct ResidentA2aGetTaskPreflightFailure<'a> {
    pub(in crate::resident) accepted: ResidentA2aGetTaskAggregateAccepted<'a>,
    pub(in crate::resident) reason: A2aGetTaskReadbackError,
}

impl ResidentA2aGetTaskPreflightFailure<'_> {
    /// Borrow the exact key retained by this fresh-preflight refusal.
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        self.accepted.key()
    }
}

/// Result of releasing one physically retained request to the carrier.
#[must_use = "preserve every unopened or carrier-started phase"]
pub(in crate::resident) enum ResidentA2aGetTaskDurableStart<'a> {
    Active,
    PreflightRejected(ResidentA2aGetTaskPreflightFailure<'a>),
    Terminal(ResidentA2aGetTaskPoll),
}

/// Aggregate acceptance stopped before one durable request reached the carrier.
#[must_use = "retry only the exact unopened aggregate transition"]
pub(in crate::resident) enum ResidentA2aGetTaskDurableStartFailure<'a> {
    Association {
        reservation: ResidentA2aGetTaskReservation,
        retained: Box<RetainedA2aGetTask<'a>>,
    },
    AggregateAcceptance(ResidentA2aGetTaskAggregateAcceptanceFailure<'a>),
}

/// Stable owner of started `GetTask` reads for one resident lifetime.
pub(in crate::resident) struct ResidentA2aGetTaskOwner {
    owner_instance: ResidentOperationTaskOwnerInstance,
    active: Vec<ActiveA2aGetTask>,
}

impl ResidentA2aGetTaskOwner {
    /// Reserve all task slots before the owner can start carrier work.
    pub(in crate::resident) fn new_in(
        owner_instance: ResidentOperationTaskOwnerInstance,
        maximum_outstanding: NonZeroUsize,
    ) -> Result<Self, ResidentOperationTaskOwnerBuildError> {
        let mut active = Vec::new();
        active
            .try_reserve_exact(maximum_outstanding.get())
            .map_err(|_| ResidentOperationTaskOwnerBuildError::Allocation)?;
        Ok(Self {
            owner_instance,
            active,
        })
    }

    /// Reserve binding-wide capacity before signing, storage, or carrier work.
    pub(in crate::resident) fn reserve(
        &self,
        key: ResidentA2aTaskWorkKey,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentA2aGetTaskReservation, ResidentA2aTaskReservationOfferFailure> {
        let offer = reservations
            .begin_a2a_task_offer(key.clone(), ResidentTaskReservationRoute::A2aGetTask)?;
        Ok(ResidentA2aGetTaskReservation {
            owner_instance: self.owner_instance.clone(),
            key: Box::new(key),
            offer: Box::new(offer),
        })
    }

    /// Return offered capacity after preparation proved no carrier use.
    pub(in crate::resident) fn release_unopened(
        &self,
        reservation: ResidentA2aGetTaskReservation,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentA2aGetTaskReservationReleaseFailure> {
        if !self.owner_instance.is_same(&reservation.owner_instance) {
            return Err(ResidentA2aGetTaskReservationReleaseFailure::Association(
                reservation,
            ));
        }
        match reservations.refuse_offer(*reservation.offer) {
            Ok(()) => Ok(()),
            Err(failure) => Err(ResidentA2aGetTaskReservationReleaseFailure::Transition {
                owner_instance: self.owner_instance.clone(),
                key: reservation.key,
                failure: Box::new(failure),
            }),
        }
    }

    /// Retry only exact offered-capacity release before carrier use.
    pub(in crate::resident) fn retry_unopened_release(
        &self,
        failure: ResidentA2aGetTaskReservationReleaseFailure,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentA2aGetTaskReservationReleaseFailure> {
        let ResidentA2aGetTaskReservationReleaseFailure::Transition {
            owner_instance,
            key,
            failure: transition,
        } = failure
        else {
            return Err(failure);
        };
        if !self.owner_instance.is_same(&owner_instance)
            || transition.a2a_get_task_key() != Some(&key)
        {
            return Err(ResidentA2aGetTaskReservationReleaseFailure::Transition {
                owner_instance,
                key,
                failure: transition,
            });
        }
        match reservations.retry_refuse_offer(*transition) {
            Ok(()) => Ok(()),
            Err(retry) => Err(ResidentA2aGetTaskReservationReleaseFailure::Transition {
                owner_instance,
                key,
                failure: Box::new(retry),
            }),
        }
    }

    /// Prove that this owner and ledger still hold one unopened reservation.
    pub(in crate::resident) fn require_unopened(
        &self,
        reservation: &ResidentA2aGetTaskReservation,
        reservations: &ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        if !self.owner_instance.is_same(&reservation.owner_instance) {
            return Err(ResidentTaskReservationTransitionFailure::OwnerMismatch);
        }
        reservations.require_a2a_get_task_offer(&reservation.offer, &reservation.key)
    }

    /// Accept capacity, then release one physically retained request to the carrier.
    pub(in crate::resident) fn start_retained<'a, Authorize>(
        &mut self,
        reservation: ResidentA2aGetTaskReservation,
        retained: RetainedA2aGetTask<'a>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        reservations: &mut ResidentAggregateTaskReservations,
        authorize: Authorize,
    ) -> Result<ResidentA2aGetTaskDurableStart<'a>, ResidentA2aGetTaskDurableStartFailure<'a>>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), pale_core::records::InvalidResidentRecordV1>,
    {
        let key = reservation.key.as_ref().clone();
        if !self.owner_instance.is_same(&reservation.owner_instance)
            || self.active.iter().any(|active| active.key == key)
            || self.require_unopened(&reservation, reservations).is_err()
        {
            return Err(ResidentA2aGetTaskDurableStartFailure::Association {
                reservation,
                retained: Box::new(retained),
            });
        }
        let accepted = match reservations.accept_offer(*reservation.offer) {
            Ok(()) => ResidentA2aGetTaskAggregateAccepted {
                owner_instance: self.owner_instance.clone(),
                key: Box::new(key),
                retained: Box::new(retained),
            },
            Err(failure) => {
                return Err(ResidentA2aGetTaskDurableStartFailure::AggregateAcceptance(
                    ResidentA2aGetTaskAggregateAcceptanceFailure {
                        owner_instance: self.owner_instance.clone(),
                        key: reservation.key,
                        retained: Box::new(retained),
                        failure: Box::new(failure),
                    },
                ));
            }
        };
        Ok(self.start_accepted(accepted, exchange, clock, reservations, authorize))
    }

    /// Retry only aggregate acceptance before any carrier call.
    pub(in crate::resident) fn retry_retained_acceptance<'a>(
        &self,
        failure: ResidentA2aGetTaskAggregateAcceptanceFailure<'a>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentA2aGetTaskAggregateAccepted<'a>,
        ResidentA2aGetTaskAggregateAcceptanceFailure<'a>,
    > {
        if !self.owner_instance.is_same(&failure.owner_instance)
            || failure.failure.a2a_get_task_key() != Some(&failure.key)
        {
            return Err(failure);
        }
        match reservations.retry_accept_offer(*failure.failure) {
            Ok(()) => Ok(ResidentA2aGetTaskAggregateAccepted {
                owner_instance: self.owner_instance.clone(),
                key: failure.key,
                retained: failure.retained,
            }),
            Err(retry) => Err(ResidentA2aGetTaskAggregateAcceptanceFailure {
                owner_instance: failure.owner_instance,
                key: failure.key,
                retained: failure.retained,
                failure: Box::new(retry),
            }),
        }
    }

    /// Apply fresh authority to one aggregate-accepted unopened request.
    pub(in crate::resident) fn start_accepted<'a, Authorize>(
        &mut self,
        accepted: ResidentA2aGetTaskAggregateAccepted<'a>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        reservations: &mut ResidentAggregateTaskReservations,
        authorize: Authorize,
    ) -> ResidentA2aGetTaskDurableStart<'a>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), pale_core::records::InvalidResidentRecordV1>,
    {
        if !self.owner_instance.is_same(&accepted.owner_instance) {
            return ResidentA2aGetTaskDurableStart::PreflightRejected(
                ResidentA2aGetTaskPreflightFailure {
                    accepted,
                    reason: A2aGetTaskReadbackError::Association,
                },
            );
        }
        let key = accepted.key.as_ref().clone();
        let request_checkpoint = {
            let (reference, checkpoint) = accepted.retained.request_checkpoint();
            Box::new((reference.clone(), checkpoint.clone()))
        };
        match (*accepted.retained).start_retained(exchange, clock, authorize) {
            Ok(started) => {
                let (in_flight, request_custody, disposition_basis) = started.into_parts();
                self.active.push(ActiveA2aGetTask {
                    key,
                    phase: ActiveA2aGetTaskPhase::AwaitingResponse {
                        in_flight: Box::new(in_flight),
                        request_checkpoint,
                        request_custody: Some(Box::new(request_custody)),
                        disposition_basis: Some(Box::new(disposition_basis)),
                    },
                });
                ResidentA2aGetTaskDurableStart::Active
            }
            Err(DurableA2aGetTaskStartFailure::Preflight { retained, reason }) => {
                ResidentA2aGetTaskDurableStart::PreflightRejected(
                    ResidentA2aGetTaskPreflightFailure {
                        accepted: ResidentA2aGetTaskAggregateAccepted {
                            owner_instance: self.owner_instance.clone(),
                            key: Box::new(key),
                            retained,
                        },
                        reason,
                    },
                )
            }
            Err(DurableA2aGetTaskStartFailure::Started {
                reason,
                custody,
                disposition_basis,
            }) => ResidentA2aGetTaskDurableStart::Terminal(self.close(
                key,
                ResidentA2aGetTaskOutcome::Closed {
                    reason: *reason,
                    request_checkpoint,
                    request_custody: Some(custody),
                    disposition_basis: Some(disposition_basis),
                },
                reservations,
            )),
        }
    }

    /// Stop one live local read without sending A2A `CancelTask`.
    pub(in crate::resident) fn stop_response_read(
        &mut self,
        key: &ResidentA2aTaskWorkKey,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aGetTaskPoll {
        let Some(index) = self.active.iter().position(|active| &active.key == key) else {
            return ResidentA2aGetTaskPoll::NotFound;
        };
        let active = self.active.swap_remove(index);
        let (request_checkpoint, request_custody, disposition_basis) = match active.phase {
            ActiveA2aGetTaskPhase::AwaitingResponse {
                in_flight,
                request_checkpoint,
                request_custody,
                disposition_basis,
            } => {
                (*in_flight).cancel();
                (request_checkpoint, request_custody, disposition_basis)
            }
            ActiveA2aGetTaskPhase::AwaitingEof {
                received,
                request_checkpoint,
                request_custody,
                disposition_basis,
            } => {
                (*received).cancel();
                (request_checkpoint, request_custody, disposition_basis)
            }
        };
        self.close(
            active.key,
            ResidentA2aGetTaskOutcome::ReadStopped {
                request_checkpoint,
                request_custody,
                disposition_basis,
            },
            reservations,
        )
    }

    /// Advance one exact read by at most one phase.
    pub(in crate::resident) fn poll(
        &mut self,
        key: &ResidentA2aTaskWorkKey,
        cx: &mut Context<'_>,
        clock: &dyn WallClockSourcePort,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aGetTaskPoll {
        let Some(index) = self.active.iter().position(|active| &active.key == key) else {
            return ResidentA2aGetTaskPoll::NotFound;
        };
        let active = self.active.swap_remove(index);
        match active.phase {
            ActiveA2aGetTaskPhase::AwaitingResponse {
                in_flight,
                request_checkpoint,
                request_custody,
                disposition_basis,
            } => match (*in_flight).poll_response(cx) {
                A2aGetTaskReadbackPoll::Pending(in_flight) => {
                    self.active.push(ActiveA2aGetTask {
                        key: active.key,
                        phase: ActiveA2aGetTaskPhase::AwaitingResponse {
                            in_flight: Box::new(in_flight),
                            request_checkpoint,
                            request_custody,
                            disposition_basis,
                        },
                    });
                    ResidentA2aGetTaskPoll::PendingResponse
                }
                A2aGetTaskReadbackPoll::Ready(Ok(received)) => {
                    self.active.push(ActiveA2aGetTask {
                        key: active.key,
                        phase: ActiveA2aGetTaskPhase::AwaitingEof {
                            received: Box::new(received),
                            request_checkpoint,
                            request_custody,
                            disposition_basis,
                        },
                    });
                    ResidentA2aGetTaskPoll::ResponseReceived
                }
                A2aGetTaskReadbackPoll::Ready(Err(reason)) => self.close(
                    active.key,
                    ResidentA2aGetTaskOutcome::Closed {
                        reason,
                        request_checkpoint,
                        request_custody,
                        disposition_basis,
                    },
                    reservations,
                ),
            },
            ActiveA2aGetTaskPhase::AwaitingEof {
                received,
                request_checkpoint,
                request_custody,
                disposition_basis,
            } => match (*received).poll_eof(cx, clock) {
                A2aGetTaskReadbackPoll::Pending(received) => {
                    self.active.push(ActiveA2aGetTask {
                        key: active.key,
                        phase: ActiveA2aGetTaskPhase::AwaitingEof {
                            received: Box::new(received),
                            request_checkpoint,
                            request_custody,
                            disposition_basis,
                        },
                    });
                    ResidentA2aGetTaskPoll::PendingEof
                }
                A2aGetTaskReadbackPoll::Ready(Ok(completed)) => self.close(
                    active.key,
                    ResidentA2aGetTaskOutcome::Completed {
                        completed: Box::new(completed),
                        request_checkpoint,
                        request_custody,
                    },
                    reservations,
                ),
                A2aGetTaskReadbackPoll::Ready(Err(reason)) => self.close(
                    active.key,
                    ResidentA2aGetTaskOutcome::Closed {
                        reason,
                        request_checkpoint,
                        request_custody,
                        disposition_basis,
                    },
                    reservations,
                ),
            },
        }
    }

    fn close(
        &self,
        key: ResidentA2aTaskWorkKey,
        outcome: ResidentA2aGetTaskOutcome,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aGetTaskPoll {
        match reservations
            .mark_a2a_task_completion_pending(&key, &ResidentTaskReservationRoute::A2aGetTask)
        {
            Ok(settlement) => match outcome {
                ResidentA2aGetTaskOutcome::Completed {
                    completed,
                    request_checkpoint,
                    request_custody,
                } => ResidentA2aGetTaskPoll::AwaitingCustody(Box::new(
                    ResidentA2aGetTaskAwaitingCustody {
                        owner_instance: self.owner_instance.clone(),
                        key,
                        completed,
                        request_checkpoint,
                        request_custody,
                        settlement: Box::new(settlement),
                    },
                )),
                ResidentA2aGetTaskOutcome::Closed {
                    reason,
                    request_checkpoint,
                    request_custody,
                    disposition_basis,
                } => {
                    ResidentA2aGetTaskPoll::Closed(Box::new(ResidentA2aGetTaskClosedDisposition {
                        owner_instance: self.owner_instance.clone(),
                        key,
                        reason,
                        request_checkpoint,
                        request_custody,
                        disposition_basis,
                        settlement: Box::new(settlement),
                    }))
                }
                ResidentA2aGetTaskOutcome::ReadStopped {
                    request_checkpoint,
                    request_custody,
                    disposition_basis,
                } => ResidentA2aGetTaskPoll::ReadStopped(Box::new(
                    ResidentA2aGetTaskReadStoppedDisposition {
                        owner_instance: self.owner_instance.clone(),
                        key,
                        request_checkpoint,
                        request_custody,
                        disposition_basis,
                        settlement: Box::new(settlement),
                    },
                )),
            },
            Err(reason) => ResidentA2aGetTaskPoll::AggregateTransitionRejected(Box::new(
                ResidentA2aGetTaskAggregateTransitionRejection {
                    owner_instance: self.owner_instance.clone(),
                    key,
                    outcome,
                    reason,
                },
            )),
        }
    }

    /// Retry only the accepted-to-completion aggregate transition.
    pub(in crate::resident) fn retry_aggregate_transition(
        &self,
        rejected: ResidentA2aGetTaskAggregateTransitionRejection,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aGetTaskPoll {
        if !self.owner_instance.is_same(&rejected.owner_instance) {
            return ResidentA2aGetTaskPoll::AggregateTransitionRejected(Box::new(
                ResidentA2aGetTaskAggregateTransitionRejection {
                    reason: ResidentTaskReservationTransitionFailure::OwnerMismatch,
                    ..rejected
                },
            ));
        }
        self.close(rejected.key, rejected.outcome, reservations)
    }

    /// Release capacity after an exact closed or durably retained disposition.
    pub(in crate::resident) fn finish_closed(
        &self,
        closed: ResidentA2aGetTaskClosedDisposition,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<A2aGetTaskReadbackError, ResidentA2aGetTaskFinishFailure> {
        if !self.owner_instance.is_same(&closed.owner_instance) {
            return Err(ResidentA2aGetTaskFinishFailure {
                closed: Box::new(closed),
                reason: ResidentTaskReservationTransitionFailure::OwnerMismatch,
            });
        }
        if let Err(reason) =
            reservations.require_a2a_get_task_completion(&closed.settlement, &closed.key)
        {
            return Err(ResidentA2aGetTaskFinishFailure {
                closed: Box::new(closed),
                reason,
            });
        }
        let ResidentA2aGetTaskClosedDisposition {
            owner_instance: _,
            key,
            reason,
            request_checkpoint,
            request_custody,
            disposition_basis,
            settlement,
        } = closed;
        let retry_key = key.clone();
        match reservations.release_completion(*settlement) {
            Ok(()) => Ok(reason),
            Err(failure) => {
                let release_reason = failure.reason;
                Err(ResidentA2aGetTaskFinishFailure {
                    closed: Box::new(ResidentA2aGetTaskClosedDisposition {
                        owner_instance: self.owner_instance.clone(),
                        key: retry_key,
                        reason,
                        request_checkpoint,
                        request_custody,
                        disposition_basis,
                        settlement: failure.settlement,
                    }),
                    reason: release_reason,
                })
            }
        }
    }

    /// Release capacity after one explicit local response-read stop.
    pub(in crate::resident) fn finish_stopped_response_read(
        &self,
        stopped: ResidentA2aGetTaskReadStoppedDisposition,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentA2aGetTaskReadStopFinishFailure> {
        if !self.owner_instance.is_same(&stopped.owner_instance) {
            return Err(ResidentA2aGetTaskReadStopFinishFailure {
                stopped: Box::new(stopped),
                reason: ResidentTaskReservationTransitionFailure::OwnerMismatch,
            });
        }
        if let Err(reason) =
            reservations.require_a2a_get_task_completion(&stopped.settlement, &stopped.key)
        {
            return Err(ResidentA2aGetTaskReadStopFinishFailure {
                stopped: Box::new(stopped),
                reason,
            });
        }
        let ResidentA2aGetTaskReadStoppedDisposition {
            owner_instance: _,
            key,
            request_checkpoint,
            request_custody,
            disposition_basis,
            settlement,
        } = stopped;
        match reservations.release_completion(*settlement) {
            Ok(()) => Ok(()),
            Err(failure) => Err(ResidentA2aGetTaskReadStopFinishFailure {
                stopped: Box::new(ResidentA2aGetTaskReadStoppedDisposition {
                    owner_instance: self.owner_instance.clone(),
                    key,
                    request_checkpoint,
                    request_custody,
                    disposition_basis,
                    settlement: failure.settlement,
                }),
                reason: failure.reason,
            }),
        }
    }
}

/// A fresh start stopped without permitting replay of its identities.
pub(in crate::resident) enum ResidentA2aGetTaskStartFailure {
    Association,
    Reservation(super::task_reservation::ResidentA2aTaskReservationOfferFailure),
    Closed(A2aGetTaskReadbackError),
    Rollback {
        reason: Box<A2aGetTaskReadbackError>,
        reservation: Box<ResidentTaskReservationOfferTransitionFailure>,
    },
    Acceptance {
        key: Box<ResidentA2aTaskWorkKey>,
        in_flight: Box<InFlightA2aGetTaskReadback>,
        request_checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        reservation: Box<ResidentTaskReservationOfferTransitionFailure>,
    },
}

impl ResidentA2aGetTaskStartFailure {
    /// Return the binding key retained by a continuable start failure.
    #[must_use]
    pub(in crate::resident) fn key(&self) -> Option<&ResidentA2aTaskWorkKey> {
        match self {
            Self::Acceptance { key, .. } => Some(key),
            Self::Rollback { reservation, .. } => reservation.a2a_get_task_key(),
            Self::Association | Self::Reservation(_) | Self::Closed(_) => None,
        }
    }
}

/// Terminal local phase before an explicit disposition or history custody.
pub(in crate::resident) enum ResidentA2aGetTaskOutcome {
    Completed {
        completed: Box<CompletedA2aGetTaskReadback>,
        request_checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        request_custody: Option<Box<super::CompletedOperationObservationCustody>>,
    },
    Closed {
        reason: A2aGetTaskReadbackError,
        request_checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        request_custody: Option<Box<super::CompletedOperationObservationCustody>>,
        disposition_basis: Option<Box<A2aGetTaskLocalDispositionBasis>>,
    },
    ReadStopped {
        request_checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        request_custody: Option<Box<super::CompletedOperationObservationCustody>>,
        disposition_basis: Option<Box<A2aGetTaskLocalDispositionBasis>>,
    },
}

/// Completed response that can release capacity only through history custody.
#[must_use = "retain until exact remote-history custody"]
pub(in crate::resident) struct ResidentA2aGetTaskAwaitingCustody {
    owner_instance: ResidentOperationTaskOwnerInstance,
    pub(in crate::resident) key: ResidentA2aTaskWorkKey,
    pub(in crate::resident) completed: Box<CompletedA2aGetTaskReadback>,
    pub(in crate::resident) request_checkpoint:
        Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    pub(in crate::resident) request_custody:
        Option<Box<super::CompletedOperationObservationCustody>>,
    pub(in crate::resident) settlement: Box<ResidentTaskReservationSettlement>,
}

/// Closed read whose explicit local disposition still owns aggregate capacity.
#[must_use = "finish the exact closed disposition"]
pub(in crate::resident) struct ResidentA2aGetTaskClosedDisposition {
    owner_instance: ResidentOperationTaskOwnerInstance,
    pub(in crate::resident) key: ResidentA2aTaskWorkKey,
    pub(in crate::resident) reason: A2aGetTaskReadbackError,
    pub(in crate::resident) request_checkpoint:
        Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    pub(in crate::resident) request_custody:
        Option<Box<super::CompletedOperationObservationCustody>>,
    pub(in crate::resident) disposition_basis: Option<Box<A2aGetTaskLocalDispositionBasis>>,
    settlement: Box<ResidentTaskReservationSettlement>,
}

/// Explicit local response-read stop that still owns aggregate capacity.
#[must_use = "finish the exact local response-read stop"]
pub(in crate::resident) struct ResidentA2aGetTaskReadStoppedDisposition {
    owner_instance: ResidentOperationTaskOwnerInstance,
    pub(in crate::resident) key: ResidentA2aTaskWorkKey,
    pub(in crate::resident) request_checkpoint:
        Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    pub(in crate::resident) request_custody:
        Option<Box<super::CompletedOperationObservationCustody>>,
    pub(in crate::resident) disposition_basis: Option<Box<A2aGetTaskLocalDispositionBasis>>,
    settlement: Box<ResidentTaskReservationSettlement>,
}

/// Terminal carrier work whose aggregate-capacity transition needs a retry.
pub(in crate::resident) struct ResidentA2aGetTaskAggregateTransitionRejection {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: ResidentA2aTaskWorkKey,
    outcome: ResidentA2aGetTaskOutcome,
    reason: ResidentTaskReservationTransitionFailure,
}

impl ResidentA2aGetTaskAggregateTransitionRejection {
    /// Return the binding key whose aggregate transition was rejected.
    #[must_use]
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.key
    }
}

/// Exact completed phase rejected while returning capacity.
pub(in crate::resident) struct ResidentA2aGetTaskFinishFailure {
    pub(in crate::resident) closed: Box<ResidentA2aGetTaskClosedDisposition>,
    pub(in crate::resident) reason: ResidentTaskReservationTransitionFailure,
}

/// Exact local read-stop phase rejected while returning capacity.
pub(in crate::resident) struct ResidentA2aGetTaskReadStopFinishFailure {
    pub(in crate::resident) stopped: Box<ResidentA2aGetTaskReadStoppedDisposition>,
    pub(in crate::resident) reason: ResidentTaskReservationTransitionFailure,
}

/// One wake-driven owner transition.
pub(in crate::resident) enum ResidentA2aGetTaskPoll {
    PendingResponse,
    ResponseReceived,
    PendingEof,
    AwaitingCustody(Box<ResidentA2aGetTaskAwaitingCustody>),
    Closed(Box<ResidentA2aGetTaskClosedDisposition>),
    ReadStopped(Box<ResidentA2aGetTaskReadStoppedDisposition>),
    AggregateTransitionRejected(Box<ResidentA2aGetTaskAggregateTransitionRejection>),
    NotFound,
}

#[cfg(test)]
#[path = "../../tests/internal/resident/a2a_get_task_owner.rs"]
mod tests;
