// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package compiled_generation_v1 contains the generated compiled-generation.v1 data model.
package compiled_generation_v1

import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type CompiledGenerationV1 struct {
	// CompilationInputDigest corresponds to the JSON schema field
	// "compilation_input_digest".
	CompilationInputDigest sha256_digest_v1.Sha256DigestV1 `json:"compilation_input_digest"`

	// CompiledGenerationDigest corresponds to the JSON schema field
	// "compiled_generation_digest".
	CompiledGenerationDigest sha256_digest_v1.Sha256DigestV1 `json:"compiled_generation_digest"`

	// DistributionLockDigest corresponds to the JSON schema field
	// "distribution_lock_digest".
	DistributionLockDigest sha256_digest_v1.Sha256DigestV1 `json:"distribution_lock_digest"`

	// ExecutableIdentityDigest corresponds to the JSON schema field
	// "executable_identity_digest".
	ExecutableIdentityDigest sha256_digest_v1.Sha256DigestV1 `json:"executable_identity_digest"`

	// ExecutionPaths corresponds to the JSON schema field "execution_paths".
	ExecutionPaths []ExecutionPathSelectionV1 `json:"execution_paths"`

	// ExperienceCompositionPlanDigest corresponds to the JSON schema field
	// "experience_composition_plan_digest".
	ExperienceCompositionPlanDigest sha256_digest_v1.Sha256DigestV1 `json:"experience_composition_plan_digest"`

	// ExperienceCompositionSourceDigest corresponds to the JSON schema field
	// "experience_composition_source_digest".
	ExperienceCompositionSourceDigest sha256_digest_v1.Sha256DigestV1 `json:"experience_composition_source_digest"`

	// InstanceDigest corresponds to the JSON schema field "instance_digest".
	InstanceDigest sha256_digest_v1.Sha256DigestV1 `json:"instance_digest"`

	// MachineOverlayDigest corresponds to the JSON schema field
	// "machine_overlay_digest".
	MachineOverlayDigest sha256_digest_v1.Sha256DigestV1 `json:"machine_overlay_digest"`

	// MaterializationClosureDigest corresponds to the JSON schema field
	// "materialization_closure_digest".
	MaterializationClosureDigest sha256_digest_v1.Sha256DigestV1 `json:"materialization_closure_digest"`

	// OneShotCompilationDigest corresponds to the JSON schema field
	// "one_shot_compilation_digest".
	OneShotCompilationDigest sha256_digest_v1.Sha256DigestV1 `json:"one_shot_compilation_digest"`

	// OrganizationalTranslationClosureDigest corresponds to the JSON schema field
	// "organizational_translation_closure_digest".
	OrganizationalTranslationClosureDigest sha256_digest_v1.Sha256DigestV1 `json:"organizational_translation_closure_digest"`

	// PackageSemanticIrDigest corresponds to the JSON schema field
	// "package_semantic_ir_digest".
	PackageSemanticIrDigest sha256_digest_v1.Sha256DigestV1 `json:"package_semantic_ir_digest"`

	// PrincipalAdmittedUniverseDigest corresponds to the JSON schema field
	// "principal_admitted_universe_digest".
	PrincipalAdmittedUniverseDigest sha256_digest_v1.Sha256DigestV1 `json:"principal_admitted_universe_digest"`

	// ProfileDigest corresponds to the JSON schema field "profile_digest".
	ProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"profile_digest"`

	// ProfileNormativePackReleaseClosure corresponds to the JSON schema field
	// "profile_normative_pack_release_closure".
	ProfileNormativePackReleaseClosure []NormativePackReleaseV1 `json:"profile_normative_pack_release_closure"`

	// ProfilePackageReleaseClosure corresponds to the JSON schema field
	// "profile_package_release_closure".
	ProfilePackageReleaseClosure []PackageReleaseV1 `json:"profile_package_release_closure"`

	// ProviderBindingClosureDigest corresponds to the JSON schema field
	// "provider_binding_closure_digest".
	ProviderBindingClosureDigest sha256_digest_v1.Sha256DigestV1 `json:"provider_binding_closure_digest"`

	// RepositorySourceFrontiers corresponds to the JSON schema field
	// "repository_source_frontiers".
	RepositorySourceFrontiers []RepositorySourceFrontierV1 `json:"repository_source_frontiers"`

	// ResponsibilityDerivationClosureDigest corresponds to the JSON schema field
	// "responsibility_derivation_closure_digest".
	ResponsibilityDerivationClosureDigest sha256_digest_v1.Sha256DigestV1 `json:"responsibility_derivation_closure_digest"`

	// Schema corresponds to the JSON schema field "schema".
	Schema SchemaID `json:"schema"`

	// SchemaManifestDigest corresponds to the JSON schema field
	// "schema_manifest_digest".
	SchemaManifestDigest sha256_digest_v1.Sha256DigestV1 `json:"schema_manifest_digest"`

	// SchemaSetDigest corresponds to the JSON schema field "schema_set_digest".
	SchemaSetDigest sha256_digest_v1.Sha256DigestV1 `json:"schema_set_digest"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// WorkspaceAdmissionEvidenceDigest corresponds to the JSON schema field
	// "workspace_admission_evidence_digest".
	WorkspaceAdmissionEvidenceDigest sha256_digest_v1.Sha256DigestV1 `json:"workspace_admission_evidence_digest"`

	// WorkspaceModuleReleaseClosure corresponds to the JSON schema field
	// "workspace_module_release_closure".
	WorkspaceModuleReleaseClosure []WorkspaceAdoptedReleaseV1 `json:"workspace_module_release_closure"`

	// WorkspaceNormativePackReleaseClosure corresponds to the JSON schema field
	// "workspace_normative_pack_release_closure".
	WorkspaceNormativePackReleaseClosure []NormativePackReleaseV1 `json:"workspace_normative_pack_release_closure"`

	// WorkspacePackageReleaseClosure corresponds to the JSON schema field
	// "workspace_package_release_closure".
	WorkspacePackageReleaseClosure []PackageReleaseV1 `json:"workspace_package_release_closure"`

	// WorkspaceResolutionLockDigest corresponds to the JSON schema field
	// "workspace_resolution_lock_digest".
	WorkspaceResolutionLockDigest sha256_digest_v1.Sha256DigestV1 `json:"workspace_resolution_lock_digest"`

	// WorkspaceSemanticIrDigest corresponds to the JSON schema field
	// "workspace_semantic_ir_digest".
	WorkspaceSemanticIrDigest sha256_digest_v1.Sha256DigestV1 `json:"workspace_semantic_ir_digest"`

	// WorkspaceUniverseLockDigest corresponds to the JSON schema field
	// "workspace_universe_lock_digest".
	WorkspaceUniverseLockDigest sha256_digest_v1.Sha256DigestV1 `json:"workspace_universe_lock_digest"`
}

