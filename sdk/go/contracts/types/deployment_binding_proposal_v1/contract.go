// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package deployment_binding_proposal_v1 contains the generated deployment-binding-proposal.v1 data model.
package deployment_binding_proposal_v1

import deployment_terms_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/deployment_terms_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type DeploymentBindingProposalV1 struct {
	// CanonicalProposalSha256 corresponds to the JSON schema field
	// "canonical_proposal_sha256".
	CanonicalProposalSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_proposal_sha256"`

	// CompilerKeyEpoch corresponds to the JSON schema field "compiler_key_epoch".
	CompilerKeyEpoch PrincipalKeyEpochV1 `json:"compiler_key_epoch"`

	// CompilerKeyID corresponds to the JSON schema field "compiler_key_id".
	CompilerKeyID ResidentKeyIDV1 `json:"compiler_key_id"`

	// CompilerPrincipalID corresponds to the JSON schema field
	// "compiler_principal_id".
	CompilerPrincipalID ResidentPrincipalIDV1 `json:"compiler_principal_id"`

	// DeploymentProposalID corresponds to the JSON schema field
	// "deployment_proposal_id".
	DeploymentProposalID DeploymentProposalIDV1 `json:"deployment_proposal_id"`

	// DeploymentTerms corresponds to the JSON schema field "deployment_terms".
	DeploymentTerms deployment_terms_v1.DeploymentTermsV1 `json:"deployment_terms"`

	// DeploymentTermsSha256 corresponds to the JSON schema field
	// "deployment_terms_sha256".
	DeploymentTermsSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_terms_sha256"`

	// ProposalExpiresAt corresponds to the JSON schema field "proposal_expires_at".
	ProposalExpiresAt UnixTimeV1 `json:"proposal_expires_at"`

	// ProposedAt corresponds to the JSON schema field "proposed_at".
	ProposedAt UnixTimeV1 `json:"proposed_at"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID string `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`
}

type DeploymentProposalIDV1 string

type PrincipalKeyEpochV1 int

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type UnixTimeV1 int
