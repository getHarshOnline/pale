+++
description = "Use it for the framework-agnostic Motion calls an island makes."
uses = [{ scope = "local", resource_id = "animate/motion/guide" }]
see_also = [{ scope = "local", resource_id = "animate/motion/token-presets" }, { scope = "local", resource_id = "figures" }, { scope = "local", resource_id = "animate" }, { scope = "local", resource_id = "animate/motion-toolchain" }, { scope = "local", resource_id = "animate/interruption-reversal" }, { scope = "local", resource_id = "surfaces" }, { scope = "local", resource_id = "web" }, { scope = "local", resource_id = "animate/temporal-hierarchy" }, { scope = "local", resource_id = "animate/animejs/guide" }, { scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "animate/productive-motion" }, { scope = "local", resource_id = "animate/scroll/guide" }, { scope = "local", resource_id = "controls" }, { scope = "local", resource_id = "animate/time-models" }, { scope = "local", resource_id = "animate/retarget-controls" }, { scope = "local", resource_id = "animate/pause-and-resume" }, { scope = "local", resource_id = "animate/read-tokens" }, { scope = "local", resource_id = "animate/feedback-roles" }]

[guide]
exposure = "internal"
+++

# Use the Motion JavaScript API in a component

The framework-agnostic Motion API as an island uses it. Shared design-role values come from tokens ([Convert design tokens to Motion options](token-presets.md)); authored scene action follows its selected temporal treatment ([Compose and check a live visual region](../../figures.md)). Which motion exists and why is decided first in [Design motion that helps people use the interface](../../animate.md); engine constraints (vendoring, imports, the mount contract) are the project’s module loading and mount guide.

## Contents

1. Builds and imports
2. `animate()`
3. Sequences and `stagger()`
4. Springs
5. Playback controls and interruption
6. `inView()` and `scroll()`
7. `hover()` and `press()`
8. Forms that are not current

## 1. Builds and imports

| Build | Import | Carries |
| --- | --- | --- |
| Hybrid | `animate` from `motion` | HTML and SVG styles plus independent transforms (`x`, `y`, `scale`, `rotate`), CSS variables, SVG paths, sequences and plain JavaScript objects |
| Mini | `animate` from `motion/mini` | HTML and SVG styles only, on native browser animation (CSS variables only when registered with `@property`); springs by passing the imported `spring` as `type` |

- Motion runs on the browser's Web Animations API (WAAPI) wherever it can; the mini build is a thin layer over it. An island whose motion is one property with no spring, sequence or gesture may need no vendored engine at all: `element.animate()` takes the token easing strings as they are and durations in milliseconds (the ladder is [Choose the lightest animation mechanism, “2. Choose by job”](../motion-toolchain.md#2-choose-by-job)).
- Choose mini when the island animates only opacity, full transforms or CSS variables registered with `@property`, and needs no sequence; choose hybrid when it needs independent transforms, sequences or unregistered CSS variables.
- `inView`, `scroll`, `stagger`, `spring`, `hover` and `press` are named exports of `motion`.
- In a theme with the same module and markup rules these names resolve through the vendored module, never the package or a URL (the project’s module loading and mount guide).

## 2. `animate()`

```js
const controls = animate(target, keyframes, options);
```

- **Target:** an element, a selector, an array of elements, a motion value or a plain object. Inside an island, pass elements found under the island's own mount so nothing outside it moves.
- **Keyframes:** an object of target values; an array gives several keyframes for one value.
- **Options that carry timing** (shared roles from tokens; authored scene values from its treatment):
  - `duration` and `delay` in seconds; a negative delay starts part way through.
  - `ease`: a named easing or a cubic Bézier array of four numbers.
  - `type`: `"tween"`, `"spring"` or `"inertia"`.
  - `visualDuration` and `bounce` for a spring (section 4).
  - `repeat` and `repeatType` (`"loop"`, `"reverse"`, `"mirror"`) follow the selected temporal purpose and clock ([Choose what advances the motion](../time-models.md)). An ambient or evolving field carries its own suspension and reduced form; a control does not repeat merely to solicit attention.
