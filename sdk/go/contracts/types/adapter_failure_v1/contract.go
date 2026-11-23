// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package adapter_failure_v1 contains the generated adapter-failure.v1 data model.
package adapter_failure_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type AdapterFailureClassV1 string

const AdapterFailureClassV1Conflict AdapterFailureClassV1 = "conflict"
const AdapterFailureClassV1Denied AdapterFailureClassV1 = "denied"
const AdapterFailureClassV1Internal AdapterFailureClassV1 = "internal"
const AdapterFailureClassV1Invalid AdapterFailureClassV1 = "invalid"
const AdapterFailureClassV1Partial AdapterFailureClassV1 = "partial"
const AdapterFailureClassV1RateLimited AdapterFailureClassV1 = "rate_limited"
const AdapterFailureClassV1Timeout AdapterFailureClassV1 = "timeout"
const AdapterFailureClassV1Unavailable AdapterFailureClassV1 = "unavailable"
const AdapterFailureClassV1Unknown AdapterFailureClassV1 = "unknown"
const AdapterFailureClassV1UnsupportedSchema AdapterFailureClassV1 = "unsupported_schema"

type AdapterFailureEffectStateV1 string

const AdapterFailureEffectStateV1IndependentlyProvenNotApplied AdapterFailureEffectStateV1 = "independently_proven_not_applied"
const AdapterFailureEffectStateV1NotStarted AdapterFailureEffectStateV1 = "not_started"

type AdapterFailureIDV1 string

type AdapterFailureV1 struct {
	// AbsenceEvidenceRef corresponds to the JSON schema field "absence_evidence_ref".
	AbsenceEvidenceRef *ResidentRecordRefV1 `json:"absence_evidence_ref,omitempty,omitzero"`

	// AbsenceEvidenceSha256 corresponds to the JSON schema field
	// "absence_evidence_sha256".
	AbsenceEvidenceSha256 *sha256_digest_v1.Sha256DigestV1 `json:"absence_evidence_sha256,omitempty,omitzero"`

	// AttemptID corresponds to the JSON schema field "attempt_id".
	AttemptID CapabilityAttemptIDV1 `json:"attempt_id"`

	// BoundedMessage corresponds to the JSON schema field "bounded_message".
	BoundedMessage *BoundedResidentTextV1 `json:"bounded_message,omitempty,omitzero"`

	// CanonicalFailureSha256 corresponds to the JSON schema field
	// "canonical_failure_sha256".
	CanonicalFailureSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_failure_sha256"`

	// EffectState corresponds to the JSON schema field "effect_state".
	EffectState AdapterFailureEffectStateV1 `json:"effect_state"`

	// FailureClass corresponds to the JSON schema field "failure_class".
	FailureClass AdapterFailureClassV1 `json:"failure_class"`

	// FailureID corresponds to the JSON schema field "failure_id".
	FailureID AdapterFailureIDV1 `json:"failure_id"`

	// ImplementationKind corresponds to the JSON schema field "implementation_kind".
	ImplementationKind BoundedResidentTextV1 `json:"implementation_kind"`

	// ImplementationVersion corresponds to the JSON schema field
	// "implementation_version".
	ImplementationVersion BoundedResidentTextV1 `json:"implementation_version"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// ProviderStatus corresponds to the JSON schema field "provider_status".
	ProviderStatus *BoundedResidentTextV1 `json:"provider_status,omitempty,omitzero"`

	// RecordedAt corresponds to the JSON schema field "recorded_at".
	RecordedAt UnixTimeV1 `json:"recorded_at"`

	// RequestBinding corresponds to the JSON schema field "request_binding".
	RequestBinding RequestBindingV1 `json:"request_binding"`

	// RetryAt corresponds to the JSON schema field "retry_at".
	RetryAt *UnixTimeV1 `json:"retry_at,omitempty,omitzero"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// SigningAuthorityBinding corresponds to the JSON schema field
	// "signing_authority_binding".
	SigningAuthorityBinding SigningAuthorityBindingV1 `json:"signing_authority_binding"`

	// StableCode corresponds to the JSON schema field "stable_code".
	StableCode ResidentReasonV1 `json:"stable_code"`
}

type BoundedResidentTextV1 string

type CanonicalSignatureV1 string

type CapabilityAttemptIDV1 string

type CapabilityIDV1 string

