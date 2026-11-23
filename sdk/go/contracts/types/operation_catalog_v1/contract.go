// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package operation_catalog_v1 contains the generated operation-catalog.v1 data model.
package operation_catalog_v1

import (
	"encoding/json"
	"fmt"
	sealed_artifact_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sealed_artifact_v1"
)
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CatalogOperationStateCapabilityBearingV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Support corresponds to the JSON schema field "support".
	Support OperationSupport `json:"support"`
}

type CatalogOperationStateSemanticOnlyV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type LogicalRepositoryIDV1 string

type OperationCatalogEntryV1 struct {
	// Contract corresponds to the JSON schema field "contract".
	Contract sealed_artifact_v1.SealedArtifactV1 `json:"contract"`

	// ContractDigest corresponds to the JSON schema field "contract_digest".
	ContractDigest sha256_digest_v1.Sha256DigestV1 `json:"contract_digest"`

	// Dispatch corresponds to the JSON schema field "dispatch".
	Dispatch OperationDispatchV1 `json:"dispatch"`

	// ID corresponds to the JSON schema field "id".
	ID PackageResourceRefV1 `json:"id"`

	// State corresponds to the JSON schema field "state".
	State CatalogOperationStateV1 `json:"state"`
}

type OperationCatalogV1 struct {
	// CatalogDigest corresponds to the JSON schema field "catalog_digest".
	CatalogDigest sha256_digest_v1.Sha256DigestV1 `json:"catalog_digest"`

	// Entries corresponds to the JSON schema field "entries".
	Entries []OperationCatalogEntryV1 `json:"entries"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// ProviderBinding corresponds to the JSON schema field "provider_binding".
	ProviderBinding ProviderBindingRefV1 `json:"provider_binding"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type OperationDispatchInternalV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type OperationDispatchModelOrUserV1 struct {
	// Aliases corresponds to the JSON schema field "aliases".
	Aliases []string `json:"aliases,omitempty,omitzero"`

	// Invocation corresponds to the JSON schema field "invocation".
	Invocation string `json:"invocation"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type OperationDispatchUserOnlyV1 struct {
	// Aliases corresponds to the JSON schema field "aliases".
	Aliases []string `json:"aliases,omitempty,omitzero"`

	// Invocation corresponds to the JSON schema field "invocation".
	Invocation string `json:"invocation"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type OperationSupport string

const OperationSupportBlocked OperationSupport = "blocked"
const OperationSupportDegraded OperationSupport = "degraded"
const OperationSupportFull OperationSupport = "full"

type PackageIDV1 string

type PackageResourceIDV1 string

type PackageResourceRefV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`
}

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

type CatalogOperationStateV1 struct {
	SemanticOnly      *CatalogOperationStateSemanticOnlyV1      `json:"-"`
	CapabilityBearing *CatalogOperationStateCapabilityBearingV1 `json:"-"`
}

func (value CatalogOperationStateV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.SemanticOnly != nil {
		if value.SemanticOnly.Kind != "semantic_only" {
			return nil, fmt.Errorf("CatalogOperationStateV1: selected variant has discriminator %q", value.SemanticOnly.Kind)
		}
		candidate, err := json.Marshal(value.SemanticOnly)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.CapabilityBearing != nil {
		if value.CapabilityBearing.Kind != "capability_bearing" {
			return nil, fmt.Errorf("CatalogOperationStateV1: selected variant has discriminator %q", value.CapabilityBearing.Kind)
		}
		candidate, err := json.Marshal(value.CapabilityBearing)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("CatalogOperationStateV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *CatalogOperationStateV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = CatalogOperationStateV1{}
	switch discriminator.Kind {
	case "semantic_only":
		var variant CatalogOperationStateSemanticOnlyV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.SemanticOnly = &variant
	case "capability_bearing":
		var variant CatalogOperationStateCapabilityBearingV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.CapabilityBearing = &variant
	default:
		return fmt.Errorf("CatalogOperationStateV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type OperationDispatchV1 struct {
	ModelOrUser *OperationDispatchModelOrUserV1 `json:"-"`
	UserOnly    *OperationDispatchUserOnlyV1    `json:"-"`
	Internal    *OperationDispatchInternalV1    `json:"-"`
}

func (value OperationDispatchV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.ModelOrUser != nil {
		if value.ModelOrUser.Kind != "model_or_user" {
			return nil, fmt.Errorf("OperationDispatchV1: selected variant has discriminator %q", value.ModelOrUser.Kind)
		}
		candidate, err := json.Marshal(value.ModelOrUser)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.UserOnly != nil {
		if value.UserOnly.Kind != "user_only" {
			return nil, fmt.Errorf("OperationDispatchV1: selected variant has discriminator %q", value.UserOnly.Kind)
		}
		candidate, err := json.Marshal(value.UserOnly)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Internal != nil {
		if value.Internal.Kind != "internal" {
			return nil, fmt.Errorf("OperationDispatchV1: selected variant has discriminator %q", value.Internal.Kind)
		}
		candidate, err := json.Marshal(value.Internal)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("OperationDispatchV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *OperationDispatchV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = OperationDispatchV1{}
	switch discriminator.Kind {
	case "model_or_user":
		var variant OperationDispatchModelOrUserV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ModelOrUser = &variant
	case "user_only":
		var variant OperationDispatchUserOnlyV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.UserOnly = &variant
	case "internal":
		var variant OperationDispatchInternalV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Internal = &variant
	default:
		return fmt.Errorf("OperationDispatchV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
