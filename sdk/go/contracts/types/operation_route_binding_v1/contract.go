// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package operation_route_binding_v1 contains the generated operation-route-binding.v1 data model.
package operation_route_binding_v1

import sealed_artifact_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sealed_artifact_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type ExecutionPathSelectionIDV1 string

type ExecutionPathSelectionV1 struct {
	// ExperiencePathID corresponds to the JSON schema field "experience_path_id".
	ExperiencePathID string `json:"experience_path_id"`

	// ID corresponds to the JSON schema field "id".
	ID ExecutionPathSelectionIDV1 `json:"id"`
}

type ExecutionPathWitnessV1 struct {
	// CompositionPlanDigest corresponds to the JSON schema field
	// "composition_plan_digest".
	CompositionPlanDigest sha256_digest_v1.Sha256DigestV1 `json:"composition_plan_digest"`

	// ResponsibilityPathIdentity corresponds to the JSON schema field
	// "responsibility_path_identity".
	ResponsibilityPathIdentity ResponsibilityPathIdentityV1 `json:"responsibility_path_identity"`

	// Selection corresponds to the JSON schema field "selection".
	Selection ExecutionPathSelectionV1 `json:"selection"`

	// TerminalProviderBinding corresponds to the JSON schema field
	// "terminal_provider_binding".
	TerminalProviderBinding *ProviderBindingRefV1 `json:"terminal_provider_binding,omitempty,omitzero"`

	// WorkDelegationEdgeIds corresponds to the JSON schema field
	// "work_delegation_edge_ids".
	WorkDelegationEdgeIds []string `json:"work_delegation_edge_ids"`
}

type LogicalRepositoryIDV1 string

type OperationRouteBindingV1 struct {
	// BindingDigest corresponds to the JSON schema field "binding_digest".
	BindingDigest sha256_digest_v1.Sha256DigestV1 `json:"binding_digest"`

	// Catalog corresponds to the JSON schema field "catalog".
	Catalog sealed_artifact_v1.SealedArtifactV1 `json:"catalog"`

	// CatalogDigest corresponds to the JSON schema field "catalog_digest".
	CatalogDigest sha256_digest_v1.Sha256DigestV1 `json:"catalog_digest"`

	// ExecutionPathWitness corresponds to the JSON schema field
	// "execution_path_witness".
	ExecutionPathWitness ExecutionPathWitnessV1 `json:"execution_path_witness"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
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

type ResponsibilityPathIdentityV1 struct {
	// ResponsibilityPathSha256 corresponds to the JSON schema field
	// "responsibility_path_sha256".
	ResponsibilityPathSha256 sha256_digest_v1.Sha256DigestV1 `json:"responsibility_path_sha256"`
}
