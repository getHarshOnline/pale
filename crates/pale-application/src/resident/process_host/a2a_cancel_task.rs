//! Deployment routing for one-shot A2A `CancelTask` calls.
//!
//! The shared binding route is acquired before cancellation-specific aggregate
//! capacity or lower I/O. Carrier-started phases never return to this module as
//! replayable requests. A local stop ends only response reading.

use std::task::Context;

use pale_core::records::{
    ArtifactDigest, CanonicalSignatureV1, ControlCheckpointSigningAuthorityAdmissionContextV1,
    ControlStateCheckpointDigest, ControlStateCheckpointPreimageV1, ControlStateCheckpointV1,
    InvalidResidentRecordV1, OperationObservationDigest, OperationObservationPreimageV1,
    OperationObservationStateV1, OperationObservationV1, ResidentRecordRefV1, SchemaDigest,
    UnixTimeV1,
};
use pale_ports::{
    A2aCancelTaskEvidencePort, A2aExchangePort, AccumulatorPreparationLimits, WallClockSourcePort,
};

use super::{
    a2a_task_routes::{ResidentProcessA2aTaskRouteCustody, ResidentProcessA2aTaskRouteInsertError},
    checkpoint_authority_matches, ResidentDeploymentAuthoritySelector,
    ResidentDeploymentOperationAccess, ResidentProcessHost, ResidentProcessHostSelectionError,
};
use crate::resident::a2a_cancel_task_owner::{
    a2a_cancel_task_key, ResidentA2aCancelTaskAggregateAccepted,
    ResidentA2aCancelTaskAggregateTransitionRejection, ResidentA2aCancelTaskAwaitingCustody,
    ResidentA2aCancelTaskDurableCustody, ResidentA2aCancelTaskOwnerPoll,
    ResidentA2aCancelTaskPreflightFailure, ResidentA2aCancelTaskReleaseFailure,
    ResidentA2aCancelTaskStart, ResidentA2aCancelTaskStartContext,
    ResidentA2aCancelTaskStartTransitionFailure, ResidentRetainedA2aCancelTaskOutcome,
};
use crate::resident::a2a_task_custody::{
    A2aCancelTaskDispositionCommitInput, A2aCancelTaskReadbackCommitInput,
};
use crate::resident::operation_dispatch::{
    complete_prepared_checkpoint, verify_recovered_operation_head_custody, OperationAppendFailure,
};
use crate::resident::operation_service::{
    ResidentA2aCancelTaskDispositionRetentionFailure,
    ResidentA2aCancelTaskNonDispatchableReplayCustody,
    ResidentA2aCancelTaskNonDispatchableReplayReleaseFailure,
    ResidentA2aCancelTaskPreparationFailure, ResidentA2aCancelTaskReadbackRetentionFailure,
    ResidentA2aCancelTaskRequestCustodyFailure, ResidentPreparedA2aCancelTask,
    ResidentRetainedUnopenedA2aCancelTask, ResidentSelectedA2aCancelTaskDisposition,
    ResidentSelectedA2aCancelTaskReadback,
};
use crate::resident::process_host::restart::{
    ResidentProcessAttemptObservation, ResidentProcessOperationLineage,
    ResidentProcessRecoveredA2aCancelTaskRequest,
};
use crate::resident::task_reservation::ResidentA2aTaskWorkKey;
use crate::resident::{
    A2aCancelTaskAuthorizationContext, A2aCancelTaskError, A2aCancelTaskInput,
    CompletedOperationObservationCustody, OperationCheckpointInput,
};

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/a2a_cancel_task_support.rs"]
mod tests_support;

/// Failure before one prepared cancellation request is returned.
#[must_use = "preserve an unresolved route and aggregate-offer release"]
pub(super) enum ResidentProcessA2aCancelTaskPreparationFailure {
    Selection(ResidentProcessHostSelectionError),
    RouteConflict,
    RouteSaturated,
    Rejected(Box<ResidentA2aCancelTaskPreparationFailure>),
    Closed(Box<A2aCancelTaskError>),
    Release {
        key: Box<ResidentA2aTaskWorkKey>,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        lower: Box<ResidentA2aCancelTaskPreparationFailure>,
    },
}

/// Prepared request tied to one exact binding route and deployment.
#[must_use = "retain exact physical custody before accepting aggregate capacity"]
pub(super) struct ResidentProcessPreparedA2aCancelTask<'a> {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    lower: Box<ResidentPreparedA2aCancelTask<'a>>,
}

enum ResidentProcessA2aCancelTaskRequestCustodyPhase<'a> {
    Prepared(Box<ResidentPreparedA2aCancelTask<'a>>),
    Continuing(Box<ResidentA2aCancelTaskRequestCustodyFailure<'a>>),
}

/// Request custody stopped while both aggregate capacity and the route remain held.
#[must_use = "continue exact request custody without carrier access"]
pub(super) struct ResidentProcessA2aCancelTaskRequestCustodyFailure<'a> {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    phase: ResidentProcessA2aCancelTaskRequestCustodyPhase<'a>,
}

/// Retained unopened cancellation request tied to its active process route.
#[must_use = "accept aggregate capacity and apply fresh preflight before carrier start"]
pub(super) struct ResidentProcessRetainedUnopenedA2aCancelTask<'a> {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    lower: Box<ResidentRetainedUnopenedA2aCancelTask<'a>>,
}

impl ResidentProcessRetainedUnopenedA2aCancelTask<'_> {
    /// Borrow the checkpoint that proves the local cancellation request custody.
    #[must_use]
    pub(super) fn request_checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        self.lower.retained.checkpoint()
    }
}

/// Exact replay custody that can release capacity but can never call the carrier.
#[must_use = "release lower aggregate capacity before the process route"]
pub(super) struct ResidentProcessNonDispatchableReplayA2aCancelTask {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    lower: Box<ResidentA2aCancelTaskNonDispatchableReplayCustody>,
}

/// Non-dispatchable replay custody whose lower aggregate release needs a retry.
#[must_use = "retry lower release before the process route"]
pub(super) enum ResidentProcessNonDispatchableReplayA2aCancelTaskReleaseFailure {
    Selection(ResidentProcessNonDispatchableReplayA2aCancelTask),
    Lower {
        key: Box<ResidentA2aTaskWorkKey>,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        lower: Box<ResidentA2aCancelTaskNonDispatchableReplayReleaseFailure>,
    },
}

/// Unopened start transition retained with its active process route.
#[must_use = "retry only process-local aggregate acceptance or preserve the phase"]
pub(super) struct ResidentProcessA2aCancelTaskStartTransitionFailure<'a> {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    lower: Box<ResidentA2aCancelTaskStartTransitionFailure<'a>>,
}

/// Aggregate-accepted unopened request waiting for fresh lower preflight.
#[must_use = "apply fresh authority before the first carrier call"]
pub(super) struct ResidentProcessAggregateAcceptedA2aCancelTask<'a> {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    lower: Box<ResidentA2aCancelTaskAggregateAccepted<'a>>,
}

/// Fresh preflight refusal that retains an unopened request and active route.
#[must_use = "retry only the same unopened request with fresh authority"]
pub(super) struct ResidentProcessA2aCancelTaskPreflightFailure<'a> {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    lower: Box<ResidentA2aCancelTaskPreflightFailure<'a>>,
}

/// Carrier evidence checked out with the exact binding route.
#[must_use = "retain exact history or disposition before releasing capacity"]
pub(super) struct ResidentProcessA2aCancelTaskAwaitingCustody {
    route: ResidentProcessA2aTaskRouteCustody,
    lower: Box<ResidentA2aCancelTaskAwaitingCustody>,
}

/// Selected complete read-back that keeps its exact checked-out route.
#[must_use = "retain predecessor-linked remote history before route release"]
pub(super) struct ResidentProcessSelectedA2aCancelTaskReadback {
    route: ResidentProcessA2aTaskRouteCustody,
    lower: Box<ResidentSelectedA2aCancelTaskReadback>,
}

/// Read-back retention stopped while exact aggregate and route custody remain held.
#[must_use = "recover pre-physical evidence or continue exact physical custody"]
enum ResidentProcessA2aCancelTaskReadbackRetentionPhase {
    Selected(Box<ResidentSelectedA2aCancelTaskReadback>),
    Continuing(Box<ResidentA2aCancelTaskReadbackRetentionFailure>),
}

