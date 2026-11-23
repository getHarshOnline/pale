/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ExecutionPathSelectionIdV1 = string;
export type SurfaceOwnershipV1 = "managed_block" | "whole_file";
export type Sha256DigestV1 = string;
export type LogicalRepositoryIdV1 = string;

export interface HarnessProjectionV1 {
  artifacts: SurfaceDigestV1[];
  front_door: ManagedFrontDoorDigest;
  instance_digest: Sha256DigestV1;
  profile_digest: Sha256DigestV1;
  provider_binding: ProviderBindingRefV1;
  repository_id: LogicalRepositoryIdV1;
  schema: "pale.agent-surface.harness-projection.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
}
export interface SurfaceDigestV1 {
  committable: boolean;
  execution_path_consumers: ExecutionPathSelectionIdV1[];
  kind: string;
  ownership: SurfaceOwnershipV1;
  rel_path: string;
  sha256: Sha256DigestV1;
}
export interface ManagedFrontDoorDigest {
  kind: string;
  managed_block_sha256: Sha256DigestV1;
  ownership: SurfaceOwnershipV1;
  rel_path: string;
}
export interface ProviderBindingRefV1 {
  provider_binding_id: string;
  provider_binding_identity: ProviderBindingIdentityV1;
}
export interface ProviderBindingIdentityV1 {
  provider_binding_sha256: Sha256DigestV1;
}
