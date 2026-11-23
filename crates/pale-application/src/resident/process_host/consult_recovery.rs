//! One-source cold-start recovery for committed root Consult operations.
//!
//! This driver owns no scheduler or consultation policy. One turn re-admits one
//! committed source, rebuilds its bounded retained roster, recovers its current
//! collection, and advances only the existing pre-policy mechanical owner.

use std::convert::Infallible;

use super::*;
use crate::resident::consultation::{
    ConsultationCollectionStop, ResidentConsultationPolicyFrontier,
    ResidentConsultationPrePolicyNeedsRecovery, ResidentConsultationPrePolicyTurn,
};
use crate::resident::local_operation_start::{
    discover_local_operation_start, LocalOperationStartDiscoveryError,
    LocalOperationStartDiscoveryTurn,
};
use crate::resident::process_host::consult_start::ResidentProcessConsultationStarted;
use crate::resident::process_host::operation_recovery_sweep::{
    admit_discovered_start, ResidentProcessOperationRecoverySourceError,
};
use crate::resident::process_host::restart::ResidentProcessRecoveredOperationStart;
use crate::resident::LocalOperationRecoveryContext;

/// Another semantic owner selected by the protected restart classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResidentProcessConsultRecoveryOtherOwner {
    /// An unparented non-Consult belongs to its root-operation recovery owner.
    Invoke,
    /// A child operation or pairwise Delegate belongs to its parent owner.
    Child,
}

/// Root Consult evidence that remains open without another mechanical turn.
#[must_use = "preserve the recovered Consult evidence for its next exact owner"]
pub(super) enum ResidentProcessConsultRecoveryPending {
    /// Recovery, uncertainty, cancellation, or later evidence must advance the parent.
    RecoveryOnly(Box<ResidentProcessRecoveredOperationStart>),
    /// A retained pre-dispatch cancellation request awaits its cancellation owner.
    LocalCancellation(Box<restart::ResidentProcessRecoveredLocalCancellation>),
}

/// An established Consult stop bound to its exact deployment and parent.
#[must_use = "preserve the stopped parent identity with its collection decision"]
pub(super) struct ResidentProcessConsultationEstablishedStop {
    selector: ResidentDeploymentAuthoritySelector,
    parent: ImmutableRecordReferenceV1,
    stop: ConsultationCollectionStop,
}

/// Exact cold-recovered start and mechanical frontier awaiting one decision.
///
/// The start retains the admitted deployment, parent, and ordered child
/// prefix. The frontier retains the current collection and its one reconciled
/// local batch. Neither value selects policy.
#[must_use = "apply one explicit decision or preserve this exact handoff"]
pub(super) struct ResidentProcessConsultationPolicyHandoff {
    started: ResidentProcessConsultationStarted,
    frontier: Box<ResidentConsultationPolicyFrontier>,
}

impl ResidentProcessConsultationPolicyHandoff {
    /// Borrow the recovered start evidence without granting continuation.
    pub(super) const fn started(&self) -> &ResidentProcessConsultationStarted {
        &self.started
    }

    /// Borrow the exact mechanical frontier awaiting policy.
    pub(super) const fn frontier(&self) -> &ResidentConsultationPolicyFrontier {
        &self.frontier
    }

    /// Borrow the immutable Package policy selected when the collection opened.
    pub(super) fn completion_policy(&self) -> &ImmutableRecordReferenceV1 {
        self.frontier.current().collection().completion_policy()
    }

    /// Consume both linear owners together.
    pub(super) fn into_parts(
        self,
    ) -> (
        ResidentProcessConsultationStarted,
        Box<ResidentConsultationPolicyFrontier>,
    ) {
        (self.started, self.frontier)
    }
}

/// Evaluator for the exact Package policy retained by one Consult collection.
///
/// The evaluator receives no caller-selected fallback. It must resolve and
/// apply the supplied immutable policy reference from the admitted compiled
/// generation, then return only the existing Continue or Stop vocabulary.
pub(super) trait ResidentConsultationPolicyEvaluator<StopInput> {
    /// Why the retained policy could not produce a decision.
    type Error;

    /// Evaluate one exact policy against its bounded mechanical frontier.
    fn evaluate(
        &mut self,
        completion_policy: &ImmutableRecordReferenceV1,
        frontier: &ResidentConsultationPolicyFrontier,
    ) -> Result<ResidentConsultationSupervisionDecision<StopInput>, Self::Error>;
}

