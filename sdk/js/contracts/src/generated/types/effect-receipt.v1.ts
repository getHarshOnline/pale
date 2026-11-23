/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ResidentRecordRefV1 = string;
export type Sha256DigestV1 = string;
export type ActionIdV1 = string;
export type CapabilityAttemptIdV1 = string;
export type UnixTimeV1 = number;
export type ResidentControllerIdV1 = string;
export type BoundedResidentTextV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type ObservedEffectV1 = "confirmed" | "not_applied" | "unknown" | "partial";
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
export type ReconciliationCaseIdV1 = string;
export type RelationshipScopeV1 = string;
export type CapabilityIdV1 = string;
export type ResidentCorrelationIdV1 = string;
export type ResidentRequestIdV1 = string;
export type RouteGenerationV1 = number;
export type SessionGrantIdV1 = string;
export type CanonicalSignatureV1 = string;

export interface EffectReceiptV1 {
  absence_evidence_ref?: ResidentRecordRefV1;
  absence_evidence_sha256?: Sha256DigestV1;
  action_body_sha256: Sha256DigestV1;
  action_id: ActionIdV1;
  attempt_id: CapabilityAttemptIdV1;
  canonical_receipt_sha256: Sha256DigestV1;
  completed_at: UnixTimeV1;
  conflict_claim_ref: ResidentRecordRefV1;
  conflict_claim_sha256: Sha256DigestV1;
  controller_scope: ResidentControllerIdV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  effect_admission_ref: ResidentRecordRefV1;
  effect_admission_sha256: Sha256DigestV1;
  effect_dispatch_intent_ref: ResidentRecordRefV1;
  effect_dispatch_intent_sha256: Sha256DigestV1;
  external_reference?: ResidentRecordRefV1;
  implementation_kind: BoundedResidentTextV1;
  implementation_version: BoundedResidentTextV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  observed_effect: ObservedEffectV1;
  observed_postcondition_digest?: TypedDigestV1;
  principal_scope: ResidentPrincipalIdV1;
  provider_request_digest?: TypedDigestV1;
  reconciliation_case_id?: ReconciliationCaseIdV1;
  reconciliation_required: boolean;
  relationship_scope?: RelationshipScopeV1;
  request_binding: RequestBindingV1;
  response_digest?: TypedDigestV1;
  schema_id: "pale/effect-receipt.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  started_at: UnixTimeV1;
  transport_status: BoundedResidentTextV1;
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
