// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package request_translation_v1 contains the generated request-translation.v1 data model.
package request_translation_v1

import composition_primitives_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/composition_primitives_v1"

type RequestTranslationV1 struct {
	// From corresponds to the JSON schema field "from".
	From composition_primitives_v1.ParticipantRefInterfaceV1 `json:"from"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// InputContract corresponds to the JSON schema field "input_contract".
	InputContract string `json:"input_contract"`

	// OutputContract corresponds to the JSON schema field "output_contract".
	OutputContract string `json:"output_contract"`

	// PreservesAuthority corresponds to the JSON schema field "preserves_authority".
	PreservesAuthority bool `json:"preserves_authority"`

	// To corresponds to the JSON schema field "to".
	To composition_primitives_v1.ParticipantRefInterfaceV1 `json:"to"`
}
