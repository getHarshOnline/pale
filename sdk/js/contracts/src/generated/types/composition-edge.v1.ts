/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type CompositionEdgeV1 =
  | {
      /**
       * @maxItems 6
       */
      authority_ceiling:
        | []
        | [EffectClass]
        | [EffectClass, EffectClass]
        | [EffectClass, EffectClass, EffectClass]
        | [EffectClass, EffectClass, EffectClass, EffectClass]
        | [EffectClass, EffectClass, EffectClass, EffectClass, EffectClass]
        | [EffectClass, EffectClass, EffectClass, EffectClass, EffectClass, EffectClass];
      budget: PathBudgetV1;
      from: ParticipantRefV1;
      id: string;
      kind: "work_delegation";
      request_translation_id: string;
      responsibility_contract_id: string;
      result_translation_id: string;
      to: ParticipantRefV1;
    }
  | {
      embedded: ParticipantRefV1;
      host: ParticipantRefV1;
      id: string;
      kind: "ui_embedding";
    }
  | {
      dependency: ParticipantRefV1;
      id: string;
      kind: "plugin_dependency";
      plugin: ParticipantRefV1;
    }
  | {
      application: ParticipantRefV1;
      caller: ParticipantRefV1;
      id: string;
      kind: "native_application_call";
      provider_binding_id: string;
      request_translation_id: string;
      result_translation_id: string;
    }
  | {
      id: string;
      kind: "terminal_leaf";
      owner: ParticipantRefV1;
      terminal_leaf: ParticipantRefV1;
      termination_contract_id: string;
    }
  | {
      id: string;
      kind: "observation_settlement";
      observer: ParticipantRefV1;
      settlement_contract_id: string;
      subject: ParticipantRefV1;
    };
export type EffectClass = "compute" | "read" | "execute" | "write" | "network" | "publish";
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

export interface PathBudgetV1 {
  max_cost_units: number;
  max_depth: number;
  max_effect_count: number;
  max_fan_out: number;
  max_recursion_iterations: number;
  max_resource_units: number;
  max_time_ms: number;
}
