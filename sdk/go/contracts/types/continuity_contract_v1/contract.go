// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package continuity_contract_v1 contains the generated continuity-contract.v1 data model.
package continuity_contract_v1

import composition_primitives_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/composition_primitives_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type ContinuityContractV1 struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Identity corresponds to the JSON schema field "identity".
	Identity ContinuityIdentityV1 `json:"identity"`

	// Obligations corresponds to the JSON schema field "obligations".
	Obligations []string `json:"obligations"`

	// Owner corresponds to the JSON schema field "owner".
	Owner composition_primitives_v1.ParticipantRefInterfaceV1 `json:"owner"`
}

type ContinuityIdentityV1 struct {
	// ContinuitySha256 corresponds to the JSON schema field "continuity_sha256".
	ContinuitySha256 sha256_digest_v1.Sha256DigestV1 `json:"continuity_sha256"`
}
