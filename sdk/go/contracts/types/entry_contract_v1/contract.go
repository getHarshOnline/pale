// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package entry_contract_v1 contains the generated entry-contract.v1 data model.
package entry_contract_v1

import composition_primitives_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/composition_primitives_v1"

type EntryContractV1 struct {
	// Entry corresponds to the JSON schema field "entry".
	Entry composition_primitives_v1.ParticipantRefInterfaceV1 `json:"entry"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Origin corresponds to the JSON schema field "origin".
	Origin composition_primitives_v1.EntryOriginRefUserV1 `json:"origin"`

	// ResponsibilityContractID corresponds to the JSON schema field
	// "responsibility_contract_id".
	ResponsibilityContractID string `json:"responsibility_contract_id"`
}
