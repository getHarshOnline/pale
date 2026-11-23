/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

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
export type ReviewSeverity = "critical" | "high" | "medium" | "low";
export type ReviewRecommendation = "accept" | "block";
export type SkillHealthTrend = "improving" | "stable" | "declining" | "insufficient_data";

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
