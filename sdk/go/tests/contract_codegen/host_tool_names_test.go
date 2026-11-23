package contract_codegen_test

import (
	"encoding/json"
	capabilities "github.com/getHarshOnline/pale/sdk/go/contracts/types/capabilities_v1"
	operation "github.com/getHarshOnline/pale/sdk/go/contracts/types/operation_contract_v1"
	"testing"
)

func TestModelAndNativeToolNamesSurviveLeafAndEnclosingContract(t *testing.T) {
	tool := map[string]any{
		"name": "Bash", "model_name": "exec_command",
		"observability": "pre_tool_observable",
	}
	encoded, err := json.Marshal(tool)
	if err != nil {
		t.Fatal(err)
	}
	var leaf operation.HostToolBinding
	if err := json.Unmarshal(encoded, &leaf); err != nil {
		t.Fatal(err)
	}
	assertSemanticJSON(t, tool, leaf)

	source := map[string]any{"capability_bindings": map[string]any{
		"inspect": map[string]any{
			"approval": "none", "effect_class": "read",
			"implementation": map[string]any{
				"kind": "host_native", "provider_tools": []any{
					map[string]any{
						"provider_binding": map[string]any{
							"provider_binding_id": "synthetic-terminal",
							"provider_binding_identity": map[string]any{
								"provider_binding_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
							},
						},
						"tools": []any{tool},
					},
				},
			},
		},
	}}
	encoded, err = json.Marshal(source)
	if err != nil {
		t.Fatal(err)
	}
	var record capabilities.CapabilitiesV1
	if err := json.Unmarshal(encoded, &record); err != nil {
		t.Fatal(err)
	}
	assertSemanticJSON(t, source, record)
}
