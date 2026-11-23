//! Prepare joint custody of a computed result and its success observation.

use super::*;
use crate::resident::record_projection;

/// Result and observation references, times and observation-signing authority.
pub struct NativeOperationCompletionInput<'a> {
    /// One protected history selecting the original dispatch and authenticated head.
    /// Without it, the append still requires the original dispatch as predecessor.
    pub recovered_history: Option<&'a RecoveredOperationObservation>,
    /// Fresh immutable reference for the signed capability result.
    pub result_ref: ResidentRecordRefV1,
    /// Fresh immutable reference for the success observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Actual completion time supplied by the execution owner, without clamping.
    pub completed_at: UnixTimeV1,
    /// Recording time, no earlier than completion and within handle retention.
    pub observed_at: UnixTimeV1,
    /// Independently admitted authority for signing the observation at recording time.
    pub signing: SignedResidentAdmissionContextV1<'a>,
}

/// Signed computation result and success observation awaiting joint custody.
/// Neither the signature nor this value proves commitment or native-owner acceptance.
#[must_use = "commit the result and observation together, or retain them for recovery"]
pub struct PreparedNativeOperationCompletion<'a> {
    operation: &'a NativeOperationRecordingAuthority,
    predecessor: NativeOperationRecordingPredecessor<'a>,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    result: CapabilityResultV1,
    result_record: ControlRecord,
}

impl ExecutedNativeOperation {
    /// Sign this computation's result and derive its exact success observation.
    ///
    /// Both records still need the existing checkpoint and atomic append. The
    /// append compares its exact predecessor with storage. A protected recovery
    /// can select a later nonconclusive head without discarding expiry or unknown
    /// observations. A conclusive head or subsequent competing write is rejected.
    /// This method may record evidence after execution permission expires; it
    /// neither renews that permission nor runs the handler again. The result
    /// signer and observation signer retain their separate authority obligations.
    ///
    /// # Errors
    /// Rejects aliased references, backward time and expired retention before
    /// signing. Preserves result, observation and record-projection failures.
    /// A failure leaves the actual computation available for another signing attempt.
    pub fn prepare_completion<ResultSign, Rotation, ObservationSign>(
        &self,
        input: NativeOperationCompletionInput<'_>,
        sign_result: ResultSign,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
    ) -> Result<PreparedNativeOperationCompletion<'_>, OperationAttemptError>
    where
        ResultSign: FnOnce(
            &[u8],
            &CapabilityResultPreimageV1,
            &CapabilityResultDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
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
    {
        self.frame.validate_terminal(
            &input.result_ref,
            &input.observation_ref,
            input.completed_at,
            input.observed_at,
        )?;
        let predecessor = self.frame.recording_predecessor(
            input.recovered_history,
            &input.result_ref,
            &input.observation_ref,
            input.observed_at,
        )?;
        let (handle_ref, handle) = self.frame.operation.handle();
        let result = self.issue_result(input.completed_at, sign_result)?;
        let observation = signing::issue_observation(
            &OperationObservationAdmissionContextV1 {
                signed: input.signing,
                handle_ref,
                handle,
                predecessor: Some(predecessor.parts()),
            },
            OperationObservationStateV1::Succeeded {
                result: CapabilityResultReferenceV1 {
                    record_ref: input.result_ref.clone(),
                    record_sha256: result.canonical_result_sha256().clone(),
                },
                effect_receipt: None,
            },
            input.observed_at,
            admit_rotation,
            sign_observation,
        )?;
        let result_record = record_projection::coordinate(
            &input.result_ref,
            result.canonical_result_sha256().as_str(),
        )
        .and_then(|coordinate| record_projection::record(coordinate, &result))
        .map_err(OperationObservationCustodyProjectionError::from)?;
        Ok(PreparedNativeOperationCompletion {
            operation: &self.frame.operation,
            predecessor,
            observation_ref: input.observation_ref,
            observation,
            result,
            result_record,
        })
    }
}

impl PreparedNativeOperationCompletion<'_> {
    /// Borrow the signed result; its physical retention is not yet established.
    #[must_use]
    pub const fn result(&self) -> &CapabilityResultV1 {
        &self.result
    }

    /// Borrow the signed success observation, not a committed terminal status.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Prepare the result and observation for one existing atomic append.
    /// # Errors
    /// Preserves observation lineage, store-resolution and physical projection errors.
    pub fn prepare_append(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<PreparedOperationObservationAppend<'_>, OperationObservationCustodyProjectionError>
    {
        let (handle_ref, handle) = self.operation.handle();
        let (predecessor_ref, predecessor) = self.predecessor.parts();
        let append = prepare_operation_observation_append(
            OperationObservationTransition::new(
                handle_ref,
                handle,
                predecessor_ref,
                predecessor,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?;
        Ok(
            append.with_associated_record(OperationCustodyRecord::CapabilityResult(
                self.result_record.clone(),
            )),
        )
    }
}
