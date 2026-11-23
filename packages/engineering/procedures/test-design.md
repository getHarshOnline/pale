# Design a test that catches the wrong behaviour

Choose a requirement or regression with an observable outcome. For each important case, state what the test must see and why an incorrect implementation would fail it. Use the project's normal entry point with suitable fixtures, and keep test execution and source changes within the permission for each.

Before treating a failing test as evidence of the bug, read its assertion and actual result. A setup or fixture error means the test has not yet reached the behaviour it needs to check. Keep that distinction in the test result or reply.

When a known-good baseline exists, a small deliberate regression in a disposable copy can establish that the test catches the failure without changing the working branch. Restore the exact candidate and rerun the same check before reporting the final result.

## Before handing off

- [ ] Does the test identify the observable requirement and how it can fail?
- [ ] Does it reach a normal entry point with suitable fixtures?
- [ ] Do the assertions cover the outcome and state the test can actually observe?
