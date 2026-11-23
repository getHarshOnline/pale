// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package capabilities_v1 contains the generated capabilities.v1 data model.
package capabilities_v1

import (
	"encoding/json"
	"fmt"
	lifecycle_event_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/lifecycle_event_v1"
)
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import tool_call_matcher_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/tool_call_matcher_v1"

type Approval string

const ApprovalAsk Approval = "ask"
const ApprovalGated Approval = "gated"
const ApprovalNone Approval = "none"

type BoundaryCapability struct {
	// ForbiddenTokenAuthority corresponds to the JSON schema field
	// "forbidden_token_authority".
	ForbiddenTokenAuthority []string `json:"forbidden_token_authority"`
}

type CapabilitiesV1 struct {
	// Boundary corresponds to the JSON schema field "boundary".
	Boundary *BoundaryCapability `json:"boundary,omitempty,omitzero"`

	// CapabilityBindings corresponds to the JSON schema field "capability_bindings".
	CapabilityBindings CapabilitiesV1CapabilityBindings `json:"capability_bindings,omitempty,omitzero"`

	// CapabilityNarrowing corresponds to the JSON schema field
	// "capability_narrowing".
	CapabilityNarrowing *CapabilityNarrowingV1 `json:"capability_narrowing,omitempty,omitzero"`

	// Lifecycle corresponds to the JSON schema field "lifecycle".
	Lifecycle *LifecycleBehaviorCatalog `json:"lifecycle,omitempty,omitzero"`

	// OperationBindings corresponds to the JSON schema field "operation_bindings".
	OperationBindings []OperationBindingEntryV1 `json:"operation_bindings,omitempty,omitzero"`

	// RepositoryCommands corresponds to the JSON schema field "repository_commands".
	RepositoryCommands CapabilitiesV1RepositoryCommands `json:"repository_commands,omitempty,omitzero"`

	// RunPolicies corresponds to the JSON schema field "run_policies".
	RunPolicies []RunPolicyEntry `json:"run_policies,omitempty,omitzero"`

	// Semantic corresponds to the JSON schema field "semantic".
	Semantic SemanticSelection `json:"semantic,omitempty,omitzero"`
}

type CapabilitiesV1CapabilityBindings map[string]CapabilityBinding

type CapabilitiesV1RepositoryCommands map[string]RepositoryCommandCapabilities

type CapabilityBinding struct {
	// Approval corresponds to the JSON schema field "approval".
	Approval Approval `json:"approval"`

	// EffectClass corresponds to the JSON schema field "effect_class".
	EffectClass EffectClass `json:"effect_class"`

	// Implementation corresponds to the JSON schema field "implementation".
	Implementation CapabilityProviderBinding `json:"implementation"`

	// Note corresponds to the JSON schema field "note".
	Note CapabilityBindingNote `json:"note,omitempty,omitzero"`

	// ToolContract corresponds to the JSON schema field "tool_contract".
	ToolContract *ToolContractIDV1 `json:"tool_contract,omitempty,omitzero"`
}

type CapabilityBindingNote *string

type CapabilityIDV1 string

