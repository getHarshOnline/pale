// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package decision_evidence_ledger_v1 contains the generated decision-evidence-ledger.v1 data model.
package decision_evidence_ledger_v1

import (
	"encoding/json"
	"fmt"
	lifecycle_event_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/lifecycle_event_v1"
)
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type DecisionActivityObservedV1 struct {
	// Activity corresponds to the JSON schema field "activity".
	Activity HarnessActivityIdentity `json:"activity"`

	// Visibility corresponds to the JSON schema field "visibility".
	Visibility string `json:"visibility"`
}

type DecisionActivityUnobservedV1 struct {
	// Reason corresponds to the JSON schema field "reason".
	Reason ResidentReasonV1 `json:"reason"`

	// Visibility corresponds to the JSON schema field "visibility".
	Visibility string `json:"visibility"`
}

type DecisionDispositionV1 string

const DecisionDispositionV1CompletionSteer DecisionDispositionV1 = "completion_steer"
const DecisionDispositionV1Deny DecisionDispositionV1 = "deny"
const DecisionDispositionV1Notification DecisionDispositionV1 = "notification"
const DecisionDispositionV1PostEffectSteer DecisionDispositionV1 = "post_effect_steer"
const DecisionDispositionV1RequestConfirmation DecisionDispositionV1 = "request_confirmation"

type DecisionEmissionOutcomeV1 string

const DecisionEmissionOutcomeV1Emitted DecisionEmissionOutcomeV1 = "emitted"
const DecisionEmissionOutcomeV1Failed DecisionEmissionOutcomeV1 = "failed"
const DecisionEmissionOutcomeV1Unknown DecisionEmissionOutcomeV1 = "unknown"

type DecisionEvidenceEntryV1 struct {
	// Disposition corresponds to the JSON schema field "disposition".
	Disposition DecisionDispositionV1 `json:"disposition"`

	// Event corresponds to the JSON schema field "event".
	Event lifecycle_event_v1.LifecycleEventV1 `json:"event"`

	// EvidenceDigest corresponds to the JSON schema field "evidence_digest".
	EvidenceDigest sha256_digest_v1.Sha256DigestV1 `json:"evidence_digest"`

	// Occurrence corresponds to the JSON schema field "occurrence".
	Occurrence DecisionOccurrenceV1 `json:"occurrence"`

	// Operation corresponds to the JSON schema field "operation".
	Operation DecisionOperationBindingV1 `json:"operation"`

	// PreviousDigest corresponds to the JSON schema field "previous_digest".
	PreviousDigest *sha256_digest_v1.Sha256DigestV1 `json:"previous_digest,omitempty,omitzero"`

	// Reason corresponds to the JSON schema field "reason".
	Reason string `json:"reason"`

	// ReasonSha256 corresponds to the JSON schema field "reason_sha256".
	ReasonSha256 sha256_digest_v1.Sha256DigestV1 `json:"reason_sha256"`

	// Sequence corresponds to the JSON schema field "sequence".
	Sequence int `json:"sequence"`

	// Stage corresponds to the JSON schema field "stage".
	Stage DecisionEvidenceStageV1 `json:"stage"`

	// Target corresponds to the JSON schema field "target".
	Target DecisionTargetV1 `json:"target"`

	// Tool corresponds to the JSON schema field "tool".
	Tool DecisionEvidenceEntryV1Tool `json:"tool,omitempty,omitzero"`
}

type DecisionEvidenceEntryV1Tool *string

type DecisionEvidenceLedgerV1 struct {
	// Activity corresponds to the JSON schema field "activity".
	Activity DecisionActivityV1 `json:"activity"`

	// CompilationInputDigest corresponds to the JSON schema field
	// "compilation_input_digest".
	CompilationInputDigest sha256_digest_v1.Sha256DigestV1 `json:"compilation_input_digest"`

	// Entries corresponds to the JSON schema field "entries".
	Entries []DecisionEvidenceEntryV1 `json:"entries"`

	// ExecutionPath corresponds to the JSON schema field "execution_path".
	ExecutionPath ExecutionPathSelectionV1 `json:"execution_path"`

	// NextSequence corresponds to the JSON schema field "next_sequence".
	NextSequence int `json:"next_sequence"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// TruncationAnchor corresponds to the JSON schema field "truncation_anchor".
	TruncationAnchor *DecisionEvidenceTruncationAnchorV1 `json:"truncation_anchor,omitempty,omitzero"`
}

type DecisionEvidenceStageHostAcknowledgedV1 struct {
	// EmissionEntrySha256 corresponds to the JSON schema field
	// "emission_entry_sha256".
	EmissionEntrySha256 sha256_digest_v1.Sha256DigestV1 `json:"emission_entry_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Outcome corresponds to the JSON schema field "outcome".
	Outcome DecisionHostOutcomeV1 `json:"outcome"`

	// Witness corresponds to the JSON schema field "witness".
	Witness ImmutableRecordReferenceV1 `json:"witness"`
}

