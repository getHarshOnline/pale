//! One bounded, nonblocking resident consultation turn.
//!
//! A turn recovers one collection, reconciles at most one local batch, and polls
//! the shared executor at most once. Mechanical progress ends at an explicit
//! policy frontier. It never loops, waits, selects a participant, invents quorum
//! or synthesis rules, or treats a stop as parent-operation acceptance.

use pale_core::records::{ImmutableRecordReferenceV1, UnixTimeV1};

use super::super::{
    ConsultationChildDispatchCandidate, ConsultationCollectionError, ConsultationCollectionStop,
    RecoveredConsultationCollection,
};
use super::task::{
    ConsultationChildRetentionOutcome, ResidentConsultationBatchCoordinate,
    ResidentConsultationBatchReconcileError, ResidentConsultationBatchReconciliation,
    ResidentConsultationBatchRetentionReason, ResidentConsultationCandidateLocalState,
    ResidentConsultationTaskAcceptanceFailure, ResidentConsultationTaskLimits,
    ResidentConsultationTaskOwner, ResidentConsultationTaskOwnerAcceptanceFailure,
    ResidentConsultationTaskOwnerPoll, ResidentConsultationTaskOwnerPollError,
};
use crate::resident::{
    task_owner::{ResidentOperationTaskExecutor, ResidentOperationTaskKey},
    task_reservation::ResidentAggregateTaskReservations,
    RecoveredOperationObservation,
};

/// One compiled-Package decision after a turn found no ready completion.
#[must_use = "apply the selected continuation or stop phase"]
pub(in crate::resident) enum ResidentConsultationSupervisionDecision<StopInput> {
    /// Keep collecting and optionally offer this exact roster candidate.
    Continue(Option<Box<ConsultationChildDispatchCandidate>>),
    /// Apply one already-selected stop transition.
    Stop(StopInput),
}

/// Exact collection and local batch state awaiting an admitted policy decision.
#[must_use = "apply one admitted decision or preserve this exact frontier"]
pub(in crate::resident) struct ResidentConsultationPolicyFrontier {
    current: RecoveredConsultationCollection,
    observed_at: UnixTimeV1,
    phase: ResidentConsultationPolicyFrontierPhase,
}

enum ResidentConsultationPolicyFrontierPhase {
    /// The collection deadline prevents another ordinary child offer.
    CollectionBoundary,
    /// One finite local batch was reconciled for this recovered collection.
    Reconciled(Box<ResidentConsultationBatchReconciliation>),
}

impl ResidentConsultationPolicyFrontier {
    fn collection_boundary(
        current: RecoveredConsultationCollection,
        observed_at: UnixTimeV1,
    ) -> Self {
        Self {
            current,
            observed_at,
            phase: ResidentConsultationPolicyFrontierPhase::CollectionBoundary,
        }
    }

    fn reconciled(
        current: RecoveredConsultationCollection,
        observed_at: UnixTimeV1,
        reconciliation: ResidentConsultationBatchReconciliation,
    ) -> Self {
        Self {
            current,
            observed_at,
            phase: ResidentConsultationPolicyFrontierPhase::Reconciled(Box::new(reconciliation)),
        }
    }

    /// Borrow the recovered collection that requires policy.
    pub(in crate::resident) fn current(&self) -> &RecoveredConsultationCollection {
        &self.current
    }

    /// Return the exact time used to construct this frontier.
    pub(in crate::resident) fn observed_at(&self) -> UnixTimeV1 {
        self.observed_at
    }
}

/// Exact completion and recovered parent selected by one bounded turn.
#[must_use = "join this exact completion through the resident operation service"]
pub(in crate::resident) struct ResidentConsultationCompletionReady<Work, Output> {
    current: RecoveredConsultationCollection,
    poll: ResidentConsultationTaskOwnerPoll<Work, Output>,
}

impl<Work, Output> ResidentConsultationCompletionReady<Work, Output> {
    pub(in crate::resident) fn from_parts(
        current: RecoveredConsultationCollection,
        poll: ResidentConsultationTaskOwnerPoll<Work, Output>,
    ) -> Self {
        Self { current, poll }
    }

    /// Consume the phase into its exact recovered parent and routed completion.
    pub(in crate::resident) fn into_parts(
        self,
    ) -> (
        RecoveredConsultationCollection,
        ResidentConsultationTaskOwnerPoll<Work, Output>,
    ) {
        (self.current, self.poll)
    }

