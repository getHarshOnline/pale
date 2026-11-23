//! Run a retained pure handler once after fresh dispatch checks.

use super::*;
use crate::resident::RecoveredOperationObservation;
use pale_ports::{AntiRollbackAnchorPort, OperationCustodyPort, WallClockSourcePort};
use std::time::{Duration, Instant};

mod completion;
mod failure;
mod refusal;
mod staging;
pub use completion::{NativeOperationCompletionInput, PreparedNativeOperationCompletion};
pub use failure::{
    FailedNativeOperation, NativeOperationFailureInput, PreparedNativeOperationFailure,
};
pub use refusal::{
    NativeOperationCancellationRefusal, NativeOperationExpiryInput, NativeOperationExpiryRefusal,
    NativeOperationInvocationRefusal, NativeOperationRefusalCancellationInput,
    PreparedNativeOperationExpiry,
};
pub use staging::{
    recover_native_operation_outcome, NativeOperationOutcomeRecoveryInput,
    NativeOperationOutcomeStageInput, NativeOperationOutcomeStagePreparationFailure,
    PreparedNativeOperationOutcomeStage, RecoveredNativeOperationOutcome,
    StagedNativeOperationOutcome, UnstagedNativeOperationOutcome,
};
pub(in crate::resident) use staging::{
    recover_native_operation_outcome_phase, NativeOperationOutcomeStagePreparation,
    RecoveredNativeOperationOutcomePhase,
};

/// Current authority, selected clock and bounded storage inputs for execution.
pub struct NativeOperationExecutionContext<'a> {
    /// Resolved authority records; their heads are rechecked before execution.
    pub current: &'a AmbientSessionAdmissionContext<'a>,
    /// Existing owner of the dispatch observation's store coordinate.
    pub resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    /// Caller-selected record-time source; this does not establish clock trust.
    pub clock: &'a dyn WallClockSourcePort,
    /// Maximum bytes for each of the four dispatch-custody reads.
    pub maximum_record_bytes: usize,
}

/// Actual pure computation retained for result issuance, not operation settlement.
/// A failed result-signing attempt does not run the handler again or discard this value.
pub struct ExecutedNativeOperation {
    frame: NativeOperationExecutionFrame,
    execution: ActionExecution,
}

/// Actual outcome of one released handler, retained for terminal recording.
/// A failed handler is distinct from refusing to release it.
#[must_use = "record this outcome without invoking the handler again"]
pub enum NativeOperationOutcome {
    /// The handler produced a schema-checked computation.
    Succeeded(Box<ExecutedNativeOperation>),
    /// The handler failed; its dispatch and typed cause remain available.
    Failed(Box<FailedNativeOperation>),
}

/// Original dispatch and authority inputs shared by either computation outcome.
pub(super) struct NativeOperationExecutionFrame {
    operation: NativeOperationRecordingAuthority,
    selection_ref: ResidentRecordRefV1,
    preparation_observation_ref: ResidentRecordRefV1,
    preparation_predecessor_ref: Option<ResidentRecordRefV1>,
    selection: AttemptSelection,
    contract_set: ContractSetV1,
    dispatch_ref: ResidentRecordRefV1,
    dispatch: OperationObservationV1,
    started_at: UnixTimeV1,
    recording_head: Option<Box<(ResidentRecordRefV1, OperationObservationV1)>>,
}

/// Immutable operation facts needed after the pure handler has run.
///
/// Recording retains no live operation continuation. Capturing only the handle
/// and checkpoint selection lets a post-handler failure outlive protected
/// recovery without cloning an [`IssuedLocalOperation`] or its session custody.
pub(in crate::resident) struct NativeOperationRecordingAuthority {
    checkpoint_selection: crate::resident::checkpoint_profile::OperationCheckpointSelection,
    request_ref: ResidentRecordRefV1,
    handle_ref: ResidentRecordRefV1,
    handle: OperationHandleV1,
    historical_runtime_authority_ref: ResidentRecordRefV1,
    historical_runtime_authority_sha256: ArtifactDigest,
    historical_principal_id: ResidentPrincipalIdV1,
}

