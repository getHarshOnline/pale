// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package control_state_checkpoint_v1 contains the generated control-state-checkpoint.v1 data model.
package control_state_checkpoint_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type ChangedControlLeafKeyV1 struct {
	// LogicalKey corresponds to the JSON schema field "logical_key".
	LogicalKey ControlLogicalKeyV1 `json:"logical_key"`

	// RecordClass corresponds to the JSON schema field "record_class".
	RecordClass ControlRecordClassV1 `json:"record_class"`

	// ScopeKeySha256 corresponds to the JSON schema field "scope_key_sha256".
	ScopeKeySha256 sha256_digest_v1.Sha256DigestV1 `json:"scope_key_sha256"`
}

type ControlCheckpointDomainIDV1 string

type ControlCheckpointIDV1 string

type ControlDomainEpochV1 int

type ControlDomainSequenceV1 int

type ControlIncarnationIDV1 string

type ControlLeafCountV1 int

type ControlLogicalKeyV1 string

type ControlRecordClassV1 string

type ControlStateCheckpointV1 struct {
	// CanonicalControlCheckpointSha256 corresponds to the JSON schema field
	// "canonical_control_checkpoint_sha256".
	CanonicalControlCheckpointSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_control_checkpoint_sha256"`

	// ChangedLeafKeys corresponds to the JSON schema field "changed_leaf_keys".
	ChangedLeafKeys []ChangedControlLeafKeyV1 `json:"changed_leaf_keys"`

	// CheckpointDomainID corresponds to the JSON schema field "checkpoint_domain_id".
	CheckpointDomainID ControlCheckpointDomainIDV1 `json:"checkpoint_domain_id"`

	// CheckpointProfileRef corresponds to the JSON schema field
	// "checkpoint_profile_ref".
	CheckpointProfileRef ResidentRecordRefV1 `json:"checkpoint_profile_ref"`

	// CheckpointProfileSha256 corresponds to the JSON schema field
	// "checkpoint_profile_sha256".
	CheckpointProfileSha256 sha256_digest_v1.Sha256DigestV1 `json:"checkpoint_profile_sha256"`

	// CheckpointSigningAuthorityRef corresponds to the JSON schema field
	// "checkpoint_signing_authority_ref".
	CheckpointSigningAuthorityRef ResidentRecordRefV1 `json:"checkpoint_signing_authority_ref"`

	// CheckpointSigningAuthoritySha256 corresponds to the JSON schema field
	// "checkpoint_signing_authority_sha256".
	CheckpointSigningAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"checkpoint_signing_authority_sha256"`

	// CommittedTransactionSequence corresponds to the JSON schema field
	// "committed_transaction_sequence".
	CommittedTransactionSequence ControlTransactionSequenceV1 `json:"committed_transaction_sequence"`

	// ControlCheckpointID corresponds to the JSON schema field
	// "control_checkpoint_id".
	ControlCheckpointID ControlCheckpointIDV1 `json:"control_checkpoint_id"`

	// ControlIncarnationID corresponds to the JSON schema field
	// "control_incarnation_id".
	ControlIncarnationID ControlIncarnationIDV1 `json:"control_incarnation_id"`

	// ControlSchemaVersion corresponds to the JSON schema field
	// "control_schema_version".
	ControlSchemaVersion int `json:"control_schema_version"`

	// ControlStoreID corresponds to the JSON schema field "control_store_id".
	ControlStoreID ControlStoreIDV1 `json:"control_store_id"`

	// DomainEpoch corresponds to the JSON schema field "domain_epoch".
	DomainEpoch ControlDomainEpochV1 `json:"domain_epoch"`

	// DomainSequence corresponds to the JSON schema field "domain_sequence".
	DomainSequence ControlDomainSequenceV1 `json:"domain_sequence"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// LeafCount corresponds to the JSON schema field "leaf_count".
	LeafCount ControlLeafCountV1 `json:"leaf_count"`

	// PreviousControlCheckpointRef corresponds to the JSON schema field
	// "previous_control_checkpoint_ref".
	PreviousControlCheckpointRef *ResidentRecordRefV1 `json:"previous_control_checkpoint_ref,omitempty,omitzero"`

	// PreviousControlCheckpointSha256 corresponds to the JSON schema field
	// "previous_control_checkpoint_sha256".
	PreviousControlCheckpointSha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_control_checkpoint_sha256,omitempty,omitzero"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// StateRootSha256 corresponds to the JSON schema field "state_root_sha256".
	StateRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"state_root_sha256"`

	// UnresolvedCustodyRootSha256 corresponds to the JSON schema field
	// "unresolved_custody_root_sha256".
	UnresolvedCustodyRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"unresolved_custody_root_sha256"`
}

type ControlStoreIDV1 string

type ControlTransactionSequenceV1 int

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int
