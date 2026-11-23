//! Deployment routing for initial A2A `SendMessage` delegation.
//!
//! The lower task service owns send custody, carrier progress, durable outcome
//! retention, and shared occupancy. This module only keeps each live send on
//! the deployment that admitted it.

use std::task::Context;

use pale_core::records::{
    A2aJsonRpcRequestIdV1, A2aRouteAdmissionV1, ArtifactDigest, CanonicalSignatureV1,
    ControlCheckpointSigningAuthorityAdmissionContextV1, ControlStateCheckpointDigest,
    ControlStateCheckpointPreimageV1, ImmutableRecordReferenceV1, InvalidResidentRecordV1,
    OperationClassV1, ResidentRecordRefV1, SchemaDigest,
};
use pale_ports::{
    A2aExchangeLimits, A2aExchangePort, A2aSendMessageEvidencePort, AccumulatorMembershipLimits,
    AccumulatorPreparationLimits, OperationCustodyIndexPredecessor, WallClockSourcePort,
};

use super::{
    ResidentApplication, ResidentDeploymentAuthoritySelector, ResidentDeploymentOperationAccess,
    ResidentProcessHost, ResidentProcessHostBuildError, ResidentProcessHostSelectionError,
    ResidentProcessRecoveredAttempt,
};
use crate::resident::a2a_delegate_task_service::{
    ResidentA2aDelegateCompletion, ResidentA2aDelegateCustodyContinuationFailure,
    ResidentA2aDelegateDiagnostic, ResidentA2aDelegateDiagnosticRecoveryFailure,
    ResidentA2aDelegateFinalization, ResidentA2aDelegateFinalizationRejection,
    ResidentA2aDelegateLaunch, ResidentA2aDelegateLaunchFailure, ResidentA2aDelegatePoll,
    ResidentA2aDelegateRecoveredOfferRejection, ResidentA2aDelegateRecoveryFailure,
    ResidentA2aDelegateSettlementRejection, ResidentA2aDelegateUnopenedRetryFailure,
    ResidentA2aDelegateUnresolvedRecoveryFailure,
};
use crate::resident::operation_dispatch::{
    retain_provider_operation_dispatch, A2aDelegateLaunchInput, A2aNoTaskOutcomeRetention,
    A2aNotStartedOutcomeRetention, A2aOutcomeRetentionInput, A2aOutcomeRetentionKind,
    A2aRecoveredUnknownOutcomeRetention, A2aSendDispatch, A2aSendInput, A2aSendRecoveryAdmission,
    A2aSendRecoveryError, A2aSendRecoveryInput, A2aTaskOutcomeRetention,
    A2aUnknownOutcomeRetention, ContinuableA2aOutcomeRetentionFailure, OperationCheckpointInput,
    ProviderOperationDispatchInput, ProviderOperationRetentionFailure, RetainedA2aOutcome,
    RetainedProviderOperationDispatch,
};
use crate::resident::process_host::operation_resolution::{
    ResidentProcessOperationResolutionFailure, ResidentProcessOperationResolutionInput,
};
use crate::resident::task_owner::{ResidentOperationTaskKey, ResidentOperationTaskOwnerInstance};
use crate::resident::task_reservation::ResidentAggregateTaskLimits;
use crate::resident::{
    A2aRouteAssociation, AmbientSessionAdmissionContext, AmbientSessionAuthorityPort,
    OperationObservationAuthorityStoreResolver,
};

/// One bounded route slot before the lower send owner receives work.
pub(super) struct ResidentProcessA2aDelegateRouteReservation {
    owner_instance: ResidentOperationTaskOwnerInstance,
    selector: ResidentDeploymentAuthoritySelector,
}

/// Exact process route retained while the lower send owner has custody.
struct ResidentProcessA2aDelegateRoute {
    key: ResidentOperationTaskKey,
    selector: ResidentDeploymentAuthoritySelector,
}

/// Linear deployment route held through durable terminal handling.
#[must_use = "release only after the exact A2A outcome and aggregate capacity are settled"]
pub(super) struct ResidentProcessA2aDelegateRouteCustody {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: ResidentOperationTaskKey,
    selector: ResidentDeploymentAuthoritySelector,
}

/// Bounded join from one live A2A send to its admitted deployment.
///
/// This table does not count process occupancy. The shared aggregate task
/// reservation ledger remains the only occupancy owner. Its vector reserves
/// complete route capacity at host construction, so accepted route tracking
/// does not grow that vector or reserve another task-capacity slot. This is not
/// a general allocator guarantee for the lower send and custody work.
pub(super) struct ResidentProcessA2aDelegateRoutes {
    owner_instance: ResidentOperationTaskOwnerInstance,
    maximum_outstanding: usize,
    reserved: usize,
    checked_out: usize,
    active: Vec<ResidentProcessA2aDelegateRoute>,
}

/// A route commit refused without hiding the reservation disposition.
pub(super) enum ResidentProcessA2aDelegateRouteCommitFailure {
    /// The reservation belongs to another process operation-task owner.
    ForeignReservation(Box<ResidentProcessA2aDelegateRouteReservation>),
    /// The task key already has an active process-local deployment route.
    DuplicateActiveKey,
}

impl ResidentProcessA2aDelegateRoutes {
    /// Return route slots reserved before a task key exists.
    pub(super) fn reserved_count(&self) -> usize {
        self.reserved
    }

    /// Return active routes temporarily checked out for settlement.
    pub(super) fn checked_out_count(&self) -> usize {
        self.checked_out
    }

    /// Return active task-to-deployment routes.
    pub(super) fn active_count(&self) -> usize {
        self.active.len()
    }

    pub(super) fn new(
        owner_instance: ResidentOperationTaskOwnerInstance,
        limits: ResidentAggregateTaskLimits,
    ) -> Result<Self, ResidentProcessHostBuildError> {
        let maximum_outstanding = limits.maximum_outstanding().get();
        let mut active = Vec::new();
        active
            .try_reserve_exact(maximum_outstanding)
            .map_err(|_| ResidentProcessHostBuildError::A2aDelegateRouteAllocation)?;
        Ok(Self {
            owner_instance,
            maximum_outstanding,
            reserved: 0,
            checked_out: 0,
            active,
        })
    }

    pub(super) fn reserve(
        &mut self,
        selector: ResidentDeploymentAuthoritySelector,
    ) -> Result<ResidentProcessA2aDelegateRouteReservation, ()> {
        if self.active.len() + self.reserved + self.checked_out == self.maximum_outstanding {
            return Err(());
        }
        self.reserved += 1;
        Ok(ResidentProcessA2aDelegateRouteReservation {
            owner_instance: self.owner_instance.clone(),
            selector,
        })
    }

    pub(super) fn commit(
        &mut self,
        reservation: ResidentProcessA2aDelegateRouteReservation,
        key: ResidentOperationTaskKey,
    ) -> Result<(), ResidentProcessA2aDelegateRouteCommitFailure> {
        if !reservation.owner_instance.is_same(&self.owner_instance) {
            return Err(
                ResidentProcessA2aDelegateRouteCommitFailure::ForeignReservation(Box::new(
                    reservation,
                )),
            );
        }
        if self.active.iter().any(|route| route.key == key) {
            debug_assert!(self.reserved > 0);
            self.reserved -= 1;
            return Err(ResidentProcessA2aDelegateRouteCommitFailure::DuplicateActiveKey);
        }
        debug_assert!(self.reserved > 0);
        self.reserved -= 1;
        self.active.push(ResidentProcessA2aDelegateRoute {
            key,
            selector: reservation.selector,
        });
        Ok(())
    }

    pub(super) fn release_reservation(
        &mut self,
        reservation: ResidentProcessA2aDelegateRouteReservation,
    ) -> Result<(), Box<ResidentProcessA2aDelegateRouteReservation>> {
        if !reservation.owner_instance.is_same(&self.owner_instance) || self.reserved == 0 {
            return Err(Box::new(reservation));
        }
        self.reserved -= 1;
        Ok(())
    }

    pub(super) fn selector(
        &self,
        key: &ResidentOperationTaskKey,
    ) -> Option<&ResidentDeploymentAuthoritySelector> {
        self.active
            .iter()
            .find(|route| &route.key == key)
            .map(|route| &route.selector)
    }

