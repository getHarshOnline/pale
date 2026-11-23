+++
description = "Use it to design the reduced state beside the full one, removing unwanted travel while state, order and emphasis stay clear."
uses = [{ scope = "local", resource_id = "animate" }]
see_also = [{ scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "access/web" }, { scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "surfaces" }, { scope = "local", resource_id = "films" }, { scope = "local", resource_id = "animate/productive-motion" }, { scope = "local", resource_id = "animate/preserve-transform-origin" }, { scope = "local", resource_id = "animate/feedback-roles" }]

[guide]
exposure = "internal"
+++

# Design the reduced-motion alternative

Design the reduced state alongside the full one. Remove unwanted travel while keeping state, order, relationships and visual emphasis clear. [Make the browser task accessible](../access/web.md) covers inputs, safety and tests; [Build the origin and reduced form into the control, “4. Reduced motion in code”](preserve-transform-origin.md#4-reduced-motion-in-code) covers reading the preference and responding to changes. The token names below illustrate one theme’s choices. Use the consuming project’s actual roles, modes and preference controls.

## Translate, don't delete

Movement, zoom, parallax, moving depth, cascades, scroll-linked travel and pointer-following movement need alternatives. Keep the information in a composed state rather than merely making the original movement faster or slower. The examples below show how each purpose can survive:

| Motion carries | Reduced form |
| --- | --- |
| A productive role (feedback, preview, enter, leave, result, sequence) | The role's own `motion.reduced.<role>`: an immediate change, or a brief opacity step in place, on `motion.reduced.easing` |
| A world's expressive moment | The same moment without travel: light changes or material clears in place over `motion.reduced.expressive` scaled by `world.motion.reduced.expressive-scale`, on `world.motion.reduced.expressive-easing`, settling on the composed still of the resolved state |
| State-bearing light arriving and releasing | Light that changes in place: present at once, released over `world.motion.reduced.decay` ([Choose what material and light communicate](../surfaces.md)) |
| A moving generative field, or shader or simulation evolution | A representative still carrying the same structure and meaning |
| An animated world motif | The motif's geometric, still form |
| A cinematic sequence scrubbed by scroll | Its storyboard: each beat's resting still, stepping in place at the holds, the words scrolling as in full motion ([Compose a sequence people can follow](../films.md)) |
| A relationship shown by motion (this came from that) | Colour, geometry and a label that state the relationship |
| A cascade where order was information | Order visible in structure: numbering, position, grouping, all present together |
| Press, commit and state change | An immediate change of form, emphasis or light (`motion.reduced.press-scale` holds the size) |

The composed still is the test: the reduced state never leaves an element at an intermediate or hidden starting state. Detect content that stays invisible or offset because its entrance did not run.

## The peak without travel

If motion carries a page’s strongest moment, preserve that emphasis through composition, tone, light or material. A brief in-place change can retain the identity’s brisk, latched or damped character where it remains comfortable; a composed still may be the better choice. Compare the reduced capture with the brief: the important region should remain distinct without requiring travel.

## Rules

1. **Choose each alternative.** A control may change immediately, another may use a brief opacity step, and a figure may show its finished geometry. Put reusable choices in the project’s reduced-motion roles. Removing durations globally is insufficient when start styles, callbacks or travel remain.
2. **Keep the information.** Give a relationship, sequence or result a visible form that does not depend on movement alone.
3. **Judge the result in every supported mode.** Where the product supports light and dark modes, inspect both reduced versions beside their full-motion versions as finished compositions. Use the [Make the browser task accessible](../access/web.md) and keep their results in the existing review.
4. **Check platform effects.** Test the view transitions and smooth scrolling the project uses; do not assume they apply the preference. Compare full and reduced states with browser emulation or an existing preference control, without adding a product control solely for a review.

If the page uses an animation-end event to release a lock or finish cleanup, check that cleanup on the reduced path too. Slower travel can make a demonstration easier to inspect. The reduced version serves a different job. Keep film playback and autoplay controls available as the host requires.

## Worked example: preserve the peak without travel

A four-step diagram draws a line after both endpoints appear. In the reduced version show the four steps in their final order, number them, and put the line and its label in place at once. A brief change in tone can still call out the final relation. Open the page with reduced motion already set and switch it on while the line is drawing. Both routes should show the final relation, with no endpoint still hidden and no change to keyboard order.

## Check the reduced state

Exercise every state change in the reduced version. Each must remain distinguishable and reach its final state, with no element stranded at its animation start. Check that each alternative preserves the information, including when it should appear immediately. Read the complete page again: its intended emphasis and character should remain comfortable.

## Sources

- [MDN: prefers-reduced-motion](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion): the media feature, its values and how operating systems expose the preference.
- [Media Queries Level 5: prefers-reduced-motion](https://www.w3.org/TR/mediaqueries-5/): the normative definition, including that reduce means removing or replacing motion rather than slowing it.
- [WCAG 2.2 Understanding 2.3.3 Animation from Interactions](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions): which interaction-triggered motion must be disableable, the harm the reduced forms design around.
- [WCAG 2.2 Technique C39](https://www.w3.org/WAI/WCAG22/Techniques/css/C39): the reduced-motion query swapping motion for its reduced form, as each table row does.
- [Fluent 2: Motion](https://fluent2.microsoft.design/motion): a first-party reduced-motion approach: constrain motion to focused elements and carry meaning through other channels.
