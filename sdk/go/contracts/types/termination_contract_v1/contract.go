// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package termination_contract_v1 contains the generated termination-contract.v1 data model.
package termination_contract_v1

import composition_primitives_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/composition_primitives_v1"

type TerminationContractV1 struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kinds corresponds to the JSON schema field "kinds".
	Kinds []TerminationKindV1 `json:"kinds"`

	// Owner corresponds to the JSON schema field "owner".
	Owner composition_primitives_v1.ParticipantRefInterfaceV1 `json:"owner"`

	// TerminalLeaf corresponds to the JSON schema field "terminal_leaf".
	TerminalLeaf *composition_primitives_v1.ParticipantRefInterfaceV1 `json:"terminal_leaf"`
}

type TerminationKindV1 string

const TerminationKindV1Cancelled TerminationKindV1 = "cancelled"
const TerminationKindV1Completed TerminationKindV1 = "completed"
const TerminationKindV1EffectUnknown TerminationKindV1 = "effect_unknown"
const TerminationKindV1Expired TerminationKindV1 = "expired"
const TerminationKindV1Failed TerminationKindV1 = "failed"
