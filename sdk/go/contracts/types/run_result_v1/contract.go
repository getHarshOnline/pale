// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package run_result_v1 contains the generated run-result.v1 data model.
package run_result_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type LogicalExecutionWorkspaceIDV1 string

type LogicalProjectIDV1 string

type LogicalRepositoryIDV1 string

type PlanKind string

const PlanKindApply PlanKind = "apply"
const PlanKindRemove PlanKind = "remove"

type RunResultV1 struct {
	// ExecutionWorkspaceID corresponds to the JSON schema field
	// "execution_workspace_id".
	ExecutionWorkspaceID LogicalExecutionWorkspaceIDV1 `json:"execution_workspace_id"`

	// InstanceDigest corresponds to the JSON schema field "instance_digest".
	InstanceDigest sha256_digest_v1.Sha256DigestV1 `json:"instance_digest"`

	// OneShotCompilationDigest corresponds to the JSON schema field
	// "one_shot_compilation_digest".
	OneShotCompilationDigest sha256_digest_v1.Sha256DigestV1 `json:"one_shot_compilation_digest"`

	// PlanKind corresponds to the JSON schema field "plan_kind".
	PlanKind PlanKind `json:"plan_kind"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// ProjectID corresponds to the JSON schema field "project_id".
	ProjectID LogicalProjectIDV1 `json:"project_id"`

	// RunDigest corresponds to the JSON schema field "run_digest".
	RunDigest sha256_digest_v1.Sha256DigestV1 `json:"run_digest"`

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

	// Targets corresponds to the JSON schema field "targets".
	Targets []RunTargetResult `json:"targets"`
}

type RunTargetResultFailedNoEffect struct {
	// Outcome corresponds to the JSON schema field "outcome".
	Outcome string `json:"outcome"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`
}

type RunTargetResultSucceeded struct {
	// Outcome corresponds to the JSON schema field "outcome".
	Outcome string `json:"outcome"`

	// ReceiptDigest corresponds to the JSON schema field "receipt_digest".
	ReceiptDigest sha256_digest_v1.Sha256DigestV1 `json:"receipt_digest"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`
}

type RunTargetResult struct {
	Succeeded      *RunTargetResultSucceeded      `json:"-"`
	FailedNoEffect *RunTargetResultFailedNoEffect `json:"-"`
}

func (value RunTargetResult) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Succeeded != nil {
		if value.Succeeded.Outcome != "succeeded" {
			return nil, fmt.Errorf("RunTargetResult: selected variant has discriminator %q", value.Succeeded.Outcome)
		}
		candidate, err := json.Marshal(value.Succeeded)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.FailedNoEffect != nil {
		if value.FailedNoEffect.Outcome != "failed_no_effect" {
			return nil, fmt.Errorf("RunTargetResult: selected variant has discriminator %q", value.FailedNoEffect.Outcome)
		}
		candidate, err := json.Marshal(value.FailedNoEffect)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("RunTargetResult: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *RunTargetResult) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Outcome string `json:"outcome"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = RunTargetResult{}
	switch discriminator.Outcome {
	case "succeeded":
		var variant RunTargetResultSucceeded
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Succeeded = &variant
	case "failed_no_effect":
		var variant RunTargetResultFailedNoEffect
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.FailedNoEffect = &variant
	default:
		return fmt.Errorf("RunTargetResult: unknown discriminator %q", discriminator.Outcome)
	}
	return nil
}
