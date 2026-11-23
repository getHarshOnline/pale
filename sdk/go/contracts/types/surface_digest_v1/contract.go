// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package surface_digest_v1 contains the generated surface-digest.v1 data model.
package surface_digest_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import surface_ownership_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/surface_ownership_v1"

type ExecutionPathSelectionIDV1 string

type SurfaceDigestV1 struct {
	// Committable corresponds to the JSON schema field "committable".
	Committable bool `json:"committable"`

	// ExecutionPathConsumers corresponds to the JSON schema field
	// "execution_path_consumers".
	ExecutionPathConsumers []ExecutionPathSelectionIDV1 `json:"execution_path_consumers"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Ownership corresponds to the JSON schema field "ownership".
	Ownership surface_ownership_v1.SurfaceOwnershipV1 `json:"ownership"`

	// RelPath corresponds to the JSON schema field "rel_path".
	RelPath string `json:"rel_path"`

	// Sha256 corresponds to the JSON schema field "sha256".
	Sha256 sha256_digest_v1.Sha256DigestV1 `json:"sha256"`
}
