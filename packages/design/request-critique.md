+++
description = "Request an independent design judgment of the exact plan or rendered artifact."

[task]
uses = [{ scope = "local", resource_id = "independent" }]
roles = [{ scope = "local", resource_id = "critic" }]
outputs = ["Located findings, qualities to preserve and a verdict for the reviewed revision in the existing review."]
+++

# Request a design critique

Give the selected design critic the current request, corrections, references and their roles, accepted direction, exact artifact and open findings. State the decision the review should answer. When using an independent fallback, include the selected critic's procedure with those inputs.

Return the verdict in the existing review or reply, identifying the reviewed revision, strengths, gaps and the source of each proposed correction. Carry unresolved implementation and verification findings forward with the affected surface and condition; the critique does not establish checks the reviewer could not run.

## Before handing off

- [ ] Did the reviewer read the original direction and inspect the output in its intended form?
- [ ] Do the findings explain the strengths and gaps that matter to the request?
- [ ] Can each requested correction be traced to the source that produces the result?
