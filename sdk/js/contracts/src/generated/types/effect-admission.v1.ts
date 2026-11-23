/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ActionIdV1 = string;
export type ResidentRecordRefV1 = string;
export type SchemaId = string;
export type EffectAdmissionDecisionV1 = "admitted";
export type EffectAdmissionIdV1 = string;
export type UnixTimeV1 = number;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type ResidentRequestIdV1 = string;
export type BoundedResidentTextV1 = string;
export type CanonicalSignatureV1 = string;

export interface EffectAdmissionV1 {
  action_body_sha256: Sha256DigestV1;
  action_id: ActionIdV1;
  authority_evidence: AuthorityEvidenceV1[];
  canonical_effect_admission_sha256: Sha256DigestV1;
  canonical_request_sha256: Sha256DigestV1;
  conflict_claim_ref: ResidentRecordRefV1;
  conflict_claim_sha256: Sha256DigestV1;
  conflict_resource_key: any;
  conflict_resource_key_schema_id: SchemaId;
  conflict_resource_key_schema_sha256: Sha256DigestV1;
  decision: EffectAdmissionDecisionV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  effect_admission_id: EffectAdmissionIdV1;
  effect_policy: ResidentRecordRefV1;
  expires_at: UnixTimeV1;
  issued_at: UnixTimeV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  reconciliation_policy: ResidentRecordRefV1;
  request_id: ResidentRequestIdV1;
  risk_class: BoundedResidentTextV1;
  schema_id: "pale/effect-admission.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
}
export interface AuthorityEvidenceV1 {
  authority_ref: ResidentRecordRefV1;
  authority_sha256: Sha256DigestV1;
}
