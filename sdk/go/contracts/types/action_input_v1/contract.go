// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package action_input_v1 contains the generated action-input.v1 data model.
package action_input_v1

import (
	"encoding/json"
	"fmt"
)

type ActionInputComplianceTraceGradeV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload ComplianceTraceGradeInput `json:"payload"`
}

type ActionInputConfigInvariantScanV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload ConfigInvariantScanInput `json:"payload"`
}

type ActionInputCostAggregateV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload CostAggregateInput `json:"payload"`
}

type ActionInputEvidenceSelectionV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload EvidenceSelectionInput `json:"payload"`
}

type ActionInputLoopStatusReduceV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload LoopStatusInput `json:"payload"`
}

type ActionInputPathInventoryV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload PathInventoryInput `json:"payload"`
}

type ActionInputReviewConvergenceReduceV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload ReviewConvergenceInput `json:"payload"`
}

type ActionInputRubricEvaluationV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload RubricEvaluationInput `json:"payload"`
}

type ActionInputServiceDefinitionNormalizeV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload ServiceDefinitionInput `json:"payload"`
}

type ActionInputSkillHealthAggregateV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload SkillHealthInput `json:"payload"`
}

// Kind corresponds to the JSON schema field "kind".

// Payload corresponds to the JSON schema field "payload".

type ActionInputWeightedRankingV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Payload corresponds to the JSON schema field "payload".
	Payload WeightedRankingInput `json:"payload"`
}

type ClassifiedTraceEvent struct {
	// Class corresponds to the JSON schema field "class".
	Class string `json:"class"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Sequence corresponds to the JSON schema field "sequence".
	Sequence int `json:"sequence"`
}

type ComplianceStep struct {
	// EventClass corresponds to the JSON schema field "event_class".
	EventClass string `json:"event_class"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// MinimumOccurrences corresponds to the JSON schema field "minimum_occurrences".
	MinimumOccurrences int `json:"minimum_occurrences"`
}

type ComplianceTemporalRuleAfter struct {
	// FirstStepID corresponds to the JSON schema field "first_step_id".
	FirstStepID string `json:"first_step_id"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// SecondStepID corresponds to the JSON schema field "second_step_id".
	SecondStepID string `json:"second_step_id"`
}

type ComplianceTemporalRuleBefore struct {
	// FirstStepID corresponds to the JSON schema field "first_step_id".
	FirstStepID string `json:"first_step_id"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// SecondStepID corresponds to the JSON schema field "second_step_id".
	SecondStepID string `json:"second_step_id"`
}

type ComplianceTemporalRuleOrdered struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// StepIds corresponds to the JSON schema field "step_ids".
	StepIds []string `json:"step_ids"`
}

type ComplianceTraceGradeInput struct {
	// Events corresponds to the JSON schema field "events".
	Events []ClassifiedTraceEvent `json:"events"`

	// PromotionThresholdMicros corresponds to the JSON schema field
	// "promotion_threshold_micros".
	PromotionThresholdMicros int `json:"promotion_threshold_micros"`

	// RequiredSteps corresponds to the JSON schema field "required_steps".
	RequiredSteps []ComplianceStep `json:"required_steps"`

	// TemporalRules corresponds to the JSON schema field "temporal_rules".
	TemporalRules []ComplianceTemporalRule `json:"temporal_rules"`
}

