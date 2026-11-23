/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;

export interface OperationDenialV1 {
  code: string;
  detail?: string | null;
  message: string;
  operation?: string | null;
  requirement?: string | null;
  schema: "pale/operation-denial.v1";
  schema_sha256: Sha256DigestV1;
  scope_digest?: Sha256DigestV1 | null;
  tool?: string | null;
}
