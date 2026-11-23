/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ActionIdV1 = string;
export type Sha256DigestV1 = string;
export type ReconciliationCaseIdV1 = string;
export type ReconciliationCaseOriginV1 = "initial" | "reopened";
export type ReconciliationCaseRevisionV1 = number;
export type SchemaId = string;
export type ResidentRecordRefV1 = string;
export type TypedDigestCanonicalizationV1 = "rfc8785_json" | "raw_bytes";
export type TypedDigestContentBindingV1 =
  | {
      inline_json: any;
      kind: "inline_json";
    }
  | {
      inline_bytes_base64: string;
      kind: "inline_bytes_base64";
    }
  | {
      content_ref: ResidentRecordRefV1;
      content_ref_sha256: Sha256DigestV1;
      kind: "immutable_content";
    };
export type TypedDigestAlgorithmV1 = "sha256";
export type BoundedResidentTextV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type UnixTimeV1 = number;
export type ReconciliationOpeningEvidenceV1 =
  | {
      evidence_ref: ResidentRecordRefV1;
      evidence_schema_id: SchemaId;
      evidence_schema_sha256: Sha256DigestV1;
      evidence_sha256: Sha256DigestV1;
      kind: "effect_receipt";
    }
  | {
      evidence_ref: ResidentRecordRefV1;
      evidence_schema_id: SchemaId;
      evidence_schema_sha256: Sha256DigestV1;
      evidence_sha256: Sha256DigestV1;
      kind: "observation";
      observation_kind: ObservationKindV1;
    }
  | {
      evidence_ref: ResidentRecordRefV1;
      evidence_schema_id: SchemaId;
      evidence_schema_sha256: Sha256DigestV1;
      evidence_sha256: Sha256DigestV1;
      kind: "reviewer_evidence";
      reviewer_or_policy_ref: ResidentRecordRefV1;
      reviewer_or_policy_sha256: Sha256DigestV1;
    };
export type ObservationKindV1 = string;
export type ResidentReasonV1 = string;
export type ReconciliationCustodyIdV1 = string;
export type CanonicalSignatureV1 = string;

export interface ReconciliationCaseV1 {
  action_id: ActionIdV1;
  canonical_case_sha256: Sha256DigestV1;
  case_id: ReconciliationCaseIdV1;
  case_origin: ReconciliationCaseOriginV1;
  case_revision: ReconciliationCaseRevisionV1;
  conflict_resource_key: any;
  conflict_resource_key_schema_id: SchemaId;
  conflict_resource_key_schema_sha256: Sha256DigestV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  effect_admission_ref: ResidentRecordRefV1;
  effect_admission_sha256: Sha256DigestV1;
  expected_state_digest: TypedDigestV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  opened_at: UnixTimeV1;
  opening_evidence: ReconciliationOpeningEvidenceV1[];
  predecessor_case_ref?: ResidentRecordRefV1;
  predecessor_case_sha256?: Sha256DigestV1;
  predecessor_terminal_result_ref?: ResidentRecordRefV1;
  predecessor_terminal_result_sha256?: Sha256DigestV1;
  reconciliation_policy_ref: ResidentRecordRefV1;
  reconciliation_policy_sha256: Sha256DigestV1;
  reopen_authority_ref?: ResidentRecordRefV1;
  reopen_authority_sha256?: Sha256DigestV1;
  reopen_policy_ref?: ResidentRecordRefV1;
  reopen_policy_sha256?: Sha256DigestV1;
  reopen_reason_code?: ResidentReasonV1;
  reserved_custody_id: ReconciliationCustodyIdV1;
  schema_id: "pale/reconciliation-case.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
}
export interface TypedDigestV1 {
  canonicalization: TypedDigestCanonicalizationV1;
  content_binding: TypedDigestContentBindingV1;
  content_schema_id: SchemaId;
  content_schema_sha256: Sha256DigestV1;
  digest: Sha256DigestV1;
  digest_algorithm: TypedDigestAlgorithmV1;
  kind: BoundedResidentTextV1;
}
