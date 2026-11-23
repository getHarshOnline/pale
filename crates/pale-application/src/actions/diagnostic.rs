//! Classify action failures once for resident and SDK diagnostic consumers.

use super::NativeActionInvocationError;
use crate::diagnostic::{
    project_failure, schema_diagnostic_coordinate, schema_violation_projection, AdmittedDiagnostic,
    CauseProjection, DiagnosticAdmissionError, FailureSpecification, SchemaViolationProjection,
};
use pale_core::records::{
    ActionErrorCode, DiagnosticDispositionV1, DiagnosticEffectCertaintyV1,
    DiagnosticRequestCorrelationV1Wire, DiagnosticRetryV1, DiagnosticStageV1,
    DiagnosticSubjectV1Wire, RecordSchemaIdentity,
};

/// Classification inputs used to project one owning action error once.
pub struct ActionFailure<'a> {
    /// Stable diagnostic code.
    pub code: &'a str,
    /// Recovery-relevant failure class.
    pub disposition: DiagnosticDispositionV1,
    /// Concise redaction input; [`project_failure`] owns sanitization.
    pub summary: &'a str,
    /// Retry posture selected exhaustively from the action error.
    pub retry: DiagnosticRetryV1,
    /// Optional digest of a request that reached typed action admission.
    pub request_digest: Option<String>,
    /// Safe structured input-schema evidence retained through application admission.
    pub schema: Option<SchemaViolationProjection>,
}

/// Classify a typed action failure without retrying or granting effect authority.
/// # Errors
/// Preserves diagnostic schema, source-coordinate and field-consistency failures.
pub fn project_invocation_failure(
    identity: &RecordSchemaIdentity,
    action_id: &str,
    error: &NativeActionInvocationError,
) -> Result<AdmittedDiagnostic, DiagnosticAdmissionError> {
    let schema = match error {
        NativeActionInvocationError::MalformedInput(source) => {
            Some(schema_violation_projection(source))
        }
        _ => None,
    };
    let (code, disposition, summary, retry, request_digest) = match error {
        NativeActionInvocationError::MalformedInput(_) => {
            let code = ActionErrorCode::MalformedInput;
            let (disposition, retry) = action_error_policy(code);
            (
                code.as_str().to_owned(),
                disposition,
                error.to_string(),
                retry,
                None,
            )
        }
        NativeActionInvocationError::Action(action) => {
            let (disposition, retry) = action_error_policy(action.code);
            (
                action.code.as_str().to_owned(),
                disposition,
                action.message.clone(),
                retry,
                action.request_digest.as_ref().map(ToString::to_string),
            )
        }
        NativeActionInvocationError::InputSchema(_)
        | NativeActionInvocationError::ExecutionSchema(_)
        | NativeActionInvocationError::RegistryInvariant(_) => (
            "native_action_invariant".to_owned(),
            DiagnosticDispositionV1::InternalFailure,
            "native-action registry or contract invariant failed".to_owned(),
            DiagnosticRetryV1::NotIndicated,
            None,
        ),
    };
    project_action_failure(
        identity,
        action_id,
        error,
        ActionFailure {
            code: &code,
            disposition,
            summary: &summary,
            retry,
            request_digest,
            schema,
        },
    )
}

/// Test whether one admitted diagnostic names an exact failed native request.
///
/// This checks correlation only. The caller still owns record identity,
/// operation history, signer authority and durable custody.
pub(crate) fn matches_native_failure_diagnostic(
    diagnostic: &AdmittedDiagnostic,
    action_id: &str,
    request_digest: &str,
) -> bool {
    let wire = diagnostic.wire();
    wire.stage == DiagnosticStageV1::NativeAction
        && wire.effect == DiagnosticEffectCertaintyV1::NotApplicable
        && wire
            .subject
            .as_ref()
            .is_some_and(|subject| subject.kind == "action" && subject.id == action_id)
        && wire.request.as_ref().is_some_and(|request| {
            request.kind == "native_action_request" && request.id == request_digest
        })
}

/// Project one action boundary's classification through the shared diagnostic owner.
/// # Errors
/// Rejects incompatible diagnostic fields or an unavailable schema coordinate.
pub fn project_action_failure(
    identity: &RecordSchemaIdentity,
    action_id: &str,
    error: &(dyn std::error::Error + 'static),
    failure: ActionFailure<'_>,
) -> Result<AdmittedDiagnostic, DiagnosticAdmissionError> {
    let schema_coordinate = failure.schema.as_ref().and_then(|projection| {
        schema_diagnostic_coordinate("action-input.v1", projection.schema_path.clone())
    });
    let schema = failure.schema.unwrap_or_default();
    project_failure(
        identity,
        error,
        FailureSpecification {
            code: failure.code,
            stage: DiagnosticStageV1::NativeAction,
            disposition: failure.disposition,
            summary: failure.summary,
            subject: (!action_id.is_empty()).then(|| DiagnosticSubjectV1Wire {
                kind: "action".to_string(),
                id: action_id.to_string(),
            }),
            schema_coordinate,
            location: schema.location,
            retry: failure.retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: failure
                .request_digest
                .map(|id| DiagnosticRequestCorrelationV1Wire {
                    kind: "native_action_request".to_string(),
                    id,
                }),
            notes: Vec::new(),
            help: Vec::new(),
            facts: schema.facts,
        },
        CauseProjection::Redacted,
    )
}

/// Map every current action error code to its public recovery policy.
fn action_error_policy(code: ActionErrorCode) -> (DiagnosticDispositionV1, DiagnosticRetryV1) {
    match code {
        ActionErrorCode::OutputMismatch
        | ActionErrorCode::CanonicalizationFailed
        | ActionErrorCode::InternalInvariant => (
            DiagnosticDispositionV1::InternalFailure,
            DiagnosticRetryV1::NotIndicated,
        ),
        ActionErrorCode::UnknownAction
        | ActionErrorCode::MalformedInput
        | ActionErrorCode::ContractMismatch
        | ActionErrorCode::InputMismatch
        | ActionErrorCode::DigestMismatch
        | ActionErrorCode::DuplicateIdentifier
        | ActionErrorCode::InvalidRanking
        | ActionErrorCode::NumericOverflow
        | ActionErrorCode::InvalidPath => (
            DiagnosticDispositionV1::InvalidInput,
            DiagnosticRetryV1::AfterCorrection,
        ),
    }
}
