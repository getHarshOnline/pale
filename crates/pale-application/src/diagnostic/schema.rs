//! Safe schema coordinates and first-violation facts for diagnostics.

use pale_core::records::{
    DiagnosticFactV1Wire, DiagnosticSchemaCoordinateV1Wire, DiagnosticSourceV1Wire,
};

/// Structured first-violation projection shared by contract-facing diagnostics.
#[derive(Default)]
pub struct SchemaViolationProjection {
    /// Safe logical input coordinate and rejected instance pointer.
    pub location: Option<DiagnosticSourceV1Wire>,
    /// Governing schema-keyword pointer.
    pub schema_path: Option<String>,
    /// Bounded presentation details that require no prose parsing.
    pub facts: Vec<DiagnosticFactV1Wire>,
}

/// Resolve one diagnostic schema coordinate from the canonical descriptor owner.
pub fn schema_diagnostic_coordinate(
    schema_id: &str,
    schema_path: Option<String>,
) -> Option<DiagnosticSchemaCoordinateV1Wire> {
    let descriptor = pale_contracts::schema_descriptor(schema_id).ok()?;
    Some(DiagnosticSchemaCoordinateV1Wire {
        registry_id: schema_id.to_string(),
        wire_schema_id: descriptor
            .wire_schema_id
            .and_then(|identity| pale_core::records::SchemaId::parse(identity).ok()),
        schema_sha256: pale_core::records::SchemaDigest::parse(descriptor.schema_sha256).ok()?,
        schema_path,
    })
}

/// Project the first deterministic schema violation without copying rejected values.
pub fn schema_violation_projection(
    error: &pale_contracts::SchemaError,
) -> SchemaViolationProjection {
    let pale_contracts::SchemaError::Invalid(violations) = error else {
        return SchemaViolationProjection::default();
    };
    let Some(first) = violations.violations().first() else {
        return SchemaViolationProjection::default();
    };
    SchemaViolationProjection {
        location: Some(DiagnosticSourceV1Wire {
            label: "input".to_string(),
            line: None,
            column: None,
            byte_start: None,
            byte_end: None,
            value_path: Some(first.instance_path().to_string()),
        }),
        schema_path: Some(first.schema_path().to_string()),
        facts: vec![
            DiagnosticFactV1Wire {
                key: "schema_keyword".to_string(),
                value: first.keyword().to_string(),
            },
            DiagnosticFactV1Wire {
                key: "schema_expectation".to_string(),
                value: first.expectation().to_string(),
            },
            DiagnosticFactV1Wire {
                key: "retained_violations".to_string(),
                value: violations.violations().len().to_string(),
            },
            DiagnosticFactV1Wire {
                key: "omitted_violations".to_string(),
                value: violations.truncated_count().to_string(),
            },
        ],
    }
}
