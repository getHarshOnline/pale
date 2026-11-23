// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package action_execution_v1 contains the generated action-execution.v1 data model.
package action_execution_v1

import action_input_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/action_input_v1"
import action_output_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/action_output_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type ActionEffect string

const ActionEffectCompute ActionEffect = "compute"

type ActionExecutionV1 struct {
	// Receipt corresponds to the JSON schema field "receipt".
	Receipt ActionReceipt `json:"receipt"`

	// Request corresponds to the JSON schema field "request".
	Request ActionRequest `json:"request"`

	// Result corresponds to the JSON schema field "result".
	Result ActionResult `json:"result"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type ActionReceipt struct {
	// ActionID corresponds to the JSON schema field "action_id".
	ActionID string `json:"action_id"`

	// ActionVersion corresponds to the JSON schema field "action_version".
	ActionVersion int `json:"action_version"`

	// ContractDigest corresponds to the JSON schema field "contract_digest".
	ContractDigest sha256_digest_v1.Sha256DigestV1 `json:"contract_digest"`

	// ReceiptDigest corresponds to the JSON schema field "receipt_digest".
	ReceiptDigest sha256_digest_v1.Sha256DigestV1 `json:"receipt_digest"`

	// RequestDigest corresponds to the JSON schema field "request_digest".
	RequestDigest sha256_digest_v1.Sha256DigestV1 `json:"request_digest"`

	// ResultDigest corresponds to the JSON schema field "result_digest".
	ResultDigest sha256_digest_v1.Sha256DigestV1 `json:"result_digest"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type ActionRequest struct {
	// ActionID corresponds to the JSON schema field "action_id".
	ActionID string `json:"action_id"`

	// ActionVersion corresponds to the JSON schema field "action_version".
	ActionVersion int `json:"action_version"`

	// ContractDigest corresponds to the JSON schema field "contract_digest".
	ContractDigest sha256_digest_v1.Sha256DigestV1 `json:"contract_digest"`

	// Effect corresponds to the JSON schema field "effect".
	Effect ActionEffect `json:"effect"`

	// Input corresponds to the JSON schema field "input".
	Input action_input_v1.ActionInputV1 `json:"input"`

	// RequestDigest corresponds to the JSON schema field "request_digest".
	RequestDigest sha256_digest_v1.Sha256DigestV1 `json:"request_digest"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type ActionResult struct {
	// ActionID corresponds to the JSON schema field "action_id".
	ActionID string `json:"action_id"`

	// ActionVersion corresponds to the JSON schema field "action_version".
	ActionVersion int `json:"action_version"`

	// ContractDigest corresponds to the JSON schema field "contract_digest".
	ContractDigest sha256_digest_v1.Sha256DigestV1 `json:"contract_digest"`

	// Output corresponds to the JSON schema field "output".
	Output action_output_v1.ActionOutputV1 `json:"output"`

	// RequestDigest corresponds to the JSON schema field "request_digest".
	RequestDigest sha256_digest_v1.Sha256DigestV1 `json:"request_digest"`

	// ResultDigest corresponds to the JSON schema field "result_digest".
	ResultDigest sha256_digest_v1.Sha256DigestV1 `json:"result_digest"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}
