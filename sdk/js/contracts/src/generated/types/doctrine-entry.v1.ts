/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type OwnedConfinedRelativePath = string;
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
export type DoctrinePlacement = "standalone_judgment" | "operation_procedure" | "startup_policy";

export interface DoctrineEntryV1 {
  doctrine: DoctrineCatalog;
}
export interface DoctrineCatalog {
  entry: DoctrineEntry[];
  schema_version: 1;
}
export interface DoctrineEntry {
  description: string;
  doctrine: OwnedConfinedRelativePath;
  extends?: PackageResourceLinkV1[];
  id: PackageResourceIdV1;
  official_sources?: OfficialReferenceV1[];
  placement: DoctrinePlacement;
  requires?: PackageResourceLinkV1[];
  see_also?: PackageResourceLinkV1[];
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
