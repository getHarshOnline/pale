// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package composition_edge_v1 contains the generated composition-edge.v1 data model.
package composition_edge_v1

import (
	"encoding/json"
	"fmt"
	composition_primitives_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/composition_primitives_v1"
)

type CompositionEdgeNativeApplicationCallV1 struct {
	// Application corresponds to the JSON schema field "application".
	Application composition_primitives_v1.ParticipantRefInterfaceV1 `json:"application"`

	// Caller corresponds to the JSON schema field "caller".
	Caller composition_primitives_v1.ParticipantRefInterfaceV1 `json:"caller"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ProviderBindingID corresponds to the JSON schema field "provider_binding_id".
	ProviderBindingID string `json:"provider_binding_id"`

	// RequestTranslationID corresponds to the JSON schema field
	// "request_translation_id".
	RequestTranslationID string `json:"request_translation_id"`

	// ResultTranslationID corresponds to the JSON schema field
	// "result_translation_id".
	ResultTranslationID string `json:"result_translation_id"`
}

type CompositionEdgeObservationSettlementV1 struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Observer corresponds to the JSON schema field "observer".
	Observer composition_primitives_v1.ParticipantRefInterfaceV1 `json:"observer"`

	// SettlementContractID corresponds to the JSON schema field
	// "settlement_contract_id".
	SettlementContractID string `json:"settlement_contract_id"`

	// Subject corresponds to the JSON schema field "subject".
	Subject composition_primitives_v1.ParticipantRefInterfaceV1 `json:"subject"`
}

type CompositionEdgePluginDependencyV1 struct {
	// Dependency corresponds to the JSON schema field "dependency".
	Dependency composition_primitives_v1.ParticipantRefInterfaceV1 `json:"dependency"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Plugin corresponds to the JSON schema field "plugin".
	Plugin composition_primitives_v1.ParticipantRefInterfaceV1 `json:"plugin"`
}

type CompositionEdgeTerminalLeafV1 struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Owner corresponds to the JSON schema field "owner".
	Owner composition_primitives_v1.ParticipantRefInterfaceV1 `json:"owner"`

	// TerminalLeaf corresponds to the JSON schema field "terminal_leaf".
	TerminalLeaf composition_primitives_v1.ParticipantRefInterfaceV1 `json:"terminal_leaf"`

	// TerminationContractID corresponds to the JSON schema field
	// "termination_contract_id".
	TerminationContractID string `json:"termination_contract_id"`
}

type CompositionEdgeUiEmbeddingV1 struct {
	// Embedded corresponds to the JSON schema field "embedded".
	Embedded composition_primitives_v1.ParticipantRefInterfaceV1 `json:"embedded"`

	// Host corresponds to the JSON schema field "host".
	Host composition_primitives_v1.ParticipantRefInterfaceV1 `json:"host"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

// Application corresponds to the JSON schema field "application".

// AuthorityCeiling corresponds to the JSON schema field "authority_ceiling".

// Budget corresponds to the JSON schema field "budget".

// Caller corresponds to the JSON schema field "caller".

// Dependency corresponds to the JSON schema field "dependency".

// Embedded corresponds to the JSON schema field "embedded".

// From corresponds to the JSON schema field "from".

// Host corresponds to the JSON schema field "host".

// ID corresponds to the JSON schema field "id".

// Kind corresponds to the JSON schema field "kind".

// Observer corresponds to the JSON schema field "observer".

// Owner corresponds to the JSON schema field "owner".

// Plugin corresponds to the JSON schema field "plugin".

// ProviderBindingID corresponds to the JSON schema field "provider_binding_id".

// RequestTranslationID corresponds to the JSON schema field
// "request_translation_id".

// ResponsibilityContractID corresponds to the JSON schema field
// "responsibility_contract_id".

// ResultTranslationID corresponds to the JSON schema field
// "result_translation_id".

// SettlementContractID corresponds to the JSON schema field
// "settlement_contract_id".

// Subject corresponds to the JSON schema field "subject".

// TerminalLeaf corresponds to the JSON schema field "terminal_leaf".

// TerminationContractID corresponds to the JSON schema field
// "termination_contract_id".

// To corresponds to the JSON schema field "to".

type CompositionEdgeWorkDelegationV1 struct {
	// AuthorityCeiling corresponds to the JSON schema field "authority_ceiling".
	AuthorityCeiling []EffectClass `json:"authority_ceiling"`

	// Budget corresponds to the JSON schema field "budget".
	Budget composition_primitives_v1.PathBudgetV1 `json:"budget"`

	// From corresponds to the JSON schema field "from".
	From composition_primitives_v1.ParticipantRefInterfaceV1 `json:"from"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// RequestTranslationID corresponds to the JSON schema field
	// "request_translation_id".
	RequestTranslationID string `json:"request_translation_id"`

	// ResponsibilityContractID corresponds to the JSON schema field
	// "responsibility_contract_id".
	ResponsibilityContractID string `json:"responsibility_contract_id"`

	// ResultTranslationID corresponds to the JSON schema field
	// "result_translation_id".
	ResultTranslationID string `json:"result_translation_id"`

	// To corresponds to the JSON schema field "to".
	To composition_primitives_v1.ParticipantRefInterfaceV1 `json:"to"`
}