    /// Borrow the physical store that owns this parent and child completion.
    pub(in crate::resident) fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        self.current.store()
    }

    /// Borrow the exact parent handle retained by this completion.
    pub(in crate::resident) fn parent(&self) -> &ImmutableRecordReferenceV1 {
        self.current.collection().parent()
    }

    /// Borrow the exact child and durable dispatch that physical recovery must join.
    pub(in crate::resident) fn join_selection(
        &self,
    ) -> Option<(
        &ConsultationChildDispatchCandidate,
        &crate::resident::task_owner::ResidentOperationTaskKey,
    )> {
        self.poll.join_selection()
    }
}

/// Exact stop decision and recovered parent selected by one bounded turn.
#[must_use = "commit this exact stop through the resident operation service"]
pub(in crate::resident) struct ResidentConsultationStopSelected<StopInput> {
    current: RecoveredConsultationCollection,
    input: StopInput,
}

impl<StopInput> ResidentConsultationStopSelected<StopInput> {
    pub(in crate::resident) fn from_parts(
        current: RecoveredConsultationCollection,
        input: StopInput,
    ) -> Self {
        Self { current, input }
    }

    /// Consume the phase into its exact recovered parent and Package input.
    pub(in crate::resident) fn into_parts(self) -> (RecoveredConsultationCollection, StopInput) {
        (self.current, self.input)
    }

    /// Borrow the Package-selected stop without separating it from its parent.
    pub(in crate::resident) const fn input(&self) -> &StopInput {
        &self.input
    }

    /// Borrow the physical store that owns the selected collection.
    pub(in crate::resident) fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        self.current.store()
    }
}

/// Useful work selected during one bounded supervision turn.
#[must_use = "handle the selected completion or offered child"]
pub(in crate::resident) enum ResidentConsultationTurnProgress<Work, Output> {
    /// One ready result and its exact parent await terminal custody.
    CompletionReady(Box<ResidentConsultationCompletionReady<Work, Output>>),
    /// One fresh child reached the executor under its exact batch coordinate.
    Offered {
        /// Exact process-local batch that owns the offer.
        coordinate: Box<ResidentConsultationBatchCoordinate>,
        /// Durable dispatch key accepted by the executor owner.
        dispatch: ResidentOperationTaskKey,
    },
}

/// A valid turn that performed no durable transition and can be called again.
#[must_use = "retain the exact pending reason before running another turn"]
pub(in crate::resident) enum ResidentConsultationTurnPending {
    /// Policy chose to continue but selected no roster candidate.
    NoCandidate {
        /// Exact batch available for a later turn.
        coordinate: ResidentConsultationBatchCoordinate,
    },
    /// The shared or per-batch executor ceiling was full before child recovery.
    Capacity {
        /// Exact batch that remains unchanged.
        coordinate: ResidentConsultationBatchCoordinate,
        /// Exact caller-selected candidate that was not released.
        candidate: ConsultationChildDispatchCandidate,
    },
    /// A stale local predecessor remains busy and was not replaced.
    BatchBusy {
        /// Exact stale batch preserved by this turn.
        retained: ResidentConsultationBatchCoordinate,
        /// Process-local fact preventing retirement.
        reason: ResidentConsultationBatchRetentionReason,
        /// Candidate selected by policy, if any; it was not released.
        candidate: Option<ConsultationChildDispatchCandidate>,
    },
    /// A deadline or existing stop prevents another ordinary child offer.
    CollectionBoundary {
        /// Candidate selected by policy, if any; it was not released.
        candidate: Option<ConsultationChildDispatchCandidate>,
    },
}

/// A collection stop observed or selected by this turn.
#[must_use = "preserve the established stop or commit the selected stop"]
pub(in crate::resident) enum ResidentConsultationTurnStopped<StopInput> {
    /// The recovered collection was already stopped; policy was not called.
    Established(ConsultationCollectionStop),
    /// Package selected one stop; physical custody has not started.
    Selected(Box<ResidentConsultationStopSelected<StopInput>>),
}

/// Evidence or custody that requires an explicit later recovery turn.
#[must_use = "preserve the exact batch, candidate, and failed offer"]
pub(in crate::resident) struct ResidentConsultationOfferRecovery<
    Work,
    ChildRecoveryError,
    RetentionFailure,
> {
    coordinate: ResidentConsultationBatchCoordinate,
    candidate: ConsultationChildDispatchCandidate,
    failure: Box<
        ResidentConsultationTaskOwnerAcceptanceFailure<Work, ChildRecoveryError, RetentionFailure>,
    >,
}

