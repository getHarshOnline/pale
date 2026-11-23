//! Private cold-restart classification for an existing operation start.
//!
//! Recovery in this module is read-only. It selects one saved handle, admits
//! one exact current head, and reports the only continuation that state permits.
//! It does not call a provider, run a native action, create
//! a task, reserve a route, sign a record, or mutate custody.

use super::*;
use crate::resident::operation_custody::{
    read_operation_start_for_request, OperationRecoveryError,
};
use crate::resident::operation_dispatch::hook_response::recovery::{
    recover_native_hook_response, RecoveredNativeHookResponse,
};
use crate::resident::operation_dispatch::OperationAttemptOwner;
use crate::resident::operation_dispatch::{
    has_indexed_a2a_send_preparation, probe_a2a_send_custody_for_known_task,
    recover_a2a_send_preparation_under_cancellation_request,
    verify_recovered_operation_head_custody, A2aSendCustodyProbe, A2aSendRecoveryAdmission,
    A2aSendRecoveryError, A2aSendRecoveryInput, A2aSendRecoveryLimits,
};
use crate::resident::{
    checkpoint_profile::{a2a_evidence_key, a2a_evidence_leaf},
    discover_a2a_task_bindings, recover_child_operation_control, recover_operation_control,
    A2aTaskObservationSelection, A2aTaskRecoveryAdmission, A2aTaskRecoveryError,
    A2aTaskRecoveryInput, A2aTaskRecoveryLimits, ChildOperationControlRecoveryInput,
    ConsultationChildDispatchBatch, ConsultationRosterSlot, ConsultationSlotId,
    LocalChildOperationHandleRecoveryInput, LocalChildOperationRecoveryContext,
    LocalOperationHandleRecoveryInput, LocalOperationRecoveryContext,
    OperationCheckpointAncestryLimits, OperationControlRecoveryAdmission,
    OperationControlRecoveryError, OperationControlRecoveryInput,
    OperationObservationRecoveryAdmission, OperationObservationRecoveryLimits,
    OperationObservationSelection, RecoveredA2aTaskHistory, RecoveredOperationControl,
};
use pale_core::records::{
    A2aJsonRpcRequestIdV1, CapabilityProviderBinding, CommonRequestEnvelopeV1,
    ImmutableRecordReferenceV1, OperationActivePhaseV1, OperationHandleV1, OperationLeaseV1,
    OperationObservationStateV1, ParticipantRefV1, ProviderBindingRefV1, ResidentRecordRefV1,
};
use pale_ports::{
    A2aCancelTaskRequestEvidencePort, AccumulatorMembershipLeaf, AccumulatorMembershipLimits,
    AuthorityStoreCoordinate, ControlRecord, ControlRecordCoordinate, OperationA2aEvidence,
};

/// Check deployment and historical authority before either protected reads or anchor repair.
pub(super) fn verify_restart_inputs(
    application: &ResidentApplication<'_>,
    existing: &ResidentProcessExistingOperationStart,
    association: &ResidentOperationControlAssociation<'_>,
    access_store: &AuthorityStoreCoordinate,
    input: &ResidentProcessOperationRestartInput<'_>,
) -> Result<(), ResidentProcessOperationRestartError> {
    if input.maximum_selection_bytes == 0 {
        return Err(ResidentProcessOperationRestartError::Bounds);
    }
    let selector = &existing.selector;
    if !selector.matches_signing_authority(association.current_request_authority) {
        return Err(ResidentProcessHostSelectionError::CurrentRequestAuthority.into());
    }
    if !selector.matches_signing_authority(association.historical_handle_authority) {
        return Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority.into());
    }
    let selected_store = application.store();
    if association.recovery_store != selected_store {
        return Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation.into());
    }
    if association.checkpoint_authority.principal_id != &selector.principal_id
        || association.checkpoint_authority.control_store_id != selected_store.store_id()
        || association.checkpoint_authority.control_incarnation_id
            != selected_store.incarnation_id()
    {
        return Err(ResidentProcessHostSelectionError::CheckpointAuthorityAssociation.into());
    }
    if access_store != selected_store || existing.evidence.store() != selected_store {
        return Err(ResidentProcessHostSelectionError::AccessStoreAssociation.into());
    }
    if input.operation.store != selected_store
        || input.operation.handle_signing_authority != association.historical_handle_authority
        || !same_checkpoint_authority(
            input.operation.checkpoint_authority,
            association.checkpoint_authority,
        )
    {
        return Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation.into());
    }

    Ok(())
}

fn reborrow_operation_recovery_admission<'short>(
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

/// Independent ceilings for one admitted operation-head read.
#[derive(Clone, Copy)]
pub(super) struct ResidentProcessRestartReadLimits {
    pub(super) observation: OperationObservationRecoveryLimits,
    pub(super) checkpoints: OperationCheckpointAncestryLimits,
    pub(super) membership: AccumulatorMembershipLimits,
}

/// Exact protected-recovery inputs for one existing operation start.
pub(super) struct ResidentProcessOperationRestartInput<'a> {
    /// Only the cold sweep may use this bounded anchor-repair allowance.
    /// Point-addressed classification remains read-only even when supplied.
    pub(super) committed_observation_limits: Option<pale_ports::AccumulatorPreparationLimits>,
    pub(super) operation: LocalOperationRecoveryContext<'a>,
    pub(super) maximum_parent_depth: usize,
    pub(super) current_head: ResidentProcessRestartReadLimits,
    pub(super) maximum_selection_bytes: usize,
    /// Optional bounded recovery needed to continue one already-known A2A Task.
    pub(super) a2a_task: Option<ResidentProcessA2aTaskRestartInput<'a>>,
    /// Optional evidence readers for an A2A dispatch with a cancellation request.
    pub(super) a2a_cancel_task: Option<ResidentProcessA2aCancelTaskRestartInput<'a>>,
}

/// Read-only inputs for selecting one exact Task already bound to a Delegate.
pub(super) struct ResidentProcessA2aTaskRestartInput<'a> {
    pub(super) maximum_bindings: usize,
    pub(super) maximum_binding_bytes: usize,
    pub(super) send: A2aSendRecoveryLimits,
    pub(super) task: A2aTaskRecoveryLimits,
    pub(super) remote: A2aTaskRecoveryAdmission<'a>,
}

/// Read-only inputs needed to distinguish one retained A2A `CancelTask` request.
pub(super) struct ResidentProcessA2aCancelTaskRestartInput<'a> {
    pub(super) protocol: &'a dyn A2aCancelTaskRequestEvidencePort,
    pub(super) maximum_request_bytes: NonZeroUsize,
    pub(super) maximum_bindings: usize,
    pub(super) maximum_binding_bytes: usize,
    pub(super) send: A2aSendRecoveryLimits,
    pub(super) task: A2aTaskRecoveryLimits,
    pub(super) remote: A2aTaskRecoveryAdmission<'a>,
}

/// How a dispatched attempt was selected from the admitted current history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResidentProcessAttemptObservation {
    /// The current head itself is the dispatched observation.
    CurrentHead,
    /// The current head directly requests cancellation of this dispatched attempt.
    CancellationPredecessor,
}

/// Recovered immutable operation state shared by every restart disposition.
pub(super) struct ResidentProcessRecoveredOperationStart {
    pub(super) selector: ResidentDeploymentAuthoritySelector,
    pub(super) control: RecoveredOperationControl,
    pub(super) generation_digest: pale_core::records::CompiledGenerationDigest,
    pub(super) lineage: ResidentProcessOperationLineage,
}

/// Protected lineage admitted before an operation may use a process-host path.
///
/// Root admission is reserved for an unparented Invoke or Consult. Consult-child admission
/// binds the exact roster membership. Pairwise Delegate admission binds only the
/// exact protected parent and child handles. No variant grants current execution
/// authority; every continuation still checks current session and operation authority.
#[derive(Clone, PartialEq, Eq)]
pub(super) enum ResidentProcessOperationLineage {
    Root,
    ConsultChild(Box<ResidentProcessConsultChildLineage>),
    PairwiseDelegate(Box<ResidentProcessPairwiseDelegateLineage>),
}

/// Exact parent-child association retained for one pairwise Delegate.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct ResidentProcessPairwiseDelegateLineage {
    parent: ImmutableRecordReferenceV1,
    child: ImmutableRecordReferenceV1,
}

/// Exact Consult roster membership retained with one recovered child Invoke.
///
/// The provider reference includes its immutable digest. The participant and
/// context remain separate coordinates and must match the admitted generation
/// and protected operation history before a handler can run.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct ResidentProcessConsultChildLineage {
    parent: ImmutableRecordReferenceV1,
    collection_frontier: ImmutableRecordReferenceV1,
    slot_id: ConsultationSlotId,
    child: ImmutableRecordReferenceV1,
    participant: ParticipantRefV1,
    provider_binding: Option<ProviderBindingRefV1>,
    context: Option<ImmutableRecordReferenceV1>,
}

impl ResidentProcessOperationLineage {
    /// Return whether this proof admits the selected operation handle.
    pub(super) fn admits(
        &self,
        handle_ref: &ResidentRecordRefV1,
        handle: &OperationHandleV1,
    ) -> bool {
        match self {
            Self::Root => {
                matches!(
                    handle.operation_class(),
                    OperationClassV1::Invoke | OperationClassV1::Consult
                ) && handle.parent_operation().is_none()
            }
            Self::ConsultChild(lineage) => {
                let Some(parent) = handle.parent_operation() else {
                    return false;
                };
                matches!(
                    handle.operation_class(),
                    OperationClassV1::Invoke | OperationClassV1::Consult
                ) && lineage.child.record_ref == *handle_ref
                    && lineage.child.record_sha256.as_str()
                        == handle.canonical_operation_handle_sha256().as_str()
                    && lineage.parent.record_ref == parent.operation_handle_ref
                    && lineage.parent.record_sha256.as_str()
                        == parent.operation_handle_sha256.as_str()
            }
            Self::PairwiseDelegate(lineage) => {
                let Some(parent) = handle.parent_operation() else {
                    return false;
                };
                handle.operation_class() == &OperationClassV1::Delegate
                    && lineage.child.record_ref == *handle_ref
                    && lineage.child.record_sha256.as_str()
                        == handle.canonical_operation_handle_sha256().as_str()
                    && lineage.parent.record_ref == parent.operation_handle_ref
                    && lineage.parent.record_sha256.as_str()
                        == parent.operation_handle_sha256.as_str()
            }
        }
    }

    /// Return whether one retained attempt belongs to this exact lineage.
    pub(super) fn admits_provider(&self, provider: Option<&ProviderBindingRefV1>) -> bool {
        match self {
            Self::Root | Self::PairwiseDelegate(_) => true,
            Self::ConsultChild(lineage) => lineage.provider_binding.as_ref() == provider,
        }
    }

    /// Return whether the admitted lineage retains a parent operation.
    pub(super) const fn has_parent_operation(&self) -> bool {
        match self {
            Self::Root => false,
            Self::ConsultChild(_) | Self::PairwiseDelegate(_) => true,
        }
    }

    /// Borrow the Consult-child coordinates, when this is Consult-child lineage.
    pub(super) const fn consult_child(&self) -> Option<&ResidentProcessConsultChildLineage> {
        match self {
            Self::Root | Self::PairwiseDelegate(_) => None,
            Self::ConsultChild(lineage) => Some(lineage),
        }
    }
}

