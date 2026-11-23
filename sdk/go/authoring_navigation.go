package pale

import (
	"encoding/json"
	"fmt"
)

// AuthoringIdentityKind is the SDK's closed semantic namespace, not a native resource type.
type AuthoringIdentityKind string

const (
	AuthoringIdentitySourceDocument                    AuthoringIdentityKind = "source-document"
	AuthoringIdentityWorkspace                         AuthoringIdentityKind = "workspace"
	AuthoringIdentityProject                           AuthoringIdentityKind = "project"
	AuthoringIdentityExecutionWorkspace                AuthoringIdentityKind = "execution-workspace"
	AuthoringIdentityRepository                        AuthoringIdentityKind = "repository"
	AuthoringIdentityExecutionPath                     AuthoringIdentityKind = "execution-path"
	AuthoringIdentityModule                            AuthoringIdentityKind = "module"
	AuthoringIdentityModuleRelease                     AuthoringIdentityKind = "module-release"
	AuthoringIdentityComponent                         AuthoringIdentityKind = "component"
	AuthoringIdentityExport                            AuthoringIdentityKind = "export"
	AuthoringIdentityNamespace                         AuthoringIdentityKind = "namespace"
	AuthoringIdentityDeclaration                       AuthoringIdentityKind = "declaration"
	AuthoringIdentityResource                          AuthoringIdentityKind = "resource"
	AuthoringIdentityRunPolicy                         AuthoringIdentityKind = "run-policy"
	AuthoringIdentityLifecycleBehavior                 AuthoringIdentityKind = "lifecycle-behavior"
	AuthoringIdentityCapability                        AuthoringIdentityKind = "capability"
	AuthoringIdentityAuthorityRoot                     AuthoringIdentityKind = "authority-root"
	AuthoringIdentityProviderBinding                   AuthoringIdentityKind = "provider-binding"
	AuthoringIdentityMachine                           AuthoringIdentityKind = "machine"
	AuthoringIdentityResponsibilityDerivation          AuthoringIdentityKind = "responsibility-derivation"
	AuthoringIdentityResponsibilityProjection          AuthoringIdentityKind = "responsibility-projection"
	AuthoringIdentityResponsibilityGenealogyLink       AuthoringIdentityKind = "responsibility-genealogy-link"
	AuthoringIdentityLogicalScope                      AuthoringIdentityKind = "logical-scope"
	AuthoringIdentityLogicalOrganization               AuthoringIdentityKind = "logical-organization"
	AuthoringIdentityOrganizationalGroup               AuthoringIdentityKind = "organizational-group"
	AuthoringIdentityDisciplineTree                    AuthoringIdentityKind = "discipline-tree"
	AuthoringIdentityOrganizationalRole                AuthoringIdentityKind = "organizational-role"
	AuthoringIdentityOrganizationalOccupant            AuthoringIdentityKind = "organizational-occupant"
	AuthoringIdentityOrganizationalProvider            AuthoringIdentityKind = "organizational-provider"
	AuthoringIdentityCommunicationActor                AuthoringIdentityKind = "communication-actor"
	AuthoringIdentityLeadershipTopic                   AuthoringIdentityKind = "leadership-topic"
	AuthoringIdentityOrganizationalRelation            AuthoringIdentityKind = "organizational-relation"
	AuthoringIdentityOrganizationalObject              AuthoringIdentityKind = "organizational-object"
	AuthoringIdentityGroupLensProfile                  AuthoringIdentityKind = "group-lens-profile"
	AuthoringIdentityDirectionalService                AuthoringIdentityKind = "directional-service"
	AuthoringIdentityOrganizationalProjection          AuthoringIdentityKind = "organizational-projection"
	AuthoringIdentityOrganizationalInspection          AuthoringIdentityKind = "organizational-inspection"
	AuthoringIdentityCompositionParticipant            AuthoringIdentityKind = "composition-participant"
	AuthoringIdentityCompositionResponsibilityContract AuthoringIdentityKind = "composition-responsibility-contract"
	AuthoringIdentityCompositionParticipantManifest    AuthoringIdentityKind = "composition-participant-manifest"
	AuthoringIdentityCompositionProvider               AuthoringIdentityKind = "composition-provider"
	AuthoringIdentityCompositionProviderBinding        AuthoringIdentityKind = "composition-provider-binding"
	AuthoringIdentityCompositionEntryContract          AuthoringIdentityKind = "composition-entry-contract"
	AuthoringIdentityCompositionRequestTranslation     AuthoringIdentityKind = "composition-request-translation"
	AuthoringIdentityCompositionResultTranslation      AuthoringIdentityKind = "composition-result-translation"
	AuthoringIdentityCompositionTerminationContract    AuthoringIdentityKind = "composition-termination-contract"
	AuthoringIdentityCompositionSettlementContract     AuthoringIdentityKind = "composition-settlement-contract"
	AuthoringIdentityCompositionContinuityContract     AuthoringIdentityKind = "composition-continuity-contract"
	AuthoringIdentityCompositionEdge                   AuthoringIdentityKind = "composition-edge"
	AuthoringIdentityExperiencePath                    AuthoringIdentityKind = "experience-path"
	AuthoringIdentityPackageDoctrine                   AuthoringIdentityKind = "package-doctrine"
	AuthoringIdentityPackageOperationArchetype         AuthoringIdentityKind = "package-operation-archetype"
	AuthoringIdentityPackageOperation                  AuthoringIdentityKind = "package-operation"
	AuthoringIdentityPackageRoleArchetype              AuthoringIdentityKind = "package-role-archetype"
	AuthoringIdentityPackageRole                       AuthoringIdentityKind = "package-role"
	AuthoringIdentityPackageStack                      AuthoringIdentityKind = "package-stack"
	AuthoringIdentityPackageRunMode                    AuthoringIdentityKind = "package-run-mode"
	AuthoringIdentityPackagePolicyFamily               AuthoringIdentityKind = "package-policy-family"
	AuthoringIdentityPackageGoverningOrder             AuthoringIdentityKind = "package-governing-order"
	AuthoringIdentityPackageCapabilityPolicy           AuthoringIdentityKind = "package-capability-policy"
	AuthoringIdentityPackageToolRecipe                 AuthoringIdentityKind = "package-tool-recipe"
	AuthoringIdentityPackage                           AuthoringIdentityKind = "package"
)

