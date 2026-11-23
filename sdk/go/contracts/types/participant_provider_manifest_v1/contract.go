// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package participant_provider_manifest_v1 contains the generated participant-provider-manifest.v1 data model.
package participant_provider_manifest_v1

import composition_primitives_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/composition_primitives_v1"

type ParticipantProviderManifestV1 struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Participants corresponds to the JSON schema field "participants".
	Participants []composition_primitives_v1.ParticipantRefInterfaceV1 `json:"participants"`

	// Providers corresponds to the JSON schema field "providers".
	Providers []ProviderCompatibilityDeclarationV1 `json:"providers"`
}

type ProviderCompatibilityDeclarationV1 struct {
	// CapabilityIds corresponds to the JSON schema field "capability_ids".
	CapabilityIds []string `json:"capability_ids"`

	// CompatibleProviderIds corresponds to the JSON schema field
	// "compatible_provider_ids".
	CompatibleProviderIds []string `json:"compatible_provider_ids"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Participant corresponds to the JSON schema field "participant".
	Participant composition_primitives_v1.ParticipantRefInterfaceV1 `json:"participant"`
}
