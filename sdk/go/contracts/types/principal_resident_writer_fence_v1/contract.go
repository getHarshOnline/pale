// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package principal_resident_writer_fence_v1 contains the generated principal-resident-writer-fence.v1 data model.
package principal_resident_writer_fence_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type ControlIncarnationIDV1 string

type ControlStoreIDV1 string

type PrincipalKeyEpochV1 int

type PrincipalResidentWriterFenceIDV1 string

type PrincipalResidentWriterFenceStateV1 string

const PrincipalResidentWriterFenceStateV1Active PrincipalResidentWriterFenceStateV1 = "active"
const PrincipalResidentWriterFenceStateV1Retired PrincipalResidentWriterFenceStateV1 = "retired"

type PrincipalResidentWriterFenceV1 struct {
	// AcquiredAt corresponds to the JSON schema field "acquired_at".
	AcquiredAt UnixTimeV1 `json:"acquired_at"`

	// BootIncarnationID corresponds to the JSON schema field "boot_incarnation_id".
	BootIncarnationID ResidentBootIncarnationIDV1 `json:"boot_incarnation_id"`

	// CanonicalPrincipalResidentWriterFenceSha256 corresponds to the JSON schema
	// field "canonical_principal_resident_writer_fence_sha256".
	CanonicalPrincipalResidentWriterFenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_principal_resident_writer_fence_sha256"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch PrincipalKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// OpenedDeploymentStoreHeadsRootSha256 corresponds to the JSON schema field
	// "opened_deployment_store_heads_root_sha256".
	OpenedDeploymentStoreHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"opened_deployment_store_heads_root_sha256"`

	// PreviousWriterFenceRef corresponds to the JSON schema field
	// "previous_writer_fence_ref".
	PreviousWriterFenceRef *ResidentRecordRefV1 `json:"previous_writer_fence_ref,omitempty,omitzero"`

	// PreviousWriterFenceSha256 corresponds to the JSON schema field
	// "previous_writer_fence_sha256".
	PreviousWriterFenceSha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_writer_fence_sha256,omitempty,omitzero"`

	// PrimaryNodeID corresponds to the JSON schema field "primary_node_id".
	PrimaryNodeID ResidentNodeIDV1 `json:"primary_node_id"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// PrincipalSessionFloorEpoch corresponds to the JSON schema field
	// "principal_session_floor_epoch".
	PrincipalSessionFloorEpoch PrincipalSessionFloorEpochV1 `json:"principal_session_floor_epoch"`

	// PriorTerminalReceiptRef corresponds to the JSON schema field
	// "prior_terminal_receipt_ref".
	PriorTerminalReceiptRef *ResidentRecordRefV1 `json:"prior_terminal_receipt_ref,omitempty,omitzero"`

	// PriorTerminalReceiptSha256 corresponds to the JSON schema field
	// "prior_terminal_receipt_sha256".
	PriorTerminalReceiptSha256 *sha256_digest_v1.Sha256DigestV1 `json:"prior_terminal_receipt_sha256,omitempty,omitzero"`

	// ProcessInstanceID corresponds to the JSON schema field "process_instance_id".
	ProcessInstanceID ResidentProcessInstanceIDV1 `json:"process_instance_id"`

	// RetiredAt corresponds to the JSON schema field "retired_at".
	RetiredAt *UnixTimeV1 `json:"retired_at,omitempty,omitzero"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// State corresponds to the JSON schema field "state".
	State PrincipalResidentWriterFenceStateV1 `json:"state"`

	// TakeoverReceiptRef corresponds to the JSON schema field "takeover_receipt_ref".
	TakeoverReceiptRef *ResidentRecordRefV1 `json:"takeover_receipt_ref,omitempty,omitzero"`

	// TakeoverReceiptSha256 corresponds to the JSON schema field
	// "takeover_receipt_sha256".
	TakeoverReceiptSha256 *sha256_digest_v1.Sha256DigestV1 `json:"takeover_receipt_sha256,omitempty,omitzero"`

	// TrustStoreID corresponds to the JSON schema field "trust_store_id".
	TrustStoreID ControlStoreIDV1 `json:"trust_store_id"`

	// TrustStoreIncarnationID corresponds to the JSON schema field
	// "trust_store_incarnation_id".
	TrustStoreIncarnationID ControlIncarnationIDV1 `json:"trust_store_incarnation_id"`

	// WriterFenceID corresponds to the JSON schema field "writer_fence_id".
	WriterFenceID PrincipalResidentWriterFenceIDV1 `json:"writer_fence_id"`

	// WriterGeneration corresponds to the JSON schema field "writer_generation".
	WriterGeneration PrincipalResidentWriterGenerationV1 `json:"writer_generation"`
}

type PrincipalResidentWriterGenerationV1 int

type PrincipalSessionFloorEpochV1 int

type ResidentBootIncarnationIDV1 string

type ResidentKeyIDV1 string

type ResidentNodeIDV1 string

type ResidentPrincipalIDV1 string

type ResidentProcessInstanceIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type UnixTimeV1 int
