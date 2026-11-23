/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type LogicalRepositoryIdV1 = string;

export interface DeferredSurfaceManifestV1 {
  /**
   * @minItems 1
   */
  artifacts: [DeferredArtifactV1, ...DeferredArtifactV1[]];
  compilation_input_digest: Sha256DigestV1;
  one_shot_compilation_digest: Sha256DigestV1;
  repository_id: LogicalRepositoryIdV1;
  repository_manifest_digest: Sha256DigestV1;
  schema: "pale.deferred-surface-manifest.v1";
  schema_manifest_sha256: Sha256DigestV1;
  schema_set_sha256: Sha256DigestV1;
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
}
export interface DeferredArtifactV1 {
  activation: string;
  content_sha256: Sha256DigestV1;
  inert_path: string;
  kind: string;
  target: string;
}
