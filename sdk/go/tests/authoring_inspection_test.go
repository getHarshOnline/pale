package tests_test

import (
	"encoding/json"
	"errors"
	"math"
	"reflect"
	"strings"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
	workspacecontract "github.com/getHarshOnline/pale/sdk/go/contracts/types/pale_workspace_v1"
)

func TestAuthoringInspectionNativeGrammarParity(t *testing.T) {
	client := newClient(t)
	fixture := readOneShotFixture(t)
	workspace, _, err := client.CompilePortableWorkspace(fixture.Workspace)
	if err != nil {
		t.Fatal(err)
	}
	profile, _, err := workspace.CompileEffectiveProfile(pale.EffectiveProfileRequest{
		ProjectID: fixture.ProjectID, ExecutionWorkspaceID: fixture.ExecutionWorkspaceID,
		MachineID: fixture.MachineID, PrivateInputs: fixture.PrivateInputs, PackageDocuments: fixture.PackageDocuments,
	})
	if err != nil {
		t.Fatal(err)
	}
	routes := []struct {
		name    string
		inspect func([]pale.StructuralPathSegment) (pale.AuthoringInspection, error)
	}{
		{"standalone", func(path []pale.StructuralPathSegment) (pale.AuthoringInspection, error) {
			return client.InspectStandaloneAuthoring(pale.RoleWorkspaceManifest, path)
		}},
		{"workspace", func(path []pale.StructuralPathSegment) (pale.AuthoringInspection, error) {
			return workspace.InspectAuthoring(fixture.Workspace.RootManifest, path)
		}},
		{"profile", func(path []pale.StructuralPathSegment) (pale.AuthoringInspection, error) {
			return profile.InspectAuthoring(fixture.Workspace.RootManifest, path)
		}},
	}
	// These constants are emitted by the existing canonical contract generator.
	// Grammar inspection does not expose the full private workspace schema.
	accessValues := []workspacecontract.ExecutionWorkspaceRepositoryAccessV1{
		workspacecontract.ExecutionWorkspaceRepositoryAccessV1Read,
		workspacecontract.ExecutionWorkspaceRepositoryAccessV1Mutate,
	}
	accessPath := []pale.StructuralPathSegment{pale.AuthoringFieldPath("workspace"), pale.AuthoringFieldPath("desired_topology"), pale.AuthoringFieldPath("execution_workspace_repositories"), pale.AuthoringArrayItemPath(), pale.AuthoringFieldPath("access")}
	var prior pale.AuthoringInspection
	var priorDiagnostic []byte
	for _, route := range routes {
		t.Run(route.name, func(t *testing.T) {
			inspection, err := route.inspect(accessPath)
			if err != nil {
				t.Fatal(err)
			}
			if inspection.State != pale.AuthoringAvailable || inspection.SchemaID == nil || *inspection.SchemaID != "pale-workspace.v1" || inspection.Definition != nil || inspection.Node == nil {
				t.Fatalf("missing exact grammar coordinate: %#v", inspection)
			}
			node := inspection.Node
			if node.Description == nil || *node.Description == "" || len(node.AllowedValues) != len(accessValues) || !reflect.DeepEqual(node.ValueKinds, []pale.AuthoringValueKind{pale.AuthoringString}) {
				t.Fatalf("schema-derived facts drifted: %#v", node)
			}
			for _, expected := range accessValues {
				found := false
				for _, literal := range node.AllowedValues {
					if literal.Value.Kind == pale.AuthoringString && literal.Value.String != nil && *literal.Value.String == string(expected) {
						found = true
						if literal.Value.Integer != nil || literal.Value.Boolean != nil || literal.Description == nil || *literal.Description == "" {
							t.Fatal("literal payload or producer description drifted")
						}
					}
				}
				if !found {
					t.Fatal("schema-declared access literal missing")
				}
			}
			if prior.Node != nil && !reflect.DeepEqual(prior, inspection) {
				t.Fatal("three routes disagree on the same grammar")
			}
			prior = inspection
			id, err := route.inspect([]pale.StructuralPathSegment{pale.AuthoringFieldPath("workspace"), pale.AuthoringFieldPath("workspace_id")})
			if err != nil || id.Node == nil || id.Node.MinLength == nil || *id.Node.MinLength != 1 || id.Node.MaxLength == nil || *id.Node.MaxLength != 255 || id.Node.Pattern == nil || *id.Node.Pattern == "" {
				t.Fatalf("typed identity bounds drifted: %#v %v", id, err)
			}
			unknown, err := route.inspect([]pale.StructuralPathSegment{pale.AuthoringFieldPath("not-a-schema-key")})
			if err != nil || unknown.State != pale.AuthoringUnknownPath || unknown.Node != nil || unknown.SchemaID == nil || *unknown.SchemaID != "pale-workspace.v1" {
				t.Fatalf("unknown grammar path misclassified: %#v %v", unknown, err)
			}
			tooDeep := make([]pale.StructuralPathSegment, 65)
			for index := range tooDeep {
				tooDeep[index] = pale.AuthoringFieldPath("private-source-sentinel")
			}
			_, err = route.inspect(tooDeep)
			var diagnostic *pale.DiagnosticError
			if !errors.As(err, &diagnostic) || diagnostic.Code != "authoring_path_too_deep" {
				t.Fatalf("lost SDK diagnostic: %v", err)
			}
			if priorDiagnostic != nil && !reflect.DeepEqual(priorDiagnostic, diagnostic.Canonical) {
				t.Fatal("same rejected grammar query changed canonical diagnostic across routes")
			}
			priorDiagnostic = diagnostic.Canonical
		})
	}
	for _, inspect := range []func(string, []pale.StructuralPathSegment) (pale.AuthoringInspection, error){workspace.InspectAuthoring, profile.InspectAuthoring} {
		unknown, err := inspect("private-source-sentinel", []pale.StructuralPathSegment{})
		if err != nil || unknown.State != pale.AuthoringUnknownDocument || unknown.Node != nil || unknown.SchemaID != nil || unknown.Definition != nil {
			t.Fatalf("unknown source fabricated grammar: %#v %v", unknown, err)
		}
	}
	text, err := profile.InspectAuthoring("engine/role.md", []pale.StructuralPathSegment{})
	if err != nil || text.State != pale.AuthoringTextDocument || text.Node != nil || text.SchemaID != nil || text.Definition != nil {
		t.Fatalf("retained markdown fabricated grammar: %#v %v", text, err)
	}
	var nilWorkspace *pale.PortableWorkspace
	var nilProfile *pale.EffectiveProfile
	for _, inspect := range []func(string, []pale.StructuralPathSegment) (pale.AuthoringInspection, error){nilWorkspace.InspectAuthoring, nilProfile.InspectAuthoring} {
		_, err := inspect("ignored", nil)
		var bridge *pale.BridgeError
		if !errors.As(err, &bridge) || bridge.Code != "invalid_handle" {
			t.Fatalf("nil authoring receiver did not return invalid_handle: %v", err)
		}
	}
}

