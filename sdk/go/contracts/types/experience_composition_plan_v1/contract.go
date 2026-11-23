// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package experience_composition_plan_v1 contains the generated experience-composition-plan.v1 data model.
package experience_composition_plan_v1

import (
	"encoding/json"
	"fmt"
	composition_edge_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/composition_edge_v1"
)
import composition_primitives_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/composition_primitives_v1"
import continuity_contract_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/continuity_contract_v1"
import entry_contract_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/entry_contract_v1"
import experience_path_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/experience_path_v1"
import participant_provider_manifest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/participant_provider_manifest_v1"
import provider_binding_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/provider_binding_v1"
import request_translation_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/request_translation_v1"
import responsibility_contract_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/responsibility_contract_v1"
import result_translation_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/result_translation_v1"
import settlement_contract_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/settlement_contract_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import termination_contract_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/termination_contract_v1"

type CompositionGraphKindV1 string

const CompositionGraphKindV1NativeApplicationCall CompositionGraphKindV1 = "native_application_call"
const CompositionGraphKindV1ObservationSettlement CompositionGraphKindV1 = "observation_settlement"
const CompositionGraphKindV1PluginDependency CompositionGraphKindV1 = "plugin_dependency"
const CompositionGraphKindV1ProviderCompatibility CompositionGraphKindV1 = "provider_compatibility"
const CompositionGraphKindV1TerminalLeaf CompositionGraphKindV1 = "terminal_leaf"
const CompositionGraphKindV1UiEmbedding CompositionGraphKindV1 = "ui_embedding"
const CompositionGraphKindV1WorkDelegation CompositionGraphKindV1 = "work_delegation"

type CompositionReasonPathV1 struct {
	// PathID corresponds to the JSON schema field "path_id".
	PathID string `json:"path_id"`

	// Reasons corresponds to the JSON schema field "reasons".
	Reasons []CompositionReasonStepV1 `json:"reasons"`
}

type CompositionReasonStepContinuityContractV1 struct {
	// ContractID corresponds to the JSON schema field "contract_id".
	ContractID string `json:"contract_id"`

	// Reason corresponds to the JSON schema field "reason".
	Reason string `json:"reason"`
}

type CompositionReasonStepDelegationEdgeV1 struct {
	// EdgeID corresponds to the JSON schema field "edge_id".
	EdgeID string `json:"edge_id"`

	// Reason corresponds to the JSON schema field "reason".
	Reason string `json:"reason"`
}

type CompositionReasonStepEntryContractV1 struct {
	// ContractID corresponds to the JSON schema field "contract_id".
	ContractID string `json:"contract_id"`

	// Reason corresponds to the JSON schema field "reason".
	Reason string `json:"reason"`
}

type CompositionReasonStepOriginReceiptV1 struct {
	// ContractID corresponds to the JSON schema field "contract_id".
	ContractID string `json:"contract_id"`

	// Reason corresponds to the JSON schema field "reason".
	Reason string `json:"reason"`
}

type CompositionReasonStepProviderBindingV1 struct {
	// BindingID corresponds to the JSON schema field "binding_id".
	BindingID string `json:"binding_id"`

	// Reason corresponds to the JSON schema field "reason".
	Reason string `json:"reason"`
}

type CompositionReasonStepResponsibilityContractV1 struct {
	// ContractID corresponds to the JSON schema field "contract_id".
	ContractID string `json:"contract_id"`

	// Reason corresponds to the JSON schema field "reason".
	Reason string `json:"reason"`
}

type CompositionReasonStepSettlementContractV1 struct {
	// ContractID corresponds to the JSON schema field "contract_id".
	ContractID string `json:"contract_id"`

	// Reason corresponds to the JSON schema field "reason".
	Reason string `json:"reason"`
}

type CompositionReasonStepTerminationContractV1 struct {
	// ContractID corresponds to the JSON schema field "contract_id".
	ContractID string `json:"contract_id"`

	// Reason corresponds to the JSON schema field "reason".
	Reason string `json:"reason"`
}

