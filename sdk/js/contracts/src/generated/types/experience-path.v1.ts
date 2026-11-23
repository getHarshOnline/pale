/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ParticipantRefV1 =
  | {
      interface_id: string;
      kind: "interface";
    }
  | {
      application_id: string;
      kind: "application";
    }
  | {
      engine_id: string;
      kind: "engine";
    }
  | {
      capability_id: string;
      kind: "capability";
    }
  | {
      kind: "terminal_leaf";
      terminal_leaf_id: string;
    }
  | {
      kind: "plugin";
      plugin_id: string;
    }
  | {
      kind: "skill";
      skill_id: string;
    };
export type EffectClass = "compute" | "read" | "execute" | "write" | "network" | "publish";
export type NativeOutcomeV1 =
  | {
      edge_id: string;
      invocation_identity: InvocationIdentityV1;
      kind: "read_only_result";
      outcome_contract_id: string;
    }
  | {
      edge_id: string;
      invocation_identity: InvocationIdentityV1;
      kind: "effectful_outcome";
      native_effect_identity: NativeEffectIdentityV1;
      outcome_contract_id: string;
    };

export interface ExperiencePathV1 {
  assurance_subject_id: string;
  budget: PathBudgetV1;
  /**
   * @maxItems 256
   */
  capability_ceiling: string[];
  compiled_content: CompiledContentCoordinateV1;
  continuity_contract_id: string;
  /**
   * @maxItems 256
   */
  destination_consequences: DestinationConsequenceV1[];
  /**
   * @maxItems 6
   */
  effect_ceiling:
    | []
    | [EffectClass]
    | [EffectClass, EffectClass]
    | [EffectClass, EffectClass, EffectClass]
    | [EffectClass, EffectClass, EffectClass, EffectClass]
    | [EffectClass, EffectClass, EffectClass, EffectClass, EffectClass]
    | [EffectClass, EffectClass, EffectClass, EffectClass, EffectClass, EffectClass];
  entry_contract_id: string;
  evidence_carrier_id: string;
  id: string;
  identity: ResponsibilityPathIdentityV1;
  /**
   * @maxItems 256
   */
  invocations: PathInvocationV1[];
  /**
   * @maxItems 256
   */
  native_outcomes: NativeOutcomeV1[];
  /**
   * @maxItems 256
   */
  native_owner_transitions: NativeOwnerTransitionV1[];
  origin_receipt_contract_id: string;
  privacy_policy_id: string;
  /**
   * @maxItems 256
   */
  provider_binding_ids: string[];
  /**
   * @maxItems 256
   */
  reconciliation: ReconciliationRequirementV1[];
  /**
   * @maxItems 256
   */
  responsibility_contract_ids: string[];
  retention_policy_id: string;
  settlement_contract_id: string;
  termination_contract_id: string;
  /**
   * @maxItems 256
   */
  work_delegation_edge_ids: string[];
}
export interface PathBudgetV1 {
  max_cost_units: number;
  max_depth: number;
  max_effect_count: number;
  max_fan_out: number;
  max_recursion_iterations: number;
  max_resource_units: number;
  max_time_ms: number;
}
export interface CompiledContentCoordinateV1 {
  compiled_content_identity: CompiledContentIdentityV1;
  source_content_identity: SourceContentIdentityV1;
}
export interface CompiledContentIdentityV1 {
  compiled_content_sha256: Sha256DigestV1;
}
export interface SourceContentIdentityV1 {
  source_content_sha256: Sha256DigestV1;
}
export interface DestinationConsequenceV1 {
  consequence_contract_id: string;
  destination: ParticipantRefV1;
}
export interface ResponsibilityPathIdentityV1 {
  responsibility_path_sha256: Sha256DigestV1;
}
export interface PathInvocationV1 {
  edge_id: string;
  identity: InvocationIdentityV1;
}
export interface InvocationIdentityV1 {
  invocation_sha256: Sha256DigestV1;
}
export interface NativeEffectIdentityV1 {
  native_effect_sha256: Sha256DigestV1;
}
export interface NativeOwnerTransitionV1 {
  edge_id: string;
  native_owner_id: string;
  receipt_contract_id: string;
  transition_contract_id: string;
}
export interface ReconciliationRequirementV1 {
  frontier_id: string;
  obligation_contract_id: string;
}
