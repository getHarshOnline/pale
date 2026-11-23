// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package capability_bindings_v1 contains the generated capability-bindings.v1 data model.
package capability_bindings_v1

import (
	"encoding/json"
	"fmt"
	capability_procedure_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/capability_procedure_v1"
)
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type Approval string

const ApprovalAsk Approval = "ask"
const ApprovalGated Approval = "gated"
const ApprovalNone Approval = "none"

type CapabilityBindingsV1 struct {
	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// Providers corresponds to the JSON schema field "providers".
	Providers []ResolvedCapabilityProviderV1 `json:"providers"`

	// Recipes corresponds to the JSON schema field "recipes".
	Recipes []ResolvedToolRecipeV1 `json:"recipes,omitempty,omitzero"`

	// Requirements corresponds to the JSON schema field "requirements".
	Requirements []ResolvedCapabilityUseV1 `json:"requirements"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type CapabilityIDV1 string

type CapabilityProviderBindingConnector struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ProviderTools corresponds to the JSON schema field "provider_tools".
	ProviderTools []ProviderToolBindingV1 `json:"provider_tools,omitempty,omitzero"`

	// Target corresponds to the JSON schema field "target".
	Target string `json:"target"`
}

type CapabilityProviderBindingHostNative struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ProviderTools corresponds to the JSON schema field "provider_tools".
	ProviderTools []ProviderToolBindingV1 `json:"provider_tools"`
}

type CapabilityProviderBindingNativeRust struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Target corresponds to the JSON schema field "target".
	Target string `json:"target"`
}

type CapabilityRequirementIDV1 string

type CapabilityUseKind string

const CapabilityUseKindOperationRequirement CapabilityUseKind = "operation_requirement"

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

type HostCommandConstraint struct {
	// AllowTrailingArgs corresponds to the JSON schema field "allow_trailing_args".
	AllowTrailingArgs bool `json:"allow_trailing_args,omitempty,omitzero"`

	// ArgvPrefix corresponds to the JSON schema field "argv_prefix".
	ArgvPrefix []string `json:"argv_prefix,omitempty,omitzero"`

	// Executable corresponds to the JSON schema field "executable".
	Executable string `json:"executable"`
}

type HostCommandSource string

const HostCommandSourceRepositoryCapabilityCommands HostCommandSource = "repository_capability_commands"

type HostToolBinding struct {
	// CommandSource corresponds to the JSON schema field "command_source".
	CommandSource *HostCommandSource `json:"command_source,omitempty,omitzero"`

	// Commands corresponds to the JSON schema field "commands".
	Commands []HostCommandConstraint `json:"commands,omitempty,omitzero"`

	// InputEquals corresponds to the JSON schema field "input_equals".
	InputEquals HostToolBindingInputEquals `json:"input_equals,omitempty,omitzero"`

	// ModelName corresponds to the JSON schema field "model_name".
	ModelName *string `json:"model_name,omitempty,omitzero"`

	// Name corresponds to the JSON schema field "name".
	Name string `json:"name"`

	// Observability corresponds to the JSON schema field "observability".
	Observability ToolObservability `json:"observability"`
}

type HostToolBindingInputEquals map[string]string

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

type ProviderToolBindingV1 struct {
	// ProviderBinding corresponds to the JSON schema field "provider_binding".
	ProviderBinding ProviderBindingRefV1 `json:"provider_binding"`

	// Tools corresponds to the JSON schema field "tools".
	Tools []HostToolBinding `json:"tools"`
}

type ResolvedCapabilityFallbackV1 struct {
	// Guidance corresponds to the JSON schema field "guidance".
	Guidance string `json:"guidance"`

	// Recipe corresponds to the JSON schema field "recipe".
	Recipe *PackageResourceRefV1 `json:"recipe,omitempty,omitzero"`
}

type ResolvedCapabilityProviderV1 struct {
	// Approval corresponds to the JSON schema field "approval".
	Approval Approval `json:"approval"`

	// CapabilityID corresponds to the JSON schema field "capability_id".
	CapabilityID CapabilityIDV1 `json:"capability_id"`

	// EffectClass corresponds to the JSON schema field "effect_class".
	EffectClass EffectClass `json:"effect_class"`

	// Implementation corresponds to the JSON schema field "implementation".
	Implementation CapabilityProviderBinding `json:"implementation"`

	// Note corresponds to the JSON schema field "note".
	Note ResolvedCapabilityProviderV1Note `json:"note,omitempty,omitzero"`

	// ToolContract corresponds to the JSON schema field "tool_contract".
	ToolContract *ToolContractIDV1 `json:"tool_contract,omitempty,omitzero"`
}

type ResolvedCapabilityProviderV1Note *string

type ResolvedCapabilityRequirementPolicyOptionalV1 struct {
	// Fallback corresponds to the JSON schema field "fallback".
	Fallback ResolvedCapabilityFallbackV1 `json:"fallback"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResolvedCapabilityRequirementPolicyRequiredBlockV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResolvedCapabilityRequirementPolicyRequiredDegradeV1 struct {
	// DegradedOutcome corresponds to the JSON schema field "degraded_outcome".
	DegradedOutcome DegradedOutcomeIDV1 `json:"degraded_outcome"`

	// Fallback corresponds to the JSON schema field "fallback".
	Fallback ResolvedCapabilityFallbackV1 `json:"fallback"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResolvedCapabilityUseV1 struct {
	// CapabilityID corresponds to the JSON schema field "capability_id".
	CapabilityID CapabilityIDV1 `json:"capability_id"`

	// EffectClass corresponds to the JSON schema field "effect_class".
	EffectClass EffectClass `json:"effect_class"`

	// FailurePosture corresponds to the JSON schema field "failure_posture".
	FailurePosture FailurePosture `json:"failure_posture"`

	// Kind corresponds to the JSON schema field "kind".
	Kind CapabilityUseKind `json:"kind"`

	// Owner corresponds to the JSON schema field "owner".
	Owner PackageResourceRefV1 `json:"owner"`

	// Policy corresponds to the JSON schema field "policy".
	Policy ResolvedCapabilityRequirementPolicyV1 `json:"policy"`

	// Recipe corresponds to the JSON schema field "recipe".
	Recipe *PackageResourceRefV1 `json:"recipe,omitempty,omitzero"`

	// RequirementID corresponds to the JSON schema field "requirement_id".
	RequirementID CapabilityRequirementIDV1 `json:"requirement_id"`
}

type ResolvedToolRecipeV1 struct {
	// Procedure corresponds to the JSON schema field "procedure".
	Procedure capability_procedure_v1.ToolRecipe `json:"procedure"`

	// Resource corresponds to the JSON schema field "resource".
	Resource PackageResourceRefV1 `json:"resource"`
}

type ToolContractIDV1 string

type ToolObservability string

const ToolObservabilityPermissionOnly ToolObservability = "permission_only"
const ToolObservabilityPreToolObservable ToolObservability = "pre_tool_observable"
const ToolObservabilityUnobservable ToolObservability = "unobservable"

type CapabilityProviderBinding struct {
	HostNative *CapabilityProviderBindingHostNative `json:"-"`
	NativeRust *CapabilityProviderBindingNativeRust `json:"-"`
	Connector  *CapabilityProviderBindingConnector  `json:"-"`
}

func (value CapabilityProviderBinding) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.HostNative != nil {
		if value.HostNative.Kind != "host_native" {
			return nil, fmt.Errorf("CapabilityProviderBinding: selected variant has discriminator %q", value.HostNative.Kind)
		}
		candidate, err := json.Marshal(value.HostNative)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.NativeRust != nil {
		if value.NativeRust.Kind != "native_rust" {
			return nil, fmt.Errorf("CapabilityProviderBinding: selected variant has discriminator %q", value.NativeRust.Kind)
		}
		candidate, err := json.Marshal(value.NativeRust)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Connector != nil {
		if value.Connector.Kind != "connector" {
			return nil, fmt.Errorf("CapabilityProviderBinding: selected variant has discriminator %q", value.Connector.Kind)
		}
		candidate, err := json.Marshal(value.Connector)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("CapabilityProviderBinding: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *CapabilityProviderBinding) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = CapabilityProviderBinding{}
	switch discriminator.Kind {
	case "host_native":
		var variant CapabilityProviderBindingHostNative
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.HostNative = &variant
	case "native_rust":
		var variant CapabilityProviderBindingNativeRust
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NativeRust = &variant
	case "connector":
		var variant CapabilityProviderBindingConnector
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Connector = &variant
	default:
		return fmt.Errorf("CapabilityProviderBinding: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type ResolvedCapabilityRequirementPolicyV1 struct {
	RequiredBlock   *ResolvedCapabilityRequirementPolicyRequiredBlockV1   `json:"-"`
	RequiredDegrade *ResolvedCapabilityRequirementPolicyRequiredDegradeV1 `json:"-"`
	Optional        *ResolvedCapabilityRequirementPolicyOptionalV1        `json:"-"`
}

func (value ResolvedCapabilityRequirementPolicyV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.RequiredBlock != nil {
		if value.RequiredBlock.Kind != "required_block" {
			return nil, fmt.Errorf("ResolvedCapabilityRequirementPolicyV1: selected variant has discriminator %q", value.RequiredBlock.Kind)
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
			return nil, fmt.Errorf("ResolvedCapabilityRequirementPolicyV1: selected variant has discriminator %q", value.RequiredDegrade.Kind)
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
			return nil, fmt.Errorf("ResolvedCapabilityRequirementPolicyV1: selected variant has discriminator %q", value.Optional.Kind)
		}
		candidate, err := json.Marshal(value.Optional)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ResolvedCapabilityRequirementPolicyV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ResolvedCapabilityRequirementPolicyV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ResolvedCapabilityRequirementPolicyV1{}
	switch discriminator.Kind {
	case "required_block":
		var variant ResolvedCapabilityRequirementPolicyRequiredBlockV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.RequiredBlock = &variant
	case "required_degrade":
		var variant ResolvedCapabilityRequirementPolicyRequiredDegradeV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.RequiredDegrade = &variant
	case "optional":
		var variant ResolvedCapabilityRequirementPolicyOptionalV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Optional = &variant
	default:
		return fmt.Errorf("ResolvedCapabilityRequirementPolicyV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
