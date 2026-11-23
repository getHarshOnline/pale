// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package deployment_acceptance_revocation_v1 contains the generated deployment-acceptance-revocation.v1 data model.
package deployment_acceptance_revocation_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type DeploymentAcceptanceRevocationEpochV1 int

type DeploymentAcceptanceRevocationIDV1 string

type DeploymentAcceptanceRevocationV1 struct {
	// CanonicalDeploymentAcceptanceRevocationSha256 corresponds to the JSON schema
	// field "canonical_deployment_acceptance_revocation_sha256".
	CanonicalDeploymentAcceptanceRevocationSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_deployment_acceptance_revocation_sha256"`

	// DeploymentAcceptanceRef corresponds to the JSON schema field
	// "deployment_acceptance_ref".
	DeploymentAcceptanceRef ResidentRecordRefV1 `json:"deployment_acceptance_ref"`

	// DeploymentAcceptanceRevocationID corresponds to the JSON schema field
	// "deployment_acceptance_revocation_id".
	DeploymentAcceptanceRevocationID DeploymentAcceptanceRevocationIDV1 `json:"deployment_acceptance_revocation_id"`

	// DeploymentAcceptanceSha256 corresponds to the JSON schema field
	// "deployment_acceptance_sha256".
	DeploymentAcceptanceSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_acceptance_sha256"`

	// DeploymentTermsSha256 corresponds to the JSON schema field
	// "deployment_terms_sha256".
	DeploymentTermsSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_terms_sha256"`

	// EffectiveAt corresponds to the JSON schema field "effective_at".
	EffectiveAt UnixTimeV1 `json:"effective_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// PreviousRevocationEpoch corresponds to the JSON schema field
	// "previous_revocation_epoch".
	PreviousRevocationEpoch *DeploymentAcceptanceRevocationEpochV1 `json:"previous_revocation_epoch,omitempty,omitzero"`

	// PreviousRevocationRef corresponds to the JSON schema field
	// "previous_revocation_ref".
	PreviousRevocationRef *ResidentRecordRefV1 `json:"previous_revocation_ref,omitempty,omitzero"`

	// PreviousRevocationSha256 corresponds to the JSON schema field
	// "previous_revocation_sha256".
	PreviousRevocationSha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_revocation_sha256,omitempty,omitzero"`

	// ReasonCode corresponds to the JSON schema field "reason_code".
	ReasonCode ResidentReasonV1 `json:"reason_code"`

	// RevocationEpoch corresponds to the JSON schema field "revocation_epoch".
	RevocationEpoch DeploymentAcceptanceRevocationEpochV1 `json:"revocation_epoch"`

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
