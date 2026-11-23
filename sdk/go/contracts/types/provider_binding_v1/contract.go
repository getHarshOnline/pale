// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package provider_binding_v1 contains the generated provider-binding.v1 data model.
package provider_binding_v1

import composition_primitives_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/composition_primitives_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type AdapterPluginRefV1 struct {
	// AdapterPluginID corresponds to the JSON schema field "adapter_plugin_id".
	AdapterPluginID string `json:"adapter_plugin_id"`
}

type CapabilityEvidenceCoordinateV1 struct {
	// CapabilityID corresponds to the JSON schema field "capability_id".
	CapabilityID string `json:"capability_id"`

	// EvidenceContractID corresponds to the JSON schema field "evidence_contract_id".
	EvidenceContractID string `json:"evidence_contract_id"`
}

type CommunicationActorHandleV1 struct {
	// CanonicalOpaqueHandle corresponds to the JSON schema field
	// "canonical_opaque_handle".
	CanonicalOpaqueHandle string `json:"canonical_opaque_handle"`

	// DisplayForm corresponds to the JSON schema field "display_form".
	DisplayForm any `json:"display_form"`

	// EqualityScope corresponds to the JSON schema field "equality_scope".
	EqualityScope string `json:"equality_scope"`

	// LifecycleEvidenceContractID corresponds to the JSON schema field
	// "lifecycle_evidence_contract_id".
	LifecycleEvidenceContractID string `json:"lifecycle_evidence_contract_id"`

	// NamespaceID corresponds to the JSON schema field "namespace_id".
	NamespaceID string `json:"namespace_id"`

	// NativeAuthorshipVerifierID corresponds to the JSON schema field
	// "native_authorship_verifier_id".
	NativeAuthorshipVerifierID string `json:"native_authorship_verifier_id"`

	// ReferenceMappingID corresponds to the JSON schema field "reference_mapping_id".
	ReferenceMappingID string `json:"reference_mapping_id"`

	// SuccessionEvidenceContractID corresponds to the JSON schema field
	// "succession_evidence_contract_id".
	SuccessionEvidenceContractID string `json:"succession_evidence_contract_id"`
}

type ContractSchemaIdentityV1 struct {
	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`
}

type DeploymentActivationIdentityV1 struct {
	// DeploymentActivationSha256 corresponds to the JSON schema field
	// "deployment_activation_sha256".
	DeploymentActivationSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_activation_sha256"`
}

type LogicalProviderInstanceRefV1 struct {
	// LogicalProviderInstanceID corresponds to the JSON schema field
	// "logical_provider_instance_id".
	LogicalProviderInstanceID string `json:"logical_provider_instance_id"`
}

type NativeAuthorityOriginRefV1 struct {
	// NativeAuthorityOriginID corresponds to the JSON schema field
	// "native_authority_origin_id".
	NativeAuthorityOriginID string `json:"native_authority_origin_id"`
}

type NativeDataGenerationIdentityV1 struct {
	// NativeDataGenerationSha256 corresponds to the JSON schema field
	// "native_data_generation_sha256".
	NativeDataGenerationSha256 sha256_digest_v1.Sha256DigestV1 `json:"native_data_generation_sha256"`
}

type PaleAdmissionGenerationIdentityV1 struct {
	// PaleAdmissionGenerationSha256 corresponds to the JSON schema field
	// "pale_admission_generation_sha256".
	PaleAdmissionGenerationSha256 sha256_digest_v1.Sha256DigestV1 `json:"pale_admission_generation_sha256"`
}

type PhysicalDeploymentRefV1 struct {
	// PhysicalDeploymentID corresponds to the JSON schema field
	// "physical_deployment_id".
	PhysicalDeploymentID string `json:"physical_deployment_id"`
}

