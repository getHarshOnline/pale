// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package workspace_resolution_lock_v1 contains the generated workspace-resolution-lock.v1 data model.
package workspace_resolution_lock_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type WorkspaceModuleID string

type WorkspacePublisherID string

type WorkspacePublisherKeyID string

type WorkspacePublisherRefV1 struct {
	// DelegatedReleaseKeyID corresponds to the JSON schema field
	// "delegated_release_key_id".
	DelegatedReleaseKeyID WorkspacePublisherKeyID `json:"delegated_release_key_id"`

	// PublisherID corresponds to the JSON schema field "publisher_id".
	PublisherID WorkspacePublisherID `json:"publisher_id"`

	// RecoveryAuthorityID corresponds to the JSON schema field
	// "recovery_authority_id".
	RecoveryAuthorityID WorkspaceRecoveryAuthorityID `json:"recovery_authority_id"`

	// RootKeyID corresponds to the JSON schema field "root_key_id".
	RootKeyID WorkspacePublisherKeyID `json:"root_key_id"`
}

type WorkspaceRecoveryAuthorityID string

type WorkspaceReleaseID string

type WorkspaceReleaseStateV1 string

const WorkspaceReleaseStateV1Candidate WorkspaceReleaseStateV1 = "candidate"
const WorkspaceReleaseStateV1Published WorkspaceReleaseStateV1 = "published"
const WorkspaceReleaseStateV1Revoked WorkspaceReleaseStateV1 = "revoked"

type WorkspaceResolutionLockV1 struct {
	// Releases corresponds to the JSON schema field "releases".
	Releases []WorkspaceResolvedReleaseV1 `json:"releases"`

	// ResolutionDigest corresponds to the JSON schema field "resolution_digest".
	ResolutionDigest sha256_digest_v1.Sha256DigestV1 `json:"resolution_digest"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type WorkspaceResolvedReleaseV1 struct {
	// ModuleDigest corresponds to the JSON schema field "module_digest".
	ModuleDigest sha256_digest_v1.Sha256DigestV1 `json:"module_digest"`

	// ModuleID corresponds to the JSON schema field "module_id".
	ModuleID WorkspaceModuleID `json:"module_id"`

	// Publisher corresponds to the JSON schema field "publisher".
	Publisher WorkspacePublisherRefV1 `json:"publisher"`

	// ReleaseID corresponds to the JSON schema field "release_id".
	ReleaseID WorkspaceReleaseID `json:"release_id"`

	// ReleaseState corresponds to the JSON schema field "release_state".
	ReleaseState WorkspaceReleaseStateV1 `json:"release_state"`
}
