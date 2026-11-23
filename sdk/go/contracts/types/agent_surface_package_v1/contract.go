// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package agent_surface_package_v1 contains the generated agent-surface-package.v1 data model.
package agent_surface_package_v1

type AgentSurfacePackageV1 struct {
	// License corresponds to the JSON schema field "license".
	License *PackageLicense `json:"license,omitempty,omitzero"`

	// Package corresponds to the JSON schema field "package".
	Package PackageMeta `json:"package"`
}

type OwnedConfinedRelativePath string

type PackageDependencyV1 struct {
	// ID corresponds to the JSON schema field "id".
	ID PackageIDV1 `json:"id"`

	// Version corresponds to the JSON schema field "version".
	Version PackageVersionV1 `json:"version"`
}

type PackageIDV1 string

type PackageLicense struct {
	// Note corresponds to the JSON schema field "note".
	Note PackageLicenseNote `json:"note,omitempty,omitzero"`

	// Spdx corresponds to the JSON schema field "spdx".
	Spdx string `json:"spdx"`
}

type PackageLicenseNote *string

type PackageMeta struct {
	// CompatibleCompilers corresponds to the JSON schema field
	// "compatible_compilers".
	CompatibleCompilers []string `json:"compatible_compilers"`

	// Dependencies corresponds to the JSON schema field "dependencies".
	Dependencies []PackageDependencyV1 `json:"dependencies,omitempty,omitzero"`

	// Description corresponds to the JSON schema field "description".
	Description string `json:"description"`

	// ID corresponds to the JSON schema field "id".
	ID PackageIDV1 `json:"id"`

	// Resources corresponds to the JSON schema field "resources".
	Resources []PackageResourceV1 `json:"resources,omitempty,omitzero"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// Sources corresponds to the JSON schema field "sources".
	Sources []PackageSourceV1 `json:"sources,omitempty,omitzero"`

	// Title corresponds to the JSON schema field "title".
	Title string `json:"title"`

	// Version corresponds to the JSON schema field "version".
	Version PackageVersionV1 `json:"version"`
}

type PackageResourceExposureV1 string

const PackageResourceExposureV1Exported PackageResourceExposureV1 = "exported"
const PackageResourceExposureV1Internal PackageResourceExposureV1 = "internal"

type PackageResourceIDV1 string

type PackageResourceKindV1 string

const PackageResourceKindV1CapabilityPolicy PackageResourceKindV1 = "capability_policy"
const PackageResourceKindV1Doctrine PackageResourceKindV1 = "doctrine"
const PackageResourceKindV1GoverningOrder PackageResourceKindV1 = "governing_order"
const PackageResourceKindV1Operation PackageResourceKindV1 = "operation"
const PackageResourceKindV1OperationArchetype PackageResourceKindV1 = "operation_archetype"
const PackageResourceKindV1PolicyFamily PackageResourceKindV1 = "policy_family"
const PackageResourceKindV1Role PackageResourceKindV1 = "role"
const PackageResourceKindV1RoleArchetype PackageResourceKindV1 = "role_archetype"
const PackageResourceKindV1RunMode PackageResourceKindV1 = "run_mode"
const PackageResourceKindV1Stack PackageResourceKindV1 = "stack"

type PackageResourceV1 struct {
	// DefinitionSource corresponds to the JSON schema field "definition_source".
	DefinitionSource OwnedConfinedRelativePath `json:"definition_source"`

	// Exposure corresponds to the JSON schema field "exposure".
	Exposure PackageResourceExposureV1 `json:"exposure"`

	// ID corresponds to the JSON schema field "id".
	ID PackageResourceIDV1 `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind PackageResourceKindV1 `json:"kind"`
}

type PackageSourceRoleV1 string

const PackageSourceRoleV1Body PackageSourceRoleV1 = "body"
const PackageSourceRoleV1Declaration PackageSourceRoleV1 = "declaration"
const PackageSourceRoleV1Documentation PackageSourceRoleV1 = "documentation"

type PackageSourceV1 struct {
	// Path corresponds to the JSON schema field "path".
	Path OwnedConfinedRelativePath `json:"path"`

	// Role corresponds to the JSON schema field "role".
	Role PackageSourceRoleV1 `json:"role"`
}

const PackageResourceKindV1ToolRecipe PackageResourceKindV1 = "tool_recipe"

type PackageVersionV1 string
