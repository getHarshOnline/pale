/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type FilesystemDeviceIdV1 = string;
export type FilesystemInodeIdV1 = string;

export interface MachineLocalRootIdentityV1 {
  canonical_path: string;
  device: FilesystemDeviceIdV1;
  inode: FilesystemInodeIdV1;
}
