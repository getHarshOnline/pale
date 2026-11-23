// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package diagnostic_v1 contains the generated diagnostic.v1 data model.
package diagnostic_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type DiagnosticDispositionV1 string

const DiagnosticDispositionV1Blocked DiagnosticDispositionV1 = "blocked"
const DiagnosticDispositionV1EffectOutcomeUnknown DiagnosticDispositionV1 = "effect_outcome_unknown"
const DiagnosticDispositionV1InternalFailure DiagnosticDispositionV1 = "internal_failure"
const DiagnosticDispositionV1InvalidInput DiagnosticDispositionV1 = "invalid_input"
const DiagnosticDispositionV1OperationalFailure DiagnosticDispositionV1 = "operational_failure"

type DiagnosticEffectCertaintyV1 string

const DiagnosticEffectCertaintyV1Confirmed DiagnosticEffectCertaintyV1 = "confirmed"
const DiagnosticEffectCertaintyV1NotApplicable DiagnosticEffectCertaintyV1 = "not_applicable"
const DiagnosticEffectCertaintyV1NotStarted DiagnosticEffectCertaintyV1 = "not_started"
const DiagnosticEffectCertaintyV1OutcomeUnknown DiagnosticEffectCertaintyV1 = "outcome_unknown"

type DiagnosticFactV1Wire struct {
	// Key corresponds to the JSON schema field "key".
	Key string `json:"key"`

	// Value corresponds to the JSON schema field "value".
	Value string `json:"value"`
}

type DiagnosticGenerationCorrelationV1Wire struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type DiagnosticReceiptCorrelationV1Wire struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type DiagnosticRequestCorrelationV1Wire struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type DiagnosticRetryV1 string

const DiagnosticRetryV1AfterCorrection DiagnosticRetryV1 = "after_correction"
const DiagnosticRetryV1AfterStateChange DiagnosticRetryV1 = "after_state_change"
const DiagnosticRetryV1NotIndicated DiagnosticRetryV1 = "not_indicated"
const DiagnosticRetryV1ReconcileFirst DiagnosticRetryV1 = "reconcile_first"

