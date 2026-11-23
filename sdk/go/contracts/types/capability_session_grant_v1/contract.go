// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package capability_session_grant_v1 contains the generated capability-session-grant.v1 data model.
package capability_session_grant_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type AuthorityEvidenceV1 struct {
	// AuthorityRef corresponds to the JSON schema field "authority_ref".
	AuthorityRef ResidentRecordRefV1 `json:"authority_ref"`

	// AuthoritySha256 corresponds to the JSON schema field "authority_sha256".
	AuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"authority_sha256"`
}

type CanonicalSignatureV1 string

type CapabilityIDV1 string

type CapabilitySessionGrantV1 struct {
	// AllowedCapabilityIds corresponds to the JSON schema field
	// "allowed_capability_ids".
	AllowedCapabilityIds []CapabilityIDV1 `json:"allowed_capability_ids"`

	// AllowedRouteConstraints corresponds to the JSON schema field
	// "allowed_route_constraints".
	AllowedRouteConstraints CapabilitySessionRouteConstraintsV1 `json:"allowed_route_constraints"`

	// CanonicalGrantSha256 corresponds to the JSON schema field
	// "canonical_grant_sha256".
	CanonicalGrantSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_grant_sha256"`

	// CheckedDeploymentAcceptanceRevocationEpoch corresponds to the JSON schema field
	// "checked_deployment_acceptance_revocation_epoch".
	CheckedDeploymentAcceptanceRevocationEpoch DeploymentAcceptanceRevocationEpochV1 `json:"checked_deployment_acceptance_revocation_epoch"`

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

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt UnixTimeV1 `json:"expires_at"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// NonTransferable corresponds to the JSON schema field "non_transferable".
	NonTransferable bool `json:"non_transferable"`

	// Nonce corresponds to the JSON schema field "nonce".
	Nonce CapabilitySessionNonceV1 `json:"nonce"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// ScopeCeiling corresponds to the JSON schema field "scope_ceiling".
	ScopeCeiling CapabilitySessionScopeCeilingV1 `json:"scope_ceiling"`

	// SessionFloorEpoch corresponds to the JSON schema field "session_floor_epoch".
	SessionFloorEpoch PrincipalSessionFloorEpochV1 `json:"session_floor_epoch"`

	// SessionGrantID corresponds to the JSON schema field "session_grant_id".
	SessionGrantID SessionGrantIDV1 `json:"session_grant_id"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// Subject corresponds to the JSON schema field "subject".
	Subject CapabilitySessionSubjectV1 `json:"subject"`
}

type CapabilitySessionNonceV1 string

type CapabilitySessionRouteConstraintsV1 struct {
	// ContractSets corresponds to the JSON schema field "contract_sets".
	ContractSets []SessionContractSetConstraintV1 `json:"contract_sets"`

	// DestinationNodeIds corresponds to the JSON schema field "destination_node_ids".
	DestinationNodeIds []ResidentNodeIDV1 `json:"destination_node_ids"`

	// DestinationPrincipalIds corresponds to the JSON schema field
	// "destination_principal_ids".
	DestinationPrincipalIds []ResidentPrincipalIDV1 `json:"destination_principal_ids"`

	// OriginNodeID corresponds to the JSON schema field "origin_node_id".
	OriginNodeID ResidentNodeIDV1 `json:"origin_node_id"`

	// OriginPrincipalID corresponds to the JSON schema field "origin_principal_id".
	OriginPrincipalID ResidentPrincipalIDV1 `json:"origin_principal_id"`

	// Routes corresponds to the JSON schema field "routes".
	Routes []SessionRouteConstraintV1 `json:"routes"`

	// TransportClasses corresponds to the JSON schema field "transport_classes".
	TransportClasses []LocalTransportClassV1 `json:"transport_classes"`
}

type CapabilitySessionScopeCeilingV1 struct {
	// AuthorityEvidence corresponds to the JSON schema field "authority_evidence".
	AuthorityEvidence []AuthorityEvidenceV1 `json:"authority_evidence"`

	// ControllerScopes corresponds to the JSON schema field "controller_scopes".
	ControllerScopes []ResidentControllerIDV1 `json:"controller_scopes"`

	// DataClassificationCeiling corresponds to the JSON schema field
	// "data_classification_ceiling".
	DataClassificationCeiling DataClassificationV1 `json:"data_classification_ceiling"`

	// Environments corresponds to the JSON schema field "environments".
	Environments []DeploymentEnvironmentV1 `json:"environments"`

	// PrincipalScopes corresponds to the JSON schema field "principal_scopes".
	PrincipalScopes []ResidentPrincipalIDV1 `json:"principal_scopes"`

	// Purposes corresponds to the JSON schema field "purposes".
	Purposes []PermittedPurposeV1 `json:"purposes"`

	// RelationshipScopes corresponds to the JSON schema field "relationship_scopes".
	RelationshipScopes []RelationshipScopeV1 `json:"relationship_scopes"`
}

type CapabilitySessionSubjectV1 struct {
	// Identity corresponds to the JSON schema field "identity".
	Identity RequestCallerIDV1 `json:"identity"`

	// Role corresponds to the JSON schema field "role".
	Role RequestCallerRoleV1 `json:"role"`

	// SubjectKeyEpoch corresponds to the JSON schema field "subject_key_epoch".
	SubjectKeyEpoch SigningKeyEpochV1 `json:"subject_key_epoch"`

	// SubjectKeyID corresponds to the JSON schema field "subject_key_id".
	SubjectKeyID ResidentKeyIDV1 `json:"subject_key_id"`

	// SubjectPublicKey corresponds to the JSON schema field "subject_public_key".
	SubjectPublicKey Ed25519PublicKeyV1 `json:"subject_public_key"`

	// Surface corresponds to the JSON schema field "surface".
	Surface RequestCallerSurfaceV1 `json:"surface"`
}

type DataClassificationV1 string

type DeploymentAcceptanceRevocationEpochV1 int

type DeploymentEnvironmentV1 string

type Ed25519PublicKeyV1 string

type LocalTransportClassV1 string

const LocalTransportClassV1InProcess LocalTransportClassV1 = "in_process"
const LocalTransportClassV1LoopbackHttp LocalTransportClassV1 = "loopback_http"
const LocalTransportClassV1NamedPipe LocalTransportClassV1 = "named_pipe"
const LocalTransportClassV1OwnerNative LocalTransportClassV1 = "owner_native"
const LocalTransportClassV1SamePrincipalRemote LocalTransportClassV1 = "same_principal_remote"
const LocalTransportClassV1UnixSocket LocalTransportClassV1 = "unix_socket"

type PermittedPurposeV1 string

type PrincipalSessionFloorEpochV1 int

type RelationshipScopeV1 string

type RequestCallerIDV1 string

type RequestCallerRoleV1 string

type RequestCallerSurfaceV1 string

type ResidentControllerIDV1 string

type ResidentKeyIDV1 string

type ResidentNodeIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type ResidentRouteIDV1 string

type RouteGenerationV1 int

type SchemaID string

type SessionContractSetConstraintV1 struct {
	// ContractSetRef corresponds to the JSON schema field "contract_set_ref".
	ContractSetRef ResidentRecordRefV1 `json:"contract_set_ref"`

	// ContractSetSha256 corresponds to the JSON schema field "contract_set_sha256".
	ContractSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"contract_set_sha256"`
}

type SessionGrantIDV1 string

type SessionRouteConstraintV1 struct {
	// RouteBindingRef corresponds to the JSON schema field "route_binding_ref".
	RouteBindingRef ResidentRecordRefV1 `json:"route_binding_ref"`

	// RouteBindingSchemaID corresponds to the JSON schema field
	// "route_binding_schema_id".
	RouteBindingSchemaID SchemaID `json:"route_binding_schema_id"`

	// RouteBindingSha256 corresponds to the JSON schema field "route_binding_sha256".
	RouteBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"route_binding_sha256"`

	// RouteGeneration corresponds to the JSON schema field "route_generation".
	RouteGeneration RouteGenerationV1 `json:"route_generation"`

	// RouteID corresponds to the JSON schema field "route_id".
	RouteID ResidentRouteIDV1 `json:"route_id"`
}

type SigningKeyEpochV1 int

type UnixTimeV1 int
