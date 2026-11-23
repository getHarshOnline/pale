/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ControlLogicalKeyV1 = string;
export type ControlRecordClassV1 = string;
export type ControlCheckpointDomainIdV1 = string;
export type ResidentRecordRefV1 = string;
export type ControlTransactionSequenceV1 = number;
export type ControlCheckpointIdV1 = string;
export type ControlIncarnationIdV1 = string;
export type ControlStoreIdV1 = string;
export type ControlDomainEpochV1 = number;
export type ControlDomainSequenceV1 = number;
export type UnixTimeV1 = number;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type ControlLeafCountV1 = number;
export type CanonicalSignatureV1 = string;

export interface ControlStateCheckpointV1 {
  canonical_control_checkpoint_sha256: Sha256DigestV1;
  changed_leaf_keys: ChangedControlLeafKeyV1[];
  checkpoint_domain_id: ControlCheckpointDomainIdV1;
  checkpoint_profile_ref: ResidentRecordRefV1;
  checkpoint_profile_sha256: Sha256DigestV1;
  checkpoint_signing_authority_ref: ResidentRecordRefV1;
  checkpoint_signing_authority_sha256: Sha256DigestV1;
  committed_transaction_sequence: ControlTransactionSequenceV1;
  control_checkpoint_id: ControlCheckpointIdV1;
  control_incarnation_id: ControlIncarnationIdV1;
  control_schema_version: number;
  control_store_id: ControlStoreIdV1;
  domain_epoch: ControlDomainEpochV1;
  domain_sequence: ControlDomainSequenceV1;
  issued_at: UnixTimeV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  leaf_count: ControlLeafCountV1;
  previous_control_checkpoint_ref?: ResidentRecordRefV1;
  previous_control_checkpoint_sha256?: Sha256DigestV1;
  schema_id: "pale/control-state-checkpoint.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  state_root_sha256: Sha256DigestV1;
  unresolved_custody_root_sha256: Sha256DigestV1;
}
export interface ChangedControlLeafKeyV1 {
  logical_key: ControlLogicalKeyV1;
  record_class: ControlRecordClassV1;
  scope_key_sha256: Sha256DigestV1;
}
