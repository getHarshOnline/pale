//! Private process-lifetime ownership for resident operation task state.
//!
//! This aggregate accepts already constructed bounded operation-task ownership
//! and only lends short-lived operation facades over caller-selected authority
//! and custody ports. It selects no deployment application, cold-open policy,
//! task limits, executor, transport, signer, anchor, writer, fence, or outer
//! process lifecycle.

use pale_core::records::UnixTimeV1;
use pale_ports::{AntiRollbackAnchorPort, OperationCustodyPort};

use super::consultation::{
    CompletedResidentConsultationNonterminal, CompletedResidentConsultationTerminal,
    ConsultationChildDispatchCandidate, ConsultationChildRetentionOutcome,
    PreparedResidentConsultationTaskLoss, PreparedResidentConsultationTaskResume,
    RecoveredConsultationCollection, ResidentConsultationBatchCoordinate,
    ResidentConsultationBatchOpenError, ResidentConsultationJoinedTerminal,
    ResidentConsultationNonterminalSettlementFailure, ResidentConsultationPolicyFrontier,
    ResidentConsultationPrePolicyTurn, ResidentConsultationPreparedResumeFailure,
    ResidentConsultationSupervisionDecision, ResidentConsultationTaskLimits,
    ResidentConsultationTaskLossCommitFailure, ResidentConsultationTaskLossOffer,
    ResidentConsultationTaskLossPreparationFailure, ResidentConsultationTaskOwnerPoll,
    ResidentConsultationTaskResumePreparationFailure, ResidentConsultationTaskRetry,
    ResidentConsultationTaskRetryFailure, ResidentConsultationTerminalJoinFailure,
    ResidentConsultationTurnDisposition,
};
use super::operation_dispatch::OwnedProviderInvocationRequest;
use super::operation_service::{ResidentOperationService, ResidentOperationTaskState};
use super::provider_task_service::ResidentProviderCompletionPoll;
use super::task_owner::{
    ResidentOperationTaskCompletion, ResidentOperationTaskExecutor,
    ResidentProviderAcceptedAggregateTransition, ResidentProviderTaskRefusal,
};
use super::RecoveredOperationObservation;
use crate::provider_invocation::ProviderInvocationAdmissionError;
use pale_ports::ProviderInvocationObservation;

/// Smallest private process-lifetime aggregate for resident operation work.
pub(in crate::resident) struct ResidentOperationHost {
    operation_tasks: ResidentOperationTaskState,
}

impl ResidentOperationHost {
    /// Return all provider, Consult, and A2A work consuming process capacity.
    #[must_use]
    pub(in crate::resident) fn outstanding_task_reservations(&self) -> usize {
        self.operation_tasks.outstanding_task_reservations()
    }

    /// Return exact Consult batches retained by this process owner.
    #[must_use]
    pub(in crate::resident) fn active_consultation_batches(&self) -> usize {
        self.operation_tasks.active_consultation_batches()
    }

    /// Return whether this process owns the exact finite Consult batch.
    #[must_use]
    pub(in crate::resident) fn retains_consultation_batch(
        &self,
        expected: &super::consultation::ResidentConsultationBatchCoordinate,
    ) -> bool {
        self.operation_tasks.retains_consultation_batch(expected)
    }

    /// Return accepted Consult children that still await a join.
    #[must_use]
    pub(in crate::resident) fn in_flight_consultation_tasks(&self) -> usize {
        self.operation_tasks.in_flight_consultation_tasks()
    }

    /// Own already bounded process-wide operation task state.
    #[must_use]
    pub(in crate::resident) const fn new(operation_tasks: ResidentOperationTaskState) -> Self {
        Self { operation_tasks }
    }

    /// Return the exact provider-route capacity owned by this task aggregate.
    #[must_use]
    pub(in crate::resident) const fn provider_task_limits(
        &self,
    ) -> super::task_owner::ResidentOperationTaskLimits {
        self.operation_tasks.provider_task_limits()
    }

    /// Return the shared process-local occupancy ceiling.
    #[must_use]
    pub(in crate::resident) const fn aggregate_task_limits(
        &self,
    ) -> super::task_reservation::ResidentAggregateTaskLimits {
        self.operation_tasks.aggregate_task_limits()
    }

