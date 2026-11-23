package pale

import (
	"encoding/json"
	"fmt"
)

// AuthoringIngressClass selects one compiler-owned source ceiling.
type AuthoringIngressClass string

const (
	AuthoringWorkspace AuthoringIngressClass = "workspace"
	AuthoringPrivate   AuthoringIngressClass = "private"
	AuthoringPackage   AuthoringIngressClass = "package"
)

// AuthoringIngressLimits are the exact finite bounds for one source class.
type AuthoringIngressLimits struct {
	MaxDocuments     uint64 `json:"maxDocuments"`
	MaxDocumentBytes uint64 `json:"maxDocumentBytes"`
	MaxTotalBytes    uint64 `json:"maxTotalBytes"`
}

// AuthoringRole identifies one standalone compiler grammar.
type AuthoringRole string

const (
	RoleWorkspaceManifest           AuthoringRole = "workspace-manifest"
	RoleWorkspaceModule             AuthoringRole = "workspace-module"
	RoleWorkspaceLanguagePrimitives AuthoringRole = "workspace-language-primitives"
	RoleResponsibilityDerivation    AuthoringRole = "responsibility-derivation"
	RoleOrganizationalTranslation   AuthoringRole = "organizational-translation"
	RoleExperienceComposition       AuthoringRole = "experience-composition"
	RoleWorkspaceResolutionLock     AuthoringRole = "workspace-resolution-lock"
	RoleWorkspaceUniverseLock       AuthoringRole = "workspace-universe-lock"
	RoleCapabilities                AuthoringRole = "capabilities"
	RoleMachineBinding              AuthoringRole = "machine-binding"
	RoleDistributionLock            AuthoringRole = "distribution-lock"
	RolePackageManifest             AuthoringRole = "package-manifest"
	RolePackageDoctrine             AuthoringRole = "package-doctrine"
	RolePackageOperations           AuthoringRole = "package-operations"
	RolePackageRoles                AuthoringRole = "package-roles"
	RolePackageStacks               AuthoringRole = "package-stacks"
	RolePackageRunModes             AuthoringRole = "package-run-modes"
	RolePackagePolicyFamily         AuthoringRole = "package-policy-family"
	RolePackageGoverningOrder       AuthoringRole = "package-governing-order"
	RolePackageCapabilityPolicy     AuthoringRole = "package-capability-policy"
	RolePackageToolRecipe           AuthoringRole = "package-tool-recipe"
)

// StructuralPathSegment identifies a grammar node without selecting an array index.
type StructuralPathSegment struct {
	kind  string
	value string
}

// AuthoringFieldPath constructs one exact structural field step.
func AuthoringFieldPath(name string) StructuralPathSegment {
	return StructuralPathSegment{kind: "field", value: name}
}

// AuthoringArrayItemPath constructs one structural array-item step.
func AuthoringArrayItemPath() StructuralPathSegment {
	return StructuralPathSegment{kind: "array-item"}
}

// MarshalJSON preserves the API-owned tagged path representation.
func (segment StructuralPathSegment) MarshalJSON() ([]byte, error) {
	if segment.kind == "array-item" {
		return []byte(`{"kind":"array-item"}`), nil
	}
	return json.Marshal(struct {
		Kind  string `json:"kind"`
		Value string `json:"value"`
	}{segment.kind, segment.value})
}

// InstancePathSegment identifies one exact source occurrence.
type InstancePathSegment struct {
	kind  string
	field string
	index uint32
}

// AuthoringField constructs one exact decoded-key step.
func AuthoringField(name string) InstancePathSegment {
	return InstancePathSegment{kind: "field", field: name}
}

// AuthoringIndex constructs one exact array-member step.
func AuthoringIndex(index uint32) InstancePathSegment {
	return InstancePathSegment{kind: "index", index: index}
}

// MarshalJSON preserves the API-owned tagged path representation without an
// untyped Go value.
func (segment InstancePathSegment) MarshalJSON() ([]byte, error) {
	if segment.kind == "index" {
		return json.Marshal(struct {
			Kind  string `json:"kind"`
			Value uint32 `json:"value"`
		}{segment.kind, segment.index})
	}
	return json.Marshal(struct {
		Kind  string `json:"kind"`
		Value string `json:"value"`
	}{segment.kind, segment.field})
}

