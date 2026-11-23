// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package package_data_v1 contains the generated package-data.v1 data model.
package package_data_v1

import (
	"encoding/json"
	"fmt"
	capability_procedure_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/capability_procedure_v1"
)

type GoverningConstraintV1 struct {
	// Policy corresponds to the JSON schema field "policy".
	Policy PackageResourceLinkV1 `json:"policy"`

	// Scope corresponds to the JSON schema field "scope".
	Scope GoverningResourceLinkV1 `json:"scope"`

	// Subject corresponds to the JSON schema field "subject".
	Subject GoverningResourceLinkV1 `json:"subject"`
}

type GoverningExclusiveAuthorityClaimV1 struct {
	// Authority corresponds to the JSON schema field "authority".
	Authority GoverningResourceLinkV1 `json:"authority"`

	// Scope corresponds to the JSON schema field "scope".
	Scope GoverningResourceLinkV1 `json:"scope"`
}

type GoverningNarrowingV1 struct {
	// Broader corresponds to the JSON schema field "broader".
	Broader GoverningResourceLinkV1 `json:"broader"`

	// Narrower corresponds to the JSON schema field "narrower".
	Narrower GoverningResourceLinkV1 `json:"narrower"`

	// Scope corresponds to the JSON schema field "scope".
	Scope GoverningResourceLinkV1 `json:"scope"`
}

type GoverningObligationV1 struct {
	// Obligation corresponds to the JSON schema field "obligation".
	Obligation GoverningResourceLinkV1 `json:"obligation"`

	// Scope corresponds to the JSON schema field "scope".
	Scope GoverningResourceLinkV1 `json:"scope"`

	// Subject corresponds to the JSON schema field "subject".
	Subject GoverningResourceLinkV1 `json:"subject"`
}

type GoverningOrderV1 struct {
	// Constraints corresponds to the JSON schema field "constraints".
	Constraints []GoverningConstraintV1 `json:"constraints,omitempty,omitzero"`

	// ExclusiveAuthority corresponds to the JSON schema field "exclusive_authority".
	ExclusiveAuthority []GoverningExclusiveAuthorityClaimV1 `json:"exclusive_authority,omitempty,omitzero"`

	// Narrowing corresponds to the JSON schema field "narrowing".
	Narrowing []GoverningNarrowingV1 `json:"narrowing,omitempty,omitzero"`

	// Obligations corresponds to the JSON schema field "obligations".
	Obligations []GoverningObligationV1 `json:"obligations,omitempty,omitzero"`

	// Order corresponds to the JSON schema field "order".
	Order []ScopedOrderRelationV1 `json:"order,omitempty,omitzero"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type GoverningResourceLinkExternalV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind *PackageResourceKindV1 `json:"kind,omitempty,omitzero"`

	// Resource corresponds to the JSON schema field "resource".
	Resource PackageResourceRefV1 `json:"resource"`

	// Scope corresponds to the JSON schema field "scope".
	Scope string `json:"scope"`
}

type GoverningResourceLinkLocalV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind *PackageResourceKindV1 `json:"kind,omitempty,omitzero"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`

	// Scope corresponds to the JSON schema field "scope".
	Scope string `json:"scope"`
}

type PackageDataCapabilityPolicyV1 struct {
	// Exposure corresponds to the JSON schema field "exposure".
	Exposure PackageResourceExposureV1 `json:"exposure,omitempty,omitzero"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Policy corresponds to the JSON schema field "policy".
	Policy capability_procedure_v1.CapabilityPolicy `json:"policy"`
}

type PackageDataGoverningOrderV1 struct {
	// Exposure corresponds to the JSON schema field "exposure".
	Exposure PackageResourceExposureV1 `json:"exposure,omitempty,omitzero"`

	// GoverningOrder corresponds to the JSON schema field "governing_order".
	GoverningOrder GoverningOrderV1 `json:"governing_order"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type PackageDataToolRecipeV1 struct {
	// Exposure corresponds to the JSON schema field "exposure".
	Exposure PackageResourceExposureV1 `json:"exposure,omitempty,omitzero"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Recipe corresponds to the JSON schema field "recipe".
	Recipe capability_procedure_v1.ToolRecipe `json:"recipe"`
}

// Exposure corresponds to the JSON schema field "exposure".

// GoverningOrder corresponds to the JSON schema field "governing_order".

// Kind corresponds to the JSON schema field "kind".

// Policy corresponds to the JSON schema field "policy".

// Recipe corresponds to the JSON schema field "recipe".

type PackageIDV1 string

type PackageResourceExposureV1 string

const PackageResourceExposureV1Exported PackageResourceExposureV1 = "exported"
const PackageResourceExposureV1Internal PackageResourceExposureV1 = "internal"

