// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package operation_handle_v1 contains the generated operation-handle.v1 data model.
package operation_handle_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type CapabilityIDV1 string

type OperationClassV1 string

const OperationClassV1Consult OperationClassV1 = "consult"
const OperationClassV1Delegate OperationClassV1 = "delegate"
const OperationClassV1Invoke OperationClassV1 = "invoke"

type OperationHandleIDV1 string

type OperationHandleV1 struct {
	// ActivationFenceSha256 corresponds to the JSON schema field
	// "activation_fence_sha256".
	ActivationFenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"activation_fence_sha256"`

	// CanonicalOperationHandleSha256 corresponds to the JSON schema field
	// "canonical_operation_handle_sha256".
	CanonicalOperationHandleSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_operation_handle_sha256"`

	// CompiledGenerationSha256 corresponds to the JSON schema field
	// "compiled_generation_sha256".
	CompiledGenerationSha256 sha256_digest_v1.Sha256DigestV1 `json:"compiled_generation_sha256"`

	// CreatedAt corresponds to the JSON schema field "created_at".
	CreatedAt UnixTimeV1 `json:"created_at"`

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt UnixTimeV1 `json:"expires_at"`

	// InstanceSha256 corresponds to the JSON schema field "instance_sha256".
	InstanceSha256 sha256_digest_v1.Sha256DigestV1 `json:"instance_sha256"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// OperationClass corresponds to the JSON schema field "operation_class".
	OperationClass OperationClassV1 `json:"operation_class"`

	// OperationHandleID corresponds to the JSON schema field "operation_handle_id".
	OperationHandleID OperationHandleIDV1 `json:"operation_handle_id"`

	// OperationLeaseRef corresponds to the JSON schema field "operation_lease_ref".
	OperationLeaseRef ResidentRecordRefV1 `json:"operation_lease_ref"`

	// OperationLeaseSha256 corresponds to the JSON schema field
	// "operation_lease_sha256".
	OperationLeaseSha256 sha256_digest_v1.Sha256DigestV1 `json:"operation_lease_sha256"`

	// ParentOperation corresponds to the JSON schema field "parent_operation".
	ParentOperation *ParentOperationReferenceV1 `json:"parent_operation,omitempty,omitzero"`

	// RequestBinding corresponds to the JSON schema field "request_binding".
	RequestBinding RequestBindingV1 `json:"request_binding"`

	// RetainUntil corresponds to the JSON schema field "retain_until".
	RetainUntil UnixTimeV1 `json:"retain_until"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`
}

type ParentOperationReferenceV1 struct {
	// OperationHandleID corresponds to the JSON schema field "operation_handle_id".
	OperationHandleID OperationHandleIDV1 `json:"operation_handle_id"`

	// OperationHandleRef corresponds to the JSON schema field "operation_handle_ref".
	OperationHandleRef ResidentRecordRefV1 `json:"operation_handle_ref"`

	// OperationHandleSha256 corresponds to the JSON schema field
	// "operation_handle_sha256".
	OperationHandleSha256 sha256_digest_v1.Sha256DigestV1 `json:"operation_handle_sha256"`
}

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

type SigningKeyEpochV1 int

type UnixTimeV1 int
