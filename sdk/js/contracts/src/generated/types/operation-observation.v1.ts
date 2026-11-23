/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type UnixTimeV1 = number;
export type OperationHandleIdV1 = string;
export type OperationObservationRevisionV1 = number;
export type CanonicalSignatureV1 = string;
export type OperationObservationStateV1 =
  | {
      phase: OperationActivePhaseV1;
      state: "active";
    }
  | {
      request: ImmutableRecordReferenceV1;
      state: "cancellation_requested";
    }
  | {
      reconciliation: ReconciliationFrontierReferenceV1;
      state: "recovering";
    }
  | {
      effect_receipt?: EffectReceiptReferenceV1 | null;
      result: CapabilityResultReferenceV1;
      state: "succeeded";
    }
  | {
      diagnostic: ImmutableRecordReferenceV1;
      result?: CapabilityResultReferenceV1 | null;
      state: "failed";
    }
  | {
      diagnostic: ImmutableRecordReferenceV1;
      state: "cancelled";
    }
  | {
      diagnostic: ImmutableRecordReferenceV1;
      effect_receipt?: EffectReceiptReferenceV1 | null;
      reconciliation: ReconciliationFrontierReferenceV1;
      state: "outcome_unknown";
    }
  | {
      diagnostic: ImmutableRecordReferenceV1;
      state: "expired";
    };
export type OperationActivePhaseV1 =
  | {
      phase: "resolving";
    }
  | {
      evidence: ImmutableRecordReferenceV1;
      phase: "context_bound";
    }
  | {
      phase: "provider_resolved";
      provider_binding: ProviderBindingRefV1;
    }
  | {
      attempt_id: CapabilityAttemptIdV1;
      evidence: ImmutableRecordReferenceV1;
      phase: "dispatch_prepared";
    }
  | {
      attempt_id: CapabilityAttemptIdV1;
      evidence: ImmutableRecordReferenceV1;
      phase: "dispatched";
    }
  | {
      evidence: ImmutableRecordReferenceV1;
      phase: "collecting_consultation";
    }
  | {
      evidence: ImmutableRecordReferenceV1;
      phase: "consultation_collected";
    };
export type CapabilityAttemptIdV1 = string;

export interface OperationObservationV1 {
  canonical_operation_observation_sha256: Sha256DigestV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  observed_at: UnixTimeV1;
  operation_handle_id: OperationHandleIdV1;
  operation_handle_ref: ResidentRecordRefV1;
  operation_handle_sha256: Sha256DigestV1;
  predecessor_ref?: ResidentRecordRefV1;
  predecessor_sha256?: Sha256DigestV1;
  revision: OperationObservationRevisionV1;
  schema_id: "pale/operation-observation.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  state: OperationObservationStateV1;
}
export interface ImmutableRecordReferenceV1 {
  record_ref: ResidentRecordRefV1;
  record_sha256: Sha256DigestV1;
}
export interface ProviderBindingRefV1 {
  provider_binding_id: string;
  provider_binding_identity: ProviderBindingIdentityV1;
}
export interface ProviderBindingIdentityV1 {
  provider_binding_sha256: Sha256DigestV1;
}
export interface ReconciliationFrontierReferenceV1 {
  record_ref: ResidentRecordRefV1;
  record_sha256: Sha256DigestV1;
}
export interface EffectReceiptReferenceV1 {
  record_ref: ResidentRecordRefV1;
  record_sha256: Sha256DigestV1;
}
export interface CapabilityResultReferenceV1 {
  record_ref: ResidentRecordRefV1;
  record_sha256: Sha256DigestV1;
}
