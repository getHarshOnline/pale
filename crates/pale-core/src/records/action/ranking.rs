//! Wire records for deterministic fixed-point weighted ranking.
//!
//! Scores and weights are integers so canonical results never depend on host
//! floating-point behavior. The action rejects incomplete candidate matrices
//! instead of inventing a score for missing evidence.

use serde::{Deserialize, Serialize};

/// Contract and dispatch key for weighted ranking in
/// [`crate::records::NativeActionCatalogV1`].
pub const WEIGHTED_RANKING_ACTION_ID: &str = "pale.compute.weighted-ranking";

/// One fixed-point criterion weight. `weight_micros` uses a one-million scale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RankingWeight {
    /// Stable criterion identity referenced by every candidate.
    #[schemars(length(min = 1))]
    pub id: String,
    /// Positive criterion weight on a one-million fixed-point scale.
    #[schemars(range(min = 1, max = 1_000_000))]
    pub weight_micros: u32,
}

/// Fixed-point criterion value keyed by a declared weight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RankingScore {
    /// Identity of the criterion this value scores.
    #[schemars(length(min = 1))]
    pub weight_id: String,
    /// Signed criterion value in the inclusive range `-1_000_000..=1_000_000`.
    #[schemars(range(min = -1_000_000, max = 1_000_000))]
    pub value_micros: i64,
}

/// One item to rank.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RankingCandidate {
    /// Stable candidate identity used for deterministic tie-breaking.
    #[schemars(length(min = 1))]
    pub id: String,
    /// Exactly one score for every declared weight.
    #[schemars(length(max = 1_024))]
    pub scores: Vec<RankingScore>,
}

/// Structured fixed-point ranking input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WeightedRankingInput {
    /// Unique criteria whose positive weights define the ranking.
    #[schemars(length(min = 1, max = 1_024))]
    pub weights: Vec<RankingWeight>,
    /// Unique candidates with complete score matrices.
    #[schemars(length(min = 1, max = 10_000))]
    pub candidates: Vec<RankingCandidate>,
}

/// Ranked value with both its exact ordering key and normalized display score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RankedCandidate {
    /// Candidate identity copied from the input.
    #[schemars(length(min = 1))]
    pub id: String,
    /// Exact sum of `weight_micros * value_micros` across all criteria.
    #[schemars(schema_with = "super::safe_i64_schema")]
    pub weighted_sum: i64,
    /// Truncating fixed-point weighted mean; ordering uses the exact sum.
    #[schemars(schema_with = "super::safe_i64_schema")]
    pub normalized_score_micros: i64,
}

/// Ranking output sorted by exact score descending then ID ascending.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WeightedRankingOutput {
    /// Fixed-point denominator used by weights, scores, and normalized values.
    pub scale_micros: u32,
    /// Sum of all declared criterion weights.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub total_weight_micros: u64,
    /// Candidates sorted by exact weighted sum descending then identity ascending.
    pub ranking: Vec<RankedCandidate>,
}
