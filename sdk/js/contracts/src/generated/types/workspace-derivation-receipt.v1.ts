/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type WorkspaceDerivationImpactV1 = WorkspaceDerivationImpactStateV1 & {
  subject: WorkspaceDerivationSubjectV1;
  [k: string]: any;
};
export type WorkspaceDerivationImpactStateV1 =
  | {
      impact: "affected";
      /**
       * @minItems 1
       * @maxItems 6154
       */
      root_digests: [Sha256DigestV1, ...Sha256DigestV1[]];
      [k: string]: any;
    }
  | {
      impact: "unaffected";
      [k: string]: any;
    };
export type WorkspaceDerivationSubjectV1 =
  | {
      source_id: WorkspaceDerivationSourceId;
      subject: "source";
    }
  | {
      component_id: WorkspaceComponentId;
      subject: "component";
    }
  | {
      module_id: WorkspaceModuleId;
      subject: "module";
    }
  | {
      declaration: WorkspaceDeclarationRefV1;
      owner_module: WorkspaceModuleId;
      subject: "declaration";
    }
  | {
      export_id: WorkspaceExportId;
      importer_module: WorkspaceModuleId;
      local_namespace: WorkspaceNamespaceId;
      source_namespace: WorkspaceNamespaceId;
      subject: "import";
      upstream_module: WorkspaceModuleId;
      upstream_release: WorkspaceReleaseId;
    }
  | {
      namespace: WorkspaceNamespaceId;
      owner_module: WorkspaceModuleId;
      subject: "extension";
      target: WorkspaceDeclarationRefV1;
    }
  | {
      declaration: WorkspaceDeclarationRefV1;
      owner_module: WorkspaceModuleId;
      subject: "policy";
    }
  | {
      subject: "responsibility_closure";
    }
  | {
      subject: "organizational_translation_closure";
    }
  | {
      subject: "experience_composition";
    }
  | {
      owner_module: WorkspaceModuleId;
      resource_id: WorkspaceResourceId;
      subject: "resource";
    }
  | {
      subject: "resolution_lock";
    }
  | {
      subject: "universe_lock";
    }
  | {
      subject: "workspace";
      workspace_id: WorkspaceId;
    };
export type WorkspaceDerivationSourceId = string;
export type WorkspaceComponentId = string;
export type WorkspaceModuleId = string;
export type WorkspaceDeclarationId = string;
export type WorkspaceNamespaceId = string;
export type WorkspaceExportId = string;
export type WorkspaceReleaseId = string;
export type WorkspaceResourceId = string;
export type WorkspaceId = string;
export type WorkspaceDerivationOriginV1 =
  | {
      origin: "source";
      source_id: WorkspaceDerivationSourceId;
    }
  | {
      module_digest: Sha256DigestV1;
      module_id: WorkspaceModuleId;
      origin: "module_release";
      release_id: WorkspaceReleaseId;
    }
  | {
      export_id: WorkspaceExportId;
      importer_module: WorkspaceModuleId;
      local_namespace: WorkspaceNamespaceId;
      origin: "import";
      source_namespace: WorkspaceNamespaceId;
      upstream_module: WorkspaceModuleId;
      upstream_release: WorkspaceReleaseId;
    }
  | {
      namespace: WorkspaceNamespaceId;
      origin: "extension";
      owner_module: WorkspaceModuleId;
      target: WorkspaceDeclarationRefV1;
    }
  | {
      origin: "compilation";
    };
export type WorkspaceDerivationOutputV1 =
  | {
      byte_len: number;
      output: "source_content";
      source_digest: Sha256DigestV1;
      source_path: OwnedConfinedRelativePath;
    }
  | {
      component_digest: Sha256DigestV1;
      output: "component";
    }
  | {
      module_digest: Sha256DigestV1;
      output: "module";
    }
  | {
      output: "declaration";
      value_digest: Sha256DigestV1;
    }
  | {
      output: "import_surface";
      surface_digest: Sha256DigestV1;
    }
  | {
      output: "extension";
      value_digest: Sha256DigestV1;
    }
  | {
      output: "effective_policy";
      value_digest: Sha256DigestV1;
    }
  | {
      closure_digest: Sha256DigestV1;
      output: "responsibility_closure";
    }
  | {
      closure_digest: Sha256DigestV1;
      output: "organizational_translation_closure";
    }
  | {
      output: "experience_composition";
      source_digest: Sha256DigestV1;
    }
  | {
      output: "resource";
      value_digest: Sha256DigestV1;
    }
  | {
      output: "resolution_lock";
      resolution_digest: Sha256DigestV1;
    }
  | {
      output: "universe_lock";
      universe_digest: Sha256DigestV1;
    }
  | {
      output: "semantic_ir";
      workspace_semantic_ir_digest: Sha256DigestV1;
    };
export type OwnedConfinedRelativePath = string;
export type WorkspaceDerivationRuleV1 =
  | "source_admission"
  | "component_admission"
  | "module_sealing"
  | "declaration_linking"
  | "import_linking"
  | "extension_application"
  | "policy_composition"
  | "responsibility_lowering"
  | "organizational_translation"
  | "experience_composition"
  | "resource_selection"
  | "resolution_locking"
  | "universe_locking"
  | "semantic_ir_sealing";
export type WorkspaceDerivationRootV1 = WorkspaceDerivationRootChangeV1 & {
  root_digest: Sha256DigestV1;
  subject: WorkspaceDerivationSubjectV1;
  [k: string]: any;
};
export type WorkspaceDerivationRootChangeV1 =
  | {
      change: "added";
      current_node: Sha256DigestV1;
      [k: string]: any;
    }
  | {
      change: "modified";
      current_node: Sha256DigestV1;
      prior_output: WorkspaceDerivationOutputV1;
      [k: string]: any;
    }
  | {
      change: "removed";
      prior_output: WorkspaceDerivationOutputV1;
      [k: string]: any;
    };

export interface WorkspaceDerivationReceiptV1 {
  current_resolution_digest: Sha256DigestV1;
  current_universe_digest: Sha256DigestV1;
  /**
   * @minItems 0
   * @maxItems 10251
   */
  impacts: WorkspaceDerivationImpactV1[];
  /**
   * @minItems 0
   * @maxItems 10251
   */
  nodes: WorkspaceDerivationNodeV1[];
  prior_resolution_digest: Sha256DigestV1;
  prior_universe_digest: Sha256DigestV1;
  receipt_digest: Sha256DigestV1;
  /**
   * @minItems 0
   * @maxItems 6154
   */
  roots: WorkspaceDerivationRootV1[];
  schema: "pale/workspace-derivation-receipt.v1";
  schema_version: 1;
  /**
   * @minItems 0
   * @maxItems 3077
   */
  sources: WorkspaceDerivationSourceV1[];
  workspace_semantic_ir_digest: Sha256DigestV1;
}
export interface WorkspaceDeclarationRefV1 {
  declaration_id: WorkspaceDeclarationId;
  namespace: WorkspaceNamespaceId;
}
export interface WorkspaceDerivationNodeV1 {
  /**
   * @minItems 0
   * @maxItems 5124
   */
  inputs: Sha256DigestV1[];
  node_digest: Sha256DigestV1;
  origin: WorkspaceDerivationOriginV1;
  output: WorkspaceDerivationOutputV1;
  rule: WorkspaceDerivationRuleV1;
  subject: WorkspaceDerivationSubjectV1;
}
export interface WorkspaceDerivationSourceV1 {
  byte_len: number;
  source_digest: Sha256DigestV1;
  source_id: WorkspaceDerivationSourceId;
  source_path: OwnedConfinedRelativePath;
}
