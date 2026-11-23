// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package generated_manifest_v1 contains the generated generated-manifest.v1 data model.
package generated_manifest_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import surface_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/surface_digest_v1"

type GeneratedManifestV1 struct {
	// CompilationInputDigest corresponds to the JSON schema field
	// "compilation_input_digest".
	CompilationInputDigest sha256_digest_v1.Sha256DigestV1 `json:"compilation_input_digest"`

	// CompilerVersion corresponds to the JSON schema field "compiler_version".
	CompilerVersion string `json:"compiler_version"`

	// ExecutionWorkspaceID corresponds to the JSON schema field
	// "execution_workspace_id".
	ExecutionWorkspaceID LogicalExecutionWorkspaceIDV1 `json:"execution_workspace_id"`

	// InstanceDigest corresponds to the JSON schema field "instance_digest".
	InstanceDigest sha256_digest_v1.Sha256DigestV1 `json:"instance_digest"`

	// ManifestDigest corresponds to the JSON schema field "manifest_digest".
	ManifestDigest sha256_digest_v1.Sha256DigestV1 `json:"manifest_digest"`

	// Outputs corresponds to the JSON schema field "outputs".
	Outputs []surface_digest_v1.SurfaceDigestV1 `json:"outputs"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// ProjectID corresponds to the JSON schema field "project_id".
	ProjectID LogicalProjectIDV1 `json:"project_id"`

	// RendererVersion corresponds to the JSON schema field "renderer_version".
	RendererVersion string `json:"renderer_version"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaManifestSha256 corresponds to the JSON schema field
	// "schema_manifest_sha256".
	SchemaManifestSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_manifest_sha256"`

	// SchemaSetSha256 corresponds to the JSON schema field "schema_set_sha256".
	SchemaSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_set_sha256"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`
}

type LogicalExecutionWorkspaceIDV1 string

type LogicalProjectIDV1 string

type LogicalRepositoryIDV1 string
