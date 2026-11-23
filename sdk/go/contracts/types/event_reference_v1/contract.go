// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package event_reference_v1 contains the generated event-reference.v1 data model.
package event_reference_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type DataClassificationV1 string

type DeploymentEnvironmentV1 string

type EventCausationIDV1 string

type EventCorrelationIDV1 string

type EventIDV1 string

type EventOrderingKeyV1 string

type EventReferenceV1 struct {
	// CanonicalEventSha256 corresponds to the JSON schema field
	// "canonical_event_sha256".
	CanonicalEventSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_event_sha256"`

	// CausationID corresponds to the JSON schema field "causation_id".
	CausationID *EventCausationIDV1 `json:"causation_id,omitempty,omitzero"`

	// Classification corresponds to the JSON schema field "classification".
	Classification DataClassificationV1 `json:"classification"`

	// CorrelationID corresponds to the JSON schema field "correlation_id".
	CorrelationID EventCorrelationIDV1 `json:"correlation_id"`

	// EventID corresponds to the JSON schema field "event_id".
	EventID EventIDV1 `json:"event_id"`

	// EventType corresponds to the JSON schema field "event_type".
	EventType EventTypeV1 `json:"event_type"`

	// EventVersion corresponds to the JSON schema field "event_version".
	EventVersion int `json:"event_version"`

	// ExpiresAt corresponds to the JSON schema field "expires_at".
	ExpiresAt *UnixTimeV1 `json:"expires_at,omitempty,omitzero"`

	// LifecycleNamespace corresponds to the JSON schema field "lifecycle_namespace".
	LifecycleNamespace LifecycleNamespaceV1 `json:"lifecycle_namespace"`

	// OccurredAt corresponds to the JSON schema field "occurred_at".
	OccurredAt UnixTimeV1 `json:"occurred_at"`

	// OrderingKey corresponds to the JSON schema field "ordering_key".
	OrderingKey *EventOrderingKeyV1 `json:"ordering_key,omitempty,omitzero"`

	// OriginNodeID corresponds to the JSON schema field "origin_node_id".
	OriginNodeID ResidentNodeIDV1 `json:"origin_node_id"`

	// OriginPrincipalID corresponds to the JSON schema field "origin_principal_id".
	OriginPrincipalID ResidentPrincipalIDV1 `json:"origin_principal_id"`

	// PayloadRef corresponds to the JSON schema field "payload_ref".
	PayloadRef *ResidentRecordRefV1 `json:"payload_ref,omitempty,omitzero"`

	// PayloadSha256 corresponds to the JSON schema field "payload_sha256".
	PayloadSha256 *sha256_digest_v1.Sha256DigestV1 `json:"payload_sha256,omitempty,omitzero"`

	// ProducerEpoch corresponds to the JSON schema field "producer_epoch".
	ProducerEpoch ProducerEpochV1 `json:"producer_epoch"`

	// RecordedAt corresponds to the JSON schema field "recorded_at".
	RecordedAt UnixTimeV1 `json:"recorded_at"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID string `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Sequence corresponds to the JSON schema field "sequence".
	Sequence *OwnerEventSequenceV1 `json:"sequence,omitempty,omitzero"`

	// SourceOwner corresponds to the JSON schema field "source_owner".
	SourceOwner SourceOwnerV1 `json:"source_owner"`

	// SourceRecordContentSha256 corresponds to the JSON schema field
	// "source_record_content_sha256".
	SourceRecordContentSha256 sha256_digest_v1.Sha256DigestV1 `json:"source_record_content_sha256"`

	// SourceRecordRef corresponds to the JSON schema field "source_record_ref".
	SourceRecordRef ResidentRecordRefV1 `json:"source_record_ref"`

	// SourceRecordSchemaID corresponds to the JSON schema field
	// "source_record_schema_id".
	SourceRecordSchemaID SchemaID `json:"source_record_schema_id"`

	// SourceRecordSchemaSha256 corresponds to the JSON schema field
	// "source_record_schema_sha256".
	SourceRecordSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"source_record_schema_sha256"`

	// SourceVersion corresponds to the JSON schema field "source_version".
	SourceVersion SourceVersionV1 `json:"source_version"`

	// TargetScope corresponds to the JSON schema field "target_scope".
	TargetScope EventScopeV1 `json:"target_scope"`
}

type EventScopeV1 struct {
	// ControllerScope corresponds to the JSON schema field "controller_scope".
	ControllerScope ResidentControllerIDV1 `json:"controller_scope"`

	// Environment corresponds to the JSON schema field "environment".
	Environment DeploymentEnvironmentV1 `json:"environment"`

	// PrincipalScope corresponds to the JSON schema field "principal_scope".
	PrincipalScope ResidentPrincipalIDV1 `json:"principal_scope"`

	// RelationshipScope corresponds to the JSON schema field "relationship_scope".
	RelationshipScope *RelationshipScopeV1 `json:"relationship_scope,omitempty,omitzero"`
}

type EventTypeV1 string

type LifecycleNamespaceV1 string

type OwnerEventSequenceV1 int

type ProducerEpochV1 string

type RelationshipScopeV1 string

type ResidentControllerIDV1 string

type ResidentNodeIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SourceOwnerV1 string

type SourceVersionV1 string

type UnixTimeV1 int
