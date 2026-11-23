/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ResidentRecordRefV1 = string;
export type Sha256DigestV1 = string;
export type UnixTimeV1 = number;
export type PrincipalKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type PrincipalSessionFloorEpochV1 = number;
export type PrincipalResidentWriterGenerationV1 = number;
export type CanonicalSignatureV1 = string;
export type ResidentBootIncarnationIdV1 = string;
export type ResidentNodeIdV1 = string;
export type ResidentProcessInstanceIdV1 = string;
export type PrincipalResidentWriterTakeoverNonceV1 = string;
export type PrincipalResidentWriterTakeoverReceiptIdV1 = string;

export interface PrincipalResidentWriterTakeoverReceiptV1 {
  accepted_trust_checkpoint_ref: ResidentRecordRefV1;
  accepted_trust_checkpoint_sha256: Sha256DigestV1;
  authorized_at: UnixTimeV1;
  canonical_principal_resident_writer_takeover_receipt_sha256: Sha256DigestV1;
  invalidated_route_lease_heads_root_sha256: Sha256DigestV1;
  issuer_key_epoch: PrincipalKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  new_session_floor_epoch: PrincipalSessionFloorEpochV1;
  predecessor_non_liveness_evidence_root_sha256: Sha256DigestV1;
  predecessor_writer_fence_ref: ResidentRecordRefV1;
  predecessor_writer_fence_sha256: Sha256DigestV1;
  predecessor_writer_generation: PrincipalResidentWriterGenerationV1;
  principal_id: ResidentPrincipalIdV1;
  prior_session_floor_epoch: PrincipalSessionFloorEpochV1;
  quarantined_custody_root_sha256: Sha256DigestV1;
  schema_id: "pale/principal-resident-writer-takeover-receipt.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  successor_boot_incarnation_id: ResidentBootIncarnationIdV1;
  successor_primary_node_id: ResidentNodeIdV1;
  successor_process_instance_id: ResidentProcessInstanceIdV1;
  takeover_nonce: PrincipalResidentWriterTakeoverNonceV1;
  takeover_receipt_id: PrincipalResidentWriterTakeoverReceiptIdV1;
  verified_store_heads_root_sha256: Sha256DigestV1;
}
