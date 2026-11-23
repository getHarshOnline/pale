package tests_test

import (
	"encoding/json"
	"fmt"
	"reflect"
	"slices"
	"strings"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func TestEffectiveAuthoringNativeJoin(t *testing.T) {
	_, workspace, profile := navigationFixture(t)
	defer workspace.Close()
	defer profile.Close()
	for _, navigate := range []func(string, []pale.InstancePathSegment) (pale.AuthoringNavigation, error){workspace.NavigateAuthoring, profile.NavigateAuthoring} {
		navigation, err := navigate("pale-workspace.toml", projectNavigationPath("projects", 0, "run_policy"))
		if err != nil {
			t.Fatal(err)
		}
		defer navigation.Close()
		result, err := profile.InspectEffectiveAuthoring(navigation.Identity)
		if err != nil {
			t.Fatal(err)
		}
		if result.Truncated || len(result.Entries) != 1 {
			t.Fatalf("unexpected joined result: %#v", result)
		}
		row := result.Entries[0]
		if row.Kind != pale.EffectiveProfileEntrySelectionKind || row.Selection == nil || row.Selection.Selection.Kind != pale.EffectiveProfileSelectionRunPolicyKind || row.Selection.Selection.RunPolicy == nil || row.Selection.Selection.RunPolicy.RunPolicyID != "standard" {
			t.Fatalf("run policy was not preserved: %#v", row)
		}
		page, err := profile.Inspect(pale.ProfileEffectiveConfiguration, 0, 256)
		if err != nil {
			t.Fatal(err)
		}
		var canonical struct {
			Entries []pale.EffectiveProfileEntry `json:"entries"`
		}
		if err = json.Unmarshal(page.Bytes, &canonical); err != nil {
			t.Fatal(err)
		}
		matches := 0
		for _, candidate := range canonical.Entries {
			if candidate.ExplanationHandle == row.ExplanationHandle {
				matches++
				if !reflect.DeepEqual(candidate, row) {
					t.Fatal("joined row differs from canonical owner page")
				}
			}
		}
		if matches != 1 {
			t.Fatal("joined row has no unique canonical-page match")
		}
		explanation, err := profile.Explain(row.ExplanationHandle)
		if err != nil {
			t.Fatal(err)
		}
		if explanation == nil || explanation.Handle != row.ExplanationHandle || explanation.Evidence.EffectiveSelection == nil || !reflect.DeepEqual(explanation.Evidence.EffectiveSelection.Selection, row.Selection.Selection) {
			t.Fatal("explanation lost exact handle correlation")
		}
		expectedCoordinates, _ := profileCoordinateFixture(t)
		if result.Coordinates != expectedCoordinates {
			t.Fatal("join and managed owner coordinates differ")
		}
		snapshot, _, err := profile.Observe()
		if err != nil {
			t.Fatal(err)
		}
		snapshot.Close()
		navigation.Identity.Description.Value = "edited display is not a selector"
		repeated, err := profile.InspectEffectiveAuthoring(navigation.Identity)
		if err != nil || !reflect.DeepEqual(result, repeated) {
			t.Fatal("copied description changed the retained join")
		}
	}
	// A project identity exists but has no direct profile-entry association.
	project, err := workspace.NavigateAuthoring("pale-workspace.toml", projectNavigationPath("projects", 0, "id"))
	if err != nil {
		t.Fatal(err)
	}
	defer project.Close()
	empty, err := profile.InspectEffectiveAuthoring(project.Identity)
	if err != nil || len(empty.Entries) != 0 || empty.Truncated {
		t.Fatal("ordinary empty association was lost")
	}
	_, otherWorkspace, otherProfile := navigationFixture(t)
	defer otherWorkspace.Close()
	defer otherProfile.Close()
	otherIdentity, err := otherWorkspace.NavigateAuthoring("pale-workspace.toml", projectNavigationPath("projects", 0, "run_policy"))
	if err != nil {
		t.Fatal(err)
	}
	defer otherIdentity.Close()
	_, err = profile.InspectEffectiveAuthoring(otherIdentity.Identity)
	requireInvalidHandle(t, err)
	_, err = otherProfile.InspectEffectiveAuthoring(project.Identity)
	requireInvalidHandle(t, err)
	if retained, err := profile.InspectEffectiveAuthoring(project.Identity); err != nil || !reflect.DeepEqual(retained, empty) {
		t.Fatal("foreign join damaged original identity")
	}
	if retained, err := otherProfile.InspectEffectiveAuthoring(otherIdentity.Identity); err != nil || len(retained.Entries) != 1 {
		t.Fatal("foreign join damaged foreign identity")
	}
}

// These are closed-codec fixtures, not native examples of all semantic variants.
func TestEffectiveAuthoringEntryCodecs(t *testing.T) {
	cases := []string{
		`{"kind":"declared-selection","selection":{"kind":"resource","resourceKind":"role","selector":{"kind":"exact","resourceId":"r"}}}`,
		`{"kind":"responsibility","contractId":"c","executionPathId":"e","participant":{"kind":"engine","engineId":"e"},"responsibilities":[{"kind":"communication"}],"authorityCeiling":["read"],"reasonSteps":[{"kind":"entry-contract","contractId":"c"}],"subject":{"kind":"responsibility-closure"}}`,
		`{"kind":"selection","selection":{"kind":"session","executionWorkspaceId":"w","activationRootCount":4294967295,"additionalRootCount":0,"activationPathCount":1}}`,
		`{"kind":"capability-narrowing","permittedCapabilities":[],"forbiddenCapabilities":["c"],"permittedEffects":["read"],"forbiddenEffects":["write"]}`,
		`{"kind":"governing-ceiling","scope":"s","subjectResource":"r","contributors":["b","a"],"policies":["p"],"permittedCapabilities":[],"forbiddenCapabilities":[],"permittedEffects":[],"forbiddenEffects":[]}`,
		`{"kind":"repository","repositoryId":"r","materializationOrder":4294967295,"accessCeiling":"read"}`,
		`{"kind":"provider-repository","repositoryId":"r","providerBindingId":"p","nativeRepositoryCoordinate":"owner://unchanged","nativeRealization":"unknown"}`,
		`{"kind":"execution-path","selectionId":"s","experiencePathId":"e","responsibilityPathIdentity":"r","responsibilityContractIds":["c"],"workDelegationEdgeIds":[],"providerBindingIds":[],"capabilityCeiling":[],"effectCeiling":[],"terminalProviderBinding":null,"nativeRealization":"unknown","reasonSteps":[],"subject":{"kind":"workspace","workspaceId":"w"}}`,
		`{"kind":"operation","repositoryId":"r","operationId":"o","exposure":"semantic-only","nativeRealization":"unknown"}`,
		`{"kind":"capability","capabilityId":"c","repositoryId":"r","operationId":"o","requirementId":"q","effectClass":"read","requirementDisposition":{"kind":"narrowed","reason":"capability-not-permitted"},"providerDisposition":{"kind":"bound","provider":"connector"},"nativeRealization":"unknown"}`,
		`{"kind":"machine-realization","machineId":"m","realization":"unknown"}`,
		`{"kind":"finding","finding":{"kind":"requirement","repositoryId":"r","operationId":"o","requirementId":"q","capabilityId":"c","disposition":{"kind":"blocked","reason":"effect-forbidden"}}}`,
	}
	for index, fixture := range cases {
		t.Run(fmt.Sprint(index), func(t *testing.T) {
			row := strings.TrimSuffix(fixture, "}") + `,"explanationHandle":"` + strings.Repeat("a", 64) + `"}`
			exerciseEffectiveCodec[pale.EffectiveProfileEntry](t, row)
			if index == 7 {
				exerciseEffectiveCodec[pale.EffectiveProfileEntry](t, strings.Replace(row, `"terminalProviderBinding":null`, `"terminalProviderBinding":{"providerBindingId":"p","providerBindingIdentity":"i"}`, 1))
			}
			for _, field := range []string{"permittedCapabilities", "forbiddenCapabilities", "contributors", "policies", "responsibilityContractIds", "workDelegationEdgeIds", "providerBindingIds", "capabilityCeiling"} {
				if strings.Contains(row, `"`+field+`":[]`) {
					invalid := strings.Replace(row, `"`+field+`":[]`, `"`+field+`":[null]`, 1)
					var value pale.EffectiveProfileEntry
					if json.Unmarshal([]byte(invalid), &value) == nil {
						t.Fatal("null string-array element accepted")
					}
				}
			}
		})
	}
	for _, fixture := range []string{
		`{"kind":"repository","explanationHandle":"h","repositoryId":"r","materializationOrder":4294967296,"accessCeiling":"read"}`,
		`{"kind":"selection","explanationHandle":"h","selection":{"kind":"session","executionWorkspaceId":"w","activationRootCount":-1,"additionalRootCount":0,"activationPathCount":0}}`,
	} {
		var value pale.EffectiveProfileEntry
		if json.Unmarshal([]byte(fixture), &value) == nil {
			t.Fatal("u32 overflow accepted")
		}
	}
}

func TestEffectiveAuthoringInspectionCodec(t *testing.T) {
	coordinates, _ := profileCoordinateFixture(t)
	encoded, err := json.Marshal(coordinates)
	if err != nil {
		t.Fatal(err)
	}
	fixture := `{"coordinates":` + string(encoded) + `,"entries":[{"kind":"machine-realization","explanationHandle":"first","machineId":"b","realization":"unknown"},{"kind":"machine-realization","explanationHandle":"second","machineId":"a","realization":"unknown"}],"truncated":true}`
	exerciseEffectiveCodec[pale.EffectiveAuthoringInspection](t, fixture)
	var result pale.EffectiveAuthoringInspection
	if err = json.Unmarshal([]byte(fixture), &result); err != nil {
		t.Fatal(err)
	}
	if !result.Truncated || result.Coordinates != coordinates || len(result.Entries) != 2 || result.Entries[0].MachineRealization.MachineID != "b" || result.Entries[1].MachineRealization.MachineID != "a" {
		t.Fatal("owner order/truncation was transformed")
	}
	for _, malformed := range []string{
		strings.Replace(fixture, `"machineId":"b"`, `"machineId":"b","machineId":"private-secret"`, 1),
		strings.Replace(fixture, `"machineId":"b"`, `"machineId":"b","MachineId":"private-secret"`, 1),
	} {
		retained := result
		err = json.Unmarshal([]byte(malformed), &retained)
		if err == nil || strings.Contains(err.Error(), "private-secret") || !reflect.DeepEqual(result, retained) {
			t.Fatal("nested duplicate/alias failed atomic privacy law")
		}
	}
	exerciseEffectiveCodec[pale.EffectiveAuthoringInspection](t, `{"coordinates":`+string(encoded)+`,"entries":[],"truncated":false}`)
}

func TestEffectiveAuthoringNestedCodecs(t *testing.T) {
	for _, fixture := range []string{
		`{"kind":"package","packageId":"p","version":"v","contentDigest":"d"}`, `{"kind":"normative-pack","packId":"p","attestorId":"a","version":"v","contentDigest":"d"}`,
		`{"kind":"semantic-resource","repositoryId":"r","resourceKind":"operation","resourceId":"o"}`, `{"kind":"run-policy","runPolicyId":"p"}`,
		`{"kind":"session","executionWorkspaceId":"w","activationRootCount":1,"additionalRootCount":2,"activationPathCount":3}`,
		`{"kind":"lifecycle","lifecycleId":"l","eventId":"e","actionId":"a"}`, `{"kind":"authority-root","authorityRootId":"r"}`, `{"kind":"disclosure-ceiling","authorityCoordinate":"a"}`,
	} {
		exerciseEffectiveCodec[pale.EffectiveProfileSelection](t, fixture)
	}
	for _, fixture := range []string{`{"kind":"exact","resourceId":"r"}`, `{"kind":"all-from-package","packageId":"p"}`} {
		exerciseEffectiveCodec[pale.EffectiveProfileDeclaredSelector](t, fixture)
	}
	for _, fixture := range []string{`{"kind":"resource","resourceKind":"role","selector":{"kind":"exact","resourceId":"r"}}`, `{"kind":"mode-default","axis":"intensity","resourceId":"r"}`} {
		exerciseEffectiveCodec[pale.EffectiveProfileDeclaredSelection](t, fixture)
	}
	for _, item := range [][2]string{{"interface", "interfaceId"}, {"application", "applicationId"}, {"engine", "engineId"}, {"capability", "capabilityId"}, {"terminal-leaf", "terminalLeafId"}, {"plugin", "pluginId"}, {"skill", "skillId"}} {
		exerciseEffectiveCodec[pale.EffectiveProfileParticipant](t, fmt.Sprintf(`{"kind":%q,%q:"id"}`, item[0], item[1]))
	}
	for _, tag := range []string{"communication", "organizational-work", "crm-business-state", "engineering-execution", "context-memory", "terminal-harness-execution", "native-event-publication", "native-effect-read-back"} {
		exerciseEffectiveCodec[pale.EffectiveProfileResponsibility](t, fmt.Sprintf(`{"kind":%q}`, tag))
	}
	exerciseEffectiveCodec[pale.EffectiveProfileResponsibility](t, `{"kind":"extension","namespaceId":"n","contractId":"c","schemaId":"s","schemaSha256":"h"}`)
	for _, fixture := range []string{`{"kind":"bound","provider":"host-native"}`, `{"kind":"missing-binding"}`, `{"kind":"excluded-by-narrowing"}`, `{"kind":"effect-mismatch"}`, `{"kind":"recipe-contract-mismatch"}`} {
		exerciseEffectiveCodec[pale.EffectiveProfileProviderDisposition](t, fixture)
	}
	for _, fixture := range []string{`{"kind":"native-realization-unknown","machineId":"m"}`, `{"kind":"operation-unbound","repositoryId":"r","operationId":"o"}`, `{"kind":"requirement","repositoryId":"r","operationId":"o","requirementId":"q","capabilityId":"c","disposition":{"kind":"satisfied"}}`} {
		exerciseEffectiveCodec[pale.EffectiveProfileFinding](t, fixture)
	}
	for _, item := range [][2]string{{"origin-receipt", "contractId"}, {"entry-contract", "contractId"}, {"responsibility-contract", "contractId"}, {"provider-binding", "bindingId"}, {"delegation-edge", "edgeId"}, {"termination-contract", "contractId"}, {"settlement-contract", "contractId"}, {"continuity-contract", "contractId"}} {
		exerciseEffectiveCodec[pale.EffectiveProfilePathReason](t, fmt.Sprintf(`{"kind":%q,%q:"id"}`, item[0], item[1]))
	}
	for _, item := range []struct{ kind, fields string }{
		{"source", "sourceId"}, {"component", "componentId"}, {"module", "moduleId"}, {"declaration", "ownerModuleId namespaceId declarationId"}, {"import", "importerModuleId upstreamModuleId upstreamReleaseId exportId sourceNamespaceId localNamespaceId"}, {"extension", "ownerModuleId extensionNamespaceId targetNamespaceId targetDeclarationId"}, {"policy", "ownerModuleId namespaceId declarationId"}, {"responsibility-closure", ""}, {"organizational-translation-closure", ""}, {"experience-composition", ""}, {"resource", "ownerModuleId resourceId"}, {"resolution-lock", ""}, {"universe-lock", ""}, {"workspace", "workspaceId"},
	} {
		wire := map[string]string{"kind": item.kind}
		for _, field := range strings.Fields(item.fields) {
			wire[field] = "id"
		}
		data, _ := json.Marshal(wire)
		exerciseEffectiveCodec[pale.WorkspaceSubjectValue](t, string(data))
	}
}

func TestEffectiveAuthoringScalarCodecs(t *testing.T) {
	for _, tag := range []string{"operation", "role", "doctrine", "startup-policy", "run-mode", "stack", "policy-family", "recipe"} {
		exerciseEffectiveTag[pale.EffectiveProfileResourceKind](t, tag)
	}
	for _, tag := range []string{"intensity", "task-stance"} {
		exerciseEffectiveTag[pale.EffectiveProfileModeAxis](t, tag)
	}
	for _, tag := range []string{"read", "mutate"} {
		exerciseEffectiveTag[pale.EffectiveProfileRepositoryAccess](t, tag)
	}
	for _, tag := range []string{"compute", "read", "execute", "write", "network", "publish"} {
		exerciseEffectiveTag[pale.EffectiveProfileEffectClass](t, tag)
	}
	for _, tag := range []string{"host-native", "native-rust", "connector"} {
		exerciseEffectiveTag[pale.EffectiveProfileProviderKind](t, tag)
	}
	exerciseEffectiveTag[pale.EffectiveProfileNativeRealization](t, "unknown")
}

func exerciseEffectiveTag[T any](t *testing.T, tag string) {
	t.Helper()
	var baseline T
	data, _ := json.Marshal(tag)
	if err := json.Unmarshal(data, &baseline); err != nil {
		t.Fatal(err)
	}
	for _, invalid := range []string{`"private-unknown-tag"`, `{"private-secret":true}`, `null`, `0`} {
		next := baseline
		err := json.Unmarshal([]byte(invalid), &next)
		if err == nil || strings.Contains(err.Error(), "private-") || !reflect.DeepEqual(next, baseline) {
			t.Fatalf("enum rejection/privacy/atomicity failed: %v", err)
		}
	}
}

// Recursively mutate each original field in a fresh JSON tree. Sorted keys keep
// enumeration stable; inserted aliases are never revisited as original fields.
func exerciseEffectiveCodec[T any](t *testing.T, fixture string) {
	t.Helper()
	var baseline T
	if err := json.Unmarshal([]byte(fixture), &baseline); err != nil {
		t.Fatalf("valid codec fixture rejected: %s: %v", fixture, err)
	}
	reject := func(data []byte) {
		t.Helper()
		next := baseline
		err := json.Unmarshal(data, &next)
		if err == nil || strings.Contains(err.Error(), "private-secret") || !reflect.DeepEqual(next, baseline) {
			t.Fatalf("invalid codec value did not fail atomically/redacted: %s: %v", data, err)
		}
	}
	reject([]byte(fixture + ` {}`))
	reject([]byte(`null`))
	var root any
	if err := json.Unmarshal([]byte(fixture), &root); err != nil {
		t.Fatal(err)
	}
	var walk func(any)
	walk = func(node any) {
		switch value := node.(type) {
		case map[string]any:
			keys := make([]string, 0, len(value))
			for key := range value {
				keys = append(keys, key)
			}
			slices.Sort(keys)
			for _, key := range keys {
				original := value[key]
				delete(value, key)
				data, _ := json.Marshal(root)
				reject(data)
				value[key] = original
				alias := strings.ToUpper(key[:1]) + key[1:]
				value[alias] = "private-secret"
				data, _ = json.Marshal(root)
				reject(data)
				delete(value, alias)
				delete(value, key)
				value[alias] = original
				data, _ = json.Marshal(root)
				reject(data)
				delete(value, alias)
				value[key] = original
				if key != "terminalProviderBinding" {
					value[key] = nil
					data, _ = json.Marshal(root)
					reject(data)
					value[key] = original
				}
				value[key] = map[string]any{"private-secret": true}
				data, _ = json.Marshal(root)
				reject(data)
				value[key] = original
				if key == "kind" {
					value[key] = "private-secret"
					data, _ = json.Marshal(root)
					reject(data)
					value[key] = original
				}
				walk(original)
			}
			value["private-secret"] = true
			data, _ := json.Marshal(root)
			reject(data)
			delete(value, "private-secret")
		case []any:
			for index, item := range value {
				value[index] = nil
				data, _ := json.Marshal(root)
				reject(data)
				value[index] = item
				walk(item)
			}
		}
	}
	walk(root)
	// Duplicate a root field before decoding/json can select the last occurrence.
	object := root.(map[string]any)
	keys := make([]string, 0, len(object))
	for key := range object {
		keys = append(keys, key)
	}
	slices.Sort(keys)
	key := keys[0]
	payload, _ := json.Marshal(object[key])
	reject([]byte(strings.TrimSuffix(fixture, "}") + fmt.Sprintf(",%q:%s}", key, payload)))
}
