//! Deployment routing for A2A `GetTask` history reads.
//!
//! The resident operation service owns carrier and durable-history phases. This
//! module keeps each phase on the deployment and binding that admitted it.

use std::task::Context;

use pale_core::records::{
    ArtifactDigest, CanonicalSignatureV1, ControlCheckpointSigningAuthorityAdmissionContextV1,
    ControlStateCheckpointDigest, ControlStateCheckpointPreimageV1, InvalidResidentRecordV1,
    ResidentRecordRefV1, SchemaDigest, UnixTimeV1,
};
use pale_ports::{
    A2aExchangePort, A2aGetTaskEvidencePort, AccumulatorMembershipLimits,
    AccumulatorPreparationLimits, WallClockSourcePort,
};

use super::{
    a2a_task_routes::{ResidentProcessA2aTaskRouteCustody, ResidentProcessA2aTaskRouteInsertError},
    restart::{
        ResidentProcessAttemptObservation, ResidentProcessOperationLineage,
        ResidentProcessRecoveredA2aTask,
    },
    ResidentDeploymentAuthoritySelector, ResidentDeploymentOperationAccess, ResidentProcessHost,
    ResidentProcessHostSelectionError,
};
use crate::resident::a2a_get_task_owner::{
    a2a_get_task_key, ResidentA2aGetTaskAggregateAccepted,
    ResidentA2aGetTaskAggregateTransitionRejection, ResidentA2aGetTaskAwaitingCustody,
    ResidentA2aGetTaskClosedDisposition, ResidentA2aGetTaskDurableStart,
    ResidentA2aGetTaskDurableStartFailure, ResidentA2aGetTaskPoll,
    ResidentA2aGetTaskPreflightFailure, ResidentA2aGetTaskReadStoppedDisposition,
    ResidentA2aGetTaskStartFailure,
};
use crate::resident::a2a_task_custody::{
    A2aGetTaskCustodyRecoveryError, A2aGetTaskPreparationInput, RecoveredA2aGetTaskCustody,
};
use crate::resident::operation_service::{
    ResidentA2aGetTaskLocalDispositionCustodyFailure, ResidentA2aGetTaskLocalDispositionInput,
    ResidentA2aGetTaskLocalDispositionReleaseFailure,
    ResidentA2aGetTaskNonDispatchableReplayCustody,
    ResidentA2aGetTaskNonDispatchableReplayReleaseFailure, ResidentA2aGetTaskPreparationFailure,
    ResidentA2aGetTaskReleaseFailure, ResidentA2aGetTaskRequestCustodyFailure,
    ResidentA2aGetTaskRetentionFailure, ResidentPreparedA2aGetTask,
    ResidentReleasedA2aGetTaskLocalDisposition, ResidentRetainedA2aGetTask,
    ResidentRetainedA2aGetTaskLocalDisposition, ResidentRetainedUnopenedA2aGetTask,
};
use crate::resident::task_reservation::ResidentA2aTaskWorkKey;
use crate::resident::{
    A2aGetTaskCommitInput, A2aGetTaskReadbackAuthorizationContext, A2aGetTaskReadbackError,
    A2aGetTaskReadbackInput, CompletedA2aGetTaskReadbackCustody,
    CompletedOperationObservationCustody, OperationCheckpointInput,
};

/// Failure before one prepared `GetTask` request is returned.
#[must_use = "preserve an unresolved route and aggregate-offer release"]
pub(super) enum ResidentProcessA2aGetTaskPreparationFailure {
    Selection(ResidentProcessHostSelectionError),
    RouteConflict,
    RouteSaturated,
    Rejected(Box<ResidentA2aGetTaskPreparationFailure>),
    Closed(Box<A2aGetTaskReadbackError>),
    Release {
        key: Box<ResidentA2aTaskWorkKey>,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        lower: Box<ResidentA2aGetTaskPreparationFailure>,
    },
}

/// Prepared request tied to one exact binding route and deployment.
#[must_use = "retain exact physical custody before accepting aggregate capacity"]
pub(super) struct ResidentProcessPreparedA2aGetTask<'a> {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    lower: Box<ResidentPreparedA2aGetTask<'a>>,
}

enum ResidentProcessA2aGetTaskRequestCustodyPhase<'a> {
    Prepared(Box<ResidentProcessPreparedA2aGetTask<'a>>),
    Continuing(Box<ResidentA2aGetTaskRequestCustodyFailure<'a>>),
}

/// Request custody stopped while both aggregate capacity and route remain held.
#[must_use = "continue exact request custody without carrier access"]
pub(super) struct ResidentProcessA2aGetTaskRequestCustodyFailure<'a> {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    phase: ResidentProcessA2aGetTaskRequestCustodyPhase<'a>,
}

/// Retained unopened read tied to its exact active process route.
#[must_use = "accept aggregate capacity and apply fresh preflight before carrier start"]
pub(super) struct ResidentProcessRetainedUnopenedA2aGetTask<'a> {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    lower: Box<ResidentRetainedUnopenedA2aGetTask<'a>>,
}

/// Recovered custody that may release capacity but may never call the carrier.
#[must_use = "release lower aggregate capacity before the process route"]
pub(super) struct ResidentProcessNonDispatchableReplayA2aGetTask {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    lower: Box<ResidentA2aGetTaskNonDispatchableReplayCustody>,
}

/// Non-dispatchable replay custody whose aggregate release needs a retry.
#[must_use = "retry lower release before the process route"]
pub(super) enum ResidentProcessNonDispatchableReplayA2aGetTaskReleaseFailure {
    Selection(ResidentProcessNonDispatchableReplayA2aGetTask),
    Lower {
        key: Box<ResidentA2aTaskWorkKey>,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        lower: Box<ResidentA2aGetTaskNonDispatchableReplayReleaseFailure>,
    },
}

/// Offered unopened start transition retained with its exact active route.
#[must_use = "retry only process-local aggregate acceptance"]
pub(super) struct ResidentProcessA2aGetTaskDurableStartFailure<'a> {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    lower: Box<ResidentA2aGetTaskDurableStartFailure<'a>>,
}

/// Aggregate-accepted unopened request waiting for fresh authority.
#[must_use = "apply fresh preflight before the first carrier call"]
pub(super) struct ResidentProcessAggregateAcceptedA2aGetTask<'a> {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    lower: Box<ResidentA2aGetTaskAggregateAccepted<'a>>,
}

/// Fresh preflight refusal retaining the accepted unopened request and route.
#[must_use = "retry only the same unopened request with fresh authority"]
pub(super) struct ResidentProcessA2aGetTaskPreflightFailure<'a> {
    key: Box<ResidentA2aTaskWorkKey>,
    selector: Box<ResidentDeploymentAuthoritySelector>,
    lower: Box<ResidentA2aGetTaskPreflightFailure<'a>>,
}

/// Result of releasing one durable unopened request to the carrier.
#[must_use = "preserve every unopened or carrier-started phase"]
pub(super) enum ResidentProcessA2aGetTaskDurableStart<'a> {
    Active,
    PreflightRejected(ResidentProcessA2aGetTaskPreflightFailure<'a>),
    Terminal(ResidentProcessA2aGetTaskPoll),
}

/// Durable `GetTask` state found after cold restart.
#[must_use = "a retained preparation reports uncertainty and cannot be replayed"]
pub(super) enum ResidentProcessRecoveredA2aGetTaskDisposition {
    Absent,
    CarrierStartUnknown,
    LocalDisposition,
    PublicObservation,
}

