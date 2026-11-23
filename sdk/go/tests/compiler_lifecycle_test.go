package tests_test

import (
	"bytes"
	"errors"
	"reflect"
	"sync"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func requireUnavailableCompiler(t *testing.T, err error) {
	t.Helper()
	var bridge *pale.BridgeError
	if !errors.As(err, &bridge) || bridge.Code != "invalid_handle" {
		t.Fatalf("closed compiler alias remained usable: %v", err)
	}
}

func TestCompilerClosePreservesIndependentDerivedResults(t *testing.T) {
	context := newImprovementContext(t)
	comparison, summary, err := context.client.ComparePortableWorkspaces(context.baseline, context.candidate)
	if err != nil {
		t.Fatal(err)
	}
	beforePage, before, err := comparison.Impacts(0, 256)
	if err != nil {
		t.Fatal(err)
	}
	inspection, err := context.baseline.InspectConfigurationImprovement()
	if err != nil {
		t.Fatal(err)
	}
	workspaceCopy := *context.baseline
	context.baseline.Close()
	workspaceCopy.Close()
	_, err = workspaceCopy.InspectAuthoring("pale-workspace.toml", nil)
	requireUnavailableCompiler(t, err)
	if _, err := inspection.State(); err != nil {
		t.Fatal(err)
	}
	if _, err := context.profile.Inspect(pale.ProfileEffectiveConfiguration, 0, 1); err != nil {
		t.Fatal(err)
	}
	explanation, err := inspection.Explain(context.profile)
	if err != nil {
		t.Fatal(err)
	}
	prepared, err := explanation.Prepare(context.candidate, context.request)
	if err != nil {
		t.Fatal(err)
	}
	snapshot, _, err := context.profile.Observe()
	if err != nil {
		t.Fatal(err)
	}
	defer snapshot.Close()
	request := pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 256}
	pageBefore, err := snapshot.Page(request)
	if err != nil {
		t.Fatal(err)
	}
	defer pageBefore.Close()
	profileCopy := *context.profile
	context.profile.Close()
	profileCopy.Close()
	_, err = profileCopy.Inspect(pale.ProfileEffectiveConfiguration, 0, 1)
	requireUnavailableCompiler(t, err)
	context.candidate.Close()
	afterPage, after, err := comparison.Impacts(0, 256)
	if err != nil || !reflect.DeepEqual(beforePage, afterPage) || !bytes.Equal(before.Bytes, after.Bytes) {
		t.Fatalf("comparison lost retained owners: %v", err)
	}
	pageAfter, err := snapshot.Page(request)
	if err != nil {
		t.Fatal(err)
	}
	defer pageAfter.Close()
	if !reflect.DeepEqual(pageBefore.Page, pageAfter.Page) {
		t.Fatal("profile disposal changed snapshot facts or coordinates")
	}
	preview, err := explanation.Commit(prepared)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := preview.Summary(); err != nil {
		t.Fatal(err)
	}
	terminal, err := preview.Accept()
	if err != nil || terminal.Disposition != pale.ImprovementAccepted {
		t.Fatalf("retained review failed: %#v %v", terminal, err)
	}
	comparisonCopy := *comparison
	comparison.Close()
	comparisonCopy.Close()
	_, _, err = comparisonCopy.Impacts(0, 1)
	requireUnavailableCompiler(t, err)
	if len(summary.Bytes) == 0 || len(before.Bytes) == 0 || !bytes.Equal(before.Bytes, after.Bytes) {
		t.Fatal("disposal invalidated copied documents")
	}
	expectedPage := bytes.Clone(after.Bytes)
	before.Bytes[0] ^= 0xff
	if !bytes.Equal(after.Bytes, expectedPage) {
		t.Fatal("mutating one page changed an independent response")
	}
	expectedTerminal := bytes.Clone(terminal.Preview.Bytes)
	if len(expectedTerminal) == 0 {
		t.Fatal("accepted review returned no terminal document")
	}
	context.client.Close()
	if !bytes.Equal(after.Bytes, expectedPage) || !bytes.Equal(terminal.Preview.Bytes, expectedTerminal) {
		t.Fatal("client closure invalidated retained page or terminal bytes")
	}
}

