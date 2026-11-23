+++
description = "Design or repair controls, search, disclosure, drafts, recovery and consequential actions through the complete user task."
uses = [{ scope = "local", resource_id = "access/web" }]
see_also = [{ scope = "local", resource_id = "controls/attention-and-choice" }, { scope = "local", resource_id = "controls/control-states" }, { scope = "local", resource_id = "controls/press-and-commit" }, { scope = "local", resource_id = "controls/command-entry" }, { scope = "local", resource_id = "controls/command-focus" }, { scope = "local", resource_id = "controls/command-results" }, { scope = "local", resource_id = "controls/progressive-disclosure" }, { scope = "local", resource_id = "controls/empty-and-error-states" }, { scope = "local", resource_id = "controls/check-a-journey" }]

[guide]
+++

# Design controls that work through the whole task

Use this page when the work changes how someone searches, activates a control, completes a form, opens a detail or recovers from failure. Decide which application state changes, how the person reaches it and what happens when they repeat, cancel or come back.

| If you are doing this | Read this |
| --- | --- |
| Reduce guessing, memory load or competing choices | [Help people choose without unnecessary effort](controls/attention-and-choice.md) |
| Draw or test control states | [Keep a control understandable in every state](controls/control-states.md) |
| Handle a fast press, cancellation or repeated action | [Answer a press immediately and commit the intended action](controls/press-and-commit.md) |
| Choose a search or command entry | [Choose where search and commands belong](controls/command-entry.md) |
| Repair focus, typing or results | [Keep focus and input working in search](controls/command-focus.md) and [Explain what a search result can do](controls/command-results.md) |
| Add a preview, disclosure or long-list route | [Reveal more without hiding the task](controls/progressive-disclosure.md) |
| Protect a draft and recover from an error | [Help someone recover from a form or service failure](controls/empty-and-error-states.md) |
| Test the complete path through failure and return | [Exercise a real journey through failure and return](controls/check-a-journey.md) |

## Before handing off

- [ ] Is the intended action visible and reachable?
- [ ] Do feedback, focus, cancellation and repeated input work on the real control?
- [ ] Does recoverable failure keep the person’s input and make the next action clear?
- [ ] Does the operation’s actual result match what the interface reports?
