/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type ResidentNodeIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type UnixTimeV1 = number;
export type PrincipalKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type RouteGenerationV1 = number;
export type LocalRouteIdV1 = string;
export type CanonicalSignatureV1 = string;
export type LocalTransportClassV1 =
  "in_process" | "unix_socket" | "named_pipe" | "loopback_http" | "owner_native" | "same_principal_remote";
export type ResidentWorkspaceIdV1 = string;

export interface LocalRouteBindingV1 {
  canonical_local_route_binding_sha256: Sha256DigestV1;
  checked_trust_checkpoint_ref: ResidentRecordRefV1;
  checked_trust_checkpoint_sha256: Sha256DigestV1;
  contract_set_ref: ResidentRecordRefV1;
  contract_set_sha256: Sha256DigestV1;
  deployment_binding_ref: ResidentRecordRefV1;
  deployment_binding_sha256: Sha256DigestV1;
  destination_node_delegation_ref?: ResidentRecordRefV1;
  destination_node_delegation_sha256?: Sha256DigestV1;
  destination_node_id?: ResidentNodeIdV1;
  destination_principal_id: ResidentPrincipalIdV1;
  endpoint_binding_ref: ResidentRecordRefV1;
  endpoint_binding_sha256: Sha256DigestV1;
  expires_at?: UnixTimeV1;
  issued_at: UnixTimeV1;
  issuer_key_epoch: PrincipalKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  origin_node_delegation_ref: ResidentRecordRefV1;
  origin_node_delegation_sha256: Sha256DigestV1;
  origin_node_id: ResidentNodeIdV1;
  origin_principal_id: ResidentPrincipalIdV1;
  previous_route_binding_ref?: ResidentRecordRefV1;
  previous_route_binding_sha256?: Sha256DigestV1;
  route_generation: RouteGenerationV1;
  route_id: LocalRouteIdV1;
  route_owner_principal_id: ResidentPrincipalIdV1;
  schema_id: "pale/local-route-binding.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  transport_class: LocalTransportClassV1;
  transport_profile_ref: ResidentRecordRefV1;
  transport_profile_sha256: Sha256DigestV1;
  workspace_id: ResidentWorkspaceIdV1;
}
