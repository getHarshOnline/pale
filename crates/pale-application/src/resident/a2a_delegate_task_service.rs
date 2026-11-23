//! Process-local ownership for started A2A `Delegate` sends.
//!
//! Durable send preparation and outcome custody remain authoritative. This
//! owner only keeps a started exchange across short-lived resident host
//! borrows and binds it to the shared process occupancy ledger. It never
//! recreates or replays `SendMessage`.

use std::{num::NonZeroUsize, task::Context};

use pale_core::records::*;
use pale_ports::{
    A2aExchangePort, A2aSendMessageEvidencePort, AntiRollbackAnchorPort, OperationCustodyPort,
    WallClockSourcePort,
};

use super::operation_dispatch::sample_time;
use super::operation_dispatch::A2aOutcomeRetentionKind;
use super::{
    launch_a2a_delegate, recover_a2a_send_outcome_unknown, A2aDelegateLaunchFailure,
    A2aDelegateLaunchInput, A2aSendCustodyFailure, A2aSendRecoveryAdmission, A2aSendRecoveryError,
    A2aSendRecoveryInput, A2aSendResponseFailure, A2aSendResponsePoll, A2aSendStartFailure,
    AmbientSessionAdmissionContext, AmbientSessionAuthorityPort, AwaitingA2aDelegate,
    CompletedA2aNoTaskResponse, CompletedA2aSendResponse, CompletedA2aTaskResponse,
    OperationAttemptError, RecoveredA2aSendDiagnostic, RecoveredA2aSendOutcomeUnknown,
    RetainedA2aOutcome, RetainedA2aSend,
};
use super::{
    task_owner::{
        ResidentOperationTaskKey, ResidentOperationTaskOwnerBuildError,
        ResidentOperationTaskOwnerInstance,
    },
    task_reservation::{
        ResidentAggregateTaskReservations, ResidentTaskReservationOfferFailure,
        ResidentTaskReservationOfferTransitionFailure, ResidentTaskReservationRoute,
        ResidentTaskReservationSettlement, ResidentTaskReservationTransitionFailure,
    },
};

struct ActiveA2aDelegate {
    key: ResidentOperationTaskKey,
    awaiting: AwaitingA2aDelegate,
}

/// Process-local owner of accepted A2A sends for one resident lifetime.
#[must_use = "retain the A2A task owner for the resident lifetime"]
pub(in crate::resident) struct ResidentA2aDelegateTaskService {
    owner_instance: ResidentOperationTaskOwnerInstance,
    active: Vec<ActiveA2aDelegate>,
}

/// Exact active A2A send selected without changing process-local state.
///
/// The token is valid only for this task-owner instance and dispatch key. It
/// grants no remote cancellation authority and can stop only the local
/// response read after durable cancellation custody exists.
#[must_use = "consume only after the cancellation request reaches durable custody"]
pub(in crate::resident) struct ResidentA2aDelegateDispatchOwner {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: ResidentOperationTaskKey,
}

/// Pure A2A send-ownership result for one durable dispatch.
pub(in crate::resident) enum ResidentA2aDelegateDispatchOwnerSelection {
    Missing,
    Owned(ResidentA2aDelegateDispatchOwner),
}

/// Local response-read stop that remains in the ordinary A2A retention path.
#[must_use = "retain the uncertain send outcome before releasing task capacity"]
pub(in crate::resident) struct ResidentA2aDelegateCancellationContinuation {
    key: ResidentOperationTaskKey,
    poll: ResidentA2aDelegatePoll,
}

impl ResidentA2aDelegateCancellationContinuation {
    /// Split the exact dispatch key from its non-replayable local-stop result.
    pub(in crate::resident) fn into_parts(
        self,
    ) -> (ResidentOperationTaskKey, ResidentA2aDelegatePoll) {
        (self.key, self.poll)
    }
}

impl ResidentA2aDelegateTaskService {
    /// Reserve enough local slots for the shared aggregate ceiling.
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

    /// Return started sends whose first response remains unresolved.
    #[must_use]
    pub(in crate::resident) fn in_flight(&self) -> usize {
        self.active.len()
    }

    /// Select one active send without stopping or polling it.
    #[must_use]
    pub(in crate::resident) fn select_dispatch_owner(
        &self,
        key: &ResidentOperationTaskKey,
    ) -> ResidentA2aDelegateDispatchOwnerSelection {
        if self.active.iter().any(|active| &active.key == key) {
            ResidentA2aDelegateDispatchOwnerSelection::Owned(ResidentA2aDelegateDispatchOwner {
                owner_instance: self.owner_instance.clone(),
                key: key.clone(),
            })
        } else {
            ResidentA2aDelegateDispatchOwnerSelection::Missing
        }
    }

    /// Consume one exact owner token and stop only its local response read.
    ///
    /// The caller must establish durable cancellation custody before this
    /// method runs. An unlike or already-consumed token changes no send.
    pub(in crate::resident) fn stop_dispatch_response_read(
        &mut self,
        owner: ResidentA2aDelegateDispatchOwner,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aDelegateCancellationContinuation {
        let key = owner.key;
        if !self.owner_instance.is_same(&owner.owner_instance)
            || !self.active.iter().any(|active| active.key == key)
        {
            return ResidentA2aDelegateCancellationContinuation {
                key,
                poll: ResidentA2aDelegatePoll::NotFound,
            };
        }
        let poll = self.cancel(&key, reservations);
        debug_assert!(!matches!(poll, ResidentA2aDelegatePoll::NotFound));
        ResidentA2aDelegateCancellationContinuation { key, poll }
    }

    /// Launch once after reserving shared capacity and retain the pending exchange.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn launch<'operation, 'call, Authority, Disclosure, Transport, Sign>(
        &mut self,
        input: A2aDelegateLaunchInput<'operation, 'call>,
        authority: &mut Authority,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        exchange: &dyn A2aExchangePort,
        reservations: &mut ResidentAggregateTaskReservations,
        admit_disclosure: Disclosure,
        admit_transport: Transport,
        sign_checkpoint: Sign,
    ) -> Result<ResidentA2aDelegateLaunch, ResidentA2aDelegateLaunchFailure<'operation, 'call>>
    where
        Authority: AmbientSessionAuthorityPort,
        Disclosure: FnMut(
            &pale_ports::ProviderInvocationRequest<'_>,
            &A2aRouteAdmissionV1,
            &str,
        ) -> Result<(), InvalidResidentRecordV1>,
        Transport: FnOnce(
            &ImmutableRecordReferenceV1,
            &pale_ports::ProviderInvocationRequest<'_>,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let key = match input.send.dispatch.task_key() {
            Ok(key) => key,
            Err(reason) => {
                return Err(ResidentA2aDelegateLaunchFailure::Association {
                    input: Box::new(input),
                    reason,
                });
            }
        };
        let current = input.send.current;
        let response_ref = input.response_ref.clone();
        let offer = match reservations
            .begin_offer(key.clone(), ResidentTaskReservationRoute::A2aDelegate)
        {
            Ok(offer) => offer,
            Err(reason) => {
                return Err(ResidentA2aDelegateLaunchFailure::Reservation {
                    input: Box::new(input),
                    reason,
                });
            }
        };