// UnmarshalJSON rejects namespaces outside the SDK projection.
func (value *AuthoringIdentityKind) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value,
		AuthoringIdentitySourceDocument,
		AuthoringIdentityWorkspace,
		AuthoringIdentityProject,
		AuthoringIdentityExecutionWorkspace,
		AuthoringIdentityRepository,
		AuthoringIdentityExecutionPath,
		AuthoringIdentityModule,
		AuthoringIdentityModuleRelease,
		AuthoringIdentityComponent,
		AuthoringIdentityExport,
		AuthoringIdentityNamespace,
		AuthoringIdentityDeclaration,
		AuthoringIdentityResource,
		AuthoringIdentityRunPolicy,
		AuthoringIdentityLifecycleBehavior,
		AuthoringIdentityCapability,
		AuthoringIdentityAuthorityRoot,
		AuthoringIdentityProviderBinding,
		AuthoringIdentityMachine,
		AuthoringIdentityResponsibilityDerivation,
		AuthoringIdentityResponsibilityProjection,
		AuthoringIdentityResponsibilityGenealogyLink,
		AuthoringIdentityLogicalScope,
		AuthoringIdentityLogicalOrganization,
		AuthoringIdentityOrganizationalGroup,
		AuthoringIdentityDisciplineTree,
		AuthoringIdentityOrganizationalRole,
		AuthoringIdentityOrganizationalOccupant,
		AuthoringIdentityOrganizationalProvider,
		AuthoringIdentityCommunicationActor,
		AuthoringIdentityLeadershipTopic,
		AuthoringIdentityOrganizationalRelation,
		AuthoringIdentityOrganizationalObject,
		AuthoringIdentityGroupLensProfile,
		AuthoringIdentityDirectionalService,
		AuthoringIdentityOrganizationalProjection,
		AuthoringIdentityOrganizationalInspection,
		AuthoringIdentityCompositionParticipant,
		AuthoringIdentityCompositionResponsibilityContract,
		AuthoringIdentityCompositionParticipantManifest,
		AuthoringIdentityCompositionProvider,
		AuthoringIdentityCompositionProviderBinding,
		AuthoringIdentityCompositionEntryContract,
		AuthoringIdentityCompositionRequestTranslation,
		AuthoringIdentityCompositionResultTranslation,
		AuthoringIdentityCompositionTerminationContract,
		AuthoringIdentityCompositionSettlementContract,
		AuthoringIdentityCompositionContinuityContract,
		AuthoringIdentityCompositionEdge,
		AuthoringIdentityExperiencePath,
		AuthoringIdentityPackageDoctrine,
		AuthoringIdentityPackageOperationArchetype,
		AuthoringIdentityPackageOperation,
		AuthoringIdentityPackageRoleArchetype,
		AuthoringIdentityPackageRole,
		AuthoringIdentityPackageStack,
		AuthoringIdentityPackageRunMode,
		AuthoringIdentityPackagePolicyFamily,
		AuthoringIdentityPackageGoverningOrder,
		AuthoringIdentityPackageCapabilityPolicy,
		AuthoringIdentityPackageToolRecipe,
		AuthoringIdentityPackage)
}

