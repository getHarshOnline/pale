package tests_test

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func TestControlStorageInspectionMissingScopesAreReadOnly(t *testing.T) {
	client := newClient(t)
	root := privateControlStorageRoot(t)
	for _, scenario := range []struct {
		name    string
		request pale.LocalControlStorageInspectionRequest
		kind    pale.LocalControlStoreKind
	}{
		{"principal", pale.NewPrincipalControlStorageInspectionRequest(root, "principal-inspection", "store-inspection", "incarnation-inspection"), pale.LocalControlStoreKindPrincipalTrust},
		{"deployment", pale.NewDeploymentControlStorageInspectionRequest(root, "workspace-inspection", "deployment-binding-inspection", strings.Repeat("0", 64), "store-inspection", "incarnation-inspection"), pale.LocalControlStoreKindDeploymentControl},
	} {
		t.Run(scenario.name, func(t *testing.T) {
			for _, format := range []string{"%v", "%+v", "%#v"} {
				if strings.Contains(fmt.Sprintf(format, scenario.request), root) {
					t.Fatal("request formatting leaked runtime root")
				}
			}
			inspection, err := client.InspectLocalControlStore(scenario.request)
			if err != nil {
				t.Fatal(err)
			}
			if inspection.Kind != scenario.kind || inspection.State.Kind() != pale.LocalControlStorageInspectionStateKindMissing {
				t.Fatalf("unexpected physical inspection: %#v", inspection)
			}
			if inspection.State.Unresolved() != nil || inspection.State.Rejection() != nil {
				t.Fatal("missing state acquired another variant's payload")
			}
			encoded, err := json.MarshalIndent(inspection, "", "  ")
			if err != nil {
				t.Fatal(err)
			}
			if bytes.Contains(encoded, []byte(root)) {
				t.Fatal("inspection leaked runtime root")
			}
			entries, err := os.ReadDir(root)
			if err != nil || len(entries) != 0 {
				t.Fatalf("inspection created files: entries=%v error=%v", entries, err)
			}
		})
	}
}

func TestControlStorageInspectionInvalidCoordinatesKeepCanonicalDiagnostics(t *testing.T) {
	client := newClient(t)
	root := privateControlStorageRoot(t)
	requests := []pale.LocalControlStorageInspectionRequest{
		pale.NewPrincipalControlStorageInspectionRequest("private-relative-runtime", "principal-inspection", "store-inspection", "incarnation-inspection"),
		pale.NewPrincipalControlStorageInspectionRequest(root, "", "store-inspection", "incarnation-inspection"),
		pale.NewPrincipalControlStorageInspectionRequest(root, "principal-inspection", "", "incarnation-inspection"),
		pale.NewPrincipalControlStorageInspectionRequest(root, "principal-inspection", "store-inspection", ""),
		pale.NewDeploymentControlStorageInspectionRequest(root, "", "deployment-binding-inspection", strings.Repeat("0", 64), "store-inspection", "incarnation-inspection"),
		pale.NewDeploymentControlStorageInspectionRequest(root, "workspace-inspection", "", strings.Repeat("0", 64), "store-inspection", "incarnation-inspection"),
		pale.NewDeploymentControlStorageInspectionRequest(root, "workspace-inspection", "deployment-binding-inspection", "not-a-digest", "store-inspection", "incarnation-inspection"),
	}
	for _, request := range requests {
		_, err := client.InspectLocalControlStore(request)
		var diagnostic *pale.DiagnosticError
		if !errors.As(err, &diagnostic) {
			t.Fatalf("expected canonical diagnostic, got %v", err)
		}
		if diagnostic.Code != "local_control_storage_inspection_failed" || diagnostic.Disposition != pale.DiagnosticInvalidInput || diagnostic.Retry != pale.DiagnosticRetryAfterCorrection || diagnostic.Effect != pale.DiagnosticEffectNotApplicable || diagnostic.Stage != pale.DiagnosticHostObservation {
			t.Fatalf("diagnostic policy drifted: %#v", diagnostic)
		}
		admitted, err := client.AdmitDiagnostic(diagnostic.Canonical)
		if err != nil || !bytes.Equal(admitted.Bytes, diagnostic.Canonical) {
			t.Fatalf("canonical diagnostic bytes drifted: %v", err)
		}
		if bytes.Contains(diagnostic.Canonical, []byte(root)) {
			t.Fatal("diagnostic leaked runtime root")
		}
		entries, err := os.ReadDir(root)
		if err != nil || len(entries) != 0 {
			t.Fatalf("invalid inspection created files: %v", err)
		}
	}
	_, err := client.InspectLocalControlStore(pale.LocalControlStorageInspectionRequest{})
	var bridge *pale.BridgeError
	if !errors.As(err, &bridge) || bridge.Code != "invalid_binding_request" {
		t.Fatalf("zero private request must fail before API: %v", err)
	}
}

