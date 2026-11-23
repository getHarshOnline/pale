// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package agent_surface_plan_v1 contains the generated agent-surface-plan.v1 data model.
package agent_surface_plan_v1

import (
	"encoding/json"
	"fmt"
	deferred_effect_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/deferred_effect_v1"
)
import materialization_state_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/materialization_state_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
import surface_ownership_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/surface_ownership_v1"

type AgentSurfacePlanV1 struct {
	// Admission corresponds to the JSON schema field "admission".
	Admission string `json:"admission"`

	// CompilerVersion corresponds to the JSON schema field "compiler_version".
	CompilerVersion string `json:"compiler_version"`

	// DeferredEffects corresponds to the JSON schema field "deferred_effects".
	DeferredEffects []deferred_effect_v1.DeferredEffectV1 `json:"deferred_effects"`

	// InstanceDigest corresponds to the JSON schema field "instance_digest".
	InstanceDigest sha256_digest_v1.Sha256DigestV1 `json:"instance_digest"`

	// ManifestDigest corresponds to the JSON schema field "manifest_digest".
	ManifestDigest sha256_digest_v1.Sha256DigestV1 `json:"manifest_digest"`

	// NormativePackClosure corresponds to the JSON schema field
	// "normative_pack_closure".
	NormativePackClosure []NormativePackReleaseV1 `json:"normative_pack_closure"`

	// OneShotCompilationDigest corresponds to the JSON schema field
	// "one_shot_compilation_digest".
	OneShotCompilationDigest sha256_digest_v1.Sha256DigestV1 `json:"one_shot_compilation_digest"`

	// OperationMarker corresponds to the JSON schema field "operation_marker".
	OperationMarker string `json:"operation_marker"`

	// Operations corresponds to the JSON schema field "operations".
	Operations []Operation `json:"operations"`

	// PackageClosure corresponds to the JSON schema field "package_closure".
	PackageClosure []PackageReleaseV1 `json:"package_closure"`

	// PackageSemanticIrDigest corresponds to the JSON schema field
	// "package_semantic_ir_digest".
	PackageSemanticIrDigest sha256_digest_v1.Sha256DigestV1 `json:"package_semantic_ir_digest"`

	// PlanDigest corresponds to the JSON schema field "plan_digest".
	PlanDigest sha256_digest_v1.Sha256DigestV1 `json:"plan_digest"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// RendererVersion corresponds to the JSON schema field "renderer_version".
	RendererVersion string `json:"renderer_version"`

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

	// TargetBaseline corresponds to the JSON schema field "target_baseline".
	TargetBaseline []BaselineEntry `json:"target_baseline"`

	// Transition corresponds to the JSON schema field "transition".
	Transition MaterializationStateTransitionV1 `json:"transition"`
}

type BaselineEntry struct {
	// Observation corresponds to the JSON schema field "observation".
	Observation BaselineStateV1 `json:"observation"`

	// RelPath corresponds to the JSON schema field "rel_path".
	RelPath string `json:"rel_path"`
}

type BaselineStateAbsentV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type BaselineStatePresentV1 struct {
	// Mode corresponds to the JSON schema field "mode".
	Mode UnixFileMode `json:"mode"`

	// Sha256 corresponds to the JSON schema field "sha256".
	Sha256 sha256_digest_v1.Sha256DigestV1 `json:"sha256"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type LogicalRepositoryIDV1 string

type MaterializationStateTransitionAdvanceV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// NextState corresponds to the JSON schema field "next_state".
	NextState materialization_state_v1.MaterializationStateV1 `json:"next_state"`

	// Prior corresponds to the JSON schema field "prior".
	Prior PriorMaterializationStateV1 `json:"prior"`
}

type MaterializationStateTransitionInitializeV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// NextState corresponds to the JSON schema field "next_state".
	NextState materialization_state_v1.MaterializationStateV1 `json:"next_state"`
}

type MaterializationStateTransitionRemoveUnownedV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type MaterializationStateTransitionRemoveV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Prior corresponds to the JSON schema field "prior".
	Prior PriorMaterializationStateV1 `json:"prior"`
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

