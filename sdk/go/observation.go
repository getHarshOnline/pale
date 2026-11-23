package pale

import (
	"encoding/json"
	"fmt"
)

// ObservationSnapshot is an opaque, source-free view of one admitted profile.
type ObservationSnapshot struct{ handle }

// ObservationPageCursor continues one exact snapshot, filter and raw-limit set.
// Its position does not prove that later facts are absent.
type ObservationPageCursor struct{ handle }

// ObservationLensCursor continues one exact snapshot, filter, causal-path
// choice and raw-limit set. Its position does not prove that later subjects are
// absent.
type ObservationLensCursor struct{ handle }

// ObservationSubject is an opaque nominal coordinate obtained from an admitted fact.
type ObservationSubject struct {
	handle
	// FactIndex identifies the corresponding fact in ObservationPageResult.Page.
	FactIndex uint32
	// Value copies the SDK subject coordinate without granting new request authority.
	Value ObservationSubjectValue
}

// ObservationCategory selects one independent observation axis. Health,
// readiness, effect certainty, native evidence, reconciliation, and process
// intent are never collapsed into one aggregate status.
type ObservationCategory string

const (
	ObservationOperationExposure                ObservationCategory = "operation-exposure"
	ObservationCapabilityRequirementDisposition ObservationCategory = "capability-requirement-disposition"
	ObservationHealth                           ObservationCategory = "health"
	ObservationReadiness                        ObservationCategory = "readiness"
	ObservationEffect                           ObservationCategory = "effect"
	ObservationNativeOwner                      ObservationCategory = "native-owner"
	ObservationReconciliation                   ObservationCategory = "reconciliation"
	ObservationProcessIntent                    ObservationCategory = "process-intent"
)

// ObservationFreshnessClass selects the temporal evidence class without
// claiming that unknown evidence is stale or unavailable.
type ObservationFreshnessClass string

const (
	ObservationFresh   ObservationFreshnessClass = "fresh"
	ObservationStale   ObservationFreshnessClass = "stale"
	ObservationUnknown ObservationFreshnessClass = "unknown"
)

// ObservationCancellation is cooperative request state observed by pale-sdk
// before bounded delta or lens construction begins.
type ObservationCancellation string

const (
	ObservationActive    ObservationCancellation = "active"
	ObservationCancelled ObservationCancellation = "cancelled"
)

// ObservationFilter is a typed builder for a bounded conjunction of optional
// category, nominal subject, and freshness predicates.
type ObservationFilter struct {
	category  *ObservationCategory
	subject   *ObservationSubject
	freshness *ObservationFreshnessClass
}

// AllObservations constructs the identity filter over the admitted snapshot.
func AllObservations() ObservationFilter { return ObservationFilter{} }

// WithCategory restricts the filter to one independent fact category.
func (filter ObservationFilter) WithCategory(category ObservationCategory) ObservationFilter {
	filter.category = &category
	return filter
}

// WithSubject restricts the filter to an opaque subject obtained from an
// admitted page. The subject is validated against the calling client.
func (filter ObservationFilter) WithSubject(subject *ObservationSubject) ObservationFilter {
	filter.subject = subject
	return filter
}

// WithFreshness restricts the filter to one temporal evidence class.
func (filter ObservationFilter) WithFreshness(freshness ObservationFreshnessClass) ObservationFilter {
	filter.freshness = &freshness
	return filter
}

type observationFilterWire struct {
	Category  *ObservationCategory       `json:"category,omitempty"`
	Subject   *uint64                    `json:"subject,omitempty"`
	Freshness *ObservationFreshnessClass `json:"freshness,omitempty"`
}

func (filter ObservationFilter) wire(client *Client) (observationFilterWire, error) {
	wire := observationFilterWire{Category: filter.category, Freshness: filter.freshness}
	if filter.subject == nil {
		return wire, nil
	}
	if filter.subject.client != client {
		return observationFilterWire{}, &BridgeError{Code: "invalid_handle", Message: "observation subject must belong to this client"}
	}
	if err := filter.subject.valid(); err != nil {
		return observationFilterWire{}, err
	}
	handle := filter.subject.id()
	wire.Subject = &handle
	return wire, nil
}

