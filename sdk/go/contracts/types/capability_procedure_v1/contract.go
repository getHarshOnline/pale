// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package capability_procedure_v1 contains the generated capability-procedure.v1 data model.
package capability_procedure_v1

import (
	"encoding/json"
	"fmt"
)

type CapabilityFallback struct {
	// Guidance corresponds to the JSON schema field "guidance".
	Guidance string `json:"guidance"`

	// Recipe corresponds to the JSON schema field "recipe".
	Recipe *PackageResourceLinkV1 `json:"recipe,omitempty,omitzero"`
}

type CapabilityIDV1 string

type CapabilityPolicy struct {
	// ForbiddenCapabilities corresponds to the JSON schema field
	// "forbidden_capabilities".
	ForbiddenCapabilities []CapabilityIDV1 `json:"forbidden_capabilities,omitempty,omitzero"`

	// ForbiddenEffects corresponds to the JSON schema field "forbidden_effects".
	ForbiddenEffects []EffectClass `json:"forbidden_effects,omitempty,omitzero"`

	// PermittedCapabilities corresponds to the JSON schema field
	// "permitted_capabilities".
	PermittedCapabilities []CapabilityIDV1 `json:"permitted_capabilities"`

	// PermittedEffects corresponds to the JSON schema field "permitted_effects".
	PermittedEffects []EffectClass `json:"permitted_effects"`
}

type CapabilityRequirementIDV1 string

type CapabilityRequirementPolicyOptional struct {
	// Fallback corresponds to the JSON schema field "fallback".
	Fallback CapabilityFallback `json:"fallback"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type CapabilityRequirementPolicyRequiredBlock struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type CapabilityRequirementPolicyRequiredDegrade struct {
	// DegradedOutcome corresponds to the JSON schema field "degraded_outcome".
	DegradedOutcome DegradedOutcomeIDV1 `json:"degraded_outcome"`

	// Fallback corresponds to the JSON schema field "fallback".
	Fallback CapabilityFallback `json:"fallback"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type DegradedOutcomeIDV1 string

type EffectClass string

const EffectClassCompute EffectClass = "compute"
const EffectClassExecute EffectClass = "execute"
const EffectClassNetwork EffectClass = "network"
const EffectClassPublish EffectClass = "publish"
const EffectClassRead EffectClass = "read"
const EffectClassWrite EffectClass = "write"

type FailurePosture string

const FailurePostureEscalate FailurePosture = "escalate"
const FailurePostureFailClosed FailurePosture = "fail_closed"
const FailurePostureFailOpen FailurePosture = "fail_open"
const FailurePostureRetry FailurePosture = "retry"
const FailurePostureSteer FailurePosture = "steer"

type NonRootJSONPointer string

type OperationCapabilityRequirement struct {
	// Capability corresponds to the JSON schema field "capability".
	Capability CapabilityIDV1 `json:"capability"`

	// Description corresponds to the JSON schema field "description".
	Description string `json:"description"`

	// EffectClass corresponds to the JSON schema field "effect_class".
	EffectClass EffectClass `json:"effect_class"`

	// FailurePosture corresponds to the JSON schema field "failure_posture".
	FailurePosture FailurePosture `json:"failure_posture"`

	// ID corresponds to the JSON schema field "id".
	ID CapabilityRequirementIDV1 `json:"id"`

	// Policy corresponds to the JSON schema field "policy".
	Policy CapabilityRequirementPolicy `json:"policy"`

	// Recipe corresponds to the JSON schema field "recipe".
	Recipe *PackageResourceLinkV1 `json:"recipe,omitempty,omitzero"`
}

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

// Resource corresponds to the JSON schema field "resource".

// ResourceID corresponds to the JSON schema field "resource_id".

// Scope corresponds to the JSON schema field "scope".

type PackageResourceRefV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`
}

type ProofCeilingIDV1 string

type ToolContractIDV1 string

type ToolEvidenceContract struct {
	// ProofCeiling corresponds to the JSON schema field "proof_ceiling".
	ProofCeiling ProofCeilingIDV1 `json:"proof_ceiling"`

	// RepositoryVerifier corresponds to the JSON schema field "repository_verifier".
	RepositoryVerifier bool `json:"repository_verifier,omitempty,omitzero"`

	// RequiredFields corresponds to the JSON schema field "required_fields".
	RequiredFields []NonRootJSONPointer `json:"required_fields"`
}

type ToolOperationIDV1 string

type ToolRecipe struct {
	// Capability corresponds to the JSON schema field "capability".
	Capability CapabilityIDV1 `json:"capability"`

	// Description corresponds to the JSON schema field "description".
	Description string `json:"description"`

	// EffectClass corresponds to the JSON schema field "effect_class".
	EffectClass EffectClass `json:"effect_class"`

	// Evidence corresponds to the JSON schema field "evidence".
	Evidence ToolEvidenceContract `json:"evidence"`

	// InputMapping corresponds to the JSON schema field "input_mapping".
	InputMapping ToolRecipeInputMapping `json:"input_mapping"`

	// ToolContract corresponds to the JSON schema field "tool_contract".
	ToolContract ToolContractIDV1 `json:"tool_contract"`

	// ToolOperation corresponds to the JSON schema field "tool_operation".
	ToolOperation ToolOperationIDV1 `json:"tool_operation"`
}

type ToolRecipeInputMapping map[string]string

type CapabilityRequirementPolicy struct {
	RequiredBlock   *CapabilityRequirementPolicyRequiredBlock   `json:"-"`
	RequiredDegrade *CapabilityRequirementPolicyRequiredDegrade `json:"-"`
	Optional        *CapabilityRequirementPolicyOptional        `json:"-"`
}

func (value CapabilityRequirementPolicy) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.RequiredBlock != nil {
		if value.RequiredBlock.Kind != "required_block" {
			return nil, fmt.Errorf("CapabilityRequirementPolicy: selected variant has discriminator %q", value.RequiredBlock.Kind)
		}
		candidate, err := json.Marshal(value.RequiredBlock)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.RequiredDegrade != nil {
		if value.RequiredDegrade.Kind != "required_degrade" {
			return nil, fmt.Errorf("CapabilityRequirementPolicy: selected variant has discriminator %q", value.RequiredDegrade.Kind)
		}
		candidate, err := json.Marshal(value.RequiredDegrade)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Optional != nil {
		if value.Optional.Kind != "optional" {
			return nil, fmt.Errorf("CapabilityRequirementPolicy: selected variant has discriminator %q", value.Optional.Kind)
		}
		candidate, err := json.Marshal(value.Optional)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("CapabilityRequirementPolicy: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *CapabilityRequirementPolicy) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = CapabilityRequirementPolicy{}
	switch discriminator.Kind {
	case "required_block":
		var variant CapabilityRequirementPolicyRequiredBlock
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.RequiredBlock = &variant
	case "required_degrade":
		var variant CapabilityRequirementPolicyRequiredDegrade
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.RequiredDegrade = &variant
	case "optional":
		var variant CapabilityRequirementPolicyOptional
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Optional = &variant
	default:
		return fmt.Errorf("CapabilityRequirementPolicy: unknown discriminator %q", discriminator.Kind)
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
