/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type PrincipalBootstrapIdV1 = string;
export type ResidentKeyIdV1 = string;
export type Sha256DigestV1 = string;
export type UnixTimeV1 = number;
export type ResidentRecordRefV1 = string;
export type ControlCheckpointDomainIdV1 = string;
export type ControlDomainEpochV1 = number;
export type ControlDomainSequenceV1 = number;
export type ControlIncarnationIdV1 = string;
export type ControlStoreIdV1 = string;
export type CanonicalSignatureV1 = string;
export type PrincipalTrustCheckpointEpochV1 = number;

export interface PrincipalBootstrapReceiptV1 {
  bootstrap_id: PrincipalBootstrapIdV1;
  candidate_root_key_id: ResidentKeyIdV1;
  canonical_bootstrap_receipt_sha256: Sha256DigestV1;
  committed_at: UnixTimeV1;
  initial_admin_authority_ref: ResidentRecordRefV1;
  initial_admin_authority_sha256: Sha256DigestV1;
  initial_control_checkpoints: PrincipalBootstrapInitialControlCheckpointV1[];
  initial_producer_incarnation_authority_ref: ResidentRecordRefV1;
  initial_producer_incarnation_authority_sha256: Sha256DigestV1;
  initial_trust_store_checkpoint_signing_authority_ref: ResidentRecordRefV1;
  initial_trust_store_checkpoint_signing_authority_sha256: Sha256DigestV1;
  principal_control_incarnation_id: ControlIncarnationIdV1;
  principal_control_store_id: ControlStoreIdV1;
  principal_record_ref: ResidentRecordRefV1;
  principal_record_sha256: Sha256DigestV1;
  recovery_policy_ref: ResidentRecordRefV1;
  recovery_policy_sha256: Sha256DigestV1;
  schema_id: "pale/principal-bootstrap-receipt.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  trust_checkpoint_epoch: PrincipalTrustCheckpointEpochV1;
  trust_checkpoint_ref: ResidentRecordRefV1;
  trust_checkpoint_sha256: Sha256DigestV1;
  trust_store_id: ControlStoreIdV1;
  trust_store_incarnation_id: ControlIncarnationIdV1;
}
export interface PrincipalBootstrapInitialControlCheckpointV1 {
  checkpoint_domain_id: ControlCheckpointDomainIdV1;
  control_checkpoint_ref: ResidentRecordRefV1;
  control_checkpoint_sha256: Sha256DigestV1;
  domain_epoch: ControlDomainEpochV1;
  domain_sequence: ControlDomainSequenceV1;
}
