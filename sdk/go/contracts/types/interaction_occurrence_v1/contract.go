// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package interaction_occurrence_v1 contains the generated interaction-occurrence.v1 data model.
package interaction_occurrence_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type CanonicalSignatureV1 string

type DataClassificationV1 string

type InteractionDisclosureCeilingV1 string

type InteractionLifecycleProjectionCurrentV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type InteractionLifecycleProjectionDeletedV1 struct {
	// Evidence corresponds to the JSON schema field "evidence".
	Evidence OwnerRecordCoordinateV1 `json:"evidence"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type InteractionLifecycleProjectionEditedV1 struct {
	// Evidence corresponds to the JSON schema field "evidence".
	Evidence OwnerRecordCoordinateV1 `json:"evidence"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type InteractionLifecycleProjectionSupersededV1 struct {
	// Evidence corresponds to the JSON schema field "evidence".
	Evidence OwnerRecordCoordinateV1 `json:"evidence"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type InteractionLifecycleProjectionUnknownV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type InteractionOccurrenceIDV1 string

type InteractionOccurrenceV1 struct {
	// ActorProvenance corresponds to the JSON schema field "actor_provenance".
	ActorProvenance OwnerRecordCoordinateV1 `json:"actor_provenance"`

	// AudienceSnapshot corresponds to the JSON schema field "audience_snapshot".
	AudienceSnapshot OwnerRecordKnowledgeV1 `json:"audience_snapshot"`

	// CanonicalOccurrenceSha256 corresponds to the JSON schema field
	// "canonical_occurrence_sha256".
	CanonicalOccurrenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_occurrence_sha256"`

	// Classification corresponds to the JSON schema field "classification".
	Classification DataClassificationV1 `json:"classification"`

	// ContainerGenre corresponds to the JSON schema field "container_genre".
	ContainerGenre NativeContainerGenreV1 `json:"container_genre"`

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`

	// DisclosureCeiling corresponds to the JSON schema field "disclosure_ceiling".
	DisclosureCeiling InteractionDisclosureCeilingV1 `json:"disclosure_ceiling"`

	// EventReferenceRef corresponds to the JSON schema field "event_reference_ref".
	EventReferenceRef ResidentRecordRefV1 `json:"event_reference_ref"`

	// EventReferenceSha256 corresponds to the JSON schema field
	// "event_reference_sha256".
	EventReferenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"event_reference_sha256"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// Lifecycle corresponds to the JSON schema field "lifecycle".
	Lifecycle InteractionLifecycleProjectionV1 `json:"lifecycle"`

	// MentionAndCcRecords corresponds to the JSON schema field
	// "mention_and_cc_records".
	MentionAndCcRecords []OwnerRecordCoordinateV1 `json:"mention_and_cc_records"`

	// NativeContainer corresponds to the JSON schema field "native_container".
	NativeContainer OwnerRecordCoordinateV1 `json:"native_container"`

	// OccurrenceID corresponds to the JSON schema field "occurrence_id".
	OccurrenceID InteractionOccurrenceIDV1 `json:"occurrence_id"`

	// ParentPlacement corresponds to the JSON schema field "parent_placement".
	ParentPlacement OwnerRecordKnowledgeV1 `json:"parent_placement"`

	// ParticipantSnapshot corresponds to the JSON schema field
	// "participant_snapshot".
	ParticipantSnapshot OwnerRecordKnowledgeV1 `json:"participant_snapshot"`

	// PayloadResolutionPolicy corresponds to the JSON schema field
	// "payload_resolution_policy".
	PayloadResolutionPolicy PayloadResolutionPolicyV1 `json:"payload_resolution_policy"`

	// ProjectedAt corresponds to the JSON schema field "projected_at".
	ProjectedAt UnixTimeV1 `json:"projected_at"`

	// ReferencedRecords corresponds to the JSON schema field "referenced_records".
	ReferencedRecords []OwnerRecordCoordinateV1 `json:"referenced_records"`

	// ResponseExpectation corresponds to the JSON schema field
	// "response_expectation".
	ResponseExpectation InteractionResponseExpectationV1 `json:"response_expectation"`

	// RootPlacement corresponds to the JSON schema field "root_placement".
	RootPlacement OwnerRecordKnowledgeV1 `json:"root_placement"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// ThreadPlacement corresponds to the JSON schema field "thread_placement".
	ThreadPlacement OwnerRecordKnowledgeV1 `json:"thread_placement"`

	// Visibility corresponds to the JSON schema field "visibility".
	Visibility InteractionVisibilityV1 `json:"visibility"`
}

type InteractionResponseExpectationExpectedV1 struct {
	// Expectation corresponds to the JSON schema field "expectation".
	Expectation ResponseExpectationV1 `json:"expectation"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type InteractionResponseExpectationNoneV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type InteractionResponseExpectationUnknownV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type InteractionVisibilityV1 string

type NativeContainerGenreV1 string

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

type OwnerRecordKnowledgeAbsentV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type OwnerRecordKnowledgeKnownV1 struct {
	// Coordinate corresponds to the JSON schema field "coordinate".
	Coordinate OwnerRecordCoordinateV1 `json:"coordinate"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type OwnerRecordKnowledgeUnknownV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type OwnerSystemIDV1 string

type PayloadResolutionPolicyV1 string

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type ResponseExpectationV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int

type InteractionLifecycleProjectionV1 struct {
	Unknown    *InteractionLifecycleProjectionUnknownV1    `json:"-"`
	Current    *InteractionLifecycleProjectionCurrentV1    `json:"-"`
	Edited     *InteractionLifecycleProjectionEditedV1     `json:"-"`
	Deleted    *InteractionLifecycleProjectionDeletedV1    `json:"-"`
	Superseded *InteractionLifecycleProjectionSupersededV1 `json:"-"`
}

func (value InteractionLifecycleProjectionV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Unknown != nil {
		if value.Unknown.State != "unknown" {
			return nil, fmt.Errorf("InteractionLifecycleProjectionV1: selected variant has discriminator %q", value.Unknown.State)
		}
		candidate, err := json.Marshal(value.Unknown)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Current != nil {
		if value.Current.State != "current" {
			return nil, fmt.Errorf("InteractionLifecycleProjectionV1: selected variant has discriminator %q", value.Current.State)
		}
		candidate, err := json.Marshal(value.Current)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Edited != nil {
		if value.Edited.State != "edited" {
			return nil, fmt.Errorf("InteractionLifecycleProjectionV1: selected variant has discriminator %q", value.Edited.State)
		}
		candidate, err := json.Marshal(value.Edited)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Deleted != nil {
		if value.Deleted.State != "deleted" {
			return nil, fmt.Errorf("InteractionLifecycleProjectionV1: selected variant has discriminator %q", value.Deleted.State)
		}
		candidate, err := json.Marshal(value.Deleted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Superseded != nil {
		if value.Superseded.State != "superseded" {
			return nil, fmt.Errorf("InteractionLifecycleProjectionV1: selected variant has discriminator %q", value.Superseded.State)
		}
		candidate, err := json.Marshal(value.Superseded)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("InteractionLifecycleProjectionV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *InteractionLifecycleProjectionV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		State string `json:"state"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = InteractionLifecycleProjectionV1{}
	switch discriminator.State {
	case "unknown":
		var variant InteractionLifecycleProjectionUnknownV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unknown = &variant
	case "current":
		var variant InteractionLifecycleProjectionCurrentV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Current = &variant
	case "edited":
		var variant InteractionLifecycleProjectionEditedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Edited = &variant
	case "deleted":
		var variant InteractionLifecycleProjectionDeletedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Deleted = &variant
	case "superseded":
		var variant InteractionLifecycleProjectionSupersededV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Superseded = &variant
	default:
		return fmt.Errorf("InteractionLifecycleProjectionV1: unknown discriminator %q", discriminator.State)
	}
	return nil
}

type InteractionResponseExpectationV1 struct {
	Unknown  *InteractionResponseExpectationUnknownV1  `json:"-"`
	None     *InteractionResponseExpectationNoneV1     `json:"-"`
	Expected *InteractionResponseExpectationExpectedV1 `json:"-"`
}

func (value InteractionResponseExpectationV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Unknown != nil {
		if value.Unknown.State != "unknown" {
			return nil, fmt.Errorf("InteractionResponseExpectationV1: selected variant has discriminator %q", value.Unknown.State)
		}
		candidate, err := json.Marshal(value.Unknown)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.None != nil {
		if value.None.State != "none" {
			return nil, fmt.Errorf("InteractionResponseExpectationV1: selected variant has discriminator %q", value.None.State)
		}
		candidate, err := json.Marshal(value.None)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Expected != nil {
		if value.Expected.State != "expected" {
			return nil, fmt.Errorf("InteractionResponseExpectationV1: selected variant has discriminator %q", value.Expected.State)
		}
		candidate, err := json.Marshal(value.Expected)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("InteractionResponseExpectationV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *InteractionResponseExpectationV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		State string `json:"state"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = InteractionResponseExpectationV1{}
	switch discriminator.State {
	case "unknown":
		var variant InteractionResponseExpectationUnknownV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unknown = &variant
	case "none":
		var variant InteractionResponseExpectationNoneV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.None = &variant
	case "expected":
		var variant InteractionResponseExpectationExpectedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Expected = &variant
	default:
		return fmt.Errorf("InteractionResponseExpectationV1: unknown discriminator %q", discriminator.State)
	}
	return nil
}

type OwnerRecordKnowledgeV1 struct {
	Unknown *OwnerRecordKnowledgeUnknownV1 `json:"-"`
	Absent  *OwnerRecordKnowledgeAbsentV1  `json:"-"`
	Known   *OwnerRecordKnowledgeKnownV1   `json:"-"`
}

func (value OwnerRecordKnowledgeV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Unknown != nil {
		if value.Unknown.State != "unknown" {
			return nil, fmt.Errorf("OwnerRecordKnowledgeV1: selected variant has discriminator %q", value.Unknown.State)
		}
		candidate, err := json.Marshal(value.Unknown)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Absent != nil {
		if value.Absent.State != "absent" {
			return nil, fmt.Errorf("OwnerRecordKnowledgeV1: selected variant has discriminator %q", value.Absent.State)
		}
		candidate, err := json.Marshal(value.Absent)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Known != nil {
		if value.Known.State != "known" {
			return nil, fmt.Errorf("OwnerRecordKnowledgeV1: selected variant has discriminator %q", value.Known.State)
		}
		candidate, err := json.Marshal(value.Known)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("OwnerRecordKnowledgeV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *OwnerRecordKnowledgeV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		State string `json:"state"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = OwnerRecordKnowledgeV1{}
	switch discriminator.State {
	case "unknown":
		var variant OwnerRecordKnowledgeUnknownV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unknown = &variant
	case "absent":
		var variant OwnerRecordKnowledgeAbsentV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Absent = &variant
	case "known":
		var variant OwnerRecordKnowledgeKnownV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Known = &variant
	default:
		return fmt.Errorf("OwnerRecordKnowledgeV1: unknown discriminator %q", discriminator.State)
	}
	return nil
}