type PackageResourceIDV1 string

type PackageResourceKindV1 string

const PackageResourceKindV1CapabilityPolicy PackageResourceKindV1 = "capability_policy"
const PackageResourceKindV1Doctrine PackageResourceKindV1 = "doctrine"
const PackageResourceKindV1GoverningOrder PackageResourceKindV1 = "governing_order"
const PackageResourceKindV1Operation PackageResourceKindV1 = "operation"
const PackageResourceKindV1OperationArchetype PackageResourceKindV1 = "operation_archetype"
const PackageResourceKindV1PolicyFamily PackageResourceKindV1 = "policy_family"
const PackageResourceKindV1Role PackageResourceKindV1 = "role"
const PackageResourceKindV1RoleArchetype PackageResourceKindV1 = "role_archetype"
const PackageResourceKindV1RunMode PackageResourceKindV1 = "run_mode"
const PackageResourceKindV1Stack PackageResourceKindV1 = "stack"
const PackageResourceKindV1ToolRecipe PackageResourceKindV1 = "tool_recipe"

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

type ScopedOrderRelationV1 struct {
	// After corresponds to the JSON schema field "after".
	After GoverningResourceLinkV1 `json:"after"`

	// Before corresponds to the JSON schema field "before".
	Before GoverningResourceLinkV1 `json:"before"`

	// Purpose corresponds to the JSON schema field "purpose".
	Purpose SemanticOrderPurposeV1 `json:"purpose"`

	// Scope corresponds to the JSON schema field "scope".
	Scope GoverningResourceLinkV1 `json:"scope"`
}

type SemanticOrderPurposeV1 string

const SemanticOrderPurposeV1Composition SemanticOrderPurposeV1 = "composition"
const SemanticOrderPurposeV1Execution SemanticOrderPurposeV1 = "execution"
const SemanticOrderPurposeV1Presentation SemanticOrderPurposeV1 = "presentation"
const SemanticOrderPurposeV1Reasoning SemanticOrderPurposeV1 = "reasoning"

type GoverningResourceLinkV1 struct {
	Local    *GoverningResourceLinkLocalV1    `json:"-"`
	External *GoverningResourceLinkExternalV1 `json:"-"`
}

func (value GoverningResourceLinkV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Local != nil {
		if value.Local.Scope != "local" {
			return nil, fmt.Errorf("GoverningResourceLinkV1: selected variant has discriminator %q", value.Local.Scope)
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
			return nil, fmt.Errorf("GoverningResourceLinkV1: selected variant has discriminator %q", value.External.Scope)
		}
		candidate, err := json.Marshal(value.External)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("GoverningResourceLinkV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *GoverningResourceLinkV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Scope string `json:"scope"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = GoverningResourceLinkV1{}
	switch discriminator.Scope {
	case "local":
		var variant GoverningResourceLinkLocalV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Local = &variant
	case "external":
		var variant GoverningResourceLinkExternalV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.External = &variant
	default:
		return fmt.Errorf("GoverningResourceLinkV1: unknown discriminator %q", discriminator.Scope)
	}
	return nil
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

type PackageDataV1 struct {
	ToolRecipe       *PackageDataToolRecipeV1       `json:"-"`
	CapabilityPolicy *PackageDataCapabilityPolicyV1 `json:"-"`
	GoverningOrder   *PackageDataGoverningOrderV1   `json:"-"`
}

func (value PackageDataV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.ToolRecipe != nil {
		if value.ToolRecipe.Kind != "tool_recipe" {
			return nil, fmt.Errorf("PackageDataV1: selected variant has discriminator %q", value.ToolRecipe.Kind)
		}
		candidate, err := json.Marshal(value.ToolRecipe)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.CapabilityPolicy != nil {
		if value.CapabilityPolicy.Kind != "capability_policy" {
			return nil, fmt.Errorf("PackageDataV1: selected variant has discriminator %q", value.CapabilityPolicy.Kind)
		}
		candidate, err := json.Marshal(value.CapabilityPolicy)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.GoverningOrder != nil {
		if value.GoverningOrder.Kind != "governing_order" {
			return nil, fmt.Errorf("PackageDataV1: selected variant has discriminator %q", value.GoverningOrder.Kind)
		}
		candidate, err := json.Marshal(value.GoverningOrder)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("PackageDataV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *PackageDataV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = PackageDataV1{}
	switch discriminator.Kind {
	case "tool_recipe":
		var variant PackageDataToolRecipeV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ToolRecipe = &variant
	case "capability_policy":
		var variant PackageDataCapabilityPolicyV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.CapabilityPolicy = &variant
	case "governing_order":
		var variant PackageDataGoverningOrderV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.GoverningOrder = &variant
	default:
		return fmt.Errorf("PackageDataV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
