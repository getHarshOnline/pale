/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type ContractSetIdV1 = string;
export type SchemaId = string;

export interface ContractSetV1 {
  canonical_contract_set_sha256: Sha256DigestV1;
  compatibility_profile_ref: ResidentRecordRefV1;
  compatibility_profile_sha256: Sha256DigestV1;
  contract_set_id: ContractSetIdV1;
  /**
   * @maxItems 1024
   */
  entries: ContractSetEntryV1[];
  schema_id: "pale/contract-set.v1";
  schema_sha256: Sha256DigestV1;
}
export interface ContractSetEntryV1 {
  contract_id: SchemaId;
  contract_schema_ref: ResidentRecordRefV1;
  contract_schema_sha256: Sha256DigestV1;
}