// AuthoringIdentityValue copies the compiler's canonical identity and its exact
// authored scalar spelling. These fields grant no native handle or authority.
type AuthoringIdentityValue struct {
	Kind          AuthoringIdentityKind `json:"kind"`
	Value         string                `json:"value"`
	AuthoredValue string                `json:"authored_value"`
}

// UnmarshalJSON requires the exact identity fields without changing their meaning.
func (value *AuthoringIdentityValue) UnmarshalJSON(data []byte) error {
	type plain AuthoringIdentityValue
	var next plain
	if err := decodeTransportObject(data, &next, []string{"kind", "value", "authored_value"}); err != nil {
		return err
	}
	*value = AuthoringIdentityValue(next)
	return nil
}

// AuthoringIdentity is an opaque compiler-resolved identity usable for a profile
// join. Description copies identity fields, not free-form prose. Editing it
// cannot change the native identity selected by the private handle.
type AuthoringIdentity struct {
	handle
	Description AuthoringIdentityValue
}

// Close releases only this identity and invalidates its copied handles.
// Description stays readable. Nil and repeated calls are harmless.
func (identity *AuthoringIdentity) Close() {
	if identity != nil {
		closeNativeHandle(&identity.handle, dropAuthoringIdentity)
	}
}

// AuthoringOccurrenceRole preserves the linker's exact source-occurrence role.
type AuthoringOccurrenceRole string

const (
	AuthoringDefinition AuthoringOccurrenceRole = "definition"
	AuthoringReference  AuthoringOccurrenceRole = "reference"
	AuthoringProjection AuthoringOccurrenceRole = "projection"
)

// UnmarshalJSON rejects unknown occurrence roles.
func (value *AuthoringOccurrenceRole) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, AuthoringDefinition, AuthoringReference, AuthoringProjection)
}

// AuthoringLocationSegmentKind distinguishes decoded keys from array indices.
type AuthoringLocationSegmentKind string

const (
	AuthoringLocationField AuthoringLocationSegmentKind = "field"
	AuthoringLocationIndex AuthoringLocationSegmentKind = "index"
)

