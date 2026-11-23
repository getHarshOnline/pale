// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package expiring_record_envelope_v1 contains the generated expiring-record-envelope.v1 data model.
package expiring_record_envelope_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type ExpiringRecordClassV1 string

const ExpiringRecordClassV1DurableEvidence ExpiringRecordClassV1 = "durable-evidence"
const ExpiringRecordClassV1SessionContinuity ExpiringRecordClassV1 = "session-continuity"

type ExpiringRecordEnvelopeV1 struct {
	// CompilationInputDigest corresponds to the JSON schema field
	// "compilation_input_digest".
	CompilationInputDigest sha256_digest_v1.Sha256DigestV1 `json:"compilation_input_digest"`

	// PayloadSchema corresponds to the JSON schema field "payload_schema".
	PayloadSchema SchemaID `json:"payload_schema"`

	// PayloadSchemaSha256 corresponds to the JSON schema field
	// "payload_schema_sha256".
	PayloadSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"payload_schema_sha256"`

	// PublicationGeneration corresponds to the JSON schema field
	// "publication_generation".
	PublicationGeneration int `json:"publication_generation"`

	// RecordBinding corresponds to the JSON schema field "record_binding".
	RecordBinding sha256_digest_v1.Sha256DigestV1 `json:"record_binding"`

	// RecordClass corresponds to the JSON schema field "record_class".
	RecordClass ExpiringRecordClassV1 `json:"record_class"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaManifestSha256 corresponds to the JSON schema field
	// "schema_manifest_sha256".
	SchemaManifestSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_manifest_sha256"`

	// SchemaSetSha256 corresponds to the JSON schema field "schema_set_sha256".
	SchemaSetSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_set_sha256"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// State corresponds to the JSON schema field "state".
	State ExpiringRecordEnvelopeV1State `json:"state"`

	// UpdatedSubsecNanos corresponds to the JSON schema field "updated_subsec_nanos".
	UpdatedSubsecNanos int `json:"updated_subsec_nanos,omitempty,omitzero"`

	// UpdatedUnixSeconds corresponds to the JSON schema field "updated_unix_seconds".
	UpdatedUnixSeconds int `json:"updated_unix_seconds"`
}

type ExpiringRecordEnvelopeV1State map[string]any

type SchemaID string