/// Exact checked-out route and read-back phase preserved across admission or custody failure.
#[must_use = "recover pre-physical input or continue exact read-back custody"]
pub(super) struct ResidentProcessA2aCancelTaskReadbackRetentionFailure {
    route: ResidentProcessA2aTaskRouteCustody,
    phase: ResidentProcessA2aCancelTaskReadbackRetentionPhase,
}

/// Selected incomplete outcome that keeps its exact checked-out route.
#[must_use = "retain local disposition before route release"]
pub(super) struct ResidentProcessSelectedA2aCancelTaskDisposition {
    route: ResidentProcessA2aTaskRouteCustody,
    lower: Box<ResidentSelectedA2aCancelTaskDisposition>,
}

enum ResidentProcessA2aCancelTaskDispositionRetentionPhase {
    Selected(Box<ResidentSelectedA2aCancelTaskDisposition>),
    Continuing(Box<ResidentA2aCancelTaskDispositionRetentionFailure>),
}

/// Disposition retention stopped while aggregate and route custody remain held.
#[must_use = "recover pre-physical evidence or continue exact physical custody"]
pub(super) struct ResidentProcessA2aCancelTaskDispositionRetentionFailure {
    route: ResidentProcessA2aTaskRouteCustody,
    phase: ResidentProcessA2aCancelTaskDispositionRetentionPhase,
}

/// Durable terminal custody that still owns aggregate capacity and its route.
#[must_use = "release aggregate capacity before the process route"]
pub(super) struct ResidentProcessRetainedA2aCancelTaskOutcome {
    route: ResidentProcessA2aTaskRouteCustody,
    lower: Box<ResidentRetainedA2aCancelTaskOutcome>,
}

/// Terminal release stopped at the lower aggregate or process-route boundary.
#[must_use = "retry the exact lower release before route release"]
pub(super) enum ResidentProcessA2aCancelTaskReleaseFailure {
    Selection(ResidentProcessRetainedA2aCancelTaskOutcome),
    Lower {
        route: ResidentProcessA2aTaskRouteCustody,
        lower: Box<ResidentA2aCancelTaskReleaseFailure>,
    },
    Route {
        route: ResidentProcessA2aTaskRouteCustody,
        custody: Box<ResidentA2aCancelTaskDurableCustody>,
    },
}

/// One bounded transition of a routed cancellation response read.
#[must_use = "retain or continue the exact routed cancellation phase"]
pub(super) enum ResidentProcessA2aCancelTaskPoll {
    PendingResponse,
    ResponseReceived,
    PendingEof,
    AwaitingCustody(ResidentProcessA2aCancelTaskAwaitingCustody),
    AggregateTransitionRejected {
        key: Box<ResidentA2aTaskWorkKey>,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        rejection: Box<ResidentA2aCancelTaskAggregateTransitionRejection>,
    },
    NotFound,
    Selection(ResidentProcessHostSelectionError),
}

/// Result of starting a retained unopened cancellation request.
#[must_use = "preserve every unopened or carrier-started phase"]
pub(super) enum ResidentProcessA2aCancelTaskStart<'a> {
    Active,
    PreflightRejected(ResidentProcessA2aCancelTaskPreflightFailure<'a>),
    AwaitingCustody(ResidentProcessA2aCancelTaskAwaitingCustody),
    AggregateTransitionRejected {
        key: Box<ResidentA2aTaskWorkKey>,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        rejection: Box<ResidentA2aCancelTaskAggregateTransitionRejection>,
    },
}

/// Completed cold-restart uncertainty custody for one retained `CancelTask` request.
pub(super) struct CompletedResidentProcessRecoveredA2aCancelTaskDisposition {
    pub(super) custody: CompletedOperationObservationCustody,
    pub(super) checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    pub(super) observation: Box<(ResidentRecordRefV1, OperationObservationV1)>,
}

/// Exact issued uncertainty append retained across database or anchor ambiguity.
#[must_use = "continue only this exact recovered CancelTask disposition custody"]
#[derive(Debug)]
pub(super) struct ResidentProcessRecoveredA2aCancelTaskCustody {
    selector: ResidentDeploymentAuthoritySelector,
    observation: Box<(ResidentRecordRefV1, OperationObservationV1)>,
    failure: OperationAppendFailure,
}

/// Why restart recovery could not retain one explicit uncertainty successor.
#[derive(Debug)]
pub(super) enum ResidentProcessRecoveredA2aCancelTaskFailure {
    /// Deployment, store, binding, selector, or current history differed.
    Selection(ResidentProcessHostSelectionError),
    /// Diagnostic, observation, reconciliation, signature, or checkpoint preparation failed.
    Preparation(A2aCancelTaskError),
    /// Database or independent-anchor outcome retains an exact continuation.
    Custody(Box<ResidentProcessRecoveredA2aCancelTaskCustody>),
}

/// Why exact post-issuance custody could not advance.
#[derive(Debug)]
pub(super) enum ResidentProcessRecoveredA2aCancelTaskCustodyReason {
    /// Deployment or physical store selection changed.
    Selection(ResidentProcessHostSelectionError),
    /// Database read-back or independent anchoring remains unresolved.
    Custody,
}

/// Unchanged post-issuance custody and the reason it remains incomplete.
#[derive(Debug)]
#[must_use = "retry only the returned exact recovered CancelTask custody"]
pub(super) struct ResidentProcessRecoveredA2aCancelTaskCustodyFailure {
    pub(super) pending: Box<ResidentProcessRecoveredA2aCancelTaskCustody>,
    pub(super) reason: ResidentProcessRecoveredA2aCancelTaskCustodyReason,
}

/// Wake inputs for one already-started response-read transition.
pub(super) struct ResidentProcessA2aCancelTaskPollContext<'a, 'cx> {
    cx: &'a mut Context<'cx>,
    protocol: &'a dyn A2aCancelTaskEvidencePort,
    clock: &'a dyn WallClockSourcePort,
}

