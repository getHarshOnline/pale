//! Cancellation routing for one already-dispatched Consult child.

use super::*;
use crate::resident::consultation::service::task::ResidentConsultationTaskOwnerPoll;
use crate::resident::consultation::tests::consultation_task_batch_fixture_in_store;
use crate::resident::task_owner::{
    ResidentOperationTask, ResidentOperationTaskCancellationState, ResidentOperationTaskCompletion,
    ResidentOperationTaskExecutor,
};

#[derive(Default)]
struct HeldConsultChildExecutor {
    accepted: Vec<ResidentOperationTask<u8>>,
    completions: VecDeque<ResidentOperationTaskCompletion<u8, u16>>,
}

impl HeldConsultChildExecutor {
    fn complete(&mut self, key: &ResidentOperationTaskKey, output: u16) {
        let index = self
            .accepted
            .iter()
            .position(|task| task.key() == key)
            .expect("the exact Consult child remains accepted");
        let task = self.accepted.remove(index);
        self.completions
            .push_back(ResidentOperationTaskCompletion::Completed { task, output });
    }

    fn lose(&mut self, key: &ResidentOperationTaskKey) {
        let index = self
            .accepted
            .iter()
            .position(|task| task.key() == key)
            .expect("the exact Consult child remains accepted");
        let task = self.accepted.remove(index);
        self.completions.push_back(task.into_lost());
    }
}

impl ResidentOperationTaskExecutor<u8, u16> for HeldConsultChildExecutor {
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<u8>,
    ) -> Result<(), ResidentOperationTask<u8>> {
        self.accepted.push(task);
        Ok(())
    }

    fn try_completion(&mut self) -> Option<ResidentOperationTaskCompletion<u8, u16>> {
        self.completions.pop_front()
    }
}

fn dispatched_task_key(
    authorized: &AuthorizedOperationCancellation<'_>,
) -> ResidentOperationTaskKey {
    let selected = authorized.recovered_observation().selected_head();
    ResidentOperationTaskKey::new(
        authorized.recovered_observation().store().clone(),
        ImmutableRecordReferenceV1 {
            record_ref: selected.0.clone(),
            record_sha256: ArtifactDigest::parse(
                selected.1.canonical_operation_observation_sha256().as_str(),
            )
            .expect("issued observation has an artifact digest"),
        },
    )
}

fn task_state(maximum_consultation_batches: usize) -> ResidentOperationTaskState {
    ResidentOperationTaskState::new(
        ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
        NonZeroUsize::new(maximum_consultation_batches).unwrap(),
        ResidentAggregateTaskLimits::new(NonZeroUsize::new(2).unwrap()),
    )
    .expect("focused process-local task state")
}

