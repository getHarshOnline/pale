// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package a2a_route_admission_v1 contains the generated a2a-route-admission.v1 data model.
package a2a_route_admission_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type A2AHttpsEndpointV1 string

type A2AProtocolVersionV1 string

const A2AProtocolVersionV1A10 A2AProtocolVersionV1 = "1.0"

type A2ARequestBindingV1 string

const A2ARequestBindingV1JSONRpc2OverVerifiedHttps A2ARequestBindingV1 = "json_rpc2_over_verified_https"

type A2ARouteAdmissionIDV1 string

type A2ARouteAdmissionV1 struct {
	// AgentCard corresponds to the JSON schema field "agent_card".
	AgentCard ImmutableRecordReferenceV1 `json:"agent_card"`

	// CanonicalA2ARouteAdmissionSha256 corresponds to the JSON schema field
	// "canonical_a2a_route_admission_sha256".
	CanonicalA2ARouteAdmissionSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_a2a_route_admission_sha256"`

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`

	// Endpoint corresponds to the JSON schema field "endpoint".
	Endpoint A2AHttpsEndpointV1 `json:"endpoint"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt UnixTimeV1 `json:"expires_at"`

	// FreshUntil corresponds to the JSON schema field "fresh_until".
	FreshUntil UnixTimeV1 `json:"fresh_until"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// ObservedAt corresponds to the JSON schema field "observed_at".
	ObservedAt UnixTimeV1 `json:"observed_at"`

	// ProtocolVersion corresponds to the JSON schema field "protocol_version".
	ProtocolVersion A2AProtocolVersionV1 `json:"protocol_version"`

	// ProviderBinding corresponds to the JSON schema field "provider_binding".
	ProviderBinding ProviderBindingRefV1 `json:"provider_binding"`

	// RequestBinding corresponds to the JSON schema field "request_binding".
	RequestBinding A2ARequestBindingV1 `json:"request_binding"`

	// Route corresponds to the JSON schema field "route".
	Route CommonRequestRouteV1 `json:"route"`

	// RouteAdmissionID corresponds to the JSON schema field "route_admission_id".
	RouteAdmissionID A2ARouteAdmissionIDV1 `json:"route_admission_id"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Security corresponds to the JSON schema field "security".
	Security A2ASecurityAdmissionV1 `json:"security"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// Streaming corresponds to the JSON schema field "streaming".
	Streaming A2AStreamingAdmissionV1 `json:"streaming"`
}

type A2ASecurityAdmissionV1 struct {
	// Evidence corresponds to the JSON schema field "evidence".
	Evidence ImmutableRecordReferenceV1 `json:"evidence"`

	// SelectedSchemeIds corresponds to the JSON schema field "selected_scheme_ids".
	SelectedSchemeIds []A2ASecuritySchemeIDV1 `json:"selected_scheme_ids"`
}

type A2ASecuritySchemeIDV1 string

type A2AStreamingAdmissionAdvertisedV1 struct {
	// Evidence corresponds to the JSON schema field "evidence".
	Evidence ImmutableRecordReferenceV1 `json:"evidence"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type A2AStreamingAdmissionNotAdvertisedV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type BoundedResidentTextV1 string

type CanonicalSignatureV1 string

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

type ImmutableRecordReferenceV1 struct {
	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSha256 corresponds to the JSON schema field "record_sha256".
	RecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_sha256"`
}

type ProviderBindingIdentityV1 struct {
	// ProviderBindingSha256 corresponds to the JSON schema field
	// "provider_binding_sha256".
	ProviderBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"provider_binding_sha256"`
}

type ProviderBindingRefV1 struct {
	// ProviderBindingID corresponds to the JSON schema field "provider_binding_id".
	ProviderBindingID string `json:"provider_binding_id"`

	// ProviderBindingIdentity corresponds to the JSON schema field
	// "provider_binding_identity".
	ProviderBindingIdentity ProviderBindingIdentityV1 `json:"provider_binding_identity"`
}

type ResidentKeyIDV1 string

type ResidentNodeIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type ResidentRouteIDV1 string

type RouteGenerationV1 int

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int

type A2AStreamingAdmissionV1 struct {
	NotAdvertised *A2AStreamingAdmissionNotAdvertisedV1 `json:"-"`
	Advertised    *A2AStreamingAdmissionAdvertisedV1    `json:"-"`
}

func (value A2AStreamingAdmissionV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.NotAdvertised != nil {
		if value.NotAdvertised.State != "not_advertised" {
			return nil, fmt.Errorf("A2AStreamingAdmissionV1: selected variant has discriminator %q", value.NotAdvertised.State)
		}
		candidate, err := json.Marshal(value.NotAdvertised)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Advertised != nil {
		if value.Advertised.State != "advertised" {
			return nil, fmt.Errorf("A2AStreamingAdmissionV1: selected variant has discriminator %q", value.Advertised.State)
		}
		candidate, err := json.Marshal(value.Advertised)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("A2AStreamingAdmissionV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *A2AStreamingAdmissionV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		State string `json:"state"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = A2AStreamingAdmissionV1{}
	switch discriminator.State {
	case "not_advertised":
		var variant A2AStreamingAdmissionNotAdvertisedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NotAdvertised = &variant
	case "advertised":
		var variant A2AStreamingAdmissionAdvertisedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Advertised = &variant
	default:
		return fmt.Errorf("A2AStreamingAdmissionV1: unknown discriminator %q", discriminator.State)
	}
	return nil
}
