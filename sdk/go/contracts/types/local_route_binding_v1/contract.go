// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package local_route_binding_v1 contains the generated local-route-binding.v1 data model.
package local_route_binding_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type LocalRouteBindingV1 struct {
	// CanonicalLocalRouteBindingSha256 corresponds to the JSON schema field
	// "canonical_local_route_binding_sha256".
	CanonicalLocalRouteBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_local_route_binding_sha256"`

	// CheckedTrustCheckpointRef corresponds to the JSON schema field
	// "checked_trust_checkpoint_ref".
	CheckedTrustCheckpointRef ResidentRecordRefV1 `json:"checked_trust_checkpoint_ref"`

	// CheckedTrustCheckpointSha256 corresponds to the JSON schema field
	// "checked_trust_checkpoint_sha256".
	CheckedTrustCheckpointSha256 sha256_digest_v1.Sha256DigestV1 `json:"checked_trust_checkpoint_sha256"`

	// ContractSetRef corresponds to the JSON schema field "contract_set_ref".
	ContractSetRef ResidentRecordRefV1 `json:"contract_set_ref"`

	// ContractSetSha256 corresponds to the JSON schema field "contract_set_sha256".
	ContractSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"contract_set_sha256"`

	// DeploymentBindingRef corresponds to the JSON schema field
	// "deployment_binding_ref".
	DeploymentBindingRef ResidentRecordRefV1 `json:"deployment_binding_ref"`

	// DeploymentBindingSha256 corresponds to the JSON schema field
	// "deployment_binding_sha256".
	DeploymentBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_binding_sha256"`

	// DestinationNodeDelegationRef corresponds to the JSON schema field
	// "destination_node_delegation_ref".
	DestinationNodeDelegationRef *ResidentRecordRefV1 `json:"destination_node_delegation_ref,omitempty,omitzero"`

	// DestinationNodeDelegationSha256 corresponds to the JSON schema field
	// "destination_node_delegation_sha256".
	DestinationNodeDelegationSha256 *sha256_digest_v1.Sha256DigestV1 `json:"destination_node_delegation_sha256,omitempty,omitzero"`

	// DestinationNodeID corresponds to the JSON schema field "destination_node_id".
	DestinationNodeID *ResidentNodeIDV1 `json:"destination_node_id,omitempty,omitzero"`

	// DestinationPrincipalID corresponds to the JSON schema field
	// "destination_principal_id".
	DestinationPrincipalID ResidentPrincipalIDV1 `json:"destination_principal_id"`

	// EndpointBindingRef corresponds to the JSON schema field "endpoint_binding_ref".
	EndpointBindingRef ResidentRecordRefV1 `json:"endpoint_binding_ref"`

	// EndpointBindingSha256 corresponds to the JSON schema field
	// "endpoint_binding_sha256".
	EndpointBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"endpoint_binding_sha256"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt *UnixTimeV1 `json:"expires_at,omitempty,omitzero"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch PrincipalKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// OriginNodeDelegationRef corresponds to the JSON schema field
	// "origin_node_delegation_ref".
	OriginNodeDelegationRef ResidentRecordRefV1 `json:"origin_node_delegation_ref"`

	// OriginNodeDelegationSha256 corresponds to the JSON schema field
	// "origin_node_delegation_sha256".
	OriginNodeDelegationSha256 sha256_digest_v1.Sha256DigestV1 `json:"origin_node_delegation_sha256"`

	// OriginNodeID corresponds to the JSON schema field "origin_node_id".
	OriginNodeID ResidentNodeIDV1 `json:"origin_node_id"`

	// OriginPrincipalID corresponds to the JSON schema field "origin_principal_id".
	OriginPrincipalID ResidentPrincipalIDV1 `json:"origin_principal_id"`

	// PreviousRouteBindingRef corresponds to the JSON schema field
	// "previous_route_binding_ref".
	PreviousRouteBindingRef *ResidentRecordRefV1 `json:"previous_route_binding_ref,omitempty,omitzero"`

	// PreviousRouteBindingSha256 corresponds to the JSON schema field
	// "previous_route_binding_sha256".
	PreviousRouteBindingSha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_route_binding_sha256,omitempty,omitzero"`

	// RouteGeneration corresponds to the JSON schema field "route_generation".
	RouteGeneration RouteGenerationV1 `json:"route_generation"`

	// RouteID corresponds to the JSON schema field "route_id".
	RouteID LocalRouteIDV1 `json:"route_id"`

	// RouteOwnerPrincipalID corresponds to the JSON schema field
	// "route_owner_principal_id".
	RouteOwnerPrincipalID ResidentPrincipalIDV1 `json:"route_owner_principal_id"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// TransportClass corresponds to the JSON schema field "transport_class".
	TransportClass LocalTransportClassV1 `json:"transport_class"`

	// TransportProfileRef corresponds to the JSON schema field
	// "transport_profile_ref".
	TransportProfileRef ResidentRecordRefV1 `json:"transport_profile_ref"`

	// TransportProfileSha256 corresponds to the JSON schema field
	// "transport_profile_sha256".
	TransportProfileSha256 sha256_digest_v1.Sha256DigestV1 `json:"transport_profile_sha256"`

	// WorkspaceID corresponds to the JSON schema field "workspace_id".
	WorkspaceID ResidentWorkspaceIDV1 `json:"workspace_id"`
}

type LocalRouteIDV1 string

type LocalTransportClassV1 string

const LocalTransportClassV1InProcess LocalTransportClassV1 = "in_process"
const LocalTransportClassV1LoopbackHttp LocalTransportClassV1 = "loopback_http"
const LocalTransportClassV1NamedPipe LocalTransportClassV1 = "named_pipe"
const LocalTransportClassV1OwnerNative LocalTransportClassV1 = "owner_native"
const LocalTransportClassV1SamePrincipalRemote LocalTransportClassV1 = "same_principal_remote"
const LocalTransportClassV1UnixSocket LocalTransportClassV1 = "unix_socket"

type PrincipalKeyEpochV1 int

type ResidentKeyIDV1 string

type ResidentNodeIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type ResidentWorkspaceIDV1 string

type RouteGenerationV1 int

type SchemaID string

type UnixTimeV1 int
