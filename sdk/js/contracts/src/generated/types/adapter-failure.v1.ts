/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type AdapterFailureV1 = {
  [k: string]: any;
} & {
  absence_evidence_ref?: ResidentRecordRefV1;
  absence_evidence_sha256?: Sha256DigestV1;
  attempt_id: CapabilityAttemptIdV1;
  bounded_message?: BoundedResidentTextV1;
  canonical_failure_sha256: Sha256DigestV1;
  effect_state: AdapterFailureEffectStateV1;
  failure_class: AdapterFailureClassV1;
  failure_id: AdapterFailureIdV1;
  implementation_kind: BoundedResidentTextV1;
  implementation_version: BoundedResidentTextV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  provider_status?: BoundedResidentTextV1;
  recorded_at: UnixTimeV1;
  request_binding: RequestBindingV1;
  retry_at?: UnixTimeV1;
  schema_id: "pale/adapter-failure.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  signing_authority_binding: SigningAuthorityBindingV1;
  stable_code: ResidentReasonV1;
} & (
    | {
        effect_state?: "not_started";
        [k: string]: any;
      }
    | {
        effect_state?: "independently_proven_not_applied";
        [k: string]: any;
      }
  );
export type ResidentRecordRefV1 = string;
export type Sha256DigestV1 = string;
export type CapabilityAttemptIdV1 = string;
export type BoundedResidentTextV1 = string;
export type AdapterFailureEffectStateV1 = "not_started" | "independently_proven_not_applied";
export type AdapterFailureClassV1 =
  | "denied"
  | "invalid"
  | "unsupported_schema"
  | "conflict"
  | "unavailable"
  | "rate_limited"
  | "timeout"
  | "unknown"
  | "partial"
  | "internal";
export type AdapterFailureIdV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type UnixTimeV1 = number;
export type CapabilityIdV1 = string;
export type ResidentCorrelationIdV1 = string;
export type ResidentRequestIdV1 = string;
export type SchemaId = string;
export type RouteGenerationV1 = number;
export type SessionGrantIdV1 = string;
export type CanonicalSignatureV1 = string;
export type SigningAuthorityBindingV1 = SigningAuthorityBindingModeConstraintV1 & {
  authority_mode: SigningAuthorityModeV1;
  integrated_owner?: IntegratedOwnerAuthorityV1;
  integrated_runtime?: IntegratedRuntimeAuthorityV1;
  principal_id: ResidentPrincipalIdV1;
  signer_key_epoch: SigningKeyEpochV1;
  signer_key_id: ResidentKeyIdV1;
  standalone_owner_native?: StandaloneOwnerNativeAuthorityV1;
};
export type SigningAuthorityBindingModeConstraintV1 =
  | {
      authority_mode?: "integrated_runtime";
      [k: string]: any;
    }
  | {
      authority_mode?: "integrated_owner";
      [k: string]: any;
    }
  | {
      authority_mode?: "standalone_owner_native";
      [k: string]: any;
    };
export type SigningAuthorityModeV1 = "integrated_runtime" | "integrated_owner" | "standalone_owner_native";
export type OwnerComponentIdV1 = string;
export type OwnerSignerRecordSequenceV1 = number;
export type ResidentReasonV1 = string;

export interface RequestBindingV1 {
  canonical_request_sha256: Sha256DigestV1;
  capability_id: CapabilityIdV1;
  contract_set_ref: ResidentRecordRefV1;
  contract_set_sha256: Sha256DigestV1;
  correlation_id: ResidentCorrelationIdV1;
  deployment_binding_ref: ResidentRecordRefV1;
  deployment_binding_sha256: Sha256DigestV1;
  execution_owner_fence_ref?: ResidentRecordRefV1;
  execution_owner_fence_sha256?: Sha256DigestV1;
  federation_request_admission_ref?: ResidentRecordRefV1;
  federation_request_admission_sha256?: Sha256DigestV1;
  request_id: ResidentRequestIdV1;
  route_binding_ref: ResidentRecordRefV1;
  route_binding_schema_id: SchemaId;
  route_binding_schema_sha256: Sha256DigestV1;
  route_binding_sha256: Sha256DigestV1;
  route_generation: RouteGenerationV1;
  session_grant_id: SessionGrantIdV1;
  session_grant_sha256: Sha256DigestV1;
}
export interface IntegratedOwnerAuthorityV1 {
  checked_principal_trust_checkpoint_ref: ResidentRecordRefV1;
  checked_principal_trust_checkpoint_sha256: Sha256DigestV1;
  owner_component_id: OwnerComponentIdV1;
  owner_contract_manifest_ref: ResidentRecordRefV1;
  owner_contract_manifest_sha256: Sha256DigestV1;
  owner_record_signing_authority_ref: ResidentRecordRefV1;
  owner_record_signing_authority_sha256: Sha256DigestV1;
  owner_signer_record_sequence: OwnerSignerRecordSequenceV1;
  previous_owner_record_sha256?: Sha256DigestV1;
}
export interface IntegratedRuntimeAuthorityV1 {
  checked_principal_trust_checkpoint_ref: ResidentRecordRefV1;
  checked_principal_trust_checkpoint_sha256: Sha256DigestV1;
  deployment_binding_ref: ResidentRecordRefV1;
  deployment_binding_sha256: Sha256DigestV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
}
export interface StandaloneOwnerNativeAuthorityV1 {
  checked_owner_revocation_frontier_ref: ResidentRecordRefV1;
  checked_owner_revocation_frontier_sha256: Sha256DigestV1;
  owner_authority_ref: ResidentRecordRefV1;
  owner_authority_schema_id: SchemaId;
  owner_authority_schema_sha256: Sha256DigestV1;
  owner_authority_sha256: Sha256DigestV1;
  owner_component_id: OwnerComponentIdV1;
  owner_contract_manifest_ref: ResidentRecordRefV1;
  owner_contract_manifest_sha256: Sha256DigestV1;
}