impl<Work, ChildRecoveryError, RetentionFailure>
    ResidentConsultationOfferRecovery<Work, ChildRecoveryError, RetentionFailure>
{
    fn new(
        coordinate: ResidentConsultationBatchCoordinate,
        candidate: ConsultationChildDispatchCandidate,
        failure: Box<
            ResidentConsultationTaskOwnerAcceptanceFailure<
                Work,
                ChildRecoveryError,
                RetentionFailure,
            >,
        >,
    ) -> Self {
        Self {
            coordinate,
            candidate,
            failure,
        }
    }

    /// Consume the carrier into its exact batch, candidate, and closed failure.
    pub(in crate::resident) fn into_parts(
        self,
    ) -> (
        ResidentConsultationBatchCoordinate,
        ConsultationChildDispatchCandidate,
        Box<
            ResidentConsultationTaskOwnerAcceptanceFailure<
                Work,
                ChildRecoveryError,
                RetentionFailure,
            >,
        >,
    ) {
        (self.coordinate, self.candidate, self.failure)
    }
}

/// Evidence or custody that requires explicit recovery outside this turn.
#[must_use = "preserve the exact recovery evidence or continuation"]
pub(in crate::resident) enum ResidentConsultationTurnNeedsRecovery<
    Work,
    Output,
    RecoveryError,
    ChildRecoveryError,
    RetentionFailure,
> {
    /// Exact collection recovery failed before local state changed.
    Collection(RecoveryError),
    /// Recovered collection time or roster planning did not admit.
    Planning(ConsultationCollectionError),
    /// Local batch reconciliation could not establish one exact owner.
    Batch(ResidentConsultationBatchReconcileError),
    /// The one executor poll returned stale, ambiguous, or unreservable work.
    Poll(ResidentConsultationTaskOwnerPollError<Work, Output>),
    /// The shared executor returned another Consult parent's exact completion.
    ///
    /// The intact routed poll still owns its settlement continuation. Recover
    /// that exact parent separately, bind the poll to it, and terminal-join it;
    /// do not pass the poll through another generic supervision turn.
    ForeignCompletion(Box<ResidentConsultationTaskOwnerPoll<Work, Output>>),
    /// Fresh-child release, custody, or executor acceptance needs recovery.
    Offer(Box<ResidentConsultationOfferRecovery<Work, ChildRecoveryError, RetentionFailure>>),
    /// The selected candidate already crossed its local first-offer phase.
    CandidateState {
        /// Exact process-local batch and recovered collection frontier.
        coordinate: Box<ResidentConsultationBatchCoordinate>,
        /// Exact selected roster candidate.
        candidate: Box<ConsultationChildDispatchCandidate>,
        /// Process-local state that prevents a second offer.
        state: ResidentConsultationCandidateLocalState,
    },
}

/// Mechanical failure or foreign evidence found before policy selection.
#[must_use = "preserve the exact recovery evidence before selecting policy"]
pub(in crate::resident) enum ResidentConsultationPrePolicyNeedsRecovery<Work, Output, RecoveryError>
{
    /// Exact collection recovery failed before local state changed.
    Collection(RecoveryError),
    /// Recovered collection time or roster planning did not admit.
    Planning(ConsultationCollectionError),
    /// Local batch reconciliation could not establish one exact owner.
    Batch(ResidentConsultationBatchReconcileError),
    /// The one executor poll returned stale, ambiguous, or unreservable work.
    Poll(ResidentConsultationTaskOwnerPollError<Work, Output>),
    /// The executor returned another Consult parent's exact completion.
    ForeignCompletion(Box<ResidentConsultationTaskOwnerPoll<Work, Output>>),
}

/// Closed mechanical result before any Consult policy is selected.
#[must_use = "handle progress, established state, recovery, or the policy frontier"]
pub(in crate::resident) enum ResidentConsultationPrePolicyTurn<Work, Output, RecoveryError> {
    /// One ready result and its exact parent await terminal custody.
    CompletionReady(Box<ResidentConsultationCompletionReady<Work, Output>>),
    /// The recovered collection already contains a durable stop.
    Stopped(ConsultationCollectionStop),
    /// Mechanical progress cannot continue without explicit recovery.
    NeedsRecovery(Box<ResidentConsultationPrePolicyNeedsRecovery<Work, Output, RecoveryError>>),
    /// No evidence selected a continuation or stop decision.
    PolicyRequired(Box<ResidentConsultationPolicyFrontier>),
}

