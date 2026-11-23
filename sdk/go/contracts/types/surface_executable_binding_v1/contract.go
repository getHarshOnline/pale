// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package surface_executable_binding_v1 contains the generated surface-executable-binding.v1 data model.
package surface_executable_binding_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type SurfaceExecutableBindingV1 struct {
	// Digest corresponds to the JSON schema field "digest".
	Digest sha256_digest_v1.Sha256DigestV1 `json:"digest"`

	// Exec corresponds to the JSON schema field "exec".
	Exec string `json:"exec"`

	// Protocol corresponds to the JSON schema field "protocol".
	Protocol string `json:"protocol"`

	// Version corresponds to the JSON schema field "version".
	Version string `json:"version"`
}
