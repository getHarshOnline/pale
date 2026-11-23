//! Focused test support for the process-local A2A delegate owner.

use super::*;
use crate::resident::task_reservation::ResidentTaskReservationPhase;
use pale_ports::{
    A2aExchangeError, A2aExchangeFacts, AuthorityStoreCoordinate, WallClockSourceError,
};
use std::sync::atomic::{AtomicUsize, Ordering};

impl ResidentA2aDelegateTaskService {
    /// Build the opaque owner token that a replacement process instance would issue.
    pub(in crate::resident) fn replacement_dispatch_owner_for_test(
        &self,
        key: ResidentOperationTaskKey,
    ) -> ResidentA2aDelegateDispatchOwner {
        ResidentA2aDelegateDispatchOwner {
            owner_instance: self.owner_instance.clone(),
            key,
        }
    }

    pub(in crate::resident) fn active_send_checkpoint_for_test(
        &self,
        key: &ResidentOperationTaskKey,
    ) -> Option<(ResidentRecordRefV1, ControlStateCheckpointV1)> {
        self.active
            .iter()
            .find(|active| &active.key == key)
            .map(|active| {
                let checkpoint = active.awaiting.send_checkpoint();
                (checkpoint.0.clone(), checkpoint.1.clone())
            })
    }

    pub(in crate::resident) fn started_association_failure_for_test(
        &mut self,
        key: &ResidentOperationTaskKey,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aDelegateLaunchFailure<'static, 'static> {
        let completion = self.cancel(key, reservations);
        let ResidentA2aDelegatePoll::Completed(ResidentA2aDelegateCompletion::OutcomeUnknown {
            outcome,
            finalization: _,
        }) = completion
        else {
            panic!("the active test exchange must cancel as outcome unknown")
        };
        reservations.set_phase_for_test(
            key,
            &ResidentTaskReservationRoute::A2aDelegate,
            ResidentTaskReservationPhase::Offered,
        );
        ResidentA2aDelegateLaunchFailure::StartedAssociation {
            completion: Box::new(ResidentA2aDelegateOfferedCompletion {
                owner_instance: self.owner_instance.clone(),
                key: key.clone(),
                outcome: ResidentA2aDelegateImmediateOutcome::OutcomeUnknown(outcome),
                offer: reservations.a2a_offer_for_test(key),
            }),
            reason: OperationAttemptError::Association,
        }
    }

    pub(in crate::resident) fn offered_completion_failure_for_test(
        &mut self,
        key: &ResidentOperationTaskKey,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aDelegateLaunchFailure<'static, 'static> {
        let completion = self.cancel(key, reservations);
        let ResidentA2aDelegatePoll::Completed(ResidentA2aDelegateCompletion::OutcomeUnknown {
            outcome,
            finalization: _,
        }) = completion
        else {
            panic!("the active test exchange must cancel as outcome unknown")
        };
        reservations.set_phase_for_test(
            key,
            &ResidentTaskReservationRoute::A2aDelegate,
            ResidentTaskReservationPhase::Accepted,
        );
        let offer = reservations.a2a_offer_for_test(key);
        let failure = match reservations.complete_offer(offer) {
            Ok(_) => panic!("the substituted accepted phase must reject offered completion"),
            Err(failure) => failure,
        };
        reservations.set_phase_for_test(
            key,
            &ResidentTaskReservationRoute::A2aDelegate,
            ResidentTaskReservationPhase::Offered,
        );
        ResidentA2aDelegateLaunchFailure::OfferSettlement(Box::new(self.offered_rejection(
            key.clone(),
            ResidentA2aDelegateImmediateOutcome::OutcomeUnknown(outcome),
            failure,
        )))
    }

    pub(in crate::resident) fn rollback_failure_from_not_started_for_test(
        &self,
        completion: ResidentA2aDelegateCompletion,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> (
        ResidentOperationTaskKey,
        ResidentA2aDelegateLaunchFailure<'static, 'static>,
    ) {
        let ResidentA2aDelegateCompletion::NotStarted {
            response,
            finalization,
        } = completion
        else {
            panic!("the rollback fixture requires a real rejected carrier start")
        };
        let key = finalization.key.clone();
        reservations.set_phase_for_test(
            &key,
            &ResidentTaskReservationRoute::A2aDelegate,
            ResidentTaskReservationPhase::Accepted,
        );
        let offer = reservations.a2a_offer_for_test(&key);
        let reservation = match reservations.refuse_offer(offer) {
            Ok(_) => panic!("the substituted accepted phase must reject offer refusal"),
            Err(failure) => failure,
        };
        reservations.set_phase_for_test(
            &key,
            &ResidentTaskReservationRoute::A2aDelegate,
            ResidentTaskReservationPhase::Offered,
        );
        (
            key,
            ResidentA2aDelegateLaunchFailure::Rollback {
                launch: Box::new(A2aDelegateLaunchFailure::Start(
                    A2aSendStartFailure::Rejected(response),
                )),
                reservation,
            },
        )
    }

    /// Complete protected recovery, then model a hostile phase substitution at
    /// the aggregate boundary. This preserves the real recovered outcome and
    /// exact linear offer without adding a fault switch to production state.
    pub(in crate::resident) fn recover_unresolved_with_rejected_loss_for_test<
        'operation,
        'call,
        'a,
    >(
        &self,
        unresolved: ResidentA2aDelegateUnresolvedLaunch<'operation, 'call>,
        input: A2aSendRecoveryInput<'a>,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        admission: A2aSendRecoveryAdmission<'_>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aDelegateRecoveredOfferRejection<'a> {
        reservations
            .require_a2a_offer(&unresolved.offer, &unresolved.key)
            .expect("the fixture must retain the exact unopened offer");
        assert!(unresolved.custody.is_recovery_only());
        let recovered_key = task_key_from_recovered(input.recovered)
            .expect("the fixture must carry an admissible recovered task key");
        assert_eq!(recovered_key, unresolved.key);
        let outcome = recover_a2a_send_outcome_unknown(input, custody, anchor, admission)
            .unwrap_or_else(|reason| panic!("protected recovery must succeed: {reason:?}"));
        let ResidentA2aDelegateUnresolvedLaunch {
            owner_instance,
            key,
            custody: _,
            current: _,
            response_ref: _,
            offer,
        } = unresolved;
        reservations.set_phase_for_test(
            &key,
            &ResidentTaskReservationRoute::A2aDelegate,
            super::super::task_reservation::ResidentTaskReservationPhase::Accepted,
        );
        let failure = match reservations.lose_offer(offer) {
            Ok(_) => panic!("the hostile phase substitution must reject loss transition"),
            Err(failure) => failure,
        };
        ResidentA2aDelegateRecoveredOfferRejection {
            recovered: Box::new(ResidentA2aDelegateRecoveredOffer {
                owner_instance,
                key,
                outcome: Box::new(outcome),
                offer: *failure.offer,
            }),
            reason: failure.reason,
        }
    }

    /// Complete restart recovery, then model the same hostile phase
    /// substitution before the recovered offer enters loss handling.
    pub(in crate::resident) fn recover_with_rejected_loss_for_test<'a>(
        &self,
        input: A2aSendRecoveryInput<'a>,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        admission: A2aSendRecoveryAdmission<'_>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentA2aDelegateRecoveredOfferRejection<'a> {
        let key = task_key_from_recovered(input.recovered)
            .expect("the fixture must carry an admissible recovered task key");
        let offer = reservations
            .begin_offer(key.clone(), ResidentTaskReservationRoute::A2aDelegate)
            .expect("the fixture must reserve the recovered offer");
        let outcome = recover_a2a_send_outcome_unknown(input, custody, anchor, admission)
            .unwrap_or_else(|reason| panic!("protected recovery must succeed: {reason:?}"));
        reservations.set_phase_for_test(
            &key,
            &ResidentTaskReservationRoute::A2aDelegate,
            super::super::task_reservation::ResidentTaskReservationPhase::Accepted,
        );
        let failure = match reservations.lose_offer(offer) {
            Ok(_) => panic!("the hostile phase substitution must reject loss transition"),
            Err(failure) => failure,
        };
        ResidentA2aDelegateRecoveredOfferRejection {
            recovered: Box::new(ResidentA2aDelegateRecoveredOffer {
                owner_instance: self.owner_instance.clone(),
                key,
                outcome: Box::new(outcome),
                offer: *failure.offer,
            }),
            reason: failure.reason,
        }
    }
}

impl<'operation> ResidentA2aDelegateUnresolvedLaunch<'operation, '_> {
    pub(in crate::resident) fn recovery_fixture(
        &self,
    ) -> Option<super::super::operation_dispatch::test_support::A2aSendRecoveryFixture<'operation>>
    {
        self.custody.recovery_fixture()
    }
}

impl ResidentA2aDelegateDiagnostic {
    /// Preserve one real uncertain completion behind a modeled diagnostic fault.
    pub(in crate::resident) fn fixture_from_outcome_unknown(
        outcome: Box<super::super::A2aSendOutcomeUnknown>,
        finalization: ResidentA2aDelegateFinalization,
    ) -> Self {
        let ResidentA2aDelegateFinalization {
            owner_instance,
            key,
            expected,
            settlement_kind,
            settlement,
        } = finalization;
        assert_eq!(expected, ResidentA2aDelegateOutcomeKind::OutcomeUnknown);
        assert_eq!(
            settlement_kind,
            ResidentA2aDelegateSettlementKind::Completion
        );
        Self {
            failure: outcome.diagnostic_failure_fixture(),
            finalization: ResidentA2aDelegatePendingFinalization {
                owner_instance,
                key,
                settlement,
            },
        }
    }

