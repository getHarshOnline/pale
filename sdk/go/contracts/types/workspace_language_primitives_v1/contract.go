// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package workspace_language_primitives_v1 contains the generated workspace-language-primitives.v1 data model.
package workspace_language_primitives_v1

import (
	"encoding/json"
	"fmt"
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

type WorkspaceLanguagePrimitivesV1 struct {
	// ComponentID corresponds to the JSON schema field "component_id".
	ComponentID WorkspaceComponentID `json:"component_id"`

	// Declarations corresponds to the JSON schema field "declarations".
	Declarations []WorkspaceDeclarationV1 `json:"declarations"`

	// ModuleID corresponds to the JSON schema field "module_id".
	ModuleID WorkspaceModuleID `json:"module_id"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`
}

type WorkspaceModuleID string

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

type WorkspaceResourceID string

type WorkspaceResourceKindV1 string

const WorkspaceResourceKindV1NativeApplication WorkspaceResourceKindV1 = "native_application"
const WorkspaceResourceKindV1NormativePack WorkspaceResourceKindV1 = "normative_pack"
const WorkspaceResourceKindV1Organization WorkspaceResourceKindV1 = "organization"
const WorkspaceResourceKindV1Package WorkspaceResourceKindV1 = "package"
const WorkspaceResourceKindV1Participant WorkspaceResourceKindV1 = "participant"
const WorkspaceResourceKindV1Principal WorkspaceResourceKindV1 = "principal"

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
