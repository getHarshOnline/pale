// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package action_contract_v1 contains the generated action-contract.v1 data model.
package action_contract_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type ActionBoundary string

const ActionBoundaryPure ActionBoundary = "pure"

type ActionContractV1 struct {
	// Boundary corresponds to the JSON schema field "boundary".
	Boundary ActionBoundary `json:"boundary"`

	// ContractDigest corresponds to the JSON schema field "contract_digest".
	ContractDigest sha256_digest_v1.Sha256DigestV1 `json:"contract_digest"`

	// Description corresponds to the JSON schema field "description".
	Description string `json:"description"`

	// Effect corresponds to the JSON schema field "effect".
	Effect ActionEffect `json:"effect"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// InputKind corresponds to the JSON schema field "input_kind".
	InputKind ActionInputKind `json:"input_kind"`

	// OutputKind corresponds to the JSON schema field "output_kind".
	OutputKind ActionOutputKind `json:"output_kind"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// Version corresponds to the JSON schema field "version".
	Version int `json:"version"`
}

type ActionEffect string

const ActionEffectCompute ActionEffect = "compute"

type ActionInputKind string

const ActionInputKindComplianceTraceGrade ActionInputKind = "compliance_trace_grade"
const ActionInputKindConfigInvariantScan ActionInputKind = "config_invariant_scan"
const ActionInputKindCostAggregate ActionInputKind = "cost_aggregate"
const ActionInputKindEvidenceSelection ActionInputKind = "evidence_selection"
const ActionInputKindLoopStatusReduce ActionInputKind = "loop_status_reduce"
const ActionInputKindPathInventory ActionInputKind = "path_inventory"
const ActionInputKindReviewConvergenceReduce ActionInputKind = "review_convergence_reduce"
const ActionInputKindRubricEvaluation ActionInputKind = "rubric_evaluation"
const ActionInputKindServiceDefinitionNormalize ActionInputKind = "service_definition_normalize"
const ActionInputKindSkillHealthAggregate ActionInputKind = "skill_health_aggregate"
const ActionInputKindWeightedRanking ActionInputKind = "weighted_ranking"

type ActionOutputKind string

const ActionOutputKindComplianceTraceGrade ActionOutputKind = "compliance_trace_grade"
const ActionOutputKindConfigInvariantScan ActionOutputKind = "config_invariant_scan"
const ActionOutputKindCostAggregate ActionOutputKind = "cost_aggregate"
const ActionOutputKindEvidenceSelection ActionOutputKind = "evidence_selection"
const ActionOutputKindLoopStatusReduce ActionOutputKind = "loop_status_reduce"
const ActionOutputKindPathInventory ActionOutputKind = "path_inventory"
const ActionOutputKindReviewConvergenceReduce ActionOutputKind = "review_convergence_reduce"
const ActionOutputKindRubricEvaluation ActionOutputKind = "rubric_evaluation"
const ActionOutputKindServiceDefinitionNormalize ActionOutputKind = "service_definition_normalize"
const ActionOutputKindSkillHealthAggregate ActionOutputKind = "skill_health_aggregate"
const ActionOutputKindWeightedRanking ActionOutputKind = "weighted_ranking"
