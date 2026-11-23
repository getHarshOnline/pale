/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ActionEffect = "compute";
export type ActionInputV1 =
  | {
      kind: "compliance_trace_grade";
      payload: ComplianceTraceGradeInput;
    }
  | {
      kind: "config_invariant_scan";
      payload: ConfigInvariantScanInput;
    }
  | {
      kind: "cost_aggregate";
      payload: CostAggregateInput;
    }
  | {
      kind: "loop_status_reduce";
      payload: LoopStatusInput;
    }
  | {
      kind: "review_convergence_reduce";
      payload: ReviewConvergenceInput;
    }
  | {
      kind: "rubric_evaluation";
      payload: RubricEvaluationInput;
    }
  | {
      kind: "service_definition_normalize";
      payload: ServiceDefinitionInput;
    }
  | {
      kind: "skill_health_aggregate";
      payload: SkillHealthInput;
    }
  | {
      kind: "weighted_ranking";
      payload: WeightedRankingInput;
    }
  | {
      kind: "path_inventory";
      payload: PathInventoryInput;
    }
  | {
      kind: "evidence_selection";
      payload: EvidenceSelectionInput;
    };
export type ComplianceTemporalRule =
  | {
      first_step_id: string;
      id: string;
      kind: "before";
      second_step_id: string;
    }
  | {
      first_step_id: string;
      id: string;
      kind: "after";
      second_step_id: string;
    }
  | {
      id: string;
      kind: "ordered";
      /**
       * @minItems 2
       * @maxItems 256
       */
      step_ids: [string, string, ...string[]];
    };
export type ConfigInvariantExpectation = "required_literal" | "forbidden_literal";
export type LoopEvent =
  | {
      id: string;
      kind: "tool_use";
      started_at_ms: number;
      tool_kind: LoopToolKind;
    }
  | {
      completed_at_ms: number;
      id: string;
      kind: "tool_result";
      succeeded: boolean;
      tool_use_id: string;
    }
  | {
      due_at_ms: number;
      id: string;
      kind: "wakeup_scheduled";
    }
  | {
      id: string;
      kind: "wakeup_observed";
      observed_at_ms: number;
      wakeup_id: string;
    }
  | {
      id: string;
      kind: "parse_error";
      observed_at_ms: number;
    };
export type LoopToolKind = "bash" | "other";
export type ReviewSeverity = "critical" | "high" | "medium" | "low";
export type PathStyle = "posix_relative" | "posix_absolute";
export type EvidenceSelectionMode = "all_matches" | "first_match";
export type ActionOutputV1 =
  | {
      kind: "compliance_trace_grade";
      payload: ComplianceTraceGradeOutput;
    }
  | {
      kind: "config_invariant_scan";
      payload: ConfigInvariantScanOutput;
    }
  | {
      kind: "cost_aggregate";
      payload: CostAggregateOutput;
    }
  | {
      kind: "loop_status_reduce";
      payload: LoopStatusOutput;
    }
  | {
      kind: "review_convergence_reduce";
      payload: ReviewConvergenceOutput;
    }
  | {
      kind: "rubric_evaluation";
      payload: RubricEvaluationOutput;
    }
  | {
      kind: "service_definition_normalize";
      payload: ServiceDefinitionOutput;
    }
  | {
      kind: "skill_health_aggregate";
      payload: SkillHealthOutput;
    }
  | {
      kind: "weighted_ranking";
      payload: WeightedRankingOutput;
    }
  | {
      kind: "path_inventory";
      payload: PathInventoryOutput;
    }
  | {
      kind: "evidence_selection";
      payload: EvidenceSelectionOutput;
    };
export type CompliancePromotion = "eligible" | "not_eligible";
export type LoopRecommendation = "continue" | "intervene" | "stop";
export type FindingDisposition = "blocking" | "advisory";
export type ReviewRecommendation = "accept" | "block";
export type SkillHealthTrend = "improving" | "stable" | "declining" | "insufficient_data";

