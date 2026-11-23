//! Laws for one bounded resident Consult supervision turn.

use super::*;
use crate::local_operation_start_fixture::{
    checkpoint_preimage, issue_observation, record_ref, schema_digest,
};
use crate::resident::consultation::service::ConsultationChildDispatchReleaseFailure;
use crate::resident::consultation::tests::{
    consultation_supervision_fixture, consultation_supervision_fixture_with_child,
};
use crate::resident::consultation::ResidentConsultationTaskOwnerLimits;
use crate::resident::operation_observation_custody::tests::{
    admit_initial_checkpoint, local_start, signing_authority, OperationFixture, RecordingStore,
};
use crate::resident::operation_service::{ResidentOperationService, ResidentOperationTaskState};
use crate::resident::task_owner::{
    ResidentOperationTask, ResidentOperationTaskCompletion, ResidentOperationTaskExecutor,
    ResidentOperationTaskKey, ResidentOperationTaskLimits, ResidentOperationTaskOwnerInstance,
};
use crate::resident::task_reservation::{
    ResidentAggregateTaskLimits, ResidentAggregateTaskReservations,
};
use pale_core::records::{
    ArtifactDigest, CapabilityAttemptIdV1, ImmutableRecordReferenceV1, OperationActivePhaseV1,
    OperationObservationRevisionV1, OperationObservationStateV1, ResidentRecordRefV1,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{ControlRecordDigest, OperationCustodyCommitOutcome};
use std::{collections::VecDeque, num::NonZeroUsize};

#[derive(Default)]
struct CountingExecutor {
    accepts: usize,
    polls: usize,
    accepted: Vec<ResidentOperationTask<u8>>,
    completions: VecDeque<ResidentOperationTaskCompletion<u8, u16>>,
}

impl CountingExecutor {
    fn complete(&mut self, output: u16) {
        let task = self.accepted.remove(0);
        self.completions
            .push_back(ResidentOperationTaskCompletion::Completed { task, output });
    }
}

impl ResidentOperationTaskExecutor<u8, u16> for CountingExecutor {
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<u8>,
    ) -> Result<(), ResidentOperationTask<u8>> {
        self.accepts += 1;
        self.accepted.push(task);
        Ok(())
    }

    fn try_completion(&mut self) -> Option<ResidentOperationTaskCompletion<u8, u16>> {
        self.polls += 1;
        self.completions.pop_front()
    }
}

fn limits() -> ResidentConsultationTaskLimits {
    ResidentConsultationTaskLimits::new(
        NonZeroUsize::new(1).expect("task ceiling"),
        NonZeroUsize::new(2).expect("batch ceiling"),
    )
}

fn owners() -> (
    ResidentConsultationTaskOwner,
    ResidentAggregateTaskReservations,
) {
    let owner = ResidentOperationTaskOwnerInstance::new();
    (
        ResidentConsultationTaskOwner::new_in(
            owner.clone(),
            ResidentConsultationTaskOwnerLimits::new(
                NonZeroUsize::new(4).expect("batch-owner ceiling"),
            ),
        )
        .expect("consultation owner"),
        ResidentAggregateTaskReservations::new(
            owner,
            ResidentAggregateTaskLimits::new(NonZeroUsize::new(4).expect("aggregate ceiling")),
        )
        .expect("aggregate owner"),
    )
}

fn dispatch(store: &pale_ports::AuthorityStoreCoordinate, name: &str) -> ResidentOperationTaskKey {
    ResidentOperationTaskKey::new(
        store.clone(),
        ImmutableRecordReferenceV1 {
            record_ref: ResidentRecordRefV1::parse(format!("dispatch/{name}"))
                .expect("dispatch reference"),
            record_sha256: ArtifactDigest::hash_bytes(name.as_bytes()),
        },
    )
}

fn child_coordinate(operation: &OperationFixture) -> ImmutableRecordReferenceV1 {
    let (reference, handle) = operation.handle();
    ImmutableRecordReferenceV1 {
        record_ref: reference.clone(),
        record_sha256: ArtifactDigest::parse(handle.canonical_operation_handle_sha256().as_str())
            .expect("fixture handle digest"),
    }
}

