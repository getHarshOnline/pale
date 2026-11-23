package pale

// EffectiveProfileResourceKind is a copied SDK classification, not a new decision.
type EffectiveProfileResourceKind string

const (
	EffectiveProfileResourceKindOperation     EffectiveProfileResourceKind = "operation"
	EffectiveProfileResourceKindRole          EffectiveProfileResourceKind = "role"
	EffectiveProfileResourceKindDoctrine      EffectiveProfileResourceKind = "doctrine"
	EffectiveProfileResourceKindStartupPolicy EffectiveProfileResourceKind = "startup-policy"
	EffectiveProfileResourceKindRunMode       EffectiveProfileResourceKind = "run-mode"
	EffectiveProfileResourceKindStack         EffectiveProfileResourceKind = "stack"
	EffectiveProfileResourceKindPolicyFamily  EffectiveProfileResourceKind = "policy-family"
	EffectiveProfileResourceKindRecipe        EffectiveProfileResourceKind = "recipe"
)

// UnmarshalJSON rejects unknown transport tags.
func (value *EffectiveProfileResourceKind) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, EffectiveProfileResourceKindOperation, EffectiveProfileResourceKindRole, EffectiveProfileResourceKindDoctrine, EffectiveProfileResourceKindStartupPolicy, EffectiveProfileResourceKindRunMode, EffectiveProfileResourceKindStack, EffectiveProfileResourceKindPolicyFamily, EffectiveProfileResourceKindRecipe)
}

// EffectiveProfileModeAxis is a copied SDK classification, not a new decision.
type EffectiveProfileModeAxis string

const (
	EffectiveProfileModeAxisIntensity  EffectiveProfileModeAxis = "intensity"
	EffectiveProfileModeAxisTaskStance EffectiveProfileModeAxis = "task-stance"
)

// UnmarshalJSON rejects unknown transport tags.
func (value *EffectiveProfileModeAxis) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, EffectiveProfileModeAxisIntensity, EffectiveProfileModeAxisTaskStance)
}

// EffectiveProfileRepositoryAccess is a copied SDK classification, not a new decision.
type EffectiveProfileRepositoryAccess string

const (
	EffectiveProfileRepositoryAccessRead   EffectiveProfileRepositoryAccess = "read"
	EffectiveProfileRepositoryAccessMutate EffectiveProfileRepositoryAccess = "mutate"
)

// UnmarshalJSON rejects unknown transport tags.
func (value *EffectiveProfileRepositoryAccess) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, EffectiveProfileRepositoryAccessRead, EffectiveProfileRepositoryAccessMutate)
}

// EffectiveProfileEffectClass is a copied SDK classification, not a new decision.
type EffectiveProfileEffectClass string

const (
	EffectiveProfileEffectClassCompute EffectiveProfileEffectClass = "compute"
	EffectiveProfileEffectClassRead    EffectiveProfileEffectClass = "read"
	EffectiveProfileEffectClassExecute EffectiveProfileEffectClass = "execute"
	EffectiveProfileEffectClassWrite   EffectiveProfileEffectClass = "write"
	EffectiveProfileEffectClassNetwork EffectiveProfileEffectClass = "network"
	EffectiveProfileEffectClassPublish EffectiveProfileEffectClass = "publish"
)

// UnmarshalJSON rejects unknown transport tags.
func (value *EffectiveProfileEffectClass) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, EffectiveProfileEffectClassCompute, EffectiveProfileEffectClassRead, EffectiveProfileEffectClassExecute, EffectiveProfileEffectClassWrite, EffectiveProfileEffectClassNetwork, EffectiveProfileEffectClassPublish)
}

// EffectiveProfileProviderKind is a copied SDK classification, not a new decision.
type EffectiveProfileProviderKind string

const (
	EffectiveProfileProviderKindHostNative EffectiveProfileProviderKind = "host-native"
	EffectiveProfileProviderKindNativeRust EffectiveProfileProviderKind = "native-rust"
	EffectiveProfileProviderKindConnector  EffectiveProfileProviderKind = "connector"
)

// UnmarshalJSON rejects unknown transport tags.
func (value *EffectiveProfileProviderKind) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, EffectiveProfileProviderKindHostNative, EffectiveProfileProviderKindNativeRust, EffectiveProfileProviderKindConnector)
}

