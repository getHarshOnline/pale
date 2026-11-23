// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package hook_context_v1 contains the generated hook-context.v1 data model.
package hook_context_v1

import lifecycle_event_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/lifecycle_event_v1"
import native_harness_binding_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/native_harness_binding_v1"
import sealed_artifact_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sealed_artifact_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import tool_call_matcher_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/tool_call_matcher_v1"

type CapabilityIDV1 string

type ChildPropagation string

const ChildPropagationExempt ChildPropagation = "exempt"
const ChildPropagationNone ChildPropagation = "none"
const ChildPropagationPropagate ChildPropagation = "propagate"

type DurableEvidenceBinding struct {
	// RetentionSeconds corresponds to the JSON schema field "retention_seconds".
	RetentionSeconds EvidenceRetentionSeconds `json:"retention_seconds"`

	// Root corresponds to the JSON schema field "root".
	Root string `json:"root"`
}

type EffectClass string

const EffectClassCompute EffectClass = "compute"
const EffectClassExecute EffectClass = "execute"
const EffectClassNetwork EffectClass = "network"
const EffectClassPublish EffectClass = "publish"
const EffectClassRead EffectClass = "read"
const EffectClassWrite EffectClass = "write"

type EvidenceRetentionSeconds int

type ExecutionPathSelectionIDV1 string

type ExecutionPathSelectionV1 struct {
	// ExperiencePathID corresponds to the JSON schema field "experience_path_id".
	ExperiencePathID string `json:"experience_path_id"`

	// ID corresponds to the JSON schema field "id".
	ID ExecutionPathSelectionIDV1 `json:"id"`
}

type ExecutionPathWitnessV1 struct {
	// CompositionPlanDigest corresponds to the JSON schema field
	// "composition_plan_digest".
	CompositionPlanDigest sha256_digest_v1.Sha256DigestV1 `json:"composition_plan_digest"`

	// ResponsibilityPathIdentity corresponds to the JSON schema field
	// "responsibility_path_identity".
	ResponsibilityPathIdentity ResponsibilityPathIdentityV1 `json:"responsibility_path_identity"`

	// Selection corresponds to the JSON schema field "selection".
	Selection ExecutionPathSelectionV1 `json:"selection"`

	// TerminalProviderBinding corresponds to the JSON schema field
	// "terminal_provider_binding".
	TerminalProviderBinding *ProviderBindingRefV1 `json:"terminal_provider_binding,omitempty,omitzero"`

	// WorkDelegationEdgeIds corresponds to the JSON schema field
	// "work_delegation_edge_ids".
	WorkDelegationEdgeIds []string `json:"work_delegation_edge_ids"`
}

type FailurePosture string

const FailurePostureEscalate FailurePosture = "escalate"
const FailurePostureFailClosed FailurePosture = "fail_closed"
const FailurePostureFailOpen FailurePosture = "fail_open"
const FailurePostureRetry FailurePosture = "retry"
const FailurePostureSteer FailurePosture = "steer"

type HookContextV1 struct {
	// AdapterPluginID corresponds to the JSON schema field "adapter_plugin_id".
	AdapterPluginID string `json:"adapter_plugin_id"`

	// AvailableToolRequirements corresponds to the JSON schema field
	// "available_tool_requirements".
	AvailableToolRequirements []CapabilityIDV1 `json:"available_tool_requirements,omitempty,omitzero"`

	// BehaviourGuidance corresponds to the JSON schema field "behaviour_guidance".
	BehaviourGuidance HookContextV1BehaviourGuidance `json:"behaviour_guidance,omitempty,omitzero"`

	// Behaviours corresponds to the JSON schema field "behaviours".
	Behaviours []LifecycleBehavior `json:"behaviours,omitempty,omitzero"`

	// CompilationInputDigest corresponds to the JSON schema field
	// "compilation_input_digest".
	CompilationInputDigest sha256_digest_v1.Sha256DigestV1 `json:"compilation_input_digest"`

	// DurableEvidence corresponds to the JSON schema field "durable_evidence".
	DurableEvidence *DurableEvidenceBinding `json:"durable_evidence,omitempty,omitzero"`

	// ExecutionPathWitness corresponds to the JSON schema field
	// "execution_path_witness".
	ExecutionPathWitness ExecutionPathWitnessV1 `json:"execution_path_witness"`

	// HostToolEffects corresponds to the JSON schema field "host_tool_effects".
	HostToolEffects []HostToolEffectBinding `json:"host_tool_effects,omitempty,omitzero"`

	// InjectionDigest corresponds to the JSON schema field "injection_digest".
	InjectionDigest sha256_digest_v1.Sha256DigestV1 `json:"injection_digest"`

	// InstanceDigest corresponds to the JSON schema field "instance_digest".
	InstanceDigest sha256_digest_v1.Sha256DigestV1 `json:"instance_digest"`

	// MaterializationLineage corresponds to the JSON schema field
	// "materialization_lineage".
	MaterializationLineage *MaterializationLineageBinding `json:"materialization_lineage,omitempty,omitzero"`

	// ModeDefaults corresponds to the JSON schema field "mode_defaults".
	ModeDefaults HookContextV1ModeDefaults `json:"mode_defaults,omitempty,omitzero"`

	// NativeHarnessBinding corresponds to the JSON schema field
	// "native_harness_binding".
	NativeHarnessBinding *native_harness_binding_v1.NativeHarnessBindingV1 `json:"native_harness_binding,omitempty,omitzero"`

	// OperationBinding corresponds to the JSON schema field "operation_binding".
	OperationBinding sealed_artifact_v1.SealedArtifactV1 `json:"operation_binding"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// ProjectionManifest corresponds to the JSON schema field "projection_manifest".
	ProjectionManifest sealed_artifact_v1.SealedArtifactV1 `json:"projection_manifest"`

	// ProtectedRuntimeDirs corresponds to the JSON schema field
	// "protected_runtime_dirs".
	ProtectedRuntimeDirs []string `json:"protected_runtime_dirs,omitempty,omitzero"`

	// ReadOnlyDirs corresponds to the JSON schema field "read_only_dirs".
	ReadOnlyDirs []string `json:"read_only_dirs,omitempty,omitzero"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`

	// RepositoryVerifiers corresponds to the JSON schema field
	// "repository_verifiers".
	RepositoryVerifiers []RepositoryVerifierBinding `json:"repository_verifiers,omitempty,omitzero"`

	// RunModes corresponds to the JSON schema field "run_modes".
	RunModes []SealedRunMode `json:"run_modes,omitempty,omitzero"`

	// RunPolicy corresponds to the JSON schema field "run_policy".
	RunPolicy RunPolicyEntry `json:"run_policy"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaManifestSha256 corresponds to the JSON schema field
	// "schema_manifest_sha256".
	SchemaManifestSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_manifest_sha256"`

	// SchemaSetSha256 corresponds to the JSON schema field "schema_set_sha256".
	SchemaSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_set_sha256"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// SessionState corresponds to the JSON schema field "session_state".
	SessionState *SessionStateBinding `json:"session_state,omitempty,omitzero"`
}

