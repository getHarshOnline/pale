+++
description = "Use it to implement a selected page treatment that responds to scrolling while the reader stays in control, once the brief has decided the effect."
uses = [{ scope = "local", resource_id = "animate" }]
see_also = [{ scope = "local", resource_id = "animate/temporal-hierarchy" }, { scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "inspect" }, { scope = "local", resource_id = "animate/productive-motion" }, { scope = "local", resource_id = "figures" }, { scope = "local", resource_id = "animate/motion/guide" }, { scope = "local", resource_id = "animate/animejs/guide" }, { scope = "local", resource_id = "animate/motion-toolchain" }, { scope = "local", resource_id = "animate/scroll/native-scroll" }, { scope = "local", resource_id = "films" }, { scope = "local", resource_id = "animate/scroll/aos" }, { scope = "local", resource_id = "animate/scroll/practical-tips" }, { scope = "local", resource_id = "animate/reduced-motion" }, { scope = "local", resource_id = "access/web" }, { scope = "local", resource_id = "user-task" }, { scope = "local", resource_id = "animate/feedback-roles" }]
official_sources = [{ label = "CSS scroll-driven animations", url = "https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Scroll-driven_animations" }]

[guide]
exposure = "internal"

[stack]
+++

# Build a scroll-linked effect or one-time arrival

