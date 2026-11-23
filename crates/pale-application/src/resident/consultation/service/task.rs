//! Bounded process-local supervision for durable consultation batches.
//!
//! The consultation collection owns the finite roster. Each child operation
//! owns its durable dispatch and result history. This module only retains fresh
//! process-local work after the child owner proves that dispatch reached
//! database and independent-anchor custody. It adds no scheduler, retry,
//! completion, cancellation, or synthesis policy.

use std::num::NonZeroUsize;

use pale_core::records::{
    ArtifactDigest, ImmutableRecordReferenceV1, OperationActivePhaseV1, OperationObservationStateV1,
};
use pale_ports::{AuthorityStoreCoordinate, WallClockSourcePort};

use super::super::{
    ConsultationChildDispatchBatch, ConsultationChildDispatchCandidate, ConsultationChildJoinInput,
    ConsultationContributionSource, ConsultationSlotDisposition, ProtectedConsultationChildPhase,
    RecoveredConsultationCollection,
};
use super::{classify_protected_child, ConsultationChildDispatchReleaseFailure};
use crate::resident::{
    task_owner::{
        ResidentOperationTask, ResidentOperationTaskCancellationNotification,
        ResidentOperationTaskCancellationState, ResidentOperationTaskCompletion,
        ResidentOperationTaskExecutor, ResidentOperationTaskJoin, ResidentOperationTaskKey,
        ResidentOperationTaskLimits, ResidentOperationTaskOwner,
        ResidentOperationTaskOwnerBuildError, ResidentOperationTaskOwnerInstance,
        ResidentOperationTaskRefusal, ResidentReservedOperationTaskJoin,
    },
    task_reservation::{
        ResidentAggregateTaskReservations, ResidentTaskReservationOffer,
        ResidentTaskReservationOfferFailure, ResidentTaskReservationOfferTransitionFailure,
        ResidentTaskReservationRoute, ResidentTaskReservationSettlement,
        ResidentTaskReservationTransitionFailure,
    },
    IssuedLocalOperation, RecoveredOperationObservation,
};

/// Positive ceilings for one finite consultation batch in this resident lifetime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) struct ResidentConsultationTaskLimits {
    maximum_in_flight: NonZeroUsize,
    maximum_batch_members: NonZeroUsize,
}

impl ResidentConsultationTaskLimits {
    /// Bind both process-local ceilings explicitly.
    #[must_use]
    pub(in crate::resident) const fn new(
        maximum_in_flight: NonZeroUsize,
        maximum_batch_members: NonZeroUsize,
    ) -> Self {
        Self {
            maximum_in_flight,
            maximum_batch_members,
        }
    }

    /// Return the maximum mechanically accepted work at one time.
    #[must_use]
    pub(in crate::resident) const fn maximum_in_flight(self) -> NonZeroUsize {
        self.maximum_in_flight
    }

    /// Return the maximum work members in the complete admitted batch.
    #[must_use]
    pub(in crate::resident) const fn maximum_batch_members(self) -> NonZeroUsize {
        self.maximum_batch_members
    }
}

/// Result of the caller-owned durable child-dispatch transition.
///
/// `Retained` means the callback returned the exact protected child after its
/// dispatch completed both custody layers. `Unopened` permits another retention
/// attempt because no dispatch was consumed. `Uncertain` never permits replay;
/// protected child recovery must determine the next action.
#[must_use = "retain or recover this exact child-dispatch result"]
pub enum ConsultationChildRetentionOutcome<Work, Failure> {
    /// Durable dispatch is available with one unopened mechanical work value.
    Retained {
        /// Protected recovery whose current head must be the retained dispatch.
        child: Box<RecoveredOperationObservation>,
        /// Mechanical work that has not reached an executor.
        work: Work,
    },
    /// The child owner proved that dispatch remained unopened.
    Unopened(Failure),
    /// The child owner cannot prove whether dispatch crossed its boundary.
    Uncertain(Failure),
}

#[derive(Clone)]
struct ConsultationTaskBasis {
    candidate: ConsultationChildDispatchCandidate,
    dispatch: ResidentOperationTaskKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ConsultationTaskContextState {
    Unopened,
    DurableOnly,
    RetryPending(ResidentOperationTaskKey),
    Accepted(ResidentOperationTaskKey),
    CompletionPendingJoin(ResidentOperationTaskKey),
    LossPendingRecovery(ResidentOperationTaskKey),
}

pub(in crate::resident) struct RetainedConsultationTask<Work> {
    basis: ConsultationTaskBasis,
    work: Work,
}

/// Exact same-process Consult ownership proved before recovered provider I/O.
#[must_use = "accept this exact resumed child or retain its prepared ownership"]
pub(in crate::resident) struct PreparedResidentConsultationTaskResume {
    coordinate: ResidentConsultationBatchCoordinate,
    basis: ConsultationTaskBasis,
}

/// Exact historical Consult dispatch prepared only for durable loss handling.
#[must_use = "commit this exact recovered loss or retain its prepared ownership"]
pub(in crate::resident) struct PreparedResidentConsultationTaskLoss {
    coordinate: ResidentConsultationBatchCoordinate,
    basis: ConsultationTaskBasis,
}

/// Offered aggregate reservation whose only valid transition is recovered loss.
#[must_use = "retry the exact loss transition without beginning another offer"]
pub(in crate::resident) struct ResidentConsultationTaskLossOffer {
    prepared: PreparedResidentConsultationTaskLoss,
    offer: ResidentTaskReservationOffer,
}

impl ResidentConsultationTaskLossOffer {
    pub(in crate::resident) const fn coordinate(&self) -> &ResidentConsultationBatchCoordinate {
        &self.prepared.coordinate
    }

    pub(in crate::resident) const fn dispatch(&self) -> &ResidentOperationTaskKey {
        &self.prepared.basis.dispatch
    }
}

/// Process-local task state for one exact, finite consultation batch.
///
/// The service has no queue. At most one fresh retention callback can cross for
/// each roster child, and accepted work never exceeds `maximum_in_flight`.
/// Restart reconstructs meaning from the durable child and parent histories;
/// this process-local table never grants a replay.
pub(in crate::resident) struct ResidentConsultationTaskService {
    coordinate: ResidentConsultationBatchCoordinate,
    planned: ConsultationChildDispatchBatch,
    limits: ResidentConsultationTaskLimits,
    tasks: ResidentOperationTaskOwner<ConsultationTaskBasis>,
    context_state: Box<[ConsultationTaskContextState]>,
}

impl ResidentConsultationTaskService {
    /// Reserve the complete process-local bounds for one admitted batch.
    pub(in crate::resident) fn new_in(
        planned: ConsultationChildDispatchBatch,
        limits: ResidentConsultationTaskLimits,
        owner_instance: ResidentOperationTaskOwnerInstance,
    ) -> Result<Self, ResidentConsultationTaskBuildError> {
        if planned.candidates().len() > limits.maximum_batch_members().get() {
            return Err(ResidentConsultationTaskBuildError::BatchTooLarge);
        }
        for (index, candidate) in planned.candidates().iter().enumerate() {
            if planned.candidates()[..index]
                .iter()
                .any(|existing| existing.child() == candidate.child())
            {
                return Err(ResidentConsultationTaskBuildError::DuplicateChild);
            }
        }
        let coordinate =
            ResidentConsultationBatchCoordinate::from_batch(owner_instance.clone(), &planned);
        let tasks = ResidentOperationTaskOwner::new_in(
            owner_instance,
            ResidentOperationTaskLimits::new(limits.maximum_in_flight()),
        )
        .map_err(ResidentConsultationTaskBuildError::TaskOwner)?;
        let mut context_state = Vec::new();
        context_state
            .try_reserve_exact(planned.candidates().len())
            .map_err(|_| ResidentConsultationTaskBuildError::Allocation)?;
        context_state.extend(std::iter::repeat_n(
            ConsultationTaskContextState::Unopened,
            planned.candidates().len(),
        ));
        Ok(Self {
            coordinate,
            planned,
            limits,
            tasks,
            context_state: context_state.into_boxed_slice(),
        })
    }

    /// Return current process-local executor occupancy.
    #[must_use]
    pub(in crate::resident) fn in_flight(&self) -> usize {
        self.tasks.in_flight()
    }

    /// Borrow the sole process-owner-bound coordinate retained for this batch.
    #[must_use]
    pub(in crate::resident) const fn coordinate(&self) -> &ResidentConsultationBatchCoordinate {
        &self.coordinate
    }

    #[must_use]
    fn owns_dispatch(&self, dispatch: &ResidentOperationTaskKey) -> bool {
        self.tasks.owns(dispatch)
    }

    #[must_use]
    fn retirement_blocker(&self) -> Option<ResidentConsultationBatchRetentionReason> {
        self.context_state.iter().find_map(|state| match state {
            ConsultationTaskContextState::Unopened | ConsultationTaskContextState::DurableOnly => {
                None
            }
            ConsultationTaskContextState::RetryPending(_) => {
                Some(ResidentConsultationBatchRetentionReason::RetryPending)
            }
            ConsultationTaskContextState::Accepted(_) => {
                Some(ResidentConsultationBatchRetentionReason::Accepted)
            }
            ConsultationTaskContextState::CompletionPendingJoin(_) => {
                Some(ResidentConsultationBatchRetentionReason::CompletionPendingJoin)
            }
            ConsultationTaskContextState::LossPendingRecovery(_) => {
                Some(ResidentConsultationBatchRetentionReason::LossPendingRecovery)
            }
        })
    }

    #[must_use]
    fn candidate_state(
        &self,
        candidate: &ConsultationChildDispatchCandidate,
    ) -> Option<ResidentConsultationCandidateLocalState> {
        let member_index = self.member_index(candidate)?;
        Some(match &self.context_state[member_index] {
            ConsultationTaskContextState::Unopened => {
                ResidentConsultationCandidateLocalState::Unopened
            }
            ConsultationTaskContextState::DurableOnly => {
                ResidentConsultationCandidateLocalState::DurableOnly
            }
            ConsultationTaskContextState::RetryPending(dispatch) => {
                ResidentConsultationCandidateLocalState::RetryPending(dispatch.clone())
            }
            ConsultationTaskContextState::Accepted(dispatch) => {
                ResidentConsultationCandidateLocalState::Accepted(dispatch.clone())
            }
            ConsultationTaskContextState::CompletionPendingJoin(dispatch) => {
                ResidentConsultationCandidateLocalState::CompletionPendingJoin(dispatch.clone())
            }
            ConsultationTaskContextState::LossPendingRecovery(dispatch) => {
                ResidentConsultationCandidateLocalState::LossPendingRecovery(dispatch.clone())
            }
        })
    }

    fn resolve_continuation(
        &mut self,
        continuation: ResidentConsultationContinuationToken,
    ) -> ResidentConsultationLocalResolution {
        let expected = match continuation.kind {
            ResidentConsultationContinuationKind::Completed => {
                ResidentConsultationTaskContextKind::CompletionPendingJoin
            }
            ResidentConsultationContinuationKind::Lost => {
                ResidentConsultationTaskContextKind::LossPendingRecovery
            }
        };
        let Some(state) = self.context_state.iter_mut().find(|state| {
            matches!(
                state,
                ConsultationTaskContextState::CompletionPendingJoin(current)
                    | ConsultationTaskContextState::LossPendingRecovery(current)
                    if current == &continuation.dispatch
            )
        }) else {
            return ResidentConsultationLocalResolution::NotPending(continuation);
        };
        let actual = ResidentConsultationTaskContextKind::from_state(state);
        if actual != expected {
            return ResidentConsultationLocalResolution::KindMismatch {
                continuation,
                actual,
            };
        }
        *state = ConsultationTaskContextState::DurableOnly;
        ResidentConsultationLocalResolution::Resolved {
            kind: continuation.kind,
            settlement: continuation.settlement,
        }
    }