fn recovered_child(
    operation: &OperationFixture,
    store: pale_ports::AuthorityStoreCoordinate,
    dispatched: bool,
) -> RecoveredOperationObservation {
    let (handle_ref, handle) = operation.handle();
    let selected = if dispatched {
        let reference = record_ref("operation/supervision-dispatched");
        let observation = issue_observation(
            &signing_authority(),
            handle_ref,
            handle,
            schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
            OperationObservationRevisionV1::new(3).expect("dispatch revision"),
            Some(operation.successor()),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched {
                    attempt_id: CapabilityAttemptIdV1::parse("supervision-attempt")
                        .expect("attempt identity"),
                    evidence: ImmutableRecordReferenceV1 {
                        record_ref: record_ref("attempt/supervision"),
                        record_sha256: ArtifactDigest::hash_bytes(b"supervision-attempt"),
                    },
                },
            },
            32,
        );
        (reference, observation)
    } else {
        (
            operation.successor().0.clone(),
            operation.successor().1.clone(),
        )
    };
    let handle = pale_ports::ControlRecordCoordinate::new(
        handle_ref.clone(),
        ControlRecordDigest::parse(handle.canonical_operation_handle_sha256().as_str())
            .expect("fixture handle coordinate"),
    );
    RecoveredOperationObservation::fixture_for_handle(
        handle,
        selected.clone(),
        selected,
        (
            record_ref("operation/supervision-checkpoint"),
            admit_initial_checkpoint(&checkpoint_preimage(1, None)),
        ),
        store,
    )
}

fn operation_task_state() -> ResidentOperationTaskState {
    ResidentOperationTaskState::new(
        ResidentOperationTaskLimits::new(NonZeroUsize::new(4).expect("provider ceiling")),
        NonZeroUsize::new(4).expect("Consult batch ceiling"),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(4).expect("aggregate ceiling")),
    )
    .expect("resident operation task state")
}

#[derive(Clone, Copy)]
enum FacadeOfferFailure {
    RecoverChild,
    RetentionUnopened,
    RetentionUncertain,
}

