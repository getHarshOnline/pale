// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package session_admission_lease_v1 contains the generated session-admission-lease.v1 data model.
package session_admission_lease_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type CanonicalSignatureV1 string

type CapabilityIDV1 string

type EffectClass string

const EffectClassCompute EffectClass = "compute"
const EffectClassExecute EffectClass = "execute"
const EffectClassNetwork EffectClass = "network"
const EffectClassPublish EffectClass = "publish"
const EffectClassRead EffectClass = "read"
const EffectClassWrite EffectClass = "write"

type ExecutionPathSelectionIDV1 string

type ExecutionPathSelectionV1 struct {
	// ExperiencePathID corresponds to the JSON schema field "experience_path_id".
	ExperiencePathID string `json:"experience_path_id"`

	// ID corresponds to the JSON schema field "id".
	ID ExecutionPathSelectionIDV1 `json:"id"`
}

type HarnessActivityIdentitySession struct {
	// Digest corresponds to the JSON schema field "digest".
	Digest sha256_digest_v1.Sha256DigestV1 `json:"digest"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type HarnessActivityIdentityTurn struct {
	// Digest corresponds to the JSON schema field "digest".
	Digest sha256_digest_v1.Sha256DigestV1 `json:"digest"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type LeaseBudgetUnitsV1 int

type LeaseTimeoutMillisV1 int

type LogicalExecutionWorkspaceIDV1 string

type LogicalProjectIDV1 string

type LogicalRepositoryIDV1 string

type NativeSessionCorrelationObservedV1 struct {
	// NativeSessionIdentity corresponds to the JSON schema field
	// "native_session_identity".
	NativeSessionIdentity HarnessActivityIdentity `json:"native_session_identity"`

	// Visibility corresponds to the JSON schema field "visibility".
	Visibility string `json:"visibility"`
}

type NativeSessionCorrelationUnobservedV1 struct {
	// Reason corresponds to the JSON schema field "reason".
	Reason ResidentReasonV1 `json:"reason"`

	// Visibility corresponds to the JSON schema field "visibility".
	Visibility string `json:"visibility"`
}

type PrincipalRevocationEpochV1 int

type RelationshipScopeV1 string

type ResidentControllerIDV1 string

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentReasonV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SessionAdmissionIDV1 string

type SessionAdmissionLeaseV1 struct {
	// ActivationFenceDigest corresponds to the JSON schema field
	// "activation_fence_digest".
	ActivationFenceDigest sha256_digest_v1.Sha256DigestV1 `json:"activation_fence_digest"`

	// ActiveGenerationHeadDigest corresponds to the JSON schema field
	// "active_generation_head_digest".
	ActiveGenerationHeadDigest sha256_digest_v1.Sha256DigestV1 `json:"active_generation_head_digest"`

	// BudgetCeilingUnits corresponds to the JSON schema field "budget_ceiling_units".
	BudgetCeilingUnits LeaseBudgetUnitsV1 `json:"budget_ceiling_units"`

	// CapabilityCeiling corresponds to the JSON schema field "capability_ceiling".
	CapabilityCeiling []CapabilityIDV1 `json:"capability_ceiling"`

	// CheckedRevocationRef corresponds to the JSON schema field
	// "checked_revocation_ref".
	CheckedRevocationRef *ResidentRecordRefV1 `json:"checked_revocation_ref,omitempty,omitzero"`

	// CheckedRevocationSha256 corresponds to the JSON schema field
	// "checked_revocation_sha256".
	CheckedRevocationSha256 *sha256_digest_v1.Sha256DigestV1 `json:"checked_revocation_sha256,omitempty,omitzero"`

	// CompiledGenerationDigest corresponds to the JSON schema field
	// "compiled_generation_digest".
	CompiledGenerationDigest sha256_digest_v1.Sha256DigestV1 `json:"compiled_generation_digest"`

	// ControllerID corresponds to the JSON schema field "controller_id".
	ControllerID ResidentControllerIDV1 `json:"controller_id"`

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

	// EffectCeiling corresponds to the JSON schema field "effect_ceiling".
	EffectCeiling []EffectClass `json:"effect_ceiling"`

	// EvidenceDestination corresponds to the JSON schema field
	// "evidence_destination".
	EvidenceDestination ResidentRecordRefV1 `json:"evidence_destination"`

	// EvidenceObligations corresponds to the JSON schema field
	// "evidence_obligations".
	EvidenceObligations []ResidentRecordRefV1 `json:"evidence_obligations"`

	// ExecutionPath corresponds to the JSON schema field "execution_path".
	ExecutionPath ExecutionPathSelectionV1 `json:"execution_path"`

	// ExecutionWorkspaceID corresponds to the JSON schema field
	// "execution_workspace_id".
	ExecutionWorkspaceID LogicalExecutionWorkspaceIDV1 `json:"execution_workspace_id"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt UnixTimeV1 `json:"expires_at"`

	// HarnessAdapterBuildSha256 corresponds to the JSON schema field
	// "harness_adapter_build_sha256".
	HarnessAdapterBuildSha256 sha256_digest_v1.Sha256DigestV1 `json:"harness_adapter_build_sha256"`

	// HarnessAdapterSourceSha256 corresponds to the JSON schema field
	// "harness_adapter_source_sha256".
	HarnessAdapterSourceSha256 sha256_digest_v1.Sha256DigestV1 `json:"harness_adapter_source_sha256"`

	// HarnessConfigurationSha256 corresponds to the JSON schema field
	// "harness_configuration_sha256".
	HarnessConfigurationSha256 sha256_digest_v1.Sha256DigestV1 `json:"harness_configuration_sha256"`

	// HarnessExecutableSha256 corresponds to the JSON schema field
	// "harness_executable_sha256".
	HarnessExecutableSha256 sha256_digest_v1.Sha256DigestV1 `json:"harness_executable_sha256"`

	// InstanceDigest corresponds to the JSON schema field "instance_digest".
	InstanceDigest sha256_digest_v1.Sha256DigestV1 `json:"instance_digest"`

	// IssuedAt corresponds to the JSON schema field "issued_at".
	IssuedAt UnixTimeV1 `json:"issued_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// NativeSessionCorrelation corresponds to the JSON schema field
	// "native_session_correlation".
	NativeSessionCorrelation NativeSessionCorrelationV1 `json:"native_session_correlation"`

	// PrincipalID corresponds to the JSON schema field "principal_id".
	PrincipalID ResidentPrincipalIDV1 `json:"principal_id"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// ProjectID corresponds to the JSON schema field "project_id".
	ProjectID LogicalProjectIDV1 `json:"project_id"`

	// ReadCeiling corresponds to the JSON schema field "read_ceiling".
	ReadCeiling []ResidentRecordRefV1 `json:"read_ceiling"`

	// RelationshipScope corresponds to the JSON schema field "relationship_scope".
	RelationshipScope *RelationshipScopeV1 `json:"relationship_scope,omitempty,omitzero"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`

	// RevocationEpoch corresponds to the JSON schema field "revocation_epoch".
	RevocationEpoch PrincipalRevocationEpochV1 `json:"revocation_epoch"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SessionAdmissionID corresponds to the JSON schema field "session_admission_id".
	SessionAdmissionID SessionAdmissionIDV1 `json:"session_admission_id"`

	// SessionAdmissionLeaseDigest corresponds to the JSON schema field
	// "session_admission_lease_digest".
	SessionAdmissionLeaseDigest sha256_digest_v1.Sha256DigestV1 `json:"session_admission_lease_digest"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// TimeoutCeilingMillis corresponds to the JSON schema field
	// "timeout_ceiling_millis".
	TimeoutCeilingMillis LeaseTimeoutMillisV1 `json:"timeout_ceiling_millis"`
}

type SigningKeyEpochV1 int

type UnixTimeV1 int

type HarnessActivityIdentity struct {
	Session *HarnessActivityIdentitySession `json:"-"`
	Turn    *HarnessActivityIdentityTurn    `json:"-"`
}

func (value HarnessActivityIdentity) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Session != nil {
		if value.Session.Kind != "session" {
			return nil, fmt.Errorf("HarnessActivityIdentity: selected variant has discriminator %q", value.Session.Kind)
		}
		candidate, err := json.Marshal(value.Session)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Turn != nil {
		if value.Turn.Kind != "turn" {
			return nil, fmt.Errorf("HarnessActivityIdentity: selected variant has discriminator %q", value.Turn.Kind)
		}
		candidate, err := json.Marshal(value.Turn)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("HarnessActivityIdentity: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *HarnessActivityIdentity) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = HarnessActivityIdentity{}
	switch discriminator.Kind {
	case "session":
		var variant HarnessActivityIdentitySession
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Session = &variant
	case "turn":
		var variant HarnessActivityIdentityTurn
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Turn = &variant
	default:
		return fmt.Errorf("HarnessActivityIdentity: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type NativeSessionCorrelationV1 struct {
	Observed   *NativeSessionCorrelationObservedV1   `json:"-"`
	Unobserved *NativeSessionCorrelationUnobservedV1 `json:"-"`
}

func (value NativeSessionCorrelationV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Observed != nil {
		if value.Observed.Visibility != "observed" {
			return nil, fmt.Errorf("NativeSessionCorrelationV1: selected variant has discriminator %q", value.Observed.Visibility)
		}
		candidate, err := json.Marshal(value.Observed)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Unobserved != nil {
		if value.Unobserved.Visibility != "unobserved" {
			return nil, fmt.Errorf("NativeSessionCorrelationV1: selected variant has discriminator %q", value.Unobserved.Visibility)
		}
		candidate, err := json.Marshal(value.Unobserved)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("NativeSessionCorrelationV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *NativeSessionCorrelationV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Visibility string `json:"visibility"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = NativeSessionCorrelationV1{}
	switch discriminator.Visibility {
	case "observed":
		var variant NativeSessionCorrelationObservedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Observed = &variant
	case "unobserved":
		var variant NativeSessionCorrelationUnobservedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unobserved = &variant
	default:
		return fmt.Errorf("NativeSessionCorrelationV1: unknown discriminator %q", discriminator.Visibility)
	}
	return nil
}