    /// Preserve one real uncertain completion behind a diagnostic fault that
    /// remains inadmissible until the fixture is explicitly repaired.
    pub(in crate::resident) fn persistent_fixture_from_outcome_unknown(
        outcome: Box<super::super::A2aSendOutcomeUnknown>,
        finalization: ResidentA2aDelegateFinalization,
    ) -> Self {
        let ResidentA2aDelegateFinalization {
            owner_instance,
            key,
            expected,
            settlement_kind,
            settlement,
        } = finalization;
        assert_eq!(expected, ResidentA2aDelegateOutcomeKind::OutcomeUnknown);
        assert_eq!(
            settlement_kind,
            ResidentA2aDelegateSettlementKind::Completion
        );
        Self {
            failure: outcome.persistent_diagnostic_failure_fixture(),
            finalization: ResidentA2aDelegatePendingFinalization {
                owner_instance,
                key,
                settlement,
            },
        }
    }

    /// Restore the original uncertain-outcome classification after the test
    /// has proved that an inadmissible retry returns this complete owner.
    pub(in crate::resident) fn restore_outcome_unknown_fixture(mut self) -> Self {
        self.failure = self.failure.restore_outcome_unknown_fixture();
        self
    }
}

struct NoCallProtocol(AtomicUsize);

impl A2aSendMessageEvidencePort for NoCallProtocol {
    fn read_initial_response(
        &self,
        _: &[u8],
        _: &[u8],
        _: &A2aJsonRpcRequestIdV1,
        _: NonZeroUsize,
    ) -> Result<A2aExchangeFacts, A2aExchangeError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        panic!("a missing process-local task must not decode carrier evidence")
    }
}

