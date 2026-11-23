// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package effect_conflict_claim_v1 contains the generated effect-conflict-claim.v1 data model.
package effect_conflict_claim_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type ActionIDV1 string

type BusinessIdempotencyKeyV1 string

type CanonicalSignatureV1 string

type EffectConflictClaimGenerationV1 int

type EffectConflictClaimIDV1 string

type EffectConflictClaimStateV1 string

const EffectConflictClaimStateV1Active EffectConflictClaimStateV1 = "active"
const EffectConflictClaimStateV1Released EffectConflictClaimStateV1 = "released"
const EffectConflictClaimStateV1Transferred EffectConflictClaimStateV1 = "transferred"

type EffectConflictClaimV1 struct {
	// ActionBodySha256 corresponds to the JSON schema field "action_body_sha256".
	ActionBodySha256 sha256_digest_v1.Sha256DigestV1 `json:"action_body_sha256"`

	// ActionID corresponds to the JSON schema field "action_id".
	ActionID ActionIDV1 `json:"action_id"`

	// BusinessIdempotencyKey corresponds to the JSON schema field
	// "business_idempotency_key".
	BusinessIdempotencyKey BusinessIdempotencyKeyV1 `json:"business_idempotency_key"`

	// CancellationAuthorityRef corresponds to the JSON schema field
	// "cancellation_authority_ref".
	CancellationAuthorityRef *ResidentRecordRefV1 `json:"cancellation_authority_ref,omitempty,omitzero"`

	// CancellationAuthoritySha256 corresponds to the JSON schema field
	// "cancellation_authority_sha256".
	CancellationAuthoritySha256 *sha256_digest_v1.Sha256DigestV1 `json:"cancellation_authority_sha256,omitempty,omitzero"`

	// CanonicalEffectConflictClaimSha256 corresponds to the JSON schema field
	// "canonical_effect_conflict_claim_sha256".
	CanonicalEffectConflictClaimSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_effect_conflict_claim_sha256"`

	// CanonicalRequestSha256 corresponds to the JSON schema field
	// "canonical_request_sha256".
	CanonicalRequestSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_request_sha256"`

	// ClaimGeneration corresponds to the JSON schema field "claim_generation".
	ClaimGeneration EffectConflictClaimGenerationV1 `json:"claim_generation"`

	// ClaimState corresponds to the JSON schema field "claim_state".
	ClaimState EffectConflictClaimStateV1 `json:"claim_state"`

	// ConflictClaimID corresponds to the JSON schema field "conflict_claim_id".
	ConflictClaimID EffectConflictClaimIDV1 `json:"conflict_claim_id"`

	// ConflictResourceKey corresponds to the JSON schema field
	// "conflict_resource_key".
	ConflictResourceKey any `json:"conflict_resource_key"`

	// ConflictResourceKeySchemaID corresponds to the JSON schema field
	// "conflict_resource_key_schema_id".
	ConflictResourceKeySchemaID SchemaID `json:"conflict_resource_key_schema_id"`

	// ConflictResourceKeySchemaSha256 corresponds to the JSON schema field
	// "conflict_resource_key_schema_sha256".
	ConflictResourceKeySchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"conflict_resource_key_schema_sha256"`

	// ControllerScope corresponds to the JSON schema field "controller_scope".
	ControllerScope ResidentControllerIDV1 `json:"controller_scope"`

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`

	// EffectReceiptRef corresponds to the JSON schema field "effect_receipt_ref".
	EffectReceiptRef *ResidentRecordRefV1 `json:"effect_receipt_ref,omitempty,omitzero"`

	// EffectReceiptSha256 corresponds to the JSON schema field
	// "effect_receipt_sha256".
	EffectReceiptSha256 *sha256_digest_v1.Sha256DigestV1 `json:"effect_receipt_sha256,omitempty,omitzero"`

	// HolderPrincipalID corresponds to the JSON schema field "holder_principal_id".
	HolderPrincipalID ResidentPrincipalIDV1 `json:"holder_principal_id"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// PreviousClaimRef corresponds to the JSON schema field "previous_claim_ref".
	PreviousClaimRef *ResidentRecordRefV1 `json:"previous_claim_ref,omitempty,omitzero"`

	// PreviousClaimSha256 corresponds to the JSON schema field
	// "previous_claim_sha256".
	PreviousClaimSha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_claim_sha256,omitempty,omitzero"`

	// PrincipalScope corresponds to the JSON schema field "principal_scope".
	PrincipalScope ResidentPrincipalIDV1 `json:"principal_scope"`

	// ReconciliationCustodyRef corresponds to the JSON schema field
	// "reconciliation_custody_ref".
	ReconciliationCustodyRef *ResidentRecordRefV1 `json:"reconciliation_custody_ref,omitempty,omitzero"`

	// ReconciliationCustodySha256 corresponds to the JSON schema field
	// "reconciliation_custody_sha256".
	ReconciliationCustodySha256 *sha256_digest_v1.Sha256DigestV1 `json:"reconciliation_custody_sha256,omitempty,omitzero"`

	// RelationshipScope corresponds to the JSON schema field "relationship_scope".
	RelationshipScope *RelationshipScopeV1 `json:"relationship_scope,omitempty,omitzero"`

	// RequestID corresponds to the JSON schema field "request_id".
	RequestID ResidentRequestIDV1 `json:"request_id"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// TerminalReason corresponds to the JSON schema field "terminal_reason".
	TerminalReason *EffectConflictTerminalReasonV1 `json:"terminal_reason,omitempty,omitzero"`

	// TransferredOrReleasedAt corresponds to the JSON schema field
	// "transferred_or_released_at".
	TransferredOrReleasedAt *UnixTimeV1 `json:"transferred_or_released_at,omitempty,omitzero"`
}

type EffectConflictTerminalReasonV1 string

const EffectConflictTerminalReasonV1CancelledBeforeDispatch EffectConflictTerminalReasonV1 = "cancelled_before_dispatch"
const EffectConflictTerminalReasonV1Confirmed EffectConflictTerminalReasonV1 = "confirmed"
const EffectConflictTerminalReasonV1NotApplied EffectConflictTerminalReasonV1 = "not_applied"
const EffectConflictTerminalReasonV1ReconciliationCustody EffectConflictTerminalReasonV1 = "reconciliation_custody"

type RelationshipScopeV1 string

type ResidentControllerIDV1 string

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type ResidentRequestIDV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int
