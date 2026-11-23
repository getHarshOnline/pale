// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package workspace_derivation_receipt_v1 contains the generated workspace-derivation-receipt.v1 data model.
package workspace_derivation_receipt_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type OwnedConfinedRelativePath string

type WorkspaceComponentID string

type WorkspaceDeclarationID string

type WorkspaceDeclarationRefV1 struct {
	// DeclarationID corresponds to the JSON schema field "declaration_id".
	DeclarationID WorkspaceDeclarationID `json:"declaration_id"`

	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`
}

type WorkspaceDerivationImpactV1 struct {
	// Subject corresponds to the JSON schema field "subject".
	Subject WorkspaceDerivationSubjectV1 `json:"subject"`
}

type WorkspaceDerivationNodeV1 struct {
	// Inputs corresponds to the JSON schema field "inputs".
	Inputs []sha256_digest_v1.Sha256DigestV1 `json:"inputs"`

	// NodeDigest corresponds to the JSON schema field "node_digest".
	NodeDigest sha256_digest_v1.Sha256DigestV1 `json:"node_digest"`

	// Origin corresponds to the JSON schema field "origin".
	Origin WorkspaceDerivationOriginV1 `json:"origin"`

	// Output corresponds to the JSON schema field "output".
	Output WorkspaceDerivationOutputV1 `json:"output"`

	// Rule corresponds to the JSON schema field "rule".
	Rule WorkspaceDerivationRuleV1 `json:"rule"`

	// Subject corresponds to the JSON schema field "subject".
	Subject WorkspaceDerivationSubjectV1 `json:"subject"`
}

type WorkspaceDerivationOriginCompilationV1 struct {
	// Origin corresponds to the JSON schema field "origin".
	Origin string `json:"origin"`
}

type WorkspaceDerivationOriginExtensionV1 struct {
	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// Origin corresponds to the JSON schema field "origin".
	Origin string `json:"origin"`

	// OwnerModule corresponds to the JSON schema field "owner_module".
	OwnerModule WorkspaceModuleID `json:"owner_module"`

	// Target corresponds to the JSON schema field "target".
	Target WorkspaceDeclarationRefV1 `json:"target"`
}

type WorkspaceDerivationOriginImportV1 struct {
	// ExportID corresponds to the JSON schema field "export_id".
	ExportID WorkspaceExportID `json:"export_id"`

	// ImporterModule corresponds to the JSON schema field "importer_module".
	ImporterModule WorkspaceModuleID `json:"importer_module"`

	// LocalNamespace corresponds to the JSON schema field "local_namespace".
	LocalNamespace WorkspaceNamespaceID `json:"local_namespace"`

	// Origin corresponds to the JSON schema field "origin".
	Origin string `json:"origin"`

	// SourceNamespace corresponds to the JSON schema field "source_namespace".
	SourceNamespace WorkspaceNamespaceID `json:"source_namespace"`

	// UpstreamModule corresponds to the JSON schema field "upstream_module".
	UpstreamModule WorkspaceModuleID `json:"upstream_module"`

	// UpstreamRelease corresponds to the JSON schema field "upstream_release".
	UpstreamRelease WorkspaceReleaseID `json:"upstream_release"`
}

type WorkspaceDerivationOriginModuleReleaseV1 struct {
	// ModuleDigest corresponds to the JSON schema field "module_digest".
	ModuleDigest sha256_digest_v1.Sha256DigestV1 `json:"module_digest"`

	// ModuleID corresponds to the JSON schema field "module_id".
	ModuleID WorkspaceModuleID `json:"module_id"`

	// Origin corresponds to the JSON schema field "origin".
	Origin string `json:"origin"`

	// ReleaseID corresponds to the JSON schema field "release_id".
	ReleaseID WorkspaceReleaseID `json:"release_id"`
}

type WorkspaceDerivationOriginSourceV1 struct {
	// Origin corresponds to the JSON schema field "origin".
	Origin string `json:"origin"`

	// SourceID corresponds to the JSON schema field "source_id".
	SourceID WorkspaceDerivationSourceID `json:"source_id"`
}

type WorkspaceDerivationOutputComponentV1 struct {
	// ComponentDigest corresponds to the JSON schema field "component_digest".
	ComponentDigest sha256_digest_v1.Sha256DigestV1 `json:"component_digest"`

	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`
}

