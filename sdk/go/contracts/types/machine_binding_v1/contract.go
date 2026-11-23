// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package machine_binding_v1 contains the generated machine-binding.v1 data model.
package machine_binding_v1

import native_harness_binding_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/native_harness_binding_v1"
import surface_executable_binding_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/surface_executable_binding_v1"

type DurableEvidenceBinding struct {
	// RetentionSeconds corresponds to the JSON schema field "retention_seconds".
	RetentionSeconds EvidenceRetentionSeconds `json:"retention_seconds"`

	// Root corresponds to the JSON schema field "root".
	Root string `json:"root"`
}

type EvidenceRetentionSeconds int

type LogicalRepositoryIDV1 string

type MachineBindingV1 struct {
	// AuthorityRoots corresponds to the JSON schema field "authority_roots".
	AuthorityRoots MachineBindingV1AuthorityRoots `json:"authority_roots"`

	// DurableEvidence corresponds to the JSON schema field "durable_evidence".
	DurableEvidence *DurableEvidenceBinding `json:"durable_evidence,omitempty,omitzero"`

	// Machine corresponds to the JSON schema field "machine".
	Machine MachineMeta `json:"machine"`

	// MaterializationLineage corresponds to the JSON schema field
	// "materialization_lineage".
	MaterializationLineage *MaterializationLineageBinding `json:"materialization_lineage,omitempty,omitzero"`

	// NativeHarnesses corresponds to the JSON schema field "native_harnesses".
	NativeHarnesses []native_harness_binding_v1.NativeHarnessBindingV1 `json:"native_harnesses,omitempty,omitzero"`

	// Preferences corresponds to the JSON schema field "preferences".
	Preferences *UserPreferences `json:"preferences,omitempty,omitzero"`

	// ProviderRepositories corresponds to the JSON schema field
	// "provider_repositories".
	ProviderRepositories []ProviderRepositoryProjectionV1 `json:"provider_repositories,omitempty,omitzero"`

	// RepositoryCheckouts corresponds to the JSON schema field
	// "repository_checkouts".
	RepositoryCheckouts []RepositoryCheckoutBindingV1 `json:"repository_checkouts,omitempty,omitzero"`

	// SessionState corresponds to the JSON schema field "session_state".
	SessionState *SessionStateBinding `json:"session_state,omitempty,omitzero"`

	// SurfaceExecutable corresponds to the JSON schema field "surface_executable".
	SurfaceExecutable *surface_executable_binding_v1.SurfaceExecutableBindingV1 `json:"surface_executable,omitempty,omitzero"`
}

type MachineBindingV1AuthorityRoots map[string]string

type MachineMeta struct {
	// DistributionRoot corresponds to the JSON schema field "distribution_root".
	DistributionRoot *string `json:"distribution_root,omitempty,omitzero"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

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

type RepositoryCheckoutBindingV1 struct {
	// CheckoutRoot corresponds to the JSON schema field "checkout_root".
	CheckoutRoot string `json:"checkout_root"`

	// GitMetadataRoot corresponds to the JSON schema field "git_metadata_root".
	GitMetadataRoot *string `json:"git_metadata_root,omitempty,omitzero"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`
}

type RouteRecordLimit int

type SessionStateBinding struct {
	// Root corresponds to the JSON schema field "root".
	Root string `json:"root"`

	// TtlSeconds corresponds to the JSON schema field "ttl_seconds".
	TtlSeconds SessionTtlSeconds `json:"ttl_seconds"`
}

type SessionTtlSeconds int

type UserPreferences struct {
	// ModeDefaults corresponds to the JSON schema field "mode_defaults".
	ModeDefaults UserPreferencesModeDefaults `json:"mode_defaults,omitempty,omitzero"`
}

type UserPreferencesModeDefaults map[string]PackageResourceRefV1