impl ResidentProcessPairwiseDelegateLineage {
    pub(super) const fn parent(&self) -> &ImmutableRecordReferenceV1 {
        &self.parent
    }

    pub(super) const fn child(&self) -> &ImmutableRecordReferenceV1 {
        &self.child
    }
}

impl ResidentProcessConsultChildLineage {
    pub(super) const fn parent(&self) -> &ImmutableRecordReferenceV1 {
        &self.parent
    }

    pub(super) const fn collection_frontier(&self) -> &ImmutableRecordReferenceV1 {
        &self.collection_frontier
    }

    pub(super) const fn slot_id(&self) -> &ConsultationSlotId {
        &self.slot_id
    }

    pub(super) const fn child(&self) -> &ImmutableRecordReferenceV1 {
        &self.child
    }

    pub(super) const fn participant(&self) -> &ParticipantRefV1 {
        &self.participant
    }

    pub(super) const fn provider_binding(&self) -> Option<&ProviderBindingRefV1> {
        self.provider_binding.as_ref()
    }

    pub(super) const fn context(&self) -> Option<&ImmutableRecordReferenceV1> {
        self.context.as_ref()
    }
}

pub(super) enum ResidentProcessRestartLineageInput<'a> {
    Root,
    ConsultChild {
        parent: &'a ResidentProcessRecoveredOperationStart,
        collection: &'a RecoveredConsultationCollection,
        planned: &'a ConsultationChildDispatchBatch,
        candidate: &'a ConsultationChildDispatchCandidate,
        observed_at: UnixTimeV1,
    },
    /// Consult child whose exact parent, current roster, and slot were recovered
    /// from durable custody after process restart.
    RecoveredConsultChild {
        parent: &'a RecoveredOperationControl,
        collection: &'a RecoveredConsultationCollection,
        slot: &'a ConsultationRosterSlot,
    },
    PairwiseDelegate {
        parent: &'a ResidentProcessRecoveredOperationStart,
    },
    /// Pairwise Delegate whose exact immediate parent was already recovered as
    /// part of the protected child's bounded handle chain.
    RecoveredPairwiseDelegate,
}

/// Lineage evidence and any control already protected while proving it.
struct ResidentProcessRestartQualification<'a> {
    lineage: ResidentProcessRestartLineageInput<'a>,
    protected: Option<RecoveredOperationControl>,
}

/// One exact start after protected control recovery and start-evidence read-back.
///
/// The admitted handle is the sole source for its class and parent coordinate.
/// This carrier grants no lineage or continuation authority.
pub(super) struct ResidentProcessProtectedOperationStart {
    existing: ResidentProcessExistingOperationStart,
    control: RecoveredOperationControl,
}

/// Exact recovered parent and roster evidence for one protected Consult child.
pub(super) struct ResidentProcessRecoveredConsultChildLineage<'a> {
    pub(super) parent: &'a RecoveredOperationControl,
    pub(super) collection: &'a RecoveredConsultationCollection,
    pub(super) slot: &'a ConsultationRosterSlot,
}

impl ResidentProcessProtectedOperationStart {
    /// Borrow the already-protected operation without exposing mutable custody.
    pub(super) fn operation(&self) -> &crate::resident::IssuedLocalOperation {
        self.control.operation().operation()
    }

    /// Borrow the operation class from the admitted, signature-checked handle.
    pub(super) fn operation_class(&self) -> &OperationClassV1 {
        self.control
            .operation()
            .operation()
            .handle()
            .1
            .operation_class()
    }

    /// Borrow the parent coordinate from the admitted, signature-checked handle.
    pub(super) fn parent_operation(
        &self,
    ) -> Option<&pale_core::records::ParentOperationReferenceV1> {
        self.control
            .operation()
            .operation()
            .handle()
            .1
            .parent_operation()
            .as_ref()
    }

    /// Borrow the selected deployment without exposing the protected control.
    pub(super) const fn selector(&self) -> &ResidentDeploymentAuthoritySelector {
        &self.existing.selector
    }
}

/// One exact retained attempt whose owner was classified without execution.
pub(super) struct ResidentProcessRecoveredAttempt {
    pub(super) start: ResidentProcessRecoveredOperationStart,
    pub(super) dispatch: ResidentProcessAttemptObservation,
    pub(super) selection: ControlRecord,
}

/// One protected native response retained only for recovery and reconciliation.
pub(super) struct ResidentProcessRecoveredNativeHookResponse {
    pub(super) start: ResidentProcessRecoveredOperationStart,
    pub(super) response: RecoveredNativeHookResponse,
}

/// One exact retained preparation whose semantic owner can finish dispatch.
pub(super) struct ResidentProcessRecoveredPreparedAttempt {
    pub(super) start: ResidentProcessRecoveredOperationStart,
    pub(super) selection: ControlRecord,
}

/// One exact current `ProviderResolved` operation awaiting its first attempt.
pub(super) struct ResidentProcessRecoveredProviderResolution {
    pub(super) start: ResidentProcessRecoveredOperationStart,
    pub(super) provider_binding: ProviderBindingRefV1,
}

/// One A2A `CancelTask` request that can only continue through recovery.
pub(super) struct ResidentProcessRecoveredA2aCancelTaskRequest {
    pub(super) attempt: ResidentProcessRecoveredAttempt,
    pub(super) task: Box<RecoveredA2aTaskHistory>,
    pub(super) request_evidence: OperationA2aEvidence,
    pub(super) request_id: A2aJsonRpcRequestIdV1,
}

/// One dispatched pairwise Delegate with one exact admitted remote Task.
pub(super) struct ResidentProcessRecoveredA2aTask {
    pub(super) attempt: ResidentProcessRecoveredAttempt,
    pub(super) task: Box<RecoveredA2aTaskHistory>,
}

/// One retained local cancellation request whose predecessor never crossed dispatch.
pub(super) struct ResidentProcessRecoveredLocalCancellation {
    pub(super) start: ResidentProcessRecoveredOperationStart,
    pub(super) request: ImmutableRecordReferenceV1,
}

/// One current Invoke or pairwise Delegate before implementation selection or dispatch.
pub(in crate::resident) struct ResidentProcessRecoveredOperationResolution {
    pub(super) start: ResidentProcessRecoveredOperationStart,
    pub(super) phase: ResidentProcessOperationResolutionPhase,
}

/// Exact pre-attempt phase admitted from the protected observation history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum ResidentProcessOperationResolutionPhase {
    /// The operation has not yet retained required context.
    Resolving,
    /// The operation already retains its exact context witness.
    ContextBound,
}

/// Closed read-only result of classifying one existing operation start.
pub(super) enum ResidentProcessOperationRestartDisposition {
    /// A conclusive terminal observation already exists.
    Terminal(Box<ResidentProcessRecoveredOperationStart>),
    /// Only recovery, reconciliation, or later evidence can advance this state.
    RecoveryOnly(Box<ResidentProcessRecoveredOperationStart>),
    /// A saved hook response cannot enter action/provider recovery or be resent.
    NativeHookResponse(Box<ResidentProcessRecoveredNativeHookResponse>),
    /// A retained local cancellation request can complete without a carrier call.
    LocalCancellationRequested(Box<ResidentProcessRecoveredLocalCancellation>),
    /// A current Invoke or pairwise Delegate can continue only through pre-attempt resolution.
    OperationResolution(Box<ResidentProcessRecoveredOperationResolution>),
    /// A built-in native preparation can recover only as a durable dispatch.
    NativePrepared(Box<ResidentProcessRecoveredPreparedAttempt>),
    /// An external-provider preparation can recover only as a durable dispatch.
    ProviderPrepared(Box<ResidentProcessRecoveredPreparedAttempt>),
    /// A current provider resolution can allocate and retain its first attempt.
    ProviderResolved(Box<ResidentProcessRecoveredProviderResolution>),
    /// The consultation owner retains this operation.
    Consult(Box<ResidentProcessRecoveredOperationStart>),
    /// A built-in native attempt crossed dispatch.
    Native(Box<ResidentProcessRecoveredAttempt>),
    /// An external provider attempt crossed dispatch.
    Provider(Box<ResidentProcessRecoveredAttempt>),
    /// An external Delegate can advance only by proving an exact retained A2A send.
    A2aDelegateSendRecoveryOnly(Box<ResidentProcessRecoveredAttempt>),
    /// An external Delegate can continue the one exact retained remote Task.
    A2aTask(Box<ResidentProcessRecoveredA2aTask>),
    /// A retained A2A `CancelTask` request was qualified without rebuilding a call.
    A2aCancelTaskRequestRecoveryOnly(Box<ResidentProcessRecoveredA2aCancelTaskRequest>),
}

impl std::fmt::Debug for ResidentProcessOperationRestartDisposition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Terminal(_) => "Terminal",
            Self::RecoveryOnly(_) => "RecoveryOnly",
            Self::NativeHookResponse(_) => "NativeHookResponse",
            Self::LocalCancellationRequested(_) => "LocalCancellationRequested",
            Self::OperationResolution(_) => "OperationResolution",
            Self::NativePrepared(_) => "NativePrepared",
            Self::ProviderPrepared(_) => "ProviderPrepared",
            Self::ProviderResolved(_) => "ProviderResolved",
            Self::Consult(_) => "Consult",
            Self::Native(_) => "Native",
            Self::Provider(_) => "Provider",
            Self::A2aDelegateSendRecoveryOnly(_) => "A2aDelegateSendRecoveryOnly",
            Self::A2aTask(_) => "A2aTask",
            Self::A2aCancelTaskRequestRecoveryOnly(_) => "A2aCancelTaskRequestRecoveryOnly",
        };
        if let Self::NativeHookResponse(value) = self {
            return formatter
                .debug_struct("ResidentProcessOperationRestartDisposition")
                .field("phase", &phase)
                .field("native_response", &value.response)
                .finish_non_exhaustive();
        }
        formatter
            .debug_struct("ResidentProcessOperationRestartDisposition")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Failure before a closed restart disposition exists.
