+++
description = "Use it to suspend work when it goes offscreen or the reader pauses it, and to resume from the same state."
uses = [{ scope = "local", resource_id = "animate/interruption-reversal" }]
see_also = [{ scope = "local", resource_id = "surfaces" }, { scope = "local", resource_id = "animate/time-models" }]

[guide]
exposure = "internal"
+++

# Pause work and resume from the same state

Suspend work when it goes offscreen or the reader pauses it.

## No perpetual loops

- An animation runs only while something is actually changing. Detect any infinite loop whose purpose is attention (pulsing dots, breathing buttons, shimmering text, idle floats); remove it.
- A live process may show continuous activity only while it runs and only if the indicator reflects that process; it stops when the process stops or fails.
- State-bearing light holds steadily while state holds; it does not throb (see [Choose what material and light communicate](../surfaces.md)).
- Judge repetition by the [Choose what advances the motion](time-models.md), not by the presence of a loop alone. A moving field or film does not justify pulsing controls beside it; each motion retains its own purpose and access contract.

## Suspend offscreen and hidden

- Suspend visual animation and generative drawing when their region is offscreen, covered or in a hidden tab. Resume according to the selected clock: continue a paused sequence, or draw the current state of a live data source. This does not stop required background computation or an explicitly requested offline render.
- Detect animation or rendering work continuing while its region is not visible.
- On return, show the appropriate current state immediately and preserve an explicit user pause; do not replay a one-time entrance.

## Worked example: return to a figure after scrolling

A figure has a continuous field and a button that changes the view. Pause the field when its observer reports it has left the viewport. Keep the current view and the field’s time. When the reader returns, render that state first and resume the field only if it was playing. A reader who paused it explicitly should still see the paused frame.

## Sources

- [MDN: Page Visibility API](https://developer.mozilla.org/en-US/docs/Web/API/Page_Visibility_API): detecting hidden tabs so animation and rendering pause.
- [MDN: Intersection Observer API](https://developer.mozilla.org/en-US/docs/Web/API/Intersection_Observer_API): detecting regions leaving the viewport so offscreen motion and generative work suspend.
