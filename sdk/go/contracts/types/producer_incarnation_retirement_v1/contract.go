// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package producer_incarnation_retirement_v1 contains the generated producer-incarnation-retirement.v1 data model.
package producer_incarnation_retirement_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type ProducerEpochV1 string

type ProducerIDV1 string

type ProducerIncarnationRetirementIDV1 string

type ProducerIncarnationRetirementV1 struct {
	// CanonicalRetirementSha256 corresponds to the JSON schema field
	// "canonical_retirement_sha256".
	CanonicalRetirementSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_retirement_sha256"`

	// IncarnationRef corresponds to the JSON schema field "incarnation_ref".
	IncarnationRef ResidentRecordRefV1 `json:"incarnation_ref"`

	// IncarnationSha256 corresponds to the JSON schema field "incarnation_sha256".
	IncarnationSha256 sha256_digest_v1.Sha256DigestV1 `json:"incarnation_sha256"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// OriginPrincipalID corresponds to the JSON schema field "origin_principal_id".
	OriginPrincipalID ResidentPrincipalIDV1 `json:"origin_principal_id"`

	// ProducerEpoch corresponds to the JSON schema field "producer_epoch".
	ProducerEpoch ProducerEpochV1 `json:"producer_epoch"`

	// ProducerID corresponds to the JSON schema field "producer_id".
	ProducerID ProducerIDV1 `json:"producer_id"`

	// ProducerIncarnationAuthorityRef corresponds to the JSON schema field
	// "producer_incarnation_authority_ref".
	ProducerIncarnationAuthorityRef ResidentRecordRefV1 `json:"producer_incarnation_authority_ref"`

	// ProducerIncarnationAuthoritySha256 corresponds to the JSON schema field
	// "producer_incarnation_authority_sha256".
	ProducerIncarnationAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"producer_incarnation_authority_sha256"`

	// ReasonCode corresponds to the JSON schema field "reason_code".
	ReasonCode ResidentReasonV1 `json:"reason_code"`

	// RetiredAt corresponds to the JSON schema field "retired_at".
	RetiredAt UnixTimeV1 `json:"retired_at"`

	// RetirementID corresponds to the JSON schema field "retirement_id".
	RetirementID ProducerIncarnationRetirementIDV1 `json:"retirement_id"`

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

type SigningKeyEpochV1 int

type UnixTimeV1 int
