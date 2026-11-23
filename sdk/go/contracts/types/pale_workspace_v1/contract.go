// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package pale_workspace_v1 contains the generated pale-workspace.v1 data model.
package pale_workspace_v1

import (
	"encoding/json"
	"fmt"
	lifecycle_event_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/lifecycle_event_v1"
)

type AuthorityAccess struct {
	// Class corresponds to the JSON schema field "class".
	Class AuthorityAccessClass `json:"class"`

	// RootID corresponds to the JSON schema field "root_id".
	RootID string `json:"root_id"`
}

type AuthorityAccessClass string

const AuthorityAccessClassReadOnly AuthorityAccessClass = "read-only"

type ExecutionPathSelectionIDV1 string

type ExecutionPathSelectionV1 struct {
	// ExperiencePathID corresponds to the JSON schema field "experience_path_id".
	ExperiencePathID string `json:"experience_path_id"`

	// ID corresponds to the JSON schema field "id".
	ID ExecutionPathSelectionIDV1 `json:"id"`
}

type ExecutionWorkspaceRepositoryAccessV1 string

const ExecutionWorkspaceRepositoryAccessV1Mutate ExecutionWorkspaceRepositoryAccessV1 = "mutate"
const ExecutionWorkspaceRepositoryAccessV1Read ExecutionWorkspaceRepositoryAccessV1 = "read"

type ExecutionWorkspaceRepositoryMembershipV1 struct {
	// Access corresponds to the JSON schema field "access".
	Access ExecutionWorkspaceRepositoryAccessV1 `json:"access"`

	// ExecutionWorkspaceID corresponds to the JSON schema field
	// "execution_workspace_id".
	ExecutionWorkspaceID LogicalExecutionWorkspaceIDV1 `json:"execution_workspace_id"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`
}

type LogicalExecutionWorkspaceIDV1 string

type LogicalExecutionWorkspaceV1 struct {
	// ExecutionPathIds corresponds to the JSON schema field "execution_path_ids".
	ExecutionPathIds []ExecutionPathSelectionIDV1 `json:"execution_path_ids"`

	// ID corresponds to the JSON schema field "id".
	ID LogicalExecutionWorkspaceIDV1 `json:"id"`

	// Session corresponds to the JSON schema field "session".
	Session *SessionModel `json:"session,omitempty,omitzero"`

	// Title corresponds to the JSON schema field "title".
	Title string `json:"title"`
}

type LogicalProjectIDV1 string

type LogicalProjectV1 struct {
	// AuthorityRootIds corresponds to the JSON schema field "authority_root_ids".
	AuthorityRootIds []string `json:"authority_root_ids"`

	// CapabilitiesFile corresponds to the JSON schema field "capabilities_file".
	CapabilitiesFile OwnedConfinedRelativePath `json:"capabilities_file"`

	// ID corresponds to the JSON schema field "id".
	ID LogicalProjectIDV1 `json:"id"`

	// Owner corresponds to the JSON schema field "owner".
	Owner *ProjectOwnerReferenceV1 `json:"owner,omitempty,omitzero"`

	// PackageSeeds corresponds to the JSON schema field "package_seeds".
	PackageSeeds []PackageIDV1 `json:"package_seeds"`

	// RunPolicy corresponds to the JSON schema field "run_policy".
	RunPolicy RunPolicyIDV1 `json:"run_policy"`

	// Title corresponds to the JSON schema field "title".
	Title string `json:"title"`
}

type LogicalRepositoryIDV1 string

type LogicalRepositoryV1 struct {
	// ID corresponds to the JSON schema field "id".
	ID LogicalRepositoryIDV1 `json:"id"`

	// Title corresponds to the JSON schema field "title".
	Title string `json:"title"`
}

type OwnedConfinedRelativePath string

type PackageIDV1 string

type PackageResourceIDV1 string

