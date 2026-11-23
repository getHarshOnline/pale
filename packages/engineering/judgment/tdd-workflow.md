# Test a behaviour before implementing it

Use this workflow when a failing executable example is the clearest way to pin down consequential behaviour. Prose, metadata, formatting and mechanical changes usually need the relevant existing check instead. Read [the shared engineering method](common.md) for useful assertions, fixtures and failure cases.

## The loop

1. **Red:** arrange the smallest input and state that express the next requirement, act through the intended entry point, and assert its result. Run the test and watch it fail because that behaviour is missing, rather than because setup or compilation failed.
2. **Green:** implement the whole required behaviour with the least machinery it needs. Avoid adding speculative features just to make the implementation feel complete.
3. **Refactor:** improve the structure while the relevant suite remains green. Keep its assertions and the behaviour they protect.

For a bug, begin with a failing regression test when the defect can be reproduced deterministically at a maintainable boundary. Otherwise use the smallest reliable reproduction and explain in the reply how you verified the fix. If the test is valid, fix the code; revise the test only when its requirement or setup is wrong.

Use fast local tests for local rules, integration tests for boundaries and a few end-to-end tests for critical journeys. Select tests for the behaviour and failure cases they protect, rather than a coverage percentage. Keep each coherent unit of work passing its relevant checks; if an existing failure prevents that, report it explicitly with the result.