    fn resolve_continuation_reserved(
        &mut self,
        continuation: ResidentConsultationContinuationToken,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentConsultationContinuationResolution {
        let expected = match continuation.kind {
            ResidentConsultationContinuationKind::Completed => {
                ResidentConsultationTaskContextKind::CompletionPendingJoin
            }
            ResidentConsultationContinuationKind::Lost => {
                ResidentConsultationTaskContextKind::LossPendingRecovery
            }
        };
        let Some(member_index) = self.context_state.iter().position(|state| {
            matches!(
                state,
                ConsultationTaskContextState::CompletionPendingJoin(current)
                    | ConsultationTaskContextState::LossPendingRecovery(current)
                    if current == &continuation.dispatch
            )
        }) else {
            return ResidentConsultationContinuationResolution::NotPending(continuation);
        };
        let actual =
            ResidentConsultationTaskContextKind::from_state(&self.context_state[member_index]);
        if actual != expected {
            return ResidentConsultationContinuationResolution::KindMismatch {
                continuation,
                actual,
            };
        }

        let ResidentConsultationContinuationToken {
            coordinate,
            dispatch,
            kind,
            settlement,
        } = continuation;
        let Some(settlement) = settlement else {
            return ResidentConsultationContinuationResolution::AggregateReservationMissing(
                ResidentConsultationContinuationToken {
                    coordinate,
                    dispatch,
                    kind,
                    settlement: None,
                },
            );
        };
        let released = match kind {
            ResidentConsultationContinuationKind::Completed => {
                reservations.release_completion(settlement)
            }
            ResidentConsultationContinuationKind::Lost => reservations.release_loss(settlement),
        };
        if let Err(failure) = released {
            return ResidentConsultationContinuationResolution::AggregateReleaseRejected {
                continuation: ResidentConsultationContinuationToken {
                    coordinate,
                    dispatch,
                    kind,
                    settlement: Some(*failure.settlement),
                },
                reason: failure.reason,
            };
        }

        // The exact pending member was selected above while this service was
        // exclusively borrowed. No batch state can change between validation
        // and aggregate release.
        self.context_state[member_index] = ConsultationTaskContextState::DurableOnly;
        ResidentConsultationContinuationResolution::Resolved
    }

    /// Recheck, durably retain, and non-blockingly offer one child exactly once.
    ///
    /// The callback owns the child-specific dispatch transaction. It must return
    /// `Retained` only after both custody layers are complete, `Unopened` only
    /// when another attempt is safe, and `Uncertain` otherwise. The executor is
    /// never called until the returned protected child proves the exact
    /// `Dispatched` head. A restart cannot call this method for an already
    /// released child because its preflight phase is no longer eligible.
    #[allow(clippy::too_many_arguments)]
    fn retain_and_accept_reserved<
        Work,
        Output,
        LookupError,
        RetentionFailure,
        Lookup,
        RecoverChild,
        Retain,
        Executor,
    >(
        &mut self,
        candidate: &ConsultationChildDispatchCandidate,
        reservations: &mut ResidentAggregateTaskReservations,
        clock: &dyn WallClockSourcePort,
        lookup: Lookup,
        recover_child: RecoverChild,
        retain: Retain,
        executor: &mut Executor,
    ) -> Result<
        ResidentOperationTaskKey,
        Box<ResidentConsultationTaskAcceptanceFailure<Work, LookupError, RetentionFailure>>,
    >
    where
        Lookup: FnOnce() -> Result<RecoveredConsultationCollection, LookupError>,
        RecoverChild: FnOnce(
            &ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, LookupError>,
        Retain: FnOnce(
            &ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        if reservations.require_capacity().is_err() {
            return Err(Box::new(
                ResidentConsultationTaskAcceptanceFailure::AggregateSaturated(candidate.clone()),
            ));
        }
        let Some((member_index, planned_candidate)) = self
            .planned
            .candidates()
            .iter()
            .enumerate()
            .find(|(_, planned)| *planned == candidate)
        else {
            return Err(Box::new(
                ResidentConsultationTaskAcceptanceFailure::NotPlanned(candidate.clone()),
            ));
        };
        let candidate = planned_candidate.clone();
        if !matches!(
            self.context_state[member_index],
            ConsultationTaskContextState::Unopened
        ) {
            return Err(Box::new(
                ResidentConsultationTaskAcceptanceFailure::AlreadyReleased(candidate),
            ));
        }
        if self.tasks.in_flight() == self.limits.maximum_in_flight().get() {
            return Err(Box::new(
                ResidentConsultationTaskAcceptanceFailure::Saturated(candidate),
            ));
        }

        let current = lookup().map_err(|reason| {
            Box::new(ResidentConsultationTaskAcceptanceFailure::Release(
                ConsultationChildDispatchReleaseFailure::ParentLookup(reason),
            ))
        })?;
        let observed_at = clock.observe_unix_time().map_err(|reason| {
            Box::new(ResidentConsultationTaskAcceptanceFailure::Release(
                ConsultationChildDispatchReleaseFailure::Clock(reason),
            ))
        })?;
        self.retain_and_accept_recovered_reserved(
            &candidate,
            reservations,
            &current,
            observed_at,
            recover_child,
            retain,
            executor,
        )
    }

    /// Retain one candidate from an already recovered exact collection head.
    ///
    /// A bounded supervision turn uses this route so collection recovery and
    /// time observation each occur once. The child and dispatch owners still
    /// perform their ordinary exact revalidation before executor acceptance.
    #[allow(clippy::too_many_arguments)]
    fn retain_and_accept_recovered_reserved<
        Work,
        Output,
        LookupError,
        RetentionFailure,
        RecoverChild,
        Retain,
        Executor,
    >(
        &mut self,
        candidate: &ConsultationChildDispatchCandidate,
        reservations: &mut ResidentAggregateTaskReservations,
        current: &RecoveredConsultationCollection,
        observed_at: pale_core::records::UnixTimeV1,
        recover_child: RecoverChild,
        retain: Retain,
        executor: &mut Executor,
    ) -> Result<
        ResidentOperationTaskKey,
        Box<ResidentConsultationTaskAcceptanceFailure<Work, LookupError, RetentionFailure>>,
    >
    where
        RecoverChild: FnOnce(
            &ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, LookupError>,
        Retain: FnOnce(
            &ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        if reservations.require_capacity().is_err() {
            return Err(Box::new(
                ResidentConsultationTaskAcceptanceFailure::AggregateSaturated(candidate.clone()),
            ));
        }
        let Some((member_index, planned_candidate)) = self
            .planned
            .candidates()
            .iter()
            .enumerate()
            .find(|(_, planned)| *planned == candidate)
        else {
            return Err(Box::new(
                ResidentConsultationTaskAcceptanceFailure::NotPlanned(candidate.clone()),
            ));
        };
        let candidate = planned_candidate.clone();
        if !matches!(
            self.context_state[member_index],
            ConsultationTaskContextState::Unopened
        ) {
            return Err(Box::new(
                ResidentConsultationTaskAcceptanceFailure::AlreadyReleased(candidate),
            ));
        }
        if self.tasks.in_flight() == self.limits.maximum_in_flight().get() {
            return Err(Box::new(
                ResidentConsultationTaskAcceptanceFailure::Saturated(candidate),
            ));
        }
        let current_candidate =
            super::revalidate_planned_child(current, &self.planned, &candidate, observed_at)
                .map_err(|reason| {
                    Box::new(ResidentConsultationTaskAcceptanceFailure::Release(
                        ConsultationChildDispatchReleaseFailure::Revalidation(reason),
                    ))
                })?;
        let child = recover_child(&current_candidate).map_err(|reason| {
            Box::new(ResidentConsultationTaskAcceptanceFailure::Release(
                ConsultationChildDispatchReleaseFailure::ChildLookup(reason),
            ))
        })?;
        match classify_protected_child(
            current,
            &self.planned,
            &current_candidate,
            observed_at,
            &child,
        )
        .map_err(|reason| {
            Box::new(ResidentConsultationTaskAcceptanceFailure::Release(
                ConsultationChildDispatchReleaseFailure::Revalidation(reason),
            ))
        })? {
            ProtectedConsultationChildPhase::EligibleForFirstDispatch => {}
            ProtectedConsultationChildPhase::ResumeOnly => {
                return Err(Box::new(
                    ResidentConsultationTaskAcceptanceFailure::Release(
                        ConsultationChildDispatchReleaseFailure::ResumeOnly,
                    ),
                ));
            }
            ProtectedConsultationChildPhase::TerminalJoin => {
                return Err(Box::new(
                    ResidentConsultationTaskAcceptanceFailure::Release(
                        ConsultationChildDispatchReleaseFailure::TerminalJoin,
                    ),
                ));
            }
        }

        let retained = match retain(&current_candidate, child) {
            ConsultationChildRetentionOutcome::Retained { child, work } => {
                self.mark_release_crossed(member_index);
                let phase = match classify_protected_child(
                    current,
                    &self.planned,
                    &current_candidate,
                    observed_at,
                    &child,
                ) {
                    Ok(phase) => phase,
                    Err(reason) => {
                        return Err(Box::new(
                            ResidentConsultationTaskAcceptanceFailure::RetainedRevalidation {
                                candidate: current_candidate.clone(),
                                work,
                                reason,
                            },
                        ));
                    }
                };
                let (dispatch_ref, dispatch) = child.selected_head();
                if phase != ProtectedConsultationChildPhase::ResumeOnly
                    || !matches!(
                        dispatch.state(),
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::Dispatched { .. }
                        }
                    )
                {
                    return Err(Box::new(
                        ResidentConsultationTaskAcceptanceFailure::RetainedPhase {
                            candidate: current_candidate,
                            work,
                            phase,
                        },
                    ));
                }
                let record_sha256 = match ArtifactDigest::parse(
                    dispatch.canonical_operation_observation_sha256().as_str(),
                ) {
                    Ok(digest) => digest,
                    Err(_) => {
                        return Err(Box::new(
                            ResidentConsultationTaskAcceptanceFailure::DispatchIdentity {
                                candidate: current_candidate.clone(),
                                work,
                            },
                        ));
                    }
                };
                RetainedConsultationTask {
                    basis: ConsultationTaskBasis {
                        candidate: current_candidate,
                        dispatch: ResidentOperationTaskKey::new(
                            self.planned.store().clone(),
                            ImmutableRecordReferenceV1 {
                                record_ref: dispatch_ref.clone(),
                                record_sha256,
                            },
                        ),
                    },
                    work,
                }
            }
            ConsultationChildRetentionOutcome::Unopened(reason) => {
                return Err(Box::new(
                    ResidentConsultationTaskAcceptanceFailure::RetentionUnopened {
                        candidate: current_candidate,
                        reason,
                    },
                ));
            }
            ConsultationChildRetentionOutcome::Uncertain(reason) => {
                self.mark_release_crossed(member_index);
                return Err(Box::new(
                    ResidentConsultationTaskAcceptanceFailure::RetentionUncertain {
                        candidate: current_candidate,
                        reason,
                    },
                ));
            }
        };
        self.try_accept_retained_reserved(retained, reservations, executor)
            .map_err(|failure| {
                Box::new(ResidentConsultationTaskAcceptanceFailure::TaskOwner(
                    failure,
                ))
            })
    }

    /// Prove exact resumed ownership before a recovered provider call is prepared.
    #[allow(clippy::too_many_arguments)]
    fn prepare_resumed_reserved(
        &self,
        candidate: &ConsultationChildDispatchCandidate,
        reservations: &ResidentAggregateTaskReservations,
        current: &RecoveredConsultationCollection,
        observed_at: pale_core::records::UnixTimeV1,
        child_store: &AuthorityStoreCoordinate,
        child_operation: &IssuedLocalOperation,
        dispatch_ref: &pale_core::records::ResidentRecordRefV1,
        dispatch: &pale_core::records::OperationObservationV1,
    ) -> Result<ConsultationTaskBasis, ResidentConsultationTaskResumePreparationFailure> {
        if reservations.require_capacity().is_err() {
            return Err(ResidentConsultationTaskResumePreparationFailure::AggregateSaturated);
        }
        let Some((member_index, planned_candidate)) = self
            .planned
            .candidates()
            .iter()
            .enumerate()
            .find(|(_, planned)| *planned == candidate)
        else {
            return Err(ResidentConsultationTaskResumePreparationFailure::NotPlanned);
        };
        if !matches!(
            self.context_state[member_index],
            ConsultationTaskContextState::Unopened
        ) {
            return Err(ResidentConsultationTaskResumePreparationFailure::AlreadyReleased);
        }
        if self.tasks.in_flight() == self.limits.maximum_in_flight().get() {
            return Err(ResidentConsultationTaskResumePreparationFailure::Saturated);
        }
        let current_candidate =
            super::revalidate_planned_child(current, &self.planned, planned_candidate, observed_at)
                .map_err(ResidentConsultationTaskResumePreparationFailure::Revalidation)?;
        let (child_ref, child_handle) = child_operation.handle();
        if child_store != self.planned.store()
            || child_ref != &current_candidate.child.record_ref
            || child_handle.canonical_operation_handle_sha256().as_str()
                != current_candidate.child.record_sha256.as_str()
            || dispatch.operation_handle_ref() != child_ref
            || dispatch.operation_handle_sha256()
                != child_handle.canonical_operation_handle_sha256()
        {
            return Err(ResidentConsultationTaskResumePreparationFailure::ChildAssociation);
        }
        if !matches!(
            dispatch.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched { .. }
            }
        ) {
            return Err(ResidentConsultationTaskResumePreparationFailure::DispatchPhase);
        }
        let record_sha256 =
            match ArtifactDigest::parse(dispatch.canonical_operation_observation_sha256().as_str())
            {
                Ok(record_sha256) => record_sha256,
                Err(_) => {
                    return Err(ResidentConsultationTaskResumePreparationFailure::DispatchIdentity);
                }
            };
        let basis = ConsultationTaskBasis {
            candidate: current_candidate,
            dispatch: ResidentOperationTaskKey::new(
                self.planned.store().clone(),
                ImmutableRecordReferenceV1 {
                    record_ref: dispatch_ref.clone(),
                    record_sha256,
                },
            ),
        };
        if self.tasks.owns(&basis.dispatch)
            || reservations
                .can_begin_offer(
                    &basis.dispatch,
                    &ResidentTaskReservationRoute::Consultation(self.coordinate.clone()),
                )
                .is_err()
        {
            return Err(ResidentConsultationTaskResumePreparationFailure::Duplicate);
        }
        Ok(basis)
    }

    /// Prove exact historical loss ownership without consulting executor capacity.
    #[allow(clippy::too_many_arguments)]
    fn prepare_recovered_loss_reserved(
        &self,
        candidate: &ConsultationChildDispatchCandidate,
        reservations: &ResidentAggregateTaskReservations,
        current: &RecoveredConsultationCollection,
        observed_at: pale_core::records::UnixTimeV1,
        child_store: &AuthorityStoreCoordinate,
        child_operation: &IssuedLocalOperation,
        dispatch_ref: &pale_core::records::ResidentRecordRefV1,
        dispatch: &pale_core::records::OperationObservationV1,
    ) -> Result<ConsultationTaskBasis, ResidentConsultationTaskLossPreparationFailure> {
        let Some((member_index, planned_candidate)) = self
            .planned
            .candidates()
            .iter()
            .enumerate()
            .find(|(_, planned)| *planned == candidate)
        else {
            return Err(ResidentConsultationTaskLossPreparationFailure::NotPlanned);
        };
        if !matches!(
            self.context_state[member_index],
            ConsultationTaskContextState::Unopened
        ) {
            return Err(ResidentConsultationTaskLossPreparationFailure::AlreadyReleased);
        }
        let current_candidate =
            super::revalidate_planned_child(current, &self.planned, planned_candidate, observed_at)
                .map_err(ResidentConsultationTaskLossPreparationFailure::Revalidation)?;
        let (child_ref, child_handle) = child_operation.handle();
        if child_store != self.planned.store()
            || child_ref != &current_candidate.child.record_ref
            || child_handle.canonical_operation_handle_sha256().as_str()
                != current_candidate.child.record_sha256.as_str()
            || dispatch.operation_handle_ref() != child_ref
            || dispatch.operation_handle_sha256()
                != child_handle.canonical_operation_handle_sha256()
        {
            return Err(ResidentConsultationTaskLossPreparationFailure::ChildAssociation);
        }
        if !matches!(
            dispatch.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched { .. }
            }
        ) {
            return Err(ResidentConsultationTaskLossPreparationFailure::DispatchPhase);
        }
        let record_sha256 =
            ArtifactDigest::parse(dispatch.canonical_operation_observation_sha256().as_str())
                .map_err(|_| ResidentConsultationTaskLossPreparationFailure::DispatchIdentity)?;
        let basis = ConsultationTaskBasis {
            candidate: current_candidate,
            dispatch: ResidentOperationTaskKey::new(
                self.planned.store().clone(),
                ImmutableRecordReferenceV1 {
                    record_ref: dispatch_ref.clone(),
                    record_sha256,
                },
            ),
        };
        if self.tasks.owns(&basis.dispatch) {
            return Err(ResidentConsultationTaskLossPreparationFailure::Duplicate);
        }
        reservations
            .can_begin_offer(
                &basis.dispatch,
                &ResidentTaskReservationRoute::Consultation(self.coordinate.clone()),
            )
            .map_err(ResidentConsultationTaskLossPreparationFailure::Reservation)?;
        Ok(basis)
    }

    /// Offer work only after resumed ownership was prepared without provider I/O.
    fn accept_prepared_resume_reserved<Work, Output, Executor>(
        &mut self,
        prepared: PreparedResidentConsultationTaskResume,
        reservations: &mut ResidentAggregateTaskReservations,
        work: Work,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentConsultationPreparedResumeFailure<Work>>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        let member_index = self.member_index(&prepared.basis.candidate);
        if prepared.coordinate != self.coordinate
            || member_index.is_none()
            || !matches!(
                self.context_state[member_index.unwrap_or_default()],
                ConsultationTaskContextState::Unopened
            )
        {
            return Err(ResidentConsultationPreparedResumeFailure::Unavailable {
                prepared: Box::new(prepared),
                work,
            });
        }
        let member_index = member_index.expect("checked exact resumed member");
        let retained = RetainedConsultationTask {
            basis: prepared.basis,
            work,
        };
        self.mark_release_crossed(member_index);
        self.try_accept_retained_reserved(retained, reservations, executor)
            .map_err(|failure| {
                ResidentConsultationPreparedResumeFailure::Refused(Box::new(failure))
            })
    }

    /// Retry executor acceptance of one already retained, still unopened task.
    ///
    /// This method does not repeat collection lookup or child dispatch. The only
    /// accepted value is the linear token returned after an executor refusal.
    fn retry_accept_reserved<Work, Output, Executor>(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
        retry: ResidentConsultationTaskRetry<Work>,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentConsultationTaskRetryFailure<Work>>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        if reservations.require_capacity().is_err() {
            return Err(ResidentConsultationTaskRetryFailure::Refused(Box::new(
                ResidentConsultationTaskRefusal::AggregateSaturated(Box::new(retry)),
            )));
        }
        if retry.coordinate != *coordinate || retry.coordinate != self.coordinate {
            return Err(ResidentConsultationTaskRetryFailure::BatchMismatch(
                Box::new(retry),
            ));
        }
        let dispatch = retry.retained.basis.dispatch.clone();
        let Some(member_index) = self.member_index(&retry.retained.basis.candidate) else {
            return Err(
                ResidentConsultationTaskRetryFailure::ContinuationUnavailable(Box::new(retry)),
            );
        };
        if !matches!(
            &self.context_state[member_index],
            ConsultationTaskContextState::RetryPending(expected) if expected == &dispatch
        ) {
            return Err(
                ResidentConsultationTaskRetryFailure::ContinuationUnavailable(Box::new(retry)),
            );
        }
        self.try_accept_retained_reserved(*retry.retained, reservations, executor)
            .map_err(|refusal| ResidentConsultationTaskRetryFailure::Refused(Box::new(refusal)))
    }

