//! Built-in action preparation, dispatch custody and result issuance.

use super::*;
use crate::actions::{NativeActionRegistry, PreparedNativeAction};
use crate::resident::CompletedOperationObservationCustody;

mod cancellation;
mod execution;
mod recovery;
mod service;
pub use cancellation::{NativeOperationCancellationInput, PreparedNativeOperationCancellation};
pub use execution::{
    recover_native_operation_outcome, ExecutedNativeOperation, FailedNativeOperation,
    NativeOperationCancellationRefusal, NativeOperationCompletionInput,
    NativeOperationExecutionContext, NativeOperationExpiryInput, NativeOperationExpiryRefusal,
    NativeOperationFailureInput, NativeOperationInvocationRefusal, NativeOperationOutcome,
    NativeOperationOutcomeRecoveryInput, NativeOperationOutcomeStageInput,
    NativeOperationOutcomeStagePreparationFailure, NativeOperationRefusalCancellationInput,
    PreparedNativeOperationCompletion, PreparedNativeOperationExpiry,
    PreparedNativeOperationFailure, PreparedNativeOperationOutcomeStage,
    RecoveredNativeOperationOutcome, StagedNativeOperationOutcome, UnstagedNativeOperationOutcome,
};
pub(in crate::resident) use execution::{
    recover_native_operation_outcome_phase, NativeOperationOutcomeStagePreparation,
    NativeOperationRecordingAuthority, RecoveredNativeOperationOutcomePhase,
};
pub(in crate::resident) use recovery::recover_native_operation_preparation;
pub use recovery::{
    recover_native_operation_dispatch, NativeOperationRecoveryContext,
    RecoveredNativeOperationDispatch,
};
pub(in crate::resident) use service::{
    execute_native_operation_work, prepare_native_operation_work, record_native_operation_outcome,
    record_recovered_native_operation_outcome, IssuedNativeOperationStageCheckpoint,
    NativeOperationOutcomeRecordingInput, NativeOperationStagePreparationContinuation,
    PreparedNativeOperationStageCustody, PreparedNativeOperationWork,
};
pub use service::{
    invoke_native_operation, AdmittedStagedNativeOperationOutcome, CompletedNativeOperation,
    NativeOperationInvokeFailure, NativeOperationInvokeInput, NativeOperationRecordingFailure,
    NativeOperationStageFinalization, NativeOperationStageFinalizationFailure,
    NativeOperationStagingAnchorPending, NativeOperationStagingCommitFailure,
    NativeOperationTerminalRecordingInput,
};

// The three diagnostic-producing lifecycle owners share only physical
// projection. Each caller establishes its own cause, permission and state.
fn prepare_diagnostic_observation_append<'a>(
    operation: (&'a ResidentRecordRefV1, &'a OperationHandleV1),
    predecessor: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    diagnostic: &ControlRecord,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
) -> Result<PreparedOperationObservationAppend<'a>, OperationObservationCustodyProjectionError> {
    let (handle_ref, handle) = operation;
    let append = prepare_operation_observation_append(
        OperationObservationTransition::new(
            handle_ref,
            handle,
            predecessor.0,
            predecessor.1,
            observation.0,
            observation.1,
        ),
        resolve_store,
    )?;
    Ok(append.with_associated_record(OperationCustodyRecord::Diagnostic(diagnostic.clone())))
}

/// Source and authority inputs for one built-in Rust action.
pub struct NativeOperationAttemptInput<'operation, 'registry> {
    /// Original operation and current preparation authority.
    pub context: OperationAttemptContext<'operation>,
    /// Existing sealed registry that owns the actual handler and input checks.
    pub registry: &'registry NativeActionRegistry,
    /// Distinct attempt identity selected before preparation.
    pub attempt_id: &'operation CapabilityAttemptIdV1,
    /// Independent outer-result identity, signer and admitted contract set.
    pub result_admission: CapabilitySuccessAdmissionContextV1<'operation>,
}