fn prove_facade_offer_failure_retains_coordinate(case: FacadeOfferFailure) {
    let operation = OperationFixture::exact();
    let current = consultation_supervision_fixture_with_child(
        "operation/facade-offer-recovery",
        "facade-offer-recovery",
        child_coordinate(&operation),
        100,
        false,
    );
    let planned = current
        .plan_child_dispatch(UnixTimeV1::new(2).expect("plan time"))
        .expect("one exact candidate");
    let expected_parent = planned.parent.clone();
    let expected_frontier = planned.collection_frontier.clone();
    let expected_store = planned.store().clone();
    let candidate = planned.candidates()[0].clone();
    let recovered = recovered_child(&operation, expected_store.clone(), false);
    let recovered = std::cell::RefCell::new(Some(recovered));
    let child_calls = std::cell::Cell::new(0);
    let retention_calls = std::cell::Cell::new(0);
    let mut executor = CountingExecutor::default();
    let mut authority = ();
    let mut custody = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let mut anchor = local_start::successor_anchor();
    let mut tasks = operation_task_state();

    let disposition = {
        let mut service =
            ResidentOperationService::new(&mut authority, &mut custody, &mut anchor, &mut tasks);
        service.supervise_consultation_turn(
            limits(),
            UnixTimeV1::new(2).expect("turn time"),
            || Ok::<_, &'static str>(current.clone()),
            &mut executor,
            |_, _| {
                ResidentConsultationSupervisionDecision::<()>::Continue(Some(Box::new(
                    candidate.clone(),
                )))
            },
            |_| {
                child_calls.set(child_calls.get() + 1);
                if matches!(case, FacadeOfferFailure::RecoverChild) {
                    Err("child-recovery-failed")
                } else {
                    Ok(recovered.borrow_mut().take().expect("one recovered child"))
                }
            },
            |_, _| {
                retention_calls.set(retention_calls.get() + 1);
                match case {
                    FacadeOfferFailure::RecoverChild => {
                        panic!("failed child recovery must precede retention")
                    }
                    FacadeOfferFailure::RetentionUnopened => {
                        ConsultationChildRetentionOutcome::<u8, &'static str>::Unopened(
                            "retention-unopened",
                        )
                    }
                    FacadeOfferFailure::RetentionUncertain => {
                        ConsultationChildRetentionOutcome::<u8, &'static str>::Uncertain(
                            "retention-uncertain",
                        )
                    }
                }
            },
        )
    };

    let ResidentConsultationTurnDisposition::NeedsRecovery(reason) = disposition else {
        panic!("offer failure must retain exact recovery coordinates")
    };
    let ResidentConsultationTurnNeedsRecovery::Offer(recovery) = *reason else {
        panic!("offer failure must use the exact offer recovery carrier")
    };
    let (coordinate, retained_candidate, failure) = recovery.into_parts();
    assert_eq!(coordinate.store(), &expected_store);
    assert_eq!(coordinate.parent(), &expected_parent);
    assert_eq!(coordinate.collection_frontier(), &expected_frontier);
    assert_eq!(retained_candidate, candidate);
    let ResidentConsultationTaskOwnerAcceptanceFailure::Acceptance(failure) = *failure else {
        panic!("façade offer failure must retain the batch acceptance reason")
    };
    match (case, *failure) {
        (
            FacadeOfferFailure::RecoverChild,
            ResidentConsultationTaskAcceptanceFailure::Release(
                ConsultationChildDispatchReleaseFailure::ChildLookup("child-recovery-failed"),
            ),
        )
        | (
            FacadeOfferFailure::RetentionUnopened,
            ResidentConsultationTaskAcceptanceFailure::RetentionUnopened {
                reason: "retention-unopened",
                ..
            },
        )
        | (
            FacadeOfferFailure::RetentionUncertain,
            ResidentConsultationTaskAcceptanceFailure::RetentionUncertain {
                reason: "retention-uncertain",
                ..
            },
        ) => {}
        (_, other) => panic!("unexpected façade offer failure: {other:?}"),
    }
    assert_eq!(executor.polls, 1);
    assert_eq!(executor.accepts, 0);

    if matches!(case, FacadeOfferFailure::RetentionUncertain) {
        let second_child_calls = std::cell::Cell::new(0);
        let second_retention_calls = std::cell::Cell::new(0);
        let second = {
            let mut service = ResidentOperationService::new(
                &mut authority,
                &mut custody,
                &mut anchor,
                &mut tasks,
            );
            service.supervise_consultation_turn(
                limits(),
                UnixTimeV1::new(3).expect("retry time"),
                || Ok::<_, &'static str>(current),
                &mut executor,
                |_, _| {
                    ResidentConsultationSupervisionDecision::<()>::Continue(Some(Box::new(
                        candidate.clone(),
                    )))
                },
                |_| -> Result<RecoveredOperationObservation, &'static str> {
                    second_child_calls.set(second_child_calls.get() + 1);
                    panic!("uncertain retention cannot recover and replay the child")
                },
                |_, _| -> ConsultationChildRetentionOutcome<u8, &'static str> {
                    second_retention_calls.set(second_retention_calls.get() + 1);
                    panic!("uncertain retention cannot repeat retention")
                },
            )
        };
        let ResidentConsultationTurnDisposition::NeedsRecovery(reason) = second else {
            panic!("uncertain retention must remain recovery-only")
        };
        let ResidentConsultationTurnNeedsRecovery::CandidateState {
            coordinate: retry_coordinate,
            candidate: retry_candidate,
            state: ResidentConsultationCandidateLocalState::DurableOnly,
        } = *reason
        else {
            panic!("uncertain retention must preserve its durable-only state")
        };
        assert_eq!(*retry_coordinate, coordinate);
        assert_eq!(*retry_candidate, candidate);
        assert_eq!(second_child_calls.get(), 0);
        assert_eq!(second_retention_calls.get(), 0);
        assert_eq!(executor.accepts, 0);
    }

    assert_eq!(child_calls.get(), 1);
    assert_eq!(
        retention_calls.get(),
        usize::from(!matches!(case, FacadeOfferFailure::RecoverChild))
    );
}

