package tests_test

import (
	"encoding/json"
	"reflect"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func observationFactProjection(t *testing.T, fact pale.ObservationFact) map[string]any {
	t.Helper()
	if fact.Category != pale.ObservationReadiness || fact.Subject == nil || fact.Readiness == nil {
		t.Fatalf("conformance vector selected an unsupported fact: %#v", fact)
	}
	return map[string]any{
		"category":  "readiness",
		"subject":   fact.Subject,
		"state":     fact.Readiness,
		"freshness": fact.Freshness,
	}
}

func observationFactsProjection(t *testing.T, facts []pale.ObservationFact) []map[string]any {
	t.Helper()
	projected := make([]map[string]any, len(facts))
	for index, fact := range facts {
		projected[index] = observationFactProjection(t, fact)
	}
	return projected
}

func observationPageProjection(t *testing.T, result pale.ObservationPageResult) map[string]any {
	t.Helper()
	if result.State != pale.ObservationPageReady || result.Page == nil {
		t.Fatalf("expected a ready observation page, got %#v", result)
	}
	var continuation any
	if result.Page.Continuation != nil {
		continuation = map[string]any{
			"snapshot":          result.Page.Continuation.Snapshot,
			"inspectedPosition": result.Page.Continuation.InspectedPosition,
		}
	}
	return map[string]any{
		"kind": "ready",
		"page": map[string]any{
			"cursor":               result.Page.Cursor,
			"facts":                observationFactsProjection(t, result.Page.Facts),
			"continuation":         continuation,
			"completion":           result.Page.Completion,
			"inspected":            result.Page.Inspected,
			"copiedStringBytes":    result.Page.CopiedStringBytes,
			"snapshotOmittedCount": result.Page.SnapshotOmittedCount,
		},
	}
}

func observationLensProjection(t *testing.T, result pale.ObservationLensResult) map[string]any {
	t.Helper()
	if result.Kind != pale.ObservationLensReady || result.Lens == nil {
		t.Fatalf("expected a ready observation lens, got %#v", result)
	}
	var continuation any
	if result.Lens.Continuation != nil {
		continuation = map[string]any{
			"snapshot":          result.Lens.Continuation.Snapshot,
			"inspectedPosition": result.Lens.Continuation.InspectedPosition,
		}
	}
	nodes := make([]map[string]any, len(result.Lens.Nodes))
	for index, node := range result.Lens.Nodes {
		nodes[index] = map[string]any{
			"subject": node.Subject,
			"facts":   observationFactsProjection(t, node.Facts),
		}
	}
	return map[string]any{
		"kind": "ready",
		"lens": map[string]any{
			"cursor":               result.Lens.Cursor,
			"nodes":                nodes,
			"causalPaths":          result.Lens.CausalPaths,
			"continuation":         continuation,
			"completion":           result.Lens.Completion,
			"inspectedSubjects":    result.Lens.InspectedSubjects,
			"copiedStringBytes":    result.Lens.CopiedStringBytes,
			"causalPathLookups":    result.Lens.CausalPathLookups,
			"causalSteps":          result.Lens.CausalSteps,
			"snapshotOmittedCount": result.Lens.SnapshotOmittedCount,
		},
	}
}

func canonicalJSONValue(t *testing.T, value any) any {
	t.Helper()
	bytes, err := json.Marshal(value)
	if err != nil {
		t.Fatal(err)
	}
	var canonical any
	if err := json.Unmarshal(bytes, &canonical); err != nil {
		t.Fatal(err)
	}
	return canonical
}

func TestObservationJourneyMatchesCrossLeafVector(t *testing.T) {
	client := newClient(t)
	snapshot, summary := observationFixture(t, client, false)
	defer snapshot.Close()

	pageFilter := pale.AllObservations().WithCategory(pale.ObservationReadiness)
	pageRequest := pale.ObservationPageRequest{
		Filter: pageFilter, MaximumInspected: 16, MaximumReturned: 1,
		MaximumCopiedStringBytes: 4096,
	}
	page, err := snapshot.Page(pageRequest)
	if err != nil {
		t.Fatal(err)
	}
	defer page.Close()
	if page.Continuation == nil {
		t.Fatal("bounded fixture page needs a continuation")
	}
	mismatchRequest := pageRequest
	mismatchRequest.MaximumReturned = 2
	mismatchRequest.After = page.Continuation
	mismatch, err := snapshot.Page(mismatchRequest)
	if err != nil {
		t.Fatal(err)
	}
	defer mismatch.Close()
	if mismatch.State != pale.ObservationPageResynchronize || mismatch.Resynchronization == nil ||
		mismatch.Resynchronization.Reason.Kind != pale.ObservationContinuationMismatch {
		t.Fatalf("continuation mismatch was not preserved: %#v", mismatch)
	}

	discovery, err := snapshot.Page(pale.ObservationPageRequest{
		Filter: pale.AllObservations(), MaximumInspected: 256, MaximumReturned: 256,
		MaximumCopiedStringBytes: ^uint32(0),
	})
	if err != nil {
		t.Fatal(err)
	}
	defer discovery.Close()
	var executionPath *pale.ObservationSubject
	for index := range discovery.Subjects {
		if discovery.Subjects[index].Value.Kind == pale.ObservationExecutionPathSubject {
			executionPath = &discovery.Subjects[index]
			break
		}
	}
	if executionPath == nil {
		t.Fatal("fixture needs one execution-path subject")
	}
	causal, err := snapshot.CausalPath(executionPath)
	if err != nil || causal.State != pale.ObservationCausalPathReady || causal.Path == nil {
		t.Fatalf("causal path was not retained: %#v %v", causal, err)
	}

	lensRequest := pale.ObservationLensRequest{
		Filter:           pale.AllObservations().WithSubject(executionPath),
		MaximumInspected: 256, MaximumNodes: 1, MaximumCopiedStringBytes: 4096,
		IncludeCausalPaths: true, MaximumCausalPathLookups: 1, MaximumCausalSteps: 256,
	}
	lens, err := snapshot.VisualLens(lensRequest)
	if err != nil {
		t.Fatal(err)
	}
	defer lens.Close()

	actual := map[string]any{
		"snapshot": summary,
		"pageLimits": map[string]any{
			"maximumInspected": 16, "maximumReturned": 1,
			"maximumCopiedStringBytes": 4096,
		},
		"page": observationPageProjection(t, page),
		"continuationMismatch": map[string]any{
			"kind": "resynchronize",
			"resynchronization": map[string]any{
				"reason": map[string]any{
					"kind": mismatch.Resynchronization.Reason.Kind,
				},
				"current": mismatch.Resynchronization.Current,
			},
		},
		"causalPath": map[string]any{"kind": "ready", "path": causal.Path},
		"lensLimits": map[string]any{
			"maximumInspected": 256, "maximumNodes": 1,
			"maximumCopiedStringBytes": 4096, "maximumCausalPathLookups": 1,
			"maximumCausalSteps": 256,
		},
		"lens": observationLensProjection(t, lens),
	}
	var expected any
	readFixture(t, "observation-journey.json", &expected)
	if normalized := canonicalJSONValue(t, actual); !reflect.DeepEqual(normalized, expected) {
		actualBytes, _ := json.MarshalIndent(normalized, "", "  ")
		expectedBytes, _ := json.MarshalIndent(expected, "", "  ")
		t.Fatalf("observation journey drifted:\nwant %s\n got %s", expectedBytes, actualBytes)
	}
}
