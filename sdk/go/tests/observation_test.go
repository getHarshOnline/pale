package tests_test

import (
	"bytes"
	"errors"
	"reflect"
	"sync"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func requireClosedObservation(t *testing.T, err error) {
	t.Helper()
	var bridge *pale.BridgeError
	if !errors.As(err, &bridge) || bridge.Code != "invalid_handle" {
		t.Fatalf("closed observation alias remained available: %v", err)
	}
}

func TestObservationCloseInvalidatesAliasesWithoutCascading(t *testing.T) {
	client := newClient(t)
	snapshot, _ := observationFixture(t, client, false)
	replacement, _ := observationFixture(t, client, false)
	defer replacement.Close()
	page, err := snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 256})
	if err != nil {
		t.Fatal(err)
	}
	var retained *pale.ObservationSubject
	for index := range page.Subjects {
		if page.Subjects[index].Value.Kind == pale.ObservationExecutionPathSubject {
			copy := page.Subjects[index]
			retained = &copy
			break
		}
	}
	if retained == nil {
		t.Fatal("fixture needs a retained causal selector")
	}
	snapshotCopy := *snapshot
	snapshot.Close()
	snapshotCopy.Close()
	_, err = snapshotCopy.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1})
	requireClosedObservation(t, err)
	path, err := replacement.CausalPath(retained)
	if err != nil || path.State != pale.ObservationCausalPathReady {
		t.Fatalf("snapshot close cascaded into a useful selector: %#v %v", path, err)
	}
	copyOfPage := page
	page.Subjects = nil
	page.Continuation = nil
	page.Close()
	copyOfPage.Close()
	_, err = replacement.CausalPath(retained)
	requireClosedObservation(t, err)
	if retained.Value.Kind != pale.ObservationExecutionPathSubject || page.Page == nil {
		t.Fatal("closing handles changed copied observation data")
	}
	fresh, err := replacement.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1})
	if err != nil {
		t.Fatal(err)
	}
	cursorCopy, subjectCopy := *fresh.Continuation, fresh.Subjects[0]
	fresh.Continuation.Close()
	cursorCopy.Close()
	_, err = replacement.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1, After: &cursorCopy})
	requireClosedObservation(t, err)
	fresh.Subjects[0].Close()
	subjectCopy.Close()
	_, err = replacement.CausalPath(&subjectCopy)
	requireClosedObservation(t, err)
	fresh.Close()
	var absentSnapshot *pale.ObservationSnapshot
	var absentCursor *pale.ObservationPageCursor
	var absentSubject *pale.ObservationSubject
	var absentPage *pale.ObservationPageResult
	absentSnapshot.Close()
	absentCursor.Close()
	absentSubject.Close()
	absentPage.Close()
}

func TestObservationPageCloseReleasesOriginalCursorDuringBoundedIteration(t *testing.T) {
	client := newClient(t)
	snapshot, summary := observationFixture(t, client, false)
	defer snapshot.Close()
	page, err := snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1})
	if err != nil {
		t.Fatal(err)
	}
	count := len(page.Page.Facts)
	for page.Continuation != nil {
		cursor := *page.Continuation
		next, err := snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1, After: &cursor})
		if err != nil {
			page.Close()
			t.Fatal(err)
		}
		page.Continuation = nil
		page.Subjects = nil
		page.Close()
		_, err = snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1, After: &cursor})
		requireClosedObservation(t, err)
		count += len(next.Page.Facts)
		page = next
	}
	page.Close()
	if uint32(count) != summary.FactCount {
		t.Fatalf("bounded iteration lost facts: %d != %d", count, summary.FactCount)
	}
}

func TestObservationHandleAndGroupCloseSerializeWithQueriesAndClientClose(t *testing.T) {
	for range 16 {
		client := newClient(t)
		snapshot, _ := observationFixture(t, client, false)
		page, err := snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1})
		if err != nil {
			t.Fatal(err)
		}
		subject, cursor := page.Subjects[0], *page.Continuation
		start := make(chan struct{})
		results := make(chan error, 2)
		var group sync.WaitGroup
		group.Add(5)
		go func() { defer group.Done(); <-start; _, err := snapshot.CausalPath(&subject); results <- err }()
		go func() {
			defer group.Done()
			<-start
			next, err := snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1, After: &cursor})
			next.Close()
			results <- err
		}()
		go func() { defer group.Done(); <-start; page.Close(); subject.Close(); cursor.Close() }()
		go func() { defer group.Done(); <-start; snapshot.Close() }()
		go func() { defer group.Done(); <-start; client.Close() }()
		close(start)
		group.Wait()
		close(results)
		for err := range results {
			if err == nil {
				continue
			}
			var bridge *pale.BridgeError
			if !errors.As(err, &bridge) || (bridge.Code != "invalid_handle" && bridge.Code != "invalid_binding_handle" && bridge.Code != "closed_client") {
				t.Fatalf("query/handle/client close returned an unrelated failure: %v", err)
			}
		}
		page.Close()
		snapshot.Close()
		subject.Close()
		cursor.Close()
	}
}