// AuthoringLocationSegment copies one output path step. Exactly one payload is
// present. Index retains the native unsigned width, independently of the uint32
// input builder; no source location is narrowed during decoding.
type AuthoringLocationSegment struct {
	Kind  AuthoringLocationSegmentKind
	Field *string
	Index *uint64
}

// UnmarshalJSON checks the exact tagged payload atomically.
func (value *AuthoringLocationSegment) UnmarshalJSON(data []byte) error {
	var wire struct {
		Kind  string          `json:"kind"`
		Value json.RawMessage `json:"value"`
	}
	if err := decodeTransportObject(data, &wire, []string{"kind", "value"}); err != nil {
		return err
	}
	next := AuthoringLocationSegment{Kind: AuthoringLocationSegmentKind(wire.Kind)}
	switch next.Kind {
	case AuthoringLocationField:
		var field string
		if err := json.Unmarshal(wire.Value, &field); err != nil {
			return fmt.Errorf("invalid authoring field location")
		}
		next.Field = &field
	case AuthoringLocationIndex:
		var index uint64
		if err := json.Unmarshal(wire.Value, &index); err != nil {
			return fmt.Errorf("invalid authoring index location")
		}
		next.Index = &index
	default:
		return fmt.Errorf("unknown authoring location kind")
	}
	*value = next
	return nil
}

// AuthoringSourceLocation is a logical source label and ordered decoded path,
// not a filesystem locator or a live editor range.
type AuthoringSourceLocation struct {
	SourceLabel string                     `json:"source_label"`
	Path        []AuthoringLocationSegment `json:"path"`
}

// UnmarshalJSON requires the complete source-free location.
func (value *AuthoringSourceLocation) UnmarshalJSON(data []byte) error {
	type plain AuthoringSourceLocation
	var next plain
	if err := decodeTransportObject(data, &next, []string{"source_label", "path"}); err != nil {
		return err
	}
	*value = AuthoringSourceLocation(next)
	return nil
}

// AuthoringNavigation is the SDK's semantic lookup, with reusable identities.
// Missing occurrences can still have completion alternatives. It does not
// classify live text, grant write authority, or establish machine readiness.
type AuthoringNavigation struct {
	Identity              *AuthoringIdentity
	OccurrenceRole        *AuthoringOccurrenceRole
	Definition            *AuthoringSourceLocation
	References            []AuthoringSourceLocation
	Alternatives          []AuthoringIdentity
	ReferencesTruncated   bool
	AlternativesTruncated bool
	ownedIdentities       []handle
}

// Close releases every identity originally issued in this result, including
// copied handles and identities removed from or replaced in public fields.
// Keep the result open while an identity is needed, or close unwanted identities
// individually. Copied descriptive data and the profile remain usable.
// Nil and repeated calls are harmless.
func (navigation *AuthoringNavigation) Close() {
	if navigation == nil {
		return
	}
	for index := range navigation.ownedIdentities {
		closeNativeHandle(&navigation.ownedIdentities[index], dropAuthoringIdentity)
	}
}

type authoringNavigationValue struct {
	Identity              *AuthoringIdentityValue   `json:"identity"`
	OccurrenceRole        *AuthoringOccurrenceRole  `json:"occurrence_role"`
	Definition            *AuthoringSourceLocation  `json:"definition"`
	References            []AuthoringSourceLocation `json:"references"`
	Alternatives          []AuthoringIdentityValue  `json:"alternatives"`
	ReferencesTruncated   bool                      `json:"references_truncated"`
	AlternativesTruncated bool                      `json:"alternatives_truncated"`
}

func (value *authoringNavigationValue) UnmarshalJSON(data []byte) error {
	type plain authoringNavigationValue
	var next plain
	if err := decodeTransportObject(data, &next,
		[]string{"references", "alternatives", "references_truncated", "alternatives_truncated"},
		"identity", "occurrence_role", "definition"); err != nil {
		return err
	}
	*value = authoringNavigationValue(next)
	return nil
}

