/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ResidentRecordRefV1 = string;
export type Sha256DigestV1 = string;
export type A2AHttpsEndpointV1 = string;
export type UnixTimeV1 = number;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type A2AProtocolVersionV1 = "1.0";
export type A2ARequestBindingV1 = "json_rpc2_over_verified_https";
export type ResidentNodeIdV1 = string;
export type SchemaId = string;
export type RouteGenerationV1 = number;
export type ResidentRouteIdV1 = string;
export type BoundedResidentTextV1 = string;
export type A2ARouteAdmissionIdV1 = string;
export type A2ASecuritySchemeIdV1 = string;
export type CanonicalSignatureV1 = string;
export type A2AStreamingAdmissionV1 =
  | {
      state: "not_advertised";
    }
  | {
      evidence: ImmutableRecordReferenceV1;
      state: "advertised";
    };

export interface A2ARouteAdmissionV1 {
  agent_card: ImmutableRecordReferenceV1;
  canonical_a2a_route_admission_sha256: Sha256DigestV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  endpoint: A2AHttpsEndpointV1;
  expires_at: UnixTimeV1;
  fresh_until: UnixTimeV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  observed_at: UnixTimeV1;
  protocol_version: A2AProtocolVersionV1;
  provider_binding: ProviderBindingRefV1;
  request_binding: A2ARequestBindingV1;
  route: CommonRequestRouteV1;
  route_admission_id: A2ARouteAdmissionIdV1;
  schema_id: "pale/a2a-route-admission.v1";
  schema_sha256: Sha256DigestV1;
  security: A2ASecurityAdmissionV1;
  signature: CanonicalSignatureV1;
  streaming: A2AStreamingAdmissionV1;
}
export interface ImmutableRecordReferenceV1 {
  record_ref: ResidentRecordRefV1;
  record_sha256: Sha256DigestV1;
}
export interface ProviderBindingRefV1 {
  provider_binding_id: string;
  provider_binding_identity: ProviderBindingIdentityV1;
}
export interface ProviderBindingIdentityV1 {
  provider_binding_sha256: Sha256DigestV1;
}
export interface CommonRequestRouteV1 {
  contract_set_ref: ResidentRecordRefV1;
  contract_set_sha256: Sha256DigestV1;
  destination_node_id?: ResidentNodeIdV1;
  destination_principal_id: ResidentPrincipalIdV1;
  origin_node_id: ResidentNodeIdV1;
  origin_principal_id: ResidentPrincipalIdV1;
  route_binding_ref: ResidentRecordRefV1;
  route_binding_schema_id: SchemaId;
  route_binding_schema_sha256: Sha256DigestV1;
  route_binding_sha256: Sha256DigestV1;
  route_generation: RouteGenerationV1;
  route_id: ResidentRouteIdV1;
  transport: BoundedResidentTextV1;
}
export interface A2ASecurityAdmissionV1 {
  evidence: ImmutableRecordReferenceV1;
  selected_scheme_ids: A2ASecuritySchemeIdV1[];
}