        match launch_a2a_delegate(
            input,
            authority,
            custody,
            anchor,
            exchange,
            admit_disclosure,
            admit_transport,
            sign_checkpoint,
        ) {
            Ok(awaiting) => {
                let actual = match awaiting.task_key() {
                    Ok(actual) => actual,
                    Err(reason) => {
                        return Err(ResidentA2aDelegateLaunchFailure::StartedAssociation {
                            completion: Box::new(ResidentA2aDelegateOfferedCompletion {
                                owner_instance: self.owner_instance.clone(),
                                key,
                                outcome: response_failure_outcome(awaiting.cancel()),
                                offer,
                            }),
                            reason,
                        });
                    }
                };
                if actual != key {
                    return Err(ResidentA2aDelegateLaunchFailure::StartedAssociation {
                        completion: Box::new(ResidentA2aDelegateOfferedCompletion {
                            owner_instance: self.owner_instance.clone(),
                            key,
                            outcome: response_failure_outcome(awaiting.cancel()),
                            offer,
                        }),
                        reason: OperationAttemptError::Association,
                    });
                }
                if let Err(failure) = reservations.accept_offer(offer) {
                    return Err(ResidentA2aDelegateLaunchFailure::OfferSettlement(Box::new(
                        self.offered_rejection(
                            key,
                            response_failure_outcome(awaiting.cancel()),
                            failure,
                        ),
                    )));
                }
                self.active.push(ActiveA2aDelegate {
                    key: key.clone(),
                    awaiting,
                });
                Ok(ResidentA2aDelegateLaunch::Awaiting(key))
            }
            Err(A2aDelegateLaunchFailure::Start(A2aSendStartFailure::Rejected(response))) => {
                let settlement = match reservations.complete_offer(offer) {
                    Ok(settlement) => settlement,
                    Err(failure) => {
                        return Err(ResidentA2aDelegateLaunchFailure::OfferSettlement(Box::new(
                            self.offered_rejection(
                                key,
                                ResidentA2aDelegateImmediateOutcome::NotStarted(response),
                                failure,
                            ),
                        )));
                    }
                };
                Ok(ResidentA2aDelegateLaunch::Completed(
                    ResidentA2aDelegateCompletion::NotStarted {
                        response,
                        finalization: self.finalization(
                            key,
                            ResidentA2aDelegateOutcomeKind::NotStarted,
                            ResidentA2aDelegateSettlementKind::Completion,
                            settlement,
                        ),
                    },
                ))
            }
            Err(A2aDelegateLaunchFailure::Start(A2aSendStartFailure::OutcomeUnknown(outcome))) => {
                let settlement = match reservations.complete_offer(offer) {
                    Ok(settlement) => settlement,
                    Err(failure) => {
                        return Err(ResidentA2aDelegateLaunchFailure::OfferSettlement(Box::new(
                            self.offered_rejection(
                                key,
                                ResidentA2aDelegateImmediateOutcome::OutcomeUnknown(outcome),
                                failure,
                            ),
                        )));
                    }
                };
                Ok(ResidentA2aDelegateLaunch::Completed(
                    ResidentA2aDelegateCompletion::OutcomeUnknown {
                        outcome,
                        finalization: self.finalization(
                            key,
                            ResidentA2aDelegateOutcomeKind::OutcomeUnknown,
                            ResidentA2aDelegateSettlementKind::Completion,
                            settlement,
                        ),
                    },
                ))
            }
            Err(A2aDelegateLaunchFailure::Start(A2aSendStartFailure::Diagnostic(failure))) => {
                let settlement = match reservations.complete_offer(offer) {
                    Ok(settlement) => settlement,
                    Err(offer_failure) => {
                        return Err(ResidentA2aDelegateLaunchFailure::OfferSettlement(Box::new(
                            self.offered_rejection(
                                key,
                                ResidentA2aDelegateImmediateOutcome::Diagnostic(failure),
                                offer_failure,
                            ),
                        )));
                    }
                };
                Ok(ResidentA2aDelegateLaunch::Diagnostic(
                    ResidentA2aDelegateDiagnostic {
                        failure,
                        finalization: ResidentA2aDelegatePendingFinalization {
                            owner_instance: self.owner_instance.clone(),
                            key,
                            settlement,
                        },
                    },
                ))
            }
            Err(
                failure @ (A2aDelegateLaunchFailure::Preparation(_)
                | A2aDelegateLaunchFailure::Checkpoint(_)
                | A2aDelegateLaunchFailure::PreCustodyClock(_)),
            ) => match reservations.refuse_offer(offer) {
                Ok(()) => Err(ResidentA2aDelegateLaunchFailure::Launch(Box::new(failure))),
                Err(reservation) => Err(ResidentA2aDelegateLaunchFailure::Rollback {
                    launch: Box::new(failure),
                    reservation,
                }),
            },
            Err(A2aDelegateLaunchFailure::Custody(custody)) => {
                Err(ResidentA2aDelegateLaunchFailure::Custody(Box::new(
                    ResidentA2aDelegateUnresolvedLaunch {
                        owner_instance: self.owner_instance.clone(),
                        key,
                        custody,
                        current,
                        response_ref,
                        offer,
                    },
                )))
            }
            Err(A2aDelegateLaunchFailure::ReleaseClock { retained, reason }) => {
                Err(ResidentA2aDelegateLaunchFailure::Unopened(Box::new(
                    ResidentA2aDelegateUnopenedLaunch {
                        owner_instance: self.owner_instance.clone(),
                        key,
                        retained: *retained,
                        current,
                        response_ref,
                        offer,
                        reason: ResidentA2aDelegateUnopenedReason::Clock(reason),
                    },
                )))
            }
            Err(A2aDelegateLaunchFailure::Start(A2aSendStartFailure::Preflight {
                retained,
                reason,
            })) => Err(ResidentA2aDelegateLaunchFailure::Unopened(Box::new(
                ResidentA2aDelegateUnopenedLaunch {
                    owner_instance: self.owner_instance.clone(),
                    key,
                    retained: *retained,
                    current,
                    response_ref,
                    offer,
                    reason: ResidentA2aDelegateUnopenedReason::Preflight(reason),
                },
            ))),
        }
    }

    /// Retry only returning an offer after launch proved no carrier start.
    pub(in crate::resident) fn retry_launch_rollback<'operation, 'call>(
        &self,
        failure: ResidentA2aDelegateLaunchFailure<'operation, 'call>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentA2aDelegateLaunchFailure<'operation, 'call>,
        ResidentA2aDelegateLaunchFailure<'operation, 'call>,
    > {
        let ResidentA2aDelegateLaunchFailure::Rollback {
            launch,
            reservation,
        } = failure
        else {
            return Err(failure);
        };
        match reservations.retry_refuse_offer(reservation) {
            Ok(()) => Ok(ResidentA2aDelegateLaunchFailure::Launch(launch)),
            Err(reservation) => Err(ResidentA2aDelegateLaunchFailure::Rollback {
                launch,
                reservation,
            }),
        }
    }

    /// Settle a closed launch whose returned exchange had an unlike task key.
    pub(in crate::resident) fn settle_started_association<'operation, 'call>(
        &self,
        failure: ResidentA2aDelegateLaunchFailure<'operation, 'call>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentA2aDelegateLaunch, ResidentA2aDelegateLaunchFailure<'operation, 'call>>
    {
        let ResidentA2aDelegateLaunchFailure::StartedAssociation { completion, reason } = failure
        else {
            return Err(failure);
        };
        if !self.owner_instance.is_same(&completion.owner_instance) {
            return Err(ResidentA2aDelegateLaunchFailure::StartedAssociation {
                completion,
                reason,
            });
        }
        let ResidentA2aDelegateOfferedCompletion {
            owner_instance: _,
            key,
            outcome,
            offer,
        } = *completion;
        self.complete_unopened_offer(key, outcome, offer, reservations)
    }

    /// Continue only an exact unresolved database or anchor transition.
    ///
    /// The aggregate offer is checked before storage or anchor I/O. Exact
    /// read-back and owner-approved physical retry happen in the custody owner;
    /// this service never retries `SendMessage` from an ambiguous commit.
    pub(in crate::resident) fn continue_custody<'operation, 'call>(
        &self,
        unresolved: ResidentA2aDelegateUnresolvedLaunch<'operation, 'call>,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        reservations: &ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentA2aDelegateUnopenedLaunch<'operation, 'call>,
        ResidentA2aDelegateCustodyContinuationFailure<'operation, 'call>,
    > {
        let reservation = if self.owner_instance.is_same(&unresolved.owner_instance) {
            reservations.require_a2a_offer(&unresolved.offer, &unresolved.key)
        } else {
            Err(ResidentTaskReservationTransitionFailure::OwnerMismatch)
        };
        if let Err(reason) = reservation {
            return Err(ResidentA2aDelegateCustodyContinuationFailure {
                unresolved: Box::new(unresolved),
                reservation: Some(reason),
            });
        }
        let ResidentA2aDelegateUnresolvedLaunch {
            owner_instance,
            key,
            custody: unresolved_custody,
            current,
            response_ref,
            offer,
        } = unresolved;
        match unresolved_custody.continue_exact(custody, anchor) {
            Ok(retained) => Ok(ResidentA2aDelegateUnopenedLaunch {
                owner_instance,
                key,
                retained,
                current,
                response_ref,
                offer,
                reason: ResidentA2aDelegateUnopenedReason::CustodyResolved,
            }),
            Err(custody) => Err(ResidentA2aDelegateCustodyContinuationFailure {
                unresolved: Box::new(ResidentA2aDelegateUnresolvedLaunch {
                    owner_instance,
                    key,
                    custody,
                    current,
                    response_ref,
                    offer,
                }),
                reservation: None,
            }),
        }
    }

    /// Convert exact recovery-only custody into durable outcome uncertainty.
    ///
    /// This path is unavailable until custody read-back proves that an earlier
    /// commit exists without carrier-release permission. It uses the existing
    /// protected recovery owner and never invokes the carrier.
    pub(in crate::resident) fn recover_unresolved_as_unknown<'operation, 'call, 'a>(
        &self,
        unresolved: ResidentA2aDelegateUnresolvedLaunch<'operation, 'call>,
        input: A2aSendRecoveryInput<'a>,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        admission: A2aSendRecoveryAdmission<'_>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentRecoveredA2aDelegate<'a>,
        ResidentA2aDelegateUnresolvedRecoveryFailure<'operation, 'call, 'a>,
    > {
        let reservation = if self.owner_instance.is_same(&unresolved.owner_instance) {
            reservations.require_a2a_offer(&unresolved.offer, &unresolved.key)
        } else {
            Err(ResidentTaskReservationTransitionFailure::OwnerMismatch)
        };
        if let Err(reason) = reservation {
            return Err(ResidentA2aDelegateUnresolvedRecoveryFailure::Reservation {
                unresolved: Box::new(unresolved),
                reason,
            });
        }
        if !unresolved.custody.is_recovery_only() {
            return Err(ResidentA2aDelegateUnresolvedRecoveryFailure::Custody(
                Box::new(unresolved),
            ));
        }
        let recovered_key = match task_key_from_recovered(input.recovered) {
            Ok(key) => key,
            Err(reason) => {
                return Err(ResidentA2aDelegateUnresolvedRecoveryFailure::Association {
                    unresolved: Box::new(unresolved),
                    reason,
                });
            }
        };
        if recovered_key != unresolved.key {
            return Err(ResidentA2aDelegateUnresolvedRecoveryFailure::Association {
                unresolved: Box::new(unresolved),
                reason: OperationAttemptError::Association,
            });
        }
        let outcome = match recover_a2a_send_outcome_unknown(input, custody, anchor, admission) {
            Ok(outcome) => outcome,
            Err(reason) => {
                return Err(ResidentA2aDelegateUnresolvedRecoveryFailure::Recovery {
                    unresolved: Box::new(unresolved),
                    reason,
                });
            }
        };
        let ResidentA2aDelegateUnresolvedLaunch {
            owner_instance,
            key,
            custody: _,
            current: _,
            response_ref: _,
            offer,
        } = unresolved;
        let settlement = match reservations.lose_offer(offer) {
            Ok(settlement) => settlement,
            Err(failure) => {
                return Err(
                    ResidentA2aDelegateUnresolvedRecoveryFailure::LossReservation(Box::new(
                        ResidentA2aDelegateRecoveredOfferRejection {
                            recovered: Box::new(ResidentA2aDelegateRecoveredOffer {
                                owner_instance,
                                key,
                                outcome: Box::new(outcome),
                                offer: *failure.offer,
                            }),
                            reason: failure.reason,
                        },
                    )),
                );
            }
        };
        Ok(ResidentRecoveredA2aDelegate {
            outcome: Box::new(outcome),
            finalization: self.finalization(
                key,
                ResidentA2aDelegateOutcomeKind::OutcomeUnknown,
                ResidentA2aDelegateSettlementKind::Loss,
                settlement,
            ),
        })
    }

    /// Retry only the loss-pending aggregate transition after recovery succeeded.
    ///
    /// This performs no carrier, clock, storage, signing, anchor, or recovery I/O.
    pub(in crate::resident) fn retry_recovered_loss_offer<'a>(
        &self,
        rejected: ResidentA2aDelegateRecoveredOfferRejection<'a>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentRecoveredA2aDelegate<'a>, ResidentA2aDelegateRecoveredOfferRejection<'a>>
    {
        let ResidentA2aDelegateRecoveredOfferRejection {
            recovered,
            reason: _,
        } = rejected;
        let recovered = *recovered;
        let reservation = if self.owner_instance.is_same(&recovered.owner_instance) {
            reservations.require_a2a_offer(&recovered.offer, &recovered.key)
        } else {
            Err(ResidentTaskReservationTransitionFailure::OwnerMismatch)
        };
        if let Err(reason) = reservation {
            return Err(ResidentA2aDelegateRecoveredOfferRejection {
                recovered: Box::new(recovered),
                reason,
            });
        }
        let ResidentA2aDelegateRecoveredOffer {
            owner_instance: _,
            key,
            outcome,
            offer,
        } = recovered;
        let settlement = match reservations.lose_offer(offer) {
            Ok(settlement) => settlement,
            Err(failure) => {
                return Err(ResidentA2aDelegateRecoveredOfferRejection {
                    recovered: Box::new(ResidentA2aDelegateRecoveredOffer {
                        owner_instance: self.owner_instance.clone(),
                        key,
                        outcome,
                        offer: *failure.offer,
                    }),
                    reason: failure.reason,
                });
            }
        };
        Ok(ResidentRecoveredA2aDelegate {
            outcome,
            finalization: self.finalization(
                key,
                ResidentA2aDelegateOutcomeKind::OutcomeUnknown,
                ResidentA2aDelegateSettlementKind::Loss,
                settlement,
            ),
        })
    }

    /// Retry an exact send only while the carrier is proven unopened.
    ///
    /// The aggregate offer is checked before clock, storage, anchor, authority,
    /// or carrier I/O. Once the carrier starts, no returned branch contains the
    /// unopened owner and no later call can replay `SendMessage`.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retry_unopened<'operation, 'call, Authority, Disclosure>(
        &mut self,
        unopened: ResidentA2aDelegateUnopenedLaunch<'operation, 'call>,
        authority: &mut Authority,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        reservations: &mut ResidentAggregateTaskReservations,
        admit_disclosure: Disclosure,
    ) -> Result<ResidentA2aDelegateLaunch, ResidentA2aDelegateUnopenedRetryFailure<'operation, 'call>>
    where
        Authority: AmbientSessionAuthorityPort,
        Disclosure: FnOnce(
            &pale_ports::ProviderInvocationRequest<'_>,
            &A2aRouteAdmissionV1,
            &str,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let reservation = if self.owner_instance.is_same(&unopened.owner_instance) {
            reservations.require_a2a_offer(&unopened.offer, &unopened.key)
        } else {
            Err(ResidentTaskReservationTransitionFailure::OwnerMismatch)
        };
        if let Err(reason) = reservation {
            return Err(ResidentA2aDelegateUnopenedRetryFailure::Reservation {
                unopened: Box::new(unopened),
                reason,
            });
        }
        let ResidentA2aDelegateUnopenedLaunch {
            owner_instance,
            key,
            retained,
            current,
            response_ref,
            offer,
            reason: _,
        } = unopened;
        let release_time = match sample_time(clock, retained.minimum_release_time()) {
            Ok(release_time) => release_time,
            Err(reason) => {
                return Err(ResidentA2aDelegateUnopenedRetryFailure::Unopened(Box::new(
                    ResidentA2aDelegateUnopenedLaunch {
                        owner_instance,
                        key,
                        retained,
                        current,
                        response_ref,
                        offer,
                        reason: ResidentA2aDelegateUnopenedReason::Clock(reason),
                    },
                )));
            }
        };
        let release_context = current.at_time(release_time);
        let awaiting = match retained.start_awaiting(
            &release_context,
            authority,
            custody,
            anchor,
            exchange,
            response_ref.clone(),
            admit_disclosure,
        ) {
            Ok(awaiting) => AwaitingA2aDelegate::Response(awaiting),
            Err(A2aSendStartFailure::Preflight { retained, reason }) => {
                return Err(ResidentA2aDelegateUnopenedRetryFailure::Unopened(Box::new(
                    ResidentA2aDelegateUnopenedLaunch {
                        owner_instance,
                        key,
                        retained: *retained,
                        current,
                        response_ref,
                        offer,
                        reason: ResidentA2aDelegateUnopenedReason::Preflight(reason),
                    },
                )));
            }
            Err(A2aSendStartFailure::Rejected(response)) => {
                return self
                    .complete_unopened_offer(
                        key,
                        ResidentA2aDelegateImmediateOutcome::NotStarted(response),
                        offer,
                        reservations,
                    )
                    .map_err(|failure| {
                        ResidentA2aDelegateUnopenedRetryFailure::Closed(Box::new(failure))
                    });
            }
            Err(A2aSendStartFailure::OutcomeUnknown(outcome)) => {
                return self
                    .complete_unopened_offer(
                        key,
                        ResidentA2aDelegateImmediateOutcome::OutcomeUnknown(outcome),
                        offer,
                        reservations,
                    )
                    .map_err(|failure| {
                        ResidentA2aDelegateUnopenedRetryFailure::Closed(Box::new(failure))
                    });
            }
            Err(A2aSendStartFailure::Diagnostic(failure)) => {
                return self
                    .complete_unopened_offer(
                        key,
                        ResidentA2aDelegateImmediateOutcome::Diagnostic(failure),
                        offer,
                        reservations,
                    )
                    .map_err(|failure| {
                        ResidentA2aDelegateUnopenedRetryFailure::Closed(Box::new(failure))
                    });
            }
        };
        let actual = match awaiting.task_key() {
            Ok(actual) => actual,
            Err(reason) => {
                return Err(ResidentA2aDelegateUnopenedRetryFailure::Closed(Box::new(
                    ResidentA2aDelegateLaunchFailure::StartedAssociation {
                        completion: Box::new(ResidentA2aDelegateOfferedCompletion {
                            owner_instance,
                            key,
                            outcome: response_failure_outcome(awaiting.cancel()),
                            offer,
                        }),
                        reason,
                    },
                )));
            }
        };
        if actual != key {
            return Err(ResidentA2aDelegateUnopenedRetryFailure::Closed(Box::new(
                ResidentA2aDelegateLaunchFailure::StartedAssociation {
                    completion: Box::new(ResidentA2aDelegateOfferedCompletion {
                        owner_instance,
                        key,
                        outcome: response_failure_outcome(awaiting.cancel()),
                        offer,
                    }),
                    reason: OperationAttemptError::Association,
                },
            )));
        }
        if let Err(failure) = reservations.accept_offer(offer) {
            return Err(ResidentA2aDelegateUnopenedRetryFailure::Closed(Box::new(
                ResidentA2aDelegateLaunchFailure::OfferSettlement(Box::new(
                    self.offered_rejection(
                        key,
                        response_failure_outcome(awaiting.cancel()),
                        failure,
                    ),
                )),
            )));
        }
        self.active.push(ActiveA2aDelegate {
            key: key.clone(),
            awaiting,
        });
        Ok(ResidentA2aDelegateLaunch::Awaiting(key))
    }

    fn complete_unopened_offer<'operation, 'call>(
        &self,
        key: ResidentOperationTaskKey,
        outcome: ResidentA2aDelegateImmediateOutcome,
        offer: super::task_reservation::ResidentTaskReservationOffer,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentA2aDelegateLaunch, ResidentA2aDelegateLaunchFailure<'operation, 'call>>
    {
        let settlement = match reservations.complete_offer(offer) {
            Ok(settlement) => settlement,
            Err(failure) => {
                return Err(ResidentA2aDelegateLaunchFailure::OfferSettlement(Box::new(
                    self.offered_rejection(key, outcome, failure),
                )));
            }
        };
        match self.completed_poll(key, outcome, settlement) {
            ResidentA2aDelegatePoll::Completed(completion) => {
                Ok(ResidentA2aDelegateLaunch::Completed(completion))
            }
            ResidentA2aDelegatePoll::Diagnostic(diagnostic) => {
                Ok(ResidentA2aDelegateLaunch::Diagnostic(diagnostic))
            }
            _ => unreachable!("a settled unopened A2A result must be terminal"),
        }
    }

    /// Retry only the offered-to-completion transition after a closed launch.
    pub(in crate::resident) fn retry_offered_completion(
        &self,
        rejected: ResidentA2aDelegateOfferedCompletionRejection,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentA2aDelegatePoll, ResidentA2aDelegateOfferedCompletionRejection> {
        let ResidentA2aDelegateOfferedCompletionRejection {
            completion,
            reason: _,
        } = rejected;
        let completion = *completion;
        if !self.owner_instance.is_same(&completion.owner_instance) {
            return Err(ResidentA2aDelegateOfferedCompletionRejection {
                completion: Box::new(completion),
                reason: ResidentTaskReservationTransitionFailure::OwnerMismatch,
            });
        }
        let ResidentA2aDelegateOfferedCompletion {
            owner_instance: _,
            key,
            outcome,
            offer,
        } = completion;
        let settlement = match reservations.complete_offer(offer) {
            Ok(settlement) => settlement,
            Err(failure) => {
                return Err(self.offered_rejection(key, outcome, failure));
            }
        };
        Ok(self.completed_poll(key, outcome, settlement))
    }

    fn offered_rejection(
        &self,
        key: ResidentOperationTaskKey,
        outcome: ResidentA2aDelegateImmediateOutcome,
        failure: ResidentTaskReservationOfferTransitionFailure,
    ) -> ResidentA2aDelegateOfferedCompletionRejection {
        ResidentA2aDelegateOfferedCompletionRejection {
            completion: Box::new(ResidentA2aDelegateOfferedCompletion {
                owner_instance: self.owner_instance.clone(),
                key,
                outcome,
                offer: *failure.offer,
            }),
            reason: failure.reason,
        }
    }

    /// Poll one exact active exchange without moving another delegate.
    pub(in crate::resident) fn poll(
        &mut self,
        key: &ResidentOperationTaskKey,
        cx: &mut Context<'_>,
        protocol: &dyn A2aSendMessageEvidencePort,
        clock: &dyn WallClockSourcePort,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aDelegatePoll {
        let Some(index) = self.active.iter().position(|active| &active.key == key) else {
            return ResidentA2aDelegatePoll::NotFound;
        };
        let ActiveA2aDelegate { key, awaiting } = self.active.swap_remove(index);
        match awaiting.poll(cx, protocol, clock) {
            A2aSendResponsePoll::Pending(awaiting) => {
                self.active.push(ActiveA2aDelegate { key, awaiting });
                ResidentA2aDelegatePoll::Pending
            }
            A2aSendResponsePoll::Ready(result) => self.complete_active(key, result, reservations),
        }
    }

    /// Stop local reads for one active send and retain explicit uncertainty.
    pub(in crate::resident) fn cancel(
        &mut self,
        key: &ResidentOperationTaskKey,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aDelegatePoll {
        let Some(index) = self.active.iter().position(|active| &active.key == key) else {
            return ResidentA2aDelegatePoll::NotFound;
        };
        let ActiveA2aDelegate { key, awaiting } = self.active.swap_remove(index);
        self.complete_active(key, Err(awaiting.cancel()), reservations)
    }

    fn complete_active(
        &self,
        key: ResidentOperationTaskKey,
        result: Result<CompletedA2aSendResponse, A2aSendResponseFailure>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aDelegatePoll {
        let outcome = match result {
            Ok(CompletedA2aSendResponse::Task(response)) => {
                ResidentA2aDelegateImmediateOutcome::Task(response)
            }
            Ok(CompletedA2aSendResponse::NoTask(response)) => {
                ResidentA2aDelegateImmediateOutcome::NoTask(response)
            }
            Err(A2aSendResponseFailure::OutcomeUnknown(outcome)) => {
                ResidentA2aDelegateImmediateOutcome::OutcomeUnknown(outcome)
            }
            Err(A2aSendResponseFailure::Diagnostic(failure)) => {
                ResidentA2aDelegateImmediateOutcome::Diagnostic(failure)
            }
        };
        let settlement = match reservations
            .mark_completion_pending(&key, &ResidentTaskReservationRoute::A2aDelegate)
        {
            Ok(settlement) => settlement,
            Err(reason) => {
                return ResidentA2aDelegatePoll::SettlementRejected(Box::new(
                    ResidentA2aDelegateSettlementRejection {
                        settlement: Box::new(ResidentA2aDelegateUnsettledCompletion {
                            owner_instance: self.owner_instance.clone(),
                            key,
                            outcome,
                        }),
                        reason,
                    },
                ));
            }
        };
        match outcome {
            ResidentA2aDelegateImmediateOutcome::Task(response) => {
                ResidentA2aDelegatePoll::Completed(ResidentA2aDelegateCompletion::Task {
                    response,
                    finalization: self.finalization(
                        key,
                        ResidentA2aDelegateOutcomeKind::Task,
                        ResidentA2aDelegateSettlementKind::Completion,
                        settlement,
                    ),
                })
            }
            ResidentA2aDelegateImmediateOutcome::NoTask(response) => {
                ResidentA2aDelegatePoll::Completed(ResidentA2aDelegateCompletion::NoTask {
                    response,
                    finalization: self.finalization(
                        key,
                        ResidentA2aDelegateOutcomeKind::NoTask,
                        ResidentA2aDelegateSettlementKind::Completion,
                        settlement,
                    ),
                })
            }
            ResidentA2aDelegateImmediateOutcome::OutcomeUnknown(outcome) => {
                ResidentA2aDelegatePoll::Completed(ResidentA2aDelegateCompletion::OutcomeUnknown {
                    outcome,
                    finalization: self.finalization(
                        key,
                        ResidentA2aDelegateOutcomeKind::OutcomeUnknown,
                        ResidentA2aDelegateSettlementKind::Completion,
                        settlement,
                    ),
                })
            }
            ResidentA2aDelegateImmediateOutcome::Diagnostic(failure) => {
                ResidentA2aDelegatePoll::Diagnostic(ResidentA2aDelegateDiagnostic {
                    failure,
                    finalization: ResidentA2aDelegatePendingFinalization {
                        owner_instance: self.owner_instance.clone(),
                        key,
                        settlement,
                    },
                })
            }
            ResidentA2aDelegateImmediateOutcome::NotStarted(_) => {
                unreachable!("an active exchange cannot become not-started")
            }
        }
    }

    /// Retry only the aggregate transition for an already-closed exchange.
    ///
    /// This method performs no clock, storage, signing, anchor, reconciliation,
    /// or carrier I/O. A replacement resident owner receives the unchanged
    /// linear outcome and cannot mutate the original ledger.
    pub(in crate::resident) fn retry_completion_settlement(
        &self,
        rejected: ResidentA2aDelegateSettlementRejection,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aDelegatePoll {
        let ResidentA2aDelegateSettlementRejection {
            settlement,
            reason: _,
        } = rejected;
        if !self.owner_instance.is_same(&settlement.owner_instance) {
            return ResidentA2aDelegatePoll::SettlementRejected(Box::new(
                ResidentA2aDelegateSettlementRejection {
                    settlement,
                    reason: ResidentTaskReservationTransitionFailure::OwnerMismatch,
                },
            ));
        }
        let ResidentA2aDelegateUnsettledCompletion {
            owner_instance: _,
            key,
            outcome,
        } = *settlement;
        let aggregate = match reservations
            .mark_completion_pending(&key, &ResidentTaskReservationRoute::A2aDelegate)
        {
            Ok(aggregate) => aggregate,
            Err(reason) => {
                return ResidentA2aDelegatePoll::SettlementRejected(Box::new(
                    ResidentA2aDelegateSettlementRejection {
                        settlement: Box::new(ResidentA2aDelegateUnsettledCompletion {
                            owner_instance: self.owner_instance.clone(),
                            key,
                            outcome,
                        }),
                        reason,
                    },
                ));
            }
        };
        self.completed_poll(key, outcome, aggregate)
    }

    fn completed_poll(
        &self,
        key: ResidentOperationTaskKey,
        outcome: ResidentA2aDelegateImmediateOutcome,
        settlement: ResidentTaskReservationSettlement,
    ) -> ResidentA2aDelegatePoll {
        match outcome {
            ResidentA2aDelegateImmediateOutcome::Task(response) => {
                ResidentA2aDelegatePoll::Completed(ResidentA2aDelegateCompletion::Task {
                    response,
                    finalization: self.finalization(
                        key,
                        ResidentA2aDelegateOutcomeKind::Task,
                        ResidentA2aDelegateSettlementKind::Completion,
                        settlement,
                    ),
                })
            }
            ResidentA2aDelegateImmediateOutcome::NoTask(response) => {
                ResidentA2aDelegatePoll::Completed(ResidentA2aDelegateCompletion::NoTask {
                    response,
                    finalization: self.finalization(
                        key,
                        ResidentA2aDelegateOutcomeKind::NoTask,
                        ResidentA2aDelegateSettlementKind::Completion,
                        settlement,
                    ),
                })
            }
            ResidentA2aDelegateImmediateOutcome::OutcomeUnknown(outcome) => {
                ResidentA2aDelegatePoll::Completed(ResidentA2aDelegateCompletion::OutcomeUnknown {
                    outcome,
                    finalization: self.finalization(
                        key,
                        ResidentA2aDelegateOutcomeKind::OutcomeUnknown,
                        ResidentA2aDelegateSettlementKind::Completion,
                        settlement,
                    ),
                })
            }
            ResidentA2aDelegateImmediateOutcome::Diagnostic(failure) => {
                ResidentA2aDelegatePoll::Diagnostic(ResidentA2aDelegateDiagnostic {
                    failure,
                    finalization: ResidentA2aDelegatePendingFinalization {
                        owner_instance: self.owner_instance.clone(),
                        key,
                        settlement,
                    },
                })
            }
            ResidentA2aDelegateImmediateOutcome::NotStarted(response) => {
                ResidentA2aDelegatePoll::Completed(ResidentA2aDelegateCompletion::NotStarted {
                    response,
                    finalization: self.finalization(
                        key,
                        ResidentA2aDelegateOutcomeKind::NotStarted,
                        ResidentA2aDelegateSettlementKind::Completion,
                        settlement,
                    ),
                })
            }
        }
    }

    /// Retry only diagnostic construction for an exact closed carrier branch.
    ///
    /// The pending aggregate settlement is checked before the diagnostic owner
    /// is consumed. Recovery performs no carrier, clock, storage, signing, or
    /// anchor I/O. Capacity stays held until the recovered disposition is
    /// durably recorded and passed to [`Self::finish`].
    pub(in crate::resident) fn recover_diagnostic(
        &self,
        diagnostic: ResidentA2aDelegateDiagnostic,
        reservations: &ResidentAggregateTaskReservations,
    ) -> Result<ResidentA2aDelegateCompletion, ResidentA2aDelegateDiagnosticRecoveryFailure> {
        let reservation = if self
            .owner_instance
            .is_same(&diagnostic.finalization.owner_instance)
        {
            reservations.require_a2a_completion(
                &diagnostic.finalization.settlement,
                &diagnostic.finalization.key,
            )
        } else {
            Err(ResidentTaskReservationTransitionFailure::OwnerMismatch)
        };
        if let Err(reason) = reservation {
            return Err(ResidentA2aDelegateDiagnosticRecoveryFailure::Reservation {
                diagnostic: Box::new(diagnostic),
                reason,
            });
        }
        let ResidentA2aDelegateDiagnostic {
            failure,
            finalization,
        } = diagnostic;
        let recovered = match failure.retry_diagnostic() {
            Ok(recovered) => recovered,
            Err(failure) => {
                return Err(
                    ResidentA2aDelegateDiagnosticRecoveryFailure::StillInadmissible(Box::new(
                        ResidentA2aDelegateDiagnostic {
                            failure,
                            finalization,
                        },
                    )),
                );
            }
        };
        let ResidentA2aDelegatePendingFinalization {
            owner_instance,
            key,
            settlement,
        } = finalization;
        match recovered {
            RecoveredA2aSendDiagnostic::NotStarted(response) => {
                Ok(ResidentA2aDelegateCompletion::NotStarted {
                    response,
                    finalization: ResidentA2aDelegateFinalization {
                        owner_instance,
                        key,
                        expected: ResidentA2aDelegateOutcomeKind::NotStarted,
                        settlement_kind: ResidentA2aDelegateSettlementKind::Completion,
                        settlement,
                    },
                })
            }
            RecoveredA2aSendDiagnostic::NoTask(response) => {
                Ok(ResidentA2aDelegateCompletion::NoTask {
                    response,
                    finalization: ResidentA2aDelegateFinalization {
                        owner_instance,
                        key,
                        expected: ResidentA2aDelegateOutcomeKind::NoTask,
                        settlement_kind: ResidentA2aDelegateSettlementKind::Completion,
                        settlement,
                    },
                })
            }
            RecoveredA2aSendDiagnostic::OutcomeUnknown(outcome) => {
                Ok(ResidentA2aDelegateCompletion::OutcomeUnknown {
                    outcome,
                    finalization: ResidentA2aDelegateFinalization {
                        owner_instance,
                        key,
                        expected: ResidentA2aDelegateOutcomeKind::OutcomeUnknown,
                        settlement_kind: ResidentA2aDelegateSettlementKind::Completion,
                        settlement,
                    },
                })
            }
        }
    }

    /// Recover a retained send as explicit uncertainty without a carrier call.
    pub(in crate::resident) fn recover<'a>(
        &self,
        input: A2aSendRecoveryInput<'a>,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        admission: A2aSendRecoveryAdmission<'_>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentRecoveredA2aDelegate<'a>, ResidentA2aDelegateRecoveryFailure<'a>> {
        let key = task_key_from_recovered(input.recovered)
            .map_err(ResidentA2aDelegateRecoveryFailure::Association)?;
        let offer = reservations
            .begin_offer(key.clone(), ResidentTaskReservationRoute::A2aDelegate)
            .map_err(ResidentA2aDelegateRecoveryFailure::Reservation)?;
        let outcome = match recover_a2a_send_outcome_unknown(input, custody, anchor, admission) {
            Ok(outcome) => outcome,
            Err(reason) => {
                return match reservations.refuse_offer(offer) {
                    Ok(()) => Err(ResidentA2aDelegateRecoveryFailure::Recovery(reason)),
                    Err(reservation) => Err(ResidentA2aDelegateRecoveryFailure::Rollback {
                        recovery: reason,
                        reservation,
                    }),
                };
            }
        };
        let settlement = match reservations.lose_offer(offer) {
            Ok(settlement) => settlement,
            Err(failure) => {
                return Err(ResidentA2aDelegateRecoveryFailure::LossReservation(
                    Box::new(ResidentA2aDelegateRecoveredOfferRejection {
                        recovered: Box::new(ResidentA2aDelegateRecoveredOffer {
                            owner_instance: self.owner_instance.clone(),
                            key,
                            outcome: Box::new(outcome),
                            offer: *failure.offer,
                        }),
                        reason: failure.reason,
                    }),
                ));
            }
        };
        Ok(ResidentRecoveredA2aDelegate {
            outcome: Box::new(outcome),
            finalization: self.finalization(
                key,
                ResidentA2aDelegateOutcomeKind::OutcomeUnknown,
                ResidentA2aDelegateSettlementKind::Loss,
                settlement,
            ),
        })
    }

    /// Retry only returning an aggregate offer after send recovery was rejected.
    ///
    /// This performs no recovery, custody, anchor, clock, signing, or carrier I/O.
    pub(in crate::resident) fn retry_recovery_rollback<'a>(
        &self,
        failure: ResidentA2aDelegateRecoveryFailure<'a>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<A2aSendRecoveryError, ResidentA2aDelegateRecoveryFailure<'a>> {
        let ResidentA2aDelegateRecoveryFailure::Rollback {
            recovery,
            reservation,
        } = failure
        else {
            return Err(failure);
        };
        match reservations.retry_refuse_offer(reservation) {
            Ok(()) => Ok(recovery),
            Err(reservation) => Err(ResidentA2aDelegateRecoveryFailure::Rollback {
                recovery,
                reservation,
            }),
        }
    }

    /// Release shared capacity only for the exact durably retained disposition.
    pub(in crate::resident) fn finish(
        &self,
        finalization: ResidentA2aDelegateFinalization,
        retained: RetainedA2aOutcome,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<RetainedA2aOutcome, ResidentA2aDelegateFinalizationRejection> {
        if !self.owner_instance.is_same(&finalization.owner_instance) {
            return Err(ResidentA2aDelegateFinalizationRejection {
                finalization: Box::new(finalization),
                retained: Box::new(retained),
                reason: ResidentA2aDelegateFinalizationFailure::Reservation(
                    ResidentTaskReservationTransitionFailure::OwnerMismatch,
                ),
            });
        }
        let reservation = match finalization.settlement_kind {
            ResidentA2aDelegateSettlementKind::Completion => {
                reservations.require_a2a_completion(&finalization.settlement, &finalization.key)
            }
            ResidentA2aDelegateSettlementKind::Loss => {
                reservations.require_a2a_loss(&finalization.settlement, &finalization.key)
            }
        };
        if let Err(reason) = reservation {
            return Err(ResidentA2aDelegateFinalizationRejection {
                finalization: Box::new(finalization),
                retained: Box::new(retained),
                reason: ResidentA2aDelegateFinalizationFailure::Reservation(reason),
            });
        }
        if !retained_matches(&retained, finalization.expected, &finalization.key) {
            return Err(ResidentA2aDelegateFinalizationRejection {
                finalization: Box::new(finalization),
                retained: Box::new(retained),
                reason: ResidentA2aDelegateFinalizationFailure::Association,
            });
        }
        let release = match finalization.settlement_kind {
            ResidentA2aDelegateSettlementKind::Completion => {
                reservations.release_completion(finalization.settlement)
            }
            ResidentA2aDelegateSettlementKind::Loss => {
                reservations.release_loss(finalization.settlement)
            }
        };
        match release {
            Ok(()) => Ok(retained),
            Err(failure) => Err(ResidentA2aDelegateFinalizationRejection {
                finalization: Box::new(ResidentA2aDelegateFinalization {
                    owner_instance: finalization.owner_instance,
                    key: finalization.key,
                    expected: finalization.expected,
                    settlement_kind: finalization.settlement_kind,
                    settlement: *failure.settlement,
                }),
                retained: Box::new(retained),
                reason: ResidentA2aDelegateFinalizationFailure::Reservation(failure.reason),
            }),
        }
    }

    fn finalization(
        &self,
        key: ResidentOperationTaskKey,
        expected: ResidentA2aDelegateOutcomeKind,
        settlement_kind: ResidentA2aDelegateSettlementKind,
        settlement: ResidentTaskReservationSettlement,
    ) -> ResidentA2aDelegateFinalization {
        ResidentA2aDelegateFinalization {
            owner_instance: self.owner_instance.clone(),
            key,
            expected,
            settlement_kind,
            settlement,
        }
    }
}

/// Result of launch after the carrier either remained pending or closed locally.
#[must_use = "retain the A2A launch state"]
pub(in crate::resident) enum ResidentA2aDelegateLaunch {
    /// One carrier exchange remains under process-local ownership.
    Awaiting(ResidentOperationTaskKey),
    /// A typed terminal carrier branch awaits durable custody.
    Completed(ResidentA2aDelegateCompletion),
    /// Diagnostic admission failed after the branch became non-replayable.
    Diagnostic(ResidentA2aDelegateDiagnostic),
}

/// A2A launch failure that never discards accepted carrier uncertainty.
#[must_use = "retain any returned A2A custody or settlement capability"]
pub(in crate::resident) enum ResidentA2aDelegateLaunchFailure<'operation, 'call> {
    /// The input could not form its exact store-qualified task key.
    Association {
        input: Box<A2aDelegateLaunchInput<'operation, 'call>>,
        reason: OperationAttemptError,
    },
    /// Shared capacity or duplicate detection rejected the unopened input.
    Reservation {
        input: Box<A2aDelegateLaunchInput<'operation, 'call>>,
        reason: ResidentTaskReservationOfferFailure,
    },
    /// Launch failed before request custody and the aggregate offer was returned.
    Launch(Box<A2aDelegateLaunchFailure<'operation, 'call>>),
    /// Returning a definitely unopened offer failed without losing either value.
    Rollback {
        launch: Box<A2aDelegateLaunchFailure<'operation, 'call>>,
        reservation: ResidentTaskReservationOfferTransitionFailure,
    },
    /// Request custody may exist or remain uncertain; the exact offer stays held.
    Custody(Box<ResidentA2aDelegateUnresolvedLaunch<'operation, 'call>>),
    /// Anchored request custody exists, but the carrier has definitely not started.
    Unopened(Box<ResidentA2aDelegateUnopenedLaunch<'operation, 'call>>),
    /// The started exchange did not rejoin the exact dispatch key.
    StartedAssociation {
        completion: Box<ResidentA2aDelegateOfferedCompletion>,
        reason: OperationAttemptError,
    },
    /// A closed launch branch could not enter durable-handling occupancy.
    OfferSettlement(Box<ResidentA2aDelegateOfferedCompletionRejection>),
}

/// Launch phase that may own unresolved request custody and aggregate capacity.
#[must_use = "reconcile the exact A2A custody before releasing its reservation"]
pub(in crate::resident) struct ResidentA2aDelegateUnresolvedLaunch<'operation, 'call> {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: ResidentOperationTaskKey,
    custody: Box<A2aSendCustodyFailure<'operation, 'call>>,
    current: &'call AmbientSessionAdmissionContext<'call>,
    response_ref: ResidentRecordRefV1,
    offer: super::task_reservation::ResidentTaskReservationOffer,
}

/// Anchored request custody whose carrier has provably never started.
#[must_use = "retry this exact unopened A2A send or retain its aggregate offer"]
pub(in crate::resident) struct ResidentA2aDelegateUnopenedLaunch<'operation, 'call> {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: ResidentOperationTaskKey,
    retained: RetainedA2aSend<'operation, 'call>,
    current: &'call AmbientSessionAdmissionContext<'call>,
    response_ref: ResidentRecordRefV1,
    offer: super::task_reservation::ResidentTaskReservationOffer,
    reason: ResidentA2aDelegateUnopenedReason,
}

/// Last proven reason why an unopened send did not reach the carrier.
#[derive(Debug)]
pub(in crate::resident) enum ResidentA2aDelegateUnopenedReason {
    /// Exact custody completed without starting the carrier.
    CustodyResolved,
    /// The release time could not be sampled in order.
    Clock(OperationAttemptError),
    /// Fresh authority, association, storage, anchor, or retention checks refused release.
    Preflight(super::A2aSendReleaseError),
}

/// Exact custody continuation refused without discarding either linear owner.
#[must_use = "retain the unresolved A2A custody and its aggregate offer"]
pub(in crate::resident) struct ResidentA2aDelegateCustodyContinuationFailure<'operation, 'call> {
    /// Complete unchanged continuation owner.
    pub(in crate::resident) unresolved: Box<ResidentA2aDelegateUnresolvedLaunch<'operation, 'call>>,
    /// Aggregate-capability mismatch, or `None` when physical custody remains unresolved.
    pub(in crate::resident) reservation: Option<ResidentTaskReservationTransitionFailure>,
}

/// Protected recovery failed without restoring carrier-release permission.
#[must_use = "retain the exact recovery or custody capability; never replay SendMessage"]
pub(in crate::resident) enum ResidentA2aDelegateUnresolvedRecoveryFailure<
    'operation,
    'call,
    'recovery,
> {
    /// The aggregate offer differs in owner, key, route, or phase. No recovery ran.
    Reservation {
        unresolved: Box<ResidentA2aDelegateUnresolvedLaunch<'operation, 'call>>,
        reason: ResidentTaskReservationTransitionFailure,
    },
    /// Exact custody has not yet proved a recovery-only committed state.
    Custody(Box<ResidentA2aDelegateUnresolvedLaunch<'operation, 'call>>),
    /// The supplied protected recovery names another dispatch.
    Association {
        unresolved: Box<ResidentA2aDelegateUnresolvedLaunch<'operation, 'call>>,
        reason: OperationAttemptError,
    },
    /// Existing protected recovery refused the historical evidence.
    Recovery {
        unresolved: Box<ResidentA2aDelegateUnresolvedLaunch<'operation, 'call>>,
        reason: A2aSendRecoveryError,
    },
    /// Recovery succeeded, but the exact aggregate offer did not enter loss handling.
    LossReservation(Box<ResidentA2aDelegateRecoveredOfferRejection<'recovery>>),
}

/// Recovered uncertainty whose exact aggregate offer has not entered loss handling.
#[must_use = "retry only the exact aggregate transition; never repeat A2A recovery"]
pub(in crate::resident) struct ResidentA2aDelegateRecoveredOffer<'a> {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: ResidentOperationTaskKey,
    outcome: Box<RecoveredA2aSendOutcomeUnknown<'a>>,
    offer: super::task_reservation::ResidentTaskReservationOffer,
}

/// Rejected recovered-loss transition retaining the outcome and linear offer.
#[must_use = "retain and retry the exact recovered loss offer"]
pub(in crate::resident) struct ResidentA2aDelegateRecoveredOfferRejection<'a> {
    recovered: Box<ResidentA2aDelegateRecoveredOffer<'a>>,
    /// Exact aggregate transition refusal.
    pub(in crate::resident) reason: ResidentTaskReservationTransitionFailure,
}

/// Exact unopened retry refused before or after its one allowed carrier start.
#[must_use = "retain every returned A2A capability; never infer replay permission"]
pub(in crate::resident) enum ResidentA2aDelegateUnopenedRetryFailure<'operation, 'call> {
    /// The aggregate offer differs in owner, key, route, or phase. No I/O occurred.
    Reservation {
        unopened: Box<ResidentA2aDelegateUnopenedLaunch<'operation, 'call>>,
        reason: ResidentTaskReservationTransitionFailure,
    },
    /// The carrier is still unopened and the complete retry owner remains available.
    Unopened(Box<ResidentA2aDelegateUnopenedLaunch<'operation, 'call>>),
    /// The carrier started or closed; this existing launch failure never restores replay.
    Closed(Box<ResidentA2aDelegateLaunchFailure<'operation, 'call>>),
}

/// Closed launch outcome whose aggregate offer has not yet settled.
#[must_use = "retry this exact offered completion without replaying SendMessage"]
pub(in crate::resident) struct ResidentA2aDelegateOfferedCompletion {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: ResidentOperationTaskKey,
    outcome: ResidentA2aDelegateImmediateOutcome,
    offer: super::task_reservation::ResidentTaskReservationOffer,
}

/// Rejected offered-completion transition retaining its exact linear state.
#[must_use = "recover and retry the unchanged offered A2A completion"]
pub(in crate::resident) struct ResidentA2aDelegateOfferedCompletionRejection {
    completion: Box<ResidentA2aDelegateOfferedCompletion>,
    /// Exact aggregate transition refusal.
    pub(in crate::resident) reason: ResidentTaskReservationTransitionFailure,
}

/// One non-blocking poll result for an accepted A2A delegate.
#[must_use = "retain or continue the exact A2A poll result"]
pub(in crate::resident) enum ResidentA2aDelegatePoll {
    /// No terminal carrier item is ready.
    Pending,
    /// A typed carrier disposition awaits durable custody.
    Completed(ResidentA2aDelegateCompletion),
    /// Diagnostic admission failed after the exchange became non-replayable.
    Diagnostic(ResidentA2aDelegateDiagnostic),
    /// This resident lifetime does not own the supplied task key.
    NotFound,
    /// The closed exchange retains its outcome after an aggregate refusal.
    SettlementRejected(Box<ResidentA2aDelegateSettlementRejection>),
}

/// Closed carrier outcome whose accepted reservation has not yet transitioned.
#[must_use = "retry the exact aggregate transition without polling or replaying"]
pub(in crate::resident) struct ResidentA2aDelegateUnsettledCompletion {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: ResidentOperationTaskKey,
    outcome: ResidentA2aDelegateImmediateOutcome,
}

/// Rejected aggregate transition retaining the complete closed carrier outcome.
#[must_use = "recover and retry the unchanged A2A settlement"]
pub(in crate::resident) struct ResidentA2aDelegateSettlementRejection {
    settlement: Box<ResidentA2aDelegateUnsettledCompletion>,
    /// Exact aggregate transition refusal.
    pub(in crate::resident) reason: ResidentTaskReservationTransitionFailure,
}

impl ResidentA2aDelegateSettlementRejection {
    /// Borrow the exact store-qualified task whose transition was rejected.
    #[must_use]
    pub(in crate::resident) fn key(&self) -> &ResidentOperationTaskKey {
        &self.settlement.key
    }
}

/// Exact carrier disposition awaiting durable outcome custody.
#[must_use = "durably retain the A2A disposition before releasing capacity"]
pub(in crate::resident) enum ResidentA2aDelegateCompletion {
    /// The response identified an actual remote Task.
    Task {
        response: Box<CompletedA2aTaskResponse>,
        finalization: ResidentA2aDelegateFinalization,
    },
    /// The response was valid but created no Task binding.
    NoTask {
        response: Box<CompletedA2aNoTaskResponse>,
        finalization: ResidentA2aDelegateFinalization,
    },
    /// The carrier proved rejection before remote queue acceptance.
    NotStarted {
        response: Box<super::A2aSendNotStarted>,
        finalization: ResidentA2aDelegateFinalization,
    },
    /// Remote acceptance or outcome cannot be known locally.
    OutcomeUnknown {
        outcome: Box<super::A2aSendOutcomeUnknown>,
        finalization: ResidentA2aDelegateFinalization,
    },
}

/// Diagnostic-construction failure after the carrier branch became non-replayable.
#[must_use = "preserve the pending A2A settlement until diagnostic recovery exists"]
pub(in crate::resident) struct ResidentA2aDelegateDiagnostic {
    /// Diagnostic-admission failure retaining all carrier evidence.
    pub(in crate::resident) failure: Box<super::A2aSendDiagnosticFailure>,
    /// Aggregate capacity that deliberately cannot be released yet.
    pub(in crate::resident) finalization: ResidentA2aDelegatePendingFinalization,
}

/// Pending reservation released only after diagnostic recovery and durable custody.
#[must_use = "this capacity remains held until the diagnostic can be durably recorded"]
pub(in crate::resident) struct ResidentA2aDelegatePendingFinalization {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: ResidentOperationTaskKey,
    settlement: ResidentTaskReservationSettlement,
}

/// Diagnostic recovery refused without releasing or replacing aggregate custody.
#[must_use = "retain the exact diagnostic and pending settlement for a later retry"]
pub(in crate::resident) enum ResidentA2aDelegateDiagnosticRecoveryFailure {
    /// The capability differs in owner, key, route, or phase. No recovery ran.
    Reservation {
        diagnostic: Box<ResidentA2aDelegateDiagnostic>,
        reason: ResidentTaskReservationTransitionFailure,
    },
    /// The same retained evidence still cannot form its required diagnostic.
    StillInadmissible(Box<ResidentA2aDelegateDiagnostic>),
}

/// Restart-recovered send that can only become durable outcome-unknown custody.
#[must_use = "retain the recovered A2A uncertainty before releasing capacity"]
pub(in crate::resident) struct ResidentRecoveredA2aDelegate<'a> {
    /// Restart-recovered send with no carrier or replay capability.
    pub(in crate::resident) outcome: Box<RecoveredA2aSendOutcomeUnknown<'a>>,
    /// Exact loss reservation released only after outcome-unknown custody.
    pub(in crate::resident) finalization: ResidentA2aDelegateFinalization,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResidentA2aDelegateOutcomeKind {
    Task,
    NoTask,
    NotStarted,
    OutcomeUnknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResidentA2aDelegateSettlementKind {
    Completion,
    Loss,
}

/// Exact linear release capability for one durable A2A disposition.
#[must_use = "finish exact A2A outcome custody before releasing capacity"]
pub(in crate::resident) struct ResidentA2aDelegateFinalization {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: ResidentOperationTaskKey,
    expected: ResidentA2aDelegateOutcomeKind,
    settlement_kind: ResidentA2aDelegateSettlementKind,
    settlement: ResidentTaskReservationSettlement,
}

impl ResidentA2aDelegateFinalization {
    pub(in crate::resident) fn matches_process_retention(
        &self,
        owner_instance: &ResidentOperationTaskOwnerInstance,
        key: &ResidentOperationTaskKey,
        kind: A2aOutcomeRetentionKind,
    ) -> bool {
        self.owner_instance.is_same(owner_instance)
            && &self.key == key
            && matches!(
                (self.expected, kind),
                (
                    ResidentA2aDelegateOutcomeKind::Task,
                    A2aOutcomeRetentionKind::Task
                ) | (
                    ResidentA2aDelegateOutcomeKind::NoTask,
                    A2aOutcomeRetentionKind::NoTask
                ) | (
                    ResidentA2aDelegateOutcomeKind::NotStarted,
                    A2aOutcomeRetentionKind::NotStarted
                ) | (
                    ResidentA2aDelegateOutcomeKind::OutcomeUnknown,
                    A2aOutcomeRetentionKind::OutcomeUnknown
                )
            )
    }
}

/// Rejected A2A release retaining both the custody result and linear capability.
#[must_use = "correct the association and retry without replaying SendMessage"]
pub(in crate::resident) struct ResidentA2aDelegateFinalizationRejection {
    /// Unchanged aggregate release capability.
    pub(in crate::resident) finalization: Box<ResidentA2aDelegateFinalization>,
    /// Exact durable disposition rejected by the association check.
    pub(in crate::resident) retained: Box<RetainedA2aOutcome>,
    /// Closed association or aggregate-capability refusal.
    pub(in crate::resident) reason: ResidentA2aDelegateFinalizationFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentA2aDelegateFinalizationFailure {
    /// Durable custody belongs to another dispatch or terminal branch.
    Association,
    /// The aggregate capability differs in owner, key, route, or phase.
    Reservation(ResidentTaskReservationTransitionFailure),
}

/// Recovery failure that never grants another `SendMessage` attempt.
#[must_use = "A2A recovery failure never grants replay authority"]
pub(in crate::resident) enum ResidentA2aDelegateRecoveryFailure<'a> {
    /// The recovered head could not form the selected task identity.
    Association(OperationAttemptError),
    /// Shared capacity or duplicate detection rejected recovery ownership.
    Reservation(ResidentTaskReservationOfferFailure),
    /// Protected historical recovery did not admit the retained send.
    Recovery(A2aSendRecoveryError),
    /// Recovery failed and the unopened aggregate offer could not be returned.
    Rollback {
        recovery: A2aSendRecoveryError,
        reservation: ResidentTaskReservationOfferTransitionFailure,
    },
    /// Recovered uncertainty could not enter loss-handling occupancy.
    LossReservation(Box<ResidentA2aDelegateRecoveredOfferRejection<'a>>),
}

fn response_failure_outcome(
    failure: A2aSendResponseFailure,
) -> ResidentA2aDelegateImmediateOutcome {
    match failure {
        A2aSendResponseFailure::OutcomeUnknown(outcome) => {
            ResidentA2aDelegateImmediateOutcome::OutcomeUnknown(outcome)
        }
        A2aSendResponseFailure::Diagnostic(failure) => {
            ResidentA2aDelegateImmediateOutcome::Diagnostic(failure)
        }
    }
}

pub(in crate::resident) enum ResidentA2aDelegateImmediateOutcome {
    /// Actual remote Task response.
    Task(Box<CompletedA2aTaskResponse>),
    /// Valid response with no Task identity.
    NoTask(Box<CompletedA2aNoTaskResponse>),
    /// Proven rejection before queue acceptance.
    NotStarted(Box<super::A2aSendNotStarted>),
    /// Accepted-send effect whose remote outcome is unknown.
    OutcomeUnknown(Box<super::A2aSendOutcomeUnknown>),
    /// Failure to admit the required local diagnostic.
    Diagnostic(Box<super::A2aSendDiagnosticFailure>),
}

fn task_key_from_recovered(
    recovered: &super::RecoveredOperationObservation,
) -> Result<ResidentOperationTaskKey, OperationAttemptError> {
    let (record_ref, observation) = recovered.selected();
    let record_sha256 = ArtifactDigest::parse(
        observation
            .canonical_operation_observation_sha256()
            .as_str(),
    )
    .map_err(|_| OperationAttemptError::Encoding)?;
    Ok(ResidentOperationTaskKey::new(
        recovered.store().clone(),
        ImmutableRecordReferenceV1 {
            record_ref: record_ref.clone(),
            record_sha256,
        },
    ))
}

fn retained_matches(
    retained: &RetainedA2aOutcome,
    expected: ResidentA2aDelegateOutcomeKind,
    key: &ResidentOperationTaskKey,
) -> bool {
    let kind = match expected {
        ResidentA2aDelegateOutcomeKind::Task => A2aOutcomeRetentionKind::Task,
        ResidentA2aDelegateOutcomeKind::NoTask => A2aOutcomeRetentionKind::NoTask,
        ResidentA2aDelegateOutcomeKind::NotStarted => A2aOutcomeRetentionKind::NotStarted,
        ResidentA2aDelegateOutcomeKind::OutcomeUnknown => A2aOutcomeRetentionKind::OutcomeUnknown,
    };
    retained.matches_process_association(key, kind)
}

#[cfg(test)]
#[path = "../../tests/internal/resident/a2a_delegate_task_service.rs"]
mod tests;
