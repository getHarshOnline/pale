// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package reconciliation_result_v1 contains the generated reconciliation-result.v1 data model.
package reconciliation_result_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import typed_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/typed_digest_v1"

type ActionIDV1 string

type CanonicalSignatureV1 string

type CapabilityAttemptIDV1 string

type CapabilityIDV1 string

type ReconciliationCaseRevisionV1 int

type ReconciliationCaseStateV1 string

const ReconciliationCaseStateV1Drift ReconciliationCaseStateV1 = "drift"
const ReconciliationCaseStateV1Matched ReconciliationCaseStateV1 = "matched"
const ReconciliationCaseStateV1NotApplied ReconciliationCaseStateV1 = "not_applied"
const ReconciliationCaseStateV1Unknown ReconciliationCaseStateV1 = "unknown"

type ReconciliationCompensationBindingV1 struct {
	// CaseRef corresponds to the JSON schema field "case_ref".
	CaseRef ResidentRecordRefV1 `json:"case_ref"`

	// CaseSha256 corresponds to the JSON schema field "case_sha256".
	CaseSha256 sha256_digest_v1.Sha256DigestV1 `json:"case_sha256"`

	// CompensationActionRef corresponds to the JSON schema field
	// "compensation_action_ref".
	CompensationActionRef ResidentRecordRefV1 `json:"compensation_action_ref"`

	// CompensationActionSha256 corresponds to the JSON schema field
	// "compensation_action_sha256".
	CompensationActionSha256 sha256_digest_v1.Sha256DigestV1 `json:"compensation_action_sha256"`

	// CompensationRequestRef corresponds to the JSON schema field
	// "compensation_request_ref".
	CompensationRequestRef ResidentRecordRefV1 `json:"compensation_request_ref"`

	// CompensationRequestSha256 corresponds to the JSON schema field
	// "compensation_request_sha256".
	CompensationRequestSha256 sha256_digest_v1.Sha256DigestV1 `json:"compensation_request_sha256"`

	// CompensationResultRef corresponds to the JSON schema field
	// "compensation_result_ref".
	CompensationResultRef ResidentRecordRefV1 `json:"compensation_result_ref"`

	// CompensationResultSha256 corresponds to the JSON schema field
	// "compensation_result_sha256".
	CompensationResultSha256 sha256_digest_v1.Sha256DigestV1 `json:"compensation_result_sha256"`

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
}

type ReconciliationCustodyGenerationV1 int

type ReconciliationEffectReceiptRefV1 struct {
	// ReceiptRef corresponds to the JSON schema field "receipt_ref".
	ReceiptRef ResidentRecordRefV1 `json:"receipt_ref"`

	// ReceiptSha256 corresponds to the JSON schema field "receipt_sha256".
	ReceiptSha256 sha256_digest_v1.Sha256DigestV1 `json:"receipt_sha256"`
}

type ReconciliationIDV1 string

