// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package operation_denial_v1 contains the generated operation-denial.v1 data model.
package operation_denial_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type OperationDenialV1 struct {
	// Code corresponds to the JSON schema field "code".
	Code string `json:"code"`

	// Detail corresponds to the JSON schema field "detail".
	Detail OperationDenialV1Detail `json:"detail,omitempty,omitzero"`

	// Message corresponds to the JSON schema field "message".
	Message string `json:"message"`

	// Operation corresponds to the JSON schema field "operation".
	Operation OperationDenialV1Operation `json:"operation,omitempty,omitzero"`

	// Requirement corresponds to the JSON schema field "requirement".
	Requirement OperationDenialV1Requirement `json:"requirement,omitempty,omitzero"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// ScopeDigest corresponds to the JSON schema field "scope_digest".
	ScopeDigest *sha256_digest_v1.Sha256DigestV1 `json:"scope_digest,omitempty,omitzero"`

	// Tool corresponds to the JSON schema field "tool".
	Tool OperationDenialV1Tool `json:"tool,omitempty,omitzero"`
}

type OperationDenialV1Detail *string

type OperationDenialV1Operation *string

type OperationDenialV1Requirement *string

type OperationDenialV1Tool *string