export interface ActionExecutionV1 {
  receipt: ActionReceipt;
  request: ActionRequest;
  result: ActionResult;
  schema: "pale/action-execution.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
}
export interface ActionReceipt {
  action_id: string;
  action_version: 1;
  contract_digest: Sha256DigestV1;
  receipt_digest: Sha256DigestV1;
  request_digest: Sha256DigestV1;
  result_digest: Sha256DigestV1;
  schema: "pale/action-receipt.v1";
  schema_version: 1;
}
export interface ActionRequest {
  action_id: string;
  action_version: 1;
  contract_digest: Sha256DigestV1;
  effect: ActionEffect;
  input: ActionInputV1;
  request_digest: Sha256DigestV1;
  schema: "pale/action-request.v1";
  schema_version: 1;
}
export interface ComplianceTraceGradeInput {
  /**
   * @maxItems 10000
   */
  events: ClassifiedTraceEvent[];
  promotion_threshold_micros: number;
  /**
   * @minItems 1
   * @maxItems 256
   */
  required_steps: [ComplianceStep, ...ComplianceStep[]];
  /**
   * @maxItems 512
   */
  temporal_rules: ComplianceTemporalRule[];
}
export interface ClassifiedTraceEvent {
  class: string;
  id: string;
  sequence: number;
}
export interface ComplianceStep {
  event_class: string;
  id: string;
  minimum_occurrences: number;
}
export interface ConfigInvariantScanInput {
  /**
   * @minItems 1
   * @maxItems 256
   */
  invariants: [ConfigInvariant, ...ConfigInvariant[]];
  /**
   * @minItems 1
   * @maxItems 128
   */
  records: [ConfigTextRecord, ...ConfigTextRecord[]];
}
export interface ConfigInvariant {
  expectation: ConfigInvariantExpectation;
  id: string;
  literal: string;
  /**
   * @maxItems 128
   */
  record_ids?: string[];
}
export interface ConfigTextRecord {
  id: string;
  text: string;
}
export interface CostAggregateInput {
  /**
   * @maxItems 100000
   */
  snapshots: CostSnapshot[];
  today: string;
}
export interface CostSnapshot {
  cost_micro_usd: number;
  date: string;
  model: string;
  session_id: string;
  snapshot_index: number;
}
export interface LoopStatusInput {
  bash_stale_after_ms: number;
  /**
   * @maxItems 50000
   */
  events: LoopEvent[];
  now_ms: number;
  wakeup_grace_ms: number;
}
export interface ReviewConvergenceInput {
  /**
   * @maxItems 10000
   */
  findings: ReviewFinding[];
  /**
   * @minItems 1
   * @maxItems 128
   */
  verifier_dimensions: [VerifierDimension, ...VerifierDimension[]];
}
export interface ReviewFinding {
  evidence?: string;
  file: string;
  id: string;
  line?: number | null;
  severity: ReviewSeverity;
  title: string;
  verifier_confidence_micros: number;
}
export interface VerifierDimension {
  id: string;
  passed: boolean;
  required: boolean;
}
export interface RubricEvaluationInput {
  /**
   * @minItems 1
   * @maxItems 128
   */
  categories: [RubricCategory, ...RubricCategory[]];
  /**
   * @minItems 1
   * @maxItems 4096
   */
  checks: [RubricCheckResult, ...RubricCheckResult[]];
  top_failure_limit: number;
}
export interface RubricCategory {
  id: string;
  weight_micros: number;
}
export interface RubricCheckResult {
  available_points: number;
  category_id: string;
  earned_points: number;
  id: string;
}
export interface ServiceDefinitionInput {
  /**
   * @minItems 1
   * @maxItems 128
   */
  services: [ServiceDefinition, ...ServiceDefinition[]];
}
export interface ServiceDefinition {
  /**
   * @maxItems 128
   */
  depends_on?: string[];
  kind: string;
  launch: ServiceLaunch;
  name: string;
  /**
   * @maxItems 64
   */
  ports?: number[];
  precedence: number;
}
export interface ServiceLaunch {
  /**
   * @maxItems 128
   */
  args?: string[];
  program: string;
}
export interface SkillHealthInput {
  evaluation_time_ms: number;
  long_window_start_ms: number;
  recent_window_start_ms: number;
  /**
   * @minItems 1
   * @maxItems 1024
   */
  skills: [SkillHealthSeries, ...SkillHealthSeries[]];
  trend_threshold_micros: number;
}
export interface SkillHealthSeries {
  /**
   * @maxItems 100000
   */
  observations: SkillRunObservation[];
  pending_amendments: number;
  skill_id: string;
}
export interface SkillRunObservation {
  id: string;
  occurred_at_ms: number;
  succeeded: boolean;
}
export interface WeightedRankingInput {
  /**
   * @minItems 1
   * @maxItems 10000
   */
  candidates: [RankingCandidate, ...RankingCandidate[]];
  /**
   * @minItems 1
   * @maxItems 1024
   */
  weights: [RankingWeight, ...RankingWeight[]];
}
export interface RankingCandidate {
  id: string;
  /**
   * @maxItems 1024
   */
  scores: RankingScore[];
}
export interface RankingScore {
  value_micros: number;
  weight_id: string;
}
export interface RankingWeight {
  id: string;
  weight_micros: number;
}
export interface PathInventoryInput {
  /**
   * @maxItems 100000
   */
  paths: string[];
  style: PathStyle;
}
export interface EvidenceSelectionInput {
  mode: EvidenceSelectionMode;
  /**
   * @maxItems 10000
   */
  observed: string[];
  /**
   * @maxItems 1024
   */
  rules: EvidenceRule[];
}
export interface EvidenceRule {
  /**
   * @maxItems 1024
   */
  exclude?: string[];
  id: string;
  /**
   * @maxItems 1024
   */
  require_all?: string[];
  /**
   * @maxItems 1024
   */
  require_any?: string[];
}
export interface ActionResult {
  action_id: string;
  action_version: 1;
  contract_digest: Sha256DigestV1;
  output: ActionOutputV1;
  request_digest: Sha256DigestV1;
  result_digest: Sha256DigestV1;
  schema: "pale/action-result.v1";
  schema_version: 1;
}
export interface ComplianceTraceGradeOutput {
  compliance_rate_micros: number;
  passed_checks: number;
  promotion: CompliancePromotion;
  scale_micros: number;
  steps: ComplianceCheckResult[];
  temporal_rules: ComplianceCheckResult[];
  total_checks: number;
}
export interface ComplianceCheckResult {
  id: string;
  passed: boolean;
}
export interface ConfigInvariantScanOutput {
  passed: boolean;
  results: ConfigInvariantResult[];
}
export interface ConfigInvariantResult {
  evidence: ConfigInvariantEvidence[];
  evidence_truncated: boolean;
  invariant_id: string;
  match_count: number;
  passed: boolean;
}
export interface ConfigInvariantEvidence {
  line: number;
  occurrences: number;
  record_id: string;
}
export interface CostAggregateOutput {
  by_day_model: CostAggregateRow[];
  selected_sessions: number;
  today_cost_micro_usd: number;
  total_cost_micro_usd: number;
}
export interface CostAggregateRow {
  cost_micro_usd: number;
  date: string;
  model: string;
  sessions: number;
}
export interface LoopStatusOutput {
  failed_tool_result_ids: string[];
  overdue_wakeups: OverdueWakeup[];
  parse_error_ids: string[];
  recommendation: LoopRecommendation;
  stale_bash: StaleToolUse[];
}
export interface OverdueWakeup {
  overdue_ms: number;
  wakeup_id: string;
}
export interface StaleToolUse {
  age_ms: number;
  tool_use_id: string;
}
export interface ReviewConvergenceOutput {
  failed_required_dimensions: string[];
  findings: ConvergedFinding[];
  recommendation: ReviewRecommendation;
  required_dimensions_passed: boolean;
}
export interface ConvergedFinding {
  canonical_id: string;
  disposition: FindingDisposition;
  evidence: string;
  file: string;
  line?: number | null;
  severity: ReviewSeverity;
  source_ids: string[];
  title: string;
  verifier_confidence_micros: number;
}
export interface RubricEvaluationOutput {
  categories: RubricCategoryScore[];
  overall_score_micros: number;
  scale_micros: number;
  top_failures: RubricFailure[];
  total_weight_micros: number;
}
export interface RubricCategoryScore {
  available_points: number;
  category_id: string;
  earned_points: number;
  score_micros: number;
}
export interface RubricFailure {
  category_id: string;
  check_id: string;
  lost_points: number;
  weighted_loss: number;
}
export interface ServiceDefinitionOutput {
  services: ServiceDefinition[];
}
export interface SkillHealthOutput {
  last_run_at_ms?: number;
  long: SkillWindowRate;
  pending_amendments: number;
  recent: SkillWindowRate;
  scale_micros: number;
  skills: SkillHealthRow[];
}
export interface SkillWindowRate {
  runs: number;
  success_rate_micros?: number | null;
  successes: number;
}
export interface SkillHealthRow {
  last_run_at_ms?: number;
  long: SkillWindowRate;
  pending_amendments: number;
  recent: SkillWindowRate;
  skill_id: string;
  trend: SkillHealthTrend;
}
export interface WeightedRankingOutput {
  ranking: RankedCandidate[];
  scale_micros: number;
  total_weight_micros: number;
}
export interface RankedCandidate {
  id: string;
  normalized_score_micros: number;
  weighted_sum: number;
}
export interface PathInventoryOutput {
  entries: PathInventoryEntry[];
  input_count: number;
  unique_count: number;
}
export interface PathInventoryEntry {
  normalized_path: string;
  occurrences: number;
}
export interface EvidenceSelectionOutput {
  matches: EvidenceMatch[];
  unmatched_observed: string[];
}
export interface EvidenceMatch {
  evidence: string[];
  id: string;
}
