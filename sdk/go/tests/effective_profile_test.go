package tests_test

import (
	"bytes"
	"encoding/json"
	"errors"
	"reflect"
	"slices"
	"strings"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func profileCoordinateFixture(t *testing.T) (pale.EffectiveProfileCoordinates, json.RawMessage) {
	t.Helper()
	var fixture struct {
		CurrentProfileResponse struct {
			Coordinates pale.EffectiveProfileCoordinates `json:"coordinates"`
			Page        json.RawMessage                  `json:"page"`
		} `json:"currentProfileResponse"`
	}
	readFixture(t, "effective-profile-wire.json", &fixture)
	return fixture.CurrentProfileResponse.Coordinates, fixture.CurrentProfileResponse.Page
}

func TestEffectiveProfileCoordinatesNativeParity(t *testing.T) {
	client := newClient(t)
	fixture := readOneShotFixture(t)
	workspace, _, err := client.CompilePortableWorkspace(fixture.Workspace)
	if err != nil {
		t.Fatal(err)
	}
	request := pale.EffectiveProfileRequest{
		ProjectID: fixture.ProjectID, ExecutionWorkspaceID: fixture.ExecutionWorkspaceID,
		MachineID: fixture.MachineID, PrivateInputs: fixture.PrivateInputs, PackageDocuments: fixture.PackageDocuments,
	}
	profile, coordinates, err := workspace.CompileEffectiveProfile(request)
	if err != nil {
		t.Fatal(err)
	}
	expected, expectedPage := profileCoordinateFixture(t)
	if coordinates != expected {
		t.Fatalf("all-field coordinate parity failed: %#v", coordinates)
	}
	page, err := profile.Inspect(pale.ProfileMachineRealization, 0, 1)
	if err != nil {
		t.Fatal(err)
	}
	var actualPage, referencePage any
	decode(t, page.Bytes, &actualPage)
	decode(t, expectedPage, &referencePage)
	if !reflect.DeepEqual(actualPage, referencePage) {
		t.Fatal("retained profile page differs from owner fixture")
	}
	snapshot, summary, err := profile.Observe()
	if err != nil {
		t.Fatal(err)
	}
	defer snapshot.Close()
	generation := summary.Cursor.Generation
	if generation.Kind != pale.ObservationEffectiveProfileGeneration || generation.ProfileDigest != coordinates.ProfileDigest || generation.InstanceDigest != coordinates.InstanceDigest || generation.CompositionPlanDigest != coordinates.CompositionPlanDigest {
		t.Fatal("observation does not retain all three distinct generation coordinates")
	}
	_, repeated, err := workspace.CompileEffectiveProfile(request)
	if err != nil || repeated != coordinates {
		t.Fatalf("same request changed coordinates: %v", err)
	}
	request.PrivateInputs.MachineBinding.Bytes = bytes.ReplaceAll(request.PrivateInputs.MachineBinding.Bytes, []byte("/captured/repository"), []byte("/captured/repository-other"))
	_, changed, err := workspace.CompileEffectiveProfile(request)
	if err != nil {
		t.Fatal(err)
	}
	if changed.InstanceDigest == coordinates.InstanceDigest {
		t.Fatal("private instance change was lost")
	}
	changed.InstanceDigest = coordinates.InstanceDigest
	if changed != coordinates {
		t.Fatal("private-only change altered portable coordinates")
	}
	var missing *pale.PortableWorkspace
	result, absent, err := missing.CompileEffectiveProfile(request)
	var bridge *pale.BridgeError
	if result != nil || absent != (pale.EffectiveProfileCoordinates{}) || !errors.As(err, &bridge) || bridge.Code != "invalid_handle" {
		t.Fatalf("nil workspace did not fail cleanly: %v", err)
	}
}

func TestEffectiveProfileCoordinatesStrictAtomicDecode(t *testing.T) {
	expected, _ := profileCoordinateFixture(t)
	canonical, err := json.Marshal(expected)
	if err != nil {
		t.Fatal(err)
	}
	var fields map[string]json.RawMessage
	decode(t, canonical, &fields)
	names := make([]string, 0, len(fields))
	for name := range fields {
		names = append(names, name)
	}
	slices.Sort(names)
	for _, name := range names {
		original := fields[name]
		alias := strings.ToUpper(name[:1]) + name[1:]
		for _, mode := range []string{"missing", "null", "number", "alias", "alias-only"} {
			t.Run(name+"/"+mode, func(t *testing.T) {
				switch mode {
				case "missing":
					delete(fields, name)
				case "null":
					fields[name] = json.RawMessage(`null`)
				case "number":
					fields[name] = json.RawMessage(`42`)
				case "alias":
					fields[alias] = json.RawMessage(`"private-marker"`)
				case "alias-only":
					delete(fields, name)
					fields[alias] = original
				}
				malformed, marshalErr := json.Marshal(fields)
				if marshalErr != nil {
					t.Fatal(marshalErr)
				}
				actual := expected
				err := json.Unmarshal(malformed, &actual)
				if err == nil || actual != expected || strings.Contains(err.Error(), "private-marker") {
					t.Fatalf("invalid coordinate mutated or leaked: %v", err)
				}
				fields[name] = original
				delete(fields, alias)
			})
		}
	}
	for _, malformed := range [][]byte{
		[]byte(`null`), []byte(`[]`), append(append([]byte{}, canonical...), []byte(` {}`)...),
		[]byte(strings.TrimSuffix(string(canonical), "}") + `,"workspaceId":"private-marker"}`),
		[]byte(strings.TrimSuffix(string(canonical), "}") + `,"unknown":"private-marker"}`),
	} {
		actual := expected
		if err := json.Unmarshal(malformed, &actual); err == nil || actual != expected || strings.Contains(err.Error(), "private-marker") {
			t.Fatalf("malformed record accepted, mutated or leaked: %v", err)
		}
	}
	var actual pale.EffectiveProfileCoordinates
	if err := json.Unmarshal(canonical, &actual); err != nil || actual != expected {
		t.Fatalf("valid coordinates rejected: %v", err)
	}
}
