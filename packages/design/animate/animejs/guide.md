+++
description = "Use it for ordered steps, lines that draw after their endpoints appear or an SVG shape that morphs, with examples for anime.js 4.5.0."
uses = [{ scope = "local", resource_id = "animate" }]
see_also = [{ scope = "local", resource_id = "films" }, { scope = "local", resource_id = "animate/temporal-hierarchy" }, { scope = "local", resource_id = "animate/motion-toolchain" }, { scope = "local", resource_id = "animate/motion/guide" }, { scope = "local", resource_id = "figures" }, { scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "animate/animejs/tokens-reduced-motion" }, { scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "presentation" }, { scope = "local", resource_id = "surfaces" }, { scope = "local", resource_id = "animate/animejs/practical-tips" }, { scope = "local", resource_id = "animate/animejs/timelines-svg" }, { scope = "local", resource_id = "animate/interruption-reversal" }, { scope = "local", resource_id = "animate/reduced-motion" }, { scope = "local", resource_id = "animate/animejs/api-v4" }, { scope = "local", resource_id = "inspect" }, { scope = "local", resource_id = "user-task" }, { scope = "local", resource_id = "animate/time-models" }, { scope = "local", resource_id = "animate/pause-and-resume" }]
official_sources = [{ label = "anime.js documentation", url = "https://animejs.com/documentation/" }]

[guide]
exposure = "internal"

[stack]
+++

# Build a page animation with anime.js

