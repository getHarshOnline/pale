/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type SettlementDispositionV1 =
  "native_receipt_required" | "read_back_required" | "owner_acceptance_required" | "unsettled";
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

export interface SettlementContractV1 {
  disposition: SettlementDispositionV1;
  id: string;
  identity: EvidenceSettlementIdentityV1;
  native_receipt_contract: string | null;
  observer: ParticipantRefV1;
}
export interface EvidenceSettlementIdentityV1 {
  evidence_settlement_sha256: Sha256DigestV1;
}
