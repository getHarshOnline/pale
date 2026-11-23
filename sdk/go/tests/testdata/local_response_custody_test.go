package pale

import (
	"encoding/json"
	"errors"
	"os"
	"reflect"
	"strings"
	"testing"
)

// These calls use actual native producers with the parent's existing local
// fixture. Its executable binding is not the child's; no settlement or host
// verification is attempted. A rejecting callback tests failed output transfer,
// not a claim that Rust emits corrupt metadata.
func TestLocalResponseCustody(t *testing.T) {
	t.Run("strict transport records", testLocalResponseDecoding)
	bytes, err := os.ReadFile(os.Getenv("PALE_GO_TEST_FIXTURE"))
	if err != nil {
		t.Fatal(err)
	}
	var fixture struct {
		Request  LocalWorkspaceRequest `json:"request"`
		Plans    string                `json:"plans"`
		Evidence string                `json:"evidence"`
	}
	if err := json.Unmarshal(bytes, &fixture); err != nil {
		t.Fatal(err)
	}
	for _, kind := range []string{"compilation", "plan", "run"} {
		t.Run(kind, func(t *testing.T) {
			client, err := NewClient()
			if err != nil {
				t.Fatal(err)
			}
			t.Cleanup(client.Close)
			independent, _, err := client.CompileLocalOneShot(fixture.Request)
			if err != nil {
				t.Fatal(err)
			}
			var request any = fixture.Request
			invoke := opCompileLocalOneShot
			if kind == "plan" {
				request = struct {
					Compilation  uint64   `json:"compilation"`
					RepositoryID string   `json:"repositoryId"`
					Kind         PlanKind `json:"kind"`
				}{independent.id(), "engine", PlanApply}
				invoke = opPlanLocalOneShot
			} else if kind == "run" {
				request = struct {
					Compilation        uint64   `json:"compilation"`
					Kind               PlanKind `json:"kind"`
					SavedPlanDirectory string   `json:"savedPlanDirectory"`
					EvidenceDirectory  string   `json:"evidenceDirectory"`
				}{independent.id(), PlanApply, fixture.Plans, fixture.Evidence}
				invoke = opAdmitLocalMaterializationRun
			}
			var pending uint64
			var probeRequest any
			var probe operation
			assertUnavailable := func(stage string, err error) {
				t.Helper()
				var failure *BridgeError
				if !errors.As(err, &failure) || failure.Code != "invalid_binding_handle" {
					t.Errorf("unclaimed %s remained reachable %s: %v", kind, stage, err)
				}
			}
			result, err := client.callDecoded(request, invoke, func(result response) error {
				pending = result.object
				var err error
				switch kind {
				case "compilation":
					err = decodeCanonicalHandleResponse(result)
					probeRequest = struct {
						Compilation  uint64   `json:"compilation"`
						RepositoryID string   `json:"repositoryId"`
						Kind         PlanKind `json:"kind"`
					}{pending, "engine", PlanApply}
					probe = opPlanLocalOneShot
				case "plan":
					_, err = decodeLocalPlanResponse(result)
					// Empty output directory rejects before any filesystem effect
					// even if a defect leaves this plan reachable.
					probeRequest = struct {
						Plan            uint64 `json:"plan"`
						OutputDirectory string `json:"outputDirectory"`
					}{pending, ""}
					probe = opPublishLocalOneShotPlan
				case "run":
					_, err = decodeLocalRunResponse(result)
					probeRequest = struct {
						Run uint64 `json:"run"`
					}{pending}
					probe = opInspectLocalReconciliation
				}
				if err != nil {
					t.Fatalf("real native projection failed decoding: %v", err)
				}
				// callDecoded already holds client.mu. The locked path avoids
				// re-locking it while checking the still-pending native value.
				_, probeErr := client.callLocked(probeRequest, probe)
				assertUnavailable("before claim", probeErr)
				return errors.New("test rejects output transfer")
			})
			if err == nil || pending == 0 || !reflect.DeepEqual(result, response{}) {
				t.Fatal("rejected decode returned an output")
			}
			_, probeErr := client.call(probeRequest, probe)
			assertUnavailable("after drop", probeErr)
			if _, _, _, err := independent.Plan("engine", PlanApply); err != nil {
				t.Fatalf("independent compilation lost: %v", err)
			}
			if _, err := os.Stat(fixture.Evidence); !errors.Is(err, os.ErrNotExist) {
				t.Fatalf("construction prepared evidence: %v", err)
			}
		})
	}
}

