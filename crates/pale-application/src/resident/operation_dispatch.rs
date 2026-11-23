//! Prepare an operation attempt and run eligible built-in actions.
//!
//! Preparation pins the selected implementation and result authority
//! to the original request. Its private record is prepared for retention with
//! the signed `DispatchPrepared` observation through the existing checkpoint
//! and custody path. External attempts also retain their selected provider.
//! Preparation alone does not permit invocation. Built-in actions require a
//! separate committed dispatch and fresh authority checks; contextual and
//! external-provider execution remain separate paths.

use pale_core::records::*;
use pale_ports::{ControlRecord, OperationCustodyRecord};

use super::{
    prepare_operation_observation_append, AdmittedAmbientSession, AdmittedCompiledGeneration,
    AmbientSessionAdmissionContext, AmbientSessionAdmissionError, AmbientSessionAuthorityPort,
    IssuedLocalOperation, OperationObservationAuthorityStoreResolver,
    OperationObservationCustodyProjectionError, OperationObservationTransition,
    PreparedOperationObservationAppend,
};
use crate::provider_invocation::AdmittedProviderInvocationSuccess;

mod a2a;
mod checkpoint_authority;
mod custody;
pub use a2a::{
    commit_a2a_send_observation, launch_a2a_delegate, prepare_a2a_send,
    recover_a2a_send_outcome_unknown, retain_a2a_outcome, A2aDelegateLaunchFailure,
    A2aDelegateLaunchInput, A2aFailedSendObservationInput, A2aInitialResponseFacts,
    A2aInitialTaskCustodyInput, A2aNoTaskObservationFailure, A2aNoTaskOutcomeRetention,
    A2aNotStartedObservationFailure, A2aNotStartedOutcomeRetention, A2aOutcomeRetentionFailure,
    A2aOutcomeRetentionInput, A2aOutcomeUnknownObservationFailure,
    A2aOutcomeUnknownObservationInput, A2aRecoveredUnknownOutcomeRetention, A2aSendCheckpointError,
    A2aSendCustodyFailure, A2aSendDiagnosticFailure, A2aSendDispatch, A2aSendInput,
    A2aSendNotStarted, A2aSendObservationCommitFailure, A2aSendObservationCommitInput,
    A2aSendOutcomeUnknown, A2aSendPreparationError, A2aSendRecoveryAdmission, A2aSendRecoveryError,
    A2aSendRecoveryInput, A2aSendRecoveryLimits, A2aSendReleaseError, A2aSendResponseFailure,
    A2aSendResponsePoll, A2aSendStartFailure, A2aTaskOutcomeRetention, A2aUnknownOutcomeRetention,
    AwaitingA2aDelegate, AwaitingA2aSendResponse, CompletedA2aNoTaskResponse,
    CompletedA2aSendObservationCustody, CompletedA2aSendResponse, CompletedA2aTaskResponse,
    DatabaseCommittedA2aSendAnchorPending, InFlightA2aSend, PreparedA2aSend,
    PreparedA2aSendCheckpoint, PreparedA2aSendCustody, PreparedA2aSendObservation,
    PreparedRecoveredA2aSendObservation, ReceivedA2aSendResponse, RecoveredA2aSendDiagnostic,
    RecoveredA2aSendObservationFailure, RecoveredA2aSendOutcomeUnknown, RetainedA2aOutcome,
    RetainedA2aSend,
};
pub(in crate::resident) use a2a::{
    has_indexed_a2a_send_preparation, probe_a2a_send_custody_for_known_task,
    recover_a2a_send_preparation_under_cancellation_request, retain_a2a_outcome_continuable,
    A2aOutcomeRetentionKind, A2aSendCustodyProbe, ContinuableA2aOutcomeRetentionFailure,
};
pub(crate) use custody::sample_time;
mod context_binding;
pub(in crate::resident) mod hook_response;
pub use hook_response::{
    HookResponseCheckpointSigner, HookResponseObservationSigner, HookResponseRotation,
    NativeHookResponseInputs,
};
mod native;
pub use context_binding::{
    prepare_operation_context_binding, CompletedOperationContextBinding,
    OperationContextBindingInput, PreparedOperationContextBinding,
};
pub(in crate::resident) use native::{
    recover_native_operation_preparation, IssuedNativeOperationStageCheckpoint,
    NativeOperationStagePreparationContinuation, PreparedNativeOperationStageCustody,
};
mod prepared_recovery;
pub(in crate::resident) mod provider;
pub(in crate::resident) use prepared_recovery::{
    CompletedRecoveredDispatchCustody, IssuedRecoveredDispatchCheckpoint,
    PreparedRecoveredDispatchCheckpoint, PreparedRecoveredOperationDispatch,
    RecoveredDispatchAnchorPending, RecoveredDispatchCommitFailure,
    RecoveredDispatchCustodyFailure,
};
pub use provider::{
    continue_consumed_provider_recording, invoke_provider_operation, prepare_provider_resolution,
    record_consumed_provider_operation, retain_provider_operation_dispatch,
    CompletedProviderOperation, ConsumedProviderOperation, ConsumedProviderRecordingContinuation,
    PreparedProviderOperationAttempt, PreparedProviderOperationDispatch,
    PreparedProviderResolution, ProviderOperationDispatchContext, ProviderOperationDispatchFailure,
    ProviderOperationDispatchInput, ProviderOperationInput, ProviderOperationInvokeFailure,
    ProviderOperationInvokeInput, ProviderOperationInvokeResult,
    ProviderOperationPreparationFailure, ProviderOperationRecordingConflict,
    ProviderOperationRecordingCustody, ProviderOperationRecordingFailure,
    ProviderOperationRecordingFinalization, ProviderOperationRecordingFinalizationFailure,
    ProviderOperationRecordingPreparationError, ProviderOperationRecordingRecoveryFailure,
    ProviderOperationRetentionFailure, ProviderOperationUncertainty,
    RetainedProviderOperationDispatch, UncertainProviderOperation,
};
pub(in crate::resident) use provider::{
    continue_issued_recovered_provider_unknown, continue_prepared_recovered_provider_unknown,
    invoke_owned_provider_invocation, prepare_pairwise_delegate_provider_call,
    prepare_provider_operation_call, prepare_recovered_provider_attempt,
    prepare_recovered_provider_completion, prepare_recovered_provider_operation_call,
    record_provider_operation_completion, record_recovered_provider_operation_completion,
    record_recovered_provider_operation_unknown,
    recover_provider_dispatch_under_cancellation_request, recover_provider_operation_preparation,
    retain_recovered_provider_attempt, validate_recovered_provider_call_references,
    validate_recovered_provider_unknown, CompletedRecoveredProviderAttempt,
    IssuedRecoveredProviderUnknownCheckpoint, OwnedProviderInvocationRequest,
    PreparedProviderOperationCall, PreparedRecoveredProviderAttempt,
    PreparedRecoveredProviderDispatch, PreparedRecoveredProviderOperationCall,
    PreparedRecoveredProviderUnknown, ProviderOperationCallBasis,
    ProviderOperationContinuationCoordinate, RecoveredProviderAttemptAnchorPending,
    RecoveredProviderAttemptCommitFailure, RecoveredProviderAttemptCustodyFailure,
    RecoveredProviderAttemptInput, RecoveredProviderAttemptMeaning,
    RecoveredProviderCallPreparationFailure, RecoveredProviderDispatchRelease,
    RecoveredProviderOperationDispatch, RecoveredProviderUnknownRecordingFailure,
};
mod result;
use result::RecoveredNativeAttempt;
pub(in crate::resident) use result::{OperationPreparationHistoryInput, RecoveredProviderAttempt};
mod recovery;
pub(in crate::resident) use recovery::{
    verify_operation_head_custody, verify_recovered_operation_head_custody,
};
pub(in crate::resident) mod selection;
pub(in crate::resident) mod signing;
pub(in crate::resident) use custody::{
    commit_prepared_checkpoint, complete_prepared_checkpoint,
    continue_owned_operation_observation_custody, observation_checkpoint_preimage,
    retain_owned_operation_observation, OwnedOperationObservationCustodyFailure,
};
pub use custody::{OperationAppendFailure, OperationCheckpointInput};
pub(in crate::resident) use native::{
    execute_native_operation_work, prepare_native_operation_work, record_native_operation_outcome,
    record_recovered_native_operation_outcome, recover_native_operation_outcome_phase,
    NativeOperationOutcomeRecordingInput, NativeOperationRecordingAuthority,
    PreparedNativeOperationWork,
};
pub use native::{
    invoke_native_operation, prepare_native_operation_attempt, recover_native_operation_dispatch,
    recover_native_operation_outcome, AdmittedStagedNativeOperationOutcome,
    CompletedNativeOperation, ExecutedNativeOperation, FailedNativeOperation,
    NativeOperationAttemptInput, NativeOperationCancellationInput,
    NativeOperationCancellationRefusal, NativeOperationCompletionInput,
    NativeOperationDispatchContext, NativeOperationExecutionContext, NativeOperationExpiryInput,
    NativeOperationExpiryRefusal, NativeOperationFailureInput, NativeOperationInvocationRefusal,
    NativeOperationInvokeFailure, NativeOperationInvokeInput, NativeOperationOutcome,
    NativeOperationOutcomeRecoveryInput, NativeOperationOutcomeStageInput,
    NativeOperationOutcomeStagePreparationFailure, NativeOperationRecordingFailure,
    NativeOperationRecoveryContext, NativeOperationRefusalCancellationInput,
    NativeOperationStageFinalization, NativeOperationStageFinalizationFailure,
    NativeOperationStagingAnchorPending, NativeOperationStagingCommitFailure,
    NativeOperationTerminalRecordingInput, PreparedNativeOperationAttempt,
    PreparedNativeOperationCancellation, PreparedNativeOperationCompletion,
    PreparedNativeOperationDispatch, PreparedNativeOperationExpiry, PreparedNativeOperationFailure,
    PreparedNativeOperationOutcomeStage, RecoveredNativeOperationDispatch,
    RecoveredNativeOperationOutcome, StagedNativeOperationOutcome, UnstagedNativeOperationOutcome,
};
pub(in crate::resident) use result::OperationAttemptOwner;
pub use result::{
    admit_native_operation_failure, admit_native_operation_result,
    validate_operation_attempt_result, NativeOperationFailureEvidenceInput,
    NativeOperationResultInput, NativeOperationTerminalEvidence, OperationAttemptHistoryInput,
    OperationAttemptResultInput,
};
use selection::{AttemptSelection, AttemptSelectionContext};

