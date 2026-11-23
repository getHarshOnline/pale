//! Pure, deterministic native-action records.
//!
//! These records describe bounded mechanics that operate only on supplied data.
//! They do not grant filesystem, process, network, clock, randomness, or model
//! access. Nuanced diagnosis and judgment remain outside this layer.

mod compliance;
mod config_invariant;
mod cost;
mod evidence_selection;
mod loop_status;
mod path_inventory;
mod payload;
mod protocol;
mod ranking;
mod review_convergence;
mod rubric;
mod service_definition;
mod skill_health;

pub(super) use crate::canon::json_safe_u64_schema as safe_u64_schema;

/// JSON Schema for a signed integer that round-trips through canonical JSON.
pub(super) fn safe_i64_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    let maximum = crate::canon::JSON_SAFE_INTEGER_MAX as i64;
    schemars::Schema::try_from(serde_json::json!({
        "type": "integer",
        "minimum": -maximum,
        "maximum": maximum
    }))
    .expect("the static canonical signed-integer schema is valid")
}

pub use compliance::{
    ClassifiedTraceEvent, ComplianceCheckResult, CompliancePromotion, ComplianceStep,
    ComplianceTemporalRule, ComplianceTraceGradeInput, ComplianceTraceGradeOutput,
    COMPLIANCE_TRACE_GRADE_ACTION_ID,
};
pub use config_invariant::{
    ConfigInvariant, ConfigInvariantEvidence, ConfigInvariantExpectation, ConfigInvariantResult,
    ConfigInvariantScanInput, ConfigInvariantScanOutput, ConfigTextRecord,
    CONFIG_INVARIANT_SCAN_ACTION_ID,
};
pub use cost::{
    CostAggregateInput, CostAggregateOutput, CostAggregateRow, CostSnapshot,
    COST_AGGREGATE_ACTION_ID,
};
pub use evidence_selection::{
    EvidenceMatch, EvidenceRule, EvidenceSelectionInput, EvidenceSelectionMode,
    EvidenceSelectionOutput, EVIDENCE_SELECTION_ACTION_ID,
};
pub use loop_status::{
    LoopEvent, LoopRecommendation, LoopStatusInput, LoopStatusOutput, LoopToolKind, OverdueWakeup,
    StaleToolUse, LOOP_STATUS_REDUCE_ACTION_ID,
};
pub use path_inventory::{
    PathInventoryEntry, PathInventoryInput, PathInventoryOutput, PathStyle,
    PATH_INVENTORY_ACTION_ID,
};
pub use payload::{ActionInput, ActionOutput};
pub use protocol::{
    native_action_host_tool_name, native_action_tool_name, ActionBoundary, ActionContract,
    ActionEffect, ActionError, ActionErrorCode, ActionExecution, ActionInputKind, ActionOutputKind,
    ActionReceipt, ActionRequest, ActionResult, NativeActionCatalogV1,
    RepositoryNativeActionCatalogV1, NATIVE_ACTION_CATALOG_V1_SCHEMA,
    NATIVE_ACTION_CATALOG_V1_SCHEMA_VERSION, NATIVE_ACTION_SERVER_NAME,
};
pub use ranking::{
    RankedCandidate, RankingCandidate, RankingScore, RankingWeight, WeightedRankingInput,
    WeightedRankingOutput, WEIGHTED_RANKING_ACTION_ID,
};
pub use review_convergence::{
    ConvergedFinding, FindingDisposition, ReviewConvergenceInput, ReviewConvergenceOutput,
    ReviewFinding, ReviewRecommendation, ReviewSeverity, VerifierDimension,
    REVIEW_BLOCKING_CONFIDENCE_MICROS, REVIEW_CONVERGENCE_REDUCE_ACTION_ID,
};
pub use rubric::{
    RubricCategory, RubricCategoryScore, RubricCheckResult, RubricEvaluationInput,
    RubricEvaluationOutput, RubricFailure, RUBRIC_EVALUATION_ACTION_ID,
};
pub use service_definition::{
    ServiceDefinition, ServiceDefinitionInput, ServiceDefinitionOutput, ServiceLaunch,
    SERVICE_DEFINITION_MAX_ARGUMENTS_V1, SERVICE_DEFINITION_MAX_ARGUMENT_BYTES_V1,
    SERVICE_DEFINITION_MAX_DEPENDENCIES_V1, SERVICE_DEFINITION_MAX_KIND_BYTES_V1,
    SERVICE_DEFINITION_MAX_NAME_BYTES_V1, SERVICE_DEFINITION_MAX_PORTS_V1,
    SERVICE_DEFINITION_MAX_PROGRAM_BYTES_V1, SERVICE_DEFINITION_MAX_SERVICES_V1,
    SERVICE_DEFINITION_NORMALIZE_ACTION_ID,
};
pub use skill_health::{
    SkillHealthInput, SkillHealthOutput, SkillHealthRow, SkillHealthSeries, SkillHealthTrend,
    SkillRunObservation, SkillWindowRate, SKILL_HEALTH_AGGREGATE_ACTION_ID,
};
