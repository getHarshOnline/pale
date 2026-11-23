// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package deployment_binding_v1 contains the generated deployment-binding.v1 data model.
package deployment_binding_v1

import deployment_terms_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/deployment_terms_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type DeploymentBindingIDV1 string

type DeploymentBindingV1 struct {
	// CanonicalBindingSha256 corresponds to the JSON schema field
	// "canonical_binding_sha256".
	CanonicalBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_binding_sha256"`

	// DeploymentAcceptanceRef corresponds to the JSON schema field
	// "deployment_acceptance_ref".
	DeploymentAcceptanceRef ResidentRecordRefV1 `json:"deployment_acceptance_ref"`

	// DeploymentAcceptanceSha256 corresponds to the JSON schema field
	// "deployment_acceptance_sha256".
	DeploymentAcceptanceSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_acceptance_sha256"`

	// DeploymentBindingID corresponds to the JSON schema field
	// "deployment_binding_id".
	DeploymentBindingID DeploymentBindingIDV1 `json:"deployment_binding_id"`

	// DeploymentProposalRef corresponds to the JSON schema field
	// "deployment_proposal_ref".
	DeploymentProposalRef ResidentRecordRefV1 `json:"deployment_proposal_ref"`

	// DeploymentProposalSha256 corresponds to the JSON schema field
	// "deployment_proposal_sha256".
	DeploymentProposalSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_proposal_sha256"`

	// DeploymentTerms corresponds to the JSON schema field "deployment_terms".
	DeploymentTerms deployment_terms_v1.DeploymentTermsV1 `json:"deployment_terms"`

	// DeploymentTermsSha256 corresponds to the JSON schema field
	// "deployment_terms_sha256".
	DeploymentTermsSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_terms_sha256"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt UnixTimeV1 `json:"expires_at"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch PrincipalKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID string `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`
}

type PrincipalKeyEpochV1 int

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type UnixTimeV1 int
