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

export interface ContinuityContractV1 {
  id: string;
  identity: ContinuityIdentityV1;
  /**
   * @maxItems 256
   */
  obligations: string[];
  owner: ParticipantRefV1;
}
export interface ContinuityIdentityV1 {
  continuity_sha256: Sha256DigestV1;
}
