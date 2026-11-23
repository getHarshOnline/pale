package pale

// WorkspaceProvenance retains one compiler-selected semantic owner. It does
// not assert native authority, provider readiness, or an effect capability.
type WorkspaceProvenance struct {
	Value WorkspaceProvenanceValue
}

// WorkspaceProvenanceValue is the closed provenance union.
type WorkspaceProvenanceValue interface {
	isWorkspaceProvenanceValue()
}

// SourceWorkspaceProvenance identifies one retained logical source.
type SourceWorkspaceProvenance struct {
	SourceID string `json:"sourceId"`
}

// ModuleReleaseWorkspaceProvenance identifies one selected module release.
type ModuleReleaseWorkspaceProvenance struct {
	ModuleID     string `json:"moduleId"`
	ReleaseID    string `json:"releaseId"`
	ModuleDigest string `json:"moduleDigest"`
}

// ImportWorkspaceProvenance identifies one resolved import path.
type ImportWorkspaceProvenance struct {
	ImporterModuleID  string `json:"importerModuleId"`
	UpstreamModuleID  string `json:"upstreamModuleId"`
	UpstreamReleaseID string `json:"upstreamReleaseId"`
	ExportID          string `json:"exportId"`
	SourceNamespaceID string `json:"sourceNamespaceId"`
	LocalNamespaceID  string `json:"localNamespaceId"`
}

// ExtensionWorkspaceProvenance identifies one exact extension declaration.
type ExtensionWorkspaceProvenance struct {
	OwnerModuleID        string `json:"ownerModuleId"`
	ExtensionNamespaceID string `json:"extensionNamespaceId"`
	TargetNamespaceID    string `json:"targetNamespaceId"`
	TargetDeclarationID  string `json:"targetDeclarationId"`
}

// CompilationWorkspaceProvenance identifies a deterministic aggregate fact.
type CompilationWorkspaceProvenance struct{}

func (SourceWorkspaceProvenance) isWorkspaceProvenanceValue()        {}
func (ModuleReleaseWorkspaceProvenance) isWorkspaceProvenanceValue() {}
func (ImportWorkspaceProvenance) isWorkspaceProvenanceValue()        {}
func (ExtensionWorkspaceProvenance) isWorkspaceProvenanceValue()     {}
func (CompilationWorkspaceProvenance) isWorkspaceProvenanceValue()   {}

// UnmarshalJSON rejects unknown and cross-variant provenance fields atomically.
func (value *WorkspaceProvenance) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{
		"source":         {"sourceId"},
		"module_release": {"moduleId", "releaseId", "moduleDigest"},
		"import":         {"importerModuleId", "upstreamModuleId", "upstreamReleaseId", "exportId", "sourceNamespaceId", "localNamespaceId"},
		"extension":      {"ownerModuleId", "extensionNamespaceId", "targetNamespaceId", "targetDeclarationId"},
		"compilation":    {},
	}
	kind, err := transportVariant(data, "kind", variants)
	if err != nil {
		return err
	}
	var wire struct {
		Kind string `json:"kind"`
		SourceWorkspaceProvenance
		ModuleReleaseWorkspaceProvenance
		ImportWorkspaceProvenance
		ExtensionWorkspaceProvenance
	}
	if err := decodeTransportObject(data, &wire, append([]string{"kind"}, variants[kind]...)); err != nil {
		return err
	}
	var next WorkspaceProvenanceValue
	switch kind {
	case "source":
		next = wire.SourceWorkspaceProvenance
	case "module_release":
		next = wire.ModuleReleaseWorkspaceProvenance
	case "import":
		next = wire.ImportWorkspaceProvenance
	case "extension":
		next = wire.ExtensionWorkspaceProvenance
	case "compilation":
		next = CompilationWorkspaceProvenance{}
	}
	*value = WorkspaceProvenance{Value: next}
	return nil
}

// WorkspaceDerivationRule identifies the deterministic compiler rule used by
// one causal proof step.
type WorkspaceDerivationRule string

const (
	WorkspaceRuleSourceAdmission           WorkspaceDerivationRule = "source_admission"
	WorkspaceRuleComponentAdmission        WorkspaceDerivationRule = "component_admission"
	WorkspaceRuleModuleSealing             WorkspaceDerivationRule = "module_sealing"
	WorkspaceRuleDeclarationLinking        WorkspaceDerivationRule = "declaration_linking"
	WorkspaceRuleImportLinking             WorkspaceDerivationRule = "import_linking"
	WorkspaceRuleExtensionApplication      WorkspaceDerivationRule = "extension_application"
	WorkspaceRulePolicyComposition         WorkspaceDerivationRule = "policy_composition"
	WorkspaceRuleResponsibilityLowering    WorkspaceDerivationRule = "responsibility_lowering"
	WorkspaceRuleOrganizationalTranslation WorkspaceDerivationRule = "organizational_translation"
	WorkspaceRuleExperienceComposition     WorkspaceDerivationRule = "experience_composition"
	WorkspaceRuleResourceSelection         WorkspaceDerivationRule = "resource_selection"
	WorkspaceRuleResolutionLocking         WorkspaceDerivationRule = "resolution_locking"
	WorkspaceRuleUniverseLocking           WorkspaceDerivationRule = "universe_locking"
	WorkspaceRuleSemanticIRSealing         WorkspaceDerivationRule = "semantic_ir_sealing"
)

