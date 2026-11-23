package pale

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
)

// Private codecs validate transport shape, never admission or native state.
func decodeTransportTag[T ~string](data []byte, destination *T, allowed ...T) error {
	var text string
	if err := json.Unmarshal(data, &text); err != nil {
		return fmt.Errorf("invalid transport tag")
	}
	for _, tag := range allowed {
		if text == string(tag) {
			*destination = tag
			return nil
		}
	}
	return fmt.Errorf("unknown transport projection tag")
}

func decodeTransportJSON(data []byte, destination any) error {
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(destination); err != nil {
		return err
	}
	var extra any
	if err := decoder.Decode(&extra); err != io.EOF {
		return fmt.Errorf("trailing transport projection data")
	}
	return nil
}

func invalidTransportProjection(message string) error {
	return fmt.Errorf("invalid transport projection: %s", message)
}

// Reject duplicate members before decoding can silently choose one spelling.
// Nested records invoke their own codec; this scans only the current object.
func transportObjectFields(data []byte) (map[string]json.RawMessage, error) {
	decoder := json.NewDecoder(bytes.NewReader(data))
	opening, err := decoder.Token()
	if err != nil || opening != json.Delim('{') {
		return nil, fmt.Errorf("transport projection must be an object")
	}
	fields := make(map[string]json.RawMessage)
	for decoder.More() {
		token, err := decoder.Token()
		if err != nil {
			return nil, fmt.Errorf("invalid transport member")
		}
		name, ok := token.(string)
		if !ok {
			return nil, fmt.Errorf("invalid transport member")
		}
		if _, duplicate := fields[name]; duplicate {
			return nil, fmt.Errorf("duplicate transport member")
		}
		var raw json.RawMessage
		if err := decoder.Decode(&raw); err != nil {
			return nil, fmt.Errorf("invalid transport member value")
		}
		fields[name] = raw
	}
	if closing, err := decoder.Token(); err != nil || closing != json.Delim('}') {
		return nil, fmt.Errorf("invalid transport object")
	}
	if _, err := decoder.Token(); err != io.EOF {
		return nil, fmt.Errorf("trailing transport data")
	}
	return fields, nil
}

// Fixed records accept exactly their emitted member names, including nullable
// members. Check names before encoding/json can match a case-folded alias.
// Callers decode into a temporary value; errors never reveal rejected content.
func decodeTransportObject(data []byte, destination any, required []string, nullable ...string) error {
	fields, err := transportObjectFields(data)
	if err != nil {
		return err
	}
	if len(fields) != len(required)+len(nullable) {
		return fmt.Errorf("unexpected transport projection fields")
	}
	for _, name := range required {
		value, exists := fields[name]
		if !exists || bytes.Equal(bytes.TrimSpace(value), []byte("null")) {
			return fmt.Errorf("transport projection is missing a required field")
		}
	}
	for _, name := range nullable {
		if _, exists := fields[name]; !exists {
			return fmt.Errorf("transport projection is missing a nullable field")
		}
	}
	if err := decodeTransportJSON(data, destination); err != nil {
		return fmt.Errorf("invalid transport projection")
	}
	return nil
}

func decodeJSON(bytes []byte, value any) error {
	if err := json.Unmarshal(bytes, value); err != nil {
		return &BridgeError{Code: "invalid_response_metadata", Message: err.Error()}
	}
	return nil
}

// A union accepts exactly its selected payload fields. Nullable members still
// have to be present when the SDK serializer emits them.
func transportVariant(data []byte, tagName string, variants map[string][]string) (string, error) {
	fields, err := transportObjectFields(data)
	if err != nil {
		return "", err
	}
	var tag string
	if err := json.Unmarshal(fields[tagName], &tag); err != nil {
		return "", fmt.Errorf("transport variant is missing its tag")
	}
	names, exists := variants[tag]
	if !exists || len(fields) != len(names)+1 {
		return "", fmt.Errorf("unknown transport variant or payload")
	}
	for _, name := range names {
		if _, exists := fields[name]; !exists {
			return "", fmt.Errorf("transport variant is missing its payload")
		}
	}
	return tag, nil
}
