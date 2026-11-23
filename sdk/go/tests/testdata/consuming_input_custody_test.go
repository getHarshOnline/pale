// This source is overlaid into package pale only by the owning test runner.
// It inspects private IDs, never a production test hook or public release API.
package pale

import (
	"encoding/json"
	"errors"
	"os"
	"testing"
)

func TestConsumingInputReleasesNativeCustody(t *testing.T) {
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
	requireBridgeError := func(t *testing.T, err error, code string) {
		t.Helper()
		var failure *BridgeError
		if !errors.As(err, &failure) || failure.Code != code {
			t.Fatalf("expected %s, got %v", code, err)
		}
	}
	for _, kind := range []string{"nil-proposal-documents", "nil-proposal-bytes", "nil-effect", "invalid-effect-document", "nil-saved"} {
		t.Run(kind, func(t *testing.T) {
			client, err := NewClient()
			if err != nil {
				t.Fatal(err)
			}
			t.Cleanup(client.Close)
			if kind == "nil-proposal-documents" || kind == "nil-proposal-bytes" {
				baseline, _, err := client.CompilePortableWorkspace(fixture.Request.Workspace)
				if err != nil {
					t.Fatal(err)
				}
				profile, _, err := baseline.CompileEffectiveProfile(EffectiveProfileRequest{
					ProjectID: fixture.Request.ProjectID, ExecutionWorkspaceID: fixture.Request.ExecutionWorkspaceID,
					MachineID: fixture.Request.MachineID, PrivateInputs: fixture.Request.PrivateInputs, PackageDocuments: fixture.Request.PackageDocuments,
				})
				if err != nil {
					t.Fatal(err)
				}
				inspection, err := baseline.InspectConfigurationImprovement()
				if err != nil {
					t.Fatal(err)
				}
				explanation, err := inspection.Explain(profile)
				if err != nil {
					t.Fatal(err)
				}
				original := explanation.id()
				source := fixture.Request.Workspace
				source.Documents = append([]Document(nil), source.Documents...)
				if kind == "nil-proposal-documents" {
					source.Documents = nil
				} else {
					source.Documents[0].Bytes = nil
				}
				_, err = explanation.Propose(ConfigurationImprovementProposalRequest{Workspace: source, Candidate: ConfigurationImprovementCandidateRequest{
					ProjectID: fixture.Request.ProjectID, ExecutionWorkspaceID: fixture.Request.ExecutionWorkspaceID, MachineID: fixture.Request.MachineID, PrivateInputs: fixture.Request.PrivateInputs,
				}})
				requireBridgeError(t, err, "invalid_binding_request")
				if explanation.id() != 0 {
					t.Fatal("Go did not surrender the explanation")
				}
				_, err = client.call(struct {
					Handle uint64 `json:"handle"`
				}{original}, opImprovementExplanationState)
				requireBridgeError(t, err, "invalid_binding_handle")
				return
			}
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
			if kind == "nil-saved" {
				bound, _, saved, err := candidate.BindMachineEffects(nil)
				if err != nil {
					t.Fatal(err)
				}
				original := bound.id()
				_, _, err = bound.AdmitSaved(nil)
				requireBridgeError(t, err, "invalid_binding_request")
				if bound.id() != 0 {
					t.Fatal("Go did not surrender the bound plan")
				}
				_, err = client.call(struct {
					BoundPlan  uint64 `json:"boundPlan"`
					SavedBytes []byte `json:"savedBytes"`
				}{original, saved.Bytes}, opAdmitOneShotSavedPlan)
				requireBridgeError(t, err, "invalid_binding_handle")
				return
			}
			original := candidate.id()
			effects := [][]byte{nil}
			if kind == "invalid-effect-document" {
				effects = [][]byte{[]byte("{}")}
			}
			_, _, _, err = candidate.BindMachineEffects(effects)
			if kind == "nil-effect" {
				requireBridgeError(t, err, "invalid_binding_request")
			} else {
				var diagnostic *DiagnosticError
				if !errors.As(err, &diagnostic) {
					t.Fatalf("expected SDK rejection after removal, got %v", err)
				}
			}
			if candidate.id() != 0 {
				t.Fatal("Go did not surrender the candidate")
			}
			_, err = client.call(struct {
				Candidate uint64   `json:"candidate"`
				Effects   [][]byte `json:"effects"`
			}{original, [][]byte{}}, opBindOneShotMachineEffects)
			requireBridgeError(t, err, "invalid_binding_handle")
		})
	}
}