/// Failed Package-policy evaluation with the untouched Consult handoff.
#[must_use = "preserve the exact handoff until its selected policy is available"]
pub(super) struct ResidentProcessConsultationPolicyEvaluationFailure<Error> {
    handoff: Box<ResidentProcessConsultationPolicyHandoff>,
    error: Error,
}

impl<Error> ResidentProcessConsultationPolicyEvaluationFailure<Error> {
    /// Recover the unchanged handoff and evaluator-owned failure.
    pub(super) fn into_parts(self) -> (Box<ResidentProcessConsultationPolicyHandoff>, Error) {
        (self.handoff, self.error)
    }
}

/// Exact recovered start retained beside one applied Consult disposition.
#[must_use = "preserve the recovered start with the applied disposition"]
pub(super) struct ResidentProcessConsultationPolicyContinuation<
    Work,
    Output,
    ChildRecoveryError,
    RetentionFailure,
    StopInput,
> {
    started: ResidentProcessConsultationStarted,
    disposition: ResidentConsultationTurnDisposition<
        Work,
        Output,
        Infallible,
        ChildRecoveryError,
        RetentionFailure,
        StopInput,
    >,
}

impl<Work, Output, ChildRecoveryError, RetentionFailure, StopInput>
    ResidentProcessConsultationPolicyContinuation<
        Work,
        Output,
        ChildRecoveryError,
        RetentionFailure,
        StopInput,
    >
{
    /// Rejoin an unchanged recovered start and applied disposition.
    pub(super) fn from_parts(
        started: ResidentProcessConsultationStarted,
        disposition: ResidentConsultationTurnDisposition<
            Work,
            Output,
            Infallible,
            ChildRecoveryError,
            RetentionFailure,
            StopInput,
        >,
    ) -> Self {
        Self {
            started,
            disposition,
        }
    }

    /// Consume the continuation into its start evidence and exact result.
    pub(super) fn into_parts(
        self,
    ) -> (
        ResidentProcessConsultationStarted,
        ResidentConsultationTurnDisposition<
            Work,
            Output,
            Infallible,
            ChildRecoveryError,
            RetentionFailure,
            StopInput,
        >,
    ) {
        (self.started, self.disposition)
    }
}

/// Why one policy continuation was refused before any callback or executor use.
pub(super) enum ResidentProcessConsultationPolicyRefusalReason {
    /// The requested deployment selector did not name one admitted application.
    Selection(ResidentProcessHostSelectionError),
    /// The handoff belongs to another admitted deployment selector.
    HandoffSelectorAssociation,
    /// The supplied access did not belong to the selected deployment store.
    AccessStoreAssociation,
    /// The recovered start belongs to another physical store.
    StartedStoreAssociation,
    /// The recovered start and policy frontier did not describe one collection.
    CollectionAssociation,
}

/// Intact policy input retained after a pre-application refusal.
#[must_use = "repair the association and retry this exact handoff and decision"]
pub(super) struct ResidentProcessConsultationPolicyRefusal<StopInput> {
    handoff: Box<ResidentProcessConsultationPolicyHandoff>,
    decision: ResidentConsultationSupervisionDecision<StopInput>,
    reason: ResidentProcessConsultationPolicyRefusalReason,
}

impl<StopInput> ResidentProcessConsultationPolicyRefusal<StopInput> {
    fn new(
        handoff: Box<ResidentProcessConsultationPolicyHandoff>,
        decision: ResidentConsultationSupervisionDecision<StopInput>,
        reason: ResidentProcessConsultationPolicyRefusalReason,
    ) -> Self {
        Self {
            handoff,
            decision,
            reason,
        }
    }

    /// Consume the refusal without losing the handoff or selected decision.
    pub(super) fn into_parts(
        self,
    ) -> (
        Box<ResidentProcessConsultationPolicyHandoff>,
        ResidentConsultationSupervisionDecision<StopInput>,
        ResidentProcessConsultationPolicyRefusalReason,
    ) {
        (self.handoff, self.decision, self.reason)
    }
}

