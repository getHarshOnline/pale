+++
description = "Choose checks that can catch the required failure, design a useful test and review what the result supports."
uses = [{ scope = "local", resource_id = "change" }]

[guide]
exposure = "internal"

[task]
roles = [{ scope = "local", resource_id = "testing" }]
outputs = ["In the reply: the observable requirement, failure cases, test design and results actually run.", "Any plan still to run, remaining untested behaviour and why the check stopped."]

[role]

[policy]
+++

# Choose checks that can catch the failure

Start with the requested outcome and the consequences found while tracing the change. For each claim, identify what can be observed, which component produces it, what a wrong result would look like, and the environment needed to distinguish correct behaviour from failure. Begin with the smallest useful check, then cover the risks the change actually introduces.

## Choose the check from the claim

Use deterministic checks for mechanical rules. Use scenarios, rendered output, performance measurements, security checks or expert review when the claim depends on those conditions. A quick self-check is the starting point; the guarantees the system must keep determine the required work.

For regressions, parsers, state changes, calculations and untrusted input, observing the missing behaviour before the fix often makes the test stronger. Use the test-first loop below when a failing executable example can pin down consequential behaviour. Prose, metadata, formatting and mechanical changes usually need the relevant existing check or inspection; a connection to another system may need a real integration check. Choose the check that can answer the question.

Read the project's declared checks and find out what they do before running them. A command called `check` can generate files, synchronize state or contact a service. Use the project's actual runner, fixtures and permission for those effects; an example command in a guide does not establish any of them. If the assertion, fixture, environment or permission cannot support a reliable run, explain the limit instead of inventing a command or treating coverage as proof.

Expand the checks according to the risk: the code that causes the behaviour, the changed interface, direct users, a representative journey, then wider regressions. A large suite can miss the failure, and a focused test supports only the path it covers. For an area expected to remain unchanged, check the interface that isolates it or the representative behaviour identified in the analysis.

## Make a wrong result observable

Choose a requirement or regression with an observable outcome. State what each important case must see and why an incorrect implementation would fail it. Use the project's normal entry point with suitable fixtures, keeping test execution and source changes within their existing permission.

Before treating a failing test as evidence of the bug, read its assertion and actual result. A setup or fixture error means the test has not reached the behaviour it needs to check. For a journey, inspect setup, state transitions, assertions, cleanup and failure output.

When a known-good baseline exists, a small deliberate regression in a disposable copy can establish that the test catches the failure without changing the working branch. Restore the exact candidate and rerun the same check before reporting the final result. For test-first work, establish that the initial failure represented the missing behaviour and that the final test would catch its return.

## Test consequential behaviour before implementing it

1. **Red:** arrange the smallest input and state that express the next requirement, act through the intended entry point, and assert its result. Run the test and watch it fail because that behaviour is missing, rather than because setup or compilation failed.
2. **Green:** implement the whole required behaviour with the least machinery it needs. Avoid adding speculative features just to make the implementation feel complete.
3. **Refactor:** improve the structure while the relevant suite remains green. Keep its assertions and the behaviour they protect.

For a bug, begin with a failing regression test when the defect can be reproduced deterministically at a maintainable boundary. Otherwise use the smallest reliable reproduction and explain in the reply how you verified the fix. If the test is valid, fix the code; revise the test only when its requirement or setup is wrong.

Use fast local tests for local rules, integration tests for boundaries and a few end-to-end tests for critical journeys. Select tests for the behaviour and failure cases they protect, rather than a coverage percentage. Keep each coherent unit of work passing its relevant checks; if an existing failure prevents that, report it explicitly with the result.

## Cover durable state and consequential effects

When a change owns durable state or an effect that cannot be taken back, a passing happy path says little about recovery. Test, as applicable, that the state commits before anything is notified or acted on; duplicate delivery and idempotency; concurrent writers, attempt generations and fencing; cursor continuity, gaps, replay and rebuild; a restart at every point where state becomes durable; timeouts and cleanup of the whole process group; cancellation races and late results; the case where the owner commits but the acknowledgement fails; an uncertain effect, recorded as unknown instead of guessed; and migration, downgrade and removal.

## Confirm the named check ran

Check the selected suite, filter and resolved test targets, then read which required assertions ran and their outcomes. Keep executed, failed, skipped and filtered tests distinct. A green command with no relevant tests may establish compilation while leaving the behaviour unchecked. Keep that claim open until its check runs or the responsible owner accepts a narrower claim.

For a new test selector or CI check, try a matching test, a deliberately nonmatching filter and a planted failure where practical. Confirm that its completion rule distinguishes all three. Report this with the test outcome, rather than creating a separate account of the run.

For generated agent instructions, check compilation, writing the output, the host making it available, invoking the tool and using its result separately. Start a fresh session to observe that route. Open links from the generated file and confirm they reach the named sections. Where a procedure claims to catch a missing input or a wrong tool route, deliberately introduce that failure in a safe test and check the response.

## Review what the tests establish

As the independent reviewer, trace each requirement to its test boundary, fixture state, relevant failure cases and executed result. Challenge tests that mirror implementation, pass for the wrong reason, depend on hidden order or omit a failure path required by the claim. Judge whether the tests would fail if the claimed behaviour were wrong.

Remove vanity checks that compare source text without testing a required behaviour. Keep literal-output assertions when that output is itself the contract. Parameterise repeated cases and share fixtures where they express the same rule, while preserving the distinct failure each case must catch. Reducing test machinery must retain the checks that establish correctness.

Distinguish a product defect, a test defect, a harness failure, an uncertain environment, an unavailable capability and a check that was not run, so the finding points to the right repair. Return findings tied to the affected requirement, the missing checks and what the available results support. The requesting task owns test design, source edits, execution and any output files; this review supplies its judgment to that task.

## Explain the result and its limits

In the reply or existing verification record, pair each claim with the check run, its result, the artifact or environment tested, and any remaining limit. Clearly label a plan that was not run. Name the behaviour the checks did not reach. A required check that fails, is stale or remains unknown leaves the work open unless the responsible owner has explicitly accepted the limit.
