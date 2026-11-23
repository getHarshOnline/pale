//! Retain an actual handler failure and prepare its terminal evidence.

use super::*;
use crate::actions::{project_invocation_failure, NativeActionInvocationError};
use crate::diagnostic::{AdmittedDiagnostic, DiagnosticAdmissionError};
use crate::resident::record_projection;

/// A released handler's failure, still available if evidence recording fails.
/// This value cannot invoke the handler again or change the original dispatch.
pub struct FailedNativeOperation {
    pub(super) frame: NativeOperationExecutionFrame,
    error: Option<NativeActionInvocationError>,
    staged_diagnostic: Option<AdmittedDiagnostic>,
}

/// Fresh references, recording times and authority for terminal failure evidence.
pub struct NativeOperationFailureInput<'a> {
    /// One protected history selecting the original dispatch and authenticated head.
    /// Without it, the append still requires the original dispatch as predecessor.
    pub recovered_history: Option<&'a RecoveredOperationObservation>,
    /// Fresh immutable reference for the existing diagnostic contract.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Fresh immutable reference for the failed observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Actual failure time supplied by the execution owner, without clamping.
    pub completed_at: UnixTimeV1,
    /// Recording time within handle retention and no earlier than failure.
    pub observed_at: UnixTimeV1,
    /// Independently admitted observation signer at recording time.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Maximum canonical diagnostic bytes retained in the append.
    pub maximum_diagnostic_bytes: usize,
}

/// A diagnostic and signed failure observation awaiting one atomic append.
#[must_use = "commit both records together, or retain them for recovery"]
pub struct PreparedNativeOperationFailure<'a> {
    operation: &'a NativeOperationRecordingAuthority,
    predecessor: NativeOperationRecordingPredecessor<'a>,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    diagnostic: AdmittedDiagnostic,
    diagnostic_record: ControlRecord,
}

impl FailedNativeOperation {
    pub(super) fn new(
        frame: NativeOperationExecutionFrame,
        error: NativeActionInvocationError,
    ) -> Self {
        Self {
            frame,
            error: Some(error),
            staged_diagnostic: None,
        }
    }

    /// Borrow the actual typed failure, not a reconstructed message.
    #[must_use]
    pub fn error(&self) -> Option<&NativeActionInvocationError> {
        self.error.as_ref()
    }

    /// Borrow the dispatch after which this handler failed.
    #[must_use]
    pub fn dispatch(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.frame.dispatch_ref, &self.frame.dispatch)
    }

    /// Prepare the shared action diagnostic and its exact failed observation.
    ///
    /// The handler is not called again. Recording may follow execution expiry,
    /// but must remain within handle retention. The signed observation binds the
    /// diagnostic's exact bytes; the diagnostic itself is not a signed capability
    /// result or an external-effect receipt. The append still needs a checkpoint
    /// and comparison with its exact predecessor before commitment. A protected
    /// history can select a later nonconclusive head; it cannot replace an
    /// established terminal outcome or authorize another handler invocation.
    ///
    /// # Errors
    /// Rejects reused references, backward time, expired retention, oversized
    /// diagnostics and failed signing. Every refusal leaves the failure retained.
    pub fn prepare_failure<Rotation, Sign>(
        &self,
        input: NativeOperationFailureInput<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedNativeOperationFailure<'_>, OperationAttemptError>
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
        self.frame.validate_terminal(
            &input.diagnostic_ref,
            &input.observation_ref,
            input.completed_at,
            input.observed_at,
        )?;
        let predecessor = self.frame.recording_predecessor(
            input.recovered_history,
            &input.diagnostic_ref,
            &input.observation_ref,
            input.observed_at,
        )?;
        let (action_id, _) = self
            .frame
            .selection
            .native_action_identity()
            .ok_or(OperationAttemptError::Association)?;
        let diagnostic = match &self.staged_diagnostic {
            Some(diagnostic) => diagnostic.clone(),
            None => {
                let identity = pale_contracts::record_schema_identity("diagnostic.v1")
                    .map_err(DiagnosticAdmissionError::Registry)?;
                project_invocation_failure(
                    &identity,
                    action_id,
                    self.error
                        .as_ref()
                        .ok_or(OperationAttemptError::Association)?,
                )?
            }
        };
        if diagnostic.canonical_bytes().len() > input.maximum_diagnostic_bytes {
            return Err(OperationAttemptError::BoundExceeded);
        }
        let (diagnostic_sha256, diagnostic_record) =
            record_projection::diagnostic_record(&input.diagnostic_ref, &diagnostic)
                .map_err(OperationObservationCustodyProjectionError::from)?;
        let (handle_ref, handle) = self.frame.operation.handle();
        let observation = signing::issue_observation(
            &OperationObservationAdmissionContextV1 {
                signed: input.signing,
                handle_ref,
                handle,
                predecessor: Some(predecessor.parts()),
            },
            OperationObservationStateV1::Failed {
                result: None,
                diagnostic: ImmutableRecordReferenceV1 {
                    record_ref: input.diagnostic_ref,
                    record_sha256: diagnostic_sha256,
                },
            },
            input.observed_at,
            admit_rotation,
            sign,
        )?;
        Ok(PreparedNativeOperationFailure {
            operation: &self.frame.operation,
            predecessor,
            observation_ref: input.observation_ref,
            observation,
            diagnostic,
            diagnostic_record,
        })
    }

    pub(super) fn stage_diagnostic(
        &mut self,
        maximum_diagnostic_bytes: usize,
    ) -> Result<AdmittedDiagnostic, OperationAttemptError> {
        let (action_id, _) = self
            .frame
            .selection
            .native_action_identity()
            .ok_or(OperationAttemptError::Association)?;
        let identity = pale_contracts::record_schema_identity("diagnostic.v1")
            .map_err(DiagnosticAdmissionError::Registry)?;
        let diagnostic = project_invocation_failure(
            &identity,
            action_id,
            self.error
                .as_ref()
                .ok_or(OperationAttemptError::Association)?,
        )?;
        if diagnostic.canonical_bytes().len() > maximum_diagnostic_bytes {
            return Err(OperationAttemptError::BoundExceeded);
        }
        self.staged_diagnostic = Some(diagnostic.clone());
        Ok(diagnostic)
    }

    pub(super) fn from_staged(
        frame: NativeOperationExecutionFrame,
        diagnostic: AdmittedDiagnostic,
    ) -> Self {
        Self {
            frame,
            error: None,
            staged_diagnostic: Some(diagnostic),
        }
    }
}

impl PreparedNativeOperationFailure<'_> {
    /// Borrow the shared diagnostic; it does not prove physical retention.
    #[must_use]
    pub fn diagnostic(&self) -> &AdmittedDiagnostic {
        &self.diagnostic
    }

    /// Borrow the signed failed observation, not a committed terminal status.
    #[must_use]
    pub fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Prepare both records for the existing checkpoint and atomic append.
    /// # Errors
    /// Preserves lineage, store-resolution and physical projection failures.
    pub fn prepare_append(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<PreparedOperationObservationAppend<'_>, OperationObservationCustodyProjectionError>
    {
        prepare_diagnostic_observation_append(
            self.operation.handle(),
            self.predecessor.parts(),
            self.observation(),
            &self.diagnostic_record,
            resolve_store,
        )
    }
}
