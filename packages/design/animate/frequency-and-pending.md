+++
description = "Use it to set how much an action moves by how often it happens, and to keep pending responses useful."
uses = [{ scope = "local", resource_id = "animate/productive-motion" }]
see_also = [{ scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "web" }, { scope = "local", resource_id = "surfaces" }]

[guide]
exposure = "internal"
+++

# Keep repeated and pending responses useful

## 3. Frequency rule

Reduce travel as an action becomes more frequent. Keep the state change immediate: typing and key repeat still need visible results.

| Frequency | Treatment to try |
| --- | --- |
| Continuous or at keystroke rate: typing, result arrows or a command shortcut | Immediate content and state. Keep entrance animation out of the way of the first character. |
| Many times in a session: hover, row selection or toggles | Minimal feedback through the project’s short response role. |
| Occasional: a visible control opens or closes a surface | The appropriate entrance, expansion or exit role, with input usable from arrival. |
| Rare: a significant result or a deliberately expressive passage | The expression allowed by the project’s direction, with the same access and reduced form. |

Frequency describes the action, not a device class. Activating a visible button by Tab and Enter can be the same occasional action as clicking it. A command shortcut is often an immediate transition to typing. Follow the actual interaction, keep hover from changing surrounding geometry and shorten travel where less distance is covered. Material and light can give an immediately available shell character without a long entrance.

## 4. Loading and pending

Show that the specific request is pending as soon as its input is accepted. A separate delayed indicator needs a justified notice threshold from the product. Show measured progress only when the operation supplies it. Preserve the expected layout when its shape is known so arriving content does not displace the reader; remove the placeholder when the content arrives.

For a long wait, provide the supported cancel, retry or recovery path. Stop an activity indicator on completion, cancellation or failure. An indefinite spinner cannot stand in for an unknown result. Keep unrelated work usable and the prior committed choice visible when it still holds.

Check stale completion as well as waiting. A result from an earlier query must not clear or replace the current view’s pending state. Exercise rapid neighboring previews, repeated open and close, and typing before an older result returns. The newest intent needs feedback immediately even while its drawing settles.

## 5. Open questions

These choices belong to the consuming design system. Put an unresolved choice with that owner rather than inventing a local value.

| Question | What must settle it | Until then |
| --- | --- | --- |
| Should entrance and exit use asymmetric curves? | The accepted motion character and the shared curve roles | Keep the existing productive curve and exit duration; compare a new pair before adoption. |
| When should a separate pending indicator appear? | A notice threshold justified by the task and expected wait | Show the control’s pending state immediately; avoid a new unexplained timer. |
| Should a released gesture keep momentum? | A gesture requirement and a spring representation that preserves velocity | Use the supported settling behavior and name the missing momentum behavior; check each release for a jump or stall. |

## 6. Review questions

1. Are the trigger, changed drawing, interruption and reduced form clear for each role?
2. Do shared interactions use the system’s timing, with any intentional exception explained at its owner?
3. Is the destination usable from the start, even when motion is slowed for inspection?
4. Do the most frequent actions keep their feedback without accumulating travel or a queue?

## Worked example

A reader previews three neighboring results in a row. Open the first with the chosen entrance, then replace its contents promptly as the reader moves. Mark a loading view pending while keeping navigation usable. An older response must not overwrite the new preview. When work stops, stop its indicator and show the observed result. Keep this check in the component’s maintained test or existing review.

## Sources

- [Carbon Design System: Motion](https://carbondesignsystem.com/elements/motion/overview/): the productive versus expressive split, productive motion for task moments and microinteractions, and duration and easing tokens by role.
- [Fluent 2: Motion](https://fluent2.microsoft.design/motion): motion as functional and consistent, scaled to element size and distance, communicating change and relationship without making people wait.
- [Apple HIG: Motion](https://developer.apple.com/design/human-interface-guidelines/motion): brief, purposeful feedback motion that never delays people, and restraint for frequent interactions.
