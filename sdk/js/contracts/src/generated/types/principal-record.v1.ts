/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type PrincipalRecordV1 = {
  canonical_record_sha256: Sha256DigestV1;
  display_hint: BoundedResidentTextV1;
  expires_at: UnixTimeV1;
  issued_at: UnixTimeV1;
  key_epoch: PrincipalKeyEpochV1;
  key_transition_ref?: ResidentRecordRefV1;
  key_transition_sha256?: Sha256DigestV1;
  previous_record_sha256?: Sha256DigestV1;
  principal_id: ResidentPrincipalIdV1;
  record_transition_kind: PrincipalRecordTransitionKindV1;
  recovery_policy_ref: ResidentRecordRefV1;
  recovery_policy_sha256: Sha256DigestV1;
  root_key_id: ResidentKeyIdV1;
  root_public_key: Ed25519PublicKeyV1;
  schema_id: "pale/principal-record.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
} & (
  | {
      record_transition_kind?: "initial";
      [k: string]: any;
    }
  | {
      record_transition_kind?: "key_transition";
      [k: string]: any;
    }
  | {
      record_transition_kind?: "policy_update";
      [k: string]: any;
    }
);
export type Sha256DigestV1 = string;
export type BoundedResidentTextV1 = string;
export type UnixTimeV1 = number;
export type PrincipalKeyEpochV1 = number;
export type ResidentRecordRefV1 = string;
export type ResidentPrincipalIdV1 = string;
export type PrincipalRecordTransitionKindV1 = "initial" | "key_transition" | "policy_update";
export type ResidentKeyIdV1 = string;
export type Ed25519PublicKeyV1 = string;
export type CanonicalSignatureV1 = string;
