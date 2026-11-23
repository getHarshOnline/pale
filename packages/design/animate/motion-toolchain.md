+++
description = "Use it to decide between CSS, a browser API and a library for the change you designed."
uses = [{ scope = "local", resource_id = "animate" }]
see_also = [{ scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "films" }, { scope = "local", resource_id = "animate/motion/guide" }, { scope = "local", resource_id = "animate/animejs/guide" }, { scope = "local", resource_id = "animate/scroll/guide" }, { scope = "local", resource_id = "animate/reduced-motion" }, { scope = "local", resource_id = "animate/interruption-reversal" }, { scope = "local", resource_id = "animate/productive-motion" }, { scope = "local", resource_id = "web" }, { scope = "local", resource_id = "figures" }, { scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "animate/scroll/aos" }, { scope = "local", resource_id = "inspect" }, { scope = "local", resource_id = "animate/time-models" }, { scope = "local", resource_id = "animate/retarget-controls" }, { scope = "local", resource_id = "animate/preserve-transform-origin" }, { scope = "local", resource_id = "animate/frequency-and-pending" }, { scope = "local", resource_id = "animate/feedback-roles" }]

[guide]
exposure = "internal"
+++

# Choose the lightest animation mechanism

Choose the smallest mechanism that can show the change you designed. Use the ladder below when deciding between CSS, a browser API and a library. How tokens reach each mechanism is in [Implement motion from the shared design](motion-implementation.md). Authored films use [Compose a sequence people can follow](../films.md) and the pinned film renderer, including its supported animation and drawing libraries; this page ladder is not a restriction on film choreography.

## Contents

1. Rules for every animation tool
2. Choose by job
3. The tool skills
4. Detection rules
5. Review questions

## 1. Rules for every animation tool

Each tool has its own skill for API knowledge: [Build a page animation with Motion](motion/guide.md), [Build a page animation with anime.js](animejs/guide.md) and [Build a scroll-linked effect or one-time arrival](scroll/guide.md). These rules govern page motion, including a film's host controls; the film itself follows the film workflow above:

