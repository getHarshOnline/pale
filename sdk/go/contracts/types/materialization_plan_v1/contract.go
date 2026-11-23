// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package materialization_plan_v1 contains the generated materialization-plan.v1 data model.
package materialization_plan_v1

import (
	"encoding/json"
	"fmt"
	agent_surface_plan_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/agent_surface_plan_v1"
)
import machine_local_root_identity_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/machine_local_root_identity_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type ExistingMachineLocalFileV1 struct {
	// Identity corresponds to the JSON schema field "identity".
	Identity MachineLocalFileIdentityV1 `json:"identity"`

	// Mode corresponds to the JSON schema field "mode".
	Mode UnixFileMode `json:"mode"`

	// Sha256 corresponds to the JSON schema field "sha256".
	Sha256 sha256_digest_v1.Sha256DigestV1 `json:"sha256"`
}

type FilesystemDeviceIDV1 string

type FilesystemInodeIDV1 string

type MachineLocalDirectoryIdentity struct {
	// Device corresponds to the JSON schema field "device".
	Device FilesystemDeviceIDV1 `json:"device"`

	// Inode corresponds to the JSON schema field "inode".
	Inode FilesystemInodeIDV1 `json:"inode"`

	// RelPath corresponds to the JSON schema field "rel_path".
	RelPath string `json:"rel_path"`
}

type MachineLocalEffectKind string

const MachineLocalEffectKindGitExcludeProtection MachineLocalEffectKind = "git_exclude_protection"

type MachineLocalEffectTransitionCreateV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// CreateDirectories corresponds to the JSON schema field "create_directories".
	CreateDirectories []string `json:"create_directories"`

	// Post corresponds to the JSON schema field "post".
	Post PlannedFileV1 `json:"post"`
}

type MachineLocalEffectTransitionDeleteV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// Prior corresponds to the JSON schema field "prior".
	Prior ExistingMachineLocalFileV1 `json:"prior"`
}

type MachineLocalEffectTransitionSpliceV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// Post corresponds to the JSON schema field "post".
	Post PlannedFileV1 `json:"post"`

	// Prior corresponds to the JSON schema field "prior".
	Prior ExistingMachineLocalFileV1 `json:"prior"`
}

type MachineLocalEffectTransitionUnchangedV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// State corresponds to the JSON schema field "state".
	State ExistingMachineLocalFileV1 `json:"state"`
}

type MachineLocalEffectTransitionUpdateV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// Post corresponds to the JSON schema field "post".
	Post PlannedFileV1 `json:"post"`

	// Prior corresponds to the JSON schema field "prior".
	Prior ExistingMachineLocalFileV1 `json:"prior"`
}

type MachineLocalEffectV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind MachineLocalEffectKind `json:"kind"`

	// RelPath corresponds to the JSON schema field "rel_path".
	RelPath string `json:"rel_path"`

	// RetainedDirectories corresponds to the JSON schema field
	// "retained_directories".
	RetainedDirectories []MachineLocalDirectoryIdentity `json:"retained_directories"`

	// Root corresponds to the JSON schema field "root".
	Root machine_local_root_identity_v1.MachineLocalRootIdentityV1 `json:"root"`

	// Transition corresponds to the JSON schema field "transition".
	Transition MachineLocalEffectTransitionV1 `json:"transition"`
}

type MachineLocalFileIdentityV1 struct {
	// Device corresponds to the JSON schema field "device".
	Device FilesystemDeviceIDV1 `json:"device"`

	// Inode corresponds to the JSON schema field "inode".
	Inode FilesystemInodeIDV1 `json:"inode"`
}

type MaterializationPlanV1 struct {
	// MachineLocalEffects corresponds to the JSON schema field
	// "machine_local_effects".
	MachineLocalEffects []MachineLocalEffectV1 `json:"machine_local_effects"`

	// OneShotCompilationDigest corresponds to the JSON schema field
	// "one_shot_compilation_digest".
	OneShotCompilationDigest sha256_digest_v1.Sha256DigestV1 `json:"one_shot_compilation_digest"`

	// PlanDigest corresponds to the JSON schema field "plan_digest".
	PlanDigest sha256_digest_v1.Sha256DigestV1 `json:"plan_digest"`

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

	// SurfacePlan corresponds to the JSON schema field "surface_plan".
	SurfacePlan agent_surface_plan_v1.AgentSurfacePlanV1 `json:"surface_plan"`
}

type PlannedFileV1 struct {
	// Mode corresponds to the JSON schema field "mode".
	Mode UnixFileMode `json:"mode"`

	// Sha256 corresponds to the JSON schema field "sha256".
	Sha256 sha256_digest_v1.Sha256DigestV1 `json:"sha256"`
}

type UnixFileMode int

type MachineLocalEffectTransitionV1 struct {
	Create    *MachineLocalEffectTransitionCreateV1    `json:"-"`
	Update    *MachineLocalEffectTransitionUpdateV1    `json:"-"`
	Splice    *MachineLocalEffectTransitionSpliceV1    `json:"-"`
	Unchanged *MachineLocalEffectTransitionUnchangedV1 `json:"-"`
	Delete    *MachineLocalEffectTransitionDeleteV1    `json:"-"`
}

func (value MachineLocalEffectTransitionV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Create != nil {
		if value.Create.Action != "create" {
			return nil, fmt.Errorf("MachineLocalEffectTransitionV1: selected variant has discriminator %q", value.Create.Action)
		}
		candidate, err := json.Marshal(value.Create)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Update != nil {
		if value.Update.Action != "update" {
			return nil, fmt.Errorf("MachineLocalEffectTransitionV1: selected variant has discriminator %q", value.Update.Action)
		}
		candidate, err := json.Marshal(value.Update)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Splice != nil {
		if value.Splice.Action != "splice" {
			return nil, fmt.Errorf("MachineLocalEffectTransitionV1: selected variant has discriminator %q", value.Splice.Action)
		}
		candidate, err := json.Marshal(value.Splice)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Unchanged != nil {
		if value.Unchanged.Action != "unchanged" {
			return nil, fmt.Errorf("MachineLocalEffectTransitionV1: selected variant has discriminator %q", value.Unchanged.Action)
		}
		candidate, err := json.Marshal(value.Unchanged)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Delete != nil {
		if value.Delete.Action != "delete" {
			return nil, fmt.Errorf("MachineLocalEffectTransitionV1: selected variant has discriminator %q", value.Delete.Action)
		}
		candidate, err := json.Marshal(value.Delete)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("MachineLocalEffectTransitionV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *MachineLocalEffectTransitionV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Action string `json:"action"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = MachineLocalEffectTransitionV1{}
	switch discriminator.Action {
	case "create":
		var variant MachineLocalEffectTransitionCreateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Create = &variant
	case "update":
		var variant MachineLocalEffectTransitionUpdateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Update = &variant
	case "splice":
		var variant MachineLocalEffectTransitionSpliceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Splice = &variant
	case "unchanged":
		var variant MachineLocalEffectTransitionUnchangedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unchanged = &variant
	case "delete":
		var variant MachineLocalEffectTransitionDeleteV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Delete = &variant
	default:
		return fmt.Errorf("MachineLocalEffectTransitionV1: unknown discriminator %q", discriminator.Action)
	}
	return nil
}