func TestObservationSemanticCategoryFilters(t *testing.T) {
	client := newClient(t)
	snapshot, _ := observationFixture(t, client, false)
	all, err := snapshot.Page(pale.ObservationPageRequest{
		Filter: pale.AllObservations(), MaximumInspected: 256, MaximumReturned: 256,
		MaximumCopiedStringBytes: ^uint32(0),
	})
	if err != nil {
		t.Fatal(err)
	}
	for _, category := range []pale.ObservationCategory{
		pale.ObservationOperationExposure, pale.ObservationCapabilityRequirementDisposition,
	} {
		t.Run(string(category), func(t *testing.T) {
			page, err := snapshot.Page(pale.ObservationPageRequest{
				Filter: pale.AllObservations().WithCategory(category), MaximumReturned: 256,
			})
			if err != nil {
				t.Fatalf("SDK-owned observation category must cross the native bridge: %v", err)
			}
			if page.State != pale.ObservationPageReady || page.Page == nil {
				t.Fatalf("current category filter requires a ready page, got %q", page.State)
			}
			expected := []pale.ObservationFact{}
			for _, fact := range all.Page.Facts {
				if fact.Category == category {
					expected = append(expected, fact)
				}
			}
			if !reflect.DeepEqual(expected, page.Page.Facts) {
				t.Fatalf("filtered typed facts differ from the full SDK projection: got %#v, want %#v", page.Page.Facts, expected)
			}
			if len(expected) == 0 {
				t.Log("canonical one-shot fixture selects only a role: this proves native filter acceptance and exact empty-subset preservation, not nonempty category behavior")
			}
		})
	}
}

func observationFixture(t *testing.T, client *pale.Client, changedInstance bool) (*pale.ObservationSnapshot, pale.ObservationSnapshotSummary) {
	t.Helper()
	fixture := readOneShotFixture(t)
	if changedInstance {
		original := fixture.PrivateInputs.MachineBinding.Bytes
		fixture.PrivateInputs.MachineBinding.Bytes = bytes.ReplaceAll(original, []byte("/captured/repository"), []byte("/captured/observation-candidate"))
		if bytes.Equal(original, fixture.PrivateInputs.MachineBinding.Bytes) {
			t.Fatal("fixture instance amendment did not change its source")
		}
	}
	workspace, _, err := client.CompilePortableWorkspace(fixture.Workspace)
	if err != nil {
		t.Fatal(err)
	}
	profile, _, err := workspace.CompileEffectiveProfile(pale.EffectiveProfileRequest{
		ProjectID: fixture.ProjectID, ExecutionWorkspaceID: fixture.ExecutionWorkspaceID,
		MachineID: fixture.MachineID, PrivateInputs: fixture.PrivateInputs,
		PackageDocuments: fixture.PackageDocuments,
	})
	if err != nil {
		t.Fatal(err)
	}
	snapshot, summary, err := profile.Observe()
	if err != nil {
		t.Fatal(err)
	}
	return snapshot, summary
}

