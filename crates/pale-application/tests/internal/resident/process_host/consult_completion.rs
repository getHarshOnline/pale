//! Ownership laws for rehoming one already-polled Consult completion.

use std::collections::VecDeque;
use std::num::NonZeroUsize;

use super::*;
use crate::resident::consultation::service::task::ResidentConsultationTaskPoll;
use crate::resident::consultation::tests::consultation_supervision_fixture_with_child_in_store;
use crate::resident::consultation::{
    ConsultationChildRetentionOutcome, ResidentConsultationPrePolicyNeedsRecovery,
    ResidentConsultationPrePolicyTurn, ResidentConsultationTaskLimits,
};
use crate::resident::operation_observation_custody::tests::{
    admit_initial_checkpoint, signing_authority, OperationFixture,
};
use crate::resident::process_host::consult_completion::ResidentProcessConsultationCompletionRehomeReason;
use crate::resident::process_host::consult_recovery::ResidentProcessConsultRecoveryTurn;
use crate::resident::task_owner::{
    ResidentOperationTask, ResidentOperationTaskCompletion, ResidentOperationTaskExecutor,
};
use crate::resident::task_reservation::ResidentAggregateTaskLimits;
use crate::resident::RecoveredOperationObservation;
use pale_core::records::{
    ArtifactDigest, CapabilityAttemptIdV1, ImmutableRecordReferenceV1, OperationActivePhaseV1,
    OperationObservationRevisionV1, OperationObservationStateV1,
};
use pale_ports::{ControlRecordDigest, WallClockSourceError, WallClockSourcePort};

#[derive(Default)]
struct ReadyExecutor {
    accepts: usize,
    polls: usize,
    accepted: Vec<ResidentOperationTask<u8>>,
    completions: VecDeque<ResidentOperationTaskCompletion<u8, u16>>,
}

impl ReadyExecutor {
    fn complete_first(&mut self, output: u16) {
        let task = self.accepted.remove(0);
        self.completions
            .push_back(ResidentOperationTaskCompletion::Completed { task, output });
    }
}

impl ResidentOperationTaskExecutor<u8, u16> for ReadyExecutor {
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

#[derive(Clone, Copy)]
struct FixedClock(UnixTimeV1);

impl WallClockSourcePort for FixedClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Ok(self.0)
    }
}

fn consultation_task_state() -> ResidentOperationTaskState {
    ResidentOperationTaskState::new(
        ResidentOperationTaskLimits::new(NonZeroUsize::new(1).expect("positive provider bound")),
        NonZeroUsize::new(2).expect("two concurrent Consult parents"),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).expect("one in-flight child")),
    )
    .expect("bounded shared task state")
}

fn task_limits() -> ResidentConsultationTaskLimits {
    ResidentConsultationTaskLimits::new(
        NonZeroUsize::new(1).expect("one child per batch"),
        NonZeroUsize::new(1).expect("one batch member"),
    )
}

fn require_ok<T, Error>(result: Result<T, Error>, message: &str) -> T {
    match result {
        Ok(value) => value,
        Err(_) => panic!("{message}"),
    }
}

fn require_error<T, Error>(result: Result<T, Error>, message: &str) -> Error {
    match result {
        Ok(_) => panic!("{message}"),
        Err(error) => error,
    }
}

fn operation_coordinate(operation: &OperationFixture) -> ImmutableRecordReferenceV1 {
    let (reference, handle) = operation.handle();
    ImmutableRecordReferenceV1 {
        record_ref: reference.clone(),
        record_sha256: ArtifactDigest::parse(handle.canonical_operation_handle_sha256().as_str())
            .expect("fixture handle digest"),
    }
}

fn assert_same_collection(
    actual: &RecoveredConsultationCollection,
    expected: &RecoveredConsultationCollection,
) {
    assert_eq!(actual.store(), expected.store());
    assert_eq!(actual.collection().parent(), expected.collection().parent());
    assert_eq!(
        actual.frontier_reference().expect("actual frontier"),
        expected.frontier_reference().expect("expected frontier")
    );
    assert_eq!(actual, expected);
}