impl<'a, 'cx> ResidentProcessA2aCancelTaskPollContext<'a, 'cx> {
    /// Keep one wake context and its evidence sources together.
    pub(super) fn new(
        cx: &'a mut Context<'cx>,
        protocol: &'a dyn A2aCancelTaskEvidencePort,
        clock: &'a dyn WallClockSourcePort,
    ) -> Self {
        Self {
            cx,
            protocol,
            clock,
        }
    }
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Retain explicit uncertainty for one request proved during cold restart.
    ///
    /// This path accepts no exchange port. It rechecks the selector, store,
    /// Task binding, request evidence, and current cancellation head before any
    /// signing or storage work. The only successor is `OutcomeUnknown`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn recover_restarted_a2a_cancel_task<
        Reconciliation,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &self,
        recovered: &ResidentProcessRecoveredA2aCancelTaskRequest,
        access: ResidentDeploymentOperationAccess<'_>,
        input: A2aCancelTaskDispositionCommitInput<'_>,
        maximum_record_bytes: usize,
        resolve_store: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        CompletedResidentProcessRecoveredA2aCancelTaskDisposition,
        ResidentProcessRecoveredA2aCancelTaskFailure,
    >
    where
        Reconciliation: FnOnce(
            &pale_core::records::ReconciliationFrontierReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
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
        let selector = recovered.attempt.start.selector.clone();
        let deployment = self
            .deployment_authority_index(&selector)
            .map_err(ResidentProcessRecoveredA2aCancelTaskFailure::Selection)?;
        let application = self.deployments[deployment].application.as_ref();
        if application.store() != access.store {
            return Err(ResidentProcessRecoveredA2aCancelTaskFailure::Selection(
                ResidentProcessHostSelectionError::AccessStoreAssociation,
            ));
        }
        if recovered.attempt.start.generation_digest
            != *recovered
                .attempt
                .start
                .control
                .operation()
                .operation()
                .handle()
                .1
                .compiled_generation_sha256()
            || application
                .generation(&recovered.attempt.start.generation_digest)
                .is_none()
        {
            return Err(ResidentProcessRecoveredA2aCancelTaskFailure::Selection(
                ResidentProcessHostSelectionError::HistoricalProviderAuthority,
            ));
        }
        validate_recovered_a2a_cancel_task(&selector, access.store, recovered, &input)
            .map_err(ResidentProcessRecoveredA2aCancelTaskFailure::Selection)?;

        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        verify_recovered_operation_head_custody(
            recovered.attempt.start.control.operation().operation(),
            recovered.attempt.start.control.observation(),
            custody,
            anchor,
            maximum_record_bytes,
        )
        .map_err(A2aCancelTaskError::Attempt)
        .map_err(ResidentProcessRecoveredA2aCancelTaskFailure::Preparation)?;

        let prepared = input
            .disposition
            .prepare_recovered_request_uncertainty(
                &recovered.request_evidence,
                &recovered.request_id,
                admit_reconciliation,
                admit_rotation,
                sign_observation,
            )
            .map_err(ResidentProcessRecoveredA2aCancelTaskFailure::Preparation)?;
        let prior = prepared.predecessor_checkpoint();
        let prior = (prior.0.clone(), prior.1.clone());
        let observation = (
            prepared.observation().0.clone(),
            prepared.observation().1.clone(),
        );
        let checkpoint = prepared
            .prepare_checkpoint(input.accumulator_limits, resolve_store, custody)
            .map_err(A2aCancelTaskError::Projection)
            .map_err(ResidentProcessRecoveredA2aCancelTaskFailure::Preparation)?;
        let completed = complete_prepared_checkpoint(
            checkpoint,
            (&prior.0, &prior.1),
            input.checkpoint,
            input.checkpoint_issued_at,
            input.checkpoint_schema,
            input.checkpoint_authority,
            custody,
            anchor,
            sign_checkpoint,
        );
        match completed {
            Ok((custody, checkpoint)) => {
                Ok(CompletedResidentProcessRecoveredA2aCancelTaskDisposition {
                    custody,
                    checkpoint: Box::new(checkpoint),
                    observation: Box::new(observation),
                })
            }
            Err(OperationAppendFailure::Operation(reason)) => {
                Err(ResidentProcessRecoveredA2aCancelTaskFailure::Preparation(
                    A2aCancelTaskError::Attempt(reason),
                ))
            }
            Err(OperationAppendFailure::Projection(reason)) => {
                Err(ResidentProcessRecoveredA2aCancelTaskFailure::Preparation(
                    A2aCancelTaskError::Projection(reason),
                ))
            }
            Err(
                failure @ (OperationAppendFailure::Commit { .. }
                | OperationAppendFailure::Anchor { .. }),
            ) => Err(ResidentProcessRecoveredA2aCancelTaskFailure::Custody(
                Box::new(ResidentProcessRecoveredA2aCancelTaskCustody {
                    selector,
                    observation: Box::new(observation),
                    failure,
                }),
            )),
        }
    }

    /// Continue exact database or anchor custody without rebuilding the append.
    pub(super) fn continue_restarted_a2a_cancel_task_custody(
        &self,
        pending: ResidentProcessRecoveredA2aCancelTaskCustody,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<
        CompletedResidentProcessRecoveredA2aCancelTaskDisposition,
        ResidentProcessRecoveredA2aCancelTaskCustodyFailure,
    > {
        let deployment = match self.deployment_authority_index(&pending.selector) {
            Ok(deployment) => deployment,
            Err(reason) => {
                return Err(ResidentProcessRecoveredA2aCancelTaskCustodyFailure {
                    pending: Box::new(pending),
                    reason: ResidentProcessRecoveredA2aCancelTaskCustodyReason::Selection(reason),
                });
            }
        };
        if self.deployments[deployment].store() != access.store {
            return Err(ResidentProcessRecoveredA2aCancelTaskCustodyFailure {
                pending: Box::new(pending),
                reason: ResidentProcessRecoveredA2aCancelTaskCustodyReason::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation,
                ),
            });
        }
        let ResidentProcessRecoveredA2aCancelTaskCustody {
            selector,
            observation,
            failure,
        } = pending;
        match failure.continue_custody(access.custody, access.anchor) {
            Ok((custody, checkpoint)) => {
                Ok(CompletedResidentProcessRecoveredA2aCancelTaskDisposition {
                    custody,
                    checkpoint: Box::new(checkpoint),
                    observation,
                })
            }
            Err(failure) => Err(ResidentProcessRecoveredA2aCancelTaskCustodyFailure {
                pending: Box::new(ResidentProcessRecoveredA2aCancelTaskCustody {
                    selector,
                    observation,
                    failure,
                }),
                reason: ResidentProcessRecoveredA2aCancelTaskCustodyReason::Custody,
            }),
        }
    }

