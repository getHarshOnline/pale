// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package principal_recovery_proof_v1 contains the generated principal-recovery-proof.v1 data model.
package principal_recovery_proof_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type Ed25519PublicKeyV1 string

type PrincipalKeyEpochV1 int

type PrincipalRecoveryNonceV1 string

type PrincipalRecoveryProofIDV1 string

type PrincipalRecoveryProofV1 struct {
	// CanonicalRecordSha256 corresponds to the JSON schema field
	// "canonical_record_sha256".
	CanonicalRecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_record_sha256"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt UnixTimeV1 `json:"expires_at"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// NewKeyEpoch corresponds to the JSON schema field "new_key_epoch".
	NewKeyEpoch PrincipalKeyEpochV1 `json:"new_key_epoch"`

	// NewKeyID corresponds to the JSON schema field "new_key_id".
	NewKeyID ResidentKeyIDV1 `json:"new_key_id"`

	// NewPublicKey corresponds to the JSON schema field "new_public_key".
	NewPublicKey Ed25519PublicKeyV1 `json:"new_public_key"`

	// Nonce corresponds to the JSON schema field "nonce".
	Nonce PrincipalRecoveryNonceV1 `json:"nonce"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// PriorPrincipalRecordRef corresponds to the JSON schema field
	// "prior_principal_record_ref".
	PriorPrincipalRecordRef ResidentRecordRefV1 `json:"prior_principal_record_ref"`

	// PriorPrincipalRecordSha256 corresponds to the JSON schema field
	// "prior_principal_record_sha256".
	PriorPrincipalRecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"prior_principal_record_sha256"`

	// RecoveryPolicyRef corresponds to the JSON schema field "recovery_policy_ref".
	RecoveryPolicyRef ResidentRecordRefV1 `json:"recovery_policy_ref"`

	// RecoveryPolicySha256 corresponds to the JSON schema field
	// "recovery_policy_sha256".
	RecoveryPolicySha256 sha256_digest_v1.Sha256DigestV1 `json:"recovery_policy_sha256"`

	// RecoveryProofID corresponds to the JSON schema field "recovery_proof_id".
	RecoveryProofID PrincipalRecoveryProofIDV1 `json:"recovery_proof_id"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signatures corresponds to the JSON schema field "signatures".
	Signatures []PrincipalRecoverySignatureV1 `json:"signatures"`
}

type PrincipalRecoverySignatureV1 struct {
	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// SignedRecordSha256 corresponds to the JSON schema field "signed_record_sha256".
	SignedRecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"signed_record_sha256"`

	// SignerKeyEpoch corresponds to the JSON schema field "signer_key_epoch".
	SignerKeyEpoch PrincipalKeyEpochV1 `json:"signer_key_epoch"`

	// SignerKeyID corresponds to the JSON schema field "signer_key_id".
	SignerKeyID ResidentKeyIDV1 `json:"signer_key_id"`

	// SignerPrincipalID corresponds to the JSON schema field "signer_principal_id".
	SignerPrincipalID ResidentPrincipalIDV1 `json:"signer_principal_id"`
}

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type UnixTimeV1 int
