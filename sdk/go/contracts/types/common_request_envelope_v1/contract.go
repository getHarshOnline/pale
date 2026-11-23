// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package common_request_envelope_v1 contains the generated common-request-envelope.v1 data model.
package common_request_envelope_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type AuthorityEvidenceV1 struct {
	// AuthorityRef corresponds to the JSON schema field "authority_ref".
	AuthorityRef ResidentRecordRefV1 `json:"authority_ref"`

	// AuthoritySha256 corresponds to the JSON schema field "authority_sha256".
	AuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"authority_sha256"`
}

type BoundedResidentTextV1 string

type CanonicalSignatureV1 string

type CapabilityIDV1 string

type CommonRequestBodyV1 struct {
	// Payload corresponds to the JSON schema field "payload".
	Payload any `json:"payload"`

	// PayloadSha256 corresponds to the JSON schema field "payload_sha256".
	PayloadSha256 sha256_digest_v1.Sha256DigestV1 `json:"payload_sha256"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`
}

type CommonRequestEnvelopeV1 struct {
	// Body corresponds to the JSON schema field "body".
	Body CommonRequestBodyV1 `json:"body"`

	// Caller corresponds to the JSON schema field "caller".
	Caller RequestCallerV1 `json:"caller"`

	// CallerSignature corresponds to the JSON schema field "caller_signature".
	CallerSignature CanonicalSignatureV1 `json:"caller_signature"`

	// CanonicalRequestSha256 corresponds to the JSON schema field
	// "canonical_request_sha256".
	CanonicalRequestSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_request_sha256"`

	// CapabilityID corresponds to the JSON schema field "capability_id".
	CapabilityID CapabilityIDV1 `json:"capability_id"`

	// CorrelationID corresponds to the JSON schema field "correlation_id".
	CorrelationID ResidentCorrelationIDV1 `json:"correlation_id"`

	// EnvelopeSchemaID corresponds to the JSON schema field "envelope_schema_id".
	EnvelopeSchemaID SchemaID `json:"envelope_schema_id"`

	// EnvelopeSchemaSha256 corresponds to the JSON schema field
	// "envelope_schema_sha256".
	EnvelopeSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"envelope_schema_sha256"`

	// ExecutionOwnerFence corresponds to the JSON schema field
	// "execution_owner_fence".
	ExecutionOwnerFence *CommonRequestExecutionOwnerFenceV1 `json:"execution_owner_fence,omitempty,omitzero"`

	// ExpectedPrecondition corresponds to the JSON schema field
	// "expected_precondition".
	ExpectedPrecondition *ExpectedPreconditionV1 `json:"expected_precondition,omitempty,omitzero"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt UnixTimeV1 `json:"expires_at"`

	// FederationClaim corresponds to the JSON schema field "federation_claim".
	FederationClaim *CommonRequestFederationClaimV1 `json:"federation_claim,omitempty,omitzero"`

	// FenceToken corresponds to the JSON schema field "fence_token".
	FenceToken *sha256_digest_v1.Sha256DigestV1 `json:"fence_token,omitempty,omitzero"`

	// Generation corresponds to the JSON schema field "generation".
	Generation *sha256_digest_v1.Sha256DigestV1 `json:"generation,omitempty,omitzero"`

	// IdempotencyKey corresponds to the JSON schema field "idempotency_key".
	IdempotencyKey RequestIdempotencyKeyV1 `json:"idempotency_key"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// Operation corresponds to the JSON schema field "operation".
	Operation PackageResourceRefV1 `json:"operation"`

	// OperationTarget corresponds to the JSON schema field "operation_target".
	OperationTarget *OperationTargetReferenceV1 `json:"operation_target,omitempty,omitzero"`

	// RequestID corresponds to the JSON schema field "request_id".
	RequestID ResidentRequestIDV1 `json:"request_id"`

	// ResourceConstraints corresponds to the JSON schema field
	// "resource_constraints".
	ResourceConstraints []BoundedResidentTextV1 `json:"resource_constraints"`

	// RiskClass corresponds to the JSON schema field "risk_class".
	RiskClass BoundedResidentTextV1 `json:"risk_class"`

	// Route corresponds to the JSON schema field "route".
	Route CommonRequestRouteV1 `json:"route"`

	// Scope corresponds to the JSON schema field "scope".
	Scope CommonRequestScopeV1 `json:"scope"`

	// SessionAdmission corresponds to the JSON schema field "session_admission".
	SessionAdmission CommonRequestSessionAdmissionV1 `json:"session_admission"`

	// SigningAuthorityBinding corresponds to the JSON schema field
	// "signing_authority_binding".
	SigningAuthorityBinding SigningAuthorityBindingV1 `json:"signing_authority_binding"`

	// WorkspaceID corresponds to the JSON schema field "workspace_id".
	WorkspaceID ResidentWorkspaceIDV1 `json:"workspace_id"`
}

type CommonRequestExecutionOwnerFenceV1 struct {
	// ControlEpoch corresponds to the JSON schema field "control_epoch".
	ControlEpoch ExecutionControlEpochV1 `json:"control_epoch"`

	// ExecutionAttemptID corresponds to the JSON schema field "execution_attempt_id".
	ExecutionAttemptID ExecutionAttemptIDV1 `json:"execution_attempt_id"`

	// ExecutionFenceRef corresponds to the JSON schema field "execution_fence_ref".
	ExecutionFenceRef ResidentRecordRefV1 `json:"execution_fence_ref"`

	// ExecutionFenceSha256 corresponds to the JSON schema field
	// "execution_fence_sha256".
	ExecutionFenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"execution_fence_sha256"`

	// ExecutionOwnerParticipantID corresponds to the JSON schema field
	// "execution_owner_participant_id".
	ExecutionOwnerParticipantID ExecutionOwnerParticipantIDV1 `json:"execution_owner_participant_id"`

	// ExecutionOwnerSystemID corresponds to the JSON schema field
	// "execution_owner_system_id".
	ExecutionOwnerSystemID ExecutionOwnerSystemIDV1 `json:"execution_owner_system_id"`

	// ExecutionPathID corresponds to the JSON schema field "execution_path_id".
	ExecutionPathID ExecutionPathSelectionIDV1 `json:"execution_path_id"`

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
}