fn recovered_child(
    operation: &OperationFixture,
    store: AuthorityStoreCoordinate,
    dispatched: bool,
) -> RecoveredOperationObservation {
    let (handle_ref, handle) = operation.handle();
    let selected = if dispatched {
        let reference = local_operation_start_fixture::record_ref("operation/rehome-consult");
        let observation = local_operation_start_fixture::issue_observation(
            &signing_authority(),
            handle_ref,
            handle,
            local_operation_start_fixture::schema_digest(
                pale_core::records::OPERATION_OBSERVATION_V1_SCHEMA_ID,
            ),
            OperationObservationRevisionV1::new(3).expect("dispatch revision"),
            Some(operation.successor()),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched {
                    attempt_id: CapabilityAttemptIdV1::parse("rehome-consult-attempt")
                        .expect("attempt identity"),
                    evidence: ImmutableRecordReferenceV1 {
                        record_ref: local_operation_start_fixture::record_ref(
                            "attempt/rehome-consult",
                        ),
                        record_sha256: ArtifactDigest::hash_bytes(b"rehome-consult-attempt"),
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
            local_operation_start_fixture::record_ref("checkpoint/rehome-consult"),
            admit_initial_checkpoint(&local_operation_start_fixture::checkpoint_preimage(1, None)),
        ),
        store,
    )
}

#[test]
fn foreign_completion_rehomes_once_to_its_exact_parent() {
    let first_port = MemoryAuthorityStore::default();
    let alternate_port = MemoryAuthorityStore::default();
    let principal_id = principal("principal-consult-completion-rehome");
    let selected_store = store("consult-rehome", "consult-rehome-incarnation");
    let alternate_store = store("consult-rehome-other", "consult-rehome-other-incarnation");
    let selector = unproved_selector(principal_id.clone(), selected_store.clone());
    let alternate_selector = unproved_selector(principal_id.clone(), alternate_store.clone());
    let mut host = ResidentProcessHost::new(
        unproved_writer(
            principal_id.clone(),
            vec![selected_store.clone(), alternate_store.clone()],
        ),
        vec![
            unproved_deployment_for_selector(
                &selector,
                selected_store.clone(),
                ready_application(&first_port, selected_store.clone(), 1),
            ),
            unproved_deployment_for_selector(
                &alternate_selector,
                alternate_store.clone(),
                ready_application(&alternate_port, alternate_store.clone(), 2),
            ),
        ],
        NonZeroUsize::new(2).expect("two deployment bound"),
        ResidentOperationHost::new(consultation_task_state()),
    )
    .expect("two exact deployments");

    let first_child = OperationFixture::exact();
    let second_child = OperationFixture::exact();
    let first_current = consultation_supervision_fixture_with_child_in_store(
        "operation/rehome-parent-a",
        "rehome-child-a",
        operation_coordinate(&first_child),
        100,
        false,
        selected_store.clone(),
    );
    let second_current = consultation_supervision_fixture_with_child_in_store(
        "operation/rehome-parent-b",
        "rehome-child-b",
        operation_coordinate(&second_child),
        100,
        false,
        selected_store.clone(),
    );
    let first_batch = first_current
        .plan_child_dispatch(UnixTimeV1::new(2).expect("dispatch time"))
        .expect("first finite batch");
    let second_batch = second_current
        .plan_child_dispatch(UnixTimeV1::new(2).expect("dispatch time"))
        .expect("second finite batch");
    let second_candidate = second_batch.candidates()[0].clone();
    let second_initial = recovered_child(&second_child, selected_store.clone(), false);
    let second_retained = recovered_child(&second_child, selected_store.clone(), true);
    let mut executor = ReadyExecutor::default();
    let (mut authority, mut custody, mut anchor) = operation_ports();

    with_unproved_operation_service_for_test(
        &mut host,
        &selector,
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut custody,
            sessions: &first_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_store, service| {
            service
                .open_consultation_batch(first_batch, task_limits())
                .expect("first parent batch");
            let second_coordinate = service
                .open_consultation_batch(second_batch, task_limits())
                .expect("second parent batch");
            service
                .retain_and_accept_consultation_task(
                    &second_coordinate,
                    &second_candidate,
                    &FixedClock(UnixTimeV1::new(2).expect("dispatch time")),
                    || Ok::<_, std::convert::Infallible>(second_current.clone()),
                    |_| Ok::<_, std::convert::Infallible>(second_initial),
                    |_, _| {
                        ConsultationChildRetentionOutcome::<u8, std::convert::Infallible>::Retained {
                            child: Box::new(second_retained),
                            work: 7,
                        }
                    },
                    &mut executor,
                )
                .expect("second parent owns one accepted child");
        },
    )
    .expect("selected deployment exposes one shared task owner");
    executor.complete_first(70);
    let turn = host
        .operations
        .supervise_recovered_consultation_pre_policy_turn::<u8, u16, std::convert::Infallible, _>(
            task_limits(),
            UnixTimeV1::new(3).expect("completion time"),
            first_current,
            &mut executor,
        );
    let ResidentConsultationPrePolicyTurn::NeedsRecovery(reason) = turn else {
        panic!("a completion owned by the second parent must remain linear")
    };
    let ResidentConsultationPrePolicyNeedsRecovery::ForeignCompletion(poll) = *reason else {
        panic!("the shared executor completion must retain its exact batch coordinate")
    };
    assert_eq!((executor.accepts, executor.polls), (1, 1));

    let retained_cursor = host.consult_recovery_attempts[0].clone();
    let foreign_successor = LocalOperationStartDiscoveryAttempt::new(alternate_store.clone());
    let retained = host.finish_consult_recovery_turn::<u8, u16>(
        0,
        foreign_successor,
        ResidentProcessConsultRecoveryTurn::NeedsRecovery(Box::new(
            ResidentConsultationPrePolicyNeedsRecovery::ForeignCompletion(poll),
        )),
    );
    assert_eq!(host.consult_recovery_attempts[0], retained_cursor);
    let ResidentProcessConsultRecoveryTurn::NeedsRecovery(reason) = retained else {
        panic!("foreign completion keeps the first source selected for retry")
    };
    let ResidentConsultationPrePolicyNeedsRecovery::ForeignCompletion(poll) = *reason else {
        panic!("the exact foreign completion survives cursor retention")
    };
    let expected_coordinate = poll.coordinate.clone();

    let other_port = MemoryAuthorityStore::default();
    let other_host = ResidentProcessHost::new(
        unproved_writer(principal_id, vec![selected_store.clone()]),
        vec![unproved_deployment_for_selector(
            &selector,
            selected_store.clone(),
            ready_application(&other_port, selected_store.clone(), 3),
        )],
        NonZeroUsize::new(1).expect("one deployment"),
        ResidentOperationHost::new(consultation_task_state()),
    )
    .expect("second process host");
    let owner_failure = require_error(
        other_host.rehome_foreign_consultation_completion(
            selector.clone(),
            second_current.clone(),
            *poll,
        ),
        "another process task owner cannot claim the poll",
    );
    assert!(matches!(
        owner_failure.reason,
        ResidentProcessConsultationCompletionRehomeReason::TaskOwner
    ));
    assert_eq!(owner_failure.selector, selector);
    assert_same_collection(&owner_failure.current, &second_current);
    assert_eq!(owner_failure.poll.coordinate, expected_coordinate);
    let mut poll = owner_failure.poll;

    let mut unknown_selector = selector.clone();
    unknown_selector.runtime_authority_ref =
        ResidentRecordRefV1::parse("authority/unknown-consult-rehome")
            .expect("unknown authority reference");
    let selection_failure = require_error(
        host.rehome_foreign_consultation_completion(unknown_selector, second_current.clone(), poll),
        "an unknown selector cannot claim the poll",
    );
    assert!(matches!(
        selection_failure.reason,
        ResidentProcessConsultationCompletionRehomeReason::Deployment(
            ResidentProcessHostSelectionError::UnknownDeploymentAuthority
        )
    ));
    assert_same_collection(&selection_failure.current, &second_current);
    assert_eq!(selection_failure.poll.coordinate, expected_coordinate);
    poll = selection_failure.poll;

    let poll_store_failure = require_error(
        host.rehome_foreign_consultation_completion(
            alternate_selector,
            second_current.clone(),
            poll,
        ),
        "another deployment store cannot claim the poll",
    );
    assert!(matches!(
        poll_store_failure.reason,
        ResidentProcessConsultationCompletionRehomeReason::PollStore
    ));
    assert_same_collection(&poll_store_failure.current, &second_current);
    assert_eq!(poll_store_failure.poll.coordinate, expected_coordinate);
    poll = poll_store_failure.poll;

    let alternate_current = consultation_supervision_fixture_with_child_in_store(
        "operation/rehome-parent-other-store",
        "rehome-child-other-store",
        operation_coordinate(&OperationFixture::exact()),
        100,
        false,
        alternate_store,
    );
    let expected_alternate_current = alternate_current.clone();
    let collection_store_failure = require_error(
        host.rehome_foreign_consultation_completion(selector.clone(), alternate_current, poll),
        "another collection store cannot claim the poll",
    );
    assert!(matches!(
        collection_store_failure.reason,
        ResidentProcessConsultationCompletionRehomeReason::CollectionStore
    ));
    assert_same_collection(
        &collection_store_failure.current,
        &expected_alternate_current,
    );
    assert_eq!(
        collection_store_failure.poll.coordinate,
        expected_coordinate
    );
    poll = collection_store_failure.poll;

    let wrong_parent = consultation_supervision_fixture_with_child_in_store(
        "operation/rehome-parent-wrong",
        "rehome-child-wrong-parent",
        operation_coordinate(&OperationFixture::exact()),
        100,
        false,
        selected_store.clone(),
    );
    let expected_wrong_parent = wrong_parent.clone();
    let parent_failure = require_error(
        host.rehome_foreign_consultation_completion(selector.clone(), wrong_parent, poll),
        "another parent cannot claim the poll",
    );
    assert!(matches!(
        parent_failure.reason,
        ResidentProcessConsultationCompletionRehomeReason::Parent
    ));
    assert_same_collection(&parent_failure.current, &expected_wrong_parent);
    assert_eq!(parent_failure.poll.coordinate, expected_coordinate);
    poll = parent_failure.poll;

    let wrong_frontier = consultation_supervision_fixture_with_child_in_store(
        "operation/rehome-parent-b",
        "rehome-child-b-altered",
        operation_coordinate(&OperationFixture::exact()),
        100,
        false,
        selected_store.clone(),
    );
    assert_eq!(
        wrong_frontier.collection().parent(),
        second_current.collection().parent()
    );
    assert_ne!(
        wrong_frontier.frontier_reference().expect("wrong frontier"),
        second_current
            .frontier_reference()
            .expect("second frontier")
    );
    let expected_wrong_frontier = wrong_frontier.clone();
    let frontier_failure = require_error(
        host.rehome_foreign_consultation_completion(selector.clone(), wrong_frontier, poll),
        "another roster frontier cannot claim the poll",
    );
    assert!(matches!(
        frontier_failure.reason,
        ResidentProcessConsultationCompletionRehomeReason::CollectionFrontier
    ));
    assert_same_collection(&frontier_failure.current, &expected_wrong_frontier);
    assert_eq!(frontier_failure.poll.coordinate, expected_coordinate);
    poll = frontier_failure.poll;

    let ready = host
        .rehome_foreign_consultation_completion(selector.clone(), second_current, poll)
        .unwrap_or_else(|_| panic!("the exact retained parent must reclaim its completion"));
    assert_eq!(ready.selector, selector);
    let (current, poll) = ready.ready.into_parts();
    assert_eq!(current.collection().parent(), poll.coordinate.parent());
    assert_eq!(
        current.frontier_reference().expect("current frontier"),
        *poll.coordinate.collection_frontier()
    );
    assert!(matches!(
        poll.outcome,
        ResidentConsultationTaskPoll::Completed { output: 70, .. }
    ));
    assert_eq!(executor.polls, 1, "rehome never polls the executor again");

    // `native_invoke::consult_child_native_invoke_recovers_runs_once_and_contributes_only_to_its_slot`
    // drives this same ready phase through durable child and parent custody,
    // including stopped-parent late evidence. The rehome owner adds no second
    // join or publication path.
}