/// Cold recovery failed before any route reservation or carrier work.
pub(super) enum ResidentProcessA2aGetTaskRecoveryFailure {
    Selection(ResidentProcessHostSelectionError),
    Custody(A2aGetTaskCustodyRecoveryError),
}

/// Failure while starting one routed `GetTask` read.
#[must_use = "do not replay a start whose carrier or aggregate phase is unresolved"]
pub(super) enum ResidentProcessA2aGetTaskStartFailure {
    Selection(ResidentProcessHostSelectionError),
    RouteConflict,
    RouteSaturated,
    Rejected(Box<ResidentA2aGetTaskStartFailure>),
    Closed(A2aGetTaskReadbackError),
    Unresolved {
        key: Box<ResidentA2aTaskWorkKey>,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        failure: Box<ResidentA2aGetTaskStartFailure>,
    },
}

/// Disposition after retrying a process-local start transition.
pub(super) enum ResidentProcessA2aGetTaskStartRetry {
    Started,
    Closed(A2aGetTaskReadbackError),
}

/// Terminal disposition after aggregate capacity and route custody are released.
pub(super) enum ResidentProcessA2aGetTaskTerminalRelease {
    Closed(A2aGetTaskReadbackError),
    ReadStopped,
}

/// One bounded transition of a routed `GetTask` read.
#[must_use = "retain or continue the exact routed read phase"]
pub(super) enum ResidentProcessA2aGetTaskPoll {
    PendingResponse,
    ResponseReceived,
    PendingEof,
    AwaitingCustody(ResidentProcessA2aGetTaskAwaitingCustody),
    Closed(ResidentProcessA2aGetTaskClosed),
    ReadStopped(ResidentProcessA2aGetTaskReadStopped),
    AggregateTransitionRejected {
        key: Box<ResidentA2aTaskWorkKey>,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        rejection: Box<ResidentA2aGetTaskAggregateTransitionRejection>,
    },
    NotFound,
    Selection(ResidentProcessHostSelectionError),
}

/// Completed response waiting for durable remote-history custody.
#[must_use = "retain exact history before releasing the route"]
pub(super) struct ResidentProcessA2aGetTaskAwaitingCustody {
    route: ResidentProcessA2aTaskRouteCustody,
    awaiting: Box<ResidentA2aGetTaskAwaitingCustody>,
}

/// Closed read waiting for its aggregate and route release.
#[must_use = "finish the closed disposition before releasing the route"]
pub(super) struct ResidentProcessA2aGetTaskClosed {
    route: ResidentProcessA2aTaskRouteCustody,
    closed: Box<ResidentA2aGetTaskClosedDisposition>,
}

/// Local response-read stop waiting for aggregate and route release.
#[must_use = "finish the local response-read stop before releasing the route"]
pub(super) struct ResidentProcessA2aGetTaskReadStopped {
    route: ResidentProcessA2aTaskRouteCustody,
    stopped: Box<ResidentA2aGetTaskReadStoppedDisposition>,
}

/// Routed local-disposition custody stopped before capacity release.
#[must_use = "continue custody without carrier access"]
pub(super) enum ResidentProcessA2aGetTaskLocalDispositionCustodyFailure {
    SelectionClosed(ResidentProcessA2aGetTaskClosed),
    SelectionReadStopped(ResidentProcessA2aGetTaskReadStopped),
    Lower {
        route: ResidentProcessA2aTaskRouteCustody,
        failure: Box<ResidentA2aGetTaskLocalDispositionCustodyFailure>,
    },
}

/// Durable local disposition that still owns its process route.
#[must_use = "release aggregate capacity before the process route"]
pub(super) struct ResidentProcessRetainedA2aGetTaskLocalDisposition {
    route: ResidentProcessA2aTaskRouteCustody,
    retained: ResidentRetainedA2aGetTaskLocalDisposition,
}

/// Terminal outcome after durable local custody and both capacity releases.
pub(super) enum ResidentProcessReleasedA2aGetTaskLocalDisposition {
    Closed {
        reason: A2aGetTaskReadbackError,
        custody: Box<CompletedOperationObservationCustody>,
    },
    ReadStopped {
        custody: Box<CompletedOperationObservationCustody>,
    },
}

/// Aggregate or route release stopped after durable local custody.
#[must_use = "retry release without carrier access"]
pub(super) enum ResidentProcessA2aGetTaskLocalDispositionReleaseFailure {
    Selection(ResidentProcessRetainedA2aGetTaskLocalDisposition),
    Lower {
        route: ResidentProcessA2aTaskRouteCustody,
        failure: ResidentA2aGetTaskLocalDispositionReleaseFailure,
    },
    Route {
        route: ResidentProcessA2aTaskRouteCustody,
        released: Box<ResidentProcessReleasedA2aGetTaskLocalDisposition>,
    },
}

/// Routed retention that must preserve exact lower custody on failure.
#[must_use = "continue exact custody or preserve this routed failure"]
pub(super) enum ResidentProcessA2aGetTaskRetentionFailure {
    Selection(ResidentProcessA2aGetTaskAwaitingCustody),
    Lower {
        route: ResidentProcessA2aTaskRouteCustody,
        failure: ResidentA2aGetTaskRetentionFailure,
    },
}

/// Durable remote-history custody that still owns its `ProcessHost` route.
#[must_use = "release only after lower capacity and route custody settle"]
pub(super) struct ResidentProcessRetainedA2aGetTask {
    route: ResidentProcessA2aTaskRouteCustody,
    retained: Box<ResidentRetainedA2aGetTask>,
}

/// Failure while releasing lower aggregate capacity or the `ProcessHost` route.
#[must_use = "retry the exact lower release before releasing the route"]
pub(super) enum ResidentProcessA2aGetTaskReleaseFailure {
    Selection(ResidentProcessRetainedA2aGetTask),
    Lower {
        route: ResidentProcessA2aTaskRouteCustody,
        failure: ResidentA2aGetTaskReleaseFailure,
    },
    Route {
        route: ResidentProcessA2aTaskRouteCustody,
        custody: Box<CompletedA2aGetTaskReadbackCustody>,
    },
}

