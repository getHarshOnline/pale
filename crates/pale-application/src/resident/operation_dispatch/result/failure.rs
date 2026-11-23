//! Read exact native failure evidence without running or retrying its handler.

use super::*;
use crate::actions::{matches_native_failure_diagnostic, NativeActionRegistry};
use crate::diagnostic::AdmittedDiagnostic;

/// Original dispatch inputs and the diagnostic selected by its failed observation.
pub struct NativeOperationFailureEvidenceInput<'a> {
    /// Original operation, request and contextually admitted dispatch history.
    pub history: OperationAttemptHistoryInput<'a>,
    /// Original sealed native registry; no handler is executed during admission.
    pub registry: &'a NativeActionRegistry,
    /// Original contract set named by the request.
    pub contract_set: &'a ContractSetV1,
    /// Direct successor or protected history linking dispatch to failure.
    pub terminal: NativeOperationTerminalEvidence<'a>,
    /// Exact diagnostic record resolved from the terminal evidence reference.
    pub diagnostic: &'a ControlRecord,
    /// Maximum diagnostic bytes, checked before parsing.
    pub maximum_diagnostic_bytes: usize,
}

/// Admit the diagnostic named by an authenticated native failure observation.
///
/// The caller must establish read permission, checkpoint and anchor custody,
/// and the observation signer's historical authority first. This function
/// performs no storage I/O and cannot establish those facts. It checks exact
/// record bytes, the original native selection and diagnostic admission rules.
/// It does not reconstruct a failure from text or run the handler to test it.
/// Historical expiry permits reading evidence, not renewing execution permission.
///
/// # Errors
/// Rejects oversized, substituted or noncanonical records, unlike history,
/// incompatible action contracts and claims of external effects.
pub fn admit_native_operation_failure(
    input: NativeOperationFailureEvidenceInput<'_>,
) -> Result<AdmittedDiagnostic, OperationAttemptError> {
    let bytes = input.diagnostic.canonical_record_jcs().as_slice();
    if bytes.len() > input.maximum_diagnostic_bytes {
        return Err(OperationAttemptError::BoundExceeded);
    }
    let selection = input.history.selection()?;
    let terminal = input.history.validate_terminal(input.terminal)?;
    let OperationObservationStateV1::Failed {
        result: None,
        diagnostic: expected,
    } = terminal.state()
    else {
        return Err(OperationAttemptError::Association);
    };
    if input.diagnostic.coordinate().record_ref() != &expected.record_ref
        || input.diagnostic.coordinate().record_sha256().as_str() != expected.record_sha256.as_str()
        || ArtifactDigest::hash_bytes(bytes) != expected.record_sha256
    {
        return Err(OperationAttemptError::Association);
    }
    let diagnostic = AdmittedDiagnostic::from_bytes(bytes)?;
    let wire = diagnostic.wire();
    if input.diagnostic.schema_id() != &wire.schema
        || input.diagnostic.schema_sha256() != &wire.schema_sha256
        || bytes != diagnostic.canonical_bytes()
    {
        return Err(OperationAttemptError::Association);
    }
    let binding = input.history.operation.handle().1.request_binding();
    let (_, schema_digest) = selection.result_schema();
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
    let (reconstructed, _) = AttemptSelection::capture_native(
        input.history.parents(),
        input.registry,
        selection.attempt_id(),
        admission,
    )?;
    if reconstructed != selection {
        return Err(OperationAttemptError::Association);
    }
    let (action_id, request_digest) = selection
        .native_action_identity()
        .ok_or(OperationAttemptError::Association)?;
    if !matches_native_failure_diagnostic(&diagnostic, action_id, request_digest.as_str()) {
        return Err(OperationAttemptError::Association);
    }
    Ok(diagnostic)
}
