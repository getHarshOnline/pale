//! Deployment-bound continuation of dispatched cancellation custody.

use std::collections::VecDeque;

use pale_ports::{
    ControlStoreError, SessionLeaseBody, SessionLeaseBodyRead, SessionLeaseBodyRetention,
    SessionLeaseCustodyPort,
};

use super::*;
use crate::resident::consultation::tests::consultation_task_batch_fixture_in_store;
use crate::resident::operation_service::tests::with_owned_dispatched_cancellation_custody;
use crate::resident::task_owner::{
    ResidentOperationTask, ResidentOperationTaskCancellationNotification,
    ResidentOperationTaskCompletion, ResidentOperationTaskExecutor,
};

#[derive(Default)]
struct HeldConsultChildExecutor {
    accepted: Vec<ResidentOperationTask<u8>>,
    completions: VecDeque<ResidentOperationTaskCompletion<u8, u16>>,
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

struct UnusedSessions;

impl SessionLeaseCustodyPort for UnusedSessions {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("a dispatched-cancellation continuation does not renew a session")
    }

    fn read(
        &self,
        _: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        panic!("a dispatched-cancellation continuation does not recover a session")
    }
}

#[test]
fn dispatched_cancellation_retry_rechecks_deployment_before_notifying_consult_child() {
    with_owned_dispatched_cancellation_custody(
        &[],
        true,
        |authority, custody, anchor, authorized, input, mut admission| {
            let selected = authorized.recovered_observation().selected_head();
            let selected_store = authorized.recovered_observation().store().clone();
            let selector = selector_from_signing_authority(input.signing.signing_authority);
            let dispatch = ResidentOperationTaskKey::new(
                selected_store.clone(),
                ImmutableRecordReferenceV1 {
                    record_ref: selected.0.clone(),
                    record_sha256: ArtifactDigest::parse(
                        selected.1.canonical_operation_observation_sha256().as_str(),
                    )
                    .expect("issued observation has an artifact digest"),
                },
            );
            let batch = consultation_task_batch_fixture_in_store(
                selected_store.clone(),
                "consultation/host-retry-parent",
                "consultation/host-retry-frontier",
                "host-retry-child",
            );
            let mut tasks = ResidentOperationTaskState::new(
                ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                NonZeroUsize::new(1).unwrap(),
                ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            )
            .expect("focused process-host task owner");
            let mut executor = HeldConsultChildExecutor::default();
            tasks.seed_consultation_dispatch_for_test(batch, dispatch, 7, &mut executor);

            let lower = ResidentOperationService::new(authority, custody, anchor, &mut tasks)
                .retain_authorized_cancellation(authorized, input, &mut admission);
            let wrapped = match bind_cancellation_custody(&selector, lower) {
                Err(ResidentProcessCancellationError::DispatchedCustody(pending)) => pending,
                _ => panic!("the host classifies the retained dispatched custody"),
            };
            let mut retention = ProgramCancellationRetentionProbe::default();
            assert!(crate::resident::process_host::program::settle_cancellation_handoff(
                &mut retention,
                (),
                crate::resident::process_host::program::ResidentProgramCancellationHandoff::DispatchedCustody(wrapped),
            )
            .is_err());
            assert_eq!(retention.retention_calls, 1);
            let wrapped = retention
                .dispatched
                .take()
                .expect("the exact dispatched custody is retained once");

            let application_port = MemoryAuthorityStore::default();
            let application = ready_application(&application_port, selected_store.clone(), 1);
            let deployment =
                unproved_deployment_for_selector(&selector, selected_store.clone(), application);
            let writer =
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]);
            let mut host = ResidentProcessHost::new(
                writer,
                vec![deployment],
                NonZeroUsize::new(1).unwrap(),
                ResidentOperationHost::new(tasks),
            )
            .expect("one exact dispatched-cancellation deployment");
            let foreign_store = AuthorityStoreCoordinate::new(
                ControlStoreIdV1::parse("foreign-continuation-store").unwrap(),
                selected_store.incarnation_id().clone(),
            );
            let failed = match host.continue_dispatched_cancellation_custody(
                ResidentDeploymentOperationAccess {
                    store: &foreign_store,
                    custody,
                    sessions: &UnusedSessions,
                    anchor,
                },
                authority,
                wrapped,
            ) {
                Err(failed) => failed,
                Ok(_) => panic!("an unlike access store cannot receive the continuation"),
            };
            assert!(matches!(
                failed.reason,
                ResidentProcessDispatchedCancellationContinuationError::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation
                )
            ));

            let outcome = host
                .continue_dispatched_cancellation_custody(
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody,
                        sessions: &UnusedSessions,
                        anchor,
                    },
                    authority,
                    failed.pending,
                )
                .unwrap_or_else(|_| panic!("the exact selector-bound continuation completes"));
            assert!(matches!(
                outcome,
                ResidentProcessCancellationOutcome::Operation(
                    ResidentOperationCancellationOutcome::DispatchedRequestRetained {
                        notification: ResidentOperationTaskCancellationNotification::Notified,
                        ..
                    }
                )
            ));
            assert_eq!(executor.accepted.len(), 1);
        },
    );
}