/// Failure while releasing a closed or locally stopped terminal phase.
#[must_use = "retry the exact terminal release"]
pub(super) enum ResidentProcessA2aGetTaskTerminalReleaseFailure {
    SelectionClosed(ResidentProcessA2aGetTaskClosed),
    SelectionReadStopped(ResidentProcessA2aGetTaskReadStopped),
    Closed {
        route: ResidentProcessA2aTaskRouteCustody,
        reason: crate::resident::a2a_get_task_owner::ResidentA2aGetTaskFinishFailure,
    },
    ReadStopped {
        route: ResidentProcessA2aTaskRouteCustody,
        reason: crate::resident::a2a_get_task_owner::ResidentA2aGetTaskReadStopFinishFailure,
    },
    RouteClosed {
        route: ResidentProcessA2aTaskRouteCustody,
        reason: Box<A2aGetTaskReadbackError>,
    },
    RouteReadStopped(ResidentProcessA2aTaskRouteCustody),
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Acquire the exact binding route, then prepare one durable read request.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_a2a_get_task<'a, Authority, Authorize, CheckpointSign>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: A2aGetTaskPreparationInput<'a>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
        checkpoint: OperationCheckpointInput,
        checkpoint_issued_at: UnixTimeV1,
        checkpoint_schema: &SchemaDigest,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        accumulator_limits: AccumulatorPreparationLimits,
        membership_limits: AccumulatorMembershipLimits,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentProcessPreparedA2aGetTask<'a>, ResidentProcessA2aGetTaskPreparationFailure>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let key = a2a_get_task_key(&input.readback);
        let deployment = self
            .deployment_authority_index(selector)
            .map_err(ResidentProcessA2aGetTaskPreparationFailure::Selection)?;
        if self.deployments[deployment].store() != access.store || key.store != *access.store {
            return Err(ResidentProcessA2aGetTaskPreparationFailure::Selection(
                ResidentProcessHostSelectionError::AccessStoreAssociation,
            ));
        }
        if !selector.matches_operation(input.readback.attempt.operation) {
            return Err(ResidentProcessA2aGetTaskPreparationFailure::Selection(
                ResidentProcessHostSelectionError::HistoricalHandleAuthority,
            ));
        }
        match self.a2a_task_routes.insert(key.clone(), selector.clone()) {
            Ok(()) => {}
            Err(ResidentProcessA2aTaskRouteInsertError::Conflict) => {
                return Err(ResidentProcessA2aGetTaskPreparationFailure::RouteConflict);
            }
            Err(ResidentProcessA2aTaskRouteInsertError::Saturated) => {
                return Err(ResidentProcessA2aGetTaskPreparationFailure::RouteSaturated);
            }
        }
        let prepared = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.prepare_owned_a2a_get_task(
                        input,
                        exchange,
                        clock,
                        authorize,
                        checkpoint,
                        checkpoint_issued_at,
                        checkpoint_schema,
                        checkpoint_authority,
                        accumulator_limits,
                        membership_limits,
                        sign_checkpoint,
                    )
                },
            )
            .expect("the deployment and store were fenced before GetTask preparation");
        match prepared {
            Ok(lower) => Ok(ResidentProcessPreparedA2aGetTask {
                key: Box::new(key),
                selector: Box::new(selector.clone()),
                lower: Box::new(lower),
            }),
            Err(failure @ ResidentA2aGetTaskPreparationFailure::Release { .. }) => {
                Err(ResidentProcessA2aGetTaskPreparationFailure::Release {
                    key: Box::new(key),
                    selector: Box::new(selector.clone()),
                    lower: Box::new(failure),
                })
            }
            Err(ResidentA2aGetTaskPreparationFailure::Closed(reason)) => {
                let removed = self.a2a_task_routes.remove_active(&key, selector);
                debug_assert!(removed);
                Err(ResidentProcessA2aGetTaskPreparationFailure::Closed(reason))
            }
            Err(failure) => {
                let removed = self.a2a_task_routes.remove_active(&key, selector);
                debug_assert!(removed);
                Err(ResidentProcessA2aGetTaskPreparationFailure::Rejected(
                    Box::new(failure),
                ))
            }
        }
    }

    /// Retry only aggregate-offer release after preparation already closed.
    pub(super) fn retry_a2a_get_task_preparation_release<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskPreparationFailure,
    ) -> Result<A2aGetTaskReadbackError, ResidentProcessA2aGetTaskPreparationFailure> {
        let ResidentProcessA2aGetTaskPreparationFailure::Release {
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
            return Err(ResidentProcessA2aGetTaskPreparationFailure::Release {
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
                    service.retry_a2a_get_task_preparation_release(*lower)
                },
            )
            .expect("the active GetTask route was fenced before release retry");
        match retried {
            Ok(reason) => {
                let removed = self.a2a_task_routes.remove_active(&key, &selector);
                debug_assert!(removed);
                Ok(reason)
            }
            Err(lower) => Err(ResidentProcessA2aGetTaskPreparationFailure::Release {
                key,
                selector,
                lower: Box::new(lower),
            }),
        }
    }

    /// Commit and independently anchor the prepared request under its exact route.
    pub(super) fn retain_prepared_a2a_get_task<'a, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        prepared: ResidentProcessPreparedA2aGetTask<'a>,
    ) -> Result<
        ResidentProcessRetainedUnopenedA2aGetTask<'a>,
        ResidentProcessA2aGetTaskRequestCustodyFailure<'a>,
    > {
        let ResidentProcessPreparedA2aGetTask {
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
            let prepared = ResidentProcessPreparedA2aGetTask {
                key: key.clone(),
                selector: selector.clone(),
                lower,
            };
            return Err(ResidentProcessA2aGetTaskRequestCustodyFailure {
                key,
                selector,
                phase: ResidentProcessA2aGetTaskRequestCustodyPhase::Prepared(Box::new(prepared)),
            });
        }
        let retained = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| service.retain_prepared_a2a_get_task(*lower),
            )
            .expect("the active GetTask route was fenced before request custody");
        match retained {
            Ok(lower) => Ok(ResidentProcessRetainedUnopenedA2aGetTask {
                key,
                selector,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aGetTaskRequestCustodyFailure {
                key,
                selector,
                phase: ResidentProcessA2aGetTaskRequestCustodyPhase::Continuing(Box::new(lower)),
            }),
        }
    }

    /// Continue only the exact unresolved database or anchor phase.
    pub(super) fn continue_a2a_get_task_request_custody<'a, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskRequestCustodyFailure<'a>,
    ) -> Result<
        ResidentProcessRetainedUnopenedA2aGetTask<'a>,
        ResidentProcessA2aGetTaskRequestCustodyFailure<'a>,
    > {
        let ResidentProcessA2aGetTaskRequestCustodyFailure {
            key,
            selector,
            phase,
        } = pending;
        let phase_key = match &phase {
            ResidentProcessA2aGetTaskRequestCustodyPhase::Prepared(lower) => lower.lower.key(),
            ResidentProcessA2aGetTaskRequestCustodyPhase::Continuing(lower) => lower.key(),
        };
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || phase_key != key.as_ref()
        {
            return Err(ResidentProcessA2aGetTaskRequestCustodyFailure {
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
                    ResidentProcessA2aGetTaskRequestCustodyPhase::Prepared(lower) => {
                        service.retain_prepared_a2a_get_task(*lower.lower)
                    }
                    ResidentProcessA2aGetTaskRequestCustodyPhase::Continuing(lower) => {
                        service.continue_a2a_get_task_request_custody(*lower)
                    }
                },
            )
            .expect("the active GetTask route was fenced before custody continuation");
        match continued {
            Ok(lower) => Ok(ResidentProcessRetainedUnopenedA2aGetTask {
                key,
                selector,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aGetTaskRequestCustodyFailure {
                key,
                selector,
                phase: ResidentProcessA2aGetTaskRequestCustodyPhase::Continuing(Box::new(lower)),
            }),
        }
    }

    /// Classify recovered physical custody without granting carrier authority.
    pub(super) fn classify_non_dispatchable_replay_a2a_get_task<'a, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskRequestCustodyFailure<'a>,
    ) -> Result<
        ResidentProcessNonDispatchableReplayA2aGetTask,
        ResidentProcessA2aGetTaskRequestCustodyFailure<'a>,
    > {
        let ResidentProcessA2aGetTaskRequestCustodyFailure {
            key,
            selector,
            phase,
        } = pending;
        let ResidentProcessA2aGetTaskRequestCustodyPhase::Continuing(lower) = phase else {
            return Err(ResidentProcessA2aGetTaskRequestCustodyFailure {
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
            return Err(ResidentProcessA2aGetTaskRequestCustodyFailure {
                key,
                selector,
                phase: ResidentProcessA2aGetTaskRequestCustodyPhase::Continuing(lower),
            });
        }
        let classified = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.classify_non_dispatchable_replay_a2a_get_task(*lower)
                },
            )
            .expect("the active GetTask route was fenced before replay classification");
        match classified {
            Ok(lower) => Ok(ResidentProcessNonDispatchableReplayA2aGetTask {
                key,
                selector,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aGetTaskRequestCustodyFailure {
                key,
                selector,
                phase: ResidentProcessA2aGetTaskRequestCustodyPhase::Continuing(Box::new(lower)),
            }),
        }
    }

    /// Release aggregate capacity and the route for non-dispatchable custody.
    pub(super) fn finish_non_dispatchable_replay_a2a_get_task<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessNonDispatchableReplayA2aGetTask,
    ) -> Result<
        CompletedOperationObservationCustody,
        ResidentProcessNonDispatchableReplayA2aGetTaskReleaseFailure,
    > {
        let ResidentProcessNonDispatchableReplayA2aGetTask {
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
                ResidentProcessNonDispatchableReplayA2aGetTaskReleaseFailure::Selection(
                    ResidentProcessNonDispatchableReplayA2aGetTask {
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
                    service.finish_non_dispatchable_replay_a2a_get_task(*lower)
                },
            )
            .expect("the active GetTask route was fenced before aggregate release");
        match finished {
            Ok(custody) => {
                let removed = self.a2a_task_routes.remove_active(&key, &selector);
                debug_assert!(removed);
                Ok(custody)
            }
            Err(lower) => Err(
                ResidentProcessNonDispatchableReplayA2aGetTaskReleaseFailure::Lower {
                    key,
                    selector,
                    lower: Box::new(lower),
                },
            ),
        }
    }

    /// Accept aggregate capacity, then apply fresh authority before carrier start.
    pub(super) fn start_retained_a2a_get_task<'a, Authority, Authorize>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        unopened: ResidentProcessRetainedUnopenedA2aGetTask<'a>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
    ) -> Result<
        ResidentProcessA2aGetTaskDurableStart<'a>,
        ResidentProcessA2aGetTaskDurableStartFailure<'a>,
    >
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let ResidentProcessRetainedUnopenedA2aGetTask {
            key,
            selector,
            lower,
        } = unopened;
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || lower.key() != key.as_ref()
        {
            return Err(ResidentProcessA2aGetTaskDurableStartFailure {
                key,
                selector,
                lower: Box::new(ResidentA2aGetTaskDurableStartFailure::Association {
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
                    service.start_retained_a2a_get_task(*lower, exchange, clock, authorize)
                },
            )
            .expect("the active GetTask route was fenced before carrier release");
        match started {
            Ok(started) => Ok(self.map_durable_a2a_get_task_start(key, selector, started)),
            Err(lower) => Err(ResidentProcessA2aGetTaskDurableStartFailure {
                key,
                selector,
                lower: Box::new(lower),
            }),
        }
    }

    /// Retry only aggregate acceptance before any carrier call.
    pub(super) fn retry_a2a_get_task_aggregate_acceptance<'a, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskDurableStartFailure<'a>,
    ) -> Result<
        ResidentProcessAggregateAcceptedA2aGetTask<'a>,
        ResidentProcessA2aGetTaskDurableStartFailure<'a>,
    > {
        let ResidentProcessA2aGetTaskDurableStartFailure {
            key,
            selector,
            lower,
        } = pending;
        let ResidentA2aGetTaskDurableStartFailure::AggregateAcceptance(lower) = *lower else {
            return Err(ResidentProcessA2aGetTaskDurableStartFailure {
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
            return Err(ResidentProcessA2aGetTaskDurableStartFailure {
                key,
                selector,
                lower: Box::new(ResidentA2aGetTaskDurableStartFailure::AggregateAcceptance(
                    lower,
                )),
            });
        }
        let retried = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retry_a2a_get_task_aggregate_acceptance(lower)
                },
            )
            .expect("the active GetTask route was fenced before acceptance retry");
        match retried {
            Ok(lower) => Ok(ResidentProcessAggregateAcceptedA2aGetTask {
                key,
                selector,
                lower: Box::new(lower),
            }),
            Err(lower) => Err(ResidentProcessA2aGetTaskDurableStartFailure {
                key,
                selector,
                lower: Box::new(ResidentA2aGetTaskDurableStartFailure::AggregateAcceptance(
                    lower,
                )),
            }),
        }
    }

    /// Apply fresh preflight to one aggregate-accepted unopened request.
    pub(super) fn start_aggregate_accepted_a2a_get_task<'a, Authority, Authorize>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        accepted: ResidentProcessAggregateAcceptedA2aGetTask<'a>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
    ) -> ResidentProcessA2aGetTaskDurableStart<'a>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let ResidentProcessAggregateAcceptedA2aGetTask {
            key,
            selector,
            lower,
        } = accepted;
        if self
            .a2a_task_routes
            .require_active(&key, &selector, access.store)
            .is_err()
            || lower.key() != key.as_ref()
        {
            return ResidentProcessA2aGetTaskDurableStart::PreflightRejected(
                ResidentProcessA2aGetTaskPreflightFailure {
                    key,
                    selector,
                    lower: Box::new(ResidentA2aGetTaskPreflightFailure {
                        accepted: *lower,
                        reason: A2aGetTaskReadbackError::Association,
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
                    service
                        .start_aggregate_accepted_a2a_get_task(*lower, exchange, clock, authorize)
                },
            )
            .expect("the active GetTask route was fenced before fresh preflight");
        self.map_durable_a2a_get_task_start(key, selector, started)
    }

    /// Validate one recovered Task, then prepare its unopened read request.
    ///
    /// This method performs no carrier call. The returned request must still
    /// reach database and independent-anchor custody before it can start.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_recovered_a2a_get_task<'a, Authority, Authorize, CheckpointSign>(
        &mut self,
        recovered: &ResidentProcessRecoveredA2aTask,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: A2aGetTaskPreparationInput<'a>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
        checkpoint: OperationCheckpointInput,
        checkpoint_issued_at: UnixTimeV1,
        checkpoint_schema: &SchemaDigest,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        accumulator_limits: AccumulatorPreparationLimits,
        membership_limits: AccumulatorMembershipLimits,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentProcessPreparedA2aGetTask<'a>, ResidentProcessA2aGetTaskPreparationFailure>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if !recovered_a2a_task_matches_input(recovered, selector, access.store, &input.readback) {
            return Err(ResidentProcessA2aGetTaskPreparationFailure::Selection(
                ResidentProcessHostSelectionError::A2aOperationAssociation,
            ));
        }
        self.prepare_a2a_get_task(
            selector,
            access,
            authority,
            input,
            exchange,
            clock,
            authorize,
            checkpoint,
            checkpoint_issued_at,
            checkpoint_schema,
            checkpoint_authority,
            accumulator_limits,
            membership_limits,
            sign_checkpoint,
        )
    }

    /// Classify durable read state after restart without reserving or calling a carrier.
    pub(super) fn recover_a2a_get_task_disposition<Authority>(
        &mut self,
        recovered: &ResidentProcessRecoveredA2aTask,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        maximum_record_bytes: usize,
    ) -> Result<
        ResidentProcessRecoveredA2aGetTaskDisposition,
        ResidentProcessA2aGetTaskRecoveryFailure,
    > {
        let operation = recovered.attempt.start.control.operation().operation();
        if selector != &recovered.attempt.start.selector
            || !matches!(
                recovered.attempt.start.lineage,
                ResidentProcessOperationLineage::PairwiseDelegate(_)
            )
            || !selector.matches_operation(operation)
            || access.store != recovered.task.local_observation().store()
        {
            return Err(ResidentProcessA2aGetTaskRecoveryFailure::Selection(
                ResidentProcessHostSelectionError::A2aOperationAssociation,
            ));
        }
        let disposition = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.recover_owned_a2a_get_task_custody(
                        &recovered.task,
                        operation,
                        maximum_record_bytes,
                    )
                },
            )
            .expect("the recovered deployment and store were fenced before GetTask recovery")
            .map_err(ResidentProcessA2aGetTaskRecoveryFailure::Custody)?;
        Ok(match disposition {
            RecoveredA2aGetTaskCustody::Absent => {
                ResidentProcessRecoveredA2aGetTaskDisposition::Absent
            }
            RecoveredA2aGetTaskCustody::Preparation(recovered) => {
                let _retained_coordinates = recovered.coordinates();
                ResidentProcessRecoveredA2aGetTaskDisposition::CarrierStartUnknown
            }
            RecoveredA2aGetTaskCustody::LocalDisposition(recovered) => {
                let _retained_coordinates = recovered.coordinates();
                ResidentProcessRecoveredA2aGetTaskDisposition::LocalDisposition
            }
            RecoveredA2aGetTaskCustody::PublicObservation(observation) => {
                let _retained_coordinate = observation.coordinate();
                ResidentProcessRecoveredA2aGetTaskDisposition::PublicObservation
            }
        })
    }

    fn map_durable_a2a_get_task_start<'a>(
        &mut self,
        key: Box<ResidentA2aTaskWorkKey>,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        started: ResidentA2aGetTaskDurableStart<'a>,
    ) -> ResidentProcessA2aGetTaskDurableStart<'a> {
        match started {
            ResidentA2aGetTaskDurableStart::Active => ResidentProcessA2aGetTaskDurableStart::Active,
            ResidentA2aGetTaskDurableStart::PreflightRejected(lower) => {
                ResidentProcessA2aGetTaskDurableStart::PreflightRejected(
                    ResidentProcessA2aGetTaskPreflightFailure {
                        key,
                        selector,
                        lower: Box::new(lower),
                    },
                )
            }
            ResidentA2aGetTaskDurableStart::Terminal(lower) => {
                let mapped = match lower {
                    ResidentA2aGetTaskPoll::AwaitingCustody(awaiting) => {
                        let route = self
                            .a2a_task_routes
                            .check_out(&key)
                            .expect("the active GetTask route must remain present");
                        ResidentProcessA2aGetTaskPoll::AwaitingCustody(
                            ResidentProcessA2aGetTaskAwaitingCustody { route, awaiting },
                        )
                    }
                    ResidentA2aGetTaskPoll::Closed(closed) => {
                        let route = self
                            .a2a_task_routes
                            .check_out(&key)
                            .expect("the active GetTask route must remain present");
                        ResidentProcessA2aGetTaskPoll::Closed(ResidentProcessA2aGetTaskClosed {
                            route,
                            closed,
                        })
                    }
                    ResidentA2aGetTaskPoll::ReadStopped(stopped) => {
                        let route = self
                            .a2a_task_routes
                            .check_out(&key)
                            .expect("the active GetTask route must remain present");
                        ResidentProcessA2aGetTaskPoll::ReadStopped(
                            ResidentProcessA2aGetTaskReadStopped { route, stopped },
                        )
                    }
                    ResidentA2aGetTaskPoll::AggregateTransitionRejected(rejection) => {
                        ResidentProcessA2aGetTaskPoll::AggregateTransitionRejected {
                            key,
                            selector,
                            rejection,
                        }
                    }
                    ResidentA2aGetTaskPoll::PendingResponse
                    | ResidentA2aGetTaskPoll::ResponseReceived
                    | ResidentA2aGetTaskPoll::PendingEof
                    | ResidentA2aGetTaskPoll::NotFound => {
                        unreachable!("a closed carrier-start failure must be terminal")
                    }
                };
                ResidentProcessA2aGetTaskDurableStart::Terminal(mapped)
            }
        }
    }

    /// Advance one exact routed read by at most one lower phase.
    pub(super) fn poll_a2a_get_task<Authority>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        key: &ResidentA2aTaskWorkKey,
        cx: &mut Context<'_>,
        clock: &dyn WallClockSourcePort,
    ) -> ResidentProcessA2aGetTaskPoll {
        if let Err(reason) = self
            .a2a_task_routes
            .require_active(key, selector, access.store)
        {
            return ResidentProcessA2aGetTaskPoll::Selection(reason);
        }
        let polled = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| service.poll_owned_a2a_get_task(key, cx, clock),
            )
            .expect("the active GetTask route was fenced before polling");
        match polled {
            ResidentA2aGetTaskPoll::PendingResponse => {
                ResidentProcessA2aGetTaskPoll::PendingResponse
            }
            ResidentA2aGetTaskPoll::ResponseReceived => {
                ResidentProcessA2aGetTaskPoll::ResponseReceived
            }
            ResidentA2aGetTaskPoll::PendingEof => ResidentProcessA2aGetTaskPoll::PendingEof,
            ResidentA2aGetTaskPoll::AwaitingCustody(awaiting) => {
                let route = self
                    .a2a_task_routes
                    .check_out(key)
                    .expect("the active GetTask route must remain present");
                ResidentProcessA2aGetTaskPoll::AwaitingCustody(
                    ResidentProcessA2aGetTaskAwaitingCustody { route, awaiting },
                )
            }
            ResidentA2aGetTaskPoll::Closed(closed) => {
                let route = self
                    .a2a_task_routes
                    .check_out(key)
                    .expect("the active GetTask route must remain present");
                ResidentProcessA2aGetTaskPoll::Closed(ResidentProcessA2aGetTaskClosed {
                    route,
                    closed,
                })
            }
            ResidentA2aGetTaskPoll::ReadStopped(stopped) => {
                let route = self
                    .a2a_task_routes
                    .check_out(key)
                    .expect("the active GetTask route must remain present");
                ResidentProcessA2aGetTaskPoll::ReadStopped(ResidentProcessA2aGetTaskReadStopped {
                    route,
                    stopped,
                })
            }
            ResidentA2aGetTaskPoll::AggregateTransitionRejected(rejection) => {
                ResidentProcessA2aGetTaskPoll::AggregateTransitionRejected {
                    key: Box::new(key.clone()),
                    selector: Box::new(selector.clone()),
                    rejection,
                }
            }
            ResidentA2aGetTaskPoll::NotFound => ResidentProcessA2aGetTaskPoll::NotFound,
        }
    }

    /// Stop one routed local read without creating a remote cancellation.
    pub(super) fn stop_a2a_get_task_response_read<Authority>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        key: &ResidentA2aTaskWorkKey,
    ) -> ResidentProcessA2aGetTaskPoll {
        if let Err(reason) = self
            .a2a_task_routes
            .require_active(key, selector, access.store)
        {
            return ResidentProcessA2aGetTaskPoll::Selection(reason);
        }
        let stopped = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.stop_owned_a2a_get_task_response_read(key)
                },
            )
            .expect("the active GetTask route was fenced before local response-read stop");
        match stopped {
            ResidentA2aGetTaskPoll::ReadStopped(stopped) => {
                let route = self
                    .a2a_task_routes
                    .check_out(key)
                    .expect("the active GetTask route must remain present");
                ResidentProcessA2aGetTaskPoll::ReadStopped(ResidentProcessA2aGetTaskReadStopped {
                    route,
                    stopped,
                })
            }
            ResidentA2aGetTaskPoll::AggregateTransitionRejected(rejection) => {
                ResidentProcessA2aGetTaskPoll::AggregateTransitionRejected {
                    key: Box::new(key.clone()),
                    selector: Box::new(selector.clone()),
                    rejection,
                }
            }
            ResidentA2aGetTaskPoll::NotFound => ResidentProcessA2aGetTaskPoll::NotFound,
            _ => unreachable!("a local read stop has only terminal or rejected phases"),
        }
    }

    /// Retry a rejected aggregate terminal transition without carrier I/O.
    pub(super) fn retry_a2a_get_task_aggregate_transition<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        key: ResidentA2aTaskWorkKey,
        selector: ResidentDeploymentAuthoritySelector,
        rejection: ResidentA2aGetTaskAggregateTransitionRejection,
    ) -> ResidentProcessA2aGetTaskPoll {
        if key.store != *access.store
            || !self.a2a_task_routes.matches(&key, &selector)
            || rejection.key() != &key
        {
            return ResidentProcessA2aGetTaskPoll::AggregateTransitionRejected {
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
                    service.retry_owned_a2a_get_task_aggregate_transition(rejection)
                },
            )
            .expect("the active GetTask route was fenced before settlement retry");
        match retried {
            ResidentA2aGetTaskPoll::AwaitingCustody(awaiting) => {
                let route = self
                    .a2a_task_routes
                    .check_out(&key)
                    .expect("the active GetTask route must remain present");
                ResidentProcessA2aGetTaskPoll::AwaitingCustody(
                    ResidentProcessA2aGetTaskAwaitingCustody { route, awaiting },
                )
            }
            ResidentA2aGetTaskPoll::Closed(closed) => {
                let route = self
                    .a2a_task_routes
                    .check_out(&key)
                    .expect("the active GetTask route must remain present");
                ResidentProcessA2aGetTaskPoll::Closed(ResidentProcessA2aGetTaskClosed {
                    route,
                    closed,
                })
            }
            ResidentA2aGetTaskPoll::ReadStopped(stopped) => {
                let route = self
                    .a2a_task_routes
                    .check_out(&key)
                    .expect("the active GetTask route must remain present");
                ResidentProcessA2aGetTaskPoll::ReadStopped(ResidentProcessA2aGetTaskReadStopped {
                    route,
                    stopped,
                })
            }
            ResidentA2aGetTaskPoll::AggregateTransitionRejected(rejection) => {
                ResidentProcessA2aGetTaskPoll::AggregateTransitionRejected {
                    key: Box::new(key),
                    selector: Box::new(selector),
                    rejection,
                }
            }
            _ => unreachable!("a settlement retry cannot return an active poll phase"),
        }
    }

    /// Retain one closed carrier disposition on its exact route.
    pub(super) fn retain_closed_a2a_get_task_local_disposition<Authority, CheckpointSign>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskClosed,
        input: ResidentA2aGetTaskLocalDispositionInput<'_>,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessRetainedA2aGetTaskLocalDisposition,
        ResidentProcessA2aGetTaskLocalDispositionCustodyFailure,
    >
    where
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentProcessA2aGetTaskClosed { route, closed } = pending;
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.as_ref() != &closed.key
            || route.key.store != *access.store
        {
            return Err(
                ResidentProcessA2aGetTaskLocalDispositionCustodyFailure::SelectionClosed(
                    ResidentProcessA2aGetTaskClosed { route, closed },
                ),
            );
        }
        let selector = route.selector.as_ref().clone();
        let retained = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retain_closed_a2a_get_task_local_disposition(
                        *closed,
                        input,
                        sign_checkpoint,
                    )
                },
            )
            .expect("the checked-out GetTask route was fenced before local custody");
        match retained {
            Ok(retained) => {
                Ok(ResidentProcessRetainedA2aGetTaskLocalDisposition { route, retained })
            }
            Err(failure) => Err(
                ResidentProcessA2aGetTaskLocalDispositionCustodyFailure::Lower {
                    route,
                    failure: Box::new(failure),
                },
            ),
        }
    }

    /// Retain one explicit local read stop on its exact route.
    pub(super) fn retain_stopped_a2a_get_task_local_disposition<Authority, CheckpointSign>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskReadStopped,
        input: ResidentA2aGetTaskLocalDispositionInput<'_>,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessRetainedA2aGetTaskLocalDisposition,
        ResidentProcessA2aGetTaskLocalDispositionCustodyFailure,
    >
    where
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentProcessA2aGetTaskReadStopped { route, stopped } = pending;
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.as_ref() != &stopped.key
            || route.key.store != *access.store
        {
            return Err(
                ResidentProcessA2aGetTaskLocalDispositionCustodyFailure::SelectionReadStopped(
                    ResidentProcessA2aGetTaskReadStopped { route, stopped },
                ),
            );
        }
        let selector = route.selector.as_ref().clone();
        let retained = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retain_stopped_a2a_get_task_local_disposition(
                        *stopped,
                        input,
                        sign_checkpoint,
                    )
                },
            )
            .expect("the checked-out GetTask route was fenced before local custody");
        match retained {
            Ok(retained) => {
                Ok(ResidentProcessRetainedA2aGetTaskLocalDisposition { route, retained })
            }
            Err(failure) => Err(
                ResidentProcessA2aGetTaskLocalDispositionCustodyFailure::Lower {
                    route,
                    failure: Box::new(failure),
                },
            ),
        }
    }

    /// Continue only database or anchor custody for one routed local disposition.
    pub(super) fn continue_a2a_get_task_local_disposition_custody<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskLocalDispositionCustodyFailure,
    ) -> Result<
        ResidentProcessRetainedA2aGetTaskLocalDisposition,
        ResidentProcessA2aGetTaskLocalDispositionCustodyFailure,
    > {
        let ResidentProcessA2aGetTaskLocalDispositionCustodyFailure::Lower { route, failure } =
            pending
        else {
            return Err(pending);
        };
        if !self.a2a_task_routes.owns_custody(&route)
            || failure.key() != route.key.as_ref()
            || route.key.store != *access.store
        {
            return Err(
                ResidentProcessA2aGetTaskLocalDispositionCustodyFailure::Lower { route, failure },
            );
        }
        let selector = route.selector.as_ref().clone();
        let retained = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.continue_a2a_get_task_local_disposition_custody(*failure)
                },
            )
            .expect("the checked-out GetTask route was fenced before custody continuation");
        match retained {
            Ok(retained) => {
                Ok(ResidentProcessRetainedA2aGetTaskLocalDisposition { route, retained })
            }
            Err(failure) => Err(
                ResidentProcessA2aGetTaskLocalDispositionCustodyFailure::Lower {
                    route,
                    failure: Box::new(failure),
                },
            ),
        }
    }

    /// Release aggregate capacity, then the route, after local custody exists.
    pub(super) fn finish_a2a_get_task_local_disposition<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessRetainedA2aGetTaskLocalDisposition,
    ) -> Result<
        ResidentProcessReleasedA2aGetTaskLocalDisposition,
        ResidentProcessA2aGetTaskLocalDispositionReleaseFailure,
    > {
        let ResidentProcessRetainedA2aGetTaskLocalDisposition { route, retained } = pending;
        if !self.a2a_task_routes.owns_custody(&route)
            || retained.key() != route.key.as_ref()
            || route.key.store != *access.store
        {
            return Err(
                ResidentProcessA2aGetTaskLocalDispositionReleaseFailure::Selection(
                    ResidentProcessRetainedA2aGetTaskLocalDisposition { route, retained },
                ),
            );
        }
        let selector = route.selector.as_ref().clone();
        let released = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.finish_a2a_get_task_local_disposition(retained)
                },
            )
            .expect("the checked-out GetTask route was fenced before local release");
        let released = match released {
            Ok(ResidentReleasedA2aGetTaskLocalDisposition::Closed { reason, custody }) => {
                ResidentProcessReleasedA2aGetTaskLocalDisposition::Closed { reason, custody }
            }
            Ok(ResidentReleasedA2aGetTaskLocalDisposition::ReadStopped { custody }) => {
                ResidentProcessReleasedA2aGetTaskLocalDisposition::ReadStopped { custody }
            }
            Err(failure) => {
                return Err(
                    ResidentProcessA2aGetTaskLocalDispositionReleaseFailure::Lower {
                        route,
                        failure,
                    },
                );
            }
        };
        match self.a2a_task_routes.release(route) {
            Ok(()) => Ok(released),
            Err(route) => Err(
                ResidentProcessA2aGetTaskLocalDispositionReleaseFailure::Route {
                    route,
                    released: Box::new(released),
                },
            ),
        }
    }

    /// Retry only aggregate or route release after local custody completed.
    pub(super) fn retry_a2a_get_task_local_disposition_release<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskLocalDispositionReleaseFailure,
    ) -> Result<
        ResidentProcessReleasedA2aGetTaskLocalDisposition,
        ResidentProcessA2aGetTaskLocalDispositionReleaseFailure,
    > {
        match pending {
            ResidentProcessA2aGetTaskLocalDispositionReleaseFailure::Selection(pending) => {
                self.finish_a2a_get_task_local_disposition(access, authority, pending)
            }
            ResidentProcessA2aGetTaskLocalDispositionReleaseFailure::Lower { route, failure } => {
                if !self.a2a_task_routes.owns_custody(&route)
                    || failure.key() != route.key.as_ref()
                    || route.key.store != *access.store
                {
                    return Err(
                        ResidentProcessA2aGetTaskLocalDispositionReleaseFailure::Lower {
                            route,
                            failure,
                        },
                    );
                }
                let selector = route.selector.as_ref().clone();
                let released = self
                    .with_selected_operation_service(
                        &selector,
                        access,
                        authority,
                        |_application, service, _sessions| {
                            service.retry_a2a_get_task_local_disposition_release(failure)
                        },
                    )
                    .expect("the checked-out GetTask route was fenced before release retry");
                let released = match released {
                    Ok(ResidentReleasedA2aGetTaskLocalDisposition::Closed { reason, custody }) => {
                        ResidentProcessReleasedA2aGetTaskLocalDisposition::Closed {
                            reason,
                            custody,
                        }
                    }
                    Ok(ResidentReleasedA2aGetTaskLocalDisposition::ReadStopped { custody }) => {
                        ResidentProcessReleasedA2aGetTaskLocalDisposition::ReadStopped { custody }
                    }
                    Err(failure) => {
                        return Err(
                            ResidentProcessA2aGetTaskLocalDispositionReleaseFailure::Lower {
                                route,
                                failure,
                            },
                        );
                    }
                };
                match self.a2a_task_routes.release(route) {
                    Ok(()) => Ok(released),
                    Err(route) => Err(
                        ResidentProcessA2aGetTaskLocalDispositionReleaseFailure::Route {
                            route,
                            released: Box::new(released),
                        },
                    ),
                }
            }
            ResidentProcessA2aGetTaskLocalDispositionReleaseFailure::Route { route, released } => {
                if !self.a2a_task_routes.owns_custody(&route) || route.key.store != *access.store {
                    return Err(
                        ResidentProcessA2aGetTaskLocalDispositionReleaseFailure::Route {
                            route,
                            released,
                        },
                    );
                }
                match self.a2a_task_routes.release(route) {
                    Ok(()) => Ok(*released),
                    Err(route) => Err(
                        ResidentProcessA2aGetTaskLocalDispositionReleaseFailure::Route {
                            route,
                            released,
                        },
                    ),
                }
            }
        }
    }

    /// Retain a completed response under its exact deployment route.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn retain_a2a_get_task<Authority, Rotation, ObservationSign, Store, CheckpointSign>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskAwaitingCustody,
        input: A2aGetTaskCommitInput<'_>,
        protocol: &dyn A2aGetTaskEvidencePort,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        resolve_store: Store,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentProcessRetainedA2aGetTask, ResidentProcessA2aGetTaskRetentionFailure>
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
        let ResidentProcessA2aGetTaskAwaitingCustody { route, awaiting } = pending;
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.as_ref() != &awaiting.key
            || route.key.as_ref() != &a2a_get_task_key_from_commit(&input)
            || route.key.store != *access.store
        {
            return Err(ResidentProcessA2aGetTaskRetentionFailure::Selection(
                ResidentProcessA2aGetTaskAwaitingCustody { route, awaiting },
            ));
        }
        let selector = route.selector.as_ref().clone();
        let retained = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retain_owned_a2a_get_task(
                        *awaiting,
                        input,
                        protocol,
                        admit_rotation,
                        sign_observation,
                        resolve_store,
                        sign_checkpoint,
                    )
                },
            )
            .expect("the checked-out GetTask route was fenced before custody");
        match retained {
            Ok(retained) => Ok(ResidentProcessRetainedA2aGetTask {
                route,
                retained: Box::new(retained),
            }),
            Err(failure) => {
                Err(ResidentProcessA2aGetTaskRetentionFailure::Lower { route, failure })
            }
        }
    }

    /// Retry only pre-physical retention with corrected fresh inputs.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn retry_a2a_get_task_retention<
        Authority,
        Rotation,
        ObservationSign,
        Store,
        CheckpointSign,
    >(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskRetentionFailure,
        input: A2aGetTaskCommitInput<'_>,
        protocol: &dyn A2aGetTaskEvidencePort,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        resolve_store: Store,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentProcessRetainedA2aGetTask, ResidentProcessA2aGetTaskRetentionFailure>
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
        let ResidentProcessA2aGetTaskRetentionFailure::Lower { route, failure } = pending else {
            return Err(pending);
        };
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.as_ref() != failure.key()
            || route.key.as_ref() != &a2a_get_task_key_from_commit(&input)
            || route.key.store != *access.store
        {
            return Err(ResidentProcessA2aGetTaskRetentionFailure::Lower { route, failure });
        }
        let selector = route.selector.as_ref().clone();
        let retried = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retry_owned_a2a_get_task_retention(
                        failure,
                        input,
                        protocol,
                        admit_rotation,
                        sign_observation,
                        resolve_store,
                        sign_checkpoint,
                    )
                },
            )
            .expect("the checked-out GetTask route was fenced before retention retry");
        match retried {
            Ok(retained) => Ok(ResidentProcessRetainedA2aGetTask {
                route,
                retained: Box::new(retained),
            }),
            Err(failure) => {
                Err(ResidentProcessA2aGetTaskRetentionFailure::Lower { route, failure })
            }
        }
    }

    /// Continue only unresolved physical custody for the exact routed binding.
    pub(super) fn continue_a2a_get_task_custody<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskRetentionFailure,
    ) -> Result<ResidentProcessRetainedA2aGetTask, ResidentProcessA2aGetTaskRetentionFailure> {
        let ResidentProcessA2aGetTaskRetentionFailure::Lower { route, failure } = pending else {
            return Err(pending);
        };
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.as_ref() != failure.key()
            || route.key.store != *access.store
        {
            return Err(ResidentProcessA2aGetTaskRetentionFailure::Lower { route, failure });
        }
        let selector = route.selector.as_ref().clone();
        let continued = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.continue_owned_a2a_get_task_custody(failure)
                },
            )
            .expect("the checked-out GetTask route was fenced before continuation");
        match continued {
            Ok(retained) => Ok(ResidentProcessRetainedA2aGetTask {
                route,
                retained: Box::new(retained),
            }),
            Err(failure) => {
                Err(ResidentProcessA2aGetTaskRetentionFailure::Lower { route, failure })
            }
        }
    }

    /// Release process capacity after exact remote-history custody.
    pub(super) fn finish_a2a_get_task<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessRetainedA2aGetTask,
    ) -> Result<CompletedA2aGetTaskReadbackCustody, ResidentProcessA2aGetTaskReleaseFailure> {
        let ResidentProcessRetainedA2aGetTask { route, retained } = pending;
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.as_ref() != retained.key()
            || route.key.store != *access.store
        {
            return Err(ResidentProcessA2aGetTaskReleaseFailure::Selection(
                ResidentProcessRetainedA2aGetTask { route, retained },
            ));
        }
        let selector = route.selector.as_ref().clone();
        let finished = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| service.finish_owned_a2a_get_task(*retained),
            )
            .expect("the checked-out GetTask route was fenced before release");
        match finished {
            Ok(custody)
                if route.key.matches_binding(custody.binding())
                    && route.key.store == *custody.custody().candidate().store() =>
            {
                match self.a2a_task_routes.release(route) {
                    Ok(()) => Ok(custody),
                    Err(route) => Err(ResidentProcessA2aGetTaskReleaseFailure::Route {
                        route,
                        custody: Box::new(custody),
                    }),
                }
            }
            Ok(custody) => Err(ResidentProcessA2aGetTaskReleaseFailure::Route {
                route,
                custody: Box::new(custody),
            }),
            Err(failure) => Err(ResidentProcessA2aGetTaskReleaseFailure::Lower { route, failure }),
        }
    }

    /// Retry only the exact pending aggregate or process-route release.
    pub(super) fn retry_a2a_get_task_release<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskReleaseFailure,
    ) -> Result<CompletedA2aGetTaskReadbackCustody, ResidentProcessA2aGetTaskReleaseFailure> {
        match pending {
            ResidentProcessA2aGetTaskReleaseFailure::Route { route, custody } => {
                if !self.a2a_task_routes.owns_custody(&route)
                    || !route.key.matches_binding(custody.binding())
                    || route.key.store != *custody.custody().candidate().store()
                    || route.key.store != *access.store
                {
                    return Err(ResidentProcessA2aGetTaskReleaseFailure::Route { route, custody });
                }
                match self.a2a_task_routes.release(route) {
                    Ok(()) => Ok(*custody),
                    Err(route) => {
                        Err(ResidentProcessA2aGetTaskReleaseFailure::Route { route, custody })
                    }
                }
            }
            ResidentProcessA2aGetTaskReleaseFailure::Selection(pending) => {
                Err(ResidentProcessA2aGetTaskReleaseFailure::Selection(pending))
            }
            ResidentProcessA2aGetTaskReleaseFailure::Lower { route, failure } => {
                if !self.a2a_task_routes.owns_custody(&route)
                    || route.key.as_ref() != failure.key()
                    || route.key.store != *access.store
                {
                    return Err(ResidentProcessA2aGetTaskReleaseFailure::Lower { route, failure });
                }
                let selector = route.selector.as_ref().clone();
                let retried = self
                    .with_selected_operation_service(
                        &selector,
                        access,
                        authority,
                        |_application, service, _sessions| {
                            service.retry_owned_a2a_get_task_release(failure)
                        },
                    )
                    .expect("the checked-out GetTask route was fenced before release retry");
                match retried {
                    Ok(custody)
                        if route.key.matches_binding(custody.binding())
                            && route.key.store == *custody.custody().candidate().store() =>
                    {
                        match self.a2a_task_routes.release(route) {
                            Ok(()) => Ok(custody),
                            Err(route) => Err(ResidentProcessA2aGetTaskReleaseFailure::Route {
                                route,
                                custody: Box::new(custody),
                            }),
                        }
                    }
                    Ok(custody) => Err(ResidentProcessA2aGetTaskReleaseFailure::Route {
                        route,
                        custody: Box::new(custody),
                    }),
                    Err(failure) => {
                        Err(ResidentProcessA2aGetTaskReleaseFailure::Lower { route, failure })
                    }
                }
            }
        }
    }
}

