// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package reconciliation_case_v1 contains the generated reconciliation-case.v1 data model.
package reconciliation_case_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)
import typed_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/typed_digest_v1"

type ActionIDV1 string

type CanonicalSignatureV1 string

type ObservationKindV1 string

type ReconciliationCaseIDV1 string

type ReconciliationCaseOriginV1 string

const ReconciliationCaseOriginV1Initial ReconciliationCaseOriginV1 = "initial"
const ReconciliationCaseOriginV1Reopened ReconciliationCaseOriginV1 = "reopened"

type ReconciliationCaseRevisionV1 int

type ReconciliationCaseV1 struct {
	// ActionID corresponds to the JSON schema field "action_id".
	ActionID ActionIDV1 `json:"action_id"`

	// CanonicalCaseSha256 corresponds to the JSON schema field
	// "canonical_case_sha256".
	CanonicalCaseSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_case_sha256"`

	// CaseID corresponds to the JSON schema field "case_id".
	CaseID ReconciliationCaseIDV1 `json:"case_id"`

	// CaseOrigin corresponds to the JSON schema field "case_origin".
	CaseOrigin ReconciliationCaseOriginV1 `json:"case_origin"`

	// CaseRevision corresponds to the JSON schema field "case_revision".
	CaseRevision ReconciliationCaseRevisionV1 `json:"case_revision"`

	// ConflictResourceKey corresponds to the JSON schema field
	// "conflict_resource_key".
	ConflictResourceKey any `json:"conflict_resource_key"`

	// ConflictResourceKeySchemaID corresponds to the JSON schema field
	// "conflict_resource_key_schema_id".
	ConflictResourceKeySchemaID SchemaID `json:"conflict_resource_key_schema_id"`

	// ConflictResourceKeySchemaSha256 corresponds to the JSON schema field
	// "conflict_resource_key_schema_sha256".
	ConflictResourceKeySchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"conflict_resource_key_schema_sha256"`

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`

	// EffectAdmissionRef corresponds to the JSON schema field "effect_admission_ref".
	EffectAdmissionRef ResidentRecordRefV1 `json:"effect_admission_ref"`

	// EffectAdmissionSha256 corresponds to the JSON schema field
	// "effect_admission_sha256".
	EffectAdmissionSha256 sha256_digest_v1.Sha256DigestV1 `json:"effect_admission_sha256"`

	// ExpectedStateDigest corresponds to the JSON schema field
	// "expected_state_digest".
	ExpectedStateDigest typed_digest_v1.TypedDigestV1 `json:"expected_state_digest"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// OpenedAt corresponds to the JSON schema field "opened_at".
	OpenedAt UnixTimeV1 `json:"opened_at"`

	// OpeningEvidence corresponds to the JSON schema field "opening_evidence".
	OpeningEvidence []ReconciliationOpeningEvidenceV1 `json:"opening_evidence"`

	// PredecessorCaseRef corresponds to the JSON schema field "predecessor_case_ref".
	PredecessorCaseRef *ResidentRecordRefV1 `json:"predecessor_case_ref,omitempty,omitzero"`

	// PredecessorCaseSha256 corresponds to the JSON schema field
	// "predecessor_case_sha256".
	PredecessorCaseSha256 *sha256_digest_v1.Sha256DigestV1 `json:"predecessor_case_sha256,omitempty,omitzero"`

	// PredecessorTerminalResultRef corresponds to the JSON schema field
	// "predecessor_terminal_result_ref".
	PredecessorTerminalResultRef *ResidentRecordRefV1 `json:"predecessor_terminal_result_ref,omitempty,omitzero"`

	// PredecessorTerminalResultSha256 corresponds to the JSON schema field
	// "predecessor_terminal_result_sha256".
	PredecessorTerminalResultSha256 *sha256_digest_v1.Sha256DigestV1 `json:"predecessor_terminal_result_sha256,omitempty,omitzero"`

	// ReconciliationPolicyRef corresponds to the JSON schema field
	// "reconciliation_policy_ref".
	ReconciliationPolicyRef ResidentRecordRefV1 `json:"reconciliation_policy_ref"`

	// ReconciliationPolicySha256 corresponds to the JSON schema field
	// "reconciliation_policy_sha256".
	ReconciliationPolicySha256 sha256_digest_v1.Sha256DigestV1 `json:"reconciliation_policy_sha256"`

	// ReopenAuthorityRef corresponds to the JSON schema field "reopen_authority_ref".
	ReopenAuthorityRef *ResidentRecordRefV1 `json:"reopen_authority_ref,omitempty,omitzero"`

	// ReopenAuthoritySha256 corresponds to the JSON schema field
	// "reopen_authority_sha256".
	ReopenAuthoritySha256 *sha256_digest_v1.Sha256DigestV1 `json:"reopen_authority_sha256,omitempty,omitzero"`

	// ReopenPolicyRef corresponds to the JSON schema field "reopen_policy_ref".
	ReopenPolicyRef *ResidentRecordRefV1 `json:"reopen_policy_ref,omitempty,omitzero"`

	// ReopenPolicySha256 corresponds to the JSON schema field "reopen_policy_sha256".
	ReopenPolicySha256 *sha256_digest_v1.Sha256DigestV1 `json:"reopen_policy_sha256,omitempty,omitzero"`

	// ReopenReasonCode corresponds to the JSON schema field "reopen_reason_code".
	ReopenReasonCode *ResidentReasonV1 `json:"reopen_reason_code,omitempty,omitzero"`

	// ReservedCustodyID corresponds to the JSON schema field "reserved_custody_id".
	ReservedCustodyID ReconciliationCustodyIDV1 `json:"reserved_custody_id"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID string `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`
}

