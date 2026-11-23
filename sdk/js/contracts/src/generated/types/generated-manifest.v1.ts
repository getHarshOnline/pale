/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type LogicalExecutionWorkspaceIdV1 = string;
export type ExecutionPathSelectionIdV1 = string;
export type SurfaceOwnershipV1 = "managed_block" | "whole_file";
export type LogicalProjectIdV1 = string;
export type LogicalRepositoryIdV1 = string;

export interface GeneratedManifestV1 {
  compilation_input_digest: Sha256DigestV1;
  compiler_version: string;
  execution_workspace_id: LogicalExecutionWorkspaceIdV1;
  instance_digest: Sha256DigestV1;
  manifest_digest: Sha256DigestV1;
  outputs: SurfaceDigestV1[];
  profile_digest: Sha256DigestV1;
  project_id: LogicalProjectIdV1;
  renderer_version: string;
  repository_id: LogicalRepositoryIdV1;
  schema: "pale.agent-surface.generated-manifest.v1";
  schema_manifest_sha256: Sha256DigestV1;
  schema_set_sha256: Sha256DigestV1;
  schema_sha256: Sha256DigestV1;
}
export interface SurfaceDigestV1 {
  committable: boolean;
  execution_path_consumers: ExecutionPathSelectionIdV1[];
  kind: string;
  ownership: SurfaceOwnershipV1;
  rel_path: string;
  sha256: Sha256DigestV1;
}
