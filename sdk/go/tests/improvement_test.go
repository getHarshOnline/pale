package tests_test

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func TestConfigurationImprovementPhaseDecodingAndRequestRedaction(t *testing.T) {
	for _, valid := range []pale.ConfigurationImprovementPhase{pale.ImprovementInspected, pale.ImprovementExplained, pale.ImprovementProposed, pale.ImprovementEvaluated, pale.ImprovementDiagnosed, pale.ImprovementPreviewed} {
		wire, err := json.Marshal(valid)
		if err != nil {
			t.Fatal(err)
		}
		var actual pale.ConfigurationImprovementPhase
		if err := json.Unmarshal(wire, &actual); err != nil || actual != valid {
			t.Fatalf("phase roundtrip: %v", err)
		}
	}
	phase := pale.ImprovementExplained
	for _, invalid := range []string{`"active"`, `null`, `{}`, `1`} {
		if err := json.Unmarshal([]byte(invalid), &phase); err == nil || phase != pale.ImprovementExplained {
			t.Fatal("unknown phase changed existing value")
		}
	}
	request := pale.ConfigurationImprovementCandidateRequest{PrivateInputs: pale.PrivateInputs{MachineBinding: pale.Document{Bytes: []byte("private-source-sentinel")}}}
	proposal := pale.ConfigurationImprovementProposalRequest{Candidate: request, Workspace: pale.PortableWorkspaceRequest{Documents: []pale.Document{{Bytes: []byte("private-source-sentinel")}}}}
	for _, value := range []any{request, proposal} {
		if strings.Contains(fmt.Sprintf("%v %+v %#v", value, value, value), "private-source-sentinel") {
			t.Fatal("request formatting disclosed private source")
		}
	}
}

type improvementFixture struct {
	CandidateMachineRoot      string                     `json:"candidate_machine_root"`
	CandidateRepositoryAccess string                     `json:"candidate_repository_access"`
	Summary                   json.RawMessage            `json:"summary"`
	Comparison                json.RawMessage            `json:"comparison"`
	Diagnostics               map[string]json.RawMessage `json:"diagnostics"`
}

type improvementContext struct {
	client    *pale.Client
	baseline  *pale.PortableWorkspace
	profile   *pale.EffectiveProfile
	candidate *pale.PortableWorkspace
	source    pale.PortableWorkspaceRequest
	request   pale.ConfigurationImprovementCandidateRequest
	expected  improvementFixture
}

func newImprovementContext(t *testing.T) improvementContext {
	t.Helper()
	client, err := pale.NewClient()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(client.Close)
	fixture := readOneShotFixture(t)
	var expected improvementFixture
	readFixture(t, "improvement-journey.json", &expected)
	baseline, _, err := client.CompilePortableWorkspace(fixture.Workspace)
	if err != nil {
		t.Fatal(err)
	}
	profile, _, err := baseline.CompileEffectiveProfile(pale.EffectiveProfileRequest{
		ProjectID: fixture.ProjectID, ExecutionWorkspaceID: fixture.ExecutionWorkspaceID,
		MachineID: fixture.MachineID, PrivateInputs: fixture.PrivateInputs, PackageDocuments: fixture.PackageDocuments,
	})
	if err != nil {
		t.Fatal(err)
	}
	source := fixture.Workspace
	source.Documents = append([]pale.Document(nil), source.Documents...)
	for index, document := range source.Documents {
		if document.Label == source.RootManifest {
			source.Documents[index].Bytes = bytes.ReplaceAll(document.Bytes, []byte("access = \"mutate\""), []byte("access = \""+expected.CandidateRepositoryAccess+"\""))
		}
	}
	candidate, _, err := client.CompilePortableWorkspace(source)
	if err != nil {
		t.Fatal(err)
	}
	private := fixture.PrivateInputs
	private.MachineBinding.Bytes = bytes.ReplaceAll(private.MachineBinding.Bytes, []byte("/captured/repository"), []byte(expected.CandidateMachineRoot))
	request := pale.ConfigurationImprovementCandidateRequest{
		ProjectID: fixture.ProjectID, ExecutionWorkspaceID: fixture.ExecutionWorkspaceID,
		MachineID: fixture.MachineID, PrivateInputs: private,
	}
	return improvementContext{client, baseline, profile, candidate, source, request, expected}
}

