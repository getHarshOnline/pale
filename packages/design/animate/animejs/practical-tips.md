+++
description = "Use it when an island that draws or sequences with anime.js 4 misbehaves, to find the check and the reference for the fault."
uses = [{ scope = "local", resource_id = "animate/animejs/guide" }]
see_also = [{ scope = "local", resource_id = "animate/animejs/api-v4" }, { scope = "local", resource_id = "animate/animejs/timelines-svg" }, { scope = "local", resource_id = "animate/animejs/tokens-reduced-motion" }, { scope = "local", resource_id = "animate" }, { scope = "local", resource_id = "figures" }, { scope = "local", resource_id = "animate/temporal-hierarchy" }, { scope = "local", resource_id = "web" }, { scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "animate/interruption-reversal" }, { scope = "local", resource_id = "inspect" }, { scope = "local", resource_id = "animate/animejs/playback-and-scopes" }, { scope = "local", resource_id = "animate/retarget-controls" }, { scope = "local", resource_id = "animate/pause-and-resume" }, { scope = "local", resource_id = "animate/read-tokens" }]

[guide]
exposure = "internal"
+++

# Diagnose an anime.js animation

Field practice for an island that draws or sequences with anime.js 4. Tips are the entry point; each ends with a check and a **Read:** line to the reference that holds the rule: [Find the anime.js 4 API for your task](api-v4.md), [Sequence steps and draw SVG with anime.js](timelines-svg.md) or [Use tokens and reduced motion with anime.js](tokens-reduced-motion.md). Whether order is worth showing is [Design motion that helps people use the interface](../../animate.md); what may be drawn is [Compose and check a live visual region](../../figures.md).

## Contents

- Building a drawn figure
- Sequencing
- Interrupting and suspending
- Reviewing the motion

## Building a drawn figure

### Build the SVG in the mount slot, after the still