// EffectiveProfileNativeRealization is a copied SDK classification, not a new decision.
type EffectiveProfileNativeRealization string

const (
	EffectiveProfileNativeRealizationUnknown EffectiveProfileNativeRealization = "unknown"
)

// UnmarshalJSON rejects unknown transport tags.
func (value *EffectiveProfileNativeRealization) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, EffectiveProfileNativeRealizationUnknown)
}

// EffectiveProfileProviderBindingRef copies the SDK-owned fields without deriving state.
type EffectiveProfileProviderBindingRef struct {
	ProviderBindingID       string `json:"providerBindingId"`
	ProviderBindingIdentity string `json:"providerBindingIdentity"`
}

// UnmarshalJSON checks exact fields and commits only a complete result.
func (value *EffectiveProfileProviderBindingRef) UnmarshalJSON(data []byte) error {
	type plain EffectiveProfileProviderBindingRef
	var next plain
	if err := decodeTransportObject(data, &next, []string{"providerBindingId", "providerBindingIdentity"}); err != nil {
		return err
	}
	*value = EffectiveProfileProviderBindingRef(next)
	return nil
}

// EffectiveProfileDeclaredSelectorKind identifies the SDK-selected payload variant.
type EffectiveProfileDeclaredSelectorKind string

const (
	EffectiveProfileDeclaredSelectorExactKind          EffectiveProfileDeclaredSelectorKind = "exact"
	EffectiveProfileDeclaredSelectorAllFromPackageKind EffectiveProfileDeclaredSelectorKind = "all-from-package"
)

// EffectiveProfileDeclaredSelector retains one SDK-selected payload. Decoding establishes the
// kind/payload relationship; editing these copied fields grants no authority.
type EffectiveProfileDeclaredSelector struct {
	Kind EffectiveProfileDeclaredSelectorKind

	Exact          *EffectiveProfileDeclaredSelectorExact
	AllFromPackage *EffectiveProfileDeclaredSelectorAllFromPackage
}

// EffectiveProfileDeclaredSelectorExact is the exact payload.
type EffectiveProfileDeclaredSelectorExact struct {
	ResourceID string `json:"resourceId"`
}

// EffectiveProfileDeclaredSelectorAllFromPackage is the all-from-package payload.
type EffectiveProfileDeclaredSelectorAllFromPackage struct {
	PackageID string `json:"packageId"`
}

// UnmarshalJSON preserves order, exact field spelling, and atomic failure.
func (value *EffectiveProfileDeclaredSelector) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"exact":            {"resourceId"},
		"all-from-package": {"packageId"},
	})
	if err != nil {
		return err
	}
	next := EffectiveProfileDeclaredSelector{Kind: EffectiveProfileDeclaredSelectorKind(kind)}
	switch kind {
	case "exact":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileDeclaredSelectorExact
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "resourceId"}); err != nil {
			return err
		}

		next.Exact = &wire.EffectiveProfileDeclaredSelectorExact
	case "all-from-package":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileDeclaredSelectorAllFromPackage
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "packageId"}); err != nil {
			return err
		}

		next.AllFromPackage = &wire.EffectiveProfileDeclaredSelectorAllFromPackage
	}
	*value = next
	return nil
}

// EffectiveProfileDeclaredSelectionKind identifies the SDK-selected payload variant.
type EffectiveProfileDeclaredSelectionKind string

const (
	EffectiveProfileDeclaredSelectionResourceKind    EffectiveProfileDeclaredSelectionKind = "resource"
	EffectiveProfileDeclaredSelectionModeDefaultKind EffectiveProfileDeclaredSelectionKind = "mode-default"
)

// EffectiveProfileDeclaredSelection retains one SDK-selected payload. Decoding establishes the
// kind/payload relationship; editing these copied fields grants no authority.
type EffectiveProfileDeclaredSelection struct {
	Kind EffectiveProfileDeclaredSelectionKind

	Resource    *EffectiveProfileDeclaredSelectionResource
	ModeDefault *EffectiveProfileDeclaredSelectionModeDefault
}

// EffectiveProfileDeclaredSelectionResource is the resource payload.
type EffectiveProfileDeclaredSelectionResource struct {
	ResourceKind EffectiveProfileResourceKind     `json:"resourceKind"`
	Selector     EffectiveProfileDeclaredSelector `json:"selector"`
}