type CommonRequestFederationClaimV1 struct {
	// GrantID corresponds to the JSON schema field "grant_id".
	GrantID FederationGrantIDV1 `json:"grant_id"`

	// GrantSha256 corresponds to the JSON schema field "grant_sha256".
	GrantSha256 sha256_digest_v1.Sha256DigestV1 `json:"grant_sha256"`

	// MembershipID corresponds to the JSON schema field "membership_id".
	MembershipID FederationMembershipIDV1 `json:"membership_id"`

	// MembershipSha256 corresponds to the JSON schema field "membership_sha256".
	MembershipSha256 sha256_digest_v1.Sha256DigestV1 `json:"membership_sha256"`

	// MembershipStateEpoch corresponds to the JSON schema field
	// "membership_state_epoch".
	MembershipStateEpoch FederationMembershipEpochV1 `json:"membership_state_epoch"`

	// RevocationProofs corresponds to the JSON schema field "revocation_proofs".
	RevocationProofs []FederationRevocationProofRefV1 `json:"revocation_proofs"`

	// SpaceGovernanceEpoch corresponds to the JSON schema field
	// "space_governance_epoch".
	SpaceGovernanceEpoch FederationGovernanceEpochV1 `json:"space_governance_epoch"`

	// SpaceID corresponds to the JSON schema field "space_id".
	SpaceID FederationSpaceIDV1 `json:"space_id"`
}

