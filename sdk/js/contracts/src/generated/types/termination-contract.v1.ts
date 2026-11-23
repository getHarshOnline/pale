/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type TerminationKindV1 = "completed" | "cancelled" | "failed" | "expired" | "effect_unknown";
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

export interface TerminationContractV1 {
  id: string;
  /**
   * @minItems 1
   * @maxItems 5
   */
  kinds:
    | [TerminationKindV1]
    | [TerminationKindV1, TerminationKindV1]
    | [TerminationKindV1, TerminationKindV1, TerminationKindV1]
    | [TerminationKindV1, TerminationKindV1, TerminationKindV1, TerminationKindV1]
    | [TerminationKindV1, TerminationKindV1, TerminationKindV1, TerminationKindV1, TerminationKindV1];
  owner: ParticipantRefV1;
  terminal_leaf: ParticipantRefV1 | null;
}
