/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ResidentControllerIdV1 = string;
/**
 * @maxItems 1024
 */
export type ResourceContinuityControllerSetV1 = ResidentControllerIdV1[];
export type ResidentPrincipalIdV1 = string;
/**
 * @maxItems 1024
 */
export type ResourceContinuityPrincipalSetV1 = ResidentPrincipalIdV1[];
export type ResidentRecordRefV1 = string;
export type Sha256DigestV1 = string;
export type ContinuityOptionalContractV1 =
  | {
      state: "absent";
    }
  | {
      contract: ContinuityRecordCoordinateV1;
      state: "bound";
    };
export type ResourceContinuityAccessRoleV1 = "reader" | "proposal_writer" | "effect_writer";
export type ResourceContinuitySubjectV1 =
  | {
      identity: ResidentPrincipalIdV1;
      kind: "principal";
    }
  | {
      identity: ResidentControllerIdV1;
      kind: "controller";
    };
/**
 * @maxItems 1024
 */
export type ResourceContinuityGrantSetV1 = ResourceContinuityGrantV1[];
export type OwnerSystemIdV1 = string;
export type ResourceConvergenceV1 =
  | {
      mode: "exclusive_writer";
      writer: ResourceContinuityWriterV1;
    }
  | {
      conflict_owner: ContinuityAuthorityBindingV1;
      durable_metadata: ResourceConvergenceMetadataSetV1;
      merge_owner: ContinuityAuthorityBindingV1;
      mode: "merge_protocol";
      protocol: ContinuityRecordCoordinateV1;
    };
/**
 * @minItems 1
 * @maxItems 7
 */
export type ResourceConvergenceMetadataSetV1 =
  | [ResourceConvergenceMetadataV1]
  | [ResourceConvergenceMetadataV1, ResourceConvergenceMetadataV1]
  | [ResourceConvergenceMetadataV1, ResourceConvergenceMetadataV1, ResourceConvergenceMetadataV1]
  | [
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1
    ]
  | [
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1
    ]
  | [
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1
    ]
  | [
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1,
      ResourceConvergenceMetadataV1
    ];
export type ResourceConvergenceMetadataV1 =
  | "tombstone"
  | "writer_clock"
  | "causal_clock"
  | "version_vector"
  | "merge_ancestry"
  | "per_field_origin"
  | "conflict_marker";
export type ResourceContinuityCopyClassV1 = "local_replica" | "remote_replica" | "view" | "cache" | "archive";
export type ResourceContinuityCopyIdV1 = string;
export type ContinuityHolderV1 =
  | {
      kind: "device";
      scope: ResourceContinuityDeviceScopeIdV1;
    }
  | {
      kind: "machine";
      scope: ResourceContinuityMachineScopeIdV1;
    }
  | {
      kind: "service";
      scope: ResidentRecordRefV1;
    }
  | {
      kind: "location";
      scope: ResidentRecordRefV1;
    };
export type ResourceContinuityDeviceScopeIdV1 = string;
export type ResourceContinuityMachineScopeIdV1 = string;
/**
 * @maxItems 1024
 */
export type ResourceContinuityCopySetV1 = ResourceContinuityCopyV1[];
/**
 * @minItems 10
 * @maxItems 10
 */
export type ResourceContinuityMechanismTableV1 = [
  ResourceContinuityMechanismPolicyV1,
  ResourceContinuityMechanismPolicyV1,
  ResourceContinuityMechanismPolicyV1,
  ResourceContinuityMechanismPolicyV1,
  ResourceContinuityMechanismPolicyV1,
  ResourceContinuityMechanismPolicyV1,
  ResourceContinuityMechanismPolicyV1,
  ResourceContinuityMechanismPolicyV1,
  ResourceContinuityMechanismPolicyV1,
  ResourceContinuityMechanismPolicyV1
];
export type ResourceContinuityMechanismDispositionV1 =
  | {
      state: "forbidden";
    }
  | {
      contract: ContinuityRecordCoordinateV1;
      state: "governed";
    };
export type ResourceContinuityMechanismKindV1 =
  | "sync"
  | "replication"
  | "transport"
  | "export_import"
  | "projection_regeneration"
  | "backup_restore"
  | "checkpoint_replay"
  | "archive"
  | "migration_cutover"
  | "recovery";
export type ResourceContinuityProfileIdV1 = string;
export type RecoveryPointObjectiveSecondsV1 = number;
export type RecoveryTimeObjectiveSecondsV1 = number;
export type WorkspaceResourceId = string;
/**
 * @maxItems 1024
 */
export type ResourceContinuityDeviceScopeSetV1 = ResourceContinuityDeviceScopeIdV1[];
/**
 * @maxItems 1024
 */
export type ResourceContinuityMachineScopeSetV1 = ResourceContinuityMachineScopeIdV1[];
export type ResourceSessionContinuityV1 =
  | {
      state: "unrelated";
    }
  | {
      relation_contract: ContinuityRecordCoordinateV1;
      state: "bound";
    };