#[test]
fn pre_policy_turn_returns_ready_completion_without_a_policy_callback() {
    let (mut tasks, mut reservations) = owners();
    let current =
        consultation_supervision_fixture("operation/pre-policy-ready", "ready", 100, false);
    let planned = current
        .plan_child_dispatch(UnixTimeV1::new(2).expect("plan time"))
        .expect("one candidate");
    let candidate = planned.candidates()[0].clone();
    let coordinate = tasks
        .reconcile_batch(planned, limits(), &reservations)
        .expect("batch reconciliation")
        .usable()
        .expect("usable batch")
        .clone();
    let mut executor = CountingExecutor::default();
    tasks
        .seed_retained_for_supervision_test(
            &coordinate,
            candidate,
            dispatch(current.store(), "pre-policy-ready"),
            7,
            &mut reservations,
            &mut executor,
        )
        .expect("accepted child");
    executor.complete(70);

    let disposition: ResidentConsultationPrePolicyTurn<u8, u16, ()> =
        supervise_recovered_consultation_pre_policy_turn(
            &mut tasks,
            &mut reservations,
            limits(),
            UnixTimeV1::new(3).expect("turn time"),
            current,
            &mut executor,
        );

    assert!(matches!(
        disposition,
        ResidentConsultationPrePolicyTurn::CompletionReady(_)
    ));
    assert_eq!(executor.polls, 1);
}

#[test]
fn pre_policy_turn_returns_an_established_stop_without_requesting_policy() {
    let (mut tasks, mut reservations) = owners();
    let current =
        consultation_supervision_fixture("operation/pre-policy-stopped", "stopped", 10, true);
    let mut executor = CountingExecutor::default();

    let disposition: ResidentConsultationPrePolicyTurn<u8, u16, ()> =
        supervise_consultation_pre_policy_turn(
            &mut tasks,
            &mut reservations,
            limits(),
            UnixTimeV1::new(11).expect("turn time"),
            || Ok(current),
            &mut executor,
        );

    assert!(matches!(
        disposition,
        ResidentConsultationPrePolicyTurn::Stopped(_)
    ));
    assert_eq!(executor.polls, 1);
}

#[test]
fn pre_policy_turn_never_infers_continue_or_stop() {
    let (mut tasks, mut reservations) = owners();
    let current = consultation_supervision_fixture(
        "operation/pre-policy-required",
        "policy-required",
        100,
        false,
    );
    let expected_parent = current.collection().parent().clone();
    let observed_at = UnixTimeV1::new(2).expect("turn time");
    let mut executor = CountingExecutor::default();

    let disposition: ResidentConsultationPrePolicyTurn<u8, u16, ()> =
        supervise_recovered_consultation_pre_policy_turn(
            &mut tasks,
            &mut reservations,
            limits(),
            observed_at,
            current,
            &mut executor,
        );

    let ResidentConsultationPrePolicyTurn::PolicyRequired(frontier) = disposition else {
        panic!("mechanical progress must stop before choosing Continue or Stop")
    };
    assert_eq!(frontier.current().collection().parent(), &expected_parent);
    assert_eq!(frontier.observed_at(), observed_at);
    assert_eq!(executor.polls, 1);
    assert_eq!(executor.accepts, 0);
}

#[test]
fn facade_child_recovery_failure_retains_the_exact_batch_frontier() {
    prove_facade_offer_failure_retains_coordinate(FacadeOfferFailure::RecoverChild);
}

#[test]
fn facade_unopened_retention_failure_retains_the_exact_batch_frontier() {
    prove_facade_offer_failure_retains_coordinate(FacadeOfferFailure::RetentionUnopened);
}

#[test]
fn facade_uncertain_retention_failure_preserves_frontier_and_forbids_replay() {
    prove_facade_offer_failure_retains_coordinate(FacadeOfferFailure::RetentionUncertain);
}

#[test]
fn empty_turn_recovers_polls_and_decides_exactly_once_without_spinning() {
    let (mut tasks, mut reservations) = owners();
    let current = consultation_supervision_fixture("operation/turn", "turn", 100, false);
    let recoveries = std::cell::Cell::new(0);
    let decisions = std::cell::Cell::new(0);
    let mut executor = CountingExecutor::default();

    let disposition = supervise_consultation_turn(
        &mut tasks,
        &mut reservations,
        limits(),
        UnixTimeV1::new(2).expect("turn time"),
        || {
            recoveries.set(recoveries.get() + 1);
            Ok::<_, ()>(current)
        },
        &mut executor,
        |_, _| {
            decisions.set(decisions.get() + 1);
            ResidentConsultationSupervisionDecision::<()>::Continue(None)
        },
        |_| -> Result<RecoveredOperationObservation, ()> {
            panic!("no candidate means no child recovery")
        },
        |_, _| -> ConsultationChildRetentionOutcome<u8, ()> {
            panic!("no candidate means no retention")
        },
    );

    assert!(matches!(
        disposition,
        ResidentConsultationTurnDisposition::Pending(pending)
            if matches!(pending.as_ref(), ResidentConsultationTurnPending::NoCandidate { .. })
    ));
    assert_eq!(recoveries.get(), 1);
    assert_eq!(executor.polls, 1);
    assert_eq!(decisions.get(), 1);
}

