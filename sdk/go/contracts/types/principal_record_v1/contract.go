// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package principal_record_v1 contains the generated principal-record.v1 data model.
package principal_record_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type BoundedResidentTextV1 string

type CanonicalSignatureV1 string

type Ed25519PublicKeyV1 string

type PrincipalKeyEpochV1 int

type PrincipalRecordTransitionKindV1 string

const PrincipalRecordTransitionKindV1Initial PrincipalRecordTransitionKindV1 = "initial"
const PrincipalRecordTransitionKindV1KeyTransition PrincipalRecordTransitionKindV1 = "key_transition"
const PrincipalRecordTransitionKindV1PolicyUpdate PrincipalRecordTransitionKindV1 = "policy_update"

type PrincipalRecordV1 struct {
	// CanonicalRecordSha256 corresponds to the JSON schema field
	// "canonical_record_sha256".
	CanonicalRecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_record_sha256"`

	// DisplayHint corresponds to the JSON schema field "display_hint".
	DisplayHint BoundedResidentTextV1 `json:"display_hint"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt UnixTimeV1 `json:"expires_at"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// KeyEpoch corresponds to the JSON schema field "key_epoch".
	KeyEpoch PrincipalKeyEpochV1 `json:"key_epoch"`

	// KeyTransitionRef corresponds to the JSON schema field "key_transition_ref".
	KeyTransitionRef *ResidentRecordRefV1 `json:"key_transition_ref,omitempty,omitzero"`

	// KeyTransitionSha256 corresponds to the JSON schema field
	// "key_transition_sha256".
	KeyTransitionSha256 *sha256_digest_v1.Sha256DigestV1 `json:"key_transition_sha256,omitempty,omitzero"`

	// PreviousRecordSha256 corresponds to the JSON schema field
	// "previous_record_sha256".
	PreviousRecordSha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_record_sha256,omitempty,omitzero"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// RecordTransitionKind corresponds to the JSON schema field
	// "record_transition_kind".
	RecordTransitionKind PrincipalRecordTransitionKindV1 `json:"record_transition_kind"`

	// RecoveryPolicyRef corresponds to the JSON schema field "recovery_policy_ref".
	RecoveryPolicyRef ResidentRecordRefV1 `json:"recovery_policy_ref"`

	// RecoveryPolicySha256 corresponds to the JSON schema field
	// "recovery_policy_sha256".
	RecoveryPolicySha256 sha256_digest_v1.Sha256DigestV1 `json:"recovery_policy_sha256"`

	// RootKeyID corresponds to the JSON schema field "root_key_id".
	RootKeyID ResidentKeyIDV1 `json:"root_key_id"`

	// RootPublicKey corresponds to the JSON schema field "root_public_key".
	RootPublicKey Ed25519PublicKeyV1 `json:"root_public_key"`

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

type SchemaID string

type UnixTimeV1 int