func TestAuthoringInspectionRejectsCaseAliases(t *testing.T) {
	for _, input := range []string{
		`{"state":"unknown-document","State":"text-document","schema_id":null,"definition":null,"node":null}`,
		`{"State":"unknown-document","schema_id":null,"definition":null,"node":null}`,
	} {
		original := pale.AuthoringInspection{State: pale.AuthoringAvailable}
		actual := original
		if err := json.Unmarshal([]byte(input), &actual); err == nil || !reflect.DeepEqual(actual, original) {
			t.Errorf("case-aliased inspection was accepted or changed prior value")
		}
	}
	for _, input := range []string{
		`{"kind":"integer","value":1,"Value":2}`,
		`{"kind":"integer","Value":2}`,
	} {
		original := pale.AuthoringLiteral{Kind: pale.AuthoringString}
		actual := original
		if err := json.Unmarshal([]byte(input), &actual); err == nil || !reflect.DeepEqual(actual, original) {
			t.Errorf("case-aliased literal was accepted or changed prior value")
		}
	}
}

func TestAuthoringTagErrorsDoNotDiscloseRejectedValues(t *testing.T) {
	for _, target := range []any{new(pale.AuthoringInspectionState), new(pale.AuthoringValueKind), new(pale.AuthoringFieldPresence)} {
		for _, input := range []string{`918273645`, `"private-source-sentinel"`} {
			err := json.Unmarshal([]byte(input), target)
			if err == nil || strings.Contains(err.Error(), "918273645") || strings.Contains(err.Error(), "private-source-sentinel") {
				t.Fatalf("closed tag returned rejected content: %v", err)
			}
		}
	}
}

