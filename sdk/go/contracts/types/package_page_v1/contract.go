// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package package_page_v1 contains the generated package-page.v1 data model.
package package_page_v1

import (
	"encoding/json"
	"fmt"
	capability_procedure_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/capability_procedure_v1"
)
import official_reference_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/official_reference_v1"

type PackageGuideV1 struct {
	// Exposure corresponds to the JSON schema field "exposure".
	Exposure *PackageResourceExposureV1 `json:"exposure,omitempty,omitzero"`
}

type PackageIDV1 string

type PackagePageV1 struct {
	// Description corresponds to the JSON schema field "description".
	Description string `json:"description"`

	// Exposure corresponds to the JSON schema field "exposure".
	Exposure PackageResourceExposureV1 `json:"exposure,omitempty,omitzero"`

	// Guide corresponds to the JSON schema field "guide".
	Guide *PackageGuideV1 `json:"guide,omitempty,omitzero"`

	// OfficialSources corresponds to the JSON schema field "official_sources".
	OfficialSources []official_reference_v1.OfficialReferenceV1 `json:"official_sources,omitempty,omitzero"`

	// Policy corresponds to the JSON schema field "policy".
	Policy *PackagePolicyV1 `json:"policy,omitempty,omitzero"`

	// Role corresponds to the JSON schema field "role".
	Role *PackageRoleV1 `json:"role,omitempty,omitzero"`

	// SeeAlso corresponds to the JSON schema field "see_also".
	SeeAlso []PackageResourceLinkV1 `json:"see_also,omitempty,omitzero"`

	// Stack corresponds to the JSON schema field "stack".
	Stack *PackageStackV1 `json:"stack,omitempty,omitzero"`

	// Task corresponds to the JSON schema field "task".
	Task *PackageTaskV1 `json:"task,omitempty,omitzero"`

	// Uses corresponds to the JSON schema field "uses".
	Uses []PackageResourceLinkV1 `json:"uses,omitempty,omitzero"`
}

type PackagePolicyV1 struct {
	// Exposure corresponds to the JSON schema field "exposure".
	Exposure *PackageResourceExposureV1 `json:"exposure,omitempty,omitzero"`
}

type PackageResourceExposureV1 string

const PackageResourceExposureV1Exported PackageResourceExposureV1 = "exported"
const PackageResourceExposureV1Internal PackageResourceExposureV1 = "internal"

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

// Resource corresponds to the JSON schema field "resource".

// ResourceID corresponds to the JSON schema field "resource_id".

// Scope corresponds to the JSON schema field "scope".

type PackageResourceRefV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`
}

type PackageRoleV1 struct {
	// AppliesTo corresponds to the JSON schema field "applies_to".
	AppliesTo []PackageResourceLinkV1 `json:"applies_to,omitempty,omitzero"`

	// Exposure corresponds to the JSON schema field "exposure".
	Exposure *PackageResourceExposureV1 `json:"exposure,omitempty,omitzero"`

	// Extends corresponds to the JSON schema field "extends".
	Extends []PackageResourceLinkV1 `json:"extends,omitempty,omitzero"`

	// Uses corresponds to the JSON schema field "uses".
	Uses []PackageResourceLinkV1 `json:"uses,omitempty,omitzero"`
}

type PackageStackV1 struct {
	// Exposure corresponds to the JSON schema field "exposure".
	Exposure *PackageResourceExposureV1 `json:"exposure,omitempty,omitzero"`

	// Extends corresponds to the JSON schema field "extends".
	Extends []PackageResourceLinkV1 `json:"extends,omitempty,omitzero"`

	// RoleUses corresponds to the JSON schema field "role_uses".
	RoleUses []PackageResourceLinkV1 `json:"role_uses,omitempty,omitzero"`

	// Uses corresponds to the JSON schema field "uses".
	Uses []PackageResourceLinkV1 `json:"uses,omitempty,omitzero"`
}

type PackageTaskV1 struct {
	// CapabilityPolicy corresponds to the JSON schema field "capability_policy".
	CapabilityPolicy *PackageResourceLinkV1 `json:"capability_policy,omitempty,omitzero"`

	// CapabilityRequirements corresponds to the JSON schema field
	// "capability_requirements".
	CapabilityRequirements []capability_procedure_v1.OperationCapabilityRequirement `json:"capability_requirements,omitempty,omitzero"`

	// Exposure corresponds to the JSON schema field "exposure".
	Exposure *PackageResourceExposureV1 `json:"exposure,omitempty,omitzero"`

	// Outputs corresponds to the JSON schema field "outputs".
	Outputs []string `json:"outputs,omitempty,omitzero"`

	// Roles corresponds to the JSON schema field "roles".
	Roles []PackageResourceLinkV1 `json:"roles,omitempty,omitzero"`

	// Uses corresponds to the JSON schema field "uses".
	Uses []PackageResourceLinkV1 `json:"uses,omitempty,omitzero"`
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
