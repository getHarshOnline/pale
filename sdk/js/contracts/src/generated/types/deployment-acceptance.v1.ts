/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ResidentRecordRefV1 = string;
export type Sha256DigestV1 = string;
export type UnixTimeV1 = number;
export type DeploymentAcceptanceDecisionV1 = "accepted";
export type DeploymentAcceptanceIdV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type CanonicalSignatureV1 = string;

export interface DeploymentAcceptanceV1 {
  acceptance_authority_ref: ResidentRecordRefV1;
  acceptance_authority_sha256: Sha256DigestV1;
  acceptance_evidence_ref: ResidentRecordRefV1;
  acceptance_evidence_sha256: Sha256DigestV1;
  accepted_at: UnixTimeV1;
  binding_valid_until: UnixTimeV1;
  canonical_acceptance_sha256: Sha256DigestV1;
  decision: DeploymentAcceptanceDecisionV1;
  deployment_acceptance_id: DeploymentAcceptanceIdV1;
  deployment_proposal_ref: ResidentRecordRefV1;
  deployment_proposal_sha256: Sha256DigestV1;
  deployment_terms_sha256: Sha256DigestV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  permitted_binding_issuer_principal_id: ResidentPrincipalIdV1;
  schema_id: "pale/deployment-acceptance.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
}
