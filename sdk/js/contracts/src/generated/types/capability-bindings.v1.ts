/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type Approval = "none" | "ask" | "gated";
export type CapabilityIdV1 = string;
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
export type HostCommandSource = "repository_capability_commands";
export type ToolObservability = "pre_tool_observable" | "permission_only" | "unobservable";
export type ToolContractIdV1 = string;
export type ProofCeilingIdV1 = string;
export type NonRootJsonPointer = string;
export type ToolOperationIdV1 = string;
export type PackageIdV1 = string;
export type PackageResourceIdV1 = string;
export type FailurePosture = "fail_closed" | "fail_open" | "retry" | "steer" | "escalate";
export type CapabilityUseKind = "operation_requirement";
export type ResolvedCapabilityRequirementPolicyV1 =
  | {
      kind: "required_block";
    }
  | {
      degraded_outcome: DegradedOutcomeIdV1;
      fallback: ResolvedCapabilityFallbackV1;
      kind: "required_degrade";
    }
  | {
      fallback: ResolvedCapabilityFallbackV1;
      kind: "optional";
    };
export type DegradedOutcomeIdV1 = string;
export type CapabilityRequirementIdV1 = string;

export interface CapabilityBindingsV1 {
  profile_digest: Sha256DigestV1;
  providers: ResolvedCapabilityProviderV1[];
  recipes?: ResolvedToolRecipeV1[];
  requirements: ResolvedCapabilityUseV1[];
  schema: "pale.agent-surface.capability-bindings.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
}
export interface ResolvedCapabilityProviderV1 {
  approval: Approval;
  capability_id: CapabilityIdV1;
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
export interface ResolvedToolRecipeV1 {
  procedure: ToolRecipe;
  resource: PackageResourceRefV1;
}
export interface ToolRecipe {
  capability: CapabilityIdV1;
  description: string;
  effect_class: EffectClass;
  evidence: ToolEvidenceContract;
  input_mapping: {
    [k: string]: string;
  };
  tool_contract: ToolContractIdV1;
  tool_operation: ToolOperationIdV1;
}
export interface ToolEvidenceContract {
  proof_ceiling: ProofCeilingIdV1;
  repository_verifier?: boolean;
  required_fields: NonRootJsonPointer[];
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface ResolvedCapabilityUseV1 {
  capability_id: CapabilityIdV1;
  effect_class: EffectClass;
  failure_posture: FailurePosture;
  kind: CapabilityUseKind;
  owner: PackageResourceRefV1;
  policy: ResolvedCapabilityRequirementPolicyV1;
  recipe?: PackageResourceRefV1 | null;
  requirement_id: CapabilityRequirementIdV1;
}
export interface ResolvedCapabilityFallbackV1 {
  guidance: string;
  recipe?: PackageResourceRefV1 | null;
}
