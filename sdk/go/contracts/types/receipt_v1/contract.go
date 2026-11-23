// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package receipt_v1 contains the generated receipt.v1 data model.
package receipt_v1

import (
	"encoding/json"
	"fmt"
	deferred_effect_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/deferred_effect_v1"
)
import machine_local_root_identity_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/machine_local_root_identity_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type FilesystemDeviceIDV1 string

type FilesystemInodeIDV1 string

type LogicalRepositoryIDV1 string

type MachineLocalEffectKind string

const MachineLocalEffectKindGitExcludeProtection MachineLocalEffectKind = "git_exclude_protection"

type MachineLocalEffectObservationCreateV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// CreatedDirectories corresponds to the JSON schema field "created_directories".
	CreatedDirectories []string `json:"created_directories"`

	// Post corresponds to the JSON schema field "post".
	Post MachineLocalPostFileV1 `json:"post"`
}

type MachineLocalEffectObservationDeleteV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// PreSha256 corresponds to the JSON schema field "pre_sha256".
	PreSha256 sha256_digest_v1.Sha256DigestV1 `json:"pre_sha256"`
}

type MachineLocalEffectObservationSpliceV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// Post corresponds to the JSON schema field "post".
	Post MachineLocalPostFileV1 `json:"post"`

	// PreSha256 corresponds to the JSON schema field "pre_sha256".
	PreSha256 sha256_digest_v1.Sha256DigestV1 `json:"pre_sha256"`
}

type MachineLocalEffectObservationUnchangedV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// State corresponds to the JSON schema field "state".
	State MachineLocalPostFileV1 `json:"state"`
}

type MachineLocalEffectObservationUpdateV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// Post corresponds to the JSON schema field "post".
	Post MachineLocalPostFileV1 `json:"post"`

	// PreSha256 corresponds to the JSON schema field "pre_sha256".
	PreSha256 sha256_digest_v1.Sha256DigestV1 `json:"pre_sha256"`
}

type MachineLocalFileIdentityV1 struct {
	// Device corresponds to the JSON schema field "device".
	Device FilesystemDeviceIDV1 `json:"device"`

	// Inode corresponds to the JSON schema field "inode".
	Inode FilesystemInodeIDV1 `json:"inode"`
}

type MachineLocalPostFileV1 struct {
	// Identity corresponds to the JSON schema field "identity".
	Identity MachineLocalFileIdentityV1 `json:"identity"`

	// Sha256 corresponds to the JSON schema field "sha256".
	Sha256 sha256_digest_v1.Sha256DigestV1 `json:"sha256"`
}

type MachineLocalReceiptOperationV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind MachineLocalEffectKind `json:"kind"`

	// Mode corresponds to the JSON schema field "mode".
	Mode UnixFileMode `json:"mode"`

	// Observation corresponds to the JSON schema field "observation".
	Observation MachineLocalEffectObservationV1 `json:"observation"`

	// RelPath corresponds to the JSON schema field "rel_path".
	RelPath string `json:"rel_path"`

	// Root corresponds to the JSON schema field "root".
	Root machine_local_root_identity_v1.MachineLocalRootIdentityV1 `json:"root"`
}

type MaterializationReceiptTransitionAdvancedV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// PostLineageDigest corresponds to the JSON schema field "post_lineage_digest".
	PostLineageDigest sha256_digest_v1.Sha256DigestV1 `json:"post_lineage_digest"`

	// PostStateDigest corresponds to the JSON schema field "post_state_digest".
	PostStateDigest sha256_digest_v1.Sha256DigestV1 `json:"post_state_digest"`

	// PriorLineageDigest corresponds to the JSON schema field "prior_lineage_digest".
	PriorLineageDigest sha256_digest_v1.Sha256DigestV1 `json:"prior_lineage_digest"`

	// PriorStateDigest corresponds to the JSON schema field "prior_state_digest".
	PriorStateDigest sha256_digest_v1.Sha256DigestV1 `json:"prior_state_digest"`
}

