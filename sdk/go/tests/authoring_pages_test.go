package tests_test

import (
	"encoding/json"
	"errors"
	"reflect"
	"sync"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

type occurrenceRead func(*pale.AuthoringIdentity, uint16, uint16, *pale.AuthoringOccurrencePageCursor) (pale.AuthoringOccurrencePage, error)

func TestAuthoringPagePrivateResponseCustody(t *testing.T) {
	encoded, err := json.Marshal(readOneShotFixture(t).Request())
	if err != nil {
		t.Fatal(err)
	}
	runPackageOverlayTest(t, "authoring_page_response_test.go", "TestAuthoringPageResponseCustody", encoded)
}

func requireAuthoringCursorDiagnostic(t *testing.T, err error) {
	t.Helper()
	var diagnostic *pale.DiagnosticError
	if !errors.As(err, &diagnostic) || diagnostic.Code != "authoring_continuation_mismatch" || diagnostic.Effect != pale.DiagnosticEffectNotApplicable {
		t.Fatalf("cursor mismatch lost SDK diagnostic: %v", err)
	}
	var canonical struct {
		Code    string `json:"code"`
		Summary string `json:"summary"`
	}
	if json.Unmarshal(diagnostic.Canonical, &canonical) != nil || canonical.Code != diagnostic.Code || canonical.Summary != diagnostic.Summary {
		t.Fatal("diagnostic payload differs from typed error")
	}
}

func scanOccurrences(t *testing.T, read occurrenceRead, identity *pale.AuthoringIdentity, inspected, returned uint16) []pale.AuthoringOccurrence {
	t.Helper()
	page, err := read(identity, inspected, returned, nil)
	if err != nil {
		t.Fatal(err)
	}
	var rows []pale.AuthoringOccurrence
	normalize := func(value uint16) uint16 {
		if value == 0 {
			return 64
		}
		return min(value, 256)
	}
	for attempts := 0; attempts < 4096; attempts++ {
		if page.Inspected > normalize(inspected) || len(page.Occurrences) > int(normalize(returned)) || page.IsComplete != (page.Continuation == nil) {
			t.Fatal("page lost exact charged bounds or completion")
		}
		var copied uint32
		for _, row := range page.Occurrences {
			copied += uint32(len(row.Location.SourceLabel))
			for _, segment := range row.Location.Path {
				if segment.Field != nil {
					copied += uint32(len(*segment.Field))
				}
			}
		}
		if page.CopiedStringBytes != copied {
			t.Fatal("occurrence copied-string accounting changed")
		}
		rows = append(rows, page.Occurrences...)
		if page.IsComplete {
			page.Close()
			return rows
		}
		cursor := *page.Continuation
		next, err := read(identity, inspected, returned, &cursor)
		page.Close()
		if err != nil {
			t.Fatal(err)
		}
		if _, err := read(identity, inspected, returned, &cursor); err == nil {
			t.Fatal("closed cursor restarted the scan")
		}
		page = next
	}
	page.Close()
	t.Fatal("bounded scan did not terminate")
	return nil
}

func TestAuthoringOccurrencePagesPreserveCanonicalNavigationAndLimits(t *testing.T) {
	_, workspace, profile := navigationFixture(t)
	defer workspace.Close()
	defer profile.Close()
	for _, route := range []struct {
		name     string
		read     occurrenceRead
		navigate func(string, []pale.InstancePathSegment) (pale.AuthoringNavigation, error)
	}{
		{"portable", workspace.AuthoringOccurrences, workspace.NavigateAuthoring},
		{"effective", profile.AuthoringOccurrences, profile.NavigateAuthoring},
	} {
		t.Run(route.name, func(t *testing.T) {
			navigation, err := route.navigate("pale-workspace.toml", projectNavigationPath("projects", 0, "id"))
			if err != nil || navigation.Identity == nil || navigation.Definition == nil || navigation.ReferencesTruncated {
				t.Fatal("fixture needs complete project navigation", err)
			}
			defer navigation.Close()
			rows := scanOccurrences(t, route.read, navigation.Identity, 1, 1)
			if len(rows) != len(navigation.References)+1 {
				t.Fatal("scan differs from complete navigation")
			}
			locations := append([]pale.AuthoringSourceLocation{*navigation.Definition}, navigation.References...)
			seen := make(map[string]bool)
			for _, row := range rows {
				var path []pale.InstancePathSegment
				for _, part := range row.Location.Path {
					if part.Field != nil {
						path = append(path, pale.AuthoringField(*part.Field))
					} else {
						if part.Index == nil || *part.Index > uint64(^uint32(0)) {
							t.Fatal("fixture index outside input width")
						}
						path = append(path, pale.AuthoringIndex(uint32(*part.Index)))
					}
				}
				resolved, err := route.navigate(row.Location.SourceLabel, path)
				if err != nil || resolved.Identity == nil || resolved.Identity.Description != navigation.Identity.Description || resolved.OccurrenceRole == nil || *resolved.OccurrenceRole != row.Role {
					t.Fatal("occurrence lost source identity or role", err)
				}
				resolved.Close()
				matched := false
				for _, location := range locations {
					matched = matched || reflect.DeepEqual(location, row.Location)
				}
				key, _ := json.Marshal(row.Location)
				if !matched || seen[string(key)] {
					t.Fatal("missing or repeated occurrence")
				}
				seen[string(key)] = true
			}
			for _, limits := range [][2]uint16{{0, 0}, {256, 256}, {65535, 65535}, {1, 3}, {3, 1}} {
				if !reflect.DeepEqual(rows, scanOccurrences(t, route.read, navigation.Identity, limits[0], limits[1])) {
					t.Fatal("page partition changed occurrence order")
				}
			}
			first, err := route.read(navigation.Identity, 1, 1, nil)
			if err != nil || first.Continuation == nil {
				t.Fatal("fixture needs continuation", err)
			}
			defer first.Close()
			_, err = route.read(navigation.Identity, 2, 1, first.Continuation)
			requireAuthoringCursorDiagnostic(t, err)
			_, err = route.read(navigation.Identity, 1, 2, first.Continuation)
			requireAuthoringCursorDiagnostic(t, err)
			defaults, err := route.read(navigation.Identity, 0, 1, nil)
			if err != nil || defaults.Continuation == nil {
				t.Fatal("fixture needs default continuation", err)
			}
			defer defaults.Close()
			_, err = route.read(navigation.Identity, 64, 1, defaults.Continuation)
			requireAuthoringCursorDiagnostic(t, err)
			returnedDefault, err := route.read(navigation.Identity, 1, 0, nil)
			if err != nil || returnedDefault.Continuation == nil {
				t.Fatal("fixture needs a default-returned continuation", err)
			}
			defer returnedDefault.Close()
			_, err = route.read(navigation.Identity, 1, 64, returnedDefault.Continuation)
			requireAuthoringCursorDiagnostic(t, err)
			capped, err := route.read(navigation.Identity, 257, 1, nil)
			if err != nil || capped.Continuation == nil {
				t.Fatal("fixture needs a capped-inspected continuation", err)
			}
			defer capped.Close()
			_, err = route.read(navigation.Identity, 65535, 1, capped.Continuation)
			requireAuthoringCursorDiagnostic(t, err)
		})
	}
}

func TestAuthoringPagesKeepExactOwnerAndOriginalCursorCustody(t *testing.T) {
	client, workspace, profile := navigationFixture(t)
	defer workspace.Close()
	defer profile.Close()
	path := projectNavigationPath("repositories", 0, "id")
	navigation, err := workspace.NavigateAuthoring("pale-workspace.toml", path)
	if err != nil || navigation.Identity == nil {
		t.Fatal(err)
	}
	defer navigation.Close()
	page, err := workspace.AuthoringOccurrences(navigation.Identity, 1, 1, nil)
	if err != nil || page.Continuation == nil {
		t.Fatal("missing cursor", err)
	}
	defer page.Close()
	_, err = profile.AuthoringOccurrences(navigation.Identity, 1, 1, page.Continuation)
	requireAuthoringCursorDiagnostic(t, err)
	project, err := workspace.NavigateAuthoring("pale-workspace.toml", projectNavigationPath("projects", 0, "id"))
	if err != nil || project.Identity == nil {
		t.Fatal("fixture needs a distinct project identity", err)
	}
	defer project.Close()
	_, err = workspace.AuthoringOccurrences(project.Identity, 1, 1, page.Continuation)
	requireAuthoringCursorDiagnostic(t, err)
	machine, err := profile.NavigateAuthoring("machines/machine.toml", []pale.InstancePathSegment{pale.AuthoringField("machine"), pale.AuthoringField("id")})
	if err != nil || machine.Identity == nil {
		t.Fatal("fixture needs a private machine identity", err)
	}
	defer machine.Close()
	if rows := scanOccurrences(t, workspace.AuthoringOccurrences, machine.Identity, 1, 1); len(rows) != 0 {
		t.Fatal("portable index fabricated a private machine occurrence")
	}
	source := readOneShotFixture(t)
	fresh, _, err := client.CompilePortableWorkspace(source.Workspace)
	if err != nil {
		t.Fatal(err)
	}
	defer fresh.Close()
	_, err = fresh.AuthoringOccurrences(navigation.Identity, 1, 1, page.Continuation)
	requireAuthoringCursorDiagnostic(t, err)
	replacement, err := workspace.AuthoringOccurrences(navigation.Identity, 1, 1, nil)
	if err != nil || replacement.Continuation == nil {
		t.Fatal(err)
	}
	defer replacement.Close()
	original := *page.Continuation
	page.Continuation = replacement.Continuation
	page.Close()
	_, err = workspace.AuthoringOccurrences(navigation.Identity, 1, 1, &original)
	requireInvalidHandle(t, err)
	next, err := workspace.AuthoringOccurrences(navigation.Identity, 1, 1, replacement.Continuation)
	if err != nil {
		t.Fatal("closing page followed replacement field", err)
	}
	next.Close()
	identityCopy := *navigation.Identity
	navigation.Close()
	_, err = workspace.AuthoringOccurrences(&identityCopy, 1, 1, replacement.Continuation)
	requireInvalidHandle(t, err)
	// The cursor remains releasable; it does not restore the closed request identity.
	replacement.Close()
	var zero pale.AuthoringOccurrencePageCursor
	_, err = workspace.AuthoringOccurrences(&identityCopy, 1, 1, &zero)
	requireInvalidHandle(t, err)
	var absent *pale.AuthoringOccurrencePage
	absent.Close()
}

func TestEffectiveAuthoringPagesPreserveRowsCoordinatesAndExplanations(t *testing.T) {
	client, workspace, profile := navigationFixture(t)
	defer workspace.Close()
	defer profile.Close()
	navigation, err := workspace.NavigateAuthoring("pale-workspace.toml", projectNavigationPath("repositories", 0, "id"))
	if err != nil || navigation.Identity == nil {
		t.Fatal(err)
	}
	defer navigation.Close()
	legacy, err := profile.InspectEffectiveAuthoring(navigation.Identity)
	if err != nil || legacy.Truncated || len(legacy.Entries) < 2 {
		t.Fatal("fixture needs multiple complete associations", err)
	}
	first, err := profile.InspectEffectiveAuthoringPage(navigation.Identity, 1, nil)
	if err != nil || first.Continuation == nil {
		t.Fatal("missing profile continuation", err)
	}
	defer first.Close()
	_, err = profile.InspectEffectiveAuthoringPage(navigation.Identity, 2, first.Continuation)
	requireAuthoringCursorDiagnostic(t, err)
	page := first
	var rows []pale.EffectiveProfileEntry
	for attempts := 0; attempts < 4096; attempts++ {
		if page.Coordinates != legacy.Coordinates || len(page.Entries) > 1 {
			t.Fatal("coordinates or row limit changed")
		}
		for _, row := range page.Entries {
			explanation, err := profile.Explain(row.ExplanationHandle)
			if err != nil || explanation == nil || explanation.Handle != row.ExplanationHandle {
				t.Fatal("explanation lost", err)
			}
		}
		rows = append(rows, page.Entries...)
		if page.Continuation == nil {
			page.Close()
			break
		}
		next, err := profile.InspectEffectiveAuthoringPage(navigation.Identity, 1, page.Continuation)
		page.Close()
		if err != nil {
			t.Fatal(err)
		}
		page = next
	}
	if !reflect.DeepEqual(rows, legacy.Entries) {
		t.Fatal("paged rows differ from canonical retained associations")
	}
	for _, limit := range []uint16{0, 64, 256, 65535} {
		complete, err := profile.InspectEffectiveAuthoringPage(navigation.Identity, limit, nil)
		if err != nil || complete.Continuation != nil || !reflect.DeepEqual(complete.Entries, rows) {
			t.Fatal("default/cap changed associations", err)
		}
		complete.Close()
	}
	project, err := workspace.NavigateAuthoring("pale-workspace.toml", projectNavigationPath("projects", 0, "id"))
	if err != nil {
		t.Fatal(err)
	}
	defer project.Close()
	empty, err := profile.InspectEffectiveAuthoringPage(project.Identity, 1, nil)
	if err != nil || len(empty.Entries) != 0 || empty.Continuation != nil {
		t.Fatal("ordinary empty association lost", err)
	}
	empty.Close()
	first, err = profile.InspectEffectiveAuthoringPage(navigation.Identity, 1, nil)
	if err != nil {
		t.Fatal(err)
	}
	defer first.Close()
	_, err = profile.InspectEffectiveAuthoringPage(project.Identity, 1, first.Continuation)
	requireAuthoringCursorDiagnostic(t, err)
	source := readOneShotFixture(t)
	fresh, coordinates, err := workspace.CompileEffectiveProfile(pale.EffectiveProfileRequest{ProjectID: source.ProjectID, ExecutionWorkspaceID: source.ExecutionWorkspaceID, MachineID: source.MachineID, PrivateInputs: source.PrivateInputs, PackageDocuments: source.PackageDocuments})
	if err != nil || coordinates != legacy.Coordinates {
		t.Fatal("equal-coordinate fixture changed", err)
	}
	defer fresh.Close()
	_, err = fresh.InspectEffectiveAuthoringPage(navigation.Identity, 1, first.Continuation)
	requireAuthoringCursorDiagnostic(t, err)
	replacement, err := profile.InspectEffectiveAuthoringPage(navigation.Identity, 1, nil)
	if err != nil {
		t.Fatal(err)
	}
	defer replacement.Close()
	alias := *first.Continuation
	first.Continuation = replacement.Continuation
	first.Close()
	_, err = profile.InspectEffectiveAuthoringPage(navigation.Identity, 1, &alias)
	requireInvalidHandle(t, err)
	next, err := profile.InspectEffectiveAuthoringPage(navigation.Identity, 1, replacement.Continuation)
	if err != nil {
		t.Fatal("group close changed replacement", err)
	}
	next.Close()
	identityCopy := *navigation.Identity
	navigation.Close()
	_, err = profile.InspectEffectiveAuthoringPage(&identityCopy, 1, replacement.Continuation)
	requireInvalidHandle(t, err)
	client.Close()
	if !reflect.DeepEqual(rows, legacy.Entries) {
		t.Fatal("client closure changed copied rows")
	}
}

func TestAuthoringPageCloseSerializesWithQueriesAndClientClose(t *testing.T) {
	client, workspace, profile := navigationFixture(t)
	navigation, err := workspace.NavigateAuthoring("pale-workspace.toml", projectNavigationPath("repositories", 0, "id"))
	if err != nil {
		t.Fatal(err)
	}
	page, err := workspace.AuthoringOccurrences(navigation.Identity, 1, 1, nil)
	if err != nil || page.Continuation == nil {
		t.Fatal(err)
	}
	association, err := profile.InspectEffectiveAuthoringPage(navigation.Identity, 1, nil)
	if err != nil || association.Continuation == nil {
		t.Fatal(err)
	}
	cursor := *page.Continuation
	profileCursor := *association.Continuation
	start := make(chan struct{})
	var group sync.WaitGroup
	for _, closeValue := range []func(){page.Close, cursor.Close, association.Close, profileCursor.Close, navigation.Close, workspace.Close, profile.Close, client.Close} {
		group.Add(1)
		go func(closeValue func()) { defer group.Done(); <-start; closeValue() }(closeValue)
	}
	close(start)
	_, _ = workspace.AuthoringOccurrences(navigation.Identity, 1, 1, &cursor)
	_, _ = profile.InspectEffectiveAuthoringPage(navigation.Identity, 1, &profileCursor)
	group.Wait()
	failed, err := workspace.AuthoringOccurrences(navigation.Identity, 1, 1, &cursor)
	if err == nil || failed.IsComplete || failed.Continuation != nil || len(failed.Occurrences) != 0 {
		t.Fatal("closed handle fabricated completion")
	}
}
