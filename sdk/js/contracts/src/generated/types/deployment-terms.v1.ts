/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ResidentRecordRefV1 = string;
export type ResidentControllerIdV1 = string;
export type DataClassificationV1 = string;
export type DeploymentIdV1 = string;
export type DistributionReleaseV1 = string;
export type DeploymentEnvironmentV1 = string;
export type PermittedPurposeV1 = string;
export type ResidentPrincipalIdV1 = string;
export type RelationshipScopeV1 = string;
export type WorkspaceCommitV1 = string;
export type WorkspaceId = string;

export interface DeploymentTermsV1 {
  canonical_deployment_terms_sha256: Sha256DigestV1;
  confidentiality_policy_ref: ResidentRecordRefV1;
  confidentiality_policy_sha256: Sha256DigestV1;
  controller_scope: ResidentControllerIdV1;
  data_classification: DataClassificationV1;
  deployment_id: DeploymentIdV1;
  distribution_release: DistributionReleaseV1;
  distribution_release_sha256: Sha256DigestV1;
  environment: DeploymentEnvironmentV1;
  environment_sha256: Sha256DigestV1;
  /**
   * @maxItems 1024
   */
  permitted_purposes: PermittedPurposeV1[];
  principal_scope: ResidentPrincipalIdV1;
  processing_role_ref: ResidentRecordRefV1;
  processing_role_sha256: Sha256DigestV1;
  relationship_scope?: RelationshipScopeV1;
  schema_id: "pale/deployment-terms.v1";
  schema_sha256: Sha256DigestV1;
  workspace_commit: WorkspaceCommitV1;
  workspace_id: WorkspaceId;
}
