// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package operation_contract_v1 contains the generated operation-contract.v1 data model.
package operation_contract_v1

import (
	"encoding/json"
	"fmt"
	sealed_artifact_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sealed_artifact_v1"
)
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type Approval string

const ApprovalAsk Approval = "ask"
const ApprovalGated Approval = "gated"
const ApprovalNone Approval = "none"

type CapabilityIDV1 string

type CapabilityRequirementIDV1 string

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

type HostedProviderExecutionAvailableV1 struct {
	// HostTools corresponds to the JSON schema field "host_tools".
	HostTools []HostToolBinding `json:"host_tools"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type HostedProviderExecutionUnavailableV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type HostedProviderExecutionUnknownV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type NativeRustProviderExecutionAvailableV1 struct {
	// HostTools corresponds to the JSON schema field "host_tools".
	HostTools []HostToolBinding `json:"host_tools"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// NativeAction corresponds to the JSON schema field "native_action".
	NativeAction string `json:"native_action"`
}

type NativeRustProviderExecutionUnavailableV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type NativeRustProviderExecutionUnknownV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type OperationCapabilityContractV1 struct {
	// CapabilityID corresponds to the JSON schema field "capability_id".
	CapabilityID CapabilityIDV1 `json:"capability_id"`

	// EffectClass corresponds to the JSON schema field "effect_class".
	EffectClass EffectClass `json:"effect_class"`

	// FailurePosture corresponds to the JSON schema field "failure_posture".
	FailurePosture FailurePosture `json:"failure_posture"`

	// Policy corresponds to the JSON schema field "policy".
	Policy ResolvedCapabilityRequirementPolicyV1 `json:"policy"`

	// ProviderState corresponds to the JSON schema field "provider_state".
	ProviderState OperationCapabilityProviderStateV1 `json:"provider_state"`

	// Recipe corresponds to the JSON schema field "recipe".
	Recipe *PackageResourceRefV1 `json:"recipe,omitempty,omitzero"`

	// RequirementID corresponds to the JSON schema field "requirement_id".
	RequirementID CapabilityRequirementIDV1 `json:"requirement_id"`
}

type OperationCapabilityProviderStateBoundV1 struct {
	// Approval corresponds to the JSON schema field "approval".
	Approval Approval `json:"approval"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Note corresponds to the JSON schema field "note".
	Note OperationCapabilityProviderStateV1_1Note `json:"note,omitempty,omitzero"`

	// Provider corresponds to the JSON schema field "provider".
	Provider RouteCapabilityProviderV1 `json:"provider"`
}

type OperationCapabilityProviderStateUnboundV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type OperationCapabilityProviderStateV1_1Note *string

type OperationContractMechanicsCapabilityBearingV1 struct {
	// Capabilities corresponds to the JSON schema field "capabilities".
	Capabilities []OperationCapabilityContractV1 `json:"capabilities"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type OperationContractMechanicsSemanticOnlyV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type OperationContractV1 struct {
	// AuxiliaryTools corresponds to the JSON schema field "auxiliary_tools".
	AuxiliaryTools []HostToolBinding `json:"auxiliary_tools,omitempty,omitzero"`

	// ContractDigest corresponds to the JSON schema field "contract_digest".
	ContractDigest sha256_digest_v1.Sha256DigestV1 `json:"contract_digest"`

	// Dispatch corresponds to the JSON schema field "dispatch".
	Dispatch OperationDispatchV1 `json:"dispatch"`

	// ID corresponds to the JSON schema field "id".
	ID PackageResourceRefV1 `json:"id"`

	// InvocationPolicy corresponds to the JSON schema field "invocation_policy".
	InvocationPolicy *sealed_artifact_v1.SealedArtifactV1 `json:"invocation_policy,omitempty,omitzero"`

	// Mechanics corresponds to the JSON schema field "mechanics".
	Mechanics OperationContractMechanicsV1 `json:"mechanics"`

	// Roles corresponds to the JSON schema field "roles".
	Roles []PackageResourceRefV1 `json:"roles,omitempty,omitzero"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// SemanticDependencies corresponds to the JSON schema field
	// "semantic_dependencies".
	SemanticDependencies []OperationSemanticDependencyV1 `json:"semantic_dependencies,omitempty,omitzero"`

	// Skill corresponds to the JSON schema field "skill".
	Skill sealed_artifact_v1.SealedArtifactV1 `json:"skill"`
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

type OperationSemanticDependencyV1 struct {
	// Artifact corresponds to the JSON schema field "artifact".
	Artifact sealed_artifact_v1.SealedArtifactV1 `json:"artifact"`

	// ID corresponds to the JSON schema field "id".
	ID PackageResourceRefV1 `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind SemanticDependencyKind `json:"kind"`
}

type PackageIDV1 string

type PackageResourceIDV1 string

type PackageResourceRefV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`
}

type ResolvedCapabilityFallbackV1 struct {
	// Guidance corresponds to the JSON schema field "guidance".
	Guidance string `json:"guidance"`

	// Recipe corresponds to the JSON schema field "recipe".
	Recipe *PackageResourceRefV1 `json:"recipe,omitempty,omitzero"`
}

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

type RouteCapabilityProviderConnectorV1 struct {
	// Execution corresponds to the JSON schema field "execution".
	Execution HostedProviderExecutionV1 `json:"execution"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Target corresponds to the JSON schema field "target".
	Target string `json:"target"`
}