    /// Record a process-local cancellation notification for accepted child work.
    ///
    /// The notification does not call the executor and proves neither a durable
    /// cancellation request nor terminal child state.
    #[must_use]
    pub(in crate::resident) fn note_cancellation_notification(
        &mut self,
        dispatch: &ResidentOperationTaskKey,
    ) -> ResidentOperationTaskCancellationNotification {
        self.tasks.note_cancellation_notification(dispatch)
    }

    fn join_completion<Work, Output>(
        &mut self,
        completion: ResidentOperationTaskCompletion<Work, Output>,
    ) -> ResidentConsultationTaskPoll<Work, Output> {
        match self.tasks.join_completion(completion) {
            ResidentOperationTaskJoin::Completed {
                basis,
                task,
                output,
                cancellation,
            } => {
                self.mark_completion_pending(&basis);
                ResidentConsultationTaskPoll::Completed {
                    candidate: basis.candidate,
                    dispatch: basis.dispatch,
                    work: task.into_parts().1,
                    output,
                    cancellation,
                }
            }
            ResidentOperationTaskJoin::Lost {
                basis,
                key,
                cancellation,
            } => {
                self.mark_loss_pending(&basis);
                ResidentConsultationTaskPoll::Lost(ResidentConsultationLostTask {
                    basis,
                    key,
                    cancellation,
                })
            }
            ResidentOperationTaskJoin::Stale(completion) => {
                ResidentConsultationTaskPoll::Stale(completion)
            }
        }
    }

    fn join_reserved_completion<Work, Output>(
        &mut self,
        completion: ResidentOperationTaskCompletion<Work, Output>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentConsultationReservedTaskPoll<Work, Output> {
        match self.tasks.join_reserved_completion(
            completion,
            reservations,
            ResidentTaskReservationRoute::Consultation(self.coordinate.clone()),
        ) {
            ResidentReservedOperationTaskJoin::Completed {
                basis,
                key,
                task,
                output,
                cancellation,
                settlement,
            } => {
                self.mark_completion_pending(&basis);
                ResidentConsultationReservedTaskPoll::Handled {
                    outcome: Box::new(ResidentConsultationTaskPoll::Completed {
                        candidate: basis.candidate,
                        dispatch: key,
                        work: task.into_parts().1,
                        output,
                        cancellation,
                    }),
                    settlement,
                }
            }
            ResidentReservedOperationTaskJoin::Lost {
                basis,
                key,
                cancellation,
                settlement,
            } => {
                self.mark_loss_pending(&basis);
                ResidentConsultationReservedTaskPoll::Handled {
                    outcome: Box::new(ResidentConsultationTaskPoll::Lost(
                        ResidentConsultationLostTask {
                            basis,
                            key,
                            cancellation,
                        },
                    )),
                    settlement,
                }
            }
            ResidentReservedOperationTaskJoin::Stale(completion) => {
                ResidentConsultationReservedTaskPoll::Stale(completion)
            }
            ResidentReservedOperationTaskJoin::ReservationRejected { completion, reason } => {
                ResidentConsultationReservedTaskPoll::ReservationRejected { completion, reason }
            }
        }
    }

    /// Classify one executor-lost task through protected custody without replay.
    ///
    /// Recovery must select the exact durable dispatch held by the lost task.
    /// An eligible pre-dispatch head is rollback or substitution, not permission
    /// to call the executor again.
    pub(in crate::resident) fn classify_lost<LookupError, Lookup, RecoverChild>(
        &self,
        lost: &ResidentConsultationLostTask,
        clock: &dyn WallClockSourcePort,
        lookup: Lookup,
        recover_child: RecoverChild,
    ) -> Result<ProtectedConsultationChildPhase, ResidentConsultationTaskRecoveryFailure<LookupError>>
    where
        Lookup: FnOnce() -> Result<RecoveredConsultationCollection, LookupError>,
        RecoverChild: FnOnce(
            &ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, LookupError>,
    {
        let phase = self.classify_recovery(
            &lost.basis.candidate,
            Some(&lost.basis.dispatch),
            clock,
            lookup,
            recover_child,
        )?;
        if phase == ProtectedConsultationChildPhase::EligibleForFirstDispatch {
            Err(ResidentConsultationTaskRecoveryFailure::DispatchRolledBack)
        } else {
            Ok(phase)
        }
    }

    /// Classify a planned child after restart without creating executor work.
    ///
    /// A previously released child must recover as `ResumeOnly` or
    /// `TerminalJoin`. Only a child that never crossed dispatch remains eligible.
    pub(in crate::resident) fn classify_after_restart<LookupError, Lookup, RecoverChild>(
        &self,
        candidate: &ConsultationChildDispatchCandidate,
        clock: &dyn WallClockSourcePort,
        lookup: Lookup,
        recover_child: RecoverChild,
    ) -> Result<ProtectedConsultationChildPhase, ResidentConsultationTaskRecoveryFailure<LookupError>>
    where
        Lookup: FnOnce() -> Result<RecoveredConsultationCollection, LookupError>,
        RecoverChild: FnOnce(
            &ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, LookupError>,
    {
        self.classify_recovery(candidate, None, clock, lookup, recover_child)
    }

    fn classify_recovery<LookupError, Lookup, RecoverChild>(
        &self,
        candidate: &ConsultationChildDispatchCandidate,
        exact_dispatch: Option<&ResidentOperationTaskKey>,
        clock: &dyn WallClockSourcePort,
        lookup: Lookup,
        recover_child: RecoverChild,
    ) -> Result<ProtectedConsultationChildPhase, ResidentConsultationTaskRecoveryFailure<LookupError>>
    where
        Lookup: FnOnce() -> Result<RecoveredConsultationCollection, LookupError>,
        RecoverChild: FnOnce(
            &ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, LookupError>,
    {
        let current = lookup().map_err(ResidentConsultationTaskRecoveryFailure::ParentLookup)?;
        let observed_at = clock
            .observe_unix_time()
            .map_err(ResidentConsultationTaskRecoveryFailure::Clock)?;
        let current_candidate =
            super::revalidate_released_child(&current, &self.planned, candidate, observed_at)
                .map_err(ResidentConsultationTaskRecoveryFailure::Revalidation)?;
        let child = recover_child(&current_candidate)
            .map_err(ResidentConsultationTaskRecoveryFailure::ChildLookup)?;
        if let Some(exact_dispatch) = exact_dispatch {
            let (selected_ref, selected) = child.selected();
            if child.store() != exact_dispatch.store()
                || selected_ref != &exact_dispatch.dispatch().record_ref
                || selected.canonical_operation_observation_sha256().as_str()
                    != exact_dispatch.dispatch().record_sha256.as_str()
            {
                return Err(ResidentConsultationTaskRecoveryFailure::DispatchAssociation);
            }
        }
        super::classify_released_child(
            &current,
            &self.planned,
            &current_candidate,
            observed_at,
            &child,
        )
        .map_err(ResidentConsultationTaskRecoveryFailure::Revalidation)
    }

    fn member_index(&self, candidate: &ConsultationChildDispatchCandidate) -> Option<usize> {
        self.planned
            .candidates()
            .iter()
            .position(|planned| planned == candidate)
    }

    fn mark_release_crossed(&mut self, member_index: usize) {
        self.context_state[member_index] = ConsultationTaskContextState::DurableOnly;
    }

    fn mark_completion_pending(&mut self, basis: &ConsultationTaskBasis) {
        if let Some(member_index) = self.member_index(&basis.candidate) {
            self.context_state[member_index] =
                ConsultationTaskContextState::CompletionPendingJoin(basis.dispatch.clone());
        }
    }

    fn mark_loss_pending(&mut self, basis: &ConsultationTaskBasis) {
        if let Some(member_index) = self.member_index(&basis.candidate) {
            self.context_state[member_index] =
                ConsultationTaskContextState::LossPendingRecovery(basis.dispatch.clone());
        }
    }

    fn try_accept_retained_reserved<Work, Output, Executor>(
        &mut self,
        retained: RetainedConsultationTask<Work>,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentConsultationTaskRefusal<Work>>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        let coordinate = self.coordinate.clone();
        let member_index = self.member_index(&retained.basis.candidate);
        let key = retained.basis.dispatch.clone();
        let offer = match reservations.begin_offer(
            key.clone(),
            ResidentTaskReservationRoute::Consultation(coordinate.clone()),
        ) {
            Ok(offer) => offer,
            Err(ResidentTaskReservationOfferFailure::Saturated) => {
                return Err(ResidentConsultationTaskRefusal::AggregateSaturated(
                    Box::new(ResidentConsultationTaskRetry {
                        coordinate,
                        retained: Box::new(retained),
                    }),
                ));
            }
            Err(ResidentTaskReservationOfferFailure::Duplicate) => {
                return Err(ResidentConsultationTaskRefusal::Duplicate(Box::new(
                    retained,
                )));
            }
            Err(
                reason @ (ResidentTaskReservationOfferFailure::OwnerMismatch
                | ResidentTaskReservationOfferFailure::StoreMismatch),
            ) => {
                return Err(ResidentConsultationTaskRefusal::AggregateAssociation {
                    retained: Box::new(retained),
                    reason,
                });
            }
        };
        let mechanical = self
            .tasks
            .try_accept(key, retained.basis, retained.work, executor);
        let protocol_loss = matches!(
            mechanical,
            Err(ResidentOperationTaskRefusal::ExecutorProtocolLoss { .. })
        );
        let transition = if mechanical.is_ok() || protocol_loss {
            reservations.accept_offer(offer)
        } else {
            reservations.refuse_offer(offer)
        };
        let outcome = mechanical.map_err(|refusal| match refusal {
            ResidentOperationTaskRefusal::Duplicate { basis, work } => {
                ResidentConsultationTaskRefusal::Duplicate(Box::new(RetainedConsultationTask {
                    basis,
                    work,
                }))
            }
            ResidentOperationTaskRefusal::Saturated { basis, work } => {
                ResidentConsultationTaskRefusal::Saturated(Box::new(
                    ResidentConsultationTaskRetry {
                        coordinate: coordinate.clone(),
                        retained: Box::new(RetainedConsultationTask { basis, work }),
                    },
                ))
            }
            ResidentOperationTaskRefusal::OfferIdentityExhausted { basis, work } => {
                ResidentConsultationTaskRefusal::OfferIdentityExhausted(Box::new(
                    RetainedConsultationTask { basis, work },
                ))
            }
            ResidentOperationTaskRefusal::Executor { basis, work } => {
                ResidentConsultationTaskRefusal::Executor(Box::new(ResidentConsultationTaskRetry {
                    coordinate,
                    retained: Box::new(RetainedConsultationTask { basis, work }),
                }))
            }
            ResidentOperationTaskRefusal::ExecutorProtocolLoss {
                unopened_basis: _,
                expected_key,
                returned,
            } => ResidentConsultationTaskRefusal::ExecutorProtocolLoss {
                expected_key,
                returned,
            },
        });
        let result = match transition {
            Ok(()) => outcome,
            Err(transition) => Err(ResidentConsultationTaskRefusal::AggregateTransition {
                transition,
                outcome: Box::new(match outcome {
                    Ok(dispatch) => {
                        ResidentConsultationTaskAggregateTransitionOutcome::Accepted { dispatch }
                    }
                    Err(refusal) => {
                        ResidentConsultationTaskAggregateTransitionOutcome::Refused(refusal)
                    }
                }),
            }),
        };
        if let Some(member_index) = member_index {
            match &result {
                Ok(dispatch) => {
                    self.context_state[member_index] =
                        ConsultationTaskContextState::Accepted(dispatch.clone());
                }
                Err(
                    ResidentConsultationTaskRefusal::Saturated(retry)
                    | ResidentConsultationTaskRefusal::AggregateSaturated(retry)
                    | ResidentConsultationTaskRefusal::Executor(retry),
                ) => {
                    self.context_state[member_index] = ConsultationTaskContextState::RetryPending(
                        retry.retained.basis.dispatch.clone(),
                    );
                }
                Err(ResidentConsultationTaskRefusal::ExecutorProtocolLoss {
                    expected_key, ..
                }) => {
                    self.context_state[member_index] =
                        ConsultationTaskContextState::Accepted(expected_key.clone());
                }
                Err(
                    ResidentConsultationTaskRefusal::Duplicate(_)
                    | ResidentConsultationTaskRefusal::OfferIdentityExhausted(_)
                    | ResidentConsultationTaskRefusal::AggregateAssociation { .. },
                ) => {
                    self.context_state[member_index] = ConsultationTaskContextState::DurableOnly;
                }
                Err(ResidentConsultationTaskRefusal::AggregateTransition { outcome, .. }) => {
                    match outcome.as_ref() {
                        ResidentConsultationTaskAggregateTransitionOutcome::Accepted {
                            dispatch,
                        } => {
                            self.context_state[member_index] =
                                ConsultationTaskContextState::Accepted(dispatch.clone());
                        }
                        ResidentConsultationTaskAggregateTransitionOutcome::Refused(refusal) => {
                            match refusal {
                                ResidentConsultationTaskRefusal::Saturated(retry)
                                | ResidentConsultationTaskRefusal::AggregateSaturated(retry)
                                | ResidentConsultationTaskRefusal::Executor(retry) => {
                                    self.context_state[member_index] =
                                        ConsultationTaskContextState::RetryPending(
                                            retry.retained.basis.dispatch.clone(),
                                        );
                                }
                                ResidentConsultationTaskRefusal::ExecutorProtocolLoss {
                                    expected_key,
                                    ..
                                } => {
                                    self.context_state[member_index] =
                                        ConsultationTaskContextState::Accepted(
                                            expected_key.clone(),
                                        );
                                }
                                _ => {
                                    self.context_state[member_index] =
                                        ConsultationTaskContextState::DurableOnly;
                                }
                            }
                        }
                    }
                }
            }
        }
        result
    }
}

/// Exact process-local coordinate of one finite consultation batch.
///
/// The parent distinguishes concurrent Consult operations. The collection
/// frontier distinguishes the exact roster snapshot admitted for that parent.
/// This coordinate grants no authority and is never a replacement for durable
/// collection recovery.
#[derive(Clone)]
pub(in crate::resident) struct ResidentConsultationBatchCoordinate {
    owner_instance: ResidentOperationTaskOwnerInstance,
    store: AuthorityStoreCoordinate,
    parent: ImmutableRecordReferenceV1,
    collection_frontier: ImmutableRecordReferenceV1,
}

impl ResidentConsultationBatchCoordinate {
    fn from_batch(
        owner_instance: ResidentOperationTaskOwnerInstance,
        batch: &ConsultationChildDispatchBatch,
    ) -> Self {
        Self {
            owner_instance,
            store: batch.store().clone(),
            parent: batch.parent.clone(),
            collection_frontier: batch.collection_frontier.clone(),
        }
    }

    #[must_use]
    pub(in crate::resident) fn belongs_to(
        &self,
        owner_instance: &ResidentOperationTaskOwnerInstance,
    ) -> bool {
        self.owner_instance.is_same(owner_instance)
    }

    /// Borrow the exact store incarnation that owns this batch.
    #[must_use]
    pub(in crate::resident) const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the exact Consult parent.
    #[must_use]
    pub(in crate::resident) const fn parent(&self) -> &ImmutableRecordReferenceV1 {
        &self.parent
    }

    /// Borrow the roster frontier from which the finite batch was planned.
    #[must_use]
    pub(in crate::resident) const fn collection_frontier(&self) -> &ImmutableRecordReferenceV1 {
        &self.collection_frontier
    }
}

impl PartialEq for ResidentConsultationBatchCoordinate {
    fn eq(&self, other: &Self) -> bool {
        self.owner_instance.is_same(&other.owner_instance)
            && self.store == other.store
            && self.parent == other.parent
            && self.collection_frontier == other.collection_frontier
    }
}

impl Eq for ResidentConsultationBatchCoordinate {}

impl std::fmt::Debug for ResidentConsultationBatchCoordinate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentConsultationBatchCoordinate")
            .field("store", &self.store)
            .field("parent", &self.parent)
            .field("collection_frontier", &self.collection_frontier)
            .finish_non_exhaustive()
    }
}

/// Positive bound on consultation batches retained by one resident process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) struct ResidentConsultationTaskOwnerLimits {
    maximum_active_batches: NonZeroUsize,
}

impl ResidentConsultationTaskOwnerLimits {
    /// Bind the owner to an explicit number of concurrently retained batches.
    #[must_use]
    pub(in crate::resident) const fn new(maximum_active_batches: NonZeroUsize) -> Self {
        Self {
            maximum_active_batches,
        }
    }
}

struct ResidentConsultationOwnedBatch {
    coordinate: ResidentConsultationBatchCoordinate,
    tasks: ResidentConsultationTaskService,
}

/// Process-local result of notifying one exact consultation batch.
///
/// This is an advisory process fact. It proves neither durable cancellation
/// custody nor that any child stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentConsultationCancellationNotification {
    /// This process does not retain the exact owner-bound batch.
    NoLocalOwner,
    /// The exact batch exists, with no accepted child awaiting completion.
    NoAcceptedChildren,
    /// Accepted children were marked without calling their executor.
    BatchMarked {
        /// Children marked by this call.
        newly_marked: usize,
        /// Children already marked by an earlier local call.
        already_marked: usize,
        /// Locally inconsistent accepted entries that no task still owned.
        no_longer_accepted: usize,
    },
}

/// Exact process-local Consult owner selected for one accepted dispatch.
///
/// The coordinate and dispatch stay together so a later notification cannot
/// drift to another batch or child. This value grants no durable authority.
#[must_use = "notify this exact Consult owner only after durable cancellation custody"]
pub(in crate::resident) struct ResidentConsultationDispatchOwner {
    coordinate: ResidentConsultationBatchCoordinate,
    dispatch: ResidentOperationTaskKey,
}

/// Process-local Consult ownership of one exact durable dispatch.
pub(in crate::resident) enum ResidentConsultationDispatchOwnerSelection {
    /// No retained Consult batch owns the dispatch.
    Missing,
    /// Exactly one retained Consult batch owns the dispatch.
    Owned(Box<ResidentConsultationDispatchOwner>),
    /// More than one retained batch claims the dispatch.
    Ambiguous,
}

/// Bounded process-lifetime owner of independent finite consultation batches.
///
/// One Consult parent has at most one retained batch. A successor collection
/// frontier does not silently create a second executor owner for the same
/// children. Callers may retire a quiescent batch and recover a newer frontier;
/// durable child recovery still decides whether any child is eligible for its
/// first dispatch.
pub(in crate::resident) struct ResidentConsultationTaskOwner {
    owner_instance: ResidentOperationTaskOwnerInstance,
    limits: ResidentConsultationTaskOwnerLimits,
    batches: Vec<ResidentConsultationOwnedBatch>,
}

impl ResidentConsultationTaskOwner {
    /// Reserve the complete process-local batch table before accepting work.
    pub(in crate::resident) fn new_in(
        owner_instance: ResidentOperationTaskOwnerInstance,
        limits: ResidentConsultationTaskOwnerLimits,
    ) -> Result<Self, ResidentConsultationTaskOwnerBuildError> {
        let mut batches = Vec::new();
        batches
            .try_reserve_exact(limits.maximum_active_batches.get())
            .map_err(|_| ResidentConsultationTaskOwnerBuildError::Allocation)?;
        Ok(Self {
            owner_instance,
            limits,
            batches,
        })
    }