/// Built-in call retained with its signed, not-yet-committed preparation.
/// No handler runs when this value is created or dropped.
#[must_use = "commit preparation before preparing dispatch, or discard it"]
pub struct PreparedNativeOperationAttempt<'operation, 'registry> {
    context: OperationAttemptContext<'operation>,
    preparation: PreparedOperationAttempt<'operation>,
    selection: AttemptSelection,
    call: PreparedNativeAction<'registry>,
    result_admission: CapabilitySuccessAdmissionContextV1<'operation>,
}

/// Current authority and fresh reference for signing a dispatch observation.
pub struct NativeOperationDispatchContext<'a> {
    /// Exact database-and-anchor completion of this attempt's preparation.
    pub completed: &'a CompletedOperationObservationCustody,
    /// Current session authority, not the preparation-time observation.
    pub current: &'a AmbientSessionAdmissionContext<'a>,
    /// Independently resolved observation schema and runtime signer.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh immutable reference for this dispatch observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Existing owner of the observation's store coordinate.
    pub resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
}

/// Signed dispatch awaiting its own checkpoint and fresh custody confirmation.
/// It retains the exact prepared handler; no replacement input is accepted.
#[must_use = "commit this exact dispatch before invoking the retained action"]
pub struct PreparedNativeOperationDispatch<'operation, 'registry> {
    attempt: PreparedNativeOperationAttempt<'operation, 'registry>,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
}

/// Prepare the exact built-in action selected by the compiled operation.
///
/// Native actions advance from `Resolving` or from an admitted `ContextBound`
/// phase when the lease retains evidence obligations. They do not acquire an
/// external provider identity. The canonical registry checks the signed request body,
/// and the existing result owner checks its selected return schema. No action,
/// database write or anchor advancement occurs here.
///
/// # Errors
/// Rejects stale authority, unlike request or implementation identities, invalid
/// action input, unselected result schemas, oversized preparation or signing failure.
pub fn prepare_native_operation_attempt<'operation, 'registry, Authority, Rotation, Sign>(
    input: NativeOperationAttemptInput<'operation, 'registry>,
    authority: &mut Authority,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<PreparedNativeOperationAttempt<'operation, 'registry>, OperationAttemptError>
where
    Authority: AmbientSessionAuthorityPort,
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
    let current = input.context.current;
    prepare_native_operation_attempt_at(input, current, authority, admit_rotation, sign)
}

fn prepare_native_operation_attempt_at<'operation, 'registry, Authority, Rotation, Sign>(
    input: NativeOperationAttemptInput<'operation, 'registry>,
    current: &AmbientSessionAdmissionContext<'_>,
    authority: &mut Authority,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<PreparedNativeOperationAttempt<'operation, 'registry>, OperationAttemptError>
where
    Authority: AmbientSessionAuthorityPort,
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
    input.context.validate_current_at(current, authority)?;
    input.context.require_context_ready()?;
    signing::validate_runtime_signer(
        input.result_admission.result.signing_authority,
        current,
        CAPABILITY_RESULT_V1_SCHEMA,
    )?;
    let (selection, call) = AttemptSelection::capture_native(
        input.context.parents(),
        input.registry,
        input.attempt_id,
        input.result_admission,
    )?;
    let preparation = input.context.clone().sign_preparation_at(
        selection.clone(),
        current,
        admit_rotation,
        sign,
    )?;
    Ok(PreparedNativeOperationAttempt {
        context: input.context,
        preparation,
        selection,
        call,
        result_admission: input.result_admission,
    })
}

impl<'operation, 'registry> PreparedNativeOperationAttempt<'operation, 'registry> {
    /// Retain only the signed `DispatchPrepared` successor for checkpoint custody.
    ///
    /// The unopened handler is deliberately discarded. A later turn must
    /// recover it from the durably retained selection and cannot use this
    /// conversion as execution authority.
    pub(in crate::resident) fn into_owned_observation(
        self,
    ) -> super::PreparedOwnedOperationObservation {
        super::PreparedOwnedOperationObservation::from_attempt(self.preparation)
    }

