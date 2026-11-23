/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type LocalAdminAuthorityIdV1 = string;
export type LocalAdminOperationV1 =
  | "rotate_principal_key"
  | "recover_principal_key"
  | "update_principal_recovery_policy"
  | "rotate_admin_authority"
  | "rotate_producer_incarnation_authority"
  | "issue_node_delegation"
  | "revoke_node_delegation"
  | "rotate_checkpoint_signer"
  | "rotate_deployment_runtime_authority"
  | "issue_owner_record_signing_authority"
  | "rotate_owner_record_signing_authority"
  | "retire_owner_record_signing_authority"
  | "allocate_producer_incarnation"
  | "retire_producer_incarnation"
  | "create_local_route"
  | "advance_local_route"
  | "propose_deployment"
  | "accept_deployment"
  | "revoke_deployment_acceptance"
  | "finalize_deployment"
  | "issue_session_grant"
  | "revoke_session_grant"
  | "advance_execution_fence"
  | "issue_apply_authorization"
  | "create_route_binding"
  | "advance_route_binding";
export type LocalAdminAuthorityEpochV1 = number;
export type Sha256DigestV1 = string;
export type UnixTimeV1 = number;
export type PrincipalKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type ResidentRecordRefV1 = string;
export type CanonicalSignatureV1 = string;
export type PrincipalAuthorityStateV1 = "active" | "retired";
export type LocalAdminSubjectIdentityV1 = string;
export type SigningKeyEpochV1 = number;
export type Ed25519PublicKeyV1 = string;

export interface LocalAdminAuthorityV1 {
  admin_authority_id: LocalAdminAuthorityIdV1;
  allowed_operations: LocalAdminOperationV1[];
  authority_epoch: LocalAdminAuthorityEpochV1;
  canonical_admin_authority_sha256: Sha256DigestV1;
  expires_at: UnixTimeV1;
  issued_at: UnixTimeV1;
  issuer_key_epoch: PrincipalKeyEpochV1;
  issuer_key_id: ResidentKeyIdV1;
  issuer_principal_id: ResidentPrincipalIdV1;
  previous_authority_ref?: ResidentRecordRefV1;
  previous_authority_sha256?: Sha256DigestV1;
  principal_id: ResidentPrincipalIdV1;
  principal_record_ref: ResidentRecordRefV1;
  principal_record_sha256: Sha256DigestV1;
  recovery_policy_ref: ResidentRecordRefV1;
  recovery_policy_sha256: Sha256DigestV1;
  schema_id: "pale/local-admin-authority.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  state: PrincipalAuthorityStateV1;
  subject_identity: LocalAdminSubjectIdentityV1;
  subject_key_epoch: SigningKeyEpochV1;
  subject_key_id: ResidentKeyIdV1;
  subject_public_key: Ed25519PublicKeyV1;
}