#[test]
fn dispatched_consult_child_is_notified_only_after_cancellation_custody() {
    with_owned_dispatched_cancellation_custody(
        &[],
        false,
        |authority, store, anchor, authorized, input, mut admission| {
            let dispatch = dispatched_task_key(&authorized);
            let batch = consultation_task_batch_fixture_in_store(
                dispatch.store().clone(),
                "consultation/cancel-parent",
                "consultation/cancel-frontier",
                "cancel-child",
            );
            let sibling_dispatch = ResidentOperationTaskKey::new(
                dispatch.store().clone(),
                ImmutableRecordReferenceV1 {
                    record_ref: record_ref("operation/unrelated-consult-sibling"),
                    record_sha256: artifact_digest('4'),
                },
            );
            let sibling_batch = consultation_task_batch_fixture_in_store(
                dispatch.store().clone(),
                "consultation/sibling-parent",
                "consultation/sibling-frontier",
                "unrelated-sibling",
            );
            let mut tasks = task_state(2);
            let mut executor = HeldConsultChildExecutor::default();
            tasks.seed_consultation_dispatch_for_test(batch, dispatch.clone(), 7, &mut executor);
            tasks.seed_consultation_dispatch_for_test(
                sibling_batch,
                sibling_dispatch.clone(),
                8,
                &mut executor,
            );

            let outcome = match ResidentOperationService::new(authority, store, anchor, &mut tasks)
                .retain_authorized_cancellation(authorized, input, &mut admission)
            {
                Ok(outcome) => outcome,
                Err(_) => panic!("the exact Consult child accepts its admitted cancellation"),
            };
            assert!(matches!(
                outcome,
                ResidentOperationCancellationOutcome::DispatchedRequestRetained {
                    notification: ResidentOperationTaskCancellationNotification::Notified,
                    ..
                }
            ));

            executor.complete(&sibling_dispatch, 13);
            let sibling_poll = tasks
                .consultation_tasks
                .try_completion(&mut executor)
                .expect("the unrelated sibling completion is routed")
                .expect("one Consult batch owns the completion");
            assert!(matches!(
                sibling_poll,
                ResidentConsultationTaskOwnerPoll {
                    outcome,
                    ..
                } if matches!(
                    outcome,
                    crate::resident::consultation::service::task::ResidentConsultationTaskPoll::Completed {
                        output: 13,
                        cancellation: ResidentOperationTaskCancellationState::Unnotified,
                        ..
                    }
                )
            ));

            executor.complete(&dispatch, 17);
            let selected_poll = tasks
                .consultation_tasks
                .try_completion(&mut executor)
                .expect("the selected late completion is routed")
                .expect("one Consult batch owns the completion");
            assert!(matches!(
                selected_poll,
                ResidentConsultationTaskOwnerPoll {
                    outcome,
                    ..
                } if matches!(
                    outcome,
                    crate::resident::consultation::service::task::ResidentConsultationTaskPoll::Completed {
                        output: 17,
                        cancellation: ResidentOperationTaskCancellationState::Notified,
                        ..
                    }
                )
            ));
        },
    );
}

#[test]
fn missing_dispatched_owner_stops_before_cancellation_custody() {
    with_authorized_cancellation(
        dispatched_state("missing-consult-child"),
        |authority, store, anchor, authorized, input, admission| {
            let mut tasks = task_state(1);
            let observation_signs = Cell::new(0);
            let checkpoint_signs = Cell::new(0);
            let original_observation_signer = admission.sign_observation;
            let original_checkpoint_signer = admission.sign_checkpoint;
            let mut sign_observation =
                |bytes: &[u8],
                 preimage: &OperationObservationPreimageV1,
                 digest: &OperationObservationDigest| {
                    observation_signs.set(observation_signs.get() + 1);
                    original_observation_signer(bytes, preimage, digest)
                };
            let mut sign_checkpoint =
                |bytes: &[u8],
                 preimage: &ControlStateCheckpointPreimageV1,
                 digest: &ControlStateCheckpointDigest| {
                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                    original_checkpoint_signer(bytes, preimage, digest)
                };
            let mut counted_admission = CancellationAppendAdmission {
                resolve_store: admission.resolve_store,
                admit_rotation: admission.admit_rotation,
                sign_observation: &mut sign_observation,
                sign_checkpoint: &mut sign_checkpoint,
            };
            let outcome = match ResidentOperationService::new(authority, store, anchor, &mut tasks)
                .retain_authorized_cancellation(authorized, input, &mut counted_admission)
            {
                Ok(outcome) => outcome,
                Err(_) => panic!("missing process-local ownership is not an operation error"),
            };
            assert!(matches!(
                outcome,
                ResidentOperationCancellationOutcome::DispatchOwnerRequired(_)
            ));
            assert_eq!((observation_signs.get(), checkpoint_signs.get()), (0, 0));
        },
    );
}

