// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package workspace_module_source_v1 contains the generated workspace-module-source.v1 data model.
package workspace_module_source_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type DeclarationDependencyKindV1 string

const DeclarationDependencyKindV1Callback DeclarationDependencyKindV1 = "callback"
const DeclarationDependencyKindV1Effect DeclarationDependencyKindV1 = "effect"
const DeclarationDependencyKindV1Negation DeclarationDependencyKindV1 = "negation"
const DeclarationDependencyKindV1Positive DeclarationDependencyKindV1 = "positive"
const DeclarationDependencyKindV1UnboundedGeneration DeclarationDependencyKindV1 = "unbounded_generation"

type DisclosureCeilingV1 string

const DisclosureCeilingV1Private DisclosureCeilingV1 = "private"
const DisclosureCeilingV1Protected DisclosureCeilingV1 = "protected"
const DisclosureCeilingV1Public DisclosureCeilingV1 = "public"

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

type WorkspaceAdoptionPolicyV1 string

const WorkspaceAdoptionPolicyV1Manual WorkspaceAdoptionPolicyV1 = "manual"

type WorkspaceAuthorityCeilingV1 struct {
	// Permitted corresponds to the JSON schema field "permitted".
	Permitted []WorkspaceAuthorityScopeID `json:"permitted"`
}

type WorkspaceAuthorityScopeID string

type WorkspaceComponentID string

type WorkspaceControllerCeilingV1 struct {
	// Permitted corresponds to the JSON schema field "permitted".
	Permitted []WorkspaceControllerScopeID `json:"permitted"`
}

type WorkspaceControllerScopeID string

type WorkspaceDeclarationAttributeV1 struct {
	// Dependencies corresponds to the JSON schema field "dependencies".
	Dependencies []WorkspaceDeclarationDependencyV1 `json:"dependencies"`

	// ID corresponds to the JSON schema field "id".
	ID WorkspaceDeclarationID `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// ValueKind corresponds to the JSON schema field "value_kind".
	ValueKind WorkspaceValueKindV1 `json:"value_kind"`
}

type WorkspaceDeclarationDependencyV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind DeclarationDependencyKindV1 `json:"kind"`

	// Target corresponds to the JSON schema field "target".
	Target WorkspaceDeclarationRefV1 `json:"target"`
}

type WorkspaceDeclarationID string

type WorkspaceDeclarationPolicyV1 struct {
	// Ceiling corresponds to the JSON schema field "ceiling".
	Ceiling WorkspacePolicyCeilingV1 `json:"ceiling"`

	// Dependencies corresponds to the JSON schema field "dependencies".
	Dependencies []WorkspaceDeclarationDependencyV1 `json:"dependencies"`

	// ExclusiveAuthority corresponds to the JSON schema field "exclusive_authority".
	ExclusiveAuthority []WorkspaceExclusiveAuthorityClaimV1 `json:"exclusive_authority"`

	// ID corresponds to the JSON schema field "id".
	ID WorkspaceDeclarationID `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// Obligations corresponds to the JSON schema field "obligations".
	Obligations []WorkspaceObligationID `json:"obligations"`
}

type WorkspaceDeclarationRefV1 struct {
	// DeclarationID corresponds to the JSON schema field "declaration_id".
	DeclarationID WorkspaceDeclarationID `json:"declaration_id"`

	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`
}

type WorkspaceDeclarationRelationshipV1 struct {
	// Dependencies corresponds to the JSON schema field "dependencies".
	Dependencies []WorkspaceDeclarationDependencyV1 `json:"dependencies"`

	// From corresponds to the JSON schema field "from".
	From WorkspaceDeclarationRefV1 `json:"from"`

	// ID corresponds to the JSON schema field "id".
	ID WorkspaceDeclarationID `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// To corresponds to the JSON schema field "to".
	To WorkspaceDeclarationRefV1 `json:"to"`
}