// EffectiveProfileDeclaredSelectionModeDefault is the mode-default payload.
type EffectiveProfileDeclaredSelectionModeDefault struct {
	Axis       EffectiveProfileModeAxis `json:"axis"`
	ResourceID string                   `json:"resourceId"`
}

// UnmarshalJSON preserves order, exact field spelling, and atomic failure.
func (value *EffectiveProfileDeclaredSelection) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"resource":     {"resourceKind", "selector"},
		"mode-default": {"axis", "resourceId"},
	})
	if err != nil {
		return err
	}
	next := EffectiveProfileDeclaredSelection{Kind: EffectiveProfileDeclaredSelectionKind(kind)}
	switch kind {
	case "resource":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileDeclaredSelectionResource
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "resourceKind", "selector"}); err != nil {
			return err
		}

		next.Resource = &wire.EffectiveProfileDeclaredSelectionResource
	case "mode-default":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileDeclaredSelectionModeDefault
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "axis", "resourceId"}); err != nil {
			return err
		}

		next.ModeDefault = &wire.EffectiveProfileDeclaredSelectionModeDefault
	}
	*value = next
	return nil
}

// EffectiveProfileSelectionKind identifies the SDK-selected payload variant.
type EffectiveProfileSelectionKind string

const (
	EffectiveProfileSelectionPackageKind           EffectiveProfileSelectionKind = "package"
	EffectiveProfileSelectionNormativePackKind     EffectiveProfileSelectionKind = "normative-pack"
	EffectiveProfileSelectionSemanticResourceKind  EffectiveProfileSelectionKind = "semantic-resource"
	EffectiveProfileSelectionRunPolicyKind         EffectiveProfileSelectionKind = "run-policy"
	EffectiveProfileSelectionSessionKind           EffectiveProfileSelectionKind = "session"
	EffectiveProfileSelectionLifecycleKind         EffectiveProfileSelectionKind = "lifecycle"
	EffectiveProfileSelectionAuthorityRootKind     EffectiveProfileSelectionKind = "authority-root"
	EffectiveProfileSelectionDisclosureCeilingKind EffectiveProfileSelectionKind = "disclosure-ceiling"
)

// EffectiveProfileSelection retains one SDK-selected payload. Decoding establishes the
// kind/payload relationship; editing these copied fields grants no authority.
type EffectiveProfileSelection struct {
	Kind EffectiveProfileSelectionKind

	Package           *EffectiveProfileSelectionPackage
	NormativePack     *EffectiveProfileSelectionNormativePack
	SemanticResource  *EffectiveProfileSelectionSemanticResource
	RunPolicy         *EffectiveProfileSelectionRunPolicy
	Session           *EffectiveProfileSelectionSession
	Lifecycle         *EffectiveProfileSelectionLifecycle
	AuthorityRoot     *EffectiveProfileSelectionAuthorityRoot
	DisclosureCeiling *EffectiveProfileSelectionDisclosureCeiling
}

// EffectiveProfileSelectionPackage is the package payload.
type EffectiveProfileSelectionPackage struct {
	PackageID     string `json:"packageId"`
	Version       string `json:"version"`
	ContentDigest string `json:"contentDigest"`
}

// EffectiveProfileSelectionNormativePack is the normative-pack payload.
type EffectiveProfileSelectionNormativePack struct {
	PackID        string `json:"packId"`
	AttestorID    string `json:"attestorId"`
	Version       string `json:"version"`
	ContentDigest string `json:"contentDigest"`
}

// EffectiveProfileSelectionSemanticResource is the semantic-resource payload.
type EffectiveProfileSelectionSemanticResource struct {
	RepositoryID string                       `json:"repositoryId"`
	ResourceKind EffectiveProfileResourceKind `json:"resourceKind"`
	ResourceID   string                       `json:"resourceId"`
}

// EffectiveProfileSelectionRunPolicy is the run-policy payload.
type EffectiveProfileSelectionRunPolicy struct {
	RunPolicyID string `json:"runPolicyId"`
}

// EffectiveProfileSelectionSession is the session payload.
type EffectiveProfileSelectionSession struct {
	ExecutionWorkspaceID string `json:"executionWorkspaceId"`
	ActivationRootCount  uint32 `json:"activationRootCount"`
	AdditionalRootCount  uint32 `json:"additionalRootCount"`
	ActivationPathCount  uint32 `json:"activationPathCount"`
}

