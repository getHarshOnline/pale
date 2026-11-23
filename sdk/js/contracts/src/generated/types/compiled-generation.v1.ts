/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ExecutionPathSelectionIdV1 = string;
export type NormativePackAttestorV1 = string;
export type NormativePackIdV1 = string;
export type NormativePackVersionV1 = string;
export type PackageIdV1 = string;
export type PackageVersionV1 = string;
export type RepositoryRevisionV1 = string;
export type LogicalRepositoryIdV1 = string;
export type WorkspaceModuleId = string;
export type WorkspaceReleaseId = string;

export interface CompiledGenerationV1 {
  compilation_input_digest: Sha256DigestV1;
  compiled_generation_digest: Sha256DigestV1;
  distribution_lock_digest: Sha256DigestV1;
  executable_identity_digest: Sha256DigestV1;
  execution_paths: ExecutionPathSelectionV1[];
  experience_composition_plan_digest: Sha256DigestV1;
  experience_composition_source_digest: Sha256DigestV1;
  instance_digest: Sha256DigestV1;
  machine_overlay_digest: Sha256DigestV1;
  materialization_closure_digest: Sha256DigestV1;
  one_shot_compilation_digest: Sha256DigestV1;
  organizational_translation_closure_digest: Sha256DigestV1;
  package_semantic_ir_digest: Sha256DigestV1;
  principal_admitted_universe_digest: Sha256DigestV1;
  profile_digest: Sha256DigestV1;
  profile_normative_pack_release_closure: NormativePackReleaseV1[];
  profile_package_release_closure: PackageReleaseV1[];
  provider_binding_closure_digest: Sha256DigestV1;
  repository_source_frontiers: RepositorySourceFrontierV1[];
  responsibility_derivation_closure_digest: Sha256DigestV1;
  schema: "pale/compiled-generation.v1";
  schema_manifest_digest: Sha256DigestV1;
  schema_set_digest: Sha256DigestV1;
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  workspace_admission_evidence_digest: Sha256DigestV1;
  workspace_module_release_closure: WorkspaceAdoptedReleaseV1[];
  workspace_normative_pack_release_closure: NormativePackReleaseV1[];
  workspace_package_release_closure: PackageReleaseV1[];
  workspace_resolution_lock_digest: Sha256DigestV1;
  workspace_semantic_ir_digest: Sha256DigestV1;
  workspace_universe_lock_digest: Sha256DigestV1;
}
export interface ExecutionPathSelectionV1 {
  experience_path_id: string;
  id: ExecutionPathSelectionIdV1;
}
export interface NormativePackReleaseV1 {
  attestor: NormativePackAttestorV1;
  content_digest: Sha256DigestV1;
  id: NormativePackIdV1;
  version: NormativePackVersionV1;
}
export interface PackageReleaseV1 {
  content_digest: Sha256DigestV1;
  id: PackageIdV1;
  version: PackageVersionV1;
}
export interface RepositorySourceFrontierV1 {
  base_revision: RepositoryRevisionV1;
  dependency_frontier_sha256: Sha256DigestV1;
  repository_id: LogicalRepositoryIdV1;
  tracked_delta_sha256: Sha256DigestV1;
  untracked_inventory_sha256: Sha256DigestV1;
}
export interface WorkspaceAdoptedReleaseV1 {
  module_digest: Sha256DigestV1;
  module_id: WorkspaceModuleId;
  release_id: WorkspaceReleaseId;
}