type DecisionEvidenceStagePolicySelectedV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type DecisionEvidenceStageResponseEmissionV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Outcome corresponds to the JSON schema field "outcome".
	Outcome DecisionEmissionOutcomeV1 `json:"outcome"`

	// ResponseSha256 corresponds to the JSON schema field "response_sha256".
	ResponseSha256 sha256_digest_v1.Sha256DigestV1 `json:"response_sha256"`

	// SelectedEntrySha256 corresponds to the JSON schema field
	// "selected_entry_sha256".
	SelectedEntrySha256 sha256_digest_v1.Sha256DigestV1 `json:"selected_entry_sha256"`
}

type DecisionEvidenceTruncationAnchorV1 struct {
	// EvidenceDigest corresponds to the JSON schema field "evidence_digest".
	EvidenceDigest sha256_digest_v1.Sha256DigestV1 `json:"evidence_digest"`

	// Sequence corresponds to the JSON schema field "sequence".
	Sequence int `json:"sequence"`
}

type DecisionHostOutcomeV1 string

const DecisionHostOutcomeV1ConfirmationPending DecisionHostOutcomeV1 = "confirmation_pending"
const DecisionHostOutcomeV1Notified DecisionHostOutcomeV1 = "notified"
const DecisionHostOutcomeV1Prevented DecisionHostOutcomeV1 = "prevented"
const DecisionHostOutcomeV1Steered DecisionHostOutcomeV1 = "steered"

type DecisionNativeCoordinateNotApplicableV1 struct {
	// Visibility corresponds to the JSON schema field "visibility".
	Visibility string `json:"visibility"`
}

type DecisionNativeCoordinateObservedV1 struct {
	// Sha256 corresponds to the JSON schema field "sha256".
	Sha256 sha256_digest_v1.Sha256DigestV1 `json:"sha256"`

	// Visibility corresponds to the JSON schema field "visibility".
	Visibility string `json:"visibility"`
}

type DecisionNativeCoordinateUnobservedV1 struct {
	// Reason corresponds to the JSON schema field "reason".
	Reason ResidentReasonV1 `json:"reason"`

	// Visibility corresponds to the JSON schema field "visibility".
	Visibility string `json:"visibility"`
}

type DecisionOccurrenceV1 struct {
	// NativeEventSha256 corresponds to the JSON schema field "native_event_sha256".
	NativeEventSha256 sha256_digest_v1.Sha256DigestV1 `json:"native_event_sha256"`

	// ProviderBinding corresponds to the JSON schema field "provider_binding".
	ProviderBinding ProviderBindingRefV1 `json:"provider_binding"`

	// ToolUse corresponds to the JSON schema field "tool_use".
	ToolUse DecisionNativeCoordinateV1 `json:"tool_use"`

	// Turn corresponds to the JSON schema field "turn".
	Turn DecisionNativeCoordinateV1 `json:"turn"`
}

type DecisionOperationBindingRegisteredV1 struct {
	// HandleRef corresponds to the JSON schema field "handle_ref".
	HandleRef ResidentRecordRefV1 `json:"handle_ref"`

	// HandleSha256 corresponds to the JSON schema field "handle_sha256".
	HandleSha256 sha256_digest_v1.Sha256DigestV1 `json:"handle_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// LeaseRef corresponds to the JSON schema field "lease_ref".
	LeaseRef ResidentRecordRefV1 `json:"lease_ref"`

	// LeaseSha256 corresponds to the JSON schema field "lease_sha256".
	LeaseSha256 sha256_digest_v1.Sha256DigestV1 `json:"lease_sha256"`
}

type DecisionOperationBindingUnboundV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type DecisionOperationBindingUnresolvedV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResidentReasonV1 `json:"reason"`
}

type DecisionTargetCommandV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Sha256 corresponds to the JSON schema field "sha256".
	Sha256 sha256_digest_v1.Sha256DigestV1 `json:"sha256"`
}

type DecisionTargetFileV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Path corresponds to the JSON schema field "path".
	Path string `json:"path"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`

	// Sha256 corresponds to the JSON schema field "sha256".
	Sha256 sha256_digest_v1.Sha256DigestV1 `json:"sha256"`
}

type DecisionTargetNotApplicableV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type DecisionTargetStructuredInputV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Sha256 corresponds to the JSON schema field "sha256".
	Sha256 sha256_digest_v1.Sha256DigestV1 `json:"sha256"`
}

type DecisionTargetUnobservedV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResidentReasonV1 `json:"reason"`
}

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

type ImmutableRecordReferenceV1 struct {
	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSha256 corresponds to the JSON schema field "record_sha256".
	RecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_sha256"`
}