#[derive(Debug, thiserror::Error)]
pub(super) enum ResidentProcessOperationRestartError {
    /// Process, deployment, authority, store, or checkpoint selection differed.
    #[error(transparent)]
    Selection(#[from] ResidentProcessHostSelectionError),
    /// A required finite selection ceiling was zero.
    #[error("existing operation restart bounds are invalid")]
    Bounds,
    /// Protected operation-control recovery failed.
    #[error(transparent)]
    Recovery(#[from] OperationControlRecoveryError),
    /// The original four start records changed after the duplicate lookup.
    #[error("existing operation start evidence changed during recovery")]
    StartEvidence,
    /// Re-reading the original start evidence failed.
    #[error(transparent)]
    StartEvidenceRead(#[from] OperationRecoveryError),
    /// The exact generation retained by the handle is unavailable.
    #[error("existing operation compiled generation is unavailable")]
    GenerationUnavailable,
    /// A dispatched attempt selection was missing or unlike its signed evidence.
    #[error("existing operation attempt selection is unavailable or unlike")]
    AttemptSelection,
    /// Attempt-owner classification rejected the retained selection.
    #[error(transparent)]
    Attempt(#[from] super::super::operation_dispatch::OperationAttemptError),
    /// The exact Delegate dispatch has present but unusable A2A send evidence.
    #[error(transparent)]
    A2aSend(#[from] A2aSendRecoveryError),
    /// A retained A2A Task candidate is incomplete, conflicting, or inadmissible.
    #[error(transparent)]
    A2aTask(#[from] A2aTaskRecoveryError),
    /// A retained A2A send exists, but this restart call supplied no A2A admission owner.
    #[error("existing Delegate A2A evidence cannot be admitted by this restart call")]
    A2aAdmissionUnavailable,
    /// Candidate pagination did not prove the complete retained Task set.
    #[error("existing Delegate A2A task candidates are incomplete")]
    A2aTaskCandidatesIncomplete,
    /// More than one candidate or an unlike binding claims the exact retained send.
    #[error("existing Delegate A2A task candidates conflict with the retained send")]
    A2aTaskCandidatesConflict,
    /// The recovered Consult roster no longer admits this exact child.
    #[error(transparent)]
    Consultation(#[from] ConsultationCollectionError),
    /// The admitted state and operation class do not permit one continuation.
    #[error("existing operation state has no valid restart continuation")]
    State,
    /// A bounded immutable record read failed.
    #[error(transparent)]
    Custody(#[from] pale_ports::ControlStoreError),
}

pub(super) fn admit_restart_lineage(
    application: &ResidentApplication<'_>,
    input: ResidentProcessRestartLineageInput<'_>,
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    child_control: &RecoveredOperationControl,
) -> Result<ResidentProcessOperationLineage, ResidentProcessOperationRestartError> {
    let (parent, collection, planned, candidate, observed_at) = match input {
        ResidentProcessRestartLineageInput::Root => {
            let handle = child_control.operation().operation().handle().1;
            if !matches!(
                handle.operation_class(),
                OperationClassV1::Invoke | OperationClassV1::Consult
            ) || handle.parent_operation().is_some()
            {
                return Err(ResidentProcessOperationRestartError::State);
            }
            return Ok(ResidentProcessOperationLineage::Root);
        }
        ResidentProcessRestartLineageInput::PairwiseDelegate { parent } => {
            return admit_pairwise_delegate_lineage(parent, selector, store, child_control);
        }
        ResidentProcessRestartLineageInput::RecoveredPairwiseDelegate => {
            let parent = child_control
                .operation()
                .recovered_parent()
                .ok_or(ResidentProcessOperationRestartError::State)?;
            return admit_recovered_pairwise_delegate_lineage(
                parent,
                selector,
                store,
                child_control,
            );
        }
        ResidentProcessRestartLineageInput::RecoveredConsultChild {
            parent,
            collection,
            slot,
        } => {
            return admit_recovered_consult_child_lineage(
                application,
                parent,
                collection,
                slot,
                selector,
                store,
                child_control,
            );
        }
        ResidentProcessRestartLineageInput::ConsultChild {
            parent,
            collection,
            planned,
            candidate,
            observed_at,
        } => (parent, collection, planned, candidate, observed_at),
    };
    let parent_operation = parent.control.operation().operation();
    let child_operation = child_control.operation().operation();
    let (parent_ref, parent_handle) = parent_operation.handle();
    let (child_ref, child_handle) = child_operation.handle();
    let parent_coordinate = ImmutableRecordReferenceV1 {
        record_ref: parent_ref.clone(),
        record_sha256: ArtifactDigest::parse(
            parent_handle.canonical_operation_handle_sha256().as_str(),
        )
        .map_err(|_| ResidentProcessOperationRestartError::State)?,
    };
    let child_coordinate = ImmutableRecordReferenceV1 {
        record_ref: child_ref.clone(),
        record_sha256: ArtifactDigest::parse(
            child_handle.canonical_operation_handle_sha256().as_str(),
        )
        .map_err(|_| ResidentProcessOperationRestartError::State)?,
    };
    let Some(parent_reference) = child_handle.parent_operation() else {
        return Err(ResidentProcessOperationRestartError::State);
    };
    let revalidated = collection.revalidate_child_dispatch(planned, observed_at)?;
    let selected = revalidated
        .candidates()
        .iter()
        .find(|selected| *selected == candidate)
        .ok_or(ResidentProcessOperationRestartError::State)?;
    let provider_binding = selected.provider_binding().cloned();
    if let Some(selected_provider) = provider_binding.as_ref() {
        let generation = application
            .generation(child_handle.compiled_generation_sha256())
            .ok_or(ResidentProcessOperationRestartError::GenerationUnavailable)?;
        let provider = generation
            .provider_binding(&selected_provider.provider_binding_id)
            .ok_or(ResidentProcessOperationRestartError::State)?;
        if provider.binding_identity != selected_provider.provider_binding_identity
            || provider.participant != *selected.participant()
        {
            return Err(ResidentProcessOperationRestartError::State);
        }
    }

    let parent_head = parent.control.observation().selected_head().1;
    let parent_session = parent_operation.historical_session_lease();
    let child_session = child_operation.historical_session_lease();
    let parent_lease = parent_operation.historical_operation_lease();
    let child_lease = child_operation.historical_operation_lease();
    let parent_collection_frontier = match parent_head.state() {
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::CollectingConsultation { evidence }
                | OperationActivePhaseV1::ConsultationCollected { evidence },
        } => evidence,
        _ => return Err(ResidentProcessOperationRestartError::State),
    };
    let context_matches = match selected.context() {
        Some(context) => child_lease
            .evidence_obligations()
            .contains(&context.record_ref),
        // The absence of a roster context means this child needs no additional
        // per-recipient context. It does not erase unrelated evidence duties
        // already narrowed from the parent lease.
        None => true,
    };
    if parent.selector != *selector
        || parent.control.operation().store() != store
        || parent.control.observation().store() != store
        || parent.generation_digest != *child_handle.compiled_generation_sha256()
        || collection.store() != store
        || planned.store() != store
        || planned.parent() != &parent_coordinate
        || collection.collection().parent() != &parent_coordinate
        || parent_collection_frontier != planned.collection_frontier()
        || selected.child() != &child_coordinate
        || parent_handle.operation_class() != &OperationClassV1::Consult
        || !selector.matches_operation(parent_operation)
        || !selector.matches_operation(child_operation)
        || child_handle.operation_class() != &OperationClassV1::Invoke
        || parent_reference.operation_handle_id != *parent_handle.operation_handle_id()
        || parent_reference.operation_handle_ref != *parent_ref
        || parent_reference.operation_handle_sha256
            != *parent_handle.canonical_operation_handle_sha256()
        || parent_handle.compiled_generation_sha256() != child_handle.compiled_generation_sha256()
        || parent_handle.activation_fence_sha256() != child_handle.activation_fence_sha256()
        || parent_handle.instance_sha256() != child_handle.instance_sha256()
        || parent_session != child_session
        || child_lease
            .verify_against_parent_operation(parent_lease)
            .is_err()
        || !context_matches
        || observed_at >= planned.deadline()
        || observed_at >= *child_handle.expires_at()
        || observed_at >= child_lease.expires_at()
    {
        return Err(ResidentProcessOperationRestartError::State);
    }

    Ok(ResidentProcessOperationLineage::ConsultChild(Box::new(
        ResidentProcessConsultChildLineage {
            parent: parent_coordinate,
            collection_frontier: planned.collection_frontier().clone(),
            slot_id: selected.slot_id().clone(),
            child: child_coordinate,
            participant: selected.participant().clone(),
            provider_binding,
            context: selected.context().cloned(),
        },
    )))
}

fn admit_recovered_consult_child_lineage(
    application: &ResidentApplication<'_>,
    parent: &RecoveredOperationControl,
    collection: &RecoveredConsultationCollection,
    slot: &ConsultationRosterSlot,
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    child_control: &RecoveredOperationControl,
) -> Result<ResidentProcessOperationLineage, ResidentProcessOperationRestartError> {
    let parent_operation = parent.operation().operation();
    let child_operation = child_control.operation().operation();
    let (parent_ref, parent_handle) = parent_operation.handle();
    let (child_ref, child_handle) = child_operation.handle();
    let Some(parent_reference) = child_handle.parent_operation() else {
        return Err(ResidentProcessOperationRestartError::State);
    };
    let parent_coordinate = ImmutableRecordReferenceV1 {
        record_ref: parent_ref.clone(),
        record_sha256: ArtifactDigest::parse(
            parent_handle.canonical_operation_handle_sha256().as_str(),
        )
        .map_err(|_| ResidentProcessOperationRestartError::State)?,
    };
    let child_coordinate = ImmutableRecordReferenceV1 {
        record_ref: child_ref.clone(),
        record_sha256: ArtifactDigest::parse(
            child_handle.canonical_operation_handle_sha256().as_str(),
        )
        .map_err(|_| ResidentProcessOperationRestartError::State)?,
    };
    let crate::resident::consultation::ConsultationContributionSource::FreshChild {
        child: selected_child,
    } = slot.source()
    else {
        return Err(ResidentProcessOperationRestartError::State);
    };
    let intent = parent_operation
        .consultation_start_intent()
        .ok_or(ResidentProcessOperationRestartError::State)?;
    intent
        .validate_parent(store, parent_operation)
        .map_err(|_| ResidentProcessOperationRestartError::State)?;
    if !intent.matches_collection(collection.collection()) {
        return Err(ResidentProcessOperationRestartError::State);
    }
    let matching_plans = intent
        .fresh_children()
        .filter(|plan| plan.matches_operation(child_operation))
        .count();
    let matching_slots = collection
        .collection()
        .slots()
        .iter()
        .filter(|candidate| {
            matches!(
                candidate.roster().source(),
                crate::resident::consultation::ConsultationContributionSource::FreshChild { child }
                    if child == &child_coordinate
            )
        })
        .count();
    let provider_binding = slot.provider_binding().cloned();
    if let Some(selected_provider) = provider_binding.as_ref() {
        let generation = application
            .generation(child_handle.compiled_generation_sha256())
            .ok_or(ResidentProcessOperationRestartError::GenerationUnavailable)?;
        let provider = generation
            .provider_binding(&selected_provider.provider_binding_id)
            .ok_or(ResidentProcessOperationRestartError::State)?;
        if provider.binding_identity != selected_provider.provider_binding_identity
            || provider.participant != *slot.participant()
        {
            return Err(ResidentProcessOperationRestartError::State);
        }
    }
    let context_matches = match slot.context() {
        Some(context) => child_operation
            .historical_operation_lease()
            .evidence_obligations()
            .contains(&context.record_ref),
        None => true,
    };
    if parent.operation().store() != store
        || parent.observation().store() != store
        || child_control.operation().store() != store
        || child_control.observation().store() != store
        || collection.store() != store
        || parent_handle.operation_class() != &OperationClassV1::Consult
        || parent_handle.parent_operation().is_some()
        || !selector.matches_operation(parent_operation)
        || !selector.matches_operation(child_operation)
        || !matches!(
            child_handle.operation_class(),
            OperationClassV1::Invoke | OperationClassV1::Consult
        )
        || parent_reference.operation_handle_id != *parent_handle.operation_handle_id()
        || parent_reference.operation_handle_ref != *parent_ref
        || parent_reference.operation_handle_sha256
            != *parent_handle.canonical_operation_handle_sha256()
        || collection.collection().parent() != &parent_coordinate
        || selected_child != &child_coordinate
        || matching_plans != 1
        || matching_slots != 1
        || parent_handle.compiled_generation_sha256() != child_handle.compiled_generation_sha256()
        || parent_handle.activation_fence_sha256() != child_handle.activation_fence_sha256()
        || parent_handle.instance_sha256() != child_handle.instance_sha256()
        || parent_operation.historical_session_lease() != child_operation.historical_session_lease()
        || child_operation
            .historical_operation_lease()
            .verify_against_parent_operation(parent_operation.historical_operation_lease())
            .is_err()
        || !context_matches
    {
        return Err(ResidentProcessOperationRestartError::State);
    }
    Ok(ResidentProcessOperationLineage::ConsultChild(Box::new(
        ResidentProcessConsultChildLineage {
            parent: parent_coordinate,
            collection_frontier: collection.frontier_reference()?,
            slot_id: slot.slot_id().clone(),
            child: child_coordinate,
            participant: slot.participant().clone(),
            provider_binding,
            context: slot.context().cloned(),
        },
    )))
}

fn admit_pairwise_delegate_lineage(
    parent: &ResidentProcessRecoveredOperationStart,
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    child_control: &RecoveredOperationControl,
) -> Result<ResidentProcessOperationLineage, ResidentProcessOperationRestartError> {
    if parent.control.operation().store() != store
        || parent.control.observation().store() != store
        || parent.selector != *selector
    {
        return Err(ResidentProcessOperationRestartError::State);
    }
    admit_recovered_pairwise_delegate_lineage(
        parent.control.operation(),
        selector,
        store,
        child_control,
    )
}

fn admit_recovered_pairwise_delegate_lineage(
    parent: &crate::resident::RecoveredLocalOperation,
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    child_control: &RecoveredOperationControl,
) -> Result<ResidentProcessOperationLineage, ResidentProcessOperationRestartError> {
    let parent_operation = parent.operation();
    let child_operation = child_control.operation().operation();
    let (parent_ref, parent_handle) = parent_operation.handle();
    let (child_ref, child_handle) = child_operation.handle();
    let Some(parent_reference) = child_handle.parent_operation() else {
        return Err(ResidentProcessOperationRestartError::State);
    };
    let parent_session = parent_operation.historical_session_lease();
    let child_session = child_operation.historical_session_lease();
    let parent_lease = parent_operation.historical_operation_lease();
    let child_lease = child_operation.historical_operation_lease();
    if parent.store() != store
        || child_control.operation().store() != store
        || child_control.observation().store() != store
        || !selector.matches_operation(parent_operation)
        || !selector.matches_operation(child_operation)
        || child_handle.operation_class() != &OperationClassV1::Delegate
        || child_operation.consultation_start_intent().is_some()
        || parent_reference.operation_handle_id != *parent_handle.operation_handle_id()
        || parent_reference.operation_handle_ref != *parent_ref
        || parent_reference.operation_handle_sha256
            != *parent_handle.canonical_operation_handle_sha256()
        || parent_handle.compiled_generation_sha256() != child_handle.compiled_generation_sha256()
        || parent_handle.activation_fence_sha256() != child_handle.activation_fence_sha256()
        || parent_handle.instance_sha256() != child_handle.instance_sha256()
        || parent_session != child_session
        || child_lease
            .verify_against_parent_operation(parent_lease)
            .is_err()
    {
        return Err(ResidentProcessOperationRestartError::State);
    }
    let parent = ImmutableRecordReferenceV1 {
        record_ref: parent_ref.clone(),
        record_sha256: ArtifactDigest::parse(
            parent_handle.canonical_operation_handle_sha256().as_str(),
        )
        .map_err(|_| ResidentProcessOperationRestartError::State)?,
    };
    let child = ImmutableRecordReferenceV1 {
        record_ref: child_ref.clone(),
        record_sha256: ArtifactDigest::parse(
            child_handle.canonical_operation_handle_sha256().as_str(),
        )
        .map_err(|_| ResidentProcessOperationRestartError::State)?,
    };
    Ok(ResidentProcessOperationLineage::PairwiseDelegate(Box::new(
        ResidentProcessPairwiseDelegateLineage { parent, child },
    )))
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Re-read and classify one operation that this process just started.
    ///
    /// The issued locator proves only the completed start. This method reads the
    /// protected start back from custody and then uses the normal restart owner,
    /// so cancellation or another committed transition can win before dispatch.
    pub(super) fn classify_started_operation(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        issued: &super::super::IssuedLocalOperation,
        request: &CommonRequestEnvelopeV1,
        access: ResidentDeploymentOperationAccess<'_>,
        restart: ResidentProcessOperationRestartInput<'_>,
        mut admission: OperationControlRecoveryAdmission<'_>,
    ) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>
    {
        if !selector.matches_operation(issued) {
            return Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority.into());
        }
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let evidence = read_operation_start_for_request(
            &mut *custody,
            store,
            request,
            restart.operation.maximum_record_bytes,
        )?
        .ok_or(ResidentProcessOperationRestartError::StartEvidence)?;
        let (handle_ref, handle) = issued.handle();
        if evidence.request().coordinate() != issued.request_coordinate()
            || evidence.handle().coordinate().record_ref() != handle_ref
            || evidence.handle().coordinate().record_sha256().as_str()
                != handle.canonical_operation_handle_sha256().as_str()
        {
            return Err(ResidentProcessOperationRestartError::StartEvidence);
        }
        let request_preimage = request.to_preimage();
        let association = ResidentOperationControlAssociation {
            current_request_authority: &request_preimage.signing_authority_binding,
            historical_handle_authority: restart.operation.handle_signing_authority,
            recovery_store: restart.operation.store,
            checkpoint_authority: restart.operation.checkpoint_authority,
        };
        self.classify_existing_operation_with_lineage(
            ResidentProcessExistingOperationStart {
                selector: selector.clone(),
                evidence: Box::new(evidence),
            },
            association,
            ResidentDeploymentOperationAccess {
                store,
                custody,
                sessions,
                anchor,
            },
            restart,
            &mut admission,
            ResidentProcessRestartQualification {
                lineage: ResidentProcessRestartLineageInput::Root,
                protected: None,
            },
        )
    }

    /// Recover and classify one duplicate operation start without resuming work.
    pub(super) fn classify_existing_operation(
        &self,
        existing: ResidentProcessExistingOperationStart,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationRestartInput<'_>,
        mut admission: OperationControlRecoveryAdmission<'_>,
    ) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>
    {
        self.classify_existing_operation_with_lineage(
            existing,
            association,
            access,
            input,
            &mut admission,
            ResidentProcessRestartQualification {
                lineage: ResidentProcessRestartLineageInput::Root,
                protected: None,
            },
        )
    }

    /// Protect one existing start before selecting its semantic recovery owner.
    ///
    /// This step admits the request and handle, recovers the exact current head,
    /// verifies checkpoint and session custody, checks the independent anchor,
    /// and re-reads the start evidence. It intentionally does not admit root,
    /// Consult-child, or pairwise-Delegate lineage.
    pub(super) fn protect_existing_operation_start(
        &self,
        existing: ResidentProcessExistingOperationStart,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: &ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
    ) -> Result<ResidentProcessProtectedOperationStart, ResidentProcessOperationRestartError> {
        let index = self.deployment_authority_index(&existing.selector)?;
        Self::protect_existing_operation_start_in(
            self.deployments[index].application.as_ref(),
            existing,
            &association,
            access,
            input,
            admission,
            None,
        )
    }

    /// Protect one exact point-recovered start before selecting its lineage owner.
    ///
    /// The caller has already admitted the locator against `recovered`. This
    /// method re-reads the immutable start evidence for the recovered request
    /// and joins it to that same handle before any root or child classifier can
    /// run. It performs no prefix discovery and does not advance a recovery
    /// cursor.
    pub(super) fn protect_point_recovered_operation_start(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        recovered: RecoveredOperationControl,
        access: ResidentDeploymentOperationAccess<'_>,
        input: &ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
    ) -> Result<ResidentProcessProtectedOperationStart, ResidentProcessOperationRestartError> {
        let index = self.deployment_authority_index(selector)?;
        let application = self.deployments[index].application.as_ref();
        let operation = recovered.operation();
        if !selector.matches_operation(operation.operation())
            || operation.store() != access.store
            || input.operation.store != access.store
        {
            return Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation.into());
        }
        let request = operation.request();
        let expected_handle = operation.operation().handle();
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let evidence = read_operation_start_for_request(
            &mut *custody,
            store,
            request,
            input.operation.maximum_record_bytes,
        )?
        .ok_or(ResidentProcessOperationRestartError::StartEvidence)?;
        if evidence.request().coordinate() != operation.operation().request_coordinate()
            || evidence.handle().coordinate().record_ref() != expected_handle.0
            || evidence.handle().coordinate().record_sha256().as_str()
                != expected_handle
                    .1
                    .canonical_operation_handle_sha256()
                    .as_str()
        {
            return Err(ResidentProcessOperationRestartError::StartEvidence);
        }
        let request_preimage = request.to_preimage();
        let association = ResidentOperationControlAssociation {
            current_request_authority: &request_preimage.signing_authority_binding,
            historical_handle_authority: input.operation.handle_signing_authority,
            recovery_store: input.operation.store,
            checkpoint_authority: input.operation.checkpoint_authority,
        };
        Self::protect_existing_operation_start_in(
            application,
            ResidentProcessExistingOperationStart {
                selector: selector.clone(),
                evidence: Box::new(evidence),
            },
            &association,
            ResidentDeploymentOperationAccess {
                store,
                custody,
                sessions,
                anchor,
            },
            input,
            admission,
            Some(recovered),
        )
    }

    /// Continue one already-protected unparented root through semantic classification.
    pub(super) fn classify_protected_root_operation(
        &self,
        protected: ResidentProcessProtectedOperationStart,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
    ) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>
    {
        let index = self.deployment_authority_index(&protected.existing.selector)?;
        Self::classify_protected_operation_with_lineage_in(
            self.deployments[index].application.as_ref(),
            protected,
            access,
            input,
            admission,
            ResidentProcessRestartLineageInput::Root,
        )
    }

    /// Continue one protected Delegate through its already recovered parent chain.
    ///
    /// The immediate parent is read only from the bounded, cycle-checked chain
    /// retained by protected handle recovery. This method does not classify,
    /// resume, append to, accept, or settle that parent.
    pub(super) fn classify_protected_delegate_child_operation(
        &self,
        protected: ResidentProcessProtectedOperationStart,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
    ) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>
    {
        if protected.operation_class() != &OperationClassV1::Delegate
            || protected.parent_operation().is_none()
        {
            return Err(ResidentProcessOperationRestartError::State);
        }
        let index = self.deployment_authority_index(&protected.existing.selector)?;
        Self::classify_protected_operation_with_lineage_in(
            self.deployments[index].application.as_ref(),
            protected,
            access,
            input,
            admission,
            ResidentProcessRestartLineageInput::RecoveredPairwiseDelegate,
        )
    }

    /// Recover the current immediate parent of one already-protected child.
    ///
    /// The child's bounded handle chain supplies the parent coordinate. Parent
    /// recovery may re-read the parent, but never re-reads the child start or
    /// child observation. The final comparison prevents an unrelated current
    /// parent recovery from replacing the historical parent proved by the child.
    pub(super) fn recover_current_parent_for_protected_child(
        &self,
        protected: ResidentProcessProtectedOperationStart,
        access: ResidentDeploymentOperationAccess<'_>,
        input: &ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
    ) -> Result<
        (
            ResidentProcessProtectedOperationStart,
            RecoveredOperationControl,
        ),
        ResidentProcessOperationRestartError,
    > {
        let historical_parent = protected
            .control
            .operation()
            .recovered_parent()
            .ok_or(ResidentProcessOperationRestartError::State)?;
        let (parent_ref, parent_handle) = historical_parent.operation().handle();
        let expected_parent = ControlRecordCoordinate::new(
            parent_ref.clone(),
            pale_ports::ControlRecordDigest::parse(
                parent_handle
                    .canonical_operation_handle_sha256()
                    .as_str()
                    .to_owned(),
            )
            .map_err(|_| ResidentProcessOperationRestartError::State)?,
        );
        let ResidentDeploymentOperationAccess {
            custody,
            sessions,
            anchor,
            ..
        } = access;
        let current_parent = recover_operation_control(
            OperationControlRecoveryInput {
                operation: LocalOperationHandleRecoveryInput {
                    context: input.operation,
                    expected_handle: &expected_parent,
                    maximum_parent_depth: input.maximum_parent_depth,
                },
                observation: OperationObservationSelection::SelectedHead,
                observation_limits: input.current_head.observation,
                checkpoint_limits: input.current_head.checkpoints,
                membership_limits: input.current_head.membership,
            },
            custody,
            sessions,
            anchor,
            reborrow_operation_recovery_admission(admission),
        )?;
        let current_operation = current_parent.operation();
        if historical_parent.store() != current_operation.store()
            || historical_parent.request() != current_operation.request()
            || historical_parent.operation().handle() != current_operation.operation().handle()
            || historical_parent.operation().request_coordinate()
                != current_operation.operation().request_coordinate()
            || historical_parent.operation().historical_session_lease()
                != current_operation.operation().historical_session_lease()
            || historical_parent.operation().historical_operation_lease()
                != current_operation.operation().historical_operation_lease()
        {
            return Err(ResidentProcessOperationRestartError::State);
        }
        Ok((protected, current_parent))
    }

    /// Classify one already-protected child against a recovered current Consult
    /// parent and the exact current roster slot.
    pub(super) fn classify_protected_recovered_consult_child_operation(
        &self,
        protected: ResidentProcessProtectedOperationStart,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
        lineage: ResidentProcessRecoveredConsultChildLineage<'_>,
    ) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>
    {
        if protected.parent_operation().is_none()
            || !matches!(
                protected.operation_class(),
                OperationClassV1::Invoke | OperationClassV1::Consult
            )
        {
            return Err(ResidentProcessOperationRestartError::State);
        }
        let index = self.deployment_authority_index(&protected.existing.selector)?;
        Self::classify_protected_operation_with_lineage_in(
            self.deployments[index].application.as_ref(),
            protected,
            access,
            input,
            admission,
            ResidentProcessRestartLineageInput::RecoveredConsultChild {
                parent: lineage.parent,
                collection: lineage.collection,
                slot: lineage.slot,
            },
        )
    }

    /// Recover one exact Consult child without treating roster scheduling as authority.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn classify_consultation_child_operation(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
        parent: &ResidentProcessRecoveredOperationStart,
        collection: &RecoveredConsultationCollection,
        planned: &ConsultationChildDispatchBatch,
        candidate: &ConsultationChildDispatchCandidate,
        observed_at: UnixTimeV1,
    ) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>
    {
        let index = self.deployment_authority_index(selector)?;
        Self::classify_consultation_child_operation_in(
            self.deployments[index].application.as_ref(),
            selector,
            association,
            access,
            input,
            admission,
            parent,
            collection,
            planned,
            candidate,
            None,
            observed_at,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn classify_consultation_child_operation_in(
        application: &ResidentApplication<'_>,
        selector: &ResidentDeploymentAuthoritySelector,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
        parent: &ResidentProcessRecoveredOperationStart,
        collection: &RecoveredConsultationCollection,
        planned: &ConsultationChildDispatchBatch,
        candidate: &ConsultationChildDispatchCandidate,
        selected_dispatch: Option<&ImmutableRecordReferenceV1>,
        observed_at: UnixTimeV1,
    ) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>
    {
        if !selector.matches_signing_authority(association.current_request_authority)
            || !selector.matches_signing_authority(association.historical_handle_authority)
        {
            return Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority.into());
        }
        let selected_store = application.store();
        if access.store != selected_store
            || input.operation.store != selected_store
            || association.recovery_store != selected_store
        {
            return Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation.into());
        }
        let expected_handle = ControlRecordCoordinate::new(
            candidate.child().record_ref.clone(),
            pale_ports::ControlRecordDigest::parse(candidate.child().record_sha256.as_str())
                .map_err(|_| ResidentProcessOperationRestartError::StartEvidence)?,
        );
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let selected_dispatch_digest = selected_dispatch
            .map(|selected| {
                OperationObservationDigest::parse(selected.record_sha256.as_str())
                    .map_err(|_| ResidentProcessOperationRestartError::AttemptSelection)
            })
            .transpose()?;
        let selection = match (selected_dispatch, selected_dispatch_digest.as_ref()) {
            (Some(selected), Some(digest)) => OperationObservationSelection::HistoryMember {
                reference: &selected.record_ref,
                digest,
            },
            _ => OperationObservationSelection::SelectedHead,
        };
        let protected = recover_control(
            &input,
            &expected_handle,
            selection,
            input.current_head,
            custody,
            sessions,
            anchor,
            admission,
        )?;
        if !selector.matches_operation(protected.operation().operation())
            || protected.operation().operation().handle().0 != &candidate.child().record_ref
            || protected
                .operation()
                .operation()
                .handle()
                .1
                .canonical_operation_handle_sha256()
                .as_str()
                != candidate.child().record_sha256.as_str()
        {
            return Err(ResidentProcessOperationRestartError::StartEvidence);
        }
        let evidence = read_operation_start_for_request(
            custody,
            selected_store,
            protected.operation().request(),
            input.operation.maximum_record_bytes,
        )?
        .ok_or(ResidentProcessOperationRestartError::StartEvidence)?;
        if evidence.handle().coordinate() != &expected_handle {
            return Err(ResidentProcessOperationRestartError::StartEvidence);
        }
        Self::classify_existing_operation_with_lineage_in(
            application,
            ResidentProcessExistingOperationStart {
                selector: selector.clone(),
                evidence: Box::new(evidence),
            },
            association,
            ResidentDeploymentOperationAccess {
                store,
                custody,
                sessions,
                anchor,
            },
            input,
            admission,
            ResidentProcessRestartQualification {
                lineage: ResidentProcessRestartLineageInput::ConsultChild {
                    parent,
                    collection,
                    planned,
                    candidate,
                    observed_at,
                },
                protected: Some(protected),
            },
        )
    }

    /// Recover and classify one pairwise Delegate against its protected parent.
    ///
    /// The exact child recovery owner proves parent identity, pair membership,
    /// session, generation and lease narrowing. This path does not consult a
    /// roster, allocate a slot, form a join, select a provider or dispatch work.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn classify_delegate_child_operation(
        &self,
        existing: ResidentProcessExistingOperationStart,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
        parent: &ResidentProcessRecoveredOperationStart,
    ) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>
    {
        let index = self.deployment_authority_index(&existing.selector)?;
        Self::classify_delegate_child_operation_in(
            self.deployments[index].application.as_ref(),
            existing,
            association,
            access,
            input,
            admission,
            parent,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn classify_delegate_child_operation_in(
        application: &ResidentApplication<'_>,
        existing: ResidentProcessExistingOperationStart,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
        parent: &ResidentProcessRecoveredOperationStart,
    ) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>
    {
        let selector = &existing.selector;
        if !selector.matches_signing_authority(association.current_request_authority)
            || !selector.matches_signing_authority(association.historical_handle_authority)
        {
            return Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority.into());
        }
        let selected_store = application.store();
        if access.store != selected_store
            || input.operation.store != selected_store
            || association.recovery_store != selected_store
            || parent.control.operation().store() != selected_store
            || parent.control.observation().store() != selected_store
            || parent.selector != *selector
        {
            return Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation.into());
        }
        let expected_handle = existing.evidence.handle().coordinate().clone();
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let protected = recover_pairwise_child_control(
            &input,
            &expected_handle,
            OperationObservationSelection::SelectedHead,
            input.current_head,
            custody,
            sessions,
            anchor,
            admission,
            parent,
        )?;
        if protected.operation().operation().handle().0 != expected_handle.record_ref()
            || protected
                .operation()
                .operation()
                .handle()
                .1
                .canonical_operation_handle_sha256()
                .as_str()
                != expected_handle.record_sha256().as_str()
        {
            return Err(ResidentProcessOperationRestartError::StartEvidence);
        }
        Self::classify_existing_operation_with_lineage_in(
            application,
            existing,
            association,
            ResidentDeploymentOperationAccess {
                store,
                custody,
                sessions,
                anchor,
            },
            input,
            admission,
            ResidentProcessRestartQualification {
                lineage: ResidentProcessRestartLineageInput::PairwiseDelegate { parent },
                protected: Some(protected),
            },
        )
    }

    fn classify_existing_operation_with_lineage(
        &self,
        existing: ResidentProcessExistingOperationStart,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
        qualification: ResidentProcessRestartQualification<'_>,
    ) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>
    {
        let index = self.deployment_authority_index(&existing.selector)?;
        Self::classify_existing_operation_with_lineage_in(
            self.deployments[index].application.as_ref(),
            existing,
            association,
            access,
            input,
            admission,
            qualification,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn classify_existing_operation_with_lineage_in(
        application: &ResidentApplication<'_>,
        existing: ResidentProcessExistingOperationStart,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
        qualification: ResidentProcessRestartQualification<'_>,
    ) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>
    {
        let ResidentProcessRestartQualification { lineage, protected } = qualification;
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let protected = Self::protect_existing_operation_start_in(
            application,
            existing,
            &association,
            ResidentDeploymentOperationAccess {
                store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            &input,
            admission,
            protected,
        )?;
        Self::classify_protected_operation_with_lineage_in(
            application,
            protected,
            ResidentDeploymentOperationAccess {
                store,
                custody,
                sessions,
                anchor,
            },
            input,
            admission,
            lineage,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn protect_existing_operation_start_in(
        application: &ResidentApplication<'_>,
        existing: ResidentProcessExistingOperationStart,
        association: &ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: &ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
        protected: Option<RecoveredOperationControl>,
    ) -> Result<ResidentProcessProtectedOperationStart, ResidentProcessOperationRestartError> {
        verify_restart_inputs(application, &existing, association, access.store, input)?;
        let selector = &existing.selector;
        let selected_store = application.store();

        let expected_handle = existing.evidence.handle().coordinate().clone();
        let ResidentDeploymentOperationAccess {
            custody,
            sessions,
            anchor,
            ..
        } = access;
        let control = if let Some(control) = protected {
            control
        } else {
            recover_control(
                input,
                &expected_handle,
                OperationObservationSelection::SelectedHead,
                input.current_head,
                custody,
                sessions,
                anchor,
                admission,
            )?
        };
        if !selector.matches_operation(control.operation().operation()) {
            return Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority.into());
        }
        verify_start_evidence(
            &existing.evidence,
            custody,
            selected_store,
            control.operation().request(),
            input.operation.maximum_record_bytes,
        )?;

        Ok(ResidentProcessProtectedOperationStart { existing, control })
    }

    #[allow(clippy::too_many_arguments)]
    fn classify_protected_operation_with_lineage_in(
        application: &ResidentApplication<'_>,
        protected: ResidentProcessProtectedOperationStart,
        access: ResidentDeploymentOperationAccess<'_>,
        mut input: ResidentProcessOperationRestartInput<'_>,
        admission: &mut OperationControlRecoveryAdmission<'_>,
        lineage: ResidentProcessRestartLineageInput<'_>,
    ) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>
    {
        let ResidentProcessProtectedOperationStart { existing, control } = protected;
        let selected_store = application.store();
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;

        let lineage = admit_restart_lineage(
            application,
            lineage,
            &existing.selector,
            selected_store,
            &control,
        )?;

        let generation_digest = control
            .operation()
            .operation()
            .handle()
            .1
            .compiled_generation_sha256()
            .clone();
        let mut attempt_observation = ResidentProcessAttemptObservation::CurrentHead;
        let mut has_cancellation_predecessor = false;
        let mut cancellation_request = None;
        if matches!(
            control.observation().selected_head().1.state(),
            OperationObservationStateV1::CancellationRequested { .. }
        ) {
            let head = control.observation().selected_head();
            let OperationObservationStateV1::CancellationRequested { request } = head.1.state()
            else {
                return Err(ResidentProcessOperationRestartError::State);
            };
            cancellation_request = Some(request.clone());
            let predecessor_ref = head
                .1
                .predecessor_ref()
                .as_ref()
                .ok_or(ResidentProcessOperationRestartError::State)?
                .clone();
            let predecessor_sha256 = head
                .1
                .predecessor_sha256()
                .as_ref()
                .ok_or(ResidentProcessOperationRestartError::State)?
                .clone();
            let Some((recovered_ref, recovered_predecessor)) =
                control.observation().selected_predecessor()
            else {
                return Err(ResidentProcessOperationRestartError::State);
            };
            if recovered_ref != &predecessor_ref
                || recovered_predecessor.canonical_operation_observation_sha256()
                    != &predecessor_sha256
            {
                return Err(ResidentProcessOperationRestartError::StartEvidence);
            }
            has_cancellation_predecessor = true;
            attempt_observation = ResidentProcessAttemptObservation::CancellationPredecessor;
        }

        let classification_state = if has_cancellation_predecessor {
            control
                .observation()
                .selected_predecessor()
                .ok_or(ResidentProcessOperationRestartError::State)?
                .1
                .state()
                .clone()
        } else {
            control.observation().selected_head().1.state().clone()
        };
        let start = |control| ResidentProcessRecoveredOperationStart {
            selector: existing.selector.clone(),
            control,
            generation_digest: generation_digest.clone(),
            lineage: lineage.clone(),
        };
        let select_cancellation_predecessor = |control: RecoveredOperationControl| {
            if has_cancellation_predecessor {
                control.select_immediate_predecessor()
            } else {
                Ok(control)
            }
        };
        if control.operation().request().body().schema_id.as_str()
            == pale_core::records::NATIVE_TOOL_REQUEST_V1_SCHEMA_ID
            && matches!(
                &classification_state,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::DispatchPrepared { .. }
                        | OperationActivePhaseV1::Dispatched { .. }
                }
            )
        {
            let control = select_cancellation_predecessor(control)?;
            let response = recover_native_hook_response(
                control.operation().operation(),
                control.operation().request(),
                control.observation(),
                custody,
                anchor,
                input.maximum_selection_bytes,
                input.operation.maximum_record_bytes,
            )?;
            return Ok(
                ResidentProcessOperationRestartDisposition::NativeHookResponse(Box::new(
                    ResidentProcessRecoveredNativeHookResponse {
                        start: start(control),
                        response,
                    },
                )),
            );
        }
        match classification_state {
            OperationObservationStateV1::Succeeded { .. }
            | OperationObservationStateV1::Failed { .. }
            | OperationObservationStateV1::Cancelled { .. } => {
                let control = select_cancellation_predecessor(control)?;
                Ok(ResidentProcessOperationRestartDisposition::Terminal(
                    Box::new(start(control)),
                ))
            }
            OperationObservationStateV1::Recovering { .. }
            | OperationObservationStateV1::OutcomeUnknown { .. }
            | OperationObservationStateV1::Expired { .. } => {
                let control = select_cancellation_predecessor(control)?;
                Ok(ResidentProcessOperationRestartDisposition::RecoveryOnly(
                    Box::new(start(control)),
                ))
            }
            OperationObservationStateV1::Active {
                phase:
                    OperationActivePhaseV1::Resolving | OperationActivePhaseV1::ContextBound { .. },
            } => {
                if attempt_observation == ResidentProcessAttemptObservation::CancellationPredecessor
                {
                    Ok(
                        ResidentProcessOperationRestartDisposition::LocalCancellationRequested(
                            Box::new(ResidentProcessRecoveredLocalCancellation {
                                start: start(control),
                                request: cancellation_request
                                    .ok_or(ResidentProcessOperationRestartError::State)?,
                            }),
                        ),
                    )
                } else {
                    let operation = control.operation().operation();
                    let phase = match classification_state {
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::Resolving,
                        } => ResidentProcessOperationResolutionPhase::Resolving,
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::ContextBound { .. },
                        } if !operation
                            .historical_operation_lease()
                            .evidence_obligations()
                            .is_empty() =>
                        {
                            ResidentProcessOperationResolutionPhase::ContextBound
                        }
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::ContextBound { .. },
                        } => return Err(ResidentProcessOperationRestartError::State),
                        _ => return Err(ResidentProcessOperationRestartError::State),
                    };
                    if !lineage.admits(operation.handle().0, operation.handle().1) {
                        return Err(ResidentProcessOperationRestartError::State);
                    }
                    Ok(
                        ResidentProcessOperationRestartDisposition::OperationResolution(Box::new(
                            ResidentProcessRecoveredOperationResolution {
                                start: start(control),
                                phase,
                            },
                        )),
                    )
                }
            }
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ProviderResolved { provider_binding },
            } => {
                if attempt_observation == ResidentProcessAttemptObservation::CancellationPredecessor
                {
                    return Ok(
                        ResidentProcessOperationRestartDisposition::LocalCancellationRequested(
                            Box::new(ResidentProcessRecoveredLocalCancellation {
                                start: start(control),
                                request: cancellation_request
                                    .ok_or(ResidentProcessOperationRestartError::State)?,
                            }),
                        ),
                    );
                }
                let issued = control.operation().operation();
                let generation = application
                    .generation(&generation_digest)
                    .ok_or(ResidentProcessOperationRestartError::GenerationUnavailable)?;
                if !lineage.admits(issued.handle().0, issued.handle().1)
                    || !lineage.admits_provider(Some(&provider_binding))
                    || !provider_resolution_matches_generation(
                        generation,
                        issued,
                        issued.historical_operation_lease(),
                        &provider_binding,
                        lineage
                            .consult_child()
                            .map(ResidentProcessConsultChildLineage::participant),
                    )
                {
                    return Err(ResidentProcessOperationRestartError::State);
                }
                let provider_binding = provider_binding.clone();
                Ok(
                    ResidentProcessOperationRestartDisposition::ProviderResolved(Box::new(
                        ResidentProcessRecoveredProviderResolution {
                            start: start(control),
                            provider_binding,
                        },
                    )),
                )
            }
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::DispatchPrepared { evidence, .. },
            } => {
                if attempt_observation == ResidentProcessAttemptObservation::CancellationPredecessor
                {
                    return Ok(
                        ResidentProcessOperationRestartDisposition::LocalCancellationRequested(
                            Box::new(ResidentProcessRecoveredLocalCancellation {
                                start: start(control),
                                request: cancellation_request
                                    .ok_or(ResidentProcessOperationRestartError::State)?,
                            }),
                        ),
                    );
                }
                let issued = control.operation().operation();
                if !lineage.admits(issued.handle().0, issued.handle().1) {
                    return Err(ResidentProcessOperationRestartError::State);
                }
                let generation = application
                    .generation(&generation_digest)
                    .ok_or(ResidentProcessOperationRestartError::GenerationUnavailable)?;
                let coordinate = selection_coordinate(&evidence)?;
                let selection = custody
                    .read_operation_record(
                        selected_store,
                        &coordinate,
                        input.maximum_selection_bytes,
                    )?
                    .ok_or(ResidentProcessOperationRestartError::AttemptSelection)?;
                if selection.coordinate() != &coordinate {
                    return Err(ResidentProcessOperationRestartError::AttemptSelection);
                }
                let history =
                    crate::resident::operation_dispatch::OperationPreparationHistoryInput {
                        operation: issued,
                        request: control.operation().request(),
                        session: issued.historical_session_lease(),
                        lease: issued.historical_operation_lease(),
                        generation,
                        prepared: control.observation().selected().1,
                        selection: &selection,
                        maximum_selection_bytes: input.maximum_selection_bytes,
                    };
                let provider_binding = history.provider_binding()?;
                let owner = history.classify_owner()?;
                if !lineage.admits_provider(provider_binding.as_ref()) {
                    return Err(ResidentProcessOperationRestartError::AttemptSelection);
                }
                let attempt = Box::new(ResidentProcessRecoveredPreparedAttempt {
                    start: start(control),
                    selection,
                });
                Ok(match owner {
                    OperationAttemptOwner::Native => {
                        ResidentProcessOperationRestartDisposition::NativePrepared(attempt)
                    }
                    OperationAttemptOwner::Provider => {
                        ResidentProcessOperationRestartDisposition::ProviderPrepared(attempt)
                    }
                })
            }
            OperationObservationStateV1::Active {
                phase:
                    OperationActivePhaseV1::CollectingConsultation { .. }
                    | OperationActivePhaseV1::ConsultationCollected { .. },
            } => {
                if control.operation().operation().handle().1.operation_class()
                    != &OperationClassV1::Consult
                {
                    return Err(ResidentProcessOperationRestartError::State);
                }
                let control = select_cancellation_predecessor(control)?;
                Ok(ResidentProcessOperationRestartDisposition::Consult(
                    Box::new(start(control)),
                ))
            }
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched { evidence, .. },
            } => {
                let control = select_cancellation_predecessor(control)?;
                let recovered_dispatch = control.observation();
                let selected = recovered_dispatch.selected();
                let generation = application
                    .generation(&generation_digest)
                    .ok_or(ResidentProcessOperationRestartError::GenerationUnavailable)?;
                let coordinate = selection_coordinate(&evidence)?;
                let selection = custody
                    .read_operation_record(
                        selected_store,
                        &coordinate,
                        input.maximum_selection_bytes,
                    )?
                    .ok_or(ResidentProcessOperationRestartError::AttemptSelection)?;
                if selection.coordinate() != &coordinate {
                    return Err(ResidentProcessOperationRestartError::AttemptSelection);
                }
                let history = OperationAttemptHistoryInput {
                    operation: control.operation().operation(),
                    request: control.operation().request(),
                    session: control.operation().operation().historical_session_lease(),
                    lease: control.operation().operation().historical_operation_lease(),
                    generation,
                    dispatched: selected.1,
                    selection: &selection,
                    maximum_selection_bytes: input.maximum_selection_bytes,
                };
                let provider_binding = history.provider_binding()?;
                let owner = history.classify_owner()?;
                if !lineage.admits_provider(provider_binding.as_ref()) {
                    return Err(ResidentProcessOperationRestartError::AttemptSelection);
                }
                if attempt_observation == ResidentProcessAttemptObservation::CancellationPredecessor
                {
                    let request = cancellation_request
                        .as_ref()
                        .ok_or(ResidentProcessOperationRestartError::State)?;
                    match classify_a2a_cancel_task_request(
                        &control,
                        request,
                        &owner,
                        OperationAttemptHistoryInput {
                            operation: history.operation,
                            request: history.request,
                            session: history.session,
                            lease: history.lease,
                            generation: history.generation,
                            dispatched: history.dispatched,
                            selection: history.selection,
                            maximum_selection_bytes: history.maximum_selection_bytes,
                        },
                        selected_store,
                        custody,
                        anchor,
                        input.operation.maximum_record_bytes,
                        input.current_head,
                        input.operation.checkpoint_authority,
                        input.a2a_cancel_task.as_mut(),
                        admission,
                    ) {
                        A2aCancelTaskRestartClassification::LocalCancellationRequest => {}
                        A2aCancelTaskRestartClassification::RecoveryOnly => {
                            return Ok(ResidentProcessOperationRestartDisposition::RecoveryOnly(
                                Box::new(start(control)),
                            ));
                        }
                        A2aCancelTaskRestartClassification::A2aCancelTaskRequest {
                            task,
                            request_evidence,
                            request_id,
                        } => {
                            if verify_recovered_operation_head_custody(
                                control.operation().operation(),
                                recovered_dispatch,
                                custody,
                                anchor,
                                input.operation.maximum_record_bytes,
                            )
                            .is_err()
                            {
                                return Ok(
                                    ResidentProcessOperationRestartDisposition::RecoveryOnly(
                                        Box::new(start(control)),
                                    ),
                                );
                            }
                            return Ok(
                                ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(
                                    Box::new(ResidentProcessRecoveredA2aCancelTaskRequest {
                                        attempt: ResidentProcessRecoveredAttempt {
                                            start: start(control),
                                            dispatch: attempt_observation,
                                            selection,
                                        },
                                        task,
                                        request_evidence,
                                        request_id,
                                    }),
                                ),
                            );
                        }
                    }
                }
                let is_delegate = control.operation().operation().handle().1.operation_class()
                    == &OperationClassV1::Delegate;
                let a2a_delegate = if matches!(owner, OperationAttemptOwner::Provider)
                    && is_delegate
                    && attempt_observation == ResidentProcessAttemptObservation::CurrentHead
                {
                    Some(classify_pairwise_a2a_task(
                        &control,
                        OperationAttemptHistoryInput {
                            operation: history.operation,
                            request: history.request,
                            session: history.session,
                            lease: history.lease,
                            generation: history.generation,
                            dispatched: history.dispatched,
                            selection: history.selection,
                            maximum_selection_bytes: history.maximum_selection_bytes,
                        },
                        selected_store,
                        custody,
                        anchor,
                        input.current_head,
                        input.operation.maximum_record_bytes,
                        input.operation.checkpoint_authority,
                        input.a2a_task.as_mut(),
                        admission,
                    )?)
                } else {
                    None
                };
                let attempt = ResidentProcessRecoveredAttempt {
                    start: start(control),
                    dispatch: attempt_observation,
                    selection,
                };
                Ok(match (owner, a2a_delegate) {
                    (OperationAttemptOwner::Native, _) => {
                        ResidentProcessOperationRestartDisposition::Native(Box::new(attempt))
                    }
                    (
                        OperationAttemptOwner::Provider,
                        Some(PairwiseA2aRestartClassification::ExactTask(task)),
                    ) => ResidentProcessOperationRestartDisposition::A2aTask(Box::new(
                        ResidentProcessRecoveredA2aTask { attempt, task },
                    )),
                    (
                        OperationAttemptOwner::Provider,
                        Some(PairwiseA2aRestartClassification::ExactSendWithoutTask),
                    ) => ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(
                        Box::new(attempt),
                    ),
                    (
                        OperationAttemptOwner::Provider,
                        Some(PairwiseA2aRestartClassification::NoRetainedSend),
                    )
                    | (OperationAttemptOwner::Provider, None) => {
                        ResidentProcessOperationRestartDisposition::Provider(Box::new(attempt))
                    }
                })
            }
            OperationObservationStateV1::CancellationRequested { .. } => {
                Err(ResidentProcessOperationRestartError::State)
            }
        }
    }
}

