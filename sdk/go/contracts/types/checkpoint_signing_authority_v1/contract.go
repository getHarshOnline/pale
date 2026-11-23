// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package checkpoint_signing_authority_v1 contains the generated checkpoint-signing-authority.v1 data model.
package checkpoint_signing_authority_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type CanonicalSignatureV1 string

type CheckpointDomainPrefixV1 string

type CheckpointSignerEpochV1 int

type CheckpointSigningAuthorityIDV1 string

type CheckpointSigningAuthorityV1 struct {
	// AllowedDomainPrefixes corresponds to the JSON schema field
	// "allowed_domain_prefixes".
	AllowedDomainPrefixes []CheckpointDomainPrefixV1 `json:"allowed_domain_prefixes"`

	// CanonicalCheckpointSigningAuthoritySha256 corresponds to the JSON schema field
	// "canonical_checkpoint_signing_authority_sha256".
	CanonicalCheckpointSigningAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_checkpoint_signing_authority_sha256"`

	// CheckpointProfileRef corresponds to the JSON schema field
	// "checkpoint_profile_ref".
	CheckpointProfileRef ResidentRecordRefV1 `json:"checkpoint_profile_ref"`

	// CheckpointProfileSha256 corresponds to the JSON schema field
	// "checkpoint_profile_sha256".
	CheckpointProfileSha256 sha256_digest_v1.Sha256DigestV1 `json:"checkpoint_profile_sha256"`

	// CheckpointSigningAuthorityID corresponds to the JSON schema field
	// "checkpoint_signing_authority_id".
	CheckpointSigningAuthorityID CheckpointSigningAuthorityIDV1 `json:"checkpoint_signing_authority_id"`

	// ControlIncarnationID corresponds to the JSON schema field
	// "control_incarnation_id".
	ControlIncarnationID ControlIncarnationIDV1 `json:"control_incarnation_id"`

	// ControlStoreID corresponds to the JSON schema field "control_store_id".
	ControlStoreID ControlStoreIDV1 `json:"control_store_id"`

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

	// PreviousAuthorityRef corresponds to the JSON schema field
	// "previous_authority_ref".
	PreviousAuthorityRef *ResidentRecordRefV1 `json:"previous_authority_ref,omitempty,omitzero"`

	// PreviousAuthoritySha256 corresponds to the JSON schema field
	// "previous_authority_sha256".
	PreviousAuthoritySha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_authority_sha256,omitempty,omitzero"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Scope corresponds to the JSON schema field "scope".
	Scope CheckpointSigningScopeV1 `json:"scope"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// SignerEpoch corresponds to the JSON schema field "signer_epoch".
	SignerEpoch CheckpointSignerEpochV1 `json:"signer_epoch"`

	// State corresponds to the JSON schema field "state".
	State PrincipalAuthorityStateV1 `json:"state"`

	// SubjectKeyEpoch corresponds to the JSON schema field "subject_key_epoch".
	SubjectKeyEpoch SigningKeyEpochV1 `json:"subject_key_epoch"`

	// SubjectKeyID corresponds to the JSON schema field "subject_key_id".
	SubjectKeyID ResidentKeyIDV1 `json:"subject_key_id"`

	// SubjectPublicKey corresponds to the JSON schema field "subject_public_key".
	SubjectPublicKey Ed25519PublicKeyV1 `json:"subject_public_key"`
}

type CheckpointSigningScopeDeploymentV1 struct {
	// DeploymentBindingRef corresponds to the JSON schema field
	// "deployment_binding_ref".
	DeploymentBindingRef ResidentRecordRefV1 `json:"deployment_binding_ref"`

	// DeploymentBindingSha256 corresponds to the JSON schema field
	// "deployment_binding_sha256".
	DeploymentBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_binding_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type CheckpointSigningScopePrincipalTrustStoreV1 struct {
	// BootstrapID corresponds to the JSON schema field "bootstrap_id".
	BootstrapID PrincipalBootstrapIDV1 `json:"bootstrap_id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ControlIncarnationIDV1 string

type ControlStoreIDV1 string

type Ed25519PublicKeyV1 string

type PrincipalAuthorityStateV1 string

const PrincipalAuthorityStateV1Active PrincipalAuthorityStateV1 = "active"
const PrincipalAuthorityStateV1Retired PrincipalAuthorityStateV1 = "retired"

type PrincipalBootstrapIDV1 string

type PrincipalKeyEpochV1 int

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int

type CheckpointSigningScopeV1 struct {
	PrincipalTrustStore *CheckpointSigningScopePrincipalTrustStoreV1 `json:"-"`
	Deployment          *CheckpointSigningScopeDeploymentV1          `json:"-"`
}

func (value CheckpointSigningScopeV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.PrincipalTrustStore != nil {
		if value.PrincipalTrustStore.Kind != "principal_trust_store" {
			return nil, fmt.Errorf("CheckpointSigningScopeV1: selected variant has discriminator %q", value.PrincipalTrustStore.Kind)
		}
		candidate, err := json.Marshal(value.PrincipalTrustStore)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Deployment != nil {
		if value.Deployment.Kind != "deployment" {
			return nil, fmt.Errorf("CheckpointSigningScopeV1: selected variant has discriminator %q", value.Deployment.Kind)
		}
		candidate, err := json.Marshal(value.Deployment)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("CheckpointSigningScopeV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *CheckpointSigningScopeV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = CheckpointSigningScopeV1{}
	switch discriminator.Kind {
	case "principal_trust_store":
		var variant CheckpointSigningScopePrincipalTrustStoreV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.PrincipalTrustStore = &variant
	case "deployment":
		var variant CheckpointSigningScopeDeploymentV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Deployment = &variant
	default:
		return fmt.Errorf("CheckpointSigningScopeV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