// UnmarshalJSON accepts only the compiler's selected rule vocabulary.
func (value *WorkspaceDerivationRule) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value,
		WorkspaceRuleSourceAdmission,
		WorkspaceRuleComponentAdmission,
		WorkspaceRuleModuleSealing,
		WorkspaceRuleDeclarationLinking,
		WorkspaceRuleImportLinking,
		WorkspaceRuleExtensionApplication,
		WorkspaceRulePolicyComposition,
		WorkspaceRuleResponsibilityLowering,
		WorkspaceRuleOrganizationalTranslation,
		WorkspaceRuleExperienceComposition,
		WorkspaceRuleResourceSelection,
		WorkspaceRuleResolutionLocking,
		WorkspaceRuleUniverseLocking,
		WorkspaceRuleSemanticIRSealing,
	)
}

// WorkspaceCausalStep is one complete content-addressed proof fact.
type WorkspaceCausalStep struct {
	NodeDigest string                  `json:"nodeDigest"`
	Subject    WorkspaceSubjectValue   `json:"subject"`
	Provenance WorkspaceProvenance     `json:"provenance"`
	Rule       WorkspaceDerivationRule `json:"rule"`
	Fact       WorkspaceFact           `json:"fact"`
}

// UnmarshalJSON preserves every proof field as one atomic value.
func (value *WorkspaceCausalStep) UnmarshalJSON(data []byte) error {
	var wire struct {
		NodeDigest string                        `json:"nodeDigest"`
		Subject    portableWorkspaceSubjectValue `json:"subject"`
		Provenance WorkspaceProvenance           `json:"provenance"`
		Rule       WorkspaceDerivationRule       `json:"rule"`
		Fact       WorkspaceFact                 `json:"fact"`
	}
	if err := decodeTransportObject(data, &wire,
		[]string{"nodeDigest", "subject", "provenance", "rule", "fact"}); err != nil {
		return err
	}
	*value = WorkspaceCausalStep{
		NodeDigest: wire.NodeDigest,
		Subject:    WorkspaceSubjectValue(wire.Subject),
		Provenance: wire.Provenance,
		Rule:       wire.Rule,
		Fact:       wire.Fact,
	}
	return nil
}

// WorkspaceExplanationFrontier identifies the admitted proof DAG traversed by
// an affected explanation.
type WorkspaceExplanationFrontier string

const (
	WorkspaceExplanationPredecessor WorkspaceExplanationFrontier = "predecessor"
	WorkspaceExplanationCurrent     WorkspaceExplanationFrontier = "current"
)

// UnmarshalJSON accepts only predecessor or current proof custody.
func (value *WorkspaceExplanationFrontier) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, WorkspaceExplanationPredecessor, WorkspaceExplanationCurrent)
}

// WorkspaceCausalExplanation is the selected root and complete shortest
// root-to-target proof sequence for an affected subject.
type WorkspaceCausalExplanation struct {
	Root     WorkspaceChangeRoot          `json:"root"`
	Frontier WorkspaceExplanationFrontier `json:"frontier"`
	Steps    []WorkspaceCausalStep        `json:"steps"`
}

// UnmarshalJSON validates proof custody and rejects empty affected paths.
func (value *WorkspaceCausalExplanation) UnmarshalJSON(data []byte) error {
	type plain WorkspaceCausalExplanation
	var next plain
	if err := decodeTransportObject(data, &next, []string{"root", "frontier", "steps"}); err != nil {
		return err
	}
	if len(next.Steps) == 0 {
		return invalidTransportProjection("affected explanation has no causal steps")
	}
	if next.Frontier == WorkspaceExplanationPredecessor && next.Root.Kind != WorkspaceChangeRemoved ||
		next.Frontier == WorkspaceExplanationCurrent && next.Root.Kind == WorkspaceChangeRemoved {
		return invalidTransportProjection("explanation frontier contradicts its change root")
	}
	*value = WorkspaceCausalExplanation(next)
	return nil
}

// WorkspaceExplanationState is the closed result of one causal query.
type WorkspaceExplanationState string

const (
	WorkspaceExplanationNotPresent WorkspaceExplanationState = "not_present"
	WorkspaceExplanationUnaffected WorkspaceExplanationState = "unaffected"
	WorkspaceExplanationAffected   WorkspaceExplanationState = "affected"
)

// UnmarshalJSON accepts only the compiler's three explanation outcomes.
func (value *WorkspaceExplanationState) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value,
		WorkspaceExplanationNotPresent, WorkspaceExplanationUnaffected, WorkspaceExplanationAffected)
}

// WorkspaceExplanation is one typed causal answer for the exact query subject.
// Causal is present only when State is WorkspaceExplanationAffected.
type WorkspaceExplanation struct {
	Subject WorkspaceSubjectValue       `json:"subject"`
	State   WorkspaceExplanationState   `json:"state"`
	Causal  *WorkspaceCausalExplanation `json:"causal"`
}

// UnmarshalJSON rejects mismatched state and causal evidence atomically.
func (value *WorkspaceExplanation) UnmarshalJSON(data []byte) error {
	var wire struct {
		Subject portableWorkspaceSubjectValue `json:"subject"`
		State   WorkspaceExplanationState     `json:"state"`
		Causal  *WorkspaceCausalExplanation   `json:"causal"`
	}
	if err := decodeTransportObject(data, &wire, []string{"subject", "state"}, "causal"); err != nil {
		return err
	}
	if (wire.State == WorkspaceExplanationAffected) != (wire.Causal != nil) {
		return invalidTransportProjection("explanation state contradicts its causal evidence")
	}
	*value = WorkspaceExplanation{
		Subject: WorkspaceSubjectValue(wire.Subject),
		State:   wire.State,
		Causal:  wire.Causal,
	}
	return nil
}