type ProviderArtifactCoordinatesV1 struct {
	// Build corresponds to the JSON schema field "build".
	Build string `json:"build"`

	// Executable corresponds to the JSON schema field "executable".
	Executable string `json:"executable"`

	// Image corresponds to the JSON schema field "image".
	Image string `json:"image"`

	// Package corresponds to the JSON schema field "package".
	Package string `json:"package"`

	// Source corresponds to the JSON schema field "source".
	Source string `json:"source"`
}

type ProviderAuthorityCoordinatesV1 struct {
	// Field corresponds to the JSON schema field "field".
	Field string `json:"field"`

	// Native corresponds to the JSON schema field "native".
	Native string `json:"native"`
}

type ProviderBehaviorCoordinatesV1 struct {
	// Degraded corresponds to the JSON schema field "degraded".
	Degraded string `json:"degraded"`

	// Removal corresponds to the JSON schema field "removal".
	Removal string `json:"removal"`
}

type ProviderBindingIdentityV1 struct {
	// ProviderBindingSha256 corresponds to the JSON schema field
	// "provider_binding_sha256".
	ProviderBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"provider_binding_sha256"`
}

type ProviderBindingV1 struct {
	// AdapterPlugin corresponds to the JSON schema field "adapter_plugin".
	AdapterPlugin AdapterPluginRefV1 `json:"adapter_plugin"`

	// ArtifactCoordinates corresponds to the JSON schema field
	// "artifact_coordinates".
	ArtifactCoordinates ProviderArtifactCoordinatesV1 `json:"artifact_coordinates"`

	// AuthorityCoordinates corresponds to the JSON schema field
	// "authority_coordinates".
	AuthorityCoordinates ProviderAuthorityCoordinatesV1 `json:"authority_coordinates"`

	// BehaviorCoordinates corresponds to the JSON schema field
	// "behavior_coordinates".
	BehaviorCoordinates ProviderBehaviorCoordinatesV1 `json:"behavior_coordinates"`

	// BindingIdentity corresponds to the JSON schema field "binding_identity".
	BindingIdentity ProviderBindingIdentityV1 `json:"binding_identity"`

	// CommunicationActorHandle corresponds to the JSON schema field
	// "communication_actor_handle".
	CommunicationActorHandle *CommunicationActorHandleV1 `json:"communication_actor_handle"`

	// ConformanceCoordinates corresponds to the JSON schema field
	// "conformance_coordinates".
	ConformanceCoordinates ProviderConformanceCoordinatesV1 `json:"conformance_coordinates"`

	// ExchangeCoordinates corresponds to the JSON schema field
	// "exchange_coordinates".
	ExchangeCoordinates ProviderExchangeCoordinatesV1 `json:"exchange_coordinates"`

	// GenerationCoordinates corresponds to the JSON schema field
	// "generation_coordinates".
	GenerationCoordinates ProviderGenerationCoordinatesV1 `json:"generation_coordinates"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// LifecycleCoordinates corresponds to the JSON schema field
	// "lifecycle_coordinates".
	LifecycleCoordinates ProviderLifecycleCoordinatesV1 `json:"lifecycle_coordinates"`

	// LogicalProviderInstance corresponds to the JSON schema field
	// "logical_provider_instance".
	LogicalProviderInstance LogicalProviderInstanceRefV1 `json:"logical_provider_instance"`

	// MappingCoordinates corresponds to the JSON schema field "mapping_coordinates".
	MappingCoordinates ProviderMappingCoordinatesV1 `json:"mapping_coordinates"`

	// NativeAuthorityOrigin corresponds to the JSON schema field
	// "native_authority_origin".
	NativeAuthorityOrigin NativeAuthorityOriginRefV1 `json:"native_authority_origin"`

	// Participant corresponds to the JSON schema field "participant".
	Participant composition_primitives_v1.ParticipantRefInterfaceV1 `json:"participant"`

	// PhysicalDeployment corresponds to the JSON schema field "physical_deployment".
	PhysicalDeployment PhysicalDeploymentRefV1 `json:"physical_deployment"`

	// ProtocolCoordinates corresponds to the JSON schema field
	// "protocol_coordinates".
	ProtocolCoordinates ProviderProtocolCoordinatesV1 `json:"protocol_coordinates"`

	// ProviderFamily corresponds to the JSON schema field "provider_family".
	ProviderFamily ProviderFamilyRefV1 `json:"provider_family"`

	// Responsibility corresponds to the JSON schema field "responsibility".
	Responsibility composition_primitives_v1.ResponsibilityCommunicationV1 `json:"responsibility"`

	// ResponsibilityContractIds corresponds to the JSON schema field
	// "responsibility_contract_ids".
	ResponsibilityContractIds []string `json:"responsibility_contract_ids"`

	// ResponsibilitySchemaIdentities corresponds to the JSON schema field
	// "responsibility_schema_identities".
	ResponsibilitySchemaIdentities []ContractSchemaIdentityV1 `json:"responsibility_schema_identities"`
}

