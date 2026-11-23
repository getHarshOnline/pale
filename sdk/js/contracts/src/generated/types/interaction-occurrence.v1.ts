/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type NativeRevisionOwnerV1 = string;
export type NativeRevisionTokenV1 = string;
export type OwnerSystemIdV1 = string;
export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type SchemaId = string;
export type OwnerRecordKnowledgeV1 =
  | {
      state: "unknown";
    }
  | {
      state: "absent";
    }
  | {
      coordinate: OwnerRecordCoordinateV1;
      state: "known";
    };
export type DataClassificationV1 = string;
export type NativeContainerGenreV1 = string;
export type InteractionDisclosureCeilingV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type InteractionLifecycleProjectionV1 =
  | {
      state: "unknown";
    }
  | {
      state: "current";
    }
  | {
      evidence: OwnerRecordCoordinateV1;
      state: "edited";
    }
  | {
      evidence: OwnerRecordCoordinateV1;
      state: "deleted";
    }
  | {
      evidence: OwnerRecordCoordinateV1;
      state: "superseded";
    };
export type InteractionOccurrenceIdV1 = string;
export type PayloadResolutionPolicyV1 = string;
export type UnixTimeV1 = number;
export type InteractionResponseExpectationV1 =
  | {
      state: "unknown";
    }
  | {
      state: "none";
    }
  | {
      expectation: ResponseExpectationV1;
      state: "expected";
    };
export type ResponseExpectationV1 = string;
export type CanonicalSignatureV1 = string;
export type InteractionVisibilityV1 = string;

export interface InteractionOccurrenceV1 {
  actor_provenance: OwnerRecordCoordinateV1;
  audience_snapshot: OwnerRecordKnowledgeV1;
  canonical_occurrence_sha256: Sha256DigestV1;
  classification: DataClassificationV1;
  container_genre: NativeContainerGenreV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  disclosure_ceiling: InteractionDisclosureCeilingV1;
  event_reference_ref: ResidentRecordRefV1;
  event_reference_sha256: Sha256DigestV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  lifecycle: InteractionLifecycleProjectionV1;
  mention_and_cc_records: OwnerRecordCoordinateV1[];
  native_container: OwnerRecordCoordinateV1;
  occurrence_id: InteractionOccurrenceIdV1;
  parent_placement: OwnerRecordKnowledgeV1;
  participant_snapshot: OwnerRecordKnowledgeV1;
  payload_resolution_policy: PayloadResolutionPolicyV1;
  projected_at: UnixTimeV1;
  referenced_records: OwnerRecordCoordinateV1[];
  response_expectation: InteractionResponseExpectationV1;
  root_placement: OwnerRecordKnowledgeV1;
  schema_id: "pale/interaction-occurrence.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  thread_placement: OwnerRecordKnowledgeV1;
  visibility: InteractionVisibilityV1;
}
export interface OwnerRecordCoordinateV1 {
  native_revision: NativeRevisionV1;
  owner_system_id: OwnerSystemIdV1;
  record_content_sha256: Sha256DigestV1;
  record_ref: ResidentRecordRefV1;
  record_schema_id: SchemaId;
  record_schema_sha256: Sha256DigestV1;
}
export interface NativeRevisionV1 {
  native_revision_owner: NativeRevisionOwnerV1;
  native_revision_token: NativeRevisionTokenV1;
}