// Synthetic malformed records are not claimed to be native producer output.
func testLocalResponseDecoding(t *testing.T) {
	const plan = `{"kind":"apply","repositoryId":"r","baselineCount":0,"operationCount":0,"totalOperationCount":0,"createCount":0,"adoptCount":0,"updateCount":0,"unchangedCount":0,"deleteCount":0,"spliceCount":0,"deferredEffectCount":0,"machineLocalEffectCount":0}`
	const run = `{"kind":"apply","targets":[{"repositoryId":"r","planDigest":"d"}]}`
	for _, test := range []struct {
		name, valid string
		decode      func(response) (any, error)
		zero        any
		payload     []byte
	}{
		{"plan", plan, func(r response) (any, error) { return decodeLocalPlanResponse(r) }, LocalPlanInfo{}, []byte{0xff}},
		{"run", run, func(r response) (any, error) { return decodeLocalRunResponse(r) }, LocalRunAdmission{}, nil},
	} {
		t.Run(test.name, func(t *testing.T) {
			check := func(encoded []byte, object uint64, payload []byte, valid bool) {
				t.Helper()
				got, err := test.decode(response{object: object, metadata: encoded, payload: payload})
				if valid {
					if err != nil {
						t.Fatal(err)
					}
				} else if err == nil || !reflect.DeepEqual(got, test.zero) || strings.Contains(err.Error(), "private-marker") {
					t.Fatalf("accepted, partially returned, or leaked metadata: %v", err)
				}
			}
			for _, kind := range []string{"apply", "adopt", "remove"} {
				check([]byte(strings.Replace(test.valid, `"apply"`, `"`+kind+`"`, 1)), 1, test.payload, true)
			}
			check([]byte(test.valid), 0, test.payload, false)
			check([]byte(test.valid+` {}`), 1, test.payload, false)
			check([]byte(`null`), 1, test.payload, false)
			check([]byte(strings.Replace(test.valid, `{`, `{"private-marker":true,`, 1)), 1, test.payload, false)
			check([]byte(strings.Replace(test.valid, `"apply"`, `"private-marker"`, 1)), 1, test.payload, false)
			if test.payload == nil {
				check([]byte(test.valid), 1, []byte{1}, false)
			} else {
				check([]byte(test.valid), 1, nil, false)
			}
			// Each required field is checked at its exact nesting level.
			levels := []string{"root"}
			if test.name == "run" {
				levels = append(levels, "target")
			}
			for _, level := range levels {
				var fields map[string]json.RawMessage
				original := test.valid
				if level == "target" {
					original = `{"repositoryId":"r","planDigest":"d"}`
				}
				if err := json.Unmarshal([]byte(original), &fields); err != nil {
					t.Fatal(err)
				}
				for field, value := range fields {
					for _, mode := range []string{"missing", "null", "alias", "duplicate", "wrong type"} {
						t.Run(level+"/"+field+"/"+mode, func(t *testing.T) {
							copy := make(map[string]json.RawMessage, len(fields))
							for key, value := range fields {
								copy[key] = value
							}
							switch mode {
							case "missing":
								delete(copy, field)
							case "null":
								copy[field] = json.RawMessage(`null`)
							case "alias":
								delete(copy, field)
								copy[strings.ToUpper(field)] = value
							case "wrong type":
								copy[field] = json.RawMessage(`{"private-marker":true}`)
							}
							encoded, err := json.Marshal(copy)
							if err != nil {
								t.Fatal(err)
							}
							if mode == "duplicate" {
								encoded = []byte(`{"` + field + `":` + string(value) + `,` + string(encoded[1:]))
							}
							if level == "target" {
								encoded = []byte(`{"kind":"apply","targets":[` + string(encoded) + `]}`)
							}
							check(encoded, 1, test.payload, false)
						})
					}
				}
			}
		})
	}
	for _, count := range []string{`-1`, `0.5`, `4294967296`} {
		encoded := strings.Replace(plan, `"operationCount":0`, `"operationCount":`+count, 1)
		if got, err := decodeLocalPlanResponse(response{object: 1, metadata: []byte(encoded), payload: []byte{1}}); err == nil || !reflect.DeepEqual(got, LocalPlanInfo{}) {
			t.Fatal("invalid count accepted")
		}
	}
	for _, targets := range []string{`[null]`, `[1]`, `[{}]`} {
		if got, err := decodeLocalRunResponse(response{object: 1, metadata: []byte(`{"kind":"apply","targets":` + targets + `}`)}); err == nil || !reflect.DeepEqual(got, LocalRunAdmission{}) {
			t.Fatal("invalid target accepted")
		}
	}
	for _, targets := range []string{`[]`, `[{"repositoryId":"b","planDigest":"2"},{"repositoryId":"a","planDigest":"1"}]`} {
		encoded := []byte(`{"kind":"apply","targets":` + targets + `}`)
		got, err := decodeLocalRunResponse(response{object: 1, metadata: encoded})
		if err != nil {
			t.Fatal(err)
		}
		roundtrip, _ := json.Marshal(got)
		if string(roundtrip) != string(encoded) {
			t.Fatal("decoder changed target order or values")
		}
	}
	for _, target := range []any{&LocalPlanInfo{RepositoryID: "keep"}, &LocalRunAdmission{Kind: PlanRemove}, &LocalRunTargetAdmission{RepositoryID: "keep"}} {
		before, _ := json.Marshal(target)
		if err := json.Unmarshal([]byte(`{"kind":"apply","repositoryId":"replace","private-marker":true}`), target); err == nil {
			t.Fatal("invalid object accepted")
		}
		after, _ := json.Marshal(target)
		if string(before) != string(after) {
			t.Fatal("failed decoding changed existing receiver")
		}
	}
}
