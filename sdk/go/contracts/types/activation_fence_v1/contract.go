// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package activation_fence_v1 contains the generated activation-fence.v1 data model.
package activation_fence_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type ActivationFenceGenerationV1 int

type ActivationFenceIDV1 string

type ActivationFenceV1 struct {
	// ActivationFenceDigest corresponds to the JSON schema field
	// "activation_fence_digest".
	ActivationFenceDigest sha256_digest_v1.Sha256DigestV1 `json:"activation_fence_digest"`

	// ActivationFenceGeneration corresponds to the JSON schema field
	// "activation_fence_generation".
	ActivationFenceGeneration ActivationFenceGenerationV1 `json:"activation_fence_generation"`

	// ActivationFenceID corresponds to the JSON schema field "activation_fence_id".
	ActivationFenceID ActivationFenceIDV1 `json:"activation_fence_id"`

	// CompiledGenerationDigest corresponds to the JSON schema field
	// "compiled_generation_digest".
	CompiledGenerationDigest sha256_digest_v1.Sha256DigestV1 `json:"compiled_generation_digest"`

	// PredecessorActivationFenceDigest corresponds to the JSON schema field
	// "predecessor_activation_fence_digest".
	PredecessorActivationFenceDigest *sha256_digest_v1.Sha256DigestV1 `json:"predecessor_activation_fence_digest,omitempty,omitzero"`

	// Schema corresponds to the JSON schema field "schema".
	Schema SchemaID `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type SchemaID string
