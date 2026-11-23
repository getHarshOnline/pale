package tests_test

import (
	"encoding/json"
	"reflect"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

const sourceFact = `{"kind":"source_content","sourcePath":"workspace.toml","sourceDigest":"digest","byteLen":1}`
const modifiedRoot = `{"rootDigest":"root","subject":{"kind":"source","sourceId":"source"},"kind":"modified","priorFact":` + sourceFact + `,"currentFact":` + sourceFact + `}`
const sourceStep = `{"nodeDigest":"node","subject":{"kind":"source","sourceId":"source"},"provenance":{"kind":"source","sourceId":"source"},"rule":"source_admission","fact":` + sourceFact + `}`
const causalWitness = `{"root":` + modifiedRoot + `,"frontier":"current","steps":[` + sourceStep + `]}`

func TestWorkspaceChangeRootPreservesClosedTransition(t *testing.T) {
	var root pale.WorkspaceChangeRoot
	if err := json.Unmarshal([]byte(modifiedRoot), &root); err != nil {
		t.Fatal(err)
	}
	if root.Kind != pale.WorkspaceChangeModified || root.Subject.Source == nil || root.Subject.Source.SourceID != "source" || root.PriorFact == nil || root.CurrentFact == nil {
		t.Fatalf("change root was flattened: %#v", root)
	}
}

func TestWorkspaceChangeRootRejectsContradictoryOrOpenTransport(t *testing.T) {
	retained := pale.WorkspaceChangeRoot{RootDigest: "retained", Kind: pale.WorkspaceChangeAdded}
	for name, encoded := range map[string]string{
		"added-with-prior":       `{"rootDigest":"root","subject":{"kind":"source","sourceId":"source"},"kind":"added","priorFact":` + sourceFact + `,"currentFact":` + sourceFact + `}`,
		"modified-without-prior": `{"rootDigest":"root","subject":{"kind":"source","sourceId":"source"},"kind":"modified","priorFact":null,"currentFact":` + sourceFact + `}`,
		"removed-with-current":   `{"rootDigest":"root","subject":{"kind":"source","sourceId":"source"},"kind":"removed","priorFact":` + sourceFact + `,"currentFact":` + sourceFact + `}`,
		"unknown-kind":           `{"rootDigest":"root","subject":{"kind":"source","sourceId":"source"},"kind":"replaced","priorFact":null,"currentFact":` + sourceFact + `}`,
		"missing-nullable":       `{"rootDigest":"root","subject":{"kind":"source","sourceId":"source"},"kind":"added","currentFact":` + sourceFact + `}`,
		"extra-field":            `{"rootDigest":"root","subject":{"kind":"source","sourceId":"source"},"kind":"added","priorFact":null,"currentFact":` + sourceFact + `,"ready":true}`,
	} {
		t.Run(name, func(t *testing.T) {
			actual := retained
			if err := json.Unmarshal([]byte(encoded), &actual); err == nil {
				t.Fatal("invalid change root was accepted")
			}
			if !reflect.DeepEqual(actual, retained) {
				t.Fatal("failed decoding replaced the previous change root")
			}
		})
	}
}

func TestWorkspaceProvenanceRetainsEveryClosedVariant(t *testing.T) {
	for name, test := range map[string]struct {
		encoded string
		typeOf  reflect.Type
	}{
		"source":         {`{"kind":"source","sourceId":"source"}`, reflect.TypeOf(pale.SourceWorkspaceProvenance{})},
		"module-release": {`{"kind":"module_release","moduleId":"module","releaseId":"release","moduleDigest":"digest"}`, reflect.TypeOf(pale.ModuleReleaseWorkspaceProvenance{})},
		"import":         {`{"kind":"import","importerModuleId":"local","upstreamModuleId":"upstream","upstreamReleaseId":"release","exportId":"export","sourceNamespaceId":"source","localNamespaceId":"local"}`, reflect.TypeOf(pale.ImportWorkspaceProvenance{})},
		"extension":      {`{"kind":"extension","ownerModuleId":"owner","extensionNamespaceId":"extension","targetNamespaceId":"target","targetDeclarationId":"declaration"}`, reflect.TypeOf(pale.ExtensionWorkspaceProvenance{})},
		"compilation":    {`{"kind":"compilation"}`, reflect.TypeOf(pale.CompilationWorkspaceProvenance{})},
	} {
		t.Run(name, func(t *testing.T) {
			var provenance pale.WorkspaceProvenance
			if err := json.Unmarshal([]byte(test.encoded), &provenance); err != nil {
				t.Fatal(err)
			}
			if reflect.TypeOf(provenance.Value) != test.typeOf {
				t.Fatalf("provenance variant was flattened: %#v", provenance)
			}
		})
	}
	retained := pale.WorkspaceProvenance{Value: pale.CompilationWorkspaceProvenance{}}
	for _, encoded := range []string{
		`{"kind":"future"}`,
		`{"kind":"source","sourceId":"source","moduleId":"module"}`,
		`{"kind":"module_release","moduleId":"module","releaseId":"release"}`,
	} {
		actual := retained
		if err := json.Unmarshal([]byte(encoded), &actual); err == nil {
			t.Fatal("open or incomplete provenance was accepted")
		}
		if !reflect.DeepEqual(actual, retained) {
			t.Fatal("failed decoding replaced the previous provenance")
		}
	}
}

func TestWorkspaceDerivationRuleVocabularyIsClosed(t *testing.T) {
	rules := []pale.WorkspaceDerivationRule{
		pale.WorkspaceRuleSourceAdmission,
		pale.WorkspaceRuleComponentAdmission,
		pale.WorkspaceRuleModuleSealing,
		pale.WorkspaceRuleDeclarationLinking,
		pale.WorkspaceRuleImportLinking,
		pale.WorkspaceRuleExtensionApplication,
		pale.WorkspaceRulePolicyComposition,
		pale.WorkspaceRuleResponsibilityLowering,
		pale.WorkspaceRuleOrganizationalTranslation,
		pale.WorkspaceRuleExperienceComposition,
		pale.WorkspaceRuleResourceSelection,
		pale.WorkspaceRuleResolutionLocking,
		pale.WorkspaceRuleUniverseLocking,
		pale.WorkspaceRuleSemanticIRSealing,
	}
	for _, expected := range rules {
		var actual pale.WorkspaceDerivationRule
		if err := json.Unmarshal([]byte(`"`+string(expected)+`"`), &actual); err != nil || actual != expected {
			t.Fatalf("derivation rule %q did not round-trip: %q %v", expected, actual, err)
		}
	}
	var rejected pale.WorkspaceDerivationRule
	if err := json.Unmarshal([]byte(`"future_rule"`), &rejected); err == nil {
		t.Fatal("unknown derivation rule was accepted")
	}
}

func TestWorkspaceExplanationPreservesClosedCausalWitness(t *testing.T) {
	encoded := `{"subject":{"kind":"source","sourceId":"source"},"state":"affected","causal":` + causalWitness + `}`
	var explanation pale.WorkspaceExplanation
	if err := json.Unmarshal([]byte(encoded), &explanation); err != nil {
		t.Fatal(err)
	}
	if explanation.State != pale.WorkspaceExplanationAffected || explanation.Causal == nil || explanation.Causal.Frontier != pale.WorkspaceExplanationCurrent || len(explanation.Causal.Steps) != 1 {
		t.Fatalf("causal witness was flattened: %#v", explanation)
	}
	step := explanation.Causal.Steps[0]
	if step.Rule != pale.WorkspaceRuleSourceAdmission || step.Subject.Source == nil {
		t.Fatalf("causal step lost its typed rule or subject: %#v", step)
	}
}

func TestWorkspaceExplanationRejectsContradictoryOrOpenTransport(t *testing.T) {
	retained := pale.WorkspaceExplanation{State: pale.WorkspaceExplanationUnaffected}
	for name, encoded := range map[string]string{
		"affected-without-cause": `{"subject":{"kind":"source","sourceId":"source"},"state":"affected","causal":null}`,
		"unaffected-with-cause":  `{"subject":{"kind":"source","sourceId":"source"},"state":"unaffected","causal":` + causalWitness + `}`,
		"empty-path":             `{"subject":{"kind":"source","sourceId":"source"},"state":"affected","causal":{"root":` + modifiedRoot + `,"frontier":"current","steps":[]}}`,
		"wrong-frontier":         `{"subject":{"kind":"source","sourceId":"source"},"state":"affected","causal":{"root":` + modifiedRoot + `,"frontier":"predecessor","steps":[` + sourceStep + `]}}`,
		"unknown-rule":           `{"subject":{"kind":"source","sourceId":"source"},"state":"affected","causal":{"root":` + modifiedRoot + `,"frontier":"current","steps":[{"nodeDigest":"node","subject":{"kind":"source","sourceId":"source"},"provenance":{"kind":"source","sourceId":"source"},"rule":"future","fact":` + sourceFact + `}]}}`,
		"missing-causal":         `{"subject":{"kind":"source","sourceId":"source"},"state":"unaffected"}`,
		"extra-field":            `{"subject":{"kind":"source","sourceId":"source"},"state":"unaffected","causal":null,"ready":true}`,
	} {
		t.Run(name, func(t *testing.T) {
			actual := retained
			if err := json.Unmarshal([]byte(encoded), &actual); err == nil {
				t.Fatal("invalid explanation was accepted")
			}
			if !reflect.DeepEqual(actual, retained) {
				t.Fatal("failed decoding replaced the previous explanation")
			}
		})
	}
}
