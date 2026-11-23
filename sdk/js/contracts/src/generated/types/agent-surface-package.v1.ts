/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type PackageIdV1 = string;
export type PackageVersionV1 = string;
export type OwnedConfinedRelativePath = string;
export type PackageResourceExposureV1 = "internal" | "exported";
export type PackageResourceIdV1 = string;
export type PackageResourceKindV1 =
  | "doctrine"
  | "operation_archetype"
  | "operation"
  | "role_archetype"
  | "role"
  | "stack"
  | "run_mode"
  | "policy_family"
  | "governing_order"
  | "capability_policy"
  | "tool_recipe";
export type PackageSourceRoleV1 = "declaration" | "body" | "documentation";

export interface AgentSurfacePackageV1 {
  license?: PackageLicense | null;
  package: PackageMeta;
}
export interface PackageLicense {
  note?: string | null;
  spdx: string;
}
export interface PackageMeta {
  compatible_compilers: string[];
  /**
   * @maxItems 1024
   */
  dependencies?: PackageDependencyV1[];
  description: string;
  id: PackageIdV1;
  /**
   * @minItems 1
   * @maxItems 4096
   */
  resources?: [PackageResourceV1, ...PackageResourceV1[]];
  schema_version: 1;
  /**
   * @minItems 1
   * @maxItems 4096
   */
  sources?: [PackageSourceV1, ...PackageSourceV1[]];
  title: string;
  version: PackageVersionV1;
}
export interface PackageDependencyV1 {
  id: PackageIdV1;
  version: PackageVersionV1;
}
export interface PackageResourceV1 {
  definition_source: OwnedConfinedRelativePath;
  exposure: PackageResourceExposureV1;
  id: PackageResourceIdV1;
  kind: PackageResourceKindV1;
}
export interface PackageSourceV1 {
  path: OwnedConfinedRelativePath;
  role: PackageSourceRoleV1;
}