#[test]
fn dispatched_consult_child_loss_preserves_the_advisory_notification() {
    with_owned_dispatched_cancellation_custody(
        &[],
        false,
        |authority, store, anchor, authorized, input, mut admission| {
            let dispatch = dispatched_task_key(&authorized);
            let batch = consultation_task_batch_fixture_in_store(
                dispatch.store().clone(),
                "consultation/lost-parent",
                "consultation/lost-frontier",
                "lost-child",
            );
            let mut tasks = task_state(1);
            let mut executor = HeldConsultChildExecutor::default();
            tasks.seed_consultation_dispatch_for_test(batch, dispatch.clone(), 9, &mut executor);

            let outcome = ResidentOperationService::new(authority, store, anchor, &mut tasks)
                .retain_authorized_cancellation(authorized, input, &mut admission);
            assert!(matches!(
                outcome,
                Ok(
                    ResidentOperationCancellationOutcome::DispatchedRequestRetained {
                        notification: ResidentOperationTaskCancellationNotification::Notified,
                        ..
                    }
                )
            ));

            executor.lose(&dispatch);
            let poll = tasks
                .consultation_tasks
                .try_completion(&mut executor)
                .expect("the exact loss is routed")
                .expect("one Consult batch owns the loss");
            assert!(matches!(
                poll,
                ResidentConsultationTaskOwnerPoll {
                    outcome,
                    ..
                } if matches!(
                    outcome,
                    crate::resident::consultation::service::task::ResidentConsultationTaskPoll::Lost(ref lost)
                        if lost.cancellation_state() == ResidentOperationTaskCancellationState::Notified
                )
            ));
        },
    );
}

#[test]
fn conflicting_consult_child_owners_stop_before_signing_or_custody() {
    with_authorized_cancellation(
        dispatched_state("conflicting-consult-child"),
        |authority, store, anchor, authorized, input, admission| {
            let dispatch = dispatched_task_key(&authorized);
            let first = consultation_task_batch_fixture_in_store(
                dispatch.store().clone(),
                "consultation/conflict-parent-a",
                "consultation/conflict-frontier-a",
                "conflict-child-a",
            );
            let second = consultation_task_batch_fixture_in_store(
                dispatch.store().clone(),
                "consultation/conflict-parent-b",
                "consultation/conflict-frontier-b",
                "conflict-child-b",
            );
            let second_candidate = second.candidates()[0].clone();
            let mut tasks = task_state(2);
            let mut executor = HeldConsultChildExecutor::default();
            tasks.seed_consultation_dispatch_for_test(first, dispatch.clone(), 1, &mut executor);
            let second_coordinate = tasks
                .consultation_tasks
                .open_batch(
                    second,
                    ResidentConsultationTaskLimits::new(
                        NonZeroUsize::new(1).unwrap(),
                        NonZeroUsize::new(1).unwrap(),
                    ),
                )
                .expect("second hostile Consult batch opens");
            tasks
                .consultation_tasks
                .seed_conflicting_dispatch_owner_for_test(
                    &second_coordinate,
                    second_candidate,
                    dispatch,
                    2,
                    &mut executor,
                )
                .expect("hostile fixture creates the duplicate exact owner");

            let observation_signs = Cell::new(0);
            let checkpoint_signs = Cell::new(0);
            let original_observation_signer = admission.sign_observation;
            let original_checkpoint_signer = admission.sign_checkpoint;
            let mut sign_observation =
                |bytes: &[u8],
                 preimage: &OperationObservationPreimageV1,
                 digest: &OperationObservationDigest| {
                    observation_signs.set(observation_signs.get() + 1);
                    original_observation_signer(bytes, preimage, digest)
                };
            let mut sign_checkpoint =
                |bytes: &[u8],
                 preimage: &ControlStateCheckpointPreimageV1,
                 digest: &ControlStateCheckpointDigest| {
                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                    original_checkpoint_signer(bytes, preimage, digest)
                };
            let mut counted_admission = CancellationAppendAdmission {
                resolve_store: admission.resolve_store,
                admit_rotation: admission.admit_rotation,
                sign_observation: &mut sign_observation,
                sign_checkpoint: &mut sign_checkpoint,
            };
            let outcome = ResidentOperationService::new(authority, store, anchor, &mut tasks)
                .retain_authorized_cancellation(authorized, input, &mut counted_admission);
            assert!(matches!(
                outcome,
                Err(ResidentOperationCancellationFailure::DispatchOwnerConflict)
            ));
            assert_eq!((observation_signs.get(), checkpoint_signs.get()), (0, 0));
        },
    );
}

