package pale

import (
	"encoding/json"
	"reflect"
	"slices"
	"strings"
	"testing"
)

// Synthetic transport records exercise the real private decoder; they do not
// claim that a native compiler emitted these identities or custody coordinates.
func TestAuthoringNavigationResponseDecoder(t *testing.T) {
	const valid = `{"navigation":{"identity":{"kind":"project","value":"demo","authored_value":"demo"},"occurrence_role":"reference","definition":{"source_label":"workspace.toml","path":[{"kind":"field","value":"id"}]},"references":[],"alternatives":[{"kind":"project","value":"demo","authored_value":"demo"},{"kind":"project","value":"other","authored_value":"other"}],"references_truncated":false,"alternatives_truncated":false},"alternativeHandles":[{"alternativeIndex":0,"handle":12},{"alternativeIndex":1,"handle":13}]}`
	baseline, err := decodeAuthoringNavigationResponse(response{object: 11, metadata: []byte(valid)})
	if err != nil {
		t.Fatal(err)
	}
	ordered := strings.Replace(valid, `"references":[]`, `"references":[{"source_label":"b","path":[]},{"source_label":"a","path":[]}]`, 1)
	ordered = strings.ReplaceAll(ordered, `truncated":false`, `truncated":true`)
	copied, err := decodeAuthoringNavigationResponse(response{object: 11, metadata: []byte(ordered)})
	if err != nil || copied.Navigation.References[0].SourceLabel != "b" || copied.Navigation.References[1].SourceLabel != "a" || !copied.Navigation.ReferencesTruncated || !copied.Navigation.AlternativesTruncated {
		t.Fatal("decoder reordered references or recomputed truncation")
	}
	checkRejected := func(t *testing.T, encoded []byte, object uint64) {
		t.Helper()
		actual, err := decodeAuthoringNavigationResponse(response{object: object, metadata: encoded})
		if err == nil || !reflect.DeepEqual(actual, authoringNavigationEnvelope{}) || strings.Contains(err.Error(), "private-marker") {
			t.Fatalf("decoder accepted, partially returned or leaked: %v", err)
		}
	}
	fresh := func() map[string]any {
		var value map[string]any
		if err := json.Unmarshal([]byte(valid), &value); err != nil {
			t.Fatal(err)
		}
		return value
	}
	mutate := func(t *testing.T, update func(map[string]any), object uint64) {
		t.Helper()
		value := fresh()
		update(value)
		encoded, err := json.Marshal(value)
		if err != nil {
			t.Fatal(err)
		}
		checkRejected(t, encoded, object)
	}
	for _, test := range []struct {
		name   string
		update func(map[string]any)
	}{
		{"missing alternative", func(v map[string]any) { v["alternativeHandles"] = v["alternativeHandles"].([]any)[:1] }},
		{"extra alternative", func(v map[string]any) {
			v["alternativeHandles"] = append(v["alternativeHandles"].([]any), map[string]any{"alternativeIndex": 2, "handle": 14})
		}},
		{"zero alternative", func(v map[string]any) { v["alternativeHandles"].([]any)[0].(map[string]any)["handle"] = 0 }},
		{"reused resolved handle", func(v map[string]any) { v["alternativeHandles"].([]any)[0].(map[string]any)["handle"] = 11 }},
		{"duplicate alternative handle", func(v map[string]any) { v["alternativeHandles"].([]any)[1].(map[string]any)["handle"] = 12 }},
		{"wrong index", func(v map[string]any) { v["alternativeHandles"].([]any)[0].(map[string]any)["alternativeIndex"] = 1 }},
		{"reordered mapping", func(v map[string]any) { a := v["alternativeHandles"].([]any); a[0], a[1] = a[1], a[0] }},
		{"identity absent with handle", func(v map[string]any) { v["navigation"].(map[string]any)["identity"] = nil }},
	} {
		t.Run(test.name, func(t *testing.T) { mutate(t, test.update, 11) })
	}
	checkRejected(t, []byte(valid), 0)
	checkRejected(t, []byte(valid+` {}`), 11)
	checkRejected(t, []byte(strings.Replace(valid, `"occurrence_role":"reference"`, `"occurrence_role":"reference","occurrence_role":"private-marker"`, 1)), 11)

	// Each selected record exercises its exact field set independently. Only the
	// SDK's three explicit nullable navigation members accept null.
	records := []struct {
		name         string
		selectRecord func(map[string]any) map[string]any
		nullable     map[string]bool
	}{
		{"envelope", func(v map[string]any) map[string]any { return v }, nil},
		{"navigation", func(v map[string]any) map[string]any { return v["navigation"].(map[string]any) }, map[string]bool{"identity": true, "occurrence_role": true, "definition": true}},
		{"identity", func(v map[string]any) map[string]any {
			return v["navigation"].(map[string]any)["identity"].(map[string]any)
		}, nil},
		{"definition", func(v map[string]any) map[string]any {
			return v["navigation"].(map[string]any)["definition"].(map[string]any)
		}, nil},
		{"path", func(v map[string]any) map[string]any {
			return v["navigation"].(map[string]any)["definition"].(map[string]any)["path"].([]any)[0].(map[string]any)
		}, nil},
		{"mapping", func(v map[string]any) map[string]any { return v["alternativeHandles"].([]any)[0].(map[string]any) }, nil},
	}
	for _, record := range records {
		keys := make([]string, 0)
		for key := range record.selectRecord(fresh()) {
			keys = append(keys, key)
		}
		slices.Sort(keys)
		for _, key := range keys {
			for _, mode := range []string{"missing", "null", "alias", "alias-only"} {
				if mode == "null" && record.nullable[key] {
					continue
				}
				t.Run(record.name+"/"+key+"/"+mode, func(t *testing.T) {
					mutate(t, func(v map[string]any) {
						object := record.selectRecord(v)
						switch mode {
						case "missing":
							delete(object, key)
						case "null":
							object[key] = nil
						case "alias":
							object[strings.ToUpper(key[:1])+key[1:]] = "private-marker"
						case "alias-only":
							object[strings.ToUpper(key[:1])+key[1:]] = object[key]
							delete(object, key)
						}
					}, 11)
				})
			}
		}
	}
	absent := fresh()
	value := absent["navigation"].(map[string]any)
	value["identity"], value["occurrence_role"], value["definition"] = nil, nil, nil
	encoded, err := json.Marshal(absent)
	if err != nil {
		t.Fatal(err)
	}
	completion, err := decodeAuthoringNavigationResponse(response{metadata: encoded})
	if err != nil || completion.Navigation.Identity != nil || len(completion.Navigation.Alternatives) != 2 {
		t.Fatalf("absence with alternatives rejected: %v", err)
	}
	atomic := baseline
	if err := json.Unmarshal([]byte(`{"navigation":null,"alternativeHandles":[]}`), &atomic); err == nil || !reflect.DeepEqual(atomic, baseline) {
		t.Fatal("envelope decoder mutated on failure")
	}
}
