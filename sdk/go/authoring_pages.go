package pale

import (
	"encoding/json"
	"fmt"
)

// AuthoringOccurrence is a copied source location and its compiler-resolved role.
// It contains no source text or renderer coordinates.
type AuthoringOccurrence struct {
	Location AuthoringSourceLocation `json:"location"`
	Role     AuthoringOccurrenceRole `json:"role"`
}

// UnmarshalJSON requires the exact location and role before replacing the value.
func (value *AuthoringOccurrence) UnmarshalJSON(data []byte) error {
	type plain AuthoringOccurrence
	var next plain
	if err := decodeTransportObject(data, &next, []string{"location", "role"}); err != nil {
		return err
	}
	*value = AuthoringOccurrence(next)
	return nil
}

// AuthoringOccurrencePageCursor retains the SDK's exact compilation, identity
// and raw requested limits. It cannot be constructed from displayed coordinates.
type AuthoringOccurrencePageCursor struct{ handle }

// Close releases this cursor and invalidates copied aliases. Nil and repeated
// calls are harmless; no compilation, cancellation or configuration effect occurs.
func (cursor *AuthoringOccurrencePageCursor) Close() {
	if cursor != nil {
		closeNativeHandle(&cursor.handle, dropAuthoringOccurrenceCursor)
	}
}

// AuthoringOccurrencePage contains matching locations and charged scan progress.
// An empty page may be incomplete. IsComplete comes from the SDK, not from the
// number of occurrences. CopiedStringBytes counts returned source labels and
// field names. Copied data remains readable after Close.
type AuthoringOccurrencePage struct {
	Occurrences       []AuthoringOccurrence
	Inspected         uint16
	CopiedStringBytes uint32
	IsComplete        bool
	Continuation      *AuthoringOccurrencePageCursor
	ownedContinuation handle
}

// Close releases the originally issued continuation even if its exported field
// was replaced. Obtain the next page before closing this one to keep at most two
// cursor batches in a sequential scan. This usage pattern is not a process-wide
// handle or byte quota: callers can deliberately retain more pages. Nil and
// repeated calls are harmless. The compilation and identity remain available.
func (page *AuthoringOccurrencePage) Close() {
	if page != nil {
		closeNativeHandle(&page.ownedContinuation, dropAuthoringOccurrenceCursor)
	}
}

type occurrencePageEnvelope struct {
	Occurrences        []AuthoringOccurrence `json:"occurrences"`
	Inspected          uint16                `json:"inspected"`
	CopiedStringBytes  uint32                `json:"copiedStringBytes"`
	IsComplete         bool                  `json:"isComplete"`
	ContinuationHandle *uint64               `json:"continuationHandle"`
}

func (value *occurrencePageEnvelope) UnmarshalJSON(data []byte) error {
	type plain occurrencePageEnvelope
	var next plain
	if err := decodeTransportObject(data, &next,
		[]string{"occurrences", "inspected", "copiedStringBytes", "isComplete"}, "continuationHandle"); err != nil {
		return err
	}
	if next.IsComplete != (next.ContinuationHandle == nil) ||
		(next.ContinuationHandle != nil && *next.ContinuationHandle == 0) {
		return fmt.Errorf("authoring continuation custody differs from completion")
	}
	*value = occurrencePageEnvelope(next)
	return nil
}

// AuthoringOccurrences scans this retained workspace's source index. Zero limits
// select 64; values above 256 are capped by the SDK. Resume with the same identity
// and raw limits, even when different values normalize to the same bounds.
// The identity may come from another compilation in this client, but a cursor
// may not. Row limits combine with one fixed copied-string ceiling. The caller
// remains responsible for permission to disclose locations and scan counts.
// Keep the identity's navigation owner open while using that identity handle;
// a cursor does not restore a handle released by AuthoringNavigation.Close.
func (workspace *PortableWorkspace) AuthoringOccurrences(identity *AuthoringIdentity,
	maximumInspected, maximumReturned uint16, after *AuthoringOccurrencePageCursor) (AuthoringOccurrencePage, error) {
	if workspace == nil {
		return AuthoringOccurrencePage{}, &BridgeError{Code: "invalid_handle", Message: "portable workspace is unavailable"}
	}
	return authoringOccurrences(&workspace.handle, identity, maximumInspected, maximumReturned, after, opPortableAuthoringOccurrences)
}

// AuthoringOccurrences uses this profile's combined retained source index. It
// follows PortableWorkspace.AuthoringOccurrences limits and disclosure rules.
// A portable cursor cannot resume an effective-profile scan.
func (profile *EffectiveProfile) AuthoringOccurrences(identity *AuthoringIdentity,
	maximumInspected, maximumReturned uint16, after *AuthoringOccurrencePageCursor) (AuthoringOccurrencePage, error) {
	if profile == nil {
		return AuthoringOccurrencePage{}, &BridgeError{Code: "invalid_handle", Message: "effective profile is unavailable"}
	}
	return authoringOccurrences(&profile.handle, identity, maximumInspected, maximumReturned, after, opEffectiveAuthoringOccurrences)
}

// The caller holds the client mutex through validation, native lookup and claim.
func authoringPageHandles(owner *handle, identity *AuthoringIdentity, cursor *handle) (*uint64, error) {
	if err := owner.valid(); err != nil {
		return nil, err
	}
	if identity == nil || identity.client != owner.client {
		return nil, &BridgeError{Code: "invalid_handle", Message: "authoring identity must belong to this client"}
	}
	if err := identity.valid(); err != nil {
		return nil, err
	}
	if cursor == nil {
		return nil, nil
	}
	if cursor.client != owner.client {
		return nil, &BridgeError{Code: "invalid_handle", Message: "authoring cursor must belong to this client"}
	}
	if err := cursor.valid(); err != nil {
		return nil, err
	}
	id := cursor.id()
	return &id, nil
}

func authoringOccurrences(owner *handle, identity *AuthoringIdentity, maximumInspected, maximumReturned uint16,
	after *AuthoringOccurrencePageCursor, invoke operation) (AuthoringOccurrencePage, error) {
	if err := owner.valid(); err != nil {
		return AuthoringOccurrencePage{}, err
	}
	owner.client.mu.Lock()
	defer owner.client.mu.Unlock()
	var cursor *handle
	if after != nil {
		cursor = &after.handle
	}
	afterID, err := authoringPageHandles(owner, identity, cursor)
	if err != nil {
		return AuthoringOccurrencePage{}, err
	}
	var envelope occurrencePageEnvelope
	_, err = owner.client.callDecodedLocked(struct {
		Handle           uint64  `json:"handle"`
		Identity         uint64  `json:"identity"`
		MaximumInspected uint16  `json:"maximumInspected"`
		MaximumReturned  uint16  `json:"maximumReturned"`
		After            *uint64 `json:"after"`
	}{owner.id(), identity.id(), maximumInspected, maximumReturned, afterID}, invoke, func(result response) error {
		if result.object != 0 || len(result.payload) != 0 {
			return fmt.Errorf("unexpected authoring page response payload")
		}
		return json.Unmarshal(result.metadata, &envelope)
	})
	if err != nil {
		return AuthoringOccurrencePage{}, err
	}
	page := AuthoringOccurrencePage{Occurrences: envelope.Occurrences, Inspected: envelope.Inspected, CopiedStringBytes: envelope.CopiedStringBytes, IsComplete: envelope.IsComplete}
	if envelope.ContinuationHandle != nil {
		page.Continuation = &AuthoringOccurrencePageCursor{newHandle(owner.client, *envelope.ContinuationHandle)}
		page.ownedContinuation = page.Continuation.handle
	}
	return page, nil
}
