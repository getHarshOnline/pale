// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package composition_primitives_v1 contains the generated composition-primitives.v1 data model.
package composition_primitives_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type CompiledContentCoordinateV1 struct {
	// CompiledContentIdentity corresponds to the JSON schema field
	// "compiled_content_identity".
	CompiledContentIdentity CompiledContentIdentityV1 `json:"compiled_content_identity"`

	// SourceContentIdentity corresponds to the JSON schema field
	// "source_content_identity".
	SourceContentIdentity SourceContentIdentityV1 `json:"source_content_identity"`
}

type CompiledContentIdentityV1 struct {
	// CompiledContentSha256 corresponds to the JSON schema field
	// "compiled_content_sha256".
	CompiledContentSha256 sha256_digest_v1.Sha256DigestV1 `json:"compiled_content_sha256"`
}

type ContractSchemaIdentityV1 struct {
	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`
}

type DestinationConsequenceV1 struct {
	// ConsequenceContractID corresponds to the JSON schema field
	// "consequence_contract_id".
	ConsequenceContractID string `json:"consequence_contract_id"`

	// Destination corresponds to the JSON schema field "destination".
	Destination ParticipantRefV1 `json:"destination"`
}

type EntryOriginRefInterfaceV1 struct {
	// InterfaceEntryID corresponds to the JSON schema field "interface_entry_id".
	InterfaceEntryID string `json:"interface_entry_id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type EntryOriginRefParticipantV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ParticipantEntryID corresponds to the JSON schema field "participant_entry_id".
	ParticipantEntryID string `json:"participant_entry_id"`
}

type EntryOriginRefUserV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// UserEntryID corresponds to the JSON schema field "user_entry_id".
	UserEntryID string `json:"user_entry_id"`
}

type InvocationIdentityV1 struct {
	// InvocationSha256 corresponds to the JSON schema field "invocation_sha256".
	InvocationSha256 sha256_digest_v1.Sha256DigestV1 `json:"invocation_sha256"`
}

type NativeEffectIdentityV1 struct {
	// NativeEffectSha256 corresponds to the JSON schema field "native_effect_sha256".
	NativeEffectSha256 sha256_digest_v1.Sha256DigestV1 `json:"native_effect_sha256"`
}

type NativeOutcomeEffectfulOutcomeV1 struct {
	// EdgeID corresponds to the JSON schema field "edge_id".
	EdgeID string `json:"edge_id"`

	// InvocationIdentity corresponds to the JSON schema field "invocation_identity".
	InvocationIdentity InvocationIdentityV1 `json:"invocation_identity"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// NativeEffectIdentity corresponds to the JSON schema field
	// "native_effect_identity".
	NativeEffectIdentity NativeEffectIdentityV1 `json:"native_effect_identity"`

	// OutcomeContractID corresponds to the JSON schema field "outcome_contract_id".
	OutcomeContractID string `json:"outcome_contract_id"`
}

type NativeOutcomeReadOnlyResultV1 struct {
	// EdgeID corresponds to the JSON schema field "edge_id".
	EdgeID string `json:"edge_id"`

	// InvocationIdentity corresponds to the JSON schema field "invocation_identity".
	InvocationIdentity InvocationIdentityV1 `json:"invocation_identity"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// OutcomeContractID corresponds to the JSON schema field "outcome_contract_id".
	OutcomeContractID string `json:"outcome_contract_id"`
}

type NativeOwnerTransitionV1 struct {
	// EdgeID corresponds to the JSON schema field "edge_id".
	EdgeID string `json:"edge_id"`

	// NativeOwnerID corresponds to the JSON schema field "native_owner_id".
	NativeOwnerID string `json:"native_owner_id"`

	// ReceiptContractID corresponds to the JSON schema field "receipt_contract_id".
	ReceiptContractID string `json:"receipt_contract_id"`

	// TransitionContractID corresponds to the JSON schema field
	// "transition_contract_id".
	TransitionContractID string `json:"transition_contract_id"`
}

type ParticipantRefApplicationV1 struct {
	// ApplicationID corresponds to the JSON schema field "application_id".
	ApplicationID string `json:"application_id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ParticipantRefCapabilityV1 struct {
	// CapabilityID corresponds to the JSON schema field "capability_id".
	CapabilityID string `json:"capability_id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ParticipantRefEngineV1 struct {
	// EngineID corresponds to the JSON schema field "engine_id".
	EngineID string `json:"engine_id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ParticipantRefInterfaceV1 struct {
	// InterfaceID corresponds to the JSON schema field "interface_id".
	InterfaceID string `json:"interface_id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ParticipantRefPluginV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// PluginID corresponds to the JSON schema field "plugin_id".
	PluginID string `json:"plugin_id"`
}

