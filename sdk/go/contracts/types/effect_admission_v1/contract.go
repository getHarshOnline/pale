// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package effect_admission_v1 contains the generated effect-admission.v1 data model.
package effect_admission_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type ActionIDV1 string

type AuthorityEvidenceV1 struct {
	// AuthorityRef corresponds to the JSON schema field "authority_ref".
	AuthorityRef ResidentRecordRefV1 `json:"authority_ref"`

	// AuthoritySha256 corresponds to the JSON schema field "authority_sha256".
	AuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"authority_sha256"`
}

type BoundedResidentTextV1 string

type CanonicalSignatureV1 string

type EffectAdmissionDecisionV1 string

const EffectAdmissionDecisionV1Admitted EffectAdmissionDecisionV1 = "admitted"

type EffectAdmissionIDV1 string

type EffectAdmissionV1 struct {
	// ActionBodySha256 corresponds to the JSON schema field "action_body_sha256".
	ActionBodySha256 sha256_digest_v1.Sha256DigestV1 `json:"action_body_sha256"`

	// ActionID corresponds to the JSON schema field "action_id".
	ActionID ActionIDV1 `json:"action_id"`

	// AuthorityEvidence corresponds to the JSON schema field "authority_evidence".
	AuthorityEvidence []AuthorityEvidenceV1 `json:"authority_evidence"`

	// CanonicalEffectAdmissionSha256 corresponds to the JSON schema field
	// "canonical_effect_admission_sha256".
	CanonicalEffectAdmissionSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_effect_admission_sha256"`

	// CanonicalRequestSha256 corresponds to the JSON schema field
	// "canonical_request_sha256".
	CanonicalRequestSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_request_sha256"`

	// ConflictClaimRef corresponds to the JSON schema field "conflict_claim_ref".
	ConflictClaimRef ResidentRecordRefV1 `json:"conflict_claim_ref"`

	// ConflictClaimSha256 corresponds to the JSON schema field
	// "conflict_claim_sha256".
	ConflictClaimSha256 sha256_digest_v1.Sha256DigestV1 `json:"conflict_claim_sha256"`

	// ConflictResourceKey corresponds to the JSON schema field
	// "conflict_resource_key".
	ConflictResourceKey any `json:"conflict_resource_key"`

	// ConflictResourceKeySchemaID corresponds to the JSON schema field
	// "conflict_resource_key_schema_id".
	ConflictResourceKeySchemaID SchemaID `json:"conflict_resource_key_schema_id"`

	// ConflictResourceKeySchemaSha256 corresponds to the JSON schema field
	// "conflict_resource_key_schema_sha256".
	ConflictResourceKeySchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"conflict_resource_key_schema_sha256"`

	// Decision corresponds to the JSON schema field "decision".
	Decision EffectAdmissionDecisionV1 `json:"decision"`

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`

	// EffectAdmissionID corresponds to the JSON schema field "effect_admission_id".
	EffectAdmissionID EffectAdmissionIDV1 `json:"effect_admission_id"`

	// EffectPolicy corresponds to the JSON schema field "effect_policy".
	EffectPolicy ResidentRecordRefV1 `json:"effect_policy"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt UnixTimeV1 `json:"expires_at"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// ReconciliationPolicy corresponds to the JSON schema field
	// "reconciliation_policy".
	ReconciliationPolicy ResidentRecordRefV1 `json:"reconciliation_policy"`

	// RequestID corresponds to the JSON schema field "request_id".
	RequestID ResidentRequestIDV1 `json:"request_id"`

	// RiskClass corresponds to the JSON schema field "risk_class".
	RiskClass BoundedResidentTextV1 `json:"risk_class"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`
}

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type ResidentRequestIDV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int
