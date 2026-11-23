package tests_test

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

// Test the actual wrappers without publishing native IDs or duplicating the
// canonical fixture builder.
func TestConsumingInputReleasesNativeCustody(t *testing.T) {
	fixture := readOneShotFixture(t)
	encoded, err := json.Marshal(struct {
		Request      pale.OneShotRequest `json:"request"`
		RepositoryID string              `json:"repositoryId"`
	}{fixture.Request(), fixture.RepositoryID})
	if err != nil {
		t.Fatal(err)
	}
	runPackageOverlayTest(t, "consuming_input_custody_test.go", "TestConsumingInputReleasesNativeCustody", encoded)
}

func handoverCandidate(t *testing.T) (*pale.Client, *pale.PlanCandidate) {
	t.Helper()
	client, err := pale.NewClient()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(client.Close)
	fixture := readOneShotFixture(t)
	compilation, _, err := client.CompileOneShot(fixture.Request())
	if err != nil {
		t.Fatal(err)
	}
	targets, err := compilation.ObservationTargets(fixture.RepositoryID, pale.PlanApply, pale.PriorMaterialization{})
	if err != nil {
		t.Fatal(err)
	}
	var projection struct {
		Targets []string `json:"targets"`
	}
	decode(t, targets.Bytes, &projection)
	baseline := make([]pale.BaselineEntry, len(projection.Targets))
	for index, path := range projection.Targets {
		baseline[index] = pale.BaselineEntry{RelativePath: path}
	}
	candidate, _, _, err := compilation.Plan(fixture.RepositoryID, pale.PlanApply, pale.PriorMaterialization{}, baseline)
	if err != nil {
		t.Fatal(err)
	}
	return client, candidate
}

func requireHandoverError(t *testing.T, err error, code string) {
	t.Helper()
	var failure *pale.BridgeError
	if !errors.As(err, &failure) || failure.Code != code {
		t.Fatalf("expected %s, got %v", code, err)
	}
}

func TestOneShotMalformedInputsSurrenderOnlyInvokedValues(t *testing.T) {
	for _, kind := range []string{"nil-effect", "invalid-effect-document", "nil-saved"} {
		t.Run(kind, func(t *testing.T) {
			_, candidate := handoverCandidate(t)
			if kind == "nil-saved" {
				bound, _, saved, err := candidate.BindMachineEffects(nil)
				if err != nil {
					t.Fatal(err)
				}
				_, _, err = bound.AdmitSaved(nil)
				requireHandoverError(t, err, "invalid_binding_request")
				_, _, err = bound.AdmitSaved(saved.Bytes)
				requireHandoverError(t, err, "invalid_handle")
				return
			}
			effects := [][]byte{nil}
			if kind == "invalid-effect-document" {
				effects = [][]byte{[]byte("{}")}
			}
			_, _, _, err := candidate.BindMachineEffects(effects)
			if kind == "nil-effect" {
				requireHandoverError(t, err, "invalid_binding_request")
			} else {
				var diagnostic *pale.DiagnosticError
				if !errors.As(err, &diagnostic) {
					t.Fatalf("expected canonical SDK rejection, got %v", err)
				}
			}
			_, _, _, err = candidate.BindMachineEffects(nil)
			requireHandoverError(t, err, "invalid_handle")
		})
	}
}

func TestOneShotClosedClientRejectsBeforeHandover(t *testing.T) {
	client, candidate := handoverCandidate(t)
	client.Close()
	for range 2 {
		_, _, _, err := candidate.BindMachineEffects(nil)
		requireHandoverError(t, err, "closed_client")
	}
	client, candidate = handoverCandidate(t)
	bound, _, saved, err := candidate.BindMachineEffects(nil)
	if err != nil {
		t.Fatal(err)
	}
	client.Close()
	for range 2 {
		_, _, err := bound.AdmitSaved(saved.Bytes)
		requireHandoverError(t, err, "closed_client")
	}
}

// All inputs are shared by two callers; neither scheduling order is privileged.
func raceHandover(t *testing.T, invoke func() error) {
	t.Helper()
	start := make(chan struct{})
	results := make(chan error, 2)
	for range 2 {
		go func() { <-start; results <- invoke() }()
	}
	close(start)
	winners := 0
	for range 2 {
		err := <-results
		if err == nil {
			winners++
		} else {
			requireHandoverError(t, err, "invalid_handle")
		}
	}
	if winners != 1 {
		t.Fatalf("expected one native handover, got %d", winners)
	}
}

func TestOneShotConcurrentHandoverHasOneWinner(t *testing.T) {
	for range 16 {
		_, candidate := handoverCandidate(t)
		raceHandover(t, func() error {
			bound, _, saved, err := candidate.BindMachineEffects(nil)
			if err != nil {
				return err
			}
			_, _, err = bound.AdmitSaved(saved.Bytes)
			return err
		})
		_, candidate = handoverCandidate(t)
		bound, _, saved, err := candidate.BindMachineEffects(nil)
		if err != nil {
			t.Fatal(err)
		}
		raceHandover(t, func() error { _, _, err := bound.AdmitSaved(saved.Bytes); return err })
	}
}

func TestCapturedActionHandoverPreflightAndSingleWinner(t *testing.T) {
	root := t.TempDir()
	if err := os.WriteFile(filepath.Join(root, "action.json"), []byte(`{"kind":"path_inventory","payload":{"style":"posix_relative","paths":["src/./lib.rs"]}}`), 0600); err != nil {
		t.Fatal(err)
	}
	client, err := pale.NewClient()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(client.Close)
	for range 16 {
		captured, err := client.CaptureLocalNativeActionRequest("pale.compute.path-inventory", root, "action.json")
		if err != nil {
			t.Fatal(err)
		}
		raceHandover(t, func() error {
			result, err := captured.Invoke()
			if err == nil && len(result.Bytes) == 0 {
				return errors.New("action lost its output")
			}
			return err
		})
	}
	captured, err := client.CaptureLocalNativeActionRequest("pale.compute.path-inventory", root, "action.json")
	if err != nil {
		t.Fatal(err)
	}
	client.Close()
	for range 2 {
		_, err := captured.Invoke()
		requireHandoverError(t, err, "closed_client")
	}
}
