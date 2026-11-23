// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package contract_set_v1 contains the generated contract-set.v1 data model.
package contract_set_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type ContractSetEntryV1 struct {
	// ContractID corresponds to the JSON schema field "contract_id".
	ContractID SchemaID `json:"contract_id"`

	// ContractSchemaRef corresponds to the JSON schema field "contract_schema_ref".
	ContractSchemaRef ResidentRecordRefV1 `json:"contract_schema_ref"`

	// ContractSchemaSha256 corresponds to the JSON schema field
	// "contract_schema_sha256".
	ContractSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"contract_schema_sha256"`
}

type ContractSetIDV1 string

type ContractSetV1 struct {
	// CanonicalContractSetSha256 corresponds to the JSON schema field
	// "canonical_contract_set_sha256".
	CanonicalContractSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_contract_set_sha256"`

	// CompatibilityProfileRef corresponds to the JSON schema field
	// "compatibility_profile_ref".
	CompatibilityProfileRef ResidentRecordRefV1 `json:"compatibility_profile_ref"`

	// CompatibilityProfileSha256 corresponds to the JSON schema field
	// "compatibility_profile_sha256".
	CompatibilityProfileSha256 sha256_digest_v1.Sha256DigestV1 `json:"compatibility_profile_sha256"`

	// ContractSetID corresponds to the JSON schema field "contract_set_id".
	ContractSetID ContractSetIDV1 `json:"contract_set_id"`

	// Entries corresponds to the JSON schema field "entries".
	Entries []ContractSetEntryV1 `json:"entries"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID string `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`
}

type ResidentRecordRefV1 string

type SchemaID string