type RouteCapabilityProviderHostNativeV1 struct {
	// Execution corresponds to the JSON schema field "execution".
	Execution HostedProviderExecutionV1 `json:"execution"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type RouteCapabilityProviderNativeRustV1 struct {
	// Execution corresponds to the JSON schema field "execution".
	Execution NativeRustProviderExecutionV1 `json:"execution"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Target corresponds to the JSON schema field "target".
	Target string `json:"target"`
}

type SemanticDependencyKind string

const SemanticDependencyKindDoctrine SemanticDependencyKind = "doctrine"
const SemanticDependencyKindRole SemanticDependencyKind = "role"
const SemanticDependencyKindTool SemanticDependencyKind = "tool"

type ToolObservability string

const ToolObservabilityPermissionOnly ToolObservability = "permission_only"
const ToolObservabilityPreToolObservable ToolObservability = "pre_tool_observable"
const ToolObservabilityUnobservable ToolObservability = "unobservable"

type HostedProviderExecutionV1 struct {
	Unknown     *HostedProviderExecutionUnknownV1     `json:"-"`
	Unavailable *HostedProviderExecutionUnavailableV1 `json:"-"`
	Available   *HostedProviderExecutionAvailableV1   `json:"-"`
}

func (value HostedProviderExecutionV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Unknown != nil {
		if value.Unknown.Kind != "unknown" {
			return nil, fmt.Errorf("HostedProviderExecutionV1: selected variant has discriminator %q", value.Unknown.Kind)
		}
		candidate, err := json.Marshal(value.Unknown)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Unavailable != nil {
		if value.Unavailable.Kind != "unavailable" {
			return nil, fmt.Errorf("HostedProviderExecutionV1: selected variant has discriminator %q", value.Unavailable.Kind)
		}
		candidate, err := json.Marshal(value.Unavailable)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Available != nil {
		if value.Available.Kind != "available" {
			return nil, fmt.Errorf("HostedProviderExecutionV1: selected variant has discriminator %q", value.Available.Kind)
		}
		candidate, err := json.Marshal(value.Available)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("HostedProviderExecutionV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *HostedProviderExecutionV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = HostedProviderExecutionV1{}
	switch discriminator.Kind {
	case "unknown":
		var variant HostedProviderExecutionUnknownV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unknown = &variant
	case "unavailable":
		var variant HostedProviderExecutionUnavailableV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unavailable = &variant
	case "available":
		var variant HostedProviderExecutionAvailableV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Available = &variant
	default:
		return fmt.Errorf("HostedProviderExecutionV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type NativeRustProviderExecutionV1 struct {
	Unknown     *NativeRustProviderExecutionUnknownV1     `json:"-"`
	Unavailable *NativeRustProviderExecutionUnavailableV1 `json:"-"`
	Available   *NativeRustProviderExecutionAvailableV1   `json:"-"`
}

func (value NativeRustProviderExecutionV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Unknown != nil {
		if value.Unknown.Kind != "unknown" {
			return nil, fmt.Errorf("NativeRustProviderExecutionV1: selected variant has discriminator %q", value.Unknown.Kind)
		}
		candidate, err := json.Marshal(value.Unknown)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Unavailable != nil {
		if value.Unavailable.Kind != "unavailable" {
			return nil, fmt.Errorf("NativeRustProviderExecutionV1: selected variant has discriminator %q", value.Unavailable.Kind)
		}
		candidate, err := json.Marshal(value.Unavailable)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Available != nil {
		if value.Available.Kind != "available" {
			return nil, fmt.Errorf("NativeRustProviderExecutionV1: selected variant has discriminator %q", value.Available.Kind)
		}
		candidate, err := json.Marshal(value.Available)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("NativeRustProviderExecutionV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *NativeRustProviderExecutionV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = NativeRustProviderExecutionV1{}
	switch discriminator.Kind {
	case "unknown":
		var variant NativeRustProviderExecutionUnknownV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unknown = &variant
	case "unavailable":
		var variant NativeRustProviderExecutionUnavailableV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unavailable = &variant
	case "available":
		var variant NativeRustProviderExecutionAvailableV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Available = &variant
	default:
		return fmt.Errorf("NativeRustProviderExecutionV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type OperationCapabilityProviderStateV1 struct {
	Unbound *OperationCapabilityProviderStateUnboundV1 `json:"-"`
	Bound   *OperationCapabilityProviderStateBoundV1   `json:"-"`
}

func (value OperationCapabilityProviderStateV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Unbound != nil {
		if value.Unbound.Kind != "unbound" {
			return nil, fmt.Errorf("OperationCapabilityProviderStateV1: selected variant has discriminator %q", value.Unbound.Kind)
		}
		candidate, err := json.Marshal(value.Unbound)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Bound != nil {
		if value.Bound.Kind != "bound" {
			return nil, fmt.Errorf("OperationCapabilityProviderStateV1: selected variant has discriminator %q", value.Bound.Kind)
		}
		candidate, err := json.Marshal(value.Bound)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("OperationCapabilityProviderStateV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *OperationCapabilityProviderStateV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = OperationCapabilityProviderStateV1{}
	switch discriminator.Kind {
	case "unbound":
		var variant OperationCapabilityProviderStateUnboundV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unbound = &variant
	case "bound":
		var variant OperationCapabilityProviderStateBoundV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Bound = &variant
	default:
		return fmt.Errorf("OperationCapabilityProviderStateV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type OperationContractMechanicsV1 struct {
	SemanticOnly      *OperationContractMechanicsSemanticOnlyV1      `json:"-"`
	CapabilityBearing *OperationContractMechanicsCapabilityBearingV1 `json:"-"`
}

func (value OperationContractMechanicsV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.SemanticOnly != nil {
		if value.SemanticOnly.Kind != "semantic_only" {
			return nil, fmt.Errorf("OperationContractMechanicsV1: selected variant has discriminator %q", value.SemanticOnly.Kind)
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
			return nil, fmt.Errorf("OperationContractMechanicsV1: selected variant has discriminator %q", value.CapabilityBearing.Kind)
		}
		candidate, err := json.Marshal(value.CapabilityBearing)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("OperationContractMechanicsV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *OperationContractMechanicsV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = OperationContractMechanicsV1{}
	switch discriminator.Kind {
	case "semantic_only":
		var variant OperationContractMechanicsSemanticOnlyV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.SemanticOnly = &variant
	case "capability_bearing":
		var variant OperationContractMechanicsCapabilityBearingV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.CapabilityBearing = &variant
	default:
		return fmt.Errorf("OperationContractMechanicsV1: unknown discriminator %q", discriminator.Kind)
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

type RouteCapabilityProviderV1 struct {
	HostNative *RouteCapabilityProviderHostNativeV1 `json:"-"`
	NativeRust *RouteCapabilityProviderNativeRustV1 `json:"-"`
	Connector  *RouteCapabilityProviderConnectorV1  `json:"-"`
}

func (value RouteCapabilityProviderV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.HostNative != nil {
		if value.HostNative.Kind != "host_native" {
			return nil, fmt.Errorf("RouteCapabilityProviderV1: selected variant has discriminator %q", value.HostNative.Kind)
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
			return nil, fmt.Errorf("RouteCapabilityProviderV1: selected variant has discriminator %q", value.NativeRust.Kind)
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
			return nil, fmt.Errorf("RouteCapabilityProviderV1: selected variant has discriminator %q", value.Connector.Kind)
		}
		candidate, err := json.Marshal(value.Connector)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("RouteCapabilityProviderV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *RouteCapabilityProviderV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = RouteCapabilityProviderV1{}
	switch discriminator.Kind {
	case "host_native":
		var variant RouteCapabilityProviderHostNativeV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.HostNative = &variant
	case "native_rust":
		var variant RouteCapabilityProviderNativeRustV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NativeRust = &variant
	case "connector":
		var variant RouteCapabilityProviderConnectorV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Connector = &variant
	default:
		return fmt.Errorf("RouteCapabilityProviderV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
