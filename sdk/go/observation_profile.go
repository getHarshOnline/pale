package pale

import (
	"encoding/json"
	"fmt"
)

// ObservationProfileContinuation resumes one snapshot, subject and limit pair.
// It retains no profile or workspace and cannot be reconstructed from coordinates.
type ObservationProfileContinuation struct{ handle }

// Close releases this continuation and invalidates copied aliases. Nil and
// repeated calls are harmless; source compilations and copied rows are unchanged.
func (cursor *ObservationProfileContinuation) Close() {
	if cursor != nil {
		closeNativeHandle(&cursor.handle, dropObservationProfileCursor)
	}
}

// ObservationProfileRequest limits raw rows inspected and matching rows returned.
// Both limits must be in 1..=256. Zero does not select a default.
type ObservationProfileRequest struct {
	MaximumInspected uint16
	MaximumReturned  uint16
	After            *ObservationProfileContinuation
}

// ObservationProfileMapping reports category support, not whether a subject exists.
type ObservationProfileMapping string

const (
	ObservationProfilePrimaryRows   ObservationProfileMapping = "primary-rows"
	ObservationProfileNoPrimaryRows ObservationProfileMapping = "no-primary-rows"
)

// UnmarshalJSON accepts only the Rust SDK's mapping variants.
func (value *ObservationProfileMapping) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, ObservationProfilePrimaryRows, ObservationProfileNoPrimaryRows)
}

// ObservationProfileRefusal explains why this query could not read profile rows.
// It is a query outcome, not an operation-failure diagnostic or a Go error.
type ObservationProfileRefusal string

const (
	ObservationProfileNoSource             ObservationProfileRefusal = "no-profile-source"
	ObservationProfileExpiredSource        ObservationProfileRefusal = "expired-source"
	ObservationProfileSourceMismatch       ObservationProfileRefusal = "source-mismatch"
	ObservationProfileCoordinateMismatch   ObservationProfileRefusal = "coordinate-mismatch"
	ObservationProfileContinuationMismatch ObservationProfileRefusal = "continuation-mismatch"
	ObservationProfileInvalidLimit         ObservationProfileRefusal = "invalid-limit"
)

// UnmarshalJSON rejects unknown refusals instead of substituting another reason.
func (value *ObservationProfileRefusal) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, ObservationProfileNoSource, ObservationProfileExpiredSource,
		ObservationProfileSourceMismatch, ObservationProfileCoordinateMismatch,
		ObservationProfileContinuationMismatch, ObservationProfileInvalidLimit)
}

// ObservationProfileState distinguishes a completed page query from a typed refusal.
type ObservationProfileState string

const (
	ObservationProfileReady   ObservationProfileState = "ready"
	ObservationProfileRefused ObservationProfileState = "refused"
)

// ObservationProfilePage copies original profile rows and their explanation handles.
// An empty page can have a continuation. Inspected counts raw rows, not matches.
type ObservationProfilePage struct {
	Coordinates  EffectiveProfileCoordinates
	Mapping      ObservationProfileMapping
	Entries      []EffectiveProfileEntry
	Inspected    uint16
	Continuation *ObservationProfileContinuation
}

// ObservationProfileResult contains either a page or a refusal. A refusal leaves
// the snapshot's source-free facts usable. Neither outcome authorizes disclosure.
type ObservationProfileResult struct {
	State             ObservationProfileState
	Page              *ObservationProfilePage
	Refusal           *ObservationProfileRefusal
	ownedContinuation handle
}

// Close releases the original continuation even if Page or its cursor was replaced.
// Copied rows survive. Obtain the next page before closing the previous result;
// retained pages and their payload sizes remain the caller's responsibility.
func (result *ObservationProfileResult) Close() {
	if result != nil {
		closeNativeHandle(&result.ownedContinuation, dropObservationProfileCursor)
	}
}

type observationProfileEnvelope struct {
	State              ObservationProfileState      `json:"state"`
	Coordinates        *EffectiveProfileCoordinates `json:"coordinates"`
	Mapping            ObservationProfileMapping    `json:"mapping"`
	Entries            []EffectiveProfileEntry      `json:"entries"`
	Inspected          uint16                       `json:"inspected"`
	ContinuationHandle *uint64                      `json:"continuationHandle"`
	Reason             *ObservationProfileRefusal   `json:"reason"`
}