func TestObservationTypedContinuationCausalityAndResynchronization(t *testing.T) {
	client := newClient(t)
	snapshot, summary := observationFixture(t, client, false)
	all, err := snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 256})
	if err != nil {
		t.Fatal(err)
	}
	firstRequest := pale.ObservationPageRequest{
		Filter: pale.AllObservations(), MaximumInspected: 256, MaximumReturned: 1,
		MaximumCopiedStringBytes: ^uint32(0),
	}
	first, err := snapshot.Page(firstRequest)
	if err != nil || first.Continuation == nil || first.Page.Continuation == nil {
		t.Fatalf("missing typed continuation: %#v %v", first, err)
	}
	continuedRequest := firstRequest
	continuedRequest.After = first.Continuation
	continued, err := snapshot.Page(continuedRequest)
	if err != nil {
		t.Fatal(err)
	}
	joined := append(append([]pale.ObservationFact{}, first.Page.Facts...), continued.Page.Facts...)
	if !reflect.DeepEqual(joined, all.Page.Facts[:len(joined)]) || first.Page.Continuation.InspectedPosition != 1 {
		t.Fatal("pagination changed the ordered facts")
	}
	cancelRequest := firstRequest
	cancelRequest.Cancellation = pale.ObservationCancelled
	cancelRequest.After = first.Continuation
	cancelledPage, err := snapshot.Page(cancelRequest)
	if err != nil || cancelledPage.State != pale.ObservationPageCancelled || cancelledPage.Continuation == nil {
		t.Fatalf("cancelled page lost its exact continuation: %#v %v", cancelledPage, err)
	}
	wrongBoundRequest := firstRequest
	wrongBoundRequest.MaximumInspected = 255
	wrongBoundRequest.After = first.Continuation
	wrongBound, err := snapshot.Page(wrongBoundRequest)
	if err != nil || wrongBound.State != pale.ObservationPageResynchronize || wrongBound.Resynchronization == nil || wrongBound.Resynchronization.Reason.Kind != pale.ObservationContinuationMismatch {
		t.Fatalf("foreign raw bound must return typed resynchronization: %#v %v", wrongBound, err)
	}
	wrongFilterRequest := firstRequest
	wrongFilterRequest.Filter = pale.AllObservations().WithCategory(pale.ObservationHealth)
	wrongFilterRequest.After = first.Continuation
	wrongFilter, err := snapshot.Page(wrongFilterRequest)
	if err != nil || wrongFilter.State != pale.ObservationPageResynchronize || wrongFilter.Page != nil || wrongFilter.Resynchronization.Reason.Kind != pale.ObservationContinuationMismatch {
		t.Fatalf("foreign filter must return typed resynchronization: %#v %v", wrongFilter, err)
	}
	lens, err := snapshot.VisualLens(pale.ObservationLensRequest{
		Filter:                   pale.AllObservations(),
		MaximumInspected:         256,
		MaximumNodes:             256,
		MaximumCopiedStringBytes: ^uint32(0),
		IncludeCausalPaths:       true,
		MaximumCausalPathLookups: 256,
		MaximumCausalSteps:       ^uint16(0),
		Cancellation:             pale.ObservationActive,
	})
	if err != nil || lens.Lens == nil {
		t.Fatalf("lens: %#v %v", lens, err)
	}
	paths := 0
	for index := range all.Subjects {
		subject := &all.Subjects[index]
		if subject.Value.Kind != pale.ObservationExecutionPathSubject {
			continue
		}
		path, err := snapshot.CausalPath(subject)
		if err != nil || path.State != pale.ObservationCausalPathReady || path.Path == nil || len(path.Path.Steps) == 0 {
			t.Fatalf("typed causal path: %#v %v", path, err)
		}
		matched := false
		for _, projected := range lens.Lens.CausalPaths {
			if reflect.DeepEqual(*path.Path, projected) {
				matched = true
			}
		}
		if !matched {
			t.Fatal("lens did not retain the same ordered causal proof")
		}
		paths++
	}
	if paths == 0 {
		t.Fatal("fixture did not exercise a retained causal proof")
	}
	candidate, candidateSummary := observationFixture(t, client, true)
	if summary.Cursor.Generation.ProfileDigest != candidateSummary.Cursor.Generation.ProfileDigest || summary.Cursor.Generation.InstanceDigest == candidateSummary.Cursor.Generation.InstanceDigest {
		t.Fatal("test must change only the private instance coordinate")
	}
	delta, err := candidate.DeltaFrom(snapshot, 256, pale.ObservationActive)
	if err != nil || delta.Kind != pale.ObservationDeltaResynchronize || delta.Resynchronization.Reason.Kind != pale.ObservationGenerationMismatch || !reflect.DeepEqual(*delta.Resynchronization.Reason.RequestedGeneration, summary.Cursor.Generation) || !reflect.DeepEqual(*delta.Resynchronization.Reason.CurrentGeneration, candidateSummary.Cursor.Generation) {
		t.Fatalf("cross-instance delta must retain both exact generations: %#v %v", delta, err)
	}
	staleRequest := firstRequest
	staleRequest.After = first.Continuation
	stale, err := candidate.Page(staleRequest)
	if err != nil || stale.State != pale.ObservationPageResynchronize || stale.Resynchronization.Reason.Kind != pale.ObservationContinuationMismatch {
		t.Fatalf("stale continuation: %#v %v", stale, err)
	}
	cancelled, err := candidate.DeltaFrom(snapshot, 1, pale.ObservationCancelled)
	if err != nil || cancelled.Kind != pale.ObservationDeltaCancelled || !reflect.DeepEqual(cancelled.Resume.Cursor, summary.Cursor) {
		t.Fatalf("cancelled delta lost caller position: %#v %v", cancelled, err)
	}
	// Go copies are inspectable data, not aliases that can change SDK custody.
	all.Subjects[0].Value.Kind = pale.ObservationResidentProcessSubject
	fresh, err := snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1})
	if err != nil || fresh.Subjects[0].Value.Kind == pale.ObservationResidentProcessSubject {
		t.Fatal("mutated Go metadata reached the retained SDK subject")
	}
}

