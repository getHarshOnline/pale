// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package event_acknowledgement_v1 contains the generated event-acknowledgement.v1 data model.
package event_acknowledgement_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CanonicalEventIdentityV1 struct {
	// EventID corresponds to the JSON schema field "event_id".
	EventID EventIDV1 `json:"event_id"`

	// OriginPrincipalID corresponds to the JSON schema field "origin_principal_id".
	OriginPrincipalID ResidentPrincipalIDV1 `json:"origin_principal_id"`

	// ProducerEpoch corresponds to the JSON schema field "producer_epoch".
	ProducerEpoch ProducerEpochV1 `json:"producer_epoch"`
}

type CanonicalSignatureV1 string

type ConsumerStateVersionV1 string

type DeliveryAttemptIDV1 string

type EventAcknowledgementDispositionV1 string

const EventAcknowledgementDispositionV1Accepted EventAcknowledgementDispositionV1 = "accepted"
const EventAcknowledgementDispositionV1Duplicate EventAcknowledgementDispositionV1 = "duplicate"
const EventAcknowledgementDispositionV1Quarantined EventAcknowledgementDispositionV1 = "quarantined"
const EventAcknowledgementDispositionV1Rejected EventAcknowledgementDispositionV1 = "rejected"

type EventAcknowledgementV1 struct {
	// AcknowledgedAt corresponds to the JSON schema field "acknowledged_at".
	AcknowledgedAt UnixTimeV1 `json:"acknowledged_at"`

	// AcknowledgingNodeDelegationRef corresponds to the JSON schema field
	// "acknowledging_node_delegation_ref".
	AcknowledgingNodeDelegationRef ResidentRecordRefV1 `json:"acknowledging_node_delegation_ref"`

	// AcknowledgingNodeDelegationSha256 corresponds to the JSON schema field
	// "acknowledging_node_delegation_sha256".
	AcknowledgingNodeDelegationSha256 sha256_digest_v1.Sha256DigestV1 `json:"acknowledging_node_delegation_sha256"`

	// AcknowledgingNodeID corresponds to the JSON schema field
	// "acknowledging_node_id".
	AcknowledgingNodeID ResidentNodeIDV1 `json:"acknowledging_node_id"`

	// AcknowledgingPrincipalID corresponds to the JSON schema field
	// "acknowledging_principal_id".
	AcknowledgingPrincipalID ResidentPrincipalIDV1 `json:"acknowledging_principal_id"`

	// AcknowledgingPrincipalKeyEpoch corresponds to the JSON schema field
	// "acknowledging_principal_key_epoch".
	AcknowledgingPrincipalKeyEpoch PrincipalKeyEpochV1 `json:"acknowledging_principal_key_epoch"`

	// CanonicalAcknowledgementSha256 corresponds to the JSON schema field
	// "canonical_acknowledgement_sha256".
	CanonicalAcknowledgementSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_acknowledgement_sha256"`

	// CanonicalEventIdentity corresponds to the JSON schema field
	// "canonical_event_identity".
	CanonicalEventIdentity CanonicalEventIdentityV1 `json:"canonical_event_identity"`

	// CanonicalEventSha256 corresponds to the JSON schema field
	// "canonical_event_sha256".
	CanonicalEventSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_event_sha256"`

	// ConsumerStateVersion corresponds to the JSON schema field
	// "consumer_state_version".
	ConsumerStateVersion *ConsumerStateVersionV1 `json:"consumer_state_version,omitempty,omitzero"`

	// DeliveryAttemptID corresponds to the JSON schema field "delivery_attempt_id".
	DeliveryAttemptID DeliveryAttemptIDV1 `json:"delivery_attempt_id"`

	// DestinationNodeID corresponds to the JSON schema field "destination_node_id".
	DestinationNodeID *ResidentNodeIDV1 `json:"destination_node_id,omitempty,omitzero"`

	// DestinationPrincipalID corresponds to the JSON schema field
	// "destination_principal_id".
	DestinationPrincipalID ResidentPrincipalIDV1 `json:"destination_principal_id"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition EventAcknowledgementDispositionV1 `json:"disposition"`

	// DuplicateOfAckRef corresponds to the JSON schema field "duplicate_of_ack_ref".
	DuplicateOfAckRef *ResidentRecordRefV1 `json:"duplicate_of_ack_ref,omitempty,omitzero"`

	// DuplicateOfAckSha256 corresponds to the JSON schema field
	// "duplicate_of_ack_sha256".
	DuplicateOfAckSha256 *sha256_digest_v1.Sha256DigestV1 `json:"duplicate_of_ack_sha256,omitempty,omitzero"`

	// QuarantineRef corresponds to the JSON schema field "quarantine_ref".
	QuarantineRef *ResidentRecordRefV1 `json:"quarantine_ref,omitempty,omitzero"`

	// QuarantineSha256 corresponds to the JSON schema field "quarantine_sha256".
	QuarantineSha256 *sha256_digest_v1.Sha256DigestV1 `json:"quarantine_sha256,omitempty,omitzero"`

	// ReasonCode corresponds to the JSON schema field "reason_code".
	ReasonCode *ResidentReasonV1 `json:"reason_code,omitempty,omitzero"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// SubscriptionID corresponds to the JSON schema field "subscription_id".
	SubscriptionID EventSubscriptionIDV1 `json:"subscription_id"`
}

type EventIDV1 string

type EventSubscriptionIDV1 string

type PrincipalKeyEpochV1 int

type ProducerEpochV1 string

type ResidentNodeIDV1 string

type ResidentPrincipalIDV1 string

type ResidentReasonV1 string

type ResidentRecordRefV1 string

type SchemaID string

type UnixTimeV1 int
