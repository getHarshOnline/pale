/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type WorkspaceModuleId = string;
export type WorkspacePublisherKeyId = string;
export type WorkspacePublisherId = string;
export type WorkspaceRecoveryAuthorityId = string;
export type WorkspaceReleaseId = string;
export type WorkspaceReleaseStateV1 = "candidate" | "published" | "revoked";

export interface WorkspaceResolutionLockV1 {
  /**
   * @minItems 0
   * @maxItems 1024
   */
  releases: WorkspaceResolvedReleaseV1[];
  resolution_digest: Sha256DigestV1;
  schema: "pale/workspace-resolution-lock.v1";
  schema_version: 1;
}
export interface WorkspaceResolvedReleaseV1 {
  module_digest: Sha256DigestV1;
  module_id: WorkspaceModuleId;
  publisher: WorkspacePublisherRefV1;
  release_id: WorkspaceReleaseId;
  release_state: WorkspaceReleaseStateV1;
}
export interface WorkspacePublisherRefV1 {
  delegated_release_key_id: WorkspacePublisherKeyId;
  publisher_id: WorkspacePublisherId;
  recovery_authority_id: WorkspaceRecoveryAuthorityId;
  root_key_id: WorkspacePublisherKeyId;
}
