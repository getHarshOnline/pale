/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type DeploymentAcceptanceRevocationIdV1 = string;
export type UnixTimeV1 = number;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type DeploymentAcceptanceRevocationEpochV1 = number;
export type ResidentReasonV1 = string;
export type CanonicalSignatureV1 = string;

export interface DeploymentAcceptanceRevocationV1 {
  canonical_deployment_acceptance_revocation_sha256: Sha256DigestV1;
  deployment_acceptance_ref: ResidentRecordRefV1;
  deployment_acceptance_revocation_id: DeploymentAcceptanceRevocationIdV1;
  deployment_acceptance_sha256: Sha256DigestV1;
  deployment_terms_sha256: Sha256DigestV1;
  effective_at: UnixTimeV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  previous_revocation_epoch?: DeploymentAcceptanceRevocationEpochV1;
  previous_revocation_ref?: ResidentRecordRefV1;
  previous_revocation_sha256?: Sha256DigestV1;
  reason_code: ResidentReasonV1;
  revocation_epoch: DeploymentAcceptanceRevocationEpochV1;
  schema_id: "pale/deployment-acceptance-revocation.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
}