// ObservationPageRequest bounds one filtered snapshot step. After must come
// from the same snapshot, filter and raw limits. The limits do not bound the
// snapshot itself or prove that omitted facts are absent. Zero selects the
// application's default; the application caps larger values at its ceilings.
type ObservationPageRequest struct {
	Filter                   ObservationFilter
	MaximumInspected         uint16
	MaximumReturned          uint16
	MaximumCopiedStringBytes uint32
	Cancellation             ObservationCancellation
	After                    *ObservationPageCursor
}

// ObservationPageState distinguishes a valid bounded result, cancellation and
// a request that must restart from a fresh snapshot.
type ObservationPageState string

const (
	ObservationPageReady         ObservationPageState = "ready"
	ObservationPageCancelled     ObservationPageState = "cancelled"
	ObservationPageResynchronize ObservationPageState = "resynchronize"
)

// ObservationPageResult retains the exact API projection and any opaque
// continuations or subjects needed for subsequent typed requests. Ready does
// not mean the selected input was exhausted; inspect Page.Completion.
type ObservationPageResult struct {
	State             ObservationPageState
	Page              *ObservationPage
	Subjects          []ObservationSubject
	Continuation      *ObservationPageCursor
	Resynchronization *ObservationResynchronization
	// Preserve issued handles even if callers replace the public projections.
	ownedSubjects     []handle
	ownedContinuation handle
}

// Close releases the snapshot and invalidates all its copied handles. It does
// not close independently retained subjects or page cursors, cancel work, or
// change the copied summary. Repeated calls and a nil receiver are harmless.
func (snapshot *ObservationSnapshot) Close() {
	if snapshot != nil {
		closeNativeHandle(&snapshot.handle, dropObservationSnapshot)
	}
}

// Close releases this continuation and invalidates its copied handles.
// Repeated calls and a nil receiver are harmless.
func (cursor *ObservationPageCursor) Close() {
	if cursor != nil {
		closeNativeHandle(&cursor.handle, dropObservationPageCursor)
	}
}

// Close releases this lens continuation. Repeated calls and a nil receiver are harmless.
func (cursor *ObservationLensCursor) Close() {
	if cursor != nil {
		closeNativeHandle(&cursor.handle, dropObservationLensCursor)
	}
}

// Close releases this subject's request handle and invalidates its copies.
// The copied Value remains readable. Repeated calls and a nil receiver are harmless.
func (subject *ObservationSubject) Close() {
	if subject != nil {
		closeNativeHandle(&subject.handle, dropObservationSubject)
	}
}

// Close releases all subjects and the continuation originally issued in this
// page, including copied aliases and handles removed from its public fields.
// The snapshot and copied Page data remain usable. Keep this page open while a
// selector is needed, or close unwanted children individually instead. To page
// with bounded retention, obtain the next page before closing the previous one;
// this keeps at most two page batches live. Repeated and nil calls are harmless.
func (page *ObservationPageResult) Close() {
	if page == nil {
		return
	}
	for index := range page.ownedSubjects {
		closeNativeHandle(&page.ownedSubjects[index], dropObservationSubject)
	}
	closeNativeHandle(&page.ownedContinuation, dropObservationPageCursor)
}

