// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package principal_resident_writer_terminal_receipt_v1 contains the generated principal-resident-writer-terminal-receipt.v1 data model.
package principal_resident_writer_terminal_receipt_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type PrincipalKeyEpochV1 int

type PrincipalResidentWriterGenerationV1 int

type PrincipalResidentWriterTerminalReceiptIDV1 string

type PrincipalResidentWriterTerminalReceiptV1 struct {
	// CanonicalPrincipalResidentWriterTerminalReceiptSha256 corresponds to the JSON
	// schema field "canonical_principal_resident_writer_terminal_receipt_sha256".
	CanonicalPrincipalResidentWriterTerminalReceiptSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_principal_resident_writer_terminal_receipt_sha256"`

	// FinalOpenedStoreHeadsRootSha256 corresponds to the JSON schema field
	// "final_opened_store_heads_root_sha256".
	FinalOpenedStoreHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"final_opened_store_heads_root_sha256"`

	// FinalPendingCustodyRootSha256 corresponds to the JSON schema field
	// "final_pending_custody_root_sha256".
	FinalPendingCustodyRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"final_pending_custody_root_sha256"`

	// FinalTrustCheckpointRef corresponds to the JSON schema field
	// "final_trust_checkpoint_ref".
	FinalTrustCheckpointRef ResidentRecordRefV1 `json:"final_trust_checkpoint_ref"`

	// FinalTrustCheckpointSha256 corresponds to the JSON schema field
	// "final_trust_checkpoint_sha256".
	FinalTrustCheckpointSha256 sha256_digest_v1.Sha256DigestV1 `json:"final_trust_checkpoint_sha256"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch PrincipalKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// StoppedAt corresponds to the JSON schema field "stopped_at".
	StoppedAt UnixTimeV1 `json:"stopped_at"`

	// TerminalReceiptID corresponds to the JSON schema field "terminal_receipt_id".
	TerminalReceiptID PrincipalResidentWriterTerminalReceiptIDV1 `json:"terminal_receipt_id"`

	// WriterFenceRef corresponds to the JSON schema field "writer_fence_ref".
	WriterFenceRef ResidentRecordRefV1 `json:"writer_fence_ref"`

	// WriterFenceSha256 corresponds to the JSON schema field "writer_fence_sha256".
	WriterFenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"writer_fence_sha256"`

	// WriterGeneration corresponds to the JSON schema field "writer_generation".
	WriterGeneration PrincipalResidentWriterGenerationV1 `json:"writer_generation"`
}

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type UnixTimeV1 int
