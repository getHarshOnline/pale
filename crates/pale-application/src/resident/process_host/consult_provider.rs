//! Run one roster-bound Consult child through an external provider.

use super::*;
use crate::provider_invocation::ProviderInvocationEvidenceAdmission;
use crate::resident::consultation::service::task::{
    ResidentConsultationTaskLossCommitFailure, ResidentConsultationTaskLossUnavailableReason,
    ResidentConsultationTaskOwnerPoll, ResidentConsultationTaskPoll,
};
use crate::resident::consultation::{
    CompletedResidentConsultationNonterminal, ResidentConsultationNonterminalSettlementFailure,
    ResidentConsultationTurnPending, ResidentConsultationTurnProgress,
    ResidentConsultationTurnStopped,
};
use crate::resident::operation_dispatch::{
    continue_consumed_provider_recording, continue_issued_recovered_provider_unknown,
    continue_prepared_recovered_provider_unknown, invoke_owned_provider_invocation,
    prepare_provider_operation_call, prepare_recovered_provider_completion,
    prepare_recovered_provider_operation_call, record_recovered_provider_operation_completion,
    record_recovered_provider_operation_unknown, retain_provider_operation_dispatch,
    validate_recovered_provider_call_references, validate_recovered_provider_unknown,
    OperationAttemptError, OperationAttemptHistoryInput, OwnedProviderInvocationRequest,
    PreparedProviderOperationCall, ProviderOperationCallBasis,
    ProviderOperationContinuationCoordinate, ProviderOperationDispatchInput,
    ProviderOperationInvokeFailure, ProviderOperationInvokeInput, ProviderOperationInvokeResult,
    ProviderOperationRecordingFailure, ProviderOperationRetentionFailure,
    RecoveredProviderCallPreparationFailure, RecoveredProviderDispatchRelease,
    RecoveredProviderUnknownRecordingFailure,
};
use crate::resident::process_host::consult_child::ResidentProcessConsultChildDriver;
use crate::resident::process_host::operation_resolution::ResidentProcessOperationResolutionFailure;
use crate::resident::process_host::provider_recovery::validate_provider_recovery_associations;
use crate::resident::process_host::restart::{
    ResidentProcessOperationLineage, ResidentProcessOperationResolutionPhase,
    ResidentProcessRecoveredAttempt, ResidentProcessRecoveredOperationResolution,
    ResidentProcessRecoveredOperationStart, ResidentProcessRecoveredPreparedAttempt,
    ResidentProcessRecoveredProviderResolution,
};
use crate::resident::{
    consultation::ResidentConsultationTaskLossPreparationFailure,
    task_owner::ResidentOperationTaskCancellationState, ConsultationChildDispatchBatch,
    ConsultationSlotId, IssuedLocalOperation, OperationCheckpointInput,
};
use pale_core::records::{
    ControlCheckpointSigningAuthorityAdmissionContextV1, OperationActivePhaseV1,
    OperationObservationStateV1, OperationObservationV1, ProviderBindingRefV1,
    ReconciliationFrontierReferenceV1, SchemaDigest, SignedResidentAdmissionContextV1,
};
use pale_ports::{
    AccumulatorPreparationLimits, ProviderInvocationObservation, ProviderInvocationPort,
};

type ResidentProcessConsultProviderMechanicalOutput = Result<
    ProviderInvocationObservation,
    crate::provider_invocation::ProviderInvocationAdmissionError,
>;

enum ResidentProcessConsultProviderCallBasis<'operation> {
    Fresh {
        basis: Box<ProviderOperationCallBasis<'operation>>,
        continuation: ProviderOperationContinuationCoordinate,
        admission: crate::provider_invocation::ProviderInvocationAdmissionContext<'operation>,
    },
    Recovered {
        continuation: ProviderOperationContinuationCoordinate,
        admission: crate::provider_invocation::ProviderInvocationAdmissionContext<'operation>,
    },
}

/// One exact Consult child retained at durable provider dispatch.
pub(super) struct ResidentProcessConsultProviderChildWork<'operation, 'call> {
    store: AuthorityStoreCoordinate,
    lineage: ResidentProcessOperationLineage,
    dispatch_ref: ResidentRecordRefV1,
    dispatch: OperationObservationV1,
    basis: Option<ResidentProcessConsultProviderCallBasis<'operation>>,
    request: OwnedProviderInvocationRequest,
    invocation: Option<ProviderOperationInvokeInput<'call>>,
}

impl ResidentProcessConsultProviderChildWork<'_, '_> {
    /// Borrow the durable dispatch offered to the Consult task owner.
    pub(super) fn dispatch(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.dispatch_ref, &self.dispatch)
    }
}

/// Why a provider-backed Consult child did not reach one retained dispatch.
pub(super) enum ResidentProcessConsultProviderChildRetentionFailure<'operation> {
    /// No durable provider phase crossed. The child can be offered again.
    Unopened(ResidentProcessConsultProviderUnopenedFailure),
    /// Durable provider progress may exist. Protected recovery must decide next.
    Crossed(ResidentProcessConsultProviderCrossedFailure<'operation>),
}

/// A provider-backed Consult child stopped before durable provider progress.
pub(super) enum ResidentProcessConsultProviderUnopenedFailure {
    /// Process, lineage, or provider selection differed before retention.
    Selection(ResidentProcessHostSelectionError),
    /// Provider admission failed before any signed provider phase existed.
    Admission(OperationAttemptError),
}

/// A provider-backed Consult child stopped after durable progress could exist.
pub(super) enum ResidentProcessConsultProviderCrossedFailure<'operation> {
    /// A post-retention association check differed.
    Selection(ResidentProcessHostSelectionError),
    /// Provider dispatch stopped in an existing recoverable phase.
    Provider(ProviderOperationRetentionFailure<'operation>),
    /// Retained dispatch could not become one unopened mechanical call.
    Call(ProviderOperationInvokeFailure<'operation>),
    /// A same-process recovered dispatch failed before mechanical release.
    RecoveredCall(RecoveredProviderCallPreparationFailure),
    /// A prepared call did not project its exact store-qualified dispatch key.
    CallIdentity {
        call: Box<PreparedProviderOperationCall<'operation>>,
        reason: OperationAttemptError,
    },
    /// Protected recovery did not admit the retained provider dispatch.
    Recovery(ResidentProcessOperationRestartError),
}

impl std::fmt::Debug for ResidentProcessConsultProviderChildRetentionFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Unopened(_) => "Unopened",
            Self::Crossed(_) => "Crossed",
        };
        formatter
            .debug_struct("ResidentProcessConsultProviderChildRetentionFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ResidentProcessConsultProviderUnopenedFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Selection(_) => "Selection",
            Self::Admission(_) => "Admission",
        };
        formatter
            .debug_struct("ResidentProcessConsultProviderUnopenedFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ResidentProcessConsultProviderCrossedFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Selection(_) => "Selection",
            Self::Provider(_) => "Provider",
            Self::Call(_) => "Call",
            Self::RecoveredCall(_) => "RecoveredCall",
            Self::CallIdentity { .. } => "CallIdentity",
            Self::Recovery(_) => "Recovery",
        };
        formatter
            .debug_struct("ResidentProcessConsultProviderCrossedFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Protected pre-dispatch phase that produced one same-process recovered release.
pub(super) enum ResidentProcessConsultProviderResumePhase<'operation> {
    /// Provider selection was durable before this process allocated its attempt.
    ProviderResolved(&'operation ResidentProcessRecoveredProviderResolution),
    /// Attempt preparation was durable before this process committed dispatch.
    DispatchPrepared(&'operation ResidentProcessRecoveredPreparedAttempt),
}

impl ResidentProcessConsultProviderResumePhase<'_> {
    fn start(&self) -> &ResidentProcessRecoveredOperationStart {
        match self {
            Self::ProviderResolved(recovered) => &recovered.start,
            Self::DispatchPrepared(recovered) => &recovered.start,
        }
    }
}

/// Why one protected pre-dispatch phase did not resume through its Consult owner.
pub(super) enum ResidentProcessConsultProviderResumeFailure<'operation, 'call> {
    /// Phase, deployment, lineage, or recovered dispatch selection differed.
    Selection {
        release: Box<RecoveredProviderDispatchRelease>,
        reason: ResidentProcessHostSelectionError,
    },
    /// Exact resumed Consult ownership was unavailable before provider preflight.
    Ownership {
        release: Box<RecoveredProviderDispatchRelease>,
        reason: crate::resident::consultation::ResidentConsultationTaskResumePreparationFailure,
    },
    /// Protected recovery did not select the newly committed provider dispatch.
    Recovery {
        release: Box<RecoveredProviderDispatchRelease>,
        reason: ResidentProcessOperationRestartError,
    },
    /// The recovered dispatch could not become one fresh mechanical call.
    Call(RecoveredProviderCallPreparationFailure),
    /// Prepared call identity diverged after its pure release checks.
    CallIdentity {
        prepared: Box<crate::resident::consultation::PreparedResidentConsultationTaskResume>,
        work: Box<ResidentProcessConsultProviderChildWork<'operation, 'call>>,
        reason: ResidentProcessHostSelectionError,
    },
    /// Local batch ownership could not reconcile the recovered parent frontier.
    Batch {
        release: Box<RecoveredProviderDispatchRelease>,
        reason:
            crate::resident::consultation::service::task::ResidentConsultationBatchReconcileError,
    },
    /// A stale busy batch prevented this recovered frontier from becoming usable.
    Busy(Box<RecoveredProviderDispatchRelease>),
    /// The exact task owner rejected the protected dispatch or its work.
    Acceptance(
        Box<
            crate::resident::consultation::ResidentConsultationPreparedResumeFailure<
                ResidentProcessConsultProviderChildWork<'operation, 'call>,
            >,
        >,
    ),
}

impl std::fmt::Debug for ResidentProcessConsultProviderResumeFailure<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Selection { .. } => "Selection",
            Self::Ownership { .. } => "Ownership",
            Self::Recovery { .. } => "Recovery",
            Self::Call(_) => "Call",
            Self::CallIdentity { .. } => "CallIdentity",
            Self::Batch { .. } => "Batch",
            Self::Busy(_) => "Busy",
            Self::Acceptance(_) => "Acceptance",
        };
        formatter
            .debug_struct("ResidentProcessConsultProviderResumeFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

pub(super) enum ResidentProcessConsultProviderChildOutput {
    /// The executor returned raw provider evidence which still needs admission.
    Mechanical(ResidentProcessConsultProviderMechanicalOutput),
    /// The application admitted and retained the exact terminal disposition.
    Recorded(ProviderOperationInvokeResult),
}

/// Durable classification after one accepted provider child was recorded.
pub(super) enum ResidentProcessConsultProviderCompletionDisposition<'operation, 'call> {
    /// Conclusive child evidence can enter the parent collection.
    Terminal {
        ready: Box<
            ResidentProcessConsultationCompletionReady<
                ResidentProcessConsultProviderChildWork<'operation, 'call>,
                ResidentProcessConsultProviderChildOutput,
            >,
        >,
        child: Box<ResidentProcessRecoveredOperationStart>,
    },
    /// Durable uncertainty released capacity without changing the parent.
    Nonterminal(
        Box<
            CompletedResidentConsultationNonterminal<
                ResidentProcessConsultProviderChildWork<'operation, 'call>,
                ResidentProcessConsultProviderChildOutput,
            >,
        >,
    ),
}

/// Parent state after an admitted provider child completion is consumed.
pub(super) enum ResidentProcessConsultProviderParentDisposition<'operation, 'call> {
    /// Conclusive child evidence joined its exact parent slot.
    Joined(
        Box<
            CompletedResidentConsultationTerminal<
                ResidentProcessConsultProviderChildWork<'operation, 'call>,
                ResidentProcessConsultProviderChildOutput,
            >,
        >,
    ),
    /// Durable uncertainty released capacity without changing a parent slot.
    Nonterminal(
        Box<
            CompletedResidentConsultationNonterminal<
                ResidentProcessConsultProviderChildWork<'operation, 'call>,
                ResidentProcessConsultProviderChildOutput,
            >,
        >,
    ),
}

/// Accepted completion whose provider recording stopped after the call.
pub(super) struct ResidentProcessConsultProviderRecordingFailure<'operation, 'call> {
    selector: ResidentDeploymentAuthoritySelector,
    current: RecoveredConsultationCollection,
    coordinate: crate::resident::consultation::ResidentConsultationBatchCoordinate,
    continuation:
        crate::resident::consultation::service::task::ResidentConsultationContinuationToken,
    candidate: ConsultationChildDispatchCandidate,
    dispatch: crate::resident::task_owner::ResidentOperationTaskKey,
    work: ResidentProcessConsultProviderChildWork<'operation, 'call>,
    cancellation: ResidentOperationTaskCancellationState,
    reason: ProviderOperationRecordingFailure<'operation>,
}

/// A lost provider task whose durable unknown outcome still needs recording.
pub(super) struct ResidentProcessConsultProviderLostRecordingFailure<'operation, 'call> {
    ready: Box<
        ResidentProcessConsultationCompletionReady<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
    >,
    reason: RecoveredProviderUnknownRecordingFailure,
}

/// Exact recorded completion awaiting protected classification.
pub(super) struct ResidentProcessConsultProviderClassificationFailure<'operation, 'call> {
    ready: Box<
        ResidentProcessConsultationCompletionReady<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
    >,
    reason: ResidentProcessOperationRestartError,
}

enum ResidentProcessConsultProviderRecordedDisposition {
    Terminal,
    RecoveryOnly,
}

/// Why an accepted provider child did not reach its Consult disposition.
pub(super) enum ResidentProcessConsultProviderCompletionFailure<'operation, 'call> {
    /// The completion or its work belongs to another deployment or child.
    Selection {
        ready: Box<
            ResidentProcessConsultationCompletionReady<
                ResidentProcessConsultProviderChildWork<'operation, 'call>,
                ResidentProcessConsultProviderChildOutput,
            >,
        >,
        reason: ResidentProcessHostSelectionError,
    },
    /// Protected dispatch rejoin rejected before terminal recording.
    Recovery {
        ready: Box<
            ResidentProcessConsultationCompletionReady<
                ResidentProcessConsultProviderChildWork<'operation, 'call>,
                ResidentProcessConsultProviderChildOutput,
            >,
        >,
        reason: OperationAttemptError,
    },
    /// A consumed completion now belongs to its exact recording continuation.
    Recording(Box<ResidentProcessConsultProviderRecordingFailure<'operation, 'call>>),
    /// Lost work could not retain its protected outcome-unknown successor.
    LostRecording(Box<ResidentProcessConsultProviderLostRecordingFailure<'operation, 'call>>),
    /// Protected read-back did not select the expected durable disposition.
    Classification(Box<ResidentProcessConsultProviderClassificationFailure<'operation, 'call>>),
    /// Durable uncertainty failed exact process-local settlement.
    Settlement {
        failure: ResidentProcessConsultProviderSettlementFailure<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
        child: Box<ResidentProcessRecoveredOperationStart>,
    },
}

/// Protected inputs for classifying the child after completion custody.
pub(super) struct ResidentProcessConsultProviderRecordedRecovery<'input, 'operation> {
    pub(super) association: ResidentOperationControlAssociation<'input>,
    pub(super) restart: ResidentProcessConsultProviderRepeatableRestart<'input, 'operation>,
    pub(super) admission: &'input mut OperationControlRecoveryAdmission<'input>,
    pub(super) parent: &'operation ResidentProcessRecoveredOperationStart,
    pub(super) planned: &'input ConsultationChildDispatchBatch,
    pub(super) observed_at: UnixTimeV1,
}

type ResidentProcessConsultProviderRotation<'input> = Box<
    dyn FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>
        + 'input,
>;

type ResidentProcessConsultProviderObservationSigner<'input> = Box<
    dyn FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
        + 'input,
>;

type ResidentProcessConsultProviderCheckpointSigner<'input> = Box<
    dyn FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
        + 'input,
>;

type ResidentProcessConsultProviderEvidenceAdmission<'input> = Box<
    dyn FnOnce(
            &crate::resident::consultation::ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>
        + 'input,
>;

type ResidentProcessConsultProviderCollectionAdmission<'input> =
    Box<dyn FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1> + 'input>;

type ResidentProcessConsultProviderReconciliationAdmission<'input> = Box<
    dyn FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1> + 'input,
>;

type ResidentProcessConsultProviderJoinTime<'input> =
    Box<dyn FnOnce(&ResidentProcessRecoveredOperationStart) -> UnixTimeV1 + 'input>;

/// Fresh one-shot callbacks consumed by exactly one selected recording attempt.
pub(super) struct ResidentProcessConsultProviderRecordingAttempt<'input> {
    pub(super) reconcile: ResidentProcessConsultProviderReconciliationAdmission<'input>,
    pub(super) rotate: ResidentProcessConsultProviderRotation<'input>,
    pub(super) sign_observation: ResidentProcessConsultProviderObservationSigner<'input>,
    pub(super) sign_checkpoint: ResidentProcessConsultProviderCheckpointSigner<'input>,
}

/// One consumed provider completion and its only safe continuation.
pub(super) enum ResidentProcessConsultProviderCompletionContinuation<'input, 'operation, 'call> {
    Recording {
        access: ResidentDeploymentOperationAccess<'input>,
        failure: Box<ResidentProcessConsultProviderRecordingFailure<'operation, 'call>>,
        recovered_head: Option<&'operation RecoveredOperationObservation>,
        recovery: ResidentProcessConsultProviderRecordedRecovery<'input, 'operation>,
    },
    LostRecording {
        access: ResidentDeploymentOperationAccess<'input>,
        failure: ResidentProcessConsultProviderLostRecordingFailure<'operation, 'call>,
        history: OperationAttemptHistoryInput<'operation>,
        recovered_dispatch: &'operation RecoveredOperationObservation,
        input: &'input ProviderOperationInvokeInput<'call>,
        recovery: ResidentProcessConsultProviderRecordedRecovery<'input, 'operation>,
    },
    Classification {
        access: ResidentDeploymentOperationAccess<'input>,
        failure: Box<ResidentProcessConsultProviderClassificationFailure<'operation, 'call>>,
        recovery: ResidentProcessConsultProviderRecordedRecovery<'input, 'operation>,
    },
    Settlement {
        failure: ResidentProcessConsultProviderSettlementFailure<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
        child: Box<ResidentProcessRecoveredOperationStart>,
    },
}

/// One continuation plus the fresh one-shot inputs selected for this attempt.
pub(super) struct ResidentProcessConsultProviderCompletionContinuationTurn<
    'input,
    'operation,
    'call,
> {
    continuation:
        Box<ResidentProcessConsultProviderCompletionContinuation<'input, 'operation, 'call>>,
    recording: Option<ResidentProcessConsultProviderRecordingAttempt<'input>>,
    checkpoint_signer: Option<ResidentProcessConsultProviderCheckpointSigner<'input>>,
}

/// Initial completion stopped before or after a resumable phase boundary.
pub(super) enum ResidentProcessConsultProviderFinalizeFailure<'input, 'operation, 'call> {
    BeforeContinuation(Box<ResidentProcessConsultProviderCompletionFailure<'operation, 'call>>),
    Continuation(
        Box<ResidentProcessConsultProviderCompletionContinuation<'input, 'operation, 'call>>,
    ),
}

