// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package materialization_lineage_v1 contains the generated materialization-lineage.v1 data model.
package materialization_lineage_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type LogicalRepositoryIDV1 string

type MaterializationLineageV1 struct {
	// AdmittedPlanDigest corresponds to the JSON schema field "admitted_plan_digest".
	AdmittedPlanDigest sha256_digest_v1.Sha256DigestV1 `json:"admitted_plan_digest"`

	// InstanceDigest corresponds to the JSON schema field "instance_digest".
	InstanceDigest sha256_digest_v1.Sha256DigestV1 `json:"instance_digest"`

	// LineageDigest corresponds to the JSON schema field "lineage_digest".
	LineageDigest sha256_digest_v1.Sha256DigestV1 `json:"lineage_digest"`

	// ManifestDigest corresponds to the JSON schema field "manifest_digest".
	ManifestDigest sha256_digest_v1.Sha256DigestV1 `json:"manifest_digest"`

	// OneShotCompilationDigest corresponds to the JSON schema field
	// "one_shot_compilation_digest".
	OneShotCompilationDigest sha256_digest_v1.Sha256DigestV1 `json:"one_shot_compilation_digest"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

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

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// StateDigest corresponds to the JSON schema field "state_digest".
	StateDigest sha256_digest_v1.Sha256DigestV1 `json:"state_digest"`

	// TargetRootSha256 corresponds to the JSON schema field "target_root_sha256".
	TargetRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"target_root_sha256"`
}
