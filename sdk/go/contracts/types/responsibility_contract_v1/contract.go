// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package responsibility_contract_v1 contains the generated responsibility-contract.v1 data model.
package responsibility_contract_v1

import composition_primitives_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/composition_primitives_v1"

type EffectClass string

const EffectClassCompute EffectClass = "compute"
const EffectClassExecute EffectClass = "execute"
const EffectClassNetwork EffectClass = "network"
const EffectClassPublish EffectClass = "publish"
const EffectClassRead EffectClass = "read"
const EffectClassWrite EffectClass = "write"

type ResponsibilityContractV1 struct {
	// AuthorityCeiling corresponds to the JSON schema field "authority_ceiling".
	AuthorityCeiling []EffectClass `json:"authority_ceiling"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Participant corresponds to the JSON schema field "participant".
	Participant composition_primitives_v1.ParticipantRefInterfaceV1 `json:"participant"`

	// Responsibilities corresponds to the JSON schema field "responsibilities".
	Responsibilities []composition_primitives_v1.ResponsibilityCommunicationV1 `json:"responsibilities"`
}