func TestObservationHandlesRemainClientScoped(t *testing.T) {
	client, foreignClient := newClient(t), newClient(t)
	snapshot, _ := observationFixture(t, client, false)
	foreign, _ := observationFixture(t, foreignClient, false)
	page, err := snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1})
	if err != nil {
		t.Fatal(err)
	}
	_, deltaErr := foreign.DeltaFrom(snapshot, 1, pale.ObservationActive)
	_, causalErr := foreign.CausalPath(&page.Subjects[0])
	_, filterErr := foreign.Page(pale.ObservationPageRequest{Filter: pale.AllObservations().WithSubject(&page.Subjects[0]), MaximumReturned: 1})
	_, cursorErr := foreign.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1, After: page.Continuation})
	for _, err := range []error{deltaErr, causalErr, filterErr, cursorErr} {
		var bridge *pale.BridgeError
		if !errors.As(err, &bridge) || bridge.Code != "invalid_handle" {
			t.Fatalf("cross-client observation escaped: %v", err)
		}
	}
	client.Close()
	_, err = snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1})
	var bridge *pale.BridgeError
	if !errors.As(err, &bridge) || bridge.Code != "closed_client" {
		t.Fatalf("closed observation client escaped: %v", err)
	}
}

func TestObservationNilReceiversReturnHandleErrors(t *testing.T) {
	var profile *pale.EffectiveProfile
	var snapshot *pale.ObservationSnapshot
	_, _, observeErr := profile.Observe()
	_, pageErr := snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1})
	_, deltaErr := snapshot.DeltaFrom(nil, 1, pale.ObservationActive)
	_, pathErr := snapshot.CausalPath(nil)
	_, lensErr := snapshot.VisualLens(pale.ObservationLensRequest{
		Filter:                   pale.AllObservations(),
		MaximumInspected:         1,
		MaximumNodes:             1,
		MaximumCopiedStringBytes: 1,
		MaximumCausalPathLookups: 1,
		MaximumCausalSteps:       1,
		Cancellation:             pale.ObservationActive,
	})
	for _, err := range []error{observeErr, pageErr, deltaErr, pathErr, lensErr} {
		var bridge *pale.BridgeError
		if !errors.As(err, &bridge) || bridge.Code != "invalid_handle" {
			t.Fatalf("nil observation receiver must return the existing handle error: %v", err)
		}
	}
}

func TestObservationPageAndClientCloseSerializeCustody(t *testing.T) {
	for range 16 {
		client := newClient(t)
		snapshot, _ := observationFixture(t, client, false)
		start := make(chan struct{})
		pageDone := make(chan error, 1)
		closeDone := make(chan struct{})
		go func() {
			<-start
			page, err := snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1})
			if err == nil && (page.State != pale.ObservationPageReady || page.Page == nil || len(page.Subjects) != 1 || page.Continuation == nil) {
				err = errors.New("successful concurrent page lost its complete claimed values")
			}
			pageDone <- err
		}()
		go func() { <-start; client.Close(); close(closeDone) }()
		close(start)
		err := <-pageDone
		<-closeDone
		if err != nil {
			var bridge *pale.BridgeError
			if !errors.As(err, &bridge) || bridge.Code != "closed_client" {
				t.Fatalf("page/close race split custody: %v", err)
			}
		}
		if _, err := snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1}); err == nil {
			t.Fatal("closed client retained a callable snapshot")
		}
	}
}
