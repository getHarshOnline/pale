/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type PackageIdV1 = string;
export type PackageVersionV1 = string;

export interface DistributionLockV1 {
  distribution: DistributionMeta;
  package?: PackageReleaseV1[];
}
export interface DistributionMeta {
  compiler: string;
  schema_manifest_sha256: Sha256DigestV1;
  schema_set_sha256: Sha256DigestV1;
}
export interface PackageReleaseV1 {
  content_digest: Sha256DigestV1;
  id: PackageIdV1;
  version: PackageVersionV1;
}
