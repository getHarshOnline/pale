package tests_test

import (
	"bytes"
	"crypto/sha256"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

const fixtureRoot = "fixtures/cross-leaf.v1"

func TestNativeABIConformsToCanonicalFixture(t *testing.T) {
	client, err := pale.NewClient()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(client.Close)

	capabilities, err := client.Capabilities()
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(capabilities, expectedCapabilities) {
		t.Fatalf("Go capability projection drifted:\nwant %q\n got %q", expectedCapabilities, capabilities)
	}

	var bundle struct {
		MemberCount          uint32 `json:"member_count"`
		SchemaManifestSHA256 string `json:"schema_manifest_sha256"`
		SchemaSetSHA256      string `json:"schema_set_sha256"`
	}
	readFixture(t, "bundle-identity.json", &bundle)
	identity, err := client.ContractBundleIdentity()
	if err != nil {
		t.Fatal(err)
	}
	var actualBundle struct {
		MemberCount          uint32 `json:"memberCount"`
		SchemaManifestSHA256 string `json:"schemaManifestSha256"`
		SchemaSetSHA256      string `json:"schemaSetSha256"`
	}
	decode(t, identity.Bytes, &actualBundle)
	if actualBundle.MemberCount != bundle.MemberCount || actualBundle.SchemaManifestSHA256 != bundle.SchemaManifestSHA256 || actualBundle.SchemaSetSHA256 != bundle.SchemaSetSHA256 {
		t.Fatalf("bundle identity drifted: want %#v, got %#v", bundle, actualBundle)
	}

	var contract struct {
		Schema struct {
			RegistryID   string `json:"registry_id"`
			SchemaURI    string `json:"schema_uri"`
			WireSchemaID string `json:"wire_schema_id"`
			SchemaSHA256 string `json:"schema_sha256"`
			ByteLength   int    `json:"byte_length"`
		} `json:"schema"`
		Projection struct {
			Code    string `json:"code"`
			Summary string `json:"summary"`
		} `json:"projection"`
		CanonicalDiagnostic string `json:"canonical_diagnostic"`
		OutcomeUnknown      struct {
			Code                string `json:"code"`
			Summary             string `json:"summary"`
			CanonicalDiagnostic string `json:"canonical_diagnostic"`
		} `json:"outcome_unknown"`
	}
	readFixture(t, "contract-diagnostic.json", &contract)
	schema, err := client.ContractSchema(contract.Schema.RegistryID)
	if err != nil {
		t.Fatal(err)
	}
	if schema.RegistryID != contract.Schema.RegistryID || schema.SchemaURI != contract.Schema.SchemaURI || schema.WireSchemaID == nil || *schema.WireSchemaID != contract.Schema.WireSchemaID || schema.SchemaSHA256 != contract.Schema.SchemaSHA256 || len(schema.Bytes) != contract.Schema.ByteLength {
		t.Fatalf("diagnostic schema projection drifted: %#v", schema)
	}
	diagnostic, err := client.ProjectDiagnostic(pale.DiagnosticProjection{
		Code: contract.Projection.Code, Stage: pale.DiagnosticInputAdmission, Disposition: pale.DiagnosticInvalidInput,
		Summary: contract.Projection.Summary, Retry: pale.DiagnosticRetryAfterCorrection, Effect: pale.DiagnosticEffectNotApplicable,
	})
	if err != nil {
		t.Fatal(err)
	}
	if string(diagnostic.Bytes) != contract.CanonicalDiagnostic {
		t.Fatalf("canonical diagnostic drifted:\nwant %s\n got %s", contract.CanonicalDiagnostic, diagnostic.Bytes)
	}
	validated, err := client.ValidateContractDocument(contract.Schema.RegistryID, diagnostic.Bytes)
	if err != nil {
		t.Fatal(err)
	}
	if string(validated.Bytes) != contract.CanonicalDiagnostic {
		t.Fatalf("validated diagnostic bytes drifted: %s", validated.Bytes)
	}
	admitted, err := client.AdmitDiagnostic(diagnostic.Bytes)
	if err != nil || string(admitted.Bytes) != contract.CanonicalDiagnostic {
		t.Fatalf("diagnostic admission drifted: %s (%v)", admitted.Bytes, err)
	}
	outcomeUnknown, err := client.ProjectDiagnostic(pale.DiagnosticProjection{
		Code: contract.OutcomeUnknown.Code, Stage: pale.DiagnosticMaterialization,
		Disposition: pale.DiagnosticEffectOutcomeUnknown, Summary: contract.OutcomeUnknown.Summary,
		Retry: pale.DiagnosticReconcileFirst, Effect: pale.DiagnosticEffectUnknown,
	})
	if err != nil || string(outcomeUnknown.Bytes) != contract.OutcomeUnknown.CanonicalDiagnostic {
		t.Fatalf("outcome-unknown diagnostic drifted: %s (%v)", outcomeUnknown.Bytes, err)
	}
	_, err = client.ContractSchema("not-a-selected-schema")
	var contractError *pale.DiagnosticError
	if !errors.As(err, &contractError) || contractError.Code != "contract_rejected" {
		t.Fatalf("unknown schema was not preserved as a diagnostic: %T %v", err, err)
	}

	compiledIdentity, err := client.CompiledInterfaceIdentity()
	if err != nil || compiledIdentity.Version != "0.0.1" || len(compiledIdentity.Canonical) == 0 {
		t.Fatalf("local executable identity failed: %#v (%v)", compiledIdentity, err)
	}
	_, _, err = client.CompileLocalOneShot(pale.LocalWorkspaceRequest{Manifest: "relative.toml"})
	var diagnosticError *pale.DiagnosticError
	if !errors.As(err, &diagnosticError) || diagnosticError.Effect == pale.DiagnosticEffectUnknown {
		t.Fatalf("invalid local locator did not fail safely: %T %v", err, err)
	}
}

func TestPortableAndEffectiveProfileJourneys(t *testing.T) {
	client := newClient(t)
	var expected struct {
		RootManifest             string   `json:"root_manifest"`
		DocumentLabels           []string `json:"document_labels"`
		WorkspaceAuthoringLimits struct {
			MaxDocuments     uint64 `json:"max_documents"`
			MaxDocumentBytes uint64 `json:"max_document_bytes,string"`
			MaxTotalBytes    uint64 `json:"max_total_bytes,string"`
		} `json:"workspace_authoring_limits"`
		Compilation struct {
			WorkspaceID               string `json:"workspace_id"`
			PrincipalUniverseDigest   string `json:"principal_universe_digest"`
			SemanticIRDigest          string `json:"semantic_ir_digest"`
			AdmissionEvidenceDigest   string `json:"admission_evidence_digest"`
			SourceDocumentCount       uint32 `json:"source_document_count"`
			DerivationSourceCount     uint32 `json:"derivation_source_count"`
			DerivationNodeCount       uint32 `json:"derivation_node_count"`
			AdmissionEvidenceCount    uint32 `json:"admission_evidence_count"`
			AdoptedReleaseCount       uint32 `json:"adopted_release_count"`
			PackageReleaseCount       uint32 `json:"package_release_count"`
			NormativePackReleaseCount uint32 `json:"normative_pack_release_count"`
		} `json:"compilation"`
		Comparison struct {
			PriorPrincipalUniverseDigest   string `json:"prior_principal_universe_digest"`
			CurrentPrincipalUniverseDigest string `json:"current_principal_universe_digest"`
			PriorSemanticIRDigest          string `json:"prior_semantic_ir_digest"`
			CurrentSemanticIRDigest        string `json:"current_semantic_ir_digest"`
			ReceiptDigest                  string `json:"receipt_digest"`
			ChangeRootCount                uint32 `json:"change_root_count"`
			AffectedCount                  uint32 `json:"affected_count"`
			UnaffectedCount                uint32 `json:"unaffected_count"`
		} `json:"comparison"`
		ChangeRoots struct {
			EntryCount uint32  `json:"entry_count"`
			NextOffset *uint32 `json:"next_offset"`
		} `json:"change_roots"`
		FirstImpact struct {
			NextOffset            uint32 `json:"next_offset"`
			SourceID              string `json:"source_id"`
			State                 string `json:"state"`
			ContributingRootCount uint32 `json:"contributing_root_count"`
		} `json:"first_impact"`
	}
	readFixture(t, "portable-journey.json", &expected)
	documents := make([]pale.Document, len(expected.DocumentLabels))
	for index, label := range expected.DocumentLabels {
		documents[index] = fixtureDocument(t, label)
	}
	limits, err := client.AuthoringIngressLimits(pale.AuthoringWorkspace)
	if err != nil {
		t.Fatal(err)
	}
	if limits.MaxDocuments != expected.WorkspaceAuthoringLimits.MaxDocuments ||
		limits.MaxDocumentBytes != expected.WorkspaceAuthoringLimits.MaxDocumentBytes ||
		limits.MaxTotalBytes != expected.WorkspaceAuthoringLimits.MaxTotalBytes {
		t.Fatalf("workspace ingress limits drifted: want %#v, got %#v", expected.WorkspaceAuthoringLimits, limits)
	}
	manifest := fixtureDocument(t, expected.RootManifest)
	if err := client.ValidateAuthoringDocument(pale.RoleWorkspaceManifest, manifest.Label, manifest.Bytes); err != nil {
		t.Fatal(err)
	}
	inspection, err := client.InspectStandaloneAuthoring(
		pale.RoleWorkspaceManifest,
		[]pale.StructuralPathSegment{pale.AuthoringFieldPath("workspace")},
	)
	if err != nil {
		t.Fatal(err)
	}
	if inspection.State != pale.AuthoringAvailable || inspection.Node == nil || len(inspection.Node.Fields) == 0 {
		t.Fatalf("standalone grammar fields unavailable: %#v", inspection)
	}
	overBound := make([]pale.StructuralPathSegment, 65)
	for index := range overBound {
		overBound[index] = pale.AuthoringFieldPath(fmt.Sprintf("over-bound-%d", index))
	}
	_, err = client.InspectStandaloneAuthoring(pale.RoleWorkspaceManifest, overBound)
	var authoringError *pale.DiagnosticError
	if !errors.As(err, &authoringError) ||
		authoringError.Code != "authoring_path_too_deep" ||
		authoringError.Stage != pale.DiagnosticInputAdmission ||
		authoringError.Disposition != pale.DiagnosticInvalidInput ||
		authoringError.Retry != pale.DiagnosticRetryAfterCorrection ||
		authoringError.Effect != pale.DiagnosticEffectNotApplicable {
		t.Fatalf("oversized authoring path did not preserve the canonical diagnostic: %T %v", err, err)
	}

	request := pale.PortableWorkspaceRequest{RootManifest: expected.RootManifest, Documents: documents}
	prior, priorSummary, err := client.CompilePortableWorkspace(request)
	if err != nil {
		t.Fatal(err)
	}
	current, currentSummary, err := client.CompilePortableWorkspace(request)
	if err != nil {
		t.Fatal(err)
	}
	actualCompilation := expected.Compilation
	decode(t, priorSummary.Bytes, &actualCompilation)
	if !reflect.DeepEqual(actualCompilation, expected.Compilation) || !reflect.DeepEqual(priorSummary.Bytes, currentSummary.Bytes) {
		t.Fatalf("portable compilation drifted: want %#v, got %#v", expected.Compilation, actualCompilation)
	}
	workspaceInspection, err := prior.InspectAuthoring(
		expected.RootManifest,
		[]pale.StructuralPathSegment{pale.AuthoringFieldPath("workspace")},
	)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(workspaceInspection, inspection) {
		t.Fatal("standalone and retained workspace grammar facts differ")
	}
	navigation, err := prior.NavigateAuthoring(expected.RootManifest, []pale.InstancePathSegment{
		pale.AuthoringField("workspace"), pale.AuthoringField("workspace_id"),
	})
	if err != nil || navigation.Identity == nil {
		t.Fatalf("portable source navigation lost its admitted identity: %v", err)
	}

	comparison, comparisonSummary, err := client.ComparePortableWorkspaces(prior, current)
	if err != nil {
		t.Fatal(err)
	}
	actualComparison := expected.Comparison
	decode(t, comparisonSummary.Bytes, &actualComparison)
	if !reflect.DeepEqual(actualComparison, expected.Comparison) {
		t.Fatalf("portable comparison drifted: want %#v, got %#v", expected.Comparison, actualComparison)
	}
	subjectComparison, subjectDocument, err := comparison.CompareSubject(pale.WorkspaceIdentitySubject("empty-adopter"))
	if err != nil {
		t.Fatal(err)
	}
	shared, ok := subjectComparison.Values.Value.(pale.SharedWorkspaceSubject)
	if !ok || shared.Comparison != pale.WorkspaceValuesEqual || subjectComparison.ReceiptDigest != actualComparison.ReceiptDigest || len(subjectDocument.Bytes) == 0 {
		t.Fatalf("subject comparison lost its parent or values: %#v %v", subjectComparison, err)
	}
	actualRoots, roots, err := comparison.ChangeRoots(0, ^uint16(0))
	if err != nil {
		t.Fatal(err)
	}
	var decodedRoots pale.WorkspaceChangeRootPage
	decode(t, roots.Bytes, &decodedRoots)
	if !reflect.DeepEqual(actualRoots, decodedRoots) {
		t.Fatal("typed and canonical change-root pages differ")
	}
	if uint32(len(actualRoots.Entries)) != expected.ChangeRoots.EntryCount || !reflect.DeepEqual(actualRoots.NextOffset, expected.ChangeRoots.NextOffset) {
		t.Fatalf("portable change-root page drifted: %#v", actualRoots)
	}
	actualImpacts, impacts, err := comparison.Impacts(0, 1)
	if err != nil {
		t.Fatal(err)
	}
	var decodedImpacts pale.WorkspaceImpactPage
	decode(t, impacts.Bytes, &decodedImpacts)
	if !reflect.DeepEqual(actualImpacts, decodedImpacts) {
		t.Fatal("typed and canonical impact pages differ")
	}
	if len(actualImpacts.Entries) != 1 || actualImpacts.NextOffset == nil {
		t.Fatalf("portable impact page is incomplete: %#v", actualImpacts)
	}
	impact := actualImpacts.Entries[0]
	if *actualImpacts.NextOffset != expected.FirstImpact.NextOffset || impact.Subject.Kind != pale.WorkspaceSubjectValueSourceKind || impact.Subject.Source == nil || impact.Subject.Source.SourceID != expected.FirstImpact.SourceID || string(impact.State) != expected.FirstImpact.State || impact.ContributingRootCount != expected.FirstImpact.ContributingRootCount {
		t.Fatalf("portable first impact drifted: %#v", actualImpacts)
	}
	actualExplanation, explanation, err := comparison.Explain(pale.SourceSubject(impact.Subject.Source.SourceID))
	if err != nil {
		t.Fatal(err)
	}
	var decodedExplanation pale.WorkspaceExplanation
	decode(t, explanation.Bytes, &decodedExplanation)
	if !reflect.DeepEqual(actualExplanation, decodedExplanation) {
		t.Fatal("typed and canonical explanations differ")
	}
	if actualExplanation.State != pale.WorkspaceExplanationUnaffected || actualExplanation.Causal != nil {
		t.Fatalf("portable explanation drifted: %#v", actualExplanation)
	}

	oneShot := readOneShotFixture(t)
	effectiveWorkspace, _, err := client.CompilePortableWorkspace(oneShot.Workspace)
	if err != nil {
		t.Fatal(err)
	}
	profile, coordinates, err := effectiveWorkspace.CompileEffectiveProfile(pale.EffectiveProfileRequest{
		ProjectID: oneShot.ProjectID, ExecutionWorkspaceID: oneShot.ExecutionWorkspaceID,
		MachineID: oneShot.MachineID, PrivateInputs: oneShot.PrivateInputs,
		PackageDocuments: oneShot.PackageDocuments,
	})
	if err != nil {
		t.Fatal(err)
	}
	actualCoordinates := coordinates
	if actualCoordinates.ProjectID != oneShot.ProjectID || actualCoordinates.ExecutionWorkspaceID != oneShot.ExecutionWorkspaceID || actualCoordinates.ProfileDigest == "" || len(actualCoordinates.SelectedTopologyDigest) != 64 {
		t.Fatalf("effective-profile coordinates drifted: %#v", actualCoordinates)
	}
	profilePage, err := profile.Inspect(pale.ProfileIntendedUseCase, 0, 1)
	if err != nil {
		t.Fatal(err)
	}
	var actualProfilePage struct {
		Section string `json:"section"`
		Entries []struct {
			ExplanationHandle string `json:"explanationHandle"`
		} `json:"entries"`
	}
	decode(t, profilePage.Bytes, &actualProfilePage)
	if actualProfilePage.Section != "intended-use-case" || len(actualProfilePage.Entries) != 1 || actualProfilePage.Entries[0].ExplanationHandle == "" {
		t.Fatalf("effective-profile page drifted: %#v", actualProfilePage)
	}
	profileExplanation, err := profile.Explain(actualProfilePage.Entries[0].ExplanationHandle)
	if err != nil || profileExplanation == nil || profileExplanation.Handle != actualProfilePage.Entries[0].ExplanationHandle {
		t.Fatalf("effective-profile explanation failed: %v", err)
	}
	profileInspection, err := profile.InspectAuthoring(
		"pale-workspace.toml",
		[]pale.StructuralPathSegment{pale.AuthoringFieldPath("workspace")},
	)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(profileInspection, inspection) {
		t.Fatal("standalone and retained profile grammar facts differ")
	}
	profileNavigation, err := profile.NavigateAuthoring("pale-workspace.toml", []pale.InstancePathSegment{
		pale.AuthoringField("workspace"), pale.AuthoringField("desired_topology"),
		pale.AuthoringField("projects"), pale.AuthoringIndex(0), pale.AuthoringField("run_policy"),
	})
	if err != nil || profileNavigation.Identity == nil {
		t.Fatalf("effective-profile authoring navigation failed: %v", err)
	}
	joined, err := profile.InspectEffectiveAuthoring(profileNavigation.Identity)
	if err != nil || len(joined.Entries) == 0 {
		t.Fatalf("effective-profile authoring join failed: %v", err)
	}

	snapshot, summary, err := profile.Observe()
	if err != nil {
		t.Fatal(err)
	}
	observationSummary := summary
	if observationSummary.FactCount == 0 || observationSummary.OmittedFactCount != 0 || observationSummary.Cursor.Generation.ProfileDigest == "" || observationSummary.Cursor.Generation.InstanceDigest == "" || observationSummary.Cursor.Generation.ProfileDigest == observationSummary.Cursor.Generation.InstanceDigest {
		t.Fatalf("observation summary drifted: %#v", observationSummary)
	}
	if len(actualCoordinates.CompositionPlanDigest) != 64 || observationSummary.Cursor.Generation.CompositionPlanDigest != actualCoordinates.CompositionPlanDigest {
		t.Fatalf("observation composition proof differs from its retained profile: %#v", observationSummary)
	}
	page, err := snapshot.Page(pale.ObservationPageRequest{
		Filter:          pale.AllObservations(),
		MaximumReturned: ^uint16(0),
	})
	if err != nil || page.State != pale.ObservationPageReady || page.Page == nil || len(page.Page.Facts) == 0 || len(page.Subjects) == 0 {
		t.Fatalf("observation page drifted: %#v (%v)", page, err)
	}
	if page.Subjects[0].FactIndex >= observationSummary.FactCount || page.Subjects[0].Value != *page.Page.Facts[page.Subjects[0].FactIndex].Subject {
		t.Fatalf("observation subject custody drifted: %#v", page.Subjects[0])
	}
	causal, err := snapshot.CausalPath(&page.Subjects[0])
	if err != nil || causal.State != pale.ObservationCausalPathAbsent {
		t.Fatalf("non-path subject must have no causal path: %#v (%v)", causal, err)
	}
	delta, err := snapshot.DeltaFrom(snapshot, ^uint16(0), pale.ObservationActive)
	if err != nil {
		t.Fatal(err)
	}
	if delta.Kind != pale.ObservationCaughtUp || delta.Cursor == nil || !reflect.DeepEqual(*delta.Cursor, summary.Cursor) {
		t.Fatalf("typed caught-up coordinate drifted: %#v", delta)
	}
	lens, err := snapshot.VisualLens(pale.ObservationLensRequest{
		Filter:                   pale.AllObservations(),
		MaximumInspected:         ^uint16(0),
		MaximumNodes:             ^uint16(0),
		MaximumCopiedStringBytes: ^uint32(0),
		IncludeCausalPaths:       true,
		MaximumCausalPathLookups: ^uint16(0),
		MaximumCausalSteps:       ^uint16(0),
		Cancellation:             pale.ObservationActive,
	})
	if err != nil {
		t.Fatal(err)
	}
	if lens.Kind != pale.ObservationLensReady || lens.Lens == nil || !reflect.DeepEqual(lens.Lens.Cursor, summary.Cursor) {
		t.Fatalf("typed lens snapshot coordinate drifted: %#v", lens)
	}
	if lens.Lens.SnapshotOmittedCount != summary.OmittedFactCount {
		t.Fatalf("typed lens snapshot omission count drifted: %#v", lens)
	}
	for _, node := range lens.Lens.Nodes {
		if len(node.Facts) == 0 {
			t.Fatalf("typed lens node dropped its selected facts: %#v", lens)
		}
	}
	boundedLensRequest := pale.ObservationLensRequest{
		Filter:                   pale.AllObservations(),
		MaximumInspected:         1,
		MaximumNodes:             8,
		MaximumCopiedStringBytes: ^uint32(0),
		MaximumCausalPathLookups: 8,
		MaximumCausalSteps:       8,
		Cancellation:             pale.ObservationActive,
	}
	boundedLens, err := snapshot.VisualLens(boundedLensRequest)
	if err != nil || boundedLens.Kind != pale.ObservationLensReady || boundedLens.Lens == nil || boundedLens.Continuation == nil || boundedLens.Lens.InspectedSubjects != 1 {
		t.Fatalf("bounded lens lost its exact continuation: %#v %v", boundedLens, err)
	}
	boundedLensRequest.Cancellation = pale.ObservationCancelled
	boundedLensRequest.After = boundedLens.Continuation
	cancelledLens, err := snapshot.VisualLens(boundedLensRequest)
	if err != nil || cancelledLens.Kind != pale.ObservationLensCancelled || cancelledLens.Continuation == nil || cancelledLens.Lens != nil {
		t.Fatalf("cancelled lens lost its exact continuation: %#v %v", cancelledLens, err)
	}
	cancelled, err := snapshot.VisualLens(pale.ObservationLensRequest{
		Filter:                   pale.AllObservations(),
		MaximumInspected:         1,
		MaximumNodes:             1,
		MaximumCopiedStringBytes: ^uint32(0),
		MaximumCausalPathLookups: 1,
		MaximumCausalSteps:       1,
		Cancellation:             pale.ObservationCancelled,
	})
	if err != nil {
		t.Fatal(err)
	}
	if cancelled.Kind != pale.ObservationLensCancelled || cancelled.Lens != nil {
		t.Fatalf("cancelled lens returned a payload: %#v", cancelled)
	}
}

func TestNativeActionAndOneShotJourneys(t *testing.T) {
	client := newClient(t)
	var action struct {
		ActionID              string   `json:"action_id"`
		TaggedInput           string   `json:"tagged_input"`
		PayloadInput          string   `json:"payload_input"`
		DescriptorIDs         []string `json:"descriptor_ids"`
		DescriptorProjections [][]any  `json:"descriptor_projections"`
		ExecutionSHA256       string   `json:"execution_sha256"`
		ExecutionLength       int      `json:"execution_length"`
	}
	readFixture(t, "native-action-journey.json", &action)
	descriptors, err := client.NativeActionCatalog()
	if err != nil {
		t.Fatal(err)
	}
	actualIDs := make([]string, len(descriptors))
	for index, descriptor := range descriptors {
		actualIDs[index] = descriptor.ActionID
		projection := action.DescriptorProjections[index]
		if descriptor.ActionID != projection[0] || descriptor.Description != projection[1] || sha256Hex(descriptor.ContractBytes) != projection[2] || len(descriptor.ContractBytes) != int(projection[3].(float64)) || sha256Hex(descriptor.PayloadSchemaBytes) != projection[4] || len(descriptor.PayloadSchemaBytes) != int(projection[5].(float64)) {
			t.Fatalf("native-action descriptor %d drifted: %#v", index, descriptor)
		}
	}
	if !reflect.DeepEqual(actualIDs, action.DescriptorIDs) {
		t.Fatalf("native-action inventory drifted: want %q, got %q", action.DescriptorIDs, actualIDs)
	}
	tagged, err := client.InvokeNativeAction(action.ActionID, []byte(action.TaggedInput))
	if err != nil {
		t.Fatal(err)
	}
	payload, err := client.InvokeNativeActionPayload(action.ActionID, []byte(action.PayloadInput))
	if err != nil || !reflect.DeepEqual(tagged.Bytes, payload.Bytes) || len(payload.Bytes) != action.ExecutionLength || sha256Hex(payload.Bytes) != action.ExecutionSHA256 {
		t.Fatalf("native-action execution drifted: length=%d sha=%s (%v)", len(payload.Bytes), sha256Hex(payload.Bytes), err)
	}

	oneShot := readOneShotFixture(t)
	compilation, summary, err := client.CompileOneShot(oneShot.Request())
	if err != nil {
		t.Fatal(err)
	}
	assertOneShotSummary(t, summary.Bytes, oneShot)
	repeatedSummary, err := compilation.Summary()
	if err != nil || !reflect.DeepEqual(summary.Bytes, repeatedSummary.Bytes) {
		t.Fatalf("one-shot summary custody drifted: %v", err)
	}
	// Returned bytes belong to Go, not the retained native compilation or a
	// previous response. Mutating one copy cannot change a later projection.
	summary.Bytes[0] ^= 0xff
	freshSummary, err := compilation.Summary()
	if err != nil || !reflect.DeepEqual(freshSummary.Bytes, repeatedSummary.Bytes) {
		t.Fatalf("returned summary aliases native or other response bytes: %v", err)
	}
	targetsDocument, err := compilation.ObservationTargets(oneShot.RepositoryID, pale.PlanApply, pale.PriorMaterialization{})
	if err != nil {
		t.Fatal(err)
	}
	var targetProjection struct {
		Targets []string `json:"targets"`
	}
	decode(t, targetsDocument.Bytes, &targetProjection)
	targets := targetProjection.Targets
	if !reflect.DeepEqual(targets, oneShot.ObservationTargets) {
		t.Fatalf("observation targets drifted: want %q, got %q", oneShot.ObservationTargets, targets)
	}
	baseline := make([]pale.BaselineEntry, len(targets))
	for index, target := range targets {
		baseline[index] = pale.BaselineEntry{RelativePath: target}
	}
	candidate, info, plan, err := compilation.Plan(oneShot.RepositoryID, pale.PlanApply, pale.PriorMaterialization{}, baseline)
	if err != nil {
		t.Fatal(err)
	}
	if info.Kind != pale.PlanApply || info.RepositoryID != oneShot.Plan.RepositoryID || info.PlanDigest != oneShot.Plan.PlanDigest || info.OneShotCompilationDigest != oneShot.Plan.OneShotCompilationDigest || info.OperationCount != oneShot.Plan.OperationCount || info.DeferredEffectCount != oneShot.Plan.DeferredEffectCount || len(plan.Bytes) != oneShot.Plan.CanonicalLength || sha256Hex(plan.Bytes) != oneShot.Plan.CanonicalSHA256 {
		t.Fatalf("one-shot plan drifted: %#v", info)
	}
	bound, boundInfo, saved, err := candidate.BindMachineEffects(nil)
	if err != nil || boundInfo.Kind != pale.PlanApply || boundInfo.RepositoryID != oneShot.RepositoryID || len(saved.Bytes) == 0 {
		t.Fatalf("machine-effect binding drifted: %#v (%v)", boundInfo, err)
	}
	if _, _, _, err := candidate.BindMachineEffects(nil); err == nil {
		t.Fatal("consumed one-shot candidate was accepted twice")
	}
	plan.Bytes[0] ^= 0xff
	var boundValue struct {
		PlanDigest  string `json:"plan_digest"`
		SurfacePlan struct {
			PlanDigest string `json:"plan_digest"`
		} `json:"surface_plan"`
		MachineLocalEffects []json.RawMessage `json:"machine_local_effects"`
	}
	decode(t, saved.Bytes, &boundValue)
	if boundValue.SurfacePlan.PlanDigest != info.PlanDigest || len(boundValue.MachineLocalEffects) != 0 {
		t.Fatalf("bound plan drifted: %#v", boundValue)
	}
	_, admittedInfo, err := bound.AdmitSaved(saved.Bytes)
	if err != nil || admittedInfo.Kind != pale.PlanApply || admittedInfo.RepositoryID != oneShot.RepositoryID || admittedInfo.PlanDigest != boundValue.PlanDigest {
		t.Fatalf("saved-plan admission drifted: %#v (%v)", admittedInfo, err)
	}
	if _, _, err := bound.AdmitSaved(saved.Bytes); err == nil {
		t.Fatal("consumed bound plan was accepted twice")
	}

	var repository struct {
		ProfileDigest  string `json:"profile_digest"`
		InstanceDigest string `json:"instance_digest"`
		RepositoryID   string `json:"repository_id"`
	}
	readFixture(t, "repository-admission.json", &repository)
	admitted, err := client.AdmitRepositoryNativeActions(
		read(t, filepath.Join(fixtureRoot, "repository-catalog.json")),
		repository.ProfileDigest, repository.InstanceDigest, repository.RepositoryID,
	)
	if err != nil || admitted.ProfileDigest != repository.ProfileDigest || admitted.InstanceDigest != repository.InstanceDigest || admitted.RepositoryID != repository.RepositoryID || len(admitted.Descriptors) != len(action.DescriptorIDs) {
		t.Fatalf("repository action admission drifted: %#v (%v)", admitted, err)
	}
	packages, err := client.InspectPackageDistribution(oneShot.PackageDocuments)
	if err != nil {
		t.Fatal(err)
	}
	var packageInspection struct {
		Packages []struct {
			ID      string `json:"id"`
			Version string `json:"version"`
		} `json:"packages"`
	}
	decode(t, packages.Bytes, &packageInspection)
	if len(packageInspection.Packages) != 1 || packageInspection.Packages[0].ID != "example.engine" || packageInspection.Packages[0].Version != "1.0.0" {
		t.Fatalf("Package distribution inspection drifted: %#v", packageInspection)
	}
	admission, err := client.AdmitResidentHook(pale.ResidentHookRequest{
		Event: "session-start", Workspace: "workspace-a", Repository: "repository-a",
		ProviderBinding: "provider-binding-claude-code", ProviderBindingIdentity: "ec03a4e4096f39aa0a8c2e3fe9b69705b0ff620963137a0558224b9826a4f3b6",
		Payload: []byte(`{"hook_event_name":"SessionStart","session_id":"native-session"}`),
	})
	if err != nil || admission == nil {
		t.Fatalf("resident hook admission drifted: %v", err)
	}
}

func TestNativeLocalLifecycleJourneys(t *testing.T) {
	client := newClient(t)
	compiledIdentity, err := client.CompiledInterfaceIdentity()
	if err != nil {
		t.Fatal(err)
	}
	root, manifest := layOutLocalWorkspace(t, compiledIdentity)
	request := pale.LocalWorkspaceRequest{
		Manifest: manifest, ProjectID: "demo", ExecutionWorkspaceID: "default", MachineID: "machine",
	}
	compilation, summary, err := client.CompileLocalOneShot(request)
	if err != nil || len(summary.Bytes) == 0 {
		t.Fatalf("local one-shot compilation failed: %v", err)
	}
	packages, err := client.InspectLocalPackageDistribution(request)
	if err != nil || len(packages.Bytes) == 0 {
		t.Fatalf("local Package inspection failed: %v", err)
	}
	status, err := compilation.RepositoryStatus("engine")
	if err != nil || status.Clean || len(status.Drift) == 0 {
		t.Fatalf("local repository status drifted: %#v (%v)", status, err)
	}
	localPlan, planInfo, plan, err := compilation.Plan("engine", pale.PlanApply)
	if err != nil || planInfo.CreateCount == 0 || len(plan.Bytes) == 0 {
		t.Fatalf("local effect-free planning drifted: %#v (%v)", planInfo, err)
	}
	plans := filepath.Join(root, "plans")
	publication, err := localPlan.Publish(plans)
	if err != nil || publication.RepositoryID != "engine" {
		t.Fatalf("local plan publication drifted: %#v (%v)", publication, err)
	}
	published := read(t, filepath.Join(plans, publication.RelativeTarget))
	if len(published) == 0 || published[len(published)-1] != '\n' || !containsIndentedJSON(published) {
		t.Fatal("published plan is not pretty-printed JSON")
	}
	evidence := filepath.Join(root, "evidence")
	run, admission, err := compilation.AdmitRun(pale.PlanApply, plans, evidence)
	if err != nil || len(admission.Targets) != 1 || admission.Targets[0].RepositoryID != "engine" {
		t.Fatalf("local run admission drifted: %#v (%v)", admission, err)
	}
	_, err = run.Reconciliation()
	var reconciliationError *pale.DiagnosticError
	if !errors.As(err, &reconciliationError) || reconciliationError.Code != "local_materialization_phase_rejected" || reconciliationError.Effect != pale.DiagnosticEffectNotApplicable {
		t.Fatalf("pre-settlement reconciliation did not close safely: %T %v", err, err)
	}
	verifiedIdentity, err := compilation.VerifyCurrentExecutableBinding()
	if err != nil || !reflect.DeepEqual(verifiedIdentity.Canonical, compiledIdentity.Canonical) {
		t.Fatalf("local runtime verification drifted: %#v (%v)", verifiedIdentity, err)
	}
	settled, err := run.Settle()
	if err != nil || settled.Kind != pale.PlanApply || settled.Outcome != pale.LocalRunSucceeded || settled.TargetCount != 1 || settled.SucceededTargetCount != 1 || len(settled.Targets) != 1 {
		t.Fatalf("local run settlement drifted: %#v (%v)", settled, err)
	}
	target := settled.Targets[0]
	if target.RepositoryID != "engine" || target.Outcome != pale.LocalTargetSucceeded || target.ChangedSurfaceCount == 0 || target.Failure != nil {
		t.Fatalf("local target settlement drifted: %#v", target)
	}
	receipt := read(t, filepath.Join(evidence, "run-result.apply.json"))
	if len(receipt) == 0 || receipt[len(receipt)-1] != '\n' || !containsIndentedJSON(receipt) {
		t.Fatal("run receipt is not pretty-printed JSON")
	}
	var compactReceipt bytes.Buffer
	if err := json.Compact(&compactReceipt, receipt); err != nil || !bytes.Equal(compactReceipt.Bytes(), settled.Canonical) {
		t.Fatalf("returned run bytes differ from the recorded receipt: %v", err)
	}
	status, err = compilation.RepositoryStatus("engine")
	if err != nil || !status.Clean {
		t.Fatalf("settled repository did not converge: %#v (%v)", status, err)
	}
	actionInput := []byte(`{"kind":"path_inventory","payload":{"style":"posix_relative","paths":["src/./lib.rs"]}}`)
	actionPath := filepath.Join(root, "action.json")
	if err := os.WriteFile(actionPath, actionInput, 0o600); err != nil {
		t.Fatal(err)
	}
	captured, err := client.CaptureLocalNativeActionRequest("pale.compute.path-inventory", root, "action.json")
	if err != nil {
		t.Fatal(err)
	}
	execution, err := captured.Invoke()
	if err != nil || len(execution.Bytes) == 0 {
		t.Fatalf("confined native-action input capture drifted: %v", err)
	}
}

type oneShotFixture struct {
	ProjectID            string
	ExecutionWorkspaceID string
	MachineID            string
	RepositoryID         string
	PrivateInputs        pale.PrivateInputs
	PackageDocuments     []pale.Document
	Workspace            pale.PortableWorkspaceRequest
	ObservationTargets   []string
	Summary              struct {
		ProjectID                string   `json:"project_id"`
		ExecutionWorkspaceID     string   `json:"execution_workspace_id"`
		ProfileDigest            string   `json:"profile_digest"`
		InstanceDigest           string   `json:"instance_digest"`
		OneShotCompilationDigest string   `json:"one_shot_compilation_digest"`
		SchemaManifestSHA256     string   `json:"schema_manifest_sha256"`
		SchemaSetSHA256          string   `json:"schema_set_sha256"`
		RepositoryIDs            []string `json:"repository_ids"`
		CanonicalSHA256          string   `json:"canonical_sha256"`
		CanonicalLength          int      `json:"canonical_length"`
	}
	Plan struct {
		RepositoryID             string `json:"repository_id"`
		PlanDigest               string `json:"plan_digest"`
		OneShotCompilationDigest string `json:"one_shot_compilation_digest"`
		OperationCount           uint32 `json:"operation_count"`
		DeferredEffectCount      uint32 `json:"deferred_effect_count"`
		CanonicalSHA256          string `json:"canonical_sha256"`
		CanonicalLength          int    `json:"canonical_length"`
	}
}

func (fixture oneShotFixture) Request() pale.OneShotRequest {
	return pale.OneShotRequest{
		ProjectID: fixture.ProjectID, ExecutionWorkspaceID: fixture.ExecutionWorkspaceID,
		MachineID: fixture.MachineID, PrivateInputs: fixture.PrivateInputs,
		PackageDocuments: fixture.PackageDocuments, Workspace: fixture.Workspace,
	}
}

func readOneShotFixture(t *testing.T) oneShotFixture {
	t.Helper()
	var source struct {
		Selectors struct {
			ProjectID            string `json:"project_id"`
			ExecutionWorkspaceID string `json:"execution_workspace_id"`
			MachineID            string `json:"machine_id"`
			RepositoryID         string `json:"repository_id"`
		} `json:"selectors"`
		ObservationTargets []string        `json:"observation_targets"`
		Summary            json.RawMessage `json:"summary"`
		Plan               json.RawMessage `json:"plan"`
	}
	readFixture(t, "one-shot-journey.json", &source)
	root := filepath.Join(fixtureRoot, "one-shot")
	document := func(label, path string) pale.Document {
		return pale.Document{Label: label, Bytes: read(t, filepath.Join(root, path))}
	}
	fixture := oneShotFixture{
		ProjectID: source.Selectors.ProjectID, ExecutionWorkspaceID: source.Selectors.ExecutionWorkspaceID,
		MachineID: source.Selectors.MachineID, RepositoryID: source.Selectors.RepositoryID,
		PrivateInputs: pale.PrivateInputs{
			Capabilities:     document("capabilities.toml", "private/capabilities.toml"),
			MachineBinding:   document("machines/machine.toml", "private/machine-binding.toml"),
			DistributionLock: document("distribution-lock.toml", "private/distribution-lock.toml"),
		},
		PackageDocuments: []pale.Document{
			document("engine/package.toml", "packages/engine/package.toml"),
			document("engine/role.md", "packages/engine/role.md"),
			document("engine/roles.toml", "packages/engine/roles.toml"),
		},
		Workspace: pale.PortableWorkspaceRequest{
			RootManifest: "pale-workspace.toml",
			Documents: []pale.Document{
				document("composition.toml", "workspace/composition.toml"),
				document("pale-workspace.toml", "workspace/pale-workspace.toml"),
				document("workspace-resolution.lock.toml", "workspace/workspace-resolution.lock.toml"),
				document("workspace-universe.lock.toml", "workspace/workspace-universe.lock.toml"),
			},
		},
		ObservationTargets: source.ObservationTargets,
	}
	decode(t, source.Summary, &fixture.Summary)
	decode(t, source.Plan, &fixture.Plan)
	return fixture
}

func assertOneShotSummary(t *testing.T, bytes []byte, fixture oneShotFixture) {
	t.Helper()
	actual := fixture.Summary
	decode(t, bytes, &actual)
	if actual.ProjectID != fixture.Summary.ProjectID || actual.ExecutionWorkspaceID != fixture.Summary.ExecutionWorkspaceID || actual.ProfileDigest != fixture.Summary.ProfileDigest || actual.InstanceDigest != fixture.Summary.InstanceDigest || actual.OneShotCompilationDigest != fixture.Summary.OneShotCompilationDigest || actual.SchemaManifestSHA256 != fixture.Summary.SchemaManifestSHA256 || actual.SchemaSetSHA256 != fixture.Summary.SchemaSetSHA256 || !reflect.DeepEqual(actual.RepositoryIDs, fixture.Summary.RepositoryIDs) || len(bytes) != fixture.Summary.CanonicalLength || sha256Hex(bytes) != fixture.Summary.CanonicalSHA256 {
		t.Fatalf("one-shot summary drifted: %#v", actual)
	}
}

func layOutLocalWorkspace(t *testing.T, compiledIdentity pale.CompiledInterfaceIdentity) (string, string) {
	t.Helper()
	root, err := filepath.EvalSymlinks(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	for _, directory := range []string{"checkout", "authority", "lineage"} {
		if err := os.MkdirAll(filepath.Join(root, directory), 0o700); err != nil {
			t.Fatal(err)
		}
	}
	copyFixture := func(target, source string) {
		t.Helper()
		path := filepath.Join(root, target)
		if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, read(t, filepath.Join(fixtureRoot, "one-shot", source)), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	for _, name := range []string{"composition.toml", "pale-workspace.toml", "workspace-resolution.lock.toml", "workspace-universe.lock.toml"} {
		copyFixture(name, filepath.Join("workspace", name))
	}
	copyFixture("capabilities.toml", "private/capabilities.toml")
	copyFixture("distribution-lock.toml", "private/distribution-lock.toml")
	for _, name := range []string{"package.toml", "role.md", "roles.toml"} {
		copyFixture(filepath.Join("distribution", "packages", "engine", name), filepath.Join("packages", "engine", name))
	}
	executable, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	executable, err = filepath.EvalSymlinks(executable)
	if err != nil {
		t.Fatal(err)
	}
	machine := fmt.Sprintf("[machine]\nid = \"machine\"\nschema_version = 1\ndistribution_root = %q\n\n[[repository_checkouts]]\nrepository_id = \"engine\"\ncheckout_root = %q\n\n[authority_roots]\nauthority = %q\n\n[materialization_lineage]\nroot = %q\nmax_route_records_per_collection = 4096\n\n[surface_executable]\nexec = %q\ndigest = %q\nversion = %q\nprotocol = %q\n",
		filepath.Join(root, "distribution"), filepath.Join(root, "checkout"), filepath.Join(root, "authority"), filepath.Join(root, "lineage"), executable, sha256Hex(read(t, executable)), compiledIdentity.Version, compiledIdentity.Protocol)
	machinePath := filepath.Join(root, "machines", "machine.toml")
	if err := os.MkdirAll(filepath.Dir(machinePath), 0o700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(machinePath, []byte(machine), 0o600); err != nil {
		t.Fatal(err)
	}
	return root, filepath.Join(root, "pale-workspace.toml")
}

func containsIndentedJSON(document []byte) bool {
	return bytes.Contains(document, []byte("\n  "))
}

func newClient(t *testing.T) *pale.Client {
	t.Helper()
	client, err := pale.NewClient()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(client.Close)
	return client
}

func assertJSONField(t *testing.T, bytes []byte, field, expected string) {
	t.Helper()
	var value map[string]json.RawMessage
	decode(t, bytes, &value)
	var actual string
	decode(t, value[field], &actual)
	if actual != expected {
		t.Fatalf("%s: want %q, got %q", field, expected, actual)
	}
}

func sha256Hex(bytes []byte) string {
	return fmt.Sprintf("%x", sha256.Sum256(bytes))
}

func readFixture(t *testing.T, name string, value any) {
	t.Helper()
	decode(t, read(t, filepath.Join(fixtureRoot, name)), value)
}

func fixtureDocument(t *testing.T, name string) pale.Document {
	t.Helper()
	return pale.Document{Label: name, Bytes: read(t, filepath.Join(fixtureRoot, "portable", name))}
}

func read(t *testing.T, path string) []byte {
	t.Helper()
	bytes, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return bytes
}

func decode(t *testing.T, bytes []byte, value any) {
	t.Helper()
	if err := json.Unmarshal(bytes, value); err != nil {
		t.Fatal(err)
	}
}

var expectedCapabilities = []pale.CapabilityID{
	"contract.bundle-identity", "contract.schema-lookup", "contract.document-validation",
	"diagnostic.projection", "diagnostic.admission", "package-distribution.inspection",
	"authoring.ingress-limits", "authoring.standalone-validation", "authoring.standalone-inspection",
	"workspace.portable-compilation", "workspace.portable-authoring-inspection", "workspace.portable-authoring-navigation",
	"workspace.portable-authoring-occurrences",
	"workspace.portable-comparison", "workspace.portable-subject-comparison", "workspace.portable-change-roots", "workspace.portable-impacts", "workspace.portable-explanation",
	"workspace.effective-profile-compilation", "workspace.effective-profile-inspection", "workspace.effective-profile-explanation",
	"workspace.effective-profile-authoring-inspection", "workspace.effective-profile-authoring-navigation", "workspace.effective-profile-authoring-occurrences", "workspace.effective-profile-authoring-join",
	"workspace.effective-profile-authoring-pages",
	"workspace.configuration-improvement",
	"observation.snapshot", "observation.page", "observation.profile-inspection", "observation.delta", "observation.causal-path", "observation.visual-lens",
	"native-action.catalog", "native-action.invocation", "native-action.payload-invocation", "native-action.repository-admission",
	"resident-hook.admission", "one-shot.compilation", "one-shot.compilation-summary", "one-shot.observation-targets",
	"one-shot.planning", "one-shot.machine-effect-binding", "one-shot.saved-plan-admission",
	"local.one-shot-compilation", "local.package-distribution-inspection", "local.native-action-input-capture",
	"local.repository-status", "local.one-shot-planning", "local.one-shot-plan-publication",
	"local.materialization-run-admission", "local.materialization-run-settlement",
	"local.materialization-reconciliation-inspection", "local.compiled-interface-identity", "local.current-executable-binding-verification",
	"local.control-storage-inspection",
	"local.configuration-improvement-preview",
}
