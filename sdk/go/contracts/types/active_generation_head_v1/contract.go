// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package active_generation_head_v1 contains the generated active-generation-head.v1 data model.
package active_generation_head_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type ActiveGenerationHeadV1 struct {
	// ActivationFenceDigest corresponds to the JSON schema field
	// "activation_fence_digest".
	ActivationFenceDigest sha256_digest_v1.Sha256DigestV1 `json:"activation_fence_digest"`

	// ActiveGenerationHeadDigest corresponds to the JSON schema field
	// "active_generation_head_digest".
	ActiveGenerationHeadDigest sha256_digest_v1.Sha256DigestV1 `json:"active_generation_head_digest"`

	// AdvertisedGenerationHeadDigest corresponds to the JSON schema field
	// "advertised_generation_head_digest".
	AdvertisedGenerationHeadDigest sha256_digest_v1.Sha256DigestV1 `json:"advertised_generation_head_digest"`

	// CompiledGenerationDigest corresponds to the JSON schema field
	// "compiled_generation_digest".
	CompiledGenerationDigest sha256_digest_v1.Sha256DigestV1 `json:"compiled_generation_digest"`

	// JournalSequence corresponds to the JSON schema field "journal_sequence".
	JournalSequence JournalSequenceV1 `json:"journal_sequence"`

	// PredecessorActiveGenerationHeadDigest corresponds to the JSON schema field
	// "predecessor_active_generation_head_digest".
	PredecessorActiveGenerationHeadDigest *sha256_digest_v1.Sha256DigestV1 `json:"predecessor_active_generation_head_digest,omitempty,omitzero"`

	// Schema corresponds to the JSON schema field "schema".
	Schema SchemaID `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type JournalSequenceV1 int

type SchemaID string
