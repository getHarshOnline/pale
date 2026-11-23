/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type PrincipalTrustCheckpointEpochV1 = number;
export type UnixTimeV1 = number;
export type PrincipalKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type ResidentRecordRefV1 = string;
export type ControlIncarnationIdV1 = string;
export type ControlStoreIdV1 = string;
export type PrincipalSessionFloorEpochV1 = number;
export type CanonicalSignatureV1 = string;
export type PrincipalTrustCheckpointIdV1 = string;

export interface PrincipalTrustCheckpointV1 {
  canonical_trust_checkpoint_sha256: Sha256DigestV1;
  checkpoint_epoch: PrincipalTrustCheckpointEpochV1;
  checkpoint_signing_authority_heads_root_sha256: Sha256DigestV1;
  deployment_acceptance_revocation_heads_root_sha256: Sha256DigestV1;
  deployment_runtime_authority_heads_root_sha256: Sha256DigestV1;
  federation_space_home_heads_root_sha256: Sha256DigestV1;
  issued_at: UnixTimeV1;
  issuer_key_epoch: PrincipalKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  local_admin_authority_ref: ResidentRecordRefV1;
  local_admin_authority_sha256: Sha256DigestV1;
  node_delegation_heads_root_sha256: Sha256DigestV1;
  node_delegation_revocation_heads_root_sha256: Sha256DigestV1;
  owner_record_signing_authority_heads_root_sha256: Sha256DigestV1;
  owner_signer_publication_frontier_heads_root_sha256: Sha256DigestV1;
  previous_trust_checkpoint_ref?: ResidentRecordRefV1;
  previous_trust_checkpoint_sha256?: Sha256DigestV1;
  principal_control_incarnation_id: ControlIncarnationIdV1;
  principal_control_store_id: ControlStoreIdV1;
  principal_id: ResidentPrincipalIdV1;
  principal_record_ref: ResidentRecordRefV1;
  principal_record_sha256: Sha256DigestV1;
  producer_incarnation_authority_ref: ResidentRecordRefV1;
  producer_incarnation_authority_sha256: Sha256DigestV1;
  producer_incarnation_heads_root_sha256: Sha256DigestV1;
  recovery_policy_ref: ResidentRecordRefV1;
  recovery_policy_sha256: Sha256DigestV1;
  schema_id: "pale/principal-trust-checkpoint.v1";
  schema_sha256: Sha256DigestV1;
  session_floor_epoch: PrincipalSessionFloorEpochV1;
  signature: CanonicalSignatureV1;
  trust_checkpoint_id: PrincipalTrustCheckpointIdV1;
  trust_store_id: ControlStoreIdV1;
  trust_store_incarnation_id: ControlIncarnationIdV1;
}
