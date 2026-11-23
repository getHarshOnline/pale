// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package reconciliation_frontier_v1 contains the generated reconciliation-frontier.v1 data model.
package reconciliation_frontier_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type AuthorityEvidenceV1 struct {
	// AuthorityRef corresponds to the JSON schema field "authority_ref".
	AuthorityRef ResidentRecordRefV1 `json:"authority_ref"`

	// AuthoritySha256 corresponds to the JSON schema field "authority_sha256".
	AuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"authority_sha256"`
}

type CanonicalSignatureV1 string

type FrontierOwnerRevisionV1 struct {
	// NativeRevision corresponds to the JSON schema field "native_revision".
	NativeRevision NativeRevisionV1 `json:"native_revision"`

	// OwnerSystemID corresponds to the JSON schema field "owner_system_id".
	OwnerSystemID OwnerSystemIDV1 `json:"owner_system_id"`

	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSha256 corresponds to the JSON schema field "record_sha256".
	RecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_sha256"`
}

type ImmutableRecordReferenceV1 struct {
	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSha256 corresponds to the JSON schema field "record_sha256".
	RecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_sha256"`
}

type NativeRevisionOwnerV1 string

type NativeRevisionTokenV1 string

type NativeRevisionV1 struct {
	// NativeRevisionOwner corresponds to the JSON schema field
	// "native_revision_owner".
	NativeRevisionOwner NativeRevisionOwnerV1 `json:"native_revision_owner"`

	// NativeRevisionToken corresponds to the JSON schema field
	// "native_revision_token".
	NativeRevisionToken NativeRevisionTokenV1 `json:"native_revision_token"`
}

type OwnerFrontierDispositionV1 string

type OwnerRecordCoordinateV1 struct {
	// NativeRevision corresponds to the JSON schema field "native_revision".
	NativeRevision NativeRevisionV1 `json:"native_revision"`

	// OwnerSystemID corresponds to the JSON schema field "owner_system_id".
	OwnerSystemID OwnerSystemIDV1 `json:"owner_system_id"`

	// RecordContentSha256 corresponds to the JSON schema field
	// "record_content_sha256".
	RecordContentSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_content_sha256"`

	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSchemaID corresponds to the JSON schema field "record_schema_id".
	RecordSchemaID SchemaID `json:"record_schema_id"`

	// RecordSchemaSha256 corresponds to the JSON schema field "record_schema_sha256".
	RecordSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_schema_sha256"`
}

type OwnerSystemIDV1 string

type ReconciliationDimensionEntryV1 struct {
	// Dimension corresponds to the JSON schema field "dimension".
	Dimension ReconciliationDimensionV1 `json:"dimension"`

	// Evidence corresponds to the JSON schema field "evidence".
	Evidence *AuthorityEvidenceV1 `json:"evidence,omitempty,omitzero"`

	// Status corresponds to the JSON schema field "status".
	Status ReconciliationDimensionStatusV1 `json:"status"`
}

