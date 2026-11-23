//! Private resident ownership of retained provider work.
//!
//! Durable dispatch remains authoritative. This service joins one retained
//! dispatch to the bounded process-local task owner, then returns raw provider
//! evidence to the existing local-operation coordinator for single-writer recording.
//! It cannot select a provider, replay work after loss, or infer that a
//! process-local cancellation notification stopped an accepted call.

use pale_core::records::*;
use pale_ports::ProviderInvocationObservation;

use super::local_operation_coordinator::{
    ResidentLocalOperationCoordinator, ResidentProviderRecordingResumeFailure,
};
use super::operation_dispatch::{
    ConsumedProviderRecordingContinuation, OwnedProviderInvocationRequest,
    ProviderOperationContinuationCoordinate, ProviderOperationInvokeFailure,
    ProviderOperationInvokeInput, ProviderOperationInvokeResult,
    ProviderOperationRecordingConflict, ProviderOperationRecordingCustody,
    ProviderOperationRecordingFailure, ProviderOperationRecordingFinalization,
    ProviderOperationRecordingPreparationError, RecoveredProviderCallPreparationFailure,
    RecoveredProviderDispatchRelease, RecoveredProviderUnknownRecordingFailure,
    RetainedProviderOperationDispatch,
};
use super::task_owner::{
    ResidentOperationTaskCancellationNotification, ResidentOperationTaskCancellationState,
    ResidentOperationTaskCompletion, ResidentOperationTaskExecutor, ResidentOperationTaskKey,
    ResidentOperationTaskLimits, ResidentOperationTaskOwnerBuildError,
    ResidentOperationTaskOwnerInstance, ResidentProviderAcceptedAggregateTransition,
    ResidentProviderTaskJoin, ResidentProviderTaskOwner, ResidentProviderTaskRefusal,
    ResidentRecoveredProviderTaskRefusal,
};
use super::task_reservation::{
    ResidentAggregateTaskReservations, ResidentTaskReservationOfferFailure,
    ResidentTaskReservationSettlement, ResidentTaskReservationSettlementTransitionFailure,
    ResidentTaskReservationTransitionFailure,
};
use super::{
    AmbientSessionAuthorityPort, OperationAttemptError, OperationAttemptHistoryInput,
    RecoveredOperationObservation,
};
use crate::provider_invocation::{
    ProviderInvocationAdmissionContext, ProviderInvocationAdmissionError,
    ProviderInvocationEvidenceAdmission,
};

type ProviderTaskOutput = Result<ProviderInvocationObservation, ProviderInvocationAdmissionError>;

enum ResidentProviderSettlementKind {
    Completion,
    Loss,
}

/// Exact accepted provider task selected without changing process-local state.
#[must_use = "notify this exact owner only after durable cancellation custody"]
pub(in crate::resident) struct ResidentProviderDispatchOwner {
    key: ResidentOperationTaskKey,
}

/// Pure provider ownership result for one durable dispatch.
pub(in crate::resident) enum ResidentProviderDispatchOwnerSelection {
    Missing,
    Owned(ResidentProviderDispatchOwner),
}

/// Exact aggregate reservation retained across provider durable recording.
#[must_use = "release only after exact provider durable settlement"]
struct ResidentProviderPendingSettlement {
    key: ResidentOperationTaskKey,
    cancellation: ResidentOperationTaskCancellationState,
    settlement: ResidentTaskReservationSettlement,
    kind: ResidentProviderSettlementKind,
}

impl ResidentProviderPendingSettlement {
    fn require(
        &self,
        reservations: &ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        match self.kind {
            ResidentProviderSettlementKind::Completion => {
                reservations.require_provider_completion(&self.settlement, &self.key)
            }
            ResidentProviderSettlementKind::Loss => {
                reservations.require_provider_loss(&self.settlement, &self.key)
            }
        }
    }

    fn release(
        self,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentTaskReservationSettlementTransitionFailure> {
        match self.kind {
            ResidentProviderSettlementKind::Completion => {
                reservations.release_completion(self.settlement)
            }
            ResidentProviderSettlementKind::Loss => reservations.release_loss(self.settlement),
        }
    }
}

#[must_use = "continue recording without replaying provider work"]
pub(in crate::resident) struct ResidentProviderRecordingContinuation<'operation> {
    continuation: ConsumedProviderRecordingContinuation<'operation>,
    pending: ResidentProviderPendingSettlement,
}

#[must_use = "recover the exact provider head before continuing recording"]
pub(in crate::resident) struct ResidentProviderRecordingConflict<'operation> {
    conflict: ProviderOperationRecordingConflict<'operation>,
    pending: ResidentProviderPendingSettlement,
}

#[must_use = "continue exact provider candidate custody before aggregate release"]
pub(in crate::resident) struct ResidentProviderRecordingCustody {
    custody: ProviderOperationRecordingCustody,
    pending: ResidentProviderPendingSettlement,
}

#[must_use = "finish provider semantic admission before aggregate release"]
pub(in crate::resident) struct ResidentProviderRecordingFinalization {
    finalization: ProviderOperationRecordingFinalization,
    pending: ResidentProviderPendingSettlement,
}

