+++
description = "Find recurring visual, token, component and accessibility problems in an existing interface."

[task]
uses = [{ scope = "local", resource_id = "system" }, { scope = "local", resource_id = "inspect" }, { scope = "local", resource_id = "creative-work" }]
outputs = ["Located findings, priorities and rendered observations in the existing review."]
+++

# Find repeated design problems

Choose the routes, states, viewports and product requirements the audit covers, then inspect the system that actually ships. Compare the same qualities across representative tasks, citing both the producing code and the rendered result. If using scores, keep a severe accessibility, hierarchy, responsive-layout or interaction failure visible in its own right; an average must not hide it.

Trace each discrepancy to its token, component, state rule or local composition. Distinguish a repeated system defect from a page using the system incorrectly and from intentional variation. Preserve the meaning and supported variation when proposing a fix. Keep a source-code finding, observed behavior and the owner's aesthetic judgment distinct.

Return the findings in the existing review or reply, ordered by consequence for users and how widely the correction would help. Name each correction's source and affected consumers. Mark visual conclusions that could not be exercised in the browser as untested.

## Before handing off

- [ ] Were recurring decisions checked in representative real tasks?
- [ ] Is each difference understood as intentional variation or a defect?
- [ ] Can the priority corrections be followed from their source into the affected pages and states?
