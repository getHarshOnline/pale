// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package workspace_composition_source_v1 contains the generated workspace-composition-source.v1 data model.
package workspace_composition_source_v1

import composition_edge_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/composition_edge_v1"
import continuity_contract_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/continuity_contract_v1"
import entry_contract_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/entry_contract_v1"
import experience_path_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/experience_path_v1"
import participant_provider_manifest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/participant_provider_manifest_v1"
import provider_binding_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/provider_binding_v1"
import request_translation_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/request_translation_v1"
import responsibility_contract_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/responsibility_contract_v1"
import result_translation_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/result_translation_v1"
import settlement_contract_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/settlement_contract_v1"
import termination_contract_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/termination_contract_v1"

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

type WorkspaceCompositionSourceV1 struct {
	// Declarations corresponds to the JSON schema field "declarations".
	Declarations ExperienceCompositionDeclarationsV1 `json:"declarations"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}