type ExecutionPathSelectionIDV1 string

type ExecutionPathSelectionV1 struct {
	// ExperiencePathID corresponds to the JSON schema field "experience_path_id".
	ExperiencePathID string `json:"experience_path_id"`

	// ID corresponds to the JSON schema field "id".
	ID ExecutionPathSelectionIDV1 `json:"id"`
}

type LogicalRepositoryIDV1 string

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

type RepositoryRevisionV1 string

type RepositorySourceFrontierV1 struct {
	// BaseRevision corresponds to the JSON schema field "base_revision".
	BaseRevision RepositoryRevisionV1 `json:"base_revision"`

	// DependencyFrontierSha256 corresponds to the JSON schema field
	// "dependency_frontier_sha256".
	DependencyFrontierSha256 sha256_digest_v1.Sha256DigestV1 `json:"dependency_frontier_sha256"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`

	// TrackedDeltaSha256 corresponds to the JSON schema field "tracked_delta_sha256".
	TrackedDeltaSha256 sha256_digest_v1.Sha256DigestV1 `json:"tracked_delta_sha256"`

	// UntrackedInventorySha256 corresponds to the JSON schema field
	// "untracked_inventory_sha256".
	UntrackedInventorySha256 sha256_digest_v1.Sha256DigestV1 `json:"untracked_inventory_sha256"`
}

type SchemaID string

type WorkspaceAdoptedReleaseV1 struct {
	// ModuleDigest corresponds to the JSON schema field "module_digest".
	ModuleDigest sha256_digest_v1.Sha256DigestV1 `json:"module_digest"`

	// ModuleID corresponds to the JSON schema field "module_id".
	ModuleID WorkspaceModuleID `json:"module_id"`

	// ReleaseID corresponds to the JSON schema field "release_id".
	ReleaseID WorkspaceReleaseID `json:"release_id"`
}

type WorkspaceModuleID string

type WorkspaceReleaseID string