When the renderer disallows SVG in templates or assets, a figure drawn from a template or file never renders. Keep the raster or CSS still in the fallback; at mount, create the SVG with `createElementNS` from the same sourced data, set its undrawn state with `utils.set`, and let the still yield only when the figure is ready. **Check:** with scripts off the still shows complete; with scripts on the figure replaces it without a blank frame. **Read:** [Sequence steps and draw SVG with anime.js, “4. Geometry built in the mount slot”](timelines-svg.md#4-geometry-built-in-the-mount-slot); the project’s supported asset formats.

### Draw a relation only after both ends exist

A line that draws before the things it connects are visible explains nothing. Place the nodes at a label, then the edge after it, and draw with `svg.createDrawable` from `'0 0'` to `'0 1'`. **Check:** step the timeline frame by frame; no edge starts drawing before both of its ends are visible. **Read:** [Sequence steps and draw SVG with anime.js, “5. Drawing and morphing”](timelines-svg.md#5-drawing-and-morphing); [Show what happens first, “Order encodes causality”](../temporal-hierarchy.md#order-encodes-causality).

### Keep drawn strokes scaling with the figure

A stroke with `vector-effect: non-scaling-stroke` forces the drawable to recompute its scale on every tick, which slows the drawing. Leave strokes scaling, and size the figure so the stroke width reads at the narrowest width. **Check:** in source, no drawable path carries `non-scaling-stroke`; at the narrowest width the drawn line stays legible. **Read:** [Sequence steps and draw SVG with anime.js, “4. Geometry built in the mount slot”](timelines-svg.md#4-geometry-built-in-the-mount-slot); [Carry a design through the web build](../../web.md).

### Match morph ends and read the middle frame

`svg.morphTo` animates `d` on paths and `points` on polylines and polygons, adding points by its precision when the shapes differ; an intermediate frame can suggest a form the data never had. Morph between two built elements of the same type, both true sourced forms, and keep the default precision unless the middle frame kinks. **Check:** seek to the halfway point of the morph; the frame reads as passage between the two forms, not as a third claim. **Read:** [Sequence steps and draw SVG with anime.js, “5. Drawing and morphing”](timelines-svg.md#5-drawing-and-morphing); [Compose and check a live visual region](../../figures.md).

## Sequencing

### Place children with labels, not accumulated offsets

Name temporal relations with `label()` ("ends", "edge", "result") so related actions retain their alignment when timing changes; `'<'` is the previous child's end and `'<<'` its start. Shared role timing reads tokens. Authored scene offsets and durations follow the action in the scene treatment. **Check:** changing a role token preserves interface relations; changing a scene action preserves its intended lead, reactions, settle and read window. **Read:** [Sequence steps and draw SVG with anime.js, “2. Time positions”](timelines-svg.md#2-time-positions); [Read timing and spring roles from the system, “1. Which values come from tokens”](../read-tokens.md#1-which-values-come-from-tokens).

### Stagger in reading order

For a semantic sequence, stagger in the order the content is read or along a meaningful grid axis. A selected atmospheric field may use another distribution; random order is not a default for readable content. Use the shared timing role where applicable. **Check:** slowed, the arrival fulfils the selected temporal purpose. In a page, navigation stays usable; in a film, inspect the authored stagger at independent seek positions. **Read:** [Sequence steps and draw SVG with anime.js, “3. `stagger()`”](timelines-svg.md#3-stagger); [Show what happens first, “Cascades”](../temporal-hierarchy.md#cascades).

### Convert tokens in milliseconds, once

anime.js takes milliseconds while Motion takes seconds; a helper copied from the wrong engine runs a thousand times too fast or too slow. Use `tweenMs(el, role)` and the emitted spring easing through `springPreset(el, prefix)`, read from the island element at construction. **Check:** the running sequence's step length matches the role's token as read in the browser's computed style. **Read:** [Use tokens and reduced motion with anime.js, “1. Reading and converting tokens”](tokens-reduced-motion.md#1-reading-and-converting-tokens).

## Interrupting and suspending

### Let the default composition retarget

With the default `'replace'` composition, a new animation on a property cancels the running one and starts from the value on screen; `'none'` lets the old tween keep running beside it. On new input, animate toward the new target and keep the default; reserve `'blend'` for additive transforms, since it takes no keyframes, colours or looped playback. **Check:** trigger the change and reverse it halfway, repeatedly; the element turns around from where it is; in source, no `composition: 'none'` sits on a reversible state. **Read:** [Control anime.js playback and scopes, “3. Playback settings, callbacks and methods”](playback-and-scopes.md#3-playback-settings-callbacks-and-methods); [Reverse or retarget a control in flight, “Rules”](../retarget-controls.md#rules).

### Pause offscreen; leave the hidden-tab pause on

The engine pauses when the document is hidden, but a figure scrolled out of view keeps running. Pause the timeline from an Intersection Observer when the island leaves the viewport and resume it on return; leave `engine.pauseOnDocumentHidden` at its default. **Check:** scroll the island away mid-sequence and back, then switch tabs and back; each time it resumes from where it was, without replaying. **Read:** [Use tokens and reduced motion with anime.js, “4. Revert, never unmount”](tokens-reduced-motion.md#4-revert-never-unmount); [Pause work and resume from the same state](../pause-and-resume.md).

### Revert to the still, never tear down

An island that removes itself on failure leaves a hole in the page. Build every animation inside a `createScope` and call `scope.revert()` on failure, on a change to reduced motion, or when the island's own logic replaces the animation. **Check:** throw inside the mount on purpose; the still returns and no half-drawn figure remains. **Read:** [Use tokens and reduced motion with anime.js, “4. Revert, never unmount”](tokens-reduced-motion.md#4-revert-never-unmount); the project’s module loading and mount guide.

## Reviewing the motion

### Slow it down with `playbackRate`

A sequence that reads well at speed can still draw an edge early or overlap two steps. During review, set `playbackRate` below one on the timeline (or `engine.speed` for every animation on the page, or the browser's animation inspector) and `seek()` through it; never ship a changed rate. **Check:** the slowed run shows each step arriving in its order and settling before the next depends on it; in source, no rate or engine speed is changed. **Read:** [Control anime.js playback and scopes, “3. Playback settings, callbacks and methods”](playback-and-scopes.md#3-playback-settings-callbacks-and-methods); [Inspect an interface in its rendered context](../../inspect.md).

## Worked example

An interactive diagram draws an edge after both endpoint nodes settle. Mount the SVG after the initial still. Keep the destination points named and use timeline labels for the two arrivals and edge start. Open the diagram with the keyboard, interrupt the second arrival, and verify the edge follows the new endpoints. Move the figure out of view and back. Confirm its saved state resumes and that failure of the mount leaves the still readable.

## Sources

- [anime.js documentation: Time position](https://animejs.com/documentation/timeline/time-position): `'<'` versus `'<<'` and label positions.
- [anime.js documentation: Timeline](https://animejs.com/documentation/timeline): `label()`, `defaults` inherited by children, and `sync()`.
- [anime.js documentation: Stagger](https://animejs.com/documentation/utilities/stagger): `from`, `grid`, `axis` and jitter.
- [anime.js documentation: createDrawable](https://animejs.com/documentation/svg/createdrawable): the `draw` syntax and the per-tick cost of `non-scaling-stroke`.
- [anime.js documentation: morphTo](https://animejs.com/documentation/svg/morphto): matching element types and the precision that adds points.
- [anime.js documentation: Composition](https://animejs.com/documentation/animation/tween-parameters/composition): `'replace'` as the default, `'none'` and the limits of `'blend'`.
- [anime.js documentation: Tween value types](https://animejs.com/documentation/animation/tween-value-types): a tween given only its target value starting from the current value.
- [anime.js documentation: Engine](https://animejs.com/documentation/engine): `pauseOnDocumentHidden` and the engine's pause and resume.
- [anime.js documentation: engine.speed](https://animejs.com/documentation/engine/engine-parameters/speed): slowing every animation the engine manages.
- [anime.js documentation: playbackRate](https://animejs.com/documentation/animation/animation-playback-settings/playbackrate): the per-animation speed multiplier and changing it while running.
- [anime.js documentation: Scope](https://animejs.com/documentation/scope): `revert()` and media-query re-runs.
- [anime.js documentation: Animation](https://animejs.com/documentation/animation): `seek()` and the playback methods.