    /// Acquire the shared binding route, then prepare a local cancellation request.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_a2a_cancel_task<
        'a,
        Authority,
        Authorize,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: A2aCancelTaskInput<'a>,
        port: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        checkpoint: OperationCheckpointInput,
        checkpoint_issued_at: UnixTimeV1,
        checkpoint_schema: &SchemaDigest,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        accumulator_limits: AccumulatorPreparationLimits,
        resolve_store: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessPreparedA2aCancelTask<'a>,
        ResidentProcessA2aCancelTaskPreparationFailure,
    >
    where
        Authorize:
            FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
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
        let key = a2a_cancel_task_key(input.history);
        let deployment = self
            .deployment_authority_index(selector)
            .map_err(ResidentProcessA2aCancelTaskPreparationFailure::Selection)?;
        if self.deployments[deployment].store() != access.store || key.store != *access.store {
            return Err(ResidentProcessA2aCancelTaskPreparationFailure::Selection(
                ResidentProcessHostSelectionError::AccessStoreAssociation,
            ));
        }
        if !selector.matches_operation(input.attempt.operation) {
            return Err(ResidentProcessA2aCancelTaskPreparationFailure::Selection(
                ResidentProcessHostSelectionError::HistoricalHandleAuthority,
            ));
        }
        match self.a2a_task_routes.insert(key.clone(), selector.clone()) {
            Ok(()) => {}
            Err(ResidentProcessA2aTaskRouteInsertError::Conflict) => {
                return Err(ResidentProcessA2aCancelTaskPreparationFailure::RouteConflict);
            }
            Err(ResidentProcessA2aTaskRouteInsertError::Saturated) => {
                return Err(ResidentProcessA2aCancelTaskPreparationFailure::RouteSaturated);
            }
        }
        let prepared = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.prepare_owned_a2a_cancel_task(
                        input,
                        port,
                        clock,
                        authorize,
                        admit_rotation,
                        sign_observation,
                        checkpoint,
                        checkpoint_issued_at,
                        checkpoint_schema,
                        checkpoint_authority,
                        accumulator_limits,
                        resolve_store,
                        sign_checkpoint,
                    )
                },
            )
            .expect("the deployment and store were fenced before CancelTask preparation");
        match prepared {
            Ok(lower) => Ok(ResidentProcessPreparedA2aCancelTask {
                key: Box::new(key),
                selector: Box::new(selector.clone()),
                lower: Box::new(lower),
            }),
            Err(failure @ ResidentA2aCancelTaskPreparationFailure::Release { .. }) => {
                Err(ResidentProcessA2aCancelTaskPreparationFailure::Release {
                    key: Box::new(key),
                    selector: Box::new(selector.clone()),
                    lower: Box::new(failure),
                })
            }
            Err(ResidentA2aCancelTaskPreparationFailure::Closed(reason)) => {
                let removed = self.a2a_task_routes.remove_active(&key, selector);
                debug_assert!(removed);
                Err(ResidentProcessA2aCancelTaskPreparationFailure::Closed(
                    reason,
                ))
            }
            Err(failure) => {
                let removed = self.a2a_task_routes.remove_active(&key, selector);
                debug_assert!(removed);
                Err(ResidentProcessA2aCancelTaskPreparationFailure::Rejected(
                    Box::new(failure),
                ))
            }
        }
    }

    /// Retry only aggregate-offer release after preparation already closed.
    pub(super) fn retry_a2a_cancel_task_preparation_release<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aCancelTaskPreparationFailure,
    ) -> Result<A2aCancelTaskError, ResidentProcessA2aCancelTaskPreparationFailure> {
        let ResidentProcessA2aCancelTaskPreparationFailure::Release {
            key,
            selector,
            lower,
        } = pending
        else {
            return Err(pending);
        };
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || lower.unresolved_key() != Some(key.as_ref())
        {
            return Err(ResidentProcessA2aCancelTaskPreparationFailure::Release {
                key,
                selector,
                lower,
            });
        }
        let retried = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retry_a2a_cancel_task_preparation_release(*lower)
                },
            )
            .expect("the active CancelTask route was fenced before release retry");
        match retried {
            Ok(reason) => {
                let removed = self.a2a_task_routes.remove_active(&key, &selector);
                debug_assert!(removed);
                Ok(reason)
            }
            Err(lower) => Err(ResidentProcessA2aCancelTaskPreparationFailure::Release {
                key,
                selector,
                lower: Box::new(lower),
            }),
        }
    }

    /// Commit and anchor the prepared request under its exact active route.
    pub(super) fn retain_prepared_a2a_cancel_task<'a, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        prepared: ResidentProcessPreparedA2aCancelTask<'a>,
    ) -> Result<
        ResidentProcessRetainedUnopenedA2aCancelTask<'a>,
        ResidentProcessA2aCancelTaskRequestCustodyFailure<'a>,
    > {
        let ResidentProcessPreparedA2aCancelTask {
            key,
            selector,
            lower,
        } = prepared;
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || lower.key() != key.as_ref()
        {
            return Err(ResidentProcessA2aCancelTaskRequestCustodyFailure {
                key,
                selector,
                phase: ResidentProcessA2aCancelTaskRequestCustodyPhase::Prepared(lower),
            });
        }
        let retained = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| service.retain_prepared_a2a_cancel_task(*lower),
            )
            .expect("the active CancelTask route was fenced before request custody");
        match retained {
            Ok(lower) => Ok(ResidentProcessRetainedUnopenedA2aCancelTask {
                key,
                selector,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aCancelTaskRequestCustodyFailure {
                key,
                selector,
                phase: ResidentProcessA2aCancelTaskRequestCustodyPhase::Continuing(Box::new(lower)),
            }),
        }
    }

    /// Continue only exact database or anchor request custody.
    pub(super) fn continue_a2a_cancel_task_request_custody<'a, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aCancelTaskRequestCustodyFailure<'a>,
    ) -> Result<
        ResidentProcessRetainedUnopenedA2aCancelTask<'a>,
        ResidentProcessA2aCancelTaskRequestCustodyFailure<'a>,
    > {
        let ResidentProcessA2aCancelTaskRequestCustodyFailure {
            key,
            selector,
            phase,
        } = pending;
        let phase_key = match &phase {
            ResidentProcessA2aCancelTaskRequestCustodyPhase::Prepared(lower) => lower.key(),
            ResidentProcessA2aCancelTaskRequestCustodyPhase::Continuing(lower) => lower.key(),
        };
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || phase_key != key.as_ref()
        {
            return Err(ResidentProcessA2aCancelTaskRequestCustodyFailure {
                key,
                selector,
                phase,
            });
        }
        let continued = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| match phase {
                    ResidentProcessA2aCancelTaskRequestCustodyPhase::Prepared(lower) => {
                        service.retain_prepared_a2a_cancel_task(*lower)
                    }
                    ResidentProcessA2aCancelTaskRequestCustodyPhase::Continuing(lower) => {
                        service.continue_a2a_cancel_task_request_custody(*lower)
                    }
                },
            )
            .expect("the active CancelTask route was fenced before custody continuation");
        match continued {
            Ok(lower) => Ok(ResidentProcessRetainedUnopenedA2aCancelTask {
                key,
                selector,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aCancelTaskRequestCustodyFailure {
                key,
                selector,
                phase: ResidentProcessA2aCancelTaskRequestCustodyPhase::Continuing(Box::new(lower)),
            }),
        }
    }

    /// Classify exact replay custody without granting carrier authority.
    pub(super) fn classify_non_dispatchable_replay_a2a_cancel_task<'a, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aCancelTaskRequestCustodyFailure<'a>,
    ) -> Result<
        ResidentProcessNonDispatchableReplayA2aCancelTask,
        ResidentProcessA2aCancelTaskRequestCustodyFailure<'a>,
    > {
        let ResidentProcessA2aCancelTaskRequestCustodyFailure {
            key,
            selector,
            phase,
        } = pending;
        let ResidentProcessA2aCancelTaskRequestCustodyPhase::Continuing(lower) = phase else {
            return Err(ResidentProcessA2aCancelTaskRequestCustodyFailure {
                key,
                selector,
                phase,
            });
        };
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || lower.key() != key.as_ref()
        {
            return Err(ResidentProcessA2aCancelTaskRequestCustodyFailure {
                key,
                selector,
                phase: ResidentProcessA2aCancelTaskRequestCustodyPhase::Continuing(lower),
            });
        }
        let classified = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.classify_non_dispatchable_replay_a2a_cancel_task(*lower)
                },
            )
            .expect("the active CancelTask route was fenced before replay classification");
        match classified {
            Ok(lower) => Ok(ResidentProcessNonDispatchableReplayA2aCancelTask {
                key,
                selector,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aCancelTaskRequestCustodyFailure {
                key,
                selector,
                phase: ResidentProcessA2aCancelTaskRequestCustodyPhase::Continuing(Box::new(lower)),
            }),
        }
    }

    /// Release lower aggregate capacity, then the route, for non-new custody.
    pub(super) fn finish_non_dispatchable_replay_a2a_cancel_task<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessNonDispatchableReplayA2aCancelTask,
    ) -> Result<
        CompletedOperationObservationCustody,
        ResidentProcessNonDispatchableReplayA2aCancelTaskReleaseFailure,
    > {
        let ResidentProcessNonDispatchableReplayA2aCancelTask {
            key,
            selector,
            lower,
        } = pending;
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || lower.key() != key.as_ref()
        {
            return Err(
                ResidentProcessNonDispatchableReplayA2aCancelTaskReleaseFailure::Selection(
                    ResidentProcessNonDispatchableReplayA2aCancelTask {
                        key,
                        selector,
                        lower,
                    },
                ),
            );
        }
        let finished = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.finish_non_dispatchable_replay_a2a_cancel_task(*lower)
                },
            )
            .expect("the active CancelTask route was fenced before aggregate release");
        match finished {
            Ok(custody) => {
                let removed = self.a2a_task_routes.remove_active(&key, &selector);
                debug_assert!(removed);
                Ok(custody)
            }
            Err(lower) => Err(
                ResidentProcessNonDispatchableReplayA2aCancelTaskReleaseFailure::Lower {
                    key,
                    selector,
                    lower: Box::new(lower),
                },
            ),
        }
    }

    /// Retry only lower aggregate-offer release for non-new custody.
    pub(super) fn retry_non_dispatchable_replay_a2a_cancel_task_release<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessNonDispatchableReplayA2aCancelTaskReleaseFailure,
    ) -> Result<
        CompletedOperationObservationCustody,
        ResidentProcessNonDispatchableReplayA2aCancelTaskReleaseFailure,
    > {
        let ResidentProcessNonDispatchableReplayA2aCancelTaskReleaseFailure::Lower {
            key,
            selector,
            lower,
        } = pending
        else {
            return Err(pending);
        };
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || lower.key() != key.as_ref()
        {
            return Err(
                ResidentProcessNonDispatchableReplayA2aCancelTaskReleaseFailure::Lower {
                    key,
                    selector,
                    lower,
                },
            );
        }
        let retried = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retry_non_dispatchable_replay_a2a_cancel_task_release(*lower)
                },
            )
            .expect("the active CancelTask route was fenced before release retry");
        match retried {
            Ok(custody) => {
                let removed = self.a2a_task_routes.remove_active(&key, &selector);
                debug_assert!(removed);
                Ok(custody)
            }
            Err(lower) => Err(
                ResidentProcessNonDispatchableReplayA2aCancelTaskReleaseFailure::Lower {
                    key,
                    selector,
                    lower: Box::new(lower),
                },
            ),
        }
    }

    /// Accept aggregate capacity, then apply fresh authority before carrier start.
    pub(super) fn start_a2a_cancel_task<'a, Authority, Authorize>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        unopened: ResidentProcessRetainedUnopenedA2aCancelTask<'a>,
        start: ResidentA2aCancelTaskStartContext<'_, Authorize>,
    ) -> Result<
        ResidentProcessA2aCancelTaskStart<'a>,
        ResidentProcessA2aCancelTaskStartTransitionFailure<'a>,
    >
    where
        Authorize:
            FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
    {
        let ResidentProcessRetainedUnopenedA2aCancelTask {
            key,
            selector,
            lower,
        } = unopened;
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || lower.key() != key.as_ref()
            || a2a_cancel_task_key(start.history) != *key
        {
            return Err(ResidentProcessA2aCancelTaskStartTransitionFailure {
                key,
                selector,
                lower: Box::new(ResidentA2aCancelTaskStartTransitionFailure::Association {
                    reservation: lower.reservation,
                    retained: lower.retained,
                }),
            });
        }
        let started = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.start_owned_a2a_cancel_task(*lower, start)
                },
            )
            .expect("the active CancelTask route was fenced before start");
        match started {
            Ok(started) => Ok(self.map_a2a_cancel_task_start(key, selector, started)),
            Err(lower) => Err(ResidentProcessA2aCancelTaskStartTransitionFailure {
                key,
                selector,
                lower: Box::new(lower),
            }),
        }
    }

    /// Retry an unopened offered start after correcting association inputs.
    pub(super) fn retry_a2a_cancel_task_start_association<'a, Authority, Authorize>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aCancelTaskStartTransitionFailure<'a>,
        start: ResidentA2aCancelTaskStartContext<'_, Authorize>,
    ) -> Result<
        ResidentProcessA2aCancelTaskStart<'a>,
        ResidentProcessA2aCancelTaskStartTransitionFailure<'a>,
    >
    where
        Authorize:
            FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
    {
        let ResidentProcessA2aCancelTaskStartTransitionFailure {
            key,
            selector,
            lower,
        } = pending;
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || lower.key() != key.as_ref()
            || a2a_cancel_task_key(start.history) != *key
        {
            return Err(ResidentProcessA2aCancelTaskStartTransitionFailure {
                key,
                selector,
                lower,
            });
        }
        let retried = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retry_a2a_cancel_task_start_association(*lower, start)
                },
            )
            .expect("the active CancelTask route was fenced before association retry");
        match retried {
            Ok(started) => Ok(self.map_a2a_cancel_task_start(key, selector, started)),
            Err(lower) => Err(ResidentProcessA2aCancelTaskStartTransitionFailure {
                key,
                selector,
                lower: Box::new(lower),
            }),
        }
    }

    /// Retry only process-local aggregate acceptance before carrier use.
    pub(super) fn retry_a2a_cancel_task_aggregate_acceptance<'a, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aCancelTaskStartTransitionFailure<'a>,
    ) -> Result<
        ResidentProcessAggregateAcceptedA2aCancelTask<'a>,
        ResidentProcessA2aCancelTaskStartTransitionFailure<'a>,
    > {
        let ResidentProcessA2aCancelTaskStartTransitionFailure {
            key,
            selector,
            lower,
        } = pending;
        let ResidentA2aCancelTaskStartTransitionFailure::AggregateAcceptance(lower) = *lower else {
            return Err(ResidentProcessA2aCancelTaskStartTransitionFailure {
                key,
                selector,
                lower,
            });
        };
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || lower.key() != key.as_ref()
        {
            return Err(ResidentProcessA2aCancelTaskStartTransitionFailure {
                key,
                selector,
                lower: Box::new(
                    ResidentA2aCancelTaskStartTransitionFailure::AggregateAcceptance(lower),
                ),
            });
        }
        let retried = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retry_a2a_cancel_task_aggregate_acceptance(lower)
                },
            )
            .expect("the active CancelTask route was fenced before acceptance retry");
        match retried {
            Ok(lower) => Ok(ResidentProcessAggregateAcceptedA2aCancelTask {
                key,
                selector,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aCancelTaskStartTransitionFailure {
                key,
                selector,
                lower: Box::new(
                    ResidentA2aCancelTaskStartTransitionFailure::AggregateAcceptance(lower),
                ),
            }),
        }
    }

    /// Apply fresh preflight to an aggregate-accepted unopened request.
    pub(super) fn start_aggregate_accepted_a2a_cancel_task<'a, Authority, Authorize>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        accepted: ResidentProcessAggregateAcceptedA2aCancelTask<'a>,
        start: ResidentA2aCancelTaskStartContext<'_, Authorize>,
    ) -> ResidentProcessA2aCancelTaskStart<'a>
    where
        Authorize:
            FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
    {
        let ResidentProcessAggregateAcceptedA2aCancelTask {
            key,
            selector,
            lower,
        } = accepted;
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || lower.key() != key.as_ref()
            || a2a_cancel_task_key(start.history) != *key
        {
            return ResidentProcessA2aCancelTaskStart::PreflightRejected(
                ResidentProcessA2aCancelTaskPreflightFailure {
                    key,
                    selector,
                    lower: Box::new(ResidentA2aCancelTaskPreflightFailure {
                        aggregate_accepted: *lower,
                        reason: A2aCancelTaskError::Association,
                    }),
                },
            );
        }
        let started = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.start_aggregate_accepted_a2a_cancel_task(*lower, start)
                },
            )
            .expect("the active CancelTask route was fenced before fresh preflight");
        self.map_a2a_cancel_task_start(key, selector, started)
    }

    /// Retry only fresh preflight for the same unopened accepted request.
    pub(super) fn retry_a2a_cancel_task_preflight<'a, Authority, Authorize>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aCancelTaskPreflightFailure<'a>,
        start: ResidentA2aCancelTaskStartContext<'_, Authorize>,
    ) -> ResidentProcessA2aCancelTaskStart<'a>
    where
        Authorize:
            FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
    {
        let ResidentProcessA2aCancelTaskPreflightFailure {
            key,
            selector,
            lower,
        } = pending;
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || lower.aggregate_accepted.key() != key.as_ref()
            || a2a_cancel_task_key(start.history) != *key
        {
            return ResidentProcessA2aCancelTaskStart::PreflightRejected(
                ResidentProcessA2aCancelTaskPreflightFailure {
                    key,
                    selector,
                    lower,
                },
            );
        }
        let started = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retry_a2a_cancel_task_preflight(*lower, start)
                },
            )
            .expect("the active CancelTask route was fenced before preflight retry");
        self.map_a2a_cancel_task_start(key, selector, started)
    }

    /// Advance one exact routed response read by at most one lower phase.
    pub(super) fn poll_a2a_cancel_task<Authority>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        key: &ResidentA2aTaskWorkKey,
        poll: ResidentProcessA2aCancelTaskPollContext<'_, '_>,
    ) -> ResidentProcessA2aCancelTaskPoll {
        if let Err(reason) = self
            .a2a_task_routes
            .require_active(key, selector, access.store)
        {
            return ResidentProcessA2aCancelTaskPoll::Selection(reason);
        }
        let polled = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.poll_owned_a2a_cancel_task(key, poll.cx, poll.protocol, poll.clock)
                },
            )
            .expect("the active CancelTask route was fenced before polling");
        self.map_a2a_cancel_task_poll(key.clone(), selector.clone(), polled)
    }

    /// Stop only one local response read; do not claim remote cancellation.
    pub(super) fn stop_a2a_cancel_task_response_read<Authority>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        key: &ResidentA2aTaskWorkKey,
    ) -> ResidentProcessA2aCancelTaskPoll {
        if let Err(reason) = self
            .a2a_task_routes
            .require_active(key, selector, access.store)
        {
            return ResidentProcessA2aCancelTaskPoll::Selection(reason);
        }
        let stopped = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.stop_owned_a2a_cancel_task_response_read(key)
                },
            )
            .expect("the active CancelTask route was fenced before response-read stop");
        self.map_a2a_cancel_task_poll(key.clone(), selector.clone(), stopped)
    }

    /// Retry only the aggregate accepted-to-completion transition.
    pub(super) fn retry_a2a_cancel_task_aggregate_transition<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        key: ResidentA2aTaskWorkKey,
        selector: ResidentDeploymentAuthoritySelector,
        rejection: ResidentA2aCancelTaskAggregateTransitionRejection,
    ) -> ResidentProcessA2aCancelTaskPoll {
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || rejection.key() != &key
        {
            return ResidentProcessA2aCancelTaskPoll::AggregateTransitionRejected {
                key: Box::new(key),
                selector: Box::new(selector),
                rejection: Box::new(rejection),
            };
        }
        let retried = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retry_owned_a2a_cancel_task_aggregate_transition(rejection)
                },
            )
            .expect("the active CancelTask route was fenced before aggregate retry");
        self.map_a2a_cancel_task_poll(key, selector, retried)
    }

    /// Select only a complete qualified read-back under its checked-out route.
    pub(super) fn select_a2a_cancel_task_readback<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aCancelTaskAwaitingCustody,
    ) -> Result<
        ResidentProcessSelectedA2aCancelTaskReadback,
        ResidentProcessA2aCancelTaskAwaitingCustody,
    > {
        let ResidentProcessA2aCancelTaskAwaitingCustody { route, lower } = pending;
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.store != *access.store
            || lower.key() != route.key.as_ref()
        {
            return Err(ResidentProcessA2aCancelTaskAwaitingCustody { route, lower });
        }
        let selected = self
            .with_selected_operation_service(
                &route.selector,
                access,
                authority,
                |_application, service, _sessions| service.select_a2a_cancel_task_readback(*lower),
            )
            .expect("the checked-out CancelTask route was fenced before read-back selection");
        match selected {
            Ok(lower) => Ok(ResidentProcessSelectedA2aCancelTaskReadback {
                route,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aCancelTaskAwaitingCustody {
                route,
                lower: Box::new(lower),
            }),
        }
    }

    /// Retain a complete response as remote history under the checked-out route.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn retain_a2a_cancel_task_readback<
        Authority,
        Rotation,
        ObservationSign,
        Store,
        CheckpointSign,
    >(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        selected: ResidentProcessSelectedA2aCancelTaskReadback,
        input: A2aCancelTaskReadbackCommitInput<'_>,
        protocol: &dyn A2aCancelTaskEvidencePort,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        resolve_store: Store,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessRetainedA2aCancelTaskOutcome,
        ResidentProcessA2aCancelTaskReadbackRetentionFailure,
    >
    where
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &pale_core::records::A2aTaskObservationPreimageV1,
            &pale_core::records::A2aTaskObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Store: FnOnce(
            &pale_core::records::A2aTaskObservationV1,
        )
            -> Result<pale_ports::AuthorityStoreCoordinate, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentProcessSelectedA2aCancelTaskReadback { route, lower } = selected;
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.store != *access.store
            || lower.key() != route.key.as_ref()
        {
            return Err(ResidentProcessA2aCancelTaskReadbackRetentionFailure {
                route,
                phase: ResidentProcessA2aCancelTaskReadbackRetentionPhase::Selected(lower),
            });
        }
        let retained = self
            .with_selected_operation_service(
                &route.selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retain_a2a_cancel_task_readback(
                        *lower,
                        input,
                        protocol,
                        admit_rotation,
                        sign_observation,
                        resolve_store,
                        sign_checkpoint,
                    )
                },
            )
            .expect("the checked-out CancelTask route was fenced before read-back custody");
        match retained {
            Ok(lower) => Ok(ResidentProcessRetainedA2aCancelTaskOutcome {
                route,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aCancelTaskReadbackRetentionFailure {
                route,
                phase: ResidentProcessA2aCancelTaskReadbackRetentionPhase::Continuing(Box::new(
                    lower,
                )),
            }),
        }
    }

    /// Recover a pre-physical read-back failure without carrier or physical I/O.
    pub(super) fn recover_a2a_cancel_task_readback_prephysical<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aCancelTaskReadbackRetentionFailure,
    ) -> Result<
        ResidentProcessSelectedA2aCancelTaskReadback,
        ResidentProcessA2aCancelTaskReadbackRetentionFailure,
    > {
        let ResidentProcessA2aCancelTaskReadbackRetentionFailure { route, phase } = pending;
        let ResidentProcessA2aCancelTaskReadbackRetentionPhase::Continuing(lower) = phase else {
            return Err(ResidentProcessA2aCancelTaskReadbackRetentionFailure { route, phase });
        };
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.store != *access.store
            || lower.key() != route.key.as_ref()
        {
            return Err(ResidentProcessA2aCancelTaskReadbackRetentionFailure {
                route,
                phase: ResidentProcessA2aCancelTaskReadbackRetentionPhase::Continuing(lower),
            });
        }
        let recovered = self
            .with_selected_operation_service(
                &route.selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.recover_a2a_cancel_task_readback_prephysical(*lower)
                },
            )
            .expect("the checked-out CancelTask route was fenced before phase recovery");
        match recovered {
            Ok(lower) => Ok(ResidentProcessSelectedA2aCancelTaskReadback {
                route,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aCancelTaskReadbackRetentionFailure {
                route,
                phase: ResidentProcessA2aCancelTaskReadbackRetentionPhase::Continuing(Box::new(
                    lower,
                )),
            }),
        }
    }

    /// Continue exact read-back database or anchor custody without carrier access.
    pub(super) fn continue_a2a_cancel_task_readback_custody<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aCancelTaskReadbackRetentionFailure,
    ) -> Result<
        ResidentProcessRetainedA2aCancelTaskOutcome,
        ResidentProcessA2aCancelTaskReadbackRetentionFailure,
    > {
        let ResidentProcessA2aCancelTaskReadbackRetentionFailure { route, phase } = pending;
        let ResidentProcessA2aCancelTaskReadbackRetentionPhase::Continuing(lower) = phase else {
            return Err(ResidentProcessA2aCancelTaskReadbackRetentionFailure { route, phase });
        };
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.store != *access.store
            || lower.key() != route.key.as_ref()
        {
            return Err(ResidentProcessA2aCancelTaskReadbackRetentionFailure {
                route,
                phase: ResidentProcessA2aCancelTaskReadbackRetentionPhase::Continuing(lower),
            });
        }
        let continued = self
            .with_selected_operation_service(
                &route.selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.continue_a2a_cancel_task_readback_custody(*lower)
                },
            )
            .expect("the checked-out CancelTask route was fenced before custody continuation");
        match continued {
            Ok(lower) => Ok(ResidentProcessRetainedA2aCancelTaskOutcome {
                route,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aCancelTaskReadbackRetentionFailure {
                route,
                phase: ResidentProcessA2aCancelTaskReadbackRetentionPhase::Continuing(Box::new(
                    lower,
                )),
            }),
        }
    }

    /// Select only an incomplete outcome under its checked-out route.
    pub(super) fn select_a2a_cancel_task_disposition<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aCancelTaskAwaitingCustody,
    ) -> Result<
        ResidentProcessSelectedA2aCancelTaskDisposition,
        ResidentProcessA2aCancelTaskAwaitingCustody,
    > {
        let ResidentProcessA2aCancelTaskAwaitingCustody { route, lower } = pending;
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.store != *access.store
            || lower.key() != route.key.as_ref()
        {
            return Err(ResidentProcessA2aCancelTaskAwaitingCustody { route, lower });
        }
        let selected = self
            .with_selected_operation_service(
                &route.selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.select_a2a_cancel_task_disposition(*lower)
                },
            )
            .expect("the checked-out CancelTask route was fenced before disposition selection");
        match selected {
            Ok(lower) => Ok(ResidentProcessSelectedA2aCancelTaskDisposition {
                route,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aCancelTaskAwaitingCustody {
                route,
                lower: Box::new(lower),
            }),
        }
    }

    /// Retain one incomplete outcome as local disposition under its route.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn retain_a2a_cancel_task_disposition<
        Authority,
        Reconciliation,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        selected: ResidentProcessSelectedA2aCancelTaskDisposition,
        input: A2aCancelTaskDispositionCommitInput<'_>,
        resolve_store: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessRetainedA2aCancelTaskOutcome,
        ResidentProcessA2aCancelTaskDispositionRetentionFailure,
    >
    where
        Reconciliation: FnOnce(
            &pale_core::records::ReconciliationFrontierReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
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
        let ResidentProcessSelectedA2aCancelTaskDisposition { route, lower } = selected;
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.store != *access.store
            || lower.key() != route.key.as_ref()
        {
            return Err(ResidentProcessA2aCancelTaskDispositionRetentionFailure {
                route,
                phase: ResidentProcessA2aCancelTaskDispositionRetentionPhase::Selected(lower),
            });
        }
        let retained = self
            .with_selected_operation_service(
                &route.selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retain_a2a_cancel_task_disposition(
                        *lower,
                        input,
                        resolve_store,
                        admit_reconciliation,
                        admit_rotation,
                        sign_observation,
                        sign_checkpoint,
                    )
                },
            )
            .expect("the checked-out CancelTask route was fenced before disposition custody");
        match retained {
            Ok(lower) => Ok(ResidentProcessRetainedA2aCancelTaskOutcome {
                route,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aCancelTaskDispositionRetentionFailure {
                route,
                phase: ResidentProcessA2aCancelTaskDispositionRetentionPhase::Continuing(Box::new(
                    lower,
                )),
            }),
        }
    }

    /// Recover pre-physical disposition failure without carrier or physical I/O.
    pub(super) fn recover_a2a_cancel_task_disposition_prephysical<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aCancelTaskDispositionRetentionFailure,
    ) -> Result<
        ResidentProcessSelectedA2aCancelTaskDisposition,
        ResidentProcessA2aCancelTaskDispositionRetentionFailure,
    > {
        let ResidentProcessA2aCancelTaskDispositionRetentionFailure { route, phase } = pending;
        let ResidentProcessA2aCancelTaskDispositionRetentionPhase::Continuing(lower) = phase else {
            return Err(ResidentProcessA2aCancelTaskDispositionRetentionFailure { route, phase });
        };
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.store != *access.store
            || lower.key() != route.key.as_ref()
        {
            return Err(ResidentProcessA2aCancelTaskDispositionRetentionFailure {
                route,
                phase: ResidentProcessA2aCancelTaskDispositionRetentionPhase::Continuing(lower),
            });
        }
        let recovered = self
            .with_selected_operation_service(
                &route.selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.recover_a2a_cancel_task_disposition_prephysical(*lower)
                },
            )
            .expect("the checked-out CancelTask route was fenced before disposition recovery");
        match recovered {
            Ok(lower) => Ok(ResidentProcessSelectedA2aCancelTaskDisposition {
                route,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aCancelTaskDispositionRetentionFailure {
                route,
                phase: ResidentProcessA2aCancelTaskDispositionRetentionPhase::Continuing(Box::new(
                    lower,
                )),
            }),
        }
    }

    /// Continue exact disposition database or anchor custody without carrier use.
    pub(super) fn continue_a2a_cancel_task_disposition_custody<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aCancelTaskDispositionRetentionFailure,
    ) -> Result<
        ResidentProcessRetainedA2aCancelTaskOutcome,
        ResidentProcessA2aCancelTaskDispositionRetentionFailure,
    > {
        let ResidentProcessA2aCancelTaskDispositionRetentionFailure { route, phase } = pending;
        let ResidentProcessA2aCancelTaskDispositionRetentionPhase::Continuing(lower) = phase else {
            return Err(ResidentProcessA2aCancelTaskDispositionRetentionFailure { route, phase });
        };
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.store != *access.store
            || lower.key() != route.key.as_ref()
        {
            return Err(ResidentProcessA2aCancelTaskDispositionRetentionFailure {
                route,
                phase: ResidentProcessA2aCancelTaskDispositionRetentionPhase::Continuing(lower),
            });
        }
        let continued = self
            .with_selected_operation_service(
                &route.selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.continue_a2a_cancel_task_disposition_custody(*lower)
                },
            )
            .expect("the checked-out CancelTask route was fenced before disposition continuation");
        match continued {
            Ok(lower) => Ok(ResidentProcessRetainedA2aCancelTaskOutcome {
                route,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aCancelTaskDispositionRetentionFailure {
                route,
                phase: ResidentProcessA2aCancelTaskDispositionRetentionPhase::Continuing(Box::new(
                    lower,
                )),
            }),
        }
    }

    /// Release lower aggregate capacity, then the checked-out process route.
    pub(super) fn finish_a2a_cancel_task<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessRetainedA2aCancelTaskOutcome,
    ) -> Result<ResidentA2aCancelTaskDurableCustody, ResidentProcessA2aCancelTaskReleaseFailure>
    {
        let ResidentProcessRetainedA2aCancelTaskOutcome { route, lower } = pending;
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.store != *access.store
            || lower.key() != route.key.as_ref()
        {
            return Err(ResidentProcessA2aCancelTaskReleaseFailure::Selection(
                ResidentProcessRetainedA2aCancelTaskOutcome { route, lower },
            ));
        }
        let released = self
            .with_selected_operation_service(
                &route.selector,
                access,
                authority,
                |_application, service, _sessions| service.finish_a2a_cancel_task(*lower),
            )
            .expect("the checked-out CancelTask route was fenced before aggregate release");
        match released {
            Ok(custody) => match self.a2a_task_routes.release(route) {
                Ok(()) => Ok(custody),
                Err(route) => Err(ResidentProcessA2aCancelTaskReleaseFailure::Route {
                    route,
                    custody: Box::new(custody),
                }),
            },
            Err(lower) => Err(ResidentProcessA2aCancelTaskReleaseFailure::Lower {
                route,
                lower: Box::new(lower),
            }),
        }
    }

    /// Retry only the unresolved lower aggregate or process-route release.
    pub(super) fn retry_a2a_cancel_task_release<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aCancelTaskReleaseFailure,
    ) -> Result<ResidentA2aCancelTaskDurableCustody, ResidentProcessA2aCancelTaskReleaseFailure>
    {
        match pending {
            ResidentProcessA2aCancelTaskReleaseFailure::Lower { route, lower } => {
                if !self.a2a_task_routes.owns_custody(&route)
                    || route.key.store != *access.store
                    || lower.key() != route.key.as_ref()
                {
                    return Err(ResidentProcessA2aCancelTaskReleaseFailure::Lower { route, lower });
                }
                let retried = self
                    .with_selected_operation_service(
                        &route.selector,
                        access,
                        authority,
                        |_application, service, _sessions| {
                            service.retry_a2a_cancel_task_release(*lower)
                        },
                    )
                    .expect("the checked-out CancelTask route was fenced before release retry");
                match retried {
                    Ok(custody) => match self.a2a_task_routes.release(route) {
                        Ok(()) => Ok(custody),
                        Err(route) => Err(ResidentProcessA2aCancelTaskReleaseFailure::Route {
                            route,
                            custody: Box::new(custody),
                        }),
                    },
                    Err(lower) => Err(ResidentProcessA2aCancelTaskReleaseFailure::Lower {
                        route,
                        lower: Box::new(lower),
                    }),
                }
            }
            ResidentProcessA2aCancelTaskReleaseFailure::Route { route, custody } => {
                if !self.a2a_task_routes.owns_custody(&route) || route.key.store != *access.store {
                    return Err(ResidentProcessA2aCancelTaskReleaseFailure::Route {
                        route,
                        custody,
                    });
                }
                match self.a2a_task_routes.release(route) {
                    Ok(()) => Ok(*custody),
                    Err(route) => {
                        Err(ResidentProcessA2aCancelTaskReleaseFailure::Route { route, custody })
                    }
                }
            }
            other => Err(other),
        }
    }

    fn map_a2a_cancel_task_start<'a>(
        &mut self,
        key: Box<ResidentA2aTaskWorkKey>,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        started: ResidentA2aCancelTaskStart<'a>,
    ) -> ResidentProcessA2aCancelTaskStart<'a> {
        match started {
            ResidentA2aCancelTaskStart::Active => ResidentProcessA2aCancelTaskStart::Active,
            ResidentA2aCancelTaskStart::PreflightRejected(lower) => {
                ResidentProcessA2aCancelTaskStart::PreflightRejected(
                    ResidentProcessA2aCancelTaskPreflightFailure {
                        key,
                        selector,
                        lower: Box::new(lower),
                    },
                )
            }
            ResidentA2aCancelTaskStart::AwaitingCustody(lower) => {
                let route = self
                    .a2a_task_routes
                    .check_out(&key)
                    .expect("the active CancelTask route must remain present");
                ResidentProcessA2aCancelTaskStart::AwaitingCustody(
                    ResidentProcessA2aCancelTaskAwaitingCustody {
                        route,
                        lower: Box::new(lower),
                    },
                )
            }
            ResidentA2aCancelTaskStart::AggregateTransitionRejected(rejection) => {
                ResidentProcessA2aCancelTaskStart::AggregateTransitionRejected {
                    key,
                    selector,
                    rejection: Box::new(rejection),
                }
            }
        }
    }

    fn map_a2a_cancel_task_poll(
        &mut self,
        key: ResidentA2aTaskWorkKey,
        selector: ResidentDeploymentAuthoritySelector,
        polled: ResidentA2aCancelTaskOwnerPoll,
    ) -> ResidentProcessA2aCancelTaskPoll {
        match polled {
            ResidentA2aCancelTaskOwnerPoll::PendingResponse => {
                ResidentProcessA2aCancelTaskPoll::PendingResponse
            }
            ResidentA2aCancelTaskOwnerPoll::ResponseReceived => {
                ResidentProcessA2aCancelTaskPoll::ResponseReceived
            }
            ResidentA2aCancelTaskOwnerPoll::PendingEof => {
                ResidentProcessA2aCancelTaskPoll::PendingEof
            }
            ResidentA2aCancelTaskOwnerPoll::AwaitingCustody(lower) => {
                let route = self
                    .a2a_task_routes
                    .check_out(&key)
                    .expect("the active CancelTask route must remain present");
                ResidentProcessA2aCancelTaskPoll::AwaitingCustody(
                    ResidentProcessA2aCancelTaskAwaitingCustody {
                        route,
                        lower: Box::new(lower),
                    },
                )
            }
            ResidentA2aCancelTaskOwnerPoll::AggregateTransitionRejected(rejection) => {
                ResidentProcessA2aCancelTaskPoll::AggregateTransitionRejected {
                    key: Box::new(key),
                    selector: Box::new(selector),
                    rejection: Box::new(rejection),
                }
            }
            ResidentA2aCancelTaskOwnerPoll::NotFound => ResidentProcessA2aCancelTaskPoll::NotFound,
        }
    }
}

