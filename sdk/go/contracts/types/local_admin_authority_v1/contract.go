// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package local_admin_authority_v1 contains the generated local-admin-authority.v1 data model.
package local_admin_authority_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalSignatureV1 string

type Ed25519PublicKeyV1 string

type LocalAdminAuthorityEpochV1 int

type LocalAdminAuthorityIDV1 string

type LocalAdminAuthorityV1 struct {
	// AdminAuthorityID corresponds to the JSON schema field "admin_authority_id".
	AdminAuthorityID LocalAdminAuthorityIDV1 `json:"admin_authority_id"`

	// AllowedOperations corresponds to the JSON schema field "allowed_operations".
	AllowedOperations []LocalAdminOperationV1 `json:"allowed_operations"`

	// AuthorityEpoch corresponds to the JSON schema field "authority_epoch".
	AuthorityEpoch LocalAdminAuthorityEpochV1 `json:"authority_epoch"`

	// CanonicalAdminAuthoritySha256 corresponds to the JSON schema field
	// "canonical_admin_authority_sha256".
	CanonicalAdminAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_admin_authority_sha256"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt UnixTimeV1 `json:"expires_at"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch PrincipalKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// PreviousAuthorityRef corresponds to the JSON schema field
	// "previous_authority_ref".
	PreviousAuthorityRef *ResidentRecordRefV1 `json:"previous_authority_ref,omitempty,omitzero"`

	// PreviousAuthoritySha256 corresponds to the JSON schema field
	// "previous_authority_sha256".
	PreviousAuthoritySha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_authority_sha256,omitempty,omitzero"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// PrincipalRecordRef corresponds to the JSON schema field "principal_record_ref".
	PrincipalRecordRef ResidentRecordRefV1 `json:"principal_record_ref"`

	// PrincipalRecordSha256 corresponds to the JSON schema field
	// "principal_record_sha256".
	PrincipalRecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"principal_record_sha256"`

	// RecoveryPolicyRef corresponds to the JSON schema field "recovery_policy_ref".
	RecoveryPolicyRef ResidentRecordRefV1 `json:"recovery_policy_ref"`

	// RecoveryPolicySha256 corresponds to the JSON schema field
	// "recovery_policy_sha256".
	RecoveryPolicySha256 sha256_digest_v1.Sha256DigestV1 `json:"recovery_policy_sha256"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// State corresponds to the JSON schema field "state".
	State PrincipalAuthorityStateV1 `json:"state"`

	// SubjectIdentity corresponds to the JSON schema field "subject_identity".
	SubjectIdentity LocalAdminSubjectIdentityV1 `json:"subject_identity"`

	// SubjectKeyEpoch corresponds to the JSON schema field "subject_key_epoch".
	SubjectKeyEpoch SigningKeyEpochV1 `json:"subject_key_epoch"`

	// SubjectKeyID corresponds to the JSON schema field "subject_key_id".
	SubjectKeyID ResidentKeyIDV1 `json:"subject_key_id"`

	// SubjectPublicKey corresponds to the JSON schema field "subject_public_key".
	SubjectPublicKey Ed25519PublicKeyV1 `json:"subject_public_key"`
}

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

type PrincipalAuthorityStateV1 string

const PrincipalAuthorityStateV1Active PrincipalAuthorityStateV1 = "active"
const PrincipalAuthorityStateV1Retired PrincipalAuthorityStateV1 = "retired"

type PrincipalKeyEpochV1 int

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int