type ReconciliationObservationEvidenceV1 struct {
	// EvidenceRef corresponds to the JSON schema field "evidence_ref".
	EvidenceRef ResidentRecordRefV1 `json:"evidence_ref"`

	// EvidenceSha256 corresponds to the JSON schema field "evidence_sha256".
	EvidenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"evidence_sha256"`
}

type ReconciliationResolutionV1 string

const ReconciliationResolutionV1Compensated ReconciliationResolutionV1 = "compensated"
const ReconciliationResolutionV1EffectAbsent ReconciliationResolutionV1 = "effect_absent"
const ReconciliationResolutionV1EffectConfirmed ReconciliationResolutionV1 = "effect_confirmed"
const ReconciliationResolutionV1ExpectedStateRestored ReconciliationResolutionV1 = "expected_state_restored"
const ReconciliationResolutionV1ManualReviewRequired ReconciliationResolutionV1 = "manual_review_required"
const ReconciliationResolutionV1ProviderStateAccepted ReconciliationResolutionV1 = "provider_state_accepted"
const ReconciliationResolutionV1StillUnknown ReconciliationResolutionV1 = "still_unknown"

type ReconciliationResultV1 struct {
	// AbsenceEvidenceRef corresponds to the JSON schema field "absence_evidence_ref".
	AbsenceEvidenceRef *ResidentRecordRefV1 `json:"absence_evidence_ref,omitempty,omitzero"`

	// AbsenceEvidenceSha256 corresponds to the JSON schema field
	// "absence_evidence_sha256".
	AbsenceEvidenceSha256 *sha256_digest_v1.Sha256DigestV1 `json:"absence_evidence_sha256,omitempty,omitzero"`

	// ActionID corresponds to the JSON schema field "action_id".
	ActionID ActionIDV1 `json:"action_id"`

	// CanonicalReconciliationSha256 corresponds to the JSON schema field
	// "canonical_reconciliation_sha256".
	CanonicalReconciliationSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_reconciliation_sha256"`

	// CaseRef corresponds to the JSON schema field "case_ref".
	CaseRef ResidentRecordRefV1 `json:"case_ref"`

	// CaseRevision corresponds to the JSON schema field "case_revision".
	CaseRevision ReconciliationCaseRevisionV1 `json:"case_revision"`

	// CaseSha256 corresponds to the JSON schema field "case_sha256".
	CaseSha256 sha256_digest_v1.Sha256DigestV1 `json:"case_sha256"`

	// CaseState corresponds to the JSON schema field "case_state".
	CaseState ReconciliationCaseStateV1 `json:"case_state"`

	// ClosedAt corresponds to the JSON schema field "closed_at".
	ClosedAt *UnixTimeV1 `json:"closed_at,omitempty,omitzero"`

	// Compensation corresponds to the JSON schema field "compensation".
	Compensation *ReconciliationCompensationBindingV1 `json:"compensation,omitempty,omitzero"`

	// ControllerScope corresponds to the JSON schema field "controller_scope".
	ControllerScope ResidentControllerIDV1 `json:"controller_scope"`

	// CustodyGeneration corresponds to the JSON schema field "custody_generation".
	CustodyGeneration ReconciliationCustodyGenerationV1 `json:"custody_generation"`

	// CustodyRef corresponds to the JSON schema field "custody_ref".
	CustodyRef ResidentRecordRefV1 `json:"custody_ref"`

	// CustodySha256 corresponds to the JSON schema field "custody_sha256".
	CustodySha256 sha256_digest_v1.Sha256DigestV1 `json:"custody_sha256"`

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

	// EffectReceipts corresponds to the JSON schema field "effect_receipts".
	EffectReceipts []ReconciliationEffectReceiptRefV1 `json:"effect_receipts"`

	// ExpectedStateDigest corresponds to the JSON schema field
	// "expected_state_digest".
	ExpectedStateDigest typed_digest_v1.TypedDigestV1 `json:"expected_state_digest"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// KnownAttemptIds corresponds to the JSON schema field "known_attempt_ids".
	KnownAttemptIds []CapabilityAttemptIDV1 `json:"known_attempt_ids"`

	// ObservationEvidence corresponds to the JSON schema field
	// "observation_evidence".
	ObservationEvidence []ReconciliationObservationEvidenceV1 `json:"observation_evidence"`

	// ObservedStateDigest corresponds to the JSON schema field
	// "observed_state_digest".
	ObservedStateDigest *typed_digest_v1.TypedDigestV1 `json:"observed_state_digest,omitempty,omitzero"`

	// OwnerEvidenceRef corresponds to the JSON schema field "owner_evidence_ref".
	OwnerEvidenceRef *ResidentRecordRefV1 `json:"owner_evidence_ref,omitempty,omitzero"`

	// OwnerEvidenceSha256 corresponds to the JSON schema field
	// "owner_evidence_sha256".
	OwnerEvidenceSha256 *sha256_digest_v1.Sha256DigestV1 `json:"owner_evidence_sha256,omitempty,omitzero"`

	// PredecessorResultRef corresponds to the JSON schema field
	// "predecessor_result_ref".
	PredecessorResultRef *ResidentRecordRefV1 `json:"predecessor_result_ref,omitempty,omitzero"`

	// PredecessorResultSha256 corresponds to the JSON schema field
	// "predecessor_result_sha256".
	PredecessorResultSha256 *sha256_digest_v1.Sha256DigestV1 `json:"predecessor_result_sha256,omitempty,omitzero"`

	// PrincipalScope corresponds to the JSON schema field "principal_scope".
	PrincipalScope ResidentPrincipalIDV1 `json:"principal_scope"`

	// ReconciliationID corresponds to the JSON schema field "reconciliation_id".
	ReconciliationID ReconciliationIDV1 `json:"reconciliation_id"`

	// ReconciliationPolicyRef corresponds to the JSON schema field
	// "reconciliation_policy_ref".
	ReconciliationPolicyRef ResidentRecordRefV1 `json:"reconciliation_policy_ref"`

	// ReconciliationPolicySha256 corresponds to the JSON schema field
	// "reconciliation_policy_sha256".
	ReconciliationPolicySha256 sha256_digest_v1.Sha256DigestV1 `json:"reconciliation_policy_sha256"`

	// RecordedAt corresponds to the JSON schema field "recorded_at".
	RecordedAt UnixTimeV1 `json:"recorded_at"`

	// RelationshipScope corresponds to the JSON schema field "relationship_scope".
	RelationshipScope *RelationshipScopeV1 `json:"relationship_scope,omitempty,omitzero"`

	// RequestBinding corresponds to the JSON schema field "request_binding".
	RequestBinding RequestBindingV1 `json:"request_binding"`

	// RequestedAt corresponds to the JSON schema field "requested_at".
	RequestedAt UnixTimeV1 `json:"requested_at"`

	// Resolution corresponds to the JSON schema field "resolution".
	Resolution *ReconciliationResolutionV1 `json:"resolution,omitempty,omitzero"`

	// ReviewerOrPolicyRef corresponds to the JSON schema field
	// "reviewer_or_policy_ref".
	ReviewerOrPolicyRef ResidentRecordRefV1 `json:"reviewer_or_policy_ref"`

	// ReviewerOrPolicySha256 corresponds to the JSON schema field
	// "reviewer_or_policy_sha256".
	ReviewerOrPolicySha256 sha256_digest_v1.Sha256DigestV1 `json:"reviewer_or_policy_sha256"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID string `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`
}

