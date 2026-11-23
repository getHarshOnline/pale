+++
description = "Use it when a form or service fails, to show what happened, what the person can still use and the next action."
uses = [{ scope = "local", resource_id = "controls" }]

[guide]
exposure = "internal"
+++

# Help someone recover from a form or service failure

Show what happened, what the person can still use and the next available action.

Use visible labels and explain format and actual requirements before failure. Validate at a point where the person can act on the result, preserve entered values and place a specific fix beside the cause. Do not interrupt an unfinished answer with an error on every keystroke. Where following the [GOV.UK validation pattern](https://design-system.service.gov.uk/patterns/validation/), wait until advancing or submitting; that pattern explicitly advises against validation merely on leaving a field. Blur validation is not its rule. Other products need their own justified timing and observed usability.

After failed submission, use the established summary/focus route and connect each error to its field. An invalid value, service failure, eligibility decision and missing permission have different causes; do not tell someone to repair their input when the service is unavailable or permission is missing. Keep optional answers optional downstream. Normalize only what the input rules allow.

Carry existing answers and drafts through a channel switch or tenant handoff. Changing the next required fields should not clear the previous work. Preserve the appropriate reading, selection, scroll and draft state through closing, backtracking and retry. Persistence needs its real privacy, lifetime and storage contract; do not silently promise cross-session retention. Test blocked storage and reload only to the retention level actually offered. Focus restoration and any justified successor follow the accessibility prerequisite.

Keep action names stable through pending, confirmation and completion. State what happened, what survived failure and the real next step. Prepared is not sent; opening a mail or calendar application is not delivery or booking; a local selection is not an order. Do not manufacture stock, citations, responses, appointments or provider counts to make a state look finished. An error code alone or cheerful copy cannot replace a useful explanation.

## Worked example: a handoff that leaves a draft intact

A person enters contact details before opening their mail application. Keep the draft when the mail application opens. Show “Opened your mail app” and leave the draft available. If launch fails, say that the app could not open and offer a copy action. After a network error in a form, keep entered values and put a field error beside the field only if the field caused the failure. Explain a service or permission problem on its own terms.
