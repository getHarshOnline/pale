package pale

// WorkspaceSubjectValueKind identifies the SDK-selected payload variant.
type WorkspaceSubjectValueKind string

const (
	WorkspaceSubjectValueSourceKind                           WorkspaceSubjectValueKind = "source"
	WorkspaceSubjectValueComponentKind                        WorkspaceSubjectValueKind = "component"
	WorkspaceSubjectValueModuleKind                           WorkspaceSubjectValueKind = "module"
	WorkspaceSubjectValueDeclarationKind                      WorkspaceSubjectValueKind = "declaration"
	WorkspaceSubjectValueImportKind                           WorkspaceSubjectValueKind = "import"
	WorkspaceSubjectValueExtensionKind                        WorkspaceSubjectValueKind = "extension"
	WorkspaceSubjectValuePolicyKind                           WorkspaceSubjectValueKind = "policy"
	WorkspaceSubjectValueResponsibilityClosureKind            WorkspaceSubjectValueKind = "responsibility-closure"
	WorkspaceSubjectValueOrganizationalTranslationClosureKind WorkspaceSubjectValueKind = "organizational-translation-closure"
	WorkspaceSubjectValueExperienceCompositionKind            WorkspaceSubjectValueKind = "experience-composition"
	WorkspaceSubjectValueResourceKind                         WorkspaceSubjectValueKind = "resource"
	WorkspaceSubjectValueResolutionLockKind                   WorkspaceSubjectValueKind = "resolution-lock"
	WorkspaceSubjectValueUniverseLockKind                     WorkspaceSubjectValueKind = "universe-lock"
	WorkspaceSubjectValueWorkspaceKind                        WorkspaceSubjectValueKind = "workspace"
)

// WorkspaceSubjectValue retains one SDK-selected payload. Decoding establishes the
// kind/payload relationship; editing these copied fields grants no authority.
type WorkspaceSubjectValue struct {
	Kind WorkspaceSubjectValueKind

	Source      *WorkspaceSubjectValueSource
	Component   *WorkspaceSubjectValueComponent
	Module      *WorkspaceSubjectValueModule
	Declaration *WorkspaceSubjectValueDeclaration
	Import      *WorkspaceSubjectValueImport
	Extension   *WorkspaceSubjectValueExtension
	Policy      *WorkspaceSubjectValuePolicy
	Resource    *WorkspaceSubjectValueResource
	Workspace   *WorkspaceSubjectValueWorkspace
}

// WorkspaceSubjectValueSource is the source payload.
type WorkspaceSubjectValueSource struct {
	SourceID string `json:"sourceId"`
}

// WorkspaceSubjectValueComponent is the component payload.
type WorkspaceSubjectValueComponent struct {
	ComponentID string `json:"componentId"`
}

// WorkspaceSubjectValueModule is the module payload.
type WorkspaceSubjectValueModule struct {
	ModuleID string `json:"moduleId"`
}

// WorkspaceSubjectValueDeclaration is the declaration payload.
type WorkspaceSubjectValueDeclaration struct {
	OwnerModuleID string `json:"ownerModuleId"`
	NamespaceID   string `json:"namespaceId"`
	DeclarationID string `json:"declarationId"`
}

// WorkspaceSubjectValueImport is the import payload.
type WorkspaceSubjectValueImport struct {
	ImporterModuleID  string `json:"importerModuleId"`
	UpstreamModuleID  string `json:"upstreamModuleId"`
	UpstreamReleaseID string `json:"upstreamReleaseId"`
	ExportID          string `json:"exportId"`
	SourceNamespaceID string `json:"sourceNamespaceId"`
	LocalNamespaceID  string `json:"localNamespaceId"`
}

// WorkspaceSubjectValueExtension is the extension payload.
type WorkspaceSubjectValueExtension struct {
	OwnerModuleID        string `json:"ownerModuleId"`
	ExtensionNamespaceID string `json:"extensionNamespaceId"`
	TargetNamespaceID    string `json:"targetNamespaceId"`
	TargetDeclarationID  string `json:"targetDeclarationId"`
}