fn validate_recovered_a2a_cancel_task(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &pale_ports::AuthorityStoreCoordinate,
    recovered: &ResidentProcessRecoveredA2aCancelTaskRequest,
    input: &A2aCancelTaskDispositionCommitInput<'_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let operation = recovered.attempt.start.control.operation().operation();
    let expected_history = recovered.task.as_ref();
    let supplied_history = input.disposition.history;
    let expected_head = expected_history.local_observation().selected_head();
    let supplied_head = supplied_history.local_observation().selected_head();
    let control_head = recovered
        .attempt
        .start
        .control
        .observation()
        .selected_head();
    let control_dispatch = recovered.attempt.start.control.observation().selected();
    let task_dispatch = expected_history.dispatch();
    let request = pale_core::records::ImmutableRecordReferenceV1 {
        record_ref: recovered.request_evidence.coordinate().record_ref().clone(),
        record_sha256: ArtifactDigest::hash_bytes(recovered.request_evidence.bytes()),
    };
    let fresh = [
        &input.disposition.diagnostic_ref,
        &input.disposition.observation_ref,
        &input.disposition.reconciliation.record_ref,
        &input.checkpoint.reference,
    ];
    let retained = [
        operation.request_coordinate().record_ref(),
        operation.handle().0,
        recovered.attempt.selection.coordinate().record_ref(),
        recovered.request_evidence.coordinate().record_ref(),
        expected_history.route().0,
        expected_history.binding().0,
        expected_history.observation().0,
        expected_history.selected_observation().0,
        task_dispatch.0,
        expected_head.0,
        expected_history.local_observation().current_checkpoint().0,
    ];
    if recovered.attempt.dispatch != ResidentProcessAttemptObservation::CancellationPredecessor
        || !matches!(
            recovered.attempt.start.lineage,
            ResidentProcessOperationLineage::PairwiseDelegate(_)
        )
        || !selector.matches_operation(operation)
        || operation.handle().1.operation_class() != &pale_core::records::OperationClassV1::Delegate
        || recovered.attempt.start.control.operation().store() != store
        || recovered.attempt.start.control.observation().store() != store
        || expected_history.local_observation().store() != store
        || supplied_history.local_observation().store() != store
        || input.disposition.operation.handle().0 != operation.handle().0
        || input
            .disposition
            .operation
            .handle()
            .1
            .canonical_operation_handle_sha256()
            != operation.handle().1.canonical_operation_handle_sha256()
        || !dispatch_selects_record(task_dispatch.1, &recovered.attempt.selection)
        || task_dispatch.0 != control_dispatch.0
        || task_dispatch.1.canonical_operation_observation_sha256()
            != control_dispatch.1.canonical_operation_observation_sha256()
        || fresh.iter().enumerate().any(|(index, reference)| {
            retained.contains(reference) || fresh[..index].contains(reference)
        })
        || !same_cancel_task_history(expected_history, supplied_history)
        || expected_head.0 != supplied_head.0
        || expected_head.1.canonical_operation_observation_sha256()
            != supplied_head.1.canonical_operation_observation_sha256()
        || expected_head.0 != control_head.0
        || expected_head.1.canonical_operation_observation_sha256()
            != control_head.1.canonical_operation_observation_sha256()
        || !matches!(
            expected_head.1.state(),
            OperationObservationStateV1::CancellationRequested { request: retained }
                if retained == &request
        )
    {
        return Err(ResidentProcessHostSelectionError::A2aOperationAssociation);
    }
    let route = expected_history.route().1;
    if route.verify().is_err()
        || route.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
        || route.deployment_runtime_authority_sha256().as_str()
            != selector.runtime_authority_sha256.as_str()
        || route.issuer_principal_id() != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::A2aRouteAuthority);
    }
    let binding = expected_history.binding().1;
    if binding.operation_handle_ref() != operation.handle().0
        || binding.operation_handle_sha256()
            != operation.handle().1.canonical_operation_handle_sha256()
        || !selector.matches_signing_authority(input.disposition.signing.signing_authority)
        || !operation_observation_matches_cancel_selector(selector, expected_head.1)
        || !checkpoint_authority_matches(selector, store, input.checkpoint_authority)
    {
        return Err(ResidentProcessHostSelectionError::A2aOperationAssociation);
    }
    let checkpoint = expected_history.local_observation().current_checkpoint().1;
    if checkpoint.control_store_id() != store.store_id()
        || checkpoint.control_incarnation_id() != store.incarnation_id()
        || checkpoint.issuer_principal_id() != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::CheckpointAuthorityAssociation);
    }
    Ok(())
}