type WorkspaceDerivationOutputDeclarationV1 struct {
	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`

	// ValueDigest corresponds to the JSON schema field "value_digest".
	ValueDigest sha256_digest_v1.Sha256DigestV1 `json:"value_digest"`
}

type WorkspaceDerivationOutputEffectivePolicyV1 struct {
	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`

	// ValueDigest corresponds to the JSON schema field "value_digest".
	ValueDigest sha256_digest_v1.Sha256DigestV1 `json:"value_digest"`
}

type WorkspaceDerivationOutputExperienceCompositionV1 struct {
	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`

	// SourceDigest corresponds to the JSON schema field "source_digest".
	SourceDigest sha256_digest_v1.Sha256DigestV1 `json:"source_digest"`
}

type WorkspaceDerivationOutputExtensionV1 struct {
	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`

	// ValueDigest corresponds to the JSON schema field "value_digest".
	ValueDigest sha256_digest_v1.Sha256DigestV1 `json:"value_digest"`
}

type WorkspaceDerivationOutputImportSurfaceV1 struct {
	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`

	// SurfaceDigest corresponds to the JSON schema field "surface_digest".
	SurfaceDigest sha256_digest_v1.Sha256DigestV1 `json:"surface_digest"`
}

type WorkspaceDerivationOutputModuleV1 struct {
	// ModuleDigest corresponds to the JSON schema field "module_digest".
	ModuleDigest sha256_digest_v1.Sha256DigestV1 `json:"module_digest"`

	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`
}

type WorkspaceDerivationOutputOrganizationalTranslationClosureV1 struct {
	// ClosureDigest corresponds to the JSON schema field "closure_digest".
	ClosureDigest sha256_digest_v1.Sha256DigestV1 `json:"closure_digest"`

	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`
}

type WorkspaceDerivationOutputResolutionLockV1 struct {
	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`

	// ResolutionDigest corresponds to the JSON schema field "resolution_digest".
	ResolutionDigest sha256_digest_v1.Sha256DigestV1 `json:"resolution_digest"`
}

type WorkspaceDerivationOutputResourceV1 struct {
	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`

	// ValueDigest corresponds to the JSON schema field "value_digest".
	ValueDigest sha256_digest_v1.Sha256DigestV1 `json:"value_digest"`
}

type WorkspaceDerivationOutputResponsibilityClosureV1 struct {
	// ClosureDigest corresponds to the JSON schema field "closure_digest".
	ClosureDigest sha256_digest_v1.Sha256DigestV1 `json:"closure_digest"`

	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`
}

type WorkspaceDerivationOutputSemanticIrV1 struct {
	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`

	// WorkspaceSemanticIrDigest corresponds to the JSON schema field
	// "workspace_semantic_ir_digest".
	WorkspaceSemanticIrDigest sha256_digest_v1.Sha256DigestV1 `json:"workspace_semantic_ir_digest"`
}

type WorkspaceDerivationOutputSourceContentV1 struct {
	// ByteLen corresponds to the JSON schema field "byte_len".
	ByteLen int `json:"byte_len"`

	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`

	// SourceDigest corresponds to the JSON schema field "source_digest".
	SourceDigest sha256_digest_v1.Sha256DigestV1 `json:"source_digest"`

	// SourcePath corresponds to the JSON schema field "source_path".
	SourcePath OwnedConfinedRelativePath `json:"source_path"`
}

