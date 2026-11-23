package pale

// WorkspaceFact is one compiler-selected value. It describes the selected output,
// not proof ancestry or impact. Editing a copied value grants no authority.
type WorkspaceFact struct {
	Value WorkspaceFactValue
}

// WorkspaceFactValue has fourteen closed payload alternatives. Use a type switch
// to inspect the selected output without flattening distinct digest meanings.
type WorkspaceFactValue interface {
	isWorkspaceFactValue()
}

// SourceContentWorkspaceFact identifies the bytes admitted from one source.
// SourcePath is an owner-relative label, not a host filesystem path. SourceDigest
// is the SHA-256 of those bytes; ByteLen counts bytes, not characters.
type SourceContentWorkspaceFact struct {
	SourcePath   string `json:"sourcePath"`
	SourceDigest string `json:"sourceDigest"`
	ByteLen      uint32 `json:"byteLen"`
}

func (SourceContentWorkspaceFact) isWorkspaceFactValue() {}

// ComponentWorkspaceFact identifies a semantic workspace component by its digest.
type ComponentWorkspaceFact struct {
	ComponentDigest string `json:"componentDigest"`
}

func (ComponentWorkspaceFact) isWorkspaceFactValue() {}

// ModuleWorkspaceFact identifies the selected reusable module by its digest.
type ModuleWorkspaceFact struct {
	ModuleDigest string `json:"moduleDigest"`
}

func (ModuleWorkspaceFact) isWorkspaceFactValue() {}

// DeclarationWorkspaceFact identifies the resolved value of a declaration.
type DeclarationWorkspaceFact struct {
	ValueDigest string `json:"valueDigest"`
}

func (DeclarationWorkspaceFact) isWorkspaceFactValue() {}

// ImportSurfaceWorkspaceFact identifies the resolved interface exposed by an import.
type ImportSurfaceWorkspaceFact struct {
	SurfaceDigest string `json:"surfaceDigest"`
}

func (ImportSurfaceWorkspaceFact) isWorkspaceFactValue() {}

// ExtensionWorkspaceFact identifies the resolved extension, including its target
// and added traits. It does not identify the resulting target declaration.
type ExtensionWorkspaceFact struct {
	ValueDigest string `json:"valueDigest"`
}

func (ExtensionWorkspaceFact) isWorkspaceFactValue() {}

// EffectivePolicyWorkspaceFact identifies the accumulated effective policy.
type EffectivePolicyWorkspaceFact struct {
	ValueDigest string `json:"valueDigest"`
}

func (EffectivePolicyWorkspaceFact) isWorkspaceFactValue() {}

// ResponsibilityClosureWorkspaceFact identifies the complete lowered responsibility set.
type ResponsibilityClosureWorkspaceFact struct {
	ClosureDigest string `json:"closureDigest"`
}

func (ResponsibilityClosureWorkspaceFact) isWorkspaceFactValue() {}

// OrganizationalTranslationClosureWorkspaceFact identifies the complete
// lowered organizational translation, when selected.
type OrganizationalTranslationClosureWorkspaceFact struct {
	ClosureDigest string `json:"closureDigest"`
}

func (OrganizationalTranslationClosureWorkspaceFact) isWorkspaceFactValue() {}

// ExperienceCompositionWorkspaceFact identifies the explicit neutral composition declarations.
type ExperienceCompositionWorkspaceFact struct {
	SourceDigest string `json:"sourceDigest"`
}

func (ExperienceCompositionWorkspaceFact) isWorkspaceFactValue() {}

// ResourceWorkspaceFact identifies a selected module resource's derived value.
type ResourceWorkspaceFact struct {
	ValueDigest string `json:"valueDigest"`
}

func (ResourceWorkspaceFact) isWorkspaceFactValue() {}

// ResolutionLockWorkspaceFact identifies the admitted resolution lock.
type ResolutionLockWorkspaceFact struct {
	ResolutionDigest string `json:"resolutionDigest"`
}

func (ResolutionLockWorkspaceFact) isWorkspaceFactValue() {}

// UniverseLockWorkspaceFact identifies the admitted workspace universe lock.
type UniverseLockWorkspaceFact struct {
	UniverseDigest string `json:"universeDigest"`
}

