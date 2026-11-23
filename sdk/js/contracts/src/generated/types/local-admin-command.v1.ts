/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ResidentRecordRefV1 = string;
export type Sha256DigestV1 = string;
export type LocalAdminCommandIdV1 = string;
export type SchemaId = string;
export type LocalAdminHeadEpochV1 = number;
export type LocalAdminHeadSelectorV1 =
  | {
      kind: "principal_record";
      principal_id: ResidentPrincipalIdV1;
    }
  | {
      kind: "principal_recovery_policy";
      principal_id: ResidentPrincipalIdV1;
    }
  | {
      kind: "local_admin_authority";
      principal_id: ResidentPrincipalIdV1;
    }
  | {
      kind: "producer_incarnation_authority";
      principal_id: ResidentPrincipalIdV1;
    }
  | {
      kind: "principal_trust_checkpoint";
      principal_id: ResidentPrincipalIdV1;
      trust_store_id: ControlStoreIdV1;
      trust_store_incarnation_id: ControlIncarnationIdV1;
    }
  | {
      kind: "producer_incarnation";
      origin_principal_id: ResidentPrincipalIdV1;
      producer_id: ProducerIdV1;
    }
  | {
      checkpoint_signing_authority_id: CheckpointSigningAuthorityIdV1;
      control_incarnation_id: ControlIncarnationIdV1;
      control_store_id: ControlStoreIdV1;
      kind: "checkpoint_signing_authority";
      principal_id: ResidentPrincipalIdV1;
      scope: CheckpointSigningScopeV1;
    }
  | {
      control_incarnation_id: ControlIncarnationIdV1;
      control_store_id: ControlStoreIdV1;
      deployment_binding_ref: ResidentRecordRefV1;
      deployment_binding_sha256: Sha256DigestV1;
      kind: "deployment_runtime_authority";
      principal_id: ResidentPrincipalIdV1;
      runtime_authority_id: DeploymentRuntimeAuthorityIdV1;
    }
  | {
      checkpoint_domain_id: ControlCheckpointDomainIdV1;
      control_incarnation_id: ControlIncarnationIdV1;
      control_store_id: ControlStoreIdV1;
      kind: "control_checkpoint";
    }
  | {
      deployment_binding_ref: ResidentRecordRefV1;
      deployment_binding_sha256: Sha256DigestV1;
      kind: "local_route";
      route_id: LocalRouteIdV1;
      route_owner_principal_id: ResidentPrincipalIdV1;
      workspace_id: ResidentWorkspaceIdV1;
    }
  | {
      controller_scope: ResidentControllerIdV1;
      deployment_binding_ref: ResidentRecordRefV1;
      deployment_binding_sha256: Sha256DigestV1;
      execution_fence_id: ExecutionOwnerFenceIdV1;
      execution_path_id: ExecutionPathSelectionIdV1;
      kind: "execution_fence";
      principal_scope: ResidentPrincipalIdV1;
      relationship_scope?: RelationshipScopeV1 | null;
    }
  | {
      deployment_acceptance_ref: ResidentRecordRefV1;
      deployment_acceptance_sha256: Sha256DigestV1;
      kind: "deployment_acceptance_revocation";
    }
  | {
      grant_ref: ResidentRecordRefV1;
      grant_sha256: Sha256DigestV1;
      kind: "capability_grant_revocation";
    }
  | {
      authority_principal_id: ResidentPrincipalIdV1;
      kind: "capability_revocation_checkpoint";
    }
  | {
      deployment_binding_ref: ResidentRecordRefV1;
      deployment_binding_sha256: Sha256DigestV1;
      kind: "owner_record_signing_authority";
      owner_component_id: OwnerComponentIdV1;
      owner_contract_manifest_ref: ResidentRecordRefV1;
      owner_contract_manifest_sha256: Sha256DigestV1;
      owner_record_signing_authority_id: OwnerRecordSigningAuthorityIdV1;
      principal_id: ResidentPrincipalIdV1;
    }
  | {
      delegation_id: NodeDelegationIdV1;
      kind: "node_delegation";
      node_id: ResidentNodeIdV1;
      principal_id: ResidentPrincipalIdV1;
    }
  | {
      kind: "node_revocation";
      node_id: ResidentNodeIdV1;
      principal_id: ResidentPrincipalIdV1;
    }
  | {
      kind: "federation_route";
      route_id: ResidentRouteIdV1;
      route_owner_principal_id: ResidentPrincipalIdV1;
    };
export type ResidentPrincipalIdV1 = string;
export type ControlStoreIdV1 = string;
export type ControlIncarnationIdV1 = string;
export type ProducerIdV1 = string;
export type CheckpointSigningAuthorityIdV1 = string;
export type CheckpointSigningScopeV1 =
  | {
      bootstrap_id: PrincipalBootstrapIdV1;
      kind: "principal_trust_store";
    }
  | {
      deployment_binding_ref: ResidentRecordRefV1;
      deployment_binding_sha256: Sha256DigestV1;
      kind: "deployment";
    };
export type PrincipalBootstrapIdV1 = string;
export type DeploymentRuntimeAuthorityIdV1 = string;
export type ControlCheckpointDomainIdV1 = string;
export type LocalRouteIdV1 = string;
export type ResidentWorkspaceIdV1 = string;
export type ResidentControllerIdV1 = string;
export type ExecutionOwnerFenceIdV1 = string;
export type ExecutionPathSelectionIdV1 = string;
export type RelationshipScopeV1 = string;
export type OwnerComponentIdV1 = string;
export type OwnerRecordSigningAuthorityIdV1 = string;
export type NodeDelegationIdV1 = string;
export type ResidentNodeIdV1 = string;
export type ResidentRouteIdV1 = string;
export type UnixTimeV1 = number;
export type LocalAdminIdempotencyKeyV1 = string;
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
export type CanonicalSignatureV1 = string;
export type LocalAdminSubjectIdentityV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;

export interface LocalAdminCommandV1 {
  admin_authority_ref: ResidentRecordRefV1;
  admin_authority_sha256: Sha256DigestV1;
  admin_command_id: LocalAdminCommandIdV1;
  candidate_records: LocalAdminCandidateRecordV1[];
  candidate_set_sha256: Sha256DigestV1;
  canonical_admin_command_sha256: Sha256DigestV1;
  expected_heads: LocalAdminExpectedHeadV1[];
  expires_at: UnixTimeV1;
  idempotency_key: LocalAdminIdempotencyKeyV1;
  issued_at: UnixTimeV1;
  operation: LocalAdminOperationV1;
  schema_id: "pale/local-admin-command.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  signer_identity: LocalAdminSubjectIdentityV1;
  signer_key_epoch: SigningKeyEpochV1;
  signer_key_id: ResidentKeyIdV1;
}
export interface LocalAdminCandidateRecordV1 {
  candidate_record_ref: ResidentRecordRefV1;
  candidate_record_schema_id: SchemaId;
  candidate_record_sha256: Sha256DigestV1;
}
export interface LocalAdminExpectedHeadV1 {
  expected_head_epoch?: LocalAdminHeadEpochV1 | null;
  expected_head_ref?: ResidentRecordRefV1 | null;
  expected_head_sha256?: Sha256DigestV1 | null;
  head_kind: LocalAdminHeadSelectorV1;
}