type authoringAlternativeHandle struct {
	AlternativeIndex uint64 `json:"alternativeIndex"`
	Handle           uint64 `json:"handle"`
}

func (value *authoringAlternativeHandle) UnmarshalJSON(data []byte) error {
	type plain authoringAlternativeHandle
	var next plain
	if err := decodeTransportObject(data, &next, []string{"alternativeIndex", "handle"}); err != nil {
		return err
	}
	*value = authoringAlternativeHandle(next)
	return nil
}

type authoringNavigationEnvelope struct {
	Navigation         authoringNavigationValue     `json:"navigation"`
	AlternativeHandles []authoringAlternativeHandle `json:"alternativeHandles"`
}

func (value *authoringNavigationEnvelope) UnmarshalJSON(data []byte) error {
	type plain authoringNavigationEnvelope
	var next plain
	if err := decodeTransportObject(data, &next, []string{"navigation", "alternativeHandles"}); err != nil {
		return err
	}
	if len(next.AlternativeHandles) != len(next.Navigation.Alternatives) {
		return fmt.Errorf("authoring alternative custody count differs")
	}
	*value = authoringNavigationEnvelope(next)
	return nil
}

// Decode the whole result before transferring any of its native identities.
func decodeAuthoringNavigationResponse(result response) (authoringNavigationEnvelope, error) {
	var envelope authoringNavigationEnvelope
	if err := json.Unmarshal(result.metadata, &envelope); err != nil {
		return authoringNavigationEnvelope{}, fmt.Errorf("invalid authoring navigation projection")
	}
	if (envelope.Navigation.Identity == nil) != (result.object == 0) {
		return authoringNavigationEnvelope{}, fmt.Errorf("authoring identity custody differs")
	}
	seen := make(map[uint64]bool, len(envelope.AlternativeHandles)+1)
	if result.object != 0 {
		seen[result.object] = true
	}
	for index, alternative := range envelope.AlternativeHandles {
		if alternative.AlternativeIndex != uint64(index) || alternative.Handle == 0 || seen[alternative.Handle] {
			return authoringNavigationEnvelope{}, fmt.Errorf("authoring alternative custody differs")
		}
		seen[alternative.Handle] = true
	}
	return envelope, nil
}

func navigateAuthoring(owner *handle, sourceLabel string, path []InstancePathSegment, invoke operation) (AuthoringNavigation, error) {
	if err := owner.valid(); err != nil {
		return AuthoringNavigation{}, err
	}
	var envelope authoringNavigationEnvelope
	result, err := owner.client.callDecoded(struct {
		Handle      uint64                `json:"handle"`
		SourceLabel string                `json:"sourceLabel"`
		Path        []InstancePathSegment `json:"path"`
	}{owner.id(), sourceLabel, path}, invoke, func(result response) error {
		var err error
		envelope, err = decodeAuthoringNavigationResponse(result)
		return err
	})
	if err != nil {
		return AuthoringNavigation{}, err
	}
	value := envelope.Navigation
	navigation := AuthoringNavigation{
		OccurrenceRole: value.OccurrenceRole, Definition: value.Definition, References: value.References,
		ReferencesTruncated: value.ReferencesTruncated, AlternativesTruncated: value.AlternativesTruncated,
		Alternatives:    make([]AuthoringIdentity, len(value.Alternatives)),
		ownedIdentities: make([]handle, 0, len(value.Alternatives)+1),
	}
	if value.Identity != nil {
		identity := newHandle(owner.client, result.object)
		navigation.Identity = &AuthoringIdentity{handle: identity, Description: *value.Identity}
		navigation.ownedIdentities = append(navigation.ownedIdentities, identity)
	}
	for index, description := range value.Alternatives {
		identity := newHandle(owner.client, envelope.AlternativeHandles[index].Handle)
		navigation.Alternatives[index] = AuthoringIdentity{handle: identity, Description: description}
		navigation.ownedIdentities = append(navigation.ownedIdentities, identity)
	}
	return navigation, nil
}