func TestComparisonClosePreservesInputsAndNilCloseIsHarmless(t *testing.T) {
	context := newImprovementContext(t)
	comparison, _, err := context.client.ComparePortableWorkspaces(context.baseline, context.candidate)
	if err != nil {
		t.Fatal(err)
	}
	comparison.Close()
	for _, workspace := range []*pale.PortableWorkspace{context.baseline, context.candidate} {
		if _, err := workspace.InspectAuthoring("pale-workspace.toml", []pale.StructuralPathSegment{}); err != nil {
			t.Fatal(err)
		}
	}
	if _, err := context.profile.Inspect(pale.ProfileEffectiveConfiguration, 0, 1); err != nil {
		t.Fatal(err)
	}
	var workspace *pale.PortableWorkspace
	var profile *pale.EffectiveProfile
	var absentComparison *pale.WorkspaceComparison
	workspace.Close()
	profile.Close()
	absentComparison.Close()
	(&pale.PortableWorkspace{}).Close()
	(&pale.EffectiveProfile{}).Close()
	(&pale.WorkspaceComparison{}).Close()
	context.client.Close()
	context.baseline.Close()
	context.candidate.Close()
	context.profile.Close()
	comparison.Close()
}

func compilerRaceError(t *testing.T, err error) {
	t.Helper()
	if err == nil {
		return
	}
	var bridge *pale.BridgeError
	if !errors.As(err, &bridge) || (bridge.Code != "invalid_handle" && bridge.Code != "invalid_binding_handle" && bridge.Code != "closed_client") {
		t.Errorf("compiler lifetime race changed error class: %T %v", err, err)
	}
}

func TestCompilerCloseSerializesWithQueriesConstructionAndClient(t *testing.T) {
	for round := 0; round < 8; round++ {
		context := newImprovementContext(t)
		fixture := readOneShotFixture(t)
		comparison, _, err := context.client.ComparePortableWorkspaces(context.baseline, context.candidate)
		if err != nil {
			t.Fatal(err)
		}
		start := make(chan struct{})
		var group sync.WaitGroup
		for _, action := range []func(){
			context.baseline.Close, context.candidate.Close, context.profile.Close, comparison.Close, context.client.Close,
			func() { _, _, err := comparison.Impacts(0, 1); compilerRaceError(t, err) },
			func() {
				value, _, err := context.profile.Observe()
				compilerRaceError(t, err)
				if value != nil {
					value.Close()
				}
			},
			func() {
				value, _, err := context.client.ComparePortableWorkspaces(context.baseline, context.candidate)
				compilerRaceError(t, err)
				if value != nil {
					value.Close()
				}
			},
			func() {
				value, _, err := context.baseline.CompileEffectiveProfile(pale.EffectiveProfileRequest{
					ProjectID: fixture.ProjectID, ExecutionWorkspaceID: fixture.ExecutionWorkspaceID, MachineID: fixture.MachineID,
					PrivateInputs: fixture.PrivateInputs, PackageDocuments: fixture.PackageDocuments,
				})
				compilerRaceError(t, err)
				if value != nil {
					value.Close()
				}
			},
		} {
			group.Add(1)
			go func() { defer group.Done(); <-start; action() }()
		}
		close(start)
		group.Wait()
	}
}

func TestProfileCloseDoesNotStrandInspectionDuringExplain(t *testing.T) {
	for round := 0; round < 8; round++ {
		context := newImprovementContext(t)
		inspection, err := context.baseline.InspectConfigurationImprovement()
		if err != nil {
			t.Fatal(err)
		}
		if round == 0 {
			context.profile.Close()
		}
		start, done := make(chan struct{}), make(chan struct{})
		go func() { <-start; context.profile.Close(); close(done) }()
		close(start)
		explanation, err := inspection.Explain(context.profile)
		<-done
		if err != nil {
			requireUnavailableCompiler(t, err)
			if _, err := inspection.State(); err != nil {
				t.Fatalf("preflight rejection consumed inspection: %v", err)
			}
		} else if _, err := explanation.State(); err != nil {
			t.Fatalf("profile disposal broke retained explanation: %v", err)
		}
	}
}

func TestWorkspaceClosePreservesBorrowedPreparation(t *testing.T) {
	for round := 0; round < 8; round++ {
		context := newImprovementContext(t)
		explanation := context.explain(t)
		if round == 0 {
			context.candidate.Close()
		}
		start, done := make(chan struct{}), make(chan struct{})
		go func() { <-start; context.candidate.Close(); close(done) }()
		close(start)
		prepared, err := explanation.Prepare(context.candidate, context.request)
		<-done
		compilerRaceError(t, err)
		if _, err := explanation.State(); err != nil {
			t.Fatalf("borrowed preparation consumed explanation: %v", err)
		}
		if prepared != nil {
			preview, err := explanation.Commit(prepared)
			if err != nil {
				t.Fatal(err)
			}
			if _, err := preview.Cancel(); err != nil {
				t.Fatal(err)
			}
		}
	}
}
