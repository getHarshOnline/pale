package tests_test

import (
	"bytes"
	"encoding/json"
	"errors"
	"reflect"
	"sync"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func compileObservationProfile(t *testing.T, workspace *pale.PortableWorkspace, fixture oneShotFixture) *pale.EffectiveProfile {
	t.Helper()
	profile, _, err := workspace.CompileEffectiveProfile(pale.EffectiveProfileRequest{
		ProjectID: fixture.ProjectID, ExecutionWorkspaceID: fixture.ExecutionWorkspaceID,
		MachineID: fixture.MachineID, PrivateInputs: fixture.PrivateInputs, PackageDocuments: fixture.PackageDocuments,
	})
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(profile.Close)
	return profile
}

func observationProfileFixture(t *testing.T) (*pale.Client, *pale.PortableWorkspace, *pale.EffectiveProfile, *pale.ObservationSnapshot, pale.ObservationPageResult) {
	t.Helper()
	client := newClient(t)
	fixture := readOneShotFixture(t)
	workspace, _, err := client.CompilePortableWorkspace(fixture.Workspace)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(workspace.Close)
	profile := compileObservationProfile(t, workspace, fixture)
	snapshot, _, err := profile.Observe()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(snapshot.Close)
	page, err := snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 256})
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(page.Close)
	return client, workspace, profile, snapshot, page
}

func machineObservation(t *testing.T, page *pale.ObservationPageResult) *pale.ObservationSubject {
	t.Helper()
	for index := range page.Subjects {
		if page.Subjects[index].Value.Kind == pale.ObservationMachineSubject {
			return &page.Subjects[index]
		}
	}
	t.Fatal("compiled fixture has no machine subject")
	return nil
}

func requireProfileRefusal(t *testing.T, result pale.ObservationProfileResult, err error, reason pale.ObservationProfileRefusal) {
	t.Helper()
	defer result.Close()
	if err != nil || result.State != pale.ObservationProfileRefused || result.Page != nil || result.Refusal == nil || *result.Refusal != reason {
		t.Fatalf("wrong profile refusal: %#v %v", result, err)
	}
}

func scanObservationProfile(t *testing.T, snapshot *pale.ObservationSnapshot, profile *pale.EffectiveProfile,
	workspace *pale.PortableWorkspace, subject *pale.ObservationSubject, limit uint16) []pale.EffectiveProfileEntry {
	t.Helper()
	request := pale.ObservationProfileRequest{MaximumInspected: limit, MaximumReturned: limit}
	var rows []pale.EffectiveProfileEntry
	previous := pale.ObservationProfileResult{}
	for visits := 0; visits < 4096; visits++ {
		result, err := snapshot.InspectProfile(profile, workspace, subject, request)
		previous.Close()
		if err != nil || result.State != pale.ObservationProfileReady || result.Page == nil || result.Refusal != nil {
			t.Fatalf("profile page failed: %#v %v", result, err)
		}
		page := result.Page
		if page.Inspected > limit || len(page.Entries) > int(limit) {
			t.Fatal("raw/result bound lost")
		}
		if subject.Value.Kind == pale.ObservationEffectiveProfileSubject ||
			subject.Value.Kind == pale.ObservationCompiledGenerationSubject ||
			subject.Value.Kind == pale.ObservationResidentProcessSubject {
			if page.Mapping != pale.ObservationProfileNoPrimaryRows || len(page.Entries) != 0 {
				t.Fatal("unsupported subject acquired a primary mapping")
			}
		} else if page.Mapping != pale.ObservationProfilePrimaryRows {
			t.Fatal("supported category lost its primary mapping")
		}
		rows = append(rows, page.Entries...)
		if page.Continuation == nil {
			result.Close()
			return rows
		}
		if page.Inspected == 0 {
			result.Close()
			t.Fatal("profile continuation did not inspect any raw row")
		}
		request.After = page.Continuation
		previous = result
	}
	previous.Close()
	t.Fatal("bounded fixture scan did not terminate")
	return nil
}

