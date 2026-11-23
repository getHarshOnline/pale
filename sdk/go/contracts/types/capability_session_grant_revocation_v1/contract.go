// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package capability_session_grant_revocation_v1 contains the generated capability-session-grant-revocation.v1 data model.
package capability_session_grant_revocation_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type CapabilityGrantRevocationEpochV1 int

type CapabilityGrantRevocationIDV1 string

type CapabilitySessionGrantRevocationV1 struct {
	// CanonicalRevocationSha256 corresponds to the JSON schema field
	// "canonical_revocation_sha256".
	CanonicalRevocationSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_revocation_sha256"`

	// EffectiveAt corresponds to the JSON schema field "effective_at".
	EffectiveAt UnixTimeV1 `json:"effective_at"`

	// GrantID corresponds to the JSON schema field "grant_id".
	GrantID SessionGrantIDV1 `json:"grant_id"`

	// GrantRef corresponds to the JSON schema field "grant_ref".
	GrantRef ResidentRecordRefV1 `json:"grant_ref"`

	// GrantSha256 corresponds to the JSON schema field "grant_sha256".
	GrantSha256 sha256_digest_v1.Sha256DigestV1 `json:"grant_sha256"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// PreviousRevocationEpoch corresponds to the JSON schema field
	// "previous_revocation_epoch".
	PreviousRevocationEpoch *CapabilityGrantRevocationEpochV1 `json:"previous_revocation_epoch,omitempty,omitzero"`

	// PreviousRevocationRef corresponds to the JSON schema field
	// "previous_revocation_ref".
	PreviousRevocationRef *ResidentRecordRefV1 `json:"previous_revocation_ref,omitempty,omitzero"`

	// PreviousRevocationSha256 corresponds to the JSON schema field
	// "previous_revocation_sha256".
	PreviousRevocationSha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_revocation_sha256,omitempty,omitzero"`

	// ReasonCode corresponds to the JSON schema field "reason_code".
	ReasonCode ResidentReasonV1 `json:"reason_code"`

	// RevocationEpoch corresponds to the JSON schema field "revocation_epoch".
	RevocationEpoch CapabilityGrantRevocationEpochV1 `json:"revocation_epoch"`

	// RevocationID corresponds to the JSON schema field "revocation_id".
	RevocationID CapabilityGrantRevocationIDV1 `json:"revocation_id"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`
}

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentReasonV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SessionGrantIDV1 string

type SigningKeyEpochV1 int

type UnixTimeV1 int
