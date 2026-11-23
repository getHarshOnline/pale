package main

import (
	"bytes"
	"encoding/json"
	"go/format"
	"testing"

	"github.com/atombender/go-jsonschema/pkg/schemas"
)

func TestObjectSumProjectionClosesUpstreamFirstBranchMerge(t *testing.T) {
	schema := []byte(`{
        "$defs": {
            "ChoiceV1": {
                "oneOf": [
                    {"type":"object","properties":{"kind":{"type":"string","const":"first"}},"required":["kind"]},
                    {"type":"object","properties":{"kind":{"type":"string","const":"second"},"value":{"type":"string"}},"required":["kind","value"]}
                ]
            }
        },
        "type":"object",
        "properties":{"choice":{"$ref":"#/$defs/ChoiceV1"}},
        "required":["choice"]
    }`)
	projected, sums, err := projectGeneratorInput(schema, "FixtureV1")
	if err != nil {
		t.Fatal(err)
	}
	if len(sums) != 1 || sums[0].name != "ChoiceV1" || len(sums[0].variants) != 2 {
		t.Fatalf("unexpected object-sum inventory: %#v", sums)
	}
	var value map[string]any
	if err := json.Unmarshal(projected, &value); err != nil {
		t.Fatal(err)
	}
	choice := value["$defs"].(map[string]any)["ChoiceV1"].(map[string]any)
	if _, ok := choice["anyOf"]; !ok {
		t.Fatal("generator projection did not expose named branches to go-jsonschema")
	}

	upstream := []byte(`package fixture
type ChoiceFirstV1 struct { Kind string ` + "`json:\"kind\"`" + ` }
type ChoiceSecondV1 struct { Kind string ` + "`json:\"kind\"`" + `; Value string ` + "`json:\"value\"`" + ` }
type Root struct { Choice ChoiceFirstV1 ` + "`json:\"choice\"`" + ` }
`)
	closed, err := closeObjectSums(upstream, sums)
	if err != nil {
		t.Fatal(err)
	}
	closed, err = format.Source(closed)
	if err != nil {
		t.Fatal(err)
	}
	for _, expected := range [][]byte{
		[]byte("Choice ChoiceV1"),
		[]byte("type ChoiceV1 struct"),
		[]byte("*ChoiceFirstV1"),
		[]byte("*ChoiceSecondV1"),
		[]byte("func (value *ChoiceV1) UnmarshalJSON"),
	} {
		if !bytes.Contains(closed, expected) {
			t.Fatalf("closed projection is missing %q:\n%s", expected, closed)
		}
	}
	for _, forbidden := range [][]byte{
		[]byte("Choice ChoiceFirstV1"),
		[]byte("interface{}"),
		[]byte("any"),
		[]byte("json.RawMessage"),
	} {
		if bytes.Contains(closed, forbidden) {
			t.Fatalf("closed projection retained %q:\n%s", forbidden, closed)
		}
	}
}

func TestRepeatedVersionDeclarationNameRejectsRepeatedVersion(t *testing.T) {
	t.Parallel()

	repeatedVersion := []byte("package fixture\ntype TypedDigestV1 struct{}\ntype TypedDigestV1TypedDigestCanonicalizationV1 struct{}\n")
	if name, err := repeatedVersionDeclarationName(repeatedVersion); err != nil || name != "TypedDigestV1TypedDigestCanonicalizationV1" {
		t.Fatalf("got name %q and error %v", name, err)
	}
	good := []byte("package fixture\ntype SHA256DigestV1 string\ntype HTTP2PolicyV1 string\n")
	if name, err := repeatedVersionDeclarationName(good); err != nil || name != "" {
		t.Fatalf("got name %q and error %v", name, err)
	}
}

func TestRootObjectSumRetainsEveryTaggedVariant(t *testing.T) {
	schema := []byte(`{"$id":"https://example.com/choice.v1.json","title":"ChoiceV1","oneOf":[
      {"type":"object","additionalProperties":false,"properties":{"kind":{"type":"string","const":"first"},"text":{"type":"string"}},"required":["kind","text"]},
      {"type":"object","additionalProperties":false,"properties":{"kind":{"type":"string","const":"second"},"count":{"type":"integer"}},"required":["kind","count"]}
    ]}`)
	projected, sums, err := projectGeneratorInput(schema, "ChoiceV1")
	if err != nil {
		t.Fatal(err)
	}
	parsed, err := schemas.FromJSONReader(bytes.NewReader(projected))
	if err != nil {
		t.Fatal(err)
	}
	outputs, err := generateModels([]loadedSchema{{family: "choice.v1", id: "https://example.com/choice.v1.json", path: "choice.v1.json", schema: parsed, objectSums: sums}})
	if err != nil {
		t.Fatal(err)
	}
	source := outputs["types/choice_v1/contract.go"]
	for _, expected := range []string{"type ChoiceV1 struct", "*ChoiceFirstV1", "*ChoiceSecondV1", "Text string", "Count int", "func (value *ChoiceV1) UnmarshalJSON"} {
		if !bytes.Contains(source, []byte(expected)) {
			t.Fatalf("missing %q in generated union:\n%s", expected, source)
		}
	}
}