impl std::fmt::Debug for ResidentProcessConsultProviderFinalizeFailure<'_, '_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::BeforeContinuation(_) => "BeforeContinuation",
            Self::Continuation(continuation) => match continuation.as_ref() {
                ResidentProcessConsultProviderCompletionContinuation::Recording { .. } => {
                    "Recording"
                }
                ResidentProcessConsultProviderCompletionContinuation::LostRecording { .. } => {
                    "LostRecording"
                }
                ResidentProcessConsultProviderCompletionContinuation::Classification { .. } => {
                    "Classification"
                }
                ResidentProcessConsultProviderCompletionContinuation::Settlement { .. } => {
                    "Settlement"
                }
            },
        };
        let mut debug = formatter.debug_struct("ResidentProcessConsultProviderFinalizeFailure");
        debug.field("phase", &phase);
        if let Self::Continuation(continuation) = self {
            if let ResidentProcessConsultProviderCompletionContinuation::Classification {
                failure,
                ..
            } = continuation.as_ref()
            {
                debug.field("reason", &failure.reason);
            }
        }
        debug.finish_non_exhaustive()
    }
}

/// A rejected continuation that has not run any recovery step.
pub(super) struct ResidentProcessConsultProviderContinuationRefusal<'input, 'operation, 'call> {
    turn: Box<ResidentProcessConsultProviderCompletionContinuationTurn<'input, 'operation, 'call>>,
    reason: ResidentProcessHostSelectionError,
}

/// Why one retained completion continuation did not advance.
pub(super) enum ResidentProcessConsultProviderContinuationFailure<'input, 'operation, 'call> {
    /// Deployment selection rejected before the continuation ran.
    Selection(Box<ResidentProcessConsultProviderContinuationRefusal<'input, 'operation, 'call>>),
    /// The selected phase owner returned the complete retry capability.
    Completion(
        Box<ResidentProcessConsultProviderCompletionContinuation<'input, 'operation, 'call>>,
    ),
}

impl<'input, 'operation, 'call>
    ResidentProcessConsultProviderContinuationRefusal<'input, 'operation, 'call>
{
    pub(super) fn into_parts(
        self,
    ) -> (
        ResidentProcessConsultProviderCompletionContinuationTurn<'input, 'operation, 'call>,
        ResidentProcessHostSelectionError,
    ) {
        (*self.turn, self.reason)
    }
}

impl<'input, 'operation, 'call>
    ResidentProcessConsultProviderCompletionContinuation<'input, 'operation, 'call>
{
    pub(super) fn settlement(
        failure: ResidentProcessConsultProviderSettlementFailure<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
        child: Box<ResidentProcessRecoveredOperationStart>,
    ) -> Self {
        Self::Settlement { failure, child }
    }

    fn selector(&self) -> &ResidentDeploymentAuthoritySelector {
        match self {
            Self::Recording { failure, .. } => &failure.selector,
            Self::LostRecording { failure, .. } => &failure.ready.selector,
            Self::Classification { failure, .. } => &failure.ready.selector,
            Self::Settlement { child, .. } => &child.selector,
        }
    }

    fn requires_recording_attempt(&self) -> bool {
        match self {
            Self::Recording { failure, .. } => matches!(
                failure.reason,
                ProviderOperationRecordingFailure::Preparation { .. }
                    | ProviderOperationRecordingFailure::ConcurrentHead { .. }
            ),
            Self::LostRecording { failure, .. } => matches!(
                failure.reason,
                RecoveredProviderUnknownRecordingFailure::Preparation(_)
            ),
            Self::Classification { .. } | Self::Settlement { .. } => false,
        }
    }

    fn requires_checkpoint_signer(&self) -> bool {
        matches!(
            self,
            Self::LostRecording { failure, .. }
                if matches!(
                    failure.reason,
                    RecoveredProviderUnknownRecordingFailure::Prepared(_)
                )
        )
    }

    pub(super) fn with_recording_attempt(
        self,
        recording: ResidentProcessConsultProviderRecordingAttempt<'input>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionContinuationTurn<'input, 'operation, 'call>,
        Box<Self>,
    > {
        if !self.requires_recording_attempt() {
            return Err(Box::new(self));
        }
        Ok(ResidentProcessConsultProviderCompletionContinuationTurn {
            continuation: Box::new(self),
            recording: Some(recording),
            checkpoint_signer: None,
        })
    }

    pub(super) fn with_checkpoint_signer(
        self,
        checkpoint_signer: ResidentProcessConsultProviderCheckpointSigner<'input>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionContinuationTurn<'input, 'operation, 'call>,
        Box<Self>,
    > {
        if !self.requires_checkpoint_signer() {
            return Err(Box::new(self));
        }
        Ok(ResidentProcessConsultProviderCompletionContinuationTurn {
            continuation: Box::new(self),
            recording: None,
            checkpoint_signer: Some(checkpoint_signer),
        })
    }

    pub(super) fn without_recording_attempt(
        self,
    ) -> Result<
        ResidentProcessConsultProviderCompletionContinuationTurn<'input, 'operation, 'call>,
        Box<Self>,
    > {
        if self.requires_recording_attempt() || self.requires_checkpoint_signer() {
            return Err(Box::new(self));
        }
        Ok(ResidentProcessConsultProviderCompletionContinuationTurn {
            continuation: Box::new(self),
            recording: None,
            checkpoint_signer: None,
        })
    }
}

impl ResidentProcessConsultProviderCompletionContinuationTurn<'_, '_, '_> {
    fn selector(&self) -> &ResidentDeploymentAuthoritySelector {
        self.continuation.selector()
    }
}

impl std::fmt::Debug for ResidentProcessConsultProviderCompletionFailure<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Selection { .. } => "Selection",
            Self::Recovery { .. } => "Recovery",
            Self::Recording(_) => "Recording",
            Self::LostRecording(_) => "LostRecording",
            Self::Classification(_) => "Classification",
            Self::Settlement { .. } => "Settlement",
        };
        formatter
            .debug_struct("ResidentProcessConsultProviderCompletionFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Why durable provider uncertainty did not release Consult capacity.
pub(super) enum ResidentProcessConsultProviderSettlementFailure<Work, Output> {
    /// The completion or recovered child belongs to another deployment.
    Selection {
        ready: Box<ResidentProcessConsultationCompletionReady<Work, Output>>,
        reason: ResidentProcessHostSelectionError,
    },
    /// The Consult owner rejected the uncertainty proof or aggregate release.
    Settlement(ResidentConsultationNonterminalSettlementFailure<Work, Output>),
}

/// Reconstructs classification inputs after a protected retryable refusal.
type ResidentProcessConsultProviderRepeatableRestart<'input, 'operation> =
    Box<dyn FnMut() -> ResidentProcessOperationRestartInput<'operation> + 'input>;

type ResidentProcessConsultProviderRotationMut<'input> = Box<
    dyn FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>
        + 'input,
>;

type ResidentProcessConsultProviderObservationSignerMut<'input> = Box<
    dyn FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
        + 'input,
>;

type ResidentProcessConsultProviderCheckpointSignerMut<'input> = Box<
    dyn FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
        + 'input,
>;

/// Inputs that open one fresh provider-backed Consult child.
pub(super) struct ResidentProcessConsultProviderOpenTurn<
    'input,
    'operation,
    'call,
    Authority,
    Executor,
> {
    pub(super) association: ResidentOperationControlAssociation<'input>,
    pub(super) access: ResidentDeploymentOperationAccess<'input>,
    pub(super) authority: &'input mut Authority,
    pub(super) task_limits: ResidentConsultationTaskLimits,
    pub(super) observed_at: UnixTimeV1,
    pub(super) current: RecoveredConsultationCollection,
    pub(super) planned: &'input ConsultationChildDispatchBatch,
    pub(super) candidate: ConsultationChildDispatchCandidate,
    pub(super) parent: &'operation ResidentProcessRecoveredOperationStart,
    pub(super) recovered: &'operation ResidentProcessRecoveredOperationResolution,
    pub(super) restart: ResidentProcessConsultProviderRepeatableRestart<'input, 'operation>,
    pub(super) admission: &'input mut OperationControlRecoveryAdmission<'input>,
    pub(super) dispatch: ProviderOperationDispatchInput<'operation, 'call>,
    pub(super) invocation: ProviderOperationInvokeInput<'call>,
    pub(super) executor: &'input mut Executor,
    pub(super) rotate: ResidentProcessConsultProviderRotationMut<'input>,
    pub(super) sign_observation: ResidentProcessConsultProviderObservationSignerMut<'input>,
    pub(super) sign_checkpoint: ResidentProcessConsultProviderCheckpointSignerMut<'input>,
}

/// Inputs that resume one same-process provider release.
pub(super) struct ResidentProcessConsultProviderResumeTurn<
    'input,
    'operation,
    'call,
    Authority,
    Executor,
> {
    pub(super) association: ResidentOperationControlAssociation<'input>,
    pub(super) access: ResidentDeploymentOperationAccess<'input>,
    pub(super) authority: &'input mut Authority,
    pub(super) task_limits: ResidentConsultationTaskLimits,
    pub(super) observed_at: UnixTimeV1,
    pub(super) current: RecoveredConsultationCollection,
    pub(super) planned: &'input ConsultationChildDispatchBatch,
    pub(super) candidate: &'input ConsultationChildDispatchCandidate,
    pub(super) parent: &'input ResidentProcessRecoveredOperationStart,
    pub(super) phase: ResidentProcessConsultProviderResumePhase<'operation>,
    pub(super) release: RecoveredProviderDispatchRelease,
    pub(super) invocation: ProviderOperationInvokeInput<'call>,
    pub(super) provider_admission:
        crate::provider_invocation::ProviderInvocationAdmissionContext<'operation>,
    pub(super) restart: ResidentProcessConsultProviderRepeatableRestart<'input, 'operation>,
    pub(super) admission: &'input mut OperationControlRecoveryAdmission<'input>,
    pub(super) executor: &'input mut Executor,
}

/// Exact cold provider dispatch plus its Consult owner and recording inputs.
pub(super) struct ResidentProcessConsultProviderColdLossTurn<'input, 'operation, 'call> {
    pub(super) attempt: &'operation ResidentProcessRecoveredAttempt,
    pub(super) access: ResidentDeploymentOperationAccess<'input>,
    pub(super) task_limits: ResidentConsultationTaskLimits,
    pub(super) current: RecoveredConsultationCollection,
    pub(super) candidate: &'input ConsultationChildDispatchCandidate,
    pub(super) history: OperationAttemptHistoryInput<'operation>,
    pub(super) input: &'input ProviderOperationInvokeInput<'call>,
    pub(super) evidence: &'input dyn ProviderInvocationEvidenceAdmission,
    pub(super) recovery: ResidentProcessConsultProviderRecordedRecovery<'input, 'operation>,
    pub(super) recording: ResidentProcessConsultProviderRecordingAttempt<'input>,
}

/// Recording state retained across one exact local recovered-loss transition.
pub(super) struct ResidentProcessConsultProviderColdLossRecording<'input, 'operation, 'call> {
    selector: ResidentDeploymentAuthoritySelector,
    access: ResidentDeploymentOperationAccess<'input>,
    current: RecoveredConsultationCollection,
    history: OperationAttemptHistoryInput<'operation>,
    recovered_dispatch: &'operation RecoveredOperationObservation,
    input: &'input ProviderOperationInvokeInput<'call>,
    evidence: &'input dyn ProviderInvocationEvidenceAdmission,
    recovery: ResidentProcessConsultProviderRecordedRecovery<'input, 'operation>,
    recording: ResidentProcessConsultProviderRecordingAttempt<'input>,
}

/// Exact prepared owner refused before an aggregate offer existed.
pub(super) struct ResidentProcessConsultProviderColdLossPrepared<'input, 'operation, 'call> {
    recording: Box<ResidentProcessConsultProviderColdLossRecording<'input, 'operation, 'call>>,
    prepared: Box<crate::resident::consultation::PreparedResidentConsultationTaskLoss>,
    reason: crate::resident::task_reservation::ResidentTaskReservationOfferFailure,
}

/// Mechanically retryable exact loss transition for one existing aggregate offer.
pub(super) struct ResidentProcessConsultProviderColdLossOffered<'input, 'operation, 'call> {
    recording: Box<ResidentProcessConsultProviderColdLossRecording<'input, 'operation, 'call>>,
    pending: Box<crate::resident::consultation::ResidentConsultationTaskLossOffer>,
    reason: crate::resident::task_reservation::ResidentTaskReservationTransitionFailure,
}

/// Exact non-retryable local owner found unavailable after preparation.
pub(super) struct ResidentProcessConsultProviderColdLossUnavailable<'input, 'operation, 'call> {
    recording: Box<ResidentProcessConsultProviderColdLossRecording<'input, 'operation, 'call>>,
    prepared: Box<crate::resident::consultation::PreparedResidentConsultationTaskLoss>,
    reason: ResidentConsultationTaskLossUnavailableReason,
}

/// Cold loss stopped before local loss ownership or durable recording completed.
pub(super) enum ResidentProcessConsultProviderColdLossFailure<'input, 'operation, 'call> {
    Selection(
        ResidentProcessConsultProviderPhaseRefusal<
            ResidentProcessConsultProviderColdLossTurn<'input, 'operation, 'call>,
        >,
    ),
    Batch {
        turn: Box<ResidentProcessConsultProviderColdLossTurn<'input, 'operation, 'call>>,
        reason:
            crate::resident::consultation::service::task::ResidentConsultationBatchReconcileError,
    },
    Busy {
        turn: Box<ResidentProcessConsultProviderColdLossTurn<'input, 'operation, 'call>>,
        reason: crate::resident::consultation::ResidentConsultationBatchRetentionReason,
    },
    Ownership {
        turn: Box<ResidentProcessConsultProviderColdLossTurn<'input, 'operation, 'call>>,
        reason: ResidentConsultationTaskLossPreparationFailure,
    },
    Unavailable(Box<ResidentProcessConsultProviderColdLossUnavailable<'input, 'operation, 'call>>),
    Prepared(Box<ResidentProcessConsultProviderColdLossPrepared<'input, 'operation, 'call>>),
    Offered(Box<ResidentProcessConsultProviderColdLossOffered<'input, 'operation, 'call>>),
    Completion(Box<ResidentProcessConsultProviderFinalizeFailure<'input, 'operation, 'call>>),
}

impl ResidentProcessConsultProviderColdLossUnavailable<'_, '_, '_> {
    pub(super) const fn prepared(
        &self,
    ) -> &crate::resident::consultation::PreparedResidentConsultationTaskLoss {
        &self.prepared
    }

    pub(super) const fn reason(&self) -> &ResidentConsultationTaskLossUnavailableReason {
        &self.reason
    }
}

impl ResidentProcessConsultProviderColdLossPrepared<'_, '_, '_> {
    pub(super) const fn prepared(
        &self,
    ) -> &crate::resident::consultation::PreparedResidentConsultationTaskLoss {
        &self.prepared
    }

    pub(super) const fn reason(
        &self,
    ) -> &crate::resident::task_reservation::ResidentTaskReservationOfferFailure {
        &self.reason
    }
}

impl ResidentProcessConsultProviderColdLossOffered<'_, '_, '_> {
    fn selector(&self) -> &ResidentDeploymentAuthoritySelector {
        &self.recording.selector
    }

    pub(super) const fn pending(
        &self,
    ) -> &crate::resident::consultation::ResidentConsultationTaskLossOffer {
        &self.pending
    }

    pub(super) const fn reason(
        &self,
    ) -> &crate::resident::task_reservation::ResidentTaskReservationTransitionFailure {
        &self.reason
    }
}

fn cold_loss_local_failure<'input, 'operation, 'call>(
    recording: Box<ResidentProcessConsultProviderColdLossRecording<'input, 'operation, 'call>>,
    failure: ResidentConsultationTaskLossCommitFailure,
) -> Box<ResidentProcessConsultProviderColdLossFailure<'input, 'operation, 'call>> {
    Box::new(match failure {
        ResidentConsultationTaskLossCommitFailure::Unavailable { prepared, reason } => {
            ResidentProcessConsultProviderColdLossFailure::Unavailable(Box::new(
                ResidentProcessConsultProviderColdLossUnavailable {
                    recording,
                    prepared,
                    reason,
                },
            ))
        }
        ResidentConsultationTaskLossCommitFailure::Prepared { prepared, reason } => {
            ResidentProcessConsultProviderColdLossFailure::Prepared(Box::new(
                ResidentProcessConsultProviderColdLossPrepared {
                    recording,
                    prepared,
                    reason,
                },
            ))
        }
        ResidentConsultationTaskLossCommitFailure::Offered { pending, reason } => {
            ResidentProcessConsultProviderColdLossFailure::Offered(Box::new(
                ResidentProcessConsultProviderColdLossOffered {
                    recording,
                    pending,
                    reason,
                },
            ))
        }
    })
}

/// Inputs that poll one historical dispatch without a provider call.
pub(super) struct ResidentProcessConsultProviderPollTurn<'input, 'operation, Executor> {
    pub(super) recovered: &'operation ResidentProcessRecoveredAttempt,
    pub(super) task_limits: ResidentConsultationTaskLimits,
    pub(super) observed_at: UnixTimeV1,
    pub(super) current: RecoveredConsultationCollection,
    pub(super) executor: &'input mut Executor,
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/consult_provider_support.rs"]
pub(in crate::resident) mod test_support;

type ResidentProcessConsultProviderOpenResult<'input, 'operation, 'call, Authority, Executor> =
    Result<
        Result<
            ResidentConsultationTurnDisposition<
                ResidentProcessConsultProviderChildWork<'operation, 'call>,
                ResidentProcessConsultProviderChildOutput,
                std::convert::Infallible,
                ResidentProcessOperationRestartError,
                ResidentProcessConsultProviderChildRetentionFailure<'operation>,
                (),
            >,
            ResidentProcessOperationRestartError,
        >,
        ResidentProcessConsultProviderPhaseRefusal<
            ResidentProcessConsultProviderOpenTurn<'input, 'operation, 'call, Authority, Executor>,
        >,
    >;

type ResidentProcessConsultProviderResumeResult<'input, 'operation, 'call, Authority, Executor> =
    Result<
        Result<
            crate::resident::task_owner::ResidentOperationTaskKey,
            ResidentProcessConsultProviderResumeFailure<'operation, 'call>,
        >,
        ResidentProcessConsultProviderPhaseRefusal<
            ResidentProcessConsultProviderResumeTurn<
                'input,
                'operation,
                'call,
                Authority,
                Executor,
            >,
        >,
    >;

type ResidentProcessConsultProviderPollResult<'input, 'operation, 'call, Executor> = Result<
    ResidentConsultationTurnDisposition<
        ResidentProcessConsultProviderChildWork<'operation, 'call>,
        ResidentProcessConsultProviderChildOutput,
        std::convert::Infallible,
        std::convert::Infallible,
        std::convert::Infallible,
        (),
    >,
    ResidentProcessConsultProviderPhaseRefusal<
        ResidentProcessConsultProviderPollTurn<'input, 'operation, Executor>,
    >,
>;

/// Inputs that record and classify one task-owned provider completion.
pub(super) struct ResidentProcessConsultProviderFinalizeTurn<'input, 'operation, 'call> {
    pub(super) access: ResidentDeploymentOperationAccess<'input>,
    pub(super) ready: ResidentConsultationCompletionReady<
        ResidentProcessConsultProviderChildWork<'operation, 'call>,
        ResidentProcessConsultProviderChildOutput,
    >,
    pub(super) history: OperationAttemptHistoryInput<'operation>,
    pub(super) recovered_dispatch: &'operation RecoveredOperationObservation,
    pub(super) lost_input: &'input ProviderOperationInvokeInput<'call>,
    pub(super) evidence: &'input dyn ProviderInvocationEvidenceAdmission,
    pub(super) recovery: ResidentProcessConsultProviderRecordedRecovery<'input, 'operation>,
    pub(super) recording: ResidentProcessConsultProviderRecordingAttempt<'input>,
}