#[test]
fn ready_completion_precedes_policy_and_is_never_polled_twice() {
    let (mut tasks, mut reservations) = owners();
    let current = consultation_supervision_fixture("operation/ready", "ready", 100, false);
    let planned = current
        .plan_child_dispatch(UnixTimeV1::new(2).expect("plan time"))
        .expect("one candidate");
    let candidate = planned.candidates()[0].clone();
    let coordinate = tasks
        .reconcile_batch(planned, limits(), &reservations)
        .expect("batch reconciliation")
        .usable()
        .expect("usable batch")
        .clone();
    let mut executor = CountingExecutor::default();
    tasks
        .seed_retained_for_supervision_test(
            &coordinate,
            candidate,
            dispatch(current.store(), "ready"),
            7,
            &mut reservations,
            &mut executor,
        )
        .expect("accepted child");
    executor.complete(70);
    let decisions = std::cell::Cell::new(0);

    let disposition = supervise_consultation_turn(
        &mut tasks,
        &mut reservations,
        limits(),
        UnixTimeV1::new(3).expect("turn time"),
        || Ok::<_, ()>(current),
        &mut executor,
        |_, _| {
            decisions.set(decisions.get() + 1);
            ResidentConsultationSupervisionDecision::<()>::Continue(None)
        },
        |_| -> Result<RecoveredOperationObservation, ()> { panic!("completion wins") },
        |_, _| -> ConsultationChildRetentionOutcome<u8, ()> { panic!("completion wins") },
    );

    assert!(matches!(
        disposition,
        ResidentConsultationTurnDisposition::Progress(
            ResidentConsultationTurnProgress::CompletionReady(_)
        )
    ));
    assert_eq!(executor.polls, 1);
    assert_eq!(decisions.get(), 0);
}

#[test]
fn another_parent_completion_is_returned_intact_without_join_or_policy() {
    let (mut tasks, mut reservations) = owners();
    let current = consultation_supervision_fixture("operation/current", "current", 100, false);
    let other = consultation_supervision_fixture("operation/other", "other", 100, false);
    let other_planned = other
        .plan_child_dispatch(UnixTimeV1::new(2).expect("plan time"))
        .expect("other candidate");
    let other_candidate = other_planned.candidates()[0].clone();
    let other_coordinate = tasks
        .reconcile_batch(other_planned, limits(), &reservations)
        .expect("other reconciliation")
        .usable()
        .expect("other batch")
        .clone();
    let mut executor = CountingExecutor::default();
    tasks
        .seed_retained_for_supervision_test(
            &other_coordinate,
            other_candidate,
            dispatch(other.store(), "other"),
            9,
            &mut reservations,
            &mut executor,
        )
        .expect("other child accepted");
    executor.complete(90);
    let decisions = std::cell::Cell::new(0);

    let disposition = supervise_consultation_turn(
        &mut tasks,
        &mut reservations,
        limits(),
        UnixTimeV1::new(3).expect("turn time"),
        || Ok::<_, ()>(current),
        &mut executor,
        |_, _| {
            decisions.set(decisions.get() + 1);
            ResidentConsultationSupervisionDecision::<()>::Continue(None)
        },
        |_| -> Result<RecoveredOperationObservation, ()> { panic!("foreign completion wins") },
        |_, _| -> ConsultationChildRetentionOutcome<u8, ()> { panic!("foreign completion wins") },
    );

    let ResidentConsultationTurnDisposition::NeedsRecovery(reason) = disposition else {
        panic!("another parent's completion must remain intact")
    };
    let ResidentConsultationTurnNeedsRecovery::ForeignCompletion(poll) = *reason else {
        panic!("another parent's completion must retain its settlement")
    };
    assert_eq!(poll.coordinate, other_coordinate);
    assert!(tasks.require_pending_terminal(&poll, &reservations).is_ok());
    assert_eq!(executor.polls, 1);
    assert_eq!(decisions.get(), 0);
}

