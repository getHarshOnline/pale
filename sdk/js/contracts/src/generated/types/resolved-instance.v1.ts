/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type EvidenceRetentionSeconds = number;
export type Sha256DigestV1 = string;
export type RouteRecordLimit = number;
export type LogicalRepositoryIdV1 = string;
export type SessionTtlSeconds = number;
export type PackageIdV1 = string;
export type PackageResourceIdV1 = string;

export interface ResolvedInstanceV1 {
  authority_roots: {
    [k: string]: string;
  };
  durable_evidence?: DurableEvidenceBinding | null;
  git_metadata_roots?: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^[a-z][a-z0-9_-]*(?:[.:/][a-z][a-z0-9_-]*)*$".
     */
    [k: string]: string;
  };
  instance_digest: Sha256DigestV1;
  machine_id: string;
  materialization_lineage?: MaterializationLineageBinding | null;
  native_harnesses?: NativeHarnessBindingV1[];
  profile_digest: Sha256DigestV1;
  provider_repositories?: ProviderRepositoryProjectionV1[];
  repositories: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^[a-z][a-z0-9_-]*(?:[.:/][a-z][a-z0-9_-]*)*$".
     */
    [k: string]: string;
  };
  schema: "pale.agent-surface.resolved-instance.v1";
  schema_sha256: Sha256DigestV1;
  session_state?: SessionStateBinding | null;
  surface_executable?: SurfaceExecutableBindingV1 | null;
  user_mode_defaults?: {
    [k: string]: PackageResourceRefV1;
  };
}
export interface DurableEvidenceBinding {
  retention_seconds: EvidenceRetentionSeconds;
  root: string;
}
export interface MaterializationLineageBinding {
  max_route_records_per_collection: RouteRecordLimit;
  root: string;
}
export interface NativeHarnessBindingV1 {
  digest: Sha256DigestV1;
  exec: string;
  provider_binding: ProviderBindingRefV1;
  version: string;
}
export interface ProviderBindingRefV1 {
  provider_binding_id: string;
  provider_binding_identity: ProviderBindingIdentityV1;
}
export interface ProviderBindingIdentityV1 {
  provider_binding_sha256: Sha256DigestV1;
}
export interface ProviderRepositoryProjectionV1 {
  native_repository_coordinate: string;
  provider_binding_id: string;
  repository_id: LogicalRepositoryIdV1;
}
export interface SessionStateBinding {
  root: string;
  ttl_seconds: SessionTtlSeconds;
}
export interface SurfaceExecutableBindingV1 {
  digest: Sha256DigestV1;
  exec: string;
  protocol: "pale.agent-surface-interface.v1";
  version: string;
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