#[must_use = "recover the unchanged provider recording continuation"]
pub(in crate::resident) struct ResidentProviderRecordingContinuationRejection<'operation> {
    pub(in crate::resident) continuation: Box<ResidentProviderRecordingContinuation<'operation>>,
    pub(in crate::resident) reason: ResidentTaskReservationTransitionFailure,
}

#[must_use = "recover the unchanged provider head-conflict capability"]
pub(in crate::resident) struct ResidentProviderRecordingConflictRejection<'operation> {
    pub(in crate::resident) conflict: Box<ResidentProviderRecordingConflict<'operation>>,
    pub(in crate::resident) reason: ResidentTaskReservationTransitionFailure,
}

#[must_use = "recover the unchanged provider custody continuation"]
pub(in crate::resident) struct ResidentProviderRecordingCustodyRejection {
    pub(in crate::resident) custody: Box<ResidentProviderRecordingCustody>,
    pub(in crate::resident) reason: ResidentTaskReservationTransitionFailure,
}

#[must_use = "recover the unchanged provider finalization token"]
pub(in crate::resident) struct ResidentProviderRecordingFinalizationRejection {
    pub(in crate::resident) finalization: Box<ResidentProviderRecordingFinalization>,
    pub(in crate::resident) reason: ResidentTaskReservationTransitionFailure,
}

/// Exact accepted completion rejected before protected recovery I/O.
#[must_use = "retry with the same completion and its owning aggregate ledger"]
pub(in crate::resident) struct ResidentProviderCompletionRecoveryRejection {
    pub(in crate::resident) completed: Box<ResidentCompletedProviderTask>,
    pub(in crate::resident) reason: ResidentTaskReservationTransitionFailure,
}

impl std::fmt::Debug for ResidentProviderRecordingContinuationRejection<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProviderRecordingContinuationRejection")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ResidentProviderRecordingConflictRejection<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProviderRecordingConflictRejection")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ResidentProviderRecordingCustodyRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProviderRecordingCustodyRejection")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ResidentProviderRecordingFinalizationRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProviderRecordingFinalizationRejection")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

/// Bounded process-local owner for provider work accepted in this resident lifetime.
pub(in crate::resident) struct ResidentProviderTaskService {
    owner: ResidentProviderTaskOwner,
}

impl ResidentProviderTaskService {
    fn completion_poll(join: ResidentProviderTaskJoin) -> ResidentProviderCompletionPoll {
        match join {
            ResidentProviderTaskJoin::Completed {
                continuation,
                key,
                request,
                output,
                cancellation,
                settlement,
            } => {
                ResidentProviderCompletionPoll::Completed(Box::new(ResidentCompletedProviderTask {
                    continuation,
                    key,
                    request: *request,
                    output,
                    cancellation,
                    settlement,
                }))
            }
            ResidentProviderTaskJoin::Lost {
                continuation,
                key,
                cancellation,
                settlement,
            } => ResidentProviderCompletionPoll::Lost(Box::new(ResidentLostProviderTask {
                continuation,
                key,
                cancellation,
                settlement,
            })),
            ResidentProviderTaskJoin::Stale(completion) => {
                ResidentProviderCompletionPoll::Stale(Box::new(completion))
            }
            ResidentProviderTaskJoin::ReservationRejected { completion, reason } => {
                ResidentProviderCompletionPoll::AggregateTransitionRejected {
                    completion: Box::new(completion),
                    reason,
                }
            }
        }
    }