type IntegratedOwnerAuthorityV1 struct {
	// CheckedPrincipalTrustCheckpointRef corresponds to the JSON schema field
	// "checked_principal_trust_checkpoint_ref".
	CheckedPrincipalTrustCheckpointRef ResidentRecordRefV1 `json:"checked_principal_trust_checkpoint_ref"`

	// CheckedPrincipalTrustCheckpointSha256 corresponds to the JSON schema field
	// "checked_principal_trust_checkpoint_sha256".
	CheckedPrincipalTrustCheckpointSha256 sha256_digest_v1.Sha256DigestV1 `json:"checked_principal_trust_checkpoint_sha256"`

	// OwnerComponentID corresponds to the JSON schema field "owner_component_id".
	OwnerComponentID OwnerComponentIDV1 `json:"owner_component_id"`

	// OwnerContractManifestRef corresponds to the JSON schema field
	// "owner_contract_manifest_ref".
	OwnerContractManifestRef ResidentRecordRefV1 `json:"owner_contract_manifest_ref"`

	// OwnerContractManifestSha256 corresponds to the JSON schema field
	// "owner_contract_manifest_sha256".
	OwnerContractManifestSha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_contract_manifest_sha256"`

	// OwnerRecordSigningAuthorityRef corresponds to the JSON schema field
	// "owner_record_signing_authority_ref".
	OwnerRecordSigningAuthorityRef ResidentRecordRefV1 `json:"owner_record_signing_authority_ref"`

	// OwnerRecordSigningAuthoritySha256 corresponds to the JSON schema field
	// "owner_record_signing_authority_sha256".
	OwnerRecordSigningAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_record_signing_authority_sha256"`

	// OwnerSignerRecordSequence corresponds to the JSON schema field
	// "owner_signer_record_sequence".
	OwnerSignerRecordSequence OwnerSignerRecordSequenceV1 `json:"owner_signer_record_sequence"`

	// PreviousOwnerRecordSha256 corresponds to the JSON schema field
	// "previous_owner_record_sha256".
	PreviousOwnerRecordSha256 *sha256_digest_v1.Sha256DigestV1 `json:"previous_owner_record_sha256,omitempty,omitzero"`
}

type IntegratedRuntimeAuthorityV1 struct {
	// CheckedPrincipalTrustCheckpointRef corresponds to the JSON schema field
	// "checked_principal_trust_checkpoint_ref".
	CheckedPrincipalTrustCheckpointRef ResidentRecordRefV1 `json:"checked_principal_trust_checkpoint_ref"`

	// CheckedPrincipalTrustCheckpointSha256 corresponds to the JSON schema field
	// "checked_principal_trust_checkpoint_sha256".
	CheckedPrincipalTrustCheckpointSha256 sha256_digest_v1.Sha256DigestV1 `json:"checked_principal_trust_checkpoint_sha256"`

	// DeploymentBindingRef corresponds to the JSON schema field
	// "deployment_binding_ref".
	DeploymentBindingRef ResidentRecordRefV1 `json:"deployment_binding_ref"`

	// DeploymentBindingSha256 corresponds to the JSON schema field
	// "deployment_binding_sha256".
	DeploymentBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_binding_sha256"`

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`
}

type OwnerComponentIDV1 string

type OwnerSignerRecordSequenceV1 int

type RequestBindingV1 struct {
	// CanonicalRequestSha256 corresponds to the JSON schema field
	// "canonical_request_sha256".
	CanonicalRequestSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_request_sha256"`

	// CapabilityID corresponds to the JSON schema field "capability_id".
	CapabilityID CapabilityIDV1 `json:"capability_id"`

	// ContractSetRef corresponds to the JSON schema field "contract_set_ref".
	ContractSetRef ResidentRecordRefV1 `json:"contract_set_ref"`

	// ContractSetSha256 corresponds to the JSON schema field "contract_set_sha256".
	ContractSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"contract_set_sha256"`

	// CorrelationID corresponds to the JSON schema field "correlation_id".
	CorrelationID ResidentCorrelationIDV1 `json:"correlation_id"`

	// DeploymentBindingRef corresponds to the JSON schema field
	// "deployment_binding_ref".
	DeploymentBindingRef ResidentRecordRefV1 `json:"deployment_binding_ref"`

	// DeploymentBindingSha256 corresponds to the JSON schema field
	// "deployment_binding_sha256".
	DeploymentBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_binding_sha256"`

	// ExecutionOwnerFenceRef corresponds to the JSON schema field
	// "execution_owner_fence_ref".
	ExecutionOwnerFenceRef *ResidentRecordRefV1 `json:"execution_owner_fence_ref,omitempty,omitzero"`

	// ExecutionOwnerFenceSha256 corresponds to the JSON schema field
	// "execution_owner_fence_sha256".
	ExecutionOwnerFenceSha256 *sha256_digest_v1.Sha256DigestV1 `json:"execution_owner_fence_sha256,omitempty,omitzero"`

	// FederationRequestAdmissionRef corresponds to the JSON schema field
	// "federation_request_admission_ref".
	FederationRequestAdmissionRef *ResidentRecordRefV1 `json:"federation_request_admission_ref,omitempty,omitzero"`

	// FederationRequestAdmissionSha256 corresponds to the JSON schema field
	// "federation_request_admission_sha256".
	FederationRequestAdmissionSha256 *sha256_digest_v1.Sha256DigestV1 `json:"federation_request_admission_sha256,omitempty,omitzero"`

	// RequestID corresponds to the JSON schema field "request_id".
	RequestID ResidentRequestIDV1 `json:"request_id"`

	// RouteBindingRef corresponds to the JSON schema field "route_binding_ref".
	RouteBindingRef ResidentRecordRefV1 `json:"route_binding_ref"`

	// RouteBindingSchemaID corresponds to the JSON schema field
	// "route_binding_schema_id".
	RouteBindingSchemaID SchemaID `json:"route_binding_schema_id"`

	// RouteBindingSchemaSha256 corresponds to the JSON schema field
	// "route_binding_schema_sha256".
	RouteBindingSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"route_binding_schema_sha256"`

	// RouteBindingSha256 corresponds to the JSON schema field "route_binding_sha256".
	RouteBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"route_binding_sha256"`

	// RouteGeneration corresponds to the JSON schema field "route_generation".
	RouteGeneration RouteGenerationV1 `json:"route_generation"`

	// SessionGrantID corresponds to the JSON schema field "session_grant_id".
	SessionGrantID SessionGrantIDV1 `json:"session_grant_id"`

	// SessionGrantSha256 corresponds to the JSON schema field "session_grant_sha256".
	SessionGrantSha256 sha256_digest_v1.Sha256DigestV1 `json:"session_grant_sha256"`
}