// EffectiveProfileSelectionLifecycle is the lifecycle payload.
type EffectiveProfileSelectionLifecycle struct {
	LifecycleID string `json:"lifecycleId"`
	EventID     string `json:"eventId"`
	ActionID    string `json:"actionId"`
}

// EffectiveProfileSelectionAuthorityRoot is the authority-root payload.
type EffectiveProfileSelectionAuthorityRoot struct {
	AuthorityRootID string `json:"authorityRootId"`
}

// EffectiveProfileSelectionDisclosureCeiling is the disclosure-ceiling payload.
type EffectiveProfileSelectionDisclosureCeiling struct {
	AuthorityCoordinate string `json:"authorityCoordinate"`
}

// UnmarshalJSON preserves order, exact field spelling, and atomic failure.
func (value *EffectiveProfileSelection) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"package":            {"packageId", "version", "contentDigest"},
		"normative-pack":     {"packId", "attestorId", "version", "contentDigest"},
		"semantic-resource":  {"repositoryId", "resourceKind", "resourceId"},
		"run-policy":         {"runPolicyId"},
		"session":            {"executionWorkspaceId", "activationRootCount", "additionalRootCount", "activationPathCount"},
		"lifecycle":          {"lifecycleId", "eventId", "actionId"},
		"authority-root":     {"authorityRootId"},
		"disclosure-ceiling": {"authorityCoordinate"},
	})
	if err != nil {
		return err
	}
	next := EffectiveProfileSelection{Kind: EffectiveProfileSelectionKind(kind)}
	switch kind {
	case "package":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileSelectionPackage
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "packageId", "version", "contentDigest"}); err != nil {
			return err
		}

		next.Package = &wire.EffectiveProfileSelectionPackage
	case "normative-pack":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileSelectionNormativePack
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "packId", "attestorId", "version", "contentDigest"}); err != nil {
			return err
		}

		next.NormativePack = &wire.EffectiveProfileSelectionNormativePack
	case "semantic-resource":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileSelectionSemanticResource
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "repositoryId", "resourceKind", "resourceId"}); err != nil {
			return err
		}

		next.SemanticResource = &wire.EffectiveProfileSelectionSemanticResource
	case "run-policy":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileSelectionRunPolicy
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "runPolicyId"}); err != nil {
			return err
		}

		next.RunPolicy = &wire.EffectiveProfileSelectionRunPolicy
	case "session":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileSelectionSession
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "executionWorkspaceId", "activationRootCount", "additionalRootCount", "activationPathCount"}); err != nil {
			return err
		}

		next.Session = &wire.EffectiveProfileSelectionSession
	case "lifecycle":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileSelectionLifecycle
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "lifecycleId", "eventId", "actionId"}); err != nil {
			return err
		}

		next.Lifecycle = &wire.EffectiveProfileSelectionLifecycle
	case "authority-root":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileSelectionAuthorityRoot
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "authorityRootId"}); err != nil {
			return err
		}

		next.AuthorityRoot = &wire.EffectiveProfileSelectionAuthorityRoot
	case "disclosure-ceiling":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileSelectionDisclosureCeiling
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "authorityCoordinate"}); err != nil {
			return err
		}

		next.DisclosureCeiling = &wire.EffectiveProfileSelectionDisclosureCeiling
	}
	*value = next
	return nil
}

// EffectiveProfileParticipantKind identifies the SDK-selected payload variant.
type EffectiveProfileParticipantKind string

const (
	EffectiveProfileParticipantInterfaceKind    EffectiveProfileParticipantKind = "interface"
	EffectiveProfileParticipantApplicationKind  EffectiveProfileParticipantKind = "application"
	EffectiveProfileParticipantEngineKind       EffectiveProfileParticipantKind = "engine"
	EffectiveProfileParticipantCapabilityKind   EffectiveProfileParticipantKind = "capability"
	EffectiveProfileParticipantTerminalLeafKind EffectiveProfileParticipantKind = "terminal-leaf"
	EffectiveProfileParticipantPluginKind       EffectiveProfileParticipantKind = "plugin"
	EffectiveProfileParticipantSkillKind        EffectiveProfileParticipantKind = "skill"
)

