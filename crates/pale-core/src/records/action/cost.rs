//! Cumulative session-cost snapshots and deterministic aggregates.

use serde::{Deserialize, Serialize};

/// Contract and dispatch key for cost aggregation in
/// [`crate::records::NativeActionCatalogV1`].
pub const COST_AGGREGATE_ACTION_ID: &str = "pale.compute.cost-aggregate";

/// One cumulative cost reading for a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CostSnapshot {
    /// Session whose cumulative cost this snapshot measures.
    #[schemars(length(min = 1, max = 256))]
    pub session_id: String,
    /// Monotonic ordering key within the session; the greatest index wins.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub snapshot_index: u64,
    /// Calendar date in `YYYY-MM-DD` form.
    #[schemars(regex(pattern = r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$"))]
    pub date: String,
    /// Model identity associated with this cumulative reading.
    #[schemars(length(min = 1, max = 256))]
    pub model: String,
    /// Cumulative session cost in millionths of a US dollar.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub cost_micro_usd: u64,
}

/// Cumulative snapshots and the caller-supplied date treated as today.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CostAggregateInput {
    /// Calendar date in `YYYY-MM-DD` form; future snapshots are rejected.
    #[schemars(regex(pattern = r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$"))]
    pub today: String,
    /// Cumulative readings; per-session index, date, and cost must be monotonic.
    #[schemars(length(max = 100_000))]
    pub snapshots: Vec<CostSnapshot>,
}

/// Aggregate for one date-and-model pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CostAggregateRow {
    /// Snapshot date shared by this row's selected sessions.
    #[schemars(length(min = 1))]
    pub date: String,
    /// Model identity shared by this row's selected sessions.
    #[schemars(length(min = 1))]
    pub model: String,
    /// Number of selected latest session snapshots in this group.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub sessions: u64,
    /// Sum of the selected cumulative session costs, in micro-USD.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub cost_micro_usd: u64,
}

/// Latest-snapshot totals with rows sorted by date then model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CostAggregateOutput {
    /// Number of sessions contributing their greatest-index snapshot.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub selected_sessions: u64,
    /// Sum of selected cumulative session costs, in micro-USD.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub total_cost_micro_usd: u64,
    /// Selected cost whose snapshot date equals the requested `today`, in micro-USD.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub today_cost_micro_usd: u64,
    /// Rows in ascending date then model order.
    pub by_day_model: Vec<CostAggregateRow>,
}
