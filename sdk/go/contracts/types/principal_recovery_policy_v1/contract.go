// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package principal_recovery_policy_v1 contains the generated principal-recovery-policy.v1 data model.
package principal_recovery_policy_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type Ed25519PublicKeyV1 string

type PrincipalKeyEpochV1 int

type PrincipalRecoveryEligibleSignerV1 struct {
	// RecoveryKeyEpoch corresponds to the JSON schema field "recovery_key_epoch".
	RecoveryKeyEpoch PrincipalKeyEpochV1 `json:"recovery_key_epoch"`

	// RecoveryKeyID corresponds to the JSON schema field "recovery_key_id".
	RecoveryKeyID ResidentKeyIDV1 `json:"recovery_key_id"`

	// RecoveryPrincipalID corresponds to the JSON schema field
	// "recovery_principal_id".
	RecoveryPrincipalID ResidentPrincipalIDV1 `json:"recovery_principal_id"`

	// RecoveryPublicKey corresponds to the JSON schema field "recovery_public_key".
	RecoveryPublicKey Ed25519PublicKeyV1 `json:"recovery_public_key"`
}

type PrincipalRecoveryPolicyEpochV1 int

type PrincipalRecoveryPolicyIDV1 string

type PrincipalRecoveryPolicyV1 struct {
	// CanonicalRecordSha256 corresponds to the JSON schema field
	// "canonical_record_sha256".
	CanonicalRecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_record_sha256"`

	// EligibleSigners corresponds to the JSON schema field "eligible_signers".
	EligibleSigners []PrincipalRecoveryEligibleSignerV1 `json:"eligible_signers"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt *UnixTimeV1 `json:"expires_at,omitempty,omitzero"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalKeyEpoch corresponds to the JSON schema field
	// "issuer_principal_key_epoch".
	IssuerPrincipalKeyEpoch PrincipalKeyEpochV1 `json:"issuer_principal_key_epoch"`

	// PolicyEpoch corresponds to the JSON schema field "policy_epoch".
	PolicyEpoch PrincipalRecoveryPolicyEpochV1 `json:"policy_epoch"`

	// PreviousPolicyRef corresponds to the JSON schema field "previous_policy_ref".
	PreviousPolicyRef *ResidentRecordRefV1 `json:"previous_policy_ref,omitempty,omitzero"`

	// PreviousPolicySha256 corresponds to the JSON schema field
	// "previous_policy_sha256".
	PreviousPolicySha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_policy_sha256,omitempty,omitzero"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// ProofValiditySeconds corresponds to the JSON schema field
	// "proof_validity_seconds".
	ProofValiditySeconds RecoveryProofValiditySecondsV1 `json:"proof_validity_seconds"`

	// RecoveryPolicyID corresponds to the JSON schema field "recovery_policy_id".
	RecoveryPolicyID PrincipalRecoveryPolicyIDV1 `json:"recovery_policy_id"`

	// ReplayScope corresponds to the JSON schema field "replay_scope".
	ReplayScope PrincipalRecoveryReplayScopeV1 `json:"replay_scope"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// Threshold corresponds to the JSON schema field "threshold".
	Threshold int `json:"threshold"`
}

type PrincipalRecoveryReplayScopeV1 string

const PrincipalRecoveryReplayScopeV1PrincipalKeyRecoveryProof PrincipalRecoveryReplayScopeV1 = "principal_key_recovery_proof"

type RecoveryProofValiditySecondsV1 int

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type UnixTimeV1 int