// EffectiveProfileParticipant retains one SDK-selected payload. Decoding establishes the
// kind/payload relationship; editing these copied fields grants no authority.
type EffectiveProfileParticipant struct {
	Kind EffectiveProfileParticipantKind

	Interface    *EffectiveProfileParticipantInterface
	Application  *EffectiveProfileParticipantApplication
	Engine       *EffectiveProfileParticipantEngine
	Capability   *EffectiveProfileParticipantCapability
	TerminalLeaf *EffectiveProfileParticipantTerminalLeaf
	Plugin       *EffectiveProfileParticipantPlugin
	Skill        *EffectiveProfileParticipantSkill
}

// EffectiveProfileParticipantInterface is the interface payload.
type EffectiveProfileParticipantInterface struct {
	InterfaceID string `json:"interfaceId"`
}

// EffectiveProfileParticipantApplication is the application payload.
type EffectiveProfileParticipantApplication struct {
	ApplicationID string `json:"applicationId"`
}

// EffectiveProfileParticipantEngine is the engine payload.
type EffectiveProfileParticipantEngine struct {
	EngineID string `json:"engineId"`
}

// EffectiveProfileParticipantCapability is the capability payload.
type EffectiveProfileParticipantCapability struct {
	CapabilityID string `json:"capabilityId"`
}

// EffectiveProfileParticipantTerminalLeaf is the terminal-leaf payload.
type EffectiveProfileParticipantTerminalLeaf struct {
	TerminalLeafID string `json:"terminalLeafId"`
}

// EffectiveProfileParticipantPlugin is the plugin payload.
type EffectiveProfileParticipantPlugin struct {
	PluginID string `json:"pluginId"`
}

// EffectiveProfileParticipantSkill is the skill payload.
type EffectiveProfileParticipantSkill struct {
	SkillID string `json:"skillId"`
}

// UnmarshalJSON preserves order, exact field spelling, and atomic failure.
func (value *EffectiveProfileParticipant) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"interface":     {"interfaceId"},
		"application":   {"applicationId"},
		"engine":        {"engineId"},
		"capability":    {"capabilityId"},
		"terminal-leaf": {"terminalLeafId"},
		"plugin":        {"pluginId"},
		"skill":         {"skillId"},
	})
	if err != nil {
		return err
	}
	next := EffectiveProfileParticipant{Kind: EffectiveProfileParticipantKind(kind)}
	switch kind {
	case "interface":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileParticipantInterface
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "interfaceId"}); err != nil {
			return err
		}

		next.Interface = &wire.EffectiveProfileParticipantInterface
	case "application":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileParticipantApplication
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "applicationId"}); err != nil {
			return err
		}

		next.Application = &wire.EffectiveProfileParticipantApplication
	case "engine":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileParticipantEngine
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "engineId"}); err != nil {
			return err
		}

		next.Engine = &wire.EffectiveProfileParticipantEngine
	case "capability":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileParticipantCapability
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "capabilityId"}); err != nil {
			return err
		}

		next.Capability = &wire.EffectiveProfileParticipantCapability
	case "terminal-leaf":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileParticipantTerminalLeaf
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "terminalLeafId"}); err != nil {
			return err
		}

		next.TerminalLeaf = &wire.EffectiveProfileParticipantTerminalLeaf
	case "plugin":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileParticipantPlugin
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "pluginId"}); err != nil {
			return err
		}

		next.Plugin = &wire.EffectiveProfileParticipantPlugin
	case "skill":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileParticipantSkill
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "skillId"}); err != nil {
			return err
		}

		next.Skill = &wire.EffectiveProfileParticipantSkill
	}
	*value = next
	return nil
}

// EffectiveProfileResponsibilityKind identifies the SDK-selected payload variant.
type EffectiveProfileResponsibilityKind string

const (
	EffectiveProfileResponsibilityCommunicationKind            EffectiveProfileResponsibilityKind = "communication"
	EffectiveProfileResponsibilityOrganizationalWorkKind       EffectiveProfileResponsibilityKind = "organizational-work"
	EffectiveProfileResponsibilityCrmBusinessStateKind         EffectiveProfileResponsibilityKind = "crm-business-state"
	EffectiveProfileResponsibilityEngineeringExecutionKind     EffectiveProfileResponsibilityKind = "engineering-execution"
	EffectiveProfileResponsibilityContextMemoryKind            EffectiveProfileResponsibilityKind = "context-memory"
	EffectiveProfileResponsibilityTerminalHarnessExecutionKind EffectiveProfileResponsibilityKind = "terminal-harness-execution"
	EffectiveProfileResponsibilityNativeEventPublicationKind   EffectiveProfileResponsibilityKind = "native-event-publication"
	EffectiveProfileResponsibilityNativeEffectReadBackKind     EffectiveProfileResponsibilityKind = "native-effect-read-back"
	EffectiveProfileResponsibilityExtensionKind                EffectiveProfileResponsibilityKind = "extension"
)

