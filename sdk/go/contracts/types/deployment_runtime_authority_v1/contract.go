// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package deployment_runtime_authority_v1 contains the generated deployment-runtime-authority.v1 data model.
package deployment_runtime_authority_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type ControlIncarnationIDV1 string

type ControlStoreIDV1 string

type DeploymentRuntimeAuthorityEpochV1 int

type DeploymentRuntimeAuthorityIDV1 string

type DeploymentRuntimeAuthorityV1 struct {
	// AuthorityEpoch corresponds to the JSON schema field "authority_epoch".
	AuthorityEpoch DeploymentRuntimeAuthorityEpochV1 `json:"authority_epoch"`

	// CanonicalDeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "canonical_deployment_runtime_authority_sha256".
	CanonicalDeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_deployment_runtime_authority_sha256"`

	// ControlIncarnationID corresponds to the JSON schema field
	// "control_incarnation_id".
	ControlIncarnationID ControlIncarnationIDV1 `json:"control_incarnation_id"`

	// ControlStoreID corresponds to the JSON schema field "control_store_id".
	ControlStoreID ControlStoreIDV1 `json:"control_store_id"`

	// DeploymentBindingRef corresponds to the JSON schema field
	// "deployment_binding_ref".
	DeploymentBindingRef ResidentRecordRefV1 `json:"deployment_binding_ref"`

	// DeploymentBindingSha256 corresponds to the JSON schema field
	// "deployment_binding_sha256".
	DeploymentBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_binding_sha256"`

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

	// PermittedRecordSchemaIds corresponds to the JSON schema field
	// "permitted_record_schema_ids".
	PermittedRecordSchemaIds []SchemaID `json:"permitted_record_schema_ids"`

	// PreviousAuthorityRef corresponds to the JSON schema field
	// "previous_authority_ref".
	PreviousAuthorityRef *ResidentRecordRefV1 `json:"previous_authority_ref,omitempty,omitzero"`

	// PreviousAuthoritySha256 corresponds to the JSON schema field
	// "previous_authority_sha256".
	PreviousAuthoritySha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_authority_sha256,omitempty,omitzero"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// RuntimeAuthorityID corresponds to the JSON schema field "runtime_authority_id".
	RuntimeAuthorityID DeploymentRuntimeAuthorityIDV1 `json:"runtime_authority_id"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// State corresponds to the JSON schema field "state".
	State PrincipalAuthorityStateV1 `json:"state"`

	// SubjectKeyEpoch corresponds to the JSON schema field "subject_key_epoch".
	SubjectKeyEpoch SigningKeyEpochV1 `json:"subject_key_epoch"`

	// SubjectKeyID corresponds to the JSON schema field "subject_key_id".
	SubjectKeyID ResidentKeyIDV1 `json:"subject_key_id"`

	// SubjectPublicKey corresponds to the JSON schema field "subject_public_key".
	SubjectPublicKey Ed25519PublicKeyV1 `json:"subject_public_key"`
}

type Ed25519PublicKeyV1 string

type PrincipalAuthorityStateV1 string

const PrincipalAuthorityStateV1Active PrincipalAuthorityStateV1 = "active"
const PrincipalAuthorityStateV1Retired PrincipalAuthorityStateV1 = "retired"

type PrincipalKeyEpochV1 int

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int