impl NativeOperationRecordingAuthority {
    fn capture(operation: &IssuedLocalOperation) -> Self {
        let (handle_ref, handle) = operation.handle();
        let historical_session = operation.historical_session_lease();
        Self {
            checkpoint_selection: operation.checkpoint_selection().clone(),
            request_ref: operation.request_coordinate().record_ref().clone(),
            handle_ref: handle_ref.clone(),
            handle: handle.clone(),
            historical_runtime_authority_ref: historical_session
                .deployment_runtime_authority_ref()
                .clone(),
            historical_runtime_authority_sha256: historical_session
                .deployment_runtime_authority_sha256()
                .clone(),
            historical_principal_id: historical_session.issuer_principal_id().clone(),
        }
    }

    pub(in crate::resident) const fn handle(&self) -> (&ResidentRecordRefV1, &OperationHandleV1) {
        (&self.handle_ref, &self.handle)
    }

    pub(in crate::resident) const fn checkpoint_selection(
        &self,
    ) -> &crate::resident::checkpoint_profile::OperationCheckpointSelection {
        &self.checkpoint_selection
    }

    pub(in crate::resident) const fn request_ref(&self) -> &ResidentRecordRefV1 {
        &self.request_ref
    }

    pub(in crate::resident) const fn historical_runtime_authority(
        &self,
    ) -> (
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentPrincipalIdV1,
    ) {
        (
            &self.historical_runtime_authority_ref,
            &self.historical_runtime_authority_sha256,
            &self.historical_principal_id,
        )
    }
}

// Ordinary completion borrows its dispatch. A later recording owns only the
// recovered head, so recording does not borrow the recovery result.
enum NativeOperationRecordingPredecessor<'a> {
    Dispatch(&'a ResidentRecordRefV1, &'a OperationObservationV1),
    Recovered(Box<(ResidentRecordRefV1, OperationObservationV1)>),
}

impl NativeOperationRecordingPredecessor<'_> {
    fn parts(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        match self {
            Self::Dispatch(reference, observation) => (reference, observation),
            Self::Recovered(head) => (&head.0, &head.1),
        }
    }
}

impl NativeOperationExecutionFrame {
    fn recording_predecessor(
        &self,
        recovered: Option<&RecoveredOperationObservation>,
        evidence_ref: &ResidentRecordRefV1,
        observation_ref: &ResidentRecordRefV1,
        observed_at: UnixTimeV1,
    ) -> Result<NativeOperationRecordingPredecessor<'_>, OperationAttemptError> {
        let Some(recovered) = recovered else {
            if let Some(head) = &self.recording_head {
                return Ok(NativeOperationRecordingPredecessor::Recovered(head.clone()));
            }
            return Ok(NativeOperationRecordingPredecessor::Dispatch(
                &self.dispatch_ref,
                &self.dispatch,
            ));
        };
        let (dispatch_ref, dispatched) = recovered.selected();
        let (head_ref, head) = recovered.selected_head();
        if !self
            .operation
            .checkpoint_selection()
            .matches_store(recovered.store())
            || dispatch_ref != &self.dispatch_ref
            || dispatched != &self.dispatch
            || head.state().is_terminal()
            || head_ref == evidence_ref
            || head_ref == observation_ref
        {
            return Err(OperationAttemptError::Association);
        }
        if observed_at < *head.observed_at() {
            return Err(OperationAttemptError::TimeOrder);
        }
        Ok(NativeOperationRecordingPredecessor::Recovered(Box::new((
            head_ref.clone(),
            head.clone(),
        ))))
    }

    fn validate_terminal(
        &self,
        evidence_ref: &ResidentRecordRefV1,
        observation_ref: &ResidentRecordRefV1,
        completed_at: UnixTimeV1,
        observed_at: UnixTimeV1,
    ) -> Result<(), OperationAttemptError> {
        let (handle_ref, handle) = self.operation.handle();
        if observed_at < completed_at || completed_at < self.started_at {
            return Err(OperationAttemptError::TimeOrder);
        }
        if observed_at > *handle.retain_until() {
            return Err(OperationAttemptError::RetentionElapsed);
        }
        let existing = [
            handle_ref,
            handle.operation_lease_ref(),
            self.operation.request_ref(),
            &self.selection_ref,
            &self.preparation_observation_ref,
            &self.dispatch_ref,
        ];
        if *evidence_ref == *observation_ref
            || existing
                .iter()
                .any(|reference| **reference == *evidence_ref || **reference == *observation_ref)
            || self
                .preparation_predecessor_ref
                .as_ref()
                .is_some_and(|reference| reference == evidence_ref || reference == observation_ref)
        {
            return Err(OperationAttemptError::Association);
        }
        Ok(())
    }
}