type LogicalRepositoryIDV1 string

type ProviderBindingIdentityV1 struct {
	// ProviderBindingSha256 corresponds to the JSON schema field
	// "provider_binding_sha256".
	ProviderBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"provider_binding_sha256"`
}

type ProviderBindingRefV1 struct {
	// ProviderBindingID corresponds to the JSON schema field "provider_binding_id".
	ProviderBindingID string `json:"provider_binding_id"`

	// ProviderBindingIdentity corresponds to the JSON schema field
	// "provider_binding_identity".
	ProviderBindingIdentity ProviderBindingIdentityV1 `json:"provider_binding_identity"`
}

type ResidentReasonV1 string

type ResidentRecordRefV1 string

type DecisionActivityV1 struct {
	Observed   *DecisionActivityObservedV1   `json:"-"`
	Unobserved *DecisionActivityUnobservedV1 `json:"-"`
}

func (value DecisionActivityV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Observed != nil {
		if value.Observed.Visibility != "observed" {
			return nil, fmt.Errorf("DecisionActivityV1: selected variant has discriminator %q", value.Observed.Visibility)
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
			return nil, fmt.Errorf("DecisionActivityV1: selected variant has discriminator %q", value.Unobserved.Visibility)
		}
		candidate, err := json.Marshal(value.Unobserved)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("DecisionActivityV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *DecisionActivityV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Visibility string `json:"visibility"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = DecisionActivityV1{}
	switch discriminator.Visibility {
	case "observed":
		var variant DecisionActivityObservedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Observed = &variant
	case "unobserved":
		var variant DecisionActivityUnobservedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unobserved = &variant
	default:
		return fmt.Errorf("DecisionActivityV1: unknown discriminator %q", discriminator.Visibility)
	}
	return nil
}

type DecisionEvidenceStageV1 struct {
	PolicySelected   *DecisionEvidenceStagePolicySelectedV1   `json:"-"`
	ResponseEmission *DecisionEvidenceStageResponseEmissionV1 `json:"-"`
	HostAcknowledged *DecisionEvidenceStageHostAcknowledgedV1 `json:"-"`
}

