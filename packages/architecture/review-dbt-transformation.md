# Review a dbt transformation

Identify the project revision, target, selected model and adapter. Apply the shared transformation method and dbt guidance. Trace the effective configuration through compiled output and the observed database work, including hooks and downstream wrapper actions.

In the review reply, give model, hook and test coverage, findings with their locations, actual output and failures, and remaining database checks. This review does not run dbt, resolve packages, change relations or start downstream work.

## Before handing off

- [ ] Resolve the effective model, target and hooks.
- [ ] Check incremental inputs and the test membership.
- [ ] Read the actual output and failures.
