/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ModeAxis = "intensity" | "task_stance";
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
export type ModeScope = "runtime" | "session_only";

export interface RunModeV1 {
  run_modes: RunModeCatalog;
}
export interface RunModeCatalog {
  mode: RunMode[];
  schema_version: 1;
}
export interface RunMode {
  aliases?: string[];
  axis: ModeAxis;
  description: string;
  doctrine?: OwnedConfinedRelativePath | null;
  exact_triggers?: string[];
  extends?: PackageResourceLinkV1[];
  id: PackageResourceIdV1;
  scope: ModeScope;
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
