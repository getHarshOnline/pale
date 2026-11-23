// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package producer_incarnation_v1 contains the generated producer-incarnation.v1 data model.
package producer_incarnation_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type ProducerEpochV1 string

type ProducerIDV1 string

type ProducerIncarnationIDV1 string

type ProducerIncarnationV1 struct {
	// AllocatedAt corresponds to the JSON schema field "allocated_at".
	AllocatedAt UnixTimeV1 `json:"allocated_at"`

	// CanonicalIncarnationSha256 corresponds to the JSON schema field
	// "canonical_incarnation_sha256".
	CanonicalIncarnationSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_incarnation_sha256"`

	// IncarnationID corresponds to the JSON schema field "incarnation_id".
	IncarnationID ProducerIncarnationIDV1 `json:"incarnation_id"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// OriginPrincipalID corresponds to the JSON schema field "origin_principal_id".
	OriginPrincipalID ResidentPrincipalIDV1 `json:"origin_principal_id"`

	// PreviousIncarnationRef corresponds to the JSON schema field
	// "previous_incarnation_ref".
	PreviousIncarnationRef *ResidentRecordRefV1 `json:"previous_incarnation_ref,omitempty,omitzero"`

	// PreviousIncarnationSha256 corresponds to the JSON schema field
	// "previous_incarnation_sha256".
	PreviousIncarnationSha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_incarnation_sha256,omitempty,omitzero"`

	// ProducerEpoch corresponds to the JSON schema field "producer_epoch".
	ProducerEpoch ProducerEpochV1 `json:"producer_epoch"`

	// ProducerID corresponds to the JSON schema field "producer_id".
	ProducerID ProducerIDV1 `json:"producer_id"`

	// ProducerImplementationRef corresponds to the JSON schema field
	// "producer_implementation_ref".
	ProducerImplementationRef ResidentRecordRefV1 `json:"producer_implementation_ref"`

	// ProducerImplementationSha256 corresponds to the JSON schema field
	// "producer_implementation_sha256".
	ProducerImplementationSha256 sha256_digest_v1.Sha256DigestV1 `json:"producer_implementation_sha256"`

	// ProducerIncarnationAuthorityRef corresponds to the JSON schema field
	// "producer_incarnation_authority_ref".
	ProducerIncarnationAuthorityRef ResidentRecordRefV1 `json:"producer_incarnation_authority_ref"`

	// ProducerIncarnationAuthoritySha256 corresponds to the JSON schema field
	// "producer_incarnation_authority_sha256".
	ProducerIncarnationAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"producer_incarnation_authority_sha256"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`
}

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int
