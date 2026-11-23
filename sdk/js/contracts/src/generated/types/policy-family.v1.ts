/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type OwnedConfinedRelativePath = string;
export type PackageResourceIdV1 = string;

export interface PolicyFamilyV1 {
  body: OwnedConfinedRelativePath;
  id: PackageResourceIdV1;
  purpose: string;
  title: string;
}
