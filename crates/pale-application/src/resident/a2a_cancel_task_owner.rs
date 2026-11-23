//! Process-local ownership for one-shot A2A `CancelTask` calls.
//!
//! A local stop closes only response reading. Carrier acceptance permanently
//! removes replay authority. Remote Task state remains durable history and
//! never becomes a Pale terminal result through this owner.

use std::{num::NonZeroUsize, task::Context};

use pale_core::records::{OperationObservationDigest, ResidentRecordRefV1};
use pale_ports::{A2aCancelTaskEvidencePort, WallClockSourcePort};

use super::a2a_task_custody::{
    A2aCancelTaskDispositionPhase, A2aCancelTaskError, A2aCancelTaskNotStarted,
    A2aCancelTaskOutcomeUnknown, A2aCancelTaskPoll, A2aCancelTaskStartFailure,
    CompletedA2aCancelTaskDispositionCustody, CompletedA2aCancelTaskReadback,
    CompletedA2aCancelTaskReadbackCustody, InFlightA2aCancelTask, ReceivedA2aCancelTaskResponse,
    RecoveredA2aTaskHistory, RetainedA2aCancelTask,
};
use super::task_owner::{ResidentOperationTaskOwnerBuildError, ResidentOperationTaskOwnerInstance};
use super::task_reservation::{
    ResidentA2aTaskReservationOfferFailure, ResidentA2aTaskWorkKey,
    ResidentAggregateTaskReservations, ResidentTaskReservationOffer,
    ResidentTaskReservationOfferTransitionFailure, ResidentTaskReservationRoute,
    ResidentTaskReservationSettlement, ResidentTaskReservationTransitionFailure,
};
use super::{start_a2a_cancel_task, A2aCancelTaskAuthorizationContext};

enum ActiveA2aCancelTaskPhase {
    AwaitingResponse(Box<InFlightA2aCancelTask>),
    AwaitingEof(Box<ReceivedA2aCancelTaskResponse>),
}

struct ActiveA2aCancelTask {
    key: ResidentA2aTaskWorkKey,
    phase: ActiveA2aCancelTaskPhase,
}

/// Derive one physical-store-qualified key from admitted Task history.
pub(in crate::resident) fn a2a_cancel_task_key(
    history: &RecoveredA2aTaskHistory,
) -> ResidentA2aTaskWorkKey {
    let binding = history.binding();
    ResidentA2aTaskWorkKey {
        store: history.local_observation().store().clone(),
        operation_handle_ref: binding.1.operation_handle_ref().clone(),
        operation_handle_sha256: binding.1.operation_handle_sha256().clone(),
        task_binding_id: binding.1.task_binding_id().clone(),
        task_binding_ref: binding.0.clone(),
        task_binding_sha256: binding.1.canonical_a2a_task_binding_sha256().clone(),
    }
}

/// Offered aggregate capacity reserved before cancellation-request preparation.
#[must_use = "retain through request custody or release before carrier use"]
pub(in crate::resident) struct ResidentA2aCancelTaskReservation {
    owner_instance: ResidentOperationTaskOwnerInstance,
    pub(in crate::resident) key: Box<ResidentA2aTaskWorkKey>,
    offer: Box<ResidentTaskReservationOffer>,
}

/// Fresh inputs needed to open one retained cancellation request.
pub(in crate::resident) struct ResidentA2aCancelTaskStartContext<'a, Authorize> {
    pub(in crate::resident) history: &'a RecoveredA2aTaskHistory,
    pub(in crate::resident) carrier: &'a dyn pale_ports::A2aExchangePort,
    pub(in crate::resident) clock: &'a dyn WallClockSourcePort,
    pub(in crate::resident) authorize: Authorize,
}

impl<'a, Authorize> ResidentA2aCancelTaskStartContext<'a, Authorize> {
    /// Keep fresh history, carrier, time, and authority together for one start.
    pub(in crate::resident) const fn new(
        history: &'a RecoveredA2aTaskHistory,
        carrier: &'a dyn pale_ports::A2aExchangePort,
        clock: &'a dyn WallClockSourcePort,
        authorize: Authorize,
    ) -> Self {
        Self {
            history,
            carrier,
            clock,
            authorize,
        }
    }
}

/// Exact fresh start rejected before carrier use.
#[must_use = "retry only the returned unopened request or release its reservation"]
pub(in crate::resident) struct ResidentA2aCancelTaskPreflightFailure<'a> {
    pub(in crate::resident) aggregate_accepted: ResidentA2aCancelTaskAggregateAccepted<'a>,
    pub(in crate::resident) reason: A2aCancelTaskError,
}