func TestAuthoringInspectionClosedDecoding(t *testing.T) {
	const valid = `{"state":"available","schema_id":"grammar.v1","definition":null,"node":{"description":"Δ grammar","value_kinds":["boolean","integer","string","array","table"],"allowed_values":[{"value":{"kind":"string","value":""},"description":null},{"value":{"kind":"integer","value":-9223372036854775808},"description":"minimum"},{"value":{"kind":"boolean","value":false},"description":""}],"fields":[{"name":"first","description":null,"presence":"required","value_kinds":["string"],"allowed_values":[]},{"name":"second","description":"optional child","presence":"optional","value_kinds":[],"allowed_values":[]},{"name":"third","description":null,"presence":"variant-dependent","value_kinds":[],"allowed_values":[]}],"format":null,"pattern":"^x$","minimum":-9223372036854775808,"maximum":9223372036854775807,"min_length":0,"max_length":18446744073709551615,"min_items":null,"max_items":9}}`
	var original pale.AuthoringInspection
	if err := json.Unmarshal([]byte(valid), &original); err != nil {
		t.Fatal(err)
	}
	node := original.Node
	if node == nil || *node.Description != "Δ grammar" || *node.Minimum != math.MinInt64 || *node.Maximum != math.MaxInt64 || *node.MaxLength != math.MaxUint64 || *node.MinLength != 0 || node.MinItems != nil || *node.MaxItems != 9 || *node.AllowedValues[0].Value.String != "" || *node.AllowedValues[1].Value.Integer != math.MinInt64 || *node.AllowedValues[2].Value.Boolean {
		t.Fatal("typed scalar payload, optionality or width lost")
	}
	if node.Fields[0].Name != "first" || node.Fields[2].Presence != pale.AuthoringVariantDependent || node.AllowedValues[0].Description != nil || *node.AllowedValues[2].Description != "" {
		t.Fatal("ordered field/literal documentation lost")
	}
	for _, state := range []pale.AuthoringInspectionState{pale.AuthoringTextDocument, pale.AuthoringUnknownDocument, pale.AuthoringUnknownPath} {
		var absent pale.AuthoringInspection
		if err := json.Unmarshal([]byte(`{"state":"`+string(state)+`","schema_id":null,"definition":null,"node":null}`), &absent); err != nil || absent.State != state || absent.Node != nil {
			t.Fatalf("absent grammar result lost: %#v %v", absent, err)
		}
	}
	invalid := []string{
		`null`, `[]`, valid + ` {}`, strings.Replace(valid, `"state":"available"`, `"state":"private-source-sentinel"`, 1),
		strings.Replace(valid, `"definition":null,`, ``, 1), strings.Replace(valid, `"state":"available",`, ``, 1),
		strings.Replace(valid, `"schema_id":"grammar.v1"`, `"schema_id":"grammar.v1","private-source-sentinel":1`, 1),
		strings.Replace(valid, `"state":"available"`, `"state":"available","state":"available"`, 1),
		strings.Replace(valid, `"description":"Δ grammar"`, `"description":"Δ grammar","description":"private-source-sentinel"`, 1),
		strings.Replace(valid, `"format":null,`, ``, 1), strings.Replace(valid, `"fields":[`, `"fields":null,"ignored":[`, 1),
		strings.Replace(valid, `"value_kinds":["boolean","integer","string","array","table"]`, `"value_kinds":null`, 1),
		strings.Replace(valid, `"allowed_values":[]`, `"allowed_values":null`, 1),
		strings.Replace(valid, `"presence":"required"`, `"presence":"private-source-sentinel"`, 1),
		strings.Replace(valid, `"kind":"string","value":""`, `"kind":"array","value":[]`, 1),
		strings.Replace(valid, `"kind":"string","value":""`, `"kind":"string","value":null`, 1),
		strings.Replace(valid, `"kind":"boolean","value":false`, `"kind":"boolean","value":"private-source-sentinel"`, 1),
		strings.ReplaceAll(valid, `-9223372036854775808`, `-9223372036854775809`),
		strings.Replace(valid, `"minimum":-9223372036854775808`, `"minimum":-9223372036854775808,"Minimum":0`, 1),
		strings.Replace(valid, `"minimum":-9223372036854775808`, `"Minimum":0`, 1),
		strings.Replace(valid, `"maximum":9223372036854775807`, `"maximum":9223372036854775808`, 1),
		strings.Replace(valid, `"max_length":18446744073709551615`, `"max_length":18446744073709551616`, 1),
		strings.Replace(valid, `"min_length":0`, `"min_length":-1`, 1),
		strings.Replace(valid, `"min_length":0`, `"min_length":0.5`, 1),
	}
	for index, input := range invalid {
		value := original
		err := json.Unmarshal([]byte(input), &value)
		if err == nil || !reflect.DeepEqual(value, original) {
			t.Fatalf("malformed case %d accepted or changed prior value", index)
		}
		if strings.Contains(err.Error(), "private-source-sentinel") {
			t.Fatalf("case %d disclosed rejected metadata", index)
		}
	}
}