    /// Borrow the process owner used to fence all operation-task capabilities.
    #[must_use]
    pub(in crate::resident) const fn operation_task_owner_instance(
        &self,
    ) -> &super::task_owner::ResidentOperationTaskOwnerInstance {
        self.operation_tasks.operation_task_owner_instance()
    }

    /// Poll one provider completion without borrowing custody or anchor ports.
    pub(in crate::resident) fn poll_provider_task<Executor>(
        &mut self,
        executor: &mut Executor,
    ) -> Option<ResidentProviderCompletionPoll>
    where
        Executor: ResidentOperationTaskExecutor<
            OwnedProviderInvocationRequest,
            Result<ProviderInvocationObservation, ProviderInvocationAdmissionError>,
        >,
    {
        self.operation_tasks.poll_provider_task(executor)
    }

    /// Retry only an aggregate join for returned provider work.
    pub(in crate::resident) fn retry_provider_completion_transition(
        &mut self,
        completion: super::task_owner::ResidentOperationTaskCompletion<
            OwnedProviderInvocationRequest,
            Result<ProviderInvocationObservation, ProviderInvocationAdmissionError>,
        >,
    ) -> ResidentProviderCompletionPoll {
        self.operation_tasks
            .retry_provider_completion_transition(completion)
    }

    /// Retry only the aggregate join for an already-returned Consult result.
    pub(in crate::resident) fn retry_consultation_completion_transition<Work, Output>(
        &mut self,
        completion: ResidentOperationTaskCompletion<Work, Output>,
    ) -> Result<
        ResidentConsultationTaskOwnerPoll<Work, Output>,
        super::consultation::ResidentConsultationTaskOwnerPollError<Work, Output>,
    > {
        self.operation_tasks
            .retry_consultation_completion_transition(completion)
    }

