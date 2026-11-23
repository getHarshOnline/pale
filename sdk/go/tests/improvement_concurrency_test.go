package tests_test

import (
	"errors"
	"testing"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

type improvementCommitResult struct {
	index   int
	preview *pale.ConfigurationImprovementPreview
	err     error
}

// A shared start barrier exercises real concurrent ABI calls without sleeps or
// timing thresholds. The assertions hold for either scheduling order.
func raceImprovementCommits(explanations [2]*pale.ConfigurationImprovementExplanation, prepared *pale.ConfigurationImprovementPreparedPreview) [2]improvementCommitResult {
	start := make(chan struct{})
	results := make(chan improvementCommitResult, 2)
	for index, explanation := range explanations {
		go func() {
			<-start
			preview, err := explanation.Commit(prepared)
			results <- improvementCommitResult{index, preview, err}
		}()
	}
	close(start)
	return [2]improvementCommitResult{<-results, <-results}
}

func TestConfigurationImprovementConcurrentSamePairHasOneWinner(t *testing.T) {
	context := newImprovementContext(t)
	for range 32 {
		explanation := context.explain(t)
		prepared, err := explanation.Prepare(context.candidate, context.request)
		if err != nil {
			t.Fatal(err)
		}
		results := raceImprovementCommits([2]*pale.ConfigurationImprovementExplanation{explanation, explanation}, prepared)
		var winner *pale.ConfigurationImprovementPreview
		for _, result := range results {
			if result.err == nil {
				if winner != nil || result.preview == nil {
					t.Fatal("same pair produced more than one preview")
				}
				winner = result.preview
			} else {
				var bridge *pale.BridgeError
				if !errors.As(result.err, &bridge) || bridge.Code != "invalid_handle" {
					t.Fatalf("losing commit changed admission semantics: %v", result.err)
				}
			}
		}
		if winner == nil {
			t.Fatal("concurrent commits lost both live owners")
		}
		summary, err := winner.Summary()
		if err != nil {
			t.Fatal(err)
		}
		assertImprovementDocument(t, summary, context.expected.Summary)
		if _, err := winner.Cancel(); err != nil {
			t.Fatal(err)
		}
	}
}

func TestConfigurationImprovementCompetingExplanationsPreserveLoser(t *testing.T) {
	context := newImprovementContext(t)
	for range 32 {
		explanations := [2]*pale.ConfigurationImprovementExplanation{context.explain(t), context.explain(t)}
		prepared, err := explanations[0].Prepare(context.candidate, context.request)
		if err != nil {
			t.Fatal(err)
		}
		results := raceImprovementCommits(explanations, prepared)
		apiOutcomes := 0
		for _, result := range results {
			var diagnostic *pale.DiagnosticError
			switch {
			case result.err == nil:
				apiOutcomes++
				if result.index != 0 || result.preview == nil {
					t.Fatal("foreign explanation acquired a semantic preview")
				}
				if _, err := result.preview.Reject(); err != nil {
					t.Fatal(err)
				}
			case errors.As(result.err, &diagnostic):
				apiOutcomes++
				if result.index != 1 {
					t.Fatalf("exact owner failed semantic admission: %v", result.err)
				}
				assertImprovementDiagnostic(t, result.err, context.expected.Diagnostics["preparation_owner_mismatch"])
			default:
				var bridge *pale.BridgeError
				if !errors.As(result.err, &bridge) || bridge.Code != "invalid_handle" {
					t.Fatalf("unexpected competing-commit failure: %v", result.err)
				}
			}
		}
		if apiOutcomes != 1 {
			t.Fatalf("expected one API-consuming outcome, got %d", apiOutcomes)
		}
		var survivor *pale.ConfigurationImprovementExplanation
		for _, explanation := range explanations {
			if _, err := explanation.State(); err == nil {
				if survivor != nil {
					t.Fatal("winning explanation remained unconsumed")
				}
				survivor = explanation
			}
		}
		if survivor == nil {
			t.Fatal("losing commit stranded its explanation")
		}
		retry, err := survivor.Prepare(context.candidate, context.request)
		if err != nil {
			t.Fatal(err)
		}
		preview, err := survivor.Commit(retry)
		if err != nil {
			t.Fatal(err)
		}
		if _, err := preview.MarkStale(); err != nil {
			t.Fatal(err)
		}
	}
}

func TestConfigurationImprovementCommitAndProposeDoNotLoseUnacquiredPreparation(t *testing.T) {
	context := newImprovementContext(t)
	for range 32 {
		explanation := context.explain(t)
		prepared, err := explanation.Prepare(context.candidate, context.request)
		if err != nil {
			t.Fatal(err)
		}
		type proposalResult struct {
			proposal *pale.ConfigurationImprovementProposal
			err      error
		}
		start := make(chan struct{})
		committed := make(chan improvementCommitResult, 1)
		proposed := make(chan proposalResult, 1)
		go func() {
			<-start
			preview, err := explanation.Commit(prepared)
			committed <- improvementCommitResult{preview: preview, err: err}
		}()
		go func() {
			<-start
			proposal, err := explanation.Propose(pale.ConfigurationImprovementProposalRequest{Candidate: context.request, Workspace: context.source})
			proposed <- proposalResult{proposal, err}
		}()
		close(start)
		commit, proposal := <-committed, <-proposed
		if (commit.err == nil) == (proposal.err == nil) {
			t.Fatalf("one live explanation must yield one transition: commit=%v propose=%v", commit.err, proposal.err)
		}
		loser := proposal.err
		preview := commit.preview
		if proposal.err == nil {
			loser = commit.err
			summary, err := prepared.Summary()
			if err != nil {
				t.Fatalf("losing commit consumed an unacquired preparation: %v", err)
			}
			assertImprovementDocument(t, summary, context.expected.Summary)
			evaluation, err := proposal.proposal.Evaluate()
			if err != nil {
				t.Fatal(err)
			}
			diagnosis, err := evaluation.Diagnose()
			if err != nil {
				t.Fatal(err)
			}
			preview, err = diagnosis.Preview()
			if err != nil {
				t.Fatal(err)
			}
		}
		var bridge *pale.BridgeError
		if !errors.As(loser, &bridge) || bridge.Code != "invalid_handle" {
			t.Fatalf("losing transition changed semantic admission: %v", loser)
		}
		if _, err := preview.Accept(); err != nil {
			t.Fatalf("winning transition lost its terminal owner: %v", err)
		}
	}
}

func TestConfigurationImprovementMalformedProposeConsumesOnlyInvokedExplanation(t *testing.T) {
	context := newImprovementContext(t)
	for _, kind := range []string{"nil-documents", "nil-bytes"} {
		t.Run(kind, func(t *testing.T) {
			explanation := context.explain(t)
			source := context.source
			source.Documents = append([]pale.Document(nil), source.Documents...)
			if kind == "nil-documents" {
				source.Documents = nil
			} else {
				source.Documents[0].Bytes = nil
			}
			_, err := explanation.Propose(pale.ConfigurationImprovementProposalRequest{Candidate: context.request, Workspace: source})
			var bridge *pale.BridgeError
			if !errors.As(err, &bridge) || bridge.Code != "invalid_binding_request" {
				t.Fatalf("wrong malformed request failure: %v", err)
			}
			if _, err := explanation.State(); !errors.As(err, &bridge) || bridge.Code != "invalid_handle" {
				t.Fatalf("invoked explanation remained available: %v", err)
			}
			// The same client and baseline remain usable after a rejected invocation.
			fresh := context.explain(t)
			prepared, err := fresh.Prepare(context.candidate, context.request)
			if err != nil {
				t.Fatal(err)
			}
			preview, err := fresh.Commit(prepared)
			if err != nil {
				t.Fatal(err)
			}
			if _, err := preview.Cancel(); err != nil {
				t.Fatal(err)
			}
		})
	}
}

func TestConfigurationImprovementClosedClientRejectsBeforeHandover(t *testing.T) {
	context := newImprovementContext(t)
	inspection, err := context.baseline.InspectConfigurationImprovement()
	if err != nil {
		t.Fatal(err)
	}
	explanation := context.explain(t)
	prepared, err := explanation.Prepare(context.candidate, context.request)
	if err != nil {
		t.Fatal(err)
	}
	context.client.Close()
	_, explainErr := inspection.Explain(context.profile)
	_, commitErr := explanation.Commit(prepared)
	_, proposeErr := explanation.Propose(pale.ConfigurationImprovementProposalRequest{Candidate: context.request, Workspace: context.source})
	_, inspectionErr := inspection.State()
	_, explanationErr := explanation.State()
	_, preparationErr := prepared.Summary()
	for _, err := range []error{explainErr, commitErr, proposeErr, inspectionErr, explanationErr, preparationErr} {
		var bridge *pale.BridgeError
		if !errors.As(err, &bridge) || bridge.Code != "closed_client" {
			t.Fatalf("pre-invocation rejection surrendered a handle: %v", err)
		}
	}
}

func TestConfigurationImprovementConcurrentExplainHasOneHandover(t *testing.T) {
	context := newImprovementContext(t)
	for range 32 {
		inspection, err := context.baseline.InspectConfigurationImprovement()
		if err != nil {
			t.Fatal(err)
		}
		type result struct {
			explanation *pale.ConfigurationImprovementExplanation
			err         error
		}
		start := make(chan struct{})
		results := make(chan result, 2)
		for range 2 {
			go func() { <-start; value, err := inspection.Explain(context.profile); results <- result{value, err} }()
		}
		close(start)
		winners := 0
		for range 2 {
			outcome := <-results
			if outcome.err != nil {
				var bridge *pale.BridgeError
				if !errors.As(outcome.err, &bridge) || bridge.Code != "invalid_handle" {
					t.Fatalf("loser crossed a second native handover: %v", outcome.err)
				}
				continue
			}
			winners++
			prepared, err := outcome.explanation.Prepare(context.candidate, context.request)
			if err != nil {
				t.Fatal(err)
			}
			preview, err := outcome.explanation.Commit(prepared)
			if err != nil {
				t.Fatal(err)
			}
			if _, err := preview.Reject(); err != nil {
				t.Fatal(err)
			}
		}
		if winners != 1 {
			t.Fatalf("got %d explanation handovers", winners)
		}
	}
}

// Each closure reaches the real next phase and reads its result. The companion
// query checks whether host preflight preserved the original Go coordinate.
func reviewHandover(t *testing.T, context improvementContext, stage string) (func() error, func() error) {
	t.Helper()
	proposal, err := context.explain(t).Propose(pale.ConfigurationImprovementProposalRequest{Candidate: context.request, Workspace: context.source})
	if err != nil {
		t.Fatal(err)
	}
	if stage == "evaluate" {
		return func() error {
			next, err := proposal.Evaluate()
			if err != nil {
				return err
			}
			_, err = next.State()
			return err
		}, func() error { _, err := proposal.Phase(); return err }
	}
	evaluation, err := proposal.Evaluate()
	if err != nil {
		t.Fatal(err)
	}
	if stage == "diagnose" {
		return func() error {
			next, err := evaluation.Diagnose()
			if err != nil {
				return err
			}
			_, err = next.State()
			return err
		}, func() error { _, err := evaluation.State(); return err }
	}
	diagnosis, err := evaluation.Diagnose()
	if err != nil {
		t.Fatal(err)
	}
	if stage == "preview" {
		return func() error {
			next, err := diagnosis.Preview()
			if err != nil {
				return err
			}
			_, err = next.Summary()
			return err
		}, func() error { _, err := diagnosis.State(); return err }
	}
	preview, err := diagnosis.Preview()
	if err != nil {
		t.Fatal(err)
	}
	var terminal func() (pale.ConfigurationImprovementTerminal, error)
	switch stage {
	case "accept":
		terminal = preview.Accept
	case "reject":
		terminal = preview.Reject
	case "cancel":
		terminal = preview.Cancel
	case "stale":
		terminal = preview.MarkStale
	default:
		t.Fatalf("unknown test stage %s", stage)
	}
	return func() error {
		result, err := terminal()
		if err == nil && len(result.Preview.Bytes) == 0 {
			return errors.New("terminal lost its retained preview")
		}
		return err
	}, func() error { _, err := preview.Summary(); return err }
}

func TestConfigurationImprovementRemainingStagesPreservePreflightAndSingleHandover(t *testing.T) {
	for _, stage := range []string{"evaluate", "diagnose", "preview", "accept", "reject", "cancel", "stale"} {
		t.Run(stage, func(t *testing.T) {
			context := newImprovementContext(t)
			for range 16 {
				invoke, _ := reviewHandover(t, context, stage)
				raceHandover(t, invoke)
			}
			invoke, query := reviewHandover(t, context, stage)
			context.client.Close()
			for range 2 {
				requireHandoverError(t, invoke(), "closed_client")
				requireHandoverError(t, query(), "closed_client")
			}
		})
	}
}
