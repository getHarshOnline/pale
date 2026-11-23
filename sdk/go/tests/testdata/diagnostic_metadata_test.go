package pale

import (
	"bytes"
	"encoding/json"
	"errors"
	"os"
	"reflect"
	"slices"
	"strings"
	"testing"
)

func TestDiagnosticMetadataPrivateDecoding(t *testing.T) {
	canonical, err := os.ReadFile(os.Getenv("PALE_GO_TEST_FIXTURE"))
	if err != nil {
		t.Fatal(err)
	}
	const correction = `{"kind":"narrow-import-visibility","source_label":"private-source-sentinel","path":[{"kind":"field","value":"module"},{"kind":"index","value":18446744073709551615}],"expected_source":"\"public\"","replacement_source":"\"protected\"","proof_subject":{"kind":"export","value":"owner::surface","authored_value":"surface"},"diagnostic_identity":"opaque-sdk-identity"}`
	const location = `{"source_label":"private-source-sentinel","path":[{"kind":"field","value":"module"}]}`
	valid := []byte(`{"diagnosticIdentity":"opaque-sdk-identity","authoringCorrections":[` + correction + `],"authoringLocation":` + location + `}`)

	reject := func(t *testing.T, data []byte) {
		t.Helper()
		var value, before diagnosticMetadata
		if err := json.Unmarshal(valid, &value); err != nil {
			t.Fatal(err)
		}
		if err := json.Unmarshal(valid, &before); err != nil {
			t.Fatal(err)
		}
		if err := json.Unmarshal(data, &value); err == nil || strings.Contains(err.Error(), "private-source-sentinel") {
			t.Fatalf("malformed metadata accepted or disclosed: %v", err)
		}
		if !reflect.DeepEqual(value, before) {
			t.Fatal("metadata rejection changed the receiver")
		}
		var failure *BridgeError
		actual := newDiagnosticError(canonical, data)
		if !errors.As(actual, &failure) || failure.Code != "invalid_response_metadata" || strings.Contains(actual.Error(), "private-source-sentinel") {
			t.Fatalf("malformed metadata became a partial diagnostic: %v", actual)
		}
	}
	for _, data := range []string{
		"", "null", "[]", "{}", string(valid) + " {}",
		strings.Replace(string(valid), `"diagnosticIdentity":`, `"diagnosticIdentity":"duplicate","diagnosticIdentity":`, 1),
		strings.Replace(string(valid), `"diagnosticIdentity":`, `"unexpected":"private-source-sentinel","diagnosticIdentity":`, 1),
		strings.Replace(string(valid), correction, "null", 1),
		strings.Replace(string(valid), `"kind":"narrow-import-visibility"`, `"kind":"private-source-sentinel"`, 1),
		strings.Replace(string(valid), `"expected_source":`, `"expected_source":"duplicate","expected_source":`, 1),
		strings.Replace(string(valid), `"kind":"field","value":"module"`, `"kind":"field","value":null`, 1),
		strings.Replace(string(valid), `"kind":"field","value":"module"`, `"kind":"field","value":17`, 1),
		strings.Replace(string(valid), `"value":18446744073709551615`, `"value":18446744073709551616`, 1),
		strings.Replace(string(valid), `"value":18446744073709551615`, `"value":-1`, 1),
		strings.Replace(string(valid), `"value":18446744073709551615`, `"value":0.5`, 1),
		strings.Replace(string(valid), `"kind":"export"`, `"kind":"private-source-sentinel"`, 1),
		strings.Replace(string(valid), `"authored_value":"surface"`, `"authored_value":null`, 1),
		strings.Replace(string(valid), `"authored_value":"surface"`, `"authored_value":"surface","Authored_value":"private-source-sentinel"`, 1),
		strings.Replace(string(valid), `"authoringLocation":`+location, `"authoringLocation":{"source_label":"private-source-sentinel","path":[null]}`, 1),
		strings.Replace(string(valid), `"authoringLocation":`+location, `"authoringLocation":{"source_label":"private-source-sentinel","path":null}`, 1),
		strings.Replace(string(valid), `"authoringLocation":`+location, `"authoringLocation":{"source_label":"private-source-sentinel","Source_label":"alias","path":[]}`, 1),
	} {
		reject(t, []byte(data))
	}

	for _, name := range []string{"diagnosticIdentity", "authoringCorrections", "authoringLocation"} {
		for _, mode := range []string{"missing", "wrong-type", "case-only", "case-alias", "null"} {
			if name == "authoringLocation" && mode == "null" {
				continue
			}
			t.Run(name+"/"+mode, func(t *testing.T) {
				var fields map[string]json.RawMessage
				if err := json.Unmarshal(valid, &fields); err != nil {
					t.Fatal(err)
				}
				alias := strings.ToUpper(name[:1]) + name[1:]
				switch mode {
				case "missing":
					delete(fields, name)
				case "wrong-type":
					fields[name] = json.RawMessage(`0`)
				case "case-only":
					fields[alias] = fields[name]
					delete(fields, name)
				case "case-alias":
					fields[alias] = fields[name]
				case "null":
					fields[name] = json.RawMessage(`null`)
				}
				data, err := json.Marshal(fields)
				if err != nil {
					t.Fatal(err)
				}
				reject(t, data)
			})
		}
	}

	var correctionFields map[string]json.RawMessage
	if err := json.Unmarshal([]byte(correction), &correctionFields); err != nil {
		t.Fatal(err)
	}
	names := make([]string, 0, len(correctionFields))
	for name := range correctionFields {
		names = append(names, name)
	}
	slices.Sort(names)
	for _, name := range names {
		for _, mode := range []string{"missing", "null", "wrong-type", "case-only", "case-alias"} {
			t.Run("correction/"+name+"/"+mode, func(t *testing.T) {
				var fields map[string]json.RawMessage
				if err := json.Unmarshal([]byte(correction), &fields); err != nil {
					t.Fatal(err)
				}
				alias := strings.ToUpper(name[:1]) + name[1:]
				switch mode {
				case "missing":
					delete(fields, name)
				case "null":
					fields[name] = json.RawMessage(`null`)
				case "wrong-type":
					fields[name] = json.RawMessage(`0`)
				case "case-only":
					fields[alias] = fields[name]
					delete(fields, name)
				case "case-alias":
					fields[alias] = fields[name]
				}
				data, err := json.Marshal(fields)
				if err != nil {
					t.Fatal(err)
				}
				var value, before AuthoringCorrection
				if err := json.Unmarshal([]byte(correction), &value); err != nil {
					t.Fatal(err)
				}
				if err := json.Unmarshal([]byte(correction), &before); err != nil {
					t.Fatal(err)
				}
				if err := json.Unmarshal(data, &value); err == nil || !reflect.DeepEqual(value, before) {
					t.Fatal("invalid correction accepted or changed receiver")
				}
				reject(t, []byte(strings.Replace(string(valid), correction, string(data), 1)))
			})
		}
	}

	for _, data := range []string{
		string(valid),
		strings.Replace(string(valid), `"authoringLocation":`+location, `"authoringLocation":null`, 1),
		`{"diagnosticIdentity":"opaque-sdk-identity","authoringCorrections":[],"authoringLocation":null}`,
		strings.Replace(string(valid), `"path":[{"kind":"field","value":"module"},{"kind":"index","value":18446744073709551615}]`, `"path":[]`, 1),
	} {
		var failure *DiagnosticError
		actual := newDiagnosticError(canonical, []byte(data))
		if !errors.As(actual, &failure) || failure.Identity != "opaque-sdk-identity" || failure.AuthoringCorrections == nil || !bytes.Equal(failure.Canonical, canonical) {
			t.Fatalf("valid sidecars lost owned values: %v", actual)
		}
		if failure.Error() != failure.Code+": "+failure.Summary {
			t.Fatal("sidecar evidence changed Error()")
		}
	}
	var decoded diagnosticMetadata
	if err := json.Unmarshal(valid, &decoded); err != nil {
		t.Fatal(err)
	}
	if decoded.AuthoringCorrections[0].Path[1].Index == nil || *decoded.AuthoringCorrections[0].Path[1].Index != ^uint64(0) {
		t.Fatal("output path index was narrowed")
	}
}