type MaterializationReceiptTransitionInitializedV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// PostLineageDigest corresponds to the JSON schema field "post_lineage_digest".
	PostLineageDigest sha256_digest_v1.Sha256DigestV1 `json:"post_lineage_digest"`

	// PostStateDigest corresponds to the JSON schema field "post_state_digest".
	PostStateDigest sha256_digest_v1.Sha256DigestV1 `json:"post_state_digest"`
}

type MaterializationReceiptTransitionNoOwnedStateV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type MaterializationReceiptTransitionRemovedV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// PriorLineageDigest corresponds to the JSON schema field "prior_lineage_digest".
	PriorLineageDigest sha256_digest_v1.Sha256DigestV1 `json:"prior_lineage_digest"`

	// PriorStateDigest corresponds to the JSON schema field "prior_state_digest".
	PriorStateDigest sha256_digest_v1.Sha256DigestV1 `json:"prior_state_digest"`
}

type NormativePackAttestorV1 string

type NormativePackIDV1 string

type NormativePackReleaseV1 struct {
	// Attestor corresponds to the JSON schema field "attestor".
	Attestor NormativePackAttestorV1 `json:"attestor"`

	// ContentDigest corresponds to the JSON schema field "content_digest".
	ContentDigest sha256_digest_v1.Sha256DigestV1 `json:"content_digest"`

	// ID corresponds to the JSON schema field "id".
	ID NormativePackIDV1 `json:"id"`

	// Version corresponds to the JSON schema field "version".
	Version NormativePackVersionV1 `json:"version"`
}

type NormativePackVersionV1 string

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

type ReceiptOperation struct {
	// Mode corresponds to the JSON schema field "mode".
	Mode UnixFileMode `json:"mode"`

	// Observation corresponds to the JSON schema field "observation".
	Observation SurfaceOperationObservationV1 `json:"observation"`

	// RelPath corresponds to the JSON schema field "rel_path".
	RelPath string `json:"rel_path"`
}