type CommonRequestRouteV1 struct {
	// ContractSetRef corresponds to the JSON schema field "contract_set_ref".
	ContractSetRef ResidentRecordRefV1 `json:"contract_set_ref"`

	// ContractSetSha256 corresponds to the JSON schema field "contract_set_sha256".
	ContractSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"contract_set_sha256"`

	// DestinationNodeID corresponds to the JSON schema field "destination_node_id".
	DestinationNodeID *ResidentNodeIDV1 `json:"destination_node_id,omitempty,omitzero"`

	// DestinationPrincipalID corresponds to the JSON schema field
	// "destination_principal_id".
	DestinationPrincipalID ResidentPrincipalIDV1 `json:"destination_principal_id"`

	// OriginNodeID corresponds to the JSON schema field "origin_node_id".
	OriginNodeID ResidentNodeIDV1 `json:"origin_node_id"`

	// OriginPrincipalID corresponds to the JSON schema field "origin_principal_id".
	OriginPrincipalID ResidentPrincipalIDV1 `json:"origin_principal_id"`

	// RouteBindingRef corresponds to the JSON schema field "route_binding_ref".
	RouteBindingRef ResidentRecordRefV1 `json:"route_binding_ref"`

	// RouteBindingSchemaID corresponds to the JSON schema field
	// "route_binding_schema_id".
	RouteBindingSchemaID SchemaID `json:"route_binding_schema_id"`

	// RouteBindingSchemaSha256 corresponds to the JSON schema field
	// "route_binding_schema_sha256".
	RouteBindingSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"route_binding_schema_sha256"`

	// RouteBindingSha256 corresponds to the JSON schema field "route_binding_sha256".
	RouteBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"route_binding_sha256"`

	// RouteGeneration corresponds to the JSON schema field "route_generation".
	RouteGeneration RouteGenerationV1 `json:"route_generation"`

	// RouteID corresponds to the JSON schema field "route_id".
	RouteID ResidentRouteIDV1 `json:"route_id"`

	// Transport corresponds to the JSON schema field "transport".
	Transport BoundedResidentTextV1 `json:"transport"`
}

type CommonRequestScopeV1 struct {
	// AuthorityEvidence corresponds to the JSON schema field "authority_evidence".
	AuthorityEvidence []AuthorityEvidenceV1 `json:"authority_evidence"`

	// ControllerScope corresponds to the JSON schema field "controller_scope".
	ControllerScope ResidentControllerIDV1 `json:"controller_scope"`

	// DataClassification corresponds to the JSON schema field "data_classification".
	DataClassification BoundedResidentTextV1 `json:"data_classification"`

	// Environment corresponds to the JSON schema field "environment".
	Environment BoundedResidentTextV1 `json:"environment"`

	// PrincipalScope corresponds to the JSON schema field "principal_scope".
	PrincipalScope ResidentPrincipalIDV1 `json:"principal_scope"`

	// Purpose corresponds to the JSON schema field "purpose".
	Purpose BoundedResidentTextV1 `json:"purpose"`

	// RelationshipScope corresponds to the JSON schema field "relationship_scope".
	RelationshipScope *RelationshipScopeV1 `json:"relationship_scope,omitempty,omitzero"`
}

type CommonRequestSessionAdmissionV1 struct {
	// CheckedAt corresponds to the JSON schema field "checked_at".
	CheckedAt UnixTimeV1 `json:"checked_at"`

	// CheckedRevocationCheckpointRef corresponds to the JSON schema field
	// "checked_revocation_checkpoint_ref".
	CheckedRevocationCheckpointRef ResidentRecordRefV1 `json:"checked_revocation_checkpoint_ref"`

	// CheckedRevocationCheckpointSha256 corresponds to the JSON schema field
	// "checked_revocation_checkpoint_sha256".
	CheckedRevocationCheckpointSha256 sha256_digest_v1.Sha256DigestV1 `json:"checked_revocation_checkpoint_sha256"`

	// CheckedRevocationEpoch corresponds to the JSON schema field
	// "checked_revocation_epoch".
	CheckedRevocationEpoch PrincipalRevocationEpochV1 `json:"checked_revocation_epoch"`

	// CheckedRevocationRef corresponds to the JSON schema field
	// "checked_revocation_ref".
	CheckedRevocationRef *ResidentRecordRefV1 `json:"checked_revocation_ref,omitempty,omitzero"`

	// CheckedRevocationSha256 corresponds to the JSON schema field
	// "checked_revocation_sha256".
	CheckedRevocationSha256 *sha256_digest_v1.Sha256DigestV1 `json:"checked_revocation_sha256,omitempty,omitzero"`

	// DeploymentBindingRef corresponds to the JSON schema field
	// "deployment_binding_ref".
	DeploymentBindingRef ResidentRecordRefV1 `json:"deployment_binding_ref"`

	// DeploymentBindingSha256 corresponds to the JSON schema field
	// "deployment_binding_sha256".
	DeploymentBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_binding_sha256"`

	// GrantIssuerKeyEpoch corresponds to the JSON schema field
	// "grant_issuer_key_epoch".
	GrantIssuerKeyEpoch PrincipalKeyEpochV1 `json:"grant_issuer_key_epoch"`

	// GrantIssuerKeyID corresponds to the JSON schema field "grant_issuer_key_id".
	GrantIssuerKeyID ResidentKeyIDV1 `json:"grant_issuer_key_id"`

	// GrantIssuerPrincipalID corresponds to the JSON schema field
	// "grant_issuer_principal_id".
	GrantIssuerPrincipalID ResidentPrincipalIDV1 `json:"grant_issuer_principal_id"`

	// SessionAdmissionID corresponds to the JSON schema field "session_admission_id".
	SessionAdmissionID SessionAdmissionIDV1 `json:"session_admission_id"`

	// SessionAdmissionLeaseDigest corresponds to the JSON schema field
	// "session_admission_lease_digest".
	SessionAdmissionLeaseDigest sha256_digest_v1.Sha256DigestV1 `json:"session_admission_lease_digest"`

	// SessionFloorEpoch corresponds to the JSON schema field "session_floor_epoch".
	SessionFloorEpoch PrincipalSessionFloorEpochV1 `json:"session_floor_epoch"`

	// SessionGrantID corresponds to the JSON schema field "session_grant_id".
	SessionGrantID SessionGrantIDV1 `json:"session_grant_id"`

	// SessionGrantSha256 corresponds to the JSON schema field "session_grant_sha256".
	SessionGrantSha256 sha256_digest_v1.Sha256DigestV1 `json:"session_grant_sha256"`
}

