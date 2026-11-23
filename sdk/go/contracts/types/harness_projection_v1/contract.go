// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package harness_projection_v1 contains the generated harness-projection.v1 data model.
package harness_projection_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import surface_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/surface_digest_v1"
import surface_ownership_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/surface_ownership_v1"

type HarnessProjectionV1 struct {
	// Artifacts corresponds to the JSON schema field "artifacts".
	Artifacts []surface_digest_v1.SurfaceDigestV1 `json:"artifacts"`

	// FrontDoor corresponds to the JSON schema field "front_door".
	FrontDoor ManagedFrontDoorDigest `json:"front_door"`

	// InstanceDigest corresponds to the JSON schema field "instance_digest".
	InstanceDigest sha256_digest_v1.Sha256DigestV1 `json:"instance_digest"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// ProviderBinding corresponds to the JSON schema field "provider_binding".
	ProviderBinding ProviderBindingRefV1 `json:"provider_binding"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type LogicalRepositoryIDV1 string

type ManagedFrontDoorDigest struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ManagedBlockSha256 corresponds to the JSON schema field "managed_block_sha256".
	ManagedBlockSha256 sha256_digest_v1.Sha256DigestV1 `json:"managed_block_sha256"`

	// Ownership corresponds to the JSON schema field "ownership".
	Ownership surface_ownership_v1.SurfaceOwnershipV1 `json:"ownership"`

	// RelPath corresponds to the JSON schema field "rel_path".
	RelPath string `json:"rel_path"`
}

type ProviderBindingIdentityV1 struct {
	// ProviderBindingSha256 corresponds to the JSON schema field
	// "provider_binding_sha256".
	ProviderBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"provider_binding_sha256"`
}

type ProviderBindingRefV1 struct {
	// ProviderBindingID corresponds to the JSON schema field "provider_binding_id".
	ProviderBindingID string `json:"provider_binding_id"`

	// ProviderBindingIdentity corresponds to the JSON schema field
	// "provider_binding_identity".
	ProviderBindingIdentity ProviderBindingIdentityV1 `json:"provider_binding_identity"`
}