type WorkspaceDeclarationResourceV1 struct {
	// Dependencies corresponds to the JSON schema field "dependencies".
	Dependencies []WorkspaceDeclarationDependencyV1 `json:"dependencies"`

	// ID corresponds to the JSON schema field "id".
	ID WorkspaceDeclarationID `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// ResourceKind corresponds to the JSON schema field "resource_kind".
	ResourceKind WorkspaceResourceKindV1 `json:"resource_kind"`
}

type WorkspaceDeclarationTraitV1 struct {
	// Dependencies corresponds to the JSON schema field "dependencies".
	Dependencies []WorkspaceDeclarationDependencyV1 `json:"dependencies"`

	// ID corresponds to the JSON schema field "id".
	ID WorkspaceDeclarationID `json:"id"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`
}

type WorkspaceDeliveryV1 string

const WorkspaceDeliveryV1Interface WorkspaceDeliveryV1 = "interface"
const WorkspaceDeliveryV1Source WorkspaceDeliveryV1 = "source"

type WorkspaceEffectCeilingV1 string

const WorkspaceEffectCeilingV1Mutate WorkspaceEffectCeilingV1 = "mutate"
const WorkspaceEffectCeilingV1None WorkspaceEffectCeilingV1 = "none"
const WorkspaceEffectCeilingV1Read WorkspaceEffectCeilingV1 = "read"

type WorkspaceExclusiveAuthorityClaimV1 struct {
	// AuthorityResolver corresponds to the JSON schema field "authority_resolver".
	AuthorityResolver WorkspaceResourceID `json:"authority_resolver"`

	// Scope corresponds to the JSON schema field "scope".
	Scope WorkspaceAuthorityScopeID `json:"scope"`
}

type WorkspaceExportID string

type WorkspaceExportProjectionV1 struct {
	// Declaration corresponds to the JSON schema field "declaration".
	Declaration WorkspaceDeclarationV1 `json:"declaration"`

	// LocalRef corresponds to the JSON schema field "local_ref".
	LocalRef WorkspaceDeclarationRefV1 `json:"local_ref"`

	// OwnerModule corresponds to the JSON schema field "owner_module".
	OwnerModule WorkspaceModuleID `json:"owner_module"`
}

type WorkspaceExportV1 struct {
	// Ceiling corresponds to the JSON schema field "ceiling".
	Ceiling WorkspacePolicyCeilingV1 `json:"ceiling"`

	// Declarations corresponds to the JSON schema field "declarations".
	Declarations []WorkspaceExportProjectionV1 `json:"declarations"`

	// Delivery corresponds to the JSON schema field "delivery".
	Delivery WorkspaceDeliveryV1 `json:"delivery"`

	// ExclusiveAuthority corresponds to the JSON schema field "exclusive_authority".
	ExclusiveAuthority []WorkspaceExclusiveAuthorityClaimV1 `json:"exclusive_authority"`

	// ExportID corresponds to the JSON schema field "export_id".
	ExportID WorkspaceExportID `json:"export_id"`

	// Obligations corresponds to the JSON schema field "obligations".
	Obligations []WorkspaceObligationID `json:"obligations"`

	// Propagation corresponds to the JSON schema field "propagation".
	Propagation WorkspacePropagationV1 `json:"propagation"`

	// Visibility corresponds to the JSON schema field "visibility".
	Visibility WorkspaceVisibilityV1 `json:"visibility"`
}

type WorkspaceExtensionV1 struct {
	// AddedObligations corresponds to the JSON schema field "added_obligations".
	AddedObligations []WorkspaceObligationID `json:"added_obligations"`

	// AddedTraits corresponds to the JSON schema field "added_traits".
	AddedTraits []WorkspaceDeclarationRefV1 `json:"added_traits"`

	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// NarrowedCeiling corresponds to the JSON schema field "narrowed_ceiling".
	NarrowedCeiling *WorkspacePolicyCeilingV1 `json:"narrowed_ceiling,omitempty,omitzero"`

	// Target corresponds to the JSON schema field "target".
	Target WorkspaceDeclarationRefV1 `json:"target"`
}

type WorkspaceImportPathV1 struct {
	// ExportID corresponds to the JSON schema field "export_id".
	ExportID WorkspaceExportID `json:"export_id"`

	// LocalNamespace corresponds to the JSON schema field "local_namespace".
	LocalNamespace WorkspaceNamespaceID `json:"local_namespace"`

	// SourceNamespace corresponds to the JSON schema field "source_namespace".
	SourceNamespace WorkspaceNamespaceID `json:"source_namespace"`
}

type WorkspaceImportV1 struct {
	// AdoptionPolicy corresponds to the JSON schema field "adoption_policy".
	AdoptionPolicy WorkspaceAdoptionPolicyV1 `json:"adoption_policy"`

	// EdgePolicy corresponds to the JSON schema field "edge_policy".
	EdgePolicy WorkspacePolicyCeilingV1 `json:"edge_policy"`

	// ModuleDigest corresponds to the JSON schema field "module_digest".
	ModuleDigest sha256_digest_v1.Sha256DigestV1 `json:"module_digest"`

	// ModuleID corresponds to the JSON schema field "module_id".
	ModuleID WorkspaceModuleID `json:"module_id"`

	// Paths corresponds to the JSON schema field "paths".
	Paths []WorkspaceImportPathV1 `json:"paths"`

	// Propagation corresponds to the JSON schema field "propagation".
	Propagation WorkspacePropagationV1 `json:"propagation"`

	// ProtectedDeliveryAuthority corresponds to the JSON schema field
	// "protected_delivery_authority".
	ProtectedDeliveryAuthority *WorkspaceResourceID `json:"protected_delivery_authority,omitempty,omitzero"`

	// Publisher corresponds to the JSON schema field "publisher".
	Publisher WorkspacePublisherRefV1 `json:"publisher"`

	// ReleaseID corresponds to the JSON schema field "release_id".
	ReleaseID WorkspaceReleaseID `json:"release_id"`

	// VisibilityCeiling corresponds to the JSON schema field "visibility_ceiling".
	VisibilityCeiling WorkspaceVisibilityV1 `json:"visibility_ceiling"`
}

type WorkspaceModuleID string

type WorkspaceModuleSourceV1 struct {
	// Exports corresponds to the JSON schema field "exports".
	Exports []WorkspaceExportV1 `json:"exports"`

	// Extensions corresponds to the JSON schema field "extensions".
	Extensions []WorkspaceExtensionV1 `json:"extensions"`

	// Imports corresponds to the JSON schema field "imports".
	Imports []WorkspaceImportV1 `json:"imports"`

	// ModuleID corresponds to the JSON schema field "module_id".
	ModuleID WorkspaceModuleID `json:"module_id"`

	// Resources corresponds to the JSON schema field "resources".
	Resources []WorkspaceResourceSelectionV1 `json:"resources"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// SourceComponents corresponds to the JSON schema field "source_components".
	SourceComponents []WorkspaceComponentID `json:"source_components"`
}