type ExecutionAttemptIDV1 string

type ExecutionControlEpochV1 int

type ExecutionOwnerGenerationV1 int

type ExecutionOwnerParticipantIDV1 string

type ExecutionOwnerSystemIDV1 string

type ExecutionPathSelectionIDV1 string

type ExpectedPreconditionV1 struct {
	// ExpectedStateSha256 corresponds to the JSON schema field
	// "expected_state_sha256".
	ExpectedStateSha256 sha256_digest_v1.Sha256DigestV1 `json:"expected_state_sha256"`

	// StateSchemaID corresponds to the JSON schema field "state_schema_id".
	StateSchemaID SchemaID `json:"state_schema_id"`

	// StateSchemaSha256 corresponds to the JSON schema field "state_schema_sha256".
	StateSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"state_schema_sha256"`
}

type FederationGovernanceEpochV1 int

type FederationGrantIDV1 string

type FederationMembershipEpochV1 int

type FederationMembershipIDV1 string

type FederationRevocationProofRefV1 struct {
	// AuthorityPrincipalID corresponds to the JSON schema field
	// "authority_principal_id".
	AuthorityPrincipalID ResidentPrincipalIDV1 `json:"authority_principal_id"`

	// ProofRef corresponds to the JSON schema field "proof_ref".
	ProofRef ResidentRecordRefV1 `json:"proof_ref"`

	// ProofSha256 corresponds to the JSON schema field "proof_sha256".
	ProofSha256 sha256_digest_v1.Sha256DigestV1 `json:"proof_sha256"`
}

type FederationSpaceIDV1 string

type IntegratedOwnerAuthorityV1 struct {
	// CheckedPrincipalTrustCheckpointRef corresponds to the JSON schema field
	// "checked_principal_trust_checkpoint_ref".
	CheckedPrincipalTrustCheckpointRef ResidentRecordRefV1 `json:"checked_principal_trust_checkpoint_ref"`

	// CheckedPrincipalTrustCheckpointSha256 corresponds to the JSON schema field
	// "checked_principal_trust_checkpoint_sha256".
	CheckedPrincipalTrustCheckpointSha256 sha256_digest_v1.Sha256DigestV1 `json:"checked_principal_trust_checkpoint_sha256"`

	// OwnerComponentID corresponds to the JSON schema field "owner_component_id".
	OwnerComponentID OwnerComponentIDV1 `json:"owner_component_id"`

	// OwnerContractManifestRef corresponds to the JSON schema field
	// "owner_contract_manifest_ref".
	OwnerContractManifestRef ResidentRecordRefV1 `json:"owner_contract_manifest_ref"`

	// OwnerContractManifestSha256 corresponds to the JSON schema field
	// "owner_contract_manifest_sha256".
	OwnerContractManifestSha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_contract_manifest_sha256"`

	// OwnerRecordSigningAuthorityRef corresponds to the JSON schema field
	// "owner_record_signing_authority_ref".
	OwnerRecordSigningAuthorityRef ResidentRecordRefV1 `json:"owner_record_signing_authority_ref"`

	// OwnerRecordSigningAuthoritySha256 corresponds to the JSON schema field
	// "owner_record_signing_authority_sha256".
	OwnerRecordSigningAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_record_signing_authority_sha256"`

	// OwnerSignerRecordSequence corresponds to the JSON schema field
	// "owner_signer_record_sequence".
	OwnerSignerRecordSequence OwnerSignerRecordSequenceV1 `json:"owner_signer_record_sequence"`

	// PreviousOwnerRecordSha256 corresponds to the JSON schema field
	// "previous_owner_record_sha256".
	PreviousOwnerRecordSha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_owner_record_sha256,omitempty,omitzero"`
}