    /// Return the number of exact consultation batches retained by this process.
    #[must_use]
    pub(in crate::resident) fn active_batches(&self) -> usize {
        self.batches.len()
    }

    /// Return all accepted child work still awaiting a join.
    #[must_use]
    pub(in crate::resident) fn in_flight(&self) -> usize {
        self.batches
            .iter()
            .map(|batch| batch.tasks.in_flight())
            .sum()
    }

    /// Retain one exact finite batch without selecting or releasing child work.
    pub(in crate::resident) fn open_batch(
        &mut self,
        planned: ConsultationChildDispatchBatch,
        task_limits: ResidentConsultationTaskLimits,
    ) -> Result<ResidentConsultationBatchCoordinate, ResidentConsultationBatchOpenError> {
        if let Some(existing) = self.batches.iter().find(|batch| {
            batch.coordinate.store == *planned.store() && batch.coordinate.parent == planned.parent
        }) {
            return if existing.coordinate.collection_frontier == planned.collection_frontier {
                Err(ResidentConsultationBatchOpenError::Duplicate)
            } else {
                Err(ResidentConsultationBatchOpenError::ParentAlreadyActive)
            };
        }
        if self.batches.len() == self.limits.maximum_active_batches.get() {
            return Err(ResidentConsultationBatchOpenError::Saturated);
        }
        let tasks = ResidentConsultationTaskService::new_in(
            planned,
            task_limits,
            self.owner_instance.clone(),
        )
        .map_err(ResidentConsultationBatchOpenError::Batch)?;
        let coordinate = tasks.coordinate().clone();
        self.batches.push(ResidentConsultationOwnedBatch {
            coordinate: coordinate.clone(),
            tasks,
        });
        Ok(coordinate)
    }

    /// Reconcile one recovered roster with the process-local batch table.
    ///
    /// Exact batches are reused. A changed frontier replaces its predecessor
    /// only when that predecessor has no local continuation or aggregate
    /// reservation. Busy stale batches remain intact so restart or a sibling
    /// turn cannot grant a second executor owner for the same parent.
    pub(in crate::resident) fn reconcile_batch(
        &mut self,
        planned: ConsultationChildDispatchBatch,
        task_limits: ResidentConsultationTaskLimits,
        reservations: &ResidentAggregateTaskReservations,
    ) -> Result<ResidentConsultationBatchReconciliation, ResidentConsultationBatchReconcileError>
    {
        let matching_parent = self.batches.iter().position(|batch| {
            batch.coordinate.store == *planned.store() && batch.coordinate.parent == planned.parent
        });
        let Some(index) = matching_parent else {
            return self
                .open_batch(planned, task_limits)
                .map(ResidentConsultationBatchReconciliation::Opened)
                .map_err(ResidentConsultationBatchReconcileError::Open);
        };

        if self.batches[index].coordinate.collection_frontier == planned.collection_frontier {
            if self.batches[index].tasks.planned != planned {
                return Err(ResidentConsultationBatchReconcileError::CoordinateConflict);
            }
            return Ok(ResidentConsultationBatchReconciliation::Reused(
                self.batches[index].coordinate.clone(),
            ));
        }

        let retained = self.batches[index].coordinate.clone();
        if reservations.has_batch_reservations(&retained) {
            return Ok(ResidentConsultationBatchReconciliation::Busy {
                retained,
                reason: ResidentConsultationBatchRetentionReason::AggregateReservation,
            });
        }
        if let Some(reason) = self.batches[index].tasks.retirement_blocker() {
            return Ok(ResidentConsultationBatchReconciliation::Busy { retained, reason });
        }

        // Build the successor before removing the quiescent predecessor. An
        // allocation or semantic failure therefore leaves the old exact batch
        // available for a later bounded recovery turn.
        let tasks = ResidentConsultationTaskService::new_in(
            planned,
            task_limits,
            self.owner_instance.clone(),
        )
        .map_err(ResidentConsultationBatchReconcileError::Batch)?;
        let coordinate = tasks.coordinate().clone();
        self.batches[index] = ResidentConsultationOwnedBatch {
            coordinate: coordinate.clone(),
            tasks,
        };
        Ok(ResidentConsultationBatchReconciliation::Advanced {
            retired: retained,
            opened: coordinate,
        })
    }

    /// Return the exact local state retained for one roster candidate.
    #[must_use]
    pub(in crate::resident) fn candidate_state(
        &self,
        coordinate: &ResidentConsultationBatchCoordinate,
        candidate: &ConsultationChildDispatchCandidate,
    ) -> Option<ResidentConsultationCandidateLocalState> {
        if !coordinate.belongs_to(&self.owner_instance) {
            return None;
        }
        self.batches
            .iter()
            .find(|batch| &batch.coordinate == coordinate)
            .and_then(|batch| batch.tasks.candidate_state(candidate))
    }

    /// Borrow the exact local batch retained for one durable collection parent.
    #[must_use]
    pub(in crate::resident) fn retained_batch_coordinate(
        &self,
        store: &AuthorityStoreCoordinate,
        parent: &ImmutableRecordReferenceV1,
    ) -> Option<&ResidentConsultationBatchCoordinate> {
        self.batches
            .iter()
            .find(|batch| batch.coordinate.store() == store && batch.coordinate.parent() == parent)
            .map(|batch| &batch.coordinate)
    }

    /// Select the sole retained Consult owner of one accepted dispatch.
    ///
    /// Selection reads process-local state only. It performs no storage,
    /// signing, anchor, executor, or provider work.
    #[must_use]
    pub(in crate::resident) fn select_dispatch_owner(
        &self,
        dispatch: &ResidentOperationTaskKey,
    ) -> ResidentConsultationDispatchOwnerSelection {
        let mut owners = self
            .batches
            .iter()
            .filter(|batch| batch.tasks.owns_dispatch(dispatch));
        let Some(owner) = owners.next() else {
            return ResidentConsultationDispatchOwnerSelection::Missing;
        };
        if owners.next().is_some() {
            return ResidentConsultationDispatchOwnerSelection::Ambiguous;
        }
        ResidentConsultationDispatchOwnerSelection::Owned(Box::new(
            ResidentConsultationDispatchOwner {
                coordinate: owner.coordinate.clone(),
                dispatch: dispatch.clone(),
            },
        ))
    }

    /// Mark one previously selected Consult child after durable cancellation.
    ///
    /// The notification is process-local and advisory. It does not call an
    /// executor or prove that provider work stopped.
    #[must_use]
    pub(in crate::resident) fn note_dispatch_cancellation_notification(
        &mut self,
        owner: ResidentConsultationDispatchOwner,
    ) -> ResidentOperationTaskCancellationNotification {
        if !owner.coordinate.belongs_to(&self.owner_instance) {
            return ResidentOperationTaskCancellationNotification::NotAccepted;
        }
        let Some(batch) = self
            .batches
            .iter_mut()
            .find(|batch| batch.coordinate == owner.coordinate)
        else {
            return ResidentOperationTaskCancellationNotification::NotAccepted;
        };
        batch.tasks.note_cancellation_notification(&owner.dispatch)
    }

    /// Mark accepted work in one exact owner-bound batch after durable cancellation.
    ///
    /// The coordinate must match this process owner, store incarnation, parent,
    /// and predecessor collection frontier. The method does not call an
    /// executor, retire the batch, or change unopened and pending-join members.
    #[must_use]
    pub(in crate::resident) fn note_cancellation_notification(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
    ) -> ResidentConsultationCancellationNotification {
        if !coordinate.belongs_to(&self.owner_instance) {
            return ResidentConsultationCancellationNotification::NoLocalOwner;
        }
        let Some(batch) = self
            .batches
            .iter_mut()
            .find(|batch| &batch.coordinate == coordinate)
        else {
            return ResidentConsultationCancellationNotification::NoLocalOwner;
        };

        let ResidentConsultationTaskService {
            tasks,
            context_state,
            ..
        } = &mut batch.tasks;
        let mut newly_marked = 0usize;
        let mut already_marked = 0usize;
        let mut no_longer_accepted = 0usize;
        let mut accepted = 0usize;
        for state in context_state.iter() {
            let ConsultationTaskContextState::Accepted(dispatch) = state else {
                continue;
            };
            accepted += 1;
            match tasks.note_cancellation_notification(dispatch) {
                ResidentOperationTaskCancellationNotification::Notified => newly_marked += 1,
                ResidentOperationTaskCancellationNotification::AlreadyNotified => {
                    already_marked += 1;
                }
                ResidentOperationTaskCancellationNotification::NotAccepted => {
                    no_longer_accepted += 1;
                }
            }
        }
        if accepted == 0 {
            return ResidentConsultationCancellationNotification::NoAcceptedChildren;
        }
        ResidentConsultationCancellationNotification::BatchMarked {
            newly_marked,
            already_marked,
            no_longer_accepted,
        }
    }

    /// Retain and offer one candidate from the already recovered turn head.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retain_and_accept_recovered<
        Work,
        Output,
        LookupError,
        RetentionFailure,
        RecoverChild,
        Retain,
        Executor,
    >(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
        candidate: &ConsultationChildDispatchCandidate,
        reservations: &mut ResidentAggregateTaskReservations,
        current: &RecoveredConsultationCollection,
        observed_at: pale_core::records::UnixTimeV1,
        recover_child: RecoverChild,
        retain: Retain,
        executor: &mut Executor,
    ) -> Result<
        ResidentOperationTaskKey,
        Box<ResidentConsultationTaskOwnerAcceptanceFailure<Work, LookupError, RetentionFailure>>,
    >
    where
        RecoverChild: FnOnce(
            &ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, LookupError>,
        Retain: FnOnce(
            &ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        if !coordinate.belongs_to(&self.owner_instance) {
            return Err(Box::new(
                ResidentConsultationTaskOwnerAcceptanceFailure::OwnerMismatch(candidate.clone()),
            ));
        }
        let Some(batch) = self
            .batches
            .iter_mut()
            .find(|batch| &batch.coordinate == coordinate)
        else {
            return Err(Box::new(
                ResidentConsultationTaskOwnerAcceptanceFailure::BatchNotFound(candidate.clone()),
            ));
        };
        batch
            .tasks
            .retain_and_accept_recovered_reserved(
                candidate,
                reservations,
                current,
                observed_at,
                recover_child,
                retain,
                executor,
            )
            .map_err(|failure| {
                Box::new(ResidentConsultationTaskOwnerAcceptanceFailure::Acceptance(
                    failure,
                ))
            })
    }

    /// Prepare exact resumed ownership before recovered provider preflight.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn prepare_resumed(
        &self,
        coordinate: &ResidentConsultationBatchCoordinate,
        candidate: &ConsultationChildDispatchCandidate,
        reservations: &ResidentAggregateTaskReservations,
        current: &RecoveredConsultationCollection,
        observed_at: pale_core::records::UnixTimeV1,
        child_store: &AuthorityStoreCoordinate,
        child_operation: &IssuedLocalOperation,
        dispatch_ref: &pale_core::records::ResidentRecordRefV1,
        dispatch: &pale_core::records::OperationObservationV1,
    ) -> Result<
        PreparedResidentConsultationTaskResume,
        ResidentConsultationTaskResumePreparationFailure,
    > {
        if !coordinate.belongs_to(&self.owner_instance) {
            return Err(ResidentConsultationTaskResumePreparationFailure::OwnerMismatch);
        }
        let Some(batch) = self
            .batches
            .iter()
            .find(|batch| &batch.coordinate == coordinate)
        else {
            return Err(ResidentConsultationTaskResumePreparationFailure::BatchNotFound);
        };
        let basis = batch.tasks.prepare_resumed_reserved(
            candidate,
            reservations,
            current,
            observed_at,
            child_store,
            child_operation,
            dispatch_ref,
            dispatch,
        )?;
        Ok(PreparedResidentConsultationTaskResume {
            coordinate: coordinate.clone(),
            basis,
        })
    }