type ProviderConformanceCoordinatesV1 struct {
	// Conformance corresponds to the JSON schema field "conformance".
	Conformance string `json:"conformance"`

	// Evidence corresponds to the JSON schema field "evidence".
	Evidence string `json:"evidence"`

	// PerCapabilityEvidence corresponds to the JSON schema field
	// "per_capability_evidence".
	PerCapabilityEvidence []CapabilityEvidenceCoordinateV1 `json:"per_capability_evidence"`
}

type ProviderExchangeCoordinatesV1 struct {
	// Cancel corresponds to the JSON schema field "cancel".
	Cancel string `json:"cancel"`

	// Cursor corresponds to the JSON schema field "cursor".
	Cursor string `json:"cursor"`

	// Drain corresponds to the JSON schema field "drain".
	Drain string `json:"drain"`

	// Outbox corresponds to the JSON schema field "outbox".
	Outbox string `json:"outbox"`

	// Request corresponds to the JSON schema field "request".
	Request string `json:"request"`

	// Result corresponds to the JSON schema field "result".
	Result string `json:"result"`
}

type ProviderFamilyRefV1 struct {
	// ProviderFamilyID corresponds to the JSON schema field "provider_family_id".
	ProviderFamilyID string `json:"provider_family_id"`
}

type ProviderGenerationCoordinatesV1 struct {
	// DeploymentActivationIdentity corresponds to the JSON schema field
	// "deployment_activation_identity".
	DeploymentActivationIdentity DeploymentActivationIdentityV1 `json:"deployment_activation_identity"`

	// NativeDataGenerationIdentity corresponds to the JSON schema field
	// "native_data_generation_identity".
	NativeDataGenerationIdentity NativeDataGenerationIdentityV1 `json:"native_data_generation_identity"`

	// PaleAdmissionGenerationIdentity corresponds to the JSON schema field
	// "pale_admission_generation_identity".
	PaleAdmissionGenerationIdentity PaleAdmissionGenerationIdentityV1 `json:"pale_admission_generation_identity"`
}

type ProviderLifecycleCoordinatesV1 struct {
	// Failure corresponds to the JSON schema field "failure".
	Failure string `json:"failure"`

	// Lifecycle corresponds to the JSON schema field "lifecycle".
	Lifecycle string `json:"lifecycle"`
}

type ProviderMappingCoordinatesV1 struct {
	// Entity corresponds to the JSON schema field "entity".
	Entity string `json:"entity"`

	// Event corresponds to the JSON schema field "event".
	Event string `json:"event"`

	// Field corresponds to the JSON schema field "field".
	Field string `json:"field"`

	// Terminology corresponds to the JSON schema field "terminology".
	Terminology string `json:"terminology"`
}

type ProviderProtocolCoordinatesV1 struct {
	// Configuration corresponds to the JSON schema field "configuration".
	Configuration string `json:"configuration"`

	// Extensions corresponds to the JSON schema field "extensions".
	Extensions []string `json:"extensions"`

	// Protocol corresponds to the JSON schema field "protocol".
	Protocol string `json:"protocol"`
}

type SchemaID string
