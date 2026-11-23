/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Approval = "none" | "ask" | "gated";
export type EffectClass = "compute" | "read" | "execute" | "write" | "network" | "publish";
export type CapabilityProviderBinding =
  | {
      kind: "host_native";
      provider_tools: ProviderToolBindingV1[];
    }
  | {
      kind: "native_rust";
      target: string;
    }
  | {
      kind: "connector";
      provider_tools?: ProviderToolBindingV1[];
      target: string;
    };
export type Sha256DigestV1 = string;
export type HostCommandSource = "repository_capability_commands";
export type ToolObservability = "pre_tool_observable" | "permission_only" | "unobservable";
export type ToolContractIdV1 = string;
export type CapabilityIdV1 = string;
export type ExecutionPathSelectionIdV1 = string;
export type LogicalExecutionWorkspaceIdV1 = string;
export type PackageIdV1 = string;
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
export type SemanticOrderPurposeV1 = "composition" | "reasoning" | "execution" | "presentation";
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
export type FailurePosture = "fail_closed" | "fail_open" | "retry" | "steer" | "escalate";
export type LifecycleBehaviorIdV1 = string;
export type PredicateKind =
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
export type StateScope = "stateless" | "message" | "session" | "durable";
export type NormativePackAttestorV1 = string;
export type NormativePackIdV1 = string;
export type NormativePackVersionV1 = string;
export type OperationSelection =
  | {
      aliases?: string[];
      kind: "model_or_user";
    }
  | {
      aliases?: string[];
      kind: "user_only";
    }
  | {
      kind: "internal";
    };
export type PackageVersionV1 = string;
export type LogicalProjectIdV1 = string;
export type ExecutionWorkspaceRepositoryAccessV1 = "read" | "mutate";
export type LogicalRepositoryIdV1 = string;
export type ToolCallMatcherV1 = string;
export type RunPolicyIdV1 = string;
export type PackageResourceSelectorV1 =
  | {
      resource: PackageResourceRefV1;
      selection: "exact";
    }
  | {
      package_id: PackageIdV1;
      selection: "all_from_package";
    };
export type AuthorityAccessClass = "read-only";
export type SessionRuntimeSelection = "machine-local";
export type WorkspaceId = string;

export interface EffectiveProfileV1 {
  authority_root_ids: string[];
  capability_bindings?: {
    [k: string]: CapabilityBinding;
  };
  capability_narrowing?: CapabilityNarrowingV1 | null;
  execution_paths: ExecutionPathSelectionV1[];
  execution_workspace_id: LogicalExecutionWorkspaceIdV1;
  forbidden_token_authority?: string[];
  governing_order: AdmittedGoverningOrderV1;
  lifecycle?: AdmittedLifecycleBehaviorV1[];
  normative_pack_closure: NormativePackReleaseV1[];
  operation_bindings?: OperationBindingEntryV1[];
  package_closure: PackageReleaseV1[];
  package_semantic_ir_digest: Sha256DigestV1;
  profile_digest: Sha256DigestV1;
  project_id: LogicalProjectIdV1;
  repositories: ProfileRepository[];
  run_policy: RunPolicyEntry;
  schema: "pale.agent-surface.effective-profile.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  semantic?: SemanticSelection;
  session?: SessionModel | null;
  workspace_id: WorkspaceId;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)*$".
 */
