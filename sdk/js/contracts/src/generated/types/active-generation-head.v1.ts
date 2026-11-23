/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type JournalSequenceV1 = number;

export interface ActiveGenerationHeadV1 {
  activation_fence_digest: Sha256DigestV1;
  active_generation_head_digest: Sha256DigestV1;
  advertised_generation_head_digest: Sha256DigestV1;
  compiled_generation_digest: Sha256DigestV1;
  journal_sequence: JournalSequenceV1;
  predecessor_active_generation_head_digest?: Sha256DigestV1;
  schema: "pale/active-generation-head.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
}