type ExperienceCompositionDeclarationsV1 struct {
	// ContinuityContracts corresponds to the JSON schema field
	// "continuity_contracts".
	ContinuityContracts []continuity_contract_v1.ContinuityContractV1 `json:"continuity_contracts"`

	// EdgeContracts corresponds to the JSON schema field "edge_contracts".
	EdgeContracts []composition_edge_v1.CompositionEdgeV1 `json:"edge_contracts"`

	// EntryContracts corresponds to the JSON schema field "entry_contracts".
	EntryContracts []entry_contract_v1.EntryContractV1 `json:"entry_contracts"`

	// ParticipantProviderManifests corresponds to the JSON schema field
	// "participant_provider_manifests".
	ParticipantProviderManifests []participant_provider_manifest_v1.ParticipantProviderManifestV1 `json:"participant_provider_manifests"`

	// ProviderBindings corresponds to the JSON schema field "provider_bindings".
	ProviderBindings []provider_binding_v1.ProviderBindingV1 `json:"provider_bindings"`

	// RequestTranslations corresponds to the JSON schema field
	// "request_translations".
	RequestTranslations []request_translation_v1.RequestTranslationV1 `json:"request_translations"`

	// ResponsibilityContracts corresponds to the JSON schema field
	// "responsibility_contracts".
	ResponsibilityContracts []responsibility_contract_v1.ResponsibilityContractV1 `json:"responsibility_contracts"`

	// ResultTranslations corresponds to the JSON schema field "result_translations".
	ResultTranslations []result_translation_v1.ResultTranslationV1 `json:"result_translations"`

	// SelectedExperiencePaths corresponds to the JSON schema field
	// "selected_experience_paths".
	SelectedExperiencePaths []experience_path_v1.ExperiencePathV1 `json:"selected_experience_paths"`

	// SettlementContracts corresponds to the JSON schema field
	// "settlement_contracts".
	SettlementContracts []settlement_contract_v1.SettlementContractV1 `json:"settlement_contracts"`

	// TerminationContracts corresponds to the JSON schema field
	// "termination_contracts".
	TerminationContracts []termination_contract_v1.TerminationContractV1 `json:"termination_contracts"`
}

