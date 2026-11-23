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
export type EntryOriginRefV1 =
  | {
      kind: "user";
      user_entry_id: string;
    }
  | {
      interface_entry_id: string;
      kind: "interface";
    }
  | {
      kind: "participant";
      participant_entry_id: string;
    };

export interface EntryContractV1 {
  entry: ParticipantRefV1;
  id: string;
  origin: EntryOriginRefV1;
  responsibility_contract_id: string;
}