struct NoCallClock(AtomicUsize);

impl WallClockSourcePort for NoCallClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        panic!("a missing process-local task must not sample time")
    }
}

fn task_key(name: &str) -> ResidentOperationTaskKey {
    ResidentOperationTaskKey::new(
        AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("store-a2a-task-service").unwrap(),
            ControlIncarnationIdV1::parse("incarnation-a2a-task-service").unwrap(),
        ),
        ImmutableRecordReferenceV1 {
            record_ref: ResidentRecordRefV1::parse(name).unwrap(),
            record_sha256: ArtifactDigest::hash_bytes(name.as_bytes()),
        },
    )
}

#[test]
fn unknown_poll_and_cancel_do_not_touch_carrier_clock_or_capacity() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let mut service =
        ResidentA2aDelegateTaskService::new_in(owner.clone(), NonZeroUsize::new(1).unwrap())
            .unwrap();
    let mut reservations = ResidentAggregateTaskReservations::new(
        owner,
        super::super::task_reservation::ResidentAggregateTaskLimits::new(
            NonZeroUsize::new(1).unwrap(),
        ),
    )
    .unwrap();
    let protocol = NoCallProtocol(AtomicUsize::new(0));
    let clock = NoCallClock(AtomicUsize::new(0));
    let waker = std::task::Waker::noop();
    let mut context = Context::from_waker(waker);
    let key = task_key("a2a/task-service/missing");

    assert!(matches!(
        service.poll(&key, &mut context, &protocol, &clock, &mut reservations),
        ResidentA2aDelegatePoll::NotFound
    ));
    assert!(matches!(
        service.cancel(&key, &mut reservations),
        ResidentA2aDelegatePoll::NotFound
    ));
    assert_eq!(protocol.0.load(Ordering::SeqCst), 0);
    assert_eq!(clock.0.load(Ordering::SeqCst), 0);
    assert_eq!(service.in_flight(), 0);
    assert_eq!(reservations.outstanding(), 0);
}
