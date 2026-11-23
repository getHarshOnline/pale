// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package capability_result_v1 contains the generated capability-result.v1 data model.
package capability_result_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type BoundedResidentTextV1 string

type CanonicalSignatureV1 string

type CapabilityAttemptIDV1 string

type CapabilityFailureV1 struct {
	// FailureRef corresponds to the JSON schema field "failure_ref".
	FailureRef ResidentRecordRefV1 `json:"failure_ref"`

	// FailureSha256 corresponds to the JSON schema field "failure_sha256".
	FailureSha256 sha256_digest_v1.Sha256DigestV1 `json:"failure_sha256"`
}

type CapabilityIDV1 string

type CapabilityResultStatusV1 string

const CapabilityResultStatusV1Failed CapabilityResultStatusV1 = "failed"
const CapabilityResultStatusV1Succeeded CapabilityResultStatusV1 = "succeeded"

type CapabilityResultV1 struct {
	// AttemptID corresponds to the JSON schema field "attempt_id".
	AttemptID CapabilityAttemptIDV1 `json:"attempt_id"`

	// CanonicalResultSha256 corresponds to the JSON schema field
	// "canonical_result_sha256".
	CanonicalResultSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_result_sha256"`

	// CompletedAt corresponds to the JSON schema field "completed_at".
	CompletedAt UnixTimeV1 `json:"completed_at"`

	// Failure corresponds to the JSON schema field "failure".
	Failure *CapabilityFailureV1 `json:"failure,omitempty,omitzero"`

	// ImplementationKind corresponds to the JSON schema field "implementation_kind".
	ImplementationKind BoundedResidentTextV1 `json:"implementation_kind"`

	// ImplementationVersion corresponds to the JSON schema field
	// "implementation_version".
	ImplementationVersion BoundedResidentTextV1 `json:"implementation_version"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// RequestBinding corresponds to the JSON schema field "request_binding".
	RequestBinding RequestBindingV1 `json:"request_binding"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// SigningAuthorityBinding corresponds to the JSON schema field
	// "signing_authority_binding".
	SigningAuthorityBinding SigningAuthorityBindingV1 `json:"signing_authority_binding"`

	// StartedAt corresponds to the JSON schema field "started_at".
	StartedAt UnixTimeV1 `json:"started_at"`

	// Status corresponds to the JSON schema field "status".
	Status CapabilityResultStatusV1 `json:"status"`

	// Success corresponds to the JSON schema field "success".
	Success *CapabilitySuccessV1 `json:"success,omitempty,omitzero"`
}

type CapabilitySuccessV1 struct {
	// ImmutableResultRef corresponds to the JSON schema field "immutable_result_ref".
	ImmutableResultRef *ResidentRecordRefV1 `json:"immutable_result_ref,omitempty,omitzero"`

	// InlinePayload corresponds to the JSON schema field "inline_payload".
	InlinePayload any `json:"inline_payload,omitempty,omitzero"`

	// ResultPayloadSha256 corresponds to the JSON schema field
	// "result_payload_sha256".
	ResultPayloadSha256 sha256_digest_v1.Sha256DigestV1 `json:"result_payload_sha256"`

	// ResultSchemaID corresponds to the JSON schema field "result_schema_id".
	ResultSchemaID SchemaID `json:"result_schema_id"`

	// ResultSchemaSha256 corresponds to the JSON schema field "result_schema_sha256".
	ResultSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"result_schema_sha256"`
}

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

type OwnerComponentIDV1 string

type OwnerSignerRecordSequenceV1 int

type RequestBindingV1 struct {
	// CanonicalRequestSha256 corresponds to the JSON schema field
	// "canonical_request_sha256".
	CanonicalRequestSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_request_sha256"`

	// CapabilityID corresponds to the JSON schema field "capability_id".
	CapabilityID CapabilityIDV1 `json:"capability_id"`

	// ContractSetRef corresponds to the JSON schema field "contract_set_ref".
	ContractSetRef ResidentRecordRefV1 `json:"contract_set_ref"`

	// ContractSetSha256 corresponds to the JSON schema field "contract_set_sha256".
	ContractSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"contract_set_sha256"`

	// CorrelationID corresponds to the JSON schema field "correlation_id".
	CorrelationID ResidentCorrelationIDV1 `json:"correlation_id"`

	// DeploymentBindingRef corresponds to the JSON schema field
	// "deployment_binding_ref".
	DeploymentBindingRef ResidentRecordRefV1 `json:"deployment_binding_ref"`

	// DeploymentBindingSha256 corresponds to the JSON schema field
	// "deployment_binding_sha256".
	DeploymentBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_binding_sha256"`

	// ExecutionOwnerFenceRef corresponds to the JSON schema field
	// "execution_owner_fence_ref".
	ExecutionOwnerFenceRef *ResidentRecordRefV1 `json:"execution_owner_fence_ref,omitempty,omitzero"`

	// ExecutionOwnerFenceSha256 corresponds to the JSON schema field
	// "execution_owner_fence_sha256".
	ExecutionOwnerFenceSha256 *sha256_digest_v1.Sha256DigestV1 `json:"execution_owner_fence_sha256,omitempty,omitzero"`

	// FederationRequestAdmissionRef corresponds to the JSON schema field
	// "federation_request_admission_ref".
	FederationRequestAdmissionRef *ResidentRecordRefV1 `json:"federation_request_admission_ref,omitempty,omitzero"`

	// FederationRequestAdmissionSha256 corresponds to the JSON schema field
	// "federation_request_admission_sha256".
	FederationRequestAdmissionSha256 *sha256_digest_v1.Sha256DigestV1 `json:"federation_request_admission_sha256,omitempty,omitzero"`

	// RequestID corresponds to the JSON schema field "request_id".
	RequestID ResidentRequestIDV1 `json:"request_id"`

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

	// SessionGrantID corresponds to the JSON schema field "session_grant_id".
	SessionGrantID SessionGrantIDV1 `json:"session_grant_id"`

	// SessionGrantSha256 corresponds to the JSON schema field "session_grant_sha256".
	SessionGrantSha256 sha256_digest_v1.Sha256DigestV1 `json:"session_grant_sha256"`
}

type ResidentCorrelationIDV1 string

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type ResidentRequestIDV1 string

type RouteGenerationV1 int

type SchemaID string

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
