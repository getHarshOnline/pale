package tests_test

import (
	"encoding/json"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func TestOneShotPrivateResponseDecoding(t *testing.T) {
	fixture := readOneShotFixture(t)
	encoded, err := json.Marshal(struct {
		Request      pale.OneShotRequest `json:"request"`
		RepositoryID string              `json:"repositoryId"`
	}{fixture.Request(), fixture.RepositoryID})
	if err != nil {
		t.Fatal(err)
	}
	runPackageOverlayTest(t, "one_shot_response_decode_test.go", "TestOneShotResponseDecoder", encoded)
}