type HookContextV1BehaviourGuidance map[string]sealed_artifact_v1.SealedArtifactV1

type HookContextV1ModeDefaults map[string]PackageResourceRefV1

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

type HostToolEffectBinding struct {
	// EffectClass corresponds to the JSON schema field "effect_class".
	EffectClass EffectClass `json:"effect_class"`

	// Tool corresponds to the JSON schema field "tool".
	Tool HostToolBinding `json:"tool"`
}

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

type LifecycleBehaviorGovernedBy *string

type LifecycleBehaviorIDV1 string

type LifecycleBehaviorTimeoutMs *int

type LifecycleFallbackV1 struct {
	// CarrierEvent corresponds to the JSON schema field "carrier_event".
	CarrierEvent lifecycle_event_v1.LifecycleEventV1 `json:"carrier_event"`

	// Guidance corresponds to the JSON schema field "guidance".
	Guidance string `json:"guidance"`
}

type LogicalRepositoryIDV1 string

type MaterializationLineageBinding struct {
	// MaxRouteRecordsPerCollection corresponds to the JSON schema field
	// "max_route_records_per_collection".
	MaxRouteRecordsPerCollection RouteRecordLimit `json:"max_route_records_per_collection"`

	// Root corresponds to the JSON schema field "root".
	Root string `json:"root"`
}

type ModeAxis string

const ModeAxisIntensity ModeAxis = "intensity"
const ModeAxisTaskStance ModeAxis = "task_stance"

type OwnedConfinedRelativePath string

type PackageIDV1 string

type PackageResourceIDV1 string

type PackageResourceRefV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`
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

type RepositoryVerifierBinding struct {
	// CapabilityID corresponds to the JSON schema field "capability_id".
	CapabilityID CapabilityIDV1 `json:"capability_id"`

	// Command corresponds to the JSON schema field "command".
	Command HostCommandConstraint `json:"command"`

	// ToolName corresponds to the JSON schema field "tool_name".
	ToolName string `json:"tool_name"`
}

type ResponsibilityPathIdentityV1 struct {
	// ResponsibilityPathSha256 corresponds to the JSON schema field
	// "responsibility_path_sha256".
	ResponsibilityPathSha256 sha256_digest_v1.Sha256DigestV1 `json:"responsibility_path_sha256"`
}

type RouteRecordLimit int

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

type SealedRunMode struct {
	// Aliases corresponds to the JSON schema field "aliases".
	Aliases []string `json:"aliases,omitempty,omitzero"`

	// Axis corresponds to the JSON schema field "axis".
	Axis ModeAxis `json:"axis"`

	// Body corresponds to the JSON schema field "body".
	Body *sealed_artifact_v1.SealedArtifactV1 `json:"body,omitempty,omitzero"`

	// ExactTriggers corresponds to the JSON schema field "exact_triggers".
	ExactTriggers []string `json:"exact_triggers,omitempty,omitzero"`

	// ID corresponds to the JSON schema field "id".
	ID PackageResourceRefV1 `json:"id"`
}

type SessionStateBinding struct {
	// Root corresponds to the JSON schema field "root".
	Root string `json:"root"`

	// TtlSeconds corresponds to the JSON schema field "ttl_seconds".
	TtlSeconds SessionTtlSeconds `json:"ttl_seconds"`
}

type SessionTtlSeconds int

type StateScope string

const StateScopeDurable StateScope = "durable"
const StateScopeMessage StateScope = "message"
const StateScopeSession StateScope = "session"
const StateScopeStateless StateScope = "stateless"

type ToolObservability string

const ToolObservabilityPermissionOnly ToolObservability = "permission_only"
const ToolObservabilityPreToolObservable ToolObservability = "pre_tool_observable"
const ToolObservabilityUnobservable ToolObservability = "unobservable"
