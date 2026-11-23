/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type UnixTimeV1 = number;
export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type GenealogyEdgeIdV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type GenealogyProofV1 =
  | {
      native_owner_evidence: OwnerRecordCoordinateV1;
      relation: GenealogyRelationV1;
      truth_class: "native_structural";
    }
  | {
      admission_ref: ResidentRecordRefV1;
      admission_sha256: Sha256DigestV1;
      destination_owner_receipt_ref: ResidentRecordRefV1;
      destination_owner_receipt_sha256: Sha256DigestV1;
      relation: GenealogyRelationV1;
      truth_class: "pale_consequence_lineage";
    }
  | {
      confidence_basis_points: GenealogyConfidenceBasisPointsV1;
      evidence_root_sha256: Sha256DigestV1;
      occupant_ref: ResidentRecordRefV1;
      occupant_sha256: Sha256DigestV1;
      profile_ref: ResidentRecordRefV1;
      profile_sha256: Sha256DigestV1;
      relation: GenealogyRelationV1;
      scope_transaction_ref: ResidentRecordRefV1;
      scope_transaction_sha256: Sha256DigestV1;
      truth_class: "semantic_inferred";
      uncertainty: SemanticUncertaintyV1;
    };
export type NativeRevisionOwnerV1 = string;
export type NativeRevisionTokenV1 = string;
export type OwnerSystemIdV1 = string;
export type SchemaId = string;
export type GenealogyRelationV1 = string;
export type GenealogyConfidenceBasisPointsV1 = number;
export type SemanticUncertaintyV1 = string;
export type CanonicalSignatureV1 = string;

export interface GenealogyEdgeV1 {
  admitted_at: UnixTimeV1;
  canonical_genealogy_edge_sha256: Sha256DigestV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  destination_occurrence_ref: ResidentRecordRefV1;
  destination_occurrence_sha256: Sha256DigestV1;
  edge_id: GenealogyEdgeIdV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  proof: GenealogyProofV1;
  schema_id: "pale/genealogy-edge.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  source_occurrence_ref: ResidentRecordRefV1;
  source_occurrence_sha256: Sha256DigestV1;
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
