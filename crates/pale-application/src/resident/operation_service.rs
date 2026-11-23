//! Private composition for resident operation journeys.
//!
//! The service joins application use cases with their existing authority,
//! custody, task, carrier, and recovery owners. It adds no semantic policy.
//! Only a request that wins before dispatch can complete locally; every
//! post-dispatch request remains nonterminal until owner-native evidence arrives.

mod a2a_cancel_task;
mod a2a_readback;
pub(in crate::resident) use a2a_cancel_task::{
    ResidentA2aCancelTaskDispositionRetentionFailure,
    ResidentA2aCancelTaskNonDispatchableReplayCustody,
    ResidentA2aCancelTaskNonDispatchableReplayReleaseFailure,
    ResidentA2aCancelTaskPreparationFailure, ResidentA2aCancelTaskReadbackRetentionFailure,
    ResidentA2aCancelTaskRequestCustodyFailure, ResidentPreparedA2aCancelTask,
    ResidentRetainedUnopenedA2aCancelTask, ResidentSelectedA2aCancelTaskDisposition,
    ResidentSelectedA2aCancelTaskReadback,
};
pub(in crate::resident) use a2a_readback::{
    ResidentA2aGetTaskLocalDispositionCustodyFailure, ResidentA2aGetTaskLocalDispositionInput,
    ResidentA2aGetTaskLocalDispositionReleaseFailure,
    ResidentA2aGetTaskNonDispatchableReplayCustody,
    ResidentA2aGetTaskNonDispatchableReplayReleaseFailure, ResidentA2aGetTaskPreparationFailure,
    ResidentA2aGetTaskReleaseFailure, ResidentA2aGetTaskRequestCustodyFailure,
    ResidentA2aGetTaskRetentionFailure, ResidentPreparedA2aGetTask,
    ResidentReleasedA2aGetTaskLocalDisposition, ResidentRetainedA2aGetTask,
    ResidentRetainedA2aGetTaskLocalDisposition, ResidentRetainedUnopenedA2aGetTask,
};

use pale_core::records::*;
use pale_ports::{
    A2aExchangePort, A2aSendMessageEvidencePort, AccumulatorPreparationLimits,
    AntiRollbackAnchorPort, OperationCustodyPort, SessionLeaseCustodyPort, WallClockSourcePort,
};

use super::a2a_cancel_task_owner::ResidentA2aCancelTaskOwner;
use super::a2a_delegate_task_service::{
    ResidentA2aDelegateCancellationContinuation, ResidentA2aDelegateCustodyContinuationFailure,
    ResidentA2aDelegateDiagnostic, ResidentA2aDelegateDiagnosticRecoveryFailure,
    ResidentA2aDelegateDispatchOwner, ResidentA2aDelegateDispatchOwnerSelection,
    ResidentA2aDelegateFinalization, ResidentA2aDelegateFinalizationRejection,
    ResidentA2aDelegateLaunch, ResidentA2aDelegateLaunchFailure,
    ResidentA2aDelegateOfferedCompletionRejection, ResidentA2aDelegatePoll,
    ResidentA2aDelegateRecoveredOfferRejection, ResidentA2aDelegateRecoveryFailure,
    ResidentA2aDelegateSettlementRejection, ResidentA2aDelegateTaskService,
    ResidentA2aDelegateUnopenedLaunch, ResidentA2aDelegateUnopenedRetryFailure,
    ResidentA2aDelegateUnresolvedLaunch, ResidentA2aDelegateUnresolvedRecoveryFailure,
    ResidentRecoveredA2aDelegate,
};
use super::a2a_get_task_owner::ResidentA2aGetTaskOwner;

use super::consultation::{
    apply_consultation_supervision_decision, commit_selected_consultation_stop,
    continue_consultation_stop_custody, continue_consultation_terminal_custody,
    join_consultation_terminal, retry_consultation_terminal_release,
    settle_consultation_nonterminal, CompletedConsultationCollection,
    CompletedResidentConsultationNonterminal, CompletedResidentConsultationTerminal,
    ConsultationCancellationFailure, ConsultationCancellationInput, ConsultationChildJoinInput,
    ConsultationChildRetentionOutcome, ConsultationCollectionCommitFailure,
    ConsultationCollectionError, ConsultationStopInput, ConsultationSuccessorCommitInput,
    PreparedResidentConsultationTaskLoss, PreparedResidentConsultationTaskResume,
    RecoveredConsultationCollection, ResidentConsultationBatchCoordinate,
    ResidentConsultationBatchOpenError, ResidentConsultationBatchRetirement,
    ResidentConsultationCancellationNotification, ResidentConsultationCompletionReady,
    ResidentConsultationDispatchOwner, ResidentConsultationDispatchOwnerSelection,
    ResidentConsultationJoinedTerminal, ResidentConsultationNonterminalSettlementFailure,
    ResidentConsultationOpenFailure, ResidentConsultationOpenInput,
    ResidentConsultationPolicyFrontier, ResidentConsultationPrePolicyTurn,
    ResidentConsultationPreparedResumeFailure, ResidentConsultationStopCommitFailure,
    ResidentConsultationStopCustody, ResidentConsultationStopSelected,
    ResidentConsultationSupervisionDecision, ResidentConsultationTaskLimits,
    ResidentConsultationTaskLossCommitFailure, ResidentConsultationTaskLossOffer,
    ResidentConsultationTaskLossPreparationFailure, ResidentConsultationTaskOwner,
    ResidentConsultationTaskOwnerAcceptanceFailure, ResidentConsultationTaskOwnerBuildError,
    ResidentConsultationTaskOwnerLimits, ResidentConsultationTaskOwnerPoll,
    ResidentConsultationTaskOwnerPollError, ResidentConsultationTaskResumePreparationFailure,
    ResidentConsultationTaskRetry, ResidentConsultationTaskRetryFailure,
    ResidentConsultationTerminalCustody, ResidentConsultationTerminalJoinFailure,
    ResidentConsultationTurnDisposition,
};
use super::local_operation_coordinator::{
    ResidentLocalOperationCoordinator, ResidentLocalOperationCustodyFailure,
    ResidentLocalOperationStartFailure, ResidentLocalOperationStartInput,
};
use super::local_operation_start::IssuedLocalOperationCheckpointFailure;
use super::operation_dispatch::{
    complete_prepared_checkpoint, retain_a2a_outcome, retain_a2a_outcome_continuable,
    A2aDelegateLaunchInput, A2aOutcomeRetentionFailure, A2aOutcomeRetentionInput,
    A2aSendRecoveryAdmission, A2aSendRecoveryError, A2aSendRecoveryInput,
    ContinuableA2aOutcomeRetentionFailure, OperationAppendFailure, OperationCheckpointInput,
    ProviderOperationDispatchInput, ProviderOperationInvokeInput, ProviderOperationInvokeResult,
    ProviderOperationRetentionFailure, RecoveredProviderDispatchRelease, RetainedA2aOutcome,
    RetainedProviderOperationDispatch,
};
use super::operation_observation_custody::reconciliation_append::{
    CompletedCurrentHeadReconciliation, CurrentHeadReconciliationAppendFailure,
    CurrentHeadReconciliationAppendInput,
};
use super::provider_task_service::{
    ResidentCompletedProviderTask, ResidentLostProviderTask, ResidentProviderCompletionPoll,
    ResidentProviderDispatchOwner, ResidentProviderDispatchOwnerSelection,
    ResidentProviderRecordingConflict, ResidentProviderRecordingConflictRejection,
    ResidentProviderRecordingContinuation, ResidentProviderRecordingContinuationRejection,
    ResidentProviderRecordingCustody, ResidentProviderRecordingCustodyRejection,
    ResidentProviderRecordingFinalization, ResidentProviderRecordingFinalizationRejection,
    ResidentProviderTaskAcceptanceFailure, ResidentProviderTaskPoll,
    ResidentProviderTaskRecoveryFailure, ResidentProviderTaskService,
    ResidentRecoveredProviderTaskAcceptanceFailure,
};
use super::task_owner::{
    ResidentOperationTaskCancellationNotification, ResidentOperationTaskCancellationState,
    ResidentOperationTaskCompletion, ResidentOperationTaskExecutor, ResidentOperationTaskKey,
    ResidentOperationTaskLimits, ResidentOperationTaskOwnerBuildError,
    ResidentOperationTaskOwnerInstance, ResidentProviderTaskRefusal,
};
use super::task_reservation::{
    ResidentAggregateTaskLimits, ResidentAggregateTaskReservationBuildError,
    ResidentAggregateTaskReservations, ResidentTaskReservationOfferFailure,
    ResidentTaskReservationSettlementTransitionFailure,
};
use super::{
    observe_local_operation, recover_operation_control, AmbientSessionAuthorityPort,
    CompletedOperationObservationCustody, DispatchedOperationCancellationRequestInput,
    IssuedLocalOperation, LocalOperationCancellationCompletionInput,
    LocalOperationCancellationRequestInput, LocalOperationObservationAdmission,
    LocalOperationObservationError, LocalOperationObservationInput, OperationAttemptHistoryInput,
    OperationCancellationCompletionError, OperationCancellationDisposition,
    OperationCancellationRequestError, OperationControlAuthorizationContext, OperationControlError,
    OperationControlObservation, OperationControlRecoveryAdmission, OperationControlRecoveryError,
    OperationControlRecoveryInput, OperationObservationAuthorityStoreResolver,
    PreparedLocalOperationCheckpoint, RecoveredOperationControl, RecoveredOperationObservation,
};
use crate::provider_invocation::ProviderInvocationAdmissionContext;
use crate::provider_invocation::{
    ProviderInvocationAdmissionError, ProviderInvocationEvidenceAdmission,
};

type ProviderTaskOutput =
    Result<pale_ports::ProviderInvocationObservation, ProviderInvocationAdmissionError>;

type CancellationAuthorization<'a> =
    dyn FnMut(OperationControlAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1> + 'a;
type RuntimeAuthorityRotationAdmission<'a> = dyn FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;
type ObservationSigning<'a> = dyn FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;
type CheckpointSigning<'a> = dyn FnMut(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;

/// Inputs that identify and bound one durable cancellation request append.
pub(in crate::resident) struct ResidentOperationCancellationInput<'a> {
    /// Protected recovery inputs for the exact current operation head.
    pub recovery: OperationControlRecoveryInput<'a>,
    /// Fresh caller-signed cancellation request.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Time at which current cancellation authority is evaluated.
    pub authorized_at: UnixTimeV1,
    /// Immutable reference for the cancellation request record.
    pub request_ref: ResidentRecordRefV1,
    /// Immutable reference for the successor cancellation observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Current admitted signer for the successor observation.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh checkpoint identity for the append.
    pub checkpoint: OperationCheckpointInput,
    /// Time assigned to the successor checkpoint.
    pub checkpoint_issued_at: UnixTimeV1,
    /// Exact selected checkpoint schema digest.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Current admitted checkpoint signer.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Native accumulator work and memory ceilings.
    pub accumulator_limits: AccumulatorPreparationLimits,
}

/// Fresh terminal identities and bounds used only when cancellation wins before dispatch.
pub(in crate::resident) struct ResidentLocalCancellationCompletionInput {
    /// Immutable reference for the terminal cancellation diagnostic.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Immutable reference for the terminal cancellation observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Time at which the absence of dispatch was established.
    pub established_at: UnixTimeV1,
    /// Fresh checkpoint identity for the terminal append.
    pub checkpoint: OperationCheckpointInput,
    /// Time assigned to the terminal checkpoint.
    pub checkpoint_issued_at: UnixTimeV1,
    /// Maximum canonical diagnostic bytes.
    pub maximum_diagnostic_bytes: usize,
}

/// Existing semantic and signing owners used by one cancellation request.
pub(in crate::resident) struct ResidentOperationCancellationAdmission<'a> {
    /// Historical request, operation, observation, checkpoint, and signature admission.
    pub recovery: OperationControlRecoveryAdmission<'a>,
    /// Present cancellation-authority decision for the recovered current head.
    pub authorize: &'a mut CancellationAuthorization<'a>,
    /// Resolve the exact physical store for the observation append.
    pub resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    /// Admit any current runtime-authority rotation.
    pub admit_rotation: &'a mut RuntimeAuthorityRotationAdmission<'a>,
    /// Sign the successor cancellation observation.
    pub sign_observation: &'a mut ObservationSigning<'a>,
    /// Sign the successor control checkpoint.
    pub sign_checkpoint: &'a mut CheckpointSigning<'a>,
}

struct ResidentRecoveredCancellationInput<'a> {
    request: &'a CommonRequestEnvelopeV1,
    authorized_at: UnixTimeV1,
    append: CancellationAppendInput<'a>,
}

struct ResidentRecoveredCancellationAdmission<'a> {
    authorize: &'a mut CancellationAuthorization<'a>,
    append: CancellationAppendAdmission<'a>,
}

fn split_cancellation_input(
    input: ResidentOperationCancellationInput<'_>,
) -> (
    OperationControlRecoveryInput<'_>,
    ResidentRecoveredCancellationInput<'_>,
) {
    let ResidentOperationCancellationInput {
        recovery,
        request,
        authorized_at,
        request_ref,
        observation_ref,
        signing,
        checkpoint,
        checkpoint_issued_at,
        checkpoint_schema,
        checkpoint_authority,
        accumulator_limits,
    } = input;
    (
        recovery,
        ResidentRecoveredCancellationInput {
            request,
            authorized_at,
            append: CancellationAppendInput {
                request_ref,
                observation_ref,
                signing,
                checkpoint,
                checkpoint_issued_at,
                checkpoint_schema,
                checkpoint_authority,
                accumulator_limits,
            },
        },
    )
}

