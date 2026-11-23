//! Construct one diagnostic from an operation's classification and typed cause.

use super::text::{bounded_causes, causal_depth};
use super::{AdmittedDiagnostic, DiagnosticAdmissionError, DiagnosticProjection};
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
    RecordSchemaIdentity,
};
use std::error::Error;

/// Owner-supplied classification separated from generic redaction and bounding.
pub struct FailureSpecification<'a> {
    /// Stable diagnostic code.
    pub code: &'a str,
    /// Boundary that established the failure.
    pub stage: DiagnosticStageV1,
    /// Recovery-relevant failure class.
    pub disposition: DiagnosticDispositionV1,
    /// Concise source text sanitized by [`project_failure`].
    pub summary: &'a str,
    /// Optional affected item.
    pub subject: Option<pale_core::records::DiagnosticSubjectV1Wire>,
    /// Optional exact embedded contract coordinate.
    pub schema_coordinate: Option<pale_core::records::DiagnosticSchemaCoordinateV1Wire>,
    /// Optional safe logical coordinate within the rejected input.
    pub location: Option<pale_core::records::DiagnosticSourceV1Wire>,
    /// Retry policy selected by the owning boundary.
    pub retry: DiagnosticRetryV1,
    /// Proven relationship to any external effect.
    pub effect: DiagnosticEffectCertaintyV1,
    /// Optional typed request correlation.
    pub request: Option<pale_core::records::DiagnosticRequestCorrelationV1Wire>,
    /// Optional ordered owner-authored explanation distinct from source causes.
    pub notes: Vec<&'a str>,
    /// Optional ordered recovery suggestions.
    pub help: Vec<&'a str>,
    /// Optional presentation facts whose values are sanitized centrally.
    pub facts: Vec<pale_core::records::DiagnosticFactV1Wire>,
}

/// Whether rendered source messages are safe to project into a diagnostic.
#[derive(Clone, Copy, Debug)]
pub enum CauseProjection {
    /// Retain the bounded, credential-redacted causal text.
    Redacted,
    /// Preserve the typed cause privately without exposing its rendered chain.
    Omitted,
}

/// Redact a selected causal chain and admit the owner's diagnostic classification.
///
/// Opaque causes are counted without formatting their messages. This function
/// neither retries the operation nor changes its effect certainty.
/// # Errors
/// Rejects fields that violate the diagnostic contract or its cross-field rules.
pub fn project_failure(
    identity: &RecordSchemaIdentity,
    source: &(dyn Error + 'static),
    specification: FailureSpecification<'_>,
    cause_projection: CauseProjection,
) -> Result<AdmittedDiagnostic, DiagnosticAdmissionError> {
    let (causes, omitted_causes) = match cause_projection {
        CauseProjection::Redacted => bounded_causes(source, specification.summary),
        CauseProjection::Omitted => (Vec::new(), causal_depth(source)),
    };
    DiagnosticProjection {
        code: specification.code.to_string(),
        stage: specification.stage,
        disposition: specification.disposition,
        summary: specification.summary.to_string(),
        subject: specification.subject.map(|value| (value.kind, value.id)),
        source: specification.location,
        schema_reference: specification
            .schema_coordinate
            .map(|value| (value.registry_id, value.schema_path)),
        causes,
        notes: specification
            .notes
            .into_iter()
            .map(str::to_string)
            .collect(),
        help: specification.help.into_iter().map(str::to_string).collect(),
        facts: specification
            .facts
            .into_iter()
            .map(|value| (value.key, value.value))
            .collect(),
        retry: specification.retry,
        effect: specification.effect,
        request: specification.request.map(|value| (value.kind, value.id)),
        generation: None,
        receipt: None,
        omitted_causes,
    }
    .admit(identity)
}