type ReconciliationDimensionStatusBlockedV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type ReconciliationDimensionStatusCompleteV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type ReconciliationDimensionStatusExpiredV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type ReconciliationDimensionStatusGapV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type ReconciliationDimensionStatusOwnerTerminalV1 struct {
	// Disposition corresponds to the JSON schema field "disposition".
	Disposition OwnerFrontierDispositionV1 `json:"disposition"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type ReconciliationDimensionStatusPendingV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type ReconciliationDimensionStatusQuarantinedV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type ReconciliationDimensionStatusUnavailableV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type ReconciliationDimensionV1 string

const ReconciliationDimensionV1Articulation ReconciliationDimensionV1 = "articulation"
const ReconciliationDimensionV1Delivery ReconciliationDimensionV1 = "delivery"
const ReconciliationDimensionV1DeterministicProjection ReconciliationDimensionV1 = "deterministic_projection"
const ReconciliationDimensionV1Judgment ReconciliationDimensionV1 = "judgment"
const ReconciliationDimensionV1NativeConsequence ReconciliationDimensionV1 = "native_consequence"
const ReconciliationDimensionV1OwnerPublication ReconciliationDimensionV1 = "owner_publication"
const ReconciliationDimensionV1PaleIngress ReconciliationDimensionV1 = "pale_ingress"
const ReconciliationDimensionV1Retention ReconciliationDimensionV1 = "retention"
const ReconciliationDimensionV1SemanticMapping ReconciliationDimensionV1 = "semantic_mapping"

type ReconciliationFrontierIDV1 string

type ReconciliationFrontierV1 struct {
	// CanonicalReconciliationFrontierSha256 corresponds to the JSON schema field
	// "canonical_reconciliation_frontier_sha256".
	CanonicalReconciliationFrontierSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_reconciliation_frontier_sha256"`

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`

	// Dimensions corresponds to the JSON schema field "dimensions".
	Dimensions []ReconciliationDimensionEntryV1 `json:"dimensions"`

	// FrontierID corresponds to the JSON schema field "frontier_id".
	FrontierID ReconciliationFrontierIDV1 `json:"frontier_id"`

	// GeneratedAt corresponds to the JSON schema field "generated_at".
	GeneratedAt UnixTimeV1 `json:"generated_at"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// OwnerRecordRevisions corresponds to the JSON schema field
	// "owner_record_revisions".
	OwnerRecordRevisions []FrontierOwnerRevisionV1 `json:"owner_record_revisions"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// SourceOccurrenceRefs corresponds to the JSON schema field
	// "source_occurrence_refs".
	SourceOccurrenceRefs []ImmutableRecordReferenceV1 `json:"source_occurrence_refs"`

	// Subject corresponds to the JSON schema field "subject".
	Subject ReconciliationSubjectV1 `json:"subject"`
}

type ReconciliationSubjectConsequenceV1 struct {
	// ConsequenceRef corresponds to the JSON schema field "consequence_ref".
	ConsequenceRef ResidentRecordRefV1 `json:"consequence_ref"`

	// ConsequenceSha256 corresponds to the JSON schema field "consequence_sha256".
	ConsequenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"consequence_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// OwnerSystemID corresponds to the JSON schema field "owner_system_id".
	OwnerSystemID OwnerSystemIDV1 `json:"owner_system_id"`
}

type ReconciliationSubjectConsumerV1 struct {
	// ConsumerID corresponds to the JSON schema field "consumer_id".
	ConsumerID OwnerSystemIDV1 `json:"consumer_id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// StateRef corresponds to the JSON schema field "state_ref".
	StateRef ResidentRecordRefV1 `json:"state_ref"`

	// StateSha256 corresponds to the JSON schema field "state_sha256".
	StateSha256 sha256_digest_v1.Sha256DigestV1 `json:"state_sha256"`
}

type ReconciliationSubjectProjectionV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ProjectionRef corresponds to the JSON schema field "projection_ref".
	ProjectionRef ResidentRecordRefV1 `json:"projection_ref"`

	// ProjectionSha256 corresponds to the JSON schema field "projection_sha256".
	ProjectionSha256 sha256_digest_v1.Sha256DigestV1 `json:"projection_sha256"`
}