type WorkspaceDerivationOutputUniverseLockV1 struct {
	// Output corresponds to the JSON schema field "output".
	Output string `json:"output"`

	// UniverseDigest corresponds to the JSON schema field "universe_digest".
	UniverseDigest sha256_digest_v1.Sha256DigestV1 `json:"universe_digest"`
}

type WorkspaceDerivationReceiptV1 struct {
	// CurrentResolutionDigest corresponds to the JSON schema field
	// "current_resolution_digest".
	CurrentResolutionDigest sha256_digest_v1.Sha256DigestV1 `json:"current_resolution_digest"`

	// CurrentUniverseDigest corresponds to the JSON schema field
	// "current_universe_digest".
	CurrentUniverseDigest sha256_digest_v1.Sha256DigestV1 `json:"current_universe_digest"`

	// Impacts corresponds to the JSON schema field "impacts".
	Impacts []WorkspaceDerivationImpactV1 `json:"impacts"`

	// Nodes corresponds to the JSON schema field "nodes".
	Nodes []WorkspaceDerivationNodeV1 `json:"nodes"`

	// PriorResolutionDigest corresponds to the JSON schema field
	// "prior_resolution_digest".
	PriorResolutionDigest sha256_digest_v1.Sha256DigestV1 `json:"prior_resolution_digest"`

	// PriorUniverseDigest corresponds to the JSON schema field
	// "prior_universe_digest".
	PriorUniverseDigest sha256_digest_v1.Sha256DigestV1 `json:"prior_universe_digest"`

	// ReceiptDigest corresponds to the JSON schema field "receipt_digest".
	ReceiptDigest sha256_digest_v1.Sha256DigestV1 `json:"receipt_digest"`

	// Roots corresponds to the JSON schema field "roots".
	Roots []WorkspaceDerivationRootV1 `json:"roots"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// Sources corresponds to the JSON schema field "sources".
	Sources []WorkspaceDerivationSourceV1 `json:"sources"`

	// WorkspaceSemanticIrDigest corresponds to the JSON schema field
	// "workspace_semantic_ir_digest".
	WorkspaceSemanticIrDigest sha256_digest_v1.Sha256DigestV1 `json:"workspace_semantic_ir_digest"`
}

type WorkspaceDerivationRootV1 struct {
	// RootDigest corresponds to the JSON schema field "root_digest".
	RootDigest sha256_digest_v1.Sha256DigestV1 `json:"root_digest"`

	// Subject corresponds to the JSON schema field "subject".
	Subject WorkspaceDerivationSubjectV1 `json:"subject"`
}

type WorkspaceDerivationRuleV1 string

const WorkspaceDerivationRuleV1ComponentAdmission WorkspaceDerivationRuleV1 = "component_admission"
const WorkspaceDerivationRuleV1DeclarationLinking WorkspaceDerivationRuleV1 = "declaration_linking"
const WorkspaceDerivationRuleV1ExperienceComposition WorkspaceDerivationRuleV1 = "experience_composition"
const WorkspaceDerivationRuleV1ExtensionApplication WorkspaceDerivationRuleV1 = "extension_application"
const WorkspaceDerivationRuleV1ImportLinking WorkspaceDerivationRuleV1 = "import_linking"
const WorkspaceDerivationRuleV1ModuleSealing WorkspaceDerivationRuleV1 = "module_sealing"
const WorkspaceDerivationRuleV1OrganizationalTranslation WorkspaceDerivationRuleV1 = "organizational_translation"
const WorkspaceDerivationRuleV1PolicyComposition WorkspaceDerivationRuleV1 = "policy_composition"
const WorkspaceDerivationRuleV1ResolutionLocking WorkspaceDerivationRuleV1 = "resolution_locking"
const WorkspaceDerivationRuleV1ResourceSelection WorkspaceDerivationRuleV1 = "resource_selection"
const WorkspaceDerivationRuleV1ResponsibilityLowering WorkspaceDerivationRuleV1 = "responsibility_lowering"
const WorkspaceDerivationRuleV1SemanticIrSealing WorkspaceDerivationRuleV1 = "semantic_ir_sealing"
const WorkspaceDerivationRuleV1SourceAdmission WorkspaceDerivationRuleV1 = "source_admission"
const WorkspaceDerivationRuleV1UniverseLocking WorkspaceDerivationRuleV1 = "universe_locking"

type WorkspaceDerivationSourceID string

type WorkspaceDerivationSourceV1 struct {
	// ByteLen corresponds to the JSON schema field "byte_len".
	ByteLen int `json:"byte_len"`

	// SourceDigest corresponds to the JSON schema field "source_digest".
	SourceDigest sha256_digest_v1.Sha256DigestV1 `json:"source_digest"`

	// SourceID corresponds to the JSON schema field "source_id".
	SourceID WorkspaceDerivationSourceID `json:"source_id"`

	// SourcePath corresponds to the JSON schema field "source_path".
	SourcePath OwnedConfinedRelativePath `json:"source_path"`
}

type WorkspaceDerivationSubjectComponentV1 struct {
	// ComponentID corresponds to the JSON schema field "component_id".
	ComponentID WorkspaceComponentID `json:"component_id"`

	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`
}

