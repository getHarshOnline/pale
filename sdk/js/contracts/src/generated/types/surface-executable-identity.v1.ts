/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type DurablePublication = "apple-full-fsync" | "unix-fsync" | "standard-sync-all";
export type Sha256DigestV1 = string;

export interface SurfaceExecutableIdentityV1 {
  arch: string;
  durable_publication: DurablePublication;
  os: string;
  protocol: string;
  schema: "pale/surface-executable-identity.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  version: string;
}