type ReceiptV1 struct {
	// DeferredEffects corresponds to the JSON schema field "deferred_effects".
	DeferredEffects []deferred_effect_v1.DeferredEffectV1 `json:"deferred_effects"`

	// InstanceDigest corresponds to the JSON schema field "instance_digest".
	InstanceDigest sha256_digest_v1.Sha256DigestV1 `json:"instance_digest"`

	// MachineLocalEffects corresponds to the JSON schema field
	// "machine_local_effects".
	MachineLocalEffects []MachineLocalReceiptOperationV1 `json:"machine_local_effects"`

	// ManifestDigest corresponds to the JSON schema field "manifest_digest".
	ManifestDigest sha256_digest_v1.Sha256DigestV1 `json:"manifest_digest"`

	// NormativePackClosure corresponds to the JSON schema field
	// "normative_pack_closure".
	NormativePackClosure []NormativePackReleaseV1 `json:"normative_pack_closure"`

	// OneShotCompilationDigest corresponds to the JSON schema field
	// "one_shot_compilation_digest".
	OneShotCompilationDigest sha256_digest_v1.Sha256DigestV1 `json:"one_shot_compilation_digest"`

	// PackageClosure corresponds to the JSON schema field "package_closure".
	PackageClosure []PackageReleaseV1 `json:"package_closure"`

	// PackageSemanticIrDigest corresponds to the JSON schema field
	// "package_semantic_ir_digest".
	PackageSemanticIrDigest sha256_digest_v1.Sha256DigestV1 `json:"package_semantic_ir_digest"`

	// PlanDigest corresponds to the JSON schema field "plan_digest".
	PlanDigest sha256_digest_v1.Sha256DigestV1 `json:"plan_digest"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// ReceiptDigest corresponds to the JSON schema field "receipt_digest".
	ReceiptDigest sha256_digest_v1.Sha256DigestV1 `json:"receipt_digest"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`

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

	// SurfaceOperations corresponds to the JSON schema field "surface_operations".
	SurfaceOperations []ReceiptOperation `json:"surface_operations"`

	// SurfacePlanDigest corresponds to the JSON schema field "surface_plan_digest".
	SurfacePlanDigest sha256_digest_v1.Sha256DigestV1 `json:"surface_plan_digest"`

	// Transition corresponds to the JSON schema field "transition".
	Transition MaterializationReceiptTransitionV1 `json:"transition"`
}

type SurfaceCreationObservationAbsentFileV1 struct {
	// PostSha256 corresponds to the JSON schema field "post_sha256".
	PostSha256 sha256_digest_v1.Sha256DigestV1 `json:"post_sha256"`

	// Target corresponds to the JSON schema field "target".
	Target string `json:"target"`
}

type SurfaceCreationObservationExistingFileV1 struct {
	// PostSha256 corresponds to the JSON schema field "post_sha256".
	PostSha256 sha256_digest_v1.Sha256DigestV1 `json:"post_sha256"`

	// PreSha256 corresponds to the JSON schema field "pre_sha256".
	PreSha256 sha256_digest_v1.Sha256DigestV1 `json:"pre_sha256"`

	// Target corresponds to the JSON schema field "target".
	Target string `json:"target"`
}

type SurfaceOperationObservationAdoptV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// PostSha256 corresponds to the JSON schema field "post_sha256".
	PostSha256 sha256_digest_v1.Sha256DigestV1 `json:"post_sha256"`

	// PreSha256 corresponds to the JSON schema field "pre_sha256".
	PreSha256 sha256_digest_v1.Sha256DigestV1 `json:"pre_sha256"`
}

type SurfaceOperationObservationCreateV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// Target corresponds to the JSON schema field "target".
	Target SurfaceCreationObservationV1 `json:"target"`
}

type SurfaceOperationObservationDeleteV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// PreSha256 corresponds to the JSON schema field "pre_sha256".
	PreSha256 sha256_digest_v1.Sha256DigestV1 `json:"pre_sha256"`
}

type SurfaceOperationObservationSpliceV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// PostSha256 corresponds to the JSON schema field "post_sha256".
	PostSha256 sha256_digest_v1.Sha256DigestV1 `json:"post_sha256"`

	// PreSha256 corresponds to the JSON schema field "pre_sha256".
	PreSha256 sha256_digest_v1.Sha256DigestV1 `json:"pre_sha256"`
}

type SurfaceOperationObservationUnchangedV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition UnchangedDispositionV1 `json:"disposition"`

	// Sha256 corresponds to the JSON schema field "sha256".
	Sha256 sha256_digest_v1.Sha256DigestV1 `json:"sha256"`
}

type SurfaceOperationObservationUpdateV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// PostSha256 corresponds to the JSON schema field "post_sha256".
	PostSha256 sha256_digest_v1.Sha256DigestV1 `json:"post_sha256"`

	// PreSha256 corresponds to the JSON schema field "pre_sha256".
	PreSha256 sha256_digest_v1.Sha256DigestV1 `json:"pre_sha256"`
}

type UnchangedDispositionV1 string

const UnchangedDispositionV1ModeRepaired UnchangedDispositionV1 = "mode_repaired"
const UnchangedDispositionV1Verified UnchangedDispositionV1 = "verified"

type UnixFileMode int

type MachineLocalEffectObservationV1 struct {
	Create    *MachineLocalEffectObservationCreateV1    `json:"-"`
	Update    *MachineLocalEffectObservationUpdateV1    `json:"-"`
	Splice    *MachineLocalEffectObservationSpliceV1    `json:"-"`
	Unchanged *MachineLocalEffectObservationUnchangedV1 `json:"-"`
	Delete    *MachineLocalEffectObservationDeleteV1    `json:"-"`
}

func (value MachineLocalEffectObservationV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Create != nil {
		if value.Create.Action != "create" {
			return nil, fmt.Errorf("MachineLocalEffectObservationV1: selected variant has discriminator %q", value.Create.Action)
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
			return nil, fmt.Errorf("MachineLocalEffectObservationV1: selected variant has discriminator %q", value.Update.Action)
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
			return nil, fmt.Errorf("MachineLocalEffectObservationV1: selected variant has discriminator %q", value.Splice.Action)
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
			return nil, fmt.Errorf("MachineLocalEffectObservationV1: selected variant has discriminator %q", value.Unchanged.Action)
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
			return nil, fmt.Errorf("MachineLocalEffectObservationV1: selected variant has discriminator %q", value.Delete.Action)
		}
		candidate, err := json.Marshal(value.Delete)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("MachineLocalEffectObservationV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *MachineLocalEffectObservationV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Action string `json:"action"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = MachineLocalEffectObservationV1{}
	switch discriminator.Action {
	case "create":
		var variant MachineLocalEffectObservationCreateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Create = &variant
	case "update":
		var variant MachineLocalEffectObservationUpdateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Update = &variant
	case "splice":
		var variant MachineLocalEffectObservationSpliceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Splice = &variant
	case "unchanged":
		var variant MachineLocalEffectObservationUnchangedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unchanged = &variant
	case "delete":
		var variant MachineLocalEffectObservationDeleteV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Delete = &variant
	default:
		return fmt.Errorf("MachineLocalEffectObservationV1: unknown discriminator %q", discriminator.Action)
	}
	return nil
}