/// Parent fields for one conclusive child join.
pub(super) struct ResidentProcessConsultProviderParentJoinInput<'operation> {
    pub(super) slot_id: &'operation ConsultationSlotId,
    pub(super) recorded_at: UnixTimeV1,
    pub(super) parent: &'operation IssuedLocalOperation,
    pub(super) parent_observation: (
        &'operation ResidentRecordRefV1,
        &'operation OperationObservationV1,
    ),
    pub(super) observation_ref: ResidentRecordRefV1,
    pub(super) observed_at: UnixTimeV1,
    pub(super) signing: SignedResidentAdmissionContextV1<'operation>,
    pub(super) limits: ConsultationCollectionLimits,
}

/// Inputs that join one conclusive provider child into its parent.
pub(super) struct ResidentProcessConsultProviderJoinTurn<'input, 'operation, 'call, Authority> {
    pub(super) association: ResidentOperationControlAssociation<'input>,
    pub(super) access: ResidentDeploymentOperationAccess<'input>,
    pub(super) authority: &'input mut Authority,
    pub(super) completion: ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
    pub(super) input: ResidentProcessConsultProviderParentJoinInput<'operation>,
    pub(super) checkpoint: ConsultationSuccessorCommitInput<'input>,
    pub(super) resolve_store: &'input dyn super::super::OperationObservationAuthorityStoreResolver,
    pub(super) admit_evidence: ResidentProcessConsultProviderEvidenceAdmission<'input>,
    pub(super) admit_collection: ResidentProcessConsultProviderCollectionAdmission<'input>,
    pub(super) rotate: ResidentProcessConsultProviderRotation<'input>,
    pub(super) sign_observation: ResidentProcessConsultProviderObservationSigner<'input>,
    pub(super) sign_checkpoint: ResidentProcessConsultProviderCheckpointSigner<'input>,
}

/// Inputs retained after one provider-backed Consult child poll.
///
/// The resident owns these inputs until a ready child is recorded and either
/// joins its exact roster slot or settles as nonterminal. A poll with no
/// completion returns this value unchanged.
#[must_use = "retain the exact child completion inputs for the next bounded turn"]
pub(super) struct ResidentProcessConsultProviderPostPoll<'input, 'operation, 'call, Authority> {
    pub(super) access: ResidentDeploymentOperationAccess<'input>,
    pub(super) history: OperationAttemptHistoryInput<'operation>,
    pub(super) recovered_dispatch: &'operation RecoveredOperationObservation,
    pub(super) lost_input: &'input ProviderOperationInvokeInput<'call>,
    pub(super) evidence: &'input dyn ProviderInvocationEvidenceAdmission,
    pub(super) recovery: ResidentProcessConsultProviderRecordedRecovery<'input, 'operation>,
    pub(super) recording: ResidentProcessConsultProviderRecordingAttempt<'input>,
    pub(super) parent: ResidentProcessConsultProviderParentTurn<'input, 'operation, Authority>,
}

/// Parent join inputs held until conclusive child evidence reaches custody.
#[must_use = "join only a conclusive recorded child through these exact parent inputs"]
pub(super) struct ResidentProcessConsultProviderParentTurn<'input, 'operation, Authority> {
    pub(super) association: ResidentOperationControlAssociation<'input>,
    pub(super) access: ResidentDeploymentOperationAccess<'input>,
    pub(super) authority: &'input mut Authority,
    pub(super) slot_id: &'operation ConsultationSlotId,
    pub(super) parent: &'operation IssuedLocalOperation,
    pub(super) parent_observation: (
        &'operation ResidentRecordRefV1,
        &'operation OperationObservationV1,
    ),
    pub(super) observation_ref: ResidentRecordRefV1,
    pub(super) signing: SignedResidentAdmissionContextV1<'operation>,
    pub(super) limits: ConsultationCollectionLimits,
    pub(super) checkpoint_accumulator_limits: AccumulatorPreparationLimits,
    pub(super) checkpoint_identity: OperationCheckpointInput,
    pub(super) checkpoint_schema: &'input SchemaDigest,
    pub(super) checkpoint_authority:
        &'input ControlCheckpointSigningAuthorityAdmissionContextV1<'input>,
    pub(super) observe_join_time: ResidentProcessConsultProviderJoinTime<'input>,
    pub(super) resolve_store: &'input dyn super::super::OperationObservationAuthorityStoreResolver,
    pub(super) admit_evidence: ResidentProcessConsultProviderEvidenceAdmission<'input>,
    pub(super) admit_collection: ResidentProcessConsultProviderCollectionAdmission<'input>,
    pub(super) rotate: ResidentProcessConsultProviderRotation<'input>,
    pub(super) sign_observation: ResidentProcessConsultProviderObservationSigner<'input>,
    pub(super) sign_checkpoint: ResidentProcessConsultProviderCheckpointSigner<'input>,
}

/// One bounded continuation of an already-open provider-backed Consult child.
#[must_use = "run or retain this exact child continuation"]
pub(super) struct ResidentProcessConsultProviderChildTurn<
    'poll,
    'input,
    'operation,
    'call,
    Authority,
    Executor,
> {
    pub(super) poll: ResidentProcessConsultProviderPollTurn<'poll, 'operation, Executor>,
    pub(super) post_poll:
        ResidentProcessConsultProviderPostPoll<'input, 'operation, 'call, Authority>,
}

/// Completed disposition of one bounded provider-backed Consult child turn.
#[must_use = "retain the exact pending, stopped, joined, or nonterminal child outcome"]
pub(super) enum ResidentProcessConsultProviderChildDisposition<'input, 'operation, 'call, Authority>
{
    /// No ready completion existed during this turn.
    Pending {
        reason: Box<ResidentConsultationTurnPending>,
        continuation:
            Box<ResidentProcessConsultProviderPostPoll<'input, 'operation, 'call, Authority>>,
    },
    /// The recovered parent collection was already stopped.
    Stopped {
        reason: ResidentConsultationTurnStopped<()>,
        continuation:
            Box<ResidentProcessConsultProviderPostPoll<'input, 'operation, 'call, Authority>>,
    },
    /// Conclusive child evidence joined its exact roster slot.
    Joined(
        Box<
            CompletedResidentConsultationTerminal<
                ResidentProcessConsultProviderChildWork<'operation, 'call>,
                ResidentProcessConsultProviderChildOutput,
            >,
        >,
    ),
    /// Durable unknown or other nonterminal evidence released local capacity.
    Nonterminal(
        Box<
            CompletedResidentConsultationNonterminal<
                ResidentProcessConsultProviderChildWork<'operation, 'call>,
                ResidentProcessConsultProviderChildOutput,
            >,
        >,
    ),
}

/// Exact continuation after a bounded child turn stopped.
#[must_use = "continue only the returned exact phase; never replay the provider call"]
pub(super) enum ResidentProcessConsultProviderChildTurnFailure<
    'poll,
    'input,
    'operation,
    'call,
    Authority,
    Executor,
> {
    /// Deployment or child association failed before polling.
    Selection {
        turn: Box<
            ResidentProcessConsultProviderChildTurn<
                'poll,
                'input,
                'operation,
                'call,
                Authority,
                Executor,
            >,
        >,
        reason: ResidentProcessHostSelectionError,
    },
    /// Polling found an exact local recovery phase instead of a completion.
    Poll {
        recovery: Box<
            ResidentConsultationTurnNeedsRecovery<
                ResidentProcessConsultProviderChildWork<'operation, 'call>,
                ResidentProcessConsultProviderChildOutput,
                std::convert::Infallible,
                std::convert::Infallible,
                std::convert::Infallible,
            >,
        >,
        continuation:
            Box<ResidentProcessConsultProviderPostPoll<'input, 'operation, 'call, Authority>>,
    },
    /// Recording, classification, or nonterminal settlement retained its exact phase.
    Completion {
        failure: Box<ResidentProcessConsultProviderFinalizeFailure<'input, 'operation, 'call>>,
        parent: Box<ResidentProcessConsultProviderParentTurn<'input, 'operation, Authority>>,
    },
    /// Conclusive evidence was recorded, but the exact parent join did not finish.
    Join(
        Box<
            ResidentProcessConsultationTerminalJoinFailure<
                ResidentProcessConsultProviderChildWork<'operation, 'call>,
                ResidentProcessConsultProviderChildOutput,
            >,
        >,
    ),
}

/// One selected provider phase refused before its owner ran.
pub(super) struct ResidentProcessConsultProviderPhaseRefusal<Input> {
    input: Box<Input>,
    reason: ResidentProcessHostSelectionError,
}

impl<Input> ResidentProcessConsultProviderPhaseRefusal<Input> {
    pub(super) fn into_parts(self) -> (Input, ResidentProcessHostSelectionError) {
        (*self.input, self.reason)
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_consult_provider_resume_associations(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    association: &ResidentOperationControlAssociation<'_>,
    access: &ResidentDeploymentOperationAccess<'_>,
    current: &RecoveredConsultationCollection,
    planned: &ConsultationChildDispatchBatch,
    candidate: &ConsultationChildDispatchCandidate,
    parent: &ResidentProcessRecoveredOperationStart,
    phase: &ResidentProcessConsultProviderResumePhase<'_>,
    release: &RecoveredProviderDispatchRelease,
    invocation: &ProviderOperationInvokeInput<'_>,
    provider_admission: &crate::provider_invocation::ProviderInvocationAdmissionContext<'_>,
    observed_at: UnixTimeV1,
) -> Result<(), ResidentProcessHostSelectionError> {
    let source = phase.start();
    let Some(lineage) = source.lineage.consult_child() else {
        return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation);
    };
    let released_operation = release.operation();
    let (child_ref, child_handle) = released_operation.handle();
    let (parent_ref, parent_handle) = parent.control.operation().operation().handle();
    let child = ImmutableRecordReferenceV1 {
        record_ref: child_ref.clone(),
        record_sha256: ArtifactDigest::parse(
            child_handle.canonical_operation_handle_sha256().as_str(),
        )
        .map_err(|_| ResidentProcessHostSelectionError::ConsultationChildAssociation)?,
    };
    let parent_coordinate = ImmutableRecordReferenceV1 {
        record_ref: parent_ref.clone(),
        record_sha256: ArtifactDigest::parse(
            parent_handle.canonical_operation_handle_sha256().as_str(),
        )
        .map_err(|_| ResidentProcessHostSelectionError::ConsultationParentAuthority)?,
    };
    let source_observation = source.control.observation();
    let parent_observation = parent.control.observation();
    let (dispatch_ref, dispatch) = release.observation();
    let (preparation_ref, preparation) = release.preparation();
    let (release_checkpoint_ref, release_checkpoint) = release.checkpoint();
    let source_selected = source_observation.selected();
    let parent_head = parent_observation.selected_head().1;

    validate_recovered_provider_call_references(release, invocation)
        .map_err(|_| ResidentProcessHostSelectionError::ProviderOperationAuthority)?;

    if source.selector != *selector
        || parent.selector != *selector
        || access.store != store
        || association.recovery_store != store
        || release.store() != store
        || source.control.operation().store() != store
        || source_observation.store() != store
        || parent.control.operation().store() != store
        || parent_observation.store() != store
        || current.store() != store
        || planned.store() != store
    {
        return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
    }
    if !selector.matches_signing_authority(association.current_request_authority) {
        return Err(ResidentProcessHostSelectionError::CurrentRequestAuthority);
    }
    if !selector.matches_signing_authority(association.historical_handle_authority) {
        return Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority);
    }
    if !checkpoint_authority_matches(selector, store, association.checkpoint_authority)
        || !checkpoint_authority_matches(selector, store, invocation.checkpoint_authority)
        || !same_checkpoint_authority(
            association.checkpoint_authority,
            invocation.checkpoint_authority,
        )
    {
        return Err(ResidentProcessHostSelectionError::ProviderCheckpointAuthority);
    }
    if !selector.matches_current_session(invocation.current)
        || !selector
            .matches_signing_authority(invocation.current.generation.signed.signing_authority)
        || invocation
            .current
            .generation
            .compiled_generation
            .compiled_generation_digest()
            != &source.generation_digest
        || invocation
            .current
            .generation
            .active_head
            .compiled_generation_digest()
            != &source.generation_digest
        || invocation
            .current
            .generation
            .activation_fence
            .activation_fence_digest()
            != child_handle.activation_fence_sha256()
    {
        return Err(ResidentProcessHostSelectionError::ProviderCurrentAuthority);
    }
    if !selector.matches_signing_authority(invocation.signing.signing_authority)
        || !selector.matches_signing_authority(
            &source
                .control
                .operation()
                .request()
                .to_preimage()
                .signing_authority_binding,
        )
    {
        return Err(ResidentProcessHostSelectionError::ProviderObservationAuthority);
    }
    if !selector.matches_operation(released_operation)
        || !selector.matches_operation(parent.control.operation().operation())
        || !source.lineage.admits(child_ref, child_handle)
        || source.control.operation().operation().handle() != released_operation.handle()
        || source.generation_digest != *child_handle.compiled_generation_sha256()
        || child_handle.issuer_key_id().as_str()
            != association.historical_handle_authority.signer_key_id()
        || child_handle.issuer_key_epoch()
            != association.historical_handle_authority.signer_key_epoch()
        || parent_handle.issuer_key_id().as_str()
            != association.historical_handle_authority.signer_key_id()
        || parent_handle.issuer_key_epoch()
            != association.historical_handle_authority.signer_key_epoch()
        || parent_handle.operation_class() != &OperationClassV1::Consult
        || child_handle.operation_class() != &OperationClassV1::Invoke
        || !operation_observation_matches_deployment(selector, source_selected.1)
        || !operation_observation_matches_deployment(selector, parent_head)
    {
        return Err(ResidentProcessHostSelectionError::ConsultationOperationAuthority);
    }
    if planned.parent() != &parent_coordinate
        || current.collection().parent() != &parent_coordinate
        || lineage.parent() != &parent_coordinate
        || lineage.collection_frontier() != planned.collection_frontier()
        || lineage.slot_id() != candidate.slot_id()
        || lineage.child() != &child
        || candidate.child() != &child
        || lineage.participant() != candidate.participant()
        || lineage.provider_binding() != candidate.provider_binding()
        || lineage.context() != candidate.context()
    {
        return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation);
    }
    let parent_frontier = match parent_head.state() {
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::CollectingConsultation { evidence }
                | OperationActivePhaseV1::ConsultationCollected { evidence },
        } => evidence,
        _ => return Err(ResidentProcessHostSelectionError::ConsultationParentAuthority),
    };
    if parent_frontier != planned.collection_frontier()
        || current
            .revalidate_child_dispatch(planned, observed_at)
            .ok()
            .and_then(|batch| {
                batch
                    .candidates()
                    .iter()
                    .find(|current| *current == candidate)
                    .cloned()
            })
            .as_ref()
            != Some(candidate)
    {
        return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation);
    }

    let request = release
        .provider_request()
        .map_err(|_| ResidentProcessHostSelectionError::ProviderOperationAuthority)?;
    let Some(provider_binding) = candidate.provider_binding() else {
        return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation);
    };
    if request.compiled_generation_digest() != &source.generation_digest
        || request.provider_binding().id != provider_binding.provider_binding_id
        || request.provider_binding().binding_identity != provider_binding.provider_binding_identity
        || request.provider_binding().participant != *candidate.participant()
        || !release.admits_recording_context(provider_admission)
    {
        return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
    }
    let OperationObservationStateV1::Active {
        phase:
            OperationActivePhaseV1::DispatchPrepared {
                attempt_id: prepared_attempt,
                evidence: prepared_evidence,
            },
    } = preparation.state()
    else {
        return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
    };
    let OperationObservationStateV1::Active {
        phase:
            OperationActivePhaseV1::Dispatched {
                attempt_id: dispatched_attempt,
                evidence: dispatched_evidence,
            },
    } = dispatch.state()
    else {
        return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
    };
    if prepared_attempt != request.attempt_id()
        || dispatched_attempt != request.attempt_id()
        || prepared_evidence != dispatched_evidence
        || prepared_evidence.record_ref != *release.selection_ref()
        || dispatch_ref == preparation_ref
        || dispatch_ref == release.selection_ref()
        || dispatch.operation_handle_ref() != child_ref
        || dispatch.operation_handle_sha256() != child_handle.canonical_operation_handle_sha256()
        || dispatch.predecessor_ref().as_ref() != Some(preparation_ref)
        || dispatch.predecessor_sha256().as_ref()
            != Some(preparation.canonical_operation_observation_sha256())
        || !operation_observation_matches_deployment(selector, preparation)
        || !operation_observation_matches_deployment(selector, dispatch)
    {
        return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
    }
    match phase {
        ResidentProcessConsultProviderResumePhase::DispatchPrepared(recovered) => {
            if source_selected != (preparation_ref, preparation)
                || recovered.selection.coordinate().record_ref() != release.selection_ref()
                || recovered.selection.coordinate().record_ref() != &prepared_evidence.record_ref
                || recovered.selection.coordinate().record_sha256().as_str()
                    != prepared_evidence.record_sha256.as_str()
                || (release_checkpoint_ref, release_checkpoint)
                    != source_observation.current_checkpoint()
            {
                return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
            }
        }
        ResidentProcessConsultProviderResumePhase::ProviderResolved(recovered) => {
            let OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ProviderResolved { provider_binding },
            } = source_selected.1.state()
            else {
                return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
            };
            if provider_binding != &recovered.provider_binding
                || candidate.provider_binding() != Some(&recovered.provider_binding)
                || preparation.predecessor_ref().as_ref() != Some(source_selected.0)
                || preparation.predecessor_sha256().as_ref()
                    != Some(source_selected.1.canonical_operation_observation_sha256())
            {
                return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
            }
        }
    }

    let historical_session = released_operation.historical_session_lease();
    let historical_lease = released_operation.historical_operation_lease();
    if historical_lease.operation_lease_digest() != child_handle.operation_lease_sha256()
        || historical_session.compiled_generation_digest()
            != child_handle.compiled_generation_sha256()
        || historical_session.activation_fence_digest() != child_handle.activation_fence_sha256()
        || historical_session.instance_digest() != child_handle.instance_sha256()
        || historical_session.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
        || historical_session
            .deployment_runtime_authority_sha256()
            .as_str()
            != selector.runtime_authority_sha256.as_str()
        || historical_session.issuer_principal_id() != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::HistoricalProviderAuthority);
    }
    if release_checkpoint.control_store_id() != store.store_id()
        || release_checkpoint.control_incarnation_id() != store.incarnation_id()
        || release_checkpoint.issuer_principal_id() != &selector.principal_id
        || release_checkpoint.checkpoint_signing_authority_ref()
            != association.checkpoint_authority.authority_ref
        || release_checkpoint.checkpoint_signing_authority_sha256()
            != association.checkpoint_authority.authority_sha256
        || release_checkpoint.checkpoint_domain_id()
            != association.checkpoint_authority.checkpoint_domain_id
        || release_checkpoint.checkpoint_profile_ref()
            != association.checkpoint_authority.checkpoint_profile_ref
        || release_checkpoint.checkpoint_profile_sha256()
            != association.checkpoint_authority.checkpoint_profile_sha256
        || release_checkpoint.issuer_key_id() != association.checkpoint_authority.subject_key_id
        || release_checkpoint.issuer_key_epoch()
            != &association.checkpoint_authority.subject_key_epoch
    {
        return Err(ResidentProcessHostSelectionError::ProviderCheckpointAuthority);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_consult_provider_cold_loss_associations(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    generation: &crate::resident::AdmittedCompiledGeneration,
    turn: &ResidentProcessConsultProviderColdLossTurn<'_, '_, '_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let attempt = turn.attempt;
    let source = &attempt.start;
    let Some(lineage) = source.lineage.consult_child() else {
        return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation);
    };
    let association = &turn.recovery.association;
    let parent = turn.recovery.parent;
    let planned = turn.recovery.planned;
    let observed_at = turn.recovery.observed_at;
    let operation = source.control.operation().operation();
    let recovered = source.control.observation();
    let (child_ref, child_handle) = operation.handle();
    let (parent_ref, parent_handle) = parent.control.operation().operation().handle();
    let child = ImmutableRecordReferenceV1 {
        record_ref: child_ref.clone(),
        record_sha256: ArtifactDigest::parse(
            child_handle.canonical_operation_handle_sha256().as_str(),
        )
        .map_err(|_| ResidentProcessHostSelectionError::ConsultationChildAssociation)?,
    };
    let parent_coordinate = ImmutableRecordReferenceV1 {
        record_ref: parent_ref.clone(),
        record_sha256: ArtifactDigest::parse(
            parent_handle.canonical_operation_handle_sha256().as_str(),
        )
        .map_err(|_| ResidentProcessHostSelectionError::ConsultationParentAuthority)?,
    };

    if source.selector != *selector
        || parent.selector != *selector
        || turn.access.store != store
        || association.recovery_store != store
        || source.control.operation().store() != store
        || recovered.store() != store
        || parent.control.operation().store() != store
        || parent.control.observation().store() != store
        || turn.current.store() != store
        || planned.store() != store
        || turn.history.operation.handle() != operation.handle()
    {
        return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
    }
    if !selector.matches_signing_authority(association.current_request_authority) {
        return Err(ResidentProcessHostSelectionError::CurrentRequestAuthority);
    }
    if !selector.matches_signing_authority(association.historical_handle_authority) {
        return Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority);
    }
    if !checkpoint_authority_matches(selector, store, association.checkpoint_authority)
        || !same_checkpoint_authority(
            association.checkpoint_authority,
            turn.input.checkpoint_authority,
        )
    {
        return Err(ResidentProcessHostSelectionError::ProviderCheckpointAuthority);
    }
    if !selector.matches_current_session(turn.input.current)
        || !selector
            .matches_signing_authority(turn.input.current.generation.signed.signing_authority)
        || turn
            .input
            .current
            .generation
            .compiled_generation
            .compiled_generation_digest()
            != &source.generation_digest
        || turn
            .input
            .current
            .generation
            .active_head
            .compiled_generation_digest()
            != &source.generation_digest
        || turn
            .input
            .current
            .generation
            .activation_fence
            .activation_fence_digest()
            != child_handle.activation_fence_sha256()
        || turn
            .history
            .generation
            .record()
            .compiled_generation_digest()
            != &source.generation_digest
    {
        return Err(ResidentProcessHostSelectionError::ProviderCurrentAuthority);
    }
    if !selector.matches_operation(operation)
        || !selector.matches_operation(parent.control.operation().operation())
        || !source.lineage.admits(child_ref, child_handle)
        || source.generation_digest != *child_handle.compiled_generation_sha256()
        || child_handle.issuer_key_id().as_str()
            != association.historical_handle_authority.signer_key_id()
        || child_handle.issuer_key_epoch()
            != association.historical_handle_authority.signer_key_epoch()
        || parent_handle.issuer_key_id().as_str()
            != association.historical_handle_authority.signer_key_id()
        || parent_handle.issuer_key_epoch()
            != association.historical_handle_authority.signer_key_epoch()
        || parent_handle.operation_class() != &OperationClassV1::Consult
        || child_handle.operation_class() != &OperationClassV1::Invoke
        || !operation_observation_matches_deployment(selector, recovered.selected().1)
        || !operation_observation_matches_deployment(
            selector,
            parent.control.observation().selected_head().1,
        )
    {
        return Err(ResidentProcessHostSelectionError::ConsultationOperationAuthority);
    }
    if planned.parent() != &parent_coordinate
        || turn.current.collection().parent() != &parent_coordinate
        || lineage.parent() != &parent_coordinate
        || lineage.collection_frontier() != planned.collection_frontier()
        || lineage.slot_id() != turn.candidate.slot_id()
        || lineage.child() != &child
        || turn.candidate.child() != &child
        || lineage.participant() != turn.candidate.participant()
        || lineage.provider_binding() != turn.candidate.provider_binding()
        || lineage.context() != turn.candidate.context()
    {
        return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation);
    }
    let historical_session = operation.historical_session_lease();
    let historical_lease = operation.historical_operation_lease();
    if historical_lease.operation_lease_digest() != child_handle.operation_lease_sha256()
        || historical_session.compiled_generation_digest()
            != child_handle.compiled_generation_sha256()
        || historical_session.activation_fence_digest() != child_handle.activation_fence_sha256()
        || historical_session.instance_digest() != child_handle.instance_sha256()
        || historical_session.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
        || historical_session
            .deployment_runtime_authority_sha256()
            .as_str()
            != selector.runtime_authority_sha256.as_str()
        || historical_session.issuer_principal_id() != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::HistoricalProviderAuthority);
    }
    let parent_head = parent.control.observation().selected_head().1;
    let parent_frontier = match parent_head.state() {
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::CollectingConsultation { evidence }
                | OperationActivePhaseV1::ConsultationCollected { evidence },
        } => evidence,
        _ => return Err(ResidentProcessHostSelectionError::ConsultationParentAuthority),
    };
    if parent_frontier != planned.collection_frontier()
        || turn
            .current
            .revalidate_child_dispatch(planned, observed_at)
            .ok()
            .and_then(|batch| {
                batch
                    .candidates()
                    .iter()
                    .find(|current| *current == turn.candidate)
                    .cloned()
            })
            .as_ref()
            != Some(turn.candidate)
    {
        return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation);
    }
    validate_provider_recovery_associations(
        selector,
        store,
        generation,
        attempt,
        turn.input,
        turn.history.maximum_selection_bytes,
    )?;
    validate_recovered_provider_unknown(&turn.history, recovered, turn.input)
        .map_err(|_| ResidentProcessHostSelectionError::ProviderOperationAuthority)
}

