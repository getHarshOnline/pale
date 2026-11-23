/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type WorkspaceComponentId = string;
export type WorkspaceDeclarationV1 =
  | {
      /**
       * @minItems 0
       * @maxItems 1024
       */
      dependencies: WorkspaceDeclarationDependencyV1[];
      id: WorkspaceDeclarationId;
      kind: "trait";
      namespace: WorkspaceNamespaceId;
    }
  | {
      /**
       * @minItems 0
       * @maxItems 1024
       */
      dependencies: WorkspaceDeclarationDependencyV1[];
      id: WorkspaceDeclarationId;
      kind: "attribute";
      namespace: WorkspaceNamespaceId;
      value_kind: WorkspaceValueKindV1;
    }
  | {
      /**
       * @minItems 0
       * @maxItems 1024
       */
      dependencies: WorkspaceDeclarationDependencyV1[];
      from: WorkspaceDeclarationRefV1;
      id: WorkspaceDeclarationId;
      kind: "relationship";
      namespace: WorkspaceNamespaceId;
      to: WorkspaceDeclarationRefV1;
    }
  | {
      ceiling: WorkspacePolicyCeilingV1;
      /**
       * @minItems 0
       * @maxItems 1024
       */
      dependencies: WorkspaceDeclarationDependencyV1[];
      /**
       * @minItems 0
       * @maxItems 1024
       */
      exclusive_authority: WorkspaceExclusiveAuthorityClaimV1[];
      id: WorkspaceDeclarationId;
      kind: "policy";
      namespace: WorkspaceNamespaceId;
      /**
       * @minItems 0
       * @maxItems 1024
       */
      obligations: WorkspaceObligationId[];
    }
  | {
      /**
       * @minItems 0
       * @maxItems 1024
       */
      dependencies: WorkspaceDeclarationDependencyV1[];
      id: WorkspaceDeclarationId;
      kind: "resource";
      namespace: WorkspaceNamespaceId;
      resource_kind: WorkspaceResourceKindV1;
    };
export type DeclarationDependencyKindV1 = "positive" | "negation" | "effect" | "callback" | "unbounded_generation";
export type WorkspaceDeclarationId = string;
export type WorkspaceNamespaceId = string;
export type WorkspaceValueKindV1 = "boolean" | "integer" | "text" | "logical_id";
export type WorkspaceAuthorityScopeId = string;
export type WorkspaceControllerScopeId = string;
export type DisclosureCeilingV1 = "private" | "protected" | "public";
export type WorkspaceEffectCeilingV1 = "none" | "read" | "mutate";
export type WorkspaceRightId = string;
export type WorkspaceResourceId = string;
export type WorkspaceObligationId = string;
export type WorkspaceResourceKindV1 =
  "principal" | "organization" | "native_application" | "participant" | "package" | "normative_pack";
export type WorkspaceModuleId = string;

export interface WorkspaceLanguagePrimitivesV1 {
  component_id: WorkspaceComponentId;
  /**
   * @minItems 0
   * @maxItems 1024
   */
  declarations: WorkspaceDeclarationV1[];
  module_id: WorkspaceModuleId;
  schema: "pale/workspace-language-primitives.v1";
  schema_version: 1;
}
export interface WorkspaceDeclarationDependencyV1 {
  kind: DeclarationDependencyKindV1;
  target: WorkspaceDeclarationRefV1;
}
export interface WorkspaceDeclarationRefV1 {
  declaration_id: WorkspaceDeclarationId;
  namespace: WorkspaceNamespaceId;
}
export interface WorkspacePolicyCeilingV1 {
  authorities: WorkspaceAuthorityCeilingV1;
  controllers: WorkspaceControllerCeilingV1;
  disclosure: DisclosureCeilingV1;
  effect: WorkspaceEffectCeilingV1;
  model_use_permitted: boolean;
  retention_seconds: number;
  rights: WorkspaceRightsCeilingV1;
}
export interface WorkspaceAuthorityCeilingV1 {
  /**
   * @minItems 0
   * @maxItems 1024
   */
  permitted: WorkspaceAuthorityScopeId[];
}
export interface WorkspaceControllerCeilingV1 {
  /**
   * @minItems 0
   * @maxItems 1024
   */
  permitted: WorkspaceControllerScopeId[];
}
export interface WorkspaceRightsCeilingV1 {
  /**
   * @minItems 0
   * @maxItems 1024
   */
  permitted: WorkspaceRightId[];
}
export interface WorkspaceExclusiveAuthorityClaimV1 {
  authority_resolver: WorkspaceResourceId;
  scope: WorkspaceAuthorityScopeId;
}