func privateControlStorageRoot(t *testing.T) string {
	t.Helper()
	root, err := filepath.EvalSymlinks(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	// testing.TempDir uses 0777 subject to umask; the owner requires a private root.
	if err := os.Chmod(root, 0o700); err != nil {
		t.Fatal(err)
	}
	return root
}

func TestControlStorageProjectionRetainsOwnerVariantsAndOrder(t *testing.T) {
	fixture := read(t, "fixtures/control-storage-projection.json")
	var cases []json.RawMessage
	decode(t, fixture, &cases)
	for _, raw := range cases {
		var inspection pale.LocalControlStorageInspection
		decode(t, raw, &inspection)
		encoded, err := json.Marshal(inspection)
		if err != nil {
			t.Fatal(err)
		}
		var before, after any
		decode(t, raw, &before)
		decode(t, encoded, &after)
		if !reflect.DeepEqual(before, after) {
			t.Fatalf("owner projection changed: %s -> %s", raw, encoded)
		}
		if inspection.State.Kind() == pale.LocalControlStorageInspectionStateKindProfileUnselected {
			requirements := inspection.State.Unresolved()
			if requirements == nil {
				t.Fatal("profile-unselected empty sequence was flattened to absence")
			}
			if len(requirements) != 0 {
				expected := requirements[0]
				requirements[0] = pale.LocalControlStorageProfileRequirementSqliteCompileOptions
				if inspection.State.Unresolved()[0] != expected {
					t.Fatal("caller mutated retained requirement order")
				}
			}
		}
		if rejection := inspection.State.Rejection(); rejection != nil {
			observed, supported, present := rejection.Versions()
			if present != (rejection.Kind() == pale.LocalControlStorageInspectionRejectionKindNewerVersion) {
				t.Fatal("version evidence attached to wrong rejection")
			}
			if present && (observed != 9 || supported != 1) {
				t.Fatal("version evidence changed")
			}
		}
	}
}

func TestControlStorageProjectionRejectsMalformedShapesWithoutChangingPriorValue(t *testing.T) {
	const prior = `{"kind":"principal-trust","state":{"kind":"missing"}}`
	for _, malformed := range []string{
		`null`, `{}`,
		`{"kind":"unknown","state":{"kind":"missing"}}`,
		`{"kind":"principal-trust","state":{"kind":"unknown"}}`,
		`{"kind":"principal-trust","state":{"kind":"missing","details":null}}`,
		`{"kind":"principal-trust","state":{"kind":"missing"},"extra":true}`,
		`{"kind":"principal-trust","state":{"kind":"profile-unselected","details":{"unresolved":["unknown"]}}}`,
		`{"kind":"principal-trust","state":{"kind":"profile-unselected","details":{"unresolved":null}}}`,
		`{"kind":"principal-trust","state":{"kind":"rejected","details":{"kind":"unknown"}}}`,
		`{"kind":"principal-trust","state":{"kind":"rejected","details":{"kind":"newer-version","observed":9}}}`,
		`{"kind":"principal-trust","state":{"kind":"rejected","details":{"kind":"newer-version","observed":null,"supported":1}}}`,
		`{"kind":"principal-trust","state":{"kind":"rejected","details":{"kind":"newer-version","observed":4294967296,"supported":1}}}`,
		`{"kind":"principal-trust","state":{"kind":"rejected","details":{"kind":"wrong-store-kind","observed":null}}}`,
		`{"kind":"principal-trust","state":{"kind":"rejected","details":{"kind":"wrong-store-kind","unresolved":[]}}}`,
	} {
		var inspection pale.LocalControlStorageInspection
		decode(t, []byte(prior), &inspection)
		if err := json.Unmarshal([]byte(malformed), &inspection); err == nil {
			t.Fatalf("malformed physical projection accepted: %s", malformed)
		}
		after, err := json.Marshal(inspection)
		if err != nil || string(after) != prior {
			t.Fatalf("failed decode changed prior physical observation: %s %v", after, err)
		}
	}
}
