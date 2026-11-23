// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package settlement_contract_v1 contains the generated settlement-contract.v1 data model.
package settlement_contract_v1

import composition_primitives_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/composition_primitives_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type EvidenceSettlementIdentityV1 struct {
	// EvidenceSettlementSha256 corresponds to the JSON schema field
	// "evidence_settlement_sha256".
	EvidenceSettlementSha256 sha256_digest_v1.Sha256DigestV1 `json:"evidence_settlement_sha256"`
}

type SettlementContractV1 struct {
	// Disposition corresponds to the JSON schema field "disposition".
	Disposition SettlementDispositionV1 `json:"disposition"`

	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Identity corresponds to the JSON schema field "identity".
	Identity EvidenceSettlementIdentityV1 `json:"identity"`

	// NativeReceiptContract corresponds to the JSON schema field
	// "native_receipt_contract".
	NativeReceiptContract any `json:"native_receipt_contract"`

	// Observer corresponds to the JSON schema field "observer".
	Observer composition_primitives_v1.ParticipantRefInterfaceV1 `json:"observer"`
}

type SettlementDispositionV1 string

const SettlementDispositionV1NativeReceiptRequired SettlementDispositionV1 = "native_receipt_required"
const SettlementDispositionV1OwnerAcceptanceRequired SettlementDispositionV1 = "owner_acceptance_required"
const SettlementDispositionV1ReadBackRequired SettlementDispositionV1 = "read_back_required"
const SettlementDispositionV1Unsettled SettlementDispositionV1 = "unsettled"
