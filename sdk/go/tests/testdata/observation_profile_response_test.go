package pale

import (
	"encoding/json"
	"errors"
	"os"
	"reflect"
	"strings"
	"testing"
)

// Existing package-overlay runner compiles this test beside the private decoder.
// The native response is genuine; only the host-decoding failure is injected.
func TestObservationProfileResponseTransfer(t *testing.T) {
	bytes, err := os.ReadFile(os.Getenv("PALE_GO_TEST_FIXTURE"))
	if err != nil {
		t.Fatal(err)
	}
	var fixture OneShotRequest
	if err := json.Unmarshal(bytes, &fixture); err != nil {
		t.Fatal(err)
	}
	client, err := NewClient()
	if err != nil {
		t.Fatal(err)
	}
	defer client.Close()
	workspace, _, err := client.CompilePortableWorkspace(fixture.Workspace)
	if err != nil {
		t.Fatal(err)
	}
	defer workspace.Close()
	profile, _, err := workspace.CompileEffectiveProfile(EffectiveProfileRequest{
		ProjectID: fixture.ProjectID, ExecutionWorkspaceID: fixture.ExecutionWorkspaceID,
		MachineID: fixture.MachineID, PrivateInputs: fixture.PrivateInputs, PackageDocuments: fixture.PackageDocuments,
	})
	if err != nil {
		t.Fatal(err)
	}
	defer profile.Close()
	snapshot, _, err := profile.Observe()
	if err != nil {
		t.Fatal(err)
	}
	defer snapshot.Close()
	observations, err := snapshot.Page(ObservationPageRequest{Filter: AllObservations(), MaximumReturned: 256})
	if err != nil {
		t.Fatal(err)
	}
	defer observations.Close()
	var subject uint64
	for _, item := range observations.Subjects {
		if item.Value.Kind == ObservationMachineSubject {
			subject = item.id()
			break
		}
	}
	if subject == 0 {
		t.Fatal("fixture has no machine")
	}
	request := map[string]any{"snapshot": snapshot.id(), "profile": profile.id(), "workspace": workspace.id(),
		"subject": subject, "maximumInspected": 1, "maximumReturned": 1, "after": nil}
	var cursor uint64
	result, err := client.callDecoded(request, opObservationInspectProfile, func(result response) error {
		var envelope observationProfileEnvelope
		if err := json.Unmarshal(result.metadata, &envelope); err != nil || envelope.ContinuationHandle == nil {
			t.Fatal("real response missing continuation", err)
		}
		cursor = *envelope.ContinuationHandle
		assertObservationProfileEnvelope(t, result.metadata)
		baseline := envelope
		truncated := json.Unmarshal(result.metadata[:len(result.metadata)-1], &envelope)
		if truncated == nil || !reflect.DeepEqual(envelope, baseline) {
			t.Fatal("partial response changed destination")
		}
		request["after"] = cursor
		_, err := client.callLocked(request, opObservationInspectProfile)
		var failure *BridgeError
		if !errors.As(err, &failure) || failure.Code != "invalid_binding_handle" {
			t.Fatal("cursor transferred before decode", err)
		}
		return truncated
	})
	if err == nil || cursor == 0 || !reflect.DeepEqual(result, response{}) {
		t.Fatal("rejected result exposed partial custody")
	}
	_, err = client.call(request, opObservationInspectProfile)
	var failure *BridgeError
	if !errors.As(err, &failure) || failure.Code != "invalid_binding_handle" {
		t.Fatal("failed decoding leaked native cursor", err)
	}
	request["after"] = nil
	var successful observationProfileEnvelope
	_, err = client.callDecoded(request, opObservationInspectProfile, func(result response) error {
		return json.Unmarshal(result.metadata, &successful)
	})
	if err != nil || successful.ContinuationHandle == nil {
		t.Fatal("prior owner changed after failed delivery", err)
	}
	cursor = *successful.ContinuationHandle
	defer dropObservationProfileCursor(client.handle, cursor)
	for _, reason := range []ObservationProfileRefusal{ObservationProfileNoSource, ObservationProfileExpiredSource,
		ObservationProfileSourceMismatch, ObservationProfileCoordinateMismatch, ObservationProfileContinuationMismatch, ObservationProfileInvalidLimit} {
		valid := []byte(`{"state":"refused","reason":"` + string(reason) + `"}`)
		var envelope observationProfileEnvelope
		if err := json.Unmarshal(valid, &envelope); err != nil || envelope.Reason == nil || *envelope.Reason != reason {
			t.Fatal("known refusal lost", err)
		}
		assertObservationProfileEnvelope(t, valid)
	}
}

func assertObservationProfileEnvelope(t *testing.T, valid []byte) {
	t.Helper()
	var baseline observationProfileEnvelope
	if err := json.Unmarshal(valid, &baseline); err != nil {
		t.Fatal(err)
	}
	reject := func(bytes []byte) {
		t.Helper()
		actual := baseline
		if err := json.Unmarshal(bytes, &actual); err == nil || !reflect.DeepEqual(actual, baseline) || strings.Contains(err.Error(), "private-marker") {
			t.Fatal("malformed profile response accepted, leaked or changed destination", err)
		}
	}
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(valid, &fields); err != nil {
		t.Fatal(err)
	}
	for name := range fields {
		for _, mode := range []string{"missing", "alias", "duplicate", "wrong-type", "null"} {
			if name == "continuationHandle" && mode == "null" {
				continue
			}
			var next map[string]json.RawMessage
			if err := json.Unmarshal(valid, &next); err != nil {
				t.Fatal(err)
			}
			switch mode {
			case "missing":
				delete(next, name)
			case "alias":
				next[strings.ToUpper(name)] = next[name]
				delete(next, name)
			case "wrong-type":
				next[name] = json.RawMessage(`{"private-marker":true}`)
			case "null":
				next[name] = json.RawMessage(`null`)
			}
			bytes, err := json.Marshal(next)
			if err != nil {
				t.Fatal(err)
			}
			if mode == "duplicate" {
				bytes = append([]byte(`{"`+name+`":`+string(next[name])+`,`), bytes[1:]...)
			}
			reject(bytes)
		}
	}
	for _, broken := range []string{`null`, `[]`, `{"state":"refused","reason":"private-marker"}`,
		`{"state":"refused","reason":"invalid-limit","entries":[]}`} {
		reject([]byte(broken))
	}
	reject(append(append([]byte{}, valid...), []byte(` {}`)...))
	if baseline.State == ObservationProfileReady {
		for name, replacement := range map[string]json.RawMessage{
			"continuationHandle": json.RawMessage(`0`), "inspected": json.RawMessage(`65536`),
			"mapping": json.RawMessage(`"private-marker"`), "entries": json.RawMessage(`[null]`),
		} {
			var next map[string]json.RawMessage
			if err := json.Unmarshal(valid, &next); err != nil {
				t.Fatal(err)
			}
			next[name] = replacement
			bytes, err := json.Marshal(next)
			if err != nil {
				t.Fatal(err)
			}
			reject(bytes)
		}
	}
}