type EffectClass string

const EffectClassCompute EffectClass = "compute"
const EffectClassExecute EffectClass = "execute"
const EffectClassNetwork EffectClass = "network"
const EffectClassPublish EffectClass = "publish"
const EffectClassRead EffectClass = "read"
const EffectClassWrite EffectClass = "write"

type CompositionEdgeV1 struct {
	WorkDelegation        *CompositionEdgeWorkDelegationV1        `json:"-"`
	UiEmbedding           *CompositionEdgeUiEmbeddingV1           `json:"-"`
	PluginDependency      *CompositionEdgePluginDependencyV1      `json:"-"`
	NativeApplicationCall *CompositionEdgeNativeApplicationCallV1 `json:"-"`
	TerminalLeaf          *CompositionEdgeTerminalLeafV1          `json:"-"`
	ObservationSettlement *CompositionEdgeObservationSettlementV1 `json:"-"`
}

func (value CompositionEdgeV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.WorkDelegation != nil {
		if value.WorkDelegation.Kind != "work_delegation" {
			return nil, fmt.Errorf("CompositionEdgeV1: selected variant has discriminator %q", value.WorkDelegation.Kind)
		}
		candidate, err := json.Marshal(value.WorkDelegation)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.UiEmbedding != nil {
		if value.UiEmbedding.Kind != "ui_embedding" {
			return nil, fmt.Errorf("CompositionEdgeV1: selected variant has discriminator %q", value.UiEmbedding.Kind)
		}
		candidate, err := json.Marshal(value.UiEmbedding)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.PluginDependency != nil {
		if value.PluginDependency.Kind != "plugin_dependency" {
			return nil, fmt.Errorf("CompositionEdgeV1: selected variant has discriminator %q", value.PluginDependency.Kind)
		}
		candidate, err := json.Marshal(value.PluginDependency)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.NativeApplicationCall != nil {
		if value.NativeApplicationCall.Kind != "native_application_call" {
			return nil, fmt.Errorf("CompositionEdgeV1: selected variant has discriminator %q", value.NativeApplicationCall.Kind)
		}
		candidate, err := json.Marshal(value.NativeApplicationCall)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.TerminalLeaf != nil {
		if value.TerminalLeaf.Kind != "terminal_leaf" {
			return nil, fmt.Errorf("CompositionEdgeV1: selected variant has discriminator %q", value.TerminalLeaf.Kind)
		}
		candidate, err := json.Marshal(value.TerminalLeaf)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ObservationSettlement != nil {
		if value.ObservationSettlement.Kind != "observation_settlement" {
			return nil, fmt.Errorf("CompositionEdgeV1: selected variant has discriminator %q", value.ObservationSettlement.Kind)
		}
		candidate, err := json.Marshal(value.ObservationSettlement)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("CompositionEdgeV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *CompositionEdgeV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = CompositionEdgeV1{}
	switch discriminator.Kind {
	case "work_delegation":
		var variant CompositionEdgeWorkDelegationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.WorkDelegation = &variant
	case "ui_embedding":
		var variant CompositionEdgeUiEmbeddingV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.UiEmbedding = &variant
	case "plugin_dependency":
		var variant CompositionEdgePluginDependencyV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.PluginDependency = &variant
	case "native_application_call":
		var variant CompositionEdgeNativeApplicationCallV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NativeApplicationCall = &variant
	case "terminal_leaf":
		var variant CompositionEdgeTerminalLeafV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.TerminalLeaf = &variant
	case "observation_settlement":
		var variant CompositionEdgeObservationSettlementV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ObservationSettlement = &variant
	default:
		return fmt.Errorf("CompositionEdgeV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
