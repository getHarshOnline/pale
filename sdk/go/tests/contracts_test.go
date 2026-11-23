package tests_test

import (
	"encoding/json"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
	"github.com/getHarshOnline/pale/sdk/go/contracts"
	diagnostic_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/diagnostic_v1"
)

func TestGeneratedContractsMatchNativeRegistry(t *testing.T) {
	client, err := pale.NewClient()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(client.Close)

	families := contracts.SchemaFamilies()
	var bundle struct {
		MemberCount uint32 `json:"member_count"`
	}
	readFixture(t, "bundle-identity.json", &bundle)
	if len(families) != int(bundle.MemberCount) {
		t.Fatalf("generated family count = %d, want %d", len(families), bundle.MemberCount)
	}
	for _, family := range families {
		_, ok := contracts.ContractIdentity(family)
		if !ok {
			t.Fatalf("generated registry has no digest for %q", family)
		}
	}
	identity, err := client.ContractBundleIdentity()
	if err != nil {
		t.Fatal(err)
	}
	var native struct {
		MemberCount          uint32 `json:"memberCount"`
		SchemaManifestSHA256 string `json:"schemaManifestSha256"`
		SchemaSetSHA256      string `json:"schemaSetSha256"`
	}
	if err := json.Unmarshal(identity.Bytes, &native); err != nil {
		t.Fatal(err)
	}
	if native.MemberCount != bundle.MemberCount || native.SchemaManifestSHA256 != contracts.SchemaManifestSHA256 || native.SchemaSetSHA256 != contracts.SchemaSetSHA256 {
		t.Fatalf("native and generated contract coordinates disagree: %#v", native)
	}
}

func TestGeneratedDiagnosticDecodesCanonicalFixture(t *testing.T) {
	var fixture struct {
		CanonicalDiagnostic string `json:"canonical_diagnostic"`
	}
	readFixture(t, "contract-diagnostic.json", &fixture)
	var diagnostic diagnostic_v1.DiagnosticV1
	if err := json.Unmarshal([]byte(fixture.CanonicalDiagnostic), &diagnostic); err != nil {
		t.Fatal(err)
	}
	if diagnostic.Schema != "pale/diagnostic.v1" || diagnostic.Code != "cross_leaf_probe" {
		t.Fatalf("generated diagnostic decoded the wrong identity: %#v", diagnostic)
	}
}