func (value *observationProfileEnvelope) UnmarshalJSON(data []byte) error {
	state, err := transportVariant(data, "state", map[string][]string{
		"ready":   {"coordinates", "mapping", "entries", "inspected", "continuationHandle"},
		"refused": {"reason"},
	})
	if err != nil {
		return err
	}
	type plain observationProfileEnvelope
	var next plain
	if state == "ready" {
		err = decodeTransportObject(data, &next, []string{"state", "coordinates", "mapping", "entries", "inspected"}, "continuationHandle")
	} else {
		err = decodeTransportObject(data, &next, []string{"state", "reason"})
	}
	if err != nil {
		return err
	}
	if next.ContinuationHandle != nil && *next.ContinuationHandle == 0 {
		return fmt.Errorf("observation profile continuation handle is absent")
	}
	*value = observationProfileEnvelope(next)
	return nil
}

// InspectProfile associates an observation subject with original profile rows.
// Supply open profile and workspace handles from the snapshot's admitted source.
// A second profile allocation from that source can qualify; a separately compiled
// equal-byte workspace cannot. The SDK checks source, coordinates and continuation.
// Both limits count rows, not bytes or all retained memory. No source is reparsed.
// Explain returned entries through profile.Explain while that profile stays open.
// SDK refusals are typed results; invalid handles and malformed transport are errors.
// This call does not apply edits, prove native state or grant disclosure permission.
func (snapshot *ObservationSnapshot) InspectProfile(profile *EffectiveProfile, workspace *PortableWorkspace,
	subject *ObservationSubject, request ObservationProfileRequest) (ObservationProfileResult, error) {
	if snapshot == nil {
		return ObservationProfileResult{}, &BridgeError{Code: "invalid_handle", Message: "observation snapshot is unavailable"}
	}
	if err := snapshot.valid(); err != nil {
		return ObservationProfileResult{}, err
	}
	client := snapshot.client
	client.mu.Lock()
	defer client.mu.Unlock()
	if profile == nil || workspace == nil || subject == nil {
		return ObservationProfileResult{}, &BridgeError{Code: "invalid_handle", Message: "profile association requires open compilation and subject handles"}
	}
	values := []*handle{&snapshot.handle, &profile.handle, &workspace.handle, &subject.handle}
	if request.After != nil {
		values = append(values, &request.After.handle)
	}
	for _, value := range values {
		if value.client != client {
			return ObservationProfileResult{}, &BridgeError{Code: "invalid_handle", Message: "profile association handles must belong to this client"}
		}
		if err := value.valid(); err != nil {
			return ObservationProfileResult{}, err
		}
	}
	var after *uint64
	if request.After != nil {
		id := request.After.id()
		after = &id
	}
	var envelope observationProfileEnvelope
	_, err := client.callDecodedLocked(struct {
		Snapshot         uint64  `json:"snapshot"`
		Profile          uint64  `json:"profile"`
		Workspace        uint64  `json:"workspace"`
		Subject          uint64  `json:"subject"`
		MaximumInspected uint16  `json:"maximumInspected"`
		MaximumReturned  uint16  `json:"maximumReturned"`
		After            *uint64 `json:"after"`
	}{snapshot.id(), profile.id(), workspace.id(), subject.id(), request.MaximumInspected,
		request.MaximumReturned, after}, opObservationInspectProfile, func(result response) error {
		if result.object != 0 || len(result.payload) != 0 {
			return fmt.Errorf("unexpected observation profile response payload")
		}
		return json.Unmarshal(result.metadata, &envelope)
	})
	if err != nil {
		return ObservationProfileResult{}, err
	}
	result := ObservationProfileResult{State: envelope.State, Refusal: envelope.Reason}
	if envelope.State == ObservationProfileReady {
		result.Page = &ObservationProfilePage{Coordinates: *envelope.Coordinates, Mapping: envelope.Mapping,
			Entries: envelope.Entries, Inspected: envelope.Inspected}
		if envelope.ContinuationHandle != nil {
			result.Page.Continuation = &ObservationProfileContinuation{newHandle(client, *envelope.ContinuationHandle)}
			result.ownedContinuation = result.Page.Continuation.handle
		}
	}
	return result, nil
}