Adapted from the Claude Skills Project’s MIT-licensed [scroll-reveal-libraries guide](https://github.com/freshtechbro/claudedesignskills/tree/1da73febff0c3e1dfefc07f8a5ef8f7d1dfdb6cd/.claude/skills/scroll-reveal-libraries). The licence travels with this guide.

Scroll is the reader's own motion through a document. This skill implements a selected page treatment while preserving that control. Art direction and the brief decide its expressive purpose; implementation must not substitute a compulsory entrance style. Engine constraints (static-first markup, the style and module profiles, vendored modules, the mount contract) are the project’s module loading and mount guide; this skill never restates them.

## Before writing

1. **Name the meaning** → [Show what happens first, “Order encodes causality”](../temporal-hierarchy.md#order-encodes-causality); bring back what arriving in order explains: a step reaching its place, a stage of a narrative, a figure read in order. "It looks more alive" is not a meaning.
2. **Match the expressive range** → [Choose a visual direction from the actual brief](../../art-direction.md) explains how to choose emphasis from the brief. A system may reserve entry motion for its strongest level; use the actual project’s rule and choose the region whose arrival matters.
3. **Never blanket** → one reveal on every section is the same-entrance tell ([Inspect an interface in its rendered context](../../inspect.md)); it fails whichever mechanism draws it. Bring back the short list of regions the template marks with `data-reveal`.
4. **Never delay content** → [Choose feedback roles and timing, “2. Rules”](../feedback-roles.md#2-rules); the element is readable and operable in its final state before, during and without the effect.
5. **Island or page?** → a scroll-linked figure, field or diagram is an island ([Compose and check a live visual region](../../figures.md)); its motion follows [Build a page animation with Motion](../motion/guide.md) or [Build a page animation with anime.js](../animejs/guide.md), and only its trigger is here.
6. **Choose the mechanism**; the lightest one that expresses the change wins ([Choose the lightest animation mechanism, “2. Choose by job”](../motion-toolchain.md#2-choose-by-job)):

| Job | Mechanism | Reference |
| --- | --- | --- |
| Progress linked to scroll position (a reading-progress mark, a figure advancing with its section) | CSS scroll-driven animation (`animation-timeline: scroll()` or `view()`), gated by `@supports` | [Use browser scroll timelines and arrival observers, “1. Scroll-linked progress with CSS”](native-scroll.md#1-scroll-linked-progress-with-css) |
| A one-time arrival when an element enters view | Intersection Observer in a small module, or Motion `inView()` inside a Motion island | [Use browser scroll timelines and arrival observers, “2. One-time arrival with Intersection Observer”](native-scroll.md#2-one-time-arrival-with-intersection-observer) |
| Start or pause an island at the viewport edge | Motion `inView()` ([Build a page animation with Motion](../motion/guide.md)) or an Intersection Observer | [Use browser scroll timelines and arrival observers, “2. One-time arrival with Intersection Observer”](native-scroll.md#2-one-time-arrival-with-intersection-observer) |
| Frames of a cinematic sequence chosen by scroll position | the edit's own frame loop in its island; only its start and suspension are triggers here | [Compose a sequence people can follow](../../films.md); [Choose the lightest animation mechanism, “2. Choose by job”](../motion-toolchain.md#2-choose-by-job) |
| A one-time reveal where AOS is already the chosen dependency | AOS, with every mitigation applied | [Keep an AOS reveal accessible and usable](aos.md) |

7. **The common moves** → [Diagnose a scroll effect or arrival](practical-tips.md); note the tip for the situation (a reveal requested, written or reviewed) and run its **Check:** once written.

## Core rules

- **Visible without scripts.** No stylesheet or markup hides, offsets or disables an element before a script has run and chosen to animate it; the hidden start state is applied by the script.
- **Choose the authored arrival.** Depth, light, translation, masking or another supported treatment may fit the reference the project uses and world. Repeating an unchosen preset on every section is the tell, not the mere presence of translation ([Use browser scroll timelines and arrival observers](native-scroll.md)).
- **Timing from tokens.** Durations and easings are the role's tokens ([Design motion that helps people use the interface](../../animate.md) motion implementation); never data-attribute numbers.
- **The reduced state is designed.** A reveal or scroll-linked figure reduces to a light change or an instant state that lands, never to nothing. Rules: [Design the reduced-motion alternative](../reduced-motion.md); the code is in [Use browser scroll timelines and arrival observers](native-scroll.md).
- **Operable throughout.** Nothing in or under a reveal loses pointer or keyboard access while waiting to animate ([Make the browser task accessible](../../access/web.md)).
- **Once.** A reveal plays once and never replays when scrolling back.
- **Never** scroll-jacking, smooth-scroll replacement or pinned scrubbing of reading content. A cinematic sequence pins its images, never its words, and reads browser scroll without replacing it ([Compose a sequence people can follow](../../films.md)).

## Read the reference when

| When the work is… | Read |
| --- | --- |
| A reveal is requested, written or reviewed, and checking it worked | [Diagnose a scroll effect or arrival](practical-tips.md) |
| A scroll-driven animation, an Intersection Observer arrival, the reveal sample, reduced motion for either | [Use browser scroll timelines and arrival observers](native-scroll.md) |
| AOS is already a dependency: fit, versions, defects and mitigations | [Keep an AOS reveal accessible and usable](aos.md) |

## Quick detection

- `[data-reveal]` or `[data-aos]` on most sections, or where no purpose was established → remove it (steps 2–3).
- An arrival copied across regions without a purpose in the brief → reconcile it with the brief ([Use browser scroll timelines and arrival observers, “2. One-time arrival with Intersection Observer”](native-scroll.md#2-one-time-arrival-with-intersection-observer)).
- An element hidden by CSS (`opacity: 0`, offset transform, `visibility: hidden`) with no script-set gate → let the script set the pending state.
- `data-aos-duration`, `data-aos-delay` or `data-aos-easing` with numbers → token custom properties ([Keep an AOS reveal accessible and usable, “3. Defects and their mitigations”](aos.md#3-defects-and-their-mitigations)).
- A scroll listener seeking an animation on every event → `animation-timeline` or an observer.
- A scroll-driven animation outside `@supports` → gate it.
- A reveal that replays on scroll-up (`mirror`, `once: false`) → once.
- No reduced-motion path, or `animation: none` where the progress had a light carrier → the reduced state in [Use browser scroll timelines and arrival observers, “1. Scroll-linked progress with CSS”](native-scroll.md#1-scroll-linked-progress-with-css).

## Gotchas

- The `animation` shorthand resets `animation-timeline`; declare the timeline after it.
- A scroll-linked animation reverses when the reader scrolls back; a one-time arrival uses an observer instead.
- AOS 3.0 beta sets `pointer-events: none` on pending elements and hides them under `html:not(.no-js)`; 2.3.4 hides them unconditionally.

## After writing

Look in a real browser, scrolling at reading speed and fast ([Choose a visual direction from the actual brief](../../art-direction.md)). Feel-check the arrival with [Inspect an interface in its rendered context](../../inspect.md): slow it in the browser's animation inspector and step it frame by frame. Then judge it (does any reveal carry meaning?) and verify with [Check a task from entry to result](../../user-task.md): scripts off, reduced motion on, a browser without scroll-driven animation, keyboard tabbing into not-yet-revealed content. Compare the result with the intended expressive character and put the observations in the component’s existing review.

## Worked example

Keep a section readable at rest. Add a supported CSS scroll timeline to the decorative marker, then inspect the same position reached from above and below. With animation unavailable or reduced, the section remains complete and the marker can use its composed rest drawing.

## Before handing off

- [ ] Does the running component show the intended change with the supported API and clock?
- [ ] Are content, focus and controls usable through interruption, failure and reduced motion?
- [ ] Do the observed result and any unrun checks appear in the existing review?
