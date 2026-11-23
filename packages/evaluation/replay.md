# Review a replay or notebook comparison

Choose the comparison that answers the question: a deterministic property check, replay of one fixed stage, an end-to-end comparison or a separate causal study. Inspect the fixed method and case set before trusting scores. Verify that an independent reviewer can see the original criterion and candidate outputs.

When reconstructing a comparison from saved reports or sheets, identify the observed rows and fields used by the criterion and scoring rule. Keep reference questions, expected answers, outputs and scored observations distinct. Compare the actual aggregation ranges and case identities on both sides. If coverage or scoring differs, report each result within its own scope. Claim improvement only after establishing a valid common comparison. Copying a worksheet or renaming a case derived from the same observation adds no independent evidence.

Use a known-bad case, including a leaked feature or a holdout used during tuning where relevant. Check denominators, baseline, coverage, failure groups, calibration, reviewer disagreement and cost. Inspect a clean notebook run when the conclusion depends on it. Show invalid and unavailable cases. Return an accept, revise or hold finding with the evidence that could change it. Applying a policy or taking an action still needs the relevant permission.

## Check deterministic event replay

Identify the initial state and its digest, event identities and bytes in order, logical time and tie order, implementation and configuration, seeds and interventions. State which outputs and final-state observations must match exactly and which measurements permit a stated tolerance. Rebuild in a fresh target with those inputs. Compare outputs and the resulting state for every implementation covered by the claim.

Test omitted, repeated and reordered events, a changed starting state and an incompatible implementation or schema. Verify rejection through the relevant entry point. Locate the first divergence and preserve its evidence in the check result. Matching final output does not establish that the required events ran in order. Keep deterministic agreement separate from release, live recovery and permission to act.

## Before handing off

- [ ] Confirm the claim and what changed between runs.
- [ ] Match inputs, metrics and relevant controls.
- [ ] Keep limitations and negative results with the comparison.
