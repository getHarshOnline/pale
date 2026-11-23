package tests_test

import (
	"bytes"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func TestLocalConfigurationPreviewRetainsReviewWithoutCandidatePackageOrWrites(t *testing.T) {
	client := newClient(t)
	identity, err := client.CompiledInterfaceIdentity()
	if err != nil {
		t.Fatal(err)
	}
	root, manifest := layOutLocalWorkspace(t, identity)
	candidate := filepath.Join(root, "candidate")
	for _, name := range []string{"composition.toml", "pale-workspace.toml", "workspace-resolution.lock.toml", "workspace-universe.lock.toml", "capabilities.toml", "distribution-lock.toml", "machines/machine.toml"} {
		contents := read(t, filepath.Join(root, name))
		if name == "pale-workspace.toml" {
			contents = bytes.ReplaceAll(contents, []byte(`access = "mutate"`), []byte(`access = "read"`))
		}
		if name == "machines/machine.toml" {
			contents = bytes.ReplaceAll(contents, []byte(filepath.Join(root, "distribution")), []byte(filepath.Join(candidate, "absent-package")))
		}
		destination := filepath.Join(candidate, name)
		if err := os.MkdirAll(filepath.Dir(destination), 0o700); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(destination, contents, 0o600); err != nil {
			t.Fatal(err)
		}
	}
	before := configurationTree(t, root)
	request := pale.LocalConfigurationImprovementRequest{
		BaselineManifest: manifest, CandidateManifest: filepath.Join(candidate, "pale-workspace.toml"),
		ProjectID: "demo", ExecutionWorkspaceID: "default", MachineID: "machine",
	}
	if strings.Contains(fmt.Sprintf("%v %+v %#v", request, request, request), root) {
		t.Fatal("request disclosed its root")
	}
	for _, terminal := range []struct {
		finish      func(*pale.ConfigurationImprovementPreview) (pale.ConfigurationImprovementTerminal, error)
		disposition pale.ConfigurationImprovementDisposition
	}{
		{(*pale.ConfigurationImprovementPreview).Accept, pale.ImprovementAccepted},
		{(*pale.ConfigurationImprovementPreview).Reject, pale.ImprovementRejected},
		{(*pale.ConfigurationImprovementPreview).Cancel, pale.ImprovementCancelled},
		{(*pale.ConfigurationImprovementPreview).MarkStale, pale.ImprovementStale},
	} {
		preview, err := client.PreviewLocalConfigurationImprovement(request)
		if err != nil {
			t.Fatal(err)
		}
		if phase, err := preview.Phase(); err != nil || phase != pale.ImprovementPreviewed {
			t.Fatalf("phase: %v %v", phase, err)
		}
		summary, err := preview.Summary()
		if err != nil {
			t.Fatal(err)
		}
		var coordinates struct {
			Profile   string `json:"profile_digest"`
			Candidate string `json:"candidate_profile_digest"`
		}
		decode(t, summary.Bytes, &coordinates)
		if coordinates.Profile == "" || coordinates.Profile == coordinates.Candidate {
			t.Fatal("candidate access change was lost")
		}
		if comparison, err := preview.ComparisonSummary(); err != nil || len(comparison.Bytes) == 0 {
			t.Fatalf("comparison: %v", err)
		}
		if impacts, err := preview.Impacts(0, 1); err != nil || len(impacts.Bytes) == 0 {
			t.Fatalf("impacts: %v", err)
		}
		for _, side := range []pale.ConfigurationImprovementProfile{pale.ImprovementBaselineProfile, pale.ImprovementCandidateProfile} {
			if page, err := preview.InspectProfile(side, pale.ProfileEffectiveConfiguration, 0, 1); err != nil || len(page.Bytes) == 0 {
				t.Fatalf("profile: %v", err)
			}
		}
		result, err := terminal.finish(preview)
		if err != nil || result.Disposition != terminal.disposition || !bytes.Equal(result.Preview.Bytes, summary.Bytes) {
			t.Fatalf("terminal: %#v %v", result, err)
		}
		if _, err := preview.Accept(); err == nil {
			t.Fatal("preview consumed twice")
		}
	}
	bad := request
	bad.BaselineManifest = "relative.toml"
	_, previewFailure := client.PreviewLocalConfigurationImprovement(bad)
	_, _, compileFailure := client.CompileLocalOneShot(pale.LocalWorkspaceRequest{Manifest: "relative.toml", ProjectID: "demo", ExecutionWorkspaceID: "default", MachineID: "machine"})
	var previewDiagnostic, compileDiagnostic *pale.DiagnosticError
	if !errors.As(previewFailure, &previewDiagnostic) || !errors.As(compileFailure, &compileDiagnostic) || !bytes.Equal(previewDiagnostic.Canonical, compileDiagnostic.Canonical) {
		t.Fatalf("canonical failure mismatch: %v / %v", previewFailure, compileFailure)
	}
	if bytes.Contains(previewDiagnostic.Canonical, []byte("relative.toml")) {
		t.Fatal("diagnostic disclosed locator")
	}
	if !reflect.DeepEqual(before, configurationTree(t, root)) {
		t.Fatal("effect-free review changed the source tree")
	}
	if _, err := os.Stat(filepath.Join(candidate, "absent-package")); !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("candidate Package unexpectedly present: %v", err)
	}
}

func configurationTree(t *testing.T, root string) map[string]string {
	t.Helper()
	result := make(map[string]string)
	err := filepath.WalkDir(root, func(path string, entry fs.DirEntry, walkError error) error {
		if walkError != nil {
			return walkError
		}
		relative, err := filepath.Rel(root, path)
		if err != nil {
			return err
		}
		if entry.IsDir() {
			result[relative] = "directory"
		} else {
			result[relative] = sha256Hex(read(t, path))
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	return result
}
