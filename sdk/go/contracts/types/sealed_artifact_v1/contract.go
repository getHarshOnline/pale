// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package sealed_artifact_v1 contains the generated sealed-artifact.v1 data model.
package sealed_artifact_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type SealedArtifactV1 struct {
	// RelPath corresponds to the JSON schema field "rel_path".
	RelPath string `json:"rel_path"`

	// Sha256 corresponds to the JSON schema field "sha256".
	Sha256 sha256_digest_v1.Sha256DigestV1 `json:"sha256"`
}