type ConfigInvariant struct {
	// Expectation corresponds to the JSON schema field "expectation".
	Expectation ConfigInvariantExpectation `json:"expectation"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Literal corresponds to the JSON schema field "literal".
	Literal string `json:"literal"`

	// RecordIds corresponds to the JSON schema field "record_ids".
	RecordIds []string `json:"record_ids,omitempty,omitzero"`
}

type ConfigInvariantExpectation string

const ConfigInvariantExpectationForbiddenLiteral ConfigInvariantExpectation = "forbidden_literal"
const ConfigInvariantExpectationRequiredLiteral ConfigInvariantExpectation = "required_literal"

type ConfigInvariantScanInput struct {
	// Invariants corresponds to the JSON schema field "invariants".
	Invariants []ConfigInvariant `json:"invariants"`

	// Records corresponds to the JSON schema field "records".
	Records []ConfigTextRecord `json:"records"`
}

type ConfigTextRecord struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Text corresponds to the JSON schema field "text".
	Text string `json:"text"`
}

type CostAggregateInput struct {
	// Snapshots corresponds to the JSON schema field "snapshots".
	Snapshots []CostSnapshot `json:"snapshots"`

	// Today corresponds to the JSON schema field "today".
	Today string `json:"today"`
}

type CostSnapshot struct {
	// CostMicroUsd corresponds to the JSON schema field "cost_micro_usd".
	CostMicroUsd int `json:"cost_micro_usd"`

	// Date corresponds to the JSON schema field "date".
	Date string `json:"date"`

	// Model corresponds to the JSON schema field "model".
	Model string `json:"model"`

	// SessionID corresponds to the JSON schema field "session_id".
	SessionID string `json:"session_id"`

	// SnapshotIndex corresponds to the JSON schema field "snapshot_index".
	SnapshotIndex int `json:"snapshot_index"`
}

type EvidenceRule struct {
	// Exclude corresponds to the JSON schema field "exclude".
	Exclude []string `json:"exclude,omitempty,omitzero"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// RequireAll corresponds to the JSON schema field "require_all".
	RequireAll []string `json:"require_all,omitempty,omitzero"`

	// RequireAny corresponds to the JSON schema field "require_any".
	RequireAny []string `json:"require_any,omitempty,omitzero"`
}

type EvidenceSelectionInput struct {
	// Mode corresponds to the JSON schema field "mode".
	Mode EvidenceSelectionMode `json:"mode"`

	// Observed corresponds to the JSON schema field "observed".
	Observed []string `json:"observed"`

	// Rules corresponds to the JSON schema field "rules".
	Rules []EvidenceRule `json:"rules"`
}

type EvidenceSelectionMode string

const EvidenceSelectionModeAllMatches EvidenceSelectionMode = "all_matches"
const EvidenceSelectionModeFirstMatch EvidenceSelectionMode = "first_match"

type LoopEventParseError struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ObservedAtMs corresponds to the JSON schema field "observed_at_ms".
	ObservedAtMs int `json:"observed_at_ms"`
}

type LoopEventToolResult struct {
	// CompletedAtMs corresponds to the JSON schema field "completed_at_ms".
	CompletedAtMs int `json:"completed_at_ms"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Succeeded corresponds to the JSON schema field "succeeded".
	Succeeded bool `json:"succeeded"`

	// ToolUseID corresponds to the JSON schema field "tool_use_id".
	ToolUseID string `json:"tool_use_id"`
}

type LoopEventToolUse struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// StartedAtMs corresponds to the JSON schema field "started_at_ms".
	StartedAtMs int `json:"started_at_ms"`

	// ToolKind corresponds to the JSON schema field "tool_kind".
	ToolKind LoopToolKind `json:"tool_kind"`
}

type LoopEventWakeupObserved struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ObservedAtMs corresponds to the JSON schema field "observed_at_ms".
	ObservedAtMs int `json:"observed_at_ms"`

	// WakeupID corresponds to the JSON schema field "wakeup_id".
	WakeupID string `json:"wakeup_id"`
}

type LoopEventWakeupScheduled struct {
	// DueAtMs corresponds to the JSON schema field "due_at_ms".
	DueAtMs int `json:"due_at_ms"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type LoopStatusInput struct {
	// BashStaleAfterMs corresponds to the JSON schema field "bash_stale_after_ms".
	BashStaleAfterMs int `json:"bash_stale_after_ms"`

	// Events corresponds to the JSON schema field "events".
	Events []LoopEvent `json:"events"`

	// NowMs corresponds to the JSON schema field "now_ms".
	NowMs int `json:"now_ms"`

	// WakeupGraceMs corresponds to the JSON schema field "wakeup_grace_ms".
	WakeupGraceMs int `json:"wakeup_grace_ms"`
}

type LoopToolKind string

const LoopToolKindBash LoopToolKind = "bash"
const LoopToolKindOther LoopToolKind = "other"

type PathInventoryInput struct {
	// Paths corresponds to the JSON schema field "paths".
	Paths []string `json:"paths"`

	// Style corresponds to the JSON schema field "style".
	Style PathStyle `json:"style"`
}

type PathStyle string

const PathStylePosixAbsolute PathStyle = "posix_absolute"
const PathStylePosixRelative PathStyle = "posix_relative"

type RankingCandidate struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Scores corresponds to the JSON schema field "scores".
	Scores []RankingScore `json:"scores"`
}

