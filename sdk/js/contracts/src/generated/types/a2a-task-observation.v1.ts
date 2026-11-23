/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type UnixTimeV1 = number;
export type A2ARemoteTaskStateV1 =
  | "unspecified"
  | "submitted"
  | "working"
  | "input_required"
  | "auth_required"
  | "completed"
  | "canceled"
  | "failed"
  | "rejected";
export type A2ATaskObservationRevisionV1 = number;
export type CanonicalSignatureV1 = string;
export type A2ATaskObservationSourceV1 =
  | {
      evidence: ImmutableRecordReferenceV1;
      source: "initial_send_message_response";
    }
  | {
      event_id?: A2ASseEventIdV1 | null;
      evidence: ImmutableRecordReferenceV1;
      source: "initial_streaming_event";
    }
  | {
      event_id?: A2ASseEventIdV1 | null;
      evidence: ImmutableRecordReferenceV1;
      source: "streaming_event";
    }
  | {
      request: ImmutableRecordReferenceV1;
      response: ImmutableRecordReferenceV1;
      source: "get_task";
    }
  | {
      request: ImmutableRecordReferenceV1;
      response: ImmutableRecordReferenceV1;
      source: "cancel_task_read_back";
    };
export type A2ASseEventIdV1 = string;
export type A2ATaskBindingIdV1 = string;

export interface A2ATaskObservationV1 {
  canonical_a2a_task_observation_sha256: Sha256DigestV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  observed_at: UnixTimeV1;
  predecessor_ref?: ResidentRecordRefV1;
  predecessor_sha256?: Sha256DigestV1;
  reconciliation?: ReconciliationFrontierReferenceV1;
  remote_state: A2ARemoteTaskStateV1;
  revision: A2ATaskObservationRevisionV1;
  schema_id: "pale/a2a-task-observation.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  source: A2ATaskObservationSourceV1;
  task_binding_id: A2ATaskBindingIdV1;
  task_binding_ref: ResidentRecordRefV1;
  task_binding_sha256: Sha256DigestV1;
}
export interface ReconciliationFrontierReferenceV1 {
  record_ref: ResidentRecordRefV1;
  record_sha256: Sha256DigestV1;
}
export interface ImmutableRecordReferenceV1 {
  record_ref: ResidentRecordRefV1;
  record_sha256: Sha256DigestV1;
}
