// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package published_generation_head_v1 contains the generated published-generation-head.v1 data model.
package published_generation_head_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type JournalSequenceV1 int

type PublishedGenerationHeadV1 struct {
	// CompiledGenerationDigest corresponds to the JSON schema field
	// "compiled_generation_digest".
	CompiledGenerationDigest sha256_digest_v1.Sha256DigestV1 `json:"compiled_generation_digest"`

	// JournalSequence corresponds to the JSON schema field "journal_sequence".
	JournalSequence JournalSequenceV1 `json:"journal_sequence"`

	// PredecessorPublishedGenerationHeadDigest corresponds to the JSON schema field
	// "predecessor_published_generation_head_digest".
	PredecessorPublishedGenerationHeadDigest *sha256_digest_v1.Sha256DigestV1 `json:"predecessor_published_generation_head_digest,omitempty,omitzero"`

	// PublishedGenerationHeadDigest corresponds to the JSON schema field
	// "published_generation_head_digest".
	PublishedGenerationHeadDigest sha256_digest_v1.Sha256DigestV1 `json:"published_generation_head_digest"`

	// Schema corresponds to the JSON schema field "schema".
	Schema SchemaID `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type SchemaID string
