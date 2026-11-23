// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package action_output_v1 contains the generated action-output.v1 data model.
package action_output_v1

import (
	"encoding/json"
	"fmt"
)

type ActionOutputComplianceTraceGradeV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload ComplianceTraceGradeOutput `json:"payload"`
}

type ActionOutputConfigInvariantScanV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload ConfigInvariantScanOutput `json:"payload"`
}

type ActionOutputCostAggregateV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload CostAggregateOutput `json:"payload"`
}

type ActionOutputEvidenceSelectionV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload EvidenceSelectionOutput `json:"payload"`
}

type ActionOutputLoopStatusReduceV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload LoopStatusOutput `json:"payload"`
}

type ActionOutputPathInventoryV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload PathInventoryOutput `json:"payload"`
}

type ActionOutputReviewConvergenceReduceV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload ReviewConvergenceOutput `json:"payload"`
}

type ActionOutputRubricEvaluationV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload RubricEvaluationOutput `json:"payload"`
}

type ActionOutputServiceDefinitionNormalizeV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload ServiceDefinitionOutput `json:"payload"`
}

type ActionOutputSkillHealthAggregateV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload SkillHealthOutput `json:"payload"`
}

// Kind corresponds to the JSON schema field "kind".

// Payload corresponds to the JSON schema field "payload".

type ActionOutputWeightedRankingV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload WeightedRankingOutput `json:"payload"`
}

type ComplianceCheckResult struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Passed corresponds to the JSON schema field "passed".
	Passed bool `json:"passed"`
}

type CompliancePromotion string

const CompliancePromotionEligible CompliancePromotion = "eligible"
const CompliancePromotionNotEligible CompliancePromotion = "not_eligible"

type ComplianceTraceGradeOutput struct {
	// ComplianceRateMicros corresponds to the JSON schema field
	// "compliance_rate_micros".
	ComplianceRateMicros int `json:"compliance_rate_micros"`

	// PassedChecks corresponds to the JSON schema field "passed_checks".
	PassedChecks int `json:"passed_checks"`

	// Promotion corresponds to the JSON schema field "promotion".
	Promotion CompliancePromotion `json:"promotion"`

	// ScaleMicros corresponds to the JSON schema field "scale_micros".
	ScaleMicros int `json:"scale_micros"`

	// Steps corresponds to the JSON schema field "steps".
	Steps []ComplianceCheckResult `json:"steps"`

	// TemporalRules corresponds to the JSON schema field "temporal_rules".
	TemporalRules []ComplianceCheckResult `json:"temporal_rules"`

	// TotalChecks corresponds to the JSON schema field "total_checks".
	TotalChecks int `json:"total_checks"`
}

type ConfigInvariantEvidence struct {
	// Line corresponds to the JSON schema field "line".
	Line int `json:"line"`

	// Occurrences corresponds to the JSON schema field "occurrences".
	Occurrences int `json:"occurrences"`

	// RecordID corresponds to the JSON schema field "record_id".
	RecordID string `json:"record_id"`
}

type ConfigInvariantResult struct {
	// Evidence corresponds to the JSON schema field "evidence".
	Evidence []ConfigInvariantEvidence `json:"evidence"`

	// EvidenceTruncated corresponds to the JSON schema field "evidence_truncated".
	EvidenceTruncated bool `json:"evidence_truncated"`

	// InvariantID corresponds to the JSON schema field "invariant_id".
	InvariantID string `json:"invariant_id"`

	// MatchCount corresponds to the JSON schema field "match_count".
	MatchCount int `json:"match_count"`

	// Passed corresponds to the JSON schema field "passed".
	Passed bool `json:"passed"`
}

type ConfigInvariantScanOutput struct {
	// Passed corresponds to the JSON schema field "passed".
	Passed bool `json:"passed"`

	// Results corresponds to the JSON schema field "results".
	Results []ConfigInvariantResult `json:"results"`
}

