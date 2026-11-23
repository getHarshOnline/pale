# Verify a behaviour through the project's test runner

State the behaviour to check and the observable result that would fail if it were wrong. Read [the shared engineering method](judgment/common.md), then choose the test and language guidance relevant to the risk. Use the repository's actual runner, fixtures and permitted execution scope.

In the reply, give the test design, what ran and its result, or clearly label a plan that was not run. Name any remaining behaviour the checks did not reach. If the assertion, fixture, environment or permission cannot support a reliable run, explain that limit instead of inventing a command or treating coverage as proof.

## Confirm the named check ran

Check the selected suite, filter and resolved test targets, then read which required assertions ran and their outcomes. Keep executed, failed, skipped and filtered tests distinct. A green command with no relevant tests may establish compilation while leaving the behaviour unchecked. Keep that claim open until its check runs or the owner accepts a narrower claim.

For a new test selector or CI check, try a matching test, a deliberately nonmatching filter and a planted failure where practical. Confirm that its completion rule distinguishes all three. Report the result with the test outcome, rather than creating a separate account of the run.
