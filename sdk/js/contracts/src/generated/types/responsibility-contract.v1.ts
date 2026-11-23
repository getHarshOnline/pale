/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

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
export type ResponsibilityV1 =
  | {
      kind: "communication";
    }
  | {
      kind: "organizational_work";
    }
  | {
      kind: "crm_business_state";
    }
  | {
      kind: "engineering_execution";
    }
  | {
      kind: "context_memory";
    }
  | {
      kind: "terminal_harness_execution";
    }
  | {
      kind: "native_event_publication";
    }
  | {
      kind: "native_effect_read_back";
    }
  | {
      contract_id: string;
      contract_schema_identity: ContractSchemaIdentityV1;
      kind: "extension";
      namespace_id: string;
    };
export type SchemaId = string;
export type Sha256DigestV1 = string;

export interface ResponsibilityContractV1 {
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
  id: string;
  participant: ParticipantRefV1;
  /**
   * @minItems 1
   * @maxItems 256
   */
  responsibilities: [ResponsibilityV1, ...ResponsibilityV1[]];
}
export interface ContractSchemaIdentityV1 {
  schema_id: SchemaId;
  schema_sha256: Sha256DigestV1;
}