// EffectiveProfileResponsibility retains one SDK-selected payload. Decoding establishes the
// kind/payload relationship; editing these copied fields grants no authority.
type EffectiveProfileResponsibility struct {
	Kind EffectiveProfileResponsibilityKind

	Extension *EffectiveProfileResponsibilityExtension
}

// EffectiveProfileResponsibilityExtension is the extension payload.
type EffectiveProfileResponsibilityExtension struct {
	NamespaceID  string `json:"namespaceId"`
	ContractID   string `json:"contractId"`
	SchemaID     string `json:"schemaId"`
	SchemaSHA256 string `json:"schemaSha256"`
}

// UnmarshalJSON preserves order, exact field spelling, and atomic failure.
func (value *EffectiveProfileResponsibility) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"communication":              {},
		"organizational-work":        {},
		"crm-business-state":         {},
		"engineering-execution":      {},
		"context-memory":             {},
		"terminal-harness-execution": {},
		"native-event-publication":   {},
		"native-effect-read-back":    {},
		"extension":                  {"namespaceId", "contractId", "schemaId", "schemaSha256"},
	})
	if err != nil {
		return err
	}
	next := EffectiveProfileResponsibility{Kind: EffectiveProfileResponsibilityKind(kind)}
	switch kind {
	case "extension":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileResponsibilityExtension
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "namespaceId", "contractId", "schemaId", "schemaSha256"}); err != nil {
			return err
		}

		next.Extension = &wire.EffectiveProfileResponsibilityExtension
	}
	*value = next
	return nil
}

// EffectiveProfileProviderDispositionKind identifies the SDK-selected payload variant.
type EffectiveProfileProviderDispositionKind string

const (
	EffectiveProfileProviderDispositionBoundKind                  EffectiveProfileProviderDispositionKind = "bound"
	EffectiveProfileProviderDispositionMissingBindingKind         EffectiveProfileProviderDispositionKind = "missing-binding"
	EffectiveProfileProviderDispositionExcludedByNarrowingKind    EffectiveProfileProviderDispositionKind = "excluded-by-narrowing"
	EffectiveProfileProviderDispositionEffectMismatchKind         EffectiveProfileProviderDispositionKind = "effect-mismatch"
	EffectiveProfileProviderDispositionRecipeContractMismatchKind EffectiveProfileProviderDispositionKind = "recipe-contract-mismatch"
)

// EffectiveProfileProviderDisposition retains one SDK-selected payload. Decoding establishes the
// kind/payload relationship; editing these copied fields grants no authority.
type EffectiveProfileProviderDisposition struct {
	Kind EffectiveProfileProviderDispositionKind

	Bound *EffectiveProfileProviderDispositionBound
}

// EffectiveProfileProviderDispositionBound is the bound payload.
type EffectiveProfileProviderDispositionBound struct {
	Provider EffectiveProfileProviderKind `json:"provider"`
}

// UnmarshalJSON preserves order, exact field spelling, and atomic failure.
func (value *EffectiveProfileProviderDisposition) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"bound":                    {"provider"},
		"missing-binding":          {},
		"excluded-by-narrowing":    {},
		"effect-mismatch":          {},
		"recipe-contract-mismatch": {},
	})
	if err != nil {
		return err
	}
	next := EffectiveProfileProviderDisposition{Kind: EffectiveProfileProviderDispositionKind(kind)}
	switch kind {
	case "bound":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileProviderDispositionBound
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "provider"}); err != nil {
			return err
		}

		next.Bound = &wire.EffectiveProfileProviderDispositionBound
	}
	*value = next
	return nil
}

// EffectiveProfileFindingKind identifies the SDK-selected payload variant.
type EffectiveProfileFindingKind string

