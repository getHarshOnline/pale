/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type PrincipalKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type CanonicalSignatureV1 = string;
export type UnixTimeV1 = number;
export type PrincipalResidentWriterTerminalReceiptIdV1 = string;
export type PrincipalResidentWriterGenerationV1 = number;

export interface PrincipalResidentWriterTerminalReceiptV1 {
  canonical_principal_resident_writer_terminal_receipt_sha256: Sha256DigestV1;
  final_opened_store_heads_root_sha256: Sha256DigestV1;
  final_pending_custody_root_sha256: Sha256DigestV1;
  final_trust_checkpoint_ref: ResidentRecordRefV1;
  final_trust_checkpoint_sha256: Sha256DigestV1;
  issuer_key_epoch: PrincipalKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  principal_id: ResidentPrincipalIdV1;
  schema_id: "pale/principal-resident-writer-terminal-receipt.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  stopped_at: UnixTimeV1;
  terminal_receipt_id: PrincipalResidentWriterTerminalReceiptIdV1;
  writer_fence_ref: ResidentRecordRefV1;
  writer_fence_sha256: Sha256DigestV1;
  writer_generation: PrincipalResidentWriterGenerationV1;
}
