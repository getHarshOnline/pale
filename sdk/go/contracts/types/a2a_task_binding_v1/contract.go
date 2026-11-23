// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package a2a_task_binding_v1 contains the generated a2a-task-binding.v1 data model.
package a2a_task_binding_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type A2AJSONRpcRequestIDV1 string

type A2ARemoteContextIDV1 string

type A2ARemoteTaskIDV1 string

type A2ARouteAdmissionIDV1 string

type A2ATaskBindingIDV1 string

type A2ATaskBindingV1 struct {
	// AttemptID corresponds to the JSON schema field "attempt_id".
	AttemptID CapabilityAttemptIDV1 `json:"attempt_id"`

	// BoundAt corresponds to the JSON schema field "bound_at".
	BoundAt UnixTimeV1 `json:"bound_at"`

	// CanonicalA2ATaskBindingSha256 corresponds to the JSON schema field
	// "canonical_a2a_task_binding_sha256".
	CanonicalA2ATaskBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_a2a_task_binding_sha256"`

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`

	// InitialTaskEvidence corresponds to the JSON schema field
	// "initial_task_evidence".
	InitialTaskEvidence ImmutableRecordReferenceV1 `json:"initial_task_evidence"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// JSONRpcRequestID corresponds to the JSON schema field "json_rpc_request_id".
	JSONRpcRequestID A2AJSONRpcRequestIDV1 `json:"json_rpc_request_id"`

	// OperationHandleID corresponds to the JSON schema field "operation_handle_id".
	OperationHandleID OperationHandleIDV1 `json:"operation_handle_id"`

	// OperationHandleRef corresponds to the JSON schema field "operation_handle_ref".
	OperationHandleRef ResidentRecordRefV1 `json:"operation_handle_ref"`

	// OperationHandleSha256 corresponds to the JSON schema field
	// "operation_handle_sha256".
	OperationHandleSha256 sha256_digest_v1.Sha256DigestV1 `json:"operation_handle_sha256"`

	// OperationObservationRef corresponds to the JSON schema field
	// "operation_observation_ref".
	OperationObservationRef ResidentRecordRefV1 `json:"operation_observation_ref"`

	// OperationObservationSha256 corresponds to the JSON schema field
	// "operation_observation_sha256".
	OperationObservationSha256 sha256_digest_v1.Sha256DigestV1 `json:"operation_observation_sha256"`

	// RemoteContextID corresponds to the JSON schema field "remote_context_id".
	RemoteContextID A2ARemoteContextIDV1 `json:"remote_context_id"`

	// RemoteTaskID corresponds to the JSON schema field "remote_task_id".
	RemoteTaskID A2ARemoteTaskIDV1 `json:"remote_task_id"`

	// RequestEvidence corresponds to the JSON schema field "request_evidence".
	RequestEvidence ImmutableRecordReferenceV1 `json:"request_evidence"`

	// RequestMode corresponds to the JSON schema field "request_mode".
	RequestMode A2ATaskRequestModeV1 `json:"request_mode"`

	// RouteAdmissionID corresponds to the JSON schema field "route_admission_id".
	RouteAdmissionID A2ARouteAdmissionIDV1 `json:"route_admission_id"`

	// RouteAdmissionRef corresponds to the JSON schema field "route_admission_ref".
	RouteAdmissionRef ResidentRecordRefV1 `json:"route_admission_ref"`

	// RouteAdmissionSha256 corresponds to the JSON schema field
	// "route_admission_sha256".
	RouteAdmissionSha256 sha256_digest_v1.Sha256DigestV1 `json:"route_admission_sha256"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// TaskBindingID corresponds to the JSON schema field "task_binding_id".
	TaskBindingID A2ATaskBindingIDV1 `json:"task_binding_id"`
}

type A2ATaskRequestModeV1 string

const A2ATaskRequestModeV1SendMessage A2ATaskRequestModeV1 = "send_message"
const A2ATaskRequestModeV1SendStreamingMessage A2ATaskRequestModeV1 = "send_streaming_message"

type CanonicalSignatureV1 string

type CapabilityAttemptIDV1 string

type ImmutableRecordReferenceV1 struct {
	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSha256 corresponds to the JSON schema field "record_sha256".
	RecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_sha256"`
}

type OperationHandleIDV1 string

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int
