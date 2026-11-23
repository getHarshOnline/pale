// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package execution_owner_fence_v1 contains the generated execution-owner-fence.v1 data model.
package execution_owner_fence_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type ExecutionAttemptIDV1 string

type ExecutionControlEpochV1 int

type ExecutionOwnerFenceIDV1 string

type ExecutionOwnerFenceStateV1 string

const ExecutionOwnerFenceStateV1Active ExecutionOwnerFenceStateV1 = "active"
const ExecutionOwnerFenceStateV1Retired ExecutionOwnerFenceStateV1 = "retired"

type ExecutionOwnerFenceV1 struct {
	// CanonicalExecutionFenceSha256 corresponds to the JSON schema field
	// "canonical_execution_fence_sha256".
	CanonicalExecutionFenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_execution_fence_sha256"`

	// ControlEpoch corresponds to the JSON schema field "control_epoch".
	ControlEpoch ExecutionControlEpochV1 `json:"control_epoch"`

	// ControllerScope corresponds to the JSON schema field "controller_scope".
	ControllerScope ResidentControllerIDV1 `json:"controller_scope"`

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

	// ExecutionAttemptID corresponds to the JSON schema field "execution_attempt_id".
	ExecutionAttemptID ExecutionAttemptIDV1 `json:"execution_attempt_id"`

	// ExecutionFenceID corresponds to the JSON schema field "execution_fence_id".
	ExecutionFenceID ExecutionOwnerFenceIDV1 `json:"execution_fence_id"`

	// ExecutionOwnerParticipantID corresponds to the JSON schema field
	// "execution_owner_participant_id".
	ExecutionOwnerParticipantID ExecutionOwnerParticipantIDV1 `json:"execution_owner_participant_id"`

	// ExecutionOwnerSystemID corresponds to the JSON schema field
	// "execution_owner_system_id".
	ExecutionOwnerSystemID ExecutionOwnerSystemIDV1 `json:"execution_owner_system_id"`

	// ExecutionPathID corresponds to the JSON schema field "execution_path_id".
	ExecutionPathID ExecutionPathSelectionIDV1 `json:"execution_path_id"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt *UnixTimeV1 `json:"expires_at,omitempty,omitzero"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// OwnerAuthorizationRef corresponds to the JSON schema field
	// "owner_authorization_ref".
	OwnerAuthorizationRef ResidentRecordRefV1 `json:"owner_authorization_ref"`

	// OwnerAuthorizationSchemaID corresponds to the JSON schema field
	// "owner_authorization_schema_id".
	OwnerAuthorizationSchemaID SchemaID `json:"owner_authorization_schema_id"`

	// OwnerAuthorizationSchemaSha256 corresponds to the JSON schema field
	// "owner_authorization_schema_sha256".
	OwnerAuthorizationSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_authorization_schema_sha256"`

	// OwnerAuthorizationSha256 corresponds to the JSON schema field
	// "owner_authorization_sha256".
	OwnerAuthorizationSha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_authorization_sha256"`

	// OwnerGeneration corresponds to the JSON schema field "owner_generation".
	OwnerGeneration ExecutionOwnerGenerationV1 `json:"owner_generation"`

	// PredecessorDispositionRef corresponds to the JSON schema field
	// "predecessor_disposition_ref".
	PredecessorDispositionRef *ResidentRecordRefV1 `json:"predecessor_disposition_ref,omitempty,omitzero"`

	// PredecessorDispositionSchemaID corresponds to the JSON schema field
	// "predecessor_disposition_schema_id".
	PredecessorDispositionSchemaID *SchemaID `json:"predecessor_disposition_schema_id,omitempty,omitzero"`

	// PredecessorDispositionSchemaSha256 corresponds to the JSON schema field
	// "predecessor_disposition_schema_sha256".
	PredecessorDispositionSchemaSha256 *sha256_digest_v1.Sha256DigestV1 `json:"predecessor_disposition_schema_sha256,omitempty,omitzero"`

	// PredecessorDispositionSha256 corresponds to the JSON schema field
	// "predecessor_disposition_sha256".
	PredecessorDispositionSha256 *sha256_digest_v1.Sha256DigestV1 `json:"predecessor_disposition_sha256,omitempty,omitzero"`

	// PreviousControlEpoch corresponds to the JSON schema field
	// "previous_control_epoch".
	PreviousControlEpoch *ExecutionControlEpochV1 `json:"previous_control_epoch,omitempty,omitzero"`

	// PreviousExecutionAttemptID corresponds to the JSON schema field
	// "previous_execution_attempt_id".
	PreviousExecutionAttemptID *ExecutionAttemptIDV1 `json:"previous_execution_attempt_id,omitempty,omitzero"`

	// PreviousFenceRef corresponds to the JSON schema field "previous_fence_ref".
	PreviousFenceRef *ResidentRecordRefV1 `json:"previous_fence_ref,omitempty,omitzero"`

	// PreviousFenceSha256 corresponds to the JSON schema field
	// "previous_fence_sha256".
	PreviousFenceSha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_fence_sha256,omitempty,omitzero"`

	// PreviousOwnerGeneration corresponds to the JSON schema field
	// "previous_owner_generation".
	PreviousOwnerGeneration *ExecutionOwnerGenerationV1 `json:"previous_owner_generation,omitempty,omitzero"`

	// PrincipalScope corresponds to the JSON schema field "principal_scope".
	PrincipalScope ResidentPrincipalIDV1 `json:"principal_scope"`

	// RelationshipScope corresponds to the JSON schema field "relationship_scope".
	RelationshipScope *RelationshipScopeV1 `json:"relationship_scope,omitempty,omitzero"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// State corresponds to the JSON schema field "state".
	State ExecutionOwnerFenceStateV1 `json:"state"`
}

type ExecutionOwnerGenerationV1 int

type ExecutionOwnerParticipantIDV1 string

type ExecutionOwnerSystemIDV1 string

type ExecutionPathSelectionIDV1 string

type RelationshipScopeV1 string

type ResidentControllerIDV1 string

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int
