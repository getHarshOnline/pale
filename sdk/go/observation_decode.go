package pale

import (
	"bytes"
	"fmt"
)

// Observation codecs check transport shape, never admission or state semantics.
// Callers decode into a temporary value and commit only after all fields pass.
func decodeObservationObject(data []byte, destination any, required ...string) error {
	if err := decodeTransportJSON(data, destination); err != nil {
		return fmt.Errorf("invalid observation projection")
	}
	fields, err := transportObjectFields(data)
	if err != nil {
		return err
	}
	for _, name := range required {
		value, exists := fields[name]
		if !exists || bytes.Equal(bytes.TrimSpace(value), []byte("null")) {
			return fmt.Errorf("observation projection is missing a required field")
		}
	}
	return nil
}

func decodeObservationTag[T ~string](data []byte, destination *T, allowed ...T) error {
	if err := decodeTransportTag(data, destination, allowed...); err != nil {
		return fmt.Errorf("invalid observation tag")
	}
	return nil
}
