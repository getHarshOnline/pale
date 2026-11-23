/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ExecutionPathSelectionIdV1 = string;
export type SurfaceOwnershipV1 = "managed_block" | "whole_file";
export type Sha256DigestV1 = string;

export interface SurfaceDigestV1 {
  committable: boolean;
  execution_path_consumers: ExecutionPathSelectionIdV1[];
  kind: string;
  ownership: SurfaceOwnershipV1;
  rel_path: string;
  sha256: Sha256DigestV1;
}