type Operation struct {
	// Committable corresponds to the JSON schema field "committable".
	Committable bool `json:"committable"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Mode corresponds to the JSON schema field "mode".
	Mode UnixFileMode `json:"mode"`

	// RelPath corresponds to the JSON schema field "rel_path".
	RelPath string `json:"rel_path"`

	// Transition corresponds to the JSON schema field "transition".
	Transition SurfaceOperationTransitionV1 `json:"transition"`
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

type PriorMaterializationStateV1 struct {
	// OneShotCompilationDigest corresponds to the JSON schema field
	// "one_shot_compilation_digest".
	OneShotCompilationDigest sha256_digest_v1.Sha256DigestV1 `json:"one_shot_compilation_digest"`

	// StateDigest corresponds to the JSON schema field "state_digest".
	StateDigest sha256_digest_v1.Sha256DigestV1 `json:"state_digest"`
}

type SurfaceOperationTransitionAdoptV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// DesiredSha256 corresponds to the JSON schema field "desired_sha256".
	DesiredSha256 sha256_digest_v1.Sha256DigestV1 `json:"desired_sha256"`

	// Ownership corresponds to the JSON schema field "ownership".
	Ownership surface_ownership_v1.SurfaceOwnershipV1 `json:"ownership"`
}

type SurfaceOperationTransitionCreateV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// DesiredSha256 corresponds to the JSON schema field "desired_sha256".
	DesiredSha256 sha256_digest_v1.Sha256DigestV1 `json:"desired_sha256"`

	// Ownership corresponds to the JSON schema field "ownership".
	Ownership surface_ownership_v1.SurfaceOwnershipV1 `json:"ownership"`
}

type SurfaceOperationTransitionDeleteV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// PriorOwnedSha256 corresponds to the JSON schema field "prior_owned_sha256".
	PriorOwnedSha256 sha256_digest_v1.Sha256DigestV1 `json:"prior_owned_sha256"`
}

type SurfaceOperationTransitionSpliceV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// PriorOwnedSha256 corresponds to the JSON schema field "prior_owned_sha256".
	PriorOwnedSha256 sha256_digest_v1.Sha256DigestV1 `json:"prior_owned_sha256"`
}

type SurfaceOperationTransitionUnchangedV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// OwnedSha256 corresponds to the JSON schema field "owned_sha256".
	OwnedSha256 sha256_digest_v1.Sha256DigestV1 `json:"owned_sha256"`

	// Ownership corresponds to the JSON schema field "ownership".
	Ownership surface_ownership_v1.SurfaceOwnershipV1 `json:"ownership"`
}

type SurfaceOperationTransitionUpdateV1 struct {
	// Action corresponds to the JSON schema field "action".
	Action string `json:"action"`

	// DesiredSha256 corresponds to the JSON schema field "desired_sha256".
	DesiredSha256 sha256_digest_v1.Sha256DigestV1 `json:"desired_sha256"`

	// Ownership corresponds to the JSON schema field "ownership".
	Ownership surface_ownership_v1.SurfaceOwnershipV1 `json:"ownership"`
}

type UnixFileMode int

type BaselineStateV1 struct {
	Absent  *BaselineStateAbsentV1  `json:"-"`
	Present *BaselineStatePresentV1 `json:"-"`
}

func (value BaselineStateV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Absent != nil {
		if value.Absent.State != "absent" {
			return nil, fmt.Errorf("BaselineStateV1: selected variant has discriminator %q", value.Absent.State)
		}
		candidate, err := json.Marshal(value.Absent)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Present != nil {
		if value.Present.State != "present" {
			return nil, fmt.Errorf("BaselineStateV1: selected variant has discriminator %q", value.Present.State)
		}
		candidate, err := json.Marshal(value.Present)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("BaselineStateV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *BaselineStateV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		State string `json:"state"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = BaselineStateV1{}
	switch discriminator.State {
	case "absent":
		var variant BaselineStateAbsentV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Absent = &variant
	case "present":
		var variant BaselineStatePresentV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Present = &variant
	default:
		return fmt.Errorf("BaselineStateV1: unknown discriminator %q", discriminator.State)
	}
	return nil
}

type MaterializationStateTransitionV1 struct {
	Initialize    *MaterializationStateTransitionInitializeV1    `json:"-"`
	Advance       *MaterializationStateTransitionAdvanceV1       `json:"-"`
	Remove        *MaterializationStateTransitionRemoveV1        `json:"-"`
	RemoveUnowned *MaterializationStateTransitionRemoveUnownedV1 `json:"-"`
}