/// Retained unopened request whose aggregate acceptance still needs a retry.
#[must_use = "retry aggregate acceptance before any carrier call"]
pub(in crate::resident) struct ResidentA2aCancelTaskAggregateAcceptanceFailure<'a> {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: Box<ResidentA2aTaskWorkKey>,
    retained: Box<RetainedA2aCancelTask<'a>>,
    failure: Box<ResidentTaskReservationOfferTransitionFailure>,
}

/// Aggregate-accepted request that remains unopened by the carrier.
#[must_use = "perform fresh preflight or preserve the accepted unopened request"]
pub(in crate::resident) struct ResidentA2aCancelTaskAggregateAccepted<'a> {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: Box<ResidentA2aTaskWorkKey>,
    retained: Box<RetainedA2aCancelTask<'a>>,
}

/// Terminal carrier evidence waiting for durable local or remote history.
pub(in crate::resident) enum ResidentA2aCancelTaskOutcome {
    ReadbackCompleted(Box<CompletedA2aCancelTaskReadback>),
    NotStarted(Box<A2aCancelTaskNotStarted>),
    OutcomeUnknown(Box<A2aCancelTaskOutcomeUnknown>),
}

/// Carrier evidence and aggregate capacity held until durable handling.
#[must_use = "retain exact history or disposition before releasing capacity"]
pub(in crate::resident) struct ResidentA2aCancelTaskAwaitingCustody {
    owner_instance: ResidentOperationTaskOwnerInstance,
    pub(in crate::resident) key: Box<ResidentA2aTaskWorkKey>,
    pub(in crate::resident) outcome: ResidentA2aCancelTaskOutcome,
    pub(in crate::resident) settlement: Box<ResidentTaskReservationSettlement>,
}

/// Qualified complete read-back waiting for predecessor-linked remote history.
#[must_use = "retain exact remote history before releasing aggregate capacity"]
pub(in crate::resident) struct ResidentA2aCancelTaskReadbackCustodyToken {
    owner_instance: ResidentOperationTaskOwnerInstance,
    pub(in crate::resident) key: Box<ResidentA2aTaskWorkKey>,
    pub(in crate::resident) settlement: Box<ResidentTaskReservationSettlement>,
}

/// Read-back custody did not match its exact process-local terminal phase.
#[must_use = "preserve both the terminal phase and durable custody"]
pub(in crate::resident) struct ResidentA2aCancelTaskReadbackAssociationFailure {
    pub(in crate::resident) token: Box<ResidentA2aCancelTaskReadbackCustodyToken>,
    pub(in crate::resident) custody: Box<CompletedA2aCancelTaskReadbackCustody>,
}

/// Incomplete disposition waiting for exact local successor custody.
#[must_use = "retain the exact local disposition before releasing aggregate capacity"]
pub(in crate::resident) struct ResidentA2aCancelTaskDispositionCustodyToken {
    owner_instance: ResidentOperationTaskOwnerInstance,
    pub(in crate::resident) key: Box<ResidentA2aTaskWorkKey>,
    pub(in crate::resident) settlement: Box<ResidentTaskReservationSettlement>,
    pub(in crate::resident) cancellation: (ResidentRecordRefV1, OperationObservationDigest),
}

/// Disposition custody did not match its exact process-local terminal phase.
#[must_use = "preserve both the terminal token and durable disposition"]
pub(in crate::resident) struct ResidentA2aCancelTaskDispositionAssociationFailure {
    pub(in crate::resident) token: Box<ResidentA2aCancelTaskDispositionCustodyToken>,
    pub(in crate::resident) custody: Box<CompletedA2aCancelTaskDispositionCustody>,
}

/// Terminal carrier evidence whose aggregate transition must be retried.
#[must_use = "retry the exact accepted-to-completion transition"]
pub(in crate::resident) struct ResidentA2aCancelTaskAggregateTransitionRejection {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: Box<ResidentA2aTaskWorkKey>,
    outcome: ResidentA2aCancelTaskOutcome,
    reason: ResidentTaskReservationTransitionFailure,
}

/// Physical outcome retained before aggregate capacity can be released.
pub(in crate::resident) enum ResidentA2aCancelTaskDurableCustody {
    ReadbackCustody(Box<CompletedA2aCancelTaskReadbackCustody>),
    Disposition(Box<CompletedA2aCancelTaskDispositionCustody>),
}