type WorkspaceNamespaceID string

type WorkspaceObligationID string

type WorkspacePolicyCeilingV1 struct {
	// Authorities corresponds to the JSON schema field "authorities".
	Authorities WorkspaceAuthorityCeilingV1 `json:"authorities"`

	// Controllers corresponds to the JSON schema field "controllers".
	Controllers WorkspaceControllerCeilingV1 `json:"controllers"`

	// Disclosure corresponds to the JSON schema field "disclosure".
	Disclosure DisclosureCeilingV1 `json:"disclosure"`

	// Effect corresponds to the JSON schema field "effect".
	Effect WorkspaceEffectCeilingV1 `json:"effect"`

	// ModelUsePermitted corresponds to the JSON schema field "model_use_permitted".
	ModelUsePermitted bool `json:"model_use_permitted"`

	// RetentionSeconds corresponds to the JSON schema field "retention_seconds".
	RetentionSeconds int `json:"retention_seconds"`

	// Rights corresponds to the JSON schema field "rights".
	Rights WorkspaceRightsCeilingV1 `json:"rights"`
}

type WorkspacePropagationV1 string

const WorkspacePropagationV1Prevent WorkspacePropagationV1 = "prevent"
const WorkspacePropagationV1Transitive WorkspacePropagationV1 = "transitive"

type WorkspacePublisherID string

type WorkspacePublisherKeyID string

type WorkspacePublisherRefV1 struct {
	// DelegatedReleaseKeyID corresponds to the JSON schema field
	// "delegated_release_key_id".
	DelegatedReleaseKeyID WorkspacePublisherKeyID `json:"delegated_release_key_id"`

	// PublisherID corresponds to the JSON schema field "publisher_id".
	PublisherID WorkspacePublisherID `json:"publisher_id"`

	// RecoveryAuthorityID corresponds to the JSON schema field
	// "recovery_authority_id".
	RecoveryAuthorityID WorkspaceRecoveryAuthorityID `json:"recovery_authority_id"`

	// RootKeyID corresponds to the JSON schema field "root_key_id".
	RootKeyID WorkspacePublisherKeyID `json:"root_key_id"`
}

