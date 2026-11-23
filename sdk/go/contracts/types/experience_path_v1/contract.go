// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package experience_path_v1 contains the generated experience-path.v1 data model.
package experience_path_v1

import composition_primitives_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/composition_primitives_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type EffectClass string

const EffectClassCompute EffectClass = "compute"
const EffectClassExecute EffectClass = "execute"
const EffectClassNetwork EffectClass = "network"
const EffectClassPublish EffectClass = "publish"
const EffectClassRead EffectClass = "read"
const EffectClassWrite EffectClass = "write"

type ExperiencePathV1 struct {
	// AssuranceSubjectID corresponds to the JSON schema field "assurance_subject_id".
	AssuranceSubjectID string `json:"assurance_subject_id"`

	// Budget corresponds to the JSON schema field "budget".
	Budget composition_primitives_v1.PathBudgetV1 `json:"budget"`

	// CapabilityCeiling corresponds to the JSON schema field "capability_ceiling".
	CapabilityCeiling []string `json:"capability_ceiling"`

	// CompiledContent corresponds to the JSON schema field "compiled_content".
	CompiledContent composition_primitives_v1.CompiledContentCoordinateV1 `json:"compiled_content"`

	// ContinuityContractID corresponds to the JSON schema field
	// "continuity_contract_id".
	ContinuityContractID string `json:"continuity_contract_id"`

	// DestinationConsequences corresponds to the JSON schema field
	// "destination_consequences".
	DestinationConsequences []composition_primitives_v1.DestinationConsequenceV1 `json:"destination_consequences"`

	// EffectCeiling corresponds to the JSON schema field "effect_ceiling".
	EffectCeiling []EffectClass `json:"effect_ceiling"`

	// EntryContractID corresponds to the JSON schema field "entry_contract_id".
	EntryContractID string `json:"entry_contract_id"`

	// EvidenceCarrierID corresponds to the JSON schema field "evidence_carrier_id".
	EvidenceCarrierID string `json:"evidence_carrier_id"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Identity corresponds to the JSON schema field "identity".
	Identity ResponsibilityPathIdentityV1 `json:"identity"`

	// Invocations corresponds to the JSON schema field "invocations".
	Invocations []composition_primitives_v1.PathInvocationV1 `json:"invocations"`

	// NativeOutcomes corresponds to the JSON schema field "native_outcomes".
	NativeOutcomes []composition_primitives_v1.NativeOutcomeReadOnlyResultV1 `json:"native_outcomes"`

	// NativeOwnerTransitions corresponds to the JSON schema field
	// "native_owner_transitions".
	NativeOwnerTransitions []composition_primitives_v1.NativeOwnerTransitionV1 `json:"native_owner_transitions"`

	// OriginReceiptContractID corresponds to the JSON schema field
	// "origin_receipt_contract_id".
	OriginReceiptContractID string `json:"origin_receipt_contract_id"`

	// PrivacyPolicyID corresponds to the JSON schema field "privacy_policy_id".
	PrivacyPolicyID string `json:"privacy_policy_id"`

	// ProviderBindingIds corresponds to the JSON schema field "provider_binding_ids".
	ProviderBindingIds []string `json:"provider_binding_ids"`

	// Reconciliation corresponds to the JSON schema field "reconciliation".
	Reconciliation []composition_primitives_v1.ReconciliationRequirementV1 `json:"reconciliation"`

	// ResponsibilityContractIds corresponds to the JSON schema field
	// "responsibility_contract_ids".
	ResponsibilityContractIds []string `json:"responsibility_contract_ids"`

	// RetentionPolicyID corresponds to the JSON schema field "retention_policy_id".
	RetentionPolicyID string `json:"retention_policy_id"`

	// SettlementContractID corresponds to the JSON schema field
	// "settlement_contract_id".
	SettlementContractID string `json:"settlement_contract_id"`

	// TerminationContractID corresponds to the JSON schema field
	// "termination_contract_id".
	TerminationContractID string `json:"termination_contract_id"`

	// WorkDelegationEdgeIds corresponds to the JSON schema field
	// "work_delegation_edge_ids".
	WorkDelegationEdgeIds []string `json:"work_delegation_edge_ids"`
}

type ResponsibilityPathIdentityV1 struct {
	// ResponsibilityPathSha256 corresponds to the JSON schema field
	// "responsibility_path_sha256".
	ResponsibilityPathSha256 sha256_digest_v1.Sha256DigestV1 `json:"responsibility_path_sha256"`
}
