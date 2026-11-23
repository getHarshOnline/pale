// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package a2a_task_observation_v1 contains the generated a2a-task-observation.v1 data model.
package a2a_task_observation_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type A2ARemoteTaskStateV1 string

const A2ARemoteTaskStateV1AuthRequired A2ARemoteTaskStateV1 = "auth_required"
const A2ARemoteTaskStateV1Canceled A2ARemoteTaskStateV1 = "canceled"
const A2ARemoteTaskStateV1Completed A2ARemoteTaskStateV1 = "completed"
const A2ARemoteTaskStateV1Failed A2ARemoteTaskStateV1 = "failed"
const A2ARemoteTaskStateV1InputRequired A2ARemoteTaskStateV1 = "input_required"
const A2ARemoteTaskStateV1Rejected A2ARemoteTaskStateV1 = "rejected"
const A2ARemoteTaskStateV1Submitted A2ARemoteTaskStateV1 = "submitted"
const A2ARemoteTaskStateV1Unspecified A2ARemoteTaskStateV1 = "unspecified"
const A2ARemoteTaskStateV1Working A2ARemoteTaskStateV1 = "working"

type A2ASseEventIDV1 string

type A2ATaskBindingIDV1 string

type A2ATaskObservationRevisionV1 int

type A2ATaskObservationSourceCancelTaskReadBackV1 struct {
	// Request corresponds to the JSON schema field "request".
	Request ImmutableRecordReferenceV1 `json:"request"`

	// Response corresponds to the JSON schema field "response".
	Response ImmutableRecordReferenceV1 `json:"response"`

	// Source corresponds to the JSON schema field "source".
	Source string `json:"source"`
}

type A2ATaskObservationSourceGetTaskV1 struct {
	// Request corresponds to the JSON schema field "request".
	Request ImmutableRecordReferenceV1 `json:"request"`

	// Response corresponds to the JSON schema field "response".
	Response ImmutableRecordReferenceV1 `json:"response"`

	// Source corresponds to the JSON schema field "source".
	Source string `json:"source"`
}

type A2ATaskObservationSourceInitialSendMessageResponseV1 struct {
	// Evidence corresponds to the JSON schema field "evidence".
	Evidence ImmutableRecordReferenceV1 `json:"evidence"`

	// Source corresponds to the JSON schema field "source".
	Source string `json:"source"`
}

type A2ATaskObservationSourceInitialStreamingEventV1 struct {
	// EventID corresponds to the JSON schema field "event_id".
	EventID *A2ASseEventIDV1 `json:"event_id,omitempty,omitzero"`

	// Evidence corresponds to the JSON schema field "evidence".
	Evidence ImmutableRecordReferenceV1 `json:"evidence"`

	// Source corresponds to the JSON schema field "source".
	Source string `json:"source"`
}

type A2ATaskObservationSourceStreamingEventV1 struct {
	// EventID corresponds to the JSON schema field "event_id".
	EventID *A2ASseEventIDV1 `json:"event_id,omitempty,omitzero"`

	// Evidence corresponds to the JSON schema field "evidence".
	Evidence ImmutableRecordReferenceV1 `json:"evidence"`

	// Source corresponds to the JSON schema field "source".
	Source string `json:"source"`
}

type A2ATaskObservationV1 struct {
	// CanonicalA2ATaskObservationSha256 corresponds to the JSON schema field
	// "canonical_a2a_task_observation_sha256".
	CanonicalA2ATaskObservationSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_a2a_task_observation_sha256"`

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// ObservedAt corresponds to the JSON schema field "observed_at".
	ObservedAt UnixTimeV1 `json:"observed_at"`

	// PredecessorRef corresponds to the JSON schema field "predecessor_ref".
	PredecessorRef *ResidentRecordRefV1 `json:"predecessor_ref,omitempty,omitzero"`

	// PredecessorSha256 corresponds to the JSON schema field "predecessor_sha256".
	PredecessorSha256 *sha256_digest_v1.Sha256DigestV1 `json:"predecessor_sha256,omitempty,omitzero"`

	// Reconciliation corresponds to the JSON schema field "reconciliation".
	Reconciliation *ReconciliationFrontierReferenceV1 `json:"reconciliation,omitempty,omitzero"`

	// RemoteState corresponds to the JSON schema field "remote_state".
	RemoteState A2ARemoteTaskStateV1 `json:"remote_state"`

	// Revision corresponds to the JSON schema field "revision".
	Revision A2ATaskObservationRevisionV1 `json:"revision"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// Source corresponds to the JSON schema field "source".
	Source A2ATaskObservationSourceV1 `json:"source"`

	// TaskBindingID corresponds to the JSON schema field "task_binding_id".
	TaskBindingID A2ATaskBindingIDV1 `json:"task_binding_id"`

	// TaskBindingRef corresponds to the JSON schema field "task_binding_ref".
	TaskBindingRef ResidentRecordRefV1 `json:"task_binding_ref"`

	// TaskBindingSha256 corresponds to the JSON schema field "task_binding_sha256".
	TaskBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"task_binding_sha256"`
}