type ReconciliationCustodyIDV1 string

type ReconciliationOpeningEvidenceEffectReceiptV1 struct {
	// EvidenceRef corresponds to the JSON schema field "evidence_ref".
	EvidenceRef ResidentRecordRefV1 `json:"evidence_ref"`

	// EvidenceSchemaID corresponds to the JSON schema field "evidence_schema_id".
	EvidenceSchemaID SchemaID `json:"evidence_schema_id"`

	// EvidenceSchemaSha256 corresponds to the JSON schema field
	// "evidence_schema_sha256".
	EvidenceSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"evidence_schema_sha256"`

	// EvidenceSha256 corresponds to the JSON schema field "evidence_sha256".
	EvidenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"evidence_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ReconciliationOpeningEvidenceObservationV1 struct {
	// EvidenceRef corresponds to the JSON schema field "evidence_ref".
	EvidenceRef ResidentRecordRefV1 `json:"evidence_ref"`

	// EvidenceSchemaID corresponds to the JSON schema field "evidence_schema_id".
	EvidenceSchemaID SchemaID `json:"evidence_schema_id"`

	// EvidenceSchemaSha256 corresponds to the JSON schema field
	// "evidence_schema_sha256".
	EvidenceSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"evidence_schema_sha256"`

	// EvidenceSha256 corresponds to the JSON schema field "evidence_sha256".
	EvidenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"evidence_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ObservationKind corresponds to the JSON schema field "observation_kind".
	ObservationKind ObservationKindV1 `json:"observation_kind"`
}

type ReconciliationOpeningEvidenceReviewerEvidenceV1 struct {
	// EvidenceRef corresponds to the JSON schema field "evidence_ref".
	EvidenceRef ResidentRecordRefV1 `json:"evidence_ref"`

	// EvidenceSchemaID corresponds to the JSON schema field "evidence_schema_id".
	EvidenceSchemaID SchemaID `json:"evidence_schema_id"`

	// EvidenceSchemaSha256 corresponds to the JSON schema field
	// "evidence_schema_sha256".
	EvidenceSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"evidence_schema_sha256"`

	// EvidenceSha256 corresponds to the JSON schema field "evidence_sha256".
	EvidenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"evidence_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ReviewerOrPolicyRef corresponds to the JSON schema field
	// "reviewer_or_policy_ref".
	ReviewerOrPolicyRef ResidentRecordRefV1 `json:"reviewer_or_policy_ref"`

	// ReviewerOrPolicySha256 corresponds to the JSON schema field
	// "reviewer_or_policy_sha256".
	ReviewerOrPolicySha256 sha256_digest_v1.Sha256DigestV1 `json:"reviewer_or_policy_sha256"`
}

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentReasonV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int

type ReconciliationOpeningEvidenceV1 struct {
	EffectReceipt    *ReconciliationOpeningEvidenceEffectReceiptV1    `json:"-"`
	Observation      *ReconciliationOpeningEvidenceObservationV1      `json:"-"`
	ReviewerEvidence *ReconciliationOpeningEvidenceReviewerEvidenceV1 `json:"-"`
}

func (value ReconciliationOpeningEvidenceV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.EffectReceipt != nil {
		if value.EffectReceipt.Kind != "effect_receipt" {
			return nil, fmt.Errorf("ReconciliationOpeningEvidenceV1: selected variant has discriminator %q", value.EffectReceipt.Kind)
		}
		candidate, err := json.Marshal(value.EffectReceipt)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Observation != nil {
		if value.Observation.Kind != "observation" {
			return nil, fmt.Errorf("ReconciliationOpeningEvidenceV1: selected variant has discriminator %q", value.Observation.Kind)
		}
		candidate, err := json.Marshal(value.Observation)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ReviewerEvidence != nil {
		if value.ReviewerEvidence.Kind != "reviewer_evidence" {
			return nil, fmt.Errorf("ReconciliationOpeningEvidenceV1: selected variant has discriminator %q", value.ReviewerEvidence.Kind)
		}
		candidate, err := json.Marshal(value.ReviewerEvidence)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ReconciliationOpeningEvidenceV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ReconciliationOpeningEvidenceV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ReconciliationOpeningEvidenceV1{}
	switch discriminator.Kind {
	case "effect_receipt":
		var variant ReconciliationOpeningEvidenceEffectReceiptV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.EffectReceipt = &variant
	case "observation":
		var variant ReconciliationOpeningEvidenceObservationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Observation = &variant
	case "reviewer_evidence":
		var variant ReconciliationOpeningEvidenceReviewerEvidenceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ReviewerEvidence = &variant
	default:
		return fmt.Errorf("ReconciliationOpeningEvidenceV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