/// Refuse a mixed child continuation before it can poll the executor.
fn validate_consult_provider_child_turn_associations<Authority, Executor>(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    turn: &ResidentProcessConsultProviderChildTurn<'_, '_, '_, '_, Authority, Executor>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let source = &turn.poll.recovered.start;
    let Some(lineage) = source.lineage.consult_child() else {
        return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation);
    };
    let child_operation = source.control.operation().operation();
    let child_observation = source.control.observation();
    let parent_start = turn.post_poll.recovery.parent;
    let parent_operation = parent_start.control.operation().operation();
    let parent_observation = parent_start.control.observation();
    let planned = turn.post_poll.recovery.planned;
    let (child_ref, child_handle) = child_operation.handle();
    let (parent_ref, parent_handle) = parent_operation.handle();

    if source.selector != *selector
        || parent_start.selector != *selector
        || source.control.operation().store() != store
        || child_observation.store() != store
        || parent_start.control.operation().store() != store
        || parent_observation.store() != store
        || turn.poll.current.store() != store
        || turn.post_poll.access.store != store
        || turn.post_poll.recovered_dispatch.store() != store
        || turn.post_poll.recovery.association.recovery_store != store
        || planned.store() != store
        || turn.post_poll.parent.access.store != store
        || turn.post_poll.parent.association.recovery_store != store
    {
        return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
    }
    let history = &turn.post_poll.history;
    let (history_child_ref, history_child_handle) = history.operation.handle();
    let selected_child = child_observation.selected();
    let retained_dispatch = turn.post_poll.recovered_dispatch.selected();
    if !selector.matches_operation(child_operation)
        || !selector.matches_operation(parent_operation)
        || history_child_ref != child_ref
        || history_child_handle.canonical_operation_handle_sha256()
            != child_handle.canonical_operation_handle_sha256()
        || history.dispatched.canonical_operation_observation_sha256()
            != selected_child.1.canonical_operation_observation_sha256()
        || history.selection.coordinate() != turn.poll.recovered.selection.coordinate()
        || history.generation.record().compiled_generation_digest() != &source.generation_digest
        || retained_dispatch.0 != selected_child.0
        || retained_dispatch.1.canonical_operation_observation_sha256()
            != selected_child.1.canonical_operation_observation_sha256()
        || child_ref != &lineage.child().record_ref
        || child_handle.canonical_operation_handle_sha256().as_str()
            != lineage.child().record_sha256.as_str()
        || parent_ref != &lineage.parent().record_ref
        || parent_handle.canonical_operation_handle_sha256().as_str()
            != lineage.parent().record_sha256.as_str()
        || child_handle.operation_class() != &OperationClassV1::Invoke
        || parent_handle.operation_class() != &OperationClassV1::Consult
        || !source.lineage.admits(child_ref, child_handle)
    {
        return Err(ResidentProcessHostSelectionError::ConsultationOperationAuthority);
    }

    let Some(candidate) = planned
        .candidates()
        .iter()
        .find(|candidate| candidate.slot_id() == lineage.slot_id())
    else {
        return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation);
    };
    let planned_parent = planned.parent();
    let current_parent = turn.poll.current.collection().parent();
    if planned_parent.record_ref != *parent_ref
        || planned_parent.record_sha256.as_str()
            != parent_handle.canonical_operation_handle_sha256().as_str()
        || current_parent.record_ref != *parent_ref
        || current_parent.record_sha256.as_str()
            != parent_handle.canonical_operation_handle_sha256().as_str()
        || planned.collection_frontier() != lineage.collection_frontier()
        || candidate.child() != lineage.child()
        || candidate.participant() != lineage.participant()
        || candidate.provider_binding() != lineage.provider_binding()
        || candidate.context() != lineage.context()
        || turn.post_poll.parent.slot_id != lineage.slot_id()
        || turn.post_poll.parent.parent.handle() != parent_operation.handle()
        || turn.post_poll.parent.parent_observation != parent_observation.selected()
    {
        return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation);
    }
    Ok(())
}