type ParticipantRefSkillV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// SkillID corresponds to the JSON schema field "skill_id".
	SkillID string `json:"skill_id"`
}

type ParticipantRefTerminalLeafV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// TerminalLeafID corresponds to the JSON schema field "terminal_leaf_id".
	TerminalLeafID string `json:"terminal_leaf_id"`
}

type PathBudgetV1 struct {
	// MaxCostUnits corresponds to the JSON schema field "max_cost_units".
	MaxCostUnits int `json:"max_cost_units"`

	// MaxDepth corresponds to the JSON schema field "max_depth".
	MaxDepth int `json:"max_depth"`

	// MaxEffectCount corresponds to the JSON schema field "max_effect_count".
	MaxEffectCount int `json:"max_effect_count"`

	// MaxFanOut corresponds to the JSON schema field "max_fan_out".
	MaxFanOut int `json:"max_fan_out"`

	// MaxRecursionIterations corresponds to the JSON schema field
	// "max_recursion_iterations".
	MaxRecursionIterations int `json:"max_recursion_iterations"`

	// MaxResourceUnits corresponds to the JSON schema field "max_resource_units".
	MaxResourceUnits int `json:"max_resource_units"`

	// MaxTimeMs corresponds to the JSON schema field "max_time_ms".
	MaxTimeMs int `json:"max_time_ms"`
}

type PathInvocationV1 struct {
	// EdgeID corresponds to the JSON schema field "edge_id".
	EdgeID string `json:"edge_id"`

	// Identity corresponds to the JSON schema field "identity".
	Identity InvocationIdentityV1 `json:"identity"`
}

type ReconciliationRequirementV1 struct {
	// FrontierID corresponds to the JSON schema field "frontier_id".
	FrontierID string `json:"frontier_id"`

	// ObligationContractID corresponds to the JSON schema field
	// "obligation_contract_id".
	ObligationContractID string `json:"obligation_contract_id"`
}

