/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type PrincipalKeyTransitionV1 = {
  canonical_record_sha256: Sha256DigestV1;
  effective_at: UnixTimeV1;
  new_key_epoch: PrincipalKeyEpochV1;
  new_key_id: ResidentKeyIdV1;
  new_key_signature: CanonicalSignatureV1;
  new_public_key: Ed25519PublicKeyV1;
  old_key_epoch: PrincipalKeyEpochV1;
  old_key_id: ResidentKeyIdV1;
  old_key_signature?: CanonicalSignatureV1;
  old_public_key: Ed25519PublicKeyV1;
  previous_record_sha256: Sha256DigestV1;
  principal_id: ResidentPrincipalIdV1;
  recovery_authority_proof_ref?: ResidentRecordRefV1;
  recovery_authority_proof_sha256?: Sha256DigestV1;
  schema_id: "pale/principal-key-transition.v1";
  schema_sha256: Sha256DigestV1;
  transition_mode: PrincipalKeyTransitionModeV1;
  transition_reason: BoundedResidentTextV1;
} & (
  | {
      transition_mode?: "normal_rotation";
      [k: string]: any;
    }
  | {
      transition_mode?: "recovery";
      [k: string]: any;
    }
);
export type Sha256DigestV1 = string;
export type UnixTimeV1 = number;
export type PrincipalKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type CanonicalSignatureV1 = string;
export type Ed25519PublicKeyV1 = string;
export type ResidentPrincipalIdV1 = string;
export type ResidentRecordRefV1 = string;
export type PrincipalKeyTransitionModeV1 = "normal_rotation" | "recovery";
export type BoundedResidentTextV1 = string;
