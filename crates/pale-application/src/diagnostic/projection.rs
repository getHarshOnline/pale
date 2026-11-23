//! Redact and bound diagnostic candidates before their identity is sealed.

use super::text::{bounded_redacted, bounded_redacted_to};
use super::{AdmittedDiagnostic, DiagnosticAdmissionError};
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticFactV1Wire, DiagnosticRetryV1,
    DiagnosticSchemaCoordinateV1Wire, DiagnosticSeverityV1, DiagnosticSourceV1Wire,
    DiagnosticStageV1, DiagnosticSubjectV1Wire, DiagnosticTruncationV1Wire, DiagnosticV1Wire,
    RecordSchemaIdentity, SchemaDigest, SchemaId, DIAGNOSTIC_V1_SCHEMA_VERSION,
};

/// Maximum members retained in each bounded diagnostic collection.
const MAX_COLLECTION_MEMBERS: usize = 16;
/// Maximum characters retained in an identity or presentation fact.
const MAX_FIELD_CHARS: usize = 1_024;
/// Maximum characters retained in a diagnostic code, kind, or fact key.
const MAX_KEY_CHARS: usize = 128;
/// Maximum characters retained in a descriptor-registry identity.
const MAX_REGISTRY_ID_CHARS: usize = 256;

/// Unadmitted text and classification supplied by an operation owner.
///
/// Admission validates these claims; it does not establish permission or native effects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticProjection {
    /// Stable machine-readable failure code.
    pub code: String,
    /// Boundary that established the failure.
    pub stage: DiagnosticStageV1,
    /// Failure class selected by the owning operation.
    pub disposition: DiagnosticDispositionV1,
    /// Explanation to redact and shorten before admission.
    pub summary: String,
    /// Optional subject kind and identity.
    pub subject: Option<(String, String)>,
    /// Optional logical source location, not a host-path permission.
    pub source: Option<DiagnosticSourceV1Wire>,
    /// Registry identity and optional schema pointer to resolve.
    pub schema_reference: Option<(String, Option<String>)>,
    /// Ordered causal messages to redact and bound.
    pub causes: Vec<String>,
    /// Ordered explanations distinct from the causal chain.
    pub notes: Vec<String>,
    /// Recovery suggestions, not permission to retry.
    pub help: Vec<String>,
    /// Presentation-only keys and values.
    pub facts: Vec<(String, String)>,
    /// Retry posture established by the owning operation.
    pub retry: DiagnosticRetryV1,
    /// What the operation established about external effects.
    pub effect: DiagnosticEffectCertaintyV1,
    /// Optional request kind and identity.
    pub request: Option<(String, String)>,
    /// Optional generation kind and identity.
    pub generation: Option<(String, String)>,
    /// Optional receipt kind and identity.
    pub receipt: Option<(String, String)>,
    /// Causal entries already omitted before this projection.
    pub omitted_causes: u32,
}

