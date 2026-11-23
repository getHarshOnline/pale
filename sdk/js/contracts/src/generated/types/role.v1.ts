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

export interface RoleV1 {
  roles: RoleCatalog;
}
export interface RoleCatalog {
  archetype?: RoleArchetype[];
  role: SpecialistRole[];
  schema_version: 1;
}
export interface RoleArchetype {
  description: string;
  doctrine: OwnedConfinedRelativePath;
  doctrine_refs?: PackageResourceLinkV1[];
  extends?: PackageResourceLinkV1[];
  id: PackageResourceIdV1;
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface SpecialistRole {
  applies_to?: PackageResourceLinkV1[];
  description: string;
  doctrine: OwnedConfinedRelativePath;
  doctrine_refs?: PackageResourceLinkV1[];
  extends?: PackageResourceLinkV1[];
  id: PackageResourceIdV1;
  inherits_lenses?: PackageResourceLinkV1[];
}
