/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ActionIdV1 = string;
export type BusinessIdempotencyKeyV1 = string;
export type ResidentRecordRefV1 = string;
export type EffectConflictClaimGenerationV1 = number;
export type EffectConflictClaimStateV1 = "active" | "released" | "transferred";
export type EffectConflictClaimIdV1 = string;
export type SchemaId = string;
export type ResidentControllerIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type UnixTimeV1 = number;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type RelationshipScopeV1 = string;
export type ResidentRequestIdV1 = string;
export type CanonicalSignatureV1 = string;
export type EffectConflictTerminalReasonV1 =
  "confirmed" | "not_applied" | "cancelled_before_dispatch" | "reconciliation_custody";

export interface EffectConflictClaimV1 {
  action_body_sha256: Sha256DigestV1;
  action_id: ActionIdV1;
  business_idempotency_key: BusinessIdempotencyKeyV1;
  cancellation_authority_ref?: ResidentRecordRefV1;
  cancellation_authority_sha256?: Sha256DigestV1;
  canonical_effect_conflict_claim_sha256: Sha256DigestV1;
  canonical_request_sha256: Sha256DigestV1;
  claim_generation: EffectConflictClaimGenerationV1;
  claim_state: EffectConflictClaimStateV1;
  conflict_claim_id: EffectConflictClaimIdV1;
  conflict_resource_key: any;
  conflict_resource_key_schema_id: SchemaId;
  conflict_resource_key_schema_sha256: Sha256DigestV1;
  controller_scope: ResidentControllerIdV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  effect_receipt_ref?: ResidentRecordRefV1;
  effect_receipt_sha256?: Sha256DigestV1;
  holder_principal_id: ResidentPrincipalIdV1;
  issued_at: UnixTimeV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  previous_claim_ref?: ResidentRecordRefV1;
  previous_claim_sha256?: Sha256DigestV1;
  principal_scope: ResidentPrincipalIdV1;
  reconciliation_custody_ref?: ResidentRecordRefV1;
  reconciliation_custody_sha256?: Sha256DigestV1;
  relationship_scope?: RelationshipScopeV1;
  request_id: ResidentRequestIdV1;
  schema_id: "pale/effect-conflict-claim.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  terminal_reason?: EffectConflictTerminalReasonV1;
  transferred_or_released_at?: UnixTimeV1;
}
