//! Validate diagnostic records and derive their content identity.

use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticV1Wire, DIAGNOSTIC_V1_SCHEMA_ID,
};
use std::collections::BTreeSet;

/// A diagnostic failed strict decoding, structural validation, or semantic admission.
#[derive(Debug, thiserror::Error)]
pub enum DiagnosticAdmissionError {
    /// Input bytes violate the strict float-free, duplicate-free JSON grammar.
    #[error("diagnostic is not strict JSON: {0}")]
    StrictJson(#[source] pale_core::canon::CanonError),
    /// Input or constructed wire data violates the registered JSON Schema.
    #[error("diagnostic contract admission failed: {0}")]
    Contract(#[source] pale_contracts::SchemaError),
    /// Schema-valid JSON could not decode into its Rust wire owner.
    #[error("schema-valid diagnostic could not be decoded: {0}")]
    Decode(#[source] serde_json::Error),
    /// An admitted typed value contradicted the canonical record grammar.
    #[error("diagnostic could not be encoded canonically: {0}")]
    Canonicalize(#[source] pale_core::canon::CanonError),
    /// The embedded registry failed while admitting a referenced schema coordinate.
    #[error("diagnostic schema coordinate could not be resolved: {0}")]
    Registry(#[source] pale_contracts::SchemaError),
    /// A source column was supplied without its governing line.
    #[error("diagnostic source column requires a source line")]
    ColumnWithoutLine,
    /// A one-based line or column was encoded as zero.
    #[error("diagnostic source line and column are one-based")]
    ZeroSourcePosition,
    /// A source byte span supplied only one endpoint.
    #[error("diagnostic source byte span requires both start and end")]
    IncompleteByteSpan,
    /// A source byte span ends before it begins.
    #[error("diagnostic source byte span end precedes its start")]
    ReversedByteSpan,
    /// A source label resembles an absolute or parent-traversing host path.
    #[error("diagnostic source label must be logical or root-relative")]
    UnsafeSourceLabel,
    /// External-effect disposition/certainty disagree or omit required reconciliation.
    #[error("effect-unknown diagnostics must require reconciliation consistently")]
    EffectSemanticsMismatch,
    /// Two presentation facts use the same key.
    #[error("diagnostic presentation fact keys must be unique")]
    DuplicateFactKey,
    /// A schema coordinate contains a non-RFC-6901 schema path.
    #[error("diagnostic schema path must be an RFC 6901 JSON Pointer")]
    InvalidSchemaPath,
    /// A schema coordinate named an identity absent from the embedded registry.
    #[error("diagnostic schema coordinate names an unknown registry identity")]
    UnknownSchemaCoordinate,
    /// A schema coordinate's digest or wire identity differs from its descriptor.
    #[error("diagnostic schema coordinate does not match its embedded descriptor")]
    SchemaCoordinateMismatch,
}

/// A diagnostic whose schema, source coordinates and effect claims were checked.
///
/// Construction redacts text first. Reading stored evidence does not redact it again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedDiagnostic {
    wire: DiagnosticV1Wire,
    canonical_json: String,
    identity: String,
}

impl AdmittedDiagnostic {
    /// Decode and validate an existing diagnostic without rewriting its fields.
    ///
    /// The caller must enforce its input byte limit before calling this method.
    /// # Errors
    /// Rejects malformed JSON, schema violations, unsafe source coordinates,
    /// conflicting effect claims and inconsistent referenced schemas.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DiagnosticAdmissionError> {
        let value =
            pale_core::canon::parse_strict(bytes).map_err(DiagnosticAdmissionError::StrictJson)?;
        pale_contracts::validate_value(&value, DIAGNOSTIC_V1_SCHEMA_ID)
            .map_err(DiagnosticAdmissionError::Contract)?;
        let wire = serde_json::from_value(value).map_err(DiagnosticAdmissionError::Decode)?;
        Self::admit_semantics(wire)
    }

    /// Validate a constructed projection before admitting its relational invariants.
    pub(super) fn from_checked_projected_wire(
        wire: DiagnosticV1Wire,
    ) -> Result<Self, DiagnosticAdmissionError> {
        pale_contracts::validate_record(&wire, DIAGNOSTIC_V1_SCHEMA_ID)
            .map_err(DiagnosticAdmissionError::Contract)?;
        Self::admit_semantics(wire)
    }

    /// Admit invariants that are relational or depend on the embedded registry.
    fn admit_semantics(wire: DiagnosticV1Wire) -> Result<Self, DiagnosticAdmissionError> {
        if let Some(source) = &wire.source {
            if source.column.is_some() && source.line.is_none() {
                return Err(DiagnosticAdmissionError::ColumnWithoutLine);
            }
            if source.line == Some(0) || source.column == Some(0) {
                return Err(DiagnosticAdmissionError::ZeroSourcePosition);
            }
            match (source.byte_start, source.byte_end) {
                (Some(start), Some(end)) if start > end => {
                    return Err(DiagnosticAdmissionError::ReversedByteSpan);
                }
                (Some(_), None) | (None, Some(_)) => {
                    return Err(DiagnosticAdmissionError::IncompleteByteSpan);
                }
                _ => {}
            }
            if !safe_source_label(&source.label) {
                return Err(DiagnosticAdmissionError::UnsafeSourceLabel);
            }
        }

        if let Some(coordinate) = &wire.schema_coordinate {
            let descriptor = match pale_contracts::schema_descriptor(&coordinate.registry_id) {
                Ok(descriptor) => descriptor,
                Err(pale_contracts::SchemaError::UnknownId(_)) => {
                    return Err(DiagnosticAdmissionError::UnknownSchemaCoordinate);
                }
                Err(error) => return Err(DiagnosticAdmissionError::Registry(error)),
            };
            if coordinate.schema_sha256.as_str() != descriptor.schema_sha256
                || coordinate
                    .wire_schema_id
                    .as_ref()
                    .map(pale_core::records::SchemaId::as_str)
                    != descriptor.wire_schema_id
            {
                return Err(DiagnosticAdmissionError::SchemaCoordinateMismatch);
            }
            if coordinate
                .schema_path
                .as_deref()
                .is_some_and(|path| jsonptr::Pointer::parse(path).is_err())
            {
                return Err(DiagnosticAdmissionError::InvalidSchemaPath);
            }
        }

        let effect_unknown = wire.effect == DiagnosticEffectCertaintyV1::OutcomeUnknown;
        let disposition_unknown = wire.disposition == DiagnosticDispositionV1::EffectOutcomeUnknown;
        let reconcile_first = wire.retry == pale_core::records::DiagnosticRetryV1::ReconcileFirst;
        if effect_unknown != disposition_unknown || (effect_unknown && !reconcile_first) {
            return Err(DiagnosticAdmissionError::EffectSemanticsMismatch);
        }

        let mut fact_keys = BTreeSet::new();
        if !wire.facts.iter().all(|fact| fact_keys.insert(&fact.key)) {
            return Err(DiagnosticAdmissionError::DuplicateFactKey);
        }
        let canonical_json = pale_core::canon::to_canonical_string(&wire)
            .map_err(DiagnosticAdmissionError::Canonicalize)?;
        let mut identity_preimage = b"pale.diagnostic.identity.v1\0".to_vec();
        identity_preimage.extend_from_slice(canonical_json.as_bytes());
        let identity = pale_core::sha256_hex(&identity_preimage);
        Ok(Self {
            wire,
            canonical_json,
            identity,
        })
    }

    /// Borrow the checked record without granting execution or retry authority.
    #[must_use]
    pub fn wire(&self) -> &DiagnosticV1Wire {
        &self.wire
    }

    /// Borrow the canonical bytes used to derive the identity.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        self.canonical_json.as_bytes()
    }

    /// Borrow the domain-separated content identity.
    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Transfer the checked record, canonical JSON and identity to an SDK projection.
    #[must_use]
    pub fn into_parts(self) -> (DiagnosticV1Wire, String, String) {
        (self.wire, self.canonical_json, self.identity)
    }
}

/// Accept only opaque or root-relative labels with no host-root or parent authority.
fn safe_source_label(label: &str) -> bool {
    if label.starts_with('/')
        || label.starts_with('\\')
        || label.starts_with('~')
        || label.as_bytes().get(1) == Some(&b':')
        || label.chars().any(char::is_control)
    {
        return false;
    }
    !label.split(['/', '\\']).any(|component| component == "..")
}