    fn remove_unstarted(
        &mut self,
        key: &ResidentOperationTaskKey,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> bool {
        let Some(index) = self
            .active
            .iter()
            .position(|route| &route.key == key && &route.selector == selector)
        else {
            return false;
        };
        self.active.remove(index);
        true
    }

    /// Relinquish only a process-local route whose lower task owner is absent.
    ///
    /// Durable send evidence and aggregate accounting remain unchanged so the
    /// existing restart recovery owner can reconstruct the unresolved send.
    pub(in crate::resident) fn relinquish_unrouted_to_recovery(
        &mut self,
        key: &ResidentOperationTaskKey,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> bool {
        let Some(index) = self
            .active
            .iter()
            .position(|route| &route.key == key && &route.selector == selector)
        else {
            return false;
        };
        self.active.remove(index);
        true
    }

    pub(super) fn check_out(
        &mut self,
        key: &ResidentOperationTaskKey,
    ) -> Option<ResidentProcessA2aDelegateRouteCustody> {
        let index = self.active.iter().position(|route| &route.key == key)?;
        let route = self.active.remove(index);
        self.checked_out += 1;
        Some(ResidentProcessA2aDelegateRouteCustody {
            owner_instance: self.owner_instance.clone(),
            key: route.key,
            selector: route.selector,
        })
    }

    /// Check out only the route owned by the exact deployment selector.
    pub(super) fn check_out_for(
        &mut self,
        key: &ResidentOperationTaskKey,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Option<ResidentProcessA2aDelegateRouteCustody> {
        let index = self
            .active
            .iter()
            .position(|route| &route.key == key && &route.selector == selector)?;
        let route = self.active.remove(index);
        self.checked_out += 1;
        Some(ResidentProcessA2aDelegateRouteCustody {
            owner_instance: self.owner_instance.clone(),
            key: route.key,
            selector: route.selector,
        })
    }

    pub(super) fn release(
        &mut self,
        route: ResidentProcessA2aDelegateRouteCustody,
    ) -> Result<(), Box<ResidentProcessA2aDelegateRouteCustody>> {
        if !route.owner_instance.is_same(&self.owner_instance) || self.checked_out == 0 {
            return Err(Box::new(route));
        }
        self.checked_out -= 1;
        Ok(())
    }
}

/// A launch that is either still active or ready for durable terminal handling.
#[must_use = "retain the exact routed A2A launch state"]
pub(super) enum ResidentProcessA2aDelegateLaunch {
    /// The carrier exchange remains under the lower task owner.
    Awaiting(ResidentOperationTaskKey),
    /// One typed terminal response holds its deployment route.
    Completed(ResidentProcessA2aDelegateCompletion),
    /// Diagnostic construction stopped after the send became non-replayable.
    Diagnostic(ResidentProcessA2aDelegateDiagnostic),
}

/// One terminal A2A response joined to its exact deployment route.
#[must_use = "retain the outcome durably before releasing its deployment route"]
pub(super) struct ResidentProcessA2aDelegateCompletion {
    pub(super) route: ResidentProcessA2aDelegateRouteCustody,
    pub(super) completion: ResidentA2aDelegateCompletion,
}

/// One terminal diagnostic joined to its exact deployment route.
#[must_use = "recover the diagnostic before releasing its deployment route"]
pub(super) struct ResidentProcessA2aDelegateDiagnostic {
    route: ResidentProcessA2aDelegateRouteCustody,
    diagnostic: ResidentA2aDelegateDiagnostic,
}

/// Restart-recovered uncertainty joined to the original deployment route.
#[must_use = "retain recovered uncertainty before releasing its deployment route"]
pub(super) struct ResidentProcessRecoveredA2aDelegate<'a> {
    pub(super) route: ResidentProcessA2aDelegateRouteCustody,
    pub(super) recovered:
        crate::resident::a2a_delegate_task_service::ResidentRecoveredA2aDelegate<'a>,
}

/// A launch failure that preserves every possibly live lower phase.
#[must_use = "do not infer that a routed failure can be replayed"]
pub(super) enum ResidentProcessA2aDelegateLaunchFailure<'operation, 'call> {
    /// A process, deployment, operation, session, route, or store fence failed.
    Selection {
        input: Box<A2aDelegateLaunchInput<'operation, 'call>>,
        reason: ResidentProcessHostSelectionError,
    },
    /// No preallocated process route remained. No lower send work ran.
    RouteSaturated(Box<A2aDelegateLaunchInput<'operation, 'call>>),
    /// This dispatch already has an active process route. No carrier work ran.
    RouteConflict(Box<A2aDelegateLaunchInput<'operation, 'call>>),
    /// The lower owner proved that the carrier never started.
    Rejected(Box<ResidentA2aDelegateLaunchFailure<'operation, 'call>>),
    /// Releasing capacity after proof that the carrier never started needs a retry.
    Rollback {
        key: ResidentOperationTaskKey,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        failure: Box<ResidentA2aDelegateLaunchFailure<'operation, 'call>>,
    },
    /// Request custody is unresolved. Carrier replay is not permitted.
    Custody {
        key: ResidentOperationTaskKey,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        failure: Box<ResidentA2aDelegateLaunchFailure<'operation, 'call>>,
    },
    /// Anchored request custody exists and the carrier is proved unopened.
    Unopened {
        key: ResidentOperationTaskKey,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        failure: Box<ResidentA2aDelegateLaunchFailure<'operation, 'call>>,
    },
    /// A closed carrier exchange failed its returned-key join.
    StartedAssociation {
        key: ResidentOperationTaskKey,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        failure: Box<ResidentA2aDelegateLaunchFailure<'operation, 'call>>,
    },
    /// A closed branch could not move its capacity into terminal handling.
    OfferSettlement {
        key: ResidentOperationTaskKey,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        failure: Box<ResidentA2aDelegateLaunchFailure<'operation, 'call>>,
    },
}

/// A recovered pairwise delegation stopped before or after durable dispatch custody.
#[must_use = "continue only the returned exact retained phase"]
pub(super) enum ResidentProcessPairwiseA2aDelegateLaunchFailure<'operation, 'call> {
    /// Pairwise lineage, child meaning, provider input, or selected deployment differed.
    Selection {
        dispatch: Box<ProviderOperationDispatchInput<'operation, 'call>>,
        reason: ResidentProcessHostSelectionError,
    },
    /// Provider dispatch retention stopped before any A2A request was constructed.
    Provider(ProviderOperationRetentionFailure<'operation>),
    /// A2A preparation, custody, carrier, or route handling retained its exact phase.
    A2a(Box<ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>>),
}

/// Carrier-independent A2A fields selected after provider dispatch retention.
///
/// The retained provider dispatch is deliberately absent. The process owner
/// injects it exactly once, so a caller cannot substitute another dispatch.
pub(super) struct ResidentProcessA2aDelegateLaunchPlan<'call> {
    pub(super) current: &'call AmbientSessionAdmissionContext<'call>,
    pub(super) route: (&'call ResidentRecordRefV1, &'call A2aRouteAdmissionV1),
    pub(super) route_predecessor: OperationCustodyIndexPredecessor,
    pub(super) route_association: A2aRouteAssociation,
    pub(super) request_id: &'call A2aJsonRpcRequestIdV1,
    pub(super) message_id: &'call str,
    pub(super) text: &'call str,
    pub(super) request_evidence_ref: ResidentRecordRefV1,
    pub(super) preparation_ref: ResidentRecordRefV1,
    pub(super) exchange_limits: A2aExchangeLimits,
    pub(super) maximum_evidence_bytes: usize,
    pub(super) maximum_record_bytes: usize,
    pub(super) maximum_dispatch_record_bytes: usize,
    pub(super) resolve_store: &'call dyn OperationObservationAuthorityStoreResolver,
    pub(super) checkpoint: OperationCheckpointInput,
    pub(super) checkpoint_schema: &'call SchemaDigest,
    pub(super) checkpoint_authority:
        &'call ControlCheckpointSigningAuthorityAdmissionContextV1<'call>,
    pub(super) response_ref: ResidentRecordRefV1,
    pub(super) clock: &'call dyn WallClockSourcePort,
    pub(super) accumulator_limits: AccumulatorPreparationLimits,
    pub(super) membership_limits: AccumulatorMembershipLimits,
}

impl<'call> ResidentProcessA2aDelegateLaunchPlan<'call> {
    fn into_input<'operation>(
        self,
        dispatch: RetainedProviderOperationDispatch<'operation>,
    ) -> A2aDelegateLaunchInput<'operation, 'call> {
        A2aDelegateLaunchInput {
            send: A2aSendInput {
                dispatch: A2aSendDispatch::fresh(dispatch),
                current: self.current,
                route: self.route,
                route_predecessor: self.route_predecessor,
                route_association: self.route_association,
                request_id: self.request_id,
                message_id: self.message_id,
                text: self.text,
                request_evidence_ref: self.request_evidence_ref,
                preparation_ref: self.preparation_ref,
                exchange_limits: self.exchange_limits,
                maximum_evidence_bytes: self.maximum_evidence_bytes,
                maximum_record_bytes: self.maximum_record_bytes,
                maximum_dispatch_record_bytes: self.maximum_dispatch_record_bytes,
                resolve_store: self.resolve_store,
            },
            checkpoint: self.checkpoint,
            checkpoint_schema: self.checkpoint_schema,
            checkpoint_authority: self.checkpoint_authority,
            response_ref: self.response_ref,
            clock: self.clock,
            accumulator_limits: self.accumulator_limits,
            membership_limits: self.membership_limits,
        }
    }
}

/// Process-host result of one non-blocking A2A response poll.
#[must_use = "retain or continue the exact routed A2A poll result"]
pub(super) enum ResidentProcessA2aDelegatePoll {
    /// No terminal item is ready and the route remains active.
    Pending(ResidentOperationTaskKey),
    /// One typed terminal response now owns the route.
    Completed(ResidentProcessA2aDelegateCompletion),
    /// Diagnostic construction stopped after the exchange closed.
    Diagnostic(ResidentProcessA2aDelegateDiagnostic),
    /// The closed exchange could not move into terminal handling. Its route remains active.
    SettlementRejected {
        selector: ResidentDeploymentAuthoritySelector,
        rejection: Box<ResidentA2aDelegateSettlementRejection>,
    },
    /// The lower task owner no longer has the route's exact key.
    Unrouted {
        selector: ResidentDeploymentAuthoritySelector,
        key: ResidentOperationTaskKey,
    },
}

/// Exact branch-specific input for durable A2A outcome retention.
#[must_use = "retain this exact terminal branch before releasing its route"]
pub(super) enum ResidentProcessA2aDelegateRetention<'a> {
    /// An actual remote Task was identified.
    Task {
        route: ResidentProcessA2aDelegateRouteCustody,
        finalization: ResidentA2aDelegateFinalization,
        input: Box<A2aTaskOutcomeRetention<'a>>,
    },
    /// The response created no remote Task binding.
    NoTask {
        route: ResidentProcessA2aDelegateRouteCustody,
        finalization: ResidentA2aDelegateFinalization,
        input: A2aNoTaskOutcomeRetention<'a>,
    },
    /// The remote carrier rejected the request before queue acceptance.
    NotStarted {
        route: ResidentProcessA2aDelegateRouteCustody,
        finalization: ResidentA2aDelegateFinalization,
        input: A2aNotStartedOutcomeRetention<'a>,
    },
    /// Remote acceptance or effect remains unknown.
    OutcomeUnknown {
        route: ResidentProcessA2aDelegateRouteCustody,
        finalization: ResidentA2aDelegateFinalization,
        input: A2aUnknownOutcomeRetention<'a>,
    },
    /// Restart recovery proved a retained send but not its remote effect.
    RecoveredOutcomeUnknown {
        route: ResidentProcessA2aDelegateRouteCustody,
        finalization: ResidentA2aDelegateFinalization,
        input: A2aRecoveredUnknownOutcomeRetention<'a>,
    },
}