type WorkspaceDerivationSubjectDeclarationV1 struct {
	// Declaration corresponds to the JSON schema field "declaration".
	Declaration WorkspaceDeclarationRefV1 `json:"declaration"`

	// OwnerModule corresponds to the JSON schema field "owner_module".
	OwnerModule WorkspaceModuleID `json:"owner_module"`

	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`
}

type WorkspaceDerivationSubjectExperienceCompositionV1 struct {
	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`
}

type WorkspaceDerivationSubjectExtensionV1 struct {
	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// OwnerModule corresponds to the JSON schema field "owner_module".
	OwnerModule WorkspaceModuleID `json:"owner_module"`

	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`

	// Target corresponds to the JSON schema field "target".
	Target WorkspaceDeclarationRefV1 `json:"target"`
}

type WorkspaceDerivationSubjectImportV1 struct {
	// ExportID corresponds to the JSON schema field "export_id".
	ExportID WorkspaceExportID `json:"export_id"`

	// ImporterModule corresponds to the JSON schema field "importer_module".
	ImporterModule WorkspaceModuleID `json:"importer_module"`

	// LocalNamespace corresponds to the JSON schema field "local_namespace".
	LocalNamespace WorkspaceNamespaceID `json:"local_namespace"`

	// SourceNamespace corresponds to the JSON schema field "source_namespace".
	SourceNamespace WorkspaceNamespaceID `json:"source_namespace"`

	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`

	// UpstreamModule corresponds to the JSON schema field "upstream_module".
	UpstreamModule WorkspaceModuleID `json:"upstream_module"`

	// UpstreamRelease corresponds to the JSON schema field "upstream_release".
	UpstreamRelease WorkspaceReleaseID `json:"upstream_release"`
}

type WorkspaceDerivationSubjectModuleV1 struct {
	// ModuleID corresponds to the JSON schema field "module_id".
	ModuleID WorkspaceModuleID `json:"module_id"`

	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`
}

type WorkspaceDerivationSubjectOrganizationalTranslationClosureV1 struct {
	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`
}

type WorkspaceDerivationSubjectPolicyV1 struct {
	// Declaration corresponds to the JSON schema field "declaration".
	Declaration WorkspaceDeclarationRefV1 `json:"declaration"`

	// OwnerModule corresponds to the JSON schema field "owner_module".
	OwnerModule WorkspaceModuleID `json:"owner_module"`

	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`
}

type WorkspaceDerivationSubjectResolutionLockV1 struct {
	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`
}

type WorkspaceDerivationSubjectResourceV1 struct {
	// OwnerModule corresponds to the JSON schema field "owner_module".
	OwnerModule WorkspaceModuleID `json:"owner_module"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID WorkspaceResourceID `json:"resource_id"`

	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`
}

