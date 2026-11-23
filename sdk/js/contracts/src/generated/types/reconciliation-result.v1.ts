/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ResidentRecordRefV1 = string;
export type Sha256DigestV1 = string;
export type ActionIdV1 = string;
export type ReconciliationCaseRevisionV1 = number;
export type ReconciliationCaseStateV1 = "matched" | "not_applied" | "drift" | "unknown";
export type UnixTimeV1 = number;
export type ResidentControllerIdV1 = string;
export type ReconciliationCustodyGenerationV1 = number;
export type TypedDigestCanonicalizationV1 = "rfc8785_json" | "raw_bytes";
export type TypedDigestContentBindingV1 =
  | {
      inline_json: any;
      kind: "inline_json";
    }
  | {
      inline_bytes_base64: string;
      kind: "inline_bytes_base64";
    }
  | {
      content_ref: ResidentRecordRefV1;
      content_ref_sha256: Sha256DigestV1;
      kind: "immutable_content";
    };
export type SchemaId = string;
export type TypedDigestAlgorithmV1 = "sha256";
export type BoundedResidentTextV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type CapabilityAttemptIdV1 = string;
export type ReconciliationIdV1 = string;
export type RelationshipScopeV1 = string;
export type CapabilityIdV1 = string;
export type ResidentCorrelationIdV1 = string;
export type ResidentRequestIdV1 = string;
export type RouteGenerationV1 = number;
export type SessionGrantIdV1 = string;
export type ReconciliationResolutionV1 =
  | "effect_confirmed"
  | "effect_absent"
  | "compensated"
  | "provider_state_accepted"
  | "expected_state_restored"
  | "manual_review_required"
  | "still_unknown";
export type CanonicalSignatureV1 = string;

export interface ReconciliationResultV1 {
  absence_evidence_ref?: ResidentRecordRefV1;
  absence_evidence_sha256?: Sha256DigestV1;
  action_id: ActionIdV1;
  canonical_reconciliation_sha256: Sha256DigestV1;
  case_ref: ResidentRecordRefV1;
  case_revision: ReconciliationCaseRevisionV1;
  case_sha256: Sha256DigestV1;
  case_state: ReconciliationCaseStateV1;
  closed_at?: UnixTimeV1;
  compensation?: ReconciliationCompensationBindingV1;
  controller_scope: ResidentControllerIdV1;
  custody_generation: ReconciliationCustodyGenerationV1;
  custody_ref: ResidentRecordRefV1;
  custody_sha256: Sha256DigestV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  effect_admission_ref: ResidentRecordRefV1;
  effect_admission_sha256: Sha256DigestV1;
  effect_receipts: ReconciliationEffectReceiptRefV1[];
  expected_state_digest: TypedDigestV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  known_attempt_ids: CapabilityAttemptIdV1[];
  observation_evidence: ReconciliationObservationEvidenceV1[];
  observed_state_digest?: TypedDigestV1;
  owner_evidence_ref?: ResidentRecordRefV1;
  owner_evidence_sha256?: Sha256DigestV1;
  predecessor_result_ref?: ResidentRecordRefV1;
  predecessor_result_sha256?: Sha256DigestV1;
  principal_scope: ResidentPrincipalIdV1;
  reconciliation_id: ReconciliationIdV1;
  reconciliation_policy_ref: ResidentRecordRefV1;
  reconciliation_policy_sha256: Sha256DigestV1;
  recorded_at: UnixTimeV1;
  relationship_scope?: RelationshipScopeV1;
  request_binding: RequestBindingV1;
  requested_at: UnixTimeV1;
  resolution?: ReconciliationResolutionV1;
  reviewer_or_policy_ref: ResidentRecordRefV1;
  reviewer_or_policy_sha256: Sha256DigestV1;
  schema_id: "pale/reconciliation-result.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
}
export interface ReconciliationCompensationBindingV1 {
  case_ref: ResidentRecordRefV1;
  case_sha256: Sha256DigestV1;
  compensation_action_ref: ResidentRecordRefV1;
  compensation_action_sha256: Sha256DigestV1;
  compensation_request_ref: ResidentRecordRefV1;
  compensation_request_sha256: Sha256DigestV1;
  compensation_result_ref: ResidentRecordRefV1;
  compensation_result_sha256: Sha256DigestV1;
  custody_ref: ResidentRecordRefV1;
  custody_sha256: Sha256DigestV1;
  effect_admission_ref: ResidentRecordRefV1;
  effect_admission_sha256: Sha256DigestV1;
  effect_receipts: ReconciliationEffectReceiptRefV1[];
}
export interface ReconciliationEffectReceiptRefV1 {
  receipt_ref: ResidentRecordRefV1;
  receipt_sha256: Sha256DigestV1;
}
export interface TypedDigestV1 {
  canonicalization: TypedDigestCanonicalizationV1;
  content_binding: TypedDigestContentBindingV1;
  content_schema_id: SchemaId;
  content_schema_sha256: Sha256DigestV1;
  digest: Sha256DigestV1;
  digest_algorithm: TypedDigestAlgorithmV1;
  kind: BoundedResidentTextV1;
}
export interface ReconciliationObservationEvidenceV1 {
  evidence_ref: ResidentRecordRefV1;
  evidence_sha256: Sha256DigestV1;
}
export interface RequestBindingV1 {
  canonical_request_sha256: Sha256DigestV1;
  capability_id: CapabilityIdV1;
  contract_set_ref: ResidentRecordRefV1;
  contract_set_sha256: Sha256DigestV1;
  correlation_id: ResidentCorrelationIdV1;
  deployment_binding_ref: ResidentRecordRefV1;
  deployment_binding_sha256: Sha256DigestV1;
  execution_owner_fence_ref?: ResidentRecordRefV1;
  execution_owner_fence_sha256?: Sha256DigestV1;
  federation_request_admission_ref?: ResidentRecordRefV1;
  federation_request_admission_sha256?: Sha256DigestV1;
  request_id: ResidentRequestIdV1;
  route_binding_ref: ResidentRecordRefV1;
  route_binding_schema_id: SchemaId;
  route_binding_schema_sha256: Sha256DigestV1;
  route_binding_sha256: Sha256DigestV1;
  route_generation: RouteGenerationV1;
  session_grant_id: SessionGrantIdV1;
  session_grant_sha256: Sha256DigestV1;
}