// AuthoringInspectionState describes availability of compiler-owned grammar facts.
type AuthoringInspectionState string

const (
	AuthoringAvailable       AuthoringInspectionState = "available"
	AuthoringTextDocument    AuthoringInspectionState = "text-document"
	AuthoringUnknownDocument AuthoringInspectionState = "unknown-document"
	AuthoringUnknownPath     AuthoringInspectionState = "unknown-path"
)

// UnmarshalJSON decodes only the SDK's closed availability vocabulary.
func (value *AuthoringInspectionState) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, AuthoringAvailable, AuthoringTextDocument, AuthoringUnknownDocument, AuthoringUnknownPath)
}

// AuthoringValueKind is a structural TOML category, not a semantic admission.
type AuthoringValueKind string

const (
	AuthoringBoolean AuthoringValueKind = "boolean"
	AuthoringInteger AuthoringValueKind = "integer"
	AuthoringString  AuthoringValueKind = "string"
	AuthoringArray   AuthoringValueKind = "array"
	AuthoringTable   AuthoringValueKind = "table"
)

// UnmarshalJSON rejects unknown structural categories.
func (value *AuthoringValueKind) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, AuthoringBoolean, AuthoringInteger, AuthoringString, AuthoringArray, AuthoringTable)
}

// AuthoringFieldPresence distinguishes unconditional and variant-dependent requirements.
type AuthoringFieldPresence string

const (
	AuthoringRequired         AuthoringFieldPresence = "required"
	AuthoringOptional         AuthoringFieldPresence = "optional"
	AuthoringVariantDependent AuthoringFieldPresence = "variant-dependent"
)

// UnmarshalJSON decodes the compiler's requiredness result without inferring it.
func (value *AuthoringFieldPresence) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, AuthoringRequired, AuthoringOptional, AuthoringVariantDependent)
}

// AuthoringLiteral retains exactly one TOML scalar payload selected by Kind.
// Only String, Integer and Boolean are literal kinds; arrays and tables are not.
type AuthoringLiteral struct {
	Kind    AuthoringValueKind
	String  *string
	Integer *int64
	Boolean *bool
}

// UnmarshalJSON checks the closed scalar payload atomically, including int64 bounds.
func (value *AuthoringLiteral) UnmarshalJSON(data []byte) error {
	var wire struct {
		Kind  AuthoringValueKind `json:"kind"`
		Value json.RawMessage    `json:"value"`
	}
	if err := decodeTransportObject(data, &wire, []string{"kind", "value"}); err != nil {
		return err
	}
	next := AuthoringLiteral{Kind: wire.Kind}
	var err error
	switch wire.Kind {
	case AuthoringString:
		var payload string
		err = json.Unmarshal(wire.Value, &payload)
		next.String = &payload
	case AuthoringInteger:
		var payload int64
		err = json.Unmarshal(wire.Value, &payload)
		next.Integer = &payload
	case AuthoringBoolean:
		var payload bool
		err = json.Unmarshal(wire.Value, &payload)
		next.Boolean = &payload
	default:
		err = fmt.Errorf("invalid authoring literal kind")
	}
	if err != nil {
		return fmt.Errorf("invalid authoring literal payload")
	}
	*value = next
	return nil
}

// AuthoringLiteralFact pairs one closed literal with optional producer documentation.
type AuthoringLiteralFact struct {
	Value       AuthoringLiteral `json:"value"`
	Description *string          `json:"description"`
}

// UnmarshalJSON preserves absent documentation separately from an empty description.
func (value *AuthoringLiteralFact) UnmarshalJSON(data []byte) error {
	type plain AuthoringLiteralFact
	var next plain
	if err := decodeTransportObject(data, &next, []string{"value"}, "description"); err != nil {
		return err
	}
	*value = AuthoringLiteralFact(next)
	return nil
}

// AuthoringFieldFact describes one child key in compiler-defined order.
type AuthoringFieldFact struct {
	Name          string                 `json:"name"`
	Description   *string                `json:"description"`
	Presence      AuthoringFieldPresence `json:"presence"`
	ValueKinds    []AuthoringValueKind   `json:"value_kinds"`
	AllowedValues []AuthoringLiteralFact `json:"allowed_values"`
}