Adapted from the Claude Skills Project’s MIT-licensed [animejs guide](https://github.com/freshtechbro/claudedesignskills/tree/1da73febff0c3e1dfefc07f8a5ef8f7d1dfdb6cd/.claude/skills/animejs). The licence travels with this guide.

Use anime.js for ordered steps, lines that draw after their endpoints appear, or an SVG shape that morphs. These examples use anime.js 4.5.0. Check the project’s installed version and import path before using the examples. Follow the project’s module and mount rules so the diagram stays readable when the script fails. If the task is a film, use [Compose a sequence people can follow](../../films.md) for the edit and its playback clock.

## Before writing

1. Read [Show what happens first](../temporal-hierarchy.md) and [Choose what advances the motion](../time-models.md). Decide what the order tells the reader. Keep a navigable list usable from the first frame.
2. Use the [Choose the lightest animation mechanism, “2. Choose by job”](../motion-toolchain.md#2-choose-by-job). CSS can handle a control state. Use [Build a page animation with Motion](../motion/guide.md) when gestures or springs are the main job. Choose how to [Compose and check a live visual region](../../figures.md) before choosing its timeline. Give each animated property one writer.
3. Read [Use tokens and reduced motion with anime.js](tokens-reduced-motion.md) for timing and easing. Pass the applicable values explicitly. The library has defaults, and its spring creates a different timing from the one the design supplies.
4. Use [Choose a visual direction from the actual brief](../../art-direction.md) and [Choose what material and light communicate](../../surfaces.md) to decide whether this figure should carry the expressive moment. Give it a still with the same information. A figure that depicts data needs the project’s actual data.
5. Read the [Diagnose an anime.js animation](practical-tips.md) for the part you are building, such as drawing lines, matching morph endpoints or checking a reversal.

## Core rules

- **Static first.** The still (raster or CSS composition) is complete without script. If the renderer disallows SVG in markup or assets, build its geometry in the mount slot. Set any undrawn start state on it and only then play ([Sequence steps and draw SVG with anime.js, “4. Geometry built in the mount slot”](timelines-svg.md#4-geometry-built-in-the-mount-slot)).
- **Interruptible and suspended.** Pause, reverse or seek on new input; never restart from zero on a second trigger. The engine pauses when the document is hidden by default; keep it that way and pause offscreen too ([Pause work and resume from the same state](../pause-and-resume.md)).
- **The reduced state has its own timeline.** Same labels and order, the finished geometry landing by opacity; with explicit final state and cleanup, including an immediate still when appropriate. Rules: [Design the reduced-motion alternative](../reduced-motion.md); the code is [Use tokens and reduced motion with anime.js, “3. Reduced motion through a scope”](tokens-reduced-motion.md#3-reduced-motion-through-a-scope).
- **Choose the clock before playback:** [Choose what advances the motion](../time-models.md).
- **Return to the still.** Clear the animation on a preference change. If the framework also removes the component, release observers and listeners at unmount.

## Read the reference when

| When the work is… | Read |
| --- | --- |
| Building a drawn figure, sequencing it or reviewing it, and checking it worked | [Diagnose an anime.js animation](practical-tips.md) |
| Imports, `animate()` parameters, tween values, keyframes, playback settings, callbacks, methods, `utils`, `engine`, `createScope`, `onScroll`; anime.js 3 forms and their 4 equivalents | [Find the anime.js 4 API for your task](api-v4.md) |
| `createTimeline()` positions and labels, `stagger()` options, building SVG in the mount slot, `svg.createDrawable`, `svg.morphTo` | [Sequence steps and draw SVG with anime.js](timelines-svg.md) |
| Converting tokens to milliseconds and eases, spring easings, reduced motion through a scope, revert | [Use tokens and reduced motion with anime.js](tokens-reduced-motion.md) |

## Quick detection

- `anime({ targets … })`, `anime.timeline`, `easing:`, `direction:`, `endDelay`, `begin:` or `complete:` → rewrite with the [Find the anime.js 4 API for your task](api-v4.md) table of anime.js 3 forms.
- A shared-role value copied into `duration`, `delay`, `ease`, `stagger()` or a position → read its token; authored scene timing stays with its scene ([Use tokens and reduced motion with anime.js](tokens-reduced-motion.md)).
- When following the example sampled-spring policy, `spring()` or `createSpring()` → use the emitted easing ([Use tokens and reduced motion with anime.js, “2. Springs: the emitted easing”](tokens-reduced-motion.md#2-springs-the-emitted-easing)).
- `loop: true` without a chosen time model → apply [Choose what advances the motion](../time-models.md).
- A stagger delaying access to a list, table or search results → immediate usable content ([Show what happens first, “Cascades”](../temporal-hierarchy.md#cascades)).
- `svg.morphTo` between elements of different types, or a morph whose shapes change the figure's meaning mid-flight → same element type, both ends sourced ([Sequence steps and draw SVG with anime.js, “5. Drawing and morphing”](timelines-svg.md#5-drawing-and-morphing)).
- If the renderer disallows SVG in markup and assets, build it in the mount slot at runtime.
- A `draw` start state (`'0 0'`) written in CSS or markup → set it with `utils.set` after mount.
- No `revert()` path or no reduced branch → [Use tokens and reduced motion with anime.js](tokens-reduced-motion.md).

## Worked example: draw a relation after both endpoints appear

```js
import { createTimeline, svg } from "animejs";

const edges = svg.createDrawable(path);
const timeline = createTimeline({ defaults: { ease }, autoplay: false })
  .label("endpoints")
  .add(nodes, { opacity: [0, 1], duration: stepMs }, "endpoints")
  .add(edges, { draw: ["0 0", "0 1"], duration: stepMs }, "<");
```

Here the component supplies `path`, `nodes`, `ease` and `stepMs`. The `"<"` position puts the line after the preceding child ends. With reduced motion, leave the finished line and endpoints visible in the still. When the moving version starts, wait until the component has mounted the geometry. Pause it offscreen if the component is still on the page. If the component goes away, revert the styles and remove the observer.

## Gotchas

- anime.js takes milliseconds; Motion takes seconds. `tweenMs` carries the unit in its name; the [Build a page animation with Motion](../motion/guide.md) helper is `tweenSeconds`.
- In a timeline, `'<'` is the end of the previous child; its start is `'<<'`.
- `engine.timeUnit` switches every animation on the page, not one island; leave it at the default.
- `spring()` replaces the animation's duration with its own settling time.

## After writing

Look at the running island in a real browser ([Choose a visual direction from the actual brief](../../art-direction.md)). Feel-check it with [Inspect an interface in its rendered context](../../inspect.md): slow it with `playbackRate` or the browser's animation inspector, seek it frame by frame, and watch it on a real device. Then judge it (does the order teach, or perform?) and verify with [Check a task from entry to result](../../user-task.md): scripts off, reduced motion on, tab hidden, island scrolled away mid-sequence, repeated triggers. Compare expressive moments with the intended character, including missed expression as well as excess. Put results and any unrun checks in the component’s existing review.

## Before handing off

- [ ] Does the running component show the intended change with the supported API and clock?
- [ ] Are content, focus and controls usable through interruption, failure and reduced motion?
- [ ] Do the observed result and any unrun checks appear in the existing review?
