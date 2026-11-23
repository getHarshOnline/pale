package tests_test

import (
	"bytes"
	"errors"
	"reflect"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

// Exercise both response buffers through their public native callers.
var contractResponseCases = []struct {
	name string
	read func(*pale.Client) ([]byte, error)
}{
	{"bundle_metadata", func(client *pale.Client) ([]byte, error) {
		value, err := client.ContractBundleIdentity()
		return value.Bytes, err
	}},
	{"schema_payload", func(client *pale.Client) ([]byte, error) {
		value, err := client.ContractSchema("diagnostic.v1")
		return value.Bytes, err
	}},
	{"unavailable_schema_diagnostic", func(client *pale.Client) ([]byte, error) {
		_, err := client.ContractSchema("not-a-selected-schema")
		if err == nil {
			return nil, errors.New("unavailable schema unexpectedly succeeded")
		}
		var diagnostic *pale.DiagnosticError
		if errors.As(err, &diagnostic) && diagnostic.Code == "contract_rejected" {
			return diagnostic.Canonical, nil
		}
		return nil, err
	}},
}

func TestContractSchemaPreservesOptionalWireIdentity(t *testing.T) {
	client := newClient(t)
	diagnosticID := "pale/diagnostic.v1"
	for _, test := range []struct {
		registryID string
		wireID     *string
	}{
		{"action-input.v1", nil},
		{"diagnostic.v1", &diagnosticID},
	} {
		t.Run(test.registryID, func(t *testing.T) {
			schema, err := client.ContractSchema(test.registryID)
			if err != nil {
				t.Fatal(err)
			}
			if !reflect.DeepEqual(schema.WireSchemaID, test.wireID) {
				t.Fatalf("wire identity lost its optional value: got %#v, want %#v", schema.WireSchemaID, test.wireID)
			}
		})
	}
}

func TestContractResponsesOwnTheirBytes(t *testing.T) {
	for _, test := range contractResponseCases {
		t.Run(test.name, func(t *testing.T) {
			client := newClient(t)
			first, err := test.read(client)
			if err != nil || len(first) == 0 {
				t.Fatalf("initial response: %v", err)
			}
			expected := bytes.Clone(first)
			second, err := test.read(client)
			if err != nil || !bytes.Equal(second, expected) {
				t.Fatalf("repeated response differs: %v", err)
			}
			first[0] ^= 0xff
			if !bytes.Equal(second, expected) {
				t.Fatal("mutating one response changed another response")
			}
			third, err := test.read(client)
			if err != nil || !bytes.Equal(third, expected) {
				t.Fatalf("mutating a response changed native data: %v", err)
			}
			client.Close()
			if !bytes.Equal(second, expected) || !bytes.Equal(third, expected) {
				t.Fatal("closing the client changed returned bytes")
			}
			closed, err := test.read(client)
			if err == nil || closed != nil {
				t.Fatal("closed client returned a successful or populated response")
			}
		})
	}
}

// Measures Go allocations for complete native calls. It does not count Rust
// allocations, resident memory, or the cost of creating the client.
func BenchmarkContractResponse(b *testing.B) {
	for _, test := range contractResponseCases {
		b.Run(test.name, func(b *testing.B) {
			client, err := pale.NewClient()
			if err != nil {
				b.Fatal(err)
			}
			b.Cleanup(client.Close)
			expected, err := test.read(client)
			if err != nil || len(expected) == 0 {
				b.Fatalf("initial response: %v", err)
			}
			b.ReportAllocs()
			b.ResetTimer()
			for b.Loop() {
				value, err := test.read(client)
				if err != nil || !bytes.Equal(value, expected) {
					b.Fatalf("response changed during measurement: %v", err)
				}
			}
		})
	}
}