const (
	EffectiveProfileFindingNativeRealizationUnknownKind EffectiveProfileFindingKind = "native-realization-unknown"
	EffectiveProfileFindingOperationUnboundKind         EffectiveProfileFindingKind = "operation-unbound"
	EffectiveProfileFindingRequirementKind              EffectiveProfileFindingKind = "requirement"
)

// EffectiveProfileFinding retains one SDK-selected payload. Decoding establishes the
// kind/payload relationship; editing these copied fields grants no authority.
type EffectiveProfileFinding struct {
	Kind EffectiveProfileFindingKind

	NativeRealizationUnknown *EffectiveProfileFindingNativeRealizationUnknown
	OperationUnbound         *EffectiveProfileFindingOperationUnbound
	Requirement              *EffectiveProfileFindingRequirement
}

// EffectiveProfileFindingNativeRealizationUnknown is the native-realization-unknown payload.
type EffectiveProfileFindingNativeRealizationUnknown struct {
	MachineID string `json:"machineId"`
}

// EffectiveProfileFindingOperationUnbound is the operation-unbound payload.
type EffectiveProfileFindingOperationUnbound struct {
	RepositoryID string `json:"repositoryId"`
	OperationID  string `json:"operationId"`
}

// EffectiveProfileFindingRequirement is the requirement payload.
type EffectiveProfileFindingRequirement struct {
	RepositoryID  string                                 `json:"repositoryId"`
	OperationID   string                                 `json:"operationId"`
	RequirementID string                                 `json:"requirementId"`
	CapabilityID  string                                 `json:"capabilityId"`
	Disposition   EffectiveProfileRequirementDisposition `json:"disposition"`
}

// UnmarshalJSON preserves order, exact field spelling, and atomic failure.
func (value *EffectiveProfileFinding) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"native-realization-unknown": {"machineId"},
		"operation-unbound":          {"repositoryId", "operationId"},
		"requirement":                {"repositoryId", "operationId", "requirementId", "capabilityId", "disposition"},
	})
	if err != nil {
		return err
	}
	next := EffectiveProfileFinding{Kind: EffectiveProfileFindingKind(kind)}
	switch kind {
	case "native-realization-unknown":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileFindingNativeRealizationUnknown
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "machineId"}); err != nil {
			return err
		}

		next.NativeRealizationUnknown = &wire.EffectiveProfileFindingNativeRealizationUnknown
	case "operation-unbound":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileFindingOperationUnbound
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "repositoryId", "operationId"}); err != nil {
			return err
		}

		next.OperationUnbound = &wire.EffectiveProfileFindingOperationUnbound
	case "requirement":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfileFindingRequirement
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "repositoryId", "operationId", "requirementId", "capabilityId", "disposition"}); err != nil {
			return err
		}

		next.Requirement = &wire.EffectiveProfileFindingRequirement
	}
	*value = next
	return nil
}

// EffectiveProfilePathReasonKind identifies the SDK-selected payload variant.
type EffectiveProfilePathReasonKind string

const (
	EffectiveProfilePathReasonOriginReceiptKind          EffectiveProfilePathReasonKind = "origin-receipt"
	EffectiveProfilePathReasonEntryContractKind          EffectiveProfilePathReasonKind = "entry-contract"
	EffectiveProfilePathReasonResponsibilityContractKind EffectiveProfilePathReasonKind = "responsibility-contract"
	EffectiveProfilePathReasonProviderBindingKind        EffectiveProfilePathReasonKind = "provider-binding"
	EffectiveProfilePathReasonDelegationEdgeKind         EffectiveProfilePathReasonKind = "delegation-edge"
	EffectiveProfilePathReasonTerminationContractKind    EffectiveProfilePathReasonKind = "termination-contract"
	EffectiveProfilePathReasonSettlementContractKind     EffectiveProfilePathReasonKind = "settlement-contract"
	EffectiveProfilePathReasonContinuityContractKind     EffectiveProfilePathReasonKind = "continuity-contract"
)

