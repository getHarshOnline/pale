/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type WorkspaceAuthorityScopeId = string;
export type WorkspaceControllerScopeId = string;
export type DisclosureCeilingV1 = "private" | "protected" | "public";
export type WorkspaceEffectCeilingV1 = "none" | "read" | "mutate";
export type WorkspaceRightId = string;
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
export type WorkspaceResourceId = string;
export type WorkspaceObligationId = string;
export type WorkspaceResourceKindV1 =
  "principal" | "organization" | "native_application" | "participant" | "package" | "normative_pack";
export type WorkspaceModuleId = string;
export type WorkspaceDeliveryV1 = "interface" | "source";
export type WorkspaceExportId = string;
export type WorkspacePropagationV1 = "transitive" | "prevent";
export type WorkspaceVisibilityV1 = "public" | "protected" | "private";
export type WorkspaceAdoptionPolicyV1 = "manual";
export type Sha256DigestV1 = string;
export type WorkspacePublisherKeyId = string;
export type WorkspacePublisherId = string;
export type WorkspaceRecoveryAuthorityId = string;
export type WorkspaceReleaseId = string;
export type WorkspaceResourceSelectionV1 =
  | {
      authority_resolver: WorkspaceResourceId;
      kind: "principal";
      resource_id: WorkspaceResourceId;
    }
  | {
      authority_resolver: WorkspaceResourceId;
      kind: "organization";
      resource_id: WorkspaceResourceId;
    }
  | {
      authority_resolver: WorkspaceResourceId;
      kind: "native_application";
      resource_id: WorkspaceResourceId;
    }
  | {
      authority_resolver: WorkspaceResourceId;
      kind: "participant";
      resource_id: WorkspaceResourceId;
    }
  | {
      kind: "package";
      release: PackageReleaseV1;
      resource_id: WorkspaceResourceId;
    }
  | {
      kind: "normative_pack";
      release: NormativePackReleaseV1;
      resource_id: WorkspaceResourceId;
    };
export type PackageIdV1 = string;
export type PackageVersionV1 = string;
export type NormativePackAttestorV1 = string;
export type NormativePackIdV1 = string;
export type NormativePackVersionV1 = string;
export type WorkspaceComponentId = string;

export interface WorkspaceModuleV1 {
  module: WorkspaceModulePreimageV1;
  module_digest: Sha256DigestV1;
  schema: "pale/workspace-module.v1";
  schema_version: 1;
}
export interface WorkspaceModulePreimageV1 {
  /**
   * @minItems 0
   * @maxItems 1024
   */
  exports: WorkspaceExportV1[];
  /**
   * @minItems 0
   * @maxItems 1024
   */
  extensions: WorkspaceExtensionV1[];
  /**
   * @minItems 0
   * @maxItems 1024
   */
  imports: WorkspaceImportV1[];
  module_id: WorkspaceModuleId;
  publisher: WorkspacePublisherRefV1;
  release_id: WorkspaceReleaseId;
  /**
   * @minItems 0
   * @maxItems 1024
   */
  resources: WorkspaceResourceSelectionV1[];
  /**
   * @minItems 0
   * @maxItems 1024
   */
  source_components: WorkspaceSourceComponentRefV1[];
}
export interface WorkspaceExportV1 {
  ceiling: WorkspacePolicyCeilingV1;
  /**
   * @minItems 1
   * @maxItems 1024
   */
  declarations: [WorkspaceExportProjectionV1, ...WorkspaceExportProjectionV1[]];
  delivery: WorkspaceDeliveryV1;
  /**
   * @minItems 0
   * @maxItems 1024
   */
  exclusive_authority: WorkspaceExclusiveAuthorityClaimV1[];
  export_id: WorkspaceExportId;
  /**
   * @minItems 0
   * @maxItems 1024
   */
  obligations: WorkspaceObligationId[];
  propagation: WorkspacePropagationV1;
  visibility: WorkspaceVisibilityV1;
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
export interface WorkspaceExportProjectionV1 {
  declaration: WorkspaceDeclarationV1;
  local_ref: WorkspaceDeclarationRefV1;
  owner_module: WorkspaceModuleId;
}
export interface WorkspaceDeclarationDependencyV1 {
  kind: DeclarationDependencyKindV1;
  target: WorkspaceDeclarationRefV1;
}
export interface WorkspaceDeclarationRefV1 {
  declaration_id: WorkspaceDeclarationId;
  namespace: WorkspaceNamespaceId;
}
export interface WorkspaceExclusiveAuthorityClaimV1 {
  authority_resolver: WorkspaceResourceId;
  scope: WorkspaceAuthorityScopeId;
}
export interface WorkspaceExtensionV1 {
  /**
   * @minItems 0
   * @maxItems 1024
   */
  added_obligations: WorkspaceObligationId[];
  /**
   * @minItems 0
   * @maxItems 1024
   */
  added_traits: WorkspaceDeclarationRefV1[];
  namespace: WorkspaceNamespaceId;
  narrowed_ceiling?: WorkspacePolicyCeilingV1 | null;
  target: WorkspaceDeclarationRefV1;
}
export interface WorkspaceImportV1 {
  adoption_policy: WorkspaceAdoptionPolicyV1;
  edge_policy: WorkspacePolicyCeilingV1;
  module_digest: Sha256DigestV1;
  module_id: WorkspaceModuleId;
  /**
   * @minItems 1
   * @maxItems 1024
   */
  paths: [WorkspaceImportPathV1, ...WorkspaceImportPathV1[]];
  propagation: WorkspacePropagationV1;
  protected_delivery_authority?: WorkspaceResourceId | null;
  publisher: WorkspacePublisherRefV1;
  release_id: WorkspaceReleaseId;
  visibility_ceiling: WorkspaceVisibilityV1;
}
export interface WorkspaceImportPathV1 {
  export_id: WorkspaceExportId;
  local_namespace: WorkspaceNamespaceId;
  source_namespace: WorkspaceNamespaceId;
}
export interface WorkspacePublisherRefV1 {
  delegated_release_key_id: WorkspacePublisherKeyId;
  publisher_id: WorkspacePublisherId;
  recovery_authority_id: WorkspaceRecoveryAuthorityId;
  root_key_id: WorkspacePublisherKeyId;
}
export interface PackageReleaseV1 {
  content_digest: Sha256DigestV1;
  id: PackageIdV1;
  version: PackageVersionV1;
}
export interface NormativePackReleaseV1 {
  attestor: NormativePackAttestorV1;
  content_digest: Sha256DigestV1;
  id: NormativePackIdV1;
  version: NormativePackVersionV1;
}
export interface WorkspaceSourceComponentRefV1 {
  component_digest: Sha256DigestV1;
  component_id: WorkspaceComponentId;
}