impl ResidentProcessConsultationEstablishedStop {
    /// Consume the handoff without separating the stop from its owner.
    pub(super) fn into_parts(
        self,
    ) -> (
        ResidentDeploymentAuthoritySelector,
        ImmutableRecordReferenceV1,
        ConsultationCollectionStop,
    ) {
        (self.selector, self.parent, self.stop)
    }
}

/// One bounded root-Consult recovery result.
///
/// Advancing the discovery cursor transfers responsibility to the returned
/// owned phase. It does not accept the parent result, settle a terminal child,
/// or establish a consultation policy.
#[must_use = "preserve the returned Consult handoff or recovery disposition"]
pub(super) enum ResidentProcessConsultRecoveryTurn<Work, Output> {
    /// Every source in this deployment's frozen prefix has been classified.
    Exhausted,
    /// The source belongs to another recovery owner and was not run here.
    SkippedOtherOwner(ResidentProcessConsultRecoveryOtherOwner),
    /// The protected root Consult is already terminal.
    ReportedTerminal(Box<ResidentProcessRecoveredOperationStart>),
    /// One exact child completion was polled once and awaits the existing join owner.
    Progress(Box<ResidentProcessConsultationCompletionReady<Work, Output>>),
    /// Exact durable evidence remains open under another established owner.
    Pending(Box<ResidentProcessConsultRecoveryPending>),
    /// The recovered collection already contains a durable stop.
    StoppedEstablished(Box<ResidentProcessConsultationEstablishedStop>),
    /// Mechanical work ended at the exact policy frontier.
    PolicyRequired(Box<ResidentProcessConsultationPolicyHandoff>),
    /// Mechanical state requires explicit repair before this source can advance.
    NeedsRecovery(Box<ResidentConsultationPrePolicyNeedsRecovery<Work, Output, Infallible>>),
}

impl<Work, Output> ResidentProcessConsultRecoveryTurn<Work, Output> {
    /// Whether this turn transfers the discovered source to an exact owner.
    ///
    /// Mechanical recovery needs retain the source so a later sweep retries
    /// the same committed record after its prerequisite is repaired.
    fn transfers_source_responsibility(&self) -> bool {
        !matches!(self, Self::NeedsRecovery(_))
    }
}

/// Why one Consult recovery turn could not reach an owned disposition.
pub(super) enum ResidentProcessConsultRecoveryError<Work, Output> {
    /// A committed observation could not establish its exact independent anchor.
    PendingObservation(crate::resident::local_operation_start::PendingRootObservationError),
    /// Bounded physical discovery failed. The Consult cursor is unchanged.
    Discovery(LocalOperationStartDiscoveryError),
    /// The discovered request, handle, private basis, or lease differed.
    Source(ResidentProcessOperationRecoverySourceError),
    /// Protected restart admission or classification failed.
    Restart(ResidentProcessOperationRestartError),
    /// The retained parent intent or its contiguous child prefix did not recover.
    Start(consult_start::ResidentProcessConsultationResumeError),
    /// The recovered collection did not belong to the selected deployment or parent.
    Selection(ResidentProcessHostSelectionError),
    /// The current collection did not recover from the protected parent head.
    Collection(ResidentProcessConsultationCollectionRecoveryError),
    /// No current collection exists for this retained root Consult.
    CollectionAbsent,
    /// The bounded keyed owner cannot accept this exact parent.
    RetentionUnavailable,
    /// Current collection membership differs from the retained start intent.
    CollectionAssociation,
    /// The ready completion could not bind to its exact deployment.
    CompletionBinding(Box<ResidentProcessConsultationCompletionBindingFailure<Work, Output>>),
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Evaluate the exact completion policy retained by one recovered Consult.
    ///
    /// Failure preserves the handoff unchanged. This method invents no
    /// majority, deadline, or caller-selected default.
    #[allow(clippy::type_complexity)]
    pub(super) fn evaluate_recovered_consultation_policy<StopInput, Evaluator>(
        &mut self,
        handoff: Box<ResidentProcessConsultationPolicyHandoff>,
        evaluator: &mut Evaluator,
    ) -> Result<
        (
            Box<ResidentProcessConsultationPolicyHandoff>,
            ResidentConsultationSupervisionDecision<StopInput>,
        ),
        ResidentProcessConsultationPolicyEvaluationFailure<Evaluator::Error>,
    >
    where
        Evaluator: ResidentConsultationPolicyEvaluator<StopInput>,
    {
        match evaluator.evaluate(handoff.completion_policy(), handoff.frontier()) {
            Ok(decision) => Ok((handoff, decision)),
            Err(error) => {
                Err(ResidentProcessConsultationPolicyEvaluationFailure { handoff, error })
            }
        }
    }

