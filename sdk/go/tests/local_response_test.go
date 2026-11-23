package tests_test

import (
	"encoding/json"
	"path/filepath"
	"reflect"
	"slices"
	"strings"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func TestLocalOutputCustody(t *testing.T) {
	client := newClient(t)
	identity, err := client.CompiledInterfaceIdentity()
	if err != nil {
		t.Fatal(err)
	}
	root, manifest := layOutLocalWorkspace(t, identity)
	request := pale.LocalWorkspaceRequest{Manifest: manifest, ProjectID: "demo", ExecutionWorkspaceID: "default", MachineID: "machine"}
	compilation, _, err := client.CompileLocalOneShot(request)
	if err != nil {
		t.Fatal(err)
	}
	plan, _, _, err := compilation.Plan("engine", pale.PlanApply)
	if err != nil {
		t.Fatal(err)
	}
	plans := filepath.Join(root, "plans")
	// Native publication is test setup in this unique temporary root. The
	// child checks only construction/admission; it never settles a run.
	if _, err := plan.Publish(plans); err != nil {
		t.Fatal(err)
	}
	encoded, err := json.Marshal(struct {
		Request  pale.LocalWorkspaceRequest `json:"request"`
		Plans    string                     `json:"plans"`
		Evidence string                     `json:"evidence"`
	}{request, plans, filepath.Join(root, "evidence")})
	if err != nil {
		t.Fatal(err)
	}
	runPackageOverlayTest(t, "local_response_custody_test.go", "TestLocalResponseCustody", encoded)
}

// These are transport fixtures, not evidence that a native settlement occurred.
func TestLocalRunResultPreservesRecordedOutcomes(t *testing.T) {
	const succeeded = `{"repositoryId":"first","outcome":"succeeded","changedSurfaceCount":2,"deferredEffectCount":1,"localGitExcludeChanged":true,"failure":null}`
	const failed = `{"repositoryId":"second","outcome":"failed-no-effect","changedSurfaceCount":0,"deferredEffectCount":0,"localGitExcludeChanged":false,"failure":"e30="}`
	successTarget := pale.LocalRunTargetResult{
		RepositoryID: "first", Outcome: pale.LocalTargetSucceeded,
		ChangedSurfaceCount: 2, DeferredEffectCount: 1, LocalGitExcludeChanged: true,
	}
	// The transport preserves these bytes; it does not admit a diagnostic.
	failedTarget := pale.LocalRunTargetResult{
		RepositoryID: "second", Outcome: pale.LocalTargetFailedNoEffect, Failure: []byte(`{}`),
	}
	for _, test := range []struct {
		name, wire string
		want       pale.LocalRunResult
	}{
		{"success", `{"kind":"apply","outcome":"succeeded","targetCount":1,"succeededTargetCount":1,"targets":[` + succeeded + `]}`,
			pale.LocalRunResult{Kind: pale.PlanApply, Outcome: pale.LocalRunSucceeded, TargetCount: 1, SucceededTargetCount: 1, Targets: []pale.LocalRunTargetResult{successTarget}}},
		{"partial", `{"kind":"adopt","outcome":"partial","targetCount":2,"succeededTargetCount":1,"targets":[` + succeeded + `,` + failed + `]}`,
			pale.LocalRunResult{Kind: pale.PlanAdopt, Outcome: pale.LocalRunPartial, TargetCount: 2, SucceededTargetCount: 1, Targets: []pale.LocalRunTargetResult{successTarget, failedTarget}}},
		{"failed", `{"kind":"remove","outcome":"failed-no-effect","targetCount":1,"succeededTargetCount":0,"targets":[` + failed + `]}`,
			pale.LocalRunResult{Kind: pale.PlanRemove, Outcome: pale.LocalRunFailedNoEffect, TargetCount: 1, Targets: []pale.LocalRunTargetResult{failedTarget}}},
	} {
		t.Run(test.name, func(t *testing.T) {
			var actual pale.LocalRunResult
			decode(t, []byte(test.wire), &actual)
			if !reflect.DeepEqual(actual, test.want) {
				t.Fatalf("recorded outcome or payload changed: %#v", actual)
			}
			wire, err := json.Marshal(actual)
			if err != nil {
				t.Fatal(err)
			}
			var roundTrip pale.LocalRunResult
			decode(t, wire, &roundTrip)
			if !reflect.DeepEqual(roundTrip, actual) {
				t.Fatal("result round trip changed an outcome or nullable failure")
			}
		})
	}
}

// Rejected metadata must not overwrite a caller's previous result.
func TestLocalRunResultRejectsIncompleteOrUnknownOutcomes(t *testing.T) {
	const target = `{"repositoryId":"r","outcome":"succeeded","changedSurfaceCount":0,"deferredEffectCount":0,"localGitExcludeChanged":false,"failure":null}`
	const result = `{"kind":"apply","outcome":"succeeded","targetCount":1,"succeededTargetCount":1,"targets":[` + target + `]}`
	for _, test := range []struct {
		name, wire string
		receiver   func() any
	}{
		{"run", result, func() any { return &pale.LocalRunResult{Kind: "remove", Canonical: []byte("keep")} }},
		{"target", target, func() any { return &pale.LocalRunTargetResult{RepositoryID: "keep"} }},
	} {
		t.Run(test.name, func(t *testing.T) {
			checkRejected := func(wire string) {
				t.Helper()
				actual, expected := test.receiver(), test.receiver()
				if err := json.Unmarshal([]byte(wire), actual); err == nil || !reflect.DeepEqual(actual, expected) || strings.Contains(err.Error(), "private-marker") {
					t.Fatalf("invalid result accepted, partially decoded, or leaked: %v", err)
				}
			}
			var fields map[string]json.RawMessage
			decode(t, []byte(test.wire), &fields)
			names := make([]string, 0, len(fields))
			for name := range fields {
				names = append(names, name)
			}
			slices.Sort(names)
			for _, name := range names {
				original := fields[name]
				for _, mode := range []string{"missing", "null", "alias", "duplicate", "wrong type"} {
					if name == "failure" && mode == "null" {
						continue // The SDK explicitly emits this nullable member.
					}
					t.Run(name+"/"+mode, func(t *testing.T) {
						mutated := make(map[string]json.RawMessage, len(fields))
						for key, value := range fields {
							mutated[key] = value
						}
						switch mode {
						case "missing":
							delete(mutated, name)
						case "null":
							mutated[name] = json.RawMessage(`null`)
						case "alias":
							delete(mutated, name)
							mutated[strings.ToUpper(name)] = original
						case "wrong type":
							mutated[name] = json.RawMessage(`{"private-marker":true}`)
						}
						wire, err := json.Marshal(mutated)
						if err != nil {
							t.Fatal(err)
						}
						if mode == "duplicate" {
							wire = []byte(`{"` + name + `":` + string(original) + `,` + string(wire[1:]))
						}
						checkRejected(string(wire))
					})
				}
			}
			for _, wire := range []string{
				`null`, `[]`, test.wire + ` {}`,
				strings.Replace(test.wire, `{`, `{"unknown":"private-marker",`, 1),
				strings.Replace(test.wire, `"outcome":"succeeded"`, `"outcome":"private-marker"`, 1),
			} {
				checkRejected(wire)
			}
			if test.name == "run" {
				checkRejected(strings.Replace(result, `"kind":"apply"`, `"kind":"private-marker"`, 1))
				// A nested target cannot use the run-only partial outcome.
				checkRejected(strings.Replace(result, target, strings.Replace(target, `"succeeded"`, `"partial"`, 1), 1))
			}
		})
	}
}
