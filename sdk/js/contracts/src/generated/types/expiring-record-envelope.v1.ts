/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type SchemaId = string;
export type ExpiringRecordClassV1 = "session-continuity" | "durable-evidence";

export interface ExpiringRecordEnvelopeV1 {
  compilation_input_digest: Sha256DigestV1;
  payload_schema: SchemaId;
  payload_schema_sha256: Sha256DigestV1;
  publication_generation: number;
  record_binding: Sha256DigestV1;
  record_class: ExpiringRecordClassV1;
  schema: "pale.expiring-record-envelope.v1";
  schema_manifest_sha256: Sha256DigestV1;
  schema_set_sha256: Sha256DigestV1;
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  state: {
    [k: string]: any;
  };
  updated_subsec_nanos?: number;
  updated_unix_seconds: number;
}
