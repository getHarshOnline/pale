// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package deployment_acceptance_v1 contains the generated deployment-acceptance.v1 data model.
package deployment_acceptance_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type DeploymentAcceptanceDecisionV1 string

const DeploymentAcceptanceDecisionV1Accepted DeploymentAcceptanceDecisionV1 = "accepted"

type DeploymentAcceptanceIDV1 string

type DeploymentAcceptanceV1 struct {
	// AcceptanceAuthorityRef corresponds to the JSON schema field
	// "acceptance_authority_ref".
	AcceptanceAuthorityRef ResidentRecordRefV1 `json:"acceptance_authority_ref"`

	// AcceptanceAuthoritySha256 corresponds to the JSON schema field
	// "acceptance_authority_sha256".
	AcceptanceAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"acceptance_authority_sha256"`

	// AcceptanceEvidenceRef corresponds to the JSON schema field
	// "acceptance_evidence_ref".
	AcceptanceEvidenceRef ResidentRecordRefV1 `json:"acceptance_evidence_ref"`

	// AcceptanceEvidenceSha256 corresponds to the JSON schema field
	// "acceptance_evidence_sha256".
	AcceptanceEvidenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"acceptance_evidence_sha256"`

	// AcceptedAt corresponds to the JSON schema field "accepted_at".
	AcceptedAt UnixTimeV1 `json:"accepted_at"`

	// BindingValidUntil corresponds to the JSON schema field "binding_valid_until".
	BindingValidUntil UnixTimeV1 `json:"binding_valid_until"`

	// CanonicalAcceptanceSha256 corresponds to the JSON schema field
	// "canonical_acceptance_sha256".
	CanonicalAcceptanceSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_acceptance_sha256"`

	// Decision corresponds to the JSON schema field "decision".
	Decision DeploymentAcceptanceDecisionV1 `json:"decision"`

	// DeploymentAcceptanceID corresponds to the JSON schema field
	// "deployment_acceptance_id".
	DeploymentAcceptanceID DeploymentAcceptanceIDV1 `json:"deployment_acceptance_id"`

	// DeploymentProposalRef corresponds to the JSON schema field
	// "deployment_proposal_ref".
	DeploymentProposalRef ResidentRecordRefV1 `json:"deployment_proposal_ref"`

	// DeploymentProposalSha256 corresponds to the JSON schema field
	// "deployment_proposal_sha256".
	DeploymentProposalSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_proposal_sha256"`

	// DeploymentTermsSha256 corresponds to the JSON schema field
	// "deployment_terms_sha256".
	DeploymentTermsSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_terms_sha256"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// PermittedBindingIssuerPrincipalID corresponds to the JSON schema field
	// "permitted_binding_issuer_principal_id".
	PermittedBindingIssuerPrincipalID ResidentPrincipalIDV1 `json:"permitted_binding_issuer_principal_id"`

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
