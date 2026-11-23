//! Portable wire shape for one bounded Pale diagnostic.
//!
//! This module owns structural JSON shape only. The application constructs
//! safe diagnostics and checks their meaning. The Rust SDK exposes that shared
//! behavior through its binding-safe types.

use serde::{Deserialize, Serialize};

use super::{SchemaDigest, SchemaId};

/// Descriptor-registry identity of [`DiagnosticV1Wire`].
pub const DIAGNOSTIC_V1_SCHEMA_ID: &str = "diagnostic.v1";
/// Stable wire identity carried by [`DiagnosticV1Wire::schema`].
pub const DIAGNOSTIC_V1_SCHEMA: &str = "pale/diagnostic.v1";
/// Wire major carried by [`DiagnosticV1Wire::schema_version`].
pub const DIAGNOSTIC_V1_SCHEMA_VERSION: u32 = 1;

wire_enum!(
    /// Processing stage at which a diagnostic was established.
    DiagnosticStageV1 {
        /// External bytes or command values failed initial admission.
        InputAdmission => "input_admission",
        /// A value failed an embedded structural contract.
        ContractValidation => "contract_validation",
        /// A structurally valid value failed owner-specific meaning.
        SemanticAdmission => "semantic_admission",
        /// Distribution Package sources failed loading or verification.
        PackageLoading => "package_loading",
        /// Principal-workspace sources failed loading or verification.
        WorkspaceLoading => "workspace_loading",
        /// Compiler resolution, projection, or sealing failed.
        Compilation => "compilation",
        /// A Pale-rendered surface could not be produced.
        Rendering => "rendering",
        /// An operation was not admitted under current compiled state.
        OperationAdmission => "operation_admission",
        /// A resident generation or durable runtime transition failed admission.
        ResidentAdmission => "resident_admission",
        /// Recovery could not establish a safe continuation or retry posture.
        ResidentRecovery => "resident_recovery",
        /// An effect-free plan could not be constructed or admitted.
        Planning => "planning",
        /// An admitted plan could not be materialized or reconciled.
        Materialization => "materialization",
        /// A pure built-in action failed admission or computation.
        NativeAction => "native_action",
        /// An external prerequisite could not be observed reliably.
        HostObservation => "host_observation",
        /// A Pale application invariant failed outside a narrower stage.
        Internal => "internal"
    }
);

wire_enum!(
    /// Importance of one initial-v1 diagnostic.
    DiagnosticSeverityV1 {
        /// The requested operation did not complete successfully.
        Error => "error"
    }
);

wire_enum!(
    /// Stable recovery-relevant classification of a failure.
    DiagnosticDispositionV1 {
        /// A runtime prerequisite or observation failed operationally.
        OperationalFailure => "operational_failure",
        /// Supplied bytes or authored meaning must be corrected.
        InvalidInput => "invalid_input",
        /// Current admitted state blocks an otherwise meaningful request.
        Blocked => "blocked",
        /// Pale application state or behavior contradicts an invariant.
        InternalFailure => "internal_failure",
        /// An external effect may have occurred and requires reconciliation.
        EffectOutcomeUnknown => "effect_outcome_unknown"
    }
);

wire_enum!(
    /// Bounded retry guidance that never substitutes for idempotency evidence.
    DiagnosticRetryV1 {
        /// No retry instruction is established by this diagnostic.
        NotIndicated => "not_indicated",
        /// Retry only after correcting the rejected input or authored meaning.
        AfterCorrection => "after_correction",
        /// Retry only after the named admitted state or prerequisite changes.
        AfterStateChange => "after_state_change",
        /// Reconcile retained evidence before deciding whether to retry.
        ReconcileFirst => "reconcile_first"
    }
);

wire_enum!(
    /// What the diagnostic proves about an associated external effect.
    DiagnosticEffectCertaintyV1 {
        /// The operation has no external-effect boundary.
        NotApplicable => "not_applicable",
        /// The selected external-effect boundary was proven not to have started.
        NotStarted => "not_started",
        /// The external effect and its result were confirmed by owner evidence.
        Confirmed => "confirmed",
        /// Available evidence cannot establish whether the effect occurred.
        OutcomeUnknown => "outcome_unknown"
    }
);

/// Bounded identity of the item principally affected by a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSubjectV1Wire {
    /// Stable subject family such as `schema`, `operation`, or `action`.
    #[schemars(length(min = 1, max = 128), regex(pattern = r"^[a-z][a-z0-9_]*$"))]
    pub kind: String,
    /// Bounded identity within [`Self::kind`].
    #[schemars(length(min = 1, max = 1_024))]
    pub id: String,
}

/// Safe logical source coordinate with no host-path authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSourceV1Wire {
    /// Root-relative or opaque source label; never an absolute operating-system path.
    #[schemars(length(min = 1, max = 1_024))]
    pub label: String,
    /// One-based source line when the owner supplied it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// One-based source column when the owner supplied it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    /// Inclusive byte offset within the bounded source when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_start: Option<u32>,
    /// Exclusive byte offset within the bounded source when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_end: Option<u32>,
    /// JSON Pointer or typed decoder path within the source value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(max = 1_024))]
    pub value_path: Option<String>,
}

