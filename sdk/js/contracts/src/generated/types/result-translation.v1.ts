/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

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

export interface ResultTranslationV1 {
  from: ParticipantRefV1;
  id: string;
  input_contract: string;
  output_contract: string;
  preserves_evidence: boolean;
  to: ParticipantRefV1;
}
