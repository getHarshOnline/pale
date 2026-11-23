/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type LogicalRepositoryIdV1 = string;

export interface MaterializationLineageV1 {
  admitted_plan_digest: Sha256DigestV1;
  instance_digest: Sha256DigestV1;
  lineage_digest: Sha256DigestV1;
  manifest_digest: Sha256DigestV1;
  one_shot_compilation_digest: Sha256DigestV1;
  profile_digest: Sha256DigestV1;
  repository_id: LogicalRepositoryIdV1;
  schema: "pale.agent-surface.materialization-lineage.v1";
  schema_manifest_sha256: Sha256DigestV1;
  schema_set_sha256: Sha256DigestV1;
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  state_digest: Sha256DigestV1;
  target_root_sha256: Sha256DigestV1;
}
