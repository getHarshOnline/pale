/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type CapabilityIdV1 = string;
export type ResidentRecordRefV1 = string;
export type Sha256DigestV1 = string;
export type ResidentNodeIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type SchemaId = string;
export type RouteGenerationV1 = number;
export type ResidentRouteIdV1 = string;
export type LocalTransportClassV1 =
  "in_process" | "unix_socket" | "named_pipe" | "loopback_http" | "owner_native" | "same_principal_remote";
export type DeploymentAcceptanceRevocationEpochV1 = number;
export type UnixTimeV1 = number;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type CapabilitySessionNonceV1 = string;
export type ResidentControllerIdV1 = string;
export type DataClassificationV1 = string;
export type DeploymentEnvironmentV1 = string;
export type PermittedPurposeV1 = string;
export type RelationshipScopeV1 = string;
export type PrincipalSessionFloorEpochV1 = number;
export type SessionGrantIdV1 = string;
export type CanonicalSignatureV1 = string;
export type RequestCallerIdV1 = string;
export type RequestCallerRoleV1 = string;
export type Ed25519PublicKeyV1 = string;
export type RequestCallerSurfaceV1 = string;

export interface CapabilitySessionGrantV1 {
  allowed_capability_ids: CapabilityIdV1[];
  allowed_route_constraints: CapabilitySessionRouteConstraintsV1;
  canonical_grant_sha256: Sha256DigestV1;
  checked_deployment_acceptance_revocation_epoch: DeploymentAcceptanceRevocationEpochV1;
  checked_principal_trust_checkpoint_ref: ResidentRecordRefV1;
  checked_principal_trust_checkpoint_sha256: Sha256DigestV1;
  deployment_binding_ref: ResidentRecordRefV1;
  deployment_binding_sha256: Sha256DigestV1;
  expires_at: UnixTimeV1;
  issued_at: UnixTimeV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  non_transferable: boolean;
  nonce: CapabilitySessionNonceV1;
  schema_id: "pale/capability-session-grant.v1";
  schema_sha256: Sha256DigestV1;
  scope_ceiling: CapabilitySessionScopeCeilingV1;
  session_floor_epoch: PrincipalSessionFloorEpochV1;
  session_grant_id: SessionGrantIdV1;
  signature: CanonicalSignatureV1;
  subject: CapabilitySessionSubjectV1;
}
export interface CapabilitySessionRouteConstraintsV1 {
  contract_sets: SessionContractSetConstraintV1[];
  destination_node_ids: ResidentNodeIdV1[];
  destination_principal_ids: ResidentPrincipalIdV1[];
  origin_node_id: ResidentNodeIdV1;
  origin_principal_id: ResidentPrincipalIdV1;
  routes: SessionRouteConstraintV1[];
  transport_classes: LocalTransportClassV1[];
}
export interface SessionContractSetConstraintV1 {
  contract_set_ref: ResidentRecordRefV1;
  contract_set_sha256: Sha256DigestV1;
}
export interface SessionRouteConstraintV1 {
  route_binding_ref: ResidentRecordRefV1;
  route_binding_schema_id: SchemaId;
  route_binding_sha256: Sha256DigestV1;
  route_generation: RouteGenerationV1;
  route_id: ResidentRouteIdV1;
}
export interface CapabilitySessionScopeCeilingV1 {
  authority_evidence: AuthorityEvidenceV1[];
  controller_scopes: ResidentControllerIdV1[];
  data_classification_ceiling: DataClassificationV1;
  environments: DeploymentEnvironmentV1[];
  principal_scopes: ResidentPrincipalIdV1[];
  purposes: PermittedPurposeV1[];
  relationship_scopes: RelationshipScopeV1[];
}
export interface AuthorityEvidenceV1 {
  authority_ref: ResidentRecordRefV1;
  authority_sha256: Sha256DigestV1;
}
export interface CapabilitySessionSubjectV1 {
  identity: RequestCallerIdV1;
  role: RequestCallerRoleV1;
  subject_key_epoch: SigningKeyEpochV1;
  subject_key_id: ResidentKeyIdV1;
  subject_public_key: Ed25519PublicKeyV1;
  surface: RequestCallerSurfaceV1;
}