#[test]
fn deadline_crossing_polls_before_one_policy_decision_and_never_offers() {
    let (mut tasks, mut reservations) = owners();
    let current = consultation_supervision_fixture("operation/deadline", "deadline", 10, false);
    let candidate = current
        .plan_child_dispatch(UnixTimeV1::new(2).expect("pre-deadline"))
        .expect("candidate")
        .candidates()[0]
        .clone();
    let decisions = std::cell::Cell::new(0);
    let mut executor = CountingExecutor::default();

    let disposition = supervise_consultation_turn(
        &mut tasks,
        &mut reservations,
        limits(),
        UnixTimeV1::new(10).expect("deadline"),
        || Ok::<_, ()>(current),
        &mut executor,
        |_, _| {
            decisions.set(decisions.get() + 1);
            ResidentConsultationSupervisionDecision::<()>::Continue(Some(Box::new(candidate)))
        },
        |_| -> Result<RecoveredOperationObservation, ()> { panic!("deadline forbids offer") },
        |_, _| -> ConsultationChildRetentionOutcome<u8, ()> { panic!("deadline forbids offer") },
    );

    assert!(matches!(
        disposition,
        ResidentConsultationTurnDisposition::Pending(pending)
            if matches!(
                pending.as_ref(),
                ResidentConsultationTurnPending::CollectionBoundary { .. }
            )
    ));
    assert_eq!(executor.polls, 1);
    assert_eq!(decisions.get(), 1);
    assert_eq!(executor.accepts, 0);
}

#[test]
fn established_stop_skips_policy_after_one_empty_poll() {
    let (mut tasks, mut reservations) = owners();
    let current = consultation_supervision_fixture("operation/stopped", "stopped", 10, true);
    let decisions = std::cell::Cell::new(0);
    let mut executor = CountingExecutor::default();

    let disposition = supervise_consultation_turn(
        &mut tasks,
        &mut reservations,
        limits(),
        UnixTimeV1::new(11).expect("post-stop"),
        || Ok::<_, ()>(current),
        &mut executor,
        |_, _| {
            decisions.set(decisions.get() + 1);
            ResidentConsultationSupervisionDecision::<()>::Continue(None)
        },
        |_| -> Result<RecoveredOperationObservation, ()> { panic!("stopped collection") },
        |_, _| -> ConsultationChildRetentionOutcome<u8, ()> { panic!("stopped collection") },
    );

    assert!(matches!(
        disposition,
        ResidentConsultationTurnDisposition::Stopped(ResidentConsultationTurnStopped::Established(
            _
        ))
    ));
    assert_eq!(executor.polls, 1);
    assert_eq!(decisions.get(), 0);
}

#[test]
fn selected_stop_runs_once_after_the_single_empty_poll() {
    let (mut tasks, mut reservations) = owners();
    let current = consultation_supervision_fixture("operation/stop", "stop", 100, false);
    let decisions = std::cell::Cell::new(0);
    let mut executor = CountingExecutor::default();

    let disposition = supervise_consultation_turn(
        &mut tasks,
        &mut reservations,
        limits(),
        UnixTimeV1::new(2).expect("turn time"),
        || Ok::<_, ()>(current),
        &mut executor,
        |_, _| {
            decisions.set(decisions.get() + 1);
            ResidentConsultationSupervisionDecision::Stop("selected-stop")
        },
        |_| -> Result<RecoveredOperationObservation, ()> { panic!("stop does not recover child") },
        |_, _| -> ConsultationChildRetentionOutcome<u8, ()> { panic!("stop does not retain") },
    );

    let ResidentConsultationTurnDisposition::Stopped(ResidentConsultationTurnStopped::Selected(
        selected,
    )) = disposition
    else {
        panic!("Package stop must leave as an owned phase")
    };
    let (_, selected) = selected.into_parts();
    assert_eq!(selected, "selected-stop");
    assert_eq!(executor.polls, 1);
    assert_eq!(decisions.get(), 1);
}

