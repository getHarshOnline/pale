package tests_test

import (
	"encoding/json"
	"fmt"
	"reflect"
	"slices"
	"strings"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

// These are private SDK transport-shape cases, not admitted resident/native evidence.
func TestObservationClosedProjectionDecoders(t *testing.T) {
	profile := `{"kind":"effective-profile","profileDigest":"profile","instanceDigest":"instance","compositionPlanDigest":"composition"}`
	resident := `{"kind":"resident","controlStoreId":"store","controlIncarnationId":"incarnation","activeGenerationDigest":null}`
	cursor := `{"generation":` + profile + `,"revision":{"kind":"static"}}`
	resume := `{"cursor":` + cursor + `}`
	subject := `{"kind":"machine","machineId":"machine"}`
	fact := `{"category":"health","subject":` + subject + `,"state":"unknown","freshness":{"kind":"unknown"}}`
	resync := `{"reason":{"kind":"generation-mismatch","requested":` + profile + `,"current":` + resident + `},"current":` + cursor + `}`
	path := `{"generation":` + profile + `,"subject":` + subject + `,"steps":[{"kind":"entry-contract","contractId":"entry"}]}`
	completion := `{"kind":"complete"}`
	pagePosition := `{"snapshot":` + cursor + `,"inspectedPosition":4294967295,"maximumInspected":65535,"maximumReturned":65535,"maximumCopiedStringBytes":4294967295}`
	lensPosition := `{"snapshot":` + cursor + `,"inspectedPosition":4294967295,"maximumInspected":65535,"maximumNodes":65535,"maximumCopiedStringBytes":4294967295,"includeCausalPaths":true,"maximumCausalPathLookups":65535,"maximumCausalSteps":65535}`
	lens := `{"cursor":` + cursor + `,"nodes":[{"subject":` + subject + `,"facts":[` + fact + `]}],"causalPaths":[` + path + `],"continuation":null,"completion":` + completion + `,"inspectedSubjects":1,"copiedStringBytes":7,"causalPathLookups":1,"causalSteps":1,"snapshotOmittedCount":0}`
	delta := `{"from":` + cursor + `,"to":` + cursor + `,"changes":[{"kind":"upsert","fact":` + fact + `},{"kind":"remove","category":"health","subject":` + subject + `}],"successorOmittedFactCount":0}`
	cases := []struct {
		name, data string
		sample     any
	}{
		{"profile-generation", profile, pale.ObservationGeneration{}},
		{"resident-generation-absent", resident, pale.ObservationGeneration{}},
		{"resident-generation-active", strings.Replace(resident, "null", `"generation"`, 1), pale.ObservationGeneration{}},
		{"static", `{"kind":"static"}`, pale.ObservationRevision{}},
		{"journal-empty", `{"kind":"resident-journal","sequence":null}`, pale.ObservationRevision{}},
		{"journal-u64", `{"kind":"resident-journal","sequence":18446744073709551615}`, pale.ObservationRevision{}},
		{"cursor", cursor, pale.ObservationCursor{}}, {"resume", resume, pale.ObservationResumeCoordinate{}},
		{"fresh", `{"kind":"fresh"}`, pale.ObservationFreshness{}},
		{"unknown", `{"kind":"unknown"}`, pale.ObservationFreshness{}},
		{"stale-u64", `{"kind":"stale","revisionsBehind":18446744073709551615}`, pale.ObservationFreshness{}},
		{"summary", `{"cursor":` + cursor + `,"factCount":4294967295,"omittedFactCount":0}`, pale.ObservationSnapshotSummary{}},
		{"page-position", pagePosition, pale.ObservationPagePosition{}},
		{"step-limit-inspected", `{"kind":"inspected"}`, pale.ObservationStepLimit{}},
		{"step-limit-copied", `{"kind":"copied-string-bytes","minimumRequired":4294967295}`, pale.ObservationStepLimit{}},
		{"step-limit-causal", `{"kind":"causal-steps","minimumRequired":65535}`, pale.ObservationStepLimit{}},
		{"step-complete", completion, pale.ObservationStepCompletion{}},
		{"step-stopped", `{"kind":"stopped","limit":{"kind":"returned"}}`, pale.ObservationStepCompletion{}},
		{"page", `{"cursor":` + cursor + `,"facts":[` + fact + `],"continuation":null,"completion":` + completion + `,"inspected":1,"copiedStringBytes":7,"snapshotOmittedCount":0}`, pale.ObservationPage{}},
		{"generation-resync", resync, pale.ObservationResynchronization{}},
		{"revision-gap", `{"kind":"revision-gap","requested":{"kind":"resident-journal","sequence":null},"current":{"kind":"resident-journal","sequence":9}}`, pale.ObservationResynchronizationReason{}},
		{"continuation-mismatch", `{"kind":"continuation-mismatch"}`, pale.ObservationResynchronizationReason{}},
		{"delta", delta, pale.ObservationDelta{}},
		{"delta-result", `{"kind":"delta","delta":` + delta + `}`, pale.ObservationDeltaResult{}},
		{"caught-up", `{"kind":"caught-up","cursor":` + cursor + `}`, pale.ObservationDeltaResult{}},
		{"delta-resync", `{"kind":"resynchronize","resynchronization":` + resync + `}`, pale.ObservationDeltaResult{}},
		{"delta-cancelled", `{"kind":"cancelled","resume":` + resume + `}`, pale.ObservationDeltaResult{}},
		{"backpressure", `{"kind":"backpressured","pendingChanges":4294967295,"maximumChanges":65535,"recovery":"request-snapshot","current":` + cursor + `}`, pale.ObservationDeltaResult{}},
		{"path", path, pale.ObservationCausalPath{}},
		{"path-ready", `{"state":"ready","path":` + path + `}`, pale.ObservationCausalPathResult{}},
		{"path-absent", `{"state":"absent"}`, pale.ObservationCausalPathResult{}},
		{"path-resync", `{"state":"resynchronize","resynchronization":` + resync + `}`, pale.ObservationCausalPathResult{}},
		{"lens", lens, pale.ObservationLens{}},
		{"lens-position", lensPosition, pale.ObservationLensPosition{}},
		{"lens-node", `{"subject":` + subject + `,"facts":[{"category":"health","subject":` + subject + `,"state":"healthy","freshness":{"kind":"fresh"}}]}`, pale.ObservationLensNode{}},
		{"lens-ready", `{"kind":"ready","lens":` + lens + `}`, pale.ObservationLensResult{}},
		{"lens-cancelled", `{"kind":"cancelled"}`, pale.ObservationLensResult{}},
	}
	for _, shape := range []struct{ kind, payload string }{
		{"effective-profile", `,"profileDigest":"profile"`}, {"machine", `,"machineId":"machine"`},
		{"repository", `,"repositoryId":"repo"`}, {"operation", `,"repositoryId":"repo","operationId":"operation"`},
		{"capability-requirement", `,"repositoryId":"repo","operationId":"operation","requirementId":"requirement","capabilityId":"capability"`},
		{"execution-path", `,"executionPathId":"path"`}, {"compiled-generation", `,"compiledGenerationDigest":"generation"`}, {"resident-process", ""},
	} {
		cases = append(cases, struct {
			name, data string
			sample     any
		}{"subject-" + shape.kind, `{"kind":"` + shape.kind + `"` + shape.payload + `}`, pale.ObservationSubjectValue{}})
	}
	for _, shape := range []struct{ kind, payload string }{
		{"satisfied", ""}, {"provider-binding-absent", ""},
		{"narrowed", `,"reason":"capability-not-permitted"`}, {"blocked", `,"reason":"provider-effect-mismatch"`},
	} {
		cases = append(cases, struct {
			name, data string
			sample     any
		}{"requirement-" + shape.kind, `{"kind":"` + shape.kind + `"` + shape.payload + `}`, pale.EffectiveProfileRequirementDisposition{}})
	}
	for _, shape := range []struct{ kind, payload string }{
		{"unknown", ""}, {"unavailable", ""}, {"observed", `,"ownerSystemId":"owner","observationKind":"kind"`},
	} {
		cases = append(cases, struct {
			name, data string
			sample     any
		}{"native-" + shape.kind, `{"kind":"` + shape.kind + `"` + shape.payload + `}`, pale.ObservationNativeState{}})
	}
	for _, shape := range []struct{ kind, payload string }{
		{"not-required", ""}, {"unknown", ""}, {"unavailable", ""},
		{"required", `,"caseDigest":"case"`}, {"in-progress", `,"caseDigest":"case"`}, {"settled", `,"resultDigest":"result"`},
	} {
		cases = append(cases, struct {
			name, data string
			sample     any
		}{"reconciliation-" + shape.kind, `{"kind":"` + shape.kind + `"` + shape.payload + `}`, pale.ObservationReconciliationState{}})
	}
	for _, shape := range []struct{ kind, field string }{
		{"origin-receipt", "contractId"}, {"entry-contract", "contractId"}, {"responsibility-contract", "contractId"},
		{"provider-binding", "bindingId"}, {"delegation-edge", "edgeId"}, {"termination-contract", "contractId"},
		{"settlement-contract", "contractId"}, {"continuity-contract", "contractId"},
	} {
		cases = append(cases, struct {
			name, data string
			sample     any
		}{"step-" + shape.kind, fmt.Sprintf(`{"kind":%q,%q:"reference"}`, shape.kind, shape.field), pale.ObservationCausalStep{}})
	}
	for _, shape := range []struct{ category, payload string }{
		{"operation-exposure", `"state":"capability-bearing"`},
		{"capability-requirement-disposition", `"state":{"kind":"narrowed","reason":"effect-not-permitted"}`},
		{"health", `"state":"degraded"`}, {"readiness", `"state":"unproven"`}, {"effect", `"certainty":"outcome_unknown"`},
		{"native-owner", `"state":{"kind":"observed","ownerSystemId":"owner","observationKind":"kind"}`},
		{"reconciliation", `"state":{"kind":"settled","resultDigest":"result"}`},
		{"process-intent", `"state":"stopped"`},
	} {
		subjectField := `,"subject":` + subject
		if shape.category == "process-intent" {
			subjectField = ""
		}
		cases = append(cases, struct {
			name, data string
			sample     any
		}{"fact-" + shape.category, `{"category":"` + shape.category + `"` + subjectField + `,` + shape.payload + `,"freshness":{"kind":"fresh"}}`, pale.ObservationFact{}})
	}
	for _, test := range cases {
		t.Run(test.name, func(t *testing.T) {
			target, expected := reflect.New(reflect.TypeOf(test.sample)).Interface(), reflect.New(reflect.TypeOf(test.sample)).Interface()
			if err := json.Unmarshal([]byte(test.data), target); err != nil {
				t.Fatalf("valid SDK projection: %v", err)
			}
			if err := json.Unmarshal([]byte(test.data), expected); err != nil {
				t.Fatal(err)
			}
			var fields map[string]json.RawMessage
			if err := json.Unmarshal([]byte(test.data), &fields); err != nil {
				t.Fatal(err)
			}
			bad := []string{"null", "[]", test.data + `{}`, strings.TrimSuffix(test.data, "}") + `,"unselected":true}`}
			names := make([]string, 0, len(fields))
			for name := range fields {
				names = append(names, name)
			}
			slices.Sort(names)
			for _, name := range names {
				original := fields[name]
				alias := strings.ToUpper(name[:1]) + name[1:]
				fields[alias] = original
				aliased, err := json.Marshal(fields)
				if err != nil {
					t.Fatal(err)
				}
				bad = append(bad, string(aliased))
				delete(fields, name)
				aliasOnly, err := json.Marshal(fields)
				if err != nil {
					t.Fatal(err)
				}
				bad = append(bad, string(aliasOnly))
				delete(fields, alias)
				missing, err := json.Marshal(fields)
				if err != nil {
					t.Fatal(err)
				}
				bad = append(bad, string(missing))
				fields[name] = original
				if name == "kind" || name == "state" || name == "category" {
					bad = append(bad, `{"`+name+`":"unknown-extra",`+strings.TrimPrefix(test.data, "{"))
				}
			}
			if test.name == "page" {
				bad = append(bad, strings.Replace(test.data, `,"revision":{"kind":"static"}`, `,"revision":{"kind":"static"},"Revision":{"kind":"static"}`, 1))
			}
			for _, malformed := range bad {
				if err := json.Unmarshal([]byte(malformed), target); err == nil {
					t.Fatalf("accepted incomplete/unknown shape: %s", malformed)
				}
				if !reflect.DeepEqual(target, expected) {
					t.Fatalf("failed decoder changed prior value: %s", malformed)
				}
			}
		})
	}
}

func TestObservationIntegerAndVariantPayloadBoundaries(t *testing.T) {
	for _, test := range []struct {
		data   string
		target any
	}{
		{`{"kind":"resident-journal","sequence":18446744073709551616}`, &pale.ObservationRevision{}},
		{`{"kind":"resident-journal","sequence":-1}`, &pale.ObservationRevision{}},
		{`{"kind":"stale","revisionsBehind":1.5}`, &pale.ObservationFreshness{}},
		{`{"kind":"stale","revisionsBehind":null}`, &pale.ObservationFreshness{}},
		{`{"kind":"narrowed","reason":"capability-forbidden"}`, &pale.EffectiveProfileRequirementDisposition{}},
		{`{"kind":"blocked","reason":"capability-not-permitted"}`, &pale.EffectiveProfileRequirementDisposition{}},
		{`{"kind":"fresh","revisionsBehind":0}`, &pale.ObservationFreshness{}},
		{`{"kind":"observed","ownerSystemId":null,"observationKind":"kind"}`, &pale.ObservationNativeState{}},
		{`{"state":"absent","path":null}`, &pale.ObservationCausalPathResult{}},
		{`{"kind":"cancelled","lens":null}`, &pale.ObservationLensResult{}},
	} {
		if err := json.Unmarshal([]byte(test.data), test.target); err == nil {
			t.Fatalf("accepted invalid typed payload: %s", test.data)
		}
	}
	var revision pale.ObservationRevision
	if err := json.Unmarshal([]byte(`{"kind":"resident-journal","sequence":18446744073709551615}`), &revision); err != nil || revision.Sequence == nil || *revision.Sequence != ^uint64(0) {
		t.Fatalf("uint64 lost precision: %#v %v", revision, err)
	}
	if err := json.Unmarshal([]byte(`{"kind":"resident-journal","sequence":null}`), &revision); err != nil || revision.Sequence != nil {
		t.Fatalf("absent sequence changed meaning: %#v %v", revision, err)
	}
}