type CapabilityNarrowingV1 struct {
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

type ChildPropagation string

const ChildPropagationExempt ChildPropagation = "exempt"
const ChildPropagationNone ChildPropagation = "none"
const ChildPropagationPropagate ChildPropagation = "propagate"

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

type LifecycleAction string

const LifecycleActionActivate LifecycleAction = "activate"
const LifecycleActionAsk LifecycleAction = "ask"
const LifecycleActionCapture LifecycleAction = "capture"
const LifecycleActionDeactivate LifecycleAction = "deactivate"
const LifecycleActionDeny LifecycleAction = "deny"
const LifecycleActionGuide LifecycleAction = "guide"
const LifecycleActionInject LifecycleAction = "inject"
const LifecycleActionInjectModes LifecycleAction = "inject_modes"
const LifecycleActionNoOp LifecycleAction = "no_op"
const LifecycleActionNotify LifecycleAction = "notify"
const LifecycleActionRebaseline LifecycleAction = "rebaseline"
const LifecycleActionSignal LifecycleAction = "signal"
const LifecycleActionWarn LifecycleAction = "warn"

type LifecycleBehavior struct {
	// Action corresponds to the JSON schema field "action".
	Action LifecycleAction `json:"action"`

	// ActivateMode corresponds to the JSON schema field "activate_mode".
	ActivateMode *PackageResourceRefV1 `json:"activate_mode,omitempty,omitzero"`

	// AllowAdmittedOperation corresponds to the JSON schema field
	// "allow_admitted_operation".
	AllowAdmittedOperation bool `json:"allow_admitted_operation,omitempty,omitzero"`

	// CaseInsensitive corresponds to the JSON schema field "case_insensitive".
	CaseInsensitive bool `json:"case_insensitive,omitempty,omitzero"`

	// Description corresponds to the JSON schema field "description".
	Description string `json:"description"`

	// Event corresponds to the JSON schema field "event".
	Event lifecycle_event_v1.LifecycleEventV1 `json:"event"`

	// ExistingTargetsOnly corresponds to the JSON schema field
	// "existing_targets_only".
	ExistingTargetsOnly bool `json:"existing_targets_only,omitempty,omitzero"`

	// FailurePosture corresponds to the JSON schema field "failure_posture".
	FailurePosture FailurePosture `json:"failure_posture,omitempty,omitzero"`

	// Fallback corresponds to the JSON schema field "fallback".
	Fallback *LifecycleFallbackV1 `json:"fallback,omitempty,omitzero"`

	// GovernedBy corresponds to the JSON schema field "governed_by".
	GovernedBy LifecycleBehaviorGovernedBy `json:"governed_by,omitempty,omitzero"`

	// Guidance corresponds to the JSON schema field "guidance".
	Guidance *OwnedConfinedRelativePath `json:"guidance,omitempty,omitzero"`

	// ID corresponds to the JSON schema field "id".
	ID LifecycleBehaviorIDV1 `json:"id"`

	// Matchers corresponds to the JSON schema field "matchers".
	Matchers []string `json:"matchers,omitempty,omitzero"`

	// Modes corresponds to the JSON schema field "modes".
	Modes []PackageResourceRefV1 `json:"modes,omitempty,omitzero"`

	// PredicateKind corresponds to the JSON schema field "predicate_kind".
	PredicateKind PredicateKind `json:"predicate_kind,omitempty,omitzero"`

	// StateScope corresponds to the JSON schema field "state_scope".
	StateScope StateScope `json:"state_scope,omitempty,omitzero"`

	// TimeoutMs corresponds to the JSON schema field "timeout_ms".
	TimeoutMs LifecycleBehaviorTimeoutMs `json:"timeout_ms,omitempty,omitzero"`

	// ToolReqs corresponds to the JSON schema field "tool_reqs".
	ToolReqs []CapabilityIDV1 `json:"tool_reqs,omitempty,omitzero"`
}

type LifecycleBehaviorCatalog struct {
	// Behaviour corresponds to the JSON schema field "behaviour".
	Behaviour []LifecycleBehavior `json:"behaviour"`
}

type LifecycleBehaviorGovernedBy *string

type LifecycleBehaviorIDV1 string

type LifecycleBehaviorTimeoutMs *int

type LifecycleFallbackV1 struct {
	// CarrierEvent corresponds to the JSON schema field "carrier_event".
	CarrierEvent lifecycle_event_v1.LifecycleEventV1 `json:"carrier_event"`

	// Guidance corresponds to the JSON schema field "guidance".
	Guidance string `json:"guidance"`
}

type NonEmptyCommandList []string

type OperationBinding struct {
	// Selection corresponds to the JSON schema field "selection".
	Selection OperationSelection `json:"selection"`
}

type OperationBindingEntryV1 struct {
	// Binding corresponds to the JSON schema field "binding".
	Binding OperationBinding `json:"binding"`

	// Operation corresponds to the JSON schema field "operation".
	Operation PackageResourceRefV1 `json:"operation"`
}

type OperationSelectionInternal struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type OperationSelectionModelOrUser struct {
	// Aliases corresponds to the JSON schema field "aliases".
	Aliases []string `json:"aliases,omitempty,omitzero"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type OperationSelectionUserOnly struct {
	// Aliases corresponds to the JSON schema field "aliases".
	Aliases []string `json:"aliases,omitempty,omitzero"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type OwnedConfinedRelativePath string

type PackageIDV1 string

type PackageResourceIDV1 string

type PackageResourceRefV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`
}

type PackageResourceSelectorAllFromPackageV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// Selection corresponds to the JSON schema field "selection".
	Selection string `json:"selection"`
}

type PackageResourceSelectorExactV1 struct {
	// Resource corresponds to the JSON schema field "resource".
	Resource PackageResourceRefV1 `json:"resource"`

	// Selection corresponds to the JSON schema field "selection".
	Selection string `json:"selection"`
}

type PredicateKind string

const PredicateKindAlways PredicateKind = "always"
const PredicateKindCommandPrefix PredicateKind = "command_prefix"
const PredicateKindConnectorBound PredicateKind = "connector_bound"
const PredicateKindDestructiveShell PredicateKind = "destructive_shell"
const PredicateKindExactMessage PredicateKind = "exact_message"
const PredicateKindFirstTouch PredicateKind = "first_touch"
const PredicateKindGitHookBypass PredicateKind = "git_hook_bypass"
const PredicateKindModeChanged PredicateKind = "mode_changed"
const PredicateKindNever PredicateKind = "never"
const PredicateKindPathMatch PredicateKind = "path_match"
const PredicateKindRebaselinePending PredicateKind = "rebaseline_pending"
const PredicateKindToolMatch PredicateKind = "tool_match"
const PredicateKindWriteEffect PredicateKind = "write_effect"

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

type RepositoryCommandCapabilities struct {
	// Commands corresponds to the JSON schema field "commands".
	Commands RepositoryCommandCapabilitiesCommands `json:"commands"`

	// MutatingNotReadonly corresponds to the JSON schema field
	// "mutating_not_readonly".
	MutatingNotReadonly []string `json:"mutating_not_readonly,omitempty,omitzero"`

	// Note corresponds to the JSON schema field "note".
	Note *string `json:"note,omitempty,omitzero"`
}

type RepositoryCommandCapabilitiesCommands map[string]NonEmptyCommandList

type RunPolicyEntry struct {
	// ActionGate corresponds to the JSON schema field "action_gate".
	ActionGate bool `json:"action_gate,omitempty,omitzero"`

	// ChildPropagation corresponds to the JSON schema field "child_propagation".
	ChildPropagation ChildPropagation `json:"child_propagation,omitempty,omitzero"`

	// ChildTypeSelectors corresponds to the JSON schema field "child_type_selectors".
	ChildTypeSelectors []string `json:"child_type_selectors,omitempty,omitzero"`

	// Description corresponds to the JSON schema field "description".
	Description string `json:"description"`

	// GatedToolMatchers corresponds to the JSON schema field "gated_tool_matchers".
	GatedToolMatchers []tool_call_matcher_v1.ToolCallMatcherV1 `json:"gated_tool_matchers,omitempty,omitzero"`

	// ID corresponds to the JSON schema field "id".
	ID RunPolicyIDV1 `json:"id"`

	// SessionInject corresponds to the JSON schema field "session_inject".
	SessionInject bool `json:"session_inject,omitempty,omitzero"`
}

type RunPolicyIDV1 string

type SemanticSelection struct {
	// Doctrine corresponds to the JSON schema field "doctrine".
	Doctrine []PackageResourceSelectorV1 `json:"doctrine,omitempty,omitzero"`

	// ModeDefaults corresponds to the JSON schema field "mode_defaults".
	ModeDefaults SemanticSelectionModeDefaults `json:"mode_defaults,omitempty,omitzero"`

	// Operations corresponds to the JSON schema field "operations".
	Operations []PackageResourceSelectorV1 `json:"operations,omitempty,omitzero"`

	// Roles corresponds to the JSON schema field "roles".
	Roles []PackageResourceSelectorV1 `json:"roles,omitempty,omitzero"`

	// RunModes corresponds to the JSON schema field "run_modes".
	RunModes []PackageResourceSelectorV1 `json:"run_modes,omitempty,omitzero"`

	// Stacks corresponds to the JSON schema field "stacks".
	Stacks []PackageResourceSelectorV1 `json:"stacks,omitempty,omitzero"`
}

type SemanticSelectionModeDefaults map[string]PackageResourceRefV1

type StateScope string

const StateScopeDurable StateScope = "durable"
const StateScopeMessage StateScope = "message"
const StateScopeSession StateScope = "session"
const StateScopeStateless StateScope = "stateless"

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

type OperationSelection struct {
	ModelOrUser *OperationSelectionModelOrUser `json:"-"`
	UserOnly    *OperationSelectionUserOnly    `json:"-"`
	Internal    *OperationSelectionInternal    `json:"-"`
}

func (value OperationSelection) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.ModelOrUser != nil {
		if value.ModelOrUser.Kind != "model_or_user" {
			return nil, fmt.Errorf("OperationSelection: selected variant has discriminator %q", value.ModelOrUser.Kind)
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
			return nil, fmt.Errorf("OperationSelection: selected variant has discriminator %q", value.UserOnly.Kind)
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
			return nil, fmt.Errorf("OperationSelection: selected variant has discriminator %q", value.Internal.Kind)
		}
		candidate, err := json.Marshal(value.Internal)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("OperationSelection: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *OperationSelection) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = OperationSelection{}
	switch discriminator.Kind {
	case "model_or_user":
		var variant OperationSelectionModelOrUser
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ModelOrUser = &variant
	case "user_only":
		var variant OperationSelectionUserOnly
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.UserOnly = &variant
	case "internal":
		var variant OperationSelectionInternal
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Internal = &variant
	default:
		return fmt.Errorf("OperationSelection: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type PackageResourceSelectorV1 struct {
	Exact          *PackageResourceSelectorExactV1          `json:"-"`
	AllFromPackage *PackageResourceSelectorAllFromPackageV1 `json:"-"`
}

func (value PackageResourceSelectorV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Exact != nil {
		if value.Exact.Selection != "exact" {
			return nil, fmt.Errorf("PackageResourceSelectorV1: selected variant has discriminator %q", value.Exact.Selection)
		}
		candidate, err := json.Marshal(value.Exact)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.AllFromPackage != nil {
		if value.AllFromPackage.Selection != "all_from_package" {
			return nil, fmt.Errorf("PackageResourceSelectorV1: selected variant has discriminator %q", value.AllFromPackage.Selection)
		}
		candidate, err := json.Marshal(value.AllFromPackage)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("PackageResourceSelectorV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *PackageResourceSelectorV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Selection string `json:"selection"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = PackageResourceSelectorV1{}
	switch discriminator.Selection {
	case "exact":
		var variant PackageResourceSelectorExactV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Exact = &variant
	case "all_from_package":
		var variant PackageResourceSelectorAllFromPackageV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.AllFromPackage = &variant
	default:
		return fmt.Errorf("PackageResourceSelectorV1: unknown discriminator %q", discriminator.Selection)
	}
	return nil
}