type CanonicalSignatureV1 string

type ImmutableRecordReferenceV1 struct {
	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSha256 corresponds to the JSON schema field "record_sha256".
	RecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_sha256"`
}

type ReconciliationFrontierReferenceV1 struct {
	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSha256 corresponds to the JSON schema field "record_sha256".
	RecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_sha256"`
}

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int

type A2ATaskObservationSourceV1 struct {
	InitialSendMessageResponse *A2ATaskObservationSourceInitialSendMessageResponseV1 `json:"-"`
	InitialStreamingEvent      *A2ATaskObservationSourceInitialStreamingEventV1      `json:"-"`
	StreamingEvent             *A2ATaskObservationSourceStreamingEventV1             `json:"-"`
	GetTask                    *A2ATaskObservationSourceGetTaskV1                    `json:"-"`
	CancelTaskReadBack         *A2ATaskObservationSourceCancelTaskReadBackV1         `json:"-"`
}

func (value A2ATaskObservationSourceV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.InitialSendMessageResponse != nil {
		if value.InitialSendMessageResponse.Source != "initial_send_message_response" {
			return nil, fmt.Errorf("A2ATaskObservationSourceV1: selected variant has discriminator %q", value.InitialSendMessageResponse.Source)
		}
		candidate, err := json.Marshal(value.InitialSendMessageResponse)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.InitialStreamingEvent != nil {
		if value.InitialStreamingEvent.Source != "initial_streaming_event" {
			return nil, fmt.Errorf("A2ATaskObservationSourceV1: selected variant has discriminator %q", value.InitialStreamingEvent.Source)
		}
		candidate, err := json.Marshal(value.InitialStreamingEvent)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.StreamingEvent != nil {
		if value.StreamingEvent.Source != "streaming_event" {
			return nil, fmt.Errorf("A2ATaskObservationSourceV1: selected variant has discriminator %q", value.StreamingEvent.Source)
		}
		candidate, err := json.Marshal(value.StreamingEvent)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.GetTask != nil {
		if value.GetTask.Source != "get_task" {
			return nil, fmt.Errorf("A2ATaskObservationSourceV1: selected variant has discriminator %q", value.GetTask.Source)
		}
		candidate, err := json.Marshal(value.GetTask)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.CancelTaskReadBack != nil {
		if value.CancelTaskReadBack.Source != "cancel_task_read_back" {
			return nil, fmt.Errorf("A2ATaskObservationSourceV1: selected variant has discriminator %q", value.CancelTaskReadBack.Source)
		}
		candidate, err := json.Marshal(value.CancelTaskReadBack)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("A2ATaskObservationSourceV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *A2ATaskObservationSourceV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Source string `json:"source"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = A2ATaskObservationSourceV1{}
	switch discriminator.Source {
	case "initial_send_message_response":
		var variant A2ATaskObservationSourceInitialSendMessageResponseV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.InitialSendMessageResponse = &variant
	case "initial_streaming_event":
		var variant A2ATaskObservationSourceInitialStreamingEventV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.InitialStreamingEvent = &variant
	case "streaming_event":
		var variant A2ATaskObservationSourceStreamingEventV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.StreamingEvent = &variant
	case "get_task":
		var variant A2ATaskObservationSourceGetTaskV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.GetTask = &variant
	case "cancel_task_read_back":
		var variant A2ATaskObservationSourceCancelTaskReadBackV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.CancelTaskReadBack = &variant
	default:
		return fmt.Errorf("A2ATaskObservationSourceV1: unknown discriminator %q", discriminator.Source)
	}
	return nil
}
