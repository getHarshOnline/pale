/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type LeaseBudgetUnitsV1 = number;
export type CapabilityIdV1 = string;
export type EffectClass = "compute" | "read" | "execute" | "write" | "network" | "publish";
export type ResidentRecordRefV1 = string;
export type UnixTimeV1 = number;
export type InvocationCorrelationV1 =
  | {
      invocation_correlation_id: InvocationCorrelationIdV1;
      native_invocation_id: NativeInvocationIdV1;
      visibility: "observed";
    }
  | {
      reason: ResidentReasonV1;
      visibility: "unobserved";
    };
export type InvocationCorrelationIdV1 = string;
export type NativeInvocationIdV1 = string;
export type ResidentReasonV1 = string;
export type PackageIdV1 = string;
export type PackageResourceIdV1 = string;
export type Sha256DigestV1 = string;
export type OperationLeaseIdV1 = string;
export type SessionAdmissionIdV1 = string;
export type LeaseTimeoutMillisV1 = number;

export interface OperationLeaseV1 {
  budget_units: LeaseBudgetUnitsV1;
  capabilities: CapabilityIdV1[];
  effects: EffectClass[];
  evidence_obligations: ResidentRecordRefV1[];
  expires_at: UnixTimeV1;
  invocation_correlation: InvocationCorrelationV1;
  operation: PackageResourceRefV1;
  operation_lease_digest: Sha256DigestV1;
  operation_lease_id: OperationLeaseIdV1;
  read_set: ResidentRecordRefV1[];
  schema: "pale/operation-lease.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  session_admission_id: SessionAdmissionIdV1;
  session_admission_lease_digest: Sha256DigestV1;
  timeout_millis: LeaseTimeoutMillisV1;
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
