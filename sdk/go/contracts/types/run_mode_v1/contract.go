// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package run_mode_v1 contains the generated run-mode.v1 data model.
package run_mode_v1

import (
	"encoding/json"
	"fmt"
)

type ModeAxis string

const ModeAxisIntensity ModeAxis = "intensity"
const ModeAxisTaskStance ModeAxis = "task_stance"

type ModeScope string

const ModeScopeRuntime ModeScope = "runtime"
const ModeScopeSessionOnly ModeScope = "session_only"

type OwnedConfinedRelativePath string

type PackageIDV1 string

type PackageResourceIDV1 string

type PackageResourceLinkExternalV1 struct {
	// Resource corresponds to the JSON schema field "resource".
	Resource PackageResourceRefV1 `json:"resource"`

	// Scope corresponds to the JSON schema field "scope".
	Scope string `json:"scope"`
}

type PackageResourceLinkLocalV1 struct {
	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`

	// Scope corresponds to the JSON schema field "scope".
	Scope string `json:"scope"`
}

type PackageResourceRefV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`
}

type RunMode struct {
	// Aliases corresponds to the JSON schema field "aliases".
	Aliases []string `json:"aliases,omitempty,omitzero"`

	// Axis corresponds to the JSON schema field "axis".
	Axis ModeAxis `json:"axis"`

	// Description corresponds to the JSON schema field "description".
	Description string `json:"description"`

	// Doctrine corresponds to the JSON schema field "doctrine".
	Doctrine *OwnedConfinedRelativePath `json:"doctrine,omitempty,omitzero"`

	// ExactTriggers corresponds to the JSON schema field "exact_triggers".
	ExactTriggers []string `json:"exact_triggers,omitempty,omitzero"`

	// Extends corresponds to the JSON schema field "extends".
	Extends []PackageResourceLinkV1 `json:"extends,omitempty,omitzero"`

	// ID corresponds to the JSON schema field "id".
	ID PackageResourceIDV1 `json:"id"`

	// Scope corresponds to the JSON schema field "scope".
	Scope ModeScope `json:"scope"`
}

type RunModeCatalog struct {
	// Mode corresponds to the JSON schema field "mode".
	Mode []RunMode `json:"mode"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type RunModeV1 struct {
	// RunModes corresponds to the JSON schema field "run_modes".
	RunModes RunModeCatalog `json:"run_modes"`
}

type PackageResourceLinkV1 struct {
	Local    *PackageResourceLinkLocalV1    `json:"-"`
	External *PackageResourceLinkExternalV1 `json:"-"`
}

func (value PackageResourceLinkV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Local != nil {
		if value.Local.Scope != "local" {
			return nil, fmt.Errorf("PackageResourceLinkV1: selected variant has discriminator %q", value.Local.Scope)
		}
		candidate, err := json.Marshal(value.Local)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.External != nil {
		if value.External.Scope != "external" {
			return nil, fmt.Errorf("PackageResourceLinkV1: selected variant has discriminator %q", value.External.Scope)
		}
		candidate, err := json.Marshal(value.External)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("PackageResourceLinkV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *PackageResourceLinkV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Scope string `json:"scope"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = PackageResourceLinkV1{}
	switch discriminator.Scope {
	case "local":
		var variant PackageResourceLinkLocalV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Local = &variant
	case "external":
		var variant PackageResourceLinkExternalV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.External = &variant
	default:
		return fmt.Errorf("PackageResourceLinkV1: unknown discriminator %q", discriminator.Scope)
	}
	return nil
}
