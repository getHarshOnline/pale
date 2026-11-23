/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type MachineLocalEffectKind = "git_exclude_protection";
export type FilesystemDeviceIdV1 = string;
export type FilesystemInodeIdV1 = string;
export type MachineLocalEffectTransitionV1 =
  | {
      action: "create";
      create_directories: string[];
      post: PlannedFileV1;
    }
  | {
      action: "update";
      post: PlannedFileV1;
      prior: ExistingMachineLocalFileV1;
    }
  | {
      action: "splice";
      post: PlannedFileV1;
      prior: ExistingMachineLocalFileV1;
    }
  | {
      action: "unchanged";
      state: ExistingMachineLocalFileV1;
    }
  | {
      action: "delete";
      prior: ExistingMachineLocalFileV1;
    };
export type UnixFileMode = number;
export type Sha256DigestV1 = string;
export type NormativePackAttestorV1 = string;
export type NormativePackIdV1 = string;
export type NormativePackVersionV1 = string;
export type SurfaceOperationTransitionV1 =
  | {
      action: "create";
      desired_sha256: Sha256DigestV1;
      ownership: SurfaceOwnershipV1;
    }
  | {
      action: "adopt";
      desired_sha256: Sha256DigestV1;
      ownership: SurfaceOwnershipV1;
    }
  | {
      action: "update";
      desired_sha256: Sha256DigestV1;
      ownership: SurfaceOwnershipV1;
    }
  | {
      action: "unchanged";
      owned_sha256: Sha256DigestV1;
      ownership: SurfaceOwnershipV1;
    }
  | {
      action: "delete";
      prior_owned_sha256: Sha256DigestV1;
    }
  | {
      action: "splice";
      prior_owned_sha256: Sha256DigestV1;
    };
export type SurfaceOwnershipV1 = "managed_block" | "whole_file";
export type PackageIdV1 = string;
export type PackageVersionV1 = string;
export type LogicalRepositoryIdV1 = string;
export type BaselineStateV1 =
  | {
      state: "absent";
    }
  | {
      mode: UnixFileMode;
      sha256: Sha256DigestV1;
      state: "present";
    };
export type MaterializationStateTransitionV1 =
  | {
      kind: "initialize";
      next_state: MaterializationStateV1;
    }
  | {
      kind: "advance";
      next_state: MaterializationStateV1;
      prior: PriorMaterializationStateV1;
    }
  | {
      kind: "remove";
      prior: PriorMaterializationStateV1;
    }
  | {
      kind: "remove_unowned";
    };
export type ExecutionPathSelectionIdV1 = string;

export interface MaterializationPlanV1 {
  machine_local_effects: MachineLocalEffectV1[];
  one_shot_compilation_digest: Sha256DigestV1;
  plan_digest: Sha256DigestV1;
  schema: "pale.materialization-plan.v1";
  schema_manifest_sha256: Sha256DigestV1;
  schema_set_sha256: Sha256DigestV1;
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  surface_plan: AgentSurfacePlanV1;
}
export interface MachineLocalEffectV1 {
  kind: MachineLocalEffectKind;
  rel_path: string;
  retained_directories: MachineLocalDirectoryIdentity[];
  root: MachineLocalRootIdentityV1;
  transition: MachineLocalEffectTransitionV1;
}
export interface MachineLocalDirectoryIdentity {
  device: FilesystemDeviceIdV1;
  inode: FilesystemInodeIdV1;
  rel_path: string;
}
export interface MachineLocalRootIdentityV1 {
  canonical_path: string;
  device: FilesystemDeviceIdV1;
  inode: FilesystemInodeIdV1;
}
export interface PlannedFileV1 {
  mode: UnixFileMode;
  sha256: Sha256DigestV1;
}
export interface ExistingMachineLocalFileV1 {
  identity: MachineLocalFileIdentityV1;
  mode: UnixFileMode;
  sha256: Sha256DigestV1;
}
export interface MachineLocalFileIdentityV1 {
  device: FilesystemDeviceIdV1;
  inode: FilesystemInodeIdV1;
}
export interface AgentSurfacePlanV1 {
  admission: string;
  compiler_version: string;
  deferred_effects: DeferredEffectV1[];
  instance_digest: Sha256DigestV1;
  manifest_digest: Sha256DigestV1;
  normative_pack_closure: NormativePackReleaseV1[];
  one_shot_compilation_digest: Sha256DigestV1;
  operation_marker: string;
  operations: Operation[];
  package_closure: PackageReleaseV1[];
  package_semantic_ir_digest: Sha256DigestV1;
  plan_digest: Sha256DigestV1;
  profile_digest: Sha256DigestV1;
  renderer_version: string;
  repository_id: LogicalRepositoryIdV1;
  schema: "pale.agent-surface-plan.v1";
  schema_manifest_sha256: Sha256DigestV1;
  schema_set_sha256: Sha256DigestV1;
  schema_sha256: Sha256DigestV1;
  target_baseline: BaselineEntry[];
  transition: MaterializationStateTransitionV1;
}
export interface DeferredEffectV1 {
  host: string;
  kind: string;
  note: string;
  target: string;
}
export interface NormativePackReleaseV1 {
  attestor: NormativePackAttestorV1;
  content_digest: Sha256DigestV1;
  id: NormativePackIdV1;
  version: NormativePackVersionV1;
}
export interface Operation {
  committable: boolean;
  kind: string;
  mode: UnixFileMode;
  rel_path: string;
  transition: SurfaceOperationTransitionV1;
}
export interface PackageReleaseV1 {
  content_digest: Sha256DigestV1;
  id: PackageIdV1;
  version: PackageVersionV1;
}
export interface BaselineEntry {
  observation: BaselineStateV1;
  rel_path: string;
}
export interface MaterializationStateV1 {
  instance_digest: Sha256DigestV1;
  manifest_digest: Sha256DigestV1;
  one_shot_compilation_digest: Sha256DigestV1;
  profile_digest: Sha256DigestV1;
  repository_id: LogicalRepositoryIdV1;
  schema: "pale.agent-surface.materialization-state.v1";
  schema_manifest_sha256: Sha256DigestV1;
  schema_set_sha256: Sha256DigestV1;
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  state_digest: Sha256DigestV1;
  surfaces: SurfaceDigestV1[];
}
export interface SurfaceDigestV1 {
  committable: boolean;
  execution_path_consumers: ExecutionPathSelectionIdV1[];
  kind: string;
  ownership: SurfaceOwnershipV1;
  rel_path: string;
  sha256: Sha256DigestV1;
}
export interface PriorMaterializationStateV1 {
  one_shot_compilation_digest: Sha256DigestV1;
  state_digest: Sha256DigestV1;
}
