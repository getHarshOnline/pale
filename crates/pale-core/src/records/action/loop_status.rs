//! Normalized loop events and deterministic intervention signals.

use serde::{Deserialize, Serialize};

/// Contract and dispatch key for loop-status reduction in
/// [`crate::records::NativeActionCatalogV1`].
pub const LOOP_STATUS_REDUCE_ACTION_ID: &str = "pale.compute.loop-status-reduce";

wire_enum!(
    /// Tool-use class assigned by the host adapter before reduction.
    LoopToolKind {
        /// Shell work subject to the caller-supplied stale-work threshold.
        Bash => "bash",
        /// Any other tool use, tracked for result integrity but not Bash staleness.
        Other => "other",
    }
);

/// One normalized lifecycle observation with millisecond timestamps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LoopEvent {
    /// Start of one tool invocation.
    ToolUse {
        /// Unique event identity and tool-use reference target.
        #[schemars(length(min = 1, max = 256))]
        id: String,
        /// Adapter-classified tool family.
        tool_kind: LoopToolKind,
        /// Start time on the caller's shared millisecond basis.
        #[schemars(schema_with = "super::safe_u64_schema")]
        started_at_ms: u64,
    },
    /// Terminal result for one prior tool invocation.
    ToolResult {
        /// Unique result-event identity reported when the tool failed.
        #[schemars(length(min = 1, max = 256))]
        id: String,
        /// Identity of the corresponding `ToolUse` event.
        #[schemars(length(min = 1, max = 256))]
        tool_use_id: String,
        /// Completion time on the caller's shared millisecond basis.
        #[schemars(schema_with = "super::safe_u64_schema")]
        completed_at_ms: u64,
        /// Whether the host reported successful completion.
        succeeded: bool,
    },
    /// Declaration that a future wakeup should occur.
    WakeupScheduled {
        /// Unique event identity and wakeup-observation reference target.
        #[schemars(length(min = 1, max = 256))]
        id: String,
        /// Scheduled time on the caller's shared millisecond basis.
        #[schemars(schema_with = "super::safe_u64_schema")]
        due_at_ms: u64,
    },
    /// Observation of one previously scheduled wakeup.
    WakeupObserved {
        /// Unique observation-event identity.
        #[schemars(length(min = 1, max = 256))]
        id: String,
        /// Identity of the corresponding `WakeupScheduled` event.
        #[schemars(length(min = 1, max = 256))]
        wakeup_id: String,
        /// Observation time on the caller's shared millisecond basis.
        #[schemars(schema_with = "super::safe_u64_schema")]
        observed_at_ms: u64,
    },
    /// Adapter-reported failure to parse lifecycle evidence.
    ParseError {
        /// Unique parse-error event identity.
        #[schemars(length(min = 1, max = 256))]
        id: String,
        /// Observation time on the caller's shared millisecond basis.
        #[schemars(schema_with = "super::safe_u64_schema")]
        observed_at_ms: u64,
    },
}

/// Loop observations and caller-selected timeout policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LoopStatusInput {
    /// Evaluation time in milliseconds on the adapter's shared time basis.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub now_ms: u64,
    /// Age in milliseconds after which an unfinished Bash use is stale.
    #[schemars(schema_with = "crate::canon::formatted_positive_json_safe_u64_schema")]
    pub bash_stale_after_ms: u64,
    /// Delay in milliseconds after which an unobserved wakeup is overdue.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub wakeup_grace_ms: u64,
    /// Normalized observations; input ordering has no semantic effect.
    #[schemars(length(max = 50_000))]
    pub events: Vec<LoopEvent>,
}

/// Unfinished Bash tool use older than the supplied threshold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StaleToolUse {
    /// Identity of the unfinished Bash `ToolUse` event.
    #[schemars(length(min = 1))]
    pub tool_use_id: String,
    /// Elapsed milliseconds at evaluation time.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub age_ms: u64,
}

/// Scheduled wakeup not observed within its supplied grace period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OverdueWakeup {
    /// Identity of the unobserved `WakeupScheduled` event.
    #[schemars(length(min = 1))]
    pub wakeup_id: String,
    /// Milliseconds elapsed beyond the due time and grace period.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub overdue_ms: u64,
}

wire_enum!(
    /// Fail-closed reduction of the supplied loop evidence.
    LoopRecommendation {
        /// No supplied evidence requires intervention.
        Continue => "continue",
        /// Stale work, an overdue wakeup, or a failed tool requires attention.
        Intervene => "intervene",
        /// A parse error makes the supplied loop evidence unsafe to continue.
        Stop => "stop",
    }
);

/// Deterministic loop health summary; identifier lists are stably ordered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LoopStatusOutput {
    /// Unfinished Bash uses at or beyond the stale threshold, sorted by identity.
    pub stale_bash: Vec<StaleToolUse>,
    /// Unobserved wakeups past their grace deadline, sorted by identity.
    pub overdue_wakeups: Vec<OverdueWakeup>,
    /// Failed result-event identities in stable order.
    #[schemars(inner(length(min = 1)))]
    pub failed_tool_result_ids: Vec<String>,
    /// Parse-error event identities in stable order.
    #[schemars(inner(length(min = 1)))]
    pub parse_error_ids: Vec<String>,
    /// Strongest response required by the supplied evidence.
    pub recommendation: LoopRecommendation,
}