type ResponsibilityCommunicationV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResponsibilityContextMemoryV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResponsibilityCrmBusinessStateV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResponsibilityEngineeringExecutionV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResponsibilityExtensionV1 struct {
	// ContractID corresponds to the JSON schema field "contract_id".
	ContractID string `json:"contract_id"`

	// ContractSchemaIdentity corresponds to the JSON schema field
	// "contract_schema_identity".
	ContractSchemaIdentity ContractSchemaIdentityV1 `json:"contract_schema_identity"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// NamespaceID corresponds to the JSON schema field "namespace_id".
	NamespaceID string `json:"namespace_id"`
}

type ResponsibilityNativeEffectReadBackV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResponsibilityNativeEventPublicationV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResponsibilityOrganizationalWorkV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResponsibilityTerminalHarnessExecutionV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type SchemaID string

type SourceContentIdentityV1 struct {
	// SourceContentSha256 corresponds to the JSON schema field
	// "source_content_sha256".
	SourceContentSha256 sha256_digest_v1.Sha256DigestV1 `json:"source_content_sha256"`
}

type EntryOriginRefV1 struct {
	User        *EntryOriginRefUserV1        `json:"-"`
	Interface   *EntryOriginRefInterfaceV1   `json:"-"`
	Participant *EntryOriginRefParticipantV1 `json:"-"`
}

func (value EntryOriginRefV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.User != nil {
		if value.User.Kind != "user" {
			return nil, fmt.Errorf("EntryOriginRefV1: selected variant has discriminator %q", value.User.Kind)
		}
		candidate, err := json.Marshal(value.User)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Interface != nil {
		if value.Interface.Kind != "interface" {
			return nil, fmt.Errorf("EntryOriginRefV1: selected variant has discriminator %q", value.Interface.Kind)
		}
		candidate, err := json.Marshal(value.Interface)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Participant != nil {
		if value.Participant.Kind != "participant" {
			return nil, fmt.Errorf("EntryOriginRefV1: selected variant has discriminator %q", value.Participant.Kind)
		}
		candidate, err := json.Marshal(value.Participant)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("EntryOriginRefV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *EntryOriginRefV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = EntryOriginRefV1{}
	switch discriminator.Kind {
	case "user":
		var variant EntryOriginRefUserV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.User = &variant
	case "interface":
		var variant EntryOriginRefInterfaceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Interface = &variant
	case "participant":
		var variant EntryOriginRefParticipantV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Participant = &variant
	default:
		return fmt.Errorf("EntryOriginRefV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type NativeOutcomeV1 struct {
	ReadOnlyResult   *NativeOutcomeReadOnlyResultV1   `json:"-"`
	EffectfulOutcome *NativeOutcomeEffectfulOutcomeV1 `json:"-"`
}

func (value NativeOutcomeV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.ReadOnlyResult != nil {
		if value.ReadOnlyResult.Kind != "read_only_result" {
			return nil, fmt.Errorf("NativeOutcomeV1: selected variant has discriminator %q", value.ReadOnlyResult.Kind)
		}
		candidate, err := json.Marshal(value.ReadOnlyResult)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.EffectfulOutcome != nil {
		if value.EffectfulOutcome.Kind != "effectful_outcome" {
			return nil, fmt.Errorf("NativeOutcomeV1: selected variant has discriminator %q", value.EffectfulOutcome.Kind)
		}
		candidate, err := json.Marshal(value.EffectfulOutcome)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("NativeOutcomeV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *NativeOutcomeV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = NativeOutcomeV1{}
	switch discriminator.Kind {
	case "read_only_result":
		var variant NativeOutcomeReadOnlyResultV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ReadOnlyResult = &variant
	case "effectful_outcome":
		var variant NativeOutcomeEffectfulOutcomeV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.EffectfulOutcome = &variant
	default:
		return fmt.Errorf("NativeOutcomeV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type ParticipantRefV1 struct {
	Interface    *ParticipantRefInterfaceV1    `json:"-"`
	Application  *ParticipantRefApplicationV1  `json:"-"`
	Engine       *ParticipantRefEngineV1       `json:"-"`
	Capability   *ParticipantRefCapabilityV1   `json:"-"`
	TerminalLeaf *ParticipantRefTerminalLeafV1 `json:"-"`
	Plugin       *ParticipantRefPluginV1       `json:"-"`
	Skill        *ParticipantRefSkillV1        `json:"-"`
}

func (value ParticipantRefV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Interface != nil {
		if value.Interface.Kind != "interface" {
			return nil, fmt.Errorf("ParticipantRefV1: selected variant has discriminator %q", value.Interface.Kind)
		}
		candidate, err := json.Marshal(value.Interface)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Application != nil {
		if value.Application.Kind != "application" {
			return nil, fmt.Errorf("ParticipantRefV1: selected variant has discriminator %q", value.Application.Kind)
		}
		candidate, err := json.Marshal(value.Application)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Engine != nil {
		if value.Engine.Kind != "engine" {
			return nil, fmt.Errorf("ParticipantRefV1: selected variant has discriminator %q", value.Engine.Kind)
		}
		candidate, err := json.Marshal(value.Engine)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Capability != nil {
		if value.Capability.Kind != "capability" {
			return nil, fmt.Errorf("ParticipantRefV1: selected variant has discriminator %q", value.Capability.Kind)
		}
		candidate, err := json.Marshal(value.Capability)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.TerminalLeaf != nil {
		if value.TerminalLeaf.Kind != "terminal_leaf" {
			return nil, fmt.Errorf("ParticipantRefV1: selected variant has discriminator %q", value.TerminalLeaf.Kind)
		}
		candidate, err := json.Marshal(value.TerminalLeaf)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Plugin != nil {
		if value.Plugin.Kind != "plugin" {
			return nil, fmt.Errorf("ParticipantRefV1: selected variant has discriminator %q", value.Plugin.Kind)
		}
		candidate, err := json.Marshal(value.Plugin)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Skill != nil {
		if value.Skill.Kind != "skill" {
			return nil, fmt.Errorf("ParticipantRefV1: selected variant has discriminator %q", value.Skill.Kind)
		}
		candidate, err := json.Marshal(value.Skill)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ParticipantRefV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ParticipantRefV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ParticipantRefV1{}
	switch discriminator.Kind {
	case "interface":
		var variant ParticipantRefInterfaceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Interface = &variant
	case "application":
		var variant ParticipantRefApplicationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Application = &variant
	case "engine":
		var variant ParticipantRefEngineV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Engine = &variant
	case "capability":
		var variant ParticipantRefCapabilityV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Capability = &variant
	case "terminal_leaf":
		var variant ParticipantRefTerminalLeafV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.TerminalLeaf = &variant
	case "plugin":
		var variant ParticipantRefPluginV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Plugin = &variant
	case "skill":
		var variant ParticipantRefSkillV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Skill = &variant
	default:
		return fmt.Errorf("ParticipantRefV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type ResponsibilityV1 struct {
	Communication            *ResponsibilityCommunicationV1            `json:"-"`
	OrganizationalWork       *ResponsibilityOrganizationalWorkV1       `json:"-"`
	CrmBusinessState         *ResponsibilityCrmBusinessStateV1         `json:"-"`
	EngineeringExecution     *ResponsibilityEngineeringExecutionV1     `json:"-"`
	ContextMemory            *ResponsibilityContextMemoryV1            `json:"-"`
	TerminalHarnessExecution *ResponsibilityTerminalHarnessExecutionV1 `json:"-"`
	NativeEventPublication   *ResponsibilityNativeEventPublicationV1   `json:"-"`
	NativeEffectReadBack     *ResponsibilityNativeEffectReadBackV1     `json:"-"`
	Extension                *ResponsibilityExtensionV1                `json:"-"`
}

func (value ResponsibilityV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Communication != nil {
		if value.Communication.Kind != "communication" {
			return nil, fmt.Errorf("ResponsibilityV1: selected variant has discriminator %q", value.Communication.Kind)
		}
		candidate, err := json.Marshal(value.Communication)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.OrganizationalWork != nil {
		if value.OrganizationalWork.Kind != "organizational_work" {
			return nil, fmt.Errorf("ResponsibilityV1: selected variant has discriminator %q", value.OrganizationalWork.Kind)
		}
		candidate, err := json.Marshal(value.OrganizationalWork)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.CrmBusinessState != nil {
		if value.CrmBusinessState.Kind != "crm_business_state" {
			return nil, fmt.Errorf("ResponsibilityV1: selected variant has discriminator %q", value.CrmBusinessState.Kind)
		}
		candidate, err := json.Marshal(value.CrmBusinessState)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.EngineeringExecution != nil {
		if value.EngineeringExecution.Kind != "engineering_execution" {
			return nil, fmt.Errorf("ResponsibilityV1: selected variant has discriminator %q", value.EngineeringExecution.Kind)
		}
		candidate, err := json.Marshal(value.EngineeringExecution)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ContextMemory != nil {
		if value.ContextMemory.Kind != "context_memory" {
			return nil, fmt.Errorf("ResponsibilityV1: selected variant has discriminator %q", value.ContextMemory.Kind)
		}
		candidate, err := json.Marshal(value.ContextMemory)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.TerminalHarnessExecution != nil {
		if value.TerminalHarnessExecution.Kind != "terminal_harness_execution" {
			return nil, fmt.Errorf("ResponsibilityV1: selected variant has discriminator %q", value.TerminalHarnessExecution.Kind)
		}
		candidate, err := json.Marshal(value.TerminalHarnessExecution)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.NativeEventPublication != nil {
		if value.NativeEventPublication.Kind != "native_event_publication" {
			return nil, fmt.Errorf("ResponsibilityV1: selected variant has discriminator %q", value.NativeEventPublication.Kind)
		}
		candidate, err := json.Marshal(value.NativeEventPublication)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.NativeEffectReadBack != nil {
		if value.NativeEffectReadBack.Kind != "native_effect_read_back" {
			return nil, fmt.Errorf("ResponsibilityV1: selected variant has discriminator %q", value.NativeEffectReadBack.Kind)
		}
		candidate, err := json.Marshal(value.NativeEffectReadBack)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Extension != nil {
		if value.Extension.Kind != "extension" {
			return nil, fmt.Errorf("ResponsibilityV1: selected variant has discriminator %q", value.Extension.Kind)
		}
		candidate, err := json.Marshal(value.Extension)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ResponsibilityV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ResponsibilityV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ResponsibilityV1{}
	switch discriminator.Kind {
	case "communication":
		var variant ResponsibilityCommunicationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Communication = &variant
	case "organizational_work":
		var variant ResponsibilityOrganizationalWorkV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.OrganizationalWork = &variant
	case "crm_business_state":
		var variant ResponsibilityCrmBusinessStateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.CrmBusinessState = &variant
	case "engineering_execution":
		var variant ResponsibilityEngineeringExecutionV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.EngineeringExecution = &variant
	case "context_memory":
		var variant ResponsibilityContextMemoryV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ContextMemory = &variant
	case "terminal_harness_execution":
		var variant ResponsibilityTerminalHarnessExecutionV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.TerminalHarnessExecution = &variant
	case "native_event_publication":
		var variant ResponsibilityNativeEventPublicationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NativeEventPublication = &variant
	case "native_effect_read_back":
		var variant ResponsibilityNativeEffectReadBackV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NativeEffectReadBack = &variant
	case "extension":
		var variant ResponsibilityExtensionV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Extension = &variant
	default:
		return fmt.Errorf("ResponsibilityV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