/// Match one caller-supplied read to the exact Task proved during restart.
///
/// This comparison runs before route reservation, authorization, or carrier
/// work. The lower owner still revalidates the complete attempt and history.
fn recovered_a2a_task_matches_input(
    recovered: &ResidentProcessRecoveredA2aTask,
    selector: &ResidentDeploymentAuthoritySelector,
    store: &pale_ports::AuthorityStoreCoordinate,
    input: &A2aGetTaskReadbackInput<'_>,
) -> bool {
    let expected_operation = recovered.attempt.start.control.operation().operation();
    let expected_history = &recovered.task;
    let supplied_history = input.history;
    let expected_local_head = expected_history.local_observation().selected_head();
    let supplied_local_head = supplied_history.local_observation().selected_head();

    recovered.attempt.dispatch == ResidentProcessAttemptObservation::CurrentHead
        && matches!(
            recovered.attempt.start.lineage,
            ResidentProcessOperationLineage::PairwiseDelegate(_)
        )
        && selector == &recovered.attempt.start.selector
        && selector.matches_operation(expected_operation)
        && store == expected_history.local_observation().store()
        && store == supplied_history.local_observation().store()
        && input.attempt.operation.handle().0 == expected_operation.handle().0
        && input
            .attempt
            .operation
            .handle()
            .1
            .canonical_operation_handle_sha256()
            == expected_operation
                .handle()
                .1
                .canonical_operation_handle_sha256()
        && input
            .attempt
            .generation
            .record()
            .compiled_generation_digest()
            == &recovered.attempt.start.generation_digest
        && input
            .attempt
            .dispatched
            .canonical_operation_observation_sha256()
            == expected_history
                .dispatch()
                .1
                .canonical_operation_observation_sha256()
        && input.attempt.selection.coordinate() == recovered.attempt.selection.coordinate()
        && same_recovered_a2a_task_history(expected_history, supplied_history)
        && expected_local_head.0 == supplied_local_head.0
        && expected_local_head
            .1
            .canonical_operation_observation_sha256()
            == supplied_local_head
                .1
                .canonical_operation_observation_sha256()
}

/// Compare every remote record selected by the admitted Task history.
fn same_recovered_a2a_task_history(
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

fn a2a_get_task_key_from_commit(input: &A2aGetTaskCommitInput<'_>) -> ResidentA2aTaskWorkKey {
    let binding = input.observation.history.binding();
    ResidentA2aTaskWorkKey {
        store: input
            .observation
            .history
            .local_observation()
            .store()
            .clone(),
        operation_handle_ref: binding.1.operation_handle_ref().clone(),
        operation_handle_sha256: binding.1.operation_handle_sha256().clone(),
        task_binding_id: binding.1.task_binding_id().clone(),
        task_binding_ref: binding.0.clone(),
        task_binding_sha256: binding.1.canonical_a2a_task_binding_sha256().clone(),
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/a2a_get_task_support.rs"]
mod test_support;
