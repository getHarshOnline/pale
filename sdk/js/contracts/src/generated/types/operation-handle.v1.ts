/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type UnixTimeV1 = number;
export type ResidentRecordRefV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type OperationClassV1 = "invoke" | "consult" | "delegate";
export type OperationHandleIdV1 = string;
export type CapabilityIdV1 = string;
export type ResidentCorrelationIdV1 = string;
export type ResidentRequestIdV1 = string;
export type SchemaId = string;
export type RouteGenerationV1 = number;
export type SessionGrantIdV1 = string;
export type CanonicalSignatureV1 = string;

export interface OperationHandleV1 {
  activation_fence_sha256: Sha256DigestV1;
  canonical_operation_handle_sha256: Sha256DigestV1;
  compiled_generation_sha256: Sha256DigestV1;
  created_at: UnixTimeV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  expires_at: UnixTimeV1;
  instance_sha256: Sha256DigestV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  operation_class: OperationClassV1;
  operation_handle_id: OperationHandleIdV1;
  operation_lease_ref: ResidentRecordRefV1;
  operation_lease_sha256: Sha256DigestV1;
  parent_operation?: ParentOperationReferenceV1;
  request_binding: RequestBindingV1;
  retain_until: UnixTimeV1;
  schema_id: "pale/operation-handle.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
}
export interface ParentOperationReferenceV1 {
  operation_handle_id: OperationHandleIdV1;
  operation_handle_ref: ResidentRecordRefV1;
  operation_handle_sha256: Sha256DigestV1;
}
export interface RequestBindingV1 {
  canonical_request_sha256: Sha256DigestV1;
  capability_id: CapabilityIdV1;
  contract_set_ref: ResidentRecordRefV1;
  contract_set_sha256: Sha256DigestV1;
  correlation_id: ResidentCorrelationIdV1;
  deployment_binding_ref: ResidentRecordRefV1;
  deployment_binding_sha256: Sha256DigestV1;
  execution_owner_fence_ref?: ResidentRecordRefV1;
  execution_owner_fence_sha256?: Sha256DigestV1;
  federation_request_admission_ref?: ResidentRecordRefV1;
  federation_request_admission_sha256?: Sha256DigestV1;
  request_id: ResidentRequestIdV1;
  route_binding_ref: ResidentRecordRefV1;
  route_binding_schema_id: SchemaId;
  route_binding_schema_sha256: Sha256DigestV1;
  route_binding_sha256: Sha256DigestV1;
  route_generation: RouteGenerationV1;
  session_grant_id: SessionGrantIdV1;
  session_grant_sha256: Sha256DigestV1;
}
