// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package principal_trust_checkpoint_v1 contains the generated principal-trust-checkpoint.v1 data model.
package principal_trust_checkpoint_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type ControlIncarnationIDV1 string

type ControlStoreIDV1 string

type PrincipalKeyEpochV1 int

type PrincipalSessionFloorEpochV1 int

type PrincipalTrustCheckpointEpochV1 int

type PrincipalTrustCheckpointIDV1 string

type PrincipalTrustCheckpointV1 struct {
	// CanonicalTrustCheckpointSha256 corresponds to the JSON schema field
	// "canonical_trust_checkpoint_sha256".
	CanonicalTrustCheckpointSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_trust_checkpoint_sha256"`

	// CheckpointEpoch corresponds to the JSON schema field "checkpoint_epoch".
	CheckpointEpoch PrincipalTrustCheckpointEpochV1 `json:"checkpoint_epoch"`

	// CheckpointSigningAuthorityHeadsRootSha256 corresponds to the JSON schema field
	// "checkpoint_signing_authority_heads_root_sha256".
	CheckpointSigningAuthorityHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"checkpoint_signing_authority_heads_root_sha256"`

	// DeploymentAcceptanceRevocationHeadsRootSha256 corresponds to the JSON schema
	// field "deployment_acceptance_revocation_heads_root_sha256".
	DeploymentAcceptanceRevocationHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_acceptance_revocation_heads_root_sha256"`

	// DeploymentRuntimeAuthorityHeadsRootSha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_heads_root_sha256".
	DeploymentRuntimeAuthorityHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_heads_root_sha256"`

	// FederationSpaceHomeHeadsRootSha256 corresponds to the JSON schema field
	// "federation_space_home_heads_root_sha256".
	FederationSpaceHomeHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"federation_space_home_heads_root_sha256"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch PrincipalKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// LocalAdminAuthorityRef corresponds to the JSON schema field
	// "local_admin_authority_ref".
	LocalAdminAuthorityRef ResidentRecordRefV1 `json:"local_admin_authority_ref"`

	// LocalAdminAuthoritySha256 corresponds to the JSON schema field
	// "local_admin_authority_sha256".
	LocalAdminAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"local_admin_authority_sha256"`

	// NodeDelegationHeadsRootSha256 corresponds to the JSON schema field
	// "node_delegation_heads_root_sha256".
	NodeDelegationHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"node_delegation_heads_root_sha256"`

	// NodeDelegationRevocationHeadsRootSha256 corresponds to the JSON schema field
	// "node_delegation_revocation_heads_root_sha256".
	NodeDelegationRevocationHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"node_delegation_revocation_heads_root_sha256"`

	// OwnerRecordSigningAuthorityHeadsRootSha256 corresponds to the JSON schema field
	// "owner_record_signing_authority_heads_root_sha256".
	OwnerRecordSigningAuthorityHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_record_signing_authority_heads_root_sha256"`

	// OwnerSignerPublicationFrontierHeadsRootSha256 corresponds to the JSON schema
	// field "owner_signer_publication_frontier_heads_root_sha256".
	OwnerSignerPublicationFrontierHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_signer_publication_frontier_heads_root_sha256"`

	// PreviousTrustCheckpointRef corresponds to the JSON schema field
	// "previous_trust_checkpoint_ref".
	PreviousTrustCheckpointRef *ResidentRecordRefV1 `json:"previous_trust_checkpoint_ref,omitempty,omitzero"`

	// PreviousTrustCheckpointSha256 corresponds to the JSON schema field
	// "previous_trust_checkpoint_sha256".
	PreviousTrustCheckpointSha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_trust_checkpoint_sha256,omitempty,omitzero"`

	// PrincipalControlIncarnationID corresponds to the JSON schema field
	// "principal_control_incarnation_id".
	PrincipalControlIncarnationID ControlIncarnationIDV1 `json:"principal_control_incarnation_id"`

	// PrincipalControlStoreID corresponds to the JSON schema field
	// "principal_control_store_id".
	PrincipalControlStoreID ControlStoreIDV1 `json:"principal_control_store_id"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// PrincipalRecordRef corresponds to the JSON schema field "principal_record_ref".
	PrincipalRecordRef ResidentRecordRefV1 `json:"principal_record_ref"`

	// PrincipalRecordSha256 corresponds to the JSON schema field
	// "principal_record_sha256".
	PrincipalRecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"principal_record_sha256"`

	// ProducerIncarnationAuthorityRef corresponds to the JSON schema field
	// "producer_incarnation_authority_ref".
	ProducerIncarnationAuthorityRef ResidentRecordRefV1 `json:"producer_incarnation_authority_ref"`

	// ProducerIncarnationAuthoritySha256 corresponds to the JSON schema field
	// "producer_incarnation_authority_sha256".
	ProducerIncarnationAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"producer_incarnation_authority_sha256"`

	// ProducerIncarnationHeadsRootSha256 corresponds to the JSON schema field
	// "producer_incarnation_heads_root_sha256".
	ProducerIncarnationHeadsRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"producer_incarnation_heads_root_sha256"`

	// RecoveryPolicyRef corresponds to the JSON schema field "recovery_policy_ref".
	RecoveryPolicyRef ResidentRecordRefV1 `json:"recovery_policy_ref"`

	// RecoveryPolicySha256 corresponds to the JSON schema field
	// "recovery_policy_sha256".
	RecoveryPolicySha256 sha256_digest_v1.Sha256DigestV1 `json:"recovery_policy_sha256"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SessionFloorEpoch corresponds to the JSON schema field "session_floor_epoch".
	SessionFloorEpoch PrincipalSessionFloorEpochV1 `json:"session_floor_epoch"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// TrustCheckpointID corresponds to the JSON schema field "trust_checkpoint_id".
	TrustCheckpointID PrincipalTrustCheckpointIDV1 `json:"trust_checkpoint_id"`

	// TrustStoreID corresponds to the JSON schema field "trust_store_id".
	TrustStoreID ControlStoreIDV1 `json:"trust_store_id"`

	// TrustStoreIncarnationID corresponds to the JSON schema field
	// "trust_store_incarnation_id".
	TrustStoreIncarnationID ControlIncarnationIDV1 `json:"trust_store_incarnation_id"`
}

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type UnixTimeV1 int