type ConvergedFinding struct {
	// CanonicalID corresponds to the JSON schema field "canonical_id".
	CanonicalID string `json:"canonical_id"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition FindingDisposition `json:"disposition"`

	// Evidence corresponds to the JSON schema field "evidence".
	Evidence string `json:"evidence"`

	// File corresponds to the JSON schema field "file".
	File string `json:"file"`

	// Line corresponds to the JSON schema field "line".
	Line ConvergedFindingLine `json:"line,omitempty,omitzero"`

	// Severity corresponds to the JSON schema field "severity".
	Severity ReviewSeverity `json:"severity"`

	// SourceIds corresponds to the JSON schema field "source_ids".
	SourceIds []string `json:"source_ids"`

	// Title corresponds to the JSON schema field "title".
	Title string `json:"title"`

	// VerifierConfidenceMicros corresponds to the JSON schema field
	// "verifier_confidence_micros".
	VerifierConfidenceMicros int `json:"verifier_confidence_micros"`
}

type ConvergedFindingLine *int

type CostAggregateOutput struct {
	// ByDayModel corresponds to the JSON schema field "by_day_model".
	ByDayModel []CostAggregateRow `json:"by_day_model"`

	// SelectedSessions corresponds to the JSON schema field "selected_sessions".
	SelectedSessions int `json:"selected_sessions"`

	// TodayCostMicroUsd corresponds to the JSON schema field "today_cost_micro_usd".
	TodayCostMicroUsd int `json:"today_cost_micro_usd"`

	// TotalCostMicroUsd corresponds to the JSON schema field "total_cost_micro_usd".
	TotalCostMicroUsd int `json:"total_cost_micro_usd"`
}

type CostAggregateRow struct {
	// CostMicroUsd corresponds to the JSON schema field "cost_micro_usd".
	CostMicroUsd int `json:"cost_micro_usd"`

	// Date corresponds to the JSON schema field "date".
	Date string `json:"date"`

	// Model corresponds to the JSON schema field "model".
	Model string `json:"model"`

	// Sessions corresponds to the JSON schema field "sessions".
	Sessions int `json:"sessions"`
}

type EvidenceMatch struct {
	// Evidence corresponds to the JSON schema field "evidence".
	Evidence []string `json:"evidence"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`
}

type EvidenceSelectionOutput struct {
	// Matches corresponds to the JSON schema field "matches".
	Matches []EvidenceMatch `json:"matches"`

	// UnmatchedObserved corresponds to the JSON schema field "unmatched_observed".
	UnmatchedObserved []string `json:"unmatched_observed"`
}

type FindingDisposition string

const FindingDispositionAdvisory FindingDisposition = "advisory"
const FindingDispositionBlocking FindingDisposition = "blocking"

type LoopRecommendation string

const LoopRecommendationContinue LoopRecommendation = "continue"
const LoopRecommendationIntervene LoopRecommendation = "intervene"
const LoopRecommendationStop LoopRecommendation = "stop"

type LoopStatusOutput struct {
	// FailedToolResultIds corresponds to the JSON schema field
	// "failed_tool_result_ids".
	FailedToolResultIds []string `json:"failed_tool_result_ids"`

	// OverdueWakeups corresponds to the JSON schema field "overdue_wakeups".
	OverdueWakeups []OverdueWakeup `json:"overdue_wakeups"`

	// ParseErrorIds corresponds to the JSON schema field "parse_error_ids".
	ParseErrorIds []string `json:"parse_error_ids"`

	// Recommendation corresponds to the JSON schema field "recommendation".
	Recommendation LoopRecommendation `json:"recommendation"`

	// StaleBash corresponds to the JSON schema field "stale_bash".
	StaleBash []StaleToolUse `json:"stale_bash"`
}

type OverdueWakeup struct {
	// OverdueMs corresponds to the JSON schema field "overdue_ms".
	OverdueMs int `json:"overdue_ms"`

	// WakeupID corresponds to the JSON schema field "wakeup_id".
	WakeupID string `json:"wakeup_id"`
}

type PathInventoryEntry struct {
	// NormalizedPath corresponds to the JSON schema field "normalized_path".
	NormalizedPath string `json:"normalized_path"`

	// Occurrences corresponds to the JSON schema field "occurrences".
	Occurrences int `json:"occurrences"`
}

type PathInventoryOutput struct {
	// Entries corresponds to the JSON schema field "entries".
	Entries []PathInventoryEntry `json:"entries"`

	// InputCount corresponds to the JSON schema field "input_count".
	InputCount int `json:"input_count"`

	// UniqueCount corresponds to the JSON schema field "unique_count".
	UniqueCount int `json:"unique_count"`
}

type RankedCandidate struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// NormalizedScoreMicros corresponds to the JSON schema field
	// "normalized_score_micros".
	NormalizedScoreMicros int `json:"normalized_score_micros"`

	// WeightedSum corresponds to the JSON schema field "weighted_sum".
	WeightedSum int `json:"weighted_sum"`
}

type ReviewConvergenceOutput struct {
	// FailedRequiredDimensions corresponds to the JSON schema field
	// "failed_required_dimensions".
	FailedRequiredDimensions []string `json:"failed_required_dimensions"`

	// Findings corresponds to the JSON schema field "findings".
	Findings []ConvergedFinding `json:"findings"`

	// Recommendation corresponds to the JSON schema field "recommendation".
	Recommendation ReviewRecommendation `json:"recommendation"`

	// RequiredDimensionsPassed corresponds to the JSON schema field
	// "required_dimensions_passed".
	RequiredDimensionsPassed bool `json:"required_dimensions_passed"`
}

type ReviewRecommendation string

const ReviewRecommendationAccept ReviewRecommendation = "accept"
const ReviewRecommendationBlock ReviewRecommendation = "block"

type ReviewSeverity string

const ReviewSeverityCritical ReviewSeverity = "critical"
const ReviewSeverityHigh ReviewSeverity = "high"
const ReviewSeverityLow ReviewSeverity = "low"
const ReviewSeverityMedium ReviewSeverity = "medium"

type RubricCategoryScore struct {
	// AvailablePoints corresponds to the JSON schema field "available_points".
	AvailablePoints int `json:"available_points"`

	// CategoryID corresponds to the JSON schema field "category_id".
	CategoryID string `json:"category_id"`

	// EarnedPoints corresponds to the JSON schema field "earned_points".
	EarnedPoints int `json:"earned_points"`

	// ScoreMicros corresponds to the JSON schema field "score_micros".
	ScoreMicros int `json:"score_micros"`
}

type RubricEvaluationOutput struct {
	// Categories corresponds to the JSON schema field "categories".
	Categories []RubricCategoryScore `json:"categories"`

	// OverallScoreMicros corresponds to the JSON schema field "overall_score_micros".
	OverallScoreMicros int `json:"overall_score_micros"`

	// ScaleMicros corresponds to the JSON schema field "scale_micros".
	ScaleMicros int `json:"scale_micros"`

	// TopFailures corresponds to the JSON schema field "top_failures".
	TopFailures []RubricFailure `json:"top_failures"`

	// TotalWeightMicros corresponds to the JSON schema field "total_weight_micros".
	TotalWeightMicros int `json:"total_weight_micros"`
}

type RubricFailure struct {
	// CategoryID corresponds to the JSON schema field "category_id".
	CategoryID string `json:"category_id"`

	// CheckID corresponds to the JSON schema field "check_id".
	CheckID string `json:"check_id"`

	// LostPoints corresponds to the JSON schema field "lost_points".
	LostPoints int `json:"lost_points"`

	// WeightedLoss corresponds to the JSON schema field "weighted_loss".
	WeightedLoss int `json:"weighted_loss"`
}

type ServiceDefinition struct {
	// DependsOn corresponds to the JSON schema field "depends_on".
	DependsOn []string `json:"depends_on,omitempty,omitzero"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Launch corresponds to the JSON schema field "launch".
	Launch ServiceLaunch `json:"launch"`

	// Name corresponds to the JSON schema field "name".
	Name string `json:"name"`

	// Ports corresponds to the JSON schema field "ports".
	Ports []int `json:"ports,omitempty,omitzero"`

	// Precedence corresponds to the JSON schema field "precedence".
	Precedence int `json:"precedence"`
}

