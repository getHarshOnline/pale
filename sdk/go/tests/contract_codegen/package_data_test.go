package contract_codegen_test

import (
	"encoding/json"
	"testing"

	data "github.com/getHarshOnline/pale/sdk/go/contracts/types/package_data_v1"
)

func TestPackageDataRoundTripsEveryRootVariant(t *testing.T) {
	t.Parallel()

	cases := []string{
		`{"kind":"tool_recipe","exposure":"internal","recipe":{"capability":"repository.read","description":"Read retained input","effect_class":"read","evidence":{"proof_ceiling":"observation","required_fields":["/result"]},"input_mapping":{"path":"/input"},"tool_contract":"reader","tool_operation":"read"}}`,
		`{"kind":"capability_policy","policy":{"permitted_capabilities":["repository.read"],"permitted_effects":["read"],"forbidden_capabilities":["write"]}}`,
		`{"kind":"governing_order","governing_order":{"schema_version":1,"order":[{"purpose":"reasoning","before":{"scope":"local","resource_id":"first"},"after":{"scope":"local","resource_id":"second"},"scope":{"scope":"local","resource_id":"topic"}}]}}`,
		`{"kind":"governing_order","governing_order":{"schema_version":1,"order":[{"purpose":"reasoning","before":{"scope":"local","resource_id":"first","kind":"policy_family"},"after":{"scope":"external","resource":{"package_id":"example.methods","resource_id":"review"},"kind":"policy_family"},"scope":{"scope":"local","resource_id":"topic","kind":"role"}}]}}`,
	}
	for _, source := range cases {
		var expected map[string]any
		if err := json.Unmarshal([]byte(source), &expected); err != nil {
			t.Fatal(err)
		}
		t.Run(expected["kind"].(string), func(t *testing.T) {
			var value data.PackageDataV1
			if err := json.Unmarshal([]byte(source), &value); err != nil {
				t.Fatal(err)
			}
			assertSemanticJSON(t, expected, value)
		})
	}
}

func TestPackageDataRejectsUnknownAndConflictingVariants(t *testing.T) {
	t.Parallel()

	var value data.PackageDataV1
	if err := json.Unmarshal([]byte(`{"kind":"unknown"}`), &value); err == nil {
		t.Fatal("unknown data kind was accepted")
	}
	if _, err := json.Marshal(value); err == nil {
		t.Fatal("empty union was accepted")
	}
	value.ToolRecipe = &data.PackageDataToolRecipeV1{Kind: "tool_recipe"}
	value.CapabilityPolicy = &data.PackageDataCapabilityPolicyV1{Kind: "capability_policy"}
	if _, err := json.Marshal(value); err == nil {
		t.Fatal("conflicting variants were accepted")
	}
}
