/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ExecutionPathSelectionIdV1 = string;
export type LogicalRepositoryIdV1 = string;

export interface OperationRouteBindingV1 {
  binding_digest: Sha256DigestV1;
  catalog: SealedArtifactV1;
  catalog_digest: Sha256DigestV1;
  execution_path_witness: ExecutionPathWitnessV1;
  profile_digest: Sha256DigestV1;
  repository_id: LogicalRepositoryIdV1;
  schema: "pale/operation-route-binding.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
}
export interface SealedArtifactV1 {
  rel_path: string;
  sha256: Sha256DigestV1;
}
export interface ExecutionPathWitnessV1 {
  composition_plan_digest: Sha256DigestV1;
  responsibility_path_identity: ResponsibilityPathIdentityV1;
  selection: ExecutionPathSelectionV1;
  terminal_provider_binding?: ProviderBindingRefV1 | null;
  /**
   * @maxItems 256
   */
  work_delegation_edge_ids: string[];
}
export interface ResponsibilityPathIdentityV1 {
  responsibility_path_sha256: Sha256DigestV1;
}
export interface ExecutionPathSelectionV1 {
  experience_path_id: string;
  id: ExecutionPathSelectionIdV1;
}
export interface ProviderBindingRefV1 {
  provider_binding_id: string;
  provider_binding_identity: ProviderBindingIdentityV1;
}
export interface ProviderBindingIdentityV1 {
  provider_binding_sha256: Sha256DigestV1;
}