// WorkspaceSubjectValuePolicy is the policy payload.
type WorkspaceSubjectValuePolicy struct {
	OwnerModuleID string `json:"ownerModuleId"`
	NamespaceID   string `json:"namespaceId"`
	DeclarationID string `json:"declarationId"`
}

// WorkspaceSubjectValueResource is the resource payload.
type WorkspaceSubjectValueResource struct {
	OwnerModuleID string `json:"ownerModuleId"`
	ResourceID    string `json:"resourceId"`
}

// WorkspaceSubjectValueWorkspace is the workspace payload.
type WorkspaceSubjectValueWorkspace struct {
	WorkspaceID string `json:"workspaceId"`
}

// UnmarshalJSON preserves order, exact field spelling, and atomic failure.
func (value *WorkspaceSubjectValue) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"source":                             {"sourceId"},
		"component":                          {"componentId"},
		"module":                             {"moduleId"},
		"declaration":                        {"ownerModuleId", "namespaceId", "declarationId"},
		"import":                             {"importerModuleId", "upstreamModuleId", "upstreamReleaseId", "exportId", "sourceNamespaceId", "localNamespaceId"},
		"extension":                          {"ownerModuleId", "extensionNamespaceId", "targetNamespaceId", "targetDeclarationId"},
		"policy":                             {"ownerModuleId", "namespaceId", "declarationId"},
		"responsibility-closure":             {},
		"organizational-translation-closure": {},
		"experience-composition":             {},
		"resource":                           {"ownerModuleId", "resourceId"},
		"resolution-lock":                    {},
		"universe-lock":                      {},
		"workspace":                          {"workspaceId"},
	})
	if err != nil {
		return err
	}
	next := WorkspaceSubjectValue{Kind: WorkspaceSubjectValueKind(kind)}
	switch kind {
	case "source":
		var wire struct {
			Kind string `json:"kind"`

			WorkspaceSubjectValueSource
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "sourceId"}); err != nil {
			return err
		}

		next.Source = &wire.WorkspaceSubjectValueSource
	case "component":
		var wire struct {
			Kind string `json:"kind"`

			WorkspaceSubjectValueComponent
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "componentId"}); err != nil {
			return err
		}

		next.Component = &wire.WorkspaceSubjectValueComponent
	case "module":
		var wire struct {
			Kind string `json:"kind"`

			WorkspaceSubjectValueModule
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "moduleId"}); err != nil {
			return err
		}

		next.Module = &wire.WorkspaceSubjectValueModule
	case "declaration":
		var wire struct {
			Kind string `json:"kind"`

			WorkspaceSubjectValueDeclaration
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "ownerModuleId", "namespaceId", "declarationId"}); err != nil {
			return err
		}

		next.Declaration = &wire.WorkspaceSubjectValueDeclaration
	case "import":
		var wire struct {
			Kind string `json:"kind"`

			WorkspaceSubjectValueImport
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "importerModuleId", "upstreamModuleId", "upstreamReleaseId", "exportId", "sourceNamespaceId", "localNamespaceId"}); err != nil {
			return err
		}

		next.Import = &wire.WorkspaceSubjectValueImport
	case "extension":
		var wire struct {
			Kind string `json:"kind"`

			WorkspaceSubjectValueExtension
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "ownerModuleId", "extensionNamespaceId", "targetNamespaceId", "targetDeclarationId"}); err != nil {
			return err
		}

		next.Extension = &wire.WorkspaceSubjectValueExtension
	case "policy":
		var wire struct {
			Kind string `json:"kind"`

			WorkspaceSubjectValuePolicy
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "ownerModuleId", "namespaceId", "declarationId"}); err != nil {
			return err
		}

		next.Policy = &wire.WorkspaceSubjectValuePolicy
	case "resource":
		var wire struct {
			Kind string `json:"kind"`

			WorkspaceSubjectValueResource
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "ownerModuleId", "resourceId"}); err != nil {
			return err
		}

		next.Resource = &wire.WorkspaceSubjectValueResource
	case "workspace":
		var wire struct {
			Kind string `json:"kind"`

			WorkspaceSubjectValueWorkspace
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "workspaceId"}); err != nil {
			return err
		}

		next.Workspace = &wire.WorkspaceSubjectValueWorkspace
	}
	*value = next
	return nil
}