impl<WriterGuard> ResidentProcessHost<'_, WriterGuard> {
    /// Join conclusive provider evidence or return an already settled uncertainty.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn complete_consultation_provider_parent<'input, 'operation, 'call, Authority>(
        &mut self,
        association: ResidentOperationControlAssociation<'input>,
        access: ResidentDeploymentOperationAccess<'input>,
        authority: &mut Authority,
        completion: ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
        input: ResidentProcessConsultProviderParentJoinInput<'operation>,
        checkpoint: ConsultationSuccessorCommitInput<'input>,
        resolve_store: &'input dyn super::super::OperationObservationAuthorityStoreResolver,
        admit_evidence: ResidentProcessConsultProviderEvidenceAdmission<'input>,
        admit_collection: ResidentProcessConsultProviderCollectionAdmission<'input>,
        rotate: ResidentProcessConsultProviderRotation<'input>,
        sign_observation: ResidentProcessConsultProviderObservationSigner<'input>,
        sign_checkpoint: ResidentProcessConsultProviderCheckpointSigner<'input>,
    ) -> Result<
        ResidentProcessConsultProviderParentDisposition<'operation, 'call>,
        Box<
            ResidentProcessConsultationTerminalJoinFailure<
                ResidentProcessConsultProviderChildWork<'operation, 'call>,
                ResidentProcessConsultProviderChildOutput,
            >,
        >,
    >
    where
        Authority: super::super::AmbientSessionAuthorityPort,
    {
        let (ready, child) = match completion {
            ResidentProcessConsultProviderCompletionDisposition::Terminal { ready, child } => {
                (ready, child)
            }
            ResidentProcessConsultProviderCompletionDisposition::Nonterminal(settled) => {
                return Ok(ResidentProcessConsultProviderParentDisposition::Nonterminal(settled));
            }
        };
        let child_observation = child.control.observation();
        let selector = ready.selector.clone();
        let mut driver = match self.consultation_child_driver(&selector) {
            Ok(driver) => driver,
            Err(reason) => {
                return Err(Box::new(
                    ResidentProcessConsultationTerminalJoinFailure::Selection { ready, reason },
                ));
            }
        };
        driver
            .join_terminal(
                association,
                access,
                authority,
                *ready,
                ConsultationChildJoinInput {
                    slot_id: input.slot_id,
                    child: child_observation,
                    recorded_at: input.recorded_at,
                    parent: input.parent,
                    parent_observation: input.parent_observation,
                    observation_ref: input.observation_ref,
                    observed_at: input.observed_at,
                    signing: input.signing,
                    limits: input.limits,
                },
                checkpoint,
                resolve_store,
                admit_evidence,
                admit_collection,
                rotate,
                sign_observation,
                sign_checkpoint,
            )
            .map(Box::new)
            .map(ResidentProcessConsultProviderParentDisposition::Joined)
            .map_err(Box::new)
    }

    /// Poll, record, classify, and consume one already-open provider Consult child.
    ///
    /// This is one nonblocking resident turn. It polls the task owner once and
    /// never invokes a provider. Empty polls do not alter durable child or
    /// parent state. Conclusive evidence is released only after the exact
    /// parent roster slot reaches custody; durable uncertainty settles only
    /// the local reservation.
    pub(super) fn continue_consultation_provider_child<
        'poll,
        'input,
        'operation,
        'call,
        Authority,
        Executor,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        turn: ResidentProcessConsultProviderChildTurn<
            'poll,
            'input,
            'operation,
            'call,
            Authority,
            Executor,
        >,
    ) -> Result<
        ResidentProcessConsultProviderChildDisposition<'input, 'operation, 'call, Authority>,
        Box<
            ResidentProcessConsultProviderChildTurnFailure<
                'poll,
                'input,
                'operation,
                'call,
                Authority,
                Executor,
            >,
        >,
    >
    where
        'operation: 'call,
        Authority: super::super::AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
    {
        let selected_store = match self.deployment_authority_index(selector) {
            Ok(index) => self.deployments[index].store(),
            Err(reason) => {
                return Err(Box::new(
                    ResidentProcessConsultProviderChildTurnFailure::Selection {
                        turn: Box::new(turn),
                        reason,
                    },
                ));
            }
        };
        if let Err(reason) =
            validate_consult_provider_child_turn_associations(selector, selected_store, &turn)
        {
            return Err(Box::new(
                ResidentProcessConsultProviderChildTurnFailure::Selection {
                    turn: Box::new(turn),
                    reason,
                },
            ));
        }
        let ResidentProcessConsultProviderChildTurn { poll, post_poll } = turn;
        let polled = match self.poll_consultation_provider_child(selector, poll) {
            Ok(polled) => polled,
            Err(refusal) => {
                let (poll, reason) = refusal.into_parts();
                return Err(Box::new(
                    ResidentProcessConsultProviderChildTurnFailure::Selection {
                        turn: Box::new(ResidentProcessConsultProviderChildTurn { poll, post_poll }),
                        reason,
                    },
                ));
            }
        };
        let ready = match polled {
            ResidentConsultationTurnDisposition::Progress(
                ResidentConsultationTurnProgress::CompletionReady(ready),
            ) => *ready,
            ResidentConsultationTurnDisposition::Pending(reason) => {
                return Ok(ResidentProcessConsultProviderChildDisposition::Pending {
                    reason,
                    continuation: Box::new(post_poll),
                });
            }
            ResidentConsultationTurnDisposition::Stopped(reason) => {
                return Ok(ResidentProcessConsultProviderChildDisposition::Stopped {
                    reason,
                    continuation: Box::new(post_poll),
                });
            }
            ResidentConsultationTurnDisposition::NeedsRecovery(recovery) => {
                return Err(Box::new(
                    ResidentProcessConsultProviderChildTurnFailure::Poll {
                        recovery,
                        continuation: Box::new(post_poll),
                    },
                ));
            }
            ResidentConsultationTurnDisposition::Progress(
                ResidentConsultationTurnProgress::Offered { .. },
            ) => unreachable!("a completion-only provider turn never offers another child"),
        };

        let ResidentProcessConsultProviderPostPoll {
            mut access,
            history,
            recovered_dispatch,
            lost_input,
            evidence,
            mut recovery,
            recording,
            parent,
        } = post_poll;
        let ready = match self.bind_consultation_completion(selector.clone(), ready) {
            Ok(ready) => ready,
            Err(failure) => {
                let ResidentProcessConsultationCompletionBindingFailure {
                    selector,
                    ready,
                    reason,
                } = *failure;
                let failure = ResidentProcessConsultProviderFinalizeFailure::BeforeContinuation(
                    Box::new(ResidentProcessConsultProviderCompletionFailure::Selection {
                        ready: Box::new(ResidentProcessConsultationCompletionReady {
                            selector,
                            ready,
                        }),
                        reason,
                    }),
                );
                return Err(Box::new(
                    ResidentProcessConsultProviderChildTurnFailure::Completion {
                        failure: Box::new(failure),
                        parent: Box::new(parent),
                    },
                ));
            }
        };
        let result = {
            let mut driver = match self.consultation_child_driver(selector) {
                Ok(driver) => driver,
                Err(reason) => {
                    let failure = ResidentProcessConsultProviderFinalizeFailure::BeforeContinuation(
                        Box::new(ResidentProcessConsultProviderCompletionFailure::Selection {
                            ready: Box::new(ready),
                            reason,
                        }),
                    );
                    return Err(Box::new(
                        ResidentProcessConsultProviderChildTurnFailure::Completion {
                            failure: Box::new(failure),
                            parent: Box::new(parent),
                        },
                    ));
                }
            };
            driver.finalize_consultation_provider_child(
                &mut access,
                ready,
                retained_operation_attempt_history(&history),
                recovered_dispatch,
                lost_input,
                evidence,
                &mut recovery,
                recording,
            )
        };
        let completion = match result {
            Ok(completion) => completion,
            Err(failure) => {
                let failure = match failure {
                    ResidentProcessConsultProviderCompletionFailure::Recording(failure) => {
                        ResidentProcessConsultProviderFinalizeFailure::Continuation(Box::new(
                            ResidentProcessConsultProviderCompletionContinuation::Recording {
                                access,
                                failure,
                                recovered_head: Some(recovered_dispatch),
                                recovery,
                            },
                        ))
                    }
                    ResidentProcessConsultProviderCompletionFailure::LostRecording(failure) => {
                        ResidentProcessConsultProviderFinalizeFailure::Continuation(Box::new(
                            ResidentProcessConsultProviderCompletionContinuation::LostRecording {
                                access,
                                failure: *failure,
                                history,
                                recovered_dispatch,
                                input: lost_input,
                                recovery,
                            },
                        ))
                    }
                    ResidentProcessConsultProviderCompletionFailure::Classification(failure) => {
                        ResidentProcessConsultProviderFinalizeFailure::Continuation(Box::new(
                            ResidentProcessConsultProviderCompletionContinuation::Classification {
                                access,
                                failure,
                                recovery,
                            },
                        ))
                    }
                    ResidentProcessConsultProviderCompletionFailure::Settlement {
                        failure,
                        child,
                    } => ResidentProcessConsultProviderFinalizeFailure::Continuation(Box::new(
                        ResidentProcessConsultProviderCompletionContinuation::settlement(
                            failure, child,
                        ),
                    )),
                    failure => ResidentProcessConsultProviderFinalizeFailure::BeforeContinuation(
                        Box::new(failure),
                    ),
                };
                return Err(Box::new(
                    ResidentProcessConsultProviderChildTurnFailure::Completion {
                        failure: Box::new(failure),
                        parent: Box::new(parent),
                    },
                ));
            }
        };

        let (ready, child) = match completion {
            ResidentProcessConsultProviderCompletionDisposition::Nonterminal(settled) => {
                return Ok(ResidentProcessConsultProviderChildDisposition::Nonterminal(
                    settled,
                ));
            }
            ResidentProcessConsultProviderCompletionDisposition::Terminal { ready, child } => {
                (ready, child)
            }
        };
        let ResidentProcessConsultProviderParentTurn {
            association,
            access: parent_access,
            authority,
            slot_id,
            parent,
            parent_observation,
            observation_ref,
            signing,
            limits,
            checkpoint_accumulator_limits,
            checkpoint_identity,
            checkpoint_schema,
            checkpoint_authority,
            observe_join_time,
            resolve_store,
            admit_evidence,
            admit_collection,
            rotate,
            sign_observation,
            sign_checkpoint,
        } = parent;
        let join_time = observe_join_time(&child);
        match self.complete_consultation_provider_parent(
            association,
            parent_access,
            authority,
            ResidentProcessConsultProviderCompletionDisposition::Terminal { ready, child },
            ResidentProcessConsultProviderParentJoinInput {
                slot_id,
                recorded_at: join_time,
                parent,
                parent_observation,
                observation_ref,
                observed_at: join_time,
                signing,
                limits,
            },
            ConsultationSuccessorCommitInput {
                accumulator_limits: checkpoint_accumulator_limits,
                identity: checkpoint_identity,
                issued_at: join_time,
                schema: checkpoint_schema,
                authority: checkpoint_authority,
            },
            resolve_store,
            admit_evidence,
            admit_collection,
            rotate,
            sign_observation,
            sign_checkpoint,
        ) {
            Ok(ResidentProcessConsultProviderParentDisposition::Joined(joined)) => Ok(
                ResidentProcessConsultProviderChildDisposition::Joined(joined),
            ),
            Ok(ResidentProcessConsultProviderParentDisposition::Nonterminal(settled)) => Ok(
                ResidentProcessConsultProviderChildDisposition::Nonterminal(settled),
            ),
            Err(failure) => Err(Box::new(
                ResidentProcessConsultProviderChildTurnFailure::Join(failure),
            )),
        }
    }

    /// Open one selected provider-backed Consult child.
    pub(super) fn open_consultation_provider_child<'input, 'operation, 'call, Authority, Executor>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        turn: ResidentProcessConsultProviderOpenTurn<
            'input,
            'operation,
            'call,
            Authority,
            Executor,
        >,
    ) -> ResidentProcessConsultProviderOpenResult<'input, 'operation, 'call, Authority, Executor>
    where
        'operation: 'call,
        Authority: super::super::AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
    {
        let selected = turn.recovered.start.selector == *selector
            && turn.recovered.start.lineage.consult_child().is_some()
            && matches!(
                turn.recovered.phase,
                ResidentProcessOperationResolutionPhase::Resolving
                    | ResidentProcessOperationResolutionPhase::ContextBound
            );
        if !selected {
            return Err(ResidentProcessConsultProviderPhaseRefusal {
                input: Box::new(turn),
                reason: ResidentProcessHostSelectionError::ConsultationChildAssociation,
            });
        }
        let mut driver = match self.consultation_child_driver(selector) {
            Ok(driver) => driver,
            Err(reason) => {
                return Err(ResidentProcessConsultProviderPhaseRefusal {
                    input: Box::new(turn),
                    reason,
                });
            }
        };
        let ResidentProcessConsultProviderOpenTurn {
            association,
            access,
            authority,
            task_limits,
            observed_at,
            current,
            planned,
            candidate,
            parent,
            recovered,
            mut restart,
            admission,
            dispatch,
            invocation,
            executor,
            rotate,
            sign_observation,
            sign_checkpoint,
        } = turn;
        Ok(driver.supervise_consultation_provider_child_turn(
            association,
            access,
            authority,
            task_limits,
            observed_at,
            current,
            planned,
            candidate,
            parent,
            recovered,
            &mut restart,
            admission,
            dispatch,
            invocation,
            executor,
            rotate,
            sign_observation,
            sign_checkpoint,
        ))
    }

    /// Resume one selected same-process provider-backed Consult child.
    pub(super) fn resume_consultation_provider_child<
        'input,
        'operation,
        'call,
        Authority,
        Executor,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        turn: ResidentProcessConsultProviderResumeTurn<
            'input,
            'operation,
            'call,
            Authority,
            Executor,
        >,
    ) -> ResidentProcessConsultProviderResumeResult<'input, 'operation, 'call, Authority, Executor>
    where
        'operation: 'call,
        Authority: super::super::AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
    {
        if turn.phase.start().selector != *selector
            || turn.phase.start().lineage.consult_child().is_none()
        {
            return Err(ResidentProcessConsultProviderPhaseRefusal {
                input: Box::new(turn),
                reason: ResidentProcessHostSelectionError::ConsultationChildAssociation,
            });
        }
        let mut driver = match self.consultation_child_driver(selector) {
            Ok(driver) => driver,
            Err(reason) => {
                return Err(ResidentProcessConsultProviderPhaseRefusal {
                    input: Box::new(turn),
                    reason,
                });
            }
        };
        let ResidentProcessConsultProviderResumeTurn {
            association,
            access,
            authority,
            task_limits,
            observed_at,
            current,
            planned,
            candidate,
            parent,
            phase,
            release,
            invocation,
            provider_admission,
            restart,
            admission,
            executor,
        } = turn;
        Ok(driver.resume_consultation_provider_child_turn(
            association,
            access,
            authority,
            task_limits,
            observed_at,
            current,
            planned,
            candidate,
            parent,
            phase,
            release,
            invocation,
            provider_admission,
            restart,
            admission,
            executor,
        ))
    }

    /// Convert one cold historical provider dispatch into durable Consult loss.
    pub(super) fn lose_cold_consultation_provider_child<'input, 'operation, 'call>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        turn: ResidentProcessConsultProviderColdLossTurn<'input, 'operation, 'call>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
        Box<ResidentProcessConsultProviderColdLossFailure<'input, 'operation, 'call>>,
    >
    where
        'operation: 'call,
    {
        if turn.attempt.start.selector != *selector {
            return Err(Box::new(
                ResidentProcessConsultProviderColdLossFailure::Selection(
                    ResidentProcessConsultProviderPhaseRefusal {
                        input: Box::new(turn),
                        reason: ResidentProcessHostSelectionError::ProviderOperationAuthority,
                    },
                ),
            ));
        }
        let index = match self.deployment_authority_index(selector) {
            Ok(index) => index,
            Err(reason) => {
                return Err(Box::new(
                    ResidentProcessConsultProviderColdLossFailure::Selection(
                        ResidentProcessConsultProviderPhaseRefusal {
                            input: Box::new(turn),
                            reason,
                        },
                    ),
                ));
            }
        };
        let application = self.deployments[index].application.as_ref();
        let Some(generation) = application.generation(&turn.attempt.start.generation_digest) else {
            return Err(Box::new(
                ResidentProcessConsultProviderColdLossFailure::Selection(
                    ResidentProcessConsultProviderPhaseRefusal {
                        input: Box::new(turn),
                        reason: ResidentProcessHostSelectionError::HistoricalProviderAuthority,
                    },
                ),
            ));
        };
        if let Err(reason) = validate_consult_provider_cold_loss_associations(
            selector,
            self.deployments[index].store(),
            generation,
            &turn,
        ) {
            return Err(Box::new(
                ResidentProcessConsultProviderColdLossFailure::Selection(
                    ResidentProcessConsultProviderPhaseRefusal {
                        input: Box::new(turn),
                        reason,
                    },
                ),
            ));
        }

        let driver = match self.consultation_child_driver(selector) {
            Ok(driver) => driver,
            Err(reason) => {
                return Err(Box::new(
                    ResidentProcessConsultProviderColdLossFailure::Selection(
                        ResidentProcessConsultProviderPhaseRefusal {
                            input: Box::new(turn),
                            reason,
                        },
                    ),
                ));
            }
        };
        let recovered = turn.attempt;
        let (dispatch_ref, dispatch) = recovered.start.control.observation().selected();
        let reconciled = match driver
            .operations
            .reconcile_consultation_batch(turn.recovery.planned.clone(), turn.task_limits)
        {
            Ok(reconciled) => reconciled,
            Err(reason) => {
                return Err(Box::new(
                    ResidentProcessConsultProviderColdLossFailure::Batch {
                        turn: Box::new(turn),
                        reason,
                    },
                ));
            }
        };
        let coordinate = match reconciled {
            crate::resident::consultation::service::task::ResidentConsultationBatchReconciliation::Reused(coordinate)
            | crate::resident::consultation::service::task::ResidentConsultationBatchReconciliation::Opened(coordinate)
            | crate::resident::consultation::service::task::ResidentConsultationBatchReconciliation::Advanced { opened: coordinate, .. } => coordinate,
            crate::resident::consultation::service::task::ResidentConsultationBatchReconciliation::Busy { reason, .. } => {
                return Err(Box::new(ResidentProcessConsultProviderColdLossFailure::Busy {
                    turn: Box::new(turn),
                    reason,
                }));
            }
        };
        let prepared = match driver.operations.prepare_recovered_consultation_loss(
            &coordinate,
            turn.candidate,
            &turn.current,
            turn.recovery.observed_at,
            recovered.start.control.operation().store(),
            recovered.start.control.operation().operation(),
            dispatch_ref,
            dispatch,
        ) {
            Ok(prepared) => prepared,
            Err(reason) => {
                return Err(Box::new(
                    ResidentProcessConsultProviderColdLossFailure::Ownership {
                        turn: Box::new(turn),
                        reason,
                    },
                ));
            }
        };
        let pending = match driver
            .operations
            .offer_recovered_consultation_loss(prepared)
        {
            Ok(pending) => pending,
            Err(local) => {
                let ResidentProcessConsultProviderColdLossTurn {
                    attempt: _,
                    access,
                    task_limits: _,
                    current,
                    candidate: _,
                    history,
                    input,
                    evidence,
                    recovery,
                    recording,
                } = turn;
                return Err(cold_loss_local_failure(
                    Box::new(ResidentProcessConsultProviderColdLossRecording {
                        selector: selector.clone(),
                        access,
                        current,
                        history,
                        recovered_dispatch: recovered.start.control.observation(),
                        input,
                        evidence,
                        recovery,
                        recording,
                    }),
                    local,
                ));
            }
        };
        let poll = match driver.operations.retry_recovered_consultation_loss::<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >(pending) {
            Ok(poll) => poll,
            Err(local) => {
                let ResidentProcessConsultProviderColdLossTurn {
                    attempt: _,
                    access,
                    task_limits: _,
                    current,
                    candidate: _,
                    history,
                    input,
                    evidence,
                    recovery,
                    recording,
                } = turn;
                return Err(cold_loss_local_failure(
                    Box::new(ResidentProcessConsultProviderColdLossRecording {
                        selector: selector.clone(),
                        access,
                        current,
                        history,
                        recovered_dispatch: recovered.start.control.observation(),
                        input,
                        evidence,
                        recovery,
                        recording,
                    }),
                    local,
                ));
            }
        };
        let ResidentProcessConsultProviderColdLossTurn {
            attempt: _,
            access,
            task_limits: _,
            current,
            candidate: _,
            history,
            input,
            evidence,
            recovery,
            recording,
        } = turn;
        let ready = ResidentConsultationCompletionReady::from_parts(current, poll);
        self.finalize_lost_consultation_provider_child(
            selector,
            ResidentProcessConsultProviderFinalizeTurn {
                access,
                ready,
                history,
                recovered_dispatch: recovered.start.control.observation(),
                lost_input: input,
                evidence,
                recovery,
                recording,
            },
        )
        .map_err(|failure| {
            Box::new(ResidentProcessConsultProviderColdLossFailure::Completion(
                failure,
            ))
        })
    }

    /// Record and classify one selected lost provider-backed Consult child.
    ///
    /// Lost cold recovery cannot produce a provider completion. Keeping this
    /// path separate avoids allocating the successful-provider continuation
    /// frame while the exact outcome-unknown successor is recorded.
    fn finalize_lost_consultation_provider_child<'input, 'operation, 'call>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        turn: ResidentProcessConsultProviderFinalizeTurn<'input, 'operation, 'call>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
        Box<ResidentProcessConsultProviderFinalizeFailure<'input, 'operation, 'call>>,
    >
    where
        'operation: 'call,
    {
        let ResidentProcessConsultProviderFinalizeTurn {
            mut access,
            ready,
            history,
            recovered_dispatch,
            lost_input,
            evidence: _,
            mut recovery,
            recording,
        } = turn;
        let ready = match self.bind_consultation_completion(selector.clone(), ready) {
            Ok(ready) => ready,
            Err(failure) => {
                let ResidentProcessConsultationCompletionBindingFailure {
                    selector,
                    ready,
                    reason,
                } = *failure;
                return Err(Box::new(
                    ResidentProcessConsultProviderFinalizeFailure::BeforeContinuation(Box::new(
                        ResidentProcessConsultProviderCompletionFailure::Selection {
                            ready: Box::new(ResidentProcessConsultationCompletionReady {
                                selector,
                                ready,
                            }),
                            reason,
                        },
                    )),
                ));
            }
        };
        let mut driver = self
            .consultation_child_driver(selector)
            .expect("a just-bound completion retains its selected deployment");
        let result = driver.finalize_lost_consultation_provider_child(
            &mut access,
            ready,
            retained_operation_attempt_history(&history),
            recovered_dispatch,
            lost_input,
            &mut recovery,
            recording,
        );
        result.map_err(|failure| {
            Box::new(match failure {
                ResidentProcessConsultProviderCompletionFailure::LostRecording(failure) => {
                    ResidentProcessConsultProviderFinalizeFailure::Continuation(Box::new(
                        ResidentProcessConsultProviderCompletionContinuation::LostRecording {
                            access,
                            failure: *failure,
                            history,
                            recovered_dispatch,
                            input: lost_input,
                            recovery,
                        },
                    ))
                }
                ResidentProcessConsultProviderCompletionFailure::Classification(failure) => {
                    ResidentProcessConsultProviderFinalizeFailure::Continuation(Box::new(
                        ResidentProcessConsultProviderCompletionContinuation::Classification {
                            access,
                            failure,
                            recovery,
                        },
                    ))
                }
                ResidentProcessConsultProviderCompletionFailure::Settlement { failure, child } => {
                    ResidentProcessConsultProviderFinalizeFailure::Continuation(Box::new(
                        ResidentProcessConsultProviderCompletionContinuation::settlement(
                            failure, child,
                        ),
                    ))
                }
                failure => ResidentProcessConsultProviderFinalizeFailure::BeforeContinuation(
                    Box::new(failure),
                ),
            })
        })
    }

    /// Retry only a retained local offer, then enter the accepted recording path.
    pub(super) fn continue_cold_consultation_provider_loss<'input, 'operation, 'call>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        continuation: ResidentProcessConsultProviderColdLossOffered<'input, 'operation, 'call>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
        Box<ResidentProcessConsultProviderColdLossFailure<'input, 'operation, 'call>>,
    >
    where
        'operation: 'call,
    {
        if continuation.selector() != selector {
            return Err(Box::new(
                ResidentProcessConsultProviderColdLossFailure::Offered(Box::new(continuation)),
            ));
        }
        let driver = match self.consultation_child_driver(selector) {
            Ok(driver) => driver,
            Err(_) => {
                return Err(Box::new(
                    ResidentProcessConsultProviderColdLossFailure::Offered(Box::new(continuation)),
                ));
            }
        };
        let ResidentProcessConsultProviderColdLossOffered {
            recording,
            pending,
            reason: _,
        } = continuation;
        let poll = driver.operations.retry_recovered_consultation_loss::<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >(*pending);
        let poll = match poll {
            Ok(poll) => poll,
            Err(local) => return Err(cold_loss_local_failure(recording, local)),
        };
        drop(driver);
        let ResidentProcessConsultProviderColdLossRecording {
            selector: _,
            access,
            current,
            history,
            recovered_dispatch,
            input,
            evidence,
            recovery,
            recording,
        } = *recording;
        self.finalize_consultation_provider_child(
            selector,
            ResidentProcessConsultProviderFinalizeTurn {
                access,
                ready: ResidentConsultationCompletionReady::from_parts(current, poll),
                history,
                recovered_dispatch,
                lost_input: input,
                evidence,
                recovery,
                recording,
            },
        )
        .map_err(|failure| {
            Box::new(ResidentProcessConsultProviderColdLossFailure::Completion(
                failure,
            ))
        })
    }

    /// Poll one selected provider-backed Consult child.
    pub(super) fn poll_consultation_provider_child<'input, 'operation, 'call, Executor>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        turn: ResidentProcessConsultProviderPollTurn<'input, 'operation, Executor>,
    ) -> ResidentProcessConsultProviderPollResult<'input, 'operation, 'call, Executor>
    where
        'operation: 'call,
        Executor: ResidentOperationTaskExecutor<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
    {
        if turn.recovered.start.selector != *selector
            || turn.recovered.start.lineage.consult_child().is_none()
        {
            return Err(ResidentProcessConsultProviderPhaseRefusal {
                input: Box::new(turn),
                reason: ResidentProcessHostSelectionError::ConsultationChildAssociation,
            });
        }
        let mut driver = match self.consultation_child_driver(selector) {
            Ok(driver) => driver,
            Err(reason) => {
                return Err(ResidentProcessConsultProviderPhaseRefusal {
                    input: Box::new(turn),
                    reason,
                });
            }
        };
        let ResidentProcessConsultProviderPollTurn {
            recovered: _,
            task_limits,
            observed_at,
            current,
            executor,
        } = turn;
        Ok(driver.poll_consultation_provider_child_turn(
            task_limits,
            observed_at,
            current,
            executor,
        ))
    }

    /// Record and classify one selected provider-backed Consult completion.
    pub(super) fn finalize_consultation_provider_child<'input, 'operation, 'call>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        turn: ResidentProcessConsultProviderFinalizeTurn<'input, 'operation, 'call>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
        Box<ResidentProcessConsultProviderFinalizeFailure<'input, 'operation, 'call>>,
    >
    where
        'operation: 'call,
    {
        let ResidentProcessConsultProviderFinalizeTurn {
            mut access,
            ready,
            history,
            recovered_dispatch,
            lost_input,
            evidence,
            mut recovery,
            recording,
        } = turn;
        let ready = match self.bind_consultation_completion(selector.clone(), ready) {
            Ok(ready) => ready,
            Err(failure) => {
                let ResidentProcessConsultationCompletionBindingFailure {
                    selector,
                    ready,
                    reason,
                } = *failure;
                return Err(Box::new(
                    ResidentProcessConsultProviderFinalizeFailure::BeforeContinuation(Box::new(
                        ResidentProcessConsultProviderCompletionFailure::Selection {
                            ready: Box::new(ResidentProcessConsultationCompletionReady {
                                selector,
                                ready,
                            }),
                            reason,
                        },
                    )),
                ));
            }
        };
        let mut driver = self
            .consultation_child_driver(selector)
            .expect("a just-bound completion retains its selected deployment");
        let result = driver.finalize_consultation_provider_child(
            &mut access,
            ready,
            retained_operation_attempt_history(&history),
            recovered_dispatch,
            lost_input,
            evidence,
            &mut recovery,
            recording,
        );
        result.map_err(|failure| {
            Box::new(match failure {
                ResidentProcessConsultProviderCompletionFailure::Recording(failure) => {
                    ResidentProcessConsultProviderFinalizeFailure::Continuation(Box::new(
                        ResidentProcessConsultProviderCompletionContinuation::Recording {
                            access,
                            failure,
                            recovered_head: Some(recovered_dispatch),
                            recovery,
                        },
                    ))
                }
                ResidentProcessConsultProviderCompletionFailure::LostRecording(failure) => {
                    ResidentProcessConsultProviderFinalizeFailure::Continuation(Box::new(
                        ResidentProcessConsultProviderCompletionContinuation::LostRecording {
                            access,
                            failure: *failure,
                            history,
                            recovered_dispatch,
                            input: lost_input,
                            recovery,
                        },
                    ))
                }
                ResidentProcessConsultProviderCompletionFailure::Classification(failure) => {
                    ResidentProcessConsultProviderFinalizeFailure::Continuation(Box::new(
                        ResidentProcessConsultProviderCompletionContinuation::Classification {
                            access,
                            failure,
                            recovery,
                        },
                    ))
                }
                ResidentProcessConsultProviderCompletionFailure::Settlement { failure, child } => {
                    ResidentProcessConsultProviderFinalizeFailure::Continuation(Box::new(
                        ResidentProcessConsultProviderCompletionContinuation::settlement(
                            failure, child,
                        ),
                    ))
                }
                failure => ResidentProcessConsultProviderFinalizeFailure::BeforeContinuation(
                    Box::new(failure),
                ),
            })
        })
    }

    /// Continue one consumed provider completion without another provider call.
    pub(super) fn continue_consultation_provider_completion<'input, 'operation, 'call>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        turn: ResidentProcessConsultProviderCompletionContinuationTurn<'input, 'operation, 'call>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
        ResidentProcessConsultProviderContinuationFailure<'input, 'operation, 'call>,
    >
    where
        'operation: 'call,
    {
        if turn.selector() != selector {
            return Err(
                ResidentProcessConsultProviderContinuationFailure::Selection(Box::new(
                    ResidentProcessConsultProviderContinuationRefusal {
                        turn: Box::new(turn),
                        reason: ResidentProcessHostSelectionError::ConsultationChildAssociation,
                    },
                )),
            );
        }
        let mut driver = match self.consultation_child_driver(selector) {
            Ok(driver) => driver,
            Err(reason) => {
                return Err(
                    ResidentProcessConsultProviderContinuationFailure::Selection(Box::new(
                        ResidentProcessConsultProviderContinuationRefusal {
                            turn: Box::new(turn),
                            reason,
                        },
                    )),
                );
            }
        };
        let ResidentProcessConsultProviderCompletionContinuationTurn {
            continuation,
            recording,
            checkpoint_signer,
        } = turn;
        match *continuation {
            ResidentProcessConsultProviderCompletionContinuation::Recording {
                mut access,
                failure,
                recovered_head,
                mut recovery,
            } => {
                debug_assert!(checkpoint_signer.is_none());
                match driver.continue_consultation_provider_recording(
                    &mut access,
                    *failure,
                    recovered_head,
                    &mut recovery,
                    recording,
                ) {
                    Ok(disposition) => Ok(disposition),
                    Err(failure) => {
                        let continuation = match failure {
                        ResidentProcessConsultProviderCompletionFailure::Recording(failure) => {
                            ResidentProcessConsultProviderCompletionContinuation::Recording {
                                access,
                                failure,
                                recovered_head,
                                recovery,
                            }
                        }
                        ResidentProcessConsultProviderCompletionFailure::Classification(
                            failure,
                        ) => ResidentProcessConsultProviderCompletionContinuation::Classification {
                            access,
                            failure,
                            recovery,
                        },
                        ResidentProcessConsultProviderCompletionFailure::Settlement {
                            failure,
                            child,
                        } => ResidentProcessConsultProviderCompletionContinuation::settlement(
                            failure, child,
                        ),
                        _ => unreachable!("recording continuation advances only owned phases"),
                    };
                        Err(
                            ResidentProcessConsultProviderContinuationFailure::Completion(
                                Box::new(continuation),
                            ),
                        )
                    }
                }
            }
            ResidentProcessConsultProviderCompletionContinuation::LostRecording {
                mut access,
                failure,
                history,
                recovered_dispatch,
                input,
                mut recovery,
            } => match driver.continue_consultation_provider_lost_recording(
                &mut access,
                failure,
                retained_operation_attempt_history(&history),
                recovered_dispatch,
                input,
                &mut recovery,
                recording,
                checkpoint_signer,
            ) {
                Ok(disposition) => Ok(disposition),
                Err(failure) => {
                    let continuation = match failure {
                        ResidentProcessConsultProviderCompletionFailure::LostRecording(failure) => {
                            ResidentProcessConsultProviderCompletionContinuation::LostRecording {
                                access,
                                failure: *failure,
                                history,
                                recovered_dispatch,
                                input,
                                recovery,
                            }
                        }
                        ResidentProcessConsultProviderCompletionFailure::Classification(
                            failure,
                        ) => ResidentProcessConsultProviderCompletionContinuation::Classification {
                            access,
                            failure,
                            recovery,
                        },
                        ResidentProcessConsultProviderCompletionFailure::Settlement {
                            failure,
                            child,
                        } => ResidentProcessConsultProviderCompletionContinuation::settlement(
                            failure, child,
                        ),
                        _ => unreachable!("lost-recording continuation advances only owned phases"),
                    };
                    Err(
                        ResidentProcessConsultProviderContinuationFailure::Completion(Box::new(
                            continuation,
                        )),
                    )
                }
            },
            ResidentProcessConsultProviderCompletionContinuation::Classification {
                mut access,
                failure,
                mut recovery,
            } => match driver.continue_consultation_provider_classification(
                &mut access,
                *failure,
                &mut recovery,
            ) {
                Ok(disposition) => Ok(disposition),
                Err(failure) => {
                    let continuation = match failure {
                        ResidentProcessConsultProviderCompletionFailure::Classification(
                            failure,
                        ) => ResidentProcessConsultProviderCompletionContinuation::Classification {
                            access,
                            failure,
                            recovery,
                        },
                        ResidentProcessConsultProviderCompletionFailure::Settlement {
                            failure,
                            child,
                        } => ResidentProcessConsultProviderCompletionContinuation::settlement(
                            failure, child,
                        ),
                        _ => unreachable!("classification continuation advances only owned phases"),
                    };
                    Err(
                        ResidentProcessConsultProviderContinuationFailure::Completion(Box::new(
                            continuation,
                        )),
                    )
                }
            },
            ResidentProcessConsultProviderCompletionContinuation::Settlement { failure, child } => {
                match driver.continue_consultation_provider_settlement(failure, child) {
                    Ok(disposition) => Ok(disposition),
                    Err(ResidentProcessConsultProviderCompletionFailure::Settlement {
                        failure,
                        child,
                    }) => Err(
                        ResidentProcessConsultProviderContinuationFailure::Completion(Box::new(
                            ResidentProcessConsultProviderCompletionContinuation::settlement(
                                failure, child,
                            ),
                        )),
                    ),
                    Err(_) => unreachable!("settlement continuation returns settlement failure"),
                }
            }
        }
    }
}

