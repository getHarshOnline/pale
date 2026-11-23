// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package reconciliation_request_v1 contains the generated reconciliation-request.v1 data model.
package reconciliation_request_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import typed_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/typed_digest_v1"

type ActionIDV1 string

type ReconciliationCaseRevisionV1 int

type ReconciliationCustodyGenerationV1 int

type ReconciliationEffectReceiptRefV1 struct {
	// ReceiptRef corresponds to the JSON schema field "receipt_ref".
	ReceiptRef ResidentRecordRefV1 `json:"receipt_ref"`

	// ReceiptSha256 corresponds to the JSON schema field "receipt_sha256".
	ReceiptSha256 sha256_digest_v1.Sha256DigestV1 `json:"receipt_sha256"`
}

type ReconciliationIDV1 string

type ReconciliationRequestV1 struct {
	// ActionID corresponds to the JSON schema field "action_id".
	ActionID ActionIDV1 `json:"action_id"`

	// CaseRef corresponds to the JSON schema field "case_ref".
	CaseRef ResidentRecordRefV1 `json:"case_ref"`

	// CaseSha256 corresponds to the JSON schema field "case_sha256".
	CaseSha256 sha256_digest_v1.Sha256DigestV1 `json:"case_sha256"`

	// CustodyRef corresponds to the JSON schema field "custody_ref".
	CustodyRef ResidentRecordRefV1 `json:"custody_ref"`

	// CustodySha256 corresponds to the JSON schema field "custody_sha256".
	CustodySha256 sha256_digest_v1.Sha256DigestV1 `json:"custody_sha256"`

	// EffectAdmissionRef corresponds to the JSON schema field "effect_admission_ref".
	EffectAdmissionRef ResidentRecordRefV1 `json:"effect_admission_ref"`

	// EffectAdmissionSha256 corresponds to the JSON schema field
	// "effect_admission_sha256".
	EffectAdmissionSha256 sha256_digest_v1.Sha256DigestV1 `json:"effect_admission_sha256"`

	// EffectReceipts corresponds to the JSON schema field "effect_receipts".
	EffectReceipts []ReconciliationEffectReceiptRefV1 `json:"effect_receipts"`

	// ExpectedCaseRevision corresponds to the JSON schema field
	// "expected_case_revision".
	ExpectedCaseRevision ReconciliationCaseRevisionV1 `json:"expected_case_revision"`

	// ExpectedCustodyGeneration corresponds to the JSON schema field
	// "expected_custody_generation".
	ExpectedCustodyGeneration ReconciliationCustodyGenerationV1 `json:"expected_custody_generation"`

	// ExpectedStateDigest corresponds to the JSON schema field
	// "expected_state_digest".
	ExpectedStateDigest typed_digest_v1.TypedDigestV1 `json:"expected_state_digest"`

	// ObservationBounds corresponds to the JSON schema field "observation_bounds".
	ObservationBounds any `json:"observation_bounds"`

	// PredecessorResultRef corresponds to the JSON schema field
	// "predecessor_result_ref".
	PredecessorResultRef *ResidentRecordRefV1 `json:"predecessor_result_ref,omitempty,omitzero"`

	// PredecessorResultSha256 corresponds to the JSON schema field
	// "predecessor_result_sha256".
	PredecessorResultSha256 *sha256_digest_v1.Sha256DigestV1 `json:"predecessor_result_sha256,omitempty,omitzero"`

	// ReconciliationID corresponds to the JSON schema field "reconciliation_id".
	ReconciliationID ReconciliationIDV1 `json:"reconciliation_id"`

	// ReconciliationPolicyRef corresponds to the JSON schema field
	// "reconciliation_policy_ref".
	ReconciliationPolicyRef ResidentRecordRefV1 `json:"reconciliation_policy_ref"`

	// ReconciliationPolicySha256 corresponds to the JSON schema field
	// "reconciliation_policy_sha256".
	ReconciliationPolicySha256 sha256_digest_v1.Sha256DigestV1 `json:"reconciliation_policy_sha256"`

	// RequestedAt corresponds to the JSON schema field "requested_at".
	RequestedAt UnixTimeV1 `json:"requested_at"`
}

type ResidentRecordRefV1 string

type UnixTimeV1 int