- **Movement is a full `transform`.** Animate `transform` as one string, and opacity, for movement. The independent transforms (`x`, `y`, `scale`, `rotate`) are animated through CSS variables and are not hardware-accelerated; use them only where two axes must animate independently on one element, and put the measured frame cost in the existing review.
- Animate a CSS variable when the stylesheet already derives appearance from it (for example emission strength, which [Choose what material and light communicate](../../surfaces.md) defines), on the element that consumes it, never on `:root` or a large ancestor: every descendant restyles on each frame ([Carry a design through the web build](../../web.md)).

## 3. Sequences and `stagger()`

```js
animate([
  [first, keyframesA, optionsA],
  [second, keyframesB, { ...optionsB, at: "<" }],
]);
```

- Each segment is `[target, keyframes, options]`. `at` places a segment: an absolute time, a label, `"<"` (with the previous segment's start), or a relative offset from the previous end (`"+…"`) or start (`"<…"`). Interface offsets derive from role tokens; scene offsets follow the selected authored temporal treatment.
- Give a sequence a temporal purpose: a relation becoming legible, coordinated UI movement or intended expressive rhythm. Follow [Show what happens first](../temporal-hierarchy.md) and the clock you chose.
- `stagger(interval, { startDelay, from, ease })` returns a per-element delay; `interval` is in seconds. Shared-role intervals read `delay-stagger` ([Convert design tokens to Motion options, “2. Role to token map”](token-presets.md#2-role-to-token-map)); scene intervals follow their authored treatment. `from` is `"first"`, `"center"`, `"last"` or an index. Pass it as `delay`.
- The hybrid API supports SVG drawing through `pathLength`, `pathSpacing` and `pathOffset`. Check the pinned version for the specific path or morph operation; keep an explicit or installed engine when it fits. Complex morphing can use [Build a page animation with anime.js](../animejs/guide.md) without imposing that choice on every SVG animation ([Choose the lightest animation mechanism, “2. Choose by job”](../motion-toolchain.md#2-choose-by-job)).
- Never stagger a list the reader scans or navigates by keyboard; a cascade is permitted only when the order itself is the content ([Show what happens first, “Cascades”](../temporal-hierarchy.md#cascades)).

## 4. Springs

The example duration-based spring policy is [Read timing and spring roles from the system, “2. How islands take their values and the spring policy”](../read-tokens.md#2-how-islands-take-their-values-and-the-spring-policy): a Motion island animates springs from the token parameters `visual-duration` and `bounce`.

- **Duration-based spring:** `type: "spring"` with `visualDuration` (seconds; when the motion first reaches its target) and `bounce` (0 is no bounce, 1 very bouncy). `visualDuration` overrides `duration`. The tokens are `motion.spring.productive.*` and `world.motion.expressive-spring.*`.
- **Physics spring:** `stiffness`, `damping` and `mass` override `visualDuration` and `bounce`; use them when the consuming system defines that model and its shared parameters.
- **Released gestures:** duration-based springs ignore velocity, including an explicit `velocity` (the pinned spring generator in Sources). A gesture that must retain momentum needs the project’s physics parameters and a release check; a duration-based preset cannot promise that behavior.
- **Mini build:** pass `spring` itself: `{ type: spring, visualDuration, bounce }`.
- **CSS:** stylesheets consume the token-emitted duration and `linear()` easing ([Carry a design through the web build](../../web.md)). Islands never generate CSS springs at runtime.

## 5. Playback controls and interruption

`animate()` returns controls:

| Member | Effect |
| --- | --- |
| `then()` | Resolves when the animation finishes |
| `play()` / `pause()` | Resume or pause |
| `stop()` | Commit the current values to style and end; cannot restart |
| `cancel()` | Revert to the state before the animation |
| `complete()` | Jump to the end state |
| `time` | Read or set the current position in seconds |
| `speed` | Read or set the playback rate; a negative rate plays backwards. Also the feel-check hook: a slowed rate during review, never in shipped code |
| `duration` | Read one iteration's length |

- **Retarget, do not restart.** On new input, start a new `animate()` for the same values toward the new target; it begins from the current rendered value. Keep the latest controls and let the newest input win ([Reverse or retarget a control in flight, “Rules”](../retarget-controls.md#rules)).
- `cancel()` snaps back, so it is for abandoning a state change entirely, not for interruption.
- Pause on visibility loss and resume according to the clock you chose from the preserved state ([Reverse or retarget a control in flight, “Rules”](../retarget-controls.md#rules)). On failure or a motion-preference change, settle or cancel into the designed still/reduced state; `complete()` is appropriate only when that state is the intended destination. Islands retain their lifecycle contract (the project’s module loading and mount guide).
- Await `then()` only for work that genuinely follows the motion; content and focus never wait for it ([Choose feedback roles and timing, “2. Rules”](../feedback-roles.md#2-rules): never delay a destination).

## 6. `inView()` and `scroll()`

```js
let controls;
const stop = inView(
  island,
  (element, entry) => {
    controls ??= animate(element, keyframes, { ...options, autoplay: false });
    controls.play();
    return () => controls.pause();
  },
  { root, margin, amount },
);
// Revert path: stop(); controls?.cancel(); restore the authored still.
```

- `inView(target, callback, options)` runs `callback(element, entry)` on entry; the function it returns runs on leave. `amount` is `"some"`, `"all"` or a fraction; `margin` extends the viewport as a CSS margin; `root` is a scrolling ancestor. The call returns a function that stops watching. It is built on Intersection Observer.
- Use it to initialise an island lazily and to suspend it offscreen ([Pause work and resume from the same state](../pause-and-resume.md)).
- `scroll(animationOrCallback, { container, axis, target, offset })` links progress to scroll position; it returns a cancel function and uses the browser's scroll timeline where supported. `offset` pairs describe where tracking starts and ends relative to the target and container.
- Choose scroll linkage for a selected page treatment, keeping reading content immediately available. CSS is a useful first mechanism; [Build a scroll-linked effect or one-time arrival](../scroll/guide.md) owns the host checks.

## 7. `hover()` and `press()`

Enhance an island's own native control under [Design controls that work through the whole task](../../controls.md). Standalone micro-states prefer CSS; retain an engine already in use where they belong to its coordinated choreography.

## 8. Forms that are not current

| Seen in older examples | Current form |
| --- | --- |
| Framework component props and hooks copied into a vanilla island | Use the API for the actual target: this example uses plain JavaScript. A React component uses the Motion React API for the installed version |
| Spring presets written as `stiffness`/`damping` numbers in examples | Read the project’s chosen spring model and shared parameters; physics options remain supported APIs |
| `staggerChildren` and related orchestration options | `stagger()` passed as `delay` |
| Durations written in milliseconds | Motion options take seconds; convert token milliseconds when reading |

## Sources

- [Motion: animate](https://motion.dev/docs/animate): the exact option set, sequence `at` syntax, playback controls, the mini versus hybrid build differences (including registered CSS variables in mini) and the transition options (`type`, `visualDuration`, `bounce`, physics parameters) that decide which spring form applies.
- [Motion: spring](https://motion.dev/docs/spring): spring options, how physics parameters override duration-based ones, and the generator form.
- [Motion: performance](https://motion.dev/docs/performance): why independent transforms animate CSS variables and are not hardware-accelerated, and which values stay on the compositor.
- [MDN: Element.animate()](https://developer.mozilla.org/en-US/docs/Web/API/Element/animate): the Web Animations call an island can make without a vendored engine, and its millisecond durations.
- [Motion: stagger](https://motion.dev/docs/stagger): `startDelay`, `from` and `ease` options and their units.
- [Motion: inView](https://motion.dev/docs/inview): callback, leave function, `amount`, `margin` and `root` semantics.
- [Motion: scroll](https://motion.dev/docs/scroll): `offset` syntax, the `info` object and hardware-accelerated scroll timelines.
- [Motion: hover](https://motion.dev/docs/hover): hover start and end callbacks and the filtering of emulated touch hover.
- [Motion: press](https://motion.dev/docs/press): press filtering, keyboard operation and the `success` flag.
- [Motion source: spring generator at a pinned commit](https://github.com/motiondivision/motion/blob/16c61222c4166bbef5ff6ab0051192faa79fa8c8/packages/motion-dom/src/animation/generators/spring.ts): `getSpringOptions`: a time-defined spring (`duration`, `visualDuration` or `bounce`, without physics parameters) sets its velocity to zero, an explicit `velocity` included.
