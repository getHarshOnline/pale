/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type PrincipalRecoveryPolicyV1 = {
  canonical_record_sha256: Sha256DigestV1;
  /**
   * @minItems 1
   * @maxItems 1024
   */
  eligible_signers: [PrincipalRecoveryEligibleSignerV1, ...PrincipalRecoveryEligibleSignerV1[]];
  expires_at?: UnixTimeV1;
  issued_at: UnixTimeV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_key_epoch: PrincipalKeyEpochV1;
  policy_epoch: PrincipalRecoveryPolicyEpochV1;
  previous_policy_ref?: ResidentRecordRefV1;
  previous_policy_sha256?: Sha256DigestV1;
  principal_id: ResidentPrincipalIdV1;
  proof_validity_seconds: RecoveryProofValiditySecondsV1;
  recovery_policy_id: PrincipalRecoveryPolicyIdV1;
  replay_scope: PrincipalRecoveryReplayScopeV1;
  schema_id: "pale/principal-recovery-policy.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  threshold: number;
} & (
  | {
      policy_epoch?: 1;
      [k: string]: any;
    }
  | {
      policy_epoch?: {
        [k: string]: any;
      };
      [k: string]: any;
    }
);
export type Sha256DigestV1 = string;
export type PrincipalKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type Ed25519PublicKeyV1 = string;
export type UnixTimeV1 = number;
export type PrincipalRecoveryPolicyEpochV1 = number;
export type ResidentRecordRefV1 = string;
export type RecoveryProofValiditySecondsV1 = number;
export type PrincipalRecoveryPolicyIdV1 = string;
export type PrincipalRecoveryReplayScopeV1 = "principal_key_recovery_proof";
export type CanonicalSignatureV1 = string;

export interface PrincipalRecoveryEligibleSignerV1 {
  recovery_key_epoch: PrincipalKeyEpochV1;
  recovery_key_id: ResidentKeyIdV1;
  recovery_principal_id: ResidentPrincipalIdV1;
  recovery_public_key: Ed25519PublicKeyV1;
}
