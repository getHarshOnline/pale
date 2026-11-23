/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type EvidenceRetentionSeconds = number;
export type RouteRecordLimit = number;
export type Sha256DigestV1 = string;
export type PackageIdV1 = string;
export type PackageResourceIdV1 = string;
export type LogicalRepositoryIdV1 = string;
export type SessionTtlSeconds = number;

export interface MachineBindingV1 {
  authority_roots: {
    [k: string]: string;
  };
  durable_evidence?: DurableEvidenceBinding;
  machine: MachineMeta;
  materialization_lineage?: MaterializationLineageBinding;
  native_harnesses?: NativeHarnessBindingV1[];
  preferences?: UserPreferences;
  provider_repositories?: ProviderRepositoryProjectionV1[];
  repository_checkouts?: RepositoryCheckoutBindingV1[];
  session_state?: SessionStateBinding;
  surface_executable?: SurfaceExecutableBindingV1;
}
export interface DurableEvidenceBinding {
  retention_seconds: EvidenceRetentionSeconds;
  root: string;
}
export interface MachineMeta {
  distribution_root?: string;
  id: string;
  schema_version: 1;
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
export interface UserPreferences {
  mode_defaults?: {
    [k: string]: PackageResourceRefV1;
  };
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface ProviderRepositoryProjectionV1 {
  native_repository_coordinate: string;
  provider_binding_id: string;
  repository_id: LogicalRepositoryIdV1;
}
export interface RepositoryCheckoutBindingV1 {
  checkout_root: string;
  git_metadata_root?: string;
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
