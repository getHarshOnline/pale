/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type SchemaId = string;
export type ReconciliationCustodyGenerationV1 = number;
export type ReconciliationCustodyIdV1 = string;
export type ReconciliationCustodyStateV1 = "active" | "transferred" | "closed";
export type ResidentNodeIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type CanonicalSignatureV1 = string;
export type UnixTimeV1 = number;

export interface ReconciliationCustodyV1 {
  canonical_custody_sha256: Sha256DigestV1;
  case_ref: ResidentRecordRefV1;
  case_sha256: Sha256DigestV1;
  conflict_resource_key: any;
  conflict_resource_key_schema_id: SchemaId;
  conflict_resource_key_schema_sha256: Sha256DigestV1;
  custody_generation: ReconciliationCustodyGenerationV1;
  custody_id: ReconciliationCustodyIdV1;
  custody_state: ReconciliationCustodyStateV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
  holder_node_id?: ResidentNodeIdV1;
  holder_principal_id: ResidentPrincipalIdV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  previous_custody_ref?: ResidentRecordRefV1;
  previous_custody_sha256?: Sha256DigestV1;
  schema_id: "pale/reconciliation-custody.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  terminal_result_ref?: ResidentRecordRefV1;
  terminal_result_sha256?: Sha256DigestV1;
  transferred_or_closed_at?: UnixTimeV1;
}
