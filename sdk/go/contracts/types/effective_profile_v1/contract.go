// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package effective_profile_v1 contains the generated effective-profile.v1 data model.
package effective_profile_v1

import (
	"encoding/json"
	"fmt"
	lifecycle_event_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/lifecycle_event_v1"
)
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import tool_call_matcher_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/tool_call_matcher_v1"

type AccumulatedGoverningObligationV1 struct {
	// Contributors corresponds to the JSON schema field "contributors".
	Contributors []PackageResourceRefV1 `json:"contributors"`

	// Obligation corresponds to the JSON schema field "obligation".
	Obligation PackageResourceUseV1 `json:"obligation"`

	// Scope corresponds to the JSON schema field "scope".
	Scope PackageResourceUseV1 `json:"scope"`

	// Subject corresponds to the JSON schema field "subject".
	Subject PackageResourceUseV1 `json:"subject"`
}

type AdmittedCapabilityPolicyV1 struct {
	// ForbiddenCapabilities corresponds to the JSON schema field
	// "forbidden_capabilities".
	ForbiddenCapabilities []CapabilityIDV1 `json:"forbidden_capabilities"`

	// ForbiddenEffects corresponds to the JSON schema field "forbidden_effects".
	ForbiddenEffects []EffectClass `json:"forbidden_effects"`

	// PermittedCapabilities corresponds to the JSON schema field
	// "permitted_capabilities".
	PermittedCapabilities []CapabilityIDV1 `json:"permitted_capabilities"`

	// PermittedEffects corresponds to the JSON schema field "permitted_effects".
	PermittedEffects []EffectClass `json:"permitted_effects"`
}

type AdmittedExclusiveAuthorityClaimV1 struct {
	// Authority corresponds to the JSON schema field "authority".
	Authority PackageResourceUseV1 `json:"authority"`

	// Claimant corresponds to the JSON schema field "claimant".
	Claimant PackageResourceRefV1 `json:"claimant"`

	// Scope corresponds to the JSON schema field "scope".
	Scope PackageResourceUseV1 `json:"scope"`
}