type WorkspaceDerivationSubjectResponsibilityClosureV1 struct {
	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`
}

type WorkspaceDerivationSubjectSourceV1 struct {
	// SourceID corresponds to the JSON schema field "source_id".
	SourceID WorkspaceDerivationSourceID `json:"source_id"`

	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`
}

type WorkspaceDerivationSubjectUniverseLockV1 struct {
	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`
}

type WorkspaceDerivationSubjectWorkspaceV1 struct {
	// Subject corresponds to the JSON schema field "subject".
	Subject string `json:"subject"`

	// WorkspaceID corresponds to the JSON schema field "workspace_id".
	WorkspaceID WorkspaceID `json:"workspace_id"`
}

type WorkspaceExportID string

type WorkspaceID string

type WorkspaceModuleID string

type WorkspaceNamespaceID string

type WorkspaceReleaseID string

type WorkspaceResourceID string

type WorkspaceDerivationOriginV1 struct {
	Source        *WorkspaceDerivationOriginSourceV1        `json:"-"`
	ModuleRelease *WorkspaceDerivationOriginModuleReleaseV1 `json:"-"`
	Import        *WorkspaceDerivationOriginImportV1        `json:"-"`
	Extension     *WorkspaceDerivationOriginExtensionV1     `json:"-"`
	Compilation   *WorkspaceDerivationOriginCompilationV1   `json:"-"`
}

func (value WorkspaceDerivationOriginV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Source != nil {
		if value.Source.Origin != "source" {
			return nil, fmt.Errorf("WorkspaceDerivationOriginV1: selected variant has discriminator %q", value.Source.Origin)
		}
		candidate, err := json.Marshal(value.Source)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ModuleRelease != nil {
		if value.ModuleRelease.Origin != "module_release" {
			return nil, fmt.Errorf("WorkspaceDerivationOriginV1: selected variant has discriminator %q", value.ModuleRelease.Origin)
		}
		candidate, err := json.Marshal(value.ModuleRelease)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Import != nil {
		if value.Import.Origin != "import" {
			return nil, fmt.Errorf("WorkspaceDerivationOriginV1: selected variant has discriminator %q", value.Import.Origin)
		}
		candidate, err := json.Marshal(value.Import)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Extension != nil {
		if value.Extension.Origin != "extension" {
			return nil, fmt.Errorf("WorkspaceDerivationOriginV1: selected variant has discriminator %q", value.Extension.Origin)
		}
		candidate, err := json.Marshal(value.Extension)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Compilation != nil {
		if value.Compilation.Origin != "compilation" {
			return nil, fmt.Errorf("WorkspaceDerivationOriginV1: selected variant has discriminator %q", value.Compilation.Origin)
		}
		candidate, err := json.Marshal(value.Compilation)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("WorkspaceDerivationOriginV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *WorkspaceDerivationOriginV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Origin string `json:"origin"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = WorkspaceDerivationOriginV1{}
	switch discriminator.Origin {
	case "source":
		var variant WorkspaceDerivationOriginSourceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Source = &variant
	case "module_release":
		var variant WorkspaceDerivationOriginModuleReleaseV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ModuleRelease = &variant
	case "import":
		var variant WorkspaceDerivationOriginImportV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Import = &variant
	case "extension":
		var variant WorkspaceDerivationOriginExtensionV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Extension = &variant
	case "compilation":
		var variant WorkspaceDerivationOriginCompilationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Compilation = &variant
	default:
		return fmt.Errorf("WorkspaceDerivationOriginV1: unknown discriminator %q", discriminator.Origin)
	}
	return nil
}

