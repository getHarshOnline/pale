// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package operation_lease_v1 contains the generated operation-lease.v1 data model.
package operation_lease_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type CapabilityIDV1 string

type EffectClass string

const EffectClassCompute EffectClass = "compute"
const EffectClassExecute EffectClass = "execute"
const EffectClassNetwork EffectClass = "network"
const EffectClassPublish EffectClass = "publish"
const EffectClassRead EffectClass = "read"
const EffectClassWrite EffectClass = "write"

type InvocationCorrelationIDV1 string

type InvocationCorrelationObservedV1 struct {
	// InvocationCorrelationID corresponds to the JSON schema field
	// "invocation_correlation_id".
	InvocationCorrelationID InvocationCorrelationIDV1 `json:"invocation_correlation_id"`

	// NativeInvocationID corresponds to the JSON schema field "native_invocation_id".
	NativeInvocationID NativeInvocationIDV1 `json:"native_invocation_id"`

	// Visibility corresponds to the JSON schema field "visibility".
	Visibility string `json:"visibility"`
}

type InvocationCorrelationUnobservedV1 struct {
	// Reason corresponds to the JSON schema field "reason".
	Reason ResidentReasonV1 `json:"reason"`

	// Visibility corresponds to the JSON schema field "visibility".
	Visibility string `json:"visibility"`
}

type LeaseBudgetUnitsV1 int

type LeaseTimeoutMillisV1 int

type NativeInvocationIDV1 string

type OperationLeaseIDV1 string

type OperationLeaseV1 struct {
	// BudgetUnits corresponds to the JSON schema field "budget_units".
	BudgetUnits LeaseBudgetUnitsV1 `json:"budget_units"`

	// Capabilities corresponds to the JSON schema field "capabilities".
	Capabilities []CapabilityIDV1 `json:"capabilities"`

	// Effects corresponds to the JSON schema field "effects".
	Effects []EffectClass `json:"effects"`

	// EvidenceObligations corresponds to the JSON schema field
	// "evidence_obligations".
	EvidenceObligations []ResidentRecordRefV1 `json:"evidence_obligations"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt UnixTimeV1 `json:"expires_at"`

	// InvocationCorrelation corresponds to the JSON schema field
	// "invocation_correlation".
	InvocationCorrelation InvocationCorrelationV1 `json:"invocation_correlation"`

	// Operation corresponds to the JSON schema field "operation".
	Operation PackageResourceRefV1 `json:"operation"`

	// OperationLeaseDigest corresponds to the JSON schema field
	// "operation_lease_digest".
	OperationLeaseDigest sha256_digest_v1.Sha256DigestV1 `json:"operation_lease_digest"`

	// OperationLeaseID corresponds to the JSON schema field "operation_lease_id".
	OperationLeaseID OperationLeaseIDV1 `json:"operation_lease_id"`

	// ReadSet corresponds to the JSON schema field "read_set".
	ReadSet []ResidentRecordRefV1 `json:"read_set"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// SessionAdmissionID corresponds to the JSON schema field "session_admission_id".
	SessionAdmissionID SessionAdmissionIDV1 `json:"session_admission_id"`

	// SessionAdmissionLeaseDigest corresponds to the JSON schema field
	// "session_admission_lease_digest".
	SessionAdmissionLeaseDigest sha256_digest_v1.Sha256DigestV1 `json:"session_admission_lease_digest"`

	// TimeoutMillis corresponds to the JSON schema field "timeout_millis".
	TimeoutMillis LeaseTimeoutMillisV1 `json:"timeout_millis"`
}

type PackageIDV1 string

type PackageResourceIDV1 string

type PackageResourceRefV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`
}

type ResidentReasonV1 string

type ResidentRecordRefV1 string

type SessionAdmissionIDV1 string

type UnixTimeV1 int

type InvocationCorrelationV1 struct {
	Observed   *InvocationCorrelationObservedV1   `json:"-"`
	Unobserved *InvocationCorrelationUnobservedV1 `json:"-"`
}

func (value InvocationCorrelationV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Observed != nil {
		if value.Observed.Visibility != "observed" {
			return nil, fmt.Errorf("InvocationCorrelationV1: selected variant has discriminator %q", value.Observed.Visibility)
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
			return nil, fmt.Errorf("InvocationCorrelationV1: selected variant has discriminator %q", value.Unobserved.Visibility)
		}
		candidate, err := json.Marshal(value.Unobserved)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("InvocationCorrelationV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *InvocationCorrelationV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Visibility string `json:"visibility"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = InvocationCorrelationV1{}
	switch discriminator.Visibility {
	case "observed":
		var variant InvocationCorrelationObservedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Observed = &variant
	case "unobserved":
		var variant InvocationCorrelationUnobservedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unobserved = &variant
	default:
		return fmt.Errorf("InvocationCorrelationV1: unknown discriminator %q", discriminator.Visibility)
	}
	return nil
}
