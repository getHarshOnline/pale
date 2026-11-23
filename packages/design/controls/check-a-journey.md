+++
description = "After a component change that affects navigation, data, drafts or the result of an action."
uses = [{ scope = "local", resource_id = "controls" }]

[guide]
exposure = "internal"
+++

# Exercise a real journey through failure and return

Use this after a component change that affects navigation, data, drafts or the result of an action.

Use real input against the actual component and native operation. Include the relevant slow response, rapid repeat, abandonment, error, retry, handoff and return path. Keep drafts and selection visible while testing those transitions. Inspect the chosen input and presentation combinations, including composition input or blocked persistence where reached. Report the states actually run; there is no requirement to generate a new atlas for an isolated repair.

Separate measured operability, visual quality and successful outcome in the service. A beautiful capture can conceal an inert control. A completed local action can leave a provider step unfinished. An illustrative film can depict a state without implementing its controls, but must not falsely claim a live capability. Keep component contracts, strings, draft schemas, storage keys, service bindings and final acceptance with their respective project owners.

## Run the paths a component actually exposes

| Component | Run it from the reader's starting point |
| --- | --- |
| Search | Scroll to mid-page, open, type, choose with arrows, follow, then use Back. Repeat with pointer and touch. Exercise no results, stale requests and a disconnected provider. |
| Shared tools | Open from a visible trigger, switch tools, return inside the shell, close. The original focus and draft should still be available. |
| Intake | Enter partial values, switch channel, trigger a failed submit, correct one field and retry. The selected channel may change required fields. It should preserve values already entered. |
| Journey | Check current position, visited state and published status separately. Move to the next part, return and read the position without scrolling back to the list. |
| Disclosure | Follow a direct question link, use keyboard, then disable scripts and search within the page. Keep essential qualifications near the action they govern. |
| Long list | Open a result beyond the first page, return and check the visible position, count and focus. If the set expands in place, the next useful action and footer remain reachable. |

Walk the path when optional features fail. Endnotes should still link back to the passage. A local draft can stay editable while discussion or sending is unavailable. Copying a prepared request, opening mail and actual delivery are separate outcomes. The message after each action should describe the outcome the tool observed and where the reader can continue.

## Worked example: retry a form safely

Enter valid text, force a slow response, press Submit twice and then return to the form after failure. Check that the draft remains, that only one request reached the service, that the explanation names the cause, and that focus lands where the person can recover. Try a retry and compare the result with the service’s read-back. If the test only ran against a stub, record the request behavior separately from the service outcome.
