/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type CapabilityIdV1 = string;
export type Sha256DigestV1 = string;
export type LifecycleAction =
  | "inject"
  | "inject_modes"
  | "deny"
  | "ask"
  | "warn"
  | "guide"
  | "notify"
  | "capture"
  | "activate"
  | "deactivate"
  | "rebaseline"
  | "signal"
  | "no_op";
export type PackageIdV1 = string;
export type PackageResourceIdV1 = string;
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
export type OwnedConfinedRelativePath = string;
export type LifecycleBehaviorIdV1 = string;
export type EvidenceRetentionSeconds = number;
export type ExecutionPathSelectionIdV1 = string;
export type EffectClass = "compute" | "read" | "execute" | "write" | "network" | "publish";
export type HostCommandSource = "repository_capability_commands";
export type ToolObservability = "pre_tool_observable" | "permission_only" | "unobservable";
export type RouteRecordLimit = number;
export type LogicalRepositoryIdV1 = string;
export type ModeAxis = "intensity" | "task_stance";
export type ToolCallMatcherV1 = string;
export type RunPolicyIdV1 = string;
export type SessionTtlSeconds = number;

export interface HookContextV1 {
  adapter_plugin_id: string;
  available_tool_requirements?: CapabilityIdV1[];
  behaviour_guidance?: {
    [k: string]: SealedArtifactV1;
  };
  behaviours?: LifecycleBehavior[];
  compilation_input_digest: Sha256DigestV1;
  durable_evidence?: DurableEvidenceBinding | null;
  execution_path_witness: ExecutionPathWitnessV1;
  host_tool_effects?: HostToolEffectBinding[];
  injection_digest: Sha256DigestV1;
  instance_digest: Sha256DigestV1;
  materialization_lineage?: MaterializationLineageBinding | null;
  mode_defaults?: {
    [k: string]: PackageResourceRefV1;
  };
  native_harness_binding?: NativeHarnessBindingV1 | null;
  operation_binding: SealedArtifactV1;
  profile_digest: Sha256DigestV1;
  projection_manifest: SealedArtifactV1;
  protected_runtime_dirs?: string[];
  read_only_dirs?: string[];
  repository_id: LogicalRepositoryIdV1;
  repository_verifiers?: RepositoryVerifierBinding[];
  run_modes?: SealedRunMode[];
  run_policy: RunPolicyEntry;
  schema: "pale.agent-surface.hook-context.v1";
  schema_manifest_sha256: Sha256DigestV1;
  schema_set_sha256: Sha256DigestV1;
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  session_state?: SessionStateBinding | null;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)*$".
 */
export interface SealedArtifactV1 {
  rel_path: string;
  sha256: Sha256DigestV1;
}
export interface LifecycleBehavior {
  action: LifecycleAction;
  activate_mode?: PackageResourceRefV1 | null;
  allow_admitted_operation?: boolean;
  case_insensitive?: boolean;
  description: string;
  event: LifecycleEventV1;
  existing_targets_only?: boolean;
  failure_posture?: "fail_closed" | "fail_open" | "retry" | "steer" | "escalate";
  fallback?: LifecycleFallbackV1 | null;
  governed_by?: string | null;
  guidance?: OwnedConfinedRelativePath | null;
  id: LifecycleBehaviorIdV1;
  /**
   * @maxItems 256
   */
  matchers?: string[];
  modes?: PackageResourceRefV1[];
  predicate_kind?:
    | "always"
    | "tool_match"
    | "destructive_shell"
    | "git_hook_bypass"
    | "write_effect"
    | "path_match"
    | "command_prefix"
    | "exact_message"
    | "mode_changed"
    | "rebaseline_pending"
    | "first_touch"
    | "connector_bound"
    | "never";
  state_scope?: "stateless" | "message" | "session" | "durable";
  timeout_ms?: number | null;
  tool_reqs?: CapabilityIdV1[];
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface LifecycleFallbackV1 {
  carrier_event: LifecycleEventV1;
  guidance: string;
}
export interface DurableEvidenceBinding {
  retention_seconds: EvidenceRetentionSeconds;
  root: string;
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
export interface HostToolEffectBinding {
  effect_class: EffectClass;
  tool: HostToolBinding;
}
export interface HostToolBinding {
  command_source?: HostCommandSource | null;
  commands?: HostCommandConstraint[];
  input_equals?: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^/(?:[^~\u0000-\u001F\u007F-\u009F]|~[01])+$".
     */
    [k: string]: string;
  };
  model_name?: string;
  name: string;
  observability: ToolObservability;
}
export interface HostCommandConstraint {
  allow_trailing_args?: boolean;
  argv_prefix?: string[];
  executable: string;
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
export interface RepositoryVerifierBinding {
  capability_id: CapabilityIdV1;
  command: HostCommandConstraint;
  tool_name: string;
}
export interface SealedRunMode {
  aliases?: string[];
  axis: ModeAxis;
  body?: SealedArtifactV1 | null;
  exact_triggers?: string[];
  id: PackageResourceRefV1;
}
export interface RunPolicyEntry {
  action_gate?: boolean;
  child_propagation?: "propagate" | "exempt" | "none";
  child_type_selectors?: string[];
  description: string;
  /**
   * @maxItems 256
   */
  gated_tool_matchers?: ToolCallMatcherV1[];
  id: RunPolicyIdV1;
  session_inject?: boolean;
}
export interface SessionStateBinding {
  root: string;
  ttl_seconds: SessionTtlSeconds;
}