impl<'host, 'store> ResidentProcessConsultChildDriver<'host, 'store> {
    /// Poll one retained provider child without selecting or dispatching another.
    pub(super) fn poll_consultation_provider_child_turn<'operation, 'call, Executor>(
        &mut self,
        task_limits: ResidentConsultationTaskLimits,
        observed_at: UnixTimeV1,
        current: RecoveredConsultationCollection,
        executor: &mut Executor,
    ) -> ResidentConsultationTurnDisposition<
        ResidentProcessConsultProviderChildWork<'operation, 'call>,
        ResidentProcessConsultProviderChildOutput,
        std::convert::Infallible,
        std::convert::Infallible,
        std::convert::Infallible,
        (),
    >
    where
        Executor: ResidentOperationTaskExecutor<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
    {
        self.operations.supervise_recovered_consultation_turn(
            task_limits,
            observed_at,
            current,
            executor,
            |_, _| ResidentConsultationSupervisionDecision::Continue(None),
            |_| unreachable!("a ready completion is polled before child recovery"),
            |_, _| unreachable!("a completion-only turn never retains provider work"),
        )
    }

    /// Offer one same-process recovered dispatch through its exact Consult owner.
    ///
    /// The input release can exist only after `ProviderResolved` or
    /// `DispatchPrepared` reached a newly committed `Dispatched` successor in
    /// this process. Historical dispatch recovery cannot construct it.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub(super) fn resume_consultation_provider_child_turn<
        'operation,
        'call,
        Authority,
        Restart,
        Executor,
    >(
        &mut self,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        task_limits: ResidentConsultationTaskLimits,
        observed_at: UnixTimeV1,
        current: RecoveredConsultationCollection,
        planned: &ConsultationChildDispatchBatch,
        candidate: &ConsultationChildDispatchCandidate,
        parent: &ResidentProcessRecoveredOperationStart,
        phase: ResidentProcessConsultProviderResumePhase<'operation>,
        release: RecoveredProviderDispatchRelease,
        invocation: ProviderOperationInvokeInput<'call>,
        provider_admission: crate::provider_invocation::ProviderInvocationAdmissionContext<
            'operation,
        >,
        restart: Restart,
        admission: &mut OperationControlRecoveryAdmission<'_>,
        executor: &mut Executor,
    ) -> Result<
        crate::resident::task_owner::ResidentOperationTaskKey,
        ResidentProcessConsultProviderResumeFailure<'operation, 'call>,
    >
    where
        'operation: 'call,
        Authority: super::super::AmbientSessionAuthorityPort,
        Restart: FnOnce() -> ResidentProcessOperationRestartInput<'operation>,
        Executor: ResidentOperationTaskExecutor<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
    {
        let source = phase.start();
        if let Err(reason) = validate_consult_provider_resume_associations(
            &self.selector,
            &self.store,
            &association,
            &access,
            &current,
            planned,
            candidate,
            parent,
            &phase,
            &release,
            &invocation,
            &provider_admission,
            observed_at,
        ) {
            return Err(ResidentProcessConsultProviderResumeFailure::Selection {
                release: Box::new(release),
                reason,
            });
        }
        let (released_ref, released_observation) = {
            let (reference, observation) = release.observation();
            (reference.clone(), observation.clone())
        };
        let selected_dispatch = ImmutableRecordReferenceV1 {
            record_ref: released_ref.clone(),
            record_sha256: match ArtifactDigest::parse(
                released_observation
                    .canonical_operation_observation_sha256()
                    .as_str(),
            ) {
                Ok(digest) => digest,
                Err(_) => {
                    return Err(ResidentProcessConsultProviderResumeFailure::Selection {
                        release: Box::new(release),
                        reason: ResidentProcessHostSelectionError::ConsultationChildAssociation,
                    });
                }
            },
        };

        let reconciled = match self
            .operations
            .reconcile_consultation_batch(planned.clone(), task_limits)
        {
            Ok(reconciled) => reconciled,
            Err(reason) => {
                return Err(ResidentProcessConsultProviderResumeFailure::Batch {
                    release: Box::new(release),
                    reason,
                });
            }
        };
        let Some(coordinate) = reconciled.usable().cloned() else {
            return Err(ResidentProcessConsultProviderResumeFailure::Busy(Box::new(
                release,
            )));
        };
        let prepared_ownership = match self.operations.prepare_resumed_consultation_task(
            &coordinate,
            candidate,
            &current,
            observed_at,
            release.store(),
            release.operation(),
            &released_ref,
            &released_observation,
        ) {
            Ok(prepared) => prepared,
            Err(reason) => {
                return Err(ResidentProcessConsultProviderResumeFailure::Ownership {
                    release: Box::new(release),
                    reason,
                });
            }
        };

        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let recovered = match self.classify_child_operation(
            association,
            ResidentDeploymentOperationAccess {
                store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            restart(),
            admission,
            parent,
            &current,
            planned,
            candidate,
            Some(&selected_dispatch),
            observed_at,
        ) {
            Ok(recovered) => recovered,
            Err(reason) => {
                return Err(ResidentProcessConsultProviderResumeFailure::Recovery {
                    release: Box::new(release),
                    reason,
                });
            }
        };
        let ResidentProcessOperationRestartDisposition::Provider(recovered) = recovered else {
            return Err(ResidentProcessConsultProviderResumeFailure::Recovery {
                release: Box::new(release),
                reason: ResidentProcessOperationRestartError::StartEvidence,
            });
        };
        if recovered.start.selector != source.selector
            || recovered.start.generation_digest != source.generation_digest
            || recovered.start.lineage != source.lineage
        {
            return Err(ResidentProcessConsultProviderResumeFailure::Recovery {
                release: Box::new(release),
                reason: ResidentProcessOperationRestartError::StartEvidence,
            });
        }
        let prepared = prepare_recovered_provider_operation_call(
            release,
            &invocation,
            authority,
            custody,
            anchor,
        )
        .map_err(ResidentProcessConsultProviderResumeFailure::Call)?;
        let (call_store, call_dispatch, continuation, request) = prepared.into_task_parts();
        let work = ResidentProcessConsultProviderChildWork {
            store: self.store.clone(),
            lineage: source.lineage.clone(),
            dispatch_ref: released_ref,
            dispatch: released_observation,
            basis: Some(ResidentProcessConsultProviderCallBasis::Recovered {
                continuation,
                admission: provider_admission,
            }),
            request,
            invocation: Some(invocation),
        };
        if call_store != self.store || call_dispatch != selected_dispatch {
            return Err(ResidentProcessConsultProviderResumeFailure::CallIdentity {
                prepared: Box::new(prepared_ownership),
                work: Box::new(work),
                reason: ResidentProcessHostSelectionError::ConsultationChildAssociation,
            });
        }
        self.operations
            .accept_prepared_resumed_consultation_task(prepared_ownership, work, executor)
            .map_err(|failure| {
                ResidentProcessConsultProviderResumeFailure::Acceptance(Box::new(failure))
            })
    }

    /// Offer one provider-backed child after its dispatch is durable.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub(super) fn supervise_consultation_provider_child_turn<
        'operation,
        'call,
        Authority,
        Restart,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Executor,
    >(
        &mut self,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        task_limits: ResidentConsultationTaskLimits,
        observed_at: UnixTimeV1,
        current: RecoveredConsultationCollection,
        planned: &ConsultationChildDispatchBatch,
        candidate: ConsultationChildDispatchCandidate,
        parent: &'operation ResidentProcessRecoveredOperationStart,
        recovered: &'operation ResidentProcessRecoveredOperationResolution,
        restart: &mut Restart,
        admission: &mut OperationControlRecoveryAdmission<'_>,
        dispatch: ProviderOperationDispatchInput<'operation, 'call>,
        invocation: ProviderOperationInvokeInput<'call>,
        executor: &mut Executor,
        mut admit_rotation: Rotation,
        mut sign_observation: ObservationSign,
        mut sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentConsultationTurnDisposition<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
            std::convert::Infallible,
            ResidentProcessOperationRestartError,
            ResidentProcessConsultProviderChildRetentionFailure<'operation>,
            (),
        >,
        ResidentProcessOperationRestartError,
    >
    where
        'operation: 'call,
        Authority: super::super::AmbientSessionAuthorityPort,
        Restart: FnMut() -> ResidentProcessOperationRestartInput<'operation>,
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
        Executor: ResidentOperationTaskExecutor<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
    {
        let selector = &self.selector;
        let ResidentOperationControlAssociation {
            current_request_authority,
            historical_handle_authority,
            recovery_store,
            checkpoint_authority,
        } = association;
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let preflight = ResidentProcessHost::<()>::classify_consultation_child_operation_in(
            self.application,
            selector,
            ResidentOperationControlAssociation {
                current_request_authority,
                historical_handle_authority,
                recovery_store,
                checkpoint_authority,
            },
            ResidentDeploymentOperationAccess {
                store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            restart(),
            admission,
            parent,
            &current,
            planned,
            &candidate,
            None,
            observed_at,
        )?;
        let ResidentProcessOperationRestartDisposition::OperationResolution(preflight) = preflight
        else {
            return Err(ResidentProcessOperationRestartError::StartEvidence);
        };
        if preflight.start.selector != recovered.start.selector
            || preflight.start.generation_digest != recovered.start.generation_digest
            || preflight.start.lineage != recovered.start.lineage
        {
            return Err(ResidentProcessOperationRestartError::StartEvidence);
        }
        let (_, preflight_observation) = preflight.start.control.into_parts();

        let application = self.application;
        let operations = &mut *self.operations;
        let current_for_release = current.clone();
        let planned_for_release = planned.clone();
        let parent_for_release = parent;
        let selector_for_release = selector.clone();
        let mut preflight_observation = Some(preflight_observation);
        let mut dispatch = Some(dispatch);
        let mut invocation = Some(invocation);
        Ok(operations.supervise_recovered_consultation_turn(
            task_limits,
            observed_at,
            current,
            executor,
            move |_, _| {
                ResidentConsultationSupervisionDecision::Continue(Some(Box::new(candidate)))
            },
            move |_| {
                Ok(preflight_observation
                    .take()
                    .expect("one provider candidate recovery"))
            },
            move |current_candidate, _| {
                let retained =
                    ResidentProcessHost::<()>::prepare_consultation_child_provider_work_in(
                        application,
                        recovered,
                        ResidentDeploymentOperationAccess {
                            store,
                            custody: &mut *custody,
                            sessions,
                            anchor: &mut *anchor,
                        },
                        dispatch.take().expect("one provider dispatch input"),
                        invocation.take().expect("one provider invocation input"),
                        authority,
                        &mut admit_rotation,
                        &mut sign_observation,
                        &mut sign_checkpoint,
                    );
                let work = match retained {
                    Ok(work) => work,
                    Err(
                        reason @ ResidentProcessConsultProviderChildRetentionFailure::Unopened(_),
                    ) => return ConsultationChildRetentionOutcome::Unopened(reason),
                    Err(
                        reason @ ResidentProcessConsultProviderChildRetentionFailure::Crossed(_),
                    ) => return ConsultationChildRetentionOutcome::Uncertain(reason),
                };
                let dispatched =
                    ResidentProcessHost::<()>::classify_consultation_child_operation_in(
                        application,
                        &selector_for_release,
                        ResidentOperationControlAssociation {
                            current_request_authority,
                            historical_handle_authority,
                            recovery_store,
                            checkpoint_authority,
                        },
                        ResidentDeploymentOperationAccess {
                            store,
                            custody: &mut *custody,
                            sessions,
                            anchor: &mut *anchor,
                        },
                        restart(),
                        admission,
                        parent_for_release,
                        &current_for_release,
                        &planned_for_release,
                        current_candidate,
                        None,
                        observed_at,
                    );
                let dispatched = match dispatched {
                    Ok(ResidentProcessOperationRestartDisposition::Provider(dispatched)) => {
                        dispatched.start.control.into_parts().1
                    }
                    Ok(_) => {
                        return ConsultationChildRetentionOutcome::Uncertain(
                            ResidentProcessConsultProviderChildRetentionFailure::Crossed(
                                ResidentProcessConsultProviderCrossedFailure::Recovery(
                                    ResidentProcessOperationRestartError::StartEvidence,
                                ),
                            ),
                        );
                    }
                    Err(reason) => {
                        return ConsultationChildRetentionOutcome::Uncertain(
                            ResidentProcessConsultProviderChildRetentionFailure::Crossed(
                                ResidentProcessConsultProviderCrossedFailure::Recovery(reason),
                            ),
                        );
                    }
                };
                ConsultationChildRetentionOutcome::Retained {
                    child: Box::new(dispatched),
                    work,
                }
            },
        ))
    }

    /// Release one provider child reservation after durable uncertainty.
    pub(super) fn settle_consultation_provider_nonterminal<Work, Output>(
        &mut self,
        ready: ResidentProcessConsultationCompletionReady<Work, Output>,
        child: &RecoveredOperationObservation,
    ) -> Result<
        CompletedResidentConsultationNonterminal<Work, Output>,
        ResidentProcessConsultProviderSettlementFailure<Work, Output>,
    > {
        let ResidentProcessConsultationCompletionReady { selector, ready } = ready;
        if selector != self.selector || ready.store() != &self.store || child.store() != &self.store
        {
            return Err(ResidentProcessConsultProviderSettlementFailure::Selection {
                ready: Box::new(ResidentProcessConsultationCompletionReady { selector, ready }),
                reason: ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation,
            });
        }
        self.operations
            .settle_consultation_nonterminal(*ready, child)
            .map_err(ResidentProcessConsultProviderSettlementFailure::Settlement)
    }

    /// Retry only the retained nonterminal settlement.
    pub(super) fn continue_consultation_provider_settlement<'operation, 'call>(
        &mut self,
        failure: ResidentProcessConsultProviderSettlementFailure<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
        child: Box<ResidentProcessRecoveredOperationStart>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
        ResidentProcessConsultProviderCompletionFailure<'operation, 'call>,
    > {
        let ready = match failure {
            ResidentProcessConsultProviderSettlementFailure::Selection { ready, .. } => *ready,
            ResidentProcessConsultProviderSettlementFailure::Settlement(failure) => {
                ResidentProcessConsultationCompletionReady {
                    selector: self.selector.clone(),
                    ready: Box::new(failure.into_ready()),
                }
            }
        };
        match self.settle_consultation_provider_nonterminal(ready, child.control.observation()) {
            Ok(completed) => Ok(
                ResidentProcessConsultProviderCompletionDisposition::Nonterminal(Box::new(
                    completed,
                )),
            ),
            Err(failure) => {
                Err(ResidentProcessConsultProviderCompletionFailure::Settlement { failure, child })
            }
        }
    }

    fn classify_consultation_provider_recorded<'input, 'operation, 'call>(
        &mut self,
        access: &mut ResidentDeploymentOperationAccess<'input>,
        ready: ResidentProcessConsultationCompletionReady<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
        recovery: &mut ResidentProcessConsultProviderRecordedRecovery<'input, 'operation>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
        ResidentProcessConsultProviderCompletionFailure<'operation, 'call>,
    > {
        let ResidentProcessConsultationCompletionReady { selector, ready } = ready;
        let (current, poll) = ready.into_parts();
        let (candidate, selected_dispatch, disposition) = match &poll.outcome {
            ResidentConsultationTaskPoll::Completed {
                candidate,
                dispatch,
                output:
                    ResidentProcessConsultProviderChildOutput::Recorded(
                        ProviderOperationInvokeResult::Completed(_),
                    ),
                ..
            } => (
                candidate,
                dispatch.dispatch(),
                ResidentProcessConsultProviderRecordedDisposition::Terminal,
            ),
            ResidentConsultationTaskPoll::Completed {
                candidate,
                dispatch,
                output:
                    ResidentProcessConsultProviderChildOutput::Recorded(
                        ProviderOperationInvokeResult::OutcomeUnknown(_),
                    ),
                ..
            } => (
                candidate,
                dispatch.dispatch(),
                ResidentProcessConsultProviderRecordedDisposition::RecoveryOnly,
            ),
            ResidentConsultationTaskPoll::Lost(lost) => (
                lost.candidate(),
                lost.dispatch().dispatch(),
                ResidentProcessConsultProviderRecordedDisposition::RecoveryOnly,
            ),
            ResidentConsultationTaskPoll::Completed {
                output: ResidentProcessConsultProviderChildOutput::Mechanical(_),
                ..
            }
            | ResidentConsultationTaskPoll::Stale(_) => {
                let ready = ResidentProcessConsultationCompletionReady {
                    selector,
                    ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                        current, poll,
                    )),
                };
                return Err(
                    ResidentProcessConsultProviderCompletionFailure::Classification(Box::new(
                        ResidentProcessConsultProviderClassificationFailure {
                            ready: Box::new(ready),
                            reason: ResidentProcessOperationRestartError::State,
                        },
                    )),
                );
            }
        };
        let association = ResidentOperationControlAssociation {
            current_request_authority: recovery.association.current_request_authority,
            historical_handle_authority: recovery.association.historical_handle_authority,
            recovery_store: recovery.association.recovery_store,
            checkpoint_authority: recovery.association.checkpoint_authority,
        };
        let restart = (recovery.restart)();
        let classified = self.classify_child_operation(
            association,
            ResidentDeploymentOperationAccess {
                store: access.store,
                custody: &mut *access.custody,
                sessions: access.sessions,
                anchor: &mut *access.anchor,
            },
            restart,
            &mut *recovery.admission,
            recovery.parent,
            &current,
            recovery.planned,
            candidate,
            Some(selected_dispatch),
            recovery.observed_at,
        );
        let ready = ResidentProcessConsultationCompletionReady {
            selector,
            ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                current, poll,
            )),
        };
        let classified = match classified {
            Ok(classified) => classified,
            Err(reason) => {
                return Err(
                    ResidentProcessConsultProviderCompletionFailure::Classification(Box::new(
                        ResidentProcessConsultProviderClassificationFailure {
                            ready: Box::new(ready),
                            reason,
                        },
                    )),
                );
            }
        };
        match (disposition, classified) {
            (
                ResidentProcessConsultProviderRecordedDisposition::Terminal,
                ResidentProcessOperationRestartDisposition::Terminal(child),
            ) => Ok(
                ResidentProcessConsultProviderCompletionDisposition::Terminal {
                    ready: Box::new(ready),
                    child,
                },
            ),
            (
                ResidentProcessConsultProviderRecordedDisposition::RecoveryOnly,
                ResidentProcessOperationRestartDisposition::RecoveryOnly(child),
            ) => match self
                .settle_consultation_provider_nonterminal(ready, child.control.observation())
            {
                Ok(completed) => Ok(
                    ResidentProcessConsultProviderCompletionDisposition::Nonterminal(Box::new(
                        completed,
                    )),
                ),
                Err(failure) => Err(
                    ResidentProcessConsultProviderCompletionFailure::Settlement { failure, child },
                ),
            },
            _ => Err(
                ResidentProcessConsultProviderCompletionFailure::Classification(Box::new(
                    ResidentProcessConsultProviderClassificationFailure {
                        ready: Box::new(ready),
                        reason: ResidentProcessOperationRestartError::State,
                    },
                )),
            ),
        }
    }

