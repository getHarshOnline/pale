/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type UnixTimeV1 = number;
export type SessionGrantIdV1 = string;
export type ResidentRecordRefV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type CapabilityGrantRevocationEpochV1 = number;
export type ResidentReasonV1 = string;
export type CapabilityGrantRevocationIdV1 = string;
export type CanonicalSignatureV1 = string;

export interface CapabilitySessionGrantRevocationV1 {
  canonical_revocation_sha256: Sha256DigestV1;
  effective_at: UnixTimeV1;
  grant_id: SessionGrantIdV1;
  grant_ref: ResidentRecordRefV1;
  grant_sha256: Sha256DigestV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  previous_revocation_epoch?: CapabilityGrantRevocationEpochV1;
  previous_revocation_ref?: ResidentRecordRefV1;
  previous_revocation_sha256?: Sha256DigestV1;
  reason_code: ResidentReasonV1;
  revocation_epoch: CapabilityGrantRevocationEpochV1;
  revocation_id: CapabilityGrantRevocationIdV1;
  schema_id: "pale/capability-session-grant-revocation.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
}