export interface ResourceContinuityProfileV1 {
  access: ResourceContinuityAccessV1;
  authority: ResourceContinuityAuthorityV1;
  convergence: ResourceConvergenceV1;
  copies: ResourceContinuityCopySetV1;
  cutover: ResourceContinuityCutoverV1;
  evidence: ResourceContinuityEvidenceV1;
  governance: ResourceContinuityGovernanceV1;
  mechanisms: ResourceContinuityMechanismTableV1;
  profile_id: ResourceContinuityProfileIdV1;
  recovery: ResourceContinuityRecoveryV1;
  resource_continuity_profile_digest: Sha256DigestV1;
  resource_id: WorkspaceResourceId;
  schema: "pale/resource-continuity-profile.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  scope: ResourceContinuityScopeV1;
  session_relation: ResourceSessionContinuityV1;
}
export interface ResourceContinuityAccessV1 {
  authorized_controllers: ResourceContinuityControllerSetV1;
  authorized_principals: ResourceContinuityPrincipalSetV1;
  grants: ResourceContinuityGrantSetV1;
}
export interface ResourceContinuityGrantV1 {
  authority: ContinuityRecordCoordinateV1;
  credential: ContinuityOptionalContractV1;
  role: ResourceContinuityAccessRoleV1;
  subject: ResourceContinuitySubjectV1;
  transport: ContinuityOptionalContractV1;
}
export interface ContinuityRecordCoordinateV1 {
  record_ref: ResidentRecordRefV1;
  record_sha256: Sha256DigestV1;
}
export interface ResourceContinuityAuthorityV1 {
  authoritative_source: ContinuityAuthorityBindingV1;
  authority_model: ContinuityRecordCoordinateV1;
  controller: ContinuityControllerBindingV1;
  effect_authority: ContinuityAuthorityBindingV1;
  native_owner: OwnerSystemIdV1;
}
export interface ContinuityAuthorityBindingV1 {
  authority: AuthorityEvidenceV1;
  owner_system_id: OwnerSystemIdV1;
}
export interface AuthorityEvidenceV1 {
  authority_ref: ResidentRecordRefV1;
  authority_sha256: Sha256DigestV1;
}
export interface ContinuityControllerBindingV1 {
  authority: ContinuityRecordCoordinateV1;
  controller_id: ResidentControllerIdV1;
}
export interface ResourceContinuityWriterV1 {
  authority: ContinuityRecordCoordinateV1;
  subject: ResourceContinuitySubjectV1;
}
export interface ResourceContinuityCopyV1 {
  class: ResourceContinuityCopyClassV1;
  copy_id: ResourceContinuityCopyIdV1;
  custodian: ContinuityAuthorityBindingV1;
  holder: ContinuityHolderV1;
  policy: ResourceContinuityCopyPolicyV1;
}
export interface ResourceContinuityCopyPolicyV1 {
  frontier_contract: ContinuityRecordCoordinateV1;
  retention_contract: ContinuityRecordCoordinateV1;
  revocation_contract: ContinuityRecordCoordinateV1;
  scope: ContinuityRecordCoordinateV1;
}
export interface ResourceContinuityCutoverV1 {
  cutover_state: ContinuityRecordCoordinateV1;
  rollback_contract: ContinuityRecordCoordinateV1;
}
export interface ResourceContinuityEvidenceV1 {
  evidence_destination: ContinuityRecordCoordinateV1;
  evidence_owner: ContinuityAuthorityBindingV1;
  read_back_contract: ContinuityRecordCoordinateV1;
  read_back_owner: ContinuityAuthorityBindingV1;
}
export interface ResourceContinuityGovernanceV1 {
  deletion_policy: ContinuityRecordCoordinateV1;
  legal_hold_policy: ContinuityRecordCoordinateV1;
  privacy_policy: ContinuityRecordCoordinateV1;
  residency_policy: ContinuityRecordCoordinateV1;
  retention_policy: ContinuityRecordCoordinateV1;
  revocation_policy: ContinuityRecordCoordinateV1;
}
export interface ResourceContinuityMechanismPolicyV1 {
  disposition: ResourceContinuityMechanismDispositionV1;
  kind: ResourceContinuityMechanismKindV1;
}
export interface ResourceContinuityRecoveryV1 {
  recovery_point_objective_seconds: RecoveryPointObjectiveSecondsV1;
  recovery_time_objective_seconds: RecoveryTimeObjectiveSecondsV1;
  verification_contract: ContinuityRecordCoordinateV1;
}
export interface ResourceContinuityScopeV1 {
  bootstrap_verification: ContinuityRecordCoordinateV1;
  cross_machine_bootstrap: ContinuityRecordCoordinateV1;
  devices: ResourceContinuityDeviceScopeSetV1;
  machines: ResourceContinuityMachineScopeSetV1;
}