// EffectiveProfilePathReason retains one SDK-selected payload. Decoding establishes the
// kind/payload relationship; editing these copied fields grants no authority.
type EffectiveProfilePathReason struct {
	Kind EffectiveProfilePathReasonKind

	OriginReceipt          *EffectiveProfilePathReasonOriginReceipt
	EntryContract          *EffectiveProfilePathReasonEntryContract
	ResponsibilityContract *EffectiveProfilePathReasonResponsibilityContract
	ProviderBinding        *EffectiveProfilePathReasonProviderBinding
	DelegationEdge         *EffectiveProfilePathReasonDelegationEdge
	TerminationContract    *EffectiveProfilePathReasonTerminationContract
	SettlementContract     *EffectiveProfilePathReasonSettlementContract
	ContinuityContract     *EffectiveProfilePathReasonContinuityContract
}

// EffectiveProfilePathReasonOriginReceipt is the origin-receipt payload.
type EffectiveProfilePathReasonOriginReceipt struct {
	ContractID string `json:"contractId"`
}

// EffectiveProfilePathReasonEntryContract is the entry-contract payload.
type EffectiveProfilePathReasonEntryContract struct {
	ContractID string `json:"contractId"`
}

// EffectiveProfilePathReasonResponsibilityContract is the responsibility-contract payload.
type EffectiveProfilePathReasonResponsibilityContract struct {
	ContractID string `json:"contractId"`
}

// EffectiveProfilePathReasonProviderBinding is the provider-binding payload.
type EffectiveProfilePathReasonProviderBinding struct {
	BindingID string `json:"bindingId"`
}

// EffectiveProfilePathReasonDelegationEdge is the delegation-edge payload.
type EffectiveProfilePathReasonDelegationEdge struct {
	EdgeID string `json:"edgeId"`
}

// EffectiveProfilePathReasonTerminationContract is the termination-contract payload.
type EffectiveProfilePathReasonTerminationContract struct {
	ContractID string `json:"contractId"`
}

// EffectiveProfilePathReasonSettlementContract is the settlement-contract payload.
type EffectiveProfilePathReasonSettlementContract struct {
	ContractID string `json:"contractId"`
}

// EffectiveProfilePathReasonContinuityContract is the continuity-contract payload.
type EffectiveProfilePathReasonContinuityContract struct {
	ContractID string `json:"contractId"`
}

// UnmarshalJSON preserves order, exact field spelling, and atomic failure.
func (value *EffectiveProfilePathReason) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"origin-receipt":          {"contractId"},
		"entry-contract":          {"contractId"},
		"responsibility-contract": {"contractId"},
		"provider-binding":        {"bindingId"},
		"delegation-edge":         {"edgeId"},
		"termination-contract":    {"contractId"},
		"settlement-contract":     {"contractId"},
		"continuity-contract":     {"contractId"},
	})
	if err != nil {
		return err
	}
	next := EffectiveProfilePathReason{Kind: EffectiveProfilePathReasonKind(kind)}
	switch kind {
	case "origin-receipt":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfilePathReasonOriginReceipt
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "contractId"}); err != nil {
			return err
		}

		next.OriginReceipt = &wire.EffectiveProfilePathReasonOriginReceipt
	case "entry-contract":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfilePathReasonEntryContract
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "contractId"}); err != nil {
			return err
		}

		next.EntryContract = &wire.EffectiveProfilePathReasonEntryContract
	case "responsibility-contract":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfilePathReasonResponsibilityContract
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "contractId"}); err != nil {
			return err
		}

		next.ResponsibilityContract = &wire.EffectiveProfilePathReasonResponsibilityContract
	case "provider-binding":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfilePathReasonProviderBinding
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "bindingId"}); err != nil {
			return err
		}

		next.ProviderBinding = &wire.EffectiveProfilePathReasonProviderBinding
	case "delegation-edge":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfilePathReasonDelegationEdge
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "edgeId"}); err != nil {
			return err
		}

		next.DelegationEdge = &wire.EffectiveProfilePathReasonDelegationEdge
	case "termination-contract":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfilePathReasonTerminationContract
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "contractId"}); err != nil {
			return err
		}

		next.TerminationContract = &wire.EffectiveProfilePathReasonTerminationContract
	case "settlement-contract":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfilePathReasonSettlementContract
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "contractId"}); err != nil {
			return err
		}

		next.SettlementContract = &wire.EffectiveProfilePathReasonSettlementContract
	case "continuity-contract":
		var wire struct {
			Kind string `json:"kind"`

			EffectiveProfilePathReasonContinuityContract
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "contractId"}); err != nil {
			return err
		}

		next.ContinuityContract = &wire.EffectiveProfilePathReasonContinuityContract
	}
	*value = next
	return nil
}
