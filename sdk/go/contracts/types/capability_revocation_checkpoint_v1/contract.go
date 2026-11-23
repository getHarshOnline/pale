// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package capability_revocation_checkpoint_v1 contains the generated capability-revocation-checkpoint.v1 data model.
package capability_revocation_checkpoint_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type CapabilityRevocationCheckpointEpochV1 int

type CapabilityRevocationCheckpointIDV1 string

type CapabilityRevocationCheckpointV1 struct {
	// AuthorityHeadsRootSha256 corresponds to the JSON schema field
	// "authority_heads_root_sha256".
	AuthorityHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"authority_heads_root_sha256"`

	// AuthorityPrincipalID corresponds to the JSON schema field
	// "authority_principal_id".
	AuthorityPrincipalID ResidentPrincipalIDV1 `json:"authority_principal_id"`

	// CanonicalCheckpointSha256 corresponds to the JSON schema field
	// "canonical_checkpoint_sha256".
	CanonicalCheckpointSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_checkpoint_sha256"`

	// CheckpointEpoch corresponds to the JSON schema field "checkpoint_epoch".
	CheckpointEpoch CapabilityRevocationCheckpointEpochV1 `json:"checkpoint_epoch"`

	// CheckpointID corresponds to the JSON schema field "checkpoint_id".
	CheckpointID CapabilityRevocationCheckpointIDV1 `json:"checkpoint_id"`

	// CoveredGrantCount corresponds to the JSON schema field "covered_grant_count".
	CoveredGrantCount CoveredGrantCountV1 `json:"covered_grant_count"`

	// GeneratedAt corresponds to the JSON schema field "generated_at".
	GeneratedAt UnixTimeV1 `json:"generated_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// PreviousCheckpointRef corresponds to the JSON schema field
	// "previous_checkpoint_ref".
	PreviousCheckpointRef *ResidentRecordRefV1 `json:"previous_checkpoint_ref,omitempty,omitzero"`

	// PreviousCheckpointSha256 corresponds to the JSON schema field
	// "previous_checkpoint_sha256".
	PreviousCheckpointSha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_checkpoint_sha256,omitempty,omitzero"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`
}

type CoveredGrantCountV1 int

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int