type ServiceDefinitionOutput struct {
	// Services corresponds to the JSON schema field "services".
	Services []ServiceDefinition `json:"services"`
}

type ServiceLaunch struct {
	// Args corresponds to the JSON schema field "args".
	Args []string `json:"args,omitempty,omitzero"`

	// Program corresponds to the JSON schema field "program".
	Program string `json:"program"`
}

type SkillHealthOutput struct {
	// LastRunAtMs corresponds to the JSON schema field "last_run_at_ms".
	LastRunAtMs *int `json:"last_run_at_ms,omitempty,omitzero"`

	// Long corresponds to the JSON schema field "long".
	Long SkillWindowRate `json:"long"`

	// PendingAmendments corresponds to the JSON schema field "pending_amendments".
	PendingAmendments int `json:"pending_amendments"`

	// Recent corresponds to the JSON schema field "recent".
	Recent SkillWindowRate `json:"recent"`

	// ScaleMicros corresponds to the JSON schema field "scale_micros".
	ScaleMicros int `json:"scale_micros"`

	// Skills corresponds to the JSON schema field "skills".
	Skills []SkillHealthRow `json:"skills"`
}

type SkillHealthRow struct {
	// LastRunAtMs corresponds to the JSON schema field "last_run_at_ms".
	LastRunAtMs *int `json:"last_run_at_ms,omitempty,omitzero"`

	// Long corresponds to the JSON schema field "long".
	Long SkillWindowRate `json:"long"`

	// PendingAmendments corresponds to the JSON schema field "pending_amendments".
	PendingAmendments int `json:"pending_amendments"`

	// Recent corresponds to the JSON schema field "recent".
	Recent SkillWindowRate `json:"recent"`

	// SkillID corresponds to the JSON schema field "skill_id".
	SkillID string `json:"skill_id"`

	// Trend corresponds to the JSON schema field "trend".
	Trend SkillHealthTrend `json:"trend"`
}