fn split_cancellation_admission(
    admission: ResidentOperationCancellationAdmission<'_>,
) -> (
    OperationControlRecoveryAdmission<'_>,
    ResidentRecoveredCancellationAdmission<'_>,
) {
    let ResidentOperationCancellationAdmission {
        recovery,
        authorize,
        resolve_store,
        admit_rotation,
        sign_observation,
        sign_checkpoint,
    } = admission;
    (
        recovery,
        ResidentRecoveredCancellationAdmission {
            authorize,
            append: CancellationAppendAdmission {
                resolve_store,
                admit_rotation,
                sign_observation,
                sign_checkpoint,
            },
        },
    )
}

/// Exact successor identities and bounds for one authorized cancellation append.
pub(in crate::resident) struct CancellationAppendInput<'a> {
    /// Immutable reference for the retained cancellation request.
    pub(in crate::resident) request_ref: ResidentRecordRefV1,
    /// Immutable reference for the successor observation.
    pub(in crate::resident) observation_ref: ResidentRecordRefV1,
    /// Admitted signer for the successor observation.
    pub(in crate::resident) signing: SignedResidentAdmissionContextV1<'a>,
    /// Identity of the successor checkpoint.
    pub(in crate::resident) checkpoint: OperationCheckpointInput,
    /// Time assigned to the successor checkpoint.
    pub(in crate::resident) checkpoint_issued_at: UnixTimeV1,
    /// Admitted checkpoint schema identity.
    pub(in crate::resident) checkpoint_schema: &'a SchemaDigest,
    /// Admitted signer for the successor checkpoint.
    pub(in crate::resident) checkpoint_authority:
        &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Native accumulator work and memory ceilings.
    pub(in crate::resident) accumulator_limits: AccumulatorPreparationLimits,
}

/// Existing store, authority, and signing owners for an authorized append.
pub(in crate::resident) struct CancellationAppendAdmission<'a> {
    /// Resolve the physical store that owns the selected observation.
    pub(in crate::resident) resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    /// Admit any runtime-authority rotation before observation issuance.
    pub(in crate::resident) admit_rotation: &'a mut RuntimeAuthorityRotationAdmission<'a>,
    /// Sign the successor operation observation.
    pub(in crate::resident) sign_observation: &'a mut ObservationSigning<'a>,
    /// Sign the successor control checkpoint.
    pub(in crate::resident) sign_checkpoint: &'a mut CheckpointSigning<'a>,
}

/// One exact existing state returned without another cancellation append.
pub(in crate::resident) struct ResidentOperationExistingState {
    reference: ResidentRecordRefV1,
    observation: OperationObservationV1,
}

impl ResidentOperationExistingState {
    /// Borrow the existing signed observation and its storage reference.
    #[must_use]
    pub(in crate::resident) const fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.reference, &self.observation)
    }
}

/// Honest result of one freshly authorized cancellation request.
pub(in crate::resident) enum ResidentOperationCancellationOutcome {
    /// A terminal observation already exists; no cancellation write occurred.
    Established(ResidentOperationExistingState),
    /// The exact cancellation request was already retained; it was not replayed.
    AlreadyRequested(ImmutableRecordReferenceV1),
    /// A before-dispatch request reached custody for a separate recovery owner.
    LocalRequestRetained {
        /// Completed physical custody for the nonterminal request observation.
        custody: CompletedOperationObservationCustody,
        /// Exact independently anchored request checkpoint.
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    },
    /// A before-dispatch request and its terminal consequence reached custody.
    LocallyCancelled {
        /// Completed physical custody for the nonterminal request observation.
        request_custody: CompletedOperationObservationCustody,
        /// Exact independently anchored request checkpoint.
        request_checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        /// Completed physical custody for the terminal cancellation observation.
        terminal_custody: CompletedOperationObservationCustody,
        /// Exact independently anchored terminal checkpoint.
        terminal_checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        /// Exact terminal cancellation observation.
        observation: Box<(ResidentRecordRefV1, OperationObservationV1)>,
    },
    /// A post-dispatch request reached custody before process-local notification.
    DispatchedRequestRetained {
        /// Completed physical custody for the nonterminal request observation.
        custody: CompletedOperationObservationCustody,
        /// Exact independently anchored successor checkpoint.
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        /// Process-local notification result; never proof that work stopped.
        notification: ResidentOperationTaskCancellationNotification,
    },
    /// A durable request stopped one exact local A2A response read.
    A2aDelegateReadStopped {
        /// Completed physical custody for the nonterminal request observation.
        custody: CompletedOperationObservationCustody,
        /// Exact independently anchored successor checkpoint.
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        /// Non-replayable uncertainty that still owns aggregate capacity.
        continuation: ResidentA2aDelegateCancellationContinuation,
    },
    /// Collection is frozen; this service did not invent a successor policy.
    ConsultationCollected(ImmutableRecordReferenceV1),
    /// Recovery, uncertainty, or expiry requires reconciliation, not replay.
    ReconciliationOnly(ResidentOperationExistingState),
    /// No process-local provider, Consult child, or A2A Delegate owns the dispatch.
    DispatchOwnerRequired(ResidentOperationExistingState),
}

/// Why the service could not retain an authorized cancellation request.
pub(in crate::resident) enum ResidentOperationCancellationFailure {
    /// Protected operation or current-head recovery failed.
    Recovery(OperationControlRecoveryError),
    /// Fresh current cancellation authority was rejected.
    Authorization(OperationControlError),
    /// Request projection or observation issuance failed.
    Request(OperationCancellationRequestError),
    /// Terminal local cancellation preparation failed after request custody.
    Completion(OperationCancellationCompletionError),
    /// Checkpoint preparation, commitment, or independent anchoring failed.
    Custody(OperationAppendFailure),
    /// Post-dispatch custody stopped after preserving its exact local owner.
    DispatchedCustody(Box<ResidentDispatchedCancellationCustody>),
    /// The current dispatched observation could not form its immutable task key.
    Association,
    /// More than one process-local task owner claimed the exact dispatch.
    DispatchOwnerConflict,
    /// An open consultation must continue through its roster custody owner.
    ConsultationRouteRequired,
}

/// Linear authorization routed to the consultation collection owner.
///
/// The route proves only current `operation.cancel` admission. It performs no
/// write and must be consumed by consultation cancellation custody.
#[must_use = "continue this authorized cancellation through consultation custody"]
pub(in crate::resident) struct ResidentConsultationCancellationRoute<'operation> {
    authorized: super::AuthorizedOperationCancellation<'operation>,
    predecessor_frontier: ImmutableRecordReferenceV1,
}

/// Linear authorization for every non-consultation cancellation route.
#[must_use = "retain or classify this authorized cancellation"]
pub(in crate::resident) struct ResidentNonConsultationCancellation<'operation> {
    authorized: super::AuthorizedOperationCancellation<'operation>,
}

/// Effect-free route selected from one freshly authorized cancellation.
#[must_use = "consume the exact owner-specific cancellation route"]
pub(in crate::resident) enum ResidentAuthorizedCancellationRoute<'operation> {
    /// An open consultation must atomically stop its roster with the request.
    Consultation(ResidentConsultationCancellationRoute<'operation>),
    /// Every other state continues through the local or dispatched-task router.
    Other(ResidentNonConsultationCancellation<'operation>),
}

/// Completed nonterminal consultation cancellation and its local advisory result.
#[must_use]
pub(in crate::resident) struct ResidentConsultationCancellationOutcome {
    /// Exact database and independent-anchor custody for the stopped roster.
    pub(in crate::resident) custody: CompletedOperationObservationCustody,
    /// Exact independently anchored successor checkpoint.
    pub(in crate::resident) checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    /// Process-local notification applied only after custody completed.
    pub(in crate::resident) notification: ResidentConsultationCancellationNotification,
}

/// Current non-consultation state returned without a cancellation append.
#[must_use]
pub(in crate::resident) enum ResidentConsultationNotCollecting {
    /// The operation has not reached a provider or consultation dispatch.
    BeforeDispatch(ResidentOperationExistingState),
    /// A dispatch exists, but the parent is not collecting a consultation.
    Dispatched(ResidentOperationExistingState),
}

/// Honest result of one consultation-specific cancellation request.
#[must_use]
pub(in crate::resident) enum ResidentConsultationCancellationDisposition {
    /// The request and stopped consultation reached database and anchor custody.
    Retained(ResidentConsultationCancellationOutcome),
    /// A terminal observation already exists.
    Established(ResidentOperationExistingState),
    /// The exact cancellation request was already retained.
    AlreadyRequested(ImmutableRecordReferenceV1),
    /// Collection has already finished and this call invented no later policy.
    ConsultationCollected(ImmutableRecordReferenceV1),
    /// Uncertainty or recovery state requires reconciliation instead of replay.
    ReconciliationOnly(ResidentOperationExistingState),
    /// The current operation is not collecting a consultation.
    NotCollecting(ResidentConsultationNotCollecting),
}

/// Exact post-write cancellation custody and its process-local notification target.
///
/// The coordinate grants no authority. It only preserves which already-owned
/// batch may be notified after the retained database and anchor continuation
/// proves durable cancellation custody.
#[must_use = "resume the exact cancellation custody before notifying local work"]
pub(in crate::resident) struct ResidentConsultationCancellationCustody {
    failure: ConsultationCollectionCommitFailure,
    local_coordinate: Option<ResidentConsultationBatchCoordinate>,
}

impl std::fmt::Debug for ResidentConsultationCancellationCustody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentConsultationCancellationCustody")
            .field("failure", &self.failure)
            .field("local_coordinate", &self.local_coordinate)
            .finish()
    }
}

/// Why an authorized consultation cancellation did not complete.
#[must_use = "preserve an exact custody continuation and do not notify local work"]
pub(in crate::resident) enum ResidentConsultationCancellationFailure {
    /// The current collection could not be recovered from the protected head.
    Recovery(ConsultationCollectionError),
    /// Current cancellation authority or request projection did not admit.
    Authorization(OperationCancellationRequestError),
    /// The collection stop or parent observation did not admit.
    Admission(ConsultationCollectionError),
    /// Checkpoint preparation or issuance stopped before a physical write.
    Preparation(Box<ConsultationCollectionCommitFailure>),
    /// Database or anchor custody stopped after preserving its exact continuation.
    Custody(Box<ResidentConsultationCancellationCustody>),
}

/// Why the consultation-specific cancellation journey did not complete.
#[must_use = "preserve any returned custody continuation"]
pub(in crate::resident) enum ResidentConsultationCancellationRequestFailure {
    /// Protected operation recovery failed before current authority was checked.
    Recovery(OperationControlRecoveryError),
    /// Fresh current cancellation authority was rejected.
    Authorization(OperationControlError),
    /// Consultation recovery, admission, or custody did not complete.
    Consultation(ResidentConsultationCancellationFailure),
    /// The owner-specific route and recovered disposition disagreed.
    Routing,
}