func TestObservationProfileOriginalRowsAndExplanationHandles(t *testing.T) {
	_, workspace, profile, snapshot, observations := observationProfileFixture(t)
	fixture := readOneShotFixture(t)
	sameSource := compileObservationProfile(t, workspace, fixture)
	var originalRows []pale.EffectiveProfileEntry
	for _, section := range []pale.EffectiveProfileSection{pale.ProfileIntendedUseCase, pale.ProfileEffectiveConfiguration,
		pale.ProfileCapabilitiesOperations, pale.ProfileMachineRealization, pale.ProfileGapsAndExplanations} {
		document, err := profile.Inspect(section, 0, 256)
		if err != nil {
			t.Fatal(err)
		}
		var original struct {
			Entries    []pale.EffectiveProfileEntry `json:"entries"`
			NextOffset *uint32                      `json:"nextOffset"`
		}
		if err := json.Unmarshal(document.Bytes, &original); err != nil {
			t.Fatal(err)
		}
		if original.NextOffset != nil {
			t.Fatal("fixture baseline requires another original page")
		}
		originalRows = append(originalRows, original.Entries...)
	}
	seen := make(map[pale.ObservationSubjectKind]bool)
	for index := range observations.Subjects {
		subject := &observations.Subjects[index]
		if seen[subject.Value.Kind] {
			continue
		}
		seen[subject.Value.Kind] = true
		rows := scanObservationProfile(t, snapshot, profile, workspace, subject, 1)
		wide := scanObservationProfile(t, snapshot, sameSource, workspace, subject, 256)
		if !reflect.DeepEqual(rows, wide) {
			t.Fatal("pagination or same-source profile allocation changed original rows")
		}
		for _, row := range rows {
			explanation, err := profile.Explain(row.ExplanationHandle)
			if err != nil || explanation == nil {
				t.Fatalf("original explanation handle lost: %v", err)
			}
			matched := false
			for _, retained := range originalRows {
				if reflect.DeepEqual(row, retained) {
					matched = true
					break
				}
			}
			if !matched {
				t.Fatal("association changed an original profile row")
			}
		}
		if subject.Value.Kind == pale.ObservationMachineSubject {
			if len(rows) == 0 {
				t.Fatal("machine mapping is empty")
			}
			alias := *subject
			alias.Value.MachineID = "display-only-not-a-selector"
			if got := scanObservationProfile(t, snapshot, profile, workspace, &alias, 256); !reflect.DeepEqual(got, rows) {
				t.Fatal("editing copied subject metadata changed the native selector")
			}
		}
	}
	if !seen[pale.ObservationMachineSubject] || !seen[pale.ObservationRepositorySubject] || !seen[pale.ObservationExecutionPathSubject] {
		t.Fatal("fixture lost required machine/repository/path witnesses")
	}
}

func TestObservationProfileSourceCoordinatesContinuationsAndExpiry(t *testing.T) {
	client, workspace, profile, snapshot, observations := observationProfileFixture(t)
	subject := machineObservation(t, &observations)
	request := pale.ObservationProfileRequest{MaximumInspected: 1, MaximumReturned: 1}
	first, err := snapshot.InspectProfile(profile, workspace, subject, request)
	if err != nil || first.Page == nil || first.Page.Continuation == nil {
		t.Fatalf("fixture needs continuation: %v", err)
	}
	defer first.Close()
	request.After = first.Page.Continuation
	changed := request
	changed.MaximumReturned = 2
	result, err := snapshot.InspectProfile(profile, workspace, subject, changed)
	requireProfileRefusal(t, result, err, pale.ObservationProfileContinuationMismatch)
	for index := range observations.Subjects {
		other := &observations.Subjects[index]
		if other.Value.Kind == pale.ObservationRepositorySubject {
			result, err := snapshot.InspectProfile(profile, workspace, other, request)
			requireProfileRefusal(t, result, err, pale.ObservationProfileContinuationMismatch)
			break
		}
	}
	for _, limits := range [][2]uint16{{0, 1}, {1, 0}, {257, 1}, {1, 257}} {
		result, err := snapshot.InspectProfile(profile, workspace, subject,
			pale.ObservationProfileRequest{MaximumInspected: limits[0], MaximumReturned: limits[1]})
		requireProfileRefusal(t, result, err, pale.ObservationProfileInvalidLimit)
	}
	otherSnapshot, _, err := profile.Observe()
	if err != nil {
		t.Fatal(err)
	}
	defer otherSnapshot.Close()
	result, err = otherSnapshot.InspectProfile(profile, workspace, subject, request)
	requireProfileRefusal(t, result, err, pale.ObservationProfileContinuationMismatch)
	fixture := readOneShotFixture(t)
	otherWorkspace, _, err := client.CompilePortableWorkspace(fixture.Workspace)
	if err != nil {
		t.Fatal(err)
	}
	defer otherWorkspace.Close()
	otherProfile := compileObservationProfile(t, otherWorkspace, fixture)
	result, err = snapshot.InspectProfile(profile, otherWorkspace, subject, request)
	requireProfileRefusal(t, result, err, pale.ObservationProfileSourceMismatch)
	result, err = snapshot.InspectProfile(otherProfile, otherWorkspace, subject, request)
	requireProfileRefusal(t, result, err, pale.ObservationProfileSourceMismatch)
	originalMachine := fixture.PrivateInputs.MachineBinding.Bytes
	fixture.PrivateInputs.MachineBinding.Bytes = bytes.ReplaceAll(originalMachine,
		[]byte("/captured/repository"), []byte("/captured/profile-association"))
	if bytes.Equal(originalMachine, fixture.PrivateInputs.MachineBinding.Bytes) {
		t.Fatal("fixture instance change did not change source")
	}
	changedProfile := compileObservationProfile(t, workspace, fixture)
	result, err = snapshot.InspectProfile(changedProfile, workspace, subject, request)
	requireProfileRefusal(t, result, err, pale.ObservationProfileCoordinateMismatch)
	changedProfile.Close()
	profile.Close()
	workspace.Close()
	result, err = snapshot.InspectProfile(otherProfile, otherWorkspace, subject, request)
	requireProfileRefusal(t, result, err, pale.ObservationProfileExpiredSource)
	remaining, err := snapshot.Page(pale.ObservationPageRequest{Filter: pale.AllObservations(), MaximumReturned: 1})
	if err != nil || remaining.Page == nil || len(remaining.Page.Facts) == 0 {
		t.Fatal("source expiry erased snapshot facts", err)
	}
	remaining.Close()
}