#[test]
fn aggregate_capacity_returns_pending_before_child_or_retention_io() {
    let (mut tasks, mut reservations) = owners();
    let current = consultation_supervision_fixture("operation/capacity", "capacity", 100, false);
    let candidate = current
        .plan_child_dispatch(UnixTimeV1::new(2).expect("plan time"))
        .expect("candidate")
        .candidates()[0]
        .clone();
    for suffix in 0..4 {
        let offer = reservations
            .begin_offer(
                dispatch(current.store(), &format!("occupied-{suffix}")),
                crate::resident::task_reservation::ResidentTaskReservationRoute::Provider,
            )
            .expect("aggregate slot");
        reservations.accept_offer(offer).expect("occupied slot");
    }
    let child_calls = std::cell::Cell::new(0);
    let retention_calls = std::cell::Cell::new(0);
    let mut executor = CountingExecutor::default();

    let disposition = supervise_consultation_turn(
        &mut tasks,
        &mut reservations,
        limits(),
        UnixTimeV1::new(2).expect("turn time"),
        || Ok::<_, ()>(current),
        &mut executor,
        |_, _| ResidentConsultationSupervisionDecision::<()>::Continue(Some(Box::new(candidate))),
        |_| -> Result<RecoveredOperationObservation, ()> {
            child_calls.set(child_calls.get() + 1);
            panic!("capacity rejection precedes child recovery")
        },
        |_, _| -> ConsultationChildRetentionOutcome<u8, ()> {
            retention_calls.set(retention_calls.get() + 1);
            panic!("capacity rejection precedes retention")
        },
    );

    assert!(matches!(
        disposition,
        ResidentConsultationTurnDisposition::Pending(pending)
            if matches!(pending.as_ref(), ResidentConsultationTurnPending::Capacity { .. })
    ));
    assert_eq!(executor.polls, 1);
    assert_eq!(child_calls.get(), 0);
    assert_eq!(retention_calls.get(), 0);
    assert_eq!(executor.accepts, 0);
}

#[test]
fn one_turn_polls_once_and_offers_at_most_one_exact_candidate() {
    let (mut tasks, mut reservations) = owners();
    let operation = OperationFixture::exact();
    let current = consultation_supervision_fixture_with_child(
        "operation/one-offer",
        "one-offer",
        child_coordinate(&operation),
        100,
        false,
    );
    let candidate = current
        .plan_child_dispatch(UnixTimeV1::new(2).expect("plan time"))
        .expect("candidate")
        .candidates()[0]
        .clone();
    let child_store = current.store().clone();
    let recovered = recovered_child(&operation, child_store.clone(), false);
    let retained = recovered_child(&operation, child_store, true);
    let decisions = std::cell::Cell::new(0);
    let child_reads = std::cell::Cell::new(0);
    let retentions = std::cell::Cell::new(0);
    let mut executor = CountingExecutor::default();

    let disposition = supervise_consultation_turn(
        &mut tasks,
        &mut reservations,
        limits(),
        UnixTimeV1::new(2).expect("turn time"),
        || Ok::<_, ()>(current),
        &mut executor,
        |_, _| {
            decisions.set(decisions.get() + 1);
            ResidentConsultationSupervisionDecision::<()>::Continue(Some(Box::new(candidate)))
        },
        |_| {
            child_reads.set(child_reads.get() + 1);
            Ok::<_, ()>(recovered)
        },
        |_, _| {
            retentions.set(retentions.get() + 1);
            ConsultationChildRetentionOutcome::<u8, ()>::Retained {
                child: Box::new(retained),
                work: 7,
            }
        },
    );

    assert!(matches!(
        disposition,
        ResidentConsultationTurnDisposition::Progress(
            ResidentConsultationTurnProgress::Offered { .. }
        )
    ));
    assert_eq!(executor.polls, 1);
    assert_eq!(executor.accepts, 1);
    assert_eq!(decisions.get(), 1);
    assert_eq!(child_reads.get(), 1);
    assert_eq!(retentions.get(), 1);
}

