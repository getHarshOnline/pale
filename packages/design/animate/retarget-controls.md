+++
description = "Use it to handle a new input before the old transition finishes, for input-driven transitions and controls."
uses = [{ scope = "local", resource_id = "animate/interruption-reversal" }]
see_also = [{ scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "films" }, { scope = "local", resource_id = "animate/read-tokens" }]

[guide]
exposure = "internal"
+++

# Reverse or retarget a control in flight

Handle a new input before the old transition finishes.

## Rules

These rules govern input-driven transitions and their controls. They do not require an authored film, ambient loop or evolving field to reverse its story when its playback control changes.

1. **Retarget from the current state.** A new input mid-motion redirects the running motion from its present position, opacity and scale. Detect a jump to the start or end state, or a restart from zero, when input changes mid-flight.
2. **Latest input wins.** Rapid repeated input never queues animations. Detect a surface that opens, closes and reopens in sequence after the user already stopped.
3. **Reversible.** Opening then immediately closing returns smoothly along the same path. Detect an exit that cannot begin until the entry completes.
4. **Stable anchors.** The trigger, the caller's scroll position and the page under an overlay do not move while motion runs. Detect layout shift in anything the user was not acting on.
5. **Input is never blocked.** Clicks, keys and pointer movement are processed during motion. Detect disabled pointer events or ignored keys for the duration of an animation.
6. **Change the state on input.** The logical state changes immediately on input; the motion catches up. Detect code that waits for an animation-end event before updating state, focus or data.

The component owns selection, ordering and submission. Keep one stable item ID through a move or a presence change, so the animation follows the same object. A pressed drawing can change without moving the hit area or submitting the abandoned press. Check the intermediate frame as well as the endpoint. If a reference calls for a continuous response, keep that response. Tell the token or component owner what is missing when the implementation can only offer a few steps.

## Choosing mechanisms

- **Transitions retarget; keyframes restart.** A CSS transition interrupted mid-flight starts its new transition from the current value, and a reversal shortens its duration in proportion; it carries position, not velocity. A CSS keyframe animation replaced mid-flight restarts from its first frame. So a state that can be reversed (open and close, hover and leave, press and release) is a transition between states; keyframes are for one-way sequences that are never reversed mid-flight.
- Prefer mechanisms that can be retargeted mid-flight (transitions between states, WAAPI animations reversed or updated from their current time, physically based interpolation) over fixed sequences that restart from their first frame.
- A direct-manipulation gesture follows the pointer. If its design calls for momentum, carry the release velocity toward the chosen resting state and define when a flick commits. Duration-based spring presets may discard velocity; use the project’s supported physics model and shared parameters, or put the missing choice with its owner ([Read timing and spring roles from the system, “2. How islands take their values and the spring policy”](read-tokens.md#2-how-islands-take-their-values-and-the-spring-policy)).
- Where the gesture permits overscroll, rising resistance can communicate its boundary; preserve any task that needs a hard stop.
- A scrubbed sequence reaches the same settled frame at the same position from either direction. Any input response retargets from its current picture; its target still comes from the reader’s position ([Compose a sequence people can follow](../films.md)).

## Worked example: reverse a drawer

| Input | State and focus | What the drawing does |
| --- | --- | --- |
| Press the trigger | Open, focus inside | Move from the closed position toward the open one. |
| Press Escape halfway through | Closed, focus back at the trigger | Turn from the current transform and opacity. Keep the trigger and page still. |
| Open again while the drawer is leaving | Open, focus inside | Head back toward open. Drop the previous close animation. |
| Turn reduced motion on during the movement | The latest state and focus stay in place | Settle there and remove the travel. |

Slow playback to check the turn, then press the trigger five times at normal speed. The last input decides where it rests. A drawer that opens after the person stopped because it kept a queue has failed even if each separate transition looked smooth. A CSS transition can preserve position during reversal. For a drag-and-release gesture, check velocity separately before promising that a flick will carry through to the resting state.

## Exercise interruption and repeated input

Reverse each input-driven transition halfway through and check that it retargets without a jump or restart. Review authored playback and loops against the chosen clock, including pause and resume. Press the trigger rapidly five times: the final state must match the last input without a queue. Inspect anything still animating and establish what is changing.

## Sources

- [Apple WWDC 2018: Designing Fluid Interfaces](https://developer.apple.com/videos/play/wwdc2018/803/): constant redirection and interruption, velocity carried from gesture to animation, and rubber-banding at boundaries.
- [CSS Transitions Level 1](https://www.w3.org/TR/css-transitions-1/): how a transition interrupted mid-flight reverses from its current value and shortens its duration.
- [MDN: Web Animations API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Animations_API): controlling, reversing and retargeting running animations from script, and reading their current state.
- [Motion: animate](https://motion.dev/docs/animate): island playback controls (stop, cancel, complete, time, speed) for retargeting and reversing from the current state.
