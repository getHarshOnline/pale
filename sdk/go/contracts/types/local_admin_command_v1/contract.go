// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package local_admin_command_v1 contains the generated local-admin-command.v1 data model.
package local_admin_command_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type CanonicalSignatureV1 string

type CheckpointSigningAuthorityIDV1 string

type CheckpointSigningScopeDeploymentV1 struct {
	// DeploymentBindingRef corresponds to the JSON schema field
	// "deployment_binding_ref".
	DeploymentBindingRef ResidentRecordRefV1 `json:"deployment_binding_ref"`

	// DeploymentBindingSha256 corresponds to the JSON schema field
	// "deployment_binding_sha256".
	DeploymentBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_binding_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type CheckpointSigningScopePrincipalTrustStoreV1 struct {
	// BootstrapID corresponds to the JSON schema field "bootstrap_id".
	BootstrapID PrincipalBootstrapIDV1 `json:"bootstrap_id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ControlCheckpointDomainIDV1 string

type ControlIncarnationIDV1 string

type ControlStoreIDV1 string

type DeploymentRuntimeAuthorityIDV1 string

type ExecutionOwnerFenceIDV1 string

type ExecutionPathSelectionIDV1 string

type LocalAdminCandidateRecordV1 struct {
	// CandidateRecordRef corresponds to the JSON schema field "candidate_record_ref".
	CandidateRecordRef ResidentRecordRefV1 `json:"candidate_record_ref"`

	// CandidateRecordSchemaID corresponds to the JSON schema field
	// "candidate_record_schema_id".
	CandidateRecordSchemaID SchemaID `json:"candidate_record_schema_id"`

	// CandidateRecordSha256 corresponds to the JSON schema field
	// "candidate_record_sha256".
	CandidateRecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"candidate_record_sha256"`
}

type LocalAdminCommandIDV1 string

type LocalAdminCommandV1 struct {
	// AdminAuthorityRef corresponds to the JSON schema field "admin_authority_ref".
	AdminAuthorityRef ResidentRecordRefV1 `json:"admin_authority_ref"`

	// AdminAuthoritySha256 corresponds to the JSON schema field
	// "admin_authority_sha256".
	AdminAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"admin_authority_sha256"`

	// AdminCommandID corresponds to the JSON schema field "admin_command_id".
	AdminCommandID LocalAdminCommandIDV1 `json:"admin_command_id"`

	// CandidateRecords corresponds to the JSON schema field "candidate_records".
	CandidateRecords []LocalAdminCandidateRecordV1 `json:"candidate_records"`

	// CandidateSetSha256 corresponds to the JSON schema field "candidate_set_sha256".
	CandidateSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"candidate_set_sha256"`

	// CanonicalAdminCommandSha256 corresponds to the JSON schema field
	// "canonical_admin_command_sha256".
	CanonicalAdminCommandSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_admin_command_sha256"`

	// ExpectedHeads corresponds to the JSON schema field "expected_heads".
	ExpectedHeads []LocalAdminExpectedHeadV1 `json:"expected_heads"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt UnixTimeV1 `json:"expires_at"`

	// IdempotencyKey corresponds to the JSON schema field "idempotency_key".
	IdempotencyKey LocalAdminIdempotencyKeyV1 `json:"idempotency_key"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// Operation corresponds to the JSON schema field "operation".
	Operation LocalAdminOperationV1 `json:"operation"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// SignerIdentity corresponds to the JSON schema field "signer_identity".
	SignerIdentity LocalAdminSubjectIdentityV1 `json:"signer_identity"`

	// SignerKeyEpoch corresponds to the JSON schema field "signer_key_epoch".
	SignerKeyEpoch SigningKeyEpochV1 `json:"signer_key_epoch"`

	// SignerKeyID corresponds to the JSON schema field "signer_key_id".
	SignerKeyID ResidentKeyIDV1 `json:"signer_key_id"`
}

type LocalAdminExpectedHeadV1 struct {
	// ExpectedHeadEpoch corresponds to the JSON schema field "expected_head_epoch".
	ExpectedHeadEpoch *LocalAdminHeadEpochV1 `json:"expected_head_epoch,omitempty,omitzero"`

	// ExpectedHeadRef corresponds to the JSON schema field "expected_head_ref".
	ExpectedHeadRef *ResidentRecordRefV1 `json:"expected_head_ref,omitempty,omitzero"`

	// ExpectedHeadSha256 corresponds to the JSON schema field "expected_head_sha256".
	ExpectedHeadSha256 *sha256_digest_v1.Sha256DigestV1 `json:"expected_head_sha256,omitempty,omitzero"`

	// HeadKind corresponds to the JSON schema field "head_kind".
	HeadKind LocalAdminHeadSelectorV1 `json:"head_kind"`
}

type LocalAdminHeadEpochV1 int

type LocalAdminHeadSelectorCapabilityGrantRevocationV1 struct {
	// GrantRef corresponds to the JSON schema field "grant_ref".
	GrantRef ResidentRecordRefV1 `json:"grant_ref"`

	// GrantSha256 corresponds to the JSON schema field "grant_sha256".
	GrantSha256 sha256_digest_v1.Sha256DigestV1 `json:"grant_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type LocalAdminHeadSelectorCapabilityRevocationCheckpointV1 struct {
	// AuthorityPrincipalID corresponds to the JSON schema field
	// "authority_principal_id".
	AuthorityPrincipalID ResidentPrincipalIDV1 `json:"authority_principal_id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type LocalAdminHeadSelectorCheckpointSigningAuthorityV1 struct {
	// CheckpointSigningAuthorityID corresponds to the JSON schema field
	// "checkpoint_signing_authority_id".
	CheckpointSigningAuthorityID CheckpointSigningAuthorityIDV1 `json:"checkpoint_signing_authority_id"`

	// ControlIncarnationID corresponds to the JSON schema field
	// "control_incarnation_id".
	ControlIncarnationID ControlIncarnationIDV1 `json:"control_incarnation_id"`

	// ControlStoreID corresponds to the JSON schema field "control_store_id".
	ControlStoreID ControlStoreIDV1 `json:"control_store_id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// Scope corresponds to the JSON schema field "scope".
	Scope CheckpointSigningScopeV1 `json:"scope"`
}

type LocalAdminHeadSelectorControlCheckpointV1 struct {
	// CheckpointDomainID corresponds to the JSON schema field "checkpoint_domain_id".
	CheckpointDomainID ControlCheckpointDomainIDV1 `json:"checkpoint_domain_id"`

	// ControlIncarnationID corresponds to the JSON schema field
	// "control_incarnation_id".
	ControlIncarnationID ControlIncarnationIDV1 `json:"control_incarnation_id"`

	// ControlStoreID corresponds to the JSON schema field "control_store_id".
	ControlStoreID ControlStoreIDV1 `json:"control_store_id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type LocalAdminHeadSelectorDeploymentAcceptanceRevocationV1 struct {
	// DeploymentAcceptanceRef corresponds to the JSON schema field
	// "deployment_acceptance_ref".
	DeploymentAcceptanceRef ResidentRecordRefV1 `json:"deployment_acceptance_ref"`

	// DeploymentAcceptanceSha256 corresponds to the JSON schema field
	// "deployment_acceptance_sha256".
	DeploymentAcceptanceSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_acceptance_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type LocalAdminHeadSelectorDeploymentRuntimeAuthorityV1 struct {
	// ControlIncarnationID corresponds to the JSON schema field
	// "control_incarnation_id".
	ControlIncarnationID ControlIncarnationIDV1 `json:"control_incarnation_id"`

	// ControlStoreID corresponds to the JSON schema field "control_store_id".
	ControlStoreID ControlStoreIDV1 `json:"control_store_id"`

	// DeploymentBindingRef corresponds to the JSON schema field
	// "deployment_binding_ref".
	DeploymentBindingRef ResidentRecordRefV1 `json:"deployment_binding_ref"`

	// DeploymentBindingSha256 corresponds to the JSON schema field
	// "deployment_binding_sha256".
	DeploymentBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_binding_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// RuntimeAuthorityID corresponds to the JSON schema field "runtime_authority_id".
	RuntimeAuthorityID DeploymentRuntimeAuthorityIDV1 `json:"runtime_authority_id"`
}

type LocalAdminHeadSelectorExecutionFenceV1 struct {
	// ControllerScope corresponds to the JSON schema field "controller_scope".
	ControllerScope ResidentControllerIDV1 `json:"controller_scope"`

	// DeploymentBindingRef corresponds to the JSON schema field
	// "deployment_binding_ref".
	DeploymentBindingRef ResidentRecordRefV1 `json:"deployment_binding_ref"`

	// DeploymentBindingSha256 corresponds to the JSON schema field
	// "deployment_binding_sha256".
	DeploymentBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_binding_sha256"`

	// ExecutionFenceID corresponds to the JSON schema field "execution_fence_id".
	ExecutionFenceID ExecutionOwnerFenceIDV1 `json:"execution_fence_id"`

	// ExecutionPathID corresponds to the JSON schema field "execution_path_id".
	ExecutionPathID ExecutionPathSelectionIDV1 `json:"execution_path_id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// PrincipalScope corresponds to the JSON schema field "principal_scope".
	PrincipalScope ResidentPrincipalIDV1 `json:"principal_scope"`

	// RelationshipScope corresponds to the JSON schema field "relationship_scope".
	RelationshipScope *RelationshipScopeV1 `json:"relationship_scope,omitempty,omitzero"`
}

type LocalAdminHeadSelectorFederationRouteV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// RouteID corresponds to the JSON schema field "route_id".
	RouteID ResidentRouteIDV1 `json:"route_id"`

	// RouteOwnerPrincipalID corresponds to the JSON schema field
	// "route_owner_principal_id".
	RouteOwnerPrincipalID ResidentPrincipalIDV1 `json:"route_owner_principal_id"`
}

type LocalAdminHeadSelectorLocalAdminAuthorityV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`
}

type LocalAdminHeadSelectorLocalRouteV1 struct {
	// DeploymentBindingRef corresponds to the JSON schema field
	// "deployment_binding_ref".
	DeploymentBindingRef ResidentRecordRefV1 `json:"deployment_binding_ref"`

	// DeploymentBindingSha256 corresponds to the JSON schema field
	// "deployment_binding_sha256".
	DeploymentBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_binding_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// RouteID corresponds to the JSON schema field "route_id".
	RouteID LocalRouteIDV1 `json:"route_id"`

	// RouteOwnerPrincipalID corresponds to the JSON schema field
	// "route_owner_principal_id".
	RouteOwnerPrincipalID ResidentPrincipalIDV1 `json:"route_owner_principal_id"`

	// WorkspaceID corresponds to the JSON schema field "workspace_id".
	WorkspaceID ResidentWorkspaceIDV1 `json:"workspace_id"`
}

type LocalAdminHeadSelectorNodeDelegationV1 struct {
	// DelegationID corresponds to the JSON schema field "delegation_id".
	DelegationID NodeDelegationIDV1 `json:"delegation_id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// NodeID corresponds to the JSON schema field "node_id".
	NodeID ResidentNodeIDV1 `json:"node_id"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`
}

type LocalAdminHeadSelectorNodeRevocationV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// NodeID corresponds to the JSON schema field "node_id".
	NodeID ResidentNodeIDV1 `json:"node_id"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`
}

type LocalAdminHeadSelectorOwnerRecordSigningAuthorityV1 struct {
	// DeploymentBindingRef corresponds to the JSON schema field
	// "deployment_binding_ref".
	DeploymentBindingRef ResidentRecordRefV1 `json:"deployment_binding_ref"`

	// DeploymentBindingSha256 corresponds to the JSON schema field
	// "deployment_binding_sha256".
	DeploymentBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_binding_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// OwnerComponentID corresponds to the JSON schema field "owner_component_id".
	OwnerComponentID OwnerComponentIDV1 `json:"owner_component_id"`

	// OwnerContractManifestRef corresponds to the JSON schema field
	// "owner_contract_manifest_ref".
	OwnerContractManifestRef ResidentRecordRefV1 `json:"owner_contract_manifest_ref"`

	// OwnerContractManifestSha256 corresponds to the JSON schema field
	// "owner_contract_manifest_sha256".
	OwnerContractManifestSha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_contract_manifest_sha256"`

	// OwnerRecordSigningAuthorityID corresponds to the JSON schema field
	// "owner_record_signing_authority_id".
	OwnerRecordSigningAuthorityID OwnerRecordSigningAuthorityIDV1 `json:"owner_record_signing_authority_id"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`
}

type LocalAdminHeadSelectorPrincipalRecordV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`
}

type LocalAdminHeadSelectorPrincipalRecoveryPolicyV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`
}

