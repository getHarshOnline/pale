/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type WorkspaceModuleId = string;
export type WorkspaceReleaseId = string;
export type WorkspaceId = string;

export interface WorkspaceUniverseLockV1 {
  /**
   * @minItems 0
   * @maxItems 1024
   */
  adopted_releases: WorkspaceAdoptedReleaseV1[];
  resolution_digest: Sha256DigestV1;
  schema: "pale/workspace-universe-lock.v1";
  schema_version: 1;
  universe_digest: Sha256DigestV1;
  workspace_id: WorkspaceId;
}
export interface WorkspaceAdoptedReleaseV1 {
  module_digest: Sha256DigestV1;
  module_id: WorkspaceModuleId;
  release_id: WorkspaceReleaseId;
}
