/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type UnixTimeV1 = number;
export type ResidentBootIncarnationIdV1 = string;
export type Sha256DigestV1 = string;
export type PrincipalKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type ResidentRecordRefV1 = string;
export type ResidentNodeIdV1 = string;
export type PrincipalSessionFloorEpochV1 = number;
export type ResidentProcessInstanceIdV1 = string;
export type CanonicalSignatureV1 = string;
export type PrincipalResidentWriterFenceStateV1 = "active" | "retired";
export type ControlStoreIdV1 = string;
export type ControlIncarnationIdV1 = string;
export type PrincipalResidentWriterFenceIdV1 = string;
export type PrincipalResidentWriterGenerationV1 = number;

export interface PrincipalResidentWriterFenceV1 {
  acquired_at: UnixTimeV1;
  boot_incarnation_id: ResidentBootIncarnationIdV1;
  canonical_principal_resident_writer_fence_sha256: Sha256DigestV1;
  issuer_key_epoch: PrincipalKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  opened_deployment_store_heads_root_sha256: Sha256DigestV1;
  previous_writer_fence_ref?: ResidentRecordRefV1;
  previous_writer_fence_sha256?: Sha256DigestV1;
  primary_node_id: ResidentNodeIdV1;
  principal_id: ResidentPrincipalIdV1;
  principal_session_floor_epoch: PrincipalSessionFloorEpochV1;
  prior_terminal_receipt_ref?: ResidentRecordRefV1;
  prior_terminal_receipt_sha256?: Sha256DigestV1;
  process_instance_id: ResidentProcessInstanceIdV1;
  retired_at?: UnixTimeV1;
  schema_id: "pale/principal-resident-writer-fence.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  state: PrincipalResidentWriterFenceStateV1;
  takeover_receipt_ref?: ResidentRecordRefV1;
  takeover_receipt_sha256?: Sha256DigestV1;
  trust_store_id: ControlStoreIdV1;
  trust_store_incarnation_id: ControlIncarnationIdV1;
  writer_fence_id: PrincipalResidentWriterFenceIdV1;
  writer_generation: PrincipalResidentWriterGenerationV1;
}