func (context improvementContext) explain(t *testing.T) *pale.ConfigurationImprovementExplanation {
	t.Helper()
	inspection, err := context.baseline.InspectConfigurationImprovement()
	if err != nil {
		t.Fatal(err)
	}
	state, err := inspection.State()
	if err != nil || state.Phase != pale.ImprovementInspected || state.WorkspaceID != "act-ws" || len(state.PrincipalUniverseDigest) != 64 {
		t.Fatalf("inspection lost typed owner coordinates: %#v %v", state, err)
	}
	explanation, err := inspection.Explain(context.profile)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := inspection.Explain(context.profile); err == nil {
		t.Fatal("inspection was reusable after consumption")
	}
	if _, err := inspection.State(); err == nil {
		t.Fatal("consumed inspection remained observable")
	}
	explained, err := explanation.State()
	if err != nil || explained.Phase != pale.ImprovementExplained || len(explained.ProfileDigest) != 64 || len(explained.InstanceDigest) != 64 {
		t.Fatalf("explanation lost its separate profile and instance coordinates: %#v %v", explained, err)
	}
	return explanation
}

func compactImprovementFixture(t *testing.T, expected json.RawMessage) []byte {
	t.Helper()
	var buffer bytes.Buffer
	if err := json.Compact(&buffer, expected); err != nil {
		t.Fatal(err)
	}
	return buffer.Bytes()
}

func assertImprovementDocument(t *testing.T, actual pale.CanonicalDocument, expected json.RawMessage) {
	t.Helper()
	if !bytes.Equal(actual.Bytes, compactImprovementFixture(t, expected)) {
		t.Fatalf("canonical improvement fixture mismatch:\n%s\n%s", actual.Bytes, expected)
	}
}

func assertImprovementDiagnostic(t *testing.T, err error, expected json.RawMessage) {
	t.Helper()
	var diagnostic *pale.DiagnosticError
	if !errors.As(err, &diagnostic) {
		t.Fatalf("expected canonical API diagnostic, got %T %v", err, err)
	}
	if !bytes.Equal(diagnostic.Canonical, compactImprovementFixture(t, expected)) {
		t.Fatalf("diagnostic projection drifted: %v", err)
	}
}

