// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package principal_bootstrap_receipt_v1 contains the generated principal-bootstrap-receipt.v1 data model.
package principal_bootstrap_receipt_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type ControlCheckpointDomainIDV1 string

type ControlDomainEpochV1 int

type ControlDomainSequenceV1 int

type ControlIncarnationIDV1 string

type ControlStoreIDV1 string

type PrincipalBootstrapIDV1 string

type PrincipalBootstrapInitialControlCheckpointV1 struct {
	// CheckpointDomainID corresponds to the JSON schema field "checkpoint_domain_id".
	CheckpointDomainID ControlCheckpointDomainIDV1 `json:"checkpoint_domain_id"`

	// ControlCheckpointRef corresponds to the JSON schema field
	// "control_checkpoint_ref".
	ControlCheckpointRef ResidentRecordRefV1 `json:"control_checkpoint_ref"`

	// ControlCheckpointSha256 corresponds to the JSON schema field
	// "control_checkpoint_sha256".
	ControlCheckpointSha256 sha256_digest_v1.Sha256DigestV1 `json:"control_checkpoint_sha256"`

	// DomainEpoch corresponds to the JSON schema field "domain_epoch".
	DomainEpoch ControlDomainEpochV1 `json:"domain_epoch"`

	// DomainSequence corresponds to the JSON schema field "domain_sequence".
	DomainSequence ControlDomainSequenceV1 `json:"domain_sequence"`
}

type PrincipalBootstrapReceiptV1 struct {
	// BootstrapID corresponds to the JSON schema field "bootstrap_id".
	BootstrapID PrincipalBootstrapIDV1 `json:"bootstrap_id"`

	// CandidateRootKeyID corresponds to the JSON schema field
	// "candidate_root_key_id".
	CandidateRootKeyID ResidentKeyIDV1 `json:"candidate_root_key_id"`

	// CanonicalBootstrapReceiptSha256 corresponds to the JSON schema field
	// "canonical_bootstrap_receipt_sha256".
	CanonicalBootstrapReceiptSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_bootstrap_receipt_sha256"`

	// CommittedAt corresponds to the JSON schema field "committed_at".
	CommittedAt UnixTimeV1 `json:"committed_at"`

	// InitialAdminAuthorityRef corresponds to the JSON schema field
	// "initial_admin_authority_ref".
	InitialAdminAuthorityRef ResidentRecordRefV1 `json:"initial_admin_authority_ref"`

	// InitialAdminAuthoritySha256 corresponds to the JSON schema field
	// "initial_admin_authority_sha256".
	InitialAdminAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"initial_admin_authority_sha256"`

	// InitialControlCheckpoints corresponds to the JSON schema field
	// "initial_control_checkpoints".
	InitialControlCheckpoints []PrincipalBootstrapInitialControlCheckpointV1 `json:"initial_control_checkpoints"`

	// InitialProducerIncarnationAuthorityRef corresponds to the JSON schema field
	// "initial_producer_incarnation_authority_ref".
	InitialProducerIncarnationAuthorityRef ResidentRecordRefV1 `json:"initial_producer_incarnation_authority_ref"`

	// InitialProducerIncarnationAuthoritySha256 corresponds to the JSON schema field
	// "initial_producer_incarnation_authority_sha256".
	InitialProducerIncarnationAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"initial_producer_incarnation_authority_sha256"`

	// InitialTrustStoreCheckpointSigningAuthorityRef corresponds to the JSON schema
	// field "initial_trust_store_checkpoint_signing_authority_ref".
	InitialTrustStoreCheckpointSigningAuthorityRef ResidentRecordRefV1 `json:"initial_trust_store_checkpoint_signing_authority_ref"`

	// InitialTrustStoreCheckpointSigningAuthoritySha256 corresponds to the JSON
	// schema field "initial_trust_store_checkpoint_signing_authority_sha256".
	InitialTrustStoreCheckpointSigningAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"initial_trust_store_checkpoint_signing_authority_sha256"`

	// PrincipalControlIncarnationID corresponds to the JSON schema field
	// "principal_control_incarnation_id".
	PrincipalControlIncarnationID ControlIncarnationIDV1 `json:"principal_control_incarnation_id"`

	// PrincipalControlStoreID corresponds to the JSON schema field
	// "principal_control_store_id".
	PrincipalControlStoreID ControlStoreIDV1 `json:"principal_control_store_id"`

	// PrincipalRecordRef corresponds to the JSON schema field "principal_record_ref".
	PrincipalRecordRef ResidentRecordRefV1 `json:"principal_record_ref"`

	// PrincipalRecordSha256 corresponds to the JSON schema field
	// "principal_record_sha256".
	PrincipalRecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"principal_record_sha256"`

	// RecoveryPolicyRef corresponds to the JSON schema field "recovery_policy_ref".
	RecoveryPolicyRef ResidentRecordRefV1 `json:"recovery_policy_ref"`

	// RecoveryPolicySha256 corresponds to the JSON schema field
	// "recovery_policy_sha256".
	RecoveryPolicySha256 sha256_digest_v1.Sha256DigestV1 `json:"recovery_policy_sha256"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// TrustCheckpointEpoch corresponds to the JSON schema field
	// "trust_checkpoint_epoch".
	TrustCheckpointEpoch PrincipalTrustCheckpointEpochV1 `json:"trust_checkpoint_epoch"`

	// TrustCheckpointRef corresponds to the JSON schema field "trust_checkpoint_ref".
	TrustCheckpointRef ResidentRecordRefV1 `json:"trust_checkpoint_ref"`

	// TrustCheckpointSha256 corresponds to the JSON schema field
	// "trust_checkpoint_sha256".
	TrustCheckpointSha256 sha256_digest_v1.Sha256DigestV1 `json:"trust_checkpoint_sha256"`

	// TrustStoreID corresponds to the JSON schema field "trust_store_id".
	TrustStoreID ControlStoreIDV1 `json:"trust_store_id"`

	// TrustStoreIncarnationID corresponds to the JSON schema field
	// "trust_store_incarnation_id".
	TrustStoreIncarnationID ControlIncarnationIDV1 `json:"trust_store_incarnation_id"`
}

type PrincipalTrustCheckpointEpochV1 int

type ResidentKeyIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type UnixTimeV1 int