    /// Recover at most one committed root Consult from one deployment.
    ///
    /// This component-level entrypoint applies no external scheduler-capacity
    /// fence. The resident program uses the retention-aware variant below.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub(super) fn recover_one_root_consult_start<
        Authority,
        Work,
        Output,
        Executor,
        AdmitCollection,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        restart: ResidentProcessOperationRestartInput<'_>,
        admission: OperationControlRecoveryAdmission<'_>,
        maximum_session_basis_bytes: usize,
        authority: &mut Authority,
        collection_limits: ConsultationCollectionLimits,
        task_limits: ResidentConsultationTaskLimits,
        observed_at: UnixTimeV1,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
        admit_collection: AdmitCollection,
        executor: &mut Executor,
    ) -> Result<
        ResidentProcessConsultRecoveryTurn<Work, Output>,
        ResidentProcessConsultRecoveryError<Work, Output>,
    >
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
        AdmitCollection: FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
    {
        self.recover_one_root_consult_start_with_retention(
            selector,
            access,
            restart,
            admission,
            maximum_session_basis_bytes,
            authority,
            collection_limits,
            task_limits,
            observed_at,
            resolve_store,
            admit_collection,
            None,
            |_| Ok(()),
            executor,
        )
    }

    /// Recover one root Consult after its exact parent passes retention admission.
    ///
    /// The turn reads one source, rebuilds only its retained child prefix, and
    /// runs at most one batch reconciliation and one executor poll. It never
    /// creates a child, selects policy, calls a provider, or performs a join.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub(super) fn recover_one_root_consult_start_with_retention<
        Authority,
        Work,
        Output,
        Executor,
        AdmitCollection,
        AdmitRetention,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        restart: ResidentProcessOperationRestartInput<'_>,
        mut admission: OperationControlRecoveryAdmission<'_>,
        maximum_session_basis_bytes: usize,
        authority: &mut Authority,
        collection_limits: ConsultationCollectionLimits,
        task_limits: ResidentConsultationTaskLimits,
        observed_at: UnixTimeV1,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
        admit_collection: AdmitCollection,
        admit_historical_collection: Option<
            &mut crate::resident::HistoricalConsultationAdmission<'_>,
        >,
        admit_retention: AdmitRetention,
        executor: &mut Executor,
    ) -> Result<
        ResidentProcessConsultRecoveryTurn<Work, Output>,
        ResidentProcessConsultRecoveryError<Work, Output>,
    >
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
        AdmitCollection: FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
        AdmitRetention: FnOnce(&ResidentProcessConsultationStarted) -> Result<(), ()>,
    {
        let index = self.deployment_authority_index(selector).map_err(|error| {
            ResidentProcessConsultRecoveryError::Restart(
                ResidentProcessOperationRestartError::Selection(error),
            )
        })?;
        if self.deployments[index].store() != access.store {
            return Err(ResidentProcessConsultRecoveryError::Restart(
                ResidentProcessHostSelectionError::AccessStoreAssociation.into(),
            ));
        }

        let attempt = self.consult_recovery_attempts[index].clone();
        let operation_context = restart.operation;
        let maximum_parent_depth = restart.maximum_parent_depth;
        let maximum_record_bytes = operation_context.maximum_record_bytes;
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let discovered =
            discover_local_operation_start(attempt, &mut *custody, maximum_session_basis_bytes)
                .map_err(ResidentProcessConsultRecoveryError::Discovery)?;
        let pending = match discovered {
            LocalOperationStartDiscoveryTurn::Exhausted(exhausted) => {
                self.consult_recovery_attempts[index] = exhausted;
                return Ok(ResidentProcessConsultRecoveryTurn::Exhausted);
            }
            LocalOperationStartDiscoveryTurn::Pending(pending) => pending,
        };

        let (request, evidence) = admit_discovered_start(
            store,
            pending.source(),
            &mut *custody,
            maximum_record_bytes,
            &mut *admission.admit_request,
        )
        .map_err(ResidentProcessConsultRecoveryError::Source)?;
        let request_preimage = request.to_preimage();
        let association = ResidentOperationControlAssociation {
            current_request_authority: &request_preimage.signing_authority_binding,
            historical_handle_authority: operation_context.handle_signing_authority,
            recovery_store: operation_context.store,
            checkpoint_authority: operation_context.checkpoint_authority,
        };
        let existing = ResidentProcessExistingOperationStart {
            selector: selector.clone(),
            evidence: Box::new(evidence),
        };
        super::restart::verify_restart_inputs(
            self.deployments[index].application.as_ref(),
            &existing,
            &association,
            store,
            &restart,
        )
        .map_err(ResidentProcessConsultRecoveryError::Restart)?;
        if let Some(limits) = restart.committed_observation_limits {
            crate::resident::local_operation_start::recover_pending_root_observation(
                LocalOperationRecoveryContext {
                    observation_limits: restart.current_head.observation,
                    checkpoint_limits: restart.current_head.checkpoints,
                    membership_limits: restart.current_head.membership,
                    ..operation_context
                },
                (&request, &existing.evidence, maximum_parent_depth),
                limits,
                custody,
                sessions,
                anchor,
                &mut admission,
                admit_historical_collection.map(|admit| (collection_limits, admit)),
            )
            .map_err(ResidentProcessConsultRecoveryError::PendingObservation)?;
        }
        let protected = self
            .protect_existing_operation_start(
                existing,
                association,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &restart,
                &mut admission,
            )
            .map_err(ResidentProcessConsultRecoveryError::Restart)?;

        let other_owner = if protected.parent_operation().is_some()
            || protected.operation_class() == &OperationClassV1::Delegate
        {
            Some(ResidentProcessConsultRecoveryOtherOwner::Child)
        } else if protected.operation_class() != &OperationClassV1::Consult {
            Some(ResidentProcessConsultRecoveryOtherOwner::Invoke)
        } else {
            None
        };
        if let Some(other_owner) = other_owner {
            self.consult_recovery_attempts[index] = pending.advance();
            return Ok(ResidentProcessConsultRecoveryTurn::SkippedOtherOwner(
                other_owner,
            ));
        }

        let disposition = self
            .classify_protected_root_operation(
                protected,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                restart,
                &mut admission,
            )
            .map_err(ResidentProcessConsultRecoveryError::Restart)?;

        let turn = match disposition {
            ResidentProcessOperationRestartDisposition::Terminal(start) => {
                ResidentProcessConsultRecoveryTurn::ReportedTerminal(start)
            }
            ResidentProcessOperationRestartDisposition::RecoveryOnly(start) => {
                ResidentProcessConsultRecoveryTurn::Pending(Box::new(
                    ResidentProcessConsultRecoveryPending::RecoveryOnly(start),
                ))
            }
            ResidentProcessOperationRestartDisposition::LocalCancellationRequested(recovered) => {
                ResidentProcessConsultRecoveryTurn::Pending(Box::new(
                    ResidentProcessConsultRecoveryPending::LocalCancellation(recovered),
                ))
            }
            ResidentProcessOperationRestartDisposition::Consult(start) => {
                let ResidentProcessRecoveredOperationStart {
                    selector: recovered_selector,
                    control,
                    generation_digest: _,
                    lineage: _,
                } = *start;
                if &recovered_selector != selector {
                    return Err(ResidentProcessConsultRecoveryError::Restart(
                        ResidentProcessHostSelectionError::ConsultationParentAuthority.into(),
                    ));
                }
                let (parent, protected) = control.into_parts();
                let started = self
                    .rebuild_consultation_start_from_protected_intent(
                        selector,
                        store,
                        parent.into_operation(),
                        operation_context,
                        maximum_parent_depth,
                        &mut *custody,
                        sessions,
                        &mut *anchor,
                        reborrow_recovery_admission(&mut admission),
                    )
                    .map_err(ResidentProcessConsultRecoveryError::Start)?;
                admit_retention(&started)
                    .map_err(|()| ResidentProcessConsultRecoveryError::RetentionUnavailable)?;
                let current = self
                    .recover_consultation_collection(
                        selector,
                        ResidentDeploymentOperationAccess {
                            store,
                            custody: &mut *custody,
                            sessions,
                            anchor: &mut *anchor,
                        },
                        authority,
                        started.parent(),
                        &protected,
                        collection_limits,
                        resolve_store,
                        admit_collection,
                    )
                    .map_err(ResidentProcessConsultRecoveryError::Collection)?
                    .ok_or(ResidentProcessConsultRecoveryError::CollectionAbsent)?;
                if !started.matches_collection(current.collection()) {
                    return Err(ResidentProcessConsultRecoveryError::CollectionAssociation);
                }
                self.advance_recovered_consultation_pre_policy(
                    started,
                    task_limits,
                    observed_at,
                    current,
                    executor,
                )?
            }
            ResidentProcessOperationRestartDisposition::NativeHookResponse(_)
            | ResidentProcessOperationRestartDisposition::OperationResolution(_)
            | ResidentProcessOperationRestartDisposition::ProviderPrepared(_)
            | ResidentProcessOperationRestartDisposition::ProviderResolved(_)
            | ResidentProcessOperationRestartDisposition::Provider(_)
            | ResidentProcessOperationRestartDisposition::NativePrepared(_)
            | ResidentProcessOperationRestartDisposition::Native(_)
            | ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(_)
            | ResidentProcessOperationRestartDisposition::A2aTask(_)
            | ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(_) => {
                return Err(ResidentProcessConsultRecoveryError::Restart(
                    ResidentProcessOperationRestartError::State,
                ));
            }
        };

        Ok(self.finish_consult_recovery_turn(index, pending.advance(), turn))
    }

    /// Close one discovered source or retain it for exact mechanical recovery.
    pub(super) fn finish_consult_recovery_turn<Work, Output>(
        &mut self,
        deployment_index: usize,
        successor: LocalOperationStartDiscoveryAttempt,
        turn: ResidentProcessConsultRecoveryTurn<Work, Output>,
    ) -> ResidentProcessConsultRecoveryTurn<Work, Output> {
        if turn.transfers_source_responsibility() {
            self.consult_recovery_attempts[deployment_index] = successor;
        }
        turn
    }

    /// Advance one already recovered Consult without selecting policy or joining.
    pub(super) fn advance_recovered_consultation_pre_policy<Work, Output, Executor>(
        &mut self,
        started: ResidentProcessConsultationStarted,
        task_limits: ResidentConsultationTaskLimits,
        observed_at: UnixTimeV1,
        current: RecoveredConsultationCollection,
        executor: &mut Executor,
    ) -> Result<
        ResidentProcessConsultRecoveryTurn<Work, Output>,
        ResidentProcessConsultRecoveryError<Work, Output>,
    >
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        let index = self
            .deployment_authority_index(started.selector())
            .map_err(ResidentProcessConsultRecoveryError::Selection)?;
        if self.deployments[index].store() != started.store() || started.store() != current.store()
        {
            return Err(ResidentProcessConsultRecoveryError::Selection(
                ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation,
            ));
        }
        if !started.matches_collection(current.collection()) {
            return Err(ResidentProcessConsultRecoveryError::CollectionAssociation);
        }
        let selector = started.selector().clone();
        let parent = current.collection().parent().clone();
        Ok(match self
            .operations
            .supervise_recovered_consultation_pre_policy_turn::<
                Work,
                Output,
                Infallible,
                Executor,
            >(task_limits, observed_at, current, executor)
        {
            ResidentConsultationPrePolicyTurn::CompletionReady(ready) => {
                let ready = self
                    .bind_consultation_completion(selector.clone(), *ready)
                    .map_err(ResidentProcessConsultRecoveryError::CompletionBinding)?;
                ResidentProcessConsultRecoveryTurn::Progress(Box::new(ready))
            }
            ResidentConsultationPrePolicyTurn::Stopped(stop) => {
                ResidentProcessConsultRecoveryTurn::StoppedEstablished(Box::new(
                    ResidentProcessConsultationEstablishedStop {
                        selector,
                        parent,
                        stop,
                    },
                ))
            }
            ResidentConsultationPrePolicyTurn::PolicyRequired(frontier) => {
                ResidentProcessConsultRecoveryTurn::PolicyRequired(Box::new(
                    ResidentProcessConsultationPolicyHandoff { started, frontier },
                ))
            }
            ResidentConsultationPrePolicyTurn::NeedsRecovery(reason) => {
                ResidentProcessConsultRecoveryTurn::NeedsRecovery(reason)
            }
        })
    }

    /// Apply one explicit decision to an exact cold-recovered Consult frontier.
    ///
    /// Selector, access, store, and collection associations are checked before
    /// any child recovery, retention, or executor callback. The existing
    /// application owner applies the decision once without another recovery,
    /// reconciliation, or poll.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub(super) fn continue_recovered_consultation_policy<
        Work,
        Output,
        ChildRecoveryError,
        RecoverChild,
        RetentionFailure,
        Retain,
        Executor,
        StopInput,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        handoff: Box<ResidentProcessConsultationPolicyHandoff>,
        decision: ResidentConsultationSupervisionDecision<StopInput>,
        executor: &mut Executor,
        recover_child: RecoverChild,
        retain: Retain,
    ) -> Result<
        ResidentProcessConsultationPolicyContinuation<
            Work,
            Output,
            ChildRecoveryError,
            RetentionFailure,
            StopInput,
        >,
        ResidentProcessConsultationPolicyRefusal<StopInput>,
    >
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
        RecoverChild: FnOnce(
            &ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, ChildRecoveryError>,
        Retain: FnOnce(
            &ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
            &mut dyn OperationCustodyPort,
            &mut dyn AntiRollbackAnchorPort,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
    {
        let index = match self.deployment_authority_index(selector) {
            Ok(index) => index,
            Err(reason) => {
                return Err(ResidentProcessConsultationPolicyRefusal::new(
                    handoff,
                    decision,
                    ResidentProcessConsultationPolicyRefusalReason::Selection(reason),
                ));
            }
        };
        if handoff.started().selector() != selector {
            return Err(ResidentProcessConsultationPolicyRefusal::new(
                handoff,
                decision,
                ResidentProcessConsultationPolicyRefusalReason::HandoffSelectorAssociation,
            ));
        }
        if self.deployments[index].store() != access.store {
            return Err(ResidentProcessConsultationPolicyRefusal::new(
                handoff,
                decision,
                ResidentProcessConsultationPolicyRefusalReason::AccessStoreAssociation,
            ));
        }
        if handoff.started().store() != self.deployments[index].store() {
            return Err(ResidentProcessConsultationPolicyRefusal::new(
                handoff,
                decision,
                ResidentProcessConsultationPolicyRefusalReason::StartedStoreAssociation,
            ));
        }
        if handoff.frontier().current().store() != handoff.started().store()
            || !handoff
                .started()
                .matches_collection(handoff.frontier().current().collection())
        {
            return Err(ResidentProcessConsultationPolicyRefusal::new(
                handoff,
                decision,
                ResidentProcessConsultationPolicyRefusalReason::CollectionAssociation,
            ));
        }

        let (started, frontier) = (*handoff).into_parts();
        let ResidentDeploymentOperationAccess {
            store: _,
            custody,
            sessions: _,
            anchor,
        } = access;
        let disposition = self
            .operations
            .apply_consultation_supervision_decision::<
                Work,
                Output,
                Infallible,
                ChildRecoveryError,
                RecoverChild,
                RetentionFailure,
                _,
                Executor,
                StopInput,
            >(
                frontier,
                decision,
                executor,
                recover_child,
                |candidate, child| retain(candidate, child, custody, anchor),
            );
        Ok(ResidentProcessConsultationPolicyContinuation {
            started,
            disposition,
        })
    }
}

fn reborrow_recovery_admission<'short>(
    admission: &'short mut OperationControlRecoveryAdmission<'_>,
) -> OperationControlRecoveryAdmission<'short> {
    OperationControlRecoveryAdmission {
        admit_request: &mut *admission.admit_request,
        admit_handle_authority: &mut *admission.admit_handle_authority,
        verify_handle_signature: &mut *admission.verify_handle_signature,
        admit_checkpoint: &mut *admission.admit_checkpoint,
        resolve_observation_signer: &mut *admission.resolve_observation_signer,
        admit_observation_state: &mut *admission.admit_observation_state,
        admit_rotation: &mut *admission.admit_rotation,
        verify_observation_signature: &mut *admission.verify_observation_signature,
        verify_checkpoint_signature: &mut *admission.verify_checkpoint_signature,
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/consult_recovery_support.rs"]
mod tests;