func TestObservationProfileClosePreflightAndCopiedCursor(t *testing.T) {
	_, workspace, profile, snapshot, observations := observationProfileFixture(t)
	subject := machineObservation(t, &observations)
	request := pale.ObservationProfileRequest{MaximumInspected: 1, MaximumReturned: 1}
	first, err := snapshot.InspectProfile(profile, workspace, subject, request)
	if err != nil || first.Page == nil || first.Page.Continuation == nil {
		t.Fatal("missing cursor", err)
	}
	alias := *first.Page.Continuation
	first.Page = nil
	first.Close()
	first.Close()
	request.After = &alias
	_, err = snapshot.InspectProfile(profile, workspace, subject, request)
	var bridge *pale.BridgeError
	if !errors.As(err, &bridge) {
		t.Fatalf("closed copied continuation accepted: %v", err)
	}
	request.After = nil
	for _, value := range []*pale.EffectiveProfile{nil, {}} {
		_, err = snapshot.InspectProfile(value, workspace, subject, request)
		if !errors.As(err, &bridge) {
			t.Fatalf("unavailable profile accepted: %v", err)
		}
	}
	_, foreignWorkspace, foreignProfile, foreignSnapshot, foreignObservations := observationProfileFixture(t)
	for _, call := range []func() error{
		func() error {
			_, err := snapshot.InspectProfile(foreignProfile, workspace, subject, request)
			return err
		},
		func() error {
			_, err := snapshot.InspectProfile(profile, foreignWorkspace, subject, request)
			return err
		},
		func() error {
			_, err := snapshot.InspectProfile(profile, workspace, machineObservation(t, &foreignObservations), request)
			return err
		},
		func() error {
			_, err := foreignSnapshot.InspectProfile(profile, workspace, subject, request)
			return err
		},
	} {
		if err := call(); !errors.As(err, &bridge) {
			t.Fatalf("foreign handle accepted: %v", err)
		}
	}
	var workers sync.WaitGroup
	errorsFound := make(chan error, 1)
	workers.Add(2)
	go func() { defer workers.Done(); profile.Close() }()
	go func() {
		defer workers.Done()
		result, err := snapshot.InspectProfile(profile, workspace, subject, request)
		defer result.Close()
		if err != nil {
			var failure *pale.BridgeError
			if !errors.As(err, &failure) {
				errorsFound <- err
			}
		}
	}()
	workers.Wait()
	close(errorsFound)
	for err := range errorsFound {
		t.Fatal(err)
	}
}

func TestObservationProfileResponseDecoderAndCustody(t *testing.T) {
	fixture, err := json.Marshal(readOneShotFixture(t).Request())
	if err != nil {
		t.Fatal(err)
	}
	runPackageOverlayTest(t, "observation_profile_response_test.go", "TestObservationProfileResponseTransfer", fixture)
}
