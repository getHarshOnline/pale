//! Closed request and result envelopes for native Rust actions.
//!
//! [`ActionInput`] and [`ActionOutput`] keep payload variants coupled to their
//! wire discriminators. The application registry verifies the additional
//! action-to-input/output pairing before dispatch.

use serde::{Deserialize, Serialize};

use super::{
    ActionInputKind, ActionOutputKind, ComplianceTraceGradeInput, ComplianceTraceGradeOutput,
    ConfigInvariantScanInput, ConfigInvariantScanOutput, CostAggregateInput, CostAggregateOutput,
    EvidenceSelectionInput, EvidenceSelectionOutput, LoopStatusInput, LoopStatusOutput,
    PathInventoryInput, PathInventoryOutput, ReviewConvergenceInput, ReviewConvergenceOutput,
    RubricEvaluationInput, RubricEvaluationOutput, ServiceDefinitionInput, ServiceDefinitionOutput,
    SkillHealthInput, SkillHealthOutput, WeightedRankingInput, WeightedRankingOutput,
};

/// Closed action request payload.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", content = "payload", deny_unknown_fields)]
pub enum ActionInput {
    /// Grade classified trace events against declared occurrence and temporal rules.
    #[serde(rename = "compliance_trace_grade")]
    ComplianceTraceGrade(ComplianceTraceGradeInput),
    /// Scan supplied text for required and forbidden literal invariants.
    #[serde(rename = "config_invariant_scan")]
    ConfigInvariantScan(ConfigInvariantScanInput),
    /// Aggregate the latest cumulative cost snapshot for each session.
    #[serde(rename = "cost_aggregate")]
    CostAggregate(CostAggregateInput),
    /// Reduce normalized lifecycle observations into loop-health signals.
    #[serde(rename = "loop_status_reduce")]
    LoopStatusReduce(LoopStatusInput),
    /// Converge review findings and required verifier dimensions.
    #[serde(rename = "review_convergence_reduce")]
    ReviewConvergenceReduce(ReviewConvergenceInput),
    /// Normalize a weighted rubric from caller-scored checks.
    #[serde(rename = "rubric_evaluation")]
    RubricEvaluation(RubricEvaluationInput),
    /// Validate and order provider-neutral resident-service definitions.
    #[serde(rename = "service_definition_normalize")]
    ServiceDefinitionNormalize(ServiceDefinitionInput),
    /// Aggregate supplied skill observations over explicit windows.
    #[serde(rename = "skill_health_aggregate")]
    SkillHealthAggregate(SkillHealthInput),
    /// Rank candidates using a complete fixed-point score matrix.
    #[serde(rename = "weighted_ranking")]
    WeightedRanking(WeightedRankingInput),
    /// Normalize and count supplied paths under a portable lexical grammar.
    #[serde(rename = "path_inventory")]
    PathInventory(PathInventoryInput),
    /// Select declared rules from observed evidence identities.
    #[serde(rename = "evidence_selection")]
    EvidenceSelection(EvidenceSelectionInput),
}

impl std::fmt::Debug for ActionInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ActionInput")
            .field("kind", &self.kind())
            .field("payload", &"<redacted>")
            .finish()
    }
}

impl ActionInput {
    /// Closed payload discriminator.
    #[must_use]
    pub fn kind(&self) -> ActionInputKind {
        match self {
            Self::ComplianceTraceGrade(_) => ActionInputKind::ComplianceTraceGrade,
            Self::ConfigInvariantScan(_) => ActionInputKind::ConfigInvariantScan,
            Self::CostAggregate(_) => ActionInputKind::CostAggregate,
            Self::LoopStatusReduce(_) => ActionInputKind::LoopStatusReduce,
            Self::ReviewConvergenceReduce(_) => ActionInputKind::ReviewConvergenceReduce,
            Self::RubricEvaluation(_) => ActionInputKind::RubricEvaluation,
            Self::ServiceDefinitionNormalize(_) => ActionInputKind::ServiceDefinitionNormalize,
            Self::SkillHealthAggregate(_) => ActionInputKind::SkillHealthAggregate,
            Self::WeightedRanking(_) => ActionInputKind::WeightedRanking,
            Self::PathInventory(_) => ActionInputKind::PathInventory,
            Self::EvidenceSelection(_) => ActionInputKind::EvidenceSelection,
        }
    }
}

/// Closed action result payload.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", content = "payload", deny_unknown_fields)]
pub enum ActionOutput {
    /// Per-check compliance results and aggregate promotion eligibility.
    #[serde(rename = "compliance_trace_grade")]
    ComplianceTraceGrade(ComplianceTraceGradeOutput),
    /// Literal-match evidence and aggregate invariant status.
    #[serde(rename = "config_invariant_scan")]
    ConfigInvariantScan(ConfigInvariantScanOutput),
    /// Latest-snapshot cost totals grouped by date and model.
    #[serde(rename = "cost_aggregate")]
    CostAggregate(CostAggregateOutput),
    /// Stale-work, wakeup, failure, and intervention signals.
    #[serde(rename = "loop_status_reduce")]
    LoopStatusReduce(LoopStatusOutput),
    /// Deduplicated findings and fail-closed review recommendation.
    #[serde(rename = "review_convergence_reduce")]
    ReviewConvergenceReduce(ReviewConvergenceOutput),
    /// Normalized category scores and ranked rubric failures.
    #[serde(rename = "rubric_evaluation")]
    RubricEvaluation(RubricEvaluationOutput),
    /// Validated service definitions in dependency-compatible order.
    #[serde(rename = "service_definition_normalize")]
    ServiceDefinitionNormalize(ServiceDefinitionOutput),
    /// Windowed skill-health rates and trends.
    #[serde(rename = "skill_health_aggregate")]
    SkillHealthAggregate(SkillHealthOutput),
    /// Candidates ordered by exact weighted score.
    #[serde(rename = "weighted_ranking")]
    WeightedRanking(WeightedRankingOutput),
    /// Sorted lexical path inventory with occurrence counts.
    #[serde(rename = "path_inventory")]
    PathInventory(PathInventoryOutput),
    /// Matching rules and observed signals unused by those matches.
    #[serde(rename = "evidence_selection")]
    EvidenceSelection(EvidenceSelectionOutput),
}

impl std::fmt::Debug for ActionOutput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ActionOutput")
            .field("kind", &self.kind())
            .field("payload", &"<redacted>")
            .finish()
    }
}

impl ActionOutput {
    /// Closed payload discriminator.
    #[must_use]
    pub fn kind(&self) -> ActionOutputKind {
        match self {
            Self::ComplianceTraceGrade(_) => ActionOutputKind::ComplianceTraceGrade,
            Self::ConfigInvariantScan(_) => ActionOutputKind::ConfigInvariantScan,
            Self::CostAggregate(_) => ActionOutputKind::CostAggregate,
            Self::LoopStatusReduce(_) => ActionOutputKind::LoopStatusReduce,
            Self::ReviewConvergenceReduce(_) => ActionOutputKind::ReviewConvergenceReduce,
            Self::RubricEvaluation(_) => ActionOutputKind::RubricEvaluation,
            Self::ServiceDefinitionNormalize(_) => ActionOutputKind::ServiceDefinitionNormalize,
            Self::SkillHealthAggregate(_) => ActionOutputKind::SkillHealthAggregate,
            Self::WeightedRanking(_) => ActionOutputKind::WeightedRanking,
            Self::PathInventory(_) => ActionOutputKind::PathInventory,
            Self::EvidenceSelection(_) => ActionOutputKind::EvidenceSelection,
        }
    }
}
