+++
description = "Use it to show that a check catches the defect it is meant to, by running it on a fixture that contains the defect and then on the real input."
uses = [{ scope = "local", resource_id = "inspect" }]

[guide]
exposure = "internal"
+++

# Show that a rendered check catches the defect

Decide what the check can establish. A source scan can find a bad rule, a browser run can show how a control works on the built page, and a design review can judge whether the result serves the brief. Keep their outcomes separate. Use the actual route and rendered build for claims about behavior. Show what was observed, what failed, and which checks could not run.

A meaningful check should reject a known bad input. Run it on a disposable fixture that contains the specific defect. Then run the same check on the intended output. Use real observations in both cases, including the dimensions, text, focused element or failing request that establish the result. Avoid a change that the check cannot see or that breaks the fixture for an unrelated reason.

| Claim | Useful failing case |
| --- | --- |
| A responsive page has no root overflow | Extend an in-flow region beyond the layout viewport. Compare `scrollWidth` and `clientWidth` on the rendered document. Inspect normal scrollbar behavior separately. |
| Focus is visible and restored | Remove the focus treatment or redirect focus after dismissal. Check the focused element and the rendered indicator. |
| Values and sources agree across variants | Mutate a value, unit or source in one emitted form. Extract the displayed text or link from both forms and check that the comparison fails. |
| The caption has enough contrast over the image | Put part of the caption over a conflicting part of a known still or transition. Measure at the place and frame where the caption is read. |
| Measurements come from the reviewed build | Change a fixture value at its source, rebuild and compare the tool's observation. A number typed into a report cannot show that the tool read the build. |

Use the negative case when introducing a check or when its reach is in question. A working unchanged check need not replay its entire fixture suite for every local design change. Compare affected routes, previous failures and plausible new failure conditions. Say when a review used a single author and reviewer. A report on one build or cut does not confer approval to publish.

## Worked example

A page comparison checks that source links are consistent. Both sides pass even after changing the rendered destination in one fixture. The test was reading the same source object twice. Extract the links after rendering each view. Re-run the bad fixture and see the mismatched address, then check the intended output at the view widths the project supports.
