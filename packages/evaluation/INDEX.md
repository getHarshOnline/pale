# Evaluation

Package ID: `pale.evaluation` · [Manifest](package.toml)

Check decisions, compare and replay runs, calibrate tests, trace results and evaluate candidates independently.

A workspace selects entries from this family for the duties its people and agents perform. To read it directly, open the page for your task and follow its reading links.

## Guides

- [Check a workbook export or reconstructed view](<workbook.md>) · `workbook` · `doctrine`
  Check workbook bytes, required model features and recalculated output separately.
  Read first: [Trace evaluation inputs and compare runs](<runs.md>).
- [Check how a provider returns decisions](<typesafe.md>) · `typesafe` · `doctrine`
  Check how the selected provider returns decisions and confidence.
  Read as needed: [Choose the decision type and when to defer](<decision.md>).
- [Check notebook execution and exported results](<jupyter.md>) · `jupyter` · `doctrine`
  Use it to execute a notebook in a clean process in the approved environment and to check that the export carries the computed results.
  Read as needed: [Make decision notebooks reproducible](<notebook.md>).
- [Check numerical results against independent references](<numbers.md>) · `numbers` · `doctrine`
  Check numerical results against independent references, boundary cases and justified tolerances.
- [Check reference answers and fix dataset membership](<reference.md>) · `reference` · `doctrine`
  Check reference answers, acceptance, eligibility for each split and fixed dataset membership.
- [Check the candidate an optimiser produces](<optimisation.md>) · `optimisation` · `doctrine`
  Check the solver's emitted candidate, its feasibility and what the search actually proves.
  Read first: [Check numerical results against independent references](<numbers.md>).
- [Check trace identities, datasets and scores](<langfuse.md>) · `langfuse` · `doctrine`
  Check trace identities, dataset membership and score targets.
  Read as needed: [Use Langfuse's skill and native tools](<langfuse-tools.md>), [Trace evaluation inputs and compare runs](<runs.md>).
- [Check what an analysis counts](<rows.md>) · `rows` · `doctrine`
  Check what an analysis counts, how rows join and how totals and denominators reconcile.
- [Check whether an evaluation method can support the decision](<instrument.md>) · `instrument` · `doctrine`
  Test evaluation controls, thresholds and rubric examples, and state what comparisons they support.
  Read first: [Trace evaluation inputs and compare runs](<runs.md>).
- [Choose the decision type and when to defer](<decision.md>) · `decision` · `doctrine`
  Choose the decision type, check calibration and define when to defer.
- [Compare changes separately and together](<compare.md>) · `compare` · `doctrine`
  Compare changes separately and together under matched conditions, with independent acceptance evidence.
- [Keep a candidate's review history](<review.md>) · `review` · `doctrine`
  Fix the candidate and brief, separate production from review, and preserve findings, responses and stopping reasons.
- [Make decision notebooks reproducible](<notebook.md>) · `notebook` · `doctrine`
  Run a notebook from clean state and check for leaked information.
  Read as needed: [Check notebook execution and exported results](<jupyter.md>), [Trace evaluation inputs and compare runs](<runs.md>).
- [Trace evaluation inputs and compare runs](<runs.md>) · `runs` · `doctrine`
  Keep replay inputs comparable and trace each result to its run.
  Read as needed: [Check trace coverage, sampling and review costs](<runs/coverage.md>), [Identify which release, method and run support the claim](<runs/identity.md>), [Freeze the decision-time inputs for a fair replay](<runs/inputs.md>).
- [Use Langfuse's skill and native tools](<langfuse-tools.md>) · `langfuse-tools` · `doctrine`
  Use Langfuse's official skill and available CLI or MCP for the selected task, checking versions, permissions and the relevant workflow.
  Read as needed: [Check trace identities, datasets and scores](<langfuse.md>).

## Tasks

- [Check a workbook export or reconstructed view](<workbook.md>) · `workbook` · `operation`
  Check workbook bytes, required model features and recalculated output separately.
  Read first: [Trace evaluation inputs and compare runs](<runs.md>).
- [Check reference answers and fix dataset membership](<reference.md>) · `reference` · `operation`
  Check reference answers, acceptance, eligibility for each split and fixed dataset membership.
  Read first: [Trace evaluation inputs and compare runs](<runs.md>).
- [Check the candidate an optimiser produces](<optimisation.md>) · `optimisation` · `operation`
  Check the solver's emitted candidate, its feasibility and what the search actually proves.
  Read first: [Check numerical results against independent references](<numbers.md>).
- [Check what the evidence says about a relation](<relations.md>) · `relations` · `operation`
  Distinguish direct relation evidence, derived reachability and other properties that need separate proof.
  Read first: [Trace evaluation inputs and compare runs](<runs.md>).
- [Check whether an evaluation method can support the decision](<instrument.md>) · `instrument` · `operation`
  Test evaluation controls, thresholds and rubric examples, and state what comparisons they support.
  Read first: [Trace evaluation inputs and compare runs](<runs.md>), [Check numerical results against independent references](<numbers.md>).
- [Reconcile two independent reviews](<review/two.md>) · `review/two` · `operation`
  Reconcile two independent reviews while keeping disagreement visible.
  Read first: [Keep a candidate's review history](<review.md>).
  Read as needed: [Evaluate a candidate without changing it](<review/independent.md>).
- [Review a replay or notebook comparison](<compare-runs.md>) · `compare-runs` · `operation`
  Review a replay or notebook comparison against the exact claim it makes.
  Read first: [Choose the decision type and when to defer](<decision.md>), [Trace evaluation inputs and compare runs](<runs.md>), [Make decision notebooks reproducible](<notebook.md>), [Compare changes separately and together](<compare.md>), [Check numerical results against independent references](<numbers.md>).
- [Review successive versions of a candidate](<review/revise.md>) · `review/revise` · `operation`
  Review successive candidate versions against the same rubric and examples.
  Read first: [Keep a candidate's review history](<review.md>).
  Read as needed: [Produce a candidate for review](<review/generate.md>), [Evaluate a candidate without changing it](<review/independent.md>).

## Roles

- [Evaluate a candidate without changing it](<review/independent.md>) · `review/independent` · `role`
  Review a fixed candidate independently against the agreed rubric, examples and evidence.
  Read first: [Keep a candidate's review history](<review.md>).
- [Produce a candidate for review](<review/generate.md>) · `review/generate` · `role`
  Propose the next candidate within scope and answer review findings without treating its rationale as evidence.
  Read first: [Keep a candidate's review history](<review.md>).

## Stacks and tools

- [Check how a provider returns decisions](<typesafe.md>) · `typesafe` · `stack`
  Check how the selected provider returns decisions and confidence.
  Read as needed: [Choose the decision type and when to defer](<decision.md>).
- [Check notebook execution and exported results](<jupyter.md>) · `jupyter` · `stack`
  Use it to execute a notebook in a clean process in the approved environment and to check that the export carries the computed results.
  Read as needed: [Make decision notebooks reproducible](<notebook.md>).
- [Check trace identities, datasets and scores](<langfuse.md>) · `langfuse` · `stack`
  Check trace identities, dataset membership and score targets.
  Read first: [Use Langfuse's skill and native tools](<langfuse-tools.md>).
  Read as needed: [Trace evaluation inputs and compare runs](<runs.md>).
