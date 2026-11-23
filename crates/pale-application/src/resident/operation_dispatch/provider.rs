//! External-provider preparation and durable dispatch phases.
//!
//! This module stops before carrier release. A signed `Dispatched` observation
//! is not evidence that a provider or A2A route was called, fresh, or ready.

use super::selection::AttemptSelectionInput;
use super::*;
use crate::provider_invocation::ProviderInvocationAdmissionContext;
use pale_ports::ProviderInvocationRequest;

pub(in crate::resident) mod execution;
mod recovery;
#[allow(
    dead_code,
    reason = "the private ProviderResolved restart owner awaits assembled resident attachment"
)]
mod resolved_recovery;
mod service;
pub use execution::{
    continue_consumed_provider_recording, invoke_provider_operation,
    record_consumed_provider_operation, CompletedProviderOperation, ConsumedProviderOperation,
    ConsumedProviderRecordingContinuation, ProviderOperationInvokeFailure,
    ProviderOperationInvokeInput, ProviderOperationInvokeResult,
    ProviderOperationRecordingConflict, ProviderOperationRecordingCustody,
    ProviderOperationRecordingFailure, ProviderOperationRecordingFinalization,
    ProviderOperationRecordingFinalizationFailure, ProviderOperationRecordingPreparationError,
    ProviderOperationRecordingRecoveryFailure, ProviderOperationUncertainty,
    UncertainProviderOperation,
};
pub(in crate::resident) use execution::{
    continue_issued_recovered_provider_unknown, continue_prepared_recovered_provider_unknown,
    invoke_owned_provider_invocation, prepare_pairwise_delegate_provider_call,
    prepare_provider_operation_call, prepare_recovered_provider_completion,
    prepare_recovered_provider_operation_call, record_provider_operation_completion,
    record_recovered_provider_operation_completion, record_recovered_provider_operation_unknown,
    validate_recovered_provider_call_references, validate_recovered_provider_unknown,
    IssuedRecoveredProviderUnknownCheckpoint, OwnedProviderInvocationRequest,
    PreparedProviderOperationCall, PreparedRecoveredProviderOperationCall,
    PreparedRecoveredProviderUnknown, ProviderOperationCallBasis,
    ProviderOperationContinuationCoordinate, RecoveredProviderCallPreparationFailure,
    RecoveredProviderDispatchRelease, RecoveredProviderUnknownRecordingFailure,
};
pub(in crate::resident) use recovery::{
    recover_provider_dispatch_under_cancellation_request, recover_provider_operation_dispatch,
    recover_provider_operation_preparation, PreparedRecoveredProviderDispatch,
    RecoveredProviderOperationDispatch,
};
pub(in crate::resident) use resolved_recovery::{
    prepare_recovered_provider_attempt, retain_recovered_provider_attempt,
    CompletedRecoveredProviderAttempt, PreparedRecoveredProviderAttempt,
    RecoveredProviderAttemptAnchorPending, RecoveredProviderAttemptCommitFailure,
    RecoveredProviderAttemptCustodyFailure, RecoveredProviderAttemptInput,
    RecoveredProviderAttemptMeaning,
};
pub use service::{
    retain_provider_operation_dispatch, ProviderOperationDispatchInput,
    ProviderOperationRetentionFailure, RetainedProviderOperationDispatch,
};

#[cfg(test)]
#[path = "../../../tests/internal/resident/operation_dispatch/provider_support.rs"]
pub(in crate::resident) mod test_support;

/// Source-selected external provider and its result admission context.
pub struct ProviderOperationInput<'a> {
    /// Original operation, current authority and observation inputs.
    pub context: OperationAttemptContext<'a>,
    /// Actual selected request, including attempt and implementation identity.
    pub invocation: ProviderInvocationRequest<'a>,
    /// Independently selected result schema, implementation, and signer.
    pub result_admission: ProviderInvocationAdmissionContext<'a>,
    /// Fresh reference for the provider-resolution observation.
    pub resolution_ref: ResidentRecordRefV1,
}

/// Signed provider resolution awaiting checkpoint, database, and anchor custody.
///
/// This phase retains the exact attempt selection but exposes no transport or
/// provider-call method.
#[must_use = "commit provider resolution before preparing its attempt"]
pub struct PreparedProviderResolution<'a> {
    context: OperationAttemptContext<'a>,
    selection: AttemptSelection,
    result_admission: ProviderInvocationAdmissionContext<'a>,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
}