enum PairwiseA2aRestartClassification {
    NoRetainedSend,
    ExactSendWithoutTask,
    ExactTask(Box<RecoveredA2aTaskHistory>),
}

#[allow(clippy::too_many_arguments)]
fn classify_pairwise_a2a_task(
    control: &RecoveredOperationControl,
    history: OperationAttemptHistoryInput<'_>,
    store: &AuthorityStoreCoordinate,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    local_limits: ResidentProcessRestartReadLimits,
    maximum_record_bytes: usize,
    checkpoint_authority: &pale_core::records::ControlCheckpointSigningAuthorityAdmissionContextV1<
        '_,
    >,
    a2a: Option<&mut ResidentProcessA2aTaskRestartInput<'_>>,
    admission: &mut OperationControlRecoveryAdmission<'_>,
) -> Result<PairwiseA2aRestartClassification, ResidentProcessOperationRestartError> {
    let operation = control.operation().operation();
    let Some(a2a) = a2a else {
        return if has_indexed_a2a_send_preparation(
            &history,
            control.observation(),
            custody,
            maximum_record_bytes,
        )? {
            Err(ResidentProcessOperationRestartError::A2aAdmissionUnavailable)
        } else {
            Ok(PairwiseA2aRestartClassification::NoRetainedSend)
        };
    };
    let send = match probe_a2a_send_custody_for_known_task(
        A2aSendRecoveryInput {
            history,
            recovered: control.observation(),
            limits: a2a.send,
        },
        custody,
        anchor,
        A2aSendRecoveryAdmission {
            admit_checkpoint: &mut *admission.admit_checkpoint,
            admit_route: &mut *a2a.remote.admit_route,
        },
    ) {
        A2aSendCustodyProbe::NoRetainedA2aSend => {
            return Ok(PairwiseA2aRestartClassification::NoRetainedSend);
        }
        A2aSendCustodyProbe::ExactRetainedA2aSend(send) => send,
        A2aSendCustodyProbe::UntrustedOrIncompleteA2aEvidence(error) => return Err(error.into()),
    };
    let candidates = discover_a2a_task_bindings(
        store,
        operation,
        None,
        a2a.maximum_bindings,
        a2a.maximum_binding_bytes,
        custody,
    )?;
    if candidates.continuation.is_some() {
        return Err(ResidentProcessOperationRestartError::A2aTaskCandidatesIncomplete);
    }
    if candidates.bindings.is_empty() {
        return Ok(PairwiseA2aRestartClassification::ExactSendWithoutTask);
    }

    let mut matched = None;
    for candidate in candidates.bindings {
        let recovered = candidate.recover(
            A2aTaskRecoveryInput {
                store,
                operation,
                task_binding_id: candidate.task_binding_id(),
                observation: A2aTaskObservationSelection::SelectedHead,
                checkpoint_authority,
                local_limits: local_limits.observation,
                checkpoint_limits: local_limits.checkpoints,
                membership_limits: local_limits.membership,
                limits: a2a.task,
            },
            custody,
            anchor,
            OperationObservationRecoveryAdmission {
                admit_checkpoint: &mut *admission.admit_checkpoint,
                resolve_signer: &mut *admission.resolve_observation_signer,
                admit_state: &mut *admission.admit_observation_state,
                admit_rotation: &mut *admission.admit_rotation,
                verify_signature: &mut *admission.verify_observation_signature,
            },
            &mut *admission.verify_checkpoint_signature,
            A2aTaskRecoveryAdmission {
                admit_route: &mut *a2a.remote.admit_route,
                resolve_binding_signer: &mut *a2a.remote.resolve_binding_signer,
                admit_binding_evidence: &mut *a2a.remote.admit_binding_evidence,
                verify_binding_signature: &mut *a2a.remote.verify_binding_signature,
                resolve_observation_signer: &mut *a2a.remote.resolve_observation_signer,
                admit_observation_evidence: &mut *a2a.remote.admit_observation_evidence,
                admit_rotation: &mut *a2a.remote.admit_rotation,
                verify_observation_signature: &mut *a2a.remote.verify_observation_signature,
            },
        )?;
        if !send.matches_binding(recovered.binding().1) || matched.is_some() {
            return Err(ResidentProcessOperationRestartError::A2aTaskCandidatesConflict);
        }
        matched = Some(Box::new(recovered));
    }
    matched
        .map(PairwiseA2aRestartClassification::ExactTask)
        .ok_or(ResidentProcessOperationRestartError::A2aTaskCandidatesConflict)
}

