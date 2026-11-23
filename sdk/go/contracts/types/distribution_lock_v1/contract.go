// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package distribution_lock_v1 contains the generated distribution-lock.v1 data model.
package distribution_lock_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type DistributionLockV1 struct {
	// Distribution corresponds to the JSON schema field "distribution".
	Distribution DistributionMeta `json:"distribution"`

	// Package corresponds to the JSON schema field "package".
	Package []PackageReleaseV1 `json:"package,omitempty,omitzero"`
}

type DistributionMeta struct {
	// Compiler corresponds to the JSON schema field "compiler".
	Compiler string `json:"compiler"`

	// SchemaManifestSha256 corresponds to the JSON schema field
	// "schema_manifest_sha256".
	SchemaManifestSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_manifest_sha256"`

	// SchemaSetSha256 corresponds to the JSON schema field "schema_set_sha256".
	SchemaSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_set_sha256"`
}

type PackageIDV1 string

type PackageReleaseV1 struct {
	// ContentDigest corresponds to the JSON schema field "content_digest".
	ContentDigest sha256_digest_v1.Sha256DigestV1 `json:"content_digest"`

	// ID corresponds to the JSON schema field "id".
	ID PackageIDV1 `json:"id"`

	// Version corresponds to the JSON schema field "version".
	Version PackageVersionV1 `json:"version"`
}

type PackageVersionV1 string