type IntegratedRuntimeAuthorityV1 struct {
	// CheckedPrincipalTrustCheckpointRef corresponds to the JSON schema field
	// "checked_principal_trust_checkpoint_ref".
	CheckedPrincipalTrustCheckpointRef ResidentRecordRefV1 `json:"checked_principal_trust_checkpoint_ref"`

	// CheckedPrincipalTrustCheckpointSha256 corresponds to the JSON schema field
	// "checked_principal_trust_checkpoint_sha256".
	CheckedPrincipalTrustCheckpointSha256 sha256_digest_v1.Sha256DigestV1 `json:"checked_principal_trust_checkpoint_sha256"`

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
}

type OperationTargetReferenceV1 struct {
	// OperationHandleRef corresponds to the JSON schema field "operation_handle_ref".
	OperationHandleRef ResidentRecordRefV1 `json:"operation_handle_ref"`

	// OperationHandleSha256 corresponds to the JSON schema field
	// "operation_handle_sha256".
	OperationHandleSha256 sha256_digest_v1.Sha256DigestV1 `json:"operation_handle_sha256"`
}

type OwnerComponentIDV1 string

type OwnerSignerRecordSequenceV1 int

type PackageIDV1 string

type PackageResourceIDV1 string

type PackageResourceRefV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`
}

type PrincipalKeyEpochV1 int

type PrincipalRevocationEpochV1 int

type PrincipalSessionFloorEpochV1 int

type RelationshipScopeV1 string

type RequestCallerIDV1 string

type RequestCallerRoleV1 string

type RequestCallerSurfaceV1 string

type RequestCallerV1 struct {
	// Identity corresponds to the JSON schema field "identity".
	Identity RequestCallerIDV1 `json:"identity"`

	// Role corresponds to the JSON schema field "role".
	Role RequestCallerRoleV1 `json:"role"`

	// SignerKeyEpoch corresponds to the JSON schema field "signer_key_epoch".
	SignerKeyEpoch SigningKeyEpochV1 `json:"signer_key_epoch"`

	// SignerKeyID corresponds to the JSON schema field "signer_key_id".
	SignerKeyID ResidentKeyIDV1 `json:"signer_key_id"`

	// Surface corresponds to the JSON schema field "surface".
	Surface RequestCallerSurfaceV1 `json:"surface"`
}

type RequestIdempotencyKeyV1 string

type ResidentControllerIDV1 string

type ResidentCorrelationIDV1 string

type ResidentKeyIDV1 string

type ResidentNodeIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type ResidentRequestIDV1 string

type ResidentRouteIDV1 string

type ResidentWorkspaceIDV1 string

type RouteGenerationV1 int

type SchemaID string

type SessionAdmissionIDV1 string

type SessionGrantIDV1 string

type SigningAuthorityBindingV1 struct {
	// AuthorityMode corresponds to the JSON schema field "authority_mode".
	AuthorityMode SigningAuthorityModeV1 `json:"authority_mode"`

	// IntegratedOwner corresponds to the JSON schema field "integrated_owner".
	IntegratedOwner *IntegratedOwnerAuthorityV1 `json:"integrated_owner,omitempty,omitzero"`

	// IntegratedRuntime corresponds to the JSON schema field "integrated_runtime".
	IntegratedRuntime *IntegratedRuntimeAuthorityV1 `json:"integrated_runtime,omitempty,omitzero"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// SignerKeyEpoch corresponds to the JSON schema field "signer_key_epoch".
	SignerKeyEpoch SigningKeyEpochV1 `json:"signer_key_epoch"`

	// SignerKeyID corresponds to the JSON schema field "signer_key_id".
	SignerKeyID ResidentKeyIDV1 `json:"signer_key_id"`

	// StandaloneOwnerNative corresponds to the JSON schema field
	// "standalone_owner_native".
	StandaloneOwnerNative *StandaloneOwnerNativeAuthorityV1 `json:"standalone_owner_native,omitempty,omitzero"`
}

