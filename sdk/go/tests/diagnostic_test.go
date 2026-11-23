package tests_test

import (
	"bytes"
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func authoringFailureRequest(t *testing.T, contractRejection bool) pale.PortableWorkspaceRequest {
	t.Helper()
	request := pale.PortableWorkspaceRequest{RootManifest: "pale-workspace.toml"}
	for _, label := range []string{"pale-workspace.toml", "module.toml", "consumer-module.toml", "alpha.toml", "beta.toml", "consumer.toml", "resolution.lock.toml", "universe.lock.toml"} {
		data, err := os.ReadFile(filepath.Join("..", "..", "..", "interfaces", "lsp", "tests", "fixtures", "authoring", "visibility-widening", label))
		if err != nil {
			t.Fatal(err)
		}
		if contractRejection && label == request.RootManifest {
			data = bytes.Replace(data, []byte("schema_version = 1"), []byte("schema_version = 2"), 1)
		}
		request.Documents = append(request.Documents, pale.Document{Label: label, Bytes: data})
	}
	return request
}

func TestDiagnosticAuthoringEvidenceSurvivesClientClose(t *testing.T) {
	field := func(value string) pale.AuthoringLocationSegment {
		return pale.AuthoringLocationSegment{Kind: pale.AuthoringLocationField, Field: &value}
	}
	index := uint64(0)
	for _, contractRejection := range []bool{false, true} {
		client := newClient(t)
		workspace, summary, err := client.CompilePortableWorkspace(authoringFailureRequest(t, contractRejection))
		var failure *pale.DiagnosticError
		if workspace != nil || len(summary.Bytes) != 0 || !errors.As(err, &failure) {
			t.Fatalf("expected only the compiler diagnostic: %v", err)
		}
		canonical := bytes.Clone(failure.Canonical)
		text := failure.Error()
		client.Close()
		if failure.Identity == "" || text != failure.Code+": "+failure.Summary {
			t.Fatal("diagnostic identity or error text missing")
		}
		if contractRejection {
			want := &pale.AuthoringSourceLocation{SourceLabel: "pale-workspace.toml", Path: []pale.AuthoringLocationSegment{field("workspace"), field("schema_version")}}
			if failure.AuthoringCorrections == nil || len(failure.AuthoringCorrections) != 0 || !reflect.DeepEqual(failure.AuthoringLocation, want) {
				t.Fatalf("location-only rejection lost its typed source coordinate: %#v", failure)
			}
		} else {
			if len(failure.AuthoringCorrections) != 1 {
				t.Fatal("the actual compiler must prove one correction")
			}
			correction := failure.AuthoringCorrections[0]
			wantPath := []pale.AuthoringLocationSegment{field("module"), field("imports"), {Kind: pale.AuthoringLocationIndex, Index: &index}, field("visibility_ceiling")}
			if correction.Kind != pale.AuthoringNarrowImportVisibility || correction.SourceLabel != "consumer-module.toml" || !reflect.DeepEqual(correction.Path, wantPath) || correction.ExpectedSource != `"public"` || correction.ReplacementSource != `"protected"` || correction.DiagnosticIdentity != failure.Identity {
				t.Fatalf("correction payload drifted: %#v", correction)
			}
			wantSubject := pale.AuthoringIdentityValue{Kind: pale.AuthoringIdentityExport, Value: "candidate::public-api", AuthoredValue: "public-api"}
			if correction.ProofSubject != wantSubject || !reflect.DeepEqual(failure.AuthoringLocation, &pale.AuthoringSourceLocation{SourceLabel: correction.SourceLabel, Path: wantPath}) {
				t.Fatal("correction lost its copied proof subject or exact location")
			}
			failure.AuthoringCorrections[0].Path[0].Field = nil
			if !reflect.DeepEqual(failure.AuthoringLocation.Path, wantPath) {
				t.Fatal("copied correction and location unexpectedly alias")
			}
		}
		if !bytes.Equal(canonical, failure.Canonical) || failure.Error() != text || strings.Contains(text, "visibility_ceiling") {
			t.Fatal("sidecars changed the canonical diagnostic or concise error")
		}
	}
	client := newClient(t)
	_, err := client.ContractSchema("not-an-owned-schema")
	var failure *pale.DiagnosticError
	if !errors.As(err, &failure) || failure.Identity == "" || failure.AuthoringCorrections == nil || len(failure.AuthoringCorrections) != 0 || failure.AuthoringLocation != nil {
		t.Fatalf("unrelated failure fabricated source evidence: %v", err)
	}
}

func TestDiagnosticMetadataStrictDecoding(t *testing.T) {
	var fixture struct {
		Canonical string `json:"canonical_diagnostic"`
	}
	readFixture(t, "contract-diagnostic.json", &fixture)
	runPackageOverlayTest(t, "diagnostic_metadata_test.go", "TestDiagnosticMetadataPrivateDecoding", []byte(fixture.Canonical))
}