impl<'operation, 'registry> PreparedNativeOperationDispatch<'operation, 'registry> {
    /// Consume one retained handler after exact new dispatch custody is confirmed.
    ///
    /// Rechecks session authority before storage inspection and again at the
    /// freshly observed execution time. A final clock read checks expiry after
    /// those authority calls, without starting another round of I/O.
    /// Database replay or ambiguous recovered
    /// commitment cannot release another call. The caller must keep these ports
    /// under the resident's single-writer ownership; sequential reads are not a
    /// cross-process lock. Only self-contained, compute-only actions are supported.
    ///
    /// The existing bounded handler runs synchronously. The monotonic timeout
    /// check covers preparation within this call; it does not preempt the handler
    /// or establish a hard CPU deadline. No external provider or native effect runs.
    ///
    /// # Errors
    /// Preserves authority, custody and clock failures. Actual handler failure
    /// is returned as a retained outcome, not as permission to repeat dispatch. Rejects
    /// unresolved context, backward time, expiry and exhausted pre-dispatch time.
    /// A refused release retains the unopened call and original custody for
    /// explicit checked re-entry; it grants no permission to replay.
    pub fn invoke(
        self,
        completed: CompletedOperationObservationCustody,
        input: NativeOperationExecutionContext<'_>,
        authority: &mut impl AmbientSessionAuthorityPort,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<
        UnstagedNativeOperationOutcome,
        NativeOperationInvocationRefusal<'operation, 'registry>,
    > {
        let floor = input.current.now.max(*self.observation.observed_at());
        refusal::NativeRelease {
            dispatch: self,
            completed,
            started: Instant::now(),
            time: refusal::ReleaseTime::Unobserved { floor },
        }
        .run(input, authority, custody, anchor)
    }

    fn execute(self, release_time: UnixTimeV1) -> UnstagedNativeOperationOutcome {
        let PreparedNativeOperationAttempt {
            context,
            selection,
            call,
            result_admission,
            ..
        } = self.attempt;
        let frame = NativeOperationExecutionFrame {
            operation: NativeOperationRecordingAuthority::capture(context.operation),
            selection_ref: context.selection_ref,
            preparation_observation_ref: context.observation_ref,
            preparation_predecessor_ref: context
                .observation
                .predecessor
                .map(|(reference, _)| reference.clone()),
            selection,
            contract_set: result_admission.contract_set.clone(),
            dispatch_ref: self.observation_ref,
            dispatch: self.observation,
            started_at: release_time,
            recording_head: None,
        };
        UnstagedNativeOperationOutcome::new(match call.execute() {
            Ok(execution) => NativeOperationOutcome::Succeeded(Box::new(ExecutedNativeOperation {
                frame,
                execution,
            })),
            Err(error) => {
                NativeOperationOutcome::Failed(Box::new(FailedNativeOperation::new(frame, error)))
            }
        })
    }
}

pub(super) fn execute_recovered_native<'operation>(
    attempt: RecoveredNativeAttempt<'operation, '_>,
    selection_ref: ResidentRecordRefV1,
    preparation_observation_ref: ResidentRecordRefV1,
    preparation_predecessor_ref: Option<ResidentRecordRefV1>,
    dispatch_ref: ResidentRecordRefV1,
    dispatch: OperationObservationV1,
    release_time: UnixTimeV1,
) -> UnstagedNativeOperationOutcome {
    let operation = attempt.operation();
    let (selection, call, contract_set) = attempt.into_parts();
    let frame = NativeOperationExecutionFrame {
        operation: NativeOperationRecordingAuthority::capture(operation),
        selection_ref,
        preparation_observation_ref,
        preparation_predecessor_ref,
        selection,
        contract_set: contract_set.clone(),
        dispatch_ref,
        dispatch,
        started_at: release_time,
        recording_head: None,
    };
    UnstagedNativeOperationOutcome::new(match call.execute() {
        Ok(execution) => NativeOperationOutcome::Succeeded(Box::new(ExecutedNativeOperation {
            frame,
            execution,
        })),
        Err(error) => {
            NativeOperationOutcome::Failed(Box::new(FailedNativeOperation::new(frame, error)))
        }
    })
}

