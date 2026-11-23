/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ExecutionPathSelectionIdV1 = string;
export type ExecutionWorkspaceRepositoryAccessV1 = "read" | "mutate";
export type LogicalExecutionWorkspaceIdV1 = string;
export type LogicalRepositoryIdV1 = string;
export type AuthorityAccessClass = "read-only";
export type LifecycleEventV1 =
  | "session-start"
  | "user-prompt-submit"
  | "user-prompt-expansion"
  | "permission-request"
  | "pre-tool-use"
  | "post-tool-use"
  | "post-tool-use-failure"
  | "subagent-start"
  | "subagent-stop"
  | "pre-compact"
  | "post-compact"
  | "stop"
  | "session-end";
export type SessionRuntimeSelection = "machine-local";
export type LogicalProjectIdV1 = string;
export type PackageIdV1 = string;
export type PackageResourceIdV1 = string;
export type ResponsibilityDerivationId = string;
export type OwnedConfinedRelativePath = string;
export type ProjectOwnerReferenceV1 =
  | {
      kind: "principal";
      module_id: WorkspaceModuleId;
      resource_id: WorkspaceResourceId;
    }
  | {
      kind: "organization";
      module_id: WorkspaceModuleId;
      resource_id: WorkspaceResourceId;
    };
export type WorkspaceModuleId = string;
export type WorkspaceResourceId = string;
export type RunPolicyIdV1 = string;
export type WorkspaceId = string;

export interface PaleWorkspaceV1 {
  workspace: WorkspaceMeta;
}
export interface WorkspaceMeta {
  desired_topology: WorkspaceDesiredTopologyV1;
  distribution_lock: OwnedConfinedRelativePath;
  /**
   * @maxItems 1
   */
  experience_compositions: [] | [OwnedConfinedRelativePath];
  machine_binding_root: OwnedConfinedRelativePath;
  /**
   * @maxItems 1024
   */
  module_manifests: OwnedConfinedRelativePath[];
  /**
   * @maxItems 1024
   */
  module_sources?: OwnedConfinedRelativePath[];
  /**
   * @maxItems 1
   */
  organizational_translations: [] | [OwnedConfinedRelativePath];
  resolution_lock: OwnedConfinedRelativePath;
  /**
   * @maxItems 1024
   */
  responsibility_derivations: OwnedConfinedRelativePath[];
  schema_version: 1;
  /**
   * @maxItems 1024
   */
  source_components: OwnedConfinedRelativePath[];
  universe_lock: OwnedConfinedRelativePath;
  workspace_id: WorkspaceId;
}
export interface WorkspaceDesiredTopologyV1 {
  /**
   * @maxItems 1024
   */
  execution_paths: ExecutionPathSelectionV1[];
  /**
   * @maxItems 4096
   */
  execution_workspace_repositories: ExecutionWorkspaceRepositoryMembershipV1[];
  /**
   * @maxItems 1024
   */
  execution_workspaces: LogicalExecutionWorkspaceV1[];
  /**
   * @maxItems 4096
   */
  project_execution_workspaces: ProjectExecutionWorkspaceMembershipV1[];
  /**
   * @maxItems 4096
   */
  project_repositories: ProjectRepositoryMembershipV1[];
  /**
   * @maxItems 1024
   */
  projects: LogicalProjectV1[];
  /**
   * @maxItems 1024
   */
  repositories: LogicalRepositoryV1[];
}
export interface ExecutionPathSelectionV1 {
  experience_path_id: string;
  id: ExecutionPathSelectionIdV1;
}
export interface ExecutionWorkspaceRepositoryMembershipV1 {
  access: ExecutionWorkspaceRepositoryAccessV1;
  execution_workspace_id: LogicalExecutionWorkspaceIdV1;
  repository_id: LogicalRepositoryIdV1;
}
export interface LogicalExecutionWorkspaceV1 {
  /**
   * @maxItems 1024
   */
  execution_path_ids: ExecutionPathSelectionIdV1[];
  id: LogicalExecutionWorkspaceIdV1;
  session?: SessionModel | null;
  title: string;
}
export interface SessionModel {
  activation_path_ids?: ExecutionPathSelectionIdV1[];
  /**
   * @minItems 1
   */
  activation_roots: [LogicalRepositoryIdV1, ...LogicalRepositoryIdV1[]];
  additional_roots?: LogicalRepositoryIdV1[];
  authority_access?: AuthorityAccess[];
  lifecycle_events?: LifecycleEventV1[];
  reading_order?: ReadingOrderEntry[];
  runtime?: SessionRuntimeSelection;
}
export interface AuthorityAccess {
  class: AuthorityAccessClass;
  root_id: string;
}
export interface ReadingOrderEntry {
  rel_path: string;
  root_id: string;
}
export interface ProjectExecutionWorkspaceMembershipV1 {
  execution_workspace_id: LogicalExecutionWorkspaceIdV1;
  project_id: LogicalProjectIdV1;
}
export interface ProjectRepositoryMembershipV1 {
  /**
   * @maxItems 1024
   */
  families?: PackageResourceRefV1[];
  materialization_order: number;
  project_id: LogicalProjectIdV1;
  repository_id: LogicalRepositoryIdV1;
  /**
   * @maxItems 1024
   */
  responsibility_derivation_ids?: ResponsibilityDerivationId[];
  role: PackageResourceRefV1;
  /**
   * @maxItems 1024
   */
  stacks?: PackageResourceRefV1[];
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface LogicalProjectV1 {
  /**
   * @maxItems 1024
   */
  authority_root_ids: string[];
  capabilities_file: OwnedConfinedRelativePath;
  id: LogicalProjectIdV1;
  owner?: ProjectOwnerReferenceV1 | null;
  /**
   * @maxItems 1024
   */
  package_seeds: PackageIdV1[];
  run_policy: RunPolicyIdV1;
  title: string;
}
export interface LogicalRepositoryV1 {
  id: LogicalRepositoryIdV1;
  title: string;
}