type WorkspaceDerivationOutputV1 struct {
	SourceContent                    *WorkspaceDerivationOutputSourceContentV1                    `json:"-"`
	Component                        *WorkspaceDerivationOutputComponentV1                        `json:"-"`
	Module                           *WorkspaceDerivationOutputModuleV1                           `json:"-"`
	Declaration                      *WorkspaceDerivationOutputDeclarationV1                      `json:"-"`
	ImportSurface                    *WorkspaceDerivationOutputImportSurfaceV1                    `json:"-"`
	Extension                        *WorkspaceDerivationOutputExtensionV1                        `json:"-"`
	EffectivePolicy                  *WorkspaceDerivationOutputEffectivePolicyV1                  `json:"-"`
	ResponsibilityClosure            *WorkspaceDerivationOutputResponsibilityClosureV1            `json:"-"`
	OrganizationalTranslationClosure *WorkspaceDerivationOutputOrganizationalTranslationClosureV1 `json:"-"`
	ExperienceComposition            *WorkspaceDerivationOutputExperienceCompositionV1            `json:"-"`
	Resource                         *WorkspaceDerivationOutputResourceV1                         `json:"-"`
	ResolutionLock                   *WorkspaceDerivationOutputResolutionLockV1                   `json:"-"`
	UniverseLock                     *WorkspaceDerivationOutputUniverseLockV1                     `json:"-"`
	SemanticIr                       *WorkspaceDerivationOutputSemanticIrV1                       `json:"-"`
}

