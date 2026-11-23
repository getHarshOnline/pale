# Check whether an evaluation method can support the decision

Build a reusable evaluation harness when behaviour recurs and comparing candidates or versions can change a decision. For a narrow mechanical property, use a direct deterministic check.

Fix the claim, cases, candidate identities, rubric, blocking criteria, scorer, threshold, uncertainty policy and cost limit before the comparison. Check schemas and state transitions with deterministic code. Use judgment with concrete examples for qualities that cannot honestly be reduced to mechanical checks.

Test the evaluation method itself. A mechanical checker must reject a known-bad control. A judgment rubric needs examples that distinguish adjacent grades, independent review when the stakes require it, and evidence that style or verbosity does not dominate the score. Keep holdout cases out of candidate-specific tuning.

Compare the same case revisions, available evidence, environment, budget and aggregation rule. Report disagreement between scorers, unstable results, missing coverage, sensitivity to thresholds and regressions hidden by an average. An average cannot cancel a blocking failure.

## Check eligibility separately from score and outcome

State the exact candidate population, what counts as one case, the decision time, evidence available then, eligibility policy, score version and outcome definition. Apply mandatory conditions before scoring. Distinguish eligible, disqualified and unresolved cases, including a condition that cannot be checked with the available evidence. Follow the owner's rule for unresolved cases. In the evaluation result, retain the reason for each decision and the strongest relevant evidence against it. For eligible cases, show the score components separately from a reviewer's decision or a later outcome.

Test a high-scoring candidate with a blocking condition, one with stale or missing eligibility evidence, and one that passes exactly at the boundary. Verify that the blocking condition is respected both in selection and in the exported view. Keep each decision tied to the policy and evidence used. A replay with different rules or evidence is a separate comparison with its own eligible population.

For calibration, report how many cases were screened, eligible, scored and observed through to an outcome. Include policy and scorer versions, the counted unit and time interval, and missing or pending outcomes. If outcomes are available only for cases that advanced, restrict outcome claims to that observed group and explain the selection limit. A rank or rubric score is not automatically a probability. Before using outcomes to justify a threshold or probability claim, state the applicable group, follow-up window and controls withheld from tuning. The owning system controls policy, exceptions, final decisions and outcome receipts.

## Test a rule-based checker on the consumed artifact

Identify the artifact revision, checker and rule versions, supported representations, parsing or segmentation rules, exception boundaries and work limits. To test an entry point, run the check through that entry point on the artifact it reads. Testing a matcher directly establishes only how it handles the input passed to it.

Choose controls before running the check. Place a prohibited value in every representation and boundary covered by the claim, paired with a permitted near match. Exercise normalization, decoding and exceptions where applicable. Move an allowed exception outside its scope and verify that it no longer excuses the violation. If an unsupported input, parse failure or work limit can stop inspection, check how the result reports that interruption and identify the unchecked region. Keep a detected violation, a checked region with no violation and an uninspected or indeterminate region distinct.

Inspect the output in the form its reader will use, including generated or stored forms when in scope. In the check result, say what the checker observed and where. Claims beyond the rules, such as meaning conveyed by a paraphrase, need independent judgment. A clean result covers only the selected rules, inputs and entry point. The owner decides which failures block acceptance and handles repair.

Return the evaluation rules, case and rubric versions, calibration evidence, candidate results, uncertainty, cost and the reason to accept, revise or stop. A score supplies evidence for a decision; it does not establish truth or permission to act.

## Before handing off

- [ ] Name the claim and what the scorer observes.
- [ ] Exercise useful controls and threshold cases.
- [ ] State the conditions where the score can support the claim.