type MaterializationReceiptTransitionV1 struct {
	Initialized  *MaterializationReceiptTransitionInitializedV1  `json:"-"`
	Advanced     *MaterializationReceiptTransitionAdvancedV1     `json:"-"`
	Removed      *MaterializationReceiptTransitionRemovedV1      `json:"-"`
	NoOwnedState *MaterializationReceiptTransitionNoOwnedStateV1 `json:"-"`
}

func (value MaterializationReceiptTransitionV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Initialized != nil {
		if value.Initialized.Kind != "initialized" {
			return nil, fmt.Errorf("MaterializationReceiptTransitionV1: selected variant has discriminator %q", value.Initialized.Kind)
		}
		candidate, err := json.Marshal(value.Initialized)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Advanced != nil {
		if value.Advanced.Kind != "advanced" {
			return nil, fmt.Errorf("MaterializationReceiptTransitionV1: selected variant has discriminator %q", value.Advanced.Kind)
		}
		candidate, err := json.Marshal(value.Advanced)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Removed != nil {
		if value.Removed.Kind != "removed" {
			return nil, fmt.Errorf("MaterializationReceiptTransitionV1: selected variant has discriminator %q", value.Removed.Kind)
		}
		candidate, err := json.Marshal(value.Removed)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.NoOwnedState != nil {
		if value.NoOwnedState.Kind != "no_owned_state" {
			return nil, fmt.Errorf("MaterializationReceiptTransitionV1: selected variant has discriminator %q", value.NoOwnedState.Kind)
		}
		candidate, err := json.Marshal(value.NoOwnedState)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("MaterializationReceiptTransitionV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *MaterializationReceiptTransitionV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = MaterializationReceiptTransitionV1{}
	switch discriminator.Kind {
	case "initialized":
		var variant MaterializationReceiptTransitionInitializedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Initialized = &variant
	case "advanced":
		var variant MaterializationReceiptTransitionAdvancedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Advanced = &variant
	case "removed":
		var variant MaterializationReceiptTransitionRemovedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Removed = &variant
	case "no_owned_state":
		var variant MaterializationReceiptTransitionNoOwnedStateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NoOwnedState = &variant
	default:
		return fmt.Errorf("MaterializationReceiptTransitionV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type SurfaceCreationObservationV1 struct {
	AbsentFile   *SurfaceCreationObservationAbsentFileV1   `json:"-"`
	ExistingFile *SurfaceCreationObservationExistingFileV1 `json:"-"`
}

func (value SurfaceCreationObservationV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.AbsentFile != nil {
		if value.AbsentFile.Target != "absent_file" {
			return nil, fmt.Errorf("SurfaceCreationObservationV1: selected variant has discriminator %q", value.AbsentFile.Target)
		}
		candidate, err := json.Marshal(value.AbsentFile)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ExistingFile != nil {
		if value.ExistingFile.Target != "existing_file" {
			return nil, fmt.Errorf("SurfaceCreationObservationV1: selected variant has discriminator %q", value.ExistingFile.Target)
		}
		candidate, err := json.Marshal(value.ExistingFile)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("SurfaceCreationObservationV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *SurfaceCreationObservationV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Target string `json:"target"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = SurfaceCreationObservationV1{}
	switch discriminator.Target {
	case "absent_file":
		var variant SurfaceCreationObservationAbsentFileV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.AbsentFile = &variant
	case "existing_file":
		var variant SurfaceCreationObservationExistingFileV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ExistingFile = &variant
	default:
		return fmt.Errorf("SurfaceCreationObservationV1: unknown discriminator %q", discriminator.Target)
	}
	return nil
}

type SurfaceOperationObservationV1 struct {
	Create    *SurfaceOperationObservationCreateV1    `json:"-"`
	Adopt     *SurfaceOperationObservationAdoptV1     `json:"-"`
	Update    *SurfaceOperationObservationUpdateV1    `json:"-"`
	Unchanged *SurfaceOperationObservationUnchangedV1 `json:"-"`
	Delete    *SurfaceOperationObservationDeleteV1    `json:"-"`
	Splice    *SurfaceOperationObservationSpliceV1    `json:"-"`
}

func (value SurfaceOperationObservationV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Create != nil {
		if value.Create.Action != "create" {
			return nil, fmt.Errorf("SurfaceOperationObservationV1: selected variant has discriminator %q", value.Create.Action)
		}
		candidate, err := json.Marshal(value.Create)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Adopt != nil {
		if value.Adopt.Action != "adopt" {
			return nil, fmt.Errorf("SurfaceOperationObservationV1: selected variant has discriminator %q", value.Adopt.Action)
		}
		candidate, err := json.Marshal(value.Adopt)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Update != nil {
		if value.Update.Action != "update" {
			return nil, fmt.Errorf("SurfaceOperationObservationV1: selected variant has discriminator %q", value.Update.Action)
		}
		candidate, err := json.Marshal(value.Update)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Unchanged != nil {
		if value.Unchanged.Action != "unchanged" {
			return nil, fmt.Errorf("SurfaceOperationObservationV1: selected variant has discriminator %q", value.Unchanged.Action)
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
			return nil, fmt.Errorf("SurfaceOperationObservationV1: selected variant has discriminator %q", value.Delete.Action)
		}
		candidate, err := json.Marshal(value.Delete)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Splice != nil {
		if value.Splice.Action != "splice" {
			return nil, fmt.Errorf("SurfaceOperationObservationV1: selected variant has discriminator %q", value.Splice.Action)
		}
		candidate, err := json.Marshal(value.Splice)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("SurfaceOperationObservationV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *SurfaceOperationObservationV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Action string `json:"action"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = SurfaceOperationObservationV1{}
	switch discriminator.Action {
	case "create":
		var variant SurfaceOperationObservationCreateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Create = &variant
	case "adopt":
		var variant SurfaceOperationObservationAdoptV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Adopt = &variant
	case "update":
		var variant SurfaceOperationObservationUpdateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Update = &variant
	case "unchanged":
		var variant SurfaceOperationObservationUnchangedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unchanged = &variant
	case "delete":
		var variant SurfaceOperationObservationDeleteV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Delete = &variant
	case "splice":
		var variant SurfaceOperationObservationSpliceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Splice = &variant
	default:
		return fmt.Errorf("SurfaceOperationObservationV1: unknown discriminator %q", discriminator.Action)
	}
	return nil
}
