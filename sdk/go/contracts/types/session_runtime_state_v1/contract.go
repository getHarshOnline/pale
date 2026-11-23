// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package session_runtime_state_v1 contains the generated session-runtime-state.v1 data model.
package session_runtime_state_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type ActiveOperationLeaseV1 struct {
	// ActivationEpoch corresponds to the JSON schema field "activation_epoch".
	ActivationEpoch int `json:"activation_epoch"`

	// AuthorizedActions corresponds to the JSON schema field "authorized_actions".
	AuthorizedActions ActiveOperationLeaseV1AuthorizedActions `json:"authorized_actions,omitempty,omitzero"`

	// BindingDigest corresponds to the JSON schema field "binding_digest".
	BindingDigest sha256_digest_v1.Sha256DigestV1 `json:"binding_digest"`

	// CatalogDigest corresponds to the JSON schema field "catalog_digest".
	CatalogDigest sha256_digest_v1.Sha256DigestV1 `json:"catalog_digest"`

	// ContractDigest corresponds to the JSON schema field "contract_digest".
	ContractDigest sha256_digest_v1.Sha256DigestV1 `json:"contract_digest"`

	// Failure corresponds to the JSON schema field "failure".
	Failure *OperationFailureV1 `json:"failure,omitempty,omitzero"`

	// InflightTools corresponds to the JSON schema field "inflight_tools".
	InflightTools ActiveOperationLeaseV1InflightTools `json:"inflight_tools,omitempty,omitzero"`

	// OperationID corresponds to the JSON schema field "operation_id".
	OperationID PackageResourceRefV1 `json:"operation_id"`

	// TurnDigest corresponds to the JSON schema field "turn_digest".
	TurnDigest *sha256_digest_v1.Sha256DigestV1 `json:"turn_digest,omitempty,omitzero"`
}

type ActiveOperationLeaseV1AuthorizedActions map[string]sha256_digest_v1.Sha256DigestV1

type ActiveOperationLeaseV1InflightTools map[string]InflightToolAdmissionV1

type CapabilityRequirementIDV1 string

type FailurePosture string

const FailurePostureEscalate FailurePosture = "escalate"
const FailurePostureFailClosed FailurePosture = "fail_closed"
const FailurePostureFailOpen FailurePosture = "fail_open"
const FailurePostureRetry FailurePosture = "retry"
const FailurePostureSteer FailurePosture = "steer"

type InflightAdmissionStatusV1 string

const InflightAdmissionStatusV1Admitted InflightAdmissionStatusV1 = "admitted"
const InflightAdmissionStatusV1EvidencePending InflightAdmissionStatusV1 = "evidence_pending"

type InflightToolAdmissionV1 struct {
	// InvocationDigest corresponds to the JSON schema field "invocation_digest".
	InvocationDigest sha256_digest_v1.Sha256DigestV1 `json:"invocation_digest"`

	// RequirementID corresponds to the JSON schema field "requirement_id".
	RequirementID *CapabilityRequirementIDV1 `json:"requirement_id,omitempty,omitzero"`

	// Status corresponds to the JSON schema field "status".
	Status InflightAdmissionStatusV1 `json:"status"`
}

type OperationFailureV1 struct {
	// Posture corresponds to the JSON schema field "posture".
	Posture FailurePosture `json:"posture"`

	// RequirementID corresponds to the JSON schema field "requirement_id".
	RequirementID CapabilityRequirementIDV1 `json:"requirement_id"`
}

type PackageIDV1 string

type PackageResourceIDV1 string

type PackageResourceRefV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`
}

type RebaselineDebtV1 struct {
	// Epoch corresponds to the JSON schema field "epoch".
	Epoch int `json:"epoch"`
}

type SessionRuntimeStateV1 struct {
	// ActiveModes corresponds to the JSON schema field "active_modes".
	ActiveModes SessionRuntimeStateV1ActiveModes `json:"active_modes,omitempty,omitzero"`

	// ActiveOperation corresponds to the JSON schema field "active_operation".
	ActiveOperation *ActiveOperationLeaseV1 `json:"active_operation,omitempty,omitzero"`

	// Captures corresponds to the JSON schema field "captures".
	Captures []string `json:"captures,omitempty,omitzero"`

	// OperationEpoch corresponds to the JSON schema field "operation_epoch".
	OperationEpoch *int `json:"operation_epoch,omitempty,omitzero"`

	// RebaselineDebt corresponds to the JSON schema field "rebaseline_debt".
	RebaselineDebt *RebaselineDebtV1 `json:"rebaseline_debt,omitempty,omitzero"`

	// RebaselineEpoch corresponds to the JSON schema field "rebaseline_epoch".
	RebaselineEpoch *int `json:"rebaseline_epoch,omitempty,omitzero"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Touched corresponds to the JSON schema field "touched".
	Touched []string `json:"touched,omitempty,omitzero"`

	// VerificationDebt corresponds to the JSON schema field "verification_debt".
	VerificationDebt *VerificationDebtV1 `json:"verification_debt,omitempty,omitzero"`

	// VerificationEpoch corresponds to the JSON schema field "verification_epoch".
	VerificationEpoch *int `json:"verification_epoch,omitempty,omitzero"`
}

type SessionRuntimeStateV1ActiveModes map[string]PackageResourceRefV1

type VerificationDebtV1 struct {
	// CompletionPrompted corresponds to the JSON schema field "completion_prompted".
	CompletionPrompted bool `json:"completion_prompted,omitempty,omitzero"`

	// Epoch corresponds to the JSON schema field "epoch".
	Epoch int `json:"epoch"`

	// Requirement corresponds to the JSON schema field "requirement".
	Requirement VerificationRequirementV1 `json:"requirement"`
}

type VerificationRequirementV1 string

const VerificationRequirementV1ProtectedConfiguration VerificationRequirementV1 = "protected_configuration"
const VerificationRequirementV1Standard VerificationRequirementV1 = "standard"
