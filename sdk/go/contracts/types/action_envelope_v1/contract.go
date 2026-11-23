// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package action_envelope_v1 contains the generated action-envelope.v1 data model.
package action_envelope_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import typed_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/typed_digest_v1"

type ActionEnvelopeV1 struct {
	// ActionID corresponds to the JSON schema field "action_id".
	ActionID ActionIDV1 `json:"action_id"`

	// BusinessIdempotencyKey corresponds to the JSON schema field
	// "business_idempotency_key".
	BusinessIdempotencyKey BusinessIdempotencyKeyV1 `json:"business_idempotency_key"`

	// ConflictResourceKey corresponds to the JSON schema field
	// "conflict_resource_key".
	ConflictResourceKey any `json:"conflict_resource_key"`

	// ConflictResourceKeySchemaID corresponds to the JSON schema field
	// "conflict_resource_key_schema_id".
	ConflictResourceKeySchemaID SchemaID `json:"conflict_resource_key_schema_id"`

	// ConflictResourceKeySchemaSha256 corresponds to the JSON schema field
	// "conflict_resource_key_schema_sha256".
	ConflictResourceKeySchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"conflict_resource_key_schema_sha256"`

	// EffectPolicy corresponds to the JSON schema field "effect_policy".
	EffectPolicy ResidentRecordRefV1 `json:"effect_policy"`

	// ExpectedPreconditionDigest corresponds to the JSON schema field
	// "expected_precondition_digest".
	ExpectedPreconditionDigest *typed_digest_v1.TypedDigestV1 `json:"expected_precondition_digest,omitempty,omitzero"`

	// ImmutablePayloadRef corresponds to the JSON schema field
	// "immutable_payload_ref".
	ImmutablePayloadRef *ImmutableActionPayloadRefV1 `json:"immutable_payload_ref,omitempty,omitzero"`

	// InlinePayload corresponds to the JSON schema field "inline_payload".
	InlinePayload any `json:"inline_payload,omitempty,omitzero"`

	// Operation corresponds to the JSON schema field "operation".
	Operation ActionOperationV1 `json:"operation"`

	// PayloadHash corresponds to the JSON schema field "payload_hash".
	PayloadHash sha256_digest_v1.Sha256DigestV1 `json:"payload_hash"`

	// PayloadSchemaID corresponds to the JSON schema field "payload_schema_id".
	PayloadSchemaID SchemaID `json:"payload_schema_id"`

	// PayloadSchemaSha256 corresponds to the JSON schema field
	// "payload_schema_sha256".
	PayloadSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"payload_schema_sha256"`

	// ReconciliationPolicy corresponds to the JSON schema field
	// "reconciliation_policy".
	ReconciliationPolicy ResidentRecordRefV1 `json:"reconciliation_policy"`

	// TargetIdentity corresponds to the JSON schema field "target_identity".
	TargetIdentity ActionTargetIdentityV1 `json:"target_identity"`

	// TargetSystem corresponds to the JSON schema field "target_system".
	TargetSystem ActionTargetSystemV1 `json:"target_system"`
}

type ActionIDV1 string

type ActionOperationV1 string

type ActionTargetIdentityV1 string

type ActionTargetSystemV1 string

type BusinessIdempotencyKeyV1 string

type ImmutableActionPayloadRefV1 struct {
	// PayloadRef corresponds to the JSON schema field "payload_ref".
	PayloadRef ResidentRecordRefV1 `json:"payload_ref"`

	// PayloadRefSha256 corresponds to the JSON schema field "payload_ref_sha256".
	PayloadRefSha256 sha256_digest_v1.Sha256DigestV1 `json:"payload_ref_sha256"`
}

type ResidentRecordRefV1 string

type SchemaID string
