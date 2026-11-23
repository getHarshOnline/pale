/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type HostCommandSource = "repository_capability_commands";
export type ToolObservability = "pre_tool_observable" | "permission_only" | "unobservable";
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
export type OperationContractMechanicsV1 =
  | {
      kind: "semantic_only";
    }
  | {
      /**
       * @minItems 1
       */
      capabilities: [OperationCapabilityContractV1, ...OperationCapabilityContractV1[]];
      kind: "capability_bearing";
    };
export type CapabilityIdV1 = string;
export type EffectClass = "compute" | "read" | "execute" | "write" | "network" | "publish";
export type FailurePosture = "fail_closed" | "fail_open" | "retry" | "steer" | "escalate";
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
export type OperationCapabilityProviderStateV1 =
  | {
      kind: "unbound";
    }
  | {
      approval: Approval;
      kind: "bound";
      note?: string | null;
      provider: RouteCapabilityProviderV1;
    };
export type Approval = "none" | "ask" | "gated";
export type RouteCapabilityProviderV1 =
  | {
      execution: HostedProviderExecutionV1;
      kind: "host_native";
    }
  | {
      execution: NativeRustProviderExecutionV1;
      kind: "native_rust";
      target: string;
    }
  | {
      execution: HostedProviderExecutionV1;
      kind: "connector";
      target: string;
    };
export type HostedProviderExecutionV1 =
  | {
      kind: "unknown";
    }
  | {
      kind: "unavailable";
    }
  | {
      /**
       * @minItems 1
       */
      host_tools: [HostToolBinding, ...HostToolBinding[]];
      kind: "available";
    };
export type NativeRustProviderExecutionV1 =
  | {
      kind: "unknown";
    }
  | {
      kind: "unavailable";
    }
  | {
      /**
       * @minItems 1
       */
      host_tools: [HostToolBinding, ...HostToolBinding[]];
      kind: "available";
      native_action: string;
    };
export type CapabilityRequirementIdV1 = string;
export type SemanticDependencyKind = "doctrine" | "role" | "tool";

export interface OperationContractV1 {
  auxiliary_tools?: HostToolBinding[];
  contract_digest: Sha256DigestV1;
  dispatch: OperationDispatchV1;
  id: PackageResourceRefV1;
  invocation_policy?: SealedArtifactV1 | null;
  mechanics: OperationContractMechanicsV1;
  roles?: PackageResourceRefV1[];
  schema: "pale/operation-contract.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  semantic_dependencies?: OperationSemanticDependencyV1[];
  skill: SealedArtifactV1;
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
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface SealedArtifactV1 {
  rel_path: string;
  sha256: Sha256DigestV1;
}
export interface OperationCapabilityContractV1 {
  capability_id: CapabilityIdV1;
  effect_class: EffectClass;
  failure_posture: FailurePosture;
  policy: ResolvedCapabilityRequirementPolicyV1;
  provider_state: OperationCapabilityProviderStateV1;
  recipe?: PackageResourceRefV1 | null;
  requirement_id: CapabilityRequirementIdV1;
}
export interface ResolvedCapabilityFallbackV1 {
  guidance: string;
  recipe?: PackageResourceRefV1 | null;
}
export interface OperationSemanticDependencyV1 {
  artifact: SealedArtifactV1;
  id: PackageResourceRefV1;
  kind: SemanticDependencyKind;
}