/// Durable typed handling that still owns aggregate capacity.
#[must_use = "release the exact aggregate settlement"]
pub(in crate::resident) struct ResidentRetainedA2aCancelTaskOutcome {
    owner_instance: ResidentOperationTaskOwnerInstance,
    pub(in crate::resident) key: Box<ResidentA2aTaskWorkKey>,
    pub(in crate::resident) settlement: Box<ResidentTaskReservationSettlement>,
    pub(in crate::resident) custody: ResidentA2aCancelTaskDurableCustody,
}

/// Aggregate release failure after durable handling completed.
#[must_use = "retry only the exact aggregate release"]
pub(in crate::resident) struct ResidentA2aCancelTaskReleaseFailure {
    pub(in crate::resident) retained: Box<ResidentRetainedA2aCancelTaskOutcome>,
    pub(in crate::resident) reason: ResidentTaskReservationTransitionFailure,
}

impl ResidentA2aCancelTaskReservationReleaseFailure {
    /// Borrow the binding-qualified key whose unopened offer must be released.
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        match self {
            Self::Association(reservation) => &reservation.key,
            Self::Transition { key, .. } => key,
        }
    }
}

impl ResidentA2aCancelTaskAggregateAcceptanceFailure<'_> {
    /// Borrow the key retained with failed process-local aggregate acceptance.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.key
    }
}

impl ResidentA2aCancelTaskAggregateAccepted<'_> {
    /// Borrow the key of this aggregate-accepted unopened request.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.key
    }
}

impl ResidentA2aCancelTaskPreflightFailure<'_> {
    /// Borrow the key retained with the unopened request.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        self.aggregate_accepted.key()
    }
}

impl ResidentA2aCancelTaskAwaitingCustody {
    /// Borrow the key whose carrier evidence awaits durable handling.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.key
    }
}

impl ResidentA2aCancelTaskReadbackCustodyToken {
    /// Borrow the key whose complete read-back awaits remote-history custody.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.key
    }
}

impl ResidentA2aCancelTaskDispositionCustodyToken {
    /// Borrow the key whose incomplete outcome awaits local disposition custody.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.key
    }

    /// Borrow the exact local cancellation observation this disposition closes.
    pub(in crate::resident) const fn cancellation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationDigest) {
        (&self.cancellation.0, &self.cancellation.1)
    }
}

impl ResidentA2aCancelTaskAggregateTransitionRejection {
    /// Borrow the key whose accepted-to-completion transition was rejected.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.key
    }
}

impl ResidentRetainedA2aCancelTaskOutcome {
    /// Borrow the key retained with exact durable terminal custody.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.key
    }
}

impl ResidentA2aCancelTaskReleaseFailure {
    /// Borrow the key whose aggregate release remains unresolved.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        self.retained.key()
    }
}

impl ResidentA2aCancelTaskStartTransitionFailure<'_> {
    /// Borrow the key retained by the failed local transition.
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        match self {
            Self::Association { reservation, .. } => &reservation.key,
            Self::AggregateAcceptance(failure) => failure.key(),
        }
    }
}

/// Result of a fresh or retried unopened start.
pub(in crate::resident) enum ResidentA2aCancelTaskStart<'a> {
    Active,
    PreflightRejected(ResidentA2aCancelTaskPreflightFailure<'a>),
    AwaitingCustody(ResidentA2aCancelTaskAwaitingCustody),
    AggregateTransitionRejected(ResidentA2aCancelTaskAggregateTransitionRejection),
}

/// One wake-driven transition for an active cancellation response read.
pub(in crate::resident) enum ResidentA2aCancelTaskOwnerPoll {
    PendingResponse,
    ResponseReceived,
    PendingEof,
    AwaitingCustody(ResidentA2aCancelTaskAwaitingCustody),
    AggregateTransitionRejected(ResidentA2aCancelTaskAggregateTransitionRejection),
    NotFound,
}

/// Stable owner of started `CancelTask` response reads for one resident lifetime.
pub(in crate::resident) struct ResidentA2aCancelTaskOwner {
    owner_instance: ResidentOperationTaskOwnerInstance,
    active: Vec<ActiveA2aCancelTask>,
}

impl ResidentA2aCancelTaskOwner {
    /// Reserve all local phase slots before any carrier can start.
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