/// Closed result of one bounded consultation supervision turn.
#[must_use = "handle the closed turn disposition before starting another turn"]
pub(in crate::resident) enum ResidentConsultationTurnDisposition<
    Work,
    Output,
    RecoveryError,
    ChildRecoveryError,
    RetentionFailure,
    StopInput,
> {
    /// The turn handled one completion or offered one child.
    Progress(ResidentConsultationTurnProgress<Work, Output>),
    /// The turn stayed valid but could not make a durable transition.
    Pending(Box<ResidentConsultationTurnPending>),
    /// Collection was already stopped or one stop transition ran.
    Stopped(ResidentConsultationTurnStopped<StopInput>),
    /// Exact evidence or phase state requires an explicit later recovery turn.
    NeedsRecovery(
        Box<
            ResidentConsultationTurnNeedsRecovery<
                Work,
                Output,
                RecoveryError,
                ChildRecoveryError,
                RetentionFailure,
            >,
        >,
    ),
}

/// Run one finite resident consultation turn.
///
/// `recover` runs once. After successful recovery and batch reconciliation,
/// `executor.try_completion` runs at most once. A rejected recovery or reconciliation
/// performs no poll. `decide` runs only when no completion was ready and the
/// collection was not already stopped. A ready completion or selected stop
/// leaves as an owned phase. This function performs no terminal custody. Child
/// release remains a single attempt.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(in crate::resident) fn supervise_consultation_turn<
    Work,
    Output,
    RecoveryError,
    Recover,
    ChildRecoveryError,
    RecoverChild,
    RetentionFailure,
    Retain,
    Executor,
    StopInput,
    Decide,
>(
    tasks: &mut ResidentConsultationTaskOwner,
    reservations: &mut ResidentAggregateTaskReservations,
    task_limits: ResidentConsultationTaskLimits,
    observed_at: UnixTimeV1,
    recover: Recover,
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
    Recover: FnOnce() -> Result<RecoveredConsultationCollection, RecoveryError>,
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
    let current = match recover() {
        Ok(current) => current,
        Err(reason) => {
            return ResidentConsultationTurnDisposition::NeedsRecovery(Box::new(
                ResidentConsultationTurnNeedsRecovery::Collection(reason),
            ));
        }
    };

    supervise_recovered_consultation_turn(
        tasks,
        reservations,
        task_limits,
        observed_at,
        current,
        executor,
        decide,
        recover_child,
        retain,
    )
}

/// Recover one collection and advance only its bounded mechanical work.
///
/// This route does not accept a policy callback. If the recovered evidence does
/// not establish a completion or stop, it returns the exact frontier as
/// [`ResidentConsultationPrePolicyTurn::PolicyRequired`].
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(in crate::resident) fn supervise_consultation_pre_policy_turn<
    Work,
    Output,
    RecoveryError,
    Recover,
    Executor,
>(
    tasks: &mut ResidentConsultationTaskOwner,
    reservations: &mut ResidentAggregateTaskReservations,
    task_limits: ResidentConsultationTaskLimits,
    observed_at: UnixTimeV1,
    recover: Recover,
    executor: &mut Executor,
) -> ResidentConsultationPrePolicyTurn<Work, Output, RecoveryError>
where
    Recover: FnOnce() -> Result<RecoveredConsultationCollection, RecoveryError>,
    Executor: ResidentOperationTaskExecutor<Work, Output>,
{
    let current = match recover() {
        Ok(current) => current,
        Err(reason) => {
            return ResidentConsultationPrePolicyTurn::NeedsRecovery(Box::new(
                ResidentConsultationPrePolicyNeedsRecovery::Collection(reason),
            ));
        }
    };

    supervise_recovered_consultation_pre_policy_turn(
        tasks,
        reservations,
        task_limits,
        observed_at,
        current,
        executor,
    )
}

/// Advance one recovered collection to an explicit policy frontier.
///
/// The function reconciles at most one finite batch and polls the shared
/// executor at most once. It does not select a child, infer continuation, or
/// select a stop.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(in crate::resident) fn supervise_recovered_consultation_pre_policy_turn<
    Work,
    Output,
    RecoveryError,
    Executor,
