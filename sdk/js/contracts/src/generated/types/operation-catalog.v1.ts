/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type OperationDispatchV1 =
  | {
      aliases?: string[];
      invocation: string;
      kind: "model_or_user";
    }
  | {
      aliases?: string[];
      invocation: string;
      kind: "user_only";
    }
  | {
      kind: "internal";
    };
export type PackageIdV1 = string;
export type PackageResourceIdV1 = string;
export type CatalogOperationStateV1 =
  | {
      kind: "semantic_only";
    }
  | {
      kind: "capability_bearing";
      support: OperationSupport;
    };
export type OperationSupport = "full" | "degraded" | "blocked";
export type LogicalRepositoryIdV1 = string;

export interface OperationCatalogV1 {
  catalog_digest: Sha256DigestV1;
  entries: OperationCatalogEntryV1[];
  profile_digest: Sha256DigestV1;
  provider_binding: ProviderBindingRefV1;
  repository_id: LogicalRepositoryIdV1;
  schema: "pale/operation-catalog.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
}
export interface OperationCatalogEntryV1 {
  contract: SealedArtifactV1;
  contract_digest: Sha256DigestV1;
  dispatch: OperationDispatchV1;
  id: PackageResourceRefV1;
  state: CatalogOperationStateV1;
}
export interface SealedArtifactV1 {
  rel_path: string;
  sha256: Sha256DigestV1;
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface ProviderBindingRefV1 {
  provider_binding_id: string;
  provider_binding_identity: ProviderBindingIdentityV1;
}
export interface ProviderBindingIdentityV1 {
  provider_binding_sha256: Sha256DigestV1;
}