func (value DecisionEvidenceStageV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.PolicySelected != nil {
		if value.PolicySelected.Kind != "policy_selected" {
			return nil, fmt.Errorf("DecisionEvidenceStageV1: selected variant has discriminator %q", value.PolicySelected.Kind)
		}
		candidate, err := json.Marshal(value.PolicySelected)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ResponseEmission != nil {
		if value.ResponseEmission.Kind != "response_emission" {
			return nil, fmt.Errorf("DecisionEvidenceStageV1: selected variant has discriminator %q", value.ResponseEmission.Kind)
		}
		candidate, err := json.Marshal(value.ResponseEmission)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.HostAcknowledged != nil {
		if value.HostAcknowledged.Kind != "host_acknowledged" {
			return nil, fmt.Errorf("DecisionEvidenceStageV1: selected variant has discriminator %q", value.HostAcknowledged.Kind)
		}
		candidate, err := json.Marshal(value.HostAcknowledged)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("DecisionEvidenceStageV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *DecisionEvidenceStageV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = DecisionEvidenceStageV1{}
	switch discriminator.Kind {
	case "policy_selected":
		var variant DecisionEvidenceStagePolicySelectedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.PolicySelected = &variant
	case "response_emission":
		var variant DecisionEvidenceStageResponseEmissionV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ResponseEmission = &variant
	case "host_acknowledged":
		var variant DecisionEvidenceStageHostAcknowledgedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.HostAcknowledged = &variant
	default:
		return fmt.Errorf("DecisionEvidenceStageV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type DecisionNativeCoordinateV1 struct {
	Observed      *DecisionNativeCoordinateObservedV1      `json:"-"`
	Unobserved    *DecisionNativeCoordinateUnobservedV1    `json:"-"`
	NotApplicable *DecisionNativeCoordinateNotApplicableV1 `json:"-"`
}

func (value DecisionNativeCoordinateV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Observed != nil {
		if value.Observed.Visibility != "observed" {
			return nil, fmt.Errorf("DecisionNativeCoordinateV1: selected variant has discriminator %q", value.Observed.Visibility)
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
			return nil, fmt.Errorf("DecisionNativeCoordinateV1: selected variant has discriminator %q", value.Unobserved.Visibility)
		}
		candidate, err := json.Marshal(value.Unobserved)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.NotApplicable != nil {
		if value.NotApplicable.Visibility != "not_applicable" {
			return nil, fmt.Errorf("DecisionNativeCoordinateV1: selected variant has discriminator %q", value.NotApplicable.Visibility)
		}
		candidate, err := json.Marshal(value.NotApplicable)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("DecisionNativeCoordinateV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *DecisionNativeCoordinateV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Visibility string `json:"visibility"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = DecisionNativeCoordinateV1{}
	switch discriminator.Visibility {
	case "observed":
		var variant DecisionNativeCoordinateObservedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Observed = &variant
	case "unobserved":
		var variant DecisionNativeCoordinateUnobservedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unobserved = &variant
	case "not_applicable":
		var variant DecisionNativeCoordinateNotApplicableV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NotApplicable = &variant
	default:
		return fmt.Errorf("DecisionNativeCoordinateV1: unknown discriminator %q", discriminator.Visibility)
	}
	return nil
}

type DecisionOperationBindingV1 struct {
	Registered *DecisionOperationBindingRegisteredV1 `json:"-"`
	Unbound    *DecisionOperationBindingUnboundV1    `json:"-"`
	Unresolved *DecisionOperationBindingUnresolvedV1 `json:"-"`
}

func (value DecisionOperationBindingV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Registered != nil {
		if value.Registered.Kind != "registered" {
			return nil, fmt.Errorf("DecisionOperationBindingV1: selected variant has discriminator %q", value.Registered.Kind)
		}
		candidate, err := json.Marshal(value.Registered)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Unbound != nil {
		if value.Unbound.Kind != "unbound" {
			return nil, fmt.Errorf("DecisionOperationBindingV1: selected variant has discriminator %q", value.Unbound.Kind)
		}
		candidate, err := json.Marshal(value.Unbound)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Unresolved != nil {
		if value.Unresolved.Kind != "unresolved" {
			return nil, fmt.Errorf("DecisionOperationBindingV1: selected variant has discriminator %q", value.Unresolved.Kind)
		}
		candidate, err := json.Marshal(value.Unresolved)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("DecisionOperationBindingV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *DecisionOperationBindingV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = DecisionOperationBindingV1{}
	switch discriminator.Kind {
	case "registered":
		var variant DecisionOperationBindingRegisteredV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Registered = &variant
	case "unbound":
		var variant DecisionOperationBindingUnboundV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unbound = &variant
	case "unresolved":
		var variant DecisionOperationBindingUnresolvedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unresolved = &variant
	default:
		return fmt.Errorf("DecisionOperationBindingV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type DecisionTargetV1 struct {
	File            *DecisionTargetFileV1            `json:"-"`
	Command         *DecisionTargetCommandV1         `json:"-"`
	StructuredInput *DecisionTargetStructuredInputV1 `json:"-"`
	Unobserved      *DecisionTargetUnobservedV1      `json:"-"`
	NotApplicable   *DecisionTargetNotApplicableV1   `json:"-"`
}

func (value DecisionTargetV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.File != nil {
		if value.File.Kind != "file" {
			return nil, fmt.Errorf("DecisionTargetV1: selected variant has discriminator %q", value.File.Kind)
		}
		candidate, err := json.Marshal(value.File)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Command != nil {
		if value.Command.Kind != "command" {
			return nil, fmt.Errorf("DecisionTargetV1: selected variant has discriminator %q", value.Command.Kind)
		}
		candidate, err := json.Marshal(value.Command)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.StructuredInput != nil {
		if value.StructuredInput.Kind != "structured_input" {
			return nil, fmt.Errorf("DecisionTargetV1: selected variant has discriminator %q", value.StructuredInput.Kind)
		}
		candidate, err := json.Marshal(value.StructuredInput)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Unobserved != nil {
		if value.Unobserved.Kind != "unobserved" {
			return nil, fmt.Errorf("DecisionTargetV1: selected variant has discriminator %q", value.Unobserved.Kind)
		}
		candidate, err := json.Marshal(value.Unobserved)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.NotApplicable != nil {
		if value.NotApplicable.Kind != "not_applicable" {
			return nil, fmt.Errorf("DecisionTargetV1: selected variant has discriminator %q", value.NotApplicable.Kind)
		}
		candidate, err := json.Marshal(value.NotApplicable)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("DecisionTargetV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *DecisionTargetV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = DecisionTargetV1{}
	switch discriminator.Kind {
	case "file":
		var variant DecisionTargetFileV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.File = &variant
	case "command":
		var variant DecisionTargetCommandV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Command = &variant
	case "structured_input":
		var variant DecisionTargetStructuredInputV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.StructuredInput = &variant
	case "unobserved":
		var variant DecisionTargetUnobservedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unobserved = &variant
	case "not_applicable":
		var variant DecisionTargetNotApplicableV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NotApplicable = &variant
	default:
		return fmt.Errorf("DecisionTargetV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

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