1. **Shared interface timing and world character use tokens.** Authored scene action follows the timing its action and reading window need even inside a page; classify that role through [Implement motion from the shared design](motion-implementation.md) before applying token checks.
2. **Reduced state:** designed forms per [Design the reduced-motion alternative](reduced-motion.md); code per [Build the origin and reduced form into the control, “4. Reduced motion in code”](preserve-transform-origin.md#4-reduced-motion-in-code).
3. **Prefer CSS for standalone micro-states.** Do not add a library just for hover, press, focus, selection or disclosure; preserve an already chosen engine when these belong to its coordinated choreography.
4. **Content is never hidden before scripts run.** Any initial hidden or offset state is applied only after the script has loaded and decided to animate; with scripts off or failed, every element is visible in its final state.
5. **Time model and interruption** follow [Choose what advances the motion](time-models.md) and [Reverse or retarget a control in flight, “Rules”](retarget-controls.md#rules); the chosen tool never changes that decision.
6. **The grammar still governs.** Frequency gates amplitude ([Keep repeated and pending responses useful, “3. Frequency rule”](frequency-and-pending.md#3-frequency-rule)), hover never changes geometry around a control, cascades that clarify order or attention without delaying use, and productive exits on their assigned shorter role.

## 2. Choose by job

This section is the one home of the mechanism ladder; other files point here.

Take the lightest step that runs the motion, in this order, while considering the engine already selected and the cost of replacing it:

1. **CSS transition** between two states, reading token custom properties; springs through the token-emitted `linear()` easing. It retargets from the current value when interrupted ([Reverse or retarget a control in flight, “Choosing mechanisms”](retarget-controls.md#choosing-mechanisms)).
2. **`@starting-style`** for the entry of a CSS state: an element or top-layer surface (dialog, popover) arriving from `display: none`, with `transition-behavior: allow-discrete` for its exit. The code is [Carry a design through the web build](../web.md); compile it once through the engine before relying on it (the project’s stylesheet and token guide).
3. **CSS animation** (`@keyframes`) for a one-way sequence that is never reversed mid-flight; keyframes restart when replaced.
4. **WAAPI** (`element.animate()`) in an island when script must start, reverse or retarget a motion from its current time, with durations and easings read from tokens.
5. **A vendored engine** when its sequencing, integration or reusable source materially serves the work: [Build a page animation with Motion](motion/guide.md) for gestures and scroll-linked progress, [Build a page animation with anime.js](animejs/guide.md) for timelines, staggers, line drawing and morphing. Prefer one animation engine per component and give each property one writer. Momentum requires a model and project parameters that carry release velocity; see [Keep repeated and pending responses useful, “5. Open questions”](frequency-and-pending.md#5-open-questions).

| Job | Mechanism | Never |
| --- | --- | --- |
| Productive micro-states: hover, press, focus, selection, disclosure | CSS transitions reading token custom properties; retain an engine already in use when these states belong to its coordinated choreography | Adding a library solely for a standalone micro-state |
| Entry of a CSS state (a dialog, a popover, an element leaving `display: none`) | `@starting-style` with a transition; `transition-behavior: allow-discrete` for exit | Keyframes for a state that can reverse; a script for what CSS enters |
| Island motion started or reversed from script: element and value animation from tokens | WAAPI | A library loaded for a single tween |
| Island motion that WAAPI cannot run: gestures, scroll-linked progress | Motion's plain JavaScript API ([Build a page animation with Motion](motion/guide.md)) | Competing animation engines; layout churn where composited motion represents the change ([Choose feedback roles and timing, “2. Rules”](feedback-roles.md#2-rules)); arbitrary spring values that bypass the project’s design |
| Sequenced steps, timelines and staggers where order is information | anime.js 4 ([Build a page animation with anime.js](animejs/guide.md)), or Motion when the island is already a Motion island for springs or gestures | A second engine without a necessary capability |
| Line drawing, morphing or motion paths on runtime-built SVG | anime.js 4 ([Build a page animation with anime.js](animejs/guide.md)) is a default for combined drawing/morphing; an existing or explicitly selected Motion engine can draw paths with its supported SVG APIs | Adding a second engine for a feature the version in use already supplies, or claiming unsupported morphing works |
| An animation authored in a design tool (a performing mark, an illustrated moment) | its player, vetted and shared, with the still, colours and timing reconciled ([Compose and check a live visual region](../figures.md)) | an unreviewed asset that misstates identity, status or rights |
| Anything visual between one page and the next | a declared cross-document view transition, earned by a crossing or a sequence step ([Carry a design through the web build](../web.md)) | a script that intercepts links and swaps markup into the current document |
| Sequenced drawing tied to scroll | Trigger per [Build a scroll-linked effect or one-time arrival](scroll/guide.md), sequence in anime.js; one engine | A second engine for the trigger |
| Motion on reach or entry (anything that starts because the reader scrolled a region into view, or the page arriving on load) | Use the project’s accepted expressive range and purpose; see [Choose a visual direction from the actual brief](../art-direction.md) | Entry motion without a reason in the page’s brief |
| Scroll-linked progress or scroll-triggered start | In order: CSS scroll-driven animations, then Intersection Observer or Motion `inView()`. AOS can serve a chosen reveal after its version’s failure modes are addressed; compare its cost with the native pattern ([Keep an AOS reveal accessible and usable, “1. When AOS fits”](scroll/aos.md#1-when-aos-fits)) | Drawing inside every scroll event when a timeline or one scheduled frame would serve |
| A one-time reveal that carries meaning in a narrative or sequence (a step arriving in its order) | The same order as above | Blanket fade-up entrances; a reveal outside the accepted direction; any reveal that delays content |
| A cinematic sequence scrubbed by the reader ([Compose a sequence people can follow](../films.md)) | its pacing table drives it: browser scroll records the position and an island draws the frame for that position once per display frame, then stops. A CSS scroll timeline can also hold a picture through repeated equal keyframe values; use it when the complete edit maps to supported CSS properties, otherwise use the position-selected island renderer | A film that plays by the clock; drawing inside the scroll handler; a replaced or slowed scroll |
| An island state played to its rest (a press, a selection, focus, the pointer entering or leaving: [Compose and check a live visual region](../figures.md)) | a CSS transition between made states on the expressive tokens, or WAAPI stepping its frames when script must retarget it from the current frame | a clip scrubbed by scroll; a transition stopped between rests; an act started by a timer |
| A picture bound to the pointer (an actor or field aware of the reader) | pointer position recorded in the island's box, drawn once per display frame using the response the design calls for ([Compose and check a live visual region](../figures.md)) | unexplained response values or silently replacing an intended continuous response with zones |

- A blanket entrance on every section is the fade-and-slide-up anti-pattern in [Inspect an interface in its rendered context](../inspect.md); it fails whichever library draws it.
- What draws an island's pixels is a separate choice from what advances its time: the drawing layer is [Compose and check a live visual region](../figures.md), and this ladder still decides the timing.
- Choosing a mechanism never changes the motion decision. Name the role and its token first, then pick the lightest tool that runs it.
- A second animation engine on a page is a reviewed decision, not a convenience. Drawing layers are separate from animation engines; the selected runtime owns how their clocks compose.
- **SVG.** Follow the renderer’s supported formats. If it disallows SVG markup or assets, build geometry inside the mounted component and retain a complete raster or CSS fallback. Other renderers may use authored SVG directly.

A one-time arrival stops watching once the content has arrived. A region that pauses offscreen keeps watching so it can resume from where it left off. Check the tallest target in the shortest supported viewport before choosing a visible fraction. That fraction can be impossible to reach. Load a link to the section directly, tab into it, and turn scripts off. Content should be visible and usable in all three cases.

## 3. The tool skills

- **[Build a page animation with Motion](motion/guide.md)**: Motion's plain JavaScript API for page islands: `animate()`, springs by visual duration and bounce, sequences and `stagger()`, `inView()`, `scroll()`, `hover()`, `press()` and playback controls, with token conversion.
- **[Build a page animation with anime.js](animejs/guide.md)**: anime.js 4 in islands whose job is ordered steps, a diagram being drawn or a shape changing: timelines, staggers, and line drawing, morphing and motion paths on geometry the island builds at runtime.
- **[Build a scroll-linked effect or one-time arrival](scroll/guide.md)**: the choice of scroll mechanism: CSS scroll-driven animation first, then Intersection Observer or Motion `inView()`, and AOS only for an earned one-time narrative reveal.
- Read the project's module loading rules before adding an engine. Check the pinned module, its imports and what the page shows if the script fails.

## 4. Detection rules

- A new JavaScript library added solely for a standalone hover, press, focus or disclosure state that CSS serves.
- Competing writers, or a second animation engine without a needed capability.
- Any initial hidden state styled without a script-set gate; content invisible with scripts off.
- A scroll listener that seeks an animation on every event where CSS scroll-driven animation or `scroll()` would do, or one that draws; a scrubbed sequence's listener only records the position.
- The same reveal on every section.
- Motion on reach, entry or page load where [Choose a visual direction from the actual brief](../art-direction.md) does not permit it.
- A library or keyframes where a CSS transition or `@starting-style` would run the motion.
- An unsupported drawing or morphing API, or an unnecessary second engine when the installed engine can do it.
- Plus every detection in [Implement motion from the shared design](motion-implementation.md) (literals, reduced motion, versions, layout properties).

## 5. Review questions

1. For each moving thing, which role or authored action, which timing source, and which mechanism serves it?
2. With scripts off, is every element visible in its final state?
3. Does each engine serve a needed capability, with one writer per animated property?
4. Does each shared role read its token, and each authored action carry deliberate scene timing rather than an unexamined example?

## Worked example: a diagram that draws in order

The diagram must show two nodes before it draws the line that connects them. A CSS transition works for the hover state of either node. It cannot tell the whole ordered story when the diagram is assembled at runtime. A labelled anime.js timeline can reveal the nodes, then draw the edge. The complete diagram and its labels remain visible if the script fails. When the sequence needs to follow scroll position instead, use a CSS scroll timeline if its properties can express the whole edit, or draw one frame per browser frame from the latest scroll position.

## Sources

- [Motion documentation](https://motion.dev/docs): the Motion plain JavaScript API used in islands.
- [Motion: scroll](https://motion.dev/docs/scroll): scroll-linked animation that uses the browser scroll timeline where supported.
- [Motion: inView](https://motion.dev/docs/inview): starting an island's work when it enters the viewport, built on Intersection Observer.
- [anime.js documentation: Installation](https://animejs.com/documentation/getting-started/installation): the current named ES module exports (`animate`, `createTimeline`, `stagger`, `svg`) and the ES module bundle.
- [anime.js documentation: Timeline](https://animejs.com/documentation/timeline): sequencing island steps with a timeline, for jobs where order is information.
- [anime.js documentation: SVG](https://animejs.com/documentation/svg): line drawing, morphing and motion paths applied to SVG an island builds at runtime.
- [anime.js documentation: Engine](https://animejs.com/documentation/engine): engine settings including pausing when the document is hidden.
- [MDN: CSS scroll-driven animations](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Scroll-driven_animations): scroll and view progress timelines in CSS, the first choice for scroll-linked motion.
- [MDN: Intersection Observer API](https://developer.mozilla.org/en-US/docs/Web/API/Intersection_Observer_API): the browser API for one-time reveals and offscreen suspension without a library.
- [AOS repository](https://github.com/michalsnik/aos): AOS behaviour and its stylesheet's hidden initial state (unless the root carries a no-script class) in the 3.0 beta; 2.x hides unconditionally; for checking that content is never hidden before scripts run.
- [MDN: @starting-style](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@starting-style): entry styles of elements and top-layer surfaces transitioning from `display: none`, and its support.
- [MDN: Element.animate()](https://developer.mozilla.org/en-US/docs/Web/API/Element/animate): starting a WAAPI animation from script and the Animation object that reverses, updates and reports its current time.
- [CSS Transitions Level 1](https://www.w3.org/TR/css-transitions-1/): how an interrupted transition starts from the current value, the reason transitions sit first on the ladder.
