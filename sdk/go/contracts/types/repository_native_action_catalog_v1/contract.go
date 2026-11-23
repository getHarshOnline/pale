// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package repository_native_action_catalog_v1 contains the generated repository-native-action-catalog.v1 data model.
package repository_native_action_catalog_v1

import action_contract_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/action_contract_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type LogicalRepositoryIDV1 string

type RepositoryNativeActionCatalogV1 struct {
	// Contracts corresponds to the JSON schema field "contracts".
	Contracts []action_contract_v1.ActionContractV1 `json:"contracts"`

	// InstanceDigest corresponds to the JSON schema field "instance_digest".
	InstanceDigest sha256_digest_v1.Sha256DigestV1 `json:"instance_digest"`

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