func (UniverseLockWorkspaceFact) isWorkspaceFactValue() {}

// SemanticIRWorkspaceFact identifies the compiled workspace semantic representation
// by its digest; it does not establish a running process.
type SemanticIRWorkspaceFact struct {
	WorkspaceSemanticIRDigest string `json:"workspaceSemanticIrDigest"`
}

func (SemanticIRWorkspaceFact) isWorkspaceFactValue() {}

// UnmarshalJSON validates the complete selected payload before replacing a value.
func (value *WorkspaceFact) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{
		"source_content":                     {"sourcePath", "sourceDigest", "byteLen"},
		"component":                          {"componentDigest"},
		"module":                             {"moduleDigest"},
		"declaration":                        {"valueDigest"},
		"import_surface":                     {"surfaceDigest"},
		"extension":                          {"valueDigest"},
		"effective_policy":                   {"valueDigest"},
		"responsibility_closure":             {"closureDigest"},
		"organizational_translation_closure": {"closureDigest"},
		"experience_composition":             {"sourceDigest"},
		"resource":                           {"valueDigest"},
		"resolution_lock":                    {"resolutionDigest"},
		"universe_lock":                      {"universeDigest"},
		"semantic_ir":                        {"workspaceSemanticIrDigest"},
	}
	kind, err := transportVariant(data, "kind", variants)
	if err != nil {
		return err
	}
	var wire struct {
		Kind                      string `json:"kind"`
		SourcePath                string `json:"sourcePath"`
		SourceDigest              string `json:"sourceDigest"`
		ByteLen                   uint32 `json:"byteLen"`
		ComponentDigest           string `json:"componentDigest"`
		ModuleDigest              string `json:"moduleDigest"`
		ValueDigest               string `json:"valueDigest"`
		SurfaceDigest             string `json:"surfaceDigest"`
		ClosureDigest             string `json:"closureDigest"`
		ResolutionDigest          string `json:"resolutionDigest"`
		UniverseDigest            string `json:"universeDigest"`
		WorkspaceSemanticIRDigest string `json:"workspaceSemanticIrDigest"`
	}
	if err := decodeTransportObject(data, &wire, append([]string{"kind"}, variants[kind]...)); err != nil {
		return err
	}
	var next WorkspaceFactValue
	switch kind {
	case "source_content":
		next = SourceContentWorkspaceFact{SourcePath: wire.SourcePath, SourceDigest: wire.SourceDigest, ByteLen: wire.ByteLen}
	case "component":
		next = ComponentWorkspaceFact{ComponentDigest: wire.ComponentDigest}
	case "module":
		next = ModuleWorkspaceFact{ModuleDigest: wire.ModuleDigest}
	case "declaration":
		next = DeclarationWorkspaceFact{ValueDigest: wire.ValueDigest}
	case "import_surface":
		next = ImportSurfaceWorkspaceFact{SurfaceDigest: wire.SurfaceDigest}
	case "extension":
		next = ExtensionWorkspaceFact{ValueDigest: wire.ValueDigest}
	case "effective_policy":
		next = EffectivePolicyWorkspaceFact{ValueDigest: wire.ValueDigest}
	case "responsibility_closure":
		next = ResponsibilityClosureWorkspaceFact{ClosureDigest: wire.ClosureDigest}
	case "organizational_translation_closure":
		next = OrganizationalTranslationClosureWorkspaceFact{ClosureDigest: wire.ClosureDigest}
	case "experience_composition":
		next = ExperienceCompositionWorkspaceFact{SourceDigest: wire.SourceDigest}
	case "resource":
		next = ResourceWorkspaceFact{ValueDigest: wire.ValueDigest}
	case "resolution_lock":
		next = ResolutionLockWorkspaceFact{ResolutionDigest: wire.ResolutionDigest}
	case "universe_lock":
		next = UniverseLockWorkspaceFact{UniverseDigest: wire.UniverseDigest}
	case "semantic_ir":
		next = SemanticIRWorkspaceFact{WorkspaceSemanticIRDigest: wire.WorkspaceSemanticIRDigest}
	}
	*value = WorkspaceFact{Value: next}
	return nil
}
