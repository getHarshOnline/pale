package tests_test

import (
	"encoding/json"
	"reflect"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func TestWorkspaceImpactPagePreservesClosedSubjectsAndStates(t *testing.T) {
	var page pale.WorkspaceImpactPage
	if err := json.Unmarshal([]byte(`{
		"entries":[
			{"subject":{"kind":"source","sourceId":"workspace-manifest"},"state":"affected","contributingRootCount":2},
			{"subject":{"kind":"universe_lock"},"state":"unaffected","contributingRootCount":0}
		],
		"nextOffset":2
	}`), &page); err != nil {
		t.Fatal(err)
	}
	if len(page.Entries) != 2 || page.NextOffset == nil || *page.NextOffset != 2 {
		t.Fatalf("impact page lost its bound or continuation: %#v", page)
	}
	affected := page.Entries[0]
	if affected.Subject.Kind != pale.WorkspaceSubjectValueSourceKind || affected.Subject.Source == nil || affected.Subject.Source.SourceID != "workspace-manifest" || affected.State != pale.WorkspaceImpactAffected || affected.ContributingRootCount != 2 {
		t.Fatalf("affected source was flattened: %#v", affected)
	}
	unaffected := page.Entries[1]
	if unaffected.Subject.Kind != pale.WorkspaceSubjectValueUniverseLockKind || unaffected.State != pale.WorkspaceImpactUnaffected || unaffected.ContributingRootCount != 0 {
		t.Fatalf("unaffected lock was flattened: %#v", unaffected)
	}
}

func TestWorkspaceImpactPageRejectsContradictoryOrOpenTransport(t *testing.T) {
	retained := pale.WorkspaceImpactPage{
		Entries: []pale.WorkspaceImpact{{
			Subject: pale.WorkspaceSubjectValue{
				Kind:   pale.WorkspaceSubjectValueSourceKind,
				Source: &pale.WorkspaceSubjectValueSource{SourceID: "retained"},
			},
			State:                 pale.WorkspaceImpactAffected,
			ContributingRootCount: 1,
		}},
	}
	for name, encoded := range map[string]string{
		"affected-without-root": `{"entries":[{"subject":{"kind":"source","sourceId":"source"},"state":"affected","contributingRootCount":0}],"nextOffset":null}`,
		"unaffected-with-root":  `{"entries":[{"subject":{"kind":"source","sourceId":"source"},"state":"unaffected","contributingRootCount":1}],"nextOffset":null}`,
		"unknown-state":         `{"entries":[{"subject":{"kind":"source","sourceId":"source"},"state":"pending","contributingRootCount":1}],"nextOffset":null}`,
		"unknown-subject":       `{"entries":[{"subject":{"kind":"future"},"state":"affected","contributingRootCount":1}],"nextOffset":null}`,
		"missing-continuation":  `{"entries":[]}`,
		"null-entries":          `{"entries":null,"nextOffset":null}`,
		"extra-field":           `{"entries":[],"nextOffset":null,"state":"affected"}`,
	} {
		t.Run(name, func(t *testing.T) {
			actual := retained
			if err := json.Unmarshal([]byte(encoded), &actual); err == nil {
				t.Fatal("invalid impact projection was accepted")
			}
			if !reflect.DeepEqual(actual, retained) {
				t.Fatal("failed decoding replaced the previous impact page")
			}
		})
	}
}