type ResidentCorrelationIDV1 string

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentReasonV1 string

type ResidentRecordRefV1 string

type ResidentRequestIDV1 string

type RouteGenerationV1 int

type SchemaID string

type SessionGrantIDV1 string

type SigningAuthorityBindingV1 struct {
	// AuthorityMode corresponds to the JSON schema field "authority_mode".
	AuthorityMode SigningAuthorityModeV1 `json:"authority_mode"`

	// IntegratedOwner corresponds to the JSON schema field "integrated_owner".
	IntegratedOwner *IntegratedOwnerAuthorityV1 `json:"integrated_owner,omitempty,omitzero"`

	// IntegratedRuntime corresponds to the JSON schema field "integrated_runtime".
	IntegratedRuntime *IntegratedRuntimeAuthorityV1 `json:"integrated_runtime,omitempty,omitzero"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// SignerKeyEpoch corresponds to the JSON schema field "signer_key_epoch".
	SignerKeyEpoch SigningKeyEpochV1 `json:"signer_key_epoch"`

	// SignerKeyID corresponds to the JSON schema field "signer_key_id".
	SignerKeyID ResidentKeyIDV1 `json:"signer_key_id"`

	// StandaloneOwnerNative corresponds to the JSON schema field
	// "standalone_owner_native".
	StandaloneOwnerNative *StandaloneOwnerNativeAuthorityV1 `json:"standalone_owner_native,omitempty,omitzero"`
}

type SigningAuthorityModeV1 string

const SigningAuthorityModeV1IntegratedOwner SigningAuthorityModeV1 = "integrated_owner"
const SigningAuthorityModeV1IntegratedRuntime SigningAuthorityModeV1 = "integrated_runtime"
const SigningAuthorityModeV1StandaloneOwnerNative SigningAuthorityModeV1 = "standalone_owner_native"

type SigningKeyEpochV1 int

type StandaloneOwnerNativeAuthorityV1 struct {
	// CheckedOwnerRevocationFrontierRef corresponds to the JSON schema field
	// "checked_owner_revocation_frontier_ref".
	CheckedOwnerRevocationFrontierRef ResidentRecordRefV1 `json:"checked_owner_revocation_frontier_ref"`

	// CheckedOwnerRevocationFrontierSha256 corresponds to the JSON schema field
	// "checked_owner_revocation_frontier_sha256".
	CheckedOwnerRevocationFrontierSha256 sha256_digest_v1.Sha256DigestV1 `json:"checked_owner_revocation_frontier_sha256"`

	// OwnerAuthorityRef corresponds to the JSON schema field "owner_authority_ref".
	OwnerAuthorityRef ResidentRecordRefV1 `json:"owner_authority_ref"`

	// OwnerAuthoritySchemaID corresponds to the JSON schema field
	// "owner_authority_schema_id".
	OwnerAuthoritySchemaID SchemaID `json:"owner_authority_schema_id"`

	// OwnerAuthoritySchemaSha256 corresponds to the JSON schema field
	// "owner_authority_schema_sha256".
	OwnerAuthoritySchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_authority_schema_sha256"`

	// OwnerAuthoritySha256 corresponds to the JSON schema field
	// "owner_authority_sha256".
	OwnerAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_authority_sha256"`

	// OwnerComponentID corresponds to the JSON schema field "owner_component_id".
	OwnerComponentID OwnerComponentIDV1 `json:"owner_component_id"`

	// OwnerContractManifestRef corresponds to the JSON schema field
	// "owner_contract_manifest_ref".
	OwnerContractManifestRef ResidentRecordRefV1 `json:"owner_contract_manifest_ref"`

	// OwnerContractManifestSha256 corresponds to the JSON schema field
	// "owner_contract_manifest_sha256".
	OwnerContractManifestSha256 sha256_digest_v1.Sha256DigestV1 `json:"owner_contract_manifest_sha256"`
}

type UnixTimeV1 int
