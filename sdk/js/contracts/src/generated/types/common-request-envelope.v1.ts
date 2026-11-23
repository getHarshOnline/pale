/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type SchemaId = string;
export type RequestCallerIdV1 = string;
export type RequestCallerRoleV1 = string;
export type SigningKeyEpochV1 = number;
export type ResidentKeyIdV1 = string;
export type RequestCallerSurfaceV1 = string;
export type CanonicalSignatureV1 = string;
export type CapabilityIdV1 = string;
export type ResidentCorrelationIdV1 = string;
export type ExecutionControlEpochV1 = number;
export type ExecutionAttemptIdV1 = string;
export type ResidentRecordRefV1 = string;
export type ExecutionOwnerParticipantIdV1 = string;
export type ExecutionOwnerSystemIdV1 = string;
export type ExecutionPathSelectionIdV1 = string;
export type ExecutionOwnerGenerationV1 = number;
export type UnixTimeV1 = number;
export type FederationGrantIdV1 = string;
export type FederationMembershipIdV1 = string;
export type FederationMembershipEpochV1 = number;
export type ResidentPrincipalIdV1 = string;
export type FederationGovernanceEpochV1 = number;
export type FederationSpaceIdV1 = string;
export type RequestIdempotencyKeyV1 = string;
export type PackageIdV1 = string;
export type PackageResourceIdV1 = string;
export type ResidentRequestIdV1 = string;
export type BoundedResidentTextV1 = string;
export type ResidentNodeIdV1 = string;
export type RouteGenerationV1 = number;
export type ResidentRouteIdV1 = string;
export type ResidentControllerIdV1 = string;
export type RelationshipScopeV1 = string;
export type PrincipalRevocationEpochV1 = number;
export type PrincipalKeyEpochV1 = number;
export type SessionAdmissionIdV1 = string;
export type PrincipalSessionFloorEpochV1 = number;
export type SessionGrantIdV1 = string;
export type SigningAuthorityBindingV1 = SigningAuthorityBindingModeConstraintV1 & {
  authority_mode: SigningAuthorityModeV1;
  integrated_owner?: IntegratedOwnerAuthorityV1;
  integrated_runtime?: IntegratedRuntimeAuthorityV1;
  principal_id: ResidentPrincipalIdV1;
  signer_key_epoch: SigningKeyEpochV1;
  signer_key_id: ResidentKeyIdV1;
  standalone_owner_native?: StandaloneOwnerNativeAuthorityV1;
};
export type SigningAuthorityBindingModeConstraintV1 =
  | {
      authority_mode?: "integrated_runtime";
      [k: string]: any;
    }
  | {
      authority_mode?: "integrated_owner";
      [k: string]: any;
    }
  | {
      authority_mode?: "standalone_owner_native";
      [k: string]: any;
    };
export type SigningAuthorityModeV1 = "integrated_runtime" | "integrated_owner" | "standalone_owner_native";
export type OwnerComponentIdV1 = string;
export type OwnerSignerRecordSequenceV1 = number;
export type ResidentWorkspaceIdV1 = string;