/// Signed preparation with its exact selected provider request.
///
/// This phase has no transport permission. Its observation and private
/// selection must first enter checkpoint, database, and independent-anchor
/// custody.
#[must_use = "commit preparation before preparing an external-provider dispatch"]
pub struct PreparedProviderOperationAttempt<'a> {
    context: OperationAttemptContext<'a>,
    preparation: PreparedOperationAttempt<'a>,
    selection: AttemptSelection,
    result_admission: ProviderInvocationAdmissionContext<'a>,
}

/// Current authority and fresh identity for one external-provider dispatch.
pub struct ProviderOperationDispatchContext<'a> {
    /// Exact database-and-anchor completion of the retained preparation.
    pub completed: &'a super::super::CompletedOperationObservationCustody,
    /// Current session authority; preparation-time authority is not reused.
    pub current: &'a AmbientSessionAdmissionContext<'a>,
    /// Independently resolved observation schema and runtime signer.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh immutable reference for the dispatch observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Existing owner of the observation's store coordinate.
    pub resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
}

/// Signed external-provider dispatch awaiting its own durable custody.
///
/// A later carrier-specific phase must prove fresh custody and authority before
/// it can consume this value. This phase does not prove A2A route freshness.
#[must_use = "commit this exact dispatch before releasing any provider request"]
pub struct PreparedProviderOperationDispatch<'a> {
    attempt: PreparedProviderOperationAttempt<'a>,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
}

/// Rejected dispatch preparation with the exact provider attempt retained.
pub struct ProviderOperationDispatchFailure<'a> {
    attempt: Box<PreparedProviderOperationAttempt<'a>>,
    reason: OperationAttemptError,
}

/// Rejected attempt preparation with the exact provider resolution retained.
pub struct ProviderOperationPreparationFailure<'a> {
    resolution: Box<PreparedProviderResolution<'a>>,
    reason: OperationAttemptError,
}

impl std::fmt::Debug for ProviderOperationDispatchFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderOperationDispatchFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for ProviderOperationDispatchFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.reason.fmt(formatter)
    }
}

impl std::error::Error for ProviderOperationDispatchFailure<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.reason)
    }
}

impl std::fmt::Debug for ProviderOperationPreparationFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderOperationPreparationFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for ProviderOperationPreparationFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.reason.fmt(formatter)
    }
}

impl std::error::Error for ProviderOperationPreparationFailure<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.reason)
    }
}

impl<'a> ProviderOperationDispatchFailure<'a> {
    /// Borrow the reason without discarding the prepared attempt.
    #[must_use]
    pub const fn reason(&self) -> &OperationAttemptError {
        &self.reason
    }

    /// Recover the exact unopened preparation after resolving the rejection.
    pub fn into_attempt(self) -> PreparedProviderOperationAttempt<'a> {
        *self.attempt
    }
}

impl<'a> ProviderOperationPreparationFailure<'a> {
    /// Borrow the reason without discarding the retained resolution.
    #[must_use]
    pub const fn reason(&self) -> &OperationAttemptError {
        &self.reason
    }

    /// Recover the exact provider resolution after resolving the rejection.
    pub fn into_resolution(self) -> PreparedProviderResolution<'a> {
        *self.resolution
    }
}

/// Resolve one external provider within the operation's frozen constitution.
///
/// The predecessor must be `Resolving` for a self-contained operation or
/// `ContextBound` when the lease retains evidence obligations.
/// This function rechecks current session and child authority, derives the
/// private selection from the exact provider request, and signs only the
/// provider-binding reference. It does not retain the private selection yet or
/// call a provider, custody port, anchor, or A2A route.
///
/// # Errors
/// Rejects changed authority, expired inputs, substituted identities, unlike
/// provider selection, reused references, and existing signing failures.
pub fn prepare_provider_resolution<'a, Authority, Rotation, Sign>(
    input: ProviderOperationInput<'a>,
    authority: &mut Authority,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<PreparedProviderResolution<'a>, OperationAttemptError>
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
    prepare_provider_resolution_at(input, current, authority, admit_rotation, sign)
}