func TestConfigurationImprovementSourceStagesAndRetainedQueries(t *testing.T) {
	context := newImprovementContext(t)
	explanation := context.explain(t)
	proposal, err := explanation.Propose(pale.ConfigurationImprovementProposalRequest{Candidate: context.request, Workspace: context.source})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := explanation.Prepare(context.candidate, context.request); err == nil {
		t.Fatal("consumed explanation allowed preparation")
	}
	if _, err := explanation.State(); err == nil {
		t.Fatal("consumed explanation remained observable")
	}
	if phase, err := proposal.Phase(); err != nil || phase != pale.ImprovementProposed {
		t.Fatalf("proposal phase: %v %v", phase, err)
	}
	evaluation, err := proposal.Evaluate()
	if err != nil {
		t.Fatal(err)
	}
	if _, err := proposal.Evaluate(); err == nil {
		t.Fatal("proposal consumed twice")
	}
	evaluated, err := evaluation.State()
	if err != nil || evaluated.Phase != pale.ImprovementEvaluated {
		t.Fatalf("evaluation phase: %#v %v", evaluated, err)
	}
	var expected struct {
		CandidatePrincipalUniverseDigest string `json:"candidate_principal_universe_digest"`
		CandidateProfileDigest           string `json:"candidate_profile_digest"`
		CandidateInstanceDigest          string `json:"candidate_instance_digest"`
		DerivationReceiptDigest          string `json:"derivation_receipt_digest"`
	}
	decode(t, context.expected.Summary, &expected)
	if evaluated.CandidatePrincipalUniverseDigest != expected.CandidatePrincipalUniverseDigest || evaluated.CandidateProfileDigest != expected.CandidateProfileDigest || evaluated.CandidateInstanceDigest != expected.CandidateInstanceDigest {
		t.Fatal("evaluation coordinates drifted from the API fixture")
	}
	diagnosis, err := evaluation.Diagnose()
	if err != nil {
		t.Fatal(err)
	}
	if _, err := evaluation.Diagnose(); err == nil {
		t.Fatal("evaluation consumed twice")
	}
	if _, err := evaluation.State(); err == nil {
		t.Fatal("consumed evaluation remained observable")
	}
	diagnosed, err := diagnosis.State()
	if err != nil || diagnosed.Phase != pale.ImprovementDiagnosed || diagnosed.DerivationReceiptDigest != expected.DerivationReceiptDigest {
		t.Fatalf("diagnosis proof drifted: %#v %v", diagnosed, err)
	}
	preview, err := diagnosis.Preview()
	if err != nil {
		t.Fatal(err)
	}
	if _, err := diagnosis.Preview(); err == nil {
		t.Fatal("diagnosis consumed twice")
	}
	if phase, err := preview.Phase(); err != nil || phase != pale.ImprovementPreviewed {
		t.Fatalf("preview phase: %v %v", phase, err)
	}
	summary, err := preview.Summary()
	if err != nil {
		t.Fatal(err)
	}
	assertImprovementDocument(t, summary, context.expected.Summary)
	comparison, err := preview.ComparisonSummary()
	if err != nil {
		t.Fatal(err)
	}
	assertImprovementDocument(t, comparison, context.expected.Comparison)

	roots, err := preview.ChangeRoots(0, 1)
	if err != nil {
		t.Fatal(err)
	}
	var page struct {
		Entries []struct {
			Subject struct {
				Kind     string `json:"kind"`
				SourceID string `json:"sourceId"`
			} `json:"subject"`
		} `json:"entries"`
	}
	decode(t, roots.Bytes, &page)
	if len(page.Entries) != 1 {
		t.Fatalf("missing source change root: %s", roots.Bytes)
	}
	impacts, err := preview.Impacts(0, 1)
	if err != nil {
		t.Fatal(err)
	}
	decode(t, impacts.Bytes, &page)
	if len(page.Entries) != 1 || page.Entries[0].Subject.Kind != "source" {
		t.Fatalf("missing source impact: %s", impacts.Bytes)
	}
	proof, err := preview.Explain(pale.SourceSubject(page.Entries[0].Subject.SourceID))
	if err != nil || len(proof.Bytes) == 0 {
		t.Fatalf("missing impact proof: %v", err)
	}

	for _, side := range []pale.ConfigurationImprovementProfile{pale.ImprovementBaselineProfile, pale.ImprovementCandidateProfile} {
		profilePage, err := preview.InspectProfile(side, pale.ProfileIntendedUseCase, 0, 1)
		if err != nil {
			t.Fatal(err)
		}
		var entries struct {
			Entries []struct {
				ExplanationHandle string `json:"explanationHandle"`
			} `json:"entries"`
		}
		decode(t, profilePage.Bytes, &entries)
		if len(entries.Entries) != 1 || entries.Entries[0].ExplanationHandle == "" {
			t.Fatalf("missing profile handle: %s", profilePage.Bytes)
		}
		explanation, err := preview.ExplainProfile(side, entries.Entries[0].ExplanationHandle)
		if err != nil || explanation == nil || explanation.Handle != entries.Entries[0].ExplanationHandle || explanation.Evidence.Kind == "" {
			t.Fatalf("profile explanation failed: %v", err)
		}
		if missing, err := preview.ExplainProfile(side, strings.Repeat("0", 64)); err != nil || missing != nil {
			t.Fatal("unmatched profile explanation did not return nil")
		}
		if malformed, err := preview.ExplainProfile(side, "not-a-handle"); err == nil || malformed != nil {
			t.Fatal("malformed profile explanation handle passed admission")
		}
	}
	if _, err := preview.InspectProfile("invalid", pale.ProfileIntendedUseCase, 0, 1); err == nil {
		t.Fatal("invalid profile side passed")
	}
	terminal, err := preview.Accept()
	if err != nil || terminal.Disposition != pale.ImprovementAccepted {
		t.Fatalf("accept failed: %v", err)
	}
	assertImprovementDocument(t, terminal.Preview, context.expected.Summary)
	if _, err := preview.Summary(); err == nil {
		t.Fatal("consumed preview retained callable proof")
	}
	if explanation, err := preview.ExplainProfile(pale.ImprovementBaselineProfile, strings.Repeat("0", 64)); err == nil || explanation != nil {
		t.Fatal("consumed preview still resolved profile explanations")
	}
	if _, err := preview.Phase(); err == nil {
		t.Fatal("consumed preview phase remained observable")
	}
	if _, err := preview.Reject(); err == nil {
		t.Fatal("preview consumed twice")
	}
}

