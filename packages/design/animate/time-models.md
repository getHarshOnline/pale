+++
description = "Use it to choose, for each moving part, what advances it before choosing a tool."
uses = [{ scope = "local", resource_id = "animate/interruption-reversal" }]
see_also = [{ scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "films" }, { scope = "local", resource_id = "access/web" }, { scope = "local", resource_id = "animate/pause-and-resume" }]

[guide]
exposure = "internal"
+++

# Choose what advances the motion

Decide what advances each moving part.

## Time models

Choose a row for each moving part before choosing a tool. A supplied reconstruction follows [Choose a visual direction from the actual brief](../art-direction.md); a generic restraint never silently changes its chosen time model.

| Model | What advances | What must remain true |
| --- | --- | --- |
| State transition | an input or state change plays toward a rest | retarget from the current value; settle when the change completes |
| Scrubbed sequence | browser scroll or pointer position selects the picture | same settled position, same picture; the edit belongs to [Compose a sequence people can follow](../films.md) |
| Authored linear film | the film’s timeline | use [Compose a sequence people can follow](../films.md) and the film production tools; the host owns playback controls and access |
| Ambient playback | elapsed time advances an ambient film or illustrative cycle | preserve its authored cadence; provide pause and a composed still; never gate text or navigation on playback |
| Evolving field | elapsed time changes a procedural field, optionally alongside input-driven scene or camera state | keep those clocks distinct; pause evolution without losing the current semantic state |
| Live indicator | a real process changes state | reflect only the process actually running; stop on completion or failure |

Input response may use a short settling motion using the project’s tokens toward the current target. It retargets on reversal, reaches the same settled picture, then stops; this catch-up is not autonomous playback. Name the response role and verify it in both directions. If that role is unavailable, tell the token owner which role is missing; never claim a weaker direct or zoned substitute preserves the behavior in the brief.

Give anything that moves on its own a job. An ambient film or field can carry atmosphere. The controls beside it can stay still. On a page, follow its design direction and access requirements ([Make the browser task accessible](../access/web.md)), a designed reduced form and [Pause work and resume from the same state](pause-and-resume.md). Verify the chosen clock in motion, including the pause, resume and input behavior that model supports; a still capture proves none of those behaviors. Offscreen suspension applies to live page work, not an offline film render.

## Worked example: keep a field and its camera separate

A field of particles evolves with elapsed time. Dragging rotates the camera. Pause stops the evolution at the visible field state. The person can still turn the camera and inspect that state. Resuming starts from the same field. If dragging instead advanced the field clock, the drag would change what the reader was trying to inspect.

## Sources

- [Apple WWDC 2018: Designing Fluid Interfaces](https://developer.apple.com/videos/play/wwdc2018/803/): constant redirection and interruption, velocity carried from gesture to animation, and rubber-banding at boundaries.
- [MDN: Web Animations API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Animations_API): controlling, reversing and retargeting running animations from script, and reading their current state.
