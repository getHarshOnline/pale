// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package effect_dispatch_intent_v1 contains the generated effect-dispatch-intent.v1 data model.
package effect_dispatch_intent_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import typed_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/typed_digest_v1"

type ActionIDV1 string

type BoundedResidentTextV1 string

type CanonicalSignatureV1 string

type CapabilityAttemptIDV1 string

type EffectDispatchIntentIDV1 string

type EffectDispatchIntentV1 struct {
	// ActionBodySha256 corresponds to the JSON schema field "action_body_sha256".
	ActionBodySha256 sha256_digest_v1.Sha256DigestV1 `json:"action_body_sha256"`

	// ActionID corresponds to the JSON schema field "action_id".
	ActionID ActionIDV1 `json:"action_id"`

	// AttemptID corresponds to the JSON schema field "attempt_id".
	AttemptID CapabilityAttemptIDV1 `json:"attempt_id"`

	// CanonicalDispatchIntentSha256 corresponds to the JSON schema field
	// "canonical_dispatch_intent_sha256".
	CanonicalDispatchIntentSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_dispatch_intent_sha256"`

	// CanonicalRequestSha256 corresponds to the JSON schema field
	// "canonical_request_sha256".
	CanonicalRequestSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_request_sha256"`

	// ConflictClaimRef corresponds to the JSON schema field "conflict_claim_ref".
	ConflictClaimRef ResidentRecordRefV1 `json:"conflict_claim_ref"`

	// ConflictClaimSha256 corresponds to the JSON schema field
	// "conflict_claim_sha256".
	ConflictClaimSha256 sha256_digest_v1.Sha256DigestV1 `json:"conflict_claim_sha256"`

	// ContractSetRef corresponds to the JSON schema field "contract_set_ref".
	ContractSetRef ResidentRecordRefV1 `json:"contract_set_ref"`

	// ContractSetSha256 corresponds to the JSON schema field "contract_set_sha256".
	ContractSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"contract_set_sha256"`

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

	// EffectDispatchIntentID corresponds to the JSON schema field
	// "effect_dispatch_intent_id".
	EffectDispatchIntentID EffectDispatchIntentIDV1 `json:"effect_dispatch_intent_id"`

	// ExecutionOwnerFenceRef corresponds to the JSON schema field
	// "execution_owner_fence_ref".
	ExecutionOwnerFenceRef *ResidentRecordRefV1 `json:"execution_owner_fence_ref,omitempty,omitzero"`

	// ExecutionOwnerFenceSha256 corresponds to the JSON schema field
	// "execution_owner_fence_sha256".
	ExecutionOwnerFenceSha256 *sha256_digest_v1.Sha256DigestV1 `json:"execution_owner_fence_sha256,omitempty,omitzero"`

	// ImplementationKind corresponds to the JSON schema field "implementation_kind".
	ImplementationKind BoundedResidentTextV1 `json:"implementation_kind"`

	// ImplementationVersion corresponds to the JSON schema field
	// "implementation_version".
	ImplementationVersion BoundedResidentTextV1 `json:"implementation_version"`

	// IntendedAt corresponds to the JSON schema field "intended_at".
	IntendedAt UnixTimeV1 `json:"intended_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// ProviderRequestDigest corresponds to the JSON schema field
	// "provider_request_digest".
	ProviderRequestDigest *typed_digest_v1.TypedDigestV1 `json:"provider_request_digest,omitempty,omitzero"`

	// RequestRef corresponds to the JSON schema field "request_ref".
	RequestRef ResidentRecordRefV1 `json:"request_ref"`

	// ReservedReconciliationCaseID corresponds to the JSON schema field
	// "reserved_reconciliation_case_id".
	ReservedReconciliationCaseID ReconciliationCaseIDV1 `json:"reserved_reconciliation_case_id"`

	// ReservedReconciliationCustodyID corresponds to the JSON schema field
	// "reserved_reconciliation_custody_id".
	ReservedReconciliationCustodyID ReconciliationCustodyIDV1 `json:"reserved_reconciliation_custody_id"`

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

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID string `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`
}

type ReconciliationCaseIDV1 string

type ReconciliationCustodyIDV1 string

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type RouteGenerationV1 int

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int
