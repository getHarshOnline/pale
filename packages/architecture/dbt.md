# Review dbt models, hooks and tests

Use this guidance for a project that uses dbt. Identify its root, exact dbt Core and adapter versions, target, selected resources and package inputs. Several projects in one checkout may have independent commands and configuration. Inspect local dependency paths before resolving packages.

Read effective project and model settings and macros, then inspect the compiled selection and actual database relation. Schema-naming macros, aliases and materialisations can change the physical destination. The declared source graph does not include raw relations referenced directly in SQL. Check every positional-union arm against the output contract, including implicit type coercions.

Before execution, inspect pre/post hooks, run-level hooks, adapter materialisation and wrappers. A model containing only SELECT can still trigger writes. Check transaction and failure behaviour for the pinned adapter. Preserve the rule a hook protects when changing it. Model build, post-processing and webhook delivery have separate outcomes.

Read the incremental predicate and key configuration together. Test nulls, duplicates and historical rows under that strategy. Setting `unique_key` does not test or enforce source uniqueness. A model contract checks only configured properties supported by the adapter and materialisation; it does not prove business meaning or completeness.

In the review reply, identify the selected data, unit and singular tests and their results. Distinguish no selected tests from passing relevant assertions, and a model build from a test run. State unavailable packages, missing credentials and unrun checks. Exact commands, schemas, hooks and acceptance remain the project's responsibility.

## Owning references

- [dbt hooks](https://docs.getdbt.com/reference/resource-configs/pre-hook-post-hook): effective hook configuration, ordering and adapter-specific transaction behavior.
- [dbt unique_key](https://docs.getdbt.com/reference/resource-configs/unique_key): selected incremental identity and separate uniqueness testing.
- [dbt contracts](https://docs.getdbt.com/reference/resource-configs/contract): configured columns and platform/materialization constraints.

## Before handing off

- [ ] Resolve the actual project, target and model.
- [ ] Check effective hooks and the incremental path.
- [ ] Record which tests ran on the resulting data.
