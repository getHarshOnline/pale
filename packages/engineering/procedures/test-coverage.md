# Find consequential gaps in the tests

Use the project's coverage report to locate code that may lack a useful test. Read [the shared engineering method](../judgment/common.md) to judge assertions and failure cases, then prioritise gaps by consequence, reachability and regression value. For each proposed test, say what result would fail if the behaviour were wrong.

Compare the covered count and eligible denominator within the same scope. Removing unexercised code or changing inclusion rules can raise a percentage without testing new behaviour; report that separately and check any removal through its caller. Keep a trivial uncovered line lower than a covered branch whose assertions miss a serious error.

Add or strengthen only meaningful tests within the task's scope. Do not weaken a threshold or add assertions that merely repeat the implementation to improve the number. In the reply, give the prioritised gaps, tests run, remaining unprotected behaviour and why the review is finished.

## Before handing off

- [ ] Do the proposed cases protect consequential behaviour?
- [ ] Could each case fail for the intended reason?
- [ ] Does the reply distinguish new protection from a changed coverage denominator?
