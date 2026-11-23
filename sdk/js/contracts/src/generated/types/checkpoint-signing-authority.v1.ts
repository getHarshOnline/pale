/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type CheckpointDomainPrefixV1 = string;
export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type CheckpointSigningAuthorityIdV1 = string;
export type ControlIncarnationIdV1 = string;
export type ControlStoreIdV1 = string;
export type UnixTimeV1 = number;
export type PrincipalKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type CheckpointSigningScopeV1 =
  | {
      bootstrap_id: PrincipalBootstrapIdV1;
      kind: "principal_trust_store";
    }
  | {
      deployment_binding_ref: ResidentRecordRefV1;
      deployment_binding_sha256: Sha256DigestV1;
      kind: "deployment";
    };
export type PrincipalBootstrapIdV1 = string;
export type CanonicalSignatureV1 = string;
export type CheckpointSignerEpochV1 = number;
export type PrincipalAuthorityStateV1 = "active" | "retired";
export type SigningKeyEpochV1 = number;
export type Ed25519PublicKeyV1 = string;

export interface CheckpointSigningAuthorityV1 {
  allowed_domain_prefixes: CheckpointDomainPrefixV1[];
  canonical_checkpoint_signing_authority_sha256: Sha256DigestV1;
  checkpoint_profile_ref: ResidentRecordRefV1;
  checkpoint_profile_sha256: Sha256DigestV1;
  checkpoint_signing_authority_id: CheckpointSigningAuthorityIdV1;
  control_incarnation_id: ControlIncarnationIdV1;
  control_store_id: ControlStoreIdV1;
  expires_at: UnixTimeV1;
  issued_at: UnixTimeV1;
  issuer_key_epoch: PrincipalKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  previous_authority_ref?: ResidentRecordRefV1;
  previous_authority_sha256?: Sha256DigestV1;
  principal_id: ResidentPrincipalIdV1;
  schema_id: "pale/checkpoint-signing-authority.v1";
  schema_sha256: Sha256DigestV1;
  scope: CheckpointSigningScopeV1;
  signature: CanonicalSignatureV1;
  signer_epoch: CheckpointSignerEpochV1;
  state: PrincipalAuthorityStateV1;
  subject_key_epoch: SigningKeyEpochV1;
  subject_key_id: ResidentKeyIdV1;
  subject_public_key: Ed25519PublicKeyV1;
}