export interface CommonRequestEnvelopeV1 {
  body: CommonRequestBodyV1;
  caller: RequestCallerV1;
  caller_signature: CanonicalSignatureV1;
  canonical_request_sha256: Sha256DigestV1;
  capability_id: CapabilityIdV1;
  correlation_id: ResidentCorrelationIdV1;
  envelope_schema_id: "pale/common-request-envelope.v1";
  envelope_schema_sha256: Sha256DigestV1;
  execution_owner_fence?: CommonRequestExecutionOwnerFenceV1;
  expected_precondition?: ExpectedPreconditionV1;
  expires_at: UnixTimeV1;
  federation_claim?: CommonRequestFederationClaimV1;
  fence_token?: Sha256DigestV1;
  generation?: Sha256DigestV1;
  idempotency_key: RequestIdempotencyKeyV1;
  issued_at: UnixTimeV1;
  operation: PackageResourceRefV1;
  operation_target?: OperationTargetReferenceV1;
  request_id: ResidentRequestIdV1;
  resource_constraints: BoundedResidentTextV1[];
  risk_class: BoundedResidentTextV1;
  route: CommonRequestRouteV1;
  scope: CommonRequestScopeV1;
  session_admission: CommonRequestSessionAdmissionV1;
  signing_authority_binding: SigningAuthorityBindingV1;
  workspace_id: ResidentWorkspaceIdV1;
}
export interface CommonRequestBodyV1 {
  payload: any;
  payload_sha256: Sha256DigestV1;
  schema_id: SchemaId;
  schema_sha256: Sha256DigestV1;
}
export interface RequestCallerV1 {
  identity: RequestCallerIdV1;
  role: RequestCallerRoleV1;
  signer_key_epoch: SigningKeyEpochV1;
  signer_key_id: ResidentKeyIdV1;
  surface: RequestCallerSurfaceV1;
}
export interface CommonRequestExecutionOwnerFenceV1 {
  control_epoch: ExecutionControlEpochV1;
  execution_attempt_id: ExecutionAttemptIdV1;
  execution_fence_ref: ResidentRecordRefV1;
  execution_fence_sha256: Sha256DigestV1;
  execution_owner_participant_id: ExecutionOwnerParticipantIdV1;
  execution_owner_system_id: ExecutionOwnerSystemIdV1;
  execution_path_id: ExecutionPathSelectionIdV1;
  owner_authorization_ref: ResidentRecordRefV1;
  owner_authorization_schema_id: SchemaId;
  owner_authorization_schema_sha256: Sha256DigestV1;
  owner_authorization_sha256: Sha256DigestV1;
  owner_generation: ExecutionOwnerGenerationV1;
}
export interface ExpectedPreconditionV1 {
  expected_state_sha256: Sha256DigestV1;
  state_schema_id: SchemaId;
  state_schema_sha256: Sha256DigestV1;
}
export interface CommonRequestFederationClaimV1 {
  grant_id: FederationGrantIdV1;
  grant_sha256: Sha256DigestV1;
  membership_id: FederationMembershipIdV1;
  membership_sha256: Sha256DigestV1;
  membership_state_epoch: FederationMembershipEpochV1;
  revocation_proofs: FederationRevocationProofRefV1[];
  space_governance_epoch: FederationGovernanceEpochV1;
  space_id: FederationSpaceIdV1;
}
export interface FederationRevocationProofRefV1 {
  authority_principal_id: ResidentPrincipalIdV1;
  proof_ref: ResidentRecordRefV1;
  proof_sha256: Sha256DigestV1;
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface OperationTargetReferenceV1 {
  operation_handle_ref: ResidentRecordRefV1;
  operation_handle_sha256: Sha256DigestV1;
}
export interface CommonRequestRouteV1 {
  contract_set_ref: ResidentRecordRefV1;
  contract_set_sha256: Sha256DigestV1;
  destination_node_id?: ResidentNodeIdV1;
  destination_principal_id: ResidentPrincipalIdV1;
  origin_node_id: ResidentNodeIdV1;
  origin_principal_id: ResidentPrincipalIdV1;
  route_binding_ref: ResidentRecordRefV1;
  route_binding_schema_id: SchemaId;
  route_binding_schema_sha256: Sha256DigestV1;
  route_binding_sha256: Sha256DigestV1;
  route_generation: RouteGenerationV1;
  route_id: ResidentRouteIdV1;
  transport: BoundedResidentTextV1;
}
export interface CommonRequestScopeV1 {
  authority_evidence: AuthorityEvidenceV1[];
  controller_scope: ResidentControllerIdV1;
  data_classification: BoundedResidentTextV1;
  environment: BoundedResidentTextV1;
  principal_scope: ResidentPrincipalIdV1;
  purpose: BoundedResidentTextV1;
  relationship_scope?: RelationshipScopeV1;
}
export interface AuthorityEvidenceV1 {
  authority_ref: ResidentRecordRefV1;
  authority_sha256: Sha256DigestV1;
}
export interface CommonRequestSessionAdmissionV1 {
  checked_at: UnixTimeV1;
  checked_revocation_checkpoint_ref: ResidentRecordRefV1;
  checked_revocation_checkpoint_sha256: Sha256DigestV1;
  checked_revocation_epoch: PrincipalRevocationEpochV1;
  checked_revocation_ref?: ResidentRecordRefV1;
  checked_revocation_sha256?: Sha256DigestV1;
  deployment_binding_ref: ResidentRecordRefV1;
  deployment_binding_sha256: Sha256DigestV1;
  grant_issuer_key_epoch: PrincipalKeyEpochV1;
  grant_issuer_key_id: ResidentKeyIdV1;
  grant_issuer_principal_id: ResidentPrincipalIdV1;
  session_admission_id: SessionAdmissionIdV1;
  session_admission_lease_digest: Sha256DigestV1;
  session_floor_epoch: PrincipalSessionFloorEpochV1;
  session_grant_id: SessionGrantIdV1;
  session_grant_sha256: Sha256DigestV1;
}
export interface IntegratedOwnerAuthorityV1 {
  checked_principal_trust_checkpoint_ref: ResidentRecordRefV1;
  checked_principal_trust_checkpoint_sha256: Sha256DigestV1;
  owner_component_id: OwnerComponentIdV1;
  owner_contract_manifest_ref: ResidentRecordRefV1;
  owner_contract_manifest_sha256: Sha256DigestV1;
  owner_record_signing_authority_ref: ResidentRecordRefV1;
  owner_record_signing_authority_sha256: Sha256DigestV1;
  owner_signer_record_sequence: OwnerSignerRecordSequenceV1;
  previous_owner_record_sha256?: Sha256DigestV1;
}
export interface IntegratedRuntimeAuthorityV1 {
  checked_principal_trust_checkpoint_ref: ResidentRecordRefV1;
  checked_principal_trust_checkpoint_sha256: Sha256DigestV1;
  deployment_binding_ref: ResidentRecordRefV1;
  deployment_binding_sha256: Sha256DigestV1;
  deployment_runtime_authority_ref: ResidentRecordRefV1;
  deployment_runtime_authority_sha256: Sha256DigestV1;
}
export interface StandaloneOwnerNativeAuthorityV1 {
  checked_owner_revocation_frontier_ref: ResidentRecordRefV1;
  checked_owner_revocation_frontier_sha256: Sha256DigestV1;
  owner_authority_ref: ResidentRecordRefV1;
  owner_authority_schema_id: SchemaId;
  owner_authority_schema_sha256: Sha256DigestV1;
  owner_authority_sha256: Sha256DigestV1;
  owner_component_id: OwnerComponentIdV1;
  owner_contract_manifest_ref: ResidentRecordRefV1;
  owner_contract_manifest_sha256: Sha256DigestV1;
}
