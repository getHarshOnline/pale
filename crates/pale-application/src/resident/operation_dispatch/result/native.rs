//! Admit stored native-result evidence without invoking its handler.

use super::*;
use crate::actions::NativeActionRegistry;

/// Exact stored result and historical inputs selected by an admitted observation.
pub struct NativeOperationResultInput<'a> {
    /// Original admitted operation and its retained dispatch selection.
    pub history: OperationAttemptHistoryInput<'a>,
    /// Sealed registry for the original contract, not a replacement implementation.
    pub registry: &'a NativeActionRegistry,
    /// Original admitted contract set named by the request.
    pub contract_set: &'a ContractSetV1,
    /// Direct successor or protected history linking dispatch to success.
    pub terminal: NativeOperationTerminalEvidence<'a>,
    /// Exact record resolved from that reference.
    pub result: &'a ControlRecord,
    /// Maximum result-record bytes, checked before parsing.
    pub maximum_result_bytes: usize,
}

/// Admit a stored inline result against its original compiled native attempt.
///
/// The caller must first establish permission to read, authenticated observation
/// history, and custody of the referenced records. This function performs no
/// storage reads and does not establish those facts. It binds the returned
/// result to the retained attempt and validates the action digest chain without
/// running its handler. The supplied signature verifier must establish the
/// retained signer's authority at result issuance, not merely verify key bytes.
///
/// Expired execution permission does not prevent historical evidence admission.
/// Success never permits replay, settles a native effect, or renews a lease.
/// A changed or unavailable historical contract is rejected, not reinterpreted.
///
/// # Errors
/// Rejects oversized or substituted records, incompatible contract/attempt
/// identities, invalid action chains, chronology and signature failures.
pub fn admit_native_operation_result<Signature>(
    input: NativeOperationResultInput<'_>,
    verify_signature: Signature,
) -> Result<CapabilityResultV1, OperationAttemptError>
where
    Signature: FnOnce(
        &[u8],
        &CapabilityResultPreimageV1,
        &CapabilityResultDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    let bytes = input.result.canonical_record_jcs().as_slice();
    if bytes.len() > input.maximum_result_bytes {
        return Err(OperationAttemptError::BoundExceeded);
    }
    let selection = input.history.selection()?;
    let terminal = input.history.validate_terminal(input.terminal)?;
    let OperationObservationStateV1::Succeeded {
        result: expected_result,
        effect_receipt: None,
    } = terminal.state()
    else {
        return Err(OperationAttemptError::Association);
    };
    let (schema, schema_digest) = selection.result_schema();
    let (payload_schema, payload_digest) = selection
        .native_payload_schema()
        .ok_or(OperationAttemptError::Association)?;
    if input.result.coordinate().record_ref() != &expected_result.record_ref
        || input.result.coordinate().record_sha256().as_str()
            != expected_result.record_sha256.as_str()
        || input.result.schema_id() != schema
        || input.result.schema_sha256() != schema_digest
    {
        return Err(OperationAttemptError::Association);
    }
    let binding = input.history.operation.handle().1.request_binding();
    let admission = CapabilitySuccessAdmissionContextV1 {
        result: CapabilityResultAdmissionContextV1 {
            expected_schema_sha256: schema_digest,
            expected_request_binding: binding,
            expected_attempt_id: selection.attempt_id(),
            expected_implementation_kind: selection.implementation_kind(),
            expected_implementation_version: selection.implementation_version(),
            signing_authority: selection.result_authority(),
        },
        contract_set_ref: binding.contract_set_ref(),
        contract_set: input.contract_set,
    };
    // Reuse preparation to recover the exact source/input/contract join. It
    // retains a handler but does not invoke it or create execution permission.
    let (reconstructed, prepared) = AttemptSelection::capture_native(
        input.history.parents(),
        input.registry,
        selection.attempt_id(),
        admission,
    )?;
    if reconstructed != selection {
        return Err(OperationAttemptError::Association);
    }
    let wire = serde_json::from_value(
        pale_core::canon::parse_strict(bytes).map_err(|_| OperationAttemptError::Encoding)?,
    )
    .map_err(|_| OperationAttemptError::Encoding)?;
    let mut action_failure = None;
    let result = CapabilityResultV1::admit_success_wire(
        wire,
        &admission,
        |_| None,
        |id, digest, payload| {
            if id != payload_schema || digest != payload_digest {
                return Err(invalid_payload());
            }
            let execution: ActionExecution =
                serde_json::from_slice(payload).map_err(|_| invalid_payload())?;
            if let Err(error) = prepared.validate_recorded_execution(&execution) {
                action_failure = Some(error);
                return Err(invalid_payload());
            }
            Ok(())
        },
        verify_signature,
    );
    if let Some(error) = action_failure {
        return Err(OperationAttemptError::NativeAction(error));
    }
    let result = result?;
    if result.canonical_result_sha256() != &expected_result.record_sha256
        || result.started_at() < input.history.dispatched.observed_at()
        || result.completed_at() > terminal.observed_at()
        || result
            .success()
            .as_ref()
            .is_none_or(|success| success.inline_payload().is_none())
        || pale_core::canon::to_canonical_string(&result)
            .map_err(|_| OperationAttemptError::Encoding)?
            .as_bytes()
            != bytes
    {
        return Err(OperationAttemptError::Association);
    }
    Ok(result)
}

fn invalid_payload() -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission {
        record: "native operation result",
        invariant: "exact retained native action chain",
    }
}
