/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type CapabilityAttemptIdV1 = string;
export type UnixTimeV1 = number;
export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type A2AJsonRpcRequestIdV1 = string;
export type OperationHandleIdV1 = string;
export type A2ARemoteContextIdV1 = string;
export type A2ARemoteTaskIdV1 = string;
export type A2ATaskRequestModeV1 = "send_message" | "send_streaming_message";
export type A2ARouteAdmissionIdV1 = string;
export type CanonicalSignatureV1 = string;
export type A2ATaskBindingIdV1 = string;

export interface A2ATaskBindingV1 {
  attempt_id: CapabilityAttemptIdV1;
  bound_at: UnixTimeV1;
  canonical_a2a_task_binding_sha256: Sha256DigestV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  initial_task_evidence: ImmutableRecordReferenceV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  json_rpc_request_id: A2AJsonRpcRequestIdV1;
  operation_handle_id: OperationHandleIdV1;
  operation_handle_ref: ResidentRecordRefV1;
  operation_handle_sha256: Sha256DigestV1;
  operation_observation_ref: ResidentRecordRefV1;
  operation_observation_sha256: Sha256DigestV1;
  remote_context_id: A2ARemoteContextIdV1;
  remote_task_id: A2ARemoteTaskIdV1;
  request_evidence: ImmutableRecordReferenceV1;
  request_mode: A2ATaskRequestModeV1;
  route_admission_id: A2ARouteAdmissionIdV1;
  route_admission_ref: ResidentRecordRefV1;
  route_admission_sha256: Sha256DigestV1;
  schema_id: "pale/a2a-task-binding.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  task_binding_id: A2ATaskBindingIdV1;
}
export interface ImmutableRecordReferenceV1 {
  record_ref: ResidentRecordRefV1;
  record_sha256: Sha256DigestV1;
}