#[test]
fn recovered_dispatched_child_is_not_replayed_after_restart() {
    let (mut tasks, mut reservations) = owners();
    let operation = OperationFixture::exact();
    let current = consultation_supervision_fixture_with_child(
        "operation/restart",
        "restart",
        child_coordinate(&operation),
        100,
        false,
    );
    let candidate = current
        .plan_child_dispatch(UnixTimeV1::new(2).expect("plan time"))
        .expect("candidate")
        .candidates()[0]
        .clone();
    let recovered = recovered_child(&operation, current.store().clone(), true);
    let retentions = std::cell::Cell::new(0);
    let mut executor = CountingExecutor::default();

    let disposition = supervise_consultation_turn(
        &mut tasks,
        &mut reservations,
        limits(),
        UnixTimeV1::new(2).expect("turn time"),
        || Ok::<_, ()>(current),
        &mut executor,
        |_, _| ResidentConsultationSupervisionDecision::<()>::Continue(Some(Box::new(candidate))),
        |_| Ok::<_, ()>(recovered),
        |_, _| {
            retentions.set(retentions.get() + 1);
            ConsultationChildRetentionOutcome::<u8, ()>::Unopened(())
        },
    );

    assert!(matches!(
        disposition,
        ResidentConsultationTurnDisposition::NeedsRecovery(reason)
            if matches!(reason.as_ref(), ResidentConsultationTurnNeedsRecovery::Offer(_))
    ));
    assert_eq!(executor.polls, 1);
    assert_eq!(executor.accepts, 0);
    assert_eq!(retentions.get(), 0);
}

#[test]
fn unplanned_policy_candidate_is_rejected_before_child_io() {
    let (mut tasks, mut reservations) = owners();
    let current = consultation_supervision_fixture("operation/planned", "planned", 100, false);
    let foreign = consultation_supervision_fixture("operation/foreign", "foreign", 100, false)
        .plan_child_dispatch(UnixTimeV1::new(2).expect("plan time"))
        .expect("foreign candidate")
        .candidates()[0]
        .clone();
    let child_reads = std::cell::Cell::new(0);
    let retentions = std::cell::Cell::new(0);
    let mut executor = CountingExecutor::default();

    let disposition = supervise_consultation_turn(
        &mut tasks,
        &mut reservations,
        limits(),
        UnixTimeV1::new(2).expect("turn time"),
        || Ok::<_, ()>(current),
        &mut executor,
        |_, _| ResidentConsultationSupervisionDecision::<()>::Continue(Some(Box::new(foreign))),
        |_| -> Result<RecoveredOperationObservation, ()> {
            child_reads.set(child_reads.get() + 1);
            panic!("an unplanned candidate must fail before child recovery")
        },
        |_, _| -> ConsultationChildRetentionOutcome<u8, ()> {
            retentions.set(retentions.get() + 1);
            panic!("an unplanned candidate must fail before retention")
        },
    );

    assert!(matches!(
        disposition,
        ResidentConsultationTurnDisposition::NeedsRecovery(reason)
            if matches!(reason.as_ref(), ResidentConsultationTurnNeedsRecovery::Offer(_))
    ));
    assert_eq!(executor.polls, 1);
    assert_eq!(executor.accepts, 0);
    assert_eq!(child_reads.get(), 0);
    assert_eq!(retentions.get(), 0);
}

#[test]
fn collection_recovery_failure_precedes_poll_and_policy() {
    let (mut tasks, mut reservations) = owners();
    let decisions = std::cell::Cell::new(0);
    let mut executor = CountingExecutor::default();

    let disposition = supervise_consultation_turn(
        &mut tasks,
        &mut reservations,
        limits(),
        UnixTimeV1::new(2).expect("turn time"),
        || Err::<RecoveredConsultationCollection, _>("recovery-failed"),
        &mut executor,
        |_, _| {
            decisions.set(decisions.get() + 1);
            ResidentConsultationSupervisionDecision::<()>::Continue(None)
        },
        |_| -> Result<RecoveredOperationObservation, ()> {
            panic!("failed collection recovery cannot read a child")
        },
        |_, _| -> ConsultationChildRetentionOutcome<u8, ()> {
            panic!("failed collection recovery cannot retain a child")
        },
    );

    assert!(matches!(
        disposition,
        ResidentConsultationTurnDisposition::NeedsRecovery(reason)
            if matches!(
                reason.as_ref(),
                ResidentConsultationTurnNeedsRecovery::Collection("recovery-failed")
            )
    ));
    assert_eq!(executor.polls, 0);
    assert_eq!(executor.accepts, 0);
    assert_eq!(decisions.get(), 0);
}
