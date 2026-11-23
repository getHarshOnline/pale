+++
description = "Use it for the Motion engine's API mechanics in an island, after the design has decided what moves, why and how fast."
uses = [{ scope = "local", resource_id = "animate" }]
see_also = [{ scope = "local", resource_id = "films" }, { scope = "local", resource_id = "animate/productive-motion" }, { scope = "local", resource_id = "animate/motion-toolchain" }, { scope = "local", resource_id = "web" }, { scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "animate/motion/token-presets" }, { scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "presentation" }, { scope = "local", resource_id = "figures" }, { scope = "local", resource_id = "surfaces" }, { scope = "local", resource_id = "animate/motion/practical-tips" }, { scope = "local", resource_id = "animate/interruption-reversal" }, { scope = "local", resource_id = "animate/motion/islands-api" }, { scope = "local", resource_id = "animate/reduced-motion" }, { scope = "local", resource_id = "controls" }, { scope = "local", resource_id = "animate/scroll/guide" }, { scope = "local", resource_id = "animate/animejs/guide" }, { scope = "local", resource_id = "inspect" }, { scope = "local", resource_id = "user-task" }, { scope = "local", resource_id = "animate/retarget-controls" }, { scope = "local", resource_id = "animate/time-models" }, { scope = "local", resource_id = "animate/feedback-roles" }, { scope = "local", resource_id = "animate/read-tokens" }]
official_sources = [{ label = "Motion documentation", url = "https://motion.dev/docs/" }]

[guide]
exposure = "internal"

[stack]
+++

# Build a page animation with Motion

