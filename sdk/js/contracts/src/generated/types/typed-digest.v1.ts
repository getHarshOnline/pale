/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

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
export type ResidentRecordRefV1 = string;
export type Sha256DigestV1 = string;
export type SchemaId = string;
export type TypedDigestAlgorithmV1 = "sha256";
export type BoundedResidentTextV1 = string;

export interface TypedDigestV1 {
  canonicalization: TypedDigestCanonicalizationV1;
  content_binding: TypedDigestContentBindingV1;
  content_schema_id: SchemaId;
  content_schema_sha256: Sha256DigestV1;
  digest: Sha256DigestV1;
  digest_algorithm: TypedDigestAlgorithmV1;
  kind: BoundedResidentTextV1;
}