    /// Prepare one historical dispatch for executor-free loss handling.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn prepare_recovered_loss(
        &self,
        coordinate: &ResidentConsultationBatchCoordinate,
        candidate: &ConsultationChildDispatchCandidate,
        reservations: &ResidentAggregateTaskReservations,
        current: &RecoveredConsultationCollection,
        observed_at: pale_core::records::UnixTimeV1,
        child_store: &AuthorityStoreCoordinate,
        child_operation: &IssuedLocalOperation,
        dispatch_ref: &pale_core::records::ResidentRecordRefV1,
        dispatch: &pale_core::records::OperationObservationV1,
    ) -> Result<PreparedResidentConsultationTaskLoss, ResidentConsultationTaskLossPreparationFailure>
    {
        if !coordinate.belongs_to(&self.owner_instance) {
            return Err(ResidentConsultationTaskLossPreparationFailure::OwnerMismatch);
        }
        let Some(batch) = self
            .batches
            .iter()
            .find(|batch| &batch.coordinate == coordinate)
        else {
            return Err(ResidentConsultationTaskLossPreparationFailure::BatchNotFound);
        };
        let basis = batch.tasks.prepare_recovered_loss_reserved(
            candidate,
            reservations,
            current,
            observed_at,
            child_store,
            child_operation,
            dispatch_ref,
            dispatch,
        )?;
        Ok(PreparedResidentConsultationTaskLoss {
            coordinate: coordinate.clone(),
            basis,
        })
    }

