// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package principal_key_transition_v1 contains the generated principal-key-transition.v1 data model.
package principal_key_transition_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type BoundedResidentTextV1 string

type CanonicalSignatureV1 string

type Ed25519PublicKeyV1 string

type PrincipalKeyEpochV1 int

type PrincipalKeyTransitionModeV1 string

const PrincipalKeyTransitionModeV1NormalRotation PrincipalKeyTransitionModeV1 = "normal_rotation"
const PrincipalKeyTransitionModeV1Recovery PrincipalKeyTransitionModeV1 = "recovery"

type PrincipalKeyTransitionV1 struct {
	// CanonicalRecordSha256 corresponds to the JSON schema field
	// "canonical_record_sha256".
	CanonicalRecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_record_sha256"`

	// EffectiveAt corresponds to the JSON schema field "effective_at".
	EffectiveAt UnixTimeV1 `json:"effective_at"`

	// NewKeyEpoch corresponds to the JSON schema field "new_key_epoch".
	NewKeyEpoch PrincipalKeyEpochV1 `json:"new_key_epoch"`

	// NewKeyID corresponds to the JSON schema field "new_key_id".
	NewKeyID ResidentKeyIDV1 `json:"new_key_id"`

	// NewKeySignature corresponds to the JSON schema field "new_key_signature".
	NewKeySignature CanonicalSignatureV1 `json:"new_key_signature"`

	// NewPublicKey corresponds to the JSON schema field "new_public_key".
	NewPublicKey Ed25519PublicKeyV1 `json:"new_public_key"`

	// OldKeyEpoch corresponds to the JSON schema field "old_key_epoch".
	OldKeyEpoch PrincipalKeyEpochV1 `json:"old_key_epoch"`

	// OldKeyID corresponds to the JSON schema field "old_key_id".
	OldKeyID ResidentKeyIDV1 `json:"old_key_id"`

	// OldKeySignature corresponds to the JSON schema field "old_key_signature".
	OldKeySignature *CanonicalSignatureV1 `json:"old_key_signature,omitempty,omitzero"`

	// OldPublicKey corresponds to the JSON schema field "old_public_key".
	OldPublicKey Ed25519PublicKeyV1 `json:"old_public_key"`

	// PreviousRecordSha256 corresponds to the JSON schema field
	// "previous_record_sha256".
	PreviousRecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"previous_record_sha256"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// RecoveryAuthorityProofRef corresponds to the JSON schema field
	// "recovery_authority_proof_ref".
	RecoveryAuthorityProofRef *ResidentRecordRefV1 `json:"recovery_authority_proof_ref,omitempty,omitzero"`

	// RecoveryAuthorityProofSha256 corresponds to the JSON schema field
	// "recovery_authority_proof_sha256".
	RecoveryAuthorityProofSha256 *sha256_digest_v1.Sha256DigestV1 `json:"recovery_authority_proof_sha256,omitempty,omitzero"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// TransitionMode corresponds to the JSON schema field "transition_mode".
	TransitionMode PrincipalKeyTransitionModeV1 `json:"transition_mode"`

	// TransitionReason corresponds to the JSON schema field "transition_reason".
	TransitionReason BoundedResidentTextV1 `json:"transition_reason"`
}

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type UnixTimeV1 int
