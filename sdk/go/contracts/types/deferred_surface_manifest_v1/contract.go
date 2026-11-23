// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package deferred_surface_manifest_v1 contains the generated deferred-surface-manifest.v1 data model.
package deferred_surface_manifest_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type DeferredArtifactV1 struct {
	// Activation corresponds to the JSON schema field "activation".
	Activation string `json:"activation"`

	// ContentSha256 corresponds to the JSON schema field "content_sha256".
	ContentSha256 sha256_digest_v1.Sha256DigestV1 `json:"content_sha256"`

	// InertPath corresponds to the JSON schema field "inert_path".
	InertPath string `json:"inert_path"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Target corresponds to the JSON schema field "target".
	Target string `json:"target"`
}

type DeferredSurfaceManifestV1 struct {
	// Artifacts corresponds to the JSON schema field "artifacts".
	Artifacts []DeferredArtifactV1 `json:"artifacts"`

	// CompilationInputDigest corresponds to the JSON schema field
	// "compilation_input_digest".
	CompilationInputDigest sha256_digest_v1.Sha256DigestV1 `json:"compilation_input_digest"`

	// OneShotCompilationDigest corresponds to the JSON schema field
	// "one_shot_compilation_digest".
	OneShotCompilationDigest sha256_digest_v1.Sha256DigestV1 `json:"one_shot_compilation_digest"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`

	// RepositoryManifestDigest corresponds to the JSON schema field
	// "repository_manifest_digest".
	RepositoryManifestDigest sha256_digest_v1.Sha256DigestV1 `json:"repository_manifest_digest"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaManifestSha256 corresponds to the JSON schema field
	// "schema_manifest_sha256".
	SchemaManifestSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_manifest_sha256"`

	// SchemaSetSha256 corresponds to the JSON schema field "schema_set_sha256".
	SchemaSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_set_sha256"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type LogicalRepositoryIDV1 string