    /// Move one exact historical dispatch directly into loss-pending ownership.
    pub(in crate::resident) fn commit_recovered_loss<Work, Output>(
        &mut self,
        prepared: PreparedResidentConsultationTaskLoss,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentConsultationTaskOwnerPoll<Work, Output>,
        ResidentConsultationTaskLossCommitFailure,
    > {
        let pending = self.offer_recovered_loss(prepared, reservations)?;
        self.retry_recovered_loss(pending, reservations)
    }

    /// Begin the exact aggregate offer without entering loss-pending ownership.
    pub(in crate::resident) fn offer_recovered_loss(
        &mut self,
        prepared: PreparedResidentConsultationTaskLoss,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentConsultationTaskLossOffer, ResidentConsultationTaskLossCommitFailure> {
        if !prepared.coordinate.belongs_to(&self.owner_instance) {
            return Err(ResidentConsultationTaskLossCommitFailure::Unavailable {
                prepared: Box::new(prepared),
                reason: ResidentConsultationTaskLossUnavailableReason::OwnerMismatch,
            });
        }
        let Some(batch) = self
            .batches
            .iter_mut()
            .find(|batch| batch.coordinate == prepared.coordinate)
        else {
            return Err(ResidentConsultationTaskLossCommitFailure::Unavailable {
                prepared: Box::new(prepared),
                reason: ResidentConsultationTaskLossUnavailableReason::BatchNotFound,
            });
        };
        let Some(member_index) = batch.tasks.member_index(&prepared.basis.candidate) else {
            return Err(ResidentConsultationTaskLossCommitFailure::Unavailable {
                prepared: Box::new(prepared),
                reason: ResidentConsultationTaskLossUnavailableReason::CandidateMismatch,
            });
        };
        if !matches!(
            batch.tasks.context_state[member_index],
            ConsultationTaskContextState::Unopened
        ) {
            return Err(ResidentConsultationTaskLossCommitFailure::Unavailable {
                prepared: Box::new(prepared),
                reason: ResidentConsultationTaskLossUnavailableReason::AlreadyReleased,
            });
        }
        let offer = match reservations.begin_offer(
            prepared.basis.dispatch.clone(),
            ResidentTaskReservationRoute::Consultation(prepared.coordinate.clone()),
        ) {
            Ok(offer) => offer,
            Err(reason) => {
                return Err(ResidentConsultationTaskLossCommitFailure::Prepared {
                    prepared: Box::new(prepared),
                    reason,
                });
            }
        };
        Ok(ResidentConsultationTaskLossOffer { prepared, offer })
    }

    /// Retry only the aggregate loss transition for an already offered dispatch.
    pub(in crate::resident) fn retry_recovered_loss<Work, Output>(
        &mut self,
        pending: ResidentConsultationTaskLossOffer,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentConsultationTaskOwnerPoll<Work, Output>,
        ResidentConsultationTaskLossCommitFailure,
    > {
        let coordinate = pending.prepared.coordinate.clone();
        let Some(batch) = self
            .batches
            .iter_mut()
            .find(|batch| batch.coordinate == coordinate)
        else {
            return Err(ResidentConsultationTaskLossCommitFailure::Offered {
                pending: Box::new(pending),
                reason: ResidentTaskReservationTransitionFailure::RouteMismatch,
            });
        };
        let Some(member_index) = batch.tasks.member_index(&pending.prepared.basis.candidate) else {
            return Err(ResidentConsultationTaskLossCommitFailure::Offered {
                pending: Box::new(pending),
                reason: ResidentTaskReservationTransitionFailure::RouteMismatch,
            });
        };
        if !matches!(
            batch.tasks.context_state[member_index],
            ConsultationTaskContextState::Unopened
        ) {
            return Err(ResidentConsultationTaskLossCommitFailure::Offered {
                pending: Box::new(pending),
                reason: ResidentTaskReservationTransitionFailure::Phase {
                    expected: crate::resident::task_reservation::ResidentTaskReservationPhase::Offered,
                    actual: crate::resident::task_reservation::ResidentTaskReservationPhase::LossPendingDurableHandling,
                },
            });
        }
        Self::finish_recovered_loss(batch, pending.prepared, pending.offer, reservations)
    }

    fn finish_recovered_loss<Work, Output>(
        batch: &mut ResidentConsultationOwnedBatch,
        prepared: PreparedResidentConsultationTaskLoss,
        offer: ResidentTaskReservationOffer,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentConsultationTaskOwnerPoll<Work, Output>,
        ResidentConsultationTaskLossCommitFailure,
    > {
        let settlement = match reservations.lose_offer(offer) {
            Ok(settlement) => settlement,
            Err(failure) => {
                return Err(ResidentConsultationTaskLossCommitFailure::Offered {
                    pending: Box::new(ResidentConsultationTaskLossOffer {
                        prepared,
                        offer: *failure.offer,
                    }),
                    reason: failure.reason,
                });
            }
        };
        let basis = prepared.basis;
        let coordinate = prepared.coordinate;
        batch.tasks.mark_loss_pending(&basis);
        let dispatch = basis.dispatch.clone();
        Ok(ResidentConsultationTaskOwnerPoll {
            coordinate: coordinate.clone(),
            continuation: ResidentConsultationContinuationToken {
                coordinate,
                dispatch: dispatch.clone(),
                kind: ResidentConsultationContinuationKind::Lost,
                settlement: Some(settlement),
            },
            outcome: ResidentConsultationTaskPoll::Lost(ResidentConsultationLostTask {
                basis,
                key: dispatch,
                cancellation: ResidentOperationTaskCancellationState::Unnotified,
            }),
        })
    }

    /// Offer one resumed work value through its already prepared exact owner.
    pub(in crate::resident) fn accept_prepared_resume<Work, Output, Executor>(
        &mut self,
        prepared: PreparedResidentConsultationTaskResume,
        reservations: &mut ResidentAggregateTaskReservations,
        work: Work,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentConsultationPreparedResumeFailure<Work>>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        if !prepared.coordinate.belongs_to(&self.owner_instance) {
            return Err(ResidentConsultationPreparedResumeFailure::Unavailable {
                prepared: Box::new(prepared),
                work,
            });
        }
        let Some(batch) = self
            .batches
            .iter_mut()
            .find(|batch| batch.coordinate == prepared.coordinate)
        else {
            return Err(ResidentConsultationPreparedResumeFailure::Unavailable {
                prepared: Box::new(prepared),
                work,
            });
        };
        batch
            .tasks
            .accept_prepared_resume_reserved(prepared, reservations, work, executor)
    }

    /// Recheck, retain, and offer one fresh child through its exact batch.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retain_and_accept<
        Work,
        Output,
        LookupError,
        RetentionFailure,
        Lookup,
        RecoverChild,
        Retain,
        Executor,
    >(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
        candidate: &ConsultationChildDispatchCandidate,
        reservations: &mut ResidentAggregateTaskReservations,
        clock: &dyn WallClockSourcePort,
        lookup: Lookup,
        recover_child: RecoverChild,
        retain: Retain,
        executor: &mut Executor,
    ) -> Result<
        ResidentOperationTaskKey,
        Box<ResidentConsultationTaskOwnerAcceptanceFailure<Work, LookupError, RetentionFailure>>,
    >
    where
        Lookup: FnOnce() -> Result<RecoveredConsultationCollection, LookupError>,
        RecoverChild: FnOnce(
            &ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, LookupError>,
        Retain: FnOnce(
            &ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        if reservations.require_capacity().is_err() {
            return Err(Box::new(
                ResidentConsultationTaskOwnerAcceptanceFailure::Acceptance(Box::new(
                    ResidentConsultationTaskAcceptanceFailure::AggregateSaturated(
                        candidate.clone(),
                    ),
                )),
            ));
        }
        if !coordinate.belongs_to(&self.owner_instance) {
            return Err(Box::new(
                ResidentConsultationTaskOwnerAcceptanceFailure::OwnerMismatch(candidate.clone()),
            ));
        }
        let Some(batch) = self
            .batches
            .iter_mut()
            .find(|batch| &batch.coordinate == coordinate)
        else {
            return Err(Box::new(
                ResidentConsultationTaskOwnerAcceptanceFailure::BatchNotFound(candidate.clone()),
            ));
        };
        batch
            .tasks
            .retain_and_accept_reserved(
                candidate,
                reservations,
                clock,
                lookup,
                recover_child,
                retain,
                executor,
            )
            .map_err(|failure| {
                Box::new(ResidentConsultationTaskOwnerAcceptanceFailure::Acceptance(
                    failure,
                ))
            })
    }

    /// Retry through the exact batch and the process-wide aggregate ledger.
    pub(in crate::resident) fn retry_accept_reserved<Work, Output, Executor>(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
        retry: ResidentConsultationTaskRetry<Work>,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentConsultationTaskRetryFailure<Work>>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        if !coordinate.belongs_to(&self.owner_instance)
            || !retry.coordinate.belongs_to(&self.owner_instance)
        {
            return Err(ResidentConsultationTaskRetryFailure::OwnerMismatch(
                Box::new(retry),
            ));
        }
        let Some(batch) = self
            .batches
            .iter_mut()
            .find(|batch| &batch.coordinate == coordinate)
        else {
            return Err(ResidentConsultationTaskRetryFailure::BatchNotFound(
                Box::new(retry),
            ));
        };
        batch
            .tasks
            .retry_accept_reserved(coordinate, retry, reservations, executor)
    }

    /// Poll one shared executor and route the result by durable dispatch key.
    ///
    /// The owner performs no buffering. A stale or ambiguously owned result is
    /// returned immediately and cannot remove task state from any batch.
    pub(in crate::resident) fn try_completion_reserved<Work, Output, Executor>(
        &mut self,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
    ) -> Option<
        Result<
            ResidentConsultationTaskOwnerPoll<Work, Output>,
            ResidentConsultationTaskOwnerPollError<Work, Output>,
        >,
    >
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        let completion = executor.try_completion()?;
        Some(self.join_completion_reserved(completion, reservations))
    }

    /// Retry only the aggregate transition for an already-returned result.
    ///
    /// This path does not poll the executor. It selects the exact retained
    /// batch again so stale or ambiguous ownership remains fail closed.
    pub(in crate::resident) fn retry_completion_reserved<Work, Output>(
        &mut self,
        completion: ResidentOperationTaskCompletion<Work, Output>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentConsultationTaskOwnerPoll<Work, Output>,
        ResidentConsultationTaskOwnerPollError<Work, Output>,
    > {
        self.join_completion_reserved(completion, reservations)
    }

    fn join_completion_reserved<Work, Output>(
        &mut self,
        completion: ResidentOperationTaskCompletion<Work, Output>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentConsultationTaskOwnerPoll<Work, Output>,
        ResidentConsultationTaskOwnerPollError<Work, Output>,
    > {
        let dispatch = match &completion {
            ResidentOperationTaskCompletion::Completed { task, .. } => task.key(),
            ResidentOperationTaskCompletion::Lost { key, .. } => key,
        };
        let mut batch_index = None;
        for (index, batch) in self.batches.iter().enumerate() {
            if batch.tasks.owns_dispatch(dispatch) {
                if batch_index.is_some() {
                    return Err(ResidentConsultationTaskOwnerPollError::Ambiguous(
                        completion,
                    ));
                }
                batch_index = Some(index);
            }
        }
        let Some(batch_index) = batch_index else {
            return Err(ResidentConsultationTaskOwnerPollError::Stale(completion));
        };
        let coordinate = self.batches[batch_index].coordinate.clone();
        let handled = self.batches[batch_index]
            .tasks
            .join_reserved_completion(completion, reservations);
        let (outcome, settlement) = match handled {
            ResidentConsultationReservedTaskPoll::Handled {
                outcome,
                settlement,
            } => (outcome, settlement),
            ResidentConsultationReservedTaskPoll::Stale(completion) => {
                return Err(ResidentConsultationTaskOwnerPollError::Stale(completion));
            }
            ResidentConsultationReservedTaskPoll::ReservationRejected { completion, reason } => {
                return Err(
                    ResidentConsultationTaskOwnerPollError::ReservationRejected {
                        completion,
                        reason,
                    },
                );
            }
        };
        let (dispatch, kind) = match outcome.as_ref() {
            ResidentConsultationTaskPoll::Completed { dispatch, .. } => (
                dispatch.clone(),
                ResidentConsultationContinuationKind::Completed,
            ),
            ResidentConsultationTaskPoll::Lost(lost) => (
                lost.dispatch().clone(),
                ResidentConsultationContinuationKind::Lost,
            ),
            ResidentConsultationTaskPoll::Stale(_) => {
                unreachable!("reserved join converts stale completions before this point")
            }
        };
        let continuation = ResidentConsultationContinuationToken {
            coordinate: coordinate.clone(),
            dispatch,
            kind,
            settlement: Some(settlement),
        };
        Ok(ResidentConsultationTaskOwnerPoll {
            coordinate,
            continuation,
            outcome: *outcome,
        })
    }

    /// Prove that one routed Consult result still owns its pending capacity.
    ///
    /// This borrowed check is used both before the initial parent join and
    /// before continuing an exact database or anchor candidate. It performs no
    /// I/O and changes no process-local state.
    pub(super) fn require_pending_terminal<Work, Output>(
        &self,
        poll: &ResidentConsultationTaskOwnerPoll<Work, Output>,
        reservations: &ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentConsultationTerminalJoinPreflightFailure> {
        if !poll.coordinate.belongs_to(&self.owner_instance)
            || !poll
                .continuation
                .coordinate
                .belongs_to(&self.owner_instance)
        {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::OwnerMismatch);
        }
        if poll.coordinate != poll.continuation.coordinate {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::BatchMismatch);
        }
        if poll.coordinate.store() != poll.continuation.dispatch.store() {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::StoreMismatch);
        }
        let Some(batch) = self
            .batches
            .iter()
            .find(|batch| batch.coordinate == poll.coordinate)
        else {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::BatchNotFound);
        };
        let (candidate, dispatch, kind) = match &poll.outcome {
            ResidentConsultationTaskPoll::Completed {
                candidate,
                dispatch,
                ..
            } => (
                candidate,
                dispatch,
                ResidentConsultationContinuationKind::Completed,
            ),
            ResidentConsultationTaskPoll::Lost(lost) => (
                lost.candidate(),
                lost.dispatch(),
                ResidentConsultationContinuationKind::Lost,
            ),
            ResidentConsultationTaskPoll::Stale(_) => {
                return Err(ResidentConsultationTerminalJoinPreflightFailure::Stale);
            }
        };
        if dispatch != &poll.continuation.dispatch {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::DispatchMismatch);
        }
        if kind != poll.continuation.kind {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::ContinuationKind);
        }
        let Some(member_index) = batch.tasks.member_index(candidate) else {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::CandidateMismatch);
        };
        let expected_context = match kind {
            ResidentConsultationContinuationKind::Completed => {
                ResidentConsultationTaskContextKind::CompletionPendingJoin
            }
            ResidentConsultationContinuationKind::Lost => {
                ResidentConsultationTaskContextKind::LossPendingRecovery
            }
        };
        let actual_context = match &batch.tasks.context_state[member_index] {
            ConsultationTaskContextState::CompletionPendingJoin(current) if current == dispatch => {
                ResidentConsultationTaskContextKind::CompletionPendingJoin
            }
            ConsultationTaskContextState::LossPendingRecovery(current) if current == dispatch => {
                ResidentConsultationTaskContextKind::LossPendingRecovery
            }
            _ => return Err(ResidentConsultationTerminalJoinPreflightFailure::NotPending),
        };
        if actual_context != expected_context {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::ContinuationKind);
        }
        let Some(settlement) = poll.continuation.settlement.as_ref() else {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::AggregateMissing);
        };
        match kind {
            ResidentConsultationContinuationKind::Completed => {
                reservations.require_consultation_completion(settlement, dispatch, &poll.coordinate)
            }
            ResidentConsultationContinuationKind::Lost => {
                reservations.require_consultation_loss(settlement, dispatch, &poll.coordinate)
            }
        }
        .map_err(ResidentConsultationTerminalJoinPreflightFailure::Aggregate)
    }

    /// Prove that one routed child result can enter its exact parent join.
    ///
    /// This check is entirely borrowed. It rejects a replacement process
    /// owner, store, batch, candidate, dispatch, continuation phase, or
    /// aggregate settlement before collection custody performs any I/O. The
    /// protected child must select the retained dispatch from its admitted
    /// history and expose a terminal authenticated head.
    pub(super) fn require_terminal_join<Work, Output>(
        &self,
        poll: &ResidentConsultationTaskOwnerPoll<Work, Output>,
        current: &RecoveredConsultationCollection,
        input: &ConsultationChildJoinInput<'_>,
        reservations: &ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentConsultationTerminalJoinPreflightFailure> {
        self.require_pending_terminal(poll, reservations)?;
        if current.store() != poll.coordinate.store() {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::StoreMismatch);
        }
        let batch = self
            .batches
            .iter()
            .find(|batch| batch.coordinate == poll.coordinate)
            .expect("pending-terminal preflight retained the exact batch");
        // The batch coordinate retains the original roster frontier. A sibling
        // may already have advanced the recovered parent to a legitimate
        // successor, so compare the stable parent here and let
        // `classify_released_child` prove that this exact candidate still
        // remains outstanding in the successor.
        if current.collection().parent() != poll.coordinate.parent() {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::BatchMismatch);
        }
        let (candidate, dispatch) = match &poll.outcome {
            ResidentConsultationTaskPoll::Completed {
                candidate,
                dispatch,
                ..
            } => (candidate, dispatch),
            ResidentConsultationTaskPoll::Lost(lost) => (lost.candidate(), lost.dispatch()),
            ResidentConsultationTaskPoll::Stale(_) => {
                unreachable!("pending-terminal preflight rejected stale work")
            }
        };
        if input.slot_id != candidate.slot_id()
            || input.child.handle.record_ref() != &candidate.child().record_ref
            || input.child.handle.record_sha256().as_str()
                != candidate.child().record_sha256.as_str()
        {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::CandidateMismatch);
        }
        let (selected_ref, selected) = input.child.selected();
        if selected_ref != &dispatch.dispatch().record_ref
            || selected.canonical_operation_observation_sha256().as_str()
                != dispatch.dispatch().record_sha256.as_str()
        {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::DispatchMismatch);
        }
        let phase = if current.collection().stopped().is_some() {
            let planned_candidate = batch
                .tasks
                .planned
                .candidates()
                .iter()
                .find(|planned| *planned == candidate)
                .ok_or(ResidentConsultationTerminalJoinPreflightFailure::CandidateMismatch)?;
            if planned_candidate.child() != candidate.child()
                || planned_candidate.slot_id() != candidate.slot_id()
            {
                return Err(ResidentConsultationTerminalJoinPreflightFailure::CandidateMismatch);
            }
            let phase = super::classify_recovered_child(current, candidate, input.child)
                .map_err(ResidentConsultationTerminalJoinPreflightFailure::Revalidation)?;
            if phase == ProtectedConsultationChildPhase::TerminalJoin {
                let preview = current
                    .admit_recovered_child_terminal(
                        input.slot_id,
                        input.child,
                        input.recorded_at,
                        input.limits,
                        |_, _| Ok(()),
                    )
                    .map_err(ResidentConsultationTerminalJoinPreflightFailure::Revalidation)?;
                if preview.stopped() != current.collection().stopped()
                    || preview.slots() != current.collection().slots()
                {
                    return Err(
                        ResidentConsultationTerminalJoinPreflightFailure::FrozenStopChanged,
                    );
                }
            }
            phase
        } else {
            super::classify_released_child(
                current,
                &batch.tasks.planned,
                candidate,
                input.recorded_at,
                input.child,
            )
            .map_err(ResidentConsultationTerminalJoinPreflightFailure::Revalidation)?
        };
        if phase != ProtectedConsultationChildPhase::TerminalJoin {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::ChildPhase(phase));
        }
        let (parent_ref, parent) = input.parent.handle();
        if parent_ref != &poll.coordinate.parent().record_ref
            || parent.canonical_operation_handle_sha256().as_str()
                != poll.coordinate.parent().record_sha256.as_str()
        {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::BatchMismatch);
        }
        Ok(())
    }

    /// Prove that one completed or lost task reached durable uncertainty.
    ///
    /// The protected child retains the task's exact dispatch in its admitted
    /// history and exposes `OutcomeUnknown` as its authenticated head. The
    /// parent collection and its outstanding roster slot remain unchanged.
    pub(super) fn require_nonterminal_settlement<Work, Output>(
        &self,
        poll: &ResidentConsultationTaskOwnerPoll<Work, Output>,
        current: &RecoveredConsultationCollection,
        child: &RecoveredOperationObservation,
        reservations: &ResidentAggregateTaskReservations,
    ) -> Result<(), ResidentConsultationTerminalJoinPreflightFailure> {
        self.require_pending_terminal(poll, reservations)?;
        if current.store() != poll.coordinate.store()
            || current.collection().parent() != poll.coordinate.parent()
            || child.store() != poll.coordinate.store()
        {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::StoreMismatch);
        }
        let batch = self
            .batches
            .iter()
            .find(|batch| batch.coordinate == poll.coordinate)
            .expect("pending continuation retains its exact batch");
        let (candidate, dispatch) = match &poll.outcome {
            ResidentConsultationTaskPoll::Completed {
                candidate,
                dispatch,
                ..
            } => (candidate, dispatch),
            ResidentConsultationTaskPoll::Lost(lost) => (lost.candidate(), lost.dispatch()),
            ResidentConsultationTaskPoll::Stale(_) => {
                unreachable!("pending continuation preflight rejected stale work")
            }
        };
        let Some(planned) = batch
            .tasks
            .planned
            .candidates()
            .iter()
            .find(|planned| *planned == candidate)
        else {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::CandidateMismatch);
        };
        let Some(slot) = current
            .collection()
            .slots()
            .iter()
            .find(|slot| slot.roster().slot_id() == planned.slot_id())
        else {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::CandidateMismatch);
        };
        if slot.roster().requirement() != planned.requirement()
            || slot.roster().requirement_evidence() != planned.requirement_evidence()
            || slot.roster().participant() != planned.participant()
            || slot.roster().provider_binding() != planned.provider_binding()
            || slot.roster().context() != planned.context()
            || !matches!(
                slot.roster().source(),
                ConsultationContributionSource::FreshChild { child }
                    if child == planned.child()
            )
            || !matches!(slot.disposition(), ConsultationSlotDisposition::Outstanding)
        {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::CandidateMismatch);
        }
        let (selected_ref, selected) = child.selected();
        let (_, head) = child.selected_head();
        if selected_ref != &dispatch.dispatch().record_ref
            || selected.canonical_operation_observation_sha256().as_str()
                != dispatch.dispatch().record_sha256.as_str()
            || !matches!(
                selected.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Dispatched { .. },
                }
            )
            || !matches!(
                head.state(),
                OperationObservationStateV1::OutcomeUnknown { .. }
            )
            || head.revision() <= selected.revision()
        {
            return Err(ResidentConsultationTerminalJoinPreflightFailure::DispatchMismatch);
        }
        Ok(())
    }

    /// Release one poll only after its exact durable parent join has completed.
    ///
    /// The safe application route calls this method only with a joined
    /// collection value returned by the consultation coordinator. Any rejected
    /// aggregate transition reconstructs the complete poll so capacity and the
    /// child outcome remain recoverable without repeating the join.
    pub(super) fn finish_proved_continuation<Work, Output>(
        &mut self,
        poll: ResidentConsultationTaskOwnerPoll<Work, Output>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentConsultationTaskPoll<Work, Output>,
        ResidentConsultationTerminalReleaseRejection<Work, Output>,
    > {
        let ResidentConsultationTaskOwnerPoll {
            coordinate,
            continuation,
            outcome,
        } = poll;
        match self.release_proved_continuation_reserved(continuation, reservations) {
            ResidentConsultationTaskOwnerResolution::Continuation(
                ResidentConsultationContinuationResolution::Resolved,
            ) => Ok(outcome),
            rejected => {
                let reason = ResidentConsultationTerminalReleaseFailure::from_resolution(&rejected);
                let continuation = rejected
                    .into_rejected_token()
                    .expect("every rejected consultation resolution retains its token");
                Err(ResidentConsultationTerminalReleaseRejection {
                    poll: Box::new(ResidentConsultationTaskOwnerPoll {
                        coordinate,
                        continuation,
                        outcome,
                    }),
                    reason,
                })
            }
        }
    }

    fn release_proved_continuation_reserved(
        &mut self,
        continuation: ResidentConsultationContinuationToken,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentConsultationTaskOwnerResolution {
        if !continuation.coordinate.belongs_to(&self.owner_instance) {
            return ResidentConsultationTaskOwnerResolution::OwnerMismatch(continuation);
        }
        if continuation.dispatch.store() != continuation.coordinate.store() {
            return ResidentConsultationTaskOwnerResolution::BatchNotFound(continuation);
        }
        let coordinate = continuation.coordinate.clone();
        let Some(batch) = self
            .batches
            .iter_mut()
            .find(|batch| batch.coordinate == coordinate)
        else {
            return ResidentConsultationTaskOwnerResolution::BatchNotFound(continuation);
        };
        let resolution = batch
            .tasks
            .resolve_continuation_reserved(continuation, reservations);
        ResidentConsultationTaskOwnerResolution::Continuation(resolution)
    }

    /// Retire only a batch with no accepted child work awaiting a join.
    ///
    /// Retirement does not mark the durable collection complete. A later
    /// reconstruction must still recover the parent and every child before it
    /// can release work, so dropping this process-local hint grants no replay.
    #[must_use]
    pub(in crate::resident) fn retire_quiescent(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
    ) -> ResidentConsultationBatchRetirement {
        let Some(index) = self
            .batches
            .iter()
            .position(|batch| &batch.coordinate == coordinate)
        else {
            return ResidentConsultationBatchRetirement::NotFound;
        };
        if let Some(reason) = self.batches[index].tasks.retirement_blocker() {
            return ResidentConsultationBatchRetirement::Busy(reason);
        }
        self.batches.remove(index);
        ResidentConsultationBatchRetirement::Retired
    }
}

/// Why fresh Consult admission could not select and use one exact batch.
pub(in crate::resident) enum ResidentConsultationTaskOwnerAcceptanceFailure<
    Work,
    LookupError,
    RetentionFailure,
> {
    /// The selected batch belongs to another process-lifetime task state.
    OwnerMismatch(ConsultationChildDispatchCandidate),
    /// This process does not retain the selected exact batch.
    BatchNotFound(ConsultationChildDispatchCandidate),
    /// The exact batch rejected release, retention, or executor acceptance.
    Acceptance(Box<ResidentConsultationTaskAcceptanceFailure<Work, LookupError, RetentionFailure>>),
}

/// Why exact same-process resumed ownership was not prepared.
pub(in crate::resident) enum ResidentConsultationTaskResumePreparationFailure {
    OwnerMismatch,
    BatchNotFound,
    AggregateSaturated,
    NotPlanned,
    AlreadyReleased,
    Saturated,
    Revalidation(crate::resident::ConsultationCollectionError),
    ChildAssociation,
    DispatchPhase,
    DispatchIdentity,
    Duplicate,
}

/// Why a historical Consult dispatch could not acquire executor-free loss ownership.
pub(in crate::resident) enum ResidentConsultationTaskLossPreparationFailure {
    OwnerMismatch,
    BatchNotFound,
    NotPlanned,
    AlreadyReleased,
    Revalidation(crate::resident::ConsultationCollectionError),
    ChildAssociation,
    DispatchPhase,
    DispatchIdentity,
    Duplicate,
    Reservation(ResidentTaskReservationOfferFailure),
}

impl std::fmt::Debug for ResidentConsultationTaskLossPreparationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::OwnerMismatch => "OwnerMismatch",
            Self::BatchNotFound => "BatchNotFound",
            Self::NotPlanned => "NotPlanned",
            Self::AlreadyReleased => "AlreadyReleased",
            Self::Revalidation(reason) => {
                let _redacted = reason;
                "Revalidation"
            }
            Self::ChildAssociation => "ChildAssociation",
            Self::DispatchPhase => "DispatchPhase",
            Self::DispatchIdentity => "DispatchIdentity",
            Self::Duplicate => "Duplicate",
            Self::Reservation(reason) => {
                let _redacted = reason;
                "Reservation"
            }
        };
        formatter
            .debug_struct("ResidentConsultationTaskLossPreparationFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Exact local owner returned when recovered loss could not reach `LossPending`.
pub(in crate::resident) enum ResidentConsultationTaskLossCommitFailure {
    Unavailable {
        prepared: Box<PreparedResidentConsultationTaskLoss>,
        reason: ResidentConsultationTaskLossUnavailableReason,
    },
    Prepared {
        prepared: Box<PreparedResidentConsultationTaskLoss>,
        reason: ResidentTaskReservationOfferFailure,
    },
    Offered {
        pending: Box<ResidentConsultationTaskLossOffer>,
        reason: ResidentTaskReservationTransitionFailure,
    },
}

/// Exact local mismatch found after recovered-loss preparation.
pub(in crate::resident) enum ResidentConsultationTaskLossUnavailableReason {
    OwnerMismatch,
    BatchNotFound,
    CandidateMismatch,
    AlreadyReleased,
}

impl std::fmt::Debug for ResidentConsultationTaskLossCommitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Unavailable { .. } => "Unavailable",
            Self::Prepared { .. } => "Prepared",
            Self::Offered { .. } => "Offered",
        };
        formatter
            .debug_struct("ResidentConsultationTaskLossCommitFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ResidentConsultationTaskResumePreparationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::OwnerMismatch => "OwnerMismatch",
            Self::BatchNotFound => "BatchNotFound",
            Self::AggregateSaturated => "AggregateSaturated",
            Self::NotPlanned => "NotPlanned",
            Self::AlreadyReleased => "AlreadyReleased",
            Self::Saturated => "Saturated",
            Self::Revalidation(reason) => {
                let _redacted = reason;
                "Revalidation"
            }
            Self::ChildAssociation => "ChildAssociation",
            Self::DispatchPhase => "DispatchPhase",
            Self::DispatchIdentity => "DispatchIdentity",
            Self::Duplicate => "Duplicate",
        };
        formatter
            .debug_struct("ResidentConsultationTaskResumePreparationFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// A prepared resumed child was refused without dropping its constructed work.
pub(in crate::resident) enum ResidentConsultationPreparedResumeFailure<Work> {
    Unavailable {
        prepared: Box<PreparedResidentConsultationTaskResume>,
        work: Work,
    },
    Refused(Box<ResidentConsultationTaskRefusal<Work>>),
}

impl<Work> std::fmt::Debug for ResidentConsultationPreparedResumeFailure<Work> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Unavailable { prepared, work } => {
                let _redacted = (prepared, work);
                "Unavailable"
            }
            Self::Refused(refusal) => {
                let _redacted = refusal;
                "Refused"
            }
        };
        formatter
            .debug_struct("ResidentConsultationPreparedResumeFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl<Work, LookupError, RetentionFailure> std::fmt::Debug
    for ResidentConsultationTaskOwnerAcceptanceFailure<Work, LookupError, RetentionFailure>
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::OwnerMismatch(_) => "OwnerMismatch",
            Self::BatchNotFound(_) => "BatchNotFound",
            Self::Acceptance(_) => "Acceptance",
        };
        formatter
            .debug_struct("ResidentConsultationTaskOwnerAcceptanceFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Construction stopped before the process-local consultation owner was usable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentConsultationTaskOwnerBuildError {
    /// The complete bounded batch table could not be reserved.
    Allocation,
}

