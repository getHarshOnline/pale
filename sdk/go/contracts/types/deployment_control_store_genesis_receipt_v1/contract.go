// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package deployment_control_store_genesis_receipt_v1 contains the generated deployment-control-store-genesis-receipt.v1 data model.
package deployment_control_store_genesis_receipt_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type ControlCheckpointDomainIDV1 string

type ControlDomainEpochV1 int

type ControlDomainSequenceV1 int

type ControlIncarnationIDV1 string

type ControlStoreIDV1 string

type DeploymentControlStoreGenesisIDV1 string

type DeploymentControlStoreGenesisReceiptV1 struct {
	// CanonicalDeploymentControlStoreGenesisReceiptSha256 corresponds to the JSON
	// schema field "canonical_deployment_control_store_genesis_receipt_sha256".
	CanonicalDeploymentControlStoreGenesisReceiptSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_deployment_control_store_genesis_receipt_sha256"`

	// CheckpointSigningAuthorityRef corresponds to the JSON schema field
	// "checkpoint_signing_authority_ref".
	CheckpointSigningAuthorityRef ResidentRecordRefV1 `json:"checkpoint_signing_authority_ref"`

	// CheckpointSigningAuthoritySha256 corresponds to the JSON schema field
	// "checkpoint_signing_authority_sha256".
	CheckpointSigningAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"checkpoint_signing_authority_sha256"`

	// CommittedAt corresponds to the JSON schema field "committed_at".
	CommittedAt UnixTimeV1 `json:"committed_at"`

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

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`

	// GenesisID corresponds to the JSON schema field "genesis_id".
	GenesisID DeploymentControlStoreGenesisIDV1 `json:"genesis_id"`

	// InitialControlCheckpoints corresponds to the JSON schema field
	// "initial_control_checkpoints".
	InitialControlCheckpoints []PrincipalBootstrapInitialControlCheckpointV1 `json:"initial_control_checkpoints"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// WorkspaceID corresponds to the JSON schema field "workspace_id".
	WorkspaceID WorkspaceID `json:"workspace_id"`
}

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

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int

type WorkspaceID string
