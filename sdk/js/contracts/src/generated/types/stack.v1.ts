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
export type OfficialSourceUrl = string;

export interface StackV1 {
  stacks: StackCatalog;
}
export interface StackCatalog {
  schema_version: 1;
  stack: Stack[];
}
export interface Stack {
  description: string;
  doctrine_refs?: PackageResourceLinkV1[];
  extends?: PackageResourceLinkV1[];
  id: PackageResourceIdV1;
  official_sources?: OfficialReferenceV1[];
  role_doctrine_refs?: PackageResourceLinkV1[];
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface OfficialReferenceV1 {
  checked?: string | null;
  jurisdiction?: string | null;
  label: string;
  url: OfficialSourceUrl;
}