func (value WorkspaceDerivationOutputV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.SourceContent != nil {
		if value.SourceContent.Output != "source_content" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.SourceContent.Output)
		}
		candidate, err := json.Marshal(value.SourceContent)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Component != nil {
		if value.Component.Output != "component" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.Component.Output)
		}
		candidate, err := json.Marshal(value.Component)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Module != nil {
		if value.Module.Output != "module" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.Module.Output)
		}
		candidate, err := json.Marshal(value.Module)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Declaration != nil {
		if value.Declaration.Output != "declaration" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.Declaration.Output)
		}
		candidate, err := json.Marshal(value.Declaration)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ImportSurface != nil {
		if value.ImportSurface.Output != "import_surface" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.ImportSurface.Output)
		}
		candidate, err := json.Marshal(value.ImportSurface)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Extension != nil {
		if value.Extension.Output != "extension" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.Extension.Output)
		}
		candidate, err := json.Marshal(value.Extension)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.EffectivePolicy != nil {
		if value.EffectivePolicy.Output != "effective_policy" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.EffectivePolicy.Output)
		}
		candidate, err := json.Marshal(value.EffectivePolicy)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ResponsibilityClosure != nil {
		if value.ResponsibilityClosure.Output != "responsibility_closure" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.ResponsibilityClosure.Output)
		}
		candidate, err := json.Marshal(value.ResponsibilityClosure)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.OrganizationalTranslationClosure != nil {
		if value.OrganizationalTranslationClosure.Output != "organizational_translation_closure" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.OrganizationalTranslationClosure.Output)
		}
		candidate, err := json.Marshal(value.OrganizationalTranslationClosure)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ExperienceComposition != nil {
		if value.ExperienceComposition.Output != "experience_composition" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.ExperienceComposition.Output)
		}
		candidate, err := json.Marshal(value.ExperienceComposition)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Resource != nil {
		if value.Resource.Output != "resource" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.Resource.Output)
		}
		candidate, err := json.Marshal(value.Resource)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ResolutionLock != nil {
		if value.ResolutionLock.Output != "resolution_lock" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.ResolutionLock.Output)
		}
		candidate, err := json.Marshal(value.ResolutionLock)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.UniverseLock != nil {
		if value.UniverseLock.Output != "universe_lock" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.UniverseLock.Output)
		}
		candidate, err := json.Marshal(value.UniverseLock)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.SemanticIr != nil {
		if value.SemanticIr.Output != "semantic_ir" {
			return nil, fmt.Errorf("WorkspaceDerivationOutputV1: selected variant has discriminator %q", value.SemanticIr.Output)
		}
		candidate, err := json.Marshal(value.SemanticIr)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("WorkspaceDerivationOutputV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *WorkspaceDerivationOutputV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Output string `json:"output"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = WorkspaceDerivationOutputV1{}
	switch discriminator.Output {
	case "source_content":
		var variant WorkspaceDerivationOutputSourceContentV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.SourceContent = &variant
	case "component":
		var variant WorkspaceDerivationOutputComponentV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Component = &variant
	case "module":
		var variant WorkspaceDerivationOutputModuleV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Module = &variant
	case "declaration":
		var variant WorkspaceDerivationOutputDeclarationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Declaration = &variant
	case "import_surface":
		var variant WorkspaceDerivationOutputImportSurfaceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ImportSurface = &variant
	case "extension":
		var variant WorkspaceDerivationOutputExtensionV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Extension = &variant
	case "effective_policy":
		var variant WorkspaceDerivationOutputEffectivePolicyV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.EffectivePolicy = &variant
	case "responsibility_closure":
		var variant WorkspaceDerivationOutputResponsibilityClosureV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ResponsibilityClosure = &variant
	case "organizational_translation_closure":
		var variant WorkspaceDerivationOutputOrganizationalTranslationClosureV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.OrganizationalTranslationClosure = &variant
	case "experience_composition":
		var variant WorkspaceDerivationOutputExperienceCompositionV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ExperienceComposition = &variant
	case "resource":
		var variant WorkspaceDerivationOutputResourceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Resource = &variant
	case "resolution_lock":
		var variant WorkspaceDerivationOutputResolutionLockV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ResolutionLock = &variant
	case "universe_lock":
		var variant WorkspaceDerivationOutputUniverseLockV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.UniverseLock = &variant
	case "semantic_ir":
		var variant WorkspaceDerivationOutputSemanticIrV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.SemanticIr = &variant
	default:
		return fmt.Errorf("WorkspaceDerivationOutputV1: unknown discriminator %q", discriminator.Output)
	}
	return nil
}

type WorkspaceDerivationSubjectV1 struct {
	Source                           *WorkspaceDerivationSubjectSourceV1                           `json:"-"`
	Component                        *WorkspaceDerivationSubjectComponentV1                        `json:"-"`
	Module                           *WorkspaceDerivationSubjectModuleV1                           `json:"-"`
	Declaration                      *WorkspaceDerivationSubjectDeclarationV1                      `json:"-"`
	Import                           *WorkspaceDerivationSubjectImportV1                           `json:"-"`
	Extension                        *WorkspaceDerivationSubjectExtensionV1                        `json:"-"`
	Policy                           *WorkspaceDerivationSubjectPolicyV1                           `json:"-"`
	ResponsibilityClosure            *WorkspaceDerivationSubjectResponsibilityClosureV1            `json:"-"`
	OrganizationalTranslationClosure *WorkspaceDerivationSubjectOrganizationalTranslationClosureV1 `json:"-"`
	ExperienceComposition            *WorkspaceDerivationSubjectExperienceCompositionV1            `json:"-"`
	Resource                         *WorkspaceDerivationSubjectResourceV1                         `json:"-"`
	ResolutionLock                   *WorkspaceDerivationSubjectResolutionLockV1                   `json:"-"`
	UniverseLock                     *WorkspaceDerivationSubjectUniverseLockV1                     `json:"-"`
	Workspace                        *WorkspaceDerivationSubjectWorkspaceV1                        `json:"-"`
}

func (value WorkspaceDerivationSubjectV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Source != nil {
		if value.Source.Subject != "source" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.Source.Subject)
		}
		candidate, err := json.Marshal(value.Source)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Component != nil {
		if value.Component.Subject != "component" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.Component.Subject)
		}
		candidate, err := json.Marshal(value.Component)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Module != nil {
		if value.Module.Subject != "module" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.Module.Subject)
		}
		candidate, err := json.Marshal(value.Module)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Declaration != nil {
		if value.Declaration.Subject != "declaration" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.Declaration.Subject)
		}
		candidate, err := json.Marshal(value.Declaration)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Import != nil {
		if value.Import.Subject != "import" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.Import.Subject)
		}
		candidate, err := json.Marshal(value.Import)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Extension != nil {
		if value.Extension.Subject != "extension" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.Extension.Subject)
		}
		candidate, err := json.Marshal(value.Extension)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Policy != nil {
		if value.Policy.Subject != "policy" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.Policy.Subject)
		}
		candidate, err := json.Marshal(value.Policy)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ResponsibilityClosure != nil {
		if value.ResponsibilityClosure.Subject != "responsibility_closure" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.ResponsibilityClosure.Subject)
		}
		candidate, err := json.Marshal(value.ResponsibilityClosure)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.OrganizationalTranslationClosure != nil {
		if value.OrganizationalTranslationClosure.Subject != "organizational_translation_closure" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.OrganizationalTranslationClosure.Subject)
		}
		candidate, err := json.Marshal(value.OrganizationalTranslationClosure)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ExperienceComposition != nil {
		if value.ExperienceComposition.Subject != "experience_composition" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.ExperienceComposition.Subject)
		}
		candidate, err := json.Marshal(value.ExperienceComposition)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Resource != nil {
		if value.Resource.Subject != "resource" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.Resource.Subject)
		}
		candidate, err := json.Marshal(value.Resource)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ResolutionLock != nil {
		if value.ResolutionLock.Subject != "resolution_lock" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.ResolutionLock.Subject)
		}
		candidate, err := json.Marshal(value.ResolutionLock)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.UniverseLock != nil {
		if value.UniverseLock.Subject != "universe_lock" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.UniverseLock.Subject)
		}
		candidate, err := json.Marshal(value.UniverseLock)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Workspace != nil {
		if value.Workspace.Subject != "workspace" {
			return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: selected variant has discriminator %q", value.Workspace.Subject)
		}
		candidate, err := json.Marshal(value.Workspace)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("WorkspaceDerivationSubjectV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *WorkspaceDerivationSubjectV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Subject string `json:"subject"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = WorkspaceDerivationSubjectV1{}
	switch discriminator.Subject {
	case "source":
		var variant WorkspaceDerivationSubjectSourceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Source = &variant
	case "component":
		var variant WorkspaceDerivationSubjectComponentV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Component = &variant
	case "module":
		var variant WorkspaceDerivationSubjectModuleV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Module = &variant
	case "declaration":
		var variant WorkspaceDerivationSubjectDeclarationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Declaration = &variant
	case "import":
		var variant WorkspaceDerivationSubjectImportV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Import = &variant
	case "extension":
		var variant WorkspaceDerivationSubjectExtensionV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Extension = &variant
	case "policy":
		var variant WorkspaceDerivationSubjectPolicyV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Policy = &variant
	case "responsibility_closure":
		var variant WorkspaceDerivationSubjectResponsibilityClosureV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ResponsibilityClosure = &variant
	case "organizational_translation_closure":
		var variant WorkspaceDerivationSubjectOrganizationalTranslationClosureV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.OrganizationalTranslationClosure = &variant
	case "experience_composition":
		var variant WorkspaceDerivationSubjectExperienceCompositionV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ExperienceComposition = &variant
	case "resource":
		var variant WorkspaceDerivationSubjectResourceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Resource = &variant
	case "resolution_lock":
		var variant WorkspaceDerivationSubjectResolutionLockV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ResolutionLock = &variant
	case "universe_lock":
		var variant WorkspaceDerivationSubjectUniverseLockV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.UniverseLock = &variant
	case "workspace":
		var variant WorkspaceDerivationSubjectWorkspaceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Workspace = &variant
	default:
		return fmt.Errorf("WorkspaceDerivationSubjectV1: unknown discriminator %q", discriminator.Subject)
	}
	return nil
}