    fn finish_recording<'operation>(
        result: Result<
            ProviderOperationInvokeResult,
            ProviderOperationRecordingFailure<'operation>,
        >,
        pending: ResidentProviderPendingSettlement,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentProviderTaskPoll<'operation> {
        match result {
            Ok(result) => {
                let cancellation = pending.cancellation;
                match pending.release(reservations) {
                    Ok(()) => ResidentProviderTaskPoll::Recorded {
                        result,
                        cancellation,
                    },
                    Err(failure) => ResidentProviderTaskPoll::AggregateReleaseRejected {
                        result: Box::new(result),
                        cancellation,
                        failure,
                    },
                }
            }
            Err(ProviderOperationRecordingFailure::Preparation {
                continuation,
                reason,
            }) => ResidentProviderTaskPoll::Preparation {
                continuation: Box::new(ResidentProviderRecordingContinuation {
                    continuation: *continuation,
                    pending,
                }),
                reason,
            },
            Err(ProviderOperationRecordingFailure::ConcurrentHead { conflict }) => {
                ResidentProviderTaskPoll::ConcurrentHead(Box::new(
                    ResidentProviderRecordingConflict {
                        conflict: *conflict,
                        pending,
                    },
                ))
            }
            Err(ProviderOperationRecordingFailure::Custody { custody }) => {
                ResidentProviderTaskPoll::Custody(Box::new(ResidentProviderRecordingCustody {
                    custody: *custody,
                    pending,
                }))
            }
            Err(ProviderOperationRecordingFailure::Finalization { failure }) => {
                let (finalization, reason) = failure.into_parts();
                ResidentProviderTaskPoll::Finalization {
                    finalization: Box::new(ResidentProviderRecordingFinalization {
                        finalization,
                        pending,
                    }),
                    reason,
                }
            }
        }
    }

    /// Reserve the complete process-local provider-work ceiling.
    pub(in crate::resident) fn new_in(
        owner_instance: ResidentOperationTaskOwnerInstance,
        limits: ResidentOperationTaskLimits,
    ) -> Result<Self, ResidentOperationTaskOwnerBuildError> {
        ResidentProviderTaskOwner::new(owner_instance, limits).map(|owner| Self { owner })
    }

    /// Return the number of calls accepted by the host executor.
    #[must_use]
    pub(in crate::resident) fn in_flight(&self) -> usize {
        self.owner.in_flight()
    }

    /// Return the exact process-local provider occupancy ceiling.
    #[must_use]
    pub(in crate::resident) const fn limits(&self) -> ResidentOperationTaskLimits {
        self.owner.limits()
    }

    /// Borrow the process owner shared by provider tasks and aggregate custody.
    #[must_use]
    pub(in crate::resident) const fn owner_instance(&self) -> &ResidentOperationTaskOwnerInstance {
        self.owner.owner_instance()
    }

    /// Select the exact accepted provider task without changing local state.
    #[must_use]
    pub(in crate::resident) fn select_dispatch_owner(
        &self,
        key: &ResidentOperationTaskKey,
    ) -> ResidentProviderDispatchOwnerSelection {
        if self.owner.owns(key) {
            ResidentProviderDispatchOwnerSelection::Owned(ResidentProviderDispatchOwner {
                key: key.clone(),
            })
        } else {
            ResidentProviderDispatchOwnerSelection::Missing
        }
    }

    /// Complete fresh release preflight and offer one retained call exactly once.
    ///
    /// Durable dispatch already completed before this method. A preflight or
    /// executor refusal returns the unopened application phase; acceptance
    /// transfers the only ordinary provider call to the host executor.
    pub(in crate::resident) fn try_accept<'operation, Authority, Executor>(
        &mut self,
        coordinator: &mut ResidentLocalOperationCoordinator<'_, Authority>,
        retained: RetainedProviderOperationDispatch<'operation>,
        input: &ProviderOperationInvokeInput<'_>,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentProviderTaskAcceptanceFailure<'operation>>
    where
        Authority: AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<OwnedProviderInvocationRequest, ProviderTaskOutput>,
    {
        if let Err(reason) = reservations.require_capacity() {
            return Err(match reason {
                ResidentTaskReservationOfferFailure::Saturated => {
                    ResidentProviderTaskAcceptanceFailure::AggregateSaturated(Box::new(retained))
                }
                ResidentTaskReservationOfferFailure::Duplicate => {
                    unreachable!("a capacity-only check cannot report a duplicate")
                }
                ResidentTaskReservationOfferFailure::OwnerMismatch
                | ResidentTaskReservationOfferFailure::StoreMismatch => {
                    unreachable!("a capacity-only check cannot report association failure")
                }
            });
        }
        let call = coordinator
            .prepare_retained_provider_call(retained, input)
            .map_err(ResidentProviderTaskAcceptanceFailure::Preflight)?;
        self.owner
            .try_accept(call, reservations, executor)
            .map_err(ResidentProviderTaskAcceptanceFailure::TaskOwner)
    }

    /// Offer one lineage-proved pairwise Delegate call exactly once.
    pub(in crate::resident) fn try_accept_pairwise_delegate<'operation, Authority, Executor>(
        &mut self,
        coordinator: &mut ResidentLocalOperationCoordinator<'_, Authority>,
        retained: RetainedProviderOperationDispatch<'operation>,
        input: &ProviderOperationInvokeInput<'_>,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentProviderTaskAcceptanceFailure<'operation>>
    where
        Authority: AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<OwnedProviderInvocationRequest, ProviderTaskOutput>,
    {
        if let Err(reason) = reservations.require_capacity() {
            return Err(match reason {
                ResidentTaskReservationOfferFailure::Saturated => {
                    ResidentProviderTaskAcceptanceFailure::AggregateSaturated(Box::new(retained))
                }
                ResidentTaskReservationOfferFailure::Duplicate => {
                    unreachable!("a capacity-only check cannot report a duplicate")
                }
                ResidentTaskReservationOfferFailure::OwnerMismatch
                | ResidentTaskReservationOfferFailure::StoreMismatch => {
                    unreachable!("a capacity-only check cannot report association failure")
                }
            });
        }
        let call = coordinator
            .prepare_pairwise_delegate_provider_call(retained, input)
            .map_err(ResidentProviderTaskAcceptanceFailure::Preflight)?;
        self.owner
            .try_accept(call, reservations, executor)
            .map_err(ResidentProviderTaskAcceptanceFailure::TaskOwner)
    }

    /// Complete recovered release preflight and offer the call exactly once.
    pub(in crate::resident) fn try_accept_recovered<Authority, Executor>(
        &mut self,
        coordinator: &mut ResidentLocalOperationCoordinator<'_, Authority>,
        release: RecoveredProviderDispatchRelease,
        input: &ProviderOperationInvokeInput<'_>,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentRecoveredProviderTaskAcceptanceFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<OwnedProviderInvocationRequest, ProviderTaskOutput>,
    {
        if let Err(reason) = reservations.require_capacity() {
            return Err(match reason {
                ResidentTaskReservationOfferFailure::Saturated => {
                    ResidentRecoveredProviderTaskAcceptanceFailure::AggregateSaturated(Box::new(
                        release,
                    ))
                }
                ResidentTaskReservationOfferFailure::Duplicate => {
                    unreachable!("a capacity-only check cannot report a duplicate")
                }
                ResidentTaskReservationOfferFailure::OwnerMismatch
                | ResidentTaskReservationOfferFailure::StoreMismatch => {
                    unreachable!("a capacity-only check cannot report association failure")
                }
            });
        }
        let call = coordinator
            .prepare_recovered_provider_call(release, input)
            .map_err(ResidentRecoveredProviderTaskAcceptanceFailure::Preflight)?;
        self.owner
            .try_accept_recovered(call, reservations, executor)
            .map_err(ResidentRecoveredProviderTaskAcceptanceFailure::TaskOwner)
    }

    /// Retry only recovered release preflight with the unchanged linear token.
    pub(in crate::resident) fn continue_recovered_preflight<Authority, Executor>(
        &mut self,
        coordinator: &mut ResidentLocalOperationCoordinator<'_, Authority>,
        failure: RecoveredProviderCallPreparationFailure,
        input: &ProviderOperationInvokeInput<'_>,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentRecoveredProviderTaskAcceptanceFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<OwnedProviderInvocationRequest, ProviderTaskOutput>,
    {
        let (release, _) = failure.into_parts();
        self.try_accept_recovered(coordinator, release, input, reservations, executor)
    }

    /// Retry only process-local acceptance of an already prepared unopened call.
    ///
    /// This does not repeat release preflight or durable dispatch retention.
    pub(in crate::resident) fn continue_refused_acceptance<'operation, Executor>(
        &mut self,
        refusal: ResidentProviderTaskRefusal<'operation>,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentProviderTaskRefusal<'operation>>
    where
        Executor: ResidentOperationTaskExecutor<OwnedProviderInvocationRequest, ProviderTaskOutput>,
    {
        if !refusal.retry_belongs_to(self.owner.owner_instance()) {
            return Err(refusal);
        }
        let call = match refusal {
            ResidentProviderTaskRefusal::Saturated { call, .. }
            | ResidentProviderTaskRefusal::Executor { call, .. } => *call,
            refusal @ (ResidentProviderTaskRefusal::Preparation { .. }
            | ResidentProviderTaskRefusal::Duplicate(_)
            | ResidentProviderTaskRefusal::OfferIdentityExhausted(_)
            | ResidentProviderTaskRefusal::ExecutorProtocolLoss { .. }
            | ResidentProviderTaskRefusal::AggregateTransition { .. }) => return Err(refusal),
        };
        self.owner.try_accept(call, reservations, executor)
    }

    /// Repair only aggregate occupancy after executor acceptance already won.
    ///
    /// No provider call, release preflight, durable retention, or signing work
    /// occurs on this path.
    pub(in crate::resident) fn continue_accepted_aggregate_transition<'operation>(
        &mut self,
        refusal: ResidentProviderTaskRefusal<'operation>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentProviderAcceptedAggregateTransition<'operation>,
        ResidentProviderTaskRefusal<'operation>,
    > {
        self.owner
            .continue_accepted_aggregate_transition(refusal, reservations)
    }

    /// Repair only aggregate occupancy after unopened work was refused.
    ///
    /// No provider call, release preflight, durable retention, or signing work
    /// occurs on this path.
    pub(in crate::resident) fn continue_refused_aggregate_transition<'operation>(
        &mut self,
        refusal: ResidentProviderTaskRefusal<'operation>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentProviderTaskRefusal<'operation>, ResidentProviderTaskRefusal<'operation>>
    {
        self.owner
            .continue_refused_aggregate_transition(refusal, reservations)
    }

    /// Record a process-local cancellation notification for accepted work.
    ///
    /// The caller must establish any durable cancellation head separately.
    /// This method never calls the executor and proves neither durable
    /// cancellation nor that the provider stopped.
    #[must_use]
    pub(in crate::resident) fn note_dispatch_cancellation_notification(
        &mut self,
        owner: ResidentProviderDispatchOwner,
    ) -> ResidentOperationTaskCancellationNotification {
        self.owner.note_cancellation_notification(&owner.key)
    }

    /// Poll once and join raw provider progress to its process-local owner.
    ///
    /// This method performs no durable custody or application recording. A
    /// completed or lost task carries the exact aggregate settlement token that
    /// the later recording owner must consume.
    pub(in crate::resident) fn poll_completion<Executor>(
        &mut self,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
    ) -> Option<ResidentProviderCompletionPoll>
    where
        Executor: ResidentOperationTaskExecutor<OwnedProviderInvocationRequest, ProviderTaskOutput>,
    {
        self.owner
            .try_completion(reservations, executor)
            .map(Self::completion_poll)
    }

    /// Retry only a rejected aggregate transition for returned provider work.
    ///
    /// The executor is not polled and the provider call is not repeated.
    pub(in crate::resident) fn retry_completion_transition(
        &mut self,
        completion: ResidentOperationTaskCompletion<
            OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentProviderCompletionPoll {
        Self::completion_poll(self.owner.retry_completion(completion, reservations))
    }

    /// Poll once and record a joined provider completion through the Invoke owner.
    ///
    /// The poll never waits. Stale completions carry no application basis and
    /// cannot advance an operation. A recording failure retains its existing
    /// continuation, competing-head conflict, or physical-custody owner.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn try_record_completion<
        'operation,
        Authority,
        Executor,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        coordinator: &mut ResidentLocalOperationCoordinator<'_, Authority>,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
        history: OperationAttemptHistoryInput<'operation>,
        recovered: &'operation RecoveredOperationObservation,
        admission: ProviderInvocationAdmissionContext<'operation>,
        input: &ProviderOperationInvokeInput<'_>,
        evidence: &dyn ProviderInvocationEvidenceAdmission,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Option<ResidentProviderTaskPoll<'static>>
    where
        Executor: ResidentOperationTaskExecutor<OwnedProviderInvocationRequest, ProviderTaskOutput>,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Reconciliation:
            FnMut(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
    {
        self.poll_completion(reservations, executor)
            .map(|completion| match completion {
                ResidentProviderCompletionPoll::Completed(completed) => {
                    Self::record_recovered_completion(
                        *completed,
                        coordinator,
                        reservations,
                        history,
                        recovered,
                        admission,
                        input,
                        evidence,
                        admit_reconciliation,
                        admit_rotation,
                        sign_observation,
                        sign_checkpoint,
                    )
                }
                ResidentProviderCompletionPoll::Lost(lost) => ResidentProviderTaskPoll::Lost(lost),
                ResidentProviderCompletionPoll::Stale(completion) => {
                    ResidentProviderTaskPoll::Stale(completion)
                }
                ResidentProviderCompletionPoll::AggregateTransitionRejected {
                    completion,
                    reason,
                } => ResidentProviderTaskPoll::AggregateTransitionRejected { completion, reason },
            })
    }

    /// Retry protected recovery for an accepted completion without another call.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn continue_recovered_completion<
        'operation,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        completed: ResidentCompletedProviderTask,
        coordinator: &mut ResidentLocalOperationCoordinator<'_, Authority>,
        reservations: &mut ResidentAggregateTaskReservations,
        history: OperationAttemptHistoryInput<'operation>,
        recovered: &'operation RecoveredOperationObservation,
        admission: ProviderInvocationAdmissionContext<'operation>,
        input: &ProviderOperationInvokeInput<'_>,
        evidence: &dyn ProviderInvocationEvidenceAdmission,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentProviderTaskPoll<'static>, ResidentProviderCompletionRecoveryRejection>
    where
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Reconciliation:
            FnMut(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
    {
        if let Err(reason) =
            reservations.require_provider_completion(&completed.settlement, &completed.key)
        {
            return Err(ResidentProviderCompletionRecoveryRejection {
                completed: Box::new(completed),
                reason,
            });
        }
        Ok(Self::record_recovered_completion(
            completed,
            coordinator,
            reservations,
            history,
            recovered,
            admission,
            input,
            evidence,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn record_recovered_completion<
        'operation,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        completed: ResidentCompletedProviderTask,
        coordinator: &mut ResidentLocalOperationCoordinator<'_, Authority>,
        reservations: &mut ResidentAggregateTaskReservations,
        history: OperationAttemptHistoryInput<'operation>,
        recovered: &'operation RecoveredOperationObservation,
        admission: ProviderInvocationAdmissionContext<'operation>,
        input: &ProviderOperationInvokeInput<'_>,
        evidence: &dyn ProviderInvocationEvidenceAdmission,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> ResidentProviderTaskPoll<'static>
    where
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Reconciliation:
            FnMut(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
    {
        match coordinator.prepare_recovered_provider_completion(
            &completed.continuation,
            &completed.request,
            history,
            recovered,
            admission,
            input.maximum_record_bytes,
        ) {
            Ok(recovered) => {
                let ResidentCompletedProviderTask {
                    continuation,
                    key,
                    request,
                    output,
                    cancellation,
                    settlement,
                } = completed;
                let result = coordinator.record_recovered_provider_completion(
                    continuation,
                    &request,
                    output,
                    recovered,
                    admission,
                    input,
                    evidence,
                    admit_reconciliation,
                    admit_rotation,
                    sign_observation,
                    sign_checkpoint,
                );
                Self::finish_recording(
                    result,
                    ResidentProviderPendingSettlement {
                        key,
                        cancellation,
                        settlement,
                        kind: ResidentProviderSettlementKind::Completion,
                    },
                    reservations,
                )
            }
            Err(reason) => ResidentProviderTaskPoll::RecoveryRejected {
                completed: Box::new(completed),
                reason,
            },
        }
    }

    /// Continue a consumed completion after a preparation-only recording failure.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn continue_recording<
        'operation,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        coordinator: &mut ResidentLocalOperationCoordinator<'_, Authority>,
        continuation: ResidentProviderRecordingContinuation<'operation>,
        reservations: &mut ResidentAggregateTaskReservations,
        input: &ProviderOperationInvokeInput<'_>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProviderTaskPoll<'operation>,
        ResidentProviderRecordingContinuationRejection<'operation>,
    >
    where
        Reconciliation:
            FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if let Err(reason) = continuation.pending.require(reservations) {
            return Err(ResidentProviderRecordingContinuationRejection {
                continuation: Box::new(continuation),
                reason,
            });
        }
        let ResidentProviderRecordingContinuation {
            continuation,
            pending,
        } = continuation;
        let result = coordinator.continue_recording(
            continuation,
            input,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        );
        Ok(Self::finish_recording(result, pending, reservations))
    }

    /// Rejoin a consumed completion to one protected cancellation-aware head.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn continue_after_concurrent_head<
        'operation,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        coordinator: &mut ResidentLocalOperationCoordinator<'_, Authority>,
        conflict: ResidentProviderRecordingConflict<'operation>,
        reservations: &mut ResidentAggregateTaskReservations,
        recovered: &RecoveredOperationObservation,
        input: &ProviderOperationInvokeInput<'_>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProviderTaskPoll<'operation>,
        ResidentProviderRecordingConflictRejection<'operation>,
    >
    where
        Reconciliation:
            FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if let Err(reason) = conflict.pending.require(reservations) {
            return Err(ResidentProviderRecordingConflictRejection {
                conflict: Box::new(conflict),
                reason,
            });
        }
        let ResidentProviderRecordingConflict { conflict, pending } = conflict;
        let result = coordinator.continue_after_concurrent_head(
            conflict,
            recovered,
            input,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        );
        let result = match result {
            Ok(result) => Ok(result),
            Err(ResidentProviderRecordingResumeFailure::Recording(failure)) => Err(failure),
            Err(ResidentProviderRecordingResumeFailure::Recovery(failure)) => {
                return Ok(ResidentProviderTaskPoll::ConcurrentHead(Box::new(
                    ResidentProviderRecordingConflict {
                        conflict: failure.into_conflict(),
                        pending,
                    },
                )));
            }
        };
        Ok(Self::finish_recording(result, pending, reservations))
    }

    /// Continue exact database/anchor custody after validating its aggregate owner.
    pub(in crate::resident) fn continue_custody<Authority>(
        &mut self,
        coordinator: &mut ResidentLocalOperationCoordinator<'_, Authority>,
        custody: ResidentProviderRecordingCustody,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentProviderTaskPoll<'static>, ResidentProviderRecordingCustodyRejection> {
        if let Err(reason) = custody.pending.require(reservations) {
            return Err(ResidentProviderRecordingCustodyRejection {
                custody: Box::new(custody),
                reason,
            });
        }
        let ResidentProviderRecordingCustody { custody, pending } = custody;
        let finalization = match coordinator.continue_provider_recording_custody(custody) {
            Ok(finalization) => finalization,
            Err(custody) => {
                return Ok(ResidentProviderTaskPoll::Custody(Box::new(
                    ResidentProviderRecordingCustody { custody, pending },
                )));
            }
        };
        Ok(Self::finish_finalization_inner(
            finalization,
            pending,
            reservations,
        ))
    }

    /// Retry only semantic admission of already-completed physical custody.
    pub(in crate::resident) fn finish_finalization(
        &mut self,
        finalization: ResidentProviderRecordingFinalization,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentProviderTaskPoll<'static>, ResidentProviderRecordingFinalizationRejection>
    {
        if let Err(reason) = finalization.pending.require(reservations) {
            return Err(ResidentProviderRecordingFinalizationRejection {
                finalization: Box::new(finalization),
                reason,
            });
        }
        let ResidentProviderRecordingFinalization {
            finalization,
            pending,
        } = finalization;
        Ok(Self::finish_finalization_inner(
            finalization,
            pending,
            reservations,
        ))
    }

    /// Retry only aggregate release after durable provider recording succeeded.
    pub(in crate::resident) fn continue_aggregate_release(
        &mut self,
        result: ProviderOperationInvokeResult,
        cancellation: ResidentOperationTaskCancellationState,
        failure: ResidentTaskReservationSettlementTransitionFailure,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentProviderTaskPoll<'static> {
        match reservations.retry_provider_release(failure) {
            Ok(()) => ResidentProviderTaskPoll::Recorded {
                result,
                cancellation,
            },
            Err(failure) => ResidentProviderTaskPoll::AggregateReleaseRejected {
                result: Box::new(result),
                cancellation,
                failure,
            },
        }
    }

    fn finish_finalization_inner(
        finalization: ProviderOperationRecordingFinalization,
        pending: ResidentProviderPendingSettlement,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentProviderTaskPoll<'static> {
        match finalization.finish() {
            Ok(result) => Self::finish_recording(Ok(result), pending, reservations),
            Err(failure) => {
                let (finalization, reason) = failure.into_parts();
                ResidentProviderTaskPoll::Finalization {
                    finalization: Box::new(ResidentProviderRecordingFinalization {
                        finalization,
                        pending,
                    }),
                    reason,
                }
            }
        }
    }

    /// Recover executor-lost work as outcome unknown without a provider port.
    ///
    /// The selected protected recovery member must be the exact durable task
    /// key returned as lost. This rejects cross-operation substitution before
    /// the existing historical and checkpoint admission runs.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn recover_lost<
        'operation,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        coordinator: &mut ResidentLocalOperationCoordinator<'_, Authority>,
        lost: ResidentLostProviderTask,
        reservations: &mut ResidentAggregateTaskReservations,
        history: OperationAttemptHistoryInput<'operation>,
        recovered: &'operation RecoveredOperationObservation,
        input: &ProviderOperationInvokeInput<'_>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentProviderTaskPoll<'static>, ResidentProviderTaskRecoveryFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        Reconciliation:
            FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if let Err(reason) = reservations.require_provider_loss(&lost.settlement, &lost.key) {
            return Err(ResidentProviderTaskRecoveryFailure::Settlement {
                lost: Box::new(lost),
                reason,
            });
        }
        if !lost.matches(recovered) {
            return Err(ResidentProviderTaskRecoveryFailure::Association(Box::new(
                lost,
            )));
        }
        let result = match coordinator.recover_dispatched(
            history,
            recovered,
            input,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        ) {
            Ok(result) => result,
            Err(RecoveredProviderUnknownRecordingFailure::Preparation(failure)) => {
                return Err(ResidentProviderTaskRecoveryFailure::Recording {
                    lost: Box::new(lost),
                    failure: Box::new(RecoveredProviderUnknownRecordingFailure::Preparation(
                        failure,
                    )),
                });
            }
            Err(
                failure @ (RecoveredProviderUnknownRecordingFailure::Prepared(_)
                | RecoveredProviderUnknownRecordingFailure::Issued(_)),
            ) => {
                return Err(ResidentProviderTaskRecoveryFailure::Recording {
                    lost: Box::new(lost),
                    failure: Box::new(failure),
                });
            }
            Err(RecoveredProviderUnknownRecordingFailure::Custody(custody)) => {
                return Ok(ResidentProviderTaskPoll::Custody(Box::new(
                    ResidentProviderRecordingCustody {
                        custody: *custody,
                        pending: ResidentProviderPendingSettlement {
                            key: lost.key,
                            cancellation: lost.cancellation,
                            settlement: lost.settlement,
                            kind: ResidentProviderSettlementKind::Loss,
                        },
                    },
                )));
            }
            Err(RecoveredProviderUnknownRecordingFailure::Finalization(failure)) => {
                let (finalization, reason) = failure.into_parts();
                return Ok(ResidentProviderTaskPoll::Finalization {
                    finalization: Box::new(ResidentProviderRecordingFinalization {
                        finalization,
                        pending: ResidentProviderPendingSettlement {
                            key: lost.key,
                            cancellation: lost.cancellation,
                            settlement: lost.settlement,
                            kind: ResidentProviderSettlementKind::Loss,
                        },
                    }),
                    reason,
                });
            }
        };
        Ok(Self::finish_recording(
            Ok(result),
            ResidentProviderPendingSettlement {
                key: lost.key,
                cancellation: lost.cancellation,
                settlement: lost.settlement,
                kind: ResidentProviderSettlementKind::Loss,
            },
            reservations,
        ))
    }
}

/// Process-local provider progress before any durable recording I/O.
#[must_use = "provider completion carries linear settlement custody"]
pub(in crate::resident) enum ResidentProviderCompletionPoll {
    /// Exact accepted output and its application basis.
    Completed(Box<ResidentCompletedProviderTask>),
    /// Exact accepted task whose executor lost its work.
    Lost(Box<ResidentLostProviderTask>),
    /// A completion that belongs to no accepted task in this process owner.
    Stale(Box<ResidentOperationTaskCompletion<OwnedProviderInvocationRequest, ProviderTaskOutput>>),
    /// Aggregate custody refused the join, so the accepted task remains active.
    AggregateTransitionRejected {
        completion: Box<
            ResidentOperationTaskCompletion<OwnedProviderInvocationRequest, ProviderTaskOutput>,
        >,
        reason: ResidentTaskReservationTransitionFailure,
    },
}

/// Why retained provider work did not transfer to the executor.
pub(in crate::resident) enum ResidentProviderTaskAcceptanceFailure<'operation> {
    /// The process-wide aggregate ceiling was full before release preflight.
    AggregateSaturated(Box<RetainedProviderOperationDispatch<'operation>>),
    /// Fresh authority or release preflight rejected the unopened dispatch.
    Preflight(ProviderOperationInvokeFailure<'operation>),
    /// The bounded task owner or host executor refused unopened work.
    TaskOwner(ResidentProviderTaskRefusal<'operation>),
}

/// Why a newly retained recovered dispatch did not transfer to the executor.
pub(in crate::resident) enum ResidentRecoveredProviderTaskAcceptanceFailure {
    /// Aggregate capacity was full before recovered release preflight.
    AggregateSaturated(Box<RecoveredProviderDispatchRelease>),
    /// Current authority, exact custody, or provider admission rejected release.
    Preflight(RecoveredProviderCallPreparationFailure),
    /// The task owner or host executor refused the unopened recovered call.
    TaskOwner(ResidentRecoveredProviderTaskRefusal),
}

/// One non-blocking provider-task poll.
#[must_use = "provider task progress can retain non-replayable evidence and capacity"]
pub(in crate::resident) enum ResidentProviderTaskPoll<'operation> {
    /// Provider evidence was passed to the application recording owner.
    Recorded {
        /// Exact completed provider recording.
        result: ProviderOperationInvokeResult,
        /// Whether the process-local owner received a cancellation notification.
        cancellation: ResidentOperationTaskCancellationState,
    },
    /// Recording stopped before a physical candidate existed.
    Preparation {
        continuation: Box<ResidentProviderRecordingContinuation<'operation>>,
        reason: Box<ProviderOperationRecordingPreparationError>,
    },
    /// A definite current operation head must be recovered first.
    ConcurrentHead(Box<ResidentProviderRecordingConflict<'operation>>),
    /// Exact physical candidate custody remains incomplete.
    Custody(Box<ResidentProviderRecordingCustody>),
    /// Completed physical custody awaits exact semantic admission.
    Finalization {
        finalization: Box<ResidentProviderRecordingFinalization>,
        reason: OperationAttemptError,
    },
    /// Accepted work disappeared and can only enter protected recovery.
    Lost(Box<ResidentLostProviderTask>),
    /// Protected durable history did not rejoin the accepted completion.
    RecoveryRejected {
        /// Exact accepted completion retained for another protected recovery.
        completed: Box<ResidentCompletedProviderTask>,
        /// Exact association, custody, or admission refusal.
        reason: OperationAttemptError,
    },
    /// A completion had no matching accepted application basis.
    Stale(Box<ResidentOperationTaskCompletion<OwnedProviderInvocationRequest, ProviderTaskOutput>>),
    /// The accepted task stayed active because aggregate custody rejected its join.
    AggregateTransitionRejected {
        completion: Box<
            ResidentOperationTaskCompletion<OwnedProviderInvocationRequest, ProviderTaskOutput>,
        >,
        reason: ResidentTaskReservationTransitionFailure,
    },
    /// Durable provider handling succeeded, but exact aggregate release failed.
    AggregateReleaseRejected {
        result: Box<ProviderOperationInvokeResult>,
        cancellation: ResidentOperationTaskCancellationState,
        failure: ResidentTaskReservationSettlementTransitionFailure,
    },
}

/// Owned accepted completion awaiting protected durable-history recovery.
pub(in crate::resident) struct ResidentCompletedProviderTask {
    continuation: ProviderOperationContinuationCoordinate,
    key: ResidentOperationTaskKey,
    request: OwnedProviderInvocationRequest,
    output: ProviderTaskOutput,
    cancellation: ResidentOperationTaskCancellationState,
    settlement: ResidentTaskReservationSettlement,
}

impl ResidentCompletedProviderTask {
    /// Borrow the exact store-qualified dispatch completed by the executor.
    #[must_use]
    pub(in crate::resident) const fn key(&self) -> &ResidentOperationTaskKey {
        &self.key
    }
}

/// Accepted provider work that disappeared without trustworthy output.
pub(in crate::resident) struct ResidentLostProviderTask {
    continuation: ProviderOperationContinuationCoordinate,
    key: ResidentOperationTaskKey,
    cancellation: ResidentOperationTaskCancellationState,
    settlement: ResidentTaskReservationSettlement,
}

impl ResidentLostProviderTask {
    /// Borrow the exact store-qualified dispatch lost by the executor.
    #[must_use]
    pub(in crate::resident) const fn key(&self) -> &ResidentOperationTaskKey {
        &self.key
    }

    /// Whether the process-local owner received a cancellation notification.
    ///
    /// This does not prove a durable cancellation head or stopped work.
    #[must_use]
    pub(in crate::resident) const fn cancellation_state(
        &self,
    ) -> ResidentOperationTaskCancellationState {
        self.cancellation
    }

    fn matches(&self, recovered: &RecoveredOperationObservation) -> bool {
        if self.key.store() != recovered.store() {
            return false;
        }
        let selected = recovered.selected();
        let Ok(digest) =
            ArtifactDigest::parse(selected.1.canonical_operation_observation_sha256().as_str())
        else {
            return false;
        };
        self.key.dispatch().record_ref == *selected.0
            && self.key.dispatch().record_sha256 == digest
            && self.continuation.store() == self.key.store()
            && self.continuation.dispatch() == self.key.dispatch()
    }
}

/// Why lost provider work did not reach protected uncertainty custody.
pub(in crate::resident) enum ResidentProviderTaskRecoveryFailure {
    /// The exact loss capability belongs to another owner, key, route, or phase.
    Settlement {
        lost: Box<ResidentLostProviderTask>,
        reason: ResidentTaskReservationTransitionFailure,
    },
    /// The supplied protected history did not select the exact lost dispatch.
    Association(Box<ResidentLostProviderTask>),
    /// Protected recovery stopped in the existing custody owner.
    Recording {
        /// Lost task basis retained for another protected read-back attempt.
        lost: Box<ResidentLostProviderTask>,
        /// Exact recording failure.
        failure: Box<RecoveredProviderUnknownRecordingFailure>,
    },
}

#[cfg(test)]
#[path = "../../tests/internal/resident/provider_task_service.rs"]
mod tests;
