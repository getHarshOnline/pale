package pale

import (
	"reflect"
	"strings"
	"testing"
)

// Synthetic transport envelopes test shape and correlation. They are not
// malformed bytes emitted by the native compiler.
func TestEffectiveProfileExplanationResponse(t *testing.T) {
	valid := response{metadata: []byte(`{"handle":"requested","evidence":{"kind":"configuration"}}`)}
	actual, err := decodeEffectiveProfileExplanation(valid, "requested")
	if err != nil || actual == nil || actual.Handle != "requested" || actual.Evidence.Kind != EffectiveProfileEvidenceConfiguration {
		t.Fatalf("complete explanation rejected: %v", err)
	}
	for _, metadata := range []string{"null", " \n null \t"} {
		if value, err := decodeEffectiveProfileExplanation(response{metadata: []byte(metadata)}, "requested"); err != nil || value != nil {
			t.Fatal("whole null was not an ordinary miss")
		}
	}
	cases := []response{
		{}, {metadata: []byte(" ")}, {metadata: []byte("null {}")},
		{metadata: []byte(`{"handle":"other","evidence":{"kind":"configuration"}}`)},
		{object: 1, metadata: valid.metadata},
		{payload: []byte("private-marker"), metadata: valid.metadata},
		{object: 1, metadata: []byte("null")},
		{payload: []byte("private-marker"), metadata: []byte("null")},
		{metadata: []byte(`{"handle":"private-marker","evidence":null}`)},
		{metadata: []byte(`{"handle":`)},
	}
	for index, malformed := range cases {
		value, err := decodeEffectiveProfileExplanation(malformed, "requested")
		if err == nil || value != nil || strings.Contains(err.Error(), "private-marker") {
			t.Fatalf("malformed response %d accepted or leaked: %v", index, err)
		}
	}
	// Results own their copied values; no response buffer is retained.
	expected := *actual
	for index := range valid.metadata {
		valid.metadata[index] = 'x'
	}
	if !reflect.DeepEqual(*actual, expected) {
		t.Fatal("response bytes alias copied evidence")
	}
}
