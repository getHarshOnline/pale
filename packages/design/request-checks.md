+++
description = "Request independent checks of measurable claims about the exact design implementation."

[task]
uses = [{ scope = "local", resource_id = "independent" }]
roles = [{ scope = "local", resource_id = "verifier" }]
outputs = ["Revision, actual observations, capture paths and unresolved checks in the existing review or reply."]
+++

# Request checks of a design implementation

Give the selected design verifier the current request, references, exact artifact, open findings and measurable claims to test. State the relevant surfaces, states and conditions. When using an independent fallback, include the selected verifier's procedure.

Return the actual observations in the existing review or reply, with the tested revision and any checks that could not run. Trace failures to their producing source where the observations support it. Keep a pending judgment about whether the design suits the brief separate from measured behavior.

## Before handing off

- [ ] Does each check answer a named question about a surface and state?
- [ ] Was the real path exercised, or is the unavailable check clearly identified?
- [ ] Was the result inspected and each failure traced as far as the observations allow?