    /// Borrow the signed preparation; this is not custody or dispatch evidence.
    #[must_use]
    pub fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.preparation.observation()
    }

    /// Prepare the existing atomic observation/selection append.
    /// # Errors
    /// Preserves the existing observation projection and store-resolution errors.
    pub fn prepare_append(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<PreparedOperationObservationAppend<'_>, OperationObservationCustodyProjectionError>
    {
        self.preparation.prepare_append(resolve_store)
    }

    /// Sign dispatch only after the exact preparation entered checkpoint custody.
    ///
    /// The returned dispatch still needs its own commit and current checks.
    /// The sealed body-only handler cannot consume its unused read authority.
    /// Evidence obligations still require their owner before this route can run.
    ///
    /// # Errors
    /// Rejects unlike custody, expired or changed authority, unresolved context,
    /// reused references, and the existing observation/signing failures.
    pub fn prepare_dispatch<Authority, Rotation, Sign>(
        self,
        input: NativeOperationDispatchContext<'_>,
        authority: &mut Authority,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedNativeOperationDispatch<'operation, 'registry>, OperationAttemptError>
    where
        Authority: AmbientSessionAuthorityPort,
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
        let (observation_ref, observation) =
            self.prepare_dispatch_observation(input, authority, admit_rotation, sign)?;
        Ok(PreparedNativeOperationDispatch {
            attempt: self,
            observation_ref,
            observation,
        })
    }

    fn prepare_dispatch_observation<Authority, Rotation, Sign>(
        &self,
        input: NativeOperationDispatchContext<'_>,
        authority: &mut Authority,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<(ResidentRecordRefV1, OperationObservationV1), OperationAttemptError>
    where
        Authority: AmbientSessionAuthorityPort,
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
        self.context.validate_current_at(input.current, authority)?;
        self.context.require_context_ready()?;
        self.prepare_append(input.resolve_store)?
            .verify_completed_custody(input.completed)?;
        let (handle_ref, handle) = self.context.operation.handle();
        let (prepared_ref, prepared) = self.preparation.observation();
        self.validate_successor_reference(&input.observation_ref)?;
        signing::validate_runtime_signer(
            input.signing.signing_authority,
            input.current,
            OPERATION_OBSERVATION_V1_SCHEMA,
        )?;
        signing::validate_runtime_signer(
            self.selection.result_authority(),
            input.current,
            CAPABILITY_RESULT_V1_SCHEMA,
        )?;
        let OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::DispatchPrepared {
                    attempt_id,
                    evidence,
                },
        } = prepared.state()
        else {
            return Err(OperationAttemptError::Association);
        };
        let observation = signing::issue_observation(
            &OperationObservationAdmissionContextV1 {
                signed: input.signing,
                handle_ref,
                handle,
                predecessor: Some((prepared_ref, prepared)),
            },
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched {
                    attempt_id: attempt_id.clone(),
                    evidence: evidence.clone(),
                },
            },
            input.current.now,
            admit_rotation,
            sign,
        )?;
        Ok((input.observation_ref, observation))
    }

    fn validate_successor_reference(
        &self,
        reference: &ResidentRecordRefV1,
    ) -> Result<(), OperationAttemptError> {
        let (handle_ref, handle) = self.context.operation.handle();
        let (prepared_ref, prepared) = self.preparation.observation();
        if reference == prepared_ref
            || reference == &self.context.selection_ref
            || reference == handle_ref
            || reference == handle.operation_lease_ref()
            || reference == self.context.operation.request_coordinate().record_ref()
            || Some(reference) == prepared.predecessor_ref().as_ref()
        {
            return Err(OperationAttemptError::Association);
        }
        Ok(())
    }
}

impl PreparedNativeOperationDispatch<'_, '_> {
    /// Borrow the signed dispatch; the handler has not run.
    #[must_use]
    pub fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Prepare dispatch custody through the existing checkpoint append owner.
    /// The private selection already belongs to the predecessor preparation.
    /// # Errors
    /// Preserves the existing observation projection and store-resolution errors.
    pub fn prepare_append(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<PreparedOperationObservationAppend<'_>, OperationObservationCustodyProjectionError>
    {
        let (handle_ref, handle) = self.attempt.context.operation.handle();
        let (predecessor_ref, predecessor) = self.attempt.observation();
        prepare_operation_observation_append(
            OperationObservationTransition::new(
                handle_ref,
                handle,
                predecessor_ref,
                predecessor,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )
    }
}