    /// Continue a consumed provider outcome without calling the provider again.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn continue_consultation_provider_recording<'input, 'operation, 'call>(
        &mut self,
        access: &mut ResidentDeploymentOperationAccess<'input>,
        failure: ResidentProcessConsultProviderRecordingFailure<'operation, 'call>,
        recovered_head: Option<&RecoveredOperationObservation>,
        recovery: &mut ResidentProcessConsultProviderRecordedRecovery<'input, 'operation>,
        recording: Option<ResidentProcessConsultProviderRecordingAttempt<'input>>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
        ResidentProcessConsultProviderCompletionFailure<'operation, 'call>,
    > {
        let ResidentProcessConsultProviderRecordingFailure {
            selector,
            current,
            coordinate,
            continuation,
            candidate,
            dispatch,
            work,
            cancellation,
            reason,
        } = failure;
        if selector != self.selector || access.store != &self.store || work.store != self.store {
            return Err(ResidentProcessConsultProviderCompletionFailure::Recording(
                Box::new(ResidentProcessConsultProviderRecordingFailure {
                    selector,
                    current,
                    coordinate,
                    continuation,
                    candidate,
                    dispatch,
                    work,
                    cancellation,
                    reason,
                }),
            ));
        }
        let input = work
            .invocation
            .as_ref()
            .expect("recording continuation retains provider invocation");
        let recorded = match reason {
            ProviderOperationRecordingFailure::Preparation { continuation, .. } => {
                let ResidentProcessConsultProviderRecordingAttempt {
                    reconcile,
                    rotate,
                    sign_observation,
                    sign_checkpoint,
                } = recording.expect("prepared recording turn owns fresh one-shot callbacks");
                continue_consumed_provider_recording(
                    *continuation,
                    input,
                    &mut *access.custody,
                    &mut *access.anchor,
                    reconcile,
                    rotate,
                    sign_observation,
                    sign_checkpoint,
                )
            }
            ProviderOperationRecordingFailure::ConcurrentHead { conflict } => {
                let ResidentProcessConsultProviderRecordingAttempt {
                    reconcile,
                    rotate,
                    sign_observation,
                    sign_checkpoint,
                } = recording.expect("concurrent-head turn owns fresh one-shot callbacks");
                let Some(recovered_head) = recovered_head else {
                    let reason = ProviderOperationRecordingFailure::ConcurrentHead { conflict };
                    return Err(ResidentProcessConsultProviderCompletionFailure::Recording(
                        Box::new(ResidentProcessConsultProviderRecordingFailure {
                            selector,
                            current,
                            coordinate,
                            continuation,
                            candidate,
                            dispatch,
                            work,
                            cancellation,
                            reason,
                        }),
                    ));
                };
                match conflict.recover(recovered_head) {
                    Ok(continuation) => continue_consumed_provider_recording(
                        continuation,
                        input,
                        &mut *access.custody,
                        &mut *access.anchor,
                        reconcile,
                        rotate,
                        sign_observation,
                        sign_checkpoint,
                    ),
                    Err(failure) => {
                        let reason = ProviderOperationRecordingFailure::ConcurrentHead {
                            conflict: Box::new(failure.into_conflict()),
                        };
                        return Err(ResidentProcessConsultProviderCompletionFailure::Recording(
                            Box::new(ResidentProcessConsultProviderRecordingFailure {
                                selector,
                                current,
                                coordinate,
                                continuation,
                                candidate,
                                dispatch,
                                work,
                                cancellation,
                                reason,
                            }),
                        ));
                    }
                }
            }
            ProviderOperationRecordingFailure::Custody { custody: pending } => {
                debug_assert!(recording.is_none());
                match pending.continue_custody(&mut *access.custody, &mut *access.anchor) {
                    Ok(finalization) => finalization.finish().map_err(|failure| {
                        ProviderOperationRecordingFailure::Finalization {
                            failure: Box::new(failure),
                        }
                    }),
                    Err(pending) => Err(ProviderOperationRecordingFailure::Custody {
                        custody: Box::new(pending),
                    }),
                }
            }
            ProviderOperationRecordingFailure::Finalization { failure } => {
                debug_assert!(recording.is_none());
                failure.into_finalization().finish().map_err(|failure| {
                    ProviderOperationRecordingFailure::Finalization {
                        failure: Box::new(failure),
                    }
                })
            }
        };
        let recorded = match recorded {
            Ok(recorded) => recorded,
            Err(reason) => {
                return Err(ResidentProcessConsultProviderCompletionFailure::Recording(
                    Box::new(ResidentProcessConsultProviderRecordingFailure {
                        selector,
                        current,
                        coordinate,
                        continuation,
                        candidate,
                        dispatch,
                        work,
                        cancellation,
                        reason,
                    }),
                ));
            }
        };
        let ready = recorded_completion_ready(
            selector,
            current,
            coordinate,
            continuation,
            candidate,
            dispatch,
            work,
            cancellation,
            recorded,
        );
        self.classify_consultation_provider_recorded(access, ready, recovery)
    }