/// Exact embedded contract coordinate implicated by a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSchemaCoordinateV1Wire {
    /// Descriptor-manifest registry identity.
    #[schemars(length(min = 1, max = 256))]
    pub registry_id: String,
    /// Exact wire schema identity when the descriptor declares one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire_schema_id: Option<SchemaId>,
    /// SHA-256 of the selected raw generated JSON Schema.
    pub schema_sha256: SchemaDigest,
    /// JSON Pointer to the governing schema keyword when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(max = 1_024))]
    pub schema_path: Option<String>,
}

/// Redaction-safe display fact that is never evidence or authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticFactV1Wire {
    /// Stable fact name used only for presentation.
    #[schemars(length(min = 1, max = 128), regex(pattern = r"^[a-z][a-z0-9_]*$"))]
    pub key: String,
    /// Bounded redaction-safe display value.
    #[schemars(length(max = 1_024))]
    pub value: String,
}

/// Request identity correlated with one diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticRequestCorrelationV1Wire {
    /// Request family that defines the identity's meaning.
    #[schemars(length(min = 1, max = 128))]
    pub kind: String,
    /// Request identity within [`Self::kind`].
    #[schemars(length(min = 1, max = 1_024))]
    pub id: String,
}

/// Generation identity correlated with one diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticGenerationCorrelationV1Wire {
    /// Generation family that defines the identity's meaning.
    #[schemars(length(min = 1, max = 128))]
    pub kind: String,
    /// Generation identity within [`Self::kind`].
    #[schemars(length(min = 1, max = 1_024))]
    pub id: String,
}

/// Receipt identity correlated with one diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticReceiptCorrelationV1Wire {
    /// Receipt family that defines the identity's meaning.
    #[schemars(length(min = 1, max = 128))]
    pub kind: String,
    /// Receipt identity within [`Self::kind`].
    #[schemars(length(min = 1, max = 1_024))]
    pub id: String,
}

/// Bounded accounting for diagnostic details omitted by projection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticTruncationV1Wire {
    /// Scalar fields shortened to their contract limit.
    pub shortened_fields: u32,
    /// Causal entries omitted after the retained bound, with an unscanned tail
    /// represented as one additional omission when the scan bound is reached.
    pub omitted_causes: u32,
    /// Sibling notes omitted after the retained bound.
    pub omitted_notes: u32,
    /// Recovery suggestions omitted after the retained bound.
    pub omitted_help: u32,
    /// Display facts omitted after the retained bound.
    pub omitted_facts: u32,
}

/// Structurally valid but not yet semantically admitted diagnostic wire value.
///
/// Consumers must use the `pale-sdk` admission boundary. Deserializing this
/// shape alone does not establish safe source labels, cross-field lifecycle
/// meaning, retry correctness, or effect certainty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticV1Wire {
    /// Stable wire identity; currently [`DIAGNOSTIC_V1_SCHEMA`].
    #[schemars(with = "String", extend("const" = DIAGNOSTIC_V1_SCHEMA))]
    pub schema: SchemaId,
    /// SHA-256 of the exact generated JSON Schema selected for this record.
    pub schema_sha256: SchemaDigest,
    /// Numeric wire major paired with [`Self::schema`].
    #[schemars(extend("const" = DIAGNOSTIC_V1_SCHEMA_VERSION))]
    pub schema_version: u32,
    /// Stable machine-readable failure code.
    #[schemars(
        length(min = 1, max = 128),
        regex(pattern = r"^[A-Za-z0-9][A-Za-z0-9_.-]*$")
    )]
    pub code: String,
    /// Processing stage that established the failure.
    pub stage: DiagnosticStageV1,
    /// Importance of this diagnostic.
    pub severity: DiagnosticSeverityV1,
    /// Recovery-relevant failure classification.
    pub disposition: DiagnosticDispositionV1,
    /// Concise redaction-safe explanation.
    #[schemars(length(min = 1, max = 4_096))]
    pub summary: String,
    /// Principally affected typed item when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<DiagnosticSubjectV1Wire>,
    /// Safe logical source coordinate when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<DiagnosticSourceV1Wire>,
    /// Embedded schema coordinate when contract validation is relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_coordinate: Option<DiagnosticSchemaCoordinateV1Wire>,
    /// Ordered redaction-safe source-chain messages after the summary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 16), inner(length(min = 1, max = 4_096)))]
    pub causes: Vec<String>,
    /// Ordered sibling or contextual explanations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 16), inner(length(min = 1, max = 4_096)))]
    pub notes: Vec<String>,
    /// Ordered bounded recovery suggestions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 16), inner(length(min = 1, max = 4_096)))]
    pub help: Vec<String>,
    /// Presentation-only context whose values carry no authority.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 16))]
    pub facts: Vec<DiagnosticFactV1Wire>,
    /// Retry posture established by the owning boundary.
    pub retry: DiagnosticRetryV1,
    /// Proven relationship to an external-effect boundary.
    pub effect: DiagnosticEffectCertaintyV1,
    /// Independently typed request correlation when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<DiagnosticRequestCorrelationV1Wire>,
    /// Independently typed generation correlation when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation: Option<DiagnosticGenerationCorrelationV1Wire>,
    /// Independently typed receipt correlation when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt: Option<DiagnosticReceiptCorrelationV1Wire>,
    /// Bounded-projection omission accounting.
    pub truncation: DiagnosticTruncationV1Wire,
}
