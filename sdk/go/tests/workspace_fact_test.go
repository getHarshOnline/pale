package tests_test

import (
	"encoding/json"
	"reflect"
	"strings"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func TestWorkspaceFactDecodesEverySelectedOutput(t *testing.T) {
	cases := []struct {
		wire string
		want pale.WorkspaceFactValue
	}{
		{`{"kind":"source_content","sourcePath":"SourcePath","sourceDigest":"SourceDigest","byteLen":4294967295}`, pale.SourceContentWorkspaceFact{SourcePath: "SourcePath", SourceDigest: "SourceDigest", ByteLen: ^uint32(0)}},
		{`{"kind":"component","componentDigest":"ComponentDigest"}`, pale.ComponentWorkspaceFact{ComponentDigest: "ComponentDigest"}},
		{`{"kind":"module","moduleDigest":"ModuleDigest"}`, pale.ModuleWorkspaceFact{ModuleDigest: "ModuleDigest"}},
		{`{"kind":"declaration","valueDigest":"ValueDigest"}`, pale.DeclarationWorkspaceFact{ValueDigest: "ValueDigest"}},
		{`{"kind":"import_surface","surfaceDigest":"SurfaceDigest"}`, pale.ImportSurfaceWorkspaceFact{SurfaceDigest: "SurfaceDigest"}},
		{`{"kind":"extension","valueDigest":"ValueDigest"}`, pale.ExtensionWorkspaceFact{ValueDigest: "ValueDigest"}},
		{`{"kind":"effective_policy","valueDigest":"ValueDigest"}`, pale.EffectivePolicyWorkspaceFact{ValueDigest: "ValueDigest"}},
		{`{"kind":"responsibility_closure","closureDigest":"ClosureDigest"}`, pale.ResponsibilityClosureWorkspaceFact{ClosureDigest: "ClosureDigest"}},
		{`{"kind":"organizational_translation_closure","closureDigest":"ClosureDigest"}`, pale.OrganizationalTranslationClosureWorkspaceFact{ClosureDigest: "ClosureDigest"}},
		{`{"kind":"experience_composition","sourceDigest":"SourceDigest"}`, pale.ExperienceCompositionWorkspaceFact{SourceDigest: "SourceDigest"}},
		{`{"kind":"resource","valueDigest":"ValueDigest"}`, pale.ResourceWorkspaceFact{ValueDigest: "ValueDigest"}},
		{`{"kind":"resolution_lock","resolutionDigest":"ResolutionDigest"}`, pale.ResolutionLockWorkspaceFact{ResolutionDigest: "ResolutionDigest"}},
		{`{"kind":"universe_lock","universeDigest":"UniverseDigest"}`, pale.UniverseLockWorkspaceFact{UniverseDigest: "UniverseDigest"}},
		{`{"kind":"semantic_ir","workspaceSemanticIrDigest":"WorkspaceSemanticIRDigest"}`, pale.SemanticIRWorkspaceFact{WorkspaceSemanticIRDigest: "WorkspaceSemanticIRDigest"}},
	}
	for _, item := range cases {
		t.Run(reflect.TypeOf(item.want).Name(), func(t *testing.T) {
			var value pale.WorkspaceFact
			if err := json.Unmarshal([]byte(item.wire), &value); err != nil || !reflect.DeepEqual(value.Value, item.want) {
				t.Fatalf("selected payload changed: %#v %v", value, err)
			}
			// Each required field is separately removed, nulled, case-folded,
			// duplicated, and given a wrong type. No variant relies on another.
			var fields map[string]json.RawMessage
			decode(t, []byte(item.wire), &fields)
			for name := range fields {
				for _, mode := range []string{"missing", "null", "case", "duplicate", "type"} {
					t.Run(name+"/"+mode, func(t *testing.T) {
						copyFields := make(map[string]json.RawMessage, len(fields))
						for key, field := range fields {
							copyFields[key] = field
						}
						switch mode {
						case "missing":
							delete(copyFields, name)
						case "null":
							copyFields[name] = json.RawMessage("null")
						case "case":
							delete(copyFields, name)
							copyFields[strings.ToUpper(name)] = fields[name]
						case "type":
							copyFields[name] = json.RawMessage("true")
						}
						wire, err := json.Marshal(copyFields)
						if err != nil {
							t.Fatal(err)
						}
						if mode == "duplicate" {
							wire = append(wire[:len(wire)-1], []byte(",\""+name+"\":"+string(fields[name])+"}")...)
						}
						before := value
						if err := json.Unmarshal(wire, &value); err == nil || !reflect.DeepEqual(value, before) {
							t.Fatalf("malformed payload was accepted or changed output: %s", wire)
						}
					})
				}
			}
			for _, wire := range []string{
				strings.TrimSuffix(item.wire, "}") + ",\"extra\":true}",
				item.wire + " {}",
				strings.Replace(item.wire, "\"kind\":", "\"kind\":\"unknown\",\"other\":", 1),
			} {
				before := value
				if err := json.Unmarshal([]byte(wire), &value); err == nil || !reflect.DeepEqual(value, before) {
					t.Fatal("invalid fact changed output")
				}
			}
		})
	}
	for _, wire := range []string{
		"null", "[]", "{}", `{"kind":"unknown"}`,
		`{"kind":"source_content","sourcePath":"p","sourceDigest":"d","byteLen":-1}`,
		`{"kind":"source_content","sourcePath":"p","sourceDigest":"d","byteLen":1.5}`,
		`{"kind":"source_content","sourcePath":"p","sourceDigest":"d","byteLen":4294967296}`,
	} {
		if err := json.Unmarshal([]byte(wire), new(pale.WorkspaceFact)); err == nil {
			t.Fatal("invalid fact accepted")
		}
	}
}

func TestWorkspaceSubjectComparisonDecodesMembershipAtomically(t *testing.T) {
	fact := `{"kind":"module","moduleDigest":"module" }`
	values := []struct {
		wire string
		want pale.WorkspaceSubjectMembership
	}{
		{`{"membership":"absent"}`, pale.AbsentWorkspaceSubject{}},
		{`{"membership":"prior_only","priorFact":` + fact + `}`, pale.PriorOnlyWorkspaceSubject{PriorFact: pale.WorkspaceFact{Value: pale.ModuleWorkspaceFact{ModuleDigest: "module"}}}},
		{`{"membership":"current_only","currentFact":` + fact + `}`, pale.CurrentOnlyWorkspaceSubject{CurrentFact: pale.WorkspaceFact{Value: pale.ModuleWorkspaceFact{ModuleDigest: "module"}}}},
		{`{"membership":"shared","priorFact":` + fact + `,"currentFact":` + fact + `,"comparison":"equal"}`, pale.SharedWorkspaceSubject{PriorFact: pale.WorkspaceFact{Value: pale.ModuleWorkspaceFact{ModuleDigest: "module"}}, CurrentFact: pale.WorkspaceFact{Value: pale.ModuleWorkspaceFact{ModuleDigest: "module"}}, Comparison: pale.WorkspaceValuesEqual}},
		{`{"membership":"shared","priorFact":` + fact + `,"currentFact":` + fact + `,"comparison":"changed"}`, pale.SharedWorkspaceSubject{PriorFact: pale.WorkspaceFact{Value: pale.ModuleWorkspaceFact{ModuleDigest: "module"}}, CurrentFact: pale.WorkspaceFact{Value: pale.ModuleWorkspaceFact{ModuleDigest: "module"}}, Comparison: pale.WorkspaceValuesChanged}},
	}
	for _, item := range values {
		var value pale.WorkspaceSubjectValues
		if err := json.Unmarshal([]byte(item.wire), &value); err != nil || !reflect.DeepEqual(value.Value, item.want) {
			t.Fatalf("membership payload changed: %#v %v", value, err)
		}
		var fields map[string]json.RawMessage
		decode(t, []byte(item.wire), &fields)
		for name := range fields {
			for _, mode := range []string{"missing", "null", "case", "duplicate"} {
				copyFields := make(map[string]json.RawMessage, len(fields))
				for key, field := range fields {
					copyFields[key] = field
				}
				switch mode {
				case "missing":
					delete(copyFields, name)
				case "null":
					copyFields[name] = json.RawMessage("null")
				case "case":
					delete(copyFields, name)
					copyFields[strings.ToUpper(name)] = fields[name]
				}
				wire, _ := json.Marshal(copyFields)
				if mode == "duplicate" {
					wire = append(wire[:len(wire)-1], []byte(",\""+name+"\":"+string(fields[name])+"}")...)
				}
				before := value
				if err := json.Unmarshal(wire, &value); err == nil || !reflect.DeepEqual(value, before) {
					t.Fatalf("invalid %s %s accepted", name, mode)
				}
			}
		}
	}
	valid := `{"receiptDigest":"receipt","subject":{"kind":"workspace","workspaceId":"w"},"values":` + values[3].wire + `}`
	var value pale.WorkspaceSubjectComparison
	if err := json.Unmarshal([]byte(valid), &value); err != nil {
		t.Fatal(err)
	}
	for _, invalid := range []string{
		"null", "{}", valid + " {}",
		strings.Replace(valid, `"receiptDigest":"receipt"`, `"receiptDigest":null`, 1),
		strings.Replace(valid, `"receiptDigest"`, `"ReceiptDigest"`, 1),
		strings.Replace(valid, `"receiptDigest":"receipt"`, `"receiptDigest":"first","receiptDigest":"second"`, 1),
		strings.Replace(valid, `"workspaceId":"w"`, `"workspaceId":"w","extra":true`, 1),
		strings.Replace(valid, `"moduleDigest":"module"`, `"moduleDigest":null`, 1),
		strings.Replace(valid, `"comparison":"equal"`, `"comparison":"affected"`, 1),
		strings.Replace(valid, `"comparison":"equal"`, `"comparison":null`, 1),
		strings.Replace(valid, `"membership":"shared"`, `"membership":"prior_only"`, 1),
		strings.Replace(valid, `"membership":"shared"`, `"membership":"unknown"`, 1),
		strings.Replace(valid, `"values":`+values[3].wire, `"values":null`, 1),
	} {
		before := value
		if err := json.Unmarshal([]byte(invalid), &value); err == nil || !reflect.DeepEqual(value, before) {
			t.Fatalf("invalid nested result changed output: %s", invalid)
		}
	}
}

func TestPortableSubjectTagsDoNotBecomeProfileAliases(t *testing.T) {
	for _, item := range []struct {
		portable string
		profile  pale.WorkspaceSubjectValueKind
	}{
		{"responsibility_closure", pale.WorkspaceSubjectValueResponsibilityClosureKind},
		{"organizational_translation_closure", pale.WorkspaceSubjectValueOrganizationalTranslationClosureKind},
		{"experience_composition", pale.WorkspaceSubjectValueExperienceCompositionKind},
		{"resolution_lock", pale.WorkspaceSubjectValueResolutionLockKind},
		{"universe_lock", pale.WorkspaceSubjectValueUniverseLockKind},
	} {
		t.Run(item.portable, func(t *testing.T) {
			wrap := func(subject string) []byte {
				return []byte(`{"receiptDigest":"receipt","subject":` + subject + `,"values":{"membership":"absent"}}`)
			}
			portable := `{"kind":"` + item.portable + `"}`
			profile := `{"kind":"` + string(item.profile) + `"}`
			var result pale.WorkspaceSubjectComparison
			if err := json.Unmarshal(wrap(portable), &result); err != nil || result.Subject.Kind != item.profile {
				t.Fatalf("portable tag lost typed identity: %#v %v", result, err)
			}
			var profileResult pale.WorkspaceSubjectValue
			if err := json.Unmarshal([]byte(profile), &profileResult); err != nil || !reflect.DeepEqual(result.Subject, profileResult) {
				t.Fatal("shared semantic payload changed", err)
			}
			beforeProfile := profileResult
			if err := json.Unmarshal([]byte(portable), &profileResult); err == nil || !reflect.DeepEqual(profileResult, beforeProfile) {
				t.Fatal("profile decoder accepted portable dialect")
			}
			for _, invalid := range []string{
				profile, `null`, `{}`, `{"kind":null}`, `{"Kind":"` + item.portable + `"}`,
				`{"kind":"` + item.portable + `","kind":"` + item.portable + `"}`,
				`{"kind":"` + item.portable + `","extra":true}`,
			} {
				before := result
				if err := json.Unmarshal(wrap(invalid), &result); err == nil || !reflect.DeepEqual(result, before) {
					t.Fatalf("invalid portable subject changed result: %s", invalid)
				}
			}
		})
	}
	// Record-bearing subjects retain the shared strict payload decoder, not a
	// second record codec. Assert every field of a compound import coordinate.
	importSubject := `{"kind":"import","importerModuleId":"importer","upstreamModuleId":"upstream","upstreamReleaseId":"release","exportId":"export","sourceNamespaceId":"source","localNamespaceId":"local"}`
	var profile pale.WorkspaceSubjectValue
	if err := json.Unmarshal([]byte(importSubject), &profile); err != nil {
		t.Fatal(err)
	}
	var result pale.WorkspaceSubjectComparison
	wire := []byte(`{"receiptDigest":"receipt","subject":` + importSubject + `,"values":{"membership":"absent"}}`)
	if err := json.Unmarshal(wire, &result); err != nil || !reflect.DeepEqual(profile, result.Subject) {
		t.Fatal("portable compound subject lost full payload", err)
	}
	for _, invalid := range []string{
		strings.Replace(string(wire), `"upstreamReleaseId":"release"`, `"upstreamReleaseId":null`, 1),
		strings.Replace(string(wire), `"upstreamReleaseId":"release"`, `"upstreamReleaseId":"release","upstreamReleaseId":"other"`, 1),
		strings.Replace(string(wire), `"upstreamReleaseId"`, `"UpstreamReleaseId"`, 1),
		strings.Replace(string(wire), `"upstreamReleaseId":"release"`, `"extra":"release"`, 1),
	} {
		before := result
		if err := json.Unmarshal([]byte(invalid), &result); err == nil || !reflect.DeepEqual(result, before) {
			t.Fatal("compound portable subject accepted invalid payload")
		}
	}
}