    /// Reserve binding-wide capacity before clock, signing, storage, or carrier work.
    pub(in crate::resident) fn reserve(
        &self,
        key: ResidentA2aTaskWorkKey,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentA2aCancelTaskReservation, ResidentA2aTaskReservationOfferFailure> {
        let offer = reservations
            .begin_a2a_task_offer(key.clone(), ResidentTaskReservationRoute::A2aCancelTask)?;
        Ok(ResidentA2aCancelTaskReservation {
            owner_instance: self.owner_instance.clone(),
            key: Box::new(key),
            offer: Box::new(offer),
        })
    }

    /// Return offered capacity after request preparation proved no carrier use.
    pub(in crate::resident) fn release_unopened(
        &self,
        reservation: ResidentA2aCancelTaskReservation,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentA2aCancelTaskReservationReleaseFailure> {
        if !self.owner_instance.is_same(&reservation.owner_instance) {
            return Err(ResidentA2aCancelTaskReservationReleaseFailure::Association(
                reservation,
            ));
        }
        match reservations.refuse_offer(*reservation.offer) {
            Ok(()) => Ok(()),
            Err(failure) => Err(ResidentA2aCancelTaskReservationReleaseFailure::Transition {
                owner_instance: self.owner_instance.clone(),
                key: reservation.key,
                failure: Box::new(failure),
            }),
        }
    }

    /// Retry only exact aggregate-offer removal after no carrier was used.
    pub(in crate::resident) fn retry_unopened_release(
        &self,
        failure: ResidentA2aCancelTaskReservationReleaseFailure,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentA2aCancelTaskReservationReleaseFailure> {
        let ResidentA2aCancelTaskReservationReleaseFailure::Transition {
            owner_instance,
            key,
            failure: transition,
        } = failure
        else {
            return Err(failure);
        };
        if !self.owner_instance.is_same(&owner_instance)
            || transition.a2a_cancel_task_key() != Some(&key)
        {
            return Err(ResidentA2aCancelTaskReservationReleaseFailure::Transition {
                owner_instance,
                key,
                failure: transition,
            });
        }
        match reservations.retry_refuse_offer(*transition) {
            Ok(()) => Ok(()),
            Err(retry) => Err(ResidentA2aCancelTaskReservationReleaseFailure::Transition {
                owner_instance,
                key,
                failure: Box::new(retry),
            }),
        }
    }

    /// Prove that this owner and ledger still hold one unopened reservation.
    pub(in crate::resident) fn require_unopened(
        &self,
        reservation: &ResidentA2aCancelTaskReservation,
        reservations: &ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        if !self.owner_instance.is_same(&reservation.owner_instance) {
            return Err(ResidentTaskReservationTransitionFailure::OwnerMismatch);
        }
        reservations.require_a2a_cancel_task_offer(&reservation.offer, &reservation.key)
    }

    /// Start the exact retained request once after fresh lower authorization.
    pub(in crate::resident) fn start<'a, Authorize>(
        &mut self,
        reservation: ResidentA2aCancelTaskReservation,
        retained: RetainedA2aCancelTask<'a>,
        start: ResidentA2aCancelTaskStartContext<'_, Authorize>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentA2aCancelTaskStart<'a>, ResidentA2aCancelTaskStartTransitionFailure<'a>>
    where
        Authorize: FnOnce(
            A2aCancelTaskAuthorizationContext<'_>,
        ) -> Result<(), pale_core::records::InvalidResidentRecordV1>,
    {
        let key = a2a_cancel_task_key(start.history);
        if !self.owner_instance.is_same(&reservation.owner_instance)
            || *reservation.key != key
            || self.active.iter().any(|active| active.key == key)
        {
            return Err(ResidentA2aCancelTaskStartTransitionFailure::Association {
                reservation,
                retained: Box::new(retained),
            });
        }
        let accepted = match reservations.accept_offer(*reservation.offer) {
            Ok(()) => ResidentA2aCancelTaskAggregateAccepted {
                owner_instance: self.owner_instance.clone(),
                key: Box::new(key),
                retained: Box::new(retained),
            },
            Err(failure) => {
                return Err(
                    ResidentA2aCancelTaskStartTransitionFailure::AggregateAcceptance(
                        ResidentA2aCancelTaskAggregateAcceptanceFailure {
                            owner_instance: self.owner_instance.clone(),
                            key: reservation.key,
                            retained: Box::new(retained),
                            failure: Box::new(failure),
                        },
                    ),
                );
            }
        };
        Ok(self.start_accepted(accepted, start, reservations))
    }

    /// Start one already-accepted unopened request after fresh lower preflight.
    pub(in crate::resident) fn start_accepted<'a, Authorize>(
        &mut self,
        aggregate_accepted: ResidentA2aCancelTaskAggregateAccepted<'a>,
        start: ResidentA2aCancelTaskStartContext<'_, Authorize>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aCancelTaskStart<'a>
    where
        Authorize: FnOnce(
            A2aCancelTaskAuthorizationContext<'_>,
        ) -> Result<(), pale_core::records::InvalidResidentRecordV1>,
    {
        let key = a2a_cancel_task_key(start.history);
        if !self
            .owner_instance
            .is_same(&aggregate_accepted.owner_instance)
            || *aggregate_accepted.key != key
        {
            return ResidentA2aCancelTaskStart::PreflightRejected(
                ResidentA2aCancelTaskPreflightFailure {
                    aggregate_accepted,
                    reason: A2aCancelTaskError::Association,
                },
            );
        }
        match start_a2a_cancel_task(
            *aggregate_accepted.retained,
            start.history,
            start.carrier,
            start.clock,
            start.authorize,
        ) {
            Ok(in_flight) => {
                self.active.push(ActiveA2aCancelTask {
                    key,
                    phase: ActiveA2aCancelTaskPhase::AwaitingResponse(Box::new(in_flight)),
                });
                ResidentA2aCancelTaskStart::Active
            }
            Err(A2aCancelTaskStartFailure::Preflight { retained, reason }) => {
                ResidentA2aCancelTaskStart::PreflightRejected(
                    ResidentA2aCancelTaskPreflightFailure {
                        aggregate_accepted: ResidentA2aCancelTaskAggregateAccepted {
                            owner_instance: self.owner_instance.clone(),
                            key: Box::new(key),
                            retained,
                        },
                        reason,
                    },
                )
            }
            Err(A2aCancelTaskStartFailure::NotStarted(phase)) => Self::start_from_poll(self.close(
                key,
                ResidentA2aCancelTaskOutcome::NotStarted(phase),
                reservations,
            )),
            Err(A2aCancelTaskStartFailure::OutcomeUnknown(phase)) => {
                Self::start_from_poll(self.close(
                    key,
                    ResidentA2aCancelTaskOutcome::OutcomeUnknown(phase),
                    reservations,
                ))
            }
        }
    }

    /// Retry only aggregate acceptance before any carrier call.
    pub(in crate::resident) fn retry_acceptance<'a>(
        &self,
        failure: ResidentA2aCancelTaskAggregateAcceptanceFailure<'a>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentA2aCancelTaskAggregateAccepted<'a>,
        ResidentA2aCancelTaskAggregateAcceptanceFailure<'a>,
    > {
        if !self.owner_instance.is_same(&failure.owner_instance)
            || failure.failure.a2a_cancel_task_key() != Some(&failure.key)
        {
            return Err(failure);
        }
        match reservations.retry_accept_offer(*failure.failure) {
            Ok(()) => Ok(ResidentA2aCancelTaskAggregateAccepted {
                owner_instance: self.owner_instance.clone(),
                key: failure.key,
                retained: failure.retained,
            }),
            Err(retry) => Err(ResidentA2aCancelTaskAggregateAcceptanceFailure {
                owner_instance: failure.owner_instance,
                key: failure.key,
                retained: failure.retained,
                failure: Box::new(retry),
            }),
        }
    }

    /// Advance one active response read by at most one lower phase.
    pub(in crate::resident) fn poll(
        &mut self,
        key: &ResidentA2aTaskWorkKey,
        cx: &mut Context<'_>,
        protocol: &dyn A2aCancelTaskEvidencePort,
        clock: &dyn WallClockSourcePort,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aCancelTaskOwnerPoll {
        let Some(index) = self.active.iter().position(|active| &active.key == key) else {
            return ResidentA2aCancelTaskOwnerPoll::NotFound;
        };
        let active = self.active.swap_remove(index);
        match active.phase {
            ActiveA2aCancelTaskPhase::AwaitingResponse(in_flight) => {
                match (*in_flight).poll_response(cx, protocol) {
                    A2aCancelTaskPoll::Pending(in_flight) => {
                        self.active.push(ActiveA2aCancelTask {
                            key: active.key,
                            phase: ActiveA2aCancelTaskPhase::AwaitingResponse(Box::new(in_flight)),
                        });
                        ResidentA2aCancelTaskOwnerPoll::PendingResponse
                    }
                    A2aCancelTaskPoll::Ready(Ok(received)) => {
                        self.active.push(ActiveA2aCancelTask {
                            key: active.key,
                            phase: ActiveA2aCancelTaskPhase::AwaitingEof(Box::new(received)),
                        });
                        ResidentA2aCancelTaskOwnerPoll::ResponseReceived
                    }
                    A2aCancelTaskPoll::Ready(Err(unknown)) => self.close(
                        active.key,
                        ResidentA2aCancelTaskOutcome::OutcomeUnknown(unknown),
                        reservations,
                    ),
                }
            }
            ActiveA2aCancelTaskPhase::AwaitingEof(received) => {
                match (*received).poll_eof(cx, clock) {
                    A2aCancelTaskPoll::Pending(received) => {
                        self.active.push(ActiveA2aCancelTask {
                            key: active.key,
                            phase: ActiveA2aCancelTaskPhase::AwaitingEof(Box::new(received)),
                        });
                        ResidentA2aCancelTaskOwnerPoll::PendingEof
                    }
                    A2aCancelTaskPoll::Ready(Ok(completed)) => self.close(
                        active.key,
                        ResidentA2aCancelTaskOutcome::ReadbackCompleted(Box::new(completed)),
                        reservations,
                    ),
                    A2aCancelTaskPoll::Ready(Err(unknown)) => self.close(
                        active.key,
                        ResidentA2aCancelTaskOutcome::OutcomeUnknown(unknown),
                        reservations,
                    ),
                }
            }
        }
    }

    /// Stop only the local response read and retain remote-effect uncertainty.
    pub(in crate::resident) fn stop_response_read(
        &mut self,
        key: &ResidentA2aTaskWorkKey,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aCancelTaskOwnerPoll {
        let Some(index) = self.active.iter().position(|active| &active.key == key) else {
            return ResidentA2aCancelTaskOwnerPoll::NotFound;
        };
        let active = self.active.swap_remove(index);
        let unknown = match active.phase {
            ActiveA2aCancelTaskPhase::AwaitingResponse(in_flight) => (*in_flight).cancel(),
            ActiveA2aCancelTaskPhase::AwaitingEof(received) => (*received).cancel(),
        };
        self.close(
            active.key,
            ResidentA2aCancelTaskOutcome::OutcomeUnknown(Box::new(unknown)),
            reservations,
        )
    }

    /// Retry only a rejected accepted-to-completion transition.
    pub(in crate::resident) fn retry_aggregate_transition(
        &self,
        rejected: ResidentA2aCancelTaskAggregateTransitionRejection,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aCancelTaskOwnerPoll {
        if !self.owner_instance.is_same(&rejected.owner_instance) {
            return ResidentA2aCancelTaskOwnerPoll::AggregateTransitionRejected(
                ResidentA2aCancelTaskAggregateTransitionRejection {
                    reason: ResidentTaskReservationTransitionFailure::OwnerMismatch,
                    ..rejected
                },
            );
        }
        self.close(*rejected.key, rejected.outcome, reservations)
    }

    /// Select a complete qualified read-back without treating remote state as a result.
    pub(in crate::resident) fn select_readback(
        &self,
        awaiting: ResidentA2aCancelTaskAwaitingCustody,
    ) -> Result<
        (
            ResidentA2aCancelTaskReadbackCustodyToken,
            CompletedA2aCancelTaskReadback,
        ),
        ResidentA2aCancelTaskAwaitingCustody,
    > {
        let ResidentA2aCancelTaskAwaitingCustody {
            owner_instance,
            key,
            outcome,
            settlement,
        } = awaiting;
        let ResidentA2aCancelTaskOutcome::ReadbackCompleted(completed) = outcome else {
            return Err(ResidentA2aCancelTaskAwaitingCustody {
                owner_instance,
                key,
                outcome,
                settlement,
            });
        };
        Ok((
            ResidentA2aCancelTaskReadbackCustodyToken {
                owner_instance,
                key,
                settlement,
            },
            *completed,
        ))
    }

    /// Prove exact terminal ownership before any remote-history custody I/O.
    pub(in crate::resident) fn require_readback(
        &self,
        token: &ResidentA2aCancelTaskReadbackCustodyToken,
        reservations: &ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        if !self.owner_instance.is_same(&token.owner_instance) {
            return Err(ResidentTaskReservationTransitionFailure::OwnerMismatch);
        }
        reservations.require_a2a_cancel_task_completion(&token.settlement, &token.key)
    }

    /// Attach exact remote-history custody without releasing aggregate capacity.
    pub(in crate::resident) fn retain_readback(
        &self,
        token: ResidentA2aCancelTaskReadbackCustodyToken,
        custody: CompletedA2aCancelTaskReadbackCustody,
        reservations: &ResidentAggregateTaskReservations,
    ) -> Result<ResidentRetainedA2aCancelTaskOutcome, ResidentA2aCancelTaskReadbackAssociationFailure>
    {
        if self.require_readback(&token, reservations).is_err()
            || !token.key.matches_binding(custody.binding())
            || custody.custody().candidate().store() != &token.key.store
            || !readback_custody_is_self_consistent(&custody)
        {
            return Err(ResidentA2aCancelTaskReadbackAssociationFailure {
                token: Box::new(token),
                custody: Box::new(custody),
            });
        }
        Ok(ResidentRetainedA2aCancelTaskOutcome {
            owner_instance: token.owner_instance,
            key: token.key,
            settlement: token.settlement,
            custody: ResidentA2aCancelTaskDurableCustody::ReadbackCustody(Box::new(custody)),
        })
    }

    /// Select an incomplete carrier outcome for durable local disposition.
    pub(in crate::resident) fn select_disposition(
        &self,
        awaiting: ResidentA2aCancelTaskAwaitingCustody,
    ) -> Result<
        (
            ResidentA2aCancelTaskDispositionCustodyToken,
            A2aCancelTaskDispositionPhase,
        ),
        ResidentA2aCancelTaskAwaitingCustody,
    > {
        let ResidentA2aCancelTaskAwaitingCustody {
            owner_instance,
            key,
            outcome,
            settlement,
        } = awaiting;
        let phase = match outcome {
            ResidentA2aCancelTaskOutcome::NotStarted(phase) => {
                A2aCancelTaskDispositionPhase::NotStarted(phase)
            }
            ResidentA2aCancelTaskOutcome::OutcomeUnknown(phase) => {
                A2aCancelTaskDispositionPhase::OutcomeUnknown(phase)
            }
            outcome @ ResidentA2aCancelTaskOutcome::ReadbackCompleted(_) => {
                return Err(ResidentA2aCancelTaskAwaitingCustody {
                    owner_instance,
                    key,
                    outcome,
                    settlement,
                });
            }
        };
        let cancellation = {
            let (reference, value) = phase.local_cancellation();
            (
                reference.clone(),
                value.canonical_operation_observation_sha256().clone(),
            )
        };
        Ok((
            ResidentA2aCancelTaskDispositionCustodyToken {
                owner_instance,
                key,
                settlement,
                cancellation,
            },
            phase,
        ))
    }

    /// Prove exact incomplete-outcome ownership before any disposition I/O.
    pub(in crate::resident) fn require_disposition(
        &self,
        token: &ResidentA2aCancelTaskDispositionCustodyToken,
        reservations: &ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        if !self.owner_instance.is_same(&token.owner_instance) {
            return Err(ResidentTaskReservationTransitionFailure::OwnerMismatch);
        }
        reservations.require_a2a_cancel_task_completion(&token.settlement, &token.key)
    }

    /// Attach exact local disposition custody without releasing capacity.
    pub(in crate::resident) fn retain_disposition(
        &self,
        token: ResidentA2aCancelTaskDispositionCustodyToken,
        custody: CompletedA2aCancelTaskDispositionCustody,
        reservations: &ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentRetainedA2aCancelTaskOutcome,
        ResidentA2aCancelTaskDispositionAssociationFailure,
    > {
        if self.require_disposition(&token, reservations).is_err()
            || !token.key.matches_binding(custody.binding())
            || custody.cancellation() != token.cancellation()
            || custody.request_custody().candidate().store() != &token.key.store
            || custody.disposition_custody().candidate().store() != &token.key.store
        {
            return Err(ResidentA2aCancelTaskDispositionAssociationFailure {
                token: Box::new(token),
                custody: Box::new(custody),
            });
        }
        Ok(ResidentRetainedA2aCancelTaskOutcome {
            owner_instance: token.owner_instance,
            key: token.key,
            settlement: token.settlement,
            custody: ResidentA2aCancelTaskDurableCustody::Disposition(Box::new(custody)),
        })
    }

    /// Release aggregate capacity after exact durable typed handling.
    pub(in crate::resident) fn finish_retained(
        &self,
        retained: ResidentRetainedA2aCancelTaskOutcome,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentA2aCancelTaskDurableCustody, ResidentA2aCancelTaskReleaseFailure> {
        if !self.owner_instance.is_same(&retained.owner_instance) {
            return Err(ResidentA2aCancelTaskReleaseFailure {
                retained: Box::new(retained),
                reason: ResidentTaskReservationTransitionFailure::OwnerMismatch,
            });
        }
        let custody_matches = match &retained.custody {
            ResidentA2aCancelTaskDurableCustody::ReadbackCustody(custody) => {
                retained.key.matches_binding(custody.binding())
                    && custody.custody().candidate().store() == &retained.key.store
                    && readback_custody_is_self_consistent(custody)
            }
            ResidentA2aCancelTaskDurableCustody::Disposition(custody) => {
                retained.key.matches_binding(custody.binding())
                    && custody.request_custody().candidate().store() == &retained.key.store
                    && custody.disposition_custody().candidate().store() == &retained.key.store
            }
        };
        if !custody_matches {
            return Err(ResidentA2aCancelTaskReleaseFailure {
                retained: Box::new(retained),
                reason: ResidentTaskReservationTransitionFailure::NotFound,
            });
        }
        if let Err(reason) =
            reservations.require_a2a_cancel_task_completion(&retained.settlement, &retained.key)
        {
            return Err(ResidentA2aCancelTaskReleaseFailure {
                retained: Box::new(retained),
                reason,
            });
        }
        let ResidentRetainedA2aCancelTaskOutcome {
            owner_instance: _,
            key,
            settlement,
            custody,
        } = retained;
        match reservations.release_completion(*settlement) {
            Ok(()) => Ok(custody),
            Err(failure) => Err(ResidentA2aCancelTaskReleaseFailure {
                retained: Box::new(ResidentRetainedA2aCancelTaskOutcome {
                    owner_instance: self.owner_instance.clone(),
                    key,
                    settlement: failure.settlement,
                    custody,
                }),
                reason: failure.reason,
            }),
        }
    }

    fn close(
        &self,
        key: ResidentA2aTaskWorkKey,
        outcome: ResidentA2aCancelTaskOutcome,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aCancelTaskOwnerPoll {
        match reservations
            .mark_a2a_task_completion_pending(&key, &ResidentTaskReservationRoute::A2aCancelTask)
        {
            Ok(settlement) => ResidentA2aCancelTaskOwnerPoll::AwaitingCustody(
                ResidentA2aCancelTaskAwaitingCustody {
                    owner_instance: self.owner_instance.clone(),
                    key: Box::new(key),
                    outcome,
                    settlement: Box::new(settlement),
                },
            ),
            Err(reason) => ResidentA2aCancelTaskOwnerPoll::AggregateTransitionRejected(
                ResidentA2aCancelTaskAggregateTransitionRejection {
                    owner_instance: self.owner_instance.clone(),
                    key: Box::new(key),
                    outcome,
                    reason,
                },
            ),
        }
    }

    fn start_from_poll<'a>(poll: ResidentA2aCancelTaskOwnerPoll) -> ResidentA2aCancelTaskStart<'a> {
        match poll {
            ResidentA2aCancelTaskOwnerPoll::AwaitingCustody(awaiting) => {
                ResidentA2aCancelTaskStart::AwaitingCustody(awaiting)
            }
            ResidentA2aCancelTaskOwnerPoll::AggregateTransitionRejected(rejected) => {
                ResidentA2aCancelTaskStart::AggregateTransitionRejected(rejected)
            }
            _ => unreachable!("a carrier-closed start has one terminal aggregate transition"),
        }
    }
}

fn readback_custody_is_self_consistent(custody: &CompletedA2aCancelTaskReadbackCustody) -> bool {
    let binding = custody.binding();
    let observation = custody.observation();
    let checkpoint = custody.checkpoint();
    let preimage = custody.checkpoint_preimage();
    observation.1.task_binding_ref() == binding.0
        && observation.1.task_binding_sha256() == binding.1.canonical_a2a_task_binding_sha256()
        && checkpoint.0
            == custody
                .custody()
                .candidate()
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref()
        && checkpoint.1.control_checkpoint_id() == &preimage.control_checkpoint_id
        && *checkpoint.1.domain_sequence() == preimage.domain_sequence
        && *checkpoint.1.committed_transaction_sequence() == preimage.committed_transaction_sequence
}

/// Offered reservation release failed before any carrier use.
#[must_use = "retry only exact aggregate offer removal"]
pub(in crate::resident) enum ResidentA2aCancelTaskReservationReleaseFailure {
    /// The reservation belongs to another cancellation owner.
    Association(ResidentA2aCancelTaskReservation),
    /// The exact ledger transition failed and can be retried without other I/O.
    Transition {
        owner_instance: ResidentOperationTaskOwnerInstance,
        key: Box<ResidentA2aTaskWorkKey>,
        failure: Box<ResidentTaskReservationOfferTransitionFailure>,
    },
}

/// Fresh start stopped without granting another carrier attempt.
pub(in crate::resident) enum ResidentA2aCancelTaskStartTransitionFailure<'a> {
    Association {
        reservation: ResidentA2aCancelTaskReservation,
        retained: Box<RetainedA2aCancelTask<'a>>,
    },
    AggregateAcceptance(ResidentA2aCancelTaskAggregateAcceptanceFailure<'a>),
}