    /// Repair only aggregate occupancy after provider acceptance already won.
    ///
    /// This path performs no executor, custody, anchor, or signer I/O.
    pub(in crate::resident) fn continue_accepted_provider_aggregate_transition<'operation>(
        &mut self,
        refusal: ResidentProviderTaskRefusal<'operation>,
    ) -> Result<
        ResidentProviderAcceptedAggregateTransition<'operation>,
        ResidentProviderTaskRefusal<'operation>,
    > {
        self.operation_tasks
            .continue_accepted_provider_aggregate_transition(refusal)
    }

    /// Repair only aggregate occupancy after unopened provider work was refused.
    ///
    /// This path performs no executor, custody, anchor, or signer I/O.
    pub(in crate::resident) fn continue_refused_provider_aggregate_transition<'operation>(
        &mut self,
        refusal: ResidentProviderTaskRefusal<'operation>,
    ) -> Result<ResidentProviderTaskRefusal<'operation>, ResidentProviderTaskRefusal<'operation>>
    {
        self.operation_tasks
            .continue_refused_provider_aggregate_transition(refusal)
    }

    /// Lend one short-lived operation facade over caller-owned ports.
    ///
    /// The closure cannot retain the facade beyond this call. The facade is
    /// dropped before the result is returned, while process-local task state
    /// remains owned by this aggregate for the next borrow.
    pub(in crate::resident) fn with_operation_service<Authority, Output>(
        &mut self,
        authority: &mut Authority,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        use_service: impl FnOnce(&mut ResidentOperationService<'_, Authority>) -> Output,
    ) -> Output {
        let output = {
            let mut service = ResidentOperationService::new(
                authority,
                custody,
                anchor,
                &mut self.operation_tasks,
            );
            use_service(&mut service)
        };
        output
    }

    /// Retain one finite Consult batch without borrowing physical ports.
    pub(in crate::resident) fn open_consultation_batch(
        &mut self,
        planned: super::ConsultationChildDispatchBatch,
        limits: ResidentConsultationTaskLimits,
    ) -> Result<ResidentConsultationBatchCoordinate, ResidentConsultationBatchOpenError> {
        self.operation_tasks
            .open_consultation_batch(planned, limits)
    }

    /// Reconcile one recovered Consult roster with its process-local owner.
    pub(in crate::resident) fn reconcile_consultation_batch(
        &mut self,
        planned: super::ConsultationChildDispatchBatch,
        limits: ResidentConsultationTaskLimits,
    ) -> Result<
        super::consultation::service::task::ResidentConsultationBatchReconciliation,
        super::consultation::service::task::ResidentConsultationBatchReconcileError,
    > {
        self.operation_tasks
            .reconcile_consultation_batch(planned, limits)
    }

    /// Run one recovered Consult turn without borrowing custody or anchor ports.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub(in crate::resident) fn supervise_recovered_consultation_turn<
        Work,
        Output,
        RecoveryError,
        ChildRecoveryError,
        RecoverChild,
        RetentionFailure,
        Retain,
        Executor,
        StopInput,
        Decide,
    >(
        &mut self,
        task_limits: ResidentConsultationTaskLimits,
        observed_at: UnixTimeV1,
        current: RecoveredConsultationCollection,
        executor: &mut Executor,
        decide: Decide,
        recover_child: RecoverChild,
        retain: Retain,
    ) -> ResidentConsultationTurnDisposition<
        Work,
        Output,
        RecoveryError,
        ChildRecoveryError,
        RetentionFailure,
        StopInput,
    >
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
        Decide: FnOnce(
            &RecoveredConsultationCollection,
            UnixTimeV1,
        ) -> ResidentConsultationSupervisionDecision<StopInput>,
        RecoverChild: FnOnce(
            &ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, ChildRecoveryError>,
        Retain: FnOnce(
            &ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
    {
        self.operation_tasks.supervise_recovered_consultation_turn(
            task_limits,
            observed_at,
            current,
            executor,
            decide,
            recover_child,
            retain,
        )
    }

    /// Advance one recovered Consult through bounded mechanical work only.
    ///
    /// This reconciles at most one finite batch and polls at most once. It does
    /// not select a child, infer a stopping rule, or perform terminal custody.
    pub(in crate::resident) fn supervise_recovered_consultation_pre_policy_turn<
        Work,
        Output,
        RecoveryError,
        Executor,
    >(
        &mut self,
        task_limits: ResidentConsultationTaskLimits,
        observed_at: UnixTimeV1,
        current: RecoveredConsultationCollection,
        executor: &mut Executor,
    ) -> ResidentConsultationPrePolicyTurn<Work, Output, RecoveryError>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        self.operation_tasks
            .supervise_recovered_consultation_pre_policy_turn(
                task_limits,
                observed_at,
                current,
                executor,
            )
    }

    /// Apply one selected decision to the exact existing Consult frontier.
    ///
    /// This route does not recover the collection, reconcile its batch, poll
    /// the executor, or select policy.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub(in crate::resident) fn apply_consultation_supervision_decision<
        Work,
        Output,
        RecoveryError,
        ChildRecoveryError,
        RecoverChild,
        RetentionFailure,
        Retain,
        Executor,
        StopInput,
    >(
        &mut self,
        frontier: Box<ResidentConsultationPolicyFrontier>,
        decision: ResidentConsultationSupervisionDecision<StopInput>,
        executor: &mut Executor,
        recover_child: RecoverChild,
        retain: Retain,
    ) -> ResidentConsultationTurnDisposition<
        Work,
        Output,
        RecoveryError,
        ChildRecoveryError,
        RetentionFailure,
        StopInput,
    >
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
        RecoverChild: FnOnce(
            &ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, ChildRecoveryError>,
        Retain: FnOnce(
            &ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
    {
        self.operation_tasks
            .apply_consultation_supervision_decision(
                frontier,
                decision,
                executor,
                recover_child,
                retain,
            )
    }

    /// Prepare exact Consult ownership before recovered provider preflight.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub(in crate::resident) fn prepare_resumed_consultation_task(
        &self,
        coordinate: &super::consultation::ResidentConsultationBatchCoordinate,
        candidate: &super::consultation::ConsultationChildDispatchCandidate,
        current: &super::consultation::RecoveredConsultationCollection,
        observed_at: pale_core::records::UnixTimeV1,
        child_store: &pale_ports::AuthorityStoreCoordinate,
        child_operation: &super::IssuedLocalOperation,
        dispatch_ref: &pale_core::records::ResidentRecordRefV1,
        dispatch: &pale_core::records::OperationObservationV1,
    ) -> Result<
        PreparedResidentConsultationTaskResume,
        ResidentConsultationTaskResumePreparationFailure,
    > {
        self.operation_tasks.prepare_resumed_consultation_task(
            coordinate,
            candidate,
            current,
            observed_at,
            child_store,
            child_operation,
            dispatch_ref,
            dispatch,
        )
    }

    /// Offer work only through its already prepared resumed Consult owner.
    pub(in crate::resident) fn accept_prepared_resumed_consultation_task<Work, Output, Executor>(
        &mut self,
        prepared: PreparedResidentConsultationTaskResume,
        work: Work,
        executor: &mut Executor,
    ) -> Result<
        super::task_owner::ResidentOperationTaskKey,
        ResidentConsultationPreparedResumeFailure<Work>,
    >
    where
        Executor: super::task_owner::ResidentOperationTaskExecutor<Work, Output>,
    {
        self.operation_tasks
            .accept_prepared_resumed_consultation_task(prepared, work, executor)
    }

    /// Prepare one historical Consult dispatch for executor-free loss handling.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn prepare_recovered_consultation_loss(
        &self,
        coordinate: &ResidentConsultationBatchCoordinate,
        candidate: &ConsultationChildDispatchCandidate,
        current: &RecoveredConsultationCollection,
        observed_at: UnixTimeV1,
        child_store: &pale_ports::AuthorityStoreCoordinate,
        child_operation: &super::IssuedLocalOperation,
        dispatch_ref: &pale_core::records::ResidentRecordRefV1,
        dispatch: &pale_core::records::OperationObservationV1,
    ) -> Result<PreparedResidentConsultationTaskLoss, ResidentConsultationTaskLossPreparationFailure>
    {
        self.operation_tasks.prepare_recovered_consultation_loss(
            coordinate,
            candidate,
            current,
            observed_at,
            child_store,
            child_operation,
            dispatch_ref,
            dispatch,
        )
    }

    pub(in crate::resident) fn commit_recovered_consultation_loss<Work, Output>(
        &mut self,
        prepared: PreparedResidentConsultationTaskLoss,
    ) -> Result<
        ResidentConsultationTaskOwnerPoll<Work, Output>,
        ResidentConsultationTaskLossCommitFailure,
    > {
        self.operation_tasks
            .commit_recovered_consultation_loss(prepared)
    }

    pub(in crate::resident) fn offer_recovered_consultation_loss(
        &mut self,
        prepared: PreparedResidentConsultationTaskLoss,
    ) -> Result<ResidentConsultationTaskLossOffer, ResidentConsultationTaskLossCommitFailure> {
        self.operation_tasks
            .offer_recovered_consultation_loss(prepared)
    }

    pub(in crate::resident) fn retry_recovered_consultation_loss<Work, Output>(
        &mut self,
        pending: ResidentConsultationTaskLossOffer,
    ) -> Result<
        ResidentConsultationTaskOwnerPoll<Work, Output>,
        ResidentConsultationTaskLossCommitFailure,
    > {
        self.operation_tasks
            .retry_recovered_consultation_loss(pending)
    }

    /// Retry only unopened Consult work returned by the exact task owner.
    pub(in crate::resident) fn retry_consultation_task<Work, Output, Executor>(
        &mut self,
        retry: ResidentConsultationTaskRetry<Work>,
        executor: &mut Executor,
    ) -> Result<
        super::task_owner::ResidentOperationTaskKey,
        ResidentConsultationTaskRetryFailure<Work>,
    >
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        let coordinate = retry.coordinate().clone();
        self.operation_tasks
            .retry_consultation_task(&coordinate, retry, executor)
    }

    /// Retry only aggregate release for a Consult join already in custody.
    pub(in crate::resident) fn retry_consultation_terminal_release<Work, Output>(
        &mut self,
        joined: ResidentConsultationJoinedTerminal<Work, Output>,
    ) -> Result<
        CompletedResidentConsultationTerminal<Work, Output>,
        ResidentConsultationTerminalJoinFailure<Work, Output>,
    > {
        self.operation_tasks
            .retry_consultation_terminal_release(joined)
    }

    /// Release one completed or lost Consult task after durable uncertainty.
    pub(in crate::resident) fn settle_consultation_nonterminal<Work, Output>(
        &mut self,
        ready: super::consultation::ResidentConsultationCompletionReady<Work, Output>,
        child: &RecoveredOperationObservation,
    ) -> Result<
        CompletedResidentConsultationNonterminal<Work, Output>,
        ResidentConsultationNonterminalSettlementFailure<Work, Output>,
    > {
        self.operation_tasks
            .settle_consultation_nonterminal(ready, child)
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/operation_host.rs"]
mod tests;
