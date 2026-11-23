// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package doctrine_entry_v1 contains the generated doctrine-entry.v1 data model.
package doctrine_entry_v1

import (
	"encoding/json"
	"fmt"
	official_reference_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/official_reference_v1"
)

type DoctrineCatalog struct {
	// Entry corresponds to the JSON schema field "entry".
	Entry []DoctrineEntry `json:"entry"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type DoctrineEntry struct {
	// Description corresponds to the JSON schema field "description".
	Description string `json:"description"`

	// Doctrine corresponds to the JSON schema field "doctrine".
	Doctrine OwnedConfinedRelativePath `json:"doctrine"`

	// Extends corresponds to the JSON schema field "extends".
	Extends []PackageResourceLinkV1 `json:"extends,omitempty,omitzero"`

	// ID corresponds to the JSON schema field "id".
	ID PackageResourceIDV1 `json:"id"`

	// OfficialSources corresponds to the JSON schema field "official_sources".
	OfficialSources []official_reference_v1.OfficialReferenceV1 `json:"official_sources,omitempty,omitzero"`

	// Placement corresponds to the JSON schema field "placement".
	Placement DoctrinePlacement `json:"placement"`

	// Requires corresponds to the JSON schema field "requires".
	Requires []PackageResourceLinkV1 `json:"requires,omitempty,omitzero"`

	// SeeAlso corresponds to the JSON schema field "see_also".
	SeeAlso []PackageResourceLinkV1 `json:"see_also,omitempty,omitzero"`
}

type DoctrineEntryV1 struct {
	// Doctrine corresponds to the JSON schema field "doctrine".
	Doctrine DoctrineCatalog `json:"doctrine"`
}

type DoctrinePlacement string

const DoctrinePlacementOperationProcedure DoctrinePlacement = "operation_procedure"
const DoctrinePlacementStandaloneJudgment DoctrinePlacement = "standalone_judgment"
const DoctrinePlacementStartupPolicy DoctrinePlacement = "startup_policy"

type OwnedConfinedRelativePath string

type PackageIDV1 string

type PackageResourceIDV1 string

type PackageResourceLinkExternalV1 struct {
	// Resource corresponds to the JSON schema field "resource".
	Resource PackageResourceRefV1 `json:"resource"`

	// Scope corresponds to the JSON schema field "scope".
	Scope string `json:"scope"`
}

type PackageResourceLinkLocalV1 struct {
	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`

	// Scope corresponds to the JSON schema field "scope".
	Scope string `json:"scope"`
}

type PackageResourceRefV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`
}

type PackageResourceLinkV1 struct {
	Local    *PackageResourceLinkLocalV1    `json:"-"`
	External *PackageResourceLinkExternalV1 `json:"-"`
}

func (value PackageResourceLinkV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Local != nil {
		if value.Local.Scope != "local" {
			return nil, fmt.Errorf("PackageResourceLinkV1: selected variant has discriminator %q", value.Local.Scope)
		}
		candidate, err := json.Marshal(value.Local)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.External != nil {
		if value.External.Scope != "external" {
			return nil, fmt.Errorf("PackageResourceLinkV1: selected variant has discriminator %q", value.External.Scope)
		}
		candidate, err := json.Marshal(value.External)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("PackageResourceLinkV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *PackageResourceLinkV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Scope string `json:"scope"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = PackageResourceLinkV1{}
	switch discriminator.Scope {
	case "local":
		var variant PackageResourceLinkLocalV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Local = &variant
	case "external":
		var variant PackageResourceLinkExternalV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.External = &variant
	default:
		return fmt.Errorf("PackageResourceLinkV1: unknown discriminator %q", discriminator.Scope)
	}
	return nil
}