type SigningAuthorityModeV1 string

const SigningAuthorityModeV1IntegratedOwner SigningAuthorityModeV1 = "integrated_owner"
const SigningAuthorityModeV1IntegratedRuntime SigningAuthorityModeV1 = "integrated_runtime"
const SigningAuthorityModeV1StandaloneOwnerNative SigningAuthorityModeV1 = "standalone_owner_native"

type SigningKeyEpochV1 int

type StandaloneOwnerNativeAuthorityV1 struct {
	// CheckedOwnerRevocationFrontierRef corresponds to the JSON schema field
	// "checked_owner_revocation_frontier_ref".
	CheckedOwnerRevocationFrontierRef ResidentRecordRefV1 `json:"checked_owner_revocation_frontier_ref"`

	// CheckedOwnerRevocationFrontierSha256 corresponds to the JSON schema field
	// "checked_owner_revocation_frontier_sha256".
	CheckedOwnerRevocationFrontierSha256 sha256_digest_v1.Sha256DigestV1 `json:"checked_owner_revocation_frontier_sha256"`

	// OwnerAuthorityRef corresponds to the JSON schema field "owner_authority_ref".
	OwnerAuthorityRef ResidentRecordRefV1 `json:"owner_authority_ref"`

	// OwnerAuthoritySchemaID corresponds to the JSON schema field
	// "owner_authority_schema_id".
	OwnerAuthoritySchemaID SchemaID `json:"owner_authority_schema_id"`

	// OwnerAuthoritySchemaSha256 corresponds to the JSON schema field
	// "owner_authority_schema_sha256".
	OwnerAuthoritySchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_authority_schema_sha256"`

	// OwnerAuthoritySha256 corresponds to the JSON schema field
	// "owner_authority_sha256".
	OwnerAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_authority_sha256"`

	// OwnerComponentID corresponds to the JSON schema field "owner_component_id".
	OwnerComponentID OwnerComponentIDV1 `json:"owner_component_id"`

	// OwnerContractManifestRef corresponds to the JSON schema field
	// "owner_contract_manifest_ref".
	OwnerContractManifestRef ResidentRecordRefV1 `json:"owner_contract_manifest_ref"`

	// OwnerContractManifestSha256 corresponds to the JSON schema field
	// "owner_contract_manifest_sha256".
	OwnerContractManifestSha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_contract_manifest_sha256"`
}

type UnixTimeV1 int
