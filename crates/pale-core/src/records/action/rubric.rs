//! Integer rubric inputs and normalized scorecard outputs.

use serde::{Deserialize, Serialize};

/// Contract and dispatch key for rubric evaluation in
/// [`crate::records::NativeActionCatalogV1`].
pub const RUBRIC_EVALUATION_ACTION_ID: &str = "pale.compute.rubric-evaluation";

/// One rubric category weighted on a one-million fixed-point scale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RubricCategory {
    /// Stable category identity referenced by check results.
    #[schemars(length(min = 1, max = 256))]
    pub id: String,
    /// Positive category weight on a one-million fixed-point scale.
    #[schemars(range(min = 1, max = 1_000_000))]
    pub weight_micros: u32,
}

/// Points earned by one check within a declared category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RubricCheckResult {
    /// Stable check identity used in ranked failures.
    #[schemars(length(min = 1, max = 256))]
    pub id: String,
    /// Identity of the category receiving these points.
    #[schemars(length(min = 1, max = 256))]
    pub category_id: String,
    /// Caller-assigned earned points, no greater than `available_points`.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub earned_points: u64,
    /// Non-zero maximum points available for this check.
    #[schemars(schema_with = "crate::canon::formatted_positive_json_safe_u64_schema")]
    pub available_points: u64,
}

/// Rubric definition and check results to evaluate together.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RubricEvaluationInput {
    /// Weighted categories, each of which must receive at least one check.
    #[schemars(length(min = 1, max = 128))]
    pub categories: Vec<RubricCategory>,
    /// Caller-scored checks; the action does not inspect or assign scores.
    #[schemars(length(min = 1, max = 4_096))]
    pub checks: Vec<RubricCheckResult>,
    /// Maximum number of highest weighted losses retained in the output.
    #[schemars(range(max = 64))]
    pub top_failure_limit: u32,
}

/// Aggregate score for one category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RubricCategoryScore {
    /// Identity of the aggregated category.
    #[schemars(length(min = 1))]
    pub category_id: String,
    /// Sum of earned points across the category's checks.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub earned_points: u64,
    /// Sum of available points across the category's checks.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub available_points: u64,
    /// Earned-to-available ratio on a one-million fixed-point scale.
    pub score_micros: u32,
}

/// One failed check ranked by weighted loss.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RubricFailure {
    /// Identity of the check that lost points.
    #[schemars(length(min = 1))]
    pub check_id: String,
    /// Category whose weight contributes to the loss product.
    #[schemars(length(min = 1))]
    pub category_id: String,
    /// Difference between available and earned points.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub lost_points: u64,
    /// Fixed-point category weight multiplied by the check's lost-point ratio.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub weighted_loss: u64,
}

/// Normalized scorecard with failures ordered by weighted loss then identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RubricEvaluationOutput {
    /// Fixed-point denominator used by category weights and normalized scores.
    pub scale_micros: u32,
    /// Sum of all category weights.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub total_weight_micros: u64,
    /// Weighted mean of category scores on the one-million scale.
    pub overall_score_micros: u32,
    /// Category aggregates sorted by category identity.
    pub categories: Vec<RubricCategoryScore>,
    /// Highest weighted losses, bounded by the requested limit.
    pub top_failures: Vec<RubricFailure>,
}