type DiagnosticSchemaCoordinateV1Wire struct {
	// RegistryID corresponds to the JSON schema field "registry_id".
	RegistryID string `json:"registry_id"`

	// SchemaPath corresponds to the JSON schema field "schema_path".
	SchemaPath DiagnosticSchemaCoordinateV1WireSchemaPath `json:"schema_path,omitempty,omitzero"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// WireSchemaID corresponds to the JSON schema field "wire_schema_id".
	WireSchemaID *SchemaID `json:"wire_schema_id,omitempty,omitzero"`
}

type DiagnosticSchemaCoordinateV1WireSchemaPath *string

type DiagnosticSeverityV1 string

const DiagnosticSeverityV1Error DiagnosticSeverityV1 = "error"

type DiagnosticSourceV1Wire struct {
	// ByteEnd corresponds to the JSON schema field "byte_end".
	ByteEnd DiagnosticSourceV1WireByteEnd `json:"byte_end,omitempty,omitzero"`

	// ByteStart corresponds to the JSON schema field "byte_start".
	ByteStart DiagnosticSourceV1WireByteStart `json:"byte_start,omitempty,omitzero"`

	// Column corresponds to the JSON schema field "column".
	Column DiagnosticSourceV1WireColumn `json:"column,omitempty,omitzero"`

	// Label corresponds to the JSON schema field "label".
	Label string `json:"label"`

	// Line corresponds to the JSON schema field "line".
	Line DiagnosticSourceV1WireLine `json:"line,omitempty,omitzero"`

	// ValuePath corresponds to the JSON schema field "value_path".
	ValuePath DiagnosticSourceV1WireValuePath `json:"value_path,omitempty,omitzero"`
}

type DiagnosticSourceV1WireByteEnd *int

type DiagnosticSourceV1WireByteStart *int

type DiagnosticSourceV1WireColumn *int

type DiagnosticSourceV1WireLine *int

type DiagnosticSourceV1WireValuePath *string

type DiagnosticStageV1 string

const DiagnosticStageV1Compilation DiagnosticStageV1 = "compilation"
const DiagnosticStageV1ContractValidation DiagnosticStageV1 = "contract_validation"
const DiagnosticStageV1HostObservation DiagnosticStageV1 = "host_observation"
const DiagnosticStageV1InputAdmission DiagnosticStageV1 = "input_admission"
const DiagnosticStageV1Internal DiagnosticStageV1 = "internal"
const DiagnosticStageV1Materialization DiagnosticStageV1 = "materialization"
const DiagnosticStageV1NativeAction DiagnosticStageV1 = "native_action"
const DiagnosticStageV1OperationAdmission DiagnosticStageV1 = "operation_admission"
const DiagnosticStageV1PackageLoading DiagnosticStageV1 = "package_loading"
const DiagnosticStageV1Planning DiagnosticStageV1 = "planning"
const DiagnosticStageV1Rendering DiagnosticStageV1 = "rendering"
const DiagnosticStageV1ResidentAdmission DiagnosticStageV1 = "resident_admission"
const DiagnosticStageV1ResidentRecovery DiagnosticStageV1 = "resident_recovery"
const DiagnosticStageV1SemanticAdmission DiagnosticStageV1 = "semantic_admission"
const DiagnosticStageV1WorkspaceLoading DiagnosticStageV1 = "workspace_loading"

type DiagnosticSubjectV1Wire struct {
	// ID corresponds to the JSON schema field "id".
	ID string `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type DiagnosticTruncationV1Wire struct {
	// OmittedCauses corresponds to the JSON schema field "omitted_causes".
	OmittedCauses int `json:"omitted_causes"`

	// OmittedFacts corresponds to the JSON schema field "omitted_facts".
	OmittedFacts int `json:"omitted_facts"`

	// OmittedHelp corresponds to the JSON schema field "omitted_help".
	OmittedHelp int `json:"omitted_help"`

	// OmittedNotes corresponds to the JSON schema field "omitted_notes".
	OmittedNotes int `json:"omitted_notes"`

	// ShortenedFields corresponds to the JSON schema field "shortened_fields".
	ShortenedFields int `json:"shortened_fields"`
}

type DiagnosticV1 struct {
	// Causes corresponds to the JSON schema field "causes".
	Causes []string `json:"causes,omitempty,omitzero"`

	// Code corresponds to the JSON schema field "code".
	Code string `json:"code"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition DiagnosticDispositionV1 `json:"disposition"`

	// Effect corresponds to the JSON schema field "effect".
	Effect DiagnosticEffectCertaintyV1 `json:"effect"`

	// Facts corresponds to the JSON schema field "facts".
	Facts []DiagnosticFactV1Wire `json:"facts,omitempty,omitzero"`

	// Generation corresponds to the JSON schema field "generation".
	Generation *DiagnosticGenerationCorrelationV1Wire `json:"generation,omitempty,omitzero"`

	// Help corresponds to the JSON schema field "help".
	Help []string `json:"help,omitempty,omitzero"`

	// Notes corresponds to the JSON schema field "notes".
	Notes []string `json:"notes,omitempty,omitzero"`

	// Receipt corresponds to the JSON schema field "receipt".
	Receipt *DiagnosticReceiptCorrelationV1Wire `json:"receipt,omitempty,omitzero"`

	// Request corresponds to the JSON schema field "request".
	Request *DiagnosticRequestCorrelationV1Wire `json:"request,omitempty,omitzero"`

	// Retry corresponds to the JSON schema field "retry".
	Retry DiagnosticRetryV1 `json:"retry"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaCoordinate corresponds to the JSON schema field "schema_coordinate".
	SchemaCoordinate *DiagnosticSchemaCoordinateV1Wire `json:"schema_coordinate,omitempty,omitzero"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// Severity corresponds to the JSON schema field "severity".
	Severity DiagnosticSeverityV1 `json:"severity"`

	// Source corresponds to the JSON schema field "source".
	Source *DiagnosticSourceV1Wire `json:"source,omitempty,omitzero"`

	// Stage corresponds to the JSON schema field "stage".
	Stage DiagnosticStageV1 `json:"stage"`

	// Subject corresponds to the JSON schema field "subject".
	Subject *DiagnosticSubjectV1Wire `json:"subject,omitempty,omitzero"`

	// Summary corresponds to the JSON schema field "summary".
	Summary string `json:"summary"`

	// Truncation corresponds to the JSON schema field "truncation".
	Truncation DiagnosticTruncationV1Wire `json:"truncation"`
}

type SchemaID string