type ExperienceCompositionPlanV1 struct {
	// Declarations corresponds to the JSON schema field "declarations".
	Declarations ExperienceCompositionDeclarationsV1 `json:"declarations"`

	// DelegationGraph corresponds to the JSON schema field "delegation_graph".
	DelegationGraph ExperienceCompositionPlanV1DelegationGraph `json:"delegation_graph"`

	// EvidenceSettlementGraph corresponds to the JSON schema field
	// "evidence_settlement_graph".
	EvidenceSettlementGraph ExperienceCompositionPlanV1EvidenceSettlementGraph `json:"evidence_settlement_graph"`

	// NativeApplicationGraph corresponds to the JSON schema field
	// "native_application_graph".
	NativeApplicationGraph ExperienceCompositionPlanV1NativeApplicationGraph `json:"native_application_graph"`

	// PlanDigest corresponds to the JSON schema field "plan_digest".
	PlanDigest sha256_digest_v1.Sha256DigestV1 `json:"plan_digest"`

	// PluginDependencyGraph corresponds to the JSON schema field
	// "plugin_dependency_graph".
	PluginDependencyGraph ExperienceCompositionPlanV1PluginDependencyGraph `json:"plugin_dependency_graph"`

	// ProviderCompatibilityGraph corresponds to the JSON schema field
	// "provider_compatibility_graph".
	ProviderCompatibilityGraph ExperienceCompositionPlanV1ProviderCompatibilityGraph `json:"provider_compatibility_graph"`

	// ReasonPaths corresponds to the JSON schema field "reason_paths".
	ReasonPaths []CompositionReasonPathV1 `json:"reason_paths"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// TerminalLeafGraph corresponds to the JSON schema field "terminal_leaf_graph".
	TerminalLeafGraph ExperienceCompositionPlanV1TerminalLeafGraph `json:"terminal_leaf_graph"`

	// UiEmbeddingGraph corresponds to the JSON schema field "ui_embedding_graph".
	UiEmbeddingGraph ExperienceCompositionPlanV1UiEmbeddingGraph `json:"ui_embedding_graph"`
}

type ExperienceCompositionPlanV1DelegationGraph struct {
	// Edges corresponds to the JSON schema field "edges".
	Edges []string `json:"edges"`

	// Kind corresponds to the JSON schema field "kind".
	Kind CompositionGraphKindV1 `json:"kind"`

	// Nodes corresponds to the JSON schema field "nodes".
	Nodes []composition_primitives_v1.ParticipantRefInterfaceV1 `json:"nodes"`
}

type ExperienceCompositionPlanV1EvidenceSettlementGraph struct {
	// Edges corresponds to the JSON schema field "edges".
	Edges []string `json:"edges"`

	// Kind corresponds to the JSON schema field "kind".
	Kind CompositionGraphKindV1 `json:"kind"`

	// Nodes corresponds to the JSON schema field "nodes".
	Nodes []composition_primitives_v1.ParticipantRefInterfaceV1 `json:"nodes"`
}

type ExperienceCompositionPlanV1NativeApplicationGraph struct {
	// Edges corresponds to the JSON schema field "edges".
	Edges []string `json:"edges"`

	// Kind corresponds to the JSON schema field "kind".
	Kind CompositionGraphKindV1 `json:"kind"`

	// Nodes corresponds to the JSON schema field "nodes".
	Nodes []composition_primitives_v1.ParticipantRefInterfaceV1 `json:"nodes"`
}

type ExperienceCompositionPlanV1PluginDependencyGraph struct {
	// Edges corresponds to the JSON schema field "edges".
	Edges []string `json:"edges"`

	// Kind corresponds to the JSON schema field "kind".
	Kind CompositionGraphKindV1 `json:"kind"`

	// Nodes corresponds to the JSON schema field "nodes".
	Nodes []composition_primitives_v1.ParticipantRefInterfaceV1 `json:"nodes"`
}

type ExperienceCompositionPlanV1ProviderCompatibilityGraph struct {
	// Edges corresponds to the JSON schema field "edges".
	Edges []string `json:"edges"`

	// Kind corresponds to the JSON schema field "kind".
	Kind CompositionGraphKindV1 `json:"kind"`

	// Nodes corresponds to the JSON schema field "nodes".
	Nodes []composition_primitives_v1.ParticipantRefInterfaceV1 `json:"nodes"`
}

type ExperienceCompositionPlanV1TerminalLeafGraph struct {
	// Edges corresponds to the JSON schema field "edges".
	Edges []string `json:"edges"`

	// Kind corresponds to the JSON schema field "kind".
	Kind CompositionGraphKindV1 `json:"kind"`

	// Nodes corresponds to the JSON schema field "nodes".
	Nodes []composition_primitives_v1.ParticipantRefInterfaceV1 `json:"nodes"`
}

type ExperienceCompositionPlanV1UiEmbeddingGraph struct {
	// Edges corresponds to the JSON schema field "edges".
	Edges []string `json:"edges"`

	// Kind corresponds to the JSON schema field "kind".
	Kind CompositionGraphKindV1 `json:"kind"`

	// Nodes corresponds to the JSON schema field "nodes".
	Nodes []composition_primitives_v1.ParticipantRefInterfaceV1 `json:"nodes"`
}

type CompositionReasonStepV1 struct {
	OriginReceipt          *CompositionReasonStepOriginReceiptV1          `json:"-"`
	EntryContract          *CompositionReasonStepEntryContractV1          `json:"-"`
	ResponsibilityContract *CompositionReasonStepResponsibilityContractV1 `json:"-"`
	ProviderBinding        *CompositionReasonStepProviderBindingV1        `json:"-"`
	DelegationEdge         *CompositionReasonStepDelegationEdgeV1         `json:"-"`
	TerminationContract    *CompositionReasonStepTerminationContractV1    `json:"-"`
	SettlementContract     *CompositionReasonStepSettlementContractV1     `json:"-"`
	ContinuityContract     *CompositionReasonStepContinuityContractV1     `json:"-"`
}

func (value CompositionReasonStepV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.OriginReceipt != nil {
		if value.OriginReceipt.Reason != "origin_receipt" {
			return nil, fmt.Errorf("CompositionReasonStepV1: selected variant has discriminator %q", value.OriginReceipt.Reason)
		}
		candidate, err := json.Marshal(value.OriginReceipt)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.EntryContract != nil {
		if value.EntryContract.Reason != "entry_contract" {
			return nil, fmt.Errorf("CompositionReasonStepV1: selected variant has discriminator %q", value.EntryContract.Reason)
		}
		candidate, err := json.Marshal(value.EntryContract)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ResponsibilityContract != nil {
		if value.ResponsibilityContract.Reason != "responsibility_contract" {
			return nil, fmt.Errorf("CompositionReasonStepV1: selected variant has discriminator %q", value.ResponsibilityContract.Reason)
		}
		candidate, err := json.Marshal(value.ResponsibilityContract)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ProviderBinding != nil {
		if value.ProviderBinding.Reason != "provider_binding" {
			return nil, fmt.Errorf("CompositionReasonStepV1: selected variant has discriminator %q", value.ProviderBinding.Reason)
		}
		candidate, err := json.Marshal(value.ProviderBinding)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.DelegationEdge != nil {
		if value.DelegationEdge.Reason != "delegation_edge" {
			return nil, fmt.Errorf("CompositionReasonStepV1: selected variant has discriminator %q", value.DelegationEdge.Reason)
		}
		candidate, err := json.Marshal(value.DelegationEdge)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.TerminationContract != nil {
		if value.TerminationContract.Reason != "termination_contract" {
			return nil, fmt.Errorf("CompositionReasonStepV1: selected variant has discriminator %q", value.TerminationContract.Reason)
		}
		candidate, err := json.Marshal(value.TerminationContract)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.SettlementContract != nil {
		if value.SettlementContract.Reason != "settlement_contract" {
			return nil, fmt.Errorf("CompositionReasonStepV1: selected variant has discriminator %q", value.SettlementContract.Reason)
		}
		candidate, err := json.Marshal(value.SettlementContract)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ContinuityContract != nil {
		if value.ContinuityContract.Reason != "continuity_contract" {
			return nil, fmt.Errorf("CompositionReasonStepV1: selected variant has discriminator %q", value.ContinuityContract.Reason)
		}
		candidate, err := json.Marshal(value.ContinuityContract)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("CompositionReasonStepV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *CompositionReasonStepV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Reason string `json:"reason"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = CompositionReasonStepV1{}
	switch discriminator.Reason {
	case "origin_receipt":
		var variant CompositionReasonStepOriginReceiptV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.OriginReceipt = &variant
	case "entry_contract":
		var variant CompositionReasonStepEntryContractV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.EntryContract = &variant
	case "responsibility_contract":
		var variant CompositionReasonStepResponsibilityContractV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ResponsibilityContract = &variant
	case "provider_binding":
		var variant CompositionReasonStepProviderBindingV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ProviderBinding = &variant
	case "delegation_edge":
		var variant CompositionReasonStepDelegationEdgeV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.DelegationEdge = &variant
	case "termination_contract":
		var variant CompositionReasonStepTerminationContractV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.TerminationContract = &variant
	case "settlement_contract":
		var variant CompositionReasonStepSettlementContractV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.SettlementContract = &variant
	case "continuity_contract":
		var variant CompositionReasonStepContinuityContractV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ContinuityContract = &variant
	default:
		return fmt.Errorf("CompositionReasonStepV1: unknown discriminator %q", discriminator.Reason)
	}
	return nil
}