func (value MaterializationStateTransitionV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Initialize != nil {
		if value.Initialize.Kind != "initialize" {
			return nil, fmt.Errorf("MaterializationStateTransitionV1: selected variant has discriminator %q", value.Initialize.Kind)
		}
		candidate, err := json.Marshal(value.Initialize)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Advance != nil {
		if value.Advance.Kind != "advance" {
			return nil, fmt.Errorf("MaterializationStateTransitionV1: selected variant has discriminator %q", value.Advance.Kind)
		}
		candidate, err := json.Marshal(value.Advance)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Remove != nil {
		if value.Remove.Kind != "remove" {
			return nil, fmt.Errorf("MaterializationStateTransitionV1: selected variant has discriminator %q", value.Remove.Kind)
		}
		candidate, err := json.Marshal(value.Remove)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.RemoveUnowned != nil {
		if value.RemoveUnowned.Kind != "remove_unowned" {
			return nil, fmt.Errorf("MaterializationStateTransitionV1: selected variant has discriminator %q", value.RemoveUnowned.Kind)
		}
		candidate, err := json.Marshal(value.RemoveUnowned)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("MaterializationStateTransitionV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *MaterializationStateTransitionV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = MaterializationStateTransitionV1{}
	switch discriminator.Kind {
	case "initialize":
		var variant MaterializationStateTransitionInitializeV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Initialize = &variant
	case "advance":
		var variant MaterializationStateTransitionAdvanceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Advance = &variant
	case "remove":
		var variant MaterializationStateTransitionRemoveV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Remove = &variant
	case "remove_unowned":
		var variant MaterializationStateTransitionRemoveUnownedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.RemoveUnowned = &variant
	default:
		return fmt.Errorf("MaterializationStateTransitionV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type SurfaceOperationTransitionV1 struct {
	Create    *SurfaceOperationTransitionCreateV1    `json:"-"`
	Adopt     *SurfaceOperationTransitionAdoptV1     `json:"-"`
	Update    *SurfaceOperationTransitionUpdateV1    `json:"-"`
	Unchanged *SurfaceOperationTransitionUnchangedV1 `json:"-"`
	Delete    *SurfaceOperationTransitionDeleteV1    `json:"-"`
	Splice    *SurfaceOperationTransitionSpliceV1    `json:"-"`
}

func (value SurfaceOperationTransitionV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Create != nil {
		if value.Create.Action != "create" {
			return nil, fmt.Errorf("SurfaceOperationTransitionV1: selected variant has discriminator %q", value.Create.Action)
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
			return nil, fmt.Errorf("SurfaceOperationTransitionV1: selected variant has discriminator %q", value.Adopt.Action)
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
			return nil, fmt.Errorf("SurfaceOperationTransitionV1: selected variant has discriminator %q", value.Update.Action)
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
			return nil, fmt.Errorf("SurfaceOperationTransitionV1: selected variant has discriminator %q", value.Unchanged.Action)
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
			return nil, fmt.Errorf("SurfaceOperationTransitionV1: selected variant has discriminator %q", value.Delete.Action)
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
			return nil, fmt.Errorf("SurfaceOperationTransitionV1: selected variant has discriminator %q", value.Splice.Action)
		}
		candidate, err := json.Marshal(value.Splice)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("SurfaceOperationTransitionV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *SurfaceOperationTransitionV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Action string `json:"action"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = SurfaceOperationTransitionV1{}
	switch discriminator.Action {
	case "create":
		var variant SurfaceOperationTransitionCreateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Create = &variant
	case "adopt":
		var variant SurfaceOperationTransitionAdoptV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Adopt = &variant
	case "update":
		var variant SurfaceOperationTransitionUpdateV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Update = &variant
	case "unchanged":
		var variant SurfaceOperationTransitionUnchangedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unchanged = &variant
	case "delete":
		var variant SurfaceOperationTransitionDeleteV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Delete = &variant
	case "splice":
		var variant SurfaceOperationTransitionSpliceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Splice = &variant
	default:
		return fmt.Errorf("SurfaceOperationTransitionV1: unknown discriminator %q", discriminator.Action)
	}
	return nil
}