pub(super) fn provider_resolution_matches_generation(
    generation: &crate::resident::AdmittedCompiledGeneration,
    issued: &crate::resident::IssuedLocalOperation,
    lease: &OperationLeaseV1,
    selected: &ProviderBindingRefV1,
    participant: Option<&ParticipantRefV1>,
) -> bool {
    let Some(provider) = generation.provider_binding(&selected.provider_binding_id) else {
        return false;
    };
    if provider.binding_identity != selected.provider_binding_identity
        || participant.is_some_and(|participant| &provider.participant != participant)
    {
        return false;
    }
    let binding = issued.handle().1.request_binding();
    let Some(operation) = generation.operation(
        issued.historical_session_lease().repository_id(),
        lease.operation(),
    ) else {
        return false;
    };
    operation
        .capabilities()
        .iter()
        .filter(|requirement| &requirement.capability_id == binding.capability_id())
        .any(|requirement| {
            let Some(capability) = requirement.provider.binding() else {
                return false;
            };
            if requirement.effect_class != EffectClass::Compute
                || capability.effect_class != EffectClass::Compute
            {
                return false;
            }
            let CapabilityProviderBinding::HostNative { provider_tools } =
                &capability.implementation
            else {
                return false;
            };
            provider_tools
                .iter()
                .any(|row| &row.provider_binding == selected && !row.tools.is_empty())
        })
}