export interface CapabilityBinding {
  approval: Approval;
  effect_class: EffectClass;
  implementation: CapabilityProviderBinding;
  note?: string | null;
  tool_contract?: ToolContractIdV1 | null;
}
export interface ProviderToolBindingV1 {
  provider_binding: ProviderBindingRefV1;
  /**
   * @minItems 1
   */
  tools: [HostToolBinding, ...HostToolBinding[]];
}
export interface ProviderBindingRefV1 {
  provider_binding_id: string;
  provider_binding_identity: ProviderBindingIdentityV1;
}
export interface ProviderBindingIdentityV1 {
  provider_binding_sha256: Sha256DigestV1;
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
export interface CapabilityNarrowingV1 {
  /**
   * @maxItems 4096
   */
  forbidden_capabilities?: CapabilityIdV1[];
  /**
   * @maxItems 4096
   */
  forbidden_effects?: EffectClass[];
  /**
   * @maxItems 4096
   */
  permitted_capabilities: CapabilityIdV1[];
  /**
   * @maxItems 4096
   */
  permitted_effects: EffectClass[];
}
export interface ExecutionPathSelectionV1 {
  experience_path_id: string;
  id: ExecutionPathSelectionIdV1;
}
export interface AdmittedGoverningOrderV1 {
  ceilings: EffectiveGoverningCeilingV1[];
  exclusive_authority: AdmittedExclusiveAuthorityClaimV1[];
  narrowing: ResolvedGoverningNarrowingV1[];
  obligations: AccumulatedGoverningObligationV1[];
  order: AdmittedScopedOrderRelationV1[];
  schema_version: 1;
}
export interface EffectiveGoverningCeilingV1 {
  contributors: PackageResourceRefV1[];
  policies: PackageResourceRefV1[];
  policy: AdmittedCapabilityPolicyV1;
  scope: PackageResourceUseV1;
  subject: PackageResourceUseV1;
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface AdmittedCapabilityPolicyV1 {
  forbidden_capabilities: CapabilityIdV1[];
  forbidden_effects: EffectClass[];
  permitted_capabilities: CapabilityIdV1[];
  permitted_effects: EffectClass[];
}
export interface PackageResourceUseV1 {
  kind: PackageResourceKindV1;
  resource: PackageResourceRefV1;
}
export interface AdmittedExclusiveAuthorityClaimV1 {
  authority: PackageResourceUseV1;
  claimant: PackageResourceRefV1;
  scope: PackageResourceUseV1;
}
export interface ResolvedGoverningNarrowingV1 {
  broader: PackageResourceUseV1;
  contributor: PackageResourceRefV1;
  narrower: PackageResourceUseV1;
  scope: PackageResourceUseV1;
}
export interface AccumulatedGoverningObligationV1 {
  contributors: PackageResourceRefV1[];
  obligation: PackageResourceUseV1;
  scope: PackageResourceUseV1;
  subject: PackageResourceUseV1;
}
export interface AdmittedScopedOrderRelationV1 {
  after: PackageResourceUseV1;
  before: PackageResourceUseV1;
  contributor: PackageResourceRefV1;
  purpose: SemanticOrderPurposeV1;
  scope: PackageResourceUseV1;
}
export interface AdmittedLifecycleBehaviorV1 {
  action: LifecycleAction;
  activate_mode?: PackageResourceRefV1 | null;
  allow_admitted_operation: boolean;
  case_insensitive: boolean;
  description: string;
  event: LifecycleEventV1;
  existing_targets_only: boolean;
  failure_posture: FailurePosture;
  fallback?: LifecycleFallbackV1 | null;
  governed_by?: string | null;
  guidance?: string | null;
  id: LifecycleBehaviorIdV1;
  matchers: string[];
  modes: PackageResourceRefV1[];
  predicate_kind: PredicateKind;
  state_scope: StateScope;
  timeout_ms?: number | null;
  tool_reqs: CapabilityIdV1[];
}
export interface LifecycleFallbackV1 {
  carrier_event: LifecycleEventV1;
  guidance: string;
}
export interface NormativePackReleaseV1 {
  attestor: NormativePackAttestorV1;
  content_digest: Sha256DigestV1;
  id: NormativePackIdV1;
  version: NormativePackVersionV1;
}
export interface OperationBindingEntryV1 {
  binding: OperationBinding;
  operation: PackageResourceRefV1;
}
export interface OperationBinding {
  selection: OperationSelection;
}
export interface PackageReleaseV1 {
  content_digest: Sha256DigestV1;
  id: PackageIdV1;
  version: PackageVersionV1;
}
export interface ProfileRepository {
  access: ExecutionWorkspaceRepositoryAccessV1;
  capability_commands?: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)*$".
     */
    [k: string]: string[];
  };
  command_note?: string | null;
  families: PackageResourceRefV1[];
  id: LogicalRepositoryIdV1;
  mutating_not_readonly?: string[];
  role: PackageResourceRefV1;
  stacks?: PackageResourceRefV1[];
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
export interface SemanticSelection {
  doctrine?: PackageResourceSelectorV1[];
  mode_defaults?: {
    [k: string]: PackageResourceRefV1;
  };
  operations?: PackageResourceSelectorV1[];
  roles?: PackageResourceSelectorV1[];
  run_modes?: PackageResourceSelectorV1[];
  stacks?: PackageResourceSelectorV1[];
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