type AdmittedGoverningOrderV1 struct {
	// Ceilings corresponds to the JSON schema field "ceilings".
	Ceilings []EffectiveGoverningCeilingV1 `json:"ceilings"`

	// ExclusiveAuthority corresponds to the JSON schema field "exclusive_authority".
	ExclusiveAuthority []AdmittedExclusiveAuthorityClaimV1 `json:"exclusive_authority"`

	// Narrowing corresponds to the JSON schema field "narrowing".
	Narrowing []ResolvedGoverningNarrowingV1 `json:"narrowing"`

	// Obligations corresponds to the JSON schema field "obligations".
	Obligations []AccumulatedGoverningObligationV1 `json:"obligations"`

	// Order corresponds to the JSON schema field "order".
	Order []AdmittedScopedOrderRelationV1 `json:"order"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type AdmittedLifecycleBehaviorV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action LifecycleAction `json:"action"`

	// ActivateMode corresponds to the JSON schema field "activate_mode".
	ActivateMode *PackageResourceRefV1 `json:"activate_mode,omitempty,omitzero"`

	// AllowAdmittedOperation corresponds to the JSON schema field
	// "allow_admitted_operation".
	AllowAdmittedOperation bool `json:"allow_admitted_operation"`

	// CaseInsensitive corresponds to the JSON schema field "case_insensitive".
	CaseInsensitive bool `json:"case_insensitive"`

	// Description corresponds to the JSON schema field "description".
	Description string `json:"description"`

	// Event corresponds to the JSON schema field "event".
	Event lifecycle_event_v1.LifecycleEventV1 `json:"event"`

	// ExistingTargetsOnly corresponds to the JSON schema field
	// "existing_targets_only".
	ExistingTargetsOnly bool `json:"existing_targets_only"`

	// FailurePosture corresponds to the JSON schema field "failure_posture".
	FailurePosture FailurePosture `json:"failure_posture"`

	// Fallback corresponds to the JSON schema field "fallback".
	Fallback *LifecycleFallbackV1 `json:"fallback,omitempty,omitzero"`

	// GovernedBy corresponds to the JSON schema field "governed_by".
	GovernedBy AdmittedLifecycleBehaviorV1GovernedBy `json:"governed_by,omitempty,omitzero"`

	// Guidance corresponds to the JSON schema field "guidance".
	Guidance AdmittedLifecycleBehaviorV1Guidance `json:"guidance,omitempty,omitzero"`

	// ID corresponds to the JSON schema field "id".
	ID LifecycleBehaviorIDV1 `json:"id"`

	// Matchers corresponds to the JSON schema field "matchers".
	Matchers []string `json:"matchers"`

	// Modes corresponds to the JSON schema field "modes".
	Modes []PackageResourceRefV1 `json:"modes"`

	// PredicateKind corresponds to the JSON schema field "predicate_kind".
	PredicateKind PredicateKind `json:"predicate_kind"`

	// StateScope corresponds to the JSON schema field "state_scope".
	StateScope StateScope `json:"state_scope"`

	// TimeoutMs corresponds to the JSON schema field "timeout_ms".
	TimeoutMs AdmittedLifecycleBehaviorV1TimeoutMs `json:"timeout_ms,omitempty,omitzero"`

	// ToolReqs corresponds to the JSON schema field "tool_reqs".
	ToolReqs []CapabilityIDV1 `json:"tool_reqs"`
}

type AdmittedLifecycleBehaviorV1GovernedBy *string

type AdmittedLifecycleBehaviorV1Guidance *string

type AdmittedLifecycleBehaviorV1TimeoutMs *int

type AdmittedScopedOrderRelationV1 struct {
	// After corresponds to the JSON schema field "after".
	After PackageResourceUseV1 `json:"after"`

	// Before corresponds to the JSON schema field "before".
	Before PackageResourceUseV1 `json:"before"`

	// Contributor corresponds to the JSON schema field "contributor".
	Contributor PackageResourceRefV1 `json:"contributor"`

	// Purpose corresponds to the JSON schema field "purpose".
	Purpose SemanticOrderPurposeV1 `json:"purpose"`

	// Scope corresponds to the JSON schema field "scope".
	Scope PackageResourceUseV1 `json:"scope"`
}

type Approval string

const ApprovalAsk Approval = "ask"
const ApprovalGated Approval = "gated"
const ApprovalNone Approval = "none"

type AuthorityAccess struct {
	// Class corresponds to the JSON schema field "class".
	Class AuthorityAccessClass `json:"class"`

	// RootID corresponds to the JSON schema field "root_id".
	RootID string `json:"root_id"`
}

type AuthorityAccessClass string

const AuthorityAccessClassReadOnly AuthorityAccessClass = "read-only"

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

type EffectiveGoverningCeilingV1 struct {
	// Contributors corresponds to the JSON schema field "contributors".
	Contributors []PackageResourceRefV1 `json:"contributors"`

	// Policies corresponds to the JSON schema field "policies".
	Policies []PackageResourceRefV1 `json:"policies"`

	// Policy corresponds to the JSON schema field "policy".
	Policy AdmittedCapabilityPolicyV1 `json:"policy"`

	// Scope corresponds to the JSON schema field "scope".
	Scope PackageResourceUseV1 `json:"scope"`

	// Subject corresponds to the JSON schema field "subject".
	Subject PackageResourceUseV1 `json:"subject"`
}

type EffectiveProfileV1 struct {
	// AuthorityRootIds corresponds to the JSON schema field "authority_root_ids".
	AuthorityRootIds []string `json:"authority_root_ids"`

	// CapabilityBindings corresponds to the JSON schema field "capability_bindings".
	CapabilityBindings EffectiveProfileV1CapabilityBindings `json:"capability_bindings,omitempty,omitzero"`

	// CapabilityNarrowing corresponds to the JSON schema field
	// "capability_narrowing".
	CapabilityNarrowing *CapabilityNarrowingV1 `json:"capability_narrowing,omitempty,omitzero"`

	// ExecutionPaths corresponds to the JSON schema field "execution_paths".
	ExecutionPaths []ExecutionPathSelectionV1 `json:"execution_paths"`

	// ExecutionWorkspaceID corresponds to the JSON schema field
	// "execution_workspace_id".
	ExecutionWorkspaceID LogicalExecutionWorkspaceIDV1 `json:"execution_workspace_id"`

	// ForbiddenTokenAuthority corresponds to the JSON schema field
	// "forbidden_token_authority".
	ForbiddenTokenAuthority []string `json:"forbidden_token_authority,omitempty,omitzero"`

	// GoverningOrder corresponds to the JSON schema field "governing_order".
	GoverningOrder AdmittedGoverningOrderV1 `json:"governing_order"`

	// Lifecycle corresponds to the JSON schema field "lifecycle".
	Lifecycle []AdmittedLifecycleBehaviorV1 `json:"lifecycle,omitempty,omitzero"`

	// NormativePackClosure corresponds to the JSON schema field
	// "normative_pack_closure".
	NormativePackClosure []NormativePackReleaseV1 `json:"normative_pack_closure"`

	// OperationBindings corresponds to the JSON schema field "operation_bindings".
	OperationBindings []OperationBindingEntryV1 `json:"operation_bindings,omitempty,omitzero"`

	// PackageClosure corresponds to the JSON schema field "package_closure".
	PackageClosure []PackageReleaseV1 `json:"package_closure"`

	// PackageSemanticIrDigest corresponds to the JSON schema field
	// "package_semantic_ir_digest".
	PackageSemanticIrDigest sha256_digest_v1.Sha256DigestV1 `json:"package_semantic_ir_digest"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// ProjectID corresponds to the JSON schema field "project_id".
	ProjectID LogicalProjectIDV1 `json:"project_id"`

	// Repositories corresponds to the JSON schema field "repositories".
	Repositories []ProfileRepository `json:"repositories"`

	// RunPolicy corresponds to the JSON schema field "run_policy".
	RunPolicy RunPolicyEntry `json:"run_policy"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// Semantic corresponds to the JSON schema field "semantic".
	Semantic SemanticSelection `json:"semantic,omitempty,omitzero"`

	// Session corresponds to the JSON schema field "session".
	Session *SessionModel `json:"session,omitempty,omitzero"`

	// WorkspaceID corresponds to the JSON schema field "workspace_id".
	WorkspaceID WorkspaceID `json:"workspace_id"`
}

type EffectiveProfileV1CapabilityBindings map[string]CapabilityBinding

type ExecutionPathSelectionIDV1 string

type ExecutionPathSelectionV1 struct {
	// ExperiencePathID corresponds to the JSON schema field "experience_path_id".
	ExperiencePathID string `json:"experience_path_id"`

	// ID corresponds to the JSON schema field "id".
	ID ExecutionPathSelectionIDV1 `json:"id"`
}

type ExecutionWorkspaceRepositoryAccessV1 string

const ExecutionWorkspaceRepositoryAccessV1Mutate ExecutionWorkspaceRepositoryAccessV1 = "mutate"
const ExecutionWorkspaceRepositoryAccessV1Read ExecutionWorkspaceRepositoryAccessV1 = "read"

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

type LifecycleBehaviorIDV1 string

type LifecycleFallbackV1 struct {
	// CarrierEvent corresponds to the JSON schema field "carrier_event".
	CarrierEvent lifecycle_event_v1.LifecycleEventV1 `json:"carrier_event"`

	// Guidance corresponds to the JSON schema field "guidance".
	Guidance string `json:"guidance"`
}

type LogicalExecutionWorkspaceIDV1 string

type LogicalProjectIDV1 string

type LogicalRepositoryIDV1 string

type NormativePackAttestorV1 string

type NormativePackIDV1 string

type NormativePackReleaseV1 struct {
	// Attestor corresponds to the JSON schema field "attestor".
	Attestor NormativePackAttestorV1 `json:"attestor"`

	// ContentDigest corresponds to the JSON schema field "content_digest".
	ContentDigest sha256_digest_v1.Sha256DigestV1 `json:"content_digest"`

	// ID corresponds to the JSON schema field "id".
	ID NormativePackIDV1 `json:"id"`

	// Version corresponds to the JSON schema field "version".
	Version NormativePackVersionV1 `json:"version"`
}

type NormativePackVersionV1 string

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

type PackageIDV1 string

type PackageReleaseV1 struct {
	// ContentDigest corresponds to the JSON schema field "content_digest".
	ContentDigest sha256_digest_v1.Sha256DigestV1 `json:"content_digest"`

	// ID corresponds to the JSON schema field "id".
	ID PackageIDV1 `json:"id"`

	// Version corresponds to the JSON schema field "version".
	Version PackageVersionV1 `json:"version"`
}

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

type PackageResourceUseV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind PackageResourceKindV1 `json:"kind"`

	// Resource corresponds to the JSON schema field "resource".
	Resource PackageResourceRefV1 `json:"resource"`
}

type PackageVersionV1 string

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

type ProfileRepository struct {
	// Access corresponds to the JSON schema field "access".
	Access ExecutionWorkspaceRepositoryAccessV1 `json:"access"`

	// CapabilityCommands corresponds to the JSON schema field "capability_commands".
	CapabilityCommands ProfileRepositoryCapabilityCommands `json:"capability_commands,omitempty,omitzero"`

	// CommandNote corresponds to the JSON schema field "command_note".
	CommandNote ProfileRepositoryCommandNote `json:"command_note,omitempty,omitzero"`

	// Families corresponds to the JSON schema field "families".
	Families []PackageResourceRefV1 `json:"families"`

	// ID corresponds to the JSON schema field "id".
	ID LogicalRepositoryIDV1 `json:"id"`

	// MutatingNotReadonly corresponds to the JSON schema field
	// "mutating_not_readonly".
	MutatingNotReadonly []string `json:"mutating_not_readonly,omitempty,omitzero"`

	// Role corresponds to the JSON schema field "role".
	Role PackageResourceRefV1 `json:"role"`

	// Stacks corresponds to the JSON schema field "stacks".
	Stacks []PackageResourceRefV1 `json:"stacks,omitempty,omitzero"`
}

type ProfileRepositoryCapabilityCommands map[string][]string

type ProfileRepositoryCommandNote *string

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

type ReadingOrderEntry struct {
	// RelPath corresponds to the JSON schema field "rel_path".
	RelPath string `json:"rel_path"`

	// RootID corresponds to the JSON schema field "root_id".
	RootID string `json:"root_id"`
}

type ResolvedGoverningNarrowingV1 struct {
	// Broader corresponds to the JSON schema field "broader".
	Broader PackageResourceUseV1 `json:"broader"`

	// Contributor corresponds to the JSON schema field "contributor".
	Contributor PackageResourceRefV1 `json:"contributor"`

	// Narrower corresponds to the JSON schema field "narrower".
	Narrower PackageResourceUseV1 `json:"narrower"`

	// Scope corresponds to the JSON schema field "scope".
	Scope PackageResourceUseV1 `json:"scope"`
}

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

type SemanticOrderPurposeV1 string

const SemanticOrderPurposeV1Composition SemanticOrderPurposeV1 = "composition"
const SemanticOrderPurposeV1Execution SemanticOrderPurposeV1 = "execution"
const SemanticOrderPurposeV1Presentation SemanticOrderPurposeV1 = "presentation"
const SemanticOrderPurposeV1Reasoning SemanticOrderPurposeV1 = "reasoning"

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

type SessionModel struct {
	// ActivationPathIds corresponds to the JSON schema field "activation_path_ids".
	ActivationPathIds []ExecutionPathSelectionIDV1 `json:"activation_path_ids,omitempty,omitzero"`

	// ActivationRoots corresponds to the JSON schema field "activation_roots".
	ActivationRoots []LogicalRepositoryIDV1 `json:"activation_roots"`

	// AdditionalRoots corresponds to the JSON schema field "additional_roots".
	AdditionalRoots []LogicalRepositoryIDV1 `json:"additional_roots,omitempty,omitzero"`

	// AuthorityAccess corresponds to the JSON schema field "authority_access".
	AuthorityAccess []AuthorityAccess `json:"authority_access,omitempty,omitzero"`

	// LifecycleEvents corresponds to the JSON schema field "lifecycle_events".
	LifecycleEvents []lifecycle_event_v1.LifecycleEventV1 `json:"lifecycle_events,omitempty,omitzero"`

	// ReadingOrder corresponds to the JSON schema field "reading_order".
	ReadingOrder []ReadingOrderEntry `json:"reading_order,omitempty,omitzero"`

	// Runtime corresponds to the JSON schema field "runtime".
	Runtime *SessionRuntimeSelection `json:"runtime,omitempty,omitzero"`
}

type SessionRuntimeSelection string

const SessionRuntimeSelectionMachineLocal SessionRuntimeSelection = "machine-local"

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

type WorkspaceID string

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
