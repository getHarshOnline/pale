//! Durable request preparation precedes the one-shot `GetTask` carrier release.

use super::*;
use crate::local_operation_start_fixture::{record_ref, signature};
use crate::resident::operation_dispatch::selection::tests::a2a_readback_service::{
    exchange_limits, with_recovered_a2a_task_history, A2aTaskHistoryFixture, Clock, GetTaskPort,
};

impl DatabaseCommittedA2aGetTaskAnchorPending<'_> {
    pub(in crate::resident) const fn database_evidence(
        &self,
    ) -> crate::resident::OperationCustodyDatabaseEvidence {
        self.physical.database_evidence()
    }
}

impl A2aGetTaskCustodyFailure<'_> {
    pub(in crate::resident) const fn is_recovery_only(&self) -> bool {
        matches!(self.phase, A2aGetTaskCustodyFailurePhase::RecoveryOnly(_))
    }
}

impl<'a> RetainedA2aGetTask<'a> {
    /// Recheck current disclosure and the retained deadline before one start.
    pub(in crate::resident) fn start<Authorize>(
        self,
        port: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
    ) -> Result<InFlightA2aGetTaskReadback, A2aGetTaskStartFailure<'a>>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        self.start_retained(port, clock, authorize)
            .map(|started| started.in_flight)
    }
}

fn checkpoint_authority(
    checkpoint: &ControlStateCheckpointV1,
) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
    ControlCheckpointSigningAuthorityAdmissionContextV1 {
        authority_ref: checkpoint.checkpoint_signing_authority_ref(),
        authority_sha256: checkpoint.checkpoint_signing_authority_sha256(),
        principal_id: checkpoint.issuer_principal_id(),
        control_store_id: checkpoint.control_store_id(),
        control_incarnation_id: checkpoint.control_incarnation_id(),
        checkpoint_domain_id: checkpoint.checkpoint_domain_id(),
        checkpoint_profile_ref: checkpoint.checkpoint_profile_ref(),
        checkpoint_profile_sha256: checkpoint.checkpoint_profile_sha256(),
        subject_key_id: checkpoint.issuer_key_id(),
        subject_key_epoch: *checkpoint.issuer_key_epoch(),
    }
}

#[test]
fn carrier_starts_once_only_after_new_database_and_anchor_custody() {
    with_recovered_a2a_task_history(
        A2aTaskHistoryFixture {
            expected_callback_appends: 1,
            ..A2aTaskHistoryFixture::default()
        },
        |history, attempt, _, _, _, custody, anchor, _, _| {
            let port = GetTaskPort::default();
            let request_id = A2aJsonRpcRequestIdV1::parse("get-task-custody-1").unwrap();
            let operation = attempt.operation;
            let selection = attempt.operation.checkpoint_selection();
            let prepared = prepare_a2a_get_task_custody(
                A2aGetTaskPreparationInput {
                    readback: A2aGetTaskReadbackInput {
                        history,
                        attempt,
                        request_id: &request_id,
                        request_record_ref: record_ref("a2a/get-task-custody-request"),
                        response_record_ref: record_ref("a2a/get-task-custody-response"),
                        limits: exchange_limits(),
                        maximum_evidence_bytes: 8_192,
                    },
                    preparation_ref: record_ref("a2a/get-task-custody-preparation"),
                    current_predecessor: OperationCustodyIndexPredecessor::Absent,
                    maximum_record_bytes: 65_536,
                },
                &port,
                &Clock::at(65),
                |_| Ok(()),
            )
            .expect("prepare request and private custody bytes");
            assert_eq!(port.prepares(), 1);
            assert_eq!(port.starts(), 0);

            let current_checkpoint = history.local_observation().current_checkpoint().1;
            let current_checkpoint_ref = history.local_observation().current_checkpoint().0;
            let authority = checkpoint_authority(current_checkpoint);
            let prepared = prepared
                .prepare_checkpoint(
                    selection,
                    current_checkpoint_ref,
                    current_checkpoint,
                    AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000).unwrap(),
                    AccumulatorMembershipLimits::new(8, 20_000, 200_000).unwrap(),
                    custody,
                )
                .expect("prepare exact roots and retained membership")
                .issue(
                    OperationCheckpointInput {
                        reference: record_ref("a2a/get-task-custody-checkpoint"),
                        id: ControlCheckpointIdV1::parse("get-task-custody-checkpoint").unwrap(),
                    },
                    UnixTimeV1::new(66).unwrap(),
                    current_checkpoint.schema_sha256(),
                    &authority,
                    |_, _, _| Ok(signature()),
                )
                .expect("close physical preparation candidate");
            assert_eq!(port.starts(), 0);

            let database = match prepared.commit(custody, anchor) {
                Ok(database) => database,
                Err(_) => panic!("new database commit must succeed"),
            };
            assert_eq!(
                database.database_evidence(),
                crate::resident::OperationCustodyDatabaseEvidence::New
            );
            assert_eq!(port.starts(), 0);
            let retained = match database.advance_anchor(anchor) {
                Ok(retained) => retained,
                Err(_) => panic!("new preparation must anchor"),
            };
            assert_eq!(port.starts(), 0);
            let recovered = recover_a2a_get_task_custody(history, operation, 65_536, custody)
                .expect("cold point reads recover private custody");
            let RecoveredA2aGetTaskCustody::Preparation(recovered) = recovered else {
                panic!("current private preparation must remain pending")
            };
            assert_eq!(
                recovered.record.schema_id().as_str(),
                PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA
            );
            assert_eq!(
                recovered.checkpoint.coordinate(),
                retained
                    .custody()
                    .candidate()
                    .checkpoint()
                    .successor_record()
                    .coordinate()
            );
            assert_eq!(port.starts(), 0);

            let retained = match retained.start(&port, &Clock::at(10_000), |_| Ok(())) {
                Err(A2aGetTaskStartFailure::Preflight { retained, .. }) => *retained,
                _ => panic!("expired fresh preflight must retain the unopened release"),
            };
            assert_eq!(port.starts(), 0);
            let _in_flight = match retained.start(&port, &Clock::at(65), |_| Ok(())) {
                Ok(in_flight) => in_flight,
                Err(_) => panic!("one fresh start must follow durable custody"),
            };
            assert_eq!(port.starts(), 1);
            assert_eq!(port.prepares(), 1);
            true
        },
    );
}