impl<'a> ResidentProcessA2aDelegateRetention<'a> {
    fn process_association(
        &self,
    ) -> Result<(ResidentOperationTaskKey, A2aOutcomeRetentionKind), ()> {
        let (route, finalization, association) = match self {
            Self::Task {
                route,
                finalization,
                input,
            } => (route, finalization, input.process_association()),
            Self::NoTask {
                route,
                finalization,
                input,
            } => (route, finalization, input.process_association()),
            Self::NotStarted {
                route,
                finalization,
                input,
            } => (route, finalization, input.process_association()),
            Self::OutcomeUnknown {
                route,
                finalization,
                input,
            } => (route, finalization, input.process_association()),
            Self::RecoveredOutcomeUnknown {
                route,
                finalization,
                input,
            } => (route, finalization, input.process_association()),
        };
        association
            .ok()
            .filter(|(key, kind)| {
                key == &route.key
                    && finalization.matches_process_retention(
                        &route.owner_instance,
                        &route.key,
                        *kind,
                    )
            })
            .ok_or(())
    }

    fn into_parts(
        self,
    ) -> (
        ResidentProcessA2aDelegateRouteCustody,
        ResidentA2aDelegateFinalization,
        A2aOutcomeRetentionInput<'a>,
    ) {
        match self {
            Self::Task {
                route,
                finalization,
                input,
            } => (route, finalization, A2aOutcomeRetentionInput::Task(input)),
            Self::NoTask {
                route,
                finalization,
                input,
            } => (route, finalization, A2aOutcomeRetentionInput::NoTask(input)),
            Self::NotStarted {
                route,
                finalization,
                input,
            } => (
                route,
                finalization,
                A2aOutcomeRetentionInput::NotStarted(input),
            ),
            Self::OutcomeUnknown {
                route,
                finalization,
                input,
            } => (
                route,
                finalization,
                A2aOutcomeRetentionInput::OutcomeUnknown(input),
            ),
            Self::RecoveredOutcomeUnknown {
                route,
                finalization,
                input,
            } => (
                route,
                finalization,
                A2aOutcomeRetentionInput::RecoveredOutcomeUnknown(input),
            ),
        }
    }
}

/// Protected send recovery stopped without granting carrier replay.
#[must_use = "continue only with the returned exact recovery phase"]
pub(super) enum ResidentProcessA2aDelegateRecoveryFailure<'operation, 'call, 'recovery> {
    /// The original unresolved send remains available for another protected recovery.
    Unresolved(Box<ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>>),
    /// Recovery succeeded, but capacity could not move into lost-send handling.
    Loss {
        key: ResidentOperationTaskKey,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        failure: Box<ResidentA2aDelegateUnresolvedRecoveryFailure<'operation, 'call, 'recovery>>,
    },
}

/// Cold-restart recovery stopped without granting another carrier call.
#[must_use = "preserve any returned route-bound recovery phase"]
pub(super) enum ResidentProcessA2aDelegateRestartFailure<'recovery> {
    /// The recovered attempt, selected deployment, or access store differs.
    Selection(ResidentProcessHostSelectionError),
    /// No preallocated process-local A2A route remained.
    RouteSaturated,
    /// The exact recovered dispatch already has a process-local route.
    RouteConflict,
    /// Recovery was rejected and both aggregate and process route capacity were returned.
    Rejected(ResidentA2aDelegateRecoveryFailure<'recovery>),
    /// Aggregate offer cleanup must finish before the process route can be returned.
    Rollback {
        key: ResidentOperationTaskKey,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        failure: Box<ResidentA2aDelegateRecoveryFailure<'recovery>>,
    },
    /// Recovery succeeded, but aggregate capacity has not entered loss handling.
    Loss {
        key: ResidentOperationTaskKey,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        failure: Box<ResidentA2aDelegateRecoveredOfferRejection<'recovery>>,
    },
}

/// Result of retrying only a retained cold-restart aggregate transition.
#[must_use = "retain recovered uncertainty or handle the original recovery rejection"]
pub(super) enum ResidentProcessA2aDelegateRestartRetry<'recovery> {
    /// Offer cleanup completed for the original rejected recovery.
    Rejected(A2aSendRecoveryError),
    /// The recovered send now holds loss-handling capacity and its process route.
    Recovered(Box<ResidentProcessRecoveredA2aDelegate<'recovery>>),
}

/// A terminal A2A branch stopped before its process route could be released.
#[must_use = "continue with the returned route and lower custody phase"]
pub(super) enum ResidentProcessA2aDelegateRetentionFailure<'a> {
    /// The supplied route no longer selects the exact deployment and store.
    Selection {
        pending: Box<ResidentProcessA2aDelegateRetention<'a>>,
        reason: ResidentProcessHostSelectionError,
    },
    /// Durable branch retention stopped; task capacity and the process route remain held.
    Retention {
        route: ResidentProcessA2aDelegateRouteCustody,
        key: ResidentOperationTaskKey,
        kind: A2aOutcomeRetentionKind,
        finalization: Box<ResidentA2aDelegateFinalization>,
        failure: Box<ContinuableA2aOutcomeRetentionFailure<'a>>,
        reason: Option<ResidentProcessHostSelectionError>,
    },
    /// Durable retention succeeded, but task-capacity release was rejected.
    Finalization {
        route: ResidentProcessA2aDelegateRouteCustody,
        rejection: Box<ResidentA2aDelegateFinalizationRejection>,
    },
    /// Lower settlement completed, but the process-route capability was invalid.
    Route {
        route: Box<ResidentProcessA2aDelegateRouteCustody>,
        key: ResidentOperationTaskKey,
        kind: A2aOutcomeRetentionKind,
        retained: Box<RetainedA2aOutcome>,
    },
}

enum ResidentProcessA2aDelegateLowerRetentionFailure<'a> {
    Retention(Box<ResidentProcessA2aDelegateLowerRetentionCustody<'a>>),
    Finalization(ResidentA2aDelegateFinalizationRejection),
}

