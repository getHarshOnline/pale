/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type PackageResourceLinkV1 =
  | {
      resource_id: PackageResourceIdV1;
      scope: "local";
    }
  | {
      resource: PackageResourceRefV1;
      scope: "external";
    };
export type PackageResourceIdV1 = string;
export type PackageIdV1 = string;
export type GoverningResourceLinkV1 =
  | {
      kind?: PackageResourceKindV1 | null;
      resource_id: PackageResourceIdV1;
      scope: "local";
    }
  | {
      kind?: PackageResourceKindV1 | null;
      resource: PackageResourceRefV1;
      scope: "external";
    };
export type PackageResourceKindV1 =
  | "doctrine"
  | "operation_archetype"
  | "operation"
  | "role_archetype"
  | "role"
  | "stack"
  | "run_mode"
  | "policy_family"
  | "governing_order"
  | "capability_policy"
  | "tool_recipe";
export type SemanticOrderPurposeV1 = "composition" | "reasoning" | "execution" | "presentation";

export interface GoverningOrderDocumentV1 {
  governing_order: GoverningOrderV1;
}
export interface GoverningOrderV1 {
  constraints?: GoverningConstraintV1[];
  exclusive_authority?: GoverningExclusiveAuthorityClaimV1[];
  narrowing?: GoverningNarrowingV1[];
  obligations?: GoverningObligationV1[];
  order?: ScopedOrderRelationV1[];
  schema_version: 1;
}
export interface GoverningConstraintV1 {
  policy: PackageResourceLinkV1;
  scope: GoverningResourceLinkV1;
  subject: GoverningResourceLinkV1;
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface GoverningExclusiveAuthorityClaimV1 {
  authority: GoverningResourceLinkV1;
  scope: GoverningResourceLinkV1;
}
export interface GoverningNarrowingV1 {
  broader: GoverningResourceLinkV1;
  narrower: GoverningResourceLinkV1;
  scope: GoverningResourceLinkV1;
}
export interface GoverningObligationV1 {
  obligation: GoverningResourceLinkV1;
  scope: GoverningResourceLinkV1;
  subject: GoverningResourceLinkV1;
}
export interface ScopedOrderRelationV1 {
  after: GoverningResourceLinkV1;
  before: GoverningResourceLinkV1;
  purpose: SemanticOrderPurposeV1;
  scope: GoverningResourceLinkV1;
}
