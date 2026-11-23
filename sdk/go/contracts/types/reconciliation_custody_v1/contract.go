// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package reconciliation_custody_v1 contains the generated reconciliation-custody.v1 data model.
package reconciliation_custody_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type ReconciliationCustodyGenerationV1 int

type ReconciliationCustodyIDV1 string

type ReconciliationCustodyStateV1 string

const ReconciliationCustodyStateV1Active ReconciliationCustodyStateV1 = "active"
const ReconciliationCustodyStateV1Closed ReconciliationCustodyStateV1 = "closed"
const ReconciliationCustodyStateV1Transferred ReconciliationCustodyStateV1 = "transferred"

type ReconciliationCustodyV1 struct {
	// CanonicalCustodySha256 corresponds to the JSON schema field
	// "canonical_custody_sha256".
	CanonicalCustodySha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_custody_sha256"`

	// CaseRef corresponds to the JSON schema field "case_ref".
	CaseRef ResidentRecordRefV1 `json:"case_ref"`

	// CaseSha256 corresponds to the JSON schema field "case_sha256".
	CaseSha256 sha256_digest_v1.Sha256DigestV1 `json:"case_sha256"`

	// ConflictResourceKey corresponds to the JSON schema field
	// "conflict_resource_key".
	ConflictResourceKey any `json:"conflict_resource_key"`

	// ConflictResourceKeySchemaID corresponds to the JSON schema field
	// "conflict_resource_key_schema_id".
	ConflictResourceKeySchemaID SchemaID `json:"conflict_resource_key_schema_id"`

	// ConflictResourceKeySchemaSha256 corresponds to the JSON schema field
	// "conflict_resource_key_schema_sha256".
	ConflictResourceKeySchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"conflict_resource_key_schema_sha256"`

	// CustodyGeneration corresponds to the JSON schema field "custody_generation".
	CustodyGeneration ReconciliationCustodyGenerationV1 `json:"custody_generation"`

	// CustodyID corresponds to the JSON schema field "custody_id".
	CustodyID ReconciliationCustodyIDV1 `json:"custody_id"`

	// CustodyState corresponds to the JSON schema field "custody_state".
	CustodyState ReconciliationCustodyStateV1 `json:"custody_state"`

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`

	// HolderNodeID corresponds to the JSON schema field "holder_node_id".
	HolderNodeID *ResidentNodeIDV1 `json:"holder_node_id,omitempty,omitzero"`

	// HolderPrincipalID corresponds to the JSON schema field "holder_principal_id".
	HolderPrincipalID ResidentPrincipalIDV1 `json:"holder_principal_id"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// PreviousCustodyRef corresponds to the JSON schema field "previous_custody_ref".
	PreviousCustodyRef *ResidentRecordRefV1 `json:"previous_custody_ref,omitempty,omitzero"`

	// PreviousCustodySha256 corresponds to the JSON schema field
	// "previous_custody_sha256".
	PreviousCustodySha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_custody_sha256,omitempty,omitzero"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// TerminalResultRef corresponds to the JSON schema field "terminal_result_ref".
	TerminalResultRef *ResidentRecordRefV1 `json:"terminal_result_ref,omitempty,omitzero"`

	// TerminalResultSha256 corresponds to the JSON schema field
	// "terminal_result_sha256".
	TerminalResultSha256 *sha256_digest_v1.Sha256DigestV1 `json:"terminal_result_sha256,omitempty,omitzero"`

	// TransferredOrClosedAt corresponds to the JSON schema field
	// "transferred_or_closed_at".
	TransferredOrClosedAt *UnixTimeV1 `json:"transferred_or_closed_at,omitempty,omitzero"`
}

type ResidentKeyIDV1 string

type ResidentNodeIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int