type SkillHealthTrend string

const SkillHealthTrendDeclining SkillHealthTrend = "declining"
const SkillHealthTrendImproving SkillHealthTrend = "improving"
const SkillHealthTrendInsufficientData SkillHealthTrend = "insufficient_data"
const SkillHealthTrendStable SkillHealthTrend = "stable"

type SkillWindowRate struct {
	// Runs corresponds to the JSON schema field "runs".
	Runs int `json:"runs"`

	// SuccessRateMicros corresponds to the JSON schema field "success_rate_micros".
	SuccessRateMicros SkillWindowRateSuccessRateMicros `json:"success_rate_micros,omitempty,omitzero"`

	// Successes corresponds to the JSON schema field "successes".
	Successes int `json:"successes"`
}

type SkillWindowRateSuccessRateMicros *int

type StaleToolUse struct {
	// AgeMs corresponds to the JSON schema field "age_ms".
	AgeMs int `json:"age_ms"`

	// ToolUseID corresponds to the JSON schema field "tool_use_id".
	ToolUseID string `json:"tool_use_id"`
}

type WeightedRankingOutput struct {
	// Ranking corresponds to the JSON schema field "ranking".
	Ranking []RankedCandidate `json:"ranking"`

	// ScaleMicros corresponds to the JSON schema field "scale_micros".
	ScaleMicros int `json:"scale_micros"`

	// TotalWeightMicros corresponds to the JSON schema field "total_weight_micros".
	TotalWeightMicros int `json:"total_weight_micros"`
}

type ActionOutputV1 struct {
	ComplianceTraceGrade       *ActionOutputComplianceTraceGradeV1       `json:"-"`
	ConfigInvariantScan        *ActionOutputConfigInvariantScanV1        `json:"-"`
	CostAggregate              *ActionOutputCostAggregateV1              `json:"-"`
	LoopStatusReduce           *ActionOutputLoopStatusReduceV1           `json:"-"`
	ReviewConvergenceReduce    *ActionOutputReviewConvergenceReduceV1    `json:"-"`
	RubricEvaluation           *ActionOutputRubricEvaluationV1           `json:"-"`
	ServiceDefinitionNormalize *ActionOutputServiceDefinitionNormalizeV1 `json:"-"`
	SkillHealthAggregate       *ActionOutputSkillHealthAggregateV1       `json:"-"`
	WeightedRanking            *ActionOutputWeightedRankingV1            `json:"-"`
	PathInventory              *ActionOutputPathInventoryV1              `json:"-"`
	EvidenceSelection          *ActionOutputEvidenceSelectionV1          `json:"-"`
}