/// Why an exact finite batch did not enter process-local ownership.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentConsultationBatchOpenError {
    /// The exact parent and collection frontier are already retained.
    Duplicate,
    /// This parent is already retained at another collection frontier.
    ParentAlreadyActive,
    /// The explicit active-batch ceiling is full.
    Saturated,
    /// The batch itself did not satisfy its finite task-service bounds.
    Batch(ResidentConsultationTaskBuildError),
}

/// Result of reconciling one recovered collection with local batch ownership.
pub(in crate::resident) enum ResidentConsultationBatchReconciliation {
    /// The exact recovered frontier already has one local owner.
    Reused(ResidentConsultationBatchCoordinate),
    /// A first local owner was created for this parent.
    Opened(ResidentConsultationBatchCoordinate),
    /// A quiescent predecessor frontier was replaced by its recovered successor.
    Advanced {
        /// Exact local coordinate removed from the table.
        retired: ResidentConsultationBatchCoordinate,
        /// Exact successor coordinate now retained by this process.
        opened: ResidentConsultationBatchCoordinate,
    },
    /// A stale predecessor remains busy and was preserved unchanged.
    Busy {
        /// Exact retained stale coordinate.
        retained: ResidentConsultationBatchCoordinate,
        /// Local fact that prevents safe replacement.
        reason: ResidentConsultationBatchRetentionReason,
    },
}

impl ResidentConsultationBatchReconciliation {
    /// Borrow the usable exact coordinate when reconciliation selected one.
    #[must_use]
    pub(in crate::resident) const fn usable(&self) -> Option<&ResidentConsultationBatchCoordinate> {
        match self {
            Self::Reused(coordinate) | Self::Opened(coordinate) => Some(coordinate),
            Self::Advanced { opened, .. } => Some(opened),
            Self::Busy { .. } => None,
        }
    }

    /// Borrow the exact batch that remained live after reconciliation.
    #[must_use]
    pub(in crate::resident) const fn retained(&self) -> &ResidentConsultationBatchCoordinate {
        match self {
            Self::Reused(coordinate) | Self::Opened(coordinate) => coordinate,
            Self::Advanced { opened, .. } => opened,
            Self::Busy { retained, .. } => retained,
        }
    }
}

/// Why local batch reconciliation could not establish an exact owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentConsultationBatchReconcileError {
    /// The same parent and frontier were presented with different roster bytes.
    CoordinateConflict,
    /// A first batch could not enter the bounded table.
    Open(ResidentConsultationBatchOpenError),
    /// A successor batch failed construction before replacing its predecessor.
    Batch(ResidentConsultationTaskBuildError),
}

/// Result of an exact process-local batch-retirement request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentConsultationBatchRetirement {
    /// The quiescent process-local batch was removed.
    Retired,
    /// No retained batch has the exact parent and frontier coordinate.
    NotFound,
    /// Process-local child context still requires explicit handling.
    Busy(ResidentConsultationBatchRetentionReason),
}

/// Why an exact consultation batch must remain in process-local ownership.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentConsultationBatchRetentionReason {
    /// Aggregate occupancy still belongs to this exact batch.
    AggregateReservation,
    /// Mechanically accepted work has not produced a completion or loss.
    Accepted,
    /// Unopened retained work remains available only through its retry token.
    RetryPending,
    /// Returned output has not yet reached durable parent collection custody.
    CompletionPendingJoin,
    /// Executor loss still requires protected child recovery and disposition.
    LossPendingRecovery,
}

/// Exact process-local ownership retained for one consultation candidate.
pub(in crate::resident) enum ResidentConsultationCandidateLocalState {
    /// The candidate has not crossed durable dispatch in this process.
    Unopened,
    /// Durable child state crossed release, but no local executor continuation exists.
    DurableOnly,
    /// Unopened retained work may only be retried with its linear token.
    RetryPending(ResidentOperationTaskKey),
    /// The executor owns the exact accepted dispatch.
    Accepted(ResidentOperationTaskKey),
    /// Returned output awaits durable parent collection custody.
    CompletionPendingJoin(ResidentOperationTaskKey),
    /// Executor loss awaits protected child recovery.
    LossPendingRecovery(ResidentOperationTaskKey),
}

/// Construction stopped before the bounded task service became usable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentConsultationTaskBuildError {
    /// The finite roster exceeds the explicit aggregate member ceiling.
    BatchTooLarge,
    /// More than one roster slot names the same fresh child operation.
    DuplicateChild,
    /// The release ledger could not reserve its complete finite capacity.
    Allocation,
    /// The generic operation task owner could not reserve its occupancy.
    TaskOwner(ResidentOperationTaskOwnerBuildError),
}

/// Why a child did not reach the executor.
pub(in crate::resident) enum ResidentConsultationTaskAcceptanceFailure<
    Work,
    LookupError,
    RetentionFailure,
> {
    /// The process-wide aggregate ceiling was full before lookup or time sampling.
    AggregateSaturated(ConsultationChildDispatchCandidate),
    /// The supplied candidate is not an exact member of the frozen batch.
    NotPlanned(ConsultationChildDispatchCandidate),
    /// This process already crossed the release boundary for the candidate.
    AlreadyReleased(ConsultationChildDispatchCandidate),
    /// The in-flight ceiling was full before any retention callback ran.
    Saturated(ConsultationChildDispatchCandidate),
    /// Fresh parent, time, or child release admission refused the candidate.
    Release(ConsultationChildDispatchReleaseFailure<LookupError>),
    /// The child owner proved that dispatch remained unopened.
    RetentionUnopened {
        candidate: ConsultationChildDispatchCandidate,
        reason: RetentionFailure,
    },
    /// Dispatch may have crossed; the service retained no replay permission.
    RetentionUncertain {
        candidate: ConsultationChildDispatchCandidate,
        reason: RetentionFailure,
    },
    /// Returned custody did not revalidate against the exact consultation child.
    RetainedRevalidation {
        candidate: ConsultationChildDispatchCandidate,
        work: Work,
        reason: crate::resident::ConsultationCollectionError,
    },
    /// Returned custody was not the exact newly dispatched child phase.
    RetainedPhase {
        candidate: ConsultationChildDispatchCandidate,
        work: Work,
        phase: ProtectedConsultationChildPhase,
    },
    /// The retained dispatch observation could not form its immutable key.
    DispatchIdentity {
        candidate: ConsultationChildDispatchCandidate,
        work: Work,
    },
    /// The process-local task owner or executor refused unopened retained work.
    TaskOwner(ResidentConsultationTaskRefusal<Work>),
}

impl<Work, LookupError, RetentionFailure> std::fmt::Debug
    for ResidentConsultationTaskAcceptanceFailure<Work, LookupError, RetentionFailure>
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::AggregateSaturated(_) => "AggregateSaturated",
            Self::NotPlanned(_) => "NotPlanned",
            Self::AlreadyReleased(_) => "AlreadyReleased",
            Self::Saturated(_) => "Saturated",
            Self::Release(_) => "Release",
            Self::RetentionUnopened { .. } => "RetentionUnopened",
            Self::RetentionUncertain { .. } => "RetentionUncertain",
            Self::RetainedRevalidation {
                candidate,
                work,
                reason,
            } => {
                let _redacted = (candidate, work, reason);
                "RetainedRevalidation"
            }
            Self::RetainedPhase {
                candidate,
                work,
                phase,
            } => {
                let _redacted = (candidate, work, phase);
                "RetainedPhase"
            }
            Self::DispatchIdentity { candidate, work } => {
                let _redacted = (candidate, work);
                "DispatchIdentity"
            }
            Self::TaskOwner(refusal) => {
                let _redacted = refusal;
                "TaskOwner"
            }
        };
        formatter
            .debug_struct("ResidentConsultationTaskAcceptanceFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Retained work refused by the process-local task owner or host executor.
pub(in crate::resident) enum ResidentConsultationTaskRefusal<Work> {
    /// The same dispatch key is or was already accepted; no retry token exists.
    Duplicate(Box<RetainedConsultationTask<Work>>),
    /// The explicit process-local occupancy ceiling is full.
    Saturated(Box<ResidentConsultationTaskRetry<Work>>),
    /// The process-wide aggregate ceiling is full.
    AggregateSaturated(Box<ResidentConsultationTaskRetry<Work>>),
    /// The host executor returned the work unopened.
    Executor(Box<ResidentConsultationTaskRetry<Work>>),
    /// The owner cannot allocate another exact process-local offer identity.
    OfferIdentityExhausted(Box<RetainedConsultationTask<Work>>),
    /// The executor retained the offered child but returned a different task.
    ExecutorProtocolLoss {
        expected_key: ResidentOperationTaskKey,
        returned: Box<ResidentOperationTask<Work>>,
    },
    /// The task and batch did not satisfy the aggregate ledger association.
    AggregateAssociation {
        retained: Box<RetainedConsultationTask<Work>>,
        reason: ResidentTaskReservationOfferFailure,
    },
    /// The executor outcome is known, but aggregate occupancy did not make its
    /// matching linear transition. The intact offer capability remains here.
    AggregateTransition {
        transition: ResidentTaskReservationOfferTransitionFailure,
        outcome: Box<ResidentConsultationTaskAggregateTransitionOutcome<Work>>,
    },
}

/// Mechanical Consult outcome retained after an aggregate transition fault.
pub(in crate::resident) enum ResidentConsultationTaskAggregateTransitionOutcome<Work> {
    /// The executor may own this exact retained child.
    Accepted { dispatch: ResidentOperationTaskKey },
    /// The ordinary refusal retains its retry or mismatch evidence.
    Refused(ResidentConsultationTaskRefusal<Work>),
}

/// Why a linear retry continuation did not reach its exact batch executor.
pub(in crate::resident) enum ResidentConsultationTaskRetryFailure<Work> {
    /// The retry belongs to another process-lifetime operation-task state.
    OwnerMismatch(Box<ResidentConsultationTaskRetry<Work>>),
    /// The caller tried to transplant the continuation to another batch.
    BatchMismatch(Box<ResidentConsultationTaskRetry<Work>>),
    /// The exact retained batch is no longer present in this process.
    BatchNotFound(Box<ResidentConsultationTaskRetry<Work>>),
    /// The batch no longer owns the exact pending retry continuation.
    ContinuationUnavailable(Box<ResidentConsultationTaskRetry<Work>>),
    /// The exact batch retried the unopened work, but acceptance still failed.
    Refused(Box<ResidentConsultationTaskRefusal<Work>>),
}

impl<Work> std::fmt::Debug for ResidentConsultationTaskRetryFailure<Work> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::OwnerMismatch(_) => "OwnerMismatch",
            Self::BatchMismatch(_) => "BatchMismatch",
            Self::BatchNotFound(_) => "BatchNotFound",
            Self::ContinuationUnavailable(_) => "ContinuationUnavailable",
            Self::Refused(_) => "Refused",
        };
        formatter
            .debug_struct("ResidentConsultationTaskRetryFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl<Work> std::fmt::Debug for ResidentConsultationTaskRefusal<Work> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Duplicate(retained) => {
                let _redacted = retained;
                "Duplicate"
            }
            Self::Saturated(_) => "Saturated",
            Self::AggregateSaturated(_) => "AggregateSaturated",
            Self::Executor(_) => "Executor",
            Self::OfferIdentityExhausted(_) => "OfferIdentityExhausted",
            Self::ExecutorProtocolLoss { .. } => "ExecutorProtocolLoss",
            Self::AggregateAssociation { .. } => "AggregateAssociation",
            Self::AggregateTransition { .. } => "AggregateTransition",
        };
        formatter
            .debug_struct("ResidentConsultationTaskRefusal")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl<Work> ResidentConsultationTaskRefusal<Work> {
    /// Consume a non-retryable duplicate and recover its exact unopened work.
    pub(in crate::resident) fn into_duplicate(
        self,
    ) -> Result<
        (
            ConsultationChildDispatchCandidate,
            ResidentOperationTaskKey,
            Work,
        ),
        Self,
    > {
        match self {
            Self::Duplicate(retained) => {
                let RetainedConsultationTask { basis, work } = *retained;
                Ok((basis.candidate, basis.dispatch, work))
            }
            other => Err(other),
        }
    }

    /// Return unopened work only for refusals whose retry is mechanically safe.
    fn into_retry(self) -> Option<ResidentConsultationTaskRetry<Work>> {
        match self {
            Self::Duplicate(_) => None,
            Self::Saturated(retry) | Self::AggregateSaturated(retry) | Self::Executor(retry) => {
                Some(*retry)
            }
            Self::OfferIdentityExhausted(_)
            | Self::ExecutorProtocolLoss { .. }
            | Self::AggregateAssociation { .. }
            | Self::AggregateTransition { .. } => None,
        }
    }
}

/// Linear unopened work returned by a retry-safe process-local refusal.
pub(in crate::resident) struct ResidentConsultationTaskRetry<Work> {
    coordinate: ResidentConsultationBatchCoordinate,
    retained: Box<RetainedConsultationTask<Work>>,
}

impl<Work> ResidentConsultationTaskRetry<Work> {
    pub(in crate::resident) const fn coordinate(&self) -> &ResidentConsultationBatchCoordinate {
        &self.coordinate
    }
}

/// Process-local context retained after executor acceptance or refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentConsultationTaskContextKind {
    CompletionPendingJoin,
    LossPendingRecovery,
}

impl ResidentConsultationTaskContextKind {
    fn from_state(state: &ConsultationTaskContextState) -> Self {
        match state {
            ConsultationTaskContextState::CompletionPendingJoin(_) => Self::CompletionPendingJoin,
            ConsultationTaskContextState::LossPendingRecovery(_) => Self::LossPendingRecovery,
            ConsultationTaskContextState::Unopened
            | ConsultationTaskContextState::DurableOnly
            | ConsultationTaskContextState::RetryPending(_)
            | ConsultationTaskContextState::Accepted(_) => {
                unreachable!("only pending continuation states are selected")
            }
        }
    }
}

/// The caller-owned durable handling that releases process-local batch context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResidentConsultationContinuationKind {
    /// A returned output reached its durable child-to-parent join.
    Completed,
    /// A lost executor task reached its explicit recovery disposition.
    Lost,
}

/// Opaque linear proof that one routed result still retains batch context.
///
/// This token is process-local bookkeeping, not authority or durable evidence.
/// Returning it asserts that the caller completed the corresponding durable
/// join or recovery path. Its private fields prevent reconstruction from public
/// coordinates, and it is intentionally neither cloneable nor copyable.
#[must_use = "complete exact durable handling before releasing this consultation reservation"]
pub(in crate::resident) struct ResidentConsultationContinuationToken {
    coordinate: ResidentConsultationBatchCoordinate,
    dispatch: ResidentOperationTaskKey,
    kind: ResidentConsultationContinuationKind,
    settlement: Option<ResidentTaskReservationSettlement>,
}

