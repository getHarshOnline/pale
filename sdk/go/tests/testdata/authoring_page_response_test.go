package pale

import (
	"encoding/json"
	"errors"
	"os"
	"reflect"
	"strings"
	"testing"
)

// Real native calls supply cursor custody. Truncation below is injected only
// at the host decoder, not represented as output emitted by the compiler.
func TestAuthoringPageResponseCustody(t *testing.T) {
	bytes, err := os.ReadFile(os.Getenv("PALE_GO_TEST_FIXTURE"))
	if err != nil {
		t.Fatal(err)
	}
	var fixture OneShotRequest
	if err = json.Unmarshal(bytes, &fixture); err != nil {
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
		ProjectID: fixture.ProjectID, ExecutionWorkspaceID: fixture.ExecutionWorkspaceID, MachineID: fixture.MachineID,
		PrivateInputs: fixture.PrivateInputs, PackageDocuments: fixture.PackageDocuments,
	})
	if err != nil {
		t.Fatal(err)
	}
	defer profile.Close()
	navigation, err := workspace.NavigateAuthoring("pale-workspace.toml", []InstancePathSegment{
		AuthoringField("workspace"), AuthoringField("desired_topology"), AuthoringField("repositories"), AuthoringIndex(0), AuthoringField("id"),
	})
	if err != nil || navigation.Identity == nil {
		t.Fatal(err)
	}
	defer navigation.Close()
	for _, route := range []struct {
		name    string
		owner   uint64
		invoke  operation
		profile bool
	}{
		{"portable", workspace.id(), opPortableAuthoringOccurrences, false},
		{"effective", profile.id(), opEffectiveAuthoringOccurrences, false},
		{"profile", profile.id(), opEffectiveAuthoringPage, true},
	} {
		t.Run(route.name, func(t *testing.T) {
			request := map[string]any{"handle": route.owner, "identity": navigation.Identity.id(), "after": nil}
			if route.profile {
				request["limit"] = 1
			} else {
				request["maximumInspected"] = 1
				request["maximumReturned"] = 1
			}
			var cursor uint64
			assertUnavailable := func(err error) {
				t.Helper()
				var failure *BridgeError
				if !errors.As(err, &failure) || failure.Code != "invalid_binding_handle" {
					t.Fatalf("unclaimed cursor was reachable: %v", err)
				}
			}
			result, err := client.callDecoded(request, route.invoke, func(result response) error {
				var truncated error
				if route.profile {
					var value effectiveAuthoringPageEnvelope
					if err := json.Unmarshal(result.metadata, &value); err != nil || value.ContinuationHandle == nil {
						t.Fatal("real profile response incomplete", err)
					}
					cursor = *value.ContinuationHandle
					baseline := value
					truncated = json.Unmarshal(result.metadata[:len(result.metadata)-1], &value)
					if !reflect.DeepEqual(value, baseline) {
						t.Fatal("partial decode replaced profile data")
					}
					assertProfileEnvelopeRejections(t, result.metadata)
				} else {
					var value occurrencePageEnvelope
					if err := json.Unmarshal(result.metadata, &value); err != nil || value.ContinuationHandle == nil {
						t.Fatal("real occurrence response incomplete", err)
					}
					cursor = *value.ContinuationHandle
					baseline := value
					truncated = json.Unmarshal(result.metadata[:len(result.metadata)-1], &value)
					if !reflect.DeepEqual(value, baseline) {
						t.Fatal("partial decode replaced occurrence data")
					}
					assertOccurrenceEnvelopeRejections(t, result.metadata)
				}
				if truncated == nil {
					t.Fatal("truncated metadata was accepted")
				}
				request["after"] = cursor
				_, probeErr := client.callLocked(request, route.invoke)
				assertUnavailable(probeErr)
				return truncated
			})
			if err == nil || cursor == 0 || !reflect.DeepEqual(result, response{}) {
				t.Fatal("failed delivery exposed partial data")
			}
			_, err = client.call(request, route.invoke)
			assertUnavailable(err)
			// Failure does not consume existing compilation or identity handles.
			request["after"] = nil
			_, err = client.callDecoded(request, route.invoke, func(result response) error {
				if route.profile {
					var value effectiveAuthoringPageEnvelope
					return json.Unmarshal(result.metadata, &value)
				}
				var value occurrencePageEnvelope
				return json.Unmarshal(result.metadata, &value)
			})
			if err != nil {
				t.Fatal("failed delivery changed producer", err)
			}
		})
	}
}

func assertOccurrenceEnvelopeRejections(t *testing.T, valid []byte) {
	t.Helper()
	var baseline occurrencePageEnvelope
	if err := json.Unmarshal(valid, &baseline); err != nil {
		t.Fatal(err)
	}
	check := func(bytes []byte) {
		t.Helper()
		value := baseline
		if err := json.Unmarshal(bytes, &value); err == nil || !reflect.DeepEqual(value, baseline) || strings.Contains(err.Error(), "private-marker") {
			t.Fatal("invalid occurrence metadata accepted or partially exposed", err)
		}
	}
	assertPageRecordRejections(t, valid, check)
	for _, mutation := range []string{
		strings.Replace(string(valid), `"isComplete":false`, `"isComplete":true`, 1),
		strings.Replace(string(valid), `"occurrences":[]`, `"occurrences":[{"location":{"source_label":"s","path":[]},"role":"private-marker"}]`, 1),
	} {
		if mutation != string(valid) {
			check([]byte(mutation))
		}
	}
}

func assertProfileEnvelopeRejections(t *testing.T, valid []byte) {
	t.Helper()
	var baseline effectiveAuthoringPageEnvelope
	if err := json.Unmarshal(valid, &baseline); err != nil {
		t.Fatal(err)
	}
	assertPageRecordRejections(t, valid, func(bytes []byte) {
		t.Helper()
		value := baseline
		if err := json.Unmarshal(bytes, &value); err == nil || !reflect.DeepEqual(value, baseline) || strings.Contains(err.Error(), "private-marker") {
			t.Fatal("invalid profile metadata accepted or partially exposed", err)
		}
	})
}

func assertPageRecordRejections(t *testing.T, valid []byte, check func([]byte)) {
	t.Helper()
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(valid, &fields); err != nil {
		t.Fatal(err)
	}
	for name := range fields {
		for _, mode := range []string{"missing", "alias", "duplicate", "wrong-type"} {
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
			}
			bytes, err := json.Marshal(next)
			if err != nil {
				t.Fatal(err)
			}
			if mode == "duplicate" {
				bytes = append([]byte(`{"`+name+`":`+string(next[name])+`,`), bytes[1:]...)
			}
			check(bytes)
		}
	}
	fields["continuationHandle"] = json.RawMessage(`0`)
	bytes, err := json.Marshal(fields)
	if err != nil {
		t.Fatal(err)
	}
	check(bytes)
	check(append(append([]byte{}, valid...), []byte(` {}`)...))
}
