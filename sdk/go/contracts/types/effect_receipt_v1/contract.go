// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package effect_receipt_v1 contains the generated effect-receipt.v1 data model.
package effect_receipt_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import typed_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/typed_digest_v1"

type ActionIDV1 string

type BoundedResidentTextV1 string

type CanonicalSignatureV1 string

type CapabilityAttemptIDV1 string

type CapabilityIDV1 string

type EffectReceiptV1 struct {
	// AbsenceEvidenceRef corresponds to the JSON schema field "absence_evidence_ref".
	AbsenceEvidenceRef *ResidentRecordRefV1 `json:"absence_evidence_ref,omitempty,omitzero"`

	// AbsenceEvidenceSha256 corresponds to the JSON schema field
	// "absence_evidence_sha256".
	AbsenceEvidenceSha256 *sha256_digest_v1.Sha256DigestV1 `json:"absence_evidence_sha256,omitempty,omitzero"`

	// ActionBodySha256 corresponds to the JSON schema field "action_body_sha256".
	ActionBodySha256 sha256_digest_v1.Sha256DigestV1 `json:"action_body_sha256"`

	// ActionID corresponds to the JSON schema field "action_id".
	ActionID ActionIDV1 `json:"action_id"`

	// AttemptID corresponds to the JSON schema field "attempt_id".
	AttemptID CapabilityAttemptIDV1 `json:"attempt_id"`

	// CanonicalReceiptSha256 corresponds to the JSON schema field
	// "canonical_receipt_sha256".
	CanonicalReceiptSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_receipt_sha256"`

	// CompletedAt corresponds to the JSON schema field "completed_at".
	CompletedAt UnixTimeV1 `json:"completed_at"`

	// ConflictClaimRef corresponds to the JSON schema field "conflict_claim_ref".
	ConflictClaimRef ResidentRecordRefV1 `json:"conflict_claim_ref"`

	// ConflictClaimSha256 corresponds to the JSON schema field
	// "conflict_claim_sha256".
	ConflictClaimSha256 sha256_digest_v1.Sha256DigestV1 `json:"conflict_claim_sha256"`

	// ControllerScope corresponds to the JSON schema field "controller_scope".
	ControllerScope ResidentControllerIDV1 `json:"controller_scope"`

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

	// EffectDispatchIntentRef corresponds to the JSON schema field
	// "effect_dispatch_intent_ref".
	EffectDispatchIntentRef ResidentRecordRefV1 `json:"effect_dispatch_intent_ref"`

	// EffectDispatchIntentSha256 corresponds to the JSON schema field
	// "effect_dispatch_intent_sha256".
	EffectDispatchIntentSha256 sha256_digest_v1.Sha256DigestV1 `json:"effect_dispatch_intent_sha256"`

	// ExternalReference corresponds to the JSON schema field "external_reference".
	ExternalReference *ResidentRecordRefV1 `json:"external_reference,omitempty,omitzero"`

	// ImplementationKind corresponds to the JSON schema field "implementation_kind".
	ImplementationKind BoundedResidentTextV1 `json:"implementation_kind"`

	// ImplementationVersion corresponds to the JSON schema field
	// "implementation_version".
	ImplementationVersion BoundedResidentTextV1 `json:"implementation_version"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// ObservedEffect corresponds to the JSON schema field "observed_effect".
	ObservedEffect ObservedEffectV1 `json:"observed_effect"`

	// ObservedPostconditionDigest corresponds to the JSON schema field
	// "observed_postcondition_digest".
	ObservedPostconditionDigest *typed_digest_v1.TypedDigestV1 `json:"observed_postcondition_digest,omitempty,omitzero"`

	// PrincipalScope corresponds to the JSON schema field "principal_scope".
	PrincipalScope ResidentPrincipalIDV1 `json:"principal_scope"`

	// ProviderRequestDigest corresponds to the JSON schema field
	// "provider_request_digest".
	ProviderRequestDigest *typed_digest_v1.TypedDigestV1 `json:"provider_request_digest,omitempty,omitzero"`

	// ReconciliationCaseID corresponds to the JSON schema field
	// "reconciliation_case_id".
	ReconciliationCaseID *ReconciliationCaseIDV1 `json:"reconciliation_case_id,omitempty,omitzero"`

	// ReconciliationRequired corresponds to the JSON schema field
	// "reconciliation_required".
	ReconciliationRequired bool `json:"reconciliation_required"`

	// RelationshipScope corresponds to the JSON schema field "relationship_scope".
	RelationshipScope *RelationshipScopeV1 `json:"relationship_scope,omitempty,omitzero"`

	// RequestBinding corresponds to the JSON schema field "request_binding".
	RequestBinding RequestBindingV1 `json:"request_binding"`

	// ResponseDigest corresponds to the JSON schema field "response_digest".
	ResponseDigest *typed_digest_v1.TypedDigestV1 `json:"response_digest,omitempty,omitzero"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID string `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// StartedAt corresponds to the JSON schema field "started_at".
	StartedAt UnixTimeV1 `json:"started_at"`

	// TransportStatus corresponds to the JSON schema field "transport_status".
	TransportStatus BoundedResidentTextV1 `json:"transport_status"`
}

type ObservedEffectV1 string

const ObservedEffectV1Confirmed ObservedEffectV1 = "confirmed"
const ObservedEffectV1NotApplied ObservedEffectV1 = "not_applied"
const ObservedEffectV1Partial ObservedEffectV1 = "partial"
const ObservedEffectV1Unknown ObservedEffectV1 = "unknown"

type ReconciliationCaseIDV1 string

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
