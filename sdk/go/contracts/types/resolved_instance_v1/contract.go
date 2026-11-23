// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package resolved_instance_v1 contains the generated resolved-instance.v1 data model.
package resolved_instance_v1

import native_harness_binding_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/native_harness_binding_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import surface_executable_binding_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/surface_executable_binding_v1"

type DurableEvidenceBinding struct {
	// RetentionSeconds corresponds to the JSON schema field "retention_seconds".
	RetentionSeconds EvidenceRetentionSeconds `json:"retention_seconds"`

	// Root corresponds to the JSON schema field "root".
	Root string `json:"root"`
}

type EvidenceRetentionSeconds int

type LogicalRepositoryIDV1 string

type MaterializationLineageBinding struct {
	// MaxRouteRecordsPerCollection corresponds to the JSON schema field
	// "max_route_records_per_collection".
	MaxRouteRecordsPerCollection RouteRecordLimit `json:"max_route_records_per_collection"`

	// Root corresponds to the JSON schema field "root".
	Root string `json:"root"`
}

type PackageIDV1 string

type PackageResourceIDV1 string

type PackageResourceRefV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`
}

type ProviderRepositoryProjectionV1 struct {
	// NativeRepositoryCoordinate corresponds to the JSON schema field
	// "native_repository_coordinate".
	NativeRepositoryCoordinate string `json:"native_repository_coordinate"`

	// ProviderBindingID corresponds to the JSON schema field "provider_binding_id".
	ProviderBindingID string `json:"provider_binding_id"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`
}

type ResolvedInstanceV1 struct {
	// AuthorityRoots corresponds to the JSON schema field "authority_roots".
	AuthorityRoots ResolvedInstanceV1AuthorityRoots `json:"authority_roots"`

	// DurableEvidence corresponds to the JSON schema field "durable_evidence".
	DurableEvidence *DurableEvidenceBinding `json:"durable_evidence,omitempty,omitzero"`

	// GitMetadataRoots corresponds to the JSON schema field "git_metadata_roots".
	GitMetadataRoots ResolvedInstanceV1GitMetadataRoots `json:"git_metadata_roots,omitempty,omitzero"`

	// InstanceDigest corresponds to the JSON schema field "instance_digest".
	InstanceDigest sha256_digest_v1.Sha256DigestV1 `json:"instance_digest"`

	// MachineID corresponds to the JSON schema field "machine_id".
	MachineID string `json:"machine_id"`

	// MaterializationLineage corresponds to the JSON schema field
	// "materialization_lineage".
	MaterializationLineage *MaterializationLineageBinding `json:"materialization_lineage,omitempty,omitzero"`

	// NativeHarnesses corresponds to the JSON schema field "native_harnesses".
	NativeHarnesses []native_harness_binding_v1.NativeHarnessBindingV1 `json:"native_harnesses,omitempty,omitzero"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// ProviderRepositories corresponds to the JSON schema field
	// "provider_repositories".
	ProviderRepositories []ProviderRepositoryProjectionV1 `json:"provider_repositories,omitempty,omitzero"`

	// Repositories corresponds to the JSON schema field "repositories".
	Repositories ResolvedInstanceV1Repositories `json:"repositories"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SessionState corresponds to the JSON schema field "session_state".
	SessionState *SessionStateBinding `json:"session_state,omitempty,omitzero"`

	// SurfaceExecutable corresponds to the JSON schema field "surface_executable".
	SurfaceExecutable *surface_executable_binding_v1.SurfaceExecutableBindingV1 `json:"surface_executable,omitempty,omitzero"`

	// UserModeDefaults corresponds to the JSON schema field "user_mode_defaults".
	UserModeDefaults ResolvedInstanceV1UserModeDefaults `json:"user_mode_defaults,omitempty,omitzero"`
}

type ResolvedInstanceV1AuthorityRoots map[string]string

type ResolvedInstanceV1GitMetadataRoots map[string]string

type ResolvedInstanceV1Repositories map[string]string

type ResolvedInstanceV1UserModeDefaults map[string]PackageResourceRefV1

type RouteRecordLimit int

type SessionStateBinding struct {
	// Root corresponds to the JSON schema field "root".
	Root string `json:"root"`

	// TtlSeconds corresponds to the JSON schema field "ttl_seconds".
	TtlSeconds SessionTtlSeconds `json:"ttl_seconds"`
}

type SessionTtlSeconds int
