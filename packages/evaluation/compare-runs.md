+++
description = "Review a replay or notebook comparison against the exact claim it makes."

[task]
uses = [{ scope = "local", resource_id = "decision" }, { scope = "local", resource_id = "runs" }, { scope = "local", resource_id = "notebook" }, { scope = "local", resource_id = "compare" }, { scope = "local", resource_id = "numbers" }]
outputs = ["Method and case versions used in the comparison.", "Control results and limits.", "Recommendation to accept, revise or hold."]
+++

# Review a replay or notebook comparison

Choose the comparison that answers the question: a deterministic property check, replay of one fixed stage, an end-to-end comparison or a separate causal study. Inspect the fixed method and case set before trusting scores. Verify that an independent reviewer can see the original criterion and candidate outputs.

When reconstructing a comparison from saved reports or sheets, identify the observed rows and fields used by the criterion and scoring rule. Keep reference questions, expected answers, outputs and scored observations distinct. Compare the actual aggregation ranges and case identities on both sides. If coverage or scoring differs, report each result within its own scope. Claim improvement only after establishing a valid common comparison. Copying a worksheet or renaming a case derived from the same observation adds no independent evidence.

Use a known-bad case, including a leaked feature or a holdout used during tuning where relevant. Check denominators, baseline, coverage, failure groups, calibration, reviewer disagreement and cost. Inspect a clean notebook run when the conclusion depends on it. Show invalid and unavailable cases. Return an accept, revise or hold finding with the evidence that could change it. Applying a policy or taking an action still needs the relevant permission.

## Check deterministic event replay

Identify the initial state and its digest, event identities and bytes in order, logical time and tie order, implementation and configuration, seeds and interventions. State which outputs and final-state observations must match exactly and which measurements permit a stated tolerance. Rebuild in a fresh target with those inputs. Compare outputs and the resulting state for every implementation covered by the claim.

Test omitted, repeated and reordered events, a changed starting state and an incompatible implementation or schema. Verify rejection through the relevant entry point. Locate the first divergence and preserve its evidence in the check result. Matching final output does not establish that the required events ran in order. Keep deterministic agreement separate from release, live recovery and permission to act.

## Explain a past run with the versions it used

Read the run's retained method and prompt content, input snapshot, expanded plan, implementation and configuration versions, provider receipts, decision thresholds and applied results. Resolve these from the run's own references. The current default or a prompt labelled Latest cannot explain an earlier decision. Preserve an older run's policy even when the current method uses a different threshold or exemption.

For example, a retained answer of 0.78 passed a run's inclusive threshold of 0.75. A later method raises that threshold to 0.80. The historical result remains a pass under its recorded rule; applying the new rule is a separate comparison whose changed policy must be named. It does not retroactively authorise or undo an action.

Distinguish explaining retained receipts, rerunning the frozen method and reassessing with changed evidence or rules. A repeated model call need not reproduce the old response. If an exact version or input is unavailable, name that limit rather than silently substituting today's version. Keep live effects disabled during replay and preserve the relationship between the original and any new assessment.

## Before handing off

- [ ] Confirm the claim and what changed between runs.
- [ ] Match inputs, metrics and relevant controls.
- [ ] Keep limitations and negative results with the comparison.
