/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

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