func (value ActionOutputV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.ComplianceTraceGrade != nil {
		if value.ComplianceTraceGrade.Kind != "compliance_trace_grade" {
			return nil, fmt.Errorf("ActionOutputV1: selected variant has discriminator %q", value.ComplianceTraceGrade.Kind)
		}
		candidate, err := json.Marshal(value.ComplianceTraceGrade)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ConfigInvariantScan != nil {
		if value.ConfigInvariantScan.Kind != "config_invariant_scan" {
			return nil, fmt.Errorf("ActionOutputV1: selected variant has discriminator %q", value.ConfigInvariantScan.Kind)
		}
		candidate, err := json.Marshal(value.ConfigInvariantScan)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.CostAggregate != nil {
		if value.CostAggregate.Kind != "cost_aggregate" {
			return nil, fmt.Errorf("ActionOutputV1: selected variant has discriminator %q", value.CostAggregate.Kind)
		}
		candidate, err := json.Marshal(value.CostAggregate)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.LoopStatusReduce != nil {
		if value.LoopStatusReduce.Kind != "loop_status_reduce" {
			return nil, fmt.Errorf("ActionOutputV1: selected variant has discriminator %q", value.LoopStatusReduce.Kind)
		}
		candidate, err := json.Marshal(value.LoopStatusReduce)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ReviewConvergenceReduce != nil {
		if value.ReviewConvergenceReduce.Kind != "review_convergence_reduce" {
			return nil, fmt.Errorf("ActionOutputV1: selected variant has discriminator %q", value.ReviewConvergenceReduce.Kind)
		}
		candidate, err := json.Marshal(value.ReviewConvergenceReduce)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.RubricEvaluation != nil {
		if value.RubricEvaluation.Kind != "rubric_evaluation" {
			return nil, fmt.Errorf("ActionOutputV1: selected variant has discriminator %q", value.RubricEvaluation.Kind)
		}
		candidate, err := json.Marshal(value.RubricEvaluation)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ServiceDefinitionNormalize != nil {
		if value.ServiceDefinitionNormalize.Kind != "service_definition_normalize" {
			return nil, fmt.Errorf("ActionOutputV1: selected variant has discriminator %q", value.ServiceDefinitionNormalize.Kind)
		}
		candidate, err := json.Marshal(value.ServiceDefinitionNormalize)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.SkillHealthAggregate != nil {
		if value.SkillHealthAggregate.Kind != "skill_health_aggregate" {
			return nil, fmt.Errorf("ActionOutputV1: selected variant has discriminator %q", value.SkillHealthAggregate.Kind)
		}
		candidate, err := json.Marshal(value.SkillHealthAggregate)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.WeightedRanking != nil {
		if value.WeightedRanking.Kind != "weighted_ranking" {
			return nil, fmt.Errorf("ActionOutputV1: selected variant has discriminator %q", value.WeightedRanking.Kind)
		}
		candidate, err := json.Marshal(value.WeightedRanking)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.PathInventory != nil {
		if value.PathInventory.Kind != "path_inventory" {
			return nil, fmt.Errorf("ActionOutputV1: selected variant has discriminator %q", value.PathInventory.Kind)
		}
		candidate, err := json.Marshal(value.PathInventory)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.EvidenceSelection != nil {
		if value.EvidenceSelection.Kind != "evidence_selection" {
			return nil, fmt.Errorf("ActionOutputV1: selected variant has discriminator %q", value.EvidenceSelection.Kind)
		}
		candidate, err := json.Marshal(value.EvidenceSelection)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ActionOutputV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ActionOutputV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ActionOutputV1{}
	switch discriminator.Kind {
	case "compliance_trace_grade":
		var variant ActionOutputComplianceTraceGradeV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ComplianceTraceGrade = &variant
	case "config_invariant_scan":
		var variant ActionOutputConfigInvariantScanV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ConfigInvariantScan = &variant
	case "cost_aggregate":
		var variant ActionOutputCostAggregateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.CostAggregate = &variant
	case "loop_status_reduce":
		var variant ActionOutputLoopStatusReduceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.LoopStatusReduce = &variant
	case "review_convergence_reduce":
		var variant ActionOutputReviewConvergenceReduceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ReviewConvergenceReduce = &variant
	case "rubric_evaluation":
		var variant ActionOutputRubricEvaluationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.RubricEvaluation = &variant
	case "service_definition_normalize":
		var variant ActionOutputServiceDefinitionNormalizeV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ServiceDefinitionNormalize = &variant
	case "skill_health_aggregate":
		var variant ActionOutputSkillHealthAggregateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.SkillHealthAggregate = &variant
	case "weighted_ranking":
		var variant ActionOutputWeightedRankingV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.WeightedRanking = &variant
	case "path_inventory":
		var variant ActionOutputPathInventoryV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.PathInventory = &variant
	case "evidence_selection":
		var variant ActionOutputEvidenceSelectionV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.EvidenceSelection = &variant
	default:
		return fmt.Errorf("ActionOutputV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