type ReconciliationSubjectSourceV1 struct {
	// Coordinate corresponds to the JSON schema field "coordinate".
	Coordinate OwnerRecordCoordinateV1 `json:"coordinate"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int

type ReconciliationDimensionStatusV1 struct {
	Complete      *ReconciliationDimensionStatusCompleteV1      `json:"-"`
	Pending       *ReconciliationDimensionStatusPendingV1       `json:"-"`
	Gap           *ReconciliationDimensionStatusGapV1           `json:"-"`
	Blocked       *ReconciliationDimensionStatusBlockedV1       `json:"-"`
	Quarantined   *ReconciliationDimensionStatusQuarantinedV1   `json:"-"`
	Unavailable   *ReconciliationDimensionStatusUnavailableV1   `json:"-"`
	Expired       *ReconciliationDimensionStatusExpiredV1       `json:"-"`
	OwnerTerminal *ReconciliationDimensionStatusOwnerTerminalV1 `json:"-"`
}

func (value ReconciliationDimensionStatusV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Complete != nil {
		if value.Complete.State != "complete" {
			return nil, fmt.Errorf("ReconciliationDimensionStatusV1: selected variant has discriminator %q", value.Complete.State)
		}
		candidate, err := json.Marshal(value.Complete)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Pending != nil {
		if value.Pending.State != "pending" {
			return nil, fmt.Errorf("ReconciliationDimensionStatusV1: selected variant has discriminator %q", value.Pending.State)
		}
		candidate, err := json.Marshal(value.Pending)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Gap != nil {
		if value.Gap.State != "gap" {
			return nil, fmt.Errorf("ReconciliationDimensionStatusV1: selected variant has discriminator %q", value.Gap.State)
		}
		candidate, err := json.Marshal(value.Gap)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Blocked != nil {
		if value.Blocked.State != "blocked" {
			return nil, fmt.Errorf("ReconciliationDimensionStatusV1: selected variant has discriminator %q", value.Blocked.State)
		}
		candidate, err := json.Marshal(value.Blocked)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Quarantined != nil {
		if value.Quarantined.State != "quarantined" {
			return nil, fmt.Errorf("ReconciliationDimensionStatusV1: selected variant has discriminator %q", value.Quarantined.State)
		}
		candidate, err := json.Marshal(value.Quarantined)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Unavailable != nil {
		if value.Unavailable.State != "unavailable" {
			return nil, fmt.Errorf("ReconciliationDimensionStatusV1: selected variant has discriminator %q", value.Unavailable.State)
		}
		candidate, err := json.Marshal(value.Unavailable)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Expired != nil {
		if value.Expired.State != "expired" {
			return nil, fmt.Errorf("ReconciliationDimensionStatusV1: selected variant has discriminator %q", value.Expired.State)
		}
		candidate, err := json.Marshal(value.Expired)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.OwnerTerminal != nil {
		if value.OwnerTerminal.State != "owner_terminal" {
			return nil, fmt.Errorf("ReconciliationDimensionStatusV1: selected variant has discriminator %q", value.OwnerTerminal.State)
		}
		candidate, err := json.Marshal(value.OwnerTerminal)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ReconciliationDimensionStatusV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ReconciliationDimensionStatusV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		State string `json:"state"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ReconciliationDimensionStatusV1{}
	switch discriminator.State {
	case "complete":
		var variant ReconciliationDimensionStatusCompleteV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Complete = &variant
	case "pending":
		var variant ReconciliationDimensionStatusPendingV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Pending = &variant
	case "gap":
		var variant ReconciliationDimensionStatusGapV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Gap = &variant
	case "blocked":
		var variant ReconciliationDimensionStatusBlockedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Blocked = &variant
	case "quarantined":
		var variant ReconciliationDimensionStatusQuarantinedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Quarantined = &variant
	case "unavailable":
		var variant ReconciliationDimensionStatusUnavailableV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unavailable = &variant
	case "expired":
		var variant ReconciliationDimensionStatusExpiredV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Expired = &variant
	case "owner_terminal":
		var variant ReconciliationDimensionStatusOwnerTerminalV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.OwnerTerminal = &variant
	default:
		return fmt.Errorf("ReconciliationDimensionStatusV1: unknown discriminator %q", discriminator.State)
	}
	return nil
}

type ReconciliationSubjectV1 struct {
	Source      *ReconciliationSubjectSourceV1      `json:"-"`
	Consumer    *ReconciliationSubjectConsumerV1    `json:"-"`
	Projection  *ReconciliationSubjectProjectionV1  `json:"-"`
	Consequence *ReconciliationSubjectConsequenceV1 `json:"-"`
}

func (value ReconciliationSubjectV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Source != nil {
		if value.Source.Kind != "source" {
			return nil, fmt.Errorf("ReconciliationSubjectV1: selected variant has discriminator %q", value.Source.Kind)
		}
		candidate, err := json.Marshal(value.Source)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Consumer != nil {
		if value.Consumer.Kind != "consumer" {
			return nil, fmt.Errorf("ReconciliationSubjectV1: selected variant has discriminator %q", value.Consumer.Kind)
		}
		candidate, err := json.Marshal(value.Consumer)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Projection != nil {
		if value.Projection.Kind != "projection" {
			return nil, fmt.Errorf("ReconciliationSubjectV1: selected variant has discriminator %q", value.Projection.Kind)
		}
		candidate, err := json.Marshal(value.Projection)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Consequence != nil {
		if value.Consequence.Kind != "consequence" {
			return nil, fmt.Errorf("ReconciliationSubjectV1: selected variant has discriminator %q", value.Consequence.Kind)
		}
		candidate, err := json.Marshal(value.Consequence)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ReconciliationSubjectV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ReconciliationSubjectV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ReconciliationSubjectV1{}
	switch discriminator.Kind {
	case "source":
		var variant ReconciliationSubjectSourceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Source = &variant
	case "consumer":
		var variant ReconciliationSubjectConsumerV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Consumer = &variant
	case "projection":
		var variant ReconciliationSubjectProjectionV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Projection = &variant
	case "consequence":
		var variant ReconciliationSubjectConsequenceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Consequence = &variant
	default:
		return fmt.Errorf("ReconciliationSubjectV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