Adapted from the Claude Skills Project’s MIT-licensed [motion-framer guide](https://github.com/freshtechbro/claudedesignskills/tree/1da73febff0c3e1dfefc07f8a5ef8f7d1dfdb6cd/.claude/skills/motion-framer). The licence travels with this guide.

Motion (`motion` package) supplies the island engine; the design system supplies every decision. This skill holds API mechanics only: what moves, why, in what order and how fast is judged before any call is written, and the code runs that decision and nothing more. Engine constraints (vendoring, module profile, mount contract, static fallback) are the project’s theme manual scripts and islands; this skill never restates them.

For film production, use [Compose a sequence people can follow](../../films.md) and the HyperFrames instructions for the project’s version.

## Before writing

1. **Whether it moves and its role** → [Choose feedback roles and timing, “1. The role contract”](../feedback-roles.md#1-the-role-contract) (and expressive motion, "Earned by meaning" for an expressive moment); note the geometry or state change it explains and its role (feedback, preview, enter, leave, result, sequence, expressive). Without a named role there is no call.
2. **The mechanism** → [Choose the lightest animation mechanism, “2. Choose by job”](../motion-toolchain.md#2-choose-by-job); note Motion, or the lighter step that wins: a control state is CSS ([Carry a design through the web build](../../web.md)), one property with no spring may use plain WAAPI. Honour an explicit or installed engine when its pinned capabilities fit; Motion supports SVG drawing, and anime.js is a useful default for complex morphs. Coordinate ownership so each animated property has one writer.
3. **The tokens** → [Read timing and spring roles from the system, “2. How islands take their values and the spring policy”](../read-tokens.md#2-how-islands-take-their-values-and-the-spring-policy); note the token ids for the role, read at runtime as in [Convert design tokens to Motion options](token-presets.md). Shared role values come from tokens; normalised progress, indices and authored geometry are not design-token gaps.
4. **Whether the page may peak here** → [Choose a visual direction from the actual brief](../../art-direction.md) and [Preserve meaning across presentations](../../presentation.md); note whether `world.motion.*` may drive this moment. Productive roles never read them.
5. **The island itself** → [Compose and check a live visual region](../../figures.md) (keep the effect in its region and make the still complete) and [Choose what material and light communicate](../../surfaces.md) for any light it animates; note the still and the bounds.
6. **The common moves** → [Diagnose a Motion animation](practical-tips.md); note the tip for the situation (starting, moving, interrupting, reviewing) and run its **Check:** once written.

## Core rules

- **Content is final in the markup.** Motion never supplies an element's resting state; an initial hidden or offset state is set by the island after it mounts and decides to animate ([Choose the lightest animation mechanism, “1. Rules for every animation tool”](../motion-toolchain.md#1-rules-for-every-animation-tool)).
- **Interruptible.** A new input retargets from the current value; nothing queues or restarts ([Reverse or retarget a control in flight, “Rules”](../retarget-controls.md#rules)). Mechanics in [Use the Motion JavaScript API in a component, “5. Playback controls and interruption”](islands-api.md#5-playback-controls-and-interruption).
- **Measure the chosen mechanism.** Prefer full transforms and opacity for ordinary movement. Intentional layout morphs and independent axes follow [Choose the lightest animation mechanism](../motion-toolchain.md) and must meet the target frame budget ([Use the Motion JavaScript API in a component, “2. `animate()`”](islands-api.md#2-animate)).
- **The reduced state is designed.** Each role plays its own reduced keyframes and preset, including an immediate final form where appropriate. Rules: [Design the reduced-motion alternative](../reduced-motion.md); the code is [Convert design tokens to Motion options, “3. Reduced motion”](token-presets.md#3-reduced-motion).
- **Suspend unseen work.** Start work with `inView()` and stop it when the island leaves; playback follows [Choose what advances the motion](../time-models.md).
- **Keep the control’s states.** Use CSS for a standalone control. Use `hover()` and `press()` when the control is part of an existing Motion sequence. See [Carry a design through the web build](../../web.md) and [Design controls that work through the whole task](../../controls.md).
- **Return to the still.** Clear the animation on a preference change. If the framework also removes the component, release observers and listeners at unmount.

## Read the reference when

| When the work is… | Read |
| --- | --- |
| Starting, moving, interrupting or reviewing a Motion island, and checking it worked | [Diagnose a Motion animation](practical-tips.md) |
| Choosing mini or hybrid, WAAPI without an engine, `animate()` options, sequences, `stagger()`, springs, `scroll()`, `inView()`, playback and interruption, outdated forms | [Use the Motion JavaScript API in a component](islands-api.md) |
| Reading motion tokens at runtime, converting them to Motion options in seconds, the role-to-token map, the motion-preference contract in code | [Convert design tokens to Motion options](token-presets.md) |

## Quick detection

- A shared interface-role or world-character value written as a literal → read its token; authored scene timing follows [Compose and check a live visual region](../../figures.md) ([Convert design tokens to Motion options](token-presets.md)).
- `x`, `y` or `scale` animated for plain movement → one `transform` string ([Use the Motion JavaScript API in a component, “2. `animate()`”](islands-api.md#2-animate)).
- A CSS variable animated on `:root` or a large ancestor → the consuming element ([Carry a design through the web build](../../web.md)).
- If the project requires vendored modules, an unbundled package or URL import → its declared local module (the project’s module loading and mount guide).
- An element hidden or offset in CSS or template before the island mounts → move the initial state into the island.
- `repeat: Infinity` without a clock you chose → apply [Choose what advances the motion](../time-models.md).
- A reduced path that zeroes every duration, or none → [Convert design tokens to Motion options, “3. Reduced motion”](token-presets.md#3-reduced-motion).
- A library added solely for standalone hover, press or focus → prefer CSS on tokens; preserve a selected coordinated engine ([Choose the lightest animation mechanism](../motion-toolchain.md)).
- `scroll()` on reading content, or an entrance outside the accepted direction → remove; reveals are judged in [Build a scroll-linked effect or one-time arrival](../scroll/guide.md).
- Competing writers on one property → assign ownership ([Choose the lightest animation mechanism, “2. Choose by job”](../motion-toolchain.md#2-choose-by-job)).

## Gotchas

- Motion takes seconds; token durations arrive in milliseconds or seconds. `tweenSeconds` carries the unit in its name; the [Build a page animation with anime.js](../animejs/guide.md) helper is `tweenMs`.
- Duration-based springs ignore velocity, an explicit `velocity` included; a released gesture that must keep momentum needs a project-supported physics model and shared parameters.
- `cancel()` snaps back to the start; interruption retargets with a new `animate()` instead.
- The mini build animates CSS variables only when they are registered with `@property`.

## After writing

Look at the running island in a real browser ([Choose a visual direction from the actual brief](../../art-direction.md)). Feel-check it with [Inspect an interface in its rendered context](../../inspect.md): slow it through the returned controls' `speed` or the browser's animation inspector, step it frame by frame, and watch it on a real device. Then judge it (does the motion carry meaning, or perform?) and verify with [Check a task from entry to result](../../user-task.md): scripts off, reduced motion on, rapid repeated input, tab hidden, island scrolled away and back. Compare the expressive moments with the brief and keep the observed results in the component’s existing review.

## Worked example

Animate one panel from its current transform and keep the returned playback controls. Reverse the request halfway through, check that the logical state and focus change at once, then verify the drawing reaches the new target. Use the playback and scopes supported by the installed Motion version.

## Before handing off

- [ ] Does the running component show the intended change with the supported API and clock?
- [ ] Are content, focus and controls usable through interruption, failure and reduced motion?
- [ ] Do the observed result and any unrun checks appear in the existing review?
