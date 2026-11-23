/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ActivationFenceGenerationV1 = number;
export type ActivationFenceIdV1 = string;

export interface ActivationFenceV1 {
  activation_fence_digest: Sha256DigestV1;
  activation_fence_generation: ActivationFenceGenerationV1;
  activation_fence_id: ActivationFenceIdV1;
  compiled_generation_digest: Sha256DigestV1;
  predecessor_activation_fence_digest?: Sha256DigestV1;
  schema: "pale/activation-fence.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
}
