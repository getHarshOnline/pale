/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type DeploymentRuntimeAuthorityEpochV1 = number;
export type Sha256DigestV1 = string;
export type ControlIncarnationIdV1 = string;
export type ControlStoreIdV1 = string;
export type ResidentRecordRefV1 = string;
export type UnixTimeV1 = number;
export type PrincipalKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type SchemaId = string;
export type DeploymentRuntimeAuthorityIdV1 = string;
export type CanonicalSignatureV1 = string;
export type PrincipalAuthorityStateV1 = "active" | "retired";
export type SigningKeyEpochV1 = number;
export type Ed25519PublicKeyV1 = string;

export interface DeploymentRuntimeAuthorityV1 {
  authority_epoch: DeploymentRuntimeAuthorityEpochV1;
  canonical_deployment_runtime_authority_sha256: Sha256DigestV1;
  control_incarnation_id: ControlIncarnationIdV1;
  control_store_id: ControlStoreIdV1;
  deployment_binding_ref: ResidentRecordRefV1;
  deployment_binding_sha256: Sha256DigestV1;
  expires_at: UnixTimeV1;
  issued_at: UnixTimeV1;
  issuer_key_epoch: PrincipalKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  /**
   * @minItems 1
   * @maxItems 1024
   */
  permitted_record_schema_ids: [SchemaId, ...SchemaId[]];
  previous_authority_ref?: ResidentRecordRefV1;
  previous_authority_sha256?: Sha256DigestV1;
  principal_id: ResidentPrincipalIdV1;
  runtime_authority_id: DeploymentRuntimeAuthorityIdV1;
  schema_id: "pale/deployment-runtime-authority.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  state: PrincipalAuthorityStateV1;
  subject_key_epoch: SigningKeyEpochV1;
  subject_key_id: ResidentKeyIdV1;
  subject_public_key: Ed25519PublicKeyV1;
}
