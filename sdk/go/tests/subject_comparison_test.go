package tests_test

import (
	"bytes"
	"encoding/json"
	"reflect"
	"sync"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func subjectComparisonRequest(t *testing.T) pale.PortableWorkspaceRequest {
	t.Helper()
	var fixture struct {
		RootManifest   string   `json:"root_manifest"`
		DocumentLabels []string `json:"document_labels"`
	}
	readFixture(t, "portable-journey.json", &fixture)
	request := pale.PortableWorkspaceRequest{RootManifest: fixture.RootManifest}
	for _, label := range fixture.DocumentLabels {
		request.Documents = append(request.Documents, fixtureDocument(t, label))
	}
	return request
}

func TestSubjectComparisonPrivateResponseFailure(t *testing.T) {
	encoded, err := json.Marshal(subjectComparisonRequest(t))
	if err != nil {
		t.Fatal(err)
	}
	runPackageOverlayTest(t, "subject_comparison_response_test.go", "TestSubjectComparisonResponseFailure", encoded)
}

func compareSubject(t *testing.T, comparison *pale.WorkspaceComparison, subject pale.WorkspaceSubject) (pale.WorkspaceSubjectComparison, pale.CanonicalDocument) {
	t.Helper()
	value, canonical, err := comparison.CompareSubject(subject)
	if err != nil {
		t.Fatal(err)
	}
	var decoded pale.WorkspaceSubjectComparison
	if err := json.Unmarshal(canonical.Bytes, &decoded); err != nil || !reflect.DeepEqual(value, decoded) {
		t.Fatalf("canonical and typed output differ: %#v %v", value, err)
	}
	return value, canonical
}

func sharedSubject(t *testing.T, value pale.WorkspaceSubjectComparison, expected pale.WorkspaceValueComparison) pale.SharedWorkspaceSubject {
	t.Helper()
	shared, ok := value.Values.Value.(pale.SharedWorkspaceSubject)
	if !ok || shared.Comparison != expected {
		t.Fatalf("incorrect membership or equality: %#v", value)
	}
	return shared
}

func TestSubjectComparisonRetainsValuesIndependentlyOfImpactAndHandles(t *testing.T) {
	client, err := pale.NewClient()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(client.Close)
	priorRequest := subjectComparisonRequest(t)
	currentRequest := subjectComparisonRequest(t)
	for index, document := range currentRequest.Documents {
		if document.Label == currentRequest.RootManifest {
			currentRequest.Documents[index].Bytes = append(bytes.Clone(document.Bytes), []byte("\n# Source-only review change.\n")...)
		}
	}
	prior, priorSummary, err := client.CompilePortableWorkspace(priorRequest)
	if err != nil {
		t.Fatal(err)
	}
	current, currentSummary, err := client.CompilePortableWorkspace(currentRequest)
	if err != nil {
		t.Fatal(err)
	}
	comparison, summary, err := client.ComparePortableWorkspaces(prior, current)
	if err != nil {
		t.Fatal(err)
	}
	var parent struct {
		Receipt         string `json:"receipt_digest"`
		PriorUniverse   string `json:"prior_principal_universe_digest"`
		CurrentUniverse string `json:"current_principal_universe_digest"`
		PriorSemantic   string `json:"prior_semantic_ir_digest"`
		CurrentSemantic string `json:"current_semantic_ir_digest"`
	}
	var coordinates [2]struct {
		Universe string `json:"principal_universe_digest"`
		Semantic string `json:"semantic_ir_digest"`
	}
	decode(t, summary.Bytes, &parent)
	decode(t, priorSummary.Bytes, &coordinates[0])
	decode(t, currentSummary.Bytes, &coordinates[1])
	if parent.PriorUniverse != coordinates[0].Universe || parent.CurrentUniverse != coordinates[1].Universe || parent.PriorSemantic != coordinates[0].Semantic || parent.CurrentSemantic != coordinates[1].Semantic || parent.PriorSemantic != parent.CurrentSemantic || parent.Receipt == "" {
		t.Fatal("comparison lost the exact pair or source-only semantic equality")
	}
	prior.Close()
	current.Close()
	workspace, workspaceBytes := compareSubject(t, comparison, pale.WorkspaceIdentitySubject("empty-adopter"))
	shared := sharedSubject(t, workspace, pale.WorkspaceValuesEqual)
	if workspace.Subject.Workspace == nil || workspace.Subject.Workspace.WorkspaceID != "empty-adopter" || workspace.ReceiptDigest != parent.Receipt || !reflect.DeepEqual(shared.PriorFact, shared.CurrentFact) {
		t.Fatal("workspace identity, parent receipt or equal payload was lost")
	}
	if shared.CurrentFact.Value.(pale.SemanticIRWorkspaceFact).WorkspaceSemanticIRDigest != parent.CurrentSemantic {
		t.Fatal("wrong semantic payload")
	}
	roots, rootDocument, err := comparison.ChangeRoots(0, 256)
	if err != nil {
		t.Fatal(err)
	}
	var decodedRoots pale.WorkspaceChangeRootPage
	decode(t, rootDocument.Bytes, &decodedRoots)
	if !reflect.DeepEqual(roots, decodedRoots) || len(roots.Entries) != 1 || roots.NextOffset != nil {
		t.Fatalf("typed change roots lost their canonical page: %#v", roots)
	}
	root := roots.Entries[0]
	if root.Kind != pale.WorkspaceChangeModified || root.Subject.Kind != pale.WorkspaceSubjectValueSourceKind || root.Subject.Source == nil || root.Subject.Source.SourceID != "workspace-manifest" || root.PriorFact == nil || root.CurrentFact == nil {
		t.Fatalf("source transition was flattened: %#v", root)
	}
	impacts, impactDocument, err := comparison.Impacts(0, 256)
	if err != nil {
		t.Fatal(err)
	}
	var decoded pale.WorkspaceImpactPage
	decode(t, impactDocument.Bytes, &decoded)
	if !reflect.DeepEqual(impacts, decoded) {
		t.Fatal("typed and canonical impact pages differ")
	}
	found := false
	for _, impact := range impacts.Entries {
		if impact.Subject.Kind == pale.WorkspaceSubjectValueWorkspaceKind && impact.Subject.Workspace != nil && impact.Subject.Workspace.WorkspaceID == "empty-adopter" {
			found = impact.State == pale.WorkspaceImpactAffected && impact.ContributingRootCount > 0
		}
	}
	if !found || impacts.NextOffset != nil {
		t.Fatal("fixture did not prove affected-but-equal workspace")
	}
	causal, causalDocument, err := comparison.Explain(pale.WorkspaceIdentitySubject("empty-adopter"))
	if err != nil {
		t.Fatal(err)
	}
	var decodedCausal pale.WorkspaceExplanation
	decode(t, causalDocument.Bytes, &decodedCausal)
	if !reflect.DeepEqual(causal, decodedCausal) || causal.State != pale.WorkspaceExplanationAffected || causal.Causal == nil {
		t.Fatalf("typed causal explanation lost its canonical witness: %#v", causal)
	}
	if causal.Causal.Root.RootDigest != root.RootDigest || causal.Causal.Frontier != pale.WorkspaceExplanationCurrent || len(causal.Causal.Steps) < 2 {
		t.Fatalf("causal root, frontier, or steps drifted: %#v", causal.Causal)
	}
	last := causal.Causal.Steps[len(causal.Causal.Steps)-1]
	if last.Subject.Kind != pale.WorkspaceSubjectValueWorkspaceKind || last.Subject.Workspace == nil || last.Subject.Workspace.WorkspaceID != "empty-adopter" {
		t.Fatalf("causal path did not terminate at its query subject: %#v", last)
	}
	source, sourceBytes := compareSubject(t, comparison, pale.SourceSubject("workspace-manifest"))
	sourceShared := sharedSubject(t, source, pale.WorkspaceValuesChanged)
	priorSource := sourceShared.PriorFact.Value.(pale.SourceContentWorkspaceFact)
	currentSource := sourceShared.CurrentFact.Value.(pale.SourceContentWorkspaceFact)
	if priorSource.SourcePath != currentSource.SourcePath || priorSource.SourceDigest == currentSource.SourceDigest || priorSource.ByteLen >= currentSource.ByteLen || source.Subject.Source.SourceID != "workspace-manifest" {
		t.Fatal("changed source payload was flattened")
	}
	for _, expected := range []struct {
		request pale.PortableWorkspaceRequest
		fact    pale.SourceContentWorkspaceFact
	}{{priorRequest, priorSource}, {currentRequest, currentSource}} {
		for _, document := range expected.request.Documents {
			if document.Label == expected.fact.SourcePath && (uint32(len(document.Bytes)) != expected.fact.ByteLen || sha256Hex(document.Bytes) != expected.fact.SourceDigest) {
				t.Fatal("source fact lost admitted bytes or byte length")
			}
		}
	}
	absent, _ := compareSubject(t, comparison, pale.ComponentSubject("absent-component"))
	if _, ok := absent.Values.Value.(pale.AbsentWorkspaceSubject); !ok {
		t.Fatal("missing subject was not absent")
	}
	missing, missingDocument, err := comparison.Explain(pale.ComponentSubject("absent-component"))
	if err != nil || missing.State != pale.WorkspaceExplanationNotPresent || missing.Causal != nil || len(missingDocument.Bytes) == 0 {
		t.Fatalf("missing subject acquired causal evidence: %#v %v", missing, err)
	}
	// A plain subject from another comparison is an identity query, not an
	// authority-bearing handle. An unmatched identity is lawfully absent.
	foreign, _ := compareSubject(t, comparison, pale.WorkspaceIdentitySubject("another-workspace"))
	if _, ok := foreign.Values.Value.(pale.AbsentWorkspaceSubject); !ok {
		t.Fatal("foreign full identity was not absent")
	}
	alias := *comparison
	comparison.Close()
	client.Close()
	_, _, err = alias.CompareSubject(pale.SourceSubject("workspace-manifest"))
	requireUnavailableCompiler(t, err)
	var retained pale.WorkspaceSubjectComparison
	decode(t, sourceBytes.Bytes, &retained)
	if !reflect.DeepEqual(source, retained) {
		t.Fatal("Close invalidated copied result")
	}
	source.Subject.Source.SourceID = "edited-copy"
	source.Values.Value = pale.AbsentWorkspaceSubject{}
	if bytes.Contains(sourceBytes.Bytes, []byte("edited-copy")) {
		t.Fatal("typed edits changed canonical bytes")
	}
	before := bytes.Clone(workspaceBytes.Bytes)
	sourceBytes.Bytes[0] ^= 0xff
	if !bytes.Equal(before, workspaceBytes.Bytes) || workspace.Subject.Workspace.WorkspaceID != "empty-adopter" {
		t.Fatal("separate results alias storage")
	}
}

func TestSubjectComparisonDistinguishesAddedRemovedAndRelabelledSources(t *testing.T) {
	client, err := pale.NewClient()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(client.Close)
	baseRequest := subjectComparisonRequest(t)
	addedRequest := subjectComparisonRequest(t)
	for index, document := range addedRequest.Documents {
		if document.Label == addedRequest.RootManifest {
			addedRequest.Documents[index].Bytes = bytes.Replace(document.Bytes, []byte("experience_compositions = []"), []byte("experience_compositions = [\"composition.toml\"]"), 1)
			if bytes.Equal(document.Bytes, addedRequest.Documents[index].Bytes) {
				t.Fatal("fixture selection was not changed")
			}
		}
	}
	addedRequest.Documents = append(addedRequest.Documents, pale.Document{Label: "composition.toml", Bytes: []byte(`schema = "pale/workspace-composition-source.v1"
schema_version = 1
[declarations]
responsibility_contracts = []
participant_provider_manifests = []
provider_bindings = []
entry_contracts = []
request_translations = []
result_translations = []
termination_contracts = []
settlement_contracts = []
continuity_contracts = []
edge_contracts = []
selected_experience_paths = []
`)})
	base, _, err := client.CompilePortableWorkspace(baseRequest)
	if err != nil {
		t.Fatal(err)
	}
	added, _, err := client.CompilePortableWorkspace(addedRequest)
	if err != nil {
		t.Fatal(err)
	}
	forward, _, err := client.ComparePortableWorkspaces(base, added)
	if err != nil {
		t.Fatal(err)
	}
	reverse, _, err := client.ComparePortableWorkspaces(added, base)
	if err != nil {
		t.Fatal(err)
	}
	currentValue, _ := compareSubject(t, forward, pale.SourceSubject("experience-composition"))
	priorValue, _ := compareSubject(t, reverse, pale.SourceSubject("experience-composition"))
	current, currentOK := currentValue.Values.Value.(pale.CurrentOnlyWorkspaceSubject)
	prior, priorOK := priorValue.Values.Value.(pale.PriorOnlyWorkspaceSubject)
	if !currentOK || !priorOK || !reflect.DeepEqual(current.CurrentFact, prior.PriorFact) {
		t.Fatal("added/removed symmetry failed")
	}
	closure, _ := compareSubject(t, forward, pale.ResponsibilityClosureSubject())
	sharedSubject(t, closure, pale.WorkspaceValuesEqual)
	renamedRequest := subjectComparisonRequest(t)
	for index, document := range renamedRequest.Documents {
		if document.Label == renamedRequest.RootManifest {
			renamedRequest.Documents[index].Label = "renamed.toml"
		}
	}
	renamedRequest.RootManifest = "renamed.toml"
	renamed, _, err := client.CompilePortableWorkspace(renamedRequest)
	if err != nil {
		t.Fatal(err)
	}
	relabelled, _, err := client.ComparePortableWorkspaces(base, renamed)
	if err != nil {
		t.Fatal(err)
	}
	value, _ := compareSubject(t, relabelled, pale.SourceSubject("workspace-manifest"))
	shared := sharedSubject(t, value, pale.WorkspaceValuesChanged)
	left := shared.PriorFact.Value.(pale.SourceContentWorkspaceFact)
	right := shared.CurrentFact.Value.(pale.SourceContentWorkspaceFact)
	if left.SourceDigest != right.SourceDigest || left.ByteLen != right.ByteLen || left.SourcePath == right.SourcePath {
		t.Fatal("equal bytes with a new source label were flattened")
	}
}

func TestSubjectComparisonRejectsUnavailableHandlesAndSerializesClose(t *testing.T) {
	for _, missing := range []*pale.WorkspaceComparison{nil, {}} {
		value, bytes, err := missing.CompareSubject(pale.SourceSubject("source"))
		requireUnavailableCompiler(t, err)
		if value.Values.Value != nil || len(bytes.Bytes) != 0 {
			t.Fatal("failed call published partial output")
		}
	}
	for round := 0; round < 8; round++ {
		client, err := pale.NewClient()
		if err != nil {
			t.Fatal(err)
		}
		workspace, _, err := client.CompilePortableWorkspace(subjectComparisonRequest(t))
		if err != nil {
			t.Fatal(err)
		}
		comparison, _, err := client.ComparePortableWorkspaces(workspace, workspace)
		if err != nil {
			t.Fatal(err)
		}
		value, canonical, err := comparison.CompareSubject(nil)
		if err == nil || value.Values.Value != nil || len(canonical.Bytes) != 0 {
			t.Fatal("nil subject produced a result")
		}
		alias := *comparison
		start := make(chan struct{})
		var group sync.WaitGroup
		for _, action := range []func(){comparison.Close, alias.Close, workspace.Close, client.Close, func() {
			value, canonical, err := alias.CompareSubject(pale.WorkspaceIdentitySubject("empty-adopter"))
			compilerRaceError(t, err)
			if err == nil {
				sharedSubject(t, value, pale.WorkspaceValuesEqual)
				if len(canonical.Bytes) == 0 {
					t.Error("successful racing read lost output")
				}
			}
		}} {
			group.Add(1)
			go func() { defer group.Done(); <-start; action() }()
		}
		close(start)
		group.Wait()
	}
}
