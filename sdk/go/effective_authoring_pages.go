package pale

import (
	"encoding/json"
	"fmt"
)

// EffectiveAuthoringContinuation retains the exact SDK profile projection,
// authoring identity and raw page limit. Displayed coordinates cannot reconstruct it.
type EffectiveAuthoringContinuation struct{ handle }

// Close releases this continuation and invalidates copied aliases without
// closing the profile. Nil and repeated calls are harmless.
func (cursor *EffectiveAuthoringContinuation) Close() {
	if cursor != nil {
		closeNativeHandle(&cursor.handle, dropEffectiveAuthoringCursor)
	}
}

// EffectiveAuthoringPage copies ordered profile associations and their complete
// profile identity. A nil Continuation means the association is complete.
// Entries use existing explanation handles; explain them while the profile is open.
type EffectiveAuthoringPage struct {
	Coordinates       EffectiveProfileCoordinates
	Entries           []EffectiveProfileEntry
	Continuation      *EffectiveAuthoringContinuation
	ownedContinuation handle
}

// Close releases the original continuation even if its exported field changed.
// Copied rows remain readable. Next-before-close iteration retains at most two
// cursor batches, not a universal memory quota; retained pages remain the caller's
// responsibility. Nil and repeated calls are harmless.
func (page *EffectiveAuthoringPage) Close() {
	if page != nil {
		closeNativeHandle(&page.ownedContinuation, dropEffectiveAuthoringCursor)
	}
}

type effectiveAuthoringPageEnvelope struct {
	Coordinates        EffectiveProfileCoordinates `json:"coordinates"`
	Entries            []EffectiveProfileEntry     `json:"entries"`
	ContinuationHandle *uint64                     `json:"continuationHandle"`
}

func (value *effectiveAuthoringPageEnvelope) UnmarshalJSON(data []byte) error {
	type plain effectiveAuthoringPageEnvelope
	var next plain
	if err := decodeTransportObject(data, &next, []string{"coordinates", "entries"}, "continuationHandle"); err != nil {
		return err
	}
	if next.ContinuationHandle != nil && *next.ContinuationHandle == 0 {
		return fmt.Errorf("effective authoring continuation handle is absent")
	}
	*value = effectiveAuthoringPageEnvelope(next)
	return nil
}

// InspectEffectiveAuthoringPage reads retained associations without recompiling.
// Zero selects 64 rows; the SDK caps requests at 256 rows. Rows may contain many
// members, so the limit does not bound bytes. Resume with the same full identity,
// raw limit and retained profile; equal digests do not make another profile the
// same cursor owner. SDK rejection is a DiagnosticError; malformed or closed
// handles are BridgeErrors. This read grants no source-disclosure permission.
// The request identity must remain open; a cursor does not restore identities
// released by AuthoringNavigation.Close.
func (profile *EffectiveProfile) InspectEffectiveAuthoringPage(identity *AuthoringIdentity,
	limit uint16, after *EffectiveAuthoringContinuation) (EffectiveAuthoringPage, error) {
	if profile == nil {
		return EffectiveAuthoringPage{}, &BridgeError{Code: "invalid_handle", Message: "effective profile is unavailable"}
	}
	if err := profile.valid(); err != nil {
		return EffectiveAuthoringPage{}, err
	}
	profile.client.mu.Lock()
	defer profile.client.mu.Unlock()
	var cursor *handle
	if after != nil {
		cursor = &after.handle
	}
	afterID, err := authoringPageHandles(&profile.handle, identity, cursor)
	if err != nil {
		return EffectiveAuthoringPage{}, err
	}
	var envelope effectiveAuthoringPageEnvelope
	_, err = profile.client.callDecodedLocked(struct {
		Handle   uint64  `json:"handle"`
		Identity uint64  `json:"identity"`
		Limit    uint16  `json:"limit"`
		After    *uint64 `json:"after"`
	}{profile.id(), identity.id(), limit, afterID}, opEffectiveAuthoringPage, func(result response) error {
		if result.object != 0 || len(result.payload) != 0 {
			return fmt.Errorf("unexpected effective authoring page response payload")
		}
		return json.Unmarshal(result.metadata, &envelope)
	})
	if err != nil {
		return EffectiveAuthoringPage{}, err
	}
	page := EffectiveAuthoringPage{Coordinates: envelope.Coordinates, Entries: envelope.Entries}
	if envelope.ContinuationHandle != nil {
		page.Continuation = &EffectiveAuthoringContinuation{newHandle(profile.client, *envelope.ContinuationHandle)}
		page.ownedContinuation = page.Continuation.handle
	}
	return page, nil
}
