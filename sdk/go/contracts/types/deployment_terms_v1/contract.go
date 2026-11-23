// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package deployment_terms_v1 contains the generated deployment-terms.v1 data model.
package deployment_terms_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type DataClassificationV1 string

type DeploymentEnvironmentV1 string

type DeploymentIDV1 string

type DeploymentTermsV1 struct {
	// CanonicalDeploymentTermsSha256 corresponds to the JSON schema field
	// "canonical_deployment_terms_sha256".
	CanonicalDeploymentTermsSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_deployment_terms_sha256"`

	// ConfidentialityPolicyRef corresponds to the JSON schema field
	// "confidentiality_policy_ref".
	ConfidentialityPolicyRef ResidentRecordRefV1 `json:"confidentiality_policy_ref"`

	// ConfidentialityPolicySha256 corresponds to the JSON schema field
	// "confidentiality_policy_sha256".
	ConfidentialityPolicySha256 sha256_digest_v1.Sha256DigestV1 `json:"confidentiality_policy_sha256"`

	// ControllerScope corresponds to the JSON schema field "controller_scope".
	ControllerScope ResidentControllerIDV1 `json:"controller_scope"`

	// DataClassification corresponds to the JSON schema field "data_classification".
	DataClassification DataClassificationV1 `json:"data_classification"`

	// DeploymentID corresponds to the JSON schema field "deployment_id".
	DeploymentID DeploymentIDV1 `json:"deployment_id"`

	// DistributionRelease corresponds to the JSON schema field
	// "distribution_release".
	DistributionRelease DistributionReleaseV1 `json:"distribution_release"`

	// DistributionReleaseSha256 corresponds to the JSON schema field
	// "distribution_release_sha256".
	DistributionReleaseSha256 sha256_digest_v1.Sha256DigestV1 `json:"distribution_release_sha256"`

	// Environment corresponds to the JSON schema field "environment".
	Environment DeploymentEnvironmentV1 `json:"environment"`

	// EnvironmentSha256 corresponds to the JSON schema field "environment_sha256".
	EnvironmentSha256 sha256_digest_v1.Sha256DigestV1 `json:"environment_sha256"`

	// PermittedPurposes corresponds to the JSON schema field "permitted_purposes".
	PermittedPurposes []PermittedPurposeV1 `json:"permitted_purposes"`

	// PrincipalScope corresponds to the JSON schema field "principal_scope".
	PrincipalScope ResidentPrincipalIDV1 `json:"principal_scope"`

	// ProcessingRoleRef corresponds to the JSON schema field "processing_role_ref".
	ProcessingRoleRef ResidentRecordRefV1 `json:"processing_role_ref"`

	// ProcessingRoleSha256 corresponds to the JSON schema field
	// "processing_role_sha256".
	ProcessingRoleSha256 sha256_digest_v1.Sha256DigestV1 `json:"processing_role_sha256"`

	// RelationshipScope corresponds to the JSON schema field "relationship_scope".
	RelationshipScope *RelationshipScopeV1 `json:"relationship_scope,omitempty,omitzero"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID string `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// WorkspaceCommit corresponds to the JSON schema field "workspace_commit".
	WorkspaceCommit WorkspaceCommitV1 `json:"workspace_commit"`

	// WorkspaceID corresponds to the JSON schema field "workspace_id".
	WorkspaceID WorkspaceID `json:"workspace_id"`
}

type DistributionReleaseV1 string

type PermittedPurposeV1 string

type RelationshipScopeV1 string

type ResidentControllerIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type WorkspaceCommitV1 string

type WorkspaceID string