type WorkspaceRecoveryAuthorityID string

type WorkspaceReleaseID string

type WorkspaceResourceID string

type WorkspaceResourceKindV1 string

const WorkspaceResourceKindV1NativeApplication WorkspaceResourceKindV1 = "native_application"
const WorkspaceResourceKindV1NormativePack WorkspaceResourceKindV1 = "normative_pack"
const WorkspaceResourceKindV1Organization WorkspaceResourceKindV1 = "organization"
const WorkspaceResourceKindV1Package WorkspaceResourceKindV1 = "package"
const WorkspaceResourceKindV1Participant WorkspaceResourceKindV1 = "participant"
const WorkspaceResourceKindV1Principal WorkspaceResourceKindV1 = "principal"

type WorkspaceResourceSelectionNativeApplicationV1 struct {
	// AuthorityResolver corresponds to the JSON schema field "authority_resolver".
	AuthorityResolver WorkspaceResourceID `json:"authority_resolver"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID WorkspaceResourceID `json:"resource_id"`
}

type WorkspaceResourceSelectionNormativePackV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Release corresponds to the JSON schema field "release".
	Release NormativePackReleaseV1 `json:"release"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID WorkspaceResourceID `json:"resource_id"`
}

type WorkspaceResourceSelectionOrganizationV1 struct {
	// AuthorityResolver corresponds to the JSON schema field "authority_resolver".
	AuthorityResolver WorkspaceResourceID `json:"authority_resolver"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID WorkspaceResourceID `json:"resource_id"`
}

type WorkspaceResourceSelectionPackageV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Release corresponds to the JSON schema field "release".
	Release PackageReleaseV1 `json:"release"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID WorkspaceResourceID `json:"resource_id"`
}

type WorkspaceResourceSelectionParticipantV1 struct {
	// AuthorityResolver corresponds to the JSON schema field "authority_resolver".
	AuthorityResolver WorkspaceResourceID `json:"authority_resolver"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID WorkspaceResourceID `json:"resource_id"`
}

type WorkspaceResourceSelectionPrincipalV1 struct {
	// AuthorityResolver corresponds to the JSON schema field "authority_resolver".
	AuthorityResolver WorkspaceResourceID `json:"authority_resolver"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID WorkspaceResourceID `json:"resource_id"`
}

type WorkspaceRightID string

type WorkspaceRightsCeilingV1 struct {
	// Permitted corresponds to the JSON schema field "permitted".
	Permitted []WorkspaceRightID `json:"permitted"`
}

type WorkspaceValueKindV1 string

const WorkspaceValueKindV1Boolean WorkspaceValueKindV1 = "boolean"
const WorkspaceValueKindV1Integer WorkspaceValueKindV1 = "integer"
const WorkspaceValueKindV1LogicalID WorkspaceValueKindV1 = "logical_id"
const WorkspaceValueKindV1Text WorkspaceValueKindV1 = "text"

type WorkspaceVisibilityV1 string

const WorkspaceVisibilityV1Private WorkspaceVisibilityV1 = "private"
const WorkspaceVisibilityV1Protected WorkspaceVisibilityV1 = "protected"
const WorkspaceVisibilityV1Public WorkspaceVisibilityV1 = "public"

type WorkspaceDeclarationV1 struct {
	Trait        *WorkspaceDeclarationTraitV1        `json:"-"`
	Attribute    *WorkspaceDeclarationAttributeV1    `json:"-"`
	Relationship *WorkspaceDeclarationRelationshipV1 `json:"-"`
	Policy       *WorkspaceDeclarationPolicyV1       `json:"-"`
	Resource     *WorkspaceDeclarationResourceV1     `json:"-"`
}

