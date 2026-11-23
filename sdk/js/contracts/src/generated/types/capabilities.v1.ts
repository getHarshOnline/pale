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
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)*$".
 */
export type NonEmptyCommandList = string[];
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

export interface CapabilitiesV1 {
  boundary?: BoundaryCapability;
  capability_bindings?: {
    [k: string]: CapabilityBinding;
  };
  capability_narrowing?: CapabilityNarrowingV1;
  lifecycle?: LifecycleBehaviorCatalog | null;
  operation_bindings?: OperationBindingEntryV1[];
  repository_commands?: {
    [k: string]: RepositoryCommandCapabilities;
  };
  run_policies?: RunPolicyEntry[];
  semantic?: SemanticSelection;
}
export interface BoundaryCapability {
  forbidden_token_authority: string[];
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
export interface LifecycleBehaviorCatalog {
  behaviour: LifecycleBehavior[];
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
export interface OperationBindingEntryV1 {
  binding: OperationBinding;
  operation: PackageResourceRefV1;
}
export interface OperationBinding {
  selection: OperationSelection;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[a-z][a-z0-9_-]*$".
 */
export interface RepositoryCommandCapabilities {
  commands: {
    [k: string]: NonEmptyCommandList;
  };
  mutating_not_readonly?: string[];
  note?: string;
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
