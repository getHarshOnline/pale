/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type LogicalRepositoryIdV1 = string;
export type ExecutionPathSelectionIdV1 = string;
export type SurfaceOwnershipV1 = "managed_block" | "whole_file";

export interface MaterializationStateV1 {
  instance_digest: Sha256DigestV1;
  manifest_digest: Sha256DigestV1;
  one_shot_compilation_digest: Sha256DigestV1;
  profile_digest: Sha256DigestV1;
  repository_id: LogicalRepositoryIdV1;
  schema: "pale.agent-surface.materialization-state.v1";
  schema_manifest_sha256: Sha256DigestV1;
  schema_set_sha256: Sha256DigestV1;
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  state_digest: Sha256DigestV1;
  surfaces: SurfaceDigestV1[];
}
export interface SurfaceDigestV1 {
  committable: boolean;
  execution_path_consumers: ExecutionPathSelectionIdV1[];
  kind: string;
  ownership: SurfaceOwnershipV1;
  rel_path: string;
  sha256: Sha256DigestV1;
}
