+++
description = "When drawing or repairing a button, link, field, disclosure or choice."
uses = [{ scope = "local", resource_id = "controls" }]
see_also = [{ scope = "local", resource_id = "controls/press-and-commit" }, { scope = "local", resource_id = "animate/productive-motion" }, { scope = "local", resource_id = "access/web" }]

[guide]
exposure = "internal"
+++

# Keep a control understandable in every state

Use this when drawing or repairing a button, link, field, disclosure or choice. A state needs both a meaning and a carrier.

| State | Keep visible | Try this to expose a fault |
| --- | --- | --- |
| Rest | The type of control and what it does | Read the page on touch without hovering. |
| Hover | A cue the pointer can reach while other input paths still work | Leave the pointer over a control and tab to it. |
| Focus | Keyboard location on every background | Tab through the page in the supported color modes. |
| Pressed | Immediate acknowledgement of the physical input | Press, move outside, and release. The drawing responds, and the effect is cancelled. |
| Held choice | The current value, with a visible and announced cue | Remove color, then tab onto the chosen item. |
| Disabled | What is unavailable and what can make it available | Try finding the reason without attempting the action. |
| Pending | The operation that started and what else can still be used | Activate twice on a slow connection. |
| Error | What failed, what was kept and what can happen next | Force a failed response and check the retained field and focus. |
| Complete | What the operation actually did | Compare the wording with the result the service returned. |

For each control kind, use one shared component contract. A field has a visible label, filled and invalid states. A disclosure names its contents and exposes whether they are open. A toggle shows its current value at rest. A command trigger is a real control at every width. A card with several links is a container. A whole-card link has one destination, a clear accessible name and ordinary link behavior.

Focus, hover, a held choice and an error can occur together. Let each keep its carrier. Check actual input at normal size, including pointer, keyboard and touch where the product supports them. For a new component family, build a working state fixture. For an isolated repair, exercise the reachable states in the actual component.

## Keep the causes of state independent

Draw hover, focus, press, activation, held, pending, succeeded, failed and disabled for a control that can reach each one. Hover follows the pointer, focus follows input, pressed means the person is still acting, and a held state reports a choice that persists. Check the pairings that actually happen. A hovered and focused control still needs its ring. A chosen filter can be both pending and focused. A pending request must clear on cancellation or failure and leave the prior committed choice visible when that choice still holds.

Keep the same verb and object on a control, its confirmation and its outcome. Use the result to say what changed. Name an unmet condition beside an unavailable action. On a failed form submission, put focus on a linked summary or the single invalid field, show the corrective message next to its cause and keep other values. Let a destructive action name its target and consequence, and confirm before an irreversible change when its risk warrants it.

## Worked example: distinguish a held filter from focus

A filter chip uses a persistent check and an announced pressed state when it is on. Add a focus outline that appears around either chip. Move the pointer over the chip and tab to it. The label, persistent check and outline remain readable together. Compare in reduced motion and forced colors. The chip should keep its hit area when pressed.

See [Answer a press immediately and commit the intended action](press-and-commit.md), [Give control motion a clear job](../animate/productive-motion.md), and [Make the browser task accessible](../access/web.md).