>(
    tasks: &mut ResidentConsultationTaskOwner,
    reservations: &mut ResidentAggregateTaskReservations,
    task_limits: ResidentConsultationTaskLimits,
    observed_at: UnixTimeV1,
    current: RecoveredConsultationCollection,
    executor: &mut Executor,
) -> ResidentConsultationPrePolicyTurn<Work, Output, RecoveryError>
where
    Executor: ResidentOperationTaskExecutor<Work, Output>,
{
    let already_stopped = current.collection().stopped().cloned();
    let boundary_reached = observed_at >= current.collection().deadline;
    let reconciliation = if already_stopped.is_none() && !boundary_reached {
        let planned = match current.plan_child_dispatch(observed_at) {
            Ok(planned) => planned,
            Err(reason) => {
                return ResidentConsultationPrePolicyTurn::NeedsRecovery(Box::new(
                    ResidentConsultationPrePolicyNeedsRecovery::Planning(reason),
                ));
            }
        };
        match tasks.reconcile_batch(planned, task_limits, reservations) {
            Ok(reconciliation) => Some(reconciliation),
            Err(reason) => {
                return ResidentConsultationPrePolicyTurn::NeedsRecovery(Box::new(
                    ResidentConsultationPrePolicyNeedsRecovery::Batch(reason),
                ));
            }
        }
    } else {
        None
    };

    let expected_batch = reconciliation
        .as_ref()
        .map(ResidentConsultationBatchReconciliation::retained)
        .or_else(|| tasks.retained_batch_coordinate(current.store(), current.collection().parent()))
        .cloned();

    if let Some(polled) = tasks.try_completion_reserved(reservations, executor) {
        return match polled {
            Ok(poll) => {
                if expected_batch.as_ref() != Some(&poll.coordinate) {
                    ResidentConsultationPrePolicyTurn::NeedsRecovery(Box::new(
                        ResidentConsultationPrePolicyNeedsRecovery::ForeignCompletion(Box::new(
                            poll,
                        )),
                    ))
                } else {
                    ResidentConsultationPrePolicyTurn::CompletionReady(Box::new(
                        ResidentConsultationCompletionReady::from_parts(current, poll),
                    ))
                }
            }
            Err(reason) => ResidentConsultationPrePolicyTurn::NeedsRecovery(Box::new(
                ResidentConsultationPrePolicyNeedsRecovery::Poll(reason),
            )),
        };
    }

    if let Some(established) = already_stopped {
        return ResidentConsultationPrePolicyTurn::Stopped(established);
    }

    let frontier = if boundary_reached {
        ResidentConsultationPolicyFrontier::collection_boundary(current, observed_at)
    } else {
        ResidentConsultationPolicyFrontier::reconciled(
            current,
            observed_at,
            reconciliation.expect("an open collection reconciles one finite batch"),
        )
    };
    ResidentConsultationPrePolicyTurn::PolicyRequired(Box::new(frontier))
}

