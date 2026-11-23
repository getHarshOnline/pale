/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type UnixTimeV1 = number;
export type ControlIncarnationIdV1 = string;
export type ControlStoreIdV1 = string;
export type DeploymentControlStoreGenesisIdV1 = string;
export type ControlCheckpointDomainIdV1 = string;
export type ControlDomainEpochV1 = number;
export type ControlDomainSequenceV1 = number;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type CanonicalSignatureV1 = string;
export type WorkspaceId = string;

export interface DeploymentControlStoreGenesisReceiptV1 {
  canonical_deployment_control_store_genesis_receipt_sha256: Sha256DigestV1;
  checkpoint_signing_authority_ref: ResidentRecordRefV1;
  checkpoint_signing_authority_sha256: Sha256DigestV1;
  committed_at: UnixTimeV1;
  control_incarnation_id: ControlIncarnationIdV1;
  control_store_id: ControlStoreIdV1;
  deployment_binding_ref: ResidentRecordRefV1;
  deployment_binding_sha256: Sha256DigestV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  genesis_id: DeploymentControlStoreGenesisIdV1;
  initial_control_checkpoints: PrincipalBootstrapInitialControlCheckpointV1[];
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  principal_id: ResidentPrincipalIdV1;
  schema_id: "pale/deployment-control-store-genesis-receipt.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  workspace_id: WorkspaceId;
}
export interface PrincipalBootstrapInitialControlCheckpointV1 {
  checkpoint_domain_id: ControlCheckpointDomainIdV1;
  control_checkpoint_ref: ResidentRecordRefV1;
  control_checkpoint_sha256: Sha256DigestV1;
  domain_epoch: ControlDomainEpochV1;
  domain_sequence: ControlDomainSequenceV1;
}