type LocalAdminHeadSelectorPrincipalTrustCheckpointV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// TrustStoreID corresponds to the JSON schema field "trust_store_id".
	TrustStoreID ControlStoreIDV1 `json:"trust_store_id"`

	// TrustStoreIncarnationID corresponds to the JSON schema field
	// "trust_store_incarnation_id".
	TrustStoreIncarnationID ControlIncarnationIDV1 `json:"trust_store_incarnation_id"`
}

type LocalAdminHeadSelectorProducerIncarnationAuthorityV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`
}

type LocalAdminHeadSelectorProducerIncarnationV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// OriginPrincipalID corresponds to the JSON schema field "origin_principal_id".
	OriginPrincipalID ResidentPrincipalIDV1 `json:"origin_principal_id"`

	// ProducerID corresponds to the JSON schema field "producer_id".
	ProducerID ProducerIDV1 `json:"producer_id"`
}

type LocalAdminIdempotencyKeyV1 string

type LocalAdminOperationV1 string

const LocalAdminOperationV1AcceptDeployment LocalAdminOperationV1 = "accept_deployment"
const LocalAdminOperationV1AdvanceExecutionFence LocalAdminOperationV1 = "advance_execution_fence"
const LocalAdminOperationV1AdvanceLocalRoute LocalAdminOperationV1 = "advance_local_route"
const LocalAdminOperationV1AdvanceRouteBinding LocalAdminOperationV1 = "advance_route_binding"
const LocalAdminOperationV1AllocateProducerIncarnation LocalAdminOperationV1 = "allocate_producer_incarnation"
const LocalAdminOperationV1CreateLocalRoute LocalAdminOperationV1 = "create_local_route"
const LocalAdminOperationV1CreateRouteBinding LocalAdminOperationV1 = "create_route_binding"
const LocalAdminOperationV1FinalizeDeployment LocalAdminOperationV1 = "finalize_deployment"
const LocalAdminOperationV1IssueApplyAuthorization LocalAdminOperationV1 = "issue_apply_authorization"
const LocalAdminOperationV1IssueNodeDelegation LocalAdminOperationV1 = "issue_node_delegation"
const LocalAdminOperationV1IssueOwnerRecordSigningAuthority LocalAdminOperationV1 = "issue_owner_record_signing_authority"
const LocalAdminOperationV1IssueSessionGrant LocalAdminOperationV1 = "issue_session_grant"
const LocalAdminOperationV1ProposeDeployment LocalAdminOperationV1 = "propose_deployment"
const LocalAdminOperationV1RecoverPrincipalKey LocalAdminOperationV1 = "recover_principal_key"
const LocalAdminOperationV1RetireOwnerRecordSigningAuthority LocalAdminOperationV1 = "retire_owner_record_signing_authority"
const LocalAdminOperationV1RetireProducerIncarnation LocalAdminOperationV1 = "retire_producer_incarnation"
const LocalAdminOperationV1RevokeDeploymentAcceptance LocalAdminOperationV1 = "revoke_deployment_acceptance"
const LocalAdminOperationV1RevokeNodeDelegation LocalAdminOperationV1 = "revoke_node_delegation"
const LocalAdminOperationV1RevokeSessionGrant LocalAdminOperationV1 = "revoke_session_grant"
const LocalAdminOperationV1RotateAdminAuthority LocalAdminOperationV1 = "rotate_admin_authority"
const LocalAdminOperationV1RotateCheckpointSigner LocalAdminOperationV1 = "rotate_checkpoint_signer"
const LocalAdminOperationV1RotateDeploymentRuntimeAuthority LocalAdminOperationV1 = "rotate_deployment_runtime_authority"
const LocalAdminOperationV1RotateOwnerRecordSigningAuthority LocalAdminOperationV1 = "rotate_owner_record_signing_authority"
const LocalAdminOperationV1RotatePrincipalKey LocalAdminOperationV1 = "rotate_principal_key"
const LocalAdminOperationV1RotateProducerIncarnationAuthority LocalAdminOperationV1 = "rotate_producer_incarnation_authority"
const LocalAdminOperationV1UpdatePrincipalRecoveryPolicy LocalAdminOperationV1 = "update_principal_recovery_policy"

type LocalAdminSubjectIdentityV1 string

type LocalRouteIDV1 string

type NodeDelegationIDV1 string

type OwnerComponentIDV1 string

type OwnerRecordSigningAuthorityIDV1 string

type PrincipalBootstrapIDV1 string

type ProducerIDV1 string

type RelationshipScopeV1 string

type ResidentControllerIDV1 string

type ResidentKeyIDV1 string

type ResidentNodeIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type ResidentRouteIDV1 string

type ResidentWorkspaceIDV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int

type CheckpointSigningScopeV1 struct {
	PrincipalTrustStore *CheckpointSigningScopePrincipalTrustStoreV1 `json:"-"`
	Deployment          *CheckpointSigningScopeDeploymentV1          `json:"-"`
}

func (value CheckpointSigningScopeV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.PrincipalTrustStore != nil {
		if value.PrincipalTrustStore.Kind != "principal_trust_store" {
			return nil, fmt.Errorf("CheckpointSigningScopeV1: selected variant has discriminator %q", value.PrincipalTrustStore.Kind)
		}
		candidate, err := json.Marshal(value.PrincipalTrustStore)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Deployment != nil {
		if value.Deployment.Kind != "deployment" {
			return nil, fmt.Errorf("CheckpointSigningScopeV1: selected variant has discriminator %q", value.Deployment.Kind)
		}
		candidate, err := json.Marshal(value.Deployment)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("CheckpointSigningScopeV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *CheckpointSigningScopeV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = CheckpointSigningScopeV1{}
	switch discriminator.Kind {
	case "principal_trust_store":
		var variant CheckpointSigningScopePrincipalTrustStoreV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.PrincipalTrustStore = &variant
	case "deployment":
		var variant CheckpointSigningScopeDeploymentV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Deployment = &variant
	default:
		return fmt.Errorf("CheckpointSigningScopeV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type LocalAdminHeadSelectorV1 struct {
	PrincipalRecord                *LocalAdminHeadSelectorPrincipalRecordV1                `json:"-"`
	PrincipalRecoveryPolicy        *LocalAdminHeadSelectorPrincipalRecoveryPolicyV1        `json:"-"`
	LocalAdminAuthority            *LocalAdminHeadSelectorLocalAdminAuthorityV1            `json:"-"`
	ProducerIncarnationAuthority   *LocalAdminHeadSelectorProducerIncarnationAuthorityV1   `json:"-"`
	PrincipalTrustCheckpoint       *LocalAdminHeadSelectorPrincipalTrustCheckpointV1       `json:"-"`
	ProducerIncarnation            *LocalAdminHeadSelectorProducerIncarnationV1            `json:"-"`
	CheckpointSigningAuthority     *LocalAdminHeadSelectorCheckpointSigningAuthorityV1     `json:"-"`
	DeploymentRuntimeAuthority     *LocalAdminHeadSelectorDeploymentRuntimeAuthorityV1     `json:"-"`
	ControlCheckpoint              *LocalAdminHeadSelectorControlCheckpointV1              `json:"-"`
	LocalRoute                     *LocalAdminHeadSelectorLocalRouteV1                     `json:"-"`
	ExecutionFence                 *LocalAdminHeadSelectorExecutionFenceV1                 `json:"-"`
	DeploymentAcceptanceRevocation *LocalAdminHeadSelectorDeploymentAcceptanceRevocationV1 `json:"-"`
	CapabilityGrantRevocation      *LocalAdminHeadSelectorCapabilityGrantRevocationV1      `json:"-"`
	CapabilityRevocationCheckpoint *LocalAdminHeadSelectorCapabilityRevocationCheckpointV1 `json:"-"`
	OwnerRecordSigningAuthority    *LocalAdminHeadSelectorOwnerRecordSigningAuthorityV1    `json:"-"`
	NodeDelegation                 *LocalAdminHeadSelectorNodeDelegationV1                 `json:"-"`
	NodeRevocation                 *LocalAdminHeadSelectorNodeRevocationV1                 `json:"-"`
	FederationRoute                *LocalAdminHeadSelectorFederationRouteV1                `json:"-"`
}

func (value LocalAdminHeadSelectorV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.PrincipalRecord != nil {
		if value.PrincipalRecord.Kind != "principal_record" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.PrincipalRecord.Kind)
		}
		candidate, err := json.Marshal(value.PrincipalRecord)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.PrincipalRecoveryPolicy != nil {
		if value.PrincipalRecoveryPolicy.Kind != "principal_recovery_policy" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.PrincipalRecoveryPolicy.Kind)
		}
		candidate, err := json.Marshal(value.PrincipalRecoveryPolicy)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.LocalAdminAuthority != nil {
		if value.LocalAdminAuthority.Kind != "local_admin_authority" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.LocalAdminAuthority.Kind)
		}
		candidate, err := json.Marshal(value.LocalAdminAuthority)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ProducerIncarnationAuthority != nil {
		if value.ProducerIncarnationAuthority.Kind != "producer_incarnation_authority" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.ProducerIncarnationAuthority.Kind)
		}
		candidate, err := json.Marshal(value.ProducerIncarnationAuthority)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.PrincipalTrustCheckpoint != nil {
		if value.PrincipalTrustCheckpoint.Kind != "principal_trust_checkpoint" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.PrincipalTrustCheckpoint.Kind)
		}
		candidate, err := json.Marshal(value.PrincipalTrustCheckpoint)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ProducerIncarnation != nil {
		if value.ProducerIncarnation.Kind != "producer_incarnation" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.ProducerIncarnation.Kind)
		}
		candidate, err := json.Marshal(value.ProducerIncarnation)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.CheckpointSigningAuthority != nil {
		if value.CheckpointSigningAuthority.Kind != "checkpoint_signing_authority" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.CheckpointSigningAuthority.Kind)
		}
		candidate, err := json.Marshal(value.CheckpointSigningAuthority)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.DeploymentRuntimeAuthority != nil {
		if value.DeploymentRuntimeAuthority.Kind != "deployment_runtime_authority" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.DeploymentRuntimeAuthority.Kind)
		}
		candidate, err := json.Marshal(value.DeploymentRuntimeAuthority)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ControlCheckpoint != nil {
		if value.ControlCheckpoint.Kind != "control_checkpoint" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.ControlCheckpoint.Kind)
		}
		candidate, err := json.Marshal(value.ControlCheckpoint)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.LocalRoute != nil {
		if value.LocalRoute.Kind != "local_route" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.LocalRoute.Kind)
		}
		candidate, err := json.Marshal(value.LocalRoute)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ExecutionFence != nil {
		if value.ExecutionFence.Kind != "execution_fence" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.ExecutionFence.Kind)
		}
		candidate, err := json.Marshal(value.ExecutionFence)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.DeploymentAcceptanceRevocation != nil {
		if value.DeploymentAcceptanceRevocation.Kind != "deployment_acceptance_revocation" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.DeploymentAcceptanceRevocation.Kind)
		}
		candidate, err := json.Marshal(value.DeploymentAcceptanceRevocation)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.CapabilityGrantRevocation != nil {
		if value.CapabilityGrantRevocation.Kind != "capability_grant_revocation" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.CapabilityGrantRevocation.Kind)
		}
		candidate, err := json.Marshal(value.CapabilityGrantRevocation)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.CapabilityRevocationCheckpoint != nil {
		if value.CapabilityRevocationCheckpoint.Kind != "capability_revocation_checkpoint" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.CapabilityRevocationCheckpoint.Kind)
		}
		candidate, err := json.Marshal(value.CapabilityRevocationCheckpoint)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.OwnerRecordSigningAuthority != nil {
		if value.OwnerRecordSigningAuthority.Kind != "owner_record_signing_authority" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.OwnerRecordSigningAuthority.Kind)
		}
		candidate, err := json.Marshal(value.OwnerRecordSigningAuthority)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.NodeDelegation != nil {
		if value.NodeDelegation.Kind != "node_delegation" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.NodeDelegation.Kind)
		}
		candidate, err := json.Marshal(value.NodeDelegation)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.NodeRevocation != nil {
		if value.NodeRevocation.Kind != "node_revocation" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.NodeRevocation.Kind)
		}
		candidate, err := json.Marshal(value.NodeRevocation)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.FederationRoute != nil {
		if value.FederationRoute.Kind != "federation_route" {
			return nil, fmt.Errorf("LocalAdminHeadSelectorV1: selected variant has discriminator %q", value.FederationRoute.Kind)
		}
		candidate, err := json.Marshal(value.FederationRoute)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("LocalAdminHeadSelectorV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *LocalAdminHeadSelectorV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = LocalAdminHeadSelectorV1{}
	switch discriminator.Kind {
	case "principal_record":
		var variant LocalAdminHeadSelectorPrincipalRecordV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.PrincipalRecord = &variant
	case "principal_recovery_policy":
		var variant LocalAdminHeadSelectorPrincipalRecoveryPolicyV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.PrincipalRecoveryPolicy = &variant
	case "local_admin_authority":
		var variant LocalAdminHeadSelectorLocalAdminAuthorityV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.LocalAdminAuthority = &variant
	case "producer_incarnation_authority":
		var variant LocalAdminHeadSelectorProducerIncarnationAuthorityV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ProducerIncarnationAuthority = &variant
	case "principal_trust_checkpoint":
		var variant LocalAdminHeadSelectorPrincipalTrustCheckpointV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.PrincipalTrustCheckpoint = &variant
	case "producer_incarnation":
		var variant LocalAdminHeadSelectorProducerIncarnationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ProducerIncarnation = &variant
	case "checkpoint_signing_authority":
		var variant LocalAdminHeadSelectorCheckpointSigningAuthorityV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.CheckpointSigningAuthority = &variant
	case "deployment_runtime_authority":
		var variant LocalAdminHeadSelectorDeploymentRuntimeAuthorityV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.DeploymentRuntimeAuthority = &variant
	case "control_checkpoint":
		var variant LocalAdminHeadSelectorControlCheckpointV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ControlCheckpoint = &variant
	case "local_route":
		var variant LocalAdminHeadSelectorLocalRouteV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.LocalRoute = &variant
	case "execution_fence":
		var variant LocalAdminHeadSelectorExecutionFenceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ExecutionFence = &variant
	case "deployment_acceptance_revocation":
		var variant LocalAdminHeadSelectorDeploymentAcceptanceRevocationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.DeploymentAcceptanceRevocation = &variant
	case "capability_grant_revocation":
		var variant LocalAdminHeadSelectorCapabilityGrantRevocationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.CapabilityGrantRevocation = &variant
	case "capability_revocation_checkpoint":
		var variant LocalAdminHeadSelectorCapabilityRevocationCheckpointV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.CapabilityRevocationCheckpoint = &variant
	case "owner_record_signing_authority":
		var variant LocalAdminHeadSelectorOwnerRecordSigningAuthorityV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.OwnerRecordSigningAuthority = &variant
	case "node_delegation":
		var variant LocalAdminHeadSelectorNodeDelegationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NodeDelegation = &variant
	case "node_revocation":
		var variant LocalAdminHeadSelectorNodeRevocationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NodeRevocation = &variant
	case "federation_route":
		var variant LocalAdminHeadSelectorFederationRouteV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.FederationRoute = &variant
	default:
		return fmt.Errorf("LocalAdminHeadSelectorV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