impl ExecutedNativeOperation {
    /// Borrow the actual computation; it is not yet a signed capability result.
    #[must_use]
    pub const fn execution(&self) -> &ActionExecution {
        &self.execution
    }

    /// Borrow the exact dispatch to which the retained computation belongs.
    #[must_use]
    pub const fn dispatch(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.frame.dispatch_ref, &self.frame.dispatch)
    }

    /// Issue a signed result from the actual computation and frozen expectations.
    ///
    /// The execution owner supplies the terminal result time without clamping it.
    /// The signing owner must establish authority at issuance. This method keeps
    /// the computation available after failure; it never invokes it again. Result
    /// custody, the terminal operation observation and owner acceptance are separate.
    ///
    /// # Errors
    /// Rejects backward completion time, oversized payloads, incompatible result
    /// context and signing failure through the existing capability-result owner.
    pub fn issue_result<Sign>(
        &self,
        completed_at: UnixTimeV1,
        sign: Sign,
    ) -> Result<CapabilityResultV1, OperationAttemptError>
    where
        Sign: FnOnce(
            &[u8],
            &CapabilityResultPreimageV1,
            &CapabilityResultDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let (payload_id, payload_digest) = self
            .frame
            .selection
            .native_payload_schema()
            .ok_or(OperationAttemptError::Association)?;
        let payload =
            serde_json::to_value(&self.execution).map_err(|_| OperationAttemptError::Encoding)?;
        let success =
            CapabilitySuccessV1::inline(payload_id.clone(), payload_digest.clone(), payload)?;
        let (schema_id, schema_sha256) = self.frame.selection.result_schema();
        let signer = self.frame.selection.result_authority();
        let binding = self.frame.operation.handle().1.request_binding();
        let admission = CapabilitySuccessAdmissionContextV1 {
            result: CapabilityResultAdmissionContextV1 {
                expected_schema_sha256: schema_sha256,
                expected_request_binding: binding,
                expected_attempt_id: self.frame.selection.attempt_id(),
                expected_implementation_kind: self.frame.selection.implementation_kind(),
                expected_implementation_version: self.frame.selection.implementation_version(),
                signing_authority: signer,
            },
            contract_set_ref: binding.contract_set_ref(),
            contract_set: &self.frame.contract_set,
        };
        let result = CapabilityResultV1::issue_success_contextual(
            CapabilityResultPreimageV1 {
                schema_id: schema_id.clone(),
                schema_sha256: schema_sha256.clone(),
                request_binding: binding.clone(),
                attempt_id: self.frame.selection.attempt_id().clone(),
                status: CapabilityResultStatusV1::Succeeded,
                success: Some(success),
                failure: None,
                implementation_kind: self.frame.selection.implementation_kind().clone(),
                implementation_version: self.frame.selection.implementation_version().clone(),
                started_at: self.frame.started_at,
                completed_at,
                signing_authority_binding: signer.clone(),
                issuer_principal_id: signer.principal_id().clone(),
                issuer_key_id: ResidentKeyIdV1::parse(signer.signer_key_id())?,
                issuer_key_epoch: *signer.signer_key_epoch(),
            },
            &admission,
            |_| None,
            |id, digest, bytes| {
                // PreparedNativeAction already schema-checked this exact result.
                // Compare its canonical bytes; do not run another validator or handler.
                if id != payload_id
                    || digest != payload_digest
                    || pale_core::canon::to_canonical_string(&self.execution)?.as_bytes() != bytes
                {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "native operation result",
                        invariant: "exact schema-checked action execution",
                    });
                }
                Ok(())
            },
            sign,
        )?;
        Ok(result)
    }
}
