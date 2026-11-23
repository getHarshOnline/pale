package pale

import (
	"encoding/json"
	"errors"
	"os"
	"reflect"
	"testing"
)

// Truncation is injected at the host decoder after a real native response.
// This proves failed delivery publishes no partial result and leaves the
// retained comparison usable; no new result handle exists to strand.
func TestSubjectComparisonResponseFailure(t *testing.T) {
	encoded, err := os.ReadFile(os.Getenv("PALE_GO_TEST_FIXTURE"))
	if err != nil {
		t.Fatal(err)
	}
	var request PortableWorkspaceRequest
	if err := json.Unmarshal(encoded, &request); err != nil {
		t.Fatal(err)
	}
	client, err := NewClient()
	if err != nil {
		t.Fatal(err)
	}
	defer client.Close()
	workspace, _, err := client.CompilePortableWorkspace(request)
	if err != nil {
		t.Fatal(err)
	}
	comparison, _, err := client.ComparePortableWorkspaces(workspace, workspace)
	if err != nil {
		t.Fatal(err)
	}
	defer comparison.Close()
	requestValue := struct {
		Handle  uint64           `json:"handle"`
		Subject WorkspaceSubject `json:"subject"`
	}{comparison.id(), WorkspaceIdentitySubject("empty-adopter")}
	result, err := client.callDecoded(requestValue, opComparePortableWorkspaceSubject, func(result response) error {
		if result.object != 0 || len(result.metadata) != 0 {
			t.Fatal("subject result unexpectedly retained a handle")
		}
		var value WorkspaceSubjectComparison
		if err := json.Unmarshal(result.payload, &value); err != nil {
			t.Fatal(err)
		}
		before := value
		err := json.Unmarshal(result.payload[:len(result.payload)-1], &value)
		if err == nil || !reflect.DeepEqual(before, value) {
			t.Fatal("partial result was published")
		}
		return err
	})
	var failure *BridgeError
	if !errors.As(err, &failure) || failure.Code != "invalid_response_metadata" || !reflect.DeepEqual(result, response{}) {
		t.Fatal("failed delivery exposed a response", err)
	}
	if _, _, err := comparison.CompareSubject(WorkspaceIdentitySubject("empty-adopter")); err != nil {
		t.Fatal("decode failure consumed the comparison", err)
	}
	foreign, err := NewClient()
	if err != nil {
		t.Fatal(err)
	}
	defer foreign.Close()
	forged := WorkspaceComparison{newHandle(foreign, comparison.id())}
	if value, canonical, err := forged.CompareSubject(WorkspaceIdentitySubject("empty-adopter")); err == nil || value.Values.Value != nil || len(canonical.Bytes) != 0 {
		t.Fatal("foreign-client handle obtained a result")
	}
}
