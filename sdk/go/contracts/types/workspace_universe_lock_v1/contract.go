// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package workspace_universe_lock_v1 contains the generated workspace-universe-lock.v1 data model.
package workspace_universe_lock_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type WorkspaceAdoptedReleaseV1 struct {
	// ModuleDigest corresponds to the JSON schema field "module_digest".
	ModuleDigest sha256_digest_v1.Sha256DigestV1 `json:"module_digest"`

	// ModuleID corresponds to the JSON schema field "module_id".
	ModuleID WorkspaceModuleID `json:"module_id"`

	// ReleaseID corresponds to the JSON schema field "release_id".
	ReleaseID WorkspaceReleaseID `json:"release_id"`
}

type WorkspaceID string

type WorkspaceModuleID string

type WorkspaceReleaseID string

type WorkspaceUniverseLockV1 struct {
	// AdoptedReleases corresponds to the JSON schema field "adopted_releases".
	AdoptedReleases []WorkspaceAdoptedReleaseV1 `json:"adopted_releases"`

	// ResolutionDigest corresponds to the JSON schema field "resolution_digest".
	ResolutionDigest sha256_digest_v1.Sha256DigestV1 `json:"resolution_digest"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// UniverseDigest corresponds to the JSON schema field "universe_digest".
	UniverseDigest sha256_digest_v1.Sha256DigestV1 `json:"universe_digest"`

	// WorkspaceID corresponds to the JSON schema field "workspace_id".
	WorkspaceID WorkspaceID `json:"workspace_id"`
}
