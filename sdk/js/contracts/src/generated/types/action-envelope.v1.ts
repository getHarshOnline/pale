/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ActionIdV1 = string;
export type BusinessIdempotencyKeyV1 = string;
export type SchemaId = string;
export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
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
export type TypedDigestAlgorithmV1 = "sha256";
export type BoundedResidentTextV1 = string;
export type ActionOperationV1 = string;
export type ActionTargetIdentityV1 = string;
export type ActionTargetSystemV1 = string;

export interface ActionEnvelopeV1 {
  action_id: ActionIdV1;
  business_idempotency_key: BusinessIdempotencyKeyV1;
  conflict_resource_key: any;
  conflict_resource_key_schema_id: SchemaId;
  conflict_resource_key_schema_sha256: Sha256DigestV1;
  effect_policy: ResidentRecordRefV1;
  expected_precondition_digest?: TypedDigestV1 | null;
  immutable_payload_ref?: ImmutableActionPayloadRefV1 | null;
  inline_payload?: any;
  operation: ActionOperationV1;
  payload_hash: Sha256DigestV1;
  payload_schema_id: SchemaId;
  payload_schema_sha256: Sha256DigestV1;
  reconciliation_policy: ResidentRecordRefV1;
  target_identity: ActionTargetIdentityV1;
  target_system: ActionTargetSystemV1;
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
export interface ImmutableActionPayloadRefV1 {
  payload_ref: ResidentRecordRefV1;
  payload_ref_sha256: Sha256DigestV1;
}
