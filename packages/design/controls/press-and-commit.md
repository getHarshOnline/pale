+++
description = "When a press feels inert, a click commits too early or a repeated action has side effects."
uses = [{ scope = "local", resource_id = "controls" }]

[guide]
exposure = "internal"
+++

# Answer a press immediately and commit the intended action

Use this when a press feels inert, a click commits too early or a repeated action has side effects.

Show feedback promptly when input is accepted. For an ordinary button, distinguish its pressed drawing from committing the effect: preserve the native activation event and cancellation behavior. Pressing, moving off and releasing should not accidentally commit the abandoned action. Do not replace a working click path with an effect on pointer-down merely to make it seem faster. The [pointer-cancellation explanation](https://www.w3.org/WAI/WCAG22/Understanding/pointer-cancellation.html) describes permitted alternatives and exceptions; timing tokens are a separate project choice. Keyboard activation needs its own observed feedback, not a claim inferred from pointer CSS.

Feedback should not reflow nearby content, change the hit region unexpectedly or make a label wrap. If movement is part of the accepted treatment, inspect the intermediate drawing, cleanup and reduced-motion alternative. Use the project's actual native tokens; scaling every control is not a universal rule. Gate hover enhancement by input capability, and retain the same task on touch and keyboard. A smaller visible icon may have a larger actual hit region.

Attach pending state to the operation that started it and keep unrelated work usable. Repeated activation must not duplicate a consequential request; a disabled-looking button alone does not establish server-side idempotency or protection after timeout. Follow the native operation's identity and read-back contract when its result is uncertain. Explain a genuinely disabled action nearby; an unexplained gray control does not tell someone how to proceed.

## Worked example: remove a saved draft

For a product that uses hold to clear local drafts, fill the control while held and leave the data in place until the hold finishes. Moving away or releasing early cancels. Give an accessible alternative, such as an explicitly named confirm action, and show progress without travel when reduced motion is active. Name what was removed. If remote drafts were outside the operation, leave the result specific to this device.

For a regular submit button, show the press immediately and let the usual activation event submit. On repeat input, keep the same pending request. A failed or timed-out response should let the person inspect what happened before trying a consequential action again.