fn prepare_provider_resolution_at<'a, Authority, Rotation, Sign>(
    input: ProviderOperationInput<'a>,
    current: &AmbientSessionAdmissionContext<'_>,
    authority: &mut Authority,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<PreparedProviderResolution<'a>, OperationAttemptError>
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
    let context = input.context.clone();
    let result_admission = input.result_admission;
    let selection = AttemptSelection::capture(AttemptSelectionInput {
        parents: input.context.parents(),
        invocation: input.invocation,
        admission: result_admission,
    })?;
    selection.record(
        input.context.selection_ref.clone(),
        input.context.maximum_selection_bytes,
    )?;
    let predecessor = input
        .context
        .observation
        .predecessor
        .ok_or(OperationAttemptError::Association)?;
    if [
        &input.resolution_ref,
        &context.selection_ref,
        &context.observation_ref,
    ]
    .iter()
    .enumerate()
    .any(|(index, reference)| {
        [
            &input.resolution_ref,
            &context.selection_ref,
            &context.observation_ref,
        ][..index]
            .contains(reference)
            || **reference == *predecessor.0
            || **reference == *context.operation.handle().0
            || **reference == *context.operation.handle().1.operation_lease_ref()
            || **reference == *context.operation.request_coordinate().record_ref()
    }) {
        return Err(OperationAttemptError::Association);
    }
    let provider_binding = selection
        .provider()
        .ok_or(OperationAttemptError::Association)?
        .clone();
    signing::validate_runtime_signer(
        input.context.observation.signed.signing_authority,
        current,
        OPERATION_OBSERVATION_V1_SCHEMA,
    )?;
    let observation = signing::issue_observation(
        &input.context.observation,
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::ProviderResolved { provider_binding },
        },
        current.now,
        admit_rotation,
        sign,
    )?;
    Ok(PreparedProviderResolution {
        context,
        selection,
        result_admission,
        observation_ref: input.resolution_ref,
        observation,
    })
}

impl<'operation> PreparedProviderResolution<'operation> {
    /// Retain only the signed `ProviderResolved` successor for checkpoint custody.
    ///
    /// The private request selection is deliberately discarded. Protected
    /// recovery reconstructs the later attempt from the provider binding named
    /// by the durable observation; this continuation cannot release a carrier.
    pub(in crate::resident) fn into_owned_observation(
        self,
    ) -> super::PreparedOwnedOperationObservation {
        let predecessor = self
            .context
            .observation
            .predecessor
            .expect("provider resolution construction requires a predecessor");
        super::PreparedOwnedOperationObservation::from_transition(
            self.context.operation,
            predecessor,
            self.observation_ref,
            self.observation,
        )
    }

    /// Borrow the signed provider resolution; it is not yet committed.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Prepare the provider-resolution append without private attempt data.
    /// # Errors
    /// Preserves the existing projection and store-resolution errors.
    pub fn prepare_append(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<PreparedOperationObservationAppend<'_>, OperationObservationCustodyProjectionError>
    {
        let (handle_ref, handle) = self.context.operation.handle();
        let predecessor = self
            .context
            .observation
            .predecessor
            .ok_or(OperationObservationCustodyProjectionError::Lineage)?;
        prepare_operation_observation_append(
            OperationObservationTransition::new(
                handle_ref,
                handle,
                predecessor.0,
                predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )
    }

    /// Sign `DispatchPrepared` only after exact provider-resolution custody.
    ///
    /// No carrier is called. On rejection, the exact signed resolution remains
    /// available through [`ProviderOperationPreparationFailure`].
    ///
    /// # Errors
    ///
    /// Returns the unopened resolution when current authority, prior custody,
    /// result admission, or observation signing no longer matches.
    pub fn prepare_attempt<Authority, Rotation, Sign>(
        self,
        completed: &super::super::CompletedOperationObservationCustody,
        current: &AmbientSessionAdmissionContext<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        authority: &mut Authority,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<
        PreparedProviderOperationAttempt<'operation>,
        ProviderOperationPreparationFailure<'operation>,
    >
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
        let prepared = (|| {
            self.context.validate_current_at(current, authority)?;
            self.prepare_append(resolve_store)?
                .verify_completed_custody(completed)?;
            self.result_admission
                .validate_for(
                    self.selection
                        .provider_request(self.context.parents())
                        .map_err(OperationAttemptError::from)?,
                )
                .map_err(|_| OperationAttemptError::Association)?;
            self.context.clone().sign_preparation_after(
                self.selection.clone(),
                current,
                (&self.observation_ref, &self.observation),
                admit_rotation,
                sign,
            )
        })();
        match prepared {
            Ok(preparation) => Ok(PreparedProviderOperationAttempt {
                context: self.context,
                preparation,
                selection: self.selection,
                result_admission: self.result_admission,
            }),
            Err(reason) => Err(ProviderOperationPreparationFailure {
                resolution: Box::new(self),
                reason,
            }),
        }
    }
}

impl<'operation> PreparedProviderOperationAttempt<'operation> {
    /// Borrow the signed preparation; this is not custody or dispatch evidence.
    #[must_use]
    pub fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.preparation.observation()
    }

    /// Reconstruct the exact retained provider request without granting access.
    /// # Errors
    /// Rejects a selection that no longer matches its immutable parents.
    pub fn provider_request(&self) -> Result<ProviderInvocationRequest<'_>, OperationAttemptError> {
        self.selection
            .provider_request(self.context.parents())
            .map_err(OperationAttemptError::from)
    }

