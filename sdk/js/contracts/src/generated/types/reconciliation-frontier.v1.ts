/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type ReconciliationDimensionV1 =
  | "owner_publication"
  | "pale_ingress"
  | "delivery"
  | "deterministic_projection"
  | "semantic_mapping"
  | "judgment"
  | "articulation"
  | "native_consequence"
  | "retention";
export type ReconciliationDimensionStatusV1 =
  | {
      state: "complete";
    }
  | {
      state: "pending";
    }
  | {
      state: "gap";
    }
  | {
      state: "blocked";
    }
  | {
      state: "quarantined";
    }
  | {
      state: "unavailable";
    }
  | {
      state: "expired";
    }
  | {
      disposition: OwnerFrontierDispositionV1;
      state: "owner_terminal";
    };
export type OwnerFrontierDispositionV1 = string;
export type ReconciliationFrontierIdV1 = string;
export type UnixTimeV1 = number;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type NativeRevisionOwnerV1 = string;
export type NativeRevisionTokenV1 = string;
export type OwnerSystemIdV1 = string;
export type CanonicalSignatureV1 = string;
export type ReconciliationSubjectV1 =
  | {
      coordinate: OwnerRecordCoordinateV1;
      kind: "source";
    }
  | {
      consumer_id: OwnerSystemIdV1;
      kind: "consumer";
      state_ref: ResidentRecordRefV1;
      state_sha256: Sha256DigestV1;
    }
  | {
      kind: "projection";
      projection_ref: ResidentRecordRefV1;
      projection_sha256: Sha256DigestV1;
    }
  | {
      consequence_ref: ResidentRecordRefV1;
      consequence_sha256: Sha256DigestV1;
      kind: "consequence";
      owner_system_id: OwnerSystemIdV1;
    };
export type SchemaId = string;

export interface ReconciliationFrontierV1 {
  canonical_reconciliation_frontier_sha256: Sha256DigestV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  dimensions: ReconciliationDimensionEntryV1[];
  frontier_id: ReconciliationFrontierIdV1;
  generated_at: UnixTimeV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  owner_record_revisions: FrontierOwnerRevisionV1[];
  schema_id: "pale/reconciliation-frontier.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  source_occurrence_refs: ImmutableRecordReferenceV1[];
  subject: ReconciliationSubjectV1;
}
export interface ReconciliationDimensionEntryV1 {
  dimension: ReconciliationDimensionV1;
  evidence?: AuthorityEvidenceV1 | null;
  status: ReconciliationDimensionStatusV1;
}
export interface AuthorityEvidenceV1 {
  authority_ref: ResidentRecordRefV1;
  authority_sha256: Sha256DigestV1;
}
export interface FrontierOwnerRevisionV1 {
  native_revision: NativeRevisionV1;
  owner_system_id: OwnerSystemIdV1;
  record_ref: ResidentRecordRefV1;
  record_sha256: Sha256DigestV1;
}
export interface NativeRevisionV1 {
  native_revision_owner: NativeRevisionOwnerV1;
  native_revision_token: NativeRevisionTokenV1;
}
export interface ImmutableRecordReferenceV1 {
  record_ref: ResidentRecordRefV1;
  record_sha256: Sha256DigestV1;
}
export interface OwnerRecordCoordinateV1 {
  native_revision: NativeRevisionV1;
  owner_system_id: OwnerSystemIdV1;
  record_content_sha256: Sha256DigestV1;
  record_ref: ResidentRecordRefV1;
  record_schema_id: SchemaId;
  record_schema_sha256: Sha256DigestV1;
}
