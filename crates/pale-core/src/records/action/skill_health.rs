//! Supplied skill observations reduced over caller-defined time windows.

use serde::{Deserialize, Serialize};

/// Contract and dispatch key for skill-health aggregation in
/// [`crate::records::NativeActionCatalogV1`].
pub const SKILL_HEALTH_AGGREGATE_ACTION_ID: &str = "pale.compute.skill-health-aggregate";

/// Outcome of one skill run at a caller-supplied millisecond timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillRunObservation {
    /// Stable run-observation identity, unique within its skill series.
    #[schemars(length(min = 1, max = 256))]
    pub id: String,
    /// Caller-supplied occurrence time on a shared millisecond basis.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub occurred_at_ms: u64,
    /// Whether the caller classified the run as successful.
    pub succeeded: bool,
}

/// Observations and pending work for one skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillHealthSeries {
    /// Stable skill identity used to group observations and sort output.
    #[schemars(length(min = 1, max = 256))]
    pub skill_id: String,
    /// Caller-supplied count of amendments awaiting review for this skill.
    pub pending_amendments: u32,
    /// Run observations belonging to this skill.
    #[schemars(length(max = 100_000))]
    pub observations: Vec<SkillRunObservation>,
}

/// Skill observations and caller-defined evaluation windows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillHealthInput {
    /// Evaluation time in milliseconds on the caller's shared time basis.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub evaluation_time_ms: u64,
    /// Inclusive lower bound, in milliseconds, for the recent window.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub recent_window_start_ms: u64,
    /// Inclusive lower bound, in milliseconds, for the long window.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub long_window_start_ms: u64,
    /// Minimum fixed-point rate change classified as improving or declining.
    #[schemars(range(max = 1_000_000))]
    pub trend_threshold_micros: u32,
    /// Skill series to aggregate; the action reads no durable health store.
    #[schemars(length(min = 1, max = 1_024))]
    pub skills: Vec<SkillHealthSeries>,
}

wire_enum!(
    /// Comparison of recent and long-window success rates.
    SkillHealthTrend {
        /// Recent success rate exceeds the long rate by at least the threshold.
        Improving => "improving",
        /// Absolute rate change remains below the threshold.
        Stable => "stable",
        /// Recent success rate trails the long rate by at least the threshold.
        Declining => "declining",
        /// At least one comparison window contains no observations.
        InsufficientData => "insufficient_data",
    }
);

/// Run and success totals for one evaluation window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillWindowRate {
    /// Number of observations in this window.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub runs: u64,
    /// Successful observations in this window.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub successes: u64,
    /// Success ratio on a one-million scale, absent when no run was observed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_rate_micros: Option<u32>,
}

/// Health summary for one skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillHealthRow {
    /// Identity of the aggregated skill.
    #[schemars(length(min = 1))]
    pub skill_id: String,
    /// Rate over the inclusive recent window.
    pub recent: SkillWindowRate,
    /// Rate over the inclusive long window, which contains the recent window.
    pub long: SkillWindowRate,
    /// Threshold comparison of recent and long success rates.
    pub trend: SkillHealthTrend,
    /// Caller-supplied pending-amendment count for this skill.
    pub pending_amendments: u32,
    /// Latest supplied run time, absent when the skill has no observations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub last_run_at_ms: Option<u64>,
}

/// Fleet-level totals followed by skill rows in stable skill-ID order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillHealthOutput {
    /// Fixed-point denominator used by every success-rate field.
    pub scale_micros: u32,
    /// Sum of pending amendments across every skill series.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub pending_amendments: u64,
    /// Portfolio rate over the inclusive recent window.
    pub recent: SkillWindowRate,
    /// Portfolio rate over the inclusive long window.
    pub long: SkillWindowRate,
    /// Latest supplied run time across the portfolio, when any run exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub last_run_at_ms: Option<u64>,
    /// Per-skill rows in stable skill-identity order.
    pub skills: Vec<SkillHealthRow>,
}