// UnmarshalJSON checks field metadata without interpreting schema or policy.
func (value *AuthoringFieldFact) UnmarshalJSON(data []byte) error {
	type plain AuthoringFieldFact
	var next plain
	if err := decodeTransportObject(data, &next, []string{"name", "presence", "value_kinds", "allowed_values"}, "description"); err != nil {
		return err
	}
	*value = AuthoringFieldFact(next)
	return nil
}

// AuthoringNodeFact contains bounded grammar facts, never the inspected source value.
// Nullable scalar bounds retain their exact signed or unsigned SDK widths.
type AuthoringNodeFact struct {
	Description   *string                `json:"description"`
	ValueKinds    []AuthoringValueKind   `json:"value_kinds"`
	AllowedValues []AuthoringLiteralFact `json:"allowed_values"`
	Fields        []AuthoringFieldFact   `json:"fields"`
	Format        *string                `json:"format"`
	Pattern       *string                `json:"pattern"`
	Minimum       *int64                 `json:"minimum"`
	Maximum       *int64                 `json:"maximum"`
	MinLength     *uint64                `json:"min_length"`
	MaxLength     *uint64                `json:"max_length"`
	MinItems      *uint64                `json:"min_items"`
	MaxItems      *uint64                `json:"max_items"`
}

// UnmarshalJSON checks nullable fields and complete ordered collections atomically.
func (value *AuthoringNodeFact) UnmarshalJSON(data []byte) error {
	type plain AuthoringNodeFact
	var next plain
	if err := decodeTransportObject(data, &next, []string{"value_kinds", "allowed_values", "fields"}, "description", "format", "pattern", "minimum", "maximum", "min_length", "max_length", "min_items", "max_items"); err != nil {
		return err
	}
	*value = AuthoringNodeFact(next)
	return nil
}

// AuthoringInspection is the SDK's grammar result for an exact structural query.
// Node is absent when no grammar node was resolved; this result cannot admit a
// source edit, select a profile or establish runtime authority.
type AuthoringInspection struct {
	State      AuthoringInspectionState `json:"state"`
	SchemaID   *string                  `json:"schema_id"`
	Definition *string                  `json:"definition"`
	Node       *AuthoringNodeFact       `json:"node"`
}

// UnmarshalJSON decodes transport shape without reclassifying availability.
func (value *AuthoringInspection) UnmarshalJSON(data []byte) error {
	type plain AuthoringInspection
	var next plain
	if err := decodeTransportObject(data, &next, []string{"state"}, "schema_id", "definition", "node"); err != nil {
		return err
	}
	*value = AuthoringInspection(next)
	return nil
}

// AuthoringIngressLimits returns the compiler-owned bound for one custody class.
func (client *Client) AuthoringIngressLimits(class AuthoringIngressClass) (AuthoringIngressLimits, error) {
	result, err := client.call(class, opAuthoringIngressLimits)
	if err != nil {
		return AuthoringIngressLimits{}, err
	}
	var limits AuthoringIngressLimits
	if err := decodeJSON(result.metadata, &limits); err != nil {
		return AuthoringIngressLimits{}, err
	}
	return limits, nil
}

// ValidateAuthoringDocument checks one document against its owning grammar.
func (client *Client) ValidateAuthoringDocument(role AuthoringRole, sourceLabel string, bytes []byte) error {
	_, err := client.call(struct {
		Role        AuthoringRole `json:"role"`
		SourceLabel string        `json:"sourceLabel"`
		Bytes       []byte        `json:"bytes"`
	}{role, sourceLabel, bytes}, opValidateAuthoringDocument)
	return err
}

// InspectStandaloneAuthoring returns compiler-generated grammar facts.
func (client *Client) InspectStandaloneAuthoring(role AuthoringRole, path []StructuralPathSegment) (AuthoringInspection, error) {
	result, err := client.call(struct {
		Role AuthoringRole           `json:"role"`
		Path []StructuralPathSegment `json:"path"`
	}{role, path}, opInspectStandaloneAuthoring)
	if err != nil {
		return AuthoringInspection{}, err
	}
	return decodeMetadata[AuthoringInspection](result)
}
