/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ProducerIncarnationAuthorityEpochV1 = number;
export type Sha256DigestV1 = string;
export type UnixTimeV1 = number;
export type PrincipalKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type ProducerNamespaceV1 = string;
export type ResidentRecordRefV1 = string;
export type ProducerIncarnationAuthorityIdV1 = string;
export type CanonicalSignatureV1 = string;
export type PrincipalAuthorityStateV1 = "active" | "retired";
export type SigningKeyEpochV1 = number;
export type Ed25519PublicKeyV1 = string;

export interface ProducerIncarnationAuthorityV1 {
  authority_epoch: ProducerIncarnationAuthorityEpochV1;
  canonical_producer_authority_sha256: Sha256DigestV1;
  expires_at: UnixTimeV1;
  issued_at: UnixTimeV1;
  issuer_key_epoch: PrincipalKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  permitted_producer_namespaces: ProducerNamespaceV1[];
  predecessor_final_publication_frontier_ref?: ResidentRecordRefV1;
  predecessor_final_publication_frontier_sha256?: Sha256DigestV1;
  previous_authority_ref?: ResidentRecordRefV1;
  previous_authority_sha256?: Sha256DigestV1;
  principal_id: ResidentPrincipalIdV1;
  producer_authority_id: ProducerIncarnationAuthorityIdV1;
  schema_id: "pale/producer-incarnation-authority.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  state: PrincipalAuthorityStateV1;
  subject_key_epoch: SigningKeyEpochV1;
  subject_key_id: ResidentKeyIdV1;
  subject_public_key: Ed25519PublicKeyV1;
}
