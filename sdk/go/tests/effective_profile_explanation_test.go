package tests_test

import (
	"bytes"
	"encoding/json"
	"reflect"
	"strings"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func TestEffectiveProfileExplanationNativeEvidence(t *testing.T) {
	client := newClient(t)
	fixture := readOneShotFixture(t)
	workspace, _, err := client.CompilePortableWorkspace(fixture.Workspace)
	if err != nil {
		t.Fatal(err)
	}
	defer workspace.Close()
	request := pale.EffectiveProfileRequest{
		ProjectID: fixture.ProjectID, ExecutionWorkspaceID: fixture.ExecutionWorkspaceID,
		MachineID: fixture.MachineID, PrivateInputs: fixture.PrivateInputs, PackageDocuments: fixture.PackageDocuments,
	}
	profile, coordinates, err := workspace.CompileEffectiveProfile(request)
	if err != nil {
		t.Fatal(err)
	}
	defer profile.Close()
	var owner struct {
		ExplanationResponse struct {
			Explanation pale.EffectiveProfileExplanation `json:"explanation"`
		} `json:"explanationResponse"`
	}
	readFixture(t, "effective-profile-wire.json", &owner)
	expected := owner.ExplanationResponse.Explanation
	actual, err := profile.Explain(expected.Handle)
	if err != nil || actual == nil || !reflect.DeepEqual(*actual, expected) {
		t.Fatalf("explanation differs from Rust-owned fixture: %v", err)
	}
	if actual.Evidence.Kind != pale.EffectiveProfileEvidenceMachineRealization || actual.Evidence.MachineRealization == nil || actual.Evidence.MachineRealization.Realization != pale.EffectiveProfileNativeRealizationUnknown {
		t.Fatal("machine evidence was promoted or lost")
	}
	for _, section := range []pale.EffectiveProfileSection{
		pale.ProfileIntendedUseCase, pale.ProfileEffectiveConfiguration, pale.ProfileCapabilitiesOperations,
		pale.ProfileMachineRealization, pale.ProfileGapsAndExplanations,
	} {
		page, err := profile.Inspect(section, 0, 256)
		if err != nil {
			t.Fatal(err)
		}
		var rows struct {
			Entries []pale.EffectiveProfileEntry `json:"entries"`
		}
		decode(t, page.Bytes, &rows)
		for _, row := range rows.Entries {
			proof, err := profile.Explain(row.ExplanationHandle)
			if err != nil || proof == nil || proof.Handle != row.ExplanationHandle || proof.Evidence.Kind == "" {
				t.Fatalf("retained %s explanation unavailable: %v", row.Kind, err)
			}
		}
	}
	if missing, err := profile.Explain(strings.Repeat("0", 64)); err != nil || missing != nil {
		t.Fatalf("valid unmatched handle was not nil: %v", err)
	}
	if malformed, err := profile.Explain("not-a-handle"); err == nil || malformed != nil {
		t.Fatal("malformed handle passed canonical admission")
	}
	request.PrivateInputs.MachineBinding.Bytes = bytes.ReplaceAll(request.PrivateInputs.MachineBinding.Bytes, []byte("/captured/repository"), []byte("/captured/repository-other-instance"))
	other, changed, err := workspace.CompileEffectiveProfile(request)
	if err != nil {
		t.Fatal(err)
	}
	defer other.Close()
	if changed.InstanceDigest == coordinates.InstanceDigest {
		t.Fatal("private fixture did not change instance")
	}
	changed.InstanceDigest = coordinates.InstanceDigest
	if changed != coordinates {
		t.Fatal("private-only change altered other coordinates")
	}
	if crossed, err := other.Explain(expected.Handle); err != nil || crossed != nil {
		t.Fatal("an explanation crossed private instance scope")
	}
	profile.Close()
	if !reflect.DeepEqual(*actual, expected) {
		t.Fatal("closing profile changed copied evidence")
	}
	if closed, err := profile.Explain(expected.Handle); err == nil || closed != nil {
		t.Fatal("closed profile still resolves handles")
	}
}

// These nine examples exercise transport variants, not nine native scenarios.
func TestEffectiveProfileExplanationEvidenceCodecs(t *testing.T) {
	examples := []struct {
		kind pale.EffectiveProfileExplanationEvidenceKind
		wire string
	}{
		{pale.EffectiveProfileEvidenceDeclaredSelection, `{"kind":"declared-selection","selection":{"kind":"resource","resourceKind":"role","selector":{"kind":"exact","resourceId":"r"}}}`},
		{pale.EffectiveProfileEvidenceResponsibility, `{"kind":"responsibility","contractId":"c","executionPathId":"e","reasonSteps":[{"kind":"entry-contract","contractId":"c"},{"kind":"provider-binding","bindingId":"b"}]}`},
		{pale.EffectiveProfileEvidenceEffectiveSelection, `{"kind":"effective-selection","selection":{"kind":"session","executionWorkspaceId":"w","activationRootCount":4294967295,"additionalRootCount":0,"activationPathCount":1}}`},
		{pale.EffectiveProfileEvidenceExecutionPath, `{"kind":"execution-path","selectionId":"s","reasonSteps":[]}`},
		{pale.EffectiveProfileEvidenceOperationClassification, `{"kind":"operation-classification","exposure":"semantic-only","nativeRealization":"unknown"}`},
		{pale.EffectiveProfileEvidenceCapabilityClassification, `{"kind":"capability-classification","requirementDisposition":{"kind":"narrowed","reason":"capability-not-permitted"},"providerDisposition":{"kind":"bound","provider":"connector"},"nativeRealization":"unknown"}`},
		{pale.EffectiveProfileEvidenceFindingClassification, `{"kind":"finding-classification","finding":{"kind":"requirement","repositoryId":"r","operationId":"o","requirementId":"q","capabilityId":"c","disposition":{"kind":"blocked","reason":"effect-forbidden"}}}`},
		{pale.EffectiveProfileEvidenceMachineRealization, `{"kind":"machine-realization","realization":"unknown"}`},
		{pale.EffectiveProfileEvidenceConfiguration, `{"kind":"configuration"}`},
	}
	for _, example := range examples {
		t.Run(string(example.kind), func(t *testing.T) {
			wire := []byte(`{"handle":"opaque","evidence":` + example.wire + `}`)
			var expected pale.EffectiveProfileExplanation
			if err := json.Unmarshal(wire, &expected); err != nil {
				t.Fatal(err)
			}
			if expected.Handle != "opaque" || expected.Evidence.Kind != example.kind {
				t.Fatal("typed evidence was lost")
			}
			// Every selected payload has its own field; configuration has none.
			fields := reflect.ValueOf(expected.Evidence)
			count := 0
			for index := 1; index < fields.NumField(); index++ {
				if !fields.Field(index).IsNil() {
					count++
				}
			}
			want := 1
			if example.kind == pale.EffectiveProfileEvidenceConfiguration {
				want = 0
			}
			if count != want {
				t.Fatal("evidence selected multiple or missing payloads")
			}
			if reasons := expected.Evidence.Responsibility; reasons != nil {
				if len(reasons.ReasonSteps) != 2 || reasons.ReasonSteps[0].Kind != pale.EffectiveProfilePathReasonEntryContractKind || reasons.ReasonSteps[1].Kind != pale.EffectiveProfilePathReasonProviderBindingKind {
					t.Fatal("reason order was changed")
				}
			}
			if path := expected.Evidence.ExecutionPath; path != nil && path.ReasonSteps == nil {
				t.Fatal("empty reason array was lost")
			}
			var evidence map[string]json.RawMessage
			decode(t, []byte(example.wire), &evidence)
			for name, original := range evidence {
				for _, mode := range []string{"missing", "null", "alias", "wrong-type"} {
					altered := make(map[string]json.RawMessage, len(evidence))
					for key, value := range evidence {
						altered[key] = value
					}
					switch mode {
					case "missing":
						delete(altered, name)
					case "null":
						altered[name] = json.RawMessage("null")
					case "alias":
						delete(altered, name)
						altered[strings.ToUpper(name[:1])+name[1:]] = original
					case "wrong-type":
						altered[name] = json.RawMessage("42")
					}
					payload, err := json.Marshal(altered)
					if err != nil {
						t.Fatal(err)
					}
					assertExplanationDecodeRejected(t, []byte(`{"handle":"opaque","evidence":`+string(payload)+`}`), expected)
				}
			}
			for _, malformed := range []string{
				`{"handle":"opaque","evidence":` + strings.TrimSuffix(example.wire, "}") + `,"unknown":"private-marker"}}`,
				`{"handle":"opaque","evidence":` + strings.TrimSuffix(example.wire, "}") + `,"kind":"private-marker"}}`,
				string(wire) + ` {}`,
			} {
				assertExplanationDecodeRejected(t, []byte(malformed), expected)
			}
		})
	}
}

func assertExplanationDecodeRejected(t *testing.T, wire []byte, expected pale.EffectiveProfileExplanation) {
	t.Helper()
	actual := expected
	err := json.Unmarshal(wire, &actual)
	if err == nil || !reflect.DeepEqual(actual, expected) || strings.Contains(err.Error(), "private-marker") {
		t.Fatalf("malformed explanation accepted, changed receiver or disclosed input: %v", err)
	}
}

func TestEffectiveProfileExplanationNestedRejection(t *testing.T) {
	var existing pale.EffectiveProfileExplanation
	decode(t, []byte(`{"handle":"kept","evidence":{"kind":"configuration"}}`), &existing)
	for _, evidence := range []string{
		`{"kind":"responsibility","contractId":"c","executionPathId":"e","reasonSteps":[null]}`,
		`{"kind":"execution-path","selectionId":"s","reasonSteps":[{"kind":"provider-binding","bindingId":"b","BindingId":"private-marker"}]}`,
		`{"kind":"effective-selection","selection":{"kind":"session","executionWorkspaceId":"w","activationRootCount":4294967296,"additionalRootCount":0,"activationPathCount":1}}`,
		`{"kind":"declared-selection","selection":{"kind":"resource","resourceKind":"private-marker","selector":{"kind":"exact","resourceId":"r"}}}`,
		`{"kind":"operation-classification","exposure":"private-marker","nativeRealization":"unknown"}`,
		`{"kind":"capability-classification","requirementDisposition":{"kind":"narrowed","reason":"private-marker"},"providerDisposition":{"kind":"bound","provider":"connector"},"nativeRealization":"unknown"}`,
		`{"kind":"capability-classification","requirementDisposition":{"kind":"satisfied"},"providerDisposition":{"kind":"bound","provider":null},"nativeRealization":"unknown"}`,
		`{"kind":"finding-classification","finding":{"kind":"native-realization-unknown","machineId":null}}`,
		`{"kind":"machine-realization","realization":"ready"}`,
		`{"kind":"configuration","selection":null}`,
	} {
		assertExplanationDecodeRejected(t, []byte(`{"handle":"opaque","evidence":`+evidence+`}`), existing)
	}
	for _, wire := range []string{
		`null`, `{}`, `{"handle":"opaque"}`, `{"Handle":"opaque","evidence":{"kind":"configuration"}}`,
		`{"handle":null,"evidence":{"kind":"configuration"}}`, `{"handle":"opaque","evidence":null}`,
		`{"handle":"opaque","handle":"private-marker","evidence":{"kind":"configuration"}}`,
		`{"handle":"opaque","evidence":{"kind":"configuration"},"coordinates":{}}`,
	} {
		assertExplanationDecodeRejected(t, []byte(wire), existing)
	}
}

func TestEffectiveProfileExplanationResponseDecoding(t *testing.T) {
	runPackageOverlayTest(t, "effective_profile_explanation_test.go", "TestEffectiveProfileExplanationResponse", nil)
}
