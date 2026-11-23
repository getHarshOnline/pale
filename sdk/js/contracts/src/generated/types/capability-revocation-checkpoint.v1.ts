/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ResidentPrincipalIdV1 = string;
export type CapabilityRevocationCheckpointEpochV1 = number;
export type CapabilityRevocationCheckpointIdV1 = string;
export type CoveredGrantCountV1 = number;
export type UnixTimeV1 = number;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentRecordRefV1 = string;
export type CanonicalSignatureV1 = string;

export interface CapabilityRevocationCheckpointV1 {
  authority_heads_root_sha256: Sha256DigestV1;
  authority_principal_id: ResidentPrincipalIdV1;
  canonical_checkpoint_sha256: Sha256DigestV1;
  checkpoint_epoch: CapabilityRevocationCheckpointEpochV1;
  checkpoint_id: CapabilityRevocationCheckpointIdV1;
  covered_grant_count: CoveredGrantCountV1;
  generated_at: UnixTimeV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  previous_checkpoint_ref?: ResidentRecordRefV1;
  previous_checkpoint_sha256?: Sha256DigestV1;
  schema_id: "pale/capability-revocation-checkpoint.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
}