fn dispatch_selects_record(
    dispatch: &OperationObservationV1,
    selection: &pale_ports::ControlRecord,
) -> bool {
    matches!(
        dispatch.state(),
        OperationObservationStateV1::Active {
            phase: pale_core::records::OperationActivePhaseV1::Dispatched { evidence, .. },
        } if evidence.record_ref == *selection.coordinate().record_ref()
            && evidence.record_sha256.as_str()
                == selection.coordinate().record_sha256().as_str()
    )
}

fn same_cancel_task_history(
    expected: &crate::resident::RecoveredA2aTaskHistory,
    supplied: &crate::resident::RecoveredA2aTaskHistory,
) -> bool {
    let expected_route = expected.route();
    let supplied_route = supplied.route();
    let expected_binding = expected.binding();
    let supplied_binding = supplied.binding();
    let expected_observation = expected.observation();
    let supplied_observation = supplied.observation();
    let expected_selected = expected.selected_observation();
    let supplied_selected = supplied.selected_observation();
    let expected_dispatch = expected.dispatch();
    let supplied_dispatch = supplied.dispatch();
    expected_route.0 == supplied_route.0
        && expected_route.1.canonical_a2a_route_admission_sha256()
            == supplied_route.1.canonical_a2a_route_admission_sha256()
        && expected_binding.0 == supplied_binding.0
        && expected_binding.1.canonical_a2a_task_binding_sha256()
            == supplied_binding.1.canonical_a2a_task_binding_sha256()
        && expected_observation.0 == supplied_observation.0
        && expected_observation
            .1
            .canonical_a2a_task_observation_sha256()
            == supplied_observation
                .1
                .canonical_a2a_task_observation_sha256()
        && expected_selected.0 == supplied_selected.0
        && expected_selected.1.canonical_a2a_task_observation_sha256()
            == supplied_selected.1.canonical_a2a_task_observation_sha256()
        && expected_dispatch.0 == supplied_dispatch.0
        && expected_dispatch.1.canonical_operation_observation_sha256()
            == supplied_dispatch.1.canonical_operation_observation_sha256()
}

fn operation_observation_matches_cancel_selector(
    selector: &ResidentDeploymentAuthoritySelector,
    observation: &OperationObservationV1,
) -> bool {
    observation.deployment_runtime_authority_ref() == &selector.runtime_authority_ref
        && observation.deployment_runtime_authority_sha256().as_str()
            == selector.runtime_authority_sha256.as_str()
        && observation.issuer_principal_id() == &selector.principal_id
}