type PackageResourceRefV1 struct {
	// PackageID corresponds to the JSON schema field "package_id".
	PackageID PackageIDV1 `json:"package_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID PackageResourceIDV1 `json:"resource_id"`
}

type PaleWorkspaceV1 struct {
	// Workspace corresponds to the JSON schema field "workspace".
	Workspace WorkspaceMeta `json:"workspace"`
}

type ProjectExecutionWorkspaceMembershipV1 struct {
	// ExecutionWorkspaceID corresponds to the JSON schema field
	// "execution_workspace_id".
	ExecutionWorkspaceID LogicalExecutionWorkspaceIDV1 `json:"execution_workspace_id"`

	// ProjectID corresponds to the JSON schema field "project_id".
	ProjectID LogicalProjectIDV1 `json:"project_id"`
}

type ProjectOwnerReferenceOrganizationV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ModuleID corresponds to the JSON schema field "module_id".
	ModuleID WorkspaceModuleID `json:"module_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID WorkspaceResourceID `json:"resource_id"`
}

type ProjectOwnerReferencePrincipalV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ModuleID corresponds to the JSON schema field "module_id".
	ModuleID WorkspaceModuleID `json:"module_id"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID WorkspaceResourceID `json:"resource_id"`
}

// Kind corresponds to the JSON schema field "kind".

// ModuleID corresponds to the JSON schema field "module_id".

// ResourceID corresponds to the JSON schema field "resource_id".

type ProjectRepositoryMembershipV1 struct {
	// Families corresponds to the JSON schema field "families".
	Families []PackageResourceRefV1 `json:"families,omitempty,omitzero"`

	// MaterializationOrder corresponds to the JSON schema field
	// "materialization_order".
	MaterializationOrder int `json:"materialization_order"`

	// ProjectID corresponds to the JSON schema field "project_id".
	ProjectID LogicalProjectIDV1 `json:"project_id"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`

	// ResponsibilityDerivationIds corresponds to the JSON schema field
	// "responsibility_derivation_ids".
	ResponsibilityDerivationIds []ResponsibilityDerivationID `json:"responsibility_derivation_ids,omitempty,omitzero"`

	// Role corresponds to the JSON schema field "role".
	Role PackageResourceRefV1 `json:"role"`

	// Stacks corresponds to the JSON schema field "stacks".
	Stacks []PackageResourceRefV1 `json:"stacks,omitempty,omitzero"`
}

type ReadingOrderEntry struct {
	// RelPath corresponds to the JSON schema field "rel_path".
	RelPath string `json:"rel_path"`

	// RootID corresponds to the JSON schema field "root_id".
	RootID string `json:"root_id"`
}

type ResponsibilityDerivationID string

type RunPolicyIDV1 string

type SessionModel struct {
	// ActivationPathIds corresponds to the JSON schema field "activation_path_ids".
	ActivationPathIds []ExecutionPathSelectionIDV1 `json:"activation_path_ids,omitempty,omitzero"`

	// ActivationRoots corresponds to the JSON schema field "activation_roots".
	ActivationRoots []LogicalRepositoryIDV1 `json:"activation_roots"`

	// AdditionalRoots corresponds to the JSON schema field "additional_roots".
	AdditionalRoots []LogicalRepositoryIDV1 `json:"additional_roots,omitempty,omitzero"`

	// AuthorityAccess corresponds to the JSON schema field "authority_access".
	AuthorityAccess []AuthorityAccess `json:"authority_access,omitempty,omitzero"`

	// LifecycleEvents corresponds to the JSON schema field "lifecycle_events".
	LifecycleEvents []lifecycle_event_v1.LifecycleEventV1 `json:"lifecycle_events,omitempty,omitzero"`

	// ReadingOrder corresponds to the JSON schema field "reading_order".
	ReadingOrder []ReadingOrderEntry `json:"reading_order,omitempty,omitzero"`

	// Runtime corresponds to the JSON schema field "runtime".
	Runtime *SessionRuntimeSelection `json:"runtime,omitempty,omitzero"`
}

type SessionRuntimeSelection string

const SessionRuntimeSelectionMachineLocal SessionRuntimeSelection = "machine-local"

type WorkspaceDesiredTopologyV1 struct {
	// ExecutionPaths corresponds to the JSON schema field "execution_paths".
	ExecutionPaths []ExecutionPathSelectionV1 `json:"execution_paths"`

	// ExecutionWorkspaceRepositories corresponds to the JSON schema field
	// "execution_workspace_repositories".
	ExecutionWorkspaceRepositories []ExecutionWorkspaceRepositoryMembershipV1 `json:"execution_workspace_repositories"`

	// ExecutionWorkspaces corresponds to the JSON schema field
	// "execution_workspaces".
	ExecutionWorkspaces []LogicalExecutionWorkspaceV1 `json:"execution_workspaces"`

	// ProjectExecutionWorkspaces corresponds to the JSON schema field
	// "project_execution_workspaces".
	ProjectExecutionWorkspaces []ProjectExecutionWorkspaceMembershipV1 `json:"project_execution_workspaces"`

	// ProjectRepositories corresponds to the JSON schema field
	// "project_repositories".
	ProjectRepositories []ProjectRepositoryMembershipV1 `json:"project_repositories"`

	// Projects corresponds to the JSON schema field "projects".
	Projects []LogicalProjectV1 `json:"projects"`

	// Repositories corresponds to the JSON schema field "repositories".
	Repositories []LogicalRepositoryV1 `json:"repositories"`
}

type WorkspaceID string

type WorkspaceMeta struct {
	// DesiredTopology corresponds to the JSON schema field "desired_topology".
	DesiredTopology WorkspaceDesiredTopologyV1 `json:"desired_topology"`

	// DistributionLock corresponds to the JSON schema field "distribution_lock".
	DistributionLock OwnedConfinedRelativePath `json:"distribution_lock"`

	// ExperienceCompositions corresponds to the JSON schema field
	// "experience_compositions".
	ExperienceCompositions []OwnedConfinedRelativePath `json:"experience_compositions"`

	// MachineBindingRoot corresponds to the JSON schema field "machine_binding_root".
	MachineBindingRoot OwnedConfinedRelativePath `json:"machine_binding_root"`

	// ModuleManifests corresponds to the JSON schema field "module_manifests".
	ModuleManifests []OwnedConfinedRelativePath `json:"module_manifests"`

	// ModuleSources corresponds to the JSON schema field "module_sources".
	ModuleSources []OwnedConfinedRelativePath `json:"module_sources,omitempty,omitzero"`

	// OrganizationalTranslations corresponds to the JSON schema field
	// "organizational_translations".
	OrganizationalTranslations []OwnedConfinedRelativePath `json:"organizational_translations"`

	// ResolutionLock corresponds to the JSON schema field "resolution_lock".
	ResolutionLock OwnedConfinedRelativePath `json:"resolution_lock"`

	// ResponsibilityDerivations corresponds to the JSON schema field
	// "responsibility_derivations".
	ResponsibilityDerivations []OwnedConfinedRelativePath `json:"responsibility_derivations"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// SourceComponents corresponds to the JSON schema field "source_components".
	SourceComponents []OwnedConfinedRelativePath `json:"source_components"`

	// UniverseLock corresponds to the JSON schema field "universe_lock".
	UniverseLock OwnedConfinedRelativePath `json:"universe_lock"`

	// WorkspaceID corresponds to the JSON schema field "workspace_id".
	WorkspaceID WorkspaceID `json:"workspace_id"`
}

type WorkspaceModuleID string

type WorkspaceResourceID string

type ProjectOwnerReferenceV1 struct {
	Principal    *ProjectOwnerReferencePrincipalV1    `json:"-"`
	Organization *ProjectOwnerReferenceOrganizationV1 `json:"-"`
}

func (value ProjectOwnerReferenceV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Principal != nil {
		if value.Principal.Kind != "principal" {
			return nil, fmt.Errorf("ProjectOwnerReferenceV1: selected variant has discriminator %q", value.Principal.Kind)
		}
		candidate, err := json.Marshal(value.Principal)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Organization != nil {
		if value.Organization.Kind != "organization" {
			return nil, fmt.Errorf("ProjectOwnerReferenceV1: selected variant has discriminator %q", value.Organization.Kind)
		}
		candidate, err := json.Marshal(value.Organization)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ProjectOwnerReferenceV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ProjectOwnerReferenceV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ProjectOwnerReferenceV1{}
	switch discriminator.Kind {
	case "principal":
		var variant ProjectOwnerReferencePrincipalV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Principal = &variant
	case "organization":
		var variant ProjectOwnerReferenceOrganizationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Organization = &variant
	default:
		return fmt.Errorf("ProjectOwnerReferenceV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
