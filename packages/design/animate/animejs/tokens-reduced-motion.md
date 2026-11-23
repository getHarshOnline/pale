+++
description = "Use it to turn motion tokens into anime.js parameters, route reduced motion into an island and clean up afterwards."
uses = [{ scope = "local", resource_id = "animate/animejs/guide" }]
see_also = [{ scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "animate/productive-motion" }, { scope = "local", resource_id = "animate/motion/token-presets" }, { scope = "local", resource_id = "animate/motion/guide" }, { scope = "local", resource_id = "web" }, { scope = "local", resource_id = "animate/reduced-motion" }, { scope = "local", resource_id = "access/web" }, { scope = "local", resource_id = "animate/read-tokens" }, { scope = "local", resource_id = "animate/feedback-roles" }, { scope = "local", resource_id = "animate/profile-a-frame" }]

[guide]
exposure = "internal"
+++

# Use tokens and reduced motion with anime.js

How motion tokens become anime.js parameters, how reduced motion reaches an island, and how it cleans up. The rule for shared design-role values and the spring policy live in [Read timing and spring roles from the system, “2. How islands take their values and the spring policy”](../read-tokens.md#2-how-islands-take-their-values-and-the-spring-policy); the meaning of each role in [Choose feedback roles and timing, “1. The role contract”](../feedback-roles.md#1-the-role-contract); values in the consuming project’s token source. The role-to-token map is shared with Motion islands: [Convert design tokens to Motion options, “2. Role to token map”](../motion/token-presets.md#2-role-to-token-map).

The names, custom-property output, `data-motion` attribute and `motion-preference` event below belong to one worked theme. Adapt them to the project’s declared interface. These helpers assume nonempty, valid CSS times and cubic-bezier values; validate token output at its source and handle missing values before calling the library. They are conversion examples, not a general CSS parser.

## Contents

1. Reading and converting tokens
2. Springs: the emitted easing
3. Reduced motion through a scope
4. Revert, never unmount
5. Detection

## 1. Reading and converting tokens

The token build emits each motion token as a custom property; reading those build outputs is how an island takes its numbers from tokens without restating them. The island reads the computed value from its own element so the world and mode in force there apply. anime.js takes milliseconds by default.

```js
import { cubicBezier, linear } from "./anime.js"; // this example project’s vendored module

const read = (el, name) =>
  getComputedStyle(el).getPropertyValue(`--${name}`).trim();
const ms = (v) => (v.endsWith("ms") ? parseFloat(v) : parseFloat(v) * 1000);
const bezier = (v) =>
  cubicBezier(...(v.match(/-?\d*\.?\d+/g) ?? []).map(Number));

export const tweenMs = (el, role, easing = "easing-productive") => ({
  duration: ms(read(el, `duration-${role}`)),
  ease: bezier(read(el, easing)),
});
```

- `cubicBezier()` is a function export; passing `'cubicBezier(…)'` as a string is a deprecated form.
- The expressive role multiplies `duration-expressive` by `world-motion-expressive-scale` and takes `world-motion-expressive-easing`; productive roles never read `world-motion-*`.
- Its reduced form is the same shape one layer down: `motion-reduced-expressive` multiplied by `world-motion-reduced-expressive-scale`, on `world-motion-reduced-expressive-easing`. A reduced expressive moment built on a productive reduced value is the defect [Profile and repair motion in the browser, “5. Detection rules”](../profile-a-frame.md#5-detection-rules) names, because the world's character goes with it.
- Stagger intervals read `delay-stagger`: as read for a productive stagger, multiplied by `world-motion-expressive-scale` for an expressive one. That scale multiplication is the composition used by this example (the spring policy section named above); other shared role values are used as read after unit conversion.
- The multiplication by 1000 is a unit change, not a design value. An empty read is a token gap to report, never a place to type a number.
- Helper names carry the unit (`tweenMs`); the [Build a page animation with Motion](../motion/guide.md) helpers carry seconds (`tweenSeconds`), so a helper copied between engines cannot pass the wrong unit silently.

## 2. Springs: the emitted easing

Under the example spring policy an anime.js island takes the token-emitted `linear()` easing (or a token cubic-bézier) and never builds its own spring. The build emits each spring token as a duration and a `linear()` easing sampled from it, so the island moves exactly like the stylesheet (CSS form: [Carry a design through the web build](../../web.md)):

```js
const points = (v) =>
  v
    .replace(/^linear\(|\)$/g, "")
    .split(",")
    .map((p) => p.trim());

export const springPreset = (el, prefix) => ({
  duration: ms(read(el, `${prefix}-duration`)),
  ease: linear(...points(read(el, `${prefix}-easing`))),
});
// springPreset(el, "motion-spring-productive") or springPreset(el, "world-motion-expressive-spring")
```

- anime.js `linear(...points)` builds a piecewise-linear ease from the same stops; a stop with one percentage (`'0.5 40%'`) is accepted.
- `spring()` (4.2+; `createSpring()` in 4.0–4.1) replaces the animation's duration with its own settling time and follows its own model. Use it only when the project deliberately chooses that model; it is not interchangeable with the sampled curve in this example.

## 3. Reduced motion through a scope

The reduced state is one of the four presentation states an island renders, so it gets its own timeline, built with the same care as the full one: the same labels and order, with drawing and travel re-expressed as the finished geometry arriving by opacity, and order carried by the figure's own numbering or position. What each form is: [Design the reduced-motion alternative, “Translate, don't delete”](../reduced-motion.md#translate-dont-delete); its floor: [Make the browser task accessible](../../access/web.md). The code reads the motion preference and rebuilds on either change; the attribute and the `motion-preference` event are the project’s stylesheet and token guide.

```js
const root = document.documentElement;
const reduced = (systemReduces) =>
  root.dataset.motion === "reduce" ||
  (root.dataset.motion !== "allow" && systemReduces);

// The reduced timeline: same labels, no drawing or travel; each part lands whole, in order.
const reducedTimeline = (el) =>
  createTimeline({
    defaults: { duration: ms(read(el, "motion-reduced-sequence")) },
    autoplay: false,
  })
    .label("ends")
    .add(nodes, { opacity: [0, 1] }, "ends")
    .label("edge")
    .add(edge, { opacity: [0, 1] }, "edge");

let timeline;
const scope = createScope({
  root: island,
  mediaQueries: { reduce: "(prefers-reduced-motion: reduce)" },
}).add((self) => {
  // fullTimeline: the drawn sequence of timelines, staggers and SVG, "createTimeline()"
  timeline = reduced(self.matches.reduce)
    ? reducedTimeline(island)
    : fullTimeline(island);
});

const onPreference = () => {
  timeline.complete(); // this diagram’s finished state is its intended still
  scope.refresh();
};
document.addEventListener("motion-preference", onPreference);
const dispose = () => {
  document.removeEventListener("motion-preference", onPreference);
  scope.revert();
};
```

- If reduced motion switches on mid-sequence, settle the current logical state before rebuilding; use `complete()` only when the timeline end represents that state. `keepTime()` only for animations whose position must survive a re-run.

This fragment assumes `nodes`, `edge`, `island` and `fullTimeline` are supplied by the component. Its playback or visibility controller starts the selected timeline and preserves an explicit pause. Call `dispose()` when that component leaves the page, along with its observer cleanup. A component that must preserve a partially selected logical state should rebuild that state instead of completing this diagram.

## 4. Revert, never unmount

Revert the enhancement while its host remains; a framework may still unmount the component when it leaves the page. Release its observers and listeners at that point.

- In a component that keeps its mount when the preference changes, `scope.revert()` is the island's way back to the still: every animation the scope created is removed and inline styles are restored.
- Pause the timeline when the island leaves the viewport (an Intersection Observer, or Motion `inView()` when Motion is already present; do not add a redundant observer controller) and resume on return; the engine already pauses when the document is hidden.
- A failure never leaves a half-drawn figure: revert and let the still show (the project’s module loading and mount guide).

## 5. Detection

- A local shared-role value that bypasses its token; authored scene offsets retain their own timing.
- A library spring substituted for the project’s sampled easing without comparing the resulting timing.
- Tokens read from the document root instead of the island element.
- A reduced branch that only shortens durations, leaves travel, hidden starting states or cleanup to a blanket zero-duration override, ignores `data-motion`, or is absent.
- No `revert()` on failure or motion-preference change.

## Sources

- [anime.js documentation: Easings](https://animejs.com/documentation/easings): `cubicBezier`, `linear`, `spring` and the ease string syntax.
- [anime.js documentation: Spring](https://animejs.com/documentation/easings/spring): how a spring overrides the animation's duration, the reason islands take the emitted easing instead.
- [anime.js documentation: Scope](https://animejs.com/documentation/scope): media queries, `self.matches`, `refresh`, `keepTime` and `revert`.
- [MDN: linear() easing function](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/easing-function/linear): the stop syntax of the emitted spring easings.
- [MDN: getComputedStyle](https://developer.mozilla.org/en-US/docs/Web/API/Window/getComputedStyle): reading resolved custom property values from the island element.
- [MDN: prefers-reduced-motion](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion): the media feature the scope queries.