/// Original operation, current authority and observation inputs for preparation.
#[derive(Clone)]
pub struct OperationAttemptContext<'a> {
    /// Locator exposed after durable start or protected recovery.
    pub operation: &'a IssuedLocalOperation,
    /// Original admitted request, not a reconstructed request binding.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Existing admitted session; preparation rechecks its current authority.
    pub session: &'a AdmittedAmbientSession,
    /// Current, independently resolved session authority and generation.
    pub current: &'a AmbientSessionAdmissionContext<'a>,
    /// Existing child lease named by the operation handle.
    pub lease: &'a OperationLeaseV1,
    /// Completed context custody when this lease retains evidence obligations.
    pub context_binding: Option<&'a CompletedOperationContextBinding<'a>>,
    /// Exact retained compiled definitions used to select this implementation.
    pub generation: &'a AdmittedCompiledGeneration,
    /// Fresh immutable reference for the private preparation record.
    pub selection_ref: ResidentRecordRefV1,
    /// Maximum encoded preparation bytes, checked before signing.
    pub maximum_selection_bytes: usize,
    /// Fresh reference for the signed preparation observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Exact admitted predecessor and observation-signing context.
    pub observation: OperationObservationAdmissionContextV1<'a>,
}

/// Attempt preparation, built-in execution or result comparison failed.
#[derive(Debug, thiserror::Error)]
pub enum OperationAttemptError {
    /// A diagnostic could not be constructed or admitted under its existing contract.
    #[error(transparent)]
    Diagnostic(#[from] crate::diagnostic::DiagnosticAdmissionError),
    /// Current session or child authority is no longer usable.
    #[error(transparent)]
    Session(#[from] AmbientSessionAdmissionError),
    /// The supplied data does not describe this operation's selected attempt.
    #[error("operation attempt does not match its retained request or implementation")]
    Association,
    /// The private record exceeds its selected byte limit.
    #[error("operation attempt record exceeds its byte limit")]
    BoundExceeded,
    /// The private record or its source-derived encoding identity is invalid.
    #[error("operation attempt record or encoding identity is invalid")]
    Encoding,
    /// The existing observation owner rejected lineage, time, authority or signing.
    #[error(transparent)]
    Admission(#[from] InvalidResidentRecordV1),
    /// The existing native-action owner rejected the input or its schema.
    #[error(transparent)]
    NativeAction(#[from] crate::actions::NativeActionInvocationError),
    /// The selected operation retains evidence obligations not resolved by this route.
    #[error("operation requires admitted context before dispatch")]
    ContextRequired,
    /// Context binding was requested for an operation with no evidence obligations.
    #[error("operation has no evidence obligations to bind")]
    ContextNotRequired,
    /// Observation projection or checkpoint attachment inputs did not agree.
    #[error(transparent)]
    Projection(#[from] OperationObservationCustodyProjectionError),
    /// Exact new dispatch custody or its current storage evidence was not proved.
    #[error(transparent)]
    Custody(#[from] super::OperationDispatchCustodyError),
    /// The selected clock could not supply a representable record time.
    #[error(transparent)]
    Clock(#[from] pale_ports::WallClockSourceError),
    /// Admission, dispatch, execution and recording times are out of order.
    #[error("operation timestamps do not preserve their required order")]
    TimeOrder,
    /// The recording time exceeds the operation handle's retention period.
    #[error("operation observation is outside its handle retention period")]
    RetentionElapsed,
    /// Pre-dispatch work exhausted this call's operation timeout.
    #[error("operation timeout elapsed before the retained handler could run")]
    Deadline,
}

impl From<selection::AttemptSelectionError> for OperationAttemptError {
    fn from(error: selection::AttemptSelectionError) -> Self {
        use selection::AttemptSelectionError;
        match error {
            AttemptSelectionError::BoundExceeded => Self::BoundExceeded,
            AttemptSelectionError::Encoding => Self::Encoding,
            AttemptSelectionError::Association
            | AttemptSelectionError::Implementation
            | AttemptSelectionError::Result => Self::Association,
            AttemptSelectionError::NativeAction(error) => Self::NativeAction(error),
        }
    }
}

/// Signed preparation and private selection awaiting joint checkpoint custody.
/// Dropping this value has no effect. It provides no provider-call method.
#[must_use = "retain preparation with its observation or discard both"]
pub(super) struct PreparedOperationAttempt<'a> {
    operation: &'a IssuedLocalOperation,
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    selection: ControlRecord,
}

/// One signed operation successor detached from its borrowed preparation inputs.
///
/// This private continuation retains only immutable operation evidence needed
/// to finish checkpoint custody. It carries no handler, provider port, current
/// authority, or permission to select or execute another implementation.
#[must_use = "retain this exact signed successor or discard it"]
pub(in crate::resident) struct PreparedOwnedOperationObservation {
    operation: IssuedLocalOperation,
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    associated_record: Option<OperationCustodyRecord>,
}

impl PreparedOwnedOperationObservation {
    pub(super) fn from_attempt(attempt: PreparedOperationAttempt<'_>) -> Self {
        Self {
            operation: attempt.operation.clone_for_recovery_continuation(),
            predecessor: attempt.predecessor,
            observation_ref: attempt.observation_ref,
            observation: attempt.observation,
            associated_record: Some(OperationCustodyRecord::AttemptSelection(attempt.selection)),
        }
    }

    pub(super) fn from_transition(
        operation: &IssuedLocalOperation,
        predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
        observation_ref: ResidentRecordRefV1,
        observation: OperationObservationV1,
    ) -> Self {
        Self {
            operation: operation.clone_for_recovery_continuation(),
            predecessor: (predecessor.0.clone(), predecessor.1.clone()),
            observation_ref,
            observation,
            associated_record: None,
        }
    }

    pub(super) fn from_expiry(
        operation: &IssuedLocalOperation,
        predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
        observation_ref: ResidentRecordRefV1,
        observation: OperationObservationV1,
        diagnostic: ControlRecord,
    ) -> Self {
        Self {
            operation: operation.clone_for_recovery_continuation(),
            predecessor: (predecessor.0.clone(), predecessor.1.clone()),
            observation_ref,
            observation,
            associated_record: Some(OperationCustodyRecord::Diagnostic(diagnostic)),
        }
    }

    pub(in crate::resident) const fn operation(&self) -> &IssuedLocalOperation {
        &self.operation
    }

    pub(in crate::resident) fn prepare_append(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<PreparedOperationObservationAppend<'_>, OperationObservationCustodyProjectionError>
    {
        let (handle_ref, handle) = self.operation.handle();
        let append = prepare_operation_observation_append(
            OperationObservationTransition::new(
                handle_ref,
                handle,
                &self.predecessor.0,
                &self.predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?;
        Ok(match &self.associated_record {
            Some(record) => append.with_associated_record(record.clone()),
            None => append,
        })
    }
}

impl PreparedOperationAttempt<'_> {
    /// Borrow the signed preparation observation; it is not yet committed.
    #[must_use]
    pub(super) const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Prepare one atomic append containing the observation and its selection.
    /// The existing checkpoint producer and independent anchor remain required.
    ///
    /// # Errors
    /// Rejects inconsistent lineage, authority-owned storage or record projection.
    /// No database write, anchor advancement or provider call occurs here.
    pub(super) fn prepare_append(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<PreparedOperationObservationAppend<'_>, OperationObservationCustodyProjectionError>
    {
        let (handle_ref, handle) = self.operation.handle();
        let append = prepare_operation_observation_append(
            OperationObservationTransition::new(
                handle_ref,
                handle,
                &self.predecessor.0,
                &self.predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?;
        Ok(
            append.with_associated_record(OperationCustodyRecord::AttemptSelection(
                self.selection.clone(),
            )),
        )
    }
}

impl<'a> OperationAttemptContext<'a> {
    fn require_context_ready(&self) -> Result<(), OperationAttemptError> {
        let predecessor = self
            .observation
            .predecessor
            .ok_or(OperationAttemptError::Association)?;
        match (
            self.lease.evidence_obligations().is_empty(),
            predecessor.1.state(),
        ) {
            (
                true,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
            ) => Ok(()),
            (
                false,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ContextBound { .. },
                },
            ) if self
                .context_binding
                .is_some_and(|binding| binding.matches(self.operation, predecessor)) =>
            {
                Ok(())
            }
            (
                false,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
            ) => Err(OperationAttemptError::ContextRequired),
            (false, _) => Err(OperationAttemptError::Association),
            (true, _) => Err(OperationAttemptError::Association),
        }
    }

    fn parents(&self) -> AttemptSelectionContext<'a> {
        AttemptSelectionContext {
            operation: self.operation,
            request_ref: self.operation.request_coordinate().record_ref(),
            request: self.request,
            session: self.session.lease(),
            lease: self.lease,
            generation: self.generation,
        }
    }

    fn validate_current_at(
        &self,
        current: &AmbientSessionAdmissionContext<'_>,
        authority: &mut impl AmbientSessionAuthorityPort,
    ) -> Result<(), OperationAttemptError> {
        validate_operation_continuation_at(
            self.operation,
            self.request,
            self.session,
            self.lease,
            &self.observation,
            current,
            authority,
        )?;
        self.validate_attempt_references()
    }

    fn validate_time_at(
        &self,
        current: &AmbientSessionAdmissionContext<'_>,
    ) -> Result<(), OperationAttemptError> {
        self.validate_original_session()?;
        self.session.revalidate_time(current)?;
        self.validate_operation_time(current.now)
    }

    fn validate_original_session(&self) -> Result<(), OperationAttemptError> {
        if !self.operation.same_session(self.session) {
            return Err(AmbientSessionAdmissionError::SessionAuthorityChanged.into());
        }
        Ok(())
    }

    fn validate_operation_time(&self, now: UnixTimeV1) -> Result<(), OperationAttemptError> {
        if now >= self.lease.expires_at()
            || now >= *self.operation.handle().1.expires_at()
            || now < self.request.issued_at()
            || now >= self.request.expires_at()
        {
            return Err(OperationAttemptError::Association);
        }
        Ok(())
    }

    fn has_temporal_refusal(&self, current: &AmbientSessionAdmissionContext<'_>) -> bool {
        self.session.has_temporal_refusal(current)
            || self.validate_operation_time(current.now).is_err()
    }

    fn validate_attempt_references(&self) -> Result<(), OperationAttemptError> {
        let (handle_ref, handle) = self.operation.handle();
        let predecessor = self
            .observation
            .predecessor
            .ok_or(OperationAttemptError::Association)?;
        if self.selection_ref == self.observation_ref
            || self.selection_ref == *handle_ref
            || self.selection_ref == *handle.operation_lease_ref()
            || self.selection_ref == *predecessor.0
            || self.observation_ref == *handle_ref
            || self.observation_ref == *handle.operation_lease_ref()
            || self.observation_ref == *predecessor.0
            || self.selection_ref == *self.operation.request_coordinate().record_ref()
            || self.observation_ref == *self.operation.request_coordinate().record_ref()
            || handle.request_binding().session_grant_id() != self.session.grant_id().as_str()
            || handle.request_binding().session_grant_sha256().as_str()
                != self.session.grant_sha256().as_str()
        {
            return Err(OperationAttemptError::Association);
        }
        Ok(())
    }

    fn sign_preparation_at<Rotation, Sign>(
        self,
        selection: AttemptSelection,
        current: &AmbientSessionAdmissionContext<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedOperationAttempt<'a>, OperationAttemptError>
    where
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let predecessor = self
            .observation
            .predecessor
            .ok_or(OperationAttemptError::Association)?;
        self.sign_preparation_after(selection, current, predecessor, admit_rotation, sign)
    }

    fn sign_preparation_after<Rotation, Sign>(
        self,
        selection: AttemptSelection,
        current: &AmbientSessionAdmissionContext<'_>,
        predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedOperationAttempt<'a>, OperationAttemptError>
    where
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let record = selection.record(self.selection_ref, self.maximum_selection_bytes)?;
        let state = OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::DispatchPrepared {
                attempt_id: selection.attempt_id().clone(),
                evidence: ImmutableRecordReferenceV1 {
                    record_ref: record.coordinate().record_ref().clone(),
                    record_sha256: ArtifactDigest::parse(
                        record.coordinate().record_sha256().as_str(),
                    )
                    .map_err(|_| OperationAttemptError::Encoding)?,
                },
            },
        };
        signing::validate_runtime_signer(
            self.observation.signed.signing_authority,
            current,
            OPERATION_OBSERVATION_V1_SCHEMA,
        )?;
        let admission = OperationObservationAdmissionContextV1 {
            signed: self.observation.signed,
            handle_ref: self.observation.handle_ref,
            handle: self.observation.handle,
            predecessor: Some(predecessor),
        };
        let observation =
            signing::issue_observation(&admission, state, current.now, admit_rotation, sign)?;
        Ok(PreparedOperationAttempt {
            operation: self.operation,
            predecessor: (predecessor.0.clone(), predecessor.1.clone()),
            observation_ref: self.observation_ref,
            observation,
            selection: record,
        })
    }
}

fn validate_operation_continuation_at<Authority: AmbientSessionAuthorityPort>(
    operation: &IssuedLocalOperation,
    request: &CommonRequestEnvelopeV1,
    session: &AdmittedAmbientSession,
    lease: &OperationLeaseV1,
    observation: &OperationObservationAdmissionContextV1<'_>,
    current: &AmbientSessionAdmissionContext<'_>,
    authority: &mut Authority,
) -> Result<(), OperationAttemptError> {
    if !operation.same_session(session) {
        return Err(AmbientSessionAdmissionError::SessionAuthorityChanged.into());
    }
    session.revalidate(current, authority)?;
    validate_operation_current_basis(operation, request, session, lease, current)?;
    let (handle_ref, handle) = operation.handle();
    let predecessor = observation
        .predecessor
        .ok_or(OperationAttemptError::Association)?;
    if observation.handle_ref != handle_ref
        || observation.handle.canonical_operation_handle_sha256()
            != handle.canonical_operation_handle_sha256()
        || predecessor.1.operation_handle_ref() != handle_ref
        || predecessor.1.operation_handle_sha256() != handle.canonical_operation_handle_sha256()
        || predecessor.1.operation_handle_id() != handle.operation_handle_id()
    {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

fn validate_operation_current_basis(
    operation: &IssuedLocalOperation,
    request: &CommonRequestEnvelopeV1,
    session: &AdmittedAmbientSession,
    lease: &OperationLeaseV1,
    current: &AmbientSessionAdmissionContext<'_>,
) -> Result<(), OperationAttemptError> {
    lease.verify_against_parent(session.lease())?;
    let (_, handle) = operation.handle();
    let request_binding = request
        .local_request_binding()
        .map_err(|_| OperationAttemptError::Association)?;
    if current.now >= lease.expires_at()
        || current.now >= *handle.expires_at()
        || current.now < request.issued_at()
        || current.now >= request.expires_at()
        || operation.request_coordinate().record_sha256().as_str()
            != request.canonical_request_sha256().as_str()
        || handle.request_binding() != &request_binding
        || handle.operation_lease_sha256() != lease.operation_lease_digest()
        || operation.historical_operation_lease() != lease
        || handle.request_binding().session_grant_id() != session.grant_id().as_str()
        || handle.request_binding().session_grant_sha256().as_str()
            != session.grant_sha256().as_str()
    {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/internal/resident/operation_dispatch/support.rs"]
pub(in crate::resident) mod test_support;