impl DiagnosticProjection {
    /// Resolve, bound, validate, and semantically admit this projection.
    /// # Errors
    /// Rejects unknown schema references, invalid source locations and fields
    /// that contradict the diagnostic contract after redaction and bounding.
    pub fn admit(
        self,
        diagnostic_identity: &RecordSchemaIdentity,
    ) -> Result<AdmittedDiagnostic, DiagnosticAdmissionError> {
        let mut shortened_fields = 0_u32;
        let subject = self
            .subject
            .map(|(kind, identity)| DiagnosticSubjectV1Wire {
                kind: bounded_redacted_to(&kind, MAX_KEY_CHARS, &mut shortened_fields),
                id: bounded_redacted_to(&identity, MAX_FIELD_CHARS, &mut shortened_fields),
            });
        let source = self.source.map(|source| DiagnosticSourceV1Wire {
            label: bounded_redacted_to(&source.label, MAX_FIELD_CHARS, &mut shortened_fields),
            line: source.line,
            column: source.column,
            byte_start: source.byte_start,
            byte_end: source.byte_end,
            value_path: source
                .value_path
                .map(|path| bounded_redacted_to(&path, MAX_FIELD_CHARS, &mut shortened_fields)),
        });
        let schema_coordinate = self
            .schema_reference
            .map(|(registry_id, schema_path)| {
                let registry_id = bounded_redacted_to(
                    &registry_id,
                    MAX_REGISTRY_ID_CHARS,
                    &mut shortened_fields,
                );
                let descriptor = match pale_contracts::schema_descriptor(&registry_id) {
                    Ok(descriptor) => descriptor,
                    Err(pale_contracts::SchemaError::UnknownId(_)) => {
                        return Err(DiagnosticAdmissionError::UnknownSchemaCoordinate);
                    }
                    Err(error) => return Err(DiagnosticAdmissionError::Registry(error)),
                };
                let wire_schema_id = descriptor
                    .wire_schema_id
                    .map(SchemaId::parse)
                    .transpose()
                    .map_err(|error| {
                        DiagnosticAdmissionError::Registry(
                            pale_contracts::SchemaError::Integrity(format!(
                                "descriptor `{registry_id}` has an invalid wire identity: {error}"
                            )),
                        )
                    })?;
                let schema_sha256 =
                    SchemaDigest::parse(descriptor.schema_sha256).map_err(|error| {
                        DiagnosticAdmissionError::Registry(
                            pale_contracts::SchemaError::Integrity(format!(
                                "descriptor `{registry_id}` has an invalid raw-schema digest: {error}"
                            )),
                        )
                    })?;
                Ok(DiagnosticSchemaCoordinateV1Wire {
                    registry_id,
                    wire_schema_id,
                    schema_sha256,
                    schema_path: schema_path.map(|path| {
                        bounded_redacted_to(&path, MAX_FIELD_CHARS, &mut shortened_fields)
                    }),
                })
            })
            .transpose()?;
        let (causes, omitted_causes) = bounded_messages(self.causes, &mut shortened_fields);
        let (notes, omitted_notes) = bounded_messages(self.notes, &mut shortened_fields);
        let (help, omitted_help) = bounded_messages(self.help, &mut shortened_fields);
        let omitted_facts = self.facts.len().saturating_sub(MAX_COLLECTION_MEMBERS);
        let facts = self
            .facts
            .into_iter()
            .take(MAX_COLLECTION_MEMBERS)
            .map(|(key, value)| DiagnosticFactV1Wire {
                key: bounded_redacted_to(&key, MAX_KEY_CHARS, &mut shortened_fields),
                value: bounded_redacted_to(&value, MAX_FIELD_CHARS, &mut shortened_fields),
            })
            .collect();
        let request =
            self.request.map(
                |(kind, id)| pale_core::records::DiagnosticRequestCorrelationV1Wire {
                    kind: bounded_redacted_to(&kind, MAX_KEY_CHARS, &mut shortened_fields),
                    id: bounded_redacted_to(&id, MAX_FIELD_CHARS, &mut shortened_fields),
                },
            );
        let generation = self.generation.map(|(kind, id)| {
            pale_core::records::DiagnosticGenerationCorrelationV1Wire {
                kind: bounded_redacted_to(&kind, MAX_KEY_CHARS, &mut shortened_fields),
                id: bounded_redacted_to(&id, MAX_FIELD_CHARS, &mut shortened_fields),
            }
        });
        let receipt =
            self.receipt.map(
                |(kind, id)| pale_core::records::DiagnosticReceiptCorrelationV1Wire {
                    kind: bounded_redacted_to(&kind, MAX_KEY_CHARS, &mut shortened_fields),
                    id: bounded_redacted_to(&id, MAX_FIELD_CHARS, &mut shortened_fields),
                },
            );
        let wire = DiagnosticV1Wire {
            schema: diagnostic_identity.schema.clone(),
            schema_sha256: diagnostic_identity.schema_sha256.clone(),
            schema_version: DIAGNOSTIC_V1_SCHEMA_VERSION,
            code: bounded_redacted_to(&self.code, MAX_KEY_CHARS, &mut shortened_fields),
            stage: self.stage,
            severity: DiagnosticSeverityV1::Error,
            disposition: self.disposition,
            summary: bounded_redacted(&self.summary, &mut shortened_fields),
            subject,
            source,
            schema_coordinate,
            causes,
            notes,
            help,
            facts,
            retry: self.retry,
            effect: self.effect,
            request,
            generation,
            receipt,
            truncation: DiagnosticTruncationV1Wire {
                shortened_fields,
                omitted_causes: self
                    .omitted_causes
                    .saturating_add(usize_to_u32(omitted_causes)),
                omitted_notes: usize_to_u32(omitted_notes),
                omitted_help: usize_to_u32(omitted_help),
                omitted_facts: usize_to_u32(omitted_facts),
            },
        };
        AdmittedDiagnostic::from_checked_projected_wire(wire)
    }
}

/// Bound one ordered diagnostic collection and return its exact omitted suffix.
fn bounded_messages(messages: Vec<String>, shortened_fields: &mut u32) -> (Vec<String>, usize) {
    let omitted = messages.len().saturating_sub(MAX_COLLECTION_MEMBERS);
    let retained = messages
        .into_iter()
        .take(MAX_COLLECTION_MEMBERS)
        .map(|message| bounded_redacted(&message, shortened_fields))
        .collect();
    (retained, omitted)
}

/// Saturate host-sized omission counts at the portable wire width.
fn usize_to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