/// Run one finite turn from an already recovered consultation collection.
///
/// The caller can finish recovery custody before lending the process task
/// owner. This function performs no collection recovery, port access, terminal
/// custody, loop, or wait.
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
    tasks: &mut ResidentConsultationTaskOwner,
    reservations: &mut ResidentAggregateTaskReservations,
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
    let mechanical = supervise_recovered_consultation_pre_policy_turn(
        tasks,
        reservations,
        task_limits,
        observed_at,
        current,
        executor,
    );
    let frontier = match mechanical {
        ResidentConsultationPrePolicyTurn::CompletionReady(ready) => {
            return ResidentConsultationTurnDisposition::Progress(
                ResidentConsultationTurnProgress::CompletionReady(ready),
            );
        }
        ResidentConsultationPrePolicyTurn::Stopped(established) => {
            return ResidentConsultationTurnDisposition::Stopped(
                ResidentConsultationTurnStopped::Established(established),
            );
        }
        ResidentConsultationPrePolicyTurn::NeedsRecovery(reason) => {
            let reason = match *reason {
                ResidentConsultationPrePolicyNeedsRecovery::Collection(reason) => {
                    ResidentConsultationTurnNeedsRecovery::Collection(reason)
                }
                ResidentConsultationPrePolicyNeedsRecovery::Planning(reason) => {
                    ResidentConsultationTurnNeedsRecovery::Planning(reason)
                }
                ResidentConsultationPrePolicyNeedsRecovery::Batch(reason) => {
                    ResidentConsultationTurnNeedsRecovery::Batch(reason)
                }
                ResidentConsultationPrePolicyNeedsRecovery::Poll(reason) => {
                    ResidentConsultationTurnNeedsRecovery::Poll(reason)
                }
                ResidentConsultationPrePolicyNeedsRecovery::ForeignCompletion(poll) => {
                    ResidentConsultationTurnNeedsRecovery::ForeignCompletion(poll)
                }
            };
            return ResidentConsultationTurnDisposition::NeedsRecovery(Box::new(reason));
        }
        ResidentConsultationPrePolicyTurn::PolicyRequired(frontier) => frontier,
    };

    let decision = decide(frontier.current(), frontier.observed_at());
    apply_consultation_supervision_decision(
        tasks,
        reservations,
        frontier,
        decision,
        executor,
        recover_child,
        retain,
    )
}

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
    tasks: &mut ResidentConsultationTaskOwner,
    reservations: &mut ResidentAggregateTaskReservations,
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
    let ResidentConsultationPolicyFrontier {
        current,
        observed_at,
        phase,
    } = *frontier;
    match decision {
        ResidentConsultationSupervisionDecision::Stop(input) => {
            ResidentConsultationTurnDisposition::Stopped(ResidentConsultationTurnStopped::Selected(
                Box::new(ResidentConsultationStopSelected::from_parts(current, input)),
            ))
        }
        ResidentConsultationSupervisionDecision::Continue(candidate) => {
            let candidate = candidate.map(|candidate| *candidate);
            let ResidentConsultationPolicyFrontierPhase::Reconciled(reconciliation) = phase else {
                return ResidentConsultationTurnDisposition::Pending(Box::new(
                    ResidentConsultationTurnPending::CollectionBoundary { candidate },
                ));
            };
            let Some(coordinate) = reconciliation.usable().cloned() else {
                let ResidentConsultationBatchReconciliation::Busy { retained, reason } =
                    *reconciliation
                else {
                    unreachable!("only a busy reconciliation lacks a usable coordinate")
                };
                return ResidentConsultationTurnDisposition::Pending(Box::new(
                    ResidentConsultationTurnPending::BatchBusy {
                        retained,
                        reason,
                        candidate,
                    },
                ));
            };
            let Some(candidate) = candidate else {
                return ResidentConsultationTurnDisposition::Pending(Box::new(
                    ResidentConsultationTurnPending::NoCandidate { coordinate },
                ));
            };
            match tasks.candidate_state(&coordinate, &candidate) {
                Some(ResidentConsultationCandidateLocalState::Unopened) => {}
                Some(state) => {
                    return ResidentConsultationTurnDisposition::NeedsRecovery(Box::new(
                        ResidentConsultationTurnNeedsRecovery::CandidateState {
                            coordinate: Box::new(coordinate),
                            candidate: Box::new(candidate),
                            state,
                        },
                    ));
                }
                None => {
                    return ResidentConsultationTurnDisposition::NeedsRecovery(Box::new(
                        ResidentConsultationTurnNeedsRecovery::Offer(Box::new(
                            ResidentConsultationOfferRecovery::new(
                                coordinate,
                                candidate.clone(),
                                Box::new(
                                    ResidentConsultationTaskOwnerAcceptanceFailure::Acceptance(
                                        Box::new(
                                            ResidentConsultationTaskAcceptanceFailure::NotPlanned(
                                                candidate,
                                            ),
                                        ),
                                    ),
                                ),
                            ),
                        )),
                    ));
                }
            }
            match tasks.retain_and_accept_recovered(
                &coordinate,
                &candidate,
                reservations,
                &current,
                observed_at,
                recover_child,
                retain,
                executor,
            ) {
                Ok(dispatch) => ResidentConsultationTurnDisposition::Progress(
                    ResidentConsultationTurnProgress::Offered {
                        coordinate: Box::new(coordinate),
                        dispatch,
                    },
                ),
                Err(failure) => {
                    if matches!(
                        failure.as_ref(),
                        ResidentConsultationTaskOwnerAcceptanceFailure::Acceptance(inner)
                            if matches!(
                                inner.as_ref(),
                                ResidentConsultationTaskAcceptanceFailure::AggregateSaturated(_)
                                    | ResidentConsultationTaskAcceptanceFailure::Saturated(_)
                            )
                    ) {
                        ResidentConsultationTurnDisposition::Pending(Box::new(
                            ResidentConsultationTurnPending::Capacity {
                                coordinate,
                                candidate,
                            },
                        ))
                    } else {
                        ResidentConsultationTurnDisposition::NeedsRecovery(Box::new(
                            ResidentConsultationTurnNeedsRecovery::Offer(Box::new(
                                ResidentConsultationOfferRecovery::new(
                                    coordinate, candidate, failure,
                                ),
                            )),
                        ))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/resident/consultation/supervision.rs"]
mod tests;