enum A2aCancelTaskRestartClassification {
    LocalCancellationRequest,
    RecoveryOnly,
    A2aCancelTaskRequest {
        task: Box<RecoveredA2aTaskHistory>,
        request_evidence: OperationA2aEvidence,
        request_id: A2aJsonRpcRequestIdV1,
    },
}

#[allow(clippy::too_many_arguments)]
fn classify_a2a_cancel_task_request(
    control: &RecoveredOperationControl,
    request: &ImmutableRecordReferenceV1,
    owner: &OperationAttemptOwner,
    history: OperationAttemptHistoryInput<'_>,
    store: &AuthorityStoreCoordinate,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    maximum_record_bytes: usize,
    local_limits: ResidentProcessRestartReadLimits,
    checkpoint_authority: &pale_core::records::ControlCheckpointSigningAuthorityAdmissionContextV1<
        '_,
    >,
    a2a: Option<&mut ResidentProcessA2aCancelTaskRestartInput<'_>>,
    admission: &mut OperationControlRecoveryAdmission<'_>,
) -> A2aCancelTaskRestartClassification {
    let recovered = control.observation();
    let Ok(digest) = pale_ports::ControlRecordDigest::parse(request.record_sha256.as_str()) else {
        return A2aCancelTaskRestartClassification::RecoveryOnly;
    };
    let coordinate = ControlRecordCoordinate::new(request.record_ref.clone(), digest);
    let head_checkpoint = recovered.selected_head_checkpoint_record().1;
    let Ok(evidence_key) = a2a_evidence_key(store, head_checkpoint, &coordinate) else {
        return A2aCancelTaskRestartClassification::RecoveryOnly;
    };
    if head_checkpoint
        .changed_leaf_keys()
        .binary_search(&evidence_key)
        .is_err()
    {
        return match control.recover_local_cancellation_request(
            custody,
            maximum_record_bytes,
            |reference, wire| (admission.admit_request)(reference, wire),
        ) {
            Ok(_) => A2aCancelTaskRestartClassification::LocalCancellationRequest,
            Err(_) => A2aCancelTaskRestartClassification::RecoveryOnly,
        };
    }

    let Some(a2a) = a2a else {
        return A2aCancelTaskRestartClassification::RecoveryOnly;
    };
    if !matches!(owner, OperationAttemptOwner::Provider) {
        return A2aCancelTaskRestartClassification::RecoveryOnly;
    }
    let evidence =
        match custody.read_a2a_evidence(store, &coordinate, a2a.maximum_request_bytes.get()) {
            Ok(Some(evidence))
                if evidence.coordinate() == &coordinate
                    && evidence.sse().is_none()
                    && evidence.byte_len() <= a2a.maximum_request_bytes.get() =>
            {
                evidence
            }
            _ => return A2aCancelTaskRestartClassification::RecoveryOnly,
        };
    let current_checkpoint = recovered.current_checkpoint().1;
    let Ok(projected) = a2a_evidence_leaf(store, current_checkpoint, &evidence) else {
        return A2aCancelTaskRestartClassification::RecoveryOnly;
    };
    let Ok(leaf) = AccumulatorMembershipLeaf::new(projected.hashed_key, projected.value) else {
        return A2aCancelTaskRestartClassification::RecoveryOnly;
    };
    let operation = control.operation().operation();
    let checkpoint_selection = operation.checkpoint_selection();
    if recovered
        .verify_current_checkpoint_membership(
            &checkpoint_selection,
            vec![leaf],
            local_limits.membership,
            custody,
            anchor,
        )
        .is_err()
    {
        return A2aCancelTaskRestartClassification::RecoveryOnly;
    }

    let send = match recover_a2a_send_preparation_under_cancellation_request(
        A2aSendRecoveryInput {
            history,
            recovered,
            limits: a2a.send,
        },
        custody,
        anchor,
        A2aSendRecoveryAdmission {
            admit_checkpoint: &mut *admission.admit_checkpoint,
            admit_route: &mut *a2a.remote.admit_route,
        },
    ) {
        Ok(send) => send,
        Err(_) => return A2aCancelTaskRestartClassification::RecoveryOnly,
    };

    let candidates = match discover_a2a_task_bindings(
        store,
        operation,
        None,
        a2a.maximum_bindings,
        a2a.maximum_binding_bytes,
        custody,
    ) {
        Ok(candidates) if candidates.continuation.is_none() && !candidates.bindings.is_empty() => {
            candidates.bindings
        }
        _ => return A2aCancelTaskRestartClassification::RecoveryOnly,
    };

    let (cancel_ref, cancel_observation) = recovered.selected_head();
    let (cancellation_predecessor_ref, cancellation_predecessor) = recovered.selected();
    let mut qualified = None;
    for candidate in candidates {
        let recovered = candidate.recover(
            A2aTaskRecoveryInput {
                store,
                operation,
                task_binding_id: candidate.task_binding_id(),
                observation: A2aTaskObservationSelection::SelectedHead,
                checkpoint_authority,
                local_limits: local_limits.observation,
                checkpoint_limits: local_limits.checkpoints,
                membership_limits: local_limits.membership,
                limits: a2a.task,
            },
            custody,
            anchor,
            OperationObservationRecoveryAdmission {
                admit_checkpoint: &mut *admission.admit_checkpoint,
                resolve_signer: &mut *admission.resolve_observation_signer,
                admit_state: &mut *admission.admit_observation_state,
                admit_rotation: &mut *admission.admit_rotation,
                verify_signature: &mut *admission.verify_observation_signature,
            },
            &mut *admission.verify_checkpoint_signature,
            A2aTaskRecoveryAdmission {
                admit_route: &mut *a2a.remote.admit_route,
                resolve_binding_signer: &mut *a2a.remote.resolve_binding_signer,
                admit_binding_evidence: &mut *a2a.remote.admit_binding_evidence,
                verify_binding_signature: &mut *a2a.remote.verify_binding_signature,
                resolve_observation_signer: &mut *a2a.remote.resolve_observation_signer,
                admit_observation_evidence: &mut *a2a.remote.admit_observation_evidence,
                admit_rotation: &mut *a2a.remote.admit_rotation,
                verify_observation_signature: &mut *a2a.remote.verify_observation_signature,
            },
        );
        let Ok(recovered) = recovered else {
            return A2aCancelTaskRestartClassification::RecoveryOnly;
        };
        if !send.matches_binding(recovered.binding().1) {
            continue;
        }
        let recovered_head = recovered.local_observation().selected_head();
        let Some((recovered_cancellation_predecessor_ref, recovered_cancellation_predecessor)) =
            recovered.local_observation().selected_predecessor()
        else {
            return A2aCancelTaskRestartClassification::RecoveryOnly;
        };
        if recovered_head.0 != cancel_ref
            || recovered_head.1.canonical_operation_observation_sha256()
                != cancel_observation.canonical_operation_observation_sha256()
            || recovered_cancellation_predecessor_ref != cancellation_predecessor_ref
            || recovered_cancellation_predecessor.canonical_operation_observation_sha256()
                != cancellation_predecessor.canonical_operation_observation_sha256()
        {
            return A2aCancelTaskRestartClassification::RecoveryOnly;
        }
        let request_id = match a2a.protocol.read_cancel_task_request(
            evidence.bytes(),
            recovered.binding().1.remote_task_id(),
            a2a.maximum_request_bytes,
        ) {
            Ok(request_id) => request_id,
            Err(_) => {
                return A2aCancelTaskRestartClassification::RecoveryOnly;
            }
        };
        if qualified.is_some() {
            return A2aCancelTaskRestartClassification::RecoveryOnly;
        }
        qualified = Some((Box::new(recovered), evidence.clone(), request_id));
    }
    qualified.map_or(
        A2aCancelTaskRestartClassification::RecoveryOnly,
        |(task, request_evidence, request_id)| {
            A2aCancelTaskRestartClassification::A2aCancelTaskRequest {
                task,
                request_evidence,
                request_id,
            }
        },
    )
}