#[test]
fn reconciled_database_commit_cannot_restore_carrier_authority() {
    with_recovered_a2a_task_history(
        A2aTaskHistoryFixture {
            expected_callback_appends: 1,
            additional_read_backs: 1,
            unknown_exact_commits: vec![2],
            ..A2aTaskHistoryFixture::default()
        },
        |history, attempt, _, _, _, custody, anchor, _, _| {
            let port = GetTaskPort::default();
            let request_id = A2aJsonRpcRequestIdV1::parse("get-task-custody-uncertain").unwrap();
            let operation = attempt.operation;
            let selection = operation.checkpoint_selection();
            let prepared = prepare_a2a_get_task_custody(
                A2aGetTaskPreparationInput {
                    readback: A2aGetTaskReadbackInput {
                        history,
                        attempt,
                        request_id: &request_id,
                        request_record_ref: record_ref("a2a/get-task-uncertain-request"),
                        response_record_ref: record_ref("a2a/get-task-uncertain-response"),
                        limits: exchange_limits(),
                        maximum_evidence_bytes: 8_192,
                    },
                    preparation_ref: record_ref("a2a/get-task-uncertain-preparation"),
                    current_predecessor: OperationCustodyIndexPredecessor::Absent,
                    maximum_record_bytes: 65_536,
                },
                &port,
                &Clock::at(65),
                |_| Ok(()),
            )
            .expect("prepare request and private custody bytes");
            let current_checkpoint = history.local_observation().current_checkpoint().1;
            let current_checkpoint_ref = history.local_observation().current_checkpoint().0;
            let authority = checkpoint_authority(current_checkpoint);
            let prepared = prepared
                .prepare_checkpoint(
                    selection,
                    current_checkpoint_ref,
                    current_checkpoint,
                    AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000).unwrap(),
                    AccumulatorMembershipLimits::new(8, 20_000, 200_000).unwrap(),
                    custody,
                )
                .expect("prepare exact roots and retained membership")
                .issue(
                    OperationCheckpointInput {
                        reference: record_ref("a2a/get-task-uncertain-checkpoint"),
                        id: ControlCheckpointIdV1::parse("get-task-uncertain-checkpoint").unwrap(),
                    },
                    UnixTimeV1::new(66).unwrap(),
                    current_checkpoint.schema_sha256(),
                    &authority,
                    |_, _, _| Ok(signature()),
                )
                .expect("close physical preparation candidate");

            let database = match prepared.commit(custody, anchor) {
                Ok(database) => database,
                Err(_) => panic!("one exact read-back must reconcile the unknown commit"),
            };
            assert_eq!(
                database.database_evidence(),
                crate::resident::OperationCustodyDatabaseEvidence::RecoveredExactAfterAmbiguousCommit
            );
            assert_eq!(port.starts(), 0);
            let recovered = match database.advance_anchor(anchor) {
                Err(recovered) => recovered,
                Ok(_) => panic!("reconciled commit cannot restore carrier authority"),
            };
            assert!(recovered.is_recovery_only());
            assert_eq!(port.starts(), 0);
            let recovered = match recovered.continue_exact(custody, anchor) {
                Err(recovered) => recovered,
                Ok(_) => panic!("recovery-only custody remains terminal for release authority"),
            };
            assert!(recovered.is_recovery_only());
            assert_eq!(port.starts(), 0);
            assert_eq!(port.prepares(), 1);
            true
        },
    );
}
