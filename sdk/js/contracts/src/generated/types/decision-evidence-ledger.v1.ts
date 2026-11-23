/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type DecisionActivityV1 =
  | {
      activity: HarnessActivityIdentity;
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
export type Sha256DigestV1 = string;
export type ResidentReasonV1 = string;
export type DecisionDispositionV1 =
  "deny" | "request_confirmation" | "post_effect_steer" | "completion_steer" | "notification";
export type LifecycleEventV1 =
  | "session-start"
  | "user-prompt-submit"
  | "user-prompt-expansion"
  | "permission-request"
  | "pre-tool-use"
  | "post-tool-use"
  | "post-tool-use-failure"
  | "subagent-start"
  | "subagent-stop"
  | "pre-compact"
  | "post-compact"
  | "stop"
  | "session-end";
export type DecisionNativeCoordinateV1 =
  | {
      sha256: Sha256DigestV1;
      visibility: "observed";
    }
  | {
      reason: ResidentReasonV1;
      visibility: "unobserved";
    }
  | {
      visibility: "not_applicable";
    };
export type DecisionOperationBindingV1 =
  | {
      handle_ref: ResidentRecordRefV1;
      handle_sha256: Sha256DigestV1;
      kind: "registered";
      lease_ref: ResidentRecordRefV1;
      lease_sha256: Sha256DigestV1;
    }
  | {
      kind: "unbound";
    }
  | {
      kind: "unresolved";
      reason: ResidentReasonV1;
    };
export type ResidentRecordRefV1 = string;
export type DecisionEvidenceStageV1 =
  | {
      kind: "policy_selected";
    }
  | {
      kind: "response_emission";
      outcome: DecisionEmissionOutcomeV1;
      response_sha256: Sha256DigestV1;
      selected_entry_sha256: Sha256DigestV1;
    }
  | {
      emission_entry_sha256: Sha256DigestV1;
      kind: "host_acknowledged";
      outcome: DecisionHostOutcomeV1;
      witness: ImmutableRecordReferenceV1;
    };
export type DecisionEmissionOutcomeV1 = "emitted" | "failed" | "unknown";
export type DecisionHostOutcomeV1 = "prevented" | "confirmation_pending" | "steered" | "notified";
export type DecisionTargetV1 =
  | {
      kind: "file";
      path: string;
      repository_id: LogicalRepositoryIdV1;
      sha256: Sha256DigestV1;
    }
  | {
      kind: "command";
      sha256: Sha256DigestV1;
    }
  | {
      kind: "structured_input";
      sha256: Sha256DigestV1;
    }
  | {
      kind: "unobserved";
      reason: ResidentReasonV1;
    }
  | {
      kind: "not_applicable";
    };
export type LogicalRepositoryIdV1 = string;
export type ExecutionPathSelectionIdV1 = string;

export interface DecisionEvidenceLedgerV1 {
  activity: DecisionActivityV1;
  compilation_input_digest: Sha256DigestV1;
  /**
   * @maxItems 64
   */
  entries: DecisionEvidenceEntryV1[];
  execution_path: ExecutionPathSelectionV1;
  next_sequence: number;
  profile_digest: Sha256DigestV1;
  repository_id: LogicalRepositoryIdV1;
  schema: "pale/decision-evidence-ledger.v1";
  schema_sha256: Sha256DigestV1;
  truncation_anchor?: DecisionEvidenceTruncationAnchorV1 | null;
}
export interface DecisionEvidenceEntryV1 {
  disposition: DecisionDispositionV1;
  event: LifecycleEventV1;
  evidence_digest: Sha256DigestV1;
  occurrence: DecisionOccurrenceV1;
  operation: DecisionOperationBindingV1;
  previous_digest?: Sha256DigestV1 | null;
  reason: string;
  reason_sha256: Sha256DigestV1;
  sequence: number;
  stage: DecisionEvidenceStageV1;
  target: DecisionTargetV1;
  tool?: string | null;
}
export interface DecisionOccurrenceV1 {
  native_event_sha256: Sha256DigestV1;
  provider_binding: ProviderBindingRefV1;
  tool_use: DecisionNativeCoordinateV1;
  turn: DecisionNativeCoordinateV1;
}
export interface ProviderBindingRefV1 {
  provider_binding_id: string;
  provider_binding_identity: ProviderBindingIdentityV1;
}
export interface ProviderBindingIdentityV1 {
  provider_binding_sha256: Sha256DigestV1;
}
export interface ImmutableRecordReferenceV1 {
  record_ref: ResidentRecordRefV1;
  record_sha256: Sha256DigestV1;
}
export interface ExecutionPathSelectionV1 {
  experience_path_id: string;
  id: ExecutionPathSelectionIdV1;
}
export interface DecisionEvidenceTruncationAnchorV1 {
  evidence_digest: Sha256DigestV1;
  sequence: number;
}