func TestConfigurationImprovementPreparedRetryOwnerAndTerminals(t *testing.T) {
	context := newImprovementContext(t)
	explanation := context.explain(t)
	invalid := context.request
	invalid.ProjectID = "unselected"
	_, err := explanation.Prepare(context.candidate, invalid)
	assertImprovementDiagnostic(t, err, context.expected.Diagnostics["context_mismatch"])
	prepared, err := explanation.Prepare(context.candidate, context.request)
	if err != nil {
		t.Fatal(err)
	}
	summary, err := prepared.Summary()
	if err != nil {
		t.Fatal(err)
	}
	assertImprovementDocument(t, summary, context.expected.Summary)
	foreign := context.explain(t)
	_, err = foreign.Commit(prepared)
	assertImprovementDiagnostic(t, err, context.expected.Diagnostics["preparation_owner_mismatch"])
	if _, err := prepared.Summary(); err == nil {
		t.Fatal("mismatched commit did not consume preparation")
	}
	if _, err := foreign.Prepare(context.candidate, context.request); err == nil {
		t.Fatal("mismatched commit did not consume supplied explanation")
	}
	// The original owner was borrowed, not the foreign explanation consumed above.
	prepared, err = explanation.Prepare(context.candidate, context.request)
	if err != nil {
		t.Fatal(err)
	}
	preview, err := explanation.Commit(prepared)
	if err != nil {
		t.Fatal(err)
	}
	terminal, err := preview.Reject()
	if err != nil || terminal.Disposition != pale.ImprovementRejected {
		t.Fatalf("reject failed: %v", err)
	}
	assertImprovementDocument(t, terminal.Preview, context.expected.Summary)
	if _, err := explanation.Commit(prepared); err == nil {
		t.Fatal("commit consumed twice")
	}

	for _, disposition := range []pale.ConfigurationImprovementDisposition{pale.ImprovementCancelled, pale.ImprovementStale} {
		explanation := context.explain(t)
		prepared, err := explanation.Prepare(context.candidate, context.request)
		if err != nil {
			t.Fatal(err)
		}
		preview, err := explanation.Commit(prepared)
		if err != nil {
			t.Fatal(err)
		}
		var terminal pale.ConfigurationImprovementTerminal
		if disposition == pale.ImprovementCancelled {
			terminal, err = preview.Cancel()
		} else {
			terminal, err = preview.MarkStale()
		}
		if err != nil || terminal.Disposition != disposition {
			t.Fatalf("terminal mismatch %v %v", terminal, err)
		}
		assertImprovementDocument(t, terminal.Preview, context.expected.Summary)
		if _, err := preview.Accept(); err == nil {
			t.Fatal("terminal preview remained available")
		}
	}
}

func TestConfigurationImprovementCrossClientAndRejectedStage(t *testing.T) {
	context := newImprovementContext(t)
	other := newImprovementContext(t)
	inspection, err := context.baseline.InspectConfigurationImprovement()
	if err != nil {
		t.Fatal(err)
	}
	if _, err := inspection.Explain(other.profile); err == nil {
		t.Fatal("foreign profile accepted")
	}
	explanation, err := inspection.Explain(context.profile)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := explanation.Prepare(other.candidate, context.request); err == nil {
		t.Fatal("foreign candidate accepted")
	}
	foreign := other.explain(t)
	foreignPrepared, err := foreign.Prepare(other.candidate, other.request)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := explanation.Commit(foreignPrepared); err == nil {
		t.Fatal("foreign preparation accepted")
	}
	// The host-scope rejection must not consume either side.
	if _, err := foreign.Commit(foreignPrepared); err != nil {
		t.Fatal(err)
	}
	badSource := context.source
	badSource.RootManifest = "missing.toml"
	_, err = explanation.Propose(pale.ConfigurationImprovementProposalRequest{Candidate: context.request, Workspace: badSource})
	var diagnostic *pale.DiagnosticError
	if !errors.As(err, &diagnostic) {
		t.Fatalf("source capture rejection lost diagnostic: %T %v", err, err)
	}
	if _, err := explanation.Prepare(context.candidate, context.request); err == nil {
		t.Fatal("failed consuming propose remained reusable")
	}
	var nilPreview *pale.ConfigurationImprovementPreview
	_, nilErr := nilPreview.Summary()
	if nilErr == nil {
		t.Fatal("nil preview accepted")
	}
	if strings.Contains(nilErr.Error(), context.expected.CandidateMachineRoot) {
		t.Fatal("invalid-handle error disclosed private candidate root")
	}
}