type RelationshipScopeV1 string

type RequestBindingV1 struct {
	// CanonicalRequestSha256 corresponds to the JSON schema field
	// "canonical_request_sha256".
	CanonicalRequestSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_request_sha256"`

	// CapabilityID corresponds to the JSON schema field "capability_id".
	CapabilityID CapabilityIDV1 `json:"capability_id"`

	// ContractSetRef corresponds to the JSON schema field "contract_set_ref".
	ContractSetRef ResidentRecordRefV1 `json:"contract_set_ref"`

	// ContractSetSha256 corresponds to the JSON schema field "contract_set_sha256".
	ContractSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"contract_set_sha256"`

	// CorrelationID corresponds to the JSON schema field "correlation_id".
	CorrelationID ResidentCorrelationIDV1 `json:"correlation_id"`

	// DeploymentBindingRef corresponds to the JSON schema field
	// "deployment_binding_ref".
	DeploymentBindingRef ResidentRecordRefV1 `json:"deployment_binding_ref"`

	// DeploymentBindingSha256 corresponds to the JSON schema field
	// "deployment_binding_sha256".
	DeploymentBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_binding_sha256"`

	// ExecutionOwnerFenceRef corresponds to the JSON schema field
	// "execution_owner_fence_ref".
	ExecutionOwnerFenceRef *ResidentRecordRefV1 `json:"execution_owner_fence_ref,omitempty,omitzero"`

	// ExecutionOwnerFenceSha256 corresponds to the JSON schema field
	// "execution_owner_fence_sha256".
	ExecutionOwnerFenceSha256 *sha256_digest_v1.Sha256DigestV1 `json:"execution_owner_fence_sha256,omitempty,omitzero"`

	// FederationRequestAdmissionRef corresponds to the JSON schema field
	// "federation_request_admission_ref".
	FederationRequestAdmissionRef *ResidentRecordRefV1 `json:"federation_request_admission_ref,omitempty,omitzero"`

	// FederationRequestAdmissionSha256 corresponds to the JSON schema field
	// "federation_request_admission_sha256".
	FederationRequestAdmissionSha256 *sha256_digest_v1.Sha256DigestV1 `json:"federation_request_admission_sha256,omitempty,omitzero"`

	// RequestID corresponds to the JSON schema field "request_id".
	RequestID ResidentRequestIDV1 `json:"request_id"`

	// RouteBindingRef corresponds to the JSON schema field "route_binding_ref".
	RouteBindingRef ResidentRecordRefV1 `json:"route_binding_ref"`

	// RouteBindingSchemaID corresponds to the JSON schema field
	// "route_binding_schema_id".
	RouteBindingSchemaID SchemaID `json:"route_binding_schema_id"`

	// RouteBindingSchemaSha256 corresponds to the JSON schema field
	// "route_binding_schema_sha256".
	RouteBindingSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"route_binding_schema_sha256"`

	// RouteBindingSha256 corresponds to the JSON schema field "route_binding_sha256".
	RouteBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"route_binding_sha256"`

	// RouteGeneration corresponds to the JSON schema field "route_generation".
	RouteGeneration RouteGenerationV1 `json:"route_generation"`

	// SessionGrantID corresponds to the JSON schema field "session_grant_id".
	SessionGrantID SessionGrantIDV1 `json:"session_grant_id"`

	// SessionGrantSha256 corresponds to the JSON schema field "session_grant_sha256".
	SessionGrantSha256 sha256_digest_v1.Sha256DigestV1 `json:"session_grant_sha256"`
}

type ResidentControllerIDV1 string

type ResidentCorrelationIDV1 string

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type ResidentRequestIDV1 string

type RouteGenerationV1 int

type SchemaID string

type SessionGrantIDV1 string

type SigningKeyEpochV1 int

type UnixTimeV1 int
