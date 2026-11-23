/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type PackageIdV1 = string;
export type PackageResourceIdV1 = string;
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)*$".
 */
export type Sha256DigestV1 = string;
export type FailurePosture = "fail_closed" | "fail_open" | "retry" | "steer" | "escalate";
export type CapabilityRequirementIdV1 = string;
export type InflightAdmissionStatusV1 = "evidence_pending" | "admitted";
export type VerificationRequirementV1 = "standard" | "protected_configuration";

export interface SessionRuntimeStateV1 {
  active_modes?: {
    [k: string]: PackageResourceRefV1;
  };
  active_operation?: ActiveOperationLeaseV1 | null;
  /**
   * @maxItems 128
   */
  captures?: string[];
  operation_epoch?: number;
  rebaseline_debt?: RebaselineDebtV1 | null;
  rebaseline_epoch?: number;
  schema: "pale.session-runtime-state.v1";
  schema_sha256: Sha256DigestV1;
  /**
   * @maxItems 1024
   */
  touched?: string[];
  verification_debt?: VerificationDebtV1 | null;
  verification_epoch?: number;
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface ActiveOperationLeaseV1 {
  activation_epoch: number;
  authorized_actions?: {
    [k: string]: Sha256DigestV1;
  };
  binding_digest: Sha256DigestV1;
  catalog_digest: Sha256DigestV1;
  contract_digest: Sha256DigestV1;
  failure?: OperationFailureV1 | null;
  inflight_tools?: {
    [k: string]: InflightToolAdmissionV1;
  };
  operation_id: PackageResourceRefV1;
  turn_digest?: Sha256DigestV1 | null;
}
export interface OperationFailureV1 {
  posture: FailurePosture;
  requirement_id: CapabilityRequirementIdV1;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[0-9a-f]{64}$".
 */
export interface InflightToolAdmissionV1 {
  invocation_digest: Sha256DigestV1;
  requirement_id?: CapabilityRequirementIdV1 | null;
  status: InflightAdmissionStatusV1;
}
export interface RebaselineDebtV1 {
  epoch: number;
}
export interface VerificationDebtV1 {
  completion_prompted?: boolean;
  epoch: number;
  requirement: VerificationRequirementV1;
}
