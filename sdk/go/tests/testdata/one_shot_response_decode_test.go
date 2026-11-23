package pale

import (
	"encoding/json"
	"errors"
	"os"
	"reflect"
	"strings"
	"testing"
)

// Synthetic transport records test the private decoder, not native compilation
// or plan admission. Native producer tests separately bind these field sets.
func TestOneShotResponseDecoder(t *testing.T) {
	t.Run("native response discarded after decoding failure", testOneShotNativeDecodeFailure)
	const candidate = `{"kind":"apply","repositoryId":"r","planDigest":"p","oneShotCompilationDigest":"c","baselineCount":0,"operationCount":0,"createCount":0,"adoptCount":0,"updateCount":0,"unchangedCount":0,"deleteCount":0,"spliceCount":0,"deferredEffectCount":0,"localOnlyTargets":[]}`
	const bound = `{"kind":"apply","repositoryId":"r"}`
	const admitted = `{"kind":"apply","repositoryId":"r","planDigest":"p"}`
	cases := []struct {
		name, valid string
		decode      func(response) (any, error)
		zero        any
		payload     []byte
	}{
		{"candidate", candidate, func(r response) (any, error) { return decodeOneShotCandidateResponse(r) }, PlanCandidateInfo{}, []byte("opaque SDK bytes")},
		{"bound", bound, func(r response) (any, error) { return decodeOneShotBoundResponse(r) }, BoundPlanInfo{}, []byte("opaque SDK bytes")},
		{"admitted", admitted, func(r response) (any, error) { return decodeOneShotAdmittedResponse(r) }, AdmittedPlanInfo{}, nil},
	}
	for _, test := range cases {
		t.Run(test.name, func(t *testing.T) {
			check := func(encoded string, object uint64, payload []byte, valid bool) {
				t.Helper()
				got, err := test.decode(response{object: object, metadata: []byte(encoded), payload: payload})
				if valid {
					if err != nil {
						t.Fatal(err)
					}
				} else if err == nil || !reflect.DeepEqual(got, test.zero) || strings.Contains(err.Error(), "private-marker") {
					t.Fatalf("accepted, partially returned, or leaked metadata: %v", err)
				}
			}
			for _, kind := range []string{"apply", "adopt", "remove"} {
				check(strings.Replace(test.valid, `"apply"`, `"`+kind+`"`, 1), 1, test.payload, true)
			}
			check(test.valid, 0, test.payload, false)
			check(test.valid+` {}`, 1, test.payload, false)
			check(`null`, 1, test.payload, false)
			check(`[]`, 1, test.payload, false)
			check(strings.Replace(test.valid, `"apply"`, `"private-marker"`, 1), 1, test.payload, false)
			check(strings.Replace(test.valid, `{`, `{"extra":"private-marker",`, 1), 1, test.payload, false)
			if test.payload == nil {
				check(test.valid, 1, []byte("unexpected"), false)
			} else {
				check(test.valid, 1, nil, false)
			}
			var fields map[string]json.RawMessage
			if err := json.Unmarshal([]byte(test.valid), &fields); err != nil {
				t.Fatal(err)
			}
			for field, value := range fields {
				for _, mode := range []string{"missing", "null", "alias", "wrong type", "duplicate"} {
					t.Run(field+"/"+mode, func(t *testing.T) {
						copy := make(map[string]json.RawMessage, len(fields))
						for key, bytes := range fields {
							copy[key] = bytes
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
						check(string(encoded), 1, test.payload, false)
					})
				}
			}
		})
	}
	for _, value := range []string{`-1`, `0.5`, `4294967296`, `"private-marker"`} {
		encoded := strings.Replace(candidate, `"operationCount":0`, `"operationCount":`+value, 1)
		if got, err := decodeOneShotCandidateResponse(response{object: 1, metadata: []byte(encoded), payload: []byte{1}}); err == nil || !reflect.DeepEqual(got, PlanCandidateInfo{}) {
			t.Fatal("invalid count accepted")
		}
	}
	for _, value := range []string{`[null]`, `[1]`, `[{}]`} {
		encoded := strings.Replace(candidate, `"localOnlyTargets":[]`, `"localOnlyTargets":`+value, 1)
		if got, err := decodeOneShotCandidateResponse(response{object: 1, metadata: []byte(encoded), payload: []byte{1}}); err == nil || !reflect.DeepEqual(got, PlanCandidateInfo{}) {
			t.Fatal("invalid target accepted")
		}
	}
	// Decoding does not reinterpret counts, target order, or opaque digest text.
	ordered := strings.Replace(candidate, `"localOnlyTargets":[]`, `"localOnlyTargets":["b","a"]`, 1)
	got, err := decodeOneShotCandidateResponse(response{object: 1, metadata: []byte(ordered), payload: []byte{0xff}})
	if err != nil || !reflect.DeepEqual(got.LocalOnlyTargets, []string{"b", "a"}) || got.PlanDigest != "p" || got.OperationCount != 0 {
		t.Fatal("projection changed SDK values")
	}
	for _, target := range []any{&PlanCandidateInfo{RepositoryID: "keep"}, &BoundPlanInfo{RepositoryID: "keep"}, &AdmittedPlanInfo{RepositoryID: "keep"}} {
		before, _ := json.Marshal(target)
		if err := json.Unmarshal([]byte(`{"kind":"apply","repositoryId":"replace","private-marker":true}`), target); err == nil {
			t.Fatal("invalid record accepted")
		}
		after, _ := json.Marshal(target)
		if string(before) != string(after) {
			t.Fatal("failed decode mutated receiver")
		}
	}
	if err := decodeCanonicalHandleResponse(response{object: 1, metadata: []byte{0xff}}); err != nil {
		t.Fatal("canonical bytes were re-admitted")
	}
	for _, result := range []response{{object: 1}, {metadata: []byte{1}}, {object: 1, metadata: []byte{1}, payload: []byte{1}}} {
		if err := decodeCanonicalHandleResponse(result); err == nil {
			t.Fatal("incomplete compilation accepted")
		}
	}
}

// The native producer emits a valid result. This test corrupts only the owned
// Go metadata copy at the existing locked decode callback, not Rust output or
// production wrapper code. It tests failed output transfer, not source admission
// or restoration of consumed input.
func testOneShotNativeDecodeFailure(t *testing.T) {
	encoded, err := os.ReadFile(os.Getenv("PALE_GO_TEST_FIXTURE"))
	if err != nil {
		t.Fatal(err)
	}
	var fixture struct {
		Request      OneShotRequest `json:"request"`
		RepositoryID string         `json:"repositoryId"`
	}
	if err := json.Unmarshal(encoded, &fixture); err != nil {
		t.Fatal(err)
	}
	client, err := NewClient()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(client.Close)
	compilation, _, err := client.CompileOneShot(fixture.Request)
	if err != nil {
		t.Fatal(err)
	}
	targets, err := compilation.ObservationTargets(fixture.RepositoryID, PlanApply, PriorMaterialization{})
	if err != nil {
		t.Fatal(err)
	}
	var projection struct {
		Targets []string `json:"targets"`
	}
	if err := json.Unmarshal(targets.Bytes, &projection); err != nil {
		t.Fatal(err)
	}
	baseline := make([]BaselineEntry, len(projection.Targets))
	for index, path := range projection.Targets {
		baseline[index] = BaselineEntry{RelativePath: path}
	}
	candidate, _, _, err := compilation.Plan(fixture.RepositoryID, PlanApply, PriorMaterialization{}, baseline)
	if err != nil {
		t.Fatal(err)
	}
	original := candidate.id()
	var pending uint64
	var saved []byte
	result, err := func() (response, error) {
		client.mu.Lock()
		defer client.mu.Unlock()
		return client.callDecodedLocked(struct {
			Candidate uint64   `json:"candidate"`
			Effects   [][]byte `json:"effects"`
		}{original, [][]byte{}}, func(client, request uint64) uint64 {
			candidate.consume()
			defer dropOneShotCandidate(client, original)
			return opBindOneShotMachineEffects(client, request)
		}, func(result response) error {
			if _, err := decodeOneShotBoundResponse(result); err != nil {
				return err
			}
			pending, saved = result.object, result.payload
			result.metadata = []byte(`{"private-marker":true}`)
			_, err := decodeOneShotBoundResponse(result)
			return err
		})
	}()
	if err == nil || !reflect.DeepEqual(result, response{}) || pending == 0 || candidate.id() != 0 {
		t.Fatal("decode failure did not surrender input and zero the result")
	}
	assertMissing := func(err error) {
		t.Helper()
		var failure *BridgeError
		if !errors.As(err, &failure) || failure.Code != "invalid_binding_handle" {
			t.Fatalf("native value unexpectedly usable: %v", err)
		}
	}
	_, err = client.call(struct {
		Candidate uint64   `json:"candidate"`
		Effects   [][]byte `json:"effects"`
	}{original, [][]byte{}}, opBindOneShotMachineEffects)
	assertMissing(err)
	_, err = client.call(struct {
		BoundPlan  uint64 `json:"boundPlan"`
		SavedBytes []byte `json:"savedBytes"`
	}{pending, saved}, opAdmitOneShotSavedPlan)
	assertMissing(err)
	if _, err := compilation.Summary(); err != nil {
		t.Fatalf("decode failure removed an independent input: %v", err)
	}
}
