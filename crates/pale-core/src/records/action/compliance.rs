//! Classified trace events and declarative temporal compliance rules.

use serde::{Deserialize, Serialize};

/// Contract and dispatch key for compliance-trace grading in
/// [`crate::records::NativeActionCatalogV1`].
pub const COMPLIANCE_TRACE_GRADE_ACTION_ID: &str = "pale.compute.compliance-trace-grade";

/// One caller-classified trace event at a logical sequence position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClassifiedTraceEvent {
    /// Caller-stable event identity used only for validation and attribution.
    #[schemars(length(min = 1, max = 256))]
    pub id: String,
    /// Caller-assigned class matched by compliance steps.
    #[schemars(length(min = 1, max = 256))]
    pub class: String,
    /// Caller-defined logical order; values must be unique.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub sequence: u64,
}

/// Required occurrence count for one event class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ComplianceStep {
    /// Stable step identity referenced by temporal rules.
    #[schemars(length(min = 1, max = 256))]
    pub id: String,
    /// Event class whose occurrences satisfy this step.
    #[schemars(length(min = 1, max = 256))]
    pub event_class: String,
    /// Non-zero number of matching events required for the step to pass.
    #[schemars(range(min = 1))]
    pub minimum_occurrences: u32,
}

/// Temporal relationship between declared compliance steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComplianceTemporalRule {
    /// The first observed occurrence of `first_step_id` must precede the first
    /// observed occurrence of `second_step_id`.
    Before {
        /// Stable rule identity reported in the output.
        #[schemars(length(min = 1, max = 256))]
        id: String,
        /// Step whose earliest occurrence must come first.
        #[schemars(length(min = 1, max = 256))]
        first_step_id: String,
        /// Step whose earliest occurrence must come second.
        #[schemars(length(min = 1, max = 256))]
        second_step_id: String,
    },
    /// Some occurrence of `first_step_id` must follow the latest observed
    /// occurrence of `second_step_id`.
    After {
        /// Stable rule identity reported in the output.
        #[schemars(length(min = 1, max = 256))]
        id: String,
        /// Step whose latest occurrence must come after the reference step.
        #[schemars(length(min = 1, max = 256))]
        first_step_id: String,
        /// Reference step whose latest occurrence establishes the lower bound.
        #[schemars(length(min = 1, max = 256))]
        second_step_id: String,
    },
    /// The event stream must contain the declared steps as a strictly ordered
    /// subsequence; repeated step ids therefore require repeated occurrences.
    Ordered {
        /// Stable rule identity reported in the output.
        #[schemars(length(min = 1, max = 256))]
        id: String,
        /// Step identities that must have strictly increasing event witnesses.
        #[schemars(length(min = 2, max = 256), inner(length(min = 1, max = 256)))]
        step_ids: Vec<String>,
    },
}

/// Classified events, required steps, and temporal rules to grade together.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ComplianceTraceGradeInput {
    /// Already-classified observations; the action does not infer event classes.
    #[schemars(length(max = 10_000))]
    pub events: Vec<ClassifiedTraceEvent>,
    /// Occurrence requirements that contribute equally to the aggregate rate.
    #[schemars(length(min = 1, max = 256))]
    pub required_steps: Vec<ComplianceStep>,
    /// Temporal requirements that contribute equally to the aggregate rate.
    #[schemars(length(max = 512))]
    pub temporal_rules: Vec<ComplianceTemporalRule>,
    /// Minimum passing-rate ratio, on a one-million scale, for promotion eligibility.
    #[schemars(range(min = 1, max = 1_000_000))]
    pub promotion_threshold_micros: u32,
}

/// Result of one required-step or temporal-rule check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ComplianceCheckResult {
    /// Step or temporal-rule identity.
    #[schemars(length(min = 1))]
    pub id: String,
    /// Whether the supplied event trace satisfied that declaration.
    pub passed: bool,
}

wire_enum!(
    /// Whether the measured trace is eligible for stronger deterministic enforcement.
    CompliancePromotion {
        /// The aggregate rate meets or exceeds the caller-supplied threshold.
        Eligible => "eligible",
        /// The aggregate rate is below the caller-supplied threshold.
        NotEligible => "not_eligible",
    }
);

/// Stable per-check results and aggregate promotion eligibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ComplianceTraceGradeOutput {
    /// Fixed-point denominator used by `compliance_rate_micros`.
    pub scale_micros: u32,
    /// Number of passing step and temporal-rule checks.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub passed_checks: u64,
    /// Total number of equally weighted checks.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub total_checks: u64,
    /// Passed-to-total check ratio on a one-million fixed-point scale.
    pub compliance_rate_micros: u32,
    /// Required-step results sorted by step identity.
    pub steps: Vec<ComplianceCheckResult>,
    /// Temporal-rule results sorted by rule identity.
    pub temporal_rules: Vec<ComplianceCheckResult>,
    /// Threshold comparison derived from `compliance_rate_micros`.
    pub promotion: CompliancePromotion,
}
