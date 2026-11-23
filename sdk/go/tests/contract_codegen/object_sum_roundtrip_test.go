package contract_codegen_test

import (
	"encoding/json"
	"reflect"
	"strings"
	"testing"

	checkpoint "github.com/getHarshOnline/pale/sdk/go/contracts/types/checkpoint_signing_authority_v1"
	receipt "github.com/getHarshOnline/pale/sdk/go/contracts/types/local_admin_receipt_v1"
)

func TestLocalAdminHeadTransitionRoundTripsEverySelectorVariant(t *testing.T) {
	t.Parallel()

	digest := strings.Repeat("a", 64)
	selectors := []map[string]any{
		selector("principal_record", "principal_id", "principal"),
		selector("principal_recovery_policy", "principal_id", "principal"),
		selector("local_admin_authority", "principal_id", "principal"),
		selector("producer_incarnation_authority", "principal_id", "principal"),
		selector("principal_trust_checkpoint",
			"principal_id", "principal",
			"trust_store_id", "trust-store",
			"trust_store_incarnation_id", "trust-incarnation"),
		selector("producer_incarnation",
			"origin_principal_id", "principal",
			"producer_id", "producer"),
		selector("checkpoint_signing_authority",
			"principal_id", "principal",
			"checkpoint_signing_authority_id", "checkpoint-signer",
			"control_store_id", "control-store",
			"control_incarnation_id", "control-incarnation",
			"scope", map[string]any{
				"kind":                      "deployment",
				"deployment_binding_ref":    "binding",
				"deployment_binding_sha256": digest,
			}),
		selector("deployment_runtime_authority",
			"principal_id", "principal",
			"runtime_authority_id", "runtime-authority",
			"deployment_binding_ref", "binding",
			"deployment_binding_sha256", digest,
			"control_store_id", "control-store",
			"control_incarnation_id", "control-incarnation"),
		selector("control_checkpoint",
			"control_store_id", "control-store",
			"control_incarnation_id", "control-incarnation",
			"checkpoint_domain_id", "domain"),
		selector("local_route",
			"route_owner_principal_id", "principal",
			"route_id", "route",
			"workspace_id", "workspace",
			"deployment_binding_ref", "binding",
			"deployment_binding_sha256", digest),
		selector("execution_fence",
			"execution_fence_id", "fence",
			"execution_path_id", "path",
			"deployment_binding_ref", "binding",
			"deployment_binding_sha256", digest,
			"principal_scope", "principal",
			"controller_scope", "controller"),
		selector("deployment_acceptance_revocation",
			"deployment_acceptance_ref", "acceptance",
			"deployment_acceptance_sha256", digest),
		selector("capability_grant_revocation",
			"grant_ref", "grant",
			"grant_sha256", digest),
		selector("capability_revocation_checkpoint",
			"authority_principal_id", "principal"),
		selector("owner_record_signing_authority",
			"principal_id", "principal",
			"owner_record_signing_authority_id", "owner-signer",
			"owner_component_id", "component",
			"deployment_binding_ref", "binding",
			"deployment_binding_sha256", digest,
			"owner_contract_manifest_ref", "manifest",
			"owner_contract_manifest_sha256", digest),
		selector("node_delegation",
			"principal_id", "principal",
			"delegation_id", "delegation",
			"node_id", "node"),
		selector("node_revocation",
			"principal_id", "principal",
			"node_id", "node"),
		selector("federation_route",
			"route_owner_principal_id", "principal",
			"route_id", "route"),
	}
	if len(selectors) != 18 {
		t.Fatalf("selector inventory has %d variants, want 18", len(selectors))
	}
	for _, headKind := range selectors {
		headKind := headKind
		t.Run(headKind["kind"].(string), func(t *testing.T) {
			expected := map[string]any{"head_kind": headKind}
			encoded, err := json.Marshal(expected)
			if err != nil {
				t.Fatal(err)
			}
			var transition receipt.LocalAdminHeadTransitionV1
			if err := json.Unmarshal(encoded, &transition); err != nil {
				t.Fatal(err)
			}
			assertSemanticJSON(t, expected, transition)
		})
	}
}

func TestCheckpointSigningScopeRoundTripsBothVariants(t *testing.T) {
	t.Parallel()

	digest := strings.Repeat("b", 64)
	scopes := []map[string]any{
		{
			"kind":         "principal_trust_store",
			"bootstrap_id": "bootstrap",
		},
		{
			"kind":                      "deployment",
			"deployment_binding_ref":    "binding",
			"deployment_binding_sha256": digest,
		},
	}
	for _, expected := range scopes {
		expected := expected
		t.Run(expected["kind"].(string), func(t *testing.T) {
			encoded, err := json.Marshal(expected)
			if err != nil {
				t.Fatal(err)
			}
			var scope checkpoint.CheckpointSigningScopeV1
			if err := json.Unmarshal(encoded, &scope); err != nil {
				t.Fatal(err)
			}
			assertSemanticJSON(t, expected, scope)
		})
	}
}

func selector(kind string, fields ...any) map[string]any {
	value := map[string]any{"kind": kind}
	for index := 0; index < len(fields); index += 2 {
		value[fields[index].(string)] = fields[index+1]
	}
	return value
}

func assertSemanticJSON(t *testing.T, expected map[string]any, value any) {
	t.Helper()
	encoded, err := json.Marshal(value)
	if err != nil {
		t.Fatal(err)
	}
	var actual map[string]any
	if err := json.Unmarshal(encoded, &actual); err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(actual, expected) {
		t.Fatalf("round trip changed semantics:\nactual: %#v\nwant:   %#v", actual, expected)
	}
}