struct ResidentProcessA2aDelegateLowerRetentionCustody<'a> {
    key: ResidentOperationTaskKey,
    kind: A2aOutcomeRetentionKind,
    finalization: ResidentA2aDelegateFinalization,
    failure: ContinuableA2aOutcomeRetentionFailure<'a>,
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Retain and launch one recovered pairwise Delegate through the A2A carrier.
    ///
    /// Protected recovery lends the exact child meaning only inside this call.
    /// Provider dispatch custody completes before A2A request construction, and
    /// the lower A2A owner retains the request before it starts the carrier.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub(super) fn launch_recovered_pairwise_a2a_delegate<
        'operation,
        'call,
        Authority,
        Build,
        Rotation,
        ObservationSign,
        ProviderCheckpointSign,
        Disclosure,
        Transport,
        A2aCheckpointSign,
    >(
        &mut self,
        recovered: &super::restart::ResidentProcessRecoveredOperationResolution,
        access: ResidentDeploymentOperationAccess<'_>,
        resolution: ResidentProcessOperationResolutionInput<'_>,
        completed_context: Option<
            &crate::resident::process_host::operation_resolution::CompletedResidentProcessOperationContext<'_>,
        >,
        authority: &mut Authority,
        dispatch: ProviderOperationDispatchInput<'operation, 'call>,
        build_a2a: Build,
        exchange: &dyn A2aExchangePort,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_provider_checkpoint: ProviderCheckpointSign,
        admit_disclosure: Disclosure,
        admit_transport: Transport,
        sign_a2a_checkpoint: A2aCheckpointSign,
    ) -> Result<
        Result<
            ResidentProcessA2aDelegateLaunch,
            ResidentProcessPairwiseA2aDelegateLaunchFailure<'operation, 'call>,
        >,
        ResidentProcessOperationResolutionFailure,
    >
    where
        Authority: AmbientSessionAuthorityPort,
        Build: FnOnce() -> ResidentProcessA2aDelegateLaunchPlan<'call>,
        Rotation: FnMut(
            &pale_core::records::ResidentRecordRefV1,
            &ArtifactDigest,
            &pale_core::records::ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &pale_core::records::OperationObservationPreimageV1,
            &pale_core::records::OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ProviderCheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        )
            -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Disclosure: FnMut(
            &pale_ports::ProviderInvocationRequest<'_>,
            &A2aRouteAdmissionV1,
            &str,
        ) -> Result<(), InvalidResidentRecordV1>,
        Transport: FnOnce(
            &pale_core::records::ImmutableRecordReferenceV1,
            &pale_ports::ProviderInvocationRequest<'_>,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        A2aCheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let selector = &recovered.start.selector;
        let index = self.deployment_authority_index(selector)?;
        let Self {
            operations,
            a2a_delegate_routes,
            deployments,
            ..
        } = self;
        let application = deployments[index].application.as_ref();
        let completed = if let Some(completed) = completed_context {
            Self::validate_completed_operation_context(recovered, completed)?;
            Some(Self::completed_operation_context_parts(completed))
        } else {
            None
        };
        Self::with_operation_resolution_context_in(
            application,
            selector,
            recovered,
            access,
            resolution,
            authority,
            completed,
            |context, checkpoint, access, authority| {
                if super::validate_recovered_pairwise_provider_dispatch(
                    application,
                    selector,
                    recovered,
                    &context,
                    checkpoint,
                    &dispatch,
                )
                .is_err()
                {
                    return Err(ResidentProcessPairwiseA2aDelegateLaunchFailure::Selection {
                        dispatch: Box::new(dispatch),
                        reason: ResidentProcessHostSelectionError::A2aOperationAssociation,
                    });
                }
                let ResidentDeploymentOperationAccess {
                    store,
                    custody,
                    sessions,
                    anchor,
                } = access;
                let retained = retain_provider_operation_dispatch(
                    dispatch,
                    authority,
                    &mut *custody,
                    &mut *anchor,
                    admit_rotation,
                    sign_observation,
                    sign_provider_checkpoint,
                )
                .map_err(ResidentProcessPairwiseA2aDelegateLaunchFailure::Provider)?;
                let input = build_a2a().into_input(retained);
                Self::launch_a2a_delegate_in(
                    application,
                    operations,
                    a2a_delegate_routes,
                    selector,
                    ResidentDeploymentOperationAccess {
                        store,
                        custody,
                        sessions,
                        anchor,
                    },
                    authority,
                    input,
                    exchange,
                    admit_disclosure,
                    admit_transport,
                    sign_a2a_checkpoint,
                )
                .map_err(ResidentProcessPairwiseA2aDelegateLaunchFailure::A2a)
            },
        )
    }

    /// Retain and start one initial A2A `SendMessage` at most once.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn launch_a2a_delegate<'operation, 'call, Authority, Disclosure, Transport, Sign>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: A2aDelegateLaunchInput<'operation, 'call>,
        exchange: &dyn A2aExchangePort,
        admit_disclosure: Disclosure,
        admit_transport: Transport,
        sign_checkpoint: Sign,
    ) -> Result<
        ResidentProcessA2aDelegateLaunch,
        Box<ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>>,
    >
    where
        Authority: AmbientSessionAuthorityPort,
        Disclosure: FnMut(
            &pale_ports::ProviderInvocationRequest<'_>,
            &A2aRouteAdmissionV1,
            &str,
        ) -> Result<(), InvalidResidentRecordV1>,
        Transport: FnOnce(
            &pale_core::records::ImmutableRecordReferenceV1,
            &pale_ports::ProviderInvocationRequest<'_>,
            &pale_core::records::ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let index = match self.deployment_authority_index(selector) {
            Ok(index) => index,
            Err(reason) => {
                return Err(Box::new(
                    ResidentProcessA2aDelegateLaunchFailure::Selection {
                        input: Box::new(input),
                        reason,
                    },
                ));
            }
        };
        let Self {
            operations,
            a2a_delegate_routes,
            deployments,
            ..
        } = self;
        Self::launch_a2a_delegate_in(
            deployments[index].application.as_ref(),
            operations,
            a2a_delegate_routes,
            selector,
            access,
            authority,
            input,
            exchange,
            admit_disclosure,
            admit_transport,
            sign_checkpoint,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn launch_a2a_delegate_in<'operation, 'call, Authority, Disclosure, Transport, Sign>(
        application: &ResidentApplication<'_>,
        operations: &mut crate::resident::operation_host::ResidentOperationHost,
        routes: &mut ResidentProcessA2aDelegateRoutes,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: A2aDelegateLaunchInput<'operation, 'call>,
        exchange: &dyn A2aExchangePort,
        admit_disclosure: Disclosure,
        admit_transport: Transport,
        sign_checkpoint: Sign,
    ) -> Result<
        ResidentProcessA2aDelegateLaunch,
        Box<ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>>,
    >
    where
        Authority: AmbientSessionAuthorityPort,
        Disclosure: FnMut(
            &pale_ports::ProviderInvocationRequest<'_>,
            &A2aRouteAdmissionV1,
            &str,
        ) -> Result<(), InvalidResidentRecordV1>,
        Transport: FnOnce(
            &pale_core::records::ImmutableRecordReferenceV1,
            &pale_ports::ProviderInvocationRequest<'_>,
            &pale_core::records::ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let key = match Self::validate_a2a_delegate_launch_in(
            application,
            selector,
            access.store,
            &input,
        ) {
            Ok(key) => key,
            Err(reason) => {
                return Err(Box::new(
                    ResidentProcessA2aDelegateLaunchFailure::Selection {
                        input: Box::new(input),
                        reason,
                    },
                ));
            }
        };
        let route = match routes.reserve(selector.clone()) {
            Ok(route) => route,
            Err(()) => {
                return Err(Box::new(
                    ResidentProcessA2aDelegateLaunchFailure::RouteSaturated(Box::new(input)),
                ));
            }
        };
        match routes.commit(route, key.clone()) {
            Ok(()) => {}
            Err(ResidentProcessA2aDelegateRouteCommitFailure::DuplicateActiveKey) => {
                return Err(Box::new(
                    ResidentProcessA2aDelegateLaunchFailure::RouteConflict(Box::new(input)),
                ));
            }
            Err(ResidentProcessA2aDelegateRouteCommitFailure::ForeignReservation(route)) => {
                let _ = routes.release_reservation(*route);
                return Err(Box::new(
                    ResidentProcessA2aDelegateLaunchFailure::RouteConflict(Box::new(input)),
                ));
            }
        }

        let ResidentDeploymentOperationAccess {
            custody,
            sessions: _,
            anchor,
            ..
        } = access;
        let launched = operations.with_operation_service(authority, custody, anchor, |service| {
            service.launch_a2a_delegate(
                input,
                exchange,
                admit_disclosure,
                admit_transport,
                sign_checkpoint,
            )
        });

        match launched {
            Ok(ResidentA2aDelegateLaunch::Awaiting(actual)) => {
                debug_assert_eq!(actual, key);
                Ok(ResidentProcessA2aDelegateLaunch::Awaiting(key))
            }
            Ok(ResidentA2aDelegateLaunch::Completed(completion)) => {
                let route = routes
                    .check_out(&key)
                    .expect("the committed A2A route must remain available");
                Ok(ResidentProcessA2aDelegateLaunch::Completed(
                    ResidentProcessA2aDelegateCompletion { route, completion },
                ))
            }
            Ok(ResidentA2aDelegateLaunch::Diagnostic(diagnostic)) => {
                let route = routes
                    .check_out(&key)
                    .expect("the committed A2A route must remain available");
                Ok(ResidentProcessA2aDelegateLaunch::Diagnostic(
                    ResidentProcessA2aDelegateDiagnostic { route, diagnostic },
                ))
            }
            Err(failure) if lower_launch_proves_unstarted(&failure) => {
                let removed = routes.remove_unstarted(&key, selector);
                debug_assert!(removed);
                Err(Box::new(ResidentProcessA2aDelegateLaunchFailure::Rejected(
                    Box::new(failure),
                )))
            }
            Err(failure) => Err(Box::new(routed_launch_failure(
                key,
                selector.clone(),
                failure,
            ))),
        }
    }

    /// Retry only capacity release after proof that the carrier never started.
    pub(super) fn retry_a2a_delegate_launch_rollback<'operation, 'call, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>,
    ) -> Result<(), Box<ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>>> {
        let ResidentProcessA2aDelegateLaunchFailure::Rollback {
            key,
            selector,
            failure,
        } = pending
        else {
            return Err(Box::new(pending));
        };
        if !self.validate_active_a2a_route(&key, &selector, access.store) {
            return Err(Box::new(
                ResidentProcessA2aDelegateLaunchFailure::Rollback {
                    key,
                    selector,
                    failure,
                },
            ));
        }
        let retried = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retry_a2a_delegate_launch_rollback(*failure)
                },
            )
            .expect("the active route and access were checked before rollback");
        match retried {
            Ok(ResidentA2aDelegateLaunchFailure::Launch(_)) => {
                let removed = self.a2a_delegate_routes.remove_unstarted(&key, &selector);
                debug_assert!(removed);
                Ok(())
            }
            Ok(failure) | Err(failure) => {
                Err(Box::new(routed_launch_failure(key, *selector, failure)))
            }
        }
    }

    /// Continue exact request custody without invoking the A2A carrier.
    pub(super) fn continue_a2a_delegate_custody<'operation, 'call, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>,
    ) -> Result<
        ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>,
        Box<ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>>,
    > {
        let ResidentProcessA2aDelegateLaunchFailure::Custody {
            key,
            selector,
            failure,
        } = pending
        else {
            return Err(Box::new(pending));
        };
        let ResidentA2aDelegateLaunchFailure::Custody(unresolved) = *failure else {
            unreachable!("the process custody branch retains lower custody")
        };
        if !self.validate_active_a2a_route(&key, &selector, access.store) {
            return Err(Box::new(ResidentProcessA2aDelegateLaunchFailure::Custody {
                key,
                selector,
                failure: Box::new(ResidentA2aDelegateLaunchFailure::Custody(unresolved)),
            }));
        }
        let continued = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.continue_a2a_delegate_custody(*unresolved)
                },
            )
            .expect("the active route and access were checked before custody continuation");
        match continued {
            Ok(unopened) => Ok(ResidentProcessA2aDelegateLaunchFailure::Unopened {
                key,
                selector,
                failure: Box::new(ResidentA2aDelegateLaunchFailure::Unopened(Box::new(
                    unopened,
                ))),
            }),
            Err(failure) => {
                let ResidentA2aDelegateCustodyContinuationFailure { unresolved, .. } = failure;
                Err(Box::new(ResidentProcessA2aDelegateLaunchFailure::Custody {
                    key,
                    selector,
                    failure: Box::new(ResidentA2aDelegateLaunchFailure::Custody(unresolved)),
                }))
            }
        }
    }

    /// Recover unresolved request custody as explicit remote-effect uncertainty.
    pub(super) fn recover_unresolved_a2a_delegate<'operation, 'call, 'recovery, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>,
        input: A2aSendRecoveryInput<'recovery>,
        admission: A2aSendRecoveryAdmission<'_>,
    ) -> Result<
        ResidentProcessRecoveredA2aDelegate<'recovery>,
        Box<ResidentProcessA2aDelegateRecoveryFailure<'operation, 'call, 'recovery>>,
    > {
        let ResidentProcessA2aDelegateLaunchFailure::Custody {
            key,
            selector,
            failure,
        } = pending
        else {
            return Err(Box::new(
                ResidentProcessA2aDelegateRecoveryFailure::Unresolved(Box::new(pending)),
            ));
        };
        let ResidentA2aDelegateLaunchFailure::Custody(unresolved) = *failure else {
            unreachable!("the process custody branch retains lower custody")
        };
        if !self.validate_active_a2a_route(&key, &selector, access.store) {
            return Err(Box::new(
                ResidentProcessA2aDelegateRecoveryFailure::Unresolved(Box::new(
                    ResidentProcessA2aDelegateLaunchFailure::Custody {
                        key,
                        selector,
                        failure: Box::new(ResidentA2aDelegateLaunchFailure::Custody(unresolved)),
                    },
                )),
            ));
        }
        let recovered = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.recover_unresolved_a2a_delegate(*unresolved, input, admission)
                },
            )
            .expect("the active route and access were checked before protected recovery");
        match recovered {
            Ok(recovered) => {
                let route = self
                    .a2a_delegate_routes
                    .check_out(&key)
                    .expect("protected recovery retains the active A2A route");
                Ok(ResidentProcessRecoveredA2aDelegate { route, recovered })
            }
            Err(failure @ ResidentA2aDelegateUnresolvedRecoveryFailure::LossReservation(_)) => {
                Err(Box::new(ResidentProcessA2aDelegateRecoveryFailure::Loss {
                    key,
                    selector,
                    failure: Box::new(failure),
                }))
            }
            Err(failure) => {
                let unresolved = unresolved_from_recovery_failure(failure);
                Err(Box::new(
                    ResidentProcessA2aDelegateRecoveryFailure::Unresolved(Box::new(
                        ResidentProcessA2aDelegateLaunchFailure::Custody {
                            key,
                            selector,
                            failure: Box::new(ResidentA2aDelegateLaunchFailure::Custody(
                                unresolved,
                            )),
                        },
                    )),
                ))
            }
        }
    }

    /// Retry only moving protected recovery into lost-send capacity.
    pub(super) fn retry_recovered_a2a_delegate_loss<'operation, 'call, 'recovery, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateRecoveryFailure<'operation, 'call, 'recovery>,
    ) -> Result<
        ResidentProcessRecoveredA2aDelegate<'recovery>,
        Box<ResidentProcessA2aDelegateRecoveryFailure<'operation, 'call, 'recovery>>,
    > {
        let ResidentProcessA2aDelegateRecoveryFailure::Loss {
            key,
            selector,
            failure,
        } = pending
        else {
            return Err(Box::new(pending));
        };
        let ResidentA2aDelegateUnresolvedRecoveryFailure::LossReservation(rejected) = *failure
        else {
            unreachable!("the process loss branch retains a lower loss rejection")
        };
        if !self.validate_active_a2a_route(&key, &selector, access.store) {
            return Err(Box::new(ResidentProcessA2aDelegateRecoveryFailure::Loss {
                key,
                selector,
                failure: Box::new(
                    ResidentA2aDelegateUnresolvedRecoveryFailure::LossReservation(rejected),
                ),
            }));
        }
        let recovered = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retry_recovered_a2a_delegate_loss(*rejected)
                },
            )
            .expect("the active route and access were checked before loss retry");
        match recovered {
            Ok(recovered) => {
                let route = self
                    .a2a_delegate_routes
                    .check_out(&key)
                    .expect("loss recovery retains the active A2A route");
                Ok(ResidentProcessRecoveredA2aDelegate { route, recovered })
            }
            Err(rejected) => Err(Box::new(ResidentProcessA2aDelegateRecoveryFailure::Loss {
                key,
                selector,
                failure: Box::new(
                    ResidentA2aDelegateUnresolvedRecoveryFailure::LossReservation(Box::new(
                        rejected,
                    )),
                ),
            })),
        }
    }

    /// Recover one cold-restart A2A send as explicit uncertainty.
    ///
    /// A fresh bounded process route is established before the existing recovery
    /// owner receives work. This path has no exchange port and cannot replay the
    /// retained `SendMessage` request.
    pub(super) fn recover_restarted_a2a_delegate<'recovery, Authority>(
        &mut self,
        attempt: ResidentProcessRecoveredAttempt,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: A2aSendRecoveryInput<'recovery>,
        admission: A2aSendRecoveryAdmission<'_>,
    ) -> Result<
        ResidentProcessRecoveredA2aDelegate<'recovery>,
        Box<ResidentProcessA2aDelegateRestartFailure<'recovery>>,
    > {
        let selector = attempt.start.selector.clone();
        let key = self
            .validate_a2a_delegate_restart(&attempt, access.store, &input)
            .map_err(|reason| {
                Box::new(ResidentProcessA2aDelegateRestartFailure::Selection(reason))
            })?;
        let route = self
            .a2a_delegate_routes
            .reserve(selector.clone())
            .map_err(|()| Box::new(ResidentProcessA2aDelegateRestartFailure::RouteSaturated))?;
        match self.a2a_delegate_routes.commit(route, key.clone()) {
            Ok(()) => {}
            Err(ResidentProcessA2aDelegateRouteCommitFailure::DuplicateActiveKey) => {
                return Err(Box::new(
                    ResidentProcessA2aDelegateRestartFailure::RouteConflict,
                ));
            }
            Err(ResidentProcessA2aDelegateRouteCommitFailure::ForeignReservation(route)) => {
                let _ = self.a2a_delegate_routes.release_reservation(*route);
                return Err(Box::new(
                    ResidentProcessA2aDelegateRestartFailure::RouteConflict,
                ));
            }
        }

        let recovered = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| service.recover_a2a_delegate(input, admission),
            )
            .expect("the deployment and store were checked before A2A restart recovery");
        match recovered {
            Ok(recovered) => {
                let route = self
                    .a2a_delegate_routes
                    .check_out(&key)
                    .expect("recovered A2A uncertainty retains its process route");
                Ok(ResidentProcessRecoveredA2aDelegate { route, recovered })
            }
            Err(failure @ ResidentA2aDelegateRecoveryFailure::Rollback { .. }) => Err(Box::new(
                ResidentProcessA2aDelegateRestartFailure::Rollback {
                    key,
                    selector: Box::new(selector),
                    failure: Box::new(failure),
                },
            )),
            Err(ResidentA2aDelegateRecoveryFailure::LossReservation(failure)) => {
                Err(Box::new(ResidentProcessA2aDelegateRestartFailure::Loss {
                    key,
                    selector: Box::new(selector),
                    failure,
                }))
            }
            Err(failure) => {
                let removed = self.a2a_delegate_routes.remove_unstarted(&key, &selector);
                debug_assert!(removed);
                Err(Box::new(
                    ResidentProcessA2aDelegateRestartFailure::Rejected(failure),
                ))
            }
        }
    }

    /// Retry only the aggregate transition retained by cold-restart recovery.
    ///
    /// This path performs no recovery reads and has no carrier access.
    pub(super) fn retry_restarted_a2a_delegate<'recovery, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateRestartFailure<'recovery>,
    ) -> Result<
        ResidentProcessA2aDelegateRestartRetry<'recovery>,
        Box<ResidentProcessA2aDelegateRestartFailure<'recovery>>,
    > {
        match pending {
            ResidentProcessA2aDelegateRestartFailure::Rollback {
                key,
                selector,
                failure,
            } => {
                if !self.validate_active_a2a_route(&key, &selector, access.store) {
                    return Err(Box::new(
                        ResidentProcessA2aDelegateRestartFailure::Rollback {
                            key,
                            selector,
                            failure,
                        },
                    ));
                }
                let retried = self
                    .with_selected_operation_service(
                        &selector,
                        access,
                        authority,
                        |_application, service, _sessions| {
                            service.retry_a2a_delegate_recovery_rollback(*failure)
                        },
                    )
                    .expect("the active route and store were checked before cleanup retry");
                match retried {
                    Ok(recovery) => {
                        let removed = self.a2a_delegate_routes.remove_unstarted(&key, &selector);
                        debug_assert!(removed);
                        Ok(ResidentProcessA2aDelegateRestartRetry::Rejected(recovery))
                    }
                    Err(failure) => Err(Box::new(
                        ResidentProcessA2aDelegateRestartFailure::Rollback {
                            key,
                            selector,
                            failure: Box::new(failure),
                        },
                    )),
                }
            }
            ResidentProcessA2aDelegateRestartFailure::Loss {
                key,
                selector,
                failure,
            } => {
                if !self.validate_active_a2a_route(&key, &selector, access.store) {
                    return Err(Box::new(ResidentProcessA2aDelegateRestartFailure::Loss {
                        key,
                        selector,
                        failure,
                    }));
                }
                let retried = self
                    .with_selected_operation_service(
                        &selector,
                        access,
                        authority,
                        |_application, service, _sessions| {
                            service.retry_recovered_a2a_delegate_loss(*failure)
                        },
                    )
                    .expect("the active route and store were checked before loss retry");
                match retried {
                    Ok(recovered) => {
                        let route = self
                            .a2a_delegate_routes
                            .check_out(&key)
                            .expect("recovered A2A uncertainty retains its process route");
                        Ok(ResidentProcessA2aDelegateRestartRetry::Recovered(Box::new(
                            ResidentProcessRecoveredA2aDelegate { route, recovered },
                        )))
                    }
                    Err(failure) => Err(Box::new(ResidentProcessA2aDelegateRestartFailure::Loss {
                        key,
                        selector,
                        failure: Box::new(failure),
                    })),
                }
            }
            pending => Err(Box::new(pending)),
        }
    }

    /// Retry one exact send only while the lower owner proves it unopened.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn retry_unopened_a2a_delegate<'operation, 'call, Authority, Disclosure>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        admit_disclosure: Disclosure,
    ) -> Result<
        ResidentProcessA2aDelegateLaunch,
        Box<ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>>,
    >
    where
        Authority: AmbientSessionAuthorityPort,
        Disclosure: FnOnce(
            &pale_ports::ProviderInvocationRequest<'_>,
            &A2aRouteAdmissionV1,
            &str,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let ResidentProcessA2aDelegateLaunchFailure::Unopened {
            key,
            selector,
            failure,
        } = pending
        else {
            return Err(Box::new(pending));
        };
        let ResidentA2aDelegateLaunchFailure::Unopened(unopened) = *failure else {
            unreachable!("the process unopened branch retains lower unopened custody")
        };
        if !self.validate_active_a2a_route(&key, &selector, access.store) {
            return Err(Box::new(
                ResidentProcessA2aDelegateLaunchFailure::Unopened {
                    key,
                    selector,
                    failure: Box::new(ResidentA2aDelegateLaunchFailure::Unopened(unopened)),
                },
            ));
        }
        let retried = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retry_unopened_a2a_delegate(
                        *unopened,
                        exchange,
                        clock,
                        admit_disclosure,
                    )
                },
            )
            .expect("the active route and access were checked before carrier retry");
        match retried {
            Ok(launch) => self.join_routed_a2a_launch(key, *selector, Ok(launch)),
            Err(ResidentA2aDelegateUnopenedRetryFailure::Reservation { unopened, .. })
            | Err(ResidentA2aDelegateUnopenedRetryFailure::Unopened(unopened)) => Err(Box::new(
                ResidentProcessA2aDelegateLaunchFailure::Unopened {
                    key,
                    selector,
                    failure: Box::new(ResidentA2aDelegateLaunchFailure::Unopened(unopened)),
                },
            )),
            Err(ResidentA2aDelegateUnopenedRetryFailure::Closed(failure)) => {
                self.join_routed_a2a_launch(key, *selector, Err(*failure))
            }
        }
    }

    /// Settle a closed launch that returned an unlike task key.
    pub(super) fn settle_a2a_delegate_started_association<'operation, 'call, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>,
    ) -> Result<
        ResidentProcessA2aDelegateLaunch,
        Box<ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>>,
    > {
        let ResidentProcessA2aDelegateLaunchFailure::StartedAssociation {
            key,
            selector,
            failure,
        } = pending
        else {
            return Err(Box::new(pending));
        };
        if !self.validate_active_a2a_route(&key, &selector, access.store) {
            return Err(Box::new(
                ResidentProcessA2aDelegateLaunchFailure::StartedAssociation {
                    key,
                    selector,
                    failure,
                },
            ));
        }
        let settled = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.settle_a2a_delegate_started_association(*failure)
                },
            )
            .expect("the active route and access were checked before settlement");
        self.join_routed_a2a_launch(key, *selector, settled)
    }

    /// Retry only moving a closed exchange into terminal-handling capacity.
    pub(super) fn retry_a2a_delegate_offered_completion<'operation, 'call, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>,
    ) -> Result<
        ResidentProcessA2aDelegatePoll,
        Box<ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>>,
    > {
        let ResidentProcessA2aDelegateLaunchFailure::OfferSettlement {
            key,
            selector,
            failure,
        } = pending
        else {
            return Err(Box::new(pending));
        };
        let ResidentA2aDelegateLaunchFailure::OfferSettlement(rejection) = *failure else {
            unreachable!("the process settlement branch retains lower settlement custody")
        };
        if !self.validate_active_a2a_route(&key, &selector, access.store) {
            return Err(Box::new(
                ResidentProcessA2aDelegateLaunchFailure::OfferSettlement {
                    key,
                    selector,
                    failure: Box::new(ResidentA2aDelegateLaunchFailure::OfferSettlement(rejection)),
                },
            ));
        }
        let retried = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retry_a2a_delegate_offered_completion(*rejection)
                },
            )
            .expect("the active route and access were checked before settlement retry");
        match retried {
            Ok(poll) => Ok(self.join_routed_a2a_poll(key, *selector, poll)),
            Err(rejection) => Err(Box::new(
                ResidentProcessA2aDelegateLaunchFailure::OfferSettlement {
                    key,
                    selector,
                    failure: Box::new(ResidentA2aDelegateLaunchFailure::OfferSettlement(Box::new(
                        rejection,
                    ))),
                },
            )),
        }
    }

    /// Poll one started send without reconnecting or replaying it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn poll_a2a_delegate<Authority>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        key: &ResidentOperationTaskKey,
        cx: &mut Context<'_>,
        protocol: &dyn A2aSendMessageEvidencePort,
        clock: &dyn WallClockSourcePort,
    ) -> Result<ResidentProcessA2aDelegatePoll, ResidentProcessHostSelectionError> {
        if self.a2a_delegate_routes.selector(key) != Some(selector) {
            return Err(ResidentProcessHostSelectionError::A2aProcessRouteAssociation);
        }
        let polled = self.with_selected_operation_service(
            selector,
            access,
            authority,
            |_application, service, _sessions| service.poll_a2a_delegate(key, cx, protocol, clock),
        )?;
        Ok(match polled {
            ResidentA2aDelegatePoll::Pending => {
                ResidentProcessA2aDelegatePoll::Pending(key.clone())
            }
            ResidentA2aDelegatePoll::Completed(completion) => {
                let route = self
                    .a2a_delegate_routes
                    .check_out(key)
                    .expect("the selected A2A route must remain available");
                ResidentProcessA2aDelegatePoll::Completed(ResidentProcessA2aDelegateCompletion {
                    route,
                    completion,
                })
            }
            ResidentA2aDelegatePoll::Diagnostic(diagnostic) => {
                let route = self
                    .a2a_delegate_routes
                    .check_out(key)
                    .expect("the selected A2A route must remain available");
                ResidentProcessA2aDelegatePoll::Diagnostic(ResidentProcessA2aDelegateDiagnostic {
                    route,
                    diagnostic,
                })
            }
            ResidentA2aDelegatePoll::SettlementRejected(rejection) => {
                ResidentProcessA2aDelegatePoll::SettlementRejected {
                    selector: selector.clone(),
                    rejection,
                }
            }
            ResidentA2aDelegatePoll::NotFound => ResidentProcessA2aDelegatePoll::Unrouted {
                selector: selector.clone(),
                key: key.clone(),
            },
        })
    }

    /// Retry only the terminal-handling transition for an already-closed exchange.
    pub(super) fn retry_a2a_delegate_settlement<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegatePoll,
    ) -> Result<ResidentProcessA2aDelegatePoll, Box<ResidentProcessA2aDelegatePoll>> {
        let ResidentProcessA2aDelegatePoll::SettlementRejected {
            selector,
            rejection,
        } = pending
        else {
            return Err(Box::new(pending));
        };
        let key = rejection.key().clone();
        if !self.validate_active_a2a_route(&key, &selector, access.store) {
            return Err(Box::new(
                ResidentProcessA2aDelegatePoll::SettlementRejected {
                    selector,
                    rejection,
                },
            ));
        }
        let poll = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.retry_a2a_delegate_settlement(*rejection)
                },
            )
            .expect("the active route and access were checked before settlement retry");
        Ok(self.join_routed_a2a_poll(key, selector, poll))
    }

    /// Retry diagnostic construction without polling or replaying the carrier.
    pub(super) fn recover_a2a_delegate_diagnostic<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateDiagnostic,
    ) -> Result<ResidentProcessA2aDelegateCompletion, Box<ResidentProcessA2aDelegateDiagnostic>>
    {
        let selector = pending.route.selector.clone();
        if !self.validate_checked_out_a2a_route(&pending.route, access.store) {
            return Err(Box::new(pending));
        }
        let ResidentProcessA2aDelegateDiagnostic { route, diagnostic } = pending;
        let recovered = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.recover_a2a_delegate_diagnostic(diagnostic)
                },
            )
            .expect("the checked-out route and access were checked before diagnostic recovery");
        match recovered {
            Ok(completion) => Ok(ResidentProcessA2aDelegateCompletion { route, completion }),
            Err(failure) => Err(Box::new(ResidentProcessA2aDelegateDiagnostic {
                route,
                diagnostic: diagnostic_from_recovery_failure(failure),
            })),
        }
    }

    /// Retain one terminal branch, release task capacity, then release its route.
    pub(super) fn retain_a2a_delegate_outcome<'a, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateRetention<'a>,
    ) -> Result<RetainedA2aOutcome, Box<ResidentProcessA2aDelegateRetentionFailure<'a>>> {
        let route = match &pending {
            ResidentProcessA2aDelegateRetention::Task { route, .. }
            | ResidentProcessA2aDelegateRetention::NoTask { route, .. }
            | ResidentProcessA2aDelegateRetention::NotStarted { route, .. }
            | ResidentProcessA2aDelegateRetention::OutcomeUnknown { route, .. }
            | ResidentProcessA2aDelegateRetention::RecoveredOutcomeUnknown { route, .. } => route,
        };
        let selector = route.selector.clone();
        let selected = self.deployment_authority_index(&selector);
        let association = pending.process_association();
        let reason = if association.is_err()
            || !route
                .owner_instance
                .is_same(&self.a2a_delegate_routes.owner_instance)
            || route.key.store() != access.store
        {
            Some(ResidentProcessHostSelectionError::A2aProcessRouteAssociation)
        } else {
            match selected {
                Ok(index) if self.deployments[index].store() == access.store => None,
                Ok(_) => Some(ResidentProcessHostSelectionError::AccessStoreAssociation),
                Err(reason) => Some(reason),
            }
        };
        if let Some(reason) = reason {
            return Err(Box::new(
                ResidentProcessA2aDelegateRetentionFailure::Selection {
                    pending: Box::new(pending),
                    reason,
                },
            ));
        }

        let (key, kind) = association.expect("the process association was checked above");
        let (route, finalization, input) = pending.into_parts();
        let settled = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| match service
                    .retain_a2a_delegate_outcome_continuable(input)
                {
                    Ok(retained) => service
                        .finish_a2a_delegate(finalization, retained)
                        .map_err(ResidentProcessA2aDelegateLowerRetentionFailure::Finalization),
                    Err(failure) => {
                        Err(ResidentProcessA2aDelegateLowerRetentionFailure::Retention(
                            Box::new(ResidentProcessA2aDelegateLowerRetentionCustody {
                                key: key.clone(),
                                kind,
                                finalization,
                                failure,
                            }),
                        ))
                    }
                },
            )
            .expect("the route selector and access were checked before A2A retention");
        let retained = match settled {
            Ok(retained) => retained,
            Err(ResidentProcessA2aDelegateLowerRetentionFailure::Retention(failure)) => {
                let ResidentProcessA2aDelegateLowerRetentionCustody {
                    key,
                    kind,
                    finalization,
                    failure,
                } = *failure;
                return Err(Box::new(
                    ResidentProcessA2aDelegateRetentionFailure::Retention {
                        route,
                        key,
                        kind,
                        finalization: Box::new(finalization),
                        failure: Box::new(failure),
                        reason: None,
                    },
                ));
            }
            Err(ResidentProcessA2aDelegateLowerRetentionFailure::Finalization(rejection)) => {
                return Err(Box::new(
                    ResidentProcessA2aDelegateRetentionFailure::Finalization {
                        route,
                        rejection: Box::new(rejection),
                    },
                ));
            }
        };
        self.release_retained_a2a_delegate(route, key, kind, retained)
    }

    /// Continue one exact physical outcome-custody failure without carrier access.
    pub(super) fn continue_a2a_delegate_outcome_custody<'a, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateRetentionFailure<'a>,
    ) -> Result<RetainedA2aOutcome, Box<ResidentProcessA2aDelegateRetentionFailure<'a>>>
    where
        Authority: AmbientSessionAuthorityPort,
    {
        let ResidentProcessA2aDelegateRetentionFailure::Retention {
            route,
            key,
            kind,
            finalization,
            failure,
            ..
        } = pending
        else {
            return Err(Box::new(pending));
        };
        let selector = route.selector.clone();
        let association_matches = route.key == key
            && finalization.matches_process_retention(&route.owner_instance, &key, kind)
            && failure.matches_process_association(&key, kind)
            && self.validate_checked_out_a2a_route(&route, access.store);
        if !association_matches {
            return Err(Box::new(
                ResidentProcessA2aDelegateRetentionFailure::Retention {
                    route,
                    key,
                    kind,
                    finalization,
                    failure,
                    reason: Some(ResidentProcessHostSelectionError::A2aProcessRouteAssociation),
                },
            ));
        }
        let continued = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| match service
                    .continue_a2a_delegate_outcome_custody(*failure)
                {
                    Ok(retained) => service
                        .finish_a2a_delegate(*finalization, retained)
                        .map_err(ResidentProcessA2aDelegateLowerRetentionFailure::Finalization),
                    Err(failure) => {
                        Err(ResidentProcessA2aDelegateLowerRetentionFailure::Retention(
                            Box::new(ResidentProcessA2aDelegateLowerRetentionCustody {
                                key: key.clone(),
                                kind,
                                finalization: *finalization,
                                failure,
                            }),
                        ))
                    }
                },
            )
            .expect("the checked-out route and access were checked before custody continuation");
        let retained = match continued {
            Ok(retained) => retained,
            Err(ResidentProcessA2aDelegateLowerRetentionFailure::Retention(failure)) => {
                let ResidentProcessA2aDelegateLowerRetentionCustody {
                    key,
                    kind,
                    finalization,
                    failure,
                } = *failure;
                return Err(Box::new(
                    ResidentProcessA2aDelegateRetentionFailure::Retention {
                        route,
                        key,
                        kind,
                        finalization: Box::new(finalization),
                        failure: Box::new(failure),
                        reason: None,
                    },
                ));
            }
            Err(ResidentProcessA2aDelegateLowerRetentionFailure::Finalization(rejection)) => {
                return Err(Box::new(
                    ResidentProcessA2aDelegateRetentionFailure::Finalization {
                        route,
                        rejection: Box::new(rejection),
                    },
                ));
            }
        };
        self.release_retained_a2a_delegate(route, key, kind, retained)
    }

    /// Finish a branch after its existing custody owner resolves retention.
    pub(super) fn finish_retained_a2a_delegate<'a, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        route: ResidentProcessA2aDelegateRouteCustody,
        finalization: ResidentA2aDelegateFinalization,
        retained: RetainedA2aOutcome,
    ) -> Result<RetainedA2aOutcome, Box<ResidentProcessA2aDelegateRetentionFailure<'a>>> {
        let selector = route.selector.clone();
        let kind = retained.process_kind();
        let key = route.key.clone();
        if !self.validate_checked_out_a2a_route(&route, access.store)
            || !finalization.matches_process_retention(&route.owner_instance, &key, kind)
        {
            return Err(Box::new(
                ResidentProcessA2aDelegateRetentionFailure::Finalization {
                    route,
                    rejection: Box::new(ResidentA2aDelegateFinalizationRejection {
                        finalization: Box::new(finalization),
                        retained: Box::new(retained),
                        reason: crate::resident::a2a_delegate_task_service::ResidentA2aDelegateFinalizationFailure::Association,
                    }),
                },
            ));
        }
        let settled = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.finish_a2a_delegate(finalization, retained)
                },
            )
            .expect("the checked-out route and access were checked before A2A finalization");
        match settled {
            Ok(retained) => self.release_retained_a2a_delegate(route, key, kind, retained),
            Err(rejection) => Err(Box::new(
                ResidentProcessA2aDelegateRetentionFailure::Finalization {
                    route,
                    rejection: Box::new(rejection),
                },
            )),
        }
    }

    /// Retry only task-capacity release after durable outcome custody succeeded.
    pub(super) fn retry_a2a_delegate_finalization<'a, Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateRetentionFailure<'a>,
    ) -> Result<RetainedA2aOutcome, Box<ResidentProcessA2aDelegateRetentionFailure<'a>>> {
        let ResidentProcessA2aDelegateRetentionFailure::Finalization { route, rejection } = pending
        else {
            return Err(Box::new(pending));
        };
        self.finish_retained_a2a_delegate(
            access,
            authority,
            route,
            *rejection.finalization,
            *rejection.retained,
        )
    }

    /// Retry only process-route release after all durable handling completed.
    pub(super) fn retry_a2a_delegate_route_release<'a>(
        &mut self,
        pending: ResidentProcessA2aDelegateRetentionFailure<'a>,
    ) -> Result<RetainedA2aOutcome, Box<ResidentProcessA2aDelegateRetentionFailure<'a>>> {
        let ResidentProcessA2aDelegateRetentionFailure::Route {
            route,
            key,
            kind,
            retained,
        } = pending
        else {
            return Err(Box::new(pending));
        };
        self.release_retained_a2a_delegate(*route, key, kind, *retained)
    }

    /// Return an orphaned process-local route to durable restart recovery.
    ///
    /// This does not classify the send as unstarted, replay it, settle it, or
    /// change aggregate task accounting. It removes only the stale local route
    /// after the lower task owner has already reported that exact key absent.
    pub(super) fn relinquish_unrouted_a2a_delegate_to_recovery(
        &mut self,
        access_store: &pale_ports::AuthorityStoreCoordinate,
        pending: ResidentProcessA2aDelegatePoll,
    ) -> Result<(), Box<ResidentProcessA2aDelegatePoll>> {
        let ResidentProcessA2aDelegatePoll::Unrouted { selector, key } = pending else {
            return Err(Box::new(pending));
        };
        if key.store() != access_store
            || self.a2a_delegate_routes.selector(&key) != Some(&selector)
            || !self
                .a2a_delegate_routes
                .relinquish_unrouted_to_recovery(&key, &selector)
        {
            return Err(Box::new(ResidentProcessA2aDelegatePoll::Unrouted {
                selector,
                key,
            }));
        }
        Ok(())
    }

    fn release_retained_a2a_delegate<'a>(
        &mut self,
        route: ResidentProcessA2aDelegateRouteCustody,
        key: ResidentOperationTaskKey,
        kind: A2aOutcomeRetentionKind,
        retained: RetainedA2aOutcome,
    ) -> Result<RetainedA2aOutcome, Box<ResidentProcessA2aDelegateRetentionFailure<'a>>> {
        if route.key != key || !retained.matches_process_association(&key, kind) {
            return Err(Box::new(
                ResidentProcessA2aDelegateRetentionFailure::Route {
                    route: Box::new(route),
                    key,
                    kind,
                    retained: Box::new(retained),
                },
            ));
        }
        match self.a2a_delegate_routes.release(route) {
            Ok(()) => Ok(retained),
            Err(route) => Err(Box::new(
                ResidentProcessA2aDelegateRetentionFailure::Route {
                    route,
                    key,
                    kind,
                    retained: Box::new(retained),
                },
            )),
        }
    }

    fn join_routed_a2a_launch<'operation, 'call>(
        &mut self,
        key: ResidentOperationTaskKey,
        selector: ResidentDeploymentAuthoritySelector,
        launch: Result<
            ResidentA2aDelegateLaunch,
            ResidentA2aDelegateLaunchFailure<'operation, 'call>,
        >,
    ) -> Result<
        ResidentProcessA2aDelegateLaunch,
        Box<ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>>,
    > {
        match launch {
            Ok(ResidentA2aDelegateLaunch::Awaiting(actual)) => {
                debug_assert_eq!(actual, key);
                Ok(ResidentProcessA2aDelegateLaunch::Awaiting(key))
            }
            Ok(ResidentA2aDelegateLaunch::Completed(completion)) => {
                let route = self
                    .a2a_delegate_routes
                    .check_out(&key)
                    .expect("the active A2A route must remain available");
                Ok(ResidentProcessA2aDelegateLaunch::Completed(
                    ResidentProcessA2aDelegateCompletion { route, completion },
                ))
            }
            Ok(ResidentA2aDelegateLaunch::Diagnostic(diagnostic)) => {
                let route = self
                    .a2a_delegate_routes
                    .check_out(&key)
                    .expect("the active A2A route must remain available");
                Ok(ResidentProcessA2aDelegateLaunch::Diagnostic(
                    ResidentProcessA2aDelegateDiagnostic { route, diagnostic },
                ))
            }
            Err(failure) if lower_launch_proves_unstarted(&failure) => {
                let removed = self.a2a_delegate_routes.remove_unstarted(&key, &selector);
                debug_assert!(removed);
                Err(Box::new(ResidentProcessA2aDelegateLaunchFailure::Rejected(
                    Box::new(failure),
                )))
            }
            Err(failure) => Err(Box::new(routed_launch_failure(key, selector, failure))),
        }
    }

    pub(super) fn join_routed_a2a_poll(
        &mut self,
        key: ResidentOperationTaskKey,
        selector: ResidentDeploymentAuthoritySelector,
        poll: ResidentA2aDelegatePoll,
    ) -> ResidentProcessA2aDelegatePoll {
        match poll {
            ResidentA2aDelegatePoll::Pending => ResidentProcessA2aDelegatePoll::Pending(key),
            ResidentA2aDelegatePoll::Completed(completion) => {
                let route = self
                    .a2a_delegate_routes
                    .check_out_for(&key, &selector)
                    .expect("the active A2A route must remain available");
                ResidentProcessA2aDelegatePoll::Completed(ResidentProcessA2aDelegateCompletion {
                    route,
                    completion,
                })
            }
            ResidentA2aDelegatePoll::Diagnostic(diagnostic) => {
                let route = self
                    .a2a_delegate_routes
                    .check_out_for(&key, &selector)
                    .expect("the active A2A route must remain available");
                ResidentProcessA2aDelegatePoll::Diagnostic(ResidentProcessA2aDelegateDiagnostic {
                    route,
                    diagnostic,
                })
            }
            ResidentA2aDelegatePoll::SettlementRejected(rejection) => {
                ResidentProcessA2aDelegatePoll::SettlementRejected {
                    selector,
                    rejection,
                }
            }
            ResidentA2aDelegatePoll::NotFound => {
                ResidentProcessA2aDelegatePoll::Unrouted { selector, key }
            }
        }
    }

    fn validate_active_a2a_route(
        &self,
        key: &ResidentOperationTaskKey,
        selector: &ResidentDeploymentAuthoritySelector,
        access_store: &pale_ports::AuthorityStoreCoordinate,
    ) -> bool {
        self.a2a_delegate_routes.selector(key) == Some(selector)
            && key.store() == access_store
            && self
                .deployment_authority_index(selector)
                .is_ok_and(|index| self.deployments[index].store() == access_store)
    }

    fn validate_checked_out_a2a_route(
        &self,
        route: &ResidentProcessA2aDelegateRouteCustody,
        access_store: &pale_ports::AuthorityStoreCoordinate,
    ) -> bool {
        route
            .owner_instance
            .is_same(&self.a2a_delegate_routes.owner_instance)
            && route.key.store() == access_store
            && self
                .deployment_authority_index(&route.selector)
                .is_ok_and(|index| self.deployments[index].store() == access_store)
    }

    fn validate_a2a_delegate_restart(
        &self,
        attempt: &ResidentProcessRecoveredAttempt,
        access_store: &pale_ports::AuthorityStoreCoordinate,
        input: &A2aSendRecoveryInput<'_>,
    ) -> Result<ResidentOperationTaskKey, ResidentProcessHostSelectionError> {
        let selector = &attempt.start.selector;
        let index = self.deployment_authority_index(selector)?;
        if self.deployments[index].store() != access_store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        let operation = attempt.start.control.operation().operation();
        if attempt.dispatch != super::restart::ResidentProcessAttemptObservation::CurrentHead
            || operation.handle().1.operation_class() != &OperationClassV1::Delegate
            || !selector.matches_operation(operation)
            || input.recovered.store() != access_store
        {
            return Err(ResidentProcessHostSelectionError::A2aOperationAssociation);
        }
        let historical_operation = input.history.operation;
        if !selector.matches_operation(historical_operation)
            || operation.handle().0 != historical_operation.handle().0
            || operation
                .handle()
                .1
                .canonical_operation_handle_sha256()
                .as_str()
                != historical_operation
                    .handle()
                    .1
                    .canonical_operation_handle_sha256()
                    .as_str()
        {
            return Err(ResidentProcessHostSelectionError::A2aOperationAssociation);
        }
        let selected = input.recovered.selected();
        if attempt.start.control.observation().selected() != selected
            || input.history.dispatched != selected.1
            || input.history.selection.coordinate() != attempt.selection.coordinate()
        {
            return Err(ResidentProcessHostSelectionError::A2aOperationAssociation);
        }
        let record_sha256 =
            ArtifactDigest::parse(selected.1.canonical_operation_observation_sha256().as_str())
                .map_err(|_| ResidentProcessHostSelectionError::A2aOperationAssociation)?;
        Ok(ResidentOperationTaskKey::new(
            access_store.clone(),
            ImmutableRecordReferenceV1 {
                record_ref: selected.0.clone(),
                record_sha256,
            },
        ))
    }

    fn validate_a2a_delegate_launch_in(
        application: &ResidentApplication<'_>,
        selector: &ResidentDeploymentAuthoritySelector,
        access_store: &pale_ports::AuthorityStoreCoordinate,
        input: &A2aDelegateLaunchInput<'_, '_>,
    ) -> Result<ResidentOperationTaskKey, ResidentProcessHostSelectionError> {
        let selected_store = application.store();
        if selected_store != access_store || input.send.dispatch.store() != selected_store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        if !selector.matches_operation(input.send.dispatch.operation()) {
            return Err(ResidentProcessHostSelectionError::A2aOperationAssociation);
        }
        if !selector.matches_current_session(input.send.current) {
            return Err(ResidentProcessHostSelectionError::A2aCurrentAuthority);
        }
        let route = input.send.route.1;
        if route.verify().is_err()
            || route.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
            || route.deployment_runtime_authority_sha256().as_str()
                != selector.runtime_authority_sha256.as_str()
            || route.issuer_principal_id() != &selector.principal_id
        {
            return Err(ResidentProcessHostSelectionError::A2aRouteAuthority);
        }
        if input.checkpoint_authority.principal_id != &selector.principal_id
            || input.checkpoint_authority.control_store_id != selected_store.store_id()
            || input.checkpoint_authority.control_incarnation_id != selected_store.incarnation_id()
        {
            return Err(ResidentProcessHostSelectionError::CheckpointAuthorityAssociation);
        }
        let key = input
            .send
            .dispatch
            .task_key()
            .map_err(|_| ResidentProcessHostSelectionError::A2aOperationAssociation)?;
        if key.store() != selected_store {
            return Err(ResidentProcessHostSelectionError::A2aOperationAssociation);
        }
        Ok(key)
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/a2a_delegate_support.rs"]
pub(in crate::resident) mod test_support;

fn lower_launch_proves_unstarted(failure: &ResidentA2aDelegateLaunchFailure<'_, '_>) -> bool {
    matches!(
        failure,
        ResidentA2aDelegateLaunchFailure::Association { .. }
            | ResidentA2aDelegateLaunchFailure::Reservation { .. }
            | ResidentA2aDelegateLaunchFailure::Launch(_)
    )
}

fn routed_launch_failure<'operation, 'call>(
    key: ResidentOperationTaskKey,
    selector: ResidentDeploymentAuthoritySelector,
    failure: ResidentA2aDelegateLaunchFailure<'operation, 'call>,
) -> ResidentProcessA2aDelegateLaunchFailure<'operation, 'call> {
    let selector = Box::new(selector);
    match failure {
        failure @ ResidentA2aDelegateLaunchFailure::Rollback { .. } => {
            ResidentProcessA2aDelegateLaunchFailure::Rollback {
                key,
                selector,
                failure: Box::new(failure),
            }
        }
        failure @ ResidentA2aDelegateLaunchFailure::Custody(_) => {
            ResidentProcessA2aDelegateLaunchFailure::Custody {
                key,
                selector,
                failure: Box::new(failure),
            }
        }
        failure @ ResidentA2aDelegateLaunchFailure::Unopened(_) => {
            ResidentProcessA2aDelegateLaunchFailure::Unopened {
                key,
                selector,
                failure: Box::new(failure),
            }
        }
        failure @ ResidentA2aDelegateLaunchFailure::StartedAssociation { .. } => {
            ResidentProcessA2aDelegateLaunchFailure::StartedAssociation {
                key,
                selector,
                failure: Box::new(failure),
            }
        }
        failure @ ResidentA2aDelegateLaunchFailure::OfferSettlement(_) => {
            ResidentProcessA2aDelegateLaunchFailure::OfferSettlement {
                key,
                selector,
                failure: Box::new(failure),
            }
        }
        ResidentA2aDelegateLaunchFailure::Association { .. }
        | ResidentA2aDelegateLaunchFailure::Reservation { .. }
        | ResidentA2aDelegateLaunchFailure::Launch(_) => {
            unreachable!("unstarted lower failures release their process route")
        }
    }
}

fn unresolved_from_recovery_failure<'operation, 'call>(
    failure: ResidentA2aDelegateUnresolvedRecoveryFailure<'operation, 'call, '_>,
) -> Box<
    crate::resident::a2a_delegate_task_service::ResidentA2aDelegateUnresolvedLaunch<
        'operation,
        'call,
    >,
> {
    match failure {
        ResidentA2aDelegateUnresolvedRecoveryFailure::Reservation { unresolved, .. }
        | ResidentA2aDelegateUnresolvedRecoveryFailure::Custody(unresolved)
        | ResidentA2aDelegateUnresolvedRecoveryFailure::Association { unresolved, .. }
        | ResidentA2aDelegateUnresolvedRecoveryFailure::Recovery { unresolved, .. } => unresolved,
        ResidentA2aDelegateUnresolvedRecoveryFailure::LossReservation(_) => {
            unreachable!("loss rejection has its own process continuation")
        }
    }
}

fn diagnostic_from_recovery_failure(
    failure: ResidentA2aDelegateDiagnosticRecoveryFailure,
) -> ResidentA2aDelegateDiagnostic {
    match failure {
        ResidentA2aDelegateDiagnosticRecoveryFailure::Reservation { diagnostic, .. }
        | ResidentA2aDelegateDiagnosticRecoveryFailure::StillInadmissible(diagnostic) => {
            *diagnostic
        }
    }
}
