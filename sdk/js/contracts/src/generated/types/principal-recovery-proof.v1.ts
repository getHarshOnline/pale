/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type UnixTimeV1 = number;
export type PrincipalKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type Ed25519PublicKeyV1 = string;
export type PrincipalRecoveryNonceV1 = string;
export type ResidentPrincipalIdV1 = string;
export type ResidentRecordRefV1 = string;
export type PrincipalRecoveryProofIdV1 = string;
export type CanonicalSignatureV1 = string;

export interface PrincipalRecoveryProofV1 {
  canonical_record_sha256: Sha256DigestV1;
  expires_at: UnixTimeV1;
  issued_at: UnixTimeV1;
  new_key_epoch: PrincipalKeyEpochV1;
  new_key_id: ResidentKeyIdV1;
  new_public_key: Ed25519PublicKeyV1;
  nonce: PrincipalRecoveryNonceV1;
  principal_id: ResidentPrincipalIdV1;
  prior_principal_record_ref: ResidentRecordRefV1;
  prior_principal_record_sha256: Sha256DigestV1;
  recovery_policy_ref: ResidentRecordRefV1;
  recovery_policy_sha256: Sha256DigestV1;
  recovery_proof_id: PrincipalRecoveryProofIdV1;
  schema_id: "pale/principal-recovery-proof.v1";
  schema_sha256: Sha256DigestV1;
  /**
   * @minItems 1
   * @maxItems 1024
   */
  signatures: [PrincipalRecoverySignatureV1, ...PrincipalRecoverySignatureV1[]];
}
export interface PrincipalRecoverySignatureV1 {
  signature: CanonicalSignatureV1;
  signed_record_sha256: Sha256DigestV1;
  signer_key_epoch: PrincipalKeyEpochV1;
  signer_key_id: ResidentKeyIdV1;
  signer_principal_id: ResidentPrincipalIdV1;
}