fn verify_start_evidence(
    expected: &OperationStartRecoveryEvidence,
    custody: &mut dyn OperationCustodyPort,
    store: &AuthorityStoreCoordinate,
    request: &CommonRequestEnvelopeV1,
    maximum_record_bytes: usize,
) -> Result<(), ResidentProcessOperationRestartError> {
    let reread = read_operation_start_for_request(custody, store, request, maximum_record_bytes)?
        .ok_or(ResidentProcessOperationRestartError::StartEvidence)?;
    if same_start_evidence(expected, &reread) {
        Ok(())
    } else {
        Err(ResidentProcessOperationRestartError::StartEvidence)
    }
}

#[allow(clippy::too_many_arguments)]
fn recover_control(
    input: &ResidentProcessOperationRestartInput<'_>,
    expected_handle: &ControlRecordCoordinate,
    observation: OperationObservationSelection<'_>,
    limits: ResidentProcessRestartReadLimits,
    custody: &mut dyn OperationCustodyPort,
    sessions: &dyn SessionLeaseCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admission: &mut OperationControlRecoveryAdmission<'_>,
) -> Result<RecoveredOperationControl, OperationControlRecoveryError> {
    recover_operation_control(
        OperationControlRecoveryInput {
            operation: LocalOperationHandleRecoveryInput {
                context: input.operation,
                expected_handle,
                maximum_parent_depth: input.maximum_parent_depth,
            },
            observation,
            observation_limits: limits.observation,
            checkpoint_limits: limits.checkpoints,
            membership_limits: limits.membership,
        },
        custody,
        sessions,
        anchor,
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
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn recover_pairwise_child_control(
    input: &ResidentProcessOperationRestartInput<'_>,
    expected_handle: &ControlRecordCoordinate,
    observation: OperationObservationSelection<'_>,
    limits: ResidentProcessRestartReadLimits,
    custody: &mut dyn OperationCustodyPort,
    sessions: &dyn SessionLeaseCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admission: &mut OperationControlRecoveryAdmission<'_>,
    parent: &ResidentProcessRecoveredOperationStart,
) -> Result<RecoveredOperationControl, OperationControlRecoveryError> {
    let parent_operation = parent.control.operation().operation();
    recover_child_operation_control(
        ChildOperationControlRecoveryInput {
            operation: LocalChildOperationHandleRecoveryInput {
                child: LocalChildOperationRecoveryContext {
                    context: input.operation,
                    parent: parent_operation,
                    parent_lease: parent_operation.historical_operation_lease(),
                },
                expected_handle,
            },
            observation,
            observation_limits: limits.observation,
            checkpoint_limits: limits.checkpoints,
            membership_limits: limits.membership,
        },
        custody,
        sessions,
        anchor,
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
        },
    )
}

fn same_start_evidence(
    expected: &OperationStartRecoveryEvidence,
    actual: &OperationStartRecoveryEvidence,
) -> bool {
    expected.store() == actual.store()
        && expected.request() == actual.request()
        && expected.handle() == actual.handle()
        && expected.initial_observation() == actual.initial_observation()
        && expected.checkpoint() == actual.checkpoint()
}

fn selection_coordinate(
    evidence: &pale_core::records::ImmutableRecordReferenceV1,
) -> Result<ControlRecordCoordinate, ResidentProcessOperationRestartError> {
    let digest = pale_ports::ControlRecordDigest::parse(evidence.record_sha256.as_str())
        .map_err(|_| ResidentProcessOperationRestartError::AttemptSelection)?;
    Ok(ControlRecordCoordinate::new(
        evidence.record_ref.clone(),
        digest,
    ))
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/restart_support.rs"]
mod test_support;