type RankingScore struct {
	// ValueMicros corresponds to the JSON schema field "value_micros".
	ValueMicros int `json:"value_micros"`

	// WeightID corresponds to the JSON schema field "weight_id".
	WeightID string `json:"weight_id"`
}

type RankingWeight struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// WeightMicros corresponds to the JSON schema field "weight_micros".
	WeightMicros int `json:"weight_micros"`
}

type ReviewConvergenceInput struct {
	// Findings corresponds to the JSON schema field "findings".
	Findings []ReviewFinding `json:"findings"`

	// VerifierDimensions corresponds to the JSON schema field "verifier_dimensions".
	VerifierDimensions []VerifierDimension `json:"verifier_dimensions"`
}

type ReviewFinding struct {
	// Evidence corresponds to the JSON schema field "evidence".
	Evidence string `json:"evidence,omitempty,omitzero"`

	// File corresponds to the JSON schema field "file".
	File string `json:"file"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Line corresponds to the JSON schema field "line".
	Line ReviewFindingLine `json:"line,omitempty,omitzero"`

	// Severity corresponds to the JSON schema field "severity".
	Severity ReviewSeverity `json:"severity"`

	// Title corresponds to the JSON schema field "title".
	Title string `json:"title"`

	// VerifierConfidenceMicros corresponds to the JSON schema field
	// "verifier_confidence_micros".
	VerifierConfidenceMicros int `json:"verifier_confidence_micros"`
}

type ReviewFindingLine *int

type ReviewSeverity string

const ReviewSeverityCritical ReviewSeverity = "critical"
const ReviewSeverityHigh ReviewSeverity = "high"
const ReviewSeverityLow ReviewSeverity = "low"
const ReviewSeverityMedium ReviewSeverity = "medium"

type RubricCategory struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// WeightMicros corresponds to the JSON schema field "weight_micros".
	WeightMicros int `json:"weight_micros"`
}

type RubricCheckResult struct {
	// AvailablePoints corresponds to the JSON schema field "available_points".
	AvailablePoints int `json:"available_points"`

	// CategoryID corresponds to the JSON schema field "category_id".
	CategoryID string `json:"category_id"`

	// EarnedPoints corresponds to the JSON schema field "earned_points".
	EarnedPoints int `json:"earned_points"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`
}

type RubricEvaluationInput struct {
	// Categories corresponds to the JSON schema field "categories".
	Categories []RubricCategory `json:"categories"`

	// Checks corresponds to the JSON schema field "checks".
	Checks []RubricCheckResult `json:"checks"`

	// TopFailureLimit corresponds to the JSON schema field "top_failure_limit".
	TopFailureLimit int `json:"top_failure_limit"`
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

type ServiceDefinitionInput struct {
	// Services corresponds to the JSON schema field "services".
	Services []ServiceDefinition `json:"services"`
}

type ServiceLaunch struct {
	// Args corresponds to the JSON schema field "args".
	Args []string `json:"args,omitempty,omitzero"`

	// Program corresponds to the JSON schema field "program".
	Program string `json:"program"`
}

type SkillHealthInput struct {
	// EvaluationTimeMs corresponds to the JSON schema field "evaluation_time_ms".
	EvaluationTimeMs int `json:"evaluation_time_ms"`

	// LongWindowStartMs corresponds to the JSON schema field "long_window_start_ms".
	LongWindowStartMs int `json:"long_window_start_ms"`

	// RecentWindowStartMs corresponds to the JSON schema field
	// "recent_window_start_ms".
	RecentWindowStartMs int `json:"recent_window_start_ms"`

	// Skills corresponds to the JSON schema field "skills".
	Skills []SkillHealthSeries `json:"skills"`

	// TrendThresholdMicros corresponds to the JSON schema field
	// "trend_threshold_micros".
	TrendThresholdMicros int `json:"trend_threshold_micros"`
}

type SkillHealthSeries struct {
	// Observations corresponds to the JSON schema field "observations".
	Observations []SkillRunObservation `json:"observations"`

	// PendingAmendments corresponds to the JSON schema field "pending_amendments".
	PendingAmendments int `json:"pending_amendments"`

	// SkillID corresponds to the JSON schema field "skill_id".
	SkillID string `json:"skill_id"`
}

type SkillRunObservation struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// OccurredAtMs corresponds to the JSON schema field "occurred_at_ms".
	OccurredAtMs int `json:"occurred_at_ms"`

	// Succeeded corresponds to the JSON schema field "succeeded".
	Succeeded bool `json:"succeeded"`
}

type VerifierDimension struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Passed corresponds to the JSON schema field "passed".
	Passed bool `json:"passed"`

	// Required corresponds to the JSON schema field "required".
	Required bool `json:"required"`
}

type WeightedRankingInput struct {
	// Candidates corresponds to the JSON schema field "candidates".
	Candidates []RankingCandidate `json:"candidates"`

	// Weights corresponds to the JSON schema field "weights".
	Weights []RankingWeight `json:"weights"`
}

type ComplianceTemporalRule struct {
	Before  *ComplianceTemporalRuleBefore  `json:"-"`
	After   *ComplianceTemporalRuleAfter   `json:"-"`
	Ordered *ComplianceTemporalRuleOrdered `json:"-"`
}

func (value ComplianceTemporalRule) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Before != nil {
		if value.Before.Kind != "before" {
			return nil, fmt.Errorf("ComplianceTemporalRule: selected variant has discriminator %q", value.Before.Kind)
		}
		candidate, err := json.Marshal(value.Before)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.After != nil {
		if value.After.Kind != "after" {
			return nil, fmt.Errorf("ComplianceTemporalRule: selected variant has discriminator %q", value.After.Kind)
		}
		candidate, err := json.Marshal(value.After)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Ordered != nil {
		if value.Ordered.Kind != "ordered" {
			return nil, fmt.Errorf("ComplianceTemporalRule: selected variant has discriminator %q", value.Ordered.Kind)
		}
		candidate, err := json.Marshal(value.Ordered)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ComplianceTemporalRule: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ComplianceTemporalRule) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ComplianceTemporalRule{}
	switch discriminator.Kind {
	case "before":
		var variant ComplianceTemporalRuleBefore
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Before = &variant
	case "after":
		var variant ComplianceTemporalRuleAfter
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.After = &variant
	case "ordered":
		var variant ComplianceTemporalRuleOrdered
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Ordered = &variant
	default:
		return fmt.Errorf("ComplianceTemporalRule: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type LoopEvent struct {
	ToolUse         *LoopEventToolUse         `json:"-"`
	ToolResult      *LoopEventToolResult      `json:"-"`
	WakeupScheduled *LoopEventWakeupScheduled `json:"-"`
	WakeupObserved  *LoopEventWakeupObserved  `json:"-"`
	ParseError      *LoopEventParseError      `json:"-"`
}

func (value LoopEvent) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.ToolUse != nil {
		if value.ToolUse.Kind != "tool_use" {
			return nil, fmt.Errorf("LoopEvent: selected variant has discriminator %q", value.ToolUse.Kind)
		}
		candidate, err := json.Marshal(value.ToolUse)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ToolResult != nil {
		if value.ToolResult.Kind != "tool_result" {
			return nil, fmt.Errorf("LoopEvent: selected variant has discriminator %q", value.ToolResult.Kind)
		}
		candidate, err := json.Marshal(value.ToolResult)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.WakeupScheduled != nil {
		if value.WakeupScheduled.Kind != "wakeup_scheduled" {
			return nil, fmt.Errorf("LoopEvent: selected variant has discriminator %q", value.WakeupScheduled.Kind)
		}
		candidate, err := json.Marshal(value.WakeupScheduled)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.WakeupObserved != nil {
		if value.WakeupObserved.Kind != "wakeup_observed" {
			return nil, fmt.Errorf("LoopEvent: selected variant has discriminator %q", value.WakeupObserved.Kind)
		}
		candidate, err := json.Marshal(value.WakeupObserved)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ParseError != nil {
		if value.ParseError.Kind != "parse_error" {
			return nil, fmt.Errorf("LoopEvent: selected variant has discriminator %q", value.ParseError.Kind)
		}
		candidate, err := json.Marshal(value.ParseError)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("LoopEvent: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *LoopEvent) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = LoopEvent{}
	switch discriminator.Kind {
	case "tool_use":
		var variant LoopEventToolUse
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ToolUse = &variant
	case "tool_result":
		var variant LoopEventToolResult
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ToolResult = &variant
	case "wakeup_scheduled":
		var variant LoopEventWakeupScheduled
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.WakeupScheduled = &variant
	case "wakeup_observed":
		var variant LoopEventWakeupObserved
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.WakeupObserved = &variant
	case "parse_error":
		var variant LoopEventParseError
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ParseError = &variant
	default:
		return fmt.Errorf("LoopEvent: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type ActionInputV1 struct {
	ComplianceTraceGrade       *ActionInputComplianceTraceGradeV1       `json:"-"`
	ConfigInvariantScan        *ActionInputConfigInvariantScanV1        `json:"-"`
	CostAggregate              *ActionInputCostAggregateV1              `json:"-"`
	LoopStatusReduce           *ActionInputLoopStatusReduceV1           `json:"-"`
	ReviewConvergenceReduce    *ActionInputReviewConvergenceReduceV1    `json:"-"`
	RubricEvaluation           *ActionInputRubricEvaluationV1           `json:"-"`
	ServiceDefinitionNormalize *ActionInputServiceDefinitionNormalizeV1 `json:"-"`
	SkillHealthAggregate       *ActionInputSkillHealthAggregateV1       `json:"-"`
	WeightedRanking            *ActionInputWeightedRankingV1            `json:"-"`
	PathInventory              *ActionInputPathInventoryV1              `json:"-"`
	EvidenceSelection          *ActionInputEvidenceSelectionV1          `json:"-"`
}

func (value ActionInputV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.ComplianceTraceGrade != nil {
		if value.ComplianceTraceGrade.Kind != "compliance_trace_grade" {
			return nil, fmt.Errorf("ActionInputV1: selected variant has discriminator %q", value.ComplianceTraceGrade.Kind)
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
			return nil, fmt.Errorf("ActionInputV1: selected variant has discriminator %q", value.ConfigInvariantScan.Kind)
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
			return nil, fmt.Errorf("ActionInputV1: selected variant has discriminator %q", value.CostAggregate.Kind)
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
			return nil, fmt.Errorf("ActionInputV1: selected variant has discriminator %q", value.LoopStatusReduce.Kind)
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
			return nil, fmt.Errorf("ActionInputV1: selected variant has discriminator %q", value.ReviewConvergenceReduce.Kind)
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
			return nil, fmt.Errorf("ActionInputV1: selected variant has discriminator %q", value.RubricEvaluation.Kind)
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
			return nil, fmt.Errorf("ActionInputV1: selected variant has discriminator %q", value.ServiceDefinitionNormalize.Kind)
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
			return nil, fmt.Errorf("ActionInputV1: selected variant has discriminator %q", value.SkillHealthAggregate.Kind)
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
			return nil, fmt.Errorf("ActionInputV1: selected variant has discriminator %q", value.WeightedRanking.Kind)
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
			return nil, fmt.Errorf("ActionInputV1: selected variant has discriminator %q", value.PathInventory.Kind)
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
			return nil, fmt.Errorf("ActionInputV1: selected variant has discriminator %q", value.EvidenceSelection.Kind)
		}
		candidate, err := json.Marshal(value.EvidenceSelection)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ActionInputV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ActionInputV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ActionInputV1{}
	switch discriminator.Kind {
	case "compliance_trace_grade":
		var variant ActionInputComplianceTraceGradeV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ComplianceTraceGrade = &variant
	case "config_invariant_scan":
		var variant ActionInputConfigInvariantScanV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ConfigInvariantScan = &variant
	case "cost_aggregate":
		var variant ActionInputCostAggregateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.CostAggregate = &variant
	case "loop_status_reduce":
		var variant ActionInputLoopStatusReduceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.LoopStatusReduce = &variant
	case "review_convergence_reduce":
		var variant ActionInputReviewConvergenceReduceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ReviewConvergenceReduce = &variant
	case "rubric_evaluation":
		var variant ActionInputRubricEvaluationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.RubricEvaluation = &variant
	case "service_definition_normalize":
		var variant ActionInputServiceDefinitionNormalizeV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ServiceDefinitionNormalize = &variant
	case "skill_health_aggregate":
		var variant ActionInputSkillHealthAggregateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.SkillHealthAggregate = &variant
	case "weighted_ranking":
		var variant ActionInputWeightedRankingV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.WeightedRanking = &variant
	case "path_inventory":
		var variant ActionInputPathInventoryV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.PathInventory = &variant
	case "evidence_selection":
		var variant ActionInputEvidenceSelectionV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.EvidenceSelection = &variant
	default:
		return fmt.Errorf("ActionInputV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