    /// Prepare the atomic observation-and-selection append.
    /// # Errors
    /// Preserves the existing projection and store-resolution errors.
    pub fn prepare_append(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<PreparedOperationObservationAppend<'_>, OperationObservationCustodyProjectionError>
    {
        self.preparation.prepare_append(resolve_store)
    }

    /// Sign dispatch after exact preparation custody and fresh authority checks.
    ///
    /// No carrier is called. On rejection, the exact attempt remains available
    /// through [`ProviderOperationDispatchFailure`].
    /// # Errors
    /// Rejects unlike custody, changed authority, unresolved context, reused
    /// identities, and existing observation or signer failures.
    pub fn prepare_dispatch<Authority, Rotation, Sign>(
        self,
        input: ProviderOperationDispatchContext<'_>,
        authority: &mut Authority,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<
        PreparedProviderOperationDispatch<'operation>,
        ProviderOperationDispatchFailure<'operation>,
    >
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
        match self.prepare_dispatch_observation(input, authority, admit_rotation, sign) {
            Ok((observation_ref, observation)) => Ok(PreparedProviderOperationDispatch {
                attempt: self,
                observation_ref,
                observation,
            }),
            Err(reason) => Err(ProviderOperationDispatchFailure {
                attempt: Box::new(self),
                reason,
            }),
        }
    }

    fn prepare_dispatch_observation<Authority, Rotation, Sign>(
        &self,
        input: ProviderOperationDispatchContext<'_>,
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
        let request = self.provider_request()?;
        self.result_admission
            .validate_for(request)
            .map_err(|_| OperationAttemptError::Association)?;
        self.prepare_append(input.resolve_store)?
            .verify_completed_custody(input.completed)?;
        self.validate_successor_reference(&input.observation_ref)?;
        signing::validate_runtime_signer(
            input.signing.signing_authority,
            input.current,
            OPERATION_OBSERVATION_V1_SCHEMA,
        )?;
        signing::validate_runtime_signer(
            self.result_admission
                .success_context()
                .result
                .signing_authority,
            input.current,
            CAPABILITY_RESULT_V1_SCHEMA,
        )?;
        let (handle_ref, handle) = self.context.operation.handle();
        let (prepared_ref, prepared) = self.preparation.observation();
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
        let selection_record = self.selection.record(
            self.context.selection_ref.clone(),
            self.context.maximum_selection_bytes,
        )?;
        if attempt_id != self.selection.attempt_id()
            || evidence.record_ref != *selection_record.coordinate().record_ref()
            || evidence.record_sha256.as_str()
                != selection_record.coordinate().record_sha256().as_str()
        {
            return Err(OperationAttemptError::Association);
        }
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

impl PreparedProviderOperationDispatch<'_> {
    /// Borrow the exact original operation context for a carrier-specific phase.
    #[must_use]
    pub const fn context(&self) -> &OperationAttemptContext<'_> {
        &self.attempt.context
    }

    /// Borrow the signed dispatch. No provider request has been released.
    #[must_use]
    pub fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Reconstruct the exact provider request retained by this dispatch.
    /// # Errors
    /// Rejects a selection that no longer matches its immutable parents.
    pub fn provider_request(&self) -> Result<ProviderInvocationRequest<'_>, OperationAttemptError> {
        self.attempt.provider_request()
    }

    /// Prepare dispatch custody without calling the provider.
    /// # Errors
    /// Preserves the existing projection and store-resolution errors.
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
