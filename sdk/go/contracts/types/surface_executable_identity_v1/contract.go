// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package surface_executable_identity_v1 contains the generated surface-executable-identity.v1 data model.
package surface_executable_identity_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type DurablePublication string

const DurablePublicationAppleFullFsync DurablePublication = "apple-full-fsync"
const DurablePublicationStandardSyncAll DurablePublication = "standard-sync-all"
const DurablePublicationUnixFsync DurablePublication = "unix-fsync"

type SurfaceExecutableIdentityV1 struct {
	// Arch corresponds to the JSON schema field "arch".
	Arch string `json:"arch"`

	// DurablePublication corresponds to the JSON schema field "durable_publication".
	DurablePublication DurablePublication `json:"durable_publication"`

	// Os corresponds to the JSON schema field "os".
	Os string `json:"os"`

	// Protocol corresponds to the JSON schema field "protocol".
	Protocol string `json:"protocol"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// Version corresponds to the JSON schema field "version".
	Version string `json:"version"`
}
