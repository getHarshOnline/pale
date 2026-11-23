/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type UnixTimeV1 = number;
export type Sha256DigestV1 = string;
export type ProducerIncarnationIdV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type ResidentRecordRefV1 = string;
export type ProducerEpochV1 = string;
export type ProducerIdV1 = string;
export type CanonicalSignatureV1 = string;

export interface ProducerIncarnationV1 {
  allocated_at: UnixTimeV1;
  canonical_incarnation_sha256: Sha256DigestV1;
  incarnation_id: ProducerIncarnationIdV1;
  issuer_key_epoch: SigningKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  origin_principal_id: ResidentPrincipalIdV1;
  previous_incarnation_ref?: ResidentRecordRefV1;
  previous_incarnation_sha256?: Sha256DigestV1;
  producer_epoch: ProducerEpochV1;
  producer_id: ProducerIdV1;
  producer_implementation_ref: ResidentRecordRefV1;
  producer_implementation_sha256: Sha256DigestV1;
  producer_incarnation_authority_ref: ResidentRecordRefV1;
  producer_incarnation_authority_sha256: Sha256DigestV1;
  schema_id: "pale/producer-incarnation.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
}
