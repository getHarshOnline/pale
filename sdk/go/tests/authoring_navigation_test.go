package tests_test

import (
	"encoding/json"
	"errors"
	"math"
	"reflect"
	"strings"
	"sync"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func requireInvalidHandle(t *testing.T, err error) {
	t.Helper()
	var bridge *pale.BridgeError
	if !errors.As(err, &bridge) || bridge.Code != "invalid_handle" {
		t.Fatalf("identity handle remained usable: %v", err)
	}
}

func navigationFixture(t *testing.T) (*pale.Client, *pale.PortableWorkspace, *pale.EffectiveProfile) {
	t.Helper()
	client := newClient(t)
	source := readOneShotFixture(t)
	workspace, _, err := client.CompilePortableWorkspace(source.Workspace)
	if err != nil {
		t.Fatal(err)
	}
	profile, _, err := workspace.CompileEffectiveProfile(pale.EffectiveProfileRequest{
		ProjectID: source.ProjectID, ExecutionWorkspaceID: source.ExecutionWorkspaceID,
		MachineID: source.MachineID, PrivateInputs: source.PrivateInputs, PackageDocuments: source.PackageDocuments,
	})
	if err != nil {
		t.Fatal(err)
	}
	return client, workspace, profile
}

func projectNavigationPath(collection string, index uint32, field string) []pale.InstancePathSegment {
	return []pale.InstancePathSegment{pale.AuthoringField("workspace"), pale.AuthoringField("desired_topology"), pale.AuthoringField(collection), pale.AuthoringIndex(index), pale.AuthoringField(field)}
}

func TestAuthoringNavigationNativeTypedResults(t *testing.T) {
	_, workspace, profile := navigationFixture(t)
	routes := []struct {
		name     string
		navigate func(string, []pale.InstancePathSegment) (pale.AuthoringNavigation, error)
	}{{"portable", workspace.NavigateAuthoring}, {"profile", profile.NavigateAuthoring}}
	for _, route := range routes {
		t.Run(route.name, func(t *testing.T) {
			navigation, err := route.navigate("pale-workspace.toml", projectNavigationPath("project_execution_workspaces", 0, "project_id"))
			if err != nil {
				t.Fatal(err)
			}
			defer navigation.Close()
			expected := pale.AuthoringIdentityValue{Kind: pale.AuthoringIdentityProject, Value: "demo", AuthoredValue: "demo"}
			if navigation.Identity == nil || navigation.Identity.Description != expected || navigation.OccurrenceRole == nil || *navigation.OccurrenceRole != pale.AuthoringReference || navigation.Definition == nil || navigation.Definition.SourceLabel != "pale-workspace.toml" {
				t.Fatalf("reference identity lost: %#v", navigation)
			}
			path := navigation.Definition.Path
			if len(path) != 5 || path[2].Field == nil || *path[2].Field != "projects" || path[3].Index == nil || *path[3].Index != 0 || path[4].Field == nil || *path[4].Field != "id" {
				t.Fatal("definition path lost exact field/index coordinates")
			}
			if len(navigation.References) == 0 || len(navigation.Alternatives) == 0 || navigation.ReferencesTruncated || navigation.AlternativesTruncated {
				t.Fatal("bounded navigation lost retained members")
			}
			original, err := profile.InspectEffectiveAuthoring(navigation.Identity)
			if err != nil {
				t.Fatal(err)
			}
			navigation.Identity.Description = pale.AuthoringIdentityValue{Kind: pale.AuthoringIdentityMachine, Value: "edited-display"}
			repeated, err := profile.InspectEffectiveAuthoring(navigation.Identity)
			if err != nil || !reflect.DeepEqual(original, repeated) {
				t.Fatal("editing copied description changed native identity")
			}
			definition, err := route.navigate("pale-workspace.toml", projectNavigationPath("projects", 0, "id"))
			if err != nil {
				t.Fatal(err)
			}
			defer definition.Close()
			if definition.OccurrenceRole == nil || *definition.OccurrenceRole != pale.AuthoringDefinition {
				t.Fatal("definition role lost")
			}
			completion, err := route.navigate("pale-workspace.toml", projectNavigationPath("project_repositories", 1, "project_id"))
			if err != nil {
				t.Fatal(err)
			}
			defer completion.Close()
			if completion.Identity != nil || completion.OccurrenceRole != nil || completion.Definition != nil || len(completion.References) != 0 || len(completion.Alternatives) != 1 || completion.Alternatives[0].Description != expected {
				t.Fatal("structural completion was collapsed into missing identity")
			}
			joined, err := profile.InspectEffectiveAuthoring(&completion.Alternatives[0])
			if err != nil || !reflect.DeepEqual(original, joined) {
				t.Fatal("completion alternative is not the same reusable semantic identity")
			}
			missing, err := route.navigate("absent.toml", []pale.InstancePathSegment{})
			if err != nil {
				t.Fatal(err)
			}
			defer missing.Close()
			if missing.Identity != nil || missing.Definition != nil || missing.OccurrenceRole != nil || len(missing.References) != 0 || len(missing.Alternatives) != 0 || missing.ReferencesTruncated || missing.AlternativesTruncated {
				t.Fatal("missing occurrence did not remain ordinary absence")
			}
		})
	}
}

func TestAuthoringNavigationCloseKeepsOriginalCustody(t *testing.T) {
	_, workspace, profile := navigationFixture(t)
	navigation, err := workspace.NavigateAuthoring("pale-workspace.toml", projectNavigationPath("project_execution_workspaces", 0, "project_id"))
	if err != nil {
		t.Fatal(err)
	}
	replacement, err := profile.NavigateAuthoring("pale-workspace.toml", projectNavigationPath("projects", 0, "run_policy"))
	if err != nil {
		t.Fatal(err)
	}
	defer replacement.Close()
	original := *navigation.Identity
	alternative := navigation.Alternatives[0]
	original.Close()
	_, err = profile.InspectEffectiveAuthoring(navigation.Identity)
	requireInvalidHandle(t, err)
	if _, err = profile.InspectEffectiveAuthoring(&alternative); err != nil {
		t.Fatal("individual close released a sibling")
	}
	snapshot, _, err := profile.Observe()
	if err != nil {
		t.Fatal(err)
	}
	defer snapshot.Close()
	navigation.Identity = replacement.Identity
	navigation.Alternatives = replacement.Alternatives
	navigation.Close()
	navigation.Close()
	_, err = profile.InspectEffectiveAuthoring(&alternative)
	requireInvalidHandle(t, err)
	if _, err = profile.InspectEffectiveAuthoring(replacement.Identity); err != nil {
		t.Fatal("aggregate close followed reassigned public handle")
	}
	if _, err = snapshot.VisualLens(pale.ObservationLensRequest{
		Filter:                   pale.AllObservations(),
		MaximumInspected:         1,
		MaximumNodes:             1,
		MaximumCopiedStringBytes: ^uint32(0),
		MaximumCausalPathLookups: 1,
		MaximumCausalSteps:       1,
		Cancellation:             pale.ObservationActive,
	}); err != nil {
		t.Fatal("identity close affected snapshot")
	}
	var absentIdentity *pale.AuthoringIdentity
	absentIdentity.Close()
	var absentNavigation *pale.AuthoringNavigation
	absentNavigation.Close()
	var absentWorkspace *pale.PortableWorkspace
	_, err = absentWorkspace.NavigateAuthoring("", nil)
	requireInvalidHandle(t, err)
	var absentProfile *pale.EffectiveProfile
	_, err = absentProfile.NavigateAuthoring("", nil)
	requireInvalidHandle(t, err)
	_, err = absentProfile.InspectEffectiveAuthoring(replacement.Identity)
	requireInvalidHandle(t, err)
}

func TestAuthoringNavigationCloseSerializesWithJoinAndClient(t *testing.T) {
	client, workspace, profile := navigationFixture(t)
	navigation, err := workspace.NavigateAuthoring("pale-workspace.toml", projectNavigationPath("project_execution_workspaces", 0, "project_id"))
	if err != nil {
		t.Fatal(err)
	}
	identity := *navigation.Identity
	start := make(chan struct{})
	var group sync.WaitGroup
	for _, close := range []func(){navigation.Close, identity.Close, client.Close} {
		group.Add(1)
		go func(close func()) { defer group.Done(); <-start; close() }(close)
	}
	close(start)
	_, _ = profile.InspectEffectiveAuthoring(&identity)
	group.Wait()
	_, err = profile.InspectEffectiveAuthoring(&identity)
	requireInvalidHandle(t, err)
}

func TestAuthoringNavigationValueCodecs(t *testing.T) {
	// This is closed-codec coverage, not native examples of every namespace.
	for _, kind := range []pale.AuthoringIdentityKind{
		pale.AuthoringIdentitySourceDocument,
		pale.AuthoringIdentityWorkspace,
		pale.AuthoringIdentityProject,
		pale.AuthoringIdentityExecutionWorkspace,
		pale.AuthoringIdentityRepository,
		pale.AuthoringIdentityExecutionPath,
		pale.AuthoringIdentityModule,
		pale.AuthoringIdentityModuleRelease,
		pale.AuthoringIdentityComponent,
		pale.AuthoringIdentityExport,
		pale.AuthoringIdentityNamespace,
		pale.AuthoringIdentityDeclaration,
		pale.AuthoringIdentityResource,
		pale.AuthoringIdentityRunPolicy,
		pale.AuthoringIdentityLifecycleBehavior,
		pale.AuthoringIdentityCapability,
		pale.AuthoringIdentityAuthorityRoot,
		pale.AuthoringIdentityProviderBinding,
		pale.AuthoringIdentityMachine,
		pale.AuthoringIdentityResponsibilityDerivation,
		pale.AuthoringIdentityResponsibilityProjection,
		pale.AuthoringIdentityResponsibilityGenealogyLink,
		pale.AuthoringIdentityLogicalScope,
		pale.AuthoringIdentityLogicalOrganization,
		pale.AuthoringIdentityOrganizationalGroup,
		pale.AuthoringIdentityDisciplineTree,
		pale.AuthoringIdentityOrganizationalRole,
		pale.AuthoringIdentityOrganizationalOccupant,
		pale.AuthoringIdentityOrganizationalProvider,
		pale.AuthoringIdentityCommunicationActor,
		pale.AuthoringIdentityLeadershipTopic,
		pale.AuthoringIdentityOrganizationalRelation,
		pale.AuthoringIdentityOrganizationalObject,
		pale.AuthoringIdentityGroupLensProfile,
		pale.AuthoringIdentityDirectionalService,
		pale.AuthoringIdentityOrganizationalProjection,
		pale.AuthoringIdentityOrganizationalInspection,
		pale.AuthoringIdentityCompositionParticipant,
		pale.AuthoringIdentityCompositionResponsibilityContract,
		pale.AuthoringIdentityCompositionParticipantManifest,
		pale.AuthoringIdentityCompositionProvider,
		pale.AuthoringIdentityCompositionProviderBinding,
		pale.AuthoringIdentityCompositionEntryContract,
		pale.AuthoringIdentityCompositionRequestTranslation,
		pale.AuthoringIdentityCompositionResultTranslation,
		pale.AuthoringIdentityCompositionTerminationContract,
		pale.AuthoringIdentityCompositionSettlementContract,
		pale.AuthoringIdentityCompositionContinuityContract,
		pale.AuthoringIdentityCompositionEdge,
		pale.AuthoringIdentityExperiencePath,
		pale.AuthoringIdentityPackageDoctrine,
		pale.AuthoringIdentityPackageOperationArchetype,
		pale.AuthoringIdentityPackageOperation,
		pale.AuthoringIdentityPackageRoleArchetype,
		pale.AuthoringIdentityPackageRole,
		pale.AuthoringIdentityPackageStack,
		pale.AuthoringIdentityPackageRunMode,
		pale.AuthoringIdentityPackagePolicyFamily,
		pale.AuthoringIdentityPackageGoverningOrder,
		pale.AuthoringIdentityPackageCapabilityPolicy,
		pale.AuthoringIdentityPackageToolRecipe,
		pale.AuthoringIdentityPackage,
	} {
		encoded, err := json.Marshal(kind)
		if err != nil {
			t.Fatal(err)
		}
		var actual pale.AuthoringIdentityKind
		if err := json.Unmarshal(encoded, &actual); err != nil || actual != kind {
			t.Fatalf("closed identity namespace rejected: %s", kind)
		}
	}
	for _, role := range []pale.AuthoringOccurrenceRole{pale.AuthoringDefinition, pale.AuthoringReference, pale.AuthoringProjection} {
		var actual pale.AuthoringOccurrenceRole
		encoded, err := json.Marshal(role)
		if err != nil {
			t.Fatal(err)
		}
		if err := json.Unmarshal(encoded, &actual); err != nil || actual != role {
			t.Fatal("occurrence role did not round trip")
		}
	}
	role := pale.AuthoringReference
	if err := json.Unmarshal([]byte(`"private-marker"`), &role); err == nil || role != pale.AuthoringReference || strings.Contains(err.Error(), "private-marker") {
		t.Fatal("unknown occurrence role changed or leaked")
	}
	identity := pale.AuthoringIdentityValue{Kind: pale.AuthoringIdentityProject, Value: "demo", AuthoredValue: "demo"}
	for _, input := range []string{
		`null`, `[]`, `{"kind":"invented","value":"private-marker","authored_value":"demo"}`,
		`{"kind":"project","value":"demo"}`, `{"kind":"project","value":null,"authored_value":"demo"}`,
		`{"kind":"project","value":"demo","authored_value":"demo","Value":"private-marker"}`,
		`{"kind":"project","value":"demo","authored_value":"demo","value":"private-marker"}`,
	} {
		actual := identity
		err := json.Unmarshal([]byte(input), &actual)
		if err == nil || actual != identity || strings.Contains(err.Error(), "private-marker") {
			t.Fatalf("identity decoder weakened atomicity/privacy: %v", err)
		}
	}
	var opaque pale.AuthoringIdentity
	if _, ok := any(&opaque).(json.Unmarshaler); ok {
		t.Fatal("description decoder became an opaque identity constructor")
	}
	var location pale.AuthoringSourceLocation
	if err := json.Unmarshal([]byte(`{"source_label":"logical.toml","path":[{"kind":"field","value":""},{"kind":"index","value":18446744073709551615}]}`), &location); err != nil {
		t.Fatal(err)
	}
	if location.Path[0].Field == nil || *location.Path[0].Field != "" || location.Path[0].Index != nil || location.Path[1].Index == nil || *location.Path[1].Index != math.MaxUint64 || location.Path[1].Field != nil {
		t.Fatal("output path lost empty field or unsigned width")
	}
	for _, input := range []string{
		`{"source_label":"x","Source_label":"private-marker","path":[]}`,
		`{"source_label":"x","path":null}`,
		`{"source_label":"x","path":[{"kind":"index","value":-1}]}`,
		`{"source_label":"x","path":[{"kind":"index","value":18446744073709551616}]}`,
		`{"source_label":"x","path":[{"kind":"field","value":42}]}`,
		`{"source_label":"x","path":[{"kind":"field","value":null}]}`,
		`{"source_label":"x","path":[{"kind":"index","value":null}]}`,
		`{"source_label":"x","path":[{"kind":"index","value":"private-marker"}]}`,
		`{"source_label":"x","path":[{"kind":"field","value":"x","Value":"private-marker"}]}`,
		`{"source_label":"x","path":[{"kind":"other","value":"private-marker"}]}`,
	} {
		actual := location
		err := json.Unmarshal([]byte(input), &actual)
		if err == nil || !reflect.DeepEqual(actual, location) || strings.Contains(err.Error(), "private-marker") {
			t.Fatalf("location decoder weakened atomicity/privacy: %v", err)
		}
	}
}

func TestAuthoringNavigationPrivateDecoding(t *testing.T) {
	runPackageOverlayTest(t, "authoring_navigation_decode_test.go", "TestAuthoringNavigationResponseDecoder", nil)
}
