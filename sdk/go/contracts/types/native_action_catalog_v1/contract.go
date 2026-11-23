// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package native_action_catalog_v1 contains the generated native-action-catalog.v1 data model.
package native_action_catalog_v1

import action_contract_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/action_contract_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type NativeActionCatalogV1 struct {
	// Contracts corresponds to the JSON schema field "contracts".
	Contracts []action_contract_v1.ActionContractV1 `json:"contracts"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}
