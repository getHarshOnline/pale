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

export interface ParticipantProviderManifestV1 {
  id: string;
  /**
   * @maxItems 1024
   */
  participants: ParticipantRefV1[];
  /**
   * @maxItems 1024
   */
  providers: ProviderCompatibilityDeclarationV1[];
}
export interface ProviderCompatibilityDeclarationV1 {
  /**
   * @maxItems 256
   */
  capability_ids: string[];
  /**
   * @maxItems 256
   */
  compatible_provider_ids: string[];
  id: string;
  participant: ParticipantRefV1;
}
