/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ActionIdV1 = string;
export type ResidentRecordRefV1 = string;
export type Sha256DigestV1 = string;
export type ReconciliationCaseRevisionV1 = number;
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
export type ReconciliationIdV1 = string;
export type UnixTimeV1 = number;

export interface ReconciliationRequestV1 {
  action_id: ActionIdV1;
  case_ref: ResidentRecordRefV1;
  case_sha256: Sha256DigestV1;
  custody_ref: ResidentRecordRefV1;
  custody_sha256: Sha256DigestV1;
  effect_admission_ref: ResidentRecordRefV1;
  effect_admission_sha256: Sha256DigestV1;
  effect_receipts: ReconciliationEffectReceiptRefV1[];
  expected_case_revision: ReconciliationCaseRevisionV1;
  expected_custody_generation: ReconciliationCustodyGenerationV1;
  expected_state_digest: TypedDigestV1;
  observation_bounds: any;
  predecessor_result_ref?: ResidentRecordRefV1 | null;
  predecessor_result_sha256?: Sha256DigestV1 | null;
  reconciliation_id: ReconciliationIdV1;
  reconciliation_policy_ref: ResidentRecordRefV1;
  reconciliation_policy_sha256: Sha256DigestV1;
  requested_at: UnixTimeV1;
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
