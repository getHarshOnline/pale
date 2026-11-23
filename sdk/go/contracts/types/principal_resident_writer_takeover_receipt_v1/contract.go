// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package principal_resident_writer_takeover_receipt_v1 contains the generated principal-resident-writer-takeover-receipt.v1 data model.
package principal_resident_writer_takeover_receipt_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type PrincipalKeyEpochV1 int

type PrincipalResidentWriterGenerationV1 int

type PrincipalResidentWriterTakeoverNonceV1 string

type PrincipalResidentWriterTakeoverReceiptIDV1 string

type PrincipalResidentWriterTakeoverReceiptV1 struct {
	// AcceptedTrustCheckpointRef corresponds to the JSON schema field
	// "accepted_trust_checkpoint_ref".
	AcceptedTrustCheckpointRef ResidentRecordRefV1 `json:"accepted_trust_checkpoint_ref"`

	// AcceptedTrustCheckpointSha256 corresponds to the JSON schema field
	// "accepted_trust_checkpoint_sha256".
	AcceptedTrustCheckpointSha256 sha256_digest_v1.Sha256DigestV1 `json:"accepted_trust_checkpoint_sha256"`

	// AuthorizedAt corresponds to the JSON schema field "authorized_at".
	AuthorizedAt UnixTimeV1 `json:"authorized_at"`

	// CanonicalPrincipalResidentWriterTakeoverReceiptSha256 corresponds to the JSON
	// schema field "canonical_principal_resident_writer_takeover_receipt_sha256".
	CanonicalPrincipalResidentWriterTakeoverReceiptSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_principal_resident_writer_takeover_receipt_sha256"`

	// InvalidatedRouteLeaseHeadsRootSha256 corresponds to the JSON schema field
	// "invalidated_route_lease_heads_root_sha256".
	InvalidatedRouteLeaseHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"invalidated_route_lease_heads_root_sha256"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch PrincipalKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// NewSessionFloorEpoch corresponds to the JSON schema field
	// "new_session_floor_epoch".
	NewSessionFloorEpoch PrincipalSessionFloorEpochV1 `json:"new_session_floor_epoch"`

	// PredecessorNonLivenessEvidenceRootSha256 corresponds to the JSON schema field
	// "predecessor_non_liveness_evidence_root_sha256".
	PredecessorNonLivenessEvidenceRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"predecessor_non_liveness_evidence_root_sha256"`

	// PredecessorWriterFenceRef corresponds to the JSON schema field
	// "predecessor_writer_fence_ref".
	PredecessorWriterFenceRef ResidentRecordRefV1 `json:"predecessor_writer_fence_ref"`

	// PredecessorWriterFenceSha256 corresponds to the JSON schema field
	// "predecessor_writer_fence_sha256".
	PredecessorWriterFenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"predecessor_writer_fence_sha256"`

	// PredecessorWriterGeneration corresponds to the JSON schema field
	// "predecessor_writer_generation".
	PredecessorWriterGeneration PrincipalResidentWriterGenerationV1 `json:"predecessor_writer_generation"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// PriorSessionFloorEpoch corresponds to the JSON schema field
	// "prior_session_floor_epoch".
	PriorSessionFloorEpoch PrincipalSessionFloorEpochV1 `json:"prior_session_floor_epoch"`

	// QuarantinedCustodyRootSha256 corresponds to the JSON schema field
	// "quarantined_custody_root_sha256".
	QuarantinedCustodyRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"quarantined_custody_root_sha256"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// SuccessorBootIncarnationID corresponds to the JSON schema field
	// "successor_boot_incarnation_id".
	SuccessorBootIncarnationID ResidentBootIncarnationIDV1 `json:"successor_boot_incarnation_id"`

	// SuccessorPrimaryNodeID corresponds to the JSON schema field
	// "successor_primary_node_id".
	SuccessorPrimaryNodeID ResidentNodeIDV1 `json:"successor_primary_node_id"`

	// SuccessorProcessInstanceID corresponds to the JSON schema field
	// "successor_process_instance_id".
	SuccessorProcessInstanceID ResidentProcessInstanceIDV1 `json:"successor_process_instance_id"`

	// TakeoverNonce corresponds to the JSON schema field "takeover_nonce".
	TakeoverNonce PrincipalResidentWriterTakeoverNonceV1 `json:"takeover_nonce"`

	// TakeoverReceiptID corresponds to the JSON schema field "takeover_receipt_id".
	TakeoverReceiptID PrincipalResidentWriterTakeoverReceiptIDV1 `json:"takeover_receipt_id"`

	// VerifiedStoreHeadsRootSha256 corresponds to the JSON schema field
	// "verified_store_heads_root_sha256".
	VerifiedStoreHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"verified_store_heads_root_sha256"`
}

type PrincipalSessionFloorEpochV1 int

type ResidentBootIncarnationIDV1 string

type ResidentKeyIDV1 string

type ResidentNodeIDV1 string

type ResidentPrincipalIDV1 string

type ResidentProcessInstanceIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type UnixTimeV1 int
