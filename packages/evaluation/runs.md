+++
description = "Keep replay inputs comparable and trace each result to its run."
see_also = [{ scope = "local", resource_id = "runs/identity" }, { scope = "local", resource_id = "runs/inputs" }, { scope = "local", resource_id = "runs/coverage" }]

[guide]

+++

# Trace evaluation inputs and compare runs

Fix the claim, case snapshot, method, prompt and shared dependencies, workflow and adapter, model and settings, validator, output schema, execution mode and available evidence. Distinguish a case, an assessment of it and a provider attempt. Keep raw output, normalized result, evaluator observation and accepted decision separately inspectable. Traces and scores record observations; the system that owns an action establishes whether it occurred.

For a replay of one stage, hold upstream outputs fixed and vary the target stage. For an end-to-end replay, regenerate upstream outputs under the candidate. Pair cases when the question and data permit it. Exclude later labels and outcomes from decision-time inputs, and prevent replay from taking live actions. Count failed attempts, retries, fallback, shadow calls and review in cost and coverage. Resolve disputed labels with independent evidence; leave unresolved cases visible instead of treating them as negative outcomes.

| If you need to do this | Read this |
| --- | --- |
| Identify which release, method and run support the claim | [Read the guide](runs/identity.md) |
| Freeze the decision-time inputs for a fair replay | [Read the guide](runs/inputs.md) |
| Check trace coverage, sampling and review costs | [Read the guide](runs/coverage.md) |

## Before handing off

- [ ] Keep the inputs, versions and output of each compared run.
- [ ] Rebuild the same comparison before interpreting a change.
- [ ] Separate an input change from a behavioral change.