enum ResidentConsultationLocalResolution {
    Resolved {
        kind: ResidentConsultationContinuationKind,
        settlement: Option<ResidentTaskReservationSettlement>,
    },
    NotPending(ResidentConsultationContinuationToken),
    KindMismatch {
        continuation: ResidentConsultationContinuationToken,
        actual: ResidentConsultationTaskContextKind,
    },
}

/// Result of releasing one exact process-local continuation after durable work.
pub(in crate::resident) enum ResidentConsultationContinuationResolution {
    /// The exact pending continuation no longer retains its batch.
    Resolved,
    /// The selected batch does not own this dispatch continuation.
    NotPending(ResidentConsultationContinuationToken),
    /// The dispatch is pending, but for a different durable handling path.
    KindMismatch {
        continuation: ResidentConsultationContinuationToken,
        actual: ResidentConsultationTaskContextKind,
    },
    /// Exact aggregate release failed; the unchanged continuation remains retryable.
    AggregateReleaseRejected {
        continuation: ResidentConsultationContinuationToken,
        reason: ResidentTaskReservationTransitionFailure,
    },
    /// A legacy test-only token had no aggregate settlement capability.
    AggregateReservationMissing(ResidentConsultationContinuationToken),
}

/// Why a routed Consult result cannot enter durable parent custody.
///
/// Every variant is established without storage, signing, anchor, clock, or
/// executor I/O. The caller retains the complete routed poll and therefore the
/// exact aggregate reservation on every rejection.
pub(in crate::resident) enum ResidentConsultationTerminalJoinPreflightFailure {
    /// The poll or continuation belongs to another process-lifetime owner.
    OwnerMismatch,
    /// The protected parent or child uses another control-store incarnation.
    StoreMismatch,
    /// The process no longer retains this exact finite batch.
    BatchNotFound,
    /// The poll, recovered collection, or parent names another batch frontier.
    BatchMismatch,
    /// The routed outcome does not name a member of the frozen batch.
    CandidateMismatch,
    /// The protected child does not select the retained dispatch.
    DispatchMismatch,
    /// Completion and loss handling were substituted for one another.
    ContinuationKind,
    /// The exact candidate and dispatch are not awaiting durable handling.
    NotPending,
    /// A stale executor result has no consultation continuation.
    Stale,
    /// Protected child or collection readmission rejected the terminal join.
    Revalidation(crate::resident::ConsultationCollectionError),
    /// A late terminal preview attempted to rewrite the frozen stop result.
    FrozenStopChanged,
    /// The exact child requires a different protected recovery phase.
    ChildPhase(ProtectedConsultationChildPhase),
    /// A legacy test-only continuation has no aggregate settlement capability.
    AggregateMissing,
    /// The aggregate owner, key, route, or phase no longer matches.
    Aggregate(ResidentTaskReservationTransitionFailure),
}

impl std::fmt::Debug for ResidentConsultationTerminalJoinPreflightFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::OwnerMismatch => "OwnerMismatch",
            Self::StoreMismatch => "StoreMismatch",
            Self::BatchNotFound => "BatchNotFound",
            Self::BatchMismatch => "BatchMismatch",
            Self::CandidateMismatch => "CandidateMismatch",
            Self::DispatchMismatch => "DispatchMismatch",
            Self::ContinuationKind => "ContinuationKind",
            Self::NotPending => "NotPending",
            Self::Stale => "Stale",
            Self::Revalidation(reason) => {
                let _redacted = reason;
                "Revalidation"
            }
            Self::FrozenStopChanged => "FrozenStopChanged",
            Self::ChildPhase(phase) => {
                let _redacted = phase;
                "ChildPhase"
            }
            Self::AggregateMissing => "AggregateMissing",
            Self::Aggregate(reason) => {
                let _redacted = reason;
                "Aggregate"
            }
        };
        formatter
            .debug_struct("ResidentConsultationTerminalJoinPreflightFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// One shared-executor result routed to exactly one retained Consult batch.
#[must_use = "settle or preserve this exact completion and continuation"]
pub(in crate::resident) struct ResidentConsultationTaskOwnerPoll<Work, Output> {
    /// Exact Consult parent and roster frontier that own the result.
    pub(in crate::resident) coordinate: ResidentConsultationBatchCoordinate,
    /// Linear process-local context released only after durable handling.
    pub(in crate::resident) continuation: ResidentConsultationContinuationToken,
    /// Attributable completion or loss from that batch.
    pub(in crate::resident) outcome: ResidentConsultationTaskPoll<Work, Output>,
}

impl<Work, Output> ResidentConsultationTaskOwnerPoll<Work, Output> {
    /// Borrow the exact planned child and durable dispatch selected for a join.
    ///
    /// A stale executor response has no admitted roster basis and therefore
    /// cannot select terminal-join inputs.
    pub(in crate::resident) fn join_selection(
        &self,
    ) -> Option<(
        &ConsultationChildDispatchCandidate,
        &ResidentOperationTaskKey,
    )> {
        match &self.outcome {
            ResidentConsultationTaskPoll::Completed {
                candidate,
                dispatch,
                ..
            } => Some((candidate, dispatch)),
            ResidentConsultationTaskPoll::Lost(lost) => Some((lost.candidate(), lost.dispatch())),
            ResidentConsultationTaskPoll::Stale(_) => None,
        }
    }
}

/// Why one shared-executor result could not select a batch owner.
#[must_use = "preserve the returned completion until its ownership can be recovered"]
pub(in crate::resident) enum ResidentConsultationTaskOwnerPollError<Work, Output> {
    /// No retained batch owns this executor result.
    Stale(ResidentOperationTaskCompletion<Work, Output>),
    /// More than one retained batch claimed the key; no batch was mutated.
    Ambiguous(ResidentOperationTaskCompletion<Work, Output>),
    /// Aggregate custody did not transition, so the accepted task stayed active.
    ReservationRejected {
        completion: ResidentOperationTaskCompletion<Work, Output>,
        reason: ResidentTaskReservationTransitionFailure,
    },
}

impl<Work, Output> std::fmt::Debug for ResidentConsultationTaskOwnerPollError<Work, Output> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Stale(_) => "Stale",
            Self::Ambiguous(_) => "Ambiguous",
            Self::ReservationRejected { .. } => "ReservationRejected",
        };
        formatter
            .debug_struct("ResidentConsultationTaskOwnerPollError")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Result of resolving one exact batch continuation through its owner.
pub(in crate::resident) enum ResidentConsultationTaskOwnerResolution {
    /// The continuation belongs to another process-lifetime task state.
    OwnerMismatch(ResidentConsultationContinuationToken),
    /// The selected batch is not retained by this process.
    BatchNotFound(ResidentConsultationContinuationToken),
    /// Exact continuation resolution result from the retained batch.
    Continuation(ResidentConsultationContinuationResolution),
}

impl std::fmt::Debug for ResidentConsultationTaskOwnerResolution {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::OwnerMismatch(_) => "OwnerMismatch",
            Self::BatchNotFound(_) => "BatchNotFound",
            Self::Continuation(_) => "Continuation",
        };
        formatter
            .debug_struct("ResidentConsultationTaskOwnerResolution")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl ResidentConsultationTaskOwnerResolution {
    /// Recover a token rejected before its owning batch was touched.
    fn into_rejected_token(self) -> Option<ResidentConsultationContinuationToken> {
        match self {
            Self::OwnerMismatch(token) | Self::BatchNotFound(token) => Some(token),
            Self::Continuation(
                ResidentConsultationContinuationResolution::NotPending(token)
                | ResidentConsultationContinuationResolution::KindMismatch {
                    continuation: token,
                    ..
                }
                | ResidentConsultationContinuationResolution::AggregateReleaseRejected {
                    continuation: token,
                    ..
                }
                | ResidentConsultationContinuationResolution::AggregateReservationMissing(token),
            ) => Some(token),
            Self::Continuation(ResidentConsultationContinuationResolution::Resolved) => None,
        }
    }
}

/// Why a custody-proved terminal join could not release its exact reservation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentConsultationTerminalReleaseFailure {
    /// The continuation belongs to another process-lifetime task owner.
    OwnerMismatch,
    /// The process no longer retains the exact finite batch.
    BatchNotFound,
    /// The batch does not own this candidate and dispatch in a pending phase.
    NotPending,
    /// Completion and loss handling were substituted for one another.
    KindMismatch,
    /// Aggregate ownership, key, route, or phase no longer matches.
    Aggregate(ResidentTaskReservationTransitionFailure),
    /// A legacy test-only continuation has no aggregate settlement capability.
    AggregateMissing,
}

impl ResidentConsultationTerminalReleaseFailure {
    fn from_resolution(resolution: &ResidentConsultationTaskOwnerResolution) -> Self {
        match resolution {
            ResidentConsultationTaskOwnerResolution::OwnerMismatch(_) => Self::OwnerMismatch,
            ResidentConsultationTaskOwnerResolution::BatchNotFound(_) => Self::BatchNotFound,
            ResidentConsultationTaskOwnerResolution::Continuation(
                ResidentConsultationContinuationResolution::NotPending(_),
            ) => Self::NotPending,
            ResidentConsultationTaskOwnerResolution::Continuation(
                ResidentConsultationContinuationResolution::KindMismatch { .. },
            ) => Self::KindMismatch,
            ResidentConsultationTaskOwnerResolution::Continuation(
                ResidentConsultationContinuationResolution::AggregateReleaseRejected {
                    reason, ..
                },
            ) => Self::Aggregate(*reason),
            ResidentConsultationTaskOwnerResolution::Continuation(
                ResidentConsultationContinuationResolution::AggregateReservationMissing(_),
            ) => Self::AggregateMissing,
            ResidentConsultationTaskOwnerResolution::Continuation(
                ResidentConsultationContinuationResolution::Resolved,
            ) => unreachable!("a resolved continuation has no release failure"),
        }
    }
}

/// Exact routed poll retained after a post-custody release rejection.
#[must_use = "retry the exact aggregate release without repeating durable custody"]
pub(in crate::resident) struct ResidentConsultationTerminalReleaseRejection<Work, Output> {
    pub(in crate::resident) poll: Box<ResidentConsultationTaskOwnerPoll<Work, Output>>,
    pub(in crate::resident) reason: ResidentConsultationTerminalReleaseFailure,
}

impl<Work, Output> std::fmt::Debug for ResidentConsultationTerminalReleaseRejection<Work, Output> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentConsultationTerminalReleaseRejection")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

/// One nonblocking completion observation for consultation child work.
enum ResidentConsultationReservedTaskPoll<Work, Output> {
    Handled {
        outcome: Box<ResidentConsultationTaskPoll<Work, Output>>,
        settlement: ResidentTaskReservationSettlement,
    },
    Stale(ResidentOperationTaskCompletion<Work, Output>),
    ReservationRejected {
        completion: ResidentOperationTaskCompletion<Work, Output>,
        reason: ResidentTaskReservationTransitionFailure,
    },
}

/// One nonblocking completion observation for consultation child work.
pub(in crate::resident) enum ResidentConsultationTaskPoll<Work, Output> {
    /// Exact attributable work returned with untrusted mechanical output.
    Completed {
        candidate: ConsultationChildDispatchCandidate,
        dispatch: ResidentOperationTaskKey,
        work: Work,
        output: Output,
        cancellation: ResidentOperationTaskCancellationState,
    },
    /// Accepted work disappeared without a trustworthy output.
    Lost(ResidentConsultationLostTask),
    /// A completion has no accepted basis in this resident lifetime.
    Stale(ResidentOperationTaskCompletion<Work, Output>),
}

impl<Work, Output> std::fmt::Debug for ResidentConsultationTaskPoll<Work, Output> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Completed { .. } => "Completed",
            Self::Lost(_) => "Lost",
            Self::Stale(_) => "Stale",
        };
        formatter
            .debug_struct("ResidentConsultationTaskPoll")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// One nonblocking child completion offered to the current collection owner.
pub(in crate::resident) enum ResidentConsultationTaskJoinPoll<
    Work,
    Output,
    JoinOutcome,
    LookupError,
> {
    /// Current parent recovery succeeded and the caller-owned join ran once.
    Joined {
        candidate: ConsultationChildDispatchCandidate,
        dispatch: ResidentOperationTaskKey,
        work: Work,
        output: Output,
        joined: JoinOutcome,
        cancellation: ResidentOperationTaskCancellationState,
    },
    /// Mechanical output remains attributable, but current parent recovery failed.
    JoinRecoveryFailed {
        candidate: ConsultationChildDispatchCandidate,
        dispatch: ResidentOperationTaskKey,
        work: Work,
        output: Output,
        cancellation: ResidentOperationTaskCancellationState,
        reason: LookupError,
    },
    /// Accepted work disappeared without trustworthy output.
    Lost(ResidentConsultationLostTask),
    /// The executor supplied completion for work this resident does not own.
    Stale(ResidentOperationTaskCompletion<Work, Output>),
}

impl<Work, Output, JoinOutcome, LookupError> std::fmt::Debug
    for ResidentConsultationTaskJoinPoll<Work, Output, JoinOutcome, LookupError>
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Joined { .. } => "Joined",
            Self::JoinRecoveryFailed { .. } => "JoinRecoveryFailed",
            Self::Lost(_) => "Lost",
            Self::Stale(_) => "Stale",
        };
        formatter
            .debug_struct("ResidentConsultationTaskJoinPoll")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Executor-lost work with the exact roster and durable dispatch coordinates.
pub(in crate::resident) struct ResidentConsultationLostTask {
    basis: ConsultationTaskBasis,
    key: ResidentOperationTaskKey,
    cancellation: ResidentOperationTaskCancellationState,
}

impl ResidentConsultationLostTask {
    /// Borrow the exact roster candidate whose work was lost.
    #[must_use]
    pub(in crate::resident) const fn candidate(&self) -> &ConsultationChildDispatchCandidate {
        &self.basis.candidate
    }

    /// Borrow the exact durable dispatch selected for recovery.
    #[must_use]
    pub(in crate::resident) const fn dispatch(&self) -> &ResidentOperationTaskKey {
        &self.key
    }

    /// Return whether this process recorded a cancellation notification first.
    #[must_use]
    pub(in crate::resident) const fn cancellation_state(
        &self,
    ) -> ResidentOperationTaskCancellationState {
        self.cancellation
    }
}

/// Why protected task recovery could not classify an existing child.
pub(in crate::resident) enum ResidentConsultationTaskRecoveryFailure<LookupError> {
    /// Current parent collection recovery failed.
    ParentLookup(LookupError),
    /// Protected child recovery failed.
    ChildLookup(LookupError),
    /// Release-time wall-clock observation failed.
    Clock(pale_ports::WallClockSourceError),
    /// The parent batch or child phase no longer admits this classification.
    Revalidation(crate::resident::ConsultationCollectionError),
    /// Lost-task recovery did not select its exact retained dispatch member.
    DispatchAssociation,
    /// An accepted lost task appeared to return before its durable dispatch.
    DispatchRolledBack,
}

impl<LookupError> std::fmt::Debug for ResidentConsultationTaskRecoveryFailure<LookupError> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::ParentLookup(_) => "ParentLookup",
            Self::ChildLookup(_) => "ChildLookup",
            Self::Clock(reason) => {
                let _redacted = reason;
                "Clock"
            }
            Self::Revalidation(reason) => {
                let _redacted = reason;
                "Revalidation"
            }
            Self::DispatchAssociation => "DispatchAssociation",
            Self::DispatchRolledBack => "DispatchRolledBack",
        };
        formatter
            .debug_struct("ResidentConsultationTaskRecoveryFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/resident/consultation/task_service.rs"]
pub(in crate::resident) mod tests;
