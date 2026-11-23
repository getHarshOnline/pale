# Check recurring behaviour after an AI-assisted change

Use this guide when a model helped generate, choose or repair a change and the affected behaviour can recur. Preserve the product's observable requirements and known failure cases. Different wording can be acceptable when it supports the same correct result.

## Decide what the check must establish

Start with the defect, requested behaviour or rule that must always hold. Name the input class, expected outcome, prohibited outcome and component responsible. Prefer deterministic tests for schemas, parsing, permissions, state changes, effect limits and artifact structure. Use a scored evaluation when the desired behaviour needs judgment, with a rubric grounded in examples and consequences.

Distinguish three concerns:

- Product behaviour must remain correct regardless of how its code was written.
- Agent behaviour includes choosing tools, using evidence and refusing prohibited actions.
- Model output can vary within the product's requirements.

Avoid freezing incidental wording, internal reasoning, timestamps or provider noise into the test.

## Build a useful regression check

Reproduce the failure before changing the implementation when safe and possible. Reduce the fixture while keeping what causes the failure. Include negative and adversarial cases for plausible misuse; keep secrets and personal data out of fixtures. For judgment evaluations, version the case, rubric, scorer, threshold and rules for ties or uncertainty. The producing agent's self-review can suggest a defect; it cannot supply an independent check.

Run the focused regression first, then the repository checks required by the parts affected. Distinguish flaky tests, environment dependence and model or version sensitivity from a product failure. Passing with one model or prompt supports only the route exercised unless other evidence establishes the broader claim.

In the review or reply, state what the regression checks, where its fixture came from, the expected result or rubric, what ran, observed variability, coverage and remaining risk. Hold the check when reproducing the failure would be unsafe, the expected result would decide an unapproved product question, evaluation lacks needed independence, private evidence cannot be sanitised, or the test would be more brittle than the behaviour it protects.

## Worked example

A review agent used a generated summary where the request required reading the source table. Give it a case where the summary and source disagree. Check that it opens the actual source and reports which row changes the result. Accept different wording that supports the same decision. Separately test whether the harness can open that source and whether its read-only tool blocks a write.

## Before handing off

- [ ] Are the required behaviour and acceptable variation clear?
- [ ] Have state changes and tool actions been exercised through the relevant turns?
- [ ] Are behaviour, cost and latency judged against their own thresholds?
