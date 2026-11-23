/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type MachineLocalEffectKind = "git_exclude_protection";
export type UnixFileMode = number;
export type MachineLocalEffectObservationV1 =
  | {
      action: "create";
      created_directories: string[];
      post: MachineLocalPostFileV1;
    }
  | {
      action: "update";
      post: MachineLocalPostFileV1;
      pre_sha256: Sha256DigestV1;
    }
  | {
      action: "splice";
      post: MachineLocalPostFileV1;
      pre_sha256: Sha256DigestV1;
    }
  | {
      action: "unchanged";
      state: MachineLocalPostFileV1;
    }
  | {
      action: "delete";
      pre_sha256: Sha256DigestV1;
    };
export type FilesystemDeviceIdV1 = string;
export type FilesystemInodeIdV1 = string;
export type NormativePackAttestorV1 = string;
export type NormativePackIdV1 = string;
export type NormativePackVersionV1 = string;
export type PackageIdV1 = string;
export type PackageVersionV1 = string;
export type LogicalRepositoryIdV1 = string;
export type SurfaceOperationObservationV1 =
  | {
      action: "create";
      target: SurfaceCreationObservationV1;
    }
  | {
      action: "adopt";
      post_sha256: Sha256DigestV1;
      pre_sha256: Sha256DigestV1;
    }
  | {
      action: "update";
      post_sha256: Sha256DigestV1;
      pre_sha256: Sha256DigestV1;
    }
  | {
      action: "unchanged";
      disposition: UnchangedDispositionV1;
      sha256: Sha256DigestV1;
    }
  | {
      action: "delete";
      pre_sha256: Sha256DigestV1;
    }
  | {
      action: "splice";
      post_sha256: Sha256DigestV1;
      pre_sha256: Sha256DigestV1;
    };
export type SurfaceCreationObservationV1 =
  | {
      post_sha256: Sha256DigestV1;
      target: "absent_file";
    }
  | {
      post_sha256: Sha256DigestV1;
      pre_sha256: Sha256DigestV1;
      target: "existing_file";
    };
export type UnchangedDispositionV1 = "verified" | "mode_repaired";
export type MaterializationReceiptTransitionV1 =
  | {
      kind: "initialized";
      post_lineage_digest: Sha256DigestV1;
      post_state_digest: Sha256DigestV1;
    }
  | {
      kind: "advanced";
      post_lineage_digest: Sha256DigestV1;
      post_state_digest: Sha256DigestV1;
      prior_lineage_digest: Sha256DigestV1;
      prior_state_digest: Sha256DigestV1;
    }
  | {
      kind: "removed";
      prior_lineage_digest: Sha256DigestV1;
      prior_state_digest: Sha256DigestV1;
    }
  | {
      kind: "no_owned_state";
    };

export interface ReceiptV1 {
  deferred_effects: DeferredEffectV1[];
  instance_digest: Sha256DigestV1;
  machine_local_effects: MachineLocalReceiptOperationV1[];
  manifest_digest: Sha256DigestV1;
  normative_pack_closure: NormativePackReleaseV1[];
  one_shot_compilation_digest: Sha256DigestV1;
  package_closure: PackageReleaseV1[];
  package_semantic_ir_digest: Sha256DigestV1;
  plan_digest: Sha256DigestV1;
  profile_digest: Sha256DigestV1;
  receipt_digest: Sha256DigestV1;
  repository_id: LogicalRepositoryIdV1;
  schema: "pale.agent-surface.receipt.v1";
  schema_manifest_sha256: Sha256DigestV1;
  schema_set_sha256: Sha256DigestV1;
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  surface_operations: ReceiptOperation[];
  surface_plan_digest: Sha256DigestV1;
  transition: MaterializationReceiptTransitionV1;
}
export interface DeferredEffectV1 {
  host: string;
  kind: string;
  note: string;
  target: string;
}
export interface MachineLocalReceiptOperationV1 {
  kind: MachineLocalEffectKind;
  mode: UnixFileMode;
  observation: MachineLocalEffectObservationV1;
  rel_path: string;
  root: MachineLocalRootIdentityV1;
}
export interface MachineLocalPostFileV1 {
  identity: MachineLocalFileIdentityV1;
  sha256: Sha256DigestV1;
}
export interface MachineLocalFileIdentityV1 {
  device: FilesystemDeviceIdV1;
  inode: FilesystemInodeIdV1;
}
export interface MachineLocalRootIdentityV1 {
  canonical_path: string;
  device: FilesystemDeviceIdV1;
  inode: FilesystemInodeIdV1;
}
export interface NormativePackReleaseV1 {
  attestor: NormativePackAttestorV1;
  content_digest: Sha256DigestV1;
  id: NormativePackIdV1;
  version: NormativePackVersionV1;
}
export interface PackageReleaseV1 {
  content_digest: Sha256DigestV1;
  id: PackageIdV1;
  version: PackageVersionV1;
}
export interface ReceiptOperation {
  mode: UnixFileMode;
  observation: SurfaceOperationObservationV1;
  rel_path: string;
}
