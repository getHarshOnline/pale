+++
description = "Decide what a movement communicates, what drives it, how it responds to input and how it reads with less motion."
uses = [{ scope = "local", resource_id = "design" }]
see_also = [{ scope = "local", resource_id = "animate/interruption-reversal" }, { scope = "local", resource_id = "films" }, { scope = "local", resource_id = "animate/productive-motion" }, { scope = "local", resource_id = "animate/temporal-hierarchy" }, { scope = "local", resource_id = "animate/expressive-motion" }, { scope = "local", resource_id = "animate/reduced-motion" }, { scope = "local", resource_id = "animate/motion-toolchain" }, { scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "animate/practical-tips" }, { scope = "local", resource_id = "animate/time-models" }, { scope = "local", resource_id = "animate/frequency-and-pending" }]

[guide]
+++

# Design motion that helps people use the interface

Use this page when an interface opens, moves, changes state, reveals part of a story or plays a moving figure. Start with the reference the project is trying to realise. Decide what the movement communicates, what advances it, what happens when the person changes their mind and how the same idea reads with reduced motion. Carry those decisions into the interface and watch it run.

## Choose what advances time

| What you are making | What advances it | Where to work it out |
| --- | --- | --- |
| A button, drawer, popover or other change of state | The latest input or result | [Keep motion responsive when its input changes](animate/interruption-reversal.md) |
| A sequence the reader scrubs | Scroll or pointer position | [Choose what advances the motion](animate/time-models.md) |
| A film or ambient passage | A timeline or clock | [Compose a sequence people can follow](films.md) and [Choose what advances the motion](animate/time-models.md) |
| A field that evolves while someone can move its view | Separate clocks for the field and the view | [Choose what advances the motion](animate/time-models.md) |
| A progress indicator | The work it reports | [Keep repeated and pending responses useful, “4. Loading and pending”](animate/frequency-and-pending.md#4-loading-and-pending) |

At the same scroll position, a scrubbed sequence should settle on the same picture from either direction. A film plays its edit and offers play, pause and seek. A drawer follows the latest input and settles at the state the person asked for. Keep the product state in the component or application that owns it. The animation draws the change. Focus, typing and navigation can work on the first frame.

## Choose the treatment

| If you are doing this | Read this |
| --- | --- |
| Timing press, focus, preview, disclosure, results or dismissal | [Give control motion a clear job](animate/productive-motion.md) |
| Ordering a sequence, a handoff between views or a cascade | [Show what happens first](animate/temporal-hierarchy.md) |
| Giving a material, light change or field an expressive moment | [Give expressive motion a purpose](animate/expressive-motion.md) |
| Showing the same state or relationship with less motion | [Design the reduced-motion alternative](animate/reduced-motion.md) |
| Choosing CSS, a browser API or a library | [Choose the lightest animation mechanism](animate/motion-toolchain.md) |
| Reading tokens, setting the origin, or tracing frame costs | [Implement motion from the shared design](animate/motion-implementation.md) |
| Finding a jump, delay, wrong curve or expensive effect | [Diagnose motion in the browser](animate/practical-tips.md) |

Productive motion answers an input or shows its result. Use the project's timing and curve roles, and cut travel as frequency rises. A command shortcut should open at typing speed. A press can change the drawing without moving the hit area. A dismissal hands control back at the start of its exit.

Expressive motion carries a relation, rhythm, material or atmosphere. Look for where it can help the subject, then use the project's direction to decide how much that part of the page can carry. A sequence can settle into a still when the person asks for less motion. Design that still and check that it keeps the order, relationship and emphasis of the moving version.

For a drawer, try the open and close states as a CSS transition before adding a library. On open, put focus inside at once. On close, return focus to the trigger. Press Escape halfway through the entrance and watch the drawer turn from its current position. Try the same interaction with reduced motion, where the open and closed states can appear in place.

## Before handing off

- [ ] Does the movement communicate its intended change on the chosen clock?
- [ ] Do timing roles preserve immediate focus, input and navigation?
- [ ] Do reversal, repeated input, pause or seeking behave as the selected clock requires?
- [ ] Does reduced motion reach a clear, usable state in the running interface?
