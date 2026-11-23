+++
description = "Test a complete user task with isolated state, controlled waits and useful failure diagnostics; distinguish product failures from test and environment problems."
uses = [{ scope = "local", resource_id = "design" }]

[guide]
+++

# Check a task from entry to result

Use an end-to-end check when the claim depends on several parts of the product working together. Before choosing automation, define who performs the task, the starting state, ordered actions, expected visible or application result, outcomes that must not occur, cleanup and the exact environment.

Give each scenario repeatable setup and independent state. Wait for an observable change instead of sleeping for an arbitrary interval. Hide unstable interface mechanics behind a helper only when it preserves the user's intent and makes failures easy to locate; every journey does not need a page-object hierarchy.

Collect only the logs, traces, screenshots or state read-back needed to locate a failed transition, and handle them under the project's privacy rules. Distinguish product failure, a stale locator or adapter, a timing race, an unavailable dependency and an environment defect.

A quarantined test remains unresolved. Give it an owner and a condition for returning it to the normal checks. Retries can reveal intermittent failures but cannot turn them into a pass.

In the existing test result or review reply, state the task tested, environment and data used, scenario outcomes, failure causes supported by the observations, useful artifact locations and parts not exercised. Keep a reusable regression in the maintained tests; no separate journey dossier is needed.

## Before handing off

- [ ] Did the check follow the complete task from entry to result?
- [ ] Did it wait for the relevant state and isolate data that could make the result unreliable?
- [ ] Do the retained failure details let someone reproduce and locate the problem?
