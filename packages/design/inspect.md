+++
description = "Inspect the actual browser interface through navigation, presentation modes, capability loss and measured captures; judge the design separately from the measurements."
uses = [{ scope = "local", resource_id = "access/web" }]
see_also = [{ scope = "local", resource_id = "animate" }, { scope = "local", resource_id = "independent" }, { scope = "local", resource_id = "responsive" }, { scope = "local", resource_id = "controls" }, { scope = "local", resource_id = "presentation" }, { scope = "local", resource_id = "variations" }, { scope = "local", resource_id = "figures" }, { scope = "local", resource_id = "inspect/choose-cases" }, { scope = "local", resource_id = "inspect/sweep" }, { scope = "local", resource_id = "inspect/lenses" }, { scope = "local", resource_id = "inspect/inspect-the-browser" }, { scope = "local", resource_id = "inspect/measure-and-look" }, { scope = "local", resource_id = "inspect/design-tools" }, { scope = "local", resource_id = "inspect/critique-a-treatment" }, { scope = "local", resource_id = "inspect/resolve-design-tensions" }, { scope = "local", resource_id = "inspect/check-failure-controls" }]

[guide]
+++

# Inspect an interface in its rendered context

Use this when making a claim about a page or browser journey. Open the actual route and compare the delivered result with the reference. Decide which findings concern the build, the task in use or the design. Check the affected states and the original failure.

Follow the repair from observation to cause, change, affected consumers and verification. Locate each defect at the level that produces it: overall direction or identity, shared component or recipe, one page composition, or local polish. Change that source, follow its actual dependents and recheck the original failure and plausible regressions. Put the findings and checks in the existing review or reply. A skipped check remains unrun, and checking your own work does not supply independent review or the owner’s acceptance.

| If you are doing this | Read this |
| --- | --- |
| Pick routes, page families and states | [Choose the cases that can answer a rendered question](inspect/choose-cases.md) |
| Choose or diagnose the actual browser, input and preference route | [Use the review browser for what it can observe](inspect/inspect-the-browser.md) |
| Check ordinary defects on the actual page | [Check the page before judging its treatment](inspect/sweep.md) |
| Judge the whole result through its design decisions | [Judge the design through its working parts](inspect/lenses.md) |
| Inspect geometry, interaction and design quality | [Measure the defect and read the whole treatment](inspect/measure-and-look.md) |
| Run a source check or browser capture | [Choose a source check or browser capture](inspect/design-tools.md) |
| Review whether the visible result suits the brief | [Critique the treatment against the actual brief](inspect/critique-a-treatment.md) |
| Compare the two ways a treatment can fail | [Find what a visual repair is trading away](inspect/resolve-design-tensions.md) |
| Use a known failure to check a test or measurement | [Show that a rendered check catches the defect](inspect/check-failure-controls.md) |

## Before handing off

- [ ] Was the intended build reached through its real route?
- [ ] Does the complete output work at the widths, states and input or recovery paths affected by the change?
- [ ] Are findings tied to their producing sources, with unrun checks stated?