    /// Continue durable unknown recording for one lost task without replay.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn continue_consultation_provider_lost_recording<'input, 'operation, 'call>(
        &mut self,
        access: &mut ResidentDeploymentOperationAccess<'input>,
        failure: ResidentProcessConsultProviderLostRecordingFailure<'operation, 'call>,
        history: OperationAttemptHistoryInput<'operation>,
        recovered_dispatch: &'operation RecoveredOperationObservation,
        input: &ProviderOperationInvokeInput<'_>,
        recovery: &mut ResidentProcessConsultProviderRecordedRecovery<'input, 'operation>,
        recording: Option<ResidentProcessConsultProviderRecordingAttempt<'input>>,
        checkpoint_signer: Option<ResidentProcessConsultProviderCheckpointSigner<'input>>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
        ResidentProcessConsultProviderCompletionFailure<'operation, 'call>,
    > {
        let ResidentProcessConsultProviderLostRecordingFailure { ready, reason } = failure;
        if access.store != &self.store
            || ready.selector != self.selector
            || ready.ready.store() != &self.store
        {
            return Err(
                ResidentProcessConsultProviderCompletionFailure::LostRecording(Box::new(
                    ResidentProcessConsultProviderLostRecordingFailure { ready, reason },
                )),
            );
        }
        let ResidentProcessConsultationCompletionReady { selector, ready } = *ready;
        let (current, poll) = ready.into_parts();
        let ResidentConsultationTaskOwnerPoll {
            coordinate,
            continuation,
            outcome,
        } = poll;
        let ResidentConsultationTaskPoll::Lost(lost) = outcome else {
            return Err(ResidentProcessConsultProviderCompletionFailure::Selection {
                ready: Box::new(ResidentProcessConsultationCompletionReady {
                    selector,
                    ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                        current,
                        ResidentConsultationTaskOwnerPoll {
                            coordinate,
                            continuation,
                            outcome,
                        },
                    )),
                }),
                reason: ResidentProcessHostSelectionError::ConsultationChildAssociation,
            });
        };
        let recorded = match reason {
            RecoveredProviderUnknownRecordingFailure::Preparation(_) => {
                debug_assert!(checkpoint_signer.is_none());
                let ResidentProcessConsultProviderRecordingAttempt {
                    reconcile,
                    rotate,
                    sign_observation,
                    sign_checkpoint,
                } = recording.expect("lost-recording turn owns fresh one-shot callbacks");
                record_recovered_provider_operation_unknown(
                    history,
                    recovered_dispatch,
                    input,
                    &mut *access.custody,
                    &mut *access.anchor,
                    reconcile,
                    rotate,
                    sign_observation,
                    sign_checkpoint,
                )
            }
            RecoveredProviderUnknownRecordingFailure::Prepared(pending) => {
                debug_assert!(recording.is_none());
                let sign_checkpoint = checkpoint_signer
                    .expect("prepared lost-recording turn owns one checkpoint signer");
                continue_prepared_recovered_provider_unknown(
                    *pending,
                    input.resolve_store,
                    &mut *access.custody,
                    &mut *access.anchor,
                    input.checkpoint_authority,
                    sign_checkpoint,
                )
            }
            RecoveredProviderUnknownRecordingFailure::Issued(pending) => {
                debug_assert!(recording.is_none());
                debug_assert!(checkpoint_signer.is_none());
                continue_issued_recovered_provider_unknown(
                    *pending,
                    input.resolve_store,
                    &mut *access.custody,
                    &mut *access.anchor,
                )
            }
            RecoveredProviderUnknownRecordingFailure::Custody(pending) => {
                debug_assert!(recording.is_none());
                debug_assert!(checkpoint_signer.is_none());
                match pending.continue_custody(&mut *access.custody, &mut *access.anchor) {
                    Ok(finalization) => finalization.finish().map_err(|failure| {
                        RecoveredProviderUnknownRecordingFailure::Finalization(Box::new(failure))
                    }),
                    Err(pending) => Err(RecoveredProviderUnknownRecordingFailure::Custody(
                        Box::new(pending),
                    )),
                }
            }
            RecoveredProviderUnknownRecordingFailure::Finalization(failure) => {
                debug_assert!(recording.is_none());
                debug_assert!(checkpoint_signer.is_none());
                failure.into_finalization().finish().map_err(|failure| {
                    RecoveredProviderUnknownRecordingFailure::Finalization(Box::new(failure))
                })
            }
        };
        if let Err(reason) = recorded {
            let ready = ResidentProcessConsultationCompletionReady {
                selector: self.selector.clone(),
                ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                    current,
                    ResidentConsultationTaskOwnerPoll {
                        coordinate,
                        continuation,
                        outcome: ResidentConsultationTaskPoll::Lost(lost),
                    },
                )),
            };
            return Err(
                ResidentProcessConsultProviderCompletionFailure::LostRecording(Box::new(
                    ResidentProcessConsultProviderLostRecordingFailure {
                        ready: Box::new(ready),
                        reason,
                    },
                )),
            );
        }
        let ready = ResidentProcessConsultationCompletionReady {
            selector: self.selector.clone(),
            ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                current,
                ResidentConsultationTaskOwnerPoll {
                    coordinate,
                    continuation,
                    outcome: ResidentConsultationTaskPoll::Lost(lost),
                },
            )),
        };
        self.classify_consultation_provider_recorded(access, ready, recovery)
    }

    /// Retry only the protected classification retained after recording.
    pub(super) fn continue_consultation_provider_classification<'input, 'operation, 'call>(
        &mut self,
        access: &mut ResidentDeploymentOperationAccess<'input>,
        failure: ResidentProcessConsultProviderClassificationFailure<'operation, 'call>,
        recovery: &mut ResidentProcessConsultProviderRecordedRecovery<'input, 'operation>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
        ResidentProcessConsultProviderCompletionFailure<'operation, 'call>,
    > {
        let ResidentProcessConsultProviderClassificationFailure { ready, .. } = failure;
        self.classify_consultation_provider_recorded(access, *ready, recovery)
    }

    /// Record and classify one task-owned provider loss.
    #[allow(clippy::too_many_arguments)]
    fn finalize_lost_consultation_provider_child<'input, 'operation, 'call>(
        &mut self,
        access: &mut ResidentDeploymentOperationAccess<'input>,
        ready: ResidentProcessConsultationCompletionReady<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
        history: OperationAttemptHistoryInput<'operation>,
        recovered_dispatch: &'operation RecoveredOperationObservation,
        lost_input: &ProviderOperationInvokeInput<'_>,
        recovery: &mut ResidentProcessConsultProviderRecordedRecovery<'input, 'operation>,
        recording: ResidentProcessConsultProviderRecordingAttempt<'input>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
        ResidentProcessConsultProviderCompletionFailure<'operation, 'call>,
    > {
        let ResidentProcessConsultationCompletionReady { selector, ready } = ready;
        if selector != self.selector || ready.store() != &self.store || access.store != &self.store
        {
            return Err(ResidentProcessConsultProviderCompletionFailure::Selection {
                ready: Box::new(ResidentProcessConsultationCompletionReady { selector, ready }),
                reason: ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation,
            });
        }
        let ResidentProcessConsultProviderRecordingAttempt {
            reconcile,
            rotate,
            sign_observation,
            sign_checkpoint,
        } = recording;
        let (current, poll) = ready.into_parts();
        let ResidentConsultationTaskOwnerPoll {
            coordinate,
            continuation,
            outcome,
        } = poll;
        let ResidentConsultationTaskPoll::Lost(lost) = outcome else {
            return Err(ResidentProcessConsultProviderCompletionFailure::Selection {
                ready: Box::new(ResidentProcessConsultationCompletionReady {
                    selector,
                    ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                        current,
                        ResidentConsultationTaskOwnerPoll {
                            coordinate,
                            continuation,
                            outcome,
                        },
                    )),
                }),
                reason: ResidentProcessHostSelectionError::ConsultationChildAssociation,
            });
        };
        if let Err(reason) = record_recovered_provider_operation_unknown(
            history,
            recovered_dispatch,
            lost_input,
            &mut *access.custody,
            &mut *access.anchor,
            reconcile,
            rotate,
            sign_observation,
            sign_checkpoint,
        ) {
            let ready = ResidentProcessConsultationCompletionReady {
                selector,
                ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                    current,
                    ResidentConsultationTaskOwnerPoll {
                        coordinate,
                        continuation,
                        outcome: ResidentConsultationTaskPoll::Lost(lost),
                    },
                )),
            };
            return Err(
                ResidentProcessConsultProviderCompletionFailure::LostRecording(Box::new(
                    ResidentProcessConsultProviderLostRecordingFailure {
                        ready: Box::new(ready),
                        reason,
                    },
                )),
            );
        }
        let ready = ResidentProcessConsultationCompletionReady {
            selector,
            ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                current,
                ResidentConsultationTaskOwnerPoll {
                    coordinate,
                    continuation,
                    outcome: ResidentConsultationTaskPoll::Lost(lost),
                },
            )),
        };
        self.classify_consultation_provider_recorded(access, ready, recovery)
    }

    /// Record and classify one task-owned provider completion.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn finalize_consultation_provider_child<'input, 'operation, 'call>(
        &mut self,
        access: &mut ResidentDeploymentOperationAccess<'input>,
        ready: ResidentProcessConsultationCompletionReady<
            ResidentProcessConsultProviderChildWork<'operation, 'call>,
            ResidentProcessConsultProviderChildOutput,
        >,
        history: OperationAttemptHistoryInput<'operation>,
        recovered_dispatch: &'operation RecoveredOperationObservation,
        lost_input: &ProviderOperationInvokeInput<'_>,
        evidence: &dyn ProviderInvocationEvidenceAdmission,
        recovery: &mut ResidentProcessConsultProviderRecordedRecovery<'input, 'operation>,
        recording: ResidentProcessConsultProviderRecordingAttempt<'input>,
    ) -> Result<
        ResidentProcessConsultProviderCompletionDisposition<'operation, 'call>,
        ResidentProcessConsultProviderCompletionFailure<'operation, 'call>,
    > {
        let ResidentProcessConsultationCompletionReady { selector, ready } = ready;
        if selector != self.selector || ready.store() != &self.store || access.store != &self.store
        {
            return Err(ResidentProcessConsultProviderCompletionFailure::Selection {
                ready: Box::new(ResidentProcessConsultationCompletionReady { selector, ready }),
                reason: ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation,
            });
        }
        let ResidentProcessConsultProviderRecordingAttempt {
            reconcile,
            rotate,
            sign_observation,
            sign_checkpoint,
        } = recording;
        let (current, poll) = ready.into_parts();
        let ResidentConsultationTaskOwnerPoll {
            coordinate,
            continuation,
            outcome,
        } = poll;
        let outcome = match outcome {
            ResidentConsultationTaskPoll::Completed {
                candidate,
                dispatch,
                mut work,
                output,
                cancellation,
            } => {
                if work.store != self.store
                    || work.dispatch_ref != dispatch.dispatch().record_ref
                    || work.lineage.consult_child().is_none()
                {
                    let poll = ResidentConsultationTaskOwnerPoll {
                        coordinate,
                        continuation,
                        outcome: ResidentConsultationTaskPoll::Completed {
                            candidate,
                            dispatch,
                            work,
                            output,
                            cancellation,
                        },
                    };
                    return Err(ResidentProcessConsultProviderCompletionFailure::Selection {
                        ready: Box::new(ResidentProcessConsultationCompletionReady {
                            selector,
                            ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                                current, poll,
                            )),
                        }),
                        reason: ResidentProcessHostSelectionError::ConsultationChildAssociation,
                    });
                }
                let ResidentProcessConsultProviderChildOutput::Mechanical(observation) = output
                else {
                    let poll = ResidentConsultationTaskOwnerPoll {
                        coordinate,
                        continuation,
                        outcome: ResidentConsultationTaskPoll::Completed {
                            candidate,
                            dispatch,
                            work,
                            output,
                            cancellation,
                        },
                    };
                    return Err(ResidentProcessConsultProviderCompletionFailure::Selection {
                        ready: Box::new(ResidentProcessConsultationCompletionReady {
                            selector,
                            ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                                current, poll,
                            )),
                        }),
                        reason: ResidentProcessHostSelectionError::ConsultationChildAssociation,
                    });
                };
                let Some(basis) = work.basis.as_ref() else {
                    let poll = ResidentConsultationTaskOwnerPoll {
                        coordinate,
                        continuation,
                        outcome: ResidentConsultationTaskPoll::Completed {
                            candidate,
                            dispatch,
                            work,
                            output: ResidentProcessConsultProviderChildOutput::Mechanical(
                                observation,
                            ),
                            cancellation,
                        },
                    };
                    return Err(ResidentProcessConsultProviderCompletionFailure::Selection {
                        ready: Box::new(ResidentProcessConsultationCompletionReady {
                            selector,
                            ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                                current, poll,
                            )),
                        }),
                        reason: ResidentProcessHostSelectionError::ConsultationChildAssociation,
                    });
                };
                let Some(input) = work.invocation.as_ref() else {
                    return Err(ResidentProcessConsultProviderCompletionFailure::Selection {
                        ready: Box::new(ResidentProcessConsultationCompletionReady {
                            selector,
                            ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                                current,
                                ResidentConsultationTaskOwnerPoll {
                                    coordinate,
                                    continuation,
                                    outcome: ResidentConsultationTaskPoll::Completed {
                                        candidate,
                                        dispatch,
                                        work,
                                        output:
                                            ResidentProcessConsultProviderChildOutput::Mechanical(
                                                observation,
                                            ),
                                        cancellation,
                                    },
                                },
                            )),
                        }),
                        reason: ResidentProcessHostSelectionError::ConsultationChildAssociation,
                    });
                };
                let (provider_continuation, admission) = match basis {
                    ResidentProcessConsultProviderCallBasis::Fresh {
                        continuation,
                        admission,
                        ..
                    } => (continuation, *admission),
                    ResidentProcessConsultProviderCallBasis::Recovered {
                        continuation,
                        admission,
                    } => (continuation, *admission),
                };
                let recovered = match prepare_recovered_provider_completion(
                    provider_continuation,
                    &work.request,
                    history,
                    recovered_dispatch,
                    admission,
                    &mut *access.custody,
                    &mut *access.anchor,
                    input.maximum_record_bytes,
                ) {
                    Ok(recovered) => recovered,
                    Err(reason) => {
                        let poll = ResidentConsultationTaskOwnerPoll {
                            coordinate,
                            continuation,
                            outcome: ResidentConsultationTaskPoll::Completed {
                                candidate,
                                dispatch,
                                work,
                                output: ResidentProcessConsultProviderChildOutput::Mechanical(
                                    observation,
                                ),
                                cancellation,
                            },
                        };
                        return Err(ResidentProcessConsultProviderCompletionFailure::Recovery {
                            ready: Box::new(ResidentProcessConsultationCompletionReady {
                                selector,
                                ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                                    current, poll,
                                )),
                            }),
                            reason,
                        });
                    }
                };
                let basis = work
                    .basis
                    .take()
                    .expect("validated provider completion basis remains owned");
                let input = work
                    .invocation
                    .as_ref()
                    .expect("validated provider invocation remains owned");
                let (provider_continuation, admission) = match basis {
                    ResidentProcessConsultProviderCallBasis::Fresh {
                        basis,
                        continuation,
                        admission,
                    } => {
                        drop(basis);
                        (continuation, admission)
                    }
                    ResidentProcessConsultProviderCallBasis::Recovered {
                        continuation,
                        admission,
                    } => (continuation, admission),
                };
                let recorded = match record_recovered_provider_operation_completion(
                    provider_continuation,
                    &work.request,
                    observation,
                    recovered,
                    admission,
                    input,
                    evidence,
                    &mut *access.custody,
                    &mut *access.anchor,
                    reconcile,
                    rotate,
                    sign_observation,
                    sign_checkpoint,
                ) {
                    Ok(recorded) => recorded,
                    Err(reason) => {
                        return Err(ResidentProcessConsultProviderCompletionFailure::Recording(
                            Box::new(ResidentProcessConsultProviderRecordingFailure {
                                selector,
                                current,
                                coordinate,
                                continuation,
                                candidate,
                                dispatch,
                                work,
                                cancellation,
                                reason,
                            }),
                        ));
                    }
                };
                ResidentConsultationTaskPoll::Completed {
                    candidate,
                    dispatch,
                    work,
                    output: ResidentProcessConsultProviderChildOutput::Recorded(recorded),
                    cancellation,
                }
            }
            ResidentConsultationTaskPoll::Lost(lost) => {
                let recorded = record_recovered_provider_operation_unknown(
                    history,
                    recovered_dispatch,
                    lost_input,
                    &mut *access.custody,
                    &mut *access.anchor,
                    reconcile,
                    rotate,
                    sign_observation,
                    sign_checkpoint,
                );
                if let Err(reason) = recorded {
                    let ready = ResidentProcessConsultationCompletionReady {
                        selector,
                        ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                            current,
                            ResidentConsultationTaskOwnerPoll {
                                coordinate,
                                continuation,
                                outcome: ResidentConsultationTaskPoll::Lost(lost),
                            },
                        )),
                    };
                    return Err(
                        ResidentProcessConsultProviderCompletionFailure::LostRecording(Box::new(
                            ResidentProcessConsultProviderLostRecordingFailure {
                                ready: Box::new(ready),
                                reason,
                            },
                        )),
                    );
                }
                ResidentConsultationTaskPoll::Lost(lost)
            }
            ResidentConsultationTaskPoll::Stale(completion) => {
                let poll = ResidentConsultationTaskOwnerPoll {
                    coordinate,
                    continuation,
                    outcome: ResidentConsultationTaskPoll::Stale(completion),
                };
                return Err(ResidentProcessConsultProviderCompletionFailure::Selection {
                    ready: Box::new(ResidentProcessConsultationCompletionReady {
                        selector,
                        ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                            current, poll,
                        )),
                    }),
                    reason: ResidentProcessHostSelectionError::ConsultationChildAssociation,
                });
            }
        };

        let ready = ResidentProcessConsultationCompletionReady {
            selector,
            ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                current,
                ResidentConsultationTaskOwnerPoll {
                    coordinate,
                    continuation,
                    outcome,
                },
            )),
        };
        self.classify_consultation_provider_recorded(access, ready, recovery)
    }
}

#[allow(clippy::too_many_arguments)]
fn recorded_completion_ready<'operation, 'call>(
    selector: ResidentDeploymentAuthoritySelector,
    current: RecoveredConsultationCollection,
    coordinate: crate::resident::consultation::ResidentConsultationBatchCoordinate,
    continuation: crate::resident::consultation::service::task::ResidentConsultationContinuationToken,
    candidate: ConsultationChildDispatchCandidate,
    dispatch: crate::resident::task_owner::ResidentOperationTaskKey,
    work: ResidentProcessConsultProviderChildWork<'operation, 'call>,
    cancellation: ResidentOperationTaskCancellationState,
    recorded: ProviderOperationInvokeResult,
) -> ResidentProcessConsultationCompletionReady<
    ResidentProcessConsultProviderChildWork<'operation, 'call>,
    ResidentProcessConsultProviderChildOutput,
> {
    ResidentProcessConsultationCompletionReady {
        selector,
        ready: Box::new(ResidentConsultationCompletionReady::from_parts(
            current,
            ResidentConsultationTaskOwnerPoll {
                coordinate,
                continuation,
                outcome: ResidentConsultationTaskPoll::Completed {
                    candidate,
                    dispatch,
                    work,
                    output: ResidentProcessConsultProviderChildOutput::Recorded(recorded),
                    cancellation,
                },
            },
        )),
    }
}

fn retained_operation_attempt_history<'operation>(
    history: &OperationAttemptHistoryInput<'operation>,
) -> OperationAttemptHistoryInput<'operation> {
    OperationAttemptHistoryInput {
        operation: history.operation,
        request: history.request,
        session: history.session,
        lease: history.lease,
        generation: history.generation,
        dispatched: history.dispatched,
        selection: history.selection,
        maximum_selection_bytes: history.maximum_selection_bytes,
    }
}

impl<WriterGuard> ResidentProcessHost<'_, WriterGuard> {
    /// Retain one exact provider-backed Consult child without calling it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_consultation_child_provider_work_in<
        'operation,
        'call,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        application: &ResidentApplication<'_>,
        recovered: &'operation ResidentProcessRecoveredOperationResolution,
        mut access: ResidentDeploymentOperationAccess<'_>,
        dispatch: ProviderOperationDispatchInput<'operation, 'call>,
        invocation: ProviderOperationInvokeInput<'call>,
        authority: &mut Authority,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessConsultProviderChildWork<'operation, 'call>,
        ResidentProcessConsultProviderChildRetentionFailure<'operation>,
    >
    where
        'operation: 'call,
        Authority: super::super::AmbientSessionAuthorityPort,
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
        let Some(lineage) = recovered.start.lineage.consult_child() else {
            return Err(
                ResidentProcessConsultProviderChildRetentionFailure::Unopened(
                    ResidentProcessConsultProviderUnopenedFailure::Selection(
                        ResidentProcessHostSelectionError::ConsultationChildAssociation,
                    ),
                ),
            );
        };
        if !matches!(
            recovered.phase,
            ResidentProcessOperationResolutionPhase::Resolving
                | ResidentProcessOperationResolutionPhase::ContextBound
        ) {
            return Err(
                ResidentProcessConsultProviderChildRetentionFailure::Unopened(
                    ResidentProcessConsultProviderUnopenedFailure::Admission(
                        OperationAttemptError::Association,
                    ),
                ),
            );
        }
        let context = &dispatch.operation.context;
        let provider = dispatch.operation.invocation.provider_binding();
        let provider_ref = ProviderBindingRefV1 {
            provider_binding_id: provider.id.clone(),
            provider_binding_identity: provider.binding_identity.clone(),
        };
        let predecessor = recovered.start.control.observation().selected_head();
        let context_matches = match (lineage.context(), context.context_binding) {
            (None, None) => true,
            (Some(expected), Some(binding)) => {
                let (reference, observation) = binding.observation();
                matches!(
                    observation.state(),
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::ContextBound { evidence },
                    } if evidence == expected
                ) && reference == predecessor.0
                    && observation.canonical_operation_observation_sha256()
                        == predecessor.1.canonical_operation_observation_sha256()
            }
            _ => false,
        };
        if application.store() != access.store
            || !recovered
                .start
                .lineage
                .admits(context.operation.handle().0, context.operation.handle().1)
            || lineage.provider_binding() != Some(&provider_ref)
            || lineage.participant() != &provider.participant
            || context.operation.handle()
                != recovered.start.control.operation().operation().handle()
            || context.request != recovered.start.control.operation().request()
            || context.observation.predecessor != Some(predecessor)
            || dispatch.checkpoint != recovered.start.control.observation().current_checkpoint()
            || !context_matches
        {
            return Err(
                ResidentProcessConsultProviderChildRetentionFailure::Unopened(
                    ResidentProcessConsultProviderUnopenedFailure::Selection(
                        ResidentProcessHostSelectionError::ConsultationChildAssociation,
                    ),
                ),
            );
        }
        Self::validate_operation_resolution_in(
            application,
            &recovered.start.selector,
            recovered,
            &mut access,
            context.session,
            context.current,
            dispatch.signing,
            invocation.maximum_record_bytes,
            true,
            authority,
        )
        .map_err(|reason| {
            ResidentProcessConsultProviderChildRetentionFailure::Unopened(
                ResidentProcessConsultProviderUnopenedFailure::Selection(match reason {
                    ResidentProcessOperationResolutionFailure::Selection(reason) => reason,
                    ResidentProcessOperationResolutionFailure::Admission(_) => {
                        ResidentProcessHostSelectionError::OperationResolutionOperationAuthority
                    }
                }),
            )
        })?;
        validate_provider_launch_associations(
            &recovered.start.selector,
            application.store(),
            &dispatch,
            &invocation,
        )
        .map_err(|reason| {
            ResidentProcessConsultProviderChildRetentionFailure::Unopened(
                ResidentProcessConsultProviderUnopenedFailure::Selection(reason),
            )
        })?;
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let provider_admission = dispatch.operation.result_admission;
        let retained = retain_provider_operation_dispatch(
            dispatch,
            authority,
            custody,
            anchor,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
        .map_err(|reason| match reason {
            ProviderOperationRetentionFailure::Admission(reason) => {
                ResidentProcessConsultProviderChildRetentionFailure::Unopened(
                    ResidentProcessConsultProviderUnopenedFailure::Admission(reason),
                )
            }
            reason => ResidentProcessConsultProviderChildRetentionFailure::Crossed(
                ResidentProcessConsultProviderCrossedFailure::Provider(reason),
            ),
        })?;
        let (dispatch_ref, dispatch) = retained.observation();
        let dispatch_ref = dispatch_ref.clone();
        let dispatch = dispatch.clone();
        let call =
            prepare_provider_operation_call(retained, &invocation, authority, custody, anchor)
                .map_err(|reason| {
                    ResidentProcessConsultProviderChildRetentionFailure::Crossed(
                        ResidentProcessConsultProviderCrossedFailure::Call(reason),
                    )
                })?;
        let (call_store, call_dispatch, continuation, basis, request) =
            call.into_task_parts().map_err(|parts| {
                let (call, reason) = parts;
                ResidentProcessConsultProviderChildRetentionFailure::Crossed(
                    ResidentProcessConsultProviderCrossedFailure::CallIdentity { call, reason },
                )
            })?;
        let dispatch_sha256 =
            ArtifactDigest::parse(dispatch.canonical_operation_observation_sha256().as_str())
                .map_err(|_| {
                    ResidentProcessConsultProviderChildRetentionFailure::Crossed(
                        ResidentProcessConsultProviderCrossedFailure::Selection(
                            ResidentProcessHostSelectionError::ConsultationChildAssociation,
                        ),
                    )
                })?;
        if call_store != *application.store()
            || call_dispatch.record_ref != dispatch_ref
            || call_dispatch.record_sha256 != dispatch_sha256
        {
            return Err(
                ResidentProcessConsultProviderChildRetentionFailure::Crossed(
                    ResidentProcessConsultProviderCrossedFailure::Selection(
                        ResidentProcessHostSelectionError::ConsultationChildAssociation,
                    ),
                ),
            );
        }
        Ok(ResidentProcessConsultProviderChildWork {
            store: application.store().clone(),
            lineage: recovered.start.lineage.clone(),
            dispatch_ref,
            dispatch,
            basis: Some(ResidentProcessConsultProviderCallBasis::Fresh {
                basis: Box::new(basis),
                continuation,
                admission: provider_admission,
            }),
            request,
            invocation: Some(invocation),
        })
    }

    /// Execute one task-owned provider child exactly once.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn execute_consultation_child_provider_work<'operation, 'call, Provider>(
        work: ResidentProcessConsultProviderChildWork<'operation, 'call>,
        provider: &Provider,
    ) -> (
        ResidentProcessConsultProviderChildWork<'operation, 'call>,
        ResidentProcessConsultProviderChildOutput,
    )
    where
        Provider: ProviderInvocationPort,
    {
        let completed = invoke_owned_provider_invocation(&work.request, provider);
        (
            work,
            ResidentProcessConsultProviderChildOutput::Mechanical(completed),
        )
    }
}