// Observe projects one admitted effective profile into an opaque, source-free
// snapshot. The summary reports only API-owned coordinates and cardinality.
func (profile *EffectiveProfile) Observe() (*ObservationSnapshot, ObservationSnapshotSummary, error) {
	if profile == nil {
		return nil, ObservationSnapshotSummary{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := profile.valid(); err != nil {
		return nil, ObservationSnapshotSummary{}, err
	}
	var summary ObservationSnapshotSummary
	result, err := profile.client.callDecoded(struct {
		Profile uint64 `json:"profile"`
	}{profile.id()}, opObserveEffectiveProfile, func(result response) error {
		if result.object == 0 {
			return fmt.Errorf("observation snapshot handle is absent")
		}
		return json.Unmarshal(result.metadata, &summary)
	})
	if err != nil {
		return nil, ObservationSnapshotSummary{}, err
	}
	return &ObservationSnapshot{newHandle(profile.client, result.object)}, summary, nil
}

// Page queries a bounded snapshot without reconstructing subjects or cursor
// identities in Go. Resynchronization is a successful typed outcome, not a
// transport error.
func (snapshot *ObservationSnapshot) Page(request ObservationPageRequest) (ObservationPageResult, error) {
	if snapshot == nil {
		return ObservationPageResult{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := snapshot.valid(); err != nil {
		return ObservationPageResult{}, err
	}
	filter, err := request.Filter.wire(snapshot.client)
	if err != nil {
		return ObservationPageResult{}, err
	}
	var after *uint64
	if request.After != nil {
		if request.After.client != snapshot.client {
			return ObservationPageResult{}, &BridgeError{Code: "invalid_handle", Message: "observation page cursor must belong to this client"}
		}
		if err := request.After.valid(); err != nil {
			return ObservationPageResult{}, err
		}
		handle := request.After.id()
		after = &handle
	}
	var envelope observationPageEnvelope
	cancellation := request.Cancellation
	if cancellation == "" {
		cancellation = ObservationActive
	}
	_, err = snapshot.client.callDecoded(struct {
		Snapshot                 uint64                  `json:"snapshot"`
		Filter                   observationFilterWire   `json:"filter"`
		MaximumInspected         uint16                  `json:"maximumInspected"`
		MaximumReturned          uint16                  `json:"maximumReturned"`
		MaximumCopiedStringBytes uint32                  `json:"maximumCopiedStringBytes"`
		Cancellation             ObservationCancellation `json:"cancellation"`
		After                    *uint64                 `json:"after,omitempty"`
	}{snapshot.id(), filter, request.MaximumInspected, request.MaximumReturned, request.MaximumCopiedStringBytes, cancellation, after}, opObservationPage, func(result response) error {
		return json.Unmarshal(result.metadata, &envelope)
	})
	if err != nil {
		return ObservationPageResult{}, err
	}
	page := ObservationPageResult{
		State:             envelope.State,
		Page:              envelope.Page,
		Resynchronization: envelope.Resynchronization,
		Subjects:          make([]ObservationSubject, len(envelope.SubjectHandles)),
		ownedSubjects:     make([]handle, len(envelope.SubjectHandles)),
	}
	for index, subject := range envelope.SubjectHandles {
		page.Subjects[index] = ObservationSubject{
			handle:    newHandle(snapshot.client, subject.Handle),
			FactIndex: subject.FactIndex,
			Value:     subject.Subject,
		}
		page.ownedSubjects[index] = page.Subjects[index].handle
	}
	if envelope.ContinuationHandle != nil {
		page.Continuation = &ObservationPageCursor{newHandle(snapshot.client, *envelope.ContinuationHandle)}
		page.ownedContinuation = page.Continuation.handle
	}
	return page, nil
}

// ObservationLensRequest bounds one snapshot-bound lens step. After must come
// from the same snapshot, filter, causal-path choice and raw limits. A stopped
// step does not prove that omitted subjects are absent.
type ObservationLensRequest struct {
	Filter                   ObservationFilter
	MaximumInspected         uint16
	MaximumNodes             uint16
	MaximumCopiedStringBytes uint32
	IncludeCausalPaths       bool
	MaximumCausalPathLookups uint16
	MaximumCausalSteps       uint16
	Cancellation             ObservationCancellation
	After                    *ObservationLensCursor
}

// DeltaFrom derives one complete adjacent delta or an explicit caught-up,
// cancelled, backpressured, or resynchronization disposition. It never returns
// a partial delta.
func (snapshot *ObservationSnapshot) DeltaFrom(prior *ObservationSnapshot, maximumChanges uint16, cancellation ObservationCancellation) (ObservationDeltaResult, error) {
	if snapshot == nil {
		return ObservationDeltaResult{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if prior == nil || prior.client != snapshot.client {
		return ObservationDeltaResult{}, &BridgeError{Code: "invalid_handle", Message: "observation snapshots must belong to the same client"}
	}
	if err := snapshot.valid(); err != nil {
		return ObservationDeltaResult{}, err
	}
	if err := prior.valid(); err != nil {
		return ObservationDeltaResult{}, err
	}
	result, err := snapshot.client.call(struct {
		Current        uint64                  `json:"current"`
		Prior          uint64                  `json:"prior"`
		MaximumChanges uint16                  `json:"maximumChanges"`
		Cancellation   ObservationCancellation `json:"cancellation"`
	}{snapshot.id(), prior.id(), maximumChanges, cancellation}, opObservationDelta)
	if err != nil {
		return ObservationDeltaResult{}, err
	}
	return decodeMetadata[ObservationDeltaResult](result)
}

// CausalPath asks pale-sdk for one retained proof selected by an opaque
// subject. Ready, absent, and resynchronization remain explicit result states.
func (snapshot *ObservationSnapshot) CausalPath(subject *ObservationSubject) (ObservationCausalPathResult, error) {
	if snapshot == nil {
		return ObservationCausalPathResult{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if subject == nil || subject.client != snapshot.client {
		return ObservationCausalPathResult{}, &BridgeError{Code: "invalid_handle", Message: "observation subject must belong to this client"}
	}
	if err := snapshot.valid(); err != nil {
		return ObservationCausalPathResult{}, err
	}
	if err := subject.valid(); err != nil {
		return ObservationCausalPathResult{}, err
	}
	result, err := snapshot.client.call(struct {
		Snapshot uint64 `json:"snapshot"`
		Subject  uint64 `json:"subject"`
	}{snapshot.id(), subject.id()}, opObservationCausalPath)
	if err != nil {
		return ObservationCausalPathResult{}, err
	}
	return decodeMetadata[ObservationCausalPathResult](result)
}

// VisualLens builds one bounded, disposable observation lens step. The result
// carries snapshot-bound facts and optional retained causal paths, not layout
// or proof that omitted subjects are absent. Cancellation is an explicit
// outcome.
func (snapshot *ObservationSnapshot) VisualLens(request ObservationLensRequest) (ObservationLensResult, error) {
	if snapshot == nil {
		return ObservationLensResult{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := snapshot.valid(); err != nil {
		return ObservationLensResult{}, err
	}
	wire, err := request.Filter.wire(snapshot.client)
	if err != nil {
		return ObservationLensResult{}, err
	}
	var after *uint64
	if request.After != nil {
		if request.After.client != snapshot.client {
			return ObservationLensResult{}, &BridgeError{Code: "invalid_handle", Message: "observation lens cursor must belong to this client"}
		}
		if err := request.After.valid(); err != nil {
			return ObservationLensResult{}, err
		}
		handle := request.After.id()
		after = &handle
	}
	cancellation := request.Cancellation
	if cancellation == "" {
		cancellation = ObservationActive
	}
	var envelope observationLensEnvelope
	_, err = snapshot.client.callDecoded(struct {
		Snapshot                 uint64                  `json:"snapshot"`
		Filter                   observationFilterWire   `json:"filter"`
		MaximumInspected         uint16                  `json:"maximumInspected"`
		MaximumNodes             uint16                  `json:"maximumNodes"`
		MaximumCopiedStringBytes uint32                  `json:"maximumCopiedStringBytes"`
		IncludeCausalPaths       bool                    `json:"includeCausalPaths"`
		MaximumCausalPathLookups uint16                  `json:"maximumCausalPathLookups"`
		MaximumCausalSteps       uint16                  `json:"maximumCausalSteps"`
		Cancellation             ObservationCancellation `json:"cancellation"`
		After                    *uint64                 `json:"after,omitempty"`
	}{snapshot.id(), wire, request.MaximumInspected, request.MaximumNodes, request.MaximumCopiedStringBytes, request.IncludeCausalPaths, request.MaximumCausalPathLookups, request.MaximumCausalSteps, cancellation, after}, opObservationVisualLens, func(result response) error {
		return json.Unmarshal(result.metadata, &envelope)
	})
	if err != nil {
		return ObservationLensResult{}, err
	}
	result := envelope.ObservationLensResult
	if envelope.ContinuationHandle != nil {
		result.Continuation = &ObservationLensCursor{newHandle(snapshot.client, *envelope.ContinuationHandle)}
		result.ownedContinuation = result.Continuation.handle
	}
	return result, nil
}

// Close releases the continuation originally issued with this lens result.
// Copied lens data remains readable. Repeated calls and nil receivers are harmless.
func (result *ObservationLensResult) Close() {
	if result != nil {
		closeNativeHandle(&result.ownedContinuation, dropObservationLensCursor)
	}
}

type observationSubjectHandle struct {
	FactIndex uint32                  `json:"factIndex"`
	Handle    uint64                  `json:"handle"`
	Subject   ObservationSubjectValue `json:"subject"`
}

func (value *observationSubjectHandle) UnmarshalJSON(data []byte) error {
	type plain observationSubjectHandle
	var next plain
	if err := decodeTransportObject(data, &next, []string{"factIndex", "handle", "subject"}); err != nil {
		return err
	}
	if next.Handle == 0 {
		return fmt.Errorf("observation subject handle is absent")
	}
	*value = observationSubjectHandle(next)
	return nil
}

type observationPageEnvelope struct {
	State              ObservationPageState          `json:"state"`
	Page               *ObservationPage              `json:"page"`
	SubjectHandles     []observationSubjectHandle    `json:"subjectHandles"`
	ContinuationHandle *uint64                       `json:"continuationHandle"`
	Resynchronization  *ObservationResynchronization `json:"resynchronization"`
}

type observationLensEnvelope struct {
	ObservationLensResult
	ContinuationHandle *uint64 `json:"continuationHandle"`
}

func (value *observationPageEnvelope) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{
		"ready":         {"page", "subjectHandles", "continuationHandle"},
		"cancelled":     {"continuationHandle"},
		"resynchronize": {"resynchronization"},
	}
	kind, err := transportVariant(data, "state", variants)
	if err != nil {
		return err
	}
	type plain observationPageEnvelope
	var next plain
	required := []string{}
	if kind == "ready" {
		required = []string{"page", "subjectHandles"}
	} else if kind == "resynchronize" {
		required = []string{"resynchronization"}
	}
	if err := decodeObservationObject(data, &next, required...); err != nil {
		return err
	}
	if kind == "ready" {
		if (next.ContinuationHandle == nil) != (next.Page.Continuation == nil) {
			return fmt.Errorf("observation continuation custody differs from its page")
		}
		seen := make(map[uint64]bool, len(next.SubjectHandles)+1)
		if next.ContinuationHandle != nil {
			if *next.ContinuationHandle == 0 {
				return fmt.Errorf("observation continuation handle is absent")
			}
			seen[*next.ContinuationHandle] = true
		}
		subjectIndex := 0
		for factIndex, fact := range next.Page.Facts {
			if fact.Subject == nil {
				continue
			}
			if subjectIndex >= len(next.SubjectHandles) {
				return fmt.Errorf("observation fact lacks subject custody")
			}
			subject := next.SubjectHandles[subjectIndex]
			if subject.FactIndex != uint32(factIndex) || subject.Subject != *fact.Subject || seen[subject.Handle] {
				return fmt.Errorf("observation subject custody differs from its fact")
			}
			seen[subject.Handle] = true
			subjectIndex++
		}
		if subjectIndex != len(next.SubjectHandles) {
			return fmt.Errorf("observation page has extra subject custody")
		}
	}
	if kind == "cancelled" && next.ContinuationHandle != nil && *next.ContinuationHandle == 0 {
		return fmt.Errorf("observation continuation handle is absent")
	}
	*value = observationPageEnvelope(next)
	return nil
}

func (value *observationLensEnvelope) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{
		"ready":         {"lens", "continuationHandle"},
		"resynchronize": {"resynchronization"},
		"cancelled":     {"continuationHandle"},
	}
	kind, err := transportVariant(data, "kind", variants)
	if err != nil {
		return err
	}
	var wire struct {
		Kind               ObservationLensState          `json:"kind"`
		Lens               *ObservationLens              `json:"lens"`
		Resynchronization  *ObservationResynchronization `json:"resynchronization"`
		ContinuationHandle *uint64                       `json:"continuationHandle"`
	}
	required := []string{}
	if kind == "ready" {
		required = []string{"lens"}
	} else if kind == "resynchronize" {
		required = []string{"resynchronization"}
	}
	if err := decodeObservationObject(data, &wire, required...); err != nil {
		return err
	}
	if wire.ContinuationHandle != nil && *wire.ContinuationHandle == 0 {
		return fmt.Errorf("observation lens continuation handle is absent")
	}
	if kind == "ready" && (wire.ContinuationHandle == nil) != (wire.Lens.Continuation == nil) {
		return fmt.Errorf("observation lens continuation custody differs from its lens")
	}
	value.ObservationLensResult = ObservationLensResult{
		Kind:              wire.Kind,
		Lens:              wire.Lens,
		Resynchronization: wire.Resynchronization,
	}
	value.ContinuationHandle = wire.ContinuationHandle
	return nil
}
