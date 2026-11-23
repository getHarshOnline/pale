/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type LeaseBudgetUnitsV1 = number;
export type CapabilityIdV1 = string;
export type ResidentRecordRefV1 = string;
export type ResidentControllerIdV1 = string;
export type EffectClass = "compute" | "read" | "execute" | "write" | "network" | "publish";
export type ExecutionPathSelectionIdV1 = string;
export type LogicalExecutionWorkspaceIdV1 = string;
export type UnixTimeV1 = number;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type NativeSessionCorrelationV1 =
  | {
      native_session_identity: HarnessActivityIdentity;
      visibility: "observed";
    }
  | {
      reason: ResidentReasonV1;
      visibility: "unobserved";
    };
export type HarnessActivityIdentity =
  | {
      digest: Sha256DigestV1;
      kind: "session";
      [k: string]: any;
    }
  | {
      digest: Sha256DigestV1;
      kind: "turn";
      [k: string]: any;
    };
export type ResidentReasonV1 = string;
export type LogicalProjectIdV1 = string;
export type RelationshipScopeV1 = string;
export type LogicalRepositoryIdV1 = string;
export type PrincipalRevocationEpochV1 = number;
export type SessionAdmissionIdV1 = string;
export type CanonicalSignatureV1 = string;
export type LeaseTimeoutMillisV1 = number;

export interface SessionAdmissionLeaseV1 {
  activation_fence_digest: Sha256DigestV1;
  active_generation_head_digest: Sha256DigestV1;
  budget_ceiling_units: LeaseBudgetUnitsV1;
  capability_ceiling: CapabilityIdV1[];
  checked_revocation_ref?: ResidentRecordRefV1;
  checked_revocation_sha256?: Sha256DigestV1;
  compiled_generation_digest: Sha256DigestV1;
  controller_id: ResidentControllerIdV1;
  deployment_binding_ref: ResidentRecordRefV1;
  deployment_binding_sha256: Sha256DigestV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  effect_ceiling: EffectClass[];
  evidence_destination: ResidentRecordRefV1;
  evidence_obligations: ResidentRecordRefV1[];
  execution_path: ExecutionPathSelectionV1;
  execution_workspace_id: LogicalExecutionWorkspaceIdV1;
  expires_at: UnixTimeV1;
  harness_adapter_build_sha256: Sha256DigestV1;
  harness_adapter_source_sha256: Sha256DigestV1;
  harness_configuration_sha256: Sha256DigestV1;
  harness_executable_sha256: Sha256DigestV1;
  instance_digest: Sha256DigestV1;
  issued_at: UnixTimeV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  native_session_correlation: NativeSessionCorrelationV1;
  principal_id: ResidentPrincipalIdV1;
  profile_digest: Sha256DigestV1;
  project_id: LogicalProjectIdV1;
  read_ceiling: ResidentRecordRefV1[];
  relationship_scope?: RelationshipScopeV1;
  repository_id: LogicalRepositoryIdV1;
  revocation_epoch: PrincipalRevocationEpochV1;
  schema_id: "pale/session-admission-lease.v1";
  schema_sha256: Sha256DigestV1;
  session_admission_id: SessionAdmissionIdV1;
  session_admission_lease_digest: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  timeout_ceiling_millis: LeaseTimeoutMillisV1;
}
export interface ExecutionPathSelectionV1 {
  experience_path_id: string;
  id: ExecutionPathSelectionIdV1;
}