func (value WorkspaceDeclarationV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Trait != nil {
		if value.Trait.Kind != "trait" {
			return nil, fmt.Errorf("WorkspaceDeclarationV1: selected variant has discriminator %q", value.Trait.Kind)
		}
		candidate, err := json.Marshal(value.Trait)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Attribute != nil {
		if value.Attribute.Kind != "attribute" {
			return nil, fmt.Errorf("WorkspaceDeclarationV1: selected variant has discriminator %q", value.Attribute.Kind)
		}
		candidate, err := json.Marshal(value.Attribute)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Relationship != nil {
		if value.Relationship.Kind != "relationship" {
			return nil, fmt.Errorf("WorkspaceDeclarationV1: selected variant has discriminator %q", value.Relationship.Kind)
		}
		candidate, err := json.Marshal(value.Relationship)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Policy != nil {
		if value.Policy.Kind != "policy" {
			return nil, fmt.Errorf("WorkspaceDeclarationV1: selected variant has discriminator %q", value.Policy.Kind)
		}
		candidate, err := json.Marshal(value.Policy)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Resource != nil {
		if value.Resource.Kind != "resource" {
			return nil, fmt.Errorf("WorkspaceDeclarationV1: selected variant has discriminator %q", value.Resource.Kind)
		}
		candidate, err := json.Marshal(value.Resource)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("WorkspaceDeclarationV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *WorkspaceDeclarationV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = WorkspaceDeclarationV1{}
	switch discriminator.Kind {
	case "trait":
		var variant WorkspaceDeclarationTraitV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Trait = &variant
	case "attribute":
		var variant WorkspaceDeclarationAttributeV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Attribute = &variant
	case "relationship":
		var variant WorkspaceDeclarationRelationshipV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Relationship = &variant
	case "policy":
		var variant WorkspaceDeclarationPolicyV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Policy = &variant
	case "resource":
		var variant WorkspaceDeclarationResourceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Resource = &variant
	default:
		return fmt.Errorf("WorkspaceDeclarationV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type WorkspaceResourceSelectionV1 struct {
	Principal         *WorkspaceResourceSelectionPrincipalV1         `json:"-"`
	Organization      *WorkspaceResourceSelectionOrganizationV1      `json:"-"`
	NativeApplication *WorkspaceResourceSelectionNativeApplicationV1 `json:"-"`
	Participant       *WorkspaceResourceSelectionParticipantV1       `json:"-"`
	Package           *WorkspaceResourceSelectionPackageV1           `json:"-"`
	NormativePack     *WorkspaceResourceSelectionNormativePackV1     `json:"-"`
}

func (value WorkspaceResourceSelectionV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Principal != nil {
		if value.Principal.Kind != "principal" {
			return nil, fmt.Errorf("WorkspaceResourceSelectionV1: selected variant has discriminator %q", value.Principal.Kind)
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
			return nil, fmt.Errorf("WorkspaceResourceSelectionV1: selected variant has discriminator %q", value.Organization.Kind)
		}
		candidate, err := json.Marshal(value.Organization)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.NativeApplication != nil {
		if value.NativeApplication.Kind != "native_application" {
			return nil, fmt.Errorf("WorkspaceResourceSelectionV1: selected variant has discriminator %q", value.NativeApplication.Kind)
		}
		candidate, err := json.Marshal(value.NativeApplication)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Participant != nil {
		if value.Participant.Kind != "participant" {
			return nil, fmt.Errorf("WorkspaceResourceSelectionV1: selected variant has discriminator %q", value.Participant.Kind)
		}
		candidate, err := json.Marshal(value.Participant)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Package != nil {
		if value.Package.Kind != "package" {
			return nil, fmt.Errorf("WorkspaceResourceSelectionV1: selected variant has discriminator %q", value.Package.Kind)
		}
		candidate, err := json.Marshal(value.Package)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.NormativePack != nil {
		if value.NormativePack.Kind != "normative_pack" {
			return nil, fmt.Errorf("WorkspaceResourceSelectionV1: selected variant has discriminator %q", value.NormativePack.Kind)
		}
		candidate, err := json.Marshal(value.NormativePack)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("WorkspaceResourceSelectionV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *WorkspaceResourceSelectionV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = WorkspaceResourceSelectionV1{}
	switch discriminator.Kind {
	case "principal":
		var variant WorkspaceResourceSelectionPrincipalV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Principal = &variant
	case "organization":
		var variant WorkspaceResourceSelectionOrganizationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Organization = &variant
	case "native_application":
		var variant WorkspaceResourceSelectionNativeApplicationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NativeApplication = &variant
	case "participant":
		var variant WorkspaceResourceSelectionParticipantV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Participant = &variant
	case "package":
		var variant WorkspaceResourceSelectionPackageV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Package = &variant
	case "normative_pack":
		var variant WorkspaceResourceSelectionNormativePackV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NormativePack = &variant
	default:
		return fmt.Errorf("WorkspaceResourceSelectionV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