#[test]
fn anchor_retry_notifies_the_exact_consult_child_only_after_custody_completes() {
    with_owned_dispatched_cancellation_custody(
        &[],
        true,
        |authority, store, anchor, authorized, input, mut admission| {
            let dispatch = dispatched_task_key(&authorized);
            let batch = consultation_task_batch_fixture_in_store(
                dispatch.store().clone(),
                "consultation/retry-parent",
                "consultation/retry-frontier",
                "retry-child",
            );
            let mut tasks = task_state(1);
            let mut executor = HeldConsultChildExecutor::default();
            tasks.seed_consultation_dispatch_for_test(batch, dispatch.clone(), 11, &mut executor);

            let pending = match ResidentOperationService::new(authority, store, anchor, &mut tasks)
                .retain_authorized_cancellation(authorized, input, &mut admission)
            {
                Err(ResidentOperationCancellationFailure::DispatchedCustody(pending)) => pending,
                _ => panic!("the first anchor attempt must retain a linear continuation"),
            };
            let outcome = ResidentOperationService::new(authority, store, anchor, &mut tasks)
                .continue_dispatched_cancellation_custody(pending)
                .unwrap_or_else(|_| panic!("the retained anchor continuation completes"));
            assert!(matches!(
                outcome,
                ResidentOperationCancellationOutcome::DispatchedRequestRetained {
                    notification: ResidentOperationTaskCancellationNotification::Notified,
                    ..
                }
            ));

            executor.complete(&dispatch, 23);
            let after_retry = tasks
                .consultation_tasks
                .try_completion(&mut executor)
                .expect("the exact completion remains attributable")
                .expect("one Consult batch owns the completion");
            assert!(matches!(
                after_retry,
                ResidentConsultationTaskOwnerPoll {
                    outcome,
                    ..
                } if matches!(
                    outcome,
                    crate::resident::consultation::service::task::ResidentConsultationTaskPoll::Completed {
                        cancellation: ResidentOperationTaskCancellationState::Notified,
                        ..
                    }
                )
            ));
        },
    );
}

#[test]
fn child_completion_before_anchor_retry_remains_unnotified() {
    with_owned_dispatched_cancellation_custody(
        &[],
        true,
        |authority, store, anchor, authorized, input, mut admission| {
            let dispatch = dispatched_task_key(&authorized);
            let batch = consultation_task_batch_fixture_in_store(
                dispatch.store().clone(),
                "consultation/delayed-parent",
                "consultation/delayed-frontier",
                "delayed-child",
            );
            let mut tasks = task_state(1);
            let mut executor = HeldConsultChildExecutor::default();
            tasks.seed_consultation_dispatch_for_test(batch, dispatch.clone(), 12, &mut executor);

            let pending = match ResidentOperationService::new(authority, store, anchor, &mut tasks)
                .retain_authorized_cancellation(authorized, input, &mut admission)
            {
                Err(ResidentOperationCancellationFailure::DispatchedCustody(pending)) => pending,
                _ => panic!("the first anchor attempt must retain a linear continuation"),
            };
            executor.complete(&dispatch, 29);
            let completed = tasks
                .consultation_tasks
                .try_completion(&mut executor)
                .expect("the completion remains attributable before retry")
                .expect("one Consult batch owns the completion");
            assert!(matches!(
                completed,
                ResidentConsultationTaskOwnerPoll {
                    outcome,
                    ..
                } if matches!(
                    outcome,
                    crate::resident::consultation::service::task::ResidentConsultationTaskPoll::Completed {
                        cancellation: ResidentOperationTaskCancellationState::Unnotified,
                        ..
                    }
                )
            ));

            let outcome = ResidentOperationService::new(authority, store, anchor, &mut tasks)
                .continue_dispatched_cancellation_custody(pending)
                .unwrap_or_else(|_| panic!("the retained anchor continuation completes"));
            assert!(matches!(
                outcome,
                ResidentOperationCancellationOutcome::DispatchedRequestRetained {
                    notification: ResidentOperationTaskCancellationNotification::NotAccepted,
                    ..
                }
            ));
        },
    );
}