impl std::fmt::Debug for ResidentConsultationCancellationRequestFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Recovery(_) => "Recovery",
            Self::Authorization(_) => "Authorization",
            Self::Consultation(reason) => match reason {
                ResidentConsultationCancellationFailure::Recovery(_) => "CollectionRecovery",
                ResidentConsultationCancellationFailure::Authorization(_) => "RequestAuthorization",
                ResidentConsultationCancellationFailure::Admission(_) => "CollectionAdmission",
                ResidentConsultationCancellationFailure::Preparation(_) => "Preparation",
                ResidentConsultationCancellationFailure::Custody(_) => "Custody",
            },
            Self::Routing => "Routing",
        };
        formatter
            .debug_struct("ResidentConsultationCancellationRequestFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ResidentConsultationCancellationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Recovery(_) => "Recovery",
            Self::Authorization(_) => "Authorization",
            Self::Admission(_) => "Admission",
            Self::Preparation(_) => "Preparation",
            Self::Custody(_) => "Custody",
        };
        formatter
            .debug_struct("ResidentConsultationCancellationFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Process-local operation task state owned independently of any borrowed port facade.
pub(in crate::resident) struct ResidentOperationTaskState {
    provider_tasks: ResidentProviderTaskService,
    consultation_tasks: ResidentConsultationTaskOwner,
    a2a_delegate_tasks: ResidentA2aDelegateTaskService,
    a2a_get_task_owner: ResidentA2aGetTaskOwner,
    a2a_cancel_task_owner: ResidentA2aCancelTaskOwner,
    reservations: ResidentAggregateTaskReservations,
}

/// Sole process-local task owner selected before a cancellation append.
#[must_use = "notify this exact owner only after durable cancellation custody"]
enum ResidentDispatchedCancellationOwner {
    Provider(ResidentProviderDispatchOwner),
    Consultation(ResidentConsultationDispatchOwner),
    A2aDelegate(ResidentA2aDelegateDispatchOwner),
}

/// Exact post-dispatch custody continuation and its advisory notification owner.
#[must_use = "complete durable custody before notifying the selected local owner"]
pub(in crate::resident) struct ResidentDispatchedCancellationCustody {
    failure: OperationAppendFailure,
    owner: ResidentDispatchedCancellationOwner,
}

impl std::fmt::Debug for ResidentDispatchedCancellationCustody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self.failure {
            OperationAppendFailure::Operation(_) => "Operation",
            OperationAppendFailure::Projection(_) => "Projection",
            OperationAppendFailure::Commit { .. } => "Commit",
            OperationAppendFailure::Anchor { .. } => "Anchor",
        };
        formatter
            .debug_struct("ResidentDispatchedCancellationCustody")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Pure process-local ownership result for one durable dispatch.
enum ResidentDispatchedCancellationOwnerSelection {
    Missing,
    Owned(Box<ResidentDispatchedCancellationOwner>),
    Ambiguous,
}

impl ResidentOperationTaskState {
    /// Create the complete bounded process-local operation task state.
    pub(in crate::resident) fn new(
        provider_task_limits: ResidentOperationTaskLimits,
        maximum_active_consultation_batches: std::num::NonZeroUsize,
        aggregate_task_limits: ResidentAggregateTaskLimits,
    ) -> Result<Self, ResidentOperationTaskStateBuildError> {
        let owner_instance = ResidentOperationTaskOwnerInstance::new();
        let aggregate_maximum = aggregate_task_limits.maximum_outstanding();
        Ok(Self {
            provider_tasks: ResidentProviderTaskService::new_in(
                owner_instance.clone(),
                provider_task_limits,
            )
            .map_err(ResidentOperationTaskStateBuildError::Provider)?,
            consultation_tasks: ResidentConsultationTaskOwner::new_in(
                owner_instance.clone(),
                ResidentConsultationTaskOwnerLimits::new(maximum_active_consultation_batches),
            )
            .map_err(ResidentOperationTaskStateBuildError::Consultation)?,
            a2a_delegate_tasks: ResidentA2aDelegateTaskService::new_in(
                owner_instance.clone(),
                aggregate_maximum,
            )
            .map_err(ResidentOperationTaskStateBuildError::A2aDelegate)?,
            a2a_get_task_owner: ResidentA2aGetTaskOwner::new_in(
                owner_instance.clone(),
                aggregate_maximum,
            )
            .map_err(ResidentOperationTaskStateBuildError::A2aGetTask)?,
            a2a_cancel_task_owner: ResidentA2aCancelTaskOwner::new_in(
                owner_instance.clone(),
                aggregate_maximum,
            )
            .map_err(ResidentOperationTaskStateBuildError::A2aCancelTask)?,
            reservations: ResidentAggregateTaskReservations::new(
                owner_instance,
                aggregate_task_limits,
            )
            .map_err(ResidentOperationTaskStateBuildError::Aggregate)?,
        })
    }

    /// Return all provider, Consult, and A2A work consuming process capacity.
    #[must_use]
    pub(in crate::resident) fn outstanding_task_reservations(&self) -> usize {
        self.reservations.outstanding()
    }

    /// Select the sole process-local owner of one accepted dispatch.
    ///
    /// Selection performs no signing, storage, anchor, executor, or provider
    /// work. A duplicate claim fails closed instead of choosing by precedence.
    #[must_use]
    fn select_dispatched_cancellation_owner(
        &self,
        dispatch: &ResidentOperationTaskKey,
    ) -> ResidentDispatchedCancellationOwnerSelection {
        let provider = self.provider_tasks.select_dispatch_owner(dispatch);
        let consultation = self.consultation_tasks.select_dispatch_owner(dispatch);
        let a2a = self.a2a_delegate_tasks.select_dispatch_owner(dispatch);
        if matches!(
            consultation,
            ResidentConsultationDispatchOwnerSelection::Ambiguous
        ) {
            return ResidentDispatchedCancellationOwnerSelection::Ambiguous;
        }

        let mut selected = None;
        let mut add = |owner| {
            if selected.is_some() {
                return false;
            }
            selected = Some(owner);
            true
        };
        if let ResidentProviderDispatchOwnerSelection::Owned(owner) = provider {
            let _ = add(ResidentDispatchedCancellationOwner::Provider(owner));
        }
        if let ResidentConsultationDispatchOwnerSelection::Owned(owner) = consultation {
            if !add(ResidentDispatchedCancellationOwner::Consultation(*owner)) {
                return ResidentDispatchedCancellationOwnerSelection::Ambiguous;
            }
        }
        if let ResidentA2aDelegateDispatchOwnerSelection::Owned(owner) = a2a {
            if !add(ResidentDispatchedCancellationOwner::A2aDelegate(owner)) {
                return ResidentDispatchedCancellationOwnerSelection::Ambiguous;
            }
        }
        match selected {
            Some(owner) => ResidentDispatchedCancellationOwnerSelection::Owned(Box::new(owner)),
            None => ResidentDispatchedCancellationOwnerSelection::Missing,
        }
    }

    /// Return the current number of accepted provider calls awaiting a join.
    #[must_use]
    pub(in crate::resident) fn in_flight_provider_tasks(&self) -> usize {
        self.provider_tasks.in_flight()
    }

    /// Return the exact process-local provider occupancy ceiling.
    #[must_use]
    pub(in crate::resident) const fn provider_task_limits(&self) -> ResidentOperationTaskLimits {
        self.provider_tasks.limits()
    }

    /// Return the shared process-local occupancy ceiling.
    #[must_use]
    pub(in crate::resident) const fn aggregate_task_limits(&self) -> ResidentAggregateTaskLimits {
        self.reservations.limits()
    }

    /// Borrow the process owner shared by all operation-task custody.
    #[must_use]
    pub(in crate::resident) const fn operation_task_owner_instance(
        &self,
    ) -> &ResidentOperationTaskOwnerInstance {
        self.provider_tasks.owner_instance()
    }

    /// Poll one provider completion without durable custody I/O.
    pub(in crate::resident) fn poll_provider_task<Executor>(
        &mut self,
        executor: &mut Executor,
    ) -> Option<ResidentProviderCompletionPoll>
    where
        Executor: ResidentOperationTaskExecutor<
            super::operation_dispatch::OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
    {
        self.provider_tasks
            .poll_completion(&mut self.reservations, executor)
    }

    /// Retry only a rejected provider completion-to-aggregate transition.
    pub(in crate::resident) fn retry_provider_completion_transition(
        &mut self,
        completion: ResidentOperationTaskCompletion<
            super::operation_dispatch::OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
    ) -> ResidentProviderCompletionPoll {
        self.provider_tasks
            .retry_completion_transition(completion, &mut self.reservations)
    }

    /// Repair only aggregate occupancy after provider acceptance already won.
    pub(in crate::resident) fn continue_accepted_provider_aggregate_transition<'operation>(
        &mut self,
        refusal: super::task_owner::ResidentProviderTaskRefusal<'operation>,
    ) -> Result<
        super::task_owner::ResidentProviderAcceptedAggregateTransition<'operation>,
        super::task_owner::ResidentProviderTaskRefusal<'operation>,
    > {
        self.provider_tasks
            .continue_accepted_aggregate_transition(refusal, &mut self.reservations)
    }

    /// Repair only aggregate occupancy after unopened provider work was refused.
    pub(in crate::resident) fn continue_refused_provider_aggregate_transition<'operation>(
        &mut self,
        refusal: super::task_owner::ResidentProviderTaskRefusal<'operation>,
    ) -> Result<
        super::task_owner::ResidentProviderTaskRefusal<'operation>,
        super::task_owner::ResidentProviderTaskRefusal<'operation>,
    > {
        self.provider_tasks
            .continue_refused_aggregate_transition(refusal, &mut self.reservations)
    }

    /// Return accepted consultation child work still awaiting a join.
    #[must_use]
    pub(in crate::resident) fn in_flight_consultation_tasks(&self) -> usize {
        self.consultation_tasks.in_flight()
    }

    /// Return started A2A sends still awaiting their first terminal branch.
    #[must_use]
    pub(in crate::resident) fn in_flight_a2a_delegate_tasks(&self) -> usize {
        self.a2a_delegate_tasks.in_flight()
    }

    /// Return exact consultation batches retained by this process.
    #[must_use]
    pub(in crate::resident) fn active_consultation_batches(&self) -> usize {
        self.consultation_tasks.active_batches()
    }

    /// Return whether this process owns the exact finite Consult batch.
    #[must_use]
    pub(in crate::resident) fn retains_consultation_batch(
        &self,
        expected: &super::consultation::ResidentConsultationBatchCoordinate,
    ) -> bool {
        self.consultation_tasks
            .retained_batch_coordinate(expected.store(), expected.parent())
            == Some(expected)
    }

    /// Run one bounded Consult turn after collection recovery has ended.
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
            &super::consultation::ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, ChildRecoveryError>,
        Retain: FnOnce(
            &super::consultation::ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
    {
        super::consultation::supervise_recovered_consultation_turn(
            &mut self.consultation_tasks,
            &mut self.reservations,
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
        super::consultation::supervise_recovered_consultation_pre_policy_turn(
            &mut self.consultation_tasks,
            &mut self.reservations,
            task_limits,
            observed_at,
            current,
            executor,
        )
    }

    /// Apply one caller-selected decision to an existing policy frontier.
    ///
    /// This performs no collection recovery, batch reconciliation, executor
    /// poll, or policy selection.
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
            &super::consultation::ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, ChildRecoveryError>,
        Retain: FnOnce(
            &super::consultation::ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
    {
        apply_consultation_supervision_decision(
            &mut self.consultation_tasks,
            &mut self.reservations,
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
        observed_at: UnixTimeV1,
        child_store: &pale_ports::AuthorityStoreCoordinate,
        child_operation: &super::IssuedLocalOperation,
        dispatch_ref: &ResidentRecordRefV1,
        dispatch: &OperationObservationV1,
    ) -> Result<
        PreparedResidentConsultationTaskResume,
        ResidentConsultationTaskResumePreparationFailure,
    > {
        self.consultation_tasks.prepare_resumed(
            coordinate,
            candidate,
            &self.reservations,
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
    ) -> Result<ResidentOperationTaskKey, ResidentConsultationPreparedResumeFailure<Work>>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        self.consultation_tasks.accept_prepared_resume(
            prepared,
            &mut self.reservations,
            work,
            executor,
        )
    }

    /// Prepare one historical Consult dispatch for executor-free loss handling.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn prepare_recovered_consultation_loss(
        &self,
        coordinate: &super::consultation::ResidentConsultationBatchCoordinate,
        candidate: &super::consultation::ConsultationChildDispatchCandidate,
        current: &super::consultation::RecoveredConsultationCollection,
        observed_at: UnixTimeV1,
        child_store: &pale_ports::AuthorityStoreCoordinate,
        child_operation: &super::IssuedLocalOperation,
        dispatch_ref: &ResidentRecordRefV1,
        dispatch: &OperationObservationV1,
    ) -> Result<PreparedResidentConsultationTaskLoss, ResidentConsultationTaskLossPreparationFailure>
    {
        self.consultation_tasks.prepare_recovered_loss(
            coordinate,
            candidate,
            &self.reservations,
            current,
            observed_at,
            child_store,
            child_operation,
            dispatch_ref,
            dispatch,
        )
    }

    /// Commit an executor-free historical loss through its exact aggregate offer.
    pub(in crate::resident) fn commit_recovered_consultation_loss<Work, Output>(
        &mut self,
        prepared: PreparedResidentConsultationTaskLoss,
    ) -> Result<
        ResidentConsultationTaskOwnerPoll<Work, Output>,
        ResidentConsultationTaskLossCommitFailure,
    > {
        self.consultation_tasks
            .commit_recovered_loss(prepared, &mut self.reservations)
    }

    /// Retain the exact aggregate offer before its recovered-loss transition.
    pub(in crate::resident) fn offer_recovered_consultation_loss(
        &mut self,
        prepared: PreparedResidentConsultationTaskLoss,
    ) -> Result<ResidentConsultationTaskLossOffer, ResidentConsultationTaskLossCommitFailure> {
        self.consultation_tasks
            .offer_recovered_loss(prepared, &mut self.reservations)
    }

    /// Retry only the already-begun aggregate loss transition.
    pub(in crate::resident) fn retry_recovered_consultation_loss<Work, Output>(
        &mut self,
        pending: ResidentConsultationTaskLossOffer,
    ) -> Result<
        ResidentConsultationTaskOwnerPoll<Work, Output>,
        ResidentConsultationTaskLossCommitFailure,
    > {
        self.consultation_tasks
            .retry_recovered_loss(pending, &mut self.reservations)
    }

    /// Retry only capacity release after a Consult parent join reached custody.
    pub(in crate::resident) fn retry_consultation_terminal_release<Work, Output>(
        &mut self,
        joined: ResidentConsultationJoinedTerminal<Work, Output>,
    ) -> Result<
        CompletedResidentConsultationTerminal<Work, Output>,
        ResidentConsultationTerminalJoinFailure<Work, Output>,
    > {
        retry_consultation_terminal_release(
            &mut self.consultation_tasks,
            &mut self.reservations,
            joined,
        )
    }

    /// Release capacity after protected recovery proves durable uncertainty.
    pub(in crate::resident) fn settle_consultation_nonterminal<Work, Output>(
        &mut self,
        ready: ResidentConsultationCompletionReady<Work, Output>,
        child: &RecoveredOperationObservation,
    ) -> Result<
        CompletedResidentConsultationNonterminal<Work, Output>,
        ResidentConsultationNonterminalSettlementFailure<Work, Output>,
    > {
        settle_consultation_nonterminal(
            &mut self.consultation_tasks,
            &mut self.reservations,
            ready,
            child,
        )
    }

    pub(in crate::resident) fn open_consultation_batch(
        &mut self,
        planned: super::ConsultationChildDispatchBatch,
        limits: ResidentConsultationTaskLimits,
    ) -> Result<ResidentConsultationBatchCoordinate, ResidentConsultationBatchOpenError> {
        self.consultation_tasks.open_batch(planned, limits)
    }

    /// Reconcile one recovered Consult roster with process-local ownership.
    pub(in crate::resident) fn reconcile_consultation_batch(
        &mut self,
        planned: super::ConsultationChildDispatchBatch,
        limits: ResidentConsultationTaskLimits,
    ) -> Result<
        super::consultation::service::task::ResidentConsultationBatchReconciliation,
        super::consultation::service::task::ResidentConsultationBatchReconcileError,
    > {
        self.consultation_tasks
            .reconcile_batch(planned, limits, &self.reservations)
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retain_and_accept_consultation_task<
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
        candidate: &super::consultation::ConsultationChildDispatchCandidate,
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
        Lookup:
            FnOnce() -> Result<super::consultation::RecoveredConsultationCollection, LookupError>,
        RecoverChild: FnOnce(
            &super::consultation::ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, LookupError>,
        Retain: FnOnce(
            &super::consultation::ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        self.consultation_tasks.retain_and_accept(
            coordinate,
            candidate,
            &mut self.reservations,
            clock,
            lookup,
            recover_child,
            retain,
            executor,
        )
    }

    pub(in crate::resident) fn retry_consultation_task<Work, Output, Executor>(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
        retry: ResidentConsultationTaskRetry<Work>,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentConsultationTaskRetryFailure<Work>>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        self.consultation_tasks.retry_accept_reserved(
            coordinate,
            retry,
            &mut self.reservations,
            executor,
        )
    }

    pub(in crate::resident) fn try_consultation_completion<Work, Output, Executor>(
        &mut self,
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
        self.consultation_tasks
            .try_completion_reserved(&mut self.reservations, executor)
    }

    /// Retry only a rejected Consult completion-to-aggregate transition.
    ///
    /// The shared executor is not polled again.
    pub(in crate::resident) fn retry_consultation_completion_transition<Work, Output>(
        &mut self,
        completion: ResidentOperationTaskCompletion<Work, Output>,
    ) -> Result<
        ResidentConsultationTaskOwnerPoll<Work, Output>,
        ResidentConsultationTaskOwnerPollError<Work, Output>,
    > {
        self.consultation_tasks
            .retry_completion_reserved(completion, &mut self.reservations)
    }

    pub(in crate::resident) fn retire_consultation_batch(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
    ) -> ResidentConsultationBatchRetirement {
        if self.reservations.has_batch_reservations(coordinate) {
            return ResidentConsultationBatchRetirement::Busy(
                super::consultation::ResidentConsultationBatchRetentionReason::AggregateReservation,
            );
        }
        self.consultation_tasks.retire_quiescent(coordinate)
    }
}

/// Why process-local operation task state could not reserve its complete bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentOperationTaskStateBuildError {
    /// Provider-task storage could not reserve its configured occupancy.
    Provider(ResidentOperationTaskOwnerBuildError),
    /// Consultation-batch storage could not reserve its configured occupancy.
    Consultation(ResidentConsultationTaskOwnerBuildError),
    /// A2A delegate storage could not reserve the aggregate occupancy ceiling.
    A2aDelegate(ResidentOperationTaskOwnerBuildError),
    /// A2A `GetTask` storage could not reserve the aggregate occupancy ceiling.
    A2aGetTask(ResidentOperationTaskOwnerBuildError),
    /// A2A `CancelTask` storage could not reserve the aggregate occupancy ceiling.
    A2aCancelTask(ResidentOperationTaskOwnerBuildError),
    /// Aggregate task storage could not reserve its configured occupancy.
    Aggregate(ResidentAggregateTaskReservationBuildError),
}

/// Short-lived private facade over resident owners and process-local task state.
pub(in crate::resident) struct ResidentOperationService<'ports, Authority> {
    authority: &'ports mut Authority,
    custody: &'ports mut dyn OperationCustodyPort,
    anchor: &'ports mut dyn AntiRollbackAnchorPort,
    tasks: &'ports mut ResidentOperationTaskState,
}

impl<'ports, Authority> ResidentOperationService<'ports, Authority> {
    /// Borrow already selected resident owners and their process-local task state.
    pub(in crate::resident) fn new(
        authority: &'ports mut Authority,
        custody: &'ports mut dyn OperationCustodyPort,
        anchor: &'ports mut dyn AntiRollbackAnchorPort,
        tasks: &'ports mut ResidentOperationTaskState,
    ) -> Self {
        Self {
            authority,
            custody,
            anchor,
            tasks,
        }
    }

    /// Return the current number of accepted provider calls awaiting a join.
    #[must_use]
    pub(in crate::resident) fn in_flight_provider_tasks(&self) -> usize {
        self.tasks.in_flight_provider_tasks()
    }

    /// Return accepted consultation child work still awaiting a join.
    #[must_use]
    pub(in crate::resident) fn in_flight_consultation_tasks(&self) -> usize {
        self.tasks.in_flight_consultation_tasks()
    }

    /// Return started A2A sends still owned by this resident lifetime.
    #[must_use]
    pub(in crate::resident) fn in_flight_a2a_delegate_tasks(&self) -> usize {
        self.tasks.in_flight_a2a_delegate_tasks()
    }

    /// Return exact consultation batches retained by this process.
    #[must_use]
    pub(in crate::resident) fn active_consultation_batches(&self) -> usize {
        self.tasks.active_consultation_batches()
    }

    /// Return all provider, Consult, and A2A work consuming process capacity.
    #[must_use]
    pub(in crate::resident) fn outstanding_task_reservations(&self) -> usize {
        self.tasks.outstanding_task_reservations()
    }

    /// Retain and start one A2A delegation at most once under shared capacity.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn launch_a2a_delegate<'operation, 'call, Disclosure, Transport, Sign>(
        &mut self,
        input: A2aDelegateLaunchInput<'operation, 'call>,
        exchange: &dyn A2aExchangePort,
        admit_disclosure: Disclosure,
        admit_transport: Transport,
        sign_checkpoint: Sign,
    ) -> Result<ResidentA2aDelegateLaunch, ResidentA2aDelegateLaunchFailure<'operation, 'call>>
    where
        Authority: AmbientSessionAuthorityPort,
        Disclosure: FnMut(
            &pale_ports::ProviderInvocationRequest<'_>,
            &A2aRouteAdmissionV1,
            &str,
        ) -> Result<(), InvalidResidentRecordV1>,
        Transport: FnOnce(
            &ImmutableRecordReferenceV1,
            &pale_ports::ProviderInvocationRequest<'_>,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let Self {
            authority,
            custody,
            anchor,
            tasks,
        } = self;
        tasks.a2a_delegate_tasks.launch(
            input,
            *authority,
            *custody,
            *anchor,
            exchange,
            &mut tasks.reservations,
            admit_disclosure,
            admit_transport,
            sign_checkpoint,
        )
    }

    /// Retry only capacity release after launch proved that the carrier never started.
    pub(in crate::resident) fn retry_a2a_delegate_launch_rollback<'operation, 'call>(
        &mut self,
        failure: ResidentA2aDelegateLaunchFailure<'operation, 'call>,
    ) -> Result<
        ResidentA2aDelegateLaunchFailure<'operation, 'call>,
        ResidentA2aDelegateLaunchFailure<'operation, 'call>,
    > {
        self.tasks
            .a2a_delegate_tasks
            .retry_launch_rollback(failure, &mut self.tasks.reservations)
    }

    /// Settle a closed exchange whose task key failed the launch join.
    pub(in crate::resident) fn settle_a2a_delegate_started_association<'operation, 'call>(
        &mut self,
        failure: ResidentA2aDelegateLaunchFailure<'operation, 'call>,
    ) -> Result<ResidentA2aDelegateLaunch, ResidentA2aDelegateLaunchFailure<'operation, 'call>>
    {
        self.tasks
            .a2a_delegate_tasks
            .settle_started_association(failure, &mut self.tasks.reservations)
    }

    /// Continue only exact unresolved A2A database or anchor custody.
    pub(in crate::resident) fn continue_a2a_delegate_custody<'operation, 'call>(
        &mut self,
        unresolved: ResidentA2aDelegateUnresolvedLaunch<'operation, 'call>,
    ) -> Result<
        ResidentA2aDelegateUnopenedLaunch<'operation, 'call>,
        ResidentA2aDelegateCustodyContinuationFailure<'operation, 'call>,
    > {
        self.tasks.a2a_delegate_tasks.continue_custody(
            unresolved,
            self.custody,
            self.anchor,
            &self.tasks.reservations,
        )
    }

    /// Convert exact recovery-only A2A custody into retained uncertainty.
    pub(in crate::resident) fn recover_unresolved_a2a_delegate<'operation, 'call, 'a>(
        &mut self,
        unresolved: ResidentA2aDelegateUnresolvedLaunch<'operation, 'call>,
        input: A2aSendRecoveryInput<'a>,
        admission: A2aSendRecoveryAdmission<'_>,
    ) -> Result<
        ResidentRecoveredA2aDelegate<'a>,
        ResidentA2aDelegateUnresolvedRecoveryFailure<'operation, 'call, 'a>,
    > {
        self.tasks.a2a_delegate_tasks.recover_unresolved_as_unknown(
            unresolved,
            input,
            self.custody,
            self.anchor,
            admission,
            &mut self.tasks.reservations,
        )
    }

    /// Retry only the loss transition after protected A2A recovery succeeded.
    pub(in crate::resident) fn retry_recovered_a2a_delegate_loss<'a>(
        &mut self,
        rejected: ResidentA2aDelegateRecoveredOfferRejection<'a>,
    ) -> Result<ResidentRecoveredA2aDelegate<'a>, ResidentA2aDelegateRecoveredOfferRejection<'a>>
    {
        self.tasks
            .a2a_delegate_tasks
            .retry_recovered_loss_offer(rejected, &mut self.tasks.reservations)
    }

    /// Retry one exact A2A send only while its carrier is proven unopened.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retry_unopened_a2a_delegate<'operation, 'call, Disclosure>(
        &mut self,
        unopened: ResidentA2aDelegateUnopenedLaunch<'operation, 'call>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        admit_disclosure: Disclosure,
    ) -> Result<ResidentA2aDelegateLaunch, ResidentA2aDelegateUnopenedRetryFailure<'operation, 'call>>
    where
        Authority: AmbientSessionAuthorityPort,
        Disclosure: FnOnce(
            &pale_ports::ProviderInvocationRequest<'_>,
            &A2aRouteAdmissionV1,
            &str,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        self.tasks.a2a_delegate_tasks.retry_unopened(
            unopened,
            self.authority,
            self.custody,
            self.anchor,
            exchange,
            clock,
            &mut self.tasks.reservations,
            admit_disclosure,
        )
    }

    /// Poll one exact retained A2A exchange without reconnecting or replaying.
    pub(in crate::resident) fn poll_a2a_delegate(
        &mut self,
        key: &ResidentOperationTaskKey,
        cx: &mut std::task::Context<'_>,
        protocol: &dyn A2aSendMessageEvidencePort,
        clock: &dyn WallClockSourcePort,
    ) -> ResidentA2aDelegatePoll {
        self.tasks
            .a2a_delegate_tasks
            .poll(key, cx, protocol, clock, &mut self.tasks.reservations)
    }

    /// Stop local reads and retain explicit remote-effect uncertainty.
    pub(in crate::resident) fn cancel_a2a_delegate_reads(
        &mut self,
        key: &ResidentOperationTaskKey,
    ) -> ResidentA2aDelegatePoll {
        self.tasks
            .a2a_delegate_tasks
            .cancel(key, &mut self.tasks.reservations)
    }

    /// Retry only a rejected aggregate transition for one closed exchange.
    pub(in crate::resident) fn retry_a2a_delegate_settlement(
        &mut self,
        rejected: ResidentA2aDelegateSettlementRejection,
    ) -> ResidentA2aDelegatePoll {
        self.tasks
            .a2a_delegate_tasks
            .retry_completion_settlement(rejected, &mut self.tasks.reservations)
    }

    /// Retry only a rejected offered-completion transition after launch closed.
    pub(in crate::resident) fn retry_a2a_delegate_offered_completion(
        &mut self,
        rejected: ResidentA2aDelegateOfferedCompletionRejection,
    ) -> Result<ResidentA2aDelegatePoll, ResidentA2aDelegateOfferedCompletionRejection> {
        self.tasks
            .a2a_delegate_tasks
            .retry_offered_completion(rejected, &mut self.tasks.reservations)
    }

    /// Retry only local diagnostic construction for a closed A2A branch.
    pub(in crate::resident) fn recover_a2a_delegate_diagnostic(
        &self,
        diagnostic: ResidentA2aDelegateDiagnostic,
    ) -> Result<
        super::a2a_delegate_task_service::ResidentA2aDelegateCompletion,
        ResidentA2aDelegateDiagnosticRecoveryFailure,
    > {
        self.tasks
            .a2a_delegate_tasks
            .recover_diagnostic(diagnostic, &self.tasks.reservations)
    }

    /// Recover a retained pre-Task send as uncertainty without carrier access.
    pub(in crate::resident) fn recover_a2a_delegate<'a>(
        &mut self,
        input: A2aSendRecoveryInput<'a>,
        admission: A2aSendRecoveryAdmission<'_>,
    ) -> Result<ResidentRecoveredA2aDelegate<'a>, ResidentA2aDelegateRecoveryFailure<'a>> {
        self.tasks.a2a_delegate_tasks.recover(
            input,
            self.custody,
            self.anchor,
            admission,
            &mut self.tasks.reservations,
        )
    }

    /// Retry only aggregate-offer cleanup after rejected A2A send recovery.
    pub(in crate::resident) fn retry_a2a_delegate_recovery_rollback<'a>(
        &mut self,
        failure: ResidentA2aDelegateRecoveryFailure<'a>,
    ) -> Result<A2aSendRecoveryError, ResidentA2aDelegateRecoveryFailure<'a>> {
        self.tasks
            .a2a_delegate_tasks
            .retry_recovery_rollback(failure, &mut self.tasks.reservations)
    }

    /// Release shared task capacity after exact A2A disposition custody succeeds.
    pub(in crate::resident) fn finish_a2a_delegate(
        &mut self,
        finalization: ResidentA2aDelegateFinalization,
        retained: RetainedA2aOutcome,
    ) -> Result<RetainedA2aOutcome, ResidentA2aDelegateFinalizationRejection> {
        self.tasks
            .a2a_delegate_tasks
            .finish(finalization, retained, &mut self.tasks.reservations)
    }

    /// Retain one closed A2A branch through its existing durable owner.
    pub(in crate::resident) fn retain_a2a_delegate_outcome<'a>(
        &mut self,
        input: A2aOutcomeRetentionInput<'a>,
    ) -> Result<RetainedA2aOutcome, A2aOutcomeRetentionFailure<'a>> {
        retain_a2a_outcome(input, self.custody, self.anchor)
    }

    pub(in crate::resident) fn retain_a2a_delegate_outcome_continuable<'a>(
        &mut self,
        input: A2aOutcomeRetentionInput<'a>,
    ) -> Result<RetainedA2aOutcome, ContinuableA2aOutcomeRetentionFailure<'a>> {
        retain_a2a_outcome_continuable(input, self.custody, self.anchor)
    }

    pub(in crate::resident) fn continue_a2a_delegate_outcome_custody<'a>(
        &mut self,
        failure: ContinuableA2aOutcomeRetentionFailure<'a>,
    ) -> Result<RetainedA2aOutcome, ContinuableA2aOutcomeRetentionFailure<'a>> {
        failure.continue_custody(self.custody, self.anchor)
    }

    /// Retain one exact finite Consult batch in process-local task ownership.
    ///
    /// This only allocates the bounded mechanical owner. Child release still
    /// requires fresh protected parent and child recovery through the batch
    /// service. No completion or synthesis policy is selected here.
    pub(in crate::resident) fn open_consultation_batch(
        &mut self,
        planned: super::ConsultationChildDispatchBatch,
        limits: ResidentConsultationTaskLimits,
    ) -> Result<ResidentConsultationBatchCoordinate, ResidentConsultationBatchOpenError> {
        self.tasks.open_consultation_batch(planned, limits)
    }

    /// Recheck, retain, and offer one fresh Consult child under the shared ceiling.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retain_and_accept_consultation_task<
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
        candidate: &super::consultation::ConsultationChildDispatchCandidate,
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
        Lookup:
            FnOnce() -> Result<super::consultation::RecoveredConsultationCollection, LookupError>,
        RecoverChild: FnOnce(
            &super::consultation::ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, LookupError>,
        Retain: FnOnce(
            &super::consultation::ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        self.tasks.retain_and_accept_consultation_task(
            coordinate,
            candidate,
            clock,
            lookup,
            recover_child,
            retain,
            executor,
        )
    }

    /// Retry unopened work only through the exact batch named by its token.
    pub(in crate::resident) fn retry_consultation_task<Work, Output, Executor>(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
        retry: ResidentConsultationTaskRetry<Work>,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentConsultationTaskRetryFailure<Work>>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        self.tasks
            .retry_consultation_task(coordinate, retry, executor)
    }

    /// Route one shared-executor result to its exact retained Consult batch.
    pub(in crate::resident) fn try_consultation_completion<Work, Output, Executor>(
        &mut self,
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
        self.tasks.try_consultation_completion(executor)
    }

    /// Retry only the aggregate join for an already-returned Consult result.
    pub(in crate::resident) fn retry_consultation_completion_transition<Work, Output>(
        &mut self,
        completion: super::task_owner::ResidentOperationTaskCompletion<Work, Output>,
    ) -> Result<
        ResidentConsultationTaskOwnerPoll<Work, Output>,
        ResidentConsultationTaskOwnerPollError<Work, Output>,
    > {
        self.tasks
            .retry_consultation_completion_transition(completion)
    }

    /// Run one recovery-first, nonblocking Consult supervision turn.
    ///
    /// The turn recovers one collection, reconciles no more than one local
    /// batch, then polls at most once after successful recovery and any
    /// required reconciliation. Package policy runs only when no completion is ready.
    /// A ready completion or selected stop leaves the borrowed turn as an
    /// owned phase; this service performs its exact custody afterward.
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
        &mut self,
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
            &super::consultation::ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, ChildRecoveryError>,
        Retain: FnOnce(
            &super::consultation::ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
    {
        super::consultation::supervise_consultation_turn(
            &mut self.tasks.consultation_tasks,
            &mut self.tasks.reservations,
            task_limits,
            observed_at,
            recover,
            executor,
            decide,
            recover_child,
            retain,
        )
    }

    /// Commit one Package-selected stop through exact database and anchor custody.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn commit_selected_consultation_stop<
        'operation,
        AdmitStop,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        selected: ResidentConsultationStopSelected<ConsultationStopInput<'operation>>,
        checkpoint: ConsultationSuccessorCommitInput<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        admit_stop: AdmitStop,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<CompletedConsultationCollection, ResidentConsultationStopCommitFailure<'operation>>
    where
        AdmitStop: FnOnce(
            &super::consultation::ConsultationCollection,
            &super::consultation::ConsultationRequirementSummary,
        ) -> Result<(), InvalidResidentRecordV1>,
        AdmitCollection: FnOnce(
            &super::consultation::ConsultationCollection,
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
        commit_selected_consultation_stop(
            selected,
            checkpoint,
            resolve_store,
            self.custody,
            self.anchor,
            admit_stop,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Continue only the exact database or anchor candidate for one selected stop.
    pub(in crate::resident) fn continue_consultation_stop_custody(
        &mut self,
        pending: ResidentConsultationStopCustody,
    ) -> Result<CompletedConsultationCollection, ResidentConsultationStopCustody> {
        continue_consultation_stop_custody(pending, self.custody, self.anchor)
    }

    /// Join the exact terminal child and release capacity after both custody layers.
    ///
    /// The borrowed preflight rejects a substituted process owner, store,
    /// batch, candidate, dispatch, or child phase before storage or anchor I/O.
    /// No completion, quorum, lateness, synthesis, or stop policy is selected.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn join_consultation_terminal<
        'operation,
        Work,
        Output,
        AdmitEvidence,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        ready: ResidentConsultationCompletionReady<Work, Output>,
        input: ConsultationChildJoinInput<'operation>,
        checkpoint: ConsultationSuccessorCommitInput<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        admit_evidence: AdmitEvidence,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        CompletedResidentConsultationTerminal<Work, Output>,
        ResidentConsultationTerminalJoinFailure<Work, Output>,
    >
    where
        AdmitEvidence: FnOnce(
            &super::consultation::ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        AdmitCollection: FnOnce(
            &super::consultation::ConsultationCollection,
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
        let Self {
            custody,
            anchor,
            tasks,
            ..
        } = self;
        join_consultation_terminal(
            &mut tasks.consultation_tasks,
            &mut tasks.reservations,
            *custody,
            *anchor,
            ready,
            input,
            checkpoint,
            resolve_store,
            admit_evidence,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Continue only the retained database or anchor candidate for one join.
    pub(in crate::resident) fn continue_consultation_terminal_custody<Work, Output>(
        &mut self,
        pending: ResidentConsultationTerminalCustody<Work, Output>,
    ) -> Result<
        CompletedResidentConsultationTerminal<Work, Output>,
        ResidentConsultationTerminalJoinFailure<Work, Output>,
    > {
        let Self {
            custody,
            anchor,
            tasks,
            ..
        } = self;
        continue_consultation_terminal_custody(
            &mut tasks.consultation_tasks,
            &mut tasks.reservations,
            *custody,
            *anchor,
            pending,
        )
    }

    /// Retry only capacity release after the exact parent join reached custody.
    pub(in crate::resident) fn retry_consultation_terminal_release<Work, Output>(
        &mut self,
        joined: ResidentConsultationJoinedTerminal<Work, Output>,
    ) -> Result<
        CompletedResidentConsultationTerminal<Work, Output>,
        ResidentConsultationTerminalJoinFailure<Work, Output>,
    > {
        retry_consultation_terminal_release(
            &mut self.tasks.consultation_tasks,
            &mut self.tasks.reservations,
            joined,
        )
    }

    /// Retire an exact process-local batch only after all accepted work joined.
    #[must_use]
    pub(in crate::resident) fn retire_consultation_batch(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
    ) -> ResidentConsultationBatchRetirement {
        self.tasks.retire_consultation_batch(coordinate)
    }

    /// Run the existing protected Observe use case through the bound custody owners.
    pub(in crate::resident) fn observe(
        &mut self,
        input: LocalOperationObservationInput<'_>,
        sessions: &dyn SessionLeaseCustodyPort,
        admission: LocalOperationObservationAdmission<'_>,
    ) -> Result<OperationControlObservation, LocalOperationObservationError> {
        observe_local_operation(input, self.custody, sessions, self.anchor, admission)
    }

    /// Append caller-admitted conclusive evidence to one recovered current head.
    ///
    /// The recovered proof is consumed once. This service supplies only the
    /// already selected database and independent-anchor owners. The caller still
    /// owns evidence admission, authority rotation, signing, and physical-store
    /// resolution. An ambiguous commit or anchor result retains the exact signed
    /// successor and custody continuation returned by the append owner.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn append_current_head_reconciliation<
        Evidence,
        Rotation,
        SignObservation,
        SignCheckpoint,
    >(
        &mut self,
        input: CurrentHeadReconciliationAppendInput<'_>,
        store_resolver: &dyn OperationObservationAuthorityStoreResolver,
        admit_evidence: Evidence,
        admit_rotation: Rotation,
        sign_observation: SignObservation,
        sign_checkpoint: SignCheckpoint,
    ) -> Result<CompletedCurrentHeadReconciliation, CurrentHeadReconciliationAppendFailure>
    where
        Evidence: FnOnce(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        SignObservation: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        SignCheckpoint: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        super::operation_observation_custody::reconciliation_append::append_current_head_reconciliation(
            input,
            store_resolver,
            self.custody,
            self.anchor,
            admit_evidence,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Prepare and durably start one local Invoke through the existing semantic owner.
    ///
    /// The handle cannot leave this service until the start has reached database
    /// custody and the independent anti-rollback anchor. This method does not
    /// select or dispatch a provider.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn start_invoke<
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        input: ResidentLocalOperationStartInput<'_>,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        ResidentLocalOperationCoordinator::new(self.authority, self.custody, self.anchor)
            .start_invoke(
                input,
                admit_compiled_authority,
                sign_handle,
                sign_observation,
                prepare_checkpoint_preimage,
                sign_checkpoint,
            )
    }

    /// Prepare and durably start one root `Consult` without selecting its roster.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn start_consult<
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        input: ResidentLocalOperationStartInput<'_>,
        intent: Option<&super::consultation::ConsultationStartIntent>,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        ResidentLocalOperationCoordinator::new(self.authority, self.custody, self.anchor)
            .start_consult(
                input,
                intent,
                admit_compiled_authority,
                sign_handle,
                sign_observation,
                prepare_checkpoint_preimage,
                sign_checkpoint,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn start_consult_with_intent_builder<
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        Intent,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        input: ResidentLocalOperationStartInput<'_>,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        build_intent: Intent,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Intent: FnOnce(
            &pale_ports::AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &OperationHandleV1,
        ) -> Result<
            super::consultation::ConsultationStartIntent,
            super::LocalOperationStartError,
        >,
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        ResidentLocalOperationCoordinator::new(self.authority, self.custody, self.anchor)
            .start_consult_with_intent_builder(
                input,
                admit_compiled_authority,
                sign_handle,
                sign_observation,
                build_intent,
                prepare_checkpoint_preimage,
                sign_checkpoint,
            )
    }

    /// Start one protected `Invoke` or nested `Consult` child of a Consult parent.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn start_consultation_child<
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        input: ResidentLocalOperationStartInput<'_>,
        parent: super::PreparedLocalChildParent,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        ResidentLocalOperationCoordinator::new(self.authority, self.custody, self.anchor)
            .start_consultation_child(
                input,
                parent,
                admit_compiled_authority,
                sign_handle,
                sign_observation,
                prepare_checkpoint_preimage,
                sign_checkpoint,
            )
    }

    /// Start one protected `Delegate` child without Consult membership.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn start_delegate_child<
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        input: ResidentLocalOperationStartInput<'_>,
        parent: super::PreparedLocalChildParent,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        ResidentLocalOperationCoordinator::new(self.authority, self.custody, self.anchor)
            .start_delegate_child(
                input,
                parent,
                admit_compiled_authority,
                sign_handle,
                sign_observation,
                prepare_checkpoint_preimage,
                sign_checkpoint,
            )
    }

    /// Commit one already-admitted initial Consult collection.
    ///
    /// The parent and every fresh child are already durable. This method adds
    /// no roster, dispatch, completion, or synthesis policy. It only sequences
    /// the existing collection and physical-custody owners.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn open_consultation_collection<
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        input: ResidentConsultationOpenInput<'_>,
        resolve_store: &dyn super::OperationObservationAuthorityStoreResolver,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<CompletedConsultationCollection, ResidentConsultationOpenFailure>
    where
        AdmitCollection: FnOnce(
            &super::consultation::ConsultationCollection,
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
        super::consultation::coordinator::ResidentConsultationCoordinator::new(
            self.custody,
            self.anchor,
        )
        .open(
            input,
            resolve_store,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Continue only the retained physical custody phase of an initial collection.
    pub(in crate::resident) fn continue_consultation_collection_custody(
        &mut self,
        failure: ConsultationCollectionCommitFailure,
    ) -> Result<CompletedConsultationCollection, ConsultationCollectionCommitFailure> {
        failure.continue_custody(self.custody, self.anchor)
    }

    /// Recover one exact current Consult collection after durable opening.
    ///
    /// The operation and protected observation have already been selected by
    /// the process owner. This method performs no roster selection, dispatch,
    /// completion decision, or synthesis.
    pub(in crate::resident) fn recover_consultation_collection<Admit>(
        &mut self,
        operation: &IssuedLocalOperation,
        protected: &RecoveredOperationObservation,
        limits: super::consultation::ConsultationCollectionLimits,
        resolve_store: &dyn super::OperationObservationAuthorityStoreResolver,
        admit_collection: Admit,
    ) -> Result<
        Option<super::consultation::RecoveredConsultationCollection>,
        super::consultation::ConsultationCollectionError,
    >
    where
        Admit: FnOnce(
            &super::consultation::ConsultationCollection,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        super::consultation::coordinator::ResidentConsultationCoordinator::new(
            self.custody,
            self.anchor,
        )
        .recover(
            operation,
            protected,
            limits,
            resolve_store,
            admit_collection,
        )
    }

    /// Continue one exact local-operation start after retry-safe custody interruption.
    ///
    /// This projection cannot repeat semantic admission, signing, checkpoint
    /// construction, or identity allocation. The lower custody owners decide
    /// whether database or anchor mutation may be attempted again.
    pub(in crate::resident) fn continue_operation_start_custody(
        &mut self,
        failure: ResidentLocalOperationCustodyFailure,
        maximum_record_bytes: usize,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationCustodyFailure> {
        ResidentLocalOperationCoordinator::new(self.authority, self.custody, self.anchor)
            .continue_start_custody(failure, maximum_record_bytes)
    }

    /// Continue one already signed initial checkpoint without another signer.
    pub(in crate::resident) fn continue_operation_checkpoint_attachment(
        &mut self,
        failure: IssuedLocalOperationCheckpointFailure,
        maximum_record_bytes: usize,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure> {
        ResidentLocalOperationCoordinator::new(self.authority, self.custody, self.anchor)
            .continue_checkpoint_attachment(failure, maximum_record_bytes)
    }

    /// Retain a provider dispatch before offering its only ordinary call to the executor.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retain_and_accept_provider<
        'operation,
        'call,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Executor,
    >(
        &mut self,
        dispatch: ProviderOperationDispatchInput<'operation, 'call>,
        invocation: &ProviderOperationInvokeInput<'_>,
        executor: &mut Executor,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentOperationTaskKey, ResidentProviderTaskLaunchFailure<'operation, 'call>>
    where
        Authority: AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<
            super::operation_dispatch::OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
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
    {
        let retained = self
            .retain_provider_dispatch(dispatch, admit_rotation, sign_observation, sign_checkpoint)
            .map_err(|failure| match failure {
                ResidentProviderDispatchRetentionFailure::AggregateSaturated(dispatch) => {
                    ResidentProviderTaskLaunchFailure::AggregateSaturated(dispatch)
                }
                ResidentProviderDispatchRetentionFailure::Retention(reason) => {
                    ResidentProviderTaskLaunchFailure::Retention(reason)
                }
            })?;
        self.accept_retained_provider(retained, invocation, executor)
            .map_err(ResidentProviderTaskLaunchFailure::Acceptance)
    }

    /// Retain a provider dispatch without releasing it to an executor.
    ///
    /// This is the process-host seam between durable dispatch custody and the
    /// only ordinary provider call. It preserves the retained phase when a
    /// later host association check or acceptance must be retried.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retain_provider_dispatch<
        'operation,
        'call,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        dispatch: ProviderOperationDispatchInput<'operation, 'call>,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        RetainedProviderOperationDispatch<'operation>,
        ResidentProviderDispatchRetentionFailure<'operation, 'call>,
    >
    where
        Authority: AmbientSessionAuthorityPort,
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
    {
        let Self {
            authority,
            custody,
            anchor,
            tasks,
        } = self;
        if let Err(reason) = tasks.reservations.require_capacity() {
            return Err(match reason {
                ResidentTaskReservationOfferFailure::Saturated => {
                    ResidentProviderDispatchRetentionFailure::AggregateSaturated(Box::new(dispatch))
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
        ResidentLocalOperationCoordinator::new(*authority, *custody, *anchor)
            .retain_provider_dispatch(dispatch, admit_rotation, sign_observation, sign_checkpoint)
            .map_err(ResidentProviderDispatchRetentionFailure::Retention)
    }

    /// Offer an already retained provider dispatch to the process-local executor.
    ///
    /// This method performs release preflight but never repeats durable retention.
    pub(in crate::resident) fn accept_retained_provider<'operation, Executor>(
        &mut self,
        retained: super::operation_dispatch::RetainedProviderOperationDispatch<'operation>,
        invocation: &ProviderOperationInvokeInput<'_>,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentProviderTaskAcceptanceFailure<'operation>>
    where
        Authority: AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<
            super::operation_dispatch::OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
    {
        let Self {
            authority,
            custody,
            anchor,
            tasks,
        } = self;
        tasks.provider_tasks.try_accept(
            &mut ResidentLocalOperationCoordinator::new(*authority, *custody, *anchor),
            retained,
            invocation,
            &mut tasks.reservations,
            executor,
        )
    }

    /// Offer a lineage-proved pairwise Delegate dispatch to the provider executor.
    pub(in crate::resident) fn accept_retained_pairwise_delegate_provider<'operation, Executor>(
        &mut self,
        retained: super::operation_dispatch::RetainedProviderOperationDispatch<'operation>,
        invocation: &ProviderOperationInvokeInput<'_>,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentProviderTaskAcceptanceFailure<'operation>>
    where
        Authority: AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<
            super::operation_dispatch::OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
    {
        let Self {
            authority,
            custody,
            anchor,
            tasks,
        } = self;
        tasks.provider_tasks.try_accept_pairwise_delegate(
            &mut ResidentLocalOperationCoordinator::new(*authority, *custody, *anchor),
            retained,
            invocation,
            &mut tasks.reservations,
            executor,
        )
    }

    /// Offer one newly retained recovered provider dispatch to the executor.
    pub(in crate::resident) fn accept_recovered_provider<Executor>(
        &mut self,
        release: RecoveredProviderDispatchRelease,
        invocation: &ProviderOperationInvokeInput<'_>,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentRecoveredProviderTaskAcceptanceFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<
            super::operation_dispatch::OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
    {
        let Self {
            authority,
            custody,
            anchor,
            tasks,
        } = self;
        tasks.provider_tasks.try_accept_recovered(
            &mut ResidentLocalOperationCoordinator::new(*authority, *custody, *anchor),
            release,
            invocation,
            &mut tasks.reservations,
            executor,
        )
    }

    /// Retry task acceptance after saturation or executor refusal.
    ///
    /// The returned refusal still owns the unopened call. This path does not
    /// repeat durable dispatch retention or provider release preflight.
    /// Duplicate and preparation refusals are returned unchanged because they
    /// do not grant a retryable unopened call.
    pub(in crate::resident) fn continue_provider_acceptance<'operation, Executor>(
        &mut self,
        refusal: ResidentProviderTaskRefusal<'operation>,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentProviderTaskRefusal<'operation>>
    where
        Executor: ResidentOperationTaskExecutor<
            super::operation_dispatch::OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
    {
        self.tasks.provider_tasks.continue_refused_acceptance(
            refusal,
            &mut self.tasks.reservations,
            executor,
        )
    }

    /// Poll once and record one accepted provider completion without waiting.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn poll_provider_completion<
        'operation,
        Executor,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
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
    ) -> Option<ResidentProviderTaskPoll<'operation>>
    where
        Executor: ResidentOperationTaskExecutor<
            super::operation_dispatch::OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
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
        let Self {
            authority,
            custody,
            anchor,
            tasks,
        } = self;
        let mut coordinator = ResidentLocalOperationCoordinator::new(*authority, *custody, *anchor);
        tasks.provider_tasks.try_record_completion(
            &mut coordinator,
            &mut tasks.reservations,
            executor,
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

    /// Retry protected history recovery for one accepted completion.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn continue_provider_completion_recovery<
        'operation,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        completed: ResidentCompletedProviderTask,
        history: OperationAttemptHistoryInput<'operation>,
        recovered: &'operation RecoveredOperationObservation,
        admission: ProviderInvocationAdmissionContext<'operation>,
        input: &ProviderOperationInvokeInput<'_>,
        evidence: &dyn ProviderInvocationEvidenceAdmission,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProviderTaskPoll<'static>,
        super::provider_task_service::ResidentProviderCompletionRecoveryRejection,
    >
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
        let Self {
            authority,
            custody,
            anchor,
            tasks,
        } = self;
        tasks.provider_tasks.continue_recovered_completion(
            completed,
            &mut ResidentLocalOperationCoordinator::new(*authority, *custody, *anchor),
            &mut tasks.reservations,
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

    /// Retry only aggregate release after provider recording completed.
    pub(in crate::resident) fn continue_provider_aggregate_release(
        &mut self,
        result: ProviderOperationInvokeResult,
        cancellation: ResidentOperationTaskCancellationState,
        failure: ResidentTaskReservationSettlementTransitionFailure,
    ) -> ResidentProviderTaskPoll<'static> {
        self.tasks.provider_tasks.continue_aggregate_release(
            result,
            cancellation,
            failure,
            &mut self.tasks.reservations,
        )
    }

    /// Record one executor-lost accepted provider task as outcome unknown.
    ///
    /// Protected recovery must select the exact durable dispatch key. This path
    /// has no provider port and cannot replay the consumed call.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn recover_lost_provider<
        'operation,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        lost: ResidentLostProviderTask,
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
        let Self {
            authority,
            custody,
            anchor,
            tasks,
        } = self;
        tasks.provider_tasks.recover_lost(
            &mut ResidentLocalOperationCoordinator::new(*authority, *custody, *anchor),
            lost,
            &mut tasks.reservations,
            history,
            recovered,
            input,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Continue one consumed provider result without reopening the provider call.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn continue_provider_recording<
        'operation,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        continuation: ResidentProviderRecordingContinuation<'operation>,
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
        let Self {
            authority,
            custody,
            anchor,
            tasks,
        } = self;
        let mut coordinator = ResidentLocalOperationCoordinator::new(*authority, *custody, *anchor);
        tasks.provider_tasks.continue_recording(
            &mut coordinator,
            continuation,
            &mut tasks.reservations,
            input,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Rejoin consumed provider evidence after a concurrent cancellation head.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn continue_provider_after_concurrent_head<
        'operation,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        conflict: ResidentProviderRecordingConflict<'operation>,
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
        let Self {
            authority,
            custody,
            anchor,
            tasks,
        } = self;
        let mut coordinator = ResidentLocalOperationCoordinator::new(*authority, *custody, *anchor);
        tasks.provider_tasks.continue_after_concurrent_head(
            &mut coordinator,
            conflict,
            &mut tasks.reservations,
            recovered,
            input,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Continue only the exact provider terminal candidate already in custody.
    pub(in crate::resident) fn continue_provider_recording_custody(
        &mut self,
        custody: ResidentProviderRecordingCustody,
    ) -> Result<ResidentProviderTaskPoll<'static>, ResidentProviderRecordingCustodyRejection> {
        let Self {
            authority,
            custody: operation_custody,
            anchor,
            tasks,
        } = self;
        tasks.provider_tasks.continue_custody(
            &mut ResidentLocalOperationCoordinator::new(*authority, *operation_custody, *anchor),
            custody,
            &mut tasks.reservations,
        )
    }

    /// Retry semantic admission only; this path performs no external I/O.
    pub(in crate::resident) fn finish_provider_recording(
        &mut self,
        finalization: ResidentProviderRecordingFinalization,
    ) -> Result<ResidentProviderTaskPoll<'static>, ResidentProviderRecordingFinalizationRejection>
    {
        self.tasks
            .provider_tasks
            .finish_finalization(finalization, &mut self.tasks.reservations)
    }

    /// Recover, authorize, and advance one cancellation request.
    ///
    /// Database commitment and independent anchoring both complete before a
    /// selected provider or Consult child receives a notification, or before an
    /// A2A Delegate response read stops. A request that wins before dispatch
    /// reaches terminal local custody in the same call. The result never claims
    /// that external work stopped.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn request_cancellation(
        &mut self,
        input: ResidentOperationCancellationInput<'_>,
        local_completion: Option<ResidentLocalCancellationCompletionInput>,
        sessions: &dyn SessionLeaseCustodyPort,
        admission: ResidentOperationCancellationAdmission<'_>,
    ) -> Result<ResidentOperationCancellationOutcome, ResidentOperationCancellationFailure> {
        let (recovery, recovered_input) = split_cancellation_input(input);
        let (recovery_admission, recovered_admission) = split_cancellation_admission(admission);
        let recovered = recover_operation_control(
            recovery,
            self.custody,
            sessions,
            self.anchor,
            recovery_admission,
        )
        .map_err(ResidentOperationCancellationFailure::Recovery)?;
        self.request_cancellation_after_recovery(
            recovered,
            recovered_input,
            local_completion,
            recovered_admission,
        )
    }

    fn request_cancellation_after_recovery(
        &mut self,
        recovered: RecoveredOperationControl,
        input: ResidentRecoveredCancellationInput<'_>,
        local_completion: Option<ResidentLocalCancellationCompletionInput>,
        mut admission: ResidentRecoveredCancellationAdmission<'_>,
    ) -> Result<ResidentOperationCancellationOutcome, ResidentOperationCancellationFailure> {
        let authorized = recovered
            .authorize_cancellation(input.request, input.authorized_at, admission.authorize)
            .map_err(ResidentOperationCancellationFailure::Authorization)?;
        let ResidentAuthorizedCancellationRoute::Other(authorized) =
            self.route_authorized_cancellation(authorized)
        else {
            return Err(ResidentOperationCancellationFailure::ConsultationRouteRequired);
        };
        self.retain_non_consultation_cancellation(
            authorized,
            input.append,
            local_completion,
            &mut admission.append,
        )
    }

    /// Authorize and retain Cancel from one exact protected recovery.
    ///
    /// The caller owns point recovery and must have admitted the recovered
    /// locator before this method. This path does not repeat custody, session,
    /// checkpoint, or anchor reads.
    pub(in crate::resident) fn request_cancellation_from_recovered(
        &mut self,
        recovered: RecoveredOperationControl,
        input: ResidentOperationCancellationInput<'_>,
        local_completion: Option<ResidentLocalCancellationCompletionInput>,
        admission: ResidentOperationCancellationAdmission<'_>,
    ) -> Result<ResidentOperationCancellationOutcome, ResidentOperationCancellationFailure> {
        let (_, recovered_input) = split_cancellation_input(input);
        let (_, recovered_admission) = split_cancellation_admission(admission);
        self.request_cancellation_after_recovery(
            recovered,
            recovered_input,
            local_completion,
            recovered_admission,
        )
    }

    /// Recover, authorize, and route one consultation cancellation request.
    ///
    /// Protected operation recovery and current cancellation authorization each
    /// run once. An open consultation continues through its atomic roster owner.
    /// Every other state returns owned current evidence without invoking the
    /// local or dispatched-task cancellation path or appending a record.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn request_consultation_cancellation<AdmitStop, AdmitCollection>(
        &mut self,
        input: ResidentOperationCancellationInput<'_>,
        sessions: &dyn SessionLeaseCustodyPort,
        admission: ResidentOperationCancellationAdmission<'_>,
        collection_limits: super::consultation::ConsultationCollectionLimits,
        admit_stop: AdmitStop,
        admit_collection: AdmitCollection,
    ) -> Result<
        ResidentConsultationCancellationDisposition,
        ResidentConsultationCancellationRequestFailure,
    >
    where
        AdmitStop: FnOnce(
            &super::consultation::ConsultationCollection,
            &super::consultation::ConsultationRequirementSummary,
        ) -> Result<(), InvalidResidentRecordV1>,
        AdmitCollection: FnMut(
            &super::consultation::ConsultationCollection,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (recovery, recovered_input) = split_cancellation_input(input);
        let (recovery_admission, recovered_admission) = split_cancellation_admission(admission);
        let recovered = recover_operation_control(
            recovery,
            self.custody,
            sessions,
            self.anchor,
            recovery_admission,
        )
        .map_err(ResidentConsultationCancellationRequestFailure::Recovery)?;
        self.request_consultation_cancellation_after_recovery(
            recovered,
            recovered_input,
            recovered_admission,
            collection_limits,
            admit_stop,
            admit_collection,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn request_consultation_cancellation_after_recovery<AdmitStop, AdmitCollection>(
        &mut self,
        recovered: RecoveredOperationControl,
        input: ResidentRecoveredCancellationInput<'_>,
        mut admission: ResidentRecoveredCancellationAdmission<'_>,
        collection_limits: super::consultation::ConsultationCollectionLimits,
        admit_stop: AdmitStop,
        mut admit_collection: AdmitCollection,
    ) -> Result<
        ResidentConsultationCancellationDisposition,
        ResidentConsultationCancellationRequestFailure,
    >
    where
        AdmitStop: FnOnce(
            &super::consultation::ConsultationCollection,
            &super::consultation::ConsultationRequirementSummary,
        ) -> Result<(), InvalidResidentRecordV1>,
        AdmitCollection: FnMut(
            &super::consultation::ConsultationCollection,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let authorized = recovered
            .authorize_cancellation(input.request, input.authorized_at, admission.authorize)
            .map_err(ResidentConsultationCancellationRequestFailure::Authorization)?;
        match self.route_authorized_cancellation(authorized) {
            ResidentAuthorizedCancellationRoute::Consultation(routed) => self
                .cancel_authorized_consultation(
                    routed,
                    input.append,
                    collection_limits,
                    &mut admission.append,
                    admit_stop,
                    &mut admit_collection,
                )
                .map(ResidentConsultationCancellationDisposition::Retained)
                .map_err(ResidentConsultationCancellationRequestFailure::Consultation),
            ResidentAuthorizedCancellationRoute::Other(routed) => {
                let ResidentNonConsultationCancellation { authorized } = routed;
                let disposition = match authorized.disposition() {
                    OperationCancellationDisposition::Established((reference, observation)) => {
                        ResidentConsultationCancellationDisposition::Established(
                            ResidentOperationExistingState {
                                reference: reference.clone(),
                                observation: observation.clone(),
                            },
                        )
                    }
                    OperationCancellationDisposition::AlreadyRequested { request } => {
                        ResidentConsultationCancellationDisposition::AlreadyRequested(
                            request.clone(),
                        )
                    }
                    OperationCancellationDisposition::ConsultationCollected { evidence } => {
                        ResidentConsultationCancellationDisposition::ConsultationCollected(
                            evidence.clone(),
                        )
                    }
                    OperationCancellationDisposition::ReconciliationOnly((
                        reference,
                        observation,
                    )) => ResidentConsultationCancellationDisposition::ReconciliationOnly(
                        ResidentOperationExistingState {
                            reference: reference.clone(),
                            observation: observation.clone(),
                        },
                    ),
                    OperationCancellationDisposition::BeforeDispatch => {
                        let selected = authorized.recovered_observation().selected_head();
                        ResidentConsultationCancellationDisposition::NotCollecting(
                            ResidentConsultationNotCollecting::BeforeDispatch(
                                ResidentOperationExistingState {
                                    reference: selected.0.clone(),
                                    observation: selected.1.clone(),
                                },
                            ),
                        )
                    }
                    OperationCancellationDisposition::Dispatched { .. } => {
                        let selected = authorized.recovered_observation().selected_head();
                        ResidentConsultationCancellationDisposition::NotCollecting(
                            ResidentConsultationNotCollecting::Dispatched(
                                ResidentOperationExistingState {
                                    reference: selected.0.clone(),
                                    observation: selected.1.clone(),
                                },
                            ),
                        )
                    }
                    OperationCancellationDisposition::CollectingConsultation { .. } => {
                        return Err(ResidentConsultationCancellationRequestFailure::Routing);
                    }
                };
                Ok(disposition)
            }
        }
    }

    /// Route a Consult Cancel from one exact already protected recovery.
    ///
    /// The caller owns point recovery and locator admission. No historical or
    /// anti-rollback read is repeated here.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn request_consultation_cancellation_from_recovered<
        AdmitStop,
        AdmitCollection,
    >(
        &mut self,
        recovered: RecoveredOperationControl,
        input: ResidentOperationCancellationInput<'_>,
        admission: ResidentOperationCancellationAdmission<'_>,
        collection_limits: super::consultation::ConsultationCollectionLimits,
        admit_stop: AdmitStop,
        admit_collection: AdmitCollection,
    ) -> Result<
        ResidentConsultationCancellationDisposition,
        ResidentConsultationCancellationRequestFailure,
    >
    where
        AdmitStop: FnOnce(
            &super::consultation::ConsultationCollection,
            &super::consultation::ConsultationRequirementSummary,
        ) -> Result<(), InvalidResidentRecordV1>,
        AdmitCollection: FnMut(
            &super::consultation::ConsultationCollection,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (_, recovered_input) = split_cancellation_input(input);
        let (_, recovered_admission) = split_cancellation_admission(admission);
        self.request_consultation_cancellation_after_recovery(
            recovered,
            recovered_input,
            recovered_admission,
            collection_limits,
            admit_stop,
            admit_collection,
        )
    }

    /// Select the exact owner-specific route without signing or writing.
    pub(in crate::resident) fn route_authorized_cancellation<'operation>(
        &self,
        authorized: super::AuthorizedOperationCancellation<'operation>,
    ) -> ResidentAuthorizedCancellationRoute<'operation> {
        match authorized.disposition() {
            OperationCancellationDisposition::CollectingConsultation { evidence } => {
                ResidentAuthorizedCancellationRoute::Consultation(
                    ResidentConsultationCancellationRoute {
                        predecessor_frontier: evidence.clone(),
                        authorized,
                    },
                )
            }
            _ => ResidentAuthorizedCancellationRoute::Other(ResidentNonConsultationCancellation {
                authorized,
            }),
        }
    }

    /// Append one freshly authorized non-consultation cancellation request.
    pub(in crate::resident) fn retain_non_consultation_cancellation(
        &mut self,
        routed: ResidentNonConsultationCancellation<'_>,
        input: CancellationAppendInput<'_>,
        local_completion: Option<ResidentLocalCancellationCompletionInput>,
        admission: &mut CancellationAppendAdmission<'_>,
    ) -> Result<ResidentOperationCancellationOutcome, ResidentOperationCancellationFailure> {
        let ResidentNonConsultationCancellation { authorized } = routed;
        match authorized.disposition() {
            OperationCancellationDisposition::Established((reference, observation)) => Ok(
                ResidentOperationCancellationOutcome::Established(ResidentOperationExistingState {
                    reference: reference.clone(),
                    observation: observation.clone(),
                }),
            ),
            OperationCancellationDisposition::AlreadyRequested { request } => Ok(
                ResidentOperationCancellationOutcome::AlreadyRequested(request.clone()),
            ),
            OperationCancellationDisposition::CollectingConsultation { .. } => {
                Err(ResidentOperationCancellationFailure::ConsultationRouteRequired)
            }
            OperationCancellationDisposition::ConsultationCollected { evidence } => Ok(
                ResidentOperationCancellationOutcome::ConsultationCollected(evidence.clone()),
            ),
            OperationCancellationDisposition::ReconciliationOnly((reference, observation)) => {
                Ok(ResidentOperationCancellationOutcome::ReconciliationOnly(
                    ResidentOperationExistingState {
                        reference: reference.clone(),
                        observation: observation.clone(),
                    },
                ))
            }
            OperationCancellationDisposition::BeforeDispatch => {
                let prepared = authorized
                    .prepare_local_request(
                        LocalOperationCancellationRequestInput {
                            request_ref: input.request_ref,
                            observation_ref: input.observation_ref,
                            signing: input.signing,
                        },
                        &mut admission.admit_rotation,
                        &mut admission.sign_observation,
                    )
                    .map_err(ResidentOperationCancellationFailure::Request)?;
                let prior = prepared.predecessor_checkpoint();
                let checkpoint = prepared
                    .prepare_checkpoint(
                        input.accumulator_limits,
                        admission.resolve_store,
                        self.custody,
                    )
                    .map_err(OperationAppendFailure::Projection)
                    .map_err(ResidentOperationCancellationFailure::Custody)?;
                let (request_custody, request_checkpoint) = complete_prepared_checkpoint(
                    checkpoint,
                    prior,
                    input.checkpoint,
                    input.checkpoint_issued_at,
                    input.checkpoint_schema,
                    input.checkpoint_authority,
                    self.custody,
                    self.anchor,
                    &mut admission.sign_checkpoint,
                )
                .map_err(ResidentOperationCancellationFailure::Custody)?;
                let Some(local_completion) = local_completion else {
                    return Ok(ResidentOperationCancellationOutcome::LocalRequestRetained {
                        custody: request_custody,
                        checkpoint: Box::new(request_checkpoint),
                    });
                };
                let terminal = prepared
                    .prepare_completion_after_custody(
                        &request_custody,
                        &request_checkpoint,
                        LocalOperationCancellationCompletionInput {
                            diagnostic_ref: local_completion.diagnostic_ref,
                            observation_ref: local_completion.observation_ref,
                            established_at: local_completion.established_at,
                            signing: input.signing,
                            maximum_diagnostic_bytes: local_completion.maximum_diagnostic_bytes,
                            resolve_store: admission.resolve_store,
                        },
                        &mut admission.admit_rotation,
                        &mut admission.sign_observation,
                    )
                    .map_err(ResidentOperationCancellationFailure::Completion)?;
                let observation = (
                    terminal.observation().0.clone(),
                    terminal.observation().1.clone(),
                );
                let prior = terminal.predecessor_checkpoint();
                let checkpoint = terminal
                    .prepare_checkpoint(
                        input.accumulator_limits,
                        admission.resolve_store,
                        self.custody,
                    )
                    .map_err(OperationAppendFailure::Projection)
                    .map_err(ResidentOperationCancellationFailure::Custody)?;
                let (terminal_custody, terminal_checkpoint) = complete_prepared_checkpoint(
                    checkpoint,
                    prior,
                    local_completion.checkpoint,
                    local_completion.checkpoint_issued_at,
                    input.checkpoint_schema,
                    input.checkpoint_authority,
                    self.custody,
                    self.anchor,
                    &mut admission.sign_checkpoint,
                )
                .map_err(ResidentOperationCancellationFailure::Custody)?;
                Ok(ResidentOperationCancellationOutcome::LocallyCancelled {
                    request_custody,
                    request_checkpoint: Box::new(request_checkpoint),
                    terminal_custody,
                    terminal_checkpoint: Box::new(terminal_checkpoint),
                    observation: Box::new(observation),
                })
            }
            OperationCancellationDisposition::Dispatched { .. } => {
                let selected = authorized.recovered_observation().selected_head();
                let existing = || ResidentOperationExistingState {
                    reference: selected.0.clone(),
                    observation: selected.1.clone(),
                };
                let task_key = ResidentOperationTaskKey::new(
                    authorized.recovered_observation().store().clone(),
                    ImmutableRecordReferenceV1 {
                        record_ref: selected.0.clone(),
                        record_sha256: ArtifactDigest::parse(
                            selected.1.canonical_operation_observation_sha256().as_str(),
                        )
                        .map_err(|_| ResidentOperationCancellationFailure::Association)?,
                    },
                );
                let owner = match self.tasks.select_dispatched_cancellation_owner(&task_key) {
                    ResidentDispatchedCancellationOwnerSelection::Missing => {
                        return Ok(ResidentOperationCancellationOutcome::DispatchOwnerRequired(
                            existing(),
                        ));
                    }
                    ResidentDispatchedCancellationOwnerSelection::Owned(owner) => *owner,
                    ResidentDispatchedCancellationOwnerSelection::Ambiguous => {
                        return Err(ResidentOperationCancellationFailure::DispatchOwnerConflict);
                    }
                };
                let prepared = authorized
                    .prepare_dispatched_request(
                        DispatchedOperationCancellationRequestInput {
                            request_ref: input.request_ref,
                            observation_ref: input.observation_ref,
                            signing: input.signing,
                        },
                        &mut admission.admit_rotation,
                        &mut admission.sign_observation,
                    )
                    .map_err(ResidentOperationCancellationFailure::Request)?;
                let prior = prepared.predecessor_checkpoint();
                let checkpoint = prepared
                    .prepare_checkpoint(
                        input.accumulator_limits,
                        admission.resolve_store,
                        self.custody,
                    )
                    .map_err(OperationAppendFailure::Projection)
                    .map_err(ResidentOperationCancellationFailure::Custody)?;
                let (custody, checkpoint) = match complete_prepared_checkpoint(
                    checkpoint,
                    prior,
                    input.checkpoint,
                    input.checkpoint_issued_at,
                    input.checkpoint_schema,
                    input.checkpoint_authority,
                    self.custody,
                    self.anchor,
                    &mut admission.sign_checkpoint,
                ) {
                    Ok(completed) => completed,
                    Err(failure @ OperationAppendFailure::Commit { .. })
                    | Err(failure @ OperationAppendFailure::Anchor { .. }) => {
                        return Err(ResidentOperationCancellationFailure::DispatchedCustody(
                            Box::new(ResidentDispatchedCancellationCustody { failure, owner }),
                        ));
                    }
                    Err(failure) => {
                        return Err(ResidentOperationCancellationFailure::Custody(failure));
                    }
                };
                Ok(self.complete_dispatched_cancellation(owner, custody, checkpoint))
            }
        }
    }

    /// Continue exact post-dispatch custody, then notify only its selected owner.
    pub(in crate::resident) fn continue_dispatched_cancellation_custody(
        &mut self,
        pending: Box<ResidentDispatchedCancellationCustody>,
    ) -> Result<ResidentOperationCancellationOutcome, Box<ResidentDispatchedCancellationCustody>>
    {
        let ResidentDispatchedCancellationCustody { failure, owner } = *pending;
        match failure.continue_custody(self.custody, self.anchor) {
            Ok((custody, checkpoint)) => {
                Ok(self.complete_dispatched_cancellation(owner, custody, checkpoint))
            }
            Err(failure) => Err(Box::new(ResidentDispatchedCancellationCustody {
                failure,
                owner,
            })),
        }
    }

    /// Continue only an exact retained pre-dispatch cancellation custody candidate.
    pub(in crate::resident) fn continue_cancellation_custody(
        &mut self,
        failure: OperationAppendFailure,
    ) -> Result<(), OperationAppendFailure> {
        failure
            .continue_custody(self.custody, self.anchor)
            .map(|_| ())
    }

    fn complete_dispatched_cancellation(
        &mut self,
        owner: ResidentDispatchedCancellationOwner,
        custody: CompletedOperationObservationCustody,
        checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    ) -> ResidentOperationCancellationOutcome {
        let notification = match owner {
            ResidentDispatchedCancellationOwner::Provider(owner) => self
                .tasks
                .provider_tasks
                .note_dispatch_cancellation_notification(owner),
            ResidentDispatchedCancellationOwner::Consultation(owner) => self
                .tasks
                .consultation_tasks
                .note_dispatch_cancellation_notification(owner),
            ResidentDispatchedCancellationOwner::A2aDelegate(owner) => {
                let continuation = self
                    .tasks
                    .a2a_delegate_tasks
                    .stop_dispatch_response_read(owner, &mut self.tasks.reservations);
                return ResidentOperationCancellationOutcome::A2aDelegateReadStopped {
                    custody,
                    checkpoint: Box::new(checkpoint),
                    continuation,
                };
            }
        };
        ResidentOperationCancellationOutcome::DispatchedRequestRetained {
            custody,
            checkpoint: Box::new(checkpoint),
            notification,
        }
    }

    /// Retain a routed cancellation only when consultation custody is unnecessary.
    ///
    /// Open consultations return an explicit routing failure without signing or
    /// writing. Callers that can supply collection admission must first use
    /// [`Self::route_authorized_cancellation`] and preserve its linear route.
    pub(in crate::resident) fn retain_authorized_cancellation(
        &mut self,
        authorized: super::AuthorizedOperationCancellation<'_>,
        input: CancellationAppendInput<'_>,
        admission: &mut CancellationAppendAdmission<'_>,
    ) -> Result<ResidentOperationCancellationOutcome, ResidentOperationCancellationFailure> {
        match self.route_authorized_cancellation(authorized) {
            ResidentAuthorizedCancellationRoute::Consultation(_) => {
                Err(ResidentOperationCancellationFailure::ConsultationRouteRequired)
            }
            ResidentAuthorizedCancellationRoute::Other(routed) => {
                self.retain_non_consultation_cancellation(routed, input, None, admission)
            }
        }
    }

    /// Stop one authorized consultation, then mark its exact local batch.
    ///
    /// Collection recovery and cancellation use the same protected parent head.
    /// Database commitment and independent anchoring complete before any local
    /// accepted child is marked. A missing process-local batch after restart is
    /// an explicit advisory result and never blocks durable cancellation.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn cancel_authorized_consultation<
        'operation,
        AdmitStop,
        AdmitCollection,
    >(
        &mut self,
        routed: ResidentConsultationCancellationRoute<'operation>,
        input: CancellationAppendInput<'_>,
        collection_limits: super::consultation::ConsultationCollectionLimits,
        admission: &mut CancellationAppendAdmission<'_>,
        admit_stop: AdmitStop,
        mut admit_collection: AdmitCollection,
    ) -> Result<ResidentConsultationCancellationOutcome, ResidentConsultationCancellationFailure>
    where
        AdmitStop: FnOnce(
            &super::consultation::ConsultationCollection,
            &super::consultation::ConsultationRequirementSummary,
        ) -> Result<(), InvalidResidentRecordV1>,
        AdmitCollection: FnMut(
            &super::consultation::ConsultationCollection,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let ResidentConsultationCancellationRoute {
            authorized,
            predecessor_frontier,
        } = routed;
        let Self {
            custody,
            anchor,
            tasks,
            ..
        } = self;
        let mut coordinator =
            super::consultation::coordinator::ResidentConsultationCoordinator::new(
                *custody, *anchor,
            );
        let recovered = coordinator
            .recover(
                authorized.recovered_operation().operation(),
                authorized.recovered_observation(),
                collection_limits,
                admission.resolve_store,
                &mut admit_collection,
            )
            .map_err(ResidentConsultationCancellationFailure::Recovery)?
            .ok_or(ResidentConsultationCancellationFailure::Recovery(
                ConsultationCollectionError::Association,
            ))?;
        let local_coordinate = tasks
            .consultation_tasks
            .retained_batch_coordinate(recovered.store(), recovered.collection().parent())
            .filter(|coordinate| coordinate.collection_frontier() == &predecessor_frontier)
            .cloned();
        let completed = match coordinator.cancel(
            &authorized,
            &recovered,
            ConsultationCancellationInput {
                request_ref: input.request_ref,
                observation_ref: input.observation_ref,
                signing: input.signing,
                limits: collection_limits,
            },
            ConsultationSuccessorCommitInput {
                accumulator_limits: input.accumulator_limits,
                identity: input.checkpoint,
                issued_at: input.checkpoint_issued_at,
                schema: input.checkpoint_schema,
                authority: input.checkpoint_authority,
            },
            admission.resolve_store,
            admit_stop,
            &mut admit_collection,
            &mut admission.admit_rotation,
            &mut admission.sign_observation,
            &mut admission.sign_checkpoint,
        ) {
            Ok(completed) => completed,
            Err(ConsultationCancellationFailure::Authorization(reason)) => {
                return Err(ResidentConsultationCancellationFailure::Authorization(
                    reason,
                ));
            }
            Err(ConsultationCancellationFailure::Admission(reason)) => {
                return Err(ResidentConsultationCancellationFailure::Admission(reason));
            }
            Err(ConsultationCancellationFailure::Custody(failure)) => match failure {
                failure @ (ConsultationCollectionCommitFailure::Preparation { .. }
                | ConsultationCollectionCommitFailure::Issue { .. }) => {
                    return Err(ResidentConsultationCancellationFailure::Preparation(
                        Box::new(failure),
                    ));
                }
                failure @ (ConsultationCollectionCommitFailure::Commit { .. }
                | ConsultationCollectionCommitFailure::Anchor { .. }) => {
                    return Err(ResidentConsultationCancellationFailure::Custody(Box::new(
                        ResidentConsultationCancellationCustody {
                            failure,
                            local_coordinate,
                        },
                    )));
                }
            },
        };
        Ok(Self::complete_consultation_cancellation(
            tasks,
            completed,
            local_coordinate,
        ))
    }

    /// Resume exact post-write cancellation custody, then notify its retained batch.
    ///
    /// The continuation never re-signs or rebuilds the cancellation. A failed
    /// database read-back or anchor advance returns the same local coordinate
    /// with the exact physical continuation.
    pub(in crate::resident) fn continue_consultation_cancellation_custody(
        &mut self,
        pending: Box<ResidentConsultationCancellationCustody>,
    ) -> Result<ResidentConsultationCancellationOutcome, Box<ResidentConsultationCancellationCustody>>
    {
        let ResidentConsultationCancellationCustody {
            failure,
            local_coordinate,
        } = *pending;
        let completed = match failure.continue_custody(self.custody, self.anchor) {
            Ok(completed) => completed,
            Err(failure) => {
                return Err(Box::new(ResidentConsultationCancellationCustody {
                    failure,
                    local_coordinate,
                }));
            }
        };
        Ok(Self::complete_consultation_cancellation(
            self.tasks,
            completed,
            local_coordinate,
        ))
    }

    /// Preserve an exact pre-write Consult cancellation owner without rebuilding it.
    ///
    /// `Preparation` and `Issue` have no physical candidate, so the lower typed
    /// continuation returns them unchanged. Keeping this call beside the physical
    /// continuation prevents a later variant from being retried as fresh work.
    pub(in crate::resident) fn continue_consultation_cancellation_preparation(
        &mut self,
        failure: ConsultationCollectionCommitFailure,
    ) -> Result<(), Box<ConsultationCollectionCommitFailure>> {
        match failure.continue_custody(self.custody, self.anchor) {
            Ok(_) => Ok(()),
            Err(failure) => Err(Box::new(failure)),
        }
    }

    fn complete_consultation_cancellation(
        tasks: &mut ResidentOperationTaskState,
        completed: CompletedConsultationCollection,
        local_coordinate: Option<ResidentConsultationBatchCoordinate>,
    ) -> ResidentConsultationCancellationOutcome {
        let (custody, checkpoint) = completed.into_custody();
        let notification = local_coordinate.map_or(
            ResidentConsultationCancellationNotification::NoLocalOwner,
            |coordinate| {
                tasks
                    .consultation_tasks
                    .note_cancellation_notification(&coordinate)
            },
        );
        ResidentConsultationCancellationOutcome {
            custody,
            checkpoint,
            notification,
        }
    }
}

/// Why retained provider work did not reach the process-local executor.
pub(in crate::resident) enum ResidentProviderTaskLaunchFailure<'operation, 'call> {
    /// The process-wide aggregate ceiling was full before durable retention.
    AggregateSaturated(Box<ProviderOperationDispatchInput<'operation, 'call>>),
    /// Durable provider dispatch failed before process-local acceptance.
    Retention(ProviderOperationRetentionFailure<'operation>),
    /// Fresh preflight, bounds, duplicate detection, or executor acceptance failed.
    Acceptance(ResidentProviderTaskAcceptanceFailure<'operation>),
}

/// Why provider dispatch did not reach its retained pre-release phase.
pub(in crate::resident) enum ResidentProviderDispatchRetentionFailure<'operation, 'call> {
    /// Process-wide task capacity was full before durable retention.
    AggregateSaturated(Box<ProviderOperationDispatchInput<'operation, 'call>>),
    /// The existing durable provider-dispatch owner stopped at a recoverable phase.
    Retention(ProviderOperationRetentionFailure<'operation>),
}

#[cfg(test)]
#[path = "../../tests/internal/resident/operation_service.rs"]
pub(in crate::resident) mod tests;
