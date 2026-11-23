//! Saved materialization-plan decoding and equality admission.
//!
//! Host leaves own bounded file reads. This module owns every semantic step
//! after capture: strict JSON parsing, contract validation, typed decoding,
//! record-identity admission, and delegation to the core equality gate.

use pale_core::admission::{AdmissionError, AdmittedPlan};
use pale_core::records::{MaterializationPlanV1, PlanKind, RecordSchemaIdentity};

/// Maximum complete saved-plan document accepted by semantic admission.
///
/// A host adapter should apply the same bound while reading so a truncated
/// document cannot reach this boundary as if it were complete.
pub const SAVED_PLAN_BYTE_LIMIT: usize = 16 * 1024 * 1024;

/// Closed failures from complete saved-plan semantic admission.
#[derive(Debug, thiserror::Error)]
pub enum SavedPlanAdmissionError {
    /// The caller supplied more bytes than the bounded contract permits.
    #[error("saved materialization plan exceeds the {limit}-byte admission bound")]
    ByteLimitExceeded {
        /// Maximum accepted complete document size.
        limit: usize,
    },
    /// The captured document is not strict JSON.
    #[error("saved materialization plan is not strict JSON: {0}")]
    Syntax(#[source] pale_core::canon::CanonError),
    /// The JSON value violates `materialization-plan.v1`.
    #[error("saved materialization plan violates its contract: {0}")]
    Contract(#[source] Box<pale_contracts::SchemaError>),
    /// The validated value cannot decode into its owning Rust record.
    #[error("saved materialization plan cannot be decoded: {0}")]
    Decode(#[source] serde_json::Error),
    /// A carried record identity does not match the embedded registry.
    #[error("saved materialization plan has an inadmissible {coordinate} identity: {source}")]
    RecordIdentity {
        /// Owning record whose identity was rejected.
        coordinate: &'static str,
        /// Exact embedded-registry admission failure.
        #[source]
        source: Box<pale_contracts::SchemaError>,
    },
    /// Core equality, self-digest, schema, or operation-class admission failed.
    #[error("saved materialization plan was not admitted: {0}")]
    Admission(#[source] AdmissionError),
}

impl SavedPlanAdmissionError {
    /// Classify the failure once for every external diagnostic projection.
    #[must_use]
    pub const fn disposition(&self) -> crate::ApplicationFailureDisposition {
        match self {
            Self::Admission(AdmissionError::Stale) => crate::ApplicationFailureDisposition::Blocked,
            Self::Admission(AdmissionError::Canon(_)) => {
                crate::ApplicationFailureDisposition::Internal
            }
            Self::ByteLimitExceeded { .. }
            | Self::Syntax(_)
            | Self::Contract(_)
            | Self::Decode(_)
            | Self::RecordIdentity { .. }
            | Self::Admission(
                AdmissionError::Forged
                | AdmissionError::KindMismatch { .. }
                | AdmissionError::UnsupportedSchema
                | AdmissionError::ForgedSurfacePlan,
            ) => crate::ApplicationFailureDisposition::InvalidInput,
        }
    }
}

/// Admit captured saved-plan bytes against one freshly rebuilt plan.
///
/// The returned [`AdmittedPlan`] contains the rebuilt record. Supplied bytes
/// never become effect authority merely because they decode; core admission
/// must prove their class, self-digests, and exact semantic equality.
///
/// # Errors
///
/// Returns [`SavedPlanAdmissionError`] for an oversized, malformed,
/// contract-invalid, wrongly identified, forged, stale, or wrong-class plan,
/// or when canonical identity reproduction fails.
pub fn admit_saved_materialization_plan(
    saved_bytes: &[u8],
    rebuilt: &MaterializationPlanV1,
    expected_kind: PlanKind,
) -> Result<AdmittedPlan, SavedPlanAdmissionError> {
    if saved_bytes.len() > SAVED_PLAN_BYTE_LIMIT {
        return Err(SavedPlanAdmissionError::ByteLimitExceeded {
            limit: SAVED_PLAN_BYTE_LIMIT,
        });
    }
    let value =
        pale_core::canon::parse_strict(saved_bytes).map_err(SavedPlanAdmissionError::Syntax)?;
    pale_contracts::validate_value(&value, "materialization-plan.v1")
        .map_err(Box::new)
        .map_err(SavedPlanAdmissionError::Contract)?;
    let saved: MaterializationPlanV1 =
        serde_json::from_value(value).map_err(SavedPlanAdmissionError::Decode)?;
    admit_identity(
        "materialization-plan.v1",
        RecordSchemaIdentity {
            schema: saved.schema.clone(),
            schema_sha256: saved.schema_sha256.clone(),
        },
    )?;
    admit_identity(
        "agent-surface-plan.v1",
        RecordSchemaIdentity {
            schema: saved.surface_plan.schema.clone(),
            schema_sha256: saved.surface_plan.schema_sha256.clone(),
        },
    )?;
    pale_core::admission::admit_plan(&saved, rebuilt, expected_kind)
        .map_err(SavedPlanAdmissionError::Admission)
}

/// Admit one carried generated-record identity against the embedded registry.
fn admit_identity(
    coordinate: &'static str,
    identity: RecordSchemaIdentity,
) -> Result<(), SavedPlanAdmissionError> {
    pale_contracts::admit_record_schema_identity(coordinate, &identity).map_err(|source| {
        SavedPlanAdmissionError::RecordIdentity {
            coordinate,
            source: Box::new(source),
        }
    })
}
